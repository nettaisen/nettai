//! Text typed into a field: the desktop's window takes it itself (its keys, and its input method's composing). A
//! browser gives a canvas no input method, so there a field being edited has an HTML `<input>` laid over it, in its
//! place and size: it takes the keys, the input method's composing and its candidates, the caret and the selection,
//! and says what it holds and how the edit ends ([`Entry`]). The field's own text is hidden under it meanwhile.

use super::platform::system::text_entry as system;

/// Where the field is, in the window's logical pixels, and how wide the window is in them (the page scales them
/// to the canvas).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub window_width: f32,
}

/// What the input says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// It holds this now.
    Changed(String),
    /// Enter: the edit is done, and its purpose follows.
    Done(String),
    /// Esc: the edit is dropped.
    Canceled,
    /// The edit is confirmed and the focus moves: up or down, or on (Tab; back with Shift).
    Up,
    Down,
    Tab { back: bool },
    /// It lost the keyboard (a click elsewhere): the edit is confirmed.
    Left,
}

/// Lay the input over the field at `place`, holding `text` (all of it selected, or the caret after it), and tell
/// `heard` what it says till it is [`close`]d. Nothing on the desktop.
pub fn open(place: Place, text: &str, select_all: bool, heard: impl FnMut(Entry) + 'static) {
    system::open(place, text, select_all, heard);
}

/// The field holds `text` now (what the app kept of what was typed): the input shows it, the caret after it, if it
/// shows anything else.
pub fn keep(text: &str) {
    system::keep(text);
}

/// The field is at `place` now (the window was resized): the open input is laid over it there, holding what it
/// holds. Nothing if none is open, or on the desktop.
pub fn place(place: Place) {
    system::place(place);
}

/// Take the input away (the edit ended), if it is there.
pub fn close() {
    system::close();
}

/// Whether the input is laid over a field.
pub fn is_open() -> bool {
    system::is_open()
}
