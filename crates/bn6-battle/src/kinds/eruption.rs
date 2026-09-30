//! A volcano panel's eruption (attack object #7, `sub_80C5A34`), spawned
//! by the panel update (`Battle::erupt`): a pillar of fire on the panel
//! whose collision turns on for one stretch of its life. It is cut short
//! once it has hit something. See docs/engine/field-collision-damage.md.

use crate::battle::Battle;
use crate::data::SpriteId;
use crate::field;
use crate::kinds::common::{self, Progress};
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, flags, state};

pub const INDEX: u8 = 7;

const SPRITE: SpriteId = SpriteId { category: 0x10, index: 0x24 };

/// Added to Param1 for its lifetime.
const LIFETIME: u8 = 0x28;

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => return crate::kinds::generic_destroy(b, r),
    }
    common::update_sprite(b, r);
}

/// `sub_80C5A58`: on its panel, one pixel back and down, with its
/// collision (off for now).
fn init(b: &mut Battle, r: ObjectRef) {
    let p = b.objects.get(r).panel;
    if !field::is_valid(p.x, p.y) {
        return b.objects.free(r);
    }
    let s = b.objects.sprite_mut(r);
    s.load(SPRITE);
    s.look.shadow = Shadow::WithSprite;
    s.look.palette = 0;
    s.set_animation(0);
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.flags |= flags::VISIBLE;
    o.anim = 0;
    o.anim_loaded = 0;
    o.params[0] = o.params[0].wrapping_add(LIFETIME);
    o.timer = o.params[0] as u16;
    common::set_coordinates_from_panels(b, r);
    let o = b.objects.get_mut(r);
    o.pos.y = o.pos.y.wrapping_sub(0x1_0000);
    o.pos.z = -0x1_0000;
    let Some(c) = b.create_collision(r) else {
        return b.objects.free(r);
    };
    b.setup_collision(r, 0x48, 0x2A, 1);
    let s = b.collision.get_mut(c);
    s.hit_effect = 1;
    s.region = 0;
    b.present_collision(c);
    common::set_progress(b, r, Progress::UPDATE);
    tick(b, r);
}

/// `sub_80C5AD4`: resolve the last tick's hits; once it hit something (or
/// its panel stops being a volcano with the collision on), the collision
/// goes off and it ends 3 ticks later; its animation and collision follow
/// the timer.
fn tick(b: &mut Battle, r: ObjectRef) {
    let c = b.objects.get(r).collision.expect("an eruption has collision data");
    b.remove_collision(c);
    crate::kinds::spark::spawn_collision_effect(b, r);
    if b.is_battle_over() {
        b.collision.get_mut(c).region = 0;
        common::set_progress(b, r, Progress::DESTROY);
        return b.present_collision(c);
    }
    let p = b.objects.get(r).panel;
    let panel = b.field.flags(p.x, p.y);
    let end = if panel & 0x1000 == 0 {
        if b.collision.get(c).region != 0 {
            b.objects.get_mut(r).timer = 3;
            true
        } else {
            false
        }
    } else {
        b.collision.get(c).acc.hit_flags != 0
    };
    if end {
        b.collision.get_mut(c).region = 0;
        if b.field.flags(p.x, p.y) & 0x0380_0000 != 0 {
            b.objects.get_mut(r).flags &= !flags::VISIBLE;
        }
    }
    let o = b.objects.get_mut(r);
    let left = o.timer as i32 - 1;
    o.timer = left as u16;
    if left <= 0 {
        b.collision.get_mut(c).region = 0;
        common::set_progress(b, r, Progress::DESTROY);
    } else {
        by_timer(b, r, c);
    }
    b.present_collision(c);
}

/// `sub_80C5B40`: 40 ticks before the end the collision comes on (and it
/// rises to the panel), with the rising animation; then the pillar; the
/// fall at the end.
fn by_timer(b: &mut Battle, r: ObjectRef, c: crate::collision::CollisionId) {
    let o = b.objects.get(r);
    let (t, start) = (o.timer as i32, o.params[0] as i32 - LIFETIME as i32);
    let anim = if t == start {
        b.collision.get_mut(c).region = 1;
        let o = b.objects.get_mut(r);
        o.pos.y = o.pos.y.wrapping_add(0x1_0000);
        o.pos.z = 0;
        1
    } else if t == start - 2 {
        2
    } else if t == 2 {
        1
    } else {
        return;
    };
    b.objects.get_mut(r).anim = anim;
}
