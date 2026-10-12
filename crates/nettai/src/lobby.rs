//! The link to another player, as Play makes it: in a room of the signaling server (a code typed, or a new
//! room, whose code the server picks and this player gives the other) or directly (the host's address typed,
//! or this player hosting on a port of their own), and the agreeing that runs over it (`crate::netplay`).
//!
//! This is the session alone: it shows nothing and reads no screen. Play's screen (`crate::screens::play`,
//! not built yet) holds a [`LobbyState`], enters a room with what its player brings, polls it each frame, and
//! starts the battle it leads to (`Shell::start_battle`).
//!
//! The signaling server is `$NETTAI_SIGNAL` (`ws://127.0.0.1:8787` for signaling/ run by `npx wrangler dev`).

// (Play's screen isn't built yet: what it will call isn't called.)
#![allow(dead_code)]

use crate::games::Ready;
use crate::netplay::{Agreeing, Progress, Standing, signal_server};
use nettai_frontend::lobby::Settings;
use std::time::{Duration, Instant};

/// How long the lobby waits for the other player once the link is made.
const WAIT: Duration = Duration::from_secs(600);

/// The UDP port a direct host listens on.
pub const DIRECT_PORT: u16 = 47_474;

/// The link's way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Way {
    /// A new room of the signaling server, which picks its code ([`LobbyState::code`]).
    MakeRoom,
    /// The room of a code typed.
    JoinRoom(String),
    /// Directly, this player hosting.
    Host,
    /// Directly, to the host's address.
    Join(String),
}

/// The link Play has, if it has one.
#[derive(Default)]
pub struct LobbyState {
    /// The link: the agreeing over it, and its way.
    room: Option<(Agreeing<nettai_rtc::Link>, Way)>,
}

/// This machine's address on its network (the one a route out goes from), for a direct host to give the other
/// player.
fn local_address() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    // (No datagram goes: connecting a UDP socket only picks the route.)
    socket.connect("192.0.2.1:9").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

/// Where a direct host is reached: this machine's address and the port.
pub fn host_address() -> Option<String> {
    local_address().map(|ip| format!("{ip}:{DIRECT_PORT}"))
}

impl LobbyState {
    /// A link is being made, or is up.
    pub fn linked(&self) -> bool {
        self.room.is_some()
    }

    /// The link's way, while there is one.
    pub fn way(&self) -> Option<&Way> {
        self.room.as_ref().map(|(_, way)| way)
    }

    /// The agreeing over the link, while there is one.
    pub fn agreeing(&mut self) -> Option<&mut Agreeing<nettai_rtc::Link>> {
        self.room.as_mut().map(|(a, _)| a)
    }

    /// How the link stands.
    pub fn standing(&self) -> Option<Standing> {
        self.room.as_ref().map(|(a, _)| a.standing())
    }

    /// Make the link `way` says, leaving any other: this player, named `name`, proposes `settings` and brings
    /// `side`, of the game `ready` is. An error is the link library's: its kind is what the player is told
    /// (`UserError::from`), its detail goes to the log.
    pub fn enter(&mut self, way: Way, ready: &Ready, settings: Settings, side: nettai_match::Side, name: &str) -> Result<(), nettai_rtc::Error> {
        self.room = None;
        let config = nettai_rtc::Config::default();
        // (With no signaling server named, there is none to reach.)
        let server = || signal_server().ok_or_else(|| nettai_rtc::Error { kind: nettai_rtc::ErrorKind::ServerUnreachable, detail: "no signaling server is named ($NETTAI_SIGNAL)".into() });
        let link = match &way {
            Way::MakeRoom => nettai_rtc::Link::create_room(&server()?, config),
            Way::JoinRoom(code) => nettai_rtc::Link::join_room(&server()?, code, config)?,
            #[cfg(not(target_arch = "wasm32"))]
            Way::Host => nettai_rtc::Link::host(DIRECT_PORT, config)?,
            #[cfg(not(target_arch = "wasm32"))]
            Way::Join(address) => nettai_rtc::Link::join(address, config)?,
            #[cfg(target_arch = "wasm32")]
            Way::Host | Way::Join(_) => return Err(nettai_rtc::Error { kind: nettai_rtc::ErrorKind::Internal, detail: "a direct link needs a socket".into() }),
        };
        let mut agreeing = Agreeing::new(link, ready.content(), settings, side, WAIT);
        agreeing.set_name(name);
        self.room = Some((agreeing, way));
        Ok(())
    }

    /// The room's code: the one joined by, or the one the server gave a new room, once it has.
    pub fn code(&self) -> Option<&str> {
        self.room.as_ref().and_then(|(a, _)| a.code())
    }

    /// The link given up.
    pub fn leave(&mut self) {
        self.room = None;
    }

    /// A frame with a link: the agreeing stepped. Once the match is agreed (or the link has failed) the link
    /// is this state's no more: it is the progress's.
    pub fn poll(&mut self, now: Instant) -> Option<Progress<nettai_rtc::Link>> {
        let (agreeing, _) = self.room.as_mut()?;
        match agreeing.poll(now) {
            Progress::Pending => Some(Progress::Pending),
            done => {
                self.room = None;
                Some(done)
            }
        }
    }
}
