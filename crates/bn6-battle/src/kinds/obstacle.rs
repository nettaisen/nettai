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
//! game's +0x30, +0x32, +0x19); a slide's step in `drag_step` (+0x0D), its
//! bounds in `slide_bounds` (+0x0C), its direction in `slide_dx`/`dy`
//! (+0x1C, +0x1D), the panels it has left in `phase_init` and its rest in
//! `phase` (+0x0B, +0x0A), and what to go back to after it in
//! `saved_state` (+0x5C).

use crate::battle::{Battle, battle_flags};
use crate::collision::{CollisionId, f1};
use crate::field::{self, PanelType, pflags};
use crate::kinds::common::{self, Progress};
use crate::object::{DragStep, ObjectRef, PanelPos, SlideBounds, StateWord, Vec3, flags};

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
    /// Shared: pushed along the field.
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
    /// `sub_8017E26`: slides up to six panels, not into bodies or other
    /// obstacles (and, pulled back, not into the puller's area).
    Slide,
    /// `sub_8017CC0`: knocked back as far as the hit says, onto free
    /// panels it reserves on the way.
    KnockedBack,
    /// `sub_80166AE`, `sub_8016B02`, `sub_8016B36`, `sub_8016B72`: the
    /// actors' hit reactions, which nothing starts on an obstacle.
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
    /// Picked up to be thrown by side 0 / side 1.
    pub const THROWN_BY_0: u32 = 0x400;
    pub const THROWN: u32 = 0xC00;
    /// Encased in ice (0x1000) or a bubble (0x2000).
    pub const ENCASED_IN_ICE: u32 = 0x1000;
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

/// What a touch by a body, another obstacle or a breaking hit does (the
/// dispatchers differ there).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crush {
    /// `sub_801B394`: the HP drops to 0.
    Breaks,
    /// `sub_801B4D4`: destroyed, the HP as it is.
    Destroys,
    /// `sub_801B610`: bodies don't break it; obstacles and breaking hits
    /// drop the HP to 0.
    SparesBodies,
}

/// When [`react`] holds the obstacle still while dimmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    /// `sub_801B394` and the others: once it has appeared (its appearing
    /// action runs on while dimmed).
    AfterAppearing,
    /// `sub_801B750`: always.
    Always,
}

/// What a pushing hit does in [`take_hits`] (the variants differ there).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Push {
    /// `sub_801AD9E`: a pushing hit (hit modifier 0x40) pushes it and its
    /// damage is forgotten.
    ForgetsDamage,
    /// `sub_801AD12`: pushed, and the damage still lands.
    KeepsDamage,
    /// `sub_801ADFA`: as `KeepsDamage`, and any hit from one side (but
    /// hits of type 0x1000) pushes it a panel away.
    AnyHit,
    /// `sub_801AD6A`: never pushed.
    Ignored,
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

/// Hit modifier bit of a pushing hit.
const PUSHING_HIT: u8 = 0x40;

/// Collision types that break an obstacle outright when they touch it:
/// bodies (0x0C00_0000), other obstacles (0x0080_0000), and breaking hits
/// (type 0x2).
const CRUSHING_HITS: u32 = 0x0C80_0002;
const CRUSHING_HITS_BUT_BODIES: u32 = 0x0080_0002;

/// What pushed it, by side: the side's attacks, objects and other bodies
/// (`sub_800F598`, `sub_801AE56`).
const PUSHERS: [u32; 2] = [0xA200_0000, 0x5100_0000];

/// Hits of this type don't push a [`Push::AnyHit`] obstacle.
const UNPUSHING_HIT: u32 = 0x1000;

/// The hit modifier a [`Push::AnyHit`] obstacle takes from any hit:
/// pushed (0x40) a panel away (0x20), flinching (0x01).
const ANY_HIT_PUSH: u8 = 0x61;

/// Panels a slide may enter: solid ones without other bodies or obstacles
/// on them (`sub_800F534`); a knockback's also nothing else on them and
/// no reservation (`sub_800F50C`).
const SLIDE_BLOCKERS: u32 = 0x0380_0000;
const KNOCKBACK_BLOCKERS: u32 = pflags::OCCUPIED;

