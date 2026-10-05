//! Live play against another player over the network (docs/frontend.md):
//! `--play --host PORT` or `--play --join ADDR:PORT`.
//!
//! Each player's frontend runs the whole battle on nettai-netplay's
//! [`Peer`]: a getgud rollback session whose inputs go to the other peer
//! over rennet, in UDP datagrams. Before the match, the handshake
//! (`nettai_netplay::transport`) checks that both run the same engine, play
//! the same game (a match is of one) and the same content, and swaps what
//! each player brings (an [`Offer`]: their navi,
//! folder, version, Crosses and patch cards, each by its name in the game;
//! the language is each player's own) and their halves of the seed. Both
//! then build the same round ([`netplay_setup`]): the host's arena (a match
//! file's, else picked from the seed on the host's stage, if it names one),
//! each player's side on their side (the host's is side 0, the left navi),
//! by the game's rules.
//!
//! Every frame the driver takes what arrived, decides the player's buttons
//! for the next tick (unless clock sync or the stall guard holds it), sends
//! its datagram and advances; the frame to show is getgud's presented
//! frame, drawn from the player's side, and the sound is the cue actions a
//! tracker makes of every tick simulated (a cue played on a wrong
//! prediction is taken back). A set's rounds follow each other on the same
//! stream, as the set says they do ([`nettai_match::Set`]).
//!
//! The sound's tracker is the peer's world's observer, which the world tells
//! everything (ticks simulated, rewinds, ticks settled); the player reads
//! its actions through the session. A player is `Send` over a `Send`
//! datagram channel (UDP), so the network side can run on a thread of its
//! own.

use std::sync::Arc;
use std::time::{Duration, Instant};

use nettai_battle::content::Content;
use nettai_battle::cues::{CueAction, CueTracker};
use nettai_battle::{Battle, BattleResult};
use nettai_content_api::StageHandle;
use nettai_match::file::{ArenaFile, SideFile};
use nettai_match::{After, Arena, Match, Picks, Place, Set, Side, ids};
use nettai_netplay::protocol::BUTTONS;
use nettai_netplay::standin::StandInBattle;
use nettai_netplay::transport::{Connection, Datagram, Hello, Role};
use nettai_netplay::{BattleWorld, Game, Observer, Peer, PeerConfig};

use crate::driver::{Driver, Ran, Step, result_text};

/// What a player brings to a netbattle: the match's game (a game is its
/// rules), their side of the match (a match file's left side, or one drawn
/// from their seed), and from the host the arena (a match file's, of the
/// offer's game) or a stage the round must be fought on (`--stage`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub game: String,
    pub side: Side,
    pub stage: Option<StageHandle>,
    pub arena: Option<Arena>,
}

/// An offer as the handshake carries it: a match file's names, in the
/// offer's game (`nettai_match::file`).
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OfferFile {
    game: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    arena: Option<ArenaFile>,
    side: SideFile,
}

impl Offer {
    /// An offer of `side` for a match of `game` (and the host's `stage`,
    /// if it names one).
    pub fn of_side(game: &str, side: Side, stage: Option<StageHandle>) -> Offer {
        Offer { game: game.to_string(), side, stage, arena: None }
    }

    /// An offer of a match file's: its left side, and its arena if this
    /// player hosts.
    pub fn of_match(m: Match, host: bool) -> Offer {
        let [side, _] = m.sides;
        Offer { game: m.arena.game.clone(), side, stage: None, arena: host.then_some(m.arena) }
    }

    /// The offer as the handshake carries it: each thing by its name in
    /// the offer's game.
    pub fn to_bytes(&self, content: &Content) -> Vec<u8> {
        let place = |s: StageHandle| ids::local(&content.defs.stage(s).key).to_string();
        let file = OfferFile {
            game: self.game.clone(),
            stage: self.stage.map(place),
            arena: self.arena.as_ref().map(|a| nettai_match::file::arena_file(content, a)),
            side: nettai_match::file::side_file(content, &self.side),
        };
        toml::to_string(&file).expect("an offer serializes").into_bytes()
    }

