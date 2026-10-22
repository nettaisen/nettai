//! Practice (docs/design/app-ui.md §6): you against a dummy set up from a menu while practicing. Its UI is
//! `ui/screens/practice.slint`: the practice menu and the input display, over the battle.
//!
//! **A practice session** is a battle against a dummy with the practice menu over it, and any screen may start
//! one ([`start`]): Practice does when the build editor's PRACTICE opens it, and Play does beside a waiting invite, with rows
//! of its own at the top of the menu in place of EXIT, the menu otherwise the same (but for Music and EDIT). It has no end and is not recorded: its battles are single battles, and one that ends is followed by another on a seed of its own. The session
//! is held by the shell (`Shell::practice`) and its battle is always this screen's (`ScreenId::Practice` owns
//! it), so this module hears its menu and its events whatever screen is shown.
//!
//! **The screen** ([`Practice`]) has no page of its own: its battle fills the window. The build editor's PRACTICE
//! opens it ([`open`]) with the build edited as the player's (the user: "the practice button should probably be
//! moved into the build editor"; it was Play's), and it goes straight into a session (the user: "i
//! feel like the practice button is in a weird place", then the whole play/build/practice flow rethought):
//! the dummy on the player's own build, Arena and Background Random, the game's own music, every option at its
//! normal value, and the practice menu open over the battle as it starts ("when practice mode starts you should
//! also bring up the menu immediately"), its focus on RESUME. The menu's EXIT, or Back, goes back to the editor
//! on that build ([`go_back`]). A [`Visit`] is the time from PRACTICE to leaving: what the menu sets lasts
//! through it, and the next starts on the defaults.
//!
//! **The practice menu** (Start or Esc; the game's own pause never runs) has the battle held behind it, its music
//! going on and its scenery moving as the game's pause shows them (`battle/hold.rs`). Its
//! rows are a label and a value between arrows, changed at once by Left and Right (Dummy's build, Arena,
//! Background and Music a dropdown: Enter opens its list, Arena's and Background's a grid of pictures): [`ROWS`]
//! lists them, and a row is added there. EDIT, under the Dummy's build, opens it in Builds' editor, whose
//! Back comes back to the session, started again on the build as the edit left it, the menu open. Under the
//! rows RESUME, RESTART and, in a session of Practice's own, EXIT.

use crate::battle::{BattleEvent, BattleStart, Play};
use crate::builds::{self, words};
use crate::errors::UserError;
use crate::games::{Names, Ready};
use crate::input::{Action, KeyPress};
use crate::screens::menu::clock_seed;
use crate::screens::{Screen, ScreenId};
use crate::settings::MAX_INPUT_DELAY;
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_frontend::driver::{ChipDraw, InFight, LivePlayer, OnCustomScreen, Opponent, Training as Options};
use nettai_match::build::Builds;
use nettai_battle::Content;
use nettai_battle::content::{ActorEntry, StageData};
use nettai_content_api::StageHandle;
use nettai_match::{Match, RoundSettings, Set, Side};
use nettai_ui::{BattleKind, PracticeOption, PracticeRow, PracticeView, VersusCard};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::rc::Rc;
use nettai_platform::time::Instant;

mod thumbs;

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
    /// What the dummy does at first, and what the battle's options are; the practice menu changes them.
    pub opponent: Opponent,
    pub options: Options,
    /// Where each round is fought: an arena and a background of the game's ([`Places`]: one it hasn't is
    /// Random), each picked from the seed where it names none (Random).
    pub round: RoundSettings,
    /// What the places left to Random, each folder's order and the dummy's random presses are picked from.
    pub seed: u32,
    /// The screen that starts it, if not Practice itself, and the tag it hears of it by: a row it added was
    /// chosen (`Screen::answered(tag, row)`, the row's place in `rows`). Its session has no EXIT (the
    /// screen's own rows are the ways out), no Music and no EDIT ([`shown_rows`]), and what its menu sets lasts
    /// for that screen's wait ([`stop`]), not for a visit to Practice.
    pub from: Option<(ScreenId, &'static str)>,
    /// The rows that screen adds at the top of the practice menu: their texts, in the language shown.
    pub rows: Vec<String>,
    /// That screen's EXIT: its word, in EXIT's place under RESUME and RESTART (Play's beside an invite, CANCEL
    /// INVITE; the user: "the stop hosting button should be in the place of the exit button on the regular
    /// practice menu"), heard as `Screen::answered(tag, `[`EXIT`]`)`; none, no EXIT.
    pub exit: Option<String>,
    /// The picture fills the window. Else the screen shown has a `Picture` of its own for it, and the session
    /// lasts while that screen is shown.
    pub full: bool,
    /// The two cards show while it is made ready.
    pub cards: bool,
    /// The builds the menu's Dummy's build offers, and the one the dummy is on (its place among them): a session
    /// of Practice's own has them, and [`start`] gives one another screen started the game's (none: the menu has
    /// no Dummy's build).
    pub builds: Vec<Pick>,
    pub dummys: usize,
    /// The practice menu opens over the battle as it starts (the editor's PRACTICE, and back from the editor).
    pub menu: bool,
}

/// A build the menu's Dummy's build offers: one of the game's builds that can be played, by its name, with the
/// side it brings and its place among the game's files (`store::list`'s order, which Builds' editor opens it by;
/// none: the screenshot pass's own, which have no file).
#[derive(Clone, Debug)]
pub struct Pick {
    pub build: Build,
    pub file: Option<usize>,
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
            options: Options::default(),
            round: RoundSettings::default(),
            seed: clock_seed(),
            from: None,
            rows: Vec::new(),
            exit: None,
            full: true,
            cards: true,
            builds: Vec::new(),
            dummys: 0,
            menu: false,
        }
    }

    /// A session screen `from` shows beside something of its own while the player waits (Play, by an invite's
    /// code): the dummy on the player's own build, standing still, every option at its normal value and the
    /// places Random (or, started again in the same wait, as the last session's menu was left); the picture in
    /// the screen's own `Picture`, with no cards before it; `rows` at the top of the practice menu, each heard by
    /// `tag`, in place of EXIT.
    pub fn beside(game: &str, build: Build, from: ScreenId, tag: &'static str, rows: Vec<String>) -> Start {
        Start { from: Some((from, tag)), rows, full: false, cards: false, ..Start::new(game, build.clone(), build) }
    }
}

/// What the shell holds of practice (`Shell::practice`).
#[derive(Default)]
pub struct Shared {
    /// The session on, if one is.
    session: Option<Session>,
    /// The screenshot pass: Practice lists the builds folder's files, as a player's are (the pass's own folder:
    /// what it made there to open in Builds), where it would list the pass's own builds.
    pub tour_files: bool,
    /// The visit to Practice on, and what it keeps.
    pub visit: Visit,
    /// The places' pictures, for the Arena and Background grids: made as they are needed, kept for the session.
    thumbs: thumbs::Thumbs,
    /// What the menu of a session another screen started was left at when that screen stopped it for a while
    /// ([`stop`]: a phone's STOP PRACTICE): its next session in the same wait starts on it. [`end`] forgets it.
    waiting: Option<Kept>,
}

/// A session's choices, kept from one to the next of the same wait ([`Shared::waiting`]).
struct Kept {
    from: ScreenId,
    opponent: Opponent,
    options: Options,
    place: [usize; 2],
    dummys: Option<String>,
}

/// A visit to Practice: from the build editor's PRACTICE until Practice is left other than by the menu's EDIT. The editor is part of it: the menu's EDIT goes there, and its Back comes back.
/// Through it the sessions keep the Dummy's build, Arena, Background and Music and the menu's choices the player
/// set; each visit starts on their defaults: the dummy on the player's own build, Random, Random, the game's own,
/// every option normal (the user: "going into practice should reset all the arena settings, there's no reason
/// for it to remember the last settings").
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Visit {
    /// A visit is on.
    on: bool,
    /// It came from the build editor's PRACTICE, which Practice goes back to, on the player's build and the tab
    /// PRACTICE was pressed on, this one (else from Play's build menu's, and back to Play).
    from_editor: Option<usize>,
    /// The player's build, by its name, and the game it is of: PLAY's chosen one (none: the one Play remembers).
    yours: Option<(String, String)>,
    /// The place chosen, and the game it is of (each its list's place, 0 Random: [`Places`]).
    place: Option<(String, [usize; 2])>,
    /// The battle's music picked (`Shell::battle_music`).
    pub music: crate::music::Music,
    /// The practice menu's choices, kept for the visit as the place is, each visit starting on their defaults (the
    /// user: "yes for the practice options, they should all reset to default"; they had been kept between runs).
    menu: MenuChoices,
    /// The dummy's build chosen, by its name, and the game it is of: the visit's too, the user: "you should also
    /// not remember the last dummy build either" (none: the player's own).
    dummys_build: Option<(String, String)>,
}

