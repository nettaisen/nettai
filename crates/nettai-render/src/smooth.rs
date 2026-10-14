//! The enhanced mode's smooth motion (`Renderer::motion`): what the original
//! itself moves, in whole frame pixels or in coarse steps, drawn between
//! them at the output's resolution, over the same frames. Nothing moves
//! that the original holds still, and nothing comes in that the original
//! shows at once.
//!
//! - **The custom screen's slide** (`custom::Slide`): eased over the
//!   original's 10 ticks in and out instead of 12 pixels a tick.
//! - (The background's glide is `Frame::glide`, `present`.)
//!
//! A position between two whole pixels is drawn twice: the frame as ever,
//! and a [`Nudge`], the frame drawn again with what moves a pixel right;
//! `present` shows on each output pixel the one its place in its frame
//! pixel falls in (left of the part of a pixel the motion is past its
//! whole pixels: the nudged one). The two differ only where the moving
//! thing is, so the rest of the frame stays as it is.
//!
//! **Deterministic.** Every place is the battle's state's: the custom
//! screen's phase and tick. Nothing reads a clock and nothing is kept in the
//! simulation, its snapshot or its digest; a rollback that draws a frame
//! again, a replay and a video draw it the same. The frame's own 240x160
//! pixels (`Frame::pixels`) are a whole-pixel picture of the same, as the
//! original's are.

use crate::compose::WIDTH;
use crate::custom::Slide;
use crate::present::Placement;

/// The enhanced mode's motion in a drawing of the frame: on or off (the
/// original mode, Reduce motion), and whether this drawing is the nudged
/// one, of which motion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Smooth {
    pub on: bool,
    pub nudged: Option<Nudged>,
}

/// What a nudged drawing draws a pixel right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nudged {
    /// The custom screen's window and what moves with it.
    Slide,
}

impl Smooth {
    /// How the custom screen's slide is drawn.
    pub fn slide(self) -> Slide {
        match self.on {
            true => Slide::Smooth { nudged: self.nudged == Some(Nudged::Slide) },
            false => Slide::Stepped,
        }
    }
}

/// The frame drawn again with what moves between whole pixels a pixel
/// right, and how far past its whole pixels it is, in sixteenths of a
/// pixel: `present` shows these pixels on the output pixels whose place in
/// their frame pixel is left of `sub` (the frame's elsewhere).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Nudge {
    pub pixels: Vec<u16>,
    pub depth: Vec<u32>,
    pub sub: u8,
}

/// Whether the output column `dx` (from the frame's left edge, of
/// `place.width`) shows the nudged frame.
pub(crate) fn nudged_column(nudge: &Nudge, dx: i32, place: &Placement) -> bool {
    ((dx as i64 * 16 * WIDTH as i64 / place.width as i64) % 16) < nudge.sub as i64
}
