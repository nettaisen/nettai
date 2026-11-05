//! The browser's: its file input, its folder dialog (`showDirectoryPicker`), drops on the page, downloads, and its
//! clipboard (`navigator.clipboard`). A browser's files have no path the app can read, so those chosen are read into
//! the app's files (nettai-files, the page's tree), each lot in a folder of its own, and the app is given their paths
//! there. A file written as it is made ([`Writing`]) is made in the page's own storage, written as the bytes come
//! (on the page's main thread: [`pump`]), and handed over as a download once whole.

use crate::platform::wanted::{is_wanted, leads_to};
use crate::stream::Ending;
use crate::{Filter, Stream};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use wasm_bindgen::prelude::*;
use web_sys::{Blob, File, FileSystemDirectoryEntry, FileSystemDirectoryHandle, FileSystemFileEntry, FileSystemFileHandle, FileSystemWritableFileStream};

fn window() -> Result<web_sys::Window, JsValue> {
    web_sys::window().ok_or_else(|| js_sys::Error::new("the page has no window").into())
}

fn document() -> Result<web_sys::Document, JsValue> {
    window()?.document().ok_or_else(|| js_sys::Error::new("the page has no document").into())
}

/// What `promise` gives, as a `T`.
async fn awaited<T: JsCast>(promise: js_sys::Promise) -> Result<T, JsValue> {
    promise.await?.dyn_into()
}

/// A promise kept once `ms` milliseconds have passed.
fn sleep(ms: i32) -> js_sys::Promise {
    js_sys::Promise::new(&mut |kept, _| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&kept, ms);
        }
    })
}

/// `bytes` copied out of the shared memory: the browser takes no view of it.
fn copied(bytes: &[u8]) -> js_sys::Uint8Array {
    let copy = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    copy.copy_from(bytes);
    copy
}

/// `blob` handed over as a download named `name`.
fn hand_over_blob(blob: &Blob, name: &str) -> Result<(), JsValue> {
    let url = web_sys::Url::create_object_url_with_blob(blob)?;
    let a: web_sys::HtmlAnchorElement = document()?.create_element("a")?.unchecked_into();
    a.set_href(&url);
    a.set_download(name);
    a.click();
    // (Let go once the download is long begun.)
    let revoke = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    window()?.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 60_000)?;
    Ok(())
}

/// A file the player gave: its path within what was given, and the browser's file.
type Given = (String, File);

/// The files `given`, read whole: each its path and bytes.
async fn read(given: Vec<Given>) -> Result<Vec<(String, Vec<u8>)>, JsValue> {
    let mut out = Vec::with_capacity(given.len());
    for (path, file) in given {
        let buffer: js_sys::ArrayBuffer = awaited(file.array_buffer()).await?;
        out.push((path, js_sys::Uint8Array::new(&buffer).to_vec()));
    }
    Ok(out)
}

/// A folder the player gave: picked in the folder dialog (its handle), or dropped on the page (its entry, the File
/// and Directory Entries API's: the one every browser has for drops).
enum Folder {
    Picked(FileSystemDirectoryHandle),
    Dropped(FileSystemDirectoryEntry),
}

/// What is in a folder given: a folder, or a file (a handle or an entry, as its folder's).
enum Item {
    Folder(Folder),
    PickedFile(FileSystemFileHandle),
    DroppedFile(FileSystemFileEntry),
}

impl Folder {
    /// What is in it, by name.
    async fn list(&self) -> Result<Vec<(String, Item)>, JsValue> {
        let mut out = Vec::new();
        match self {
            Folder::Picked(dir) => {
                let handles = dir.values();
                loop {
                    let next: js_sys::IteratorNext = handles.next()?.await?.unchecked_into();
                    if next.done() {
                        break;
                    }
                    let handle: web_sys::FileSystemHandle = next.value().unchecked_into();
                    let item = match handle.kind() {
                        web_sys::FileSystemHandleKind::Directory => Item::Folder(Folder::Picked(handle.clone().unchecked_into())),
                        web_sys::FileSystemHandleKind::File => Item::PickedFile(handle.clone().unchecked_into()),
                        _ => continue,
                    };
                    out.push((handle.name(), item));
                }
            }
            // (Its entries come in batches, till one is empty.)
            Folder::Dropped(dir) => {
                let reader = dir.create_reader();
                loop {
                    let batch = js_sys::Promise::new(&mut |kept, failed| {
                        if let Err(why) = reader.read_entries_with_callback_and_callback(&kept, &failed) {
                            let _ = failed.call1(&JsValue::NULL, &why);
                        }
                    });
                    let batch: js_sys::Array = awaited(batch).await?;
                    if batch.length() == 0 {
                        break;
                    }
                    for entry in batch.iter() {
                        let entry: web_sys::FileSystemEntry = entry.unchecked_into();
                        let item = if entry.is_directory() {
                            Item::Folder(Folder::Dropped(entry.clone().unchecked_into()))
                        } else if entry.is_file() {
                            Item::DroppedFile(entry.clone().unchecked_into())
                        } else {
                            continue;
                        };
                        out.push((entry.name(), item));
                    }
                }
            }
        }
        Ok(out)
    }
}

