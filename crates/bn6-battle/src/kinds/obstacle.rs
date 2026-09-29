//! What field objects share: obstacles without actor data (rocks, cubes
//! and the like) taking hits, living out their timer, reacting to status
//! and chips through one dispatcher, and leaving the field.
//! See docs/engine/field-objects.md.
//!
//! An obstacle's update is, in the game's order: [`take_hits`], hit
//! sparks, [`tick_lifetime`], [`react`] (which runs the current
//! [`Action`]), the sprite step, and presenting its collision again.

use crate::battle::{Battle, battle_flags};
use crate::collision::{CollisionId, f1};
use crate::kinds::common::{self, Progress};
use crate::object::{Object, ObjectRef, Vec3, flags};

/// What an obstacle kind provides to the shared code.
pub trait Obstacle {
    /// The obstacle's shared state (kept in its behavior state).
    fn state(o: &mut Object) -> &mut State;
    /// [`Action::Appear`].
    fn appear(b: &mut Battle, r: ObjectRef);
    /// [`Action::Destroyed`].
    fn destroyed(b: &mut Battle, r: ObjectRef);
    /// [`Action::Idle`].
    fn idle(b: &mut Battle, r: ObjectRef);
}

/// State every obstacle keeps.
#[derive(Clone, Debug, Default)]
pub struct State {
    /// During time stop: where the obstacle is held (integer pixels) and
    /// how many more ticks it shakes after a hit.
    pub held_x: i16,
    pub held_z: i16,
    pub shake: u8,
    /// What to go back to after a push.
    pub resume: Option<Progress>,
    /// Step within a push.
    pub slide_step: u8,
}

/// Obstacle actions (the object's `action` byte indexes the game's
/// per-obstacle table; the entries marked shared are the same for all).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Coming onto the field.
    Appear = 0,
    /// Shared: go back to idle.
    ReturnToIdle = 1,
    /// Break apart or leave.
    Destroyed = 2,
    /// Shared hit reactions that exist only for objects with actor data.
    Flinch = 3,
    Paralyzed = 4,
    /// Shared: pushed or dragged along the field.
    Slide = 5,
    Frozen = 6,
    Bubbled = 7,
    /// Standing.
    Idle = 8,
}

impl Action {
    pub fn of(b: &Battle, r: ObjectRef) -> Action {
        const ALL: [Action; 9] = [
            Action::Appear,
            Action::ReturnToIdle,
            Action::Destroyed,
            Action::Flinch,
            Action::Paralyzed,
            Action::Slide,
            Action::Frozen,
            Action::Bubbled,
            Action::Idle,
        ];
        let a = b.objects.get(r).action;
        *ALL.get(a as usize).unwrap_or_else(|| panic!("obstacle action {a} does not exist"))
    }

    /// Switch to this action from its first phase.
    pub fn start(self, b: &mut Battle, r: ObjectRef) {
        common::set_action(b, r, self as u8);
    }
}

