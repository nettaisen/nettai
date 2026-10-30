//! The system's files and folders, each chosen here, once. The files: `std::fs` natively (the desktop and both
//! phones share [`native`]), the browser's tree in memory ([`web`], on its `memory`). The app's folders: each
//! system's own ([`desktop`], [`ios`], [`android`], [`web`]). What only one has is public from here as the crate's
//! own (the browser's `listed`).

#[cfg(not(web))]
pub(crate) mod native;
// (In tests anywhere too: the browser's tree's tests run on the desktop.)
#[cfg(any(web, test))]
#[cfg_attr(not(web), allow(dead_code, unused_imports))]
pub(crate) mod web;
#[cfg(desktop)]
pub(crate) mod desktop;
#[cfg(ios)]
pub(crate) mod ios;
#[cfg(android)]
pub(crate) mod android;

#[cfg(not(web))]
pub(crate) use native as files;
#[cfg(web)]
pub(crate) use web as files;

#[cfg(desktop)]
pub(crate) use desktop as folders;
#[cfg(ios)]
pub(crate) use ios as folders;
#[cfg(android)]
pub(crate) use android as folders;
#[cfg(web)]
pub(crate) use web as folders;

#[cfg(web)]
pub use web::listed;
