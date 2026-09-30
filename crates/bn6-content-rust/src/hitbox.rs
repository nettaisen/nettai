//! One-tick hit regions (attack object #3, `object_spawnCollisionRegion`).
//! It registers on its panels, resolves against whatever is there, and
//! (by default) frees itself within its first update.

use bn6_content_api::api::{CollisionFields, ObjectFields, OtherFields};
use bn6_content_api::{CoreApi, Lifecycle, ObjectRef, PanelPos, Pool, Vec3, content_state};

use crate::{set_state, state};

pub const INDEX: u8 = 3;

content_state! {
    /// Hitbox state: what the spawner passed besides the header fields.
    pub struct State {
        hit_mod: u8,
        status: u8,
        bug: u8,
        bug_arg: u8,
    }
}

/// What a hitbox does.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spec {
    pub panel: PanelPos,
    pub element: u8,
    pub z: i32,
    /// Region shape.
    pub region: u8,
    pub hit_effect: u8,
    /// Collision type indices.
    pub target: u8,
    pub self_type: u8,
    pub damage: u16,
    pub stamina: u16,
    pub hit_mod: u8,
    pub status: u8,
    pub bug: u8,
    pub bug_arg: u8,
}

/// `object_spawnCollisionRegion`, spawned by `owner`.
pub fn spawn(api: &mut dyn CoreApi, owner: ObjectRef, s: &Spec) -> Option<ObjectRef> {
    // The spawn position is the caller's registers (panel y, element, z)
    // until init places it.
    let pos = Vec3 { x: s.panel.y as i32, y: s.element as i32, z: s.z };
    let r = api.spawn(Pool::Attack, INDEX, pos, [s.region, s.hit_effect, s.target, s.self_type])?;
    let (alliance, flip) = (api.alliance(owner), api.flip(owner));
    api.set_panel_x(r, s.panel.x);
    api.set_panel_y(r, s.panel.y);
    api.set_element(r, s.element);
    api.set_damage(r, s.damage);
    api.set_stamina(r, s.stamina);
    api.set_alliance(r, alliance);
    api.set_flip(r, flip);
    set_state(api, r, &State { hit_mod: s.hit_mod, status: s.status, bug: s.bug, bug_arg: s.bug_arg });
    Some(r)
}

pub fn update(api: &mut dyn CoreApi, me: ObjectRef) {
    match api.lifecycle(me) {
        Lifecycle::Init => init(api, me),
        Lifecycle::Update => resolve(api, me),
        Lifecycle::Destroy => api.destroy(me),
    }
}

/// `sub_80C52D0`.
fn init(api: &mut dyn CoreApi, me: ObjectRef) {
    let p = PanelPos { x: api.panel_x(me), y: api.panel_y(me) };
    if !api.panel_valid(p) {
        api.free(me);
        return;
    }
    let (x, y) = api.panel_center(p);
    let pos = api.pos(me);
    api.set_pos(me, Vec3 { x, y, ..pos });
    if !api.create_collision(me) {
        api.free(me);
        return;
    }
    let s: State = state(api, me);
    let (region, hit_effect, target, self_type) =
        (api.param(me, 0), api.param(me, 1), api.param(me, 2), api.param(me, 3));
    api.setup_collision(me, self_type, target, s.hit_mod);
    api.set_region(me, region);
    api.set_hit_effect(me, hit_effect);
    if s.status != 0 {
        api.set_status_base(me, s.status);
    }
    if s.bug != 0 {
        api.set_bugs(me, (s.bug_arg as u16) << 8 | s.bug as u16);
    }
    api.present_collision(me);
    api.set_lifecycle(me, Lifecycle::Update);
    resolve(api, me);
}

/// `sub_80C532E`.
fn resolve(api: &mut dyn CoreApi, me: ObjectRef) {
    api.remove_collision(me);
    api.hit_spark(me);
    if api.hit_flags(me) == 0 {
        let left = api.timer(me) as i32 - 1;
        api.set_timer(me, left as u16);
        if left > 0 {
            api.present_collision(me);
            return;
        }
    }
    api.set_region(me, 0);
    api.free_collision(me);
    api.free(me);
}
