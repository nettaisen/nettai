//! What the page calls (packaging/web/index.html): the app in a browser.
//!
//! The page, before the app starts: it puts the files it keeps (the data folder: the settings, the builds, the
//! replays, the games' content as downloaded) in the app's files with [`preload`], but for the stowed ones (the
//! Legacy Collection's ROMs as kept), which it lists ([`stowed`]) for the app to read when it wants them; then it
//! [`start`]s the app, which downloads each game's content from the game's site as the desktop app does
//! (`contents.rs`: the page ships none). After: it takes what changed in the data folder ([`changes`]) now and then,
//! and writes it to its storage (the origin private file system). Built with Wasm's atomics, so the app's threads
//! are Web Workers on its memory (nettai-threads); packaging/web/build.sh builds the module (this crate's library, as
//! a cdylib) and the page, and serve.py serves them as a page with threads must be.

use wasm_bindgen::prelude::*;

/// A file the page keeps, put in the data folder (`path` within it) before the app starts.
#[wasm_bindgen]
pub fn preload(path: &str, data: Vec<u8>) {
    nettai_files::preload(crate::paths::data().join(path.trim_start_matches('/')), data);
}

/// A file the page keeps and doesn't put in the app's files as it starts (`path` within the data folder, in its
/// stowed folder: `nettai_files::STOWED`): listed, to be read in when the app wants it (`nettai_files::fetch`).
#[wasm_bindgen]
pub fn stowed(path: &str) {
    nettai_files::listed(crate::paths::data().join(path.trim_start_matches('/')));
}

/// The app started. A panic is said on the console. It returns once the window's loop is running on the page's;
/// an error if the app couldn't start.
#[wasm_bindgen]
pub fn start() -> Result<(), JsError> {
    console_error_panic_hook::set_once();
    nettai_files::watch(crate::paths::data());
    crate::run().map_err(|e| JsError::new(&e.to_string()))
}

/// What changed in the data folder since the last call, in order: each `["write", path, bytes]` or
/// `["remove", path]` (a file or a folder and all in it), `path` within the data folder; none in a folder whose name
/// starts with a dot.
#[wasm_bindgen]
pub fn changes() -> js_sys::Array {
    let data = crate::paths::data();
    let out = js_sys::Array::new();
    for change in nettai_files::changes() {
        let (kind, path, bytes) = match change {
            nettai_files::Change::Write(path, bytes) => ("write", path, Some(bytes)),
            nettai_files::Change::Remove(path) => ("remove", path, None),
        };
        let Ok(path) = path.strip_prefix(&data) else { continue };
        // (A folder whose name starts with a dot is the app's scratch: what is kept is what lands in place.)
        if path.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.')) {
            continue;
        }
        let entry = js_sys::Array::of2(&kind.into(), &path.to_string_lossy().as_ref().into());
        if let Some(bytes) = bytes {
            entry.push(&js_sys::Uint8Array::from(bytes.as_slice()));
        }
        out.push(&entry);
    }
    out
}

