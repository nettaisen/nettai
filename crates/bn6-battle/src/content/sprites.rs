//! Sprites as the simulation sees them: ids, animation timing and the
//! effect registries.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A sprite: (category byte offset, index) into the game's sprite table.
/// In a content file, `"CC-II"` in hex (the sprite's `sprite.json` holds
/// it; its folder in a pack's `graphics/sprites` is its name).
pub use bn6_content_api::SpriteId;

/// One animation frame's timing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AnimFrame {
    /// Ticks the frame shows for.
    pub duration: u8,
    /// Frame cue bits: `object::sprite::FRAME_LAST` (0x80) ends the
    /// animation, `FRAME_LOOP` (0x40) loops it; attacks read others as cues.
    pub flags: u8,
}

/// Every sprite's animations (from the pack's `animations.json` files),
/// each a list of frames.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Animations {
    pub sprites: BTreeMap<SpriteId, Vec<Vec<AnimFrame>>>,
}

impl Animations {
    /// An animation's frames (empty when the sprite or animation has no
    /// data).
    pub fn get(&self, sprite: SpriteId, anim: u8) -> &[AnimFrame] {
        self.sprites.get(&sprite).and_then(|a| a.get(anim as usize)).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// A one-shot effect's look: which sprite animation it plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectSprite {
    pub sprite: SpriteId,
    pub anim: u8,
    pub palette: u8,
}
