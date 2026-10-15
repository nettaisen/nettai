//! Replays (docs/design/app-ui.md §6; docs/app.md §16): the selected game's replays, newest first, with the
//! selected one beside them; the viewer; and the export.
//!
//! The list is read off the window's thread (`crate::replays`): the folder is listed again each time the screen
//! shows, each replay of the game is played out there for its score, and the selected one's still is made
//! there. A file that doesn't read, or a replay that isn't played here, is a row with its reason
//! ([`UserError`]'s sentence), never a failure of the list. No build's name is anywhere: a replay has none.
//!
//! The viewer is the library's `Viewer` ([`viewer::Watching`]): the shell makes it ready behind the two cards
//! and hands it over, and this screen shows its picture, plays its sound and has its transport. The export is
//! a dialog of its own ([`export::Export`]) over the list or the viewer.

mod export;
mod viewer;

use crate::battle::{BattleEvent, BattleStart, Play};
use crate::errors::UserError;
use crate::input::{Action, KeyPress};
use crate::replays::{Entry, Found, Library, Look, Stamp};
use crate::screens::{Screen, ScreenId};
use crate::shell::{Dialog, Shell, button};
use crate::sound::Cue;
use crate::stage::Surface;
use export::Export;
use nettai_battle::{BattleResult, Content};
use nettai_frontend::game::GraphicsMode;
use nettai_frontend::player::time_of;
use nettai_frontend::replay::{ENGINE, Info, MatchKind, Outcome};
use nettai_render::Frame;
use nettai_render::vfont::TextRenderer;
use nettai_ui::{BattleKind, ButtonKind, DialogButton, ReplayRow, ReplaysView, VersusCard};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};
use viewer::{Overlays, Watching};

/// The viewer's view: its focus is remembered apart from the list's.
const VIEWER: &str = "replays.viewer";

/// The most a search is.
const SEARCH_LENGTH: usize = 24;

/// A length of time as minutes and seconds (`5:31`; with `padded`, `05:31`).
fn clock(time: Duration, padded: bool) -> String {
    let seconds = time.as_secs();
    if padded { format!("{:02}:{:02}", seconds / 60, seconds % 60) } else { format!("{}:{:02}", seconds / 60, seconds % 60) }
}

/// When a replay was played, in the player's own time: the day (`Oct 11`), the day with its year, the time of
/// day (`21:42`), and the day as a file's name says it (`2026-10-11`).
struct Played {
    day: String,
    full: String,
    time: String,
    file: String,
}

fn played(view: &ReplaysView, when: u64) -> Played {
    use chrono::{Datelike, Timelike};
    let Some(at) = chrono::DateTime::from_timestamp(when as i64, 0).map(|t| t.with_timezone(&chrono::Local)) else {
        return Played { day: String::new(), full: String::new(), time: String::new(), file: String::new() };
    };
    Played {
        day: view.invoke_day(at.month0() as i32, at.day() as i32).into(),
        full: view.invoke_full_day(at.month0() as i32, at.day() as i32, at.year()).into(),
        time: format!("{:02}:{:02}", at.hour(), at.minute()),
        file: at.format("%Y-%m-%d").to_string(),
    }
}

/// A side's player as shown: their name, or which player they were where the replay has none.
fn name(view: &ReplaysView, info: &Info, side: u8) -> String {
    match info.names[side.min(1) as usize].trim() {
        "" if side == 0 => view.get_p1().into(),
        "" => view.get_p2().into(),
        name => name.to_string(),
    }
}

/// How a set went for `side`: 1 won, 2 lost, 3 drawn, 0 not finished; and the rounds it won and lost.
fn score(outcome: &Outcome, side: u8) -> (i32, i32, i32) {
    let rounds = |of: u8| outcome.rounds.iter().filter(|w| **w == Some(of)).count() as i32;
    // (The set's result is side 0's.)
    let tone = match (outcome.result, side) {
        (Some(BattleResult::Won), 0) | (Some(BattleResult::Lost), 1) => 1,
        (Some(BattleResult::Lost), 0) | (Some(BattleResult::Won), 1) => 2,
        (Some(BattleResult::Drawn), _) => 3,
        _ => 0,
    };
    (tone, rounds(side), rounds(side ^ 1))
}

