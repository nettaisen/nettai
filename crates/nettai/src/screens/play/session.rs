//! A link to another player and the lobby over it: the netplay library's one loop (docs/design/rollback.md
//! §4.7, "What a host does"; docs/frontend.md §2).
//!
//! The link is nettai-rtc's: made in a room of the signaling server (a new room, whose code the server picks:
//! an invite; or the room of a code typed) or directly (this machine hosting on a UDP port, or the host's
//! address typed). Once it is open the lobby is made over it, and lasts for as long as the two players stay
//! together: through every match they play, and between them. Each frame, whatever is on screen: every
//! datagram the link has goes to the lobby ([`Session::feed`], before the battle's ticks), the lobby is polled
//! ([`Session::poll`], after them), and what it has to say is sent ([`Session::send`]).
//!
//! This holds no screen and shows nothing: `super::Play` drives it, and says what goes wrong in the player's
//! words (`crate::errors`).

use nettai_battle::Content;
use nettai_frontend::connection::{Ping, Reconnecting};
use nettai_frontend::lobby::{Agreement, Failure, Lobby, MatchChannel, Role, Settings, Status, entropy};
use nettai_match::Side;
use nettai_rtc::{Config, Error, ErrorKind, Link};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The UDP port a direct host listens on.
pub const DIRECT_PORT: u16 = 47_474;

/// How long a lobby hears nothing from the other player before it gives them up: longer than the link's 30
/// seconds of making a dropped connection again, so that a drop the link mends doesn't end the lobby.
const LOBBY_TIMEOUT: Duration = Duration::from_secs(45);

/// How the two players meet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Way {
    /// A new room of the signaling server, which picks its code: an invite.
    Invite,
    /// The room of a code typed.
    Code(String),
    /// Directly, this machine hosting.
    Host,
    /// Directly, at the host's address.
    Address(String),
}

/// Where the links are made: the signaling server (none: there is none to reach), how connections are made
/// and kept, and the UDP port a direct host listens on (0: one the system picks).
#[derive(Clone, Debug)]
pub struct Net {
    pub server: Option<String>,
    pub config: Config,
    pub port: u16,
}

impl Net {
    /// The player's: the signaling server `$NETTAI_SIGNAL` names (`ws://127.0.0.1:8787` for signaling/ run by
    /// `npx wrangler dev`), and the link library's own way of connecting.
    pub fn of_environment() -> Net {
        Net { server: std::env::var("NETTAI_SIGNAL").ok().filter(|s| !s.is_empty()), config: Config::default(), port: DIRECT_PORT }
    }
}

/// This machine's address on its network (the one a route out goes from), for a direct host to give the other
/// player.
fn local_address() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    // (No datagram goes: connecting a UDP socket only picks the route.)
    socket.connect("192.0.2.1:9").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

/// Where a direct host listening on `port` is reached: this machine's address and the port (the port alone
/// where the machine has no network to name it on).
pub fn host_address(port: u16) -> String {
    match local_address() {
        Some(ip) => format!("{ip}:{port}"),
        None => format!(":{port}"),
    }
}

/// How a session stands after a poll.
#[derive(Clone, Debug, PartialEq)]
pub enum Stands {
    /// The link isn't open: the server or the other player is being reached, or the other player hasn't come.
    Meeting,
    /// In the lobby.
    Lobby,
    /// A match is agreed (by its seed) and each side makes it ready.
    Loading(u32),
    /// Both have: the match starts on its channel ([`Session::channel`]).
    Start(u32),
    /// The match is being played, or is over with its result still shown.
    Playing,
    /// The lobby ended: why.
    Failed(Failure),
}

/// The link, and the lobby once the link is open.
pub struct Session {
    link: Link,
    pub way: Way,
    lobby: Option<Lobby>,
}

impl Session {
    /// Make the link `way` says. An error is the link library's: its kind is what the player is told
    /// (`UserError::of_link`), its detail goes to the log.
    pub fn open(way: Way, net: &Net) -> Result<Session, Error> {
        // (With no signaling server named, there is none to reach.)
        let server = || net.server.clone().ok_or_else(|| Error { kind: ErrorKind::ServerUnreachable, detail: "no signaling server is named ($NETTAI_SIGNAL)".into() });
        let config = net.config.clone();
        let link = match &way {
            Way::Invite => Link::create_room(&server()?, config),
            Way::Code(code) => Link::join_room(&server()?, code, config)?,
            #[cfg(not(target_arch = "wasm32"))]
            Way::Host => Link::host(net.port, config)?,
            #[cfg(not(target_arch = "wasm32"))]
            Way::Address(address) => Link::join(address, config)?,
            #[cfg(target_arch = "wasm32")]
            Way::Host | Way::Address(_) => return Err(Error { kind: ErrorKind::Internal, detail: "a direct link needs a socket".into() }),
        };
        Ok(Session { link, way, lobby: None })
    }

