//! Netplay's transport: WebRTC data channels between two players
//! (docs/design/rollback.md §4.7, §4.8), one API on two backends: native,
//! on the `rtc` crate (sans-I/O WebRTC, the sockets and the timers this
//! crate's), and the browser's `RTCPeerConnection` on wasm32.
//!
//! What a game sends is a datagram: the netplay protocol's frames (and the
//! lobby's messages before them), which tolerate loss, reordering and
//! duplicates. So the one channel is opened unordered and without
//! retransmits, negotiated out of band (id 0 on both sides, no in-band
//! announcement): a lost datagram is the next one's to make up, never
//! SCTP's to resend late. The lobby says its part again every 100 ms until
//! it is answered, so it needs no reliable channel of its own.
//!
//! Two layers:
//!
//! - [`PeerConnection`]: one WebRTC connection and its data channel, told
//!   the other peer's description and candidates and handing over its own
//!   ([`PeerEvent`]); event-driven, so that the browser's asynchronous
//!   offer and answer fit the same API. Nothing waits.
//! - [`Link`]: the channel a match is played on, which keeps a connection
//!   up: it meets the other player (in a room of the signaling server,
//!   `signal`; or directly, at a host's address and port, natively), and
//!   when the connection drops (its state, its data channel, or silence) it
//!   makes a new one through the same room or to the same address, until
//!   [`Config::reconnect_timeout`]. Its `send` and `recv` are a datagram
//!   channel's, which nettai-frontend's `netplay::Channel` takes; while it
//!   is down, what is sent is dropped and nothing comes ([`Link::down_for`]
//!   says so, and for how long).
//!
//! The signaling server is the Cloudflare Worker in `signaling/`
//! (TypeScript, a Durable Object a room); its protocol is [`signal`], and
//! the `testing` feature has a server of its own speaking it, in process.

pub mod signal;

mod link;
mod room;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(all(feature = "testing", not(target_arch = "wasm32")))]
pub mod testing;

pub use link::Link;
#[cfg(not(target_arch = "wasm32"))]
pub use native::PeerConnection;
#[cfg(target_arch = "wasm32")]
pub use web::PeerConnection;
pub use web_time::{Duration, Instant};

use std::fmt;

/// A public STUN server, which a connection asks for its address as the
/// Internet sees it (its server-reflexive candidate) unless told another.
pub const DEFAULT_STUN: &str = "stun:stun.l.google.com:19302";

/// How connections are made and kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// The STUN and TURN servers (as the browser's `RTCIceServer`). TURN
    /// relays when no direct path is found; it needs credentials (a
    /// public one such as Cloudflare's hands out short-lived ones from its
    /// own API).
    pub ice_servers: Vec<IceServer>,
    /// Nothing from the other peer this long, while the channel is open, is
    /// a dropped connection (both sides send many times a second: the
    /// lobby every 100 ms, a match every frame).
    pub silence: Duration,
    /// A connection being made that isn't open this long is given up and
    /// made again.
    pub dial_timeout: Duration,
    /// A dropped connection not back this long ends the link.
    pub reconnect_timeout: Duration,
    /// Offer the loopback interface's address too (two peers on one
    /// machine; it is offered anyway where there is no other).
    pub loopback: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            ice_servers: vec![IceServer::new(DEFAULT_STUN)],
            silence: Duration::from_secs(3),
            dial_timeout: Duration::from_secs(10),
            reconnect_timeout: Duration::from_secs(30),
            loopback: false,
        }
    }
}

/// A STUN or TURN server: `stun:HOST[:PORT]` or
/// `turn:HOST[:PORT][?transport=udp]` (the native backend relays over UDP;
/// the browser takes what it takes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
}

impl IceServer {
    /// A server without credentials (STUN).
    pub fn new(url: &str) -> IceServer {
        IceServer { urls: vec![url.to_string()], ..IceServer::default() }
    }
}

/// Which side a peer is: the host answers, the joiner dials (offers). In a
/// room the first to come hosts; directly, the one that listens hosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Host,
    Join,
}

/// What went wrong, said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl Error {
    pub(crate) fn new(what: impl fmt::Display) -> Error {
        Error(what.to_string())
    }
}

/// An ICE candidate as the browser's `RTCIceCandidateInit` has it: the
/// candidate line, and the media section it is of.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Candidate {
    pub candidate: String,
    #[serde(default)]
    pub mid: Option<String>,
    #[serde(default)]
    pub mline: Option<u16>,
}

/// What a [`PeerConnection`] has for its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeerEvent {
    /// The local description (the offer, or the answer), for the other
    /// peer.
    Description(String),
    /// A local candidate, for the other peer (trickled as each is found).
    Candidate(Candidate),
    /// The data channel is open: what is sent now goes.
    Open,
    /// The connection is lost, failed or closed, and why: it won't come
    /// back (a new one does).
    Down(String),
}

/// A peer's random id, and a direct joiner's nonce.
pub(crate) fn random_u64() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::hash::{BuildHasher, Hasher};
        // (The standard library's hasher keys come from the system's
        // randomness, once a process, then counted on.)
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        h.finish()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let half = || (js_sys::Math::random() * 4294967296.0) as u64;
        half() << 32 | half()
    }
}
