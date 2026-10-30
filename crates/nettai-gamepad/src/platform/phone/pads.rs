//! What the phone backends share: each pad's state as last seen, and the events of what changed since. iOS reads a
//! snapshot of each pad every poll; Android builds one from the events the host hands over. Either way the backend
//! says what a pad holds now ([`Pads::update`]) and this emits the edges, as SDL's queue would.

use crate::{Axis, Button, Event, EventKind, Id};
use std::collections::{HashMap, VecDeque};

/// Every button, in the order their changes are emitted: [`Button`]'s own order, so a button's place here is its
/// discriminant (`tests`).
pub(crate) const BUTTONS: [Button; 26] = [
    Button::South,
    Button::East,
    Button::West,
    Button::North,
    Button::Back,
    Button::Start,
    Button::Guide,
    Button::LeftStick,
    Button::RightStick,
    Button::LeftShoulder,
    Button::RightShoulder,
    Button::DPadUp,
    Button::DPadDown,
    Button::DPadLeft,
    Button::DPadRight,
    Button::Misc1,
    Button::Misc2,
    Button::Misc3,
    Button::Misc4,
    Button::Misc5,
    Button::Misc6,
    Button::RightPaddle1,
    Button::LeftPaddle1,
    Button::RightPaddle2,
    Button::LeftPaddle2,
    Button::Touchpad,
];

/// Every axis, in [`Axis`]'s own order.
pub(crate) const AXES: [Axis; 6] = [Axis::LeftX, Axis::LeftY, Axis::RightX, Axis::RightY, Axis::TriggerLeft, Axis::TriggerRight];

/// How much an axis must move before it counts as motion: a resting stick jitters in the last digits (the browser
/// backend's threshold). Coming to rest, or to an end, always counts.
const AXIS_EPSILON: f32 = 1.0 / 512.0;

/// One pad's state: the buttons held, and each axis's value (sticks in `[-1, 1]` with up and left negative,
/// triggers in `[0, 1]`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Pad {
    held: [bool; BUTTONS.len()],
    axes: [f32; AXES.len()],
}

impl Pad {
    pub fn held(&self, button: Button) -> bool {
        self.held[button as usize]
    }

    pub fn set(&mut self, button: Button, held: bool) {
        self.held[button as usize] = held;
    }

    pub fn axis(&self, axis: Axis) -> f32 {
        self.axes[axis as usize]
    }

    /// An axis's value, held to its range.
    pub fn set_axis(&mut self, axis: Axis, value: f32) {
        let low = if matches!(axis, Axis::TriggerLeft | Axis::TriggerRight) { 0.0 } else { -1.0 };
        self.axes[axis as usize] = value.clamp(low, 1.0);
    }
}

/// The pads known, by the backend's own key for each (a controller's address, a device's id), with their [`Id`]s
/// and state, and the events not handed out yet.
pub(crate) struct Pads<K> {
    known: HashMap<K, (Id, Pad)>,
    next_id: u32,
    queue: VecDeque<Event>,
}

impl<K> Default for Pads<K> {
    fn default() -> Self {
        Pads { known: HashMap::new(), next_id: 0, queue: VecDeque::new() }
    }
}

