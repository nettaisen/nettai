//! The system's files: `std::fs`'s natively, the browser's tree in memory ([`web`]), chosen here.

// (In tests anywhere too: the tree's tests run on the desktop.)
#[cfg(any(web, test))]
#[cfg_attr(not(web), allow(dead_code))]
mod web;

#[cfg(not(web))]
pub use std::fs::*;
#[cfg(web)]
pub use web::*;
