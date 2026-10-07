//! The netplay lobby screen: the link to the other player, made one of
//! three ways (a room of the signaling server, made here and its code given
//! to the other player, or joined by the code they give; or directly, this
//! player hosting on a port of their own or joining the other's address),
//! your name, the game, your build, both players' readiness, the link's
//! standing; and the battle it leads to, a `NetPlayer` over the link
//! (`crate::netplay`).
//!
//! The room is entered as soon as there is one: on the screen, making a
//! room enters a new one at once; joining enters the room once its six
//! letters are typed. Directly, this player hosts while no address is
//! typed, and joins the address typed once it is confirmed. The signaling
//! server is `$NETTAI_SIGNAL` (`ws://127.0.0.1:8787` for signaling/ run by
//! `npx wrangler dev`).

use crate::app::{App, Battle, Kind, clock_seed, game_names};
use crate::builds::screen::BuildsState;
use crate::games::{Names, Ready};
use crate::netplay::{Agreeing, Progress, Standing, room_code, signal_server};
use crate::stage::Stage;
use crate::{BattleKind, Connection, LinkState, Peer, Screen, Side, Strings};
use nettai_frontend::lobby::Settings;
use nettai_frontend::netplay::NetOptions;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::time::{Duration, Instant};

/// How long the lobby waits for the other player once the link is made.
const WAIT: Duration = Duration::from_secs(600);

/// The UDP port a direct host listens on.
pub const DIRECT_PORT: u16 = 47_474;

/// How the link to the other player is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    MakeRoom,
    JoinRoom,
    Direct,
}

impl Mode {
    fn index(self) -> i32 {
        self as i32
    }

    fn of(i: i32) -> Mode {
        match i.rem_euclid(3) {
            0 => Mode::MakeRoom,
            1 => Mode::JoinRoom,
            _ => Mode::Direct,
        }
    }
}

/// The lobby's choices, and the link it has.
pub struct LobbyState {
    pub mode: Mode,
    pub made: String,
    pub joined: String,
    /// The direct host's address typed (empty: this player hosts), and
    /// whether it is confirmed.
    pub address: String,
    pub address_confirmed: bool,
    /// The game, by its place among those ready.
    pub game: usize,
    /// The build this player brings: 0 a random side, else a build of the
    /// game's by its place among them (`BuildsState::choices`).
    pub build: usize,
    /// The link: the agreeing over it, and what it was made to (a room's
    /// code, the address joined, the port hosted on).
    pub room: Option<(Agreeing<nettai_rtc::Link>, String)>,
    /// Why the last link ended (the library's or the link's words).
    pub problem: Option<String>,
    /// The seed a random side is picked from.
    pub seed: u32,
}

