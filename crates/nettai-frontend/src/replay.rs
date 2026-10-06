//! Replays (nettai-replay's files): a set recorded as it is played
//! ([`Recorder`], which a [`crate::Player`] writes: [`crate::Player::record`]),
//! played back ([`ReplayPlayer`], a driver that checks what it plays
//! against the recording), and played to its end without a picture or a
//! sound ([`play_out`]) for whoever wants what happened in it.
//!
//! A replay holds the match (nettai-match's binary) and each tick's
//! buttons by side, and marks the ticks a round and the set ended on; the
//! battle's digest is kept every [`DIGEST_EVERY`] ticks of the set and at
//! each round's end, and of the first battle before any tick. Playing one
//! back makes the set again from those: a digest or a mark that differs is
//! a difference from the recording ([`crate::Player::diverged`]).
//!
//! The simulation is side 0's on every console (a round's setup says so:
//! its `local_side`), whoever watches it: a replay shown from side 1 draws
//! side 1's console, and its digests are still the simulation's.

use std::io::Write;
use std::sync::Arc;

use nettai_battle::content::Content;
use nettai_battle::{Battle, BattleResult, RoundEnd, TickInput};
use nettai_match::{After, Match, Set};
// (What a host reads of a replay, without naming nettai-replay.)
pub use nettai_replay::{End, Info, Replay, Tick};
use nettai_replay::{Head, Writer};

use crate::driver::{Driver, Step};

/// The engine's version as a replay's head states it (the frontend's:
/// the engine's crates are versioned together).
pub const ENGINE: &str = env!("CARGO_PKG_VERSION");

/// The set's ticks between two digests a recording keeps (besides each
/// round's end's).
pub const DIGEST_EVERY: u64 = 60;

/// Whether a recording keeps the battle's digest after the set's tick
/// `tick` (counted from 0, through every round): every [`DIGEST_EVERY`]th.
pub fn digest_due(tick: u64) -> bool {
    (tick + 1) % DIGEST_EVERY == 0
}

/// A set being recorded: the replay being written, a tick at a time.
pub struct Recorder {
    writer: Writer<Box<dyn Write + Send>>,
    /// The set's ticks written.
    ticks: u64,
    /// Why writing failed, once it has: the recording stops there (the
    /// file holds the ticks before).
    failed: Option<String>,
}

impl Recorder {
    /// A recording of match `m` (a checked one, with its seed: the set
    /// played is `Set::of(content, m, seed)`) on `content` to `out`, which
    /// gets the replay's head, match and `info` at once and each tick as
    /// it is played.
    pub fn new(out: Box<dyn Write + Send>, content: &Arc<Content>, m: &Match, info: &Info) -> Result<Recorder, String> {
        let seed = m.seed.ok_or("a recorded match states its seed")?;
        let head = Head {
            engine: ENGINE.to_string(),
            game: content.game().to_string(),
            content: content.hash().0,
            start: Set::of(content, m, seed).start().digest(),
        };
        let bytes = nettai_match::binary::match_bytes(content, m)?;
        let writer = Writer::new(out, &head, &bytes, info).map_err(|e| format!("can't write the replay: {e}"))?;
        Ok(Recorder { writer, ticks: 0, failed: None })
    }

    /// Whether the battle's digest is kept after the next tick.
    pub fn digest_due(&self) -> bool {
        digest_due(self.ticks)
    }

    /// The next tick of the set (nothing once writing failed or the set's
    /// end is written).
    pub(crate) fn tick(&mut self, t: &Tick) {
        if self.failed.is_some() || self.writer.ended() {
            return;
        }
        match self.writer.tick(t) {
            Ok(()) => self.ticks += 1,
            Err(e) => self.failed = Some(format!("the replay stopped being written at tick {}: {e}", self.ticks)),
        }
    }

