//! Keeping the system from napping the app (docs/app.md §1).
//!
//! macOS gives an app that isn't in front, or whose window is covered, the lowest priority and delays its
//! timers ("App Nap"). On a busy machine such an app gets no time at all: a battle in a covered window would
//! stall and its link read as dropped. (The screenshot pass found it: covered by another window while builds
//! loaded the machine, it stopped mid-walk and never went on.) While something runs that must keep its time, a
//! battle, a link, the pass, the app holds an activity that says so, and that keeps the display on (a player
//! on a pad presses no key). On the other systems there is nothing to hold.

/// The activity, held for as long as this is.
pub struct Awake {
    #[cfg(target_os = "macos")]
    activity: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2::runtime::NSObjectProtocol>>,
}

impl Awake {
    #[cfg(target_os = "macos")]
    pub fn new() -> Awake {
        use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};
        let options = NSActivityOptions::UserInitiated | NSActivityOptions::LatencyCritical | NSActivityOptions::IdleDisplaySleepDisabled;
        Awake { activity: NSProcessInfo::processInfo().beginActivityWithOptions_reason(options, &NSString::from_str("a battle is running")) }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn new() -> Awake {
        Awake {}
    }
}

#[cfg(target_os = "macos")]
impl Drop for Awake {
    fn drop(&mut self) {
        // SAFETY: the activity is the object `beginActivityWithOptions:reason:` returned, ended once.
        unsafe { objc2_foundation::NSProcessInfo::processInfo().endActivity(&self.activity) };
    }
}
