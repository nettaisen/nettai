//! Watching a replay: [`Viewer`] plays one back for a host to show and
//! hear, as a [`crate::Player`] does, and has a transport besides. It knows
//! the replay's length and where each round starts as soon as it opens (the
//! file marks them), seeks to any tick or time, goes five seconds or a
//! round back or on, steps a frame either way, plays at four speeds, and
//! shows either console, switched while it plays.
//!
//! **A seek lands in the state playing straight there reaches**, all of
//! it: the battle (its digest is straight playback's), what the drawing
//! follows over time (the HUD's rolling numbers and timers) and the sound,
//! down to the sample, for both consoles. Nothing is approximated, so what
//! is shown and heard after a seek, a step back or a change of console is
//! what was shown and heard there the first time. The simulation is never
//! asked anything a tick doesn't ask: a seek restores a state the replay
//! passed through and plays the recorded buttons on from it.
//!
//! A viewer has no pause of the battle's: where a match's host answers
//! START (`HostKeys::pause`), the presses a replay recorded are reported
//! again as it plays, and a viewer drops them. Its pause is
//! [`Viewer::set_paused`], its host's own.
//!
//! **How.** As the replay plays (or is sought through), the viewer keeps a
//! copy of everything it is at every [`KEYFRAME_EVERY`]th tick and at each
//! round's first tick: the battle (`Battle::save_state`), each console's
//! HUD state, each console's sound (`BattleAudio` is plain state over its
//! shared banks). A seek goes on from the nearest kept state at or before
//! its target, or from where the viewer is if that is nearer, and plays
//! the ticks between in silence. Both consoles' HUD states and sounds run
//! every tick, so the one not shown is ready when the view changes; the
//! battle itself is one simulation whoever watches. The ticks a seek plays
//! through leave states of their own every [`NEAR_EVERY`]th tick, of that
//! one stretch between two kept states, so a frame back and the next one
//! each go on from a tick or so before, not from four seconds before.
//!
//! **What it costs.** A kept state is a battle and two consoles' sound and
//! HUD state; with [`KEYFRAME_EVERY`] at four seconds, a five-minute set
//! keeps about 80. Reaching a tick for the first time plays every tick up
//! to it, sound included (the sound is most of a tick's cost); after
//! that a seek plays at most [`KEYFRAME_EVERY`] - 1 ticks, and one into the
//! stretch last sought through fewer than [`NEAR_EVERY`]. [`Viewer::index`]
//! plays ahead a slice at a time without moving, for a host that wants
//! every seek short before its user drags the bar. The figures measured on
//! a game's own packs are docs/frontend.md's (§8, "Seeking").
//!
//! ```ignore
//! let mut viewer = Viewer::open(&game, &replay, None)?;       // a Refused says why not
//! let (length, rounds) = (viewer.duration(), viewer.rounds()); // for the bar and its marks
//! while window.is_open() {
//!     viewer.advance(now - last);                              // the ticks due, at the speed set
//!     viewer.present(window.pixels(), window.width(), window.height());
//!     viewer.take_samples(&mut samples);
//!     // The transport: viewer.seek_time(t), viewer.skip(-5), viewer.previous_round(),
//!     // viewer.step_back(), viewer.set_speed(Speed::Double), viewer.set_view(1), ...
//! }
//! ```

use crate::driver::Place;
use crate::game::{Graphics, Loaded};
use crate::player::{Clock, MOST_SAMPLES, tick_at, time_of};
use crate::replay::{End, Info, Refused, Replay, ReplayPlayer};
use crate::session::Session;
use nettai_audio::BattleAudio;
use nettai_battle::content::Content;
use nettai_battle::{Battle, BattleResult, RoundEnd, Snapshot};
use nettai_render::hud::HudState;
use nettai_render::vfont::TextRenderer;
use nettai_render::{Frame, Renderer};
use std::sync::Arc;
use std::time::Duration;

/// The ticks between two of the states a viewer keeps to seek from (it
/// keeps each round's first tick's besides): about four seconds. A seek
/// plays fewer ticks than this once its target has been reached before.
pub const KEYFRAME_EVERY: u32 = 240;

