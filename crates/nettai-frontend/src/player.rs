//! A battle being played, for a host to show and hear: [`Player`] owns the
//! session, the renderer, the text renderer (the font mode's) and the
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

use crate::driver::{Driver, NetStatus};
use crate::game::{Graphics, Loaded};
use crate::session::Session;
use nettai_audio::BattleAudio;
use nettai_battle::{Battle, BattleResult};
use nettai_render::vfont::TextRenderer;
use nettai_render::{Frame, Renderer};
use std::time::Duration;

/// The original's frame rate: a tick is one frame of it.
pub const FRAME_RATE: f64 = 59.7275;

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
const MOST_SAMPLES: usize = nettai_audio::SAMPLE_RATE as usize;

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
    speed: usize,
    /// Ticks the clock owes: less than one, or less than two on a display
    /// with frames to spare after a late frame.
    owed: f64,
    /// The time between the host's frames, averaged (seconds).
    frame_time: f64,
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
    /// font mode's text renderer (none: the frames carry their own text)
    /// and the audio (none: silent).
    pub fn with(renderer: Renderer, text: Option<TextRenderer>, audio: Option<BattleAudio>, driver: Box<dyn Driver>) -> Player {
        let mut player = Player {
            session: Session::new(driver),
            renderer,
            text,
            audio,
            samples: Vec::new(),
            paused: false,
            speed: NORMAL,
            owed: 0.0,
            frame_time: 1.0 / FRAME_RATE,
        };
        player.start_over();
        player
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
        self.owed = 0.0;
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
            self.owed = 0.0;
            return 0;
        }
        let elapsed = elapsed.min(MOST_ELAPSED).as_secs_f64();
        self.frame_time += (elapsed - self.frame_time) / 16.0;
        let rate = FRAME_RATE * SPEEDS[self.speed];
        self.owed += elapsed * rate;
        let mut due = self.owed.floor() as u32;
        if due >= 2 && self.frame_time * rate <= SPARE_FRAMES {
            due -= 1;
        }
        self.owed -= due as f64;
        (0..due).take_while(|_| self.tick(buttons)).count() as u32
    }

    /// Run one tick with `buttons` held, whatever the clock or the pause
    /// say: a host that paces itself, or a frame's step while paused. (A
    /// driver that runs the battle itself, netplay's, runs one wall-clock
    /// frame, which may wait.) False, with [`Player::stopped`] saying why,
    /// when the battle can't go on.
    pub fn tick(&mut self, buttons: u16) -> bool {
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
                None => audio.handle(self.session.battle.sound_cues()),
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

    /// Start over: what the driver plays from its beginning (a recording's
    /// round, live play's set from its first round). False in real time.
    pub fn restart(&mut self) -> bool {
        if self.real_time() {
            return false;
        }
        self.session.restart();
        self.renderer.reset();
        self.owed = 0.0;
        true
    }

    // ---- The picture and the sound ---------------------------------------

    /// The battle's picture now, 240x160, with nothing over it.
    pub fn frame(&mut self) -> Frame {
        self.renderer.render(&self.session.battle)
    }

    /// The picture into the host's pixels: `buffer` holds `width` x
    /// `height` of them, 0x00RRGGBB each, row by row. The frame is scaled
    /// up by the largest whole factor that fits and centered, and the font
    /// mode's text (the game's own) is drawn at the buffer's resolution.
    /// Nothing else is drawn.
    pub fn present(&mut self, buffer: &mut [u32], width: usize, height: usize) {
        let frame = self.frame();
        nettai_render::present::present(&frame, self.text.as_mut(), buffer, width, height);
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
    /// (its ping, loss, present delay and rollbacks); none for a battle
    /// without one.
    pub fn net_status(&self) -> Option<NetStatus> {
        self.session.driver.net_status()
    }

    /// In a netplay match, show the frame `ticks` behind the player's
    /// newest input from the next frame on (the present delay, the
    /// player's own); false for a battle without one.
    pub fn set_present_delay(&mut self, ticks: u32) -> bool {
        self.session.driver.set_present_delay(ticks)
    }

    /// Where playback is, in a few words.
    pub fn position(&self) -> String {
        self.session.driver.position()
    }

    /// Why the battle stopped, once it has: the engine stopped, the input
    /// ran out, the set is over, the other player left.
    pub fn stopped(&self) -> Option<&str> {
        self.session.stopped.as_deref()
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

    /// The renderer (what it looked up and didn't find, its text mode).
    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    /// The font mode's text renderer, for drawing a frame's text items
    /// oneself (`nettai_render::present`).
    pub fn text(&mut self) -> Option<&mut TextRenderer> {
        self.text.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::{LivePlayer, Step, folder_of, live_setup};
    use crate::game::TextMode;
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

    /// Graphics whose panels are one solid tile of `color` (the field's
    /// alone: no sprite, no lettering), as a language's graphics.
    fn graphics_of(color: u16) -> Graphics {
        use nettai_assets::{Bundle, Field, MapEntry, Tiles};
        let block = [MapEntry { tile: 0xA3, hflip: false, vflip: false, palette: 1 }; 15];
        let mut palettes = vec![[0u16; 16]; 8];
        palettes[0][1] = color;
        let field = Field {
            tiles: Tiles { pixels: vec![1; Tiles::TILE] },
            first_tile: 0xA3,
            palettes,
            first_palette: 1,
            panel_types: (0..13).collect(),
            panels: (0..13 * 6).map(|_| block).collect(),
            front_edges: [[MapEntry::default(); 5]; 2],
            highlights: vec![[MapEntry::default(); 15]; 2],
            ..Field::default()
        };
        Graphics {
            bundles: vec![Arc::new(Bundle { field, ..Bundle::default() })],
            strings: None,
            own: nettai_battle::content::PackId(0),
            report: Default::default(),
        }
    }

    /// The language changes mid-battle without a reset: a player shown in
    /// one language's graphics and then another's draws the first's frames
    /// up to the change and, from it on, exactly the frames of a player
    /// shown in the other from the start (what the renderer follows over
    /// time carries across). The battle itself is the same in all three.
    #[test]
    fn the_language_changes_mid_battle() {
        let (red, green) = (graphics_of(0x001F), graphics_of(0x03E0));
        let player = |g: &Graphics| Player::with(g.renderer(TextMode::Original, None), None, None, live());
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
}
