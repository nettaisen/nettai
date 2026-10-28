//! A TURN server for tests, on loopback over UDP: as much of RFC 5766 as a
//! TURN client asks of one (turn-client-proto's, nettai-rtc's natively).
//! Long-term credentials (one username and password: a request without
//! them, or with others, is told 401 and the realm and nonce), an allocation
//! a client with a relayed address of its own, permissions (an address's,
//! every port), Send and Data indications, channels and their data. On one
//! machine every address is 127.0.0.1, so what comes to a relayed address is
//! let through only from another client's relayed address: a client's own
//! socket can't reach the other's past its own, or a test of relaying could
//! be passed on a path that isn't one. It counts the datagrams it relays,
//! each way. Its STUN is its own (RFC 8489's messages, a few attributes).

use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use hmac::{Hmac, Mac};
use md5::{Digest, Md5};

const REALM: &str = "nettai.test";
const NONCE: &str = "nonce";

const MAGIC_COOKIE: u32 = 0x2112_a442;

// Methods.
const BINDING: u16 = 0x001;
const ALLOCATE: u16 = 0x003;
const REFRESH: u16 = 0x004;
const SEND: u16 = 0x006;
const DATA: u16 = 0x007;
const CREATE_PERMISSION: u16 = 0x008;
const CHANNEL_BIND: u16 = 0x009;

// Classes.
const REQUEST: u16 = 0b00;
const INDICATION: u16 = 0b01;
const SUCCESS: u16 = 0b10;
const ERROR: u16 = 0b11;

// Attributes.
const USERNAME: u16 = 0x0006;
const MESSAGE_INTEGRITY: u16 = 0x0008;
const ERROR_CODE: u16 = 0x0009;
const CHANNEL_NUMBER: u16 = 0x000c;
const LIFETIME: u16 = 0x000d;
const XOR_PEER_ADDRESS: u16 = 0x0012;
const DATA_ATTRIBUTE: u16 = 0x0013;
const REALM_ATTRIBUTE: u16 = 0x0014;
const NONCE_ATTRIBUTE: u16 = 0x0015;
const XOR_RELAYED_ADDRESS: u16 = 0x0016;
const XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// A STUN message's type, of its method and class (RFC 8489 §5).
fn message_type(method: u16, class: u16) -> u16 {
    (method & 0x000f) | ((method & 0x0070) << 1) | ((method & 0x0f80) << 2) | ((class & 1) << 4) | ((class & 2) << 7)
}

fn method_of(typ: u16) -> u16 {
    (typ & 0x000f) | ((typ >> 1) & 0x0070) | ((typ >> 2) & 0x0f80)
}

fn class_of(typ: u16) -> u16 {
    ((typ >> 4) & 1) | ((typ >> 7) & 2)
}

/// A STUN message as it came: its type, transaction id, attributes (in
/// order, with where each starts), and its bytes.
struct Message<'a> {
    typ: u16,
    transaction: [u8; 12],
    attributes: Vec<(u16, &'a [u8], usize)>,
    raw: &'a [u8],
}

impl<'a> Message<'a> {
    fn read(raw: &'a [u8]) -> Option<Message<'a>> {
        if raw.len() < 20 || raw[0] & 0xc0 != 0 || raw[4..8] != MAGIC_COOKIE.to_be_bytes() {
            return None;
        }
        let typ = u16::from_be_bytes([raw[0], raw[1]]);
        let length = u16::from_be_bytes([raw[2], raw[3]]) as usize;
        let raw = raw.get(..20 + length)?;
        let transaction = raw[8..20].try_into().unwrap();
        let mut attributes = Vec::new();
        let mut at = 20;
        while at + 4 <= raw.len() {
            let kind = u16::from_be_bytes([raw[at], raw[at + 1]]);
            let length = u16::from_be_bytes([raw[at + 2], raw[at + 3]]) as usize;
            attributes.push((kind, raw.get(at + 4..at + 4 + length)?, at));
            at += 4 + length.next_multiple_of(4);
        }
        Some(Message { typ, transaction, attributes, raw })
    }

    fn get(&self, kind: u16) -> Option<&'a [u8]> {
        self.attributes.iter().find(|(k, _, _)| *k == kind).map(|(_, v, _)| *v)
    }

    /// Whether its MESSAGE-INTEGRITY is the one `key` makes.
    fn signed_with(&self, key: &[u8]) -> bool {
        let Some(&(_, value, at)) = self.attributes.iter().find(|(k, _, _)| *k == MESSAGE_INTEGRITY) else { return false };
        let mut signed = self.raw[..at].to_vec();
        signed[2..4].copy_from_slice(&((at + 24 - 20) as u16).to_be_bytes());
        hmac_sha1(key, &signed) == value
    }
}

/// A STUN message being made.
struct Out {
    raw: Vec<u8>,
}

impl Out {
    fn new(method: u16, class: u16, transaction: [u8; 12]) -> Out {
        let mut raw = message_type(method, class).to_be_bytes().to_vec();
        raw.extend([0, 0]);
        raw.extend(MAGIC_COOKIE.to_be_bytes());
        raw.extend(transaction);
        Out { raw }
    }

