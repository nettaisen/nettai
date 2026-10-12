//! What two netplay peers say before a match, and how they agree it: the
//! lobby, the handshake and the loading, without any IO of their own. The
//! host brings the datagrams (the program's: a WebRTC data channel,
//! nettai-rtc's link): [`Lobby::receive`] takes each one that arrives from
//! the other peer (every one, the match's frames too), [`Lobby::poll`] says
//! where things stand and [`Lobby::outgoing`] hands over what to send.
//! Nothing is assumed of delivery: a datagram may be lost, reordered or come
//! twice, and each side says its part again every [`RESEND`] until the other
//! has answered.
//!
//! **Nothing of a player's build crosses before the Reveal**: not its navi,
//! not its version, not a label of any kind; and a build's name never
//! crosses at all (a side is its facts, nothing more). What a peer says of
//! its player before the Reveal is their name and whether they are ready.
//!
//! Every datagram starts with a byte that says what it is ([`Kind`]): a
//! frame of the match's protocol (once it has started), or one of:
//!
//! 1. **Lobby** — the compatibility fields (the protocol's version, the
//!    engine's, the game, the content's hash, the role), so a peer that
//!    can't play the other's match says so at once; the match's settings
//!    ([`Settings`]: the game, the rounds by name, each round's stage and
//!    background stated or left to the seed, and whether builds are closed)
//!    with their stamp; whether the peer is ready to play them; its
//!    player's name ([`player_name`]: shown, never taken for who they are);
//!    and a clock to time the round trip by ([`Lobby::ping`]).
//!
//!    **The settings are the match's, not a player's.** Either player
//!    changes them ([`Lobby::propose`]); the other peer adopts the change,
//!    so both show the same, and both players' ready marks clear. A change
//!    carries a stamp one above every stamp its peer has seen, and a peer
//!    adopts settings of a higher stamp than its own; where both changed at
//!    once (the same stamp, other settings), the host's stand and the
//!    joiner adopts them, as it adopts the host's when it comes into the
//!    lobby. The settings are agreed when both players are ready on the
//!    same, of the same stamp.
//! 2. **Hello** — once a peer sees the settings agreed: the compatibility
//!    fields, the lobby revisions it agrees to and the rounds' count, in
//!    the clear, and one commitment: SHA-256 of the agreed settings' hash
//!    and the bytes of the Reveal it will send. A peer reveals nothing
//!    before it has the other's Hello.
//! 3. **Reveal** — the lobby revisions it is of (as its Hello said them),
//!    a peer's half of the seed (a nonce) and its side (its player's setup,
//!    in nettai-match's binary). A Reveal of other revisions is of an
//!    agreement that was taken back, come late: it is left. Each peer
//!    checks the other's Reveal against its commitment, and stops on one
//!    that doesn't match. The seed comes from both nonces; each round's
//!    place a round leaves is picked from it, alike on both peers
//!    (`Match::stated`).
//! 4. **Loading** — the match is agreed ([`Status::Loading`]), and each
//!    host loads what it will draw and play of it before its first tick:
//!    each peer says how far its player's has got ([`Lobby::set_loading`],
//!    [`Lobby::set_loaded`]), which the other can show
//!    ([`Lobby::their_loading`]). The match starts ([`Status::Start`]) once
//!    this peer has loaded and has heard the other has: by its Loading, or
//!    by a frame of the match, which it sends only once it has started.
//!
//! A **Refuse** ends it: the compatibility fields, a reason and, for a log,
//! the detail. The peer that refuses and the one that is refused each end
//! with a [`Failure`], whose [`Reason`] is what a host tells its player, in
//! its own words. A player who leaves the lobby says so the same way
//! ([`Lobby::leave`]). There is no message after the Loading: the match's
//! frames follow, and a peer answers a Reveal or a Hello that comes again,
//! after the match is agreed, with its own Reveal ([`Agreement::answer`]).

use std::sync::Arc;
use std::time::{Duration, Instant};

use nettai_battle::content::{Content, ContentHash};
use nettai_match::{Match, RoundSettings, Set, Side};
use nettai_netplay::protocol;
use nettai_netplay::rng::SplitMix64;
use nettai_netplay::wire::{Reader, Writer};
use sha2::{Digest, Sha256};

use crate::connection::Ping;

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
    /// Why a peer won't play: its compatibility fields, a reason, and the
    /// detail as UTF-8 text.
    Refuse = 4,
    /// How far a peer is with loading the agreed match.
    Loading = 5,
}

impl Kind {
    pub fn of(byte: u8) -> Option<Kind> {
        [Kind::Frame, Kind::Lobby, Kind::Hello, Kind::Reveal, Kind::Refuse, Kind::Loading].into_iter().find(|k| *k as u8 == byte)
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

/// Why a lobby ended without a match, as a host can tell its player: each
/// reason in the host's own words (a catalog's string, in the player's
/// language). A [`Failure`]'s detail is never shown.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Reason {
    /// The two programs differ: another netplay protocol, or another
    /// engine. Both players need the same version of the program.
    DifferentVersion,
    /// The other player's match is of another game: `theirs`, by its id.
    DifferentGame { theirs: String },
    /// The same game, but other content: the two players' packs differ.
    DifferentContent,
    /// This player's build breaks a rule of the game, by the other side's
    /// checks, which refused it.
    YourBuild,
    /// The other player's build breaks a rule of the game.
    TheirBuild,
    /// The other player left the lobby.
    OpponentLeft,
    /// This player left the lobby ([`Lobby::leave`]).
    Left,
    /// Nothing came from the other side for the lobby's timeout.
    NoAnswer,
    /// The other side didn't keep the handshake: it revealed something
    /// other than it committed to, said what doesn't read, or plays the
    /// same role (or it says this side did). Nothing a player can mend.
    Protocol,
}

/// A lobby that ended without a match: the reason, which is what the player
/// is told (in the host's words), and the detail, in English, for a log
/// only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub reason: Reason,
    pub detail: String,
}

/// A refusal's reason as the wire carries it: what the peer that refuses
/// found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Refusal {
    /// Another protocol or engine.
    Version = 1,
    /// Another game.
    Game = 2,
    /// The same game, other content.
    Content = 3,
    /// The handshake wasn't kept (the same role, a Reveal that doesn't
    /// match its commitment, a message that doesn't read).
    Protocol = 4,
    /// The other's build breaks a rule.
    Build = 5,
    /// The player left the lobby.
    Left = 6,
}

impl Refusal {
    fn of(byte: u8) -> Option<Refusal> {
        [Refusal::Version, Refusal::Game, Refusal::Content, Refusal::Protocol, Refusal::Build, Refusal::Left].into_iter().find(|r| *r as u8 == byte)
    }

    /// The refusal of what this peer found (`reason`, a local one).
    fn saying(reason: &Reason) -> Refusal {
        match reason {
            Reason::DifferentVersion => Refusal::Version,
            Reason::DifferentGame { .. } => Refusal::Game,
            Reason::DifferentContent => Refusal::Content,
            Reason::TheirBuild => Refusal::Build,
            Reason::Left => Refusal::Left,
            Reason::YourBuild | Reason::OpponentLeft | Reason::NoAnswer | Reason::Protocol => Refusal::Protocol,
        }
    }

    /// What the peer that is refused tells its player: `theirs` is the
    /// refusing peer's.
    fn heard(self, theirs: &Compat) -> Reason {
        match self {
            Refusal::Version => Reason::DifferentVersion,
            Refusal::Game => Reason::DifferentGame { theirs: theirs.game.clone() },
            Refusal::Content => Reason::DifferentContent,
            Refusal::Protocol => Reason::Protocol,
            Refusal::Build => Reason::YourBuild,
            Refusal::Left => Reason::OpponentLeft,
        }
    }
}

/// What two peers must share to play together: the protocol's version, the
/// engine's (the state's digest covers its layout, and the simulation must
/// be the same code), the game (a match is of one) and the content (its
/// hash); and opposite roles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compat {
    pub protocol: u16,
    pub engine: String,
    pub game: String,
    pub content: ContentHash,
    pub role: Role,
}

