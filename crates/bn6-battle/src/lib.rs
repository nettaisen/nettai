//! The Mega Man Battle Network 6 battle engine, reimplemented in Rust.
//!
//! This is a from-scratch implementation of the game's battle rules with its
//! own data model. Game content (chips, navis, the ruleset's tables,
//! animation timing) comes in a [`Content`], which a loader outside the
//! engine reads from a content pack (bn6-content); a battle shares it
//! read-only. Behavior is verified tick by tick against traces recorded
//! from the original game (bn6-compat's `trace`).

pub mod actor;
pub mod battle;
pub mod collision;
pub mod behavior;
pub mod console;
pub mod content;
pub mod cues;
pub mod custom;
pub mod digest;
pub mod field;
pub mod hand;
pub mod hud;
pub mod input;
pub mod kinds;
pub mod link;
pub mod object;
pub mod perspective;
pub mod rng;
pub mod rollback;
#[cfg(any(test, feature = "test-content"))]
pub mod scenario;
pub mod setup;
pub mod sound;
pub mod dimming;
pub mod transform;

pub use battle::{Battle, BattleResult, CustomResult, RoundEnd, TickEvents};
pub use content::{Content, ContentHash};
pub use input::PlayerTick;
pub use rng::Rng;
pub use rollback::{Snapshot, TickInput};
pub use setup::{BattleSettings, NaviStats, RoundSetup, SetScore, Stage};
pub use sound::{SoundCue, SoundId};
