//! The native backend: a WebRTC connection on the `rtc` crate, which does
//! no I/O of its own (sans-I/O), so its sockets and timers are this
//! module's. All of it runs on the caller's thread, without waiting: each
//! call pumps the connection (the datagrams that came, the timers that are
//! due, what it has to send, its events), and the game calls in every
//! frame, which is often enough for ICE, DTLS and SCTP. Only what may
//! block runs on threads of its own: the signaling WebSocket (`ws`) and,
//! once a link, resolving the STUN and TURN servers' names.
//!
//! **Candidates.** `rtc` gathers nothing: a connection binds a UDP socket on
//! each address of the machine's interfaces (IPv4, and IPv6's global ones;
//! the loopback address where there is no other, or where asked) and
//! offers each as a host candidate, since `rtc` tells which candidate a
//! datagram came to by the address it came to. On the socket the system
//! routes to the STUN server it asks its address as the Internet sees it
//! (a server-reflexive candidate), and with a TURN server it allocates a
//! relayed address there (a relay candidate), what it sends from that
//! address going through the TURN server (`rtc`'s TURN client, sans-I/O
//! too). Direct connect's sockets are another matter (`direct`).

pub(crate) mod direct;
pub(crate) mod ws;

use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

use bytes::BytesMut;
use rtc::data_channel::{RTCDataChannelId, RTCDataChannelInit};
use rtc::peer_connection::configuration::setting_engine::{SettingEngine, SettingEngineBuilder};
use rtc::peer_connection::event::{RTCDataChannelEvent, RTCPeerConnectionEvent};
use rtc::peer_connection::message::{RTCMessage, TaggedRTCMessage};
use rtc::peer_connection::sdp::RTCSessionDescription;
use rtc::peer_connection::state::{RTCIceConnectionState, RTCPeerConnectionState};
use rtc::peer_connection::transport::{
    CandidateConfig, CandidateHostConfig, CandidateRelayConfig, CandidateServerReflexiveConfig, RTCDtlsRole, RTCIceCandidate,
    RTCIceCandidateInit,
};
use rtc::peer_connection::{RTCPeerConnection, RTCPeerConnectionBuilder};
use rtc::sansio::Protocol;
use rtc::shared::{TaggedBytesMut, TransportContext, TransportProtocol};
use rtc::turn::client::{Client as TurnClient, ClientConfig as TurnConfig, Event as TurnEvent};

use crate::{Candidate, Config, Error, PeerEvent};

/// The data channel's label (both sides make it, negotiated: the label is
/// for show).
pub(crate) const LABEL: &str = "nettai";

/// The largest datagram taken.
pub(crate) const MAX_DATAGRAM: usize = 64 * 1024;

/// The game's channel: negotiated out of band as stream 0, unordered, no
/// retransmits.
pub(crate) fn channel_init() -> RTCDataChannelInit {
    RTCDataChannelInit { ordered: false, max_retransmits: Some(0), negotiated: Some(0), ..Default::default() }
}

fn err(what: &str) -> impl Fn(rtc::shared::error::Error) -> Error + '_ {
    move |e| Error(format!("{what}: {e}"))
}

/// The STUN and TURN servers of a [`Config`], their names resolved (which
/// may wait on the system's resolver: done once, when a link starts, not
/// each time it connects).
#[derive(Clone, Debug, Default)]
pub(crate) struct Servers {
    stun: Vec<SocketAddr>,
    turn: Vec<Turn>,
}

#[derive(Clone, Debug)]
struct Turn {
    addr: SocketAddr,
    url: String,
    username: String,
    credential: String,
}

impl Servers {
    /// `config`'s servers, resolved: those whose URL this backend can't use
    /// (`turns:`, TURN over TCP) or whose name doesn't resolve are left out.
    pub(crate) fn resolve(config: &Config) -> Servers {
        let mut servers = Servers::default();
        for server in &config.ice_servers {
            for url in &server.urls {
                let Some((scheme, rest)) = url.split_once(':') else { continue };
                let (host, query) = rest.split_once('?').unwrap_or((rest, ""));
                if query.contains("transport=tcp") {
                    continue;
                }
                let host = host.trim_start_matches("//");
                let with_port = if host.rsplit_once(':').is_some_and(|(_, p)| p.parse::<u16>().is_ok()) && !host.ends_with(']') {
                    host.to_string()
                } else {
                    format!("{host}:3478")
                };
                // (IPv4 first: the sockets it is asked from are.)
                let Ok(addrs) = with_port.to_socket_addrs() else { continue };
                let mut addrs: Vec<SocketAddr> = addrs.collect();
                addrs.sort_by_key(|a| !a.is_ipv4());
                let Some(&addr) = addrs.first() else { continue };
                match scheme {
                    "stun" => servers.stun.push(addr),
                    "turn" => servers.turn.push(Turn { addr, url: url.clone(), username: server.username.clone(), credential: server.credential.clone() }),
                    _ => {}
                }
            }
        }
        servers
    }
}

