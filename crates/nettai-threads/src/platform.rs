//! The system's threads, one module chosen here: std's natively (the desktop, a phone), Web Workers in a browser.
//! Each has `lock` and `spawn`; what only one has is public from here as the crate's own (the browser's
//! `spawn_async`).

#[cfg(not(web))]
pub(crate) mod native;
#[cfg(web)]
pub(crate) mod web;

#[cfg(not(web))]
pub(crate) use native as system;
#[cfg(web)]
pub(crate) use web as system;

#[cfg(web)]
pub use web::spawn_async;
