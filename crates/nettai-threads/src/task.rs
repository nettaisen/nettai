//! Work for the app's futures, which run on the window's event loop (`slint::spawn_local`: their results land on the
//! window's thread, and its `JoinHandle::abort` cancels one). On the desktop the program runs under tokio (nettai's
//! main.rs: `#[tokio::main]`, Slint's loop in `block_in_place`), so those futures can use tokio's (reqwest's:
//! nettai-http); in the browser there is no tokio, the page's fetch instead.
//!
//! [`off_thread`]: work too long for the window's thread (reading a file, hashing it), done on a thread of its own
//! ([`crate::spawn`]: a Web Worker in the browser) and awaited.

/// `work` done on a thread of its own, called `name`, awaited: what it made, or why it couldn't be done (no thread
/// started, or it ended without an answer).
pub async fn off_thread<T: Send + 'static>(name: &str, work: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    let (answer, answered) = futures_channel::oneshot::channel();
    crate::spawn(name, move || drop(answer.send(work()))).map_err(|e| format!("a thread for {name}: {e}"))?;
    answered.await.map_err(|_| format!("{name}: its thread ended without an answer"))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// Work done off the thread comes back to the future that awaits it.
    #[test]
    fn work_comes_back_from_a_thread() {
        assert_eq!(futures_executor::block_on(off_thread("adding", || 2 + 2)), Ok(4));
    }
}
