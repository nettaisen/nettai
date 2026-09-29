//! GunDelSol's sun beam (effect object #0x48, `sub_80E5C2C`): the light
//! shining on the target column. Visual only, but it takes an effect slot
//! and a place in the update order. It follows its owner at a fixed
//! offset and ends itself once the owner's first related object is
//! cleared. See objects-and-player.md §A.6.

use crate::battle::Battle;
use crate::data::attacks::{self, SunBeamLook};
use crate::kinds::attachment::AttachSlot;
use crate::kinds::common::{Progress, set_progress, update_sprite};
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0x48;

/// Sun-beam-private state.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    pub slot: Option<AttachSlot>,
    /// Offset from the owner's position, 16.16.
    pub offset: Vec3,
    /// Ticks shown (it hums every 11).
    pub ticks: u16,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::SunBeam(v) => v,
        v => panic!("sun beam with {v:?}"),
    }
}

/// `sub_80E5D12`: a beam at `offset` from `owner`, stored in `slot`.
pub fn spawn(b: &mut Battle, owner: ObjectRef, look: SunBeamLook, offset: Vec3, slot: AttachSlot) -> Option<ObjectRef> {
    let r = b.objects.spawn(Pool::Effect, INDEX, offset, [look.sprite, look.palette, 0, 0])?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flip = flip;
    o.vars = crate::kinds::Vars::SunBeam(Vars { slot: Some(slot), offset, ticks: 0 });
    slot.set(b, Some(r));
    Some(r)
}

/// `sub_80E5D3E`: end the beam (its next update frees it).
pub fn end(b: &mut Battle, r: ObjectRef) {
    set_progress(b, r, Progress::DESTROY);
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => follow(b, r),
        _ => b.objects.free(r),
    }
}

/// `sub_80E5C4C`.
fn init(b: &mut Battle, r: ObjectRef) {
    let [look, palette, ..] = b.objects.get(r).params;
    let flip = b.objects.get(r).alliance ^ b.objects.get(r).flip;
    let sprite = b.objects.sprite_mut(r);
    sprite.load(attacks::sun_beam_sprite(look));
    sprite.set_animation(0);
    sprite.update();
    // sprite_noShadow; sprite_setFlip(object_getFlip()); the palette.
    sprite.look.shadow = crate::object::sprite::Shadow::WithSprite;
    sprite.look.set_flip(flip);
    sprite.look.palette = palette;
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = 0;
    o.anim_loaded = 0;
    o.flags |= flags::VISIBLE;
    vars(b, r).ticks = 0;
    set_progress(b, r, Progress::UPDATE);
    follow(b, r);
}

/// `sub_80E5CA0`: hidden in time stop (unless the third parameter is 1);
/// otherwise follow the owner, or end once the slot is cleared.
fn follow(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).params[2] != 1 && b.is_time_stop() {
        b.objects.get_mut(r).flags &= !flags::VISIBLE;
        return;
    }
    b.objects.get_mut(r).flags |= flags::VISIBLE;
    let v = vars(b, r).clone();
    if !v.slot.expect("sun beam has a slot").is_occupied(b) {
        b.objects.get_mut(r).flags &= !flags::VISIBLE;
        set_progress(b, r, Progress::DESTROY);
        return;
    }
    if v.ticks % 11 == 0 {
        b.play_sound(crate::sound::SoundId(0xF9));
    }
    vars(b, r).ticks = v.ticks.wrapping_add(1);
    let owner = b.objects.get(r).related[0].expect("sun beam has an owner");
    let p = b.objects.get(owner).pos;
    b.objects.get_mut(r).pos =
        Vec3 { x: p.x.wrapping_add(v.offset.x), y: p.y.wrapping_add(v.offset.y), z: p.z.wrapping_add(v.offset.z) };
    update_sprite(b, r);
}
