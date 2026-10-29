//! The link: the channel a match is played on, which keeps a WebRTC
//! connection to the other player up (docs/design/rollback.md §4.8).
//!
//! The joiner dials and the host answers, through the signaling server and
//! directly alike. Every connection is a generation: the joiner's first dial
//! is 1, each dial after it one more, and what is said of an older one (an
//! answer, a candidate) is dropped. A dropped connection isn't mended (no ICE
//! restart): the joiner dials a new one, the next generation, which works
//! the same on both backends and directly, and the game's channel has
//! nothing in it worth keeping (what was lost on it, the protocol sends
//! again).
//!
//! **The signaling server only while it is needed.** The user's request:
//! "the signaling server session should last just long enough for the
//! peers to reach each other, so it doesn't have to hold onto a connection
//! longer than required. but it should also support reconnect somehow,
//! maybe by some special lobby code that the two players agree on for each
//! reconnect?" So a link is in a room of the server only until its
//! connection is through, and again while a dropped one is made anew:
//!
//! - Over each connection, once it is open, the two agree on the code of
//!   the room they will meet in if it drops (a reconnect room,
//!   [`crate::signal`]): the host draws [`signal::RECONNECT_CODE_BYTES`]
//!   bytes from the system's cryptographic randomness and says them, every
//!   100 ms until the joiner acknowledges them. The channel is DTLS's, so
//!   the server first sees the code when it is used. The link's own
//!   messages (`code HEX` and `ack HEX`) go on a channel of their own
//!   beside the game's (`crate::peer`), so they cost the game's frames no
//!   byte.
//! - The connection is through, for the joiner, when it has the code: it
//!   has heard the host on an open channel. For the host it is when the
//!   acknowledgment comes: a datagram has gone each way. Each side then
//!   gives its place in the room up and closes its socket. A room both
//!   have left is forgotten by the server at once, and the invite's code
//!   with it. Between drops neither side has a socket to the server.
//! - On a drop, each side goes to the room of the code agreed last (before
//!   any, the invite room, which neither has left), where the joiner dials
//!   as below. A code agreed is spent once the next one is: until then it
//!   is where the two meet, the room made again if it was forgotten. A
//!   host whose newest code isn't acknowledged doesn't know whether the
//!   joiner has it (the acknowledgment may be what was lost), so it waits
//!   in that code's room too, and keeps the one the joiner's offer comes
//!   through. The joiner always knows its one code.
//!
//! **A drop** is any of: the connection's state (disconnected, failed,
//! closed), its data channel closing, or silence: nothing from the other
//! side for [`Config::silence`] while it is open. Then:
//!
//! - the joiner dials again at once, and again after 0.5, 1, 2, then every
//!   3 seconds while dials don't open (each given [`Config::dial_timeout`]);
//!   through the server, also when the host says it lost the connection, or
//!   when the host comes into the room while it isn't connected;
//! - the host waits for the next generation: an offer in the room (which it
//!   asks for every 2 seconds), or the joiner's connectivity checks
//!   directly (once its connection is over, which has the port till then:
//!   `crate::native::direct`; one that doesn't open within
//!   [`Config::dial_timeout`] is given up).
//!
//! **STUN and TURN** in a room are the server's: once both players are in
//! the room, it gives each the servers its connections are made on
//! (`crate::signal`), in the welcome or, for the one that waited, in the
//! peer message as the other comes, and the latest given are what the next
//! connection uses, so a reconnection has the credentials its reconnect
//! room gave. The backend resolves their names itself, without waiting
//! (natively on the connections' thread, the browser its own way). The
//! servers come before anything the other side sends, so neither side
//! starts ICE on servers it hasn't had. A direct link uses none.
//!
//! A reconnect room's socket that doesn't open, or drops, is made again:
//! the server being out of reach is no reason to stop sooner than the
//! reconnection's time.
//!
//! What a connection had taken before it dropped is still the link's to
//! hand over ([`Link::recv`]): a player who leaves says so in their last
//! datagrams, which come with the close that follows them.
//!
//! While it is down, what is sent is dropped and nothing new comes, and
//! [`Link::down_for`] says how long it has been ([`Link::gives_up_in`], how
//! long it has left); after [`Config::reconnect_timeout`] the link gives
//! up ([`ErrorKind::ConnectionLost`]), and every call after errs the same.
//! Before it is first open nothing is down: it is still connecting, in the
//! invite room. How long to wait for the other player to come is the
//! caller's to say (a host waits on its invite as long as its player likes,
//! and the server keeps the room); once they are there (in the room, or
//! dialed directly) and no connection opens in [`Config::connect_timeout`],
//! the link gives up ([`ErrorKind::CantConnect`]).
//!
//! **Errors** are [`Error`]s: a kind a player can be told, and a detail for
//! a log. The first one a link meets is the one every later call returns.

