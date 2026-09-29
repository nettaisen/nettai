//! Small helpers many object behaviors share: sprite stepping, panel and
//! coordinate conversion, the state word, and damage totals.

use crate::battle::Battle;
use crate::field::PanelType;
use crate::object::{ObjectRef, PanelPos, Vec3, flags, state};

/// `object_setAnimation`: request animation `anim`, restarting it even if
/// it is the current one.
pub fn set_animation(b: &mut Battle, r: ObjectRef, anim: u8) {
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = 0xFF;
}

/// `object_updateSprite`: load a newly requested animation and step the
/// sprite one tick. Skipped while paused, during time stop (unless the
/// object runs in time stop), while `prevent_anim` holds an object with
/// collision still, and for inactive or non-animating objects.
pub fn update_sprite(b: &mut Battle, r: ObjectRef) {
    if b.paused {
        return;
    }
    let o = b.objects.get(r);
    if o.flags & flags::ACTIVE == 0 || o.flags & flags::NO_SPRITE_UPDATE != 0 {
        return;
    }
    if o.flags & flags::RUN_IN_TIME_STOP == 0 && b.is_time_stop() {
        return;
    }
    if o.collision.is_some() && o.prevent_anim != 0 {
        return;
    }
    let (anim, loaded) = (o.anim, o.anim_loaded);
    if anim != loaded {
        b.objects.sprite_mut(r).set_animation(anim);
        b.objects.get_mut(r).anim_loaded = anim;
    }
    b.objects.sprite_mut(r).update();
}

/// `sub_800E258`: the panel a field position is over (x 1..=6 and y
/// 1..=3 on the field; positions off the field give border or wrapped
/// values, as in the game).
pub fn panel_at(x: i32, y: i32) -> PanelPos {
    PanelPos { x: (((x >> 16) + 0xA0) / 0x28) as u8, y: (((y >> 16) + 0x20) / 0x18) as u8 }
}

/// `object_setPanelsFromCoordinates`.
pub fn set_panels_from_coordinates(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.panel = panel_at(o.pos.x, o.pos.y);
}

/// `object_setCoordinatesFromPanels`: x and y from the panel (z is left
/// alone).
pub fn set_coordinates_from_panels(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    let (x, y) = crate::kinds::player::panel_coordinates(o.panel.x, o.panel.y);
    o.pos.x = x;
    o.pos.y = y;
}

/// `object_getFlipDirection`: +1 when facing right, -1 when facing left.
pub fn facing(alliance: u8, flip: u8) -> i32 {
    if alliance ^ flip == 0 { 1 } else { -1 }
}

/// `object_highlightPanel`: mark a panel for highlighting (drawn only).
pub fn highlight_panel(b: &mut Battle, x: u8, y: u8) {
    if let Some(p) = b.field.panel_mut(x, y) {
        p.highlight = 1;
    }
}

/// Where an object is in its behavior: lifecycle state, action, phase and
/// whether the phase's entry ran. The game saves and restores these
/// together (one word store).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub state: u8,
    pub action: u8,
    pub phase: u8,
    pub phase_init: u8,
}

impl Progress {
    /// Destroy at the next update (state 8, everything else 0).
    pub const DESTROY: Progress = Progress { state: state::DESTROY, action: 0, phase: 0, phase_init: 0 };
    /// Running, from the first action (state 4, everything else 0).
    pub const UPDATE: Progress = Progress { state: state::UPDATE, action: 0, phase: 0, phase_init: 0 };
}

pub fn progress(b: &Battle, r: ObjectRef) -> Progress {
    let o = b.objects.get(r);
    Progress { state: o.state, action: o.action, phase: o.phase, phase_init: o.phase_init }
}

pub fn set_progress(b: &mut Battle, r: ObjectRef, p: Progress) {
    let o = b.objects.get_mut(r);
    o.state = p.state;
    o.action = p.action;
    o.phase = p.phase;
    o.phase_init = p.phase_init;
}

/// Set the action and restart its phases.
pub fn set_action(b: &mut Battle, r: ObjectRef, action: u8) {
    let o = b.objects.get_mut(r);
    o.action = action;
    o.phase = 0;
    o.phase_init = 0;
}

/// `object_calculateFinalDamage2`: total this tick's damage by element
/// (halved, rounding up, on a holy panel) into `final_damage`.
pub fn total_damage(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get(r);
    let holy = b.field.panel(o.panel.x, o.panel.y).is_some_and(|p| p.kind == PanelType::Holy) as u32;
    let c = o.collision.expect("object with collision data");
    let acc = &mut b.collision.get_mut(c).acc;
    let mut total = 0u32;
    for d in &mut acc.element_damage[..5] {
        let v = (*d as u32 + holy) >> holy;
        *d = v as u16;
        total += v;
    }
    acc.final_damage = total as u16;
}

/// `object_spawnHiteffect`: an object that blocked a hit shows a guard
/// spark (one simulation RNG draw).
pub fn spawn_guard_spark(b: &mut Battle, r: ObjectRef) {
    if b.paused {
        return;
    }
    let o = b.objects.get(r);
    let c = o.collision.expect("object with collision data");
    // Hit flag 0x20000: it blocked a hit.
    if b.collision.get(c).acc.hit_flags & 0x2_0000 == 0 {
        return;
    }
    let pos = Vec3 { z: o.pos.z.wrapping_add(0x10 << 16), ..o.pos };
    let pos = crate::kinds::spark::jitter(b, 0xF, pos);
    crate::kinds::spark::spawn(b, r, pos, 8);
}
