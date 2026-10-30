//! Keeping the app's time and the display on while something runs that must go on (docs/app.md §1): a battle,
//! a link, the screenshot pass. The app holds an [`Awake`] for as long as one does.
//!
//! macOS gives an app that isn't in front, or whose window is covered, the lowest priority and delays its
//! timers ("App Nap"). On a busy machine such an app gets no time at all: a battle in a covered window would
//! stall and its link read as dropped. (The screenshot pass found it: covered by another window while builds
//! loaded the machine, it stopped mid-walk and never went on.) There an `Awake` is an activity that says so, and
//! that keeps the display on (a player on a pad presses no key). A phone dims and locks its screen after a while
//! without a touch, which a player watching a battle or holding a pad doesn't give: there it keeps the screen on
//! (iOS's idle timer turned off; Android's window kept on, `FLAG_KEEP_SCREEN_ON`: android/nettai/lifecycle/NettaiAwake.kt). Windows, Linux and a browser
//! have nothing to hold.

/// What is held, for as long as this is.
pub struct Awake {
    #[cfg(target_os = "macos")]
    activity: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2::runtime::NSObjectProtocol>>,
}

impl Awake {
    /// Hold it. (On iOS from the main thread, the window's, as the app's frames are.)
    pub fn new() -> Awake {
        #[cfg(target_os = "macos")]
        {
            use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};
            let options = NSActivityOptions::UserInitiated | NSActivityOptions::LatencyCritical | NSActivityOptions::IdleDisplaySleepDisabled;
            Awake { activity: NSProcessInfo::processInfo().beginActivityWithOptions_reason(options, &NSString::from_str("a battle is running")) }
        }
        #[cfg(not(target_os = "macos"))]
        {
            keep_screen_on(true);
            Awake {}
        }
    }
}

impl Default for Awake {
    fn default() -> Awake {
        Awake::new()
    }
}

impl Drop for Awake {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        // SAFETY: the activity is the object `beginActivityWithOptions:reason:` returned, ended once.
        unsafe {
            objc2_foundation::NSProcessInfo::processInfo().endActivity(&self.activity)
        };
        #[cfg(not(target_os = "macos"))]
        keep_screen_on(false);
    }
}

/// A phone's screen kept on (or let go); what goes wrong is logged. Elsewhere nothing.
#[cfg(not(target_os = "macos"))]
fn keep_screen_on(on: bool) {
    #[cfg(target_os = "ios")]
    match objc2::MainThreadMarker::new() {
        Some(main) => objc2_ui_kit::UIApplication::sharedApplication(main).setIdleTimerDisabled(on),
        None => log::warn!("keeping the screen on: not on the main thread"),
    }
    // (Not android-activity's `set_window_flags`: it takes the activity's state to write, which android-activity
    // holds to read while the app's loop runs, where this is called, and waits for itself.)
    #[cfg(target_os = "android")]
    if let Err(why) = nettai_android::java(|env, activity| {
        use jni::objects::{JClass, JValue};
        use jni::{jni_sig, jni_str};
        // (By the activity's class loader: a native thread's `FindClass` sees only the system's classes.)
        let loader = env.call_method(activity, jni_str!("getClassLoader"), jni_sig!("()Ljava/lang/ClassLoader;"), &[])?.l()?;
        let name = env.new_string("nettai.lifecycle.NettaiAwake")?;
        let class = env.call_method(&loader, jni_str!("loadClass"), jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"), &[JValue::Object(&name)])?.l()?;
        let class = env.cast_local::<JClass>(class)?;
        env.call_static_method(&class, jni_str!("keepScreenOn"), jni_sig!("(Landroid/app/Activity;Z)V"), &[JValue::Object(activity), JValue::Bool(on)])?;
        Ok(())
    }) {
        log::warn!("keeping the screen on: {why}");
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    let _ = on;
}
