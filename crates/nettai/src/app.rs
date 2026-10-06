//! The app's state and its loop: the screen shown and the wipe between two,
//! the games, the battle (and the title's demo battle), the menus' screens,
//! the sound and the hands. The window's rendering is its clock:
//! [`App::frame`] runs before each frame is drawn.

use crate::games::{Games, Names, Ready, State};
use crate::input::{BattleKey, Keys, Pad};
use crate::replays;
use crate::sound::Sound;
use crate::stage::{self, Stage};
use crate::{
    AppWindow, BattleKind, GameCard, GameState, Input, MatchPreview, NavAction, Playback, ReplayRow, Screen, Side, Strings,
    UiSound, lang,
};
use nettai_battle::BattleResult;
use nettai_frontend::driver::{Driver, LivePlayer};
use nettai_frontend::game::TextMode;
use nettai_frontend::player::Player;
use nettai_frontend::replay::{Info, Recorder, Replay, ReplayPlayer};
use nettai_render::vfont::TextRenderer;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// How long the wipe between two screens takes.
const WIPE: Duration = Duration::from_millis(560);

/// What the battle on screen is.
pub enum Kind {
    /// A set against the stand-in: its game, match and seed (a rematch
    /// plays the match again).
    Live { game: String, m: nettai_match::Match },
    /// A replay, watched from a side.
    Replay { game: String, replay: Box<Replay>, side: u8, starts: Vec<usize> },
    /// A match with another player, over the link the lobby made.
    Netplay { game: String },
}

impl Kind {
    fn game(&self) -> &str {
        match self {
            Kind::Live { game, .. } | Kind::Replay { game, .. } | Kind::Netplay { game } => game,
        }
    }
}

pub struct Battle {
    pub stage: Stage,
    pub kind: Kind,
    /// Paused by the player (the pause's panel is up).
    pub paused: bool,
    /// The replay it is recorded to.
    pub recorded: Option<PathBuf>,
    /// The result and the stop shown (each is shown once).
    shown_result: i32,
    shown_stopped: bool,
    /// The set's rounds, for the pips.
    rounds: usize,
    /// The demo's buttons play the left side (the tour's battle).
    autoplay: bool,
}

impl Battle {
    /// A battle on the screen, nothing over it yet.
    pub fn new(stage: Stage, kind: Kind, recorded: Option<PathBuf>, rounds: usize) -> Battle {
        Battle { stage, kind, paused: false, recorded, shown_result: 0, shown_stopped: false, rounds, autoplay: false }
    }

    /// Whether the battle has the keys (it runs, nothing is over it).
    pub fn playing(&self) -> bool {
        !self.paused && self.shown_result == 0 && !self.shown_stopped
    }
}

/// The title's demo battle: a random set of a game, both sides pressing
/// through their custom screens, the left one fighting.
struct Attract {
    stage: Stage,
    game: String,
    ticks: u32,
}

/// What the Play screen has chosen.
#[derive(Default)]
struct PlayChoice {
    game: Option<String>,
    /// 0: random (from `seed`); else the match file `files[source - 1]`.
    source: usize,
    seed: u32,
    files: Vec<PathBuf>,
}

pub struct App {
    pub ui: slint::Weak<AppWindow>,
    pub games: Games,
    pub lang: &'static str,
    pub sound: Sound,
    pub keys: Keys,
    pub pad: Pad,
    /// The GBA buttons the touch pad holds.
    pub touch: u16,
    pub battle: Option<Battle>,
    attract: Option<Attract>,
    play: PlayChoice,
    pub lobby: crate::lobby::LobbyState,
    replays: Vec<replays::Entry>,
    replay_rows: Rc<VecModel<ReplayRow>>,
    transition: Option<(Instant, Screen, bool)>,
    /// Present the picture at the display's density.
    pub sharp: bool,
    pub text: TextMode,
    pub volume: u32,
    /// The player's name (netplay, and the replays they record).
    pub name: String,
}

/// A seed from the clock.
pub fn clock_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() ^ d.as_secs() as u32).unwrap_or(1)
}

/// The folder replays are written to and listed from: `$NETTAI_REPLAYS`,
/// else `replays`.
pub fn replays_dir() -> PathBuf {
    std::env::var_os("NETTAI_REPLAYS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("replays"))
}

/// The folder match files are listed from: `$NETTAI_MATCHES`, else
/// `matches`.
pub fn matches_dir() -> PathBuf {
    std::env::var_os("NETTAI_MATCHES").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("matches"))
}

/// A game's short name and its number, from its id (`exe6`: EXE6, 6).
pub fn game_names(id: &str) -> (String, String) {
    let number: String = id.chars().filter(|c| c.is_ascii_digit()).collect();
    (id.to_ascii_uppercase(), number)
}

