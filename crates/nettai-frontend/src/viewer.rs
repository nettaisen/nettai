//! Watching a replay: [`Viewer`] plays one back for a host to show and
//! hear, as a [`crate::Player`] does, and has a transport besides. It knows
//! the replay's length and where each round starts as soon as it opens (the
//! file marks them), seeks to any tick or time, goes five seconds or a
//! round back or on, steps a frame either way, plays at four speeds, and
//! shows either console or both on one screen ([`Viewer::set_both`]),
//! switched while it plays.
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
//! to it, sound included (the two consoles' sound is half a tick's cost or
//! more); after that a seek plays at most [`KEYFRAME_EVERY`] - 1 ticks, and
//! one into the stretch last sought through fewer than [`NEAR_EVERY`].
//! [`Viewer::index`] plays ahead a slice at a time without moving, for a
//! host that wants every seek short before its user drags the bar. Or the
//! index is made elsewhere: a second viewer of the replay, on a thread of
//! its own, plays it through ([`Viewer::index_part`]) and hands over the
//! states it would keep, which the viewer watched takes in
//! ([`Viewer::take_index`]). Or on several threads, as fast as the slowest
//! of them: a silent viewer plays the battle on one
//! ([`Viewer::index_battle`]: the states without their sound, and each
//! tick's cues for each of the viewer's sounds, [`SOUNDS`]), each sound
//! plays its cues on one of its own ([`ConsoleSound`]) and puts itself into
//! the states, and the parts go from one to the next as they are made. The replay plays
//! the same anywhere, so they are the states the viewer makes itself, and
//! its own thread spends nothing on them. The figures measured on a game's
//! own packs are docs/frontend.md's (§8, "Seeking").
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
use nettai_audio::{BattleAudio, SoundCue};
use nettai_battle::content::Content;
use nettai_battle::{Battle, BattleResult, RoundEnd, Snapshot};
use nettai_render::hud::HudState;
use nettai_render::vfont::TextRenderer;
use nettai_render::{Consoles, Frame, Renderer};
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
    /// By sound ([`SOUNDS`]); none: a silent viewer.
    audio: Option<[BattleAudio; SOUNDS]>,
    /// The session's: why it stopped, if it had, whether that was the
    /// replay's end, and the set's result.
    stopped: Option<String>,
    finished: bool,
    result: Option<BattleResult>,
}

/// A part of a replay's index, made by one viewer of it for another to take
/// in ([`Viewer::index_part`], [`Viewer::take_index`]): the states kept to
/// seek from in the ticks it played, and how far it reached. It can go to
/// another thread (each state is plain data: the battle's snapshot, the
/// HUDs', the sound's over banks shared by `Arc`).
pub struct IndexPart {
    states: Vec<State>,
    reached: u32,
    whole: bool,
}

impl IndexPart {
    /// The tick the part reached: the viewer that made it was there.
    pub fn reached(&self) -> u32 {
        self.reached
    }

    /// The replay's end is in it, or where the engine stopped before that:
    /// the index is whole with it.
    pub fn whole(&self) -> bool {
        self.whole
    }
}

/// The battle's share of a part of a replay's index, made by a silent
/// viewer for each sound to be made elsewhere ([`Viewer::index_battle`]):
/// the states kept to seek from in the ticks it played, without their
/// sound, and each tick's sound cues for each of the viewer's sounds
/// ([`SOUNDS`]), which are all a sound is made of. Each sound plays the
/// part ([`ConsoleSound::play`]) and puts itself in at each state; then it
/// is the part a viewer that makes its own sound makes
/// ([`BattlePart::into_part`]).
pub struct BattlePart {
    part: IndexPart,
    /// The tick it starts at: the ticks a sound is to have played before
    /// it.
    from: u32,
    /// By sound: the cues of the part's ticks, all in a row.
    cues: [Vec<SoundCue>; SOUNDS],
    /// Where each tick's cues end in them, by sound.
    ends: Vec<[u32; SOUNDS]>,
    /// The part's ticks played before each state was kept.
    kept_after: Vec<u32>,
    /// By sound: the sound at each state, once it played the part.
    sounds: [Option<Vec<BattleAudio>>; SOUNDS],
}

impl BattlePart {
    /// The tick the part reached ([`IndexPart::reached`]).
    pub fn reached(&self) -> u32 {
        self.part.reached
    }

    /// The replay's end is in it ([`IndexPart::whole`]).
    pub fn whole(&self) -> bool {
        self.part.whole
    }