/// The browser's file of a file handle or entry.
async fn file_of(item: &Item) -> Result<Option<File>, JsValue> {
    Ok(match item {
        Item::PickedFile(handle) => Some(awaited(handle.get_file()).await?),
        Item::DroppedFile(entry) => Some(awaited(js_sys::Promise::new(&mut |kept, failed| entry.file_with_callback_and_callback(&kept, &failed))).await?),
        Item::Folder(_) => None,
    })
}

/// The files of `folder` at `wanted` (paths within it: `crate::platform::wanted`'s), by their paths in it. Only the
/// folders on the way to one are looked in.
async fn wanted_files(folder: Folder, wanted: &[String]) -> Result<Vec<Given>, JsValue> {
    let mut out = Vec::new();
    let mut folders = vec![(String::new(), folder)];
    while let Some((at, folder)) = folders.pop() {
        for (name, item) in folder.list().await? {
            let path = if at.is_empty() { name } else { format!("{at}/{name}") };
            match item {
                Item::Folder(sub) if leads_to(&path, wanted) => folders.push((path, sub)),
                Item::Folder(_) => {}
                file if is_wanted(&path, wanted) => out.extend(file_of(&file).await?.map(|f| (path, f))),
                _ => {}
            }
        }
    }
    Ok(out)
}

/// The files picked in a file input, `extensions` theirs: once picked (none if the player cancels). The input is
/// clicked at once: the dialog wants the click that asks for it.
fn pick_files(extensions: &[&str]) -> Result<impl Future<Output = Result<Vec<Given>, JsValue>> + use<>, JsValue> {
    let input: web_sys::HtmlInputElement = document()?.create_element("input")?.unchecked_into();
    input.set_type("file");
    input.set_multiple(true);
    input.set_accept(&extensions.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(","));
    // (Its change, or its cancel: whichever comes.)
    let chosen = js_sys::Promise::new(&mut |kept, _| {
        let _ = input.add_event_listener_with_callback("change", &kept);
        let _ = input.add_event_listener_with_callback("cancel", &kept);
    });
    input.click();
    Ok(async move {
        chosen.await?;
        let files = input.files().map(|list| (0..list.length()).filter_map(|i| list.get(i)).map(|f| (f.name(), f)).collect());
        Ok(files.unwrap_or_default())
    })
}

/// The folder picked in the folder dialog (`showDirectoryPicker`), to read: asked at once, as the dialog wants.
fn pick_folder() -> Result<js_sys::Promise<FileSystemDirectoryHandle>, JsValue> {
    let window = window()?;
    if !js_sys::Reflect::has(&window, &"showDirectoryPicker".into())? {
        return Err(js_sys::Error::new("this browser has no folder dialog (showDirectoryPicker)").into());
    }
    let options = web_sys::DirectoryPickerOptions::new();
    options.set_mode(web_sys::FileSystemPermissionMode::Read);
    window.show_directory_picker_with_options(&options)
}

/// A file made in the page's own storage, to be written as it comes: in a folder of its own, out of the app's kept
/// files, which the page empties as it starts (index.html's MAKING). One at a time, each in the last's place: the
/// last's download is long done by the time another is whole.
async fn make_file() -> Result<(FileSystemFileHandle, FileSystemWritableFileStream), JsValue> {
    let root: FileSystemDirectoryHandle = awaited(window()?.navigator().storage().get_directory()).await?;
    let made = web_sys::FileSystemGetDirectoryOptions::new();
    made.set_create(true);
    let dir: FileSystemDirectoryHandle = awaited(root.get_directory_handle_with_options(".making", &made)).await?;
    let made = web_sys::FileSystemGetFileOptions::new();
    made.set_create(true);
    let handle: FileSystemFileHandle = awaited(dir.get_file_handle_with_options("file", &made)).await?;
    let writable = awaited(handle.create_writable()).await?;
    Ok((handle, writable))
}