/// The practice menu's choices in a visit: the dummy's Action (none: Stands still), each side's
/// HP refilling, the custom gauge full, the chips drawn in the folder's order, the input display.
#[derive(Clone, Debug, Default, PartialEq)]
struct MenuChoices {
    action: Option<InFight>,
    refills: [bool; 2],
    full_gauge: bool,
    folder_order: bool,
    inputs: bool,
}

impl Visit {
    /// PRACTICE (the build editor's on its tab `tab`, or Play's build menu's): a visit starts, on the defaults, the
    /// player's build `yours` of `game` (its name).
    fn new(game: &str, yours: &str, tab: Option<usize>) -> Visit {
        Visit { on: true, from_editor: tab, yours: Some((game.to_string(), yours.to_string())), ..Visit::default() }
    }

    /// Practice is shown: a visit starts, on the defaults, unless one is on. Whether one started.
    pub fn arrive(&mut self) -> bool {
        if self.on {
            return false;
        }
        *self = Visit { on: true, ..Visit::default() };
        true
    }

    /// Screen `screen` is the one shown: one other than Practice and Builds' editor ends the visit.
    pub fn shown(&mut self, screen: ScreenId) {
        if !matches!(screen, ScreenId::Practice | ScreenId::Builds) {
            self.leave();
        }
    }

    /// The visit is over (Practice left, or the screenshot pass starting a walk): what it kept goes, and the next
    /// starts on the defaults.
    pub fn leave(&mut self) {
        *self = Visit::default();
    }

    /// `game`'s build `from` was renamed `to` (in the editor's header, which the menu's EDIT opened): the visit keeps
    /// it by its new name.
    pub fn renamed(&mut self, game: &str, from: &str, to: &str) {
        for (of, name) in [&mut self.yours, &mut self.dummys_build].into_iter().flatten() {
            if of == game && name == from {
                *name = to.to_string();
            }
        }
    }

    /// The name of `game`'s build the visit has for side `side` (0 the player's, 1 the dummy's), if it has one.
    fn build(&self, game: &str, side: usize) -> Option<&str> {
        let kept = if side == 0 { &self.yours } else { &self.dummys_build };
        kept.as_ref().filter(|(of, _)| of == game).map(|(_, name)| name.as_str())
    }

    /// The place chosen for `game` in this visit, in `places` (Random where none is, or for another game).
    fn place(&self, game: &str, places: &Places) -> [usize; 2] {
        match &self.place {
            Some((of, place)) if of == game => [ARENA, BACKGROUND].map(|part| if place[part] < places.len(part) { place[part] } else { 0 }),
            _ => [0; 2],
        }
    }

    /// Choose `place` for `game` for the rest of the visit.
    fn set_place(&mut self, game: &str, place: [usize; 2]) {
        self.place = Some((game.to_string(), place));
    }
}

/// The practice session on.
struct Session {
    /// The screen that started it, if not Practice, and the tag it hears of it by; whether that screen gave it an
    /// EXIT.
    from: Option<(ScreenId, &'static str)>,
    exit: bool,
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
    /// The battle's music, as its row says it (`crate::music::shown`: the player's one pick).
    music: String,
    /// Its picture fills the window.
    full: bool,
    /// The rows its starter added: the menu's first; and the menu's own shown under them ([`shown_rows`]).
    added: usize,
    rows: Vec<PracticeOption>,
    /// What the dummy does, the battle's options, and whether the input display is on.
    opponent: Opponent,
    options: Options,
    inputs: bool,
    /// The builds the Dummy's build row offers, and the one the dummy is on (none: the row isn't there).
    builds: Vec<Pick>,
    dummys: usize,
    /// The menu is to open as the battle starts.
    menu_at_start: bool,
    /// The menu is open (it opens on RESUME every time).
    menu_open: bool,
    /// What the UI was last told: the round, and the buttons each side holds.
    round: Option<u8>,
    held: [u16; 2],
}

/// The practice menu's rows, in order, under whatever rows another screen added: each group's title, then its
/// options. A row is added here, worded in practice.slint (`PracticeView.label` and `value`), and given what it
/// does in [`values`], [`value`] and `Practice::change_option`. The dummy's build and EDIT are shown in a session
/// of Practice's own ([`shown_rows`]).
pub(crate) const ROWS: [PracticeOption; 15] = [
    PracticeOption::Dummy,
    // (Who the dummy is, then what it does: the user, "add Dummy's build to its DUMMY group".)
    PracticeOption::DummysBuild,
    PracticeOption::EditDummy,
    PracticeOption::Action,
    // (Where the battle is apart from the battle's options: the user, "you should split the practice menu settings
    // into arena and battle so the arena settings aren't mixed together"; and the groups named for neither of their
    // rows, none of one row: "having an "arena" field under "arena" is so weird".)
    PracticeOption::Stage,
    PracticeOption::Arena,
    PracticeOption::Background,
    PracticeOption::Music,
    PracticeOption::Options,
    PracticeOption::YourHp,
    PracticeOption::DummysHp,
    PracticeOption::CustomGauge,
    PracticeOption::ChipDraw,
    PracticeOption::InputDisplay,
    // (Netplay's input delay, the one setting: the user, "practice mode should have the same input delay as netplay
    // and adjusting it should adjust the netplay delay".)
    PracticeOption::InputDelay,
];

/// What a session's starter hears when its EXIT is chosen ([`Start::exit`]): its rows are heard by their places.
pub const EXIT: usize = usize::MAX;

/// The practice menu's buttons under its rows (practice.slint's `PracticeMenu`): RESUME, RESTART, and in a session
/// of Practice's own EXIT, back to Play (the user: "you should also get rid of "quit to menu" and "change
/// builds" should just be "exit""). A session another screen started has its own rows as its ways out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuButton {
    Resume,
    Restart,
    Exit,
}

/// The menu's buttons by their controls' names.
const MENU_BUTTONS: [(&str, MenuButton); 3] = [("menu.resume", MenuButton::Resume), ("menu.restart", MenuButton::Restart), ("menu.exit", MenuButton::Exit)];

/// The menu's button whose control is `id`, if it is one.
fn menu_button(id: &str) -> Option<MenuButton> {
    MENU_BUTTONS.iter().find(|(name, _)| *name == id).map(|(_, b)| *b)
}

/// How many values a row steps through (none: a group's title, or EDIT; Arena and Background the
/// game's `places` and Random).
fn values(option: PracticeOption, places: &Places) -> usize {
    match option {
        PracticeOption::Arena => places.len(ARENA),
        PracticeOption::Background => places.len(BACKGROUND),
        // (The battle's music steps through what `crate::music` offers, and the dummy's build through the session's
        // builds: `Practice::change_option`.)
        PracticeOption::Music | PracticeOption::DummysBuild => 1,
        PracticeOption::Action => ACTIONS.len(),
        PracticeOption::YourHp | PracticeOption::DummysHp | PracticeOption::CustomGauge | PracticeOption::ChipDraw | PracticeOption::InputDisplay => 2,
        PracticeOption::InputDelay => MAX_INPUT_DELAY as usize + 1,
        PracticeOption::Added | PracticeOption::Dummy | PracticeOption::Stage | PracticeOption::Options | PracticeOption::EditDummy => 0,
    }
}

/// The value a row has: its place among its values.
fn value(option: PracticeOption, opponent: Opponent, options: Options, inputs: bool, place: [usize; 2]) -> usize {
    match option {
        PracticeOption::Arena => place[ARENA],
        PracticeOption::Background => place[BACKGROUND],
        PracticeOption::Music | PracticeOption::DummysBuild => 0,
        PracticeOption::Action => ACTIONS.iter().position(|a| *a == opponent.in_fight).unwrap_or(0),
        PracticeOption::YourHp => options.refills[0] as usize,
        PracticeOption::DummysHp => options.refills[1] as usize,
        PracticeOption::CustomGauge => options.full_gauge as usize,
        PracticeOption::ChipDraw => (options.chip_draw == ChipDraw::FolderOrder) as usize,
        PracticeOption::InputDisplay => inputs as usize,
        // (The setting's, not the session's: `row`.)
        PracticeOption::InputDelay => 0,
        PracticeOption::Added | PracticeOption::Dummy | PracticeOption::Stage | PracticeOption::Options | PracticeOption::EditDummy => 0,
    }
}

