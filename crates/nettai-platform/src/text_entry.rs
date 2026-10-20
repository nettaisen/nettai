//! Text typed into a field: the desktop's window takes it itself (its keys, and its input method's composing). A
//! browser gives a canvas no input method, so there a field being edited has an HTML `<input>` laid over it, in its
//! place and size: it takes the keys, the input method's composing and its candidates, the caret and the selection,
//! and says what it holds and how the edit ends ([`Entry`]). The field's own text is hidden under it meanwhile.

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
    #[cfg(target_arch = "wasm32")]
    web::open(place, text, select_all, Box::new(heard));
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (place, text, select_all, heard);
}

/// The field holds `text` now (what the app kept of what was typed): the input shows it, the caret after it, if it
/// shows anything else.
pub fn keep(text: &str) {
    #[cfg(target_arch = "wasm32")]
    web::keep(text);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = text;
}

/// Take the input away (the edit ended), if it is there.
pub fn close() {
    #[cfg(target_arch = "wasm32")]
    web::close();
}

/// Whether the input is laid over a field.
pub fn is_open() -> bool {
    #[cfg(target_arch = "wasm32")]
    return web::is_open();
    #[cfg(not(target_arch = "wasm32"))]
    false
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::{Entry, Place};
    use std::cell::RefCell;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(inline_js = r##"
let entry = null;
function canvas() {
    return document.querySelector("canvas");
}
export function nettai_entry_open(x, y, width, height, windowWidth, text, selectAll, heard) {
    nettai_entry_close();
    const area = canvas().getBoundingClientRect();
    const scale = area.width / windowWidth;
    const input = document.createElement("input");
    input.type = "text";
    input.value = text;
    input.spellcheck = false;
    input.autocomplete = "off";
    Object.assign(input.style, {
        position: "fixed",
        left: `${area.left + (x + 12) * scale}px`,
        top: `${area.top + y * scale}px`,
        width: `${(width - 24) * scale}px`,
        height: `${height * scale}px`,
        margin: "0", padding: "0", border: "none", outline: "none", background: "transparent",
        color: "#e8e6f0", caretColor: "#e8e6f0",
        font: `${Math.round(14 * scale)}px system-ui, sans-serif`,
    });
    let ended = false;
    const end = (said) => {
        if (ended) return;
        ended = true;
        heard(said);
    };
    input.addEventListener("input", () => entry === input && heard(["changed", input.value]));
    input.addEventListener("keydown", (e) => {
        // (While the input method composes, its keys are its own: Enter takes its candidate.)
        if (e.isComposing || e.keyCode === 229) return;
        const said = { Enter: ["done", input.value], Escape: ["canceled"], ArrowUp: ["up"], ArrowDown: ["down"], Tab: [e.shiftKey ? "back" : "tab"] }[e.key];
        if (!said) return;
        e.preventDefault();
        end(said);
    });
    // (Not as it is taken away: `entry` is let go first.)
    input.addEventListener("blur", () => entry === input && end(["left"]));
    document.body.appendChild(input);
    entry = input;
    // (After the window's own handling of what began the edit, which may focus the canvas.)
    setTimeout(() => {
        if (entry !== input) return;
        input.focus();
        if (selectAll) input.select(); else input.setSelectionRange(text.length, text.length);
    }, 0);
}
export function nettai_entry_close() {
    if (!entry) return;
    const input = entry;
    entry = null;
    input.remove();
    canvas()?.focus();
}
export function nettai_entry_keep(text) {
    if (!entry || entry.value === text) return;
    entry.value = text;
    entry.setSelectionRange(text.length, text.length);
}
export function nettai_entry_open_now() {
    return entry !== null;
}
"##)]
    extern "C" {
        fn nettai_entry_open(x: f32, y: f32, width: f32, height: f32, window_width: f32, text: &str, select_all: bool, heard: &Closure<dyn FnMut(js_sys::Array)>);
        fn nettai_entry_close();
        fn nettai_entry_keep(text: &str);
        fn nettai_entry_open_now() -> bool;
    }

    thread_local! {
        /// The open input's listener (kept as long as the input can call it).
        static HEARD: RefCell<Option<Closure<dyn FnMut(js_sys::Array)>>> = const { RefCell::new(None) };
    }

    pub fn open(place: Place, text: &str, select_all: bool, mut heard: Box<dyn FnMut(Entry)>) {
        let listener = Closure::<dyn FnMut(js_sys::Array)>::new(move |said: js_sys::Array| {
            let text = || said.get(1).as_string().unwrap_or_default();
            let entry = match said.get(0).as_string().as_deref() {
                Some("changed") => Entry::Changed(text()),
                Some("done") => Entry::Done(text()),
                Some("canceled") => Entry::Canceled,
                Some("up") => Entry::Up,
                Some("down") => Entry::Down,
                Some("tab") => Entry::Tab { back: false },
                Some("back") => Entry::Tab { back: true },
                Some("left") => Entry::Left,
                _ => return,
            };
            heard(entry);
        });
        nettai_entry_open(place.x, place.y, place.width, place.height, place.window_width, text, select_all, &listener);
        HEARD.with(|h| *h.borrow_mut() = Some(listener));
    }

    pub fn close() {
        nettai_entry_close();
        // (Not dropped while it may be the one running: the next open replaces it.)
    }

    pub fn keep(text: &str) {
        nettai_entry_keep(text);
    }

    pub fn is_open() -> bool {
        nettai_entry_open_now()
    }
}
