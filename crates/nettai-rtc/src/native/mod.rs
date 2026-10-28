//! What only the native build has: the WebRTC connections themselves, on
//! str0m (`rtc`, shaped like the browser's that web-datachannel gives the
//! web build, so that `crate::peer` is one module on both), direct connect
//! (`direct`), on what `rtc` can do and a browser can't, and the signaling
//! server's WebSocket, on a thread of its own (`ws`).

pub(crate) mod direct;
pub(crate) mod rtc;
pub(crate) mod ws;
