//! The browser's: its file input, its folder dialog (`showDirectoryPicker`), drops on the page, downloads, and its
//! clipboard (`navigator.clipboard`). A browser's files have no path the app can read, so those chosen are read into
//! the app's files (nettai-files, the page's tree), each lot in a folder of its own, and the app is given their paths
//! there. A file written as it is made ([`Writing`]) is made in the page's own storage, written as the bytes come
//! (on the page's main thread: [`pump`]), and handed over as a download once whole.

use crate::stream::Ending;
use crate::{Filter, Stream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
// A dropped folder's contents, through the File and Directory Entries API (the one every browser has for drops):
// its entries, which come in batches till one is empty; a file entry's file.
function entries(dir) {
    const reader = dir.createReader();
    return new Promise((resolve, reject) => {
        const all = [];
        const more = () => reader.readEntries((batch) => {
            if (!batch.length) return resolve(all);
            all.push(...batch);
            more();
        }, reject);
        more();
    });
}
function fileOf(entry) {
    return new Promise((resolve, reject) => entry.file(resolve, reject));
}
async function findIn(dir, parts, at, out) {
    const [part, ...rest] = parts;
    for (const entry of await entries(dir)) {
        if (rest.length === 0) {
            if (entry.isFile && matches(part, entry.name)) out.push([at + entry.name, await fileOf(entry)]);
        } else if (entry.isDirectory && entry.name.toLowerCase() === part.toLowerCase()) {
            await findIn(entry, rest, at + entry.name + "/", out);
        }
    }
}
export function nettai_take_drops(wanted, dropped) {
    document.addEventListener("dragover", (e) => e.preventDefault());
    document.addEventListener("drop", async (e) => {
        e.preventDefault();
        // (Each item's entry and file taken at once: the drop's data is gone once the handler waits.)
        const items = [...(e.dataTransfer?.items ?? [])].filter((i) => i.kind === "file").map((i) => [i.webkitGetAsEntry?.(), i.getAsFile()]);
        const files = [];
        const folders = [];
        for (const [entry, file] of items) {
            if (entry?.isDirectory) {
                folders.push(entry.name);
                try {
                    for (const path of wanted) await findIn(entry, path.split("/").filter(Boolean), entry.name + "/", files);
                } catch (why) {
                    console.warn(`reading the folder dropped, ${entry.name}:`, why);
                }
            } else if (file) {
                files.push([file.name, file]);
            }
        }
        if (files.length || folders.length) dropped(await read(files), folders);
    });
}
// A file made in the page's own storage, written as it comes: in a folder of its own, out of the app's kept files,
// which the page empties as it starts (index.html's MAKING). One at a time, each in the last's place: the last's
// download is long done by the time another is whole.
export async function nettai_make_file() {
    const root = await navigator.storage.getDirectory();
    const handle = await (await root.getDirectoryHandle(".making", { create: true })).getFileHandle("file", { create: true });
    return [handle, await handle.createWritable()];
}
// The file made, handed over as a download named `name`: `head`, then what was written.
export async function nettai_hand_over(handle, head, name) {
    // (`head` copied: a Blob takes no view of the shared memory. The file's own bytes are read as it downloads.)
    const url = URL.createObjectURL(new Blob([head.slice(), await handle.getFile()]));
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 60000);
}
export function nettai_sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
export function nettai_download(name, bytes) {
    // (A copy: a Blob takes no view of the shared memory.)
    const url = URL.createObjectURL(new Blob([bytes.slice()]));
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
    fn nettai_take_drops(wanted: Vec<String>, dropped: &Closure<dyn FnMut(js_sys::Array, js_sys::Array)>);
    fn nettai_download(name: &str, bytes: &[u8]);
    fn nettai_make_file() -> js_sys::Promise;
    fn nettai_hand_over(handle: &JsValue, head: &[u8], name: &str) -> js_sys::Promise;
    fn nettai_sleep(ms: u32) -> js_sys::Promise;

    type Writable;
    #[wasm_bindgen(method)]
    fn write(this: &Writable, bytes: &js_sys::Uint8Array) -> js_sys::Promise;
    #[wasm_bindgen(method)]
    fn close(this: &Writable) -> js_sys::Promise;
    #[wasm_bindgen(method)]
    fn abort(this: &Writable) -> js_sys::Promise;
}

