//! iOS: the idle timer turned off while an [`crate::lifecycle::Awake`] is held (the screen kept on), and the app made active
//! again or its sound given back, heard through the system's notifications.

use objc2_foundation::{NSNotification, NSNotificationCenter, NSNotificationName};
use std::ptr::NonNull;
use std::sync::Once;

static WATCH: Once = Once::new();

/// The app made active, and its sound session interrupted or given back (each counted: starting the device
/// while it is taken does nothing, and once it is given back starts it), heard for the app's life.
pub fn watch() {
    WATCH.call_once(|| {
        let center = NSNotificationCenter::defaultCenter();
        let came = block2::RcBlock::new(|_: NonNull<NSNotification>| {
            crate::lifecycle::came();
        });
        // SAFETY: the names are the frameworks' constants; the block holds nothing and may run on any thread.
        let names: [Option<&NSNotificationName>; 2] = unsafe {
            [Some(objc2_ui_kit::UIApplicationDidBecomeActiveNotification), objc2_avf_audio::AVAudioSessionInterruptionNotification]
        };
        for name in names.into_iter().flatten() {
            // SAFETY: no object is named, no queue (the block runs where the notification is posted), and the
            // block is sendable (it touches an atomic alone).
            let observer = unsafe { center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &came) };
            // (Heard for the app's life.)
            std::mem::forget(observer);
        }
    });
}

pub struct Held;

impl Held {
    pub fn new() -> Held {
        keep_screen_on(true);
        Held
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        keep_screen_on(false);
    }
}

/// The idle timer off (or on again); what goes wrong is logged.
fn keep_screen_on(on: bool) {
    match objc2::MainThreadMarker::new() {
        Some(main) => objc2_ui_kit::UIApplication::sharedApplication(main).setIdleTimerDisabled(on),
        None => log::warn!("keeping the screen on: not on the main thread"),
    }
}
