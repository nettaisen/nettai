//! A signaling server in process, for tests: the Worker's protocol
//! (`crate::signal`, `signaling/src/index.ts`), its rooms in memory, a
//! thread a client. It picks an invite room's code as the Worker does,
//! refuses a code no invite room has and a reconnect code not of the
//! clients' form, makes a reconnect room for the first to come to it, and
//! forgets a room as the Worker does: at once when both have left it, and
//! when it has had no socket for ten minutes (an invite room) or
//! [`RECONNECT_ROOM_KEPT`] (a reconnect room). [`Server::kick`] closes a
//! client's WebSocket, as a network or the Worker's restart would;
//! [`Server::forget`] ends a room at once; [`Server::set_reachable`] takes
//! the server off the network and puts it back. [`Client`] is a WebSocket
//! of a test's own, to either server.

use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tungstenite::handshake::server::{Request, Response};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use crate::Role;
use crate::signal::{ALPHABET, CODE_LENGTH, FromServer, PING, PONG, RECONNECT_ROOM_KEPT, Refusal, ToServer, VERSION, valid_code, valid_reconnect_code};

/// How long an invite room with no socket is kept (the Worker's ten
/// minutes).
const INVITE_ROOM_KEPT: Duration = Duration::from_secs(10 * 60);

/// What a client's thread is told to do.
enum Out {
    Text(String),
    /// Close the socket (another of the same client took its place, or a
    /// test kicks it): the other client isn't told it went.
    Close,
}

/// Which kind of room.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Kind {
    Invite,
    Reconnect,
}

/// The rooms, by kind and code.
#[derive(Default)]
struct Rooms {
    rooms: HashMap<(Kind, String), Room>,
    connections: u64,
    /// The codes reconnect rooms were made under, in order.
    made: Vec<String>,
}

impl Rooms {
    /// Forget the rooms whose time ran out.
    fn expire(&mut self, now: Instant) {
        self.rooms.retain(|(kind, _), room| {
            let kept = if *kind == Kind::Invite { INVITE_ROOM_KEPT } else { RECONNECT_ROOM_KEPT };
            room.empty_since.is_none_or(|since| now.duration_since(since) < kept)
        });
    }
}

/// A room: who has each place, and the place's socket, if it is connected
/// (its connection's number, and its thread's inbox). In a reconnect room
/// the places are the first to come and the second, whatever their roles.
#[derive(Default)]
struct Room {
    places: HashMap<Role, String>,
    sockets: HashMap<Role, (u64, Sender<Out>)>,
    /// Since when no socket is in it.
    empty_since: Option<Instant>,
}

fn other(role: Role) -> Role {
    match role {
        Role::Host => Role::Join,
        Role::Join => Role::Host,
    }
}

fn said(m: &FromServer) -> String {
    serde_json::to_string(m).expect("JSON")
}

/// A code drawn from the alphabet.
fn new_code() -> String {
    let mut random = crate::random_u64();
    (0..CODE_LENGTH)
        .map(|_| {
            let c = ALPHABET.as_bytes()[(random % ALPHABET.len() as u64) as usize] as char;
            random /= ALPHABET.len() as u64;
            c
        })
        .collect()
}

/// Refuse a client: say why, and close.
fn refuse(ws: &mut WebSocket<TcpStream>, reason: Refusal, message: &str) {
    let _ = ws.send(Message::text(said(&FromServer::Error { reason, message: message.into() })));
    let _ = ws.close(None);
    let _ = ws.flush();
}

/// The server, listening on loopback until dropped (its threads end with
/// their clients).
pub struct Server {
    addr: SocketAddr,
    rooms: Arc<Mutex<Rooms>>,
    /// Messages relayed, for tests.
    relayed: Arc<AtomicU64>,
    reachable: Arc<AtomicBool>,
}

impl Server {
    pub fn start() -> Server {
        Server::speaking(VERSION)
    }

