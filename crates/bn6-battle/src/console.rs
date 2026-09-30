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
use crate::setup::Navi;

/// What a player's console brings to a round besides the folder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ConsoleSetup {
    /// RNG1 on the round's first battle frame, after the folder shuffle.
    pub rng: u32,
    /// Where the shuffle put the folder's tag pair (BattleState+0x45, while
    /// +0x44 says the pair is there): ChpShufl's re-deal leaves the pair
    /// alone. The index is the shuffle's; nothing updates it as the folder
    /// closes up.
    pub tag_pair: Option<u8>,
    /// The save's event flag 0x1720: MegaMan's emotion window flickers as
    /// a bugged navi's does, bugs or not.
    pub emotion_window_glitch: bool,
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
    /// +0x1E: the save's glitch (`ConsoleSetup::emotion_window_glitch`),
    /// kept outside random battles and battle modes 1-5 and 8; BugFix
    /// clears it.
    pub glitch: bool,
}

/// A player's console: see the module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Console {
    /// RNG1.
    pub rng: Rng,
    pub camera: CameraShake,
    pub emotion_window: EmotionWindow,
    /// Where the folder's tag pair was shuffled to (`ConsoleSetup`).
    pub tag_pair: Option<u8>,
    /// The save's glitch, for the emotion window's start.
    pub emotion_window_glitch: bool,
}

impl Console {
    pub fn new(setup: &ConsoleSetup) -> Console {
        Console {
            rng: Rng::new(setup.rng),
            camera: CameraShake::default(),
            emotion_window: EmotionWindow::default(),
            tag_pair: setup.tag_pair,
            emotion_window_glitch: setup.emotion_window_glitch,
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

    /// `sub_801CC94`: count down to a check; on a bugged navi (or MegaMan
    /// with the save's glitch) flicker once or twice (an RNG1 draw), 12
    /// ticks each, then 20 ticks to the next check.
    fn update_emotion_window(&mut self, navi: Navi, bugged: bool) {
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
        let flickers = if navi == Navi::MEGAMAN { w.glitch || bugged } else { bugged };
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

    /// `sub_80302B6`: the same on the secondary channel.
    pub fn shake_camera_secondary(&mut self, magnitude: u16, ticks: u16) {
        for c in &mut self.consoles {
            c.shake_secondary(magnitude, ticks);
        }
    }

    /// `sub_802FFF4`'s shake, each running tick after the objects: the
    /// primary channel first unless the battle is paused without dimming
    /// while player 0's status (BattleState+0x14) has neither bit 0 nor 2
    /// (`sub_80269D0`); the secondary otherwise. (Battle flag 0x20, which
    /// nothing sets, would also let the primary shake; the battle's
    /// subsystem is always in use.)
    pub(crate) fn update_cameras(&mut self) {
        let primary_first = self.round.remote_status[0] & 5 != 0 || !self.paused || self.is_dimmed();
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
        let mode = self.setup.settings.mode;
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
    /// window (`sub_800FE52` counts the navi's NaviCust bugs).
    pub(crate) fn update_emotion_windows(&mut self) {
        for side in 0..2 {
            if !self.consoles[side].emotion_window.running {
                continue;
            }
            let stats = self.stats[side];
            self.consoles[side].update_emotion_window(stats.navi, bugs(&stats) != 0);
        }
    }

    /// The main loop's draw, once a frame after the battle's.
    pub(crate) fn end_console_frames(&mut self) {
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
        c.update_emotion_window(Navi::MEGAMAN, true);
        c.update_emotion_window(Navi::MEGAMAN, true);
        assert_eq!(c.rng, start);
        c.update_emotion_window(Navi::MEGAMAN, true);
        let mut expected = start;
        let flickers = (expected.next_positive() & 1) as u8 + 1;
        assert_eq!(c.rng, expected);
        assert_eq!(c.emotion_window.flickers, flickers);
        // 12 ticks a flicker, then 20 to the next check.
        for _ in 0..12 * flickers as u32 + 19 {
            c.update_emotion_window(Navi::MEGAMAN, true);
        }
        assert_eq!(c.rng, expected);
        c.update_emotion_window(Navi::MEGAMAN, true);
        expected.next();
        assert_eq!(c.rng, expected);
    }

    #[test]
    fn an_unbugged_navi_needs_the_glitch() {
        let mut c = console();
        c.emotion_window = EmotionWindow { running: true, timer: 1, glitch: true, ..EmotionWindow::default() };
        let start = c.rng;
        c.update_emotion_window(Navi(3), false);
        assert_eq!(c.rng, start);
        c.emotion_window.timer = 1;
        c.update_emotion_window(Navi::MEGAMAN, false);
        assert_ne!(c.rng, start);
    }
}
