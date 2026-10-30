//! What the app asks of the system, the same calls on the desktop, in a browser (crates/nettai-web) and on a phone
//! (iOS, Android), each module the desktop's own where it has one:
//!
//! - [`fs`]: files (`std::fs`; in the browser a tree in memory the page keeps);
//! - [`thread`]: threads and the locks they share (`std::thread`; in the browser Web Workers);
//! - [`time`]: the clock (std's; in the browser `performance.now()` and `Date.now()`);
//! - [`dialog`]: files and folders to open, and where to save one (the system's dialogs; in the browser its file
//!   input, its folder dialog and downloads), and files dropped on the page;
//! - [`clipboard`]: text copied;
//! - [`page`]: the page the app is in (a browser's), whose size its window follows;
//! - [`text_entry`]: a field's text typed (in the browser an HTML input laid over it, for its input method);
//! - [`task`]: work done off the window's thread for the app's futures (run on its event loop, under tokio);
//! - [`http`]: a file fetched over HTTP(S), async (reqwest's; in the browser its `fetch`);
//! - [`folders`]: the system's folders for the app's data and for files kept a while;
//! - [`awake`]: the app's time and the display kept on while a battle runs;
//! - [`lifecycle`]: a phone's app coming back (made active again, its sound given back);
//! - `android`: Android's activity, which the program's entry point hands over first ([`android::init`]).

#[cfg(target_os = "android")]
pub mod android;
pub mod awake;
pub mod clipboard;
pub mod dialog;
pub mod folders;
pub mod fs;
pub mod http;
pub mod lifecycle;
pub mod task;
pub mod page;
pub mod text_entry;
pub mod thread;

/// The clock, as `std::time` names it.
pub mod time {
    pub use web_time::{Instant, SystemTime, UNIX_EPOCH};
}
