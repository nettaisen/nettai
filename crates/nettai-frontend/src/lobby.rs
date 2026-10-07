//! What two netplay peers say before a match, and how they agree it: the
//! lobby and the handshake, without any IO of their own. The host brings
//! the datagrams (the program's: a WebRTC data channel, nettai-rtc's link):
//! [`Lobby::receive`] takes each one that arrives from the other peer,
//! [`Lobby::poll`] says where things stand and [`Lobby::outgoing`] hands
//! over what to send. Nothing is assumed of delivery: a datagram may be
//! lost, reordered or come twice, and each side says its part again every
//! [`RESEND`] until the other has answered.
//!
//! Every datagram starts with a byte that says what it is ([`Kind`]): a
//! frame of the match's protocol (once it is agreed), or one of:
//!
//! 1. **Lobby** — a peer's proposal, `Settings { game, rounds }`, the
//!    rounds by name (each round's stage and background stated, or left to
//!    the seed), whether the peer is ready to play it, and its player's
//!    name ([`player_name`]: shown, never taken for who they are); with the
//!    compatibility fields (the protocol's version, the engine's, the
//!    content's hash, the role), so a peer that can't play the other's
//!    match says so at once. The lobby is symmetric: each peer proposes
//!    ([`Lobby::propose`]) and readies ([`Lobby::set_ready`]) as its player
//!    says; any change by either peer clears both readies; the settings are
//!    agreed when both are ready on the same.
//! 2. **Hello** — once a peer sees the settings agreed: the compatibility
//!    fields, the lobby revisions it agrees to and the rounds' count, in
//!    the clear, and one commitment: SHA-256 of the agreed settings' hash
//!    and the bytes of the Reveal it will send. A peer reveals nothing
//!    before it has the other's Hello.
//! 3. **Reveal** — a peer's half of the seed (a nonce) and its side (its
//!    player's setup, in nettai-match's binary). Each peer checks the
//!    other's Reveal against its commitment, and stops on one that doesn't
//!    match. The seed comes from both nonces; each round's place a round
//!    leaves is picked from it, alike on both peers (`Match::stated`).
//!
//! A **Refuse** ends it: the reason, as text. There is no third message:
//! the match's frames follow the Reveals, and a peer answers a Reveal or a
//! Hello that comes again, after the match is agreed, with its own Reveal
//! ([`Agreement::answer`]).

use std::sync::Arc;
use std::time::{Duration, Instant};

use nettai_battle::content::{Content, ContentHash};
use nettai_match::{Match, RoundSettings, Set, Side};
use nettai_netplay::protocol;
use nettai_netplay::rng::SplitMix64;
use nettai_netplay::wire::{Reader, Writer};
use sha2::{Digest, Sha256};

/// How often a peer says its part again until the other answers.
pub const RESEND: Duration = Duration::from_millis(100);

/// What a datagram is: its first byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    /// A frame of the match's protocol (`nettai_netplay::protocol::Frame`).
    Frame = 0,
    Lobby = 1,
    Hello = 2,
    Reveal = 3,
    /// Why a peer won't play: UTF-8 text.
    Refuse = 4,
}

impl Kind {
    pub fn of(byte: u8) -> Option<Kind> {
        [Kind::Frame, Kind::Lobby, Kind::Hello, Kind::Reveal, Kind::Refuse].into_iter().find(|k| *k as u8 == byte)
    }
}

/// Which side a peer plays: the host is side 0, the joiner side 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Host,
    Join,
}

impl Role {
    /// The side of the battle this role's player plays.
    pub fn side(self) -> usize {
        match self {
            Role::Host => 0,
            Role::Join => 1,
        }
    }
}

/// The engine's version as peers compare it (the engine's crates are
/// versioned together).
pub const ENGINE: &str = env!("CARGO_PKG_VERSION");

/// What two peers must share to play together: the protocol's version, the
/// engine's (the state's digest covers its layout, and the simulation must
/// be the same code), and the content (its hash: one game's, which the
/// settings name too); and opposite roles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compat {
    pub protocol: u16,
    pub engine: String,
    pub content: ContentHash,
    pub role: Role,
}

impl Compat {
    /// This build's, on `content`, in `role`.
    pub fn new(content: &Content, role: Role) -> Compat {
        Compat { protocol: protocol::VERSION, engine: ENGINE.to_string(), content: content.hash(), role }
    }

    fn write(&self, w: &mut Writer) {
        w.put(&self.protocol);
        w.put(&self.engine);
        w.put(&self.content);
        w.put(&(self.role == Role::Join));
    }

    fn read(r: &mut Reader) -> std::io::Result<Compat> {
        Ok(Compat {
            protocol: r.get()?,
            engine: r.get()?,
            content: r.get()?,
            role: if r.get::<bool>()? { Role::Join } else { Role::Host },
        })
    }