    /// An offer from the other side for a match of `game`, its names
    /// resolved in the game and checked against `content` as a match file's
    /// side is (`nettai_match::check_side`), its arena or stage a link
    /// battle's.
    pub fn from_bytes(content: &Arc<Content>, game: &str, bytes: &[u8]) -> Result<Offer, String> {
        let undecoded = |e: String| format!("the other player's setup doesn't decode ({e})");
        let text = std::str::from_utf8(bytes).map_err(|e| undecoded(e.to_string()))?;
        let f: OfferFile = toml::from_str(text).map_err(|e| undecoded(e.to_string()))?;
        if f.game != game {
            return Err(format!("the other player's setup is of {}, this match {game}'s", f.game));
        }
        let mut problems = Vec::new();
        let side = nettai_match::file::resolve_side(content, game, &f.side, "side", &mut problems);
        let stage = match f.stage.as_deref().map(|name| nettai_match::link_stage(content, game, name)) {
            Some(Err(e)) => {
                problems.push(e);
                None
            }
            s => s.and_then(Result::ok),
        };
        let arena = f.arena.as_ref().and_then(|a| nettai_match::file::resolve_arena(content, game, a, &mut problems));
        let Some(side) = side else {
            return Err(format!("the other player's setup breaks the rules: {}", problems.join("; ")));
        };
        if !problems.is_empty() || f.arena.is_some() && arena.is_none() {
            return Err(format!("the other player's setup breaks the rules: {}", problems.join("; ")));
        }
        let offer = Offer { game: game.to_string(), side, stage, arena };
        offer.check(content)?;
        Ok(offer)
    }

    /// The offer is one this content can play.
    pub fn check(&self, content: &Arc<Content>) -> Result<(), String> {
        let arena = match &self.arena {
            Some(a) => a.clone(),
            None => {
                let first = *nettai_match::link_battle_stages(content, &self.game).first().ok_or_else(|| format!("{} has no link battle stage", self.game))?;
                Arena::on(&self.game, Place { stage: first, background: None })
            }
        };
        if arena.game != self.game {
            return Err("the other player's arena is of another game than their setup".into());
        }
        let broken = nettai_match::check::check_arena(content, &arena);
        if !broken.is_empty() {
            return Err(format!("the other player's arena breaks the rules: {}", broken.join("; ")));
        }
        let broken = nettai_match::check_side(content, &arena, &self.side);
        if !broken.is_empty() {
            return Err(format!("the other player's setup breaks the rules: {}", broken.join("; ")));
        }
        if self.side.folder.saved().is_none() {
            return Err("the other player's folder isn't whole".into());
        }
        if self.stage.is_some_and(|s| !nettai_match::link_battle_stages(content, &self.game).contains(&s)) {
            return Err("the other player's stage isn't a link battle stage".into());
        }
        Ok(())
    }
}

/// The set both players of a match play, and the match: the host's arena
/// (else one picked from the match's seed, on the host's stage if it names
/// one), each player's side on their side (`offers` by side: the host's,
/// then the joiner's; checked ones, their folders whole).
pub fn netplay_setup(content: &Arc<Content>, seed: u32, offers: &[Offer; 2]) -> Result<(Set, Match), String> {
    let [host, join] = offers;
    if host.game != join.game {
        return Err(format!("the host plays {}, the joiner {}: a match is of one game", host.game, join.game));
    }
    let arena = match &host.arena {
        Some(a) => a.clone(),
        None => nettai_match::pick::arena(content, &host.game, &mut Picks::new(seed), host.stage)?,
    };
    let m = Match { seed: Some(seed), arena, sides: [host.side.clone(), join.side.clone()] };
    Ok((Set::of(content, &m, seed), m))
}

/// How a netplay match plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetOptions {
    /// Input delay: ticks the presented frame is behind the player's newest
    /// input (getgud's present delay). More delay, fewer rollbacks.
    pub delay: u32,
    /// The stall guard: inputs ahead of the other player's before a frame
    /// waits for them.
    pub max_lead: u32,
    /// Nothing from the other side this long ends the match.
    pub timeout: Duration,
}