// (On the web the signaling server is the one way to meet.)
#![cfg_attr(target_arch = "wasm32", allow(irrefutable_let_patterns))]

use std::collections::VecDeque;

use crate::room::{Room, RoomEvent};
use crate::signal::{self, Signal};
use crate::{Config, Duration, Error, ErrorKind, IceServer, Instant, Message, PeerConnection, PeerEvent, Role};

#[cfg(not(target_arch = "wasm32"))]
use crate::native::direct::{Listener, MAX_DATAGRAM};

/// How often a host in a room asks the joiner to dial again while it is
/// down.
const ASK_AGAIN: Duration = Duration::from_secs(2);
/// The longest wait between two dials.
const MAX_BACKOFF: Duration = Duration::from_secs(3);
/// Nothing from the other side this long, and [`Link::send`] sends one
/// datagram every [`QUIET_EVERY`] until something comes. What it holds back
/// would only go into a path that is out: SCTP keeps each unacknowledged
/// datagram in its congestion window, and a window full of them holds what
/// is sent after back until its retransmission timer, backed off to seconds,
/// fires, so a 1.5 s outage froze a match for 1.6 s more. One every 100 ms
/// leaves the window room, and what is sent once the path is back goes at
/// once (nettai-verify tools/dcbench, `--quiet 100,100`). Netplay loses
/// nothing by it: each datagram carries every input not yet acknowledged;
/// the lobby says its part again every 100 ms anyway.
const QUIET_AFTER: Duration = Duration::from_millis(100);
const QUIET_EVERY: Duration = Duration::from_millis(100);
/// How often the host says the next reconnect code until it is
/// acknowledged.
const SAY_AGAIN: Duration = Duration::from_millis(100);

/// The link's own messages, as text on its channel: the host's next
/// reconnect code, and the joiner's acknowledgment of it (each followed by
/// the code).
const CODE: &str = "code ";
const ACK: &str = "ack ";

/// How the two players meet.
enum Way {
    /// Through the signaling server.
    Server(Signaling),
    /// Directly: the host listens.
    #[cfg(not(target_arch = "wasm32"))]
    Listen(Listener),
    /// Directly: the joiner dials the host's address, as `nonce`.
    #[cfg(not(target_arch = "wasm32"))]
    Dial { host: std::net::SocketAddr, nonce: u32 },
}

/// A room of the signaling server.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Place {
    /// The invite room.
    Invite,
    /// A reconnect room, by the code the two agreed on.
    Reconnect(String),
}

/// Meeting through the signaling server (the module's docs).
struct Signaling {
    server: String,
    peer: String,
    server_timeout: Duration,
    /// The invite's code, once the server has said it (the room may have
    /// been left since).
    invite: Option<String>,
    /// The rooms this side is in: the invite room until the first
    /// connection is through; none while a connection is through; while it
    /// is down, the room of the code agreed last (and a host's of the code
    /// it said and wasn't told it has).
    rooms: Vec<(Place, Room)>,
    /// Where the two meet after a drop: the invite room until a code is
    /// agreed, then the code agreed last.
    agreed: Place,
    /// The host: the code it said on this connection, not yet acknowledged,
    /// and when it last said it.
    said: Option<(String, Option<Instant>)>,
    /// (Tests) the host says no code, the joiner acknowledges none.
    hold: bool,
}

impl Signaling {
    fn new(server: &str, peer: &str, server_timeout: Duration, invite: Room) -> Signaling {
        Signaling {
            server: server.to_string(),
            peer: peer.to_string(),
            server_timeout,
            invite: invite.code().map(str::to_string),
            rooms: vec![(Place::Invite, invite)],
            agreed: Place::Invite,
            said: None,
            hold: false,
        }
    }

    /// The other is in a room this side is in.
    fn there(&self) -> bool {
        self.rooms.iter().any(|(_, r)| r.there())
    }

    /// Say `signal` in every room this side is in (the other is in one at
    /// most).
    fn send(&mut self, signal: Signal) {
        for (_, room) in &mut self.rooms {
            room.send(signal.clone());
        }
    }

