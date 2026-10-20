//! The browser's stand-in for the app's updates (`updates.rs`): the page is the app's, and is new when it is
//! loaded again, so no update is ever found.

use crate::shell::Shell;

/// The tag of the question asked when an update is found (never, here).
pub const ASK: &str = "update";

#[derive(Default)]
pub struct Updates;

pub fn startup() {}

impl Updates {
    pub fn check(&mut self) {}
}

impl Shell {
    pub fn at_menu(&mut self) {}
    pub fn download_update(&mut self) {}
}