/// The ticks between two of the states kept of the stretch a seek last
/// played through (between two kept states: at most
/// `KEYFRAME_EVERY / NEAR_EVERY - 1` of them, dropped when a seek plays
/// through another stretch). A frame back plays fewer ticks than this once
/// the one before it has played its stretch.
pub const NEAR_EVERY: u32 = 16;

/// While it plays, a viewer within this many ticks of a round's start (a
/// second) goes to the round before's when asked for the round's start
/// ([`Viewer::previous_round`]), as a music player's "previous" does.
pub const ROUND_START_GRACE: u32 = 60;

/// A speed a replay is watched at, as a multiple of the original's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Speed {
    Half,
    #[default]
    Normal,
    Double,
    Quadruple,
}

impl Speed {
    /// Every speed, slowest first.
    pub const ALL: [Speed; 4] = [Speed::Half, Speed::Normal, Speed::Double, Speed::Quadruple];

    /// The speed as a multiple of the original's.
    pub fn factor(self) -> f64 {
        match self {
            Speed::Half => 0.5,
            Speed::Normal => 1.0,
            Speed::Double => 2.0,
            Speed::Quadruple => 4.0,
        }
    }
}

/// Everything a viewer is at a tick: the battle, what each console's
/// drawing follows over time, each console's sound, and how the session
/// stands. Going on from a copy is going on from there.
#[derive(Clone)]
struct State {
    /// The set's ticks played before it.
    tick: u32,
    battle: Snapshot,
    /// By side.
    huds: [HudState; 2],
    /// By side; none: a silent viewer.
    audio: Option<[BattleAudio; 2]>,
    /// The session's: why it stopped, if it had, whether that was the
    /// replay's end, and the set's result.
    stopped: Option<String>,
    finished: bool,
    result: Option<BattleResult>,
}

/// A kept state to go on from.
#[derive(Clone, Copy)]
enum Kept {
    Keyframe(usize),
    Frontier,
    Near(usize),
}

/// A replay being watched. It owns everything it needs (the graphics and
/// the sound banks are shared with whoever loaded them), so a host keeps
/// it wherever it keeps its state.
pub struct Viewer {
    session: Session,
    renderer: Renderer,
    text: Option<TextRenderer>,
    /// Each console's sound, by side (none: silent). Both run every tick.
    audio: Option<[BattleAudio; 2]>,
    /// The host's levels for the sound, the music's and the effects'
    /// ([`Viewer::set_sound_levels`]): the viewer's, whatever state it is
    /// put back to.
    levels: (f32, f32),
    /// What the drawing of the console not shown follows over time (the
    /// renderer has the shown one's).
    other_hud: HudState,
    /// The side whose console is shown.
    view: u8,
    /// The replay's ticks, how its input ends, and what it says of
    /// itself.
    length: u32,
    end: End,
    info: Info,
    /// Each round's first tick, in order (the file's marks).
    rounds: Vec<u32>,
    /// The states kept, by tick, ascending: tick 0's, every
    /// [`KEYFRAME_EVERY`]th's and each round's first's, up to `reached`.
    keyframes: Vec<State>,
    /// Where indexing ahead left off, between two keyframes
    /// ([`Viewer::index`]).
    frontier: Option<State>,
    /// States of the stretch a seek last played through, every
    /// [`NEAR_EVERY`]th tick between two keyframes, by tick, ascending.
    near: Vec<State>,
    /// The ticks seeks have played in silence (what seeking cost so far).
    sought: u64,
    /// The furthest tick reached, by playing, seeking or indexing.
    reached: u32,
    /// Nothing is left to index: the end was reached, or the engine
    /// stopped before it.
    whole: bool,
    paused: bool,
    speed: Speed,
    clock: Clock,
    /// The sound of the ticks played since the host last took it.
    samples: Vec<[f32; 2]>,
    /// A tick's samples as the sound makes them, and those of a group not
    /// yet whole at a speed above the original's.
    scratch: Vec<[f32; 2]>,
    group: Vec<[f32; 2]>,
}