impl<K: std::hash::Hash + Eq + Clone> Pads<K> {
    /// The pad `key` is there: `Connected` if it is new, with an [`Id`] never given before and its state at rest.
    pub fn connect(&mut self, key: &K) -> Id {
        if let Some((id, _)) = self.known.get(key) {
            return *id;
        }
        let id = Id(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        self.known.insert(key.clone(), (id, Pad::default()));
        self.queue.push_back(Event { id, kind: EventKind::Connected });
        id
    }

    /// The pad `key` is gone: `Disconnected`, if it was known.
    pub fn disconnect(&mut self, key: &K) {
        if let Some((id, _)) = self.known.remove(key) {
            self.queue.push_back(Event { id, kind: EventKind::Disconnected });
        }
    }

    /// The state of the pad `key` as last seen (at rest for one not known).
    #[cfg_attr(not(any(android, test)), allow(dead_code))]
    pub fn state(&self, key: &K) -> Pad {
        self.known.get(key).map(|(_, pad)| *pad).unwrap_or_default()
    }

    /// The pad `key` holds `now` (connected first if new): an event for each button and axis that changed.
    pub fn update(&mut self, key: &K, now: Pad) {
        let id = self.connect(key);
        let Some((_, was)) = self.known.get_mut(key) else { return };
        for button in BUTTONS {
            if now.held(button) != was.held(button) {
                let kind = if now.held(button) { EventKind::ButtonDown(button) } else { EventKind::ButtonUp(button) };
                self.queue.push_back(Event { id, kind });
                was.set(button, now.held(button));
            }
        }
        for axis in AXES {
            let (value, before) = (now.axis(axis), was.axis(axis));
            let settled = value != before && (value == 0.0 || value.abs() == 1.0);
            if (value - before).abs() >= AXIS_EPSILON || settled {
                self.queue.push_back(Event { id, kind: EventKind::AxisMotion { axis, value } });
                was.set_axis(axis, value);
            }
        }
    }

    /// The next event not handed out.
    pub fn next(&mut self) -> Option<Event> {
        self.queue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain(pads: &mut Pads<u32>) -> Vec<Event> {
        std::iter::from_fn(|| pads.next()).collect()
    }

    /// A button's and an axis's place in the arrays is their discriminant.
    #[test]
    fn the_orders_are_the_enums() {
        assert!(BUTTONS.iter().enumerate().all(|(at, b)| *b as usize == at));
        assert!(AXES.iter().enumerate().all(|(at, a)| *a as usize == at));
    }

    /// A new pad is connected before its first change; each change is one edge; jitter under the threshold is no
    /// motion, but coming to rest is; a pad gone is disconnected, and one found again has a new id.
    #[test]
    fn changes_are_edges() {
        let mut pads = Pads::<u32>::default();
        let mut pad = Pad::default();
        pad.set(Button::South, true);
        pad.set_axis(Axis::LeftX, 0.5);
        pads.update(&7, pad);
        let kinds: Vec<EventKind> = drain(&mut pads).into_iter().map(|e| e.kind).collect();
        assert!(matches!(kinds[..], [EventKind::Connected, EventKind::ButtonDown(Button::South), EventKind::AxisMotion { axis: Axis::LeftX, value }] if value == 0.5));

        pads.update(&7, pad);
        assert!(drain(&mut pads).is_empty(), "nothing changed");

        pad.set_axis(Axis::LeftX, 0.5 + AXIS_EPSILON / 2.0);
        pads.update(&7, pad);
        assert!(drain(&mut pads).is_empty(), "jitter");

        pad.set(Button::South, false);
        pad.set_axis(Axis::LeftX, 0.0);
        pads.update(&7, pad);
        let kinds: Vec<EventKind> = drain(&mut pads).into_iter().map(|e| e.kind).collect();
        assert!(matches!(kinds[..], [EventKind::ButtonUp(Button::South), EventKind::AxisMotion { axis: Axis::LeftX, value }] if value == 0.0));
        assert_eq!(pads.state(&7), pad);

        pads.disconnect(&7);
        pads.update(&7, Pad::default());
        let seen: Vec<(Id, bool)> = drain(&mut pads).into_iter().map(|e| (e.id, matches!(e.kind, EventKind::Connected))).collect();
        assert_eq!(seen, [(Id(0), false), (Id(1), true)], "disconnected, then connected anew");
    }

    /// Triggers stay in `[0, 1]`, sticks in `[-1, 1]`.
    #[test]
    fn axes_are_held_to_their_ranges() {
        let mut pad = Pad::default();
        pad.set_axis(Axis::TriggerLeft, -0.5);
        pad.set_axis(Axis::RightY, 2.0);
        assert_eq!((pad.axis(Axis::TriggerLeft), pad.axis(Axis::RightY)), (0.0, 1.0));
    }
}