/// A socket a connection sends and takes on, and the address its datagrams
/// are told by (the candidate's).
struct Sock {
    socket: UdpSocket,
    local: SocketAddr,
    /// Read it in each pump (a direct host's own reader hands its
    /// datagrams in instead).
    read: bool,
}

/// The STUN and TURN client on the socket the system routes to the servers:
/// its server-reflexive address, and its relayed one.
struct Gatherer {
    client: TurnClient,
    sock: usize,
    servers: Vec<SocketAddr>,
    mapped: Option<SocketAddr>,
    relayed: Option<SocketAddr>,
    turn_url: Option<String>,
    stun_url: Option<String>,
}

/// One WebRTC connection and its data channel (the crate's docs).
pub struct PeerConnection {
    pc: RTCPeerConnection,
    channel: RTCDataChannelId,
    socks: Vec<Sock>,
    gatherer: Option<Gatherer>,
    events: VecDeque<PeerEvent>,
    inbox: VecDeque<Vec<u8>>,
    /// The connection is up (DTLS too), and its channel open.
    connected: bool,
    open: bool,
    down: bool,
    /// Whether the other's description is in (candidates wait for it).
    remote: bool,
    early: Vec<Candidate>,
    /// Drop everything sent and taken until then (tests: a network
    /// outage).
    outage: Option<Instant>,
    buf: Vec<u8>,
}

impl PeerConnection {
    /// The dialing side's connection, made at once: its offer comes as the
    /// first event, then its candidates. `config`'s servers are resolved
    /// now (which may wait; a [`crate::Link`] resolves them once).
    pub fn offer(config: &Config) -> Result<PeerConnection, Error> {
        PeerConnection::offer_with(config, &Servers::resolve(config))
    }

    /// The answering side's connection to `offer`: its answer comes as the
    /// first event, then its candidates.
    pub fn answer(config: &Config, offer: &str) -> Result<PeerConnection, Error> {
        PeerConnection::answer_with(config, &Servers::resolve(config), offer)
    }

    pub(crate) fn offer_with(config: &Config, servers: &Servers) -> Result<PeerConnection, Error> {
        let pc = RTCPeerConnectionBuilder::new().with_setting_engine(settings(config).build()).build(Instant::now()).map_err(err("can't make a connection"))?;
        let mut c = PeerConnection::new(pc, Vec::new())?;
        let offer = c.pc.create_offer(None).map_err(err("can't make an offer"))?;
        c.pc.set_local_description(Instant::now(), offer.clone()).map_err(err("can't take the offer"))?;
        c.events.push_back(PeerEvent::Description(offer.sdp));
        c.gather(config, servers)?;
        Ok(c)
    }

    pub(crate) fn answer_with(config: &Config, servers: &Servers, offer: &str) -> Result<PeerConnection, Error> {
        let pc = RTCPeerConnectionBuilder::new()
            .with_setting_engine(answering(config).build())
            .build(Instant::now())
            .map_err(err("can't make a connection"))?;
        let mut c = PeerConnection::new(pc, Vec::new())?;
        c.take_offer(offer)?;
        let answer = c.pc.create_answer(None).map_err(err("can't make an answer"))?;
        c.pc.set_local_description(Instant::now(), answer.clone()).map_err(err("can't take the answer"))?;
        c.events.push_back(PeerEvent::Description(answer.sdp));
        c.gather(config, servers)?;
        Ok(c)
    }

    /// A connection on `pc` (its descriptions to come) with the game's
    /// channel, on these sockets.
    fn new(mut pc: RTCPeerConnection, socks: Vec<Sock>) -> Result<PeerConnection, Error> {
        let channel = pc.create_data_channel(LABEL, Some(channel_init())).map_err(err("can't make the data channel"))?.id();
        Ok(PeerConnection {
            pc,
            channel,
            socks,
            gatherer: None,
            events: VecDeque::new(),
            inbox: VecDeque::new(),
            connected: false,
            open: false,
            down: false,
            remote: false,
            early: Vec::new(),
            outage: None,
            buf: vec![0; MAX_DATAGRAM],
        })
    }

