//! A connection's sockets and its candidates: the host candidates on the
//! interfaces, a STUN server's word of the address the Internet sees
//! (server-reflexive), and a TURN server's relayed address, with the TURN
//! client that sends and takes through it (turn-client-proto, sans-I/O).

use std::io::{self, IoSliceMut};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;

use quinn_udp::{RecvMeta, UdpSockRef, UdpSocketState};
use str0m::Candidate;
use stun_proto::Instant as StunInstant;
use stun_proto::agent::{StunAgent, StunAgentPollRet, Transmit};
use stun_proto::types::attribute::XorMappedAddress;
use stun_proto::types::message::{BINDING, Message, MessageClass, MessageWriteVec, TransactionId};
use stun_proto::types::prelude::*;
use tokio::io::Interest;
use turn_client_proto::api::{TurnClientApi, TurnConfig, TurnEvent, TurnPollRet, TurnRecvRet};
use turn_client_proto::types::{TransportType, TurnCredentials};
use turn_client_proto::udp::TurnClientUdp;

use crate::IceServer;
use crate::rtc::{Configuration, Error};

/// The most TURN servers a connection allocates on (libjuice's limit too).
const MAX_RELAYS: usize = 2;

/// One UDP socket, on every interface of its family or on one address, its
/// datagrams read with their destination address.
pub(super) struct Sock {
    pub(super) socket: tokio::net::UdpSocket,
    state: UdpSocketState,
    /// The address bound (unspecified: every interface) and its port.
    local: SocketAddr,
}

impl Sock {
    /// Bound to `addr` (an IPv6 one IPv6 alone). Needs the runtime's context.
    fn bind(addr: SocketAddr) -> io::Result<Sock> {
        use socket2::{Domain, Protocol, Socket, Type};
        let socket = Socket::new(Domain::for_address(addr), Type::DGRAM, Some(Protocol::UDP))?;
        if addr.is_ipv6() {
            socket.set_only_v6(true)?;
        }
        socket.bind(&addr.into())?;
        let socket: std::net::UdpSocket = socket.into();
        let state = UdpSocketState::new((&socket).into())?;
        socket.set_nonblocking(true)?;
        let local = socket.local_addr()?;
        Ok(Sock { socket: tokio::net::UdpSocket::from_std(socket)?, state, local })
    }

