//! nettai's desktop program: a host of the nettai-frontend library. It is
//! what is around the playing: the window and the keys ([`app`]), the
//! command line (`main.rs`), headless output ([`headless`]), the audits
//! ([`content_audit`], [`headless::audit_traces`], and the audio's lookups,
//! [`sound_lookups`]), the replay of the original's recordings ([`trace`],
//! with the compat crates), and netplay's transport ([`net`]: the UDP
//! socket and the handshake). The sound goes to the audio device
//! (nettai-audio's `AudioOut`). docs/frontend.md.

pub mod app;
pub mod content_audit;
pub mod headless;
pub mod net;
pub mod sound_lookups;
pub mod trace;
