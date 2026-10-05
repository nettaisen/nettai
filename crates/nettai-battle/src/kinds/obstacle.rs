//! The obstacle framework: what field objects share (rocks, cubes and the
//! like, obstacles without actor data) taking hits, living out their
//! timer, reacting to status and chips through one dispatcher, and leaving
//! the field. The kinds themselves are content (chips/rockcube/rock and
//! the others); they call these steps through the content API's `obstacle`
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

use nettai_content_api::RecordHandle;

use crate::battle::{Battle, battle_flags};
use crate::collision::{CollisionId, f1};
use crate::content::{CollisionRole, EffectRole, PushReading, SoundRole, SparkRole};
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
        let byte = action_byte(b, r, self as u8).unwrap_or_else(|e| panic!("{e}"));
        common::set_action(b, r, byte);
    }
}

/// Whether the game numbers its obstacles' action tables as EXE5's
/// (`effects.obstacle_actions`, the game's rules).
fn exe5_actions(b: &Battle) -> bool {
    b.game_rules().effects.obstacle_actions == crate::content::ObstacleActions::Exe5
}

/// The byte the game stores in obstacle `r` for action `a` of the
/// framework's numbering ([`Action`], EXE6's: the kind's own from 8). EXE5's
/// obstacles have no frozen or bubbled entries (6 and 7), so their own start
/// at 6.
pub fn action_byte(b: &Battle, r: ObjectRef, a: u8) -> Result<u8, String> {
    if !exe5_actions(b) {
        return Ok(a);
    }
    match a {
        0..=5 => Ok(a),
        6 | 7 => Err(format!("{}: EXE5's obstacles have no action {a} (frozen, bubbled)", b.kind_key(r))),
        _ => Ok(a - 2),
    }
}

