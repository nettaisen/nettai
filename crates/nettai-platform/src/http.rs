//! A file over HTTP(S) ([`get`], [`get_reporting`]): reqwest's, async, the same call on both sides: hyper and
//! rustls (ring) on the desktop, the browser's `fetch` there. Await it where tokio's context is (on the desktop the
//! app's window's thread, the program running under tokio: [`crate::task`]); dropping it cancels the request.

/// GET `url`, at most `limit` bytes of it: the body, or why there is none (no connection, a status other than a
/// success, a body over `limit`).
pub async fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    get_reporting(url, limit, |_| {}).await
}

/// [`get`], telling `progress` how many bytes of the body have come as they come (a download's bar).
pub async fn get_reporting(url: &str, limit: u64, mut progress: impl FnMut(u64)) -> Result<Vec<u8>, String> {
    use futures_util::StreamExt;
    let said = |e: reqwest::Error| format!("{url}: {e}");
    let response = client()?.get(url).send().await.map_err(said)?;
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

/// A client: on the desktop with rustls's ring provider, installed as the process's the first time (reqwest has
/// none of its own here: the tree's rustls is ring's, nettai-rtc's too).
fn client() -> Result<reqwest::Client, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static PROVIDER: std::sync::Once = std::sync::Once::new();
        // (Err: one is installed already, which serves as well.)
        PROVIDER.call_once(|| drop(rustls::crypto::ring::default_provider().install_default()));
    }
    reqwest::Client::builder().build().map_err(|e| format!("an HTTP client: {e}"))
}
