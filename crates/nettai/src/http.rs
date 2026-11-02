//! A file over HTTP(S) ([`get`], [`get_reporting`], [`get_each`]): the games' content (`contents.rs`), the dummy's
//! network (`screens/practice/weights.rs`) and Android's update (`updates/android.rs`). reqwest's, async, the same
//! call everywhere: hyper and rustls (ring) on the desktop and a phone, the browser's `fetch` there. Await it where
//! tokio's context is (on the desktop the app's window's thread, the program running under tokio: nettai-threads'
//! `off_thread`); dropping it cancels the request.

/// GET `url`, at most `limit` bytes of it: the body, or why there is none (no connection, a status other than a
/// success, a body over `limit`).
pub async fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    get_reporting(url, limit, |_| {}).await
}

/// [`get`], telling `progress` how many bytes of the body have come as they come (a download's bar).
pub async fn get_reporting(url: &str, limit: u64, mut progress: impl FnMut(u64)) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    get_each(url, limit, |piece, got| {
        body.extend_from_slice(piece);
        progress(got);
        Ok(())
    })
    .await?;
    Ok(body)
}

/// GET `url`, at most `limit` bytes of it, each piece of the body handed to `take` as it comes, with how many bytes
/// have come so far (a body written to a file as it comes: Android's update, `updates/android.rs`); what `take` says
/// went wrong ends it.
pub async fn get_each(url: &str, limit: u64, mut take: impl FnMut(&[u8], u64) -> Result<(), String>) -> Result<(), String> {
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
    let mut got = 0;
    let mut pieces = response.bytes_stream();
    while let Some(piece) = pieces.next().await {
        let piece = piece.map_err(said)?;
        got += piece.len() as u64;
        if got > limit {
            return Err(format!("{url}: over {limit} bytes"));
        }
        take(&piece, got)?;
    }
    Ok(())
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
    let client = reqwest::Client::builder();
    // (Natively the app says who it is, which GitHub's API answers no request without: Android's update check. A
    // browser says its own.)
    #[cfg(not(target_arch = "wasm32"))]
    let client = client.user_agent(concat!("nettai/", env!("CARGO_PKG_VERSION")));
    client.build().map_err(|e| format!("an HTTP client: {e}"))
}
