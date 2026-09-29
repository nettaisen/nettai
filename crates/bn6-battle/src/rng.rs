//! The game's random number generator.
//!
//! The battle engine uses two independent streams. `sim` (RNG2 in the
//! original) drives everything that must agree between linked players:
//! object behavior, damage variance, AI. `local` (RNG1) advances once per
//! frame and drives things each console decides for itself, such as the
//! folder shuffle, whose results are then exchanged.

/// One RNG stream: `x' = (rotl(x, 1) + 1) ^ 0x873CA9E5`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng {
    pub state: u32,
}

impl Rng {
    /// The seed the game gives the simulation stream at boot.
    pub const BOOT_SEED: u32 = 0xA338_244F;

    pub fn new(state: u32) -> Rng {
        Rng { state }
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
}
