//! Keeping the app's time and the display on while something runs that must go on (docs/app.md §1): a battle,
//! a link, the screenshot pass. The app holds an [`Awake`] for as long as one does.
//!
//! macOS gives an app that isn't in front, or whose window is covered, the lowest priority and delays its
//! timers ("App Nap"). On a busy machine such an app gets no time at all: a battle in a covered window would
//! stall and its link read as dropped. (The screenshot pass found it: covered by another window while builds
//! loaded the machine, it stopped mid-walk and never went on.) There an `Awake` is an activity that says so, and
//! that keeps the display on (a player on a pad presses no key). A phone dims and locks its screen after a while
//! without a touch, which a player watching a battle or holding a pad doesn't give: there it keeps the screen on
//! (iOS's idle timer turned off; Android's window kept on, `FLAG_KEEP_SCREEN_ON`:
//! android/nettai/lifecycle/NettaiAwake.kt). Windows, Linux and a browser have nothing to hold.

/// What is held, for as long as this is: the system's ([`super::platform`]), let go as this is dropped.
pub struct Awake {
    _held: super::platform::system::Held,
}

impl Awake {
    /// Hold it. (On iOS from the main thread, the window's, as the app's frames are.)
    pub fn new() -> Awake {
        Awake { _held: super::platform::system::Held::new() }
    }
}

impl Default for Awake {
    fn default() -> Awake {
        Awake::new()
    }
}