/// Collision `f2` bits obstacles react to.
pub mod f2 {
    /// Destroy at the next reaction.
    pub const DESTROY: u32 = 0x1;
    pub const UNK_4: u32 = 0x4;
    /// Pushed by a hit (hit modifier 0x40).
    pub const PUSHED: u32 = 0x100;
    /// Picked up to be thrown.
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

/// Battle flag: some chips track targets per side; obstacles leaving the
/// field update that tracking.
const TARGET_TRACKING: u16 = 0x40;

/// Hit modifier bit of a pushing hit.
const PUSHING_HIT: u8 = 0x40;

/// Collision types that break an obstacle outright when they touch it:
/// bodies and other obstacles (0x0C80_0000), and type 0x2.
const CRUSHING_HITS: u32 = 0x0C80_0002;

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

/// `sub_801AD9E`: once the fight is on, resolve this tick's hits and total
/// the damage. A pushing hit (hit modifier 0x40) starts a push and
/// forgets the damage.
pub fn take_hits(b: &mut Battle, r: ObjectRef) {
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
        clear_f2(b, r, f2::UNK_4);
        // `sub_801A6A6`.
        let acc = &mut b.collision.get_mut(c).acc;
        acc.final_damage = 0;
        acc.element_damage = [0; 6];
        acc.mood_damage = 0;
        acc.counter = 0;
        acc.drain_hits = 0;
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
    if b.is_time_stop() || b.paused {
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

/// `sub_801B394`: apply damage and removal requests, then either run a
/// status routine or the current action.
pub fn react<T: Obstacle>(b: &mut Battle, r: ObjectRef) {
    let c = collision(b, r);
    let damage = b.collision.get(c).acc.final_damage;
    // (Damage also flashes the obstacle white.)
    if damage != 0 {
        b.play_sound(crate::sound::SoundId(0x85));
    }
    let killed = damage != 0 && {
        crate::kinds::subtract_hp(b, r, damage);
        b.objects.get(r).hp == 0
    };
    let destroy = killed || {
        if b.collision.get(c).acc.hit_flags & CRUSHING_HITS != 0 {
            b.objects.get_mut(r).hp = 0;
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
            return thrown(b, r);
        } else if f1_of(b, r) & obstacle_f1::CARRIED != 0 {
            return thrown(b, r);
        } else if f2_of(b, r) & f2::ENCASED != 0 {
            b.objects.get_mut(r).prevent_anim = 0;
            return encased(b, r);
        } else if f1_of(b, r) & (obstacle_f1::ENCASED_ICE | obstacle_f1::ENCASED_BUBBLE) != 0 {
            return encased(b, r);
        } else if b.is_time_stop() {
            if Action::of(b, r) != Action::Appear {
                return hold_in_time_stop::<T>(b, r);
            }
        } else {
            b.objects.get_mut(r).prevent_anim = 0;
            if f2_of(b, r) & f2::PUSHED != 0 {
                clear_f2(b, r, f2::PUSHED);
                let now = common::progress(b, r);
                let o = b.objects.get_mut(r);
                let s = T::state(o);
                s.resume.get_or_insert(now);
                s.slide_step = 0;
                o.action = Action::Slide as u8;
                o.phase = 0;
            } else if f1_of(b, r) & f1::DRAG != 0 {
                b.objects.get_mut(r).action = Action::Slide as u8;
            } else {
                T::state(b.objects.get_mut(r)).slide_step = 0;
            }
        }
    }
    // (The color shader is reset here.)
    update_visibility(b, r);
    match Action::of(b, r) {
        Action::Appear => T::appear(b, r),
        Action::ReturnToIdle => return_to_idle(b, r),
        Action::Destroyed => T::destroyed(b, r),
        Action::Slide => slide(b, r),
        Action::Idle => T::idle(b, r),
        // `sub_80166AE`, `sub_8016B02`, `sub_8016B36`, `sub_8016B72` look up
        // per-actor-type routines through the object's actor data, which
        // obstacles don't have (the game reads through a null pointer).
        // Nothing sets these actions on an obstacle.
        a @ (Action::Flinch | Action::Paralyzed | Action::Frozen | Action::Bubbled) => {
            panic!("obstacle action {a:?} needs actor data, which obstacles don't have")
        }
    }
}

/// `sub_80181F6`: visible unless in time stop; hidden from a blinded
/// local player when on the other side.
fn update_visibility(b: &mut Battle, r: ObjectRef) {
    if !b.is_time_stop() {
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

/// `sub_801823C`: during time stop, stand still, shaking for 30 ticks
/// after each hit (a simulation RNG draw per shaking tick).
fn hold_in_time_stop<T: Obstacle>(b: &mut Battle, r: ObjectRef) {
    update_visibility(b, r);
    let o = b.objects.get_mut(r);
    if o.prevent_anim == 0 {
        let (x, z) = ((o.pos.x >> 16) as i16, (o.pos.z >> 16) as i16);
        let s = T::state(o);
        s.held_x = x;
        s.held_z = z;
        s.shake = 0;
        o.prevent_anim = 4;
    }
    let hit = b.collision.get(collision(b, r)).acc.final_damage != 0;
    let o = b.objects.get_mut(r);
    let s = T::state(o);
    if hit {
        s.shake = 0x1E;
    }
    let (x, z) = ((s.held_x as i32) << 16, (s.held_z as i32) << 16);
    if s.shake != 0 {
        s.shake -= 1;
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
pub fn return_to_idle(b: &mut Battle, r: ObjectRef) {
    Action::Idle.start(b, r);
}

/// [`Action::Slide`], `sub_8017E26`: pushed or dragged along the field.
pub fn slide(_b: &mut Battle, _r: ObjectRef) {
    panic!("pushed obstacles (sub_8017E26) are not implemented yet");
}

// ---- Leaving the field -----------------------------------------------------

/// `sub_802EF5C`: update per-side target tracking when an obstacle
/// leaves (only with battle flag 0x40, which netbattles don't set).
pub fn release_tracking(b: &mut Battle, _r: ObjectRef) {
    if b.round.flags & TARGET_TRACKING != 0 {
        panic!("battle flag 0x40 target tracking (sub_802EF74) is not implemented yet");
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
/// `f2` is the collision `f2` word as the caller read it.
pub fn blink_out(b: &mut Battle, r: ObjectRef, f2: u32) -> BlinkOut {
    if f2 & f2::VANISH == 0 {
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
/// `data::ABSORBED_SPRITES`) flies to the absorbing side's navi.
/// `palette` is the obstacle's sprite palette.
pub fn fly_to_absorber(b: &mut Battle, r: ObjectRef, kind: u8, palette: u8) {
    let side = (f2_of(b, r) & f2::ABSORBED_BY_1 != 0) as u8;
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
