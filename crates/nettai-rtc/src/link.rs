//! The link: the channel a match is played on, which keeps a WebRTC
//! connection to the other player up (docs/design/rollback.md §4.8).
//!
//! The joiner dials and the host answers, in a room and directly alike.
//! Every connection is a generation: the joiner's first dial is 1, each
//! dial after it one more, and what is said of an older one (an answer, a
//! candidate) is dropped. A dropped connection isn't mended (no ICE
//! restart): the joiner dials a new one, the next generation, which works
//! the same on both backends and directly, and the game's channel has
//! nothing in it worth keeping (what was lost on it, the protocol sends
//! again).
//!
//! **A drop** is any of: the connection's state (disconnected, failed,
//! closed), its data channel closing, or silence: nothing from the other
//! side for [`Config::silence`] while it is open. Then:
//!
//! - the joiner dials again at once, and again after 0.5, 1, 2, then every
//!   3 seconds while dials don't open (each given [`Config::dial_timeout`]);
//!   in a room, also when the host says it lost the connection, or when the
//!   host (back) is in the room while it isn't connected;
//! - the host waits for the next generation: an offer in the room (which it
//!   asks for every 2 seconds), or the joiner's connectivity checks
//!   directly.
//!
//! While it is down, what is sent is dropped and nothing comes, and
//! [`Link::down_for`] says how long it has been; after
//! [`Config::reconnect_timeout`] the link gives up, and every call after
//! errs, saying so. Before it is first open nothing is down: it is still
//! connecting, and how long to wait for the other player is the caller's to
//! say (its lobby's timeout).

// (On the web a room is the one way to meet.)
#![cfg_attr(target_arch = "wasm32", allow(irrefutable_let_patterns))]

use crate::room::{Room, RoomEvent};
use crate::signal::Signal;
use crate::{Config, Duration, Error, Instant, PeerConnection, PeerEvent, Role};

#[cfg(not(target_arch = "wasm32"))]
use crate::native::{Servers, direct::Listener};
#[cfg(target_arch = "wasm32")]
use crate::web::Servers;

/// How often a host in a room asks the joiner to dial again while it is
/// down.
const ASK_AGAIN: Duration = Duration::from_secs(2);
/// The longest wait between two dials.
const MAX_BACKOFF: Duration = Duration::from_secs(3);

/// How the two players meet.
enum Way {
    /// In a room of the signaling server.
    Room(Room),
    /// Directly: the host listens.
    #[cfg(not(target_arch = "wasm32"))]
    Listen(Listener),
    /// Directly: the joiner dials the host's address, as `nonce`.
    #[cfg(not(target_arch = "wasm32"))]
    Dial { host: std::net::SocketAddr, nonce: u32 },
}

/// The channel to the other player (the module's docs).
pub struct Link {
    config: Config,
    servers: Servers,
    way: Way,
    role: Option<Role>,
    /// The connection, if there is one, and its generation.
    pc: Option<PeerConnection>,
    generation: u32,
    /// Its channel is open.
    open: bool,
    /// When the open channel last heard from the other side.
    heard: Instant,
    /// Since when it is down, and why (it was open before).
    down: Option<(Instant, String)>,
    /// The joiner: when to dial next, the wait after that, and when the
    /// dial that isn't open yet began.
    next_dial: Option<Instant>,
    backoff: Duration,
    dialed: Option<Instant>,
    /// A host in a room: when it last asked for a new dial.
    asked: Option<Instant>,
    /// In a room: the other is there.
    peer: bool,
    failed: Option<Error>,
    taken: Vec<u8>,
    #[cfg(not(target_arch = "wasm32"))]
    buf: Vec<u8>,
    /// Drop everything sent and taken until then (tests).
    outage: Option<Instant>,
}

impl Link {
    fn new(config: Config, servers: Servers, way: Way, role: Option<Role>) -> Link {
        let now = Instant::now();
        Link {
            config,
            servers,
            way,
            role,
            pc: None,
            generation: 0,
            open: false,
            heard: now,
            down: None,
            next_dial: Some(now),
            backoff: Duration::ZERO,
            dialed: None,
            asked: None,
            peer: false,
            failed: None,
            taken: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            buf: vec![0; crate::native::MAX_DATAGRAM],
            outage: None,
        }
    }

    /// Meet the other player in room `code` of the signaling server at
    /// `server` (`wss://...`): the first in the room hosts. The STUN and
    /// TURN servers' names are resolved now (natively: which may wait).
    pub fn room(server: &str, code: &str, config: Config) -> Result<Link, Error> {
        let peer = format!("{:016x}", crate::random_u64());
        let room = Room::new(server, code, &peer)?;
        let servers = Servers::resolve(&config);
        Ok(Link::new(config, servers, Way::Room(room), None))
    }

