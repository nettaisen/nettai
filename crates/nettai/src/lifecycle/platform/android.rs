//! Android: the window kept on while an [`crate::lifecycle::Awake`] is held (`FLAG_KEEP_SCREEN_ON`, set by
//! android/nettai/lifecycle/NettaiAwake.kt on the activity's thread), and the activity resumed, which the entry
//! point says ([`came_back`]).

/// Android's activity resumed (`MainEvent::Resume`): the entry point's event listener says so, as Slint's Android
/// backend hears the activity's events.
pub fn came_back() {
    crate::lifecycle::came();
}

/// Nothing to watch: the entry point says when the activity resumes.
pub fn watch() {}

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

/// The window kept on (or let go); what goes wrong is logged. (Not android-activity's `set_window_flags`: it takes
/// the activity's state to write, which android-activity holds to read while the app's loop runs, where this is
/// called, and waits for itself.)
fn keep_screen_on(on: bool) {
    let kept = crate::activity::java(|env, activity| {
        use jni::objects::{JClass, JValue};
        use jni::{jni_sig, jni_str};
        // (By the activity's class loader: a native thread's `FindClass` sees only the system's classes.)
        let loader = env.call_method(activity, jni_str!("getClassLoader"), jni_sig!("()Ljava/lang/ClassLoader;"), &[])?.l()?;
        let name = env.new_string("nettai.lifecycle.NettaiAwake")?;
        let class = env.call_method(&loader, jni_str!("loadClass"), jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"), &[JValue::Object(&name)])?.l()?;
        let class = env.cast_local::<JClass>(class)?;
        env.call_static_method(&class, jni_str!("keepScreenOn"), jni_sig!("(Landroid/app/Activity;Z)V"), &[JValue::Object(activity), JValue::Bool(on)])?;
        Ok(())
    });
    if let Err(why) = kept {
        log::warn!("keeping the screen on: {why}");
    }
}
