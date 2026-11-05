//! In a browser: what the page calls ([`entry`]: the app started, its files kept), the page's size followed, and
//! the input laid over a field ([`text_entry`]).

mod entry;
pub mod text_entry;

use wasm_bindgen::prelude::*;

pub fn follow(mut resized: impl FnMut(f32, f32) + 'static) {
    let Some(window) = web_sys::window() else { return };
    let mut tell = {
        let window = window.clone();
        move || {
            let size = |v: Result<JsValue, JsValue>| v.ok().and_then(|v| v.as_f64()).unwrap_or_default() as f32;
            resized(size(window.inner_width()), size(window.inner_height()));
        }
    };
    tell();
    let tell = Closure::<dyn FnMut()>::new(tell);
    let _ = window.add_event_listener_with_callback("resize", tell.as_ref().unchecked_ref());
    // (The page's for as long as it is open.)
    tell.forget();
}
