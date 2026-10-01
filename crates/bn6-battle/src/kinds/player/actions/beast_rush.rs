//! The Beast Out rush (`sub_80EAD9C`). In a Beast form, a chip with the
//! lock-on flag (and the charged claw, action 0x52) runs inside this
//! wrapper: the navi holds its panel, warps next to the lock-on marker's
//! target, runs the chip's action there, and warps back to its panel. An
//! A press during the rush queues the next chip, which then warps and
//! attacks in turn without going home first. See chips.md §2.11 and §4.6.

use crate::battle::Battle;
use crate::collision::f1;
use crate::content::PanelOffset;
use crate::field;
use crate::input::keys;
use crate::kinds::common::{facing, set_animation};
use crate::kinds::player::{
    ai, ai_mut, chip_use, clear_flag1, clear_flag2, end_attack, exit_attack_state, flag1, form_of, panel_coordinates,
    reset_attack_links, set_coordinates_from_panel, set_flag1, snap_to_future_panel,
};
use crate::kinds::{afterimage, lockon_marker};
use crate::object::{ObjectRef, PanelPos, Vec3};
use bn6_content_api::LockonHandle;

/// Ticks after the rush starts before an A press can queue the next chip.
const CHAIN_BLOCK_TICKS: u8 = 12;

/// Where the rush is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Hold the panel (`sub_80EADDC`).
    #[default]
    Hold,
    /// Wait, then warp next to the target (`sub_80EAE28`).
    Warp,
    /// Run the chip's action; chain or warp back (`sub_80EAF36`).
    Attack,
}

/// The rush's state (AIAttackVars+0x1E..+0x27), next to the chip action's
/// own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    pub phase: Phase,
    /// The phase's entry ran.
    pub started: bool,
    /// The panel the warp left.
    pub home: PanelPos,
    /// An A press queued the next chip.
    pub chain: bool,
    /// Ticks left before A presses queue.
    pub chain_block: u8,
    /// Ticks left before the warp.
    pub countdown: u16,
}

impl Vars {
    /// `sub_801011A`'s clear: back to the first phase.
    pub fn restart(&mut self) {
        self.phase = Phase::Hold;
        self.started = false;
    }
}

fn rush(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    &mut ai_mut(b, r).attack.rush
}

fn set_phase(b: &mut Battle, r: ObjectRef, phase: Phase) {
    let v = rush(b, r);
    v.phase = phase;
    v.started = false;
}

/// `sub_80EAD9C`: one tick of the rush, then the chain window: after it
/// opens, an A press queues the next chip.
pub(in crate::kinds::player) fn update(b: &mut Battle, r: ObjectRef) {
    match ai(b, r).attack.rush.phase {
        Phase::Hold => hold(b, r),
        Phase::Warp => warp(b, r),
        Phase::Attack => attack(b, r),
    }
    let pressed = ai(b, r).pad.pressed;
    let v = rush(b, r);
    if v.chain_block != 0 {
        v.chain_block -= 1;
    } else if pressed & keys::A != 0 {
        v.chain = true;
    }
}

/// `sub_80EADDC`: settle on the destination panel and reserve it; the
/// navi counts as moving.
fn hold(b: &mut Battle, r: ObjectRef) {
    clear_flag1(b, r, f1::USING_ACTION);
    snap_to_future_panel(b, r);
    clear_flag1(b, r, f1::SLIDING | f1::MOVING);
    // The slide request.
    clear_flag2(b, r, 0x10);
    let p = b.objects.get(r).panel;
    b.objects.get_mut(r).future_panel = p;
    b.reserve_panel(r, p.x, p.y);
    set_flag1(b, r, f1::MOVING);
    let v = rush(b, r);
    v.chain = false;
    v.chain_block = CHAIN_BLOCK_TICKS;
    set_phase(b, r, Phase::Warp);
}

