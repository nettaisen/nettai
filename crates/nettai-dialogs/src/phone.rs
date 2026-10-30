//! A phone's pickers, its share sheet and its clipboard: iOS's UIKit ([`ios`]), Android's through its activity's
//! Kotlin side ([`android`], with `android/nettai/dialogs/NettaiFiles.kt`).
//!
//! As in a browser, what the player picks is read in: copied into the app's temporary folder, a lot of its own
//! (`picked/<n>`), and the app is given the copies' paths. Of a folder, only the files at `wanted` (paths in it, as
//! [`super::open_folder`] takes them) are copied, at the same paths. A file to save is made in the app's temporary
//! folder (`outgoing`) and handed over once whole: iOS's share sheet (which saves to Files, AirDrops, sends), or
//! Android's save dialog (a document made where the player says, the file copied into it).
//!
//! The answers come on the system's own threads (iOS: the main thread, the app's; Android: Java's), and a folder
//! is copied on a thread of its own, so each waits in a queue until the app's next frame takes it ([`poll`]), which
//! calls the dialog's `done` on the app's thread.

use super::Filter;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "android")]
use android as system;
#[cfg(target_os = "ios")]
use ios as system;

/// A dialog waiting for its answer: what hears it.
enum Waiting {
    Files(Box<dyn FnOnce(Vec<PathBuf>)>),
    Folder(Box<dyn FnOnce(Option<PathBuf>)>),
}

/// A dialog's answer: the files read in, or the folder they were read into (none: nothing chosen, or nothing
/// read; why is logged).
pub(crate) enum Answer {
    Files(Vec<PathBuf>),
    Folder(Option<PathBuf>),
}

thread_local! {
    /// The dialogs waiting, by request (the app's thread's: their `done`s are its).
    static WAITING: RefCell<HashMap<u64, Waiting>> = RefCell::new(HashMap::new());
}

/// The answers come, by request, from any thread.
static ANSWERS: Mutex<Vec<(u64, Answer)>> = Mutex::new(Vec::new());

/// Files to hand over, from any thread (a video is made whole on one of its own): handed over on the app's.
static HANDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// The next request's number.
static NEXT: AtomicU64 = AtomicU64::new(1);

fn ask(waiting: Waiting) -> u64 {
    let request = NEXT.fetch_add(1, Ordering::Relaxed);
    WAITING.with(|w| w.borrow_mut().insert(request, waiting));
    request
}

/// Request `request` answered (from any thread): heard at the app's next frame.
pub(crate) fn answer(request: u64, answer: Answer) {
    ANSWERS.lock().unwrap_or_else(|e| e.into_inner()).push((request, answer));
}

/// The answers that came, each to its dialog's `done`, and what the system wants done on the app's thread after
/// one (iOS: a folder's access let go). The app's frame calls this ([`super::poll`]).
pub fn poll() {
    for path in std::mem::take(&mut *HANDING.lock().unwrap_or_else(|e| e.into_inner())) {
        system::hand_over(path);
    }
    let answers = std::mem::take(&mut *ANSWERS.lock().unwrap_or_else(|e| e.into_inner()));
    for (request, answer) in answers {
        system::answered(request);
        let Some(waiting) = WAITING.with(|w| w.borrow_mut().remove(&request)) else { continue };
        match (waiting, answer) {
            (Waiting::Files(done), Answer::Files(files)) => done(files),
            (Waiting::Folder(done), Answer::Folder(folder)) => done(folder),
            (Waiting::Files(done), Answer::Folder(_)) => done(Vec::new()),
            (Waiting::Folder(done), Answer::Files(_)) => done(None),
        }
    }
}

pub fn open_files(filter: Filter, done: Box<dyn FnOnce(Vec<PathBuf>)>) {
    let request = ask(Waiting::Files(done));
    system::open_files(request, filter);
}

pub fn open_folder(wanted: &[&str], done: Box<dyn FnOnce(Option<PathBuf>)>) {
    let request = ask(Waiting::Folder(done));
    system::open_folder(request, wanted.iter().map(|w| w.to_string()).collect());
}

/// Where a file to hand over is made: the app's temporary folder's `outgoing`.
pub fn outgoing(name: &str) -> Option<PathBuf> {
    Some(nettai_files::folders::temporary()?.join("outgoing").join(name))
}

/// The file at `path` (the app's, made whole) handed over: the share sheet, or the save dialog. From any thread:
/// at the app's next frame.
pub fn hand_over(path: &Path) {
    HANDING.lock().unwrap_or_else(|e| e.into_inner()).push(path.to_path_buf());
}

/// A file of the app's shown to the player (a replay): a copy of it handed over, made in `outgoing` first, as the
/// file handed over is the system's to remove once it is saved (Android's save dialog does).
pub fn share(path: &Path) {
    let copied = (|| -> Result<PathBuf, String> {
        let to = outgoing(&path.file_name().unwrap_or_default().to_string_lossy()).ok_or("no temporary folder")?;
        if let Some(folder) = to.parent() {
            std::fs::create_dir_all(folder).map_err(|e| format!("making {}: {e}", folder.display()))?;
        }
        std::fs::copy(path, &to).map_err(|e| e.to_string())?;
        Ok(to)
    })();
    match copied {
        Ok(copy) => hand_over(&copy),
        Err(why) => log::warn!("sharing {}: {why}", path.display()),
    }
}

pub fn copy(text: &str) {
    system::copy(text);
}

/// The folder request `request`'s files are read into: the temporary folder's `picked/<request>`, made empty.
pub(crate) fn lot(request: u64) -> Result<PathBuf, String> {
    let dir = nettai_files::folders::temporary().ok_or("no temporary folder")?.join("picked").join(request.to_string());
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("making {}: {e}", dir.display()))?;
    Ok(dir)
}