    /// A server that speaks `version` of the protocol: of another than this
    /// crate's ([`VERSION`]), it refuses every client, as a server newer or
    /// older than a program does.
    pub fn speaking(version: u32) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let addr = listener.local_addr().expect("its address");
        let rooms = Arc::new(Mutex::new(Rooms::default()));
        let relayed = Arc::new(AtomicU64::new(0));
        let reachable = Arc::new(AtomicBool::new(true));
        let (r, n, up) = (rooms.clone(), relayed.clone(), reachable.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                // (Off the network: the connection is dropped unanswered.)
                if !up.load(Ordering::Relaxed) {
                    continue;
                }
                let (r, n) = (r.clone(), n.clone());
                std::thread::spawn(move || serve(stream, version, &r, &n));
            }
        });
        Server { addr, rooms, relayed, reachable }
    }

    /// Its URL, for [`crate::Link::create_room`] and
    /// [`crate::Link::join_room`].
    pub fn url(&self) -> String {
        format!("ws://{}", self.addr)
    }

    /// The codes of the invite rooms there are.
    pub fn codes(&self) -> Vec<String> {
        self.kept(Kind::Invite)
    }

    /// The codes of the reconnect rooms there are.
    pub fn reconnect_rooms(&self) -> Vec<String> {
        self.kept(Kind::Reconnect)
    }

    fn kept(&self, kind: Kind) -> Vec<String> {
        let mut rooms = self.rooms.lock().unwrap();
        rooms.expire(Instant::now());
        rooms.rooms.keys().filter(|(k, _)| *k == kind).map(|(_, code)| code.clone()).collect()
    }

    /// The codes reconnect rooms were made under so far, in order (a code
    /// whose room was forgotten and made again, twice).
    pub fn reconnects_made(&self) -> Vec<String> {
        self.rooms.lock().unwrap().made.clone()
    }

    /// How many sockets are in rooms.
    pub fn sockets(&self) -> usize {
        self.rooms.lock().unwrap().rooms.values().map(|r| r.sockets.len()).sum()
    }

    /// End room `code` (either kind), as its time running out does: its
    /// code names no room from now, and the sockets still in it close.
    pub fn forget(&self, code: &str) {
        let mut rooms = self.rooms.lock().unwrap();
        for kind in [Kind::Invite, Kind::Reconnect] {
            if let Some(room) = rooms.rooms.remove(&(kind, code.to_string())) {
                for (_, tx) in room.sockets.values() {
                    let _ = tx.send(Out::Close);
                }
            }
        }
    }

    /// Close the WebSocket of invite room `code`'s client of `role` (it
    /// comes back on its own).
    pub fn kick(&self, code: &str, role: Role) {
        let rooms = self.rooms.lock().unwrap();
        if let Some((_, tx)) = rooms.rooms.get(&(Kind::Invite, code.to_string())).and_then(|r| r.sockets.get(&role)) {
            let _ = tx.send(Out::Close);
        }
    }

    /// Close every WebSocket in every room, as the server's restart would
    /// (each comes back on its own).
    pub fn kick_all(&self) {
        let rooms = self.rooms.lock().unwrap();
        for (_, tx) in rooms.rooms.values().flat_map(|r| r.sockets.values()) {
            let _ = tx.send(Out::Close);
        }
    }

    /// Take the server off the network (false: every socket in it closes,
    /// and none opens) or put it back (true).
    pub fn set_reachable(&self, reachable: bool) {
        self.reachable.store(reachable, Ordering::Relaxed);
        if !reachable {
            self.kick_all();
        }
    }

    /// The signals relayed so far.
    pub fn relayed(&self) -> u64 {
        self.relayed.load(Ordering::Relaxed)
    }
}