/// Show a file in the system's file manager.
fn show_file(path: &Path) {
    let mut command = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg("-R").arg(path);
        c
    } else if cfg!(target_os = "windows") {
        let mut select = std::ffi::OsString::from("/select,");
        select.push(path);
        let mut c = std::process::Command::new("explorer");
        c.arg(select);
        c
    } else {
        // (No one way to point at a file: its folder.)
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path.parent().unwrap_or(path));
        c
    };
    if let Err(why) = command.spawn() {
        crate::errors::log("showing a file", &why);
    }
}

/// The selected replay's still: the frame the thread made, shown in the screen's `Picture`.
#[derive(Default)]
struct Still {
    surface: Surface,
    /// The frame, and whose it is.
    frame: Option<(PathBuf, Box<Frame>)>,
    /// The enhanced mode's text over it.
    text: Option<TextRenderer>,
    /// The replay whose still was asked for last.
    asked: Option<PathBuf>,
}

#[derive(Default)]
pub struct Replays {
    library: Library,
    /// The files listed, as places in the library's entries, in the list's order.
    rows: Vec<usize>,
    model: Rc<VecModel<ReplayRow>>,
    online_only: bool,
    search: String,
    /// The replay shown beside the list.
    selected: Option<PathBuf>,
    /// Why each file that isn't played isn't: found (and written to the log) once.
    reasons: HashMap<(PathBuf, Stamp), UserError>,
    /// The selected game's content's hash (computing it walks all of the content: it is kept).
    hash: Option<(String, u64)>,
    still: Still,
    watching: Option<Watching>,
    /// What was on over the last replay's picture (the input display, the other console's picture): the next
    /// one watched has them too, while the app runs. Off at first.
    overlays: Overlays,
    /// The replay being made ready to watch (the two cards show), and its two sides where its builds may be
    /// shown (the match's weren't closed).
    opening: Option<PathBuf>,
    sides: Option<Box<[nettai_match::Side; 2]>>,
    export: Option<Export>,
    /// A phone: the selected replay's sheet is open.
    sheet: bool,
    /// `NETTAI_REPLAY` was answered; its game was selected for it.
    started: bool,
    env_game_asked: bool,
    /// The app is kept awake for this screen (`Shell::stay_awake`): a replay plays, or a video is being made.
    awake: bool,
}

impl Replays {
    /// The entry of the selected replay.
    fn selected(&self) -> Option<&Entry> {
        let path = self.selected.as_ref()?;
        self.rows.iter().map(|&i| &self.library.entries[i]).find(|e| e.path == *path)
    }

    /// `game`'s content's hash.
    fn hash(&mut self, game: &str, content: &Content) -> u64 {
        match &self.hash {
            Some((of, hash)) if of == game => *hash,
            _ => {
                let hash = content.hash().0;
                self.hash = Some((game.to_string(), hash));
                hash
            }
        }
    }

    /// Why `entry` isn't played on `content` (whose hash is `hash`), if it isn't, as far as is known.
    fn reason(&mut self, entry: &Entry, content: &Content, hash: u64) -> Option<UserError> {
        let key = (entry.path.clone(), entry.stamp);
        if let Some(known) = self.reasons.get(&key) {
            return Some(known.clone());
        }
        let doing = format!("listing {}", entry.path.display());
        let why = match &entry.read {
            Err(e) => UserError::of_replay_read(&doing, e),
            Ok(s) => match (s.head.require(ENGINE, content.game(), hash), self.library.played(entry)) {
                (Err(differs), _) => {
                    crate::errors::log(&doing, &differs);
                    UserError::ReplayVersion
                }
                (Ok(()), Some(Err(refused))) => UserError::of_refusal(&doing, refused),
                // (One that plays another battle than it recorded was made by another build of this version.)
                (Ok(()), Some(Ok(Outcome { diverged: Some(differs), .. }))) => {
                    crate::errors::log(&doing, differs);
                    UserError::ReplayVersion
                }
                _ => return None,
            },
        };
        self.reasons.insert(key, why.clone());
        Some(why)
    }

