//! The Mega Man Battle Network 6 battle engine, reimplemented in Rust.
//!
//! This is a from-scratch implementation of the game's battle rules with its
//! own data model. Game content (chip data, navi stats, animation timing)
//! lives in `data`, extracted from the original game by `bn6-extract`.
//! Behavior is verified tick by tick against traces recorded from the
//! original game (see `trace`).

pub mod actor;
pub mod battle;
pub mod collision;
pub mod data;
pub mod field;
pub mod hand;
pub mod hud;
pub mod input;
pub mod kinds;
pub mod object;
pub mod rng;
pub mod setup;
pub mod sound;
pub mod transform;
#[cfg(feature = "trace")]
pub mod trace;

pub use battle::{Battle, BattleResult, CustomResult, RoundEnd, TickEvents};
pub use input::PlayerTick;
pub use rng::Rng;
pub use setup::{BattleSettings, NaviStats, RoundSetup, SetScore, Stage};
pub use sound::{SoundCue, SoundId};
