//! The obstacle framework: what field objects share (rocks, cubes and the
//! like, obstacles without actor data) taking hits, living out their
//! timer, reacting to status and chips through one dispatcher, and leaving
//! the field. The kinds themselves are content (objects/rock and the
//! others); they call these steps through the content API's `obstacle`
//! service. See docs/engine/field-objects.md.
//!
//! An obstacle's update is, in the game's order: [`take_hits`], hit
//! sparks, [`tick_lifetime`], [`react`] (which leaves the kind's own
//! action table to run: its entries for [`Action::Appear`],
//! [`Action::Destroyed`] and [`Action::Idle`] are the kind's, the others
//! the [`shared_action`]s), the sprite step, and presenting its collision
//! again.
//!
//! The framework keeps its state in fields every object has: the dimming
//! hold in the shake fields (`shake_origin_x`/`z`, `shake_timer`, the
//! game's +0x30, +0x32, +0x19), the push's step in `drag_step` (+0x0D) and
//! what to go back to after it in `saved_state` (+0x5C).

use crate::battle::{Battle, battle_flags};
use crate::collision::{CollisionId, f1};
use crate::kinds::common::{self, Progress};
use crate::object::{DragStep, ObjectRef, StateWord, Vec3, flags};

/// Obstacle actions (the object's `action` byte indexes the kind's
/// table; the entries marked shared are the same for every kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// The kind's: coming onto the field.
    Appear = 0,
    /// Shared: go back to idle.
    ReturnToIdle = 1,
    /// The kind's: break apart or leave.
    Destroyed = 2,
    /// Shared hit reactions that exist only for objects with actor data.
    Flinch = 3,
    Paralyzed = 4,
    /// Shared: pushed or dragged along the field.
    Slide = 5,
    Frozen = 6,
    Bubbled = 7,
    /// The kind's: standing.
    Idle = 8,
}

impl Action {
    /// Switch to this action from its first phase.
    pub fn start(self, b: &mut Battle, r: ObjectRef) {
        common::set_action(b, r, self as u8);
    }
}

/// The shared entries of an obstacle's action table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedAction {
    /// `sub_80165B8`.
    ReturnToIdle,
    /// `sub_8017E26`.
    Slide,
    /// `sub_8017CC0` (the thrown obstacles' entry 5).
    Pushed,
    /// `sub_80166AE`, `sub_8016B02`, `sub_8016B36`, `sub_8016B72`.
    Flinch,
    Paralyzed,
    Frozen,
    Bubbled,
}

/// Collision `f2` bits obstacles react to.
pub mod f2 {
    /// Destroy at the next reaction.
    pub const DESTROY: u32 = 0x1;
    /// Flinch request (a hit with hit modifier bit 0x1); a push cancels
    /// it.
    pub const FLINCH: u32 = 0x4;
    /// Pushed by a hit (hit modifier 0x40).
    pub const PUSHED: u32 = 0x100;
    /// Picked up to be thrown (by side 0 / side 1).
    pub const THROWN: u32 = 0xC00;
    /// Encased in ice (0x1000) or a bubble (0x2000).
    pub const ENCASED: u32 = 0x3000;
    /// Removed by a chip.
    pub const REMOVED: u32 = 0x8000;
    /// Removed by blinking out.
    pub const VANISH: u32 = 0x4_0000;
    /// Absorbed by side 0 / side 1.
    pub const ABSORBED_BY_0: u32 = 0x10_0000;
    pub const ABSORBED_BY_1: u32 = 0x20_0000;
    pub const ABSORBED: u32 = ABSORBED_BY_0 | ABSORBED_BY_1;
}

/// Collision `f1` bits only obstacles use.
pub mod obstacle_f1 {
    /// Being carried to be thrown.
    pub const CARRIED: u32 = 0x0400_0000;
    /// Encased in ice / in a bubble.
    pub const ENCASED_ICE: u32 = 0x1000_0000;
    pub const ENCASED_BUBBLE: u32 = 0x2000_0000;
}

