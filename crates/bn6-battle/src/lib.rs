//! The Mega Man Battle Network 6 battle engine, reimplemented in Rust.
//!
//! This is a from-scratch implementation of the game's battle rules with its
//! own data model. Game content (chip data, navi stats, animation timing)
//! lives in `data`, extracted from the original game by `bn6-extract`.
//! Behavior is verified tick by tick against traces recorded from the
//! original game (see `trace` and data/traces).

pub mod data;
pub mod rng;
#[cfg(feature = "trace")]
pub mod trace;

pub use rng::Rng;
