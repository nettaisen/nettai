//! The showreel (`NETTAI_REEL=<out.mp4>`, with the pass's folder `NETTAI_TOUR=<folder>`; docs/app.md §13): the app
//! walks a scripted demo of itself with everything moving and sounding, and every frame of the window and the sound
//! it mixes go to an MP4 (H.264 and AAC, through `ffmpeg`).
//!
//! **Time is the reel's.** The app runs a frame at a time on a clock of its own: each frame captured moves it on by
//! exactly 1/60 s, whatever the capture costs, so the video plays smoothly at 60 frames a second. Two clocks follow
//! it: Slint's (its animations and timers, among them the walk's steps), through the window's backend wrapped here
//! ([`Platform`]: the winit backend, its clock this one's), and the app's own ([`crate::clock`]: a frame's time, a
//! screen's change, the cards' time up). The frames are the reel's alone: the window's own draws run none
//! (`main.rs`), nor does the timer that runs them when the window isn't drawn.
//!
//! **The sound is what the device would play** (`Sound::frame` hands it here at the device's volume: the menus'
//! sounds and music, a battle's, a replay's), a frame's worth with each frame, in step with the picture: it is
//! written a little over a battle tick late (`LEAD`), so a frame without a tick doesn't leave a gap.
//!
//! **Off the air.** What the walk only waits for (the games loading, the replays being made and played out, the
//! other player's link) runs with the clock going but nothing written: the video cuts from before it to after.
//!
//! The walk is the pass's, held still as it is (no settings read or written, its own builds and replays, the
//! demo battles' seeds fixed), but with motion on and the battles playing: the main menu and its demo battle
//! through a change of game to each and back; Practice (the editor's PRACTICE, the practice menu open as the battle
//! starts, the arena and the music stepped there, the dummy set to move around, a fight in which the player's
//! side presses by itself);
//! the builds (Play's; the editor's Folder, NaviCust and Patch cards tabs in EXE6, the Auto battle tab in EXE5);
//! Play (an invite, a second player joining, the lobby, a few seconds of their match); Replays (the list, a
//! replay watched, a seek and a speed change); About, from Settings. `reel.txt` in the pass's folder says what
//! was written.

use super::{SETTLE, Step, Tour};
use crate::input::Action;
use crate::screens::ScreenId;
use crate::shell::App;
use nettai_battle::battle::{battle_flags, mode};
use nettai_battle::input::keys;
use nettai_frontend::driver::bot_buttons;
use nettai_ui::{AppWindow, BattleKind, PracticeOption, ReplaysView};
use nettai_video::ffmpeg::nut;
use slint::{ComponentHandle, PhysicalSize};
use std::cell::{Cell, RefCell};
use std::io::{BufWriter, Read, Write as _};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread::JoinHandle;
use std::time::Duration;
use nettai_platform::time::Instant;

/// The frames a second: each frame captured is 1/60 s of the reel's clock.
const RATE: u64 = 60;

/// The video's size: the window's 1280 by 800 units, at the display's pixels (2560 by 1600 at a scale of two),
/// scaled to this.
const OUT: (u32, u32) = (1920, 1200);

/// The sound's rate (the battle's).
const SAMPLE_RATE: u64 = nettai_audio::SAMPLE_RATE as u64;

/// How far the sound is written behind the picture, in samples: a battle tick's (549) and a little, so that a
/// frame in which no tick ran (a 60 Hz frame is a little shorter than the GBA's) still has its sound.
const LEAD: usize = 640;

/// The reel runs: its clock is the app's and Slint's.
static ON: AtomicBool = AtomicBool::new(false);
/// The frames the reel's clock has run, and when it started (by the system's clock).
static FRAMES: AtomicU64 = AtomicU64::new(0);
static ORIGIN: OnceLock<Instant> = OnceLock::new();