impl Default for LobbyState {
    fn default() -> LobbyState {
        LobbyState {
            mode: Mode::MakeRoom,
            made: room_code(clock_seed() as u64),
            joined: String::new(),
            address: String::new(),
            address_confirmed: false,
            game: 0,
            build: 0,
            room: None,
            problem: None,
            seed: clock_seed(),
        }
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

impl App {
    /// The game the lobby proposes, and its content, if one is ready.
    fn lobby_game_ready(&self) -> Option<(String, std::rc::Rc<Ready>)> {
        let ready = self.ready_games();
        let id = ready.get(self.lobby.game)?.clone();
        let r = self.games.ready(&id)?;
        Some((id, r))
    }

    /// The builds the lobby offers of `game`: their names and sides.
    fn lobby_builds(&self, ready: &Ready, game: &str) -> Vec<(String, nettai_match::Side)> {
        BuildsState::choices(ready.content(), game)
    }

    /// What this player proposes: the game's triple battle, each place left
    /// to the seed, and their side: the build chosen, else a random one.
    fn lobby_proposal(&self, game: &str, ready: &Ready) -> Option<(Settings, nettai_match::Side)> {
        let settings = Settings { game: game.to_string(), rounds: vec![nettai_match::RoundSettings::default(); nettai_match::TRIPLE_BATTLE] };
        if self.lobby.build > 0
            && let Some((_, side)) = self.lobby_builds(ready, game).into_iter().nth(self.lobby.build - 1)
        {
            return Some((settings, side));
        }
        let m = nettai_match::pick::live(ready.content(), game, self.lobby.seed, None).ok()?;
        Some((settings, m.sides[0].clone()))
    }

    /// Make the link the lobby's mode says (a new room's, the one typed,
    /// the direct host's or the address typed), leaving any other.
    fn enter_room(&mut self) {
        let target = match self.lobby.mode {
            Mode::MakeRoom => self.lobby.made.clone(),
            Mode::JoinRoom => self.lobby.joined.clone(),
            Mode::Direct if self.lobby.address.is_empty() => format!("host:{DIRECT_PORT}"),
            Mode::Direct if self.lobby.address_confirmed => format!("join:{}", self.lobby.address),
            Mode::Direct => String::new(),
        };
        if self.lobby.room.as_ref().is_some_and(|(_, c)| *c == target) {
            return;
        }
        self.lobby.room = None;
        self.lobby.problem = None;
        let rooms = matches!(self.lobby.mode, Mode::MakeRoom | Mode::JoinRoom);
        if (rooms && target.len() < 6) || target.is_empty() {
            return;
        }
        let Some((game, ready)) = self.lobby_game_ready() else { return };
        let Some((settings, side)) = self.lobby_proposal(&game, &ready) else { return };
        let config = nettai_rtc::Config::default();
        let link = match self.lobby.mode {
            Mode::MakeRoom | Mode::JoinRoom => {
                let Some(server) = signal_server() else { return };
                // (The room's name on the server: its code, in lowercase.)
                nettai_rtc::Link::room(&server, &target.to_ascii_lowercase(), config)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Mode::Direct if self.lobby.address.is_empty() => nettai_rtc::Link::host(DIRECT_PORT, config),
            #[cfg(not(target_arch = "wasm32"))]
            Mode::Direct => nettai_rtc::Link::join(&self.lobby.address, config),
            #[cfg(target_arch = "wasm32")]
            Mode::Direct => return,
        };
        match link {
            Ok(link) => {
                let mut a = Agreeing::new(link, ready.content(), settings, side, WAIT);
                a.set_name(&self.name);
                self.lobby.room = Some((a, target));
            }
            Err(e) => self.lobby.problem = Some(e.0),
        }
    }

    pub fn show_lobby(&mut self) {
        let ui = self.ui();
        let ready = self.ready_games();
        self.lobby.game = self.lobby.game.min(ready.len().saturating_sub(1));
        ui.set_lobby_games(ModelRc::new(VecModel::from(ready.iter().map(|g| SharedString::from(game_names(g).0)).collect::<Vec<_>>())));
        ui.set_lobby_game_index(self.lobby.game as i32);
        ui.set_lobby_mode_index(self.lobby.mode.index());
        let code = match self.lobby.mode {
            Mode::MakeRoom => self.lobby.made.as_str(),
            Mode::JoinRoom => self.lobby.joined.as_str(),
            Mode::Direct => self.lobby.address.as_str(),
        };
        ui.set_lobby_code_text(code.into());
        ui.set_lobby_server(signal_server().unwrap_or_default().into());
        let host = local_address().map_or_else(|| format!("…:{DIRECT_PORT}"), |ip| format!("{ip}:{DIRECT_PORT}"));
        ui.set_lobby_direct_address(host.into());
        // The builds the player can bring.
        let mut builds: Vec<SharedString> = vec![ui.global::<Strings>().invoke_random()];
        if let Some((game, r)) = self.lobby_game_ready() {
            builds.extend(self.lobby_builds(&r, &game).into_iter().map(|(name, _)| SharedString::from(name)));
        }
        self.lobby.build = self.lobby.build.min(builds.len() - 1);
        ui.set_lobby_builds(ModelRc::new(VecModel::from(builds)));
        ui.set_lobby_build_index(self.lobby.build as i32);
        self.enter_room();
        self.follow_lobby();
    }

    /// The lobby's standing, shown: the link, both players, a problem.
    fn follow_lobby(&mut self) {
        let ui = self.ui();
        // Your navi: your side's, as proposed.
        let navi = self
            .lobby_game_ready()
            .and_then(|(g, r)| {
                let side = self.lobby_proposal(&g, &r)?.1;
                let graphics = r.graphics(self.lang);
                Some(Names::of(r.content(), &graphics).navi(side.navi(r.content())))
            })
            .unwrap_or_default();
        let direct = self.lobby.mode == Mode::Direct;
        let (link, ready, them) = match &self.lobby.room {
            None if signal_server().is_none() && !direct => (LinkState::NoServer, false, Peer::default()),
            None if self.lobby.problem.is_some() => (LinkState::Failed, false, Peer::default()),
            None => (LinkState::Unavailable, false, Peer::default()),
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
                (link, a.ready(), Peer { present, name: name.into(), navi: theirs.into(), ready: a.their_ready() })
            }
        };
        ui.set_lobby_me(Peer { present: true, name: self.name.as_str().into(), navi: navi.into(), ready });
        ui.set_lobby_them(them);
        ui.set_lobby_link(link);
        let problem = self.lobby.problem.clone().or_else(|| self.lobby.room.as_ref().and_then(|(a, _)| a.problem())).unwrap_or_default();
        ui.set_lobby_problem(problem.into());
    }

    pub fn lobby_mode(&mut self, mode: i32) {
        self.lobby.mode = Mode::of(mode);
        if self.lobby.mode == Mode::MakeRoom {
            // (A new room each time one is made.)
            self.lobby.made = room_code(clock_seed() as u64);
        }
        self.show_lobby();
    }

    /// The code (or the address) typed.
    pub fn lobby_code(&mut self, code: &str) {
        match self.lobby.mode {
            Mode::Direct => {
                // (An address: no spaces; joined once confirmed.)
                self.lobby.address = code.chars().filter(|c| !c.is_whitespace() && !c.is_control()).take(64).collect();
                self.lobby.address_confirmed = false;
                if self.lobby.address != code {
                    self.ui().set_lobby_code_text(self.lobby.address.as_str().into());
                }
                // (Hosting stops once an address is typed.)
                self.enter_room();
                self.follow_lobby();
            }
            _ => {
                // (A room's code: its letters in capitals, six at most.)
                self.lobby.joined = code.chars().filter(|c| c.is_ascii_alphanumeric()).take(6).collect::<String>().to_ascii_uppercase();
                if self.lobby.joined != code {
                    self.ui().set_lobby_code_text(self.lobby.joined.as_str().into());
                }
                if self.lobby.mode == Mode::JoinRoom {
                    self.enter_room();
                    self.follow_lobby();
                }
            }
        }
    }

    /// The code (or the address) confirmed: a direct address joined.
    pub fn lobby_code_done(&mut self) {
        if self.lobby.mode == Mode::Direct {
            self.lobby.address_confirmed = !self.lobby.address.is_empty();
            self.enter_room();
            self.follow_lobby();
        }
    }

    pub fn lobby_game(&mut self, game: usize) {
        self.lobby.game = game;
        self.lobby.build = 0;
        self.propose();
        self.show_lobby();
    }

    pub fn lobby_build(&mut self, build: usize) {
        self.lobby.build = build;
        self.propose();
        self.show_lobby();
    }

    /// Propose the game and side the lobby has now, over the link it has.
    fn propose(&mut self) {
        if let Some((id, ready)) = self.lobby_game_ready()
            && let Some((settings, side)) = self.lobby_proposal(&id, &ready)
            && let Some((a, _)) = &mut self.lobby.room
        {
            a.propose(ready.content(), settings, side);
        }
    }

    pub fn lobby_ready(&mut self) {
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
    pub fn lobby_copy(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let text = match self.lobby.mode {
                Mode::Direct => self.ui().get_lobby_direct_address().to_string(),
                _ => self.lobby.made.clone(),
            };
            let _ = clipboard.set_text(text);
        }
    }

    /// `NETTAI_NETPLAY=<game>:<make|join>:<CODE>` (or `<game>:host:` and
    /// `<game>:direct:<HOST:PORT>`): once `game` is loaded, straight into
    /// the lobby, linked that way, ready (two windows on one machine, and
    /// the tour).
    pub fn auto_netplay(&mut self, game: &str) {
        let Ok(spec) = std::env::var("NETTAI_NETPLAY") else { return };
        let mut parts = spec.splitn(3, ':');
        let (Some(g), Some(way), code) = (parts.next(), parts.next(), parts.next().unwrap_or("")) else { return };
        if g != game || self.lobby.room.is_some() || self.battle.is_some() {
            return;
        }
        self.lobby.game = self.ready_games().iter().position(|r| r == game).unwrap_or(0);
        match way {
            "join" => {
                self.lobby.mode = Mode::JoinRoom;
                self.lobby.joined = code.to_ascii_uppercase();
            }
            "host" => {
                self.lobby.mode = Mode::Direct;
                self.lobby.address.clear();
            }
            "direct" => {
                self.lobby.mode = Mode::Direct;
                self.lobby.address = code.to_string();
                self.lobby.address_confirmed = true;
            }
            _ => {
                self.lobby.mode = Mode::MakeRoom;
                self.lobby.made = code.to_ascii_uppercase();
            }
        }
        self.enter(Screen::Lobby);
        if let Some((a, _)) = &mut self.lobby.room {
            a.set_ready(true);
        }
    }

    /// Leave the room (the lobby screen was left).
    pub fn leave_room(&mut self) {
        self.lobby.room = None;
    }

    /// The player's name, said over the link too.
    pub fn lobby_name(&mut self) {
        let name = self.name.clone();
        if let Some((a, _)) = &mut self.lobby.room {
            a.set_name(&name);
        }
    }

    /// A frame on the lobby screen: the agreeing stepped; the match, once
    /// agreed, played.
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
                let Some((game, ready)) = self.lobby_game_ready() else { return };
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
