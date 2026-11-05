//! The desktop's folders: Windows' `%APPDATA%`, macOS's `~/Library/Application Support`, Linux's `$XDG_DATA_HOME`
//! or `~/.local/share`; the system's temporary folder.

use std::path::PathBuf;

pub fn app_data() -> Option<PathBuf> {
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

pub fn temporary() -> Option<PathBuf> {
    Some(std::env::temp_dir())
}
