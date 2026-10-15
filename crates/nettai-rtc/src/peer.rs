//! One WebRTC connection and its data channels, on datachannel-facade: one
//! synchronous, callback-based API over libdatachannel natively (C++, which
//! does the sockets, the timers, ICE, DTLS and SCTP on threads of its own)
//! and the browser's `RTCPeerConnection` on wasm32. The user's words: "instead
//! of nettai-rtc you should use https://github.com/tangobattle/datachannel-facade,
//! it's way more stable". So this module is the same on both targets: the
//! facade's callbacks put what they learn in a queue (natively behind a mutex,
//! since they come from libdatachannel's threads; on the web in a `RefCell`,
//! since the browser calls them on the page's one thread), and the calls here
//! take from it, never waiting, from the caller's frame loop. Nothing needs
//! pumping: there is no async runtime and no thread of this crate's.
//!
//! **Candidates and servers** are the backend's: libdatachannel (libjuice)
//! offers a host candidate on each of the machine's interfaces, asks one STUN
//! server for its address as the Internet sees it, allocates a relayed address
//! on up to two TURN servers (over UDP), and resolves their names on a thread
//! of its own; the browser does the same its own way. [`Config::loopback`]
//! adds, for each host candidate, the same port on 127.0.0.1 (two peers on one
//! machine whatever its network).
//!
//! **Two channels.** The facade's messages are bytes, with no text or binary
//! type to tell them apart. So a connection has two channels, both negotiated
//! out of band (no in-band announcement), opened together when SCTP is up:
//!
//! - the game's, stream 0: unordered and without retransmits. What a game
//!   sends is a datagram (the netplay protocol's frames, the lobby's messages
//!   before them), which tolerate loss, reordering and duplicates: a lost one
//!   is the next one's to make up, never SCTP's to resend late;
//! - the link's own, stream 1: reliable and ordered, for its few messages (the
//!   reconnect code and its acknowledgment, [`crate::Link`]).
//!
//! The channel a message came on says whose it is, so the game's datagrams
//! carry no tag: they cost what they did when the link's messages were text on
//! the one channel. (A one-byte tag on one channel was the other way, and
//! would have cost every datagram a byte.) Nothing here sends a text message;
//! one from the other side reaches the facade's libdatachannel binding with
//! the negative size libdatachannel's C API gives a string, which the binding
//! takes for a length: the facade's to fix (it should drop it, or hand it on
//! as bytes).
//!
//! **ICE servers** go to the facade as URL strings with the credentials
//! inline, libdatachannel's form (`turn:USER:PASSWORD@HOST:PORT?transport=udp`):
//! see [`ice_urls`].

use std::collections::VecDeque;

use datachannel_facade::{Configuration, DataChannel, DataChannelOptions, Description, Reliability, SdpType, State, TransportPolicy};

use crate::{Candidate, Config, Error, IceServer, Instant, Message, PeerEvent};

/// The game's channel's label (both sides make it, negotiated: the label is
/// for show), and the link's.
const GAME_LABEL: &str = "nettai";
const LINK_LABEL: &str = "nettai-link";

/// The game's channel: negotiated out of band as stream 0, unordered, no
/// retransmits.
fn game_channel() -> DataChannelOptions {
    DataChannelOptions {
        reliability: Reliability { unordered: true, unreliable: true, max_retransmits: 0, ..Reliability::default() },
        negotiated: true,
        stream: Some(0),
        ..DataChannelOptions::default()
    }
}

/// The link's own channel: negotiated out of band as stream 1, reliable and
/// ordered.
fn link_channel() -> DataChannelOptions {
    DataChannelOptions { negotiated: true, stream: Some(1), ..DataChannelOptions::default() }
}

/// What the callbacks share with the connection: natively a mutex (they come
/// from libdatachannel's threads), on the web a `RefCell` (the page's one
/// thread).
#[cfg(not(target_arch = "wasm32"))]
type Shared<T> = std::sync::Arc<std::sync::Mutex<T>>;
#[cfg(target_arch = "wasm32")]
type Shared<T> = std::rc::Rc<std::cell::RefCell<T>>;