impl App {
    pub fn new(ui: &AppWindow) -> App {
        let lang = lang::initial();
        lang::select(lang);
        ui.global::<crate::Theme>().set_tracking(lang::tracking(lang));
        let replay_rows = Rc::new(VecModel::default());
        ui.set_replay_rows(ModelRc::from(replay_rows.clone()));
        let app = App {
            ui: ui.as_weak(),
            games: Games::find(),
            lang,
            sound: Sound::open(),
            keys: Keys::default(),
            pad: Pad::new(),
            touch: 0,
            battle: None,
            attract: None,
            play: PlayChoice { seed: clock_seed(), ..PlayChoice::default() },
            lobby: crate::lobby::LobbyState::default(),
            replays: Vec::new(),
            replay_rows,
            transition: None,
            sharp: std::env::var_os("NETTAI_PHYSICAL_PIXELS").is_some(),
            text: TextMode::Font,
            volume: 8,
            name: String::new(),
        };
        app.sound.set_volume(app.volume);
        app.show_settings();
        app
    }

    pub fn ui(&self) -> AppWindow {
        self.ui.upgrade().expect("the window outlives the app's state")
    }

    // ---- The screens ----------------------------------------------------

    /// Go to `screen`, with the wipe.
    pub fn go(&mut self, screen: Screen) {
        if self.transition.is_none() {
            self.transition = Some((Instant::now(), screen, false));
        }
    }

    /// The wipe's progress at `now`; the screen changes halfway.
    fn step_transition(&mut self, now: Instant) {
        let ui = self.ui();
        let Some((start, to, switched)) = self.transition else { return };
        let p = (now.saturating_duration_since(start).as_secs_f32() / WIPE.as_secs_f32()).min(1.0);
        // (Eased in and out: fast through the middle, where it covers all.)
        let eased = if p < 0.5 { 4.0 * p * p * p } else { 1.0 - (-2.0 * p + 2.0).powi(3) / 2.0 };
        ui.set_wipe(eased);
        if p >= 0.5 && !switched {
            self.transition = Some((start, to, true));
            self.enter(to);
        }
        if p >= 1.0 {
            self.transition = None;
            ui.set_wipe(0.0);
        }
    }

    /// The screen is now `screen`.
    pub fn enter(&mut self, screen: Screen) {
        let ui = self.ui();
        if ui.get_screen() == Screen::Battle && screen != Screen::Battle {
            self.battle = None;
            ui.set_playing(false);
        }
        if screen != Screen::Lobby {
            self.leave_room();
        }
        ui.set_screen(screen);
        ui.invoke_focus_keys();
        match screen {
            Screen::Play => self.show_play(),
            Screen::Lobby => self.show_lobby(),
            Screen::Replays => self.show_replays(),
            Screen::Settings => self.show_settings(),
            Screen::Title => {
                if let Some(a) = &mut self.attract {
                    a.stage.hold();
                    a.stage.stale = true;
                }
            }
            Screen::Battle => ui.set_playing(self.battle.as_ref().is_some_and(Battle::playing)),
        }
    }

    // ---- The games ------------------------------------------------------

    /// Load every game that has a pack, a thread each.
    pub fn load_games(&mut self) {
        let ids: Vec<String> = self.games.list.iter().map(|e| e.id.clone()).collect();
        for id in ids {
            self.games.load(&id, self.lang, move |id, loaded| {
                crate::with_app(|app| {
                    app.games.loaded(&id, loaded);
                    if let Some(ready) = app.games.ready(&id)
                        && app.attract.is_none()
                    {
                        app.sound.use_game(&id, &ready.loaded);
                    }
                    app.show_games();
                    // (`NETTAI_PLAY=<game>`: straight into a random set of it.)
                    if std::env::var("NETTAI_PLAY").is_ok_and(|g| g == id) && app.battle.is_none() {
                        app.play_game(app.games.list.iter().position(|e| e.id == id).unwrap_or(0));
                        app.play_fight();
                    }
                    app.auto_netplay(&id);
                    match app.ui().get_screen() {
                        Screen::Replays => app.check_replays(),
                        Screen::Lobby => app.show_lobby(),
                        _ => {}
                    }
                })
            });
        }
        self.show_games();
    }

    /// The game cards, as the games stand.
    fn show_games(&mut self) {
        let ui = self.ui();
        let cards: Vec<GameCard> = self
            .games
            .list
            .iter()
            .map(|e| {
                let (name, number) = game_names(&e.id);
                let (state, detail) = match &e.state {
                    State::NoPack(why) => (GameState::NoPack, why.clone()),
                    State::NotLoaded | State::Loading => (GameState::Loading, String::new()),
                    State::Ready(_) => (GameState::Ready, String::new()),
                    State::Failed(why) => (GameState::Failed, why.clone()),
                };
                GameCard { id: e.id.as_str().into(), name: name.into(), number: number.into(), state, detail: detail.into() }
            })
            .collect();
        ui.set_games(ModelRc::new(VecModel::from(cards)));
    }

