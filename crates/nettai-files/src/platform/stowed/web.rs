//! The browser's stowed files: kept in the page's storage (the origin private file system), written to it and read
//! from it, each by a thread of its own (a Web Worker: `nettai_threads::spawn_async`) whose event loop awaits the
//! browser.

use crate::platform::files;
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetDirectoryOptions, FileSystemGetFileOptions, FileSystemWritableFileStream};

/// What `promise` gives, as a `T`.
async fn awaited<T: JsCast>(promise: js_sys::Promise) -> Result<T, JsValue> {
    JsFuture::from(promise).await?.dyn_into()
}

/// The folder `parts` of the page's storage, its folders made if `make`.
async fn folder(parts: &[&str], make: bool) -> Result<FileSystemDirectoryHandle, JsValue> {
    let worker: web_sys::WorkerGlobalScope = js_sys::global().dyn_into()?;
    let mut dir: FileSystemDirectoryHandle = awaited(worker.navigator().storage().get_directory()).await?;
    let options = FileSystemGetDirectoryOptions::new();
    options.set_create(make);
    for part in parts {
        dir = awaited(dir.get_directory_handle_with_options(part, &options)).await?;
    }
    Ok(dir)
}

/// `at`, a path in the page's storage: its folder's parts, and its name.
fn split(at: &str) -> (Vec<&str>, &str) {
    let mut parts: Vec<&str> = at.split('/').filter(|p| !p.is_empty()).collect();
    let name = parts.pop().unwrap_or_default();
    (parts, name)
}

/// Write `bytes` at `at` in the page's storage, its folders made.
async fn keep(at: &str, bytes: &[u8]) -> Result<(), JsValue> {
    let (parts, name) = split(at);
    let options = FileSystemGetFileOptions::new();
    options.set_create(true);
    let file: FileSystemFileHandle = awaited(folder(&parts, true).await?.get_file_handle_with_options(name, &options)).await?;
    let writable: FileSystemWritableFileStream = awaited(file.create_writable()).await?;
    // (Copied out of the shared memory: the browser takes no view of it.)
    let copy = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    copy.copy_from(bytes);
    JsFuture::from(writable.write_with_js_u8_array(&copy)?).await?;
    JsFuture::from(writable.close()).await?;
    Ok(())
}

/// The bytes at `at` in the page's storage.
async fn read(at: &str) -> Result<Vec<u8>, JsValue> {
    let (parts, name) = split(at);
    let file: FileSystemFileHandle = awaited(folder(&parts, false).await?.get_file_handle(name)).await?;
    let blob: web_sys::Blob = awaited(file.get_file()).await?;
    let buffer: js_sys::ArrayBuffer = awaited(blob.array_buffer()).await?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// The stowed files the page listed as it started, and those stowed since, by their paths in the app's files.
static KNOWN: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

/// A stowed file the page has (`path`, in the app's files): the page lists each as it starts (crates/nettai-web).
pub fn listed(path: PathBuf) {
    if let Ok(mut known) = nettai_threads::lock(&KNOWN) {
        known.insert(path);
    }
}

/// `path` as the page's storage has it: within the data folder (the folder whose changes are kept).
fn in_storage(path: &Path) -> io::Result<String> {
    let root = files::watched().ok_or_else(|| io::Error::other("no data folder is kept"))?;
    let within = path.strip_prefix(&root).map_err(|_| io::Error::other(format!("{} isn't in the data folder", path.display())))?;
    Ok(within.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"))
}

pub fn stow(path: &Path, bytes: Vec<u8>) -> io::Result<()> {
    let at = in_storage(path)?;
    if let Some(dir) = path.parent() {
        files::create_dir_all(dir)?;
    }
    files::write(path, &bytes)?;
    listed(path.to_path_buf());
    let name = path.display().to_string();
    nettai_threads::spawn_async("stowing", move || {
        Box::pin(async move {
            if let Err(why) = keep(&at, &bytes).await {
                log::warn!("keeping {name}: {why:?}");
            }
        })
    })
}

pub fn stowed(dir: &Path) -> Vec<PathBuf> {
    let mut out: BTreeSet<PathBuf> = nettai_threads::lock(&KNOWN).map(|k| k.iter().filter(|p| p.parent() == Some(dir)).cloned().collect()).unwrap_or_default();
    if let Ok(read) = files::read_dir(dir) {
        out.extend(read.flatten().map(|e| e.path()).filter(|p| crate::is_file(p)));
    }
    out.into_iter().collect()
}

pub fn fetch(paths: &[PathBuf]) -> io::Result<()> {
    let wanted: Vec<PathBuf> = paths.iter().filter(|p| !crate::is_file(p)).cloned().collect();
    if wanted.is_empty() {
        return Ok(());
    }
    let at: Vec<(PathBuf, String)> = wanted.into_iter().map(|p| in_storage(&p).map(|s| (p, s))).collect::<io::Result<_>>()?;
    // (Read on a thread of its own, whose event loop awaits the page's storage, while this one waits.)
    let done: Arc<(Mutex<Option<io::Result<()>>>, Condvar)> = Arc::default();
    let told = done.clone();
    nettai_threads::spawn_async("fetching", move || {
        Box::pin(async move {
            let mut result = Ok(());
            for (path, at) in at {
                match read(&at).await {
                    Ok(bytes) => files::preload(&path, bytes),
                    Err(why) => {
                        result = Err(io::Error::new(io::ErrorKind::NotFound, format!("{}: {why:?}", path.display())));
                        break;
                    }
                }
            }
            if let Ok(mut slot) = told.0.lock() {
                *slot = Some(result);
            }
            told.1.notify_all();
        })
    })?;
    let mut slot = done.0.lock().map_err(|_| io::Error::other("fetching stopped"))?;
    loop {
        if let Some(result) = slot.take() {
            return result;
        }
        slot = done.1.wait(slot).map_err(|_| io::Error::other("fetching stopped"))?;
    }
}
