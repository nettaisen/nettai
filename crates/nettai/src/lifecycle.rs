//! The app's life on its system: kept going while a battle runs ([`Awake`]: its time and the display kept on), and
//! coming back after the system stopped it ([`resumed`]).
//!
//! The app coming back: a phone's app made active again (from the background, the app switcher, a call's screen),
//! or its sound given back after something else took it (a call, an alarm, Siri). Either stops the sound's device,
//! which the app then starts again (`nettai_audio::Output::resume`). Polled: [`resumed`] says it once each time.
//! On the desktop and in a browser it never happens.

mod awake;
mod platform;
pub use awake::Awake;
// (What only one system has, public as this module's own: Android's `came_back`.)
#[allow(unused_imports)]
pub use platform::*;

use std::sync::atomic::{AtomicU64, Ordering};

/// How many times the app came back, and how many [`resumed`] has said.
static CAME: AtomicU64 = AtomicU64::new(0);
static SAID: AtomicU64 = AtomicU64::new(0);

/// Whether the app came back since this was last asked. (On iOS the first call starts watching: from the main
/// thread, the window's.)
pub fn resumed() -> bool {
    platform::system::watch();
    let came = CAME.load(Ordering::Relaxed);
    SAID.swap(came, Ordering::Relaxed) != came
}

/// The system says the app came back (its module's watch, or Android's entry point). (Where nothing stops the app,
/// nothing calls it.)
#[allow(dead_code)]
pub(crate) fn came() {
    CAME.fetch_add(1, Ordering::Relaxed);
}
