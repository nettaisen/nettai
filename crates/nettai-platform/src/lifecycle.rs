//! The app coming back: a phone's app made active again (from the background, the app switcher, a call's screen),
//! or its sound given back after something else took it (a call, an alarm, Siri). Either stops the sound's device,
//! which the app then starts again (`nettai_audio::Output::resume`). Polled: [`resumed`] says it once each time.
//! On the desktop and in a browser it never happens.

use std::sync::atomic::{AtomicU64, Ordering};

/// How many times the app came back, and how many [`resumed`] has said.
static CAME: AtomicU64 = AtomicU64::new(0);
static SAID: AtomicU64 = AtomicU64::new(0);

/// Whether the app came back since this was last asked. (On iOS the first call starts watching: from the main
/// thread, the window's.)
pub fn resumed() -> bool {
    #[cfg(target_os = "ios")]
    ios::watch();
    let came = CAME.load(Ordering::Relaxed);
    SAID.swap(came, Ordering::Relaxed) != came
}

/// Android's activity resumed (`MainEvent::Resume`): the entry point's event listener says so, as Slint's Android
/// backend hears the activity's events.
#[cfg(target_os = "android")]
pub fn came_back() {
    CAME.fetch_add(1, Ordering::Relaxed);
}

#[cfg(target_os = "ios")]
mod ios {
    use super::CAME;
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSNotificationName};
    use std::ptr::NonNull;
    use std::sync::Once;
    use std::sync::atomic::Ordering;

    static WATCH: Once = Once::new();

    /// The app made active, and its sound session interrupted or given back (each counted: starting the device
    /// while it is taken does nothing, and once it is given back starts it), heard for the app's life.
    pub fn watch() {
        WATCH.call_once(|| {
            let center = NSNotificationCenter::defaultCenter();
            let came = block2::RcBlock::new(|_: NonNull<NSNotification>| {
                CAME.fetch_add(1, Ordering::Relaxed);
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
}