/// The file made (`handle`'s), handed over as a download named `name`: `head`, then what was written (read as it
/// downloads).
async fn hand_over(handle: &FileSystemFileHandle, head: &[u8], name: &str) -> Result<(), JsValue> {
    let file: Blob = awaited(handle.get_file()).await?;
    // (Parts of either kind: a Blob takes both.)
    let blob = Blob::new_with_blob_sequence(&js_sys::Array::of2(&copied(head), &file))?;
    hand_over_blob(&blob, name)
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
    let Some(navigator) = web_sys::window().map(|w| w.navigator()) else { return };
    // (A page that isn't secure has none.)
    if js_sys::Reflect::get(&navigator, &"clipboard".into()).is_ok_and(|c| c.is_undefined()) {
        return;
    }
    let written = navigator.clipboard().write_text(text);
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(why) = written.await {
            log::warn!("copying to the clipboard: {why:?}");
        }
    });
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
    let name = name.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let (handle, writable) = match make_file().await {
            Ok(made) => made,
            Err(why) => return done(Err(std::io::Error::other(format!("{why:?}")))),
        };
        let shared = Arc::new(Shared { name, state: std::sync::Mutex::new(State::default()) });
        wasm_bindgen_futures::spawn_local(pump(shared.clone(), handle, writable));
        done(Ok(Some(Stream::new(Writing { shared }))));
    });
}

/// The page's main thread writes what the stream's writer queued, in order, till it ends; whole, the file
/// (`handle`'s) is handed over, its head first.
async fn pump(shared: Arc<Shared>, handle: FileSystemFileHandle, writable: FileSystemWritableFileStream) {
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
                let _ = writable.abort().await;
                return;
            }
            (Some(bytes), _) => {
                let written = match writable.write_with_js_u8_array(&copied(&bytes)) {
                    Ok(writing) => writing.await.map(drop),
                    Err(why) => Err(why),
                };
                if let Err(why) = written {
                    log::warn!("writing {}: {why:?}", shared.name);
                    state().failed = Some(format!("{why:?}"));
                    let _ = writable.abort().await;
                    return;
                }
            }
            (None, Some(Ending::Finished)) => {
                if let Err(why) = writable.close().await {
                    log::warn!("closing {}: {why:?}", shared.name);
                    return;
                }
                let head = std::mem::take(&mut state().head);
                if let Err(why) = hand_over(&handle, &head, &shared.name).await {
                    log::warn!("handing over {}: {why:?}", shared.name);
                }
                return;
            }
            (None, None) => {
                let _ = sleep(4).await;
            }
        }
    }
}

/// A folder of the app's files for one lot of files chosen.
fn lot(under: &str) -> PathBuf {
    static LOTS: AtomicU32 = AtomicU32::new(0);
    PathBuf::from(under).join((LOTS.fetch_add(1, Ordering::Relaxed) + 1).to_string())
}

/// A path within what was chosen, as a path within the lot's folder alone: no `..` or root takes it out.
fn within(path: &str) -> PathBuf {
    Path::new(path).components().filter(|c| matches!(c, std::path::Component::Normal(_))).collect()
}

/// Files read (each its path within what was chosen, and its bytes) put in the app's files under `dir`; their
/// paths there.
fn keep(dir: &Path, files: Vec<(String, Vec<u8>)>) -> Vec<PathBuf> {
    files
        .into_iter()
        .map(|(path, bytes)| {
            let path = dir.join(within(&path));
            nettai_files::preload(&path, bytes);
            path
        })
        .collect()
}

/// What the player gave, read; or None, and why said (but for a dialog closed: an AbortError, nothing to say).
fn answer<T>(given: Result<T, JsValue>, doing: &str) -> Option<T> {
    given
        .map_err(|why| {
            if why.dyn_ref::<js_sys::Error>().is_none_or(|e| e.name() != "AbortError") {
                log::warn!("{doing}: {why:?}");
            }
        })
        .ok()
}

pub fn open_files(filter: Filter, done: Box<dyn FnOnce(Vec<PathBuf>)>) {
    let picking = pick_files(filter.extensions);
    wasm_bindgen_futures::spawn_local(async move {
        let files = async { read(picking?.await?).await }.await;
        done(answer(files, "choosing files").map(|f| keep(&lot("/picked"), f)).unwrap_or_default());
    });
}

