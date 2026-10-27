//! The nettai app in a browser: what the page calls (web/index.html).
//!
//! The page, before the app starts: it puts the files it keeps (the data folder: the settings, the builds, the
//! replays, the games' content as downloaded) in the app's files with [`preload`], then [`start`]s it,
//! which downloads each game's content from the game's site as the desktop app does (nettai's `contents.rs`: the
//! page ships none). After:
//! it takes what changed in the data folder ([`changes`]) now and then, and writes it to its storage (the origin
//! private file system). Built with Wasm's atomics, so the app's threads are Web Workers on its memory
//! (nettai-platform's `thread`); build.sh builds it and the page, and serve.py serves them as a page with threads must be.

/// Where the data folder is in the app's files (`nettai::paths::data`, in the browser).
pub const DATA: &str = "/data";

#[cfg(target_arch = "wasm32")]
mod page {
    use wasm_bindgen::prelude::*;

    /// A file the page keeps, put in the data folder (`path` within it) before the app starts.
    #[wasm_bindgen]
    pub fn preload(path: &str, data: Vec<u8>) {
        nettai_platform::fs::preload(std::path::Path::new(super::DATA).join(path.trim_start_matches('/')), data);
    }

    /// The app started. A panic is said on the console. It returns once the window's loop is running on the
    /// page's; an error if the app couldn't start.
    #[wasm_bindgen]
    pub fn start() -> Result<(), JsError> {
        console_error_panic_hook::set_once();
        nettai_platform::fs::watch(super::DATA);
        nettai::run().map_err(|e| JsError::new(&e.to_string()))
    }

    /// What changed in the data folder since the last call, in order: each `["write", path, bytes]` or
    /// `["remove", path]` (a file or a folder and all in it), `path` within the data folder; none in a folder whose
    /// name starts with a dot.
    #[wasm_bindgen]
    pub fn changes() -> js_sys::Array {
        let out = js_sys::Array::new();
        for change in nettai_platform::fs::changes() {
            let (kind, path, data) = match change {
                nettai_platform::fs::Change::Write(path, data) => ("write", path, Some(data)),
                nettai_platform::fs::Change::Remove(path) => ("remove", path, None),
            };
            let Ok(path) = path.strip_prefix(super::DATA) else { continue };
            // (A folder whose name starts with a dot is the app's scratch: what is kept is what lands in place.)
            if path.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.')) {
                continue;
            }
            let entry = js_sys::Array::of2(&kind.into(), &path.to_string_lossy().as_ref().into());
            if let Some(data) = data {
                entry.push(&js_sys::Uint8Array::from(data.as_slice()));
            }
            out.push(&entry);
        }
        out
    }
}