    /// Each datagram that came, until none is left: `f(from, to, data)`.
    pub(super) fn recv_all(&self, buf: &mut [u8], mut f: impl FnMut(SocketAddr, SocketAddr, &[u8])) {
        loop {
            let mut meta = [RecvMeta::default()];
            let got = self.socket.try_io(Interest::READABLE, || {
                let mut bufs = [IoSliceMut::new(buf)];
                self.state.recv(UdpSockRef::from(&self.socket), &mut bufs, &mut meta)
            });
            match got {
                Ok(n) => {
                    for m in &meta[..n] {
                        let to = SocketAddr::new(m.dst_ip.unwrap_or(self.local.ip()), self.local.port());
                        // (Received coalesced, on Linux: `stride` bytes each.)
                        for datagram in buf[..m.len].chunks(m.stride.max(1)) {
                            f(m.addr, to, datagram);
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return,
                // (Windows says a datagram sent earlier went nowhere.)
                Err(e) if e.kind() == io::ErrorKind::ConnectionReset => continue,
                Err(e) => {
                    log::warn!("can't read the connection's socket: {e}");
                    return;
                }
            }
        }
    }

    fn send(&self, data: &[u8], to: SocketAddr) {
        // (A full buffer drops it, as the path might.)
        let _ = self.socket.try_send_to(data, to);
    }
}

/// Send `data` to `to` on socket `i`, if the connection still has it (it
/// lets them go when it closes).
fn send_on(socks: &[Arc<Sock>], i: usize, data: &[u8], to: SocketAddr) {
    if let Some(s) = socks.get(i) {
        s.send(data, to);
    }
}

/// A candidate found: one to say to the other side, or one str0m takes
/// datagrams on but nobody is told of (127.0.0.1).
pub(super) enum Found {
    Said(Candidate),
    Unsaid(Candidate),
}

/// What a datagram that came was.
pub(super) enum Came {
    /// The STUN or TURN servers' own: taken here.
    Taken,
    /// The other side's, through a relay: as if it came from `peer` to
    /// `relayed`.
    Relayed { peer: SocketAddr, relayed: SocketAddr, data: Vec<u8> },
    /// The other side's, straight to the socket.
    Direct,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ServerKind {
    Stun,
    Turn { username: String, credential: String },
}

/// A STUN or TURN server, before its name is resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Server {
    kind: ServerKind,
    pub(super) host: String,
    pub(super) port: u16,
}

/// The servers `servers` say that a connection uses: `stun:` (unless it is
/// relayed only), and `turn:` over UDP with its credentials (the first two),
/// each once.
pub(super) fn servers(servers: &[IceServer], relay_only: bool) -> Vec<Server> {
    let mut out: Vec<Server> = Vec::new();
    for server in servers {
        for url in &server.urls {
            let Some((scheme, rest)) = url.split_once(':') else { continue };
            let (address, query) = rest.trim_start_matches("//").split_once('?').unwrap_or((rest.trim_start_matches("//"), ""));
            let transport = query.split('&').find_map(|kv| kv.strip_prefix("transport="));
            let kind = match scheme {
                "stun" if !relay_only => ServerKind::Stun,
                "turn" if transport.is_none_or(|t| t.eq_ignore_ascii_case("udp")) && !server.username.is_empty() => {
                    ServerKind::Turn { username: server.username.clone(), credential: server.credential.clone() }
                }
                _ => continue,
            };
            let Some((host, port)) = host_port(address) else { continue };
            let s = Server { kind, host, port };
            let relays = out.iter().filter(|s| matches!(s.kind, ServerKind::Turn { .. })).count();
            if !out.contains(&s) && !(matches!(s.kind, ServerKind::Turn { .. }) && relays == MAX_RELAYS) {
                out.push(s);
            }
        }
    }
    out
}

/// A URL's `host[:port]` (an IPv6 address in brackets), 3478 by default.
fn host_port(address: &str) -> Option<(String, u16)> {
    const DEFAULT: u16 = 3478;
    if let Some(rest) = address.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        let port = match after.strip_prefix(':') {
            Some(p) => p.parse().ok()?,
            None if after.is_empty() => DEFAULT,
            None => return None,
        };
        return Some((host.to_string(), port));
    }
    match address.rsplit_once(':') {
        Some((host, port)) => Some((host.to_string(), port.parse().ok()?)),
        None if !address.is_empty() => Some((address.to_string(), DEFAULT)),
        None => None,
    }
}

/// An interface address worth a host candidate: not the loopback address,
/// a link-local one or (IPv6) a unique local one.
fn usable(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => !v.is_loopback() && !v.is_link_local() && !v.is_unspecified(),
        IpAddr::V6(v) => {
            let first = v.segments()[0];
            !v.is_loopback() && !v.is_unspecified() && first & 0xffc0 != 0xfe80 && first & 0xfe00 != 0xfc00 && v.to_ipv4_mapped().is_none()
        }
    }
}

/// A binding request out to a STUN server, its transaction and retries
/// stun-proto's.
struct Query {
    agent: StunAgent,
    id: TransactionId,
    server: SocketAddr,
    sock: usize,
    /// The agent's clock counts from this.
    base: Instant,
    next: Instant,
}

impl Query {
    /// A binding request to `server` from socket `sock` (whose address
    /// toward it is `local`), sent.
    fn send(sock: usize, local: SocketAddr, server: SocketAddr, now: Instant, socks: &[Arc<Sock>]) -> Option<Query> {
        let request = Message::builder_request(BINDING, MessageWriteVec::new()).finish();
        let id = Message::from_bytes(&request).ok()?.transaction_id();
        let agent = StunAgent::builder(TransportType::Udp, local).remote_addr(server).build();
        let mut q = Query { agent, id, server, sock, base: now, next: now };
        let at = q.at(now);
        let t = q.agent.send_request(request, server, at).ok()?;
        send_on(socks, sock, t.data.as_ref(), t.to);
        q.poll(now, socks).then_some(q)
    }

    fn at(&self, now: Instant) -> StunInstant {
        StunInstant::from_nanos(now.saturating_duration_since(self.base).as_nanos() as i64)
    }

