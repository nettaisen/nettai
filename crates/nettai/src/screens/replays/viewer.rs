//! Watching a replay: the library's `Viewer` on screen, with its transport (docs/app.md §16).
//!
//! Each frame runs the ticks due at the speed set, presents the picture if one ran, and hands the sound on. So
//! that no seek stalls the window, the replay is indexed ahead a slice a frame, each held to a budget of time,
//! from the moment it is shown; a seek past what is indexed waits for the index to reach it, the bar showing
//! where it is going meanwhile. A drag of the bar moves the picture in steps of the states the viewer keeps
//! (which cost no playing) and lands where it is let go.
//!
//! Both builds may be shown over it, where the match's setups were open: the narrow sheets a match shows its
//! players' builds in, the console shown's player's at the left; the sides of a match whose setups weren't open
//! aren't kept.
//!
//! Two things more may be on over the picture, each off at first and toggled by its transport button or a key
//! ([`Overlays`]): the input display, both players' buttons as the tick shown holds them (Practice's display,
//! the console shown's player's row over the other's); and the other console's picture, small in the picture's
//! corner, presented from the same tick as the one shown (the library's `Viewer::present_view`: the replay is
//! played once) into a texture of its own (`Shell::show_inset`). Each follows the replay wherever it goes,
//! playing, paused, stepped or sought, since each is drawn from the tick the picture is.
//!
//! Its sound is at the levels Settings has (the shell's `battle_levels`: Music is the replay's music too). And
//! it stands still while the window isn't drawn (minimized, hidden, covered: the frames the shell's timer runs
//! then): nothing of a replay has to go on unseen, and the player finds it where they left it.

use super::{clock, name, played};
use crate::nav::Key;
use crate::shell::Shell;
use crate::sound::Cue;
use crate::stage::Surface;
use nettai_match::Side;
use nettai_frontend::player::{FRAME_RATE, TICK_SECONDS, tick_at, time_of};
use nettai_frontend::viewer::{KEYFRAME_EVERY, ROUND_START_GRACE, Speed, Viewer};
use nettai_ui::ReplaysView;
use slint::{ComponentHandle, ModelRc, VecModel};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long a frame spends indexing ahead while the replay plays, and while nothing does (it is paused, or a
/// seek waits for the index).
const INDEX_BUDGET: Duration = Duration::from_micros(1500);
const IDLE_BUDGET: Duration = Duration::from_millis(5);
/// How long the window goes without drawing before the time isn't the replay's (`Watching::frame`): the most
/// time one frame catches up (the player's own cap).
const STILL_AFTER: Duration = Duration::from_millis(250);

/// Both builds' scope: each sheet is a control of it (`builds.sheet`).
const BUILDS: &str = "builds.";

/// The samples a tick's sound is, at the device's rate: 4389/8.
const TICK_SAMPLES: f64 = 548.625;

/// What is on over the picture, besides the builds: the input display, and the other console's picture in the
/// corner. Off at first; the screen keeps them from one replay to the next while the app runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Overlays {
    pub inputs: bool,
    pub inset: bool,
}

/// The other console's picture is a whole multiple of the frame about this part of the shown one's width (a
/// third), at least the frame itself, in the display's pixels.
const INSET_PART: f32 = 3.0;

/// How far the other console's picture is in from the shown one's bottom right corner, in the game's pixels.
const INSET_MARGIN: f32 = 4.0;

/// What the view was last told, so that it is told only what changed.
#[derive(Default, PartialEq)]
struct Told {
    position: f32,
    second: u64,
    round: u32,
    playing: bool,
}

/// What the viewer costs (`NETTAI_PLAY_STATS`): said every two seconds.
#[derive(Default)]
struct Stats {
    since: Option<Instant>,
    /// The time between each two frames.
    gaps: Vec<Duration>,
    ticks: u32,
    ticking: (Duration, u32, Duration),
    pictures: (Duration, u32, Duration),
    indexing: (Duration, u32, Duration),
    /// The viewer's own sound over a stretch of the replay (the samples as it gives them, before the device's
    /// level): the stretch, the sum of their squares, how many, and the loudest.
    sound: (u32, f64, u64, f32),
}

