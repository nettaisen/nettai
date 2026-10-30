//! Threads, the same calls on the desktop, in a browser and on a phone: a thread to work on ([`spawn`]), the locks
//! a thread shares ([`lock`]), and work done off the window's thread for the app's futures ([`off_thread`]).
//!
//! A thread to work on, named: `std::thread`'s on the desktop and a phone. In the browser `std::thread::spawn` can't
//! start one; there it is a Web Worker that loads the page's own module (the wasm-bindgen shim's JavaScript) on the
//! page's shared memory, runs the work, frees its stack and ends. The module must be built with Wasm's atomics
//! (crates/nettai-web), and the page must have said where its shim is ([`SHIM`]).
//!
//! Every lock the page's main thread shares with a thread is taken through [`lock`].
//!
//! Nothing waits for the thread: what it makes goes back by a channel or the event loop. (A browser's page may not
//! block, so there is no `join`.) A thread a worker starts is started by the page, which the worker asks: a
//! worker's own workers would start only while its event loop runs.

mod task;
pub use task::off_thread;

use std::sync::{LockResult, Mutex, MutexGuard};
#[cfg(target_arch = "wasm32")]
use std::sync::TryLockError;

/// `mutex` locked, as `Mutex::lock` does. A browser's page may not wait on its main thread (`Atomics.wait` is
/// refused there, and the module is ended), so in the browser a lock held by another thread is tried again till it
/// is free: every lock the main thread shares with a thread must be taken through this, and held for moments only.
pub fn lock<T: ?Sized>(mutex: &Mutex<T>) -> LockResult<MutexGuard<'_, T>> {
    #[cfg(not(target_arch = "wasm32"))]
    return mutex.lock();
    #[cfg(target_arch = "wasm32")]
    loop {
        match mutex.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(TryLockError::Poisoned(poisoned)) => return Err(poisoned),
            Err(TryLockError::WouldBlock) => std::hint::spin_loop(),
        }
    }
}

/// Run the future `work` makes on a thread of its own, on its event loop, till it is done: the browser's (a
/// Web Worker), for work that waits on what the browser answers later (WebCodecs: nettai-video). (Only the
/// browser has it.)
#[cfg(target_arch = "wasm32")]
pub fn spawn_async(name: &str, work: impl FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>> + Send + 'static) -> std::io::Result<()> {
    web::spawn_async(name, Box::new(work))
}

/// The global the page sets to its shim's URL, before the app starts (crates/nettai-web/web/index.html).
pub const SHIM: &str = "nettaiShim";

/// Run `work` on a thread of its own, called `name`; an error if none can be started (and `work` is dropped).
pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    return std::thread::Builder::new().name(name.to_string()).spawn(work).map(drop);
    #[cfg(target_arch = "wasm32")]
    return web::spawn(name, Box::new(work));
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;

    type Work = Box<dyn FnOnce() + Send>;
    /// Work that waits on the thread's own event loop: what makes the future, which needn't be `Send`.
    pub type AsyncWork = Box<dyn FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>> + Send>;

    #[wasm_bindgen(inline_js = r#"
const WORKER = `
self.onmessage = async (event) => {
    const [shim, module, memory, work, waits] = event.data;
    self.nettaiShim = shim;
    // (A thread this one starts is started by the page: a worker's own workers start only while its event loop
    // runs, and this one may be busy, or waiting on the thread it starts.)
    self.nettaiSpawnHere = (name, work, waits) => self.postMessage(["spawn", name, work, waits]);
    let m;
    try {
        m = await import(shim);
    } catch (why) {
        // (Its scripts not fetched: the page starts another in its place, the work not yet taken.)
        self.postMessage(["failed", String(why)]);
        self.close();
        return;
    }
    const wasm = await m.default({ module_or_path: module, memory });
    if (waits) await m.nettai_thread_run_async(work); else m.nettai_thread_run(work);
    if (wasm.__wbindgen_thread_destroy) wasm.__wbindgen_thread_destroy();
    self.close();
};
`;
let url = null;
// (How many times a worker whose scripts weren't fetched is started again.)
const TRIES = 4;
export function nettai_thread_spawn(name, module, memory, work, waits, tried = 0) {
    if (typeof self.nettaiSpawnHere === "function") return self.nettaiSpawnHere(name, work, waits);
    const shim = globalThis.nettaiShim;
    if (!shim) throw new Error("the page didn't say where its module is (globalThis.nettaiShim)");
    url ??= URL.createObjectURL(new Blob([WORKER], { type: "text/javascript" }));
    const worker = new Worker(url, { type: "module", name });
    worker.onerror = (e) => console.error(`thread ${name}:`, e.message ?? e);
    worker.onmessage = (e) => {
        if (e.data?.[0] === "spawn") nettai_thread_spawn(e.data[1], module, memory, e.data[2], e.data[3]);
        if (e.data?.[0] === "failed") {
            if (tried + 1 < TRIES) {
                console.warn(`thread ${name} didn't start (${e.data[1]}): starting it again`);
                setTimeout(() => nettai_thread_spawn(name, module, memory, work, waits, tried + 1), 100 * (tried + 1));
            } else console.error(`thread ${name} didn't start: ${e.data[1]}`);
        }
    };
    worker.postMessage([shim, module, memory, work, waits]);
}
"#)]
    extern "C" {
        #[wasm_bindgen(catch)]
        fn nettai_thread_spawn(name: &str, module: JsValue, memory: JsValue, work: u32, waits: bool) -> Result<(), JsValue>;
    }

    pub fn spawn(name: &str, work: Work) -> std::io::Result<()> {
        let work = Box::into_raw(Box::new(work)) as u32;
        nettai_thread_spawn(name, wasm_bindgen::module(), wasm_bindgen::memory(), work, false).map_err(|why| {
            // (The work never runs: it is dropped here, as a thread that couldn't start drops it.)
            drop(unsafe { Box::from_raw(work as *mut Work) });
            std::io::Error::other(format!("{why:?}"))
        })
    }

    /// A Web Worker of its own that runs a future on its event loop (what WebCodecs and the like answer on) till it
    /// is done; the future is made there.
    pub fn spawn_async(name: &str, work: AsyncWork) -> std::io::Result<()> {
        let work = Box::into_raw(Box::new(work)) as u32;
        nettai_thread_spawn(name, wasm_bindgen::module(), wasm_bindgen::memory(), work, true).map_err(|why| {
            drop(unsafe { Box::from_raw(work as *mut AsyncWork) });
            std::io::Error::other(format!("{why:?}"))
        })
    }

    /// An async thread's start: the future the work makes, run to its end.
    #[wasm_bindgen]
    pub fn nettai_thread_run_async(work: u32) -> js_sys::Promise {
        let work = unsafe { Box::from_raw(work as *mut AsyncWork) };
        wasm_bindgen_futures::future_to_promise(async move {
            work().await;
            Ok(JsValue::UNDEFINED)
        })
    }

    /// The new thread's start: the work handed over by [`spawn`].
    #[wasm_bindgen]
    pub fn nettai_thread_run(work: u32) {
        let work = unsafe { Box::from_raw(work as *mut Work) };
        work();
    }
}
