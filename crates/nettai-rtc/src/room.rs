//! A client of the signaling server, in a room (`crate::signal`): its
//! WebSocket, made again whenever it closes (the same peer id, so the same
//! place in the room), what it says queued while it is down, a keepalive,
//! and what the server says, read.

use std::collections::VecDeque;

#[cfg(not(target_arch = "wasm32"))]
use crate::native::ws::{Socket, WsEvent};
use crate::signal::{self, FromServer, Signal, ToServer};
#[cfg(target_arch = "wasm32")]
use crate::web::ws::{Socket, WsEvent};
use crate::{Duration, Error, Instant, Role};

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
    url: String,
    code: String,
    socket: Option<Socket>,
    open: bool,
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
    /// Room `code` on the server at `server`, as the client `peer`:
    /// connecting at once.
    pub(crate) fn new(server: &str, code: &str, peer: &str) -> Result<Room, Error> {
        if !signal::valid_code(code) {
            return Err(Error(format!("{code:?} isn't a room code: 1 to 64 letters, digits, _ and -")));
        }
        let url = signal::room_url(server, code, peer);
        Ok(Room {
            socket: Some(Socket::connect(&url)),
            url,
            code: code.to_string(),
            open: false,
            retry: None,
            backoff: Duration::from_millis(500),
            outbox: VecDeque::new(),
            pinged: Instant::now(),
            last_error: None,
        })
    }

    pub(crate) fn code(&self) -> &str {
        &self.code
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

    /// What the server said since, without waiting; an error is a refusal
    /// (the room is full, the code is bad), which ends it.
    pub(crate) fn poll(&mut self, now: Instant) -> Result<Option<RoomEvent>, Error> {
        if self.socket.is_none() {
            match self.retry {
                Some(at) if now >= at => {
                    self.socket = Some(Socket::connect(&self.url));
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
                    Ok(FromServer::Welcome { role, peer }) => return Ok(Some(RoomEvent::Welcome { role, peer })),
                    Ok(FromServer::Peer { present }) => return Ok(Some(RoomEvent::Peer(present))),
                    Ok(FromServer::Signal { data }) => return Ok(Some(RoomEvent::Signal(data))),
                    Ok(FromServer::Error { message }) => {
                        self.leave();
                        return Err(Error(format!("the signaling server refused: {message}")));
                    }
                    // (Something newer than this client: not for it.)
                    Err(_) => {}
                },
                WsEvent::Closed(why) => {
                    self.last_error = Some(why);
                    self.open = false;
                    self.socket = None;
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
