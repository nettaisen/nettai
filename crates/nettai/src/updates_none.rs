//! The stand-in for the app's updates (`updates.rs`) where the app has no updater of its own: a browser's page is new
//! each time it is loaded, and a phone's app is updated by its store, so no update is ever found here.

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
