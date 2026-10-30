//! Files and folders the player chooses: to open, where to save one, and files dropped on the page. On the desktop
//! the system's dialogs (rfd); in the browser its file input, its folder dialog (`showDirectoryPicker`) and
//! downloads (a file made as it is written, in the page's own storage, then handed over as one: [`Stream`]). A
//! browser's files have no path the app can read, so those chosen are read into the app's files (`fs`, the page's
//! tree), each lot in a folder of its own, and the app is given their paths there.
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

/// Whether a folder the player chose can be read again later, at another start, by its path: the desktop's can; a
/// browser's and a phone's can't (the system hands the app its files once), so the app keeps copies of what it
/// needs of one ([`open_folder`]'s files are read in).
pub const FOLDERS_READ_AGAIN: bool = cfg!(desktop);

/// Files to open, one or more: `done` hears those chosen (none if none were).
pub fn open_files(filter: Filter, done: impl FnOnce(Vec<PathBuf>) + 'static) {
    #[cfg(phone)]
    phone::open_files(filter, Box::new(done));
    #[cfg(desktop)]
    done(rfd::FileDialog::new().add_filter(filter.kind, filter.extensions).pick_files().unwrap_or_default());
    #[cfg(web)]
    web::open_files(filter, Box::new(done));
}

/// A folder: `done` hears it (none if none was chosen). On the desktop it is the folder chosen. In the browser a
/// folder has no path: of the one chosen, the files at `wanted` (paths within it, `*` and `?` standing for any
/// letters and one letter of a file's name: `exe/data/exe?*.dat`) are read into the app's files, in a folder of
/// their own at the same paths, and `done` hears that folder.
pub fn open_folder(wanted: &[&str], done: impl FnOnce(Option<PathBuf>) + 'static) {
    #[cfg(phone)]
    phone::open_folder(wanted, Box::new(done));
    #[cfg(desktop)]
    {
        let _ = wanted;
        done(rfd::FileDialog::new().pick_folder());
    }
    #[cfg(web)]
    web::open_folder(wanted, Box::new(done));
}

/// Where to save a file named `name` (`near`: a path to start the dialog at): none if the player chose none. In the
/// browser no dialog asks: it is a path in the app's files, and the file is downloaded once it is written there
/// ([`saved`]).
pub fn save_path(filter: Filter, name: &str, near: Option<&Path>) -> Option<PathBuf> {
    #[cfg(phone)]
    {
        let _ = (filter, near);
        phone::outgoing(name)
    }
    #[cfg(desktop)]
    {
        let mut dialog = rfd::FileDialog::new().add_filter(filter.kind, filter.extensions).set_file_name(name);
        if let Some(folder) = near.and_then(Path::parent).filter(|f| crate::fs::is_dir(f)) {
            dialog = dialog.set_directory(folder);
        }
        dialog.save_file()
    }
    #[cfg(web)]
    {
        let _ = (filter, near);
        Some(web::download_path(name))
    }
}

/// A file to write as it is made (a video, as it is encoded): `done` hears it, or none if the player chose none.
/// On the desktop the system's save dialog and the file; in the browser no dialog: the file is made in the page's
/// own storage, written as the bytes come, so nothing of it is kept in memory, and handed over as a download named
/// `name` once whole (the browser's downloads say where it went).
pub fn save_stream(filter: Filter, name: &str, done: impl FnOnce(Option<Stream>) + 'static) {
    #[cfg(not(web))]
    done(save_path(filter, name, None).and_then(file_stream));
    #[cfg(web)]
    {
        let _ = filter;
        web::save_stream(name, Box::new(|made: std::io::Result<Option<Stream>>| done(made.ok().flatten())));
    }
}

/// A file to write as it is made at `path`, the app's choice of place: on the desktop it is written there, no dialog
/// asked; in the browser, which writes nowhere unasked, it is made as [`save_stream`] makes one and handed over as
/// a download named as `path`'s file. `done` hears it: an error if it couldn't be made.
pub fn save_stream_to(filter: Filter, path: &Path, done: impl FnOnce(std::io::Result<Option<Stream>>) + 'static) {
    let _ = filter;
    #[cfg(not(web))]
    done(Stream::at(path.to_path_buf()).map(Some));
    #[cfg(web)]
    web::save_stream(&path.file_name().unwrap_or_default().to_string_lossy(), Box::new(done));
}

