//! Sprites as the simulation sees them: ids, animation timing and the
//! effect registries.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A sprite: its handle over the loaded packs' sprites (what its pack
/// calls it, a `PackSprite`, is the asset names'; docs/design/
/// rules-in-luau.md §7.4). The asset types come along for the frontends.
pub use nettai_content_api::{AssetNames, InPack, PackId, PackSprite, SpriteId};

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
/// each a list of frames; and where each frame's parts sit.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Animations {
    pub sprites: BTreeMap<SpriteId, Vec<Vec<AnimFrame>>>,
    pub parts: BTreeMap<SpriteId, SpriteParts>,
}

impl Animations {
    /// Add pack `pack`'s sprites' timing and parts (by the pack's own ids),
    /// keyed by the content's handles: each name the asset names give a
    /// sprite of the pack is a handle of it.
    pub fn add_pack(
        &mut self,
        assets: &AssetNames,
        pack: PackId,
        sprites: &BTreeMap<PackSprite, Vec<Vec<AnimFrame>>>,
        parts: &BTreeMap<PackSprite, SpriteParts>,
    ) {
        for (h, a) in assets.sprites.values().enumerate() {
            if a.pack != pack {
                continue;
            }
            let id = SpriteId(h as u16);
            if let Some(anims) = sprites.get(&a.id) {
                self.sprites.insert(id, anims.clone());
            }
            if let Some(p) = parts.get(&a.id) {
                self.parts.insert(id, p.clone());
            }
        }
    }
}

/// A sprite's frames' parts, as far as the simulation reads them: each
/// frame's layout (by animation, then frame) and each layout's parts'
/// offsets from the object, in pixels, in the order the frame lists them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpriteParts {
    pub frame_layouts: Vec<Vec<u16>>,
    pub layouts: Vec<Vec<(i8, i8)>>,
}

impl Animations {
    /// An animation's frames (empty when the sprite or animation has no
    /// data).
    pub fn get(&self, sprite: SpriteId, anim: u8) -> &[AnimFrame] {
        self.sprites.get(&sprite).and_then(|a| a.get(anim as usize)).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Part `n`'s offset in frame `frame` of animation `anim` (none when
    /// the frame has fewer parts, or no data).
    pub fn part_offset(&self, sprite: SpriteId, anim: u8, frame: u16, n: usize) -> Option<(i8, i8)> {
        let parts = self.parts.get(&sprite)?;
        let layout = *parts.frame_layouts.get(anim as usize)?.get(frame as usize)?;
        parts.layouts.get(layout as usize)?.get(n).copied()
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
