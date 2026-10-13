//! Training (docs/design/app-ui.md §6): you against a dummy set up from a menu while in training. Its UI is
//! `ui/screens/training.slint`.
//!
//! **The entry** ([`Training`], the screen): two rows, Your build and Dummy's build, both the player's own
//! builds of the selected game and both remembered between runs, and START TRAINING; beside them the two
//! builds facing each other on the arena the session opens on. With no build of the game there is one button,
//! Create a build. A build that can't be used says so in the reserved line, and training doesn't start.
//!
//! **A training session** is a battle against a dummy with the training menu over it, and any screen may start
//! one ([`start`]): Training does from its entry, and Play does beside a waiting invite, with rows of its own at
//! the top of the menu. It has no end and is not recorded: a set that ends starts over. The session is held by
//! the shell (`Shell::training`) and its battle is always this screen's (`ScreenId::Training` owns it), so this
//! module hears its menu and its events whatever screen is shown.
//!
//! **The training menu** (Start or Esc; the game's own pause never runs) has the battle paused behind it. Its
//! rows are a label and a value between arrows, changed at once by Left and Right: [`ROWS`] lists them, and a
//! row is added there.

use crate::battle::{BattleEvent, BattleStart, Play};
use crate::builds::store;
use crate::errors::UserError;
use crate::games::{Names, Ready};
use crate::input::{Action, KeyPress};
use crate::nav::Key;
use crate::screens::menu::clock_seed;
use crate::screens::{Screen, ScreenId};
use crate::shell::Shell;
use crate::sound::Cue;
use crate::stage::Stage;
use nettai_frontend::driver::{InFight, LivePlayer, OnCustomScreen, Opponent};
use nettai_match::build::{Builds, Checked};
use nettai_match::{Match, Set, Side, Wrong};
use nettai_ui::{BattleKind, TrainingBuild, TrainingOption, TrainingRow, TrainingView, VersusCard};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// ---- A training session, which any screen may start ----------------------------------------------------------

/// A build in a training session: its name, as the cards and the menu say it, and the side it brings.
#[derive(Clone, Debug)]
pub struct Build {
    pub name: String,
    pub side: Side,
}

/// A training session to start ([`start`]).
pub struct Start {
    /// The game it is of (ready).
    pub game: String,
    /// The player's build, and the dummy's. Both can be played (`nettai_match::build::Checked::ready`).
    pub you: Build,
    pub dummy: Build,
    /// What the dummy does at first; the training menu changes it.
    pub opponent: Opponent,
    /// What the arenas, each folder's order and the dummy's random presses are picked from.
    pub seed: u32,
    /// The screen that starts it, if not Training itself, and the tag it hears of it by: a row it added was
    /// chosen (`Screen::answered(tag, row)`, the row's place in `rows`). Its session has no CHANGE BUILDS and
    /// no QUIT TO MENU (the screen's own rows are the ways out), and keeps nothing of the dummy's setup.
    pub from: Option<(ScreenId, &'static str)>,
    /// The rows that screen adds at the top of the training menu: their texts, in the language shown.
    pub rows: Vec<String>,
    /// The picture fills the window. Else the screen shown has a `Picture` of its own for it, and the session
    /// lasts while that screen is shown.
    pub full: bool,
    /// The two cards show while it is made ready.
    pub cards: bool,
}

impl Start {
    /// A session of `game` as Training starts one: the dummy stands still and picks the first chips it can,
    /// the picture fills the window after the two cards.
    pub fn new(game: &str, you: Build, dummy: Build) -> Start {
        Start { game: game.to_string(), you, dummy, opponent: Opponent::default(), seed: clock_seed(), from: None, rows: Vec::new(), full: true, cards: true }
    }

