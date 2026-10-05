//! A running battle: its driver, the engine state, and why it stopped.

use crate::driver::{Driver, Ran, result_text};
use nettai_battle::cues::CueAction;
use nettai_battle::{Battle, BattleResult};
use nettai_match::After;

/// Called after every step with the session (the window runs its hooks
/// after each one; sound plugs in here, handing the step's cues, or a
/// netplay frame's cue actions, to the audio output).
pub trait TickHook {
    fn after_tick(&mut self, s: &Session);
}

impl<F: FnMut(&Session)> TickHook for F {
    fn after_tick(&mut self, s: &Session) {
        self(s)
    }
}

pub struct Session {
    pub driver: Box<dyn Driver>,
    pub battle: Battle,
    /// Why the battle stopped (engine panic, end of the trace).
    pub stopped: Option<String>,
    /// The driver ran out of input (a trace's round is over) or the set
    /// was played to its end, as opposed to the engine stopping.
    pub finished: bool,
    /// The set's result for the local player, once it is over (a set
    /// played to its end).
    pub result: Option<BattleResult>,
    /// The first difference from the trace, once seen.
    pub diverged: Option<String>,
    /// Trace frame of the latest tick, if the driver has frame numbers.
    pub frame: Option<u32>,
    pub ticks: u64,
    /// The last step showed a new frame (a netplay frame can wait).
    pub fresh: bool,
    /// The last step started a new round (the presentation starts over).
    pub new_round: bool,
    /// The last step's sound as cue actions, from a driver that runs the
    /// battle itself (netplay), and of a step that ended a round (the
    /// ended round's last cues: `battle` is the next round's by then);
    /// none: the battle's own cues (`Battle::sound_cues`).
    pub sound: Option<Vec<CueAction>>,
}

impl Session {
    pub fn new(mut driver: Box<dyn Driver>) -> Session {
        let battle = driver.start();
        Session {
            driver,
            battle,
            stopped: None,
            finished: false,
            result: None,
            diverged: None,
            frame: None,
            ticks: 0,
            fresh: false,
            new_round: false,
            sound: None,
        }
    }

    /// Start over.
    pub fn restart(&mut self) {
        self.battle = self.driver.start();
        self.stopped = None;
        self.finished = false;
        self.result = None;
        self.diverged = None;
        self.frame = None;
        self.ticks = 0;
    }

    /// Run one tick (one wall-clock frame of a driver that runs the battle
    /// itself). Returns false (and records why) when the battle can't go
    /// on. (The tick that ends a set runs and is shown as any other; it is
    /// the step after it that returns false.)
    pub fn step(&mut self, keys: u16) -> bool {
        if self.stopped.is_some() {
            return false;
        }
        IN_ENGINE.with(|f| f.set(true));
        let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.driver.run_frame(keys, &mut self.battle)));
        IN_ENGINE.with(|f| f.set(false));
        match ran {
            Ok(None) => {}
            Ok(Some(Ok(Ran { advanced, new_round, sound }))) => {
                self.fresh = advanced;
                self.new_round = new_round;
                self.sound = Some(sound);
                if advanced {
                    self.ticks += 1;
                }
                return true;
            }
            Ok(Some(Err(why))) => {
                self.stopped = Some(why);
                return false;
            }
            Err(e) => {
                self.stopped = Some(format!("engine stopped at {}: {}", self.driver.position(), panic_message(e.as_ref())));
                return false;
            }
        }
        self.fresh = true;
        self.new_round = false;
        self.sound = None;
        let Some(step) = self.driver.next(&self.battle, keys) else {
            self.stopped = Some(format!("end of input at {}", self.driver.position()));
            self.finished = true;
            return false;
        };
        let battle = &mut self.battle;
        IN_ENGINE.with(|f| f.set(true));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| battle.tick(&step.input, step.events)));
        IN_ENGINE.with(|f| f.set(false));
        self.frame = step.frame;
        self.ticks += 1;
        if let Err(e) = result {
            self.stopped = Some(format!("engine stopped at {}: {}", self.driver.position(), panic_message(e.as_ref())));
            return false;
        }
        if self.diverged.is_none() {
            let diffs = self.driver.check(&self.battle);
            if !diffs.is_empty() {
                self.diverged = Some(format!("differs from the trace at {}:\n  {}", self.driver.position(), diffs.join("\n  ")));
            }
        }
        // A set's round ended on this tick: on with the next one, or the
        // set is over.
        match self.driver.round_ended(&self.battle) {
            None => {}
            Some(After::Round(next)) => {
                // (The ended round's last tick is still heard; what is
                // shown is the next round, at its start, as a netplay
                // peer shows it.)
                self.sound = Some(self.battle.sound_cues().iter().map(|&cue| CueAction::Play(cue)).collect());
                self.battle = *next;
                self.new_round = true;
            }
            Some(After::Over(result)) => {
                self.result = Some(result);
                self.stopped = Some(format!("the match is over: {}", result_text(result)));
                self.finished = true;
            }
            Some(After::Stopped(why)) => {
                self.stopped = Some(format!("engine stopped at {}: {why}", self.driver.position()));
            }
        }
        true
    }
}