impl Default for NetOptions {
    fn default() -> NetOptions {
        NetOptions { delay: 2, max_lead: 30, timeout: Duration::from_secs(10) }
    }
}

/// What the player hears: every tick the peer simulates, through a cue
/// tracker for their side. The peer's world owns it.
struct Sound {
    viewer: u8,
    tracker: CueTracker,
    /// Every action of the round so far, in order: the player takes the
    /// ones after those it took the frame before.
    actions: Vec<CueAction>,
}

/// Frames a cue a re-simulation makes again may move and still be the one
/// played (docs/design/rollback.md §3.2).
const CUE_TOLERANCE: u32 = 3;

impl Sound {
    fn new(viewer: u8) -> Sound {
        Sound { viewer, tracker: CueTracker::new(CUE_TOLERANCE), actions: Vec::new() }
    }
}

impl<G: Game> Observer<G> for Sound {
    fn rolled_back(&mut self, frame: u32) {
        self.tracker.rolled_back(frame);
    }

    fn simulated(&mut self, frame: u32, game: &G) {
        self.tracker.simulated(frame, game.battle().sound_cues_for(self.viewer));
        self.actions.extend(self.tracker.drain());
    }

    fn confirmed(&mut self, frame: u32, _: Option<&Battle>) {
        self.tracker.confirmed(frame + 1);
    }
}

type NetPeer = Peer<StandInBattle, Sound>;

// A peer, its world and its sound go to another thread, and a player over
// UDP with them.
const _: () = {
    const fn send<T: Send>() {}
    send::<NetPeer>();
    send::<NetPlayer<nettai_netplay::transport::Udp>>();
};

/// Plays a match against another player over `D` (UDP, `nettai_netplay::transport::Udp`).
pub struct NetPlayer<D: Datagram> {
    conn: Connection<D>,
    side: usize,
    peer: NetPeer,
    /// The sound's actions of this round the player has taken.
    heard: usize,
    /// The set being played.
    set: Set,
    options: NetOptions,
    start: Instant,
    /// The set's result for this player, once it is over.
    over: Option<BattleResult>,
}

impl<D: Datagram> NetPlayer<D> {
    /// The match on `conn` (after the handshake): the set both players
    /// agreed on ([`agree`]).
    pub fn new(conn: Connection<D>, set: Set, options: NetOptions) -> NetPlayer<D> {
        let side = conn.side();
        let config = PeerConfig::new(options.delay, options.max_lead);
        let world = BattleWorld::with_observer(StandInBattle::new(set.start()), side, Sound::new(side as u8));
        NetPlayer { conn, side, peer: Peer::new(world, config), heard: 0, set, options, start: Instant::now(), over: None }
    }

    /// The side this player plays.
    pub fn side(&self) -> usize {
        self.side
    }

    /// What the peer did: its rollbacks and waits, and its link.
    pub fn stats(&self) -> (&nettai_netplay::peer::PeerStats, &nettai_netplay::link::LinkStats) {
        (self.peer.stats(), self.peer.link_stats())
    }

    /// The settled state's tick and digest (both peers' agree).
    pub fn settled(&self) -> (u32, u64) {
        let s = self.peer.session().settled_state();
        (s.tick(), s.battle().digest())
    }

    /// The set's result for this player, once it is over.
    pub fn result(&self) -> Option<BattleResult> {
        self.over
    }

