//! A client of the signaling server, in a room (`crate::protocol`): an invite
//! room it asked the server to make (whose code the server's welcome says)
//! or one it was given the code of, or a reconnect room, by the code the two
//! clients agreed on. Its WebSocket is made again whenever it closes (the
//! same peer id, so the same place in the room), what it says is queued
//! while it is down, a keepalive, and what the server says, read.
//!
//! In an invite room, before the server has first let it in, a socket that
//! doesn't open or closes is made again too (a connection that drops
//! mid-handshake is common on long, lossy routes, and the next one usually
//! works), until its `server_timeout` has passed since it began: that ends
//! it, saying the server can't be reached and why the last socket closed
//! (the player asked for a room now, and is told soon). Once it is in, a
//! socket that drops is made again for as long as the room is kept. A
//! reconnect room's socket is made again whatever happened, from the first:
//! its owner gives up on its own time.

use std::collections::VecDeque;
use std::fmt;
use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;
use web_time::{Duration, Instant};

use crate::platform::system::{Socket, WsEvent};
use crate::protocol::{self as signal, FromServer, IceServer, Refusal, Role, ToServer};

/// Why a room ended: the server can't be reached, or it refused. The room
/// is left; nothing more comes of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    /// What happened, in English, for a log.
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The server didn't let this client into an invite room within its
    /// `server_timeout` (its socket didn't open, or the server didn't
    /// answer).
    Unreachable,
    /// The server refused this client, for this reason. A code that isn't
    /// one is refused before the server is asked: no room has it
    /// ([`Refusal::NotFound`]).
    Refused(Refusal),
    /// The server said what no server of this protocol says.
    Broken,
}

impl Error {
    fn new(kind: ErrorKind, detail: impl Into<String>) -> Error {
        Error { kind, detail: detail.into() }
    }
}

/// (The detail: what a log says.)
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for Error {}

/// What the room says; `T`, what the other client says.
#[derive(Debug)]
pub enum RoomEvent<T> {
    /// This client's place (in an invite room), whether the other is there,
    /// and the STUN and TURN servers its connections use (none: the server
    /// gives none).
    Welcome { role: Option<Role>, peer: bool, ice_servers: Vec<IceServer> },
    /// The other came, or went; the servers the server gives now that both
    /// are here (none: those from before).
    Peer { present: bool, ice_servers: Vec<IceServer> },
    Signal(T),
}

/// How often a client says it is there (a WebSocket that says nothing for
/// long is closed by some networks).
const PING: Duration = Duration::from_secs(30);
/// The longest wait before connecting again.
const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// Which room.
enum Kind {
    /// An invite room: by its code, or none until the server has made the
    /// room and said its code.
    Invite(Option<String>),
    /// A reconnect room, by the code the two clients agreed on.
    Reconnect(String),
}

/// A client in one room, saying `T` to the other client and hearing it.
pub struct Room<T> {
    server: String,
    peer: String,
    kind: Kind,
    socket: Option<Socket>,
    open: bool,
    /// The server has let this client in (at least once).
    welcomed: bool,
    /// The other is in the room, as the server last said.
    there: bool,
    /// When it began: the server answers within `server_timeout` of it.
    began: Instant,
    server_timeout: Duration,
    /// When to connect again, while there is no socket.
    retry: Option<Instant>,
    backoff: Duration,
    /// What to say once it is open.
    outbox: VecDeque<String>,
    pinged: Instant,
    /// Why the socket last closed (for an owner that gives up).
    last_error: Option<String>,
    said: PhantomData<fn(T) -> T>,
}

impl<T> Room<T> {
    /// A new invite room on the server at `server`, which this client `peer`
    /// hosts: connecting at once. Its code comes with the server's welcome
    /// ([`Room::code`]).
    pub fn create(server: &str, peer: &str, server_timeout: Duration) -> Room<T> {
        Room::new(server, Kind::Invite(None), peer, server_timeout)
    }

    /// The invite room with `code` on the server at `server`, as the client
    /// `peer`: connecting at once. No room has a code that isn't one
    /// ([`signal::normalize_code`]).
    pub fn join(server: &str, code: &str, peer: &str, server_timeout: Duration) -> Result<Room<T>, Error> {
        let code = signal::normalize_code(code).ok_or_else(|| Error::new(ErrorKind::Refused(Refusal::NotFound), format!("{code:?} isn't a room's code: {} of {}", signal::CODE_LENGTH, signal::ALPHABET)))?;
        Ok(Room::new(server, Kind::Invite(Some(code)), peer, server_timeout))
    }

    /// The reconnect room with `code` (one the two clients agreed on) on the
    /// server at `server`, as the client `peer`: connecting at once.
    pub fn reconnect(server: &str, code: &str, peer: &str) -> Room<T> {
        Room::new(server, Kind::Reconnect(code.to_string()), peer, Duration::MAX)
    }

    fn new(server: &str, kind: Kind, peer: &str, server_timeout: Duration) -> Room<T> {
        let now = Instant::now();
        let mut room = Room {
            server: server.to_string(),
            peer: peer.to_string(),
            kind,
            socket: None,
            open: false,
            welcomed: false,
            there: false,
            began: now,
            server_timeout,
            retry: None,
            backoff: Duration::from_millis(500),
            outbox: VecDeque::new(),
            pinged: now,
            last_error: None,
            said: PhantomData,
        };
        room.socket = Some(Socket::connect(&room.url()));
        room
    }

