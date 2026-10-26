//! Replays (docs/design/app-ui.md §6; docs/app.md §16): the selected game's replays, newest first, with the
//! selected one beside them; the viewer; and the export.
//!
//! The list is read off the window's thread (`crate::replays`): the folder is listed again each time the screen
//! shows, each replay of the game is played out there for its score, and the selected one's still is made
//! there. A file that doesn't read, or a replay that isn't played here, is a row with its reason
//! ([`UserError`]'s sentence), never a failure of the list. No build's name is anywhere: a replay has none.
//!
//! The viewer is the library's `Viewer` ([`viewer::Watching`]): the shell makes it ready behind the two cards
//! and hands it over, and this screen shows its picture, plays its sound and has its transport. The export is the
//! viewer's export mode ([`export::Export`], docs/design/app-ui.md §2.96): the transport's bar has the range's two
//! handles, the transport the range's buttons, EXPORT and CANCEL, and a panel over it the export's options. EXPORT
//! VIDEO in the list opens the viewer in it.
//!
//! The picture fills the window, and the viewer's controls are over it while they are wanted ([`HUD_IDLE`]).
//! WATCH watches the one replay; the arrow beside it (a split button: `Shell::pick_split`) adds it to the play
//! queue ([`Queue`], docs/design/app-ui.md §2.91, §2.92), at its end or at its start, as does a right click on its
//! row. The Queue tab shows the queue, reorders it, takes entries out of it, clears it and watches it from its start
//! or from an entry. The queue is what is still to watch: while it plays its head is the replay watched, which
//! leaves it once played to its end or skipped. The viewer's transport then has the queue's button with the count
//! up next, which opens a small popup of them with the queue's next (skip; N) and Autoplay next (the settings
//! remember it: [`AUTOPLAY`]), which starts the next a moment after one ends ([`NEXT_AFTER`]), behind the two cards
//! as the first was (§2.90). A queue is the selected game's, kept while the app runs and never saved.
//!
//! The transport keeps few buttons (§2.94, §2.95): to the round's start, play or pause, the next round, and ⋯, whose
//! menu (`Shell::pick_marked`, [`viewer::MenuItem`]) has the side watched from, the builds, the input display, the
//! other side's picture (each also a key's), the playback speed (its own menu of the four; a badge by the buttons
//! says a speed other than 1×) and the export. The bar previews the battle under the pointer
//! ([`thumbnails::Thumbnails`]).

mod export;
mod indexing;
mod queue;
mod thumbnails;
mod viewer;

use crate::battle::{BattleEvent, BattleStart, Play};
use crate::errors::UserError;
use crate::input::{Action, KeyPress};
use crate::nav::Key;
use crate::replays::{Entry, Found, Library, Look, Stamp};
use crate::screens::{Screen, ScreenId};
use crate::shell::{Dialog, Shell, button};
use crate::sound::Cue;
use crate::stage::Surface;
use export::Export;
use queue::Queue;
use nettai_battle::{BattleResult, Content};
use nettai_frontend::game::GraphicsMode;
use nettai_frontend::player::{tick_at, time_of};
use nettai_frontend::replay::{ENGINE, Info, MatchKind, Outcome};
use nettai_render::Frame;
use nettai_render::vfont::TextRenderer;
use nettai_ui::{BattleKind, ButtonKind, DialogButton, ReplayRow, ReplaysView, VersusCard};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use nettai_platform::time::Instant;
use viewer::{MENU, MenuItem, Overlays, Watching};

/// The viewer's view: its focus is remembered apart from the list's.
const VIEWER: &str = "replays.viewer";

/// The most a search is.
const SEARCH_LENGTH: usize = 24;

/// How long the viewer's controls stay over a replay playing after the last thing the player did (a key, a pad's
/// button, the pointer moved): then they fade.
const HUD_IDLE: Duration = Duration::from_millis(2500);

/// How long a replay of the queue stays at its end before the next starts, with Autoplay next on.
const NEXT_AFTER: Duration = Duration::from_secs(2);

/// The settings' key Autoplay next is remembered by (`Settings::remembered`): "off", else it is on.
const AUTOPLAY: &str = "replays.autoplay";

/// Whether the queue's next replay starts by itself at the end of one.
fn autoplay(shell: &Shell) -> bool {
    shell.settings.remembered.get(AUTOPLAY).is_none_or(|v| v != "off")
}

/// Autoplay next on or off, remembered.
fn set_autoplay(shell: &mut Shell, on: bool) {
    if on {
        shell.settings.remembered.remove(AUTOPLAY);
    } else {
        shell.settings.remembered.insert(AUTOPLAY.into(), "off".into());
    }
    shell.save_settings();
    shell.ui().global::<ReplaysView>().set_autoplay(on);
}

/// The viewer's popup of what is up next in the queue: its scope (`Nav::open`), its rows, its width on a desktop,
/// the most rows it shows at once (more scroll), and the room over its rows (NEXT and Autoplay next, and the
/// heading).
const UP_NEXT: &str = "upnext.";
const UP_NEXT_ROW: &str = "upnext.row";
const UP_NEXT_WIDTH: f32 = 360.0;
const UP_NEXT_ROWS: usize = 5;
const UP_NEXT_ROOM: f32 = 96.0;

/// What a replay's menu (a right click on its row) does, and what a queue entry's does: each in its order, DELETE
/// and REMOVE last, set apart.
#[derive(Clone, Copy, PartialEq)]
enum MenuAction {
    Watch,
    AddToQueue,
    AddToStart,
    Export,
    ShowFile,
    Delete,
    WatchFromHere,
    MoveUp,
    MoveDown,
    Remove,
}

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