/// A browser's folder can't be read again by its path: the page reads in what it needs of one.
pub const FOLDERS_READ_AGAIN: bool = false;

/// No dialog asks: a path in the app's files, downloaded once it is written there ([`saved`]).
pub fn save_path(filter: Filter, name: &str, near: Option<&Path>) -> Option<PathBuf> {
    let _ = (filter, near);
    Some(download_path(name))
}

pub fn saved(path: &Path) {
    download(path);
}

pub fn show_file(path: &Path) {
    send(path);
}

/// Nothing waits: the page's answers come on its own event loop.
pub fn poll() {}

pub fn copy(text: &str) {
    nettai_copy(text);
}

pub fn save_stream(filter: Filter, name: &str, done: Box<dyn FnOnce(Option<Stream>)>) {
    let _ = filter;
    make(name, Box::new(|made: std::io::Result<Option<Stream>>| done(made.ok().flatten())));
}

/// Made as [`save_stream`] makes one (the browser writes nowhere unasked), handed over as a download named as
/// `path`'s file.
pub fn save_stream_to(path: &Path, done: Box<dyn FnOnce(std::io::Result<Option<Stream>>)>) {
    make(&path.file_name().unwrap_or_default().to_string_lossy(), done);
}

/// A file being written as it is made: what is written waits in a queue the page's main thread writes ([`pump`]).
pub struct Writing {
    shared: Arc<Shared>,
}

struct Shared {
    name: String,
    state: std::sync::Mutex<State>,
}

#[derive(Default)]
struct State {
    /// Bytes waiting to be written, and how many.
    queue: std::collections::VecDeque<Vec<u8>>,
    queued: usize,
    /// What goes before what is written, once it is whole.
    head: Vec<u8>,
    ended: Option<Ending>,
    failed: Option<String>,
}

impl Writing {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        nettai_threads::lock(&self.shared.state).unwrap_or_else(|e| e.into_inner())
    }

    pub fn name(&self) -> &str {
        &self.shared.name
    }

    pub fn send(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let mut state = self.state();
        if let Some(why) = &state.failed {
            return Err(why.clone());
        }
        state.queued += bytes.len();
        state.queue.push_back(bytes);
        Ok(())
    }

    pub fn waiting(&self) -> usize {
        self.state().queued
    }

    /// What waits is written, and then (finished) the file handed over, `head` first: what goes wrong from here is
    /// logged. A write that failed: the file isn't whole, and is let go.
    pub fn end(&mut self, ending: Ending, head: Vec<u8>) -> std::io::Result<()> {
        let mut state = self.state();
        let failed = state.failed.clone().filter(|_| ending == Ending::Finished).map(std::io::Error::other);
        state.ended = Some(if failed.is_some() { Ending::Aborted } else { ending });
        state.head = head;
        failed.map_or(Ok(()), Err)
    }
}

/// A file made in the page's storage, to be handed over as a download named `name` once whole.
fn make(name: &str, done: Box<dyn FnOnce(std::io::Result<Option<Stream>>)>) {
    let making = nettai_make_file();
    let name = name.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let made = match wasm_bindgen_futures::JsFuture::from(making).await {
            Ok(made) => js_sys::Array::from(&made),
            Err(why) => return done(Err(std::io::Error::other(format!("{why:?}")))),
        };
        let writable: Writable = made.get(1).unchecked_into();
        let shared = Arc::new(Shared { name, state: std::sync::Mutex::new(State::default()) });
        wasm_bindgen_futures::spawn_local(pump(shared.clone(), made.get(0), writable));
        done(Ok(Some(Stream::new(Writing { shared }))));
    });
}

