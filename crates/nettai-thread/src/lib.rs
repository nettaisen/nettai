//! A thread to work on, named: `std::thread`'s on the desktop. In the browser `std::thread::spawn` can't start
//! one; there it is a Web Worker that loads the page's own module (the wasm-bindgen shim's JavaScript) on the
//! page's shared memory, runs the work, frees its stack and ends. The module must be built with Wasm's atomics
//! (crates/nettai-web), and the page must have said where its shim is ([`SHIM`]).
//!
//! Nothing waits for the thread: what it makes goes back by a channel or the event loop. (A browser's page may not
//! block, so there is no `join`.)

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

    #[wasm_bindgen(inline_js = r#"
const WORKER = `
self.onmessage = async (event) => {
    const [shim, module, memory, work] = event.data;
    self.nettaiShim = shim;
    const m = await import(shim);
    const wasm = await m.default({ module_or_path: module, memory });
    m.nettai_thread_run(work);
    if (wasm.__wbindgen_thread_destroy) wasm.__wbindgen_thread_destroy();
    self.close();
};
`;
let url = null;
export function nettai_thread_spawn(name, module, memory, work) {
    const shim = globalThis.nettaiShim;
    if (!shim) throw new Error("the page didn't say where its module is (globalThis.nettaiShim)");
    url ??= URL.createObjectURL(new Blob([WORKER], { type: "text/javascript" }));
    const worker = new Worker(url, { type: "module", name });
    worker.onerror = (e) => console.error(`thread ${name}:`, e.message ?? e);
    worker.postMessage([shim, module, memory, work]);
}
"#)]
    extern "C" {
        #[wasm_bindgen(catch)]
        fn nettai_thread_spawn(name: &str, module: JsValue, memory: JsValue, work: u32) -> Result<(), JsValue>;
    }

    pub fn spawn(name: &str, work: Work) -> std::io::Result<()> {
        let work = Box::into_raw(Box::new(work)) as u32;
        nettai_thread_spawn(name, wasm_bindgen::module(), wasm_bindgen::memory(), work).map_err(|why| {
            // (The work never runs: it is dropped here, as a thread that couldn't start drops it.)
            drop(unsafe { Box::from_raw(work as *mut Work) });
            std::io::Error::other(format!("{why:?}"))
        })
    }

    /// The new thread's start: the work handed over by [`spawn`].
    #[wasm_bindgen]
    pub fn nettai_thread_run(work: u32) {
        let work = unsafe { Box::from_raw(work as *mut Work) };
        work();
    }
}
