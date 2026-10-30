//! What Android's gamepad keys and axes are in this crate's terms: its key codes (`KeyEvent.KEYCODE_*`) and motion
//! axes (`MotionEvent.AXIS_*`), as Android's standard layout has them for every gamepad. Apart from the backend
//! (`android`) so that it builds, and is tested, anywhere.

use crate::pads::Pad;
use crate::{Axis, Button};

/// What a gamepad key is: a button, or a trigger pressed (a pad with digital triggers' L2 and R2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Key {
    Button(Button),
    Trigger(Axis),
}

/// The key codes of a gamepad's keys. (`KEYCODE_BACK` isn't one: Android sends it in place of a B nothing took.)
pub(crate) fn key(code: u32) -> Option<Key> {
    Some(match code {
        96 => Key::Button(Button::South),          // KEYCODE_BUTTON_A
        97 => Key::Button(Button::East),           // KEYCODE_BUTTON_B
        99 => Key::Button(Button::West),           // KEYCODE_BUTTON_X
        100 => Key::Button(Button::North),         // KEYCODE_BUTTON_Y
        102 => Key::Button(Button::LeftShoulder),  // KEYCODE_BUTTON_L1
        103 => Key::Button(Button::RightShoulder), // KEYCODE_BUTTON_R1
        104 => Key::Trigger(Axis::TriggerLeft),    // KEYCODE_BUTTON_L2
        105 => Key::Trigger(Axis::TriggerRight),   // KEYCODE_BUTTON_R2
        106 => Key::Button(Button::LeftStick),     // KEYCODE_BUTTON_THUMBL
        107 => Key::Button(Button::RightStick),    // KEYCODE_BUTTON_THUMBR
        108 => Key::Button(Button::Start),         // KEYCODE_BUTTON_START
        109 => Key::Button(Button::Back),          // KEYCODE_BUTTON_SELECT
        110 => Key::Button(Button::Guide),         // KEYCODE_BUTTON_MODE
        19 => Key::Button(Button::DPadUp),         // KEYCODE_DPAD_UP
        20 => Key::Button(Button::DPadDown),       // KEYCODE_DPAD_DOWN
        21 => Key::Button(Button::DPadLeft),       // KEYCODE_DPAD_LEFT
        22 => Key::Button(Button::DPadRight),      // KEYCODE_DPAD_RIGHT
        _ => return None,
    })
}

/// A pad as a key went down or up.
pub(crate) fn keyed(mut pad: Pad, key: Key, down: bool) -> Pad {
    match key {
        Key::Button(button) => pad.set(button, down),
        Key::Trigger(axis) => pad.set_axis(axis, if down { 1.0 } else { 0.0 }),
    }
    pad
}

/// The axes a motion event says, by `AXIS_*` number.
pub(crate) mod axis {
    pub const X: u32 = 0;
    pub const Y: u32 = 1;
    pub const Z: u32 = 11;
    pub const RZ: u32 = 14;
    pub const HAT_X: u32 = 15;
    pub const HAT_Y: u32 = 16;
    pub const LTRIGGER: u32 = 17;
    pub const RTRIGGER: u32 = 18;
    pub const GAS: u32 = 22;
    pub const BRAKE: u32 = 23;
}

/// A pad as a motion event says it is now, `value` giving each axis by number: the sticks (X and Y, Z and RZ: up
/// is negative, as here), the triggers (LTRIGGER and RTRIGGER, or BRAKE and GAS, whichever a pad uses), and a hat
/// d-pad (HAT_X and HAT_Y, -1, 0 or 1), whose directions are the d-pad's buttons.
pub(crate) fn moved(mut pad: Pad, value: impl Fn(u32) -> f32) -> Pad {
    pad.set_axis(Axis::LeftX, value(axis::X));
    pad.set_axis(Axis::LeftY, value(axis::Y));
    pad.set_axis(Axis::RightX, value(axis::Z));
    pad.set_axis(Axis::RightY, value(axis::RZ));
    pad.set_axis(Axis::TriggerLeft, value(axis::LTRIGGER).max(value(axis::BRAKE)));
    pad.set_axis(Axis::TriggerRight, value(axis::RTRIGGER).max(value(axis::GAS)));
    let (hat_x, hat_y) = (value(axis::HAT_X), value(axis::HAT_Y));
    pad.set(Button::DPadLeft, hat_x < -0.5);
    pad.set(Button::DPadRight, hat_x > 0.5);
    pad.set(Button::DPadUp, hat_y < -0.5);
    pad.set(Button::DPadDown, hat_y > 0.5);
    pad
}

/// Whether an input source (`InputDevice.SOURCE_*` bits) is a gamepad's or a joystick's.
pub(crate) fn is_gamepad(source: u32) -> bool {
    const GAMEPAD: u32 = 0x0000_0401;
    const JOYSTICK: u32 = 0x0100_0010;
    source & GAMEPAD == GAMEPAD || source & JOYSTICK == JOYSTICK
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The face buttons, the d-pad keys and the digital triggers; KEYCODE_BACK and a keyboard's letters aren't a
    /// gamepad's.
    #[test]
    fn keys_are_the_standard_layouts() {
        assert_eq!(key(96), Some(Key::Button(Button::South)));
        assert_eq!(key(100), Some(Key::Button(Button::North)));
        assert_eq!(key(21), Some(Key::Button(Button::DPadLeft)));
        assert_eq!(key(105), Some(Key::Trigger(Axis::TriggerRight)));
        assert_eq!(key(4), None, "KEYCODE_BACK");
        assert_eq!(key(29), None, "KEYCODE_A");
        let pad = keyed(Pad::default(), Key::Trigger(Axis::TriggerLeft), true);
        assert_eq!(pad.axis(Axis::TriggerLeft), 1.0);
        assert_eq!(keyed(pad, Key::Trigger(Axis::TriggerLeft), false), Pad::default());
    }

    /// A motion event's sticks, triggers (either pair) and hat.
    #[test]
    fn motion_is_sticks_triggers_and_the_hat() {
        let pad = moved(Pad::default(), |a| match a {
            axis::X => 0.25,
            axis::Y => -1.0,
            axis::RZ => 0.5,
            axis::BRAKE => 0.75,
            axis::RTRIGGER => 0.1,
            axis::HAT_X => -1.0,
            axis::HAT_Y => 1.0,
            _ => 0.0,
        });
        assert_eq!((pad.axis(Axis::LeftX), pad.axis(Axis::LeftY), pad.axis(Axis::RightY)), (0.25, -1.0, 0.5));
        assert_eq!((pad.axis(Axis::TriggerLeft), pad.axis(Axis::TriggerRight)), (0.75, 0.1));
        assert!(pad.held(Button::DPadLeft) && pad.held(Button::DPadDown));
        assert!(!pad.held(Button::DPadRight) && !pad.held(Button::DPadUp));
    }

    /// A pad's sources: a gamepad's or a joystick's class bits, not a keyboard's or a touch screen's.
    #[test]
    fn sources_are_told_apart() {
        assert!(is_gamepad(0x0100_0511), "gamepad + joystick + keyboard, as a typical pad says");
        assert!(is_gamepad(0x0000_0401));
        assert!(!is_gamepad(0x0000_0101), "a keyboard");
        assert!(!is_gamepad(0x0000_1002), "a touch screen");
    }
}
