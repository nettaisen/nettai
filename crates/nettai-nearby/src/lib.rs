//! The Legacy Collection's ROMs from one device to another on the local network, with no server and nothing typed. A
//! device that has the collection offers it ([`Nearby::offer`]): it listens for a TCP connection and advertises itself
//! by DNS-SD (Bonjour: multicast DNS, the service [`SERVICE`]). One that hasn't seeks ([`Nearby::seek`]): it browses for
//! those, asks one ([`Nearby::ask`]), whose player answers ([`Nearby::answer`]: the files, or none; and may stop the
//! send, [`Nearby::stop`]), and the files arrive. Polled: [`Nearby::poll`] says what happened since, on the app's thread; the network's work is on threads of
//! this crate's.
//!
//! Discovery is the system's, a module each (`src/platform.rs`): `mdns-sd` on the desktop, which offers and seeks;
//! Apple's DNS-SD on iOS and NsdManager on Android, which seek; nothing in a browser ([`can_offer`], [`can_seek`]).
//!
//! The transfer ([`protocol`]): the asker connects to an address the offer resolved to (IPv4: the offerer listens on
//! IPv4 alone) and says its name; the offerer's app asks its player, and sends the files, or says no. One transfer at
//! a time on each side.
//!
//! Unsafe code is the systems' discovery alone: their C APIs and JNI (`src/platform/`).
#![deny(unsafe_code)]

mod platform;
pub mod protocol;
#[cfg(test)]
mod tests;
mod transfer;

use platform::system;
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The DNS-SD service nettai advertises and browses for.
pub const SERVICE: &str = "_nettai._tcp";

/// A device found offering ([`Event::Found`]), to ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Peer(u32);

/// A device's ask ([`Event::Asked`]), to answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Request(u32);

/// A file that goes: its name (no folder) and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// What happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A device offering was found (seeking), by its name.
    Found { peer: Peer, name: String },
    /// One found is gone.
    Lost { peer: Peer },
    /// A device asks for what is offered (offering): [`Nearby::answer`] it.
    Asked { request: Request, name: String },
    /// The files' bytes arrived so far, of their total (asking).
    Receiving { done: u64, total: u64 },
    /// What was asked for arrived whole, from the device named.
    Received { name: String, files: Vec<File> },
    /// The files' bytes sent so far for `request`, of their total (offering).
    Sending { request: Request, done: u64, total: u64 },
    /// What was asked for was sent whole for `request`, to the device named.
    Sent { request: Request, name: String },
    /// Sending for `request` to the device named ended before the files did: stopped ([`Nearby::stop`]), or it
    /// failed: why, for the log.
    NotSent { request: Request, name: String, why: String },
    /// The device asked said no.
    Declined { name: String },
    /// The device asked is sending already, or asking its player of another.
    Busy { name: String },
    /// The device named runs another version of this (and of nettai): its version.
    OtherVersion { name: String, version: u16 },
    /// A transfer with the device named failed, or discovery did (no name): why, for the log.
    Failed { name: String, why: String },
    /// Seeking stopped: the system doesn't allow the app to look on the local network (iOS: the player said no to
    /// its question, and can say yes in Settings > Privacy & Security > Local Network).
    NotAllowed,
}

/// Whether this system offers: the desktop's.
pub const fn can_offer() -> bool {
    system::OFFERS
}

/// Whether this system seeks: all but a browser.
pub const fn can_seek() -> bool {
    system::SEEKS
}

/// This device's name as others are told it: the computer's host name (without `.local`); on iOS UIDevice's (the
/// model's, as iOS tells apps: "iPhone"); on Android the name its player set, else its model's.
pub fn device_name() -> String {
    system::device_name()
}

/// What discovery hears, by the system's key for a service (its instance's full name).
#[derive(Clone, Debug)]
#[allow(dead_code)] // (Each system's discovery says some of these.)
pub(crate) enum Seen {
    Found { key: String, name: String, addrs: Vec<SocketAddr> },
    Lost { key: String },
    /// Browsing stopped: why.
    Failed { why: String },
    /// Browsing stopped: the system doesn't allow the app the local network (iOS, where the player said no).
    NotAllowed,
}