/// A slide's speed, 16.16 a tick: 10 pixels across, 6 in depth.
const SLIDE_SPEED_X: i32 = 0xA_0000;
const SLIDE_SPEED_Y: i32 = 0x6_0000;
/// The panels a slide goes at most (`sub_8017E44`, whatever the hit says).
const SLIDE_PANELS: u8 = 6;
/// Ticks of rest after a slide that couldn't start, and after one that
/// went.
const BLOCKED_REST: u8 = 0x18;
const SLID_REST: u8 = 0x14;

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

fn clear_f1(b: &mut Battle, r: ObjectRef, bits: u32) {
    let c = collision(b, r);
    b.collision.get_mut(c).f1 &= !bits;
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

/// `sub_801AD9E` and its variants (`push`): the hit flash ends; once the
/// fight is on, resolve this tick's hits and total the damage. A pushing
/// hit (hit modifier 0x40) asks for a push, unless the obstacle is moving.
pub fn take_hits(b: &mut Battle, r: ObjectRef, push: Push) {
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
    if push == Push::AnyHit {
        push_on_any_hit(b, c);
    }
    let hit_mod = b.collision.get(c).hit_mod_final;
    // (f1 0x40 marks objects that can't be pushed.)
    if push != Push::Ignored && f1_of(b, r) & f1::MOVING == 0 && hit_mod & PUSHING_HIT != 0 {
        set_f2(b, r, f2::PUSHED);
        clear_f2(b, r, f2::FLINCH);
        if push == Push::ForgetsDamage {
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

/// `sub_801AE56`: a hit from one side only (and not of type 0x1000, nor
/// already a push) pushes it a panel away from that side.
fn push_on_any_hit(b: &mut Battle, c: CollisionId) {
    let d = b.collision.get(c);
    let hits = d.acc.hit_flags;
    if hits & UNPUSHING_HIT != 0 || d.hit_mod_final & PUSHING_HIT != 0 {
        return;
    }
    // byte_801AEA0, by (side 0 hit it) + 2 * (side 1 hit it): only one
    // side's hit pushes.
    let by = (hits & PUSHERS[0] != 0, hits & PUSHERS[1] != 0);
    if by.0 != by.1 {
        b.collision.get_mut(c).hit_mod_final |= ANY_HIT_PUSH;
    }
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

/// `sub_801B394` and its variants (`crush`, `hold`): apply damage and
/// removal requests, then either run a status routine (None) or leave the
/// current action for the kind's action table to run (its number).
/// (`sub_801B878`, LilBolr's, is `Crush::Destroys` or `Crush::Breaks` by
/// the kind's own state.)
pub fn react(b: &mut Battle, r: ObjectRef, crush: Crush, hold: Hold) -> Option<u8> {
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
        let crushing = if crush == Crush::SparesBodies { CRUSHING_HITS_BUT_BODIES } else { CRUSHING_HITS };
        if b.collision.get(c).acc.hit_flags & crushing != 0 {
            if crush != Crush::Destroys {
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
            if hold == Hold::Always || b.objects.get(r).action != Action::Appear as u8 {
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
                // (A byte store each: the phase's entry flag stays.)
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

/// Run a shared entry of an obstacle's action table. The actors' hit
/// reactions are an error: they look up per-actor-type routines through
/// the object's actor data, which obstacles don't have (the game would
/// read through a null pointer), and nothing sets those actions on an
/// obstacle (only [`react`] and the kinds' own actions set an obstacle's
/// action, and a slide only restores what [`react`] saved).
pub fn shared_action(b: &mut Battle, r: ObjectRef, a: SharedAction) -> Result<(), String> {
    match a {
        SharedAction::ReturnToIdle => return_to_idle(b, r),
        SharedAction::Slide => slide(b, r, Slide::Bounded),
        SharedAction::KnockedBack => slide(b, r, Slide::KnockedBack),
        SharedAction::Flinch | SharedAction::Paralyzed | SharedAction::Frozen | SharedAction::Bubbled => {
            let routine = match a {
                SharedAction::Flinch => "sub_80166AE",
                SharedAction::Paralyzed => "sub_8016B02",
                SharedAction::Frozen => "sub_8016B36",
                _ => "sub_8016B72",
            };
            return Err(format!("{routine} on an obstacle reads actor data it doesn't have (nothing starts it on one)"));
        }
    }
    Ok(())
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

/// The two pushes of the shared table's slot 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slide {
    /// `sub_8017E26`.
    Bounded,
    /// `sub_8017CC0`.
    KnockedBack,
}

/// [`Action::Slide`]: `sub_8017E26` (`Slide::Bounded`) or `sub_8017CC0`
/// (`Slide::KnockedBack`), by `drag_step`: start, slide, rest.
fn slide(b: &mut Battle, r: ObjectRef, kind: Slide) {
    match b.objects.get(r).drag_step {
        DragStep::Start => start_slide(b, r, kind),
        DragStep::Slide => step_slide(b, r, kind),
        DragStep::Recover => rest_after_slide(b, r),
    }
}

/// Which way a hit pushes an obstacle (`sub_800F598`).
struct PushVector {
    /// The pusher's side as a direction: +1 pushed by side 0, -1 by side 1,
    /// 0 by both or neither.
    pusher: i8,
    dx: i8,
    dy: i8,
    /// Panels to go (`sub_8017CC0` goes that far; `sub_8017E26` always
    /// six).
    panels: u8,
}

/// `sub_800F598`: the push from this window's hits and those kept while
/// dimmed (hit modifier bits 0x04..0x20 pick the vector, `byte_800F604`,
/// turned toward the pusher's facing). Nothing when both sides or neither
/// pushed it.
///
/// The collision's +0x54 word (the hit flags `object_presentCollisionData`
/// means to keep while dimmed) takes part too; by a bug it keeps the
/// caller's r1 instead, which is 0 outside a dimming and 0x10 for an
/// obstacle held still in one (`object_updateSprite` leaves it): no pusher
/// bits either way, so it's left out.
fn push_vector(b: &Battle, r: ObjectRef) -> PushVector {
    let d = b.collision.get(collision(b, r));
    let hits = d.acc.hit_flags;
    let pusher = match (hits & PUSHERS[0] != 0, hits & PUSHERS[1] != 0) {
        (true, false) => 1,
        (false, true) => -1,
        _ => return PushVector { pusher: 0, dx: 0, dy: 0, panels: 0 },
    };
    // byte_800F604: (dx, dy, panels) for hit modifier 0x04, 0x08, 0x10,
    // 0x20.
    const VECTORS: [(i8, i8, u8); 4] = [(-1, 0, 6), (1, 0, 6), (-1, 0, 1), (1, 0, 1)];
    let bits = d.hit_mod_final >> 2;
    let Some(i) = (0..4).find(|&i| bits & (1 << i) != 0) else {
        panic!("sub_800F598: a push without a direction reads its vector from the BIOS (address 4)");
    };
    let (dx, dy, panels) = VECTORS[i];
    PushVector { pusher, dx: dx * pusher, dy, panels }
}

/// The panel `(dx, dy)` from `p`, if it's on the field.
fn step_from(p: PanelPos, dx: i8, dy: i8) -> Option<PanelPos> {
    let (x, y) = (p.x as i32 + dx as i32, p.y as i32 + dy as i32);
    let (x, y) = (u8::try_from(x).ok()?, u8::try_from(y).ok()?);
    field::is_valid(x, y).then_some(PanelPos { x, y })
}

/// Whether a slide may enter `to` (`sub_800F534`, `sub_800F55C` by the
/// slide's bounds; `sub_800F50C` for a knockback).
fn can_enter(b: &Battle, r: ObjectRef, kind: Slide, to: Option<PanelPos>) -> bool {
    let Some(p) = to else { return false };
    let (require, forbid) = match (kind, b.objects.get(r).slide_bounds) {
        (Slide::KnockedBack, _) => (pflags::SOLID, KNOCKBACK_BLOCKERS),
        (Slide::Bounded, SlideBounds::Anywhere) => (pflags::SOLID, SLIDE_BLOCKERS),
        // byte_800F588.
        (Slide::Bounded, SlideBounds::Area(1)) => (pflags::SOLID | pflags::ALLIANCE_1, SLIDE_BLOCKERS),
        (Slide::Bounded, SlideBounds::Area(_)) => (pflags::SOLID, SLIDE_BLOCKERS | pflags::ALLIANCE_1),
    };
    b.field.check(p.x, p.y, require, forbid)
}

/// `sub_8017E44` / `sub_8017CE0`: onto the panel it was heading for, and
/// off toward the next one the push points to, or rest if it can't go.
fn start_slide(b: &mut Battle, r: ObjectRef, kind: Slide) {
    set_f1(b, r, f1::DRAG);
    let o = b.objects.get_mut(r);
    o.related[0] = None;
    o.panel = o.future_panel;
    common::set_coordinates_from_panels(b, r);
    b.update_collision_panels(r);
    clear_f1(b, r, f1::SLIDING | f1::MOVING);
    let fp = b.objects.get(r).future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    let v = push_vector(b, r);
    let o = b.objects.get_mut(r);
    o.slide_dx = v.dx as u8;
    o.slide_dy = v.dy as u8;
    let panels = match kind {
        Slide::Bounded => {
            // byte_8017F24, by (pushed by side 1 or neither) * 4 + (the
            // vector goes back toward the pusher) * 2 + (the panel is side
            // 1's): a pull stays out of the puller's area (whichever side
            // the panel is).
            let back = v.dx != v.pusher;
            o.slide_bounds = match (v.pusher == 1, back) {
                (true, true) => SlideBounds::Area(1),
                (false, true) => SlideBounds::Area(0),
                (_, false) => SlideBounds::Anywhere,
            };
            SLIDE_PANELS
        }
        Slide::KnockedBack => v.panels,
    };
    o.phase_init = panels;
    let target = step_from(o.panel, v.dx, v.dy);
    if panels != 0 && can_enter(b, r, kind, target) {
        let target = target.expect("an enterable panel is on the field");
        let o = b.objects.get_mut(r);
        o.vel.x = v.dx as i32 * SLIDE_SPEED_X;
        o.vel.y = v.dy as i32 * SLIDE_SPEED_Y;
        o.future_panel = target;
        if kind == Slide::KnockedBack {
            b.reserve_panel(r, target.x, target.y);
        }
        b.objects.get_mut(r).drag_step = DragStep::Slide;
        return;
    }
    let o = b.objects.get_mut(r);
    o.phase = BLOCKED_REST;
    o.drag_step = DragStep::Recover;
}

/// `sub_8017F38` / `sub_8017D64`: move toward the panel ahead; there, go
/// on to the next (a panel further on ice, unless aqua) or come to rest.
fn step_slide(b: &mut Battle, r: ObjectRef, kind: Slide) {
    let fp = b.objects.get(r).future_panel;
    let (tx, ty) = crate::kinds::player::panel_coordinates(fp.x, fp.y);
    let o = b.objects.get_mut(r);
    let old_x = o.pos.x;
    o.pos.x = o.pos.x.wrapping_add(o.vel.x);
    let mut arrived = crate::kinds::player::passed(o.pos.x, old_x, tx);
    if !arrived {
        let old_y = o.pos.y;
        o.pos.y = o.pos.y.wrapping_add(o.vel.y);
        arrived = crate::kinds::player::passed(o.pos.y, old_y, ty);
    }
    if !arrived {
        common::set_panels_from_coordinates(b, r);
        b.update_collision_panels(r);
        return;
    }
    b.unreserve_panel(r, fp.x, fp.y);
    let aqua = b.collision.get(collision(b, r)).element == 2;
    if !aqua && b.field.panel(fp.x, fp.y).is_some_and(|p| p.kind == PanelType::Ice) {
        let o = b.objects.get_mut(r);
        o.phase_init = o.phase_init.wrapping_add(1);
    }
    let o = b.objects.get_mut(r);
    let left = o.phase_init as i32 - 1;
    o.phase_init = left as u8;
    if left > 0 {
        let next = step_from(fp, o.slide_dx as i8, o.slide_dy as i8);
        if can_enter(b, r, kind, next) {
            let next = next.expect("an enterable panel is on the field");
            b.objects.get_mut(r).future_panel = next;
            if kind == Slide::KnockedBack {
                b.reserve_panel(r, next.x, next.y);
            }
            return;
        }
    }
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    common::set_coordinates_from_panels(b, r);
    b.update_collision_panels(r);
    let o = b.objects.get_mut(r);
    o.phase = SLID_REST;
    o.drag_step = DragStep::Recover;
}

/// `sub_8017FE6` / `sub_8017E0A`: rest, then back to what it was doing
/// before the push (the whole state word; nothing saved is state 0).
fn rest_after_slide(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    let left = o.phase as i32 - 1;
    o.phase = left as u8;
    if left >= 0 {
        return;
    }
    clear_f1(b, r, f1::DRAG);
    let o = b.objects.get_mut(r);
    let w = o.saved_state.take().unwrap_or_default();
    common::set_progress(b, r, Progress { state: w.state, action: w.action, phase: w.phase, phase_init: w.phase_init });
}

// ---- The wind registry -----------------------------------------------------

/// `sub_80E541A`: `o` is `side`'s wind. The wind there before is destroyed
/// at once (`sub_80E5410`: its state word goes to destroy, and its first
/// variable, a fan's record, to 0, which nothing reads after).
pub fn set_wind(b: &mut Battle, o: ObjectRef, side: u8, source: field::WindSource) {
    if let Some(old) = b.field.winds[side as usize].object {
        common::set_progress(b, old, Progress::DESTROY);
    }
    b.field.winds[side as usize] = field::Wind { object: Some(o), source };
}

/// `sub_80E544C`: `o` is no side's wind.
pub fn clear_wind(b: &mut Battle, o: ObjectRef) {
    for w in &mut b.field.winds {
        if w.object == Some(o) {
            *w = field::Wind::default();
        }
    }
}

// ---- Leaving the field -----------------------------------------------------

/// `sub_802EF5C`: an obstacle leaving the field hands on each side's
/// tracked target (`sub_802EF74`) in the battle flag 0x40 mode, which
/// netbattles don't use.
pub fn release_tracking(b: &mut Battle, r: ObjectRef) {
    if b.round.flags & battle_flags::PER_PLAYER_GAUGES == 0 {
        return;
    }
    for side in 0..2 {
        retarget(b, r, side);
    }
}

/// `sub_802EF74`: if the other side tracks `r`, it tracks the best of
/// `side`'s actors instead (none: no target).
fn retarget(b: &mut Battle, r: ObjectRef, side: u8) {
    if b.sides[(side ^ 1) as usize].tracked != Some(r) {
        return;
    }
    let mut best = None;
    for i in 0..4 {
        let Some(a) = b.round.alive_actors[side as usize][i] else { continue };
        best = Some(match best {
            None => a,
            Some(held) => closer_target(b, r, a, held),
        });
    }
    b.sides[(side ^ 1) as usize].tracked = best;
}

/// `sub_802F006`: which of `a` and `held` the navi on the other side of
/// `r` would rather track: the one nearest the column in front of it, then
/// the one nearest its row, then the lower row; `a` when it has no navi.
fn closer_target(b: &Battle, r: ObjectRef, a: ObjectRef, held: ObjectRef) -> ObjectRef {
    let side = b.objects.get(r).alliance ^ 1;
    let Some(navi) = b.player(side) else { return a };
    let n = b.objects.get(navi);
    let front = n.panel.x as i32 + common::facing(n.alliance, 0);
    let flip = if n.alliance != 0 { -1 } else { 1 };
    let (ax, hx) = ((b.objects.get(a).panel.x as i32 - front) * flip, (b.objects.get(held).panel.x as i32 - front) * flip);
    // (Compared unsigned.)
    if (ax as u32) < (hx as u32) {
        return a;
    }
    if (ax as u32) > (hx as u32) {
        return held;
    }
    let (ay, hy) = (b.objects.get(a).panel.y as i32, b.objects.get(held).panel.y as i32);
    let (da, dh) = ((ay - n.panel.y as i32).abs(), (hy - n.panel.y as i32).abs());
    if da < dh {
        return a;
    }
    if da > dh || ay >= hy {
        return held;
    }
    a
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
