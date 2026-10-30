//! In a browser: the files are a tree in memory every thread shares ([`super::memory`]), which the page fills
//! before the app starts and writes back to its own storage (the origin private file system); stowed files are read
//! from that storage when wanted. The page has no folders of the system's: the app's data is the page's `/data`.

pub mod fs {
    pub use super::super::memory::*;
}

pub fn app_data() -> Option<std::path::PathBuf> {
    None
}

pub fn temporary() -> Option<std::path::PathBuf> {
    Some(std::env::temp_dir())
}

use super::memory;
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
async function folder(parts, make) {
    let dir = await navigator.storage.getDirectory();
    for (const part of parts) dir = await dir.getDirectoryHandle(part, { create: make });
    return dir;
}
export async function nettai_stow(path, bytes) {
    const parts = path.split("/").filter(Boolean);
    const name = parts.pop();
    const writable = await (await (await folder(parts, true)).getFileHandle(name, { create: true })).createWritable();
    await writable.write(bytes);
    await writable.close();
}
export async function nettai_fetch(path) {
    const parts = path.split("/").filter(Boolean);
    const name = parts.pop();
    const file = await (await (await folder(parts, false)).getFileHandle(name)).getFile();
    return new Uint8Array(await file.arrayBuffer());
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn nettai_stow(path: &str, bytes: js_sys::Uint8Array) -> Result<(), JsValue>;
    #[wasm_bindgen(catch)]
    async fn nettai_fetch(path: &str) -> Result<JsValue, JsValue>;
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
    let root = memory::watched().ok_or_else(|| io::Error::other("no data folder is kept"))?;
    let within = path.strip_prefix(&root).map_err(|_| io::Error::other(format!("{} isn't in the data folder", path.display())))?;
    Ok(within.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"))
}

pub fn stow(path: &Path, bytes: Vec<u8>) -> io::Result<()> {
    let at = in_storage(path)?;
    if let Some(dir) = path.parent() {
        memory::create_dir_all(dir)?;
    }
    memory::write(path, &bytes)?;
    listed(path.to_path_buf());
    let name = path.display().to_string();
    nettai_threads::spawn_async("stowing", move || {
        Box::pin(async move {
            // (Copied out of the shared memory: the browser takes no view of it.)
            let copy = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
            copy.copy_from(&bytes);
            if let Err(why) = nettai_stow(&at, copy).await {
                log::warn!("keeping {name}: {why:?}");
            }
        })
    })
}

pub fn stowed(dir: &Path) -> Vec<PathBuf> {
    let mut out: BTreeSet<PathBuf> = nettai_threads::lock(&KNOWN).map(|k| k.iter().filter(|p| p.parent() == Some(dir)).cloned().collect()).unwrap_or_default();
    if let Ok(read) = memory::read_dir(dir) {
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
                match nettai_fetch(&at).await {
                    Ok(bytes) => memory::preload(&path, js_sys::Uint8Array::new(&bytes).to_vec()),
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
