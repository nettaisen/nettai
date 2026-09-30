//! The content API: the contract between the battle engine's core and the
//! game content built on it (chips, navi actions, weapons, object kinds).
//!
//! The core owns every piece of battle state as typed data: the object
//! pools and their update order, actors, collision, sprites, the RNG. It
//! knows no particular chip. Content is a set of Luau modules in a content
//! pack; the modules make *definitions* (docs/design/content-model-v2.md):
//! records in registries ([`registry`]), each with a key that interns to a
//! dense handle when content loads. What content implements is found by
//! definition slot, or (until the model v2 migration ends) by module export
//! registered from the pack's data (see [`host`]):
//!
//! - *object kinds*: the schema of a kind's own state ([`state::Schema`]:
//!   named, typed fields the core stores next to the object) and its update;
//! - *navi actions*: an action's state and update;
//! - *hooks*: ruleset tables content fills (weapon routines, dimming chips'
//!   dimming controllers, navi chips' navis).
//!
//! Content functions are stateless: everything they keep between ticks
//! lives in engine-owned state, reached through [`CoreApi`]. That is what
//! makes a battle snapshot a plain `Clone` of the engine state (see
//! docs/design/scripting.md).
//!
//! A runtime (Luau) implements [`ContentHost`]; the engine implements
//! [`CoreApi`], plans what the runtime binds ([`BindPlan`]) and builds the
//! scripts' [`Data`] from the pack.

pub mod api;
pub mod data;
pub mod definitions;
pub mod host;
pub mod registry;
pub mod state;
pub mod types;

pub use api::{
    ACTOR_TYPES, ActorField, ApiError, ApiResult, BattleInfo, BlinkOut, CollisionField, ColumnInfo, CoreApi,
    DimmingStep, Emotion, HitboxSpec, Key, Lifecycle, LinkedChip, NaviRecordInfo, NaviStat, NaviState, ObjectField, SpawnAt,
    OVERLAY_STEPPINGS,
    ObstacleAction, ObstacleCrush, ObstacleRemoval, ObstacleRequest, PANEL_TYPES, Pad, PanelInfo, RequestFlag, Shadow,
    SpriteField, StatusFlag, StatusTimer,
    SideSpecial,
};
pub use api::AfterimageSpec;
// Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
pub use api::{ObstacleHold, ObstaclePush, WindSource};
pub use data::{Data, Key as DataKey};
pub use definitions::{Definition, Definitions, ModuleExports};
pub use host::{
    ActorListEntrySpec, BindPlan, ContentError, ContentHost, DimmingChipSpec, FnId, FnSource, Hook, HookCall,
    InstantChipSpec, Manifest, NaviChipSpec,
};
pub use registry::{
    ActionHandle, ChipHandle, CollisionHandle, EffectHandle, FormHandle, KindHandle, LockonHandle, NaviHandle,
    RecordHandle, RegionHandle, Registry, SparkHandle, StageHandle, StatusHandle, WeaponHandle, valid_key,
};
pub use state::{ContentState, FieldDef, FieldType, FieldValue, MAX_BYTES, Schema, StateId, Value};
pub use types::{ObjectRef, PanelPos, Pool, SpriteId, Vec3};
