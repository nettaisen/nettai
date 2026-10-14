//! A replay's export as video (docs/design/app-ui.md §6, "Export"; docs/app.md §16): the dialog's choices, and
//! the export itself, which is nettai-video's `export_mp4` on a thread of its own: it plays the replay through
//! a viewer of its own there and pipes the pictures and the sound to `ffmpeg`, which writes the MP4.
//!
//! The thread tells how far it is and is told to stop through what the two share ([`Shared`]); the dialog
//! reads it each frame. The file takes its name only once it is whole (the library's `mp4::Encoder`): stopped
//! or failed, nothing of it is left. What went wrong is said in the dialog's line, in the player's words
//! (`UserError::of_export`); the app saves an MP4 and nothing else.

use super::{clock, name, played};
use crate::errors::UserError;
use crate::nav::Key;
use crate::replays::Summary;
use crate::shell::Shell;
use nettai_frontend::game::{Loaded, Sound, TextMode};
use nettai_frontend::player::time_of;
use nettai_ui::{ExportState, ReplaysView};
use nettai_video::{Options, Part, Progress, mp4};
use slint::ComponentHandle;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// The sizes offered, in multiples of the frame's 240 by 160, and the one chosen at first. They are even: an
/// MP4 keeps its color at half the resolution, and at an odd multiple every pixel's edge fringes
/// (docs/frontend.md §8, "Video").
const SIZES: [u32; 3] = [2, 4, 6];
const FIRST_SIZE: usize = 1;

/// Where the folder last exported to is remembered (the settings' `remembered`).
const FOLDER: &str = "replays.export-folder";

/// The most of a path the dialog shows (its middle is left out past it).
const PATH_SHOWN: usize = 42;

/// What the export's thread and the dialog share.
#[derive(Default)]
struct Shared {
    progress: Option<Progress>,
    done: Option<Result<(), nettai_video::Error>>,
}