    /// A session screen `from` shows beside something of its own while the player waits (Play, by an invite's
    /// code): the dummy a copy of the player's own build, standing still; the picture in the screen's own
    /// `Picture`, with no cards before it; `rows` at the top of the training menu, each heard by `tag`.
    pub fn beside(game: &str, build: Build, from: ScreenId, tag: &'static str, rows: Vec<String>) -> Start {
        Start { from: Some((from, tag)), rows, full: false, cards: false, ..Start::new(game, build.clone(), build) }
    }
}

/// What the shell holds of training (`Shell::training`).
#[derive(Default)]
pub struct Shared {
    /// The session on, if one is.
    session: Option<Session>,
    /// The screenshot pass: the entry is shown as with no build of the game.
    pub tour_no_builds: bool,
}

/// The training session on.
struct Session {
    /// The screen that started it, if not Training, and the tag it hears of it by.
    from: Option<(ScreenId, &'static str)>,
    /// Its picture fills the window.
    full: bool,
    /// The rows its starter added: the menu's first.
    added: usize,
    /// What the dummy does, and whether the input display is on.
    opponent: Opponent,
    inputs: bool,
    /// The menu is open, and the row the focus was last on in it (it opens there).
    menu_open: bool,
    menu_focus: Key,
    /// What the UI was last told: the round, and the buttons each side holds.
    round: Option<u8>,
    held: [u16; 2],
}

/// The training menu's rows, in order, under whatever rows another screen added: each group's title, then its
/// options. A row is added here, worded in training.slint (`TrainingView.label` and `value`), and given what it
/// does in [`values`], [`value`] and [`set`]. (The design's other rows, Your HP, Dummy's HP, Custom gauge, Chip
/// draw, Recording and a second pad, wait for the engine.)
const ROWS: [TrainingOption; 5] = [TrainingOption::Dummy, TrainingOption::Action, TrainingOption::CustomScreen, TrainingOption::Display, TrainingOption::InputDisplay];

/// How many values a row steps through (none: a group's title).
fn values(option: TrainingOption) -> usize {
    match option {
        TrainingOption::Action => InFight::ALL.len(),
        TrainingOption::CustomScreen => OnCustomScreen::ALL.len(),
        TrainingOption::InputDisplay => 2,
        TrainingOption::Added | TrainingOption::Dummy | TrainingOption::Display => 0,
    }
}

/// The value a row has: its place among its values.
fn value(option: TrainingOption, opponent: Opponent, inputs: bool) -> usize {
    match option {
        TrainingOption::Action => InFight::ALL.iter().position(|a| *a == opponent.in_fight).unwrap_or(0),
        TrainingOption::CustomScreen => OnCustomScreen::ALL.iter().position(|c| *c == opponent.on_custom_screen).unwrap_or(0),
        TrainingOption::InputDisplay => inputs as usize,
        TrainingOption::Added | TrainingOption::Dummy | TrainingOption::Display => 0,
    }
}

/// What is remembered between runs, by these keys of the settings' `remembered`: the dummy's setup and the
/// input display (the builds chosen are by game: [`kept_build`]).
const KEPT_ACTION: &str = "training.action";
const KEPT_CUSTOM_SCREEN: &str = "training.custom-screen";
const KEPT_INPUTS: &str = "training.inputs";

/// The dummy's choices by their names in the settings, in `InFight::ALL`'s and `OnCustomScreen::ALL`'s order.
const ACTIONS: [&str; 3] = ["stands-still", "moves-around", "attacks-at-random"];
const CUSTOM_SCREENS: [&str; 2] = ["picks-nothing", "picks-first"];

/// The dummy's setup as it was last left in one of Training's own sessions.
fn kept_opponent(shell: &Shell) -> Opponent {
    let kept = |key: &str, names: &[&str]| shell.settings.remembered.get(key).and_then(|v| names.iter().position(|n| n == v));
    let mut opponent = Opponent::default();
    if let Some(at) = kept(KEPT_ACTION, &ACTIONS) {
        opponent.in_fight = InFight::ALL[at];
    }
    if let Some(at) = kept(KEPT_CUSTOM_SCREEN, &CUSTOM_SCREENS) {
        opponent.on_custom_screen = OnCustomScreen::ALL[at];
    }
    opponent
}

/// Whether the input display is on.
fn inputs_on(settings: &crate::settings::Settings) -> bool {
    settings.remembered.get(KEPT_INPUTS).is_some_and(|v| v == "on")
}

/// Start a training session: the two cards while it is made ready (if it has them), then the battle, with the
/// training menu behind Start and Esc. A session already on ends first. Err: why it can't start, as the player
/// reads it (what the code said is in the log).
pub fn start(shell: &mut Shell, start: Start) -> Result<(), UserError> {
    let Some(ready) = shell.games.ready(&start.game) else {
        crate::errors::log("starting training", &format_args!("{} isn't ready", start.game));
        return Err(UserError::Other);
    };
    let content = ready.content();
    let mut m = Match::empty(content, &start.game).map_err(|why| {
        crate::errors::log("making a training match", &why);
        UserError::Other
    })?;
    m.sides = [start.you.side.clone(), start.dummy.side.clone()];
    // (A set is made of a checked match: both folders whole.)
    if let Some(problem) = nettai_match::check::problems(content, &m).first() {
        crate::errors::log("starting training", &problem.said());
        return Err(UserError::BuildUnusable { reason: None });
    }
    end(shell);
    let ui = shell.ui();
    let view = ui.global::<TrainingView>();
    let cards = start.cards.then(|| {
        let card = |role: slint::SharedString, name: &str, build: &Build| {
            let shown = shown(shell, &start.game, &ready, build);
            VersusCard { role, name: name.into(), build: shown.name, version: shown.version, navi: shown.navi, progress: 0.0 }
        };
        // (The dummy has no name: its build is who it is.)
        let mut cards = [card(view.get_you(), &shell.settings.name, &start.you), card(view.get_dummy(), "", &start.dummy)];
        cards[1].name = std::mem::take(&mut cards[1].build);
        cards
    });
    let inputs = inputs_on(&shell.settings);
    let session = Session {
        from: start.from,
        full: start.full,
        added: start.rows.len(),
        opponent: start.opponent,
        inputs,
        menu_open: false,
        // (The menu opens on its first row: a row its starter added, else the first option.)
        menu_focus: if start.rows.is_empty() { Key::new("menu.option", ROWS.iter().position(|o| values(*o) > 0).unwrap_or(0) as i32) } else { Key::new("menu.added", 0) },
        round: None,
        held: [0; 2],
    };
    let rows: Vec<TrainingRow> = start
        .rows
        .iter()
        .map(|text| TrainingRow { option: TrainingOption::Added, value: 0, text: text.into() })
        .chain(ROWS.iter().map(|o| TrainingRow { option: *o, value: value(*o, session.opponent, inputs) as i32, text: Default::default() }))
        .collect();
    view.set_rows(ModelRc::new(VecModel::from(rows)));
    view.set_leave(start.from.is_none());
    view.set_session_yours(start.you.name.as_str().into());
    view.set_session_dummys(start.dummy.name.as_str().into());
    view.set_inputs(inputs);
    view.set_held_yours(0);
    view.set_held_dummys(0);
    let driver = LivePlayer::against(Set::of(content, &m, start.seed), start.opponent, start.seed as u64);
    shell.start_battle(BattleStart {
        owner: ScreenId::Training,
        kind: BattleKind::Training,
        game: start.game.clone(),
        set: Set::of(content, &m, start.seed),
        play: Play::Drive(Box::new(driver)),
        viewer: 0,
        recorder: None,
        cards,
        full: start.full,
        gate: None,
    });
    shell.training.session = Some(session);
    Ok(())
}

/// End the training session on, if one is: its battle (or the cards before it) goes, and the screen shown is
/// what shows. What starts a session from outside calls this when its wait is over (an opponent joined).
pub fn end(shell: &mut Shell) {
    if shell.training.session.take().is_none() {
        return;
    }
    if shell.loading_of(ScreenId::Training) {
        shell.cancel_loading();
    }
    if ours(shell) {
        shell.end_battle();
    }
    forget(shell);
}

/// Whether a training session is on (being made ready, or running).
pub fn running(shell: &Shell) -> bool {
    shell.training.session.is_some()
}

/// Whether the battle on is a training session's.
fn ours(shell: &Shell) -> bool {
    shell.battle.as_ref().is_some_and(|b| b.owner == ScreenId::Training && b.kind == BattleKind::Training)
}

/// The UI no longer says anything of a session.
fn forget(shell: &Shell) {
    let ui = shell.ui();
    ui.global::<nettai_ui::Shell>().set_battle_note(Default::default());
    let view = ui.global::<TrainingView>();
    view.set_held_yours(0);
    view.set_held_dummys(0);
}

/// The training battle's set again from its start, if one is being played (not under its menu).
fn restart(shell: &mut Shell) -> bool {
    let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Training && b.playing()) else { return false };
    b.restart()
}

/// A build as a card says it: its name, its version's tag and its navi's name in the language shown.
fn shown(shell: &Shell, game: &str, ready: &Ready, build: &Build) -> TrainingBuild {
    let content = ready.content();
    let graphics = ready.graphics(shell.lang);
    let names = Names::of(content, &graphics);
    TrainingBuild {
        name: build.name.as_str().into(),
        version: crate::screens::menu::version_tag(content, game, &build.side).into(),
        navi: names.navi(build.side.navi(content)).into(),
    }
}

/// Start a training session of the selected game between new builds of its first two versions, the dummy
/// standing still: for development (`NETTAI_TRAINING=<game>`) and the screenshot pass, which have no builds of
/// the player's to go by.
pub fn start_development_battle(shell: &mut Shell, seed: u32) {
    let Some((game, ready)) = shell.ready() else { return };
    let builds = match new_builds(ready.content(), &game) {
        Ok(builds) => builds,
        Err(why) => return crate::errors::log("making a training match", &why),
    };
    let dummy = builds.get(1).unwrap_or(&builds[0]).clone();
    let start = Start { seed, ..Start::new(&game, builds[0].clone(), dummy) };
    // (Why it can't start is in the log.)
    let _ = self::start(shell, start);
}

/// A new build of each of `game`'s versions, named for its version.
pub fn new_builds(content: &std::sync::Arc<nettai_battle::Content>, game: &str) -> Result<Vec<Build>, String> {
    let builds = Builds::of(content, game)?;
    builds
        .versions
        .iter()
        .map(|v| {
            // (A name to tell them by: the version's id, as a title.)
            let mut name = v.id.replace(['-', '_'], " ");
            if let Some(first) = name.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            Ok(Build { name, side: builds.new_build(content, &v.id)? })
        })
        .collect()
}

// ---- The entry ------------------------------------------------------------------------------------------------

/// The most ticks the arena's round is played for its picture: it is taken as the round's opening ends (the
/// field faded in, the two navis on it, nothing yet over them), and no later than this.
const OPENING: u32 = 300;

/// How long the builds chosen stay as they are before the arena is drawn again with them: a step through the
/// builds doesn't make a round each time, and a value has slid into its box before the window is held a moment
/// to make one.
const ARENA_AFTER: Duration = Duration::from_millis(160);

/// A build of the selected game, as the entry lists it.
struct Listed {
    /// What it is remembered by: its file's name.
    file: String,
    name: String,
    /// What its file says; and the side it brings (none: its file names what the game hasn't).
    text: String,
    side: Option<Side>,
}

/// Why a build can't be used: nothing a file's own words say (those are in the log), but what kind of thing is
/// wrong, for the reserved line.
#[derive(Clone, Debug, PartialEq)]
struct Unusable(Wrong);

#[derive(Default)]
pub struct Training {
    /// `NETTAI_TRAINING` was answered.
    started: bool,
    /// The game the entry lists the builds of, and they.
    game: Option<String>,
    builds: Vec<Listed>,
    /// The builds chosen, by their place in the list: the player's, the dummy's.
    chosen: [usize; 2],
    /// Each build checked, by what its file says (a build's file is checked once while it says the same).
    checked: HashMap<(String, String), Option<Unusable>>,
    /// What the session will be picked from: the arena shown is the one it opens on.
    seed: u32,
    /// The arena: the round as it opens, held there. None until it is made, and while a build can't be used.
    arena: Option<Stage>,
    /// The frames the arena's picture has been shown (of those the window drew), to two: the view shows it from
    /// the second (the first may have gone to the room of what was on screen before).
    arena_frames: u8,
    /// The builds chosen changed, and when: the arena is made again once they have stayed a moment.
    changed: Option<Instant>,
}

/// The key the build chosen for `game` is remembered by: the player's, or the dummy's.
fn kept_build(game: &str, dummys: bool) -> String {
    format!("training.{}.{game}", if dummys { "dummy" } else { "build" })
}

impl Training {
    /// List the selected game's builds, and choose the two remembered: with none remembered, the first that can
    /// be used (else the first), for both.
    fn list(&mut self, shell: &mut Shell, game: &str, ready: &Ready) {
        let content = ready.content();
        self.builds = if shell.still {
            // (The screenshot pass's pictures don't depend on what the player has made: a new build of each
            // version, and one that can't be used, a side with nothing chosen.)
            let mut builds: Vec<Listed> = new_builds(content, game).unwrap_or_default().into_iter().map(|b| Listed { file: b.name.clone(), name: b.name, text: String::new(), side: Some(b.side) }).collect();
            if let Ok(side) = Side::fresh(content, game) {
                builds.push(Listed { file: "Unfinished".into(), name: "Unfinished".into(), text: String::new(), side: Some(side) });
            }
            if shell.training.tour_no_builds { Vec::new() } else { builds }
        } else {
            store::list(game)
                .into_iter()
                .map(|b| {
                    let side = store::read(content, game, &b.text).map_err(|why| crate::errors::log(&format!("reading {}", b.path.display()), &why)).ok().map(|(_, side)| side);
                    let file = b.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    Listed { file, name: b.name, text: b.text, side }
                })
                .collect()
        };
        if self.game.as_deref() != Some(game) {
            self.game = Some(game.to_string());
            self.checked.clear();
            self.seed = if shell.still { 7 } else { clock_seed() };
        }
        let kept = |dummys: bool| shell.settings.remembered.get(&kept_build(game, dummys)).and_then(|file| self.builds.iter().position(|b| b.file == *file));
        let (yours, dummys) = (kept(false), kept(true));
        let yours = yours.or_else(|| (0..self.builds.len()).find(|&at| self.unusable_at(ready, at).is_none())).unwrap_or(0);
        self.chosen = [yours, dummys.unwrap_or(yours)];
        // (The arena is made as soon as the screen is on screen: what is in its place until then isn't its own.)
        self.changed = Instant::now().checked_sub(ARENA_AFTER);
        self.drop_arena(shell);
    }

