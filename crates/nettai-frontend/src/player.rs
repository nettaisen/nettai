//! A battle being played, for a host to show and hear: [`Player`] owns the
//! session, the renderer, the text renderer (the enhanced mode's) and the
//! battle's audio, and the host drives it.
//!
//! The host gives it the GBA buttons held (its own keys' business) and
//! either the time that passed ([`Player::advance`]: the player keeps the
//! original's 59.7275 Hz clock and runs the ticks due) or asks for a tick
//! outright ([`Player::tick`]: a host that paces itself). It takes back
//! the picture ([`Player::frame`], or [`Player::present`] into a pixel
//! buffer of any size) and the sound as samples for its own output
//! ([`Player::take_samples`]). The player draws nothing over the battle's
//! picture and composes no text: where playback is, whether it is paused
//! and how fast, why it stopped, a set's result, a difference from a
//! recording and a netplay connection's figures are values
//! ([`Player::position`], [`Player::stopped`], [`Player::result`],
//! [`Player::diverged`], [`Player::net_status`]) for the host to show as it
//! likes. Pause, speed and starting over are methods, refused while the
//! battle runs in real time with another player.
//!
//! ```ignore
//! let game = nettai_frontend::game::load("exe6", &Options::default())?;
//! let m = nettai_match::pick::live(&game.game.content, "exe6", seed, None)?;
//! let driver = LivePlayer::new(nettai_match::Set::of(&game.game.content, &m, seed));
//! let mut player = Player::new(&game, Box::new(driver));
//! let mut last = Instant::now();
//! while window.is_open() {
//!     let now = Instant::now();
//!     player.advance(now - last, window.gba_buttons());
//!     last = now;
//!     player.present(window.pixels(), window.width(), window.height());
//!     player.take_samples(&mut samples);
//!     speaker.queue(&samples);
//! }
//! ```

use crate::driver::{Driver, LivePlayer, NetStatus, Opponent, StopReason, Training};
use crate::game::{Graphics, Loaded};
use crate::preload::Late;
use crate::session::Session;
use nettai_audio::BattleAudio;
use nettai_battle::{Battle, BattleResult};
use nettai_render::vfont::TextRenderer;
use nettai_render::{Frame, Renderer};
use std::time::Duration;

/// The original's frame rate: a tick is one frame of it.
pub const FRAME_RATE: f64 = 59.7275;

/// A tick's length in seconds, exactly, as a fraction (numerator,
/// denominator): 280,896 cycles of the GBA's 16,777,216 Hz clock, which is
/// 4389/262144 s. [`FRAME_RATE`] is its inverse to four places. The sound
/// is 4389/8 samples a tick at `nettai_audio::SAMPLE_RATE` by the same
/// clock, so a video's frames and its sound keep step by these numbers.
pub const TICK_SECONDS: (u32, u32) = (4389, 262_144);

/// The time `ticks` ticks take (down to the nanosecond).
pub fn time_of(ticks: u64) -> Duration {
    let (n, d) = (TICK_SECONDS.0 as u128, TICK_SECONDS.1 as u128);
    let nanos = ticks as u128 * n * 1_000_000_000 / d;
    Duration::new((nanos / 1_000_000_000) as u64, (nanos % 1_000_000_000) as u32)
}

/// The tick nearest to `time` after a start: how many ticks it is in.
pub fn tick_at(time: Duration) -> u64 {
    let (n, d) = (TICK_SECONDS.0 as u128, TICK_SECONDS.1 as u128);
    let one = n * 1_000_000_000;
    ((time.as_nanos() * d + one / 2) / one) as u64
}

/// The speeds a player runs at, as multiples of [`FRAME_RATE`]:
/// [`Player::slower`] and [`Player::faster`] step through them.
pub const SPEEDS: [f64; 8] = [0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0];

/// The original's own speed among [`SPEEDS`].
const NORMAL: usize = 3;

/// The most time one [`Player::advance`] runs ticks for: a host that
/// stalled (a dragged window) doesn't make the battle catch up.
const MOST_ELAPSED: Duration = Duration::from_millis(250);

/// A display with frames to spare: its frames come this many ticks apart
/// or less, on average. On one, a late frame's ticks are spread over the
/// frames after it ([`Player::advance`]).
const SPARE_FRAMES: f64 = 0.95;

/// The most sound kept for a host that hasn't taken it: about a second.
pub(crate) const MOST_SAMPLES: usize = nettai_audio::SAMPLE_RATE as usize;

/// The clock a host's frames run ticks by: the ticks it owes for the time
/// that passed, at a rate, and the part of a tick left over
/// ([`Player::advance`], the replay viewer's).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Clock {
    /// Ticks the clock owes: less than one, or less than two on a display
    /// with frames to spare after a late frame.
    owed: f64,
    /// The time between the host's frames, averaged (seconds).
    frame_time: f64,
}

impl Clock {
    pub(crate) fn new() -> Clock {
        Clock { owed: 0.0, frame_time: 1.0 / FRAME_RATE }
    }

    /// Nothing owed: the clock starts again from now (a pause, a stop, a
    /// start over).
    pub(crate) fn rest(&mut self) {
        self.owed = 0.0;
    }

    /// The ticks due after `elapsed` more time at `speed` times the
    /// original's rate: at most [`MOST_ELAPSED`]'s, the part of a tick left
    /// over kept; on a display with frames to spare, a late frame that owes
    /// two keeps one for the next frame.
    pub(crate) fn due(&mut self, elapsed: Duration, speed: f64) -> u32 {
        let elapsed = elapsed.min(MOST_ELAPSED).as_secs_f64();
        self.frame_time += (elapsed - self.frame_time) / 16.0;
        let rate = FRAME_RATE * speed;
        self.owed += elapsed * rate;
        let mut due = self.owed.floor() as u32;
        if due >= 2 && self.frame_time * rate <= SPARE_FRAMES {
            due -= 1;
        }
        self.owed -= due as f64;
        due
    }
}

