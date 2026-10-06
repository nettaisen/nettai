//! Netplay's transport, the program's own (`--host`, `--join`): the UDP
//! socket to the other player, and the handshake that starts a match on
//! it. The library has none: it plays a match on any channel its host
//! gives it (nettai-frontend's `netplay::Channel`), and a larger app brings
//! its own (a signaling server and WebRTC, say) and hands the library the
//! agreed match and the frames.
//!
//! [`Datagram`] is all the handshake needs from a socket: send a datagram
//! to the other peer, and take the next one that arrived, without waiting.
//! Nothing is assumed of delivery: datagrams may be lost, reordered or
//! duplicated (rennet recovers). [`Udp`] is direct play (a LAN, or the
//! Internet with the host's port forwarded).
//!
//! On the channel, every datagram starts with a byte that says what it is
//! ([`Kind`]): a frame of the protocol, a [`Hello`], or a refusal. The
//! handshake ([`Handshake`], which the window polls each frame, so it never
//! waits):
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
//!
//! [`NetHandshake`] is that handshake, then the match agreed from both
//! players' offers (`netplay::agree`); the [`Connection`] it makes is the
//! channel the library's `NetPlayer` plays on.

use std::fmt;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, Instant};

use nettai_battle::{Content, ContentHash};
use nettai_frontend::netplay::{Channel, Offer, agree};
use nettai_match::{Match, Set};
use nettai_netplay::protocol;
use nettai_netplay::rng::SplitMix64;
use nettai_netplay::wire::{Reader, Writer};

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
/// frame.
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
    /// A frame of the protocol ([`protocol::Frame`]).
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
    /// The game the side plays (`exe6`, `exe5`): a match is of one game, so
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

/// The engine's version as the handshake compares it (the program's: the
/// engine's crates are versioned with it).
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

/// A handshake under way, which never waits: the window steps it with
/// [`Handshake::poll`] each frame until it makes a [`Connection`] or fails.
pub struct Handshake<D: Datagram> {
    /// The channel, until the handshake ends (the connection takes it).
    datagram: Option<D>,
    ours: Hello,
    /// `ours` as sent.
    hello: Vec<u8>,
    buf: Vec<u8>,
    timeout: Duration,
    /// When it was first polled.
    start: Option<Instant>,
    /// When the joiner last sent its Hello.
    sent: Option<Instant>,
}

impl<D: Datagram> Handshake<D> {
    /// A handshake on `datagram` in `hello`'s role (a host waits for a
    /// joiner's Hello; a joiner sends its own to the host `datagram` is
    /// connected to), failing if nothing comes from the other side within
    /// `timeout` of its first poll.
    pub fn new(datagram: D, hello: Hello, timeout: Duration) -> Handshake<D> {
        Handshake {
            datagram: Some(datagram),
            hello: hello.to_datagram(),
            ours: hello,
            buf: vec![0u8; MAX_DATAGRAM],
            timeout,
            start: None,
            sent: None,
        }
    }