fn panic_message(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| "unknown panic".into())
}

thread_local! {
    static IN_ENGINE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Keep engine panics off stderr (the session reports them); other panics
/// print as usual.
pub fn quiet_engine_panics() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if !IN_ENGINE.with(|f| f.get()) {
            default(info);
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::{LivePlayer, short_set};
    use nettai_match::Set;

    /// What a session of a short set showed, played by the shooter to its
    /// end: the session, and at each new round's start its frame number,
    /// the score it starts with (rounds played, wins, losses), its stage and
    /// whether the ended round's last tick was still heard.
    fn played(game: &str) -> (Session, Vec<(u32, (u8, u8, u8), nettai_content_api::StageHandle, bool)>, nettai_match::Match) {
        let content = if game == "exe5" { nettai_match::testing::exe5_content() } else { nettai_match::testing::exe6_content() };
        let m = short_set::of(&content, game, 7);
        let mut s = Session::new(Box::new(LivePlayer::new(Set::of(&content, &m, 7))));
        let mut rounds = Vec::new();
        while s.step(short_set::shooter(&s.battle, 0, s.ticks as u32)) {
            assert!(s.ticks < 30_000, "{game}: the set doesn't end ({})", s.driver.position());
            if s.new_round {
                let r = &s.battle.round;
                assert_eq!(r.ticks, 0, "{game}: the next round is shown at its start");
                rounds.push((s.frame.unwrap(), (r.round, r.wins, r.losses), s.battle.setup.settings.stage, s.sound.is_some()));
            }
        }
        (s, rounds, m)
    }

    /// Offline play goes on after a round: the next one starts with the
    /// score carried, on the arena's next stage, the presentation told to
    /// start over (`new_round`), and the frames numbered on from the round
    /// before. Both games.
    #[test]
    fn a_live_set_goes_on_to_its_next_round() {
        for game in ["exe6", "exe5"] {
            let (s, rounds, m) = played(game);
            let (frame, score, stage, heard) = rounds[0];
            assert_eq!(score, (1, 1, 0), "{game}: one round played, won");
            assert_eq!(stage, m.arena.later[0].stage, "{game}");
            assert!(heard && frame > 100, "{game}: frame {frame}");
            assert!(s.driver.position().starts_with("live round 2 tick "), "{game}: {}", s.driver.position());
        }
    }

    /// And to the set's end: two rounds won decide it, and the session is
    /// finished with the result; no tick runs after it. Starting over
    /// plays the set again from its first round.
    #[test]
    fn a_live_set_ends_with_its_result() {
        for game in ["exe6", "exe5"] {
            let (mut s, rounds, _) = played(game);
            assert_eq!(rounds.len(), 1, "{game}: two rounds decide a set won 2-0");
            assert_eq!((s.finished, s.result), (true, Some(BattleResult::Won)), "{game}");
            assert_eq!(s.stopped.as_deref(), Some("the match is over: you won"), "{game}");
            let ticks = s.ticks;
            assert!(!s.step(0) && s.ticks == ticks, "{game}");
            s.restart();
            assert_eq!((s.finished, s.result, s.stopped.as_deref()), (false, None, None), "{game}");
            assert_eq!(s.driver.position(), "live round 1 tick 0", "{game}");
            assert_eq!((s.battle.round.round, s.battle.round.wins), (0, 0), "{game}");
            assert!(s.step(0), "{game}");
        }
    }
}
