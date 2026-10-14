//! Practice (docs/design/app-ui.md §6): you against a dummy set up from a menu while practicing. Its UI is
//! `ui/screens/practice.slint`.
//!
//! **The entry** ([`Practice`], the screen): four rows, Your build and Dummy's build (both the player's own
//! builds of the selected game), Arena and Background (the game's, after Random: [`Places`]), all remembered
//! between runs by game, and START PRACTICE; beside them the two builds facing each other on the arena the
//! session opens on. With no build of the game there is one button, Create a build. A build that can't be used
//! says so in the reserved line, and practice doesn't start. Each build's EDIT (or E, or the pad's north button,
//! on its row) opens it in Builds' editor (`crate::screens::builds::open`), and Back from there comes back here,
//! which lists the builds again as they were left.
//!
//! **A practice session** is a battle against a dummy with the practice menu over it, and any screen may start
//! one ([`start`]): Practice does from its entry, and Play does beside a waiting invite, with rows of its own at
//! the top of the menu. It has no end and is not recorded: a set that ends starts over. The session is held by
//! the shell (`Shell::practice`) and its battle is always this screen's (`ScreenId::Practice` owns it), so this
//! module hears its menu and its events whatever screen is shown.
//!
//! **The practice menu** (Start or Esc; the game's own pause never runs) has the battle paused behind it. Its
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
use nettai_frontend::driver::{Ai, ChipDraw, InFight, LivePlayer, OnCustomScreen, Opponent, Training as Options};
use nettai_training::{Fighter, Level};
use nettai_match::build::{Builds, Checked};
use nettai_battle::Content;
use nettai_content_api::StageHandle;
use nettai_match::{Match, RoundSettings, Set, Side, Wrong};
use nettai_ui::{BattleKind, PracticeBuild, PracticeOption, PracticeRow, PracticeView, VersusCard};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

// ---- A practice session, which any screen may start ----------------------------------------------------------

/// A build in a practice session: its name, as the cards and the menu say it, and the side it brings.
#[derive(Clone, Debug)]
pub struct Build {
    pub name: String,
    pub side: Side,
}

/// A practice session to start ([`start`]).
pub struct Start {
    /// The game it is of (ready).
    pub game: String,
    /// The player's build, and the dummy's. Both can be played (`nettai_match::build::Checked::ready`).
    pub you: Build,
    pub dummy: Build,
    /// What the dummy does at first, how well it fights (its place in [`LEVELS`]), and what the battle's
    /// options are; the practice menu changes them.
    pub opponent: Opponent,
    pub level: usize,
    pub options: Options,
    /// Where each round is fought: an arena and a background of the game's ([`Places`]: one it hasn't is
    /// Random), each picked from the seed where it names none (Random).
    pub round: RoundSettings,
    /// What the places left to Random, each folder's order and the dummy's random presses are picked from.
    pub seed: u32,
    /// The screen that starts it, if not Practice itself, and the tag it hears of it by: a row it added was
    /// chosen (`Screen::answered(tag, row)`, the row's place in `rows`). Its session has no CHANGE BUILDS and
    /// no QUIT TO MENU (the screen's own rows are the ways out), and keeps nothing of the dummy's setup.
    pub from: Option<(ScreenId, &'static str)>,
    /// The rows that screen adds at the top of the practice menu: their texts, in the language shown.
    pub rows: Vec<String>,
    /// The picture fills the window. Else the screen shown has a `Picture` of its own for it, and the session
    /// lasts while that screen is shown.
    pub full: bool,
    /// The two cards show while it is made ready.
    pub cards: bool,
}

impl Start {
    /// A session of `game` as Practice starts one: the dummy stands still and picks the first chips it can,
    /// the picture fills the window after the two cards.
    pub fn new(game: &str, you: Build, dummy: Build) -> Start {
        Start {
            game: game.to_string(),
            you,
            dummy,
            opponent: Opponent::default(),
            level: NORMAL,
            options: Options::default(),
            round: RoundSettings::default(),
            seed: clock_seed(),
            from: None,
            rows: Vec::new(),
            full: true,
            cards: true,
        }
    }

