//! A client of the signaling server, in a room (`crate::signal`): one it
//! asked the server to make (whose code the server's welcome says) or one it
//! was given the code of. Its WebSocket is made again whenever it closes
//! (the same peer id, so the same place in the room), what it says is queued
//! while it is down, a keepalive, and what the server says, read.
//!
//! Before the server has first let it in, nothing is tried twice: a socket
//! that doesn't open, or a server that doesn't answer in
//! [`Config::server_timeout`](crate::Config::server_timeout), ends it,
//! saying the server can't be reached (the player asked for a room now, and
//! is told now). Once it is in, a socket that drops is made again for as
//! long as the room is kept.

use std::collections::VecDeque;

#[cfg(not(target_arch = "wasm32"))]
use crate::native::ws::{Socket, WsEvent};
use crate::signal::{self, FromServer, Refusal, Signal, ToServer};
#[cfg(target_arch = "wasm32")]
use crate::web::ws::{Socket, WsEvent};
use crate::{Duration, Error, ErrorKind, Instant, Role};

/// What the room says, for the link.
#[derive(Debug)]
pub(crate) enum RoomEvent {
    /// This client's place, and whether the other is there.
    Welcome { role: Role, peer: bool },
    /// The other came, or went.
    Peer(bool),
    Signal(Signal),
}

/// How often a client says it is there (a WebSocket that says nothing for
/// long is closed by some networks).
const PING: Duration = Duration::from_secs(30);
/// The longest wait before connecting again.
const MAX_BACKOFF: Duration = Duration::from_secs(5);

pub(crate) struct Room {
    server: String,
    peer: String,
    /// The room's code: the one given, or none until the server has made
    /// the room and said its code.
    code: Option<String>,
    socket: Option<Socket>,
    open: bool,
    /// The server has let this client in (at least once).
    welcomed: bool,
    /// When it began: the server answers within `server_timeout` of it.
    began: Instant,
    server_timeout: Duration,
    /// When to connect again, while there is no socket.
    retry: Option<Instant>,
    backoff: Duration,
    /// What to say once it is open.
    outbox: VecDeque<String>,
    pinged: Instant,
    /// Why the socket last closed (for a link that gives up).
    last_error: Option<String>,
}

impl Room {
    /// A new room on the server at `server`, which this client `peer`
    /// hosts: connecting at once. Its code comes with the server's welcome
    /// ([`Room::code`]).
    pub(crate) fn create(server: &str, peer: &str, server_timeout: Duration) -> Room {
        Room::new(server, None, peer, server_timeout)
    }

    /// The room with `code` on the server at `server`, as the client
    /// `peer`: connecting at once. No room has a code that isn't one
    /// ([`signal::normalize_code`]).
    pub(crate) fn join(server: &str, code: &str, peer: &str, server_timeout: Duration) -> Result<Room, Error> {
        let code = signal::normalize_code(code).ok_or_else(|| Error::new(ErrorKind::NoSuchRoom, format!("{code:?} isn't a room's code: {} of {}", signal::CODE_LENGTH, signal::ALPHABET)))?;
        Ok(Room::new(server, Some(code), peer, server_timeout))
    }

    fn new(server: &str, code: Option<String>, peer: &str, server_timeout: Duration) -> Room {
        let now = Instant::now();
        let mut room = Room {
            server: server.to_string(),
            peer: peer.to_string(),
            code,
            socket: None,
            open: false,
            welcomed: false,
            began: now,
            server_timeout,
            retry: None,
            backoff: Duration::from_millis(500),
            outbox: VecDeque::new(),
            pinged: now,
            last_error: None,
        };
        room.socket = Some(Socket::connect(&room.url()));
        room
    }

    /// Where its socket connects: the room, by its code; before it has one,
    /// the server's maker of rooms.
    fn url(&self) -> String {
        match &self.code {
            Some(code) => signal::room_url(&self.server, code, &self.peer),
            None => signal::new_room_url(&self.server, &self.peer),
        }
    }

    /// The room's code, once it has one (a new room's is the server's to
    /// say).
    pub(crate) fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Whether the socket is up.
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Why the socket last closed.
    pub(crate) fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Say `signal` to the other client (queued while the socket is down).
    pub(crate) fn send(&mut self, signal: Signal) {
        let text = serde_json::to_string(&ToServer::Signal { data: signal }).expect("a signal is JSON");
        match &mut self.socket {
            Some(s) if self.open => s.send(text),
            _ => self.outbox.push_back(text),
        }
    }

    /// Give the place in the room up, and close.
    pub(crate) fn leave(&mut self) {
        if let Some(s) = &mut self.socket {
            if self.open {
                s.send(serde_json::to_string(&ToServer::Leave).expect("JSON"));
            }
            s.close();
        }
        self.socket = None;
        self.retry = None;
    }

    /// What the server said since, without waiting. An error ends it: the
    /// server refused (no such room, a full one, another version), or
    /// couldn't be reached before it first let this client in.
    pub(crate) fn poll(&mut self, now: Instant) -> Result<Option<RoomEvent>, Error> {
        if !self.welcomed && now.duration_since(self.began) >= self.server_timeout {
            self.leave();
            return Err(Error::new(ErrorKind::ServerUnreachable, format!("no answer from the signaling server in {} seconds", self.server_timeout.as_secs())));
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
                WsEvent::Text(text) => match serde_json::from_str::<FromServer>(&text) {
                    Ok(FromServer::Welcome { role, peer, code }) => {
                        // (A new room's code; a room come back to says the
                        // one it was asked by.)
                        if self.code.is_none() && signal::valid_code(&code) {
                            self.code = Some(code);
                        }
                        self.welcomed = true;
                        return Ok(Some(RoomEvent::Welcome { role, peer }));
                    }
                    Ok(FromServer::Peer { present }) => return Ok(Some(RoomEvent::Peer(present))),
                    Ok(FromServer::Signal { data }) => return Ok(Some(RoomEvent::Signal(data))),
                    Ok(FromServer::Error { reason, message }) => {
                        self.leave();
                        let kind = match reason {
                            Refusal::NotFound => ErrorKind::NoSuchRoom,
                            Refusal::Full => ErrorKind::RoomFull,
                            Refusal::Version => ErrorKind::ServerVersion,
                            Refusal::BadRequest | Refusal::Busy | Refusal::Unknown => ErrorKind::Internal,
                        };
                        return Err(Error::new(kind, format!("the signaling server refused: {message}")));
                    }
                    // (Something newer than this client: not for it.)
                    Err(_) => {}
                },
                WsEvent::Closed(why) => {
                    self.open = false;
                    self.socket = None;
                    if !self.welcomed {
                        return Err(Error::new(ErrorKind::ServerUnreachable, why));
                    }
                    self.last_error = Some(why);
                    self.retry = Some(now + self.backoff);
                    self.backoff = (self.backoff * 2).min(MAX_BACKOFF);
                    return Ok(None);
                }
            }
        }
    }
}

impl Drop for Room {
    fn drop(&mut self) {
        self.leave();
    }
}