/// A row as the menu shows it: its value's place; for Arena and Background, the name of the one chosen (none:
/// Random) and its picture (`thumbs`'); for Music, the battle's music's name; for the Dummy's build, its name; for
/// the Input delay, the setting's ticks (`delay`).
fn row(option: PracticeOption, session: &Session, thumbs: &thumbs::Thumbs, delay: u32) -> PracticeRow {
    if option == PracticeOption::InputDelay {
        return PracticeRow { option, value: delay as i32, text: Default::default(), picture: Default::default() };
    }
    if option == PracticeOption::DummysBuild {
        let name = session.builds.get(session.dummys).map(|p| p.build.name.as_str()).unwrap_or_default();
        return PracticeRow { option, value: session.dummys as i32, text: name.into(), picture: Default::default() };
    }
    let value = value(option, session.opponent, session.options, session.inputs, session.place) as i32;
    let part = match option {
        PracticeOption::Arena => Some(ARENA),
        PracticeOption::Background => Some(BACKGROUND),
        _ => None,
    };
    let text = part.and_then(|part| session.place[part].checked_sub(1).and_then(|at| session.place_labels[part].get(at))).map(|name| name.as_str().into()).unwrap_or_default();
    let text = if option == PracticeOption::Music { session.music.as_str().into() } else { text };
    let picture = part.map(|part| thumbs.picture(&session.game, part, session.place[part])).unwrap_or_default();
    PracticeRow { option, value, text, picture }
}

/// The menu's rows shown: [`ROWS`], but for the Dummy's build and its EDIT where the session offers no builds (`builds`), and beside a waiting invite
/// (`beside`, the user: "practice menu in the matchmaking waiting screen should be the same as the usual
/// practice menu") for two rows that would do nothing there or drop the wait: the Music, its music being the
/// menus' (the user: "in the waiting for opponent screen, it really shouldn't let you change the music because
/// it does nothing"), and EDIT, whose editor would leave the screen that holds the invite (Play leaves its link
/// as it is hidden).
fn shown_rows(beside: bool, builds: bool) -> Vec<PracticeOption> {
    ROWS.into_iter()
        .filter(|o| match o {
            PracticeOption::Music => !beside,
            PracticeOption::DummysBuild => builds,
            PracticeOption::EditDummy => builds && !beside,
            _ => true,
        })
        .collect()
}

/// Show `options`' rows as `session` has them now (those shown).
fn tell_rows(shell: &Shell, session: &Session, options: &[PracticeOption]) {
    let rows = shell.ui().global::<PracticeView>().get_rows();
    for option in options {
        if let Some(at) = session.rows.iter().position(|o| o == option) {
            rows.set_row_data(session.added + at, row(*option, session, &shell.practice.thumbs, shell.settings.input_delay));
        }
    }
}

/// The dummy that does `in_fight`: on its custom screen it picks the first chip it can (a second pad's own
/// presses there).
fn dummy(in_fight: InFight) -> Opponent {
    Opponent { in_fight, on_custom_screen: OnCustomScreen::PicksFirst }
}

/// Where the menu shows `option` now, among its rows (another screen's added first), if it shows it: for the
/// screenshot pass.
#[cfg(any(test, feature = "tour"))]
pub(crate) fn row_of(shell: &Shell, option: PracticeOption) -> Option<i32> {
    let session = shell.practice.session.as_ref()?;
    session.rows.iter().position(|o| *o == option).map(|at| (session.added + at) as i32)
}

/// The place among its builds of the one the dummy of the session on is on, where it offers builds: for the
/// screenshot pass.
#[cfg(any(test, feature = "tour"))]
pub(crate) fn dummys_build(shell: &Shell) -> Option<usize> {
    shell.practice.session.as_ref().filter(|s| !s.builds.is_empty()).map(|s| s.dummys)
}

/// The dummy's Actions, in the row's order.
const ACTIONS: [InFight; 3] = [InFight::StandsStill, InFight::MovesAround, InFight::SecondPad];

/// The dummy's setup as this visit last left it in one of Practice's own sessions.
fn kept_opponent(shell: &Shell) -> Opponent {
    dummy(shell.practice.visit.menu.action.unwrap_or(InFight::StandsStill))
}

/// The battle's options as this visit last left them in one of Practice's own sessions.
fn kept_options(shell: &Shell) -> Options {
    let menu = &shell.practice.visit.menu;
    let chip_draw = if menu.folder_order { ChipDraw::FolderOrder } else { ChipDraw::Shuffled };
    Options { refills: menu.refills, full_gauge: menu.full_gauge, chip_draw }
}

/// Start a practice session: the two cards while it is made ready (if it has them), then the battle, with the
/// practice menu behind Start and Esc. A session already on ends first. Err: why it can't start, as the player
/// reads it (what the code said is in the log).
pub fn start(shell: &mut Shell, mut start: Start) -> Result<(), UserError> {
    // A session another screen started (Play's, beside a waiting invite) has the usual menu: the Dummy's build
    // offers the game's builds that can be played, the dummy on the player's own at first; and one started
    // again in the same wait goes on with what the last was left at.
    let kept = start.from.and_then(|(from, _)| shell.practice.waiting.take().filter(|k| k.from == from));
    if start.from.is_some()
        && start.builds.is_empty()
        && let Some(kit) = builds::kit(shell).filter(|kit| kit.game == start.game)
    {
        start.builds = picks(&listed(shell, &kit));
        let named = |name: &str| start.builds.iter().position(|p| p.build.name == name);
        let dummys = kept.as_ref().and_then(|k| k.dummys.as_deref()).and_then(named);
        match (dummys, named(&start.you.name)) {
            (Some(at), _) => {
                start.dummys = at;
                start.dummy = start.builds[at].build.clone();
            }
            (None, Some(at)) => start.dummys = at,
            // (The player's own isn't among them: the menu offers none.)
            (None, None) => start.builds.clear(),
        }
    }
    if let Some(k) = &kept {
        (start.opponent, start.options) = (k.opponent, k.options);
    }
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
    // (Single battles, one after another: the user, "training mode should just be repeated single battles". A
    // match of one round is the single battle's type, with no BATTLE 1 START! banner: `nettai_match::match_effects`.)
    m.rounds.truncate(1);
    let places = Places::of(content, &start.game);
    let place = match &kept {
        Some(k) => [ARENA, BACKGROUND].map(|part| if k.place[part] < places.len(part) { k.place[part] } else { 0 }),
        None => places.chosen(content, &start.round),
    };
    want_pictures(shell, &start.game);
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
        // (The dummy has no name: its build is who it is.)
        let mut cards = [card(shell, &ready, view.get_you(), &shell.settings.name, &start.you), card(shell, &ready, view.get_dummy(), "", &start.dummy)];
        cards[1].name = std::mem::take(&mut cards[1].build);
        cards
    });
    let inputs = shell.practice.visit.menu.inputs;
    let place_labels = places.labels(content);
    let session = Session {
        from: start.from,
        exit: start.exit.is_some(),
        game: start.game.clone(),
        m,
        seed: start.seed,
        places,
        place_labels,
        place,
        music: crate::music::shown(shell),
        full: start.full,
        added: start.rows.len(),
        rows: shown_rows(start.from.is_some(), !start.builds.is_empty()),
        opponent: start.opponent,
        options: start.options,
        inputs,
        dummys: start.dummys.min(start.builds.len().saturating_sub(1)),
        builds: start.builds,
        menu_at_start: start.menu,
        menu_open: false,
        round: None,
        held: [0; 2],
    };
    let rows: Vec<PracticeRow> = start
        .rows
        .iter()
        .map(|text| PracticeRow { option: PracticeOption::Added, value: 0, text: text.into(), picture: Default::default() })
        .chain(session.rows.iter().map(|o| row(*o, &session, &shell.practice.thumbs, shell.settings.input_delay)))
        .collect();
    view.set_rows(ModelRc::new(VecModel::from(rows)));
    view.set_leave(start.from.is_none() || start.exit.is_some());
    view.set_exit_word(start.exit.clone().unwrap_or_default().into());
    view.set_session_yours(start.you.name.as_str().into());
    view.set_session_dummys(start.dummy.name.as_str().into());
    view.set_inputs(inputs);
    view.set_held_yours(0);
    view.set_held_dummys(0);
    // (The player's input delay netplay's, the one setting both change: the user, "practice mode should have the same
    // input delay as netplay and adjusting it should adjust the netplay delay".)
    let driver = LivePlayer::against(set_of(content, &session.m, start.seed), start.opponent, start.seed as u64)
        .with_training(start.options)
        .with_input_delay(shell.settings.input_delay);
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
        // (Practice's own plays the pick; the session beside a waiting invite leaves the menus' music on.)
        music: if start.from.is_none() { crate::battle::BattleMusic::Picked } else { crate::battle::BattleMusic::None },
        gate: None,
    });
    shell.practice.session = Some(session);
    Ok(())
}

/// The set a practice session plays: the match's, its rounds' START the host's as every match's
/// (`HostKeys::pause`): START in the fight opens the practice menu, which holds the battle (`battle/hold.rs`),
/// and the game's own pause never runs. (Practice is neither online nor recorded.)
fn set_of(content: &std::sync::Arc<nettai_battle::Content>, m: &Match, seed: u32) -> Set {
    let first = m.round(content, seed);
    let folders = [0, 1].map(|side| m.sides[side].folder(content).saved().expect("a checked match's folder is whole"));
    Set::new(content.clone(), first, folders)
}