/// A file's name, as a replay that says nothing else of itself is shown by.
fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
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
    /// shown (the match's setups were open).
    opening: Option<PathBuf>,
    sides: Option<Box<[nettai_match::Side; 2]>>,
    /// The viewer's export mode, while it is in it; and that the replay being made ready opens in it (EXPORT VIDEO
    /// from the list).
    export: Option<Export>,
    export_due: bool,
    /// A phone: the selected replay's sheet is open.
    sheet: bool,
    /// `NETTAI_REPLAY` was answered; its game was selected for it.
    started: bool,
    env_game_asked: bool,
    /// The app is kept awake for this screen (`Shell::stay_awake`): a replay plays, or a video is being made.
    awake: bool,
    /// The Queue tab is shown in place of the replays.
    queue_tab: bool,
    /// Each game's play queue, while the app runs; the selected game's rows (each entry's: a missing one's says
    /// so), and the entry chosen in the Queue tab (and its path: the replay shown beside the queue).
    queues: HashMap<String, Queue>,
    queue_rows: Vec<ReplayRow>,
    queue_model: Rc<VecModel<ReplayRow>>,
    queue_chosen: usize,
    queue_path: Option<PathBuf>,
    /// The viewer plays the queue: it has the queue's next, its count and Autoplay next.
    queued: bool,
    /// The replay watched is still the queue's head: it hasn't been played to its end (when it leaves the queue).
    current: bool,
    /// The viewer's popup of what is up next is open.
    up_next: bool,
    /// The speed the queue's next replay starts at: the last one's.
    carry_speed: Option<i32>,
    /// Whether the viewer's controls are shown, as the view was last told; and when a replay began, which shows
    /// them as an input does.
    hud: Option<bool>,
    woke: Option<Instant>,
}

impl Replays {
    /// The replay shown beside the list: the one chosen, or in the Queue tab the entry chosen.
    fn shown_path(&self) -> Option<&PathBuf> {
        if self.queue_tab { self.queue_path.as_ref() } else { self.selected.as_ref() }
    }

    /// The entry of the replay shown beside the list (none for a queue entry whose file is gone).
    fn selected(&self) -> Option<&Entry> {
        let path = self.shown_path()?;
        self.library.entries.iter().find(|e| e.path == *path)
    }

    /// Whether each entry of the queue plays (its row says no reason it doesn't).
    fn plays(&self) -> Vec<bool> {
        self.queue_rows.iter().map(|r| r.reason.is_empty()).collect()
    }

    /// The selected game's queue, made where it has none.
    fn queue_mut(&mut self, shell: &Shell) -> Option<&mut Queue> {
        let game = shell.game()?.to_string();
        Some(self.queues.entry(game).or_default())
    }

    /// The selected game's queue, if it has one.
    fn queue(&self, shell: &Shell) -> Option<&Queue> {
        self.queues.get(shell.game()?)
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
        self.fill_queue(shell);
        self.show_selected(shell);
    }

    /// The selected game's queue, from what the folder was last found to hold: a row for each entry (one whose
    /// file is gone says it is missing), the one chosen kept where it is. Each is played out for its score, if it
    /// wasn't.
    fn fill_queue(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let ready = shell.ready();
        let paths = self.queue(shell).map(|q| q.paths.clone()).unwrap_or_default();
        let mut rows = Vec::new();
        if let Some((game, ready)) = &ready {
            let content = ready.content();
            let hash = self.hash(game, content);
            for path in &paths {
                match self.library.entries.iter().find(|e| e.path == *path).cloned() {
                    Some(entry) => {
                        if entry.read.as_ref().is_ok_and(|s| s.head.require(ENGINE, content.game(), hash).is_ok()) {
                            self.library.play(&entry, content, hash);
                        }
                        rows.push(self.row(shell, &entry, content, hash));
                    }
                    None => rows.push(ReplayRow { reason: view.invoke_missing(file_name(path).into()), ..ReplayRow::default() }),
                }
            }
        }
        self.queue_rows = rows.clone();
        self.queue_model.set_vec(rows);
        view.set_queue(ModelRc::from(self.queue_model.clone()));
        self.queue_chosen = self.queue_chosen.min(paths.len().saturating_sub(1));
        self.queue_path = paths.get(self.queue_chosen).cloned();
        view.set_queue_chosen(if paths.is_empty() { -1 } else { self.queue_chosen as i32 });
    }

    /// The Replays tab (`queue` false) or the Queue tab, the focus on its list.
    fn set_tab(&mut self, shell: &mut Shell, queue: bool) {
        if self.queue_tab == queue {
            return;
        }
        self.close_sheet(shell);
        self.queue_tab = queue;
        shell.ui().global::<ReplaysView>().set_tab(queue as i32);
        self.fill(shell);
        if queue {
            shell.nav.focus(if self.queue_rows.is_empty() { "tab" } else { "queue" }, if self.queue_rows.is_empty() { 1 } else { self.queue_chosen as i32 });
        } else {
            let at = self.selected.as_ref().and_then(|path| self.rows.iter().position(|&i| self.library.entries[i].path == *path)).unwrap_or(0);
            shell.nav.focus("list", at as i32);
        }
    }

