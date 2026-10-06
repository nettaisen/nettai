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
//! What the peers say before the match is the library's lobby and
//! handshake (`nettai_frontend::lobby`, sans IO): the settings agreed, then
//! each side's commitment and its Reveal. [`NetHandshake`] runs it over the
//! datagrams, which the window polls each frame (it never waits), with this
//! program's policy: each side proposes its match file's settings (the game
//! and the rounds) and is ready; settings that differ stop it, saying what
//! differs. The [`Connection`] it makes is the channel the library's
//! `NetPlayer` plays on.

use std::fmt;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nettai_battle::Content;
use nettai_frontend::lobby::{Agreement, Lobby, Settings, Status};
pub use nettai_frontend::lobby::{Kind, RESEND, Role};
use nettai_frontend::netplay::Channel;
use nettai_match::Side;

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
    /// first lobby datagram ([`Kind::Lobby`]) that arrives names the other
    /// peer.
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
                    // The first lobby datagram names the joiner; anything else
                    // before it is ignored.
                    if n > 0 && buf[0] == Kind::Lobby as u8 {
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

/// Why a match's channel ended.
#[derive(Debug)]
pub enum ChannelError {
    Io(io::Error),
    /// The other side refused, for this reason.
    Refused(String),
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChannelError::Io(e) => write!(f, "network error: {e}"),
            ChannelError::Refused(why) => write!(f, "the other side refused: {why}"),
        }
    }
}

impl From<io::Error> for ChannelError {
    fn from(e: io::Error) -> ChannelError {
        ChannelError::Io(e)
    }
}

/// A match's channel after the handshake: the frames, and this side's
/// Reveal, which answers a Hello or a Reveal that comes again (the other
/// side hasn't this one's yet).
pub struct Connection<D: Datagram> {
    datagram: D,
    answer: Vec<u8>,
    buf: Vec<u8>,
}

impl<D: Datagram> Connection<D> {
    /// Send a frame of the protocol.
    pub fn send_frame(&mut self, frame: &[u8]) -> io::Result<()> {
        let mut out = Vec::with_capacity(frame.len() + 1);
        out.push(Kind::Frame as u8);
        out.extend_from_slice(frame);
        self.datagram.send(&out)
    }