/// The page's main thread writes what the stream's writer queued, in order, till it ends; whole, the file
/// (`handle`'s) is handed over, its head first.
async fn pump(shared: Arc<Shared>, handle: JsValue, writable: Writable) {
    let state = || nettai_threads::lock(&shared.state).unwrap_or_else(|e| e.into_inner());
    loop {
        let (next, ended) = {
            let mut state = state();
            let next = state.queue.pop_front();
            if let Some(bytes) = &next {
                state.queued -= bytes.len();
            }
            (next, state.ended)
        };
        match (next, ended) {
            (_, Some(Ending::Aborted)) => {
                let _ = wasm_bindgen_futures::JsFuture::from(writable.abort()).await;
                return;
            }
            (Some(bytes), _) => {
                // (Copied out of the shared memory: the browser takes no view of it.)
                let copy = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
                copy.copy_from(&bytes);
                if let Err(why) = wasm_bindgen_futures::JsFuture::from(writable.write(&copy)).await {
                    log::warn!("writing {}: {why:?}", shared.name);
                    state().failed = Some(format!("{why:?}"));
                    let _ = wasm_bindgen_futures::JsFuture::from(writable.abort()).await;
                    return;
                }
            }
            (None, Some(Ending::Finished)) => {
                if let Err(why) = wasm_bindgen_futures::JsFuture::from(writable.close()).await {
                    log::warn!("closing {}: {why:?}", shared.name);
                    return;
                }
                let head = std::mem::take(&mut state().head);
                if let Err(why) = wasm_bindgen_futures::JsFuture::from(nettai_hand_over(&handle, &head, &shared.name)).await {
                    log::warn!("handing over {}: {why:?}", shared.name);
                }
                return;
            }
            (None, None) => {
                let _ = wasm_bindgen_futures::JsFuture::from(nettai_sleep(4)).await;
            }
        }
    }
}

/// A folder of the app's files for one lot of files chosen.
fn lot(under: &str) -> PathBuf {
    PathBuf::from(under).join(nettai_lot().to_string())
}

/// A path within what was chosen, as a path within the lot's folder alone: no `..` or root takes it out.
fn within(path: &str) -> PathBuf {
    Path::new(path).components().filter(|c| matches!(c, std::path::Component::Normal(_))).collect()
}

/// Files the page read (`[path, bytes]` each, the path within what was chosen) put in the app's files under
/// `dir`; their paths there.
fn keep(dir: &Path, files: &JsValue) -> Vec<PathBuf> {
    js_sys::Array::from(files)
        .iter()
        .filter_map(|entry| {
            let entry = js_sys::Array::from(&entry);
            let path = dir.join(within(&entry.get(0).as_string()?));
            nettai_files::preload(&path, js_sys::Uint8Array::new(&entry.get(1)).to_vec());
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

pub fn take_drops(wanted: &[&str], mut dropped: Box<dyn FnMut(Vec<PathBuf>)>) {
    let heard = Closure::<dyn FnMut(js_sys::Array, js_sys::Array)>::new(move |files: js_sys::Array, folders: js_sys::Array| {
        let dir = lot("/dropped");
        let kept = keep(&dir, &files);
        // (Each folder dropped as a path, its files in it; each file dropped outside one as its own.)
        let folders: Vec<PathBuf> = folders.iter().filter_map(|f| f.as_string()).map(|f| dir.join(within(&f))).filter(|f| f != &dir).collect();
        for folder in &folders {
            if let Err(why) = nettai_files::create_dir_all(folder) {
                log::warn!("keeping the folder dropped, {}: {why}", folder.display());
            }
        }
        let loose = kept.into_iter().filter(|p| !folders.iter().any(|f| p.starts_with(f)));
        dropped(folders.iter().cloned().chain(loose).collect());
    });
    nettai_take_drops(wanted.iter().map(|w| w.to_string()).collect(), &heard);
    // (The page's for as long as it is open.)
    heard.forget();
}

pub fn download_path(name: &str) -> PathBuf {
    lot("/downloads").join(name)
}

/// The file at `path` downloaded, and kept.
pub fn send(path: &Path) {
    match nettai_files::read(path) {
        Ok(bytes) => nettai_download(&path.file_name().unwrap_or_default().to_string_lossy(), &bytes),
        Err(why) => log::warn!("downloading {}: {why}", path.display()),
    }
}

pub fn download(path: &Path) {
    send(path);
    let _ = nettai_files::remove_file(path);
    // (Its folder too, if it was a download's own: `download_path`'s.)
    if let Some(dir) = path.parent() {
        let _ = nettai_files::remove_dir(dir);
    }
}

#[wasm_bindgen(inline_js = r#"
export function nettai_copy(text) {
    navigator.clipboard?.writeText(text).catch((e) => console.warn("nettai: copying to the clipboard:", e));
}
"#)]
extern "C" {
    fn nettai_copy(text: &str);
}
