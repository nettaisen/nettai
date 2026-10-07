//! What each player's console keeps for itself that the simulation needs:
//! its own random number stream (RNG1) and everything that advances it.
//!
//! In the original each console has an RNG1 of its own, never shared over
//! the link: it shuffles the console's folder at the round's init and
//! ChpShufl's re-deal on its custom screen (the one use the simulation
//! sees). During a battle it advances:
//!
//! - once a frame, after the battle's (the main loop's `GetRNG1`);
//! - twice a tick while the console's camera shakes
//!   (`camera_doShakeEffect_80301e8`): the shared simulation starts shakes
//!   on both consoles, the custom screen's Beast Out on its own;
//! - once when the emotion window's timer runs out on a bugged navi
//!   (`sub_801CC94`, the flicker);
//! - by ChpShufl's re-deal (`sub_8029788`, `sub_8029688`) on the
//!   console's custom screen.
//!
//! All of that is a function of the round's start, the shared simulation
//! and the player's buttons, so each player's console is simulated here
//! (both, on every peer). Only link stalls, frames on which a console waits
//! for the link and the battle doesn't tick, are the console's own; the
//! main loop's draw still runs on them. This engine has none (its link is
//! simulated), and nor do the recordings. See docs/engine/custom-screen.md
//! §8.

use crate::battle::Battle;
use crate::rng::Rng;

/// What a player's console brings to a round besides the folder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ConsoleSetup {
    /// RNG1 on the round's first battle frame, after the folder shuffle.
    pub rng: u32,
    /// Where the shuffle put the folder's tag pair (BattleState+0x45, while
    /// +0x44 says the pair is there): ChpShufl's re-deal leaves the pair
    /// alone. Each chip OK takes out of the folder moves the index down by
    /// one, so it follows the pair as the folder closes up.
    pub tag_pair: Option<u8>,
    /// The console's frame counter before the round's first tick
    /// (`Console::frames`).
    pub frames: u32,
}

/// One camera shake channel: ticks left and the magnitude (`byte_8030284`'s
/// row).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Shake {
    pub ticks: u16,
    pub magnitude: u16,
}

/// A console's camera shake (`eCamera` +0x0C..+0x12, +0x3C..+0x48).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CameraShake {
    /// `camera_initShakeEffect_80302a8`'s channel (+0x0C ticks, +0x0E
    /// magnitude).
    pub primary: Shake,
    /// `sub_80302B6`'s (+0x10, +0x12).
    pub secondary: Shake,
    /// This tick's jitter of the camera, 16.16 (next X/Y minus X/Y;
    /// presentation).
    pub jitter: (i32, i32),
}

/// The jitter of each magnitude (`byte_8030284`): the RNG draw's mask, and
/// the bias taken off (16.16).
const JITTER: [(u32, i32); 4] = [(0x1, 0x1_0000), (0x3, 0x2_0000), (0x7, 0x4_0000), (0xF, 0x8_0000)];

/// The emotion window's flicker (`eStruct2035280`: the HUD task bit 14,
/// `sub_801CADC` and `sub_801CC94`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EmotionWindow {
    /// The task runs (from the intro's second tick until the round's
    /// result).
    pub running: bool,
    /// +0x38: ticks until the next check (20, the first 120).
    pub timer: u16,
    /// +0x1D: flickers left, and +0x0F: ticks left of the current one.
    pub flickers: u8,
    pub flicker_ticks: u8,
    /// +0x1E: the save's glitch (`Console::emotion_window_glitch`), kept
    /// outside random battles and battle modes 1-5 and 8; BugFix clears
    /// it.
    pub glitch: bool,
}

/// A player's console: see the module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Console {
    /// RNG1.
    pub rng: Rng,
    pub camera: CameraShake,
    pub emotion_window: EmotionWindow,
    /// Where the folder's tag pair is (`ConsoleSetup`; OK moves it with
    /// the chips it takes out, an opening that could deal it drops it).
    pub tag_pair: Option<u8>,
    /// The save's glitch (EXE6's event flag 0x1720, 0x1723 with patch
    /// cards; EXE5's 0x10C1 and 0x10C4), for the emotion window's start:
    /// MegaMan's window flickers as a bugged navi's does, bugs or not. No
    /// setup gives it: the game's rules make it as the round is set up
    /// (`battle.set_emotion_window_glitch`: the NaviCust's compile when a
    /// bug applies, the patch cards' routine).
    pub emotion_window_glitch: bool,
    /// The HP box's count to its next low-HP sound (`eStruct2035280`+6:
    /// only the sound depends on it).
    pub low_hp_ticks: u8,
    /// The game's frame counter on this console, which counts every frame
    /// since the match began, whatever the battle does. Only the HUD's
    /// periodic sounds and blinks go by it (`sub_800AE90`).
    pub frames: u32,
}

