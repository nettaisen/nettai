//! Action 8: the idle controller (`sub_80EA734` → `sub_80F0354`). Its
//! phase 0 arms the "controllable" state for 10 ticks; every tick it
//! starts the highest-priority requested action: special forms, reactive
//! chips, the forced charged shot, buster, charged shot, B+Back special,
//! mode-9 A, a chip, a move, a turn or a buffered move. See
//! objects-and-player.md §M5 and §B5.

use super::actions::movement::{self, MoveKind};
use super::{
    Emotion, ai, ai_mut, clear_flag1, clear_flag2, coll_mut, cross_protected, emotion, exit_attack_state, flag1,
    is_link, reset_charge, set_attack, stats, stats_mut,
};
use crate::actor::{request, status};
use crate::battle::Battle;
use crate::collision::{f1, timer};
use crate::input::keys;
use crate::object::ObjectRef;
use crate::setup::Navi;

/// Action 8, `sub_80EA734`.
pub(super) fn control(b: &mut Battle, r: ObjectRef) {
    if b.is_battle_over() {
        return battle_over(b, r);
    }
    let f = ai(b, r).requests;
    if f & (request::ANTI_DAMAGE_TRIGGERED | request::ANTI_SWORD_TRIGGERED | request::BODY_GUARD_TRIGGERED) != 0 {
        return reactive_chip(b, r);
    }
    if f & request::STUN_STRIKE != 0 {
        return set_attack(b, r, 0x49, 0);
    }
    // JumpTable80EA7B0[enemy struct byte 4]: every entry is sub_80F0354.
    decide(b, r);
}

/// Once the battle is over the survivor drops its charge and statuses and
/// stands still (a Cross navi returns to base form).
fn battle_over(b: &mut Battle, r: ObjectRef) {
    reset_charge(b, r);
    // sub_801A264
    clear_flag1(b, r, 0x8001_E800);
    clear_flag2(b, r, 0x3_00E8);
    let c = coll_mut(b, r);
    for t in [timer::PARALYZE, timer::CONFUSE, timer::BLIND, timer::IMMOBILIZE, timer::FREEZE, timer::BUBBLE] {
        c.status_timers[t] = 0;
    }
    // sub_801DACC(0x42): HUD.
    if cross_protected(b, r) {
        ai_mut(b, r).attack.variant = 1;
        return set_attack(b, r, 0x4D, 0);
    }
    b.objects.get_mut(r).anim = 0;
}

/// `sub_801056A`: a reactive defensive chip fires.
fn reactive_chip(_b: &mut Battle, _r: ObjectRef) {
    panic!("reactive defensive chips (sub_801056A) are not implemented yet");
}

/// `sub_80F0354`.
fn decide(b: &mut Battle, r: ObjectRef) {
    // HUD (local side): the chip window follows `sub_800A772`.
    phase_timer(b, r);
    if stats(b, r).form.is_beast_over() {
        panic!("berserk form controller (sub_802D322) is not implemented yet");
    }
    select_specials(b, r);
    if ai(b, r).requests & (request::ANTI_DAMAGE_TRIGGERED | request::ANTI_SWORD_TRIGGERED) != 0 {
        return reactive_chip(b, r);
    }
    if low_hp_navicust_effect(b, r) {
        return;
    }
    let f = ai(b, r).requests;
    if f & request::FORCED_CHARGED_SHOT != 0 {
        leave_idle(b, r);
        return set_attack(b, r, 0x16, 1);
    }
    if f & request::BUSTER != 0 {
        leave_idle(b, r);
        let action = weapon_routine(b, r, ai(b, r).buster);
        return set_attack(b, r, action, 1);
    }
    if f & request::CHARGED_SHOT != 0 {
        leave_idle(b, r);
        let routine = ai(b, r).charge_shot;
        let action = weapon_routine(b, r, routine);
        let kind = if (0x21..=0x26).contains(&ai(b, r).charge_shot) { 2 } else { 1 };
        return set_attack(b, r, action, kind);
    }
    if ai(b, r).requests & request::BACK_SPECIAL != 0 {
        leave_idle(b, r);
        let action = weapon_routine(b, r, ai(b, r).back_special);
        return set_attack(b, r, action, 3);
    }
    if ai(b, r).requests & request::MODE9_A != 0 {
        leave_idle(b, r);
        let action = weapon_routine(b, r, ai(b, r).mode9_a);
        return set_attack(b, r, action, 1);
    }
    if let Some(chip) = super::chip_use::use_chip(b, r) {
        return after_chip(b, r, chip);
    }
    let dir = held_direction(b, r);
    if dir != 0 {
        return start_move(b, r, dir);
    }
    if ai(b, r).requests & (request::TURN_L | request::TURN_R) != 0 {
        return set_attack(b, r, 0x3B, 4);
    }
    let buffered = ai(b, r).buffered_move;
    if buffered != 0 {
        let lag = move_lag(b, r);
        movement::start(b, r, buffered, lag, MoveKind::Fallback);
    }
}

