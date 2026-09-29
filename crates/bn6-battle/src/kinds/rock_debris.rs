//! Rock debris (effect object #0x38): a chunk thrown up when a rock
//! breaks, which bounces off with random speed and puffs into dust where
//! it lands. Visual, but it draws the simulation RNG twice.

use crate::battle::Battle;
use crate::data::SpriteId;
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0x38;

const SPRITE: SpriteId = SpriteId { category: 0x10, index: 1 };

/// `sub_80E47A4`: a chunk at `pos` in sprite palette `palette`.
pub fn spawn(b: &mut Battle, pos: Vec3, palette: u8) -> Option<ObjectRef> {
    b.objects.spawn(Pool::Effect, INDEX, pos, [palette, 0, 0, 0])
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => fly(b, r),
        _ => land(b, r),
    }
}

/// `sub_80E46F8`: one of two chunk shapes, and a random velocity: up to
/// 3.5 pixels a tick sideways and in depth, 6 to 13.5 up.
fn init(b: &mut Battle, r: ObjectRef) {
    let shape = (b.rng.next() & 1) as u8;
    let palette = b.objects.get(r).params[0];
    let sprite = b.objects.sprite_mut(r);
    sprite.load(SPRITE);
    sprite.set_animation(shape);
    sprite.update();
    sprite.look.shadow = crate::object::sprite::Shadow::WithSprite;
    sprite.look.palette = palette;
    let v = b.rng.next();
    let nibble = |shift: u32| ((v >> shift) & 0xF) as i32;
    let o = b.objects.get_mut(r);
    o.flags = (o.flags | flags::VISIBLE) & !flags::NO_SPRITE_UPDATE;
    o.vel = Vec3 { x: (nibble(0) - 7) << 15, y: (nibble(4) - 7) << 15, z: (nibble(8) + 12) << 15 };
    o.state = state::UPDATE;
    fly(b, r);
}

/// `sub_80E475C`: move, fall, and land at height 0.
fn fly(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.pos.x = o.pos.x.wrapping_add(o.vel.x);
    o.pos.y = o.pos.y.wrapping_add(o.vel.y);
    o.pos.z = o.pos.z.wrapping_add(o.vel.z).max(0);
    o.vel.z = o.vel.z.wrapping_sub(0x14000);
    if o.pos.z == 0 {
        land(b, r);
    }
}

/// `sub_80E4790`: a dust puff, and gone.
fn land(b: &mut Battle, r: ObjectRef) {
    let pos = b.objects.get(r).pos;
    crate::kinds::effect::spawn(b, pos, 1, 0, 0, 0);
    b.objects.free(r);
}
