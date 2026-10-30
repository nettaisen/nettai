//! Play (docs/design/app-ui.md §6: "Play", "In an online match", "After a match"): the player's builds, and
//! online play.
//!
//! The player's builds of the selected game are here (`list`: + NEW BUILD over them, whose dialog also starts one
//! from a save file; the chosen one's overview beside them with EDIT, which opens it in Builds' editor, and ⋯, its
//! menu of DUPLICATE, RENAME and DELETE). The player picks one and practices with it (the editor's PRACTICE), or
//! makes a link with it: HOST makes an invite (a room of the signaling server, whose code the server picks), JOIN
//! joins one by the code typed, and DIRECT CONNECT's dialog hosts directly (this machine listening on a UDP port)
//! or joins a direct host by its address. From the moment an invite is out (or this machine hosts) Play shows the
//! lobby: while the host waits for the other player, the other player's place has the invite's code, a practice
//! session runs in the player's own (`crate::screens::practice`'s, with this screen's rows in its menu), and the
//! match's settings are already the host's to set (`Play::waiting`, which the lobby starts on). Once the link is
//! open the two are in the lobby proper (`session`: one `Lobby` a link, for as long as they stay together): the
//! match's shared settings, the host's to change, each one's ready mark, the connection's quality. Both ready,
//! the match is agreed and made ready behind the shell's two cards, each side saying how far it is; both loaded,
//! it starts. When its set is over, nettai's
//! result shows over the picture (`MatchResult`) until the player goes on; when it is, or one of them leaves the
//! match, both are back in the lobby on the same link, neither waiting on the other's result.
//!
//! What goes wrong is said as its typed kind's sentence (`crate::errors`): a field's problem in the space
//! reserved beside it, a failure that ends what the player was doing as the shell's dialog. A build's name is
//! this machine's alone: nothing here sends it.

mod list;
mod session;
// (The screenshot pass's second player is the `tour` feature's; without it Play hears nothing of it.)
#[cfg(any(test, feature = "tour"))]
#[cfg_attr(not(feature = "tour"), allow(dead_code, unused_imports))]
pub mod tour;
#[cfg(not(any(test, feature = "tour")))]
mod tour {
    //! A build without the screenshot pass: nothing of it is there (`tour.rs` is the pass's).
    use super::session::Net;
    use crate::shell::Shell;
    use nettai_battle::Content;
    use nettai_match::Side;
    use std::sync::Arc;
    use web_time::Instant;

    pub fn net() -> Option<Net> {
        None
    }

    pub fn builds(_still: bool, _content: &Arc<Content>, _game: &str) -> Option<Vec<(String, Side)>> {
        None
    }

    pub fn entropy() -> Option<u64> {
        None
    }

    pub fn steady_ping(_still: bool) -> Option<(i32, i32)> {
        None
    }

    pub fn frame(_shell: &mut Shell, _now: Instant) {}

    pub fn heard(_now: Instant, _datagrams: usize, _down: bool) {}
}

use crate::battle::{BattleEvent, BattleStart, Play as Played};
use crate::builds::{self, overview, shelf, words};
use crate::rulesets::page::{self as rules_page, Outcome, Page as RulesPage};
use crate::rulesets::{self, Pickable};
use list::Naming;
use crate::errors::UserError;
use crate::games::Names;
use slint::winit_030::winit::keyboard::KeyCode;
use crate::input::{Action, KeyPress, Scroll, button_index, key_cap, pad_cap, pad_face};
use crate::nav::Key;
use crate::screens::{Screen, ScreenId, practice, settings};
use crate::settings::MAX_INPUT_DELAY;
use crate::shell::{Dialog, Shell};
use crate::sound::Cue;
use nettai_gamepad::Button;
use nettai_battle::{BattleResult, ContentHash};
use nettai_battle::input::keys;
use nettai_frontend::driver::{Driver, StopReason};
use nettai_frontend::lobby::{Agreement, Settings};
use nettai_frontend::netplay::{NetOptions, NetPlayer};
use nettai_frontend::player::FRAME_RATE;
use nettai_frontend::replay::Recorder;
use nettai_match::Side;
use nettai_match::check::Problem;
use nettai_match::ruleset::Rulesets;
use nettai_rtc::signal::{ALPHABET, CODE_LENGTH};
use nettai_ui::{BattleKind, BuildSheet, Hint, LobbyNote, MatchOutcome, MatchPage, PadButton, PlayBuild, PlayControl, PlayPart, PlayView, VersusCard};
use session::{Net, Session, Stands, Way};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;

/// How long a match stays on screen once its set is over, before the player is back in the lobby, where nettai
/// shows no result of its own (a set that ended some other way than won, lost or drawn). The game shows the
/// result itself, in its own way, before the set is over for the engine: the picture has faded out by then, so
/// this is a beat, not the time the result is read in.
const RESULT_SHOWN: Duration = Duration::from_millis(500);

/// How long nettai's result stays, once its set is over, before the player goes on to the lobby by themselves.
const RESULT_STAYS: Duration = Duration::from_secs(6);

/// The result's numbers tick with the game's cursor sound, at most this often (the time's hundredths tick
/// every frame).
const TICK_SPACING: Duration = Duration::from_millis(45);

/// The round wins a match's set may be first to: 1 (a single battle) to `nettai_match::MAX_FIRST_TO`, 2 (the
/// original's triple battle) unless the player last played another.
const FIRST_TO: std::ops::RangeInclusive<u8> = 1..=nettai_match::MAX_FIRST_TO;

/// The tag this screen hears of its rows in the practice menu by (`Screen::answered`).
const WAIT_ROWS: &str = "play.wait";

/// The tag of the question Esc in the lobby asks (`Screen::answered`: 1 is LEAVE).
const LEAVE_LOBBY: &str = "play.leave";

/// What the settings remember for Play: the match's settings as this player last set them hosting (the user: "the
/// host should remember their last match settings"; the ruleset is the one their build is made for, §2.73), and
/// the build last chosen, by game.
const REMEMBERED_FIRST_TO: &str = "play.first-to";
const REMEMBERED_OPEN_SETUPS: &str = "play.open-setups";
/// The Custom timer's seconds, or "unlimited".
const REMEMBERED_CUSTOM_TIME: &str = "play.custom-time";
const UNLIMITED: &str = "unlimited";

fn remembered_build(game: &str) -> String {
    format!("play.build.{game}")
}

/// The name of `game`'s build Play last had chosen, if it remembers one (what Practice opens on where it is
/// shown with none from Play).
pub(crate) fn remembered(shell: &Shell, game: &str) -> Option<String> {
    shell.settings.remembered.get(&remembered_build(game)).cloned()
}

/// `game`'s build `from` was renamed `to` (in the editor's header): Play remembers it, and the visit to Practice
/// keeps it, by its new name.
pub(crate) fn follow_rename(shell: &mut Shell, game: &str, from: &str, to: &str) {
    let key = remembered_build(game);
    if shell.settings.remembered.get(&key).is_some_and(|name| name == from) {
        shell.settings.remembered.insert(key, to.to_string());
        shell.save_settings();
    }
    shell.practice.visit.renamed(game, from, to);
}

/// The selected game's builds (`builds::shelf`: its builds folder's files), each checked under the ruleset it is
/// made for (`rulesets::made_for`; a lobby checks them again under the host's: `Play::check_builds`), as Play lists
/// them and Practice's Dummy's build offers them. The screenshot pass lists builds of its own, which
/// have no file: its pictures don't depend on what the player keeps.
pub(crate) fn shelved(shell: &Shell, kit: &builds::Kit) -> Vec<shelf::Shelved> {
    let Some(made) = tour::builds(shell.still, kit.content(), &kit.game) else { return shelf::load(shell, kit) };
    let ui = shell.ui();
    let graphics = kit.ready.graphics(shell.i18n.code);
    let names = Names::of(kit.content(), &graphics);
    let words = words::Words { ui: &ui, kit, names: &names, i18n: &shell.i18n };
    let own = shelf::Own::of(kit);
    made.into_iter()
        .map(|(name, side)| {
            let listed = builds::store::Listed { path: Default::default(), name: name.clone(), text: String::new() };
            shelf::shelved(&words, listed, Some(builds::store::Build::new(kit.content(), &name, side)), own.as_ref())
        })
        .collect()
}

/// The settings a lobby of `game` opens with: a set first to `wins` round wins, each place picked from the match's
/// seed, its background from the background pool's backgrounds of the game alone (the user: "in actual pvp mode,
/// don't choose from the entire background pool, only choose from the background pool for that game"; docs/app.md
/// §1, "The battle's background"), shared by both players as the round's settings are, under `ruleset` (the one the player's build is made for).
pub(crate) fn settings(game: &str, wins: u8, ruleset: nettai_match::ruleset::Ruleset) -> Settings {
    Settings { backgrounds: nettai_match::BackgroundPick::GamePool, ..Settings::new(game, wins, ruleset) }
}

/// A direct connection's dialog.
#[derive(Default)]
struct Direct {
    host_address: String,
    address: String,
    host_problem: Option<UserError>,
    join_problem: Option<UserError>,
}

/// What the match's loading says across the shell's gate: how far this side is, how far the other, and the
/// match's driver once the lobby says it starts (the battle begins when it is there).
#[derive(Default)]
struct Gate {
    mine: Cell<f32>,
    theirs: Cell<f32>,
    driver: RefCell<Option<Box<dyn Driver>>>,
    go: Cell<bool>,
}

/// Where Play is.
enum Stage {
    /// No link.
    Idle,
    /// A link is being made (the server reached, a room joined, a host dialed): the space beside the control
    /// that asked says so.
    Meeting,
    /// An invite is out, or this machine hosts: the other player hasn't come, and a practice battle runs.
    Waiting,
    /// The two players are in the lobby.
    Lobby,
    /// A match is agreed and is being made ready behind the two cards.
    Loading { seed: u32, gate: Rc<Gate> },
    /// The match is on: how its set ended, once it has.
    Match { result: Option<Ended> },
}

/// What is done about the other player's content, where it isn't this side's (docs/design/content-deployment.md
/// §5, "Netplay"). Where its hash isn't this side's, both play the newer revision, the side without it downloading
/// it: those states are of the other's content's hash. Where its background pool isn't this side's, both take
/// every game's newest content: those are of the other's pool's identity.
enum Switch {
    /// Their revision is looked for: their game's index read again; done once it has been.
    Finding(ContentHash, Rc<Cell<bool>>),
    /// Theirs is newer: it is being downloaded (what came of it, once it has), then the game loaded again on it.
    Fetching(ContentHash, Rc<RefCell<Option<Result<(), String>>>>),
    Reloading(ContentHash),
    /// Theirs is older: this side waits for them to take its own.
    Waiting(ContentHash),
    /// Every game's newest content is being taken (what came of it, once it has), the catalog and the indexes read
    /// again.
    Updating(u64, Rc<RefCell<Option<crate::contents::Updated>>>),
    /// Then the game is loaded again on it, with the pool as it is now; what couldn't be had is said once it is.
    Rebuilding(u64, Option<String>),
}

/// A match's set is over: since when, and whether nettai's result is over the picture (`MatchResult`).
#[derive(Clone, Copy)]
struct Ended {
    since: Instant,
    shown: bool,
}

/// What the result's sounds have played: the numbers' ticks the UI had counted, when the last one played, and
/// the confirm sound as the last number came to its value.
#[derive(Default)]
struct ResultHeard {
    ticks: i32,
    at: Option<Instant>,
    done: bool,
}

/// What the lobby shows that changes by itself: told to the UI when it does.
#[derive(Clone, PartialEq, Default)]
struct Live {
    their_present: bool,
    their_name: String,
    ready: bool,
    their_ready: bool,
    first_to: i32,
    /// The match's ruleset, and whether this player is the host, who chooses it.
    ruleset: Option<nettai_match::ruleset::Ruleset>,
    host: bool,
    /// What is wrong with this player's build under the settings (the line says the first and how many more).
    problems: Vec<nettai_match::check::Problem>,
    open: bool,
    /// The custom screen's time limit in seconds, none for none.
    custom_time: Option<u16>,
    bars: i32,
    ping: i32,
    note: Option<LobbyNote>,
    seconds: i32,
}

