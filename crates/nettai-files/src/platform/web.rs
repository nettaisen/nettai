//! In a browser: the files are a tree in memory every thread shares ([`memory`]), which the page fills
//! before the app starts and writes back to its own storage (the origin private file system); stowed files are read
//! from that storage when wanted (`storage`). The page has no folders of the system's: the app's data is the page's `/data`.

pub(crate) mod memory;
// (Only in the browser: it waits on the page's storage from threads the browser starts. The tree is tested on the
// desktop too.)
#[cfg(web)]
mod storage;

#[cfg(web)]
pub use storage::{fetch, listed, stow, stowed};

pub mod fs {
    pub use super::memory::*;
}

pub fn app_data() -> Option<std::path::PathBuf> {
    None
}

pub fn temporary() -> Option<std::path::PathBuf> {
    Some(std::env::temp_dir())
}

