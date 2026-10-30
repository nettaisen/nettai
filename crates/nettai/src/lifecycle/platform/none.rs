//! Windows, Linux and a browser: nothing to hold, nothing stops the app.

pub struct Held;

impl Held {
    pub fn new() -> Held {
        Held
    }
}

pub fn watch() {}