thread_local! {
    /// A frame is being captured: the window's rendering runs the app's frame for it.
    static CAPTURING: Cell<bool> = const { Cell::new(false) };
    /// The sound the app mixed in the frame being captured.
    static HEARD: RefCell<Vec<[f32; 2]>> = const { RefCell::new(Vec::new()) };
    /// The player's side presses by itself in a practice battle ([`pilot`]).
    static PILOT: Cell<bool> = const { Cell::new(false) };
    static REEL: RefCell<Option<Reel>> = const { RefCell::new(None) };
}

/// The reel's clock since it started.
fn elapsed() -> Duration {
    Duration::from_nanos(FRAMES.load(Ordering::Relaxed) * 1_000_000_000 / RATE)
}

/// The app's now, while the reel runs ([`crate::clock::now`]).
pub fn now() -> Option<Instant> {
    ON.load(Ordering::Relaxed)
        .then(|| *ORIGIN.get_or_init(Instant::now) + elapsed())
}

/// Whether the window's rendering runs the app's frame now (`main.rs`): always, but in the reel, where only the
/// frames it captures do.
pub fn frame_due() -> bool {
    !ON.load(Ordering::Relaxed) || CAPTURING.with(Cell::get)
}

/// The sound the app sends the device this frame (`Sound::frame`), at its volume `level`.
pub fn hear(samples: &[[f32; 2]], level: f32) {
    if ON.load(Ordering::Relaxed) && CAPTURING.with(Cell::get) {
        HEARD.with(|h| {
            h.borrow_mut()
                .extend(samples.iter().map(|s| [s[0] * level, s[1] * level]))
        });
    }
}

/// The window's backend, the winit one the app uses (its renderer FemtoVG, its window not taking the keyboard, as
/// the pass's), with its clock the reel's.
struct Platform(i_slint_backend_winit::Backend);

impl i_slint_core::platform::Platform for Platform {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn i_slint_core::window::WindowAdapter>, slint::PlatformError> {
        self.0.create_window_adapter()
    }

    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        self.0.run_event_loop()
    }

    fn process_events(
        &self,
        timeout: Option<Duration>,
        token: i_slint_core::InternalToken,
    ) -> Result<core::ops::ControlFlow<()>, slint::PlatformError> {
        self.0.process_events(timeout, token)
    }

    fn bind_context(
        &self,
        context: i_slint_core::SlintContextWeak,
        token: i_slint_core::InternalToken,
    ) {
        self.0.bind_context(context, token)
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn i_slint_core::platform::EventLoopProxy>> {
        self.0.new_event_loop_proxy()
    }

    fn duration_since_start(&self) -> Duration {
        elapsed()
    }

    fn click_interval(&self) -> Duration {
        self.0.click_interval()
    }

    fn cursor_flash_cycle(&self) -> Duration {
        self.0.cursor_flash_cycle()
    }

    fn set_clipboard_text(&self, text: &str, clipboard: i_slint_core::platform::Clipboard) {
        self.0.set_clipboard_text(text, clipboard)
    }

    fn clipboard_text(&self, clipboard: i_slint_core::platform::Clipboard) -> Option<String> {
        self.0.clipboard_text(clipboard)
    }

    fn debug_log(&self, arguments: core::fmt::Arguments) {
        self.0.debug_log(arguments)
    }

    fn open_url(&self, url: &str) -> Result<(), slint::PlatformError> {
        self.0.open_url(url)
    }
}

/// Set the window's backend, in place of `main.rs`'s selection, and start the reel's clock.
pub fn platform() -> Result<(), slint::PlatformError> {
    let backend = i_slint_backend_winit::Backend::builder()
        .with_renderer_name("femtovg")
        .with_window_attributes_hook(|a| a.with_active(false))
        .build()?;
    slint::platform::set_platform(Box::new(Platform(backend)))
        .map_err(slint::PlatformError::SetPlatformError)?;
    ORIGIN.get_or_init(Instant::now);
    ON.store(true, Ordering::Relaxed);
    Ok(())
}