impl Console {
    pub fn new(setup: &ConsoleSetup) -> Console {
        Console {
            rng: Rng::new(setup.rng),
            camera: CameraShake::default(),
            emotion_window: EmotionWindow::default(),
            tag_pair: setup.tag_pair,
            emotion_window_glitch: false,
            low_hp_ticks: 0,
            frames: setup.frames,
        }
    }

    /// `sub_80302B6` on this console only: its secondary channel shakes
    /// `ticks` ticks at `magnitude`.
    pub fn shake_secondary(&mut self, magnitude: u16, ticks: u16) {
        self.camera.secondary = Shake { ticks, magnitude };
    }

    /// `camera_doShakeEffect_80301e8`: `primary_first` when the primary
    /// channel may shake.
    fn update_camera(&mut self, primary_first: bool) {
        let c = &mut self.camera;
        let channel = if primary_first && c.primary.ticks != 0 {
            &mut c.primary
        } else if c.secondary.ticks != 0 {
            &mut c.secondary
        } else {
            c.primary.magnitude = 0;
            c.jitter = (0, 0);
            return;
        };
        channel.ticks -= 1;
        let magnitude = channel.magnitude;
        let Some(&(mask, bias)) = JITTER.get(magnitude as usize) else {
            panic!("camera shake magnitude {magnitude} reads past byte_8030284")
        };
        let dx = ((self.rng.next() & mask) << 16) as i32 - bias;
        let dy = ((self.rng.next() & mask) << 16) as i32 - bias;
        c.jitter = (dx, dy);
    }

    /// `sub_801CC94`: count down to a check; on a bugged navi (or MegaMan,
    /// the navi that changes form, with the save's glitch) flicker once or
    /// twice (an RNG1 draw), 12 ticks each, then 20 ticks to the next
    /// check.
    fn update_emotion_window(&mut self, megaman: bool, bugged: bool) {
        let w = &mut self.emotion_window;
        if w.flickers != 0 {
            w.flicker_ticks -= 1;
            if w.flicker_ticks == 0 {
                w.flickers -= 1;
                w.flicker_ticks = FLICKER_TICKS;
            }
            return;
        }
        w.timer = w.timer.wrapping_sub(1);
        if w.timer != 0 {
            return;
        }
        // The window shows the side's navi (+0x15; +0x17, which would
        // show another, is never set in a battle).
        let flickers = if megaman { w.glitch || bugged } else { bugged };
        if flickers {
            w.flickers = (self.rng.next_positive() & 1) as u8 + 1;
            w.flicker_ticks = FLICKER_TICKS;
        }
        w.timer = CHECK_TICKS;
    }
}

/// `sub_800FE52`: how many of the navi's NaviCust bugs it has.
fn bugs(stats: &crate::setup::NaviStats) -> u32 {
    let b = &stats.bugs;
    [
        b.processing == 1,
        b.panel_trail_level != 0,
        b.buster_blanks != 0,
        b.hit_status != 0,
        b.custom_damage != 0,
        b.emotion != 0,
        b.hp_drain != 0,
        b.custom_drain != 0,
        b.battle_start != 0,
        b.hand_shrink_turn != 0,
    ]
    .into_iter()
    .filter(|&x| x)
    .count() as u32
}

/// The emotion window: ticks between checks, to the first, and of a
/// flicker.
const CHECK_TICKS: u16 = 0x14;
const FIRST_CHECK_TICKS: u16 = 0x78;
const FLICKER_TICKS: u8 = 0x0C;

impl Battle {
    /// `camera_initShakeEffect_80302a8`: both consoles' cameras shake
    /// `ticks` ticks at `magnitude` (0-3) on the primary channel (the
    /// shared simulation starts it on both).
    pub fn shake_camera(&mut self, magnitude: u16, ticks: u16) {
        for c in &mut self.consoles {
            c.camera.primary = Shake { ticks, magnitude };
        }
    }