/// `sub_80EAE28`: lock on (freeze the marker), wait 3 ticks, then warp to
/// the panel the chip's lock-on mode picks near the target, leaving two
/// afterimages when the navi moved.
fn warp(b: &mut Battle, r: ObjectRef) {
    if !rush(b, r).started {
        rush(b, r).started = true;
        // sub_80E1654 (a no-op without a marker).
        if let Some(m) = ai(b, r).lockon_marker {
            lockon_marker::freeze(b, m);
        }
        if super::movement::animates(b, r) {
            set_animation(b, r, 4);
        }
        rush(b, r).countdown = 3;
    }
    let v = rush(b, r);
    v.countdown = v.countdown.wrapping_sub(1);
    if v.countdown != 0 {
        return;
    }
    let here = b.objects.get(r).panel;
    super::movement::panel_trail(b, r, here);
    let mode = lockon_mode(b, r);
    let target = lockon_marker::panel_of(b, ai(b, r).lockon_marker);
    let dest = destination(b, r, target, mode).unwrap_or(here);
    rush(b, r).home = here;
    b.objects.get_mut(r).panel = dest;
    set_coordinates_from_panel(b, r);
    b.update_collision_panels(r);
    if let Some(t) = ai(b, r).attack.face_target
        && matches!(b.panel_pattern(), 0x31 | 0x23 | 0x33)
    {
        crate::kinds::player::face_toward(b, r, t);
    }
    if here != dest {
        let o = b.objects.get(r);
        let (x, y, z) = (o.pos.x, o.pos.y, o.pos.z);
        let (hx, hy) = panel_coordinates(here.x, here.y);
        let anim = afterimage_anim(o.name_id);
        afterimage::spawn(b, r, Vec3 { x: hx, y: hy, z }, anim, 12);
        let mid = Vec3 { x: hx.wrapping_add(x) >> 1, y: hy.wrapping_add(y) >> 1, z };
        afterimage::spawn(b, r, mid, anim, 20);
    }
    set_animation(b, r, 0);
    set_phase(b, r, Phase::Attack);
}

/// The afterimage's animation: 0xF, or 0 for the link navis.
fn afterimage_anim(name_id: u16) -> u8 {
    if (0x1A1..=0x1AB).contains(&name_id) { 0 } else { 0xF }
}

/// The lock-on mode: none (stay) while blind or confused outside Beast
/// Over; else the claw's own (`sub_80EAF1A`: the role `lockon.beast_claw`),
/// the charged sword's (`sub_80EAF26`: a table by the attack's variant in
/// the original; here the mode the charged sword's setup gave with its
/// slash, `AttackVars::rush_lockon`), and failing those, the chip's.
fn lockon_mode(b: &Battle, r: ObjectRef) -> Option<LockonHandle> {
    let beast_over = matches!(form_of(b, r).0, 0x17 | 0x18);
    if !beast_over && flag1(b, r) & (f1::BLIND | f1::CONFUSED) != 0 {
        return None;
    }
    use crate::content::{ActionRole, LockonRole};
    let attack = &ai(b, r).attack;
    let special = if crate::kinds::player::runs_role(b, r, ActionRole::BeastClaw) {
        Some(b.content.defs.roles.lockon(LockonRole::BeastClaw))
    } else if crate::kinds::player::runs_role(b, r, ActionRole::ChargedSword) {
        attack.rush_lockon
    } else {
        None
    };
    special.or(b.content.chip_field(attack.chip).lockon_mode)
}

/// `ho_8026554` as its callers outside the rush see it (the claw's and
/// the Beast lunge's setups): the panel, or (0, 0x7F) when no panel fits
/// (`sub_80265D0`'s registers then).
pub(crate) fn lockon_panel(b: &Battle, r: ObjectRef, target: PanelPos, mode: Option<LockonHandle>) -> PanelPos {
    destination(b, r, target, mode).unwrap_or(PanelPos { x: 0, y: 0x7F })
}

/// `ho_8026554`: the panel to attack `target` from in lock-on `mode`;
/// None to stay. No mode (the original's mode 0), or a target off the
/// field's playable panels, is the navi's own panel.
fn destination(b: &Battle, r: ObjectRef, target: PanelPos, mode: Option<LockonHandle>) -> Option<PanelPos> {
    use crate::content::LockonRule;
    let Some(mode) = mode.filter(|_| field::is_valid(target.x, target.y)) else {
        // sub_802661C: the navi's own panel.
        return Some(b.objects.get(r).panel);
    };
    let m = b.content.lockon(mode);
    let found = match m.rule {
        LockonRule::Stay => return Some(b.objects.get(r).panel),
        LockonRule::Row => search_row(b, r, target, m),
        LockonRule::Near => search_near(b, r, target, m),
    };
    if !m.prefers_middle_row {
        return found;
    }
    // sub_80265FE: the middle row of that column, if it will do. (Its
    // not-found test reads flags a `mov` just set, so it always runs.)
    let x = found.map_or(0, |p| p.x);
    if can_stand(b, r, x, 2) { Some(PanelPos { x, y: 2 }) } else { found }
}

/// `sub_8026622`: along the target's row from the navi's own column (the
/// same-row list when they share a row), then along the rows the mode's
/// shifts away from the target's, with the plain list.
fn search_row(b: &Battle, r: ObjectRef, target: PanelPos, m: &crate::content::LockonMode) -> Option<PanelPos> {
    let own = b.objects.get(r).panel;
    let first = if target.y == own.y { &m.same_row_offsets } else { &m.offsets };
    let x = own.x as i32;
    let rows = std::iter::once((target.y as i32, first)).chain(m.row_shifts.iter().map(|&s| (target.y as i32 + s as i32, &m.offsets)));
    rows.into_iter().find_map(|(y, offsets)| search_from(b, r, target, x, y, offsets, false))
}