pub struct Play {
    /// The builds listed, and the game they are of.
    builds: Vec<shelf::Shelved>,
    builds_of: Option<String>,
    chosen: usize,
    /// The dialog for a build's name open (`list`), and what the last import made, where that is worth saying
    /// (quietly, in the space beside JOIN).
    naming: Option<Naming>,
    notice: String,
    /// The code typed (its characters alone), and what the reserved line under the row says.
    code: String,
    problem: Option<UserError>,
    direct: Option<Direct>,
    /// The code's six boxes are open (a pad can't type): the character each is on, by its place in the
    /// alphabet codes are written in.
    boxes: Option<[usize; CODE_LENGTH]>,
    /// The lobby's list of builds is open.
    changing: bool,
    /// The ruleset's page is open over the lobby.
    rules: Option<RulesPage>,
    session: Option<Session>,
    /// What is done about the other player's content, where it isn't this side's.
    switch: Option<Switch>,
    /// The match's settings the host holds while it waits for the other player (the user: "the host should be
    /// able to change settings before the joiner joins"): the lobby starts on them, and the joiner takes them.
    waiting: Option<Settings>,
    /// This player's build checked under the settings it holds while it waits (made again when they change): the
    /// settings, the game's rulesets, the problems.
    wait_checked: Option<(Settings, Arc<Rulesets>, Vec<Problem>)>,
    stage: Stage,
    /// The match agreed last: what the match's panels show.
    agreed: Option<Agreement>,
    /// Why the match before ended, for the lobby's line.
    ended: Option<UserError>,
    live: Live,
    page: MatchPage,
    /// The match menu's Controls page changing a binding.
    binding: settings::Binding,
    peek: bool,
    /// Counted for the UI: each player's turns to ready, the copies.
    ready_seq: i32,
    their_ready_seq: i32,
    copied_seq: i32,
    /// The build the overview beside the list was made of (its name and side), and in which language.
    sheet_of: Option<(String, Option<Side>, &'static str)>,
    /// The screen is shown anew (from another): the focus starts on the chosen build.
    fresh: bool,
    /// The result's sounds.
    heard: ResultHeard,
}

impl Default for Play {
    fn default() -> Play {
        Play {
            builds: Vec::new(),
            builds_of: None,
            chosen: 0,
            naming: None,
            notice: String::new(),
            code: String::new(),
            problem: None,
            direct: None,
            boxes: None,
            changing: false,
            rules: None,
            session: None,
            switch: None,
            waiting: None,
            wait_checked: None,
            stage: Stage::Idle,
            agreed: None,
            ended: None,
            live: Live::default(),
            page: MatchPage::Menu,
            binding: Default::default(),
            peek: false,
            ready_seq: 0,
            their_ready_seq: 0,
            copied_seq: 0,
            sheet_of: None,
            fresh: true,
            heard: ResultHeard::default(),
        }
    }
}

/// A box of the code's six, by its place: its control's name.
fn code_box(at: usize) -> String {
    format!("boxes.{at}")
}

/// What the direct connection's address field keeps of what was typed: the text without spaces or control
/// characters, at most 80 characters.
fn address_typed(text: &str) -> String {
    text.chars().filter(|c| !c.is_control() && !c.is_whitespace()).take(80).collect()
}

/// The direct connection's address, as its CONNECT takes it: a host or an IP and a port, an IPv6 one in
/// brackets (`[::1]:47474`). One without a port is no address, whatever it resolves to.
fn direct_address(text: &str) -> Result<String, UserError> {
    let text = text.trim();
    let (host, port) = match text.strip_prefix('[') {
        Some(rest) => rest.split_once("]:").ok_or(UserError::BadAddress)?,
        None => text.rsplit_once(':').filter(|(host, _)| !host.contains(':')).ok_or(UserError::BadAddress)?,
    };
    match !host.is_empty() && port.parse::<u16>().is_ok_and(|p| p > 0) {
        true => Ok(text.to_string()),
        false => Err(UserError::BadAddress),
    }
}

/// A code as it is kept for its room, and for the pad's six boxes: its letters in capitals and its digits, at
/// most a code's six; whatever else was typed (spaces, hyphens) left out.
fn code_typed(text: &str) -> String {
    text.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_uppercase()).take(CODE_LENGTH).collect()
}

/// A code as it is shown: in threes.
fn code_shown(code: &str) -> String {
    match code.char_indices().nth(3) {
        Some((at, _)) => format!("{}-{}", &code[..at], &code[at..]),
        None => code.to_string(),
    }
}

impl Play {
    /// The part shown: the builds, or the lobby (the wait for the other player its start).
    fn part(&self) -> PlayPart {
        match self.stage {
            Stage::Idle | Stage::Meeting => PlayPart::Builds,
            Stage::Waiting | Stage::Lobby | Stage::Loading { .. } | Stage::Match { .. } => PlayPart::Lobby,
        }
    }

    /// The part's name for the focus's memory, and its main control.
    fn view(&self) -> (&'static str, &'static str) {
        match (self.part(), &self.stage) {
            (PlayPart::Builds, _) => ("play", "builds"),
            (_, Stage::Waiting) => ("play.waiting", "wait.copy"),
            _ => ("play.lobby", "lobby.ready"),
        }
    }

    fn build(&self) -> Option<&shelf::Shelved> {
        self.builds.get(self.chosen)
    }

    /// Where links are made: as the environment says (the screenshot pass has a server of its own, in the
    /// process).
    fn net() -> Net {
        tour::net().unwrap_or_else(Net::of_environment)
    }

    // ---- The builds --------------------------------------------------------------------------------------

    /// Read the selected game's builds ([`shelved`]): the one chosen before chosen again, else the one remembered
    /// (the one chosen before, renamed in the editor: [`follow_rename`]).
    fn read_builds(&mut self, shell: &Shell, game: &str) {
        let before = self.build().map(|b| b.name().to_string());
        let Some(kit) = builds::kit(shell) else {
            self.builds = Vec::new();
            return;
        };
        self.builds = shelved(shell, &kit);
        self.builds_of = Some(game.to_string());
        self.chosen = [before, remembered(shell, game)].into_iter().flatten().find_map(|name| self.builds.iter().position(|b| b.name() == name)).unwrap_or(0);
    }

    /// The builds checked again under `under` (a lobby's ruleset: the host's), and the rows shown with them. Their
    /// files aren't read again.
    fn check_builds(&mut self, shell: &Shell, under: Option<shelf::Under>) {
        let Some(kit) = builds::kit(shell) else { return };
        let ui = shell.ui();
        let graphics = kit.ready.graphics(shell.i18n.code);
        let names = Names::of(kit.content(), &graphics);
        let words = words::Words { ui: &ui, kit: &kit, names: &names, i18n: &shell.i18n };
        for build in &mut self.builds {
            build.check_under(&words, under);
        }
        let view = ui.global::<PlayView>();
        view.set_builds(ModelRc::new(VecModel::from(self.builds.iter().map(|b| b.row.clone()).collect::<Vec<_>>())));
        view.set_build(self.build().map(|b| b.row.clone()).unwrap_or_default());
    }

    /// The chosen build's overview, beside the list (made again only when the build or the language shown
    /// changed: the builds are read each time the screen is shown), and whether it reads as one.
    fn show_sheet(&mut self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_readable(self.build().is_some_and(|b| b.build.is_some()));
        let key = self.build().map(|b| (b.name().to_string(), b.side().cloned(), shell.i18n.code));
        if key == self.sheet_of {
            return;
        }
        self.sheet_of = key;
        let sheet = match (builds::kit(shell), self.build().and_then(|b| Some((b.name(), b.side()?)))) {
            (Some(kit), Some((name, side))) => overview::sheet(shell, &kit, "", name, side),
            _ => BuildSheet::default(),
        };
        view.set_sheet(sheet);
    }

    /// This player's card in the lobby: the build they bring, as the match plays it (while the host waits for the
    /// other player, the chosen build).
    fn show_card(&mut self, shell: &Shell) {
        let side = self.session.as_mut().and_then(Session::lobby).map(|lobby| lobby.side().clone()).or_else(|| self.side(shell, false).ok());
        let (Some(side), Some((_, ready)), Some(build)) = (side, shell.ready(), self.build()) else { return };
        let graphics = ready.graphics(shell.i18n.code);
        let names = Names::of(ready.content(), &graphics);
        let card = PlayBuild { name: build.name().into(), navi: names.navi(side.navi(ready.content())).into() };
        shell.ui().global::<PlayView>().set_card(card);
    }

    /// Choose a build: the one a link brings, and the one the overview shows and its buttons act on.
    fn choose_build(&mut self, shell: &mut Shell, index: usize) {
        if index >= self.builds.len() || index == self.chosen {
            return;
        }
        self.chosen = index;
        self.remember_chosen(shell);
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_chosen(index as i32);
        view.set_build(self.builds[index].row.clone());
        self.show_sheet(shell);
        // (What was wrong with the build before is no more.)
        if matches!(self.problem, Some(UserError::BuildUnusable { .. })) {
            self.problem = None;
            view.set_message(self.notice.as_str().into());
            view.set_message_quiet(true);
        }
    }

    /// The chosen build is remembered for its game: the one chosen when Play is next shown.
    fn remember_chosen(&self, shell: &mut Shell) {
        if let (Some(game), Some(build)) = (&self.builds_of, self.build()) {
            shell.settings.remembered.insert(remembered_build(game), build.name().to_string());
            shell.save_settings();
        }
    }

    /// The chosen build's side, if it can be played; else the space beside the control says it can't (or that
    /// there is no build), and nothing is made. It is played under the ruleset the list checked it under (the one
    /// it is made for, what a lobby they host starts on); `joining` another player's lobby, by the game's rules
    /// alone, as that lobby's ruleset is the host's and the lobby checks the build under it.
    fn side(&self, shell: &Shell, joining: bool) -> Result<Side, UserError> {
        match self.build() {
            None => Err(UserError::NoBuild),
            Some(build) if build.usable() || (joining && build.playable()) => Ok(build.side().cloned().expect("a usable build has its side")),
            // (What is wrong, in the player's words: the editor's for each kind. The game's rules' problems come
            // before the ruleset's.)
            Some(build) => {
                let reason = builds::kit(shell).and_then(|kit| words::summary(shell, &kit, &build.problems));
                Err(UserError::BuildUnusable { reason: reason.filter(|r| !r.is_empty()) })
            }
        }
    }

    /// Whether the link is made by joining another player's (by a code or an address).
    fn joining(&self) -> bool {
        self.session.as_ref().is_some_and(|s| matches!(s.way, Way::Code(_) | Way::Address(_)))
    }

    // ---- Making a link -----------------------------------------------------------------------------------

    /// Make a link `way`, with the chosen build. What keeps it from being made is the control's problem.
    fn connect(&mut self, shell: &mut Shell, way: Way) -> Result<(), UserError> {
        self.drop_session(shell);
        self.notice.clear();
        self.side(shell, matches!(way, Way::Code(_) | Way::Address(_)))?;
        let doing = match &way {
            Way::Invite => "making an invite",
            Way::Code(_) => "joining by a code",
            Way::Host => "hosting directly",
            Way::Address(_) => "joining directly",
        };
        // (A host sets the match's settings from the start: the lobby starts on them.)
        let hosting = matches!(way, Way::Invite | Way::Host);
        let session = Session::open(way, &Play::net()).map_err(|e| UserError::of_link(doing, &e))?;
        self.waiting = if hosting { self.opening_settings(shell) } else { None };
        // (A direct host waits at once: there is no server to hear from.)
        self.stage = if session.way == Way::Host { Stage::Waiting } else { Stage::Meeting };
        self.session = Some(session);
        self.ended = None;
        shell.stay_awake = true;
        if matches!(self.stage, Stage::Waiting) {
            self.wait(shell);
        }
        Ok(())
    }

    /// The link and everything over it are given up, unsaid (the other player is told by [`Play::leave`]).
    fn drop_session(&mut self, shell: &mut Shell) {
        // (A link that ends over a match's result takes the result with it, before its battle: else it stays the
        // UI's, and the next match opens under it.)
        self.hide_result(shell);
        if self.session.take().is_some() || !matches!(self.stage, Stage::Idle) {
            if matches!(self.stage, Stage::Waiting) {
                practice::end(shell);
            }
            shell.cancel_loading();
            if shell.battle.as_ref().is_some_and(|b| b.owner == ScreenId::Play) {
                shell.end_battle();
            }
        }
        self.stage = Stage::Idle;
        // (What was being done about the other player's content is the link's: a later one starts anew.)
        self.switch = None;
        self.agreed = None;
        self.waiting = None;
        self.wait_checked = None;
        self.live = Live::default();
        self.changing = false;
        shell.nav.close("change.");
        self.close_rules(shell);
        shell.stay_awake = false;
        self.set_peek(shell, false);
    }

    /// Leave the other player (or stop waiting for one): they are told, and the builds show again.
    fn leave(&mut self, shell: &mut Shell) {
        if let Some(session) = self.session.take() {
            session.leave();
        }
        self.drop_session(shell);
        self.show(shell);
    }

    /// The link ended by itself (its error), or the lobby did: the player is told in a dialog, and the builds
    /// show again.
    fn ended(&mut self, shell: &mut Shell, error: Option<UserError>) {
        // (What the lobby says last goes out: a refusal.)
        if let Some(session) = &mut self.session {
            let _ = session.send();
        }
        self.session = None;
        self.drop_session(shell);
        self.direct = None;
        shell.nav.close("direct.");
        self.show(shell);
        if let Some(error) = error {
            let button = shell.ui().global::<PlayView>().get_back_to_play();
            shell.fail_with(error, "play.ended", &button);
        }
    }

    /// The link failed before the other player came: the control that asked for it says why.
    fn not_met(&mut self, shell: &mut Shell, error: UserError) {
        let direct = self.session.as_ref().is_some_and(|s| matches!(s.way, Way::Address(_)));
        self.session = None;
        self.drop_session(shell);
        shell.sound.play(Cue::Refused);
        match &mut self.direct {
            Some(dialog) if direct => dialog.join_problem = Some(error),
            _ => self.problem = Some(error),
        }
        self.show(shell);
    }

    // ---- Waiting -----------------------------------------------------------------------------------------

    /// The other player is waited for, in the lobby: the invite's code (or this machine's address) shows in their
    /// place, copied already (the user: "you should automatically copy the invite code when hosting starts"; COPY
    /// CODE flashes as though pressed), and a practice session starts in the player's own. (On a phone it starts
    /// when the player asks: there it fills the window.) The match's settings are the host's to set meanwhile.
    fn wait(&mut self, shell: &mut Shell) {
        self.stage = Stage::Waiting;
        self.direct = None;
        shell.nav.close("direct.");
        // (Not in the screenshot pass: it leaves the player's clipboard alone.)
        let invite = self.invite();
        if !invite.is_empty() && !shell.still {
            self.copy(shell, &invite);
        }
        self.show_card(shell);
        self.show(shell);
        if !shell.scale.phone {
            self.train(shell);
        }
    }

