//! Netplay's transport: WebRTC data channels between two players
//! (docs/design/rollback.md §4.7, §4.8), one API on both targets: natively
//! str0m (pure Rust, sans I/O: ICE, DTLS and SCTP), its sockets, timers,
//! candidates and TURN relays this crate's, on a thread of its own
//! (`native::rtc`); on wasm32 the browser's `RTCPeerConnection`
//! (web-datachannel, nettai's copy of datachannel-facade's web backend). It
//! took libdatachannel's place natively (C++, through datachannel-facade),
//! which had taken the `rtc` crate's.
//!
//! What a game sends is a datagram: the netplay protocol's frames (and the
//! lobby's messages before them), which tolerate loss, reordering and
//! duplicates. So its channel is opened unordered and without retransmits,
//! negotiated out of band (stream 0 on both sides, no in-band
//! announcement): a lost datagram is the next one's to make up, never
//! SCTP's to resend late. The lobby says its part again every 100 ms until
//! it is answered, so it needs no reliable channel of its own.
//!
//! Two layers:
//!
//! - [`PeerConnection`]: one WebRTC connection and its data channels, told
//!   the other peer's description and candidates and handing over its own
//!   ([`PeerEvent`]); event-driven, the backend's callbacks putting what
//!   they learn in a queue its calls take from, so nothing waits: the
//!   caller's frame loop polls.
//! - [`Link`]: the channel a match is played on, which keeps a connection
//!   up: it meets the other player (in an invite room of the signaling
//!   server, `signal`: one it asks the server to make, [`Link::create_room`],
//!   whose code the server picks, or one it was given the code of,
//!   [`Link::join_room`]; or directly, at a host's address and port,
//!   natively). It needs the server only until the connection is through:
//!   then both sides leave the room and close their sockets, having agreed
//!   over the connection on the code of a reconnect room, a new one each
//!   connection. When the connection drops (its state, its data channel, or
//!   silence) it makes a new one through that reconnect room or to the same
//!   address, until [`Config::reconnect_timeout`]. Its `send` and `recv` are
//!   a datagram channel's, which nettai-frontend's `netplay::Channel` takes;
//!   while it is down, what is sent is dropped and nothing comes
//!   ([`Link::down_for`] says so, and for how long; [`Link::gives_up_in`],
//!   how long it has left).
//!
//! The link's own messages (the reconnect code and its acknowledgment) go
//! on a channel of their own, stream 1, reliable and ordered: a message is
//! its bytes, with no text or binary type to tell the link's from the
//! game's, and the channel they came on tells them apart at no cost in
//! bytes, as the message type did (`peer`).
//!
//! What goes wrong is an [`Error`]: its [`ErrorKind`] is what a player can
//! be told, each kind in the host's own words (its catalog's), and its
//! detail is for a log, never for the player.
//!
//! **STUN and TURN** are the signaling server's to give. The user's words:
//! "the signaling server should return stun and turn server credentials",
//! and of a default STUN server, "you don't even need a google default
//! right? direct connect doesn't even need stun?" So a link in a room uses
//! exactly the servers the server's welcome gives (the Worker's: Cloudflare's
//! STUN server, and its TURN service's with credentials of a few hours; or
//! the STUN server alone when it has none), and has none of its own; a
//! direct link uses none, since it says no candidate to anyone.
//!
//! The signaling server is the Cloudflare Worker in `signaling/`
//! (TypeScript, a Durable Object a room); its protocol is [`signal`], and
//! the `testing` feature has a server of its own speaking it, in process,
//! and a few of the link's insides for tests (`Link::hold_codes`,
//! `Link::reconnect_code`, `Link::in_a_room`).

#![forbid(unsafe_code)]

pub mod signal;

mod link;
mod peer;
mod room;
mod rtc;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(all(feature = "testing", not(target_arch = "wasm32")))]
pub mod testing;

pub use link::Link;
pub use peer::PeerConnection;
pub use web_time::{Duration, Instant};

use std::fmt;

