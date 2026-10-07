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
/// sprite one tick. Skipped while paused, while dimmed (unless the
/// object runs while dimmed), while `prevent_anim` holds an object with
/// collision still, and for inactive or non-animating objects.
pub fn update_sprite(b: &mut Battle, r: ObjectRef) {
    if b.paused {
        return;
    }
    update_sprite_even_paused(b, r);
}

/// `sub_801BC64`: `object_updateSprite` without its pause check.
pub fn update_sprite_even_paused(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get(r);
    if o.flags & flags::ACTIVE == 0 || o.flags & flags::NO_SPRITE_UPDATE != 0 {
        return;
    }
    if o.flags & flags::RUN_WHILE_DIMMED == 0 && b.is_dimmed() {
        return;
    }
    if o.collision.is_some() && o.prevent_anim != 0 {
        return;
    }
    let (anim, loaded) = (o.anim, o.anim_loaded);
    if anim != loaded {
        b.objects.sprite_mut(r).set_animation(anim, &b.content);
        b.objects.get_mut(r).anim_loaded = anim;
    }
    b.objects.sprite_mut(r).update(&b.content);
}

/// A panel that burns (its type's `burn`: EXE5's lava, 0x08016E18 for
/// navis, 0x08016D80 for other bodies, the same tests in another order;
/// EXE4's, 0x08013128 and 0x0801309E) burns a grounded body on it that
/// isn't of fire nor spared: its damage in fire (shifted by the body's
/// weakness to fire) as a hit, and what it wears off the mood, unless the
/// body is flagged 0x09; either way the panel turns normal and its burn
/// shows (the arena's spark `panel_burn`, jittered: one simulation RNG
/// draw). Not while the battle is dimmed, unless `player` and the burn's
/// `players_while_dimmed` (EXE4's).
pub fn panel_burn(b: &mut Battle, r: ObjectRef, player: bool) {
    use crate::collision::f1;
    let Some(c) = b.objects.get(r).collision else { return };
    let d = b.collision.get(c);
    let p = d.panel;
    let Some(kind) = b.field.panel(p.x, p.y).map(|p| p.kind) else { return };
    let rules = b.content.rules();
    let Some(burn) = rules.panels.types[kind as usize].burn else { return };
    if b.is_dimmed() && !(player && burn.players_while_dimmed) {
        return;
    }
    if d.element == 1 || d.region.is_none() || d.f1 & f1::FLOATSHOE != 0 || d.f1 & burn.spared_by != 0 {
        return;
    }
    if d.f1 & 0x09 == 0 {
        let shift = rules.element_weakness.get(d.element as usize).map_or(0, |row| row[1]);
        let damage = burn.damage.wrapping_shl(shift as u32);
        let d = b.collision.get_mut(c);
        d.acc.element_damage[1] = d.acc.element_damage[1].wrapping_add(damage);
        d.acc.raw_element_damage[1] = d.acc.raw_element_damage[1].wrapping_add(damage);
        d.acc.mood_damage = d.acc.mood_damage.wrapping_add(burn.mood);
        d.hit_mod_final |= 3;
        d.hit_mod_by_side[0] |= 3;
        d.hit_mod_by_side[1] |= 3;
    }
    b.set_panel_type(p.x, p.y, PanelType::Normal);
    let (x, y) = crate::kinds::player::panel_coordinates(p.x, p.y);
    let at = crate::kinds::spark::jitter(b, 0xF, Vec3 { x, y, z: 0 });
    let spark = b.roles().spark(crate::content::SparkRole::PanelBurn);
    crate::kinds::spark::spawn(b, r, at, spark);
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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

/// `sub_801156A`: an object with HP (a thrown or placed thing, not a
/// navi) takes this tick's damage. It shows a guard spark if it blocked a
/// hit, totals its damage by element (`sub_800E3BE`, no holy-panel
/// halving) into `final_damage`, and loses that much HP unless `mode` is 1.
/// Returns -1 when its HP runs out (the rest is skipped), else it flashes
/// white while hit, with sound 0x85 (mode 0) or 0x6D (mode 2) and 0, or 1
/// for a hit in another mode; 0 when not hit.
pub fn take_damage(b: &mut Battle, r: ObjectRef, mode: u8) -> i32 {
    spawn_guard_spark(b, r);
    let c = b.objects.get(r).collision.expect("object with collision data");
    let acc = &mut b.collision.get_mut(c).acc;
    let total: u32 = acc.element_damage[..5].iter().map(|&d| d as u32).sum();
    acc.final_damage = total as u16;
    if mode != 1 {
        let o = b.objects.get_mut(r);
        let hp = (o.hp as i32).wrapping_sub(total as i32);
        o.hp = hp as u16;
        if hp <= 0 {
            return -1;
        }
    }
    b.objects.sprite_mut(r).look.white = total != 0;
    if total == 0 {
        return 0;
    }
    match mode {
        0 => b.sound(crate::content::SoundRole::Damage),
        2 => b.sound(crate::content::SoundRole::Hit),
        _ => return 1,
    }
    0
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
    b.sound(crate::content::SoundRole::Guard);
    let o = b.objects.get(r);
    let pos = Vec3 { z: o.pos.z.wrapping_add(0x10 << 16), ..o.pos };
    let pos = crate::kinds::spark::jitter(b, 0xF, pos);
    let spark = b.roles().spark(crate::content::SparkRole::Guard);
    crate::kinds::spark::spawn(b, r, pos, spark);
}

/// `object_updateSpriteTimestop`: like `update_sprite`, but it also steps
/// while dimmed and whatever the object's collision says.
pub fn update_sprite_while_dimmed(b: &mut Battle, r: ObjectRef) {
    if b.paused {
        return;
    }
    let o = b.objects.get(r);
    if o.flags & flags::ACTIVE == 0 {
        return;
    }
    step_sprite(b, r);
}

/// `object_updateSpritePaused`: load a newly requested animation and step
/// the sprite, paused or not, but not while dimmed. (It skips only
/// inactive objects: `NO_SPRITE_UPDATE` doesn't stop it.)
pub fn update_sprite_while_paused(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).flags & flags::ACTIVE == 0 || b.is_dimmed() {
        return;
    }
    let o = b.objects.get(r);
    let (anim, loaded) = (o.anim, o.anim_loaded);
    if anim != loaded {
        b.objects.sprite_mut(r).set_animation(anim, &b.content);
        b.objects.get_mut(r).anim_loaded = anim;
    }
    b.objects.sprite_mut(r).update(&b.content);
}