    /// Step the handshake at `now`: send what is due, take what has
    /// arrived, without waiting. Pending until it makes a connection or
    /// fails; polling it after that panics.
    pub fn poll(&mut self, now: Instant) -> Poll<Result<Connection<D>, HandshakeError>> {
        let mut datagram = self.datagram.take().expect("a handshake polled after it ended");
        match self.step(&mut datagram, now) {
            Ok(None) => {
                self.datagram = Some(datagram);
                Poll::Pending
            }
            Ok(Some(theirs)) => Poll::Ready(Ok(Connection {
                datagram,
                ours: self.ours.clone(),
                theirs,
                hello: std::mem::take(&mut self.hello),
                buf: std::mem::take(&mut self.buf),
            })),
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    /// One step: the other side's Hello, once it has come.
    fn step(&mut self, datagram: &mut D, now: Instant) -> Result<Option<Hello>, HandshakeError> {
        let start = *self.start.get_or_insert(now);
        // The joiner speaks first; the host answers each Hello.
        if self.ours.role == Role::Join && self.sent.is_none_or(|t| now.duration_since(t) >= RESEND) {
            datagram.send(&self.hello)?;
            self.sent = Some(now);
        }
        while let Some(n) = datagram.try_recv(&mut self.buf)? {
            let Some((&kind, body)) = self.buf[..n].split_first() else { continue };
            if kind == Kind::Refuse as u8 {
                return Err(HandshakeError::Refused(String::from_utf8_lossy(body).into_owned()));
            }
            if kind != Kind::Hello as u8 {
                // A frame: the other side has our Hello and plays; its
                // Hello comes again with our next one.
                continue;
            }
            let Ok(theirs) = Hello::from_datagram(body) else { continue };
            if let Some(why) = self.ours.mismatch(&theirs) {
                let mut refusal = vec![Kind::Refuse as u8];
                refusal.extend_from_slice(why.as_bytes());
                for _ in 0..3 {
                    datagram.send(&refusal)?;
                }
                return Err(HandshakeError::Mismatch(why));
            }
            datagram.send(&self.hello)?;
            return Ok(Some(theirs));
        }
        if now.duration_since(start) >= self.timeout {
            return Err(HandshakeError::TimedOut);
        }
        Ok(None)
    }
}

/// A match's channel after the handshake: both Hellos, and the frames
/// (the library's [`Channel`]).
pub struct Connection<D: Datagram> {
    datagram: D,
    ours: Hello,
    theirs: Hello,
    hello: Vec<u8>,
    buf: Vec<u8>,
}

impl<D: Datagram> Connection<D> {
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

    pub fn datagram(&self) -> &D {
        &self.datagram
    }
}

/// The frames, for the library's player: an error says what
/// [`Connection::send_frame`] or [`Connection::try_recv_frame`] does.
impl<D: Datagram> Channel for Connection<D> {
    fn send(&mut self, frame: &[u8]) -> Result<(), String> {
        self.send_frame(frame).map_err(|e| e.to_string())
    }

    fn recv(&mut self) -> Result<Option<&[u8]>, String> {
        self.try_recv_frame().map_err(|e| e.to_string())
    }
}

/// This side's Hello for a match of the offer's game on `content`, in
/// `role`, offering `offer` (in the frontend's encoding, `Offer::to_bytes`).
pub fn hello(role: Role, content: &Content, offer: &Offer) -> Hello {
    let entropy = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0) ^ std::process::id() as u64;
    Hello::new(role, &offer.game, content.hash(), offer.to_bytes(content), entropy)
}

/// A netplay match being agreed, which never waits: the window polls it
/// each frame (and stays responsive) until the match is agreed or can't be.
/// It is the [`Handshake`] (the Hellos, with this player's offer, and a
/// refusal of another game, content or version), then `netplay::agree`.
pub struct NetHandshake<D: Datagram> {
    content: Arc<Content>,
    mine: Offer,
    handshake: Handshake<D>,
}

/// How a [`NetHandshake`] stands after a poll.
pub enum Progress<D: Datagram> {
    /// Still going: poll it again.
    Pending,
    /// The match is agreed.
    Agreed(Agreed<D>),
    /// No match, and why (the timeout, a refusal either way, the other
    /// player's offer refused, a network error).
    Failed(String),
}

/// An agreed match: the channel to play it on and what is played, for
/// `netplay::NetPlayer::new` (the connection, its side and the set).
pub struct Agreed<D: Datagram> {
    pub conn: Connection<D>,
    /// Both offers, by side (the other's checked).
    pub offers: [Offer; 2],
    pub set: Set,
    /// The set's match.
    pub m: Match,
}

impl<D: Datagram> NetHandshake<D> {
    /// A handshake on `datagram` in `role` for a match on `content`,
    /// offering `mine`; it fails if nothing comes from the other side
    /// within `timeout` of its first poll.
    pub fn new(role: Role, datagram: D, content: &Arc<Content>, mine: Offer, timeout: Duration) -> NetHandshake<D> {
        let handshake = Handshake::new(datagram, hello(role, content, &mine), timeout);
        NetHandshake { content: content.clone(), mine, handshake }
    }

