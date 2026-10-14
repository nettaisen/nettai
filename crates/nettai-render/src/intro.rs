//! The HUD's entrance at a round's start: the enhanced mode's motion
//! (`Renderer::motion`), which the original has none of. Each piece of the
//! HUD comes in on its own, one after another, in whole frame pixels (the
//! HUD is drawn into the 240x160 frame):
//!
//! - the HP box slides in from the left edge as the intro's fade clears,
//!   and the emotion window after it;
//! - the HP numbers under objects pop in (a squash and stretch about their
//!   middle, then a little hop) as each shows, before the round's first
//!   custom screen;
//! - the custom gauge drops in from the top as the round's first custom
//!   screen sends, and a light sweeps along its bar;
//! - the chip icons over the navis pop in, and the next chip's name slides
//!   in from the left, as the round's first chips arrive.
//!
//! Nothing else moves, and the custom screen is the game's own.
//!
//! **Deterministic.** Where each piece is depends on the battle alone: the
//! round's ticks ([`Battle`]'s `round.ticks`, which a new round starts over),
//! the tick the local console sent its first result and the tick both
//! results were in (the custom screen's `Sent` records), and for an HP
//! number the tick it first showed, which `HudState` follows (before the
//! round's first custom screen, where no input reaches the battle: the same
//! on every timeline). Nothing reads a clock and nothing is kept in the
//! simulation, its snapshot or its digest; a rollback that draws a frame
//! again, a replay and a video draw it the same.

use crate::compose::SpritePart;
use nettai_battle::Battle;

/// When the HP box starts in: the round's tick after the intro's fade has
/// cleared (`Fade::TICKS` of it from the round's first).
const ARRIVAL: u32 = nettai_battle::battle::Fade::TICKS as u32 + 1;
/// A slide's ticks, and how far it starts off its place.
const SLIDE: u32 = 12;
const SLIDE_FROM: i32 = 56;
/// The emotion window after the HP box.
const WINDOW_AFTER: u32 = 5;
/// The gauge's drop and how far above it starts; the sweep after it lands.
const DROP: u32 = 10;
const DROP_FROM: i32 = 20;
const SWEEP: u32 = 12;
/// A pop's squash and stretch, then its hop.
const POP: u32 = 6;
const HOP: [i32; 4] = [-2, -2, -1, -1];
/// One HP number after another, and the chip icons after the chip's name.
const NUMBER_AFTER: u32 = 3;
const ICONS_AFTER: u32 = 3;

/// Where the HUD's pieces are this frame. All at rest (`Intro::REST`) where
/// the motion is off: nothing drawn changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intro {
    /// The round's ticks; none: at rest.
    now: Option<u32>,
    /// The tick the local console's first result went out (the gauge shows
    /// from the tick after), and the tick both were in (the chips show
    /// from the tick after).
    sent: Option<u32>,
    arrived: Option<u32>,
}

/// A piece that pops: hidden, or squashed (`vscale`, as the banners') and
/// lifted by `dy`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pop {
    pub hidden: bool,
    pub vscale: Option<i32>,
    pub dy: i32,
}

impl Pop {
    /// Pop a group of sprite parts.
    pub fn apply(self, group: &mut Vec<SpritePart>) {
        if self == Pop::default() {
            return;
        }
        if self.hidden {
            group.clear();
            return;
        }
        for part in group {
            part.y = (part.y as i32 + self.dy) as u8;
            if self.vscale.is_some() {
                part.vscale = self.vscale;
            }
        }
    }
}

impl Intro {
    pub const REST: Intro = Intro { now: None, sent: None, arrived: None };

    /// Where the pieces are in `b`'s frame, with the motion `on`.
    pub fn of(b: &Battle, on: bool) -> Intro {
        if !on {
            return Intro::REST;
        }
        // (The round's first custom screen: the records stay until the next
        // screen opens. Both results are in on the tick the later one's
        // last word arrives: the screen runs every tick until then.)
        let first = b.round.turn == 1;
        let sides = &b.custom.sides;
        let sent = sides[b.setup.local_side as usize & 1].sent.as_ref().filter(|_| first).map(|s| s.sent_at + 1);
        let arrived = match (&sides[0].sent, &sides[1].sent) {
            (Some(a), Some(c)) if first => Some(a.arrives.max(c.arrives) + 1),
            _ => None,
        };
        Intro { now: Some(b.round.ticks), sent, arrived }
    }

    /// Ticks since `from` (none before it, or at rest).
    fn since(&self, from: u32) -> Option<u32> {
        self.now?.checked_sub(from)
    }

    /// The HP box's move right (negative: left, off the screen).
    pub fn hp_box_dx(&self) -> i32 {
        self.slide(Some(ARRIVAL))
    }

    /// The emotion window's.
    pub fn window_dx(&self) -> i32 {
        self.slide(Some(ARRIVAL + WINDOW_AFTER))
    }

    /// The next chip's name's (and its damage's), at the bottom left.
    pub fn chip_name_dx(&self) -> i32 {
        self.slide(self.arrived)
    }

    /// The custom gauge's move down (negative: up, off the screen).
    pub fn gauge_dy(&self) -> i32 {
        self.come(self.sent, DROP_FROM, DROP)
    }

