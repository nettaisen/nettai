//! Files, the same calls everywhere: `fs::read`, `fs::write`, `fs::File` and the rest, named as `std::fs` names
//! them.
//!
//! On the desktop they are `std::fs`'s. In the browser there is no file system a program can call without waiting,
//! so there they are a tree in memory ([`memory`]), one that every thread shares: the page puts the files it keeps in
//! it before the app starts ([`preload`]), and takes what changed under the folder it keeps ([`watch`],
//! [`changes`]) to write it to its own storage (crates/nettai-web: the origin private file system).
//!
//! `Path::exists`, `is_file` and `is_dir` ask the system's file system: [`exists`], [`is_file`] and [`is_dir`] are
//! this crate's.

use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
pub use std::fs::*;

#[cfg(any(target_arch = "wasm32", test))]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) mod memory;
#[cfg(target_arch = "wasm32")]
pub use memory::*;

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
    use super::memory as m;
    use std::io::{Read, Seek, SeekFrom, Write};

    /// The browser's tree, as `std::fs` behaves: folders made and listed, files written, read, appended to and
    /// moved, and what changed under the folder kept, in order. (One test: the tree is the process's.)
    #[test]
    fn the_browser_s_tree_behaves_as_the_system_s_files() {
        m::preload("/kept/old.txt", b"old".to_vec());
        m::watch("/kept");
        assert!(m::write("/kept/a/b.txt", "x").is_err(), "no folder");
        m::create_dir_all("kept/a/c").unwrap();
        m::write("/kept/a/b.txt", "hello").unwrap();
        m::write("/elsewhere.txt", "not kept").unwrap();
        assert_eq!(m::read_to_string("/kept/./a/../a/b.txt").unwrap(), "hello");
        let mut names: Vec<_> = m::read_dir("/kept/a").unwrap().map(|e| e.unwrap().file_name()).collect();
        names.sort();
        assert_eq!(names, ["b.txt", "c"]);
        {
            let mut log = m::OpenOptions::new().create(true).append(true).open("/kept/log").unwrap();
            log.write_all(b"one ").unwrap();
        }
        let mut log = m::OpenOptions::new().create(true).append(true).open("/kept/log").unwrap();
        log.write_all(b"two").unwrap();
        drop(log);
        assert_eq!(m::read("/kept/log").unwrap(), b"one two");
        let mut f = m::File::open("/kept/a/b.txt").unwrap();
        f.seek(SeekFrom::Start(1)).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        assert_eq!(s, "ello");
        assert!(f.write(b"no").is_err(), "opened to read");
        m::rename("/kept/a", "/kept/z").unwrap();
        assert!(!m::metadata("/kept/a/b.txt").is_ok() && m::metadata("/kept/z/b.txt").unwrap().is_file());
        assert!(m::metadata("/kept/z/c").unwrap().is_dir());
        assert!(m::remove_dir("/kept/z").is_err(), "not empty");
        m::remove_dir_all("/kept/z").unwrap();
        assert!(m::read_dir("/kept/z").is_err());
        let changes: Vec<String> = m::changes()
            .into_iter()
            .map(|c| match c {
                m::Change::Write(p, d) => format!("write {} {}", p.display(), String::from_utf8_lossy(&d)),
                m::Change::Remove(p) => format!("remove {}", p.display()),
            })
            .collect();
        assert_eq!(
            changes,
            [
                "write /kept/a/b.txt hello",
                "write /kept/log ",
                "write /kept/log one ",
                "write /kept/log one two",
                "write /kept/z/b.txt hello",
                "remove /kept/a",
                "remove /kept/z",
            ]
        );
        assert!(m::changes().is_empty());
        assert_eq!(m::read("/kept/old.txt").unwrap(), b"old");
    }

    #[test]
    fn the_desktop_has_the_system_s_files() {
        use super::*;
        let dir = std::env::temp_dir().join(format!("nettai-platform-fs-{}", std::process::id()));
        create_dir_all(dir.join("a")).unwrap();
        write(dir.join("a/b.txt"), "b").unwrap();
        assert!(is_dir(dir.join("a")) && is_file(dir.join("a/b.txt")) && !exists(dir.join("c")));
        assert_eq!(read_to_string(dir.join("a/b.txt")).unwrap(), "b");
        remove_dir_all(&dir).unwrap();
    }
}