// (A viewer borrows nothing: a host's state of any lifetime holds one.)
const _: () = {
    const fn owned<T: 'static>() {}
    owned::<Viewer>();
};

impl Viewer {
    /// A viewer of `replay` on a loaded game (its sound, if it was loaded),
    /// shown from `view`'s side (none: the side that recorded it), at the
    /// replay's start, playing. Refused as [`ReplayPlayer::new`] refuses a
    /// replay: one of another game, of another version of nettai, or whose
    /// match doesn't read.
    pub fn open(game: &Loaded, replay: &Replay, view: Option<u8>) -> Result<Viewer, Refused> {
        let audio = game.sound.as_ref().map(|s| BattleAudio::with_banks(s.banks.clone(), s.songs.clone()));
        Viewer::with(&game.game.content, replay, view, game.renderer(), game.font.clone().map(TextRenderer::new), audio)
    }

    /// A viewer from its parts: the content the replay plays on, the
    /// renderer, the enhanced mode's text renderer (none: the frames carry
    /// their own text) and the sound (none: silent; it is copied for the
    /// other console).
    pub fn with(
        content: &Arc<Content>,
        replay: &Replay,
        view: Option<u8>,
        mut renderer: Renderer,
        text: Option<TextRenderer>,
        audio: Option<BattleAudio>,
    ) -> Result<Viewer, Refused> {
        let driver = ReplayPlayer::new(content, replay, view)?;
        let view = view.unwrap_or(replay.info.side);
        let session = Session::new(Box::new(driver));
        renderer.reset();
        renderer.console_version = session.driver.console_version();
        let mut viewer = Viewer {
            session,
            renderer,
            text,
            audio: audio.map(|a| [a.clone(), a]),
            levels: (1.0, 1.0),
            other_hud: HudState::default(),
            view,
            length: replay.ticks.len() as u32,
            end: replay.end,
            info: replay.info.clone(),
            rounds: replay.round_starts(),
            keyframes: Vec::new(),
            frontier: None,
            near: Vec::new(),
            sought: 0,
            reached: 0,
            whole: replay.ticks.is_empty(),
            paused: false,
            speed: Speed::Normal,
            clock: Clock::new(),
            samples: Vec::new(),
            scratch: Vec::new(),
            group: Vec::new(),
        };
        viewer.keyframes.push(viewer.state());
        Ok(viewer)
    }

    // ---- What it plays -----------------------------------------------------

    /// The replay's length in ticks.
    pub fn length(&self) -> u32 {
        self.length
    }

    /// The replay's length in time, at the original's speed.
    pub fn duration(&self) -> Duration {
        time_of(self.length as u64)
    }

    /// Each round's first tick, in order: the first round's 0. (The time of
    /// one: [`crate::player::time_of`].)
    pub fn rounds(&self) -> &[u32] {
        &self.rounds
    }

    /// How the replay's input ends: with the set's end, or before it (the
    /// recording stopped, or its file was cut short).
    pub fn end(&self) -> End {
        self.end
    }

    /// What the replay says of itself: when it was played, by whom (the
    /// players' names by side), which side recorded it, what kind of match
    /// it was, and whether its builds were closed. Where they were
    /// (`Info::closed_builds`), the host shows neither side's build: the
    /// viewer plays the battle, which is made of both, and tells nobody
    /// what either brought.
    pub fn info(&self) -> &Info {
        &self.info
    }

    // ---- Where it is -------------------------------------------------------

    /// The ticks played before where the viewer is: 0 at the start, the
    /// length at the end.
    pub fn position(&self) -> u32 {
        self.session.ticks as u32
    }

    /// Where the viewer is in time, at the original's speed.
    pub fn time(&self) -> Duration {
        time_of(self.session.ticks)
    }

    /// The round being shown, from 1.
    pub fn round(&self) -> u32 {
        self.round_at(self.position())
    }

    /// The round the tick `tick` is in, from 1.
    pub fn round_at(&self, tick: u32) -> u32 {
        self.rounds.partition_point(|&start| start <= tick).max(1) as u32
    }

