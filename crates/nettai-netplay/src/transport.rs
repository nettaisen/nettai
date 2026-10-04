//! The channel peers send their datagrams over, and the handshake that
//! starts a match on it.
//!
//! [`Datagram`] is all a peer needs from a transport: send a datagram to the
//! other peer, and take the next one that arrived, without waiting. Nothing
//! is assumed of delivery: datagrams may be lost, reordered or duplicated
//! (rennet recovers). [`Udp`] is the transport for direct play (a LAN, or
//! the Internet with the host's port forwarded); a WebRTC data channel
//! opened unordered and without retransmits (Tango's) fits the same trait.
//!
//! On the channel, every datagram starts with a byte that says what it is
//! ([`Kind`]): a frame of the protocol, a [`Hello`], or a refusal. The
//! handshake ([`Connection::host`], [`Connection::join`]):
//!
//! 1. each side sends its [`Hello`] (the protocol and engine versions, the
//!    content's hash, its half of the seed and what its player brings), the
//!    joiner first, again every [`RESEND`] until it has the other's;
//! 2. a side that gets a Hello it can't play with (another version, other
//!    content) says why in a refusal and stops, and so does the other on
//!    reading it;
//! 3. once a side has the other's Hello the match is on: both sides know
//!    the seed (both halves) and both players' setups. A Hello that comes
//!    again later (the other side didn't hear ours yet) is answered with
//!    ours; the first frame from the other side is the sign it has ours.

use std::fmt;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

use nettai_battle::ContentHash;

use crate::protocol;
use crate::rng::SplitMix64;
use crate::wire::{Reader, Writer};

/// A datagram channel to the other peer.
pub trait Datagram {
    /// Send one datagram. Never waits; a datagram that can't go now may be
    /// dropped, as the network may drop it.
    fn send(&mut self, datagram: &[u8]) -> io::Result<()>;

    /// The next datagram that has arrived into `buf`, and its length; none
    /// if nothing has. Never waits.
    fn try_recv(&mut self, buf: &mut [u8]) -> io::Result<Option<usize>>;
}

/// The largest datagram a peer sends or takes: a horizon of one-byte ticks
/// and a frame's header with room to spare, under the 1,500-byte Ethernet
/// frame (a payload's chunks can make a frame longer; only recordings have
/// payloads, and they play on one machine).
pub const MAX_DATAGRAM: usize = 64 * 1024;

/// UDP to one other peer.
pub struct Udp {
    socket: UdpSocket,
    /// The other peer, once known (a host learns it from the first Hello).
    peer: Option<SocketAddr>,
}

impl Udp {
    /// A host on `port` of every IPv4 interface, waiting for a joiner: the
    /// first [`Kind::Hello`] that arrives names the other peer.
    pub fn host(port: u16) -> io::Result<Udp> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        socket.set_nonblocking(true)?;
        Ok(Udp { socket, peer: None })
    }

    /// A host on a given address (`127.0.0.1:0` for a test on loopback).
    pub fn host_on(addr: impl ToSocketAddrs) -> io::Result<Udp> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Udp { socket, peer: None })
    }

    /// A joiner of the host at `addr` (`host:port`).
    pub fn join(addr: impl ToSocketAddrs) -> io::Result<Udp> {
        let peer = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "the host's address resolves to nothing"))?;
        let any: SocketAddr = if peer.is_ipv4() { ([0, 0, 0, 0], 0).into() } else { (std::net::Ipv6Addr::UNSPECIFIED, 0).into() };
        let socket = UdpSocket::bind(any)?;
        socket.connect(peer)?;
        socket.set_nonblocking(true)?;
        Ok(Udp { socket, peer: Some(peer) })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    pub fn peer(&self) -> Option<SocketAddr> {
        self.peer
    }
}

/// An error a UDP socket reports for an earlier datagram the other side
/// didn't take (an ICMP "port unreachable": the host isn't up yet, or is
/// gone); the channel itself is fine.
fn transient(e: &io::Error) -> bool {
    matches!(e.kind(), io::ErrorKind::ConnectionRefused | io::ErrorKind::ConnectionReset | io::ErrorKind::WouldBlock)
}

impl Datagram for Udp {
    fn send(&mut self, datagram: &[u8]) -> io::Result<()> {
        if self.peer.is_none() {
            // A host that hasn't heard from a joiner has no one to send to.
            return Ok(());
        }
        match self.socket.send(datagram) {
            Err(e) if transient(&e) => Ok(()),
            r => r.map(|_| ()),
        }
    }

