//! Play (docs/design/app-ui.md §6: "Play", "In an online match", "After a match"): the player's builds, and
//! online play.
//!
//! The player's builds of the selected game are here (`list`: NEW BUILD and IMPORT SAVE… over them, the chosen
//! one's overview beside them with EDIT, which opens it in Builds' editor, DUPLICATE, RENAME and DELETE). The
//! player picks one and makes a link with it: an invite (a room of the signaling server, whose code the server
//! picks), a code typed, or a direct connection. While they wait
//! for the other player a practice session runs beside the invite's code (`crate::screens::practice`'s, with
//! this screen's rows in its menu). Once the link is open the two are in
//! the lobby (`session`: one `Lobby` a link, for as long as they stay together): the match's shared settings,
//! each one's ready mark, the connection's quality. Both ready, the match is agreed and made ready behind the
//! shell's two cards, each side saying how far it is; both loaded, it starts. When its set is over, nettai's
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
    use std::time::Instant;

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
use list::Naming;
use crate::errors::UserError;
use crate::games::Names;
use crate::input::{Action, KeyPress, key_cap, pad_cap};
use crate::nav::Key;
use crate::screens::{Screen, ScreenId, practice};
use crate::settings::MAX_INPUT_DELAY;
use crate::shell::{Dialog, Shell};
use crate::sound::Cue;
use gamepad_facade::Button;
use nettai_battle::BattleResult;
use nettai_battle::input::keys;
use nettai_frontend::driver::{Driver, StopReason};
use nettai_frontend::lobby::{Agreement, Settings};
use nettai_frontend::netplay::{NetOptions, NetPlayer};
use nettai_frontend::player::FRAME_RATE;
use nettai_frontend::replay::Recorder;
use nettai_match::Side;
use nettai_rtc::signal::{ALPHABET, CODE_LENGTH};
use nettai_ui::{BattleKind, BuildSheet, LobbyNote, MatchOutcome, MatchPage, PadButton, PlayBuild, PlayControl, PlayPart, PlayView, VersusCard};
use session::{Net, Session, Stands, Way};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

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

/// How far Up and Down scroll a build's page in the match's menu: two of its chips' rows.
const SHEET_STEP: f32 = 56.0;

/// A row this screen adds at the top of the practice menu while the other player is waited for. (Its number is
/// what the UI words it by: `PlayView.wait-row`.)
#[derive(Clone, Copy, PartialEq)]
enum WaitRow {
    /// Copy the invite's code (a direct host's address).
    Copy = 0,
    /// Stop practice, for the invite again: where the session fills the window (a phone).
    Stop = 1,
    /// Cancel the invite.
    Cancel = 2,
}

/// What the settings remember for Play: the round wins the last match's set was first to, and the build last
/// chosen, by game.
const REMEMBERED_FIRST_TO: &str = "play.first-to";

fn remembered_build(game: &str) -> String {
    format!("play.build.{game}")
}

/// The settings a lobby of `game` opens with: a set first to `wins` round wins, each place picked from the match's
/// seed, its background from the background pool, every game's (docs/app.md §1, "The battle's background"), shared
/// by both players as the round's settings are.
pub(crate) fn settings(game: &str, wins: u8) -> Settings {
    Settings { backgrounds: nettai_match::BackgroundPick::Pool, ..Settings::new(game, wins) }
}