#[cfg(not(web))]
fn file_stream(path: PathBuf) -> Option<Stream> {
    match Stream::at(path.clone()) {
        Ok(stream) => Some(stream),
        Err(why) => {
            log::warn!("saving {}: {why}", path.display());
            None
        }
    }
}

/// A file being written as it is made ([`save_stream`]), from any thread: what is written goes to it in order.
/// [`Stream::finish`] ends it whole, with what goes before it put first (a head made last: an MP4's index); dropped
/// unfinished, or [`Stream::abort`]ed, it is let go (in the browser, nothing of it is handed over).
pub struct Stream {
    shared: std::sync::Arc<Shared>,
    finished: bool,
}

struct Shared {
    name: String,
    state: std::sync::Mutex<State>,
}

#[derive(Default)]
struct State {
    /// Bytes waiting to be written (the browser's: its writes are its main thread's), and how many.
    queue: std::collections::VecDeque<Vec<u8>>,
    queued: usize,
    /// The desktop's file: its place, and what is written, as `NAME.body.part` beside it (made whole as
    /// `NAME.part`, its head first, which then takes the name).
    #[cfg(not(web))]
    file: Option<(PathBuf, PathBuf, std::fs::File)>,
    /// The browser's: what goes before what is written, once it is whole.
    #[cfg(web)]
    head: Vec<u8>,
    ended: Option<Ending>,
    failed: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ending {
    Finished,
    Aborted,
}

impl Stream {
    /// The desktop's file at `path`: written beside it as `NAME.body.part`, put after its head as `NAME.part`
    /// once it is whole, which then takes the name (a file there before stays till then); removed if it is let go.
    #[cfg(not(web))]
    fn at(path: PathBuf) -> std::io::Result<Stream> {
        if let Some(folder) = path.parent().filter(|f| !f.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder)?;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let body = path.with_file_name(format!("{name}.body.part"));
        let file = std::fs::File::create(&body)?;
        let state = State { file: Some((path, body, file)), ..State::default() };
        Ok(Stream { shared: std::sync::Arc::new(Shared { name, state: std::sync::Mutex::new(state) }), finished: false })
    }

    /// The file's name, as the player chose it.
    pub fn name(&self) -> &str {
        &self.shared.name
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        crate::thread::lock(&self.shared.state).unwrap_or_else(|e| e.into_inner())
    }

    /// Write `bytes` next (taken as they are: no copy). An error: writing failed (what the system said), and the
    /// file is let go.
    pub fn send(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        let mut state = self.state();
        if let Some(why) = &state.failed {
            return Err(why.clone());
        }
        #[cfg(not(web))]
        if let Some((_, _, file)) = &mut state.file {
            return std::io::Write::write_all(file, &bytes).map_err(|e| e.to_string());
        }
        state.queued += bytes.len();
        state.queue.push_back(bytes);
        Ok(())
    }

    /// How many bytes wait to be written (a writer far ahead waits for them).
    pub fn waiting(&self) -> usize {
        self.state().queued
    }

    /// The file is whole, `head` first and then what was written: on the desktop it is made so and takes its name
    /// (an error: it couldn't be, and nothing of it is left); in the browser what waits is written and it is
    /// handed over as a download (what goes wrong from here is logged).
    pub fn finish(mut self, head: Vec<u8>) -> std::io::Result<()> {
        self.finished = true;
        self.end(Ending::Finished, head)
    }

    /// The file is let go: in the browser nothing of it is handed over; on the desktop it is removed.
    pub fn abort(mut self) {
        self.finished = true;
        let _ = self.end(Ending::Aborted, Vec::new());
    }