    /// The other side's offer, taken (the answering side's channel is made
    /// before it: the same negotiated one).
    fn take_offer(&mut self, offer: &str) -> Result<(), Error> {
        let offer = RTCSessionDescription::offer(offer.to_string()).map_err(err("the offer doesn't read"))?;
        self.pc.set_remote_description(Instant::now(), offer).map_err(err("can't take the offer"))?;
        self.remote = true;
        Ok(())
    }

    /// The other side's answer to this side's offer.
    pub fn set_answer(&mut self, answer: &str) -> Result<(), Error> {
        let answer = RTCSessionDescription::answer(answer.to_string()).map_err(err("the answer doesn't read"))?;
        self.pc.set_remote_description(Instant::now(), answer).map_err(err("can't take the answer"))?;
        self.remote = true;
        for c in std::mem::take(&mut self.early) {
            self.add_candidate(&c)?;
        }
        Ok(())
    }

    /// One of the other side's candidates (kept until its description is
    /// in). What this backend can't reach is left out: a browser's
    /// candidate under an mDNS name (`.local`), and TCP.
    pub fn add_candidate(&mut self, c: &Candidate) -> Result<(), Error> {
        if !self.remote {
            self.early.push(c.clone());
            return Ok(());
        }
        let fields: Vec<&str> = c.candidate.split_whitespace().collect();
        if fields.get(2).is_some_and(|p| !p.eq_ignore_ascii_case("udp")) || fields.get(4).is_some_and(|a| a.ends_with(".local")) || c.candidate.is_empty() {
            return Ok(());
        }
        let init = RTCIceCandidateInit { candidate: c.candidate.clone(), sdp_mid: c.mid.clone(), sdp_mline_index: c.mline, ..Default::default() };
        self.pc.add_remote_candidate(init).map_err(err("can't take a candidate"))?;
        // (Permission for the relay to pass what comes from it.)
        if let (Some(g), Some(addr)) = (&mut self.gatherer, candidate_addr(&c.candidate))
            && let Some(relayed) = g.relayed
            && let Ok(mut relay) = g.client.relay(relayed)
        {
            let _ = relay.create_permission(Instant::now(), addr);
        }
        Ok(())
    }