/// Play opened for screen `from` (Practice's CREATE A BUILD), Back from its builds going back to it.
pub fn open(shell: &mut Shell, from: ScreenId) {
    shell.go(ScreenId::Play);
    shell.tell(ScreenId::Play, &crate::screens::opened_tag(from), 0);
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
    patch_cards: bool,
    /// What is wrong with this player's build as the settings would play it: the first thing.
    problem: Option<nettai_match::check::Problem>,
    closed: bool,
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
    /// The screen that opened this one (`open`), which Back from the builds goes back to; and whether the editor
    /// is being opened from here (EDIT), whose Back comes back here: leaving for it keeps that.
    back_to: Option<ScreenId>,
    to_editor: bool,
    /// The code typed (its characters alone), and what the space beside JOIN says.
    code: String,
    problem: Option<UserError>,
    direct: Option<Direct>,
    /// The code's six boxes are open (a pad can't type): the character each is on, by its place in the
    /// alphabet codes are written in.
    boxes: Option<[usize; CODE_LENGTH]>,
    /// The lobby's list of builds is open.
    changing: bool,
    session: Option<Session>,
    stage: Stage,
    /// The match agreed last: what the match's panels show.
    agreed: Option<Agreement>,
    /// Why the match before ended, for the lobby's line.
    ended: Option<UserError>,
    live: Live,
    page: MatchPage,
    peek: bool,
    /// Counted for the UI: each player's turns to ready, the copies.
    ready_seq: i32,
    their_ready_seq: i32,
    copied_seq: i32,
    /// The rows this screen added to the practice menu of the session on, in their order.
    wait_rows: Vec<WaitRow>,
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
            back_to: None,
            to_editor: false,
            code: String::new(),
            problem: None,
            direct: None,
            boxes: None,
            changing: false,
            session: None,
            stage: Stage::Idle,
            agreed: None,
            ended: None,
            live: Live::default(),
            page: MatchPage::Menu,
            peek: false,
            ready_seq: 0,
            their_ready_seq: 0,
            copied_seq: 0,
            wait_rows: Vec::new(),
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

/// A code as it is kept while typed: its letters in capitals and its digits, at most a code's six; whatever
/// else was typed (spaces, hyphens) left out.
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
    fn part(&self) -> PlayPart {
        match self.stage {
            Stage::Idle | Stage::Meeting => PlayPart::Builds,
            Stage::Waiting => PlayPart::Waiting,
            Stage::Lobby | Stage::Loading { .. } | Stage::Match { .. } => PlayPart::Lobby,
        }
    }

    /// The part's name for the focus's memory, and its main control.
    fn view(&self) -> (&'static str, &'static str) {
        match self.part() {
            PlayPart::Builds => ("play", "builds"),
            PlayPart::Waiting => ("play.waiting", "wait.copy"),
            PlayPart::Lobby => ("play.lobby", "lobby.ready"),
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

    /// Read the selected game's builds (`builds::shelf`), each checked by the game's rules: the one chosen before
    /// chosen again, else the one remembered.
    fn read_builds(&mut self, shell: &Shell, game: &str) {
        let before = self.build().map(|b| b.name().to_string()).or_else(|| shell.settings.remembered.get(&remembered_build(game)).cloned());
        let Some(kit) = builds::kit(shell) else {
            self.builds = Vec::new();
            return;
        };
        // (The screenshot pass lists builds of its own: its pictures don't depend on what the player keeps.)
        self.builds = match tour::builds(shell.still, kit.content(), game) {
            Some(made) => {
                let ui = shell.ui();
                let graphics = kit.ready.graphics(shell.lang);
                let names = Names::of(kit.content(), &graphics);
                let words = words::Words { ui: &ui, kit: &kit, names: &names };
                made.into_iter()
                    .map(|(name, side)| {
                        let listed = builds::store::Listed { path: Default::default(), name: name.clone(), text: String::new() };
                        shelf::shelved(&words, listed, Some(builds::store::Build::new(kit.content(), &name, side)))
                    })
                    .collect()
            }
            None => shelf::load(shell, &kit),
        };
        self.builds_of = Some(game.to_string());
        self.chosen = before.and_then(|name| self.builds.iter().position(|b| b.name() == name)).unwrap_or(0);
    }

    /// The chosen build's overview, beside the list (made again only when the build or the language shown
    /// changed: the builds are read each time the screen is shown), and whether it reads as one.
    fn show_sheet(&mut self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_readable(self.build().is_some_and(|b| b.build.is_some()));
        let key = self.build().map(|b| (b.name().to_string(), b.side().cloned(), shell.lang));
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

    /// This player's card in the lobby: the build as the match's settings would play it.
    fn show_card(&mut self, shell: &Shell) {
        let side = self.session.as_mut().and_then(Session::lobby).map(|lobby| lobby.side_as_played());
        let (Some(side), Some((game, ready)), Some(build)) = (side, shell.ready(), self.build()) else { return };
        let graphics = ready.graphics(shell.lang);
        let names = Names::of(ready.content(), &graphics);
        let card = PlayBuild { name: build.name().into(), navi: names.navi(side.navi(ready.content())).into(), version: builds::version_tag(shell, &game, &ready, &side) };
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
    /// there is no build), and nothing is made.
    fn side(&self, shell: &Shell) -> Result<Side, UserError> {
        match self.build() {
            None => Err(UserError::NoBuild),
            Some(build) if build.usable() => Ok(build.side().cloned().expect("a usable build has its side")),
            // (What is wrong, in the player's words: the editor's for each kind.)
            Some(build) => {
                let reason = build.side().and_then(|side| builds::kit(shell).and_then(|kit| words::first_problem(shell, &kit, side)));
                Err(UserError::BuildUnusable { reason: reason.filter(|r| !r.is_empty()) })
            }
        }
    }

    // ---- Making a link -----------------------------------------------------------------------------------

    /// Make a link `way`, with the chosen build. What keeps it from being made is the control's problem.
    fn connect(&mut self, shell: &mut Shell, way: Way) -> Result<(), UserError> {
        self.drop_session(shell);
        self.notice.clear();
        self.side(shell)?;
        let doing = match &way {
            Way::Invite => "making an invite",
            Way::Code(_) => "joining by a code",
            Way::Host => "hosting directly",
            Way::Address(_) => "joining directly",
        };
        let session = Session::open(way, &Play::net()).map_err(|e| UserError::of_link(doing, &e))?;
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
        self.agreed = None;
        self.changing = false;
        shell.nav.close("change.");
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

    /// The other player is waited for: the invite's code (or this machine's address) shows, and a practice
    /// session starts beside it. (On a phone it starts when the player asks: there it fills the window.)
    fn wait(&mut self, shell: &mut Shell) {
        self.stage = Stage::Waiting;
        self.direct = None;
        shell.nav.close("direct.");
        self.show(shell);
        if !shell.scale.phone {
            self.train(shell);
        }
    }

    /// A practice session for the wait (Practice's own: `practice::start`): this player's build against a copy
    /// of it that stands still, with this screen's rows at the top of the practice menu. It is beside the
    /// invite, in the screen's `Picture`. On a phone a battle has its pad only where it fills the window, so
    /// there it does, and a row of the menu stops it for the invite again.
    fn train(&mut self, shell: &mut Shell) {
        let (Some(game), Ok(side)) = (shell.game().map(str::to_string), self.side(shell)) else { return };
        let name = self.builds.get(self.chosen).map(|b| b.name().to_string()).unwrap_or_default();
        let full = shell.scale.phone;
        self.wait_rows = match full {
            true => vec![WaitRow::Copy, WaitRow::Stop, WaitRow::Cancel],
            false => vec![WaitRow::Copy, WaitRow::Cancel],
        };
        let direct = self.session.as_ref().is_some_and(|s| s.way == Way::Host);
        let view = shell.ui();
        let view = view.global::<PlayView>();
        let rows = self.wait_rows.iter().map(|row| view.invoke_wait_row(*row as i32, direct).to_string()).collect();
        let mut start = practice::Start::beside(&game, practice::Build { name, side }, ScreenId::Play, WAIT_ROWS, rows);
        start.full = full;
        if shell.still {
            start.seed = 7;
        }
        // (Why it can't start is in the log: the wait goes on without it.)
        let _ = practice::start(shell, start);
    }

    // ---- The lobby ---------------------------------------------------------------------------------------

    /// The link is open: the lobby over it, the practice battle ended.
    fn enter_lobby(&mut self, shell: &mut Shell) {
        let (Some((game, ready)), Ok(side)) = (shell.ready(), self.side(shell)) else {
            return self.ended(shell, Some(UserError::Other));
        };
        let first_to = shell.settings.remembered.get(REMEMBERED_FIRST_TO).and_then(|r| r.parse().ok()).filter(|n| FIRST_TO.contains(n)).unwrap_or(nettai_match::TRIPLE_BATTLE);
        // (The screenshot pass's matches are the same from run to run.)
        let entropy = tour::entropy().unwrap_or_else(nettai_frontend::lobby::entropy);
        let made = self.session.as_mut().is_some_and(|s| s.make_lobby(ready.content(), settings(&game, first_to), side, &shell.settings.name, entropy));
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
            VersusCard { role: view.get_you(), name: shell.settings.name.as_str().into(), build: mine.name.clone(), version: mine.version.clone(), navi: mine.navi.clone(), progress: 0.0 },
            // (The other player's card carries their name alone.)
            VersusCard { role: view.get_opponent(), name: agreed.names[other].as_str().into(), ..VersusCard::default() },
        ];
        let gate = Rc::new(Gate::default());
        let (said, made) = (gate.clone(), gate.clone());
        shell.settings.remembered.insert(REMEMBERED_FIRST_TO.into(), agreed.settings.first_to().to_string());
        shell.save_settings();
        shell.start_battle(BattleStart {
            owner: ScreenId::Play,
            kind: BattleKind::Online,
            game: game.clone(),
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

    /// The lobby's settings as the player's controls change them.
    fn set_first_to(&mut self, wins: u8) {
        if let Some(lobby) = self.session.as_mut().and_then(Session::lobby) {
            lobby.set_first_to(wins);
        }
        self.ended = None;
    }

    fn set_closed(&mut self, closed: bool) {
        if let Some(lobby) = self.session.as_mut().and_then(Session::lobby) {
            lobby.set_closed_builds(closed);
        }
        self.ended = None;
    }

    fn set_patch_cards(&mut self, on: bool) {
        if let Some(lobby) = self.session.as_mut().and_then(Session::lobby) {
            lobby.set_patch_cards(on);
        }
        self.ended = None;
    }

    fn set_delay(&mut self, shell: &mut Shell, by: i32) {
        shell.settings.input_delay = (shell.settings.input_delay as i32 + by).clamp(0, MAX_INPUT_DELAY as i32) as u32;
        shell.save_settings();
        if let Some(b) = &mut shell.battle {
            b.stage.player.set_input_delay(shell.settings.input_delay);
        }
        shell.ui().global::<PlayView>().set_input_delay(shell.settings.input_delay as i32);
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

    /// The lobby's own state, told to the UI where it changed.
    fn show_live(&mut self, shell: &mut Shell) {
        let Some(session) = &mut self.session else { return };
        let reconnecting = session.reconnecting();
        let ping = session.ping();
        // (The screenshot pass shows one round trip: what two players of one process measure moves about.)
        let steady = tour::steady_ping(shell.still).filter(|_| ping.is_some());
        let Some(lobby) = session.lobby() else { return };
        let (ready, their_ready) = (lobby.ready(), lobby.their_ready());
        let live = Live {
            their_present: lobby.their_present(),
            their_name: lobby.their_name().unwrap_or_default().to_string(),
            ready,
            their_ready,
            first_to: lobby.settings().first_to() as i32,
            patch_cards: lobby.settings().patch_cards,
            closed: lobby.settings().closed_builds,
            bars: if reconnecting.is_some() { 0 } else { steady.map_or(ping.map_or(0, |p| p.quality().bars() as i32), |s| s.0) },
            ping: if reconnecting.is_some() { -1 } else { steady.map_or(ping.map_or(-1, |p| p.ms.round() as i32), |s| s.1) },
            // (A build the settings can't play: the lobby's own check, typed. Until the Builds screen words each
            // kind, the line says only that it can't be used so.)
            problem: lobby.problems().first().cloned(),
            note: Some(match (reconnecting, ready, their_ready) {
                (Some(_), ..) => LobbyNote::Reconnecting,
                (None, ..) if !lobby.problems().is_empty() => LobbyNote::Unusable,
                (None, false, true) => LobbyNote::You,
                (None, true, false) => LobbyNote::Them,
                (None, false, false) | (None, true, true) => LobbyNote::None,
            }),
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
        view.set_patch_cards(live.patch_cards);
        // (The build's problem in the player's words: the Builds screen's for each kind.)
        if live.problem != self.live.problem || live.patch_cards != self.live.patch_cards {
            let said = match (&live.problem, shell.ready()) {
                (Some(problem), Some((game, ready))) => Some(words::problem(shell, &game, &ready, problem)),
                _ => None,
            };
            let line = said.map(|said| UserError::BuildUnusable { reason: Some(said).filter(|s| !s.is_empty()) }.line(&ui));
            view.set_note_problem(line.unwrap_or_default().into());
            self.show_card(shell);
        }
        view.set_closed(live.closed);
        view.set_bars(live.bars);
        view.set_ping(live.ping);
        view.set_note(live.note.unwrap_or(LobbyNote::None));
        view.set_note_seconds(live.seconds);
        self.live = live;
    }

    /// What the match's menu and panels show: both builds (the other player's has no name here: none was
    /// ever sent), whether they are closed, the controls, the levels.
    fn show_match(&self, shell: &Shell) {
        let (Some(agreed), Some(kit)) = (&self.agreed, builds::kit(shell)) else { return };
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        // (The agreement's sides are as the match plays them: without their patch cards where those are off.
        // With closed builds the other player's isn't shown, and isn't made.)
        let (me, other) = (agreed.side, 1 - agreed.side);
        view.set_mine(overview::sheet(shell, &kit, &view.get_you(), &shell.settings.name, &agreed.sides[me]));
        view.set_theirs(match agreed.closed_builds() {
            true => BuildSheet::default(),
            false => overview::sheet(shell, &kit, &view.get_opponent(), &agreed.names[other], &agreed.sides[other]),
        });
        view.set_closed_match(agreed.closed_builds());
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
        view.set_music(shell.settings.music as i32);
        view.set_effects(shell.settings.effects as i32);
        view.set_input_delay(shell.settings.input_delay as i32);
        view.set_page(self.page);
    }

    fn set_page(&mut self, shell: &mut Shell, page: MatchPage) {
        self.page = page;
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        view.set_page(page);
        view.set_sheet_y(0.0);
        // (The page's main control: the menu's first item, a page's Back.)
        shell.nav.focus(if page == MatchPage::Menu { "menu.resume" } else { "menu.back" }, 0);
    }

    fn set_peek(&mut self, shell: &Shell, peek: bool) {
        if peek != self.peek {
            self.peek = peek;
            shell.ui().global::<PlayView>().set_peek(peek);
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
/// (an online match, from this player's side, by the names the lobby had, closed builds where they were
/// agreed). None if the file can't be made (why is in the log).
fn recorder(content: &std::sync::Arc<nettai_battle::Content>, agreed: &Agreement, game: &str) -> Option<Recorder> {
    let dir = crate::paths::replays();
    let made = (|| -> Result<Recorder, String> {
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let stamp = chrono::DateTime::from_timestamp(when as i64, 0).map(|t| t.with_timezone(&chrono::Local).format("%Y%m%d-%H%M%S").to_string()).unwrap_or_default();
        let path = dir.join(format!("{stamp}-{game}.ntrp"));
        let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Recorder::of_agreement(Box::new(std::io::BufWriter::new(file)), content, agreed, when)
    })();
    made.map_err(|why| crate::errors::log("making a match's replay file", &why)).ok()
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
        if shell.ready().is_none() {
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
        view.set_to_practice(self.back_to == Some(ScreenId::Practice));
        if let Some(kit) = builds::kit(shell) {
            self.show_versions(shell, &kit);
        }
        self.show_dialog(shell);
        view.set_code(code_shown(&self.code).into());
        self.show_boxes(shell);
        // The space beside JOIN: a problem, or that a link is being made, or what an import made.
        let meeting = matches!(self.stage, Stage::Meeting);
        let asked_directly = self.session.as_ref().is_some_and(|s| matches!(s.way, Way::Address(_)));
        view.set_message(match &self.problem {
            Some(problem) => problem.line(&ui).into(),
            None if meeting && !asked_directly => view.get_connecting(),
            None => self.notice.as_str().into(),
        });
        view.set_message_quiet(self.problem.is_none());
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
        view.set_invite(self.invite().into());
        view.set_invite_direct(self.session.as_ref().is_some_and(|s| s.way == Way::Host));
        view.set_room(self.session.as_ref().and_then(|s| s.code()).map(code_shown).unwrap_or_default().into());
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
        // (Left for the editor through EDIT, whose Back comes back here: whoever opened this one still is.)
        if !std::mem::take(&mut self.to_editor) {
            self.back_to = None;
        }
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        // (A control of a part that isn't shown any more does nothing: a press may come as the part changes.)
        let part = self.part();
        let of_its_part = match id {
            "builds" | "build" | "new" | "import" | "edit" | "duplicate" | "rename" | "delete" | "invite" | "join" | "direct" => part == PlayPart::Builds,
            "wait.copy" | "wait.cancel" | "wait.train" => part == PlayPart::Waiting,
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
            "import" => self.import_chosen_file(shell),
            "edit" => self.edit_chosen(shell),
            "duplicate" => self.duplicate(shell),
            "rename" => self.ask_rename(shell),
            "delete" => self.ask_delete(shell),
            "name.confirm" => self.confirm(shell),
            "name.cancel" => self.close_dialog(shell),
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
            "direct" => self.open_direct(shell),
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
                let address = self.direct.as_ref().map(|d| d.address.trim().to_string()).unwrap_or_default();
                // (An address is a host and a port: one without is no address, whatever it resolves to.)
                let problem = if address.rsplit_once(':').is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok()) {
                    self.connect(shell, Way::Address(address)).err()
                } else {
                    Some(UserError::BadAddress)
                };
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
            "lobby.first-to" => self.step(shell, id, index, 1),
            "lobby.closed" => {
                let closed = !self.live.closed;
                self.set_closed(closed);
            }
            "lobby.cards" => {
                let on = !self.live.patch_cards;
                self.set_patch_cards(on);
            }
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
            "menu.yours" => self.set_page(shell, MatchPage::Yours),
            "menu.theirs" => self.set_page(shell, MatchPage::Theirs),
            "menu.controls" => self.set_page(shell, MatchPage::Controls),
            "menu.settings" => self.set_page(shell, MatchPage::Settings),
            "menu.back" => self.set_page(shell, MatchPage::Menu),
            "menu.delay" => {}
            "menu.leave" if matches!(self.stage, Stage::Match { .. }) => self.reopen(shell, None),
            // ---- The match's result.
            "result.go" => self.go_on(shell),
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, id: &str, _index: i32, by: i32) {
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
                self.set_first_to((first + (self.live.first_to - first + by).rem_euclid(count)) as u8);
            }
            "lobby.closed" => self.set_closed(by > 0),
            "lobby.cards" => self.set_patch_cards(by < 0),
            "lobby.delay" | "menu.delay" => self.set_delay(shell, by),
            "menu.music" => self.set_levels(shell, (shell.settings.music as i32 + 5 * by).clamp(0, 100) as u32, shell.settings.effects),
            "menu.effects" => self.set_levels(shell, shell.settings.music, (shell.settings.effects as i32 + 5 * by).clamp(0, 100) as u32),
            "name.version" => self.step_version(shell, by),
            _ => {}
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, _index: i32, choice: i32) {
        match id {
            "build" => self.choose_build(shell, choice.max(0) as usize),
            "name.version" => self.choose_version(shell, choice.max(0) as usize),
            "lobby.closed" => self.set_closed(choice == 1),
            "lobby.cards" => self.set_patch_cards(choice == 0),
            "menu.music" => self.set_levels(shell, choice.clamp(0, 100) as u32, shell.settings.effects),
            "menu.effects" => self.set_levels(shell, shell.settings.music, choice.clamp(0, 100) as u32),
            _ => {}
        }
    }

    fn edit(&mut self, shell: &mut Shell, id: &str, _index: i32, text: &str, done: bool) {
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
                    direct.address = text.chars().filter(|c| !c.is_control() && !c.is_whitespace()).take(80).collect();
                    view.set_address(direct.address.as_str().into());
                    if direct.join_problem.take().is_some() {
                        view.set_join_message(Default::default());
                    }
                }
            }
            _ => {}
        }
    }

    fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
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

    /// In the code's boxes, a character typed goes into the box the focus is on, and the focus to the next.
    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
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

    /// A pad can't type a code: its confirm on the code's field opens the six boxes.
    fn button(&mut self, shell: &mut Shell, button: Button) -> bool {
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
        } else if let Some(to) = self.back_to {
            // (Back to the screen that opened this one: Practice, through its CREATE A BUILD.)
            shell.go(to);
        } else {
            return false;
        }
        true
    }

    /// Practice opened this screen (`open`); a row this screen added to the practice menu was chosen; a question
    /// this screen asked (whether a build goes, whether the lobby is left) was answered.
    fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) {
        if let Some(from) = crate::screens::opened_by(tag) {
            self.back_to = Some(from);
            shell.ui().global::<PlayView>().set_to_practice(from == ScreenId::Practice);
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
        match self.wait_rows.get(choice) {
            Some(WaitRow::Copy) => {
                let invite = self.invite();
                self.copy(shell, &invite);
                shell.close_menu();
            }
            Some(WaitRow::Stop) => practice::end(shell),
            Some(WaitRow::Cancel) => self.leave(shell),
            None => {}
        }
    }

    /// A save dropped on the builds is made a build.
    fn dropped(&mut self, shell: &mut Shell, path: &std::path::Path) {
        if self.part() == PlayPart::Builds && self.naming.is_none() && self.direct.is_none() && self.boxes.is_none() {
            self.import(shell, path);
        }
    }

    /// A build's page in the match's menu: Up and Down scroll it a step, Page Up and Down a page less a
    /// step, Home and End to its ends. It takes them all while the page shows, at an end too: its one control
    /// is Back, which the focus is on.
    fn scroll(&mut self, shell: &mut Shell, action: Action) -> bool {
        let open = shell.battle.as_ref().is_some_and(|b| b.owner == ScreenId::Play && b.menu_open());
        if !open || !matches!(self.page, MatchPage::Yours | MatchPage::Theirs) {
            return false;
        }
        let ui = shell.ui();
        let view = ui.global::<PlayView>();
        let (at, span, page) = (-view.get_sheet_y(), view.get_sheet_span(), (view.get_sheet_view() - SHEET_STEP).max(SHEET_STEP));
        let to = match action {
            Action::Up => at - SHEET_STEP,
            Action::Down => at + SHEET_STEP,
            Action::PageUp => at - page,
            Action::PageDown => at + page,
            Action::Home => 0.0,
            Action::End => span,
            _ => return false,
        }
        .clamp(0.0, span.max(0.0));
        if to != at {
            view.set_sheet_y(-to);
            shell.sound.play(Cue::Cursor);
        }
        true
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
        if matches!(self.stage, Stage::Lobby) {
            self.show_live(shell);
        }
        // In a match: both builds while Tab (Select on a pad) is held; the menu opens on its first page.
        if matches!(self.stage, Stage::Match { .. }) {
            let held = shell.keys.tab || (shell.pad.held(&shell.bindings) | shell.touch.held()) & keys::SELECT != 0;
            self.set_peek(shell, held && shell.battle_has_keys());
            if self.page != MatchPage::Menu && !shell.battle.as_ref().is_some_and(|b| b.menu_open()) {
                self.page = MatchPage::Menu;
                shell.ui().global::<PlayView>().set_page(MatchPage::Menu);
            }
        }
    }
}

impl Play {
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
}
