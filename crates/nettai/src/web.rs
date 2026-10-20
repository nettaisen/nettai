//! The browser's side of the app (crates/nettai-web is the page's).
//!
//! The Legacy Collection's folder is chosen in the browser's folder dialog (`showDirectoryPicker`, which a page
//! may open only on a click or a key): of the folder chosen, the install's own, its `exe` or its `exe/data`
//! (`bnlc::Install::at`), the per-game archives are read and put in the page's files under [`COLLECTION`], where
//! the extraction reads them as it reads an install's on the desktop. They aren't kept past the page: the packs
//! made from them are.

use std::path::PathBuf;
use wasm_bindgen::prelude::*;

/// Where the archives chosen are put: as an install's own folder.
pub const COLLECTION: &str = "/legacy-collection";

#[wasm_bindgen(inline_js = r#"
async function folder(dir, name) {
    try { return await dir.getDirectoryHandle(name); } catch { return null; }
}
export async function nettai_choose_collection() {
    if (!window.showDirectoryPicker) throw new Error("this browser has no folder dialog (showDirectoryPicker)");
    const root = await window.showDirectoryPicker({ id: "legacy-collection", mode: "read" });
    const exe = await folder(root, "exe");
    const places = [exe && await folder(exe, "data"), await folder(root, "data"), root];
    for (const dir of places) {
        if (!dir) continue;
        const files = [];
        for await (const [name, handle] of dir.entries()) {
            const lower = name.toLowerCase();
            if (handle.kind === "file" && lower !== "exe.dat" && lower.startsWith("exe") && lower.endsWith(".dat")) {
                files.push([name, new Uint8Array(await (await handle.getFile()).arrayBuffer())]);
            }
        }
        if (files.length) return files;
    }
    return [];
}
"#)]
extern "C" {
    fn nettai_choose_collection() -> js_sys::Promise;
}

/// The player chooses the collection's folder: `chosen` is told the folder its archives are in now, or none if the
/// dialog was closed (or the browser has none: said in the log).
pub fn choose_collection(chosen: impl FnOnce(Option<PathBuf>) + 'static) {
    wasm_bindgen_futures::spawn_local(async move {
        let files = match wasm_bindgen_futures::JsFuture::from(nettai_choose_collection()).await {
            Ok(files) => files,
            Err(why) => {
                // (Closing the dialog is an AbortError, and no error of the app's.)
                let aborted = why.dyn_ref::<js_sys::Error>().is_some_and(|e| e.name() == "AbortError");
                if !aborted {
                    crate::errors::log("choosing the Legacy Collection's folder", &format_args!("{why:?}"));
                }
                return chosen(None);
            }
        };
        let _ = nettai_fs::remove_dir_all(COLLECTION);
        let data = PathBuf::from(COLLECTION).join("exe").join("data");
        for entry in js_sys::Array::from(&files).iter() {
            let entry = js_sys::Array::from(&entry);
            let (Some(name), bytes) = (entry.get(0).as_string(), js_sys::Uint8Array::new(&entry.get(1))) else { continue };
            nettai_fs::preload(data.join(name), bytes.to_vec());
        }
        chosen(Some(PathBuf::from(COLLECTION)));
    });
}