    /// The next frame that arrived, if any, without waiting. A Hello or a
    /// Reveal that comes again is answered; a refusal ends the match.
    pub fn try_recv_frame(&mut self) -> Result<Option<&[u8]>, ChannelError> {
        loop {
            let Some(n) = self.datagram.try_recv(&mut self.buf)? else { return Ok(None) };
            match self.buf[..n].split_first().map(|(&k, body)| (Kind::of(k), body)) {
                Some((Some(Kind::Frame), _)) => return Ok(Some(&self.buf[1..n])),
                Some((Some(Kind::Hello | Kind::Reveal), _)) => self.datagram.send(&self.answer)?,
                Some((Some(Kind::Refuse), body)) => return Err(ChannelError::Refused(String::from_utf8_lossy(body).into_owned())),
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

/// A netplay match being agreed, which never waits: the window polls it
/// each frame (and stays responsive) until the match is agreed or can't be.
/// It is the library's lobby and handshake over the datagrams, this side
/// proposing its match file's settings, ready; settings of the other's that
/// differ stop it, saying what differs (the host isn't the only one that
/// picks: both players' files state the match).
pub struct NetHandshake<D: Datagram> {
    datagram: Option<D>,
    lobby: Lobby,
    content: Arc<Content>,
    buf: Vec<u8>,
}

/// How a [`NetHandshake`] stands after a poll.
pub enum Progress<D: Datagram> {
    /// Still going: poll it again.
    Pending,
    /// The match is agreed.
    Agreed(Agreed<D>),
    /// No match, and why (the timeout, a refusal either way, settings that
    /// differ, the other player's side refused, a network error).
    Failed(String),
}

/// An agreed match: the channel to play it on and what is played
/// (`netplay::NetPlayer::new`'s: the connection, its side and the set).
pub struct Agreed<D: Datagram> {
    pub conn: Connection<D>,
    pub agreement: Agreement,
}

impl<D: Datagram> NetHandshake<D> {
    /// A handshake on `datagram` in `role` for a match on `content`,
    /// proposing `settings` (ready) with this player's `side`; it fails if
    /// nothing comes from the other side for `timeout`.
    pub fn new(role: Role, datagram: D, content: &Arc<Content>, settings: Settings, side: Side, timeout: Duration) -> NetHandshake<D> {
        let mut lobby = Lobby::new(role, content, settings, side, nettai_frontend::lobby::entropy(), timeout);
        lobby.set_ready(true);
        NetHandshake { datagram: Some(datagram), lobby, content: content.clone(), buf: vec![0u8; MAX_DATAGRAM] }
    }

    /// Step the handshake at `now`, without waiting; polling it after it
    /// ended panics.
    pub fn poll(&mut self, now: Instant) -> Progress<D> {
        let mut datagram = self.datagram.take().expect("a handshake polled after it ended");
        let step = self.step(&mut datagram, now);
        match step {
            Ok(None) => {
                self.datagram = Some(datagram);
                Progress::Pending
            }
            Ok(Some(agreement)) => {
                let buf = std::mem::take(&mut self.buf);
                Progress::Agreed(Agreed { conn: Connection { datagram, answer: agreement.answer.clone(), buf }, agreement })
            }
            Err(why) => {
                // (What the lobby says last goes: a refusal.)
                for d in self.lobby.outgoing() {
                    let _ = datagram.send(&d);
                }
                Progress::Failed(why)
            }
        }
    }

    fn step(&mut self, datagram: &mut D, now: Instant) -> Result<Option<Agreement>, String> {
        while let Some(n) = datagram.try_recv(&mut self.buf).map_err(|e| format!("network error: {e}"))? {
            self.lobby.receive(now, &self.buf[..n]);
        }
        // This program's policy: the settings both files state, or none.
        match self.lobby.theirs() {
            Some(Ok(theirs)) if theirs != self.lobby.mine() => {
                let differs = self.lobby.mine().differences(&self.content, theirs).join("; ");
                self.lobby.refuse(&format!("the two players' matches differ ({differs})"));
            }
            Some(Err(why)) => {
                let why = format!("the other player's match can't be played here ({why})");
                self.lobby.refuse(&why);
            }
            _ => {}
        }
        let done = match self.lobby.poll(now) {
            Status::Pending => None,
            Status::Agreed(a) => Some(Ok(a.clone())),
            Status::Failed(why) => Some(Err(why.to_string())),
        };
        for d in self.lobby.outgoing() {
            datagram.send(&d).map_err(|e| format!("network error: {e}"))?;
        }
        done.transpose()
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
    use nettai_match::{Picks, RoundSettings, TRIPLE_BATTLE};

    fn side(content: &Arc<Content>, seed: u32) -> Side {
        Side::picked(content, "exe6", &mut Picks::new(seed)).unwrap()
    }

    fn triple() -> Settings {
        Settings { game: "exe6".into(), rounds: vec![RoundSettings::default(); TRIPLE_BATTLE] }
    }

    /// Two handshakes polled in turn on this thread (neither waits), until
    /// both have ended or `timeout`.
    fn both<D: Datagram>(mut shakes: [NetHandshake<D>; 2], timeout: Duration) -> [Progress<D>; 2] {
        let mut ended: [Option<Progress<D>>; 2] = [None, None];
        let start = Instant::now();
        while ended.iter().any(Option::is_none) {
            assert!(start.elapsed() < timeout, "no end");
            for (shake, ended) in shakes.iter_mut().zip(&mut ended) {
                if ended.is_none() {
                    match shake.poll(Instant::now()) {
                        Progress::Pending => {}
                        done => *ended = Some(done),
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        ended.map(Option::unwrap)
    }

    /// Both players' handshakes on UDP on this machine, polled in turn on
    /// one thread (the window's frames): neither waits, and they agree the
    /// same match, each player's side on their side; then a frame goes
    /// each way on the connection.
    #[test]
    fn a_udp_handshake_on_localhost_agrees_the_match() {
        let content = nettai_match::testing::exe6_content();
        let host = Udp::host_on("127.0.0.1:0").unwrap();
        let joiner = Udp::join(host.local_addr().unwrap()).unwrap();
        let timeout = Duration::from_secs(10);
        let sides = [side(&content, 11), side(&content, 22)];
        let shakes = [
            NetHandshake::new(Role::Host, host, &content, triple(), sides[0].clone(), timeout),
            NetHandshake::new(Role::Join, joiner, &content, triple(), sides[1].clone(), timeout),
        ];
        let [Progress::Agreed(mut h), Progress::Agreed(mut j)] = both(shakes, timeout) else { panic!("not agreed") };
        assert_eq!((h.agreement.side, j.agreement.side), (0, 1));
        assert_eq!(h.agreement.seed, j.agreement.seed);
        assert_eq!((&h.agreement.sides, &j.agreement.sides), (&sides, &sides));
        assert_eq!(h.agreement.m, j.agreement.m);
        assert_eq!(format!("{:?}", h.agreement.set.first()), format!("{:?}", j.agreement.set.first()));
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

    /// Two match files that state other rounds: no match, both sides saying
    /// what differs (the one that saw it first, and the other that heard
    /// it).
    #[test]
    fn matches_that_differ_say_what_differs() {
        let content = nettai_match::testing::exe6_content();
        let (x, y) = memory_pair();
        let timeout = Duration::from_secs(10);
        let mut five = triple();
        five.rounds.resize(5, RoundSettings::default());
        let shakes = [
            NetHandshake::new(Role::Host, x, &content, triple(), side(&content, 11), timeout),
            NetHandshake::new(Role::Join, y, &content, five, side(&content, 22), timeout),
        ];
        let [Progress::Failed(h), Progress::Failed(j)] = both(shakes, timeout) else { panic!("agreed") };
        let said = [h, j];
        assert!(said.contains(&"can't play: the two players' matches differ (the rounds: 5 here, 3 there)".to_string()) || said.contains(&"can't play: the two players' matches differ (the rounds: 3 here, 5 there)".to_string()), "{said:?}");
        assert!(said.iter().any(|w| w.starts_with("the other side refused: the two players' matches differ")), "{said:?}");
    }

    /// A joiner the host never answers gives up after the timeout.
    #[test]
    fn a_handshake_nobody_answers_times_out() {
        let content = nettai_match::testing::exe6_content();
        let (x, _nobody) = memory_pair();
        let timeout = Duration::from_secs(10);
        let mut join = NetHandshake::new(Role::Join, x, &content, triple(), side(&content, 3), timeout);
        let t0 = Instant::now();
        assert!(matches!(join.poll(t0), Progress::Pending));
        assert!(matches!(join.poll(t0 + timeout - Duration::from_millis(1)), Progress::Pending));
        let Progress::Failed(why) = join.poll(t0 + timeout) else { panic!("no timeout") };
        assert_eq!(why, "no answer from the other side");
    }
}