    /// A row of the list.
    fn row(&mut self, shell: &Shell, entry: &Entry, content: &Content, hash: u64) -> ReplayRow {
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let when = played(&view, entry.when);
        let mut row = ReplayRow { day: when.day.into(), time: when.time.into(), ..ReplayRow::default() };
        if let Some(why) = self.reason(entry, content, hash) {
            row.reason = why.line(&ui).into();
            return row;
        }
        let Ok(s) = &entry.read else { return row };
        row.opponent = name(&view, &s.info, s.info.side ^ 1).into();
        row.length = clock(time_of(s.ticks as u64), false).into();
        if let Some(Ok(outcome)) = self.library.played(entry) {
            let (tone, won, lost) = score(outcome, s.info.side);
            row.tone = tone;
            row.score = if tone == 0 { format!("{won}–{lost}").into() } else { view.invoke_score(tone, won, lost) };
        }
        row
    }

    /// The list, from what the folder was last found to hold: the selected game's replays, filtered, with the
    /// selected one marked. Each is played out for its score, if it wasn't.
    fn fill(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let ready = shell.ready();
        view.set_listed(self.library.listed && ready.is_some());
        let entries = std::mem::take(&mut self.library.entries);
        self.rows.clear();
        let mut rows = Vec::new();
        if let Some((game, ready)) = &ready {
            let content = ready.content();
            let hash = self.hash(game, content);
            // The game's replays: those whose head names it. (A file whose head doesn't read names no game: it is
            // listed with every one.)
            let of_game: Vec<usize> = (0..entries.len()).filter(|&i| entries[i].game.as_ref().is_none_or(|g| g == game)).collect();
            let kind = |i: usize| entries[i].read.as_ref().ok().map(|s| s.info.kind);
            let kinds = of_game.iter().any(|&i| kind(i) == Some(MatchKind::Online)) && of_game.iter().any(|&i| kind(i) == Some(MatchKind::Offline));
            self.online_only &= kinds;
            view.set_kinds(kinds);
            view.set_kind(self.online_only as i32);
            view.set_count(of_game.len() as i32);
            let needle = self.search.trim().to_lowercase();
            for i in of_game {
                let entry = &entries[i];
                let shown = match &entry.read {
                    Ok(s) => (!self.online_only || s.info.kind == MatchKind::Online) && (needle.is_empty() || name(&view, &s.info, s.info.side ^ 1).to_lowercase().contains(&needle)),
                    Err(_) => !self.online_only && needle.is_empty(),
                };
                if !shown {
                    continue;
                }
                if entry.read.as_ref().is_ok_and(|s| s.head.require(ENGINE, content.game(), hash).is_ok()) {
                    self.library.play(entry, content, hash);
                }
                rows.push(self.row(shell, entry, content, hash));
                self.rows.push(i);
            }
        } else {
            view.set_kinds(false);
            view.set_count(0);
        }
        self.library.entries = entries;
        self.model.set_vec(rows);
        view.set_rows(ModelRc::from(self.model.clone()));
        // The selected replay: the one it was, if it is still listed, else the first.
        let chosen = self.selected.as_ref().and_then(|path| self.rows.iter().position(|&i| self.library.entries[i].path == *path)).or((!self.rows.is_empty()).then_some(0));
        self.selected = chosen.map(|at| self.library.entries[self.rows[at]].path.clone());
        view.set_chosen(chosen.map_or(-1, |at| at as i32));
        // (The cursor stays on it, wherever its row went.)
        if let (Some(at), Some(focus)) = (chosen, shell.nav.focused())
            && focus.id == "list"
            && focus.index != at as i32
        {
            shell.nav.focus("list", at as i32);
        }
        self.show_selected(shell);
    }