    /// The arena's picture is no longer the one in the window: nothing is shown in its place.
    fn drop_arena(&mut self, shell: &Shell) {
        (self.arena, self.arena_frames) = (None, 0);
        shell.ui().global::<TrainingView>().set_arena(false);
    }

    /// The build chosen: the player's (0) or the dummy's (1).
    fn build(&self, which: usize) -> Option<&Listed> {
        self.builds.get(self.chosen[which])
    }

    /// Why the build chosen can't be used, if it can't: the player's (0) or the dummy's (1).
    fn unusable(&mut self, ready: &Ready, which: usize) -> Option<Unusable> {
        self.unusable_at(ready, self.chosen[which])
    }

    /// Why the build at `at` of the list can't be used, if it can't: checked once for what its file says.
    fn unusable_at(&mut self, ready: &Ready, at: usize) -> Option<Unusable> {
        let game = self.game.clone()?;
        let build = self.builds.get(at)?;
        let Some(side) = &build.side else { return Some(Unusable(Wrong::Foreign)) };
        // (A build of the screenshot pass has no file: it is told by its name.)
        let key = (build.file.clone(), build.text.clone());
        if let Some(known) = self.checked.get(&key) {
            return known.clone();
        }
        let checked = Checked::of(ready.content(), &game, side);
        for problem in &checked.problems {
            crate::errors::log(&format!("checking the build {}", build.name), &problem.said());
        }
        let found = checked.problems.first().map(|p| Unusable(p.what.clone()));
        self.checked.insert(key, found.clone());
        found
    }