    /// While the connection is down: be in every room the other may come
    /// to.
    fn enter(&mut self) -> Result<(), Error> {
        let mut places = vec![self.agreed.clone()];
        if let Some((code, _)) = &self.said {
            places.push(Place::Reconnect(code.clone()));
        }
        for place in places {
            if self.rooms.iter().any(|(p, _)| *p == place) {
                continue;
            }
            let room = match &place {
                // (Not left before a code is agreed; this is for a room
                // whose socket was given up.)
                Place::Invite => match &self.invite {
                    Some(code) => Room::join(&self.server, code, &self.peer, self.server_timeout)?,
                    None => continue,
                },
                Place::Reconnect(code) => Room::reconnect(&self.server, code, &self.peer),
            };
            self.rooms.push((place, room));
        }
        Ok(())
    }

    /// The joiner's offer came through the room at `place`: that is where
    /// the two meet, the other rooms are left, and a code said but not
    /// acknowledged is no longer the host's to wait for (the joiner has it
    /// if, and only if, it came through that code's room).
    fn met(&mut self, place: &Place) {
        self.rooms.retain(|(p, _)| p == place);
        self.agreed = place.clone();
        self.said = None;
    }

    /// The connection is through, and `code` agreed: until the next drop,
    /// no room is needed.
    fn through(&mut self, code: &str) {
        self.agreed = Place::Reconnect(code.to_string());
        self.said = None;
        self.rooms.clear();
    }
}

/// `bytes` in lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The channel to the other player (the module's docs).
pub struct Link {
    config: Config,
    /// The STUN and TURN servers its connections are made on: in a room,
    /// those the latest welcome gave (or, if it gave none, the config's;
    /// none until the first); directly, none.
    servers: Vec<IceServer>,
    way: Way,
    role: Option<Role>,
    /// The connection, if there is one, and its generation.
    pc: Option<PeerConnection>,
    generation: u32,
    /// Its channel is open.
    open: bool,
    /// A channel has been open.
    ever_open: bool,
    /// Since when the other player is there to connect to, before the
    /// first connection opens.
    meeting: Option<Instant>,
    /// When the open channel last heard from the other side.
    heard: Instant,
    /// When it last sent a datagram of the game's.
    sent: Option<Instant>,
    /// Since when it is down, and why (it was open before).
    down: Option<(Instant, String)>,
    /// The joiner: when to dial next, the wait after that, and when the
    /// dial that isn't open yet began (a direct host: when it answered the
    /// dial that isn't open yet).
    next_dial: Option<Instant>,
    backoff: Duration,
    dialed: Option<Instant>,
    /// A host in a room: when it last asked for a new dial.
    asked: Option<Instant>,
    failed: Option<Error>,
    /// The datagrams that came and the caller hasn't taken, those of a
    /// connection that has since dropped first.
    inbox: VecDeque<Vec<u8>>,
    taken: Vec<u8>,
    #[cfg(not(target_arch = "wasm32"))]
    buf: Vec<u8>,
    /// Drop everything sent and taken until then (tests).
    outage: Option<Instant>,
}

impl Link {
    fn new(config: Config, way: Way, role: Option<Role>) -> Link {
        let now = Instant::now();
        Link {
            config,
            servers: Vec::new(),
            way,
            role,
            pc: None,
            generation: 0,
            open: false,
            ever_open: false,
            meeting: None,
            heard: now,
            sent: None,
            down: None,
            next_dial: Some(now),
            backoff: Duration::ZERO,
            dialed: None,
            asked: None,
            failed: None,
            inbox: VecDeque::new(),
            taken: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            buf: vec![0; MAX_DATAGRAM],
            outage: None,
        }
    }

    /// Ask the signaling server at `server` (`wss://...`) for a new room,
    /// which this player hosts: an invite. The server picks the room's code
    /// and says it as it lets this player in ([`Link::code`], none until
    /// then), for the other player to join by ([`Link::join_room`]). The
    /// STUN and TURN servers come with the server's welcome: nothing here
    /// waits.
    pub fn create_room(server: &str, config: Config) -> Link {
        let peer = format!("{:016x}", crate::random_u64());
        let room = Room::create(server, &peer, config.server_timeout);
        let way = Way::Server(Signaling::new(server, &peer, config.server_timeout, room));
        Link::new(config, way, None)
    }

