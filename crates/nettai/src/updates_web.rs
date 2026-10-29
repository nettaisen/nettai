//! The browser's stand-in for the app's updates (`updates.rs`): the page is the app's, and is new when it is
//! loaded again, so no update is ever found.

/// What the loading screen waits on (never anything, here).
pub enum Holding {
    Checking,
    Downloading { version: String, progress: f32 },
}

#[derive(Default)]
pub struct Updates;

pub fn startup() {}

impl Updates {
    pub fn check(&mut self) {}

    pub fn holding(&mut self) -> Option<Holding> {
        None
    }
}
