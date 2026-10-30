//! macOS: a process activity that keeps the app's time (no App Nap) and the display on.

use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};

pub struct Held {
    activity: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

impl Held {
    pub fn new() -> Held {
        let options = NSActivityOptions::UserInitiated | NSActivityOptions::LatencyCritical | NSActivityOptions::IdleDisplaySleepDisabled;
        Held { activity: NSProcessInfo::processInfo().beginActivityWithOptions_reason(options, &NSString::from_str("a battle is running")) }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        // SAFETY: the activity is the object `beginActivityWithOptions:reason:` returned, ended once.
        unsafe { NSProcessInfo::processInfo().endActivity(&self.activity) };
    }
}

/// Nothing to watch: a Mac's app isn't stopped.
pub fn watch() {}