    /// The sockets and this side's candidates: a host candidate a socket,
    /// and the STUN and TURN servers asked on the one routed to them.
    fn gather(&mut self, config: &Config, servers: &Servers) -> Result<(), Error> {
        let mut ips: Vec<IpAddr> = if_addrs::get_if_addrs()
            .unwrap_or_default()
            .into_iter()
            .filter(|i| !i.is_loopback() && !i.is_link_local())
            .map(|i| i.ip())
            .filter(|ip| match ip {
                IpAddr::V4(_) => true,
                // (Global unicast only: no link-local scopes, no ULAs.)
                IpAddr::V6(v6) => v6.segments()[0] & 0xe000 == 0x2000,
            })
            .collect();
        ips.dedup();
        if config.loopback || ips.is_empty() {
            ips.push(IpAddr::V4(Ipv4Addr::LOCALHOST));
        }
        for ip in ips {
            let Ok(socket) = UdpSocket::bind((ip, 0)) else { continue };
            if socket.set_nonblocking(true).is_err() {
                continue;
            }
            let Ok(local) = socket.local_addr() else { continue };
            self.socks.push(Sock { socket, local, read: true });
        }
        if self.socks.is_empty() {
            return Err(Error("can't bind a UDP socket on any of this machine's addresses".into()));
        }
        for i in 0..self.socks.len() {
            let local = self.socks[i].local;
            self.add_local(host(local))?;
        }
        // STUN and TURN, on the socket routed to the first server (IPv4: a
        // server's address is resolved IPv4 first).
        let turn = servers.turn.first();
        let stun = servers.stun.iter().find(|a| a.is_ipv4()).copied();
        let Some(towards) = turn.map(|t| t.addr).or(stun) else { return Ok(()) };
        let routed = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).and_then(|s| s.connect(towards).and_then(|_| s.local_addr())).ok();
        let sock = routed.and_then(|r| self.socks.iter().position(|s| s.local.ip() == r.ip())).or_else(|| self.socks.iter().position(|s| s.local.is_ipv4()));
        let Some(sock) = sock else { return Ok(()) };
        let mut turn_config = TurnConfig { local_addr: self.socks[sock].local, transport_protocol: TransportProtocol::UDP, ..TurnConfig::default() };
        if let Some(stun) = stun {
            turn_config.stun_serv_addr = stun.to_string();
        }
        if let Some(t) = turn {
            turn_config.turn_serv_addr = t.addr.to_string();
            turn_config.username = t.username.clone();
            turn_config.password = t.credential.clone();
        }
        let provider = rtc::crypto::default_provider().map_err(|e| Error(format!("no crypto: {e}")))?;
        let mut client = TurnClient::new(turn_config, provider).map_err(err("can't make the STUN client"))?;
        let now = Instant::now();
        if stun.is_some() {
            let _ = client.send_binding_request(now);
        }
        if turn.is_some() {
            let _ = client.allocate(now);
        }
        self.gatherer = Some(Gatherer {
            client,
            sock,
            servers: stun.into_iter().chain(turn.map(|t| t.addr)).collect(),
            mapped: None,
            relayed: None,
            turn_url: turn.map(|t| t.url.clone()),
            stun_url: stun.map(|a| format!("stun:{a}")),
        });
        Ok(())
    }

    /// Add a local candidate, and say it.
    fn add_local(&mut self, init: RTCIceCandidateInit) -> Result<(), Error> {
        let said = Candidate { candidate: init.candidate.clone(), mid: Some("0".into()), mline: Some(0) };
        self.pc.add_local_candidate(init).map_err(err("can't add a candidate"))?;
        self.events.push_back(PeerEvent::Candidate(said));
        Ok(())
    }

    /// Pump, then the next event, if any.
    pub fn poll(&mut self) -> Option<PeerEvent> {
        self.pump(Instant::now());
        self.events.pop_front()
    }

    /// The next event, if any, without pumping.
    pub(crate) fn next_event(&mut self) -> Option<PeerEvent> {
        self.events.pop_front()
    }

    /// The next datagram that came, if any, without pumping.
    /// Whether a datagram came that wasn't taken.
    pub(crate) fn has_datagram(&self) -> bool {
        !self.inbox.is_empty()
    }

    pub(crate) fn next_datagram(&mut self) -> Option<Vec<u8>> {
        self.inbox.pop_front()
    }

    /// Send a datagram to the other side; dropped if the channel isn't open.
    pub fn send(&mut self, data: &[u8]) {
        let now = Instant::now();
        if self.open
            && !self.outage.is_some_and(|t| now < t)
            && let Some(mut dc) = self.pc.data_channel(self.channel)
        {
            let _ = dc.send(now, BytesMut::from(data));
            self.flush(now);
        }
    }

    /// The next datagram that came from the other side, if one has.
    pub fn recv(&mut self) -> Option<Vec<u8>> {
        if self.inbox.is_empty() {
            self.pump(Instant::now());
        }
        self.inbox.pop_front()
    }

    pub fn is_open(&self) -> bool {
        self.open && !self.down
    }

    /// Close it (the other side hears, if the path is up).
    pub fn close(&mut self) {
        let _ = self.pc.close();
        self.flush(Instant::now());
        self.open = false;
        self.down = true;
    }

    /// Drop every datagram sent and taken until `until` (a test's network
    /// outage).
    pub(crate) fn set_outage(&mut self, until: Option<Instant>) {
        self.outage = until;
    }

    /// Hand in a datagram that came to `local` from `from` (the direct
    /// host's reader).
    pub(crate) fn feed(&mut self, now: Instant, local: SocketAddr, from: SocketAddr, data: &[u8]) {
        if self.outage.is_some_and(|t| now < t) {
            return;
        }
        let _ = self.pc.handle_read(tagged(now, local, from, data));
    }

    /// Everything that is due: what came, the timers, the events, what to
    /// send.
    pub(crate) fn pump(&mut self, now: Instant) {
        let out = self.outage.is_some_and(|t| now < t);
        for i in 0..self.socks.len() {
            if !self.socks[i].read {
                continue;
            }
            loop {
                let (n, from) = match self.socks[i].socket.recv_from(&mut self.buf) {
                    Ok(got) => got,
                    // (A datagram the other end refused, on some systems.)
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue,
                    Err(_) => break,
                };
                if out {
                    continue;
                }
                let local = self.socks[i].local;
                match &mut self.gatherer {
                    Some(g) if g.sock == i && g.servers.contains(&from) => {
                        let _ = g.client.handle_read(tagged(now, local, from, &self.buf[..n]));
                    }
                    _ => {
                        let _ = self.pc.handle_read(tagged(now, local, from, &self.buf[..n]));
                    }
                }
            }
        }
        self.pump_gatherer(now);
        let _ = self.pc.handle_timeout(now);
        while let Some(event) = self.pc.poll_event() {
            self.event(event);
        }
        // A negotiated channel's open is passed on with the next datagram
        // that comes (`rtc` 0.21: the open waits in its data channel
        // handler's queue, which only an inbound datagram drains), up to
        // two seconds later, at the ICE keepalive; an empty one, which it
        // drops, drains it at once.
        if self.connected && !self.open && !self.down {
            let local = self.socks.first().map_or(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)), |s| s.local);
            let _ = self.pc.handle_read(tagged(now, local, local, &[]));
            while let Some(event) = self.pc.poll_event() {
                self.event(event);
            }
        }
        while let Some(TaggedRTCMessage { message, .. }) = self.pc.poll_read() {
            if let RTCMessage::DataChannelMessage(id, m) = message
                && id == self.channel
                && !out
            {
                self.inbox.push_back(m.data.to_vec());
            }
        }
        self.flush(now);
    }

    fn event(&mut self, event: RTCPeerConnectionEvent) {
        let down = |why: String| PeerEvent::Down(why);
        match event {
            RTCPeerConnectionEvent::OnConnectionStateChangeEvent(RTCPeerConnectionState::Connected) => self.connected = true,
            RTCPeerConnectionEvent::OnDataChannel(RTCDataChannelEvent::OnOpen(id)) if id == self.channel && !self.down => {
                self.open = true;
                self.events.push_back(PeerEvent::Open);
            }
            RTCPeerConnectionEvent::OnDataChannel(RTCDataChannelEvent::OnClose(id)) if id == self.channel => {
                self.go_down(down("the data channel closed".into()));
            }
            RTCPeerConnectionEvent::OnConnectionStateChangeEvent(
                s @ (RTCPeerConnectionState::Disconnected | RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed),
            ) => self.go_down(down(format!("the connection is {}", s.to_string().to_lowercase()))),
            RTCPeerConnectionEvent::OnIceConnectionStateChangeEvent(
                s @ (RTCIceConnectionState::Disconnected | RTCIceConnectionState::Failed | RTCIceConnectionState::Closed),
            ) => self.go_down(down(format!("ICE is {}", s.to_string().to_lowercase()))),
            _ => {}
        }
    }

    fn go_down(&mut self, event: PeerEvent) {
        if !self.down {
            self.down = true;
            self.open = false;
            self.events.push_back(event);
        }
    }

    /// The STUN and TURN client's due work and what it found: the
    /// server-reflexive and relay candidates, the relay's permissions, what
    /// came through the relay.
    fn pump_gatherer(&mut self, now: Instant) {
        let Some(g) = &mut self.gatherer else { return };
        let _ = g.client.handle_timeout(now);
        let mut found = Vec::new();
        while let Some(event) = g.client.poll_event() {
            match event {
                TurnEvent::BindingResponse(_, mapped) if g.mapped.is_none() => {
                    g.mapped = Some(mapped);
                    let base = self.socks[g.sock].local;
                    found.push(srflx(mapped, base, g.stun_url.clone()));
                }
                TurnEvent::AllocateResponse(_, relayed) => {
                    g.relayed = Some(relayed);
                    let base = g.mapped.unwrap_or(self.socks[g.sock].local);
                    found.push(relay(relayed, base, g.turn_url.clone()));
                }
                TurnEvent::DataIndicationOrChannelData(_, from, data) => {
                    if let Some(relayed) = g.relayed {
                        let _ = self.pc.handle_read(tagged(now, relayed, from, &data));
                    }
                }
                _ => {}
            }
        }
        while let Some(t) = g.client.poll_write() {
            let _ = self.socks[g.sock].socket.send_to(&t.message, t.transport.peer_addr);
        }
        for init in found.into_iter().flatten() {
            let _ = self.add_local(init);
        }
    }

    /// Send what the connection has to send, each datagram from the socket
    /// of the address it names (or through the relay, from the relayed
    /// address).
    fn flush(&mut self, now: Instant) {
        let out = self.outage.is_some_and(|t| now < t);
        while let Some(w) = self.pc.poll_write() {
            if out {
                continue;
            }
            let (from, to) = (w.transport.local_addr, w.transport.peer_addr);
            if let Some(g) = &mut self.gatherer
                && g.relayed == Some(from)
            {
                if let Ok(mut relay) = g.client.relay(from)
                    && relay.send_to(now, &w.message, to).is_err()
                {
                    let _ = relay.create_permission(now, to);
                }
                continue;
            }
            let sock = self.socks.iter().find(|s| s.local == from).or_else(|| self.socks.iter().find(|s| s.local.is_ipv4() == to.is_ipv4()));
            if let Some(s) = sock {
                let _ = s.socket.send_to(&w.message, to);
            }
        }
        if let Some(g) = &mut self.gatherer {
            while let Some(t) = g.client.poll_write() {
                if !out {
                    let _ = self.socks[g.sock].socket.send_to(&t.message, t.transport.peer_addr);
                }
            }
        }
    }
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        if !self.down {
            self.close();
        }
    }
}

