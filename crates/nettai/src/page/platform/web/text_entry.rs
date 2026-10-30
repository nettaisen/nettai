//! The browser's input over a field being edited (crate::page::text_entry).

use crate::page::text_entry::{Entry, Place};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r##"
let entry = null;
function canvas() {
    return document.querySelector("canvas");
}
// The input over the field at (x, y), width by height in the window's units (`windowWidth` of them across the canvas).
function lay(input, x, y, width, height, windowWidth) {
    const area = canvas().getBoundingClientRect();
    const scale = area.width / windowWidth;
    Object.assign(input.style, {
        left: `${area.left + (x + 12) * scale}px`,
        top: `${area.top + y * scale}px`,
        width: `${(width - 24) * scale}px`,
        height: `${height * scale}px`,
        font: `${Math.round(14 * scale)}px system-ui, sans-serif`,
    });
}
export function nettai_entry_place(x, y, width, height, windowWidth) {
    if (entry) lay(entry, x, y, width, height, windowWidth);
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
        margin: "0", padding: "0", border: "none", outline: "none", background: "transparent",
        color: "#e8e6f0", caretColor: "#e8e6f0",
    });
    lay(input, x, y, width, height, windowWidth);
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
    fn nettai_entry_place(x: f32, y: f32, width: f32, height: f32, window_width: f32);
    fn nettai_entry_keep(text: &str);
    fn nettai_entry_open_now() -> bool;
}

thread_local! {
    /// The open input's listener (kept as long as the input can call it).
    static HEARD: RefCell<Option<Closure<dyn FnMut(js_sys::Array)>>> = const { RefCell::new(None) };
}

pub fn open(place: Place, text: &str, select_all: bool, mut heard: impl FnMut(Entry) + 'static) {
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

pub fn place(place: Place) {
    nettai_entry_place(place.x, place.y, place.width, place.height, place.window_width);
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