/// `f` on what is shared. (Never call into the facade inside: a callback
/// waiting on the lock may be holding one of libdatachannel's.)
fn with<T, R>(shared: &Shared<T>, f: impl FnOnce(&mut T) -> R) -> R {
    #[cfg(not(target_arch = "wasm32"))]
    {
        f(&mut shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        f(&mut shared.borrow_mut())
    }
}

/// What the facade's callbacks found, for the next call.
#[derive(Default)]
struct Found {
    events: VecDeque<PeerEvent>,
    inbox: VecDeque<Message>,
    /// The game's channel is open (and said so).
    open: bool,
    /// It opened during an outage: it is said open when the outage is over.
    held_open: bool,
    down: bool,
    /// Drop everything sent and taken until then (tests: a network outage).
    outage: Option<Instant>,
    /// Say each host candidate's port on 127.0.0.1 too ([`Config::loopback`]),
    /// and the ports said so.
    loopback: bool,
    looped: Vec<u16>,
}

impl Found {
    fn out(&self, now: Instant) -> bool {
        self.outage.is_some_and(|t| now < t)
    }

    fn down(&mut self, why: String) {
        if !self.down {
            self.down = true;
            self.open = false;
            self.held_open = false;
            self.events.push_back(PeerEvent::Down(why));
        }
    }

    /// A local candidate, said (and its loopback twin, if asked for).
    fn candidate(&mut self, line: &str) {
        // (The end of gathering, on the web: nothing to say.)
        if line.is_empty() {
            return;
        }
        let said = |candidate: String| PeerEvent::Candidate(Candidate { candidate, mid: Some("0".into()), mline: Some(0) });
        self.events.push_back(said(line.to_string()));
        if self.loopback
            && let Some((line, port)) = loopback(line)
            && !self.looped.contains(&port)
        {
            self.looped.push(port);
            self.events.push_back(said(line));
        }
    }
}

/// A UDP host candidate's port on 127.0.0.1 (`candidate:F C udp P ADDR PORT
/// typ host ...`), a little less preferred, and the port.
fn loopback(line: &str) -> Option<(String, u16)> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let [foundation, _, transport, priority, _, port, "typ", "host", ..] = fields[..] else { return None };
    if !transport.eq_ignore_ascii_case("udp") {
        return None;
    }
    let priority: u32 = priority.parse().ok()?;
    let port: u16 = port.parse().ok()?;
    Some((format!("{foundation}lo 1 UDP {} 127.0.0.1 {port} typ host", priority.saturating_sub(1)), port))
}

fn err<E: std::fmt::Display>(what: &str) -> impl Fn(E) -> Error + '_ {
    move |e| Error::internal(format!("{what}: {e}"))
}

/// `servers`, as the facade takes them: one URL a server URL, a TURN server's
/// credentials inline (`turn:USER:PASSWORD@HOST:PORT?transport=udp`).
///
/// Natively libdatachannel reads them: it takes the username up to the first
/// `:` and the password up to the first `@`, and percent-decodes both (and
/// nothing else of the URL's), so each is percent-encoded here, everything but
/// RFC 3986's unreserved characters, which any credential survives (a TURN
/// REST username's `:`, a base64 password's `+`, `/` and `=`, an `@`, a `%`).
/// It uses `stun:` and `turn:` over UDP (no `transport`, or
/// `transport=udp`); the others (`turns:`, TURN over TCP) are left out here,
/// as it would leave them out.
///
/// On the web the facade splits them back for the browser: the username up to
/// the first `:`, the password up to the last `@`, neither decoded. So they go
/// in as they are, which any password survives, and any username without a
/// `:` (Cloudflare's are hex; a username with one can't be said through the
/// facade on the web). The browser takes every URL.
pub(crate) fn ice_urls(servers: &[IceServer]) -> Vec<String> {
    ice_urls_for(servers, cfg!(not(target_arch = "wasm32")))
}

/// [`ice_urls`], for libdatachannel (`native`) or for the facade's web split.
fn ice_urls_for(servers: &[IceServer], native: bool) -> Vec<String> {
    let mut urls = Vec::new();
    for server in servers {
        for url in &server.urls {
            let Some((scheme, rest)) = url.split_once(':') else { continue };
            if native {
                let transport = rest.split_once('?').map_or("", |(_, query)| query).split('&').find_map(|kv| kv.strip_prefix("transport="));
                let usable = match scheme {
                    "stun" => true,
                    "turn" => transport.is_none_or(|t| t.eq_ignore_ascii_case("udp")),
                    _ => false,
                };
                if !usable {
                    continue;
                }
            }
            let relayed = matches!(scheme, "turn" | "turns");
            if !relayed || server.username.is_empty() {
                urls.push(url.clone());
                continue;
            }
            let rest = rest.trim_start_matches("//");
            urls.push(match native {
                true => format!("{scheme}:{}:{}@{rest}", percent_encode(&server.username), percent_encode(&server.credential)),
                false => format!("{scheme}:{}:{}@{rest}", server.username, server.credential),
            });
        }
    }
    urls
}

