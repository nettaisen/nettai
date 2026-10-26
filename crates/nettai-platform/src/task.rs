//! A future run to its end ([`spawn`], the `tasks` feature): on the desktop on a tokio runtime of this crate's own
//! (one worker thread, started when first asked for: reqwest's I/O needs it), in the browser on the page's event
//! loop. Its [`Task`] cancels it when dropped, or by [`Task::cancel`]: the future is dropped where it waits (a request
//! of `http::get` aborted with it).

use std::future::Future;

/// What a future [`spawn`] runs must be: `Send` on the desktop (the runtime's thread runs it), anything in the
/// browser (its futures hold the page's objects, and the page's own loop runs them).
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> MaybeSend for T {}
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T {}

/// A future being run ([`spawn`]): cancelled when this is dropped.
pub struct Task {
    #[cfg(not(target_arch = "wasm32"))]
    handle: tokio::task::JoinHandle<()>,
    #[cfg(target_arch = "wasm32")]
    abort: futures_util::future::AbortHandle,
}

impl Task {
    /// It is cancelled now: its future is dropped where it waits, and never runs again.
    pub fn cancel(self) {}
}

impl Drop for Task {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.handle.abort();
        #[cfg(target_arch = "wasm32")]
        self.abort.abort();
    }
}

/// Run `work` to its end, or till its [`Task`] is dropped. Err: the desktop's runtime couldn't start.
pub fn spawn(work: impl Future<Output = ()> + MaybeSend + 'static) -> std::io::Result<Task> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(Task { handle: desktop::runtime()?.spawn(work) })
    }
    #[cfg(target_arch = "wasm32")]
    {
        let (abort, registration) = futures_util::future::AbortHandle::new_pair();
        wasm_bindgen_futures::spawn_local(async move {
            let _ = futures_util::future::Abortable::new(work, registration).await;
        });
        Ok(Task { abort })
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use std::sync::OnceLock;
    use tokio::runtime::{Builder, Runtime};

    /// The crate's runtime: one worker thread, its I/O and timers, started the first time it is asked for.
    pub fn runtime() -> std::io::Result<&'static Runtime> {
        static RUNTIME: OnceLock<Result<Runtime, String>> = OnceLock::new();
        RUNTIME
            .get_or_init(|| Builder::new_multi_thread().worker_threads(1).thread_name("nettai-tasks").enable_all().build().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|why| std::io::Error::other(format!("the tasks' runtime: {why}")))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    /// A task runs to its end; one dropped while it waits is dropped there (what it held let go), never going on.
    #[test]
    fn a_task_runs_and_a_dropped_one_stops() {
        struct Held(Arc<AtomicBool>);
        impl Drop for Held {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let (done, let_go, went_on) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)));
        let d = done.clone();
        let _ran = spawn(async move { d.store(true, Ordering::SeqCst) }).unwrap();
        let (held, w) = (Held(let_go.clone()), went_on.clone());
        let waiting = spawn(async move {
            let _held = held;
            std::future::pending::<()>().await;
            w.store(true, Ordering::SeqCst);
        })
        .unwrap();
        waiting.cancel();
        let asked = std::time::Instant::now();
        while !(done.load(Ordering::SeqCst) && let_go.load(Ordering::SeqCst)) {
            assert!(asked.elapsed() < Duration::from_secs(5), "ran {done:?}, let go {let_go:?}");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!went_on.load(Ordering::SeqCst));
    }
}
