//! Action 0x10: step one panel (`sub_80EB04C`). See objects-and-player.md
//! §M6.
//!
//! The step reserves its destination at once, commits to it on the third
//! tick after, clears the moving state five ticks later, then waits out
//! the navi's move lag. A blocked step leaves for idle on the spot.

use super::super::{
    ai, ai_mut, clear_flag1, end_attack, exit_attack_state, flag1, set_attack, set_coordinates_from_panel, set_flag1,
    stats,
};
use super::ActionVars;
use crate::actor::{ActorType, status};
use crate::battle::Battle;
use crate::collision::f1;
use crate::object::{ObjectRef, PanelPos};

/// The action number.
pub const ACTION: u8 = 0x10;

/// How a step picks its destination (the game's move type byte).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MoveKind {
    /// One panel in the held direction.
    #[default]
    Input,
    /// The given direction, else the others in a fixed order (buffered
    /// auto-steps).
    Fallback,
    /// A given destination.
    Absolute,
    /// A step gone astray (the NaviCust processing bug).
    Astray,
}

/// The step's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    /// Direction code: 1 up, 2 down, 3 back, 4 forward.
    pub dir: u8,
    pub kind: MoveKind,
    /// Ticks to wait after arriving (the navi's move lag).
    pub end_lag: u16,
    /// Where the step goes.
    pub target: PanelPos,
    /// Ticks left in the current phase.
    pub timer: u16,
}

/// The phases, on the attack's `step`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Pick and reserve the destination (`sub_80EB088`).
    Start,
    /// Wait, then commit to the destination (`sub_80EB128`).
    Depart,
    /// Wait, then stop moving (`sub_80EB194`).
    Arrive,
    /// The move lag (`sub_80EB1C4`).
    Recover,
}

impl Phase {
    fn of(step: u8) -> Phase {
        match step {
            0x0 => Phase::Start,
            0x4 => Phase::Depart,
            0x8 => Phase::Arrive,
            0xC => Phase::Recover,
            s => panic!("move phase {s:#x} (sub_80EB1F8) is not implemented yet"),
        }
    }

    fn step(self) -> u8 {
        match self {
            Phase::Start => 0x0,
            Phase::Depart => 0x4,
            Phase::Arrive => 0x8,
            Phase::Recover => 0xC,
        }
    }
}

/// `sub_80116AE` / `sub_80116D8`: start a step toward `dir` (a direction
/// code); its first phase runs at once.
pub(in crate::kinds::player) fn start(b: &mut Battle, r: ObjectRef, dir: u8, end_lag: u16, kind: MoveKind) {
    ai_mut(b, r).attack.action = ActionVars::Move(Vars { dir, kind, end_lag, ..Vars::default() });
    // The game also clears the step's panel-trail argument here; only
    // absolute steps (`sub_80116F6`) set one.
    set_attack(b, r, ACTION, 4);
    update(b, r);
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut ai_mut(b, r).attack.action {
        ActionVars::Move(v) => v,
        v => panic!("a move without its state ({v:?})"),
    }
}

fn set_phase(b: &mut Battle, r: ObjectRef, p: Phase) {
    let a = &mut ai_mut(b, r).attack;
    a.step = p.step();
    a.step_init = 0;
}

/// `sub_80EB04C`: the phase, then note whether the held direction still
/// matches the step's (a marker nothing in the step reads).
pub fn update(b: &mut Battle, r: ObjectRef) {
    match Phase::of(ai(b, r).attack.step) {
        Phase::Start => begin(b, r),
        Phase::Depart => depart(b, r),
        Phase::Arrive => arrive(b, r),
        Phase::Recover => recover(b, r),
    }
    let dir = vars(b, r).dir;
    if ai(b, r).attack.marker == 0 && super::super::idle::held_direction(b, r) != dir {
        ai_mut(b, r).attack.marker = 1;
    }
}

/// `sub_80F02A2`: whether the step sets the navi's animations. A state
/// bit (0x8000) together with the attack parameter byte 1 left by the last
/// chip turns them off.
pub(super) fn animates(b: &Battle, r: ObjectRef) -> bool {
    let a = ai(b, r);
    !(a.attack.params[1] != 0 && a.status & 0x8000 != 0)
}

/// Leave for idle (keeping pending requests and the charge).
fn leave(b: &mut Battle, r: ObjectRef) {
    if animates(b, r) { exit_attack_state(b, r) } else { end_attack(b, r) }
}

fn set_animation(b: &mut Battle, r: ObjectRef, anim: u8) {
    if animates(b, r) {
        crate::kinds::common::set_animation(b, r, anim);
    }
}