    /// The games ready to play, by id.
    pub fn ready_games(&self) -> Vec<String> {
        self.games.list.iter().filter(|e| matches!(e.state, State::Ready(_))).map(|e| e.id.clone()).collect()
    }

    // ---- Play -----------------------------------------------------------

    fn show_play(&mut self) {
        self.show_games();
        let ui = self.ui();
        let chosen = self.play.game.as_ref().and_then(|g| self.games.list.iter().position(|e| &e.id == g));
        ui.set_play_chosen(chosen.map_or(-1, |i| i as i32));
        if chosen.is_none() {
            ui.set_play_cursor(0);
        }
        self.show_preview();
    }

    pub fn play_game(&mut self, index: usize) {
        let Some(entry) = self.games.list.get(index) else { return };
        let id = entry.id.clone();
        if let Some(ready) = self.games.ready(&id) {
            self.sound.use_game(&id, &ready.loaded);
        }
        self.play.files = match_files(&id);
        self.play.game = Some(id);
        self.play.source = 0;
        self.ui().set_play_chosen(index as i32);
        self.show_preview();
    }

    pub fn play_source(&mut self, source: usize) {
        self.play.source = source.min(self.play.files.len());
        self.show_preview();
    }

    pub fn play_reroll(&mut self) {
        self.play.seed = clock_seed();
        self.show_preview();
    }

    /// The match the Play screen has chosen, and its seed, or why it can't
    /// be played.
    fn play_match(&self, ready: &Ready) -> Result<(nettai_match::Match, u32), String> {
        let game = self.play.game.as_deref().ok_or("no game is chosen")?;
        let content = ready.content();
        if self.play.source == 0 {
            let m = nettai_match::pick::live(content, game, self.play.seed, None)?;
            return Ok((m, self.play.seed));
        }
        let path = &self.play.files[self.play.source - 1];
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let m = nettai_match::parse(content, &text).map_err(|problems| problems.join("\n"))?;
        let seed = m.seed.unwrap_or(self.play.seed);
        Ok((m, seed))
    }

