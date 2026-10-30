//! A phone's pickers, its share sheet and its clipboard. (Placeholders until the pickers are written: each says
//! so in the log and answers as if the player chose nothing.)

use super::Filter;
use std::path::{Path, PathBuf};

pub fn open_files(filter: Filter, done: Box<dyn FnOnce(Vec<PathBuf>)>) {
    let _ = filter;
    log::warn!("opening files: a phone's picker isn't written yet");
    done(Vec::new());
}

pub fn open_folder(wanted: &[&str], done: Box<dyn FnOnce(Option<PathBuf>)>) {
    let _ = wanted;
    log::warn!("opening a folder: a phone's picker isn't written yet");
    done(None);
}

/// Where a file to hand over is made: the app's temporary folder's `outgoing`.
pub fn outgoing(name: &str) -> Option<PathBuf> {
    Some(crate::folders::temporary()?.join("outgoing").join(name))
}

pub fn hand_over(path: &Path) {
    log::warn!("handing over {}: a phone's share sheet isn't written yet", path.display());
}

pub fn share(path: &Path) {
    log::warn!("sharing {}: a phone's share sheet isn't written yet", path.display());
}

pub fn copy(text: &str) {
    let _ = text;
    log::warn!("copying: a phone's clipboard isn't written yet");
}
