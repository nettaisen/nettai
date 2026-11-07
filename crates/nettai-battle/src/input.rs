//! Player input as the engine consumes it.


bitflags::bitflags! {
    /// GBA buttons (KEYINPUT order).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Keys: u16 {
        const A = 1 << 0;
        const B = 1 << 1;
        const SELECT = 1 << 2;
        const START = 1 << 3;
        const RIGHT = 1 << 4;
        const LEFT = 1 << 5;
        const UP = 1 << 6;
        const DOWN = 1 << 7;
        const R = 1 << 8;
        const L = 1 << 9;
        /// The ten buttons.
        const BUTTONS = 0x3FF;
        /// Bits the game always sets in a live input word (it uses them to tell
        /// a real packet from an empty one); no button has them.
        const PRESENT = 0xFC00;
        const _ = !0;
    }
}

impl Keys {
    /// The button of KEYINPUT bit `bit` (0..=9).
    pub const fn of_bit(bit: usize) -> Keys {
        Keys::from_bits_retain(1 << bit)
    }
}

/// A key's bit by its name ("a", "b", "select", "start", "right", "left",
/// "up", "down", "r", "l"): what a script names keys by.
pub fn key_named(name: &str) -> Option<Keys> {
    Some(match name {
        "a" => Keys::A,
        "b" => Keys::B,
        "select" => Keys::SELECT,
        "start" => Keys::START,
        "right" => Keys::RIGHT,
        "left" => Keys::LEFT,
        "up" => Keys::UP,
        "down" => Keys::DOWN,
        "r" => Keys::R,
        "l" => Keys::L,
        _ => return None,
    })
}

/// Held/pressed/released for one player, updated from each tick's input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct InputRecord {
    pub held: Keys,
    pub pressed: Keys,
    pub released: Keys,
}

impl InputRecord {
    /// Apply this tick's held buttons.
    pub fn update(&mut self, held: Keys) {
        let old = self.held;
        self.held = held;
        self.pressed = held - old;
        self.released = old - held;
    }
}

/// A player's joypad as menus read it (the game's `eJoypad`, updated once
/// per frame by `main_static_80003E4`): held and newly pressed buttons, and
/// the held buttons that auto-repeat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Joypad {
    pub held: Keys,
    /// Buttons down this frame and not the last.
    pub pressed: Keys,
    /// Held buttons that repeat this frame: on the second frame of a hold,
    /// then, from the 17th frame on, every fifth frame.
    pub repeat: Keys,
    /// Frames each button has been held, capped at `REPEAT_DELAY`.
    hold_frames: [u8; 10],
    /// The repeat beat: counts 0..=4, one step per frame (a console-wide
    /// counter; a held button repeats on the frames it reads 0).
    phase: u8,
}

impl Joypad {
    /// Frames of holding before a button starts repeating.
    pub const REPEAT_DELAY: u8 = 16;
    /// Frames between repeats.
    pub const REPEAT_PERIOD: u8 = 5;

    /// A joypad with nothing held whose first update runs at repeat beat
    /// `phase` (0..=4).
    pub fn new(phase: u8) -> Joypad {
        Joypad { phase: (phase + Self::REPEAT_PERIOD - 1) % Self::REPEAT_PERIOD, ..Joypad::default() }
    }

    /// The repeat beat of the latest update.
    pub fn phase(&self) -> u8 {
        self.phase
    }

    /// One frame with these buttons held.
    pub fn update(&mut self, keys: Keys) {
        let keys = keys & Keys::BUTTONS;
        self.phase = (self.phase + 1) % Self::REPEAT_PERIOD;
        let old = self.held;
        let mut repeat = keys & old;
        for (bit, frames) in self.hold_frames.iter_mut().enumerate() {
            let button = Keys::of_bit(bit);
            if !repeat.contains(button) {
                *frames = 0;
            } else if *frames >= Self::REPEAT_DELAY {
                if self.phase != 0 {
                    repeat.remove(button);
                }
            } else {
                *frames += 1;
                if *frames != 1 {
                    repeat.remove(button);
                }
            }
        }
        self.held = keys;
        self.pressed = keys - old;
        self.repeat = repeat;
    }
}

