//! One-tick hit regions (attack object #3, `object_spawnCollisionRegion`).
//! Most attacks use one: it registers on its panels, resolves against
//! whatever is there, and (by default) frees itself within its first
//! update. See docs/engine/field-collision-damage.md §3.5.

use crate::battle::Battle;
use crate::kinds::player::panel_coordinates;
use crate::object::{ObjectRef, PanelPos, Vec3, state};
use bn6_content_api::{CollisionHandle, RegionHandle, SparkHandle};

#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// The region it covers, the spark its hits show, and its collision
    /// types (what it reaches, what it is): the original's four spawn
    /// parameters.
    pub region: Option<RegionHandle>,
    pub hit_effect: Option<SparkHandle>,
    pub target: CollisionHandle,
    pub self_type: CollisionHandle,
    pub hit_mod: u8,
    pub status: Option<bn6_content_api::StatusHandle>,
    pub bug: u8,
    pub bug_arg: u8,
    // The game also lets the spawner pass a slot that receives which bodies
    // were hit (`hit_flags & 0x0C000000`). Added as a typed field on the
    // spawner's action state when the first attack that reads it is ported.
}

/// What a hitbox does.
#[derive(Clone, Copy, Debug, Default)]
pub struct HitboxSpec {
    pub panel: PanelPos,
    pub element: u8,
    pub z: i32,
    /// The region it covers and the spark its hits show, or none.
    pub region: Option<RegionHandle>,
    pub hit_effect: Option<SparkHandle>,
    /// Its collision types: what it reaches, what it is.
    pub target: CollisionHandle,
    pub self_type: CollisionHandle,
    /// The damage word (damage | flag bits) and the counter byte.
    pub damage: u16,
    pub stamina: u16,
    pub hit_mod: u8,
    pub status: Option<bn6_content_api::StatusHandle>,
    pub bug: u8,
    pub bug_arg: u8,
}

/// `object_spawnCollisionRegion`, spawned by `owner`.
pub fn spawn(b: &mut Battle, owner: ObjectRef, s: &HitboxSpec) -> Option<ObjectRef> {
    // The spawn position is the caller's registers (panel y, element, z)
    // until init places it.
    let pos = Vec3 { x: s.panel.y as i32, y: s.element as i32, z: s.z };
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::Hitbox, pos, [0; 4])?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.panel = s.panel;
    o.element = s.element;
    o.damage = s.damage;
    o.stamina = s.stamina;
    o.alliance = alliance;
    o.flip = flip;
    b.objects.get_mut(r).vars = crate::kinds::Vars::Hitbox(Vars {
        region: s.region,
        hit_effect: s.hit_effect,
        target: s.target,
        self_type: s.self_type,
        hit_mod: s.hit_mod,
        status: s.status,
        bug: s.bug,
        bug_arg: s.bug_arg,
    });
    Some(r)
}

fn vars(b: &Battle, r: ObjectRef) -> Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::Hitbox(v) => v.clone(),
        _ => unreachable!("hitbox without hitbox state"),
    }
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => resolve(b, r),
        _ => crate::kinds::generic_destroy(b, r),
    }
}

/// `sub_80C52D0`.
fn init(b: &mut Battle, r: ObjectRef) {
    let p = b.objects.get(r).panel;
    if !crate::field::is_valid(p.x, p.y) {
        b.objects.free(r);
        return;
    }
    let (x, y) = panel_coordinates(p.x, p.y);
    {
        let o = b.objects.get_mut(r);
        o.pos.x = x;
        o.pos.y = y;
    }
    let Some(c) = b.create_collision(r) else {
        b.objects.free(r);
        return;
    };
    let v = vars(b, r);
    b.setup_collision(r, v.self_type, v.target, v.hit_mod);
    let s = b.collision.get_mut(c);
    s.region = v.region;
    s.hit_effect = v.hit_effect;
    if v.status.is_some() {
        s.status_base = v.status;
    }
    if v.bug != 0 {
        s.bugs = (v.bug_arg as u16) << 8 | v.bug as u16;
    }
    b.present_collision(c);
    let o = b.objects.get_mut(r);
    o.state = state::UPDATE;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
    resolve(b, r);
}

/// `sub_80C532E`.
fn resolve(b: &mut Battle, r: ObjectRef) {
    let Some(c) = b.objects.get(r).collision else { return };
    b.remove_collision(c);
    crate::kinds::spark::spawn_collision_effect(b, r);
    if b.collision.get(c).acc.hit_flags == 0 {
        let o = b.objects.get_mut(r);
        let left = o.timer as i32 - 1;
        o.timer = left as u16;
        if left > 0 {
            b.present_collision(c);
            return;
        }
    }
    b.collision.get_mut(c).region = None;
    b.collision.free(c);
    b.objects.free(r);
}
