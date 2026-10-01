//! What a player's custom screen shows that the simulation never reads:
//! the counters its window and sprites animate by, and the screen fades
//! its console runs (presentation; the state digest leaves it out, like
//! `Look`). A frontend draws the screen from this and the `Screen` itself
//! (bn6-frontend `custom`). See docs/engine/custom-screen.md §9.

use crate::battle::{Fade, FadeMode};

/// The original's presentation state of a screen (the control block at
/// `0x020364C0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenLook {
    /// `+0x40`: while the window slides in or out its offset (0x78 off the
    /// screen, 0 in place); while the chips are chosen a frame counter, which
    /// the cursor and the Regular chip's frame blink by; the sub-screens'
    /// own timers otherwise.
    pub frame: u32,
    /// `+0xF`: the emblem's spin after a pick (0 at rest, else its step,
    /// up to 0x14).
    pub spin: u8,
    /// The emblem's affine matrix as last set (`sub_802FE7A`): its angle
    /// and scale (`byte_8029CAC`).
    pub emblem_matrix: (u8, u8),
    /// The late turns' block is on the window (`sub_8029D34` draws it,
    /// `sub_8029D80` takes it off).
    pub turn_limit: bool,
    /// The Regular chip's frame the sprite's tiles hold (`sub_802899C`
    /// copies one every 8 frames).
    pub regular_frame: u8,
    /// The screen fade the screen runs on its console (Beast Out's, the
    /// Program Advance's).
    pub fade: Fade,
    /// What this tick drew.
    pub drawn: Drawn,
    /// The battle's last turns have come (`sub_800A97A`).
    pub late_turns: bool,
}

/// The sprites a tick of the screen queued.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drawn {
    /// The cursor (`sub_8028820`), in its first or second frame.
    pub cursor: Option<u8>,
    /// The emblem (`sub_8029C08`): the window's offset it was drawn at, and
    /// the spin it was drawn with.
    pub emblem: Option<(u32, u8)>,
    /// The Regular chip's frame (`sub_802899C`).
    pub regular: bool,
}

/// The emblem's spin (`byte_8029CAC`): per step, the angle and the scale
/// `sub_802FE7A` sets.
const SPIN: [(u8, u8); 19] = [
    (0x00, 0x40),
    (0x20, 0x3C),
    (0x40, 0x3A),
    (0x60, 0x38),
    (0x80, 0x37),
    (0xA0, 0x36),
    (0xB0, 0x36),
    (0xC0, 0x36),
    (0xD0, 0x37),
    (0xD8, 0x38),
    (0xE0, 0x39),
    (0xE8, 0x3A),
    (0xEC, 0x3B),
    (0xF0, 0x3C),
    (0xF4, 0x3D),
    (0xF8, 0x3E),
    (0xFB, 0x3F),
    (0xFE, 0x40),
    (0x00, 0x40),
];
/// The window's offset past which the emblem isn't drawn.
const EMBLEM_HIDDEN_PAST: u32 = 0x67;
/// The spin's last step.
const SPIN_STEPS: u8 = 0x14;

impl ScreenLook {
    pub fn new(late_turns: bool) -> ScreenLook {
        ScreenLook {
            frame: 0,
            spin: 0,
            // sub_8026A50: the emblem's matrix starts unrotated, at scale 1.
            emblem_matrix: (0, 0x40),
            turn_limit: false,
            regular_frame: 0,
            fade: Fade { mode: FadeMode::BeastOutBack, level: 0, speed: 0, target: 0, active: false, stepped: false },
            drawn: Drawn::default(),
            late_turns,
        }
    }

    /// `sub_8029C08`: the emblem over the picked column, at the window's
    /// offset `x` (Beast Out's states pass 0), not drawn while the window
    /// is further out than 0x67; its spin steps on.
    pub(crate) fn draw_emblem(&mut self, x: u32) {
        if x > EMBLEM_HIDDEN_PAST {
            return;
        }
        self.drawn.emblem = Some((x, self.spin));
        if self.spin == 0 {
            return;
        }
        let step = self.spin;
        self.spin = if step >= SPIN_STEPS { 0 } else { step + 1 };
        if step < SPIN_STEPS {
            self.emblem_matrix = SPIN[step as usize - 1];
        }
    }

    /// `sub_8028820`: the cursor, in the frame the counter gives.
    pub(crate) fn draw_cursor(&mut self) {
        self.drawn.cursor = Some(((self.frame >> 3) & 1) as u8);
    }

    /// `sub_802899C`: the Regular chip's frame while the folder still has
    /// its Regular chip, its tiles changed every 8 frames.
    pub(crate) fn draw_regular(&mut self, regular_pending: bool) {
        if !regular_pending {
            return;
        }
        if self.frame & 7 == 0 {
            self.regular_frame = ((self.frame >> 3) & 1) as u8;
        }
        self.drawn.regular = true;
    }

    /// `sub_8029D34`: in the last turns the block blinks, off 4 frames of
    /// every 32.
    pub(crate) fn draw_turn_limit(&mut self) {
        if self.late_turns {
            self.turn_limit = self.frame & 0x1F < 0x1C;
        }
    }
}

impl std::hash::Hash for ScreenLook {
    /// Presentation: left out of the state digest.
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}
