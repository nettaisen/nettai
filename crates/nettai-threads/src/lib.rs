//! Threads, the same calls on the desktop, in a browser and on a phone: a thread to work on ([`spawn`]), the locks
//! a thread shares ([`lock`]), and work done off the window's thread for the app's futures ([`off_thread`]).
//!
//! A thread to work on, named: `std::thread`'s on the desktop and a phone. In the browser `std::thread::spawn` can't
//! start one; there it is a Web Worker that loads the page's own module (the wasm-bindgen shim's JavaScript) on the
//! page's shared memory, runs the work, frees its stack and ends. The module must be built with Wasm's atomics
//! (packaging/web), and the page must have said where its shim is ([`SHIM`]).
//!
//! Every lock the page's main thread shares with a thread is taken through [`lock`].
//!
//! Nothing waits for the thread: what it makes goes back by a channel or the event loop. (A browser's page may not
//! block, so there is no `join`.) A thread a worker starts is started by the page, which the worker asks: a
//! worker's own workers would start only while its event loop runs.

mod platform;
mod task;
// (What only one system has, public as the crate's own: the browser's `spawn_async`.)
#[allow(unused_imports)]
pub use platform::*;
pub use task::off_thread;

use platform::system;
use std::sync::{LockResult, Mutex, MutexGuard};

/// `mutex` locked, as `Mutex::lock` does. A browser's page may not wait on its main thread (`Atomics.wait` is
/// refused there, and the module is ended), so in the browser a lock held by another thread is tried again till it
/// is free: every lock the main thread shares with a thread must be taken through this, and held for moments only.
pub fn lock<T: ?Sized>(mutex: &Mutex<T>) -> LockResult<MutexGuard<'_, T>> {
    system::lock(mutex)
}

/// The global the page sets to its shim's URL, before the app starts (packaging/web/index.html).
pub const SHIM: &str = "nettaiShim";

/// Run `work` on a thread of its own, called `name`; an error if none can be started (and `work` is dropped).
pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    system::spawn(name, work)
}