/// How long an offerer waits for its player's answer, and an asker for it.
pub(crate) const ANSWER_WAIT: Duration = Duration::from_secs(10 * 60);

/// The answers awaited, by request: what the offerer's connection waits on.
pub(crate) type Answers = Arc<Mutex<HashMap<u32, Sender<Option<Vec<File>>>>>>;

/// The offerer's connections, by request, from the ask till the send's end: what [`Nearby::stop`] closes.
pub(crate) type Connections = Arc<Mutex<HashMap<u32, TcpStream>>>;

/// Offering, seeking and the transfers: one an app.
pub struct Nearby {
    name: String,
    /// Whether an offer is advertised (the tests' isn't: they ask by the port).
    advertise: bool,
    tell: Sender<Event>,
    heard: Receiver<Event>,
    seen_tell: Sender<Seen>,
    seen: Receiver<Seen>,
    offering: Option<Offering>,
    seeking: Option<system::Browsing>,
    /// The devices found: by peer, the service's key, its name and addresses; by key, the peer.
    peers: HashMap<u32, (String, String, Vec<SocketAddr>)>,
    by_key: HashMap<String, u32>,
    next: u32,
    answers: Answers,
    connections: Connections,
    next_request: Arc<AtomicU32>,
    /// A transfer is under way on this side (one at a time).
    busy: Arc<AtomicBool>,
}

/// The listener and its advertisement, while offering.
struct Offering {
    stop: Arc<AtomicBool>,
    _advertised: Option<system::Advertised>,
    #[cfg_attr(not(test), allow(dead_code))]
    port: u16,
}

impl Drop for Offering {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Default for Nearby {
    fn default() -> Nearby {
        Nearby::new()
    }
}

impl Nearby {
    pub fn new() -> Nearby {
        Nearby::named(device_name(), true)
    }

    /// One named `name`, whose offers are advertised or not.
    pub(crate) fn named(name: String, advertise: bool) -> Nearby {
        let (tell, heard) = mpsc::channel();
        let (seen_tell, seen) = mpsc::channel();
        Nearby {
            name,
            advertise,
            tell,
            heard,
            seen_tell,
            seen,
            offering: None,
            seeking: None,
            peers: HashMap::new(),
            by_key: HashMap::new(),
            next: 0,
            answers: Arc::default(),
            connections: Arc::default(),
            next_request: Arc::default(),
            busy: Arc::default(),
        }
    }

    /// Offer, or stop: advertise and listen while on (where the system offers: [`can_offer`]; elsewhere nothing).
    /// What fails is said as [`Event::Failed`].
    pub fn offer(&mut self, on: bool) {
        if !on {
            self.offering = None;
            return;
        }
        if self.offering.is_some() || self.advertise && !can_offer() {
            return;
        }
        match self.listen() {
            Ok(offering) => self.offering = Some(offering),
            Err(why) => self.say_failed(String::new(), format!("offering: {why}")),
        }
    }

