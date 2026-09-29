//! A segment of EraseMan's slash (attack object #0xC3, `sub_80DD940`): it
//! stands on its panel with its collision on (status 0x10) for Param2
//! ticks, or until it has hit something. With Param3 0 (EraseMan's) it
//! runs in time stop.

use crate::battle::Battle;
use crate::data::SpriteId;
use crate::kinds::common::{self, Progress};
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0xC3;

const SPRITE: SpriteId = SpriteId { category: 0x10, index: 0x51 };

/// By aim (Param1): its height (whole pixels, `dword_80DD9FC`) and
/// animation (`dword_80DDA04`).
const HEIGHTS: [i32; 3] = [0x23, 0x28, 0x2D];
const ANIMS: [u8; 3] = [1, 0, 2];

/// `sub_80DDA5A`: a segment on `panel` for the slash `aim`, lasting
/// `ticks`.
pub fn spawn(
    b: &mut Battle,
    owner: ObjectRef,
    panel: PanelPos,
    element: u8,
    aim: u8,
    ticks: u8,
    damage: u32,
) -> Option<ObjectRef> {
    let r = b.objects.spawn(Pool::Attack, INDEX, Vec3::default(), [aim, ticks, 0, 0])?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.panel = panel;
    o.element = element;
    o.related[0] = Some(owner);
    o.damage = damage as u16;
    o.stamina = (damage >> 16) as u16;
    o.alliance = alliance;
    o.flip = flip;
    o.flags |= flags::RUN_IN_TIME_STOP;
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => return crate::kinds::generic_destroy(b, r),
    }
    if b.objects.get(r).params[2] != 0 {
        panic!("EraseMan beams with Param3 (object_updateSpritePaused) are not implemented yet");
    }
    common::update_sprite_in_time_stop(b, r);
}

/// `sub_80DD970`.
fn init(b: &mut Battle, r: ObjectRef) {
    common::set_coordinates_from_panels(b, r);
    let aim = b.objects.get(r).params[0] as usize;
    let o = b.objects.get_mut(r);
    o.pos.z = (o.pos.z & 0xFFFF) | HEIGHTS[aim] << 16;
    let front = common::facing(o.alliance, o.flip);
    o.pos.x = o.pos.x.wrapping_sub(front * 0x8_0000);
    o.flags |= flags::VISIBLE;
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = ANIMS[aim];
    o.anim_loaded = ANIMS[aim];
    let flip = o.alliance ^ o.flip;
    let s = b.objects.sprite_mut(r);
    s.load(SPRITE);
    s.look.shadow = Shadow::WithSprite;
    s.look.palette = 0;
    s.look.set_flip(flip);
    s.set_animation(ANIMS[aim]);
    let Some(c) = b.create_collision(r) else {
        return b.objects.free(r);
    };
    b.setup_collision(r, 0x16, 5, 1);
    let s = b.collision.get_mut(c);
    s.status_base = 0x10;
    s.hit_effect = 0xC;
    b.present_collision(c);
    let o = b.objects.get_mut(r);
    o.timer = o.params[1] as u16;
    common::set_progress(b, r, Progress::UPDATE);
}

/// `sub_80DDA08`: resolve the hits; once it hit something, its region
/// goes; after its time, it ends.
fn tick(b: &mut Battle, r: ObjectRef) {
    let c = b.objects.get(r).collision.expect("a beam has collision data");
    b.remove_collision(c);
    crate::kinds::spark::spawn_collision_effect(b, r);
    if !b.is_battle_over() {
        if b.collision.get(c).acc.hit_flags != 0 {
            b.collision.get_mut(c).region = 0;
        }
        let o = b.objects.get_mut(r);
        let left = o.timer as i32 - 1;
        o.timer = left as u16;
        if left > 0 {
            return b.present_collision(c);
        }
    }
    b.collision.get_mut(c).region = 0;
    common::set_progress(b, r, Progress::DESTROY);
}
