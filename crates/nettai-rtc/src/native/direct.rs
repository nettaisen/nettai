//! Direct connect (native only): a WebRTC connection to a host's address
//! and port, with no signaling at all. Nothing is exchanged before ICE:
//! each side makes up the other's description, which it can, since every
//! field of it is fixed but two.
//!
//! - **The ICE credentials** are fixed: the host's username fragment and
//!   both passwords constants, the joiner's fragment its nonce (drawn once
//!   a link) and the connection's generation (`j<nonce>g<generation>`).
//! - **The candidates**: the joiner is told the host's address, its one
//!   remote candidate. The host is told nothing: it learns the joiner's
//!   address from the joiner's first connectivity check (a peer-reflexive
//!   candidate). Neither side's own candidate is said to anyone, so each
//!   is a placeholder that names its socket, bound on every interface
//!   (`0.0.0.0:PORT`): `rtc` tells which candidate a datagram came to by
//!   the address it is tagged with, and the socket's datagrams are tagged
//!   with that.
//! - **SCTP and the data channel**: the same as a room's (port 5000, the
//!   channel negotiated as stream 0).
//! - **The DTLS fingerprint** is the one thing a real exchange carries that
//!   can't be made up (each side's certificate is its own, generated): it
//!   isn't checked. So direct connect authenticates nobody, as the raw UDP
//!   it replaces didn't; a room checks the fingerprints its descriptions
//!   carry.
//!
//! The joiner dials (the offer, ICE's controlling side, the DTLS client).
//! Dialing again (after a drop) is a new connection of the next
//! generation, from a new socket; the host sees a connectivity check with
//! a newer generation in its username and answers it with a new connection
//! of its own on its one socket, whose datagrams it reads itself to see
//! that ([`Listener`]). It takes only the first joiner's nonce: another
//! joiner can't take the match over.
//!
//! What crosses a NAT is what crossed it before: on a LAN, or over the
//! Internet with the host's UDP port forwarded to it. The joiner dials out,
//! so its own NAT needs nothing.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::Instant;

use rtc::peer_connection::RTCPeerConnectionBuilder;
use rtc::stun::attributes::ATTR_USERNAME;
use rtc::stun::message::{BINDING_REQUEST, Message, is_stun_message};
use rtc::stun::textattrs::TextAttribute;

use super::{PeerConnection, Sock, answering, direct_settings, err, host, settings};
use crate::{Config, Error};

const HOST_UFRAG: &str = "nettaihost";
const HOST_PWD: &str = "nettaidirectconnecthostpwd";
const JOIN_PWD: &str = "nettaidirectconnectjoinpwd";

/// The joiner's username fragment for connection `generation`.
fn join_ufrag(nonce: u32, generation: u32) -> String {
    format!("j{nonce:08x}g{generation}")
}

/// The nonce and the generation a joiner's username fragment names.
fn read_join_ufrag(ufrag: &str) -> Option<(u32, u32)> {
    let (nonce, generation) = ufrag.strip_prefix('j')?.split_once('g')?;
    Some((u32::from_str_radix(nonce, 16).ok()?, generation.parse().ok()?))
}

/// A made-up description: the one media section, the data channel's, with
/// these ICE credentials and this DTLS setup; its fingerprint a dummy
/// (never checked).
fn description(ufrag: &str, pwd: &str, setup: &str) -> String {
    let fingerprint = vec!["00"; 32].join(":");
    [
        "v=0",
        "o=- 1 1 IN IP4 0.0.0.0",
        "s=-",
        "t=0 0",
        "a=group:BUNDLE 0",
        "m=application 9 UDP/DTLS/SCTP webrtc-datachannel",
        "c=IN IP4 0.0.0.0",
        &format!("a=setup:{setup}"),
        "a=mid:0",
        &format!("a=ice-ufrag:{ufrag}"),
        &format!("a=ice-pwd:{pwd}"),
        &format!("a=fingerprint:sha-256 {fingerprint}"),
        "a=sctp-port:5000",
        "",
    ]
    .join("\r\n")
}