    fn end(&mut self, ending: Ending, head: Vec<u8>) -> std::io::Result<()> {
        let mut state = self.state();
        // (A write that failed: the file isn't whole, and is let go.)
        let failed = state.failed.clone().filter(|_| ending == Ending::Finished).map(std::io::Error::other);
        let ending = if failed.is_some() { Ending::Aborted } else { ending };
        state.ended = Some(ending);
        #[cfg(web)]
        let made = {
            state.head = head;
            Ok(())
        };
        #[cfg(not(web))]
        let made = match state.file.take() {
            Some((path, body, file)) => {
                drop(file);
                let made = if ending == Ending::Finished { whole(&path, &body, &head) } else { Ok(()) };
                let _ = std::fs::remove_file(body);
                // (A phone's is in the app's own storage, which the player doesn't see: handed over, once whole.)
                #[cfg(phone)]
                if made.is_ok() && ending == Ending::Finished {
                    phone::hand_over(&path);
                }
                made
            }
            None => Ok(()),
        };
        failed.map_or(made, Err)
    }
}

/// The desktop's file at `path` made whole: `head`, then what was written at `body`; made beside it as `NAME.part`,
/// which takes the name once it is whole (and is removed if it can't be).
#[cfg(not(web))]
fn whole(path: &Path, body: &Path, head: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let part = path.with_file_name(format!("{}.part", path.file_name().unwrap_or_default().to_string_lossy()));
    let made = (|| {
        let mut out = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(&part)?);
        out.write_all(head)?;
        std::io::copy(&mut std::fs::File::open(body)?, &mut out)?;
        out.into_inner().map_err(|e| e.into_error())?.sync_all()?;
        std::fs::rename(&part, path)
    })();
    if made.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    made
}

/// Written as any `std::io::Write` is (a writer of the app's that takes one: a video).
impl std::io::Write for Stream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.send(bytes.to_vec()).map_err(std::io::Error::other)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.end(Ending::Aborted, Vec::new());
        }
    }
}

/// The file [`save_path`] gave is written. In the browser it is handed over as a download and let go of here; on
/// the desktop it is where it was asked to be, and nothing is done.
pub fn saved(path: &Path) {
    #[cfg(web)]
    web::download(path);
    #[cfg(phone)]
    phone::hand_over(path);
    #[cfg(desktop)]
    let _ = path;
}

/// A file of the app's shown to the player: on the desktop in the system's file manager; in the browser, whose
/// files the player can't see, handed over as a download (a copy: the file stays).
pub fn show_file(path: &Path) {
    #[cfg(web)]
    web::send(path);
    #[cfg(phone)]
    phone::share(path);
    #[cfg(desktop)]
    {
        let mut command = if cfg!(target_os = "macos") {
            let mut c = std::process::Command::new("open");
            c.arg("-R").arg(path);
            c
        } else if cfg!(target_os = "windows") {
            let mut select = std::ffi::OsString::from("/select,");
            select.push(path);
            let mut c = std::process::Command::new("explorer");
            c.arg(select);
            c
        } else {
            // (No one way to point at a file: its folder.)
            let mut c = std::process::Command::new("xdg-open");
            c.arg(path.parent().unwrap_or(path));
            c
        };
        if let Err(why) = command.spawn() {
            log::warn!("showing {}: {why}", path.display());
        }
    }
}

/// The dialogs' answers that came since, each heard by its `done`: a phone's come on the system's threads, and wait
/// for this, which the app calls each frame, on its own thread. (On the desktop and in a browser, nothing waits.)
pub fn poll() {
    #[cfg(phone)]
    phone::poll();
}

/// Files and folders dropped on the page go to `dropped`, from now on: a path for each. A file is read into the
/// app's files whole; of a folder, only the files at `wanted` (paths within it, as [`open_folder`] takes them), and
/// its path is a folder holding those (an empty one if none is there). (On the desktop the window's own events
/// carry them, winit's `DroppedFile`, the folder's own path, and this does nothing.)
pub fn take_drops(wanted: &[&str], dropped: impl FnMut(Vec<PathBuf>) + 'static) {
    #[cfg(web)]
    web::take_drops(wanted, Box::new(dropped));
    #[cfg(not(web))]
    drop((wanted, dropped));
}