/// `sub_801BC24`: load a newly requested animation (without stepping it),
/// else step the sprite; not while paused, nor while dimmed unless the
/// object runs while dimmed (skipped for inactive or non-animating
/// objects).
pub fn load_or_step_sprite(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get(r);
    if b.paused || o.flags & flags::ACTIVE == 0 || o.flags & flags::NO_SPRITE_UPDATE != 0 {
        return;
    }
    if o.flags & flags::RUN_WHILE_DIMMED == 0 && b.is_dimmed() {
        return;
    }
    let (anim, loaded) = (o.anim, o.anim_loaded);
    if anim != loaded {
        b.objects.sprite_mut(r).set_animation(anim, &b.content);
        b.objects.get_mut(r).anim_loaded = anim;
        return;
    }
    b.objects.sprite_mut(r).update(&b.content);
}

/// `sub_801BCD0`: load a newly requested animation and step the sprite,
/// paused or not (skipped only for non-animating objects).
pub fn step_sprite(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get(r);
    if o.flags & flags::NO_SPRITE_UPDATE != 0 {
        return;
    }
    let (anim, loaded) = (o.anim, o.anim_loaded);
    if anim != loaded {
        b.objects.sprite_mut(r).set_animation(anim, &b.content);
        b.objects.get_mut(r).anim_loaded = anim;
    }
    b.objects.sprite_mut(r).update(&b.content);
}
