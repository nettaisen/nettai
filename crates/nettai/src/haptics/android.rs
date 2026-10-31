//! Android: the window's view performs a keyboard tap (`HapticFeedbackConstants.KEYBOARD_TAP`), posted to the
//! activity's thread by android/nettai/haptics/NettaiHaptics.kt; the app's loop doesn't wait for it. Android plays
//! nothing where Touch feedback is off.

use jni::objects::{JClass, JValue};
use jni::refs::Global;
use jni::{jni_sig, jni_str};
use std::sync::OnceLock;

/// NettaiHaptics's class, found once (by the activity's class loader: a native thread's `FindClass` sees only the
/// system's classes).
static CLASS: OnceLock<Global<JClass<'static>>> = OnceLock::new();

pub fn tap() {
    let tapped = crate::activity::java(|env, activity| {
        let class = match CLASS.get() {
            Some(class) => class,
            None => {
                let loader = env.call_method(activity, jni_str!("getClassLoader"), jni_sig!("()Ljava/lang/ClassLoader;"), &[])?.l()?;
                let name = env.new_string("nettai.haptics.NettaiHaptics")?;
                let class = env.call_method(&loader, jni_str!("loadClass"), jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"), &[JValue::Object(&name)])?.l()?;
                let class = env.cast_local::<JClass>(class)?;
                let global = env.new_global_ref(&class)?;
                CLASS.get_or_init(|| global)
            }
        };
        env.call_static_method(class, jni_str!("tap"), jni_sig!("(Landroid/app/Activity;)V"), &[JValue::Object(activity)])?;
        Ok(())
    });
    if let Err(why) = tapped {
        log::warn!("a haptic tap: {why}");
    }
}
