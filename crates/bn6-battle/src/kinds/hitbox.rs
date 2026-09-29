//! One-tick hit regions (attack object #3, `object_spawnCollisionRegion`).
//! Most attacks use one: it registers on its panels, resolves against
//! whatever is there, and (by default) frees itself within its first
//! update. See docs/engine/field-collision-damage.md §3.5.

use crate::battle::Battle;
use crate::kinds::player::panel_coordinates;
use crate::object::{ObjectRef, PanelPos, Pool, Vec3, state};

#[derive(Clone, Debug, Default)]
pub struct Vars {
    pub hit_mod: u8,
    pub status: u8,
    pub bug: u8,
    pub bug_arg: u8,
    /// Where to report whether a body was hit.
    pub report: Option<HitReport>,
}

/// Where a hitbox reports `hit_flags & 0x0C000000` (the bodies it hit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitReport {
    /// Into the spawner's attack scratch, at this offset.
    AttackScratch(ObjectRef, usize),
}

/// What a hitbox does.
#[derive(Clone, Copy, Debug, Default)]
pub struct HitboxSpec {
    pub panel: PanelPos,
    pub element: u8,
    pub z: i32,
    /// Region shape (see `data::collision_generated::REGIONS`).
    pub region: u8,
    pub hit_effect: u8,
    /// Collision type indices.
    pub target: u8,
    pub self_type: u8,
    /// The damage word (damage | flag bits) and the counter byte.
    pub damage: u16,
    pub stamina: u16,
    pub hit_mod: u8,
    pub status: u8,
    pub bug: u8,
    pub bug_arg: u8,
}

/// `object_spawnCollisionRegion`, spawned by `owner`.
pub fn spawn(b: &mut Battle, owner: ObjectRef, s: &HitboxSpec) -> Option<ObjectRef> {
    // The spawn position is the caller's registers (panel y, element, z)
    // until init places it.
    let pos = Vec3 { x: s.panel.y as i32, y: s.element as i32, z: s.z };
    let params = [s.region, s.hit_effect, s.target, s.self_type];
    let r = b.objects.spawn(Pool::Attack, 3, pos, params)?;
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
    o.vars = crate::kinds::Vars::Hitbox(Vars { hit_mod: s.hit_mod, status: s.status, bug: s.bug, bug_arg: s.bug_arg, report: None });
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
    let params = b.objects.get(r).params;
    b.setup_collision(r, params[3], params[2], v.hit_mod);
    let s = b.collision.get_mut(c);
    s.region = params[0];
    s.hit_effect = params[1];
    if v.status != 0 {
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
    let hit = b.collision.get(c).acc.hit_flags;
    if let Some(HitReport::AttackScratch(owner, off)) = vars(b, r).report
        && let Some(a) = b.objects.get(owner).actor
    {
        b.actors.get_mut(a).attack.set_scratch_u32(off, hit & 0x0C00_0000);
    }
    if hit == 0 {
        let o = b.objects.get_mut(r);
        let left = o.timer as i32 - 1;
        o.timer = left as u16;
        if left > 0 {
            b.present_collision(c);
            return;
        }
    }
    b.collision.get_mut(c).region = 0;
    b.collision.free(c);
    b.objects.free(r);
}
