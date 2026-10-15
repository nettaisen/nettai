//! What only the native build has: direct connect (`direct`), on what
//! libdatachannel can do and a browser can't, and the signaling server's
//! WebSocket, on a thread of its own (`ws`). The connection itself is
//! `crate::peer`'s, the same on both targets.

pub(crate) mod direct;
pub(crate) mod ws;