    fn show_preview(&mut self) {
        let ui = self.ui();
        let mut sources: Vec<SharedString> = vec![ui.global::<Strings>().invoke_random()];
        sources.extend(self.play.files.iter().map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default().into()));
        ui.set_play_sources(ModelRc::new(VecModel::from(sources)));
        ui.set_play_source_index(self.play.source as i32);
        let Some(ready) = self.play.game.as_deref().and_then(|g| self.games.ready(g)) else {
            ui.set_play_preview(MatchPreview::default());
            return;
        };
        let mut preview = MatchPreview { ready: true, seed: self.play.seed.to_string().into(), ..MatchPreview::default() };
        match self.play_match(&ready) {
            Ok((m, seed)) => {
                let graphics = ready.graphics(self.lang);
                let content = ready.content();
                let names = Names::of(content, &graphics);
                preview.left = names.navi(m.sides[0].navi(content)).into();
                preview.right = names.navi(m.sides[1].navi(content)).into();
                preview.seed = seed.to_string().into();
                preview.rounds = ModelRc::new(VecModel::from(crate::arenas::pictures(&ready, &graphics, &m, seed)));
            }
            Err(why) => preview.problem = why.into(),
        }
        ui.set_play_preview(preview);
    }

    /// FIGHT: the chosen match against the stand-in.
    pub fn play_fight(&mut self) {
        let Some(game) = self.play.game.clone() else { return };
        let Some(ready) = self.games.ready(&game) else { return };
        let Ok((m, seed)) = self.play_match(&ready) else { return };
        self.start_live(&game, &ready, m, seed);
        self.go(Screen::Battle);
    }

    // ---- The battle -----------------------------------------------------

    /// A player of `driver` on `ready`'s game, shown in the language.
    pub fn player(&self, ready: &Ready, driver: Box<dyn Driver>, sound: bool) -> Player {
        let graphics = ready.graphics(self.lang);
        let font = ready.loaded.font.clone().filter(|_| self.text == TextMode::Font);
        let renderer = graphics.renderer(self.text, font.clone());
        let audio = ready.loaded.sound.as_ref().filter(|_| sound).map(|s| nettai_audio::BattleAudio::with_banks(s.banks.clone(), s.songs.clone()));
        Player::with(renderer, font.map(TextRenderer::new), audio, driver)
    }

    fn start_live(&mut self, game: &str, ready: &Ready, m: nettai_match::Match, seed: u32) {
        let content = ready.content();
        let driver = LivePlayer::new(nettai_match::Set::of(content, &m, seed));
        let mut player = self.player(ready, Box::new(driver), true);
        // Recorded, to watch again.
        let recorded = self.recorder(content, &m, seed, game, 0).and_then(|(r, path)| player.record(r).ok().map(|_| path));
        let graphics = ready.graphics(self.lang);
        let names = Names::of(content, &graphics);
        let ui = self.ui();
        let left = Side { name: self.name.as_str().into(), navi: names.navi(m.sides[0].navi(content)).into(), detail: SharedString::default() };
        let right = Side { name: ui.global::<Strings>().invoke_stand_in(), navi: names.navi(m.sides[1].navi(content)).into(), detail: SharedString::default() };
        let rounds = m.rounds.len();
        self.show_battle(Battle {
            stage: Stage::new(player),
            kind: Kind::Live { game: game.to_string(), m },
            paused: false,
            recorded,
            shown_result: 0,
            shown_stopped: false,
            rounds,
            autoplay: false,
        });
        ui.set_battle_kind(BattleKind::Live);
        ui.set_battle_left(left);
        ui.set_battle_right(right);
    }

    /// Watch replay `index` of the list from `side`.
    pub fn replays_watch(&mut self, index: usize, side: u8) {
        let Some(entry) = self.replays.get(index) else { return };
        let Ok(replay) = &entry.replay else { return };
        let game = replay.head.game.clone();
        let Some(ready) = self.games.ready(&game) else {
            self.sound.play(UiSound::Refused);
            return;
        };
        let content = ready.content();
        let driver = match ReplayPlayer::new(content, replay, Some(side)) {
            Ok(d) => d,
            Err(_) => {
                self.sound.play(UiSound::Refused);
                return;
            }
        };
        let row = self.replay_rows.row_data(index).unwrap_or_default();
        let player = self.player(&ready, Box::new(driver), true);
        let rounds = nettai_match::binary::read_match(content, &replay.match_bytes).map_or(3, |m| m.rounds.len());
        let starts = replays::round_starts(replay);
        let (left, right) = (
            Side { name: row.left_name.clone(), navi: row.left_navi.clone(), detail: SharedString::default() },
            Side { name: row.right_name.clone(), navi: row.right_navi.clone(), detail: SharedString::default() },
        );
        self.show_battle(Battle {
            stage: Stage::new(player),
            kind: Kind::Replay { game, replay: Box::new(replay.clone()), side, starts },
            paused: false,
            recorded: None,
            shown_result: 0,
            shown_stopped: false,
            rounds,
            autoplay: false,
        });
        let ui = self.ui();
        ui.set_battle_kind(BattleKind::Replay);
        // (The console shown has its navi on the left.)
        let (left, right) = if side == 0 { (left, right) } else { (right, left) };
        ui.set_battle_left(left);
        ui.set_battle_right(right);
        self.go(Screen::Battle);
    }

    /// Put `battle` on the battle screen, nothing over it.
    pub fn show_battle(&mut self, battle: Battle) {
        self.battle = Some(battle);
        let ui = self.ui();
        ui.set_battle_paused(false);
        ui.set_battle_result(0);
        ui.set_battle_stopped(SharedString::default());
        ui.set_battle_recorded(SharedString::default());
        ui.set_battle_language_name(lang::name(self.lang).into());
        ui.set_battle_cursor(0);
        ui.set_battle_playback(Playback::default());
        self.show_pips();
        self.keys.release();
    }

    /// A recorder of the set to a new file in the replays folder.
    pub fn recorder(&self, content: &std::sync::Arc<nettai_battle::Content>, m: &nettai_match::Match, seed: u32, game: &str, side: u8) -> Option<(Recorder, PathBuf)> {
        let dir = replays_dir();
        std::fs::create_dir_all(&dir).ok()?;
        let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let stamp = chrono::DateTime::from_timestamp(when as i64, 0).map(|t| t.with_timezone(&chrono::Local).format("%Y%m%d-%H%M%S").to_string()).unwrap_or_default();
        let path = dir.join(format!("{stamp}-{game}.ntrp"));
        let file = std::fs::File::create(&path).ok()?;
        let mut names = [String::new(), String::new()];
        names[side as usize] = self.name.clone();
        let info = Info { when, side, names };
        let m = nettai_match::Match { seed: Some(seed), ..m.clone() };
        let r = Recorder::new(Box::new(std::io::BufWriter::new(file)), content, &m, &info).ok()?;
        Some((r, path))
    }

    /// The rounds won, as pips under each banner: the console shown's on
    /// the left.
    fn show_pips(&mut self) {
        let Some(b) = &self.battle else { return };
        let ui = self.ui();
        let battle = b.stage.player.battle();
        let (mut wins, mut losses) = (battle.round.wins as usize, battle.round.losses as usize);
        if let Kind::Replay { side: 1, .. } = b.kind {
            (wins, losses) = (losses, wins);
        }
        let need = b.rounds / 2 + 1;
        let pips = |won: usize| -> ModelRc<i32> { ModelRc::new(VecModel::from((0..need).map(|i| if i < won { 1 } else { 0 }).collect::<Vec<i32>>())) };
        ui.set_battle_left_pips(pips(wins));
        ui.set_battle_right_pips(pips(losses));
    }

    pub fn battle_resume(&mut self) {
        if let Some(b) = &mut self.battle {
            b.paused = false;
            b.stage.player.set_paused(false);
            b.stage.hold();
            let ui = self.ui();
            ui.set_battle_paused(false);
            ui.set_playing(true);
        }
    }

    pub fn battle_pause(&mut self) {
        if let Some(b) = &mut self.battle {
            b.paused = true;
            b.stage.player.set_paused(true);
            let ui = self.ui();
            ui.set_battle_paused(true);
            ui.set_battle_cursor(0);
            ui.set_playing(false);
            self.keys.release();
            self.sound.play(UiSound::Pick);
        }
    }

    pub fn battle_restart(&mut self) {
        if let Some(b) = &mut self.battle {
            b.stage.player.restart();
            b.stage.stale = true;
            b.shown_result = 0;
            b.shown_stopped = false;
        }
        let ui = self.ui();
        ui.set_battle_result(0);
        ui.set_battle_stopped(SharedString::default());
        self.battle_resume();
        self.show_pips();
    }

    /// The set again: a live match with another seed (the folders drawn
    /// again), a replay from its start.
    pub fn battle_rematch(&mut self) {
        match self.battle.take() {
            Some(Battle { kind: Kind::Live { game, m }, .. }) => {
                let Some(ready) = self.games.ready(&game) else { return };
                self.start_live(&game, &ready, m, clock_seed());
            }
            Some(Battle { kind: Kind::Netplay { .. }, .. }) => {
                self.go(Screen::Lobby);
                return;
            }
            Some(mut b) => {
                b.stage.player.restart();
                b.stage.stale = true;
                self.show_battle(Battle { shown_result: 0, shown_stopped: false, paused: false, ..b });
            }
            None => return,
        }
        self.ui().set_playing(true);
    }

    pub fn battle_quit(&mut self) {
        let back = match self.battle.as_ref().map(|b| &b.kind) {
            Some(Kind::Replay { .. }) => Screen::Replays,
            Some(Kind::Netplay { .. }) => Screen::Lobby,
            _ => Screen::Title,
        };
        self.go(back);
    }

    pub fn battle_language(&mut self) {
        self.set_language(lang::step(self.lang, 1));
    }

    /// Show the app and the battle in `code`: the app's strings from its
    /// catalog, the content's names from its strings table.
    pub fn set_language(&mut self, code: &'static str) {
        self.lang = code;
        lang::select(code);
        self.ui().global::<crate::Theme>().set_tracking(lang::tracking(code));
        if let Some(b) = &mut self.battle
            && let Some(ready) = self.games.ready(b.kind.game())
        {
            b.stage.player.set_language(&ready.graphics(code));
            b.stage.stale = true;
        }
        if let Some(a) = &mut self.attract
            && let Some(ready) = self.games.ready(&a.game)
        {
            a.stage.player.set_language(&ready.graphics(code));
            a.stage.stale = true;
        }
        let ui = self.ui();
        ui.set_battle_language_name(lang::name(code).into());
        self.show_settings();
        match ui.get_screen() {
            Screen::Play => self.show_preview(),
            Screen::Replays => self.show_replay_rows(),
            Screen::Lobby => self.show_lobby(),
            _ => {}
        }
    }

    /// A key of the window, before Slint sees it: the battle's, while it has
    /// the keys. True if it was the battle's.
    pub fn key(&mut self, event: &slint::winit_030::winit::event::KeyEvent) -> bool {
        let now = Instant::now();
        let playing = self.battle.as_ref().is_some_and(Battle::playing) && self.transition.is_none();
        // (The buttons are followed whatever has the keys, so none sticks.)
        let key = self.keys.event(event, now);
        if !playing {
            return false;
        }
        let replay = matches!(self.battle.as_ref().map(|b| &b.kind), Some(Kind::Replay { .. }));
        match key {
            Some(BattleKey::Pause) => self.battle_pause(),
            Some(BattleKey::Language) => self.battle_language(),
            // A replay's own: its pause, its speed, a frame's step.
            Some(BattleKey::PlayPause) if replay => {
                let b = self.battle.as_mut().expect("playing");
                let paused = b.stage.player.paused();
                b.stage.player.set_paused(!paused);
                b.stage.hold();
            }
            Some(BattleKey::Slower) if replay => {
                self.battle.as_mut().expect("playing").stage.player.slower();
            }
            Some(BattleKey::Faster) if replay => {
                self.battle.as_mut().expect("playing").stage.player.faster();
            }
            Some(BattleKey::Step) if replay => {
                let b = self.battle.as_mut().expect("playing");
                if b.stage.player.paused() {
                    b.stage.player.tick(0);
                    b.stage.stale = true;
                }
            }
            Some(_) => {}
            None => return false,
        }
        self.ui().global::<Input>().set_pad(false);
        true
    }

    /// A key probe's press (`NETTAI_KEY_PROBE`): the right arrow, as a key
    /// event of the window would press it.
    pub fn probe(&mut self, down: bool) {
        use nettai_battle::input::keys;
        if !self.battle.as_ref().is_some_and(Battle::playing) {
            return;
        }
        if down {
            if self.keys.held & keys::RIGHT == 0 {
                self.keys.pressed.push(Instant::now());
            }
            self.keys.held |= keys::RIGHT;
        } else {
            self.keys.held &= !keys::RIGHT;
        }
    }

    // ---- The tour's (`NETTAI_TOUR`) ----------------------------------------

    /// The first game ready, by its place in the list.
    pub fn first_ready(&self) -> Option<usize> {
        self.games.list.iter().position(|e| matches!(e.state, State::Ready(_)))
    }

    /// A battle the demo's buttons play, against a stand-in of 1 HP (a set
    /// decided at the first hits): the Play screen's game, a random match.
    pub fn tour_battle(&mut self) {
        let Some(game) = self.play.game.clone() else { return };
        let Some(ready) = self.games.ready(&game) else { return };
        let content = ready.content();
        let Ok(mut m) = nettai_match::pick::live(content, &game, self.play.seed, None) else { return };
        let base_hp = nettai_battle::content::PlayerFact::BaseHp.name();
        let one = [nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1))];
        if m.sides[1].set_fact(content, base_hp, &one).is_err() {
            return;
        }
        self.start_live(&game, &ready, m, self.play.seed);
        if let Some(b) = &mut self.battle {
            b.autoplay = true;
        }
    }

    /// The battle at eight times the speed.
    pub fn tour_fast(&mut self) {
        if let Some(b) = &mut self.battle {
            for _ in 0..3 {
                b.stage.player.faster();
            }
        }
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.chars().take(16).collect();
        let ui = self.ui();
        // (The field keeps its text while it is typed in: a longer one is
        // cut when it's left.)
        if self.name != name && !ui.global::<Input>().get_editing() {
            ui.set_name(self.name.as_str().into());
        }
        let mut me = ui.get_lobby_me();
        me.name = self.name.as_str().into();
        ui.set_lobby_me(me);
    }

    // ---- The replays ----------------------------------------------------

    fn show_replays(&mut self) {
        let ui = self.ui();
        ui.set_replays_folder(replays_dir().display().to_string().into());
        self.replays = replays::scan(&replays_dir());
        let count = self.replays.len() as i32;
        ui.set_replays_cursor(ui.get_replays_cursor().clamp(0, (count - 1).max(0)));
        self.show_replay_rows();
        self.check_replays();
    }

    fn show_replay_rows(&mut self) {
        let rows: Vec<ReplayRow> = self.replays.iter().map(|e| replays::row(e, &self.games, self.lang)).collect();
        self.replay_rows.set_vec(rows);
    }

    /// Play out, on a thread, each replay whose game is loaded and that
    /// isn't played out yet.
    fn check_replays(&mut self) {
        for id in self.ready_games() {
            let Some(ready) = self.games.ready(&id) else { continue };
            let todo: Vec<(PathBuf, Replay)> = self
                .replays
                .iter_mut()
                .filter(|e| e.outcome.is_none())
                .filter_map(|e| e.replay.as_ref().ok().filter(|r| r.head.game == id).map(|r| (e.path.clone(), r.clone())))
                .collect();
            if todo.is_empty() {
                continue;
            }
            let content = ready.content().clone();
            std::thread::spawn(move || {
                for (path, replay) in todo {
                    let outcome = nettai_frontend::replay::play_out(&content, &replay);
                    let _ = slint::invoke_from_event_loop(move || crate::with_app(|app| app.played_out(&path, outcome)));
                }
            });
        }
        self.show_replay_rows();
    }

    fn played_out(&mut self, path: &std::path::Path, outcome: Result<nettai_frontend::replay::Outcome, String>) {
        let Some(at) = self.replays.iter().position(|e| e.path == path) else { return };
        self.replays[at].outcome = Some(outcome);
        if at < self.replay_rows.row_count() {
            self.replay_rows.set_row_data(at, replays::row(&self.replays[at], &self.games, self.lang));
        }
    }

    // ---- Settings -------------------------------------------------------

    fn show_settings(&self) {
        let ui = self.ui();
        ui.set_settings_language(lang::name(self.lang).into());
        ui.set_settings_volume(self.volume as i32);
        ui.set_settings_menu_sounds(self.sound.menu_sounds);
        ui.set_settings_crisp_text(self.text == TextMode::Font);
        ui.set_settings_sharp(self.sharp);
        ui.set_name(self.name.as_str().into());
    }

    /// A setting stepped: its row, and which way.
    pub fn settings_step(&mut self, row: i32, by: i32) {
        match row {
            0 => self.set_language(lang::step(self.lang, by)),
            1 => {
                self.volume = (self.volume as i32 + by).clamp(0, 10) as u32;
                self.sound.set_volume(self.volume);
            }
            2 => self.sound.menu_sounds = !self.sound.menu_sounds,
            // (The next battle's.)
            3 => self.text = if self.text == TextMode::Font { TextMode::Original } else { TextMode::Font },
            4 => self.sharp = !self.sharp,
            _ => {}
        }
        self.show_settings();
    }

    // ---- The frame ------------------------------------------------------

    /// Before a frame is drawn at `now`: the hands, the wipe, the battle's
    /// ticks and its picture, the sound. What the gamepad asks of the
    /// menus, for the caller to give them (after this borrow).
    pub fn frame(&mut self, now: Instant) -> Vec<NavAction> {
        // (The last frame was drawn, if the renderer didn't say so.)
        self.drawn();
        let ui = self.ui();
        let pad = self.pad.poll();
        if pad.used {
            ui.global::<Input>().set_pad(true);
        }
        self.step_transition(now);
        let screen = ui.get_screen();
        let mut nav = Vec::new();
        let factor = ui.window().scale_factor();
        let area = (ui.get_picture_area_width() * factor, ui.get_picture_area_height() * factor);
        let mut samples = Vec::new();
        match screen {
            Screen::Battle if self.battle.is_some() => {
                if self.battle.as_ref().is_some_and(Battle::playing) {
                    if let Some(at) = pad.pressed {
                        self.keys.pressed.push(at);
                    }
                    if pad.pause {
                        self.battle_pause();
                    }
                } else {
                    nav = pad.nav;
                }
                let mut buttons = self.keys.held | self.pad.held() | self.touch;
                let b = self.battle.as_mut().expect("matched");
                if b.paused {
                    // (Netplay goes on under its pause: nothing held.)
                    buttons = 0;
                }
                if b.autoplay {
                    buttons = demo_buttons(b.stage.player.battle(), b.stage.player.ticks() as u32);
                }
                b.stage.advance(now, buttons, &mut self.keys.pressed);
                let fit = stage::fit(area.0, area.1, factor, self.sharp);
                if let Some(image) = b.stage.picture(fit) {
                    ui.set_picture(image);
                    ui.set_scale(fit.scale as i32);
                }
                std::mem::swap(&mut samples, &mut b.stage.samples);
                b.stage.report(now);
                self.follow_battle();
                self.show_connection();
            }
            Screen::Title => {
                nav = pad.nav;
                self.keys.pressed.clear();
                self.attract(now, area, factor);
            }
            Screen::Lobby => {
                nav = pad.nav;
                self.keys.pressed.clear();
                self.poll_lobby(now);
            }
            _ => {
                nav = pad.nav;
                self.keys.pressed.clear();
            }
        }
        self.sound.frame(now, &mut samples);
        if self.transition.is_some() {
            nav.clear();
        }
        nav
    }

    /// The frame was drawn.
    pub fn drawn(&mut self) {
        if let Some(b) = &mut self.battle {
            b.stage.drawn();
        }
    }

    /// What the battle says, shown: its result, why it stopped, the pips,
    /// a replay's place.
    fn follow_battle(&mut self) {
        let ui = self.ui();
        let Some(b) = &mut self.battle else { return };
        if let Kind::Replay { replay, side, starts, .. } = &b.kind {
            let played = b.stage.player.ticks() as usize;
            let round = starts.iter().filter(|&&s| s <= played).count();
            ui.set_battle_playback(Playback {
                round: round as i32,
                tick: (played - starts.get(round.saturating_sub(1)).copied().unwrap_or(0)) as i32,
                played: played as i32,
                total: replay.ticks.len() as i32,
                speed: b.stage.player.speed() as f32,
                side: *side as i32,
            });
        }
        let result = match b.stage.player.result() {
            Some(BattleResult::Won) => 1,
            Some(BattleResult::Lost) => 2,
            Some(BattleResult::Drawn) => 3,
            _ => 0,
        };
        // (A replay's result is side 0's: the console shown's, for the
        // banner.)
        let result = match (&b.kind, result) {
            (Kind::Replay { side: 1, .. }, 1) => 2,
            (Kind::Replay { side: 1, .. }, 2) => 1,
            (_, r) => r,
        };
        let finished = b.stage.player.finished();
        if finished && b.shown_result == 0 && !b.shown_stopped {
            b.shown_result = result.max(3 * (result == 0) as i32);
            ui.set_battle_result(b.shown_result);
            ui.set_battle_cursor(0);
            ui.set_playing(false);
            if let Some(path) = &b.recorded {
                ui.set_battle_recorded(path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().into());
            }
            self.show_pips();
            return;
        }
        if let Some(why) = b.stage.player.stopped()
            && !finished
            && !b.shown_stopped
        {
            b.shown_stopped = true;
            ui.set_battle_stopped(why.into());
            ui.set_battle_cursor(0);
            ui.set_playing(false);
            return;
        }
        if b.stage.player.session().new_round {
            self.show_pips();
        }
    }

    /// The title's demo battle: a random set of a game that is ready, both
    /// sides pressing through their custom screens, the left one fighting;
    /// another when it ends.
    fn attract(&mut self, now: Instant, area: (f32, f32), factor: f32) {
        let ui = self.ui();
        let over = self.attract.as_ref().is_none_or(|a| a.stage.player.stopped().is_some());
        if over {
            let ready = self.ready_games();
            if ready.is_empty() {
                ui.set_scale(0);
                return;
            }
            let seed = clock_seed();
            let game = ready[seed as usize % ready.len()].clone();
            let ready = self.games.ready(&game).expect("listed as ready");
            let Ok(m) = nettai_match::pick::live(ready.content(), &game, seed, None) else { return };
            let set = nettai_match::Set::of(ready.content(), &m, seed);
            let player = self.player(&ready, Box::new(LivePlayer::new(set)), false);
            ui.set_monitor_game(game_names(&game).0.into());
            self.sound.use_game(&game, &ready.loaded);
            self.attract = Some(Attract { stage: Stage::new(player), game, ticks: 0 });
        }
        let a = self.attract.as_mut().expect("made above");
        let buttons = demo_buttons(a.stage.player.battle(), a.ticks);
        a.ticks += a.stage.advance(now, buttons, &mut Vec::new());
        a.stage.samples.clear();
        // (The monitor: at most three times the frame.)
        let fit = stage::fit(area.0.min(240.0 * 3.0 * factor), area.1.min(160.0 * 3.0 * factor), factor, self.sharp);
        if let Some(image) = a.stage.picture(fit) {
            ui.set_picture(image);
            ui.set_scale(fit.scale as i32);
        }
    }
}