    /// The viewer is at the replay's end: nothing more plays.
    pub fn at_end(&self) -> bool {
        self.position() >= self.length
    }

    /// Why the replay can't go on from here, where the engine stopped
    /// before the replay's end (it plays up to there and no further).
    pub fn stopped(&self) -> Option<&str> {
        self.session.stopped.as_deref().filter(|_| !self.session.finished)
    }

    /// The first difference from what the replay recorded (its digests,
    /// its rounds' ends), once one was played through.
    pub fn diverged(&self) -> Option<&str> {
        self.session.diverged.as_deref()
    }

    /// The set's result for the console shown, at the end of a replay that
    /// holds the whole set.
    pub fn result(&self) -> Option<BattleResult> {
        self.session.result?;
        match self.session.battle.round_end_for(self.view) {
            Some(RoundEnd::Over(result)) => Some(result),
            _ => self.session.result,
        }
    }

    /// The battle as it stands.
    pub fn battle(&self) -> &Battle {
        &self.session.battle
    }

    // ---- Playing -----------------------------------------------------------

    /// Run the ticks due after `elapsed` more time at the speed set, the
    /// part of a tick left over kept for the next call (a host calls it
    /// once a display frame, as [`crate::Player::advance`]). None while
    /// paused, at the end, or where the engine stopped. The ticks run.
    pub fn advance(&mut self, elapsed: Duration) -> u32 {
        if self.paused || self.at_end() || self.session.stopped.is_some() {
            self.clock.rest();
            return 0;
        }
        let due = self.clock.due(elapsed, self.speed.factor());
        (0..due).take_while(|_| self.run(true)).count() as u32
    }

    /// Pause, or go on.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        self.clock.rest();
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Play at `speed` from now on. The sound follows it as a tape's does:
    /// at half speed an octave down, at double and four times one and two
    /// octaves up, so the host is given as much sound a second at any
    /// speed.
    pub fn set_speed(&mut self, speed: Speed) {
        self.speed = speed;
        self.group.clear();
    }

    pub fn speed(&self) -> Speed {
        self.speed
    }

    /// A frame on: one tick, whatever the clock or the pause say, with its
    /// sound. False at the end, or where the engine stopped.
    pub fn step_forward(&mut self) -> bool {
        self.run(true)
    }

    /// A frame back: the tick before this one, in silence.
    pub fn step_back(&mut self) {
        self.seek(self.position().saturating_sub(1));
    }

    // ---- Seeking -----------------------------------------------------------

    /// Go to `tick` (the ticks played before it; past the end: the end).
    /// The viewer is then exactly as playing straight there leaves it; what
    /// it had of sound for the host is dropped, and the ticks between are
    /// not heard. Paused or playing, it stays as it was.
    pub fn seek(&mut self, tick: u32) {
        let target = tick.min(self.length);
        let here = self.position();
        if target == here {
            return;
        }
        // From here, if the target is ahead and no kept state is nearer to
        // it; else from the nearest kept state at or before it.
        let (kept, at) = self.nearest(target);
        if here > target || at > here || self.session.stopped.is_some() {
            self.restore(kept);
        }
        while self.position() < target && self.run(false) {
            self.sought += 1;
            self.keep_near();
        }
        self.settle();
    }

    /// Keep the state a seek is passing through, if it is one of the
    /// stretch's every [`NEAR_EVERY`]th: the states of another stretch go.
    fn keep_near(&mut self) {
        let here = self.position();
        if here % NEAR_EVERY != 0 || self.session.stopped.is_some() || self.keyframes.binary_search_by_key(&here, |k| k.tick).is_ok() {
            return;
        }
        let stretch = |tick: u32| tick / KEYFRAME_EVERY;
        if self.near.first().is_some_and(|n| stretch(n.tick) != stretch(here)) {
            self.near.clear();
        }
        if let Err(at) = self.near.binary_search_by_key(&here, |n| n.tick) {
            let state = self.state();
            self.near.insert(at, state);
        }
    }

    /// Go to the tick nearest to `time` from the replay's start.
    pub fn seek_time(&mut self, time: Duration) {
        self.seek(tick_at(time).min(self.length as u64) as u32);
    }