/// End the practice session on, if one is: its battle (or the cards before it) goes, and the screen shown is
/// what shows. What starts a session from outside calls this when its wait is over (an opponent joined, the
/// invite canceled): what its menu was left at is forgotten.
pub fn end(shell: &mut Shell) {
    shell.practice.waiting = None;
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

/// End the practice session another screen started for a while, its wait going on (a phone's STOP PRACTICE):
/// its next session in the same wait starts where its menu was left (the Dummy's build, the dummy's Action, the
/// place and the battle's options).
pub fn stop(shell: &mut Shell) {
    let kept = shell.practice.session.as_ref().and_then(|s| {
        let (from, _) = s.from?;
        let dummys = s.builds.get(s.dummys).map(|p| p.build.name.clone());
        Some(Kept { from, opponent: s.opponent, options: s.options, place: s.place, dummys })
    });
    end(shell);
    shell.practice.waiting = kept;
}

/// Whether a practice session is on (being made ready, or running).
#[cfg_attr(not(feature = "tour"), allow(dead_code))] // (The screenshot pass asks.)
pub fn running(shell: &Shell) -> bool {
    shell.practice.session.is_some()
}

/// The place the practice session on is fought on (its lists' places, 0 Random).
#[cfg_attr(not(feature = "tour"), allow(dead_code))] // (The screenshot pass asks.)
pub(crate) fn session_place(shell: &Shell) -> Option<[usize; 2]> {
    shell.practice.session.as_ref().map(|s| s.place)
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

/// The session's set again from its start on the arena and with the dummy's build it has now (the Arena row
/// changed, or the Dummy's build: a round's field and its navis are set as it starts), as RESTART starts it
/// over, the menu staying open: the battle behind it steps through the round's opening and is held as its
/// custom screen begins, as ever under the menu (`battle/hold.rs`). (What a new build or arena draws that the battle
/// wasn't made ready for is loaded as it is first drawn.)
fn restart_set(shell: &mut Shell, session: &mut Session) {
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

/// How a menu's option changes: stepped by so many, or to a value chosen from its list.
#[derive(Clone, Copy)]
enum Change {
    By(i32),
    To(usize),
}

/// Open the arenas or the backgrounds (`part`) of `game` as the grid of the select `id`, `index` (the menu's
/// row): Random, then `labels`, the list's as the rows say them, each over its picture (the user:
/// "arena selector should be like a dropdown that shows previews of all arenas. background dropdown should be
/// similar"); place `at` chosen.
fn open_places(shell: &mut Shell, id: &str, index: i32, game: &str, part: usize, labels: &[String], at: usize) {
    let random = shell.ui().global::<PracticeView>().get_random().to_string();
    let names: Vec<String> = std::iter::once(random).chain(labels.iter().cloned()).collect();
    let pictures = shell.practice.thumbs.open(game, part, names.len());
    shell.pick_grid(id, index, names, at, ModelRc::from(pictures));
}

/// Make the pictures of `game`'s places for its grids, if they aren't (on a thread of their own; the screenshot
/// pass's at once).
fn want_pictures(shell: &mut Shell, game: &str) {
    let Some(ready) = shell.games.ready(game) else { return };
    let now = shell.still;
    shell.practice.thumbs.want(game, ready.content(), &ready.loaded.graphics, now);
}

/// A build's card before the battle, `role` (YOU, DUMMY) over `name`: the build's name and its navi's name in the
/// language shown.
fn card(shell: &Shell, ready: &Rc<Ready>, role: slint::SharedString, name: &str, build: &Build) -> VersusCard {
    let content = ready.content();
    let graphics = ready.graphics(shell.lang);
    let names = Names::of(content, &graphics);
    VersusCard {
        role,
        name: name.into(),
        build: build.name.as_str().into(),
        navi: names.navi(build.side.navi(content)).into(),
        progress: 0.0,
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
    let start = Start { seed, options, ..Start::new(&game, builds[0].clone(), dummy) };
    // (Why it can't start is in the log.)
    let _ = self::start(shell, start);
}

/// A new build from each of `game`'s groups (its versions), named for its version.
pub fn new_builds(content: &std::sync::Arc<nettai_battle::Content>, game: &str) -> Result<Vec<Build>, String> {
    let builds = Builds::of(content, game)?;
    builds
        .groups
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

/// The places a practice set may be fought on, as the menu steps through them: the game's arenas
/// (the link battle stages a match may name, `nettai_match::link_battle_stages`: the panel layouts and what
/// stands on them that its netbattles are fought on) and the background pool, every game's battle backgrounds
/// (`nettai_match::ids::pool_backgrounds`; docs/app.md §1, "The battle's background"), the games in their order
/// and each list in its keys' natural order (a run of digits by its number: `netbattle-2` before
/// `netbattle-10`; a background by its name in its game's pack). Each list is stepped through after Random: the
/// place picked from the seed each round, as a match that names none has it (what practice did before it had
/// them), a background from the whole pool (`BackgroundPick::Pool`). A choice is a list's place, 0 Random.
///
/// The games keep the two apart: a link battle picks its stage and its background each on its own (the rules'
/// `link_pick`), so either goes with any of the other. An arena is said by its place in the list ("12 / 48":
/// the user, "the arena names are meaningless"); a background by its game's name and what that game's locales
/// call it (`crate::backgrounds::name`: "BN4 ACDC Area").
///
/// The arenas are listed once each: a game's link battle stages hold an arena more than once, as records that
/// differ in nothing but the order their actor lists place in ([`same_arena`]: EXE6's each arena twice or four
/// times, one record placing side 0's navi first and another side 1's), and the list shows the first of each
/// such group in its order (the user: "don't show duplicate arenas in the practice mode selector, just show the
/// first of any duplicates"). An arena chosen plays its first record, so a round on it always updates the same
/// side first; Random is the rules' pick over every record, as a match's, the side a coin flip each round.
#[derive(Clone, Default)]
pub struct Places {
    /// Each arena's records, in the list's order: the first is the one it plays.
    arenas: Vec<Vec<StageHandle>>,
    /// By the names a match writes for them: the game's own by its name in its pack, another game's with its
    /// game (`exe4:acdc-area`).
    backgrounds: Vec<String>,
}

impl Places {
    pub fn of(content: &Content, game: &str) -> Places {
        let mut stages = nettai_match::link_battle_stages(content, game);
        stages.sort_by_cached_key(|&s| natural(nettai_match::ids::local(&content.defs.stage(s).key)));
        let mut arenas: Vec<Vec<StageHandle>> = Vec::new();
        for s in stages {
            match arenas.iter_mut().find(|records| same_arena(content.stage(records[0]), content.stage(s))) {
                Some(records) => records.push(s),
                None => arenas.push(vec![s]),
            }
        }
        let mut backgrounds: Vec<String> = nettai_match::ids::pool_backgrounds(content).into_iter().map(str::to_string).collect();
        backgrounds.sort_by_cached_key(|b| pool_background(content, game, b).map(|(of, name)| (of.to_string(), natural(name))));
        Places { arenas, backgrounds }
    }

    /// How many choices list `part` has, Random with them.
    fn len(&self, part: usize) -> usize {
        1 + if part == ARENA { self.arenas.len() } else { self.backgrounds.len() }
    }

    /// A round on the place `chosen`.
    fn round(&self, chosen: [usize; 2]) -> RoundSettings {
        RoundSettings {
            stage: chosen[ARENA].checked_sub(1).and_then(|at| self.arenas.get(at)).map(|records| records[0]),
            background: chosen[BACKGROUND].checked_sub(1).and_then(|at| self.backgrounds.get(at)).cloned(),
        }
    }

    /// Every round of `m` on the place `chosen`, a Random background drawn from the background pool.
    fn put(&self, m: &mut Match, chosen: [usize; 2]) {
        let round = self.round(chosen);
        m.rounds.iter_mut().for_each(|r| *r = round.clone());
        m.backgrounds = nettai_match::BackgroundPick::Pool;
    }

    /// The place `round` states, by the lists' places (what it names that they haven't: Random).
    fn chosen(&self, content: &Content, round: &RoundSettings) -> [usize; 2] {
        let arena = round.stage.map(|s| nettai_match::ids::local(&content.defs.stage(s).key));
        [self.find(content, ARENA, arena), self.find(content, BACKGROUND, round.background.as_deref())]
    }

    /// Where the choice named by `key` is in list `part` (none, or one the list hasn't: Random). A stage is
    /// found as its arena, the first of its records or another.
    fn find(&self, content: &Content, part: usize, key: Option<&str>) -> usize {
        let stage = |s: StageHandle| nettai_match::ids::local(&content.defs.stage(s).key);
        let named = |at: usize| match part {
            ARENA => self.arenas[at - 1].iter().any(|&s| key == Some(stage(s))),
            _ => self.key(content, part, at) == key,
        };
        key.and_then(|_| (1..self.len(part)).find(|&at| named(at))).unwrap_or(0)
    }

    /// The key choice `at` of list `part` is named by: the stage its arena plays, the name a match writes for a
    /// background (none: Random).
    fn key<'c>(&'c self, content: &'c Content, part: usize, at: usize) -> Option<&'c str> {
        let at = at.checked_sub(1)?;
        if part == ARENA { self.arenas.get(at).map(|records| nettai_match::ids::local(&content.defs.stage(records[0]).key)) } else { self.backgrounds.get(at).map(String::as_str) }
    }

    /// What each choice of the two lists but Random is shown as, in the language shown: an arena its place in
    /// the list ("12 / 48"), a background its game's name and its own ("BN4 ACDC Area").
    fn labels(&self, content: &Content) -> [Vec<String>; 2] {
        let place = |at: usize, of: usize| format!("{} / {of}", at + 1);
        let background = |b: &String| match pool_background(content, content.game(), b) {
            Some((game, name)) => crate::backgrounds::name(game, name),
            None => b.clone(),
        };
        [(0..self.arenas.len()).map(|at| place(at, self.arenas.len())).collect(), self.backgrounds.iter().map(background).collect()]
    }
}

/// Whether two stages are the same arena: everything they state the same, the actors they place with them,
/// whatever the order their actor lists place them in (which only says who is updated first), and whatever their
/// effects (the user, of EXE6's netbattle-95 and 96 and EXE5's 93 and 94, which differ in effects alone, by a bit
/// nothing reads: "they should count as duplicates too").
fn same_arena(a: &StageData, b: &StageData) -> bool {
    let StageData { layout, panel_pattern, music, background, mode, battle_number, effects: _, actors } = a;
    let count = |list: &[ActorEntry], e: &ActorEntry| list.iter().filter(|x| *x == e).count();
    (layout, panel_pattern, music, background, mode, battle_number) == (&b.layout, &b.panel_pattern, &b.music, &b.background, &b.mode, &b.battle_number)
        && actors.len() == b.actors.len()
        && actors.iter().all(|e| count(actors, e) == count(&b.actors, e))
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

/// The game and the name in its pack of the background a match of `game` names `written` (one of the pool's).
fn pool_background<'c>(content: &'c Content, game: &str, written: &str) -> Option<(&'c str, &'c str)> {
    nettai_match::ids::background(content, game, written).and_then(|id| content.background_of(id))
}

// ---- The screen ----------------------------------------------------------------------------------------------

/// PRACTICE: a visit to Practice starts, the player's build `build` of `game` (its name, as its file names it), and
/// Practice goes straight into a session on it ([`Practice`]); leaving, it goes back to the editor on that build and
/// its tab `tab` where the editor's PRACTICE opened it, else (Play's build menu's: none) to Play ([`go_back`]).
pub fn open(shell: &mut Shell, game: &str, build: &str, tab: Option<usize>) {
    shell.practice.visit = Visit::new(game, build, tab);
    shell.go(ScreenId::Practice);
}

/// Leave Practice for where its visit came from: the build editor, on the player's build as it is kept now (by its
/// place among the game's files: one gone meanwhile, or a visit from elsewhere, goes to Play) and the tab PRACTICE
/// was pressed on (the user: "returning from practice should return to the same tab"), Back from there going to
/// Play.
fn go_back(shell: &mut Shell) {
    let visit = &shell.practice.visit;
    let at = visit.from_editor.zip(visit.yours.as_ref()).and_then(|(tab, (game, name))| Some((builds::store::list(game).iter().position(|l| l.name == *name)?, tab)));
    match at {
        Some((at, tab)) => crate::screens::builds::open_on_tab(shell, ScreenId::Play, at, tab),
        None => shell.go(ScreenId::Play),
    }
}

/// The game's builds as Practice reads them: as Play lists them (`play::shelved`), or with the screenshot pass's
/// `tour_files` the builds folder's files; each checked under the ruleset it is made for, so a build that can't be
/// played under it can't be practiced either.
fn listed(shell: &Shell, kit: &builds::Kit) -> Vec<builds::shelf::Shelved> {
    match shell.practice.tour_files {
        true => builds::shelf::load(shell, kit),
        false => crate::screens::play::shelved(shell, kit),
    }
}

/// The builds of `listed` the menu's Dummy's build offers: those that can be played.
fn picks(listed: &[builds::shelf::Shelved]) -> Vec<Pick> {
    listed
        .iter()
        .enumerate()
        .filter_map(|(at, b)| {
            let side = b.side().filter(|_| b.usable())?.clone();
            // (The screenshot pass's own have no file.)
            let file = (!b.listed.path.as_os_str().is_empty()).then_some(at);
            Some(Pick { build: Build { name: b.name().to_string(), side }, file })
        })
        .collect()
}

/// Practice: no page of its own, its session's battle filling the window. Shown, it starts a session of its own on
/// the visit ([`Practice::start_own`]); EXIT or Back goes back to the editor ([`go_back`]).
#[derive(Default)]
pub struct Practice {
    /// `NETTAI_PRACTICE` was answered.
    started: bool,
    /// Builds' editor is being opened from the menu (its EDIT), whose Back comes back: leaving for it
    /// goes on with the visit.
    away: bool,
}

impl Practice {
    /// A session of Practice's own, as the visit has it: the player's build the visit's (PLAY's chosen one; with
    /// none, the one Play remembers, else the first that can be played), the dummy's the visit's (at first the
    /// player's own), the place, the music and the menu's choices the visit's (at first Random, Random, the
    /// game's own, every option normal), the menu open as the battle starts. The Dummy's build row offers the
    /// game's builds that can be played. Err: why it can't start (no build, or the player's can't be played).
    fn start_own(&mut self, shell: &mut Shell) -> Result<(), UserError> {
        let Some(kit) = builds::kit(shell) else { return Err(UserError::Other) };
        let game = kit.game.clone();
        let listed = listed(shell, &kit);
        let named = |name: &str| listed.iter().position(|b| b.name() == name);
        let yours = shell.practice.visit.build(&game, 0).map(str::to_string).or_else(|| crate::screens::play::remembered(shell, &game));
        let yours = yours.and_then(|name| named(&name)).or_else(|| listed.iter().position(|b| b.usable())).ok_or(UserError::NoBuild)?;
        let entry = &listed[yours];
        if !entry.usable() {
            let reason = words::summary(shell, &kit, &entry.problems);
            return Err(UserError::BuildUnusable { reason: reason.filter(|r| !r.is_empty()) });
        }
        let picks = picks(&listed);
        let at = |name: &str| picks.iter().position(|p| p.build.name == name);
        let you = at(entry.name()).ok_or(UserError::Other)?;
        let dummys = shell.practice.visit.build(&game, 1).and_then(at).unwrap_or(you);
        let places = Places::of(kit.content(), &game);
        let round = places.round(shell.practice.visit.place(&game, &places));
        let (you_build, dummy_build) = (picks[you].build.clone(), picks[dummys].build.clone());
        let start = Start {
            opponent: kept_opponent(shell),
            options: kept_options(shell),
            round,
            seed: if shell.still { 7 } else { clock_seed() },
            builds: picks,
            dummys,
            menu: true,
            ..Start::new(&game, you_build, dummy_build)
        };
        self::start(shell, start)
    }

    /// EDIT, under the Dummy's build: that build in Builds' editor (by its place among the game's files), whose Back
    /// comes back here, the visit going on: the session starts again on its choices, the dummy's build as the
    /// edit left it, the menu open. A build with no file (the screenshot pass's own) is refused.
    fn edit_dummys(&mut self, shell: &mut Shell) {
        let file = shell.practice.session.as_ref().filter(|s| s.from.is_none()).and_then(|s| s.builds.get(s.dummys)).and_then(|p| p.file);
        let Some(at) = file else { return shell.sound.play(Cue::Refused) };
        shell.sound.play(Cue::Pick);
        self.away = true;
        crate::screens::builds::open(shell, ScreenId::Practice, at);
    }

    /// Enter, a click or a tap on the menu's option at row `index`: the Dummy's build, Arena, Background and Music
    /// open their lists (Arena's and Background's grids of their pictures), EDIT opens the editor,
    /// another steps on by one.
    fn open_option(&mut self, shell: &mut Shell, index: i32) {
        let Some(session) = shell.practice.session.as_ref() else { return };
        let option = (index as usize).checked_sub(session.added).and_then(|at| session.rows.get(at)).copied();
        match option {
            Some(PracticeOption::DummysBuild) => {
                let names = session.builds.iter().map(|p| p.build.name.clone()).collect();
                let at = session.dummys;
                shell.pick("menu.rows.option", index, names, at);
            }
            Some(PracticeOption::EditDummy) => self.edit_dummys(shell),
            Some(option @ (PracticeOption::Arena | PracticeOption::Background)) => {
                let part = if option == PracticeOption::Arena { ARENA } else { BACKGROUND };
                let (game, labels, at) = (session.game.clone(), session.place_labels[part].clone(), session.place[part]);
                open_places(shell, "menu.rows.option", index, &game, part, &labels, at);
            }
            Some(PracticeOption::Music) => crate::music::open(shell, "menu.rows.option", index),
            _ => self.change_option(shell, index, Change::By(1)),
        }
    }

    /// Change the menu's option at row `index` (`change`: stepped, around from its last value to its first, or
    /// chosen from its list): it changes at once.
    fn change_option(&mut self, shell: &mut Shell, index: i32, change: Change) {
        let Some(mut session) = shell.practice.session.take() else { return };
        let option = (index as usize).checked_sub(session.added).and_then(|at| session.rows.get(at)).copied();
        // Music: the player's one pick, and the battle's music switched to it at once (the user: "also you should
        // be able to change the bgm in the practice menu").
        if option == Some(PracticeOption::Music) {
            match change {
                Change::By(by) => crate::music::step(shell, by),
                Change::To(at) => crate::music::choose(shell, at as i32),
            }
            crate::music::retheme(shell);
            session.music = crate::music::shown(shell);
            tell_rows(shell, &session, &[PracticeOption::Music]);
            shell.practice.session = Some(session);
            return;
        }
        // The Input delay: netplay's, saved, and the battle's player taking it at once (Play's lobby shows it too),
        // stepped within its range, not round it.
        if option == Some(PracticeOption::InputDelay) {
            let next = match change {
                Change::By(by) => (shell.settings.input_delay as i32 + by).clamp(0, MAX_INPUT_DELAY as i32) as u32,
                Change::To(at) => (at as u32).min(MAX_INPUT_DELAY),
            };
            if next != shell.settings.input_delay {
                shell.settings.input_delay = next;
                shell.save_settings();
                if let Some(b) = &mut shell.battle {
                    b.stage.player.set_input_delay(next);
                }
                shell.ui().global::<nettai_ui::PlayView>().set_input_delay(next as i32);
            }
            tell_rows(shell, &session, &[PracticeOption::InputDelay]);
            shell.practice.session = Some(session);
            return;
        }
        // The Dummy's build: one of the session's builds, kept for the visit, and the set started again with the
        // dummy on it, as for an arena.
        if option == Some(PracticeOption::DummysBuild) && !session.builds.is_empty() {
            let count = session.builds.len();
            let next = match change {
                Change::By(by) => (session.dummys as i32 + by).rem_euclid(count as i32) as usize,
                Change::To(at) => at.min(count - 1),
            };
            if next != session.dummys {
                session.dummys = next;
                let build = session.builds[next].build.clone();
                // (A session another screen started keeps its choices for its wait, not the visit's.)
                if session.from.is_none() {
                    shell.practice.visit.dummys_build = Some((session.game.clone(), build.name.clone()));
                }
                session.m.sides[1] = build.side;
                restart_set(shell, &mut session);
                shell.ui().global::<PracticeView>().set_session_dummys(build.name.as_str().into());
            }
            tell_rows(shell, &session, &[PracticeOption::DummysBuild]);
            shell.practice.session = Some(session);
            return;
        }
        if let Some(option) = option.filter(|o| values(*o, &session.places) > 0) {
            let next = match change {
                Change::By(by) => (value(option, session.opponent, session.options, session.inputs, session.place) as i32 + by).rem_euclid(values(option, &session.places) as i32) as usize,
                Change::To(at) => at.min(values(option, &session.places) - 1),
            };
            // (A session another screen started keeps its choices for its wait alone, not for the visit.)
            let keep = session.from.is_none();
            match option {
                // Arena: the set again from its start on the new arena, as RESTART starts it, the menu staying open.
                // Background: the battle drawn on it from now, nothing else changing.
                PracticeOption::Arena | PracticeOption::Background => {
                    let part = if option == PracticeOption::Arena { ARENA } else { BACKGROUND };
                    session.place[part] = next;
                    if keep {
                        shell.practice.visit.set_place(&session.game, session.place);
                    }
                    if part == ARENA {
                        restart_set(shell, &mut session);
                    } else {
                        redraw_on_background(shell, &mut session);
                    }
                }
                PracticeOption::Action => {
                    let in_fight = ACTIONS[next];
                    session.opponent = dummy(in_fight);
                    if keep {
                        shell.practice.visit.menu.action = Some(in_fight);
                    }
                }
                PracticeOption::YourHp | PracticeOption::DummysHp => {
                    let side = (option == PracticeOption::DummysHp) as usize;
                    session.options.refills[side] = next == 1;
                    if keep {
                        shell.practice.visit.menu.refills[side] = next == 1;
                    }
                }
                PracticeOption::CustomGauge => {
                    session.options.full_gauge = next == 1;
                    if keep {
                        shell.practice.visit.menu.full_gauge = next == 1;
                    }
                }
                PracticeOption::ChipDraw => {
                    session.options.chip_draw = if next == 1 { ChipDraw::FolderOrder } else { ChipDraw::Shuffled };
                    if keep {
                        shell.practice.visit.menu.folder_order = next == 1;
                    }
                }
                PracticeOption::InputDisplay => {
                    shell.practice.visit.menu.inputs = next == 1;
                    // (What each side holds is told from the next frame.)
                    session.inputs = next == 1;
                    session.held = [0; 2];
                    let ui = shell.ui();
                    let view = ui.global::<PracticeView>();
                    view.set_inputs(next == 1);
                    view.set_held_yours(0);
                    view.set_held_dummys(0);
                }
                // (Above.)
                PracticeOption::Music | PracticeOption::DummysBuild | PracticeOption::InputDelay => {}
                PracticeOption::Added | PracticeOption::Dummy | PracticeOption::Stage | PracticeOption::Options | PracticeOption::EditDummy => {}
            }
            // (At once, on the battle as it stands behind the menu.)
            if let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Practice) {
                b.stage.player.set_opponent(session.opponent);
                b.stage.player.set_training(session.options);
            }
            // (The row's own value: its stepper slides to it.)
            tell_rows(shell, &session, &[option]);
        }
        shell.practice.session = Some(session);
    }
}

impl Screen for Practice {
    fn show(&mut self, shell: &mut Shell) {
        // (While the game loads its bar is up, and the screen is shown again once it is ready.)
        if shell.ready().is_none() {
            return;
        }
        // (Shown again with its session on: the language changed under it, say.)
        if running(shell) {
            return;
        }
        // Come from elsewhere than the editor's PRACTICE (the screenshot pass), a visit starts on the defaults; back from
        // the editor, the visit goes on.
        shell.practice.visit.arrive();
        if let Err(error) = self.start_own(shell) {
            go_back(shell);
            shell.fail(error);
        }
    }

    fn hide(&mut self, shell: &mut Shell) {
        // (Its own session ends with it: back to the editor, or off to the menu's EDIT's.)
        if shell.practice.session.as_ref().is_some_and(|s| s.from.is_none()) {
            end(shell);
        }
        // (Leaving for another screen than the editor the menu's EDIT opens ends the visit.)
        if !std::mem::take(&mut self.away) {
            shell.practice.visit.leave();
        }
    }

    /// Back (before the battle has the keys: in the battle Esc opens the menu) goes back to the editor
    /// ([`go_back`]).
    fn back(&mut self, shell: &mut Shell) -> bool {
        go_back(shell);
        true
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        match id {
            "menu.resume" | "menu.restart" | "menu.exit" => match menu_button(id) {
                Some(MenuButton::Resume) => shell.close_menu(),
                Some(MenuButton::Restart) => {
                    shell.close_menu();
                    restart(shell);
                }
                // (A session another screen started has its EXIT, if it gave one: that screen hears it.)
                Some(MenuButton::Exit) => match &shell.practice.session {
                    Some(Session { from: None, .. }) => go_back(shell),
                    Some(Session { from: Some((to, tag)), exit: true, .. }) => {
                        let (to, tag) = (*to, *tag);
                        shell.tell(to, tag, EXIT);
                    }
                    _ => {}
                },
                _ => {}
            },
            "menu.rows.option" => self.open_option(shell, index),
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
        if id == "menu.rows.option" {
            self.change_option(shell, index, Change::By(by));
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, index: i32, choice: i32) {
        if id == "menu.rows.option" {
            self.change_option(shell, index, Change::To(choice.max(0) as usize));
        }
    }

    /// R or F5 in a practice battle: the set again from its start.
    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        matches!(key.code, KeyCode::KeyR | KeyCode::F5) && restart(shell)
    }

    /// The pad's north button: as R.
    fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        action == Action::Alternate && restart(shell)
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
            // Under the practice menu the battle is held, wherever the menu opens, and looks as the game's own
            // pause does: its music goes on and its scenery moves, no PAUSED banner (`battle/hold.rs`; the user:
            // "pause during custom screen vs not custom screen is so janky now"). A session of Practice's own opens
            // with the menu over it (the user: "when practice mode starts you should also bring up the menu
            // immediately"), before its first tick, as Esc there would: the battle steps through its opening under
            // it and is held as its custom screen begins, nothing of it changing until RESUME ("pause at start
            // doesn't seem to stop custom hp drain").
            BattleEvent::Started => {
                if shell.practice.session.as_mut().is_some_and(|s| std::mem::take(&mut s.menu_at_start)) {
                    shell.open_menu();
                }
            }
            BattleEvent::Watching(_) => {}
            // Practice has no end: a battle that ends is followed by another, a single battle of its own on a seed
            // of its own (its folders shuffled anew, a Random arena or background picked anew; the screenshot
            // pass's the next seed on, so it is the same each time).
            BattleEvent::Finished(_) => match shell.practice.session.take() {
                Some(mut session) => {
                    session.seed = if shell.still { session.seed.wrapping_add(1) } else { clock_seed() };
                    restart_set(shell, &mut session);
                    shell.practice.session = Some(session);
                }
                None => {
                    if let Some(b) = shell.battle.as_mut() {
                        b.restart();
                    }
                }
            },
            BattleEvent::Stopped(reason) => {
                end(shell);
                // (Practice has nothing to show without its battle: back where it came from.)
                if shell.screen() == ScreenId::Practice {
                    go_back(shell);
                }
                if let Some(error) = UserError::of_stop(reason) {
                    shell.fail(error);
                }
            }
        }
    }

    fn frame(&mut self, shell: &mut Shell, _now: Instant, _shown: bool) {
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
                live.set_second_pad(held.map(|h| if playing { h & !crate::input::SWITCHES } else { 0 }));
            }
        }
        // Its menu opens on RESUME, every time (the user: "when opening the pause menu in training, focus should
        // always default to the resume button").
        let open = shell.battle.as_ref().is_some_and(|b| b.owner == ScreenId::Practice && b.menu_open());
        if let Some(session) = &mut shell.practice.session
            && session.menu_open != open
        {
            session.menu_open = open;
            if open {
                shell.nav.focus("menu.resume", 0);
            }
        }
        // (The input delay as the setting has it now: Play's lobby changes it too, beside a waiting invite.)
        if open && let Some(session) = shell.practice.session.as_ref() {
            tell_rows(shell, session, &[PracticeOption::InputDelay]);
        }
        // A screen other than Practice and Builds ends the visit.
        shell.practice.visit.shown(shell.screen());
        // The places' pictures made since the last frame: in the grid open, and the rows of the ones chosen.
        if shell.practice.thumbs.poll()
            && let Some(session) = shell.practice.session.as_ref()
        {
            tell_rows(shell, session, &[PracticeOption::Arena, PracticeOption::Background]);
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
        // (Under the menu the battle takes no button, and the dummy's go nowhere: nothing is shown held.)
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
    #[test]
    fn the_menus_rows_have_their_values() {
        let content = nettai_match::testing::exe6_content();
        let (opponent, options, places, mut seen) = (Opponent::default(), Options::default(), Places::of(&content, "exe6"), 0);
        for option in ROWS {
            if values(option, &places) > 0 {
                seen += 1;
                assert!(value(option, opponent, options, false, [0; 2]) < values(option, &places), "{option:?}");
            }
            // (A session another screen starts has the battle's options at their normal values and the places
            // Random: the first.)
            if matches!(option, PracticeOption::Arena | PracticeOption::Background | PracticeOption::YourHp | PracticeOption::DummysHp | PracticeOption::CustomGauge | PracticeOption::ChipDraw) {
                assert_eq!(value(option, opponent, options, false, [0; 2]), 0, "{option:?}");
            }
        }
        assert_eq!(seen, 11, "Dummy's build, Action, Arena, Background, Music, Your HP, Dummy's HP, Custom gauge, Chip draw, Input display, Input delay");
        assert_eq!(values(PracticeOption::InputDelay, &places), MAX_INPUT_DELAY as usize + 1, "0 to netplay's most");
        assert_eq!(ACTIONS, [InFight::StandsStill, InFight::MovesAround, InFight::SecondPad]);
        assert_eq!(value(PracticeOption::Action, dummy(InFight::SecondPad), options, false, [0; 2]), 2);
        assert_eq!(value(PracticeOption::DummysHp, opponent, Options { refills: [false, true], ..options }, false, [0; 2]), 1);
        assert_eq!(value(PracticeOption::InputDisplay, opponent, options, true, [0; 2]), 1);
        assert_eq!(value(PracticeOption::Background, opponent, options, false, [3, 2]), 2);
        // (The arenas and the backgrounds after Random.)
        assert_eq!(values(PracticeOption::Arena, &places), 1 + places.arenas.len());
        assert_eq!(values(PracticeOption::Background, &places), 1 + nettai_match::ids::pool_backgrounds(&content).len(), "every game's");
    }

    /// The Dummy's build and its EDIT are the DUMMY group's first, where the session offers builds (Practice's
    /// own), and the Music is not beside a waiting invite; the dummy picks the first chip it can on its custom
    /// screen whatever its Action.
    #[test]
    fn the_menus_rows_go_with_the_session() {
        assert_eq!(shown_rows(false, true), ROWS);
        assert_eq!(shown_rows(false, true)[..4], [PracticeOption::Dummy, PracticeOption::DummysBuild, PracticeOption::EditDummy, PracticeOption::Action]);
        // Beside a waiting invite the usual menu, but for its Music and EDIT.
        let beside = shown_rows(true, true);
        let usual: Vec<PracticeOption> = shown_rows(false, true).into_iter().filter(|o| !matches!(o, PracticeOption::Music | PracticeOption::EditDummy)).collect();
        assert_eq!(beside, usual, "beside a waiting invite");
        assert!(beside.contains(&PracticeOption::DummysBuild));
        assert_eq!(shown_rows(false, false).len(), ROWS.len() - 2, "no builds offered: no Dummy's build, no EDIT");
        for in_fight in ACTIONS {
            assert_eq!(dummy(in_fight).on_custom_screen, OnCustomScreen::PicksFirst, "{in_fight:?}");
        }
        assert_eq!(dummy(InFight::StandsStill), Opponent::default());
    }

    /// Each game's places are its link battle stages and the background pool, every game's backgrounds, after
    /// Random: a place chosen is every round's (and the set's places are those) and is found again from what the
    /// round names (a background named by its name in the game's pack too), and Random names nothing, a Random
    /// background drawn from the pool. An arena is called by its place, a background by its game's name and its
    /// own in the language (each has one).
    #[test]
    fn the_places_are_the_games_after_random() {
        use nettai_match::testing::{exe4_content, exe5_content, exe6_content};
        crate::shell::Shell::read_game_names(&["exe4", "exe5", "exe6"], "en");
        for (game, content) in [("exe6", exe6_content()), ("exe5", exe5_content()), ("exe4", exe4_content())] {
            let places = Places::of(&content, game);
            let (mut arenas, mut stages) = (places.arenas.concat(), nettai_match::link_battle_stages(&content, game));
            arenas.sort();
            stages.sort();
            assert_eq!(arenas, stages, "{game}");
            assert!(places.len(ARENA) > 1 && places.len(BACKGROUND) > 1, "{game}");
            assert_eq!(places.round([0, 0]), RoundSettings::default(), "{game}: Random names nothing");
            let last = [places.len(ARENA) - 1, places.len(BACKGROUND) - 1];
            let mut m = nettai_match::pick::live(&content, game, 7, None).unwrap();
            places.put(&mut m, last);
            let (arena, background) = (places.arenas.last().unwrap()[0], places.backgrounds.last().unwrap().clone());
            assert!(m.rounds.iter().all(|r| *r == RoundSettings { stage: Some(arena), background: Some(background.clone()) }), "{game}");
            let fought = m.places(&content, 7).unwrap();
            assert!(fought.iter().all(|p| p.stage == arena && nettai_match::ids::background_name(&content, p.background) == Some(background.as_str())), "{game}: every round there");
            assert_eq!(places.chosen(&content, &places.round(last)), last, "{game}");
            assert_eq!(places.key(&content, BACKGROUND, last[BACKGROUND]), Some(background.as_str()), "{game}");
            assert_eq!(places.chosen(&content, &places.round([0, last[BACKGROUND]])), [0, last[BACKGROUND]], "{game}: Random names nothing");
            let [arenas, backgrounds] = places.labels(&content);
            let n = places.arenas.len();
            assert_eq!((arenas[0].clone(), arenas[n - 1].clone()), (format!("1 / {n}"), format!("{n} / {n}")), "{game}: each by its place");
            let key = |at: usize| nettai_match::ids::local(&content.defs.stage(places.arenas[at][0]).key);
            let numbers: Vec<u32> = (0..n).map(|at| key(at).trim_start_matches("netbattle-").parse().unwrap()).collect();
            assert!(numbers[0] == 1 && numbers.is_sorted(), "{game}: in their numbers' order");
            for (name, key) in backgrounds.iter().zip(&places.backgrounds) {
                assert!(!name.is_empty() && name != key && !name.contains(" / ") && !name.contains('-'), "{game}: {key} has a name");
            }
            // The pool: BN4's first, BN6's last, every game's in any game's practice, named by its game.
            assert_eq!(places.backgrounds.len(), 27 + 22 + 21, "{game}");
            assert_eq!((backgrounds[0].as_str(), backgrounds[27 + 22 + 20].as_str()), ("BN4 ACDC Area", "BN6 Undernet"), "{game}");
            assert!(backgrounds.contains(&"BN5 NebulaGray".to_string()) && backgrounds.contains(&"BN4 Duo".to_string()), "{game}");
            let own = |b: &str| !b.contains(':');
            assert_eq!(places.backgrounds.iter().filter(|b| own(b)).count(), nettai_match::ids::backgrounds(&content, game).len(), "{game}: its own by their names");
            // (A background named by its name in the game's pack is found.)
            let named = nettai_match::ids::backgrounds(&content, game)[0].to_string();
            let at = places.find(&content, BACKGROUND, Some(&named));
            assert_eq!(places.backgrounds[at - 1], named, "{game}");
            // Random draws from the pool.
            let mut m = nettai_match::pick::live(&content, game, 7, None).unwrap();
            places.put(&mut m, [0, 0]);
            assert_eq!(m.backgrounds, nettai_match::BackgroundPick::Pool, "{game}");
        }
    }

    /// Each arena is listed once, as the first of its records in the list's order: EXE6's 96 link battle stages
    /// are 47 arenas (EXE5's 94 47, EXE4's 96 48), each a group of records the same but for the order their actor
    /// lists place in (and EXE6's netbattle-95 and 96, EXE5's 93 and 94, their effects), every record not its
    /// group's first in a group before it. A round that names a later record is on its arena, which plays its
    /// first.
    #[test]
    fn each_arena_is_listed_once() {
        use nettai_match::testing::{exe4_content, exe5_content, exe6_content};
        for (game, content, stages, arenas) in [("exe6", exe6_content(), 96, 47), ("exe5", exe5_content(), 94, 47), ("exe4", exe4_content(), 96, 48)] {
            let places = Places::of(&content, game);
            let all = nettai_match::link_battle_stages(&content, game);
            assert_eq!((all.len(), places.arenas.len()), (stages, arenas), "{game}");
            let key = |s: StageHandle| natural(nettai_match::ids::local(&content.defs.stage(s).key));
            for (at, records) in places.arenas.iter().enumerate() {
                // (Its records the same arena, the first the earliest; no earlier arena the same.)
                assert!(records.iter().all(|&s| same_arena(content.stage(records[0]), content.stage(s)) && key(records[0]) <= key(s)), "{game}");
                assert!(places.arenas[..at].iter().all(|other| !same_arena(content.stage(other[0]), content.stage(records[0]))), "{game}");
                for &s in records {
                    let round = RoundSettings { stage: Some(s), background: None };
                    assert_eq!(places.chosen(&content, &round), [at + 1, 0], "{game}");
                    assert_eq!(places.round([at + 1, 0]).stage, Some(records[0]), "{game}");
                }
            }
        }
        // (EXE6's records the same to the byte: one arena.)
        let content = exe6_content();
        let places = Places::of(&content, "exe6");
        let stage = |name: &str| nettai_match::link_stage(&content, "exe6", name).unwrap();
        for (first, later) in [("netbattle-1", "netbattle-89"), ("netbattle-2", "netbattle-90")] {
            let at = places.find(&content, ARENA, Some(later));
            assert!(at > 0 && places.arenas[at - 1].contains(&stage(first)), "{later}");
        }
        assert_eq!(places.arenas[0][..2], [stage("netbattle-1"), stage("netbattle-2")], "side 0's navi first, then side 1's");
    }

    /// A background chosen leaves the arenas as they were: with Arena Random the seed picks each round's arena
    /// whether the round names its background or not (the menu's Background keeps the arena).
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

    /// A visit starts on the defaults (the dummy on your build, Random, Random, the game's own music, the menu's
    /// options normal) and keeps what the player sets through its sessions and Builds' editor through EDIT
    /// DUMMY'S BUILD; another screen (Play, back) ends it, and the next visit starts on the defaults again. A place
    /// and a build are the game's: another game's are the defaults.
    #[test]
    fn a_visit_starts_on_the_defaults() {
        use crate::music::Music;
        let content = nettai_match::testing::exe6_content();
        let places = Places::of(&content, "exe6");
        let theme = Music::Theme { game: "exe4".into(), song: "tournament-battle".into() };
        // (the editor's PRACTICE: your build its chosen one, the rest the defaults; Practice shown keeps it.)
        let mut visit = Visit::new("exe6", "Swords", None);
        assert!(!visit.arrive(), "the editor's PRACTICE started it");
        assert_eq!((visit.build("exe6", 0), visit.build("exe6", 1), visit.build("exe5", 0)), (Some("Swords"), None, None));
        assert_eq!((visit.place("exe6", &places), &visit.music), ([0, 0], &Music::GameOwn), "Random, Random, the game's own");
        assert_eq!((&visit.menu, &visit.dummys_build), (&MenuChoices::default(), &None), "the menu's defaults, the dummy on your build");
        visit.leave();
        assert!(visit.arrive(), "from elsewhere, a visit starts");
        assert_eq!(visit.build("exe6", 0), None, "Play's remembered build");
        let menu = MenuChoices { action: Some(InFight::SecondPad), refills: [true, false], full_gauge: true, folder_order: true, inputs: true };
        visit.set_place("exe6", [3, 5]);
        visit.music = theme.clone();
        visit.menu = menu.clone();
        visit.dummys_build = Some(("exe6".into(), "Swords".into()));
        // (A session; Builds through the menu's EDIT and back; a session again.)
        for screen in [ScreenId::Practice, ScreenId::Builds, ScreenId::Practice] {
            visit.shown(screen);
            assert!(!visit.arrive(), "{screen:?}: the same visit");
            assert_eq!((visit.place("exe6", &places), &visit.music), ([3, 5], &theme), "{screen:?}");
            assert_eq!((&visit.menu, visit.dummys_build.is_some()), (&menu, true), "{screen:?}");
        }
        assert_eq!(visit.place("exe5", &places), [0, 0], "another game's: Random");
        assert_eq!(visit.place("exe6", &Places::default()), [0, 0], "a place the list hasn't: Random");
        for elsewhere in [ScreenId::Menu, ScreenId::Play] {
            visit.shown(elsewhere);
            assert!(visit.arrive(), "{elsewhere:?}: a new visit");
            assert_eq!((visit.place("exe6", &places), &visit.music), ([0, 0], &Music::GameOwn), "{elsewhere:?}: the defaults again");
            assert_eq!((&visit.menu, &visit.dummys_build), (&MenuChoices::default(), &None), "{elsewhere:?}: the menu's too");
            visit.set_place("exe6", [1, 1]);
            visit.music = theme.clone();
        }
        // (Left from Practice itself, for another screen than the editor: the same.)
        visit.leave();
        assert_eq!(visit, Visit::default());
        assert!(visit.arrive());
    }

    /// The practice menu's buttons are RESUME, RESTART and EXIT, and nothing else (no CHANGE BUILDS, no QUIT TO
    /// MENU): practice.slint's `PracticeMenu` has a control of each, this screen hears each, and EXIT (back to
    /// Play) is shown in a session of Practice's own alone. There is no entry: practice.slint has no screen.
    #[test]
    fn the_menus_buttons() {
        let slint = include_str!("../../../nettai-ui/ui/screens/practice.slint");
        let menu = &slint[slint.find("export component PracticeMenu").unwrap()..];
        let menu = &menu[..menu.find("\n}\n").unwrap()];
        let navs: Vec<&str> = menu.match_indices("nav: \"menu.").map(|(at, _)| &menu[at + 6..][..menu[at + 6..].find('"').unwrap()]).collect();
        let buttons: Vec<&str> = navs.iter().copied().filter(|n| !n.starts_with("menu.rows")).collect();
        assert_eq!(buttons, MENU_BUTTONS.map(|(name, _)| name), "RESUME, RESTART, EXIT");
        assert_eq!(MENU_BUTTONS.map(|(name, _)| menu_button(name)), [Some(MenuButton::Resume), Some(MenuButton::Restart), Some(MenuButton::Exit)]);
        for gone in ["menu.change", "menu.quit"] {
            assert_eq!(menu_button(gone), None);
            assert!(!slint.contains(gone), "{gone}");
        }
        let exit = &menu[..menu.find("nav: \"menu.exit\"").unwrap()];
        let exit = &exit[exit.rfind("Button {").unwrap()..];
        assert!(exit.contains("visible: PracticeView.leave;") && exit.contains("enabled: PracticeView.leave;"), "EXIT in Practice's own sessions alone");
        assert!(menu.contains("Caps.of(@tr(\"Exit\"))"));
        for gone in ["PracticeScreen", "START PRACTICE", "nav: \"start\"", "nav: \"dummys\""] {
            assert!(!slint.contains(gone), "{gone}");
        }
    }
}
