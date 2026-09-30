//! The content API: the contract between the battle engine's core and the
//! game content built on it (chips, attacks, navi actions, effects).
//!
//! The core owns every piece of battle state as typed data: the object
//! pools and their update order, actors, collision, sprites, the RNG. It
//! knows no particular chip. Content is a set of *object kinds* and
//! *actions*, each declaring
//!
//! - which object slot or action number it implements,
//! - the schema of its own state ([`state::Schema`]: named, typed fields
//!   the core stores next to the object), and
//! - an update function, run by the core once per tick.
//!
//! Update functions are stateless: everything they keep between ticks
//! lives in engine-owned state, reached through [`CoreApi`]. That is what
//! makes a battle snapshot a plain `Clone` of the engine state, whatever
//! language the content is written in (see docs/design/scripting.md).
//!
//! A runtime (native Rust, Luau, ...) implements [`ContentHost`]; the engine
//! implements [`CoreApi`].

pub mod api;
pub mod host;
pub mod state;
pub mod types;

pub use api::{
    ActorField, ApiError, CollisionField, CoreApi, Lifecycle, NaviStat, ObjectField, Shadow, SpriteField, StatusFlag,
};
pub use host::{ActionDef, ContentError, ContentHost, KindId, Manifest, ObjectKindDef};
pub use state::{ContentState, FieldDef, FieldType, FieldValue, Schema, StateId, Value};
pub use types::{ObjectRef, PanelPos, Pool, SpriteId, Vec3};
