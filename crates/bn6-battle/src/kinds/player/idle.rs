//! Action 8: the idle controller (`sub_80EA734` → `sub_80F0354`). Its
//! phase 0 arms the "controllable" state for 10 ticks; every tick it
//! starts the highest-priority requested action: special forms, reactive
//! chips, the forced charged shot, buster, charged shot, B+Back special,
//! mode-9 A, a chip, a move, a turn or a buffered move. See
//! objects-and-player.md §M5 and §B5.

use super::actions::movement::{self, MoveKind};
use super::{
    Emotion, ai, ai_mut, cross_protected, emotion, exit_attack_state, flag1, form_of, navi_of,
    is_link, reset_charge, set_attack, stats, stats_mut,
};
use crate::actor::{request, status};
use crate::battle::Battle;
use crate::collision::f1;
use crate::input::keys;
use crate::object::ObjectRef;
use crate::setup::Navi;
use bn6_content_api::{ChipHandle, WeaponHandle};

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
    super::clear_statuses(b, r);
    // sub_801DACC(0x42): HUD.
    if cross_protected(b, r) {
        ai_mut(b, r).attack.variant = 1;
        return set_attack(b, r, 0x4D, 0);
    }
    b.objects.get_mut(r).anim = 0;
}

/// `sub_801056A`: a reactive defensive chip fires.
fn reactive_chip(b: &mut Battle, r: ObjectRef) {
    super::actions::reactive::counter(b, r);
}