/// The ticks of a stretch whose sound is said (`NETTAI_PLAY_STATS`): two seconds of the replay, by its own
/// ticks, so that two runs of one replay say the same stretches.
const SOUND_STRETCH: u32 = 120;

/// `NETTAI_VIEWER_PROBE`: the viewer driven as a player would, for its figures (docs/app.md §5): steps
/// `<seconds>:<what>`, comma-separated, each that long after the replay first shows. `click:<part>` presses the
/// bar `part` of the way along and lets go there; `drag:<from>:<to>` presses it at `from`, moves to `to` over a
/// second and lets go there; `speed:<half|1|2|4>` sets the speed. The pointer's are the window's own events,
/// put to it as the system puts them (from a timer, outside the frame).
struct Probe {
    steps: Vec<(f64, ProbeStep)>,
    began: Option<Instant>,
}

#[derive(Clone, Copy)]
enum ProbeStep {
    Click(f32),
    Drag(f32, f32),
    Speed(usize),
}

impl Probe {
    fn of_env() -> Option<Probe> {
        let text = std::env::var("NETTAI_VIEWER_PROBE").ok()?;
        let mut steps = Vec::new();
        for step in text.split(',').filter(|s| !s.trim().is_empty()) {
            let parts: Vec<&str> = step.trim().split(':').collect();
            let num = |at: usize| parts.get(at).and_then(|p| p.parse::<f32>().ok());
            let what = match (parts.get(1).copied(), num(2), num(3)) {
                (Some("click"), Some(part), _) => ProbeStep::Click(part),
                (Some("drag"), Some(from), Some(to)) => ProbeStep::Drag(from, to),
                (Some("speed"), ..) => match ["half", "1", "2", "4"].iter().position(|n| parts.get(2) == Some(n)) {
                    Some(at) => ProbeStep::Speed(at),
                    None => continue,
                },
                _ => {
                    eprintln!("viewer probe: {step:?} is no step");
                    continue;
                }
            };
            if let Some(at) = num(0) {
                steps.push((at as f64, what));
            }
        }
        steps.sort_by(|a, b| a.0.total_cmp(&b.0));
        Some(Probe { steps, began: None })
    }
}

fn add(to: &mut (Duration, u32, Duration), took: Duration) {
    to.0 += took;
    to.1 += 1;
    to.2 = to.2.max(took);
}

/// A replay being watched.
pub struct Watching {
    viewer: Box<Viewer>,
    surface: Surface,
    path: PathBuf,
    last: Option<Instant>,
    /// The sound made and not yet handed to the device.
    queue: Vec<[f32; 2]>,
    /// The silence owed to the device while nothing plays, in ticks.
    quiet: f64,
    /// A seek that waits for the replay to be indexed that far.
    pending: Option<u32>,
    /// Where the bar is held, while it is dragged.
    drag: Option<u32>,
    /// The ticks an index slice plays (fitted to its budget as it goes), and whether the whole replay is
    /// indexed.
    slice: u32,
    whole: bool,
    told: Option<Told>,
    stats: Option<Stats>,
    probe: Option<Probe>,
    /// When the seek that waits was asked for (`NETTAI_PLAY_STATS` says how long it waited).
    asked: Option<Instant>,
    /// The match's two sides, where its builds may be shown; and whether they are.
    sides: Option<Box<[Side; 2]>>,
    builds: bool,
    /// What else is on over the picture; the other console's picture (its own texture); and the buttons the
    /// input display was last told each side holds.
    overlays: Overlays,
    inset: Surface,
    held: Option<[u16; 2]>,
}