impl Compat {
    /// This build's, on `content`, in `role`.
    pub fn new(content: &Content, role: Role) -> Compat {
        Compat { protocol: protocol::VERSION, engine: ENGINE.to_string(), game: content.game().to_string(), content: content.hash(), role }
    }

    fn write(&self, w: &mut Writer) {
        w.put(&self.protocol);
        w.put(&self.engine);
        w.put(&self.game);
        w.put(&self.content);
        w.put(&(self.role == Role::Join));
    }

    /// A peer's fields from the start of one of its datagrams. The
    /// protocol's version comes first, and nothing after it is read of a
    /// peer of another (its datagrams may be laid out another way): such a
    /// peer has that version and nothing else known of it.
    fn read(&self, r: &mut Reader) -> std::io::Result<Compat> {
        let protocol: u16 = r.get()?;
        if protocol != self.protocol {
            return Ok(Compat { protocol, engine: String::new(), game: String::new(), content: ContentHash(0), role: self.role });
        }
        Ok(Compat {
            protocol,
            engine: r.get()?,
            game: r.get()?,
            content: r.get()?,
            role: if r.get::<bool>()? { Role::Join } else { Role::Host },
        })
    }

    /// Why this peer can't play with `other`, if it can't.
    pub fn mismatch(&self, other: &Compat) -> Option<Failure> {
        let failure = |reason, detail| Some(Failure { reason, detail });
        if other.protocol != self.protocol {
            return failure(Reason::DifferentVersion, format!("the other side speaks netplay protocol {}, this one {}", other.protocol, self.protocol));
        }
        if other.engine != self.engine {
            return failure(Reason::DifferentVersion, format!("the other side runs engine {}, this one {}", other.engine, self.engine));
        }
        if other.game != self.game {
            return failure(Reason::DifferentGame { theirs: other.game.clone() }, format!("the other side plays {}, this one {}: a match is of one game", other.game, self.game));
        }
        if other.content != self.content {
            return failure(
                Reason::DifferentContent,
                format!("the other side plays other content (its hash {}, this one's {}): both must play the same content", other.content, self.content),
            );
        }
        if other.role == self.role {
            let what = if self.role == Role::Host { "host" } else { "join" };
            return failure(Reason::Protocol, format!("both sides {what}: one hosts and the other joins"));
        }
        None
    }
}

/// What a match is played as, that both peers agree: its game (a match is
/// of one), its rounds, each round's stage and background stated or left to
/// the seed, and whether the builds are closed. They are the match's, not
/// a player's: either player changes them in the lobby and both see the
/// same ([`Lobby::propose`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub game: String,
    /// The rounds: one is a single battle, three the original's triple
    /// battle (best of three), and a set takes any number from 1 to
    /// [`nettai_match::MAX_ROUNDS`].
    pub rounds: Vec<RoundSettings>,
    /// The players keep their builds to themselves during the match: a host
    /// shows each player their own build only, where with open builds it
    /// may show both. (It is the host's to honor: each peer has the other's
    /// side from the Reveal on, as the simulation needs it.)
    pub closed_builds: bool,
}

/// A part of a round as the lobby names it: its name, or none (from the
/// seed).
fn put_name(w: &mut Writer, name: Option<&str>) {
    w.put(&name.unwrap_or("").to_string());
}

impl Settings {
    /// A match of `game` of `rounds` rounds, every place left to the seed,
    /// with open builds.
    pub fn new(game: &str, rounds: usize) -> Settings {
        Settings { game: game.to_string(), rounds: vec![RoundSettings::default(); rounds], closed_builds: false }
    }

    /// A match file's: its game and its rounds, with open builds.
    pub fn of_match(m: &Match) -> Settings {
        Settings { game: m.game.clone(), rounds: m.rounds.clone(), closed_builds: false }
    }

    /// The settings as the lobby carries them: the game, whether builds are
    /// closed, then each round's stage and background by name (empty: from
    /// the seed).
    pub fn to_bytes(&self, content: &Content) -> Vec<u8> {
        let mut out = Vec::new();
        let mut w = Writer(&mut out);
        w.put(&self.game);
        w.put(&self.closed_builds);
        w.put(&(self.rounds.len() as u16));
        for r in &self.rounds {
            put_name(&mut w, r.stage.map(|s| nettai_match::ids::local(&content.defs.stage(s).key)));
            put_name(&mut w, r.background.as_deref());
        }
        out
    }

    /// Settings from the lobby's bytes, on `content`: refused, with why,
    /// where they have no round or too many, name a stage or a background
    /// the content hasn't, or a game other than the content's.
    pub fn from_bytes(content: &Content, bytes: &[u8]) -> Result<Settings, String> {
        let bad = |e: std::io::Error| format!("the settings don't decode ({e})");
        let mut r = Reader::new(bytes);
        let game: String = r.get().map_err(bad)?;
        let closed_builds: bool = r.get().map_err(bad)?;
        let count: u16 = r.get().map_err(bad)?;
        if count == 0 || count as usize > nettai_match::MAX_ROUNDS {
            return Err(format!("{count} rounds: a match has 1 to {}", nettai_match::MAX_ROUNDS));
        }
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
        Ok(Settings { game, rounds, closed_builds })
    }

    /// The settings' hash, of their bytes.
    pub fn hash(&self, content: &Content) -> [u8; 32] {
        Sha256::digest(self.to_bytes(content)).into()
    }
}

/// The match both peers agreed: the settings (its rounds, and whether the
/// builds are closed), the seed (from both peers' nonces), both sides (by
/// side: the host's, then the joiner's), this peer's side, the set they
/// play and its match (every round's place stated, as the seed picks those
/// the settings leave: what a replay keeps); both players' names as their
/// lobbies said them last (by side); and this peer's Reveal as sent, which
/// it sends again to a Hello or a Reveal that comes after the match is
/// agreed.
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

impl Agreement {
    /// Whether the players agreed to keep their builds to themselves: the
    /// host then shows this player their own build only (`sides[side]`),
    /// during the match and wherever the match is shown again (a recording
    /// that is to honor it must keep it).
    pub fn closed_builds(&self) -> bool {
        self.settings.closed_builds
    }
}

