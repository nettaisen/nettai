//! Attachments (actor object #5, `sub_80B8CD8`): a sprite a navi holds
//! during an attack, such as GunDelSol's gun. It follows its owner's
//! position and visibility, and ends itself once the owner clears the slot
//! it was stored in.

use bn6_content_api::api::{ObjectFields, OtherFields, SpriteFields};
use bn6_content_api::{CoreApi, Lifecycle, NaviStat, ObjectRef, Pool, Shadow, Value, Vec3, content_state};

use crate::data::Data;
use crate::{Slot, set_state, state};

pub const INDEX: u8 = 5;

content_state! {
    /// Attachment state.
    pub struct State {
        slot: Slot,
        /// Offset from the owner's position (its attach point), 16.16.
        offset_x: i32,
        offset_z: i32,
        /// Pixels raised by (subtracted from y and z).
        lift: i8,
    }
}

/// `sub_80B8E30`: attach an object of `kind` to `owner`, stored in `slot`
/// (which gets None if the pool is full).
pub fn spawn(api: &mut dyn CoreApi, owner: ObjectRef, kind: u8, slot: Slot) -> Option<ObjectRef> {
    let r = api.spawn(Pool::Actor, INDEX, Vec3::default(), [kind, 0, 0, 0]);
    if let Some(r) = r {
        api.set_related1(r, Some(owner));
        let (x, y) = (api.panel_x(owner), api.panel_y(owner));
        api.set_panel_x(r, x);
        api.set_panel_y(r, y);
        let (alliance, flip) = (api.alliance(owner), api.flip(owner));
        api.set_alliance(r, alliance);
        api.set_flip(r, flip);
        api.set_run_while_paused(r, true);
        api.set_run_in_time_stop(r, true);
        set_state(api, r, &State { slot, ..State::default() });
    }
    slot.set(api, owner, r);
    r
}

pub fn update(data: &Data, api: &mut dyn CoreApi, me: ObjectRef) {
    match api.lifecycle(me) {
        Lifecycle::Init => init(data, api, me),
        Lifecycle::Update => follow(api, me),
        Lifecycle::Destroy => api.free(me),
    }
}

fn owner(api: &dyn CoreApi, me: ObjectRef) -> ObjectRef {
    api.related1(me).expect("attachment has an owner")
}

/// `sub_80B8CF8`.
fn init(data: &Data, api: &mut dyn CoreApi, me: ObjectRef) {
    let (kind, anim, palette_add) = (api.param(me, 0), api.param(me, 1), api.param(me, 3));
    let k = data.attachments[kind as usize];
    let owner = owner(api, me);
    api.sprite_load(me, k.sprite);
    api.set_shadow(me, Shadow::WithSprite);
    api.set_palette(me, k.palette.wrapping_add(palette_add));
    api.set_no_sprite_update(me, false);
    // The Cross forms' NameIDs raise it by 8 pixels.
    let name = api.name_id(owner);
    let form = api.navi_stat(api.alliance(me), NaviStat::Form);
    let cross = (0x1AC..=0x1B5).contains(&name) && form != Value::Int(0);
    let mut s: State = state(api, me);
    s.lift = if cross { -8 } else { k.lift };
    api.set_anim(me, anim);
    api.set_anim_loaded(me, anim);
    api.set_visible(me, true);
    api.sprite_set_animation(me, anim);
    if kind == 0xF {
        panic!("attachment kind 0xF offsets (sub_80B8CF8) are not implemented yet");
    }
    let (x, z) = match k.attach_point {
        Some(p) => api.attach_point(owner, p),
        None => (0, 0),
    };
    s.offset_x = x << 16;
    s.offset_z = z << 16;
    set_state(api, me, &s);
    api.set_lifecycle(me, Lifecycle::Update);
    follow(api, me);
}

/// `sub_80B8DA6`: follow the owner's position and visibility; animate,
/// except while paused (or in time stop, unless the third parameter says
/// otherwise); end once the slot is cleared.
fn follow(api: &mut dyn CoreApi, me: ObjectRef) {
    let owner = owner(api, me);
    let p = api.pos(owner);
    let s: State = state(api, me);
    let lift = (s.lift as i32) << 16;
    api.set_pos(
        me,
        Vec3 {
            x: p.x.wrapping_add(s.offset_x),
            y: p.y.wrapping_sub(lift),
            z: p.z.wrapping_add(s.offset_z).wrapping_sub(lift),
        },
    );
    let visible = api.visible(owner);
    api.set_visible(me, visible);
    // It wears the owner's colour shader, white flash, alpha and flip.
    let (shader, white, alpha) = (api.color_shader(owner), api.white(owner), api.alpha(owner));
    let (hflip, vflip) = (api.hflip(owner), api.vflip(owner));
    api.set_color_shader(me, shader);
    api.set_white(me, white);
    api.set_alpha(me, alpha);
    api.set_hflip(me, hflip);
    api.set_vflip(me, vflip);
    if !s.slot.is_occupied(api, owner) {
        api.set_visible(me, false);
        api.set_lifecycle(me, Lifecycle::Destroy);
        return;
    }
    if api.param(me, 2) == 0 && api.is_time_stop() {
        return;
    }
    if api.is_paused() {
        return;
    }
    api.update_sprite(me);
}