    /// Tell the view the entry: the two builds chosen, why one can't be used, whether training can start.
    fn tell(&mut self, shell: &mut Shell, game: &str, ready: &Ready) {
        let ui = shell.ui();
        let view = ui.global::<TrainingView>();
        view.set_none(self.builds.is_empty());
        let card = |build: Option<&Listed>| match build {
            Some(Listed { name, side: Some(side), .. }) => shown(shell, game, ready, &Build { name: name.clone(), side: side.clone() }),
            Some(Listed { name, .. }) => TrainingBuild { name: name.as_str().into(), ..Default::default() },
            None => TrainingBuild::default(),
        };
        view.set_yours(card(self.build(0)));
        view.set_dummys(card(self.build(1)));
        let wrong = [self.unusable(ready, 0), self.unusable(ready, 1)];
        let message = match &wrong {
            [Some(why), _] => view.invoke_unusable(false, rule(why).into()),
            [None, Some(why)] => view.invoke_unusable(true, rule(why).into()),
            [None, None] => Default::default(),
        };
        view.set_message(message);
        view.set_ready(!self.builds.is_empty() && wrong.iter().all(Option::is_none));
        // The focus is never left on a control that went with the builds, or came with them; and where START
        // TRAINING can't be pressed, the screen opens on the row to change.
        let on = shell.nav.focused().map(|k| k.id.clone());
        match (self.builds.is_empty(), on.as_deref(), wrong.iter().position(Option::is_some)) {
            (true, None | Some("start" | "yours" | "dummys"), _) => shell.nav.focus("create", 0),
            (false, Some("create"), None) => shell.nav.focus("start", 0),
            (false, None | Some("start" | "create"), Some(which)) => shell.nav.focus(if which == 1 { "dummys" } else { "yours" }, 0),
            _ => {}
        }
    }

