//! The system's side of the dialogs, one module chosen here as `system`, each with the same calls: the desktop's
//! ([`desktop`]: rfd's dialogs, arboard's clipboard), the browser's ([`web`]), or a phone's ([`phone`]: the same on
//! iOS and Android, over the system's own pickers, share sheet and clipboard).
//! A file written as it is made (`stream`) is a file on the desktop and a phone ([`native`]), and the page's storage
//! in a browser.

#[cfg(desktop)]
pub(crate) mod desktop;
#[cfg(web)]
pub(crate) mod web;
// (In tests anywhere too, with Android's pickers: the files wanted in a folder are matched in plain Rust, whose tests
// run on the desktop; JNI builds there.)
#[cfg(any(phone, test))]
#[cfg_attr(not(phone), allow(dead_code))]
pub(crate) mod phone;
#[cfg(not(web))]
pub(crate) mod native;

#[cfg(desktop)]
pub(crate) use desktop as system;
#[cfg(web)]
pub(crate) use web as system;
#[cfg(phone)]
pub(crate) use phone as system;

#[cfg(not(web))]
pub(crate) use native as stream;
#[cfg(web)]
pub(crate) use web as stream;