    /// Join the room with `code` on the signaling server at `server`
    /// (`wss://...`): the code a host was given by the server and gave this
    /// player, as typed ([`crate::signal::normalize_code`]). No room is made
    /// of a code no room has: that errs ([`ErrorKind::NoSuchRoom`]), here
    /// for a code that can't be one, else when the server says so. Nothing
    /// here waits: the STUN and TURN servers come with the welcome.
    pub fn join_room(server: &str, code: &str, config: Config) -> Result<Link, Error> {
        let peer = format!("{:016x}", crate::random_u64());
        let room = Room::join(server, code, &peer, config.server_timeout)?;
        let way = Way::Server(Signaling::new(server, &peer, config.server_timeout, room));
        Ok(Link::new(config, way, None))
    }

    /// Host directly: listen on UDP `port` of every IPv4 interface (0: one
    /// the system picks, [`Link::port`]) for the joiner. A port that can't
    /// be listened on errs ([`ErrorKind::PortUnavailable`]). A direct link
    /// asks no STUN or TURN server: it says no candidate to anyone.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn host(port: u16, config: Config) -> Result<Link, Error> {
        let listener = Listener::bind(port)?;
        Ok(Link::new(config, Way::Listen(listener), Some(Role::Host)))
    }

    /// Join the host at `addr` (`host:port`) directly (resolving its name
    /// now, which may wait). An address that names no machine errs
    /// ([`ErrorKind::BadAddress`]).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn join(addr: &str, config: Config) -> Result<Link, Error> {
        use std::net::ToSocketAddrs;
        let host = addr
            .to_socket_addrs()
            .map_err(|e| Error::new(ErrorKind::BadAddress, format!("can't resolve {addr}: {e}")))?
            .min_by_key(|a| !a.is_ipv4())
            .ok_or_else(|| Error::new(ErrorKind::BadAddress, format!("{addr} resolves to nothing")))?;
        let nonce = crate::random_u64() as u32;
        Ok(Link::new(config, Way::Dial { host, nonce }, Some(Role::Join)))
    }

    /// This side's role: known at once directly, and from the server's
    /// welcome in a room.
    pub fn role(&self) -> Option<Role> {
        self.role
    }

    /// The direct host's port.
    pub fn port(&self) -> Option<u16> {
        match &self.way {
            #[cfg(not(target_arch = "wasm32"))]
            Way::Listen(l) => Some(l.port()),
            _ => None,
        }
    }

    /// The invite's code: the one joined by, or the one the server gave the
    /// room this player asked it to make, once it has (with its welcome,
    /// when [`Link::role`] is known). It is still said once the link has
    /// left the room (the connection through, the room is forgotten). None
    /// on a direct link.
    pub fn code(&self) -> Option<&str> {
        match &self.way {
            Way::Server(s) => s.invite.as_deref(),
            #[cfg(not(target_arch = "wasm32"))]
            _ => None,
        }
    }

    /// Who the other player is, as far as this side knows, for a log: the
    /// invite's room, or the address.
    pub fn describe(&self) -> String {
        match &self.way {
            Way::Server(s) => s.invite.as_ref().map_or_else(|| "a new room".to_string(), |code| format!("room {code}")),
            #[cfg(not(target_arch = "wasm32"))]
            Way::Listen(l) => format!("UDP port {}", l.port()),
            #[cfg(not(target_arch = "wasm32"))]
            Way::Dial { host, .. } => host.to_string(),
        }
    }

    /// What keeps a new connection from being made, if something does: the
    /// signaling server can't be reached from the rooms this side needs
    /// ([`ErrorKind::ServerUnreachable`]; their sockets are being made
    /// again, and a connection that is up needs none).
    pub fn problem(&self) -> Option<Error> {
        match &self.way {
            Way::Server(s) if !s.rooms.iter().any(|(_, r)| r.is_open()) => {
                s.rooms.iter().find_map(|(_, r)| r.last_error()).map(|why| Error::new(ErrorKind::ServerUnreachable, why))
            }
            _ => None,
        }
    }

    /// Whether the channel is open: what is sent goes.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// How long the channel has been down, while it is (it was open, and is
    /// being made again).
    pub fn down_for(&self) -> Option<Duration> {
        self.down.as_ref().map(|(since, _)| since.elapsed())
    }

    /// How long the link goes on making the channel again before it gives
    /// up ([`Config::reconnect_timeout`], less [`Link::down_for`]), while it
    /// is down: what a "reconnecting" countdown shows.
    pub fn gives_up_in(&self) -> Option<Duration> {
        self.down_for().map(|down| self.config.reconnect_timeout.saturating_sub(down))
    }

    /// The connection's generation (1 for the first; one more each
    /// connection after it).
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Send a datagram to the other player: dropped while the channel is
    /// down, as the network may drop one, and while nothing has come from
    /// the other side for a moment all but one every 100 ms ([`QUIET_AFTER`]).
    /// An error: the link gave up.
    pub fn send(&mut self, datagram: &[u8]) -> Result<(), Error> {
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        let now = Instant::now();
        if self.open
            && sends(now, self.heard, self.sent)
            && let Some(pc) = &mut self.pc
        {
            pc.send(datagram);
            self.sent = Some(now);
        }
        Ok(())
    }

    /// The next datagram that came from the other player, if one has,
    /// without waiting (it keeps the connection up, too): what a connection
    /// that has since dropped had taken comes first. An error: the link gave
    /// up, or the signaling server refused.
    pub fn recv(&mut self) -> Result<Option<&[u8]>, Error> {
        self.step(Instant::now())?;
        match self.inbox.pop_front() {
            Some(d) => {
                self.taken = d;
                Ok(Some(&self.taken))
            }
            None => Ok(None),
        }
    }

    /// Keep the connection up without taking what came.
    pub fn poll(&mut self) -> Result<(), Error> {
        self.step(Instant::now())
    }

    /// Drop everything sent and taken for `length`, as a network outage
    /// would (tests).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn outage(&mut self, length: Duration) {
        let until = Instant::now() + length;
        self.outage = Some(until);
        if let Some(pc) = &mut self.pc {
            pc.set_outage(Some(until));
        }
    }

    /// (Tests) while `hold`, a host says no reconnect code and a joiner
    /// acknowledges none (it still takes the code it is told), as if each
    /// were lost on the way.
    #[cfg(feature = "testing")]
    pub fn hold_codes(&mut self, hold: bool) {
        if let Way::Server(s) = &mut self.way {
            s.hold = hold;
        }
    }

    /// (Tests) the reconnect code agreed last, as this side has it: none
    /// before the first is agreed, or on a direct link.
    #[cfg(feature = "testing")]
    pub fn reconnect_code(&self) -> Option<&str> {
        match &self.way {
            Way::Server(Signaling { agreed: Place::Reconnect(code), .. }) => Some(code),
            _ => None,
        }
    }

    /// (Tests) whether this side is in a room of the signaling server, or
    /// has a socket to it being made.
    #[cfg(feature = "testing")]
    pub fn in_a_room(&self) -> bool {
        matches!(&self.way, Way::Server(s) if !s.rooms.is_empty())
    }

    fn step(&mut self, now: Instant) -> Result<(), Error> {
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        if let Err(e) = self.step_on(now) {
            self.failed = Some(e.clone());
            if let Some(pc) = &mut self.pc {
                pc.close();
            }
            if let Way::Server(s) = &mut self.way {
                s.rooms.clear();
            }
            return Err(e);
        }
        Ok(())
    }

    fn step_on(&mut self, now: Instant) -> Result<(), Error> {
        if self.outage.is_some_and(|t| now >= t) {
            self.outage = None;
        }
        // What the way brings: the rooms' news, the listener's datagrams.
        match &mut self.way {
            Way::Server(s) => {
                if self.down.is_some() {
                    s.enter()?;
                }
                let mut events = Vec::new();
                for (place, room) in &mut s.rooms {
                    while let Some(e) = room.poll(now)? {
                        events.push((place.clone(), e));
                    }
                }
                if s.invite.is_none() {
                    s.invite = s.rooms.iter().find(|(p, _)| *p == Place::Invite).and_then(|(_, r)| r.code()).map(str::to_string);
                }
                for (place, e) in events {
                    self.room_event(now, place, e)?;
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            Way::Listen(_) => self.listen(now)?,
            #[cfg(not(target_arch = "wasm32"))]
            Way::Dial { .. } => {}
        }
        // The connection's news.
        if let Some(pc) = &mut self.pc {
            pc.pump(now);
        }
        while let Some(e) = self.pc.as_mut().and_then(|pc| pc.next_event()) {
            self.peer_event(now, e);
        }
        // What it brought: the caller's datagrams, kept for it, and the
        // link's own (heard when they come, taken or not).
        while self.open
            && let Some(m) = self.pc.as_mut().and_then(PeerConnection::next_message)
        {
            self.heard = now;
            match m {
                Message::Datagram(d) => self.inbox.push_back(d),
                Message::Link(text) => self.told(&text),
            }
        }
        self.say_code(now);
        if self.open && now.duration_since(self.heard) >= self.config.silence {
            let secs = self.config.silence.as_secs_f32();
            self.lost(now, format!("nothing from the other player for {secs} seconds"));
        }
        match self.role {
            Some(Role::Join) => self.dial(now)?,
            Some(Role::Host) => {
                self.give_up_answer(now);
                self.ask(now);
            }
            None => {}
        }
        if let Some((since, why)) = &self.down
            && now.duration_since(*since) >= self.config.reconnect_timeout
        {
            return Err(Error::new(
                ErrorKind::ConnectionLost,
                format!("the connection to the other player dropped ({why}) and didn't come back within {} seconds", self.config.reconnect_timeout.as_secs()),
            ));
        }
        // Before the first connection: the other player is there to connect
        // to (in the room; dialed; dialing this host), and for how long.
        let meeting = !self.ever_open
            && match &self.way {
                Way::Server(s) => s.there() && self.role.is_some(),
                #[cfg(not(target_arch = "wasm32"))]
                Way::Dial { .. } => true,
                #[cfg(not(target_arch = "wasm32"))]
                Way::Listen(_) => self.generation > 0,
            };
        match (meeting, self.meeting) {
            (false, _) => self.meeting = None,
            (true, None) => self.meeting = Some(now),
            (true, Some(since)) if now.duration_since(since) >= self.config.connect_timeout => {
                return Err(Error::new(
                    ErrorKind::CantConnect,
                    format!("no connection to the other player ({}) within {} seconds", self.describe(), self.config.connect_timeout.as_secs()),
                ));
            }
            (true, Some(_)) => {}
        }
        Ok(())
    }

    fn room_event(&mut self, now: Instant, place: Place, event: RoomEvent) -> Result<(), Error> {
        // (A room left since its news came: none of it matters.)
        if let Way::Server(s) = &self.way
            && !s.rooms.iter().any(|(p, _)| *p == place)
        {
            return Ok(());
        }
        match event {
            RoomEvent::Welcome { role, peer, ice_servers } => {
                if let Some(role) = role {
                    if self.role.is_some_and(|r| r != role) {
                        return Err(Error::internal("the signaling server forgot this player's place in the room"));
                    }
                    self.role = Some(role);
                }
                self.given(ice_servers);
                self.peer_here(now, peer);
            }
            RoomEvent::Peer { present, ice_servers } => {
                // (Servers as the other comes: a server gives them only once
                // both players are in the room.)
                if !ice_servers.is_empty() {
                    self.given(ice_servers);
                }
                self.peer_here(now, present);
            }
            RoomEvent::Signal(signal) => self.signal(now, &place, signal),
        }
        Ok(())
    }

    /// A welcome's STUN and TURN servers: exactly those are what the next
    /// connections are made on (a server that gives none leaves it to the
    /// config's).
    fn given(&mut self, ice_servers: Vec<IceServer>) {
        self.servers = if ice_servers.is_empty() { self.config.ice_servers.clone() } else { ice_servers };
    }

    /// The other is in a room (or not): a joiner that isn't connected dials
    /// it (what it sent before it came, or while it was away, is lost).
    fn peer_here(&mut self, now: Instant, present: bool) {
        if present && self.role == Some(Role::Join) && !self.open {
            self.pc = None;
            self.dialed = None;
            self.next_dial = Some(now);
        }
    }

    fn signal(&mut self, now: Instant, place: &Place, signal: Signal) {
        match (self.role, signal) {
            (Some(Role::Host), Signal::Offer { generation, sdp }) if generation > self.generation => self.answer(now, place, generation, &sdp),
            (Some(Role::Join), Signal::Answer { generation, sdp }) if generation == self.generation => {
                if let Some(pc) = &mut self.pc
                    && let Err(e) = pc.set_answer(&sdp)
                {
                    self.lost(now, e.detail);
                }
            }
            (_, Signal::Candidate { generation, candidate }) if generation == self.generation => {
                if let Some(pc) = &mut self.pc {
                    let _ = pc.add_candidate(&candidate);
                }
            }
            (Some(Role::Join), Signal::Redial { generation }) if generation == self.generation => {
                self.lost(now, "the other player lost the connection".into());
                self.next_dial = Some(now);
            }
            _ => {}
        }
    }

    /// The host: answer the joiner's offer of `generation`, which came
    /// through the room at `place`.
    fn answer(&mut self, now: Instant, place: &Place, generation: u32, sdp: &str) {
        // The joiner dialed again: what this side had is dropped.
        if self.open {
            self.lost(now, "the other player dialed again".into());
        }
        if let Way::Server(s) = &mut self.way {
            s.met(place);
        }
        match PeerConnection::answer_with(&self.config, &self.servers, sdp) {
            Ok(pc) => {
                self.adopt(pc, generation);
            }
            // (A bad offer: the joiner's next dial is another.)
            Err(_) => self.pc = None,
        }
    }

    /// A direct host: give up a connection that doesn't open within
    /// [`Config::dial_timeout`] (it has the port, which the next dial
    /// needs).
    fn give_up_answer(&mut self, now: Instant) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Way::Listen(_) = self.way
            && !self.open
            && self.dialed.is_some_and(|t| now.duration_since(t) >= self.config.dial_timeout)
        {
            let secs = self.config.dial_timeout.as_secs();
            self.lost(now, format!("no connection in {secs} seconds"));
        }
        #[cfg(target_arch = "wasm32")]
        let _ = now;
    }

    /// What the other side's link said on the channel: the host's code (the
    /// joiner takes it, is through, and acknowledges it), or the joiner's
    /// acknowledgment of the host's (the host is through).
    fn told(&mut self, text: &str) {
        let Way::Server(s) = &mut self.way else { return };
        match self.role {
            Some(Role::Host) => {
                if let Some(code) = text.strip_prefix(ACK)
                    && s.said.as_ref().is_some_and(|(said, _)| said == code)
                {
                    s.through(code);
                }
            }
            Some(Role::Join) => {
                if let Some(code) = text.strip_prefix(CODE)
                    && signal::valid_reconnect_code(code)
                {
                    s.through(code);
                    if !s.hold
                        && let Some(pc) = &mut self.pc
                    {
                        pc.send_link(&format!("{ACK}{code}"));
                    }
                }
            }
            None => {}
        }
    }

    /// The host, on an open connection: say its code until it is
    /// acknowledged.
    fn say_code(&mut self, now: Instant) {
        if !self.open || self.role != Some(Role::Host) {
            return;
        }
        let Way::Server(s) = &mut self.way else { return };
        let Some((code, at)) = &mut s.said else { return };
        if s.hold || at.is_some_and(|t| now.duration_since(t) < SAY_AGAIN) {
            return;
        }
        if let Some(pc) = &mut self.pc {
            pc.send_link(&format!("{CODE}{code}"));
        }
        *at = Some(now);
    }

    /// The direct host without a connection: listen on its port, and answer
    /// a joiner's check of a newer generation with a new connection (which
    /// has the port then).
    #[cfg(not(target_arch = "wasm32"))]
    fn listen(&mut self, now: Instant) -> Result<(), Error> {
        let Way::Listen(listener) = &mut self.way else { return Ok(()) };
        if self.pc.is_some() {
            return Ok(());
        }
        listener.listen();
        while let Some((n, from)) = listener.recv(&mut self.buf) {
            if self.outage.is_some() {
                continue;
            }
            if let Some(generation) = listener.dialed(&self.buf[..n])
                && generation > self.generation
            {
                let pc = listener.answer(&self.config, generation, from)?;
                self.adopt(pc, generation);
                self.dialed = Some(now);
                break;
            }
        }
        Ok(())
    }

    fn peer_event(&mut self, now: Instant, event: PeerEvent) {
        let generation = self.generation;
        match event {
            PeerEvent::Description(sdp) => {
                if let Way::Server(s) = &mut self.way {
                    s.send(match self.role {
                        Some(Role::Host) => Signal::Answer { generation, sdp },
                        _ => Signal::Offer { generation, sdp },
                    });
                }
            }
            PeerEvent::Candidate(candidate) => {
                if let Way::Server(s) = &mut self.way {
                    s.send(Signal::Candidate { generation, candidate });
                }
            }
            PeerEvent::Open => {
                self.open = true;
                self.ever_open = true;
                self.heard = now;
                self.down = None;
                self.dialed = None;
                self.asked = None;
                self.backoff = Duration::ZERO;
                // The host draws the code of the room to meet in if this
                // connection drops. (A system with no randomness gives none:
                // the two stay in the room they met in.)
                if self.role == Some(Role::Host)
                    && let Way::Server(s) = &mut self.way
                {
                    s.said = crate::secure_random::<{ signal::RECONNECT_CODE_BYTES }>().ok().map(|b| (hex(&b), None));
                }
            }
            PeerEvent::Down(why) => self.lost(now, why),
        }
    }

    /// The connection is lost: dropped (closed), and down if it was open.
    /// What it had taken is kept for the caller (the link's own messages
    /// in it are of a connection that is over, and left).
    fn lost(&mut self, now: Instant, why: String) {
        if self.open {
            self.down = Some((now, why));
            self.open = false;
            self.asked = None;
        }
        if let Some(mut pc) = self.pc.take() {
            while let Some(m) = pc.next_message() {
                if let Message::Datagram(d) = m {
                    self.inbox.push_back(d);
                }
            }
            pc.close();
        }
        self.dialed = None;
        if self.role == Some(Role::Join) {
            self.next_dial = Some(now + self.backoff);
            self.backoff = (self.backoff * 2).clamp(Duration::from_millis(500), MAX_BACKOFF);
        }
    }

    /// A new connection, of `generation`.
    fn adopt(&mut self, pc: PeerConnection, generation: u32) -> &mut PeerConnection {
        self.generation = generation;
        self.open = false;
        let pc = self.pc.insert(pc);
        #[cfg(not(target_arch = "wasm32"))]
        pc.set_outage(self.outage);
        pc
    }

    /// The joiner: dial when it is time (the host in a room with it), and
    /// give a dial that doesn't open up.
    fn dial(&mut self, now: Instant) -> Result<(), Error> {
        if self.open {
            return Ok(());
        }
        if self.dialed.is_some_and(|t| now.duration_since(t) >= self.config.dial_timeout) {
            let secs = self.config.dial_timeout.as_secs();
            self.lost(now, format!("no connection in {secs} seconds"));
        }
        if self.pc.is_some() || self.next_dial.is_none_or(|t| now < t) {
            return Ok(());
        }
        let generation = self.generation + 1;
        // (In a room: on the servers its welcome gave, which came before the
        // other's being there.)
        let pc = match &self.way {
            Way::Server(s) => {
                if !s.there() {
                    return Ok(());
                }
                PeerConnection::offer_with(&self.config, &self.servers)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Way::Dial { host, nonce } => PeerConnection::direct_dial(&self.config, *host, *nonce, generation),
            #[cfg(not(target_arch = "wasm32"))]
            Way::Listen(_) => return Ok(()),
        };
        match pc {
            Ok(pc) => {
                self.adopt(pc, generation);
                self.dialed = Some(now);
                self.next_dial = None;
            }
            // (No socket, say: try again after the wait.)
            Err(e) => {
                self.generation = generation;
                self.lost(now, e.detail);
            }
        }
        Ok(())
    }

    /// A host in a room that is down, with no new connection under way:
    /// ask the joiner to dial again.
    fn ask(&mut self, now: Instant) {
        let Way::Server(s) = &mut self.way else { return };
        if self.down.is_none() || self.pc.is_some() || !s.there() || self.asked.is_some_and(|t| now.duration_since(t) < ASK_AGAIN) {
            return;
        }
        s.send(Signal::Redial { generation: self.generation });
        self.asked = Some(now);
    }
}

impl Drop for Link {
    /// Close the connection, and leave the rooms.
    fn drop(&mut self) {
        if let Some(pc) = &mut self.pc {
            pc.close();
        }
        if let Way::Server(s) = &mut self.way {
            s.rooms.clear();
        }
    }
}

/// Whether a datagram goes at `now`, the other side last heard at `heard`
/// and the last one sent at `sent`: always, unless the other side has been
/// silent for [`QUIET_AFTER`]; then one every [`QUIET_EVERY`].
fn sends(now: Instant, heard: Instant, sent: Option<Instant>) -> bool {
    now.saturating_duration_since(heard) < QUIET_AFTER || sent.is_none_or(|t| now.saturating_duration_since(t) >= QUIET_EVERY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_silent_other_side_is_sent_one_datagram_every_100_ms() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        // Heard just now: every datagram goes.
        assert!(sends(at(50), at(0), Some(at(49))));
        // Silent for 100 ms: one every 100 ms.
        assert!(!sends(at(150), at(0), Some(at(100))));
        assert!(sends(at(200), at(0), Some(at(100))));
        assert!(sends(at(150), at(0), None));
    }
}
