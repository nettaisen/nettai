//! The custom screen by pointer (docs/design/custom-screen-actions.md §5):
//! the mouse's and the fingers' events on the battle's picture, in the
//! frame's pixels, made into the local player's layer's gestures
//! (`nettai_battle::custom::layer::Gesture`) by what the last frame drew
//! where (`nettai_render::Renderer::target_at`).
//!
//! The mouse and a finger are one: pressed on a target, the cursor goes
//! there; let go on the same target, it chooses what it is (sliding off
//! cancels, as a button does); held still on it for [`LONG_PRESS`] ticks,
//! it describes it instead, as a right-click does. A click or a tap on no
//! target is still a click: it closes a description, as a key does, and the
//! form list's window, as B does (its targets span it). The pointer's moves
//! choose nothing and move no cursor: only what a gesture chooses goes in
//! the player's tick (docs/design/custom-screen-actions.md §3).

use nettai_battle::custom::layer::{Gesture, Target};

/// The ticks a press stays down on a target before it describes it (half
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
/// pixels (`Frame::size`, from its top left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub button: PointerButton,
    pub x: f32,
    pub y: f32,
}

/// A press of the primary button or a finger, until it is let go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Press {
    target: Option<Target>,
    /// The ticks it has been down, and whether it has left its target.
    held: u32,
    off: bool,
    /// A long press described its target: its letting go chooses nothing.
    described: bool,
}

/// The pointer's state on the custom screen: the press under way, and where
/// the pointer is (frame pixels; none once it leaves the picture), for the
/// host's hand cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenPointer {
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
            _ => Some((e.x.floor() as i32, e.y.floor() as i32)),
        };
        match (e.kind, e.button) {
            (PointerKind::Down, PointerButton::Primary) => {
                self.press = Some(Press { target: under, held: 0, off: false, described: false });
                under.map(Gesture::Down)
            }
            (PointerKind::Down, PointerButton::Secondary) => under.map(Gesture::Describe),
            (PointerKind::Down, PointerButton::Other) => None,
            (PointerKind::Up, PointerButton::Primary) => {
                let press = self.press.take()?;
                (!press.described && !press.off && under == press.target).then_some(Gesture::Click(under))
            }
            (PointerKind::Up, _) => None,
            // (A press follows nothing: off its target, it is cancelled.)
            (PointerKind::Moved, _) => {
                if let Some(press) = &mut self.press {
                    press.off |= under != press.target;
                }
                None
            }
            (PointerKind::Cancel, _) => {
                self.press = None;
                None
            }
            (PointerKind::Left, _) => {
                if let Some(press) = &mut self.press {
                    press.off = true;
                }
                None
            }
        }
    }

    /// A tick of the battle's: a press held still on a target long enough
    /// describes it.
    pub fn tick(&mut self) -> Option<Gesture> {
        let press = self.press.as_mut()?;
        press.held = press.held.saturating_add(1);
        if !press.off && !press.described && press.held >= LONG_PRESS {
            press.described = true;
            return press.target.map(Gesture::Describe);
        }
        None
    }

    /// Where the pointer is on the picture, in frame pixels (none off the
    /// picture), for the host's hand cursor.
    pub fn at(&self) -> Option<(i32, i32)> {
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

    fn e(kind: PointerKind, button: PointerButton, x: f32, y: f32) -> PointerEvent {
        PointerEvent { kind, button, x, y }
    }

    fn p(kind: PointerKind, x: f32, y: f32) -> PointerEvent {
        e(kind, PointerButton::Primary, x, y)
    }

    /// The pointer's place, for the hand cursor: where it last was on the
    /// picture; none once it leaves.
    #[test]
    fn the_pointer_is_where_it_last_was() {
        let mut s = ScreenPointer::default();
        s.event(p(PointerKind::Moved, 20.5, 10.0), at);
        assert_eq!(s.at(), Some((20, 10)));
        s.event(p(PointerKind::Down, 120.0, 10.0), at);
        assert_eq!(s.at(), Some((120, 10)));
        s.event(p(PointerKind::Left, 0.0, 0.0), at);
        assert_eq!(s.at(), None);
    }

    /// Moves make no gesture; down on a target puts the cursor there, up on
    /// the same target chooses; sliding off cancels; on nothing, a click on
    /// nothing; right-click or a long press describes.
    #[test]
    fn a_press_moves_the_cursor_and_its_letting_go_chooses() {
        let mut s = ScreenPointer::default();
        assert_eq!(s.event(p(PointerKind::Moved, 10.0, 10.0), at), None);
        assert_eq!(s.event(p(PointerKind::Moved, 120.0, 10.0), at), None);
        assert_eq!(s.event(p(PointerKind::Left, 0.0, 0.0), at), None);
        assert_eq!(s.event(p(PointerKind::Down, 10.0, 10.0), at), Some(Gesture::Down(CHIP)));
        assert_eq!(s.tick(), None);
        assert_eq!(s.event(p(PointerKind::Moved, 12.0, 10.0), at), None);
        assert_eq!(s.event(p(PointerKind::Up, 12.0, 10.0), at), Some(Gesture::Click(Some(CHIP))));
        assert_eq!(s.event(p(PointerKind::Down, 120.0, 10.0), at), Some(Gesture::Down(OK)));
        assert_eq!(s.event(p(PointerKind::Up, 121.0, 11.0), at), Some(Gesture::Click(Some(OK))));
        // Down on one, up on another; slid off and back; off the picture: nothing.
        s.event(p(PointerKind::Down, 10.0, 10.0), at);
        s.event(p(PointerKind::Moved, 120.0, 10.0), at);
        assert_eq!(s.event(p(PointerKind::Up, 120.0, 10.0), at), None);
        s.event(p(PointerKind::Down, 10.0, 10.0), at);
        s.event(p(PointerKind::Moved, 10.0, 90.0), at);
        s.event(p(PointerKind::Moved, 10.0, 10.0), at);
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 10.0), at), None);
        s.event(p(PointerKind::Down, 10.0, 10.0), at);
        s.event(p(PointerKind::Left, 0.0, 0.0), at);
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 10.0), at), None);
        // On nothing: a click on nothing.
        assert_eq!(s.event(p(PointerKind::Down, 10.0, 90.0), at), None);
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 90.0), at), Some(Gesture::Click(None)));
        // Right-click describes.
        assert_eq!(s.event(e(PointerKind::Down, PointerButton::Secondary, 10.0, 10.0), at), Some(Gesture::Describe(CHIP)));
        assert_eq!(s.event(e(PointerKind::Up, PointerButton::Secondary, 10.0, 10.0), at), None);
        // Held: it describes once, and its letting go chooses nothing.
        s.event(p(PointerKind::Down, 10.0, 10.0), at);
        let described: Vec<_> = (0..2 * LONG_PRESS).filter_map(|_| s.tick()).collect();
        assert_eq!(described, [Gesture::Describe(CHIP)]);
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 10.0), at), None);
        // Cancelled by the system: nothing.
        s.event(p(PointerKind::Down, 10.0, 10.0), at);
        s.event(p(PointerKind::Cancel, 10.0, 10.0), at);
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 10.0), at), None);
    }
}
