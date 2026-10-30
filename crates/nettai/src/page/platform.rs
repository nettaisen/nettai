//! The page around the app, one module chosen here: a browser's, and nothing elsewhere (the desktop, a phone).

#[cfg(target_arch = "wasm32")]
pub(crate) mod web;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod none;

#[cfg(target_arch = "wasm32")]
pub(crate) use web as system;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use none as system;
