//! iOS's folders: the app's own `Library/Application Support`, in its container (`HOME` is the container's); the
//! system's temporary folder (the container's `tmp`).

use std::path::PathBuf;

pub fn app_data() -> Option<PathBuf> {
    std::env::var_os("HOME").filter(|v| !v.is_empty()).map(|h| PathBuf::from(h).join("Library/Application Support"))
}

pub fn temporary() -> Option<PathBuf> {
    Some(std::env::temp_dir())
}
