//! The stowed files: files like any other natively ([`native`]), the page's storage in a browser ([`web`]), chosen
//! here.

#[cfg(not(web))]
mod native;
#[cfg(web)]
mod web;

#[cfg(not(web))]
pub use native::*;
#[cfg(web)]
pub use web::*;