/// A frame for the encoder: the window's pixels (red, green, blue and alpha) and the sound that goes with it.
struct Frame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    sound: Vec<[f32; 2]>,
}

/// What the reel keeps as it goes.
struct Reel {
    ui: slint::Weak<AppWindow>,
    out: PathBuf,
    dir: PathBuf,
    /// Frames are written.
    on_air: bool,
    /// The sound heard and not yet written.
    owed: Vec<[f32; 2]>,
    /// Frames and samples written; samples written as silence for want of any heard; the most owed at once.
    frames: u64,
    samples: u64,
    short: u64,
    most_owed: usize,
    /// The size of the frames written (the first's), and frames passed by for being of another.
    size: Option<(u32, u32)>,
    odd: u64,
    /// The frames run off the air.
    off: u64,
    to: Option<SyncSender<Frame>>,
    encoder: Option<JoinHandle<Result<String, String>>>,
    done: bool,
}

impl Reel {
    /// The frame captured (`shot`, if there was one) and the sound heard in it.
    fn take(
        &mut self,
        shot: Option<slint::SharedPixelBuffer<slint::Rgba8Pixel>>,
        heard: Vec<[f32; 2]>,
    ) {
        if !self.on_air {
            self.off += 1;
            return;
        }
        let Some(shot) = shot else { return };
        let size = (shot.width(), shot.height());
        if *self.size.get_or_insert(size) != size {
            self.odd += 1;
            return;
        }
        if self.to.is_none() {
            let (to, from) = sync_channel(4);
            self.to = Some(to);
            let out = self.out.clone();
            self.encoder = Some(std::thread::spawn(move || encode(&out, size, from)));
        }
        // The sound due by the end of this frame, from what was heard (silence where there is too little).
        self.owed.extend(heard);
        self.most_owed = self.most_owed.max(self.owed.len());
        let due = ((self.frames + 1) * SAMPLE_RATE / RATE - self.samples) as usize;
        let mut sound: Vec<[f32; 2]> = self.owed.drain(..due.min(self.owed.len())).collect();
        self.short += (due - sound.len()) as u64;
        sound.resize(due, [0.0; 2]);
        self.samples += due as u64;
        self.frames += 1;
        let frame = Frame {
            width: size.0,
            height: size.1,
            rgba: shot.as_bytes().to_vec(),
            sound,
        };
        if self.to.as_ref().is_some_and(|to| to.send(frame).is_err()) {
            // (The encoder stopped: it says why at the end.)
            self.to = None;
        }
    }

    /// On the air, or off it. (The sound starts `LEAD` behind the picture each time it comes on.)
    fn air(&mut self, on: bool) {
        if on && !self.on_air {
            self.owed = vec![[0.0; 2]; LEAD];
        }
        self.on_air = on;
    }
}