fn serve(stream: TcpStream, version: u32, rooms: &Mutex<Rooms>, relayed: &AtomicU64) {
    let mut uri = String::new();
    let Ok(mut ws) = tungstenite::accept_hdr(stream, |req: &Request, res: Response| {
        uri = req.uri().to_string();
        Ok(res)
    }) else {
        return;
    };
    let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(10)));
    let (path, query) = uri.split_once('?').unwrap_or((&uri, ""));
    let asked = |name: &str| query.split('&').find_map(|kv| kv.strip_prefix(name)?.strip_prefix('=')).unwrap_or("").to_string();
    // A new room is /rooms; an invite room, /rooms/CODE; a reconnect room,
    // /reconnect/CODE.
    let (kind, code) = match (path.strip_prefix("/rooms"), path.strip_prefix("/reconnect/")) {
        (Some(rest), _) => (Kind::Invite, rest.trim_matches('/').to_ascii_uppercase()),
        (_, Some(code)) => (Kind::Reconnect, code.trim_end_matches('/').to_string()),
        _ => return refuse(&mut ws, Refusal::BadRequest, "a new room is /rooms?peer=ID&v=1, a room /rooms/CODE?peer=ID&v=1, a reconnect room /reconnect/CODE?peer=ID&v=1"),
    };
    if asked("v") != version.to_string() {
        return refuse(&mut ws, Refusal::Version, &format!("this server speaks version {version} of the signaling protocol"));
    }
    let peer = asked("peer");
    if peer.is_empty() {
        return refuse(&mut ws, Refusal::BadRequest, "a peer's ID is 1 to 64 letters, digits, _ and -");
    }
    if kind == Kind::Reconnect && !valid_reconnect_code(&code) {
        return refuse(&mut ws, Refusal::BadRequest, "a reconnect code is 32 lowercase hex digits");
    }
    let (tx, rx) = mpsc::channel();
    // Its place: the one it had, else the first free.
    let (key, role, id) = {
        let mut all = rooms.lock().unwrap();
        all.expire(Instant::now());
        all.connections += 1;
        let id = all.connections;
        let key = match kind {
            Kind::Invite if code.is_empty() => {
                // A new room, under a code no room has.
                let code = std::iter::repeat_with(new_code).find(|c| !all.rooms.contains_key(&(Kind::Invite, c.clone()))).expect("a free code");
                all.rooms.insert((Kind::Invite, code.clone()), Room::default());
                (Kind::Invite, code)
            }
            Kind::Invite if valid_code(&code) && all.rooms.contains_key(&(Kind::Invite, code.clone())) => (Kind::Invite, code),
            Kind::Invite => {
                drop(all);
                return refuse(&mut ws, Refusal::NotFound, "no room has that code");
            }
            // The first to come makes a reconnect room.
            Kind::Reconnect => {
                let key = (Kind::Reconnect, code.clone());
                if !all.rooms.contains_key(&key) {
                    all.rooms.insert(key.clone(), Room::default());
                    all.made.push(code);
                }
                key
            }
        };
        let room = all.rooms.get_mut(&key).expect("the room");
        let had = room.places.iter().find(|(_, p)| **p == peer).map(|(r, _)| *r);
        let Some(role) = had.or_else(|| [Role::Host, Role::Join].into_iter().find(|r| !room.places.contains_key(r))) else {
            drop(all);
            return refuse(&mut ws, Refusal::Full, "the room is full");
        };
        room.places.insert(role, peer);
        room.empty_since = None;
        if let Some((_, old)) = room.sockets.insert(role, (id, tx)) {
            let _ = old.send(Out::Close);
        }
        let there = room.sockets.get(&other(role));
        let welcome = match kind {
            Kind::Invite => FromServer::Welcome { role: Some(role), peer: there.is_some(), code: Some(key.1.clone()) },
            Kind::Reconnect => FromServer::Welcome { role: None, peer: there.is_some(), code: None },
        };
        let _ = ws.send(Message::text(said(&welcome)));
        if let Some((_, them)) = there {
            let _ = them.send(Out::Text(said(&FromServer::Peer { present: true })));
        }
        (key, role, id)
    };
    let left = relay(&mut ws, &rx, rooms, &key, role, relayed);
    // Gone: its socket, unless another took its place already; its place
    // too, if it left; and the room, once both have left.
    let mut all = rooms.lock().unwrap();
    let Some(room) = all.rooms.get_mut(&key) else { return };
    if room.sockets.get(&role).is_some_and(|(i, _)| *i == id) {
        room.sockets.remove(&role);
        if let Some((_, them)) = room.sockets.get(&other(role)) {
            let _ = them.send(Out::Text(said(&FromServer::Peer { present: false })));
        }
        if room.sockets.is_empty() {
            room.empty_since = Some(Instant::now());
        }
    }
    if left {
        room.places.remove(&role);
        if room.places.is_empty() {
            all.rooms.remove(&key);
        }
    }
}

