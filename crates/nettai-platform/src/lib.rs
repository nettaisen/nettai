//! What the app asks of the system, the same calls on the desktop and in a browser (crates/nettai-web), each module
//! the desktop's own where it has one:
//!
//! - [`fs`]: files (`std::fs`; in the browser a tree in memory the page keeps);
//! - [`thread`]: threads and the locks they share (`std::thread`; in the browser Web Workers);
//! - [`time`]: the clock (std's; in the browser `performance.now()` and `Date.now()`);
//! - [`dialog`]: files and folders to open, and where to save one (the system's dialogs; in the browser its file
//!   input, its folder dialog and downloads), and files dropped on the page;
//! - [`clipboard`]: text copied.

pub mod clipboard;
pub mod dialog;
pub mod fs;
pub mod thread;

/// The clock, as `std::time` names it.
pub mod time {
    pub use web_time::{Instant, SystemTime, UNIX_EPOCH};
}