/// ffmpeg writing `out` from the frames that come, as a NUT stream on its standard input: H.264 at `OUT`'s size and
/// AAC. What it said, or why it failed.
fn encode(
    out: &std::path::Path,
    (width, height): (u32, u32),
    frames: Receiver<Frame>,
) -> Result<String, String> {
    let filter = format!(
        "scale={}:{}:flags=lanczos:out_color_matrix=bt709:out_range=tv,format=yuv420p",
        OUT.0, OUT.1
    );
    let mut child = Command::new(nettai_video::ffmpeg::find_ffmpeg())
        .args([
            "-hide_banner",
            "-nostdin",
            "-loglevel",
            "error",
            "-f",
            "nut",
            "-i",
            "pipe:0",
            "-vf",
            &filter,
        ])
        .args([
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "21",
            "-colorspace",
            "bt709",
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
            "-color_range",
            "tv",
        ])
        .args([
            "-c:a",
            "aac",
            "-b:a",
            "160k",
            "-ar",
            "48000",
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
            "-y",
        ])
        .arg(out)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg can't be started: {e}"))?;
    let stdin = child.stdin.take().expect("piped");
    let mut said = child.stderr.take().expect("piped");
    let log = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = said.read_to_string(&mut text);
        text
    });
    let video = nettai_video::Video {
        width,
        height,
        frame_time: (1, RATE as u32),
    };
    let written = (|| -> std::io::Result<()> {
        let mut nut = nut::Writer::new(
            BufWriter::with_capacity(1 << 22, stdin),
            video,
            Some(nettai_video::Audio {
                sample_rate: SAMPLE_RATE as u32,
                tracks: 1,
            }),
        )?;
        let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
        for frame in frames {
            if (frame.width, frame.height) != (width, height) {
                continue;
            }
            rgb.clear();
            rgb.extend(frame.rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]));
            nut.frame(&rgb, &[&frame.sound[..]])?;
        }
        nut.finish()?.flush()
    })();
    let status = child.wait().map_err(|e| e.to_string())?;
    let log = log.join().unwrap_or_default();
    match (written, status.success()) {
        (Ok(()), true) => Ok(log),
        (Err(e), _) => Err(format!("writing to ffmpeg: {e}\n{log}")),
        (_, false) => Err(format!("ffmpeg ended with {status}\n{log}")),
    }
}

/// A frame of the reel: its clock a frame on, Slint's timers and animations to it (the walk's steps among them),
/// the player's side's buttons, the window captured (which runs the app's frame), and the next frame asked for.
fn frame() {
    let Some(ui) = REEL.with(|r| {
        r.borrow()
            .as_ref()
            .filter(|r| !r.done)
            .and_then(|r| r.ui.upgrade())
    }) else {
        return;
    };
    FRAMES.fetch_add(1, Ordering::Relaxed);
    slint::platform::update_timers_and_animations();
    if REEL.with(|r| r.borrow().as_ref().is_none_or(|r| r.done)) {
        return;
    }
    crate::with_app(pilot);
    CAPTURING.with(|c| c.set(true));
    let shot = ui.window().take_snapshot().ok();
    CAPTURING.with(|c| c.set(false));
    let heard = HEARD.with(|h| std::mem::take(&mut *h.borrow_mut()));
    REEL.with(|r| {
        if let Some(reel) = r.borrow_mut().as_mut() {
            reel.take(shot, heard);
        }
    });
    if slint::invoke_from_event_loop(frame).is_err() {
        eprintln!("reel: the window's loop ended");
    }
}

/// The player's side in a practice battle, while [`PILOT`] says: its custom screen picked through as the bots
/// pick, and in the fight a chip used, a move up or down, a charged shot and another chip each two and a half
/// seconds, the custom screen opened once the gauge is full.
fn pilot(app: &mut App) {
    if !PILOT.with(Cell::get) {
        return;
    }
    let held = match &app.shell.battle {
        Some(b) if b.kind == BattleKind::Practice && b.playing() && !b.menu_open() => {
            let battle = b.stage.player.battle();
            let tick = b.stage.player.ticks() as u32;
            if battle.round.mode == mode::CUSTOM {
                bot_buttons(battle, b.viewer as usize, tick)
            } else {
                let full = battle.round.flags & battle_flags::GAUGE_FULL != 0;
                match tick % 150 {
                    0..6 | 120..126 => keys::A,
                    30..36 => [keys::UP, keys::DOWN][(tick / 150 % 2) as usize],
                    50..110 => keys::B,
                    135..141 if full => keys::L,
                    _ => 0,
                }
            }
        }
        _ => 0,
    };
    app.shell.keys.held = held;
}