/// What a hit by a body or another obstacle does (the two dispatchers
/// differ only there).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crush {
    /// `sub_801B394`: the HP drops to 0.
    Breaks,
    /// `sub_801B4D4`: destroyed, the HP as it is.
    Destroys,
    /// `sub_801B878` while its object says so (a LilBoiler boiling
    /// over): nothing; only its HP or a chip's removal destroys it.
    Passes,
}

/// How an obstacle left the field, as its `Destroyed` action reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Removal {
    /// Broken (HP 0, the battle over, its time up, crushed).
    Broken,
    /// A chip removed it.
    Removed,
    /// A chip made it blink out.
    Vanished,
    /// Absorbed by `side`'s navi.
    Absorbed { side: u8 },
}

/// Battle flag: some chips track targets per side; obstacles leaving the
/// field update that tracking.
const TARGET_TRACKING: u16 = 0x40;

/// Hit modifier bit of a pushing hit.
const PUSHING_HIT: u8 = 0x40;

/// Collision types that break an obstacle outright when they touch it:
/// bodies and other obstacles (0x0C80_0000), and type 0x2.
const CRUSHING_HITS: u32 = 0x0C80_0002;

/// The damage sound.
const HIT_SOUND: u16 = 0x85;

fn collision(b: &Battle, r: ObjectRef) -> CollisionId {
    b.objects.get(r).collision.expect("obstacle without collision data")
}

fn f1_of(b: &Battle, r: ObjectRef) -> u32 {
    b.collision.get(collision(b, r)).f1
}

/// The collision `f2` word: requests other objects make of this one.
pub fn f2_of(b: &Battle, r: ObjectRef) -> u32 {
    b.collision.get(collision(b, r)).f2
}

fn set_f1(b: &mut Battle, r: ObjectRef, bits: u32) {
    let c = collision(b, r);
    b.collision.get_mut(c).f1 |= bits;
}

fn set_f2(b: &mut Battle, r: ObjectRef, bits: u32) {
    let c = collision(b, r);
    b.collision.get_mut(c).f2 |= bits;
}

fn clear_f2(b: &mut Battle, r: ObjectRef, bits: u32) {
    let c = collision(b, r);
    b.collision.get_mut(c).f2 &= !bits;
}

/// `object_setCollisionRegion` / `object_clearCollisionRegion` (0).
pub fn set_region(b: &mut Battle, r: ObjectRef, region: u8) {
    let c = collision(b, r);
    b.collision.get_mut(c).region = region;
}

// ---- The field-object registry ------------------------------------------

/// `setFieldBattleObject_800F614`: register a new obstacle for `side` in
/// `class`. A third class-0 obstacle (or a second class-1 one) evicts the
/// oldest, whose HP drops to 0 so it breaks at its next update.
pub fn register(b: &mut Battle, r: ObjectRef, side: u8, class: u8) {
    if let Some(evicted) = b.field.objects.register(r, side, class) {
        b.objects.get_mut(evicted).hp = 0;
    }
}

/// `sub_800F656`: forget `r` in the registry.
pub fn unregister(b: &mut Battle, r: ObjectRef) {
    b.field.objects.unregister(r);
}

// ---- Requests from chips ---------------------------------------------------

/// `sub_800F884`: a chip removes `obj`.
pub fn remove(b: &mut Battle, obj: ObjectRef) {
    if b.objects.get(obj).collision.is_some() {
        set_f2(b, obj, f2::REMOVED);
    }
}

/// `sub_800F898`: a chip makes `obj` blink out.
pub fn vanish(b: &mut Battle, obj: ObjectRef) {
    remove(b, obj);
    if b.objects.get(obj).collision.is_some() {
        set_f2(b, obj, f2::VANISH);
    }
}

/// `sub_800F8B0`: `absorber` pulls `obj` in; it flies to the navi of
/// `absorber`'s side as an absorbed-obstacle effect.
pub fn absorb(b: &mut Battle, obj: ObjectRef, absorber: ObjectRef) {
    remove(b, obj);
    if b.objects.get(obj).collision.is_some() {
        let side = b.objects.get(absorber).alliance;
        set_f2(b, obj, f2::ABSORBED_BY_0 << side);
    }
}

