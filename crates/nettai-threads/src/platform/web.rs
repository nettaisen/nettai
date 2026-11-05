//! In a browser: a thread is a Web Worker that loads the page's own module (the wasm-bindgen shim's JavaScript) on
//! the page's shared memory, runs the work, frees its stack and ends; a lock the main thread takes is tried again
//! till it is free (the page may not wait).

use std::cell::OnceCell;
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

/// A worker's own script: it loads the page's module (the shim at the URL it is sent) on the module and memory it is
/// sent, runs the work (a [`Work`], or an [`AsyncWork`] if it waits), frees its stack and ends.
const WORKER: &str = r#"
self.onmessage = async (event) => {
    const [shim, module, memory, work, waits] = event.data;
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
"#;

/// How many times a worker whose scripts weren't fetched is started.
const TRIES: u32 = 4;

/// The URL of the worker's script ([`WORKER`]), made once.
fn script() -> Result<String, JsValue> {
    thread_local! {
        static URL: OnceCell<String> = const { OnceCell::new() };
    }
    URL.with(|url| {
        if let Some(url) = url.get() {
            return Ok(url.clone());
        }
        let options = web_sys::BlobPropertyBag::new();
        options.set_type("text/javascript");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&js_sys::Array::of1(&WORKER.into()), &options)?;
        let made = web_sys::Url::create_object_url_with_blob(&blob)?;
        Ok(url.get_or_init(|| made).clone())
    })
}

/// Start a worker called `name` to run `work` (a [`Work`], or an [`AsyncWork`] if it `waits`). A worker's own workers
/// would start only while its event loop runs, and it may be busy, or waiting on the thread it starts: so a worker
/// asks the page, which starts it.
fn start(name: &str, work: u32, waits: bool) -> Result<(), JsValue> {
    match js_sys::global().dyn_ref::<web_sys::DedicatedWorkerGlobalScope>() {
        Some(worker) => worker.post_message(&js_sys::Array::of4(&"spawn".into(), &name.into(), &work.into(), &waits.into())),
        None => start_here(name, work, waits, 0),
    }
}

/// [`start`] on the page, the `tried`th time.
fn start_here(name: &str, work: u32, waits: bool, tried: u32) -> Result<(), JsValue> {
    let shim = js_sys::Reflect::get(&js_sys::global(), &crate::SHIM.into())?;
    if !shim.is_string() {
        return Err(js_sys::Error::new(&format!("the page didn't say where its module is (globalThis.{})", crate::SHIM)).into());
    }
    let options = web_sys::WorkerOptions::new();
    options.set_type(web_sys::WorkerType::Module);
    options.set_name(name);
    let worker = web_sys::Worker::new_with_options(&script()?, &options)?;
    let failed = {
        let name = name.to_string();
        Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
            let why: JsValue = e.dyn_ref::<web_sys::ErrorEvent>().map(|e| e.message().into()).unwrap_or_else(|| e.into());
            web_sys::console::error_2(&format!("thread {name}:").into(), &why);
        })
    };
    let heard = {
        let name = name.to_string();
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            let Ok(said) = e.data().dyn_into::<js_sys::Array>() else { return };
            match said.get(0).as_string().as_deref() {
                // (A thread the worker starts.)
                Some("spawn") => {
                    let name = said.get(1).as_string().unwrap_or_default();
                    let (work, waits) = (said.get(2).as_f64().unwrap_or_default() as u32, said.get(3).as_bool().unwrap_or_default());
                    if let Err(why) = start_here(&name, work, waits, 0) {
                        web_sys::console::error_2(&format!("thread {name} didn't start:").into(), &why);
                    }
                }
                Some("failed") => {
                    let why = said.get(1).as_string().unwrap_or_default();
                    if tried + 1 >= TRIES {
                        web_sys::console::error_1(&format!("thread {name} didn't start: {why}").into());
                        return;
                    }
                    web_sys::console::warn_1(&format!("thread {name} didn't start ({why}): starting it again").into());
                    let again = {
                        let name = name.clone();
                        Closure::once_into_js(move || {
                            if let Err(why) = start_here(&name, work, waits, tried + 1) {
                                web_sys::console::error_2(&format!("thread {name} didn't start:").into(), &why);
                            }
                        })
                    };
                    if let Some(window) = web_sys::window() {
                        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(again.unchecked_ref(), 100 * (tried as i32 + 1));
                    }
                }
                _ => {}
            }
        })
    };
    // (The worker's for as long as it is: let go with it.)
    worker.set_onerror(Some(failed.into_js_value().unchecked_ref()));
    worker.set_onmessage(Some(heard.into_js_value().unchecked_ref()));
    worker.post_message(&js_sys::Array::of5(&shim, &wasm_bindgen::module(), &wasm_bindgen::memory(), &work.into(), &waits.into()))
}

pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
    let work: Work = Box::new(work);
    let work = Box::into_raw(Box::new(work)) as u32;
    start(name, work, false).map_err(|why| {
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
    start(name, work, true).map_err(|why| {
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