/// Run the reel: the walk's steps, captured, to `out`.
pub fn start(ui: &AppWindow, app: &Rc<RefCell<App>>, dir: PathBuf, out: PathBuf) {
    let tour = super::begin(ui, app, dir.clone());
    {
        let mut app = app.borrow_mut();
        app.shell.probe_motion = true;
        app.shell.reel = true;
        app.shell.apply_motion();
    }
    REEL.with(|r| {
        *r.borrow_mut() = Some(Reel {
            ui: ui.as_weak(),
            out,
            dir,
            on_air: false,
            owed: Vec::new(),
            frames: 0,
            samples: 0,
            short: 0,
            most_owed: 0,
            size: None,
            odd: 0,
            off: 0,
            to: None,
            encoder: None,
            done: false,
        })
    });
    let mut steps: Vec<(u64, Step)> = Vec::new();
    walk(&mut steps);
    steps.push((0, Box::new(|_, _, _| end())));
    super::when_ready(ui.as_weak(), app.clone(), tour, Rc::new(steps), 0);
    let _ = slint::invoke_from_event_loop(frame);
}

/// The reel is over: the encoder finished, `reel.txt` written, and the app quits.
fn end() {
    let Some(mut reel) = REEL.with(|r| r.borrow_mut().take()) else {
        return;
    };
    reel.done = true;
    PILOT.with(|p| p.set(false));
    drop(reel.to.take());
    let encoded = match reel.encoder.take() {
        Some(encoder) => encoder
            .join()
            .unwrap_or_else(|_| Err("the encoder's thread panicked".into())),
        None => Err("no frame was written".into()),
    };
    let seconds = reel.frames as f64 / RATE as f64;
    let mut said = format!(
        "{}: {} frames ({seconds:.2} s at {RATE} a second) of {:?} pixels, scaled to {}x{}; {} samples of sound at {SAMPLE_RATE} Hz, {} of them silence for want of any heard (its lead {LEAD}), at most {} owed at once; {} frames run off the air, {} passed by for another size.\n",
        reel.out.display(),
        reel.frames,
        reel.size.unwrap_or_default(),
        OUT.0,
        OUT.1,
        reel.samples,
        reel.short,
        reel.most_owed,
        reel.off,
        reel.odd,
    );
    match &encoded {
        Ok(log) if log.trim().is_empty() => said += "ffmpeg: done.\n",
        Ok(log) => said += &format!("ffmpeg: done, and said:\n{log}\n"),
        Err(why) => said += &format!("FAILED: {why}\n"),
    }
    eprint!("reel: {said}");
    let _ = nettai_platform::fs::write(reel.dir.join("reel.txt"), &said);
    super::FAILED.with(|f| f.set(encoded.is_err()));
    super::WALKING.with(|w| *w.borrow_mut() = None);
    let _ = slint::quit_event_loop();
}

// ---- The walk ---------------------------------------------------------------------------------------------------

/// On the air, or off it.
fn air(on: bool) -> Step {
    Box::new(move |_, _, _| {
        REEL.with(|r| r.borrow_mut().as_mut().map(|r| r.air(on)))
            .unwrap_or_default()
    })
}

fn act(action: Action) -> Step {
    Box::new(move |_, app, _| app.borrow_mut().act(action))
}

fn focus(id: &'static str, index: i32) -> Step {
    Box::new(move |_, app, _| app.borrow_mut().shell.nav.focus(id, index))
}

fn go(to: ScreenId) -> Step {
    Box::new(move |_, app, _| {
        let mut app = app.borrow_mut();
        app.go_now(to);
        app.shell.nav.show_cursor(true, false);
    })
}

fn nothing() -> Step {
    Box::new(|_, _, _| {})
}

/// The focus on the practice menu's row of `option`, where the menu shows it.
fn row(option: PracticeOption) -> Step {
    Box::new(move |_, app, _| {
        let mut app = app.borrow_mut();
        match crate::screens::practice::row_of(&app.shell, option) {
            Some(at) => app.shell.nav.focus("menu.rows.option", at),
            None => eprintln!(
                "reel: the practice menu shows no row of {option:?} (the focus on {:?})",
                app.shell.nav.focused()
            ),
        }
    })
}