    /// A practice session for the wait (Practice's own: `practice::start`): this player's build against a copy
    /// of it that stands still, with this screen's rows at the top of the practice menu. It is in the player's
    /// place in the lobby, the screen's `Picture`. On a phone a battle has its pad only where it fills the window, so
    /// there it does, and a row of the menu stops it for the invite again.
    fn train(&mut self, shell: &mut Shell) {
        let (Some(game), Ok(side)) = (shell.game().map(str::to_string), self.side(shell, false)) else { return };
        let name = self.builds.get(self.chosen).map(|b| b.name().to_string()).unwrap_or_default();
        // The menu's way out of the wait is in EXIT's place, CANCEL INVITE (STOP HOSTING), and the code (the
        // address) is the invite's place in the lobby to copy (the user: "the stop hosting button should be in the
        // place of the exit button on the regular practice menu. get rid of copy code"). Where the session fills the
        // window and covers the lobby, a phone, one row at the top stops it, back to the lobby.
        let full = shell.scale.phone;
        let direct = self.session.as_ref().is_some_and(|s| s.way == Way::Host);
        let view = shell.ui();
        let view = view.global::<PlayView>();
        let rows = if full { vec![view.get_stop_practice().to_string()] } else { Vec::new() };
        let mut start = practice::Start::beside(&game, practice::Build { name, side }, ScreenId::Play, WAIT_ROWS, rows);
        start.exit = Some(view.invoke_wait_exit(direct).to_string());
        start.full = full;
        if shell.still {
            start.seed = 7;
        }
        // (Why it can't start is in the log: the wait goes on without it.)
        let _ = practice::start(shell, start);
    }

    // ---- The lobby ---------------------------------------------------------------------------------------

    /// The settings a lobby opens with: First to, Open setups and the Custom timer as this player last set them
    /// hosting (else a new lobby's: [`settings`]), under the ruleset this player's build is made for, which it was
    /// checked under (else the one they last chose). What the other player adopts if this one hosts.
    fn opening_settings(&self, shell: &Shell) -> Option<Settings> {
        let game = shell.game()?;
        let made_for = self.build().and_then(|b| b.made_for()).map(|made| made.pick.clone());
        let played = made_for.or_else(|| builds::kit(shell).and_then(|kit| rulesets::chosen_last(shell, &kit)))?;
        let remembered = |key: &str| shell.settings.remembered.get(key).map(String::as_str);
        let first_to = remembered(REMEMBERED_FIRST_TO).and_then(|r| r.parse().ok()).filter(|n| FIRST_TO.contains(n)).unwrap_or(nettai_match::TRIPLE_BATTLE);
        let mut settings = settings(game, first_to, played.ruleset);
        if let Some(open) = remembered(REMEMBERED_OPEN_SETUPS).and_then(|r| r.parse().ok()) {
            settings.open_setups = open;
        }
        match remembered(REMEMBERED_CUSTOM_TIME) {
            Some(UNLIMITED) => settings.custom_screen_time = None,
            Some(r) => {
                if let Some(time) = r.parse().ok().filter(|t| nettai_match::CUSTOM_SCREEN_TIMES.contains(t)) {
                    settings.custom_screen_time = Some(time);
                }
            }
            None => {}
        }
        Some(settings)
    }

    /// The match's settings the host set, remembered for the next lobby it opens ([`Play::opening_settings`]).
    fn remember_settings(shell: &mut Shell, settings: &Settings) {
        let time = settings.custom_screen_time.map_or(UNLIMITED.to_string(), |t| t.to_string());
        let remembered = &mut shell.settings.remembered;
        remembered.insert(REMEMBERED_FIRST_TO.into(), settings.first_to().to_string());
        remembered.insert(REMEMBERED_OPEN_SETUPS.into(), settings.open_setups.to_string());
        remembered.insert(REMEMBERED_CUSTOM_TIME.into(), time);
        shell.save_settings();
    }

    /// The link is open: the lobby over it, the practice battle ended. A host's starts on the settings it set
    /// while it waited, which the joiner takes.
    fn enter_lobby(&mut self, shell: &mut Shell) {
        let (Some((_, ready)), Ok(side)) = (shell.ready(), self.side(shell, self.joining())) else {
            return self.ended(shell, Some(UserError::Other));
        };
        let (Some(rulesets), Some(settings)) = (builds::kit(shell).and_then(|kit| kit.rulesets()), self.waiting.take().or_else(|| self.opening_settings(shell))) else {
            return self.ended(shell, Some(UserError::Other));
        };
        // (The screenshot pass's matches are the same from run to run.)
        let entropy = tour::entropy().unwrap_or_else(nettai_rtc::random_u64);
        let made = self.session.as_mut().is_some_and(|s| s.make_lobby(ready.content(), rulesets, settings, side, &shell.settings.name, entropy));
        if !made {
            return;
        }
        // (The wait is over: its practice session ends. The keys were its battle's, and the cursor on nothing:
        // it is the lobby's again.)
        if matches!(self.stage, Stage::Waiting) {
            practice::end(shell);
            let pad = shell.nav.pad();
            shell.nav.used_keys(pad);
        }
        self.stage = Stage::Lobby;
        self.direct = None;
        shell.nav.close("direct.");
        self.problem = None;
        self.live = Live::default();
        self.show_card(shell);
        self.show(shell);
        // (The player hosting may have gone to another window as they waited: the other player's coming calls
        // them back.)
        if !self.joining() {
            shell.call_attention();
        }
    }

    /// A match is agreed: the two cards, and the match made ready behind them; each side's bar is its own
    /// machine's.
    fn load(&mut self, shell: &mut Shell, agreed: Agreement) {
        let Some((game, _)) = shell.ready() else { return };
        let (me, other) = (agreed.side, 1 - agreed.side);
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        let mine = view.get_card();
        let cards = [
            VersusCard { role: view.get_you(), name: shell.settings.name.as_str().into(), build: mine.name.clone(), navi: mine.navi.clone(), progress: 0.0 },
            // (The other player's card carries their name alone.)
            VersusCard { role: view.get_opponent(), name: agreed.names[other].as_str().into(), ..VersusCard::default() },
        ];
        let gate = Rc::new(Gate::default());
        let (said, made) = (gate.clone(), gate.clone());
        shell.start_battle(BattleStart {
            owner: ScreenId::Play,
            kind: BattleKind::Online,
            game: game.clone(),
            ready: None,
            set: agreed.set.clone(),
            // (Its driver is the lobby's to give: the match's channel is there once both sides have loaded.)
            play: Played::Make(Box::new(move || made.driver.borrow_mut().take())),
            viewer: me as u8,
            // (Recorded from its first tick: `Play::battle`. A match that is agreed and never starts leaves no
            // file.)
            recorder: None,
            cards: Some(cards),
            full: true,
            // (A match plays its game's own music: the user, "for pvp it should use the game's default for now".)
            music: crate::battle::BattleMusic::GameOwn,
            gate: Some(Box::new(move |mine| {
                said.mine.set(mine);
                if said.go.get() { 1.0 } else { said.theirs.get().min(0.995) }
            })),
        });
        self.stage = Stage::Loading { seed: agreed.seed, gate };
        self.changing = false;
        shell.nav.close("change.");
        self.close_rules(shell);
        self.agreed = Some(agreed);
        self.show_match(shell);
    }

    /// Back in the lobby after a match (its set over, or stopped with the link still up): the match's player
    /// is dropped, and the lobby opens again, nobody ready. `why`: what ended it, if it didn't end by itself.
    fn reopen(&mut self, shell: &mut Shell, why: Option<UserError>) {
        self.hide_result(shell);
        shell.cancel_loading();
        if shell.battle.as_ref().is_some_and(|b| b.owner == ScreenId::Play) {
            shell.end_battle();
        }
        if let Some(lobby) = self.session.as_mut().and_then(Session::lobby) {
            lobby.reopen();
        }
        self.stage = Stage::Lobby;
        self.ended = why;
        self.page = MatchPage::Menu;
        self.set_peek(shell, false);
        self.show(shell);
    }

    /// How the match went, over the picture once its set is over (docs/design/app-ui.md §6, "After a match"): won,
    /// lost or drawn, the round wins each side has and the match's time, which the match's player already has (no
    /// state of the battle's is kept for it). The keys are the result's while it shows. False where the set ended
    /// some other way, which shows none.
    fn show_result(&mut self, shell: &mut Shell, result: BattleResult) -> bool {
        let outcome = match result {
            BattleResult::Won => MatchOutcome::Won,
            BattleResult::Lost => MatchOutcome::Lost,
            BattleResult::Drawn => MatchOutcome::Drawn,
            _ => return false,
        };
        let Some(b) = shell.battle.as_ref() else { return false };
        let (wins, losses) = b.stage.player.battle().wins_for(b.viewer);
        let time = b.stage.player.ticks() as f64 / FRAME_RATE;
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_outcome(outcome);
        view.set_result_wins(wins as i32);
        view.set_result_losses(losses as i32);
        view.set_result_time((time * 100.0).round().min(i32::MAX as f64) as i32);
        view.set_result_settled(false);
        view.set_result_ticks(0);
        view.set_result_done(false);
        view.set_result_shown(true);
        self.heard = ResultHeard::default();
        self.set_peek(shell, false);
        shell.cover_battle("result.", Some(Key::new("result.go", 0)));
        true
    }

    /// Whether the match's result is over the picture.
    fn result_shown(&self) -> bool {
        matches!(self.stage, Stage::Match { result: Some(Ended { shown: true, .. }) })
    }

    /// CONTINUE: the result settles where it still moves; settled, the player goes on to the lobby.
    fn go_on(&mut self, shell: &mut Shell) {
        if !self.result_shown() {
            return;
        }
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        if !view.get_result_done() && !view.get_result_settled() {
            view.set_result_settled(true);
            return;
        }
        self.reopen(shell, None);
    }

    fn hide_result(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        if view.get_result_shown() {
            view.set_result_shown(false);
            shell.cover_battle("result.", None);
        }
    }

    /// The result's sounds, the game's own: its cursor's as its numbers tick (not more often than
    /// `TICK_SPACING`), its confirm as the last one comes to its value.
    fn result_sounds(&mut self, shell: &mut Shell, now: Instant) {
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        if self.heard.done {
            return;
        }
        if view.get_result_done() {
            self.heard.done = true;
            shell.sound.play(Cue::Pick);
            return;
        }
        let ticks = view.get_result_ticks();
        if ticks > self.heard.ticks && self.heard.at.is_none_or(|at| now.saturating_duration_since(at) >= TICK_SPACING) {
            self.heard = ResultHeard { ticks, at: Some(now), done: false };
            shell.sound.play(Cue::Cursor);
        }
    }

    /// The match's settings as they stand: the lobby's, or while the host waits for the other player its own.
    fn match_settings(&mut self) -> Option<Settings> {
        match self.session.as_mut().and_then(Session::lobby) {
            Some(lobby) => Some(lobby.settings().clone()),
            None => self.waiting.clone(),
        }
    }

    /// The match's settings as the player's controls change them: the host's alone (the user: "only the host
    /// should be able to change rules"; a joiner's lobby changes nothing, and its controls are unavailable), in
    /// the lobby or while it waits for the other player (what the lobby will start on). What the host sets is
    /// remembered for its next lobby.
    fn change_settings(&mut self, shell: &mut Shell, change: impl FnOnce(&mut Settings)) {
        let changed = if let Some(lobby) = self.session.as_mut().and_then(Session::lobby) {
            let mut settings = lobby.settings().clone();
            change(&mut settings);
            lobby.propose(settings.clone()).then_some(settings)
        } else if let Some(waiting) = &mut self.waiting {
            change(waiting);
            Some(waiting.clone())
        } else {
            None
        };
        if let Some(settings) = changed {
            Self::remember_settings(shell, &settings);
        }
        self.ended = None;
    }

    fn set_first_to(&mut self, shell: &mut Shell, wins: u8) {
        self.change_settings(shell, |s| s.set_first_to(wins));
    }

    fn set_open(&mut self, shell: &mut Shell, open: bool) {
        self.change_settings(shell, |s| s.open_setups = open);
    }

    /// The Custom timer stepped `by`: through the match's time limits, then none (∞), and around.
    fn step_custom_time(&mut self, shell: &mut Shell, by: i32) {
        let choices: Vec<Option<u16>> = nettai_match::CUSTOM_SCREEN_TIMES.into_iter().map(Some).chain([None]).collect();
        let now = choices.iter().position(|&c| c == self.live.custom_time).unwrap_or(0) as i32;
        let time = choices[(now + by).rem_euclid(choices.len() as i32) as usize];
        self.change_settings(shell, |s| s.custom_screen_time = time);
    }

    /// The host picks the match's ruleset (a joiner's lobby changes nothing): it is the one this player last chose,
    /// remembered (the one a new build is made for).
    fn set_ruleset(&mut self, shell: &mut Shell, picked: &Pickable) {
        if let Some(game) = shell.game().map(str::to_string) {
            rulesets::remember(shell, &game, &picked.ruleset.name);
        }
        self.change_settings(shell, |s| s.ruleset = picked.ruleset.clone());
    }

