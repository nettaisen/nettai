//! The WebRTC connection under [`crate::peer`], one API on both targets:
//! natively nettai-rtc's own (`native::rtc`, on str0m), on the web the
//! browser's (`web::rtc`, its `RTCPeerConnection`). A connection is told
//! things by calls and says what happened through [`PeerConnection::poll`]'s
//! [`Event`]s, which its caller takes in turn: no callbacks.

use std::fmt;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use crate::native::rtc::PeerConnection;
#[cfg(target_arch = "wasm32")]
pub(crate) use crate::web::rtc::PeerConnection;

use crate::IceServer;

/// One of a connection's data channels, by the order it was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChannelId(pub(crate) usize);

/// What a connection has to say, in turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    /// This side's description, made ([`PeerConnection::set_local_description`]):
    /// before its candidates.
    Description(String),
    /// One of this side's candidates (`candidate:...`).
    Candidate(String),
    State(State),
    Open(ChannelId),
    Closed(ChannelId),
    /// A message that came on a channel.
    Message(ChannelId, Vec<u8>),
}

/// The connection's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    /// Checking candidate pairs.
    Connecting,
    /// ICE and DTLS through.
    Connected,
    /// The pair in use stopped answering (it may come back).
    Disconnected,
    /// The connection broke.
    Failed,
    Closed,
}

/// A description's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SdpType {
    Offer,
    Answer,
}

/// How a connection is made.
#[derive(Clone, Debug, Default)]
pub(crate) struct Configuration {
    /// The STUN and TURN servers; the backend uses the URLs it can.
    pub(crate) ice_servers: Vec<IceServer>,
    /// Offer the relayed candidates alone.
    pub(crate) relay_only: bool,
    /// Bind this address and port (port 0: one the system picks) instead of
    /// every interface's, both families, on ports the system picks.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) bind: Option<std::net::SocketAddr>,
    /// This side's ICE username fragment and password, instead of random
    /// ones.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) ice_credentials: Option<(String, String)>,
    /// Don't check the other side's DTLS certificate against its
    /// description's fingerprint.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) skip_fingerprint_check: bool,
    /// How long ICE keeps checking a pair that doesn't answer: as long as a
    /// dial is given ([`crate::Config::dial_timeout`]), so that a dial made
    /// while the path is out (or while a direct host's port is another
    /// connection's) connects as soon as it is back.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) checks_for: std::time::Duration,
}

/// A data channel, negotiated out of band as stream `stream` on both sides.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChannelOptions {
    pub(crate) stream: u16,
    pub(crate) ordered: bool,
    /// How often a message is sent again before it is given up (`None`:
    /// until it gets through).
    pub(crate) max_retransmits: Option<u16>,
}

#[derive(Clone, Debug)]
pub(crate) struct Error(pub(crate) String);

impl Error {
    pub(crate) fn new(what: impl Into<String>) -> Error {
        Error(what.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