    /// Where its socket connects: the room, by its code; before an invite
    /// room has one, the server's maker of rooms.
    fn url(&self) -> String {
        match &self.kind {
            Kind::Invite(Some(code)) => signal::room_url(&self.server, code, &self.peer),
            Kind::Invite(None) => signal::new_room_url(&self.server, &self.peer),
            Kind::Reconnect(code) => signal::reconnect_url(&self.server, code, &self.peer),
        }
    }

    /// An invite room's code, once it has one (a new room's is the server's
    /// to say).
    pub fn code(&self) -> Option<&str> {
        match &self.kind {
            Kind::Invite(code) => code.as_deref(),
            Kind::Reconnect(_) => None,
        }
    }

    fn invite(&self) -> bool {
        matches!(self.kind, Kind::Invite(_))
    }

    /// Whether the socket is up.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Whether the other is in the room, as far as this client knows.
    pub fn there(&self) -> bool {
        self.open && self.there
    }

    /// Why the socket last closed.
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Say `signal` to the other client (queued while the socket is down).
    pub fn send(&mut self, signal: &T)
    where
        T: Serialize,
    {
        let text = serde_json::to_string(&ToServer::Signal { data: signal }).expect("a signal is JSON");
        match &mut self.socket {
            Some(s) if self.open => s.send(text),
            _ => self.outbox.push_back(text),
        }
    }

    /// Give the place in the room up, and close (a socket still opening
    /// says so once it has opened: the server may have given it a place).
    pub fn leave(&mut self) {
        if let Some(s) = &mut self.socket {
            s.send(serde_json::to_string(&ToServer::<()>::Leave).expect("JSON"));
            s.close();
        }
        self.socket = None;
        self.retry = None;
        self.open = false;
    }

    /// What the server said since, without waiting. An error ends it: the
    /// server refused (no such room, a full one, another version), or, in an
    /// invite room, couldn't be reached before it first let this client in.
    pub fn poll(&mut self, now: Instant) -> Result<Option<RoomEvent<T>>, Error>
    where
        T: DeserializeOwned,
    {
        if self.invite() && !self.welcomed && now.duration_since(self.began) >= self.server_timeout {
            self.leave();
            let mut why = format!("no answer from the signaling server in {} seconds", self.server_timeout.as_secs());
            if let Some(last) = &self.last_error {
                why = format!("{why} (the last try: {last})");
            }
            return Err(Error::new(ErrorKind::Unreachable, why));
        }
        if self.socket.is_none() {
            match self.retry {
                Some(at) if now >= at => {
                    self.socket = Some(Socket::connect(&self.url()));
                    self.retry = None;
                }
                _ => return Ok(None),
            }
        }
        let socket = self.socket.as_mut().expect("a socket");
        if self.open && now.duration_since(self.pinged) >= PING {
            socket.send(signal::PING.into());
            self.pinged = now;
        }
        loop {
            let Some(event) = socket.poll() else { return Ok(None) };
            match event {
                WsEvent::Open => {
                    self.open = true;
                    self.backoff = Duration::from_millis(500);
                    self.pinged = now;
                    for text in self.outbox.drain(..) {
                        socket.send(text);
                    }
                }
                WsEvent::Text(text) if text == signal::PONG => {}
                WsEvent::Text(text) => match serde_json::from_str::<FromServer<T>>(&text) {
                    Ok(FromServer::Welcome { role, peer, code, ice_servers }) => {
                        if let Kind::Invite(had) = &mut self.kind {
                            if role.is_none() {
                                self.leave();
                                return Err(Error::new(ErrorKind::Broken, "the signaling server didn't say this player's place in the room"));
                            }
                            // (A new room's code; a room come back to says
                            // the one it was asked by.)
                            if had.is_none()
                                && let Some(code) = code.filter(|c| signal::valid_code(c))
                            {
                                *had = Some(code);
                            }
                        }
                        self.welcomed = true;
                        self.there = peer;
                        let role = if self.invite() { role } else { None };
                        return Ok(Some(RoomEvent::Welcome { role, peer, ice_servers }));
                    }
                    Ok(FromServer::Peer { present, ice_servers }) => {
                        self.there = present;
                        return Ok(Some(RoomEvent::Peer { present, ice_servers }));
                    }
                    Ok(FromServer::Signal { data }) => return Ok(Some(RoomEvent::Signal(data))),
                    Ok(FromServer::Error { reason, message }) => {
                        self.leave();
                        return Err(Error::new(ErrorKind::Refused(reason), format!("the signaling server refused: {message}")));
                    }
                    // (Something newer than this client: not for it.)
                    Err(_) => {}
                },
                WsEvent::Closed(why) => {
                    self.open = false;
                    self.there = false;
                    self.socket = None;
                    self.last_error = Some(why);
                    self.retry = Some(now + self.backoff);
                    self.backoff = (self.backoff * 2).min(MAX_BACKOFF);
                    return Ok(None);
                }
            }
        }
    }
}

impl<T> Drop for Room<T> {
    fn drop(&mut self) {
        self.leave();
    }
}
