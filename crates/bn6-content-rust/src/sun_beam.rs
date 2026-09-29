//! GunDelSol's sun beam (effect object #0x48, `sub_80E5C2C`): the light
//! shining on the target column. It follows its owner at a fixed offset
//! and ends itself once the owner's first related object is cleared.

use bn6_content_api::api::{ObjectFields, SpriteFields};
use bn6_content_api::{CoreApi, Lifecycle, ObjectRef, Pool, Shadow, Vec3, content_state};

use crate::data::{SUN_BEAM_SPRITES, SunBeamLook};
use crate::{Slot, set_state, state};

pub const INDEX: u8 = 0x48;

content_state! {
    /// Sun beam state.
    pub struct State {
        slot: Slot,
        /// Offset from the owner's position, 16.16.
        offset: Vec3,
        /// Ticks shown (it hums every 11).
        ticks: u16,
    }
}

/// `sub_80E5D12`: a beam at `offset` from `owner`, stored in `slot`.
pub fn spawn(api: &mut dyn CoreApi, owner: ObjectRef, look: SunBeamLook, offset: Vec3, slot: Slot) -> Option<ObjectRef> {
    let r = api.spawn(Pool::Effect, INDEX, offset, [look.sprite, look.palette, 0, 0])?;
    api.set_related1(r, Some(owner));
    let (alliance, flip) = (api.alliance(owner), api.flip(owner));
    api.set_alliance(r, alliance);
    api.set_flip(r, flip);
    set_state(api, r, &State { slot, offset, ticks: 0 });
    slot.set(api, owner, Some(r));
    Some(r)
}

/// `sub_80E5D3E`: end the beam (its next update frees it).
pub fn end(api: &mut dyn CoreApi, r: ObjectRef) {
    api.set_lifecycle(r, Lifecycle::Destroy);
}

pub fn update(api: &mut dyn CoreApi, me: ObjectRef) {
    match api.lifecycle(me) {
        Lifecycle::Init => init(api, me),
        Lifecycle::Update => follow(api, me),
        Lifecycle::Destroy => api.free(me),
    }
}

/// `sub_80E5C4C`.
fn init(api: &mut dyn CoreApi, me: ObjectRef) {
    let (look, palette) = (api.param(me, 0), api.param(me, 1));
    let flip = api.alliance(me) ^ api.flip(me);
    api.sprite_load(me, SUN_BEAM_SPRITES[look as usize]);
    api.sprite_set_animation(me, 0);
    api.sprite_step(me);
    use bn6_content_api::api::OtherFields;
    api.set_shadow(me, Shadow::WithSprite);
    api.set_hflip(me, flip & 1 != 0);
    api.set_vflip(me, flip & 2 != 0);
    api.set_palette(me, palette);
    api.set_no_sprite_update(me, false);
    api.set_anim(me, 0);
    api.set_anim_loaded(me, 0);
    api.set_visible(me, true);
    let s: State = state(api, me);
    set_state(api, me, &State { ticks: 0, ..s });
    api.set_lifecycle(me, Lifecycle::Update);
    follow(api, me);
}

/// `sub_80E5CA0`: hidden in time stop (unless the third parameter is 1);
/// otherwise follow the owner, or end once the slot is cleared.
fn follow(api: &mut dyn CoreApi, me: ObjectRef) {
    if api.param(me, 2) != 1 && api.is_time_stop() {
        api.set_visible(me, false);
        return;
    }
    api.set_visible(me, true);
    let s: State = state(api, me);
    let owner = api.related1(me).expect("sun beam has an owner");
    if !s.slot.is_occupied(api, owner) {
        api.set_visible(me, false);
        api.set_lifecycle(me, Lifecycle::Destroy);
        return;
    }
    if s.ticks % 11 == 0 {
        api.play_sound(0xF9);
    }
    set_state(api, me, &State { ticks: s.ticks.wrapping_add(1), ..s });
    let p = api.pos(owner);
    api.set_pos(me, p.wrapping_add(s.offset));
    api.update_sprite(me);
}
