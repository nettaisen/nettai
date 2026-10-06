//! Live play against another player over the network (docs/frontend.md):
//! `--match FILE` with `--room CODE`, `--host PORT` or `--join ADDR:PORT`.
//!
//! Each player's frontend runs the whole battle on nettai-netplay's
//! [`Peer`]: a getgud rollback session whose inputs go to the other peer
//! over rennet, in datagrams on a [`Channel`] the host provides (the
//! program's is a WebRTC data channel, nettai-rtc's link): the library has
//! no socket and no transport of its own. A channel that drops and comes
//! back ([`Channel::down_for`]) carries on the same match: both peers wait
//! at the stall guard meanwhile, and rennet sends again what was lost.
//! Before the match, the players'
//! peers agree it in the lobby and the handshake (`crate::lobby`, over the
//! host's datagrams: the program's is nettai-demo's `net`): both run the
//! same engine and play the same content, agree the match's settings (its
//! game and its rounds) and swap their sides (their facts as the game's
//! rules take them, in nettai-match's binary; the language is each
//! player's own) and their halves of the seed, committed before either
//! reveals. Both then play the same set ([`netplay_setup`]): the agreed
//! rounds, each part they leave picked from the seed, each player's side on
//! their side (the host's is side 0, the left navi), by the game's rules.
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
//! channel, so the network side can run on a thread of its own.

use std::sync::Arc;
use std::time::{Duration, Instant};

use nettai_battle::content::Content;
use nettai_battle::cues::{CueAction, CueTracker};
use nettai_battle::{Battle, BattleResult};
use nettai_match::{After, Match, Set, Side};
use nettai_netplay::protocol::BUTTONS;
use nettai_netplay::standin::StandInBattle;
use nettai_netplay::{BattleWorld, Game, Observer, Peer, PeerConfig};

use crate::driver::{Driver, NetStatus, Ran, Step, result_text};

/// The set both players of a match play, and the match: the agreed
/// settings' rounds, every place stated as the match's seed picks those
/// they leave (as a replay keeps them), each player's side on their side
/// (`sides` by side: the host's, then the joiner's; checked ones, their
/// folders whole).
pub fn netplay_setup(content: &Arc<Content>, seed: u32, settings: &crate::lobby::Settings, sides: [Side; 2]) -> Result<(Set, Match), String> {
    let m = Match { game: settings.game.clone(), seed: Some(seed), rounds: settings.rounds.clone(), sides };
    let m = m.stated(content, seed)?;
    Ok((Set::of(content, &m, seed), m))
}

/// The channel a [`NetPlayer`] plays over, which the host provides: it
/// carries the protocol's frames to the other player and theirs back (the
/// program's is a WebRTC data channel opened unordered and without
/// retransmits, nettai-rtc's link, after its handshake). Nothing is assumed
/// of delivery: frames may be lost, reordered or duplicated (rennet
/// recovers). Neither call waits.
pub trait Channel {
    /// Send a frame to the other player; one that can't go now may be
    /// dropped, as the network may drop it. An error ends the match.
    fn send(&mut self, frame: &[u8]) -> Result<(), String>;

    /// The next frame that has come from the other player, if one has. An
    /// error (the network's, or the other side refusing) ends the match.
    fn recv(&mut self) -> Result<Option<&[u8]>, String>;

    /// While the channel is down and the host's transport is making it
    /// again: how long it has been down. The match goes on where it was
    /// when it is back (the frames lost meanwhile are sent again, and both
    /// players waited at the stall guard); meanwhile nothing from the other
    /// player doesn't end it ([`NetOptions::timeout`] counts from when it is
    /// back): giving up is the transport's, an error. None (the default):
    /// up, or a channel that doesn't say.
    fn down_for(&self) -> Option<Duration> {
        None
    }
}

/// How a netplay match plays, as this player chose (none of it goes to the
/// other player).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetOptions {
    /// Ticks the frame shown is behind the player's newest input (getgud's
    /// present delay; [`NetPlayer::set_present_delay`] changes it during
    /// the match). 0 (the default) shows the newest tick, its prediction of
    /// the other player's input corrected by rollback; more shows older,
    /// more often confirmed ticks: fewer rollbacks seen, the player's own
    /// input that much later. The sound is the shown tick's.
    pub present_delay: u32,
    /// The stall guard: inputs ahead of the other player's before a frame
    /// waits for them.
    pub max_lead: u32,
    /// Nothing from the other side this long ends the match (but not while
    /// the channel says it is down and being made again: [`Channel::down_for`]).
    pub timeout: Duration,
}

impl Default for NetOptions {
    fn default() -> NetOptions {
        NetOptions { present_delay: 0, max_lead: 30, timeout: Duration::from_secs(10) }
    }
}