    /// EXE5's shake (0x08030D78, the arena's `effects.shake`): one channel
    /// (EXE5 has no `sub_80302B6`), shaking while the battle isn't paused,
    /// while it dims or while battle flag 0x20 is set (TomahawkSoul's change:
    /// `battle_isTimeStopPauseOrBattleFlags0x20_800a0a4`; the battle's
    /// subsystem is always in use), its jitter two draws from
    /// the battle's RNG2 (the simulation's: every console shakes alike);
    /// otherwise held, its jitter none and its magnitude kept.
    fn update_cameras_from_battle_rng(&mut self) {
        let runs = !self.paused || self.is_dimmed() || self.round.flags & crate::battle::battle_flags::SHAKE_THROUGH_PAUSE != 0;
        let shake = self.consoles[0].camera.primary;
        if !runs || shake.ticks == 0 {
            for c in &mut self.consoles {
                c.camera.jitter = (0, 0);
            }
            return;
        }
        let Some(&(mask, bias)) = JITTER.get(shake.magnitude as usize) else {
            panic!("camera shake magnitude {} reads past 0x08030DF4", shake.magnitude)
        };
        let dx = ((self.rng.next() & mask) << 16) as i32 - bias;
        let dy = ((self.rng.next() & mask) << 16) as i32 - bias;
        for c in &mut self.consoles {
            c.camera.primary.ticks = c.camera.primary.ticks.saturating_sub(1);
            c.camera.jitter = (dx, dy);
        }
    }

    /// `sub_80302B6`: the same on the secondary channel.
    pub fn shake_camera_secondary(&mut self, magnitude: u16, ticks: u16) {
        for c in &mut self.consoles {
            c.shake_secondary(magnitude, ticks);
        }
    }

    /// `sub_802FFF4`'s shake, each running tick after the objects: the
    /// primary channel first unless the battle is paused without dimming
    /// while player 0's status (BattleState+0x14) has neither bit 0 nor 2
    /// (`sub_80269D0`) or battle flag 0x20 is set (which nothing in EXE6
    /// sets); the secondary otherwise. (The battle's subsystem is always in
    /// use.)
    pub(crate) fn update_cameras(&mut self) {
        if self.game_rules().effects.shake == crate::content::ShakeRule::BattleRng {
            self.update_cameras_from_battle_rng();
            return;
        }
        let primary_first = self.round.remote_status[0] & 5 != 0
            || !self.paused
            || self.is_dimmed()
            || self.round.flags & crate::battle::battle_flags::SHAKE_THROUGH_PAUSE != 0;
        for c in &mut self.consoles {
            c.update_camera(primary_first);
        }
    }

    /// `sub_801E5F8` (the intro's HUD setup): the emotion windows' task
    /// starts, their first check 120 ticks away.
    pub(crate) fn start_emotion_windows(&mut self) {
        use crate::setup::effects;
        // The glitch is kept outside random battles and battle modes 1-5
        // and 8.
        let mode = self.round.mode_copy;
        let keeps = self.setup.settings.effects & effects::RANDOM == 0 && !matches!(mode, 1..=5 | 8);
        for c in &mut self.consoles {
            let w = &mut c.emotion_window;
            w.timer = FIRST_CHECK_TICKS;
            w.glitch = keeps && c.emotion_window_glitch;
            w.running = true;
        }
    }

    /// The round's result hides the HUD: the emotion windows' task stops
    /// (`sub_801BED6` with bit 14, by the win, loss and draw states).
    pub(crate) fn stop_emotion_windows(&mut self) {
        for c in &mut self.consoles {
            c.emotion_window.running = false;
        }
    }

    /// `sub_801E658` (BugFix's effect): the save's glitch is gone from
    /// every console's emotion window.
    pub fn clear_emotion_window_glitch(&mut self) {
        for c in &mut self.consoles {
            c.emotion_window.glitch = false;
        }
    }

    /// The HUD's task bit 14 on each console: its own navi's emotion
    /// window (`sub_800FE52` counts the navi's NaviCust bugs), where the
    /// game's has its check (`effects.bug_flicker`).
    pub(crate) fn update_emotion_windows(&mut self) {
        if !self.content.rules().effects.bug_flicker {
            return;
        }
        for side in 0..2 {
            if !self.consoles[side].emotion_window.running {
                continue;
            }
            let (megaman, bugged) = (self.navi(side).changes_form(), bugs(&self.stats[side]) != 0);
            self.consoles[side].update_emotion_window(megaman, bugged);
        }
    }