/// A decision a player takes on their custom screen, which the screen takes
/// on the tick it comes in while it chooses (docs/design/custom-screen-actions.md
/// §2). Where the cursor is, what it hovers and what only shows (R's
/// description, L's message, SELECT's hiding, the form list's own cursor) are
/// the player's own screen layer's (`custom::layer`), outside the
/// simulation: the screen takes these alone. Each is checked as A on its slot
/// was: one that can't be taken now plays the refusal, or (a slot of another
/// kind) does nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenChoice {
    /// The chip in a slot (0-11: a dealt chip, a link navi's own, one the
    /// rules offer) picked.
    Pick(u8),
    /// The rules' button on a slot (0-11, either cell of a button two wide)
    /// pressed: its `pressed` runs.
    Press(u8),
    /// OK, with what is picked (nothing picked too).
    Ok,
    /// B: the last pick taken back, or what the rules take back with none
    /// (`custom.unchoose_form`), in the screen's order.
    TakeBack,
    /// An entry (0-4) of the rules' form list chosen (their
    /// `custom.choose_form`: EXE6's Cross).
    ChooseForm(u8),
    /// The pick in this place (0-4) taken back, the picks after it moving
    /// up a place, and what it carries with it (a button attached to it, a
    /// chip a button was traded for: back in its place where it goes with
    /// the picks left, else back to the hand); B on the last pick.
    Unpick(u8),
    /// The form the rules' list chose un-chosen (`custom.unchoose_form`),
    /// the picks kept: B with nothing picked, with picks too.
    Unchoose,
    /// The chip the rules' button on a slot (0-11, either cell) holds
    /// (`custom.hold_last_pick`) released: back as a pick in the place it
    /// left where it goes with the picks, else back to the hand.
    Release(u8),
}

impl ScreenChoice {
    /// How many numbers the choices take, none's 0 with them.
    pub const CODES: u32 = 50;

    /// The choice's number (none is 0), below [`ScreenChoice::CODES`].
    pub const fn code(self) -> u32 {
        match self {
            ScreenChoice::Pick(s) => 1 + (s as u32 % 12),
            ScreenChoice::Press(s) => 13 + (s as u32 % 12),
            ScreenChoice::Ok => 25,
            ScreenChoice::TakeBack => 26,
            ScreenChoice::ChooseForm(e) => 27 + (e as u32 % 5),
            ScreenChoice::Unpick(k) => 32 + (k as u32 % 5),
            ScreenChoice::Unchoose => 37,
            ScreenChoice::Release(s) => 38 + (s as u32 % 12),
        }
    }

    /// The choice a number names; none for 0 and the numbers no choice has.
    pub const fn from_code(code: u32) -> Option<ScreenChoice> {
        Some(match code {
            1..=12 => ScreenChoice::Pick(code as u8 - 1),
            13..=24 => ScreenChoice::Press(code as u8 - 13),
            25 => ScreenChoice::Ok,
            26 => ScreenChoice::TakeBack,
            27..=31 => ScreenChoice::ChooseForm(code as u8 - 27),
            32..=36 => ScreenChoice::Unpick(code as u8 - 32),
            37 => ScreenChoice::Unchoose,
            38..=49 => ScreenChoice::Release(code as u8 - 38),
            _ => return None,
        })
    }

    /// A choice by its name, as a script names it (nettai-tool's `--keys`):
    /// `pick3`, `press11`, `ok`, `take_back`, `form2`, `unpick1`,
    /// `unchoose`, `release8`.
    pub fn named(name: &str) -> Option<ScreenChoice> {
        let number = |prefix: &str, max: u8| name.strip_prefix(prefix).and_then(|n| n.parse::<u8>().ok()).filter(|&n| n <= max);
        match name {
            "ok" => Some(ScreenChoice::Ok),
            "take_back" => Some(ScreenChoice::TakeBack),
            "unchoose" => Some(ScreenChoice::Unchoose),
            _ => number("pick", 11)
                .map(ScreenChoice::Pick)
                .or_else(|| number("press", 11).map(ScreenChoice::Press))
                .or_else(|| number("form", 4).map(ScreenChoice::ChooseForm))
                .or_else(|| number("unpick", 4).map(ScreenChoice::Unpick))
                .or_else(|| number("release", 11).map(ScreenChoice::Release)),
        }
    }
}

const _: () = assert!(ScreenChoice::CODES <= 1 << 6);

