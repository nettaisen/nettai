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
use crate::field::PanelType;
use crate::object::{ObjectRef, PanelPos};

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
    /// Wait, then land on the destination with collision back on, and on to
    /// `Arrive` (`sub_80EB1F8`; nothing in the port starts a step here).
    Land,
}

impl Phase {
    fn of(step: u8) -> Phase {
        match step {
            0x0 => Phase::Start,
            0x4 => Phase::Depart,
            0x8 => Phase::Arrive,
            0xC => Phase::Recover,
            0x10 => Phase::Land,
            s => panic!("move phase {s:#x} reads past its table (off_80EB074)"),
        }
    }

    fn step(self) -> u8 {
        match self {
            Phase::Start => 0x0,
            Phase::Depart => 0x4,
            Phase::Arrive => 0x8,
            Phase::Recover => 0xC,
            Phase::Land => 0x10,
        }
    }
}

/// `sub_80116AE` / `sub_80116D8`: start a step toward `dir` (a direction
/// code); its first phase runs at once. The object the step faces in some
/// panel patterns (AIAttackVars+0x2C) is cleared.
pub(in crate::kinds::player) fn start(b: &mut Battle, r: ObjectRef, dir: u8, end_lag: u16, kind: MoveKind) {
    let target = match &ai(b, r).attack.action {
        ActionVars::Move(v) => v.target,
        _ => PanelPos::default(),
    };
    ai_mut(b, r).attack.action = ActionVars::Move(Vars { dir, kind, end_lag, target, ..Vars::default() });
    ai_mut(b, r).attack.face_target = None;
    set_attack(b, r, crate::kinds::player::EngineAction::Move, 4);
    update(b, r);
}

/// `sub_80116AE(5, end_lag, 2)` after setting AIAttackVars+0x16/+0x17: a
/// step straight to `target` (the berserk controller's), facing nothing;
/// with `face`, EXE5's `sub_80116F6` (the auto-battling navis' AI's), which keeps
/// the object to face. A target in column 0 means no step.
pub(crate) fn start_absolute_facing(b: &mut Battle, r: ObjectRef, target: PanelPos, end_lag: u16, kind: MoveKind, face: Option<ObjectRef>) {
    let vars = Vars { dir: ABSOLUTE_DIRECTION, kind, end_lag, target, ..Vars::default() };
    ai_mut(b, r).attack.action = ActionVars::Move(vars);
    ai_mut(b, r).attack.face_target = face;
    set_attack(b, r, crate::kinds::player::EngineAction::Move, 4);
    update(b, r);
}

/// The direction code an absolute step carries (none of the four).
const ABSOLUTE_DIRECTION: u8 = 5;

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
        Phase::Land => land(b, r),
    }
    let dir = vars(b, r).dir;
    if ai(b, r).attack.marker == 0 && super::super::idle::held_direction(b, r) != dir {
        ai_mut(b, r).attack.marker = 1;
    }
}

