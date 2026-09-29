//! The M4A ("Sappy") sound driver of GBA games, in Rust.
//!
//! - [`bank`]: what the driver plays, as typed data: mixer settings, music
//!   players, songs (track commands) and instruments.
//! - [`rom`]: reading that data out of a ROM image, once.
//! - [`driver`]: the driver itself, one frame at a time: the music players
//!   and their sequencers, hardware channel allocation between them, and
//!   the Direct Sound and PSG mixer.
//!
//! Ported from the `m4a-core` and `m4a-engine` crates of bnmusic's
//! arranger (MIT license), which port bnmusic's `m4a.py`. The mixer math
//! (envelopes, sample resampling, PSG synthesis, reverb, output levels)
//! comes from there. The sequencer and music players are rewritten after
//! the GBA driver (`MPlayMain`, `ply_note`, `MPlayStart` in m4a_1.s) so that
//! several songs play at once and compete for channels as they do in a game.

pub mod bank;
pub mod driver;
mod mixer;
pub mod rom;

pub use bank::{PlayerId, SongId, SoundBank};
pub use driver::Driver;

/// GBA frames per second; the driver runs once per frame.
pub const FPS: f64 = 59.7275;
/// The GBA's sound output rate, Hz.
pub const OUT_RATE: u32 = 32768;
