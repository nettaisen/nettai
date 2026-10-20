//! Files and folders the player chooses: to open, where to save one, and files dropped on the page. On the desktop
//! the system's dialogs (rfd); in the browser its file input, its folder dialog (`showDirectoryPicker`) and
//! downloads. A browser's files have no path the app can read, so those chosen are read into the app's files
//! (`fs`, the page's tree), each lot in a folder of its own, and the app is given their paths there.
//!
//! A dialog answers through `done`: on the desktop before the call returns (the app's frames wait while it is
//! up), in the browser later, on the page's event loop. What goes wrong is said in `log`'s warning.

use std::path::{Path, PathBuf};

/// What a dialog asks for: its kind in the language shown ("Save files") and the extensions they may have.
#[derive(Clone, Copy, Debug)]
pub struct Filter<'a> {
    pub kind: &'a str,
    pub extensions: &'a [&'a str],
}

/// Files to open, one or more: `done` hears those chosen (none if none were).
pub fn open_files(filter: Filter, done: impl FnOnce(Vec<PathBuf>) + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    done(rfd::FileDialog::new().add_filter(filter.kind, filter.extensions).pick_files().unwrap_or_default());
    #[cfg(target_arch = "wasm32")]
    web::open_files(filter, Box::new(done));
}

/// A folder: `done` hears it (none if none was chosen). On the desktop it is the folder chosen. In the browser a
/// folder has no path: of the one chosen, the files at `wanted` (paths within it, `*` and `?` standing for any
/// letters and one letter of a file's name: `exe/data/exe?*.dat`) are read into the app's files, in a folder of
/// their own at the same paths, and `done` hears that folder.
pub fn open_folder(wanted: &[&str], done: impl FnOnce(Option<PathBuf>) + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = wanted;
        done(rfd::FileDialog::new().pick_folder());
    }
    #[cfg(target_arch = "wasm32")]
    web::open_folder(wanted, Box::new(done));
}

/// Where to save a file named `name` (`near`: a path to start the dialog at): none if the player chose none. In the
/// browser no dialog asks: it is a path in the app's files, and the file is downloaded once it is written there
/// ([`saved`]).
pub fn save_path(filter: Filter, name: &str, near: Option<&Path>) -> Option<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut dialog = rfd::FileDialog::new().add_filter(filter.kind, filter.extensions).set_file_name(name);
        if let Some(folder) = near.and_then(Path::parent).filter(|f| crate::fs::is_dir(f)) {
            dialog = dialog.set_directory(folder);
        }
        dialog.save_file()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (filter, near);
        Some(web::download_path(name))
    }
}

/// The file [`save_path`] gave is written. In the browser it is handed over as a download and let go of here; on
/// the desktop it is where it was asked to be, and nothing is done.
pub fn saved(path: &Path) {
    #[cfg(target_arch = "wasm32")]
    web::download(path);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = path;
}

