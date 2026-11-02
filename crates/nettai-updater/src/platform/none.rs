//! No updater (a browser, iOS): a browser's page is new each time it is loaded, and iOS's app is its sideloading
//! tool's (AltStore's source: packaging/ios). Nothing is asked, and no update is ever found.

use crate::{Releases, Told};
use std::sync::mpsc::Sender;

/// An update, which there never is.
pub enum Found {}

impl Found {
    pub fn version(&self) -> &str {
        match *self {}
    }
}

pub fn startup() {}

/// Nothing to ask.
pub fn check(_releases: &Releases, _tell: Sender<Told>) -> bool {
    false
}

pub fn download(_releases: &Releases, found: &Found, _tell: Sender<Told>) {
    match *found {}
}

pub fn install(_releases: &Releases, found: Found, _tell: Sender<Told>) -> Result<(), String> {
    match found {}
}