impl PeerConnection {
    /// The joiner's connection `generation` to the host at `host_addr`, from
    /// a new socket: dialing at once.
    pub(crate) fn direct_dial(config: &Config, host_addr: SocketAddr, nonce: u32, generation: u32) -> Result<PeerConnection, Error> {
        let any = if host_addr.is_ipv4() { SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)) } else { SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)) };
        let socket = UdpSocket::bind(any).map_err(|e| Error(format!("can't bind a UDP socket: {e}")))?;
        socket.set_nonblocking(true).map_err(|e| Error(e.to_string()))?;
        let local = socket.local_addr().map_err(|e| Error(e.to_string()))?;
        let settings = direct_settings(settings(config), &join_ufrag(nonce, generation), JOIN_PWD);
        let pc = RTCPeerConnectionBuilder::new().with_setting_engine(settings).build(Instant::now()).map_err(err("can't make a connection"))?;
        let mut c = PeerConnection::new(pc, vec![Sock { socket, local, read: true }])?;
        let offer = c.pc.create_offer(None).map_err(err("can't make an offer"))?;
        c.pc.set_local_description(Instant::now(), offer).map_err(err("can't take the offer"))?;
        c.set_answer(&description(HOST_UFRAG, HOST_PWD, "passive"))?;
        c.pc.add_local_candidate(host(local)).map_err(err("can't add a candidate"))?;
        c.pc.add_remote_candidate(host(host_addr)).map_err(err("can't add the host's address"))?;
        Ok(c)
    }

    /// The host's connection answering the joiner `nonce`'s dial
    /// `generation`, on the listener's socket (whose datagrams the listener
    /// reads and hands in).
    fn direct_answer(config: &Config, listener: &Listener, nonce: u32, generation: u32) -> Result<PeerConnection, Error> {
        let socket = listener.socket.try_clone().map_err(|e| Error(e.to_string()))?;
        let settings = direct_settings(answering(config), HOST_UFRAG, HOST_PWD);
        let pc = RTCPeerConnectionBuilder::new().with_setting_engine(settings).build(Instant::now()).map_err(err("can't make a connection"))?;
        let mut c = PeerConnection::new(pc, vec![Sock { socket, local: listener.local, read: false }])?;
        c.take_offer(&description(&join_ufrag(nonce, generation), JOIN_PWD, "actpass"))?;
        let answer = c.pc.create_answer(None).map_err(err("can't make an answer"))?;
        c.pc.set_local_description(Instant::now(), answer).map_err(err("can't take the answer"))?;
        c.pc.add_local_candidate(host(listener.local)).map_err(err("can't add a candidate"))?;
        Ok(c)
    }
}

/// The host's end of direct connect: its socket on every interface, read
/// here, and the joiner it took.
pub(crate) struct Listener {
    socket: UdpSocket,
    local: SocketAddr,
    nonce: Option<u32>,
}

impl Listener {
    /// Listen on UDP `port` of every IPv4 interface (0: a port the system
    /// picks).
    pub(crate) fn bind(port: u16) -> Result<Listener, Error> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).map_err(|e| Error(format!("can't listen on UDP port {port}: {e}")))?;
        socket.set_nonblocking(true).map_err(|e| Error(e.to_string()))?;
        let local = socket.local_addr().map_err(|e| Error(e.to_string()))?;
        Ok(Listener { socket, local, nonce: None })
    }

    pub(crate) fn port(&self) -> u16 {
        self.local.port()
    }

    /// The next datagram that came into `buf`, its length, and from where.
    pub(crate) fn recv(&mut self, buf: &mut [u8]) -> Option<(usize, SocketAddr)> {
        loop {
            match self.socket.recv_from(buf) {
                Ok(got) => return Some(got),
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue,
                Err(_) => return None,
            }
        }
    }

    /// The generation `datagram` dials, if it is a connectivity check of
    /// this match's joiner (the first joiner to check takes the match).
    pub(crate) fn dialed(&mut self, datagram: &[u8]) -> Option<u32> {
        if !is_stun_message(datagram) {
            return None;
        }
        let mut m = Message { raw: datagram.to_vec(), ..Message::default() };
        m.decode().ok()?;
        if m.typ != BINDING_REQUEST {
            return None;
        }
        let username = TextAttribute::get_from_as(&m, ATTR_USERNAME).ok()?;
        let (to, from) = username.text.split_once(':')?;
        let (nonce, generation) = read_join_ufrag(from).filter(|_| to == HOST_UFRAG)?;
        if *self.nonce.get_or_insert(nonce) != nonce {
            return None;
        }
        Some(generation)
    }

    /// The host's connection answering dial `generation` of the joiner it
    /// took.
    pub(crate) fn answer(&self, config: &Config, generation: u32) -> Result<PeerConnection, Error> {
        PeerConnection::direct_answer(config, self, self.nonce.expect("a joiner dialed"), generation)
    }

    pub(crate) fn local(&self) -> SocketAddr {
        self.local
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_joiners_fragment_names_its_dial() {
        assert_eq!(read_join_ufrag(&join_ufrag(0xdead_beef, 12)), Some((0xdead_beef, 12)));
        assert_eq!(read_join_ufrag("nettaihost"), None);
        // (At least 4 ICE characters; the passwords at least 22.)
        assert!(join_ufrag(0, 0).len() >= 4 && HOST_PWD.len() >= 22 && JOIN_PWD.len() >= 22);
    }
}
