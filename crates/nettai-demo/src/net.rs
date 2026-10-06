//! Netplay's transport, the program's own (`--room`, `--host`, `--join`):
//! a WebRTC data channel to the other player (nettai-rtc's [`Link`]), and
//! the handshake that starts a match on it. The library has none: it plays
//! a match on any channel its host gives it (nettai-frontend's
//! `netplay::Channel`).
//!
//! [`Datagram`] is all the handshake needs from a transport: send a
//! datagram to the other peer, and take the next one that arrived, without
//! waiting; and this side's role, once known. Nothing is assumed of
//! delivery: datagrams may be lost, reordered or duplicated (rennet
//! recovers). A [`Link`] meets the other player in a room of the signaling
//! server, or directly (the host listens on a UDP port, the joiner dials
//! it: a LAN, or the Internet with the host's port forwarded); it makes the
//! connection again when it drops, and says how long it has been down,
//! which the match waits out.
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
use std::sync::Arc;
use std::time::{Duration, Instant};

use nettai_battle::Content;
use nettai_frontend::lobby::{Agreement, Lobby, Settings, Status};
pub use nettai_frontend::lobby::{Kind, RESEND, Role};
use nettai_frontend::netplay::Channel;
use nettai_match::Side;
pub use nettai_rtc::Link;

/// A datagram channel to the other peer.
pub trait Datagram {
    /// Send one datagram. Never waits; a datagram that can't go now may be
    /// dropped, as the network may drop it.
    fn send(&mut self, datagram: &[u8]) -> io::Result<()>;

    /// The next datagram that has arrived into `buf`, and its length; none
    /// if nothing has. Never waits.
    fn try_recv(&mut self, buf: &mut [u8]) -> io::Result<Option<usize>>;

    /// This side's role, once it is known (a room's server says which as
    /// the player comes in).
    fn role(&self) -> Option<Role>;

    /// While the channel is down and being made again, how long it has been
    /// (`netplay::Channel::down_for`).
    fn down_for(&self) -> Option<Duration> {
        None
    }

    /// What keeps it from connecting, if it knows.
    fn problem(&self) -> Option<String> {
        None
    }

    /// The other player, as far as this side knows: a room, an address.
    fn describe(&self) -> String;
}

/// The largest datagram a peer sends or takes: a horizon of one-byte ticks
/// and a frame's header with room to spare.
pub const MAX_DATAGRAM: usize = 64 * 1024;

/// What a WebRTC link's error is to the handshake and the match.
fn io_error(e: nettai_rtc::Error) -> io::Error {
    io::Error::other(e.0)
}

impl Datagram for Link {
    fn send(&mut self, datagram: &[u8]) -> io::Result<()> {
        Link::send(self, datagram).map_err(io_error)
    }

    fn try_recv(&mut self, buf: &mut [u8]) -> io::Result<Option<usize>> {
        let Some(d) = Link::recv(self).map_err(io_error)? else { return Ok(None) };
        let n = d.len().min(buf.len());
        buf[..n].copy_from_slice(&d[..n]);
        Ok(Some(n))
    }

    fn role(&self) -> Option<Role> {
        Link::role(self).map(|r| match r {
            nettai_rtc::Role::Host => Role::Host,
            nettai_rtc::Role::Join => Role::Join,
        })
    }

    fn down_for(&self) -> Option<Duration> {
        Link::down_for(self)
    }

    fn problem(&self) -> Option<String> {
        Link::problem(self)
    }

    fn describe(&self) -> String {
        Link::describe(self)
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

    fn down_for(&self) -> Option<Duration> {
        self.datagram.down_for()
    }
}

/// How long a handshake waits for the other player: a host for a joiner, a
/// joiner for the host; and for its role, the joiner's (a room's server
/// that doesn't let it in).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Waits {
    pub host: Duration,
    pub join: Duration,
}