/// How connections are made and kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// STUN and TURN servers of the caller's own (as the browser's
    /// `RTCIceServer`), none by default: what a [`PeerConnection`] made by
    /// hand uses, and a link in a room whose server's welcome gives none. A
    /// link in a room uses exactly the servers the welcome gives (Cloudflare's
    /// STUN server, and its TURN service's with short-lived credentials:
    /// [`signal`]). A direct link uses none: it says no candidate to anyone
    /// (the joiner dials the host's address, and the host learns the
    /// joiner's from its checks), so its address as the Internet sees it
    /// would go nowhere.
    pub ice_servers: Vec<IceServer>,
    /// Nothing from the other peer this long, while the channel is open, is
    /// a dropped connection (both sides send many times a second: the
    /// lobby every 100 ms, a match every frame).
    pub silence: Duration,
    /// A connection being made that isn't open this long is given up and
    /// made again.
    pub dial_timeout: Duration,
    /// A dropped connection not back this long ends the link
    /// ([`ErrorKind::ConnectionLost`]), whether or not the signaling server
    /// could be reached meanwhile.
    pub reconnect_timeout: Duration,
    /// The signaling server not answering this long, before it first lets
    /// this player into the invite room, ends the link
    /// ([`ErrorKind::ServerUnreachable`]).
    pub server_timeout: Duration,
    /// The other player there (in the room, or dialed directly) and no
    /// connection to them open this long, before the first one, ends the
    /// link ([`ErrorKind::CantConnect`]): no path between the two was
    /// found.
    pub connect_timeout: Duration,
    /// Offer the loopback interface's address too: each host candidate's
    /// port on 127.0.0.1 (two peers on one machine, whatever its network).
    pub loopback: bool,
    /// Offer only relayed candidates (the browser's `iceTransportPolicy:
    /// "relay"`): every datagram goes through a TURN server, and nothing
    /// connects without one. (Tests: a connection relayed on one machine.)
    /// Natively nothing is checked but through the relays either.
    pub relay_only: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            ice_servers: Vec::new(),
            silence: Duration::from_secs(3),
            dial_timeout: Duration::from_secs(10),
            reconnect_timeout: signal::RECONNECT_ROOM_KEPT,
            server_timeout: Duration::from_secs(10),
            connect_timeout: Duration::from_secs(30),
            loopback: false,
            relay_only: false,
        }
    }
}

/// A STUN or TURN server as the browser's `RTCIceServer` has it (and the
/// signaling server's welcome, [`signal`]): its URLs, `stun:HOST[:PORT]`,
/// `turn:HOST[:PORT][?transport=udp|tcp]` or
/// `turns:HOST[:PORT][?transport=tcp]`, and a TURN server's credentials.
/// The browser takes every URL; natively the connection relays over UDP
/// only (`turn:` without `transport=tcp`, on up to two TURN servers), and
/// the others are left out.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub username: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
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

/// What went wrong, as a player can be told it. A host says each kind in
/// its own words (a catalog's string a kind, in the player's language); it
/// never shows an [`Error::detail`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// The signaling server can't be reached: its socket didn't open, or it
    /// didn't answer within [`Config::server_timeout`]. (While a dropped
    /// connection is made again it is only what [`Link::problem`] says: the
    /// link goes on trying.)
    ServerUnreachable,
    /// The signaling server speaks another version of its protocol than
    /// this program does: one of the two is out of date.
    ServerVersion,
    /// No room has that code: it is mistyped, or its room is over.
    NoSuchRoom,
    /// The room has two players already.
    RoomFull,
    /// A direct host's address doesn't name a machine (`host:port`).
    BadAddress,
    /// A direct host can't listen on its port (another program has it).
    PortUnavailable,
    /// The other player is there, and no connection to them could be made
    /// within [`Config::connect_timeout`]: no path between the two
    /// networks was found (or nothing answers at a direct host's address).
    CantConnect,
    /// The connection dropped and didn't come back within
    /// [`Config::reconnect_timeout`].
    ConnectionLost,
    /// Anything else: nothing the player did or can mend. The detail says
    /// what, for a log.
    Internal,
}

/// What went wrong: its kind, which is what a player is told (in the
/// host's words), and the detail, in English, for a log only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub detail: String,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, detail: impl Into<String>) -> Error {
        Error { kind, detail: detail.into() }
    }

    /// An error that is nothing a player did or can mend.
    pub(crate) fn internal(detail: impl Into<String>) -> Error {
        Error::new(ErrorKind::Internal, detail)
    }
}

/// (The detail: what a log says.)
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for Error {}

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

/// What came on a data channel: on the game's, a datagram of the caller's;
/// on the link's, a message of its own (the channel tells them apart, at no
/// cost in bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Message {
    Datagram(Vec<u8>),
    Link(String),
}

/// `N` bytes from the system's cryptographic randomness (the browser's
/// `crypto.getRandomValues` on the web).
pub(crate) fn secure_random<const N: usize>() -> Result<[u8; N], Error> {
    let mut bytes = [0; N];
    #[cfg(not(target_arch = "wasm32"))]
    getrandom::fill(&mut bytes).map_err(|e| Error::internal(format!("no randomness from the system: {e}")))?;
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let crypto = js_sys::Reflect::get(&js_sys::global(), &"crypto".into())
            .ok()
            .and_then(|c| c.dyn_into::<web_sys::Crypto>().ok())
            .ok_or_else(|| Error::internal("the browser has no crypto"))?;
        crypto.get_random_values_with_u8_array(&mut bytes).map_err(|e| Error::internal(format!("no randomness from the browser: {e:?}")))?;
    }
    Ok(bytes)
}

/// A random number from the system's cryptographic randomness (the
/// browser's `crypto.getRandomValues` on the web): a peer's id (all that
/// gives a player its place in a room back), a direct joiner's nonce, and
/// the app's lobby's entropy (its nonces must be past guessing). Only when
/// the system gives none, a weaker one (the clock's, hashed; the browser's
/// `Math.random`).
pub fn random_u64() -> u64 {
    match secure_random::<8>() {
        Ok(bytes) => u64::from_le_bytes(bytes),
        Err(_) => clock_random_u64(),
    }
}

fn clock_random_u64() -> u64 {
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
