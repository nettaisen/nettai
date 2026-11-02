//! Android's updates: the newest release's APK (`nettai-android.apk`, every release's signed with the one release key:
//! packaging/android), asked of GitHub's API (`newer`) against the installed app's version (the APK's
//! `versionName`, as Android says it), downloaded into the app's cache folder, checked by the SHA-256 GitHub says, and
//! handed to the system's installer by `android/nettai/updater/NettaiUpdater.kt`: a session of Android's
//! `PackageInstaller` that asks for no confirmation (`USER_ACTION_NOT_REQUIRED`: the app updates itself, with
//! `UPDATE_PACKAGES_WITHOUT_USER_ACTION`, and targets API 33 or later). Android installs it without a word, and ends
//! the app; the player opens it again. The first time, where the player hasn't let the app install apps (Settings'
//! "Install unknown apps", which `REQUEST_INSTALL_PACKAGES` asks for), Android asks: its screen leads to the setting,
//! then asks to update. Android checks the APK is signed with the installed app's key.
//!
//! The release and the APK come over reqwest (as the app's own files do: rustls with ring's provider, Android's
//! certificate checks, which the app sets up as it starts), each on a thread of its own with a tokio runtime of its
//! own. The installer's answer comes back through the native method registered here, on Java's main thread.

use crate::Told;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// The newest published release, as GitHub's API says it.
const LATEST: &str = "https://api.github.com/repos/nettaisen/nettai/releases/latest";

/// The most of the release's JSON read (its assets and its notes: some tens of kilobytes).
const MAX_RELEASE: u64 = 1 << 20;

/// The APK each release carries, signed with the release key (packaging/android).
const APK: &str = "nettai-android.apk";

/// A newer release's APK.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    /// The release's version (its tag's, without the `v`).
    pub version: String,
    /// Where the APK is downloaded from.
    pub url: String,
    /// The APK's size, in bytes.
    pub size: u64,
    /// The APK's SHA-256, as GitHub says it (in hexadecimal, lowercase).
    pub sha256: String,
}

impl Found {
    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    browser_download_url: String,
    /// `sha256:` and the file's SHA-256 in hexadecimal (GitHub's, on every release asset since 2025).
    digest: Option<String>,
}

/// The release's APK, if the release (GitHub's JSON for it) is newer than `installed`, the installed app's version; a
/// release that has none, or doesn't say its SHA-256, is an error.
pub fn newer(json: &[u8], installed: &str) -> Result<Option<Found>, String> {
    let release: Release = serde_json::from_slice(json).map_err(|e| format!("the release's JSON: {e}"))?;
    let tag = release.tag_name;
    let version = tag.strip_prefix('v').unwrap_or(&tag);
    let (Some(out), Some(ours)) = (numbers(version), numbers(installed)) else {
        return Err(format!("the release's version {version} and the app's {installed} don't compare"));
    };
    if out <= ours {
        return Ok(None);
    }
    let apk = release.assets.into_iter().find(|a| a.name == APK).ok_or_else(|| format!("{tag} has no {APK}"))?;
    let sha256 = apk.digest.as_deref().and_then(|d| d.strip_prefix("sha256:"));
    let sha256 = sha256.ok_or_else(|| format!("{tag}'s {APK} has no SHA-256"))?.to_ascii_lowercase();
    Ok(Some(Found { version: version.to_owned(), url: apk.browser_download_url, size: apk.size, sha256 }))
}

/// A version's numbers: major, minor, patch.
fn numbers(version: &str) -> Option<[u64; 3]> {
    let mut parts = version.split('.').map(|part| part.parse().ok());
    let numbers = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(numbers)
}

/// How long a connection may take to be made, and a download may hear nothing, before it fails (and the loading
/// screen goes on).
const CONNECT_WAIT: Duration = Duration::from_secs(10);
const READ_WAIT: Duration = Duration::from_secs(30);

/// `PackageInstaller.STATUS_FAILURE_ABORTED`: the player turned the update down.
const ABORTED: jint = 3;

/// Where the installer's answer goes, while the update is being installed.
static ANSWER: Mutex<Option<Sender<Told>>> = Mutex::new(None);

pub fn startup() {}

/// Ask GitHub's API for the newest release, on a thread.
pub fn check(tell: Sender<Told>) -> bool {
    let started = on_thread("nettai-updater: checking", async move {
        let told = match ask().await {
            Ok(Some(found)) => Told::Found(found),
            Ok(None) => Told::None,
            Err(why) => {
                log::warn!("checking for an update: {why}");
                Told::None
            }
        };
        let _ = tell.send(told);
    });
    if let Err(why) = &started {
        log::warn!("checking for an update: {why}");
    }
    started.is_ok()
}

async fn ask() -> Result<Option<Found>, String> {
    let installed = installed()?;
    let mut json = Vec::new();
    get(LATEST, MAX_RELEASE, |piece, _| {
        json.extend_from_slice(piece);
        Ok(())
    })
    .await?;
    newer(&json, &installed)
}

