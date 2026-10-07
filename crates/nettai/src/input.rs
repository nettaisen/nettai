//! The player's hands: the keyboard's keys (by where they are on the
//! keyboard, whatever its layout) and a gamepad's buttons, as the GBA's
//! buttons for the battle and as the menus' navigation.
//!
//! In the battle the keys come straight from the window's events
//! ([`Keys::event`], before Slint's focus sees them), timestamped as they
//! arrive; in the menus Slint's own key handling navigates. A gamepad is
//! polled each frame ([`Pad::poll`]): its buttons held, for the battle, and
//! its presses as navigation, for the menus.

use crate::NavAction;
use nettai_battle::input::keys;
use slint::winit_030::winit::event::{ElementState, KeyEvent};
use slint::winit_030::winit::keyboard::{KeyCode, PhysicalKey};
use std::time::Instant;

/// What a key does in the battle: a GBA button, or one of the battle's own
/// keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleKey {
    Button(u16),
    /// Esc: the pause.
    Pause,
    /// Tab: nothing in the battle (it would move the menus' focus).
    Nothing,
    /// Space, `-`, `=`, `.`: a replay's pause, speed and step.
    PlayPause,
    Slower,
    Faster,
    Step,
}

/// The battle's keys: the arrows move, Z is A, X is B, A is L, S is R,
/// Enter is START and Backspace is SELECT (nettai-demo's keys), by where
/// they are on the keyboard.
pub fn battle_key(code: KeyCode) -> Option<BattleKey> {
    use BattleKey::*;
    Some(match code {
        KeyCode::ArrowUp => Button(keys::UP),
        KeyCode::ArrowDown => Button(keys::DOWN),
        KeyCode::ArrowLeft => Button(keys::LEFT),
        KeyCode::ArrowRight => Button(keys::RIGHT),
        KeyCode::KeyZ => Button(keys::A),
        KeyCode::KeyX => Button(keys::B),
        KeyCode::KeyA => Button(keys::L),
        KeyCode::KeyS => Button(keys::R),
        KeyCode::Enter | KeyCode::NumpadEnter => Button(keys::START),
        KeyCode::Backspace | KeyCode::ShiftRight => Button(keys::SELECT),
        KeyCode::Escape => Pause,
        KeyCode::Tab => Nothing,
        KeyCode::Space => PlayPause,
        KeyCode::Minus => Slower,
        KeyCode::Equal => Faster,
        KeyCode::Period => Step,
        _ => return None,
    })
}

/// The keys held, as GBA buttons, and when each press came (for the
/// latency figures).
#[derive(Default)]
pub struct Keys {
    pub held: u16,
    /// The presses since the last tick, when each came.
    pub pressed: Vec<Instant>,
}

impl Keys {
    /// A key event of the window: a button held or let go (true), or the
    /// battle's own key pressed (returned). A key that isn't the battle's
    /// is none of these.
    pub fn event(&mut self, event: &KeyEvent, at: Instant) -> Option<BattleKey> {
        let PhysicalKey::Code(code) = event.physical_key else { return None };
        let key = battle_key(code)?;
        let down = event.state == ElementState::Pressed;
        match key {
            BattleKey::Button(b) => {
                if down && !event.repeat && self.held & b == 0 {
                    self.pressed.push(at);
                }
                if down {
                    self.held |= b;
                } else {
                    self.held &= !b;
                }
                Some(key)
            }
            _ if down && !event.repeat => Some(key),
            _ => Some(BattleKey::Button(0)),
        }
    }

    /// Every key let go (the window lost the keyboard).
    pub fn release(&mut self) {
        self.held = 0;
    }
}

/// A gamepad's buttons, as the GBA's: the south button (Xbox's A,
/// PlayStation's cross) is A, the west one B, the shoulders L and R, Start
/// and Select theirs; the D-pad and the left stick move.
fn gba_button(b: gilrs::Button) -> u16 {
    use gilrs::Button::*;
    match b {
        South => keys::A,
        West | East => keys::B,
        LeftTrigger => keys::L,
        RightTrigger => keys::R,
        Start => keys::START,
        Select => keys::SELECT,
        DPadUp => keys::UP,
        DPadDown => keys::DOWN,
        DPadLeft => keys::LEFT,
        DPadRight => keys::RIGHT,
        _ => 0,
    }
}

/// A gamepad press as the menus' navigation.
fn nav(b: gilrs::Button) -> Option<NavAction> {
    use gilrs::Button::*;
    Some(match b {
        DPadUp => NavAction::Up,
        DPadDown => NavAction::Down,
        DPadLeft => NavAction::Left,
        DPadRight => NavAction::Right,
        South | Start => NavAction::Confirm,
        East | Select => NavAction::Back,
        North => NavAction::Alternate,
        West => NavAction::Remove,
        LeftTrigger | LeftTrigger2 => NavAction::Previous,
        RightTrigger | RightTrigger2 => NavAction::Next,
        _ => return None,
    })
}

/// What a gamepad did since the last poll.
#[derive(Default)]
pub struct PadEvents {
    pub nav: Vec<NavAction>,
    /// The pause (the Mode or Guide button) was pressed.
    pub pause: bool,
    /// When the first button press came.
    pub pressed: Option<Instant>,
    /// Anything happened (the hints show the pad's buttons).
    pub used: bool,
}

/// The gamepads (gilrs), if they can be read.
pub struct Pad {
    gilrs: Option<gilrs::Gilrs>,
    /// The GBA buttons held by the D-pad and buttons, and by the left
    /// stick.
    held: u16,
    stick: u16,
}

impl Pad {
    pub fn new() -> Pad {
        Pad { gilrs: gilrs::Gilrs::new().ok(), held: 0, stick: 0 }
    }

    /// The GBA buttons the pads hold.
    pub fn held(&self) -> u16 {
        self.held | self.stick
    }

    /// Take what the pads did since the last poll.
    pub fn poll(&mut self) -> PadEvents {
        let mut out = PadEvents::default();
        let Some(gilrs) = &mut self.gilrs else { return out };
        while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
            use gilrs::EventType::*;
            match event {
                ButtonPressed(b, _) => {
                    out.used = true;
                    out.pressed.get_or_insert_with(Instant::now);
                    self.held |= gba_button(b);
                    if b == gilrs::Button::Mode {
                        out.pause = true;
                    }
                    out.nav.extend(nav(b));
                }
                ButtonReleased(b, _) => self.held &= !gba_button(b),
                AxisChanged(axis, value, _) => {
                    let (minus, plus) = match axis {
                        gilrs::Axis::LeftStickX => (keys::LEFT, keys::RIGHT),
                        // (gilrs's Y is up.)
                        gilrs::Axis::LeftStickY => (keys::DOWN, keys::UP),
                        _ => continue,
                    };
                    let was = self.stick;
                    self.stick &= !(minus | plus);
                    if value < -0.5 {
                        self.stick |= minus;
                    } else if value > 0.5 {
                        self.stick |= plus;
                    }
                    let new = self.stick & !was;
                    if new != 0 {
                        out.used = true;
                        out.pressed.get_or_insert_with(Instant::now);
                        out.nav.extend(match new {
                            keys::UP => Some(NavAction::Up),
                            keys::DOWN => Some(NavAction::Down),
                            keys::LEFT => Some(NavAction::Left),
                            keys::RIGHT => Some(NavAction::Right),
                            _ => None,
                        });
                    }
                }
                Disconnected => {
                    self.held = 0;
                    self.stick = 0;
                }
                _ => {}
            }
        }
        out
    }
}
