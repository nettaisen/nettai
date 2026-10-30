//! One WebRTC connection and its data channels, on `crate::rtc`'s one API on
//! both targets: nettai-rtc's own natively (`platform::native::rtc`, on
//! str0m: its sockets, timers, ICE, DTLS and SCTP driven on a thread of this
//! crate's) and the browser's on wasm32 (`platform::web::rtc`: its
//! `RTCPeerConnection` and data channels). So this module is the same on
//! both targets: each backend puts what it learns in a queue (natively from
//! the connections' thread; on the web from the browser's handlers and
//! Promises, on the page's one thread), and the calls here take from it
//! ([`crate::rtc`]'s `poll`), never waiting, from the caller's frame loop.
//!
//! **Candidates and servers** are the backend's: natively a host candidate on
//! each of the machine's interfaces, one STUN server asked for the address the
//! Internet sees, a relayed address on up to two TURN servers (over UDP), their
//! names resolved off the caller's thread; the browser does the same its own
//! way. [`Config::loopback`] adds, for each host candidate, the same port on
//! 127.0.0.1 (two peers on one machine whatever its network).
//!
//! **Two channels.** A message's bytes are all there is of it: no text or
//! binary type tells them apart. So a connection has two channels, both
//! negotiated out of band (no in-band announcement), opened together when SCTP
//! is up:
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
//! one from the other side comes as its bytes.

use std::collections::VecDeque;

use crate::rtc::{self, ChannelId, ChannelOptions, Configuration, Event, SdpType, State};
use crate::{Candidate, Config, Error, IceServer, Instant, Message, PeerEvent};

/// The game's channel's label (both sides make it, negotiated: the label is
/// for show), and the link's.
const GAME_LABEL: &str = "nettai";
const LINK_LABEL: &str = "nettai-link";

/// The game's channel: negotiated out of band as stream 0, unordered, no
/// retransmits.
const GAME_CHANNEL: ChannelOptions = ChannelOptions { stream: 0, ordered: false, max_retransmits: Some(0) };

/// The link's own channel: negotiated out of band as stream 1, reliable and
/// ordered.
const LINK_CHANNEL: ChannelOptions = ChannelOptions { stream: 1, ordered: true, max_retransmits: None };

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

