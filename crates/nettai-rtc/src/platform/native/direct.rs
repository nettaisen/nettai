//! Direct connect (native only): a WebRTC connection to a host's address
//! and port, with no signaling at all. Nothing is exchanged before ICE:
//! there are no descriptions, each side starting ICE, DTLS and SCTP knowing
//! what a description would have told it (str0m's direct API,
//! `PeerConnection::start_direct`). It is built on what the native
//! connection can do and a browser can't: a fixed local port, local ICE
//! credentials of the caller's choosing, a start without descriptions, and
//! no DTLS fingerprint check.
//!
//! - **The ICE credentials** are fixed: the host's username fragment and
//!   both passwords constants, the joiner's fragment its nonce (drawn once
//!   a link) and the connection's generation (`j<nonce>g<generation>`).
//! - **The candidates**: the joiner is told the host's address, its one
//!   remote candidate. The host learns the joiner's from the joiner's first
//!   connectivity check (below), and is told it as one too. Neither side's
//!   own candidates are said to anyone.
//! - **SCTP and the data channels**: the same as a room's (port 5000, the
//!   channels negotiated as streams 0 and 1).
//! - **The DTLS fingerprint** is the one thing a description carries that
//!   can't be known beforehand (each side's certificate is its own,
//!   generated): it isn't checked. So direct connect authenticates nobody,
//!   as the raw UDP it replaced didn't; a room checks the fingerprints its
//!   descriptions carry.
//!
//! The joiner dials (ICE's controlling side, the DTLS and SCTP client),
//! from a port the system picks. The host listens on its port with a plain
//! UDP socket of its own ([`Listener`]) while it has no connection: the
//! first connectivity check of a dial names its generation in its username,
//! and the host then closes the socket and makes the connection answering
//! that dial on the same port (the connection binds it), which takes the
//! dial's next checks (the joiner sends each again, from half a second on).
//! Once that connection is over (dropped, or not open within
//! [`Config::dial_timeout`]) the host listens again. It takes only the first
//! joiner's nonce: another joiner can't take the match over. While the
//! host's connection has the port, a dial of a newer generation is refused
//! by it (the fragment isn't the one it expects): the host hears it once
//! that connection is over, which the joiner's dialing again means it soon
//! is (its old connection closed, or silent).
//!
//! What crosses a NAT is what crossed it before: on a LAN, or over the
//! Internet with the host's UDP port forwarded to it. The joiner dials out,
//! so its own NAT needs nothing.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

use crate::rtc::{Configuration, Options};

use crate::link::Link;
use crate::peer::PeerConnection;
use crate::{Candidate, Config, Error, ErrorKind, Role};

const HOST_UFRAG: &str = "nettaihost";
const HOST_PWD: &str = "nettaidirectconnecthostpwd";
const JOIN_PWD: &str = "nettaidirectconnectjoinpwd";

/// The largest datagram the listener reads.
const MAX_DATAGRAM: usize = 64 * 1024;

/// The joiner's username fragment for connection `generation`.
fn join_ufrag(nonce: u32, generation: u32) -> String {
    format!("j{nonce:08x}g{generation}")
}

/// The nonce and the generation a joiner's username fragment names.
fn read_join_ufrag(ufrag: &str) -> Option<(u32, u32)> {
    let (nonce, generation) = ufrag.strip_prefix('j')?.split_once('g')?;
    Some((u32::from_str_radix(nonce, 16).ok()?, generation.parse().ok()?))
}

/// A host candidate at `addr`, the other side's one.
fn candidate(addr: SocketAddr) -> Candidate {
    Candidate { candidate: format!("candidate:1 1 UDP 2122317823 {} {} typ host", addr.ip(), addr.port()), mid: Some("0".into()), mline: Some(0) }
}

/// A direct connection's configuration: no STUN or TURN server, bound to
/// `bind` (port 0: one the system picks), these ICE credentials, no
/// certificate check (direct connect exchanges no descriptions: nothing
/// tells either side the other's fingerprint), and checks for as long as a
/// dial is given.
fn configuration(config: &Config, bind: SocketAddr, ufrag: &str, pwd: &str) -> Configuration {
    Configuration {
        system: Options {
            bind: Some(bind),
            ice_credentials: Some((ufrag.to_string(), pwd.to_string())),
            skip_fingerprint_check: true,
            checks_for: config.dial_timeout,
        },
        ..Configuration::default()
    }
}

impl PeerConnection {
    /// The joiner's connection `generation` to the host at `host_addr`, from
    /// a new port: dialing at once.
    pub(crate) fn direct_dial(config: &Config, host_addr: SocketAddr, nonce: u32, generation: u32) -> Result<PeerConnection, Error> {
        let any = if host_addr.is_ipv4() { IpAddr::V4(Ipv4Addr::UNSPECIFIED) } else { IpAddr::V6(Ipv6Addr::UNSPECIFIED) };
        let mut c = PeerConnection::new(configuration(config, SocketAddr::new(any, 0), &join_ufrag(nonce, generation), JOIN_PWD), config)?;
        c.start_direct(HOST_UFRAG, HOST_PWD, true)?;
        c.add_candidate(&candidate(host_addr))?;
        Ok(c)
    }