/// `sub_80265D0` (with the column shifts) or `sub_8026450` /
/// `sub_80264A8` alone: next to the target, from the far-column list
/// when the target stands in the column farthest ahead.
fn search_near(b: &Battle, r: ObjectRef, target: PanelPos, m: &crate::content::LockonMode) -> Option<PanelPos> {
    let o = b.objects.get(r);
    let front = facing(o.alliance, o.flip);
    // sub_80266BA: column 6 facing right, 1 facing left.
    let far_column = if front > 0 { 6 } else { 1 };
    let offsets = match &m.far_column_offsets {
        Some(far) if target.x == far_column => far,
        _ => &m.offsets,
    };
    let shifts: &[i8] = if m.column_shifts { &b.content.rules.lockon.column_shifts } else { &[] };
    std::iter::once(0).chain(shifts.iter().copied()).find_map(|shift| {
        let x = target.x as i32 + front * shift as i32;
        search_from(b, r, target, x, target.y as i32, offsets, m.clear_path)
    })
}

/// `sub_8026450` (`sub_80264A8` with `clear_path`): the first panel of
/// `offsets` from (x, y) the navi can stand on, on the field's columns,
/// not past the target's column, and with a clear path to it if asked.
fn search_from(
    b: &Battle,
    r: ObjectRef,
    target: PanelPos,
    x: i32,
    y: i32,
    offsets: &[PanelOffset],
    clear_path: bool,
) -> Option<PanelPos> {
    let o = b.objects.get(r);
    let front = facing(o.alliance, o.flip);
    offsets.iter().find_map(|off| {
        let cx = x + front * off.dx as i32;
        if !(0..=6).contains(&cx) || cx * front > target.x as i32 * front {
            return None;
        }
        let cy = y + off.dy as i32;
        if !(0..=3).contains(&cy) {
            return None;
        }
        let p = PanelPos { x: cx as u8, y: cy as u8 };
        (can_stand(b, r, p.x, p.y) && (!clear_path || path_clear(b, r, p, target))).then_some(p)
    })
}

/// `sub_8026510`: every panel from `from` toward the front, up to the
/// target's column, meets the clear-path condition (a panel off the field
/// never does).
fn path_clear(b: &Battle, r: ObjectRef, from: PanelPos, target: PanelPos) -> bool {
    let o = b.objects.get(r);
    let front = facing(o.alliance, o.flip);
    let rule = b.content.rules.lockon.clear_path[o.alliance as usize & 1];
    let mut x = from.x as i32;
    loop {
        if !(0..=0xFF).contains(&x) || !b.field.meets(x as u8, from.y, rule) {
            return false;
        }
        x += front;
        if x == target.x as i32 {
            return true;
        }
    }
}

/// `sub_800E680`: the navi could stand on (x, y), whichever side owns it.
fn can_stand(b: &Battle, r: ObjectRef, x: u8, y: u8) -> bool {
    if !field::is_valid(x, y) {
        return false;
    }
    let o = b.objects.get(r);
    let floor_free = flag1(b, r) & f1::AIRSHOE != 0 || !b.field.is_solid(o.panel.x, o.panel.y);
    let rule = b.content.rules.panels.any_side_step.get(floor_free, o.alliance);
    b.field.meets(x, y, rule)
}

/// `sub_80EAF36`: run the chip's action; when it is back to idle, chain
/// the queued chip or warp home.
fn attack(b: &mut Battle, r: ObjectRef) {
    let action = crate::kinds::player::navi_action(b, r);
    if action.is_attack() {
        super::dispatch(b, r, action);
        if crate::kinds::player::navi_action(b, r) != crate::kinds::player::NaviAction::Idle {
            return;
        }
        if ai(b, r).attack.rush.chain && chip_use::chain_next_chip(b, r) {
            // sub_800FC7C
            let side = b.objects.get(r).alliance as usize;
            b.hands[side].advance();
            set_phase(b, r, Phase::Warp);
            rush(b, r).chain = false;
            // (It also writes 0xC to AIData+0x85, which no player code
            // reads.)
            if let Some(m) = ai(b, r).lockon_marker {
                lockon_marker::unfreeze(b, m);
            }
            return;
        }
    }
    reset_attack_links(b, r);
    snap_to_future_panel(b, r);
    clear_flag1(b, r, f1::MOVING);
    set_flag1(b, r, f1::MOVE_COMPLETE);
    if super::movement::animates(b, r) {
        exit_attack_state(b, r);
    } else {
        end_attack(b, r);
    }
}
