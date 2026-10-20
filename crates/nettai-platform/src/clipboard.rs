//! Text copied to the system's clipboard (an invite's code, an address): the desktop's (arboard), or the
//! browser's (`navigator.clipboard`). What goes wrong is said in `log`'s warning.

/// Copy `text`.
pub fn copy(text: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(why) = arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_string())) {
        log::warn!("copying to the clipboard: {why}");
    }
    #[cfg(target_arch = "wasm32")]
    web::nettai_copy(text);
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(inline_js = r#"
export function nettai_copy(text) {
    navigator.clipboard?.writeText(text).catch((e) => console.warn("nettai: copying to the clipboard:", e));
}
"#)]
    extern "C" {
        pub fn nettai_copy(text: &str);
    }
}
