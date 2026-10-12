//! The player's hands: the keyboard and a gamepad, as the GBA's buttons for the battle and as actions for the
//! menus (docs/app.md §1, "The keys", and §3).
//!
//! The keys come from the window's events before Slint sees them (`main`'s winit hook), read by where they are
//! on the keyboard, whatever its layout, and timestamped as they arrive. The battle's buttons are the player's
//! bindings ([`Bindings`], kept in the settings); the menus' keys are fixed (the arrows, Enter, Esc, Tab), with
//! the keys bound to L and R as their previous and next. A gamepad is read through gamepad-facade and polled
//! each frame ([`Pad::poll`]): its bound buttons for the battle, its presses as the menus' actions.

use gamepad_facade::{Axis, Button, EventKind};
use nettai_battle::input::keys;
use slint::winit_030::winit::event::{ElementState, KeyEvent};
use slint::winit_030::winit::keyboard::{KeyCode, PhysicalKey};
use std::time::{Duration, Instant};

/// The GBA's buttons, in the order the settings list them, each with its name in the settings' file.
pub const BUTTONS: [(u16, &str); 10] = [
    (keys::UP, "up"),
    (keys::DOWN, "down"),
    (keys::LEFT, "left"),
    (keys::RIGHT, "right"),
    (keys::A, "a"),
    (keys::B, "b"),
    (keys::L, "l"),
    (keys::R, "r"),
    (keys::START, "start"),
    (keys::SELECT, "select"),
];

/// A GBA button's place in [`BUTTONS`].
pub fn button_index(bit: u16) -> usize {
    BUTTONS.iter().position(|(b, _)| *b == bit).unwrap_or(0)
}

/// What a key or a pad's button asks of the menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    /// Enter; the pad's south button.
    Confirm,
    /// Esc; the pad's east button.
    Back,
    /// Tab and Shift+Tab.
    NextFocus,
    PreviousFocus,
    /// The keys bound to L and R; the pad's shoulders: the game before and after, a tab before and after.
    Previous,
    Next,
    /// Delete or Backspace; the pad's west button.
    Delete,
    /// The pad's north button (a screen's second action).
    Alternate,
    /// The pad's Start: nettai's menu in a battle; Enter's in a menu.
    Menu,
    PageUp,
    PageDown,
    Home,
    End,
}