    /// A session screen `from` shows beside something of its own while the player waits (Play, by an invite's
    /// code): the dummy a copy of the player's own build, standing still, every option at its normal value and
    /// the places Random; the picture in the screen's own `Picture`, with no cards before it; `rows` at the top
    /// of the practice menu, each heard by `tag`.
    pub fn beside(game: &str, build: Build, from: ScreenId, tag: &'static str, rows: Vec<String>) -> Start {
        Start { from: Some((from, tag)), rows, full: false, cards: false, ..Start::new(game, build.clone(), build) }
    }
}

/// What the shell holds of practice (`Shell::practice`).
#[derive(Default)]
pub struct Shared {
    /// The session on, if one is.
    session: Option<Session>,
    /// The screenshot pass: the entry is shown as with no build of the game; or it lists the builds folder's
    /// files, as a player's does (the pass's own folder: what it made there to open in Builds).
    pub tour_no_builds: bool,
    pub tour_files: bool,
}

/// The practice session on.
struct Session {
    /// The screen that started it, if not Practice, and the tag it hears of it by.
    from: Option<(ScreenId, &'static str)>,
    /// Its game, the match it plays (the two builds, every round on the place chosen) and the seed its set is
    /// picked from: an Arena or Background row changed plays the match's set again on the new place.
    game: String,
    m: Match,
    seed: u32,
    /// The game's places, what each is called in the language shown (Random not among them), and the two
    /// chosen (each its list's place, 0 Random).
    places: Places,
    place_labels: [Vec<String>; 2],
    place: [usize; 2],
    /// Its picture fills the window.
    full: bool,
    /// The rows its starter added: the menu's first; and the menu's own shown under them ([`shown_rows`]).
    added: usize,
    rows: Vec<PracticeOption>,
    /// What the dummy does, how well it fights (its place in [`LEVELS`]), the battle's options, and whether the
    /// input display is on.
    opponent: Opponent,
    level: usize,
    options: Options,
    inputs: bool,
    /// The level of the AI plugged into the dummy's live play, while one is ([`plug`]).
    ai: Option<usize>,
    /// The menu is open, and the row the focus was last on in it (it opens there).
    menu_open: bool,
    menu_focus: Key,
    /// What the UI was last told: the round, and the buttons each side holds.
    round: Option<u8>,
    held: [u16; 2],
}

/// The practice menu's rows, in order, under whatever rows another screen added: each group's title, then its
/// options. A row is added here, worded in practice.slint (`PracticeView.label` and `value`), and given what it
/// does in [`values`], [`value`] and `Practice::step_option`. The dummy's Level is shown under its Action while
/// that is Fights alone ([`shown_rows`]).
pub(crate) const ROWS: [PracticeOption; 12] = [
    PracticeOption::Dummy,
    PracticeOption::Action,
    PracticeOption::Level,
    PracticeOption::Battle,
    PracticeOption::Arena,
    PracticeOption::Background,
    PracticeOption::YourHp,
    PracticeOption::DummysHp,
    PracticeOption::CustomGauge,
    PracticeOption::ChipDraw,
    PracticeOption::Display,
    PracticeOption::InputDisplay,
];

/// How many values a row steps through (none: a group's title; Arena and Background the game's `places` and
/// Random).
fn values(option: PracticeOption, places: &Places) -> usize {
    match option {
        PracticeOption::Arena => places.len(ARENA),
        PracticeOption::Background => places.len(BACKGROUND),
        PracticeOption::Action => ACTIONS.len(),
        PracticeOption::Level => LEVELS.len(),
        PracticeOption::YourHp | PracticeOption::DummysHp | PracticeOption::CustomGauge | PracticeOption::ChipDraw | PracticeOption::InputDisplay => 2,
        PracticeOption::Added | PracticeOption::Dummy | PracticeOption::Battle | PracticeOption::Display => 0,
    }
}

/// The value a row has: its place among its values.
fn value(option: PracticeOption, opponent: Opponent, level: usize, options: Options, inputs: bool, place: [usize; 2]) -> usize {
    match option {
        PracticeOption::Level => level,
        PracticeOption::Arena => place[ARENA],
        PracticeOption::Background => place[BACKGROUND],
        PracticeOption::Action => ACTIONS.iter().position(|(_, a)| *a == opponent.in_fight).unwrap_or(0),
        PracticeOption::YourHp => options.refills[0] as usize,
        PracticeOption::DummysHp => options.refills[1] as usize,
        PracticeOption::CustomGauge => options.full_gauge as usize,
        PracticeOption::ChipDraw => (options.chip_draw == ChipDraw::FolderOrder) as usize,
        PracticeOption::InputDisplay => inputs as usize,
        PracticeOption::Added | PracticeOption::Dummy | PracticeOption::Battle | PracticeOption::Display => 0,
    }
}

/// A row as the menu shows it: its value's place; for Arena and Background, the name of the one chosen (none:
/// Random).
fn row(option: PracticeOption, session: &Session) -> PracticeRow {
    let value = value(option, session.opponent, session.level, session.options, session.inputs, session.place) as i32;
    let part = match option {
        PracticeOption::Arena => Some(ARENA),
        PracticeOption::Background => Some(BACKGROUND),
        _ => None,
    };
    let text = part.and_then(|part| session.place[part].checked_sub(1).and_then(|at| session.place_labels[part].get(at))).map(|name| name.as_str().into()).unwrap_or_default();
    PracticeRow { option, value, text }
}

/// The menu's rows shown with the dummy's Action `in_fight`: [`ROWS`], but for the Level (with Fights alone).
fn shown_rows(in_fight: InFight) -> Vec<PracticeOption> {
    ROWS.into_iter().filter(|o| *o != PracticeOption::Level || in_fight == InFight::Ai).collect()
}

/// Show `options`' rows as `session` has them now (those shown).
fn tell_rows(shell: &Shell, session: &Session, options: &[PracticeOption]) {
    let rows = shell.ui().global::<PracticeView>().get_rows();
    for option in options {
        if let Some(at) = session.rows.iter().position(|o| o == option) {
            rows.set_row_data(session.added + at, row(*option, session));
        }
    }
}

/// The rows shown as the dummy's Action now has them: a row its Action no longer has taken out, one it has
/// now put in under it (the rows above them, the Action's among them, staying where they are, and the focus
/// with them).
fn lay_out_rows(shell: &Shell, session: &mut Session) {
    let wanted = shown_rows(session.opponent.in_fight);
    if wanted == session.rows {
        return;
    }
    let model = shell.ui().global::<PracticeView>().get_rows();
    let Some(rows) = model.as_any().downcast_ref::<VecModel<PracticeRow>>() else { return };
    // (Gone from the last, then new from the first: each at its place in the list it is in.)
    for at in (0..session.rows.len()).rev() {
        if !wanted.contains(&session.rows[at]) {
            rows.remove(session.added + at);
        }
    }
    session.rows.retain(|o| wanted.contains(o));
    for (at, option) in wanted.iter().enumerate() {
        if session.rows.get(at) != Some(option) {
            session.rows.insert(at, *option);
            rows.insert(session.added + at, row(*option, session));
        }
    }
}

/// The dummy that does `in_fight`: its custom screen goes with it, the AI's for one that fights, the first chip
/// it can for any other (a second pad's own on its screen).
fn dummy(in_fight: InFight) -> Opponent {
    Opponent { in_fight, on_custom_screen: if in_fight == InFight::Ai { OnCustomScreen::Ai } else { OnCustomScreen::PicksFirst } }
}

/// Where the menu shows `option` now, among its rows (another screen's added first), if it shows it: for the
/// screenshot pass.
#[cfg(any(test, feature = "tour"))]
pub(crate) fn row_of(shell: &Shell, option: PracticeOption) -> Option<i32> {
    let session = shell.practice.session.as_ref()?;
    session.rows.iter().position(|o| *o == option).map(|at| (session.added + at) as i32)
}

/// What is remembered between runs, by these keys of the settings' `remembered`: the dummy's setup, the battle's
/// options and the input display (the builds chosen are by game: [`kept_build`]).
const KEPT_ACTION: &str = "training.action";
const KEPT_LEVEL: &str = "training.level";
const KEPT_HP: [&str; 2] = ["training.your-hp", "training.dummys-hp"];
const KEPT_GAUGE: &str = "training.custom-gauge";
const KEPT_DRAW: &str = "training.chip-draw";
const KEPT_INPUTS: &str = "training.inputs";

/// The dummy's Actions, in the row's order, by their names in the settings.
const ACTIONS: [(&str, InFight); 4] = [
    ("stands-still", InFight::StandsStill),
    ("moves-around", InFight::MovesAround),
    ("fights", InFight::Ai),
    ("second-pad", InFight::SecondPad),
];

/// How well the dummy fights (its Action Fights: the AI's), by its name in the settings, in the Level row's
/// order; Normal at first.
const LEVELS: [(&str, Level); 3] = [("easy", Level::EASY), ("normal", Level::NORMAL), ("hard", Level::HARD)];
const NORMAL: usize = 1;

/// The dummy's level as it was last left in one of Practice's own sessions.
fn kept_level(shell: &Shell) -> usize {
    shell.settings.remembered.get(KEPT_LEVEL).and_then(|v| LEVELS.iter().position(|(name, _)| name == v)).unwrap_or(NORMAL)
}

/// The dummy's AI as its Action needs it: a fighter at `level` while it fights (a new one for another level:
/// nettai-training's `Fighter`, its thinking on threads of its own), none otherwise. `plugged`: the level of the
/// one plugged in now, if any. What is plugged in after.
pub(crate) fn plug(live: &mut LivePlayer, opponent: Opponent, level: usize, seed: u32, plugged: Option<usize>) -> Option<usize> {
    let wanted = (opponent.in_fight == InFight::Ai || opponent.on_custom_screen == OnCustomScreen::Ai).then_some(level);
    if wanted != plugged {
        live.set_ai(wanted.map(|level| Box::new(Fighter::new(LEVELS[level].1, seed as u64)) as Box<dyn Ai>));
    }
    wanted
}

/// The dummy's setup as it was last left in one of Practice's own sessions.
fn kept_opponent(shell: &Shell) -> Opponent {
    let kept = shell.settings.remembered.get(KEPT_ACTION).and_then(|v| ACTIONS.iter().find(|(name, _)| name == v));
    dummy(kept.map_or(InFight::StandsStill, |(_, in_fight)| *in_fight))
}

/// The battle's options as they were last left in one of Practice's own sessions.
fn kept_options(shell: &Shell) -> Options {
    let kept = |key: &str, on: &str| shell.settings.remembered.get(key).is_some_and(|v| v == on);
    let chip_draw = if kept(KEPT_DRAW, "folder-order") { ChipDraw::FolderOrder } else { ChipDraw::Shuffled };
    Options { refills: KEPT_HP.map(|key| kept(key, "refills")), full_gauge: kept(KEPT_GAUGE, "full"), chip_draw }
}

/// Whether the input display is on.
fn inputs_on(settings: &crate::settings::Settings) -> bool {
    settings.remembered.get(KEPT_INPUTS).is_some_and(|v| v == "on")
}

/// Start a practice session: the two cards while it is made ready (if it has them), then the battle, with the
/// practice menu behind Start and Esc. A session already on ends first. Err: why it can't start, as the player
/// reads it (what the code said is in the log).
pub fn start(shell: &mut Shell, start: Start) -> Result<(), UserError> {
    let Some(ready) = shell.games.ready(&start.game) else {
        crate::errors::log("starting practice", &format_args!("{} isn't ready", start.game));
        return Err(UserError::Other);
    };
    let content = ready.content();
    let mut m = Match::empty(content, &start.game).map_err(|why| {
        crate::errors::log("making a practice match", &why);
        UserError::Other
    })?;
    m.sides = [start.you.side.clone(), start.dummy.side.clone()];
    let places = Places::of(content, &start.game);
    let place = places.chosen(content, &start.round);
    places.put(&mut m, place);
    // (A set is made of a checked match: both folders whole.)
    if let Some(problem) = nettai_match::check::problems(content, &m).first() {
        crate::errors::log("starting practice", &problem.said());
        return Err(UserError::BuildUnusable { reason: None });
    }
    end(shell);
    let ui = shell.ui();
    let view = ui.global::<PracticeView>();
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
    let place_labels = places.labels(&Names::of(content, &ready.graphics(shell.lang)));
    let mut session = Session {
        from: start.from,
        game: start.game.clone(),
        m,
        seed: start.seed,
        places,
        place_labels,
        place,
        full: start.full,
        added: start.rows.len(),
        rows: shown_rows(start.opponent.in_fight),
        opponent: start.opponent,
        level: start.level,
        options: start.options,
        inputs,
        ai: None,
        menu_open: false,
        // (The menu opens on its first row: a row its starter added, else the first option.)
        menu_focus: if start.rows.is_empty() { Key::new("menu.rows.option", shown_rows(start.opponent.in_fight).iter().position(|o| values(*o, &Places::default()) > 0).unwrap_or(0) as i32) } else { Key::new("menu.rows.added", 0) },
        round: None,
        held: [0; 2],
    };
    let rows: Vec<PracticeRow> = start
        .rows
        .iter()
        .map(|text| PracticeRow { option: PracticeOption::Added, value: 0, text: text.into() })
        .chain(session.rows.iter().map(|o| row(*o, &session)))
        .collect();
    view.set_rows(ModelRc::new(VecModel::from(rows)));
    view.set_leave(start.from.is_none());
    view.set_session_yours(start.you.name.as_str().into());
    view.set_session_dummys(start.dummy.name.as_str().into());
    view.set_inputs(inputs);
    view.set_held_yours(0);
    view.set_held_dummys(0);
    let mut driver = LivePlayer::against(set_of(content, &session.m, start.seed), start.opponent, start.seed as u64).with_training(start.options);
    session.ai = plug(&mut driver, session.opponent, session.level, session.seed, None);
    shell.start_battle(BattleStart {
        owner: ScreenId::Practice,
        kind: BattleKind::Practice,
        game: start.game.clone(),
        set: set_of(content, &session.m, start.seed),
        play: Play::Drive(Box::new(driver)),
        viewer: 0,
        recorder: None,
        cards,
        full: start.full,
        gate: None,
    });
    shell.practice.session = Some(session);
    Ok(())
}

/// The set a practice session plays: the match's, its rounds leaving the game its START (`HostKeys::pause`
/// off), so the practice menu holds the battle in the game's own pause (`Battle::pause_in_game`). The rest of
/// the round is as any match's. (A match's every round has the host answer START, for both players of an
/// online one and its replay; practice is neither online nor recorded.)
fn set_of(content: &std::sync::Arc<nettai_battle::Content>, m: &Match, seed: u32) -> Set {
    let mut first = m.round(content, seed);
    first.host_keys.pause = false;
    let folders = [0, 1].map(|side| m.sides[side].folder(content).saved().expect("a checked match's folder is whole"));
    Set::new(content.clone(), first, folders)
}

/// End the practice session on, if one is: its battle (or the cards before it) goes, and the screen shown is
/// what shows. What starts a session from outside calls this when its wait is over (an opponent joined).
pub fn end(shell: &mut Shell) {
    if shell.practice.session.take().is_none() {
        return;
    }
    if shell.loading_of(ScreenId::Practice) {
        shell.cancel_loading();
    }
    if ours(shell) {
        shell.end_battle();
    }
    forget(shell);
}

/// Whether a practice session is on (being made ready, or running).
#[cfg_attr(not(feature = "tour"), allow(dead_code))] // (The screenshot pass asks.)
pub fn running(shell: &Shell) -> bool {
    shell.practice.session.is_some()
}

/// Whether the battle on is a practice session's.
fn ours(shell: &Shell) -> bool {
    shell.battle.as_ref().is_some_and(|b| b.owner == ScreenId::Practice && b.kind == BattleKind::Practice)
}

/// The UI no longer says anything of a session.
fn forget(shell: &Shell) {
    let ui = shell.ui();
    ui.global::<nettai_ui::Shell>().set_battle_note(Default::default());
    let view = ui.global::<PracticeView>();
    view.set_held_yours(0);
    view.set_held_dummys(0);
}

/// The practice battle's set again from its start, if one is being played (not under its menu).
fn restart(shell: &mut Shell) -> bool {
    let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Practice && b.playing()) else { return false };
    b.restart()
}

/// The session's set again from its start on the arena it has now (the Arena row changed: a round's field is
/// set as it starts), as RESTART starts it over, the menu staying open: the battle behind it steps through the
/// round's opening on the new arena and waits in the game's own pause, or on its custom screen, as ever under
/// the menu.
fn restart_on_place(shell: &mut Shell, session: &mut Session) {
    let Some(ready) = shell.games.ready(&session.game) else { return };
    session.places.put(&mut session.m, session.place);
    let set = set_of(ready.content(), &session.m, session.seed);
    if let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Practice)
        && let Some(live) = b.stage.player.live_mut()
    {
        live.play_set(set);
        b.restart();
    }
}