/// `sub_80F02A2` (EXE5's 0x080F01CA): whether the step sets the navi's
/// animations: not while the navi is in the air (the state bit 0x8000,
/// `status::HOVERING`), where it keeps the animation it has and its ends
/// leave it. The original also tests a byte of the attack (+0x0D), which
/// EXE5's move starts set for AI index 2 alone (0x0800F234, 0x0802D544):
/// GyroMan, whose tick is the only thing that sets the bit, so the bit
/// says it all. (EXE6 keeps the test and sets neither: its steps always
/// animate.)
pub(super) fn animates(b: &Battle, r: ObjectRef) -> bool {
    ai(b, r).status & status::HOVERING == 0
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
    // object_canMove (EXE4's, 0x0800AD2A: nor trapped by its panel)
    if flag1(b, r) & (f1::IMMOBILIZED | f1::SLIDING | f1::MOVING) != 0 || b.trapped(r) {
        return leave(b, r);
    }
    ai_mut(b, r).attack.marker = 0;
    let Vars { dir, kind, .. } = *vars(b, r);
    let target = match kind {
        MoveKind::Input => step_target(b, r, dir),
        MoveKind::Fallback => fallback_target(b, r, dir),
        // The given destination, unchecked (column 0: none).
        MoveKind::Absolute => {
            let t = vars(b, r).target;
            (t.x != 0).then_some(t)
        }
        MoveKind::Astray => astray_target(b, r, dir),
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

/// `sub_800F9DE` (`byte_800FA14`): the panel offset of direction code
/// `dir` for side `alliance` (back and forward follow the side). Codes 0
/// and 5 go nowhere.
fn direction_offset(dir: u8, alliance: u8) -> (i8, i8) {
    let (dx, dy): (i8, i8) = match dir {
        0 | 5 => (0, 0),
        1 => (0, -1),
        2 => (0, 1),
        3 => (-1, 0),
        4 => (1, 0),
        _ => panic!("direction {dir:#x} reads past its offsets (sub_800F9DE)"),
    };
    // object_getAllianceDirection: +1 for the left side, -1 for the right.
    let forward = 1 - 2 * alliance as i8;
    (dx * forward, dy)
}

fn offset(p: PanelPos, (dx, dy): (i8, i8)) -> PanelPos {
    PanelPos { x: p.x.wrapping_add(dx as u8), y: p.y.wrapping_add(dy as u8) }
}

/// `sub_800F964`: one panel toward `dir`, if the navi may step there; none
/// while sliding.
pub(crate) fn step_target(b: &Battle, r: ObjectRef, dir: u8) -> Option<PanelPos> {
    if flag1(b, r) & f1::SLIDING != 0 {
        return None;
    }
    let o = b.objects.get(r);
    let target = offset(o.panel, direction_offset(dir, o.alliance));
    b.can_step(r, target.x, target.y).then_some(target)
}

/// `byte_800FA00`: the directions a buffered step tries, in order, by the
/// direction it was buffered with (the fifth row is the start of the
/// offsets table after it: an absolute step's code 5).
const FALLBACK_ORDERS: [[u8; 4]; 6] =
    [[0, 0, 0, 0], [1, 3, 2, 4], [2, 4, 1, 3], [3, 2, 4, 1], [4, 1, 3, 2], [0, 0, 0, 0xFF]];

/// `sub_800F998`: a buffered step: toward `dir` if the navi may step there,
/// else the other directions in `FALLBACK_ORDERS`' order; none while
/// sliding.
fn fallback_target(b: &Battle, r: ObjectRef, dir: u8) -> Option<PanelPos> {
    if flag1(b, r) & f1::SLIDING != 0 {
        return None;
    }
    let order = *FALLBACK_ORDERS
        .get(dir as usize)
        .unwrap_or_else(|| panic!("a buffered step toward {dir:#x} reads past its orders (sub_800F998)"));
    let o = b.objects.get(r);
    order.into_iter().map(|d| offset(o.panel, direction_offset(d, o.alliance))).find(|t| b.can_step(r, t.x, t.y))
}

/// `sub_800FA20`: a step gone astray (the NaviCust processing bug): as far
/// as it goes toward `dir`, to the farthest panel along the row or column
/// (through ones in between that don't qualify) that a dash may land on
/// (`sub_8010368`: `PanelRules::dash_step`, by AirShoes and side); none if
/// that is where the navi stands.
fn astray_target(b: &Battle, r: ObjectRef, dir: u8) -> Option<PanelPos> {
    let o = b.objects.get(r);
    let (dx, dy) = direction_offset(dir, o.alliance);
    if (dx, dy) == (0, 0) {
        panic!("an astray step toward {dir:#x} walks in place forever (sub_800D15A)");
    }
    let airshoes = flag1(b, r) & f1::AIRSHOE != 0;
    let cond = b.game_rules().panels.dash_step.get(airshoes, o.alliance);
    // sub_800D120 along the row, sub_800D15A along the column.
    let mut p = o.panel;
    let mut farthest = o.panel;
    loop {
        if b.field.meets(p.x, p.y, cond) {
            farthest = p;
        }
        p = offset(p, (dx, dy));
        if !crate::field::is_valid(p.x, p.y) {
            break;
        }
    }
    (farthest != o.panel).then_some(farthest)
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
    if let Some(t) = ai(b, r).attack.face_target
        && matches!(b.panel_pattern(), 0x31 | 0x23 | 0x33)
    {
        crate::kinds::player::face_toward(b, r, t);
    }
    set_animation(b, r, 3);
    vars(b, r).timer = 5;
    set_phase(b, r, Phase::Arrive);
}

/// `sub_8013CC4`: the NaviCust panel-trail bugs and programs (stats 0x12,
/// 0x13): at a chance of level in 8 (EXE4's: every step, its kind not
/// 0xFF; the rules' `effects.panel_trail`), the panel a player steps off
/// (unless it is missing or broken) breaks (kind 1), cracks (3, by chance
/// alone) or turns to the kind's panel type, with the type's trail sound
/// (`byte_8013D44`, the panel rules' `trail_sound`) when the type changes.
pub(crate) fn panel_trail(b: &mut Battle, r: ObjectRef, from: PanelPos) {
    use crate::content::PanelTrail;
    if ai(b, r).actor_type != ActorType::Player {
        return;
    }
    let rule = b.game_rules().effects.panel_trail;
    let kind = stats(b, r).bugs.panel_trail_kind;
    match rule {
        PanelTrail::ByChance => {
            let level = stats(b, r).bugs.panel_trail_level;
            if level == 0 {
                return;
            }
            if (b.rng.next_positive() & 7) as i32 > level as i32 - 1 {
                return;
            }
        }
        PanelTrail::Always if kind == 0xFF => return,
        PanelTrail::Always => {}
    }
    let Some(panel) = b.field.panel(from.x, from.y) else {
        panic!("the panel trail reads the type of panel {from:?}, off the field (sub_8013CC4)");
    };
    let old = panel.kind;
    if matches!(old, PanelType::Missing | PanelType::Broken) {
        return;
    }
    match kind {
        1 => {
            b.break_panel(from.x, from.y);
        }
        3 if rule == PanelTrail::ByChance => {
            b.crack_panel(from.x, from.y);
        }
        _ => {
            // (By the game's panel numbers: EXE5's own, its panels section's.)
            let Some(t) = b.game_rules().panels.numbered(kind) else {
                panic!("panel-trail kind {kind:#x} is past the panel types (sub_8013CC4)");
            };
            b.set_panel_type(from.x, from.y, t);
            if t != old
                && let Some(sound) = b.game_rules().panels.types[t as usize].trail_sound
            {
                b.play_sound(sound);
            }
        }
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

/// `sub_8013FAE`: the NaviCust auto-step bug (stat 0x11) repeats a step at
/// a chance of its level in 8.
fn auto_step(b: &mut Battle, r: ObjectRef) -> bool {
    let level = stats(b, r).bugs.auto_step;
    if level == 0 {
        return false;
    }
    (b.rng.next() & 7) as i32 <= level as i32 - 1
}

/// `sub_80EB1F8`: after the timer, land on the destination with collision
/// back on (region 1), then five ticks to `Arrive`.
fn land(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r);
    let t = v.timer as i32 - 1;
    v.timer = t as u16;
    if t > 0 {
        return;
    }
    super::super::coll_mut(b, r).region = b.anchor_region();
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    let p = o.panel;
    b.unreserve_panel(r, p.x, p.y);
    set_coordinates_from_panel(b, r);
    b.update_collision_panels(r);
    set_animation(b, r, 3);
    vars(b, r).timer = 5;
    set_phase(b, r, Phase::Arrive);
}
