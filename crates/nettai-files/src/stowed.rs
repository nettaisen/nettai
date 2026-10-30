//! Files kept in the data folder that the page doesn't put in the app's files as it starts: large ones, read only
//! when a part of the app wants them (the Legacy Collection's ROMs, which the browser keeps: nettai's `games`). They
//! are in the data folder's [`STOWED`] folder. On the desktop they are files like any other, read as any other.

use crate::platform::files;
use std::io;
use std::path::{Path, PathBuf};

/// The data folder's folder of stowed files: the page lists them as it starts and reads none (packaging/web's page).
/// Its name starts with a dot, so the page's keeping of what changes passes it over: what is in it is written by
/// [`stow`].
pub const STOWED: &str = ".stowed";

/// Keep `bytes` at `path`, a file of a data folder's [`STOWED`] folder: on the desktop written as any file; in the
/// browser put in the app's files and written to the page's storage (the origin private file system) as well, by a
/// thread of its own, and listed by [`stowed`] from now on and at each start.
pub fn stow(path: &Path, bytes: Vec<u8>) -> io::Result<()> {
    files::stow(path, bytes)
}

/// The stowed files in folder `dir`, read into the app's files or not, by path, in order.
pub fn stowed(dir: &Path) -> Vec<PathBuf> {
    files::stowed(dir)
}

/// Read the stowed files `paths` into the app's files, those that aren't: in the browser from the page's storage,
/// a thread of its own reading them and this one waiting for it, which a page's main thread may not (there, call it
/// only for files read in already). On the desktop they are files like any other, and nothing is done.
pub fn fetch(paths: &[PathBuf]) -> io::Result<()> {
    files::fetch(paths)
}