    /// Choose a build, the player's (0) or the dummy's (1): remembered for the game, and shown.
    fn choose_build(&mut self, shell: &mut Shell, which: usize, at: usize) {
        let Some((game, ready)) = shell.ready() else { return };
        if at >= self.builds.len() || self.chosen[which] == at {
            return;
        }
        self.chosen[which] = at;
        shell.settings.remembered.insert(kept_build(&game, which == 1), self.builds[at].file.clone());
        shell.save_settings();
        self.changed = Some(Instant::now());
        self.tell(shell, &game, &ready);
    }

    /// The two builds chosen, if both can be played.
    fn pair(&mut self, ready: &Ready) -> Option<[Build; 2]> {
        if self.unusable(ready, 0).is_some() || self.unusable(ready, 1).is_some() {
            return None;
        }
        let build = |which: usize| self.build(which).and_then(|b| Some(Build { name: b.name.clone(), side: b.side.clone()? }));
        Some([build(0)?, build(1)?])
    }

    /// Make the arena's picture: the session's first round as it opens, the two builds' navis on it.
    fn make_arena(&mut self, shell: &mut Shell) {
        let made = (|| {
            let (game, ready) = shell.ready()?;
            let [you, dummy] = self.pair(&ready)?;
            let content = ready.content();
            let mut m = Match::empty(content, &game).map_err(|why| crate::errors::log("making the arena's match", &why)).ok()?;
            m.sides = [you.side, dummy.side];
            let mut player = crate::screens::menu::player(shell, &ready, Box::new(LivePlayer::new(Set::of(content, &m, self.seed))));
            let opening = player.battle().round.mode;
            for _ in 0..OPENING {
                if player.battle().round.mode != opening || !player.tick(0) {
                    break;
                }
            }
            Some(Stage::new(player))
        })();
        match made {
            // (In place of the one before it, if there was one: the picture changes, and nothing else.)
            Some(arena) => self.arena = Some(arena),
            None => self.drop_arena(shell),
        }
    }