/// A battle being played. It owns everything it needs (the graphics it
/// draws from are shared with whoever loaded them), so a host keeps it
/// wherever it keeps its state.
pub struct Player {
    session: Session,
    renderer: Renderer,
    text: Option<TextRenderer>,
    audio: Option<BattleAudio>,
    /// The sound of the ticks run since the host last took it.
    samples: Vec<[f32; 2]>,
    paused: bool,
    /// Held where it stands, its sound going on ([`Player::set_held`]).
    held: bool,
    speed: usize,
    clock: Clock,
    /// What had been made before the player was: this thread's runtimes
    /// loaded and parts read, and its text renderers' strings and glyphs
    /// ([`Player::late`] counts from them).
    before: Late,
}

/// What this thread and these text renderers have made so far, as
/// [`Late`] counts it.
fn made_so_far(renderer: &Renderer, text: Option<&TextRenderer>) -> Late {
    let made = text.map(TextRenderer::made).unwrap_or_default();
    Late {
        runtimes: nettai_battle::behavior::Behaviors::loads(),
        reads: nettai_content::pack::reads(),
        layouts: made.layouts + renderer.layouts_made(),
        glyphs: made.glyphs,
    }
}

// (A player borrows nothing: a host's state of any lifetime holds one.)
const _: () = {
    const fn owned<T: 'static>() {}
    owned::<Player>();
};

impl Player {
    /// A player of what `driver` plays, shown and heard from a loaded game
    /// (its sound, if it was loaded).
    pub fn new(game: &Loaded, driver: Box<dyn Driver>) -> Player {
        let audio = game.sound.as_ref().map(|s| BattleAudio::with_banks(s.banks.clone(), s.songs.clone()));
        Player::with(game.renderer(), game.font.clone().map(TextRenderer::new), audio, driver)
    }

    /// A player of what `driver` plays, from its parts: the renderer, the
    /// enhanced mode's text renderer (none: the frames carry their own text)
    /// and the audio (none: silent).
    pub fn with(renderer: Renderer, text: Option<TextRenderer>, audio: Option<BattleAudio>, driver: Box<dyn Driver>) -> Player {
        // (Counted from before the driver makes its first battle: a thread
        // without the content's runtime loads it there.)
        let before = made_so_far(&renderer, text.as_ref());
        let mut player = Player {
            session: Session::new(driver),
            renderer,
            text,
            audio,
            samples: Vec::new(),
            paused: false,
            held: false,
            speed: NORMAL,
            clock: Clock::new(),
            before,
        };
        player.start_over();
        player
    }

    /// What the player made on the way since it was made, because nothing
    /// had made it ready (`crate::preload`): content runtimes loaded on
    /// this thread, parts of a pack read from disk on it, and its text's
    /// strings laid out and glyphs drawn for the first time. A player a
    /// [`crate::preload::Preload`] made, on the thread that stepped it,
    /// has loaded no runtime and read nothing however long it plays; the
    /// text's counts go up as new strings show (a chip's name, a number),
    /// each a part of a millisecond. For a host's debug line or its log,
    /// and for the tests that hold the preload to it.
    pub fn late(&self) -> Late {
        let now = made_so_far(&self.renderer, self.text.as_ref());
        Late {
            runtimes: now.runtimes - self.before.runtimes,
            reads: now.reads - self.before.reads,
            layouts: now.layouts.saturating_sub(self.before.layouts),
            glyphs: now.glyphs.saturating_sub(self.before.glyphs),
        }
    }

    /// Record the set as a replay from here (`crate::replay`): refused
    /// once a tick has run, and for a battle that isn't a match's set
    /// played on buttons alone (a trace's, a replay's). Live play records
    /// each tick as it runs, netplay each as it settles (both players'
    /// replays the same but for their info). Starting over stops the
    /// recording.
    pub fn record(&mut self, recorder: crate::replay::Recorder) -> Result<(), String> {
        self.session.record(recorder)
    }

    /// Why the recording stopped being written, if it has (a full disk).
    pub fn recording_failed(&self) -> Option<&str> {
        self.session.recorder.as_ref().and_then(|r| r.failed())
    }

    /// Go on with another driver (a recording's next round), in the same
    /// picture and sound.
    pub fn play(&mut self, driver: Box<dyn Driver>) {
        self.session = Session::new(driver);
        self.start_over();
    }

    /// The presentation at a session's start: nothing remembered, the
    /// console the driver's.
    fn start_over(&mut self) {
        self.renderer.reset();
        self.renderer.console_version = self.session.driver.console_version();
        self.clock.rest();
    }

    // ---- Running ----------------------------------------------------------

    /// Run the ticks due after `elapsed` more time, with `buttons` held (the
    /// GBA's button mask, `nettai_battle::input::keys`): `elapsed` at the
    /// original's rate times the speed, the part of a tick left over kept
    /// for the next call. None while paused or stopped. The ticks run.
    ///
    /// A host calls it once a display frame. A frame that comes late owes
    /// two ticks, of which only the second's picture would be shown: on a
    /// display with frames to spare (faster than the ticks, on average), it
    /// keeps one of what it owes for the next frame, so each tick has its
    /// picture (one tick later, until the frames to spare make it up).
    pub fn advance(&mut self, elapsed: Duration, buttons: u16) -> u32 {
        if self.paused || self.session.stopped.is_some() {
            self.clock.rest();
            return 0;
        }
        let due = self.clock.due(elapsed, SPEEDS[self.speed]);
        (0..due).take_while(|_| self.tick(buttons)).count() as u32
    }