/// The keys that can be bound, each with its name in the settings' file and its cap in the UI.
const KEYS: &[(KeyCode, &str, &str)] = &[
    (KeyCode::ArrowUp, "ArrowUp", "↑"),
    (KeyCode::ArrowDown, "ArrowDown", "↓"),
    (KeyCode::ArrowLeft, "ArrowLeft", "←"),
    (KeyCode::ArrowRight, "ArrowRight", "→"),
    (KeyCode::KeyA, "KeyA", "A"),
    (KeyCode::KeyB, "KeyB", "B"),
    (KeyCode::KeyC, "KeyC", "C"),
    (KeyCode::KeyD, "KeyD", "D"),
    (KeyCode::KeyE, "KeyE", "E"),
    (KeyCode::KeyF, "KeyF", "F"),
    (KeyCode::KeyG, "KeyG", "G"),
    (KeyCode::KeyH, "KeyH", "H"),
    (KeyCode::KeyI, "KeyI", "I"),
    (KeyCode::KeyJ, "KeyJ", "J"),
    (KeyCode::KeyK, "KeyK", "K"),
    (KeyCode::KeyL, "KeyL", "L"),
    (KeyCode::KeyM, "KeyM", "M"),
    (KeyCode::KeyN, "KeyN", "N"),
    (KeyCode::KeyO, "KeyO", "O"),
    (KeyCode::KeyP, "KeyP", "P"),
    (KeyCode::KeyQ, "KeyQ", "Q"),
    (KeyCode::KeyR, "KeyR", "R"),
    (KeyCode::KeyS, "KeyS", "S"),
    (KeyCode::KeyT, "KeyT", "T"),
    (KeyCode::KeyU, "KeyU", "U"),
    (KeyCode::KeyV, "KeyV", "V"),
    (KeyCode::KeyW, "KeyW", "W"),
    (KeyCode::KeyX, "KeyX", "X"),
    (KeyCode::KeyY, "KeyY", "Y"),
    (KeyCode::KeyZ, "KeyZ", "Z"),
    (KeyCode::Digit0, "Digit0", "0"),
    (KeyCode::Digit1, "Digit1", "1"),
    (KeyCode::Digit2, "Digit2", "2"),
    (KeyCode::Digit3, "Digit3", "3"),
    (KeyCode::Digit4, "Digit4", "4"),
    (KeyCode::Digit5, "Digit5", "5"),
    (KeyCode::Digit6, "Digit6", "6"),
    (KeyCode::Digit7, "Digit7", "7"),
    (KeyCode::Digit8, "Digit8", "8"),
    (KeyCode::Digit9, "Digit9", "9"),
    (KeyCode::Enter, "Enter", "Enter"),
    (KeyCode::NumpadEnter, "NumpadEnter", "Num Enter"),
    (KeyCode::Space, "Space", "Space"),
    (KeyCode::Backspace, "Backspace", "Backspace"),
    (KeyCode::ShiftLeft, "ShiftLeft", "L Shift"),
    (KeyCode::ShiftRight, "ShiftRight", "R Shift"),
    (KeyCode::ControlLeft, "ControlLeft", "L Ctrl"),
    (KeyCode::ControlRight, "ControlRight", "R Ctrl"),
    (KeyCode::AltLeft, "AltLeft", "L Alt"),
    (KeyCode::AltRight, "AltRight", "R Alt"),
    (KeyCode::Comma, "Comma", ","),
    (KeyCode::Period, "Period", "."),
    (KeyCode::Slash, "Slash", "/"),
    (KeyCode::Semicolon, "Semicolon", ";"),
    (KeyCode::Quote, "Quote", "'"),
    (KeyCode::BracketLeft, "BracketLeft", "["),
    (KeyCode::BracketRight, "BracketRight", "]"),
    (KeyCode::Backslash, "Backslash", "\\"),
    (KeyCode::Minus, "Minus", "-"),
    (KeyCode::Equal, "Equal", "="),
    (KeyCode::Backquote, "Backquote", "`"),
    (KeyCode::Numpad0, "Numpad0", "Num 0"),
    (KeyCode::Numpad1, "Numpad1", "Num 1"),
    (KeyCode::Numpad2, "Numpad2", "Num 2"),
    (KeyCode::Numpad3, "Numpad3", "Num 3"),
    (KeyCode::Numpad4, "Numpad4", "Num 4"),
    (KeyCode::Numpad5, "Numpad5", "Num 5"),
    (KeyCode::Numpad6, "Numpad6", "Num 6"),
    (KeyCode::Numpad7, "Numpad7", "Num 7"),
    (KeyCode::Numpad8, "Numpad8", "Num 8"),
    (KeyCode::Numpad9, "Numpad9", "Num 9"),
];

/// The pad's buttons that can be bound, each with its name in the settings' file and its cap in the UI (a face
/// button is drawn by its place, so it has none).
const PAD_BUTTONS: &[(Button, &str, &str)] = &[
    (Button::South, "south", ""),
    (Button::East, "east", ""),
    (Button::West, "west", ""),
    (Button::North, "north", ""),
    (Button::LeftShoulder, "left-shoulder", "L"),
    (Button::RightShoulder, "right-shoulder", "R"),
    (Button::Start, "start", "START"),
    (Button::Back, "back", "SELECT"),
    (Button::DPadUp, "dpad-up", "↑"),
    (Button::DPadDown, "dpad-down", "↓"),
    (Button::DPadLeft, "dpad-left", "←"),
    (Button::DPadRight, "dpad-right", "→"),
    (Button::LeftStick, "left-stick", "L3"),
    (Button::RightStick, "right-stick", "R3"),
    (Button::LeftPaddle1, "left-paddle-1", "P1"),
    (Button::RightPaddle1, "right-paddle-1", "P2"),
    (Button::LeftPaddle2, "left-paddle-2", "P3"),
    (Button::RightPaddle2, "right-paddle-2", "P4"),
];

