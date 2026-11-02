//! Where the app has no updater of its own: a browser's page is new each time it is loaded, and iOS's app is its
//! sideloading tool's (AltStore's source: packaging/ios). Nothing is asked, and no update is ever found.

/// An update, which there never is.
pub enum Found {}

impl Found {
    pub fn version(&self) -> &str {
        match *self {}
    }
}

pub fn startup() {}

/// Nothing to ask.
pub fn check() -> bool {
    false
}

pub fn download(found: &Found) {
    match *found {}
}

pub fn apply(found: Found) -> Result<(), String> {
    match found {}
}
