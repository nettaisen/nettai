//! The netplay lobby screen: a room of the signaling server (made here, its
//! code given to the other player; or joined by the code they give), the
//! game, both players' readiness, the link's standing; and the battle it
//! leads to, a `NetPlayer` over the link (`crate::netplay`).
//!
//! The room is entered as soon as there is one: on the screen, making a
//! room enters a new one at once; joining enters the room once its six
//! letters are typed. The signaling server is `$NETTAI_SIGNAL`
//! (`ws://127.0.0.1:8787` for signaling/ run by `npx wrangler dev`).

use crate::app::{App, Battle, Kind, clock_seed, game_names};
use crate::games::{Names, Ready};
use crate::netplay::{Agreeing, Progress, Standing, room_code, signal_server};
use crate::stage::Stage;
use crate::{BattleKind, Connection, LinkState, Peer, Screen, Side};
use nettai_frontend::lobby::Settings;
use nettai_frontend::netplay::NetOptions;
use slint::{ModelRc, SharedString, VecModel};
use std::time::{Duration, Instant};

/// How long the lobby waits for the other player once in the room.
const WAIT: Duration = Duration::from_secs(600);

/// The lobby's choices, and the room it is in.
pub struct LobbyState {
    /// 0 make a room, 1 join one.
    pub mode: i32,
    pub made: String,
    pub joined: String,
    /// The game, by its place among those ready.
    pub game: usize,
    /// The room: the agreeing over its link, and the room's code.
    pub room: Option<(Agreeing<nettai_rtc::Link>, String)>,
    /// Why the last room ended (the library's or the link's words).
    pub problem: Option<String>,
    /// The seed this player's side is picked from.
    pub seed: u32,
}

impl Default for LobbyState {
    fn default() -> LobbyState {
        LobbyState { mode: 0, made: room_code(clock_seed() as u64), joined: String::new(), game: 0, room: None, problem: None, seed: clock_seed() }
    }
}

impl App {
    /// The game the lobby proposes, and its content, if one is ready.
    fn lobby_game_ready(&self) -> Option<(String, std::rc::Rc<Ready>)> {
        let ready = self.ready_games();
        let id = ready.get(self.lobby.game)?.clone();
        let r = self.games.ready(&id)?;
        Some((id, r))
    }

    /// What this player proposes: the game's triple battle, each place left
    /// to the seed, and a random side of it.
    fn lobby_proposal(&self, game: &str, ready: &Ready) -> Option<(Settings, nettai_match::Side)> {
        let m = nettai_match::pick::live(ready.content(), game, self.lobby.seed, None).ok()?;
        let settings = Settings { game: game.to_string(), rounds: vec![nettai_match::RoundSettings::default(); nettai_match::TRIPLE_BATTLE] };
        Some((settings, m.sides[0].clone()))
    }

    /// Enter the room the lobby's mode says (a new one's code, or the one
    /// typed), leaving any other.
    fn enter_room(&mut self) {
        let code = if self.lobby.mode == 0 { self.lobby.made.clone() } else { self.lobby.joined.clone() };
        if self.lobby.room.as_ref().is_some_and(|(_, c)| *c == code) {
            return;
        }
        self.lobby.room = None;
        self.lobby.problem = None;
        if code.len() < 6 {
            return;
        }
        let Some(server) = signal_server() else { return };
        let Some((game, ready)) = self.lobby_game_ready() else { return };
        let Some((settings, side)) = self.lobby_proposal(&game, &ready) else { return };
        // (The room's name on the server: its code, in lowercase.)
        match nettai_rtc::Link::room(&server, &code.to_ascii_lowercase(), nettai_rtc::Config::default()) {
            Ok(link) => self.lobby.room = Some((Agreeing::new(link, ready.content(), settings, side, WAIT), code)),
            Err(e) => self.lobby.problem = Some(e.0),
        }
    }

