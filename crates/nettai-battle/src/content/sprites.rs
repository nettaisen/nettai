//! Sprites as the simulation sees them: ids and the effect registries. (A
//! sprite's timing is its sheet's: the battle counts its steps.)

use serde::{Deserialize, Serialize};

/// A sprite: its handle over the sprites the content names (what the
/// game's graphics call it, a `SpriteNumber`, is theirs: `AssetNumbers`; docs/design/
/// rules-in-luau.md §7.4). The asset types come along for the frontends.
pub use nettai_content_api::{AssetIndex, AssetNames, AssetNumbers, SpriteId, SpriteNumber};

/// A one-shot effect's look: which sprite animation it plays, and for how
/// many of its sprite's steps it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectSprite {
    pub sprite: SpriteId,
    pub anim: u8,
    pub palette: u8,
    pub steps: u16,
}