/// `sub_80F0354`.
fn decide(b: &mut Battle, r: ObjectRef) {
    // HUD (local side): the chip window follows `sub_800A772`.
    phase_timer(b, r);
    // Beast Over (and any form past it): the berserk controller decides.
    if form_of(b, r).0 >= 0x17 {
        use super::berserk::Outcome;
        match super::berserk::control(b, r) {
            Outcome::Nothing | Outcome::Moved => {}
            Outcome::Chip => {
                let chip = super::next_chip(b, r);
                after_chip(b, r, chip);
            }
            Outcome::Buster => {
                leave_idle(b, r);
                let action = buster_routine(b, r);
                set_attack(b, r, action, 1);
            }
        }
        return;
    }
    start_specials(b, r);
    let side = b.objects.get(r).alliance as usize;
    // sub_802E4B8: a running special takes the navi over.
    if b.sides[side].select_special != 0 {
        return select_special(b, r);
    }
    if b.sides[side].cross_special != 0 {
        if b.sides[side].cross_special_ticks == 0 {
            b.sides[side].cross_special = 0;
            return set_attack(b, r, super::actions::cross_special::ACTION, 0);
        }
        use super::berserk::Outcome;
        match super::berserk::cross_special(b, r) {
            Outcome::Nothing | Outcome::Moved => {}
            Outcome::Chip => {
                let chip = ai(b, r).attack.chip;
                after_chip(b, r, chip);
            }
            Outcome::Buster => {
                leave_idle(b, r);
                let action = buster_routine(b, r);
                set_attack(b, r, action, 1);
            }
        }
        return;
    }
    if ai(b, r).requests & (request::ANTI_DAMAGE_TRIGGERED | request::ANTI_SWORD_TRIGGERED) != 0 {
        return reactive_chip(b, r);
    }
    if low_hp_navicust_effect(b, r) {
        return;
    }
    let f = ai(b, r).requests;
    if f & request::FORCED_CHARGED_SHOT != 0 {
        leave_idle(b, r);
        let role = b.content.defs.roles.action(crate::content::ActionRole::ForcedChargedShot);
        let shot = super::NaviAttack::content(&b.content.defs, role);
        return set_attack(b, r, shot, 1);
    }
    if f & request::BUSTER != 0 {
        leave_idle(b, r);
        let action = buster_routine(b, r);
        return set_attack(b, r, action, 1);
    }
    if f & request::CHARGED_SHOT != 0 {
        leave_idle(b, r);
        let routine = ai(b, r).charge_shot;
        let action = weapon_slot_routine(b, r, routine);
        let special = b.weapon_number(ai(b, r).charge_shot).is_some_and(|n| (0x21..=0x26).contains(&n));
        let kind = if special { 2 } else { 1 };
        return set_attack(b, r, action, kind);
    }
    if ai(b, r).requests & request::BACK_SPECIAL != 0 {
        leave_idle(b, r);
        let action = weapon_slot_routine(b, r, ai(b, r).back_special);
        return set_attack(b, r, action, 3);
    }
    if ai(b, r).requests & request::MODE9_A != 0 {
        leave_idle(b, r);
        let action = weapon_slot_routine(b, r, ai(b, r).mode9_a);
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

/// `sub_802E4E4`: the specials' requests start them: the SELECT special
/// (the battle flag 0x40 mode's), and the Cross special (DarkInvs'
/// auto-battle: 0x1E0 ticks, invulnerable meanwhile, its controller's
/// state cleared).
fn start_specials(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    if ai(b, r).requests & request::SELECT_SPECIAL != 0 {
        ai_mut(b, r).requests &= !request::SELECT_SPECIAL;
        b.sides[side].select_special = 1;
        clear_special_selection(b, r);
    }
    if ai(b, r).requests & request::CROSS_SPECIAL != 0 {
        ai_mut(b, r).requests &= !request::CROSS_SPECIAL;
        b.sides[side].cross_special = 1;
        clear_special_selection(b, r);
        b.sides[side].cross_special_ticks = 0x1E0;
        super::set_invulnerable(b, r, 0xFFFF);
        super::berserk::reset(b, r);
    }
}

/// `sub_802E1EC`: the special's selection is cleared: requests
/// 0x100000..=0x1000000 (and side bytes +0x40, +0x41, which nothing ported
/// reads).
fn clear_special_selection(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).requests &= !0x01F0_0000;
}

/// `sub_802F068`: the SELECT special holds the navi for its ticks, then
/// ends (`sub_802F084`: the gauge would be emptied if side byte +3 were set,
/// but only `sub_802E07C` writes it, with 0).
fn select_special(b: &mut Battle, r: ObjectRef) {
    let s = &mut b.sides[b.objects.get(r).alliance as usize];
    if s.select_ticks != 0 {
        s.select_ticks -= 1;
        if s.select_ticks != 0 {
            return;
        }
    }
    super::reset_select_special(s);
    ai_mut(b, r).requests &= !request::SELECT_SPECIAL;
}

/// `sub_8010660`: in link battles the NaviCust support Tango (stat 0x0D
/// bit 2) comes once when the navi's HP drops to a quarter; the navi does
/// nothing else this tick.
fn low_hp_navicust_effect(b: &mut Battle, r: ObjectRef) -> bool {
    let Some(support) = stats(b, r).support else { return false };
    let o = b.objects.get(r);
    if !is_link(b) || !support.tango || o.max_hp / 4 < o.hp {
        return false;
    }
    stats_mut(b, r).support = Some(crate::setup::Supports { tango: false, ..support });
    summon_support(b, r, Support::Tango, None);
    true
}

/// The buster's weapon routine.
fn buster_routine(b: &mut Battle, r: ObjectRef) -> super::NaviAttack {
    weapon_slot_routine(b, r, ai(b, r).buster)
}

/// A weapon slot's routine; an empty slot (0xFF) reads past
/// `off_80117D4` (the input decoding never requests one).
fn weapon_slot_routine(b: &mut Battle, r: ObjectRef, weapon: Option<WeaponHandle>) -> super::NaviAttack {
    let Some(weapon) = weapon else { panic!("weapon routine 0xff reads past off_80117D4") };
    weapon_routine(b, r, weapon)
}

/// `off_80117D4[routine]`: set up a weapon's attack variables and name
/// its action: the weapon's `setup`.
pub(super) fn weapon_routine(b: &mut Battle, r: ObjectRef, weapon: WeaponHandle) -> super::NaviAttack {
    use bn6_content_api::{ActionHandle, HookCall, Registry, Value};
    if let Some(setup) = b.content.defs.weapon(weapon).setup {
        let action = match crate::behavior::call_hook(b, setup, HookCall::Weapon { navi: r }) {
            Value::Int(n) => super::NaviAttack::from(n as u8),
            Value::Def(Registry::Action, h) => super::NaviAttack::content(&b.content.defs, ActionHandle(h)),
            v => panic!("weapon {:?} names {v:?}, not an action", b.content.defs.weapon(weapon).key),
        };
        // A weapon that names an instant effect no chip has (TenguCross's
        // wind) runs its own.
        if let Some(f) = b.content.defs.weapon(weapon).instant {
            ai_mut(b, r).attack.instant = Some(super::actions::instant::Effect::Runs(f));
        }
        return action;
    }
    let Some(routine) = b.content.weapon_number(weapon) else {
        panic!("content error: weapon {:?} has no setup", b.content.defs.weapon(weapon).key)
    };
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
fn after_chip(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) {
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
        if hand.cursor < 5 && hand.ids[hand.cursor as usize].is_some() {
            hand.cursor += 1;
        }
    }
}

/// `sub_80106C0`, then `sub_8010740`: the opponent's NaviCust support
/// turns the chip back, once: Beat (stat 0x0D bit 1) a Mega or Giga
/// chip, Rush (bit 0) a chip flagged for him. (The game reads the chip's
/// record with the id as it is, flag bits and all.)
fn intercepted(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) -> bool {
    use crate::content::{ChipClass, ExtraChipFlags};
    if !is_link(b) {
        return false;
    }
    // The attack's chip; none reads as the pack's chip 0.
    let record = b.content.chip_field(chip).clone();
    let other = b.objects.get(r).alliance ^ 1;
    let support = match b.stats[other as usize].support {
        Some(opp) if opp.beat && matches!(record.class, ChipClass::Mega | ChipClass::Giga) => {
            b.stats[other as usize].support = Some(crate::setup::Supports { beat: false, ..opp });
            Support::Beat
        }
        Some(opp) if opp.rush && record.extra_flags.has(ExtraChipFlags::RUSH_CANCELS) => {
            b.stats[other as usize].support = Some(crate::setup::Supports { rush: false, ..opp });
            Support::Rush
        }
        _ => return false,
    };
    let Some(host) = b.player(other) else {
        // The game goes on with a null navi: its panel and side come from
        // the BIOS.
        panic!("the opponent's support has no navi to come from (sub_80106C0 / sub_8010740 read a null object)");
    };
    summon_support(b, host, support, chip);
    true
}

/// The NaviCust supports, by the controller's first parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Support {
    Rush = 0,
    Beat = 1,
    Tango = 2,
}