    /// The room's code: the one joined by, or the one the server gave a new room, once it has.
    pub fn code(&self) -> Option<&str> {
        self.link.code()
    }

    /// The UDP port a direct host listens on.
    pub fn port(&self) -> Option<u16> {
        self.link.port()
    }

    /// Whether the link's channel is open: the other player has come.
    pub fn is_open(&self) -> bool {
        self.link.is_open()
    }

    /// What the link is to, for the log.
    pub fn describe(&self) -> String {
        self.link.describe()
    }

    /// The lobby, once the link is open.
    pub fn lobby(&mut self) -> Option<&mut Lobby> {
        self.lobby.as_mut()
    }

    /// The link is open: the lobby over it, in the link's role, holding `settings` (a joiner's last until it
    /// hears the host's) and bringing `side`, this player named `name`. False while the link has no role.
    pub fn make_lobby(&mut self, content: &Arc<Content>, settings: Settings, side: Side, name: &str) -> bool {
        let role = match self.link.role() {
            Some(nettai_rtc::Role::Host) => Role::Host,
            Some(nettai_rtc::Role::Join) => Role::Join,
            None => return false,
        };
        let mut lobby = Lobby::new(role, content, settings, side, entropy(), LOBBY_TIMEOUT);
        lobby.set_name(name);
        self.lobby = Some(lobby);
        true
    }

    /// The frame's first half, before the battle's ticks: every datagram the link has to the lobby, a match's
    /// frames too, and how the link is. Before the lobby, the link kept up. An error is the link's, and the
    /// end.
    pub fn feed(&mut self, now: Instant) -> Result<(), Error> {
        let Some(lobby) = &mut self.lobby else { return self.link.poll() };
        while let Some(datagram) = self.link.recv()? {
            lobby.receive(now, datagram);
        }
        lobby.set_link_down(self.link.down_for().map(|elapsed| Reconnecting { elapsed, gives_up_in: self.link.gives_up_in() }));
        Ok(())
    }

    /// The frame's second half, after the battle's ticks: the lobby polled.
    pub fn poll(&mut self, now: Instant) -> Stands {
        let Some(lobby) = &mut self.lobby else { return Stands::Meeting };
        match lobby.poll(now) {
            Status::Pending => Stands::Lobby,
            Status::Loading(agreed) => Stands::Loading(agreed.seed),
            Status::Start(agreed) => Stands::Start(agreed.seed),
            Status::Playing => Stands::Playing,
            Status::Failed(failure) => Stands::Failed(failure.clone()),
        }
    }

    /// The match agreed, while it is being made ready or starts.
    pub fn agreement(&mut self, now: Instant) -> Option<Agreement> {
        match self.lobby.as_mut()?.poll(now) {
            Status::Loading(agreed) | Status::Start(agreed) => Some(agreed.clone()),
            _ => None,
        }
    }

    /// The channel of the match that starts ([`Stands::Start`]), once.
    pub fn channel(&mut self) -> Option<MatchChannel> {
        self.lobby.as_mut()?.channel()
    }

    /// Send what the lobby has to say: its own, and the frame the match's player made this frame. An error
    /// is the link's, and the end.
    pub fn send(&mut self) -> Result<(), Error> {
        let Some(lobby) = &mut self.lobby else { return Ok(()) };
        for datagram in lobby.outgoing() {
            self.link.send(&datagram)?;
        }
        Ok(())
    }

    /// This player leaves: the other is told (from the lobby, a match or its result), and the link is dropped
    /// with the session.
    pub fn leave(mut self) {
        if let Some(lobby) = &mut self.lobby {
            lobby.leave();
            let _ = self.send();
        }
    }

    /// The round trip, as the lobby's messages time it.
    pub fn ping(&self) -> Option<Ping> {
        self.lobby.as_ref().and_then(Lobby::ping)
    }

    /// While the link is down and being made again: the seconds it has left before it gives up.
    pub fn reconnecting(&self) -> Option<u64> {
        self.link.gives_up_in().map(|left| left.as_secs())
    }
}