    /// Host directly: listen on UDP `port` of every IPv4 interface (0: one
    /// the system picks, [`Link::port`]) for the joiner.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn host(port: u16, config: Config) -> Result<Link, Error> {
        let listener = Listener::bind(port)?;
        Ok(Link::new(config, Servers::default(), Way::Listen(listener), Some(Role::Host)))
    }

    /// Join the host at `addr` (`host:port`) directly (resolving its name
    /// now, which may wait).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn join(addr: &str, config: Config) -> Result<Link, Error> {
        use std::net::ToSocketAddrs;
        let host = addr
            .to_socket_addrs()
            .map_err(|e| Error(format!("can't resolve {addr}: {e}")))?
            .min_by_key(|a| !a.is_ipv4())
            .ok_or_else(|| Error(format!("{addr} resolves to nothing")))?;
        let nonce = crate::random_u64() as u32;
        Ok(Link::new(config, Servers::default(), Way::Dial { host, nonce }, Some(Role::Join)))
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

    /// Who the other player is, as far as this side knows: the room, or
    /// the address.
    pub fn describe(&self) -> String {
        match &self.way {
            Way::Room(r) => format!("room {}", r.code()),
            #[cfg(not(target_arch = "wasm32"))]
            Way::Listen(l) => format!("UDP port {}", l.port()),
            #[cfg(not(target_arch = "wasm32"))]
            Way::Dial { host, .. } => host.to_string(),
        }
    }

    /// What keeps it from connecting, if something does (the signaling
    /// server can't be reached).
    pub fn problem(&self) -> Option<String> {
        match &self.way {
            Way::Room(r) if !r.is_open() => r.last_error().map(str::to_string),
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

    /// The connection's generation (1 for the first; one more each
    /// connection after it).
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Send a datagram to the other player: dropped while the channel is
    /// down, as the network may drop one. An error: the link gave up.
    pub fn send(&mut self, datagram: &[u8]) -> Result<(), Error> {
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        if self.open
            && let Some(pc) = &mut self.pc
        {
            pc.send(datagram);
        }
        Ok(())
    }

    /// The next datagram that came from the other player, if one has,
    /// without waiting (it keeps the connection up, too). An error: the link
    /// gave up, or the signaling server refused.
    pub fn recv(&mut self) -> Result<Option<&[u8]>, Error> {
        let now = Instant::now();
        self.step(now)?;
        let Some(pc) = &mut self.pc else { return Ok(None) };
        match pc.next_datagram() {
            Some(d) if self.open => {
                self.heard = now;
                self.taken = d;
                Ok(Some(&self.taken))
            }
            _ => Ok(None),
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

    fn step(&mut self, now: Instant) -> Result<(), Error> {
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        if let Err(e) = self.step_on(now) {
            self.failed = Some(e.clone());
            if let Some(pc) = &mut self.pc {
                pc.close();
            }
            if let Way::Room(r) = &mut self.way {
                r.leave();
            }
            return Err(e);
        }
        Ok(())
    }

    fn step_on(&mut self, now: Instant) -> Result<(), Error> {
        if self.outage.is_some_and(|t| now >= t) {
            self.outage = None;
        }
        // What the way brings: the room's news, the listener's datagrams.
        match &mut self.way {
            Way::Room(room) => {
                let mut events = Vec::new();
                while let Some(e) = room.poll(now)? {
                    events.push(e);
                }
                for e in events {
                    self.room_event(now, e)?;
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
        if self.open && now.duration_since(self.heard) >= self.config.silence {
            let secs = self.config.silence.as_secs_f32();
            self.lost(now, format!("nothing from the other player for {secs} seconds"));
        }
        match self.role {
            Some(Role::Join) => self.dial(now)?,
            Some(Role::Host) => self.ask(now),
            None => {}
        }
        if let Some((since, why)) = &self.down
            && now.duration_since(*since) >= self.config.reconnect_timeout
        {
            return Err(Error(format!(
                "the connection to the other player dropped ({why}) and didn't come back within {} seconds",
                self.config.reconnect_timeout.as_secs()
            )));
        }
        Ok(())
    }

    fn room_event(&mut self, now: Instant, event: RoomEvent) -> Result<(), Error> {
        match event {
            RoomEvent::Welcome { role, peer } => {
                if self.role.is_some_and(|r| r != role) {
                    return Err(Error("the signaling server forgot this player's place in the room".into()));
                }
                self.role = Some(role);
                self.peer_here(now, peer);
            }
            RoomEvent::Peer(present) => self.peer_here(now, present),
            RoomEvent::Signal(signal) => self.signal(now, signal)?,
        }
        Ok(())
    }

    /// The other is in the room (or not): a joiner that isn't connected
    /// dials it (what it sent before it came, or while it was away, is
    /// lost).
    fn peer_here(&mut self, now: Instant, present: bool) {
        self.peer = present;
        if present && self.role == Some(Role::Join) && !self.open {
            self.pc = None;
            self.dialed = None;
            self.next_dial = Some(now);
        }
    }

    fn signal(&mut self, now: Instant, signal: Signal) -> Result<(), Error> {
        match (self.role, signal) {
            (Some(Role::Host), Signal::Offer { generation, sdp }) if generation > self.generation => {
                // The joiner dialed again: what this side had is dropped.
                if self.open {
                    self.lost(now, "the other player dialed again".into());
                }
                match PeerConnection::answer_with(&self.config, &self.servers, &sdp) {
                    Ok(pc) => self.adopt(pc, generation),
                    // (A bad offer: the joiner's next dial is another.)
                    Err(_) => self.pc = None,
                }
            }
            (Some(Role::Join), Signal::Answer { generation, sdp }) if generation == self.generation => {
                if let Some(pc) = &mut self.pc
                    && let Err(e) = pc.set_answer(&sdp)
                {
                    self.lost(now, e.0);
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
        Ok(())
    }

    /// The direct host's datagrams: a joiner's check of a newer generation
    /// makes a new connection to answer it; every datagram goes to the
    /// connection.
    #[cfg(not(target_arch = "wasm32"))]
    fn listen(&mut self, now: Instant) -> Result<(), Error> {
        loop {
            let Way::Listen(listener) = &mut self.way else { return Ok(()) };
            let Some((n, from)) = listener.recv(&mut self.buf) else { return Ok(()) };
            if self.outage.is_some() {
                continue;
            }
            let local = listener.local();
            if let Some(generation) = listener.dialed(&self.buf[..n])
                && generation > self.generation
            {
                let pc = listener.answer(&self.config, generation)?;
                if self.open {
                    self.lost(now, "the other player dialed again".into());
                }
                self.adopt(pc, generation);
            }
            if let Some(pc) = &mut self.pc {
                pc.feed(now, local, from, &self.buf[..n]);
            }
        }
    }

    fn peer_event(&mut self, now: Instant, event: PeerEvent) {
        let generation = self.generation;
        match event {
            PeerEvent::Description(sdp) => {
                if let Way::Room(room) = &mut self.way {
                    room.send(match self.role {
                        Some(Role::Host) => Signal::Answer { generation, sdp },
                        _ => Signal::Offer { generation, sdp },
                    });
                }
            }
            PeerEvent::Candidate(candidate) => {
                if let Way::Room(room) = &mut self.way {
                    room.send(Signal::Candidate { generation, candidate });
                }
            }
            PeerEvent::Open => {
                self.open = true;
                self.heard = now;
                self.down = None;
                self.dialed = None;
                self.asked = None;
                self.backoff = Duration::ZERO;
            }
            PeerEvent::Down(why) => self.lost(now, why),
        }
    }

    /// The connection is lost: dropped (closed), and down if it was open.
    fn lost(&mut self, now: Instant, why: String) {
        if self.open {
            self.down = Some((now, why));
            self.open = false;
            self.asked = None;
        }
        if let Some(mut pc) = self.pc.take() {
            pc.close();
        }
        self.dialed = None;
        if self.role == Some(Role::Join) {
            self.next_dial = Some(now + self.backoff);
            self.backoff = (self.backoff * 2).clamp(Duration::from_millis(500), MAX_BACKOFF);
        }
    }

    /// A new connection, of `generation`.
    fn adopt(&mut self, pc: PeerConnection, generation: u32) {
        let pc = self.pc.insert(pc);
        #[cfg(not(target_arch = "wasm32"))]
        pc.set_outage(self.outage);
        #[cfg(target_arch = "wasm32")]
        let _ = pc;
        self.generation = generation;
        self.open = false;
    }

    /// The joiner: dial when it is time (a room's host there), and give a
    /// dial that doesn't open up.
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
        let pc = match &self.way {
            Way::Room(room) => {
                if !self.peer || !room.is_open() {
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
                self.lost(now, e.0);
            }
        }
        Ok(())
    }

    /// A host in a room that is down, with no new connection under way:
    /// ask the joiner to dial again.
    fn ask(&mut self, now: Instant) {
        let Way::Room(room) = &mut self.way else { return };
        if self.down.is_none() || self.pc.is_some() || !self.peer || self.asked.is_some_and(|t| now.duration_since(t) < ASK_AGAIN) {
            return;
        }
        room.send(Signal::Redial { generation: self.generation });
        self.asked = Some(now);
    }
}

impl Drop for Link {
    /// Close the connection, and leave the room.
    fn drop(&mut self) {
        if let Some(pc) = &mut self.pc {
            pc.close();
        }
        if let Way::Room(r) = &mut self.way {
            r.leave();
        }
    }
}
