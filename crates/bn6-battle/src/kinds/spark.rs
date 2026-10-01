//! Hit sparks (effect object #4), spawned where an attack connects.

use crate::battle::Battle;
use crate::content::EffectSprite;
use crate::object::{ObjectRef, Vec3, flags, sprite::FRAME_LAST, state};
use bn6_content_api::SparkHandle;

/// `AddRandomVarianceToTwoCoords`: jitter x and z by up to ±mask/2 pixels
/// (one simulation RNG draw).
pub fn jitter(b: &mut Battle, mask: u32, mut pos: Vec3) -> Vec3 {
    let r = b.rng.next();
    pos.x = pos.x.wrapping_add((((r & mask) as i32) - (mask >> 1) as i32) << 16);
    pos.z = pos.z.wrapping_add(((((r >> 16) & mask) as i32) - (mask >> 1) as i32) << 16);
    pos
}

/// `object_spawnCollisionEffect`: a hitter that touched a body or object,
/// unguarded, shows its hit effect.
pub fn spawn_collision_effect(b: &mut Battle, hitter: ObjectRef) {
    let Some(c) = b.objects.get(hitter).collision else { return };
    let s = b.collision.get(c);
    let flags = s.acc.hit_flags;
    if flags & 0x3F80_0000 == 0 || flags & 1 != 0 {
        return;
    }
    let Some(effect) = s.hit_effect else { return };
    let pos = jitter(b, 0xF, b.objects.get(hitter).pos);
    spawn(b, hitter, pos, effect);
}

/// Spark-private state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// What it shows (the original's first spawn parameter, a row of its
    /// table of looks).
    pub look: Option<SparkHandle>,
}

/// `sub_80E08C4`.
pub fn spawn(b: &mut Battle, owner: ObjectRef, pos: Vec3, look: SparkHandle) -> Option<ObjectRef> {
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::Spark, pos, [0; 4])?;
    let alliance = b.objects.get(owner).alliance;
    let o = b.objects.get_mut(r);
    o.vars = crate::kinds::Vars::Spark(Vars { look: Some(look) });
    o.related[0] = Some(owner);
    o.alliance = alliance;
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => {
            let look = match &b.objects.get(r).vars {
                crate::kinds::Vars::Spark(v) => v.look,
                _ => None,
            };
            let EffectSprite { sprite: id, anim, palette } = b.content.spark(look.expect("a hit spark spawned with its look"));
            let sprite = b.objects.sprite_mut(r);
            sprite.load(id);
            sprite.set_animation(anim, &b.content);
            sprite.look.shadow = crate::object::sprite::Shadow::WithSprite;
            sprite.look.palette = palette;
            sprite.update(&b.content);
            let o = b.objects.get_mut(r);
            o.flags &= !flags::NO_SPRITE_UPDATE;
            o.anim = anim;
            o.anim_loaded = anim;
            o.flags |= flags::VISIBLE;
            o.state = state::UPDATE;
            o.action = 0;
            o.phase = 0;
            o.phase_init = 0;
        }
        state::UPDATE => {
            if b.objects.sprite(r).frame_parameters() & FRAME_LAST != 0 {
                let o = b.objects.get_mut(r);
                o.flags &= !flags::VISIBLE;
                o.state = state::DESTROY;
                o.action = 0;
                o.phase = 0;
                o.phase_init = 0;
            }
            b.objects.sprite_mut(r).update(&b.content);
        }
        _ => b.objects.free(r),
    }
}