    /// Go `seconds` on (or, negative, back), to the start or the end at
    /// most.
    pub fn skip(&mut self, seconds: i32) {
        let ticks = tick_at(Duration::from_secs(seconds.unsigned_abs() as u64)).min(u32::MAX as u64) as u32;
        let here = self.position();
        self.seek(if seconds < 0 { here.saturating_sub(ticks) } else { here.saturating_add(ticks) });
    }

    /// Go to the start of the round being shown; from its start, to the
    /// round before's. While playing, "its start" is its first second
    /// ([`ROUND_START_GRACE`]), so asking twice goes back two rounds.
    pub fn previous_round(&mut self) {
        let here = self.position();
        let round = self.round() as usize - 1;
        let start = self.rounds[round];
        let grace = if self.paused { 0 } else { ROUND_START_GRACE };
        let before = round > 0 && here - start <= grace;
        self.seek(if before { self.rounds[round - 1] } else { start });
    }

    /// Go to the next round's start. False, and nothing changes, in the
    /// last round.
    pub fn next_round(&mut self) -> bool {
        let Some(&start) = self.rounds.get(self.round() as usize) else { return false };
        self.seek(start);
        true
    }

    /// The side whose console is shown.
    pub fn view(&self) -> u8 {
        self.view
    }

    /// Show `side`'s console from now on: its picture, its HUD and its
    /// sound, each as a viewer shown from that side all along has them.
    /// False for a side a match doesn't have.
    pub fn set_view(&mut self, side: u8) -> bool {
        if side > 1 {
            return false;
        }
        if side != self.view {
            self.view = side;
            std::mem::swap(&mut self.renderer.hud_state, &mut self.other_hud);
            self.session.battle.setup.local_side = side;
            let here = self.place(self.position());
            self.session.driver.go_to(here);
            self.settle();
        }
        true
    }

    // ---- Indexing ahead ----------------------------------------------------

    /// The furthest tick reached so far, by playing, seeking or indexing: a
    /// seek to a tick up to it plays fewer than [`KEYFRAME_EVERY`] ticks.
    pub fn indexed(&self) -> u32 {
        self.reached
    }

    /// Play up to `ticks` ticks of the replay beyond what was reached so
    /// far, in silence and without moving: the viewer is left exactly
    /// where and as it was, and the states seeks go on from are kept for
    /// the part played. True once the whole replay is indexed. A host that
    /// wants every seek short calls it a slice a frame (or all at once
    /// behind a loading screen) until then; one that doesn't call it pays
    /// for the ticks between on the first seek past [`Viewer::indexed`].
    pub fn index(&mut self, ticks: u32) -> bool {
        if self.whole || ticks == 0 {
            return self.whole;
        }
        let here = self.state();
        let (furthest, _) = self.nearest(self.length);
        self.restore(furthest);
        let mut ran = 0;
        while ran < ticks && self.run(false) {
            ran += 1;
        }
        self.frontier = (!self.whole).then(|| self.state());
        let Viewer { session, renderer, other_hud, audio, view, rounds, .. } = self;
        put(&here, session, renderer, other_hud, audio, *view, rounds);
        self.level();
        self.whole
    }

    /// How many states the viewer keeps to seek from.
    pub fn kept(&self) -> usize {
        self.keyframes.len() + self.frontier.is_some() as usize + self.near.len()
    }

    /// The ticks seeks have played in silence so far: what seeking has
    /// cost, in ticks (for a host's debug line, and the tests).
    pub fn sought(&self) -> u64 {
        self.sought
    }

    // ---- The picture and the sound ---------------------------------------

    /// The battle's picture now, 240x160, with nothing over it.
    pub fn frame(&mut self) -> Frame {
        self.renderer.render(&self.session.battle)
    }

    /// The picture into the host's pixels, as [`crate::Player::present`]:
    /// `buffer` holds `width` x `height` of them, 0x00RRGGBB each; the
    /// frame is scaled up by the largest whole factor that fits, and the
    /// enhanced mode's text is drawn at the buffer's resolution.
    pub fn present(&mut self, buffer: &mut [u32], width: usize, height: usize) {
        let frame = self.frame();
        nettai_render::present::present(&frame, self.text.as_mut(), buffer, width, height);
    }

