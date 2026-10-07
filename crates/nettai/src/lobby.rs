//! Play: online, as Tango's Play tab. The selector strip's game and build
//! (`App::select`) are what this player brings, its sheet shows the build,
//! and the band at the foot makes the link: in a room of the signaling
//! server (the code typed, or none: a new room, its code given to the other
//! player) or directly (the host's address typed, or none: this player hosts
//! on a port of their own), when FIGHT is pressed. Once linked, the band is
//! the room's: its code to share, READY and LEAVE; both players' cards, the
//! link's standing; and the battle it leads to, a `NetPlayer` over the link
//! (`crate::netplay`). The room stays open while another tab is shown (the
//! bar marks Play), and the match starts there when both are ready.
//!
//! The signaling server is `$NETTAI_SIGNAL` (`ws://127.0.0.1:8787` for
//! signaling/ run by `npx wrangler dev`).

use crate::app::{App, Battle, Kind, clock_seed, game_names};
use crate::games::{Names, Ready};
use crate::netplay::{Agreeing, Progress, Standing, room_code, signal_server};
use crate::stage::Stage;
use crate::{BattleKind, Connection, LinkState, Peer, Screen, Side, Strings};
use nettai_frontend::lobby::Settings;
use nettai_frontend::netplay::NetOptions;
use slint::{ComponentHandle, SharedString};
use std::time::{Duration, Instant};

/// How long the lobby waits for the other player once the link is made.
const WAIT: Duration = Duration::from_secs(600);

/// The UDP port a direct host listens on.
pub const DIRECT_PORT: u16 = 47_474;

/// How the link to the other player is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A room of the signaling server: the code typed, or a new one.
    Room,
    /// Directly: the host's address typed, or none (this player hosts).
    Direct,
}

impl Mode {
    fn index(self) -> i32 {
        self as i32
    }

    fn of(i: i32) -> Mode {
        if i.rem_euclid(2) == 0 { Mode::Room } else { Mode::Direct }
    }
}

/// The link's way, as FIGHT makes it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Way {
    MakeRoom(String),
    JoinRoom(String),
    Host,
    Join(String),
}

/// The band's choices, and the link it has.
pub struct LobbyState {
    pub mode: Mode,
    /// The room's code typed (empty: FIGHT makes a new room).
    pub code: String,
    /// The direct host's address typed (empty: FIGHT hosts).
    pub address: String,
    /// The link: the agreeing over it, and its way.
    room: Option<(Agreeing<nettai_rtc::Link>, Way)>,
    /// Why the last link ended (the library's or the link's words).
    pub problem: Option<String>,
    /// The seed a random side is picked from.
    pub seed: u32,
}

impl Default for LobbyState {
    fn default() -> LobbyState {
        LobbyState { mode: Mode::Room, code: String::new(), address: String::new(), room: None, problem: None, seed: clock_seed() }
    }
}

impl LobbyState {
    /// A link is being made, or is up.
    pub fn linked(&self) -> bool {
        self.room.is_some()
    }
}

/// This machine's address on its network (the one a route out goes from),
/// for a direct host to give the other player.
fn local_address() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    // (No datagram goes: connecting a UDP socket only picks the route.)
    socket.connect("192.0.2.1:9").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

/// Where a direct host is reached: this machine's address and the port.
fn host_address() -> String {
    local_address().map_or_else(|| format!("…:{DIRECT_PORT}"), |ip| format!("{ip}:{DIRECT_PORT}"))
}

impl App {
    /// The game the strip has chosen, and its content, if it is ready.
    fn play_game_ready(&self) -> Option<(String, std::rc::Rc<Ready>)> {
        let id = self.select.game.clone()?;
        let r = self.games.ready(&id)?;
        Some((id, r))
    }

    /// What this player proposes: the game's triple battle, each place left
    /// to the seed, and their side: the strip's build, else a random one.
    fn play_proposal(&self, game: &str, ready: &Ready) -> Option<(Settings, nettai_match::Side)> {
        let settings = Settings { game: game.to_string(), rounds: vec![nettai_match::RoundSettings::default(); nettai_match::TRIPLE_BATTLE] };
        if let Some(side) = self.selected_side() {
            return Some((settings, side));
        }
        let m = nettai_match::pick::live(ready.content(), game, self.lobby.seed, None).ok()?;
        Some((settings, m.sides[0].clone()))
    }

