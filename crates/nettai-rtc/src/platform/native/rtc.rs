//! The native WebRTC connection: str0m (sans-I/O ICE, DTLS, SCTP and data
//! channels, pure Rust) with the sockets, the timers, the candidates and the
//! TURN relays this module's, behind `crate::rtc`'s API: calls, and
//! [`Event`]s taken with [`PeerConnection::poll`].
//!
//! **The driver.** Each connection is a task on nettai-rtc's own runtime
//! (tokio, one thread of its own: [`runtime`]), so what the app's other work
//! does can't hold a datagram up. The task waits on the connection's sockets,
//! str0m's next timeout, the servers' names resolved and a wake-up, takes
//! what came, and hands str0m's output on: datagrams to the sockets (or
//! through a TURN relay), events to the connection's queue. A call from the
//! caller's thread (a send, a description, a candidate) does its part under
//! the lock there, its datagrams sent at once, and wakes the task if str0m
//! then wants a sooner timeout.
//!
//! **Sockets and candidates** (`gather`): one UDP socket a family, on every
//! interface (`0.0.0.0`, `[::]`) unless the configuration binds one address
//! and port (direct connect). Each datagram's destination address is read
//! with it (quinn-udp), which tells str0m which candidate it came to. The
//! host candidates are the interfaces' addresses on the socket's port (IPv4,
//! and IPv6's global ones; the loopback address only where there is no
//! other), and 127.0.0.1 always, unsaid: what `crate::peer` offers on it for
//! [`crate::Config::loopback`] reaches the socket, and str0m takes it. A
//! STUN server is asked each family's address as the Internet sees it (a
//! server-reflexive candidate, stun-proto's request), and a TURN server (two
//! at most, over UDP) for a relayed address (turn-client-proto's client). A
//! relay-only configuration offers the relayed candidates alone, and str0m
//! checks nothing but through them.
//!
//! **Descriptions** are str0m's SDP. Its candidates are left out of them and
//! said one by one ([`Event::Candidate`]), after the description, as a
//! browser trickles them. Only channels negotiated out of band are made.

mod driver;
mod gather;

use std::sync::{Arc, OnceLock};

use driver::Shared;

use crate::rtc::{ChannelId, ChannelOptions, Configuration, Error, Event, SdpType};

/// nettai-rtc's runtime: tokio on one thread of its own, made at the first
/// connection, for every connection after.
fn runtime() -> Result<&'static tokio::runtime::Handle, Error> {
    static RUNTIME: OnceLock<Result<tokio::runtime::Handle, String>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().map_err(|e| e.to_string())?;
            let handle = runtime.handle().clone();
            std::thread::Builder::new()
                .name("nettai-rtc".into())
                .spawn(move || runtime.block_on(std::future::pending::<()>()))
                .map_err(|e| e.to_string())?;
            Ok(handle)
        })
        .as_ref()
        .map_err(|e| Error::new(format!("can't start the connections' thread: {e}")))
}

/// What the native connection takes beyond a browser's
/// ([`Configuration::system`]).
#[derive(Clone, Debug, Default)]
pub(crate) struct Options {
    /// Bind this address and port (port 0: one the system picks) instead of
    /// every interface's, both families, on ports the system picks.
    pub(crate) bind: Option<std::net::SocketAddr>,
    /// This side's ICE username fragment and password, instead of random
    /// ones.
    pub(crate) ice_credentials: Option<(String, String)>,
    /// Don't check the other side's DTLS certificate against its
    /// description's fingerprint.
    pub(crate) skip_fingerprint_check: bool,
    /// How long ICE keeps checking a pair that doesn't answer: as long as a
    /// dial is given ([`crate::Config::dial_timeout`]), so that a dial made
    /// while the path is out (or while a direct host's port is another
    /// connection's) connects as soon as it is back.
    pub(crate) checks_for: std::time::Duration,
}

impl Options {
    /// A room's connection's: ICE checks for as long as a dial is given.
    pub(crate) fn in_room(config: &crate::Config) -> Options {
        Options { checks_for: config.dial_timeout, ..Options::default() }
    }
}

/// Whether the connection can reach the other side's candidate `candidate`
/// (`candidate:...`): not a browser's under an mDNS name (`.local`), nor TCP.
pub(crate) fn reaches(candidate: &str) -> bool {
    let fields: Vec<&str> = candidate.split_whitespace().collect();
    !(fields.get(2).is_some_and(|p| !p.eq_ignore_ascii_case("udp")) || fields.get(4).is_some_and(|a| a.ends_with(".local")))
}

/// One WebRTC connection.
pub(crate) struct PeerConnection {
    shared: Arc<Shared>,
}

impl PeerConnection {
    /// A connection on `config`: its sockets bound and its host candidates
    /// known (said once its description is).
    pub(crate) fn new(config: Configuration) -> Result<PeerConnection, Error> {
        Ok(PeerConnection { shared: Shared::new(config, runtime()?)? })
    }

    /// A channel, before this side's description.
    pub(crate) fn create_channel(&mut self, label: &str, options: ChannelOptions) -> Result<ChannelId, Error> {
        self.shared.create_channel(label, options)
    }

    /// Make this side's description: it comes as [`Event::Description`],
    /// and the candidates after it.
    pub(crate) fn set_local_description(&mut self, type_: SdpType) -> Result<(), Error> {
        self.shared.set_local_description(type_)
    }

    pub(crate) fn set_remote_description(&mut self, type_: SdpType, sdp: &str) -> Result<(), Error> {
        self.shared.set_remote_description(type_, sdp)
    }

    /// Start without descriptions (direct connect), knowing the other side's
    /// ICE credentials: this side dials (ICE's controlling side, the DTLS
    /// and SCTP client) or answers.
    pub(crate) fn start_direct(&mut self, ufrag: &str, pwd: &str, dialing: bool) -> Result<(), Error> {
        self.shared.start_direct(ufrag, pwd, dialing)
    }

    /// One of the other side's candidates (`candidate:...`, as a browser
    /// says them; empty: the end of them).
    pub(crate) fn add_remote_candidate(&mut self, candidate: &str) -> Result<(), Error> {
        self.shared.add_remote_candidate(candidate)
    }

    /// Send `data` on `channel`; an error if it isn't open or its buffer is
    /// full.
    pub(crate) fn send(&mut self, channel: ChannelId, data: &[u8]) -> Result<(), Error> {
        self.shared.send(channel, data)
    }

    /// Close it: the other side is told (if the path is up), the sockets
    /// let go, and [`crate::rtc::State::Closed`] said.
    pub(crate) fn close(&mut self) {
        self.shared.close();
    }

    /// The next thing the connection has to say, if any.
    pub(crate) fn poll(&mut self) -> Option<Event> {
        self.shared.poll()
    }
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        self.shared.close();
    }
}