/// A netplay match being agreed, which never waits: the window polls it
/// each frame (and stays responsive) until the match is agreed or can't be.
/// It is the library's lobby and handshake over the datagrams, this side
/// proposing its match file's settings, ready; settings of the other's that
/// differ stop it, saying what differs (the host isn't the only one that
/// picks: both players' files state the match). The lobby is made once the
/// datagrams know this side's role.
pub struct NetHandshake<D: Datagram> {
    datagram: Option<D>,
    lobby: Option<Lobby>,
    /// What the lobby proposes once it is made.
    proposal: Option<(Settings, Side)>,
    waits: Waits,
    start: Option<Instant>,
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
    /// A handshake on `datagram` for a match on `content`, proposing
    /// `settings` (ready) with this player's `side`; it fails if nothing
    /// comes from the other side as long as `waits` says for its role.
    pub fn new(datagram: D, content: &Arc<Content>, settings: Settings, side: Side, waits: Waits) -> NetHandshake<D> {
        NetHandshake { datagram: Some(datagram), lobby: None, proposal: Some((settings, side)), waits, start: None, content: content.clone(), buf: vec![0u8; MAX_DATAGRAM] }
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
                for d in self.lobby.as_mut().map(Lobby::outgoing).unwrap_or_default() {
                    let _ = datagram.send(&d);
                }
                Progress::Failed(why)
            }
        }
    }

    /// Make the lobby once the role is known: whether it is made. Until
    /// then the datagrams are polled (which, in a room, is what brings the
    /// role), for as long as a joiner waits.
    fn make_lobby(&mut self, datagram: &mut D, now: Instant) -> Result<bool, String> {
        let start = *self.start.get_or_insert(now);
        if self.lobby.is_some() {
            return Ok(true);
        }
        let Some(role) = datagram.role() else {
            datagram.try_recv(&mut self.buf).map_err(|e| format!("network error: {e}"))?;
            if now.duration_since(start) >= self.waits.join {
                return Err(match datagram.problem() {
                    Some(why) => format!("no way to the other player: {why}"),
                    None => "no answer from the signaling server".into(),
                });
            }
            return Ok(false);
        };
        let (settings, side) = self.proposal.take().expect("the proposal");
        let timeout = if role == Role::Host { self.waits.host } else { self.waits.join };
        let mut lobby = Lobby::new(role, &self.content, settings, side, nettai_frontend::lobby::entropy(), timeout);
        lobby.set_ready(true);
        self.lobby = Some(lobby);
        Ok(true)
    }

    fn step(&mut self, datagram: &mut D, now: Instant) -> Result<Option<Agreement>, String> {
        if !self.make_lobby(datagram, now)? {
            return Ok(None);
        }
        let lobby = self.lobby.as_mut().expect("made");
        while let Some(n) = datagram.try_recv(&mut self.buf).map_err(|e| format!("network error: {e}"))? {
            lobby.receive(now, &self.buf[..n]);
        }
        // This program's policy: the settings both files state, or none.
        match lobby.theirs() {
            Some(Ok(theirs)) if theirs != lobby.mine() => {
                let differs = lobby.mine().differences(&self.content, theirs).join("; ");
                lobby.refuse(&format!("the two players' matches differ ({differs})"));
            }
            Some(Err(why)) => {
                let why = format!("the other player's match can't be played here ({why})");
                lobby.refuse(&why);
            }
            _ => {}
        }
        let done = match lobby.poll(now) {
            Status::Pending => None,
            Status::Agreed(a) => Some(Ok(a.clone())),
            Status::Failed(why) => Some(Err(why.to_string())),
        };
        for d in lobby.outgoing() {
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
    (Memory { inbox: a.clone(), outbox: b.clone(), role: Role::Host }, Memory { inbox: b, outbox: a, role: Role::Join })
}

/// One end of [`memory_pair`].
#[cfg(test)]
struct Memory {
    inbox: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<Vec<u8>>>>,
    outbox: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<Vec<u8>>>>,
    role: Role,
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

    fn role(&self) -> Option<Role> {
        Some(self.role)
    }

    fn describe(&self) -> String {
        "memory".into()
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

    fn waits(both: Duration) -> Waits {
        Waits { host: both, join: both }
    }

    /// Links that find each other on this machine, with no STUN server.
    fn config() -> nettai_rtc::Config {
        nettai_rtc::Config { ice_servers: Vec::new(), loopback: true, ..nettai_rtc::Config::default() }
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

    /// Both players' handshakes on `links` (the host's, then the joiner's),
    /// polled in turn on one thread (the window's frames): neither waits,
    /// and they agree the same match, each player's side on their side;
    /// then a frame goes each way on the connection.
    fn agree_on(links: [Link; 2]) {
        let content = nettai_match::testing::exe6_content();
        let timeout = Duration::from_secs(20);
        let sides = [side(&content, 11), side(&content, 22)];
        let [host, joiner] = links;
        let shakes = [NetHandshake::new(host, &content, triple(), sides[0].clone(), waits(timeout)), NetHandshake::new(joiner, &content, triple(), sides[1].clone(), waits(timeout))];
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
            assert_eq!(Channel::down_for(conn), None);
        }
    }

    /// Direct connect on this machine: the host on a port, the joiner
    /// dialing it.
    #[test]
    fn a_direct_handshake_on_localhost_agrees_the_match() {
        let host = Link::host(0, config()).unwrap();
        let joiner = Link::join(&format!("127.0.0.1:{}", host.port().unwrap()), config()).unwrap();
        agree_on([host, joiner]);
    }

    /// A room of a signaling server (in process): the first in hosts.
    #[test]
    fn a_room_handshake_agrees_the_match() {
        let server = nettai_rtc::testing::Server::start();
        let mut host = Link::room(&server.url(), "handshake", config()).unwrap();
        let start = Instant::now();
        while host.role().is_none() {
            assert!(start.elapsed() < Duration::from_secs(10), "not let in");
            host.poll().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        let joiner = Link::room(&server.url(), "handshake", config()).unwrap();
        agree_on([host, joiner]);
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
        let shakes = [NetHandshake::new(x, &content, triple(), side(&content, 11), waits(timeout)), NetHandshake::new(y, &content, five, side(&content, 22), waits(timeout))];
        let [Progress::Failed(h), Progress::Failed(j)] = both(shakes, timeout) else { panic!("agreed") };
        let said = [h, j];
        assert!(said.contains(&"can't play: the two players' matches differ (the rounds: 5 here, 3 there)".to_string()) || said.contains(&"can't play: the two players' matches differ (the rounds: 3 here, 5 there)".to_string()), "{said:?}");
        assert!(said.iter().any(|w| w.starts_with("the other side refused: the two players' matches differ")), "{said:?}");
    }

    /// A joiner the host never answers gives up after the timeout.
    #[test]
    fn a_handshake_nobody_answers_times_out() {
        let content = nettai_match::testing::exe6_content();
        let (_nobody, x) = memory_pair();
        let timeout = Duration::from_secs(10);
        let mut join = NetHandshake::new(x, &content, triple(), side(&content, 3), Waits { host: timeout * 10, join: timeout });
        let t0 = Instant::now();
        assert!(matches!(join.poll(t0), Progress::Pending));
        assert!(matches!(join.poll(t0 + timeout - Duration::from_millis(1)), Progress::Pending));
        let Progress::Failed(why) = join.poll(t0 + timeout) else { panic!("no timeout") };
        assert_eq!(why, "no answer from the other side");
    }

    /// A room whose server can't be reached: the handshake gives up after
    /// the joiner's wait, saying why.
    #[test]
    fn a_room_nobody_runs_gives_up() {
        let content = nettai_match::testing::exe6_content();
        // (A port nothing listens on: bound, then let go.)
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let link = Link::room(&format!("ws://127.0.0.1:{port}"), "nobody", config()).unwrap();
        let timeout = Duration::from_secs(2);
        let mut shake = NetHandshake::new(link, &content, triple(), side(&content, 3), waits(timeout));
        let t0 = Instant::now();
        let why = loop {
            match shake.poll(Instant::now()) {
                Progress::Pending => assert!(t0.elapsed() < timeout * 3, "no end"),
                Progress::Failed(why) => break why,
                Progress::Agreed(_) => panic!("agreed with nobody"),
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(why.starts_with("no way to the other player: can't reach the signaling server"), "{why}");
    }
}