/// `sub_80EFD74`: pull in every registered field object (the obstacle
/// absorbing chip, player action 0x58), except name 0xDA and objects
/// already leaving.
pub fn absorb_all(b: &mut Battle, absorber: ObjectRef) {
    for i in 0..b.field.objects.slots.len() {
        let Some(obj) = b.field.objects.slots[i] else { continue };
        let o = b.objects.get(obj);
        if o.name_id == 0xDA {
            continue;
        }
        let Some(c) = o.collision else { continue };
        if b.collision.get(c).f2 & (f2::ABSORBED | f2::VANISH | f2::REMOVED) != 0 {
            continue;
        }
        absorb(b, obj, absorber);
    }
}

// ---- The per-tick steps ----------------------------------------------------

/// `sub_801AD9E`: the hit flash ends; once the fight is on, resolve this
/// tick's hits and total the damage. A pushing hit (hit modifier 0x40)
/// starts a push and forgets the damage.
pub fn take_hits(b: &mut Battle, r: ObjectRef) {
    take_hits_as(b, r, true);
}

/// `sub_801AD12`: [`take_hits`], but a push keeps the damage.
pub fn take_hits_keeping_damage(b: &mut Battle, r: ObjectRef) {
    take_hits_as(b, r, false);
}

fn take_hits_as(b: &mut Battle, r: ObjectRef, push_forgets_damage: bool) {
    // sprite_clearFinalPalette
    b.objects.sprite_mut(r).look.white = false;
    let c = collision(b, r);
    if b.round.flags & battle_flags::FIGHTING == 0 {
        return;
    }
    b.remove_collision(c);
    if b.is_battle_over() || f1_of(b, r) & f1::DEAD != 0 {
        return;
    }
    let hit_mod = b.collision.get(c).hit_mod_final;
    // (f1 0x40 marks objects that can't be pushed.)
    if f1_of(b, r) & f1::MOVING == 0 && hit_mod & PUSHING_HIT != 0 {
        set_f2(b, r, f2::PUSHED);
        clear_f2(b, r, f2::FLINCH);
        if push_forgets_damage {
            // `sub_801A6A6`.
            let acc = &mut b.collision.get_mut(c).acc;
            acc.final_damage = 0;
            acc.element_damage = [0; 6];
            acc.mood_damage = 0;
            acc.counter = 0;
            acc.drain_hits = 0;
        }
        b.objects.get_mut(r).slide_type = 1;
    }
    common::total_damage(b, r);
    common::spawn_guard_spark(b, r);
}

/// `sub_800F672`: the obstacle's lifetime. It breaks when the battle is
/// over or its timer runs out, and blinks for its last three seconds.
pub fn tick_lifetime(b: &mut Battle, r: ObjectRef) {
    if b.is_battle_over() {
        set_region(b, r, 0);
        b.objects.get_mut(r).hp = 0;
        return;
    }
    if b.is_dimmed() || b.paused {
        return;
    }
    let o = b.objects.get_mut(r);
    let t = (o.timer as u32).wrapping_sub(1);
    o.timer = t as u16;
    if t == 0 {
        set_region(b, r, 0);
        b.objects.get_mut(r).hp = 0;
        return;
    }
    if t <= 0xB4 && t & 2 != 0 {
        o.flags &= !flags::VISIBLE;
    }
}

/// `sub_801B394` / `sub_801B4D4`: apply damage and removal requests, then
/// either run a status routine (None) or leave the current action for the
/// kind's action table to run (its number).
pub fn react(b: &mut Battle, r: ObjectRef, crush: Crush) -> Option<u8> {
    react_as(b, r, crush, true)
}

/// `sub_801B750`: [`react`] (breaking), but while dimmed the obstacle
/// holds still even while appearing.
pub fn react_holding(b: &mut Battle, r: ObjectRef, crush: Crush) -> Option<u8> {
    react_as(b, r, crush, false)
}