    /// The rulesets the host can pick, and which of them the match's is.
    fn ruleset_choices(&mut self, shell: &Shell) -> Option<(Vec<Pickable>, usize)> {
        let kit = builds::kit(shell)?;
        let settings = self.match_settings()?;
        let picks = rulesets::pickable(&kit);
        let at = picks.iter().position(|p| p.ruleset.name == settings.ruleset.name).unwrap_or(0);
        Some((picks, at))
    }

    /// The lobby's Ruleset stepped or chosen (the host's): the `at`th of the rulesets it can pick.
    fn pick_ruleset(&mut self, shell: &mut Shell, at: i32, by: i32) {
        let Some((picks, now)) = self.ruleset_choices(shell) else { return };
        if picks.is_empty() || !self.is_host() {
            return;
        }
        let at = if by == 0 { at } else { now as i32 + by };
        let picked = picks[at.rem_euclid(picks.len() as i32) as usize].clone();
        self.set_ruleset(shell, &picked);
    }

    /// Whether this player hosts the lobby (and so chooses the match's settings): its lobby's role, or while it
    /// waits for the other player, an invite out or this machine hosting.
    fn is_host(&mut self) -> bool {
        match self.session.as_mut() {
            Some(session) if matches!(self.stage, Stage::Waiting) => matches!(session.way, Way::Invite | Way::Host),
            Some(session) => session.lobby().is_some_and(|l| l.role() == nettai_frontend::lobby::Role::Host),
            None => false,
        }
    }

    /// EDIT (the host's) or VIEW (the joiner's): the lobby's ruleset's page, over the lobby. The host's own
    /// ruleset can be changed there, and a change is the match's at once; a preset, or the host's ruleset seen by
    /// the joiner, is read alone. (While the host waits for the other player, the practice battle stops under it,
    /// as the page takes the keys, and starts again on what its menu was left at as it closes.)
    fn open_rules(&mut self, shell: &mut Shell) {
        let host = self.is_host();
        let (Some(kit), Some(ruleset)) = (builds::kit(shell), self.match_settings().map(|s| s.ruleset)) else { return };
        if matches!(self.stage, Stage::Waiting) && practice::running(shell) {
            practice::stop(shell);
        }
        let shown = match host {
            true => rulesets::pickable(&kit).into_iter().find(|p| p.ruleset == ruleset),
            false => None,
        };
        let shown = shown.unwrap_or(Pickable { ruleset, path: None });
        let back = shell.ui().global::<nettai_ui::RulesetView>().get_lobby_word().to_string();
        shell.sound.play(Cue::Pick);
        self.rules = RulesPage::open(shell, kit, shown, !host, &back);
        // (A page that didn't open gives the wait its practice battle back.)
        if self.rules.is_none() && matches!(self.stage, Stage::Waiting) && !shell.scale.phone {
            self.train(shell);
        }
    }

    /// The ruleset's page closed (Back), or gone with the lobby. (Waiting for the other player on a desktop, the
    /// practice battle starts again.)
    fn close_rules(&mut self, shell: &mut Shell) {
        if let Some(page) = self.rules.take() {
            page.close(shell);
            if matches!(self.stage, Stage::Waiting) && !shell.scale.phone {
                self.train(shell);
            }
        }
    }

    /// What the ruleset's page did: the ruleset it shows changed, or is another (a copy), and is the match's
    /// (the host's) and the one the player last chose; renamed, the builds made for it are by its new name (their
    /// files are already); or it was deleted, and the game's first preset is.
    fn ruleset_outcome(&mut self, shell: &mut Shell, outcome: Outcome) {
        match outcome {
            Outcome::Nothing => {}
            Outcome::Changed(picked) => self.set_ruleset(shell, &picked),
            Outcome::Renamed { from, to } => {
                for build in &mut self.builds {
                    build.follow_rename(&from, &to);
                }
                self.set_ruleset(shell, &to);
            }
            Outcome::Deleted => {
                self.rules = None;
                if let Some(first) = builds::kit(shell).and_then(|kit| rulesets::pickable(&kit).into_iter().next()) {
                    self.set_ruleset(shell, &first);
                }
            }
        }
        self.show_live(shell);
    }

    fn set_delay(&mut self, shell: &mut Shell, by: i32) {
        shell.settings.input_delay = (shell.settings.input_delay as i32 + by).clamp(0, MAX_INPUT_DELAY as i32) as u32;
        shell.save_settings();
        if let Some(b) = &mut shell.battle {
            b.stage.player.set_input_delay(shell.settings.input_delay);
        }
        shell.ui().global::<PlayView>().set_input_delay(shell.settings.input_delay as i32);
    }

    fn set_full_screen(&mut self, shell: &mut Shell, on: bool) {
        settings::set_full_screen(shell, on);
        self.show_match(shell);
    }

    fn set_enhanced(&mut self, shell: &mut Shell, enhanced: bool) {
        shell.set_enhanced(enhanced);
        self.show_match(shell);
    }

    fn set_levels(&mut self, shell: &mut Shell, music: u32, effects: u32) {
        (shell.settings.music, shell.settings.effects) = (music.min(100), effects.min(100));
        shell.set_sound_levels(shell.settings.music, shell.settings.effects);
        shell.save_settings();
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_music(shell.settings.music as i32);
        view.set_effects(shell.settings.effects as i32);
    }

    // ---- What is shown -----------------------------------------------------------------------------------

    /// The lobby's own state, told to the UI where it changed: the lobby's, or while the host waits for the other
    /// player the settings it holds, and this player's build checked under them.
    fn show_live(&mut self, shell: &mut Shell) {
        let waiting = self.waiting.clone().filter(|_| matches!(self.stage, Stage::Waiting));
        if let Some(settings) = &waiting
            && self.wait_checked.as_ref().is_none_or(|(checked, ..)| checked != settings)
            && let (Some((_, ready)), Some(rulesets), Ok(side)) = (shell.ready(), builds::kit(shell).and_then(|kit| kit.rulesets()), self.side(shell, false))
        {
            let problems = settings.checked(ready.content(), &rulesets, &side).problems;
            self.wait_checked = Some((settings.clone(), rulesets, problems));
        }
        let waited = waiting.and(self.wait_checked.clone());
        let Some(session) = &mut self.session else { return };
        let reconnecting = session.reconnecting();
        let ping = session.ping();
        // (The screenshot pass shows one round trip: what two players of one process measure moves about.)
        let steady = tour::steady_ping(shell.still).filter(|_| ping.is_some());
        let (settings, rulesets, problems, host, (ready, their_ready), their) = match (session.lobby(), waited) {
            (Some(lobby), _) => (
                lobby.settings().clone(),
                lobby.rulesets().clone(),
                lobby.problems().to_vec(),
                lobby.role() == nettai_frontend::lobby::Role::Host,
                (lobby.ready(), lobby.their_ready()),
                lobby.their_present().then(|| lobby.their_name().unwrap_or_default().to_string()),
            ),
            (None, Some((settings, rulesets, problems))) => (settings, rulesets, problems, true, (false, false), None),
            (None, None) => return,
        };
        let live = Live {
            their_present: their.is_some(),
            their_name: their.unwrap_or_default(),
            ready,
            their_ready,
            first_to: settings.first_to() as i32,
            ruleset: Some(settings.ruleset.clone()),
            host,
            open: settings.open_setups,
            custom_time: settings.custom_screen_time,
            bars: if reconnecting.is_some() { 0 } else { steady.map_or(ping.map_or(0, |p| p.quality().bars() as i32), |s| s.0) },
            ping: if reconnecting.is_some() { -1 } else { steady.map_or(ping.map_or(-1, |p| p.ms.round() as i32), |s| s.1) },
            // (A build the settings can't play: the lobby's own check, typed; while the host waits, the same check
            // of the build it brings.)
            note: Some(match (reconnecting, ready, their_ready) {
                (Some(_), ..) => LobbyNote::Reconnecting,
                (None, ..) if !problems.is_empty() => LobbyNote::Unusable,
                (None, false, true) => LobbyNote::You,
                (None, true, false) => LobbyNote::Them,
                (None, false, false) | (None, true, true) => LobbyNote::None,
            }),
            problems,
            seconds: reconnecting.unwrap_or(0) as i32,
        };
        if live == self.live {
            return;
        }
        // A mark stamps as its player turns ready.
        if live.ready && !self.live.ready {
            self.ready_seq += 1;
        }
        if live.their_ready && !self.live.their_ready {
            self.their_ready_seq += 1;
        }
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_their_present(live.their_present);
        view.set_their_name(live.their_name.as_str().into());
        view.set_ready(live.ready);
        view.set_their_ready(live.their_ready);
        view.set_ready_seq(self.ready_seq);
        view.set_their_ready_seq(self.their_ready_seq);
        view.set_first_to(live.first_to);
        view.set_host(live.host);
        if let (Some(ruleset), Some((_, ready))) = (&live.ruleset, shell.ready()) {
            let graphics = ready.graphics(shell.i18n.code);
            view.set_ruleset(rulesets::shown_name(&Names::of(ready.content(), &graphics), &ruleset.name).into());
        }
        // The builds CHANGE BUILD offers are checked under the lobby's ruleset, the host's, where it changed.
        if live.ruleset != self.live.ruleset
            && let Some(ruleset) = &live.ruleset
        {
            self.check_builds(shell, Some(shelf::Under { rulesets: &rulesets, ruleset }));
            // (A joiner viewing the host's ruleset sees each change as it comes.)
            if let Some(page) = self.rules.as_mut() {
                page.follow(shell, ruleset);
            }
        }
        // (The build's problems in the player's words, as the editor's header says them: the first and "+n more".)
        if live.problems != self.live.problems || live.ruleset != self.live.ruleset {
            let said = match (live.problems.is_empty(), shell.ready()) {
                (false, Some((game, ready))) => Some(words::problems(shell, &game, &ready, &live.problems)),
                _ => None,
            };
            let line = said.map(|said| UserError::BuildUnusable { reason: Some(said).filter(|s| !s.is_empty()) }.line(&ui));
            view.set_note_problem(line.unwrap_or_default().into());
            self.show_card(shell);
        }
        view.set_open(live.open);
        view.set_custom_time(live.custom_time.map_or(0, i32::from));
        view.set_bars(live.bars);
        view.set_ping(live.ping);
        view.set_note(live.note.unwrap_or(LobbyNote::None));
        view.set_note_seconds(live.seconds);
        self.live = live;
    }

    /// What the match's menu and panels show: both builds (the other player's has no name here: none was
    /// ever sent), whether the setups are open, the controls (and the footer's of them), the levels.
    fn show_match(&self, shell: &Shell) {
        let (Some(agreed), Some(kit)) = (&self.agreed, builds::kit(shell)) else { return };
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        // (The agreement's sides are as the match plays them: without their patch cards where those are off.
        // Where the setups aren't open the other player's isn't shown, and isn't made.)
        let (me, other) = (agreed.side, 1 - agreed.side);
        view.set_mine(overview::sheet(shell, &kit, &view.get_you(), &shell.settings.name, &agreed.sides[me]));
        view.set_theirs(match agreed.open_setups() {
            true => overview::sheet(shell, &kit, &view.get_opponent(), &agreed.names[other], &agreed.sides[other]),
            false => BuildSheet::default(),
        });
        view.set_open_match(agreed.open_setups());
        let controls: Vec<PlayControl> = shell
            .bindings
            .keys
            .iter()
            .zip(&shell.bindings.pad)
            .map(|(key, button)| {
                let face = match button {
                    Button::South => PadButton::South,
                    Button::East => PadButton::East,
                    Button::West => PadButton::West,
                    Button::North => PadButton::North,
                    _ => PadButton::None,
                };
                PlayControl { key: key_cap(*key).into(), face, pad: pad_cap(*button).into() }
            })
            .collect();
        view.set_controls(ModelRc::new(VecModel::from(controls)));
        view.set_match_keys(ModelRc::new(VecModel::from(match_keys(shell, &view))));
        view.set_music(shell.settings.music as i32);
        view.set_effects(shell.settings.effects as i32);
        view.set_input_delay(shell.settings.input_delay as i32);
        settings::show_values(shell, &self.binding);
        view.set_page(self.page);
    }

    fn set_page(&mut self, shell: &mut Shell, page: MatchPage) {
        self.page = page;
        self.binding.stop(shell);
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_page(page);
        // (The page's main control: the menu's first item, a page's Back.)
        shell.nav.focus(if page == MatchPage::Menu { "menu.resume" } else { "menu.back" }, 0);
    }

    /// The builds show over the match (Tab or Select held), to be scrolled.
    fn peeking(&self) -> bool {
        self.peek && matches!(self.stage, Stage::Match { .. })
    }

    /// The builds shown over the match scrolled by `by`, both by the same: whether they moved.
    fn scroll_peek(shell: &Shell, by: Scroll) -> bool {
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        let at = -view.get_peek_y();
        let to = by.to(at, view.get_peek_span(), view.get_peek_view());
        if (to - at).abs() < 0.01 {
            return false;
        }
        view.set_peek_y(-to);
        true
    }

    /// Show the builds or put them away; they show from their tops. While they show over the match, the match's keys
    /// are theirs to scroll with, and none reaches the battle (the user: "holding tab should allow you to scroll
    /// the build instead of passing input to the game").
    fn set_peek(&mut self, shell: &mut Shell, peek: bool) {
        if peek != self.peek {
            self.peek = peek;
            shell.keep_keys(peek);
            let ui = shell.ui();
            let view = ui.global::<PlayView>();
            view.set_peek_y(0.0);
            view.set_peek(peek);
        }
    }