    fn add(mut self, kind: u16, value: &[u8]) -> Out {
        self.raw.extend(kind.to_be_bytes());
        self.raw.extend((value.len() as u16).to_be_bytes());
        self.raw.extend(value);
        self.raw.resize(self.raw.len().next_multiple_of(4), 0);
        let length = (self.raw.len() - 20) as u16;
        self.raw[2..4].copy_from_slice(&length.to_be_bytes());
        self
    }

    fn address(self, kind: u16, addr: SocketAddr) -> Out {
        let transaction: [u8; 12] = self.raw[8..20].try_into().unwrap();
        let value = xor_address(addr, &transaction);
        self.add(kind, &value)
    }

    /// Its MESSAGE-INTEGRITY, by `key`: the last attribute.
    fn sign(mut self, key: &[u8]) -> Out {
        let length = (self.raw.len() + 24 - 20) as u16;
        self.raw[2..4].copy_from_slice(&length.to_be_bytes());
        let mac = hmac_sha1(key, &self.raw);
        self.add(MESSAGE_INTEGRITY, &mac)
    }
}

fn hmac_sha1(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<sha1::Sha1>::new_from_slice(key).unwrap();
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// An XOR-…-ADDRESS's value (RFC 8489 §14.2).
fn xor_address(addr: SocketAddr, transaction: &[u8; 12]) -> Vec<u8> {
    let cookie = MAGIC_COOKIE.to_be_bytes();
    let port = addr.port() ^ (MAGIC_COOKIE >> 16) as u16;
    let mut value = vec![0];
    match addr.ip() {
        IpAddr::V4(ip) => {
            value.push(0x01);
            value.extend(port.to_be_bytes());
            value.extend(ip.octets().iter().zip(cookie).map(|(a, b)| a ^ b));
        }
        IpAddr::V6(ip) => {
            value.push(0x02);
            value.extend(port.to_be_bytes());
            value.extend(ip.octets().iter().zip(cookie.iter().chain(transaction)).map(|(a, b)| a ^ b));
        }
    }
    value
}

fn read_xor_address(value: &[u8], transaction: &[u8; 12]) -> Option<SocketAddr> {
    let cookie = MAGIC_COOKIE.to_be_bytes();
    let port = u16::from_be_bytes([*value.get(2)?, *value.get(3)?]) ^ (MAGIC_COOKIE >> 16) as u16;
    let ip = match value.get(1)? {
        0x01 => {
            let octets: [u8; 4] = value.get(4..8)?.iter().zip(cookie).map(|(a, b)| a ^ b).collect::<Vec<u8>>().try_into().ok()?;
            IpAddr::V4(Ipv4Addr::from(octets))
        }
        0x02 => {
            let octets: [u8; 16] = value.get(4..20)?.iter().zip(cookie.iter().chain(transaction)).map(|(a, b)| a ^ b).collect::<Vec<u8>>().try_into().ok()?;
            IpAddr::V6(Ipv6Addr::from(octets))
        }
        _ => return None,
    };
    Some(SocketAddr::new(ip, port))
}

/// The server, until dropped.
pub struct TurnServer {
    addr: SocketAddr,
    relayed: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
}

/// A client's allocation: its relayed address's socket, and its channels.
struct Allocation {
    relay: UdpSocket,
    relayed: SocketAddr,
    channels: HashMap<u16, SocketAddr>,
    /// The peers' addresses what comes is let through from.
    permitted: HashSet<IpAddr>,
}

impl TurnServer {
    pub fn start(username: &str, password: &str) -> TurnServer {
        let socket = UdpSocket::bind("127.0.0.1:0").expect("a loopback port");
        socket.set_nonblocking(true).unwrap();
        let addr = socket.local_addr().unwrap();
        let relayed = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        // The long-term key: MD5 of `username:realm:password`.
        let key = Md5::digest(format!("{username}:{REALM}:{password}")).to_vec();
        let mut server = Serve { socket, username: username.into(), key, allocations: HashMap::new(), relayed: relayed.clone() };
        let stopped = stop.clone();
        std::thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                if !server.turn() {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        });
        TurnServer { addr, relayed, stop }
    }

    /// Its URL, as a welcome gives one.
    pub fn url(&self) -> String {
        format!("turn:{}?transport=udp", self.addr)
    }

    /// The datagrams relayed so far, each way.
    pub fn relayed(&self) -> u64 {
        self.relayed.load(Ordering::Relaxed)
    }
}

impl Drop for TurnServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

struct Serve {
    socket: UdpSocket,
    username: String,
    key: Vec<u8>,
    allocations: HashMap<SocketAddr, Allocation>,
    relayed: Arc<AtomicU64>,
}