    fn now(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// The sound's actions since the player last took them.
    fn take_sound(&mut self, into: &mut Vec<CueAction>) {
        let actions = &self.peer.session().world().observer().actions;
        into.extend_from_slice(&actions[self.heard..]);
        self.heard = actions.len();
    }

    /// `battle` as this player sees it: the frontend draws the console of
    /// `setup.local_side`; the simulation's own perspective
    /// (`round.local_side`, side 0's on both peers) stays.
    fn show(&self, battle: &Battle, shown: &mut Battle) {
        shown.clone_from(battle);
        shown.setup.local_side = self.side as u8;
    }

    /// One wall-clock frame.
    fn frame(&mut self, keys: u16, shown: &mut Battle) -> Result<Ran, String> {
        let now = self.now();
        loop {
            match self.conn.try_recv_frame() {
                Ok(Some(datagram)) => self.peer.receive(datagram, now).map_err(|e| format!("netplay stopped: {e}"))?,
                Ok(None) => break,
                Err(e) => return Err(format!("netplay stopped: {e}")),
            }
        }
        if self.peer.remote_left() {
            return Err(match self.over {
                Some(r) => format!("the match is over ({}); the other player left", result_text(r)),
                None => "the other player left the match".into(),
            });
        }
        if self.conn.silence() > self.options.timeout {
            return Err(format!("nothing from the other player for {} seconds: the connection is lost", self.options.timeout.as_secs()));
        }
        let mut ran = Ran::default();
        let decided = self.over.is_none() && self.peer.wait().is_none();
        if decided {
            self.peer.decide(keys & BUTTONS);
        }
        let datagram = self.peer.datagram(now);
        self.conn.send_frame(&datagram).map_err(|e| format!("netplay stopped: {e}"))?;
        if decided {
            let side = self.side as u8;
            self.peer.advance(|advanced| {
                shown.clone_from(advanced.frame.state.battle());
                shown.setup.local_side = side;
            });
            ran.advanced = true;
            // What settled: the round's end, and how the set goes on from
            // it (the next round is the shared simulation's, side 0's; the
            // result is this player's). (The world has told the sound.)
            match self.set.after(self.peer.session().settled_state().battle(), side) {
                None => {}
                Some(After::Over(result)) => self.over = Some(result),
                Some(After::Round(battle)) => {
                    self.take_sound(&mut ran.sound);
                    // A new round, a new tracker.
                    let world = BattleWorld::with_observer(StandInBattle::new((*battle).clone()), self.side, Sound::new(side));
                    self.peer.end_round(world);
                    self.heard = 0;
                    self.show(&battle, shown);
                    ran.new_round = true;
                }
                // The engine stopped the settled battle: on both peers alike.
                Some(After::Stopped(why)) => {
                    self.take_sound(&mut ran.sound);
                    return Err(format!("engine stopped at {}: {why}", self.position()));
                }
            }
        }
        self.take_sound(&mut ran.sound);
        Ok(ran)
    }
}

impl<D: Datagram> Driver for NetPlayer<D> {
    fn start(&mut self) -> Battle {
        let mut shown = self.set.start();
        shown.setup.local_side = self.side as u8;
        shown
    }

    /// (A netplay match runs its own frames.)
    fn next(&mut self, _: &Battle, _: u16) -> Option<Step> {
        None
    }

    fn run_frame(&mut self, keys: u16, shown: &mut Battle) -> Option<Result<Ran, String>> {
        Some(self.frame(keys, shown))
    }

    fn position(&self) -> String {
        let s = self.peer.session();
        format!("netplay round {} tick {} (settled {})", self.peer.round() + 1, s.local_frontier(), s.settled_state().tick())
    }

    fn status(&self) -> Option<String> {
        let s = self.peer.stats();
        let l = self.peer.link_stats();
        let ping = l.srtt.map_or("-".to_string(), |r| format!("{r:.0}MS"));
        let mut line = format!(
            "PING {ping} LOSS {:.0}% DELAY {} ROLLBACK {} MAX {} ({}) WAIT {}",
            l.loss() * 100.0,
            self.options.delay,
            s.last_rollback,
            s.max_rollback,
            s.rollbacks,
            s.stalls + s.parked,
        );
        if let Some(r) = self.over {
            line.push_str(&format!("\nTHE MATCH IS OVER: {}", result_text(r).to_uppercase()));
        }
        Some(line)
    }

    fn real_time(&self) -> bool {
        true
    }
}

impl<D: Datagram> Drop for NetPlayer<D> {
    /// Tell the other player this one leaves (a few times: datagrams get
    /// lost; if all are, the other side's timeout ends its match).
    fn drop(&mut self) {
        self.peer.leave();
        let now = self.now();
        for _ in 0..3 {
            let datagram = self.peer.datagram(now);
            let _ = self.conn.send_frame(&datagram);
        }
    }
}

/// This side's Hello for a match of the offer's game on `content`,
/// offering `offer`.
pub fn hello(role: Role, content: &Content, offer: &Offer) -> Hello {
    let entropy = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0) ^ std::process::id() as u64;
    Hello::new(role, &offer.game, content.hash(), offer.to_bytes(content), entropy)
}