/// The walk goes on once `come` has (or after a minute): the next step's time counts from then.
fn reach(add: &mut dyn FnMut(u64, Step), come: impl Fn(&App) -> bool + 'static) {
    let come = Rc::new(come);
    add(
        0,
        Box::new(move |_, _, tour: &Rc<Tour>| {
            let come = come.clone();
            tour.wait_until(move |app| come(app));
        }),
    );
    add(0, nothing());
}

/// A change of game to `id`, by the shoulder that is nearer it (R's: the next): its ground sweeps in from that side.
fn step_to(id: &'static str) -> Step {
    Box::new(move |_, app, _| {
        let mut app = app.borrow_mut();
        let games = app.shell.games.playable();
        let (Some(at), Some(to)) = (
            app.shell
                .game()
                .and_then(|g| games.iter().position(|x| x == g)),
            games.iter().position(|x| x == id),
        ) else {
            return;
        };
        if at != to {
            app.act(if (at + 1) % games.len() == to {
                Action::Next
            } else {
                Action::Previous
            });
        }
    })
}

/// The selected game is ready, its runtime too.
fn game_ready(app: &App) -> bool {
    app.shell
        .ready()
        .is_some_and(|(_, ready)| ready.has_language("en"))
}

/// The builds of each game the reel's Builds and Practice list: a new build of each of its versions, kept as a
/// player's are (in the pass's own builds folder).
fn make_builds(app: &App) {
    for game in app.shell.games.playable() {
        let Some(ready) = app.shell.games.ready(&game) else {
            continue;
        };
        let content = ready.content();
        let _ = nettai_platform::fs::remove_dir_all(crate::builds::store::folder(&game));
        for b in crate::screens::practice::new_builds(content, &game).unwrap_or_default() {
            let build = crate::builds::store::Build::new(content, &b.name, b.side);
            if let Err(why) = crate::builds::store::write(
                &crate::builds::store::new_path(&game, &b.name),
                content,
                &game,
                &build,
            ) {
                crate::errors::log("keeping the reel's build", &why);
            }
        }
    }
}