    /// Send it again if due; false once it has timed out.
    fn poll(&mut self, now: Instant, socks: &[Arc<Sock>]) -> bool {
        let at = self.at(now);
        while let Some(t) = self.agent.poll_transmit(at) {
            send_on(socks, self.sock, t.data.as_ref(), t.to);
        }
        match self.agent.poll(at) {
            StunAgentPollRet::WaitUntil(t) => {
                self.next = t.to_std(self.base);
                true
            }
            _ => false,
        }
    }
}

/// A TURN allocation: its client, the socket it talks to the server on, and
/// the relayed address once it has one.
struct Relay {
    client: TurnClientUdp,
    sock: usize,
    server: SocketAddr,
    local: SocketAddr,
    /// The TURN client's clock counts from this.
    base: Instant,
    relayed: Option<SocketAddr>,
    next: Option<Instant>,
}

impl Relay {
    fn at(&self, now: Instant) -> StunInstant {
        StunInstant::from_nanos(now.saturating_duration_since(self.base).as_nanos() as i64)
    }

    /// What the client has to send, and what it found.
    fn pump(&mut self, now: Instant, socks: &[Arc<Sock>], found: &mut Vec<Found>) {
        let at = self.at(now);
        self.next = match self.client.poll(at) {
            TurnPollRet::WaitUntil(t) => Some(t.to_std(self.base)),
            TurnPollRet::Closed => None,
            _ => self.next,
        };
        while let Some(t) = self.client.poll_transmit(at) {
            send_on(socks, self.sock, t.data.as_ref(), t.to);
        }
        while let Some(event) = self.client.poll_event() {
            match event {
                TurnEvent::AllocationCreated(TransportType::Udp, relayed) if self.relayed.is_none() => match Candidate::relayed(relayed, self.local, "udp") {
                    Ok(c) => {
                        self.relayed = Some(relayed);
                        found.push(Found::Said(c));
                    }
                    Err(e) => log::warn!("the TURN server's relayed address {relayed} is no candidate: {e}"),
                },
                TurnEvent::AllocationCreateFailed(family) => log::warn!("no {family:?} allocation on the TURN server {}", self.server),
                TurnEvent::PermissionCreateFailed(_, ip) => log::warn!("the TURN server {} gave no permission for {ip}", self.server),
                _ => {}
            }
        }
    }
}

/// The sockets, the candidates found on them and the servers asked.
pub(super) struct Io {
    pub(super) socks: Vec<Arc<Sock>>,
    servers: Vec<Server>,
    /// Whether the servers' names were handed out to resolve.
    resolving: bool,
    queries: Vec<Query>,
    reflexive: Vec<SocketAddr>,
    relays: Vec<Relay>,
}

impl Io {
    /// The sockets `config` binds, and the host candidates on them (none
    /// for a relay-only one). Needs the runtime's context.
    pub(super) fn new(config: &Configuration) -> Result<(Io, Vec<Found>), Error> {
        let binds = match config.system.bind {
            Some(addr) => vec![addr],
            None => vec![SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)), SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0))],
        };
        let mut socks = Vec::new();
        for (i, addr) in binds.iter().enumerate() {
            match Sock::bind(*addr) {
                Ok(s) => socks.push(Arc::new(s)),
                // (A machine without IPv6 still has IPv4.)
                Err(e) if config.system.bind.is_none() && i > 0 => log::info!("no IPv6 socket: {e}"),
                Err(e) => return Err(Error::new(format!("can't bind UDP {addr}: {e}"))),
            }
        }
        let relay_only = config.relay_only;
        let mut found = Vec::new();
        if !relay_only {
            let interfaces: Vec<IpAddr> = if_addrs::get_if_addrs().unwrap_or_default().iter().map(|i| i.ip()).collect();
            for sock in &socks {
                let local = sock.local;
                let mut ips: Vec<IpAddr> = if local.ip().is_unspecified() {
                    interfaces.iter().copied().filter(|ip| ip.is_ipv4() == local.is_ipv4() && usable(*ip)).collect()
                } else {
                    vec![local.ip()]
                };
                ips.dedup();
                let loopback = if local.is_ipv4() { IpAddr::V4(Ipv4Addr::LOCALHOST) } else { IpAddr::V6(Ipv6Addr::LOCALHOST) };
                // (Only the loopback address: said, as the one way in.)
                let say_loopback = ips.is_empty() && local.ip().is_unspecified();
                for ip in ips {
                    if let Ok(c) = Candidate::host(SocketAddr::new(ip, local.port()), "udp") {
                        found.push(Found::Said(c));
                    }
                }
                if local.ip().is_unspecified()
                    && (local.is_ipv4() || say_loopback)
                    && let Ok(c) = Candidate::host(SocketAddr::new(loopback, local.port()), "udp")
                {
                    found.push(if say_loopback { Found::Said(c) } else { Found::Unsaid(c) });
                }
            }
        }
        let servers = servers(&config.ice_servers, relay_only);
        Ok((Io { socks, servers, resolving: false, queries: Vec::new(), reflexive: Vec::new(), relays: Vec::new() }, found))
    }

    /// Start asking the servers, once: when the description is made, as a
    /// browser gathers. One named by its address is asked at once; the
    /// others' names are handed out to resolve (each its index).
    pub(super) fn start(&mut self, now: Instant, found: &mut Vec<Found>) -> Vec<(usize, String, u16)> {
        if std::mem::replace(&mut self.resolving, true) {
            return Vec::new();
        }
        let mut names = Vec::new();
        for (i, server) in self.servers.clone().iter().enumerate() {
            match server.host.parse::<IpAddr>() {
                Ok(ip) => found.extend(self.resolved(i, &[SocketAddr::new(ip, server.port)], now)),
                Err(_) => names.push((i, server.host.clone(), server.port)),
            }
        }
        names
    }

    /// Server `index`'s addresses: ask it (STUN on each family's socket it
    /// has an address for; TURN on the first).
    pub(super) fn resolved(&mut self, index: usize, addrs: &[SocketAddr], now: Instant) -> Vec<Found> {
        let mut found = Vec::new();
        let Some(server) = self.servers.get(index).cloned() else { return found };
        match server.kind {
            ServerKind::Stun => {
                for sock in 0..self.socks.len() {
                    let Some(addr) = addrs.iter().copied().find(|a| a.is_ipv4() == self.socks[sock].local.is_ipv4()) else { continue };
                    let Some(local) = self.route(sock, addr) else { continue };
                    self.queries.extend(Query::send(sock, local, addr, now, &self.socks));
                }
            }
            ServerKind::Turn { username, credential } => {
                let Some((sock, addr)) = addrs.iter().find_map(|a| self.socks.iter().position(|s| s.local.is_ipv4() == a.is_ipv4()).map(|i| (i, *a))) else {
                    log::warn!("no socket of the TURN server {}'s family", server.host);
                    return found;
                };
                let Some(local) = self.route(sock, addr) else {
                    log::warn!("no route to the TURN server {addr}");
                    return found;
                };
                let config = TurnConfig::new(TurnCredentials::new(&username, &credential));
                let mut relay = Relay { client: TurnClientUdp::allocate(local, addr, config), sock, server: addr, local, base: now, relayed: None, next: None };
                relay.pump(now, &self.socks, &mut found);
                self.relays.push(relay);
            }
        }
        found
    }

    /// The local address socket `sock` reaches `to` from: its own, or (on
    /// every interface) the one the system routes through.
    fn route(&self, sock: usize, to: SocketAddr) -> Option<SocketAddr> {
        let local = self.socks.get(sock)?.local;
        if !local.ip().is_unspecified() {
            return Some(local);
        }
        let probe = std::net::UdpSocket::bind(SocketAddr::new(local.ip(), 0)).ok()?;
        probe.connect(to).ok()?;
        Some(SocketAddr::new(probe.local_addr().ok()?.ip(), local.port()))
    }

    /// A datagram that came on socket `sock` from `from` to `to`.
    pub(super) fn came(&mut self, sock: usize, from: SocketAddr, to: SocketAddr, data: &[u8], now: Instant, found: &mut Vec<Found>) -> Came {
        if let Ok(m) = Message::from_bytes(data)
            && m.is_response()
            && let Some(i) = self.queries.iter().position(|q| q.id == m.transaction_id() && q.server == from)
        {
            let mut q = self.queries.remove(i);
            let at = q.at(now);
            if q.agent.handle_stun_message_with_time(&m, from, at)
                && m.has_class(MessageClass::Success)
                && let Ok(mapped) = m.attribute::<XorMappedAddress>()
            {
                let (mapped, base) = (mapped.addr(m.transaction_id()), to);
                if mapped != base && !self.reflexive.contains(&mapped) {
                    self.reflexive.push(mapped);
                    match Candidate::server_reflexive(mapped, base, "udp") {
                        Ok(c) => found.push(Found::Said(c)),
                        Err(e) => log::warn!("the STUN server's word {mapped} is no candidate: {e}"),
                    }
                }
            }
            return Came::Taken;
        }
        let Some(relay) = self.relays.iter_mut().find(|r| r.sock == sock && r.server == from) else { return Came::Direct };
        let at = relay.at(now);
        let came = match relay.client.recv(Transmit::new(data, TransportType::Udp, from, relay.local), at) {
            TurnRecvRet::PeerData(d) => match relay.relayed {
                Some(relayed) => Came::Relayed { peer: d.peer, relayed, data: d.data().to_vec() },
                None => Came::Taken,
            },
            TurnRecvRet::Ignored(_) => Came::Direct,
            _ => Came::Taken,
        };
        relay.pump(now, &self.socks, found);
        came
    }

    /// Send `data` from candidate `source` to `to`: through the relay whose
    /// address `source` is, or from the socket of `to`'s family.
    pub(super) fn send(&mut self, source: SocketAddr, to: SocketAddr, data: &[u8], now: Instant, found: &mut Vec<Found>) {
        if let Some(relay) = self.relays.iter_mut().find(|r| r.relayed == Some(source)) {
            let at = relay.at(now);
            if !relay.client.have_permission(TransportType::Udp, to.ip()) {
                // (This one is lost: ICE sends its checks again.)
                let _ = relay.client.create_permission(TransportType::Udp, to.ip(), at);
            } else if let Ok(Some(t)) = relay.client.send_to(TransportType::Udp, to, data, at) {
                let t = t.build();
                send_on(&self.socks, relay.sock, &t.data, t.to);
            }
            relay.pump(now, &self.socks, found);
            return;
        }
        // (The unsaid loopback candidate talks to the loopback address
        // alone, and the others don't.)
        if source.ip().is_loopback() != to.ip().is_loopback() {
            return;
        }
        if let Some(s) = self.socks.iter().find(|s| s.local.is_ipv4() == to.is_ipv4()) {
            s.send(data, to);
        }
    }

    /// Ask every relay for a permission for `ip` (one of the other side's
    /// candidates), before ICE checks it.
    pub(super) fn permit(&mut self, ip: IpAddr, now: Instant, found: &mut Vec<Found>) {
        for relay in &mut self.relays {
            if relay.relayed.is_some() && !relay.client.have_permission(TransportType::Udp, ip) {
                let at = relay.at(now);
                let _ = relay.client.create_permission(TransportType::Udp, ip, at);
                relay.pump(now, &self.socks, found);
            }
        }
    }

    /// When something here is next due.
    pub(super) fn next(&self) -> Option<Instant> {
        self.queries.iter().map(|q| q.next).chain(self.relays.iter().filter_map(|r| r.next)).min()
    }

    /// What is due at `now`: STUN requests sent again (or given up), the
    /// TURN clients' timers.
    pub(super) fn due(&mut self, now: Instant, found: &mut Vec<Found>) {
        let socks = &self.socks;
        self.queries.retain_mut(|q| now < q.next || q.poll(now, socks));
        for relay in &mut self.relays {
            if relay.next.is_some_and(|t| now >= t) {
                relay.pump(now, &self.socks, found);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(urls: &[&str], username: &str) -> IceServer {
        let credential = if username.is_empty() { "" } else { "pw:+/=@%" };
        IceServer { urls: urls.iter().map(|u| u.to_string()).collect(), username: username.into(), credential: credential.into() }
    }

    #[test]
    fn urls_say_stun_and_udp_turn_servers() {
        let s = servers(
            &[
                server(&["stun:stun.example.com:3478", "stun:[2001:db8::1]"], ""),
                server(&["turn:turn.example.com:3478?transport=udp", "turn:turn.example.com:3478?transport=tcp", "turns:turn.example.com:5349?transport=tcp", "turn:other.example.com"], "u:1"),
            ],
            false,
        );
        let hosts: Vec<(&str, u16, bool)> = s.iter().map(|s| (s.host.as_str(), s.port, matches!(s.kind, ServerKind::Turn { .. }))).collect();
        assert_eq!(hosts, [("stun.example.com", 3478, false), ("2001:db8::1", 3478, false), ("turn.example.com", 3478, true), ("other.example.com", 3478, true)]);
        assert_eq!(s[2].kind, ServerKind::Turn { username: "u:1".into(), credential: "pw:+/=@%".into() });
    }

    #[test]
    fn relayed_only_asks_no_stun_server_and_turn_needs_credentials() {
        let s = servers(&[server(&["stun:stun.example.com"], ""), server(&["turn:turn.example.com"], "")], true);
        assert!(s.is_empty());
    }
}
