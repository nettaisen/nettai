//! A file over HTTP(S) ([`get`]): reqwest's, async, the same call on both sides: hyper and
//! rustls (ring) on the desktop, the browser's `fetch` there. Await it where tokio's context is (on the desktop the
//! app's window's thread, the program running under tokio: [`crate::task`]); dropping it cancels the request.

/// GET `url`, at most `limit` bytes of it: the body, or why there is none (no connection, a status other than a
/// success, a body over `limit`).
pub async fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let said = |e: reqwest::Error| format!("{url}: {e}");
    let response = client()?.get(url).send().await.map_err(said)?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{url}: {status}"));
    }
    if response.content_length().is_some_and(|n| n > limit) {
        return Err(format!("{url}: over {limit} bytes"));
    }
    let body = response.bytes().await.map_err(said)?;
    if body.len() as u64 > limit {
        return Err(format!("{url}: over {limit} bytes"));
    }
    Ok(body.to_vec())
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
