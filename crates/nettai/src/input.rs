//! The player's hands: the keyboard and a gamepad, as the GBA's buttons for the battle and as actions for the
//! menus (docs/app.md §1, "The keys", and §11).
//!
//! The keys come from the window's events before Slint sees them (`main`'s winit hook), read by where they are
//! on the keyboard, whatever its layout, and timestamped as they arrive. The battle's buttons are the player's
//! bindings ([`Bindings`], kept in the settings). The menus' keys are fixed (the arrows, Enter, Esc, Tab: the
//! user tried the keys bound to A and B and went back, "actually z and x for navigation is not very good,
//! enter + esc was fine"), with the keys bound to L and R as their previous and next ([`menu_action`]). A
//! gamepad is read through gamepad-facade and polled each frame ([`Pad::poll`]): its bound buttons for the
//! battle, and its presses as the menus' actions by the same bindings, the buttons bound to A and B confirming
//! and going back ([`Bindings::pad_action`]). On a phone
//! the battle's buttons are drawn under the picture, and the window's touches are theirs ([`TouchPad`]): each
//! finger by itself, so a direction and a button are held together.

use crate::nav::Rect;
use gamepad_facade::{Axis, Button, EventKind, Id};
use nettai_battle::input::keys;
use slint::winit_030::winit::event::{ElementState, KeyEvent};
use slint::winit_030::winit::keyboard::{KeyCode, PhysicalKey};
use std::time::Duration;
use web_time::Instant;

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
    /// Enter, Num Enter, Space; the pad's button bound to A (south at first).
    Confirm,
    /// Esc; the pad's button bound to B (west at first).
    Back,
    /// Tab and Shift+Tab.
    NextFocus,
    PreviousFocus,
    /// The keys and the pad's buttons bound to L and R: the game before and after, a tab before and after.
    Previous,
    Next,
    /// Delete or Backspace; the pad's face button left of A's and B's (east at first).
    Delete,
    /// The pad's north button (a screen's second action).
    Alternate,
    /// The pad's button bound to START (Start at first): it shuts the battle's menu it opened, and does nothing
    /// in the app's own menus.
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
    match key {
        // (Esc can't be bound: it is a menu's fallback for going back, and asks a binding to stop.)
        KeyCode::Escape => "Esc",
        _ => KEYS.iter().find(|(k, _, _)| *k == key).map_or("?", |(_, _, c)| *c),
    }
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

    /// The pad's button bound to a GBA button.
    pub fn pad_button(&self, bit: u16) -> Button {
        self.pad[button_index(bit)]
    }

    /// The pad's button that confirms in the menus: the one bound to A, unless that is a direction of the
    /// D-pad (the menus' own) or bound twice: then the south button.
    pub fn confirm_button(&self) -> Button {
        self.menu_button(keys::A).unwrap_or(Button::South)
    }

    /// The pad's button that goes back in the menus: the one bound to B, else the east button.
    pub fn back_button(&self) -> Button {
        self.menu_button(keys::B).unwrap_or(Button::East)
    }

    /// The pad's button that deletes in the menus (a build, a chip from its slot): the west or the east face
    /// button, whichever doesn't confirm or go back.
    pub fn delete_button(&self) -> Button {
        let taken = [self.confirm_button(), self.back_button()];
        if taken.contains(&Button::East) { Button::West } else { Button::East }
    }

    fn menu_button(&self, bit: u16) -> Option<Button> {
        let button = self.pad_button(bit);
        let direction = matches!(button, Button::DPadUp | Button::DPadDown | Button::DPadLeft | Button::DPadRight);
        (!direction && self.pad.iter().filter(|b| **b == button).count() == 1).then_some(button)
    }

    /// What a pad's button asks of the menus: the D-pad moves; the buttons bound to A, B, L, R and START are
    /// confirm, back, previous, next and the menu (`Action::Menu`); of the rest, north is a screen's second
    /// action and the face button left of A and B deletes.
    pub fn pad_action(&self, button: Button) -> Option<Action> {
        Some(match button {
            Button::DPadUp => Action::Up,
            Button::DPadDown => Action::Down,
            Button::DPadLeft => Action::Left,
            Button::DPadRight => Action::Right,
            _ if button == self.confirm_button() => Action::Confirm,
            _ if button == self.back_button() => Action::Back,
            _ if button == self.delete_button() => Action::Delete,
            _ => match self.of_pad(button) {
                keys::L => Action::Previous,
                keys::R => Action::Next,
                keys::START => Action::Menu,
                _ if button == Button::North => Action::Alternate,
                _ => return None,
            },
        })
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

/// How the footer's hints draw a pad's button: a face button by its place, START and SELECT by their caps,
/// a shoulder as the shoulders.
pub fn pad_face(button: Button) -> nettai_ui::PadButton {
    use nettai_ui::PadButton;
    match button {
        Button::South => PadButton::South,
        Button::East => PadButton::East,
        Button::West => PadButton::West,
        Button::North => PadButton::North,
        Button::Start => PadButton::Start,
        Button::Back => PadButton::Select,
        Button::LeftShoulder | Button::RightShoulder => PadButton::Shoulders,
        Button::DPadUp | Button::DPadDown | Button::DPadLeft | Button::DPadRight => PadButton::Move,
        _ => PadButton::None,
    }
}

/// The menus' fixed keys: the arrows, Enter, Num Enter and Space confirm, Esc goes back, Tab and Shift+Tab,
/// Delete and Backspace, Page Up and Down, Home and End. Whatever is bound, a binding can't take them.
fn fixed_action(code: KeyCode, shift: bool) -> Option<Action> {
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

/// What a key asks of the menus: their fixed keys first ([`fixed_action`]), then the keys bound to L and R
/// as previous and next. (The keys bound to A and B are nothing to the menus: the user, "actually z and x for
/// navigation is not very good, enter + esc was fine".) A text field being typed in takes its keys before this
/// (`App::key`).
pub fn menu_action(code: KeyCode, shift: bool, bindings: &Bindings) -> Option<Action> {
    if let Some(action) = fixed_action(code, shift) {
        return Some(action);
    }
    match bindings.of_key(code) {
        Some(keys::L) => Some(Action::Previous),
        Some(keys::R) => Some(Action::Next),
        _ => None,
    }
}

/// The keys held, as GBA buttons, and when each press came (for the latency figures).
#[derive(Default)]
pub struct Keys {
    pub held: u16,
    /// The presses since the last tick, when each came.
    pub pressed: Vec<Instant>,
    /// Tab is held: in an online match, both builds show while it is (no button is bound to it).
    pub tab: bool,
}

impl Keys {
    /// A key event of the window: the GBA button it is bound to, held or let go. (The buttons are followed
    /// whatever has the keys, so none sticks.) The button, if it was newly pressed.
    #[cfg_attr(not(feature = "tour"), allow(dead_code))]
    pub fn event(&mut self, bindings: &Bindings, event: &KeyEvent, at: Instant) -> Option<u16> {
        let PhysicalKey::Code(code) = event.physical_key else { return None };
        self.key(bindings, code, event.state == ElementState::Pressed, event.repeat, at)
    }

    /// [`Keys::event`], of a key's parts.
    pub fn key(&mut self, bindings: &Bindings, code: KeyCode, pressed: bool, repeat: bool, at: Instant) -> Option<u16> {
        if code == KeyCode::Tab {
            self.tab = pressed;
        }
        let bit = bindings.of_key(code)?;
        if !pressed {
            self.held &= !bit;
            return None;
        }
        let fresh = !repeat && self.held & bit == 0;
        if fresh {
            self.pressed.push(at);
        }
        self.held |= bit;
        fresh.then_some(bit)
    }

    /// Every key let go (the window lost the keyboard, the bindings changed).
    pub fn release(&mut self) {
        self.held = 0;
        self.tab = false;
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

/// The gamepads, through gamepad-facade. Every pad plays the one player, unless practice's dummy has the second
/// ([`Pad::apart`]).
#[derive(Default)]
pub struct Pad {
    /// The buttons held, as pad buttons, by the pad holding them (the bindings may change under them).
    down: Vec<(Id, Button)>,
    /// Each pad's left stick: where it is, and the directions it holds as GBA buttons.
    sticks: Vec<(Id, (f32, f32), u16)>,
    /// The pads, in the order they were first used (a pad is known by its first event).
    pads: Vec<Id>,
    /// The second pad is practice's dummy's (its Action "a second pad"): its buttons aren't the player's, and
    /// its presses don't reach the menus.
    pub apart: bool,
    /// The direction held in the menus, since when, and when it last repeated.
    repeat: Option<(Action, Instant, Instant)>,
}

impl Pad {
    /// Start reading gamepads: on the thread that polls them (the UI's).
    pub fn start() -> Pad {
        gamepad_facade::init("nettai");
        Pad::default()
    }

    /// Whether `id` is the dummy's pad: the second used, while it is apart.
    fn dummys(&self, id: Id) -> bool {
        self.apart && self.pads.get(1) == Some(&id)
    }

    /// What the pads `of` hold, as `bindings` has them; the left stick moves whatever is bound.
    fn held_by(&self, bindings: &Bindings, of: impl Fn(Id) -> bool) -> u16 {
        let sticks = self.sticks.iter().filter(|(id, ..)| of(*id)).fold(0, |held, (.., at)| held | at);
        self.down.iter().filter(|(id, _)| of(*id)).fold(sticks, |held, (_, b)| held | bindings.of_pad(*b))
    }

    /// The GBA buttons the player's pads hold, as `bindings` has them; the left stick moves whatever is bound.
    pub fn held(&self, bindings: &Bindings) -> u16 {
        self.held_by(bindings, |id| !self.dummys(id))
    }

    /// The GBA buttons practice's dummy's pad holds, while there is one (the second used, while it is apart).
    pub fn second(&self, bindings: &Bindings) -> Option<u16> {
        let id = *self.pads.get(1).filter(|_| self.apart)?;
        Some(self.held_by(bindings, |of| of == id))
    }

    /// Take what the pads did since the last poll (the dummy's pad's presses aren't the menus').
    pub fn poll(&mut self, bindings: &Bindings, now: Instant) -> PadEvents {
        let mut out = PadEvents::default();
        while let Some(event) = gamepad_facade::next_event() {
            let id = event.id;
            if !matches!(event.kind, EventKind::Disconnected) && !self.pads.contains(&id) {
                self.pads.push(id);
            }
            let dummys = self.dummys(id);
            match event.kind {
                EventKind::ButtonDown(button) => {
                    if !self.down.contains(&(id, button)) {
                        self.down.push((id, button));
                    }
                    if dummys {
                        continue;
                    }
                    out.at.get_or_insert(now);
                    out.buttons.push(button);
                    out.pressed |= bindings.of_pad(button);
                    let Some(action) = bindings.pad_action(button) else { continue };
                    out.actions.push(action);
                    if matches!(action, Action::Up | Action::Down | Action::Left | Action::Right) {
                        self.repeat = Some((action, now, now));
                    }
                }
                EventKind::ButtonUp(button) => {
                    self.down.retain(|d| *d != (id, button));
                    if !dummys && matches!(button, Button::DPadUp | Button::DPadDown | Button::DPadLeft | Button::DPadRight) {
                        self.repeat = None;
                    }
                }
                EventKind::AxisMotion { axis, value } => {
                    let at = match self.sticks.iter().position(|(of, ..)| *of == id) {
                        Some(at) => at,
                        None => {
                            self.sticks.push((id, (0.0, 0.0), 0));
                            self.sticks.len() - 1
                        }
                    };
                    let (_, stick_at, stick) = &mut self.sticks[at];
                    match axis {
                        Axis::LeftX => stick_at.0 = value,
                        // (Up is negative.)
                        Axis::LeftY => stick_at.1 = value,
                        _ => continue,
                    }
                    let was = *stick;
                    *stick = 0;
                    for (at, minus, plus) in [(stick_at.0, keys::LEFT, keys::RIGHT), (stick_at.1, keys::UP, keys::DOWN)] {
                        if at < -0.5 {
                            *stick |= minus;
                        } else if at > 0.5 {
                            *stick |= plus;
                        }
                    }
                    let (now_held, fresh) = (*stick, *stick & !was);
                    if dummys {
                        continue;
                    }
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
                    } else if now_held == 0 && was != 0 {
                        self.repeat = None;
                    }
                }
                EventKind::Disconnected => {
                    self.down.retain(|(of, _)| *of != id);
                    self.sticks.retain(|(of, ..)| *of != id);
                    self.pads.retain(|of| *of != id);
                    if !dummys {
                        self.repeat = None;
                    }
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

/// The on-screen pad (a phone, in battle): where its buttons are, and which finger holds which.
///
/// Slint's `TouchArea` follows one pointer, so the pad takes the window's touch events itself, before Slint
/// (`main`'s winit hook): a finger that comes down on a button is the pad's until it lifts, holds the button
/// under it, and holds another as it slides onto it (a thumb rolling from Left to Down). The four directions
/// are read as one control: anywhere in their square, the direction is the one the finger is farthest toward
/// from the middle, so a thumb needn't land on a key; in the very middle it holds none.
#[derive(Default)]
pub struct TouchPad {
    /// Each button's place in the window, in units: a GBA button's bit number, and its rectangle (the UI says
    /// them as it lays the pad out).
    keys: Vec<(u8, Rect)>,
    /// Each finger on the pad, by the system's number for it, and the button under it (none: between two).
    fingers: Vec<(u64, Option<u8>)>,
}

impl TouchPad {
    /// The mouse, as one more finger (the pad clicked on a desktop).
    pub const MOUSE: u64 = u64::MAX;
    /// How far outside a button a finger still holds it.
    const REACH: f32 = 8.0;
    /// The directions' bits: right, left, up, down.
    const DIRECTIONS: [u8; 4] = [4, 5, 6, 7];

    /// The UI laid a button out at `rect`.
    pub fn place(&mut self, button: u8, rect: Rect) {
        match self.keys.iter_mut().find(|(b, _)| *b == button) {
            Some(key) => key.1 = rect,
            None => self.keys.push((button, rect)),
        }
    }

    /// Where a button is, if the UI has said.
    pub fn rect(&self, button: u8) -> Option<Rect> {
        self.keys.iter().find(|(b, _)| *b == button).map(|(_, r)| *r)
    }

    /// The square the four directions make, if all four are laid out.
    fn directions(&self) -> Option<Rect> {
        let rects: Vec<Rect> = TouchPad::DIRECTIONS.iter().filter_map(|b| self.rect(*b)).collect();
        if rects.len() < 4 {
            return None;
        }
        let (x, y) = (rects.iter().map(|r| r.x).fold(f32::MAX, f32::min), rects.iter().map(|r| r.y).fold(f32::MAX, f32::min));
        let (right, bottom) = (rects.iter().map(Rect::right).fold(f32::MIN, f32::max), rects.iter().map(Rect::bottom).fold(f32::MIN, f32::max));
        Some(Rect { x, y, w: right - x, h: bottom - y })
    }

    /// What is at a point: nothing of the pad's (`None`), or the pad's, with the button there if there is one.
    fn at(&self, x: f32, y: f32) -> Option<Option<u8>> {
        let within = |r: &Rect, reach: f32| x >= r.x - reach && x < r.right() + reach && y >= r.y - reach && y < r.bottom() + reach;
        if let Some(square) = self.directions().filter(|r| within(r, TouchPad::REACH)) {
            let (dx, dy) = (x - (square.x + square.w / 2.0), y - (square.y + square.h / 2.0));
            // (The middle sixth holds none: a thumb at rest there isn't pressing a direction.)
            if dx.abs().max(dy.abs()) < square.w.min(square.h) / 12.0 {
                return Some(None);
            }
            return Some(Some(if dx.abs() > dy.abs() { if dx > 0.0 { 4 } else { 5 } } else if dy < 0.0 { 6 } else { 7 }));
        }
        let near = |r: &Rect| (x - (r.x + r.w / 2.0)).abs() + (y - (r.y + r.h / 2.0)).abs();
        self.keys
            .iter()
            .filter(|(b, r)| !TouchPad::DIRECTIONS.contains(b) && within(r, TouchPad::REACH))
            .min_by(|a, b| near(&a.1).total_cmp(&near(&b.1)))
            .map(|(b, _)| Some(*b))
    }

    /// A finger came down. True if it is on the pad (and so the pad's until it lifts: nothing else gets it).
    pub fn down(&mut self, finger: u64, x: f32, y: f32) -> bool {
        let Some(button) = self.at(x, y) else { return false };
        self.fingers.retain(|(f, _)| *f != finger);
        self.fingers.push((finger, button));
        true
    }

    /// A finger moved. True if it is one of the pad's: it holds what is under it now, or nothing.
    pub fn moved(&mut self, finger: u64, x: f32, y: f32) -> bool {
        let under = self.at(x, y).flatten();
        match self.fingers.iter_mut().find(|(f, _)| *f == finger) {
            Some(held) => {
                held.1 = under;
                true
            }
            None => false,
        }
    }

    /// A finger lifted. True if it was one of the pad's.
    pub fn up(&mut self, finger: u64) -> bool {
        let before = self.fingers.len();
        self.fingers.retain(|(f, _)| *f != finger);
        self.fingers.len() != before
    }

    /// The mouse on a button, or off it (the UI's own word for it: one pointer).
    pub fn press(&mut self, button: Option<u8>) {
        self.fingers.retain(|(f, _)| *f != TouchPad::MOUSE);
        if button.is_some() {
            self.fingers.push((TouchPad::MOUSE, button));
        }
    }

    /// Every finger off (the battle no longer has the buttons: its menu opened, the window lost them).
    pub fn clear(&mut self) {
        self.fingers.clear();
    }

    /// The buttons held, as the GBA's bits.
    pub fn held(&self) -> u16 {
        self.fingers.iter().filter_map(|(_, b)| *b).fold(0, |held, b| held | 1 << b.min(9))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pad as the UI lays it out on a 390-wide phone (battle.slint): the directions in a plus at the left,
    /// A and B at the right, L and R over them.
    fn pad() -> TouchPad {
        let mut pad = TouchPad::default();
        let at = |x: f32, y: f32, w: f32, h: f32| Rect { x, y, w, h };
        pad.place(6, at(72.0, 556.0, 56.0, 56.0));
        pad.place(5, at(16.0, 612.0, 56.0, 56.0));
        pad.place(4, at(128.0, 612.0, 56.0, 56.0));
        pad.place(7, at(72.0, 668.0, 56.0, 56.0));
        pad.place(0, at(298.0, 556.0, 76.0, 76.0));
        pad.place(1, at(210.0, 630.0, 76.0, 76.0));
        pad.place(9, at(16.0, 484.0, 120.0, 48.0));
        pad.place(8, at(254.0, 484.0, 120.0, 48.0));
        pad
    }

    /// Two fingers hold two buttons, and one lifting leaves the other's held: a direction and A together.
    #[test]
    fn two_fingers_hold_two_buttons() {
        let mut pad = pad();
        assert!(pad.down(1, 40.0, 640.0));
        assert_eq!(pad.held(), keys::LEFT);
        assert!(pad.down(2, 330.0, 590.0));
        assert_eq!(pad.held(), keys::LEFT | keys::A);
        assert!(pad.down(3, 250.0, 660.0));
        assert_eq!(pad.held(), keys::LEFT | keys::A | keys::B);
        assert!(pad.up(2));
        assert_eq!(pad.held(), keys::LEFT | keys::B);
        assert!(pad.up(1));
        assert!(pad.up(3));
        assert_eq!(pad.held(), 0);
        assert!(!pad.up(3));
    }

    /// A thumb rolls from one direction to the next without lifting; in the square's very middle it holds none;
    /// just off a key's edge it still holds it; and it stays the pad's when it slides off every button.
    #[test]
    fn a_thumb_slides_across_the_directions() {
        let mut pad = pad();
        assert!(pad.down(1, 40.0, 640.0));
        assert_eq!(pad.held(), keys::LEFT);
        assert!(pad.moved(1, 100.0, 700.0));
        assert_eq!(pad.held(), keys::DOWN);
        assert!(pad.moved(1, 100.0, 640.0));
        assert_eq!(pad.held(), 0);
        assert!(pad.moved(1, 160.0, 640.0));
        assert_eq!(pad.held(), keys::RIGHT);
        // (The corner between Up and Right, on neither key: the direction it is farther toward.)
        assert!(pad.moved(1, 150.0, 600.0));
        assert_eq!(pad.held(), keys::RIGHT);
        assert!(pad.moved(1, 100.0, 552.0));
        assert_eq!(pad.held(), keys::UP);
        assert!(pad.moved(1, 200.0, 400.0));
        assert_eq!(pad.held(), 0);
        assert!(pad.moved(1, 330.0, 590.0));
        assert_eq!(pad.held(), keys::A);
    }

    /// A finger that comes down off the pad isn't the pad's (the picture, MENU): it goes on to the UI, and
    /// moving it over a button later presses nothing.
    #[test]
    fn a_finger_elsewhere_is_the_uis() {
        let mut pad = pad();
        assert!(!pad.down(1, 200.0, 100.0));
        assert!(!pad.moved(1, 330.0, 590.0));
        assert_eq!(pad.held(), 0);
        assert!(!pad.up(1));
        // (The mouse is one more finger: a pad clicked on a desktop, beside a finger's.)
        pad.press(Some(3));
        assert!(pad.down(1, 40.0, 640.0));
        assert_eq!(pad.held(), keys::START | keys::LEFT);
        pad.press(None);
        assert_eq!(pad.held(), keys::LEFT);
        pad.clear();
        assert_eq!(pad.held(), 0);
    }

    /// The menus' keys are fixed: Enter, Num Enter and Space confirm and Esc goes back whatever is bound; the
    /// keys bound to A and B (Z and X at first) do nothing there; the keys bound to L and R are previous and
    /// next.
    #[test]
    fn the_menus_keys_are_fixed() {
        let at = |b: &Bindings, code| menu_action(code, false, b);
        let b = Bindings::default();
        for confirm in [KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space] {
            assert_eq!(at(&b, confirm), Some(Action::Confirm));
        }
        assert_eq!(at(&b, KeyCode::Escape), Some(Action::Back));
        assert_eq!((at(&b, KeyCode::KeyZ), at(&b, KeyCode::KeyX)), (None, None));
        assert_eq!((at(&b, KeyCode::KeyA), at(&b, KeyCode::KeyS)), (Some(Action::Previous), Some(Action::Next)));
        assert_eq!(at(&b, KeyCode::ArrowUp), Some(Action::Up));
        assert_eq!(at(&b, KeyCode::Backspace), Some(Action::Delete));
        // A bound to Enter (START takes Z): Enter still confirms, and Z is nothing to the menus.
        let mut enter = Bindings::default();
        enter.bind_key(button_index(keys::A), KeyCode::Enter);
        assert_eq!((at(&enter, KeyCode::Enter), at(&enter, KeyCode::KeyZ)), (Some(Action::Confirm), None));
        // L on an arrow: the arrow moves.
        let mut up = Bindings::default();
        up.bind_key(button_index(keys::L), KeyCode::ArrowUp);
        assert_eq!(at(&up, KeyCode::ArrowUp), Some(Action::Up));
    }

    /// The pad's buttons follow the bindings too: A's confirms (south at first), B's goes back (west), the face
    /// button left of those two deletes (east), north is a screen's second action, the shoulders and Start are
    /// previous, next and the menu.
    #[test]
    fn the_pads_buttons_follow_the_bindings() {
        let b = Bindings::default();
        let at = |b: &Bindings, button| b.pad_action(button);
        assert_eq!(at(&b, Button::South), Some(Action::Confirm));
        assert_eq!(at(&b, Button::West), Some(Action::Back));
        assert_eq!(at(&b, Button::East), Some(Action::Delete));
        assert_eq!(at(&b, Button::North), Some(Action::Alternate));
        assert_eq!((at(&b, Button::LeftShoulder), at(&b, Button::RightShoulder)), (Some(Action::Previous), Some(Action::Next)));
        assert_eq!(at(&b, Button::Start), Some(Action::Menu));
        assert_eq!(at(&b, Button::DPadUp), Some(Action::Up));
        let mut east = Bindings::default();
        east.bind_pad(button_index(keys::B), Button::East);
        assert_eq!((at(&east, Button::East), at(&east, Button::West)), (Some(Action::Back), Some(Action::Delete)));
        // A on the D-pad: it moves, and south confirms.
        let mut pad = Bindings::default();
        pad.bind_pad(button_index(keys::A), Button::DPadUp);
        assert_eq!((at(&pad, Button::DPadUp), pad.confirm_button()), (Some(Action::Up), Button::South));
    }

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
