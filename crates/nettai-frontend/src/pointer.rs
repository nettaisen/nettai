//! The custom screen by pointer (docs/design/custom-screen-actions.md §5):
//! the mouse's and the fingers' events on the battle's picture, in the
//! frame's pixels, made into the local player's layer's gestures
//! (`nettai_battle::custom::layer::Gesture`) by what the last frame drew
//! where (`nettai_render::Renderer::target_at`).
//!
//! The mouse: over a target, the cursor goes there; a click (down and up on
//! the same target) chooses what it is; a right-click describes it. A
//! finger: down on a target puts the cursor there; up on the same target
//! chooses (sliding off cancels, as a button does); held still on it for
//! [`LONG_PRESS`] ticks, it describes it instead. A click or a tap on no
//! target is still a click: it closes a description, as a key does, and the
//! form list's window, as B does (its targets span it). A hover is a
//! gesture only as the target under the pointer changes (the mouse onto
//! another, a finger down after it was lifted); the mouse off the targets
//! (onto none, or off the picture) is away, which takes a cursor on a pick
//! back to its slot. Each gesture goes in the player's tick, for a replay
//! to show their cursor (docs/design/custom-screen-actions.md §3).

use nettai_battle::custom::layer::{Gesture, Target};

/// The ticks a finger stays down on a target before it describes it (half
/// a second).
pub const LONG_PRESS: u32 = 30;

/// What a pointer did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    /// Pressed (a button, a finger down).
    Down,
    /// Let go.
    Up,
    /// Moved, pressed or not.
    Moved,
    /// The system took it (a gesture of its own, the window lost it): what
    /// it began is forgotten.
    Cancel,
    /// It left the picture.
    Left,
}

/// Which of the mouse's buttons (a finger is the primary).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Other,
}

/// A pointer's event on the battle's picture: `x` and `y` in the frame's
/// pixels (`Frame::size`, from its top left); `touch` for a finger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub button: PointerButton,
    pub touch: bool,
    pub x: f32,
    pub y: f32,
}

/// A press of the primary button or a finger, until it is let go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Press {
    target: Option<Target>,
    touch: bool,
    /// The ticks it has been down, and whether it has left its target.
    held: u32,
    off: bool,
    /// A long press described its target: its letting go chooses nothing.
    described: bool,
}

/// The pointer's state on the custom screen: the target the pointer was
/// last over (the mouse's; a finger's while it is down), the press under
/// way, and where the mouse is (frame pixels; none for a finger or a mouse
/// gone off the picture), for its cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenPointer {
    over: Option<Target>,
    press: Option<Press>,
    at: Option<(i32, i32)>,
}

impl ScreenPointer {
    /// An event, `at` saying what is under a place of the frame: the
    /// gesture it makes, if it makes one.
    pub fn event(&mut self, e: PointerEvent, at: impl Fn(f32, f32) -> Option<Target>) -> Option<Gesture> {
        let under = at(e.x, e.y);
        self.at = match e.kind {
            PointerKind::Left | PointerKind::Cancel => None,
            _ if e.touch => None,
            _ => Some((e.x.floor() as i32, e.y.floor() as i32)),
        };
        match (e.kind, e.button) {
            (PointerKind::Down, PointerButton::Primary) => {
                self.press = Some(Press { target: under, touch: e.touch, held: 0, off: false, described: false });
                self.hover(under)
            }
            (PointerKind::Down, PointerButton::Secondary) => under.map(Gesture::Describe),
            (PointerKind::Down, PointerButton::Other) => None,
            (PointerKind::Up, PointerButton::Primary) => {
                // (A finger lifted is over nothing.)
                if e.touch {
                    self.over = None;
                }
                let press = self.press.take()?;
                (!press.described && !press.off && under == press.target).then_some(Gesture::Click(under))
            }
            (PointerKind::Up, _) => None,
            (PointerKind::Moved, _) => match &mut self.press {
                // (A press follows nothing: off its target, it is
                // cancelled.)
                Some(press) => {
                    press.off |= under != press.target;
                    None
                }
                None => self.hover(under),
            },
            (PointerKind::Cancel, _) => {
                if e.touch {
                    self.over = None;
                }
                self.press = None;
                None
            }
            (PointerKind::Left, _) => {
                if let Some(press) = &mut self.press {
                    press.off = true;
                }
                // (Off the picture from a target: away.)
                self.over.take().map(|_| Gesture::Away)
            }
        }
    }

