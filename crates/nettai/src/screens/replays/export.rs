//! A replay's export as video (docs/design/app-ui.md §2.96; docs/app.md §16): the viewer's export mode, its range
//! and its options, and the export itself, which is nettai-video's `export` on a thread of its own: it plays the
//! replay through a viewer of its own there and pipes the pictures and the sound to `ffmpeg`, which writes the MP4.
//!
//! The thread tells how far it is and is told to stop through what the two share ([`Shared`]); the export mode
//! reads it each frame. The file takes its name only once it is whole, the MP4's head put before its samples
//! (nettai-video's `Made`, the platform's `Stream::finish`): stopped or failed, nothing of it is left. What went
//! wrong is said in the options' line, in the player's words (`UserError::of_export`); the app saves an MP4 and
//! nothing else.
//!
//! The part exported is a range of the replay's ticks, shown on the viewer's bar between two handles: the whole
//! match at first; a round, or the whole match again, by the Range's list; or any part by the handles (dragged,
//! nudged by a frame or a second, or set where the viewer is). The range is always whole: its start before its end,
//! both within the replay ([`moved`], [`set_here`]). One that is no round and not the whole match is the player's
//! own (Custom), and the Range's list keeps the last such to go back to.

use super::{clock, name, played};
use crate::errors::UserError;
use crate::replays::Summary;
use crate::shell::Shell;
use nettai_frontend::game::{GraphicsMode, Loaded, Sound};
use nettai_frontend::player::time_of;
use nettai_ui::{ExportState, ReplaysView};
use nettai_platform::dialog::{Filter, Stream};
use nettai_video::{Encoding, Options, Part, Progress, Sides, Titles};
use slint::ComponentHandle;
use std::ops::{ControlFlow, Range};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// The sizes offered, in multiples of the frame's 240 by 160, and the one chosen at first. They are even: an
/// MP4 keeps its color at half the resolution, and at an odd multiple every pixel's edge fringes
/// (docs/frontend.md §8, "Video").
const SIZES: [u32; 3] = [2, 4, 6];
const FIRST_SIZE: usize = 1;

/// The sides offered, in the Sides switch's order: One, Both (on one screen), Side by side.
const SIDES: [Sides; 3] = [Sides::One, Sides::Both, Sides::SideBySide];

/// Where the folder last exported to is remembered (the settings' `remembered`).
const FOLDER: &str = "replays.export-folder";

/// The most of a path the options show (its middle is left out past it).
const PATH_SHOWN: usize = 42;

/// What the video's file is for, as `Replays` hears it once it is made (`Screen::chosen`).
pub const VIDEO_FILE: &str = "video";

thread_local! {
    /// The video's file as it was made, till `Replays` hears of it.
    static CHOSEN: std::cell::RefCell<Option<std::io::Result<Option<Stream>>>> = const { std::cell::RefCell::new(None) };
}

/// What the export's thread and the export mode share.
#[derive(Default)]
struct Shared {
    progress: Option<Progress>,
    done: Option<Result<(), nettai_video::Error>>,
}

/// An export under way. Dropped, it is stopped: the thread is told, and ends at its next frame, its file let go.
struct Running {
    stop: Arc<AtomicBool>,
    shared: Arc<Mutex<Shared>>,
}

/// What the export's thread and the export mode share, taken as the page's main thread may take a lock.
fn share(shared: &Mutex<Shared>) -> std::sync::MutexGuard<'_, Shared> {
    nettai_platform::thread::lock(shared).unwrap_or_else(|e| e.into_inner())
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The viewer's export mode.
pub struct Export {
    /// The replay's file, and what it says of itself.
    replay: PathBuf,
    summary: Summary,
    size: usize,
    /// The console exported: the viewer's, which it follows.
    side: u8,
    /// `side`'s console alone, both consoles on one screen (`side`'s at the left, one sound track: both consoles'),
    /// or both side by side (`side`'s at the left, its sound the first track, the other's the second); by its place
    /// among [`SIDES`].
    sides: usize,
    /// The ticks exported: the start before the end, both within the replay.
    range: Range<u32>,
    /// The last range of the player's own (no round, not the whole match), which the Range's list offers as Custom.
    custom: Option<Range<u32>>,
    to: PathBuf,
    /// The player chose the place: it isn't named again as the range changes.
    placed: bool,
    running: Option<Running>,
    state: ExportState,
    error: Option<UserError>,
}