/// A bindable key by its name in the settings' file.
pub fn key_named(name: &str) -> Option<KeyCode> {
    KEYS.iter().find(|(_, n, _)| *n == name).map(|(k, _, _)| *k)
}

/// A key's name in the settings' file, if it can be bound.
pub fn key_name(key: KeyCode) -> Option<&'static str> {
    KEYS.iter().find(|(k, _, _)| *k == key).map(|(_, n, _)| *n)
}

/// A key's cap, as the UI shows it.
pub fn key_cap(key: KeyCode) -> &'static str {
    KEYS.iter().find(|(k, _, _)| *k == key).map_or("?", |(_, _, c)| *c)
}

pub fn pad_named(name: &str) -> Option<Button> {
    PAD_BUTTONS.iter().find(|(_, n, _)| *n == name).map(|(b, _, _)| *b)
}

pub fn pad_name(button: Button) -> Option<&'static str> {
    PAD_BUTTONS.iter().find(|(b, _, _)| *b == button).map(|(_, n, _)| *n)
}

/// A pad button's cap (empty for a face button, which is drawn by its place).
pub fn pad_cap(button: Button) -> &'static str {
    PAD_BUTTONS.iter().find(|(b, _, _)| *b == button).map_or("?", |(_, _, c)| *c)
}

/// The battle's buttons: the key and the pad button of each GBA button, in [`BUTTONS`]' order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bindings {
    pub keys: [KeyCode; 10],
    pub pad: [Button; 10],
}

impl Default for Bindings {
    /// The arrows move; Z is A, X is B, A is L, S is R; Enter is START, Backspace is SELECT. On a pad: the
    /// D-pad, the south button A, the west button B, the shoulders, Start and Back.
    fn default() -> Bindings {
        Bindings {
            keys: [
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
                KeyCode::ArrowLeft,
                KeyCode::ArrowRight,
                KeyCode::KeyZ,
                KeyCode::KeyX,
                KeyCode::KeyA,
                KeyCode::KeyS,
                KeyCode::Enter,
                KeyCode::Backspace,
            ],
            pad: [
                Button::DPadUp,
                Button::DPadDown,
                Button::DPadLeft,
                Button::DPadRight,
                Button::South,
                Button::West,
                Button::LeftShoulder,
                Button::RightShoulder,
                Button::Start,
                Button::Back,
            ],
        }
    }
}

impl Bindings {
    /// The GBA button a key is bound to.
    pub fn of_key(&self, key: KeyCode) -> Option<u16> {
        self.keys.iter().position(|k| *k == key).map(|i| BUTTONS[i].0)
    }

    fn of_pad(&self, button: Button) -> u16 {
        self.pad.iter().position(|b| *b == button).map_or(0, |i| BUTTONS[i].0)
    }

    /// The key bound to a GBA button.
    pub fn key(&self, bit: u16) -> KeyCode {
        self.keys[button_index(bit)]
    }

    /// Bind `key` to the GBA button at `index`; the button that had it takes the key this one had.
    pub fn bind_key(&mut self, index: usize, key: KeyCode) {
        if let Some(other) = self.keys.iter().position(|k| *k == key) {
            self.keys[other] = self.keys[index];
        }
        self.keys[index] = key;
    }

    /// Bind a pad's `button` to the GBA button at `index`, as [`Bindings::bind_key`].
    pub fn bind_pad(&mut self, index: usize, button: Button) {
        if let Some(other) = self.pad.iter().position(|b| *b == button) {
            self.pad[other] = self.pad[index];
        }
        self.pad[index] = button;
    }
}

