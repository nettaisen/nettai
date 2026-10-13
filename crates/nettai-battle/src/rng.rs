//! The game's random number generator.
//!
//! The battle engine uses two independent streams. `sim` (RNG2 in the
//! original) drives everything that must agree between linked players:
//! object behavior, damage variance, AI. `local` (RNG1) advances once per
//! frame and drives things each console decides for itself, such as the
//! folder shuffle, whose results are then exchanged.

/// One RNG stream: `x' = (rotl(x, 1) + 1) ^ 0x873CA9E5`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rng {
    pub state: u32,
}

impl Rng {
    /// The seed the game gives the simulation stream at boot.
    pub const BOOT_SEED: u32 = 0xA338_244F;

    pub fn new(state: u32) -> Rng {
        Rng { state }
    }

    /// Whether `x` is `to`, or one of the states this stream reaches on its
    /// way to `to` within `max` steps: a recording's samples of a console's
    /// stream bound what it drew between them, and a stream that drew some
    /// of a tick's picks before the sample and some after lies between two
    /// samples. (A stream that drew nothing between them is at `to`.)
    pub fn passes(self, x: u32, to: u32, max: u32) -> bool {
        if x == to {
            return true;
        }
        let mut r = self;
        for _ in 0..max {
            r.next();
            if r.state == x {
                return true;
            }
            if r.state == to {
                return false;
            }
        }
        false
    }

    /// Advance and return the new state.
    pub fn next(&mut self) -> u32 {
        self.state = self.state.rotate_left(1).wrapping_add(1) ^ 0x873C_A9E5;
        self.state
    }

    /// Advance and return the state with the sign bit cleared.
    pub fn next_positive(&mut self) -> u32 {
        self.next() & 0x7FFF_FFFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_matches_the_game() {
        // rotl(0xA338244F, 1) = 0x4670489F; + 1 = 0x467048A0; ^ 0x873CA9E5.
        let mut r = Rng::new(Rng::BOOT_SEED);
        assert_eq!(r.next(), 0x467048A0 ^ 0x873CA9E5);
    }

    #[test]
    fn a_state_between_two_samples_passes() {
        // (timpani/hit's frames 550 and 551: the main loop's pick, then the
        // next battle update's shake picks two.)
        let (from, to) = (0xED78_CDFF, 0xFE71_2DB8);
        let start = Rng::new(from);
        assert!(start.passes(0x5DCD_35E5, to, 64), "one pick on");
        assert!(start.passes(to, to, 64), "the next sample itself");
        assert!(!start.passes(from, to, 64), "no pick at all");
        assert!(!start.passes(0x7BDE_F297, to, 64), "past the next sample");
        assert!(Rng::new(from).passes(from, from, 64), "no pick and none to make");
    }
}