/// How a lobby stands after a poll.
pub enum Status<'a> {
    /// Still agreeing: poll it again.
    Pending,
    /// The match is agreed and both sides are revealed: the host loads what
    /// it needs of it, saying how far it is ([`Lobby::set_loading`]) and
    /// when it is done ([`Lobby::set_loaded`]), and polls on.
    Loading(&'a Agreement),
    /// Both players have loaded: the match starts now (a `NetPlayer` of the
    /// agreement's set, on the same datagrams).
    Start(&'a Agreement),
    /// No match, and why.
    Failed(&'a Failure),
}

/// The other peer's latest lobby state: its revision, the settings it holds
/// and their stamp, whether it is ready, its player's name.
#[derive(Clone, Debug)]
struct Theirs {
    revision: u32,
    stamp: u32,
    settings: Settings,
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
    /// The match is agreed: loading, then started.
    Done(Box<Agreement>),
    Failed(Failure),
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

/// How far a peer's loading is, as the wire carries it: 255 parts.
const LOADING_PARTS: f32 = 255.0;

/// One peer's lobby and handshake.
pub struct Lobby {
    content: Arc<Content>,
    compat: Compat,
    /// The match's settings as this peer holds them, and their stamp.
    mine: Settings,
    stamp: u32,
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
    their_hello: Option<((u32, u32), u16, [u8; 32])>,
    their_reveal: Option<Vec<u8>>,
    /// Loading the agreed match: how far this peer is (of
    /// [`LOADING_PARTS`]) and whether it is done; the other's, with the
    /// seed of the match it is loading; and whether the other has started
    /// (a frame of the match came).
    loading: (u8, bool),
    their_loading: Option<(u32, u8, bool)>,
    their_frame: bool,
    started: bool,
    /// The round trip, timed by the lobby's own messages: the other's
    /// clock as its latest message said it, and when that came; and the
    /// ping so far.
    their_clock: Option<(u32, Instant)>,
    ping: Option<Ping>,
    outgoing: Vec<Vec<u8>>,
    /// The lobby state changed: say it now.
    changed: bool,
    sent: Option<Instant>,
    start: Option<Instant>,
    heard: Option<Instant>,
    timeout: Duration,
}

impl Lobby {
    /// A peer's lobby in `role` on `content`, holding `settings` (not ready
    /// yet) with this player's `side`, its nonces drawn from `entropy`; it
    /// fails if nothing comes from the other peer for `timeout`. (A joiner's
    /// settings last until it hears the host's, which it adopts.) A host
    /// makes it once its link to the other player is open, with a timeout
    /// longer than its link takes to give a dropped connection up: the
    /// lobby hears nothing while the link is down.
    pub fn new(role: Role, content: &Arc<Content>, settings: Settings, side: Side, entropy: u64, timeout: Duration) -> Lobby {
        let mut nonces = SplitMix64::new(entropy);
        let nonce = nonces.next_u64();
        Lobby {
            content: content.clone(),
            compat: Compat::new(content, role),
            mine: settings,
            stamp: 0,
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
            loading: (0, false),
            their_loading: None,
            their_frame: false,
            started: false,
            their_clock: None,
            ping: None,
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

    /// The match's settings as they stand: the ones both players see, once
    /// each peer has heard the other.
    pub fn settings(&self) -> &Settings {
        &self.mine
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn their_ready(&self) -> bool {
        self.theirs.as_ref().is_some_and(|t| t.ready)
    }

    /// Whether the other peer's lobby has been heard: the other player is
    /// there.
    pub fn their_present(&self) -> bool {
        self.theirs.is_some()
    }

    /// The other player's name, as their lobby said it last ([`player_name`]:
    /// shown, never taken for who they are).
    pub fn their_name(&self) -> Option<&str> {
        self.theirs.as_ref().map(|t| t.name.as_str())
    }

    /// The round trip to the other peer as the lobby's messages time it
    /// (`crate::connection`): none until one has gone and come back.
    pub fn ping(&self) -> Option<Ping> {
        self.ping
    }

    /// The side this player brings, from now (another build). Nothing of it
    /// is said to the other peer; a ready player is no longer ready (their
    /// ready mark was for the side they had), which takes back an agreement
    /// not yet revealed.
    pub fn set_side(&mut self, side: Side) {
        if side != self.side && !self.settled() {
            self.side = side;
            if self.ready {
                self.change(false);
            }
        }
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

    /// Change the match's settings: the other peer adopts them, both
    /// players' ready marks clear, and an agreement not yet revealed is
    /// taken back. (Where the other player changed them at the same time,
    /// the host's change stands.)
    pub fn propose(&mut self, settings: Settings) {
        if settings != self.mine && !self.settled() {
            self.mine = settings;
            self.stamp = self.stamp.max(self.theirs.as_ref().map_or(0, |t| t.stamp)) + 1;
            // (The other's mark clears when it adopts them; until it says
            // so, it isn't ready for these.)
            if let Some(t) = &mut self.theirs {
                t.ready = false;
            }
            self.change(false);
        }
    }

    /// Change how many rounds the match has (each place left to the seed):
    /// [`Lobby::propose`] of the settings with that many.
    pub fn set_rounds(&mut self, rounds: usize) {
        let rounds = vec![RoundSettings::default(); rounds.clamp(1, nettai_match::MAX_ROUNDS)];
        self.propose(Settings { rounds, ..self.mine.clone() });
    }

    /// Change whether the match's builds are closed: [`Lobby::propose`] of
    /// the settings with that.
    pub fn set_closed_builds(&mut self, closed_builds: bool) {
        self.propose(Settings { closed_builds, ..self.mine.clone() });
    }

    /// Ready (or not) to play the settings as they stand.
    pub fn set_ready(&mut self, ready: bool) {
        if ready != self.ready && !self.settled() {
            self.change(ready);
        }
    }

    /// This player leaves the lobby: the other peer is told, and both
    /// lobbies end ([`Reason::Left`] here, [`Reason::OpponentLeft`] there).
    /// What it says is in [`Lobby::outgoing`], to send before the link is
    /// dropped.
    pub fn leave(&mut self) {
        self.fail(Failure { reason: Reason::Left, detail: "this player left the lobby".into() }, true);
    }

    /// The lobby ends without a match, for `failure`; `refuse`: the other
    /// peer is told (a few times: datagrams get lost).
    fn fail(&mut self, failure: Failure, refuse: bool) {
        if matches!(self.phase, Phase::Failed(_)) || self.started {
            return;
        }
        if refuse {
            let mut refusal = vec![Kind::Refuse as u8];
            let mut w = Writer(&mut refusal);
            self.compat.write(&mut w);
            w.put(&(Refusal::saying(&failure.reason) as u8));
            w.put(&failure.detail);
            for _ in 0..3 {
                self.outgoing.push(refusal.clone());
            }
        }
        self.phase = Phase::Failed(failure);
    }

    /// Whether the lobby's part is over (agreed or failed): nothing of it
    /// changes any more.
    fn settled(&self) -> bool {
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

    // ---- Loading -----------------------------------------------------------------

    /// Say how far this host is with loading the agreed match, from 0 to 1
    /// (short of done: [`Lobby::set_loaded`] says that). Before the match
    /// is agreed there is nothing to load, and this says nothing.
    pub fn set_loading(&mut self, fraction: f32) {
        let parts = (fraction.clamp(0.0, 1.0) * LOADING_PARTS) as u8;
        if matches!(self.phase, Phase::Done(_)) && !self.loading.1 && parts != self.loading.0 {
            self.loading.0 = parts;
            self.changed = true;
        }
    }

    /// This host has loaded the agreed match: it is ready for its first
    /// tick. (Before the match is agreed there is nothing to have loaded,
    /// and this says nothing.)
    pub fn set_loaded(&mut self) {
        if matches!(self.phase, Phase::Done(_)) && !self.loading.1 {
            self.loading = (LOADING_PARTS as u8, true);
            self.changed = true;
        }
    }

    /// How far this host's loading is, as it said: 0 to 1.
    pub fn loading(&self) -> f32 {
        self.loading.0 as f32 / LOADING_PARTS
    }

    /// How far the other host is with loading the agreed match, 0 to 1 (0
    /// until it has said).
    pub fn their_loading(&self) -> f32 {
        if self.their_loaded() {
            return 1.0;
        }
        self.their_load().map_or(0.0, |(parts, _)| parts as f32 / LOADING_PARTS)
    }

    /// Whether the other host has loaded: it said so, or it has started.
    pub fn their_loaded(&self) -> bool {
        self.their_frame || self.their_load().is_some_and(|(_, loaded)| loaded)
    }

    /// The other's loading of this agreement, if it has said (what it said
    /// of another match, by its seed, is none of this one's).
    fn their_load(&self) -> Option<(u8, bool)> {
        let Phase::Done(a) = &self.phase else { return None };
        self.their_loading.filter(|(seed, ..)| *seed == a.seed).map(|(_, parts, loaded)| (parts, loaded))
    }

    // ---- The datagrams -----------------------------------------------------------

    /// The datagrams to send now, in order.
    pub fn outgoing(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.outgoing)
    }

    /// Take a datagram from the other peer, arrived at `now`: every one the
    /// link brings, a frame of the match too (the other peer sends them
    /// once it has started: it has loaded).
    pub fn receive(&mut self, now: Instant, datagram: &[u8]) {
        let Some((&kind, body)) = datagram.split_first() else { return };
        if matches!(self.phase, Phase::Failed(_)) {
            return;
        }
        self.start.get_or_insert(now);
        self.heard = Some(now);
        match Kind::of(kind) {
            Some(Kind::Refuse) => self.heard_refusal(body),
            Some(Kind::Lobby) => self.heard_lobby(now, body),
            Some(Kind::Hello) => self.heard_hello(body),
            Some(Kind::Reveal) => self.heard_reveal(body),
            Some(Kind::Loading) => self.heard_loading(body),
            Some(Kind::Frame) => self.their_frame |= matches!(self.phase, Phase::Done(_)),
            None => {}
        }
    }

    /// Their compatibility fields: refused (and this lobby failed) where
    /// this peer can't play with them.
    fn compatible(&mut self, theirs: &Compat) -> bool {
        match self.compat.mismatch(theirs) {
            Some(mut failure) => {
                failure.detail = format!("can't play: {}", failure.detail);
                self.fail(failure, true);
                false
            }
            None => true,
        }
    }

    /// The other peer refuses: why, as this player is told it.
    fn heard_refusal(&mut self, body: &[u8]) {
        let mut r = Reader::new(body);
        let Ok(theirs) = self.compat.read(&mut r) else { return };
        // (A peer of another protocol: whatever it refuses for, in whatever
        // words, that is why.)
        if theirs.protocol != self.compat.protocol {
            let detail = format!("the other side refused, speaking netplay protocol {} (this one speaks {})", theirs.protocol, self.compat.protocol);
            return self.fail(Failure { reason: Reason::DifferentVersion, detail }, false);
        }
        let (Ok(reason), Ok(detail)) = (r.get::<u8>(), r.get::<String>()) else { return };
        let reason = Refusal::of(reason).map_or(Reason::Protocol, |r| r.heard(&theirs));
        self.fail(Failure { reason, detail: format!("the other side refused: {detail}") }, false);
    }

    fn heard_lobby(&mut self, now: Instant, body: &[u8]) {
        let mut r = Reader::new(body);
        let Ok(compat) = self.compat.read(&mut r) else { return };
        if !self.compatible(&compat) {
            return;
        }
        let (Ok(revision), Ok(ready), Ok(stamp), Ok(settings), Ok(name), Ok(clock), Ok(echoes)) =
            (r.get::<u32>(), r.get::<bool>(), r.get::<u32>(), r.bytes(), r.get::<String>(), r.get::<u32>(), r.get::<bool>())
        else {
            return;
        };
        // The round trip: their clock, to say back; and this peer's, said
        // back with how long they held it.
        self.their_clock = Some((clock, now));
        if echoes && let (Ok(mine), Ok(held)) = (r.get::<u32>(), r.get::<u32>()) {
            let sample = self.clock(now).wrapping_sub(mine).saturating_sub(held);
            // (A clock said back from before this lobby's is nothing.)
            if sample < self.timeout.as_millis() as u32 {
                self.ping = Some(Ping::after(self.ping, sample as f64));
            }
        }
        if self.settled() || self.theirs.as_ref().is_some_and(|t| revision <= t.revision) {
            return;
        }
        let settings = match Settings::from_bytes(&self.content, settings) {
            Ok(settings) => settings,
            Err(why) => return self.fail(Failure { reason: Reason::Protocol, detail: format!("can't play: the other side's settings don't read here: {why}") }, true),
        };
        // The settings are the match's: theirs of a higher stamp are
        // adopted, and of the same stamp where they differ, by the joiner.
        let adopts = stamp > self.stamp || (stamp == self.stamp && settings != self.mine && self.compat.role == Role::Join);
        if adopts {
            let differ = settings != self.mine;
            self.stamp = stamp;
            self.mine = settings.clone();
            // (Said back at once, so the other peer sees them adopted; a
            // player who was ready was ready for the settings before.)
            self.change(self.ready && !differ);
        }
        self.theirs = Some(Theirs { revision, stamp, settings, ready, name: player_name(&name) });
        // (A commitment stands while the other's state is the one agreed.)
        if let Phase::Hello(c) | Phase::Reveal(c, _) = &self.phase
            && !(ready && c.revisions.1 == revision)
        {
            self.back_to_lobby();
        }
    }

    fn heard_hello(&mut self, body: &[u8]) {
        let mut r = Reader::new(body);
        let Ok(compat) = self.compat.read(&mut r) else { return };
        if !self.compatible(&compat) {
            return;
        }
        let (Ok(theirs), Ok(mine), Ok(rounds), Ok(commitment)) = (r.get::<u32>(), r.get::<u32>(), r.get::<u16>(), r.bytes()) else { return };
        let Ok(commitment) = <[u8; 32]>::try_from(commitment) else { return };
        match &self.phase {
            // After the match is agreed: the other hasn't our Reveal.
            Phase::Done(a) => self.outgoing.push(a.answer.clone()),
            _ => self.their_hello = Some(((theirs, mine), rounds, commitment)),
        }
    }

    fn heard_reveal(&mut self, body: &[u8]) {
        match &self.phase {
            Phase::Done(a) => self.outgoing.push(a.answer.clone()),
            _ => self.their_reveal = Some(body.to_vec()),
        }
    }

    fn heard_loading(&mut self, body: &[u8]) {
        let mut r = Reader::new(body);
        let (Ok(seed), Ok(parts), Ok(loaded)) = (r.get::<u32>(), r.get::<u8>(), r.get::<bool>()) else { return };
        // (Loading only goes on: a datagram that comes late takes nothing
        // back.)
        let before = self.their_loading.filter(|(s, ..)| *s == seed);
        let (parts, loaded) = before.map_or((parts, loaded), |(_, p, l)| (parts.max(p), loaded || l));
        self.their_loading = Some((seed, parts, loaded));
    }

    /// This peer's clock, in milliseconds since its lobby began, at `now`.
    fn clock(&self, now: Instant) -> u32 {
        now.duration_since(self.start.unwrap_or(now)).as_millis() as u32
    }

    /// Where it stands at `now`, having said what is due (`outgoing`).
    pub fn poll(&mut self, now: Instant) -> Status<'_> {
        let start = *self.start.get_or_insert(now);
        if !self.started && !matches!(self.phase, Phase::Failed(_)) {
            match self.phase {
                Phase::Done(_) => self.load(now),
                _ => self.step(now),
            }
            if !self.started && !matches!(self.phase, Phase::Failed(_)) && now.duration_since(self.heard.unwrap_or(start)) >= self.timeout {
                self.fail(Failure { reason: Reason::NoAnswer, detail: "no answer from the other side".into() }, false);
            }
        }
        match &self.phase {
            Phase::Done(a) if self.started => Status::Start(a),
            Phase::Done(a) => Status::Loading(a),
            Phase::Failed(failure) => Status::Failed(failure),
            _ => Status::Pending,
        }
    }

    fn step(&mut self, now: Instant) {
        // Agreed in the lobby: commit.
        if let Phase::Lobby = self.phase
            && let Some(t) = &self.theirs
            && t.ready
            && self.ready
            && t.stamp == self.stamp
            && t.settings == self.mine
        {
            let committed = self.commit(t.revision);
            self.phase = Phase::Hello(committed);
            self.changed = true;
        }
        // The other's Hello, once it is of this agreement: reveal.
        if let Phase::Hello(c) = &self.phase
            && let Some(((theirs, mine), rounds, commitment)) = &self.their_hello
            && (*mine, *theirs) == c.revisions
            && *rounds as usize == c.settings.rounds.len()
        {
            let (c, commitment) = (c.clone(), *commitment);
            self.outgoing.push(c.reveal.clone());
            self.phase = Phase::Reveal(c, commitment);
        }
        // The other's Reveal, if it is of this agreement (one of another,
        // taken back since, came late), checked against its commitment.
        if let Phase::Reveal(c, commitment) = &self.phase
            && let Some(reveal) = self.their_reveal.take()
            && revealed_for(&reveal) == Some((c.revisions.1, c.revisions.0))
        {
            let (c, commitment) = (c.clone(), *commitment);
            match self.agree(&c, &commitment, &reveal) {
                Ok(a) => {
                    self.phase = Phase::Done(Box::new(a));
                    // (The loading is said at once.)
                    self.changed = true;
                }
                Err(mut failure) => {
                    failure.detail = format!("can't play: {}", failure.detail);
                    self.fail(failure, true);
                }
            }
            return;
        }
        // What is due: the lobby state when it changed and every RESEND,
        // with the Hello and the Reveal this peer is at.
        if self.changed || self.sent.is_none_or(|t| now.duration_since(t) >= RESEND) {
            self.outgoing.push(self.lobby_datagram(now));
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

    /// The match is agreed: say how far this host's loading is, when it
    /// changed and every RESEND, until both have loaded; then it starts.
    fn load(&mut self, now: Instant) {
        let Phase::Done(a) = &self.phase else { return };
        if self.changed || self.sent.is_none_or(|t| now.duration_since(t) >= RESEND) {
            let mut out = vec![Kind::Loading as u8];
            let mut w = Writer(&mut out);
            w.put(&a.seed);
            w.put(&self.loading.0);
            w.put(&self.loading.1);
            self.outgoing.push(out);
            self.changed = false;
            self.sent = Some(now);
        }
        if self.loading.1 && self.their_loaded() {
            self.started = true;
        }
    }

    fn lobby_datagram(&self, now: Instant) -> Vec<u8> {
        let mut out = vec![Kind::Lobby as u8];
        let mut w = Writer(&mut out);
        self.compat.write(&mut w);
        w.put(&self.revision);
        w.put(&self.ready);
        w.put(&self.stamp);
        w.bytes(&self.mine.to_bytes(&self.content));
        w.put(&self.name);
        // The round trip: this peer's clock, and the other's latest said
        // back with how long it was held here.
        w.put(&self.clock(now));
        w.put(&self.their_clock.is_some());
        if let Some((clock, at)) = self.their_clock {
            w.put(&clock);
            w.put(&(now.duration_since(at).as_millis() as u32));
        }
        out
    }

    /// This peer's commitment to the agreed settings (the other's lobby at
    /// `theirs`): its Reveal (the revisions it is of, the nonce and its
    /// side), and its Hello.
    fn commit(&self, theirs: u32) -> Committed {
        let hash = self.mine.hash(&self.content);
        let mut reveal = vec![Kind::Reveal as u8];
        let mut w = Writer(&mut reveal);
        w.put(&self.revision);
        w.put(&theirs);
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
    fn agree(&self, c: &Committed, theirs: &[u8; 32], reveal: &[u8]) -> Result<Agreement, Failure> {
        let failed = |reason: Reason, detail: String| Failure { reason, detail };
        if commitment(&c.hash, reveal) != *theirs {
            return Err(failed(Reason::Protocol, "the other side's Reveal doesn't match its commitment".into()));
        }
        let mut r = Reader::new(reveal);
        let bad = |e: std::io::Error| failed(Reason::Protocol, format!("the other side's Reveal doesn't decode ({e})"));
        let _revisions: (u32, u32) = (r.get().map_err(bad)?, r.get().map_err(bad)?);
        let nonce: u64 = r.get().map_err(bad)?;
        let side_bytes = r.bytes().map_err(bad)?;
        r.finish().map_err(bad)?;
        let side = nettai_match::binary::read_side(&self.content, side_bytes).map_err(|e| failed(Reason::Protocol, format!("the other player's side doesn't decode ({e})")))?;
        let problems = nettai_match::check_side(&self.content, &c.settings.game, &c.settings.rounds, &side);
        if !problems.is_empty() {
            return Err(failed(Reason::TheirBuild, format!("the other player's side breaks the rules: {}", problems.join("; "))));
        }
        if side.folder(&self.content).saved().is_none() {
            return Err(failed(Reason::TheirBuild, "the other player's folder isn't whole".into()));
        }
        let me = self.compat.role.side();
        let (host, join) = if me == 0 { (self.nonce, nonce) } else { (nonce, self.nonce) };
        let seed = seed(host, join);
        let sides = if me == 0 { [self.side.clone(), side] } else { [side, self.side.clone()] };
        let (set, m) = crate::netplay::netplay_setup(&self.content, seed, &c.settings, sides.clone()).map_err(|why| failed(Reason::Protocol, why))?;
        let their_name = self.their_name().unwrap_or("").to_string();
        let names = if me == 0 { [self.name.clone(), their_name] } else { [their_name, self.name.clone()] };
        Ok(Agreement { settings: c.settings.clone(), seed, sides, side: me, set, m, names, answer: c.reveal.clone() })
    }
}

/// The lobby revisions a Reveal (its bytes, without its kind byte) says it
/// is of: its sender's, then the other peer's.
fn revealed_for(reveal: &[u8]) -> Option<(u32, u32)> {
    let mut r = Reader::new(reveal);
    Some((r.get().ok()?, r.get().ok()?))
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
    /// `rounds` times or until neither is still agreeing; `tamper` may
    /// change what goes from one to the other (by the sender's side; a
    /// datagram it empties is lost).
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

    /// Two peers holding these settings with these sides (by side: the
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

    /// Both peers have loaded, and say so: the match starts on both.
    pub fn load(lobbies: &mut [Lobby; 2], now: Instant) {
        for l in lobbies.iter_mut() {
            assert!(matches!(l.poll(now), Status::Loading(_)), "not agreed");
            l.set_loaded();
        }
        run(lobbies, now, 4, &mut |_, _| {});
    }

    /// The match a lobby starts.
    pub fn started(lobby: &mut Lobby, now: Instant) -> Agreement {
        match lobby.poll(now) {
            Status::Start(a) => a.clone(),
            Status::Failed(failure) => panic!("{}", failure.detail),
            Status::Loading(_) => panic!("not loaded"),
            Status::Pending => panic!("not agreed"),
        }
    }

    /// What both agreed, by side, from these settings (both holding them)
    /// and sides, once both have loaded.
    pub fn agree(content: &Arc<Content>, settings: &Settings, sides: [Side; 2]) -> [Agreement; 2] {
        let now = Instant::now();
        let mut lobbies = pair(content, [settings.clone(), settings.clone()], sides);
        run(&mut lobbies, now, 20, &mut |_, _| {});
        load(&mut lobbies, now);
        lobbies.map(|mut l| started(&mut l, now))
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{agree, load, pair, run, started};
    use super::*;
    use crate::connection::Quality;
    use nettai_match::{Picks, TRIPLE_BATTLE};

    fn side(content: &Arc<Content>, seed: u32) -> Side {
        Side::picked(content, content.game(), &mut Picks::new(seed)).unwrap()
    }

    /// A side of another navi than the game's own, with its own folder.
    fn elecman(content: &Arc<Content>) -> Side {
        let elecman = nettai_match::ids::navi(content, "exe6", "elecman").unwrap();
        let mut other = side(content, 5);
        other.set_navi(content, elecman).unwrap();
        other.set_level(content, Some(10)).unwrap();
        // (A link navi has no Crosses.)
        other.set_fact(content, "crosses", &[]).unwrap();
        assert_eq!(nettai_match::check::check_side_alone(content, "exe6", &other), Vec::<String>::new());
        other
    }

    fn settings(rounds: usize) -> Settings {
        Settings::new("exe6", rounds)
    }

    /// Why a lobby failed.
    fn failed(lobby: &mut Lobby, now: Instant) -> Failure {
        match lobby.poll(now) {
            Status::Failed(failure) => failure.clone(),
            _ => panic!("the lobby hasn't failed"),
        }
    }

    /// Each player's name goes to the other, cut to its length and without
    /// control characters, and the agreement holds both by side; a name
    /// changes nobody's readiness.
    #[test]
    fn names_go_to_the_other_player() {
        let content = nettai_match::testing::exe6_content();
        let now = Instant::now();
        let s = settings(TRIPLE_BATTLE);
        let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
        assert!(!lobbies[0].their_present());
        lobbies[0].set_name("  Lan\u{7}\u{1b}[31m ");
        lobbies[1].set_name("Chaud Blaze, of Officials");
        lobbies[1].set_ready(false);
        run(&mut lobbies, now, 3, &mut |_, _| {});
        assert!(lobbies[0].their_present() && lobbies[1].their_present());
        assert_eq!(lobbies[1].their_name(), Some("Lan[31m"));
        assert_eq!(lobbies[0].their_name(), Some("Chaud Blaze, of"));
        assert!(lobbies[0].ready(), "a name changes no readiness");
        lobbies[1].set_ready(true);
        run(&mut lobbies, now, 20, &mut |_, _| {});
        load(&mut lobbies, now);
        let names: Vec<[String; 2]> = lobbies.iter_mut().map(|l| started(l, now).names).collect();
        assert_eq!(names[0], ["Lan[31m".to_string(), "Chaud Blaze, of".to_string()]);
        assert_eq!(names[1], names[0]);
        assert_eq!(player_name("\u{0}\u{0}"), "");
    }

    /// Builds are private: until the Reveal, what a peer sends is the same
    /// whatever build its player brings (its Hello but for the commitment),
    /// and changing the build says nothing. The Reveal then brings the
    /// side, and nothing sent at any point holds a build's name: a side has
    /// none.
    #[test]
    fn nothing_of_a_build_crosses_before_the_reveal() {
        let content = nettai_match::testing::exe6_content();
        let now = Instant::now();
        let s = settings(TRIPLE_BATTLE);
        // The host's datagrams by kind, with one build and with another (a
        // different navi, a different folder), everything else the same.
        let said = |host: Side| -> [Vec<Vec<u8>>; 6] {
            let mut lobbies = pair(&content, [s.clone(), s.clone()], [host, side(&content, 4)]);
            lobbies[0].set_name("Lan");
            let mut out: [Vec<Vec<u8>>; 6] = Default::default();
            run(&mut lobbies, now, 20, &mut |from, d| {
                if from == 0 {
                    out[d[0] as usize].push(d.clone());
                }
            });
            assert!(matches!(lobbies[0].poll(now), Status::Loading(_)));
            out
        };
        let megaman = side(&content, 3);
        let (a, b) = (said(megaman.clone()), said(elecman(&content)));
        assert!(!a[Kind::Lobby as usize].is_empty() && !a[Kind::Hello as usize].is_empty() && !a[Kind::Reveal as usize].is_empty());
        assert_eq!(a[Kind::Lobby as usize], b[Kind::Lobby as usize], "the lobby's messages say nothing of the build");
        // (A Hello ends with its commitment, 32 bytes, which is of the
        // Reveal: of the nonce and the side.)
        let open = |hellos: &[Vec<u8>]| -> Vec<Vec<u8>> { hellos.iter().map(|h| h[..h.len() - 32].to_vec()).collect() };
        assert_eq!(open(&a[Kind::Hello as usize]), open(&b[Kind::Hello as usize]));
        assert_ne!(a[Kind::Hello as usize], b[Kind::Hello as usize]);
        assert_ne!(a[Kind::Reveal as usize], b[Kind::Reveal as usize], "the Reveal brings the side");
        // Nothing before the Reveal names a navi, in either.
        for d in a[Kind::Lobby as usize].iter().chain(&a[Kind::Hello as usize]).chain(&b[Kind::Lobby as usize]).chain(&b[Kind::Hello as usize]) {
            let text = String::from_utf8_lossy(d);
            assert!(!text.contains("megaman") && !text.contains("elecman"), "{text}");
        }
        // A player who isn't ready changes their build: nothing is said (the
        // same revision, the same message). One who is ready is no longer,
        // which is all the other sees; the other's own mark stays.
        let mut lobbies = pair(&content, [s.clone(), s.clone()], [megaman, side(&content, 4)]);
        lobbies[0].set_ready(false);
        run(&mut lobbies, now, 3, &mut |_, _| {});
        let before = lobbies[0].lobby_datagram(now);
        lobbies[0].set_side(elecman(&content));
        assert_eq!(lobbies[0].lobby_datagram(now), before);
        lobbies[0].set_ready(true);
        lobbies[0].set_side(side(&content, 9));
        run(&mut lobbies, now, 3, &mut |_, _| {});
        assert!(!lobbies[0].ready() && !lobbies[1].their_ready(), "a new build clears its player's ready");
        assert!(lobbies[1].ready() && lobbies[0].their_ready(), "and not the other's");
        // The side revealed is the one brought last.
        lobbies[0].set_side(elecman(&content));
        lobbies[0].set_ready(true);
        run(&mut lobbies, now, 20, &mut |_, _| {});
        load(&mut lobbies, now);
        for l in &mut lobbies {
            let a = started(l, now);
            assert_eq!(nettai_match::ids::local(&content.defs.navi(a.m.sides[0].navi(&content)).key), "elecman");
        }
    }

    /// Settings go by name and read back, closed builds with them; ones
    /// this content can't play are refused with why.
    #[test]
    fn settings_go_by_name() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 9, None).unwrap();
        let s = Settings::of_match(&m);
        assert_eq!(Settings::from_bytes(&content, &s.to_bytes(&content)).unwrap(), s);
        let closed = Settings { closed_builds: true, ..s.clone() };
        assert_eq!(Settings::from_bytes(&content, &closed.to_bytes(&content)).unwrap(), closed);
        assert_ne!(closed.hash(&content), s.hash(&content), "what both commit to says whether builds are closed");
        let blank = settings(5);
        assert_eq!(Settings::from_bytes(&content, &blank.to_bytes(&content)).unwrap(), blank);
        assert_eq!(settings(1).rounds, vec![RoundSettings::default()]);
        let mut other = s.clone();
        other.game = "exe5".into();
        assert!(Settings::from_bytes(&content, &other.to_bytes(&content)).unwrap_err().contains("no stage"));
        let mut bytes = Vec::new();
        let mut w = Writer(&mut bytes);
        w.put(&"exe6".to_string());
        w.put(&false);
        w.put(&1u16);
        w.put(&"no-such-stage".to_string());
        w.put(&String::new());
        assert_eq!(Settings::from_bytes(&content, &bytes).unwrap_err(), "round 1: no stage no-such-stage in exe6");
        for count in [0, nettai_match::MAX_ROUNDS + 1] {
            let none = Settings { rounds: vec![RoundSettings::default(); count], ..blank.clone() };
            assert_eq!(Settings::from_bytes(&content, &none.to_bytes(&content)).unwrap_err(), format!("{count} rounds: a match has 1 to 99"));
        }
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
            let s = Settings { rounds: rounds.clone(), ..settings(1) };
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

    /// The settings are the match's: a joiner coming into the lobby takes
    /// the host's; either player changes them and the other's lobby shows
    /// the same, both ready marks cleared; where both change them at once
    /// the host's change stands; and what both then ready on is what is
    /// agreed, closed builds reaching both hosts with the agreement.
    #[test]
    fn the_settings_are_shared() {
        let content = nettai_match::testing::exe6_content();
        let now = Instant::now();
        let both = |lobbies: &[Lobby; 2]| -> [Settings; 2] { [lobbies[0].settings().clone(), lobbies[1].settings().clone()] };
        let marks = |lobbies: &[Lobby; 2]| -> [bool; 4] { [lobbies[0].ready(), lobbies[1].ready(), lobbies[0].their_ready(), lobbies[1].their_ready()] };
        // The joiner last played single battles; the host's lobby is a
        // triple battle. The joiner's shows the host's once it hears it,
        // and its player isn't ready for settings they hadn't seen.
        let mut lobbies = pair(&content, [settings(TRIPLE_BATTLE), settings(1)], [side(&content, 3), side(&content, 4)]);
        run(&mut lobbies, now, 4, &mut |_, _| {});
        assert_eq!(both(&lobbies), [settings(TRIPLE_BATTLE), settings(TRIPLE_BATTLE)]);
        assert_eq!(marks(&lobbies), [true, false, false, true], "the joiner readied on other settings");
        assert!(lobbies.iter_mut().all(|l| matches!(l.poll(now), Status::Pending)));
        // The joiner readies; before the host has heard it, the host makes
        // it a single battle: both show that, and both marks are clear.
        lobbies[1].set_ready(true);
        lobbies[0].set_rounds(1);
        assert_eq!(marks(&lobbies)[0], false, "the player who changes them isn't ready for them yet");
        run(&mut lobbies, now, 4, &mut |_, _| {});
        assert_eq!(both(&lobbies), [settings(1), settings(1)]);
        assert_eq!(marks(&lobbies), [false; 4]);
        // Both ready; the joiner closes the builds as the agreement is
        // under way: it is taken back, both show closed builds, both marks
        // are clear again.
        lobbies[0].set_ready(true);
        lobbies[1].set_ready(true);
        for from in [0, 1, 0] {
            let _ = lobbies[from].poll(now);
            for d in lobbies[from].outgoing() {
                lobbies[1 - from].receive(now, &d);
            }
        }
        lobbies[1].set_closed_builds(true);
        run(&mut lobbies, now, 6, &mut |_, _| {});
        let closed = Settings { closed_builds: true, ..settings(1) };
        assert_eq!(both(&lobbies), [closed.clone(), closed.clone()]);
        assert_eq!(marks(&lobbies), [false; 4]);
        assert!(lobbies.iter_mut().all(|l| matches!(l.poll(now), Status::Pending)));
        // Both change them at once (neither has heard the other's change):
        // the host's stands, on both.
        lobbies[0].set_rounds(TRIPLE_BATTLE);
        lobbies[1].set_rounds(5);
        run(&mut lobbies, now, 6, &mut |_, _| {});
        let agreed = Settings { closed_builds: true, ..settings(TRIPLE_BATTLE) };
        assert_eq!(both(&lobbies), [agreed.clone(), agreed.clone()]);
        // One ready isn't an agreement; both are, on what both show.
        lobbies[0].set_ready(true);
        run(&mut lobbies, now, 6, &mut |_, _| {});
        assert_eq!(marks(&lobbies), [true, false, false, true]);
        assert!(lobbies.iter_mut().all(|l| matches!(l.poll(now), Status::Pending)));
        lobbies[1].set_ready(true);
        run(&mut lobbies, now, 20, &mut |_, _| {});
        load(&mut lobbies, now);
        for l in &mut lobbies {
            let a = started(l, now);
            assert_eq!(a.settings, agreed);
            assert!(a.closed_builds() && a.m.rounds.len() == TRIPLE_BATTLE);
        }
    }

    /// A Reveal that isn't what its peer committed to is refused, a
    /// tampered nonce or a tampered side alike: the other peer stops saying
    /// so, and the tampered one hears it.
    #[test]
    fn a_tampered_reveal_is_refused() {
        let content = nettai_match::testing::exe6_content();
        let s = settings(3);
        // (The joiner's Reveal: its kind, the two revisions it is of, then
        // the nonce, then its side.)
        for (what, at) in [("nonce", 3usize), ("side", 14)] {
            let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
            run(&mut lobbies, Instant::now(), 20, &mut |from, d| {
                if from == 1 && d[0] == Kind::Reveal as u8 {
                    d[at] ^= 1;
                }
            });
            let host = failed(&mut lobbies[0], Instant::now());
            assert_eq!((host.reason, host.detail.as_str()), (Reason::Protocol, "can't play: the other side's Reveal doesn't match its commitment"), "{what}");
            let join = failed(&mut lobbies[1], Instant::now());
            assert_eq!((join.reason, join.detail.as_str()), (Reason::Protocol, "the other side refused: can't play: the other side's Reveal doesn't match its commitment"), "{what}");
        }
    }

    /// A Reveal of an agreement that was taken back, which comes late, is
    /// not the next agreement's: it is left, and the two agree when the
    /// next one's comes.
    #[test]
    fn a_reveal_of_an_agreement_taken_back_is_left() {
        let content = nettai_match::testing::exe6_content();
        let now = Instant::now();
        let s = settings(3);
        let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
        // Both commit and reveal; no Reveal arrives (the joiner's first is
        // kept, on its way).
        let mut late = None;
        run(&mut lobbies, now, 6, &mut |from, d| {
            if d[0] == Kind::Reveal as u8 {
                if from == 1 && late.is_none() {
                    late = Some(d.clone());
                }
                d.clear();
            }
        });
        let late = late.expect("the joiner revealed");
        assert!(lobbies.iter_mut().all(|l| matches!(l.poll(now), Status::Pending)));
        // The host's player takes their ready back and gives it again; the
        // old Reveal comes then, and is what the host has of the joiner's
        // when the two have committed again (the joiner's new one is still
        // on its way): it is left, and nobody is refused.
        lobbies[0].set_ready(false);
        lobbies[0].set_ready(true);
        lobbies[0].receive(now, &late);
        run(&mut lobbies, now, 6, &mut |from, d| {
            if from == 1 && d[0] == Kind::Reveal as u8 {
                assert_ne!(*d, late, "a new agreement, a new Reveal");
                d.clear();
            }
        });
        assert!(matches!(lobbies[0].poll(now), Status::Pending), "the host hasn't the joiner's new Reveal");
        assert!(matches!(lobbies[1].poll(now), Status::Loading(_)), "the joiner has the host's");
        // The new one comes (the host says its part again, and the joiner
        // answers it).
        let now = now + RESEND;
        run(&mut lobbies, now, 20, &mut |_, _| {});
        load(&mut lobbies, now);
        let [h, j] = [started(&mut lobbies[0], now), started(&mut lobbies[1], now)];
        assert_eq!((h.seed, &h.m), (j.seed, &j.m));
    }

    /// Peers that can't play together stop, each with a reason its host can
    /// say: another protocol or engine (the program's version), another
    /// game, other content, a build that breaks a rule; one nobody answers
    /// gives up after the timeout, having said its part every RESEND.
    #[test]
    fn peers_that_cant_play_say_why() {
        let content = nettai_match::testing::exe6_content();
        let s = settings(3);
        let now = Instant::now();
        // (Each failed: the joiner, who hears the host first, refuses, and
        // the host hears it.)
        let cant = |change: &dyn Fn(&mut [Lobby; 2])| -> [Failure; 2] {
            let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
            change(&mut lobbies);
            run(&mut lobbies, now, 4, &mut |_, _| {});
            [0, 1].map(|i| failed(&mut lobbies[i], now))
        };
        let v = protocol::VERSION;
        let [h, j] = cant(&|l| l[1].compat.protocol += 1);
        assert_eq!((j.reason, j.detail), (Reason::DifferentVersion, format!("can't play: the other side speaks netplay protocol {v}, this one {}", v + 1)));
        assert_eq!((h.reason, h.detail), (Reason::DifferentVersion, format!("the other side refused, speaking netplay protocol {} (this one speaks {v})", v + 1)));
        let [h, j] = cant(&|l| l[1].compat.engine = "0.0.0".into());
        assert_eq!((j.reason, j.detail), (Reason::DifferentVersion, format!("can't play: the other side runs engine {ENGINE}, this one 0.0.0")));
        assert_eq!((h.reason, h.detail), (Reason::DifferentVersion, format!("the other side refused: can't play: the other side runs engine {ENGINE}, this one 0.0.0")));
        // Another game: each is told the other's.
        let [h, j] = cant(&|l| l[1].compat.game = "exe5".into());
        assert_eq!((j.reason, j.detail.as_str()), (Reason::DifferentGame { theirs: "exe6".into() }, "can't play: the other side plays exe6, this one exe5: a match is of one game"));
        assert_eq!(h.reason, Reason::DifferentGame { theirs: "exe5".into() });
        let [h, j] = cant(&|l| l[1].compat.content = ContentHash(l[1].compat.content.0 ^ 1));
        assert!(j.reason == Reason::DifferentContent && j.detail.starts_with("can't play: the other side plays other content"), "{j:?}");
        assert!(h.reason == Reason::DifferentContent && h.detail.starts_with("the other side refused: can't play: the other side plays other content"), "{h:?}");
        let [h, j] = cant(&|l| l[1].compat.role = Role::Host);
        assert_eq!((j.reason, j.detail.as_str()), (Reason::Protocol, "can't play: both sides host: one hosts and the other joins"));
        assert_eq!(h.reason, Reason::Protocol);
        // A peer of the protocol before this one refuses in words alone:
        // that it is another version is all that is read of it.
        let mut lobby = Lobby::new(Role::Host, &content, s.clone(), side(&content, 3), 7, Duration::from_secs(10));
        let mut old = vec![Kind::Refuse as u8];
        old.extend_from_slice(b"the other side speaks netplay protocol 6, this one 5");
        lobby.receive(now, &old);
        assert_eq!(failed(&mut lobby, now).reason, Reason::DifferentVersion);
        // A build that breaks a rule (a side nothing was chosen for: its
        // folder is empty): the one who brought it is told so, and so is
        // the other.
        let mut lobbies = pair(&content, [s.clone(), s.clone()], [Side::fresh(&content, "exe6").unwrap(), side(&content, 4)]);
        run(&mut lobbies, now, 20, &mut |_, _| {});
        let [h, j] = [0, 1].map(|i| failed(&mut lobbies[i], now));
        assert!(j.reason == Reason::TheirBuild && j.detail.starts_with("can't play: the other player's "), "{j:?}");
        assert!(h.reason == Reason::YourBuild && h.detail.starts_with("the other side refused: can't play: the other player's "), "{h:?}");
        // Nobody answers.
        let timeout = Duration::from_secs(10);
        let mut alone = Lobby::new(Role::Join, &content, s, side(&content, 3), 7, timeout);
        assert!(matches!(alone.poll(now), Status::Pending));
        assert!(matches!(alone.poll(now + RESEND / 2), Status::Pending));
        assert!(matches!(alone.poll(now + RESEND), Status::Pending));
        assert_eq!(alone.outgoing().len(), 2, "said at once, and again after RESEND");
        let none = failed(&mut alone, now + timeout);
        assert_eq!((none.reason, none.detail.as_str()), (Reason::NoAnswer, "no answer from the other side"));
        assert!(alone.outgoing().iter().all(|d| d[0] == Kind::Lobby as u8), "nobody is there to refuse");
    }

    /// A player who leaves the lobby says so: the other's lobby ends at
    /// once, told they left, before the match is agreed and while it loads
    /// alike.
    #[test]
    fn a_player_who_leaves_says_so() {
        let content = nettai_match::testing::exe6_content();
        let now = Instant::now();
        let s = settings(3);
        for agreed in [false, true] {
            let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
            if !agreed {
                lobbies[0].set_ready(false);
            }
            run(&mut lobbies, now, 20, &mut |_, _| {});
            assert_eq!(matches!(lobbies[1].poll(now), Status::Loading(_)), agreed);
            lobbies[1].leave();
            run(&mut lobbies, now, 1, &mut |_, _| {});
            assert_eq!(failed(&mut lobbies[1], now).reason, Reason::Left, "agreed: {agreed}");
            let host = failed(&mut lobbies[0], now);
            assert_eq!((host.reason, host.detail.as_str()), (Reason::OpponentLeft, "the other side refused: this player left the lobby"), "agreed: {agreed}");
        }
    }

    /// Once the match is agreed each host loads, and says how far it is:
    /// the other sees it; the match starts on neither before both have
    /// loaded, and on both once they have. A peer that never hears the
    /// other say it has loaded starts on the other's first frame.
    #[test]
    fn the_match_starts_once_both_have_loaded() {
        let content = nettai_match::testing::exe6_content();
        let now = Instant::now();
        let s = settings(3);
        let loading = |lobbies: &mut [Lobby; 2]| -> [bool; 2] { [0, 1].map(|i| matches!(lobbies[i].poll(now), Status::Loading(_))) };
        let starts = |lobbies: &mut [Lobby; 2]| -> [bool; 2] { [0, 1].map(|i| matches!(lobbies[i].poll(now), Status::Start(_))) };
        let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
        // (Before the match is agreed nothing is loaded, whatever is said.)
        lobbies[0].set_loaded();
        run(&mut lobbies, now, 20, &mut |_, _| {});
        assert_eq!(loading(&mut lobbies), [true, true]);
        assert_eq!((lobbies[0].loading(), lobbies[1].their_loading(), lobbies[1].their_loaded()), (0.0, 0.0, false));
        // Each says how far it is, and the other sees it.
        lobbies[0].set_loading(0.5);
        lobbies[1].set_loading(0.2);
        run(&mut lobbies, now, 2, &mut |_, _| {});
        assert!((lobbies[1].their_loading() - 0.5).abs() < 0.01 && (lobbies[0].their_loading() - 0.2).abs() < 0.01);
        assert!((lobbies[0].loading() - 0.5).abs() < 0.01);
        // The host has loaded; the joiner hasn't: neither starts.
        lobbies[0].set_loaded();
        run(&mut lobbies, now, 4, &mut |_, _| {});
        assert_eq!((lobbies[1].their_loading(), lobbies[1].their_loaded(), lobbies[0].their_loaded()), (1.0, true, false));
        assert_eq!((loading(&mut lobbies), starts(&mut lobbies)), ([true, true], [false, false]));
        // The joiner has: both start, on the same match.
        lobbies[1].set_loaded();
        run(&mut lobbies, now, 4, &mut |_, _| {});
        assert_eq!(starts(&mut lobbies), [true, true]);
        assert_eq!(started(&mut lobbies[0], now).m, started(&mut lobbies[1], now).m);

        // The same, but nothing the host says once it has loaded arrives:
        // the host starts on the joiner's word, the joiner on the host's
        // first frame.
        let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
        run(&mut lobbies, now, 20, &mut |_, _| {});
        lobbies[0].set_loaded();
        lobbies[1].set_loaded();
        run(&mut lobbies, now, 4, &mut |from, d| {
            if from == 0 {
                d.clear();
            }
        });
        assert_eq!(starts(&mut lobbies), [true, false]);
        lobbies[1].receive(now, &[Kind::Frame as u8, 0, 0, 0]);
        assert_eq!(starts(&mut lobbies), [true, true]);
        // (A frame before the match is agreed starts nothing.)
        let mut early = Lobby::new(Role::Join, &content, s, side(&content, 3), 7, Duration::from_secs(10));
        early.receive(now, &[Kind::Frame as u8, 0, 0, 0]);
        assert!(matches!(early.poll(now), Status::Pending) && !early.their_loaded());
    }

    /// The lobby times the round trip by its own messages: over a link
    /// that takes 30 ms each way, both peers measure about 60 ms, steady,
    /// which is an excellent connection; over one that takes 120 ms each
    /// way, about 240 ms, a poor one.
    #[test]
    fn the_lobby_times_the_round_trip() {
        let content = nettai_match::testing::exe6_content();
        for (each_way, quality) in [(30u64, Quality::Excellent), (120, Quality::Poor)] {
            let s = settings(3);
            let mut lobbies = pair(&content, [s.clone(), s.clone()], [side(&content, 3), side(&content, 4)]);
            lobbies[0].set_ready(false);
            assert_eq!(lobbies[0].ping(), None);
            let start = Instant::now();
            // What each said that is on its way, with when it arrives.
            let mut on_the_way: [Vec<(Duration, Vec<u8>)>; 2] = Default::default();
            // (Each peer polls every 5 ms, as a host polls every frame.)
            for step in 0..600u64 {
                let at = Duration::from_millis(5 * step);
                for to in 0..2 {
                    let (due, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut on_the_way[1 - to]).into_iter().partition(|(arrives, _)| *arrives <= at);
                    on_the_way[1 - to] = rest;
                    for (_, d) in due {
                        lobbies[to].receive(start + at, &d);
                    }
                    assert!(matches!(lobbies[to].poll(start + at), Status::Pending));
                    on_the_way[to].extend(lobbies[to].outgoing().into_iter().map(|d| (at + Duration::from_millis(each_way), d)));
                }
            }
            for l in &lobbies {
                let ping = l.ping().expect("timed");
                assert!((ping.ms - 2.0 * each_way as f64).abs() <= 5.0 && ping.jitter_ms <= 5.0, "{each_way} ms each way: {ping:?}");
                assert_eq!(ping.quality(), quality);
            }
        }
    }
}
