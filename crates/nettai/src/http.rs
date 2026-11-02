//! A file over HTTP(S) ([`get`], [`get_reporting`]): the games' content (`contents.rs`) and the dummy's network
//! (`screens/practice/weights.rs`). reqwest's, async, the same call everywhere: hyper and rustls (ring) on the
//! desktop and a phone, the browser's `fetch` there. Await it where tokio's context is (on the desktop
//! the app's window's thread, the program running under tokio: nettai-threads' `off_thread`); dropping it cancels
//! the request.
//!
//! Natively a request gives up on a server that doesn't answer: [`CONNECT_WAIT`] to connect, [`READ_WAIT`] with
//! nothing heard. Without them a connection whose packets go nowhere waits out the system's own retries (two
//! minutes a request on Android), and the loading screen with it. (The browser's `fetch` has its own.)

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

/// How long a connection may take to be made (the name looked up, TCP, TLS), and a response may go without a byte,
/// before the request fails.
#[cfg(not(target_arch = "wasm32"))]
pub const CONNECT_WAIT: Duration = Duration::from_secs(10);
#[cfg(not(target_arch = "wasm32"))]
pub const READ_WAIT: Duration = Duration::from_secs(30);

/// GET `url`, at most `limit` bytes of it: the body, or why there is none (no connection, a status other than a
/// success, a body over `limit`).
pub async fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    get_reporting(url, limit, |_| {}).await
}

/// [`get`], telling `progress` how many bytes of the body have come as they come (a download's bar).
pub async fn get_reporting(url: &str, limit: u64, progress: impl FnMut(u64)) -> Result<Vec<u8>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let client = client(CONNECT_WAIT, READ_WAIT)?;
    #[cfg(target_arch = "wasm32")]
    let client = client()?;
    get_with(&client, url, limit, progress).await
}

/// [`get_reporting`] by `client`.
async fn get_with(client: &reqwest::Client, url: &str, limit: u64, mut progress: impl FnMut(u64)) -> Result<Vec<u8>, String> {
    use futures_util::StreamExt;
    let said = |e: reqwest::Error| format!("{url}: {e}");
    let response = client.get(url).send().await.map_err(said)?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{url}: {status}"));
    }
    if response.content_length().is_some_and(|n| n > limit) {
        return Err(format!("{url}: over {limit} bytes"));
    }
    let mut body = Vec::new();
    let mut chunks = response.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        body.extend_from_slice(&chunk.map_err(said)?);
        if body.len() as u64 > limit {
            return Err(format!("{url}: over {limit} bytes"));
        }
        progress(body.len() as u64);
    }
    Ok(body)
}

/// A client giving up on a server after `connect` to connect and `read` with nothing heard ([`CONNECT_WAIT`] and
/// [`READ_WAIT`]), with rustls's ring provider, installed as the process's the first time (reqwest has none of its
/// own here: the tree's rustls is ring's, nettai-rtc's too).
#[cfg(not(target_arch = "wasm32"))]
fn client(connect: Duration, read: Duration) -> Result<reqwest::Client, String> {
    static PROVIDER: std::sync::Once = std::sync::Once::new();
    // (Err: one is installed already, which serves as well.)
    PROVIDER.call_once(|| drop(rustls::crypto::ring::default_provider().install_default()));
    let builder = reqwest::Client::builder().connect_timeout(connect).read_timeout(read);
    builder.build().map_err(|e| format!("an HTTP client: {e}"))
}

/// The browser's client: its `fetch`.
#[cfg(target_arch = "wasm32")]
fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().build().map_err(|e| format!("an HTTP client: {e}"))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// A server that takes the connection and never answers is given up on once the read wait has passed with
    /// nothing heard (here 200 ms).
    #[test]
    fn a_server_that_never_answers_is_given_up_on() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/catalog.json", listener.local_addr().unwrap());
        // (Taken, held open and silent.)
        std::thread::spawn(move || {
            let held: Vec<_> = listener.incoming().take(1).collect();
            std::thread::sleep(Duration::from_secs(5));
            drop(held);
        });
        let client = client(CONNECT_WAIT, Duration::from_millis(200)).unwrap();
        let started = std::time::Instant::now();
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let failed = runtime.block_on(get_with(&client, &url, 1 << 20, |_| {}));
        assert!(failed.is_err(), "{failed:?}");
        assert!(started.elapsed() < Duration::from_secs(4), "it waited {:?}", started.elapsed());
    }
}