/// `text` with every byte but RFC 3986's unreserved characters (letters,
/// digits, `-`, `.`, `_`, `~`) percent-encoded.
fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// One WebRTC connection and its channels (the crate's docs).
pub struct PeerConnection {
    // (The channels before the connection: they are deleted first.)
    game: DataChannel,
    link: DataChannel,
    pub(crate) pc: datachannel_facade::PeerConnection,
    found: Shared<Found>,
    /// Whether the other's description is in (candidates wait for it).
    remote: bool,
    early: Vec<Candidate>,
}

impl PeerConnection {
    /// The dialing side's connection, made at once: its offer comes as the
    /// first event, then its candidates, on `config`'s servers.
    pub fn offer(config: &Config) -> Result<PeerConnection, Error> {
        PeerConnection::offer_with(config, &config.ice_servers)
    }

    /// The answering side's connection to `offer`: its answer comes as the
    /// first event, then its candidates.
    pub fn answer(config: &Config, offer: &str) -> Result<PeerConnection, Error> {
        PeerConnection::answer_with(config, &config.ice_servers, offer)
    }

    pub(crate) fn offer_with(config: &Config, servers: &[IceServer]) -> Result<PeerConnection, Error> {
        let c = PeerConnection::new(configuration(config, servers), config)?;
        c.pc.set_local_description(Some(SdpType::Offer)).map_err(err("can't make an offer"))?;
        Ok(c)
    }

    pub(crate) fn answer_with(config: &Config, servers: &[IceServer], offer: &str) -> Result<PeerConnection, Error> {
        let mut c = PeerConnection::new(configuration(config, servers), config)?;
        c.take_remote(SdpType::Offer, offer)?;
        c.pc.set_local_description(Some(SdpType::Answer)).map_err(err("can't make an answer"))?;
        Ok(c)
    }

    /// A connection of `configuration` (its descriptions to come) with its
    /// two channels, its callbacks set.
    pub(crate) fn new(configuration: Configuration, config: &Config) -> Result<PeerConnection, Error> {
        let found: Shared<Found> = Shared::default();
        with(&found, |f| f.loopback = config.loopback && !config.relay_only);
        let mut pc = datachannel_facade::PeerConnection::new(configuration).map_err(err("can't make a connection"))?;
        let f = found.clone();
        pc.set_on_local_description(Some(move |sdp: &str, _: SdpType| {
            // (Before the candidates it has, if any came first.)
            with(&f, |f| f.events.push_front(PeerEvent::Description(sdp.to_string())));
        }));
        let f = found.clone();
        pc.set_on_local_candidate(Some(move |line: &str| with(&f, |f| f.candidate(line))));
        let f = found.clone();
        pc.set_on_state_change(Some(move |state: State| {
            if matches!(state, State::Disconnected | State::Failed | State::Closed) {
                with(&f, |f| f.down(format!("the connection is {}", format!("{state:?}").to_lowercase())));
            }
        }));
        let mut game = pc.create_data_channel(GAME_LABEL, game_channel()).map_err(err("can't make the game's data channel"))?;
        let mut link = pc.create_data_channel(LINK_LABEL, link_channel()).map_err(err("can't make the link's data channel"))?;
        let f = found.clone();
        game.set_on_open(Some(move || {
            with(&f, |f| {
                if f.down || f.open {
                    return;
                }
                // (Opened during a test's outage: as if it opened after.)
                if f.outage.is_some() {
                    f.held_open = true;
                } else {
                    f.open = true;
                    f.events.push_back(PeerEvent::Open);
                }
            })
        }));
        for (channel, which) in [(&mut game, "game's"), (&mut link, "link's")] {
            let f = found.clone();
            channel.set_on_closed(Some(move || with(&f, |f| f.down(format!("the {which} data channel closed")))));
        }
        let f = found.clone();
        game.set_on_message(Some(move |data: &[u8]| {
            with(&f, |f| {
                if !f.out(Instant::now()) {
                    f.inbox.push_back(Message::Datagram(data.to_vec()));
                }
            })
        }));
        let f = found.clone();
        link.set_on_message(Some(move |data: &[u8]| {
            with(&f, |f| {
                if !f.out(Instant::now()) {
                    f.inbox.push_back(Message::Link(String::from_utf8_lossy(data).into_owned()));
                }
            })
        }));
        Ok(PeerConnection { game, link, pc, found, remote: false, early: Vec::new() })
    }