    /// START TRAINING: a session between the two builds chosen, on the arena shown.
    fn start_training(&mut self, shell: &mut Shell) {
        let Some((game, ready)) = shell.ready() else { return };
        let Some([you, dummy]) = self.pair(&ready) else { return shell.sound.play(Cue::Refused) };
        let start = Start { opponent: kept_opponent(shell), seed: self.seed, ..Start::new(&game, you, dummy) };
        self.drop_arena(shell);
        if let Err(error) = self::start(shell, start) {
            shell.fail(error);
        }
    }

    /// The entry again, after a session of its own: another arena, another deal.
    fn back_to_entry(&mut self, shell: &mut Shell) {
        end(shell);
        self.seed = if shell.still { 7 } else { clock_seed() };
        self.changed = Instant::now().checked_sub(ARENA_AFTER);
    }

    /// Step the menu's option at row `index` by `by`, around from its last value to its first: it changes at
    /// once.
    fn step_option(&mut self, shell: &mut Shell, index: i32, by: i32) {
        let Some(mut session) = shell.training.session.take() else { return };
        let option = (index as usize).checked_sub(session.added).and_then(|at| ROWS.get(at)).copied();
        if let Some(option) = option.filter(|o| values(*o) > 0) {
            let next = (value(option, session.opponent, session.inputs) as i32 + by).rem_euclid(values(option) as i32) as usize;
            // (A session another screen started keeps nothing of the dummy's setup.)
            let keep = session.from.is_none();
            match option {
                TrainingOption::Action => {
                    session.opponent.in_fight = InFight::ALL[next];
                    if keep {
                        shell.settings.remembered.insert(KEPT_ACTION.into(), ACTIONS[next].into());
                    }
                }
                TrainingOption::CustomScreen => {
                    session.opponent.on_custom_screen = OnCustomScreen::ALL[next];
                    if keep {
                        shell.settings.remembered.insert(KEPT_CUSTOM_SCREEN.into(), CUSTOM_SCREENS[next].into());
                    }
                }
                TrainingOption::InputDisplay => {
                    shell.settings.remembered.insert(KEPT_INPUTS.into(), if next == 1 { "on" } else { "off" }.into());
                    // (What each side holds is told from the next frame.)
                    session.inputs = next == 1;
                    session.held = [0; 2];
                    let ui = shell.ui();
                    let view = ui.global::<TrainingView>();
                    view.set_inputs(next == 1);
                    view.set_held_yours(0);
                    view.set_held_dummys(0);
                }
                TrainingOption::Added | TrainingOption::Dummy | TrainingOption::Display => {}
            }
            shell.save_settings();
            if let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Training) {
                b.stage.player.set_opponent(session.opponent);
            }
            // (The row's own value: its stepper slides to it.)
            let rows = shell.ui().global::<TrainingView>().get_rows();
            rows.set_row_data(index as usize, TrainingRow { option, value: next as i32, text: Default::default() });
        }
        shell.training.session = Some(session);
    }
}

