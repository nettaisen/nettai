//! In a browser: what the page calls ([`entry`]: the app started, its files kept), the page's size followed, and
//! the input laid over a field ([`text_entry`]).

mod entry;
pub mod text_entry;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
export function nettai_page_follow(resized) {
    const tell = () => resized(window.innerWidth, window.innerHeight);
    window.addEventListener("resize", tell);
    tell();
}
"#)]
extern "C" {
    fn nettai_page_follow(resized: &Closure<dyn FnMut(f32, f32)>);
}

pub fn follow(resized: impl FnMut(f32, f32) + 'static) {
    let resized = Closure::<dyn FnMut(f32, f32)>::new(resized);
    nettai_page_follow(&resized);
    // (The page's for as long as it is open.)
    resized.forget();
}
