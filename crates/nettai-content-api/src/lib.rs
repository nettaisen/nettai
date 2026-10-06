//! The content API: the contract between the battle engine's core and the
//! game content built on it (chips, navi actions, weapons, object kinds).
//!
//! The core owns every piece of battle state as typed data: the object
//! pools and their update order, actors, collision, sprites, the RNG. It
//! knows no particular chip. Content is a set of Luau modules in a content
//! pack; the modules make *definitions* (docs/design/content-model-v2.md):
//! records in registries ([`registry`]), each with a key that interns to a
//! dense handle when content loads. What content implements is found by
//! definition slot (see [`host`]):
//!
//! - *object kinds*: the schema of a kind's own state ([`state::Schema`]:
//!   named, typed fields the core stores next to the object) and its update;
//! - *navi actions*: an action's state and update;
//! - *hooks*: the other function slots the ruleset calls (a weapon's setup,
//!   a dimming chip's `dimming`, a navi chip's `navi`, an instant chip's
//!   `instant`).
//!
//! Content functions are stateless: everything they keep between ticks
//! lives in engine-owned state, reached through [`CoreApi`]. That is what
//! makes a battle snapshot a plain `Clone` of the engine state (see
//! docs/design/scripting.md).
//!
//! A runtime (Luau) implements [`ContentHost`]; the engine implements
//! [`CoreApi`] and plans what the runtime binds ([`BindPlan`]) from the
//! definitions the runtime read back as [`Data`].

pub mod api;
pub mod assets;
pub mod data;
pub mod definitions;
pub mod host;
pub mod keys;
pub mod packs;
pub mod registry;
pub mod state;
pub mod types;

pub use api::{
    ACTOR_TYPES, ActorField, ApiError, ApiResult, BattleInfo, BlinkOut, CollisionField, ColumnInfo, CoreApi,
    DimmingStep, Emotion, HitboxSpec, HudPart, Key, Lifecycle, LinkedChip, NaviStat, NaviState, ObjectField, NaviAction, SpawnAt,
    OVERLAY_STEPPINGS,
    ObstacleAction, ObstacleCrush, ObstacleRemoval, ObstacleRequest, PANEL_TYPES, Pad, PanelInfo, RequestFlag, ScreenFade,
    Shadow, SpriteField, StatusFlag, StatusTimer,
    SideSpecial,
};
pub use api::AfterimageSpec;
pub use api::{AutoBattleEntry, AutoBattlePatternRead};
// Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
pub use api::{ObstacleHold, ObstaclePush, WindSource};
pub use assets::{AssetKind, AssetNames, PackIndex};
pub use data::{Data, Key as DataKey};
pub use definitions::{Definition, Definitions, GAME_LISTS};
pub use packs::{PackKind, PackManifest};
pub use host::{
    BindPlan, ContentError, ContentHost, DimmingChipSpec, FnId, FnSource, HookCall,
    InstantChipSpec, Manifest, NaviChipSpec, PlaceSpec, RulesHook,
};
pub use registry::{
    ActionHandle, ChipHandle, CollisionHandle, EffectHandle, FormHandle, IdentityHandle, KindHandle, NaviHandle,
    NaviCustProgramHandle, PatchCardHandle, RULESET_KEY, RecordHandle, RegionHandle, Registry, SparkHandle, StageHandle, StatusHandle, WeaponHandle,
    valid_key,
};
pub use state::{Block, FieldDef, FieldType, FieldValue, Fields, Schema, StateArena, StateId, StateMut, StateRef, Value};
pub use types::{InPack, ObjectRef, PackId, PackSprite, PanelPos, Pool, SpriteId, Vec3};
