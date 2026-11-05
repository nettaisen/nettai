//! Files, the same calls on the desktop, in a browser and on a phone: `nettai_files::read`, `write`, `File` and the
//! rest, named as `std::fs` names them; the system's folders for the app ([`folders`]); and large files kept but read
//! in only when wanted ([`stow`], [`stowed`], [`fetch`]). Each system's are in `platform`, by concern.
//!
//! On the desktop and a phone the files are `std::fs`'s. In the browser there is no file system a program can call
//! without waiting, so there they are a tree in memory (`platform::tree`), one that every thread shares: the page
//! puts the files it keeps in it before the app starts (`preload`), and takes what changed under the folder it keeps
//! (`watch`, `changes`) to write it to its own storage (packaging/web's page: the origin private file system).
//!
//! `Path::exists`, `is_file` and `is_dir` ask the system's file system: [`exists`], [`is_file`] and [`is_dir`] are
//! this crate's.

use std::path::Path;

pub mod folders;
mod platform;
mod stowed;

// The system's files: `std::fs` natively, the browser's tree in memory.
#[cfg(web)]
pub use platform::tree::*;
#[cfg(not(web))]
pub use std::fs::*;

#[cfg(web)]
pub use stowed::listed;
pub use stowed::{STOWED, fetch, stow, stowed};

/// Whether anything is at `path`.
pub fn exists(path: impl AsRef<Path>) -> bool {
    metadata(path).is_ok()
}

/// Whether a file is at `path`.
pub fn is_file(path: impl AsRef<Path>) -> bool {
    metadata(path).is_ok_and(|m| m.is_file())
}

/// Whether a folder is at `path`.
pub fn is_dir(path: impl AsRef<Path>) -> bool {
    metadata(path).is_ok_and(|m| m.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_has_the_system_s_files() {
        let dir = std::env::temp_dir().join(format!("nettai-files-{}", std::process::id()));
        create_dir_all(dir.join("a")).unwrap();
        write(dir.join("a/b.txt"), "b").unwrap();
        assert!(is_dir(dir.join("a")) && is_file(dir.join("a/b.txt")) && !exists(dir.join("c")));
        assert_eq!(read_to_string(dir.join("a/b.txt")).unwrap(), "b");
        remove_dir_all(&dir).unwrap();
    }
}