/// The session's set on the background it has now (the Background row changed; the user: "changing the
/// background shouldn't change the entire arena either in trianing mode"): the battle goes on where it is, the
/// arena, the HP, the dummy and the round as they were, drawn on the new background from now
/// (`Player::set_backgrounds`: no tick reads a round's background). Its set's arenas are the same: the seed
/// picks a round's arena whether the round names its background or not.
fn redraw_on_background(shell: &mut Shell, session: &mut Session) {
    let Some(ready) = shell.games.ready(&session.game) else { return };
    session.places.put(&mut session.m, session.place);
    let set = set_of(ready.content(), &session.m, session.seed);
    if let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Practice) {
        b.stage.player.set_backgrounds(set);
    }
}

/// A build as a card says it: its name, its version's tag and its navi's name in the language shown.
fn shown(shell: &Shell, game: &str, ready: &Rc<Ready>, build: &Build) -> PracticeBuild {
    let content = ready.content();
    let graphics = ready.graphics(shell.lang);
    let names = Names::of(content, &graphics);
    PracticeBuild {
        name: build.name.as_str().into(),
        version: crate::builds::version_tag(shell, game, ready, &build.side),
        navi: names.navi(build.side.navi(content)).into(),
        editable: true,
    }
}

/// Start a practice session of the selected game between new builds of its first two versions, the dummy
/// standing still, the battle's options `options`: for development (`NETTAI_PRACTICE=<game>`, with the options
/// as Practice last left them) and the screenshot pass (with none), which have no builds of the player's to go
/// by.
pub fn start_development_battle(shell: &mut Shell, seed: u32, options: Options) {
    let Some((game, ready)) = shell.ready() else { return };
    let builds = match new_builds(ready.content(), &game) {
        Ok(builds) => builds,
        Err(why) => return crate::errors::log("making a practice match", &why),
    };
    let dummy = builds.get(1).unwrap_or(&builds[0]).clone();
    let start = Start { seed, options, level: kept_level(shell), ..Start::new(&game, builds[0].clone(), dummy) };
    // (Why it can't start is in the log.)
    let _ = self::start(shell, start);
}

