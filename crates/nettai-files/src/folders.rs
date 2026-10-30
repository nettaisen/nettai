//! The system's folders for the app: where apps keep their data, and where files go for a while.

use std::path::PathBuf;

/// The folder the system gives apps for their data, the app's own folder to be made in it: Windows' `%APPDATA%`,
/// macOS's `~/Library/Application Support`, Linux's `$XDG_DATA_HOME` or `~/.local/share`; on iOS the app's own
/// `Library/Application Support` (in its container, which the player doesn't see); on Android the app's files
/// folder (`Context.getFilesDir`). None in a browser (the page keeps the app's files itself), and where the system
/// says none (no `HOME`; Android's activity not handed over yet, which is logged).
pub fn app_data() -> Option<PathBuf> {
    #[cfg(web)]
    return None;
    #[cfg(target_os = "android")]
    return nettai_android::folder("getFilesDir").map_err(|why| log::warn!("finding the app's files folder: {why}")).ok();
    #[cfg(not(any(web, target_os = "android")))]
    {
        let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
        if cfg!(target_os = "windows") {
            return var("APPDATA");
        }
        let home = var("HOME");
        if cfg!(any(target_os = "macos", target_os = "ios")) {
            return home.map(|h| h.join("Library/Application Support"));
        }
        var("XDG_DATA_HOME").or_else(|| home.map(|h| h.join(".local/share")))
    }
}

/// A folder for files kept a while (those the player picked, read in; one made to hand over): on Android the app's
/// cache folder (`Context.getCacheDir`), which the system may empty while the app isn't running; elsewhere the
/// system's temporary folder. None where Android's activity isn't handed over yet (logged).
pub fn temporary() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    return nettai_android::folder("getCacheDir").map_err(|why| log::warn!("finding the app's cache folder: {why}")).ok();
    #[cfg(not(target_os = "android"))]
    Some(std::env::temp_dir())
}
