//! A running battle: its driver, the engine state, and why it stopped.

use crate::driver::{Driver, Ran};
use nettai_battle::Battle;
use nettai_battle::cues::CueAction;

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
    /// The driver ran out of input (the round is over), as opposed to the
    /// engine stopping.
    pub finished: bool,
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
    /// battle itself (netplay); none: the battle's own cues
    /// (`Battle::sound_cues`).
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
        self.diverged = None;
        self.frame = None;
        self.ticks = 0;
    }

    /// Run one tick (one wall-clock frame of a driver that runs the battle
    /// itself). Returns false (and records why) when the battle can't go
    /// on.
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