/// A new build of each of `game`'s versions, named for its version.
pub fn new_builds(content: &std::sync::Arc<nettai_battle::Content>, game: &str) -> Result<Vec<Build>, String> {
    let builds = Builds::of(content, game)?;
    builds
        .versions
        .iter()
        // (A name to tell them by: the version's id, as a title.)
        .map(|v| Ok(Build { name: as_title(&v.id), side: builds.new_build(content, &v.id)? }))
        .collect()
}

/// A key as a title: its words apart, the first letter a capital (`falzar`, "Falzar").
fn as_title(key: &str) -> String {
    let mut name = key.replace(['-', '_'], " ");
    if let Some(first) = name.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    name
}

// ---- The places -----------------------------------------------------------------------------------------------

/// The two lists of [`Places`], by their places in a pair: the arenas and the backgrounds.
const ARENA: usize = 0;
const BACKGROUND: usize = 1;

/// The places a practice set may be fought on, as the entry and the menu step through them: the game's arenas
/// (the link battle stages a match may name, `nettai_match::link_battle_stages`: the panel layouts and what
/// stands on them that its netbattles are fought on) and its battle backgrounds, each in its keys' natural order
/// (a run of digits by its number: `netbattle-2` before `netbattle-10`; a background by its name in the pack).
/// Each list is stepped through after Random: the place picked from the seed each round, as a match that names
/// none has it (what practice did before it had them). A choice is a list's place, 0 Random.
///
/// The games keep the two apart: a link battle picks its stage and its background each on its own (the rules'
/// `link_pick`), so either goes with any of the other. An arena is said by its place in the list ("12 / 96":
/// the user, "the arena names are meaningless"); a background by what the language's strings call it (their
/// `[backgrounds]`, by the area it comes from), or its place where they don't name it.
#[derive(Clone, Default)]
pub struct Places {
    arenas: Vec<StageHandle>,
    /// By their names in the game's pack.
    backgrounds: Vec<String>,
}

impl Places {
    pub fn of(content: &Content, game: &str) -> Places {
        let mut arenas = nettai_match::link_battle_stages(content, game);
        arenas.sort_by_cached_key(|&s| natural(nettai_match::ids::local(&content.defs.stage(s).key)));
        let mut backgrounds: Vec<String> = nettai_match::ids::backgrounds(content, game).into_iter().map(str::to_string).collect();
        backgrounds.sort_by_cached_key(|b| natural(b));
        Places { arenas, backgrounds }
    }

    /// How many choices list `part` has, Random with them.
    fn len(&self, part: usize) -> usize {
        1 + if part == ARENA { self.arenas.len() } else { self.backgrounds.len() }
    }

    /// A round on the place `chosen`.
    fn round(&self, chosen: [usize; 2]) -> RoundSettings {
        RoundSettings {
            stage: chosen[ARENA].checked_sub(1).and_then(|at| self.arenas.get(at)).copied(),
            background: chosen[BACKGROUND].checked_sub(1).and_then(|at| self.backgrounds.get(at)).cloned(),
        }
    }

    /// Every round of `m` on the place `chosen`.
    fn put(&self, m: &mut Match, chosen: [usize; 2]) {
        let round = self.round(chosen);
        m.rounds.iter_mut().for_each(|r| *r = round.clone());
    }

    /// The place `round` states, by the lists' places (what it names that they haven't: Random).
    fn chosen(&self, content: &Content, round: &RoundSettings) -> [usize; 2] {
        let arena = round.stage.map(|s| nettai_match::ids::local(&content.defs.stage(s).key));
        [self.find(content, ARENA, arena), self.find(content, BACKGROUND, round.background.as_deref())]
    }

    /// Where the choice remembered by `key` is in list `part` (none, or one the list hasn't: Random).
    fn find(&self, content: &Content, part: usize, key: Option<&str>) -> usize {
        key.and_then(|key| (1..self.len(part)).find(|&at| self.key(content, part, at) == Some(key))).unwrap_or(0)
    }

    /// The key choice `at` of list `part` is remembered by: a stage's, a background's name in the pack (none:
    /// Random).
    fn key<'c>(&'c self, content: &'c Content, part: usize, at: usize) -> Option<&'c str> {
        let at = at.checked_sub(1)?;
        if part == ARENA { self.arenas.get(at).map(|&s| nettai_match::ids::local(&content.defs.stage(s).key)) } else { self.backgrounds.get(at).map(String::as_str) }
    }

    /// What each choice of the two lists but Random is shown as, in `names`' language: its place in the list
    /// ("12 / 96"), a background its name where it has one.
    fn labels(&self, names: &Names) -> [Vec<String>; 2] {
        let place = |at: usize, of: usize| format!("{} / {of}", at + 1);
        [
            (0..self.arenas.len()).map(|at| place(at, self.arenas.len())).collect(),
            self.backgrounds.iter().enumerate().map(|(at, b)| names.background_named(b).unwrap_or_else(|| place(at, self.backgrounds.len()))).collect(),
        ]
    }

    /// The place remembered for `game` (Random where nothing is, or the content no longer has it).
    fn kept(&self, content: &Content, remembered: &std::collections::BTreeMap<String, String>, game: &str) -> [usize; 2] {
        [ARENA, BACKGROUND].map(|part| self.find(content, part, remembered.get(&kept_place(game, part)).map(String::as_str)))
    }

    /// Remember choice `at` of list `part` for `game` (Random: nothing).
    fn keep(&self, content: &Content, remembered: &mut std::collections::BTreeMap<String, String>, game: &str, part: usize, at: usize) {
        match self.key(content, part, at) {
            Some(key) => remembered.insert(kept_place(game, part), key.to_string()),
            None => remembered.remove(&kept_place(game, part)),
        };
    }
}

/// What `key` sorts by in its natural order: each run of digits as wide as any number's, so by its number
/// (`netbattle-2` before `netbattle-10`).
fn natural(key: &str) -> String {
    let (mut out, mut digits) = (String::with_capacity(key.len() + 16), String::new());
    for c in key.chars().map(Some).chain([None]) {
        if let Some(c) = c.filter(char::is_ascii_digit) {
            digits.push(c);
            continue;
        }
        if !digits.is_empty() {
            out += &format!("{digits:0>20}");
            digits.clear();
        }
        out.extend(c);
    }
    out
}

