//! Files and folders the player chooses: to open, where to save one, and files dropped on the page; and text copied
//! to the clipboard ([`clipboard`]). On the desktop the system's dialogs (rfd); in the browser its file input, its
//! folder dialog (`showDirectoryPicker`) and downloads (a file made as it is written, in the page's own storage,
//! then handed over as one: [`Stream`]). A browser's files have no path the app can read, so those chosen are read
//! into the app's files (nettai-files, the page's tree), each lot in a folder of its own, and the app is given their
//! paths there. A phone's are its pickers and its share sheet. Each system's are its own module (`platform`).
//!
//! A dialog answers through `done`: on the desktop before the call returns (the app's frames wait while it is
//! up), in the browser later, on the page's event loop. What goes wrong is said in `log`'s warning.

use platform::system;
use std::path::{Path, PathBuf};

pub mod clipboard;
mod platform;
mod stream;
pub use stream::Stream;

/// What a dialog asks for: its kind in the language shown ("Save files") and the extensions they may have.
#[derive(Clone, Copy, Debug)]
pub struct Filter<'a> {
    pub kind: &'a str,
    pub extensions: &'a [&'a str],
}

/// Whether a folder the player chose can be read again later, at another start, by its path: the desktop's can; a
/// browser's and a phone's can't (the system hands the app its files once), so the app keeps copies of what it
/// needs of one ([`open_folder`]'s files are read in).
pub const FOLDERS_READ_AGAIN: bool = system::FOLDERS_READ_AGAIN;

/// Files to open, one or more: `done` hears those chosen (none if none were).
pub fn open_files(filter: Filter, done: impl FnOnce(Vec<PathBuf>) + 'static) {
    system::open_files(filter, Box::new(done));
}

/// A folder: `done` hears it (none if none was chosen). On the desktop it is the folder chosen. In the browser a
/// folder has no path: of the one chosen, the files at `wanted` (paths within it, `*` and `?` standing for any
/// letters and one letter of a file's name: `exe/data/exe?*.dat`) are read into the app's files, in a folder of
/// their own at the same paths, and `done` hears that folder.
pub fn open_folder(wanted: &[&str], done: impl FnOnce(Option<PathBuf>) + 'static) {
    system::open_folder(wanted, Box::new(done));
}

/// Where to save a file named `name` (`near`: a path to start the dialog at): none if the player chose none. In the
/// browser no dialog asks: it is a path in the app's files, and the file is downloaded once it is written there
/// ([`saved`]).
pub fn save_path(filter: Filter, name: &str, near: Option<&Path>) -> Option<PathBuf> {
    system::save_path(filter, name, near)
}

/// A file to write as it is made (a video, as it is encoded): `done` hears it, or none if the player chose none.
/// On the desktop the system's save dialog and the file; in the browser no dialog: the file is made in the page's
/// own storage, written as the bytes come, so nothing of it is kept in memory, and handed over as a download named
/// `name` once whole (the browser's downloads say where it went).
pub fn save_stream(filter: Filter, name: &str, done: impl FnOnce(Option<Stream>) + 'static) {
    platform::stream::save_stream(filter, name, Box::new(done));
}

/// A file to write as it is made at `path`, the app's choice of place: on the desktop it is written there, no dialog
/// asked; in the browser, which writes nowhere unasked, it is made as [`save_stream`] makes one and handed over as
/// a download named as `path`'s file. `done` hears it: an error if it couldn't be made.
pub fn save_stream_to(filter: Filter, path: &Path, done: impl FnOnce(std::io::Result<Option<Stream>>) + 'static) {
    let _ = filter;
    platform::stream::save_stream_to(path, Box::new(done));
}

/// The file [`save_path`] gave is written. In the browser it is handed over as a download and let go of here; on
/// the desktop it is where it was asked to be, and nothing is done.
pub fn saved(path: &Path) {
    system::saved(path);
}

/// A file of the app's shown to the player: on the desktop in the system's file manager; in the browser, whose
/// files the player can't see, handed over as a download (a copy: the file stays).
pub fn show_file(path: &Path) {
    system::show_file(path);
}

/// The dialogs' answers that came since, each heard by its `done`: a phone's come on the system's threads, and wait
/// for this, which the app calls each frame, on its own thread. (On the desktop and in a browser, nothing waits.)
pub fn poll() {
    system::poll();
}

/// Files and folders dropped on the page go to `dropped`, from now on: a path for each. A file is read into the
/// app's files whole; of a folder, only the files at `wanted` (paths within it, as [`open_folder`] takes them), and
/// its path is a folder holding those (an empty one if none is there). (On the desktop the window's own events
/// carry them, winit's `DroppedFile`, the folder's own path, and this does nothing.)
pub fn take_drops(wanted: &[&str], dropped: impl FnMut(Vec<PathBuf>) + 'static) {
    system::take_drops(wanted, Box::new(dropped));
}
