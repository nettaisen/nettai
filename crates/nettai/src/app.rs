//! The app's state and its loop: the screen shown (the top bar's tabs, the
//! first run's welcome, the creator, the battle) and the wipe into and out
//! of a battle, the games and the selection Play and Training share, the
//! battle (and the welcome's demo battle), the sound and the hands. The window's rendering is its clock:
//! [`App::frame`] runs before each frame is drawn.

use crate::games::{Games, Names, Ready, State};
use crate::input::{BattleKey, Keys, Pad};
use crate::replays;
use crate::sound::Sound;
use crate::stage::{self, Stage};
use crate::{
    AppWindow, BattleKind, GameCard, GameState, Input, MatchPreview, NavAction, Playback, ReplayRow, Screen, Side, Strings,
    TrainingText, UiSound, lang,
};
use nettai_battle::BattleResult;
use nettai_frontend::driver::{Driver, LivePlayer, Opponent};
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
    /// Training: a set against the computer, its game, match, seed and
    /// opponent (starting over plays the same set again, a rematch the
    /// match with another seed).
    Live { game: String, m: nettai_match::Match, seed: u32, opponent: Opponent },
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

/// The welcome's demo battle: a random set of a game, both sides pressing
/// through their custom screens, the left one fighting.
struct Attract {
    stage: Stage,
    game: String,
    ticks: u32,
}

/// What the selector strip has chosen, Play's and Training's alike (as
/// Tango's loadout is the app's): the game, and the build brought.
#[derive(Default)]
pub struct Selection {
    pub game: Option<String>,
    /// 0 none of the player's (a random side, or a match file's); else
    /// `builds[build - 1]`.
    pub build: usize,
    /// The game's builds: their files, names and sides.
    pub builds: Vec<(PathBuf, String, nettai_match::Side)>,
}

/// What the Training screen has chosen besides the selection.
#[derive(Default)]
struct TrainingChoice {
    /// 0: random (from `seed`); else the match file `files[source - 1]`.
    source: usize,
    seed: u32,
    files: Vec<PathBuf>,
    /// The opponent's side: 0 the match's (a random one, or the file's);
    /// else the build `builds[opponent_build - 1]` of the selection's.
    opponent_build: usize,
    /// What the opponent does: `Opponent::ALL[opponent]`.
    opponent: usize,
    /// The set goes on round after round (`nettai_match::MAX_ROUNDS` of
    /// them), not as the match lists them.
    endless: bool,
}

/// The arenas the Training screen shows of a match: its first rounds'.
const ARENAS_SHOWN: usize = nettai_match::TRIPLE_BATTLE;

/// The most pips an endless set shows on each side.
const MAX_PIPS: usize = 12;

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
    /// The game and the build Play and Training bring.
    pub select: Selection,
    training: TrainingChoice,
    pub lobby: crate::lobby::LobbyState,
    /// The player's builds, and the one open in the creator.
    pub builds: crate::builds::screen::BuildsState,
    replays: Vec<replays::Entry>,
    replay_rows: Rc<VecModel<ReplayRow>>,
    /// The replays shown: their places in `replays`, as the filter keeps
    /// them; the filter (0 all, else the game `games.list[filter - 1]`).
    replay_shown: Vec<usize>,
    replays_filter: usize,
    transition: Option<(Instant, Screen, bool)>,
    /// Present the picture at the display's density.
    pub sharp: bool,
    pub text: TextMode,
    pub volume: u32,
    /// The player's name (netplay, and the replays they record).
    pub name: String,
    /// The window's texture the picture is written into (femtovg's
    /// OpenGL); none: a new image each picture.
    pub gl: Option<crate::gl::GlPicture>,
}

/// A seed from the clock.
pub fn clock_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() ^ d.as_secs() as u32).unwrap_or(1)
}

/// The folder replays are written to and listed from.
pub fn replays_dir() -> PathBuf {
    crate::paths::replays()
}

