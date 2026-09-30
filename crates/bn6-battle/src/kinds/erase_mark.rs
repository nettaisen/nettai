//! A mark on a panel EraseMan aims at (effect object #0x62,
//! `sub_80E78BC`): it shows for Param1 ticks (it runs in time stop).
//! Purely visual, but it holds an effect slot.

use crate::battle::Battle;
use crate::content::SpriteId;
use crate::kinds::common;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0x62;

const SPRITE: SpriteId = SpriteId { category: 0x10, index: 0x50 };

/// `sub_80E7942`: a mark on `panel` for `ticks` ticks. (Its Z is the 1
/// its spawner leaves in r3, of which the init keeps the fraction.)
pub fn spawn(b: &mut Battle, owner: ObjectRef, panel: PanelPos, ticks: u8) -> Option<ObjectRef> {
    let r = b.objects.spawn(Pool::Effect, INDEX, Vec3 { x: 0, y: 0, z: 1 }, [ticks, 0, 0, 0])?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.panel = panel;
    o.alliance = alliance;
    o.flip = flip;
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => {
            // sub_80E792A
            let o = b.objects.get_mut(r);
            let left = o.timer as i32 - 1;
            o.timer = left as u16;
            if b.is_battle_over() || left <= 0 {
                b.objects.free(r);
            }
        }
        _ => b.objects.free(r),
    }
    common::update_sprite_in_time_stop(b, r);
}

/// `sub_80E78E0`.
fn init(b: &mut Battle, r: ObjectRef) {
    common::set_coordinates_from_panels(b, r);
    let o = b.objects.get_mut(r);
    o.pos.z &= 0xFFFF;
    o.flags |= flags::VISIBLE;
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = 0;
    o.anim_loaded = 0;
    o.timer = o.params[0] as u16;
    let flip = o.alliance ^ o.flip;
    let s = b.objects.sprite_mut(r);
    s.load(SPRITE);
    s.look.shadow = Shadow::WithSprite;
    s.look.palette = 0;
    s.look.set_flip(flip);
    s.set_animation(0, &b.content);
    common::set_progress(b, r, common::Progress::UPDATE);
}
