//! The browser's input over a field being edited (crate::page::text_entry).

use crate::page::text_entry::{Entry, Place};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{Event, HtmlElement, HtmlInputElement, KeyboardEvent};

/// What the open input tells.
type Heard = Rc<RefCell<dyn FnMut(Entry)>>;

/// A listener of the input's.
type Listener = Closure<dyn FnMut(Event)>;

thread_local! {
    /// The open input, if one is.
    static ENTRY: RefCell<Option<HtmlInputElement>> = const { RefCell::new(None) };
    /// The open input's listeners (kept as long as the input can call them: the next open replaces them).
    static LISTENERS: RefCell<Vec<Listener>> = const { RefCell::new(Vec::new()) };
}

fn document() -> Option<web_sys::Document> {
    web_sys::window()?.document()
}

fn canvas() -> Option<web_sys::Element> {
    document()?.query_selector("canvas").ok()?
}

/// Whether `input` is the one open.
fn is_entry(input: &HtmlInputElement) -> bool {
    ENTRY.with(|e| e.borrow().as_ref() == Some(input))
}

/// `heard` told `entry`, unless it is telling already (what it does may end the edit, which the input says too).
fn tell(heard: &Heard, entry: Entry) {
    if let Ok(mut heard) = heard.try_borrow_mut() {
        (*heard)(entry);
    }
}

/// `text`'s length as the input counts it (UTF-16's units).
fn length(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

/// `input` over the field at `place`, in the window's units (`place.window_width` of them across the canvas).
fn lay(input: &HtmlInputElement, place: Place) {
    let Some(area) = canvas().map(|c| c.get_bounding_client_rect()) else { return };
    let scale = area.width() / f64::from(place.window_width);
    let style = input.style();
    for (name, value) in [
        ("left", format!("{}px", area.left() + (f64::from(place.x) + 12.0) * scale)),
        ("top", format!("{}px", area.top() + f64::from(place.y) * scale)),
        ("width", format!("{}px", (f64::from(place.width) - 24.0) * scale)),
        ("height", format!("{}px", f64::from(place.height) * scale)),
        ("font", format!("{}px system-ui, sans-serif", (14.0 * scale).round())),
    ] {
        let _ = style.set_property(name, &value);
    }
}

pub fn open(place: Place, text: &str, select_all: bool, heard: impl FnMut(Entry) + 'static) {
    close();
    let Some(document) = document() else { return };
    let Some(body) = document.body() else { return };
    let Ok(input) = document.create_element("input").map(JsCast::unchecked_into::<HtmlInputElement>) else { return };
    input.set_type("text");
    input.set_value(text);
    input.set_spellcheck(false);
    input.set_autocomplete("off");
    let style = input.style();
    for (name, value) in [
        ("position", "fixed"),
        ("margin", "0"),
        ("padding", "0"),
        ("border", "none"),
        ("outline", "none"),
        ("background", "transparent"),
        ("color", "#e8e6f0"),
        ("caret-color", "#e8e6f0"),
    ] {
        let _ = style.set_property(name, value);
    }
    lay(&input, place);

    let heard: Heard = Rc::new(RefCell::new(heard));
    // (The edit's end, told once.)
    let end: Rc<dyn Fn(Entry)> = {
        let (heard, ended) = (heard.clone(), Cell::new(false));
        Rc::new(move |entry| {
            if !ended.replace(true) {
                tell(&heard, entry);
            }
        })
    };
    let changed = {
        let (input, heard) = (input.clone(), heard.clone());
        move |_: Event| {
            if is_entry(&input) {
                tell(&heard, Entry::Changed(input.value()));
            }
        }
    };
    let key = {
        let (input, end) = (input.clone(), end.clone());
        move |e: Event| {
            let Some(e) = e.dyn_ref::<KeyboardEvent>() else { return };
            // (While the input method composes, its keys are its own: Enter takes its candidate.)
            if e.is_composing() || e.key_code() == 229 {
                return;
            }
            let said = match e.key().as_str() {
                "Enter" => Entry::Done(input.value()),
                "Escape" => Entry::Canceled,
                "ArrowUp" => Entry::Up,
                "ArrowDown" => Entry::Down,
                "Tab" => Entry::Tab { back: e.shift_key() },
                _ => return,
            };
            e.prevent_default();
            end(said);
        }
    };
    // (Not as it is taken away: the input is let go first.)
    let left = {
        let input = input.clone();
        move |_: Event| {
            if is_entry(&input) {
                end(Entry::Left);
            }
        }
    };
    let listeners: [(&str, Listener); 3] = [("input", Closure::new(changed)), ("keydown", Closure::new(key)), ("blur", Closure::new(left))];
    for (kind, listener) in &listeners {
        let _ = input.add_event_listener_with_callback(kind, listener.as_ref().unchecked_ref());
    }
    let _ = body.append_child(&input);
    ENTRY.with(|e| *e.borrow_mut() = Some(input.clone()));
    LISTENERS.with(|l| *l.borrow_mut() = listeners.into_iter().map(|(_, listener)| listener).collect());

    // (After the window's own handling of what began the edit, which may focus the canvas.)
    let at = length(text);
    let focus = Closure::once_into_js(move || {
        if !is_entry(&input) {
            return;
        }
        let _ = input.focus();
        if select_all {
            input.select();
        } else {
            let _ = input.set_selection_range(at, at);
        }
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(focus.unchecked_ref(), 0);
    }
}

pub fn place(place: Place) {
    if let Some(input) = ENTRY.with(|e| e.borrow().clone()) {
        lay(&input, place);
    }
}

pub fn close() {
    let Some(input) = ENTRY.with(|e| e.borrow_mut().take()) else { return };
    input.remove();
    if let Some(canvas) = canvas().and_then(|c| c.dyn_into::<HtmlElement>().ok()) {
        let _ = canvas.focus();
    }
    // (Its listeners aren't dropped while one may be the one running: the next open replaces them.)
}

pub fn keep(text: &str) {
    let Some(input) = ENTRY.with(|e| e.borrow().clone()) else { return };
    if input.value() == text {
        return;
    }
    input.set_value(text);
    let at = length(text);
    let _ = input.set_selection_range(at, at);
}

pub fn is_open() -> bool {
    ENTRY.with(|e| e.borrow().is_some())
}