    /// The part of the index, its states with every sound once each played
    /// it: what a viewer making its own sound makes ([`Viewer::index_part`]).
    /// Where none did (a replay watched without sound), its states have
    /// none.
    pub fn into_part(self) -> IndexPart {
        let BattlePart { mut part, sounds, .. } = self;
        if sounds.iter().all(Option::is_none) {
            return part;
        }
        let mut sounds = sounds.map(|s| s.expect("every sound played the part of the index, or none").into_iter());
        for state in &mut part.states {
            state.audio = Some(sounds.each_mut().map(|s| s.next().expect("a sound at each state")));
        }
        part
    }
}

/// One of a viewer's sounds ([`SOUNDS`]: a console's, or a side's own
/// effects), made apart from the battle for a replay's index: it plays the
/// cues a silent viewer's parts carry for it ([`Viewer::index_battle`]),
/// each tick as a viewer's own sound plays them (the tick's cues, then the
/// frame, its samples made and not heard), and puts itself as it stands at
/// each state kept into the part. The sound takes nothing else from the
/// battle: a host has each played on a thread of its own beside the
/// battle's, and the parts it hands over are the ones a viewer that makes
/// its own sound makes.
pub struct ConsoleSound {
    sound: usize,
    audio: BattleAudio,
    /// The ticks it has played.
    at: u32,
    scratch: Vec<[f32; 2]>,
}

impl ConsoleSound {
    /// Each of a viewer's sounds at a replay's start, as a viewer opened on
    /// `game` has them ([`Viewer::open`]); none where the game's sound isn't
    /// loaded.
    pub fn of(game: &Loaded) -> Option<[ConsoleSound; SOUNDS]> {
        sound_of(game).map(ConsoleSound::all)
    }

    /// Each of a viewer's sounds at a replay's start, as a viewer given
    /// `audio` has them ([`Viewer::with`]).
    pub fn all(audio: BattleAudio) -> [ConsoleSound; SOUNDS] {
        std::array::from_fn(|sound| ConsoleSound { sound, audio: audio.clone(), at: 0, scratch: Vec::new() })
    }

    /// Play `part`'s ticks, each tick's cues for this sound and then its
    /// frame, and put the sound as it stands after each state's tick into
    /// the part. The parts in the order they were made, from the replay's
    /// start.
    pub fn play(&mut self, part: &mut BattlePart) {
        assert_eq!(part.from, self.at, "a sound plays the parts in order");
        let side = self.sound;
        let mut sounds = Vec::with_capacity(part.kept_after.len());
        let mut kept = part.kept_after.iter().copied().peekable();
        let mut start = 0;
        for (ran, ends) in (1..).zip(&part.ends) {
            let end = ends[side] as usize;
            self.audio.handle(&part.cues[side][start..end]);
            start = end;
            self.scratch.clear();
            self.audio.tick(&mut self.scratch);
            if kept.next_if_eq(&ran).is_some() {
                sounds.push(self.audio.clone());
            }
        }
        self.at += part.ends.len() as u32;
        part.sounds[side] = Some(sounds);
    }
}

// (A part is made on one thread and taken in on another; a console's sound
// is made on a thread of its own.)
const _: () = {
    const fn send<T: Send>() {}
    send::<IndexPart>();
    send::<BattlePart>();
    send::<ConsoleSound>();
};

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
    /// Each of its sounds ([`SOUNDS`]: each console's, each side's own
    /// effects; none: silent). All run every tick.
    audio: Option<[BattleAudio; SOUNDS]>,
    /// The host's levels for the sound, the music's and the effects'
    /// ([`Viewer::set_sound_levels`]): the viewer's, whatever state it is
    /// put back to.
    levels: (f32, f32),
    /// The side whose console is shown (on a screen of both, the left).
    view: u8,
    /// Both consoles on one screen ([`Viewer::set_both`]).
    both: bool,
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
    /// The sound of the ticks played since the host last took it: each
    /// console's, by side, then both consoles' on one screen's (`BOTH`).
    samples: [Vec<[f32; 2]>; 3],
    /// A tick's samples as each sound makes them (by sound), a tick's cues
    /// for one, and the samples of a group not yet whole at a speed above
    /// the original's (as `samples`).
    ticks: [Vec<[f32; 2]>; SOUNDS],
    cues: Vec<SoundCue>,
    group: [Vec<[f32; 2]>; 3],
}