    /// The set's ticks written.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Why writing failed, once it has.
    pub fn failed(&self) -> Option<&str> {
        self.failed.as_deref()
    }
}

/// The digest of `b` as the simulation has it: its round's console the
/// one its setup states (`local_side`), whichever a viewer is shown.
fn simulation_digest(b: &Battle, local_side: u8) -> u64 {
    if b.setup.local_side == local_side {
        return b.digest();
    }
    let mut b = b.clone();
    b.setup.local_side = local_side;
    b.digest()
}

/// Plays a replay back, tick by tick, shown from one side: each tick's
/// buttons as the recording has them, the set going on round after round
/// where it marks a round's end. It compares the battle with what the
/// recording kept (the digests, the marks) and says where it differs
/// (`Driver::check`).
pub struct ReplayPlayer {
    set: Set,
    ticks: Vec<Tick>,
    /// The ticks played.
    at: usize,
    /// The round being played, from 1.
    round: u32,
    /// The side shown.
    viewer: u8,
    /// The side the simulation's setup states.
    local_side: u8,
    end: End,
}

impl ReplayPlayer {
    /// A replay's playback on `content`, shown from `viewer`'s side (none:
    /// the side that recorded it). Refused: a replay of another engine,
    /// game or content, a match that doesn't read or that the checks
    /// refuse, and a first battle other than the recording's.
    pub fn new(content: &Arc<Content>, replay: &Replay, viewer: Option<u8>) -> Result<ReplayPlayer, String> {
        replay.head.require(ENGINE, content.game(), content.hash().0)?;
        let m = nettai_match::binary::read_match(content, &replay.match_bytes).map_err(|e| format!("the replay's match doesn't read: {e}"))?;
        let problems = nettai_match::check_match(content, &m);
        if !problems.is_empty() {
            return Err(format!("the replay's match breaks the rules: {}", problems.join("; ")));
        }
        let set = Set::of(content, &m, m.seed.expect("a replay's match states its seed"));
        let start = set.start().digest();
        if start != replay.head.start {
            return Err(format!(
                "the replay's match makes another battle on this engine (its first battle's digest {start:016x}, the recording's {:016x})",
                replay.head.start
            ));
        }
        let viewer = viewer.unwrap_or(replay.info.side);
        if viewer > 1 {
            return Err(format!("no side {viewer}: a match has sides 0 and 1"));
        }
        let local_side = set.first().local_side;
        Ok(ReplayPlayer { set, ticks: replay.ticks.clone(), at: 0, round: 1, viewer, local_side, end: replay.end })
    }

    /// The tick last played.
    fn last(&self) -> &Tick {
        &self.ticks[self.at - 1]
    }
}

impl Driver for ReplayPlayer {
    fn start(&mut self) -> Battle {
        self.at = 0;
        self.round = 1;
        let mut b = self.set.start();
        b.setup.local_side = self.viewer;
        b
    }

    fn next(&mut self, b: &Battle, _: u16) -> Option<Step> {
        let t = *self.ticks.get(self.at)?;
        self.at += 1;
        // (The buttons are the whole input, as a netplay peer's are.)
        let TickInput { players, events } = nettai_netplay::standin::tick_input(b, t.buttons);
        Some(Step { input: players, events, frame: Some(self.at as u32) })
    }

    fn check(&self, b: &Battle) -> Vec<String> {
        let t = self.last();
        let tick = self.at - 1;
        let mut out = Vec::new();
        if let Some(d) = t.digest {
            let ours = simulation_digest(b, self.local_side);
            if ours != d {
                out.push(format!("the battle's digest after tick {tick} is {ours:016x}, the recording's {d:016x}"));
            }
        }
        let ended = matches!(b.round_end(), Some(RoundEnd::NextRound { .. } | RoundEnd::Over(_)));
        let over = matches!(b.round_end(), Some(RoundEnd::Over(_)));
        match (t.round_ended, ended) {
            (true, false) => out.push(format!("the recording's round {} ends on tick {tick}; the battle's goes on", self.round)),
            (false, true) => out.push(format!("round {} ends on tick {tick}; the recording's goes on", self.round)),
            _ if t.set_ended != over && ended => out.push(format!(
                "the set {} on tick {tick}; the recording's {}",
                if over { "ends" } else { "goes on" },
                if t.set_ended { "ends" } else { "goes on" }
            )),
            _ => {}
        }
        out
    }