    /// The other side's description, taken: the candidates that waited for it
    /// go.
    pub(crate) fn take_remote(&mut self, type_: SdpType, sdp: &str) -> Result<(), Error> {
        let what = match type_ {
            SdpType::Offer => "can't take the offer",
            _ => "can't take the answer",
        };
        self.pc.set_remote_description(&Description { type_, sdp: sdp.to_string() }).map_err(err(what))?;
        self.remote = true;
        for c in std::mem::take(&mut self.early) {
            self.add_candidate(&c)?;
        }
        Ok(())
    }

    /// The other side's answer to this side's offer.
    pub fn set_answer(&mut self, answer: &str) -> Result<(), Error> {
        self.take_remote(SdpType::Answer, answer)
    }

    /// One of the other side's candidates (kept until its description is
    /// in). Natively, what libdatachannel can't reach is left out: a
    /// browser's candidate under an mDNS name (`.local`), and TCP.
    pub fn add_candidate(&mut self, c: &Candidate) -> Result<(), Error> {
        if !self.remote {
            self.early.push(c.clone());
            return Ok(());
        }
        // (An empty one is the end of the candidates: not said.)
        if c.candidate.is_empty() {
            return Ok(());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let fields: Vec<&str> = c.candidate.split_whitespace().collect();
            if fields.get(2).is_some_and(|p| !p.eq_ignore_ascii_case("udp")) || fields.get(4).is_some_and(|a| a.ends_with(".local")) {
                return Ok(());
            }
        }
        self.pc.add_remote_candidate(&c.candidate).map_err(err("can't take a candidate"))
    }

    /// The next event, if any.
    pub fn poll(&mut self) -> Option<PeerEvent> {
        self.pump(Instant::now());
        self.next_event()
    }

    pub(crate) fn next_event(&mut self) -> Option<PeerEvent> {
        with(&self.found, |f| f.events.pop_front())
    }

    /// The next message that came, if any.
    pub(crate) fn next_message(&mut self) -> Option<Message> {
        with(&self.found, |f| f.inbox.pop_front())
    }

    /// What is due: a test's outage that is over, and the channel that opened
    /// during it said open. (The backend does the rest on its own.)
    pub(crate) fn pump(&mut self, now: Instant) {
        with(&self.found, |f| {
            if f.outage.is_some_and(|t| now >= t) {
                f.outage = None;
                if f.held_open && !f.down {
                    f.held_open = false;
                    f.open = true;
                    f.events.push_back(PeerEvent::Open);
                }
            }
        });
    }

    /// Whether what is sent now goes (the lock isn't held while sending).
    fn sending(&self) -> bool {
        let now = Instant::now();
        with(&self.found, |f| f.open && !f.down && !f.out(now))
    }

    /// Send a datagram to the other side; dropped if the channel isn't open.
    pub fn send(&mut self, data: &[u8]) {
        if self.sending() {
            let _ = self.game.send(data);
        }
    }

    /// Send a message of the link's own on its channel; dropped if it isn't
    /// open.
    pub(crate) fn send_link(&mut self, text: &str) {
        if self.sending() {
            let _ = self.link.send(text.as_bytes());
        }
    }

    /// The next datagram that came from the other side, if one has (the
    /// link's own messages are skipped).
    pub fn recv(&mut self) -> Option<Vec<u8>> {
        self.pump(Instant::now());
        while let Some(m) = self.next_message() {
            if let Message::Datagram(d) = m {
                return Some(d);
            }
        }
        None
    }

    pub fn is_open(&self) -> bool {
        with(&self.found, |f| f.open && !f.down)
    }

    /// Close it (the other side hears, if the path is up).
    pub fn close(&mut self) {
        let _ = self.game.close();
        let _ = self.link.close();
        let _ = self.pc.close();
        with(&self.found, |f| {
            f.open = false;
            f.held_open = false;
            f.down = true;
        });
    }

