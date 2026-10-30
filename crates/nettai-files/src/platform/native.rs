//! The desktop and a phone: the system's files, `std::fs`'s; the stowed files are files like any other.

use std::io;
use std::path::{Path, PathBuf};

pub mod fs {
    pub use std::fs::*;
}

pub fn stow(path: &Path, bytes: Vec<u8>) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, bytes)
}

pub fn stowed(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<PathBuf> = read.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
    out.sort();
    out
}

pub fn fetch(paths: &[PathBuf]) -> io::Result<()> {
    let _ = paths;
    Ok(())
}