/// `sub_80EB088`.
fn begin(b: &mut Battle, r: ObjectRef) {
    clear_flag1(b, r, f1::USING_ACTION);
    // object_canMove
    if flag1(b, r) & (f1::IMMOBILIZED | f1::SLIDING | f1::MOVING) != 0 {
        return leave(b, r);
    }
    ai_mut(b, r).attack.marker = 0;
    let Vars { dir, kind, .. } = *vars(b, r);
    let target = match kind {
        MoveKind::Input => step_target(b, r, dir),
        MoveKind::Fallback => panic!("fallback steps (sub_800F998) are not implemented yet"),
        MoveKind::Absolute => panic!("absolute steps (sub_80116F6) are not implemented yet"),
        MoveKind::Astray => panic!("astray steps (sub_800FA20) are not implemented yet"),
    };
    let Some(target) = target else { return leave(b, r) };
    vars(b, r).target = target;
    b.objects.get_mut(r).future_panel = target;
    b.reserve_panel(r, target.x, target.y);
    let here = b.objects.get(r).panel;
    ai_mut(b, r).status |= direction_bits(target, here);
    set_flag1(b, r, f1::MOVING);
    let side = b.objects.get(r).alliance;
    b.bump_side_stat(side, 4, 1);
    set_animation(b, r, 4);
    vars(b, r).timer = 3;
    set_phase(b, r, Phase::Depart);
}

/// `sub_800F964`: one panel toward `dir` (back and forward follow the
/// side), if the navi may step there; none while sliding.
pub(crate) fn step_target(b: &Battle, r: ObjectRef, dir: u8) -> Option<PanelPos> {
    if flag1(b, r) & f1::SLIDING != 0 {
        return None;
    }
    let o = b.objects.get(r);
    let (dx, dy): (i8, i8) = match dir {
        1 => (0, -1),
        2 => (0, 1),
        3 => (-1, 0),
        4 => (1, 0),
        _ => (0, 0),
    };
    // object_getAllianceDirection: +1 for the left side, -1 for the right.
    let forward = 1 - 2 * o.alliance as i8;
    let target = PanelPos { x: o.panel.x.wrapping_add((dx * forward) as u8), y: o.panel.y.wrapping_add(dy as u8) };
    b.can_step(r, target.x, target.y).then_some(target)
}

/// `sub_801BE04`: the state bits for a move from `from` to `to` (8 right,
/// 4 left, 2 down, 1 up).
fn direction_bits(to: PanelPos, from: PanelPos) -> u32 {
    let x = match to.x.cmp(&from.x) {
        std::cmp::Ordering::Greater => 8,
        std::cmp::Ordering::Less => 4,
        std::cmp::Ordering::Equal => 0,
    };
    let y = match to.y.cmp(&from.y) {
        std::cmp::Ordering::Greater => 2,
        std::cmp::Ordering::Less => 1,
        std::cmp::Ordering::Equal => 0,
    };
    x | y
}

/// `sub_80EB128`: on the third tick, move onto the destination.
fn depart(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).status &= !status::MOVE_DIRECTIONS;
    let v = vars(b, r);
    v.timer = v.timer.wrapping_sub(1);
    if v.timer != 0 {
        return;
    }
    let from = b.objects.get(r).panel;
    panel_trail(b, r, from);
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    let p = o.panel;
    b.unreserve_panel(r, p.x, p.y);
    set_coordinates_from_panel(b, r);
    b.update_collision_panels(r);
    // A panel-trail argument (absolute steps only) would convert panels
    // here in some column patterns.
    set_animation(b, r, 3);
    vars(b, r).timer = 5;
    set_phase(b, r, Phase::Arrive);
}

/// `sub_8013CC4`: the NaviCust panel-trail bug, which breaks or cracks
/// the panel a player steps off at random.
pub(super) fn panel_trail(b: &Battle, r: ObjectRef, _from: PanelPos) {
    if ai(b, r).actor_type == ActorType::Player && stats(b, r).bugs.panel_trail_level != 0 {
        panic!("the panel-trail bug (sub_8013CC4) is not implemented yet");
    }
}

/// `sub_80EB194`: five ticks later, stop moving.
fn arrive(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r);
    v.timer = v.timer.wrapping_sub(1);
    if v.timer != 0 {
        return;
    }
    clear_flag1(b, r, f1::MOVING);
    set_flag1(b, r, f1::MOVE_COMPLETE);
    let v = vars(b, r);
    v.timer = v.end_lag;
    set_animation(b, r, 0);
    set_phase(b, r, Phase::Recover);
}

/// `sub_80EB1C4`: wait out the move lag, then back to idle.
fn recover(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r);
    let t = v.timer as i32 - 1;
    v.timer = t as u16;
    if t > 0 {
        return;
    }
    let Vars { dir, kind, .. } = *v;
    let again = if kind != MoveKind::Fallback && auto_step(b, r) { dir } else { 0 };
    ai_mut(b, r).buffered_move = again;
    leave(b, r);
}

/// `sub_8013FAE`: the NaviCust auto-step bug repeats a step at random.
fn auto_step(b: &Battle, r: ObjectRef) -> bool {
    if stats(b, r).bugs.auto_step != 0 {
        panic!("the auto-step bug (sub_8013FAE) is not implemented yet");
    }
    false
}