/// `range` with its start (or, `end`, its end) moved to `tick` and held there: the start a tick before the end at
/// least, both within the replay's `length` ticks. (A handle dragged or nudged stops at the other.)
pub fn moved(range: &Range<u32>, length: u32, end: bool, tick: u32) -> Range<u32> {
    let length = length.max(1);
    let (start, stop) = (range.start.min(length - 1), range.end.clamp(1, length));
    if end { start..tick.clamp(start + 1, length) } else { tick.min(stop - 1)..stop }
}

/// `range` with its start (or, `end`, its end) set at `tick`, where the viewer is (SET START HERE, SET END HERE):
/// where that passes the other end, the other end goes to the replay's edge, its end (or its start).
pub fn set_here(range: &Range<u32>, length: u32, end: bool, tick: u32) -> Range<u32> {
    let length = length.max(1);
    if end {
        let tick = tick.clamp(1, length);
        (if range.start < tick { range.start } else { 0 })..tick
    } else {
        let tick = tick.min(length - 1);
        tick..(if range.end > tick { range.end.min(length) } else { length })
    }
}

/// The system's folder for videos.
fn videos() -> PathBuf {
    // (In the browser a video is made in the app's files and downloaded: `Shell::saved`.)
    if cfg!(target_arch = "wasm32") {
        return PathBuf::from("/videos");
    }
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    let home = if cfg!(target_os = "windows") { var("USERPROFILE") } else { var("HOME") };
    let found = if cfg!(target_os = "macos") {
        home.map(|h| h.join("Movies"))
    } else if cfg!(target_os = "windows") {
        home.map(|h| h.join("Videos"))
    } else {
        var("XDG_VIDEOS_DIR").or_else(|| home.map(|h| h.join("Videos")))
    };
    found.unwrap_or_default()
}

/// `text` as a file's name may hold it.
fn file_safe(text: &str) -> String {
    let kept: String = text.chars().map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { ' ' } else { c }).collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A path as the options show it: from the player's home where it is in it, its middle left out where it is