impl Serve {
    /// One turn: what came from the clients, and to their relayed
    /// addresses. Whether anything did.
    fn turn(&mut self) -> bool {
        let mut buf = vec![0; 64 * 1024];
        let mut any = false;
        while let Ok((n, from)) = self.socket.recv_from(&mut buf) {
            any = true;
            self.client_says(from, &buf[..n]);
        }
        let relayed: HashSet<SocketAddr> = self.allocations.values().map(|a| a.relayed).collect();
        for (client, a) in &self.allocations {
            while let Ok((n, peer)) = a.relay.recv_from(&mut buf) {
                any = true;
                if !a.permitted.contains(&peer.ip()) || !relayed.contains(&peer) {
                    continue;
                }
                self.relayed.fetch_add(1, Ordering::Relaxed);
                let out = match a.channels.iter().find(|(_, p)| **p == peer) {
                    Some((number, _)) => {
                        let mut out = number.to_be_bytes().to_vec();
                        out.extend((n as u16).to_be_bytes());
                        out.extend(&buf[..n]);
                        out
                    }
                    None => Out::new(DATA, INDICATION, std::array::from_fn(|i| i as u8)).address(XOR_PEER_ADDRESS, peer).add(DATA_ATTRIBUTE, &buf[..n]).raw,
                };
                let _ = self.socket.send_to(&out, client);
            }
        }
        any
    }

    /// What a client sent: channel data, an indication, a request.
    fn client_says(&mut self, from: SocketAddr, datagram: &[u8]) {
        // Channel data: to the channel's peer.
        if datagram.len() >= 4 && (0x40..0x80).contains(&datagram[0]) {
            let number = u16::from_be_bytes([datagram[0], datagram[1]]);
            let length = u16::from_be_bytes([datagram[2], datagram[3]]) as usize;
            if let Some(a) = self.allocations.get(&from)
                && let Some(peer) = a.channels.get(&number)
                && let Some(data) = datagram.get(4..4 + length)
            {
                let _ = a.relay.send_to(data, peer);
                self.relayed.fetch_add(1, Ordering::Relaxed);
            }
            return;
        }
        let Some(m) = Message::read(datagram) else { return };
        let (method, class) = (method_of(m.typ), class_of(m.typ));
        let peer = m.get(XOR_PEER_ADDRESS).and_then(|v| read_xor_address(v, &m.transaction));
        if (method, class) == (SEND, INDICATION) {
            if let Some(a) = self.allocations.get(&from)
                && let (Some(peer), Some(data)) = (peer, m.get(DATA_ATTRIBUTE))
            {
                let _ = a.relay.send_to(data, peer);
                self.relayed.fetch_add(1, Ordering::Relaxed);
            }
            return;
        }
        if class != REQUEST {
            return;
        }
        if method == BINDING {
            self.send(Out::new(method, SUCCESS, m.transaction).address(XOR_MAPPED_ADDRESS, from), from);
            return;
        }
        // Long-term credentials: a request without them, or with others,
        // is told the realm and the nonce.
        let username = m.get(USERNAME).and_then(|u| std::str::from_utf8(u).ok()).unwrap_or_default();
        if username != self.username || !m.signed_with(&self.key) {
            self.send(
                Out::new(method, ERROR, m.transaction)
                    .add(ERROR_CODE, &[0, 0, 4, 1, b'U', b'n', b'a', b'u', b't', b'h', b'o', b'r', b'i', b'z', b'e', b'd'])
                    .add(REALM_ATTRIBUTE, REALM.as_bytes())
                    .add(NONCE_ATTRIBUTE, NONCE.as_bytes()),
                from,
            );
            return;
        }
        let lifetime = 600u32.to_be_bytes();
        let out = Out::new(method, SUCCESS, m.transaction);
        let out = match method {
            ALLOCATE => {
                let a = self.allocations.entry(from).or_insert_with(|| {
                    let relay = UdpSocket::bind("127.0.0.1:0").unwrap();
                    relay.set_nonblocking(true).unwrap();
                    let relayed = relay.local_addr().unwrap();
                    Allocation { relay, relayed, channels: HashMap::new(), permitted: HashSet::new() }
                });
                out.address(XOR_RELAYED_ADDRESS, a.relayed).add(LIFETIME, &lifetime).address(XOR_MAPPED_ADDRESS, from)
            }
            REFRESH => {
                if m.get(LIFETIME) == Some(&[0, 0, 0, 0][..]) {
                    self.allocations.remove(&from);
                }
                out.add(LIFETIME, &lifetime)
            }
            CHANNEL_BIND => {
                if let Some(a) = self.allocations.get_mut(&from)
                    && let (Some(number), Some(peer)) = (m.get(CHANNEL_NUMBER), peer)
                    && number.len() >= 2
                {
                    a.channels.insert(u16::from_be_bytes([number[0], number[1]]), peer);
                    a.permitted.insert(peer.ip());
                }
                out
            }
            CREATE_PERMISSION => {
                if let Some(a) = self.allocations.get_mut(&from) {
                    let peers = m.attributes.iter().filter(|(k, _, _)| *k == XOR_PEER_ADDRESS);
                    a.permitted.extend(peers.filter_map(|(_, v, _)| read_xor_address(v, &m.transaction)).map(|p| p.ip()));
                }
                out
            }
            _ => return,
        };
        self.send(out.sign(&self.key), from);
    }

    fn send(&self, out: Out, to: SocketAddr) {
        let _ = self.socket.send_to(&out.raw, to);
    }
}
