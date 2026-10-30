//! In a browser: a thread is a Web Worker that loads the page's own module (the wasm-bindgen shim's JavaScript) on
//! the page's shared memory, runs the work, frees its stack and ends; a lock the main thread takes is tried again
//! till it is free (the page may not wait).

use std::sync::{LockResult, Mutex, MutexGuard, TryLockError};
use wasm_bindgen::prelude::*;

pub fn lock<T: ?Sized>(mutex: &Mutex<T>) -> LockResult<MutexGuard<'_, T>> {
    loop {
        match mutex.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(TryLockError::Poisoned(poisoned)) => return Err(poisoned),
            Err(TryLockError::WouldBlock) => std::hint::spin_loop(),
        }
    }
}

type Work = Box<dyn FnOnce() + Send>;
/// Work that waits on the thread's own event loop: what makes the future, which needn't be `Send`.
type AsyncWork = Box<dyn FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>> + Send>;

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

pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    let work: Work = Box::new(work);
    let work = Box::into_raw(Box::new(work)) as u32;
    nettai_thread_spawn(name, wasm_bindgen::module(), wasm_bindgen::memory(), work, false).map_err(|why| {
        // (The work never runs: it is dropped here, as a thread that couldn't start drops it.)
        drop(unsafe { Box::from_raw(work as *mut Work) });
        std::io::Error::other(format!("{why:?}"))
    })
}

/// Run the future `work` makes on a thread of its own, on its event loop, till it is done: the browser's (a Web
/// Worker), for work that waits on what the browser answers later (WebCodecs: nettai-video). (Only the browser has
/// it.) The future is made there.
pub fn spawn_async(name: &str, work: impl FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>> + Send + 'static) -> std::io::Result<()> {
    let work: AsyncWork = Box::new(work);
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
