//! nettai's desktop program: a host of the nettai-frontend library. It is
//! what is around the playing: the window, its keys and the match editor in
//! it ([`window`], [`editor`]: iced), the command line (`main.rs`),
//! headless output ([`headless`]), the audits
//! ([`content_audit`], [`headless::audit_traces`], and the audio's lookups,
//! [`sound_lookups`]), the replay of the original's recordings ([`trace`],
//! with the compat crates), and netplay's transport ([`net`]: a WebRTC
//! link, nettai-rtc's, and the handshake). The sound goes to the audio device
//! (nettai-audio's `AudioOut`). docs/frontend.md.

// (Whether the picture's wgpu state may go to another thread is deeper than
// the compiler looks by default: `picture`.)
#![recursion_limit = "256"]

pub mod content_audit;
pub mod editor;
pub mod headless;
pub mod net;
mod picture;
pub mod save_import;
pub mod sound_lookups;
pub mod trace;
pub mod window;