/// Phase 0: arm the controllable state (charging needs it) for 10 ticks;
/// then phase 4, which does nothing. Decisions run in both phases.
fn phase_timer(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase != 0 {
        return;
    }
    if b.objects.get(r).phase_init == 0 {
        // sub_801DA48(2): HUD.
        b.objects.get_mut(r).timer = 10;
        let a = ai_mut(b, r);
        a.status |= status::CONTROLLABLE;
        a.status &= !status::CHIP_IN_PROGRESS;
        b.objects.get_mut(r).phase_init = 4;
    }
    let o = b.objects.get_mut(r);
    let t = o.timer as i32 - 1;
    o.timer = t as u16;
    if t <= 0 {
        o.phase = 4;
        o.phase_init = 0;
    }
}

/// An attack starts: no longer controllable (the local side's chip
/// window HUD closes).
fn leave_idle(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).status &= !status::CONTROLLABLE;
}

/// `sub_802E4E4` + `sub_802E4B8`: the SELECT and Cross specials of the
/// battle flag 0x40 mode.
fn select_specials(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).requests & (request::SELECT_SPECIAL | request::CROSS_SPECIAL) != 0 {
        panic!("SELECT / Cross specials (sub_802E4E4) are not implemented yet");
    }
    let side = &b.sides[b.objects.get(r).alliance as usize];
    if side.select_special != 0 || side.cross_special != 0 {
        panic!("SELECT / Cross special controllers (sub_802F068, sub_802D4C6) are not implemented yet");
    }
}

/// `sub_8010660`: a NaviCust program (stat 0x0D bit 4) fires once when
/// HP drops to a quarter in link battles.
fn low_hp_navicust_effect(b: &mut Battle, r: ObjectRef) -> bool {
    let Some(support) = stats(b, r).support else { return false };
    let o = b.objects.get(r);
    if !is_link(b) || !support.tango || o.max_hp / 4 < o.hp {
        return false;
    }
    stats_mut(b, r).support = Some(crate::setup::SupportNavis { tango: false, ..support });
    panic!("low-HP NaviCust effect (sub_80E90FE) is not implemented yet");
}

/// `off_80117D4[routine]`: set up a weapon's attack variables and name
/// its action: the content pack's script for the routine
/// (`Hook::Weapon`).
pub(super) fn weapon_routine(b: &mut Battle, r: ObjectRef, routine: u8) -> u8 {
    use bn6_content_api::{Hook, HookCall};
    if let Some(hook) = b.behaviors.hook(Hook::Weapon(routine)) {
        let action = crate::behavior::call_hook(b, hook, HookCall::Weapon { navi: r });
        return action.int().expect("a weapon routine names its action") as u8;
    }
    // These entries are `nullsub_44`: the game starts whatever action the
    // register it called through holds. (Forms name them only as charged
    // chip bonuses, which `chip_use` handles before calling here.)
    if matches!(routine, 0x05 | 0x0D | 0x0E | 0x13 | 0x18 | 0x1F | 0x20 | 0x29 | 0x2D | 0x38) {
        panic!("weapon routine {routine:#x} is nullsub_44 (off_80117D4): the game starts an action from a stale register")
    }
    panic!("weapon routine {routine:#x} (off_80117D4) has no script in the content pack")
}

