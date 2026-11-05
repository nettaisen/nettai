//! The system's folders: each system's module, chosen here.

#[cfg(desktop)]
mod desktop;
#[cfg(ios)]
mod ios;
#[cfg(android)]
mod android;
#[cfg(web)]
mod web;

#[cfg(desktop)]
pub use desktop::*;
#[cfg(ios)]
pub use ios::*;
#[cfg(android)]
pub use android::*;
#[cfg(web)]
pub use web::*;