/// A key pressed, as a screen may want it for itself (`Screen::key`): where it is on the keyboard, what it
/// types, and whether it is the system's repeat of a key held.
pub struct KeyPress {
    pub code: KeyCode,
    pub text: Option<String>,
    pub repeat: bool,
    pub shift: bool,
}

/// The menus' fixed keys.
pub fn menu_action(code: KeyCode, shift: bool) -> Option<Action> {
    Some(match code {
        KeyCode::ArrowUp => Action::Up,
        KeyCode::ArrowDown => Action::Down,
        KeyCode::ArrowLeft => Action::Left,
        KeyCode::ArrowRight => Action::Right,
        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => Action::Confirm,
        KeyCode::Escape => Action::Back,
        KeyCode::Tab if shift => Action::PreviousFocus,
        KeyCode::Tab => Action::NextFocus,
        KeyCode::Delete | KeyCode::Backspace => Action::Delete,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Home => Action::Home,
        KeyCode::End => Action::End,
        _ => return None,
    })
}

/// The keys held, as GBA buttons, and when each press came (for the latency figures).
#[derive(Default)]
pub struct Keys {
    pub held: u16,
    /// The presses since the last tick, when each came.
    pub pressed: Vec<Instant>,
}

impl Keys {
    /// A key event of the window: the GBA button it is bound to, held or let go. (The buttons are followed
    /// whatever has the keys, so none sticks.) The button, if it was newly pressed.
    pub fn event(&mut self, bindings: &Bindings, event: &KeyEvent, at: Instant) -> Option<u16> {
        let PhysicalKey::Code(code) = event.physical_key else { return None };
        let bit = bindings.of_key(code)?;
        if event.state != ElementState::Pressed {
            self.held &= !bit;
            return None;
        }
        let fresh = !event.repeat && self.held & bit == 0;
        if fresh {
            self.pressed.push(at);
        }
        self.held |= bit;
        fresh.then_some(bit)
    }

    /// Every key let go (the window lost the keyboard, the bindings changed).
    pub fn release(&mut self) {
        self.held = 0;
    }
}

/// How long a direction is held before it repeats in the menus, and how often it does then.
const REPEAT_AFTER: Duration = Duration::from_millis(380);
const REPEAT_EVERY: Duration = Duration::from_millis(90);

/// What the pads did since the last poll.
#[derive(Default)]
pub struct PadEvents {
    /// The menus' actions, in order.
    pub actions: Vec<Action>,
    /// The buttons pressed (the settings' binding takes the first).
    pub buttons: Vec<Button>,
    /// The GBA buttons newly pressed, as bound.
    pub pressed: u16,
    /// When the first press came.
    pub at: Option<Instant>,
}

/// The gamepads, through gamepad-facade. Every pad plays the one player.
#[derive(Default)]
pub struct Pad {
    /// The buttons held, as pad buttons (the bindings may change under them).
    down: Vec<Button>,
    /// The directions the left stick holds, as GBA buttons.
    stick: u16,
    stick_at: (f32, f32),
    /// The direction held in the menus, since when, and when it last repeated.
    repeat: Option<(Action, Instant, Instant)>,
}

impl Pad {
    /// Start reading gamepads: on the thread that polls them (the UI's).
    pub fn start() -> Pad {
        gamepad_facade::init("nettai");
        Pad::default()
    }

    /// The GBA buttons the pads hold, as `bindings` has them; the left stick moves whatever is bound.
    pub fn held(&self, bindings: &Bindings) -> u16 {
        self.down.iter().fold(self.stick, |held, b| held | bindings.of_pad(*b))
    }