impl Watching {
    /// A viewer made ready, of the replay at `path`: playing from its start (the screenshot pass holds it
    /// still, a little way into its second round where it has one), with `overlays` on over its picture.
    pub fn new(mut viewer: Box<Viewer>, path: PathBuf, sides: Option<Box<[Side; 2]>>, overlays: Overlays) -> Watching {
        if let Some(speed) = std::env::var("NETTAI_REPLAY_SPEED").ok().and_then(|s| ["half", "1", "2", "4"].iter().position(|name| *name == s)) {
            viewer.set_speed(Speed::ALL[speed]);
        }
        Watching { viewer, surface: Surface::new(), path, last: None, queue: Vec::new(), quiet: 0.0, pending: None, drag: None, slice: 16, whole: false, told: None, stats: std::env::var_os("NETTAI_PLAY_STATS").map(|_| Stats::default()), probe: Probe::of_env(), asked: None, sides, builds: false, overlays, inset: Surface::new(), held: None }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What is on over the picture besides the builds.
    pub fn overlays(&self) -> Overlays {
        self.overlays
    }

    /// The side whose console is shown.
    pub fn side(&self) -> u8 {
        self.viewer.view()
    }

    /// The speed, as its place among the four.
    pub fn speed(&self) -> i32 {
        Speed::ALL.iter().position(|s| *s == self.viewer.speed()).unwrap_or(1) as i32
    }

    /// Why the replay can't go on from where it is shown: the engine stopped there, short of its end.
    pub fn stopped(&self) -> Option<&str> {
        self.viewer.stopped()
    }

    /// Whether ticks run: it isn't paused, held by a seek or a drag, at its end, or stopped.
    pub fn playing(&self) -> bool {
        !self.viewer.paused() && self.pending.is_none() && self.drag.is_none() && !self.viewer.at_end() && self.viewer.stopped().is_none()
    }

    /// The tick the bar shows: where a drag holds it, where a seek is going, else where the viewer is.
    fn shown(&self) -> u32 {
        self.drag.or(self.pending).unwrap_or(self.viewer.position())
    }

    /// Set the view from the viewer: everything of it (it is shown, or the language changed under it).
    pub fn show(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        if let Some((_, ready)) = shell.ready() {
            self.viewer.set_language(&ready.graphics(shell.lang));
        }
        // (Settings' Music and Sound effects: the difference between the two is the viewer's, as a battle's.)
        let (music, effects) = shell.sound.battle_levels();
        self.viewer.set_sound_levels(music, effects);
        if shell.still && self.told.is_none() {
            // (The screenshot pass: one picture, the same each time. Its showreel plays from the start, and a seek
            // lands at once: the replay indexed whole first, as here.)
            if !shell.reel {
                self.viewer.set_paused(true);
            }
            while !self.viewer.index(u32::MAX) {}
            self.whole = true;
            if !shell.reel {
                let rounds = self.viewer.rounds();
                let at = rounds.get(1).map_or(self.viewer.length() / 2, |start| start + 420);
                self.viewer.seek(at);
            }
        }
        self.surface.refresh();
        self.inset.refresh();
        self.held = None;
        self.show_overlays(&view);
        let length = self.viewer.length().max(1) as f32;
        let marks: Vec<f32> = self.viewer.rounds().iter().skip(1).map(|&start| start as f32 / length).collect();
        view.set_marks(ModelRc::new(VecModel::from(marks)));
        view.set_rounds(self.viewer.rounds().len() as i32);
        view.set_total(clock(self.viewer.duration(), true).into());
        let when = played(&view, self.viewer.info().when);
        view.set_when(format!("{} · {}", when.full, when.time).into());
        view.set_builds_open(self.sides.is_some());
        if self.builds {
            self.show_builds(shell);
        }
        view.set_builds_shown(self.builds);
        self.show_side(&view);
        view.set_speed(self.speed());
        self.told = None;
        self.tell(&view);
        view.set_watching(true);
    }

    /// Who is playing, the console shown first (its side is at the picture's left).
    fn show_side(&self, view: &ReplaysView) {
        let (info, side) = (self.viewer.info(), self.viewer.view());
        view.set_view(side as i32);
        view.set_title(view.invoke_versus(name(view, info, side).into(), name(view, info, side ^ 1).into()));
    }

    /// Tell the view where the replay is, where that changed.
    fn tell(&mut self, view: &ReplaysView) {
        let tick = self.shown();
        let now = Told { position: tick as f32 / self.viewer.length().max(1) as f32, second: time_of(tick as u64).as_secs(), round: self.viewer.round_at(tick), playing: self.playing() };
        let was = self.told.as_ref();
        if was.is_none_or(|t| t.position != now.position) {
            view.set_position(now.position);
        }
        if was.is_none_or(|t| t.second != now.second) {
            view.set_time(clock(Duration::from_secs(now.second), true).into());
        }
        if was.is_none_or(|t| t.round != now.round) {
            view.set_round(now.round as i32);
        }
        if was.is_none_or(|t| t.playing != now.playing) {
            view.set_playing(now.playing);
        }
        self.told = Some(now);
    }

    // ---- The transport ----------------------------------------------------------------------------------

    /// Go to `tick`: at once where the replay is indexed that far, else once it is.
    fn go(&mut self, tick: u32) {
        let tick = tick.min(self.viewer.length());
        if self.whole || tick <= self.viewer.indexed() {
            self.pending = None;
            self.viewer.seek(tick);
            self.landed();
            if self.stats.is_some() {
                eprintln!("viewer seek: to tick {tick}, indexed that far: landed at once, at tick {}", self.viewer.position());
            }
        } else {
            if self.stats.is_some() {
                eprintln!("viewer seek: to tick {tick}, past the index ({} of {} ticks): it waits", self.viewer.indexed(), self.viewer.length());
            }
            self.pending = Some(tick);
            self.asked = Some(Instant::now());
        }
    }

    /// The viewer is somewhere else: what was left of the sound is of where it was.
    fn landed(&mut self) {
        self.queue.clear();
        self.surface.stale = true;
    }

    /// Play, or pause; from the end, play again from the start.
    pub fn toggle(&mut self) {
        if self.drag.is_some() {
            return;
        }
        if self.viewer.at_end() && self.pending.is_none() {
            self.go(0);
            self.viewer.set_paused(false);
        } else {
            self.viewer.set_paused(!self.viewer.paused());
        }
    }

    pub fn pause(&mut self) {
        self.viewer.set_paused(true);
    }

    /// `seconds` on, or back.
    pub fn skip(&mut self, seconds: i32) {
        let ticks = tick_at(Duration::from_secs(seconds.unsigned_abs() as u64)) as u32;
        let from = self.pending.unwrap_or(self.viewer.position());
        self.go(if seconds < 0 { from.saturating_sub(ticks) } else { from.saturating_add(ticks) });
    }

    /// A frame on (with its sound) or back, paused.
    pub fn step(&mut self, by: i32) {
        self.viewer.set_paused(true);
        if self.pending.is_some() || self.drag.is_some() {
            return;
        }
        if by > 0 {
            self.viewer.step_forward();
            self.surface.stale = true;
        } else {
            self.viewer.step_back();
            self.landed();
        }
    }

    /// To the next round's start; or to this round's, and from there (while it plays, from its first second)
    /// to the round before's.
    pub fn round(&mut self, by: i32) {
        let from = self.pending.unwrap_or(self.viewer.position());
        let at = self.viewer.round_at(from) as usize - 1;
        let rounds = self.viewer.rounds();
        if by > 0 {
            if let Some(&next) = rounds.get(at + 1) {
                self.go(next);
            }
        } else {
            let start = rounds[at];
            let grace = if self.viewer.paused() { 0 } else { ROUND_START_GRACE };
            let to = if at > 0 && from - start <= grace { rounds[at - 1] } else { start };
            self.go(to);
        }
    }

    /// The speed at `at` of the four (past either end: that end).
    pub fn set_speed(&mut self, at: i32) {
        self.viewer.set_speed(Speed::ALL[at.clamp(0, 3) as usize]);
        self.queue.clear();
    }

    /// Show `side`'s console.
    pub fn set_view(&mut self, shell: &mut Shell, side: i32) {
        if self.viewer.set_view(side.clamp(0, 1) as u8) {
            self.landed();
            self.show_side(&shell.ui().global::<ReplaysView>());
            if self.builds {
                self.show_builds(shell);
            }
        }
    }

    /// The input display on, or off.
    pub fn toggle_inputs(&mut self, shell: &mut Shell) {
        self.overlays.inputs = !self.overlays.inputs;
        self.held = None;
        self.show_overlays(&shell.ui().global::<ReplaysView>());
    }

    /// The other console's picture in the corner on, or off.
    pub fn toggle_inset(&mut self, shell: &mut Shell) {
        self.overlays.inset = !self.overlays.inset;
        self.inset.refresh();
        self.show_overlays(&shell.ui().global::<ReplaysView>());
    }

    fn show_overlays(&self, view: &ReplaysView) {
        view.set_inputs(self.overlays.inputs);
        view.set_inset_on(self.overlays.inset);
    }

    /// Show both builds, or put them away (a match whose setups weren't open: refused).
    pub fn toggle_builds(&mut self, shell: &mut Shell) {
        if self.sides.is_none() {
            return shell.sound.play(Cue::Refused);
        }
        self.builds = !self.builds;
        if self.builds {
            self.show_builds(shell);
            shell.nav.open(BUILDS, Key::new("builds.sheet", 0));
        } else {
            shell.nav.close(BUILDS);
        }
        shell.ui().global::<ReplaysView>().set_builds_shown(self.builds);
    }

    /// Put both builds away. True if they were shown.
    pub fn hide_builds(&mut self, shell: &mut Shell) -> bool {
        let was = std::mem::take(&mut self.builds);
        if was {
            shell.nav.close(BUILDS);
            shell.ui().global::<ReplaysView>().set_builds_shown(false);
        }
        was
    }

    /// Both builds' sheets, in the language shown: the console shown's player's at the left. Over each name,
    /// YOU for the side that recorded it, else OPPONENT; the name is the player's (a replay has no build's).
    fn show_builds(&self, shell: &Shell) {
        let (Some(sides), Some(kit)) = (&self.sides, crate::builds::kit(shell)) else { return };
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        let info = self.viewer.info();
        let left = self.viewer.view();
        let sheet = |of: u8| {
            let label = if of == info.side { view.get_you() } else { view.get_opponent() };
            crate::builds::overview::sheet(shell, &kit, &label, &name(&view, info, of), &sides[of as usize])
        };
        view.set_left_build(sheet(left));
        view.set_right_build(sheet(left ^ 1));
    }

    /// The bar held at `part` of 10,000 of the replay, or (`done`) let go there. While it is held the picture
    /// follows in steps: the state the viewer keeps at or before it, which costs no playing.
    pub fn drag(&mut self, part: i32, done: bool) {
        let target = (self.viewer.length() as u64 * part.clamp(0, 10_000) as u64 / 10_000) as u32;
        self.pending = None;
        if done {
            self.drag = None;
            return self.go(target);
        }
        self.drag = Some(target);
        let step = target.min(self.viewer.indexed()) / KEYFRAME_EVERY * KEYFRAME_EVERY;
        if step != self.viewer.position() {
            self.viewer.seek(step);
            self.landed();
            if self.stats.is_some() {
                eprintln!("viewer drag: held at tick {target}: the picture at tick {}", self.viewer.position());
            }
        }
    }

    /// `NETTAI_VIEWER_PROBE`'s steps that are due.
    fn probe(&mut self, shell: &Shell, now: Instant) {
        let Some(probe) = &mut self.probe else { return };
        let since = now.duration_since(*probe.began.get_or_insert(now)).as_secs_f64();
        let due = probe.steps.iter().take_while(|(at, _)| *at <= since).count();
        let steps: Vec<ProbeStep> = probe.steps.drain(..due).map(|(_, step)| step).collect();
        for step in steps {
            let (from, to) = match step {
                ProbeStep::Speed(at) => {
                    self.set_speed(at as i32);
                    eprintln!("viewer probe: at {since:.1} s, the speed set to {}", ["half", "1", "2", "4"][at]);
                    continue;
                }
                ProbeStep::Click(part) => (part, part),
                ProbeStep::Drag(from, to) => (from, to),
            };
            let Some(bar) = shell.nav.registry().borrow().rect(&crate::nav::Key::new("seek", 0)) else {
                eprintln!("viewer probe: the bar isn't laid out");
                continue;
            };
            eprintln!("viewer probe: at {since:.1} s, the bar pressed at {from} and let go at {to} (at tick {})", self.viewer.position());
            let at = move |part: f32| slint::LogicalPosition::new(bar.x + bar.w * part, bar.y + bar.h / 2.0);
            use slint::platform::{PointerEventButton, WindowEvent};
            let button = PointerEventButton::Left;
            let moves = if from == to { 0 } else { 20 };
            let mut events = vec![(0, WindowEvent::PointerPressed { position: at(from), button })];
            events.extend((1..=moves).map(|n| (n * 50, WindowEvent::PointerMoved { position: at(from + (to - from) * n as f32 / moves as f32) })));
            events.push((moves * 50 + 50, WindowEvent::PointerReleased { position: at(to), button }));
            for (ms, event) in events {
                let ui = shell.ui().as_weak();
                slint::Timer::single_shot(Duration::from_millis(ms as u64), move || {
                    if let Some(ui) = ui.upgrade() {
                        ui.window().dispatch_event(event);
                    }
                });
            }
        }
    }

    // ---- The frame --------------------------------------------------------------------------------------

    /// A frame at `now`: the seek that waited, a slice of the index, the ticks due, the sound, the picture.
    pub fn frame(&mut self, shell: &mut Shell, now: Instant) {
        // (A gap of [`STILL_AFTER`] or more since the last drawn frame isn't the replay's time: the window wasn't
        // drawn. A shorter one, frames the system was late with, is made up, so the replay and its bar don't
        // lose it.)
        let elapsed = self.last.map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        let elapsed = if elapsed >= STILL_AFTER { Duration::ZERO } else { elapsed };
        self.last = Some(now);
        self.probe(shell, now);
        if let Some(target) = self.pending
            && (self.whole || self.viewer.indexed() >= target)
        {
            self.pending = None;
            self.viewer.seek(target);
            self.landed();
            if let (Some(_), Some(asked)) = (&self.stats, self.asked.take()) {
                eprintln!("viewer seek: landed at tick {} ({target} asked for), {:.2?} after it was asked", self.viewer.position(), asked.elapsed());
            }
        }
        // Indexing ahead: a slice, of as many ticks as the last one's time says fit the budget.
        if !self.whole {
            let budget = if self.playing() { INDEX_BUDGET } else { IDLE_BUDGET };
            let began = Instant::now();
            self.whole = self.viewer.index(self.slice);
            let took = began.elapsed();
            let fits = budget.as_secs_f64() / took.as_secs_f64().max(1e-6) * self.slice as f64;
            // (Half way there each time: one slow slice doesn't halve the next.)
            self.slice = ((self.slice as f64 + fits) / 2.0).clamp(4.0, 4000.0) as u32;
            if let Some(s) = &mut self.stats {
                add(&mut s.indexing, took);
            }
        }
        let playing = self.playing();
        let began = Instant::now();
        let ran = if playing { self.viewer.advance(elapsed) } else { 0 };
        if ran > 0 {
            self.surface.stale = true;
            if let Some(s) = &mut self.stats {
                s.ticks += ran;
                add(&mut s.ticking, began.elapsed());
            }
        }
        let had = self.queue.len();
        self.viewer.take_samples(&mut self.queue);
        self.measure_sound(had);
        self.hand_sound(shell, elapsed, playing);
        let Watching { viewer, surface, inset, .. } = self;
        let (stale, began) = (surface.stale, Instant::now());
        // (The other console's picture is of the same tick: out of date when this one is.)
        inset.stale |= stale;
        shell.show_surface(surface, |buffer, width, height| viewer.present(buffer, width, height));
        if self.overlays.inset {
            self.show_inset(shell);
        }
        if let (Some(s), true) = (self.stats.as_mut(), stale) {
            add(&mut s.pictures, began.elapsed());
        }
        let ui = shell.ui();
        let view = ui.global::<ReplaysView>();
        self.show_held(&view);
        self.tell(&view);
        self.report(now, elapsed);
    }

    /// The other console's picture, small in the shown one's bottom right corner: a whole multiple of the frame
    /// about a third of its width, a few of the game's pixels in from its corner, on the display's pixels.
    fn show_inset(&mut self, shell: &mut Shell) {
        let Some(fit) = self.surface.fit() else { return };
        let multiple = ((fit.scale as f32 / INSET_PART).round() as u32).max(1);
        let other = self.viewer.view() ^ 1;
        let viewer = &mut self.viewer;
        let Some((image, width, height)) = shell.show_inset(&mut self.inset, multiple, |buffer, w, h| viewer.present_view(other, buffer, w, h)) else { return };
        let ui = shell.ui();
        let (view, picture) = (ui.global::<ReplaysView>(), ui.global::<nettai_ui::Shell>());
        if let Some(image) = image {
            view.set_inset(image);
        }
        let margin = INSET_MARGIN * fit.scale as f32 / ui.window().scale_factor();
        view.set_inset_x(picture.get_picture_x() + picture.get_picture_width() - width - margin);
        view.set_inset_y(picture.get_picture_y() + picture.get_picture_height() - height - margin);
        view.set_inset_width(width);
        view.set_inset_height(height);
    }

    /// The input display's buttons: each side's as the tick shown holds them (the battle's own, as Practice's
    /// are), the console shown's player's first. Told where they changed.
    fn show_held(&mut self, view: &ReplaysView) {
        if !self.overlays.inputs {
            return;
        }
        let shown = self.viewer.view() as usize;
        let joypads = &self.viewer.battle().custom.sides;
        let held = [joypads[shown].joypad.held, joypads[shown ^ 1].joypad.held];
        if self.held != Some(held) {
            self.held = Some(held);
            view.set_held_shown(held[0] as i32);
            view.set_held_other(held[1] as i32);
        }
    }

    /// A frame the window doesn't draw (the shell's timer runs it: the window is minimized, hidden or covered):
    /// the replay stands where it is. (The next drawn frame makes up a gap shorter than [`STILL_AFTER`]: `frame`.)
    pub fn rest(&mut self, shell: &mut Shell) {
        // (Nothing sounds; and nothing of the menus' music starts under a replay.)
        shell.play_samples(&mut Vec::new(), 0);
    }

    /// `NETTAI_PLAY_STATS`: how loud the viewer's own sound was over each stretch of the replay, from the
    /// samples added to the queue since it held `had`.
    fn measure_sound(&mut self, had: usize) {
        let Some(s) = &mut self.stats else { return };
        let stretch = self.viewer.position() / SOUND_STRETCH;
        if stretch != s.sound.0 {
            let (of, squares, count, peak) = s.sound;
            eprintln!("viewer sound: ticks {} to {}: rms {:.5}, peak {:.4}, {count} samples", of * SOUND_STRETCH, (of + 1) * SOUND_STRETCH, (squares / count.max(1) as f64).sqrt(), peak);
            s.sound = (stretch, 0.0, 0, 0.0);
        }
        for sample in &self.queue[had.min(self.queue.len())..] {
            s.sound.1 += (sample[0] as f64).powi(2) + (sample[1] as f64).powi(2);
            s.sound.2 += 2;
            s.sound.3 = s.sound.3.max(sample[0].abs()).max(sample[1].abs());
        }
    }

    /// Hand the device this frame's sound. The menus' sounds are ticked with what is handed over (as with a
    /// battle's: `Shell::play_samples`), so it goes in whole ticks' worth: the viewer's own while it plays, at
    /// whatever speed; and while it doesn't, silence at the original's rate (with what was left of its sound),
    /// so that a move of the cursor is heard and the title music isn't.
    fn hand_sound(&mut self, shell: &mut Shell, elapsed: Duration, playing: bool) {
        let (ticks, samples) = if playing {
            let ticks = self.queue.len() / TICK_SAMPLES as usize;
            (ticks, self.queue.len().min((ticks as f64 * TICK_SAMPLES).ceil() as usize))
        } else {
            self.quiet += elapsed.min(Duration::from_millis(250)).as_secs_f64() * FRAME_RATE;
            let ticks = self.quiet.floor();
            self.quiet -= ticks;
            (ticks as usize, (ticks * TICK_SAMPLES).round() as usize)
        };
        let mut out: Vec<[f32; 2]> = self.queue.drain(..samples.min(self.queue.len())).collect();
        out.resize(samples, [0.0; 2]);
        shell.play_samples(&mut out, ticks as u32);
    }

    /// `NETTAI_PLAY_STATS`: every two seconds, what the viewer held and what it cost.
    fn report(&mut self, now: Instant, elapsed: Duration) {
        let Some(s) = &mut self.stats else { return };
        if elapsed > Duration::ZERO {
            s.gaps.push(elapsed);
        }
        let since = *s.since.get_or_insert(now);
        let seconds = now.duration_since(since).as_secs_f64();
        if seconds < 2.0 {
            return;
        }
        s.gaps.sort();
        let at = |q: f64| s.gaps.get(((s.gaps.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or_default();
        // (A frame that took half again the usual: a refresh missed.)
        let long = s.gaps.iter().filter(|g| g.as_secs_f64() > at(0.5).as_secs_f64() * 1.5).count();
        let mean = |of: &(Duration, u32, Duration)| of.0 / of.1.max(1);
        let size = self.surface.fit().map_or((0, 0), |fit| (240 * fit.density, 160 * fit.density));
        eprintln!(
            "viewer stats: {:.1} frames/s (frame time p50 {:.2?} p95 {:.2?} p99 {:.2?} max {:.2?}, {long} long), {:.2} ticks/s at {}x ({:.2?} a frame that ran them, worst {:.2?}); \
             {} pictures ({}x{}): {:.2?} each (worst {:.2?}); index: {} slices, {:.2?} each (worst {:.2?}), {} of {} ticks",
            s.gaps.len() as f64 / seconds,
            at(0.5),
            at(0.95),
            at(0.99),
            at(1.0),
            s.ticks as f64 / seconds,
            self.viewer.speed().factor(),
            mean(&s.ticking),
            s.ticking.2,
            s.pictures.1,
            size.0,
            size.1,
            mean(&s.pictures),
            s.pictures.2,
            s.indexing.1,
            mean(&s.indexing),
            s.indexing.2,
            self.viewer.indexed(),
            self.viewer.length(),
        );
        // (The sound's stretch is the replay's own, and goes on across the two seconds.)
        *s = Stats { since: Some(now), sound: s.sound, ..Stats::default() };
    }
}

// (A tick's sound is 4389/8 samples by the library's clock.)
const _: () = assert!(TICK_SECONDS.0 == 4389 && nettai_audio::SAMPLE_RATE == 32768);