fn react_as(b: &mut Battle, r: ObjectRef, crush: Crush, appears_while_dimmed: bool) -> Option<u8> {
    let c = collision(b, r);
    let damage = b.collision.get(c).acc.final_damage;
    let killed = damage != 0 && {
        // sprite_forceWhitePalette
        b.objects.sprite_mut(r).look.white = true;
        b.play_sound(crate::sound::SoundId(HIT_SOUND));
        crate::kinds::subtract_hp(b, r, damage);
        b.objects.get(r).hp == 0
    };
    let destroy = killed || {
        if b.collision.get(c).acc.hit_flags & CRUSHING_HITS != 0 && crush != Crush::Passes {
            if crush == Crush::Breaks {
                b.objects.get_mut(r).hp = 0;
            }
            true
        } else {
            f2_of(b, r) & f2::REMOVED != 0 || b.objects.get(r).hp == 0
        }
    };
    if destroy {
        set_f2(b, r, f2::DESTROY);
    }

    if f1_of(b, r) & f1::DEAD == 0 {
        if f2_of(b, r) & f2::DESTROY != 0 {
            clear_f2(b, r, f2::DESTROY);
            set_f1(b, r, f1::DEAD);
            Action::Destroyed.start(b, r);
        } else if f2_of(b, r) & f2::THROWN != 0 {
            b.objects.get_mut(r).prevent_anim = 0;
            thrown(b, r);
            return None;
        } else if f1_of(b, r) & obstacle_f1::CARRIED != 0 {
            thrown(b, r);
            return None;
        } else if f2_of(b, r) & f2::ENCASED != 0 {
            b.objects.get_mut(r).prevent_anim = 0;
            encased(b, r);
            return None;
        } else if f1_of(b, r) & (obstacle_f1::ENCASED_ICE | obstacle_f1::ENCASED_BUBBLE) != 0 {
            encased(b, r);
            return None;
        } else if b.is_dimmed() {
            if !appears_while_dimmed || b.objects.get(r).action != Action::Appear as u8 {
                hold_while_dimmed(b, r);
                return None;
            }
        } else {
            b.objects.get_mut(r).prevent_anim = 0;
            if f2_of(b, r) & f2::PUSHED != 0 {
                clear_f2(b, r, f2::PUSHED);
                let o = b.objects.get_mut(r);
                if o.saved_state.is_none() {
                    o.saved_state =
                        Some(StateWord { state: o.state, action: o.action, phase: o.phase, phase_init: o.phase_init });
                }
                o.action = Action::Slide as u8;
                o.phase = 0;
                o.drag_step = DragStep::Start;
            } else if f1_of(b, r) & f1::DRAG != 0 {
                b.objects.get_mut(r).action = Action::Slide as u8;
            } else {
                b.objects.get_mut(r).drag_step = DragStep::Start;
            }
        }
    }
    // sprite_zeroColorShader
    b.objects.sprite_mut(r).look.color_shader = 0;
    update_visibility(b, r);
    Some(b.objects.get(r).action)
}

/// Run a shared entry of an obstacle's action table.
pub fn shared_action(b: &mut Battle, r: ObjectRef, a: SharedAction) {
    match a {
        SharedAction::ReturnToIdle => return_to_idle(b, r),
        SharedAction::Slide => slide(b, r),
        SharedAction::Pushed => pushed(b, r),
        // `sub_80166AE`, `sub_8016B02`, `sub_8016B36`, `sub_8016B72` look up
        // per-actor-type routines through the object's actor data, which
        // obstacles don't have (the game reads through a null pointer).
        // Nothing sets these actions on an obstacle.
        a @ (SharedAction::Flinch | SharedAction::Paralyzed | SharedAction::Frozen | SharedAction::Bubbled) => {
            panic!("obstacle action {a:?} needs actor data, which obstacles don't have")
        }
    }
}