    /// Copy what the other player needs (the invite's code, this machine's address): it flashes.
    fn copy(&mut self, shell: &Shell, text: &str) {
        shell.copy(text);
        self.copied_seq += 1;
        shell.ui().global::<PlayView>().set_copied_seq(self.copied_seq);
    }

    /// What the other player is given: the invite's code in threes, or this machine's address.
    fn invite(&self) -> String {
        match &self.session {
            Some(session) if session.way == Way::Host => session::host_address(session.port().unwrap_or(session::DIRECT_PORT)),
            Some(session) => session.code().map(code_shown).unwrap_or_default(),
            None => String::new(),
        }
    }

    // ---- The code's six boxes ----------------------------------------------------------------------------

    /// Open the code's boxes, each on the character the field has there (the first of the alphabet where it
    /// has none, or one no code is written in).
    fn open_boxes(&mut self, shell: &mut Shell) {
        let mut rows = [0; CODE_LENGTH];
        for (row, c) in rows.iter_mut().zip(self.code.chars()) {
            *row = ALPHABET.find(c).unwrap_or(0);
        }
        // (The focus comes to each box on the character it shows, not the one it last left it on.)
        for (at, row) in rows.iter().enumerate() {
            shell.nav.set_row(&code_box(at), *row as i32);
        }
        self.boxes = Some(rows);
        self.problem = None;
        shell.nav.open("boxes.", Key::new(&code_box(0), rows[0] as i32));
        self.show(shell);
    }

    fn close_boxes(&mut self, shell: &mut Shell) {
        // (What the boxes say is the code now.)
        if let Some(rows) = self.boxes.take() {
            self.code = rows.iter().filter_map(|row| ALPHABET.chars().nth(*row)).collect();
        }
        shell.nav.close("boxes.");
        self.show(shell);
    }

    /// Put box `at` on the character at `row`.
    fn set_box(&mut self, shell: &Shell, at: usize, row: usize) {
        let Some(rows) = &mut self.boxes else { return };
        if at >= CODE_LENGTH || row >= ALPHABET.len() {
            return;
        }
        rows[at] = row;
        self.show_boxes(shell);
    }

    fn show_boxes(&self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_boxes(self.boxes.is_some());
        view.set_code_set(ALPHABET.len() as i32);
        let characters: Vec<slint::SharedString> = self.boxes.iter().flatten().map(|row| ALPHABET[*row..*row + 1].into()).collect();
        view.set_box_characters(ModelRc::new(VecModel::from(characters)));
    }

    /// The box the focus is on.
    fn box_focused(shell: &Shell) -> Option<usize> {
        shell.nav.focused().and_then(|k| k.id.strip_prefix("boxes.")).and_then(|at| at.parse().ok())
    }

    // ---- The direct connection's dialog ------------------------------------------------------------------

    fn open_direct(&mut self, shell: &mut Shell) {
        self.direct = Some(Direct { host_address: session::host_address(session::DIRECT_PORT), ..Direct::default() });
        self.problem = None;
        shell.nav.open_editing("direct.", Key::new("direct.address", 0));
        self.show(shell);
    }

    fn close_direct(&mut self, shell: &mut Shell) {
        // (A join under way is given up with its dialog.)
        if matches!(self.stage, Stage::Meeting) && self.session.as_ref().is_some_and(|s| matches!(s.way, Way::Address(_))) {
            self.session = None;
            self.drop_session(shell);
        }
        self.direct = None;
        shell.nav.close("direct.");
        self.show(shell);
    }
}

/// A recorder of the match the lobby agreed, to a new file in the replays folder: its info is the agreement's
/// (an online match, from this player's side, by the names the lobby had, open setups where they were
/// agreed). None if the file can't be made (why is in the log).
fn recorder(content: &std::sync::Arc<nettai_battle::Content>, agreed: &Agreement, game: &str) -> Option<Recorder> {
    let dir = crate::paths::replays();
    let made = (|| -> Result<Recorder, String> {
        nettai_files::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let when = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let stamp = chrono::DateTime::from_timestamp(when as i64, 0).map(|t| t.with_timezone(&chrono::Local).format("%Y%m%d-%H%M%S").to_string()).unwrap_or_default();
        let path = dir.join(format!("{stamp}-{game}.{}", nettai_frontend::replay::EXTENSION));
        let file = nettai_files::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Recorder::of_agreement(Box::new(std::io::BufWriter::new(file)), content, agreed, when)
    })();
    made.map_err(|why| crate::errors::log("making a match's replay file", &why)).ok()
}

impl Play {
    /// The other player's content, where it isn't this side's.
    ///
    /// Its hash isn't this side's (`Lobby::their_content`): both play the newer revision. This side's revision is
    /// the bundle its game is loaded from; theirs is found in the store or the game's index (read again where it
    /// isn't listed). Theirs newer: it is downloaded and the game loaded again on it, and the lobby plays it
    /// ([`Lobby::replace_content`], the build made again of it); theirs older: this side waits for them to take its
    /// own. Neither found, or the same revision of other content, or development's content: the lobby refuses, the
    /// reason said to both.
    ///
    /// Its background pool isn't this side's (`Lobby::their_backgrounds`): this side takes every game's newest
    /// content, whatever Settings say (`contents::fetch_every_newest`), and where that changed its game's content or
    /// the pool, loads the game again on it, behind the bar, for the lobby to play; then it tells the lobby what it
    /// couldn't have (`Lobby::updated_backgrounds`), which waits for the other to do the same, or refuses for that.
    /// Once under way, that goes on to its end whatever the lobby says meanwhile.
    fn follow_content(&mut self, shell: &mut Shell) {
        let game = shell.game().map(str::to_string);
        match (self.switch.take(), &game) {
            (Some(Switch::Updating(of, done)), Some(game)) => return self.updated(shell, game, of, done),
            (Some(Switch::Rebuilding(of, failed)), Some(game)) => return self.rebuilt(shell, game, of, failed),
            (other, _) => self.switch = other,
        }
        let lobby = self.session.as_mut().and_then(Session::lobby);
        let (theirs, their_pool) = lobby.map_or((None, None), |l| (l.their_content(), l.their_backgrounds()));
        if let (Some(pool), Some(game)) = (their_pool, &game) {
            return self.update_every_game(game, pool);
        }
        let (Some(theirs), Some(game)) = (theirs, game) else {
            if !matches!(self.switch, Some(Switch::Reloading(_))) {
                self.switch = None;
            }
            return;
        };
        let refuse = |play: &mut Play, why: String| play.refuse_content(&format!("playing {game} with the other player's content {theirs}"), why);
        match self.switch.take() {
            Some(Switch::Finding(of, done)) if of == theirs => {
                if !done.get() {
                    self.switch = Some(Switch::Finding(of, done));
                    return;
                }
                self.decide_content(shell, &game, theirs, false, &refuse);
            }
            Some(Switch::Fetching(of, done)) if of == theirs => match done.borrow_mut().take() {
                None => self.switch = Some(Switch::Fetching(of, done.clone())),
                Some(Err(why)) => refuse(self, why),
                Some(Ok(())) => {
                    if shell.play_newest(&game) {
                        self.switch = Some(Switch::Reloading(theirs));
                    } else {
                        refuse(self, format!("{theirs} isn't {game}'s newest content in the store"));
                    }
                }
            },
            Some(Switch::Reloading(of)) if of == theirs => {
                let Some((_, ready)) = shell.ready() else {
                    if shell.games.playable().iter().any(|g| *g == game) {
                        self.switch = Some(Switch::Reloading(of));
                    } else {
                        refuse(self, format!("{game} couldn't be loaded on it"));
                    }
                    return;
                };
                if ready.content().hash() != theirs {
                    return refuse(self, format!("{game} loaded on the content downloaded plays {}", ready.content().hash()));
                }
                if let Err(why) = self.play_reloaded(shell, &game, ready.content(), "the other's content") {
                    refuse(self, why);
                }
            }
            Some(Switch::Waiting(of)) if of == theirs => self.switch = Some(Switch::Waiting(of)),
            _ => self.decide_content(shell, &game, theirs, true, &refuse),
        }
    }

    /// Refuse the other player for their content (`Lobby::refuse_content`), `why` said to both and logged as of
    /// `doing`; nothing more is done about it.
    fn refuse_content(&mut self, doing: &str, why: String) {
        crate::errors::log(doing, &why);
        if let Some(lobby) = self.session.as_mut().and_then(Session::lobby) {
            lobby.refuse_content(&why);
        }
        self.switch = None;
    }

    /// Game `game`, loaded again on `content` (the other player's, or every game's newest): the lobby plays it from
    /// now, with this player's build made again of it ([`Lobby::replace_content`]: not ready). Why it can't, if it
    /// can't (`of` says what the content is, for that).
    fn play_reloaded(&mut self, shell: &mut Shell, game: &str, content: &Arc<nettai_battle::Content>, of: &str) -> Result<(), String> {
        self.read_builds(shell, game);
        let (side, rulesets) = (self.side(shell, self.joining()), builds::kit(shell).and_then(|kit| kit.rulesets()));
        let played = match (side, rulesets, self.session.as_mut().and_then(Session::lobby)) {
            (Ok(side), Some(rulesets), Some(lobby)) => {
                lobby.replace_content(content, rulesets, side);
                Ok(())
            }
            (Err(problem), _, _) => {
                self.problem = Some(problem);
                Err(format!("this player's build can't be played on {of}"))
            }
            _ => Err("the game's rulesets can't be read".into()),
        };
        self.show(shell);
        played
    }

    /// The other player's background pool, `theirs`, isn't this side's: every game's newest content is taken
    /// ([`Play::follow_content`]).
    fn update_every_game(&mut self, game: &str, theirs: u64) {
        let done: Rc<RefCell<Option<crate::contents::Updated>>> = Rc::default();
        let told = done.clone();
        match slint::spawn_local(async move { *told.borrow_mut() = Some(crate::contents::fetch_every_newest().await) }) {
            Ok(_) => self.switch = Some(Switch::Updating(theirs, done)),
            Err(why) => self.refuse_content(&format!("taking every game's newest content for {game}'s match"), format!("the download couldn't start: {why}")),
        }
    }

    /// Every game's newest content was being taken against the other's pool `of` ([`Play::update_every_game`]):
    /// once it has been, the games whose content arrived play it (another game's once it is next shown, as its
    /// update would), and game `game` is loaded again where its content or the pool changed; else the lobby is told.
    fn updated(&mut self, shell: &mut Shell, game: &str, of: u64, done: Rc<RefCell<Option<crate::contents::Updated>>>) {
        let Some(updated) = done.borrow_mut().take() else {
            self.switch = Some(Switch::Updating(of, done));
            return;
        };
        if let Some(why) = &updated.failed {
            crate::errors::log("taking every game's newest content for an online match", why);
        }
        for other in updated.arrived.iter().filter(|g| *g != game) {
            shell.games.arrived(other);
        }
        let current = shell.ready().is_some_and(|(_, ready)| {
            crate::contents::dir(game).as_deref() == Some(ready.loaded.game.dir.as_path()) && ready.content().background_pool.identity() == updated.pool
        });
        if current {
            if let Some(lobby) = self.session.as_mut().and_then(Session::lobby) {
                lobby.updated_backgrounds(of, updated.failed);
            }
            return;
        }
        shell.reload_game(game);
        self.switch = Some(Switch::Rebuilding(of, updated.failed));
    }

    /// Game `game` is being loaded again on every game's newest content ([`Play::updated`]): once it is ready, the
    /// lobby plays it, and is told what couldn't be had (`failed`) against the other's pool `of`.
    fn rebuilt(&mut self, shell: &mut Shell, game: &str, of: u64, failed: Option<String>) {
        let doing = "taking every game's newest content for an online match";
        let Some((_, ready)) = shell.ready() else {
            if shell.games.playable().iter().any(|g| g == game) {
                self.switch = Some(Switch::Rebuilding(of, failed));
            } else {
                self.refuse_content(doing, format!("{game} couldn't be loaded on its newest content"));
            }
            return;
        };
        match self.play_reloaded(shell, game, ready.content(), "every game's newest content") {
            Ok(()) => {
                if let Some(lobby) = self.session.as_mut().and_then(Session::lobby) {
                    lobby.updated_backgrounds(of, failed);
                }
            }
            Err(why) => self.refuse_content(doing, why),
        }
    }

    /// Which side takes the other's content ([`Play::follow_content`]): `theirs` is the other player's content's
    /// hash; with `look`, their game's index may be read again first where their revision isn't known.
    fn decide_content(&mut self, shell: &Shell, game: &str, theirs: ContentHash, look: bool, refuse: &dyn Fn(&mut Play, String)) {
        let mine = shell.ready().and_then(|(_, ready)| crate::contents::stored_at(&ready.loaded.game.dir)).map(|s| s.record.revision);
        let Some(mine) = mine else {
            return refuse(self, "this side plays content of no revision (development's)".into());
        };
        let Some(their_revision) = crate::contents::revision_of(game, theirs) else {
            if !look {
                return refuse(self, format!("{game}'s index lists no content {theirs} for nettai {}", crate::contents::line()));
            }
            let (done, game) = (Rc::new(Cell::new(false)), game.to_string());
            let told = done.clone();
            let started = slint::spawn_local(async move {
                let read = async { crate::contents::fetch_catalog().await?; crate::contents::fetch_index(&game).await };
                if let Err(why) = read.await {
                    crate::errors::log(&format!("reading {game}'s index for the other player's content"), &why);
                }
                told.set(true);
            });
            return match started {
                Ok(_) => self.switch = Some(Switch::Finding(theirs, done)),
                Err(why) => refuse(self, format!("reading the index couldn't start: {why}")),
            };
        };
        match their_revision.cmp(&mine) {
            std::cmp::Ordering::Less => self.switch = Some(Switch::Waiting(theirs)),
            std::cmp::Ordering::Equal => refuse(self, format!("both play r{mine}, of other content")),
            std::cmp::Ordering::Greater => {
                let (done, game) = (Rc::new(RefCell::new(None)), game.to_string());
                let told = done.clone();
                let started = slint::spawn_local(async move {
                    let fetched = match crate::contents::fetch_playing(&game, theirs).await {
                        Ok(Some(_)) => Ok(()),
                        Ok(None) => Err(format!("no bundle of {game} plays {theirs}")),
                        Err(why) => Err(why),
                    };
                    *told.borrow_mut() = Some(fetched);
                });
                match started {
                    Ok(_) => self.switch = Some(Switch::Fetching(theirs, done)),
                    Err(why) => refuse(self, format!("the download couldn't start: {why}")),
                }
            }
        }
    }
}

