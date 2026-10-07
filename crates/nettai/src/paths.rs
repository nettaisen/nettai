//! Where the app keeps what the player makes: their builds, the replays it
//! records, the match files it lists and their settings, each in a folder
//! of the app's data folder unless an environment variable names another.
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

/// The builds, a folder per game: `$NETTAI_BUILDS`, else `builds`.
pub fn builds() -> PathBuf {
    folder("NETTAI_BUILDS", "builds")
}

/// The replays recorded and listed: `$NETTAI_REPLAYS`, else `replays`.
pub fn replays() -> PathBuf {
    folder("NETTAI_REPLAYS", "replays")
}

/// The match files Play lists: `$NETTAI_MATCHES`, else `matches`.
pub fn matches() -> PathBuf {
    folder("NETTAI_MATCHES", "matches")
}

/// The settings' file: `$NETTAI_SETTINGS`, else `settings.toml`.
pub fn settings() -> PathBuf {
    std::env::var_os("NETTAI_SETTINGS").map(PathBuf::from).unwrap_or_else(|| data().join("settings.toml"))
}