    fn try_recv(&mut self, buf: &mut [u8]) -> io::Result<Option<usize>> {
        loop {
            let r = if self.peer.is_some() { self.socket.recv(buf).map(|n| (n, None)) } else { self.socket.recv_from(buf).map(|(n, a)| (n, Some(a))) };
            match r {
                Ok((n, None)) => return Ok(Some(n)),
                Ok((n, Some(from))) => {
                    // The first Hello names the joiner; anything else before
                    // it is ignored.
                    if n > 0 && buf[0] == Kind::Hello as u8 {
                        self.socket.connect(from)?;
                        self.peer = Some(from);
                        return Ok(Some(n));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(None),
                Err(e) if transient(&e) => continue,
                Err(e) => return Err(e),
            }
        }
    }
}

/// What a datagram on the channel is: its first byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    /// A frame of the protocol ([`crate::protocol::Frame`]).
    Frame = 0,
    /// A [`Hello`].
    Hello = 1,
    /// A refusal: why this side won't play (UTF-8 text).
    Refuse = 2,
}

/// Which side a peer plays: the host is side 0, the joiner side 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Host,
    Join,
}

impl Role {
    /// The side of the battle this role's player plays.
    pub fn side(self) -> usize {
        match self {
            Role::Host => 0,
            Role::Join => 1,
        }
    }
}

/// What a side says before the match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hello {
    /// [`protocol::VERSION`].
    pub protocol: u16,
    /// The engine's version: peers must run the same engine (the state's
    /// digest covers its layout, and the simulation must be the same code).
    pub engine: String,
    /// The game the side plays (`bn6`, `bn5`): a match is of one game, so
    /// both sides must play the same.
    pub game: String,
    /// The content the side plays ([`nettai_battle::Content::hash`]): both
    /// sides must play the same.
    pub content: ContentHash,
    pub role: Role,
    /// This side's half of the match's seed.
    pub nonce: u32,
    /// What this side's player brings (folder, Crosses, game...), in the
    /// frontend's encoding.
    pub setup: Vec<u8>,
}

/// The engine's version as the handshake compares it.
pub const ENGINE: &str = env!("CARGO_PKG_VERSION");

impl Hello {
    /// This side's Hello: this build's versions, the `game` and the
    /// `content` it plays, a nonce from `entropy`, and `setup`.
    pub fn new(role: Role, game: &str, content: ContentHash, setup: Vec<u8>, entropy: u64) -> Hello {
        Hello {
            protocol: protocol::VERSION,
            engine: ENGINE.to_string(),
            game: game.to_string(),
            content,
            role,
            nonce: SplitMix64::new(entropy).next_u64() as u32,
            setup,
        }
    }

    fn to_datagram(&self) -> Vec<u8> {
        let mut out = vec![Kind::Hello as u8];
        let Hello { protocol, engine, game, content, role, nonce, setup } = self;
        let mut w = Writer(&mut out);
        w.put(protocol);
        w.put(engine);
        w.put(game);
        w.put(content);
        w.put(&(*role == Role::Join));
        w.put(nonce);
        w.bytes(setup);
        out
    }

    fn from_datagram(body: &[u8]) -> io::Result<Hello> {
        let mut r = Reader::new(body);
        let protocol = r.get()?;
        let engine = r.get()?;
        let game = r.get()?;
        let content = r.get()?;
        let role = if r.get::<bool>()? { Role::Join } else { Role::Host };
        let nonce = r.get()?;
        let setup = r.bytes()?.to_vec();
        r.finish()?;
        Ok(Hello { protocol, engine, game, content, role, nonce, setup })
    }

    /// Why this side can't play with `other`, if it can't.
    pub fn mismatch(&self, other: &Hello) -> Option<String> {
        if other.protocol != self.protocol {
            return Some(format!("the other side speaks netplay protocol {}, this one {}", other.protocol, self.protocol));
        }
        if other.engine != self.engine {
            return Some(format!("the other side runs engine {}, this one {}", other.engine, self.engine));
        }
        if other.game != self.game {
            return Some(format!("the other side plays {}, this one {}: a match is of one game, both sides playing it", other.game, self.game));
        }
        if other.content != self.content {
            return Some(format!(
                "the other side plays other content (its hash {}, this one's {}): both must play the same content",
                other.content, self.content
            ));
        }
        if other.role == self.role {
            let what = if self.role == Role::Host { "host" } else { "join" };
            return Some(format!("both sides {what}: one hosts and the other joins"));
        }
        None
    }
}

/// Why a handshake didn't make a match.
#[derive(Debug)]
pub enum HandshakeError {
    Io(io::Error),
    /// Nothing from the other side in time.
    TimedOut,
    /// This side refused the other's Hello, for this reason.
    Mismatch(String),
    /// The other side refused this one's, for this reason.
    Refused(String),
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HandshakeError::Io(e) => write!(f, "network error: {e}"),
            HandshakeError::TimedOut => write!(f, "no answer from the other side"),
            HandshakeError::Mismatch(why) => write!(f, "can't play: {why}"),
            HandshakeError::Refused(why) => write!(f, "the other side refused: {why}"),
        }
    }
}