pub fn open_folder(wanted: &[&str], done: Box<dyn FnOnce(Option<PathBuf>)>) {
    let wanted: Vec<String> = wanted.iter().map(|w| w.to_string()).collect();
    let picking = pick_folder();
    wasm_bindgen_futures::spawn_local(async move {
        let files = async { read(wanted_files(Folder::Picked(picking?.await?), &wanted).await?).await }.await;
        done(answer(files, "choosing a folder").map(|files| {
            let dir = lot("/picked");
            keep(&dir, files);
            dir
        }));
    });
}

/// What was dropped, read (each file's path within what was dropped, and its bytes; the folders dropped by name), put
/// in the app's files: each folder dropped as a path, its files in it; each file dropped outside one as its own.
fn dropped_in(files: Vec<(String, Vec<u8>)>, folders: Vec<String>) -> Vec<PathBuf> {
    let dir = lot("/dropped");
    let kept = keep(&dir, files);
    let folders: Vec<PathBuf> = folders.iter().map(|f| dir.join(within(f))).filter(|f| f != &dir).collect();
    for folder in &folders {
        if let Err(why) = nettai_files::create_dir_all(folder) {
            log::warn!("keeping the folder dropped, {}: {why}", folder.display());
        }
    }
    let loose = kept.into_iter().filter(|p| !folders.iter().any(|f| p.starts_with(f)));
    folders.iter().cloned().chain(loose).collect()
}

/// A drop's files and folders (only their `wanted` files), read.
async fn read_drop(taken: Vec<(Option<web_sys::FileSystemEntry>, Option<File>)>, wanted: &[String]) -> Result<(Vec<(String, Vec<u8>)>, Vec<String>), JsValue> {
    let (mut given, mut folders) = (Vec::new(), Vec::new());
    for (entry, file) in taken {
        match (entry, file) {
            (Some(entry), _) if entry.is_directory() => {
                let name = entry.name();
                match wanted_files(Folder::Dropped(entry.unchecked_into()), wanted).await {
                    Ok(files) => given.extend(files.into_iter().map(|(path, file)| (format!("{name}/{path}"), file))),
                    Err(why) => log::warn!("reading the folder dropped, {name}: {why:?}"),
                }
                folders.push(name);
            }
            (_, Some(file)) => given.push((file.name(), file)),
            _ => {}
        }
    }
    Ok((read(given).await?, folders))
}

pub fn take_drops(wanted: &[&str], dropped: Box<dyn FnMut(Vec<PathBuf>)>) {
    let Ok(document) = document() else { return };
    let wanted: Rc<[String]> = wanted.iter().map(|w| w.to_string()).collect();
    let dropped = Rc::new(RefCell::new(dropped));
    let over = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| e.prevent_default());
    let dropping = Closure::<dyn FnMut(web_sys::DragEvent)>::new(move |e: web_sys::DragEvent| {
        e.prevent_default();
        // (Each item's entry and file taken at once: the drop's data is gone once the handler waits.)
        let mut taken = Vec::new();
        if let Some(items) = e.data_transfer().map(|d| d.items()) {
            for item in (0..items.length()).filter_map(|i| items.get(i)).filter(|i| i.kind() == "file") {
                taken.push((item.webkit_get_as_entry().ok().flatten(), item.get_as_file().ok().flatten()));
            }
        }
        let (wanted, dropped) = (wanted.clone(), dropped.clone());
        wasm_bindgen_futures::spawn_local(async move {
            match read_drop(taken, &wanted).await {
                Ok((files, folders)) if !files.is_empty() || !folders.is_empty() => (dropped.borrow_mut())(dropped_in(files, folders)),
                Ok(_) => {}
                Err(why) => log::warn!("reading what was dropped: {why:?}"),
            }
        });
    });
    let _ = document.add_event_listener_with_callback("dragover", over.as_ref().unchecked_ref());
    let _ = document.add_event_listener_with_callback("drop", dropping.as_ref().unchecked_ref());
    // (The page's for as long as it is open.)
    over.forget();
    dropping.forget();
}

pub fn download_path(name: &str) -> PathBuf {
    lot("/downloads").join(name)
}

/// The file at `path` downloaded, and kept.
pub fn send(path: &Path) {
    match nettai_files::read(path) {
        Ok(bytes) => {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if let Err(why) = Blob::new_with_u8_array_sequence(&js_sys::Array::of1(&copied(&bytes))).and_then(|blob| hand_over_blob(&blob, &name)) {
                log::warn!("downloading {}: {why:?}", path.display());
            }
        }
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
