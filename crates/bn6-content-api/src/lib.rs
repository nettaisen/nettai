//! The content API: the contract between the battle engine's core and the
//! game content built on it (chips, navi actions, weapons, object kinds).
//!
//! The core owns every piece of battle state as typed data: the object
//! pools and their update order, actors, collision, sprites, the RNG. It
//! knows no particular chip. Content is a set of scripts in a content pack,
//! and the pack's data says what each implements (see [`host`]):
//!
//! - *object kinds*: which object slot (pool and index) a module
//!   implements, the schema of its own state ([`state::Schema`]: named,
//!   typed fields the core stores next to the object) and its update;
//! - *navi actions*: an action number, its state and update;
//! - *hooks*: ruleset tables filled by number (weapon routines, dimming
//!   chips' dimming controllers, navi chips' navis).
//!
//! Content functions are stateless: everything they keep between ticks
//! lives in engine-owned state, reached through [`CoreApi`]. That is what
//! makes a battle snapshot a plain `Clone` of the engine state (see
//! docs/design/scripting.md).
//!
//! A runtime (Luau) implements [`ContentHost`]; the engine implements
//! [`CoreApi`] and builds the [`Registrations`] and the scripts' [`Data`]
//! from the pack.

pub mod api;
pub mod data;
pub mod host;
pub mod state;
pub mod types;

pub use api::{
    ACTOR_TYPES, ActorField, ApiError, ApiResult, BattleInfo, BlinkOut, CollisionField, ColumnInfo, CoreApi,
    DimmingStep, Emotion, HitboxSpec, Key, Lifecycle, LinkedChip, NaviRecordInfo, NaviStat, NaviState, ObjectField,
    ObstacleAction, ObstacleCrush, ObstacleRemoval, ObstacleRequest, PANEL_TYPES, Pad, PanelInfo, RequestFlag, Shadow,
    SpriteField, StatusFlag, StatusTimer,
};
pub use api::AfterimageSpec;
pub use data::{Data, Key as DataKey};
pub use host::{
    ActionDef, ActionReg, ActorListEntrySpec, ContentError, ContentHost, DimmingChipSpec, Hook, HookCall, HookDef,
    HookId, HookReg, KindId, KindReg, Manifest, NaviChipSpec, ObjectKindDef, Registrations,
};
pub use state::{ContentState, FieldDef, FieldType, FieldValue, MAX_BYTES, Schema, StateId, Value};
pub use types::{ObjectRef, PanelPos, Pool, SpriteId, Vec3};
