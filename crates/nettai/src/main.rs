//! The nettai app's program (the app is the library: `nettai::run`).

// A release build on Windows opens no console window beside its own (what it prints goes nowhere; the log is the
// data folder's nettai.log). A development build keeps one, for what it prints.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// The app, under tokio: its futures run on the window's event loop (`slint::spawn_local`), and tokio's (reqwest's)
/// need its context, which this thread has inside the runtime. Slint's loop runs in `block_in_place`, as Slint's
/// docs ask of `#[tokio::main]`: this thread's share of tokio's work is handed to another, and the worker thread
/// drives the I/O.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> Result<(), slint::PlatformError> {
    tokio::task::block_in_place(nettai::run)
}

/// (A browser's page runs the app through crates/nettai-web, without tokio: its fetch is the page's.)
#[cfg(target_arch = "wasm32")]
fn main() -> Result<(), slint::PlatformError> {
    nettai::run()
}