/// The rule a build breaks, in the player's words, for the reserved line. The Builds screen words each kind of
/// `Wrong` in its own catalog (docs/app.md §12): until that wording is here to call, the line says which build
/// it is, and the rule is in the log.
fn rule(_why: &Unusable) -> String {
    String::new()
}

impl Screen for Training {
    fn main(&self) -> &'static str {
        "start"
    }

    fn show(&mut self, shell: &mut Shell) {
        // (While the game loads its bar is up, and the screen is shown again once it is ready.)
        let Some((game, ready)) = shell.ready() else { return };
        self.list(shell, &game, &ready);
        self.tell(shell, &game, &ready);
    }

    fn hide(&mut self, shell: &mut Shell) {
        self.drop_arena(shell);
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        match id {
            "create" => shell.go(ScreenId::Builds),
            "yours" | "dummys" => {
                let which = (id == "dummys") as usize;
                let options = self.builds.iter().map(|b| b.name.clone()).collect();
                shell.pick(id, 0, options, self.chosen[which]);
            }
            "start" => self.start_training(shell),
            "menu.resume" => shell.close_menu(),
            "menu.restart" => {
                shell.close_menu();
                restart(shell);
            }
            "menu.change" => self.back_to_entry(shell),
            "menu.quit" => {
                end(shell);
                shell.go(ScreenId::Menu);
            }
            "menu.option" => self.step_option(shell, index, 1),
            // A row another screen added: it is told which.
            "menu.added" => {
                if let Some(Session { from: Some((to, tag)), .. }) = &shell.training.session {
                    let (to, tag) = (*to, *tag);
                    shell.tell(to, tag, index.max(0) as usize);
                }
            }
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, id: &str, index: i32, by: i32) {
        match id {
            "yours" | "dummys" if !self.builds.is_empty() => {
                let which = (id == "dummys") as usize;
                let at = (self.chosen[which] as i32 + by).rem_euclid(self.builds.len() as i32) as usize;
                self.choose_build(shell, which, at);
            }
            "menu.option" => self.step_option(shell, index, by),
            _ => {}
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, _index: i32, choice: i32) {
        if matches!(id, "yours" | "dummys") {
            self.choose_build(shell, (id == "dummys") as usize, choice.max(0) as usize);
        }
    }

    /// The menu's row the focus is on is where the menu opens next.
    fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if let Some(session) = shell.training.session.as_mut().filter(|_| id.starts_with("menu.")) {
            session.menu_focus = Key::new(id, index);
        }
    }

    /// R or F5 in a training battle: the set again from its start.
    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        if !matches!(key.code, KeyCode::KeyR | KeyCode::F5) {
            return false;
        }
        restart(shell)
    }

    /// The pad's north button, as R.
    fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        action == Action::Alternate && restart(shell)
    }

    fn button(&mut self, shell: &mut Shell, button: gamepad_facade::Button) -> bool {
        button == gamepad_facade::Button::North && restart(shell)
    }

    /// A dialog asked while a session another screen started was on is that screen's: it hears the answer.
    fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) {
        if let Some(Session { from: Some((to, _)), .. }) = &shell.training.session {
            let to = *to;
            shell.tell(to, tag, choice);
        }
    }

    fn battle(&mut self, shell: &mut Shell, event: BattleEvent) {
        match event {
            BattleEvent::Started | BattleEvent::Watching(_) => {}
            // Training has no end: a set that ends starts over.
            BattleEvent::Finished(_) => {
                if let Some(b) = shell.battle.as_mut() {
                    b.restart();
                }
            }
            BattleEvent::Stopped(reason) => {
                end(shell);
                if let Some(error) = UserError::of_stop(reason) {
                    shell.fail(error);
                }
            }
        }
    }

    fn frame(&mut self, shell: &mut Shell, now: Instant, shown: bool) {
        // `NETTAI_TRAINING=<game>`: straight into a training battle of it, once it is ready. (Asked until it is
        // answered, and not at all without the variable: this runs every frame.)
        if !self.started {
            match std::env::var("NETTAI_TRAINING") {
                Ok(game) if shell.game() != Some(game.as_str()) => shell.select_game(&game),
                Ok(_) if shell.ready().is_some() => {
                    self.started = true;
                    shell.go(ScreenId::Training);
                    start_development_battle(shell, clock_seed());
                }
                Ok(_) => {}
                Err(_) => self.started = true,
            }
        }
        // The session on: gone with its battle, however that went (another battle started in its place); and
        // one shown in another screen's `Picture` lasts while that screen is shown.
        let live = ours(shell) || shell.loading_of(ScreenId::Training);
        let left = shell.training.session.as_ref().is_some_and(|s| !s.full && s.from.is_some_and(|(from, _)| from != shell.screen()));
        if shell.training.session.is_some() && (!live || left) {
            // (A battle of another's is left alone: `end` ends a training session's.)
            end(shell);
        }
        // Its menu opens where the focus last was in it.
        let open = shell.battle.as_ref().is_some_and(|b| b.owner == ScreenId::Training && b.menu_open());
        if let Some(session) = &mut shell.training.session
            && session.menu_open != open
        {
            session.menu_open = open;
            if open {
                let at = session.menu_focus.clone();
                shell.nav.focus(&at.id, at.index);
            }
        }
        // The entry's arena, once the builds chosen have stayed a moment (and the screen is the one on screen:
        // while the one before it leaves, the picture's room is still that one's).
        if !shown || shell.battle.is_some() || shell.loading_battle() || shell.ready().is_none() {
            // (Something else has the window: when the entry shows again, the arena is shown anew.)
            if self.arena_frames != 0 {
                self.arena_frames = 0;
                shell.ui().global::<TrainingView>().set_arena(false);
            }
            return;
        }
        let ui = shell.ui();
        let screen = ui.global::<nettai_ui::Shell>();
        if screen.get_screen() != ScreenId::Training.view() || screen.get_leaving() {
            return;
        }
        // (The screenshot pass doesn't wait; its motion probe does, as a player's app.)
        if self.changed.is_some_and(|at| (shell.still && !shell.probe_motion) || now.saturating_duration_since(at) >= ARENA_AFTER) {
            self.changed = None;
            self.make_arena(shell);
        }
        if let Some(arena) = &mut self.arena {
            shell.show(arena);
            // (A frame the window doesn't draw shows nothing, and places no picture's room.)
            if self.arena_frames < 2 && shell.drawing {
                self.arena_frames += 1;
                if self.arena_frames == 2 {
                    ui.global::<TrainingView>().set_arena(true);
                }
            }
        }
    }

    /// After the battle's ticks: the round, for the line beside MENU on a phone, and with the input display on,
    /// the buttons each side holds.
    fn frame_end(&mut self, shell: &mut Shell, _now: Instant) {
        let Some(b) = shell.battle.as_ref().filter(|b| b.owner == ScreenId::Training) else { return };
        let Some(session) = shell.training.session.as_mut() else { return };
        let battle = b.stage.player.battle();
        // (The round's number is counted as the round opens: before its first tick it is the one before.)
        let round = battle.round.round.max(1);
        let held = if session.inputs { [0, 1].map(|side| battle.custom.sides[side].joypad.held) } else { [0; 2] };
        let (new_round, pressed) = (session.round != Some(round), session.held != held);
        if !new_round && !pressed {
            return;
        }
        (session.round, session.held) = (Some(round), held);
        let ui = shell.ui();
        let view = ui.global::<TrainingView>();
        if new_round {
            ui.global::<nettai_ui::Shell>().set_battle_note(view.invoke_round_note(round as i32));
        }
        if pressed {
            view.set_held_yours(held[0] as i32);
            view.set_held_dummys(held[1] as i32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every row of the menu that steps has its values, and each value of the dummy's choices is kept by a
    /// name of its own.
    #[test]
    fn the_menus_rows_have_their_values() {
        let (opponent, mut seen) = (Opponent::default(), 0);
        for option in ROWS {
            if values(option) > 0 {
                seen += 1;
                assert!(value(option, opponent, false) < values(option), "{option:?}");
            }
        }
        assert_eq!(seen, 3, "Action, Custom screen, Input display");
        assert_eq!((ACTIONS.len(), CUSTOM_SCREENS.len()), (InFight::ALL.len(), OnCustomScreen::ALL.len()));
        assert_eq!(value(TrainingOption::Action, Opponent { in_fight: InFight::AttacksAtRandom, ..opponent }, false), 2);
        assert_eq!(value(TrainingOption::CustomScreen, Opponent { on_custom_screen: OnCustomScreen::PicksNothing, ..opponent }, false), 0);
        assert_eq!(value(TrainingOption::InputDisplay, opponent, true), 1);
    }

    /// The builds chosen are remembered by game, the player's and the dummy's apart.
    #[test]
    fn builds_are_remembered_by_game() {
        assert_eq!(kept_build("exe6", false), "training.build.exe6");
        assert_eq!(kept_build("exe5", true), "training.dummy.exe5");
    }
}
