//! A browser's folders: none. The page keeps the app's files itself, under the folder the app names (its `/data`).
//! (std's `temp_dir` panics in a browser: there is no system's temporary folder to give.)

use std::path::PathBuf;

pub fn app_data() -> Option<PathBuf> {
    None
}

pub fn temporary() -> Option<PathBuf> {
    None
}
