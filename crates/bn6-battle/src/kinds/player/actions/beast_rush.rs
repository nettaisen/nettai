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
    ai, ai_mut, chip_use, clear_flag1, clear_flag2, end_attack, exit_attack_state, flag1, panel_coordinates,
    reset_attack_links, set_coordinates_from_panel, set_flag1, snap_to_future_panel, stats,
};
use crate::kinds::{afterimage, lockon_marker};
use crate::object::{ObjectRef, PanelPos, Vec3};

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
    let marker = ai(b, r).lockon_marker.expect("a Beast Out rush without a lock-on marker reads through a null pointer");
    let target = lockon_marker::panel(b, marker);
    let dest = destination(b, r, target, mode).unwrap_or(here);
    rush(b, r).home = here;
    b.objects.get_mut(r).panel = dest;
    set_coordinates_from_panel(b, r);
    b.update_collision_panels(r);
    if matches!(b.setup.settings.panel_pattern, 0x31 | 0x23 | 0x33) {
        // sub_800F2FC: face the attack's target object (AIAttackVars+0x2C).
        panic!("facing the rush target on this panel pattern (sub_800F2FC) is not implemented yet");
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
/// Over; 0xC for the claw (`sub_80EAF1A`); else the chip's.
fn lockon_mode(b: &Battle, r: ObjectRef) -> u8 {
    let beast_over = matches!(stats(b, r).form.0, 0x17 | 0x18);
    if !beast_over && flag1(b, r) & (f1::BLIND | f1::CONFUSED) != 0 {
        return 0;
    }
    match b.objects.get(r).action {
        super::beast_claw::ACTION => 0x0C,
        // sub_80EAF26
        0x41 => panic!("the lock-on of action 0x41 (sub_80EAF26) is not implemented yet"),
        _ => b.content.chip(ai(b, r).attack.chip_id).lockon_mode,
    }
}

/// `ho_8026554`: the panel to attack `target` from in lock-on `mode`;
/// None to stay.
fn destination(b: &Battle, r: ObjectRef, target: PanelPos, mode: u8) -> Option<PanelPos> {
    let mode = if field::is_valid(target.x, target.y) { mode } else { 0 };
    if mode == 0 {
        // sub_802661C: the navi's own panel.
        return Some(b.objects.get(r).panel);
    }
    let Some(search) = b.content.rules.lockon.search(mode) else {
        panic!("lock-on mode {mode:#x} (jt_8026584) is not implemented yet");
    };
    let found = search_near(b, r, target, &search.offsets);
    if !search.prefers_middle_row {
        return found;
    }
    // sub_80265FE: the middle row of that column, if it will do. (Its
    // not-found test reads flags a `mov` just set, so it always runs.)
    let x = found.map_or(0, |p| p.x);
    if can_stand(b, r, x, 2) { Some(PanelPos { x, y: 2 }) } else { found }
}

/// `sub_80265D0`: the first panel of `offsets` next to the target that
/// the navi can stand on; if none, the same from columns shifted toward
/// the navi.
fn search_near(b: &Battle, r: ObjectRef, target: PanelPos, offsets: &[PanelOffset]) -> Option<PanelPos> {
    let o = b.objects.get(r);
    let front = facing(o.alliance, o.flip);
    let columns = std::iter::once(0).chain(b.content.rules.lockon.column_shifts.iter().copied());
    for shift in columns {
        let x = target.x as i32 + front * shift as i32;
        // sub_8026450
        for off in offsets {
            let cx = x + front * off.dx as i32;
            if !(0..=6).contains(&cx) || cx * front > target.x as i32 * front {
                continue;
            }
            let cy = target.y as i32 + off.dy as i32;
            if !(0..=3).contains(&cy) {
                continue;
            }
            if can_stand(b, r, cx as u8, cy as u8) {
                return Some(PanelPos { x: cx as u8, y: cy as u8 });
            }
        }
    }
    None
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
    let action = b.objects.get(r).action;
    if action >= 0x10 {
        super::dispatch(b, r, action);
        if b.objects.get(r).action != 8 {
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