    /// The pointer over `under`: a hover there if it is another target than
    /// the one it was over; off the targets, away.
    fn hover(&mut self, under: Option<Target>) -> Option<Gesture> {
        if under == self.over {
            return None;
        }
        self.over = under;
        Some(under.map_or(Gesture::Away, Gesture::Hover))
    }

    /// A tick of the battle's: a finger held still on a target long enough
    /// describes it.
    pub fn tick(&mut self) -> Option<Gesture> {
        let press = self.press.as_mut()?;
        press.held = press.held.saturating_add(1);
        if press.touch && !press.off && !press.described && press.held >= LONG_PRESS {
            press.described = true;
            return press.target.map(Gesture::Describe);
        }
        None
    }

    /// Where the mouse is on the picture, in frame pixels (none for a finger,
    /// or off the picture).
    pub fn mouse_at(&self) -> Option<(i32, i32)> {
        self.at
    }

    /// Forget what is under way (a new round, the battle held).
    pub fn clear(&mut self) {
        *self = ScreenPointer::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHIP: Target = Target::Slot(1);
    const OK: Target = Target::Slot(10);

    /// Slot 1 left of x 100, OK right of it; nothing below y 50.
    fn at(x: f32, y: f32) -> Option<Target> {
        (y < 50.0).then_some(if x < 100.0 { CHIP } else { OK })
    }

    fn e(kind: PointerKind, button: PointerButton, touch: bool, x: f32, y: f32) -> PointerEvent {
        PointerEvent { kind, button, touch, x, y }
    }

    /// The mouse's place, for its cursor: where it last was on the picture;
    /// none once it leaves, and none for a finger.
    #[test]
    fn the_mouse_is_where_it_last_was() {
        let mut p = ScreenPointer::default();
        p.event(e(PointerKind::Moved, PointerButton::Primary, false, 20.5, 10.0), at);
        assert_eq!(p.mouse_at(), Some((20, 10)));
        p.event(e(PointerKind::Down, PointerButton::Primary, false, 120.0, 10.0), at);
        assert_eq!(p.mouse_at(), Some((120, 10)));
        p.event(e(PointerKind::Left, PointerButton::Primary, false, 0.0, 0.0), at);
        assert_eq!(p.mouse_at(), None);
        p.event(e(PointerKind::Down, PointerButton::Primary, true, 20.0, 10.0), at);
        assert_eq!(p.mouse_at(), None);
    }

    #[test]
    fn the_mouse_hovers_clicks_and_describes() {
        let mut p = ScreenPointer::default();
        let m = |kind, x, y| e(kind, PointerButton::Primary, false, x, y);
        assert_eq!(p.event(m(PointerKind::Moved, 10.0, 10.0), at), Some(Gesture::Hover(CHIP)));
        // (Over the same target: nothing new.)
        assert_eq!(p.event(m(PointerKind::Moved, 20.0, 10.0), at), None);
        assert_eq!(p.event(m(PointerKind::Moved, 120.0, 10.0), at), Some(Gesture::Hover(OK)));
        // Off the targets: away, once; off the picture from one: away.
        assert_eq!(p.event(m(PointerKind::Moved, 10.0, 90.0), at), Some(Gesture::Away));
        assert_eq!(p.event(m(PointerKind::Moved, 11.0, 90.0), at), None);
        assert_eq!(p.event(m(PointerKind::Moved, 120.0, 10.0), at), Some(Gesture::Hover(OK)));
        assert_eq!(p.event(m(PointerKind::Left, 0.0, 0.0), at), Some(Gesture::Away));
        assert_eq!(p.event(m(PointerKind::Left, 0.0, 0.0), at), None);
        assert_eq!(p.event(m(PointerKind::Moved, 120.0, 10.0), at), Some(Gesture::Hover(OK)));
        // (Pressed where it is over: no hover again.)
        assert_eq!(p.event(m(PointerKind::Down, 120.0, 10.0), at), None);
        assert_eq!(p.event(m(PointerKind::Up, 121.0, 11.0), at), Some(Gesture::Click(Some(OK))));
        // Pressed on another target than it was over: a hover there first.
        assert_eq!(p.event(m(PointerKind::Down, 10.0, 10.0), at), Some(Gesture::Hover(CHIP)));
        assert_eq!(p.event(m(PointerKind::Up, 10.0, 10.0), at), Some(Gesture::Click(Some(CHIP))));
        // Down on one, up on another: nothing.
        p.event(m(PointerKind::Down, 10.0, 10.0), at);
        p.event(m(PointerKind::Moved, 120.0, 10.0), at);
        assert_eq!(p.event(m(PointerKind::Up, 120.0, 10.0), at), None);
        // On nothing: a click on nothing.
        p.event(m(PointerKind::Down, 10.0, 90.0), at);
        assert_eq!(p.event(m(PointerKind::Up, 10.0, 90.0), at), Some(Gesture::Click(None)));
        // Right-click describes; the mouse held long describes nothing.
        assert_eq!(p.event(e(PointerKind::Down, PointerButton::Secondary, false, 10.0, 10.0), at), Some(Gesture::Describe(CHIP)));
        p.event(m(PointerKind::Down, 10.0, 10.0), at);
        assert!((0..2 * LONG_PRESS).all(|_| p.tick().is_none()));
    }

    #[test]
    fn a_finger_taps_slides_off_and_long_presses() {
        let mut p = ScreenPointer::default();
        let f = |kind, x, y| e(kind, PointerButton::Primary, true, x, y);
        // Down moves the cursor; up on the same target chooses.
        assert_eq!(p.event(f(PointerKind::Down, 10.0, 10.0), at), Some(Gesture::Hover(CHIP)));
        assert_eq!(p.tick(), None);
        assert_eq!(p.event(f(PointerKind::Up, 12.0, 10.0), at), Some(Gesture::Click(Some(CHIP))));
        // (Lifted, it is over nothing: down on the same target again is a hover again.)
        assert_eq!(p.event(f(PointerKind::Down, 10.0, 10.0), at), Some(Gesture::Hover(CHIP)));
        assert_eq!(p.event(f(PointerKind::Up, 10.0, 10.0), at), Some(Gesture::Click(Some(CHIP))));
        // Slid off and back: cancelled.
        p.event(f(PointerKind::Down, 10.0, 10.0), at);
        p.event(f(PointerKind::Moved, 10.0, 90.0), at);
        p.event(f(PointerKind::Moved, 10.0, 10.0), at);
        assert_eq!(p.event(f(PointerKind::Up, 10.0, 10.0), at), None);
        // Held: it describes once, and its letting go chooses nothing.
        p.event(f(PointerKind::Down, 10.0, 10.0), at);
        let described: Vec<_> = (0..2 * LONG_PRESS).filter_map(|_| p.tick()).collect();
        assert_eq!(described, [Gesture::Describe(CHIP)]);
        assert_eq!(p.event(f(PointerKind::Up, 10.0, 10.0), at), None);
        // Cancelled by the system: nothing.
        p.event(f(PointerKind::Down, 10.0, 10.0), at);
        p.event(f(PointerKind::Cancel, 10.0, 10.0), at);
        assert_eq!(p.event(f(PointerKind::Up, 10.0, 10.0), at), None);
    }
}
