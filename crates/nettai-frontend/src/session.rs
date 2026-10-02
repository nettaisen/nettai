//! A running battle: its driver, the engine state, and why it stopped.

use crate::driver::Driver;
use nettai_battle::Battle;

/// Called after every tick with the battle's new state (the window runs
/// its hooks after each tick; sound plugs in here, handing the tick's
/// cues to the audio output).
pub trait TickHook {
    fn after_tick(&mut self, b: &Battle);
}

impl<F: FnMut(&Battle)> TickHook for F {
    fn after_tick(&mut self, b: &Battle) {
        self(b)
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
}

impl Session {
    pub fn new(mut driver: Box<dyn Driver>) -> Session {
        let battle = driver.start();
        Session { driver, battle, stopped: None, finished: false, diverged: None, frame: None, ticks: 0 }
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

    /// Run one tick. Returns false (and records why) when the battle can't
    /// go on.
    pub fn step(&mut self, keys: u16) -> bool {
        if self.stopped.is_some() {
            return false;
        }
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
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".into());
            self.stopped = Some(format!("engine stopped at {}: {msg}", self.driver.position()));
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
