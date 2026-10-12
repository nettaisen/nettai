//! Live play against another player over the network (docs/frontend.md
//! §2).
//!
//! Each player's frontend runs the whole battle on nettai-netplay's
//! [`Peer`]: a getgud rollback session whose inputs go to the other peer
//! over rennet, in datagrams on a [`Channel`] the host provides (the
//! program's is a WebRTC data channel, nettai-rtc's link): the library has
//! no socket and no transport of its own. A channel that drops and comes
//! back ([`Channel::reconnecting`]) carries on the same match: both peers
//! wait at the stall guard meanwhile, and rennet sends again what was lost.
//! Before the match, the players'
//! peers agree it in the lobby and the handshake (`crate::lobby`, over the
//! host's datagrams: the app's are nettai's `netplay`): both run the
//! same engine and play the same content, agree the match's settings (its
//! game, its rounds, whether builds are closed) and swap their sides (their
//! facts as the game's rules take them, in nettai-match's binary; the
//! language is each player's own) and their halves of the seed, committed
//! before either reveals; then each loads the match, and it starts once
//! both have. Both then play the same set ([`netplay_setup`]): the agreed
//! rounds, each part they leave picked from the seed, each player's side on
//! their side (the host's is side 0, the left navi), by the game's rules.
//!
//! **The input delay** ([`NetOptions::input_delay`], [`DEFAULT_INPUT_DELAY`],
//! [`INPUT_DELAY`]) is each player's own setting, in ticks: how long after
//! a press the player sees it take effect. It is the "input delay" (or
//! "frame delay") of a rollback fighter, and here it is getgud's *present
//! delay*: the peer shows the frame that many ticks behind the player's
//! newest input. The two are the same thing counted from either end. A
//! fighter schedules a press `d` frames ahead and shows the present frame;
//! getgud takes the press for the newest tick and shows the tick `d` behind
//! it. Either way the press is seen `d` frames later, and the other
//! player's input for the frame shown has had `d` frames longer to arrive,
//! so a one-way trip of up to `d` ticks needs no prediction and shows no
//! rollback. There is no second delay under it: the rollback layer (getgud)
//! has that one knob, the engine has no link delay of its own, and the
//! press goes to the other peer in the frame it is made, whatever the
//! delay. It never crosses the wire and changes nothing of the simulation
//! (both peers settle the same ticks from the same inputs whatever each
//! one's delay), so each player sets their own, and may change it during a
//! match.
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

use crate::connection::{Quality, Reconnecting};
use crate::driver::{Driver, NetStatus, Ran, Step, Stop, StopReason, result_text};

/// The input delay a player has who never chose one, in ticks
/// ([`NetOptions::input_delay`]): 4, which hides a one-way trip of up to
/// four ticks (a ping of some 130 ms), and is the delay of the original's
/// own link queue in the recordings (docs/design/rollback.md §1.1).
pub const DEFAULT_INPUT_DELAY: u32 = 4;

/// The input delays a player may choose, in ticks: none (every frame shown
/// is the newest, and every late input of the other player's is predicted)
/// to 10 (a sixth of a second: past that the player's own navi answers too
/// late to play).
pub const INPUT_DELAY: std::ops::RangeInclusive<u32> = 0..=10;

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
    /// again: how long it has been down, and how long the transport goes on
    /// trying, if it says. The match goes on where it was when it is back
    /// (the frames lost meanwhile are sent again, and both players waited at
    /// the stall guard); meanwhile nothing from the other player doesn't end
    /// it ([`NetOptions::timeout`] counts from when it is back): giving up
    /// is the transport's, an error. None (the default): up, or a channel
    /// that doesn't say.
    fn reconnecting(&self) -> Option<Reconnecting> {
        None
    }
}