/// The reel's steps.
fn walk(steps: &mut Vec<(u64, Step)>) {
    let mut add = |wait: u64, step: Step| steps.push((wait, step));
    let add = &mut add;

    // Off the air: the window at the desktop's size, the player's name, the main menu in English; each game made
    // ready (the menu's demo battle and the title music of each are there at once when it is changed to); the
    // reel's builds, and the pass's replays made and played out by the Replays screen; BN6 again.
    add(
        0,
        Box::new(|ui, app, _| {
            let os = slint::winit_030::WinitWindowAccessor::with_winit_window(ui.window(), |w| {
                w.scale_factor()
            })
            .unwrap_or(1.0);
            ui.window().set_size(PhysicalSize::new(
                (1280.0 * os).round() as u32,
                (800.0 * os).round() as u32,
            ));
            let mut app = app.borrow_mut();
            app.shell.set_name("Lan");
            app.shell.set_language("en");
            crate::tour::practice::forget(&mut app);
            app.go_now(ScreenId::Menu);
            app.shell.nav.focus("play", 0);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    for game in ["exe4", "exe5", "exe6"] {
        add(
            SETTLE,
            Box::new(move |_, app, _| app.borrow_mut().shell.select_game(game)),
        );
        reach(add, game_ready);
    }
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            let app = app.borrow();
            make_builds(&app);
            if let Err(why) = super::replays::make(&app.shell, &tour.dir.join("replays")) {
                eprintln!("reel: the replays couldn't be made: {why}");
            }
        }),
    );
    add(SETTLE, go(ScreenId::Replays));
    reach(add, |app| {
        let ui = app.shell.ui();
        let view = ui.global::<ReplaysView>();
        view.get_listed()
            && view.get_still()
            && slint::Model::iter(&view.get_rows())
                .all(|row| !row.score.is_empty() || !row.reason.is_empty())
    });
    add(SETTLE * 2, go(ScreenId::Menu));
    add(SETTLE, focus("play", 0));
    // (The menu's demo battle under way.)
    add(2500, air(true));

    // 1. The main menu: the demo battle beside it, the title music; the cursor down a step and back; a change of
    // game to each (the ground sweeping in, the title music and the demo battle the game's), and back to BN6.
    add(1800, act(Action::Down));
    add(600, act(Action::Up));
    add(1200, step_to("exe5"));
    add(3000, step_to("exe4"));
    add(3000, step_to("exe6"));

    // 2. Practice: the editor's PRACTICE (Play's EDIT, then the tab bar's), the cards, the battle with the practice menu open over it; the arena stepped
    // (the set starting over on it behind the menu), the music stepped to a battle theme, which plays; the dummy
    // set to move around; the fight, the player's side pressing by itself.
    add(3000, go(ScreenId::Play));
    add(1500, focus("edit", 0));
    add(500, act(Action::Confirm));
    add(1500, focus("practice", 0));
    add(500, act(Action::Confirm));
    reach(add, |app| {
        app.shell
            .battle
            .as_ref()
            .is_some_and(|b| b.kind == BattleKind::Practice && b.menu_open())
    });
    add(1200, row(PracticeOption::Arena));
    // (Arena and Music are chosen from their lists; Action is taken, stepped and let go.)
    add(500, act(Action::Confirm));
    add(400, act(Action::Right));
    add(400, act(Action::Confirm));
    add(1800, row(PracticeOption::Music));
    add(500, act(Action::Confirm));
    add(400, act(Action::Down));
    add(400, act(Action::Confirm));
    add(2500, row(PracticeOption::Action));
    // (Stands still, Moves around.)
    add(300, act(Action::Confirm));
    add(400, act(Action::Right));
    add(300, act(Action::Confirm));
    add(
        900,
        Box::new(|_, app, _| {
            app.borrow_mut().shell.close_menu();
            PILOT.with(|p| p.set(true));
        }),
    );
    add(
        10500,
        Box::new(|_, app, _| {
            PILOT.with(|p| p.set(false));
            let mut app = app.borrow_mut();
            app.shell.keys.held = 0;
            crate::screens::practice::end(&mut app.shell);
            crate::tour::practice::forget(&mut app);
            app.go_now(ScreenId::Play);
            app.shell.nav.show_cursor(true, false);
        }),
    );

    // 3. The builds: Play's, a build's overview; the editor's Folder tab, a chip put in from the library; the
    // NaviCust, a program taken and put down; the patch cards, three put in. Then EXE5's builds, and the Auto
    // battle tab, three places filled.
    add(1200, focus("builds", 1));
    add(700, focus("builds", 0));
    add(
        500,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.nav.focus("builds", 0);
            app.shell.nav.focus("edit", 0);
            app.act(Action::Confirm);
        }),
    );
    add(
        900,
        crate::tour::build_tabs::to_tab(nettai_ui::EditorTab::Folder),
    );
    add(800, focus("slot", 13));
    add(500, act(Action::Confirm));
    add(
        600,
        Box::new(|_, app, _| {
            app.borrow_mut().with_screen(ScreenId::Builds, |s, shell| {
                s.choose(shell, "lib.sort", 0, 3)
            })
        }),
    );
    add(
        300,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.nav.focus("lib.direction", 0);
            app.act(Action::Confirm);
        }),
    );
    add(500, focus("lib.rows", 1));
    add(300, focus("lib.rows", 2));
    add(400, act(Action::Confirm));
    add(1200, crate::tour::build_tabs::out_of_library());
    add(
        200,
        crate::tour::build_tabs::to_tab(nettai_ui::EditorTab::Navicust),
    );
    add(
        1000,
        crate::tour::build_tabs::take(0, crate::tour::build_tabs::program),
    );
    add(500, act(Action::Right));
    add(300, act(Action::Down));
    add(
        400,
        crate::tour::build_tabs::put(crate::tour::build_tabs::Spot::Free),
    );
    add(
        900,
        crate::tour::build_tabs::take(0, crate::tour::build_tabs::program),
    );
    add(
        600,
        crate::tour::build_tabs::put(crate::tour::build_tabs::Spot::Free),
    );
    add(
        1100,
        crate::tour::build_tabs::to_tab(nettai_ui::EditorTab::PatchCards),
    );
    add(900, crate::tour::build_tabs::library());
    for row in 0..3 {
        add(450, focus("lib.rows", row));
        add(250, act(Action::Confirm));
    }
    add(1000, crate::tour::build_tabs::out_of_library());
    add(
        300,
        Box::new(|_, app, _| crate::tour::builds::leave(&mut app.borrow_mut())),
    );
    add(700, step_to("exe5"));
    add(
        1500,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.nav.focus("builds", 0);
            app.shell.nav.focus("edit", 0);
            app.act(Action::Confirm);
        }),
    );
    add(
        800,
        crate::tour::build_tabs::to_tab(nettai_ui::EditorTab::AutoBattle),
    );
    add(700, focus("places.place", 0));
    for row in 0..3 {
        add(350, crate::tour::build_tabs::library());
        add(400, focus("lib.rows", row * 4));
        add(250, act(Action::Confirm));
    }
    add(1300, crate::tour::build_tabs::out_of_library());
    add(0, air(false));
    add(
        SETTLE,
        Box::new(|_, app, _| crate::tour::builds::leave(&mut app.borrow_mut())),
    );
    add(
        SETTLE,
        Box::new(|_, app, _| app.borrow_mut().shell.select_game("exe6")),
    );
    reach(add, game_ready);
    add(SETTLE, go(ScreenId::Menu));
    add(SETTLE * 2, air(true));

    // 4. Play: an invite, a practice session beside it while it waits; the second player joins by its code; the
    // lobby with both, both ready; a few seconds of their match (`screens/play/tour.rs`).
    crate::screens::play::tour::reel(add, &air);

    // 5. Replays: the list, a replay's still; one watched (the cards, then the viewer playing it), a seek further
    // on, at twice the speed.
    add(
        SETTLE,
        Box::new(|_, app, _| app.borrow_mut().shell.nav.focus("play", 0)),
    );
    add(SETTLE * 2, air(true));
    add(600, go(ScreenId::Replays));
    add(0, focus("list", 0));
    add(1300, focus("list", 1));
    add(700, focus("list", 0));
    add(
        600,
        Box::new(|_, app, _| {
            app.borrow_mut()
                .with_screen(ScreenId::Replays, |s, shell| s.activate(shell, "watch", 0))
        }),
    );
    reach(add, |app| {
        app.shell.ui().global::<ReplaysView>().get_watching()
    });
    add(200, focus("seek", 0));
    add(
        2600,
        Box::new(|_, app, _| {
            app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| {
                s.choose(shell, "seek", 0, 6200);
                s.choose(shell, "seek", 1, 6200);
            })
        }),
    );
    add(2200, focus("speed", 2));
    add(
        400,
        Box::new(|_, app, _| {
            app.borrow_mut()
                .with_screen(ScreenId::Replays, |s, shell| s.activate(shell, "speed", 2))
        }),
    );
    add(3000, act(Action::Back));

    // 6. About (Settings' SHOW LICENSES), a moment.
    add(700, go(ScreenId::Settings));
    add(
        0,
        Box::new(|_, app, _| {
            app.borrow_mut()
                .with_screen(ScreenId::Settings, |s, shell| {
                    s.activate(shell, "set.about", 0)
                })
        }),
    );
    add(2800, air(false));
}