    /// Step the handshake at `now`, without waiting; polling it after it
    /// ended panics.
    pub fn poll(&mut self, now: Instant) -> Progress<D> {
        match self.handshake.poll(now) {
            Poll::Pending => Progress::Pending,
            Poll::Ready(Err(e)) => Progress::Failed(e.to_string()),
            Poll::Ready(Ok(conn)) => match agree(&self.content, conn.side(), conn.seed(), &self.mine, &conn.theirs().setup) {
                Ok((offers, set, m)) => Progress::Agreed(Agreed { conn, offers, set, m }),
                Err(why) => Progress::Failed(why),
            },
        }
    }
}

/// A datagram channel in memory, for tests: two ends, each taking what the
/// other sends, in order, nothing lost.
#[cfg(test)]
fn memory_pair() -> (Memory, Memory) {
    use std::sync::{Arc, Mutex};
    let a = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    let b = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    (Memory { inbox: a.clone(), outbox: b.clone() }, Memory { inbox: b, outbox: a })
}

/// One end of [`memory_pair`].
#[cfg(test)]
struct Memory {
    inbox: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<Vec<u8>>>>,
    outbox: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<Vec<u8>>>>,
}

#[cfg(test)]
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
        Hello::new(role, "exe6", ContentHash(content), setup.to_vec(), content ^ role as u64)
    }

    /// A host offering `a` and a joiner offering `b`, polled in turn on
    /// this thread (neither waits): each has ended by its second poll.
    fn both(a: Hello, b: Hello) -> (Result<Connection<Memory>, HandshakeError>, Result<Connection<Memory>, HandshakeError>) {
        let (x, y) = memory_pair();
        let timeout = Duration::from_secs(5);
        let mut shakes = [Handshake::new(x, a, timeout), Handshake::new(y, b, timeout)];
        let mut ended = [None, None];
        let now = Instant::now();
        for _ in 0..2 {
            for (shake, ended) in shakes.iter_mut().zip(&mut ended) {
                if ended.is_none()
                    && let Poll::Ready(done) = shake.poll(now)
                {
                    *ended = Some(done);
                }
            }
        }
        let [h, j] = ended.map(|e| e.expect("a handshake still going after its second poll"));
        (h, j)
    }

    fn offer(content: &Arc<Content>, seed: u32) -> Offer {
        let side = nettai_match::Side::picked(content, "exe6", &mut nettai_match::Picks::new(seed)).unwrap();
        Offer::of_side("exe6", vec![nettai_match::RoundSettings::default(); nettai_match::TRIPLE_BATTLE], side)
    }

    /// Both players' handshakes on UDP on this machine, polled in turn on
    /// one thread (the window's frames): neither waits, and they agree the
    /// same match, each player's offer on their side; then a frame goes
    /// each way on the connection.
    #[test]
    fn a_udp_handshake_on_localhost_agrees_the_match() {
        let content = nettai_match::testing::exe6_content();
        let host = Udp::host_on("127.0.0.1:0").unwrap();
        let joiner = Udp::join(host.local_addr().unwrap()).unwrap();
        let timeout = Duration::from_secs(10);
        let mine = [offer(&content, 11), offer(&content, 22)];
        let mut shakes = [
            NetHandshake::new(Role::Host, host, &content, mine[0].clone(), timeout),
            NetHandshake::new(Role::Join, joiner, &content, mine[1].clone(), timeout),
        ];
        let mut agreed = [None, None];
        let start = Instant::now();
        while agreed.iter().any(Option::is_none) {
            assert!(start.elapsed() < timeout, "no agreement");
            for (shake, agreed) in shakes.iter_mut().zip(&mut agreed) {
                if agreed.is_none() {
                    match shake.poll(Instant::now()) {
                        Progress::Pending => {}
                        Progress::Agreed(a) => *agreed = Some(a),
                        Progress::Failed(why) => panic!("{why}"),
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let [mut h, mut j] = agreed.map(Option::unwrap);
        assert_eq!((h.conn.side(), j.conn.side()), (0, 1));
        assert_eq!(h.conn.seed(), j.conn.seed());
        assert_eq!(h.offers, mine);
        assert_eq!(j.offers, mine);
        assert_eq!(h.m, j.m);
        assert_eq!(format!("{:?}", h.set.first()), format!("{:?}", j.set.first()));
        // The library's channel: frames, each way.
        Channel::send(&mut h.conn, b"from the host").unwrap();
        Channel::send(&mut j.conn, b"from the joiner").unwrap();
        for (conn, from) in [(&mut j.conn, &b"from the host"[..]), (&mut h.conn, b"from the joiner")] {
            let start = Instant::now();
            loop {
                assert!(start.elapsed() < timeout, "no frame");
                if let Some(frame) = Channel::recv(conn).unwrap() {
                    assert_eq!(frame, from);
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    /// A refused Hello, either way, polled: the match fails with the
    /// handshake's messages (other content refused by this side, another
    /// protocol by the other).
    #[test]
    fn a_refused_match_says_why() {
        let content = nettai_match::testing::exe6_content();
        let (ours, theirs) = (content.hash(), ContentHash(content.hash().0 ^ 1));
        let timeout = Duration::from_secs(10);
        let now = Instant::now();
        // Other content, from a joiner: this host refuses it.
        let (x, y) = memory_pair();
        let mut host = NetHandshake::new(Role::Host, x, &content, offer(&content, 11), timeout);
        let mut join = Handshake::new(y, Hello::new(Role::Join, "exe6", theirs, Vec::new(), 1), timeout);
        assert!(matches!(host.poll(now), Progress::Pending));
        assert!(join.poll(now).is_pending());
        let Progress::Failed(why) = host.poll(now) else { panic!("not refused") };
        assert_eq!(why, format!("can't play: the other side plays other content (its hash {theirs}, this one's {ours}): both must play the same content"));
        // Another protocol, from a host: it refuses this joiner.
        let (x, y) = memory_pair();
        let mut other = Hello::new(Role::Host, "exe6", ours, Vec::new(), 1);
        other.protocol += 1;
        let mut host = Handshake::new(x, other, timeout);
        let mut join = NetHandshake::new(Role::Join, y, &content, offer(&content, 22), timeout);
        assert!(host.poll(now).is_pending());
        assert!(matches!(join.poll(now), Progress::Pending));
        assert!(host.poll(now).is_ready());
        let Progress::Failed(why) = join.poll(now) else { panic!("not refused") };
        let v = protocol::VERSION;
        assert_eq!(why, format!("the other side refused: the other side speaks netplay protocol {v}, this one {}", v + 1));
    }

    /// A joiner the host never answers says its Hello again every
    /// [`RESEND`], and gives up after the timeout.
    #[test]
    fn a_handshake_nobody_answers_times_out() {
        let (x, mut nobody) = memory_pair();
        let timeout = Duration::from_secs(10);
        let mut join = Handshake::new(x, hello(Role::Join, 7, b""), timeout);
        let t0 = Instant::now();
        assert!(join.poll(t0).is_pending());
        assert!(join.poll(t0 + RESEND / 2).is_pending());
        assert!(join.poll(t0 + timeout - Duration::from_millis(1)).is_pending());
        let Poll::Ready(Err(e)) = join.poll(t0 + timeout) else { panic!("no timeout") };
        assert_eq!(e.to_string(), "no answer from the other side");
        let mut buf = [0u8; 256];
        let mut hellos = 0;
        while let Some(n) = nobody.try_recv(&mut buf).unwrap() {
            assert_eq!(buf[..n][0], Kind::Hello as u8);
            hellos += 1;
        }
        assert_eq!(hellos, 2);
    }

    /// Another protocol: both sides stop, each saying why.
    #[test]
    fn another_protocol_is_refused_on_both_sides() {
        let mut joiner = hello(Role::Join, 7, b"");
        joiner.protocol += 1;
        let (h, j) = both(hello(Role::Host, 7, b""), joiner);
        let (h, j) = (h.err().unwrap().to_string(), j.err().unwrap().to_string());
        let v = protocol::VERSION;
        assert_eq!(h, format!("can't play: the other side speaks netplay protocol {}, this one {v}", v + 1));
        assert_eq!(j, format!("the other side refused: the other side speaks netplay protocol {}, this one {v}", v + 1));
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
        joiner.game = "exe5".into();
        let (h, j) = both(hello(Role::Host, 7, b""), joiner);
        let (h, j) = (h.err().unwrap().to_string(), j.err().unwrap().to_string());
        assert_eq!(h, "can't play: the other side plays exe5, this one exe6: a match is of one game, both sides playing it");
        assert_eq!(j, "the other side refused: the other side plays exe5, this one exe6: a match is of one game, both sides playing it");
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
