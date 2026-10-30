//! Android's side of the app: the app's activity, which the program's entry point hands over before anything else
//! ([`init`]), Java's calls through JNI, from any thread ([`java`]), which the other crates' Android parts use
//! (nettai-files' folders, nettai-dialogs' pickers, nettai-lifecycle's screen kept on), and the safe area the
//! activity hands over ([`safe_area`]). On any other system the crate is empty.
#![cfg(target_os = "android")]

use android_activity::AndroidApp;
use jni::objects::JObject;
use jni::sys::jint;
use jni::EnvUnowned;
use std::sync::{Mutex, OnceLock};

static APP: OnceLock<AndroidApp> = OnceLock::new();

/// The safe area's insets, as the activity last handed them over.
static SAFE_AREA: Mutex<Option<[i32; 4]>> = Mutex::new(None);

/// Keep the app's activity: the entry point's first call (`android_main` is given it).
pub fn init(app: AndroidApp) {
    let _ = APP.set(app);
}

/// The app's activity, as [`init`] kept it.
pub fn app() -> Result<&'static AndroidApp, String> {
    APP.get().ok_or_else(|| "the app's activity wasn't handed over (nettai_android::init)".to_string())
}

/// `call` run with Java's environment (this thread attached to the VM, for as long as it lives) and the activity (an
/// `android.app.Activity`, which is a `Context`). What Java threw, or what went wrong, is the error, said.
pub fn java<T>(call: impl FnOnce(&mut jni::Env, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
    let app = app()?;
    // SAFETY: the VM is the process's, valid for its life.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: android-activity holds the activity as a global reference for the app's life.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        call(env, &activity)
    })
    .map_err(|e: jni::errors::Error| e.to_string())
}

/// The path of the `java.io.File` that the activity's method `name` (no arguments) gives: its files folder
/// (`getFilesDir`), its cache folder (`getCacheDir`).
pub fn folder(name: &'static str) -> Result<std::path::PathBuf, String> {
    java(|env, activity| {
        let method = jni::strings::JNIString::from(name);
        let file = env.call_method(activity, &method, jni::jni_sig!("()Ljava/io/File;"), &[])?.l()?;
        let path = env.call_method(&file, jni::jni_str!("getAbsolutePath"), jni::jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        let path = env.cast_local::<jni::objects::JString>(path)?;
        Ok(std::path::PathBuf::from(path.try_to_string(env)?))
    })
}

/// The safe area's insets in the window's pixels (left, top, right, bottom): how far Android's bars and cutouts
/// reach into the window, which the app's own screens keep out of. Android says them to the activity
/// (crates/nettai-android-app/android/nettai/app/NettaiActivity.kt), which hands them over on its main thread
/// whenever they change: none till it first has.
pub fn safe_area() -> Option<[i32; 4]> {
    *SAFE_AREA.lock().unwrap_or_else(|e| e.into_inner())
}

/// `NettaiActivity.safeArea`: the safe area's insets as Android says them now.
#[unsafe(no_mangle)]
extern "system" fn Java_nettai_app_NettaiActivity_safeArea<'local>(_env: EnvUnowned<'local>, _activity: JObject<'local>, left: jint, top: jint, right: jint, bottom: jint) {
    *SAFE_AREA.lock().unwrap_or_else(|e| e.into_inner()) = Some([left, top, right, bottom]);
}