/// `sub_80181F6`: visible unless while dimmed; hidden from a blinded
/// local player when on the other side.
fn update_visibility(b: &mut Battle, r: ObjectRef) {
    if !b.is_dimmed() {
        b.objects.get_mut(r).flags |= flags::VISIBLE;
    }
    let alliance = b.objects.get(r).alliance;
    if b.round.local_side ^ alliance == 0 {
        return;
    }
    let Some(p) = b.player(alliance ^ 1) else { return };
    // (Players always have collision data.)
    let blind = b.objects.get(p).collision.is_some_and(|c| b.collision.get(c).f1 & f1::BLIND != 0);
    if blind {
        b.objects.get_mut(r).flags &= !flags::VISIBLE;
    }
}

/// `sub_801823C`: while dimmed, stand still, shaking for 30 ticks
/// after each hit (a simulation RNG draw per shaking tick).
fn hold_while_dimmed(b: &mut Battle, r: ObjectRef) {
    update_visibility(b, r);
    let o = b.objects.get_mut(r);
    if o.prevent_anim == 0 {
        o.shake_origin_x = (o.pos.x >> 16) as i16;
        o.shake_origin_z = (o.pos.z >> 16) as i16;
        o.shake_timer = 0;
        o.prevent_anim = 4;
    }
    let hit = b.collision.get(collision(b, r)).acc.final_damage != 0;
    let o = b.objects.get_mut(r);
    if hit {
        o.shake_timer = 0x1E;
    }
    let (x, z) = ((o.shake_origin_x as u16 as i32) << 16, (o.shake_origin_z as u16 as i32) << 16);
    if o.shake_timer != 0 {
        o.shake_timer -= 1;
        let base = Vec3 { x, y: o.pos.y, z };
        b.objects.get_mut(r).pos = crate::kinds::spark::jitter(b, 3, base);
    } else {
        // Only the integer parts are restored.
        o.pos.x = (o.pos.x & 0xFFFF) | x;
        o.pos.z = (o.pos.z & 0xFFFF) | z;
    }
}

/// `sub_8018002`: picked up and thrown at the enemy.
fn thrown(_b: &mut Battle, _r: ObjectRef) {
    panic!("thrown obstacles (sub_8018002) are not implemented yet");
}

/// `sub_801813A`: encased in ice or a bubble, then replaced.
fn encased(_b: &mut Battle, _r: ObjectRef) {
    panic!("encased obstacles (sub_801813A) are not implemented yet");
}

// ---- Shared actions -------------------------------------------------------

/// [`Action::ReturnToIdle`], `sub_80165B8`.
fn return_to_idle(b: &mut Battle, r: ObjectRef) {
    Action::Idle.start(b, r);
}

/// [`Action::Slide`], `sub_8017E26`: pushed or dragged along the field.
fn slide(_b: &mut Battle, _r: ObjectRef) {
    panic!("pushed obstacles (sub_8017E26) are not implemented yet");
}

/// [`SharedAction::Pushed`], `sub_8017CC0`: pushed along the field by a
/// hit, panel by panel (one more over ice, unless aqua) while the panels
/// ahead are free and solid, then a 21-tick pause, and back to what it
/// was doing. Its step is `drag_step`, the panels left `phase_init`, the
/// pause `phase`.
fn pushed(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).drag_step {
        DragStep::Start => push_start(b, r),
        DragStep::Slide => push_slide(b, r),
        DragStep::Recover => push_recover(b, r),
    }
}

/// The panels a pushed obstacle may slide onto (`sub_800F50C`): on the
/// field, solid-standing (flag 0x10) and free.
fn can_push_onto(b: &Battle, x: i32, y: i32) -> bool {
    const REQUIRE: u32 = 0x10;
    const FORBID: u32 = 0x0F88_0080;
    (1..=6).contains(&x) && (1..=3).contains(&y) && b.field.check(x as u8, y as u8, REQUIRE, FORBID)
}

