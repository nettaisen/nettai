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

/// A key's bit by its name ("a", "b", "select", "start", "right", "left",
/// "up", "down", "r", "l"): what a script names keys by.
pub fn key_named(name: &str) -> Option<u16> {
    Some(match name {
        "a" => keys::A,
        "b" => keys::B,
        "select" => keys::SELECT,
        "start" => keys::START,
        "right" => keys::RIGHT,
        "left" => keys::LEFT,
        "up" => keys::UP,
        "down" => keys::DOWN,
        "r" => keys::R,
        "l" => keys::L,
        _ => return None,
    })
}

/// Held/pressed/released for one player, updated from each tick's input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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

/// A player's joypad as menus read it (the game's `eJoypad`, updated once
/// per frame by `main_static_80003E4`): held and newly pressed buttons, and
/// the held buttons that auto-repeat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Joypad {
    pub held: u16,
    /// Buttons down this frame and not the last.
    pub pressed: u16,
    /// Held buttons that repeat this frame: on the second frame of a hold,
    /// then, from the 17th frame on, every fifth frame.
    pub repeat: u16,
    /// Frames each button has been held, capped at `REPEAT_DELAY`.
    hold_frames: [u8; 10],
    /// The repeat beat: counts 0..=4, one step per frame (a console-wide
    /// counter; a held button repeats on the frames it reads 0).
    phase: u8,
}

impl Joypad {
    /// Frames of holding before a button starts repeating.
    const REPEAT_DELAY: u8 = 16;
    /// Frames between repeats.
    const REPEAT_PERIOD: u8 = 5;

    /// A joypad with nothing held whose first update runs at repeat beat
    /// `phase` (0..=4).
    pub fn new(phase: u8) -> Joypad {
        Joypad { phase: (phase + Self::REPEAT_PERIOD - 1) % Self::REPEAT_PERIOD, ..Joypad::default() }
    }

    /// The repeat beat of the latest update.
    pub fn phase(&self) -> u8 {
        self.phase
    }

    /// One frame with these buttons held (GBA bits, 0..=0x3FF).
    pub fn update(&mut self, keys: u16) {
        let keys = keys & 0x3FF;
        self.phase = (self.phase + 1) % Self::REPEAT_PERIOD;
        let old = self.held;
        let mut repeat = keys & old;
        for (bit, frames) in self.hold_frames.iter_mut().enumerate() {
            let mask = 1 << bit;
            if repeat & mask == 0 {
                *frames = 0;
            } else if *frames >= Self::REPEAT_DELAY {
                if self.phase != 0 {
                    repeat &= !mask;
                }
            } else {
                *frames += 1;
                if *frames != 1 {
                    repeat &= !mask;
                }
            }
        }
        self.held = keys;
        self.pressed = keys & !old;
        self.repeat = repeat;
    }
}

/// What one player contributes to a tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlayerTick {
    /// The buttons the player holds on this tick (GBA bits, 0..=0x3FF),
    /// which their custom screen and the fight both read.
    pub held: u16,
    /// The member of their side's party the player asks to switch to on
    /// this tick, if any (0 is the side's own navi, 1 and 2 its partners:
    /// `Battle::party`). A request is input, as the buttons are: the DS's
    /// battle packet carries it beside the keys (its +7, the member + 1,
    /// 0x020370C8), and the side's input record keeps it (+8). A side
    /// without partners has nothing to switch to, and a request of it does
    /// nothing.
    pub switch_to: Option<u8>,
}

/// The bits of a [`PlayerTick`]'s word ([`PlayerTick::word`]).
pub const TICK_WORD: u16 = 0xFFF;

impl PlayerTick {
    /// A tick of these buttons alone.
    pub const fn buttons(held: u16) -> PlayerTick {
        PlayerTick { held, switch_to: None }
    }

    /// The tick as one 16-bit word, for the hosts that carry a tick as a
    /// number (netplay's wire, replays, a frontend's input): the buttons in
    /// the low ten bits, the switch request in bits 10 and 11 (the member
    /// plus one, 0 none).
    pub const fn word(self) -> u16 {
        let switch = match self.switch_to {
            Some(m) => (m as u16 & 3) + 1,
            None => 0,
        };
        self.held & 0x3FF | (switch & 3) << 10
    }

    /// The tick a word ([`PlayerTick::word`]) holds; the bits above it are
    /// ignored. (A word's switch field of 3 names member 2.)
    pub const fn from_word(word: u16) -> PlayerTick {
        let switch = (word >> 10) & 3;
        PlayerTick { held: word & 0x3FF, switch_to: if switch == 0 { None } else { Some(switch as u8 - 1) } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joypad_repeats_on_the_second_frame_then_every_fifth() {
        // Hold A from the first frame, which runs at beat 3.
        let mut j = Joypad::new(3);
        let mut pressed = Vec::new();
        let mut repeats = Vec::new();
        for frame in 0..40 {
            j.update(keys::A);
            if j.pressed & keys::A != 0 {
                pressed.push(frame);
            }
            if j.repeat & keys::A != 0 {
                repeats.push(frame);
            }
        }
        assert_eq!(pressed, [0]);
        // Frame 1, then from frame 16 on the frames at beat 0 (frame f is
        // at beat (3 + f) % 5).
        assert_eq!(repeats, [1, 17, 22, 27, 32, 37]);
        j.update(0);
        assert_eq!((j.held, j.pressed, j.repeat), (0, 0, 0));
    }
}