    /// The selected replay, beside the list; its still is asked for.
    fn show_selected(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let (Some(entry), Some((game, ready))) = (self.selected().cloned(), shell.ready()) else {
            // (A queue entry whose file is gone: its file's name, and that it is missing.)
            let missing = self.shown_path().filter(|_| self.queue_tab).map(|p| file_name(p));
            view.set_selected(missing.is_some());
            view.set_playable(false);
            view.set_still(false);
            if let Some(file) = missing {
                for set in [ReplaysView::set_right_name, ReplaysView::set_played, ReplaysView::set_result, ReplaysView::set_length] {
                    set(&view, Default::default());
                }
                view.set_reason(view.invoke_missing(file.as_str().into()));
                view.set_left_name(file.into());
            }
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
                    let look = Look { graphics: ready.graphics(shell.i18n.code), mode, font: ready.loaded.font_in(shell.i18n.code).filter(|_| mode == GraphicsMode::Enhanced) };
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
                    if self.shown_path() == Some(&path) {
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
            for path in &played {
                self.fill_row(shell, path);
            }
            // (The queue's rows of those played out too, and the panel if it shows one of them.)
            if played.iter().any(|path| self.queue(shell).is_some_and(|q| q.paths.contains(path))) {
                self.fill_queue(shell);
                if self.queue_tab {
                    self.show_selected(shell);
                }
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
        if !self.queue_tab && self.selected.as_deref() == Some(path) {
            self.show_selected(shell);
        }
    }

    /// Watch the replay at `path` from `side`'s console (none: its recorder's): the two cards while it is made
    /// ready, then the viewer (`BattleEvent::Watching`); with `export`, in its export mode, paused at its start.
    fn watch(&mut self, shell: &mut Shell, path: &Path, side: Option<u8>, export: bool) {
        self.export_due = export;
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
        // (The sides of a match whose setups weren't open aren't kept at all.)
        self.sides = replay.info.open_setups.then(|| nettai_match::binary::read_match(ready.content(), &replay.match_bytes).ok()).flatten().map(|m| Box::new(m.sides));
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
        let played = w.path().to_path_buf();
        if let Some(e) = self.export.take() {
            e.close(shell);
        }
        self.close_up_next(shell);
        self.overlays = w.overlays();
        shell.lock_game(false);
        shell.ui().global::<ReplaysView>().set_watching(false);
        shell.ui().global::<ReplaysView>().set_builds_shown(false);
        shell.nav.enter(ScreenId::Replays.name(), self.main());
        // (The replay played is the one selected, the cursor on its row, where it is listed: the user, "when exiting
        // a replay, it should highlight the replay that was played instead of moving the cursor to the start". The
        // Queue tab keeps its own.)
        if !self.queue_tab {
            self.selected = Some(played);
        }
        self.still.surface.refresh();
        self.fill(shell);
        if !self.queue_tab
            && let Some(at) = self.selected.as_ref().and_then(|path| self.rows.iter().position(|&i| self.library.entries[i].path == *path))
        {
            shell.nav.focus("list", at as i32);
        }
    }

    /// Watch the queue from its entry `from` on: the first of them that plays (none: refused), the ones before it
    /// passed and out of the queue. The queue's rows are as the list last made them.
    fn watch_queue(&mut self, shell: &mut Shell, from: usize) {
        let plays = self.plays();
        let Some(path) = self.queue_mut(shell).and_then(|q| q.start(from, &plays)) else { return shell.sound.play(Cue::Refused) };
        self.queue_chosen = 0;
        self.fill_queue(shell);
        self.queued = true;
        self.current = true;
        self.watch(shell, &path, None, false);
    }

    /// While the queue plays: its entry `from` (counted from its head, the replay watched while it is still there)
    /// on, the first that plays, the ones before it out of the queue (the replay watched among them: it is done);
    /// none that plays: refused. The replay watched is left (its speed and what is on over its picture go on to the
    /// next), and the next is made ready behind the two cards. Back while they show goes to the list.
    fn play_from(&mut self, shell: &mut Shell, from: usize) {
        let plays = self.plays();
        let queued = self.queued;
        let Some(path) = self.queue_mut(shell).filter(|_| queued).and_then(|q| q.start(from, &plays)) else { return shell.sound.play(Cue::Refused) };
        self.close_up_next(shell);
        self.carry_speed = self.watching.as_ref().map(Watching::speed);
        if let Some(e) = self.export.take() {
            e.close(shell);
        }
        self.queue_chosen = 0;
        self.current = true;
        self.stop_watching(shell);
        self.watch(shell, &path, None, false);
    }

    /// The queue's next (the replay watched skipped, if it is still the head): refused where none plays after it,
    /// or where the queue isn't playing.
    fn next_in_queue(&mut self, shell: &mut Shell) {
        self.play_from(shell, self.current as usize);
    }

    /// The replay watched was played to its end: it leaves the queue (once).
    fn played_out(&mut self, shell: &mut Shell) {
        if !std::mem::take(&mut self.current) {
            return;
        }
        if let Some(queue) = self.queue_mut(shell) {
            queue.done();
        }
        self.fill_queue(shell);
        self.show_queue(shell);
    }

    /// The queue, in the viewer, while it plays: the entries up next (after the replay watched while it is the
    /// head), whether one of them plays, and Autoplay next.
    fn show_queue(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let skip = self.current as usize;
        let rows = if self.queued { self.queue_rows.get(skip..).unwrap_or_default().to_vec() } else { Vec::new() };
        view.set_queued(self.queued);
        view.set_has_next(self.queued && Queue::first_from(skip, &self.plays()).is_some());
        view.set_up_next(ModelRc::new(VecModel::from(rows)));
        view.set_autoplay(autoplay(shell));
        self.hud = None;
    }

    /// Open the popup of what is up next by the queue's button, or close it (the button again).
    fn toggle_up_next(&mut self, shell: &mut Shell) {
        if self.up_next {
            return self.close_up_next(shell);
        }
        if !self.queued {
            return shell.sound.play(Cue::Refused);
        }
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let count = view.get_up_next().row_count();
        // (Above the button where there is room, else under it; at its right edge on a desktop, across a phone.)
        let by = shell.rect_of(&Key::new("up-next", 0));
        let window = ui.window().size().to_logical(ui.window().scale_factor());
        let width = if shell.scale.phone { window.width - 32.0 } else { UP_NEXT_WIDTH };
        let rows = count.clamp(1, UP_NEXT_ROWS);
        let height = UP_NEXT_ROOM + rows as f32 * 44.0 + 8.0;
        let y = if by.y - 6.0 - height >= 8.0 { by.y - 6.0 - height } else { by.bottom() + 6.0 };
        let x = if shell.scale.phone { 16.0 } else { (by.right() - width).clamp(8.0, (window.width - width - 8.0).max(8.0)) };
        view.set_up_next_x(x);
        view.set_up_next_y(y);
        view.set_up_next_width(width);
        view.set_up_next_rows(rows as i32);
        view.set_up_next_open(true);
        self.up_next = true;
        // (On its first row; with none, on NEXT, or where no replay plays after this one, on Autoplay next.)
        let first = if count > 0 {
            UP_NEXT_ROW
        } else if view.get_has_next() {
            "upnext.next"
        } else {
            "upnext.autoplay"
        };
        shell.nav.open(UP_NEXT, Key::new(first, 0));
    }

    fn close_up_next(&mut self, shell: &mut Shell) {
        if std::mem::take(&mut self.up_next) {
            shell.nav.close(UP_NEXT);
            shell.ui().global::<ReplaysView>().set_up_next_open(false);
        }
    }

    /// The selected replay added to the queue: at its end, or (`first`) at its start, what plays next. Its
    /// opponent's name flies from `from` (its row, or the button pressed) to the Queue tab.
    fn enqueue(&mut self, shell: &mut Shell, first: bool, from: (&str, i32)) {
        let Some(path) = self.selected.clone().filter(|_| !self.queue_tab && self.playable(shell)) else { return shell.sound.play(Cue::Refused) };
        let Some(queue) = self.queue_mut(shell) else { return };
        if first {
            queue.add_first(&path);
        } else {
            queue.add(&path);
        }
        self.fill_queue(shell);
        let row = self.rows.iter().position(|&i| self.library.entries[i].path == path).and_then(|at| self.model.row_data(at));
        let text = row.map(|r| r.opponent.to_string()).filter(|o| !o.is_empty()).unwrap_or_else(|| file_name(&path));
        shell.sound.play(Cue::Pick);
        shell.fly(from, ("tab", 1), slint::Image::default(), &text);
    }

    /// The Queue tab's chosen entry moved up (`by` -1) or down (refused at an end); the cursor goes with it where it
    /// was on the queue.
    fn move_entry(&mut self, shell: &mut Shell, by: i32) {
        let chosen = self.queue_chosen;
        match self.queue_mut(shell).and_then(|q| q.shift(chosen, by)) {
            Some(to) => {
                self.queue_chosen = to;
                self.fill_queue(shell);
                self.show_selected(shell);
                if shell.nav.focused().is_some_and(|k| k.id == "queue") {
                    shell.nav.focus("queue", to as i32);
                }
            }
            None => shell.sound.play(Cue::Refused),
        }
    }

    /// The Queue tab's chosen entry taken out: the one after it takes its place.
    fn remove_entry(&mut self, shell: &mut Shell) {
        let chosen = self.queue_chosen;
        let Some(queue) = self.queue_mut(shell).filter(|q| chosen < q.paths.len()) else { return shell.sound.play(Cue::Refused) };
        queue.remove(chosen);
        let empty = queue.paths.is_empty();
        self.fill_queue(shell);
        self.show_selected(shell);
        if empty {
            self.close_sheet(shell);
            self.set_tab(shell, false);
        } else if shell.nav.focused().is_some_and(|k| k.id == "queue") {
            shell.nav.focus("queue", self.queue_chosen as i32);
        }
    }

    /// Ask whether to clear the queue: once, Cancel focused.
    fn ask_clear(&mut self, shell: &mut Shell) {
        let count = self.queue(shell).map_or(0, |q| q.paths.len());
        if count == 0 {
            return shell.sound.play(Cue::Refused);
        }
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        shell.ask(Dialog {
            tag: "clear",
            title: view.get_clear_title().into(),
            message: view.invoke_replay_count(count as i32).into(),
            buttons: vec![button(&view.get_cancel(), ButtonKind::Normal), DialogButton { text: view.get_clear(), kind: ButtonKind::Normal, danger: true }],
            focus: 0,
            cancel: 0,
        });
    }

    /// What the menu of the replay shown beside the list offers (a right click on its row): a replay's (WATCH, ADD
    /// TO QUEUE, PLAY NEXT, EXPORT VIDEO where it plays; SHOW FILE, DELETE), or a queue entry's (WATCH
    /// FROM HERE where it plays; MOVE UP, MOVE DOWN, REMOVE).
    fn menu_actions(&self, shell: &Shell) -> Vec<MenuAction> {
        use MenuAction::*;
        let plays = shell.ui().global::<ReplaysView>().get_playable();
        match (self.queue_tab, plays) {
            (false, true) => vec![Watch, AddToQueue, AddToStart, Export, ShowFile, Delete],
            (false, false) => vec![ShowFile, Delete],
            (true, true) => vec![WatchFromHere, MoveUp, MoveDown, Remove],
            (true, false) => vec![MoveUp, MoveDown, Remove],
        }
    }

    /// The menu of the replay shown beside the list, by the row a right click was on (`at`): its choice is heard as
    /// `replay-menu`'s.
    fn open_menu(&mut self, shell: &mut Shell, at: &Key) {
        if self.shown_path().is_none() {
            return shell.sound.play(Cue::Refused);
        }
        let actions = self.menu_actions(shell);
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let words = actions
            .iter()
            .map(|a| match a {
                MenuAction::Watch => view.get_watch_word(),
                MenuAction::AddToQueue => view.get_add_word(),
                MenuAction::AddToStart => view.get_start_word(),
                MenuAction::Export => view.get_export_word(),
                MenuAction::ShowFile => view.get_file_word(),
                MenuAction::Delete => view.get_delete(),
                MenuAction::WatchFromHere => view.get_from_here_word(),
                MenuAction::MoveUp => view.get_up_word(),
                MenuAction::MoveDown => view.get_down_word(),
                MenuAction::Remove => view.get_remove_word(),
            })
            .map(|w| w.to_string())
            .collect();
        let apart = actions.iter().position(|a| matches!(a, MenuAction::Delete | MenuAction::Remove));
        shell.pick_menu("replay-menu", 0, at, words, apart);
    }

    /// WATCH's arrow (a phone's sheet's, `sheet`): the other ways to watch the selected replay, under the two.
    fn open_watch_menu(&mut self, shell: &mut Shell, sheet: bool) {
        if !self.playable(shell) || self.queue_tab {
            return shell.sound.play(Cue::Refused);
        }
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let words = vec![view.get_add_word().to_string(), view.get_start_word().to_string()];
        let (watch, arrow) = if sheet { ("sheet.watch", "sheet.watch-more") } else { ("watch", "watch-more") };
        shell.pick_split("watch-more", sheet as i32, [&Key::new(watch, 0), &Key::new(arrow, 0)], words);
    }

    /// A choice of the menu, by its place in it (a name flies from the row it was opened on).
    fn menu_chosen(&mut self, shell: &mut Shell, choice: i32) {
        let from = match (self.queue_tab, self.selected.as_ref()) {
            (false, Some(path)) => self.rows.iter().position(|&i| self.library.entries[i].path == *path).map_or(("watch", 0), |at| ("list", at as i32)),
            _ => ("watch", 0),
        };
        match usize::try_from(choice).ok().and_then(|at| self.menu_actions(shell).get(at).copied()) {
            Some(MenuAction::Watch) => self.act(shell, "watch"),
            Some(MenuAction::AddToQueue) => self.enqueue(shell, false, from),
            Some(MenuAction::AddToStart) => self.enqueue(shell, true, from),
            Some(MenuAction::Export) => self.act(shell, "export"),
            Some(MenuAction::ShowFile) => self.act(shell, "file"),
            Some(MenuAction::Delete) => self.act(shell, "delete"),
            Some(MenuAction::WatchFromHere) => self.watch_queue(shell, self.queue_chosen),
            Some(MenuAction::MoveUp) => self.move_entry(shell, -1),
            Some(MenuAction::MoveDown) => self.move_entry(shell, 1),
            Some(MenuAction::Remove) => self.remove_entry(shell),
            None => {}
        }
    }

    /// A frame of the viewer: its controls shown or hidden, and the queue's next started where it is due.
    ///
    /// The controls show while the player has done anything in the last [`HUD_IDLE`] (or the replay has just
    /// begun), while the replay doesn't play (paused, at its end, held by a drag), while a setting is taken,
    /// while the builds, the export, the ⋯ menu or the popup of what is up next are open, and always on a phone; the pointer
    /// resting on the transport holds them too (the UI's own). Hidden, the cursor goes with them.
    ///
    /// A replay of the queue played to its end leaves the queue at once; with Autoplay next on, the next that plays
    /// starts [`NEXT_AFTER`] later.
    fn follow_viewer(&mut self, shell: &mut Shell, now: Instant) {
        let Some(w) = &self.watching else { return };
        let lately = [shell.last_input, self.woke].into_iter().flatten().any(|at| now.saturating_duration_since(at) < HUD_IDLE);
        let open = w.builds_shown() || self.export.is_some() || self.up_next || shell.picker_open();
        let shown = shell.scale.phone || lately || !w.playing() || shell.nav.grabbed() || open;
        if self.hud != Some(shown) {
            self.hud = Some(shown);
            shell.ui().global::<ReplaysView>().set_hud(shown);
        }
        let ended = w.ended_for(now);
        let held = open || shell.dialog.is_some();
        if !self.queued || ended.is_none() {
            return;
        }
        self.played_out(shell);
        if ended.is_some_and(|t| t >= NEXT_AFTER) && autoplay(shell) && !held && Queue::first_from(0, &self.plays()).is_some() {
            self.play_from(shell, 0);
        }
    }

    /// The viewer into its export mode (docs/design/app-ui.md §2.96): the range the whole match, its options over the
    /// transport, the focus on the range's start. The replay goes on as it was (the builds are put away: the options
    /// are where they end).
    fn enter_export(&mut self, shell: &mut Shell) {
        let Some(w) = &mut self.watching else { return };
        if self.export.is_some() {
            return;
        }
        let Some(entry) = self.library.entries.iter().find(|e| e.path.as_path() == w.path()) else { return shell.sound.play(Cue::Refused) };
        let Ok(summary) = &entry.read else { return shell.sound.play(Cue::Refused) };
        w.hide_builds(shell);
        let (path, side) = (w.path().to_path_buf(), w.side());
        self.export = Some(Export::open(shell, &path, summary, side, w.both()));
        self.hud = None;
        shell.nav.focus("export.handle", 0);
    }

    /// Out of the export mode (an export under way is stopped), the focus back on the bar.
    fn leave_export(&mut self, shell: &mut Shell) {
        if let Some(e) = self.export.take() {
            e.close(shell);
            shell.nav.focus("seek", 0);
        }
    }

    /// The export mode's handles, buttons and settings that the viewer has a part in (their names after `export.`):
    /// each handle (`index` 0 the start, 1 the end) dragged (`choice` of 10,000 of the replay; `held` false: let go
    /// there), Enter on one going to it, the range's start or end set where the viewer is, the side shown. True: it
    /// was one of them.
    fn export_with_viewer(&mut self, shell: &mut Shell, what: &str, index: i32, place: Option<(i32, bool)>) -> bool {
        let (Some(w), Some(e)) = (&mut self.watching, &mut self.export) else { return false };
        let end = index == 1;
        match (what, place) {
            ("handle", Some((part, held))) => {
                let tick = (w.length() as u64 * part.clamp(0, 10_000) as u64 / 10_000) as u32;
                // (The picture follows the handle, in steps while it is held, exactly where it is let go.)
                let to = e.drag(shell, end, tick).unwrap_or(e.handle(end));
                w.pause();
                w.drag_to(to, !held);
            }
            ("handle", None) => w.go_to(e.handle(end)),
            ("start-here" | "end-here", _) => {
                if !e.here(shell, what == "end-here", w.shown()) {
                    shell.sound.play(Cue::Refused);
                }
            }
            ("view", Some((side, _))) => {
                w.set_view(shell, side);
                e.set_side(shell, w.side());
            }
            _ => return false,
        }
        true
    }

    /// A handle nudged by `by` ticks, the picture going to where it is then.
    fn nudge(&mut self, shell: &mut Shell, end: bool, by: i32) {
        let (Some(w), Some(e)) = (&mut self.watching, &mut self.export) else { return };
        if let Some(to) = e.nudge(shell, end, by) {
            w.go_to(to);
        }
    }

    /// The handle the focus is on, in the export mode: false the start, true the end.
    fn handle_focused(&self, shell: &Shell) -> Option<bool> {
        let key = shell.nav.focused().filter(|k| k.id == "export.handle" && self.export.is_some())?;
        Some(key.index == 1)
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

    /// The actions of the panel beside the list, or of a phone's sheet: the replay's, or the queue entry's.
    fn act(&mut self, shell: &mut Shell, what: &str) {
        match what {
            "watch-from" => return self.watch_queue(shell, self.queue_chosen),
            "move-up" => return self.move_entry(shell, -1),
            "move-down" => return self.move_entry(shell, 1),
            "remove" => return self.remove_entry(shell),
            _ => {}
        }
        let Some(path) = self.selected.clone().filter(|_| !self.queue_tab) else { return };
        match what {
            // (WATCH watches the one replay: the queue is made ahead of time.)
            "watch" => {
                self.queued = false;
                self.watch(shell, &path, None, false);
            }
            // (The viewer, in its export mode, paused at the start.)
            "export" if self.playable(shell) => {
                self.queued = false;
                self.watch(shell, &path, None, true);
            }
            "export" => shell.sound.play(Cue::Refused),
            "file" => nettai_platform::dialog::show_file(&path),
            "delete" => self.ask_delete(shell),
            "watch-more" => self.open_watch_menu(shell, self.sheet),
            _ => {}
        }
    }

    /// Whether the replay shown beside the list plays here.
    fn playable(&self, shell: &Shell) -> bool {
        self.shown_path().is_some() && shell.ui().global::<ReplaysView>().get_playable()
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
            self.queued = false;
            self.watch(shell, &path, None, false);
        }
    }
}

impl Screen for Replays {
    /// The video's file, made as the export's EXPORT asked (in the browser, in the page's storage till it is whole).
    fn chosen(&mut self, shell: &mut Shell, purpose: &str, _paths: &[std::path::PathBuf]) {
        if purpose == export::VIDEO_FILE
            && let Some(export) = self.export.as_mut()
        {
            export.chosen_file(shell);
        }
    }

    fn main(&self) -> &'static str {
        "list"
    }

    fn show(&mut self, shell: &mut Shell) {
        // The game or the language may have changed under it: the stills are drawn anew.
        self.still.frame = None;
        self.still.asked = None;
        self.still.text = shell.ready().and_then(|(_, ready)| ready.loaded.font_in(shell.i18n.code).filter(|_| shell.settings.enhanced)).map(TextRenderer::new);
        if let Some(w) = &mut self.watching {
            shell.nav.enter(VIEWER, "seek");
            shell.lock_game(true);
            // (Opened in its export mode, from the list: paused at its start.)
            let export = std::mem::take(&mut self.export_due);
            if export {
                w.pause();
            }
            w.show(shell);
            if export {
                self.enter_export(shell);
            } else if let Some(e) = &mut self.export {
                e.show(shell);
            }
            self.show_queue(shell);
            return;
        }
        shell.ui().global::<ReplaysView>().set_watching(false);
        shell.ui().global::<ReplaysView>().set_tab(self.queue_tab as i32);
        if !shell.scale.phone {
            self.close_sheet(shell);
        }
        // The folder, listed again: a match just recorded is there. (What playing out gave is kept between runs.)
        self.library.cache.get_or_insert_with(crate::replays::cache_file);
        self.library.list(&crate::replays::folder());
        self.fill(shell);
    }

    fn hide(&mut self, shell: &mut Shell) {
        if let Some(e) = self.export.take() {
            e.close(shell);
        }
        self.close_sheet(shell);
        self.close_up_next(shell);
        if let Some(w) = self.watching.take() {
            self.overlays = w.overlays();
            shell.ui().global::<ReplaysView>().set_watching(false);
        }
        self.opening = None;
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if let Some(what) = id.strip_prefix("export.") {
            // (View: the viewer's side, which the export follows.)
            let other = self.watching.as_ref().map_or(0, |w| w.side() as i32 ^ 1);
            if self.export_with_viewer(shell, what, index, (what == "view").then_some((other, false))) {
                return;
            }
            if let Some(e) = &mut self.export
                && e.activate(shell, what)
            {
                self.leave_export(shell);
            }
            return;
        }
        if self.watching.is_some() {
            match id {
                "up-next" => return self.toggle_up_next(shell),
                "upnext.close" => return self.close_up_next(shell),
                "upnext.next" => return self.next_in_queue(shell),
                "upnext.autoplay" => return set_autoplay(shell, !autoplay(shell)),
                // (A row up next: its replay, the ones before it passed; one that is missing is refused.)
                UP_NEXT_ROW => {
                    let at = index.max(0) as usize + self.current as usize;
                    return if self.plays().get(at) == Some(&true) { self.play_from(shell, at) } else { shell.sound.play(Cue::Refused) };
                }
                _ => {}
            }
        }
        if let Some(w) = &mut self.watching {
            match id {
                "seek" | "play" => w.toggle(),
                "round-start" => w.round(-1),
                "next-round" => w.round(1),
                "builds.sheet" => w.toggle_builds(shell),
                "more" => {
                    let rows = w.menu(&shell.ui().global::<ReplaysView>(), self.export.is_some());
                    shell.pick_marked("more", 0, &Key::new("more", 0), rows);
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
            "tab" => self.set_tab(shell, index == 1),
            "watch-queue" => self.watch_queue(shell, 0),
            "clear" => self.ask_clear(shell),
            // (An entry of the queue: chosen, as a replay of the list is; Enter watches the queue from it.)
            "queue" => {
                self.queue_chosen = index.max(0) as usize;
                self.fill_queue(shell);
                self.show_selected(shell);
                if shell.scale.phone {
                    if self.queue_path.is_some() && !self.sheet {
                        self.sheet = true;
                        shell.ui().global::<ReplaysView>().set_detail(true);
                        shell.nav.open("sheet.", crate::nav::Key::new("sheet.watch-from", 0));
                    }
                } else if !shell.nav.pointing() {
                    self.watch_queue(shell, self.queue_chosen);
                }
            }
            "sheet.close" => self.close_sheet(shell),
            _ => self.act(shell, id.strip_prefix("sheet.").unwrap_or(id)),
        }
    }

    fn step(&mut self, shell: &mut Shell, id: &str, index: i32, by: i32) {
        if let Some(what) = id.strip_prefix("export.") {
            if what == "view" {
                self.export_with_viewer(shell, what, index, Some(((by > 0) as i32, false)));
            } else if let Some(e) = &mut self.export {
                e.step(shell, what, by);
            }
            return;
        }
        match (&mut self.watching, id) {
            (Some(w), "seek") => w.skip(5 * by),
            (None, "kind") => {
                self.online_only = by > 0;
                self.fill(shell);
            }
            _ => {}
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, index: i32, choice: i32) {
        if let Some(what) = id.strip_prefix("export.") {
            // (A handle held at `choice` of 10,000 of the replay, `index` 0 the start and 1 the end; `handle-done`: let
            // go there. View: the viewer's side.)
            let place = match what {
                "handle" => Some((choice, true)),
                "handle-done" => Some((choice, false)),
                "view" => Some((choice, false)),
                _ => None,
            };
            let what = if what == "handle-done" { "handle" } else { what };
            if place.is_some() && self.export_with_viewer(shell, what, index, place) {
                return;
            }
            if let Some(e) = &mut self.export {
                e.choose(shell, what, choice);
            }
            return;
        }
        match (&mut self.watching, id) {
            // (The bar: held at `choice` of 10,000, or with `index` 1 let go there.)
            (Some(w), "seek") => w.drag(choice, index == 1),
            // (The pointer over the bar, `index` 1, at `choice` of 10,000; 0, off it: the preview.)
            (Some(w), "seek-hover") => w.hover(shell, (index == 1).then_some(choice)),
            // (The transport's ⋯ menu: `Watching::menu`'s rows. PLAYBACK SPEED opens the speeds' menu in its place;
            // EXPORT VIDEO the export mode.)
            (Some(w), "more") => match MENU.get(choice.max(0) as usize) {
                Some(MenuItem::Export) => self.enter_export(shell),
                Some(MenuItem::Side) => {
                    let rows = w.side_menu(&shell.ui().global::<ReplaysView>());
                    shell.nav.focus("more", 0);
                    shell.pick_marked("side", 0, &Key::new("more", 0), rows);
                }
                Some(MenuItem::Speed) => {
                    let rows = w.speed_menu();
                    // (The focus is put back on ⋯ first, for the speeds' menu to give it back to.)
                    shell.nav.focus("more", 0);
                    shell.pick_marked("speed", 0, &Key::new("more", 0), rows);
                }
                Some(&item) => {
                    w.choose(shell, item);
                    // (In the export mode, the side exported is the one shown.)
                    if let Some(e) = &mut self.export {
                        e.set_side(shell, w.side());
                    }
                }
                None => {}
            },
            (Some(w), "speed") => w.choose_speed(shell, choice),
            // (The side's menu: the side shown, or both on one screen; in the export mode, the side exported with
            // it.)
            (Some(w), "side") => {
                w.choose_side(shell, choice);
                if let Some(e) = &mut self.export {
                    e.set_side(shell, w.side());
                }
            }
            (None, "kind") => {
                self.online_only = choice == 1;
                self.fill(shell);
            }
            // WATCH's arrow's menu (`index` 1: a phone's sheet's): ADD TO QUEUE, PLAY NEXT. (A name flies
            // from the arrow.)
            (None, "watch-more") => {
                let from = if index == 1 { ("sheet.watch-more", 0) } else { ("watch-more", 0) };
                self.enqueue(shell, choice == 1, from);
            }
            // The menu of the replay or the queue entry shown beside the list, by a right click on a row (the press
            // chose it first).
            (None, "replay-menu") => self.menu_chosen(shell, choice),
            (None, "row-menu") => match shell.nav.focused().map(|k| k.id.clone()).as_deref() {
                Some(list @ ("list" | "queue")) => {
                    let at = Key::new(list, index);
                    self.open_menu(shell, &at);
                }
                _ => {}
            },
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
        if self.watching.is_some() {
            return;
        }
        if id == "queue" {
            let at = index.max(0) as usize;
            if at != self.queue_chosen && at < self.queue_rows.len() {
                self.queue_chosen = at;
                self.fill_queue(shell);
                self.show_selected(shell);
            }
            return;
        }
        if id != "list" {
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

    /// Right from the list goes to the chosen replay's pane, its first button that can be pressed (the pane's other
    /// controls are far below the row, and the All/Online switch over the list nearer by place).
    fn route(&mut self, shell: &mut Shell, action: Action) -> bool {
        // (A handle of the export's range, on a pad: Left and Right nudge it a frame. The keys' own are `key`'s.)
        if let Some(end) = self.handle_focused(shell)
            && matches!(action, Action::Left | Action::Right)
        {
            self.nudge(shell, end, if action == Action::Left { -1 } else { 1 });
            return true;
        }
        let list = shell.nav.focused().map(|k| k.id.clone());
        if action != Action::Right || self.watching.is_some() || shell.scale.phone {
            return false;
        }
        match list.as_deref() {
            Some("list") => shell.nav.focus_first(&shell.ui(), &["watch", "watch-more", "export", "file", "delete"]),
            Some("queue") => shell.nav.focus_first(&shell.ui(), &["watch-from", "move-up", "move-down", "remove"]),
            _ => false,
        }
    }

    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        if self.sheet {
            return false;
        }
        // (On a handle of the export's range, Left and Right nudge it a frame, with Shift a second.)
        if let Some(end) = self.handle_focused(shell)
            && matches!(key.code, KeyCode::ArrowLeft | KeyCode::ArrowRight)
        {
            let by = if key.shift { tick_at(Duration::from_secs(1)) as i32 } else { 1 };
            self.nudge(shell, end, if key.code == KeyCode::ArrowLeft { -by } else { by });
            return true;
        }
        // (N the queue's next replay, while it plays: its popup open or not.)
        if self.watching.is_some() && key.code == KeyCode::KeyN {
            if !key.repeat {
                self.next_in_queue(shell);
            }
            return true;
        }
        if self.up_next {
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
        if key.code == KeyCode::KeyE && !self.queue_tab && !shell.nav.on_field() && !key.repeat {
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
            Some(w) if !self.up_next && button == gamepad_facade::Button::Back => {
                w.toggle_builds(shell);
                true
            }
            _ => false,
        }
    }

    fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        if self.up_next {
            return false;
        }
        // (On a handle of the export's range, the shoulders nudge it a second.)
        if let Some(end) = self.handle_focused(shell)
            && matches!(action, Action::Previous | Action::Next)
        {
            let by = tick_at(Duration::from_secs(1)) as i32;
            self.nudge(shell, end, if action == Action::Previous { -by } else { by });
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
            // (In the Queue tab Delete takes the entry out of the queue: its file stays.)
            Action::Delete if self.queue_tab => self.remove_entry(shell),
            Action::Delete if !shell.nav.on_field() && self.selected.is_some() => self.ask_delete(shell),
            Action::Alternate if !self.queue_tab && self.playable(shell) => self.act(shell, "export"),
            _ => return false,
        }
        true
    }

    fn back(&mut self, shell: &mut Shell) -> bool {
        if self.sheet {
            self.close_sheet(shell);
            return true;
        }
        if self.up_next {
            self.close_up_next(shell);
            return true;
        }
        if let Some(w) = &mut self.watching
            && w.hide_builds(shell)
        {
            return true;
        }
        // (The export mode: an export under way is stopped, else the mode is left.)
        if let Some(e) = &mut self.export {
            if e.back(shell) {
                self.leave_export(shell);
            }
            return true;
        }
        if self.watching.is_some() {
            // (From the queue, its head is the one chosen in the Queue tab: the replay watched stays there where it
            // wasn't played to its end.)
            if std::mem::take(&mut self.queued) {
                self.queue_chosen = 0;
            }
            self.stop_watching(shell);
            // (The queue played out: no Queue tab to come back to.)
            if self.queue_tab && self.queue_rows.is_empty() {
                self.set_tab(shell, false);
            } else if self.queue_tab {
                shell.nav.focus("queue", self.queue_chosen as i32);
            }
            return true;
        }
        if self.opening.take().is_some() && shell.loading_battle() {
            shell.cancel_loading();
            return true;
        }
        false
    }

    fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) {
        if tag == "clear" && choice == 1 {
            if let Some(queue) = self.queue_mut(shell) {
                queue.clear();
            }
            self.queue_chosen = 0;
            self.close_sheet(shell);
            self.fill_queue(shell);
            self.show_selected(shell);
            self.set_tab(shell, false);
            return;
        }
        if tag != "delete" || choice != 1 {
            return;
        }
        let Some(path) = self.selected.clone() else { return };
        self.close_sheet(shell);
        match nettai_platform::fs::remove_file(&path) {
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
        let mut watching = Watching::new(viewer, path, self.sides.take(), self.overlays);
        if let Some(speed) = self.carry_speed.take() {
            watching.set_speed(speed);
        }
        self.watching = Some(watching);
        self.woke = Some(Instant::now());
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
                self.queued = false;
                self.stop_watching(shell);
                shell.fail(UserError::Other);
                return;
            }
            self.follow_viewer(shell, now);
            return;
        }
        // The selected replay's still, where its box is on screen.
        let shown_path = self.shown_path().cloned();
        let Still { surface, frame, text, .. } = &mut self.still;
        if let Some((path, frame)) = frame
            && shown_path.as_ref() == Some(&*path)
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