/// What a round's simulation tells the player: what they hear (every tick
/// the peer simulates, through a cue tracker for their side), the tick the
/// round ended on once it settles (the battle the set goes on from: the
/// same on both peers, whenever each sees it settle), and for a recording
/// each tick that settles. The peer's world owns it.
struct Watch {
    viewer: u8,
    tracker: CueTracker,
    /// Every action of the round so far, in order: the player takes the
    /// ones after those it took the frame before.
    actions: Vec<CueAction>,
    /// The first frame simulated so far whose state has the round over,
    /// and that state.
    end: Option<(u32, Battle)>,
    /// The round's end, settled: its frame and its battle, for the set to
    /// go on from. Frames that settle after it are none of the set's.
    settled_end: Option<(u32, Battle)>,
    /// The ticks that settle, for a recording.
    record: Option<Record>,
}

/// What a recording takes of a round's settled ticks.
struct Record {
    /// The set's ticks before this round's (a digest's cadence counts
    /// the set's).
    before: u64,
    /// The digests of frames the latest simulation of which took one, by
    /// frame (a frame due one, and the round's end).
    digests: std::collections::BTreeMap<u32, u64>,
    /// The ticks settled since the player last took them.
    settled: Vec<nettai_replay::Tick>,
}

/// Frames a cue a re-simulation makes again may move and still be the one
/// played (docs/design/rollback.md §3.2).
const CUE_TOLERANCE: u32 = 3;

impl Watch {
    /// A round's, for the player of `viewer`'s side; recording it after
    /// the set's ticks `before` (none: not recording).
    fn new(viewer: u8, before: Option<u64>) -> Watch {
        Watch {
            viewer,
            tracker: CueTracker::new(CUE_TOLERANCE),
            actions: Vec::new(),
            end: None,
            settled_end: None,
            record: before.map(|before| Record { before, digests: Default::default(), settled: Vec::new() }),
        }
    }
}

impl Observer<StandInBattle> for Watch {
    fn rolled_back(&mut self, frame: u32) {
        self.tracker.rolled_back(frame);
        if self.end.as_ref().is_some_and(|(f, _)| *f >= frame) {
            self.end = None;
        }
    }

    fn simulated(&mut self, frame: u32, game: &StandInBattle) {
        let battle = game.battle();
        self.tracker.simulated(frame, battle.sound_cues_for(self.viewer));
        self.actions.extend(self.tracker.drain());
        let ends = self.end.is_none() && battle.round_end().is_some();
        if ends {
            self.end = Some((frame, battle.clone()));
        }
        if let Some(r) = &mut self.record
            && (ends || crate::replay::digest_due(r.before + frame as u64))
        {
            r.digests.insert(frame, battle.digest());
        }
    }

    fn confirmed(&mut self, frame: u32, inputs: [&u16; 2], _: Option<&Battle>) {
        self.tracker.confirmed(frame + 1);
        if self.settled_end.is_some() {
            return;
        }
        let ended = self.end.as_ref().is_some_and(|(f, _)| *f == frame);
        if let Some(r) = &mut self.record {
            let due = ended || crate::replay::digest_due(r.before + frame as u64);
            let digest = r.digests.remove(&frame).filter(|_| due);
            let buttons = inputs.map(|&b| b & nettai_replay::BUTTON_MASK);
            r.settled.push(nettai_replay::Tick { buttons, digest, round_ended: ended, set_ended: false });
        }
        if ended {
            self.settled_end = self.end.take();
        }
    }
}

type NetPeer = Peer<StandInBattle, Watch>;

// A peer, its world and its sound go to another thread, and a player over
// a channel that goes too with them.
const _: () = {
    const fn send<T: Send>() {}
    send::<NetPeer>();
    struct Sendable;
    impl Channel for Sendable {
        fn send(&mut self, _: &[u8]) -> Result<(), String> {
            Ok(())
        }
        fn recv(&mut self) -> Result<Option<&[u8]>, String> {
            Ok(None)
        }
    }
    send::<NetPlayer<Sendable>>();
};

/// Plays a match against another player over the host's channel `C`.
pub struct NetPlayer<C: Channel> {
    channel: C,
    side: usize,
    peer: NetPeer,
    /// The sound's actions of this round the player has taken.
    heard: usize,
    /// When a frame last came from the other player.
    last_frame: Instant,
    /// The set being played.
    set: Set,
    options: NetOptions,
    start: Instant,
    /// The set's result for this player, once it is over.
    over: Option<BattleResult>,
    /// Recording: the set's ticks before this round's, and the settled
    /// ticks the session hasn't taken.
    recording: Option<(u64, Vec<nettai_replay::Tick>)>,
}