#[cfg(phone)]
pub(crate) mod phone;
#[cfg(any(phone, test))]
#[cfg_attr(not(phone), allow(dead_code))]
mod wanted;

#[cfg(web)]
mod web {
    use super::{Ending, Filter, Shared, State, Stream};
    use std::sync::Arc;
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

    /// A file made in the page's storage, to be handed over as a download named `name` once whole.
    pub fn save_stream(name: &str, done: Box<dyn FnOnce(std::io::Result<Option<Stream>>)>) {
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
            done(Ok(Some(Stream { shared, finished: false })));
        });
    }

    /// The page's main thread writes what the stream's writer queued, in order, till it ends; whole, the file
    /// (`handle`'s) is handed over, its head first.
    async fn pump(shared: Arc<Shared>, handle: JsValue, writable: Writable) {
        let state = || crate::thread::lock(&shared.state).unwrap_or_else(|e| e.into_inner());
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

    pub fn take_drops(wanted: &[&str], mut dropped: Box<dyn FnMut(Vec<PathBuf>)>) {
        let heard = Closure::<dyn FnMut(js_sys::Array, js_sys::Array)>::new(move |files: js_sys::Array, folders: js_sys::Array| {
            let dir = lot("/dropped");
            let kept = keep(&dir, &files);
            // (Each folder dropped as a path, its files in it; each file dropped outside one as its own.)
            let folders: Vec<PathBuf> = folders.iter().filter_map(|f| f.as_string()).map(|f| dir.join(within(&f))).filter(|f| f != &dir).collect();
            for folder in &folders {
                if let Err(why) = crate::fs::create_dir_all(folder) {
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
        match crate::fs::read(path) {
            Ok(bytes) => nettai_download(&path.file_name().unwrap_or_default().to_string_lossy(), &bytes),
            Err(why) => log::warn!("downloading {}: {why}", path.display()),
        }
    }

    pub fn download(path: &Path) {
        send(path);
        let _ = crate::fs::remove_file(path);
        // (Its folder too, if it was a download's own: `download_path`'s.)
        if let Some(dir) = path.parent() {
            let _ = crate::fs::remove_dir(dir);
        }
    }
}

#[cfg(all(test, not(web)))]
mod tests {
    use super::*;
    use std::io::Write;

    /// A folder of this test's own under the system's temporary one, and what it holds, by name.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nettai-platform-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn held(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    fn stream(path: &Path) -> Stream {
        let made = std::rc::Rc::new(std::cell::RefCell::new(None));
        let heard = made.clone();
        save_stream_to(Filter { kind: "Videos", extensions: &["mp4"] }, path, move |s| *heard.borrow_mut() = Some(s));
        made.take().unwrap().unwrap().unwrap()
    }

    /// Finished, the file is its head and then what was written, under its name, nothing else beside it; a file
    /// there before stays till then. Let go, nothing of it is left, and the file before stays.
    #[test]
    fn a_stream_is_its_head_then_what_was_written() {
        let dir = scratch("stream");
        let path = dir.join("a video.mp4");
        std::fs::write(&path, b"before").unwrap();
        let mut s = stream(&path);
        s.write_all(b"samples ").unwrap();
        s.write_all(b"and more").unwrap();
        assert_eq!(held(&dir), ["a video.mp4", "a video.mp4.body.part"]);
        assert_eq!(std::fs::read(&path).unwrap(), b"before");
        s.finish(b"head: ".to_vec()).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"head: samples and more");
        assert_eq!(held(&dir), ["a video.mp4"]);
        let mut s = stream(&path);
        s.write_all(b"half").unwrap();
        s.abort();
        assert_eq!(held(&dir), ["a video.mp4"]);
        drop(stream(&path));
        assert_eq!((held(&dir), std::fs::read(&path).unwrap()), (vec!["a video.mp4".to_string()], b"head: samples and more".to_vec()));
        let _ = std::fs::remove_dir_all(dir);
    }
}