impl std::error::Error for HandshakeError {}

impl From<io::Error> for HandshakeError {
    fn from(e: io::Error) -> HandshakeError {
        HandshakeError::Io(e)
    }
}

/// How often a side sends its Hello again until it has the other's.
pub const RESEND: Duration = Duration::from_millis(100);

/// A match's channel after the handshake: both Hellos, and the frames.
pub struct Connection<D: Datagram> {
    datagram: D,
    ours: Hello,
    theirs: Hello,
    hello: Vec<u8>,
    buf: Vec<u8>,
    /// When the other side was last heard from.
    heard: Instant,
}

impl<D: Datagram> Connection<D> {
    /// Host on `datagram`: wait up to `timeout` for a joiner's Hello.
    pub fn host(datagram: D, hello: Hello, timeout: Duration) -> Result<Connection<D>, HandshakeError> {
        assert_eq!(hello.role, Role::Host);
        Connection::handshake(datagram, hello, timeout)
    }

    /// Join the host `datagram` is connected to, waiting up to `timeout`.
    pub fn join(datagram: D, hello: Hello, timeout: Duration) -> Result<Connection<D>, HandshakeError> {
        assert_eq!(hello.role, Role::Join);
        Connection::handshake(datagram, hello, timeout)
    }

    fn handshake(mut datagram: D, ours: Hello, timeout: Duration) -> Result<Connection<D>, HandshakeError> {
        let start = Instant::now();
        let hello = ours.to_datagram();
        let mut buf = vec![0u8; MAX_DATAGRAM];
        let mut sent: Option<Instant> = None;
        loop {
            // The joiner speaks first; the host answers each Hello.
            if ours.role == Role::Join && sent.is_none_or(|t| t.elapsed() >= RESEND) {
                datagram.send(&hello)?;
                sent = Some(Instant::now());
            }
            while let Some(n) = datagram.try_recv(&mut buf)? {
                let Some((&kind, body)) = buf[..n].split_first() else { continue };
                if kind == Kind::Refuse as u8 {
                    return Err(HandshakeError::Refused(String::from_utf8_lossy(body).into_owned()));
                }
                if kind != Kind::Hello as u8 {
                    // A frame: the other side has our Hello and plays;
                    // its Hello comes again with our next one.
                    continue;
                }
                let Ok(theirs) = Hello::from_datagram(body) else { continue };
                if let Some(why) = ours.mismatch(&theirs) {
                    let mut refusal = vec![Kind::Refuse as u8];
                    refusal.extend_from_slice(why.as_bytes());
                    for _ in 0..3 {
                        datagram.send(&refusal)?;
                    }
                    return Err(HandshakeError::Mismatch(why));
                }
                datagram.send(&hello)?;
                return Ok(Connection { datagram, ours, theirs, hello, buf, heard: Instant::now() });
            }
            if start.elapsed() >= timeout {
                return Err(HandshakeError::TimedOut);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// This side's Hello.
    pub fn ours(&self) -> &Hello {
        &self.ours
    }

    /// The other side's.
    pub fn theirs(&self) -> &Hello {
        &self.theirs
    }

    /// The side this peer's player plays.
    pub fn side(&self) -> usize {
        self.ours.role.side()
    }

    /// The match's seed, from both halves.
    pub fn seed(&self) -> u32 {
        let (host, join) = if self.ours.role == Role::Host { (&self.ours, &self.theirs) } else { (&self.theirs, &self.ours) };
        SplitMix64::new((host.nonce as u64) << 32 | join.nonce as u64).next_u64() as u32
    }

    /// Each side's setup bytes, by side.
    pub fn setups(&self) -> [&[u8]; 2] {
        let mine = &self.ours.setup[..];
        let theirs = &self.theirs.setup[..];
        if self.side() == 0 { [mine, theirs] } else { [theirs, mine] }
    }

    /// Send a frame of the protocol.
    pub fn send_frame(&mut self, frame: &[u8]) -> io::Result<()> {
        let mut out = Vec::with_capacity(frame.len() + 1);
        out.push(Kind::Frame as u8);
        out.extend_from_slice(frame);
        self.datagram.send(&out)
    }

    /// The next frame that arrived, if any, without waiting. A Hello that
    /// comes again is answered; a refusal ends the match.
    pub fn try_recv_frame(&mut self) -> Result<Option<&[u8]>, HandshakeError> {
        loop {
            let Some(n) = self.datagram.try_recv(&mut self.buf)? else { return Ok(None) };
            self.heard = Instant::now();
            match self.buf[..n].split_first() {
                Some((&k, _)) if k == Kind::Frame as u8 => return Ok(Some(&self.buf[1..n])),
                Some((&k, _)) if k == Kind::Hello as u8 => self.datagram.send(&self.hello)?,
                Some((&k, body)) if k == Kind::Refuse as u8 => {
                    return Err(HandshakeError::Refused(String::from_utf8_lossy(body).into_owned()));
                }
                _ => {}
            }
        }
    }

    /// How long since the other side was last heard from.
    pub fn silence(&self) -> Duration {
        self.heard.elapsed()
    }

    pub fn datagram(&self) -> &D {
        &self.datagram
    }
}

/// A datagram channel in memory, for tests: two ends, each taking what the
/// other sends, in order, nothing lost.
pub fn memory_pair() -> (Memory, Memory) {
    use std::sync::{Arc, Mutex};
    let a = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    let b = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    (Memory { inbox: a.clone(), outbox: b.clone() }, Memory { inbox: b, outbox: a })
}

/// One end of [`memory_pair`].
pub struct Memory {
    inbox: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<Vec<u8>>>>,
    outbox: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<Vec<u8>>>>,
}

impl Datagram for Memory {
    fn send(&mut self, datagram: &[u8]) -> io::Result<()> {
        self.outbox.lock().unwrap().push_back(datagram.to_vec());
        Ok(())
    }

    fn try_recv(&mut self, buf: &mut [u8]) -> io::Result<Option<usize>> {
        let Some(d) = self.inbox.lock().unwrap().pop_front() else { return Ok(None) };
        let n = d.len().min(buf.len());
        buf[..n].copy_from_slice(&d[..n]);
        Ok(Some(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(role: Role, content: u64, setup: &[u8]) -> Hello {
        Hello::new(role, "bn6", ContentHash(content), setup.to_vec(), content ^ role as u64)
    }

    fn both(a: Hello, b: Hello) -> (Result<Connection<Memory>, HandshakeError>, Result<Connection<Memory>, HandshakeError>) {
        let (x, y) = memory_pair();
        let host = std::thread::spawn(move || Connection::host(x, a, Duration::from_secs(5)));
        let join = Connection::join(y, b, Duration::from_secs(5));
        (host.join().unwrap(), join)
    }

    #[test]
    fn a_handshake_agrees_the_seed_and_swaps_the_setups() {
        let (h, j) = both(hello(Role::Host, 7, b"host's"), hello(Role::Join, 7, b"joiner's"));
        let (h, j) = (h.unwrap(), j.unwrap());
        assert_eq!(h.seed(), j.seed());
        assert_eq!((h.side(), j.side()), (0, 1));
        assert_eq!(h.setups(), [&b"host's"[..], &b"joiner's"[..]]);
        assert_eq!(j.setups(), h.setups());
    }

    /// Other content: both sides stop, each saying why.
    #[test]
    fn other_content_is_refused_on_both_sides() {
        let (h, j) = both(hello(Role::Host, 7, b""), hello(Role::Join, 8, b""));
        let (h, j) = (h.err().unwrap(), j.err().unwrap());
        let both = [h.to_string(), j.to_string()];
        assert!(both.iter().any(|e| e.starts_with("can't play: the other side plays other content")), "{both:?}");
        assert!(both.iter().any(|e| e.starts_with("the other side refused: the other side plays other content")), "{both:?}");
    }

    /// Another game: both sides stop, each saying which game the other
    /// plays (before the content, which differs too).
    #[test]
    fn another_game_is_refused_on_both_sides() {
        let mut joiner = hello(Role::Join, 8, b"");
        joiner.game = "bn5".into();
        let (h, j) = both(hello(Role::Host, 7, b""), joiner);
        let (h, j) = (h.err().unwrap().to_string(), j.err().unwrap().to_string());
        assert_eq!(h, "can't play: the other side plays bn5, this one bn6: a match is of one game, both sides playing it");
        assert_eq!(j, "the other side refused: the other side plays bn5, this one bn6: a match is of one game, both sides playing it");
    }

    #[test]
    fn another_engine_or_protocol_is_refused() {
        let mut j = hello(Role::Join, 7, b"");
        j.engine = "0.0.1".into();
        assert!(hello(Role::Host, 7, b"").mismatch(&j).unwrap().contains("engine 0.0.1"));
        j = hello(Role::Join, 7, b"");
        j.protocol += 1;
        assert!(hello(Role::Host, 7, b"").mismatch(&j).unwrap().contains("protocol"));
        assert!(hello(Role::Host, 7, b"").mismatch(&hello(Role::Host, 7, b"")).unwrap().contains("both sides host"));
    }

    #[test]
    fn a_hello_roundtrips() {
        let h = hello(Role::Join, 0xABCD, &[1, 2, 3]);
        let d = h.to_datagram();
        assert_eq!(d[0], Kind::Hello as u8);
        assert_eq!(Hello::from_datagram(&d[1..]).unwrap(), h);
    }
}
