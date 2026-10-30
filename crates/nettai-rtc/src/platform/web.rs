//! The web build's: the WebRTC connection, the browser's (`rtc`: its
//! `RTCPeerConnection`, behind `crate::rtc`'s API), the signaling server's
//! WebSocket, the browser's (`ws`), and the browser's randomness. Direct
//! connect is native only (`direct`: nothing): a browser can't listen on a
//! port, nor dial one without a description from the other side.

use crate::Error;

pub(crate) mod direct;
pub(crate) mod rtc;
pub(crate) mod ws;

/// `bytes` filled from the browser's `crypto.getRandomValues`.
pub(crate) fn secure_random(bytes: &mut [u8]) -> Result<(), Error> {
    use wasm_bindgen::JsCast;
    let crypto = js_sys::Reflect::get(&js_sys::global(), &"crypto".into())
        .ok()
        .and_then(|c| c.dyn_into::<web_sys::Crypto>().ok())
        .ok_or_else(|| Error::internal("the browser has no crypto"))?;
    crypto.get_random_values_with_u8_array(bytes).map_err(|e| Error::internal(format!("no randomness from the browser: {e:?}")))?;
    Ok(())
}

/// A weaker random number, when the browser gives none: `Math.random`'s.
pub(crate) fn clock_random_u64() -> u64 {
    let half = || (js_sys::Math::random() * 4294967296.0) as u64;
    half() << 32 | half()
}
