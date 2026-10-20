//! The app's clock: what "now" is for everything timed against a frame's time (a screen's change, a change of
//! game's sweep, the cards' time up, a choice's delay). It is the system's, but in the showreel (`NETTAI_REEL`, the
//! `tour` feature's: `tour/reel.rs`), which runs the app a frame at a time on a clock of its own whatever each frame
//! takes to capture, and whose frames' times are that clock's.

use nettai_platform::time::Instant;

/// Now.
pub fn now() -> Instant {
    #[cfg(feature = "tour")]
    if let Some(now) = crate::tour::reel::now() {
        return now;
    }
    Instant::now()
}
