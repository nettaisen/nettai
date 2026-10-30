//! The native build's (the desktop's and a phone's): the WebRTC connections
//! themselves, on str0m (`rtc`, behind `crate::rtc`'s API as the browser's
//! is in the web build, so that `crate::peer` is one module on both), direct
//! connect (`direct`), on what `rtc` can do and a browser can't, the
//! signaling server's WebSocket, on a thread of its own (`ws`), and the
//! system's randomness.

use crate::Error;

pub(crate) mod direct;
pub(crate) mod rtc;
pub(crate) mod ws;

/// `bytes` filled from the system's cryptographic randomness.
pub(crate) fn secure_random(bytes: &mut [u8]) -> Result<(), Error> {
    getrandom::fill(bytes).map_err(|e| Error::internal(format!("no randomness from the system: {e}")))
}

/// A weaker random number, when the system gives none: the clock's,
/// hashed.
pub(crate) fn clock_random_u64() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    // (The standard library's hasher keys come from the system's
    // randomness, once a process, then counted on.)
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
    h.finish()
}

// A connection goes to another thread with the link it is in.
const _: () = {
    const fn send<T: Send>() {}
    send::<crate::PeerConnection>();
};
