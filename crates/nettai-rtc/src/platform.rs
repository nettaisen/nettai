//! The system's side of the transport, one module chosen here as `system`:
//! natively ([`native`]: the desktop's and a phone's) str0m's connections on
//! sockets of this crate's, and direct connect; in a browser ([`web`]) its
//! own connection. Each has `rtc` (the connection under [`crate::rtc`]),
//! `direct` (direct connect, which a browser hasn't) and the randomness.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod native;
#[cfg(target_arch = "wasm32")]
pub(crate) mod web;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native as system;
#[cfg(target_arch = "wasm32")]
pub(crate) use web as system;
