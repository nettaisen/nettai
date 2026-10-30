//! The desktop's: the system's dialogs (rfd), its clipboard (arboard), and its file manager. A dialog answers before
//! the call returns (the app's frames wait while it is up).

use crate::Filter;
use std::path::{Path, PathBuf};

/// A folder the player chose is read again by its path, at another start.
pub const FOLDERS_READ_AGAIN: bool = true;

pub fn open_files(filter: Filter, done: Box<dyn FnOnce(Vec<PathBuf>)>) {
    done(rfd::FileDialog::new().add_filter(filter.kind, filter.extensions).pick_files().unwrap_or_default());
}

/// The folder chosen, as it is.
pub fn open_folder(wanted: &[&str], done: Box<dyn FnOnce(Option<PathBuf>)>) {
    let _ = wanted;
    done(rfd::FileDialog::new().pick_folder());
}

pub fn save_path(filter: Filter, name: &str, near: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().add_filter(filter.kind, filter.extensions).set_file_name(name);
    if let Some(folder) = near.and_then(Path::parent).filter(|f| nettai_files::is_dir(f)) {
        dialog = dialog.set_directory(folder);
    }
    dialog.save_file()
}

/// The file is where it was asked to be: nothing is done.
pub fn saved(path: &Path) {
    let _ = path;
}

/// In the system's file manager.
pub fn show_file(path: &Path) {
    let mut command = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg("-R").arg(path);
        c
    } else if cfg!(target_os = "windows") {
        let mut select = std::ffi::OsString::from("/select,");
        select.push(path);
        let mut c = std::process::Command::new("explorer");
        c.arg(select);
        c
    } else {
        // (No one way to point at a file: its folder.)
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path.parent().unwrap_or(path));
        c
    };
    if let Err(why) = command.spawn() {
        log::warn!("showing {}: {why}", path.display());
    }
}

/// Nothing waits: a dialog answers as it closes.
pub fn poll() {}

/// The window's own events carry what is dropped (winit's `DroppedFile`, a folder's own path).
pub fn take_drops(wanted: &[&str], dropped: Box<dyn FnMut(Vec<PathBuf>)>) {
    drop((wanted, dropped));
}

pub fn copy(text: &str) {
    if let Err(why) = arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_string())) {
        log::warn!("copying to the clipboard: {why}");
    }
}