    /// The host's connection answering the joiner `nonce`'s dial
    /// `generation`, whose check came from `joiner`, on `port`.
    fn direct_answer(config: &Config, port: u16, nonce: u32, generation: u32, joiner: SocketAddr) -> Result<PeerConnection, Error> {
        let mut c = PeerConnection::new(configuration(config, SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)), HOST_UFRAG, HOST_PWD), config)?;
        c.start_direct(&join_ufrag(nonce, generation), JOIN_PWD, false)?;
        c.add_candidate(&candidate(joiner))?;
        Ok(c)
    }
}

/// The host's end of direct connect: its port on every IPv4 interface, a
/// socket of its own on it while there is no connection (which has it then),
/// and the joiner it took.
pub(crate) struct Listener {
    socket: Option<UdpSocket>,
    port: u16,
    nonce: Option<u32>,
}

impl Listener {
    /// Listen on UDP `port` of every IPv4 interface (0: a port the system
    /// picks).
    pub(crate) fn bind(port: u16) -> Result<Listener, Error> {
        let socket = Listener::open(port).map_err(|e| Error::new(ErrorKind::PortUnavailable, format!("can't listen on UDP port {port}: {e}")))?;
        let port = socket.local_addr().map_err(|e| Error::internal(e.to_string()))?.port();
        Ok(Listener { socket: Some(socket), port, nonce: None })
    }

    fn open(port: u16) -> std::io::Result<UdpSocket> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))?;
        socket.set_nonblocking(true)?;
        Ok(socket)
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    /// Listen again, once the connection that had the port is over (its
    /// thread lets the port go a moment after it closes: until then, not
    /// yet).
    pub(crate) fn listen(&mut self) {
        if self.socket.is_none() {
            self.socket = Listener::open(self.port).ok();
        }
    }

    /// The next datagram that came into `buf`, its length, and from where.
    pub(crate) fn recv(&mut self, buf: &mut [u8]) -> Option<(usize, SocketAddr)> {
        let socket = self.socket.as_ref()?;
        loop {
            match socket.recv_from(buf) {
                Ok(got) => return Some(got),
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue,
                Err(_) => return None,
            }
        }
    }

    /// The generation `datagram` dials, if it is a connectivity check of
    /// this match's joiner (the first joiner to check takes the match).
    pub(crate) fn dialed(&mut self, datagram: &[u8]) -> Option<u32> {
        let username = binding_username(datagram)?;
        let (to, from) = username.split_once(':')?;
        let (nonce, generation) = read_join_ufrag(from).filter(|_| to == HOST_UFRAG)?;
        if *self.nonce.get_or_insert(nonce) != nonce {
            return None;
        }
        Some(generation)
    }

    /// The host's connection answering dial `generation` of the joiner it
    /// took, whose check came from `joiner`: the port is the connection's
    /// now.
    pub(crate) fn answer(&mut self, config: &Config, generation: u32, joiner: SocketAddr) -> Result<PeerConnection, Error> {
        self.socket = None;
        PeerConnection::direct_answer(config, self.port, self.nonce.expect("a joiner dialed"), generation, joiner)
    }
}

impl Link {
    /// Host directly: listen on UDP `port` of every IPv4 interface (0: one
    /// the system picks, [`Link::port`]) for the joiner. A port that can't
    /// be listened on errs ([`ErrorKind::PortUnavailable`]). A direct link
    /// asks no STUN or TURN server: it says no candidate to anyone.
    pub fn host(port: u16, config: Config) -> Result<Link, Error> {
        let listener = Listener::bind(port)?;
        let direct = Direct::Listen { listener, buf: vec![0; MAX_DATAGRAM] };
        Ok(Link::directly(config, direct, Role::Host))
    }

    /// Join the host at `addr` (`host:port`) directly (resolving its name
    /// now, which may wait). An address that names no machine errs
    /// ([`ErrorKind::BadAddress`]).
    pub fn join(addr: &str, config: Config) -> Result<Link, Error> {
        use std::net::ToSocketAddrs;
        let host = addr
            .to_socket_addrs()
            .map_err(|e| Error::new(ErrorKind::BadAddress, format!("can't resolve {addr}: {e}")))?
            .min_by_key(|a| !a.is_ipv4())
            .ok_or_else(|| Error::new(ErrorKind::BadAddress, format!("{addr} resolves to nothing")))?;
        let nonce = crate::random_u64() as u32;
        Ok(Link::directly(config, Direct::Dial { host, nonce }, Role::Join))
    }
}

/// How a link meets directly: the host listening on its port (reading what
/// comes into `buf`), or the joiner dialing the host's address, as `nonce`.
pub(crate) enum Direct {
    Listen { listener: Listener, buf: Vec<u8> },
    Dial { host: SocketAddr, nonce: u32 },
}

