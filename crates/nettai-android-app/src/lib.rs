//! The nettai app on Android: the library the APK's activity loads (`libnettai_app.so`, the manifest's
//! `android.app.lib_name`; the activity is a NativeActivity: android/nettai/app/NettaiActivity.kt), whose
//! [`android_main`] android-activity calls on a thread of its own once the activity is made. android-activity puts
//! what the app writes to stdout and stderr on logcat (tagged RustStdoutStderr), and a panic of this thread too
//! (RustPanic). packaging/android/package.sh makes the APK (docs/app.md §7). Elsewhere it is empty.
#![cfg(target_os = "android")]

use android_activity::AndroidApp;

/// The app, under tokio as the desktop's `main` runs it (crates/nettai/src/main.rs), on this thread, which runs the
/// activity's event loop (winit's, through Slint) till the activity is gone. First the activity is handed over
/// (everything the app asks of Java goes through it: `nettai_android`), and the certificate verifier given it.
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    nettai_android::init(app);
    let verifier = nettai_android::java(|env, activity| {
        let context = env.new_local_ref(activity)?;
        rustls_platform_verifier::android::init_with_env(env, context)
    });
    if let Err(why) = verifier {
        eprintln!("nettai: setting up the certificate verifier: {why}");
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build() {
        Ok(runtime) => runtime,
        Err(why) => return eprintln!("nettai: starting tokio: {why}"),
    };
    if let Err(why) = runtime.block_on(async { tokio::task::block_in_place(nettai::run) }) {
        eprintln!("nettai: the app stopped: {why}");
    }
}