impl Screen for Play {
    fn main(&self) -> &'static str {
        self.view().1
    }

    fn show(&mut self, shell: &mut Shell) {
        // Another game was selected: its builds, and no link of the one before (nor what an import made).
        if self.builds_of.as_deref() != shell.game() {
            self.notice.clear();
            if !matches!(self.stage, Stage::Idle) {
                if let Some(session) = self.session.take() {
                    session.leave();
                }
                self.drop_session(shell);
            }
        }
        // (While a link is up nothing is read or checked here: a link nobody polls reads as dropped.)
        if self.session.is_none()
            && let Some((game, ready)) = shell.ready()
        {
            let _ = ready;
            self.read_builds(shell, &game);
        }
        // (A game being loaded again on the other player's content, in the lobby, keeps its builds and its link.)
        if shell.ready().is_none() && self.session.is_none() {
            self.builds.clear();
            self.builds_of = None;
        }
        let part = self.part();
        let (name, main) = self.view();
        shell.nav.enter(name, main);
        shell.lock_game(part != PlayPart::Builds);
        // The list's cursor is on the chosen build (the chosen one is the one under it): as the screen is
        // shown, and when the list was read again under the cursor. With no build, NEW BUILD.
        if part == PlayPart::Builds && self.naming.is_none() {
            let on_list = shell.nav.focused().is_some_and(|k| k.id == "builds" || k.id == "build");
            if self.builds.is_empty() {
                if self.fresh || on_list {
                    shell.nav.focus("new", 0);
                }
            } else if self.fresh || on_list {
                match shell.scale.phone {
                    // (On a phone the builds are one box.)
                    true => shell.nav.focus("build", 0),
                    false => shell.nav.focus("builds", self.chosen as i32),
                }
            }
        }
        self.fresh = false;
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_part(part);
        view.set_builds(ModelRc::new(VecModel::from(self.builds.iter().map(|b| b.row.clone()).collect::<Vec<_>>())));
        view.set_chosen(self.chosen as i32);
        view.set_build(self.build().map(|b| b.row.clone()).unwrap_or_default());
        self.show_sheet(shell);
        if let Some(kit) = builds::kit(shell) {
            self.show_versions(shell, &kit);
        }
        self.show_dialog(shell);
        view.set_code(code_shown(&self.code).into());
        self.show_boxes(shell);
        // The reserved line under the row: a problem, or that a link is being made, or what an import made.
        let meeting = matches!(self.stage, Stage::Meeting);
        let asked_directly = self.session.as_ref().is_some_and(|s| matches!(s.way, Way::Address(_)));
        view.set_message(match &self.problem {
            Some(problem) => problem.line(&ui).into(),
            None if meeting && !asked_directly => view.get_connecting(),
            None => self.notice.as_str().into(),
        });
        view.set_message_quiet(self.problem.is_none());
        // (A build's problems have their mark: what keeps it from being played is the build's, not the control's.)
        view.set_message_problem(matches!(self.problem, Some(UserError::BuildUnusable { .. })));
        // The direct connection's dialog.
        view.set_direct(self.direct.is_some());
        if let Some(direct) = &self.direct {
            view.set_host_address(direct.host_address.as_str().into());
            view.set_address(direct.address.as_str().into());
            view.set_host_message(direct.host_problem.as_ref().map(|p| p.line(&ui)).unwrap_or_default().into());
            view.set_join_message(match &direct.join_problem {
                Some(problem) => problem.line(&ui).into(),
                None if meeting && asked_directly => view.get_connecting(),
                None => Default::default(),
            });
            view.set_join_quiet(direct.join_problem.is_none());
        }
        // Waiting, and the lobby.
        view.set_waiting(matches!(self.stage, Stage::Waiting));
        view.set_invite(self.invite().into());
        view.set_invite_direct(self.session.as_ref().is_some_and(|s| s.way == Way::Host));
        view.set_input_delay(shell.settings.input_delay as i32);
        view.set_note_error(self.ended.as_ref().map(|e| e.line(&ui)).unwrap_or_default().into());
        view.set_changing(self.changing);
        self.show_live(shell);
        self.show_match(shell);
    }

    fn hide(&mut self, shell: &mut Shell) {
        // (Another screen: whoever was linked is left.)
        if let Some(session) = self.session.take() {
            session.leave();
        }
        self.drop_session(shell);
        self.direct = None;
        shell.nav.close("direct.");
        self.boxes = None;
        shell.nav.close("boxes.");
        self.close_dialog(shell);
        self.problem = None;
        self.notice.clear();
        self.fresh = true;
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        // (The ruleset's page's controls are its own.)
        if RulesPage::owns(id) {
            let outcome = self.rules.as_mut().map_or(Outcome::Nothing, |page| page.activate(shell, id, index));
            return self.ruleset_outcome(shell, outcome);
        }
        // (A control of a part that isn't shown any more does nothing: a press may come as the part changes.)
        let part = self.part();
        let of_its_part = match id {
            "builds" | "build" | "new" | "edit" | "more" | "invite" | "join" | "direct" => part == PlayPart::Builds,
            "wait.copy" | "wait.cancel" | "wait.train" => matches!(self.stage, Stage::Waiting),
            // (The match's settings and the player's own are the host's to set while it waits too.)
            "lobby.first-to" | "lobby.open" | "lobby.custom-time" | "lobby.ruleset" | "lobby.rules" | "lobby.delay" => matches!(self.stage, Stage::Lobby | Stage::Waiting),
            _ if id.starts_with("name.") => self.naming.is_some(),
            _ if id.starts_with("direct.") => self.direct.is_some(),
            _ if id.starts_with("boxes.") => self.boxes.is_some(),
            _ if id.starts_with("lobby.") || id.starts_with("change.") => matches!(self.stage, Stage::Lobby),
            _ => true,
        };
        if !of_its_part {
            return;
        }
        match id {
            // ---- The builds, what is done with them, and the row.
            // (On a phone the builds are one box, which opens them as a sheet.)
            "build" => {
                let names = self.builds.iter().map(|b| b.name().to_string()).collect();
                shell.pick("build", 0, names, self.chosen);
            }
            "builds" => {
                self.choose_build(shell, index.max(0) as usize);
                shell.nav.focus("invite", 0);
            }
            "new" => self.ask_new(shell),
            "edit" => self.edit_chosen(shell),
            "more" => self.open_menu(shell, &Key::new("more", 0)),
            "name.confirm" => self.confirm(shell),
            "name.cancel" => self.close_dialog(shell),
            // (New builds of saves: the system's file dialog, as the dialog closes.)
            "name.save" => {
                self.close_dialog(shell);
                self.import_chosen_files(shell);
            }
            // (Enter on the versions takes the other one.)
            "name.version" => self.step(shell, id, index, 1),
            "invite" => {
                self.problem = self.connect(shell, Way::Invite).err();
                self.refuse_or_show(shell);
            }
            "join" => {
                self.problem = self.connect(shell, Way::Code(self.code.clone())).err();
                self.refuse_or_show(shell);
            }
            // (The browser has no direct links: its button isn't there.)
            "direct" if !cfg!(target_arch = "wasm32") => self.open_direct(shell),
            // ---- The code's boxes: Enter on one goes to the next, and from the last to JOIN.
            "boxes.join" => {
                self.close_boxes(shell);
                self.problem = self.connect(shell, Way::Code(self.code.clone())).err();
                self.refuse_or_show(shell);
            }
            "boxes.cancel" => self.close_boxes(shell),
            _ if id.starts_with("boxes.") => match Play::box_focused(shell) {
                Some(at) if at + 1 < CODE_LENGTH => {
                    let row = self.boxes.map_or(0, |rows| rows[at + 1]);
                    shell.nav.focus(&code_box(at + 1), row as i32);
                }
                _ => shell.nav.focus("boxes.join", 0),
            },
            // ---- The direct connection's dialog.
            "direct.copy" => {
                let address = self.direct.as_ref().map(|d| d.host_address.clone()).unwrap_or_default();
                self.copy(shell, &address);
            }
            "direct.host" => {
                let problem = self.connect(shell, Way::Host).err();
                if let Some(direct) = &mut self.direct {
                    (direct.host_problem, direct.join_problem) = (problem, None);
                    shell.sound.play(Cue::Refused);
                    self.show(shell);
                }
            }
            "direct.connect" => {
                let address = self.direct.as_ref().map(|d| d.address.clone()).unwrap_or_default();
                let problem = direct_address(&address).and_then(|address| self.connect(shell, Way::Address(address))).err();
                if let Some(direct) = &mut self.direct {
                    if problem.is_some() {
                        shell.sound.play(Cue::Refused);
                    }
                    (direct.join_problem, direct.host_problem) = (problem, None);
                }
                self.show(shell);
            }
            "direct.cancel" => self.close_direct(shell),
            // ---- Waiting.
            "wait.copy" => {
                let invite = self.invite();
                self.copy(shell, &invite);
            }
            "wait.cancel" => self.leave(shell),
            "wait.train" => self.train(shell),
            // ---- The lobby.
            "lobby.ready" => {
                // (The lobby's answer is the mark: a build the settings can't play isn't made ready, and the line
                // says why.)
                if let Some(lobby) = self.session.as_mut().and_then(Session::lobby)
                    && !lobby.set_ready(!lobby.ready())
                {
                    shell.sound.play(Cue::Refused);
                }
                self.ended = None;
                shell.ui().global::<PlayView>().set_note_error(Default::default());
            }
            "lobby.first-to" | "lobby.custom-time" => self.step(shell, id, index, 1),
            "lobby.open" => {
                let open = !self.live.open;
                self.set_open(shell, open);
            }
            // The host's Ruleset opens its rulesets as a list; the joiner's VIEW is beside it.
            "lobby.ruleset" => {
                if let Some((picks, at)) = self.ruleset_choices(shell).filter(|_| self.is_host())
                    && let Some((_, ready)) = shell.ready()
                {
                    let graphics = ready.graphics(shell.i18n.code);
                    let names = Names::of(ready.content(), &graphics);
                    let shown = picks.iter().map(|p| rulesets::shown_name(&names, &p.ruleset.name)).collect();
                    shell.pick("lobby.ruleset", 0, shown, at);
                }
            }
            "lobby.rules" => self.open_rules(shell),
            "lobby.change" => {
                self.changing = true;
                shell.nav.open("change.", Key::new("change.builds", self.chosen as i32));
                shell.ui().global::<PlayView>().set_changing(true);
            }
            // Another build: any whose file reads. (One that can't be played as the match's settings stand is
            // the lobby's to say: it can't be made ready, and the line says why.)
            "change.builds" => {
                let index = index.max(0) as usize;
                if let Some(side) = self.builds.get(index).and_then(|b| b.side().cloned()) {
                    self.choose_build(shell, index);
                    // (Nothing of it is said to the other player; a ready mark was for the build before.)
                    if let Some(lobby) = self.session.as_mut().and_then(Session::lobby) {
                        lobby.set_side(side);
                    }
                    {
                        self.show_card(shell);
                    }
                    self.close_change(shell);
                } else {
                    shell.sound.play(Cue::Refused);
                }
            }
            "change.cancel" => self.close_change(shell),
            "lobby.leave" => self.leave(shell),
            // ---- The match's menu. (The practice session's beside an invite is Practice's to hear.)
            "menu.resume" => shell.close_menu(),
            "menu.controls" => self.set_page(shell, MatchPage::Controls),
            "menu.settings" => self.set_page(shell, MatchPage::Settings),
            "menu.back" => self.set_page(shell, MatchPage::Menu),
            "menu.delay" => {}
            // (The settings and the controls as the Settings screen changes them.)
            "menu.language" => settings::pick_language(shell, id),
            "menu.picture" => settings::pick_picture_size(shell, id),
            "menu.fullscreen" => self.set_full_screen(shell, !shell.settings.full_screen),
            "menu.graphics" => self.set_enhanced(shell, !shell.settings.enhanced),
            "menu.key" | "menu.button" => self.binding.ask(shell, index, id == "menu.button"),
            "menu.reset" => {
                self.binding.reset(shell);
                self.show_match(shell);
            }
            "menu.leave" if matches!(self.stage, Stage::Match { .. }) => self.reopen(shell, None),
            // ---- The match's result.
            "result.go" => self.go_on(shell),
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, id: &str, index: i32, by: i32) {
        if RulesPage::owns(id) {
            let outcome = self.rules.as_mut().map_or(Outcome::Nothing, |page| page.step(shell, id, index, by));
            return self.ruleset_outcome(shell, outcome);
        }
        match id {
            "build" => {
                let count = self.builds.len() as i32;
                if count > 0 {
                    self.choose_build(shell, (self.chosen as i32 + by).rem_euclid(count) as usize);
                }
            }
            // (Around from the last to the first, as the stepper went from best of three to a single battle.)
            "lobby.first-to" => {
                let (first, count) = (*FIRST_TO.start() as i32, FIRST_TO.len() as i32);
                self.set_first_to(shell, (first + (self.live.first_to - first + by).rem_euclid(count)) as u8);
            }
            "lobby.open" => self.set_open(shell, by > 0),
            "lobby.custom-time" => self.step_custom_time(shell, by),
            "lobby.ruleset" => self.pick_ruleset(shell, 0, by),
            "lobby.delay" | "menu.delay" => self.set_delay(shell, by),
            "menu.music" => self.set_levels(shell, (shell.settings.music as i32 + 5 * by).clamp(0, 100) as u32, shell.settings.effects),
            "menu.effects" => self.set_levels(shell, shell.settings.music, (shell.settings.effects as i32 + 5 * by).clamp(0, 100) as u32),
            "menu.language" => settings::set_language(shell, crate::lang::step(shell.i18n.code, by)),
            "menu.picture" => {
                settings::step_picture_size(shell, by);
                self.show_match(shell);
            }
            "menu.fullscreen" => self.set_full_screen(shell, by > 0),
            // (Enhanced is the first of the two.)
            "menu.graphics" => self.set_enhanced(shell, by < 0),
            "name.version" => self.step_version(shell, by),
            _ => {}
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, index: i32, choice: i32) {
        if RulesPage::owns(id) {
            let outcome = self.rules.as_mut().map_or(Outcome::Nothing, |page| page.choose(shell, id, index, choice));
            return self.ruleset_outcome(shell, outcome);
        }
        match id {
            "build" => self.choose_build(shell, choice.max(0) as usize),
            // The build's menu: from ⋯, or a right click on a row of the list (the press chose that build).
            "more" if self.part() == PlayPart::Builds => self.menu_chosen(shell, choice),
            "row-menu" if self.part() == PlayPart::Builds && self.naming.is_none() => self.open_menu(shell, &Key::new("builds", index)),
            "name.version" => self.choose_version(shell, choice.max(0) as usize),
            "lobby.open" => self.set_open(shell, choice == 1),
            "lobby.ruleset" => self.pick_ruleset(shell, choice, 0),
            "menu.music" => self.set_levels(shell, choice.clamp(0, 100) as u32, shell.settings.effects),
            "menu.effects" => self.set_levels(shell, shell.settings.music, choice.clamp(0, 100) as u32),
            "menu.language" => settings::choose_language(shell, choice),
            "menu.picture" => {
                settings::set_picture_size(shell, choice.max(0) as u32);
                self.show_match(shell);
            }
            "menu.fullscreen" => self.set_full_screen(shell, choice == 1),
            "menu.graphics" => self.set_enhanced(shell, choice == 0),
            _ => {}
        }
    }

    fn edit(&mut self, shell: &mut Shell, id: &str, _index: i32, text: &str, done: bool) {
        if RulesPage::owns(id) {
            let outcome = self.rules.as_mut().map_or(Outcome::Nothing, |page| page.edit(shell, id, text, done));
            return self.ruleset_outcome(shell, outcome);
        }
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        match id {
            "code" => {
                self.code = code_typed(text);
                view.set_code(code_shown(&self.code).into());
                // (What was wrong with the code before is no more.)
                if self.problem.take().is_some() {
                    view.set_message(self.notice.as_str().into());
                    view.set_message_quiet(true);
                }
            }
            "name.name" => self.edit_name(shell, text, done),
            "direct.address" => {
                if let Some(direct) = &mut self.direct {
                    direct.address = address_typed(text);
                    view.set_address(direct.address.as_str().into());
                    if direct.join_problem.take().is_some() {
                        view.set_join_message(Default::default());
                    }
                }
            }
            _ => {}
        }
    }

    /// Enter in the code's field joins with it, as JOIN does (the user: "pressing enter in the link code dialog
    /// box should confirm and connect"), and in the direct connection's address connects, as CONNECT does.
    fn submitted(&mut self, shell: &mut Shell, id: &str, index: i32) {
        match id {
            "code" => self.activate(shell, "join", index),
            "direct.address" => self.activate(shell, "direct.connect", index),
            _ => {}
        }
    }

    fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if let Some(page) = self.rules.as_mut().filter(|_| RulesPage::owns(id)) {
            return page.focused(shell, id, index);
        }
        // (A binding asked for is asked no more once the focus moves, as on the Settings screen.)
        self.binding.stop(shell);
        // The list's row under the cursor is the build chosen: the overview beside it follows.
        if id == "builds" {
            self.choose_build(shell, index.max(0) as usize);
        }
        // A box of the code's: the character the focus is on in it.
        if let Some(at) = id.strip_prefix("boxes.").and_then(|at| at.parse::<usize>().ok()) {
            self.set_box(shell, at, index.max(0) as usize);
        }
    }

    /// Delete on the builds' list asks whether the chosen build goes (and with a build's name being asked for,
    /// nothing else is done: the game stays). The shoulders, in the lobby: the next control and the one before
    /// (its row of settings can't be walked with Left and Right, which change them).
    fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        if self.naming.is_some() {
            return true;
        }
        let on_list = shell.nav.focused().is_some_and(|k| k.id == "builds" || k.id == "build");
        if action == Action::Delete && on_list && self.part() == PlayPart::Builds {
            self.ask_delete(shell);
            return true;
        }
        if !matches!(self.stage, Stage::Lobby) || shell.battle.is_some() || !matches!(action, Action::Previous | Action::Next) {
            return false;
        }
        shell.nav.tab(&shell.ui(), action == Action::Previous);
        shell.sound.play(Cue::Cursor);
        true
    }

    /// The builds' part on a desktop: Right from the list goes past the overview (no control) to EDIT, or ⋯
    /// where it can't be used; Tab goes through the zones in their order, the builds (+ NEW BUILD, the list),
    /// the chosen build (EDIT, ⋯), then the ways to play it online (HOST, the code, JOIN, DIRECT
    /// CONNECT). (On a phone the one column is that order already.)
    fn route(&mut self, shell: &mut Shell, action: Action) -> bool {
        if self.part() != PlayPart::Builds || shell.scale.phone || self.naming.is_some() || self.direct.is_some() || self.boxes.is_some() {
            return false;
        }
        let ui = shell.ui();
        match action {
            Action::Right if shell.nav.focused().is_some_and(|k| k.id == "builds") => shell.nav.focus_first(&ui, &["edit", "more"]),
            Action::NextFocus | Action::PreviousFocus => {
                shell.nav.tab_in(&ui, action == Action::PreviousFocus, &["new", "builds", "edit", "more", "invite", "code", "join", "direct"]);
                true
            }
            _ => false,
        }
    }

    /// In the code's boxes, a character typed goes into the box the focus is on, and the focus to the next. Over
    /// the match's menu Tab shows the builds while it is held, as in the match (`Keys::tab`), and moves no focus.
    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        if self.binding.key(shell, key) {
            self.show_match(shell);
            return true;
        }
        if key.code == KeyCode::Tab && self.menu_open(shell) {
            return true;
        }
        let (Some(rows), Some(at)) = (self.boxes, Play::box_focused(shell)) else { return false };
        let typed = key.text.as_deref().and_then(|t| t.chars().next()).map(|c| c.to_ascii_uppercase());
        let Some(row) = typed.and_then(|c| ALPHABET.find(c)) else { return false };
        self.set_box(shell, at, row);
        shell.nav.set_row(&code_box(at), row as i32);
        match at + 1 < CODE_LENGTH {
            true => shell.nav.focus(&code_box(at + 1), rows[at + 1] as i32),
            false => shell.nav.focus("boxes.join", 0),
        }
        true
    }

    /// A pad can't type a code: its confirm on the code's field opens the six boxes. (A pad's button for a binding
    /// asked for is the binding's.)
    fn button(&mut self, shell: &mut Shell, button: Button) -> bool {
        if self.binding.button(shell, button) {
            self.show_match(shell);
            return true;
        }
        let on_field = shell.nav.focused().is_some_and(|k| k.id == "code") && !shell.nav.editing();
        if button != shell.bindings.confirm_button() || !on_field || !matches!(self.stage, Stage::Idle | Stage::Meeting) || self.boxes.is_some() {
            return false;
        }
        shell.sound.play(Cue::Pick);
        self.open_boxes(shell);
        true
    }

    fn back(&mut self, shell: &mut Shell) -> bool {
        // (Over a match's result, Back goes on as CONTINUE does: the match is over, and the other player stays.)
        if self.result_shown() {
            self.go_on(shell);
        } else if self.rules.is_some() {
            self.close_rules(shell);
        } else if self.naming.is_some() {
            self.close_dialog(shell);
        } else if self.boxes.is_some() {
            self.close_boxes(shell);
        } else if self.direct.is_some() {
            self.close_direct(shell);
        } else if self.changing {
            self.close_change(shell);
        } else if matches!(self.stage, Stage::Lobby) {
            // (Leaving ends the link for both players: one key pressed by mistake shouldn't. The lobby's own
            // LEAVE leaves at once.)
            let ui = shell.ui();
            let view = ui.global::<PlayView>();
            let mut leave = crate::shell::button(&view.get_leave_word(), nettai_ui::ButtonKind::Ghost);
            leave.danger = true;
            shell.ask(Dialog {
                tag: LEAVE_LOBBY,
                title: view.get_leave_question().into(),
                message: String::new(),
                buttons: vec![crate::shell::button(&view.get_cancel_word(), nettai_ui::ButtonKind::Ghost), leave],
                focus: 0,
                cancel: 0,
            });
        } else if !matches!(self.stage, Stage::Idle) {
            self.leave(shell);
        } else {
            return false;
        }
        true
    }

    /// A row this screen added to the practice menu was chosen; a question this screen asked (whether a build goes,
    /// whether the lobby is left) was answered.
    fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) {
        if tag == rules_page::DELETE {
            if let Some(page) = self.rules.take() {
                match page.answered(shell, choice) {
                    Ok(outcome) => self.ruleset_outcome(shell, outcome),
                    Err(page) => self.rules = Some(page),
                }
            }
            return;
        }
        if tag == list::DELETE {
            if choice == 1 {
                self.delete(shell);
            }
            return;
        }
        // (Still in the lobby: a match may have been agreed meanwhile, or the lobby ended.)
        if tag == LEAVE_LOBBY {
            if choice == 1 && matches!(self.stage, Stage::Lobby | Stage::Loading { .. }) {
                self.leave(shell);
            }
            return;
        }
        if tag != WAIT_ROWS || !matches!(self.stage, Stage::Waiting) {
            return;
        }
        match choice {
            practice::EXIT => self.leave(shell),
            // (A phone's one row: STOP PRACTICE.)
            0 => practice::stop(shell),
            _ => {}
        }
    }

    /// A save dropped on the builds is made a build.
    fn dropped(&mut self, shell: &mut Shell, path: &std::path::Path) {
        if self.part() == PlayPart::Builds && self.naming.is_none() && self.direct.is_none() && self.boxes.is_none() {
            self.import(shell, &[path.to_path_buf()]);
        }
    }

    /// Saves or rulesets chosen in the system's dialog, which FROM A SAVE FILE… or IMPORT opened.
    fn chosen(&mut self, shell: &mut Shell, purpose: &str, paths: &[std::path::PathBuf]) {
        match purpose {
            list::SAVE_FILES => self.import(shell, paths),
            rules_page::RULESET_FILES => {
                if let Some(page) = self.rules.as_mut() {
                    let outcome = page.import_paths(shell, paths);
                    self.ruleset_outcome(shell, outcome);
                }
            }
            _ => {}
        }
    }

    /// The builds shown (Tab or Select held), over the match or its menu: Up and Down scroll them a step, Page Up
    /// and Down a page less a step, Home and End to their ends. They take them all while they show, at an end too:
    /// the menu's focus stays where it was under them, and over the match the battle hears none of them (the
    /// shell keeps its keys: [`Play::set_peek`]).
    fn scroll(&mut self, shell: &mut Shell, action: Action) -> bool {
        let Some(by) = Scroll::of(action).filter(|_| self.peeking()) else { return false };
        if Self::scroll_peek(shell, by) {
            shell.sound.play(Cue::Cursor);
        }
        true
    }

    /// The right stick and Shift's scrolls: the builds shown over the match, while they show; else, on the
    /// builds, the chosen build's overview where it is taller than its frame (its column; the list beside it
    /// scrolls with the focus).
    fn scroll_view(&mut self, shell: &mut Shell, by: Scroll) -> Option<bool> {
        if self.peeking() {
            return Some(Self::scroll_peek(shell, by));
        }
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        // (As the overview shows: a readable build chosen, on the builds with nothing open over them.)
        let shown = self.part() == PlayPart::Builds && !shell.scale.phone && view.get_chosen() >= 0 && view.get_readable();
        let over = self.naming.is_some() || self.direct.is_some() || self.boxes.is_some();
        if !shown || over || view.get_sheet_span() <= 0.5 {
            return None;
        }
        let at = (-view.get_sheet_y()).min(view.get_sheet_span());
        let to = by.to(at, view.get_sheet_span(), view.get_sheet_view());
        if (to - at).abs() < 0.01 {
            return Some(false);
        }
        view.set_sheet_y(-to);
        Some(true)
    }

    fn battle(&mut self, shell: &mut Shell, event: BattleEvent) {
        match (event, &self.stage) {
            // The match begins: it is recorded from its first tick, by its agreement.
            (BattleEvent::Started, Stage::Loading { .. }) => {
                self.stage = Stage::Match { result: None };
                // (Not in the screenshot pass, which keeps nothing, unless it is told where the replays go.)
                if (!shell.still || std::env::var_os("NETTAI_REPLAYS").is_some())
                    && let (Some(agreed), Some((game, ready)), Some(b)) = (&self.agreed, shell.ready(), shell.battle.as_mut())
                    && let Some(recorder) = recorder(ready.content(), agreed, &game)
                    && let Err(why) = b.stage.player.record(recorder)
                {
                    crate::errors::log("recording the match", &why);
                }
            }
            // The match stopped with the link still up (the other player left it, or went silent): both are
            // back in the lobby, which says why.
            (BattleEvent::Stopped(reason), Stage::Match { .. }) => self.reopen(shell, UserError::of_stop(reason)),
            (BattleEvent::Finished(_), Stage::Match { .. }) => self.reopen(shell, None),
            _ => {}
        }
    }

    /// The loop's first half: what the link brought, to the lobby.
    fn frame(&mut self, shell: &mut Shell, now: Instant, _shown: bool) {
        tour::frame(shell, now);
        let fed = match &mut self.session {
            Some(session) => session.feed(now).map(|heard| (heard, session.reconnecting().is_some())),
            None => return,
        };
        match fed {
            // (The pass's count of what a link saw, where it keeps one.)
            Ok((heard, down)) if shell.still => tour::heard(now, heard, down),
            Ok(_) => {}
            Err(error) => self.link_failed(shell, &error),
        }
        // (The keys are the practice battle's beside an invite: the cursor is on none of the panel's buttons.)
        if matches!(self.stage, Stage::Waiting) && shell.battle_has_keys() {
            let pad = shell.nav.pad();
            shell.nav.show_cursor(false, pad);
        }
    }

    /// The loop's second half, after the battle's ticks: the lobby polled, what it says shown and sent.
    fn frame_end(&mut self, shell: &mut Shell, now: Instant) {
        if matches!(self.stage, Stage::Lobby) {
            self.follow_content(shell);
        }
        let Some(session) = &mut self.session else { return };
        // This side's loading, said to the other.
        if let Stage::Loading { gate, .. } = &self.stage
            && let Some(lobby) = session.lobby()
        {
            let mine = gate.mine.get();
            lobby.set_loading(mine);
            if mine >= 1.0 {
                lobby.set_loaded();
            }
        }
        let stands = session.poll(now);
        match (stands, &self.stage) {
            // The server gave the invite its code: the wait for the other player.
            (Stands::Meeting, Stage::Meeting) if session.way == Way::Invite && session.code().is_some() => self.wait(shell),
            // The other player has come.
            (Stands::Meeting, Stage::Meeting | Stage::Waiting) if session.is_open() => self.enter_lobby(shell),
            (Stands::Meeting, _) => {}
            (Stands::Failed(failure), _) => {
                let error = UserError::of_lobby(&failure);
                return self.ended(shell, error);
            }
            // A match is agreed.
            (Stands::Loading(_), Stage::Lobby) => {
                if let Some(agreed) = session.agreement(now) {
                    self.load(shell, agreed);
                }
            }
            (Stands::Loading(seed), Stage::Loading { seed: loading, gate }) if seed == *loading => {
                let theirs = session.lobby().map_or(0.0, |l| l.their_loading());
                gate.theirs.set(theirs);
            }
            // Both sides have loaded: the match's driver, on the channel the lobby gives it once.
            (Stands::Start(seed), Stage::Loading { seed: loading, gate }) if seed == *loading => {
                if !gate.go.get()
                    && let (Some(agreed), Some(channel)) = (&self.agreed, session.channel())
                {
                    let options = NetOptions { input_delay: shell.settings.input_delay, ..NetOptions::default() };
                    *gate.driver.borrow_mut() = Some(Box::new(NetPlayer::new(channel, agreed.side, agreed.set.clone(), options)));
                    gate.theirs.set(1.0);
                    gate.go.set(true);
                }
            }
            // The other player took their ready back as the match was agreed: the lobby again.
            (Stands::Lobby, Stage::Loading { .. }) => self.reopen(shell, None),
            (Stands::Lobby | Stands::Loading(_) | Stands::Start(_), _) => {}
            (Stands::Playing, Stage::Match { result }) => {
                let over = shell.battle.as_ref().and_then(|b| b.stage.player.result());
                let ended = *result;
                let their_back = session.lobby().is_some_and(|l| l.their_reopened());
                match (over, ended) {
                    // The set is over, and the game has shown its result: nettai's, over the picture, until the
                    // player goes on or a few seconds pass; then the lobby.
                    (Some(over), None) => {
                        let shown = self.show_result(shell, over);
                        self.stage = Stage::Match { result: Some(Ended { since: now, shown }) };
                    }
                    (Some(_), Some(Ended { since, shown })) if now.saturating_duration_since(since) >= if shown { RESULT_STAYS } else { RESULT_SHOWN } => self.reopen(shell, None),
                    (Some(_), Some(Ended { shown: true, .. })) => self.result_sounds(shell, now),
                    // The other player is back in the lobby and this one's set isn't over: they ended the
                    // match on their side.
                    (None, _) if their_back => self.reopen(shell, UserError::of_stop(StopReason::OpponentLeft)),
                    _ => {}
                }
            }
            (Stands::Playing, _) => {}
        }
        // What the lobby and the match's player have to say goes out.
        let sent = match &mut self.session {
            Some(session) => session.send(),
            None => return,
        };
        if let Err(error) = sent {
            return self.link_failed(shell, &error);
        }
        if matches!(self.stage, Stage::Lobby | Stage::Waiting) {
            self.show_live(shell);
        }
        // In a match: both builds while Tab (Select on a pad) is held, over the match and over its menu; the menu
        // opens on its first page.
        if matches!(self.stage, Stage::Match { .. }) {
            let held = shell.keys.tab || (shell.pad.held(&shell.bindings) | shell.touch.held()) & keys::SELECT != 0;
            let shown = shell.battle_has_keys() || (self.menu_open(shell) && shell.dialog.is_none());
            self.set_peek(shell, held && shown);
            if self.page != MatchPage::Menu && !shell.battle.as_ref().is_some_and(|b| b.menu_open()) {
                self.binding.stop(shell);
                self.page = MatchPage::Menu;
                shell.ui().global::<PlayView>().set_page(MatchPage::Menu);
            }
        }
    }
}