    /// The selected replay, beside the list; its still is asked for.
    fn show_selected(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let (Some(entry), Some((game, ready))) = (self.selected().cloned(), shell.ready()) else {
            view.set_selected(false);
            view.set_playable(false);
            view.set_still(false);
            return;
        };
        let content = ready.content();
        let hash = self.hash(&game, content);
        let reason = self.reason(&entry, content, hash);
        let when = played(&view, entry.when);
        view.set_selected(true);
        view.set_playable(reason.is_none() && entry.read.is_ok());
        view.set_played(format!("{} · {}", when.full, when.time).into());
        view.set_reason(reason.as_ref().map(|r| r.line(&ui)).unwrap_or_default().into());
        let (mut result, mut tone, mut length) = (String::new(), 0, String::new());
        let (left, right) = match &entry.read {
            Ok(s) => {
                if reason.is_none() {
                    length = clock(time_of(s.ticks as u64), false);
                    if let Some(Ok(outcome)) = self.library.played(&entry) {
                        let (how, won, lost) = score(outcome, s.info.side);
                        tone = how;
                        result = view.invoke_result_of(how, won, lost).into();
                    }
                }
                (name(&view, &s.info, s.info.side), name(&view, &s.info, s.info.side ^ 1))
            }
            // (Nothing is known of it but its file.)
            Err(_) => (entry.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), String::new()),
        };
        view.set_left_name(left.into());
        view.set_right_name(right.into());
        view.set_result(result.into());
        view.set_result_tone(tone);
        view.set_length(length.into());
        // Its still: the one made, or asked for now. Until its own is there the box is empty: the still of the
        // replay selected before is another replay's picture, however short the wait.
        let made = self.still.frame.as_ref().is_some_and(|(path, _)| *path == entry.path);
        match &entry.read {
            Ok(s) if reason.is_none() => {
                if !made && self.still.asked.as_ref() != Some(&entry.path) {
                    self.still.asked = Some(entry.path.clone());
                    let mode = shell.settings.graphics();
                    let look = Look { graphics: ready.graphics(shell.lang), mode, font: ready.loaded.font.clone().filter(|_| mode == GraphicsMode::Enhanced) };
                    self.library.still(&entry, s.info.side, content, look);
                }
                view.set_still(made);
            }
            _ => view.set_still(false),
        }
    }

    /// What the thread found since the last frame.
    fn take_found(&mut self, shell: &mut Shell, shown: bool) {
        let found = self.library.poll();
        if found.is_empty() {
            return;
        }
        let listing = shown && self.watching.is_none();
        let (mut refill, mut played) = (false, Vec::new());
        for found in found {
            match found {
                Found::Listed => refill = true,
                Found::Played(path) => played.push(path),
                Found::Still(path, frame) => {
                    if self.still.asked.as_ref() == Some(&path) {
                        self.still.asked = None;
                    }
                    if self.selected.as_ref() == Some(&path) {
                        self.still.frame = frame.map(|f| (path, f));
                        self.still.surface.stale = true;
                        if listing {
                            shell.ui().global::<ReplaysView>().set_still(self.still.frame.is_some());
                        }
                    }
                }
            }
        }
        if refill && listing {
            self.fill(shell);
        } else if listing {
            for path in played {
                self.fill_row(shell, &path);
            }
        }
    }

    /// The row of the replay at `path` again (it was played out), and the panel beside the list if it is the
    /// selected one.
    fn fill_row(&mut self, shell: &mut Shell, path: &Path) {
        let Some((game, ready)) = shell.ready() else { return };
        let Some(at) = self.rows.iter().position(|&i| self.library.entries[i].path == *path) else { return };
        let entry = self.library.entries[self.rows[at]].clone();
        let content = ready.content();
        let hash = self.hash(&game, content);
        let row = self.row(shell, &entry, content, hash);
        self.model.set_row_data(at, row);
        if self.selected.as_deref() == Some(path) {
            self.show_selected(shell);
        }
    }

    /// Watch the replay at `path` from `side`'s console (none: its recorder's): the two cards while it is made
    /// ready, then the viewer (`BattleEvent::Watching`).
    fn watch(&mut self, shell: &mut Shell, path: &Path, side: Option<u8>) {
        let Some((game, ready)) = shell.ready() else { return };
        let doing = format!("opening {}", path.display());
        let replay = match crate::replays::read(path) {
            Ok(replay) => replay,
            Err(why) => return shell.fail(UserError::of_replay_read(&doing, &why)),
        };
        // (Its set is started to be checked: an engine that stops there is a failure, not a crash.)
        let set = match nettai_frontend::session::in_engine(|| nettai_frontend::replay::set_of(ready.content(), &replay)) {
            Ok(Ok(set)) => set,
            Ok(Err(refused)) => return shell.fail(UserError::of_refusal(&doing, &refused)),
            Err(panic) => {
                crate::errors::log(&doing, &format_args!("the engine stopped starting its set: {}", nettai_frontend::session::panic_message(panic.as_ref())));
                return shell.fail(UserError::Other);
            }
        };
        let side = side.unwrap_or(replay.info.side).min(1);
        // (A closed-builds match's sides aren't kept at all.)
        self.sides = (!replay.info.closed_builds).then(|| nettai_match::binary::read_match(ready.content(), &replay.match_bytes).ok()).flatten().map(|m| Box::new(m.sides));
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        // (The console shown has its side at the left. A card says who played, and nothing of what they
        // brought.)
        let card = |of: u8| VersusCard { role: if of == 0 { view.get_p1() } else { view.get_p2() }, name: replay.info.names[of as usize].as_str().into(), ..VersusCard::default() };
        let cards = [card(side), card(side ^ 1)];
        self.close_sheet(shell);
        self.opening = Some(path.to_path_buf());
        shell.start_battle(BattleStart {
            owner: ScreenId::Replays,
            kind: BattleKind::Replay,
            game,
            set,
            play: Play::Watch(Box::new(replay), Some(side)),
            viewer: side,
            recorder: None,
            cards: Some(cards),
            full: false,
            music: crate::battle::BattleMusic::GameOwn,
            gate: None,
        });
    }

    /// Leave the viewer for the list.
    fn stop_watching(&mut self, shell: &mut Shell) {
        let Some(w) = self.watching.take() else { return };
        self.overlays = w.overlays();
        shell.lock_game(false);
        shell.ui().global::<ReplaysView>().set_watching(false);
        shell.ui().global::<ReplaysView>().set_builds_shown(false);
        shell.nav.enter(ScreenId::Replays.name(), self.main());
        self.still.surface.refresh();
        self.fill(shell);
    }

    /// Open the export's dialog for the replay at `path`, shown from `side` (none: its recorder's).
    fn open_export(&mut self, shell: &mut Shell, path: &Path, side: Option<u8>) {
        let Some(entry) = self.library.entries.iter().find(|e| e.path == *path) else { return };
        let Ok(summary) = &entry.read else { return };
        if let Some(w) = &mut self.watching {
            w.pause();
            w.hide_builds(shell);
        }
        self.export = Some(Export::open(shell, path, summary, side.unwrap_or(summary.info.side)));
    }

    fn close_sheet(&mut self, shell: &mut Shell) {
        if std::mem::take(&mut self.sheet) {
            shell.nav.close("sheet.");
            shell.ui().global::<ReplaysView>().set_detail(false);
        }
    }

    /// Ask whether to delete the selected replay: once, Cancel focused.
    fn ask_delete(&mut self, shell: &mut Shell) {
        let Some(entry) = self.selected().cloned() else { return };
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let when = played(&view, entry.when);
        let who = match &entry.read {
            Ok(s) => view.invoke_versus(name(&view, &s.info, s.info.side).into(), name(&view, &s.info, s.info.side ^ 1).into()).to_string(),
            Err(_) => entry.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        };
        shell.ask(Dialog {
            tag: "delete",
            title: view.get_delete_title().into(),
            message: format!("{who} · {} · {}", when.full, when.time),
            buttons: vec![button(&view.get_cancel(), ButtonKind::Normal), DialogButton { text: view.get_delete(), kind: ButtonKind::Normal, danger: true }],
            focus: 0,
            cancel: 0,
        });
    }

    /// The list's actions, from the panel beside it or a phone's sheet.
    fn act(&mut self, shell: &mut Shell, what: &str) {
        let Some(path) = self.selected.clone() else { return };
        match what {
            "watch" => self.watch(shell, &path, None),
            "export" => self.open_export(shell, &path, None),
            "file" => show_file(&path),
            "delete" => self.ask_delete(shell),
            _ => {}
        }
    }

    /// Whether the selected replay plays here.
    fn playable(&self, shell: &Shell) -> bool {
        self.selected.is_some() && shell.ui().global::<ReplaysView>().get_playable()
    }

    /// `NETTAI_REPLAY=<file>`: straight into watching it, once its game is ready (`NETTAI_REPLAY_SPEED`: at
    /// that speed, `half`, `1`, `2` or `4`).
    fn answer_env(&mut self, shell: &mut Shell) {
        let Some(path) = std::env::var_os("NETTAI_REPLAY").map(PathBuf::from) else { return self.started = true };
        let game = match crate::replays::read(&path) {
            Ok(replay) => replay.head.game,
            Err(why) => {
                crate::errors::log(&format!("opening {}", path.display()), &why);
                return self.started = true;
            }
        };
        if shell.game() != Some(game.as_str()) {
            // (Its game is selected once: one that can't be played here, the shell says why and selects another.)
            if std::mem::replace(&mut self.env_game_asked, true) {
                crate::errors::log(&format!("opening {}", path.display()), &format_args!("its game, {game}, can't be played here"));
                return self.started = true;
            }
            shell.select_game(&game);
        } else if shell.ready().is_some() {
            self.started = true;
            shell.go(ScreenId::Replays);
            self.watch(shell, &path, None);
        }
    }
}