impl<C: Channel> NetPlayer<C> {
    /// The match on `channel` (after the host's handshake) for the player
    /// of `side` (the host's is 0): the set both players agreed on
    /// ([`agree`]).
    pub fn new(channel: C, side: usize, set: Set, options: NetOptions) -> NetPlayer<C> {
        let config = PeerConfig::new(options.present_delay, options.max_lead);
        let world = BattleWorld::with_observer(StandInBattle::new(set.start()), side, Watch::new(side as u8, None));
        let start = Instant::now();
        NetPlayer { channel, side, peer: Peer::new(world, config), heard: 0, last_frame: start, set, options, start, over: None, recording: None }
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

    /// Show the frame `ticks` behind the player's newest input from the next
    /// frame on ([`NetOptions::present_delay`]).
    pub fn set_present_delay(&mut self, ticks: u32) {
        self.options.present_delay = ticks;
        self.peer.set_present_delay(ticks);
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
            match self.channel.recv() {
                Ok(Some(frame)) => {
                    self.last_frame = Instant::now();
                    self.peer.receive(frame, now).map_err(|e| format!("netplay stopped: {e}"))?;
                }
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
        // (The channel is being made again: its transport gives up, not
        // this.)
        if self.channel.down_for().is_some() {
            self.last_frame = Instant::now();
        }
        if self.last_frame.elapsed() > self.options.timeout {
            return Err(format!("nothing from the other player for {} seconds: the connection is lost", self.options.timeout.as_secs()));
        }
        let mut ran = Ran::default();
        let decided = self.over.is_none() && self.peer.wait().is_none();
        if decided {
            self.peer.decide(keys & BUTTONS);
        }
        let datagram = self.peer.datagram(now);
        self.channel.send(&datagram).map_err(|e| format!("netplay stopped: {e}"))?;
        if decided {
            let side = self.side as u8;
            self.peer.advance(|advanced| {
                shown.clone_from(advanced.frame.state.battle());
                shown.setup.local_side = side;
            });
            ran.advanced = true;
            // What settled: the ticks, for a recording; the round's end at
            // the tick it ended on, and how the set goes on from it (the
            // next round is the shared simulation's, side 0's; the result
            // is this player's). (The world has told the sound.)
            let watch = self.peer.session_mut().world_mut().observer_mut();
            if let (Some(r), Some((_, recorded))) = (&mut watch.record, &mut self.recording) {
                recorded.append(&mut r.settled);
            }
            let ended = watch.settled_end.take();
            if let Some((frame, ended)) = ended {
                let after = self.set.after(&ended, side).expect("the round is over");
                let last = self.recording.as_mut().and_then(|(_, r)| r.last_mut());
                match after {
                    After::Over(result) => {
                        self.over = Some(result);
                        if let Some(t) = last {
                            t.set_ended = true;
                        }
                    }
                    After::Round(battle) => {
                        self.take_sound(&mut ran.sound);
                        // A new round, a new tracker; a recording's ticks
                        // counted on.
                        let before = self.recording.as_mut().map(|(before, _)| {
                            *before += frame as u64 + 1;
                            *before
                        });
                        let world = BattleWorld::with_observer(StandInBattle::new((*battle).clone()), self.side, Watch::new(side, before));
                        self.peer.end_round(world);
                        self.heard = 0;
                        self.show(&battle, shown);
                        ran.new_round = true;
                    }
                    // The engine stopped the settled battle: on both peers
                    // alike (a recording marks no end: the set goes no
                    // further).
                    After::Stopped(why) => {
                        if let Some(t) = last {
                            t.round_ended = false;
                        }
                        self.take_sound(&mut ran.sound);
                        return Err(format!("engine stopped at {}: {why}", self.position()));
                    }
                }
            }
        }
        self.take_sound(&mut ran.sound);
        Ok(ran)
    }
}

impl<C: Channel> Driver for NetPlayer<C> {
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

    /// From the set's start: each tick as it settles (both peers' files
    /// alike, but for their info).
    fn record(&mut self) -> bool {
        if self.peer.round() > 0 || self.peer.session().local_frontier() > 0 {
            return false;
        }
        self.recording = Some((0, Vec::new()));
        self.peer.session_mut().world_mut().observer_mut().record = Some(Record { before: 0, digests: Default::default(), settled: Vec::new() });
        true
    }

    fn take_recorded(&mut self, out: &mut Vec<nettai_replay::Tick>) {
        if let Some((_, recorded)) = &mut self.recording {
            out.append(recorded);
        }
    }

    fn position(&self) -> String {
        let s = self.peer.session();
        format!("netplay round {} tick {} (settled {})", self.peer.round() + 1, s.local_frontier(), s.settled_state().tick())
    }

    fn net_status(&self) -> Option<NetStatus> {
        let s = self.peer.stats();
        let l = self.peer.link_stats();
        Some(NetStatus {
            ping_ms: l.srtt.map(|r| r as f64),
            loss: l.loss() as f64,
            present_delay: self.options.present_delay,
            last_rollback: s.last_rollback,
            max_rollback: s.max_rollback,
            rollbacks: s.rollbacks,
            waits: s.stalls + s.parked,
            reconnecting: self.channel.down_for(),
        })
    }

    fn result(&self) -> Option<BattleResult> {
        self.over
    }

    fn real_time(&self) -> bool {
        true
    }

    fn set_present_delay(&mut self, ticks: u32) -> bool {
        NetPlayer::set_present_delay(self, ticks);
        true
    }
}

impl<C: Channel> Drop for NetPlayer<C> {
    /// Tell the other player this one leaves (a few times: datagrams get
    /// lost; if all are, the other side's timeout ends its match).
    fn drop(&mut self) {
        self.peer.leave();
        let now = self.now();
        for _ in 0..3 {
            let datagram = self.peer.datagram(now);
            let _ = self.channel.send(&datagram);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::exe6_content as exe6_test_content;
    use nettai_netplay::standin::Masher;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    use crate::lobby::Settings;
    use nettai_match::{Picks, RoundSettings, TRIPLE_BATTLE};

    fn side_of(content: &Arc<Content>, game: &str, seed: u32) -> Side {
        Side::picked(content, game, &mut Picks::new(seed)).unwrap()
    }

    fn side(content: &Arc<Content>, seed: u32) -> Side {
        side_of(content, "exe6", seed)
    }

    /// A triple battle of `game`, every place from the seed.
    fn triple(game: &str) -> Settings {
        Settings { game: game.into(), rounds: vec![RoundSettings::default(); TRIPLE_BATTLE] }
    }

    /// An EXE5 side's facts (its karma, its souls) go through the
    /// handshake: both peers' rounds start from them alike, as they start
    /// from the side itself.
    #[test]
    fn sides_carry_karma_and_souls() {
        use nettai_battle::rules::Fact;
        use nettai_content_api::{Registry, Value};
        let content = nettai_match::testing::exe5_content();
        let mut s = side_of(&content, "exe5", 5);
        s.set_fact(&content, "karma", &[Fact::Value(Value::Int(100))]).unwrap();
        let proto = nettai_match::ids::form(&content, "exe5", "protosoul").unwrap();
        s.set_fact(&content, "souls", &[Fact::Value(Value::Def(Registry::Form, proto.0))]).unwrap();
        let [h, j] = crate::lobby::testing::agree(&content, &triple("exe5"), [s.clone(), side_of(&content, "exe5", 6)]);
        assert_eq!(j.sides[0], s);
        let (direct, _) = netplay_setup(&content, h.seed, &triple("exe5"), [s, side_of(&content, "exe5", 6)]).unwrap();
        assert_eq!(format!("{:?}", h.set.first()), format!("{:?}", j.set.first()));
        assert_eq!(format!("{:?}", h.set.first()), format!("{:?}", direct.first()));
    }

    /// Both players record the set as it settles, over a link with
    /// latency and rollback: their replays are the same but for the side
    /// that recorded, the rounds end on the tick each ended on (whenever
    /// each peer saw it settle), and either plays back to the set's end
    /// with no difference. (Each player shoots; side 1's navi has 1 HP.)
    #[test]
    fn both_players_record_the_same_set() {
        use crate::driver::short_set::shooter;
        use crate::replay::{End, Recorder, play_out, testing::Shared};
        use crate::session::Session;
        let content = exe6_test_content();
        let base_hp = nettai_battle::content::PlayerFact::BaseHp.name();
        let mine = [0, 1].map(|s| {
            let mut d = side(&content, 3 + s as u32);
            if s == 1 {
                d.set_fact(&content, base_hp, &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1))]).unwrap();
            }
            d
        });
        let (mut shuttle, hands) = Shuttle::new();
        let sinks = [Shared::default(), Shared::default()];
        let mut playing: Vec<Session> = Vec::new();
        for (side, (hand, crate::lobby::Agreement { set, m, .. })) in hands.into_iter().zip(agreed(&content, &triple("exe6"), mine)).enumerate() {
            let mut s = Session::new(Box::new(NetPlayer::new(hand, side, set, NetOptions::default())));
            let info = nettai_replay::Info { when: 0, side: side as u8, names: Default::default() };
            s.record(Recorder::new(Box::new(sinks[side].clone()), &content, &m, &info).unwrap()).unwrap();
            playing.push(s);
        }
        let start = Instant::now();
        for frame in 1..40_000u32 {
            shuttle.carry(frame);
            for (side, s) in playing.iter_mut().enumerate() {
                let keys = shooter(&s.battle, side, frame);
                assert!(s.step(keys), "side {side}: {:?}", s.stopped);
            }
            if playing.iter().all(|s| s.driver.result().is_some()) {
                break;
            }
            let next = start + Duration::from_micros(500) * frame;
            std::thread::sleep(next.saturating_duration_since(Instant::now()));
        }
        assert_eq!(playing.iter().map(|s| s.driver.result()).collect::<Vec<_>>(), [Some(BattleResult::Won), Some(BattleResult::Lost)]);
        let [a, b] = sinks.map(|s| nettai_replay::Replay::read(&s.bytes()).unwrap());
        assert_eq!((a.end, a.rounds().len()), (End::Set, 2));
        assert_eq!((&a.head, &a.match_bytes, &a.ticks), (&b.head, &b.match_bytes, &b.ticks));
        assert_eq!((a.info.side, b.info.side), (0, 1));
        let out = play_out(&content, &a).unwrap();
        assert_eq!((out.diverged, out.stopped, out.result, out.rounds), (None, None, Some(BattleResult::Won), vec![Some(0), Some(0)]));
    }

    /// A player's channel on a WebRTC link (nettai-rtc's, as the program's
    /// is, without the handshake's kind byte: the match is agreed in
    /// memory), shared with the test, which cuts its network.
    struct Rtc(Rc<RefCell<nettai_rtc::Link>>, Vec<u8>);

    impl Channel for Rtc {
        fn send(&mut self, frame: &[u8]) -> Result<(), String> {
            self.0.borrow_mut().send(frame).map_err(|e| e.0)
        }

        fn recv(&mut self) -> Result<Option<&[u8]>, String> {
            let mut link = self.0.borrow_mut();
            let Some(frame) = link.recv().map_err(|e| e.0)? else { return Ok(None) };
            self.1 = frame.to_vec();
            Ok(Some(&self.1))
        }

        fn down_for(&self) -> Option<Duration> {
            self.0.borrow().down_for()
        }
    }

    /// Two players on WebRTC links through a room of a signaling server (in
    /// process) play the short set (the host's navi shoots, the joiner's has
    /// 1 HP and stands) and record it; in round one, while both fight, the
    /// joiner's network goes for three seconds. Both see the connection
    /// down (the window's title would say so) and wait; it comes back, a new
    /// connection, and the set goes on to its end in the same session: the
    /// results are the host's win and the joiner's loss, both replays are
    /// the same but for who recorded (each tick's buttons, both players',
    /// and the settled states' digests), and one plays back to the set's end
    /// with no difference.
    #[test]
    fn a_set_goes_on_after_the_connection_drops() {
        use crate::driver::short_set;
        use crate::replay::{End, Recorder, play_out, testing::Shared};
        use crate::session::Session;
        use nettai_battle::battle::mode;
        let content = exe6_test_content();
        let m = short_set::of(&content, "exe6", 7);
        let server = nettai_rtc::testing::Server::start();
        let config = nettai_rtc::Config { ice_servers: Vec::new(), loopback: true, silence: Duration::from_secs(1), ..nettai_rtc::Config::default() };
        // (The first in the room hosts: the host is let in first.)
        let mut host = nettai_rtc::Link::room(&server.url(), "a-set", config.clone()).unwrap();
        let start = Instant::now();
        while host.role().is_none() {
            assert!(start.elapsed() < Duration::from_secs(10), "not let in");
            host.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        let links = [host, nettai_rtc::Link::room(&server.url(), "a-set", config).unwrap()].map(|l| Rc::new(RefCell::new(l)));
        let sinks = [Shared::default(), Shared::default()];
        let mut playing: Vec<Session> = Vec::new();
        for (side, crate::lobby::Agreement { set, m, .. }) in agreed(&content, &Settings::of_match(&m), m.sides.clone()).into_iter().enumerate() {
            let channel = Rtc(links[side].clone(), Vec::new());
            let mut s = Session::new(Box::new(NetPlayer::new(channel, side, set, NetOptions::default())));
            let info = nettai_replay::Info { when: 0, side: side as u8, names: Default::default() };
            s.record(Recorder::new(Box::new(sinks[side].clone()), &content, &m, &info).unwrap()).unwrap();
            playing.push(s);
        }
        let outage = Duration::from_secs(3);
        let (mut cut, mut seen_down, mut over) = (None, [false, false], [None, None]);
        let start = Instant::now();
        for frame in 1u32.. {
            assert!(start.elapsed() < Duration::from_secs(120), "the set doesn't end: {:?}", playing.iter().map(|s| s.driver.position()).collect::<Vec<_>>());
            for (side, s) in playing.iter_mut().enumerate() {
                if over[side].is_some() {
                    continue;
                }
                let keys = if side == 0 { short_set::shooter(&s.battle, side, frame) } else { crate::driver::bot_buttons(&s.battle, side, frame) };
                assert!(s.step(keys), "side {side}: {:?}", s.stopped);
                seen_down[side] |= s.driver.net_status().is_some_and(|n| n.reconnecting.is_some());
                if s.driver.result().is_some() {
                    over[side] = Some(frame);
                }
            }
            // The cut: once the host's navi fights in round one.
            let r = &playing[0].battle.round;
            if cut.is_none() && r.mode == mode::FIGHTING && r.battle_time > 0 && r.wins + r.losses == 0 {
                links[1].borrow_mut().outage(outage);
                cut = Some(Instant::now());
            }
            // (Both stay a little once over: the other settles the end.)
            if over.iter().all(|o| o.is_some_and(|f| frame > f + 200)) {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let cut = cut.expect("round one was fought");
        eprintln!("the cut came {:?} in; the set ended {:?} after it, generations {:?}", cut.duration_since(start), cut.elapsed(), links.each_ref().map(|l| l.borrow().generation()));
        assert_eq!(seen_down, [true, true], "both saw the connection down");
        assert!(links.iter().all(|l| l.borrow().generation() > 1 && l.borrow().down_for().is_none()));
        assert_eq!(playing.iter().map(|s| s.driver.result()).collect::<Vec<_>>(), [Some(BattleResult::Won), Some(BattleResult::Lost)]);
        drop(playing);
        let [a, b] = sinks.map(|s| nettai_replay::Replay::read(&s.bytes()).unwrap());
        assert_eq!((a.end, a.rounds().len()), (End::Set, 2));
        assert_eq!((&a.head, &a.match_bytes, &a.ticks), (&b.head, &b.match_bytes, &b.ticks));
        assert_eq!((a.info.side, b.info.side), (0, 1));
        let out = play_out(&content, &a).unwrap();
        assert_eq!((out.diverged, out.stopped, out.result, out.rounds), (None, None, Some(BattleResult::Won), vec![Some(0), Some(0)]));
    }

    type Queue = Rc<RefCell<VecDeque<Vec<u8>>>>;

    /// A player's end of a channel the test shuttles by hand: what the
    /// player sends waits in `sent` until the test takes it, and what the
    /// test delivers waits in `arrived`.
    #[derive(Default)]
    struct Hand {
        sent: Queue,
        arrived: Queue,
        taken: Vec<u8>,
    }

    impl Channel for Hand {
        fn send(&mut self, frame: &[u8]) -> Result<(), String> {
            self.sent.borrow_mut().push_back(frame.to_vec());
            Ok(())
        }

        fn recv(&mut self) -> Result<Option<&[u8]>, String> {
            let Some(frame) = self.arrived.borrow_mut().pop_front() else { return Ok(None) };
            self.taken = frame;
            Ok(Some(&self.taken))
        }
    }

    /// Frames a frame is on its way from one player to the other.
    const LATENCY: u32 = 3;

    /// The frames between two players, shuttled by hand: each arrives
    /// [`LATENCY`] frames after it was sent, in order.
    struct Shuttle {
        /// Each side's queues: what it sent, and what arrived for it.
        ends: [(Queue, Queue); 2],
        /// What each side sent that is on its way, with the frame it
        /// arrives at.
        on_the_way: [VecDeque<(u32, Vec<u8>)>; 2],
    }

    impl Shuttle {
        /// The players' channels, side 0's and side 1's, and the shuttle
        /// between them.
        fn new() -> (Shuttle, [Hand; 2]) {
            let hands = [Hand::default(), Hand::default()];
            let ends = [0, 1].map(|side| (hands[side].sent.clone(), hands[side].arrived.clone()));
            (Shuttle { ends, on_the_way: Default::default() }, hands)
        }

        /// At `frame`: take what each player sent, and deliver what is due.
        fn carry(&mut self, frame: u32) {
            for from in 0..2 {
                let on_the_way = &mut self.on_the_way[from];
                on_the_way.extend(self.ends[from].0.borrow_mut().drain(..).map(|f| (frame + LATENCY, f)));
                while on_the_way.front().is_some_and(|(at, _)| *at <= frame) {
                    self.ends[1 - from].1.borrow_mut().push_back(on_the_way.pop_front().unwrap().1);
                }
            }
        }
    }

    /// What each player agrees, by side, through the lobby and the
    /// handshake (`crate::lobby`), both proposing `settings`, with these
    /// sides (the host's, then the joiner's).
    fn agreed(content: &Arc<Content>, settings: &Settings, sides: [Side; 2]) -> [crate::lobby::Agreement; 2] {
        crate::lobby::testing::agree(content, settings, sides)
    }

    /// What one player of [`pair`] saw: the round, the match, the settled
    /// ticks' digests, how many sounds played, a report, and why it stopped
    /// (none: it settled them all first).
    type Played = (String, Match, Vec<(u32, u64)>, usize, String, Option<String>);

    /// Two players on EXE6's content, each with their own `NetPlayer` on a
    /// channel the test shuttles by hand, agreeing `settings` and bringing
    /// what `sides` makes of the content (the host's, then the joiner's),
    /// mashing with rollback until one has settled `ticks` and leaves; what
    /// each saw, by side. The settled states must agree, and the other must
    /// hear it leave.
    fn pair(ticks: u32, settings: fn(&Arc<Content>) -> Settings, sides: fn(&Arc<Content>, usize) -> Side) -> [Played; 2] {
        /// A player still playing, and what it has seen.
        struct Playing {
            player: NetPlayer<Hand>,
            shown: Battle,
            masher: Masher,
            setup: String,
            m: Match,
            settled: Vec<(u32, u64)>,
            plays: usize,
        }
        let content = exe6_test_content();
        let mine = [0, 1].map(|side| sides(&content, side));
        let (mut shuttle, hands) = Shuttle::new();
        let mut hands = hands.into_iter().enumerate();
        let mut playing = agreed(&content, &settings(&content), mine.clone()).map(|a| {
            let crate::lobby::Agreement { sides: agreed, set, m, .. } = a;
            let (side, hand) = hands.next().unwrap();
            assert_eq!(agreed, mine);
            let setup = format!("{:?}", set.first());
            let mut player = NetPlayer::new(hand, side, set, NetOptions::default());
            let shown = player.start();
            assert_eq!(shown.setup.local_side as usize, side);
            Some(Playing { player, shown, masher: Masher::new(11 + 11 * side as u64), setup, m, settled: Vec::new(), plays: 0 })
        });
        let mut played: [Option<Played>; 2] = [None, None];
        let start = Instant::now();
        for frame in 1.. {
            assert!(frame < 10 * ticks, "stuck at {:?}", playing.iter().flatten().map(|p| p.player.settled()).collect::<Vec<_>>());
            shuttle.carry(frame);
            for side in 0..2 {
                let Some(p) = &mut playing[side] else { continue };
                let left = match p.player.run_frame(p.masher.buttons(), &mut p.shown).unwrap() {
                    Ok(ran) => {
                        p.plays += ran.sound.iter().filter(|a| matches!(a, CueAction::Play(_))).count();
                        if ran.advanced {
                            p.settled.push(p.player.settled());
                        }
                        None
                    }
                    Err(why) => Some(why),
                };
                if left.is_none() && p.player.settled().0 < ticks {
                    continue;
                }
                // It stops, and leaves (the other hears it).
                let Playing { player, setup, m, settled, plays, .. } = playing[side].take().unwrap();
                let (s, l) = player.stats();
                let report = format!(
                    "side {side}: settled {} ticks, rollbacks {} (deepest {}), waits {}, {plays} plays; {} datagrams of {:.1} bytes; {}",
                    player.settled().0,
                    s.rollbacks,
                    s.max_rollback,
                    s.stalls + s.parked,
                    l.sent,
                    l.mean_size(),
                    left.as_deref().unwrap_or("left first"),
                );
                played[side] = Some((setup, m, settled, plays, report, left));
            }
            if playing.iter().all(Option::is_none) {
                break;
            }
            // (Each frame takes a little time, as a host's does.)
            let next = start + Duration::from_millis(2) * frame;
            std::thread::sleep(next.saturating_duration_since(Instant::now()));
        }
        let [hosted, joined] = played.map(Option::unwrap);
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

    /// Two players, each bringing a side drawn from their seed: the same
    /// round on both (the field from the shared seed, each player's own
    /// side), the settled states agree, and each player hears the battle.
    #[test]
    fn two_players_over_a_channel() {
        pair(900, |_| triple("exe6"), |content, s| side(content, 11 + 11 * s as u32));
    }

    /// What one player of [`set_pair`] saw: the result, each new round's
    /// start (the settled tick count it came at, and the simulation's score:
    /// rounds played, side 0's wins and losses), the settled digests by
    /// round and tick, the connection's figures at the end, the present
    /// delay its session ended with, and why it stopped.
    #[derive(Default)]
    struct SetPlayed {
        result: Option<BattleResult>,
        session_present_delay: u32,
        rounds: Vec<(usize, (u8, u8, u8))>,
        settled: Vec<(usize, u32, u64)>,
        status: NetStatus,
        left: Option<String>,
        report: String,
    }

    /// Two players on a channel the test shuttles by hand play a set of
    /// `game` of `rounds` rounds to its end: the host's navi shoots, the
    /// joiner's has 1 HP and stands still (the short set's sides, each
    /// player bringing theirs).
    fn set_pair(game: &'static str, rounds: usize) -> [SetPlayed; 2] {
        use crate::driver::short_set;
        let content = if game == "exe5" { nettai_match::testing::exe5_content() } else { exe6_test_content() };
        // Both agree the match's rounds (the stages of the first three, the
        // rest left to the seed); the host brings the shooter's side, the
        // joiner the side with 1 HP.
        let mut m = short_set::of(&content, game, 7);
        m.rounds.resize(rounds, RoundSettings::default());
        let (mut shuttle, hands) = Shuttle::new();
        let mut hands = hands.into_iter().enumerate();
        let mut playing = agreed(&content, &Settings::of_match(&m), m.sides.clone()).map(|a| {
            let set = a.set;
            let (side, hand) = hands.next().unwrap();
            let mut player = NetPlayer::new(hand, side, set, NetOptions::default());
            let shown = player.start();
            Some((player, shown, SetPlayed::default(), 0))
        });
        let mut played: [Option<SetPlayed>; 2] = [None, None];
        let start = Instant::now();
        for frame in 1u32.. {
            shuttle.carry(frame);
            for side in 0..2 {
                let Some((player, shown, out, after)) = &mut playing[side] else { continue };
                assert!(frame < 40_000, "{game} side {side}: the set doesn't end ({})", player.position());
                let keys = if side == 0 { short_set::shooter(shown, side, frame) } else { crate::driver::bot_buttons(shown, side, frame) };
                // The host changes its present delay during the first
                // round (the joiner keeps 0): its own, kept for the round
                // after, and the settled states still agree.
                if side == 0 && frame == 100 {
                    assert!(Driver::set_present_delay(player, 3));
                }
                let stops = match player.run_frame(keys, shown).unwrap() {
                    Ok(ran) => {
                        if ran.new_round {
                            let r = &shown.round;
                            assert_eq!((r.ticks, shown.setup.local_side as usize), (0, side), "{game}: the next round is shown at its start, from this side");
                            out.rounds.push((out.settled.len(), (r.round, r.wins, r.losses)));
                        } else if ran.advanced {
                            let (tick, digest) = player.settled();
                            out.settled.push((out.rounds.len(), tick, digest));
                        }
                        out.result = player.result();
                        // Over: the host stays a little (the joiner settles
                        // the end from its last inputs), then leaves; the
                        // joiner hears it.
                        if out.result.is_some() {
                            *after += 1;
                        }
                        side == 0 && *after > 200
                    }
                    Err(why) => {
                        out.left = Some(why);
                        true
                    }
                };
                if !stops {
                    continue;
                }
                let (player, _, mut out, _) = playing[side].take().unwrap();
                out.status = player.net_status().expect("a netplay driver has a connection");
                out.session_present_delay = player.peer.session().present_delay();
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
                played[side] = Some(out);
            }
            if playing.iter().all(Option::is_none) {
                break;
            }
            let next = start + Duration::from_millis(2) * frame;
            std::thread::sleep(next.saturating_duration_since(Instant::now()));
        }
        let [hosted, joined] = played.map(Option::unwrap);
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
            let [hosted, joined] = set_pair(game, TRIPLE_BATTLE);
            assert_eq!((hosted.result, joined.result), (Some(BattleResult::Won), Some(BattleResult::Lost)), "{game}");
            // One new round each (2-0 decides the set), with the score the
            // simulation carries (side 0's).
            for p in [&hosted, &joined] {
                assert_eq!(p.rounds.iter().map(|r| r.1).collect::<Vec<_>>(), [(1, 1, 0)], "{game}");
            }
            // (The connection's figures are values: the present delay each
            // player asked for, the host's changed during the match and kept
            // into the second round's session; a round trip measured.)
            assert_eq!(NetOptions::default().present_delay, 0);
            assert_eq!((hosted.status.present_delay, hosted.session_present_delay), (3, 3), "{game}");
            assert_eq!((joined.status.present_delay, joined.session_present_delay), (0, 0), "{game}");
            for p in [&hosted, &joined] {
                assert!(p.status.ping_ms.is_some() && (0.0..=1.0).contains(&p.status.loss), "{game}: {:?}", p.status);
            }
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

    /// A netplay set of five rounds, best of five: the host wins three
    /// straight, which decides it, on both peers alike (rounds two and three
    /// start with the score carried, the settled states agree in every
    /// round).
    #[test]
    fn a_netplay_set_of_five_rounds() {
        let [hosted, joined] = set_pair("exe6", 5);
        assert_eq!((hosted.result, joined.result), (Some(BattleResult::Won), Some(BattleResult::Lost)));
        for p in [&hosted, &joined] {
            assert_eq!(p.rounds.iter().map(|r| r.1).collect::<Vec<_>>(), [(1, 1, 0), (2, 2, 0)]);
        }
        let theirs: std::collections::HashMap<(usize, u32), u64> = joined.settled.iter().map(|&(r, t, d)| ((r, t), d)).collect();
        let mut common = [0; 3];
        for (round, tick, digest) in &hosted.settled {
            if let Some(d) = theirs.get(&(*round, *tick)) {
                assert_eq!(d, digest, "the settled states differ in round {} at tick {tick}", round + 1);
                common[*round] += 1;
            }
        }
        assert!(common.iter().all(|&n| n > 100), "{common:?} ticks compared");
    }

    /// Netplay from match files (`--match`): both players' files state the
    /// same rounds (they agree them), and each brings its left side; both
    /// play that match.
    #[test]
    fn two_players_with_match_files() {
        // Each player's file, as the editor or --save-match writes one.
        fn file(content: &Arc<Content>, seed: u32) -> Match {
            let text = nettai_match::write(content, &nettai_match::pick::live(content, "exe6", seed, None).unwrap());
            nettai_match::parse(content, &text).unwrap()
        }
        let [hosted, _] = pair(600, |content| Settings::of_match(&file(content, 40)), |content, role| file(content, 40 + role as u32).sides[0].clone());
        let content = exe6_test_content();
        let m = &hosted.1;
        assert_eq!(m.rounds, file(&content, 40).rounds, "the agreed rounds, stated");
        assert_eq!(m.sides[0], file(&content, 40).sides[0]);
        assert_eq!(m.sides[1], file(&content, 41).sides[0], "the joiner's left side, on the right");
    }
}