    /// Run one tick with `buttons` held, whatever the clock or the pause
    /// say: a host that paces itself, or a frame's step while paused. (A
    /// driver that runs the battle itself, netplay's, runs one wall-clock
    /// frame, which may wait.) False, with [`Player::stopped`] saying why,
    /// when the battle can't go on.
    pub fn tick(&mut self, buttons: u16) -> bool {
        if self.held && self.session.stopped.is_none() {
            // (Held: the sound of a tick, and nothing of the battle's.)
            if let Some(audio) = &mut self.audio {
                audio.tick(&mut self.samples);
                if self.samples.len() > MOST_SAMPLES {
                    let over = self.samples.len() - MOST_SAMPLES;
                    self.samples.drain(..over);
                }
            }
            return true;
        }
        if !self.session.step(buttons) {
            return false;
        }
        if self.session.new_round {
            self.renderer.reset();
        }
        if self.session.fresh {
            self.renderer.observe(&self.session.battle);
        }
        if let Some(audio) = &mut self.audio {
            match &self.session.sound {
                // What a driver that runs the battle itself made of the
                // frame (netplay: plays, and cancels of cues played on a
                // wrong prediction), or a round's last cues.
                Some(actions) => audio.handle_actions(actions.iter().copied()),
                None => audio.handle(crate::session::heard(&self.session.battle)),
            }
            audio.tick(&mut self.samples);
            if self.samples.len() > MOST_SAMPLES {
                let over = self.samples.len() - MOST_SAMPLES;
                self.samples.drain(..over);
            }
        }
        true
    }

    /// Whether the battle runs in real time with another player (netplay):
    /// it can't be paused, run at another speed or started over.
    pub fn real_time(&self) -> bool {
        self.session.driver.real_time()
    }

