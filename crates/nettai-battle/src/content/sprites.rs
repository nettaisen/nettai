//! Sprites as the simulation sees them: ids, animation timing and the
//! effect registries.

use serde::{Deserialize, Serialize};

/// A sprite: its handle over the sprites the content defines (what a pack
/// calls it, a `PackSprite`, is the pack's: `PackAssets`; docs/design/
/// rules-in-luau.md §7.4). The asset types come along for the frontends.
pub use nettai_content_api::{AssetNames, InPack, PackAssets, PackId, PackIndex, PackSprite, SpriteAnimation, SpriteDef, SpriteId};

/// One animation frame's timing, as the sprite steps it: how long it
/// shows, and its flags (`object::sprite::FRAME_LAST` on an animation's last
/// frame, with `FRAME_LOOP` where the animation loops), which follow from
/// its place in its animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AnimFrame {
    /// Ticks the frame shows for.
    pub duration: u8,
    pub flags: u8,
}

/// Every sprite's animations and the parts content reads, as the content
/// defines them (`asset.sprite(name, animations)`: `SpriteDef`), by sprite
/// handle.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Animations {
    pub sprites: Vec<SpriteDef>,
}

impl Animations {
    /// Animation `anim`'s frame `frame` of `sprite`, if it has one.
    pub fn frame(&self, sprite: SpriteId, anim: u8, frame: u16) -> Option<AnimFrame> {
        use crate::object::sprite::{FRAME_LAST, FRAME_LOOP};
        let a = self.sprites.get(sprite.index())?.animations.get(anim as usize)?;
        let duration = *a.frames.get(frame as usize)?;
        let last = frame as usize + 1 == a.frames.len();
        let flags = match (last, a.looping) {
            (false, _) => 0,
            (true, false) => FRAME_LAST,
            (true, true) => FRAME_LAST | FRAME_LOOP,
        };
        Some(AnimFrame { duration, flags })
    }

    /// Part `n`'s offset in frame `frame` of animation `anim` (none where
    /// the sprite states no such part).
    pub fn part_offset(&self, sprite: SpriteId, anim: u8, frame: u16, n: u8) -> Option<(i8, i8)> {
        self.sprites.get(sprite.index())?.parts.get(&n)?.get(anim as usize)?.get(frame as usize).copied()
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
