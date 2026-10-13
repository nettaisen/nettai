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
/// folders whole) as the settings play it: without its patch cards where
/// they are off (`Settings::as_played`). This is where the agreed match is
/// made of the two sides, on both peers alike, so the set, its first
/// round's digest and the match a recording keeps all hold the sides as
/// they are played.
pub fn netplay_setup(content: &Arc<Content>, seed: u32, settings: &crate::lobby::Settings, sides: [Side; 2]) -> Result<(Set, Match), String> {
    let sides = sides.map(|side| settings.as_played(content, &side));
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

    /// Whether the two players stay together once this match's set is over,
    /// on what the channel goes over: a lobby's channel
    /// (`crate::lobby::MatchChannel`), whose lobby goes on for another match
    /// (`Lobby::reopen`). A player dropped once its set is over then says
    /// nothing of leaving: it sends its last frame once more and is gone,
    /// and a player who does leave says so in the lobby. False (the
    /// default): the channel is this match's alone, and a player dropped
    /// tells the other it leaves, its set over or not.
    fn outlives_match(&self) -> bool {
        false
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
    /// the channel says it is down and being made again,
    /// [`Channel::reconnecting`]; and not once the set is over, when this
    /// player needs nothing more of the other, who may be back in the lobby
    /// already).
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
    send::<NetPlayer<crate::lobby::MatchChannel>>();
    send::<crate::lobby::Lobby>();
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
        if self.over.is_none() && self.last_frame.elapsed() > self.options.timeout {
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
    /// lost; if all are, the other side's timeout ends its match). Not
    /// once the set is over on a channel whose players stay together
    /// ([`Channel::outlives_match`]): the player sends its last frame once
    /// more (the inputs the other hasn't acknowledged, which the other may
    /// need to end the set too), and nobody has left.
    fn drop(&mut self) {
        let now = self.now();
        if self.over.is_some() && self.channel.outlives_match() {
            let datagram = self.peer.datagram(now);
            let _ = self.channel.send(&datagram);
            return;
        }
        self.peer.leave();
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
        // (The players closed their builds: each one's recording, made of
        // its agreement, says so.)
        let closed = Settings { closed_builds: true, ..triple("exe6") };
        let mut names = Vec::new();
        for (side, (hand, agreement)) in hands.into_iter().zip(agreed(&content, &closed, mine)).enumerate() {
            let recorder = Recorder::of_agreement(Box::new(sinks[side].clone()), &content, &agreement, 1_790_000_000).unwrap();
            names.push(agreement.names.clone());
            let mut s = Session::new(Box::new(NetPlayer::new(hand, side, agreement.set, no_delay())));
            s.record(recorder).unwrap();
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
        // Each one's info is its agreement's: an online match with closed
        // builds, recorded from its own side, by the names the lobby had.
        for (side, replay) in [&a, &b].into_iter().enumerate() {
            let info = nettai_replay::Info { when: 1_790_000_000, side: side as u8, kind: nettai_replay::MatchKind::Online, closed_builds: true, names: names[side].clone() };
            assert_eq!(replay.info, info, "side {side}");
        }
        assert_eq!(names[0], names[1]);
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

    /// One player's end of an online session, as a host holds it: the link
    /// to the other player, the lobby over it, and the match's player while
    /// one is up.
    struct Seat {
        link: nettai_rtc::Link,
        lobby: crate::lobby::Lobby,
        player: Option<crate::Player>,
    }

    /// How a seat's lobby stands after a frame.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Stands {
        Agreeing,
        Loading,
        Start,
        Playing,
        Failed(crate::lobby::Reason),
    }

    impl Seat {
        /// One frame of a host, the same whatever its screen shows: what the
        /// link brings goes to the lobby (a match's frames too), which is
        /// told how the link is; the match's player, if one is up, runs a
        /// frame on `keys` (told the battle it shows); the lobby is polled;
        /// and what the lobby has to say, the player's frame with it, is
        /// sent.
        fn frame(&mut self, keys: impl FnOnce(&Battle) -> u16) -> Stands {
            use crate::lobby::Status;
            let now = Instant::now();
            while let Some(d) = self.link.recv().unwrap() {
                self.lobby.receive(now, d);
            }
            self.lobby.set_link_down(self.link.down_for().map(|elapsed| Reconnecting { elapsed, gives_up_in: self.link.gives_up_in() }));
            if let Some(player) = &mut self.player {
                let keys = keys(player.battle());
                // (One that has stopped runs no more: a test looks at why.)
                let _ = player.tick(keys);
            }
            let stands = match self.lobby.poll(now) {
                Status::Pending => Stands::Agreeing,
                Status::Loading(_) => Stands::Loading,
                Status::Start(_) => Stands::Start,
                Status::Playing => Stands::Playing,
                Status::Failed(failure) => Stands::Failed(failure.reason.clone()),
            };
            for d in self.lobby.outgoing() {
                self.link.send(&d).unwrap();
            }
            stands
        }

        fn player(&self) -> &crate::Player {
            self.player.as_ref().expect("a match is up")
        }
    }

    /// Frames of both seats, each match's player on the keys `keys` gives
    /// its side, until `done` (told how each lobby stands).
    fn until(seats: &mut [Seat; 2], what: &str, keys: impl Fn(&Battle, usize, u32) -> u16, mut done: impl FnMut(&[Seat; 2], &[Stands; 2]) -> bool) {
        let start = Instant::now();
        for frame in 0u32.. {
            let stands = [0, 1].map(|i| seats[i].frame(|b| keys(b, i, frame)));
            if done(seats, &stands) {
                return;
            }
            // (Long: the tests run beside each other, on a machine that may
            // be busy.)
            if start.elapsed() > Duration::from_secs(120) {
                let players = seats.each_ref().map(|seat| seat.player.as_ref().map(|p| (p.position(), p.result(), p.stopped().map(str::to_string))));
                panic!("{what}: not within two minutes, {frame} frames (the lobbies stand {stands:?}; the players: {players:?})");
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// No keys: a player that presses nothing.
    fn idle(_: &Battle, _: usize, _: u32) -> u16 {
        0
    }

    /// Two players meet on WebRTC links: one asks the signaling server for
    /// a room and is given its code, the other joins by the code as typed
    /// (in small letters, in two groups). Each one's lobby is made once the
    /// links are open, in its link's role, holding its own settings and
    /// bringing its side (the host's, then the joiner's). The seats, and the
    /// room's code.
    fn meet(content: &Arc<Content>, server: &nettai_rtc::testing::Server, settings: [Settings; 2], sides: [Side; 2]) -> ([Seat; 2], String) {
        use crate::lobby::{Lobby, Role, entropy};
        let config = nettai_rtc::Config { ice_servers: Vec::new(), loopback: true, ..nettai_rtc::Config::default() };
        let mut host = nettai_rtc::Link::create_room(&server.url(), config.clone());
        let start = Instant::now();
        while host.role().is_none() {
            assert!(start.elapsed() < Duration::from_secs(60), "not let in");
            host.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        let code = host.code().expect("the room's code").to_string();
        let typed = format!("{} {}", &code[..3], &code[3..]).to_lowercase();
        let mut join = nettai_rtc::Link::join_room(&server.url(), &typed, config).unwrap();
        while !(host.is_open() && join.is_open()) {
            assert!(start.elapsed() < Duration::from_secs(120), "the links don't open");
            host.poll().unwrap();
            join.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!((host.role(), join.role()), (Some(nettai_rtc::Role::Host), Some(nettai_rtc::Role::Join)));
        let timeout = Duration::from_secs(60);
        let [host_settings, join_settings] = settings;
        let [host_side, join_side] = sides;
        let mut seats = [
            Seat { link: host, lobby: Lobby::new(Role::Host, content, host_settings, host_side, entropy(), timeout), player: None },
            Seat { link: join, lobby: Lobby::new(Role::Join, content, join_settings, join_side, entropy() ^ 1, timeout), player: None },
        ];
        seats[0].lobby.set_name("Lan");
        seats[1].lobby.set_name("Chaud");
        (seats, code)
    }

    /// A renderer of a pack with nothing in it.
    fn renderer() -> crate::Renderer {
        crate::Renderer::new(Arc::new(nettai_assets::Bundle::default()))
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
    /// it, and play it on the lobby's channel with the players their
    /// preloads made (nothing is loaded late), each at its own input delay,
    /// with the connection's quality to show. One leaves the match, and the
    /// other's player is told; both are back in the lobby. Then that one
    /// leaves for good, and the other's lobby is told.
    #[test]
    fn an_invite_becomes_a_match() {
        use crate::lobby::{Reason, Status};
        use crate::preload::Preload;
        let content = exe6_test_content();
        let server = nettai_rtc::testing::Server::start();
        // Each one's lobby: the host's a triple battle, the joiner's what
        // it last played, a single battle.
        let (mut seats, code) = meet(&content, &server, [Settings::new("exe6", TRIPLE_BATTLE), Settings::new("exe6", 1)], [side(&content, 3), side(&content, 4)]);
        assert_eq!(server.codes(), [code]);
        until(&mut seats, "the lobbies meet", idle, |s, _| s.iter().all(|seat| seat.lobby.their_present() && seat.lobby.ping().is_some()) && s[1].lobby.settings() == s[0].lobby.settings());
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
        until(&mut seats, "the settings are both players'", idle, |s, _| s.iter().all(|seat| *seat.lobby.settings() == single && !seat.lobby.ready() && !seat.lobby.their_ready()));
        // Both ready: agreed, and loading.
        seats[0].lobby.set_ready(true);
        seats[1].lobby.set_ready(true);
        until(&mut seats, "the match is agreed", idle, |_, stands| *stands == [Stands::Loading, Stands::Loading]);
        // Each host preloads the match it agreed, on the thread that will
        // play it, and says how far its preload is; the other sees it.
        let mut preloads: Vec<Preload> = seats
            .iter_mut()
            .map(|seat| {
                let Status::Loading(a) = seat.lobby.poll(Instant::now()) else { panic!("loading") };
                // (A pack with nothing in it: the steps are the match's.)
                Preload::with(renderer(), None, None, &a.set, a.side as u8, (240, 160))
            })
            .collect();
        while preloads[0].fraction() < 0.5 && preloads[0].step() {}
        let half = preloads[0].fraction();
        assert!((0.5..1.0).contains(&half), "{half}");
        seats[0].lobby.set_loading(half);
        until(&mut seats, "the joiner sees the host loading", idle, |s, _| (s[1].lobby.their_loading() - half).abs() < 0.01 && !s[1].lobby.their_loaded());
        // The host done and the joiner not begun: nothing starts.
        while preloads[0].step() {}
        assert!(preloads[0].done() && preloads[0].stopped().is_none());
        seats[0].lobby.set_loaded();
        until(&mut seats, "the joiner sees the host loaded", idle, |s, _| s[1].lobby.their_loaded());
        let loading = [Stands::Loading, Stands::Loading];
        for _ in 0..50 {
            assert_eq!([seats[0].frame(|_| 0), seats[1].frame(|_| 0)], loading, "a match starts once both have loaded");
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
            assert_eq!([seats[0].frame(|_| 0), seats[1].frame(|_| 0)], loading, "a match starts once both have loaded");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(preloads[1].done());
        until(&mut seats, "the host sees the joiner's bar full", idle, |s, stands| s[0].lobby.their_loading() > 0.99 && !s[0].lobby.their_loaded() && *stands == loading);
        seats[1].lobby.set_loaded();
        until(&mut seats, "the match starts", idle, |_, stands| *stands == [Stands::Start, Stands::Start]);
        // The same match on both, closed builds with it; played on the
        // lobby's channel by the players the preloads made, each player at
        // their own input delay.
        let mut seeds = Vec::new();
        for (i, (seat, preload)) in seats.iter_mut().zip(preloads).enumerate() {
            let Status::Start(a) = seat.lobby.poll(Instant::now()) else { panic!("started") };
            assert!(a.closed_builds() && a.m.rounds.len() == 1 && a.side == i);
            assert_eq!(a.names, ["Lan".to_string(), "Chaud".to_string()]);
            seeds.push(a.seed);
            let (side, set) = (a.side, a.set.clone());
            let channel = seat.lobby.channel().expect("the match's channel");
            let options = NetOptions { input_delay: [DEFAULT_INPUT_DELAY, 2][i], ..NetOptions::default() };
            seat.player = Some(preload.player(Box::new(NetPlayer::new(channel, side, set, options))));
        }
        assert_eq!(seeds[0], seeds[1]);
        let playing = [Stands::Playing, Stands::Playing];
        for _ in 0..300 {
            assert_eq!([seats[0].frame(|_| 0), seats[1].frame(|_| 0)], playing);
            for (i, seat) in seats.iter().enumerate() {
                assert!(seat.player().stop_reason().is_none(), "side {i}: {:?}", seat.player().stopped());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        for (i, seat) in seats.iter().enumerate() {
            let player = seat.player();
            let status = player.net_status().expect("a netplay match");
            assert!(player.ticks() > 100, "side {i} played {} ticks", player.ticks());
            assert_eq!(status.input_delay, [4, 2][i]);
            assert!(status.quality.is_some() && status.ping_ms.is_some() && status.jitter_ms.is_some() && status.reconnecting.is_none(), "side {i}: {status:?}");
            let late = player.late();
            assert_eq!((late.runtimes, late.reads), (0, 0), "side {i} loaded nothing late");
        }
        // The joiner leaves the match (its player is dropped before the set
        // is over): the host's player is told so.
        seats[1].player = None;
        until(&mut seats, "the host is told the joiner left the match", idle, |s, _| s[0].player().stop_reason().is_some());
        assert_eq!(seats[0].player().stop_reason(), Some(StopReason::OpponentLeft), "{:?}", seats[0].player().stopped());
        // The match stopped with the link still up: both go back to the
        // lobby, where the settings are as they were and nobody is ready.
        seats[0].player = None;
        assert!(seats[0].lobby.reopen() && seats[1].lobby.reopen());
        let since = Instant::now();
        until(&mut seats, "both are back in the lobby", idle, |_, stands| {
            assert_eq!(*stands, [Stands::Agreeing, Stands::Agreeing]);
            since.elapsed() > Duration::from_millis(300)
        });
        assert!(seats.iter().all(|seat| !seat.lobby.ready() && !seat.lobby.their_ready() && *seat.lobby.settings() == single));
        // Then the joiner leaves for good, and says so: the host's lobby
        // ends, told.
        seats[1].lobby.leave();
        until(&mut seats, "the host is told the joiner left", idle, |_, stands| stands[0] != Stands::Agreeing);
        assert_eq!([seats[0].frame(|_| 0), seats[1].frame(|_| 0)], [Stands::Failed(Reason::OpponentLeft), Stands::Failed(Reason::Left)]);
    }

    /// The player who is back in the lobby first leaves the other what it
    /// needs to end the match. The host's frames reach the joiner ten frames
    /// late, so the host's set is over while the joiner still lacks the
    /// host's last inputs; the host's player is dropped then and its lobby
    /// reopened, and everything the host had sent that was on its way is
    /// lost, with all it says for the next 150 ms. The host's lobby says
    /// its player's last frame again with its own messages, every 100 ms:
    /// the joiner's player has the inputs from it, and its set ends too,
    /// lost as the host's was won. Then the joiner comes back, and
    /// the two are in the lobby. (In memory: the two lobbies' datagrams
    /// handed across by the test.)
    #[test]
    fn the_player_back_in_the_lobby_first_leaves_the_other_its_last_frames() {
        use crate::driver::short_set;
        use crate::lobby::{Status, testing};
        const LATE: u32 = 10;
        let content = exe6_test_content();
        let m = short_set::of(&content, "exe6", 7);
        let single = Settings::new("exe6", 1);
        let mut lobbies = testing::pair(&content, [single.clone(), single], m.sides.clone());
        testing::run(&mut lobbies, Instant::now(), 20, &mut |_, _| {});
        testing::load(&mut lobbies, Instant::now());
        let mut players = [0, 1].map(|i| {
            let Status::Start(a) = lobbies[i].poll(Instant::now()) else { panic!("started") };
            let (side, set) = (a.side, a.set.clone());
            let mut player = NetPlayer::new(lobbies[i].channel().expect("the match's channel"), side, set, no_delay());
            let shown = player.start();
            Some((player, shown))
        });
        // The host's datagrams on their way to the joiner, with the frame
        // each arrives on.
        let mut on_the_way: VecDeque<(u32, Vec<u8>)> = VecDeque::new();
        let (mut host_back, mut results) = (None::<(u32, Instant)>, [None, None]);
        for frame in 1u32.. {
            assert!(frame < 30_000, "the joiner's set doesn't end: the host back in the lobby at frame {host_back:?}");
            let now = Instant::now();
            for side in 0..2 {
                let Some((player, shown)) = &mut players[side] else { continue };
                let keys = if side == 0 { short_set::shooter(shown, side, frame) } else { crate::driver::bot_buttons(shown, side, frame) };
                player.run_frame(keys, shown).unwrap().unwrap();
                results[side] = player.result();
            }
            // The host's set is over, the joiner's isn't: the host goes back
            // to the lobby, and what it had sent that hasn't arrived is lost.
            if host_back.is_none() && results[0].is_some() {
                assert_eq!(results, [Some(BattleResult::Won), None], "the joiner hasn't the host's last inputs yet");
                players[0] = None;
                assert!(lobbies[0].reopen());
                on_the_way.clear();
                host_back = Some((frame, now));
            }
            let _ = lobbies[0].poll(now);
            let said = lobbies[0].outgoing();
            // (And what it says in the lobby's first 150 ms.)
            if host_back.is_none_or(|(_, at)| now.duration_since(at) >= Duration::from_millis(150)) {
                on_the_way.extend(said.into_iter().map(|d| (frame + LATE, d)));
            }
            let _ = lobbies[1].poll(now);
            for d in lobbies[1].outgoing() {
                lobbies[0].receive(now, &d);
            }
            while on_the_way.front().is_some_and(|(at, _)| *at <= frame) {
                lobbies[1].receive(now, &on_the_way.pop_front().unwrap().1);
            }
            if results[1].is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(host_back.is_some());
        assert_eq!(results, [Some(BattleResult::Won), Some(BattleResult::Lost)]);
        assert!(matches!(lobbies[0].poll(Instant::now()), Status::Pending) && matches!(lobbies[1].poll(Instant::now()), Status::Playing));
        assert!(lobbies[1].their_reopened());
        // The joiner comes back too.
        players[1] = None;
        assert!(lobbies[1].reopen());
        testing::run(&mut lobbies, Instant::now(), 4, &mut |_, _| {});
        assert!(lobbies.iter_mut().all(|l| matches!(l.poll(Instant::now()), Status::Pending) && !l.ready() && !l.their_ready()));
    }

    /// Two matches in a row over one link, as "play again" goes: no new
    /// invite, the same room. The first match (a single battle: the host's
    /// navi shoots, the joiner's has 1 HP) is played to its end on both. The
    /// host goes back to the lobby while the joiner still looks at the
    /// result: the host's lobby shows the same settings and nobody ready,
    /// through a second and a half of the joiner's match frames, which are
    /// none of its; nothing stops the joiner's player, whose lobby knows the
    /// host is back. The host brings another build and readies. The joiner
    /// comes back, sees the host ready, readies: a second match is agreed,
    /// of another seed, with the host's new build; a frame of the first
    /// match, come late into its loading, starts nothing. It is played to
    /// its end too. Both back in the lobby, the host leaves, and the joiner
    /// is told.
    #[test]
    fn two_matches_in_a_row_over_one_link() {
        use crate::driver::short_set;
        use crate::lobby::{Agreement, Reason, Status, frame_datagram};
        /// Both lobbies say a match starts: each seat takes its channel,
        /// once, and makes its player. What each agreed.
        fn start(seats: &mut [Seat; 2], options: [NetOptions; 2]) -> [Agreement; 2] {
            [0, 1].map(|i| {
                let seat = &mut seats[i];
                let Status::Start(a) = seat.lobby.poll(Instant::now()) else { panic!("started") };
                let a = a.clone();
                let channel = seat.lobby.channel().expect("the match's channel");
                assert!(seat.lobby.channel().is_none(), "a match has one channel");
                seat.player = Some(crate::Player::with(renderer(), None, None, Box::new(NetPlayer::new(channel, a.side, a.set.clone(), options[i]))));
                a
            })
        }
        let results = |seats: &[Seat; 2]| seats.each_ref().map(|seat| seat.player().result());
        let content = exe6_test_content();
        let server = nettai_rtc::testing::Server::start();
        let m = short_set::of(&content, "exe6", 7);
        let single = Settings::new("exe6", 1);
        let (mut seats, code) = meet(&content, &server, [single.clone(), single.clone()], m.sides.clone());
        let keys = |b: &Battle, side: usize, frame: u32| if side == 0 { short_set::shooter(b, side, frame) } else { crate::driver::bot_buttons(b, side, frame) };
        let (agreeing, loading, starting, playing) =
            ([Stands::Agreeing, Stands::Agreeing], [Stands::Loading, Stands::Loading], [Stands::Start, Stands::Start], [Stands::Playing, Stands::Playing]);

        // ---- The first match, to its end.
        for seat in &mut seats {
            seat.lobby.set_ready(true);
        }
        until(&mut seats, "the first match is agreed", keys, |_, stands| *stands == loading);
        for seat in &mut seats {
            seat.lobby.set_loaded();
        }
        until(&mut seats, "the first match starts", keys, |_, stands| *stands == starting);
        let first = start(&mut seats, [NetOptions::default(); 2]);
        assert_eq!((first[0].seed, &first[0].sides), (first[1].seed, &m.sides));
        until(&mut seats, "the first match is played to its end", keys, |s, stands| {
            assert_eq!(*stands, playing);
            results(s).iter().all(Option::is_some)
        });
        assert_eq!(results(&seats), [Some(BattleResult::Won), Some(BattleResult::Lost)]);

        // ---- The host is back in the lobby; the joiner looks at the result.
        seats[0].player = None;
        assert!(seats[0].lobby.reopen());
        let since = Instant::now();
        until(&mut seats, "the host waits in the lobby", keys, |s, stands| {
            assert_eq!(*stands, [Stands::Agreeing, Stands::Playing]);
            assert!(s[1].player().stop_reason().is_none(), "the joiner's match is over, and nothing stops it: {:?}", s[1].player().stopped());
            assert!(!s[0].lobby.ready() && !s[0].lobby.their_ready(), "nobody is ready");
            since.elapsed() > Duration::from_millis(1500) && s[1].lobby.their_reopened()
        });
        assert_eq!(seats[0].lobby.settings(), &single);
        // The host brings another build, and is ready for another match.
        let another = side(&content, 21);
        assert_ne!(another, m.sides[0]);
        seats[0].lobby.set_side(another.clone());
        seats[0].lobby.set_ready(true);
        // The joiner comes back: the same settings, the host ready, itself
        // not.
        seats[1].player = None;
        assert!(seats[1].lobby.reopen());
        until(&mut seats, "the joiner is back in the lobby", keys, |s, stands| *stands == agreeing && s[1].lobby.their_ready());
        assert_eq!(seats[1].lobby.settings(), &single);
        assert!(!seats[1].lobby.ready() && !seats[0].lobby.their_ready() && seats[0].lobby.ready());
        assert_eq!(server.codes(), [code], "the room they met in, kept");

        // ---- The second match.
        seats[1].lobby.set_ready(true);
        until(&mut seats, "the second match is agreed", keys, |_, stands| *stands == loading);
        // The host has loaded and waits for the joiner; a frame of the first
        // match, come late, isn't the joiner starting.
        seats[0].lobby.set_loaded();
        let late = frame_datagram(first[0].seed, &[0, 0, 0]);
        for _ in 0..20 {
            seats[0].lobby.receive(Instant::now(), &late);
            assert_eq!([seats[0].frame(|_| 0), seats[1].frame(|_| 0)], loading, "a frame of the match before starts nothing");
            std::thread::sleep(Duration::from_millis(2));
        }
        seats[1].lobby.set_loaded();
        until(&mut seats, "the second match starts", keys, |_, stands| *stands == starting);
        let second = start(&mut seats, [NetOptions::default(); 2]);
        assert_ne!(second[0].seed, first[0].seed, "another match, another seed");
        assert_eq!((second[0].seed, &second[0].sides), (second[1].seed, &second[1].sides));
        assert_eq!(second[0].sides, [another, m.sides[1].clone()], "the host's new build, the joiner's as it was");
        // (Nor is such a frame the second match's player's.)
        seats[1].lobby.receive(Instant::now(), &late);
        until(&mut seats, "the second match is played to its end", keys, |s, stands| {
            assert_eq!(*stands, playing);
            for (i, seat) in s.iter().enumerate() {
                assert!(seat.player().stop_reason().is_none(), "side {i}: {:?}", seat.player().stopped());
            }
            results(s).iter().all(Option::is_some)
        });
        assert_eq!(results(&seats), [Some(BattleResult::Won), Some(BattleResult::Lost)]);

        // ---- Both back in the lobby; the host leaves, and says so.
        for seat in &mut seats {
            seat.player = None;
            assert!(seat.lobby.reopen());
        }
        let since = Instant::now();
        until(&mut seats, "both are in the lobby again", keys, |_, stands| {
            assert_eq!(*stands, agreeing);
            since.elapsed() > Duration::from_millis(300)
        });
        assert!(seats.iter().all(|seat| !seat.lobby.ready() && !seat.lobby.their_ready() && *seat.lobby.settings() == single));
        seats[0].lobby.leave();
        until(&mut seats, "the joiner is told the host left", keys, |_, stands| stands[1] != Stands::Agreeing);
        assert_eq!([seats[0].frame(|_| 0), seats[1].frame(|_| 0)], [Stands::Failed(Reason::Left), Stands::Failed(Reason::OpponentLeft)]);
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
            let info = nettai_replay::Info::of(nettai_replay::MatchKind::Online, side as u8);
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