    /// Drop every datagram sent and taken until `until` (a test's network
    /// outage): what comes is dropped as it comes, and a channel that opens
    /// meanwhile is said open once it is over, as a dial made during a real
    /// outage connects only after it.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn set_outage(&mut self, until: Option<Instant>) {
        with(&self.found, |f| f.outage = until);
    }
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        if !with(&self.found, |f| f.down) {
            self.close();
        }
    }
}

/// A connection's configuration in a room: its servers, relayed only if
/// asked; natively, no description made but when asked for (as in the
/// browser).
fn configuration(config: &Config, servers: &[IceServer]) -> Configuration {
    let mut c = Configuration::new();
    c.ice_servers = ice_urls(servers);
    if config.relay_only {
        c.ice_transport_policy = TransportPolicy::Relay;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use datachannel_facade::platform::native::ConfigurationExt;
        c.set_disable_auto_negotiation(true);
    }
    c
}

// A connection goes to another thread with the link it is in (natively).
#[cfg(not(target_arch = "wasm32"))]
const _: () = {
    const fn send<T: Send>() {}
    send::<PeerConnection>();
};

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(username: &str, credential: &str) -> IceServer {
        IceServer {
            urls: vec![
                "stun:stun.example.com:3478".into(),
                "turn:turn.example.com:3478?transport=udp".into(),
                "turn:turn.example.com:3478?transport=tcp".into(),
                "turns:turn.example.com:5349?transport=tcp".into(),
            ],
            username: username.into(),
            credential: credential.into(),
        }
    }

    /// For libdatachannel, the credentials percent-encoded (it decodes them),
    /// and only the URLs it uses.
    #[test]
    fn credentials_go_inline_percent_encoded() {
        let ice_urls = |servers: &[IceServer]| ice_urls_for(servers, true);
        assert_eq!(
            ice_urls(&[turn("1700000000:nettai", "a+b/c=d@e:f%g~h")]),
            ["stun:stun.example.com:3478", "turn:1700000000%3Anettai:a%2Bb%2Fc%3Dd%40e%3Af%25g~h@turn.example.com:3478?transport=udp"],
        );
        // (Cloudflare's are hex: as they are.)
        assert_eq!(ice_urls(&[turn("0af3", "9b1e")])[1], "turn:0af3:9b1e@turn.example.com:3478?transport=udp");
        // A TURN server without credentials, and a bare one, as they are.
        assert_eq!(ice_urls(&[IceServer::new("turn:turn.example.com"), IceServer::new("stun:stun.example.com")]), ["turn:turn.example.com", "stun:stun.example.com"]);
    }

    /// For the facade's web split, as they are: it takes the username up to
    /// the first `:` and the password up to the last `@`, and decodes neither.
    #[test]
    fn on_the_web_credentials_go_inline_as_they_are() {
        assert_eq!(
            ice_urls_for(&[turn("alice", "a+b/c=d@e:f%g")], false),
            [
                "stun:stun.example.com:3478",
                "turn:alice:a+b/c=d@e:f%g@turn.example.com:3478?transport=udp",
                "turn:alice:a+b/c=d@e:f%g@turn.example.com:3478?transport=tcp",
                "turns:alice:a+b/c=d@e:f%g@turn.example.com:5349?transport=tcp",
            ],
        );
    }

    #[test]
    fn percent_encoding_leaves_the_unreserved_alone() {
        assert_eq!(percent_encode("AZaz09-._~"), "AZaz09-._~");
        assert_eq!(percent_encode(" :@/+=%é"), "%20%3A%40%2F%2B%3D%25%C3%A9");
    }

    #[test]
    fn a_host_candidate_has_a_loopback_twin() {
        assert_eq!(
            loopback("candidate:1 1 UDP 2122317823 192.168.1.5 51234 typ host"),
            Some(("candidate:1lo 1 UDP 2122317822 127.0.0.1 51234 typ host".to_string(), 51234))
        );
        assert_eq!(loopback("candidate:2 1 UDP 1686052607 203.0.113.9 40000 typ srflx raddr 0.0.0.0 rport 0"), None);
        assert_eq!(loopback("candidate:3 1 TCP 2122317823 192.168.1.5 9 typ host tcptype active"), None);
    }
}
