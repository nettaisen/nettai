//! The system's side of the signaling client, one module chosen here as
//! `system`: natively ([`native`]: the desktop's and a phone's) a WebSocket
//! on a thread of its own (tungstenite); in a browser ([`web`]) the
//! browser's. Each has the same `Socket` and `WsEvent`: one socket is one
//! connection, polled (`crate::room` makes another when it closes).

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod native;
#[cfg(target_arch = "wasm32")]
pub(crate) mod web;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native as system;
#[cfg(target_arch = "wasm32")]
pub(crate) use web as system;