    /// The sound of the ticks played since the last call, appended to
    /// `out`: stereo samples at `nettai_audio::SAMPLE_RATE`, about 549 a
    /// tick at the original's speed. Nothing from a silent viewer, and
    /// nothing of the ticks a seek passed over.
    pub fn take_samples(&mut self, out: &mut Vec<[f32; 2]>) {
        out.append(&mut self.samples);
    }

    /// Whether the viewer has sound (a silent one gives no samples).
    pub fn has_sound(&self) -> bool {
        self.audio.is_some()
    }

    /// Show the battle in another language from the next frame, as
    /// [`crate::Player::set_language`].
    pub fn set_language(&mut self, graphics: &Graphics) {
        self.renderer.set_graphics(graphics.packs());
        self.renderer.set_strings(graphics.strings.clone());
    }

    /// The renderer (what it looked up and didn't find, its graphics mode).
    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    // ---- Inside --------------------------------------------------------------

    /// Where tick `tick` is, for the driver.
    fn place(&self, tick: u32) -> Place {
        Place { tick, round: self.round_at(tick), viewer: self.view }
    }

    /// One tick of the replay: the battle's, then both consoles' HUD
    /// states and sounds; with `heard`, the shown console's sound is kept
    /// for the host. The state it leaves is kept if it is one seeks go on
    /// from. False, and nothing runs, at the end or where the engine
    /// stopped.
    fn run(&mut self, heard: bool) -> bool {
        if self.at_end() || !self.session.step(0) {
            // (The engine stopping is the end of what there is to index.)
            self.whole |= self.session.stopped.is_some();
            return false;
        }
        // (A START a player pressed in a fight asks its host for the pause
        // again as the replay plays, `Battle::pause_requests`; nobody is
        // asked here: a viewer's pause is its own host's.)
        self.session.pause_requests = [false; 2];
        let view = self.view;
        if self.session.new_round {
            self.renderer.reset();
            self.other_hud = HudState::default();
        }
        self.renderer.observe(&self.session.battle);
        // (The other console's HUD follows the same battle, seen from its
        // side: the side shown is the setup's, which the simulation
        // doesn't read.)
        self.session.battle.setup.local_side = view ^ 1;
        self.other_hud.tick(&self.session.battle);
        self.session.battle.setup.local_side = view;
        if let Some(audio) = &mut self.audio {
            // (A tick that ended a round is heard from the round it ended:
            // the battle is the next round's by now.)
            let ticked = self.session.ended.as_deref().unwrap_or(&self.session.battle);
            for (side, sound) in audio.iter_mut().enumerate() {
                sound.handle(ticked.sound_cues_for(side as u8));
                self.scratch.clear();
                sound.tick(&mut self.scratch);
                if heard && side as u8 == view {
                    hear(&self.scratch, self.speed, &mut self.group, &mut self.samples);
                }
            }
        }
        let here = self.position();
        self.reached = self.reached.max(here);
        self.whole |= here >= self.length;
        let due = here % KEYFRAME_EVERY == 0 || self.rounds.binary_search(&here).is_ok();
        if due && self.keyframes.last().is_none_or(|last| last.tick < here) {
            self.keyframes.push(self.state());
            // (Where indexing left off is behind a kept state now.)
            if self.frontier.as_ref().is_some_and(|f| f.tick <= here) {
                self.frontier = None;
            }
        }
        true
    }

    /// Everything the viewer is now, to come back to.
    fn state(&self) -> State {
        let mut huds = [self.renderer.hud_state.clone(), self.other_hud.clone()];
        if self.view == 1 {
            huds.swap(0, 1);
        }
        State {
            tick: self.position(),
            battle: self.session.battle.save_state(),
            huds,
            audio: self.audio.clone(),
            stopped: self.session.stopped.clone(),
            finished: self.session.finished,
            result: self.session.result,
        }
    }

