//! A TURN server for tests, on loopback over UDP: as much of RFC 5766 as
//! `rtc`'s client asks of one. Long-term credentials (one username and
//! password: a request without them, or with others, is told 401 and the
//! realm and nonce), an allocation a client with a relayed address of its
//! own, permissions (all granted), Send and Data indications, channels and
//! their data. It counts the datagrams it relays, each way.

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use rtc::stun::attributes::{ATTR_MESSAGE_INTEGRITY, ATTR_NONCE, ATTR_REALM, ATTR_USERNAME};
use rtc::stun::error_code::{CODE_UNAUTHORIZED, ErrorCodeAttribute};
use rtc::stun::integrity::MessageIntegrity;
use rtc::stun::message::{
    CLASS_ERROR_RESPONSE, CLASS_INDICATION, CLASS_REQUEST, CLASS_SUCCESS_RESPONSE, Getter, METHOD_ALLOCATE, METHOD_BINDING, METHOD_CHANNEL_BIND,
    METHOD_CREATE_PERMISSION, METHOD_DATA, METHOD_REFRESH, METHOD_SEND, Message, MessageType, Setter,
};
use rtc::stun::textattrs::TextAttribute;
use rtc::stun::xoraddr::XorMappedAddress;
use rtc::turn::proto::channum::ChannelNumber;
use rtc::turn::proto::data::Data;
use rtc::turn::proto::lifetime::Lifetime;
use rtc::turn::proto::peeraddr::PeerAddress;
use rtc::turn::proto::relayaddr::RelayedAddress;

const REALM: &str = "nettai.test";
const NONCE: &str = "nonce";

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
}