impl Play {
    /// The match's menu is open over this screen's battle.
    fn menu_open(&self, shell: &Shell) -> bool {
        matches!(self.stage, Stage::Match { .. }) && shell.battle.as_ref().is_some_and(|b| b.owner == ScreenId::Play && b.menu_open())
    }

    /// A control's problem is said (with the refusal's sound), or what it started shows.
    fn refuse_or_show(&mut self, shell: &mut Shell) {
        if self.problem.is_some() {
            shell.sound.play(Cue::Refused);
        }
        self.show(shell);
    }

    fn close_change(&mut self, shell: &mut Shell) {
        self.changing = false;
        shell.nav.close("change.");
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_changing(false);
        view.set_build(self.build().map(|b| b.row.clone()).unwrap_or_default());
    }

    /// The link's own error: before the other player came, the control that asked says it; after, it ends
    /// what the two were doing.
    fn link_failed(&mut self, shell: &mut Shell, error: &nettai_rtc::Error) {
        let doing = match self.stage {
            Stage::Idle | Stage::Meeting => "reaching the other player",
            Stage::Waiting => "waiting for the other player",
            Stage::Lobby | Stage::Loading { .. } => "in the lobby",
            Stage::Match { .. } => "in a match",
        };
        let detail = format!("{doing} ({})", self.session.as_ref().map(Session::describe).unwrap_or_default());
        let said = UserError::of_link(&detail, error);
        match self.stage {
            Stage::Idle | Stage::Meeting => self.not_met(shell, said),
            _ => self.ended(shell, Some(said)),
        }
    }
}