/// The viewer's sounds, all running every tick so that what is heard after
/// a seek or a change of view is what was heard there the first time: each
/// console's, by side (0, 1); and each side's own effects, by side (2, 3):
/// the effects its console plays that the other console doesn't play that
/// tick ([`own_effects`]), on a sound of their own that plays nothing else
/// (its console's stop of all sound too). A screen of both consoles is
/// heard as the left console's sound with the right side's own effects
/// over it ([`Viewer::set_both`]: the user's "music follows the left
/// console", each effect once).
pub const SOUNDS: usize = 4;

/// Which of `samples` holds the sound of both consoles on one screen.
const BOTH: usize = 2;

/// The cues of the viewer's sound `sound` ([`SOUNDS`]) on the tick `b` ran,
/// appended to `out`.
fn sound_cues(b: &Battle, sound: usize, out: &mut Vec<SoundCue>) {
    match sound {
        0 | 1 => out.extend_from_slice(b.sound_cues_for(sound as u8)),
        _ => {
            let side = (sound - 2) as u8 & 1;
            own_effects(b.sound_cues_for(side), b.sound_cues_for(side ^ 1), out);
        }
    }
}

/// The effects of a console's cues `mine` that the other console's `theirs`
/// don't play (each of theirs taking one of mine), and its stops of all
/// sound, appended to `out`: what a side's own effects' sound plays.
fn own_effects(mine: &[SoundCue], theirs: &[SoundCue], out: &mut Vec<SoundCue>) {
    let mut taken = vec![false; theirs.len()];
    for &cue in mine {
        match cue {
            SoundCue::Effect(_) => match theirs.iter().enumerate().position(|(k, &c)| c == cue && !taken[k]) {
                Some(k) => taken[k] = true,
                None => out.push(cue),
            },
            SoundCue::StopMusic => out.push(cue),
            _ => {}
        }
    }
}

