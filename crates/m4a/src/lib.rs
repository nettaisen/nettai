//! The M4A ("Sappy") sound driver of GBA games, in Rust.
//!
//! - [`bank`]: what the driver plays, as typed data: mixer settings, music
//!   players, songs (track commands) and instruments.
//! - [`rom`]: reading that data out of a ROM image, once.
//! - [`driver`]: the driver itself, one frame at a time: the music players
//!   and their sequencers, hardware channel allocation between them, the
//!   PSG envelopes and the Direct Sound mix.
//! - [`apu`]: the GBA's sound hardware the driver drives: the PSG channels
//!   and the DAC.
//!
//! The driver follows the GBA library's code (`MPlayMain`, `ply_note`,
//! `TrkVolPitSet`, `CgbSound`, `SoundMainRAM` in m4a_1.s and m4a.c) in
//! its integer arithmetic, so that its PCM buffer and PSG registers hold
//! what the game's do. It began as a port of the `m4a-core` and
//! `m4a-engine` crates of bnmusic's arranger (MIT license).

pub mod apu;
pub mod bank;
pub mod driver;
mod mixer;
pub mod rom;
pub mod tables;

pub use bank::{PlayerId, SongId, SoundBank};
pub use driver::Driver;
pub use mixer::{Phase, Status};

/// GBA frames per second; the driver runs once per frame.
pub const FPS: f64 = 59.7275;
/// The rate [`Driver::take_output`] gives, Hz.
pub const OUT_RATE: u32 = 32768;