    /// Make the link `way` says, leaving any other.
    fn enter_room(&mut self, way: Way) {
        self.lobby.room = None;
        self.lobby.problem = None;
        let Some((game, ready)) = self.play_game_ready() else {
            self.lobby.problem = Some("No game is ready to play".into());
            return;
        };
        let Some((settings, side)) = self.play_proposal(&game, &ready) else { return };
        let config = nettai_rtc::Config::default();
        let link = match &way {
            Way::MakeRoom(code) | Way::JoinRoom(code) => {
                let Some(server) = signal_server() else { return };
                // (The room's name on the server: its code, in lowercase.)
                nettai_rtc::Link::room(&server, &code.to_ascii_lowercase(), config)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Way::Host => nettai_rtc::Link::host(DIRECT_PORT, config),
            #[cfg(not(target_arch = "wasm32"))]
            Way::Join(address) => nettai_rtc::Link::join(address, config),
            #[cfg(target_arch = "wasm32")]
            Way::Host | Way::Join(_) => return,
        };
        match link {
            Ok(link) => {
                let mut a = Agreeing::new(link, ready.content(), settings, side, WAIT);
                a.set_name(&self.name);
                self.lobby.room = Some((a, way));
            }
            Err(e) => self.lobby.problem = Some(e.0),
        }
    }

    /// The Play tab as it stands: the strip, the sheet, the band, the
    /// cards.
    pub fn show_play(&mut self) {
        // (A build made or renamed in the creator since: read again.)
        self.refresh_builds();
        self.show_select();
        let ui = self.ui();
        // The sheet: the build brought (a random side's, from the seed).
        let sheet = match self.play_game_ready() {
            Some((game, ready)) => match self.play_proposal(&game, &ready) {
                Some((_, side)) => {
                    let name = self.selected_name().unwrap_or_else(|| ui.global::<Strings>().invoke_random().to_string());
                    self.sheet(&game, &side, &name)
                }
                None => crate::builds::view::empty_sheet(),
            },
            None => crate::builds::view::empty_sheet(),
        };
        ui.set_play_sheet(sheet);
        ui.set_play_mode_index(self.lobby.mode.index());
        let code = match self.lobby.mode {
            Mode::Room => self.lobby.code.as_str(),
            Mode::Direct => self.lobby.address.as_str(),
        };
        ui.set_play_code_text(code.into());
        self.follow_lobby();
    }

    /// The link's standing, shown: the band, both cards, a problem.
    fn follow_lobby(&mut self) {
        let ui = self.ui();
        // Your navi: your side's, as proposed.
        let navi = self
            .play_game_ready()
            .and_then(|(g, r)| {
                let side = self.play_proposal(&g, &r)?.1;
                let graphics = r.graphics(self.lang);
                Some(Names::of(r.content(), &graphics).navi(side.navi(r.content())))
            })
            .unwrap_or_default();
        let direct = self.lobby.mode == Mode::Direct;
        let (link, ready, them, ping) = match &self.lobby.room {
            None if signal_server().is_none() && !direct => (LinkState::NoServer, false, Peer::default(), -1),
            None if self.lobby.problem.is_some() => (LinkState::Failed, false, Peer::default(), -1),
            None => (LinkState::Unavailable, false, Peer::default(), -1),
            Some((a, _)) => {
                let link = match a.standing() {
                    Standing::Connecting => LinkState::Connecting,
                    Standing::Waiting => LinkState::Waiting,
                    Standing::Open => LinkState::Open,
                    Standing::Reconnecting(_) => LinkState::Reconnecting,
                };
                // (Their game, if it isn't this one: their card says so.)
                let theirs = match a.theirs() {
                    Some(Ok(s)) => game_names(&s.game).0,
                    _ => String::new(),
                };
                let present = link == LinkState::Open || a.theirs().is_some();
                let name = a.their_name().unwrap_or_default();
                (link, a.ready(), Peer { present, name: name.into(), navi: theirs.into(), ready: a.their_ready() }, -1)
            }
        };
        ui.set_play_me(Peer { present: true, name: self.name.as_str().into(), navi: navi.into(), ready });
        ui.set_play_them(them);
        ui.set_play_link(link);
        ui.set_play_ping(ping);
        ui.set_play_linked(self.lobby.linked());
        ui.set_room_open(self.lobby.linked());
        let room = match self.lobby.room.as_ref().map(|(_, w)| w) {
            Some(Way::MakeRoom(code) | Way::JoinRoom(code)) => code.clone(),
            Some(Way::Host) => host_address(),
            Some(Way::Join(address)) => address.clone(),
            None => String::new(),
        };
        ui.set_play_room(room.into());
        let problem = self.lobby.problem.clone().or_else(|| self.lobby.room.as_ref().and_then(|(a, _)| a.problem())).unwrap_or_default();
        ui.set_play_problem(problem.into());
    }

    pub fn play_mode(&mut self, mode: i32) {
        self.lobby.mode = Mode::of(mode);
        self.show_play();
    }

    /// The code (or the address) typed.
    pub fn play_code(&mut self, code: &str) {
        let (kept, typed) = match self.lobby.mode {
            // (An address: no spaces.)
            Mode::Direct => {
                self.lobby.address = code.chars().filter(|c| !c.is_whitespace() && !c.is_control()).take(64).collect();
                (self.lobby.address.clone(), code)
            }
            // (A room's code: its letters in capitals, six at most.)
            Mode::Room => {
                self.lobby.code = code.chars().filter(|c| c.is_ascii_alphanumeric()).take(6).collect::<String>().to_ascii_uppercase();
                (self.lobby.code.clone(), code)
            }
        };
        if kept != typed {
            self.ui().set_play_code_text(kept.into());
        }
    }

    /// FIGHT: the link the band says (a room, the code's or a new one; the
    /// address's, or this player hosting).
    pub fn play_fight(&mut self) {
        let way = match self.lobby.mode {
            Mode::Room if self.lobby.code.is_empty() => Way::MakeRoom(room_code(clock_seed() as u64)),
            Mode::Room if self.lobby.code.len() == 6 => Way::JoinRoom(self.lobby.code.clone()),
            Mode::Room => {
                self.lobby.problem = Some("A room's code has six letters".into());
                self.sound.play(crate::UiSound::Refused);
                return self.follow_lobby();
            }
            Mode::Direct if self.lobby.address.is_empty() => Way::Host,
            Mode::Direct => Way::Join(self.lobby.address.clone()),
        };
        self.enter_room(way);
        if self.lobby.linked() {
            // (The keys go to READY.)
            self.ui().set_play_cursor(3);
        }
        self.follow_lobby();
    }

    /// LEAVE: the link given up.
    pub fn play_leave(&mut self) {
        self.lobby.room = None;
        self.lobby.problem = None;
        self.ui().set_play_cursor(5);
        self.follow_lobby();
    }

    /// Propose the game and side the strip has now, over the link it has.
    pub fn propose(&mut self) {
        if let Some((id, ready)) = self.play_game_ready()
            && let Some((settings, side)) = self.play_proposal(&id, &ready)
            && let Some((a, _)) = &mut self.lobby.room
        {
            a.propose(ready.content(), settings, side);
        }
    }

    pub fn play_ready(&mut self) {
        match &mut self.lobby.room {
            Some((a, _)) => {
                let ready = !a.ready();
                a.set_ready(ready);
            }
            None => self.sound.play(crate::UiSound::Refused),
        }
        self.follow_lobby();
    }

    /// Copy the room's code (or the direct host's address), for the other
    /// player.
    pub fn play_copy(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(self.ui().get_play_room().to_string());
        }
    }