/// `sub_800F598`: which way this tick's hit pushes the obstacle (dx, dy)
/// and how many panels: by the lowest of hit-modifier bits 2..5, away
/// from the side whose attack hit it; nothing if both sides' or neither's
/// did.
fn push_vector(b: &Battle, r: ObjectRef) -> (i8, i8, u8) {
    let c = b.collision.get(collision(b, r));
    // (The game ORs in CollisionData+0x54, which presenting outside a
    // dimming zeroes.)
    let flags = c.acc.hit_flags;
    if flags & 0xF300_0000 == 0 {
        return (0, 0, 0);
    }
    let dir = if flags & 0xA200_0000 == 0 {
        -1
    } else if flags & 0x5100_0000 == 0 {
        1
    } else {
        return (0, 0, 0);
    };
    let bits = c.hit_mod_final >> 2;
    let Some(i) = (0..4).find(|i| bits & (1 << i) != 0) else {
        panic!("sub_800F598: a push without a direction bit reads its vector from the BIOS");
    };
    let v = b.content.rules.obstacle_push_vectors[i];
    (v.dx.wrapping_mul(dir), v.dy, v.tiles)
}

/// `sub_8017CE0`: off the reserved panel, the push begins, or (nothing
/// to push toward) the pause.
fn push_start(b: &mut Battle, r: ObjectRef) {
    set_f1(b, r, f1::DRAG);
    let o = b.objects.get_mut(r);
    o.related[0] = None;
    o.panel = o.future_panel;
    common::set_coordinates_from_panels(b, r);
    b.update_collision_panels(r);
    let c = collision(b, r);
    b.collision.get_mut(c).f1 &= !(f1::SLIDING | f1::MOVING);
    let fp = b.objects.get(r).future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    let (dx, dy, tiles) = push_vector(b, r);
    let o = b.objects.get_mut(r);
    o.slide_dx = dx as u8;
    o.slide_dy = dy as u8;
    o.phase_init = tiles;
    let (x, y) = (o.panel.x as i32 + dx as i32, o.panel.y as i32 + dy as i32);
    if tiles != 0 && can_push_onto(b, x, y) {
        let o = b.objects.get_mut(r);
        o.vel.x = (dx as i32).wrapping_mul(0xA_0000);
        o.vel.y = (dy as i32).wrapping_mul(0x6_0000);
        o.future_panel = crate::object::PanelPos { x: x as u8, y: y as u8 };
        b.reserve_panel(r, x as u8, y as u8);
        b.objects.get_mut(r).drag_step = DragStep::Slide;
        return;
    }
    let o = b.objects.get_mut(r);
    o.phase = 0x18;
    o.drag_step = DragStep::Recover;
}

/// `sub_800E6E8`: moving from `from` to `to`, it passed `mark`.
fn passed(to: i32, from: i32, mark: i32) -> bool {
    if to <= from { mark > to && mark <= from } else { mark > from && mark <= to }
}

/// `sub_8017D64`: slide toward the reserved panel; there, on to the next
/// if any panels are left and it is free, else stop.
fn push_slide(b: &mut Battle, r: ObjectRef) {
    let fp = b.objects.get(r).future_panel;
    let (tx, ty) = crate::kinds::player::panel_coordinates(fp.x, fp.y);
    let o = b.objects.get_mut(r);
    let from = o.pos.x;
    o.pos.x = from.wrapping_add(o.vel.x);
    let mut arrived = passed(o.pos.x, from, tx);
    if !arrived {
        let from = o.pos.y;
        o.pos.y = from.wrapping_add(o.vel.y);
        arrived = passed(o.pos.y, from, ty);
    }
    if !arrived {
        common::set_panels_from_coordinates(b, r);
        b.update_collision_panels(r);
        return;
    }
    b.unreserve_panel(r, fp.x, fp.y);
    let aqua = b.collision.get(collision(b, r)).element == 2;
    let ice = b.field.panel(fp.x, fp.y).is_some_and(|p| p.kind == crate::field::PanelType::Ice);
    let o = b.objects.get_mut(r);
    if !aqua && ice {
        o.phase_init = o.phase_init.wrapping_add(1);
    }
    let left = o.phase_init as i32 - 1;
    o.phase_init = left as u8;
    if left > 0 {
        let (x, y) = (fp.x as i32 + o.slide_dx as i8 as i32, fp.y as i32 + o.slide_dy as i8 as i32);
        if can_push_onto(b, x, y) {
            b.objects.get_mut(r).future_panel = crate::object::PanelPos { x: x as u8, y: y as u8 };
            b.reserve_panel(r, x as u8, y as u8);
            return;
        }
    }
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    common::set_coordinates_from_panels(b, r);
    b.update_collision_panels(r);
    let o = b.objects.get_mut(r);
    o.phase = 0x14;
    o.drag_step = DragStep::Recover;
}