/// The key the place chosen for `game` is remembered by: its arena's, or its background's.
fn kept_place(game: &str, part: usize) -> String {
    format!("training.{}.{game}", if part == ARENA { "arena" } else { "background" })
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

/// Why a build can't be used: its first problem, typed (its own words are in the log), for the reserved line.
#[derive(Clone, Debug, PartialEq)]
struct Unusable(nettai_match::check::Problem);

#[derive(Default)]
pub struct Practice {
    /// `NETTAI_PRACTICE` was answered.
    started: bool,
    /// The game the entry lists the builds of, and they.
    game: Option<String>,
    builds: Vec<Listed>,
    /// The builds chosen, by their place in the list: the player's, the dummy's.
    chosen: [usize; 2],
    /// The game's places, what each is called in the language shown (Random not among them), and the place
    /// chosen (each its list's place, 0 Random).
    places: Places,
    place_labels: [Vec<String>; 2],
    place: [usize; 2],
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

impl Practice {
    /// List the selected game's builds and places, and choose those remembered: with no build remembered, the
    /// first that can be used (else the first), for both; with no place, Random.
    fn list(&mut self, shell: &mut Shell, game: &str, ready: &Ready) {
        let content = ready.content();
        self.places = Places::of(content, game);
        self.place_labels = self.places.labels(&Names::of(content, &ready.graphics(shell.lang)));
        self.place = self.places.kept(content, &shell.settings.remembered, game);
        self.builds = if shell.still && !shell.practice.tour_files {
            // (The screenshot pass's pictures don't depend on what the player has made: a new build of each
            // version, and one that can't be used, a side with nothing chosen.)
            let mut builds: Vec<Listed> = new_builds(content, game).unwrap_or_default().into_iter().map(|b| Listed { file: b.name.clone(), name: b.name, text: String::new(), side: Some(b.side) }).collect();
            if let Ok(side) = Side::fresh(content, game) {
                builds.push(Listed { file: "Unfinished".into(), name: "Unfinished".into(), text: String::new(), side: Some(side) });
            }
            if shell.practice.tour_no_builds { Vec::new() } else { builds }
        } else {
            store::list(game)
                .into_iter()
                .map(|b| {
                    let side = store::read(content, game, &b.text).map_err(|why| crate::errors::log(&format!("reading {}", b.path.display()), &why)).ok().map(|build| build.side);
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
        shell.ui().global::<PracticeView>().set_arena(false);
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
        let Some(side) = &build.side else {
            // (Its file names what the game hasn't: what, the log has.)
            return Some(Unusable(nettai_match::check::Problem { side: None, field: None, entry: None, what: Wrong::Foreign, text: String::new() }));
        };
        // (A build of the screenshot pass has no file: it is told by its name.)
        let key = (build.file.clone(), build.text.clone());
        if let Some(known) = self.checked.get(&key) {
            return known.clone();
        }
        let checked = Checked::of(ready.content(), &game, side);
        for problem in &checked.problems {
            crate::errors::log(&format!("checking the build {}", build.name), &problem.said());
        }
        let found = checked.problems.first().map(|p| Unusable(p.clone()));
        self.checked.insert(key, found.clone());
        found
    }

    /// Tell the view the entry: the two builds chosen and the place, why a build can't be used, whether practice
    /// can start.
    fn tell(&mut self, shell: &mut Shell, game: &str, ready: &Rc<Ready>) {
        let ui = shell.ui();
        let view = ui.global::<PracticeView>();
        view.set_none(self.builds.is_empty());
        let name = |part: usize| self.place[part].checked_sub(1).and_then(|at| self.place_labels[part].get(at)).map(|n| n.as_str().into()).unwrap_or_default();
        view.set_arena_shown(name(ARENA));
        view.set_background_shown(name(BACKGROUND));
        view.set_music_shown(crate::music::shown(shell).into());
        let card = |build: Option<&Listed>| match build {
            Some(Listed { name, side: Some(side), .. }) => shown(shell, game, ready, &Build { name: name.clone(), side: side.clone() }),
            Some(Listed { name, .. }) => PracticeBuild { name: name.as_str().into(), ..Default::default() },
            None => PracticeBuild::default(),
        };
        view.set_yours(card(self.build(0)));
        view.set_dummys(card(self.build(1)));
        let wrong = [self.unusable(ready, 0), self.unusable(ready, 1)];
        let message = match &wrong {
            [Some(why), _] => view.invoke_unusable(false, rule(shell, game, ready, why).into()),
            [None, Some(why)] => view.invoke_unusable(true, rule(shell, game, ready, why).into()),
            [None, None] => Default::default(),
        };
        view.set_message(message);
        view.set_ready(!self.builds.is_empty() && wrong.iter().all(Option::is_none));
        // The focus is never left on a control that went with the builds, or came with them; and where START
        // PRACTICE can't be pressed, the screen opens on the row to change.
        let on = shell.nav.focused().map(|k| k.id.clone());
        match (self.builds.is_empty(), on.as_deref(), wrong.iter().position(Option::is_some)) {
            (true, None | Some("start" | "yours" | "dummys" | "edit" | "arena" | "background" | "music"), _) => shell.nav.focus("create", 0),
            (false, Some("create"), None) => shell.nav.focus("start", 0),
            (false, None | Some("start" | "create"), Some(which)) => shell.nav.focus(if which == 1 { "dummys" } else { "yours" }, 0),
            _ => {}
        }
    }

    /// The battle's music the player picked, as its row shows it (one pick with Play's lobby's: `crate::music`).
    /// Nothing else changes: the arena's picture is silent, and the next battle plays it.
    fn tell_music(&self, shell: &Shell) {
        shell.ui().global::<PracticeView>().set_music_shown(crate::music::shown(shell).into());
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

    /// Open the build chosen, the player's (0) or the dummy's (1), in Builds' editor; Back from it comes back
    /// here. It is found among the game's files as they are now, by its file (Builds lists them in that order);
    /// one that isn't there, or doesn't read as a build, is refused. The two builds chosen are remembered for the
    /// game first, so that the entry shows them again as they were left, whatever the edit did to them (one
    /// that can't be used now stays chosen, the reserved line saying why).
    fn edit_build(&mut self, shell: &mut Shell, which: usize) {
        let Some(game) = self.game.clone() else { return };
        let file = self.build(which).filter(|b| b.side.is_some()).map(|b| b.file.clone());
        let Some(at) = file.and_then(|file| place_of(&store::list(&game), &file)) else { return shell.sound.play(Cue::Refused) };
        for side in [0, 1] {
            if let Some(b) = self.build(side) {
                shell.settings.remembered.insert(kept_build(&game, side == 1), b.file.clone());
            }
        }
        shell.save_settings();
        crate::screens::builds::open(shell, ScreenId::Practice, Some(at));
    }

    /// Step the place of list `part` (the arenas or the backgrounds, 0 Random) by `by`, around from its last to
    /// Random: remembered for the game, and shown (the arena drawn again on it a moment after, as for a build;
    /// what the seed picks, a Random arena, the same whatever the background).
    fn step_place(&mut self, shell: &mut Shell, part: usize, by: i32) {
        let Some((game, ready)) = shell.ready() else { return };
        let at = (self.place[part] as i32 + by).rem_euclid(self.places.len(part) as i32) as usize;
        if self.builds.is_empty() || self.place[part] == at {
            return;
        }
        self.place[part] = at;
        self.places.keep(ready.content(), &mut shell.settings.remembered, &game, part, at);
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
            self.places.put(&mut m, self.place);
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

    /// START PRACTICE: a session between the two builds chosen, on the arena shown.
    fn start_practice(&mut self, shell: &mut Shell) {
        let Some((game, ready)) = shell.ready() else { return };
        let Some([you, dummy]) = self.pair(&ready) else { return shell.sound.play(Cue::Refused) };
        let round = self.places.round(self.place);
        let start = Start { opponent: kept_opponent(shell), level: kept_level(shell), options: kept_options(shell), round, seed: self.seed, ..Start::new(&game, you, dummy) };
        self.drop_arena(shell);
        if let Err(error) = self::start(shell, start) {
            shell.fail(error);
        }
    }

    /// E or the pad's north button on the entry: a build opened in Builds' editor (`build_meant`). False with no
    /// build, or a session on.
    fn edit_focused(&mut self, shell: &mut Shell) -> bool {
        if shell.battle.is_some() || shell.screen() != ScreenId::Practice || self.builds.is_empty() {
            return false;
        }
        shell.sound.play(Cue::Pick);
        self.edit_build(shell, build_meant(shell));
        true
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
        let Some(mut session) = shell.practice.session.take() else { return };
        let option = (index as usize).checked_sub(session.added).and_then(|at| session.rows.get(at)).copied();
        if let Some(option) = option.filter(|o| values(*o, &session.places) > 0) {
            let next = (value(option, session.opponent, session.level, session.options, session.inputs, session.place) as i32 + by).rem_euclid(values(option, &session.places) as i32) as usize;
            // (A session another screen started keeps nothing of the dummy's setup, or of the place.)
            let keep = session.from.is_none();
            match option {
                // Arena: the set again from its start on the new arena, as RESTART starts it, the menu staying open.
                // Background: the battle drawn on it from now, nothing else changing.
                PracticeOption::Arena | PracticeOption::Background => {
                    let part = if option == PracticeOption::Arena { ARENA } else { BACKGROUND };
                    session.place[part] = next;
                    if keep && let Some(ready) = shell.games.ready(&session.game) {
                        session.places.keep(ready.content(), &mut shell.settings.remembered, &session.game, part, next);
                    }
                    if part == ARENA {
                        restart_on_place(shell, &mut session);
                    } else {
                        redraw_on_background(shell, &mut session);
                    }
                }
                PracticeOption::Action => {
                    let (name, in_fight) = ACTIONS[next];
                    session.opponent = dummy(in_fight);
                    if keep {
                        shell.settings.remembered.insert(KEPT_ACTION.into(), name.into());
                    }
                }
                PracticeOption::Level => {
                    session.level = next;
                    if keep {
                        shell.settings.remembered.insert(KEPT_LEVEL.into(), LEVELS[next].0.into());
                    }
                }
                PracticeOption::YourHp | PracticeOption::DummysHp => {
                    let side = (option == PracticeOption::DummysHp) as usize;
                    session.options.refills[side] = next == 1;
                    if keep {
                        shell.settings.remembered.insert(KEPT_HP[side].into(), if next == 1 { "refills" } else { "normal" }.into());
                    }
                }
                PracticeOption::CustomGauge => {
                    session.options.full_gauge = next == 1;
                    if keep {
                        shell.settings.remembered.insert(KEPT_GAUGE.into(), if next == 1 { "full" } else { "normal" }.into());
                    }
                }
                PracticeOption::ChipDraw => {
                    session.options.chip_draw = if next == 1 { ChipDraw::FolderOrder } else { ChipDraw::Shuffled };
                    if keep {
                        shell.settings.remembered.insert(KEPT_DRAW.into(), if next == 1 { "folder-order" } else { "shuffled" }.into());
                    }
                }
                PracticeOption::InputDisplay => {
                    shell.settings.remembered.insert(KEPT_INPUTS.into(), if next == 1 { "on" } else { "off" }.into());
                    // (What each side holds is told from the next frame.)
                    session.inputs = next == 1;
                    session.held = [0; 2];
                    let ui = shell.ui();
                    let view = ui.global::<PracticeView>();
                    view.set_inputs(next == 1);
                    view.set_held_yours(0);
                    view.set_held_dummys(0);
                }
                PracticeOption::Added | PracticeOption::Dummy | PracticeOption::Battle | PracticeOption::Display => {}
            }
            shell.save_settings();
            // (At once, on the battle as it stands behind the menu.)
            if let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Practice) {
                b.stage.player.set_opponent(session.opponent);
                b.stage.player.set_training(session.options);
                if let Some(live) = b.stage.player.live_mut() {
                    session.ai = plug(live, session.opponent, session.level, session.seed, session.ai);
                }
            }
            // (The row's own value: its stepper slides to it. Then the rows its Action has under it.)
            tell_rows(shell, &session, &[option]);
            lay_out_rows(shell, &mut session);
        }
        shell.practice.session = Some(session);
    }
}

/// Where the build kept in file `file` (its name without the extension) is among `files`, a game's builds as
/// `store::list` has them.
fn place_of(files: &[store::Listed], file: &str) -> Option<usize> {
    files.iter().position(|b| b.path.file_stem().is_some_and(|stem| stem == file))
}

/// The build E and the pad's north button open: the dummy's (1) where the focus is on its row or its EDIT, else
/// the player's (0).
fn build_meant(shell: &Shell) -> usize {
    let dummys = shell.nav.focused().is_some_and(|k| k.id == "dummys" || (k.id == "edit" && k.index == 1));
    dummys as usize
}

/// The rule a build breaks, in the player's words, for the reserved line: its kind's sentence of the builds'
/// catalog with its numbers, and what it is about (`builds::words::problem`; docs/app.md §8).
fn rule(shell: &Shell, game: &str, ready: &Rc<Ready>, why: &Unusable) -> String {
    crate::builds::words::problem(shell, game, ready, &why.0)
}

impl Screen for Practice {
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
            // (Back from Builds comes back here.)
            "create" => crate::screens::builds::open(shell, ScreenId::Practice, None),
            "edit" => self.edit_build(shell, index.clamp(0, 1) as usize),
            "yours" | "dummys" => {
                let which = (id == "dummys") as usize;
                let options = self.builds.iter().map(|b| b.name.clone()).collect();
                shell.pick(id, 0, options, self.chosen[which]);
            }
            // (A place row steps on, as the menu's rows do.)
            "arena" => self.step_place(shell, ARENA, 1),
            "background" => self.step_place(shell, BACKGROUND, 1),
            // (The music's list opens: its names say more than a place's number.)
            "music" if !self.builds.is_empty() => crate::music::open(shell, id),
            "start" => self.start_practice(shell),
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
            "menu.rows.option" => self.step_option(shell, index, 1),
            // A row another screen added: it is told which.
            "menu.rows.added" => {
                if let Some(Session { from: Some((to, tag)), .. }) = &shell.practice.session {
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
            "arena" => self.step_place(shell, ARENA, by),
            "background" => self.step_place(shell, BACKGROUND, by),
            "music" if !self.builds.is_empty() => {
                crate::music::step(shell, by);
                self.tell_music(shell);
            }
            "menu.rows.option" => self.step_option(shell, index, by),
            _ => {}
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, _index: i32, choice: i32) {
        if matches!(id, "yours" | "dummys") {
            self.choose_build(shell, (id == "dummys") as usize, choice.max(0) as usize);
        }
        if id == "music" {
            crate::music::choose(shell, choice);
            self.tell_music(shell);
        }
    }

    /// The menu's row the focus is on is where the menu opens next.
    fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if let Some(session) = shell.practice.session.as_mut().filter(|_| id.starts_with("menu.")) {
            session.menu_focus = Key::new(id, index);
        }
    }

    /// R or F5 in a practice battle: the set again from its start. E on the entry, on a build's row or its EDIT:
    /// the build opened in Builds' editor.
    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        match key.code {
            KeyCode::KeyR | KeyCode::F5 => restart(shell),
            KeyCode::KeyE => self.edit_focused(shell),
            _ => false,
        }
    }

    /// The pad's north button: as R in a practice battle, as E on the entry.
    fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        action == Action::Alternate && (restart(shell) || self.edit_focused(shell))
    }

    fn button(&mut self, shell: &mut Shell, button: gamepad_facade::Button) -> bool {
        button == gamepad_facade::Button::North && restart(shell)
    }

    /// A dialog asked while a session another screen started was on is that screen's: it hears the answer.
    fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) {
        if let Some(Session { from: Some((to, _)), .. }) = &shell.practice.session {
            let to = *to;
            shell.tell(to, tag, choice);
        }
    }

    fn battle(&mut self, shell: &mut Shell, event: BattleEvent) {
        match event {
            // Under the practice menu the battle is in the game's own pause (the user: "the training pause should
            // actually just use the game's pause behavior: music continues, stuff still animates. but don't
            // show the PAUSED banner in the middle of the screen"): its HUD's pause display is left out.
            BattleEvent::Started => {
                if let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Practice) {
                    b.pause_in_game();
                    b.stage.player.renderer().pause_display = false;
                }
            }
            BattleEvent::Watching(_) => {}
            // Practice has no end: a set that ends starts over.
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
        // `NETTAI_PRACTICE=<game>`: straight into a practice battle of it, once it is ready. (Asked until it is
        // answered, and not at all without the variable: this runs every frame.)
        if !self.started {
            match std::env::var("NETTAI_PRACTICE") {
                Ok(game) if shell.game() != Some(game.as_str()) => shell.select_game(&game),
                Ok(_) if shell.ready().is_some() => {
                    self.started = true;
                    shell.go(ScreenId::Practice);
                    let options = kept_options(shell);
                    start_development_battle(shell, clock_seed(), options);
                }
                Ok(_) => {}
                Err(_) => self.started = true,
            }
        }
        // The session on: gone with its battle, however that went (another battle started in its place); and
        // one shown in another screen's `Picture` lasts while that screen is shown.
        let live = ours(shell) || shell.loading_of(ScreenId::Practice);
        let left = shell.practice.session.as_ref().is_some_and(|s| !s.full && s.from.is_some_and(|(from, _)| from != shell.screen()));
        if shell.practice.session.is_some() && (!live || left) {
            // (A battle of another's is left alone: `end` ends a practice session's.)
            end(shell);
        }
        // With its Action "a second pad", the second pad used is the dummy's (none of the player's or the menus'):
        // what it holds is the dummy's buttons this frame, none while the battle hasn't the keys.
        let apart = shell.practice.session.as_ref().is_some_and(|s| s.opponent.in_fight == InFight::SecondPad);
        shell.pad.apart = apart;
        let held = shell.pad.second(&shell.bindings);
        if let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Practice && (apart || b.stage.player.live().is_some_and(|l| l.opponent().in_fight == InFight::SecondPad))) {
            let playing = b.playing();
            if let Some(live) = b.stage.player.live_mut() {
                live.set_second_pad(held.map(|h| if playing { h } else { 0 }));
            }
        }
        // Its menu opens where the focus last was in it.
        let open = shell.battle.as_ref().is_some_and(|b| b.owner == ScreenId::Practice && b.menu_open());
        if let Some(session) = &mut shell.practice.session
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
                shell.ui().global::<PracticeView>().set_arena(false);
            }
            return;
        }
        let ui = shell.ui();
        let screen = ui.global::<nettai_ui::Shell>();
        if screen.get_screen() != ScreenId::Practice.view() || screen.get_leaving() {
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
                    ui.global::<PracticeView>().set_arena(true);
                }
            }
        }
    }

    /// After the battle's ticks: the round, for the line beside MENU on a phone, and with the input display on,
    /// the buttons each side holds.
    fn frame_end(&mut self, shell: &mut Shell, _now: Instant) {
        let Some(b) = shell.battle.as_ref().filter(|b| b.owner == ScreenId::Practice) else { return };
        let Some(session) = shell.practice.session.as_mut() else { return };
        let battle = b.stage.player.battle();
        // (The round's number is counted as the round opens: before its first tick it is the one before.)
        let round = battle.round.round.max(1);
        // (Under the menu the battle takes no button but the START that holds it in the game's pause, and the
        // dummy's go nowhere: nothing is shown held.)
        let held = if session.inputs && !b.menu_open() { [0, 1].map(|side| battle.custom.sides[side].joypad.held) } else { [0; 2] };
        let (new_round, pressed) = (session.round != Some(round), session.held != held);
        if !new_round && !pressed {
            return;
        }
        (session.round, session.held) = (Some(round), held);
        let ui = shell.ui();
        let view = ui.global::<PracticeView>();
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
    /// The dummy's AI is plugged in while its Action or its custom screen is the AI's, at the level chosen (a
    /// new one for another level), and taken out otherwise.
    #[test]
    fn the_dummys_ai_is_plugged_in_while_its_choices_are_the_ais() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 3, None).unwrap();
        let mut live = LivePlayer::new(Set::of(&content, &m, 3));
        let still = Opponent::default();
        let fights = Opponent { in_fight: InFight::Ai, ..still };
        let picks = Opponent { on_custom_screen: OnCustomScreen::Ai, ..still };
        assert_eq!(plug(&mut live, still, NORMAL, 3, None), None);
        assert_eq!(plug(&mut live, fights, NORMAL, 3, None), Some(NORMAL));
        assert_eq!(plug(&mut live, fights, 2, 3, Some(NORMAL)), Some(2), "another level: another fighter");
        assert_eq!(plug(&mut live, picks, 2, 3, Some(2)), Some(2), "the custom screen alone");
        assert_eq!(plug(&mut live, still, 2, 3, Some(2)), None, "taken out");
    }

    #[test]
    fn the_menus_rows_have_their_values() {
        let content = nettai_match::testing::exe6_content();
        let (opponent, options, places, mut seen) = (Opponent::default(), Options::default(), Places::of(&content, "exe6"), 0);
        for option in ROWS {
            if values(option, &places) > 0 {
                seen += 1;
                assert!(value(option, opponent, NORMAL, options, false, [0; 2]) < values(option, &places), "{option:?}");
            }
            // (A session another screen starts has the battle's options at their normal values and the places
            // Random: the first.)
            if matches!(option, PracticeOption::Arena | PracticeOption::Background | PracticeOption::YourHp | PracticeOption::DummysHp | PracticeOption::CustomGauge | PracticeOption::ChipDraw) {
                assert_eq!(value(option, opponent, NORMAL, options, false, [0; 2]), 0, "{option:?}");
            }
        }
        assert_eq!(seen, 9, "Action, Level, Arena, Background, Your HP, Dummy's HP, Custom gauge, Chip draw, Input display");
        assert_eq!(ACTIONS.map(|(_, a)| a), [InFight::StandsStill, InFight::MovesAround, InFight::Ai, InFight::SecondPad]);
        assert_eq!(value(PracticeOption::Action, dummy(InFight::Ai), NORMAL, options, false, [0; 2]), 2);
        assert_eq!(value(PracticeOption::Level, opponent, NORMAL, options, false, [0; 2]), 1, "Normal at first");
        assert_eq!(LEVELS.map(|(_, level)| level), [Level::EASY, Level::NORMAL, Level::HARD]);
        assert_eq!(value(PracticeOption::DummysHp, opponent, NORMAL, Options { refills: [false, true], ..options }, false, [0; 2]), 1);
        assert_eq!(value(PracticeOption::InputDisplay, opponent, NORMAL, options, true, [0; 2]), 1);
        assert_eq!(value(PracticeOption::Background, opponent, NORMAL, options, false, [3, 2]), 2);
        // (The arenas and the backgrounds after Random.)
        assert_eq!(values(PracticeOption::Arena, &places), 1 + nettai_match::link_battle_stages(&content, "exe6").len());
        assert_eq!(values(PracticeOption::Background, &places), 1 + nettai_match::ids::backgrounds(&content, "exe6").len());
    }

    /// The dummy's Level is under its Action while that is Fights alone; and its custom screen goes with its Action
    /// (the AI's for one that fights).
    #[test]
    fn the_dummys_action_brings_its_own_row() {
        // (How many rows it shows, and the one under the Action.)
        let under = |in_fight| {
            let rows = shown_rows(in_fight);
            let at = rows.iter().position(|o| *o == PracticeOption::Action).unwrap();
            (rows.len(), rows.get(at + 1).copied())
        };
        let (none, one) = (ROWS.len() - 1, ROWS.len());
        assert_eq!(under(InFight::StandsStill), (none, Some(PracticeOption::Battle)));
        assert_eq!(under(InFight::MovesAround), (none, Some(PracticeOption::Battle)));
        assert_eq!(under(InFight::Ai), (one, Some(PracticeOption::Level)));
        assert_eq!(under(InFight::SecondPad), (none, Some(PracticeOption::Battle)));
        assert_eq!(dummy(InFight::Ai).on_custom_screen, OnCustomScreen::Ai);
        assert_eq!(dummy(InFight::StandsStill), Opponent::default());
    }

    /// Each game's places are its link battle stages and its backgrounds, after Random: a place chosen is every
    /// round's (and the set's places are those), is remembered by its keys and comes back from them, and Random
    /// names nothing and is remembered as nothing. An arena is called by its key as a title, a background by the
    /// language's strings (each of the game's has a name there).
    #[test]
    fn the_places_are_the_games_after_random() {
        use nettai_match::testing::{exe4_content, exe5_content, exe6_content};
        for (game, content) in [("exe6", exe6_content()), ("exe5", exe5_content()), ("exe4", exe4_content())] {
            let places = Places::of(&content, game);
            let (mut arenas, mut stages) = (places.arenas.clone(), nettai_match::link_battle_stages(&content, game));
            arenas.sort();
            stages.sort();
            assert_eq!(arenas, stages, "{game}");
            assert!(places.len(ARENA) > 1 && places.len(BACKGROUND) > 1, "{game}");
            assert_eq!(places.round([0, 0]), RoundSettings::default(), "{game}: Random names nothing");
            let last = [places.len(ARENA) - 1, places.len(BACKGROUND) - 1];
            let mut m = nettai_match::pick::live(&content, game, 7, None).unwrap();
            places.put(&mut m, last);
            let (arena, background) = (*places.arenas.last().unwrap(), places.backgrounds.last().unwrap().clone());
            assert!(m.rounds.iter().all(|r| *r == RoundSettings { stage: Some(arena), background: Some(background.clone()) }), "{game}");
            let fought = m.places(&content, 7).unwrap();
            assert!(fought.iter().all(|p| p.stage == arena && nettai_match::ids::background_name(&content, p.background) == Some(background.as_str())), "{game}: every round there");
            assert_eq!(places.chosen(&content, &places.round(last)), last, "{game}");
            let mut remembered = std::collections::BTreeMap::new();
            places.keep(&content, &mut remembered, game, ARENA, last[ARENA]);
            places.keep(&content, &mut remembered, game, BACKGROUND, last[BACKGROUND]);
            assert_eq!(places.kept(&content, &remembered, game), last, "{game}");
            assert_eq!(remembered.get(&format!("training.background.{game}")), Some(&background));
            places.keep(&content, &mut remembered, game, ARENA, 0);
            assert_eq!(places.kept(&content, &remembered, game), [0, last[BACKGROUND]], "{game}: Random is remembered as nothing");
            assert_eq!(remembered.len(), 1);
            let [arenas, backgrounds] = places.labels(&Names { content: &content, strings: None });
            let n = places.arenas.len();
            assert_eq!((arenas[0].clone(), arenas[n - 1].clone()), (format!("1 / {n}"), format!("{n} / {n}")), "{game}: each by its place");
            let key = |at: usize| nettai_match::ids::local(&content.defs.stage(places.arenas[at]).key);
            assert_eq!([key(0), key(1), key(9)], ["netbattle-1", "netbattle-2", "netbattle-10"], "{game}: in their numbers' order");
            for (name, key) in backgrounds.iter().zip(&places.backgrounds) {
                assert!(!name.is_empty() && name != key && !name.contains(" / "), "{game}: {key} has a name");
            }
        }
    }

    /// A background chosen leaves the arenas as they were: with Arena Random the seed picks each round's arena
    /// whether the round names its background or not (the entry's preview and the menu's Background keep the
    /// arena).
    #[test]
    fn a_background_chosen_keeps_the_arenas() {
        use nettai_match::testing::{exe4_content, exe5_content, exe6_content};
        for (game, content) in [("exe6", exe6_content()), ("exe5", exe5_content()), ("exe4", exe4_content())] {
            let places = Places::of(&content, game);
            for seed in [1, 7, 0x1234_5678] {
                let fought = |background: usize| {
                    let mut m = nettai_match::pick::live(&content, game, seed, None).unwrap();
                    places.put(&mut m, [0, background]);
                    m.places(&content, seed).unwrap()
                };
                let random = fought(0);
                for background in [1, places.len(BACKGROUND) - 1] {
                    let chosen = fought(background);
                    assert_eq!(chosen.iter().map(|p| p.stage).collect::<Vec<_>>(), random.iter().map(|p| p.stage).collect::<Vec<_>>(), "{game} seed {seed}");
                    assert!(chosen.iter().all(|p| nettai_match::ids::background_name(&content, p.background) == Some(places.backgrounds[background - 1].as_str())));
                }
            }
        }
    }

    /// The builds chosen are remembered by game, the player's and the dummy's apart.
    #[test]
    fn builds_are_remembered_by_game() {
        assert_eq!(kept_build("exe6", false), "training.build.exe6");
        assert_eq!(kept_build("exe5", true), "training.dummy.exe5");
    }

    /// EDIT finds its build among the game's files by its file alone (Builds lists them in that order): not by
    /// its name, nor by a file whose name begins with its.
    #[test]
    fn a_build_is_found_among_the_files_by_its_file() {
        let listed = |file: &str, name: &str| store::Listed { path: std::path::PathBuf::from(format!("/data/builds/exe6/{file}.toml")), name: name.into(), text: String::new() };
        let files = [listed("Falzar heat", "Falzar heat"), listed("Falzar", "Gregar"), listed("Gregar", "Falzar")];
        assert_eq!(place_of(&files, "Falzar"), Some(1));
        assert_eq!(place_of(&files, "Gregar"), Some(2));
        assert_eq!(place_of(&files, "Falzar heat"), Some(0));
        assert_eq!(place_of(&files, "Falz"), None);
        assert_eq!(place_of(&[], "Falzar"), None);
    }
}