/// Where the update's APK is downloaded: the app's cache folder.
fn apk() -> Result<PathBuf, String> {
    let dir = nettai_files::folders::temporary().ok_or("the app has no cache folder")?;
    Ok(dir.join("nettai-update.apk"))
}

/// Download `found`'s APK on a thread, telling how far.
pub fn download(found: &Found, tell: Sender<Told>) {
    let found = found.clone();
    let failed = tell.clone();
    let started = on_thread("nettai-updater: downloading", async move {
        let done = match apk() {
            Ok(path) => {
                let fetched = fetch(&found, &path, &tell).await;
                if fetched.is_err() {
                    let _ = std::fs::remove_file(&path);
                }
                fetched
            }
            Err(why) => Err(why),
        };
        let _ = tell.send(Told::Downloaded(done));
    });
    if let Err(why) = started {
        let _ = failed.send(Told::Downloaded(Err(why)));
    }
}

/// `found`'s APK written to `path`, its SHA-256 the one the release says.
async fn fetch(found: &Found, path: &Path, tell: &Sender<Told>) -> Result<(), String> {
    let written = |e: std::io::Error| format!("{}: {e}", path.display());
    let mut file = std::fs::File::create(path).map_err(written)?;
    let mut sum = Sha256::new();
    let mut said = 0;
    get(&found.url, found.size, |piece, got| {
        sum.update(piece);
        file.write_all(piece).map_err(written)?;
        // (Told by the percent: no more often.)
        let percent = got * 100 / found.size.max(1);
        if percent != said {
            said = percent;
            let _ = tell.send(Told::Progress(percent as f32 / 100.0));
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

/// Hand the APK to the system's installer, on a thread (the APK is copied into its session): its answer comes back
/// through `NettaiUpdater.ended`, unless the update is installed, which ends the app.
pub fn install(found: Found, tell: Sender<Told>) -> Result<(), String> {
    let path = apk()?;
    log::info!("installing nettai {}", found.version);
    *ANSWER.lock().unwrap_or_else(|e| e.into_inner()) = Some(tell.clone());
    let spawned = std::thread::Builder::new().name("nettai-updater: installing".into()).spawn(move || {
        let committed = commit(&path);
        // (The session has its own copy by now, or there is none.)
        let _ = std::fs::remove_file(&path);
        if let Err(why) = committed {
            let _ = tell.send(Told::NotInstalled { why, declined: false });
        }
    });
    spawned.map(drop).map_err(|e| format!("a thread for the update: {e}"))
}

/// `work` run to its end on a thread of its own, with a tokio runtime of its own (reqwest's).
fn on_thread(name: &str, work: impl Future<Output = ()> + Send + 'static) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
    let runtime = runtime.map_err(|e| format!("a runtime for the update: {e}"))?;
    let spawned = std::thread::Builder::new().name(name.into()).spawn(move || runtime.block_on(work));
    spawned.map(drop).map_err(|e| format!("a thread for the update: {e}"))
}

/// GET `url`, at most `limit` bytes of it, each piece of the body handed to `take` as it comes, with how many bytes have
/// come so far; what `take` says went wrong ends it.
async fn get(url: &str, limit: u64, mut take: impl FnMut(&[u8], u64) -> Result<(), String>) -> Result<(), String> {
    use futures_util::StreamExt;
    // (Err: the app has installed it already, which serves as well.)
    let _ = rustls::crypto::ring::default_provider().install_default();
    // (GitHub's API answers no request that doesn't say what it comes from.)
    let client = reqwest::Client::builder().user_agent("nettai").connect_timeout(CONNECT_WAIT).read_timeout(READ_WAIT);
    let client = client.build().map_err(|e| format!("an HTTP client: {e}"))?;
    let said = |e: reqwest::Error| format!("{url}: {e}");
    let response = client.get(url).send().await.map_err(said)?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{url}: {status}"));
    }
    if response.content_length().is_some_and(|n| n > limit) {
        return Err(format!("{url}: over {limit} bytes"));
    }
    let mut got = 0;
    let mut pieces = response.bytes_stream();
    while let Some(piece) = pieces.next().await {
        let piece = piece.map_err(said)?;
        got += piece.len() as u64;
        if got > limit {
            return Err(format!("{url}: over {limit} bytes"));
        }
        take(&piece, got)?;
    }
    Ok(())
}

/// `NettaiUpdater.version`: the installed app's version, as Android says it.
fn installed() -> Result<String, String> {
    java(|env, activity| {
        let class = helper(env, activity)?;
        let version = env
            .call_static_method(&class, jni_str!("version"), jni_sig!("(Landroid/app/Activity;)Ljava/lang/String;"), &[JValue::Object(activity)])?
            .l()?;
        let version = env.cast_local::<JString>(version)?;
        version.try_to_string(env)
    })
}

/// `NettaiUpdater.install`: the session made, the APK written into it, and committed; or why not.
fn commit(apk: &Path) -> Result<(), String> {
    let refused = java(|env, activity| {
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

/// `call` run with Java's environment (this thread attached to the VM) and the activity: ndk-context's, which the
/// app sets as it starts (its `android_main`). What Java threw, or what went wrong, is the error, said.
fn java<T>(call: impl FnOnce(&mut Env, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
    let context = ndk_context::android_context();
    // SAFETY: the process's VM, which android-activity hands ndk-context as the app starts.
    let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: the activity, which GameActivity holds as a global reference for the app's life.
        let activity = unsafe { JObject::from_raw(env, context.context().cast()) };
        call(env, &activity)
    })
    .map_err(|e: jni::errors::Error| e.to_string())
}

/// `NettaiUpdater`, by the activity's class loader (a native thread's `FindClass` sees only the system's classes),
/// its native method registered (the library is the activity's, loaded natively, which Java's lookup by name
/// doesn't search).
fn helper<'local>(env: &mut Env<'local>, activity: &JObject) -> jni::errors::Result<JClass<'local>> {
    let loader = env.call_method(activity, jni_str!("getClassLoader"), jni_sig!("()Ljava/lang/ClassLoader;"), &[])?.l()?;
    let name = env.new_string("nettai.updater.NettaiUpdater")?;
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

/// `NettaiUpdater.ended`: the installer's answer, where the update wasn't installed (`PackageInstaller`'s status and
/// its message).
extern "system" fn ended<'local>(mut env: EnvUnowned<'local>, _class: JClass<'local>, status: jint, message: JString<'local>) {
    let message = env.with_env(|env| message.try_to_string(env)).resolve::<jni::errors::LogErrorAndDefault>();
    if let Some(tell) = ANSWER.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let why = format!("{message} (the installer's status {status})");
        let _ = tell.send(Told::NotInstalled { why, declined: status == ABORTED });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GitHub's JSON for a release of `tag` with these assets (name and digest), as much of it as is read.
    fn release(tag: &str, assets: &[(&str, Option<&str>)]) -> Vec<u8> {
        let assets: Vec<_> = assets
            .iter()
            .map(|(name, digest)| {
                serde_json::json!({
                    "name": name,
                    "size": 97_122_818,
                    "digest": digest,
                    "browser_download_url": format!("https://github.com/nettaisen/nettai/releases/download/{tag}/{name}"),
                })
            })
            .collect();
        serde_json::json!({ "tag_name": tag, "draft": false, "prerelease": false, "assets": assets }).to_string().into_bytes()
    }

    const SUM: &str = "5b06cc304ef839b194a770ccaa01ae4cbc49ec50c9883a8276d498ef6bc8fc1e";

    fn with_apk(tag: &str) -> Vec<u8> {
        let digest = format!("sha256:{}", SUM.to_ascii_uppercase());
        release(tag, &[("nettai-ios.ipa", Some("sha256:00")), (APK, Some(&digest)), ("altstore.json", None)])
    }

    #[test]
    fn a_newer_release_gives_its_apk() {
        let found = newer(&with_apk("v0.21.0"), "0.20.0").unwrap();
        assert_eq!(
            found,
            Some(Found {
                version: "0.21.0".into(),
                url: "https://github.com/nettaisen/nettai/releases/download/v0.21.0/nettai-android.apk".into(),
                size: 97_122_818,
                sha256: SUM.into(),
            })
        );
    }

    #[test]
    fn versions_compare_by_their_numbers() {
        assert!(newer(&with_apk("v0.10.0"), "0.9.3").unwrap().is_some());
        assert!(newer(&with_apk("v1.0.0"), "0.20.4").unwrap().is_some());
        assert!(newer(&with_apk("v0.20.1"), "0.20.0").unwrap().is_some());
        assert_eq!(newer(&with_apk("v0.9.3"), "0.10.0").unwrap(), None);
    }

    #[test]
    fn the_same_or_an_older_release_is_no_update() {
        assert_eq!(newer(&with_apk("v0.20.0"), "0.20.0").unwrap(), None);
        assert_eq!(newer(&with_apk("v0.19.3"), "0.20.0").unwrap(), None);
    }

    #[test]
    fn a_newer_release_without_its_apk_or_its_sum_is_an_error() {
        assert!(newer(&release("v0.21.0", &[("nettai-ios.ipa", Some("sha256:00"))]), "0.20.0").is_err());
        assert!(newer(&release("v0.21.0", &[(APK, None)]), "0.20.0").is_err());
    }

    #[test]
    fn a_version_that_isnt_three_numbers_doesnt_compare() {
        assert!(newer(&with_apk("v0.21"), "0.20.0").is_err());
        assert!(newer(&with_apk("v0.21.0-rc1"), "0.20.0").is_err());
    }
}