    fn round_ended(&mut self, b: &Battle) -> Option<After> {
        if !self.last().round_ended {
            return None;
        }
        let mut after = self.set.after(b, self.viewer)?;
        if let After::Round(next) = &mut after {
            next.setup.local_side = self.viewer;
            self.round += 1;
        }
        Some(after)
    }

    fn position(&self) -> String {
        format!("replay round {} tick {} of {}", self.round, self.at, self.ticks.len())
    }
}

/// What playing a replay to its end made: for a list of replays, which
/// keeps none of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The ticks played.
    pub ticks: u64,
    /// Each round played to its end, its winner by side (none: a draw).
    pub rounds: Vec<Option<u8>>,
    /// The set's result for side 0, if it was played to its end.
    pub result: Option<BattleResult>,
    /// How the recording ends.
    pub end: End,
    /// The first difference from the recording, if any (the replay didn't
    /// reproduce from there).
    pub diverged: Option<String>,
    /// Why playing stopped before the input's end: the engine stopped.
    pub stopped: Option<String>,
}

/// Play a replay to its end on `content`, with no picture and no sound:
/// what happened in it (who won each round and the set), and whether it
/// reproduced. Refused as [`ReplayPlayer::new`] refuses one.
pub fn play_out(content: &Arc<Content>, replay: &Replay) -> Result<Outcome, String> {
    let mut d = ReplayPlayer::new(content, replay, Some(0))?;
    let mut b = d.start();
    let mut out = Outcome { ticks: 0, rounds: Vec::new(), result: None, end: d.end, diverged: None, stopped: None };
    while let Some(step) = d.next(&b, 0) {
        let ticked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.tick(&step.input, step.events)));
        out.ticks += 1;
        if let Err(e) = ticked {
            let why = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            out.stopped = Some(format!("engine stopped at {}: {why}", d.position()));
            break;
        }
        if out.diverged.is_none() {
            let diffs = d.check(&b);
            if !diffs.is_empty() {
                out.diverged = Some(format!("differs from the recording at {}: {}", d.position(), diffs.join("; ")));
            }
        }
        // (Who won a round: the score the next round starts with, side 0's,
        // against the one this round started with.)
        let before = b.setup.score;
        match d.round_ended(&b) {
            None => {}
            Some(After::Round(next)) => {
                let after = next.setup.score;
                out.rounds.push(if after.wins > before.wins {
                    Some(0)
                } else if after.losses > before.losses {
                    Some(1)
                } else {
                    None
                });
                b = *next;
            }
            Some(After::Over(result)) => {
                out.rounds.push(match result {
                    BattleResult::Won => Some(0),
                    BattleResult::Lost => Some(1),
                    _ => None,
                });
                out.result = Some(result);
                break;
            }
            Some(After::Stopped(why)) => {
                out.stopped = Some(format!("engine stopped at {}: {why}", d.position()));
                break;
            }
        }
    }
    Ok(out)
}

/// A sink tests write a replay to and read it back from.
#[cfg(test)]
pub(crate) mod testing {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    pub struct Shared(pub Arc<Mutex<Vec<u8>>>);

    impl Shared {
        pub fn bytes(&self) -> Vec<u8> {
            self.0.lock().unwrap().clone()
        }
    }

    impl Write for Shared {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::Shared;
    use super::*;
    use crate::driver::{LivePlayer, short_set};
    use crate::session::Session;

