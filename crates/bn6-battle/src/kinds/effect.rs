//! Generic one-shot effects (effect object #0: explosions, sparkles...).
//! Visual, but they hold pool slots and list positions, and without a
//! timer they live exactly as long as their animation.
//! See docs/engine/objects-and-player.md §A.3.

use crate::battle::Battle;
use crate::content::EffectSprite;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

/// Effect-private state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// Visibility follows `related[0]`.
    pub follow_related: bool,
    /// The game spawned it with X and Y left in registers by the spawn
    /// before it (see `spawn_after_spawn`), which the engine doesn't know;
    /// its `pos.x` and `pos.y` are placeholders. Nothing reads an
    /// effect's position.
    pub xy_unknown: bool,
    /// Its look, when a definition gave it (else effect `Param1`'s).
    pub look: Option<EffectSprite>,
}

/// `SpawnT4BattleObjectWithId0`: effect `id` at `pos`.
pub fn spawn(b: &mut Battle, pos: Vec3, id: u8, flip: u8, palette_add: u8, priority: u8) -> Option<ObjectRef> {
    b.objects.spawn(Pool::Effect, 0, pos, [id, flip, palette_add, priority])
}

/// The same with the look an effect definition gives (its first parameter
/// is then 0).
pub fn spawn_look(b: &mut Battle, pos: Vec3, look: EffectSprite, flip: u8, palette_add: u8, priority: u8) -> Option<ObjectRef> {
    let r = spawn(b, pos, 0, flip, palette_add, priority)?;
    if let crate::kinds::Vars::Effect(v) = &mut b.objects.get_mut(r).vars {
        v.look = Some(look);
    }
    Some(r)
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

/// `sub_801BD3C`: effect `id` on each field panel of hit region `region`
/// around (x, y), turned the way side `side` faces, at height `z`; for a
/// whole-field region (0x80 and up), on each panel it covers from the
/// bottom right, on the ground.
pub fn spawn_over_region(b: &mut Battle, x: i32, y: i32, region: u8, side: u8, id: u8, z: i32) {
    spawn_over_region_as(b, x, y, region, side, z, |b, pos| {
        spawn(b, pos, id, 0, 0, 0);
    });
}

/// The same with the look an effect definition gives.
pub fn spawn_look_over_region(b: &mut Battle, x: i32, y: i32, region: u8, side: u8, look: EffectSprite, z: i32) {
    spawn_over_region_as(b, x, y, region, side, z, |b, pos| {
        spawn_look(b, pos, look, 0, 0, 0);
    });
}

/// `sub_801BD3C`'s walk over the region, spawning with `spawn`.
fn spawn_over_region_as(b: &mut Battle, x: i32, y: i32, region: u8, side: u8, z: i32, spawn: impl Fn(&mut Battle, Vec3)) {
    let at = |b: &mut Battle, px: u8, py: u8, z: i32| {
        let (cx, cy) = crate::kinds::player::panel_coordinates(px, py);
        spawn(b, Vec3 { x: cx, y: cy, z });
    };
    if region & 0x80 != 0 {
        let cond = b.content.rules.field_regions[(region & 0x7F) as usize];
        for py in (1..=3).rev() {
            for px in (1..=6).rev() {
                if b.field.check(px, py, cond.require, cond.forbid) {
                    at(b, px, py, 0);
                }
            }
        }
        return;
    }
    // `object_getAllianceDirection`: by side alone, whatever the flip.
    let dir = if side == 0 { 1 } else { -1 };
    for o in b.content.region(region).to_vec() {
        let (px, py) = (x + o.dx as i32 * dir, y + o.dy as i32);
        if (1..=6).contains(&px) && (1..=3).contains(&py) {
            at(b, px as u8, py as u8, z);
        }
    }
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
    let [id, flip, palette_add, priority] = b.objects.get(r).params;
    let given = match &b.objects.get(r).vars {
        crate::kinds::Vars::Effect(v) => v.look,
        _ => None,
    };
    let EffectSprite { sprite: id, anim, palette } = given.unwrap_or_else(|| b.content.effect(id));
    let sprite = b.objects.sprite_mut(r);
    sprite.load(id);
    sprite.set_animation(anim, &b.content);
    sprite.update(&b.content);
    let look = &mut sprite.look;
    look.shadow = Shadow::WithSprite;
    look.palette = palette.wrapping_add(palette_add);
    look.set_flip(flip);
    if priority != 0 {
        look.priority = 0;
    }
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
    b.objects.sprite_mut(r).update(&b.content);
    if destroy {
        let o = b.objects.get_mut(r);
        o.flags &= !flags::VISIBLE;
        o.state = state::DESTROY;
    }
}