/// How ICE checks: every 200 ms, for as long as a dial is given, so that a
/// dial made while the network is out connects as soon as it is back
/// (rather than failing a pair after `rtc`'s 1.4 seconds and waiting out
/// the dial).
fn settings(config: &Config) -> SettingEngineBuilder {
    const CHECK: Duration = Duration::from_millis(200);
    let checks = (config.dial_timeout.as_millis() / CHECK.as_millis()).clamp(7, u16::MAX as u128) as u16;
    SettingEngineBuilder::new().with_ice_connection_attempts(Some(CHECK), Some(checks))
}

/// The setting engine of an answering connection: the DTLS server, so that
/// the dialing side is its client.
fn answering(config: &Config) -> SettingEngineBuilder {
    settings(config).with_answering_dtls_role(RTCDtlsRole::Server)
}

/// A direct connection's setting engine: these ICE credentials, and no
/// certificate check (direct connect exchanges no descriptions: nothing
/// tells either side the other's fingerprint).
fn direct_settings(builder: SettingEngineBuilder, ufrag: &str, pwd: &str) -> SettingEngine {
    builder.with_ice_credentials(ufrag.to_string(), pwd.to_string()).with_disable_certificate_fingerprint_verification(true).build()
}

fn tagged(now: Instant, local: SocketAddr, peer: SocketAddr, data: &[u8]) -> TaggedBytesMut {
    TaggedBytesMut {
        now,
        transport: TransportContext { local_addr: local, peer_addr: peer, ecn: None, transport_protocol: TransportProtocol::UDP },
        message: BytesMut::from(data),
    }
}