/// Files dropped on the page go to `dropped`, from now on. (On the desktop the window's own events carry them,
/// winit's `DroppedFile`, and this does nothing.)
pub fn take_drops(dropped: impl FnMut(Vec<PathBuf>) + 'static) {
    #[cfg(target_arch = "wasm32")]
    web::take_drops(Box::new(dropped));
    #[cfg(not(target_arch = "wasm32"))]
    drop(dropped);
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::Filter;
    use std::path::{Path, PathBuf};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(inline_js = r#"
let lots = 0;
async function read(files) {
    const out = [];
    for (const [path, file] of files) out.push([path, new Uint8Array(await file.arrayBuffer())]);
    return out;
}
export function nettai_lot() {
    return ++lots;
}
export function nettai_open_files(extensions) {
    return new Promise((resolve) => {
        const input = document.createElement("input");
        input.type = "file";
        input.multiple = true;
        input.accept = extensions.map((e) => "." + e).join(",");
        input.onchange = async () => resolve(await read([...input.files].map((f) => [f.name, f])));
        input.oncancel = () => resolve([]);
        input.click();
    });
}
function matches(pattern, name) {
    const re = new RegExp("^" + pattern.replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*").replace(/\?/g, ".") + "$", "i");
    return re.test(name);
}
async function find(dir, parts, at, out) {
    const [part, ...rest] = parts;
    if (rest.length === 0) {
        for await (const [name, handle] of dir.entries()) {
            if (handle.kind === "file" && matches(part, name)) out.push([at + name, await handle.getFile()]);
        }
        return;
    }
    let sub;
    try { sub = await dir.getDirectoryHandle(part); } catch { return; }
    await find(sub, rest, at + part + "/", out);
}
export async function nettai_open_folder(wanted) {
    if (!window.showDirectoryPicker) throw new Error("this browser has no folder dialog (showDirectoryPicker)");
    const root = await window.showDirectoryPicker({ mode: "read" });
    const files = [];
    for (const path of wanted) await find(root, path.split("/").filter(Boolean), "", files);
    return await read(files);
}
export function nettai_take_drops(dropped) {
    document.addEventListener("dragover", (e) => e.preventDefault());
    document.addEventListener("drop", async (e) => {
        e.preventDefault();
        const files = [...(e.dataTransfer?.files ?? [])];
        if (files.length) dropped(await read(files.map((f) => [f.name, f])));
    });
}
export function nettai_download(name, bytes) {
    const url = URL.createObjectURL(new Blob([bytes]));
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 60000);
}
"#)]
    extern "C" {
        fn nettai_lot() -> u32;
        fn nettai_open_files(extensions: Vec<String>) -> js_sys::Promise;
        fn nettai_open_folder(wanted: Vec<String>) -> js_sys::Promise;
        fn nettai_take_drops(dropped: &Closure<dyn FnMut(js_sys::Array)>);
        fn nettai_download(name: &str, bytes: &[u8]);
    }

    /// A folder of the app's files for one lot of files chosen.
    fn lot(under: &str) -> PathBuf {
        PathBuf::from(under).join(nettai_lot().to_string())
    }

    /// Files the page read (`[path, bytes]` each, the path within what was chosen) put in the app's files under
    /// `dir`; their paths there.
    fn keep(dir: &Path, files: &JsValue) -> Vec<PathBuf> {
        js_sys::Array::from(files)
            .iter()
            .filter_map(|entry| {
                let entry = js_sys::Array::from(&entry);
                let within = entry.get(0).as_string()?;
                // (Within the lot's folder alone: no `..` or root takes a file out of it.)
                let within: PathBuf = Path::new(&within).components().filter(|c| matches!(c, std::path::Component::Normal(_))).collect();
                let path = dir.join(within);
                crate::fs::preload(&path, js_sys::Uint8Array::new(&entry.get(1)).to_vec());
                Some(path)
            })
            .collect()
    }

    /// `asked`'s answer: what the page read, or why not (a dialog closed is an AbortError, and nothing to say).
    async fn answer(asked: js_sys::Promise, doing: &str) -> Option<JsValue> {
        match wasm_bindgen_futures::JsFuture::from(asked).await {
            Ok(files) => Some(files),
            Err(why) => {
                if why.dyn_ref::<js_sys::Error>().is_none_or(|e| e.name() != "AbortError") {
                    log::warn!("{doing}: {why:?}");
                }
                None
            }
        }
    }

    pub fn open_files(filter: Filter, done: Box<dyn FnOnce(Vec<PathBuf>)>) {
        let asked = nettai_open_files(filter.extensions.iter().map(|e| e.to_string()).collect());
        wasm_bindgen_futures::spawn_local(async move {
            let files = answer(asked, "choosing files").await;
            done(files.map(|f| keep(&lot("/picked"), &f)).unwrap_or_default());
        });
    }

    pub fn open_folder(wanted: &[&str], done: Box<dyn FnOnce(Option<PathBuf>)>) {
        let asked = nettai_open_folder(wanted.iter().map(|w| w.to_string()).collect());
        wasm_bindgen_futures::spawn_local(async move {
            let folder = answer(asked, "choosing a folder").await.map(|files| {
                let dir = lot("/picked");
                keep(&dir, &files);
                dir
            });
            done(folder);
        });
    }

    pub fn take_drops(mut dropped: Box<dyn FnMut(Vec<PathBuf>)>) {
        let heard = Closure::<dyn FnMut(js_sys::Array)>::new(move |files: js_sys::Array| dropped(keep(&lot("/dropped"), &files)));
        nettai_take_drops(&heard);
        // (The page's for as long as it is open.)
        heard.forget();
    }

    pub fn download_path(name: &str) -> PathBuf {
        lot("/downloads").join(name)
    }

    pub fn download(path: &Path) {
        match crate::fs::read(path) {
            Ok(bytes) => nettai_download(&path.file_name().unwrap_or_default().to_string_lossy(), &bytes),
            Err(why) => log::warn!("downloading {}: {why}", path.display()),
        }
        if let Some(dir) = path.parent() {
            let _ = crate::fs::remove_dir_all(dir);
        }
    }
}
