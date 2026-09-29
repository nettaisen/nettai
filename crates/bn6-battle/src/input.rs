//! Player input as the engine consumes it.

/// GBA button bits (KEYINPUT order).
pub mod keys {
    pub const A: u16 = 1 << 0;
    pub const B: u16 = 1 << 1;
    pub const SELECT: u16 = 1 << 2;
    pub const START: u16 = 1 << 3;
    pub const RIGHT: u16 = 1 << 4;
    pub const LEFT: u16 = 1 << 5;
    pub const UP: u16 = 1 << 6;
    pub const DOWN: u16 = 1 << 7;
    pub const R: u16 = 1 << 8;
    pub const L: u16 = 1 << 9;
    /// Bits the game always sets in a live input word (it uses them to tell
    /// a real packet from an empty one).
    pub const PRESENT: u16 = 0xFC00;
}

/// Held/pressed/released for one player, updated from each tick's input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputRecord {
    pub held: u16,
    pub pressed: u16,
    pub released: u16,
}

impl InputRecord {
    /// Apply this tick's held buttons.
    pub fn update(&mut self, held: u16) {
        let old = self.held;
        self.held = held;
        self.pressed = !old & held;
        self.released = old & !held;
    }
}

/// What one player contributes to a tick.
#[derive(Clone, Debug, Default)]
pub struct PlayerTick {
    /// Buttons held (GBA bits; `keys::PRESENT` is added by the engine).
    pub held: u16,
    /// The player's custom screen is open (their local UI state, shared
    /// over the link; drives the NaviCust HP-drain bug).
    pub in_custom: bool,
}