    /// A short set played live and recorded (the shooter wins it 2-0): the
    /// replay, and the session's ticks.
    fn recorded(content: &Arc<Content>, game: &str) -> (Replay, u64) {
        let m = short_set::of(content, game, 7);
        let sink = Shared::default();
        let mut s = Session::new(Box::new(LivePlayer::new(Set::of(content, &m, 7))));
        let info = Info { when: 1_790_000_000, side: 0, names: ["Lan".into(), String::new()] };
        s.record(Recorder::new(Box::new(sink.clone()), content, &m, &info).unwrap()).unwrap();
        while s.step(short_set::shooter(&s.battle, 0, s.ticks as u32)) {}
        assert_eq!(s.result, Some(BattleResult::Won), "{game}");
        (Replay::read(&sink.bytes()).unwrap(), s.ticks)
    }

    /// A live set recorded plays back to the same end, round by round,
    /// every digest and mark the recording's, from either side's console
    /// (side 1's sees the set lost); and played out without a picture.
    /// Both games.
    #[test]
    fn a_recorded_set_plays_back() {
        for (content, game) in [(nettai_match::testing::exe6_content(), "exe6"), (nettai_match::testing::exe5_content(), "exe5")] {
            let (replay, ticks) = recorded(&content, game);
            assert_eq!((replay.end, replay.ticks.len() as u64, replay.rounds().len()), (End::Set, ticks, 2), "{game}");
            assert!(replay.ticks.iter().filter(|t| t.digest.is_some()).count() as u64 >= ticks / DIGEST_EVERY, "{game}");
            let out = play_out(&content, &replay).unwrap();
            let want = Outcome { ticks, rounds: vec![Some(0), Some(0)], result: Some(BattleResult::Won), end: End::Set, diverged: None, stopped: None };
            assert_eq!(out, want, "{game}");
            for (side, result) in [(0, BattleResult::Won), (1, BattleResult::Lost)] {
                let mut p = Session::new(Box::new(ReplayPlayer::new(&content, &replay, Some(side)).unwrap()));
                assert_eq!(p.battle.setup.local_side, side);
                let mut rounds = 0;
                while p.step(0) {
                    rounds += p.new_round as u32;
                    assert_eq!(p.battle.setup.local_side, side, "{game}: shown from side {side}");
                }
                assert_eq!((p.diverged, p.result, p.finished, rounds, p.ticks), (None, Some(result), true, 1, ticks), "{game} side {side}");
            }
        }
    }

    /// A replay that doesn't play as recorded says where: other buttons at
    /// a tick show at the next digest; a cut file plays to its cut; one of
    /// another game or content isn't played.
    #[test]
    fn a_replay_that_differs_says_where() {
        let content = nettai_match::testing::exe6_content();
        let (replay, _) = recorded(&content, "exe6");
        let mut other = replay.clone();
        let fight = other.ticks.iter().position(|t| t.buttons[0] != 0).unwrap() + 200;
        for t in &mut other.ticks[fight..] {
            t.buttons[0] = 0;
        }
        let out = play_out(&content, &other).unwrap();
        let diverged = out.diverged.unwrap();
        assert!(diverged.contains("the battle's digest after tick") || diverged.contains("ends on tick"), "{diverged}");
        let mut cut = replay.clone();
        cut.ticks.truncate(100);
        cut.end = End::Cut;
        let out = play_out(&content, &cut).unwrap();
        assert_eq!((out.ticks, out.result, out.diverged, out.end), (100, None, None, End::Cut));
        let five = nettai_match::testing::exe5_content();
        assert_eq!(ReplayPlayer::new(&five, &replay, None).err().unwrap(), "the replay is of exe6, not exe5");
        let mut moved = replay.clone();
        moved.head.start ^= 1;
        assert!(ReplayPlayer::new(&content, &moved, None).err().unwrap().starts_with("the replay's match makes another battle on this engine"));
    }
}
