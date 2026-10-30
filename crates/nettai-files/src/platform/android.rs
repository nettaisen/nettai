//! Android's folders: the app's files and cache folders (`Context.getFilesDir`, `getCacheDir`), asked of the
//! activity through JNI (ndk-context's, which android-activity sets as the app starts); what goes wrong is logged.

use jni::objects::{JObject, JString};
use jni::{jni_sig, jni_str};
use std::path::PathBuf;

pub fn app_data() -> Option<PathBuf> {
    folder("getFilesDir").map_err(|why| log::warn!("finding the app's files folder: {why}")).ok()
}

pub fn temporary() -> Option<PathBuf> {
    folder("getCacheDir").map_err(|why| log::warn!("finding the app's cache folder: {why}")).ok()
}

/// The path of the `java.io.File` that the activity's method `name` (no arguments) gives.
fn folder(name: &'static str) -> Result<PathBuf, jni::errors::Error> {
    let context = ndk_context::android_context();
    // SAFETY: the process's VM, which android-activity hands ndk-context as the app starts.
    let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: the activity, which android-activity holds as a global reference for the app's life.
        let activity = unsafe { JObject::from_raw(env, context.context().cast()) };
        let method = jni::strings::JNIString::from(name);
        let file = env.call_method(&activity, &method, jni_sig!("()Ljava/io/File;"), &[])?.l()?;
        let path = env.call_method(&file, jni_str!("getAbsolutePath"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        let path = env.cast_local::<JString>(path)?;
        Ok(PathBuf::from(path.try_to_string(env)?))
    })
}