/// The match's own controls as `shell`'s bindings have them, for the match menu's footer (the user: "the menu footer
/// should show in-game controls"): moving, by the four directions' keys and the D-pad where the pad's are it; A and
/// B; L and R, one hint where the pad's are the shoulders (the footer draws them as a pair); the builds, Tab and the
/// pad's button bound to SELECT; the menu, Esc and the key bound to START, and the pad's.
fn match_keys(shell: &Shell, view: &PlayView) -> Vec<Hint> {
    let (keys, pad) = (&shell.bindings.keys, &shell.bindings.pad);
    let at = button_index;
    let one = |button: u16, text: &str| Hint { key: key_cap(keys[at(button)]).into(), pad: pad_face(pad[at(button)]), text: text.into(), ..Default::default() };
    let ways = [keys::UP, keys::DOWN, keys::LEFT, keys::RIGHT];
    let caps: Vec<&str> = ways.iter().map(|&b| key_cap(keys[at(b)])).collect();
    let moving = Hint {
        // (Single glyphs together, "↑↓←→"; longer names apart.)
        key: caps.join(if caps.iter().all(|c| c.chars().count() == 1) { "" } else { " " }).into(),
        pad: match ways.iter().all(|&b| pad_face(pad[at(b)]) == PadButton::Move) {
            true => PadButton::Move,
            false => PadButton::None,
        },
        text: view.get_move_word(),
        ..Default::default()
    };
    let mut out = vec![moving, one(keys::A, "A"), one(keys::B, "B")];
    let shoulders = [keys::L, keys::R].iter().all(|&b| pad_face(pad[at(b)]) == PadButton::Shoulders);
    match shoulders {
        true => out.push(Hint { key2: key_cap(keys[at(keys::R)]).into(), text: "L R".into(), ..one(keys::L, "") }),
        false => out.extend([one(keys::L, "L"), one(keys::R, "R")]),
    }
    out.push(Hint { key: "Tab".into(), pad: pad_face(pad[at(keys::SELECT)]), text: view.get_show_builds_word(), ..Default::default() });
    out.push(Hint { key: "Esc".into(), key2: key_cap(keys[at(keys::START)]).into(), pad: pad_face(pad[at(keys::START)]), text: view.get_menu_word() });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A code is kept as its characters, in capitals, six at most, whatever it was typed with; and shown in
    /// threes.
    #[test]
    fn a_code_typed_in_any_case_with_spaces_or_hyphens() {
        assert_eq!(code_typed("k7m-2qx"), "K7M2QX");
        assert_eq!(code_typed(" k7m 2qx "), "K7M2QX");
        assert_eq!(code_typed("K7M2QXYZ"), "K7M2QX");
        assert_eq!(code_typed("k7"), "K7");
        assert_eq!((code_shown("K7M2QX"), code_shown("K7M2"), code_shown("K7M"), code_shown("")), ("K7M-2QX".to_string(), "K7M-2".to_string(), "K7M".to_string(), String::new()));
        // (What the link library takes: the same code.)
        assert_eq!(nettai_rtc::signal::normalize_code(&code_shown(&code_typed("k7m 2qx"))), Some("K7M2QX".to_string()));
    }

    /// JOIN's field takes an invite's code alone: whatever is typed is kept as a code's characters (capitals,
    /// at most six), so an address typed there is no address (the user: "the box with the combined code and
    /// address sucks"; addresses are the direct connection's dialog's).
    #[test]
    fn joins_field_takes_a_code_only() {
        assert_eq!(code_typed("k7m2qx"), "K7M2QX");
        assert_eq!(code_typed("K7M-2QX"), "K7M2QX");
        assert_eq!(code_typed(" k7m 2qx "), "K7M2QX");
        assert_eq!(code_typed("k7m2qxyz"), "K7M2QX");
        assert_eq!(code_shown(&code_typed("k7m2qx")), "K7M-2QX");
        // (An address typed there is cut to a code's characters, not joined as an address.)
        assert_eq!(code_typed("192.168.1.20:47474"), "192168");
        assert_eq!(code_typed("[::1]:47474"), "147474");
    }

    /// The direct connection's dialog's JOIN takes a host's address: a host or an IP with its port, an IPv6 one
    /// in brackets; anything else is refused as an address.
    #[test]
    fn directs_join_takes_an_address() {
        let address = |a: &str| Ok(a.to_string());
        assert_eq!(direct_address("192.168.1.20:47474"), address("192.168.1.20:47474"));
        assert_eq!(direct_address(" 127.0.0.1:7000 "), address("127.0.0.1:7000"));
        assert_eq!(direct_address("my-host.local:47474"), address("my-host.local:47474"));
        assert_eq!(direct_address("localhost:47474"), address("localhost:47474"));
        assert_eq!(direct_address("[::1]:47474"), address("[::1]:47474"));
        assert_eq!(direct_address("[fe80::1%en0]:9000"), address("[fe80::1%en0]:9000"));
        for bad in ["", "k7m2qx", "192.168.1.20", "192.168.1.20:", ":47474", "host:port", "host:0", "host:70000", "::1:47474", "[::1]", "[::1]47474", "[]:47474"] {
            assert_eq!(direct_address(bad), Err(UserError::BadAddress), "{bad:?}");
        }
        // (What the field keeps: no spaces, 80 characters at most.)
        assert_eq!(address_typed("192.168.1.20 :47474\n"), "192.168.1.20:47474");
        assert_eq!(address_typed(&"a".repeat(100)).len(), 80);
    }
}

