//! Where the app keeps what the player makes: their builds, the replays it
//! records, their settings and its log, each in the app's data folder
//! unless an environment variable names another.
//!
//! The data folder is `$NETTAI_DATA`, else the system's place for an app's
//! data: `~/Library/Application Support/nettai` on macOS,
//! `$XDG_DATA_HOME/nettai` (`~/.local/share/nettai`) on Linux and the
//! other Unixes, `%APPDATA%\nettai` on Windows; the working folder where
//! none of those can be found.

use std::path::PathBuf;

/// The app's data folder.
pub fn data() -> PathBuf {
    if let Some(dir) = std::env::var_os("NETTAI_DATA") {
        return PathBuf::from(dir);
    }
    system().map(|d| d.join("nettai")).unwrap_or_default()
}

/// The system's folder for apps' data.
fn system() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(target_os = "windows") {
        return var("APPDATA");
    }
    let home = var("HOME");
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library/Application Support"));
    }
    var("XDG_DATA_HOME").or_else(|| home.map(|h| h.join(".local/share")))
}

/// A folder of the data folder, unless `var` names another.
fn folder(var: &str, name: &str) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).unwrap_or_else(|| data().join(name))
}

/// Where the builds are kept instead, once [`keep_builds_in`] says.
static BUILDS: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// The builds, a folder per game: `$NETTAI_BUILDS`, else `builds`.
pub fn builds() -> PathBuf {
    if let Some(dir) = BUILDS.lock().ok().and_then(|kept| kept.clone()) {
        return dir;
    }
    folder("NETTAI_BUILDS", "builds")
}

/// Keep the builds in `dir` from here on, whatever the environment says: the screenshot pass makes and edits
/// builds of its own, and never the player's.
pub fn keep_builds_in(dir: PathBuf) {
    if let Ok(mut kept) = BUILDS.lock() {
        *kept = Some(dir);
    }
}

/// The replays recorded and listed: `$NETTAI_REPLAYS`, else `replays`.
pub fn replays() -> PathBuf {
    folder("NETTAI_REPLAYS", "replays")
}

/// The log, which takes what goes wrong as the code says it (the player reads a plain sentence instead:
/// `crate::errors`): `$NETTAI_LOG`, else `nettai.log`.
pub fn log() -> PathBuf {
    std::env::var_os("NETTAI_LOG").map(PathBuf::from).unwrap_or_else(|| data().join("nettai.log"))
}

/// The settings' file: `$NETTAI_SETTINGS`, else `settings.toml`.
pub fn settings() -> PathBuf {
    std::env::var_os("NETTAI_SETTINGS").map(PathBuf::from).unwrap_or_else(|| data().join("settings.toml"))
}