    /// Pause or go on. False (and nothing changes) in real time.
    pub fn set_paused(&mut self, paused: bool) -> bool {
        if self.real_time() {
            return false;
        }
        self.paused = paused;
        true
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Hold the battle where it stands, or let it go on (a host's menu over a battle, where the game's own pause
    /// can't hold it: its custom screen, a dimming, a round's end). Held, a tick ([`Player::tick`],
    /// [`Player::advance`]) steps nothing of the battle (no state, no cue, no new picture: its ticks don't
    /// count), but its sound goes on: the music and what was sounding play on as the ticks due would have them,
    /// no cue new. Unlike a pause, the clock runs, for the sound's sake. False (and nothing changes) in real time.
    pub fn set_held(&mut self, held: bool) -> bool {
        if self.real_time() {
            return false;
        }
        self.held = held;
        true
    }

    pub fn held(&self) -> bool {
        self.held
    }

    /// The next slower of [`SPEEDS`]. False in real time.
    pub fn slower(&mut self) -> bool {
        if self.real_time() {
            return false;
        }
        self.speed = self.speed.saturating_sub(1);
        true
    }

    /// The next faster of [`SPEEDS`]. False in real time.
    pub fn faster(&mut self) -> bool {
        if self.real_time() {
            return false;
        }
        self.speed = (self.speed + 1).min(SPEEDS.len() - 1);
        true
    }

    /// The speed, as a multiple of the original's.
    pub fn speed(&self) -> f64 {
        SPEEDS[self.speed]
    }

    /// The players who pressed START in the fight since the last call, by
    /// side, each where the game's own pause was: a match's pause is its
    /// host's (`nettai_match::HOST_KEYS`), so the battle went on and this is
    /// the host's cue to open its own menu, holding the ticks meanwhile
    /// where it may ([`Player::set_paused`]: live play and replays, never in
    /// real time). START anywhere else (the custom screen, a window, a
    /// chatbox) is the game's and isn't said. In a netplay match only the
    /// local player's is said, on the tick as it was first simulated. None
    /// from a recording of the original, whose battle pauses itself.
    pub fn take_pause_requests(&mut self) -> [bool; 2] {
        std::mem::take(&mut self.session.pause_requests)
    }

    /// What the navi the local player fights does, in live play; none for a
    /// battle whose other navi is somebody's (netplay, a replay, a trace).
    pub fn opponent(&self) -> Option<Opponent> {
        self.session.driver.opponent()
    }

    /// Have it do `opponent` from the next tick on: either choice changed
    /// alone, while the set runs. False (and nothing changes) for a battle
    /// that plays no opponent.
    pub fn set_opponent(&mut self, opponent: Opponent) -> bool {
        self.session.driver.set_opponent(opponent)
    }

    /// A training session's options on live play (none for a battle that
    /// isn't live play).
    pub fn training(&self) -> Option<Training> {
        self.session.driver.live().map(LivePlayer::training)
    }

    /// Have a training session's options change the battle, at once and
    /// before every tick after (`LivePlayer::set_training`). False (and
    /// nothing changes) for a battle that isn't live play, and while a
    /// replay of it is written: what the options change isn't a match's.
    pub fn set_training(&mut self, training: Training) -> bool {
        let session = &mut self.session;
        let Some(live) = session.driver.live_mut().filter(|_| session.recorder.is_none()) else { return false };
        live.set_training(training, &mut session.battle);
        true
    }

    /// Live play's set on other backgrounds (a training session's
    /// Background changed): `set` is the one it plays from now, the same
    /// as the one playing but for its rounds' backgrounds. The battle goes
    /// on as it is, drawn on the background `set` has for the round being
    /// played, and the rounds after (and the set's next start) are on
    /// theirs. A round's background is in its setup, and the digest with
    /// it, but no tick reads it: the renderer alone does, so the battle
    /// steps as it would have. False (and nothing changes) for a battle
    /// that isn't live play, and while a recording is made.
    pub fn set_backgrounds(&mut self, set: nettai_match::Set) -> bool {
        let session = &mut self.session;
        let Some(live) = session.driver.live_mut().filter(|_| session.recorder.is_none()) else { return false };
        let (first, setup) = (set.first(), &mut session.battle.setup);
        // (The round's place among the set's: the rounds before it, as its score counts them.)
        setup.settings.background = match setup.score.round {
            0 => first.settings.background,
            n => first.later_stages.get(n as usize - 1).map_or(setup.settings.background, |s| s.background),
        };
        for (now, then) in setup.later_stages.iter_mut().zip(&first.later_stages) {
            now.background = then.background;
        }
        live.play_set(set);
        true
    }

    /// Live play's driver, for the rest a training session sets on it (the
    /// opponent's recording, its second pad); none for a battle that isn't
    /// live play.
    pub fn live(&self) -> Option<&LivePlayer> {
        self.session.driver.live()
    }

    pub fn live_mut(&mut self) -> Option<&mut LivePlayer> {
        self.session.driver.live_mut()
    }

    /// Have the navi the local player fights press nothing from the next
    /// tick on (a host's menu over a battle that steps on), or press as it
    /// chose again: its choices and random presses go on as they would,
    /// only what reaches the battle is held. False for a battle that plays
    /// no opponent.
    pub fn hold_opponent(&mut self, held: bool) -> bool {
        self.session.driver.hold_opponent(held)
    }

    /// Start over: what the driver plays from its beginning (a recording's
    /// round, live play's set from its first round). False in real time.
    pub fn restart(&mut self) -> bool {
        if self.real_time() {
            return false;
        }
        self.session.restart();
        self.renderer.reset();
        self.clock.rest();
        true
    }

    // ---- The picture and the sound ---------------------------------------

    /// The battle's picture now, 240x160, with nothing over it.
    pub fn frame(&mut self) -> Frame {
        self.renderer.render(&self.session.battle)
    }

    /// The picture into the host's pixels: `buffer` holds `width` x
    /// `height` of them, 0x00RRGGBB each, row by row. The frame is scaled
    /// up by the largest whole factor that fits and centered, and the enhanced
    /// mode's text (the game's own) is drawn at the buffer's resolution.
    /// Nothing else is drawn.
    pub fn present(&mut self, buffer: &mut [u32], width: usize, height: usize) {
        let frame = self.frame();
        nettai_render::present::present(&frame, self.text.as_mut(), buffer, width, height);
    }

    /// The host's levels for the battle's sound, each 0 to 1 (1: as the
    /// game plays it): its music's and its effects'
    /// (`BattleAudio::set_levels`: the sound driver mixes them, so a
    /// host can't turn one down in the samples it is given).
    pub fn set_sound_levels(&mut self, music: f32, effects: f32) {
        if let Some(audio) = &mut self.audio {
            audio.set_levels(music, effects);
        }
    }

    /// Play the battle's sounds by `parts` from now on (`Sound::parts`: the
    /// game's own, or a battle theme in place of its music), as a host
    /// changes the battle's music while it runs: the music playing now, if it
    /// is the battle's own, starts again as the new song at once
    /// (`BattleAudio::set_songs`). Presentation alone: the battle and its
    /// cues are the same. Nothing for a player without audio.
    pub fn set_sound_parts(&mut self, (banks, songs): crate::preload::SoundParts) {
        if let Some(audio) = &mut self.audio {
            audio.set_songs(banks, songs);
        }
    }

    /// The battle's sound as it stands (none: silent): what each pack's
    /// driver plays (a host's check of the music it was given).
    pub fn audio(&self) -> Option<&BattleAudio> {
        self.audio.as_ref()
    }

    /// The sound of the ticks run since the last call, appended to `out`:
    /// stereo samples at `nettai_audio::SAMPLE_RATE`, about 549 a tick,
    /// for the host's own output. Nothing from a player without audio. (A
    /// host that never asks loses all but the last second.)
    pub fn take_samples(&mut self, out: &mut Vec<[f32; 2]>) {
        out.append(&mut self.samples);
    }

    /// Show the battle in another language from the next frame: `graphics`
    /// are the game's in it (`Game::graphics(lang)`; the host loads each
    /// language it offers and keeps it, and nothing is loaded here). Only the
    /// drawing changes, the pack's lettering and the content's strings: the
    /// battle doesn't know its language, what the renderer follows over
    /// time (the HUD's rolling numbers, its timers) carries on.
    pub fn set_language(&mut self, graphics: &Graphics) {
        self.renderer.set_graphics(graphics.packs());
        self.renderer.set_strings(graphics.strings.clone());
    }

    // ---- What a host may show ------------------------------------------------

    /// How the connection to the other player is doing, in a netplay match
    /// (its ping, jitter and quality, the loss, the input delay and the
    /// rollbacks; and while it is down, for how long and how long is left);
    /// none for a battle without one.
    pub fn net_status(&self) -> Option<NetStatus> {
        self.session.driver.net_status()
    }

    /// In a netplay match, the player's input delay from the next frame on,
    /// in ticks (`crate::netplay::NetOptions::input_delay`: the player's
    /// own, never sent; kept within `crate::netplay::INPUT_DELAY`); false
    /// for a battle without one.
    pub fn set_input_delay(&mut self, ticks: u32) -> bool {
        self.session.driver.set_input_delay(ticks)
    }

    /// Where playback is, in a few words.
    pub fn position(&self) -> String {
        self.session.driver.position()
    }

    /// Why the battle stopped, once it has, in words for a log: the engine
    /// stopped, the input ran out, the set is over, the other player left.
    /// What a host tells its player is [`Player::stop_reason`], in its own
    /// words.
    pub fn stopped(&self) -> Option<&str> {
        self.session.stopped.as_deref()
    }

    /// Why the battle stopped, once it has, as a host can tell its player
    /// (the set ended, the other player left, the connection was lost,
    /// something internal).
    pub fn stop_reason(&self) -> Option<StopReason> {
        self.session.stop_reason
    }

    /// It stopped because what the driver plays came to its end (a
    /// recording's round ran out, a set was played out), not because
    /// something went wrong.
    pub fn finished(&self) -> bool {
        self.session.finished
    }

    /// A set's result for the local player, once it was played out (a
    /// netplay match's as soon as it is over, while the players are still
    /// connected).
    pub fn result(&self) -> Option<BattleResult> {
        self.session.result.or_else(|| self.session.driver.result())
    }

    /// The first difference from what the driver's source recorded, once
    /// there is one.
    pub fn diverged(&self) -> Option<&str> {
        self.session.diverged.as_deref()
    }

    /// Ticks run since the start (wall-clock frames, of a driver that runs
    /// the battle itself).
    pub fn ticks(&self) -> u64 {
        self.session.ticks
    }

    /// The battle as it stands.
    pub fn battle(&self) -> &Battle {
        &self.session.battle
    }

    // ---- The parts, for a host that needs one ------------------------------

    /// The session: the driver, and the last step's details.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// The renderer (what it looked up and didn't find, its graphics mode).
    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    /// The enhanced mode's text renderer, for drawing a frame's text items
    /// oneself (`nettai_render::present`).
    pub fn text(&mut self) -> Option<&mut TextRenderer> {
        self.text.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::{LivePlayer, Step, folder_of, live_setup};
    use crate::game::GraphicsMode;
    use nettai_battle::content::testing;
    use std::sync::Arc;

    /// A live set on the engine's test content, to play.
    fn live() -> Box<dyn Driver> {
        let content = testing::content();
        let settings = nettai_battle::BattleSettings::on(&content, content.stage_by_key(testing::LINK_BATTLE));
        let folder = folder_of(&content, &[("gundels3", 13)]);
        let setup = live_setup(&content, settings, [folder, folder], "falzar", 1);
        Box::new(LivePlayer::new(nettai_match::Set::new(content.clone(), setup, [folder, folder])))
    }

    /// A player's time, at the original's rate: `ticks` of them.
    fn time(ticks: f64) -> Duration {
        Duration::from_secs_f64(ticks / FRAME_RATE)
    }

    /// The clock is the player's: it runs the ticks due for the time that
    /// passed at the speed set, keeps the part of a tick left over, and
    /// doesn't catch up after a stall. Paused it runs none, and a tick
    /// asked for outright still runs.
    #[test]
    fn the_player_keeps_the_clock() {
        let graphics = Arc::new(nettai_assets::Bundle::default());
        let mut p = Player::with(Renderer::new(graphics.clone()), None, None, live());
        assert_eq!(p.advance(time(10.5), 0), 10);
        assert_eq!(p.advance(time(0.6), 0), 1, "the half tick left over counts");
        assert_eq!(p.ticks(), 11);
        assert_eq!(p.advance(Duration::from_secs(5), 0), 15, "a quarter of a second at most (14.9 ticks, and the 0.1 left over)");
        assert!(p.faster() && p.speed() == 2.0);
        assert_eq!(p.advance(time(10.2), 0), 20);
        assert!(p.slower() && p.slower() && p.speed() == 0.5);
        assert_eq!(p.advance(time(10.2), 0), 5);
        assert!(p.faster());
        let ticks = p.ticks();
        assert!(p.set_paused(true) && p.paused());
        assert_eq!((p.advance(time(30.0), 0), p.ticks()), (0, ticks));
        assert!(p.tick(0), "a frame's step while paused");
        assert_eq!(p.ticks(), ticks + 1);
        assert!(p.set_paused(false));
        assert_eq!(p.advance(time(3.1), 0), 3, "nothing owed from the pause");
        // Starting over, and going on with another driver.
        assert!(p.restart());
        assert_eq!((p.ticks(), p.position().as_str()), (0, "live round 1 tick 0"));
        p.advance(time(4.1), 0);
        p.play(live());
        assert_eq!((p.ticks(), p.stopped(), p.finished(), p.result()), (0, None, false, None));
    }

    /// START is the host's where the game's pause was: on the custom screen
    /// it takes the cursor to OK as it did and nothing is said; in the fight
    /// the battle runs on unpaused and the player says who pressed it, once
    /// a press, until the host takes it. The host holds the ticks itself;
    /// starting over forgets a request nobody took.
    #[test]
    fn start_in_the_fight_is_the_hosts_to_answer() {
        use nettai_battle::battle::{fight, mode};
        use nettai_battle::input::keys;
        let graphics = Arc::new(nettai_assets::Bundle::default());
        let mut p = Player::with(Renderer::new(graphics.clone()), None, None, live());
        let fighting = |p: &Player| p.battle().round.mode == mode::FIGHTING && p.battle().fight.state == fight::FIGHTING;
        // Through the custom screen as the opponent's is pressed: a chip,
        // START to OK, A.
        let mut starts = 0;
        for tick in 0..3000u32 {
            if fighting(&p) {
                break;
            }
            let buttons = crate::driver::bot_buttons(p.battle(), 0, tick);
            starts += (buttons & keys::START != 0) as u32;
            assert!(p.tick(buttons));
        }
        assert!(fighting(&p) && starts > 0, "the fight runs, after {starts} START presses on the custom screen");
        assert_eq!(p.take_pause_requests(), [false; 2]);
        // In the fight: held three ticks, one press.
        for _ in 0..3 {
            assert!(p.tick(keys::START));
            assert!(fighting(&p) && !p.battle().paused);
        }
        assert!(p.tick(0));
        assert_eq!(p.take_pause_requests(), [true, false]);
        assert_eq!(p.take_pause_requests(), [false; 2], "taken once");
        // The host's own pause: no tick runs while it holds them.
        assert!(p.set_paused(true));
        let ticks = p.ticks();
        assert_eq!((p.advance(time(30.0), keys::START), p.ticks()), (0, ticks));
        assert_eq!(p.take_pause_requests(), [false; 2]);
        assert!(p.set_paused(false));
        assert_eq!(p.advance(time(1.1), keys::START), 1);
        assert_eq!(p.session().pause_requests, [true, false]);
        assert!(p.restart());
        assert_eq!(p.take_pause_requests(), [false; 2]);
    }

    /// The opponent's two choices are the player's to read and change
    /// while a live set runs; a battle that plays no opponent has none.
    #[test]
    fn the_opponent_is_changed_through_the_player() {
        use crate::driver::{InFight, OnCustomScreen, Opponent};
        let graphics = Arc::new(nettai_assets::Bundle::default());
        let mut p = Player::with(Renderer::new(graphics.clone()), None, None, live());
        assert_eq!(p.opponent(), Some(Opponent::default()));
        p.advance(time(20.2), 0);
        let wanted = Opponent { in_fight: InFight::AttacksAtRandom, on_custom_screen: OnCustomScreen::PicksNothing };
        assert!(p.set_opponent(wanted));
        assert_eq!(p.opponent(), Some(wanted));
        // (It stays through starting over.)
        assert!(p.restart());
        assert_eq!(p.opponent(), Some(wanted));
        let mut other = Player::with(Renderer::new(graphics.clone()), None, None, Box::new(RealTime(live())));
        assert_eq!(other.opponent(), None);
        assert!(!other.set_opponent(wanted));
    }

    /// On a display with frames to spare (120 a second), a late frame's two
    /// ticks are spread over it and the next: no frame runs two, and the
    /// clock loses nothing. On a display without (50 a second), a frame
    /// runs what it owes.
    #[test]
    fn a_late_frame_on_a_fast_display_spreads_its_ticks() {
        let graphics = Arc::new(nettai_assets::Bundle::default());
        let mut p = Player::with(Renderer::new(graphics.clone()), None, None, live());
        let frame = Duration::from_secs_f64(1.0 / 120.0);
        let mut elapsed = Duration::ZERO;
        for i in 0..240 {
            // Every 40th frame comes two frames late (25 ms).
            let e = if i % 40 == 39 { frame * 3 } else { frame };
            elapsed += e;
            assert!(p.advance(e, 0) <= 1, "frame {i}");
        }
        // One five frames late (42 ms, two and a half ticks) runs two at most.
        elapsed += frame * 5;
        assert!(p.advance(frame * 5, 0) <= 2);
        let clock = elapsed.as_secs_f64() * FRAME_RATE;
        assert!((clock - p.ticks() as f64).abs() < 2.0, "{} ticks for {clock:.2}", p.ticks());
        let mut p = Player::with(Renderer::new(graphics.clone()), None, None, live());
        let frame = Duration::from_secs_f64(1.0 / 50.0);
        let ran: Vec<u32> = (0..100).map(|i| p.advance(if i % 20 == 19 { frame * 2 } else { frame }, 0)).collect();
        assert!(ran.contains(&2), "{ran:?}");
    }

    /// The picture is the battle's, 240x160, and goes into a buffer of any
    /// size with nothing written over it: paused or not, the buffer is the
    /// frame scaled up, and what a host might say is values.
    #[test]
    fn the_player_gives_the_picture_alone() {
        use nettai_render::compose::{HEIGHT, WIDTH};
        let graphics = Arc::new(nettai_assets::Bundle::default());
        let mut p = Player::with(Renderer::new(graphics.clone()), None, None, live());
        for _ in 0..3 {
            p.advance(time(10.2), 0);
        }
        let frame = p.frame();
        assert_eq!(frame.pixels.len(), WIDTH * HEIGHT);
        let (w, h) = (WIDTH * 2 + 7, HEIGHT * 2 + 3);
        let mut running = vec![0u32; w * h];
        p.present(&mut running, w, h);
        let mut plain = vec![0u32; w * h];
        nettai_render::present::present(&frame, None, &mut plain, w, h);
        assert_eq!(running, plain, "the frame, and nothing over it");
        p.set_paused(true);
        let mut paused = vec![0u32; w * h];
        p.present(&mut paused, w, h);
        assert_eq!(paused, running, "a pause writes nothing on the picture");
        assert_eq!((p.paused(), p.position().as_str(), p.speed(), p.stopped(), p.net_status()), (true, "live round 1 tick 30", 1.0, None, None));
    }

    /// A driver that runs in real time with another player (netplay's): it
    /// advances a tick a frame and says so.
    struct RealTime(Box<dyn Driver>);

    impl Driver for RealTime {
        fn start(&mut self) -> Battle {
            self.0.start()
        }
        fn next(&mut self, b: &Battle, keys: u16) -> Option<Step> {
            self.0.next(b, keys)
        }
        fn position(&self) -> String {
            self.0.position()
        }
        fn net_status(&self) -> Option<NetStatus> {
            Some(NetStatus { ping_ms: Some(3.0), ..NetStatus::default() })
        }
        fn result(&self) -> Option<BattleResult> {
            Some(BattleResult::Won)
        }
        fn real_time(&self) -> bool {
            true
        }
    }

    /// In real time the player refuses a pause, another speed and starting
    /// over, and goes on at the original's rate; the connection's figures
    /// and the result the driver knows are the player's to give.
    #[test]
    fn a_real_time_battle_is_not_paused_or_restarted() {
        let graphics = Arc::new(nettai_assets::Bundle::default());
        let mut p = Player::with(Renderer::new(graphics.clone()), None, None, Box::new(RealTime(live())));
        assert!(p.real_time());
        assert!(!p.set_paused(true) && !p.faster() && !p.slower() && !p.restart());
        assert_eq!((p.paused(), p.speed()), (false, 1.0));
        assert_eq!(p.advance(time(5.1), 0), 5);
        assert_eq!(p.net_status().and_then(|n| n.ping_ms), Some(3.0));
        assert_eq!((p.result(), p.stopped()), (Some(BattleResult::Won), None));
    }

    /// Graphics whose panels are one solid tile of `color`, as a language's
    /// graphics.
    fn graphics_of(color: u16) -> Graphics {
        crate::testing::graphics(&nettai_battle::content::testing::content(), color)
    }

    /// The language changes mid-battle without a reset: a player shown in
    /// one language's graphics and then another's draws the first's frames
    /// up to the change and, from it on, exactly the frames of a player
    /// shown in the other from the start (what the renderer follows over
    /// time carries across). The battle itself is the same in all three.
    #[test]
    fn the_language_changes_mid_battle() {
        let (red, green) = (graphics_of(0x001F), graphics_of(0x03E0));
        let player = |g: &Graphics| Player::with(g.renderer(GraphicsMode::Original, None), None, None, live());
        let (mut first, mut second, mut swapped) = (player(&red), player(&green), player(&red));
        let mut differed = 0;
        for tick in 1..=240u32 {
            if tick == 150 {
                swapped.set_language(&green);
            }
            for p in [&mut first, &mut second, &mut swapped] {
                assert!(p.tick(0));
            }
            let (a, b, s) = (first.frame(), second.frame(), swapped.frame());
            differed += (a.pixels != b.pixels) as u32;
            let same = if tick < 150 { &a } else { &b };
            assert!(s.pixels == same.pixels && s.depth == same.depth, "tick {tick}");
            assert_eq!(swapped.battle().digest(), first.battle().digest(), "tick {tick}: the battle doesn't know its language");
        }
        assert!(differed > 100, "the two graphics draw alike ({differed} frames differ)");
        assert_eq!((swapped.ticks(), swapped.position()), (first.ticks(), first.position()));
    }

    /// The sound comes as samples, a frame of them a tick, for the host to
    /// take: taking them empties what the player kept; a player without
    /// audio gives none.
    #[test]
    fn the_player_gives_the_sound_as_samples() {
        use m4a::bank::{MixerConfig, PlayerConfig};
        let graphics = Arc::new(nettai_assets::Bundle::default());
        // A sound bank with EXE6's players and no songs: a silent frame a tick.
        let players = (0..32).map(|p| PlayerConfig { max_tracks: if p == 31 { 8 } else { 2 }, uses_priority: p != 31, track_order: p as u8 }).collect();
        let bank = m4a::SoundBank {
            mixer: MixerConfig { mix_rate: 10512, ds_channels: 4, master_volume: 15, reverb: 0, dac_resolution: 1 },
            players,
            songs: vec![None; 0x100],
            voicegroups: vec![],
            key_maps: vec![],
            samples: vec![],
            waves: vec![],
        };
        let audio = BattleAudio::with_banks(vec![Arc::new(bank)], nettai_audio::Songs::numbers(0x100));
        let mut p = Player::with(Renderer::new(graphics.clone()), None, Some(audio), live());
        assert_eq!(p.advance(time(10.1), 0) + p.advance(time(10.1), 0), 20);
        let mut samples = Vec::new();
        p.take_samples(&mut samples);
        // (A frame is 548 or 549 samples at 32768 Hz.)
        assert!((20 * 548..=20 * 549).contains(&samples.len()), "{} samples", samples.len());
        let had = samples.len();
        p.take_samples(&mut samples);
        assert_eq!(samples.len(), had, "taken once");
        // A host that never takes them keeps no more than a second.
        for _ in 0..200 {
            p.tick(0);
        }
        samples.clear();
        p.take_samples(&mut samples);
        assert_eq!(samples.len(), MOST_SAMPLES);
        let mut silent = Player::with(Renderer::new(graphics.clone()), None, None, live());
        silent.advance(time(5.1), 0);
        samples.clear();
        silent.take_samples(&mut samples);
        assert!(samples.is_empty());
    }

    /// The enhanced mode's smooth motion (`nettai_render::smooth`): the gauge's fill shows between its whole
    /// pixels (a nudge) without changing the frame's own pixels, and everything else, the custom screen's slide
    /// among it, moves as the original's does (the same pixels as the original mode's, here with no font for the
    /// text); every frame is the battle's alone: a renderer that draws a copy of the battle as it was at that tick, with the HUD's rolling state as
    /// it was, draws the same frame (as a rollback or a seek draws a frame again). The original mode draws alike
    /// with the motion reduced or not.
    #[test]
    fn the_smooth_motion_is_the_battles_alone_and_moves_nothing_the_original_doesnt() {
        use crate::driver::bot_buttons;
        use nettai_battle::battle::{fight, mode};
        let content = testing::content();
        let mut graphics = crate::testing::graphics(&content, 0x001F);
        // (A HUD with tiles: the HP box and the gauge drawn, its cells by eighths in their own colors.)
        let mut bundle = (*graphics.bundles[0]).clone();
        let hud = &mut bundle.hud;
        hud.tiles = nettai_assets::Tiles { pixels: vec![1; 0x40 * nettai_assets::Tiles::TILE] };
        hud.first_tile = 0x100;
        hud.gauge_tiles = nettai_assets::Tiles { pixels: (0..0x20 * nettai_assets::Tiles::TILE).map(|i| (i / nettai_assets::Tiles::TILE) as u8 % 15 + 1).collect() };
        hud.gauge_first_tile = 0x200;
        hud.hp_palettes = [[0x7FFF; 16]; 3];
        hud.gauge_palette = std::array::from_fn(|i| i as u16 * 0x421);
        let entry = |tile| nettai_assets::MapEntry { tile, hflip: false, vflip: false, palette: 13 };
        hud.hp_box = vec![entry(0x120); 12];
        hud.gauge_frame = vec![entry(0x200); 36];
        graphics.bundles[0] = Arc::new(bundle);
        let renderer = |mode: GraphicsMode, reduce: bool| {
            let mut renderer = Renderer::with_packs(graphics.packs());
            renderer.set_mode(mode, None);
            renderer.reduce_motion = reduce;
            renderer
        };
        let player = |mode, reduce| Player::with(renderer(mode, reduce), None, None, live());
        let (mut on, mut off) = (player(GraphicsMode::Enhanced, false), player(GraphicsMode::Enhanced, true));
        let (mut original, mut original_reduced) = (player(GraphicsMode::Original, false), player(GraphicsMode::Original, true));
        let (mut nudged_gauge, mut lifted, mut overlaid, mut fought) = (0, 0, 0, 0);
        for tick in 0..3000u32 {
            let k = bot_buttons(on.battle(), 0, tick);
            assert!(on.tick(k) && off.tick(k) && original.tick(k) && original_reduced.tick(k));
            let frame = on.frame();
            let t = on.battle().round.ticks;
            let plain = off.frame();
            assert!(plain.nudge.is_none() && plain.lift.is_none() && plain.overlays.is_empty() && plain.pixels == frame.pixels, "tick {t}: reduced motion's");
            if let Some(n) = &frame.nudge {
                assert!(n.sub > 0 && n.sub < 16);
                nudged_gauge += 1;
            }
            // The custom screen's slide: the camera half a pixel past its whole pixels on its odd ticks.
            if let Some(l) = &frame.lift {
                assert!(l.sub == 0x8000 && l.pixels != frame.pixels, "tick {t}: the field a pixel up");
                lifted += 1;
            }
            // Drawn again from a copy of the battle at this tick.
            let mut again = renderer(GraphicsMode::Enhanced, false);
            again.hud_state = on.renderer().hud_state.clone();
            let copy = on.battle().clone();
            let redrawn = again.render(&copy);
            assert!(
                redrawn.pixels == frame.pixels && redrawn.nudge == frame.nudge && redrawn.lift == frame.lift && redrawn.overlays == frame.overlays,
                "tick {t}: drawn again"
            );
            let o = original.frame();
            assert!(
                o.pixels == original_reduced.frame().pixels && o.nudge.is_none() && o.lift.is_none() && o.overlays.is_empty(),
                "tick {t}: the original's"
            );
            // The enhanced frame's own pixels are the original's but where the sprites it leaves to the output are
            // (squashed, turned, or between whole pixels: overlays).
            for i in (0..o.pixels.len()).filter(|&i| o.pixels[i] != frame.pixels[i]) {
                let (x, y) = ((i % 240) as i32, (i / 240) as i32);
                let under = |v: &nettai_render::smooth::Overlay| (v.x..v.x + v.width).contains(&x) && (v.y..v.y + v.height).contains(&y);
                assert!(frame.overlays.iter().any(under), "tick {t}: ({x}, {y}) is the original's");
            }
            overlaid += !frame.overlays.is_empty() as u32;
            fought += (on.battle().round.mode == mode::FIGHTING && on.battle().fight.state == fight::FIGHTING) as u32;
            if fought > 60 {
                break;
            }
        }
        assert!(fought > 60, "the fight runs");
        assert!(nudged_gauge > 10, "the gauge fills between its whole pixels ({nudged_gauge} frames)");
        assert!(lifted >= 10, "the field slides between its whole pixels ({lifted} frames)");
        assert!(overlaid >= lifted, "the sprites on the field slide with it ({overlaid} frames)");
    }

    /// The renderer's pause display: drawn while a player holds the fight in the game's own pause, as the
    /// game draws it (the default), and left out with `pause_display` off, which changes nothing else of the
    /// frame; out of the pause the two draw alike. (A round that leaves the game its START: its own pause.)
    #[test]
    fn the_pause_display_is_left_out_alone() {
        use crate::driver::bot_buttons;
        use nettai_battle::input::keys;
        let content = testing::content();
        let mut graphics = crate::testing::graphics(&content, 0x001F);
        // (A HUD with tiles, so it is drawn, and "PAUSE" in a color of its own.)
        let mut bundle = (*graphics.bundles[0]).clone();
        bundle.hud.tiles = nettai_assets::Tiles { pixels: vec![0; 0x40 * nettai_assets::Tiles::TILE] };
        bundle.hud.pause = nettai_assets::Tiles { pixels: vec![1; 10 * nettai_assets::Tiles::TILE] };
        bundle.hud.enemy_palette[1] = 0x7FFF;
        graphics.bundles[0] = Arc::new(bundle);
        let settings = nettai_battle::BattleSettings::on(&content, content.stage_by_key(testing::LINK_BATTLE));
        let folder = folder_of(&content, &[("gundels3", 13)]);
        let mut setup = live_setup(&content, settings, [folder, folder], "falzar", 1);
        setup.host_keys.pause = false;
        let set = || Box::new(LivePlayer::new(nettai_match::Set::new(content.clone(), setup.clone(), [folder, folder])));
        let mut on = Player::with(Renderer::with_packs(graphics.packs()), None, None, set());
        let mut off = Player::with(Renderer::with_packs(graphics.packs()), None, None, set());
        off.renderer().pause_display = false;
        let (mut tick, mut fought) = (0, 0);
        while fought < 30 {
            let k = bot_buttons(on.battle(), 0, tick);
            assert!(on.tick(k) && off.tick(k) && tick < 3000);
            tick += 1;
            fought += on.battle().pausable() as u32;
            assert_eq!(on.frame().pixels, off.frame().pixels, "tick {tick}: out of the pause, alike");
        }
        assert!(on.tick(keys::START) && off.tick(keys::START));
        assert_eq!(on.battle().paused_by(), Some(0));
        let (with, without) = (on.frame().pixels, off.frame().pixels);
        let differ: Vec<usize> = (0..with.len()).filter(|&i| with[i] != without[i]).collect();
        assert!(!differ.is_empty(), "the pause display is drawn by default");
        let (x, y) = bundle_pause(&graphics);
        for i in differ {
            let (px, py) = (i % 240, i / 240);
            assert!((x..x + 40).contains(&px) && (y..y + 16).contains(&py), "({px}, {py}) differs, outside the pause display");
        }
    }

    /// Where the HUD draws "PAUSE".
    fn bundle_pause(graphics: &Graphics) -> (usize, usize) {
        let (x, y) = graphics.bundles[0].hud.layout.pause;
        (x as usize, y as usize)
    }
}