/// The folder match files are listed from.
pub fn matches_dir() -> PathBuf {
    crate::paths::matches()
}

/// A game's short name and its number, from its id (`exe6`: EXE6, 6).
pub fn game_names(id: &str) -> (String, String) {
    let number: String = id.chars().filter(|c| c.is_ascii_digit()).collect();
    (id.to_ascii_uppercase(), number)
}

impl App {
    pub fn new(ui: &AppWindow) -> App {
        // The settings kept (`NETTAI_LANG` still names the language).
        let saved = crate::settings::load();
        let lang = match (std::env::var_os("NETTAI_LANG"), saved.language.as_deref()) {
            (None, Some(code)) => lang::LANGUAGES.iter().map(|(c, _)| *c).find(|c| *c == code).unwrap_or_else(lang::initial),
            _ => lang::initial(),
        };
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
            select: Selection::default(),
            training: TrainingChoice { seed: clock_seed(), ..TrainingChoice::default() },
            lobby: crate::lobby::LobbyState::default(),
            builds: Default::default(),
            replays: Vec::new(),
            replay_rows,
            replay_shown: Vec::new(),
            replays_filter: 0,
            transition: None,
            sharp: std::env::var_os("NETTAI_PHYSICAL_PIXELS").is_some() || saved.sharp == Some(true),
            text: if saved.crisp_text == Some(false) { TextMode::Original } else { TextMode::Font },
            volume: saved.volume.unwrap_or(8).min(10),
            name: saved.name.clone().unwrap_or_default(),
            gl: None,
        };
        let mut app = app;
        if let Some(on) = saved.menu_sounds {
            app.sound.menu_sounds = on;
        }
        app.sound.set_volume(app.volume);
        app.show_settings();
        app
    }

    /// Keep the settings as they are now (`crate::settings`).
    pub fn save_settings(&self) {
        // (The tour's walk changes them as it goes: none are kept.)
        if std::env::var_os("NETTAI_TOUR").is_some() {
            return;
        }
        let saved = crate::settings::Saved {
            language: Some(self.lang.to_string()),
            volume: Some(self.volume),
            menu_sounds: Some(self.sound.menu_sounds),
            crisp_text: Some(self.text == TextMode::Font),
            sharp: Some(self.sharp),
            name: Some(self.name.clone()),
        };
        if let Err(e) = crate::settings::save(&saved) {
            eprintln!("nettai: the settings aren't kept: {e}");
        }
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

    /// The screen is now `screen`. (A tab chosen in the top bar is entered
    /// at once; a battle, and the creator, with the wipe: `go`.)
    pub fn enter(&mut self, screen: Screen) {
        let ui = self.ui();
        if ui.get_screen() == Screen::Battle && screen != Screen::Battle {
            self.battle = None;
            ui.set_playing(false);
        }
        ui.set_screen(screen);
        ui.invoke_focus_keys();
        match screen {
            Screen::Play => self.show_play(),
            Screen::Training => self.show_training(),
            Screen::Replays => self.show_replays(),
            Screen::Settings => self.show_settings(),
            Screen::Builds => self.show_builds(),
            Screen::Build => self.enter_build(),
            Screen::Welcome => {
                self.show_games();
                if let Some(a) = &mut self.attract {
                    a.stage.hold();
                    a.stage.stale = true;
                }
            }
            Screen::Battle => ui.set_playing(self.battle.as_ref().is_some_and(Battle::playing)),
        }
    }

    /// The first screen: the welcome on a first run (no name yet), else
    /// Play.
    pub fn start(&mut self) {
        self.enter(if self.name.trim().is_empty() { Screen::Welcome } else { Screen::Play });
    }

    /// The welcome's language a step on.
    pub fn welcome_language(&mut self, by: i32) {
        self.settings_step(0, by);
    }

    /// The welcome done: with a name, on to Play.
    pub fn welcome_done(&mut self) {
        if self.name.trim().is_empty() {
            self.sound.play(UiSound::Refused);
            self.ui().set_welcome_cursor(0);
            return;
        }
        self.save_settings();
        self.go(Screen::Play);
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
                    // (`NETTAI_TRAINING=<game>`: straight into a random training set of it.)
                    if std::env::var("NETTAI_TRAINING").is_ok_and(|g| g == id) && app.battle.is_none() {
                        app.select_game(&id);
                        app.training_fight();
                    }
                    app.auto_netplay(&id);
                    match app.ui().get_screen() {
                        Screen::Replays => app.check_replays(),
                        Screen::Play => app.show_play(),
                        Screen::Training => app.show_training(),
                        Screen::Builds => app.show_builds(),
                        _ => {}
                    }
                })
            });
        }
        self.show_games();
    }

    /// The game cards, as the games stand (the welcome's), and the strip.
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
        if self.select.game.is_none()
            && let Some(first) = self.games.list.iter().find(|e| matches!(e.state, State::Ready(_))).map(|e| e.id.clone())
        {
            // (The first game ready, until the player chooses.)
            self.select_game(&first);
        }
        self.show_select();
    }

    // ---- The selection (the strip's) ------------------------------------

    /// The selection is game `id`, with none of the player's builds.
    pub fn select_game(&mut self, id: &str) {
        self.select.game = Some(id.to_string());
        self.select.build = 0;
        self.training.opponent_build = 0;
        self.training.source = 0;
        self.training.files = match_files(id);
        self.refresh_builds();
        if let Some(ready) = self.games.ready(id) {
            self.sound.use_game(id, &ready.loaded);
        }
    }

    /// The selection's builds read again (one made or edited since), the
    /// one chosen kept by its file.
    pub fn refresh_builds(&mut self) {
        let chosen = self.select.build.checked_sub(1).and_then(|b| self.select.builds.get(b)).map(|(p, _, _)| p.clone());
        let opponent = self.training.opponent_build.checked_sub(1).and_then(|b| self.select.builds.get(b)).map(|(p, _, _)| p.clone());
        self.select.builds = match self.select.game.as_deref().and_then(|g| self.games.ready(g).map(|r| (g.to_string(), r))) {
            Some((g, r)) => crate::builds::screen::BuildsState::choices(r.content(), &g),
            None => Vec::new(),
        };
        let place = |p: Option<PathBuf>, builds: &[(PathBuf, String, nettai_match::Side)]| p.and_then(|p| builds.iter().position(|(q, _, _)| *q == p)).map_or(0, |i| i + 1);
        self.select.build = place(chosen, &self.select.builds);
        self.training.opponent_build = place(opponent, &self.select.builds);
    }

    /// The selection's build's side, if it is one of the player's.
    pub fn selected_side(&self) -> Option<nettai_match::Side> {
        self.select.build.checked_sub(1).and_then(|b| self.select.builds.get(b)).map(|(_, _, s)| s.clone())
    }

    /// The selection's build's name, if it is one of the player's.
    pub fn selected_name(&self) -> Option<String> {
        self.select.build.checked_sub(1).and_then(|b| self.select.builds.get(b)).map(|(_, n, _)| n.clone())
    }

    /// The strip, as the selection is.
    pub fn show_select(&mut self) {
        let ui = self.ui();
        let entry = self.select.game.as_deref().and_then(|g| self.games.list.iter().find(|e| e.id == g));
        let (name, state) = match entry {
            Some(e) => (
                game_names(&e.id).0,
                match &e.state {
                    State::NoPack(_) => GameState::NoPack,
                    State::NotLoaded | State::Loading => GameState::Loading,
                    State::Ready(_) => GameState::Ready,
                    State::Failed(_) => GameState::Failed,
                },
            ),
            None => (String::new(), GameState::NoPack),
        };
        ui.set_select_game(name.into());
        ui.set_select_game_state(state);
        let build = self.selected_name().map_or_else(|| ui.global::<Strings>().invoke_random(), SharedString::from);
        ui.set_select_build(build);
        ui.set_select_build_index(self.select.build as i32);
        let navi = match (self.selected_side(), self.select.game.as_deref().and_then(|g| self.games.ready(g))) {
            (Some(side), Some(ready)) => {
                let graphics = ready.graphics(self.lang);
                Names::of(ready.content(), &graphics).navi(side.navi(ready.content()))
            }
            _ => String::new(),
        };
        ui.set_select_build_navi(navi.into());
        ui.set_select_can_act(self.select.game.as_deref().is_some_and(|g| self.games.ready(g).is_some()));
    }

    /// The strip's game (row 0) or build (row 1) a step on.
    pub fn select_step(&mut self, row: i32, by: i32) {
        if row == 0 {
            let ids: Vec<String> = self.games.list.iter().map(|e| e.id.clone()).collect();
            if ids.is_empty() {
                return;
            }
            let at = self.select.game.as_ref().and_then(|g| ids.iter().position(|i| i == g)).unwrap_or(0) as i32;
            let next = ids[(at + by).rem_euclid(ids.len() as i32) as usize].clone();
            self.select_game(&next);
        } else {
            let n = self.select.builds.len() as i32 + 1;
            self.select.build = (self.select.build as i32 + by).rem_euclid(n) as usize;
        }
        self.after_select();
    }

    /// The strip's action: its build in the creator, or a new build.
    pub fn select_act(&mut self) {
        let Some(game) = self.select.game.clone() else { return self.sound.play(UiSound::Refused) };
        let chosen = self.select.build.checked_sub(1).and_then(|b| self.select.builds.get(b)).cloned();
        self.edit_selected(&game, chosen);
    }

    /// After the selection changed: the tab shown again, a room told.
    fn after_select(&mut self) {
        self.propose();
        match self.ui().get_screen() {
            Screen::Play => self.show_play(),
            Screen::Training => self.show_training(),
            _ => self.show_select(),
        }
    }

    /// The games ready to play, by id.
    pub fn ready_games(&self) -> Vec<String> {
        self.games.list.iter().filter(|e| matches!(e.state, State::Ready(_))).map(|e| e.id.clone()).collect()
    }

    // ---- Training -------------------------------------------------------

    pub fn show_training(&mut self) {
        self.refresh_builds();
        self.show_select();
        self.show_preview();
    }

    /// The opponent's side: the match's (0), or a build of the game.
    pub fn training_opponent(&mut self, build: usize) {
        self.training.opponent_build = build.min(self.select.builds.len());
        self.show_preview();
    }

    /// What the opponent does: `Opponent::ALL[i]`.
    pub fn training_behavior(&mut self, i: usize) {
        self.training.opponent = i.min(Opponent::ALL.len() - 1);
        self.show_preview();
    }

    pub fn training_endless(&mut self, on: bool) {
        self.training.endless = on;
        self.show_preview();
    }

    pub fn training_source(&mut self, source: usize) {
        self.training.source = source.min(self.training.files.len());
        self.show_preview();
    }

    pub fn training_reroll(&mut self) {
        self.training.seed = clock_seed();
        self.show_preview();
    }

    /// The match the Training screen has chosen, and its seed, or why it
    /// can't be played.
    fn training_match(&self, ready: &Ready) -> Result<(nettai_match::Match, u32), String> {
        let game = self.select.game.as_deref().ok_or("no game is chosen")?;
        let content = ready.content();
        let (mut m, seed) = if self.training.source == 0 {
            (nettai_match::pick::live(content, game, self.training.seed, None)?, self.training.seed)
        } else {
            let path = &self.training.files[self.training.source - 1];
            let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let m = nettai_match::parse(content, &text).map_err(|problems| problems.join("\n"))?;
            let seed = m.seed.unwrap_or(self.training.seed);
            (m, seed)
        };
        // (A build of yours, in place of your side or the opponent's: one
        // that can't play says why.)
        let build = |b: usize| b.checked_sub(1).and_then(|b| self.select.builds.get(b)).map(|(_, _, side)| side.clone());
        let built = [build(self.select.build), build(self.training.opponent_build)];
        for (side, b) in built.iter().enumerate() {
            if let Some(b) = b {
                m.sides[side] = b.clone();
            }
        }
        if self.training.endless {
            m.rounds.resize(nettai_match::MAX_ROUNDS, nettai_match::RoundSettings::default());
        }
        if built.iter().any(Option::is_some) {
            let problems = nettai_match::check_match(content, &m);
            if !problems.is_empty() {
                return Err(problems.join("\n"));
            }
        }
        Ok((m, seed))
    }

    /// What the opponent the Training screen has chosen does.
    fn training_opponent_does(&self) -> Opponent {
        Opponent::ALL.get(self.training.opponent).copied().unwrap_or_default()
    }

    fn show_preview(&mut self) {
        let ui = self.ui();
        let mut sources: Vec<SharedString> = vec![ui.global::<Strings>().invoke_random()];
        sources.extend(self.training.files.iter().map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default().into()));
        ui.set_training_sources(ModelRc::new(VecModel::from(sources)));
        ui.set_training_source_index(self.training.source as i32);
        let strings = ui.global::<Strings>();
        let mut builds: Vec<SharedString> = vec![if self.training.source == 0 { strings.invoke_random() } else { strings.invoke_from_the_match() }];
        builds.extend(self.select.builds.iter().map(|(_, name, _)| SharedString::from(name.as_str())));
        ui.set_training_opponents(ModelRc::new(VecModel::from(builds)));
        ui.set_training_opponent_index(self.training.opponent_build as i32);
        ui.set_training_behavior_index(self.training.opponent as i32);
        ui.set_training_endless_on(self.training.endless);
        ui.set_training_rounds(nettai_match::TRIPLE_BATTLE as i32);
        let Some(ready) = self.select.game.as_deref().and_then(|g| self.games.ready(g)) else {
            ui.set_training_chosen(false);
            ui.set_training_preview(MatchPreview::default());
            return;
        };
        ui.set_training_chosen(true);
        let mut preview = MatchPreview { ready: true, seed: self.training.seed.to_string().into(), ..MatchPreview::default() };
        match self.training_match(&ready) {
            Ok((m, seed)) => {
                let graphics = ready.graphics(self.lang);
                let content = ready.content();
                let names = Names::of(content, &graphics);
                preview.left = names.navi(m.sides[0].navi(content)).into();
                preview.right = names.navi(m.sides[1].navi(content)).into();
                preview.seed = seed.to_string().into();
                // (An endless set's first arenas: each is played to be drawn.)
                let first = nettai_match::Match { rounds: m.rounds.iter().take(ARENAS_SHOWN).cloned().collect(), ..m.clone() };
                if !self.training.endless {
                    ui.set_training_rounds(m.rounds.len() as i32);
                }
                preview.rounds = ModelRc::new(VecModel::from(crate::arenas::pictures(&ready, &graphics, &first, seed)));
            }
            Err(why) => preview.problem = why.into(),
        }
        ui.set_training_preview(preview);
    }

    /// FIGHT: the chosen match against the chosen opponent.
    pub fn training_fight(&mut self) {
        let Some(game) = self.select.game.clone() else { return };
        let Some(ready) = self.games.ready(&game) else { return };
        let Ok((m, seed)) = self.training_match(&ready) else { return };
        self.start_live(&game, &ready, m, seed, self.training_opponent_does());
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

    /// A training set of `m` from `seed` against `opponent` (whose random
    /// presses start from the seed too), recorded.
    fn start_live(&mut self, game: &str, ready: &Ready, m: nettai_match::Match, seed: u32, opponent: Opponent) {
        let content = ready.content();
        let driver = LivePlayer::against(nettai_match::Set::of(content, &m, seed), opponent, seed as u64);
        let mut player = self.player(ready, Box::new(driver), true);
        // Recorded, to watch again.
        let recorded = self.recorder(content, &m, seed, game, 0, None).and_then(|(r, path)| player.record(r).ok().map(|_| path));
        let graphics = ready.graphics(self.lang);
        let names = Names::of(content, &graphics);
        let ui = self.ui();
        let left = Side { name: self.name.as_str().into(), navi: names.navi(m.sides[0].navi(content)).into(), detail: SharedString::default() };
        let does = Opponent::ALL.iter().position(|&o| o == opponent).unwrap_or(0) as i32;
        let right = Side { name: ui.global::<TrainingText>().invoke_opponent(does), navi: names.navi(m.sides[1].navi(content)).into(), detail: SharedString::default() };
        let rounds = m.rounds.len();
        self.show_battle(Battle {
            stage: Stage::new(player),
            kind: Kind::Live { game: game.to_string(), m, seed, opponent },
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
        let Some(entry) = self.replay_shown.get(index).and_then(|&i| self.replays.get(i)) else { return };
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
        ui.set_battle_cursor(0);
        ui.set_battle_playback(Playback::default());
        self.show_pips();
        self.keys.release();
    }

    /// A recorder of the set to a new file in the replays folder, this
    /// player on `side`, both players' names by side (`names`; none: this
    /// player's alone).
    pub fn recorder(
        &self,
        content: &std::sync::Arc<nettai_battle::Content>,
        m: &nettai_match::Match,
        seed: u32,
        game: &str,
        side: u8,
        names: Option<[String; 2]>,
    ) -> Option<(Recorder, PathBuf)> {
        let dir = replays_dir();
        std::fs::create_dir_all(&dir).ok()?;
        let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let stamp = chrono::DateTime::from_timestamp(when as i64, 0).map(|t| t.with_timezone(&chrono::Local).format("%Y%m%d-%H%M%S").to_string()).unwrap_or_default();
        let path = dir.join(format!("{stamp}-{game}.ntrp"));
        let file = std::fs::File::create(&path).ok()?;
        let names = names.unwrap_or_else(|| {
            let mut names = [String::new(), String::new()];
            names[side as usize] = self.name.clone();
            names
        });
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
        // (An endless set's, the rounds won alone: it is decided at 50.)
        let need = if b.rounds >= nettai_match::MAX_ROUNDS { wins.max(losses).min(MAX_PIPS) } else { b.rounds / 2 + 1 };
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
        // (Training's set again from its start, the same draw: recorded
        // anew, as every set played is.)
        if let Some(Battle { kind: Kind::Live { game, m, seed, opponent }, .. }) = &self.battle {
            let (game, m, seed, opponent) = (game.clone(), m.clone(), *seed, *opponent);
            let Some(ready) = self.games.ready(&game) else { return };
            self.start_live(&game, &ready, m, seed, opponent);
            self.ui().set_playing(true);
            return;
        }
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
            Some(Battle { kind: Kind::Live { game, m, opponent, .. }, .. }) => {
                let Some(ready) = self.games.ready(&game) else { return };
                self.start_live(&game, &ready, m, clock_seed(), opponent);
            }
            Some(Battle { kind: Kind::Netplay { .. }, .. }) => {
                self.go(Screen::Play);
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
            Some(Kind::Netplay { .. }) => Screen::Play,
            _ => Screen::Training,
        };
        self.go(back);
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
        self.show_settings();
        match ui.get_screen() {
            Screen::Training => self.show_training(),
            Screen::Replays => self.show_replay_rows(),
            Screen::Play => self.show_play(),
            Screen::Builds => self.show_builds(),
            Screen::Build => self.enter_build(),
            Screen::Welcome => self.show_games(),
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
        let training = matches!(self.battle.as_ref().map(|b| &b.kind), Some(Kind::Live { .. }));
        match key {
            Some(BattleKey::Pause) => self.battle_pause(),
            Some(BattleKey::Restart) if training => self.battle_restart(),
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
    /// decided at the first hits): the selection's game, a random match.
    pub fn tour_battle(&mut self) {
        let Some(game) = self.select.game.clone() else { return };
        let Some(ready) = self.games.ready(&game) else { return };
        let content = ready.content();
        let Ok(mut m) = nettai_match::pick::live(content, &game, self.training.seed, None) else { return };
        let base_hp = nettai_battle::content::PlayerFact::BaseHp.name();
        let one = [nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(1))];
        if m.sides[1].set_fact(content, base_hp, &one).is_err() {
            return;
        }
        self.start_live(&game, &ready, m, self.training.seed, Opponent::StandIn);
        if let Some(b) = &mut self.battle {
            b.autoplay = true;
        }
    }

    /// The Builds screen of the first game ready, with a build of a random
    /// match's side there ("Tour", made again each time).
    pub fn tour_build(&mut self) {
        // (`NETTAI_TOUR_GAME`: that game's, where it is ready.)
        let asked = std::env::var("NETTAI_TOUR_GAME").ok().filter(|g| self.games.ready(g).is_some());
        let Some(game) = asked.or_else(|| self.first_ready().map(|i| self.games.list[i].id.clone())) else { return };
        let Some(ready) = self.games.ready(&game) else { return };
        let content = ready.content();
        if let Ok(m) = nettai_match::pick::live(content, &game, 5, None) {
            let path = crate::builds::store::folder(&game).join("tour.toml");
            let _ = crate::builds::store::write(&path, content, &game, "Tour", &m.sides[0]);
        }
        self.builds.game = self.ready_games().iter().position(|g| *g == game).unwrap_or(0);
        self.builds.editor = None;
        self.enter(Screen::Builds);
    }

    /// The selection: the Builds screen's game, its first build (the tour's).
    pub fn tour_select(&mut self) {
        if let Some(game) = self.builds_game_id() {
            self.select_game(&game);
            self.select.build = 1.min(self.select.builds.len());
        }
    }

    /// The selection: the first game ready, a random side.
    pub fn tour_select_random(&mut self) {
        if let Some(i) = self.first_ready() {
            let id = self.games.list[i].id.clone();
            self.select_game(&id);
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
        // (As the lobby carries it: no control characters, 16 at most.)
        self.name = name.chars().filter(|c| !c.is_control()).take(nettai_frontend::lobby::NAME_LENGTH).collect();
        self.lobby_name();
        self.save_settings();
        let ui = self.ui();
        // (The field keeps its text while it is typed in: a longer one is
        // cut when it's left.)
        if !ui.global::<Input>().get_editing() {
            ui.set_name(self.name.as_str().into());
        }
        let mut me = ui.get_play_me();
        me.name = self.name.as_str().into();
        ui.set_play_me(me);
    }

    // ---- The replays ----------------------------------------------------

    fn show_replays(&mut self) {
        let ui = self.ui();
        ui.set_replays_folder(replays_dir().display().to_string().into());
        self.replays = replays::scan(&replays_dir());
        self.show_replay_rows();
        self.check_replays();
    }

    /// The replays the filter keeps, as rows; the filter's games.
    fn show_replay_rows(&mut self) {
        let games: Vec<String> = self.games.list.iter().map(|e| e.id.clone()).collect();
        self.replays_filter = self.replays_filter.min(games.len());
        let kept = self.replays_filter.checked_sub(1).map(|g| games[g].clone());
        self.replay_shown = (0..self.replays.len())
            .filter(|&i| kept.as_ref().is_none_or(|g| self.replays[i].replay.as_ref().is_ok_and(|r| &r.head.game == g)))
            .collect();
        let rows: Vec<ReplayRow> = self.replay_shown.iter().map(|&i| replays::row(&self.replays[i], &self.games, self.lang)).collect();
        self.replay_rows.set_vec(rows);
        let ui = self.ui();
        let filters: Vec<SharedString> = std::iter::once(SharedString::default()).chain(games.iter().map(|g| SharedString::from(game_names(g).0))).collect();
        ui.set_replays_filters(ModelRc::new(VecModel::from(filters)));
        ui.set_replays_filter_index(self.replays_filter as i32);
        ui.set_replays_total(self.replays.len() as i32);
        let count = self.replay_shown.len() as i32;
        ui.set_replays_cursor(ui.get_replays_cursor().clamp(0, (count - 1).max(0)));
    }

    /// The replays' game filter: 0 all, else a game by its place.
    pub fn replays_filter(&mut self, filter: usize) {
        self.replays_filter = filter;
        self.ui().set_replays_cursor(0);
        self.show_replay_rows();
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
        if let Some(row) = self.replay_shown.iter().position(|&i| i == at)
            && row < self.replay_rows.row_count()
        {
            self.replay_rows.set_row_data(row, replays::row(&self.replays[at], &self.games, self.lang));
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
        ui.set_welcome_language_name(lang::name(self.lang).into());
        // About: where things are kept.
        let about = [
            format!("{} · {}", crate::paths::data().display(), "data"),
            format!("{} · {}", crate::paths::builds().display(), "builds"),
            format!("{} · {}", replays_dir().display(), "replays"),
            format!("{} · {}", matches_dir().display(), "matches"),
            format!("{} · {}", crate::netplay::signal_server().unwrap_or_else(|| "—".into()), "NETTAI_SIGNAL"),
        ];
        ui.set_settings_about(ModelRc::new(VecModel::from(about.into_iter().map(SharedString::from).collect::<Vec<_>>())));
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
        self.save_settings();
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
        // (The battle's ticks this frame, while one runs: the menus' sound
        // keeps its time.)
        let mut ticks = None;
        match screen {
            Screen::Battle if self.battle.is_some() => {
                if self.battle.as_ref().is_some_and(Battle::playing) {
                    if let Some(at) = pad.pressed {
                        self.keys.pressed.push(at);
                    }
                    if pad.pause {
                        self.battle_pause();
                    } else if pad.restart && matches!(self.battle.as_ref().map(|b| &b.kind), Some(Kind::Live { .. })) {
                        self.battle_restart();
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
                let ran = b.stage.advance(now, buttons, &mut self.keys.pressed);
                if !b.paused || b.stage.player.real_time() {
                    ticks = Some(ran);
                }
                let fit = stage::fit(area.0, area.1, factor, self.sharp);
                if let stage::Shown::New(image) = b.stage.picture(fit, self.gl.as_mut()) {
                    ui.set_picture(image);
                }
                ui.set_scale(fit.scale as i32);
                std::mem::swap(&mut samples, &mut b.stage.samples);
                b.stage.report(now);
                self.follow_battle();
                self.show_connection();
            }
            Screen::Welcome => {
                nav = pad.nav;
                self.keys.pressed.clear();
                self.attract(now, area, factor);
            }
            _ => {
                nav = pad.nav;
                self.keys.pressed.clear();
                // (A room stays open whichever tab is shown.)
                self.poll_lobby(now);
            }
        }
        self.sound.frame(now, ticks, &mut samples);
        if let Some(s) = self.battle.as_mut().and_then(|b| b.stage.stats.as_mut()) {
            s.audio_queued = s.audio_queued.max(self.sound.queued());
        }
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

    /// The welcome's demo battle: a random set of a game that is ready, both
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
        if let stage::Shown::New(image) = a.stage.picture(fit, self.gl.as_mut()) {
            ui.set_picture(image);
        }
        ui.set_scale(fit.scale as i32);
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