impl Screen for Replays {
    fn main(&self) -> &'static str {
        "list"
    }

    fn show(&mut self, shell: &mut Shell) {
        // The game or the language may have changed under it: the stills are drawn anew.
        self.still.frame = None;
        self.still.asked = None;
        self.still.text = shell.ready().and_then(|(_, ready)| ready.loaded.font.clone().filter(|_| shell.settings.enhanced)).map(TextRenderer::new);
        if let Some(w) = &mut self.watching {
            shell.nav.enter(VIEWER, "seek");
            shell.lock_game(true);
            w.show(shell);
            if let Some(e) = &mut self.export {
                e.show(shell);
            }
            return;
        }
        shell.ui().global::<ReplaysView>().set_watching(false);
        if !shell.scale.phone {
            self.close_sheet(shell);
        }
        // The folder, listed again: a match just recorded is there. (What playing out gave is kept between runs.)
        self.library.cache.get_or_insert_with(crate::replays::cache_file);
        self.library.list(&crate::replays::folder());
        self.fill(shell);
        if let Some(e) = &mut self.export {
            e.show(shell);
        }
    }

    fn hide(&mut self, shell: &mut Shell) {
        if let Some(e) = self.export.take() {
            e.close(shell);
        }
        self.close_sheet(shell);
        if let Some(w) = self.watching.take() {
            self.overlays = w.overlays();
            shell.ui().global::<ReplaysView>().set_watching(false);
        }
        self.opening = None;
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if let Some(what) = id.strip_prefix("export.") {
            if let Some(e) = &mut self.export
                && e.activate(shell, what)
            {
                self.export.take().expect("it is open").close(shell);
            }
            return;
        }
        if let Some(w) = &mut self.watching {
            match id {
                "seek" | "play" => w.toggle(),
                "round-start" => w.round(-1),
                "next-round" => w.round(1),
                "frame-back" => w.step(-1),
                "frame-on" => w.step(1),
                "builds" | "builds.sheet" => w.toggle_builds(shell),
                "inputs" => w.toggle_inputs(shell),
                "inset" => w.toggle_inset(shell),
                "speed" => w.set_speed(index),
                "view" => w.set_view(shell, index),
                "export" => {
                    let (path, side) = (w.path().to_path_buf(), w.side());
                    self.open_export(shell, &path, Some(side));
                }
                _ => {}
            }
            return;
        }
        match id {
            "kind" => {
                self.online_only = !self.online_only;
                self.fill(shell);
            }
            "list" => {
                self.selected = self.rows.get(index.max(0) as usize).map(|&i| self.library.entries[i].path.clone());
                self.fill(shell);
                if shell.scale.phone {
                    // (A phone: the replay's sheet, from which it is watched.)
                    if self.selected.is_some() && !self.sheet {
                        self.sheet = true;
                        shell.ui().global::<ReplaysView>().set_detail(true);
                        shell.nav.open("sheet.", crate::nav::Key::new("sheet.watch", 0));
                    }
                } else if shell.nav.pointing() {
                    // (A click chooses the replay; WATCH, Enter or the pad's A starts it.)
                } else if self.playable(shell) {
                    self.act(shell, "watch");
                } else {
                    shell.sound.play(Cue::Refused);
                }
            }
            "sheet.close" => self.close_sheet(shell),
            _ => self.act(shell, id.strip_prefix("sheet.").unwrap_or(id)),
        }
    }

    fn step(&mut self, shell: &mut Shell, id: &str, _index: i32, by: i32) {
        if let Some(what) = id.strip_prefix("export.") {
            if let Some(e) = &mut self.export {
                e.step(shell, what, by);
            }
            return;
        }
        match (&mut self.watching, id) {
            (Some(w), "seek") => w.skip(5 * by),
            (Some(w), "speed-step") => w.set_speed(w.speed() + by),
            (Some(w), "view-step") => w.set_view(shell, (w.side() ^ 1) as i32),
            (None, "kind") => {
                self.online_only = by > 0;
                self.fill(shell);
            }
            _ => {}
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, index: i32, choice: i32) {
        if let Some(what) = id.strip_prefix("export.") {
            if let Some(e) = &mut self.export {
                e.choose(shell, what, choice);
            }
            return;
        }
        match (&mut self.watching, id) {
            // (The bar: held at `choice` of 10,000, or with `index` 1 let go there.)
            (Some(w), "seek") => w.drag(choice, index == 1),
            (None, "kind") => {
                self.online_only = choice == 1;
                self.fill(shell);
            }
            _ => {}
        }
    }

    fn edit(&mut self, shell: &mut Shell, id: &str, _index: i32, text: &str, _done: bool) {
        if id != "search" {
            return;
        }
        self.search = text.chars().filter(|c| !c.is_control()).take(SEARCH_LENGTH).collect();
        shell.ui().global::<ReplaysView>().set_search(self.search.as_str().into());
        self.fill(shell);
        // (What the search finds is shown from its top: the user, "when a search box is changed, it should reset
        // scroll".)
        let ui = shell.ui();
        shell.nav.list_to_top(&ui, "list");
    }

    fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if id != "list" || self.watching.is_some() {
            return;
        }
        let Some(&at) = self.rows.get(index.max(0) as usize) else { return };
        let path = self.library.entries[at].path.clone();
        if self.selected.as_ref() != Some(&path) {
            self.selected = Some(path);
            shell.ui().global::<ReplaysView>().set_chosen(index);
            self.show_selected(shell);
        }
    }

    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        if self.export.is_some() || self.sheet {
            return false;
        }
        if let Some(w) = &mut self.watching {
            match key.code {
                KeyCode::Space if !key.repeat => w.toggle(),
                KeyCode::Space => {}
                KeyCode::Comma => w.step(-1),
                KeyCode::Period => w.step(1),
                KeyCode::BracketLeft if !key.repeat => w.round(-1),
                KeyCode::BracketRight if !key.repeat => w.round(1),
                KeyCode::BracketLeft | KeyCode::BracketRight => {}
                KeyCode::KeyB if !key.repeat => w.toggle_builds(shell),
                // (I for the input display; P for the other console's picture in the corner: a picture in the
                // picture.)
                KeyCode::KeyI if !key.repeat => w.toggle_inputs(shell),
                KeyCode::KeyP if !key.repeat => w.toggle_inset(shell),
                KeyCode::KeyB | KeyCode::KeyI | KeyCode::KeyP => {}
                _ => return false,
            }
            return true;
        }
        // E exports the selected replay (but in the search, where it is a letter).
        if key.code == KeyCode::KeyE && !shell.nav.on_field() && !key.repeat {
            if self.playable(shell) {
                self.act(shell, "export");
            } else {
                shell.sound.play(Cue::Refused);
            }
            return true;
        }
        false
    }

    fn button(&mut self, shell: &mut Shell, button: gamepad_facade::Button) -> bool {
        // Select shows both builds, or puts them away.
        match &mut self.watching {
            Some(w) if self.export.is_none() && button == gamepad_facade::Button::Back => {
                w.toggle_builds(shell);
                true
            }
            _ => false,
        }
    }

    fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        if self.export.is_some() {
            return true;
        }
        if let Some(w) = &mut self.watching {
            // The shoulders go a round; the pad's north button plays and pauses.
            match action {
                Action::Previous => w.round(-1),
                Action::Next => w.round(1),
                Action::Alternate => w.toggle(),
                _ => return false,
            }
            return true;
        }
        match action {
            Action::Delete if !shell.nav.on_field() && self.selected.is_some() => self.ask_delete(shell),
            Action::Alternate if self.playable(shell) => self.act(shell, "export"),
            _ => return false,
        }
        true
    }

    fn back(&mut self, shell: &mut Shell) -> bool {
        if let Some(e) = &mut self.export {
            if e.back(shell) {
                self.export.take().expect("it is open").close(shell);
            }
            return true;
        }
        if self.sheet {
            self.close_sheet(shell);
            return true;
        }
        if let Some(w) = &mut self.watching
            && w.hide_builds(shell)
        {
            return true;
        }
        if self.watching.is_some() {
            self.stop_watching(shell);
            return true;
        }
        if self.opening.take().is_some() && shell.loading_battle() {
            shell.cancel_loading();
            return true;
        }
        false
    }

    fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) {
        if tag != "delete" || choice != 1 {
            return;
        }
        let Some(path) = self.selected.clone() else { return };
        self.close_sheet(shell);
        match std::fs::remove_file(&path) {
            Ok(()) => {
                // The row after it takes its place.
                let at = self.rows.iter().position(|&i| self.library.entries[i].path == path);
                self.library.entries.retain(|e| e.path != path);
                self.selected = None;
                self.fill(shell);
                if let Some(at) = at.filter(|_| !self.rows.is_empty()) {
                    let at = at.min(self.rows.len() - 1);
                    self.selected = Some(self.library.entries[self.rows[at]].path.clone());
                    self.fill(shell);
                    shell.nav.focus("list", at as i32);
                }
                self.library.list(&crate::replays::folder());
            }
            Err(why) => {
                crate::errors::log(&format!("deleting {}", path.display()), &why);
                shell.fail(UserError::Other);
            }
        }
    }

    fn battle(&mut self, shell: &mut Shell, event: BattleEvent) {
        let BattleEvent::Watching(viewer) = event else { return };
        let Some(path) = self.opening.take() else { return };
        self.watching = Some(Watching::new(viewer, path, self.sides.take(), self.overlays));
        // (`show` sets the viewer's view.)
        shell.refresh();
    }

    fn frame(&mut self, shell: &mut Shell, now: Instant, shown: bool) {
        if !self.started {
            self.answer_env(shell);
        }
        self.take_found(shell, shown);
        if let Some(e) = &mut self.export {
            e.frame(shell);
        }
        // A video being made goes on whatever covers the window, and a replay playing in a window that is drawn
        // keeps the display on with no key pressed. A replay in a window that isn't drawn stands still (the
        // frame is the shell's timer's then: `Shell::drawing`), and asks for nothing.
        let busy = self.export.as_ref().is_some_and(Export::running) || (shell.drawing && self.watching.as_ref().is_some_and(Watching::playing));
        if busy != self.awake {
            self.awake = busy;
            shell.stay_awake = busy;
        }
        if !shown || shell.loading_battle() {
            return;
        }
        if let Some(w) = &mut self.watching {
            if shell.drawing { w.frame(shell, now) } else { w.rest(shell) }
            // The engine stopped short of the replay's end (as it played, or at a frame stepped or sought to):
            // the replay ends there, and the player is told so over the list.
            if let Some(why) = w.stopped() {
                crate::errors::log(&format!("watching {}", w.path().display()), &why);
                if let Some(e) = self.export.take() {
                    e.close(shell);
                }
                self.stop_watching(shell);
                shell.fail(UserError::Other);
            }
            return;
        }
        // The selected replay's still, where its box is on screen.
        let Still { surface, frame, text, .. } = &mut self.still;
        if let Some((path, frame)) = frame
            && self.selected.as_ref() == Some(path)
            && (!shell.scale.phone || self.sheet)
        {
            shell.show_surface(surface, |buffer, width, height| {
                nettai_render::present::present(frame, text.as_mut(), buffer, width, height);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_frontend::replay::End;

    /// A length is minutes and seconds, the seconds always two digits; the viewer's has two for the minutes
    /// too, so its box keeps its width.
    #[test]
    fn a_length_is_minutes_and_seconds() {
        assert_eq!(clock(Duration::from_secs(331), false), "5:31");
        assert_eq!(clock(Duration::from_secs(331), true), "05:31");
        assert_eq!(clock(Duration::from_millis(59_999), false), "0:59");
        assert_eq!(clock(Duration::from_secs(4000), true), "66:40");
    }

    /// A set's score is the recorder's: the rounds they won and lost, and how the set went for them (the
    /// library's result is side 0's). A set that didn't end is neither won nor lost.
    #[test]
    fn a_score_is_the_recorders() {
        let outcome = |rounds: &[Option<u8>], result| Outcome { ticks: 0, rounds: rounds.to_vec(), result, end: End::Set, diverged: None, stopped: None };
        let won = outcome(&[Some(0), Some(1), Some(0)], Some(BattleResult::Won));
        assert_eq!(score(&won, 0), (1, 2, 1));
        assert_eq!(score(&won, 1), (2, 1, 2));
        assert_eq!(score(&outcome(&[Some(1), None], Some(BattleResult::Drawn)), 1), (3, 1, 0));
        assert_eq!(score(&outcome(&[Some(1)], None), 0), (0, 0, 1));
    }
}