/// An export under way. Dropped, it is stopped: the thread is told, and waited for (it removes what it wrote).
struct Running {
    stop: Arc<AtomicBool>,
    shared: Arc<Mutex<Shared>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The export's dialog, open.
pub struct Export {
    /// The replay's file, and what it says of itself.
    replay: PathBuf,
    summary: Summary,
    size: usize,
    side: u8,
    /// The part: 0 the whole match, else that round.
    part: u32,
    to: PathBuf,
    /// The player chose the place: it isn't named again as the part changes.
    placed: bool,
    running: Option<Running>,
    state: ExportState,
    error: Option<UserError>,
}

/// The system's folder for videos.
fn videos() -> PathBuf {
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

/// The `ffmpeg` to run: `$NETTAI_FFMPEG`, else the one on the path or where a package manager puts it (an app
/// started from the desktop has a short path), else the name alone.
fn ffmpeg() -> PathBuf {
    if let Some(named) = std::env::var_os("NETTAI_FFMPEG") {
        return named.into();
    }
    let name = if cfg!(target_os = "windows") { "ffmpeg.exe" } else { "ffmpeg" };
    let path = std::env::var_os("PATH").unwrap_or_default();
    let usual = ["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"].map(PathBuf::from);
    std::env::split_paths(&path).chain(usual.into_iter().filter(|_| cfg!(unix))).map(|dir| dir.join(name)).find(|p| p.is_file()).unwrap_or_else(|| name.into())
}

/// `text` as a file's name may hold it.
fn file_safe(text: &str) -> String {
    let kept: String = text.chars().map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { ' ' } else { c }).collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A path as the dialog shows it: from the player's home where it is in it, its middle left out where it is
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

impl Export {
    /// Open the dialog for the replay at `replay`, shown from `side`.
    pub fn open(shell: &mut Shell, replay: &Path, summary: &Summary, side: u8) -> Export {
        let mut export = Export {
            replay: replay.to_path_buf(),
            summary: summary.clone(),
            size: FIRST_SIZE,
            side: side.min(1),
            part: 0,
            to: PathBuf::new(),
            placed: false,
            running: None,
            state: ExportState::Idle,
            error: None,
        };
        export.name(shell);
        export.show(shell);
        shell.ui().global::<ReplaysView>().set_exporting(true);
        shell.nav.open("export.", Key::new("export.go", 0));
        export
    }

    /// Close it: an export under way is stopped.
    pub fn close(self, shell: &mut Shell) {
        shell.nav.close("export.");
        shell.ui().global::<ReplaysView>().set_exporting(false);
    }

    /// Whether an export is under way.
    pub fn running(&self) -> bool {
        self.running.is_some()
    }

    /// The parts there are: the whole match, and each round where there is more than one.
    fn parts(&self) -> u32 {
        match self.summary.rounds.len() as u32 {
            1 => 0,
            rounds => rounds,
        }
    }

    /// A part's length, in ticks.
    fn ticks(&self, part: u32) -> u32 {
        let rounds = &self.summary.rounds;
        match part {
            0 => self.summary.ticks,
            n => rounds.get(n as usize).copied().unwrap_or(self.summary.ticks) - rounds[n as usize - 1],
        }
    }

    fn part_label(&self, view: &ReplaysView, part: u32) -> String {
        let length = clock(time_of(self.ticks(part) as u64), true);
        match part {
            0 => view.invoke_whole(length.into()).into(),
            n => view.invoke_round_part(n as i32, length.into()).into(),
        }
    }

    fn size_label(size: u32) -> String {
        format!("{} × {} ({size}×)", 240 * size, 160 * size)
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
        if self.part > 0 {
            stem.push_str(&format!(" round {}", self.part));
        }
        self.to = (1..).map(|n| folder.join(if n == 1 { format!("{stem}.mp4") } else { format!("{stem} ({n}).mp4") })).find(|p| !p.exists()).expect("some number is free");
    }

    /// Set the view from the dialog's state.
    pub fn show(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        view.set_export_size(Export::size_label(SIZES[self.size]).into());
        view.set_export_view(self.side as i32);
        view.set_export_part(self.part_label(&view, self.part).into());
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

    fn set_part(&mut self, shell: &mut Shell, part: u32) {
        self.part = part.min(self.parts());
        self.name(shell);
        self.show(shell);
    }

    /// Enter or a click on one of its controls (its name after `export.`). True: the dialog is to close.
    pub fn activate(&mut self, shell: &mut Shell, what: &str) -> bool {
        match what {
            "cancel" => return self.back(shell),
            "go" => self.start(shell),
            "size" if self.open_to_change() => shell.pick("export.size", 0, SIZES.iter().map(|&s| Export::size_label(s)).collect(), self.size),
            "part" if self.open_to_change() => {
                let ui = shell.ui();
                let view = ui.global::<ReplaysView>();
                let options = (0..=self.parts()).map(|part| self.part_label(&view, part)).collect();
                shell.pick("export.part", 0, options, self.part as usize);
            }
            "view" if self.open_to_change() => {
                self.side ^= 1;
                self.show(shell);
            }
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

    /// Left or Right on one of its controls.
    pub fn step(&mut self, shell: &mut Shell, what: &str, by: i32) {
        if !self.open_to_change() {
            return;
        }
        match what {
            "size" => self.size = (self.size as i32 + by).clamp(0, SIZES.len() as i32 - 1) as usize,
            "view" => self.side = (by > 0) as u8,
            "part" => return self.set_part(shell, (self.part as i32 + by).clamp(0, self.parts() as i32) as u32),
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
            "view" => self.side = choice.clamp(0, 1) as u8,
            "part" => return self.set_part(shell, choice.max(0) as u32),
            _ => {}
        }
        self.show(shell);
    }

    /// Esc, or CANCEL: an export under way is stopped (the dialog stays, to export again); else true, and the
    /// dialog is to close.
    pub fn back(&mut self, _shell: &mut Shell) -> bool {
        match &self.running {
            Some(running) => {
                running.stop.store(true, Ordering::Relaxed);
                false
            }
            None => true,
        }
    }

    /// Start the export, on a thread of its own.
    fn start(&mut self, shell: &mut Shell) {
        if !self.open_to_change() {
            return;
        }
        let Some((_, ready)) = shell.ready() else { return };
        let doing = format!("exporting {}", self.replay.display());
        let replay = match crate::replays::read(&self.replay) {
            Ok(replay) => replay,
            Err(why) => return self.fail(shell, UserError::of_replay_read(&doing, &why)),
        };
        // (A file of that name made since the dialog opened isn't written over, unless the player chose it.)
        self.name(shell);
        if let Some(folder) = self.to.parent()
            && let Err(why) = std::fs::create_dir_all(folder)
        {
            crate::errors::log(&doing, &format_args!("{}: {why}", folder.display()));
            return self.fail(shell, UserError::CantWriteVideo);
        }
        // What the thread plays from: the game as the window shows it (the language, the text), shared.
        let text = if shell.settings.sharp_text { TextMode::Font } else { TextMode::Original };
        let game = Loaded {
            found: ready.loaded.found.clone(),
            game: ready.loaded.game.clone(),
            graphics: ready.graphics(shell.lang),
            text,
            font: ready.loaded.font.clone().filter(|_| text == TextMode::Font),
            // (With the game's own music, as the replay plays in the viewer: the pick is Practice's alone.)
            sound: crate::music::sound(&shell.games, &ready, &crate::music::Music::GameOwn).map(|(banks, songs)| Sound { banks, songs, report: Default::default() }),
        };
        let options = Options { scale: SIZES[self.size], view: Some(self.side), part: if self.part == 0 { Part::Whole } else { Part::Round(self.part) } };
        let encoding = mp4::Settings { ffmpeg: ffmpeg(), ..mp4::Settings::default() };
        let (stop, shared, to) = (Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(Shared::default())), self.to.clone());
        let (stopped, told) = (stop.clone(), shared.clone());
        let thread = std::thread::Builder::new().name("export".into()).spawn(move || {
            let result = nettai_video::export_mp4(&game, &replay, &options, &encoding, &to, &mut |progress| {
                told.lock().unwrap_or_else(|e| e.into_inner()).progress = Some(progress);
                if stopped.load(Ordering::Relaxed) { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
            });
            told.lock().unwrap_or_else(|e| e.into_inner()).done = Some(result);
        });
        match thread {
            Ok(thread) => {
                self.running = Some(Running { stop, shared, thread: Some(thread) });
                self.state = ExportState::Running;
                let ui = shell.ui();
                let view = ui.global::<ReplaysView>();
                view.set_export_progress(0.0);
                view.set_export_round(self.part.max(1) as i32);
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
            let mut shared = running.shared.lock().unwrap_or_else(|e| e.into_inner());
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut dialog = rfd::FileDialog::new().add_filter(kind, &["mp4"]);
        if let Some(folder) = to.parent().filter(|f| f.is_dir()) {
            dialog = dialog.set_directory(folder);
        }
        if let Some(name) = to.file_name() {
            dialog = dialog.set_file_name(name.to_string_lossy());
        }
        dialog.save_file()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (kind, to);
        None
    }
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

    /// The sizes offered are even multiples of the frame, the first one four times.
    #[test]
    fn the_sizes_are_even() {
        assert!(SIZES.iter().all(|s| s % 2 == 0));
        assert_eq!(SIZES[FIRST_SIZE], 4);
        assert_eq!(Export::size_label(4), "960 × 640 (4×)");
    }
}
