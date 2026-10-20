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

/// A file to write as it is made (a video, as it is encoded): `done` hears it, or none if the player chose none.
/// On the desktop the system's save dialog and the file; in the browser its save dialog (`showSaveFilePicker`, which
/// a page may open only on a click or a key) and the file's writable stream, written as the bytes come, so nothing
/// of it is kept in memory.
pub fn save_stream(filter: Filter, name: &str, done: impl FnOnce(Option<Stream>) + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    done(save_path(filter, name, None).and_then(file_stream));
    #[cfg(target_arch = "wasm32")]
    web::save_stream(filter, name, Box::new(|chosen: std::io::Result<Option<Stream>>| done(chosen.ok().flatten())));
}

/// A file to write as it is made at `path`, the app's choice of place: on the desktop it is written there, no dialog
/// asked; in the browser, which writes nowhere unasked, its save dialog asks, `path`'s name the one suggested.
/// `done` hears it: none if the player chose none, an error if it couldn't be made.
pub fn save_stream_to(filter: Filter, path: &Path, done: impl FnOnce(std::io::Result<Option<Stream>>) + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = filter;
        done(Stream::at(path.to_path_buf()).map(Some));
    }
    #[cfg(target_arch = "wasm32")]
    web::save_stream(filter, &path.file_name().unwrap_or_default().to_string_lossy(), Box::new(done));
}

#[cfg(not(target_arch = "wasm32"))]
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
/// [`Stream::finish`] ends it whole; dropped unfinished, or [`Stream::abort`]ed, it is let go (in the browser,
/// nothing of it is kept).
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
    /// The desktop's file.
    #[cfg(not(target_arch = "wasm32"))]
    /// (Written as `NAME.part` beside its place, which takes the name once it is whole.)
    file: Option<(PathBuf, PathBuf, std::fs::File)>,
    ended: Option<Ending>,
    failed: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ending {
    Finished,
    Aborted,
}

impl Stream {
    /// The desktop's file at `path`: written beside it as `NAME.part`, which takes the name once it is whole (a
    /// file there before stays till then), and is removed if it is let go.
    #[cfg(not(target_arch = "wasm32"))]
    fn at(path: PathBuf) -> std::io::Result<Stream> {
        if let Some(folder) = path.parent().filter(|f| !f.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder)?;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let part = path.with_file_name(format!("{name}.part"));
        let file = std::fs::File::create(&part)?;
        let state = State { file: Some((path, part, file)), ..State::default() };
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
        #[cfg(not(target_arch = "wasm32"))]
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

    /// The file is whole: what waits is written, and it is closed.
    pub fn finish(mut self) {
        self.finished = true;
        self.end(Ending::Finished);
    }

    /// The file is let go: in the browser nothing of it is kept; on the desktop it is removed.
    pub fn abort(mut self) {
        self.finished = true;
        self.end(Ending::Aborted);
    }

    fn end(&mut self, ending: Ending) {
        let mut state = self.state();
        state.ended = Some(ending);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((path, part, file)) = state.file.take() {
            let flushed = file.sync_all();
            drop(file);
            let kept = ending == Ending::Finished && flushed.is_ok() && std::fs::rename(&part, &path).is_ok();
            if !kept {
                let _ = std::fs::remove_file(part);
            }
        }
    }
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
            self.end(Ending::Aborted);
        }
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

/// A file of the app's shown to the player: on the desktop in the system's file manager; in the browser, whose
/// files the player can't see, handed over as a download (a copy: the file stays).
pub fn show_file(path: &Path) {
    #[cfg(target_arch = "wasm32")]
    web::send(path);
    #[cfg(not(target_arch = "wasm32"))]
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
export function nettai_take_drops(dropped) {
    document.addEventListener("dragover", (e) => e.preventDefault());
    document.addEventListener("drop", async (e) => {
        e.preventDefault();
        const files = [...(e.dataTransfer?.files ?? [])];
        if (files.length) dropped(await read(files.map((f) => [f.name, f])));
    });
}
export async function nettai_save_stream(kind, extensions, name) {
    if (!window.showSaveFilePicker) throw new Error("this browser has no save dialog (showSaveFilePicker)");
    const accept = { "application/octet-stream": extensions.map((e) => "." + e) };
    const handle = await window.showSaveFilePicker({ suggestedName: name, types: [{ description: kind, accept }] });
    return [handle.name, await handle.createWritable()];
}
export function nettai_sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
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
        fn nettai_save_stream(kind: &str, extensions: Vec<String>, name: &str) -> js_sys::Promise;
        fn nettai_sleep(ms: u32) -> js_sys::Promise;

        type Writable;
        #[wasm_bindgen(method)]
        fn write(this: &Writable, bytes: &js_sys::Uint8Array) -> js_sys::Promise;
        #[wasm_bindgen(method)]
        fn close(this: &Writable) -> js_sys::Promise;
        #[wasm_bindgen(method)]
        fn abort(this: &Writable) -> js_sys::Promise;
    }

    pub fn save_stream(filter: Filter, name: &str, done: Box<dyn FnOnce(std::io::Result<Option<Stream>>)>) {
        let asked = nettai_save_stream(filter.kind, filter.extensions.iter().map(|e| e.to_string()).collect(), name);
        wasm_bindgen_futures::spawn_local(async move {
            let chosen = match wasm_bindgen_futures::JsFuture::from(asked).await {
                Ok(chosen) => chosen,
                // (The dialog closed: an AbortError, and nothing chosen.)
                Err(why) if why.dyn_ref::<js_sys::Error>().is_some_and(|e| e.name() == "AbortError") => return done(Ok(None)),
                Err(why) => return done(Err(std::io::Error::other(format!("{why:?}")))),
            };
            let chosen = js_sys::Array::from(&chosen);
            let name = chosen.get(0).as_string().unwrap_or_default();
            let writable: Writable = chosen.get(1).unchecked_into();
            let shared = Arc::new(Shared { name, state: std::sync::Mutex::new(State::default()) });
            wasm_bindgen_futures::spawn_local(pump(shared.clone(), writable));
            done(Ok(Some(Stream { shared, finished: false })));
        });
    }

    /// The page's main thread writes what the stream's writer queued, in order, till it ends.
    async fn pump(shared: Arc<Shared>, writable: Writable) {
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
