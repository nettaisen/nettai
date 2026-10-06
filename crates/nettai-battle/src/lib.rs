//! nettai: a Battle Network battle engine in Rust.
//!
//! It reimplements Mega Man Battle Network 6's battle rules from scratch with
//! its own data model; EXE6 is the first game it plays (content/exe6), and the
//! aim is to bring the other Battle Network games over as content too. Game content (chips, navis, the rules' tables,
//! animation timing) comes in a [`Content`], which a loader outside the
//! engine reads from a content pack (nettai-content); a battle shares it
//! read-only. Behavior is verified tick by tick against traces recorded
//! from the original game (exe6-compat's `trace`).

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
pub mod given;
pub mod hand;
pub mod hud;
pub mod input;
pub mod kinds;
pub mod link;
pub mod object;
pub mod navicust;
pub mod patch_cards;
pub mod perspective;
pub mod rng;
pub mod rollback;
pub mod rules;
#[cfg(any(test, feature = "test-content"))]
pub mod scenario;
pub mod setup;
pub mod auto_battle;
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
