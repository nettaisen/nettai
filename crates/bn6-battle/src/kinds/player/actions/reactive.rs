//! Reactive defensive chips: when a trap caught a hit (AntiDmg 0xBB,
//! AntiSwrd 0xBC, BodyGrd 0x157, or the AntiDmg program's stance), the
//! navi drops what it was doing and counterattacks: the ruleset's
//! `sub_801056A` (from idle and after each phase of some attacks) and
//! `sub_80105F2` (from the stance) set up the counter's attack and start
//! its action, AntiDmg's, AntiSwrd's or BodyGrd's (the original's 0x47,
//! 0x48 and 0x4B), which are the chips' content: the roles
//! `actions.anti_damage_counter`, `anti_sword_counter` and
//! `body_guard_counter` (`define.roles`). See docs/engine/chips.md §3.6.10.

use crate::actor::{request, status};
use crate::battle::Battle;
use crate::content::Roles;
use crate::kinds::player::{NaviAttack, ai, ai_mut, clear_flag2, coll_mut, set_attack};
use crate::object::ObjectRef;

/// The counter action a trap's request starts: AntiDmg's, AntiSwrd's, or
/// (neither) BodyGrd's.
fn counter_action(b: &Battle, requests: u32, body_guard: bool) -> NaviAttack {
    let roles = &b.content.defs.roles.actions;
    let h = if requests & request::ANTI_DAMAGE_TRIGGERED != 0 {
        Roles::action(roles.anti_damage_counter, "anti_damage_counter")
    } else if requests & request::ANTI_SWORD_TRIGGERED != 0 || !body_guard {
        Roles::action(roles.anti_sword_counter, "anti_sword_counter")
    } else {
        Roles::action(roles.body_guard_counter, "body_guard_counter")
    };
    NaviAttack::content(&b.content.defs, h)
}

/// The trap requests that start a counter from idle or mid-attack.
pub(crate) const TRIGGERS: u32 =
    request::ANTI_DAMAGE_TRIGGERED | request::ANTI_SWORD_TRIGGERED | request::BODY_GUARD_TRIGGERED;

/// What both set-ups share: the attack's links cut, no longer controllable
/// or trap-armed (`sub_801031C(0x810)`), the hit that set the trap off
/// dropped (`sub_800E9FA`), the chip telop the other player sees (HUD).
fn drop_everything(b: &mut Battle, r: ObjectRef) {
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
/// (`sub_802CE78`), and its action starts and runs its first step now.
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
    super::dispatch(b, r, action.number);
}

/// `sub_80105F2(requests, lockout, variant, damage)`: the AntiDmg
/// program's stance (action 0x5A) caught a hit: the counter keeps the
/// stance's damage word, lockout and variant (no chip), and its action
/// starts at the next tick: AntiDmg's, or AntiSwrd's for a sword hit.
pub(crate) fn stance_counter(b: &mut Battle, r: ObjectRef) {
    let requests = ai(b, r).requests;
    let (lockout, variant, damage) = {
        let a = &ai(b, r).attack;
        (a.lockout, a.variant, a.damage as u32 | (a.hit_param as u32) << 16)
    };
    drop_everything(b, r);
    let a = &mut ai_mut(b, r).attack;
    a.damage = damage as u16;
    a.hit_param = (damage >> 16) as u16;
    a.extra = 0;
    a.chip = None;
    a.element = 0;
    a.lockout = lockout;
    a.variant = variant;
    let action = counter_action(b, requests, false);
    set_attack(b, r, action, 0);
}