/// What one player contributes to a tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlayerTick {
    /// The buttons the player holds on this tick (`Keys::BUTTONS`), which
    /// the fight reads. Their custom screen takes their choices
    /// instead (`choice`); the fight's record still takes the buttons
    /// through it, so a button held across the screen's close isn't a new
    /// press.
    pub held: Keys,
    /// The member of their side's party the player asks to switch to on
    /// this tick, if any (0 is the side's own navi, 1 and 2 its partners:
    /// `Battle::party`). A request is input, as the buttons are: the DS's
    /// battle packet carries it beside the keys (its +7, the member + 1,
    /// 0x020370C8), and the side's input record keeps it (+8). A side
    /// without partners has nothing to switch to, and a request of it does
    /// nothing.
    pub switch_to: Option<u8>,
    /// The player's microphone heard them at full volume on this tick; else
    /// it heard nothing. The DS's battle packet carries a level, 0 to 255
    /// (its +5: the average of its sampler's last loud burst, 0x0215F208
    /// +0x3E, which its input record keeps at +9): nettai's input is one
    /// bit of it, silent or 255 (the user: "microphone input can be 1 bit:
    /// max volume or silent"). A game's rules may hear it (`mic_heard`).
    /// Nothing in nettai sets it: every match and replay is silent.
    pub mic_loud: bool,
    /// What the player chose on their custom screen this tick, if anything
    /// ([`ScreenChoice`]): taken if the screen chooses as it comes in,
    /// else nothing.
    pub choice: Option<ScreenChoice>,
}

impl PlayerTick {
    /// A tick of these buttons alone.
    pub const fn buttons(held: Keys) -> PlayerTick {
        PlayerTick { held, switch_to: None, mic_loud: false, choice: None }
    }

    /// The tick with this choice besides.
    pub const fn with_choice(self, choice: Option<ScreenChoice>) -> PlayerTick {
        PlayerTick { choice, ..self }
    }

    /// The tick as one word, for the hosts that carry a tick as a number
    /// (netplay's wire, replays, a frontend's input): the buttons in the
    /// low ten bits, the switch request in bits 10 and 11 (the member plus
    /// one, 0 none), the microphone heard loud in bit 12, and the screen's
    /// choice's number ([`ScreenChoice::code`], 0 none) in bits 13 to 18:
    /// 19 bits ([`PlayerTick::WORD_BITS`]).
    pub const fn word(self) -> u32 {
        let switch = match self.switch_to {
            Some(m) => (m as u32 & 3) + 1,
            None => 0,
        };
        let choice = match self.choice {
            Some(c) => c.code(),
            None => 0,
        };
        (self.held.bits() as u32 & Keys::BUTTONS.bits() as u32) | (switch & 3) << 10 | (self.mic_loud as u32) << 12 | (choice & 0x3F) << 13
    }

    /// The bits a word ([`PlayerTick::word`]) uses, from its lowest.
    pub const WORD_BITS: u32 = 19;

    /// The tick a word ([`PlayerTick::word`]) holds; a choice's number
    /// none has, and the bits past its 19, are ignored. (A word's switch
    /// field of 3 names member 2.)
    pub const fn from_word(word: u32) -> PlayerTick {
        let switch = (word >> 10) & 3;
        PlayerTick {
            held: Keys::from_bits_retain((word & Keys::BUTTONS.bits() as u32) as u16),
            switch_to: if switch == 0 { None } else { Some(switch as u8 - 1) },
            mic_loud: (word >> 12) & 1 != 0,
            choice: ScreenChoice::from_code((word >> 13) & 0x3F),
        }
    }