    /// `NETTAI_NETPLAY=<game>:<make|join>:<CODE>` (or `<game>:host:` and
    /// `<game>:direct:<HOST:PORT>`): once `game` is loaded, straight into
    /// Play, linked that way, ready (two windows on one machine, and the
    /// tour).
    pub fn auto_netplay(&mut self, game: &str) {
        let Ok(spec) = std::env::var("NETTAI_NETPLAY") else { return };
        let mut parts = spec.splitn(3, ':');
        let (Some(g), Some(way), code) = (parts.next(), parts.next(), parts.next().unwrap_or("")) else { return };
        if g != game || self.lobby.linked() || self.battle.is_some() {
            return;
        }
        self.select_game(game);
        let way = match way {
            "join" => {
                self.lobby.mode = Mode::Room;
                Way::JoinRoom(code.to_ascii_uppercase())
            }
            "host" => {
                self.lobby.mode = Mode::Direct;
                Way::Host
            }
            "direct" => {
                self.lobby.mode = Mode::Direct;
                self.lobby.address = code.to_string();
                Way::Join(code.to_string())
            }
            _ => {
                self.lobby.mode = Mode::Room;
                Way::MakeRoom(code.to_ascii_uppercase())
            }
        };
        self.enter_room(way);
        self.enter(Screen::Play);
        if let Some((a, _)) = &mut self.lobby.room {
            a.set_ready(true);
        }
    }