    /// Why this peer can't play with `other`, if it can't.
    pub fn mismatch(&self, other: &Compat) -> Option<String> {
        if other.protocol != self.protocol {
            return Some(format!("the other side speaks netplay protocol {}, this one {}", other.protocol, self.protocol));
        }
        if other.engine != self.engine {
            return Some(format!("the other side runs engine {}, this one {}", other.engine, self.engine));
        }
        if other.content != self.content {
            return Some(format!(
                "the other side plays other content (its hash {}, this one's {}): both must play the same content",
                other.content, self.content
            ));
        }
        if other.role == self.role {
            let what = if self.role == Role::Host { "host" } else { "join" };
            return Some(format!("both sides {what}: one hosts and the other joins"));
        }
        None
    }
}

/// What a match is played as, that both peers agree: its game (a match is
/// of one) and its rounds, each round's stage and background stated or
/// left to the seed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub game: String,
    pub rounds: Vec<RoundSettings>,
}

/// A part of a round as the lobby names it: its name, or none (from the
/// seed).
fn put_name(w: &mut Writer, name: Option<&str>) {
    w.put(&name.unwrap_or("").to_string());
}

impl Settings {
    /// A match file's: its game and its rounds.
    pub fn of_match(m: &Match) -> Settings {
        Settings { game: m.game.clone(), rounds: m.rounds.clone() }
    }

    /// The settings as the lobby carries them: the game, then each round's
    /// stage and background by name (empty: from the seed).
    pub fn to_bytes(&self, content: &Content) -> Vec<u8> {
        let mut out = Vec::new();
        let mut w = Writer(&mut out);
        w.put(&self.game);
        w.put(&(self.rounds.len() as u16));
        for r in &self.rounds {
            put_name(&mut w, r.stage.map(|s| nettai_match::ids::local(&content.defs.stage(s).key)));
            put_name(&mut w, r.background.as_deref());
        }
        out
    }

    /// Settings from the lobby's bytes, on `content`: refused, with why,
    /// where they name a stage or a background the content hasn't, or a game
    /// other than the content's.
    pub fn from_bytes(content: &Content, bytes: &[u8]) -> Result<Settings, String> {
        let bad = |e: std::io::Error| format!("the settings don't decode ({e})");
        let mut r = Reader::new(bytes);
        let game: String = r.get().map_err(bad)?;
        let count: u16 = r.get().map_err(bad)?;
        let mut rounds = Vec::with_capacity(count as usize);
        for i in 0..count {
            let stage: String = r.get().map_err(bad)?;
            let background: String = r.get().map_err(bad)?;
            let stage = match stage.as_str() {
                "" => None,
                name => Some(nettai_match::ids::stage(content, &game, name).ok_or_else(|| format!("round {}: no stage {name} in {game}", i + 1))?),
            };
            let background = match background.as_str() {
                "" => None,
                name => {
                    nettai_match::background(content, &game, name).ok_or_else(|| format!("round {}: no background {name} in {game}'s pack", i + 1))?;
                    Some(name.to_string())
                }
            };
            rounds.push(RoundSettings { stage, background });
        }
        r.finish().map_err(bad)?;
        if content.game() != game {
            return Err(format!("a match of {game}, and this side has {}'s content", content.game()));
        }
        Ok(Settings { game, rounds })
    }

    /// The settings' hash, of their bytes.
    pub fn hash(&self, content: &Content) -> [u8; 32] {
        Sha256::digest(self.to_bytes(content)).into()
    }

    /// What differs between these settings and `other`, each said: the
    /// game, the rounds' count, a round's stage or background.
    pub fn differences(&self, content: &Content, other: &Settings) -> Vec<String> {
        let mut out = Vec::new();
        if self.game != other.game {
            out.push(format!("the game: {} here, {} there", self.game, other.game));
        }
        if self.rounds.len() != other.rounds.len() {
            out.push(format!("the rounds: {} here, {} there", self.rounds.len(), other.rounds.len()));
        }
        let stage = |s: Option<nettai_content_api::StageHandle>| s.map_or("from the seed".to_string(), |s| nettai_match::ids::local(&content.defs.stage(s).key).to_string());
        let background = |b: &Option<String>| b.clone().unwrap_or_else(|| "from the seed".to_string());
        for (i, (a, b)) in self.rounds.iter().zip(&other.rounds).enumerate() {
            if a.stage != b.stage {
                out.push(format!("round {}'s stage: {} here, {} there", i + 1, stage(a.stage), stage(b.stage)));
            }
            if a.background != b.background {
                out.push(format!("round {}'s background: {} here, {} there", i + 1, background(&a.background), background(&b.background)));
            }
        }
        out
    }
}