/// `a` and `b`'s samples together, a sample of each at a time.
fn mixed(a: &[[f32; 2]], b: &[[f32; 2]]) -> Vec<[f32; 2]> {
    a.iter().zip(b).map(|(a, b)| [(a[0] + b[0]).clamp(-1.0, 1.0), (a[1] + b[1]).clamp(-1.0, 1.0)]).collect()
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
        Viewer::with(&game.game.content, replay, view, game.renderer(), game.font.clone().map(TextRenderer::new), sound_of(game))
    }

    /// A viewer from its parts: the content the replay plays on, the
    /// renderer, the enhanced mode's text renderer (none: the frames carry
    /// their own text) and the sound (none: silent; it is copied for the
    /// other console). The other games' backgrounds its rounds are drawn on
    /// are extracted first, if they aren't (`GameGraphics::wait_backgrounds`:
    /// a page's main thread has them from a [`crate::preload::Preload`]).
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
        // (The other games' backgrounds its rounds are drawn on, extracted now: a frame shows them from the first.
        // On a page's main thread they are a preload's, made ready already.)
        renderer.graphics().wait_backgrounds(content, nettai_render::graphics::set_backgrounds(&session.battle.setup));
        renderer.reset();
        renderer.console_version = session.driver.console_version();
        let mut viewer = Viewer {
            session,
            renderer,
            text,
            audio: audio.map(|a| std::array::from_fn(|_| a.clone())),
            levels: (1.0, 1.0),
            view,
            both: false,
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
            samples: Default::default(),
            ticks: Default::default(),
            cues: Vec::new(),
            group: Default::default(),
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
    /// it was, and whether its setups were open. Where they weren't
    /// (`Info::open_setups`), the host shows neither side's build: the
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
        self.group.iter_mut().for_each(Vec::clear);
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
    /// sound, each as a viewer shown from that side all along has them (on
    /// a screen of both, `side`'s on the left). False for a side a match
    /// doesn't have.
    pub fn set_view(&mut self, side: u8) -> bool {
        if side > 1 {
            return false;
        }
        if side != self.view {
            self.view = side;
            swap_huds(&mut self.renderer);
            self.session.battle.setup.local_side = side;
            let here = self.place(self.position());
            self.session.driver.go_to(here);
            self.settle();
        }
        true
    }

    /// Whether both consoles are shown on one screen.
    pub fn both(&self) -> bool {
        self.both
    }

    /// Show both consoles on one screen from now on (`nettai_render::Consoles::Both`), the side shown's on the left,
    /// or (false) its console alone. On a screen of both the sound heard is both consoles' (`take_samples`): the
    /// left console's, its music and all, with the right side's own effects over it ([`SOUNDS`]), as a viewer shown
    /// so all along has it: every sound runs every tick.
    pub fn set_both(&mut self, both: bool) {
        if both != self.both {
            self.both = both;
            self.renderer.consoles = if both { Consoles::Both } else { Consoles::One };
            self.settle();
        }
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
    /// behind a loading screen) until then, or has the index made on a
    /// thread of its own ([`Viewer::index_part`]); one that does neither
    /// pays for the ticks between on the first seek past
    /// [`Viewer::indexed`].
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
        let Viewer { session, renderer, audio, view, rounds, .. } = self;
        put(&here, session, renderer, audio, *view, rounds);
        self.level();
        self.whole
    }

    /// Play up to `ticks` ticks on from where the viewer is, for another
    /// viewer of the same replay to take in ([`Viewer::take_index`]): as
    /// [`Viewer::index`] plays them, the sound running and not heard (so
    /// each state holds it), but the states this viewer would keep go in
    /// the part and not into its own keeping, and it is left where the
    /// ticks took it. A host indexes on a thread of its own so: a second
    /// viewer of the replay, opened there at its start, makes the parts
    /// one after another and sends each as it is made. The part is whole
    /// ([`IndexPart::whole`]) once the replay's end is in it. (Or the
    /// sound is made elsewhere, each console's beside the battle:
    /// [`Viewer::index_battle`].)
    pub fn index_part(&mut self, ticks: u32) -> IndexPart {
        self.index_ticks(ticks, |_, _, _| {})
    }

    /// Play up to `ticks` ticks on from where the viewer is, as
    /// [`Viewer::index_part`] does, for each console's sound to be made
    /// elsewhere ([`ConsoleSound`]): a silent viewer's (one given no sound,
    /// [`Viewer::with`]) is the battle's share of the work. The part holds
    /// the states without their sound, and each tick's sound cues for each
    /// console (what a viewer's own sound would be given: the cues of the
    /// round that ended, on a tick that ended one). A host indexes on three
    /// threads so: this viewer on one, each console's sound on one of its
    /// own, the parts going from one to the next as they are made.
    pub fn index_battle(&mut self, ticks: u32) -> BattlePart {
        let from = self.position();
        let (mut cues, mut ends, mut kept_after) = (<[Vec<SoundCue>; SOUNDS]>::default(), Vec::new(), Vec::new());
        let part = self.index_ticks(ticks, |ticked, ran, kept| {
            for (sound, cues) in cues.iter_mut().enumerate() {
                sound_cues(ticked, sound, cues);
            }
            ends.push(cues.each_ref().map(|c| c.len() as u32));
            if kept {
                kept_after.push(ran);
            }
        });
        BattlePart { part, from, cues, ends, kept_after, sounds: Default::default() }
    }

    /// Play up to `ticks` ticks on from where the viewer is, the states it
    /// would keep going into a part and not into its keeping; after each
    /// tick `each` is told the battle that ticked, the ticks played and
    /// whether the state was kept.
    fn index_ticks(&mut self, ticks: u32, mut each: impl FnMut(&Battle, u32, bool)) -> IndexPart {
        let mut states = Vec::new();
        let mut ran = 0;
        while ran < ticks && self.play(false) {
            ran += 1;
            let kept = self.due(self.position());
            if kept {
                states.push(self.state());
            }
            each(ticked(&self.session), ran, kept);
        }
        // (Where the engine stopped, the tick it stopped on is past what was reached.)
        IndexPart { states, reached: self.reached.min(self.position()), whole: self.whole }
    }

    /// Take in a part of the index another viewer of the same replay made
    /// ([`Viewer::index_part`]; the parts in the order they were made, from
    /// the replay's start): its states are kept as this viewer's own (one
    /// of a tick it keeps already is dropped), and [`Viewer::indexed`] goes
    /// on to where the part reached. The replay plays the same on either
    /// viewer, so a seek from a state taken in lands exactly as from one
    /// this viewer made, its sound and all; the other viewer is to have
    /// sound where this one has it.
    pub fn take_index(&mut self, part: IndexPart) {
        for state in part.states {
            if let Err(at) = self.keyframes.binary_search_by_key(&state.tick, |k| k.tick) {
                self.keyframes.insert(at, state);
            }
        }
        // (Where indexing here left off is behind a kept state now.)
        if let (Some(f), Some(last)) = (&self.frontier, self.keyframes.last())
            && f.tick <= last.tick
        {
            self.frontier = None;
        }
        self.reached = self.reached.max(part.reached);
        self.whole |= part.whole;
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

    /// The battle's picture now, with nothing over it: 160 rows of the renderer's columns (`Renderer::frame_wide`).
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

    /// The picture `side`'s console shows now, into the host's pixels as [`Viewer::present`]: the console shown's
    /// own (`present`) or the other's, drawn from the same tick with that side's HUD state (both run every
    /// tick), so a host can show both consoles at once (a video's two sides; an inset of the other) without
    /// playing the replay twice. Nothing of what is shown or heard changes: the side shown stays the viewer's.
    pub fn present_view(&mut self, side: u8, buffer: &mut [u32], width: usize, height: usize) {
        let frame = self.frame_view(side);
        nettai_render::present::present(&frame, self.text.as_mut(), buffer, width, height);
    }

    /// The picture `side`'s console shows now, 240x160, with nothing over it: its own screen, whether or not the
    /// viewer shows both on one ([`Viewer::frame`] for the side shown on its own, the other's as
    /// [`Viewer::present_view`] draws it).
    pub fn frame_view(&mut self, side: u8) -> Frame {
        let consoles = std::mem::replace(&mut self.renderer.consoles, Consoles::One);
        let frame = if side == self.view || side > 1 {
            self.renderer.render(&self.session.battle)
        } else {
            // (The renderer follows nothing else by side: its stage clock and its graphics are both consoles'.)
            swap_huds(&mut self.renderer);
            self.session.battle.setup.local_side = side;
            let frame = self.renderer.render(&self.session.battle);
            self.session.battle.setup.local_side = self.view;
            swap_huds(&mut self.renderer);
            frame
        };
        self.renderer.consoles = consoles;
        frame
    }

    /// A text renderer like the viewer's own, its font with nothing made yet (none where the frames carry their own
    /// text): for a thread that presents the viewer's frames beside it ([`nettai_render::present::present`]), as
    /// a video's are, drawing them as the viewer would.
    pub fn text_renderer(&self) -> Option<TextRenderer> {
        self.text.as_ref().map(TextRenderer::like)
    }

    /// The sound of the ticks played since the last call, appended to
    /// `out`: stereo samples at `nettai_audio::SAMPLE_RATE`, about 549 a
    /// tick at the original's speed: the console shown's, or on a screen of
    /// both both consoles' ([`Viewer::set_both`]). Nothing from a silent
    /// viewer, and nothing of the ticks a seek passed over.
    pub fn take_samples(&mut self, out: &mut Vec<[f32; 2]>) {
        let shown = if self.both { BOTH } else { self.view as usize };
        out.append(&mut self.samples[shown]);
    }

    /// The sound `side`'s console played in the ticks played since the
    /// last call for that side, appended to `out` as [`Viewer::take_samples`]
    /// gives the shown one's (which this is, for the side shown): both
    /// consoles' sounds run every tick, so a host can have both at once (a
    /// video's two sides, each its own track) without playing the replay
    /// twice. A host that takes only the shown side's leaves the other's
    /// kept to a second, as it keeps the shown one's.
    pub fn take_samples_of(&mut self, side: u8, out: &mut Vec<[f32; 2]>) {
        if let Some(samples) = self.samples.get_mut(side as usize) {
            out.append(samples);
        }
    }

    /// Whether the viewer has sound (a silent one gives no samples).
    pub fn has_sound(&self) -> bool {
        self.audio.is_some()
    }

    /// Show the battle in another language from the next frame, as
    /// [`crate::Player::set_language`].
    pub fn set_language(&mut self, graphics: &Graphics) {
        self.renderer.set_graphics(graphics.game_graphics());
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

    /// One tick of the replay ([`Viewer::play`]), the state it leaves kept
    /// if it is one seeks go on from. False, and nothing runs, at the end
    /// or where the engine stopped.
    fn run(&mut self, heard: bool) -> bool {
        if !self.play(heard) {
            return false;
        }
        let here = self.position();
        if self.due(here) && self.keyframes.last().is_none_or(|last| last.tick < here) {
            self.keyframes.push(self.state());
            // (Where indexing left off is behind a kept state now.)
            if self.frontier.as_ref().is_some_and(|f| f.tick <= here) {
                self.frontier = None;
            }
        }
        true
    }

    /// Whether the state at `tick` is one kept to seek from: every
    /// [`KEYFRAME_EVERY`]th tick's, and each round's first's.
    fn due(&self, tick: u32) -> bool {
        tick % KEYFRAME_EVERY == 0 || self.rounds.binary_search(&tick).is_ok()
    }

    /// One tick of the replay: the battle's, then both consoles' HUD
    /// states and every sound ([`SOUNDS`]); with `heard`, each console's
    /// sound is kept for the host, and on a screen of both, both consoles'
    /// (the left's and the right side's own effects). Nothing of it is kept
    /// to seek from. False, and nothing runs, at the end or where the
    /// engine stopped.
    fn play(&mut self, heard: bool) -> bool {
        if self.at_end() || !self.session.step(0) {
            // (The engine stopping is the end of what there is to index.)
            self.whole |= self.session.stopped.is_some();
            return false;
        }
        // (A START a player pressed in a fight asks its host for the pause
        // again as the replay plays, `Battle::pause_requests`; nobody is
        // asked here: a viewer's pause is its own host's.)
        self.session.pause_requests = [false; 2];
        let speed = self.speed;
        if self.session.new_round {
            self.renderer.reset();
        }
        // (Both consoles' HUDs: the renderer follows each.)
        self.renderer.observe(&self.session.battle);
        if let Some(audio) = &mut self.audio {
            let ticked = ticked(&self.session);
            for (k, sound) in audio.iter_mut().enumerate() {
                self.cues.clear();
                sound_cues(ticked, k, &mut self.cues);
                sound.handle(&self.cues);
                self.ticks[k].clear();
                sound.tick(&mut self.ticks[k]);
            }
            if heard {
                for side in 0..2 {
                    hear(&self.ticks[side], speed, &mut self.group[side], &mut self.samples[side]);
                }
                if self.both {
                    let left = self.view as usize & 1;
                    let both = mixed(&self.ticks[left], &self.ticks[2 + (left ^ 1)]);
                    hear(&both, speed, &mut self.group[BOTH], &mut self.samples[BOTH]);
                }
            }
        }
        let here = self.position();
        self.reached = self.reached.max(here);
        self.whole |= here >= self.length;
        true
    }

    /// Everything the viewer is now, to come back to.
    fn state(&self) -> State {
        let mut huds = [self.renderer.hud_state.clone(), self.renderer.other_hud.clone()];
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
        let Viewer { session, renderer, audio, view, rounds, keyframes, frontier, near, .. } = self;
        let state = match kept {
            Kept::Keyframe(i) => &keyframes[i],
            Kept::Frontier => frontier.as_ref().expect("a frontier to go on from"),
            Kept::Near(i) => &near[i],
        };
        put(state, session, renderer, audio, *view, rounds);
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
        self.samples.iter_mut().chain(&mut self.group).for_each(Vec::clear);
        self.clock.rest();
    }
}

/// A viewer's sound as it opens on `game` (none: the game's sound isn't
/// loaded): one console's, copied for the other.
fn sound_of(game: &Loaded) -> Option<BattleAudio> {
    game.sound.as_ref().map(|s| BattleAudio::new(s.bank.clone(), s.songs.clone()))
}

/// The battle the last tick ran, whose sound cues are that tick's: a tick
/// that ended a round is heard from the round it ended (the session's
/// battle is the next round's by then).
fn ticked(session: &Session) -> &Battle {
    session.ended.as_deref().unwrap_or(&session.battle)
}

/// Put `state` in a viewer's parts, shown from `view`'s side.
fn put(
    state: &State,
    session: &mut Session,
    renderer: &mut Renderer,
    audio: &mut Option<[BattleAudio; SOUNDS]>,
    view: u8,
    rounds: &[u32],
) {
    session.resume(state.battle.battle(), state.tick as u64, state.stopped.clone(), state.finished, state.result);
    session.battle.setup.local_side = view;
    let round = rounds.partition_point(|&start| start <= state.tick).max(1) as u32;
    session.driver.go_to(Place { tick: state.tick, round, viewer: view });
    renderer.hud_state.clone_from(&state.huds[view as usize]);
    renderer.other_hud.clone_from(&state.huds[view as usize ^ 1]);
    if let (Some(audio), Some(kept)) = (audio, &state.audio) {
        audio.clone_from(kept);
    }
}

/// The renderer's two consoles' HUD states the other way round: the other
/// console's is shown from now.
fn swap_huds(renderer: &mut Renderer) {
    std::mem::swap(&mut renderer.hud_state, &mut renderer.other_hud);
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