/// Relay a client's signals to the other, and the other's to it, until it
/// goes; whether it left (gave its place up).
fn relay(ws: &mut WebSocket<TcpStream>, rx: &Receiver<Out>, rooms: &Mutex<Rooms>, key: &(Kind, String), role: Role, relayed: &AtomicU64) -> bool {
    loop {
        loop {
            match rx.try_recv() {
                Ok(Out::Text(text)) => {
                    if ws.send(Message::text(text)).is_err() {
                        return false;
                    }
                }
                Ok(Out::Close) | Err(TryRecvError::Disconnected) => {
                    let _ = ws.close(None);
                    let _ = ws.flush();
                    return false;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        match ws.read() {
            Ok(Message::Text(text)) if text.as_str() == PING => {
                let _ = ws.send(Message::text(PONG));
            }
            Ok(Message::Text(text)) => match serde_json::from_str::<ToServer>(&text) {
                Ok(ToServer::Signal { data }) => {
                    let all = rooms.lock().unwrap();
                    if let Some((_, them)) = all.rooms.get(key).and_then(|r| r.sockets.get(&other(role))) {
                        let _ = them.send(Out::Text(said(&FromServer::Signal { data })));
                        relayed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Ok(ToServer::Leave) => return true,
                Err(_) => {}
            },
            Ok(Message::Close(_)) => return false,
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {
                let _ = ws.flush();
            }
            Err(_) => return false,
        }
    }
}

/// A WebSocket of a test's own to a signaling server (this one or the
/// Worker): a client that isn't a link, to see what the server says to
/// one.
pub struct Client {
    ws: WebSocket<MaybeTlsStream<TcpStream>>,
}

impl Client {
    /// Connect to `url` (as [`crate::signal::reconnect_url`] and the
    /// others make).
    pub fn open(url: &str) -> Result<Client, String> {
        let (ws, _) = tungstenite::connect(url).map_err(|e| e.to_string())?;
        if let MaybeTlsStream::Plain(s) = ws.get_ref() {
            let _ = s.set_read_timeout(Some(Duration::from_millis(10)));
        }
        Ok(Client { ws })
    }

    /// The next thing the server says, waiting up to `wait` for it; none if
    /// it says nothing or the socket closes.
    pub fn next(&mut self, wait: Duration) -> Option<FromServer> {
        let until = Instant::now() + wait;
        while Instant::now() < until {
            match self.ws.read() {
                Ok(Message::Text(text)) => {
                    if let Ok(m) = serde_json::from_str(&text) {
                        return Some(m);
                    }
                }
                Ok(Message::Close(_)) => return None,
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                Err(_) => return None,
            }
        }
        None
    }

    /// Give the place in the room up, and close.
    pub fn leave(mut self) {
        let _ = self.ws.send(Message::text(serde_json::to_string(&ToServer::Leave).expect("JSON")));
        let _ = self.ws.close(None);
        // (The close goes out; its answer may not come in time to matter.)
        for _ in 0..20 {
            match self.ws.read() {
                Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                Err(_) => break,
                Ok(_) => {}
            }
        }
    }
}