    pub fn show_lobby(&mut self) {
        let ui = self.ui();
        let ready = self.ready_games();
        self.lobby.game = self.lobby.game.min(ready.len().saturating_sub(1));
        ui.set_lobby_games(ModelRc::new(VecModel::from(ready.iter().map(|g| SharedString::from(game_names(g).0)).collect::<Vec<_>>())));
        ui.set_lobby_game_index(self.lobby.game as i32);
        ui.set_lobby_mode_index(self.lobby.mode);
        ui.set_lobby_code_text(if self.lobby.mode == 0 { self.lobby.made.as_str() } else { self.lobby.joined.as_str() }.into());
        ui.set_lobby_server(signal_server().unwrap_or_default().into());
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
        let (link, ready, them) = match &self.lobby.room {
            None if signal_server().is_none() => (LinkState::NoServer, false, Peer::default()),
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
                (link, a.ready(), Peer { present, name: SharedString::default(), navi: theirs.into(), ready: a.their_ready() })
            }
        };
        ui.set_lobby_me(Peer { present: true, name: self.name.as_str().into(), navi: navi.into(), ready });
        ui.set_lobby_them(them);
        ui.set_lobby_link(link);
        let problem = self.lobby.problem.clone().or_else(|| self.lobby.room.as_ref().and_then(|(a, _)| a.problem())).unwrap_or_default();
        ui.set_lobby_problem(problem.into());
    }

    pub fn lobby_mode(&mut self, mode: i32) {
        self.lobby.mode = mode.clamp(0, 1);
        if self.lobby.mode == 0 {
            // (A new room each time one is made.)
            self.lobby.made = room_code(clock_seed() as u64);
        }
        self.show_lobby();
    }

    pub fn lobby_code(&mut self, code: &str) {
        // (A room's code: its letters in capitals, six at most.)
        self.lobby.joined = code.chars().filter(|c| c.is_ascii_alphanumeric()).take(6).collect::<String>().to_ascii_uppercase();
        if self.lobby.joined != code {
            self.ui().set_lobby_code_text(self.lobby.joined.as_str().into());
        }
        if self.lobby.mode == 1 {
            self.enter_room();
            self.follow_lobby();
        }
    }

    pub fn lobby_game(&mut self, game: usize) {
        self.lobby.game = game;
        if let (Some((id, ready)), true) = (self.lobby_game_ready(), self.lobby.room.is_some())
            && let Some((settings, side)) = self.lobby_proposal(&id, &ready)
            && let Some((a, _)) = &mut self.lobby.room
        {
            a.propose(ready.content(), settings, side);
        }
        self.show_lobby();
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

    /// Copy the room's code, for the other player.
    pub fn lobby_copy(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(self.lobby.made.clone());
        }
    }

    /// `NETTAI_NETPLAY=<game>:<make|join>:<CODE>`: once `game` is loaded,
    /// straight into the lobby, in that room, ready (two windows on one
    /// machine, and the tour).
    pub fn auto_netplay(&mut self, game: &str) {
        let Ok(spec) = std::env::var("NETTAI_NETPLAY") else { return };
        let [g, way, code] = spec.split(':').collect::<Vec<_>>()[..] else { return };
        if g != game || self.lobby.room.is_some() || self.battle.is_some() {
            return;
        }
        self.lobby.game = self.ready_games().iter().position(|r| r == game).unwrap_or(0);
        self.lobby.mode = if way == "join" { 1 } else { 0 };
        let code = code.to_ascii_uppercase();
        if self.lobby.mode == 0 {
            self.lobby.made = code;
        } else {
            self.lobby.joined = code;
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

    /// A frame on the lobby screen: the room's agreeing stepped; the match,
    /// once agreed, played.
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
                let recorded = self.recorder(&content, &m, agreement.seed, &game, agreement.side as u8).and_then(|(r, path)| player.record(r).ok().map(|_| path));
                let graphics = ready.graphics(self.lang);
                let names = Names::of(&content, &graphics);
                let navi = |side: usize| names.navi(agreement.m.sides[side].navi(&content));
                let (mine, theirs) = (agreement.side, 1 - agreement.side);
                let rounds = agreement.m.rounds.len();
                self.show_battle(Battle::new(Stage::new(player), Kind::Netplay { game: game.clone() }, recorded, rounds));
                let ui = self.ui();
                ui.set_battle_kind(BattleKind::Netplay);
                // (The console shown is this player's: their navi on the left.)
                ui.set_battle_left(Side { name: self.name.as_str().into(), navi: navi(mine).into(), detail: SharedString::default() });
                ui.set_battle_right(Side { name: SharedString::default(), navi: navi(theirs).into(), detail: SharedString::default() });
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