    /// The player's name, said over the link too.
    pub fn lobby_name(&mut self) {
        let name = self.name.clone();
        if let Some((a, _)) = &mut self.lobby.room {
            a.set_name(&name);
        }
    }

    /// A frame with a link: the agreeing stepped (whichever tab is shown);
    /// the match, once agreed, played.
    pub fn poll_lobby(&mut self, now: Instant) {
        let Some((a, _)) = &mut self.lobby.room else { return };
        match a.poll(now) {
            Progress::Pending => self.follow_lobby(),
            Progress::Failed(why) => {
                self.lobby.room = None;
                self.lobby.problem = Some(why);
                self.follow_lobby();
            }
            Progress::Agreed(agreed) => {
                self.lobby.room = None;
                self.ui().set_room_open(false);
                let Some((game, ready)) = self.play_game_ready() else { return };
                let (driver, agreement) = agreed.player(NetOptions::default());
                let content = ready.content().clone();
                let mut player = self.player(&ready, Box::new(driver), true);
                let m = nettai_match::Match { seed: Some(agreement.seed), ..agreement.m.clone() };
                let names = agreement.names.clone();
                let recorded = self
                    .recorder(&content, &m, agreement.seed, &game, agreement.side as u8, Some(names.clone()))
                    .and_then(|(r, path)| player.record(r).ok().map(|_| path));
                let graphics = ready.graphics(self.lang);
                let navi_names = Names::of(&content, &graphics);
                let navi = |side: usize| navi_names.navi(agreement.m.sides[side].navi(&content));
                let (mine, theirs) = (agreement.side, 1 - agreement.side);
                let rounds = agreement.m.rounds.len();
                self.show_battle(Battle::new(Stage::new(player), Kind::Netplay { game: game.clone() }, recorded, rounds));
                let ui = self.ui();
                ui.set_battle_kind(BattleKind::Netplay);
                // (The console shown is this player's: their navi on the left.)
                ui.set_battle_left(Side { name: names[mine].as_str().into(), navi: navi(mine).into(), detail: SharedString::default() });
                ui.set_battle_right(Side { name: names[theirs].as_str().into(), navi: navi(theirs).into(), detail: SharedString::default() });
                self.lobby.seed = clock_seed();
                self.go(Screen::Battle);
            }
        }
    }

    /// A netplay battle's connection, shown: its figures, and how long the
    /// link has been down while it is.
    pub fn show_connection(&mut self) {
        let Some(b) = &self.battle else { return };
        let Some(n) = b.stage.player.net_status() else { return };
        self.ui().set_battle_connection(Connection {
            ping: n.ping_ms.map_or(-1, |ms| ms.round() as i32),
            loss_percent: (n.loss * 100.0).round() as i32,
            delay: n.present_delay as i32,
            rollback: n.last_rollback as i32,
            max_rollback: n.max_rollback as i32,
            down: n.reconnecting.map_or(-1, |d| d.as_secs() as i32),
        });
    }
}
