//! Android's updates: the newest release's APK (`nettai-android.apk`, every release's signed with the one release key:
//! packaging/android), asked of GitHub's API (`release.rs`), downloaded into the app's cache folder on the event
//! loop's thread as the games' content is (`http.rs`: reqwest), checked by the SHA-256 GitHub says, and handed to the
//! system's installer by android/nettai/updates/NettaiUpdates.kt: a session of Android's `PackageInstaller` that asks
//! for no confirmation (`USER_ACTION_NOT_REQUIRED`: the app updates itself, with `UPDATE_PACKAGES_WITHOUT_USER_ACTION`,
//! and targets API 33 or later). Android installs it without a word, and ends the app; the player opens it again. The
//! first time, where the player hasn't let the app install apps (Settings' "Install unknown apps", which
//! `REQUEST_INSTALL_PACKAGES` asks for), Android asks: its screen leads to the setting, then asks to update. Android
//! checks the APK is signed with the installed app's key.

use super::{Told, tell};
use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};

pub use super::release::Found;

/// The newest published release, as GitHub's API says it.
const LATEST: &str = "https://api.github.com/repos/nettaisen/nettai/releases/latest";

/// The most of the release's JSON read (its assets and its notes: some tens of kilobytes).
const MAX_RELEASE: u64 = 1 << 20;

/// `PackageInstaller.STATUS_FAILURE_ABORTED`: the player turned the update down.
const ABORTED: jint = 3;

pub fn startup() {}

/// Ask GitHub's API for the newest release, on the event loop (reqwest's, under tokio there).
pub fn check() -> bool {
    let started = slint::spawn_local(async {
        let told = match ask().await {
            Ok(Some(found)) => Told::Found(found),
            Ok(None) => Told::None,
            Err(why) => {
                crate::errors::log("checking for an update", &why);
                Told::None
            }
        };
        tell(told);
    });
    if let Err(why) = &started {
        crate::errors::log("checking for an update", why);
    }
    started.is_ok()
}

async fn ask() -> Result<Option<Found>, String> {
    let release = crate::http::get(LATEST, MAX_RELEASE).await?;
    super::release::newer(&release, env!("CARGO_PKG_VERSION"))
}

/// Where the update's APK is downloaded: the app's cache folder.
fn apk() -> Result<PathBuf, String> {
    let dir = nettai_files::folders::temporary().ok_or("the app has no cache folder")?;
    Ok(dir.join("nettai-update.apk"))
}

/// Download `found`'s APK, on the event loop, telling how far.
pub fn download(found: &Found) {
    let found = found.clone();
    let started = slint::spawn_local(async move {
        let path = match apk() {
            Ok(path) => path,
            Err(why) => return tell(Told::Downloaded(Err(why))),
        };
        let done = fetch(&found, &path).await;
        if done.is_err() {
            let _ = std::fs::remove_file(&path);
        }
        tell(Told::Downloaded(done));
    });
    if let Err(why) = started {
        tell(Told::Downloaded(Err(format!("the download couldn't start: {why}"))));
    }
}

/// `found`'s APK written to `path`, its SHA-256 the one the release says.
async fn fetch(found: &Found, path: &Path) -> Result<(), String> {
    let written = |e: std::io::Error| format!("{}: {e}", path.display());
    let mut file = std::fs::File::create(path).map_err(written)?;
    let mut sum = Sha256::new();
    let mut said = 0;
    crate::http::get_each(&found.url, found.size, |piece, got| {
        sum.update(piece);
        file.write_all(piece).map_err(written)?;
        let percent = (got * 100 / found.size.max(1)) as i16;
        if percent != said {
            said = percent;
            tell(Told::Progress(percent));
        }
        Ok(())
    })
    .await?;
    let sum = format!("{:x}", sum.finalize());
    if sum != found.sha256 {
        return Err(format!("the APK downloaded has the SHA-256 {sum}, where the release says {}", found.sha256));
    }
    Ok(())
}

/// Hand the APK to the system's installer, on a thread (it copies the APK into its session): its answer comes back
/// through `NettaiUpdates.ended` unless the update is installed, which ends the app.
pub fn apply(found: Found) -> Result<(), String> {
    let path = apk()?;
    crate::errors::log("installing an update", &format!("nettai {}", found.version));
    let spawned = nettai_threads::spawn("installing an update", move || {
        let started = install(&path);
        // (The session has its own copy by now, or there is none.)
        let _ = std::fs::remove_file(&path);
        if let Err(why) = started {
            tell(Told::NotApplied { why, declined: false });
        }
    });
    spawned.map_err(|e| format!("a thread for the update: {e}"))
}

/// `NettaiUpdates.install`: the session made, the APK written into it, and committed; or why not.
fn install(apk: &Path) -> Result<(), String> {
    let refused = crate::activity::java(|env, activity| {
        let class = helper(env, activity)?;
        let path = env.new_string(apk.to_string_lossy())?;
        let refused = env
            .call_static_method(
                &class,
                jni_str!("install"),
                jni_sig!("(Landroid/app/Activity;Ljava/lang/String;)Ljava/lang/String;"),
                &[JValue::Object(activity), JValue::Object(&path)],
            )?
            .l()?;
        if refused.is_null() {
            return Ok(None);
        }
        let refused = env.cast_local::<JString>(refused)?;
        Ok(Some(refused.try_to_string(env)?))
    })?;
    refused.map_or(Ok(()), Err)
}

/// `NettaiUpdates`, by the activity's class loader (a native thread's `FindClass` sees only the system's classes),
/// its native method registered (the library is the activity's, loaded natively, which Java's lookup by name
/// doesn't search).
fn helper<'local>(env: &mut Env<'local>, activity: &JObject) -> jni::errors::Result<JClass<'local>> {
    let loader = env.call_method(activity, jni_str!("getClassLoader"), jni_sig!("()Ljava/lang/ClassLoader;"), &[])?.l()?;
    let name = env.new_string("nettai.updates.NettaiUpdates")?;
    let class = env.call_method(&loader, jni_str!("loadClass"), jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"), &[JValue::Object(&name)])?.l()?;
    let class = env.cast_local::<JClass>(class)?;
    // SAFETY: the function's parameters are what its signature says: (JNIEnv*, jclass, jint, jstring).
    unsafe {
        env.register_native_methods(
            &class,
            &[NativeMethod::from_raw_parts(jni_str!("ended"), jni_str!("(ILjava/lang/String;)V"), ended as *mut std::ffi::c_void)],
        )?;
    }
    Ok(class)
}

/// `NettaiUpdates.ended`: the installer's answer, where the update wasn't installed (`PackageInstaller`'s status and
/// its message).
extern "system" fn ended<'local>(mut env: EnvUnowned<'local>, _class: JClass<'local>, status: jint, message: JString<'local>) {
    let message = env.with_env(|env| message.try_to_string(env)).resolve::<jni::errors::LogErrorAndDefault>();
    tell(Told::NotApplied { why: format!("{message} (the installer's status {status})"), declined: status == ABORTED });
}