/// One WebRTC connection and its channels (the crate's docs).
pub struct PeerConnection {
    pub(crate) pc: rtc::PeerConnection,
    game: ChannelId,
    link: ChannelId,
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
        let mut c = PeerConnection::new(configuration(config, servers), config)?;
        c.pc.set_local_description(SdpType::Offer).map_err(err("can't make an offer"))?;
        Ok(c)
    }

    pub(crate) fn answer_with(config: &Config, servers: &[IceServer], offer: &str) -> Result<PeerConnection, Error> {
        let mut c = PeerConnection::new(configuration(config, servers), config)?;
        c.take_remote(SdpType::Offer, offer)?;
        c.pc.set_local_description(SdpType::Answer).map_err(err("can't make an answer"))?;
        Ok(c)
    }

    /// A connection of `configuration` (its descriptions to come) with its
    /// two channels.
    pub(crate) fn new(configuration: Configuration, config: &Config) -> Result<PeerConnection, Error> {
        let mut pc = rtc::PeerConnection::new(configuration).map_err(err("can't make a connection"))?;
        let game = pc.create_channel(GAME_LABEL, GAME_CHANNEL).map_err(err("can't make the game's data channel"))?;
        let link = pc.create_channel(LINK_LABEL, LINK_CHANNEL).map_err(err("can't make the link's data channel"))?;
        Ok(PeerConnection {
            pc,
            game,
            link,
            events: VecDeque::new(),
            inbox: VecDeque::new(),
            open: false,
            held_open: false,
            down: false,
            outage: None,
            loopback: config.loopback && !config.relay_only,
            looped: Vec::new(),
            remote: false,
            early: Vec::new(),
        })
    }

    /// The other side's description, taken: the candidates that waited for it
    /// go.
    pub(crate) fn take_remote(&mut self, type_: SdpType, sdp: &str) -> Result<(), Error> {
        let what = match type_ {
            SdpType::Offer => "can't take the offer",
            SdpType::Answer => "can't take the answer",
        };
        self.pc.set_remote_description(type_, sdp).map_err(err(what))?;
        self.remote = true;
        for c in std::mem::take(&mut self.early) {
            self.add_candidate(&c)?;
        }
        Ok(())
    }

    /// Start without descriptions (direct connect), knowing the other side's
    /// ICE credentials: the candidates that waited go.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn start_direct(&mut self, ufrag: &str, pwd: &str, dialing: bool) -> Result<(), Error> {
        self.pc.start_direct(ufrag, pwd, dialing).map_err(err("can't start the connection"))?;
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
    /// in). Natively, what the connection can't reach is left out: a
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
        if !rtc::reaches(&c.candidate) {
            return Ok(());
        }
        self.pc.add_remote_candidate(&c.candidate).map_err(err("can't take a candidate"))
    }

    /// The next event, if any.
    pub fn poll(&mut self) -> Option<PeerEvent> {
        self.pump(Instant::now());
        self.next_event()
    }

    pub(crate) fn next_event(&mut self) -> Option<PeerEvent> {
        self.events.pop_front()
    }

    /// The next message that came, if any.
    pub(crate) fn next_message(&mut self) -> Option<Message> {
        self.inbox.pop_front()
    }

    /// Take what the connection has said, and what is due: a test's outage
    /// that is over, and the channel that opened during it said open.
    pub(crate) fn pump(&mut self, now: Instant) {
        if self.outage.is_some_and(|t| now >= t) {
            self.outage = None;
            if self.held_open && !self.down {
                self.held_open = false;
                self.open = true;
                self.events.push_back(PeerEvent::Open);
            }
        }
        while let Some(e) = self.pc.poll() {
            self.event(e, now);
        }
    }

    fn event(&mut self, event: Event, now: Instant) {
        let out = self.out(now);
        match event {
            Event::Description(sdp) => self.events.push_back(PeerEvent::Description(sdp)),
            Event::Candidate(line) => self.candidate(&line),
            Event::State(state @ (State::Disconnected | State::Failed | State::Closed)) => {
                self.went_down(format!("the connection is {}", format!("{state:?}").to_lowercase()))
            }
            Event::State(_) => {}
            Event::Open(c) if c == self.game && !self.down && !self.open => {
                // (Opened during a test's outage: as if it opened after.)
                if self.outage.is_some() {
                    self.held_open = true;
                } else {
                    self.open = true;
                    self.events.push_back(PeerEvent::Open);
                }
            }
            Event::Open(_) => {}
            Event::Closed(c) => self.went_down(format!("the {} data channel closed", if c == self.game { "game's" } else { "link's" })),
            Event::Message(_, _) if out => {}
            Event::Message(c, data) if c == self.game => self.inbox.push_back(Message::Datagram(data)),
            Event::Message(_, data) => self.inbox.push_back(Message::Link(String::from_utf8_lossy(&data).into_owned())),
        }
    }

    fn out(&self, now: Instant) -> bool {
        self.outage.is_some_and(|t| now < t)
    }

    fn went_down(&mut self, why: String) {
        if !self.down {
            self.down = true;
            self.open = false;
            self.held_open = false;
            self.events.push_back(PeerEvent::Down(why));
        }
    }

    /// A local candidate, said (and its loopback twin, if asked for).
    fn candidate(&mut self, line: &str) {
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

    /// Whether what is sent now goes.
    fn sending(&self) -> bool {
        self.open && !self.down && !self.out(Instant::now())
    }

    /// Send a datagram to the other side; dropped if the channel isn't open.
    pub fn send(&mut self, data: &[u8]) {
        if self.sending() {
            let _ = self.pc.send(self.game, data);
        }
    }

    /// Send a message of the link's own on its channel; dropped if it isn't
    /// open.
    pub(crate) fn send_link(&mut self, text: &str) {
        if self.sending() {
            let _ = self.pc.send(self.link, text.as_bytes());
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
        self.open && !self.down
    }

    /// Close it (the other side hears, if the path is up).
    pub fn close(&mut self) {
        self.pc.close();
        self.open = false;
        self.held_open = false;
        self.down = true;
    }

    /// Drop every datagram sent and taken until `until` (a test's network
    /// outage): what comes is dropped as it comes, and a channel that opens
    /// meanwhile is said open once it is over, as a dial made during a real
    /// outage connects only after it.
    pub(crate) fn set_outage(&mut self, until: Option<Instant>) {
        self.outage = until;
    }
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        if !self.down {
            self.close();
        }
    }
}

/// A connection's configuration in a room: its servers, relayed only if
/// asked.
fn configuration(config: &Config, servers: &[IceServer]) -> Configuration {
    Configuration { ice_servers: servers.to_vec(), relay_only: config.relay_only, system: rtc::Options::in_room(config) }
}

#[cfg(test)]
mod tests {
    use super::*;

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