/// After the handshake (which refused another game): both offers by side
/// (the other's checked), the set they play and its match.
pub fn agree<D: Datagram>(content: &Arc<Content>, conn: &Connection<D>, mine: &Offer) -> Result<([Offer; 2], Set, Match), String> {
    let theirs = Offer::from_bytes(content, &mine.game, &conn.theirs().setup)?;
    let offers = if conn.side() == 0 { [mine.clone(), theirs] } else { [theirs, mine.clone()] };
    let (set, m) = netplay_setup(content, conn.seed(), &offers)?;
    Ok((offers, set, m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::exe6_content as exe6_test_content;
    use nettai_netplay::standin::Masher;
    use nettai_netplay::transport::Udp;

    fn offer_of(content: &Arc<Content>, game: &str, seed: u32) -> Offer {
        Offer::of_side(game, Side::picked(content, game, &mut Picks::new(seed)).unwrap(), None)
    }

    fn offer(content: &Arc<Content>, seed: u32) -> Offer {
        offer_of(content, "exe6", seed)
    }

    /// An offer goes over the wire as it is, every name the game's; one
    /// the content can't play is refused with a reason.
    #[test]
    fn offers_roundtrip_and_bad_ones_are_refused() {
        let content = exe6_test_content();
        let mut o = offer(&content, 5);
        o.stage = Some(nettai_match::link_battle_stages(&content, "exe6")[3]);
        o.side.cards = nettai_match::patch_cards(&content, "exe6", "canodumb,-shadow").unwrap_or_default();
        let bytes = o.to_bytes(&content);
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.starts_with("game = \"exe6\"") && !text.contains("exe6:"), "{text}");
        assert_eq!(Offer::from_bytes(&content, "exe6", &bytes).unwrap(), o);
        // A match file's arena goes too.
        let mut a = o.clone();
        a.arena = Some(nettai_match::pick::live(&content, "exe6", 9, None).unwrap().arena);
        assert_eq!(Offer::from_bytes(&content, "exe6", &a.to_bytes(&content)).unwrap(), a);
        let mut bad = o.clone();
        bad.side.folder.chips = [bad.side.folder.chips[0]; 30];
        bad.side.folder.regular = None;
        assert!(Offer::from_bytes(&content, "exe6", &bad.to_bytes(&content)).unwrap_err().contains("breaks the rules"));
        // A name the game hasn't: the ordinary unknown name.
        let bad = text.replacen("navi = \"megaman\"", "navi = \"exe5:megaman\"", 1); // (written in full)
        let e = Offer::from_bytes(&content, "exe6", bad.as_bytes()).unwrap_err();
        assert!(e.contains("side: no navi \"exe5:megaman\" in exe6"), "{e}"); // (written in full)
        // Another game's offer, and bytes that aren't one.
        assert!(Offer::from_bytes(&content, "exe5", &bytes).unwrap_err().contains("is of exe6, this match exe5's"));
        assert!(Offer::from_bytes(&content, "exe6", &bytes[..10]).is_err());
    }

    /// An EXE5 side's offer carries its karma and souls; both peers' rounds
    /// start from them alike. Karma past 1000, or a soul list under rules
    /// without souls, is refused.
    #[test]
    fn offers_carry_karma_and_souls() {
        let content = nettai_match::testing::exe5_content();
        let mut o = offer_of(&content, "exe5", 5);
        o.side.karma = 100;
        o.side.souls = Some(vec![ids::form(&content, "exe5", "protosoul").unwrap()]);
        let back = Offer::from_bytes(&content, "exe5", &o.to_bytes(&content)).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, o);
        let (one, _) = netplay_setup(&content, 9, &[o.clone(), offer_of(&content, "exe5", 6)]).unwrap();
        let (two, _) = netplay_setup(&content, 9, &[back, offer_of(&content, "exe5", 6)]).unwrap();
        assert_eq!(format!("{:?}", one.first()), format!("{:?}", two.first()));
        let mut bad = o.clone();
        bad.side.karma = 1200;
        assert!(Offer::from_bytes(&content, "exe5", &bad.to_bytes(&content)).unwrap_err().contains("karma 1200"));
        let six = exe6_test_content();
        let mut bad = offer(&six, 6);
        bad.side.souls = Some(Vec::new());
        assert!(Offer::from_bytes(&six, "exe6", &bad.to_bytes(&six)).unwrap_err().contains("no Soul Unison"));
        // Offers of two games make no match.
        let mut other = o.clone();
        other.game = "exe6".into();
        let Err(e) = netplay_setup(&content, 9, &[o.clone(), other]) else { panic!("offers of two games made a match") };
        assert_eq!(e, "the host plays exe5, the joiner exe6: a match is of one game");
    }

    /// What one player of [`pair`] saw: the round, the match, the settled
    /// ticks' digests, how many sounds played, a report, and why it stopped
    /// (none: it settled them all first).
    type Played = (String, Match, Vec<(u32, u64)>, usize, String, Option<String>);

    /// Two players on loopback UDP, each in a thread with its own
    /// `NetPlayer` on EXE6's content, offering what `offers` makes of the
    /// content (the host's, then the joiner's), mashing with rollback until
    /// one has settled `ticks` and leaves; what each saw, by side. The
    /// settled states must agree, and the other must hear it leave.
    fn pair(ticks: u32, offers: fn(&Arc<Content>, usize) -> Offer) -> [Played; 2] {
        let host = Udp::host_on("127.0.0.1:0").unwrap();
        let addr = host.local_addr().unwrap();
        let play = move |udp: Udp, role: Role| -> Played {
            let content = exe6_test_content();
            let mine = offers(&content, role as usize);
            let seed = 11 + 11 * role as u32;
            let timeout = Duration::from_secs(20);
            let conn = match role {
                Role::Host => Connection::host(udp, hello(role, &content, &mine), timeout),
                Role::Join => Connection::join(udp, hello(role, &content, &mine), timeout),
            }
            .unwrap();
            let (offers, set, m) = agree(&content, &conn, &mine).unwrap();
            let side = conn.side();
            assert_eq!(offers[side], mine);
            let setup = set.first().clone();
            let mut player = NetPlayer::new(conn, set, NetOptions::default());
            let mut shown = player.start();
            assert_eq!(shown.setup.local_side as usize, side);
            let mut masher = Masher::new(seed as u64);
            let start = Instant::now();
            let (mut settled, mut plays) = (Vec::new(), 0);
            let mut left = None;
            for frame in 1.. {
                assert!(frame < 10 * ticks, "side {side}: stuck at {:?}", player.settled());
                let ran = match player.run_frame(masher.buttons(), &mut shown).unwrap() {
                    Ok(ran) => ran,
                    Err(why) => {
                        left = Some(why);
                        break;
                    }
                };
                plays += ran.sound.iter().filter(|a| matches!(a, CueAction::Play(_))).count();
                if ran.advanced {
                    settled.push(player.settled());
                }
                if player.settled().0 >= ticks {
                    break;
                }
                let next = start + Duration::from_millis(3) * frame;
                std::thread::sleep(next.saturating_duration_since(Instant::now()));
            }
            let (s, l) = player.stats();
            let report = format!(
                "side {side}: settled {} ticks, rollbacks {} (deepest {}), waits {}, {} plays; {} datagrams of {:.1} bytes; {}",
                player.settled().0,
                s.rollbacks,
                s.max_rollback,
                s.stalls + s.parked,
                plays,
                l.sent,
                l.mean_size(),
                left.as_deref().unwrap_or("left first"),
            );
            (format!("{setup:?}"), m, settled, plays, report, left)
        };
        let host = std::thread::spawn(move || play(host, Role::Host));
        let joined = play(Udp::join(addr).unwrap(), Role::Join);
        let hosted = host.join().unwrap();
        eprintln!("{}\n{}", hosted.4, joined.4);
        assert_eq!(hosted.0, joined.0, "the two sides play different rounds");
        assert_eq!(hosted.1, joined.1);
        let theirs: std::collections::HashMap<u32, u64> = joined.2.iter().copied().collect();
        let mut common = 0;
        for (tick, digest) in &hosted.2 {
            if let Some(d) = theirs.get(tick) {
                assert_eq!(d, digest, "the settled states differ at tick {tick}");
                common += 1;
            }
        }
        assert!(common > ticks / 2, "{common} ticks compared");
        assert!(hosted.3 > 0 && joined.3 > 0, "no sound");
        // The one that didn't settle all of them first heard the other leave.
        for left in [&hosted.5, &joined.5].into_iter().flatten() {
            assert_eq!(left, "the other player left the match");
        }
        [hosted, joined]
    }

    /// Two players on loopback UDP, each bringing a side drawn from their
    /// seed: the handshake, the same round on both (the field from the
    /// shared seed, each player's own side), the settled states agree, and
    /// each player hears the battle.
    #[test]
    fn two_players_over_loopback() {
        pair(900, |content, role| offer(content, 11 + 11 * role as u32));
    }

    /// What one player of [`set_pair`] saw: the result, each new round's
    /// start (the settled tick count it came at, and the simulation's score:
    /// rounds played, side 0's wins and losses), the settled digests by
    /// round and tick, the status line at the end, and why it stopped.
    struct SetPlayed {
        result: Option<BattleResult>,
        rounds: Vec<(usize, (u8, u8, u8))>,
        settled: Vec<(usize, u32, u64)>,
        status: String,
        left: Option<String>,
        report: String,
    }

    /// Two players on loopback UDP play a set of `game` to its end: the
    /// host's navi shoots, the joiner's has 1 HP and stands still (the
    /// short set's sides, each player bringing theirs).
    fn set_pair(game: &'static str) -> [SetPlayed; 2] {
        use crate::driver::short_set;
        let host = Udp::host_on("127.0.0.1:0").unwrap();
        let addr = host.local_addr().unwrap();
        let play = move |udp: Udp, role: Role| -> SetPlayed {
            let content = if game == "exe5" { nettai_match::testing::exe5_content() } else { exe6_test_content() };
            // The host brings the match's arena (the stages of every round:
            // the match's seed is the handshake's, another each run) and
            // its left side, the shooter's; the joiner the one with 1 HP.
            let m = short_set::of(&content, game, 7);
            let mine = match role {
                Role::Host => Offer::of_match(m, true),
                Role::Join => Offer::of_side(game, m.sides[1].clone(), None),
            };
            let timeout = Duration::from_secs(20);
            let conn = match role {
                Role::Host => Connection::host(udp, hello(role, &content, &mine), timeout),
                Role::Join => Connection::join(udp, hello(role, &content, &mine), timeout),
            }
            .unwrap();
            let (_, set, _) = agree(&content, &conn, &mine).unwrap();
            let side = conn.side();
            let mut player = NetPlayer::new(conn, set, NetOptions::default());
            let mut shown = player.start();
            let start = Instant::now();
            let mut out = SetPlayed { result: None, rounds: Vec::new(), settled: Vec::new(), status: String::new(), left: None, report: String::new() };
            let mut after = 0;
            for frame in 1u32.. {
                assert!(frame < 40_000, "{game} side {side}: the set doesn't end ({})", player.position());
                let keys = if side == 0 { short_set::shooter(&shown, side, frame) } else { crate::driver::bot_buttons(&shown, side, frame) };
                let ran = match player.run_frame(keys, &mut shown).unwrap() {
                    Ok(ran) => ran,
                    Err(why) => {
                        out.left = Some(why);
                        break;
                    }
                };
                if ran.new_round {
                    let r = &shown.round;
                    assert_eq!((r.ticks, shown.setup.local_side as usize), (0, side), "{game}: the next round is shown at its start, from this side");
                    out.rounds.push((out.settled.len(), (r.round, r.wins, r.losses)));
                } else if ran.advanced {
                    let (tick, digest) = player.settled();
                    out.settled.push((out.rounds.len(), tick, digest));
                }
                out.result = player.result();
                // Over: the host stays a little (the joiner settles the
                // end from its last inputs), then leaves; the joiner hears it.
                if out.result.is_some() {
                    after += 1;
                    if side == 0 && after > 200 {
                        break;
                    }
                }
                let next = start + Duration::from_millis(2) * frame;
                std::thread::sleep(next.saturating_duration_since(Instant::now()));
            }
            out.status = player.status().unwrap_or_default();
            let (s, _) = player.stats();
            out.report = format!(
                "{game} side {side}: {:?}, new rounds at {:?}, {} settled, rollbacks {} (deepest {}), waits {}; {}",
                out.result,
                out.rounds,
                out.settled.len(),
                s.rollbacks,
                s.max_rollback,
                s.stalls + s.parked,
                out.left.as_deref().unwrap_or("left first"),
            );
            out
        };
        let host = std::thread::spawn(move || play(host, Role::Host));
        let joined = play(Udp::join(addr).unwrap(), Role::Join);
        let hosted = host.join().unwrap();
        eprintln!("{}\n{}", hosted.report, joined.report);
        [hosted, joined]
    }

    /// A netplay set goes on after a round and ends, on both peers alike:
    /// round two starts on both with the score carried, the settled states
    /// agree through both rounds, the host has won and the joiner lost, and
    /// each is told so. Both games.
    #[test]
    fn a_netplay_set_goes_on_to_its_end() {
        for game in ["exe6", "exe5"] {
            let [hosted, joined] = set_pair(game);
            assert_eq!((hosted.result, joined.result), (Some(BattleResult::Won), Some(BattleResult::Lost)), "{game}");
            // One new round each (2-0 decides the set), with the score the
            // simulation carries (side 0's).
            for p in [&hosted, &joined] {
                assert_eq!(p.rounds.iter().map(|r| r.1).collect::<Vec<_>>(), [(1, 1, 0)], "{game}");
            }
            assert!(hosted.status.ends_with("THE MATCH IS OVER: YOU WON"), "{game}: {}", hosted.status);
            assert!(joined.status.ends_with("THE MATCH IS OVER: YOU LOST"), "{game}: {}", joined.status);
            assert_eq!(joined.left.as_deref(), Some("the match is over (you lost); the other player left"), "{game}");
            // The settled states agree, in the second round too.
            let theirs: std::collections::HashMap<(usize, u32), u64> = joined.settled.iter().map(|&(r, t, d)| ((r, t), d)).collect();
            let mut common = [0, 0];
            for (round, tick, digest) in &hosted.settled {
                if let Some(d) = theirs.get(&(*round, *tick)) {
                    assert_eq!(d, digest, "{game}: the settled states differ in round {} at tick {tick}", round + 1);
                    common[*round] += 1;
                }
            }
            assert!(common[0] > 100 && common[1] > 100, "{game}: {common:?} ticks compared");
        }
    }

    /// Netplay from match files (`--match`): each player brings their
    /// file's left side, and the host its arena; both play that match.
    #[test]
    fn two_players_over_loopback_with_match_files() {
        // Each player's file, as the editor or --save-match writes one.
        fn file(content: &Arc<Content>, seed: u32) -> Match {
            let text = nettai_match::write(content, &nettai_match::pick::live(content, "exe6", seed, None).unwrap());
            nettai_match::parse(content, &text).unwrap()
        }
        let offers = |content: &Arc<Content>, role: usize| Offer::of_match(file(content, 40 + role as u32), role == 0);
        let [hosted, _] = pair(600, offers);
        let content = exe6_test_content();
        let m = &hosted.1;
        assert_eq!(m.arena, file(&content, 40).arena, "the host's arena");
        assert_eq!(m.sides[0], file(&content, 40).sides[0]);
        assert_eq!(m.sides[1], file(&content, 41).sides[0], "the joiner's left side, on the right");
    }
}