impl TurnServer {
    pub fn start(username: &str, password: &str) -> TurnServer {
        let socket = UdpSocket::bind("127.0.0.1:0").expect("a loopback port");
        socket.set_nonblocking(true).unwrap();
        let addr = socket.local_addr().unwrap();
        let relayed = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let mut server = Serve { socket, username: username.into(), password: password.into(), allocations: HashMap::new(), relayed: relayed.clone() };
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
    password: String,
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
            self.from_client(from, &buf[..n]);
        }
        for (client, a) in &self.allocations {
            while let Ok((n, peer)) = a.relay.recv_from(&mut buf) {
                any = true;
                self.relayed.fetch_add(1, Ordering::Relaxed);
                let out = match a.channels.iter().find(|(_, p)| **p == peer) {
                    Some((number, _)) => {
                        let mut out = number.to_be_bytes().to_vec();
                        out.extend((n as u16).to_be_bytes());
                        out.extend(&buf[..n]);
                        out
                    }
                    None => {
                        let mut m = Message::new();
                        let setters: Vec<Box<dyn Setter>> = vec![
                            Box::new(rtc::stun::message::TransactionId::new()),
                            Box::new(MessageType::new(METHOD_DATA, CLASS_INDICATION)),
                            Box::new(PeerAddress { ip: peer.ip(), port: peer.port() }),
                            Box::new(Data(buf[..n].to_vec())),
                        ];
                        m.build(&setters).unwrap();
                        m.raw
                    }
                };
                let _ = self.socket.send_to(&out, client);
            }
        }
        any
    }

    fn from_client(&mut self, from: SocketAddr, datagram: &[u8]) {
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
        let mut m = Message::new();
        m.raw = datagram.to_vec();
        if m.decode().is_err() {
            return;
        }
        if m.typ == MessageType::new(METHOD_SEND, CLASS_INDICATION) {
            let (mut peer, mut data) = (PeerAddress::default(), Data::default());
            if let Some(a) = self.allocations.get(&from)
                && peer.get_from(&m).is_ok()
                && data.get_from(&m).is_ok()
            {
                let _ = a.relay.send_to(&data.0, SocketAddr::new(peer.ip, peer.port));
                self.relayed.fetch_add(1, Ordering::Relaxed);
            }
            return;
        }
        if m.typ.class != CLASS_REQUEST {
            return;
        }
        let method = m.typ.method;
        let mapped = XorMappedAddress { ip: from.ip(), port: from.port() };
        if method == METHOD_BINDING {
            self.respond(from, &m, vec![Box::new(mapped)], false);
            return;
        }
        // Long-term credentials: a request without them, or with others,
        // is told the realm and the nonce.
        let provider = rtc::crypto::default_provider().unwrap();
        let key = MessageIntegrity::long_term_integrity_key(self.username.clone(), REALM.into(), self.password.clone(), provider.crypto()).unwrap();
        let username = TextAttribute::get_from_as(&m, ATTR_USERNAME).map(|u| u.text).unwrap_or_default();
        if !m.contains(ATTR_MESSAGE_INTEGRITY) || username != self.username || MessageIntegrity::check(&mut m.clone(), &key, provider.crypto()).is_err() {
            let mut out = Message::new();
            let setters: Vec<Box<dyn Setter>> = vec![
                Box::new(m.transaction_id),
                Box::new(MessageType::new(method, CLASS_ERROR_RESPONSE)),
                Box::new(ErrorCodeAttribute { code: CODE_UNAUTHORIZED, reason: b"Unauthorized".to_vec() }),
                Box::new(TextAttribute::new(ATTR_REALM, REALM.into())),
                Box::new(TextAttribute::new(ATTR_NONCE, NONCE.into())),
            ];
            out.build(&setters).unwrap();
            let _ = self.socket.send_to(&out.raw, from);
            return;
        }
        let lifetime = Lifetime(Duration::from_secs(600));
        let attributes: Vec<Box<dyn Setter>> = if method == METHOD_ALLOCATE {
            let a = self.allocations.entry(from).or_insert_with(|| {
                let relay = UdpSocket::bind("127.0.0.1:0").unwrap();
                relay.set_nonblocking(true).unwrap();
                let relayed = relay.local_addr().unwrap();
                Allocation { relay, relayed, channels: HashMap::new() }
            });
            vec![Box::new(RelayedAddress { ip: a.relayed.ip(), port: a.relayed.port() }), Box::new(lifetime), Box::new(mapped)]
        } else if method == METHOD_REFRESH {
            let mut asked = Lifetime::default();
            if asked.get_from(&m).is_ok() && asked.0.is_zero() {
                self.allocations.remove(&from);
            }
            vec![Box::new(lifetime)]
        } else if method == METHOD_CHANNEL_BIND {
            let (mut number, mut peer) = (ChannelNumber::default(), PeerAddress::default());
            if let Some(a) = self.allocations.get_mut(&from)
                && number.get_from(&m).is_ok()
                && peer.get_from(&m).is_ok()
            {
                a.channels.insert(number.0, SocketAddr::new(peer.ip, peer.port));
            }
            Vec::new()
        } else if method == METHOD_CREATE_PERMISSION {
            Vec::new()
        } else {
            return;
        };
        self.respond(from, &m, attributes, true);
    }

    /// A success response to `request`, with these attributes (and its
    /// integrity, for a request that had credentials).
    fn respond(&self, to: SocketAddr, request: &Message, attributes: Vec<Box<dyn Setter>>, integrity: bool) {
        let provider = rtc::crypto::default_provider().unwrap();
        let integrity = integrity.then(|| {
            MessageIntegrity::new_long_term_integrity_with_provider(self.username.clone(), REALM.into(), self.password.clone(), provider.crypto()).unwrap()
        });
        let mut setters: Vec<Box<dyn Setter + '_>> = vec![Box::new(request.transaction_id), Box::new(MessageType::new(request.typ.method, CLASS_SUCCESS_RESPONSE))];
        setters.extend(attributes);
        if let Some(i) = integrity {
            setters.push(Box::new(i));
        }
        let mut out = Message::new();
        out.build(&setters).unwrap();
        let _ = self.socket.send_to(&out.raw, to);
    }
}