    /// The gauge's cells the sweep lights this frame (of 16, as a range).
    pub fn gauge_sweep(&self) -> std::ops::Range<i32> {
        let Some(t) = self.sent.and_then(|s| self.since(s + DROP)) else { return 0..0 };
        if t >= SWEEP {
            return 0..0;
        }
        // (Its head crosses the 16 cells and its tail of three after them.)
        let head = (t as i32 * 19) / SWEEP as i32;
        head - 2..head + 1
    }

    /// The HP number at `place`, which first showed on the round's tick
    /// `shown` before the round's first custom screen (none: later, or not
    /// followed).
    pub fn number_pop(&self, place: usize, shown: Option<u32>) -> Pop {
        self.pop(shown.map(|s| s + NUMBER_AFTER * place as u32))
    }

    /// The chip icons over the navis.
    pub fn icons_pop(&self) -> Pop {
        self.pop(self.arrived.map(|a| a + ICONS_AFTER))
    }

    /// A slide in from `SLIDE_FROM` to the left, from the tick `from`.
    fn slide(&self, from: Option<u32>) -> i32 {
        self.come(from, SLIDE_FROM, SLIDE)
    }

    /// How far back a piece is that comes `travel` pixels over `ticks` from
    /// the tick `from` (all of it before).
    fn come(&self, from: Option<u32>, travel: i32, ticks: u32) -> i32 {
        let (Some(now), Some(from)) = (self.now, from) else { return 0 };
        match now.checked_sub(from) {
            None => -travel,
            Some(t) => -arrive(travel, t, ticks),
        }
    }

    /// A pop from the tick `from`: hidden before, then squashed flat and
    /// stretched back to its height over `POP` ticks, then a hop.
    fn pop(&self, from: Option<u32>) -> Pop {
        let (Some(now), Some(from)) = (self.now, from) else { return Pop::default() };
        let Some(t) = now.checked_sub(from) else { return Pop { hidden: true, ..Pop::default() } };
        if t < POP {
            // (Its height grows (t + 1) / POP of the way, eased: the rows
            // step by 256 / height.)
            let u = ease_out((t + 1) as f64 / POP as f64);
            return Pop { hidden: false, vscale: Some((256.0 / u.max(1.0 / 16.0)).round() as i32), dy: 0 };
        }
        Pop { dy: HOP.get((t - POP) as usize).copied().unwrap_or(0), ..Pop::default() }
    }
}

/// How far a piece that comes `from` pixels away still has to go `t` ticks
/// into a move of `ticks`: eased out with a little overshoot (past its
/// place, then back), in whole pixels; 0 once done.
fn arrive(from: i32, t: u32, ticks: u32) -> i32 {
    if t >= ticks {
        return 0;
    }
    let u = (t + 1) as f64 / ticks as f64;
    (from as f64 * (1.0 - ease_out_back(u))).round() as i32
}

/// Ease out (cubic). Only the basic operations, which round alike on every
/// machine.
fn ease_out(u: f64) -> f64 {
    let v = 1.0 - u;
    1.0 - v * v * v
}

/// Ease out with an overshoot of about a tenth (the "back" curve).
fn ease_out_back(u: f64) -> f64 {
    const C1: f64 = 1.2;
    const C3: f64 = C1 + 1.0;
    let v = u - 1.0;
    1.0 + C3 * v * v * v + C1 * v * v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slide_comes_from_off_the_screen_overshoots_and_settles() {
        let at = |t: u32| -arrive(SLIDE_FROM, t, SLIDE);
        assert!(at(0) < -SLIDE_FROM / 2, "starts well out: {}", at(0));
        assert!((0..SLIDE).any(|t| at(t) > 0), "overshoots");
        assert_eq!(at(SLIDE - 1), 0);
        assert_eq!(at(SLIDE + 5), 0);
    }

    #[test]
    fn at_rest_nothing_moves() {
        let r = Intro::REST;
        assert_eq!((r.hp_box_dx(), r.window_dx(), r.chip_name_dx(), r.gauge_dy()), (0, 0, 0, 0));
        assert_eq!(r.gauge_sweep(), 0..0);
        assert_eq!(r.number_pop(0, Some(3)), Pop::default());
        assert_eq!(r.icons_pop(), Pop::default());
    }

    #[test]
    fn the_pieces_come_one_after_another_and_rest_within_600_ms() {
        let at = |now: u32| Intro { now: Some(now), sent: Some(200), arrived: Some(300) };
        assert_eq!(at(1).hp_box_dx(), -SLIDE_FROM, "out before the fade clears");
        assert!(at(ARRIVAL + 2).hp_box_dx() > at(ARRIVAL + 2).window_dx(), "the window follows the box");
        // About 36 ticks to 600 ms.
        assert_eq!(at(36).hp_box_dx(), 0);
        assert_eq!(at(36).window_dx(), 0);
        assert_eq!(at(199).gauge_dy(), -DROP_FROM);
        assert_eq!(at(200 + DROP + SWEEP).gauge_dy(), 0);
        assert_eq!(at(200 + DROP + SWEEP).gauge_sweep(), 0..0);
        assert!(at(299).icons_pop().hidden);
        assert_eq!(at(300 + 36).icons_pop(), Pop::default());
        assert_eq!(at(300 + 36).chip_name_dx(), 0);
    }
}
