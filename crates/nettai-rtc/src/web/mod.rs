//! What only the web build has: the WebRTC connection, the browser's
//! (`rtc`: its `RTCPeerConnection`, behind `crate::rtc`'s API), and the
//! signaling server's WebSocket, the browser's (`ws`). Direct connect is
//! native only: a browser can't listen on a port, nor dial one without a
//! description from the other side.

pub(crate) mod rtc;
pub(crate) mod ws;