/// The demo's buttons: the custom screen pressed as the stand-in presses
/// it; in the fight, the buster, a chip now and then, a step up or down
/// (through every row in turn), and L now and then (the custom screen, once
/// its gauge is full).
fn demo_buttons(b: &nettai_battle::Battle, tick: u32) -> u16 {
    use nettai_battle::battle::mode;
    use nettai_battle::input::keys;
    if b.round.mode == mode::CUSTOM {
        return nettai_frontend::driver::bot_buttons(b, 0, tick);
    }
    let mut k = 0;
    if tick % 9 < 2 {
        k |= keys::B;
    }
    if tick % 83 == 40 {
        k |= keys::A;
    }
    if tick % 240 == 200 {
        k |= keys::L;
    }
    // (Up, up, down, down: each row a while; a direction is held a few
    // ticks, as it acts on a hold's second.)
    if (45..48).contains(&(tick % 90)) {
        k |= if (tick / 90) % 4 < 2 { keys::UP } else { keys::DOWN };
    }
    k
}

/// The match files of game `game` in the matches folder, by name.
fn match_files(game: &str) -> Vec<PathBuf> {
    let Ok(dir) = std::fs::read_dir(matches_dir()) else { return Vec::new() };
    let mut files: Vec<PathBuf> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .filter(|p| std::fs::read_to_string(p).ok().and_then(|t| nettai_match::file::game_of(&t).ok()).is_some_and(|g| g == game))
        .collect();
    files.sort();
    files
}
