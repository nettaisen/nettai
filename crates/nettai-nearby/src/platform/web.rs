//! A browser's: no discovery (a page can neither browse multicast DNS nor listen for a connection). It neither offers
//! nor seeks, and a browse finds nothing.

use crate::Seen;
use std::sync::mpsc::Sender;

pub const OFFERS: bool = false;
pub const SEEKS: bool = false;

/// Never made: a page doesn't offer.
pub struct Advertised;

pub fn advertise(_name: &str, _port: u16) -> Result<Advertised, String> {
    Err("a browser doesn't offer".into())
}

/// A browse that finds nothing.
pub struct Browsing;

pub fn browse(_seen: Sender<Seen>) -> Result<Browsing, String> {
    Ok(Browsing)
}

pub fn device_name() -> String {
    "nettai".into()
}