fn base(addr: SocketAddr) -> CandidateConfig {
    CandidateConfig { network: "udp".into(), address: addr.ip().to_string(), port: addr.port(), component: 1, ..Default::default() }
}

/// A host candidate at `addr`.
pub(crate) fn host(addr: SocketAddr) -> RTCIceCandidateInit {
    let c = CandidateHostConfig { base_config: base(addr), ..Default::default() }.new_candidate_host().expect("a host candidate");
    init(&c, None)
}

fn srflx(mapped: SocketAddr, base_addr: SocketAddr, url: Option<String>) -> Option<RTCIceCandidateInit> {
    let c = CandidateServerReflexiveConfig { base_config: base(mapped), rel_addr: base_addr.ip().to_string(), rel_port: base_addr.port(), ..Default::default() }
        .new_candidate_server_reflexive()
        .ok()?;
    Some(init(&c, url))
}

fn relay(relayed: SocketAddr, base_addr: SocketAddr, url: Option<String>) -> Option<RTCIceCandidateInit> {
    let c = CandidateRelayConfig { base_config: base(relayed), rel_addr: base_addr.ip().to_string(), rel_port: base_addr.port(), url: url.clone() }
        .new_candidate_relay()
        .ok()?;
    Some(init(&c, url))
}

fn init(c: &rtc::ice::candidate::Candidate, url: Option<String>) -> RTCIceCandidateInit {
    let mut init = RTCIceCandidate::from(c).to_json().expect("a candidate line");
    init.sdp_mid = Some("0".into());
    init.sdp_mline_index = Some(0);
    init.url = url;
    init
}

/// The address a candidate line names (`candidate:F C udp P ADDR PORT
/// typ ...`).
fn candidate_addr(line: &str) -> Option<SocketAddr> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let ip: IpAddr = fields.get(4)?.parse().ok()?;
    let port: u16 = fields.get(5)?.parse().ok()?;
    Some(SocketAddr::new(ip, port))
}

// A connection goes to another thread with the link it is in.
const _: () = {
    const fn send<T: Send>() {}
    send::<PeerConnection>();
};
