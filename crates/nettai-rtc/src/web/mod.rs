//! What only the web build has: the signaling server's WebSocket, the
//! browser's (`ws`). The connection itself is `crate::peer`'s, the same on
//! both targets (the browser's `RTCPeerConnection`, through
//! datachannel-facade). Direct connect is native only: a browser can't
//! listen on a port, nor dial one without a description from the other side.

pub(crate) mod ws;