/// The match both peers agreed: the settings, the seed (from both peers'
/// nonces), both sides (by side: the host's, then the joiner's), this
/// peer's side, the set they play and its match (every round's place
/// stated, as the seed picks those the settings leave: what a replay
/// keeps); both players' names as their lobbies said them last (by side);
/// and this peer's Reveal as sent, which it sends again to a Hello or a
/// Reveal that comes after the match is agreed.
#[derive(Clone)]
pub struct Agreement {
    pub settings: Settings,
    pub seed: u32,
    pub sides: [Side; 2],
    pub side: usize,
    pub set: Set,
    pub m: Match,
    pub names: [String; 2],
    pub answer: Vec<u8>,
}

/// How a lobby stands after a poll.
pub enum Status<'a> {
    /// Still agreeing: poll it again.
    Pending,
    /// The match is agreed.
    Agreed(&'a Agreement),
    /// No match, and why.
    Failed(&'a str),
}

/// The other peer's latest lobby state: its revision, its settings (or why
/// they don't decode here), whether it is ready.
#[derive(Clone, Debug)]
struct Theirs {
    revision: u32,
    settings: Result<Settings, String>,
    ready: bool,
    name: String,
}

/// The most characters of a player's name the lobby carries.
pub const NAME_LENGTH: usize = 16;

/// A player's name as the lobby carries and shows it: its control
/// characters left out, trimmed, at most [`NAME_LENGTH`] characters. A
/// name is only shown: nothing takes it for who the player is.
pub fn player_name(name: &str) -> String {
    name.chars().filter(|c| !c.is_control()).collect::<String>().trim().chars().take(NAME_LENGTH).collect::<String>().trim_end().to_string()
}

/// Where the handshake is past the lobby.
enum Phase {
    Lobby,
    /// The settings are agreed: this peer's Hello and Reveal are made and
    /// its Hello is out; the other's Hello hasn't come.
    Hello(Committed),
    /// The other's Hello has come: this peer's Reveal is out.
    Reveal(Committed, [u8; 32]),
    Done(Box<Agreement>),
    Failed(String),
}

/// What a peer committed to: the settings (and their hash), the revisions
/// agreed (this peer's, the other's), its Hello and its Reveal as sent.
#[derive(Clone)]
struct Committed {
    settings: Settings,
    hash: [u8; 32],
    revisions: (u32, u32),
    hello: Vec<u8>,
    reveal: Vec<u8>,
}

/// One peer's lobby and handshake.
pub struct Lobby {
    content: Arc<Content>,
    compat: Compat,
    mine: Settings,
    ready: bool,
    /// This player's name ([`player_name`]).
    name: String,
    revision: u32,
    theirs: Option<Theirs>,
    side: Side,
    /// The nonces this peer draws from (a fresh one each time it reveals).
    nonces: SplitMix64,
    nonce: u64,
    phase: Phase,
    /// A Hello or a Reveal of the other's that came before this peer could
    /// take it (by its revisions, or a Reveal before its Hello).
    their_hello: Option<(Compat, (u32, u32), u16, [u8; 32])>,
    their_reveal: Option<Vec<u8>>,
    outgoing: Vec<Vec<u8>>,
    /// The lobby state changed: say it now.
    changed: bool,
    sent: Option<Instant>,
    start: Option<Instant>,
    heard: Option<Instant>,
    timeout: Duration,
}

impl Lobby {
    /// A peer's lobby in `role` on `content`, proposing `settings` (not
    /// ready yet) with this player's `side`, its nonces drawn from
    /// `entropy`; it fails if nothing comes from the other peer for
    /// `timeout`.
    pub fn new(role: Role, content: &Arc<Content>, settings: Settings, side: Side, entropy: u64, timeout: Duration) -> Lobby {
        let mut nonces = SplitMix64::new(entropy);
        let nonce = nonces.next_u64();
        Lobby {
            content: content.clone(),
            compat: Compat::new(content, role),
            mine: settings,
            ready: false,
            name: String::new(),
            revision: 0,
            theirs: None,
            side,
            nonces,
            nonce,
            phase: Phase::Lobby,
            their_hello: None,
            their_reveal: None,
            outgoing: Vec::new(),
            changed: true,
            sent: None,
            start: None,
            heard: None,
            timeout,
        }
    }

    pub fn role(&self) -> Role {
        self.compat.role
    }

    /// This peer's proposal.
    pub fn mine(&self) -> &Settings {
        &self.mine
    }

    /// The other peer's proposal, once it has come (or why it doesn't
    /// decode on this content).
    pub fn theirs(&self) -> Option<&Result<Settings, String>> {
        self.theirs.as_ref().map(|t| &t.settings)
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn their_ready(&self) -> bool {
        self.theirs.as_ref().is_some_and(|t| t.ready)
    }

    /// The other player's name, as their lobby said it last ([`player_name`]:
    /// shown, never taken for who they are).
    pub fn their_name(&self) -> Option<&str> {
        self.theirs.as_ref().map(|t| t.name.as_str())
    }

    /// Say this player's name ([`player_name`] of it) to the other peer.
    /// Not in the middle of agreeing: a name stays as it was agreed with.
    pub fn set_name(&mut self, name: &str) {
        let name = player_name(name);
        if name != self.name && matches!(self.phase, Phase::Lobby) {
            self.name = name;
            self.revision += 1;
            self.changed = true;
        }
    }

    /// Propose `settings`: a change clears this peer's ready (and the
    /// other's, which sees it), and takes back an agreement not yet played.
    pub fn propose(&mut self, settings: Settings) {
        if settings != self.mine && !self.over() {
            self.mine = settings;
            self.change(false);
        }
    }

    /// Ready (or not) to play the proposal as it stands.
    pub fn set_ready(&mut self, ready: bool) {
        if ready != self.ready && !self.over() {
            self.change(ready);
        }
    }

    /// Refuse to play, saying `why` to the other peer: the lobby fails.
    pub fn refuse(&mut self, why: &str) {
        if self.over() {
            return;
        }
        let mut refusal = vec![Kind::Refuse as u8];
        refusal.extend_from_slice(why.as_bytes());
        for _ in 0..3 {
            self.outgoing.push(refusal.clone());
        }
        self.phase = Phase::Failed(format!("can't play: {why}"));
    }

    /// Whether it has ended (agreed or failed).
    fn over(&self) -> bool {
        matches!(self.phase, Phase::Done(_) | Phase::Failed(_))
    }

    /// This peer's lobby state changes (`ready` now): a new revision, said
    /// at once; an agreement not yet played is taken back.
    fn change(&mut self, ready: bool) {
        self.ready = ready;
        self.revision += 1;
        self.changed = true;
        self.back_to_lobby();
    }

    /// Back to the lobby from a commitment: a Reveal sent is spent (the
    /// next agreement reveals a fresh nonce).
    fn back_to_lobby(&mut self) {
        if let Phase::Reveal(..) = self.phase {
            self.nonce = self.nonces.next_u64();
        }
        if matches!(self.phase, Phase::Hello(_) | Phase::Reveal(..)) {
            self.phase = Phase::Lobby;
        }
    }

    /// The datagrams to send now, in order.
    pub fn outgoing(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.outgoing)
    }

    /// Take a datagram from the other peer, arrived at `now`.
    pub fn receive(&mut self, now: Instant, datagram: &[u8]) {
        let Some((&kind, body)) = datagram.split_first() else { return };
        if matches!(self.phase, Phase::Failed(_)) {
            return;
        }
        self.heard = Some(now);
        match Kind::of(kind) {
            Some(Kind::Refuse) => self.phase = Phase::Failed(format!("the other side refused: {}", String::from_utf8_lossy(body))),
            Some(Kind::Lobby) => self.their_lobby(body),
            Some(Kind::Hello) => self.their_hello(body),
            Some(Kind::Reveal) => self.their_reveal(body),
            Some(Kind::Frame) | None => {}
        }
    }

    /// Their compatibility fields: refused (and this lobby failed) where
    /// this peer can't play with them.
    fn compatible(&mut self, theirs: &Compat) -> bool {
        match self.compat.mismatch(theirs) {
            Some(why) => {
                self.refuse(&why);
                false
            }
            None => true,
        }
    }

    fn their_lobby(&mut self, body: &[u8]) {
        let mut r = Reader::new(body);
        let Ok(compat) = Compat::read(&mut r) else { return };
        if !self.compatible(&compat) || self.over() {
            return;
        }
        let (Ok(revision), Ok(ready), Ok(settings), Ok(name)) = (r.get::<u32>(), r.get::<bool>(), r.bytes(), r.get::<String>()) else { return };
        if self.theirs.as_ref().is_some_and(|t| revision <= t.revision) {
            return;
        }
        let settings = Settings::from_bytes(&self.content, settings);
        let name = player_name(&name);
        // A change of theirs clears this peer's ready too.
        if self.theirs.as_ref().is_some_and(|t| t.settings != settings) && self.ready {
            self.change(false);
        }
        self.theirs = Some(Theirs { revision, settings, ready, name });
        // (A commitment stands while the other's state is the one agreed.)
        if let Phase::Hello(c) | Phase::Reveal(c, _) = &self.phase
            && !(ready && c.revisions.1 == revision)
        {
            self.back_to_lobby();
        }
    }

    fn their_hello(&mut self, body: &[u8]) {
        let mut r = Reader::new(body);
        let Ok(compat) = Compat::read(&mut r) else { return };
        if !self.compatible(&compat) {
            return;
        }
        let (Ok(theirs), Ok(mine), Ok(rounds), Ok(commitment)) = (r.get::<u32>(), r.get::<u32>(), r.get::<u16>(), r.bytes()) else { return };
        let Ok(commitment) = <[u8; 32]>::try_from(commitment) else { return };
        match &self.phase {
            // After the match is agreed: the other hasn't our Reveal.
            Phase::Done(a) => self.outgoing.push(a.answer.clone()),
            _ => {
                self.their_hello = Some((compat, (theirs, mine), rounds, commitment));
            }
        }
    }

    fn their_reveal(&mut self, body: &[u8]) {
        match &self.phase {
            Phase::Done(a) => self.outgoing.push(a.answer.clone()),
            _ => self.their_reveal = Some(body.to_vec()),
        }
    }

    /// Where it stands at `now`, having said what is due (`outgoing`).
    pub fn poll(&mut self, now: Instant) -> Status<'_> {
        let start = *self.start.get_or_insert(now);
        if !self.over() {
            self.step(now);
            if !self.over() && now.duration_since(self.heard.unwrap_or(start)) >= self.timeout {
                self.phase = Phase::Failed("no answer from the other side".into());
            }
        }
        match &self.phase {
            Phase::Done(a) => Status::Agreed(a),
            Phase::Failed(why) => Status::Failed(why),
            _ => Status::Pending,
        }
    }

    fn step(&mut self, now: Instant) {
        // Agreed in the lobby: commit.
        if let Phase::Lobby = self.phase
            && let Some(t) = &self.theirs
            && t.ready
            && self.ready
            && t.settings.as_ref() == Ok(&self.mine)
        {
            let committed = self.commit(t.revision);
            self.phase = Phase::Hello(committed);
            self.changed = true;
        }
        // The other's Hello, once it is of this agreement: reveal.
        if let Phase::Hello(c) = &self.phase
            && let Some((_, (theirs, mine), rounds, commitment)) = &self.their_hello
            && (*mine, *theirs) == c.revisions
            && *rounds as usize == c.settings.rounds.len()
        {
            let (c, commitment) = (c.clone(), *commitment);
            self.outgoing.push(c.reveal.clone());
            self.phase = Phase::Reveal(c, commitment);
        }
        // The other's Reveal, checked against its commitment.
        if let Phase::Reveal(c, commitment) = &self.phase
            && let Some(reveal) = self.their_reveal.take()
        {
            let (c, commitment) = (c.clone(), *commitment);
            match self.agree(&c, &commitment, &reveal) {
                Ok(a) => self.phase = Phase::Done(Box::new(a)),
                Err(why) => self.refuse(&why),
            }
            return;
        }
        // What is due: the lobby state when it changed and every RESEND,
        // with the Hello and the Reveal this peer is at.
        if self.changed || self.sent.is_none_or(|t| now.duration_since(t) >= RESEND) {
            self.outgoing.push(self.lobby_datagram());
            match &self.phase {
                Phase::Hello(c) => self.outgoing.push(c.hello.clone()),
                Phase::Reveal(c, _) => {
                    self.outgoing.push(c.hello.clone());
                    self.outgoing.push(c.reveal.clone());
                }
                _ => {}
            }
            self.changed = false;
            self.sent = Some(now);
        }
    }

    fn lobby_datagram(&self) -> Vec<u8> {
        let mut out = vec![Kind::Lobby as u8];
        let mut w = Writer(&mut out);
        self.compat.write(&mut w);
        w.put(&self.revision);
        w.put(&self.ready);
        w.bytes(&self.mine.to_bytes(&self.content));
        w.put(&self.name);
        out
    }

    /// This peer's commitment to the agreed settings (the other's lobby at
    /// `theirs`): its Reveal (the nonce and its side), and its Hello.
    fn commit(&self, theirs: u32) -> Committed {
        let hash = self.mine.hash(&self.content);
        let mut reveal = vec![Kind::Reveal as u8];
        let mut w = Writer(&mut reveal);
        w.put(&self.nonce);
        w.bytes(&nettai_match::binary::side_bytes(&self.content, &self.side));
        let mut hello = vec![Kind::Hello as u8];
        let mut w = Writer(&mut hello);
        self.compat.write(&mut w);
        w.put(&self.revision);
        w.put(&theirs);
        w.put(&(self.mine.rounds.len() as u16));
        w.bytes(&commitment(&hash, &reveal[1..]));
        Committed { settings: self.mine.clone(), hash, revisions: (self.revision, theirs), hello, reveal }
    }

    /// The match, from the other's Reveal (its bytes): checked against its
    /// commitment, its side read and checked; the seed from both nonces.
    fn agree(&self, c: &Committed, theirs: &[u8; 32], reveal: &[u8]) -> Result<Agreement, String> {
        if commitment(&c.hash, reveal) != *theirs {
            return Err("the other side's Reveal doesn't match its commitment".into());
        }
        let mut r = Reader::new(reveal);
        let bad = |e: std::io::Error| format!("the other side's Reveal doesn't decode ({e})");
        let nonce: u64 = r.get().map_err(bad)?;
        let side_bytes = r.bytes().map_err(bad)?;
        r.finish().map_err(bad)?;
        let side = nettai_match::binary::read_side(&self.content, side_bytes).map_err(|e| format!("the other player's side doesn't decode ({e})"))?;
        let problems = nettai_match::check_side(&self.content, &c.settings.game, &c.settings.rounds, &side);
        if !problems.is_empty() {
            return Err(format!("the other player's side breaks the rules: {}", problems.join("; ")));
        }
        if side.folder(&self.content).saved().is_none() {
            return Err("the other player's folder isn't whole".into());
        }
        let me = self.compat.role.side();
        let (host, join) = if me == 0 { (self.nonce, nonce) } else { (nonce, self.nonce) };
        let seed = seed(host, join);
        let sides = if me == 0 { [self.side.clone(), side] } else { [side, self.side.clone()] };
        let (set, m) = crate::netplay::netplay_setup(&self.content, seed, &c.settings, sides.clone())?;
        let their_name = self.their_name().unwrap_or("").to_string();
        let names = if me == 0 { [self.name.clone(), their_name] } else { [their_name, self.name.clone()] };
        Ok(Agreement { settings: c.settings.clone(), seed, sides, side: me, set, m, names, answer: c.reveal.clone() })
    }
}