impl Direct {
    /// The host's port.
    pub(crate) fn port(&self) -> Option<u16> {
        match self {
            Direct::Listen { listener, .. } => Some(listener.port()),
            Direct::Dial { .. } => None,
        }
    }

    /// Who the other player is, for a log: the port listened on, or the
    /// address dialed.
    pub(crate) fn describe(&self) -> String {
        match self {
            Direct::Listen { listener, .. } => format!("UDP port {}", listener.port()),
            Direct::Dial { host, .. } => host.to_string(),
        }
    }

    /// Whether this side is the host, which listens.
    pub(crate) fn hosts(&self) -> bool {
        matches!(self, Direct::Listen { .. })
    }

    /// The host without a connection: listen on its port, and answer a
    /// joiner's check of a generation newer than `generation` with a new
    /// connection, and its generation (what comes during a test's `outage`
    /// is dropped). The joiner: nothing.
    pub(crate) fn answer(&mut self, config: &Config, generation: u32, outage: bool) -> Result<Option<(PeerConnection, u32)>, Error> {
        let Direct::Listen { listener, buf } = self else { return Ok(None) };
        listener.listen();
        while let Some((n, from)) = listener.recv(buf) {
            if outage {
                continue;
            }
            if let Some(dialed) = listener.dialed(&buf[..n])
                && dialed > generation
            {
                return Ok(Some((listener.answer(config, dialed, from)?, dialed)));
            }
        }
        Ok(None)
    }

    /// The joiner's connection `generation`, dialing the host. The host
    /// dials no one: none.
    pub(crate) fn dial(&self, config: &Config, generation: u32) -> Option<Result<PeerConnection, Error>> {
        match self {
            Direct::Dial { host, nonce } => Some(PeerConnection::direct_dial(config, *host, *nonce, generation)),
            Direct::Listen { .. } => None,
        }
    }
}

/// A STUN binding request's USERNAME (RFC 8489: a 20-byte header, the
/// magic cookie, then type-length-value attributes padded to four bytes).
fn binding_username(datagram: &[u8]) -> Option<&str> {
    const BINDING_REQUEST: u16 = 0x0001;
    const MAGIC_COOKIE: u32 = 0x2112_a442;
    const USERNAME: u16 = 0x0006;
    let u16_at = |i: usize| datagram.get(i..i + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    if u16_at(0)? != BINDING_REQUEST || datagram.get(4..8)? != MAGIC_COOKIE.to_be_bytes() {
        return None;
    }
    let end = (20 + u16_at(2)? as usize).min(datagram.len());
    let mut at = 20;
    while at + 4 <= end {
        let (kind, length) = (u16_at(at)?, u16_at(at + 2)? as usize);
        let value = datagram.get(at + 4..at + 4 + length)?;
        if kind == USERNAME {
            return std::str::from_utf8(value).ok();
        }
        at += 4 + length.next_multiple_of(4);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_joiners_fragment_names_its_dial() {
        assert_eq!(read_join_ufrag(&join_ufrag(0xdead_beef, 12)), Some((0xdead_beef, 12)));
        assert_eq!(read_join_ufrag("nettaihost"), None);
        // (At least 4 ICE characters; the passwords at least 22.)
        assert!(join_ufrag(0, 0).len() >= 4 && HOST_PWD.len() >= 22 && JOIN_PWD.len() >= 22);
    }

    /// A binding request's username is read, past the attributes before it;
    /// anything else isn't a dial.
    #[test]
    fn a_checks_username_is_read() {
        // (A binding request: PRIORITY, then USERNAME, padded.)
        let check = |username: &str| {
            let mut check = vec![0x00, 0x01, 0, 0, 0x21, 0x12, 0xa4, 0x42];
            check.extend([7; 12]);
            check.extend([0x00, 0x24, 0, 4, 1, 2, 3, 4]);
            check.extend([0x00, 0x06, 0, username.len() as u8]);
            check.extend(username.as_bytes());
            check.resize(check.len().next_multiple_of(4), 0);
            let length = (check.len() - 20) as u16;
            check[2..4].copy_from_slice(&length.to_be_bytes());
            check
        };
        assert_eq!(binding_username(&check("nettaihost:j0000002ag3")), Some("nettaihost:j0000002ag3"));
        let mut listener = Listener { socket: None, port: 0, nonce: None };
        assert_eq!(listener.dialed(&check("nettaihost:j0000002ag3")), Some(3));
        assert_eq!(listener.dialed(&check("nettaihost:j0000002ag4")), Some(4));
        // Another joiner's is refused, and a check for someone else; so is
        // what isn't a binding request.
        assert_eq!(listener.dialed(&check("nettaihost:j0000002bg5")), None);
        assert_eq!(listener.dialed(&check("someone:j0000002ag5")), None);
        let mut response = check("nettaihost:j0000002ag3");
        response[1] = 0x03;
        assert_eq!(binding_username(&response), None);
        assert_eq!(binding_username(b"not stun"), None);
    }
}

