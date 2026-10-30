//! The WebRTC connection under [`crate::peer`], one API on both targets:
//! natively nettai-rtc's own (`platform::native::rtc`, on str0m), on the web
//! the browser's (`platform::web::rtc`, its `RTCPeerConnection`). A connection is told
//! things by calls and says what happened through [`PeerConnection::poll`]'s
//! [`Event`]s, which its caller takes in turn: no callbacks.

use std::fmt;

pub(crate) use crate::platform::system::rtc::{Options, PeerConnection, reaches};

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
    /// What only the system's connection takes ([`Options`]: natively the
    /// address bound, the ICE credentials, the fingerprint check and how
    /// long ICE checks; nothing in a browser).
    pub(crate) system: Options,
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