/// long; on a phone, whose box is short, the file's name alone.
fn shown(path: &Path, phone: bool) -> String {
    if phone {
        return path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    }
    let home = std::env::var_os(if cfg!(target_os = "windows") { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    let text = home.and_then(|h| path.strip_prefix(h).ok().map(|p| p.display().to_string())).unwrap_or_else(|| path.display().to_string());
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= PATH_SHOWN {
        return text;
    }
    let (head, tail) = (PATH_SHOWN / 3, PATH_SHOWN - PATH_SHOWN / 3 - 1);
    chars[..head].iter().chain(['…'].iter()).chain(chars[chars.len() - tail..].iter()).collect()
}

/// The game as the window shows it (the language, the graphics mode, the text as the settings have it), for a thread
/// of its own to play a replay on: the export's with the game's own sound, the bar's previews' with none.
pub fn game_for_thread(shell: &Shell, ready: &crate::games::Ready, sound: bool) -> Loaded {
    let mode = shell.settings.graphics();
    Loaded {
        found: ready.loaded.found.clone(),
        game: ready.loaded.game.clone(),
        graphics: ready.graphics(shell.i18n.code),
        mode,
        reduce_motion: shell.reduces_motion(),
        font: ready.loaded.font.clone().filter(|_| mode == GraphicsMode::Enhanced),
        // (With the game's own music, as the replay plays in the viewer: the pick is Practice's alone.)
        sound: if sound {
            crate::music::sound(&shell.games, ready, &crate::music::Music::GameOwn).map(|(banks, songs)| Sound { banks, songs, report: Default::default() })
        } else {
            None
        },
    }
}

impl Export {
    /// Begin the export mode for the replay at `replay`, shown from `side` (`both`: both consoles on one screen),
    /// the whole match chosen, the sides as the viewer shows them.
    pub fn open(shell: &mut Shell, replay: &Path, summary: &Summary, side: u8, both: bool) -> Export {
        let mut export = Export {
            replay: replay.to_path_buf(),
            summary: summary.clone(),
            size: FIRST_SIZE,
            side: side.min(1),
            sides: SIDES.iter().position(|&s| s == if both { Sides::Both } else { Sides::One }).unwrap_or(0),
            range: 0..summary.ticks.max(1),
            custom: None,
            to: PathBuf::new(),
            placed: false,
            running: None,
            state: ExportState::Idle,
            error: None,
        };
        export.name(shell);
        export.show(shell);
        shell.ui().global::<ReplaysView>().set_exporting(true);
        export
    }

    /// End the export mode: an export under way is stopped.
    pub fn close(self, shell: &mut Shell) {
        shell.ui().global::<ReplaysView>().set_exporting(false);
    }

    /// Whether an export is under way.
    pub fn running(&self) -> bool {
        self.running.is_some()
    }

    /// The replay's ticks.
    fn length(&self) -> u32 {
        self.summary.ticks.max(1)
    }

    /// The console exported follows the viewer's.
    pub fn set_side(&mut self, shell: &mut Shell, side: u8) {
        self.side = side.min(1);
        self.show(shell);
    }

    /// The rounds there are to choose: each where there is more than one.
    fn parts(&self) -> u32 {
        match self.summary.rounds.len() as u32 {
            1 => 0,
            rounds => rounds,
        }
    }

    /// A part's ticks: 0 the whole match, else that round's.
    fn range_of(&self, part: u32) -> Range<u32> {
        let rounds = &self.summary.rounds;
        match part {
            0 => 0..self.length(),
            n => rounds[n as usize - 1]..rounds.get(n as usize).copied().unwrap_or(self.length()),
        }
    }

    /// The part the range is (none: the player's own).
    fn part(&self) -> Option<u32> {
        (0..=self.parts()).find(|&part| self.range_of(part) == self.range)
    }

    /// The Range's list: the whole match, each round where there is more than one, and the last range of the player's
    /// own where there is one.
    fn choices(&self) -> Vec<Range<u32>> {
        (0..=self.parts()).map(|part| self.range_of(part)).chain(self.custom.clone()).collect()
    }

    /// A range as the Range's list says it, with its length.
    fn label(&self, view: &ReplaysView, range: &Range<u32>) -> String {
        let length = clock(time_of(range.len() as u64), true);
        match (0..=self.parts()).find(|&part| self.range_of(part) == *range) {
            Some(0) => view.invoke_whole(length.into()).into(),
            Some(n) => view.invoke_round_part(n as i32, length.into()).into(),
            None => view.invoke_custom_part(length.into()).into(),
        }
    }

    /// The round the range starts in, from 1.
    fn first_round(&self) -> u32 {
        self.summary.rounds.iter().filter(|&&r| r <= self.range.start).count().max(1) as u32
    }

    /// Where a handle is: the range's start, or (`end`) its end.
    pub fn handle(&self, end: bool) -> u32 {
        if end { self.range.end } else { self.range.start }
    }

    /// A handle dragged or nudged to `tick`, held by the other and the replay's ends: where it is then (none while it
    /// exports, when the range can't change).
    pub fn drag(&mut self, shell: &mut Shell, end: bool, tick: u32) -> Option<u32> {
        let range = moved(&self.range, self.length(), end, tick);
        self.set_range(shell, range).then(|| self.handle(end))
    }

    /// A handle nudged by `by` ticks (refused, and heard so, while it exports).
    pub fn nudge(&mut self, shell: &mut Shell, end: bool, by: i32) -> Option<u32> {
        let tick = self.handle(end).saturating_add_signed(by);
        let to = self.drag(shell, end, tick);
        if to.is_none() {
            shell.sound.play(crate::sound::Cue::Refused);
        }
        to
    }

    /// A handle set at `tick`, where the viewer is (SET START HERE, SET END HERE): the other goes to the replay's
    /// edge where this one would pass it.
    pub fn here(&mut self, shell: &mut Shell, end: bool, tick: u32) -> bool {
        let range = set_here(&self.range, self.length(), end, tick);
        self.set_range(shell, range)
    }

    /// The range made `range` (refused while it exports, silently: a drag asks at every move): the player's own, if it
    /// is no round and not the whole match.
    fn set_range(&mut self, shell: &mut Shell, range: Range<u32>) -> bool {
        if !self.open_to_change() {
            return false;
        }
        self.range = range;
        if self.part().is_none() {
            self.custom = Some(self.range.clone());
        }
        self.name(shell);
        self.show(shell);
        true
    }

    /// A size as the video's pixels and the multiple of the frame: with both side by side, twice as wide.
    fn size_label(size: u32, sides: Sides) -> String {
        format!("{} × {} ({size}×)", 240 * size * sides.count() as u32, 160 * size)
    }

    /// Name the file, unless the player chose its place: the day it was played, the opponent and the part, in
    /// the folder last exported to (at first the system's videos folder's `nettai`), with a number after it
    /// where a file of that name is there.
    fn name(&mut self, shell: &Shell) {
        if self.placed {
            return;
        }
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let folder = shell.settings.remembered.get(FOLDER).map(PathBuf::from).unwrap_or_else(|| videos().join("nettai"));
        let info = &self.summary.info;
        let mut stem = file_safe(&format!("{} {}", played(&view, info.when).file, name(&view, info, info.side ^ 1)));
        match self.part() {
            Some(0) => {}
            Some(round) => stem.push_str(&format!(" round {round}")),
            None => {
                // (The range's start and end, as the bar's clock says them, with dots: a colon is no file's.)
                let at = |tick: u32| clock(time_of(tick as u64), true).replace(':', ".");
                stem.push_str(&format!(" part {}–{}", at(self.range.start), at(self.range.end)));
            }
        }
        match SIDES[self.sides] {
            Sides::One => {}
            Sides::Both => stem.push_str(" both"),
            Sides::SideBySide => stem.push_str(" side by side"),
        }
        self.to = (1..).map(|n| folder.join(if n == 1 { format!("{stem}.mp4") } else { format!("{stem} ({n}).mp4") })).find(|p| !nettai_platform::fs::exists(p)).expect("some number is free");
    }

    /// Set the view from the export mode's state.
    pub fn show(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        view.set_export_size(Export::size_label(SIZES[self.size], SIDES[self.sides]).into());
        view.set_export_view(self.side as i32);
        view.set_export_sides(self.sides as i32);
        view.set_export_part(self.label(&view, &self.range).into());
        let at = |tick: u32| clock(time_of(tick as u64), true);
        view.set_export_range(format!("{}–{}", at(self.range.start), at(self.range.end)).into());
        let length = self.length() as f32;
        view.set_range_start(self.range.start as f32 / length);
        view.set_range_end(self.range.end as f32 / length);
        view.set_export_path(shown(&self.to, shell.scale.phone).into());
        view.set_export_state(self.state);
        view.set_export_error(self.error.as_ref().map(|e| e.line(&ui)).unwrap_or_default().into());
        if self.state != ExportState::Running {
            view.set_export_progress(0.0);
        }
    }

    /// The options are the player's to change: not while it exports.
    fn open_to_change(&mut self) -> bool {
        if self.running.is_some() {
            return false;
        }
        // (A change is the start of another export: what the last one came to is no longer said.)
        self.state = ExportState::Idle;
        self.error = None;
        true
    }

    /// The Range's choice at `at` (the whole match, a round, the last of the player's own).
    fn set_choice(&mut self, shell: &mut Shell, at: usize) {
        let choices = self.choices();
        let range = choices[at.min(choices.len() - 1)].clone();
        self.set_range(shell, range);
    }

    /// The sides exported, by their place among [`SIDES`] (the file's name says which).
    fn set_sides(&mut self, shell: &mut Shell, sides: usize) {
        self.sides = sides.min(SIDES.len() - 1);
        self.name(shell);
        self.show(shell);
    }

    /// Enter or a click on one of its controls (its name after `export.`). True: the export mode is to end.
    pub fn activate(&mut self, shell: &mut Shell, what: &str) -> bool {
        match what {
            "cancel" => return self.back(shell),
            "go" => self.start(shell),
            "size" if self.open_to_change() => shell.pick("export.size", 0, SIZES.iter().map(|&s| Export::size_label(s, SIDES[self.sides])).collect(), self.size),
            "part" if self.open_to_change() => {
                let ui = shell.ui();
                let view = ui.global::<ReplaysView>();
                let choices = self.choices();
                let chosen = choices.iter().position(|r| *r == self.range).unwrap_or(0);
                let options = choices.iter().map(|range| self.label(&view, range)).collect();
                shell.pick("export.part", 0, options, chosen);
            }
            "sides" if self.open_to_change() => self.set_sides(shell, (self.sides + 1) % SIDES.len()),
            "path" if self.open_to_change() => {
                let kind = shell.ui().global::<ReplaysView>().get_video_kind();
                if let Some(mut to) = choose(&kind, &self.to) {
                    // (An MP4 and nothing else.)
                    to.set_extension("mp4");
                    self.to = to;
                    self.placed = true;
                }
                self.show(shell);
            }
            _ => {}
        }
        false
    }

    /// Left or Right on one of its settings.
    pub fn step(&mut self, shell: &mut Shell, what: &str, by: i32) {
        if !self.open_to_change() {
            return;
        }
        match what {
            "size" => self.size = (self.size as i32 + by).clamp(0, SIZES.len() as i32 - 1) as usize,
            "sides" => return self.set_sides(shell, (self.sides as i32 + by.signum()).clamp(0, SIDES.len() as i32 - 1) as usize),
            _ => {}
        }
        self.show(shell);
    }

    /// A choice made outright: a row of a list, a side of the switch.
    pub fn choose(&mut self, shell: &mut Shell, what: &str, choice: i32) {
        if !self.open_to_change() {
            return;
        }
        match what {
            "size" => self.size = choice.clamp(0, SIZES.len() as i32 - 1) as usize,
            "sides" => return self.set_sides(shell, choice.max(0) as usize),
            "part" => return self.set_choice(shell, choice.max(0) as usize),
            _ => {}
        }
        self.show(shell);
    }

    /// Esc, B or CANCEL: an export under way is stopped (the export mode stays, to export again); else true, and the
    /// export mode is to end.
    pub fn back(&mut self, _shell: &mut Shell) -> bool {
        match &self.running {
            Some(running) => {
                running.stop.store(true, Ordering::Relaxed);
                false
            }
            None => true,
        }
    }

    /// EXPORT: the export starts, on a thread of its own, written to its file as it is made: on the desktop at the
    /// path the export mode says; in the browser in the page's storage, handed over as a download of that name once
    /// whole. The file is heard once this call is done (`Replays`' `Screen::chosen`, [`VIDEO_FILE`]).
    fn start(&mut self, shell: &mut Shell) {
        if !self.open_to_change() {
            return;
        }
        // (A file of that name made since the export mode began isn't written over, unless the player chose it.)
        self.name(shell);
        let kind = shell.ui().global::<ReplaysView>().get_video_kind().to_string();
        nettai_platform::dialog::save_stream_to(Filter { kind: &kind, extensions: &["mp4"] }, &self.to, |chosen| {
            CHOSEN.with(|kept| *kept.borrow_mut() = Some(chosen));
            let _ = slint::invoke_from_event_loop(|| crate::with_app(|app| app.chosen(VIDEO_FILE, &[])));
        });
    }

    /// The video's file, made (or none chosen, or it couldn't be): the export starts, written to it.
    pub fn chosen_file(&mut self, shell: &mut Shell) {
        let Some(chosen) = CHOSEN.with(|kept| kept.borrow_mut().take()) else { return };
        match chosen {
            Ok(Some(stream)) if self.open_to_change() => self.begin(shell, stream),
            Ok(_) => {}
            Err(why) => {
                crate::errors::log(&format!("exporting {}", self.replay.display()), &format_args!("{}: {why}", self.to.display()));
                self.fail(shell, UserError::CantWriteVideo);
            }
        }
    }

    /// The export started, written to `to` as it is made.
    fn begin(&mut self, shell: &mut Shell, to: Stream) {
        let Some((_, ready)) = shell.ready() else { return };
        let doing = format!("exporting {}", self.replay.display());
        let replay = match crate::replays::read(&self.replay) {
            Ok(replay) => replay,
            Err(why) => return self.fail(shell, UserError::of_replay_read(&doing, &why)),
        };
        // What the thread plays from: the game as the window shows it, shared.
        let game = game_for_thread(shell, &ready, true);
        let sides = SIDES[self.sides];
        // The chapters ("Round 1"...) and the sound's tracks (each side's player) in the words the window shows.
        let titles = {
            let ui = shell.ui();
            let view = ui.global::<ReplaysView>();
            let rounds = (1..=self.summary.rounds.len() as i32).map(|n| view.invoke_chapter(n).into()).collect();
            Titles { rounds, sides: [0, 1].map(|side| name(&view, &self.summary.info, side)) }
        };
        let part = match self.part() {
            Some(0) => Part::Whole,
            Some(round) => Part::Round(round),
            None => Part::Ticks(self.range.clone()),
        };
        let options = Options { scale: SIZES[self.size], view: Some(self.side), sides, part, titles };
        // (The platform's encoder: on the desktop the ffmpeg found here.)
        let encoding = Encoding::default();
        let (stop, shared) = (Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(Shared::default())));
        let (stopped, told) = (stop.clone(), shared.clone());
        let work = move || {
            let mut progress = |progress| {
                share(&told).progress = Some(progress);
                if stopped.load(Ordering::Relaxed) { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
            };
            // (Whole: the file kept, its head first. Canceled or failed: let go, nothing of it left.)
            let result = nettai_video::export(&game, &replay, &options, &encoding, to, &mut progress)
                .and_then(|made| made.body.finish(made.head).map_err(nettai_video::Error::Disk));
            share(&told).done = Some(result);
        };
        match nettai_platform::thread::spawn("export", work) {
            Ok(()) => {
                self.running = Some(Running { stop, shared });
                self.state = ExportState::Running;
                let ui = shell.ui();
                let view = ui.global::<ReplaysView>();
                view.set_export_progress(0.0);
                view.set_export_round(self.first_round() as i32);
                view.set_export_rounds(self.summary.rounds.len() as i32);
                self.show(shell);
                shell.nav.focus("export.cancel", 0);
            }
            Err(why) => {
                crate::errors::log(&doing, &why);
                self.fail(shell, UserError::ExportFailed);
            }
        }
    }

    fn fail(&mut self, shell: &mut Shell, error: UserError) {
        self.state = ExportState::Failed;
        self.error = Some(error);
        shell.sound.play(crate::sound::Cue::Refused);
        self.show(shell);
    }

    /// A frame: how far the export is, and how it ended once it has.
    pub fn frame(&mut self, shell: &mut Shell) {
        let Some(running) = &self.running else { return };
        let (progress, done) = {
            let mut shared = share(&running.shared);
            (shared.progress.take(), shared.done.take())
        };
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        if let Some(p) = progress {
            view.set_export_progress(p.fraction());
            view.set_export_round(p.round as i32);
            view.set_export_rounds(p.rounds as i32);
        }
        let Some(done) = done else { return };
        self.running = None;
        match done {
            Ok(()) => {
                self.state = ExportState::Done;
                // The next one goes to the same folder, under a name of its own.
                if let Some(folder) = self.to.parent() {
                    shell.settings.remembered.insert(FOLDER.into(), folder.display().to_string());
                    shell.save_settings();
                }
                self.placed = false;
                shell.sound.play(crate::sound::Cue::Pick);
                self.show(shell);
                self.name(shell);
            }
            Err(why) => match UserError::of_export(&why) {
                Some(error) => self.fail(shell, error),
                None => {
                    self.state = ExportState::Canceled;
                    self.show(shell);
                }
            },
        }
        shell.nav.focus(if self.state == ExportState::Done { "export.cancel" } else { "export.go" }, 0);
    }
}

/// Ask the player where to save the video, in the system's dialog, starting from where it is to go.
fn choose(kind: &str, to: &Path) -> Option<PathBuf> {
    let name = to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    nettai_platform::dialog::save_path(Filter { kind, extensions: &["mp4"] }, &name, Some(to))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file's name holds nothing a file system refuses, and no run of spaces.
    #[test]
    fn a_name_is_safe_for_a_file() {
        assert_eq!(file_safe("2026-10-11 a/b\\c: d?"), "2026-10-11 a b c d");
        assert_eq!(file_safe("  ネット\tバトラー "), "ネット バトラー");
        assert_eq!(file_safe(""), "");
    }

    /// A long path is shown with its start and its end, its middle left out; a short one whole.
    #[test]
    fn a_long_path_keeps_its_ends() {
        let path = Path::new("/a/folder/with/a/long/name/and/another/inside/it/2026-10-11 someone round 2.mp4");
        let long = shown(path, false);
        assert_eq!(long.chars().count(), PATH_SHOWN);
        assert!(long.starts_with("/a/folder/with") && long.ends_with("someone round 2.mp4") && long.contains('…'), "{long}");
        assert_eq!(shown(Path::new("/videos/a.mp4"), false), "/videos/a.mp4");
        assert_eq!(shown(path, true), "2026-10-11 someone round 2.mp4");
    }

    /// A handle dragged or nudged is held: the start stays a tick before the end at least, and both stay within the
    /// replay, however far it is taken.
    #[test]
    fn a_handle_is_held_by_the_other_and_the_ends() {
        let length = 1000;
        let range = 100..500;
        // Within: where it is taken.
        assert_eq!(moved(&range, length, false, 250), 250..500);
        assert_eq!(moved(&range, length, true, 750), 100..750);
        // Past the other: a tick before it, or after.
        assert_eq!(moved(&range, length, false, 500), 499..500);
        assert_eq!(moved(&range, length, false, 900), 499..500);
        assert_eq!(moved(&range, length, true, 100), 100..101);
        assert_eq!(moved(&range, length, true, 0), 100..101);
        // Past the replay's ends: at them.
        assert_eq!(moved(&range, length, true, 5000), 100..1000);
        assert_eq!(moved(&range, length, false, 0), 0..500);
        // (A range already at an end, nudged on: as it was.)
        assert_eq!(moved(&(0..1000), length, true, 1001), 0..1000);
        assert_eq!(moved(&(999..1000), length, false, 1000), 999..1000);
        // (Nothing makes it empty or reversed, whatever is asked.)
        for (start, end) in [(0, 1), (0, 1000), (998, 999), (999, 1000), (500, 501)] {
            for tick in [0, 1, 499, 500, 501, 999, 1000, u32::MAX] {
                for end_moved in [false, true] {
                    let r = moved(&(start..end), length, end_moved, tick);
                    assert!(r.start < r.end && r.end <= length, "{start}..{end}, {} to {tick}: {r:?}", if end_moved { "end" } else { "start" });
                }
            }
        }
    }

    /// SET START HERE and SET END HERE put the handle where the viewer is; where that passes the other handle, the
    /// other goes to the replay's edge, so the range is still whole and holds the time set.
    #[test]
    fn here_moves_the_other_handle_aside() {
        let length = 1000;
        let range = 100..500;
        assert_eq!(set_here(&range, length, false, 300), 300..500);
        assert_eq!(set_here(&range, length, true, 300), 100..300);
        // Past the end: the end goes to the replay's.
        assert_eq!(set_here(&range, length, false, 700), 700..1000);
        assert_eq!(set_here(&range, length, false, 500), 500..1000);
        // Before the start: the start goes to the replay's.
        assert_eq!(set_here(&range, length, true, 50), 0..50);
        assert_eq!(set_here(&range, length, true, 100), 0..100);
        // At the replay's very ends: a tick from them.
        assert_eq!(set_here(&range, length, false, 1000), 999..1000);
        assert_eq!(set_here(&range, length, true, 0), 0..1);
        for tick in [0, 1, 500, 999, 1000, u32::MAX] {
            for end in [false, true] {
                let r = set_here(&range, length, end, tick);
                assert!(r.start < r.end && r.end <= length, "{tick}: {r:?}");
            }
        }
    }

    /// The sizes offered are even multiples of the frame, the first one four times.
    #[test]
    fn the_sizes_are_even() {
        assert!(SIZES.iter().all(|s| s % 2 == 0));
        assert_eq!(SIZES[FIRST_SIZE], 4);
        assert_eq!(Export::size_label(4, Sides::One), "960 × 640 (4×)");
        assert_eq!(Export::size_label(4, Sides::Both), "960 × 640 (4×)");
        assert_eq!(Export::size_label(4, Sides::SideBySide), "1920 × 640 (4×)");
        assert_eq!(SIDES, [Sides::One, Sides::Both, Sides::SideBySide], "the Sides switch's order");
    }
}