/// `sub_801265A`: buster damage, attack + 1 (+1 in some forms), at most
/// 10; 1 when worn out.
pub(crate) fn buster_damage(b: &Battle, r: ObjectRef) -> u16 {
    let s = stats(b, r);
    let mut d = s.attack as u16 + b.content.navi(s.navi).buster_bonus as u16;
    if emotion(b, b.objects.get(r).alliance) == Emotion::WornOut {
        d = 1;
    } else {
        d += b.content.form(s.form).buster_bonus as u16;
    }
    d.min(10)
}

/// `loc_80F057C`: after a chip starts: interception by the opponent's
/// NaviCust, the chip-in-progress state, and the hand advances.
fn after_chip(b: &mut Battle, r: ObjectRef, chip: u16) {
    if intercepted(b, r, chip) {
        exit_attack_state(b, r);
        return;
    }
    let a = ai_mut(b, r);
    a.status &= !status::CONTROLLABLE;
    a.status |= status::CHIP_IN_PROGRESS;
    // The remote side shows the chip's name.
    let a = &ai(b, r).attack;
    if a.special_source == 0 && a.kind != 5 {
        // sub_800FC7C
        let side = b.objects.get(r).alliance as usize;
        let hand = &mut b.hands[side];
        if hand.cursor < 5 && hand.ids[hand.cursor as usize] != crate::hand::NO_CHIP {
            hand.cursor += 1;
        }
    }
}

/// `sub_80106C0` / `sub_8010740`: the opponent's NaviCust (stat 0x0D bits
/// 2 and 1) cancels a Mega/Giga chip or a flagged chip, once.
fn intercepted(b: &Battle, r: ObjectRef, chip: u16) -> bool {
    if !is_link(b) {
        return false;
    }
    let Some(opp) = b.stats[(b.objects.get(r).alliance ^ 1) as usize].support else { return false };
    let data = b.content.chip(chip & 0x7FFF);
    let mega_or_giga = matches!(data.class, crate::content::ChipClass::Mega | crate::content::ChipClass::Giga);
    if (opp.beat && mega_or_giga) || (opp.rush && data.extra_flags.has(crate::content::ExtraChipFlags::RUSH_CANCELS)) {
        panic!("NaviCust chip interception (sub_80E90FE) is not implemented yet");
    }
    false
}

/// `sub_800FA54`: the held direction (up, down, right, left in that
/// priority; swapped when confused), none while sliding.
pub(crate) fn held_direction(b: &Battle, r: ObjectRef) -> u8 {
    if flag1(b, r) & f1::SLIDING != 0 {
        return 0;
    }
    let held = ai(b, r).pad.held;
    let dir = if held & keys::UP != 0 {
        1
    } else if held & keys::DOWN != 0 {
        2
    } else if held & keys::RIGHT != 0 {
        4
    } else if held & keys::LEFT != 0 {
        3
    } else {
        return 0;
    };
    if flag1(b, r) & f1::CONFUSED != 0 { [0, 2, 1, 4, 3][dir as usize] } else { dir }
}

/// `sub_80116AE(dir, sub_8010332(), sub_80103A8())`: a step toward `dir`
/// from input (astray with the NaviCust processing bug).
pub(crate) fn start_move(b: &mut Battle, r: ObjectRef, dir: u8) {
    let lag = move_lag(b, r);
    let kind = if stats(b, r).bugs.processing != 0 { MoveKind::Astray } else { MoveKind::Input };
    movement::start(b, r, dir, lag, kind);
}

/// `sub_8010332`: ticks of lag at the end of a move (4 for MegaMan).
fn move_lag(b: &Battle, r: ObjectRef) -> u16 {
    if super::battle_mode(b) == 9 {
        return 1;
    }
    let s = stats(b, r);
    if s.navi == Navi::MEGAMAN {
        return 4;
    }
    b.content.navi(s.navi).move_lag[s.navi_variant as usize] as u16
}
