//! Generic one-shot effects (effect object #0: explosions, sparkles...).
//! Visual, but they hold pool slots and list positions, and without a
//! timer they live exactly as long as their animation.
//! See docs/engine/objects-and-player.md §A.3.

use crate::battle::Battle;
use crate::data::{EffectSprite, effects_generated::EFFECTS};
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

/// Effect-private state.
#[derive(Clone, Debug, Default)]
pub struct Vars {
    /// Visibility follows `related[0]`.
    pub follow_related: bool,
    /// The game spawned it with X and Y left in registers by the spawn
    /// before it (see `spawn_after_spawn`), which the engine doesn't know;
    /// its `pos.x` and `pos.y` are placeholders. Nothing reads an
    /// effect's position.
    pub xy_unknown: bool,
}

/// `SpawnT4BattleObjectWithId0`: effect `id` at `pos`.
pub fn spawn(b: &mut Battle, pos: Vec3, id: u8, flip: u8, palette_add: u8, priority: u8) -> Option<ObjectRef> {
    b.objects.spawn(Pool::Effect, 0, pos, [id, flip, palette_add, priority])
}

/// `SpawnT4BattleObjectWithId0` called again straight after a spawn at
/// height `z`, without setting a position: Z is still the previous
/// spawn's, but X and Y hold what the object allocator left in those
/// registers (list-node addresses, objects-and-player.md §A.3). The effect
/// gets placeholder X and Y and is marked as not knowing them.
pub fn spawn_after_spawn(b: &mut Battle, z: i32, id: u8, flip: u8, palette_add: u8, priority: u8) -> Option<ObjectRef> {
    let r = spawn(b, Vec3 { x: 0, y: 0, z }, id, flip, palette_add, priority)?;
    if let crate::kinds::Vars::Effect(v) = &mut b.objects.get_mut(r).vars {
        v.xy_unknown = true;
    }
    Some(r)
}

/// Whether `r` is an effect whose X and Y the engine doesn't know.
pub fn xy_unknown(b: &Battle, r: ObjectRef) -> bool {
    matches!(&b.objects.get(r).vars, crate::kinds::Vars::Effect(v) if v.xy_unknown)
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
    let EffectSprite { sprite: id, anim, .. } = EFFECTS[id as usize];
    let sprite = b.objects.sprite_mut(r);
    sprite.load(id);
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