    /// Take what the pads did since the last poll.
    pub fn poll(&mut self, bindings: &Bindings, now: Instant) -> PadEvents {
        let mut out = PadEvents::default();
        while let Some(event) = gamepad_facade::next_event() {
            match event.kind {
                EventKind::ButtonDown(button) => {
                    out.at.get_or_insert(now);
                    if !self.down.contains(&button) {
                        self.down.push(button);
                    }
                    out.buttons.push(button);
                    out.pressed |= bindings.of_pad(button);
                    let action = match button {
                        Button::DPadUp => Action::Up,
                        Button::DPadDown => Action::Down,
                        Button::DPadLeft => Action::Left,
                        Button::DPadRight => Action::Right,
                        Button::South => Action::Confirm,
                        Button::East => Action::Back,
                        Button::West => Action::Delete,
                        Button::North => Action::Alternate,
                        Button::LeftShoulder => Action::Previous,
                        Button::RightShoulder => Action::Next,
                        Button::Start => Action::Menu,
                        _ => continue,
                    };
                    out.actions.push(action);
                    if matches!(action, Action::Up | Action::Down | Action::Left | Action::Right) {
                        self.repeat = Some((action, now, now));
                    }
                }
                EventKind::ButtonUp(button) => {
                    self.down.retain(|b| *b != button);
                    if matches!(button, Button::DPadUp | Button::DPadDown | Button::DPadLeft | Button::DPadRight) {
                        self.repeat = None;
                    }
                }
                EventKind::AxisMotion { axis, value } => {
                    match axis {
                        Axis::LeftX => self.stick_at.0 = value,
                        // (Up is negative.)
                        Axis::LeftY => self.stick_at.1 = value,
                        _ => continue,
                    }
                    let was = self.stick;
                    self.stick = 0;
                    for (at, minus, plus) in [(self.stick_at.0, keys::LEFT, keys::RIGHT), (self.stick_at.1, keys::UP, keys::DOWN)] {
                        if at < -0.5 {
                            self.stick |= minus;
                        } else if at > 0.5 {
                            self.stick |= plus;
                        }
                    }
                    let fresh = self.stick & !was;
                    if fresh != 0 {
                        out.at.get_or_insert(now);
                        out.pressed |= fresh;
                        let action = match fresh {
                            keys::UP => Action::Up,
                            keys::DOWN => Action::Down,
                            keys::LEFT => Action::Left,
                            _ => Action::Right,
                        };
                        out.actions.push(action);
                        self.repeat = Some((action, now, now));
                    } else if self.stick == 0 && was != 0 {
                        self.repeat = None;
                    }
                }
                EventKind::Disconnected => {
                    self.down.clear();
                    self.stick = 0;
                    self.stick_at = (0.0, 0.0);
                    self.repeat = None;
                }
                EventKind::Connected => {}
            }
        }
        // A direction held repeats, as a key does.
        if let Some((action, since, last)) = &mut self.repeat
            && now.duration_since(*since) >= REPEAT_AFTER
            && now.duration_since(*last) >= REPEAT_EVERY
        {
            *last = now;
            out.actions.push(*action);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key bound to another button's key swaps the two, so every button keeps a key of its own.
    #[test]
    fn binding_a_taken_key_swaps() {
        let mut b = Bindings::default();
        b.bind_key(button_index(keys::A), KeyCode::KeyX);
        assert_eq!(b.key(keys::A), KeyCode::KeyX);
        assert_eq!(b.key(keys::B), KeyCode::KeyZ);
        assert_eq!(b.of_key(KeyCode::KeyX), Some(keys::A));
        b.bind_pad(button_index(keys::B), Button::South);
        assert_eq!((b.pad[button_index(keys::B)], b.pad[button_index(keys::A)]), (Button::South, Button::West));
    }

    /// Every bindable key and pad button reads back from its name in the settings' file.
    #[test]
    fn names_read_back() {
        for (key, name, _) in KEYS {
            assert_eq!(key_named(name), Some(*key));
        }
        for (button, name, _) in PAD_BUTTONS {
            assert_eq!(pad_named(name), Some(*button));
        }
        let b = Bindings::default();
        assert!(b.keys.iter().all(|k| key_name(*k).is_some()) && b.pad.iter().all(|p| pad_name(*p).is_some()));
    }
}
