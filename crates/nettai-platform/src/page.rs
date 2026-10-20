//! The page the app is in, in a browser: its window is the page's whole, and follows it as it is resized. (The
//! desktop's window is the system's, which the player sizes: nothing here does anything there.)

/// `resized` hears the page's size in its CSS pixels (the window's logical ones) now, and each time it changes.
pub fn follow(resized: impl FnMut(f32, f32) + 'static) {
    #[cfg(target_arch = "wasm32")]
    web::follow(Box::new(resized));
    #[cfg(not(target_arch = "wasm32"))]
    drop(resized);
}

#[cfg(target_arch = "wasm32")]
mod web {
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

    pub fn follow(resized: Box<dyn FnMut(f32, f32)>) {
        let resized = Closure::<dyn FnMut(f32, f32)>::new(resized);
        nettai_page_follow(&resized);
        // (The page's for as long as it is open.)
        resized.forget();
    }
}
