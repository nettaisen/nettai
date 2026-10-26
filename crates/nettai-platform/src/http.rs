//! A file over HTTP(S): [`get`]. On the desktop ureq's (with rustls: the same crate the app's updates fetch with), on
//! a thread of its own; in the browser its `fetch`, on the page's event loop. Nothing waits: the body, or why there
//! is none, goes to a callback.

/// GET `url`, at most `limit` bytes of it: `done` gets the body, or why there is none (no connection, a status
/// other than 200, a body over `limit`), on another thread on the desktop and on the page's event loop in the
/// browser. Err: the desktop's thread couldn't start (`done` dropped unasked).
pub fn get(url: &str, limit: u64, done: impl FnOnce(Result<Vec<u8>, String>) + Send + 'static) -> std::io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let url = url.to_string();
        crate::thread::spawn("http-get", move || done(desktop::get(&url, limit)))
    }
    #[cfg(target_arch = "wasm32")]
    {
        web::get(url, limit, Box::new(done));
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    pub fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
        let mut response = ureq::get(url).call().map_err(|e| format!("{url}: {e}"))?;
        // (ureq's own limit, 10 MB, is under a network's weights: the caller's instead.)
        response.body_mut().with_config().limit(limit).read_to_vec().map_err(|e| format!("{url}: {e}"))
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(inline_js = r#"
export async function nettai_get(url, limit) {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`${url}: ${response.status} ${response.statusText}`);
    const body = new Uint8Array(await response.arrayBuffer());
    if (body.length > limit) throw new Error(`${url}: ${body.length} bytes, over ${limit}`);
    return body;
}
"#)]
    extern "C" {
        fn nettai_get(url: &str, limit: f64) -> js_sys::Promise;
    }

    pub fn get(url: &str, limit: u64, done: Box<dyn FnOnce(Result<Vec<u8>, String>)>) {
        let asked = nettai_get(url, limit as f64);
        wasm_bindgen_futures::spawn_local(async move {
            done(match wasm_bindgen_futures::JsFuture::from(asked).await {
                Ok(body) => Ok(js_sys::Uint8Array::new(&body).to_vec()),
                Err(why) => Err(why.as_string().or_else(|| js_sys::Error::from(why).message().as_string()).unwrap_or_else(|| "fetch failed".into())),
            });
        });
    }
}