    /// The main loop's draw, once a frame after the battle's.
    pub(crate) fn end_console_frames(&mut self) {
        // (The main loop's draw, where the game's makes one.)
        if !self.content.rules().effects.rng1_per_frame {
            return;
        }
        for c in &mut self.consoles {
            c.rng.next();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> Console {
        Console::new(&ConsoleSetup { rng: 0xA5A5_0001, ..ConsoleSetup::default() })
    }

    #[test]
    fn a_shaking_camera_draws_twice_a_tick() {
        let mut c = console();
        c.shake_secondary(1, 3);
        let mut expected = c.rng;
        for _ in 0..3 {
            c.update_camera(true);
            let dx = ((expected.next() & 3) << 16) as i32 - 0x2_0000;
            let dy = ((expected.next() & 3) << 16) as i32 - 0x2_0000;
            assert_eq!(c.camera.jitter, (dx, dy));
            assert_eq!(c.rng, expected);
        }
        c.update_camera(true);
        assert_eq!(c.rng, expected);
        assert_eq!(c.camera.jitter, (0, 0));
    }

    /// docs/design/exe5-map.md §15.3 item 15: EXE5's shake (the arena's
    /// `effects.shake`) draws its jitter twice a tick from the battle's
    /// RNG, alike on both consoles, and holds while paused.
    #[test]
    fn exe5s_shake_draws_from_the_battle() {
        use crate::content::{Content, testing};
        let mut c: Content = testing::build();
        c.define().unwrap_or_else(|e| panic!("{e}"));
        {
            let rules = c.rules_mut();
            rules.effects.shake = crate::content::ShakeRule::BattleRng;
        }
        let c = std::sync::Arc::new(c);
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(&c));
        crate::content::testing::on(&mut setup, &c);
        let mut b = Battle::new(setup, c);
        let consoles = [b.consoles[0].rng, b.consoles[1].rng];
        b.shake_camera(1, 3);
        let mut expected = b.rng;
        for left in (0..3).rev() {
            b.update_cameras();
            let dx = ((expected.next() & 3) << 16) as i32 - 0x2_0000;
            let dy = ((expected.next() & 3) << 16) as i32 - 0x2_0000;
            assert_eq!(b.rng, expected);
            for c in &b.consoles {
                assert_eq!((c.camera.jitter, c.camera.primary.ticks), ((dx, dy), left));
            }
        }
        assert_eq!([b.consoles[0].rng, b.consoles[1].rng], consoles, "the consoles' own RNGs stay");
        b.update_cameras();
        assert_eq!(b.rng, expected);
        b.shake_camera(1, 2);
        b.paused = true;
        b.update_cameras();
        assert_eq!((b.rng, b.consoles[0].camera.primary.ticks), (expected, 2), "held while paused");
    }

    #[test]
    fn the_primary_channel_waits_while_paused() {
        let mut c = console();
        c.camera.primary = Shake { ticks: 2, magnitude: 0 };
        c.update_camera(false);
        assert_eq!(c.camera.primary.ticks, 2);
        c.update_camera(true);
        assert_eq!(c.camera.primary.ticks, 1);
    }

    #[test]
    fn a_bugged_navis_emotion_window_flickers() {
        let mut c = console();
        c.emotion_window = EmotionWindow { running: true, timer: 3, ..EmotionWindow::default() };
        let start = c.rng;
        c.update_emotion_window(true, true);
        c.update_emotion_window(true, true);
        assert_eq!(c.rng, start);
        c.update_emotion_window(true, true);
        let mut expected = start;
        let flickers = (expected.next_positive() & 1) as u8 + 1;
        assert_eq!(c.rng, expected);
        assert_eq!(c.emotion_window.flickers, flickers);
        // 12 ticks a flicker, then 20 to the next check.
        for _ in 0..12 * flickers as u32 + 19 {
            c.update_emotion_window(true, true);
        }
        assert_eq!(c.rng, expected);
        c.update_emotion_window(true, true);
        expected.next();
        assert_eq!(c.rng, expected);
    }

    #[test]
    fn an_unbugged_navi_needs_the_glitch() {
        let mut c = console();
        c.emotion_window = EmotionWindow { running: true, timer: 1, glitch: true, ..EmotionWindow::default() };
        let start = c.rng;
        c.update_emotion_window(false, false);
        assert_eq!(c.rng, start);
        c.emotion_window.timer = 1;
        c.update_emotion_window(true, false);
        assert_ne!(c.rng, start);
    }
}