/// `sub_8017E0A`: the pause; then back to the state word saved when the
/// push came.
fn push_recover(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    let t = o.phase as i32 - 1;
    o.phase = t as u8;
    if t >= 0 {
        return;
    }
    let c = collision(b, r);
    b.collision.get_mut(c).f1 &= !f1::DRAG;
    let o = b.objects.get_mut(r);
    let w = o.saved_state.take().unwrap_or_default();
    (o.state, o.action, o.phase, o.phase_init) = (w.state, w.action, w.phase, w.phase_init);
}

// ---- Leaving the field -----------------------------------------------------

/// `sub_802EF5C`: update per-side target tracking when an obstacle
/// leaves (only with battle flag 0x40, which netbattles don't set).
pub fn release_tracking(b: &mut Battle, _r: ObjectRef) {
    if b.round.flags & TARGET_TRACKING != 0 {
        panic!("battle flag 0x40 target tracking (sub_802EF74) is not implemented yet");
    }
}

/// How the obstacle is leaving, from its `f2` word.
pub fn removal(b: &Battle, r: ObjectRef) -> Removal {
    let f2 = f2_of(b, r);
    if f2 & f2::REMOVED == 0 {
        Removal::Broken
    } else if f2 & f2::ABSORBED != 0 {
        Removal::Absorbed { side: (f2 & f2::ABSORBED_BY_1 != 0) as u8 }
    } else if f2 & f2::VANISH != 0 {
        Removal::Vanished
    } else {
        Removal::Removed
    }
}

/// How a removed obstacle's blink-out is going (`sub_800F8CE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlinkOut {
    /// Not blinking out: removed some other way.
    No,
    Blinking,
    Done,
}

/// `sub_800F8CE`: blink for 20 ticks when removed with `f2::VANISH`.
pub fn blink_out(b: &mut Battle, r: ObjectRef) -> BlinkOut {
    if f2_of(b, r) & f2::VANISH == 0 {
        return BlinkOut::No;
    }
    let o = b.objects.get_mut(r);
    if o.phase_init == 0 {
        o.phase_init = 1;
        o.timer = 0x14;
    }
    o.flags |= flags::VISIBLE;
    if o.timer & 2 == 0 {
        o.flags &= !flags::VISIBLE;
    }
    let t = o.timer as i32 - 1;
    o.timer = t as u16;
    if t > 0 { BlinkOut::Blinking } else { BlinkOut::Done }
}

/// `sub_800F90E`: an absorbed obstacle of kind `kind` (see
/// `ObjectData::absorbed_sprites`) flies to the absorbing side's navi,
/// with the obstacle's animation and sprite palette.
pub fn fly_to_absorber(b: &mut Battle, r: ObjectRef, kind: u8) {
    let side = (f2_of(b, r) & f2::ABSORBED_BY_1 != 0) as u8;
    let palette = b.objects.sprite(r).look.palette;
    let o = b.objects.get(r);
    let spec = crate::kinds::absorbed_obstacle::Spec { kind, side, anim: o.anim, palette };
    let pos = o.pos;
    crate::kinds::absorbed_obstacle::spawn(b, r, pos, spec);
}

/// Done: hidden, and destroyed at the next update.
pub fn finish(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).flags &= !flags::VISIBLE;
    common::set_progress(b, r, Progress::DESTROY);
}
