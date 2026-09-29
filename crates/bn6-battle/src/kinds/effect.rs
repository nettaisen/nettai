//! Generic one-shot effects (effect object #0: explosions, sparkles...).
//! Visual, but they hold pool slots and list positions, and without a
//! timer they live exactly as long as their animation.
//! See docs/engine/objects-and-player.md §A.3.

use crate::battle::Battle;
use crate::data::{SpriteId, effects_generated::EFFECTS};
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

/// Effect-private state.
#[derive(Clone, Debug, Default)]
pub struct Vars {
    /// Visibility follows `related[0]`.
    pub follow_related: bool,
}

/// `SpawnT4BattleObjectWithId0`: effect `id` at `pos`.
pub fn spawn(b: &mut Battle, pos: Vec3, id: u8, flip: u8, palette_add: u8, priority: u8) -> Option<ObjectRef> {
    b.objects.spawn(Pool::Effect, 0, pos, [id, flip, palette_add, priority])
}

/// `sub_80E060E`: make an effect's visibility follow `owner`.
pub fn follow(b: &mut Battle, r: ObjectRef, owner: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    if let crate::kinds::Vars::Effect(v) = &mut o.vars {
        v.follow_related = true;
    }
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

fn init(b: &mut Battle, r: ObjectRef) {
    let id = b.objects.get(r).params[0];
    let (category, index, anim, _palette) = EFFECTS[id as usize];
    let sprite = b.objects.sprite_mut(r);
    sprite.load(SpriteId { category, index });
    sprite.set_animation(anim);
    sprite.update();
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = anim;
    o.anim_loaded = anim;
    o.flags |= flags::VISIBLE;
    o.state = state::UPDATE;
}

fn tick(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.timer = o.timer.wrapping_sub(1);
    let mut destroy = o.timer == 0;
    if !destroy {
        // Visibility follows the related object.
        let follow = matches!(&o.vars, crate::kinds::Vars::Effect(v) if v.follow_related);
        if let (true, Some(owner)) = (follow, o.related[0]) {
            let visible = b.objects.get(owner).flags & flags::VISIBLE;
            let o = b.objects.get_mut(r);
            o.flags = (o.flags & !flags::VISIBLE) | visible;
        }
        let finished = b.objects.sprite(r).frame_parameters() & crate::object::sprite::FRAME_LAST != 0;
        destroy = finished && (b.objects.get(r).timer as i16) <= 0;
    }
    b.objects.sprite_mut(r).update();
    if destroy {
        let o = b.objects.get_mut(r);
        o.flags &= !flags::VISIBLE;
        o.state = state::DESTROY;
    }
}