    /// The tick as a missing word of the player's is guessed from their
    /// last (netplay's prediction): its buttons and microphone carry on; a
    /// switch request and a choice are a tick's own, and happen once.
    pub const fn carried_on(self) -> PlayerTick {
        PlayerTick { switch_to: None, choice: None, ..self }
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
            j.update(Keys::A);
            if j.pressed.contains(Keys::A) {
                pressed.push(frame);
            }
            if j.repeat.contains(Keys::A) {
                repeats.push(frame);
            }
        }
        assert_eq!(pressed, [0]);
        // Frame 1, then from frame 16 on the frames at beat 0 (frame f is
        // at beat (3 + f) % 5).
        assert_eq!(repeats, [1, 17, 22, 27, 32, 37]);
        j.update(Keys::empty());
        assert!(j.held.is_empty() && j.pressed.is_empty() && j.repeat.is_empty());
    }

    #[test]
    fn a_tick_word_holds_the_buttons_the_switch_the_microphone_and_the_choice() {
        let t = |held, switch_to, mic_loud, choice| PlayerTick { held, switch_to, mic_loud, choice };
        for t in [
            PlayerTick::buttons(Keys::empty()),
            PlayerTick::buttons(Keys::BUTTONS),
            t(Keys::A, Some(0), false, None),
            t(Keys::L, Some(2), true, Some(ScreenChoice::Ok)),
            t(Keys::empty(), None, true, Some(ScreenChoice::ChooseForm(4))),
            t(Keys::A, None, false, Some(ScreenChoice::Pick(11))),
            t(Keys::empty(), Some(1), false, Some(ScreenChoice::Press(8))),
            t(Keys::empty(), None, false, Some(ScreenChoice::TakeBack)),
            t(Keys::empty(), None, false, Some(ScreenChoice::Unpick(4))),
            t(Keys::B, None, false, Some(ScreenChoice::Unchoose)),
            t(Keys::BUTTONS, Some(2), true, Some(ScreenChoice::Release(11))),
            t(Keys::BUTTONS, Some(2), true, None),
        ] {
            assert_eq!(PlayerTick::from_word(t.word()), t);
        }
        // A tick of buttons alone is its buttons: what every match and
        // replay before the party battle held.
        assert_eq!(PlayerTick::buttons(Keys::A | Keys::B).word(), (Keys::A | Keys::B).bits() as u32);
        // Every field at its most fills the word's 19 bits.
        let most = t(Keys::BUTTONS, Some(2), true, ScreenChoice::from_code(ScreenChoice::CODES - 1));
        assert_eq!(most.word() >> PlayerTick::WORD_BITS, 0);
        assert!(most.word() >= 1 << (PlayerTick::WORD_BITS - 1));
    }

    #[test]
    fn a_word_holds_every_choice() {
        for choice in (0..ScreenChoice::CODES).map(ScreenChoice::from_code) {
            let t = PlayerTick { held: Keys::BUTTONS, switch_to: Some(2), mic_loud: true, choice };
            assert_eq!(PlayerTick::from_word(t.word()), t);
        }
    }

    #[test]
    fn every_choice_has_its_own_number_and_name() {
        let all: Vec<ScreenChoice> = (0..12)
            .map(ScreenChoice::Pick)
            .chain((0..12).map(ScreenChoice::Press))
            .chain([ScreenChoice::Ok, ScreenChoice::TakeBack])
            .chain((0..5).map(ScreenChoice::ChooseForm))
            .chain((0..5).map(ScreenChoice::Unpick))
            .chain([ScreenChoice::Unchoose])
            .chain((0..12).map(ScreenChoice::Release))
            .collect();
        for (i, c) in all.iter().enumerate() {
            assert_eq!(ScreenChoice::from_code(c.code()), Some(*c));
            assert!(all[..i].iter().all(|d| d.code() != c.code()), "{c:?}");
            assert!((1..ScreenChoice::CODES).contains(&c.code()), "{c:?}");
        }
        assert_eq!(all.len() as u32 + 1, ScreenChoice::CODES);
        assert_eq!(ScreenChoice::from_code(0), None);
        assert_eq!(ScreenChoice::from_code(ScreenChoice::CODES), None);
        assert_eq!(ScreenChoice::named("unpick4"), Some(ScreenChoice::Unpick(4)));
        assert_eq!(ScreenChoice::named("unpick5"), None);
        assert_eq!(ScreenChoice::named("unchoose"), Some(ScreenChoice::Unchoose));
        assert_eq!(ScreenChoice::named("release9"), Some(ScreenChoice::Release(9)));
        assert_eq!(ScreenChoice::named("pick3"), Some(ScreenChoice::Pick(3)));
        assert_eq!(ScreenChoice::named("press11"), Some(ScreenChoice::Press(11)));
        assert_eq!(ScreenChoice::named("form2"), Some(ScreenChoice::ChooseForm(2)));
        assert_eq!(ScreenChoice::named("form5"), None);
        assert_eq!(ScreenChoice::named("take_back"), Some(ScreenChoice::TakeBack));
    }
}