/// Obstacle `r`'s action in the framework's numbering (EXE6's), from the
/// game's byte.
pub fn current_action(b: &Battle, r: ObjectRef) -> u8 {
    let a = b.objects.get(r).action;
    if exe5_actions(b) && a >= 6 { a + 2 } else { a }
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
    /// `sub_801B878` (LilBoiler's) while its ExtraVars+4 is set: such a
    /// touch is as any hit (while it is clear the routine is `Breaks`).
    Ignores,
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

// (A slide's speed is the obstacle's game's reactions section's
// `slide_speed`: EXE6's 10 pixels a tick across and 6 in depth, EXE5's 8 in
// depth, 0x08014894 and 0x08014730.)
/// The panels a slide goes at most (`sub_8017E44`, whatever the hit says).
const SLIDE_PANELS: u8 = 6;
/// Ticks of rest after a slide that couldn't start, and after one that
/// went.
const BLOCKED_REST: u8 = 0x18;
const SLID_REST: u8 = 0x14;

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

/// `object_clearCollisionRegion`: it covers no panel.
pub fn clear_region(b: &mut Battle, r: ObjectRef) {
    let c = collision(b, r);
    b.collision.get_mut(c).region = None;
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
        if !b.content.identity(o.identity).absorbable {
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
    // (EXE5's lava burns first: 0x08017A18 and its variants.)
    common::panel_burn(b, r);
    if push == Push::AnyHit {
        match b.game_rules().push_reading {
            PushReading::Exe6 => push_on_any_hit(b, c),
            PushReading::Exe5 => push_on_any_hit_by_flip(b, c),
        }
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
/// EXE5's (0x08017AD8, from its any-hit reaction 0x08017A78): the same by
/// the hitters' flips (`hit_flags_by_flip`, 0x08017B34): one side's hits
/// by unflipped hitters alone push it by the unflipped hitters' modifier
/// byte (`hit_mod_by_side[0]`), by flipped ones alone by the other; any
/// other mix, nothing.
fn push_on_any_hit_by_flip(b: &mut Battle, c: CollisionId) {
    let d = b.collision.get(c);
    if d.acc.hit_flags & UNPUSHING_HIT != 0 || d.hit_mod_final & PUSHING_HIT != 0 {
        return;
    }
    let [unflipped, flipped] = d.acc.hit_flags_by_flip;
    let i = (unflipped & PUSHERS[0] != 0) as u8
        | ((unflipped & PUSHERS[1] != 0) as u8) << 1
        | ((flipped & PUSHERS[0] != 0) as u8) << 2
        | ((flipped & PUSHERS[1] != 0) as u8) << 3;
    let byte = match i {
        1 | 2 => 0,
        4 | 8 => 1,
        _ => return,
    };
    let d = b.collision.get_mut(c);
    d.hit_mod_by_side[byte] |= ANY_HIT_PUSH;
    d.hit_mod_final |= ANY_HIT_PUSH;
}

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
        clear_region(b, r);
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
        clear_region(b, r);
        b.objects.get_mut(r).hp = 0;
        return;
    }
    if t <= 0xB4 && t & 2 != 0 {
        o.set_visible(false);
    }
}

/// `sub_801B394` and its variants (`crush`, `hold`): apply damage and
/// removal requests, then either run a status routine (None) or leave the
/// current action for the kind's action table to run (its number, in the
/// framework's numbering: [`current_action`]).
/// (`sub_801B878`, LilBolr's, is `Crush::Breaks` or, while it erupts,
/// `Crush::Ignores`, by the kind's own state.)
pub fn react(b: &mut Battle, r: ObjectRef, crush: Crush, hold: Hold) -> Option<u8> {
    let c = collision(b, r);
    let damage = b.collision.get(c).acc.final_damage;
    let killed = damage != 0 && {
        // sprite_forceWhitePalette
        b.objects.sprite_mut(r).look.white = true;
        b.sound(SoundRole::Damage);
        crate::kinds::subtract_hp(b, r, damage);
        b.objects.get(r).hp == 0
    };
    let destroy = killed || {
        let crushing = if crush == Crush::SparesBodies { CRUSHING_HITS_BUT_BODIES } else { CRUSHING_HITS };
        if crush != Crush::Ignores && b.collision.get(c).acc.hit_flags & crushing != 0 {
            if crush != Crush::Destroys {
                b.objects.get_mut(r).hp = 0;
            }
            true
        } else {
            soldier_step(b, r);
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
    Some(current_action(b, r))
}

/// Per side: EXE5's ColonelSoul army (docs/design/exe5-map.md §15.11). Armed
/// is BattleState+0x5C's bit 0x10 (side 0) or 0x20 (side 1), which
/// ColonelSoul's start sets (0x080CAC1E) and its end clears (0x080CAC30);
/// the words are the side's soldiers' damage words (0x02034000 + 8 × side,
/// 0x080CABF8: the sword soldier's, the gun soldier's), which they read as
/// they strike (0x080CAC06, 0x080CAC12), and which disarming leaves. While
/// a side is armed, an obstacle of a game whose rules have the step
/// (`effects.obstacle_soldiers`) turns into its soldier ([`soldier_step`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Soldiers {
    pub armed: bool,
    pub words: [u32; 2],
}

/// What EXE5's `object_getPanelParameters` leaves in r2: the address it
/// calls `_object_getPanelDataOffset` through (0x0800BD1C, Thumb), which
/// 0x080CAB02 leaves in turn when the sword's soldier answers; the
/// soldier's element byte comes of it (0x1D).
const PANEL_LOOKUP_REGISTER: u32 = 0x0800_BD1D;

/// EXE5's step in its four obstacle reactions (0x08018000, 0x08018168,
/// 0x080182D4 and 0x08018404, after the damage and the crushing hits,
/// for an obstacle they leave standing): outside the dimming and past its
/// first action, an obstacle where an armed side can use it (0x080CAB02,
/// [`soldier_call`]) turns into that side's soldier (0x080CAAE2) and its HP
/// and max HP go to 0 (a word store), so the reaction breaks it (whether a
/// soldier came or the pool was full).
fn soldier_step(b: &mut Battle, r: ObjectRef) {
    if !b.content.rules().effects.obstacle_soldiers
        || b.is_dimmed()
        || b.objects.get(r).action == Action::Appear as u8
    {
        return;
    }
    let PanelPos { x, y } = b.objects.get(r).panel;
    let Some((gun, side, left)) = soldier_call(b, x, y) else { return };
    // 0x080CAAE2: attack object #0x30 at the registers (the row, what the
    // search left, the side), Param1 the soldier; `sub_801155A` gives it
    // the panel, an element byte of what the search left, a damage word of
    // 0 (r6), the obstacle's side and flip and the obstacle as its first
    // related; then the side, and a flip of Param1 ^ 1 (the sword's
    // soldier faces back toward the side's own area).
    let kind = b.content.defs.roles().kind(crate::content::KindRole::ObstacleSoldier);
    let pos = Vec3 { x: y as i32, y: left as i32, z: side as i32 };
    if let Some(e) = crate::kinds::spawn(b, kind, nettai_content_api::SpawnAt::AfterCurrent, pos, [gun, 0, 0, 0]) {
        crate::behavior::set_state_field(b, e, "gun", nettai_content_api::Value::Int(gun as i64));
        let o = b.objects.get_mut(e);
        o.panel = PanelPos { x, y };
        o.element = left as u8;
        o.damage = 0;
        o.stamina = 0;
        o.related[0] = Some(r);
        o.alliance = side;
        o.flip = gun ^ 1;
    }
    let o = b.objects.get_mut(r);
    o.hp = 0;
    o.max_hp = 0;
}

/// 0x080CAB02: whether an obstacle on panel (x, y) stands where an armed
/// side can use it. On a solid panel of side A, the other side (`side`)
/// armed: a body of `side`'s enemy on one of the two panels on `side`'s
/// side of it (0x080CAB5A, stopping off the field) calls the sword's
/// soldier (0); else one anywhere ahead of it on the row, the way `side`
/// faces (0x080CABB0, `object_getFirstPanelInDirectionFiltered`), the
/// gun's (1). The soldier, the side, and what the search left in r2.
fn soldier_call(b: &Battle, x: u8, y: u8) -> Option<(u8, u8, u32)> {
    // (Off the field its flags word would be read from the BIOS, whose
    // protected reads have no bit 0x10: not solid.)
    let panel = b.field.panel(x, y)?;
    if panel.flags & pflags::SOLID == 0 {
        return None;
    }
    let side = panel.alliance ^ 1;
    if !b.obstacle_soldiers[side as usize & 1].armed {
        return None;
    }
    // The enemy's bodies (0x080CABA8 and 0x080CABF0 by side), and the way
    // the side faces (`object_getAllianceDirection`).
    let enemy = if side == 0 { pflags::BODY_SIDE1 } else { pflags::BODY_SIDE0 };
    let dir: i16 = if side == 0 { 1 } else { -1 };
    let at = |px: i16| if (0..=0xFF).contains(&px) { px as u8 } else { 0xFF };
    let mut px = x as i16;
    for _ in 0..2 {
        px -= dir;
        let f = b.field.flags(at(px), y);
        if f == 0 {
            break;
        }
        if f & enemy != 0 {
            return Some((0, side, PANEL_LOOKUP_REGISTER));
        }
    }
    let mut px = x as i16 + dir;
    loop {
        if b.field.check(at(px), y, enemy, 0) {
            return Some((1, side, enemy));
        }
        px += dir;
        if !field::is_valid(at(px), y) {
            return None;
        }
    }
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
/// player when on the other side (each console's rule, decided for both
/// viewers).
fn update_visibility(b: &mut Battle, r: ObjectRef) {
    if !b.is_dimmed() {
        b.objects.get_mut(r).set_visible(true);
    }
    let alliance = b.objects.get(r).alliance;
    // (Players always have collision data.)
    let hidden = [0u8, 1].map(|viewer| {
        viewer != alliance & 1
            && b.player(viewer).and_then(|p| b.objects.get(p).collision).is_some_and(|c| b.collision.get(c).f1 & f1::BLIND != 0)
    });
    b.hide_from(r, hidden);
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

/// How high a thrown obstacle is lifted, and how fast it flies.
const THROW_HEIGHT: i32 = 0x40_0000;
const THROW_RISE_TICKS: u8 = 0x20;
const THROW_SPEED: i32 = 0x8_0000;
/// The landing's hit (`sub_80C53A6`'s r4 = 0x06050001, r7 = 3): its own
/// panel, the thrown obstacle's spark and collision types (the roles
/// `sparks.thrown_obstacle`, `collision.thrown_obstacle` and
/// `collision.thrown_obstacle_target`), hit modifier 3.
const THROW_HIT_MOD: u8 = 3;

/// `sub_800F6AC`: ask `r` to be picked up by `side` and thrown at (x, y)
/// after shaking `shake` ticks, with the damage word: +0x1C, +0x1D and
/// +0x1E, the word, f2 0x400 (side 0) or 0x800 (side 1). EXE5's Poltergeist
/// makes it (0x080E8DD8; nothing in EXE6 does).
pub fn request_throw(b: &mut Battle, r: ObjectRef, side: u8, x: u8, y: u8, shake: u8, damage: u32) {
    let o = b.objects.get_mut(r);
    (o.slide_dx, o.slide_dy, o.slide_timer) = (x, y, shake);
    (o.damage, o.stamina) = (damage as u16, (damage >> 16) as u16);
    set_f2(b, r, if side == 0 { f2::THROWN_BY_0 } else { f2::THROWN & !f2::THROWN_BY_0 });
}

/// `sub_8018002`: picked up and thrown, the request `sub_800F6AC` makes
/// (EXE5's Poltergeist; nothing in EXE6): the thrower's side (`f2::THROWN`),
/// the target panel in `slide_dx`/`slide_dy` (+0x1C, +0x1D), the ticks it
/// shakes in `slide_timer` (+0x1E) and the damage word. It rises 64 px in
/// 32 ticks, shakes, flies onto the target panel at 8 px a tick and breaks
/// there with a hit. Steps on `prevent_anim`; the step counter is
/// `shake_timer` (+0x19) and the shake's origin `shake_origin_x`/`z`
/// (+0x30, +0x32), as the dimming hold's. See field-objects.md §4.5.
fn thrown(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).prevent_anim {
        // sub_801802C: taken by the thrower's side.
        0 => {
            let side = if f2_of(b, r) & f2::THROWN_BY_0 != 0 { 0 } else { 1 };
            b.objects.get_mut(r).alliance = side;
            clear_f2(b, r, f2::THROWN);
            set_f1(b, r, obstacle_f1::CARRIED);
            let o = b.objects.get_mut(r);
            o.vel.z = bios_div(THROW_HEIGHT.wrapping_sub(o.pos.z), THROW_RISE_TICKS as i32);
            let fp = o.future_panel;
            b.unreserve_panel(r, fp.x, fp.y);
            b.objects.get_mut(r).shake_timer = THROW_RISE_TICKS;
            b.sound(SoundRole::ObstacleLift);
            clear_region(b, r);
            b.objects.get_mut(r).prevent_anim = 4;
        }
        // sub_8018076: up.
        4 => {
            let o = b.objects.get_mut(r);
            o.pos.z = o.pos.z.wrapping_add(o.vel.z);
            o.shake_timer = o.shake_timer.wrapping_sub(1);
            if o.shake_timer == 0 {
                o.pos.z = THROW_HEIGHT;
                o.prevent_anim = 8;
            }
        }
        // sub_8018094: the shake's length and origin (whole pixels).
        8 => {
            let o = b.objects.get_mut(r);
            o.shake_timer = o.slide_timer;
            o.shake_origin_x = (o.pos.x >> 16) as i16;
            o.shake_origin_z = (o.pos.z >> 16) as i16;
            o.prevent_anim = 0xC;
        }
        // sub_80180A8: shake (a simulation RNG draw a tick), then aim.
        0xC => {
            let o = b.objects.get(r);
            let (x, z) = ((o.shake_origin_x as u16 as i32) << 16, (o.shake_origin_z as u16 as i32) << 16);
            let base = Vec3 { x, y: o.pos.y, z };
            let pos = crate::kinds::spark::jitter(b, 3, base);
            let o = b.objects.get_mut(r);
            o.pos = pos;
            o.shake_timer = o.shake_timer.wrapping_sub(1);
            if o.shake_timer != 0 {
                return;
            }
            // Only the whole pixels are restored.
            o.pos.x = (o.pos.x & 0xFFFF) | x;
            o.pos.z = (o.pos.z & 0xFFFF) | z;
            o.future_panel = PanelPos { x: o.slide_dx, y: o.slide_dy };
            let ticks = aim_throw(b, r);
            b.objects.get_mut(r).shake_timer = ticks;
            b.sound(SoundRole::ObstacleThrow);
            b.objects.get_mut(r).prevent_anim = 0x10;
        }
        // sub_80180EC: fly, land with a hit, break.
        0x10 => {
            let o = b.objects.get_mut(r);
            o.pos.x = o.pos.x.wrapping_add(o.vel.x);
            o.pos.y = o.pos.y.wrapping_add(o.vel.y);
            o.pos.z = o.pos.z.wrapping_sub(o.vel.z);
            o.shake_timer = o.shake_timer.wrapping_sub(1);
            if o.shake_timer != 0 {
                return;
            }
            o.panel = o.future_panel;
            common::set_coordinates_from_panels(b, r);
            let o = b.objects.get(r);
            let roles = b.roles();
            let spec = crate::kinds::hitbox::HitboxSpec {
                panel: o.panel,
                element: o.element,
                z: 0,
                region: b.anchor_region(),
                hit_effect: Some(roles.spark(SparkRole::ThrownObstacle)),
                target: roles.collision(CollisionRole::ThrownObstacleTarget),
                self_type: roles.collision(CollisionRole::ThrownObstacle),
                damage: o.damage,
                stamina: o.stamina,
                hit_mod: THROW_HIT_MOD,
                ..Default::default()
            };
            // sub_80C53A6: the hit resolves while dimmed too.
            if let Some(h) = crate::kinds::hitbox::spawn(b, r, &spec) {
                b.objects.get_mut(h).flags |= flags::RUN_WHILE_DIMMED;
            }
            b.objects.get_mut(r).hp = 0;
            Action::Destroyed.start(b, r);
        }
        // nullsub_57.
        0x14 => {}
        step => panic!("sub_8018002: step {step:#x} reads past off_8018014"),
    }
}

/// `sub_800F768`: aim a thrown obstacle at its target panel: 8 px a tick
/// along the ground toward the panel's center from its whole-pixel
/// position, rising at 64 px over the flight; the flight's ticks. (The
/// angle also goes to +0x0C, where nothing reads it.) The distance is
/// taken from the offsets shifted **logically** by 8 and squared in 32
/// bits: a negative offset wraps, harmlessly, as the offsets are whole
/// pixels (multiples of 1 << 16).
fn aim_throw(b: &mut Battle, r: ObjectRef) -> u8 {
    let o = b.objects.get(r);
    let (px, py) = crate::kinds::player::panel_coordinates(o.future_panel.x, o.future_panel.y);
    let dx = px.wrapping_sub(((o.pos.x as u32 >> 16) << 16) as i32);
    let dy = py.wrapping_sub(((o.pos.y as u32 >> 16) << 16) as i32);
    let angle = bios_arctan2(dx >> 16, dy >> 16) >> 8;
    let sine = &b.game_rules().sine;
    let (cos, sin) = (sine[angle as usize + 64] as i32, -(sine[angle as usize + 128] as i32));
    let (vx, vy) = (cos.wrapping_mul(THROW_SPEED) >> 8, sin.wrapping_mul(THROW_SPEED) >> 8);
    let (ax, ay) = (dx as u32 >> 8, dy as u32 >> 8);
    let distance = (bios_sqrt(ay.wrapping_mul(ay).wrapping_add(ax.wrapping_mul(ax))) << 8) as i32;
    let ticks = bios_div(distance, THROW_SPEED);
    let o = b.objects.get_mut(r);
    if ticks == 0 {
        o.vel = Vec3 { x: 0, y: 0, z: THROW_SPEED };
        return 8;
    }
    o.vel = Vec3 { x: vx, y: vy, z: bios_div(THROW_HEIGHT, ticks) };
    ticks as u8
}

/// `SWI_Div` (BIOS call 6): the quotient, truncated toward zero. (The BIOS
/// loops forever dividing by zero; no caller here does.)
fn bios_div(num: i32, den: i32) -> i32 {
    num.wrapping_div(den)
}

/// `SWI_Sqrt` (BIOS call 8): the whole square root of an unsigned word.
fn bios_sqrt(x: u32) -> u32 {
    (x as u64).isqrt() as u32
}

/// `SWI_ArcTan2` (BIOS call 10): the angle of (x, y), 0x10000 a turn, as the
/// BIOS computes it (its polynomial on the quotient of the smaller over the
/// larger coordinate, 1.0 = 1 << 14).
fn bios_arctan2(x: i32, y: i32) -> u32 {
    fn arctan(i: i32) -> i32 {
        let a = -(i.wrapping_mul(i) >> 14);
        let mut b = (0xA9i32.wrapping_mul(a) >> 14) + 0x390;
        for c in [0x91C, 0xFB6, 0x16AA, 0x2081, 0x3651, 0xA2F9] {
            b = (b.wrapping_mul(a) >> 14) + c;
        }
        (i.wrapping_mul(b) >> 16) as i16 as i32
    }
    let q = |n: i32, d: i32| n.wrapping_shl(14).wrapping_div(d);
    let r = if y == 0 {
        if x >= 0 { 0 } else { 0x8000 }
    } else if x == 0 {
        if y >= 0 { 0x4000 } else { 0xC000 }
    } else if y >= 0 {
        if x >= 0 && x >= y {
            arctan(q(y, x))
        } else if x < 0 && -x >= y {
            arctan(q(y, x)) + 0x8000
        } else {
            0x4000 - arctan(q(x, y))
        }
    } else if x <= 0 && -x > -y {
        arctan(q(y, x)) + 0x8000
    } else if x > 0 && x >= -y {
        arctan(q(y, x)) + 0x10000
    } else {
        0xC000 - arctan(q(x, y))
    };
    r as u32 & 0xFFFF
}

/// How long an encased obstacle shows before it is replaced.
const ENCASE_TICKS: u8 = 0x3C;
/// (The effect it flickers with is the role `effects.encased`.)

/// `sub_801813A`, by PreventAnim (`off_801814C`): encased in ice or a
/// bubble, it flickers for 60 ticks, then leaves the registry (and its
/// side's wind) for what content's role `hooks.encased` puts on its panel
/// (an ice block of its registry class, or the bubble), and goes.
fn encased(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).prevent_anim {
        // sub_8018154: ice or a bubble, remembered in f1.
        0 => {
            let ice = f2_of(b, r) & f2::ENCASED_IN_ICE != 0;
            set_f1(b, r, if ice { obstacle_f1::ENCASED_ICE } else { obstacle_f1::ENCASED_BUBBLE });
            clear_f2(b, r, f2::ENCASED);
            b.objects.get_mut(r).shake_timer = ENCASE_TICKS;
            let fp = b.objects.get(r).future_panel;
            b.unreserve_panel(r, fp.x, fp.y);
            clear_region(b, r);
            b.objects.get_mut(r).prevent_anim = 4;
        }
        // sub_8018186: flicker (hidden, with a new effect, two ticks of
        // every four); at the end, replaced.
        4 => {
            b.objects.get_mut(r).set_visible(true);
            if b.objects.get(r).shake_timer & 2 == 0 {
                b.objects.get_mut(r).set_visible(false);
                let pos = b.objects.get(r).pos;
                let look = b.roles().effect(EffectRole::Encased);
                crate::kinds::effect::spawn(b, pos, look, 0, 0, 0);
            }
            let o = b.objects.get_mut(r);
            o.shake_timer = o.shake_timer.wrapping_sub(1);
            if o.shake_timer != 0 {
                return;
            }
            // sub_800F806 (0xFF when not registered), sub_800F656,
            // sub_80E544C.
            let class = b.field.objects.class_of(r);
            unregister(b, r);
            clear_wind(b, r);
            let ice = f1_of(b, r) & obstacle_f1::ENCASED_ICE != 0;
            // (A game without the role has nothing for it to do: EXE5,
            // docs/design/exe5-map.md §15.3 item 4.)
            if let Some(hook) = b.roles().try_hook(crate::content::HookRole::Encased) {
                crate::behavior::call_hook(b, hook, nettai_content_api::HookCall::RoleEncased { obstacle: r, ice, class });
            }
            common::set_progress(b, r, Progress::DESTROY);
        }
        step => panic!("sub_801813A: step {step:#x} reads past off_801814C"),
    }
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
    if b.game_rules().push_reading == PushReading::Exe5 {
        return push_vector_exe5(b, r);
    }
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

/// EXE5's `sub_800F598` (0x0800D4B0): the pusher is side 0 when its hits
/// alone pushed it (nothing when both sides' did), else side 1 (neither's
/// too); the vector is the first of bits 2 to 5 of the unflipped hitters'
/// modifier byte (`hit_mod_by_side[0]`), else of the flipped ones' with
/// the direction reversed, from EXE5's table (0x0800D53B: EXE6's four rows
/// and a fifth of nothing when neither has a bit). (The +0x54 word takes
/// part as in EXE6's, left out the same.)
fn push_vector_exe5(b: &Battle, r: ObjectRef) -> PushVector {
    let d = b.collision.get(collision(b, r));
    let hits = d.acc.hit_flags;
    let pusher: i8 = if hits & PUSHERS[0] != 0 {
        if hits & PUSHERS[1] != 0 {
            return PushVector { pusher: 0, dx: 0, dy: 0, panels: 0 };
        }
        1
    } else {
        -1
    };
    const VECTORS: [(i8, i8, u8); 5] = [(-1, 0, 6), (1, 0, 6), (-1, 0, 1), (1, 0, 1), (0, 0, 0)];
    let first = |hm: u8| (0..4).find(|&i| (hm >> 2) & (1 << i) != 0);
    let [unflipped, flipped] = d.hit_mod_by_side;
    let (i, sign) = match first(unflipped) {
        Some(i) => (i, pusher),
        None => (first(flipped).unwrap_or(4), -pusher),
    };
    let (dx, dy, panels) = VECTORS[i];
    PushVector { pusher, dx: dx * sign, dy, panels }
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
    let (exe5, speed) = {
        let rules = b.game_rules();
        (rules.push_reading == PushReading::Exe5, rules.slide_speed)
    };
    let o = b.objects.get_mut(r);
    o.slide_dx = v.dx as u8;
    o.slide_dy = v.dy as u8;
    let panels = match kind {
        // EXE5's (0x08014894) keeps no bounds: it slides anywhere open.
        Slide::Bounded if exe5 => {
            o.slide_bounds = SlideBounds::Anywhere;
            SLIDE_PANELS
        }
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
        o.vel.x = v.dx as i32 * speed.x;
        o.vel.y = v.dy as i32 * speed.y;
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
/// tracked target (`sub_802EF74`) in the own-gauges mode, which
/// netbattles don't use.
pub fn release_tracking(b: &mut Battle, r: ObjectRef) {
    if b.round.flags & battle_flags::OWN_GAUGES == 0 {
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
    o.set_visible(true);
    if o.timer & 2 == 0 {
        o.set_visible(false);
    }
    let t = o.timer as i32 - 1;
    o.timer = t as u16;
    if t > 0 { BlinkOut::Blinking } else { BlinkOut::Done }
}

/// `sub_800F90E`: an absorbed obstacle of look `look` (content's
/// absorbed-look record; the original's obstacle kind, an index into
/// `byte_80E98C0`'s sprites) flies from `r` to the absorbing side's navi:
/// the content pack's absorbed obstacle (`sub_80E996E`, effect #0x87,
/// objects/absorbed-obstacle), spawned where `r` is with its look, the side
/// that absorbs it, its animation, sprite palette, hidden sprite parts and
/// facing, running neither while paused nor while dimmed.
pub fn fly_to_absorber(b: &mut Battle, r: ObjectRef, look: RecordHandle) {
    use nettai_content_api::{Registry, Value};
    let side = (f2_of(b, r) & f2::ABSORBED_BY_1 != 0) as u8;
    let o = b.objects.get(r);
    let (pos, anim, alliance, flip) = (o.pos, o.anim, o.alliance, o.flip);
    let sprite = b.objects.sprite(r).look;
    let kind = b.roles().kind(crate::content::KindRole::AbsorbedObstacle);
    let Some(e) = crate::kinds::spawn(b, kind, nettai_content_api::SpawnAt::AfterCurrent, pos, [0; 4]) else { return };
    for (field, v) in [
        ("look", Value::Def(Registry::Record, look.0)),
        ("side", Value::Int(side as i64)),
        ("anim", Value::Int(anim as i64)),
        ("palette", Value::Int(sprite.palette as i64)),
        ("hidden_parts", Value::Int(sprite.hidden_parts as i64)),
    ] {
        crate::behavior::set_state_field(b, e, field, v);
    }
    let o = b.objects.get_mut(e);
    o.alliance = alliance;
    o.flip = flip;
    o.flags &= !(flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED);
}

/// Done: hidden, and destroyed at the next update.
pub fn finish(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).set_visible(false);
    common::set_progress(b, r, Progress::DESTROY);
}
