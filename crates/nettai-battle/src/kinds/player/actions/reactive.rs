//! Reactive defensive chips: when a trap caught a hit (AntiDmg 0xBB,
//! AntiSwrd 0xBC, BodyGrd 0x157, or the AntiDmg program's stance), the
//! navi drops what it was doing and counterattacks: the rules'
//! `sub_801056A` (from idle and after each phase of some attacks) and
//! `sub_80105F2` (from the stance) set up the counter's attack and start
//! its action, AntiDmg's, AntiSwrd's or BodyGrd's (the original's 0x47,
//! 0x48 and 0x4B), which are the chips' content: the roles
//! `actions.anti_damage_counter`, `anti_sword_counter` and
//! `body_guard_counter` (the rules' roles). See docs/engine/chips.md §3.6.10.

use crate::actor::{request, status};
use crate::battle::Battle;
use crate::content::ActionRole;
use crate::kinds::player::{NaviAction, ai, ai_mut, clear_flag2, coll_mut, set_attack};
use crate::object::ObjectRef;

/// The counter action a trap's request starts: AntiDmg's, AntiSwrd's, or
/// (neither) BodyGrd's.
fn counter_action(b: &Battle, requests: u32, body_guard: bool) -> NaviAction {
    let roles = b.roles();
    let h = if requests & request::ANTI_DAMAGE_TRIGGERED != 0 {
        roles.action(ActionRole::AntiDamageCounter)
    } else if requests & request::ANTI_SWORD_TRIGGERED != 0 || !body_guard {
        roles.action(ActionRole::AntiSwordCounter)
    } else {
        roles.action(ActionRole::BodyGuardCounter)
    };
    NaviAction::Content(h)
}

/// The trap requests that start a counter from idle or mid-attack.
pub(crate) const TRIGGERS: u32 =
    request::ANTI_DAMAGE_TRIGGERED | request::ANTI_SWORD_TRIGGERED | request::BODY_GUARD_TRIGGERED;

/// What both set-ups share: the attack's links cut, no longer controllable
/// or trap-armed (`sub_801031C(0x810)`), the hit that set the trap off
/// dropped (`sub_800E9FA`), the chip telop the other player sees (HUD).
fn drop_everything(b: &mut Battle, r: ObjectRef) {
    // sub_801DACC(0x40): the console's chip window goes, whichever navi
    // this is.
    for hud in &mut b.chip_hud {
        hud.window = false;
    }
    b.objects.get_mut(r).related[0] = None;
    let a = ai_mut(b, r);
    a.overlay = None;
    a.status &= !(status::CONTROLLABLE | status::TRAP_ARMED);
    // sub_800E9FA
    clear_flag2(b, r, 0x3_01FE);
    let acc = &mut coll_mut(b, r).acc;
    acc.final_damage = 0;
    acc.element_damage = [0; 6];
}

/// `sub_801056A(requests, 0, 0)`: a trap chip caught a hit: the counter
/// takes the side's defensive-chip record's chip, damage and bonus
/// (`sub_802CE78`) and variant 0, and its action starts and runs its first
/// step now.
pub(crate) fn counter(b: &mut Battle, r: ObjectRef) {
    let requests = ai(b, r).requests;
    drop_everything(b, r);
    let side = b.objects.get(r).alliance as usize & 1;
    let rec = b.linked[side];
    let a = &mut ai_mut(b, r).attack;
    a.damage = rec.damage as u16;
    a.hit_param = (rec.damage >> 16) as u16;
    a.extra = rec.bonus;
    a.chip = rec.chip;
    a.element = 0;
    a.lockout = 0;
    a.variant = 0;
    let action = counter_action(b, requests, true);
    set_attack(b, r, action, 0);
    // The other player's console shows the trap chip's name (sub_801EB18).
    if let Some(chip) = rec.chip {
        b.show_used_chip(side as u8, chip, rec.damage as u16, rec.bonus);
    }
    super::dispatch(b, r, action);
}

/// `sub_80105F2(requests, lockout, variant, damage)`: the AntiDmg
/// program's stance (action 0x5A) caught a hit: the counter keeps the
/// stance's damage word, lockout and variant (which the counters read:
/// EXE6's AntiDmg program sets 0, EXE5's ShadowSoul 1; no chip), and its
/// action starts: AntiDmg's, or AntiSwrd's for a sword hit. It runs from
/// the next tick, or at once by the navi's rules (EXE5's 0x0800E340).
pub(crate) fn stance_counter(b: &mut Battle, r: ObjectRef) {
    let requests = ai(b, r).requests;
    let (lockout, damage) = {
        let a = &ai(b, r).attack;
        (a.lockout, a.damage as u32 | (a.hit_param as u32) << 16)
    };
    drop_everything(b, r);
    let a = &mut ai_mut(b, r).attack;
    a.damage = damage as u16;
    a.hit_param = (damage >> 16) as u16;
    a.extra = 0;
    a.chip = None;
    a.element = 0;
    a.lockout = lockout;
    let action = counter_action(b, requests, false);
    set_attack(b, r, action, 0);
    if b.game_rules().stance_counter == crate::content::StanceCounter::AtOnce {
        super::dispatch(b, r, action);
    }
}