/// A commitment: SHA-256 of the agreed settings' hash and the Reveal's
/// bytes as sent (without its kind byte).
fn commitment(settings: &[u8; 32], reveal: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(settings);
    h.update(reveal);
    h.finalize().into()
}

/// The match's seed from both peers' nonces (the host's, the joiner's).
pub fn seed(host: u64, join: u64) -> u32 {
    SplitMix64::new(SplitMix64::new(host).next_u64() ^ join).next_u64() as u32
}

/// A nonce's entropy for a peer: the clock and the process.
pub fn entropy() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0) ^ (std::process::id() as u64) << 32
}

/// Two peers' lobbies agreeing a match in memory, for tests: what each says
/// reaches the other, in order.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// Poll both lobbies at `now`, each taking what the other said, up to
    /// `rounds` times or until both have ended; `tamper` may change what
    /// goes from one to the other (by the sender's side).
    pub fn run(lobbies: &mut [Lobby; 2], now: Instant, rounds: usize, tamper: &mut dyn FnMut(usize, &mut Vec<u8>)) {
        for _ in 0..rounds {
            for from in 0..2 {
                let _ = lobbies[from].poll(now);
                for mut d in lobbies[from].outgoing() {
                    tamper(from, &mut d);
                    lobbies[1 - from].receive(now, &d);
                }
            }
            if lobbies.iter_mut().all(|l| !matches!(l.poll(now), Status::Pending)) {
                break;
            }
        }
        // (What the last polls said reaches the other: a refusal, say.)
        for from in 0..2 {
            for mut d in lobbies[from].outgoing() {
                tamper(from, &mut d);
                lobbies[1 - from].receive(now, &d);
            }
        }
    }

    /// Two peers proposing these settings with these sides (by side: the
    /// host's, then the joiner's), both ready.
    pub fn pair(content: &Arc<Content>, settings: [Settings; 2], sides: [Side; 2]) -> [Lobby; 2] {
        let [s0, s1] = settings;
        let [d0, d1] = sides;
        let mut lobbies = [
            Lobby::new(Role::Host, content, s0, d0, 0x1111, Duration::from_secs(10)),
            Lobby::new(Role::Join, content, s1, d1, 0x2222, Duration::from_secs(10)),
        ];
        for l in &mut lobbies {
            l.set_ready(true);
        }
        lobbies
    }

    /// What both agreed, by side, from these settings (both proposing
    /// them) and sides.
    pub fn agree(content: &Arc<Content>, settings: &Settings, sides: [Side; 2]) -> [Agreement; 2] {
        let mut lobbies = pair(content, [settings.clone(), settings.clone()], sides);
        run(&mut lobbies, Instant::now(), 20, &mut |_, _| {});
        lobbies.map(|mut l| match l.poll(Instant::now()) {
            Status::Agreed(a) => a.clone(),
            Status::Failed(why) => panic!("{why}"),
            Status::Pending => panic!("not agreed"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{agree, pair, run};
    use super::*;
    use nettai_match::{Picks, TRIPLE_BATTLE};

    fn side(content: &Arc<Content>, seed: u32) -> Side {
        Side::picked(content, content.game(), &mut Picks::new(seed)).unwrap()
    }

    fn settings(rounds: Vec<RoundSettings>) -> Settings {
        Settings { game: "exe6".into(), rounds }
    }

    /// Each player's name goes to the other, cut to its length and without
    /// control characters, and the agreement holds both by side; a name
    /// changes nobody's readiness.
    #[test]
    fn names_go_to_the_other_player() {
        let content = nettai_match::testing::exe6_content();
        let s = settings(vec![RoundSettings::default(); TRIPLE_BATTLE]);
        let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
        lobbies[0].set_name("  Lan\u{7}\u{1b}[31m ");
        lobbies[1].set_name("Chaud Blaze, of Officials");
        lobbies[1].set_ready(false);
        run(&mut lobbies, Instant::now(), 3, &mut |_, _| {});
        assert_eq!(lobbies[1].their_name(), Some("Lan[31m"));
        assert_eq!(lobbies[0].their_name(), Some("Chaud Blaze, of"));
        assert!(lobbies[0].ready(), "a name changes no readiness");
        lobbies[1].set_ready(true);
        run(&mut lobbies, Instant::now(), 20, &mut |_, _| {});
        let names: Vec<[String; 2]> = lobbies
            .iter_mut()
            .map(|l| match l.poll(Instant::now()) {
                Status::Agreed(a) => a.names.clone(),
                _ => panic!("not agreed"),
            })
            .collect();
        assert_eq!(names[0], ["Lan[31m".to_string(), "Chaud Blaze, of".to_string()]);
        assert_eq!(names[1], names[0]);
        assert_eq!(player_name("\u{0}\u{0}"), "");
    }

    /// Settings go by name and read back; ones this content can't play are
    /// refused with why; what differs between two is said, part by part.
    #[test]
    fn settings_go_by_name() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 9, None).unwrap();
        let s = Settings::of_match(&m);
        assert_eq!(Settings::from_bytes(&content, &s.to_bytes(&content)).unwrap(), s);
        let mut blank = s.clone();
        blank.rounds = vec![RoundSettings::default(); 5];
        assert_eq!(Settings::from_bytes(&content, &blank.to_bytes(&content)).unwrap(), blank);
        let mut other = s.clone();
        other.game = "exe5".into();
        assert!(Settings::from_bytes(&content, &other.to_bytes(&content)).unwrap_err().contains("no stage"));
        let mut bytes = Vec::new();
        let mut w = Writer(&mut bytes);
        w.put(&"exe6".to_string());
        w.put(&1u16);
        w.put(&"no-such-stage".to_string());
        w.put(&String::new());
        assert_eq!(Settings::from_bytes(&content, &bytes).unwrap_err(), "round 1: no stage no-such-stage in exe6");
        let said = s.differences(&content, &blank);
        assert_eq!(said[0], "the rounds: 3 here, 5 there");
        assert!(said[1].starts_with("round 1's stage: netbattle-") && said[1].ends_with(" here, from the seed there"), "{said:?}");
    }

    /// Two peers agree each kind of arena alike (every part from the seed;
    /// a stage alone; a background alone; a random match's three, stated;
    /// five of every kind; one round), each player's side on their side,
    /// every place stated as the seed picks those left, the seed from both
    /// nonces; the set both start is the same.
    #[test]
    fn peers_agree_each_arena() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 9, None).unwrap();
        let stage = RoundSettings { stage: m.rounds[0].stage, background: None };
        let background = RoundSettings { stage: None, background: m.rounds[1].background.clone() };
        let five = vec![RoundSettings::default(), stage.clone(), background.clone(), m.rounds[2].clone(), RoundSettings::default()];
        for rounds in [vec![RoundSettings::default(); TRIPLE_BATTLE], vec![stage], vec![background], m.rounds.clone(), five] {
            let s = settings(rounds.clone());
            let sides = [side(&content, 3), side(&content, 4)];
            let [h, j] = agree(&content, &s, sides.clone());
            assert_eq!((h.side, j.side), (0, 1));
            assert_eq!(h.seed, j.seed);
            assert_eq!((&h.m, &h.sides), (&j.m, &j.sides));
            assert_eq!(h.sides, sides);
            assert_eq!(h.m.rounds.len(), rounds.len());
            for (stated, asked) in h.m.rounds.iter().zip(&rounds) {
                assert!(stated.stage.is_some() && stated.background.is_some());
                assert!(asked.stage.is_none() || asked.stage == stated.stage);
                assert!(asked.background.is_none() || asked.background == stated.background);
            }
            assert_eq!(format!("{:?}", h.set.first()), format!("{:?}", j.set.first()));
        }
    }

    /// The lobby: a peer that changes its proposal clears both readies;
    /// different settings never agree (each says the other's); once both
    /// propose the same and ready again, they agree on it.
    #[test]
    fn a_change_clears_both_readies() {
        let content = nettai_match::testing::exe6_content();
        let now = Instant::now();
        let three = settings(vec![RoundSettings::default(); 3]);
        let five = settings(vec![RoundSettings::default(); 5]);
        let mut lobbies = pair(&content, [three.clone(), three.clone()], [side(&content, 3), side(&content, 4)]);
        // (The host waits: only the joiner is ready.)
        lobbies[0].set_ready(false);
        run(&mut lobbies, now, 4, &mut |_, _| {});
        assert!(matches!(lobbies[1].poll(now), Status::Pending));
        assert!(lobbies[0].their_ready() && !lobbies[1].their_ready());
        // The host proposes five rounds: both readies are cleared.
        lobbies[0].propose(five.clone());
        run(&mut lobbies, now, 4, &mut |_, _| {});
        assert!(!lobbies[0].ready() && !lobbies[1].ready() && !lobbies[0].their_ready() && !lobbies[1].their_ready());
        assert_eq!(lobbies[1].theirs(), Some(&Ok(five.clone())));
        // Both ready on different settings: no agreement.
        lobbies[0].set_ready(true);
        lobbies[1].set_ready(true);
        run(&mut lobbies, now, 6, &mut |_, _| {});
        assert!(lobbies.iter_mut().all(|l| matches!(l.poll(now), Status::Pending)));
        // The joiner takes five too, and readies: agreed, on five.
        lobbies[1].propose(five.clone());
        lobbies[1].set_ready(true);
        run(&mut lobbies, now, 4, &mut |_, _| {});
        assert!(!lobbies[0].ready(), "the joiner's change cleared the host's ready");
        lobbies[0].set_ready(true);
        run(&mut lobbies, now, 10, &mut |_, _| {});
        for l in &mut lobbies {
            let Status::Agreed(a) = l.poll(now) else { panic!("not agreed") };
            assert_eq!(a.settings, five);
        }
    }

    /// A Reveal that isn't what its peer committed to is refused, a
    /// tampered nonce or a tampered side alike: the other peer stops saying
    /// so, and the tampered one hears it.
    #[test]
    fn a_tampered_reveal_is_refused() {
        let content = nettai_match::testing::exe6_content();
        let s = settings(vec![RoundSettings::default(); 3]);
        for (what, at) in [("nonce", 1usize), ("side", 12)] {
            let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
            // (The joiner's Reveal: its kind, then the nonce, then its side.)
            run(&mut lobbies, Instant::now(), 20, &mut |from, d| {
                if from == 1 && d[0] == Kind::Reveal as u8 {
                    d[at] ^= 1;
                }
            });
            let Status::Failed(why) = lobbies[0].poll(Instant::now()) else { panic!("{what}: not refused") };
            assert_eq!(why, "can't play: the other side's Reveal doesn't match its commitment", "{what}");
            let Status::Failed(why) = lobbies[1].poll(Instant::now()) else { panic!("{what}: the joiner didn't hear it") };
            assert_eq!(why, "the other side refused: the other side's Reveal doesn't match its commitment", "{what}");
        }
    }

    /// Peers that can't play together stop, each saying why: other
    /// content, another protocol, two hosts; and one nobody answers gives up
    /// after the timeout, having said its part every RESEND.
    #[test]
    fn peers_that_cant_play_say_why() {
        let content = nettai_match::testing::exe6_content();
        let s = settings(vec![RoundSettings::default(); 3]);
        let now = Instant::now();
        // (Each failed, saying why: the joiner, who hears the host first,
        // refuses, and the host hears it.)
        let failed = |lobbies: &mut [Lobby; 2]| -> [String; 2] {
            [0, 1].map(|i| match lobbies[i].poll(now) {
                Status::Failed(why) => why.to_string(),
                _ => panic!("side {i} plays"),
            })
        };
        let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
        lobbies[1].compat.content = ContentHash(content.hash().0 ^ 1);
        run(&mut lobbies, now, 4, &mut |_, _| {});
        let [h, j] = failed(&mut lobbies);
        assert!(j.starts_with("can't play: the other side plays other content"), "{j}");
        assert!(h.starts_with("the other side refused: the other side plays other content"), "{h}");
        let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
        lobbies[1].compat.protocol += 1;
        run(&mut lobbies, now, 4, &mut |_, _| {});
        let v = protocol::VERSION;
        let [h, j] = failed(&mut lobbies);
        assert_eq!(j, format!("can't play: the other side speaks netplay protocol {v}, this one {}", v + 1));
        assert_eq!(h, format!("the other side refused: the other side speaks netplay protocol {v}, this one {}", v + 1));
        let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
        lobbies[1].compat.role = Role::Host;
        run(&mut lobbies, now, 4, &mut |_, _| {});
        assert_eq!(failed(&mut lobbies)[1], "can't play: both sides host: one hosts and the other joins");
        // Nobody answers.
        let timeout = Duration::from_secs(10);
        let mut alone = Lobby::new(Role::Join, &content, s, side(&content, 3), 7, timeout);
        assert!(matches!(alone.poll(now), Status::Pending));
        assert!(matches!(alone.poll(now + RESEND / 2), Status::Pending));
        assert!(matches!(alone.poll(now + RESEND), Status::Pending));
        assert_eq!(alone.outgoing().len(), 2, "said at once, and again after RESEND");
        let Status::Failed(why) = alone.poll(now + timeout) else { panic!("no timeout") };
        assert_eq!(why, "no answer from the other side");
    }
}
