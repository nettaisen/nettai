//! A signaling server in process, for tests: the Worker's protocol
//! (`crate::signal`, `signaling/src/index.ts`), its rooms in memory, a
//! thread a client. It picks a new room's code as the Worker does, and
//! refuses a code no room has. [`Server::kick`] closes a client's WebSocket,
//! as a network or the Worker's restart would; [`Server::forget`] ends a
//! room, as the Worker does ten minutes after its last socket went.

use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tungstenite::handshake::server::{Request, Response};
use tungstenite::{Message, WebSocket};

use crate::Role;
use crate::signal::{ALPHABET, CODE_LENGTH, FromServer, PING, PONG, Refusal, ToServer, VERSION, valid_code};

/// What a client's thread is told to do.
enum Out {
    Text(String),
    /// Close the socket (another of the same client took its place, or a
    /// test kicks it): the other client isn't told it went.
    Close,
}

/// The rooms, by code.
#[derive(Default)]
struct Rooms {
    rooms: HashMap<String, Room>,
    connections: u64,
}

/// A room: who has each place, and the place's socket, if it is connected
/// (its connection's number, and its thread's inbox).
#[derive(Default)]
struct Room {
    places: HashMap<Role, String>,
    sockets: HashMap<Role, (u64, Sender<Out>)>,
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
        let (r, n) = (rooms.clone(), relayed.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (r, n) = (r.clone(), n.clone());
                std::thread::spawn(move || serve(stream, version, &r, &n));
            }
        });
        Server { addr, rooms, relayed }
    }

    /// Its URL, for [`crate::Link::create_room`] and
    /// [`crate::Link::join_room`].
    pub fn url(&self) -> String {
        format!("ws://{}", self.addr)
    }

    /// The codes of the rooms there are.
    pub fn codes(&self) -> Vec<String> {
        self.rooms.lock().unwrap().rooms.keys().cloned().collect()
    }

    /// End room `code`, as its time running out does: its code names no
    /// room from now, and the sockets still in it close.
    pub fn forget(&self, code: &str) {
        let mut rooms = self.rooms.lock().unwrap();
        if let Some(room) = rooms.rooms.remove(code) {
            for (_, tx) in room.sockets.values() {
                let _ = tx.send(Out::Close);
            }
        }
    }

    /// Close the WebSocket of room `code`'s client of `role` (it comes back
    /// on its own).
    pub fn kick(&self, code: &str, role: Role) {
        let rooms = self.rooms.lock().unwrap();
        if let Some((_, tx)) = rooms.rooms.get(code).and_then(|r| r.sockets.get(&role)) {
            let _ = tx.send(Out::Close);
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
    // A new room is /rooms; a room, /rooms/CODE.
    let Some(code) = path.strip_prefix("/rooms").map(|rest| rest.trim_matches('/').to_ascii_uppercase()) else {
        return refuse(&mut ws, Refusal::BadRequest, "a new room is /rooms?peer=ID&v=2, a room /rooms/CODE?peer=ID&v=2");
    };
    if asked("v") != version.to_string() {
        return refuse(&mut ws, Refusal::Version, &format!("this server speaks version {version} of the signaling protocol"));
    }
    let peer = asked("peer");
    if peer.is_empty() {
        return refuse(&mut ws, Refusal::BadRequest, "a peer's ID is 1 to 64 letters, digits, _ and -");
    }
    let (tx, rx) = mpsc::channel();
    // Its place: the one it had, else the first free.
    let (code, role, id) = {
        let mut all = rooms.lock().unwrap();
        all.connections += 1;
        let id = all.connections;
        let code = if code.is_empty() {
            // A new room, under a code no room has.
            let code = std::iter::repeat_with(new_code).find(|c| !all.rooms.contains_key(c)).expect("a free code");
            all.rooms.insert(code.clone(), Room::default());
            code
        } else if valid_code(&code) && all.rooms.contains_key(&code) {
            code
        } else {
            drop(all);
            return refuse(&mut ws, Refusal::NotFound, "no room has that code");
        };
        let room = all.rooms.get_mut(&code).expect("the room");
        let had = room.places.iter().find(|(_, p)| **p == peer).map(|(r, _)| *r);
        let Some(role) = had.or_else(|| [Role::Host, Role::Join].into_iter().find(|r| !room.places.contains_key(r))) else {
            drop(all);
            return refuse(&mut ws, Refusal::Full, "the room is full");
        };
        room.places.insert(role, peer);
        if let Some((_, old)) = room.sockets.insert(role, (id, tx)) {
            let _ = old.send(Out::Close);
        }
        let there = room.sockets.get(&other(role));
        let _ = ws.send(Message::text(said(&FromServer::Welcome { role, peer: there.is_some(), code: code.clone() })));
        if let Some((_, them)) = there {
            let _ = them.send(Out::Text(said(&FromServer::Peer { present: true })));
        }
        (code, role, id)
    };
    let left = relay(&mut ws, &rx, rooms, &code, role, relayed);
    // Gone: its socket, unless another took its place already; its place
    // too, if it left; and the room, once both have left.
    let mut all = rooms.lock().unwrap();
    let Some(room) = all.rooms.get_mut(&code) else { return };
    if room.sockets.get(&role).is_some_and(|(i, _)| *i == id) {
        room.sockets.remove(&role);
        if let Some((_, them)) = room.sockets.get(&other(role)) {
            let _ = them.send(Out::Text(said(&FromServer::Peer { present: false })));
        }
    }
    if left {
        room.places.remove(&role);
        if room.places.is_empty() {
            all.rooms.remove(&code);
        }
    }
}

/// Relay a client's signals to the other, and the other's to it, until it
/// goes; whether it left (gave its place up).
fn relay(ws: &mut WebSocket<TcpStream>, rx: &Receiver<Out>, rooms: &Mutex<Rooms>, code: &str, role: Role, relayed: &AtomicU64) -> bool {
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
                    if let Some((_, them)) = all.rooms.get(code).and_then(|r| r.sockets.get(&other(role))) {
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
