//! The iOS backend, over Apple's GameController framework.
//!
//! `GCController.controllers` lists the pads connected, and each one with the extended gamepad profile (every MFi,
//! Xbox, PlayStation and Switch Pro pad has it) holds its current state. So, as the browser backend does, this reads
//! a snapshot of each pad when the queue is empty, and the shared differ ([`crate::pads`]) emits what changed. A pad
//! is known by its controller object, which is held while it is listed (so no other takes its address); one gone
//! from the list is disconnected. A controller without the profile (the Siri Remote's micro gamepad) is passed over.
//!
//! The framework's objects are read on the thread that polls: the main thread, as UIKit's are.

use crate::pads::{Pad, Pads};
use crate::{Axis, Button, Event};
use objc2::rc::Retained;
use objc2_game_controller::{GCController, GCExtendedGamepad};
use std::cell::RefCell;

struct Backend {
    pads: Pads<usize>,
    /// The controllers listed at the last poll.
    held: Vec<Retained<GCController>>,
}

thread_local! {
    /// None until `init`, so `next_event` is a no-op before it, as the other backends' are.
    static BACKEND: RefCell<Option<Backend>> = const { RefCell::new(None) };
}

pub fn init(_app_name: &str) {
    BACKEND.with(|b| *b.borrow_mut() = Some(Backend { pads: Pads::default(), held: Vec::new() }));
}

pub fn next_event() -> Option<Event> {
    BACKEND.with(|b| {
        let mut b = b.borrow_mut();
        let b = b.as_mut()?;
        if let Some(event) = b.pads.next() {
            return Some(event);
        }
        poll(b);
        b.pads.next()
    })
}

/// A pad's key: its controller object's address, while it is held.
fn key(controller: &Retained<GCController>) -> usize {
    Retained::as_ptr(controller) as usize
}

/// The controllers listed now: the ones gone disconnected, each with the profile read and diffed.
fn poll(b: &mut Backend) {
    // SAFETY: the framework's class property, read on the thread that polls.
    let listed = unsafe { GCController::controllers() }.to_vec();
    for gone in b.held.iter().filter(|c| !listed.iter().any(|l| key(l) == key(c))) {
        b.pads.disconnect(&key(gone));
    }
    b.held = listed;
    for controller in &b.held {
        // SAFETY: a property of a controller the framework listed, read on the thread that polls.
        if let Some(gamepad) = unsafe { controller.extendedGamepad() } {
            b.pads.update(&key(controller), read(&gamepad));
        }
    }
}

/// What a pad holds now. Its stick's up is GameController's positive and this crate's negative (SDL's convention).
fn read(gamepad: &GCExtendedGamepad) -> Pad {
    let mut pad = Pad::default();
    // SAFETY: the profile's elements and their values, read on the thread that polls; each getter returns an object
    // the profile holds.
    unsafe {
        pad.set(Button::South, gamepad.buttonA().isPressed());
        pad.set(Button::East, gamepad.buttonB().isPressed());
        pad.set(Button::West, gamepad.buttonX().isPressed());
        pad.set(Button::North, gamepad.buttonY().isPressed());
        pad.set(Button::LeftShoulder, gamepad.leftShoulder().isPressed());
        pad.set(Button::RightShoulder, gamepad.rightShoulder().isPressed());
        pad.set(Button::Start, gamepad.buttonMenu().isPressed());
        let optional = [
            (Button::Back, gamepad.buttonOptions()),
            (Button::Guide, gamepad.buttonHome()),
            (Button::LeftStick, gamepad.leftThumbstickButton()),
            (Button::RightStick, gamepad.rightThumbstickButton()),
        ];
        for (button, input) in optional {
            pad.set(button, input.is_some_and(|i| i.isPressed()));
        }
        let dpad = gamepad.dpad();
        pad.set(Button::DPadUp, dpad.up().isPressed());
        pad.set(Button::DPadDown, dpad.down().isPressed());
        pad.set(Button::DPadLeft, dpad.left().isPressed());
        pad.set(Button::DPadRight, dpad.right().isPressed());
        let (left, right) = (gamepad.leftThumbstick(), gamepad.rightThumbstick());
        pad.set_axis(Axis::LeftX, left.xAxis().value());
        pad.set_axis(Axis::LeftY, -left.yAxis().value());
        pad.set_axis(Axis::RightX, right.xAxis().value());
        pad.set_axis(Axis::RightY, -right.yAxis().value());
        pad.set_axis(Axis::TriggerLeft, gamepad.leftTrigger().value());
        pad.set_axis(Axis::TriggerRight, gamepad.rightTrigger().value());
    }
    pad
}

#[cfg(test)]
mod tests {
    /// The framework is asked for its controllers and each one read, through the calls iOS makes (on macOS, whose
    /// GameController is the same): every pad attached says `Connected` first.
    #[test]
    fn the_controllers_are_read() {
        super::init("nettai-gamepad tests");
        let events: Vec<_> = std::iter::from_fn(super::next_event).collect();
        if let Some(first) = events.first() {
            assert!(matches!(first.kind, crate::EventKind::Connected));
        }
    }
}