/// How a netplay match plays, as this player chose (none of it goes to the
/// other player).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetOptions {
    /// The player's input delay, in ticks (the module's docs): the frame
    /// shown is that many behind the player's newest input (getgud's
    /// present delay; [`NetPlayer::set_input_delay`] changes it during the
    /// match). 0 shows the newest tick, its prediction of the other
    /// player's input corrected by rollback; more shows older, more often
    /// confirmed ticks: fewer rollbacks seen, the player's own input that
    /// much later. The sound is the shown tick's. [`DEFAULT_INPUT_DELAY`]
    /// by default; kept within [`INPUT_DELAY`].
    pub input_delay: u32,
    /// The stall guard: inputs ahead of the other player's before a frame
    /// waits for them.
    pub max_lead: u32,
    /// Nothing from the other side this long ends the match (but not while
    /// the channel says it is down and being made again:
    /// [`Channel::reconnecting`]).
    pub timeout: Duration,
}

impl Default for NetOptions {
    fn default() -> NetOptions {
        NetOptions { input_delay: DEFAULT_INPUT_DELAY, max_lead: 30, timeout: Duration::from_secs(10) }
    }
}

/// `ticks` as an input delay: within [`INPUT_DELAY`].
fn input_delay(ticks: u32) -> u32 {
    ticks.clamp(*INPUT_DELAY.start(), *INPUT_DELAY.end())
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
    /// The frames simulated so far, counted once each (a rollback
    /// simulates a frame again).
    seen: u32,
    /// The viewer asked for the pause (`Battle::pause_requests`) on a frame
    /// simulated for the first time since the player last took it: their
    /// own buttons are never a prediction, and what the fight made of them
    /// is the prediction's the picture shows.
    pause_asked: bool,
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
            seen: 0,
            pause_asked: false,
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
        if frame >= self.seen {
            self.seen = frame + 1;
            self.pause_asked |= battle.pause_requests()[self.viewer as usize];
        }
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
    /// The match on `channel` (after the host's lobby says it starts:
    /// `crate::lobby::Status::Start`) for the player of `side` (the host's
    /// is 0): the set both players agreed on (its `Agreement::set`).
    pub fn new(channel: C, side: usize, set: Set, options: NetOptions) -> NetPlayer<C> {
        let options = NetOptions { input_delay: input_delay(options.input_delay), ..options };
        let config = PeerConfig::new(options.input_delay, options.max_lead);
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

    /// The player's input delay from the next frame on, in ticks, kept
    /// within [`INPUT_DELAY`] ([`NetOptions::input_delay`]): this round and
    /// the rounds after.
    pub fn set_input_delay(&mut self, ticks: u32) {
        self.options.input_delay = input_delay(ticks);
        self.peer.set_present_delay(self.options.input_delay);
    }

    /// The player's input delay, in ticks.
    pub fn input_delay(&self) -> u32 {
        self.options.input_delay
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
    fn frame(&mut self, keys: u16, shown: &mut Battle) -> Result<Ran, Stop> {
        // (A channel's error is its transport giving the connection up.)
        let lost = |e: String| Stop::new(StopReason::ConnectionLost, format!("netplay stopped: {e}"));
        let now = self.now();
        loop {
            match self.channel.recv() {
                Ok(Some(frame)) => {
                    self.last_frame = Instant::now();
                    // (A datagram that isn't the protocol's, or input past
                    // the horizon: the other side isn't a peer of this
                    // match any more.)
                    self.peer.receive(frame, now).map_err(|e| Stop::new(StopReason::Internal, format!("netplay stopped: {e}")))?;
                }
                Ok(None) => break,
                Err(e) => return Err(lost(e)),
            }
        }
        if self.peer.remote_left() {
            let detail = match self.over {
                Some(r) => format!("the match is over ({}); the other player left", result_text(r)),
                None => "the other player left the match".into(),
            };
            return Err(Stop::new(StopReason::OpponentLeft, detail));
        }
        // (The channel is being made again: its transport gives up, not
        // this.)
        if self.channel.reconnecting().is_some() {
            self.last_frame = Instant::now();
        }
        if self.last_frame.elapsed() > self.options.timeout {
            return Err(Stop::new(StopReason::ConnectionLost, format!("nothing from the other player for {} seconds: the connection is lost", self.options.timeout.as_secs())));
        }
        let mut ran = Ran::default();
        let decided = self.over.is_none() && self.peer.wait().is_none();
        if decided {
            self.peer.decide(keys & BUTTONS);
        }
        let datagram = self.peer.datagram(now);
        self.channel.send(&datagram).map_err(lost)?;
        if decided {
            let side = self.side as u8;
            self.peer.advance(|advanced| {
                shown.clone_from(advanced.frame.state.battle());
                shown.setup.local_side = side;
            });
            ran.advanced = true;
            ran.pause_requests[self.side] = std::mem::take(&mut self.peer.session_mut().world_mut().observer_mut().pause_asked);
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
                        return Err(Stop::new(StopReason::Internal, format!("engine stopped at {}: {why}", self.position())));
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

    fn run_frame(&mut self, keys: u16, shown: &mut Battle) -> Option<Result<Ran, Stop>> {
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
        // (The link's round trips are in the unit this player times its
        // frames in: milliseconds.)
        Some(NetStatus {
            ping_ms: l.srtt,
            jitter_ms: l.rttvar,
            quality: l.srtt.zip(l.rttvar).map(|(ping, jitter)| Quality::of(ping, jitter)),
            loss: l.loss(),
            input_delay: self.options.input_delay,
            last_rollback: s.last_rollback,
            max_rollback: s.max_rollback,
            rollbacks: s.rollbacks,
            waits: s.stalls + s.parked,
            reconnecting: self.channel.reconnecting(),
        })
    }

    fn result(&self) -> Option<BattleResult> {
        self.over
    }

    fn real_time(&self) -> bool {
        true
    }

    fn set_input_delay(&mut self, ticks: u32) -> bool {
        NetPlayer::set_input_delay(self, ticks);
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
        Settings::new(game, TRIPLE_BATTLE)
    }

    /// A player who shows the newest tick (no input delay), so that every
    /// late input of the other's is predicted, and corrected by rollback.
    fn no_delay() -> NetOptions {
        NetOptions { input_delay: 0, ..NetOptions::default() }
    }

    /// The input delay is the player's own, 4 ticks unless they say, and
    /// kept within the range the library gives.
    #[test]
    fn the_input_delay_is_four_by_default_and_kept_in_range() {
        assert_eq!((NetOptions::default().input_delay, DEFAULT_INPUT_DELAY, INPUT_DELAY), (4, 4, 0..=10));
        assert!(INPUT_DELAY.contains(&DEFAULT_INPUT_DELAY));
        let content = exe6_test_content();
        let [a, _] = agreed(&content, &triple("exe6"), [side(&content, 3), side(&content, 4)]);
        let mut player = NetPlayer::new(Hand::default(), 0, a.set.clone(), NetOptions::default());
        assert_eq!((player.input_delay(), player.peer.session().present_delay()), (4, 4));
        assert_eq!(player.net_status().map(|n| (n.input_delay, n.ping_ms, n.quality, n.reconnecting)), Some((4, None, None, None)));
        for (asked, kept) in [(0, 0), (7, 7), (10, 10), (11, 10), (u32::MAX, 10)] {
            assert!(Driver::set_input_delay(&mut player, asked));
            assert_eq!((player.input_delay(), player.peer.session().present_delay()), (kept, kept), "asked for {asked}");
        }
        let player = NetPlayer::new(Hand::default(), 0, a.set, NetOptions { input_delay: 60, ..NetOptions::default() });
        assert_eq!((player.input_delay(), player.peer.session().present_delay()), (10, 10));
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
            let mut s = Session::new(Box::new(NetPlayer::new(hand, side, set, no_delay())));
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
            self.0.borrow_mut().send(frame).map_err(|e| e.detail)
        }

        fn reconnecting(&self) -> Option<Reconnecting> {
            let link = self.0.borrow();
            link.down_for().map(|elapsed| Reconnecting { elapsed, gives_up_in: link.gives_up_in() })
        }

        fn recv(&mut self) -> Result<Option<&[u8]>, String> {
            let mut link = self.0.borrow_mut();
            let Some(frame) = link.recv().map_err(|e| e.detail)? else { return Ok(None) };
            self.1 = frame.to_vec();
            Ok(Some(&self.1))
        }
    }

    /// A link as a match's channel once its lobby says the match starts, as
    /// a host has it: the match's frames go under their kind byte; a Hello
    /// or a Reveal that comes again is answered with this peer's Reveal (the
    /// other side hasn't it yet); what else the lobby said, come late, is
    /// left.
    struct Framed {
        link: nettai_rtc::Link,
        answer: Vec<u8>,
        frame: Vec<u8>,
    }

    impl Channel for Framed {
        fn send(&mut self, frame: &[u8]) -> Result<(), String> {
            let mut out = vec![crate::lobby::Kind::Frame as u8];
            out.extend_from_slice(frame);
            self.link.send(&out).map_err(|e| e.detail)
        }

        fn recv(&mut self) -> Result<Option<&[u8]>, String> {
            use crate::lobby::Kind;
            loop {
                let Some(d) = self.link.recv().map_err(|e| e.detail)? else { return Ok(None) };
                match d.split_first().map(|(&k, body)| (Kind::of(k), body)) {
                    Some((Some(Kind::Frame), body)) => {
                        self.frame.clear();
                        self.frame.extend_from_slice(body);
                        return Ok(Some(&self.frame));
                    }
                    Some((Some(Kind::Hello | Kind::Reveal), _)) => self.link.send(&self.answer).map_err(|e| e.detail)?,
                    _ => {}
                }
            }
        }

        fn reconnecting(&self) -> Option<Reconnecting> {
            self.link.down_for().map(|elapsed| Reconnecting { elapsed, gives_up_in: self.link.gives_up_in() })
        }
    }

    /// The whole way from an invite to a match, as a host's Play screen
    /// goes it, on WebRTC links through a signaling server (in process).
    /// One player asks the server for a room and is given its code; the
    /// other joins by the code as typed. Their lobbies meet: the joiner's
    /// takes the host's settings, each sees the other's name and the round
    /// trip. The host makes it a single battle with closed builds, which
    /// both show, nobody ready any more. Both ready: the match is agreed,
    /// and each host preloads it (`crate::preload`), saying how far it is as
    /// it goes, which the other sees; nothing starts while one is still
    /// loading. Both loaded: both start the same match, closed builds with
    /// it, and play it over the links with the players their preloads made
    /// (nothing is loaded late), each at its own input delay, with the
    /// connection's quality to show. One leaves, and the other is told.
    #[test]
    fn an_invite_becomes_a_match() {
        use crate::lobby::{Lobby, Role, Status, entropy};
        use crate::preload::Preload;

        struct Seat {
            link: nettai_rtc::Link,
            lobby: Lobby,
        }
        /// One frame of a seat: what its link brings goes to its lobby, the
        /// lobby is polled, and what it says is sent. How the lobby stands:
        /// 0 agreeing, 1 loading, 2 starting.
        fn frame(seat: &mut Seat) -> u8 {
            let now = Instant::now();
            while let Some(d) = seat.link.recv().unwrap() {
                seat.lobby.receive(now, d);
            }
            let stands = match seat.lobby.poll(now) {
                Status::Pending => 0,
                Status::Loading(_) => 1,
                Status::Start(_) => 2,
                Status::Failed(failure) => panic!("{:?}: {}", failure.reason, failure.detail),
            };
            for d in seat.lobby.outgoing() {
                seat.link.send(&d).unwrap();
            }
            stands
        }
        /// Frames of both seats until `done` (told how each lobby stands).
        fn until(seats: &mut [Seat; 2], what: &str, mut done: impl FnMut(&[Seat; 2], [u8; 2]) -> bool) {
            let start = Instant::now();
            loop {
                let stands = [frame(&mut seats[0]), frame(&mut seats[1])];
                if done(seats, stands) {
                    return;
                }
                assert!(start.elapsed() < Duration::from_secs(20), "{what}: not within 20 seconds (the lobbies stand {stands:?})");
                std::thread::sleep(Duration::from_millis(2));
            }
        }

        let content = exe6_test_content();
        let server = nettai_rtc::testing::Server::start();
        let config = nettai_rtc::Config { ice_servers: Vec::new(), loopback: true, ..nettai_rtc::Config::default() };
        // The invite: the server makes the room and says its code.
        let mut host = nettai_rtc::Link::create_room(&server.url(), config.clone());
        let start = Instant::now();
        while host.role().is_none() {
            assert!(start.elapsed() < Duration::from_secs(10), "not let in");
            host.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        let code = host.code().expect("the room's code").to_string();
        // The other player types it, in small letters, in two groups.
        let typed = format!("{} {}", &code[..3], &code[3..]).to_lowercase();
        let mut join = nettai_rtc::Link::join_room(&server.url(), &typed, config).unwrap();
        while join.role().is_none() {
            assert!(start.elapsed() < Duration::from_secs(10), "not let in");
            join.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!((host.role(), join.role()), (Some(nettai_rtc::Role::Host), Some(nettai_rtc::Role::Join)));
        // Each one's lobby: the host's a triple battle, the joiner's what
        // it last played, a single battle.
        let timeout = Duration::from_secs(60);
        let mut seats = [
            Seat { link: host, lobby: Lobby::new(Role::Host, &content, Settings::new("exe6", TRIPLE_BATTLE), side(&content, 3), entropy(), timeout) },
            Seat { link: join, lobby: Lobby::new(Role::Join, &content, Settings::new("exe6", 1), side(&content, 4), entropy() ^ 1, timeout) },
        ];
        seats[0].lobby.set_name("Lan");
        seats[1].lobby.set_name("Chaud");
        until(&mut seats, "the lobbies meet", |s, _| s.iter().all(|seat| seat.lobby.their_present() && seat.lobby.ping().is_some()) && s[1].lobby.settings() == s[0].lobby.settings());
        assert_eq!(seats[1].lobby.settings(), &Settings::new("exe6", TRIPLE_BATTLE), "the joiner takes the host's settings");
        assert_eq!((seats[0].lobby.their_name(), seats[1].lobby.their_name()), (Some("Chaud"), Some("Lan")));
        for seat in &seats {
            let ping = seat.lobby.ping().unwrap();
            assert!(ping.ms < 500.0 && ping.jitter_ms < 500.0, "two links on one machine: {ping:?}");
        }
        // The joiner readies; the host makes it a single battle with closed
        // builds: both show that, and nobody is ready.
        seats[1].lobby.set_ready(true);
        seats[0].lobby.set_rounds(1);
        seats[0].lobby.set_closed_builds(true);
        let single = Settings { closed_builds: true, ..Settings::new("exe6", 1) };
        until(&mut seats, "the settings are both players'", |s, _| s.iter().all(|seat| *seat.lobby.settings() == single && !seat.lobby.ready() && !seat.lobby.their_ready()));
        // Both ready: agreed, and loading.
        seats[0].lobby.set_ready(true);
        seats[1].lobby.set_ready(true);
        until(&mut seats, "the match is agreed", |_, stands| stands == [1, 1]);
        // Each host preloads the match it agreed, on the thread that will
        // play it, and says how far its preload is; the other sees it.
        let mut preloads: Vec<Preload> = seats
            .iter_mut()
            .map(|seat| {
                let Status::Loading(a) = seat.lobby.poll(Instant::now()) else { panic!("loading") };
                // (A pack with nothing in it: the steps are the match's.)
                let renderer = crate::Renderer::new(Arc::new(nettai_assets::Bundle::default()));
                Preload::with(renderer, None, None, &a.set, a.side as u8, (240, 160))
            })
            .collect();
        while preloads[0].fraction() < 0.5 && preloads[0].step() {}
        let half = preloads[0].fraction();
        assert!((0.5..1.0).contains(&half), "{half}");
        seats[0].lobby.set_loading(half);
        until(&mut seats, "the joiner sees the host loading", |s, _| (s[1].lobby.their_loading() - half).abs() < 0.01 && !s[1].lobby.their_loaded());
        // The host done and the joiner not begun: nothing starts.
        while preloads[0].step() {}
        assert!(preloads[0].done() && preloads[0].stopped().is_none());
        seats[0].lobby.set_loaded();
        until(&mut seats, "the joiner sees the host loaded", |s, _| s[1].lobby.their_loaded());
        for _ in 0..50 {
            assert_eq!([frame(&mut seats[0]), frame(&mut seats[1])], [1, 1], "a match starts once both have loaded");
            std::thread::sleep(Duration::from_millis(2));
        }
        // The joiner's, a step a frame, said as it goes (a loading screen's
        // frames); done, the match starts on both.
        loop {
            let more = preloads[1].step();
            seats[1].lobby.set_loading(preloads[1].fraction());
            if !more {
                break;
            }
            assert_eq!([frame(&mut seats[0]), frame(&mut seats[1])], [1, 1], "a match starts once both have loaded");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(preloads[1].done());
        until(&mut seats, "the host sees the joiner's bar full", |s, stands| s[0].lobby.their_loading() > 0.99 && !s[0].lobby.their_loaded() && stands == [1, 1]);
        seats[1].lobby.set_loaded();
        until(&mut seats, "the match starts", |_, stands| stands == [2, 2]);
        // The same match on both, closed builds with it; played over the
        // links by the players the preloads made, each player at their own
        // input delay.
        let mut playing: Vec<crate::Player> = Vec::new();
        let mut seeds = Vec::new();
        for (i, (Seat { link, mut lobby }, preload)) in seats.into_iter().zip(preloads).enumerate() {
            let Status::Start(a) = lobby.poll(Instant::now()) else { panic!("started") };
            assert!(a.closed_builds() && a.m.rounds.len() == 1 && a.side == i);
            assert_eq!(a.names, ["Lan".to_string(), "Chaud".to_string()]);
            seeds.push(a.seed);
            let channel = Framed { link, answer: a.answer.clone(), frame: Vec::new() };
            let options = NetOptions { input_delay: [DEFAULT_INPUT_DELAY, 2][i], ..NetOptions::default() };
            playing.push(preload.player(Box::new(NetPlayer::new(channel, a.side, a.set.clone(), options))));
        }
        assert_eq!(seeds[0], seeds[1]);
        for _ in 0..300 {
            for (i, player) in playing.iter_mut().enumerate() {
                assert!(player.tick(0), "side {i}: {:?} {:?}", player.stop_reason(), player.stopped());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        for (i, player) in playing.iter().enumerate() {
            let status = player.net_status().expect("a netplay match");
            assert!(player.ticks() > 100, "side {i} played {} ticks", player.ticks());
            assert_eq!(status.input_delay, [4, 2][i]);
            assert!(status.quality.is_some() && status.ping_ms.is_some() && status.jitter_ms.is_some() && status.reconnecting.is_none(), "side {i}: {status:?}");
            let late = player.late();
            assert_eq!((late.runtimes, late.reads), (0, 0), "side {i} loaded nothing late");
        }
        // One leaves; the other is told so.
        drop(playing.pop());
        let start = Instant::now();
        while playing[0].tick(0) {
            assert!(start.elapsed() < Duration::from_secs(10), "the host isn't told the joiner left");
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(playing[0].stop_reason(), Some(StopReason::OpponentLeft), "{:?}", playing[0].stopped());
    }

    /// Two players on WebRTC links through a room of a signaling server (in
    /// process) play the short set (the host's navi shoots, the joiner's has
    /// 1 HP and stands) and record it; in round one, while both fight, the
    /// joiner's network goes for three seconds. Both see the connection
    /// down, with the time it has left (what a host's "reconnecting" shows),
    /// and wait; it comes back, a new
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
        // (The host asks the server for a room; the joiner comes by the
        // code the server gave it.)
        let mut host = nettai_rtc::Link::create_room(&server.url(), config.clone());
        let start = Instant::now();
        while host.role().is_none() {
            assert!(start.elapsed() < Duration::from_secs(10), "not let in");
            host.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        let code = host.code().expect("the room's code").to_string();
        let links = [host, nettai_rtc::Link::join_room(&server.url(), &code, config).unwrap()].map(|l| Rc::new(RefCell::new(l)));
        let sinks = [Shared::default(), Shared::default()];
        let mut playing: Vec<Session> = Vec::new();
        for (side, crate::lobby::Agreement { set, m, .. }) in agreed(&content, &Settings::of_match(&m), m.sides.clone()).into_iter().enumerate() {
            let channel = Rtc(links[side].clone(), Vec::new());
            let mut s = Session::new(Box::new(NetPlayer::new(channel, side, set, no_delay())));
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
                if let Some(down) = s.driver.net_status().and_then(|n| n.reconnecting) {
                    // (Of the link's 30 seconds.)
                    let left = down.gives_up_in.expect("the link says how long it has left");
                    assert!(left <= Duration::from_secs(30) && down.elapsed + left >= Duration::from_secs(29), "{down:?}");
                    seen_down[side] = true;
                }
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
    type Played = (String, Match, Vec<(u32, u64)>, usize, String, Option<Stop>);

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
            let mut player = NetPlayer::new(hand, side, set, no_delay());
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
                    left.as_ref().map_or("left first", |stop| stop.detail.as_str()),
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
            assert_eq!(*left, Stop::new(StopReason::OpponentLeft, "the other player left the match"));
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

    /// START in a netplay fight pauses neither peer's battle, and is said
    /// to the host of the player who pressed it alone, once a press, on the
    /// frame that first simulates the press's tick (the one after the
    /// press was taken), however often a rollback simulates that tick
    /// again (the other player moves about, so every prediction of them
    /// fails). The other peer is told nothing, and the settled states agree
    /// throughout.
    #[test]
    fn start_in_a_netplay_fight_is_said_once_to_its_own_player() {
        use nettai_battle::battle::{fight, mode};
        use nettai_battle::input::keys;
        const MOVES: u16 = keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT;
        let content = exe6_test_content();
        let mine = [0, 1].map(|s| side(&content, 11 + 11 * s as u32));
        let (mut shuttle, hands) = Shuttle::new();
        let mut hands = hands.into_iter().enumerate();
        let mut players = agreed(&content, &triple("exe6"), mine).map(|a| {
            let (side, hand) = hands.next().unwrap();
            let mut player = NetPlayer::new(hand, side, a.set, no_delay());
            let shown = player.start();
            (player, shown)
        });
        let mut masher = Masher::new(5);
        // By side: the START presses a frame took, and the frames that said
        // the player asked for the pause; and side 1's presses taken and
        // not yet said.
        let (mut pressed, mut said, mut owed) = ([0u32; 2], [0u32; 2], 0u32);
        let mut settled: [std::collections::HashMap<u32, u64>; 2] = Default::default();
        let start = Instant::now();
        for frame in 1..=2410u32 {
            shuttle.carry(frame);
            for side in 0..2 {
                let (player, shown) = &mut players[side];
                let fighting = shown.round.mode == mode::FIGHTING && shown.fight.state == fight::FIGHTING;
                let keys = match (shown.round.mode == mode::CUSTOM, side) {
                    (true, _) => crate::driver::bot_buttons(shown, side, frame),
                    // Side 0 moves about; side 1 presses START now and then.
                    (false, 0) => masher.buttons() & MOVES,
                    (false, _) if fighting && frame % 40 == 0 => keys::START,
                    (false, _) => 0,
                };
                let ran = player.run_frame(keys, shown).unwrap().unwrap();
                // (A frame simulates the tick of the input the frame before
                // took: the press is said then.)
                assert!(!ran.pause_requests[1 - side], "side {side}, frame {frame}: told of the other player's START");
                if ran.pause_requests[side] {
                    assert!(side == 1 && owed == 1, "side {side}, frame {frame}: said with {owed} presses taken");
                    owed -= 1;
                    said[side] += 1;
                }
                if ran.advanced && keys & keys::START != 0 && fighting {
                    pressed[side] += 1;
                    owed += 1;
                }
                assert_ne!(shown.fight.state, fight::PAUSE, "side {side}, frame {frame}");
                if ran.advanced {
                    let (tick, digest) = player.settled();
                    settled[side].insert(tick, digest);
                }
            }
            let next = start + Duration::from_millis(2) * frame;
            std::thread::sleep(next.saturating_duration_since(Instant::now()));
        }
        assert!(pressed[1] >= 10, "{} presses in the fight", pressed[1]);
        assert_eq!((said, owed), ([0, pressed[1]], 0));
        let rollbacks = players[1].0.stats().0.rollbacks;
        assert!(rollbacks > 20, "{rollbacks} rollbacks");
        let common = settled[0].iter().filter(|(tick, digest)| settled[1].get(tick).is_some_and(|d| d == *digest)).count();
        let differing = settled[0].iter().filter(|(tick, digest)| settled[1].get(tick).is_some_and(|d| d != *digest)).count();
        assert!(differing == 0 && common > 1000, "{common} settled ticks alike, {differing} differing");
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
        left: Option<Stop>,
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
            let mut player = NetPlayer::new(hand, side, set, no_delay());
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
                // The host changes its input delay during the first round
                // (the joiner keeps none): its own, kept for the round
                // after, and the settled states still agree.
                if side == 0 && frame == 100 {
                    assert!(Driver::set_input_delay(player, 3));
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
                    out.left.as_ref().map_or("left first", |stop| stop.detail.as_str()),
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
            // (The connection's figures are values: the input delay each
            // player asked for, the host's changed during the match and kept
            // into the second round's session; a round trip measured, how
            // unsteady it is, and the quality the two make.)
            assert_eq!((hosted.status.input_delay, hosted.session_present_delay), (3, 3), "{game}");
            assert_eq!((joined.status.input_delay, joined.session_present_delay), (0, 0), "{game}");
            for p in [&hosted, &joined] {
                let (ping, jitter) = (p.status.ping_ms.expect("a round trip"), p.status.jitter_ms.expect("its jitter"));
                assert!(ping > 0.0 && jitter >= 0.0 && (0.0..=1.0).contains(&p.status.loss), "{game}: {:?}", p.status);
                assert_eq!(p.status.quality, Some(Quality::of(ping, jitter)), "{game}");
                assert_eq!(p.status.reconnecting, None, "{game}");
            }
            // (The joiner is told the host left, and why it stopped says so
            // in a way a host can word.)
            assert_eq!(joined.left, Some(Stop::new(StopReason::OpponentLeft, "the match is over (you lost); the other player left")), "{game}");
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
        // Each player's file, as --save-match writes one.
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