    /// The listener, its thread, and the advertisement of its port.
    fn listen(&self) -> Result<Offering, String> {
        let listener = TcpListener::bind(("0.0.0.0", 0)).map_err(|e| format!("listening: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let advertised = if self.advertise { Some(system::advertise(&self.name, port)?) } else { None };
        let stop = Arc::new(AtomicBool::new(false));
        let offerer = transfer::Offerer {
            tell: self.tell.clone(),
            answers: self.answers.clone(),
            connections: self.connections.clone(),
            next: self.next_request.clone(),
            busy: self.busy.clone(),
        };
        let stopped = stop.clone();
        std::thread::Builder::new().name("nettai-nearby-offer".into()).spawn(move || offerer.accept(listener, &stopped)).map_err(|e| format!("no thread: {e}"))?;
        Ok(Offering { stop, _advertised: advertised, port })
    }

    /// The port offered on (the tests' way to ask without discovery).
    #[cfg(test)]
    pub(crate) fn port(&self) -> Option<u16> {
        self.offering.as_ref().map(|o| o.port)
    }

    /// Seek, or stop: browse while on (where the system seeks: [`can_seek`]), devices found and lost said as events.
    pub fn seek(&mut self, on: bool) {
        if !on {
            self.seeking = None;
            return;
        }
        if self.seeking.is_none() && can_seek() {
            match system::browse(self.seen_tell.clone()) {
                Ok(browsing) => self.seeking = Some(browsing),
                Err(why) => self.say_failed(String::new(), format!("browsing: {why}")),
            }
        }
    }

    /// Ask `peer` for what it offers: its answer and the files come as events.
    pub fn ask(&mut self, peer: Peer) {
        let Some((_, name, addrs)) = self.peers.get(&peer.0).cloned() else { return };
        self.ask_at(name, addrs);
    }

    /// Ask the device at `addrs`, named `name`.
    pub(crate) fn ask_at(&mut self, name: String, addrs: Vec<SocketAddr>) {
        if self.busy.swap(true, Ordering::Relaxed) {
            self.say_failed(name, "a transfer is under way already".into());
            return;
        }
        let (me, tell, busy) = (self.name.clone(), self.tell.clone(), self.busy.clone());
        let spawned = std::thread::Builder::new().name("nettai-nearby-ask".into()).spawn(move || {
            let event = transfer::receive(&me, &addrs, &name, &tell).unwrap_or_else(|why| Event::Failed { name: name.clone(), why });
            busy.store(false, Ordering::Relaxed);
            let _ = tell.send(event);
        });
        if let Err(e) = spawned {
            self.busy.store(false, Ordering::Relaxed);
            self.say_failed(String::new(), format!("no thread: {e}"));
        }
    }

    /// Answer `request`: the files to send, or none (declined).
    pub fn answer(&mut self, request: Request, files: Option<Vec<File>>) {
        if let Some(waiting) = self.answers.lock().unwrap_or_else(|e| e.into_inner()).remove(&request.0) {
            let _ = waiting.send(files);
        }
    }

    /// Stop `request`: one not answered yet is declined; one being sent ends where it is, said as [`Event::NotSent`].
    pub fn stop(&mut self, request: Request) {
        let waiting = self.answers.lock().unwrap_or_else(|e| e.into_inner()).remove(&request.0);
        match waiting {
            Some(waiting) => {
                let _ = waiting.send(None);
            }
            None => {
                if let Some(connection) = self.connections.lock().unwrap_or_else(|e| e.into_inner()).get(&request.0) {
                    // (The send's next write fails.)
                    let _ = connection.shutdown(std::net::Shutdown::Both);
                }
            }
        }
    }

    /// What happened since the last poll, in order.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(seen) = self.seen.try_recv() {
            match seen {
                Seen::Found { key, name, addrs } => {
                    let addrs: Vec<SocketAddr> = addrs.into_iter().filter(SocketAddr::is_ipv4).collect();
                    if addrs.is_empty() {
                        continue;
                    }
                    let id = *self.by_key.entry(key.clone()).or_insert_with(|| {
                        self.next += 1;
                        self.next
                    });
                    if self.peers.insert(id, (key, name.clone(), addrs)).is_none() {
                        events.push(Event::Found { peer: Peer(id), name });
                    }
                }
                Seen::Lost { key } => {
                    if let Some(id) = self.by_key.remove(&key) {
                        self.peers.remove(&id);
                        events.push(Event::Lost { peer: Peer(id) });
                    }
                }
                Seen::Failed { why } => {
                    self.seeking = None;
                    events.push(Event::Failed { name: String::new(), why: format!("browsing: {why}") });
                }
                Seen::NotAllowed => {
                    self.seeking = None;
                    events.push(Event::NotAllowed);
                }
            }
        }
        events.extend(self.heard.try_iter());
        events
    }

    fn say_failed(&self, name: String, why: String) {
        let _ = self.tell.send(Event::Failed { name, why });
    }
}