impl Support {
    /// The chip record the controller's telop names (its +0x30).
    fn telop_chip(self) -> u16 {
        match self {
            Support::Rush => 0x179,
            Support::Beat => 0x17A,
            Support::Tango => 0x17B,
        }
    }
}


/// `sub_80E90FE`, then `sub_800BF16(side, 1, controller)`: `support`'s
/// controller on `host`'s panel, its side's, and a dimming its side starts
/// that no one can cut in on (`host` its user). Rush's controller carries
/// the chip he eats in its third and fourth parameters.
fn summon_support(b: &mut Battle, host: ObjectRef, support: Support, chip: Option<ChipHandle>) {
    let h = b.objects.get(host);
    let (panel, side) = (h.panel, h.alliance);
    // (The numeric API's chip, as the controller's parameters carry it.)
    let chip = if support == Support::Rush { b.api_chip_field(chip, 0) } else { 0 };
    let params = [support as u8, 0, chip as u8, (chip >> 8) as u8];
    // The spawn's position is the caller's r1..r3: the host's panel row
    // and two zeros.
    let pos = crate::object::Vec3 { x: panel.y as i32, y: 0, z: 0 };
    let kind = b.content.defs.roles.kind(crate::content::KindRole::Support);
    let controller = crate::kinds::spawn(b, kind, bn6_content_api::SpawnAt::AfterCurrent, pos, params);
    if let Some(c) = controller {
        let o = b.objects.get_mut(c);
        o.panel = panel;
        o.element = 0;
        o.related[0] = Some(host);
        o.alliance = side;
        o.damage = 0;
        o.stamina = 0;
        let telop = bn6_content_api::Value::Int(support.telop_chip() as i64);
        crate::behavior::set_state_field(b, c, "telop_chip", telop);
    }
    b.start_dimming(side, true, controller, host);
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
    if navi_of(b, r) == Navi::MEGAMAN {
        return 4;
    }
    b.content.navi(s.navi).move_lag[s.navi_variant as usize] as u16
}