    /// The kept state nearest to `target` at or before it, and its tick.
    fn nearest(&self, target: u32) -> (Kept, u32) {
        let i = self.keyframes.partition_point(|k| k.tick <= target) - 1;
        let mut nearest = (Kept::Keyframe(i), self.keyframes[i].tick);
        if let Some(f) = self.frontier.as_ref().filter(|f| f.tick <= target && f.tick > nearest.1) {
            nearest = (Kept::Frontier, f.tick);
        }
        if let Some(n) = self.near.partition_point(|n| n.tick <= target).checked_sub(1).filter(|&n| self.near[n].tick > nearest.1) {
            nearest = (Kept::Near(n), self.near[n].tick);
        }
        nearest
    }

    /// Be as a kept state was.
    fn restore(&mut self, kept: Kept) {
        let Viewer { session, renderer, other_hud, audio, view, rounds, keyframes, frontier, near, .. } = self;
        let state = match kept {
            Kept::Keyframe(i) => &keyframes[i],
            Kept::Frontier => frontier.as_ref().expect("a frontier to go on from"),
            Kept::Near(i) => &near[i],
        };
        put(state, session, renderer, other_hud, audio, *view, rounds);
        self.level();
    }

    /// The host's levels on the sound as it stands (a kept state's sound has the levels it was kept with).
    fn level(&mut self) {
        let (music, effects) = self.levels;
        for sound in self.audio.iter_mut().flatten() {
            sound.set_levels(music, effects);
        }
    }

    /// The host's levels for the replay's sound, each 0 to 1 (1: as the
    /// game plays it): its music's and its effects', as
    /// [`crate::Player::set_sound_levels`]. They hold across seeks.
    pub fn set_sound_levels(&mut self, music: f32, effects: f32) {
        self.levels = (music, effects);
        self.level();
    }

    /// After a jump: the sound kept for the host is of somewhere else, and
    /// the clock starts again from here.
    fn settle(&mut self) {
        self.samples.clear();
        self.group.clear();
        self.clock.rest();
    }
}

/// Put `state` in a viewer's parts, shown from `view`'s side.
fn put(
    state: &State,
    session: &mut Session,
    renderer: &mut Renderer,
    other_hud: &mut HudState,
    audio: &mut Option<[BattleAudio; 2]>,
    view: u8,
    rounds: &[u32],
) {
    session.resume(state.battle.battle(), state.tick as u64, state.stopped.clone(), state.finished, state.result);
    session.battle.setup.local_side = view;
    let round = rounds.partition_point(|&start| start <= state.tick).max(1) as u32;
    session.driver.go_to(Place { tick: state.tick, round, viewer: view });
    renderer.hud_state.clone_from(&state.huds[view as usize]);
    other_hud.clone_from(&state.huds[view as usize ^ 1]);
    if let (Some(audio), Some(kept)) = (audio, &state.audio) {
        audio.clone_from(kept);
    }
}

/// A tick's samples as the host hears them at `speed`: as they are at the
/// original's; each twice at half of it; at double and four times, each
/// two or four as their mean (`group` holds those of a group not yet
/// whole). The host is not kept more than a second of them.
fn hear(tick: &[[f32; 2]], speed: Speed, group: &mut Vec<[f32; 2]>, out: &mut Vec<[f32; 2]>) {
    match speed {
        Speed::Normal => out.extend_from_slice(tick),
        Speed::Half => out.extend(tick.iter().flat_map(|&s| [s, s])),
        Speed::Double | Speed::Quadruple => {
            let n = if speed == Speed::Double { 2 } else { 4 };
            group.extend_from_slice(tick);
            let whole = group.len() / n * n;
            out.extend(group[..whole].chunks(n).map(|g| {
                let sum = g.iter().fold([0.0f32; 2], |a, s| [a[0] + s[0], a[1] + s[1]]);
                [sum[0] / n as f32, sum[1] / n as f32]
            }));
            group.drain(..whole);
        }
    }
    if out.len() > MOST_SAMPLES {
        let over = out.len() - MOST_SAMPLES;
        out.drain(..over);
    }
}

#[cfg(test)]
mod tests;
