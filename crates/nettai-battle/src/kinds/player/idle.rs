//! Action 8: the idle controller (`sub_80EA734` → `sub_80F0354`). Its
//! phase 0 arms the "controllable" state for 10 ticks; every tick it
//! starts the highest-priority requested action: special forms, reactive
//! chips, the forced charged shot, buster, charged shot, B+Back special,
//! mode-9 A, a chip, a move, a turn or a buffered move. See
//! objects-and-player.md §M5 and §B5.

use super::actions::movement::{self, MoveKind};
use super::{
    Emotion, ai, ai_mut, switch_protected, emotion, exit_attack_state, flag1, form_of,
    is_link, reset_charge, set_attack, stats, stats_mut,
};
use crate::actor::{request, status};
use crate::battle::Battle;
use crate::collision::f1;
use crate::input::keys;
use crate::object::ObjectRef;
use nettai_content_api::{ChipHandle, WeaponHandle};

/// Action 8, `sub_80EA734`. (A navi no player controls idles its own way,
/// its driver's: `ai_navi::idle`.)
pub(super) fn control(b: &mut Battle, r: ObjectRef) {
    if super::ai_navi::is_ai_navi(b, r) {
        return super::ai_navi::idle(b, r);
    }
    if b.is_battle_over() {
        return battle_over(b, r);
    }
    let f = ai(b, r).requests;
    if f & (request::ANTI_DAMAGE_TRIGGERED | request::ANTI_SWORD_TRIGGERED | request::BODY_GUARD_TRIGGERED) != 0 {
        return reactive_chip(b, r);
    }
    if f & request::STUN_STRIKE != 0 {
        let strike = super::role_action(b, crate::content::ActionRole::StunStrike);
        return set_attack(b, r, strike, 0);
    }
    // JumpTable80EA7B0[enemy struct byte 4]: every entry is sub_80F0354.
    // EXE5's table (0x080EB068, by control mode and AI index) has an entry
    // of GyroMan's own (0x080F0978): the navi's `idle`, which goes on to
    // the common idle unless it started an action.
    let navi = stats(b, r).navi;
    if let Some(f) = b.content.defs.navi(navi).idle {
        crate::behavior::call_hook(b, f, nettai_content_api::HookCall::FormNavi { navi: r });
        if super::navi_action(b, r) != super::NaviAction::Idle {
            return;
        }
    }
    decide(b, r);
}

/// Once the battle is over the survivor drops its charge and statuses and
/// stands still (a Cross navi returns to base form).
fn battle_over(b: &mut Battle, r: ObjectRef) {
    reset_charge(b, r);
    super::clear_statuses(b, r);
    // sub_801DACC(0x42): the console's chip icons and window go, whichever
    // navi this is.
    b.chip_hud = Default::default();
    if switch_protected(b, r) {
        // (The original stores 1 in the attack's variant byte first; the
        // action it starts doesn't use it.)
        let protect = super::role_action(b, crate::content::ActionRole::SwitchProtect);
        return set_attack(b, r, protect, 0);
    }
    b.objects.get_mut(r).anim = 0;
}

/// `sub_801056A`: a reactive defensive chip fires.
fn reactive_chip(b: &mut Battle, r: ObjectRef) {
    super::actions::reactive::counter(b, r);
}

/// `sub_80F0354`.
fn decide(b: &mut Battle, r: ObjectRef) {
    // Its console's chip window follows `sub_800A772`.
    let side = b.objects.get(r).alliance as usize & 1;
    b.chip_hud[side].window = super::input::chips_enabled(b, r);
    phase_timer(b, r);
    // A controlled form (EXE6's Beast Over): the side's systems' controller
    // decides (`sub_802D322`, the berserk).
    if form_of(b, r).traits.has(crate::content::FormTraits::CONTROLLED) {
        match b.systems_controller(side as u8, r) {
            // A chip's use started: the hand's (1), or the attack's own (4).
            1 => {
                let chip = super::next_chip(b, r);
                after_chip(b, r, chip);
            }
            4 => {
                let chip = ai(b, r).attack.chip;
                after_chip(b, r, chip);
            }
            // The buster.
            2 => {
                leave_idle(b, r);
                let action = buster_routine(b, r);
                set_attack(b, r, action, 1);
            }
            // Nothing, or a step started.
            _ => {}
        }
        return;
    }
    start_specials(b, r);
    let side = b.objects.get(r).alliance as usize;
    // sub_802E4B8: a running special takes the navi over.
    if b.sides[side].select_special != 0 {
        return select_special(b, r);
    }
    // A system's takeover (EXE6's Cross special, `sub_802D4C6`): the side's
    // systems' `takeover` decides, and ends it.
    if b.sides[side].takeover != 0 {
        match b.systems_takeover(side as u8, r) {
            // A chip's use started: the hand's (1), or the attack's own (4).
            1 => {
                let chip = super::next_chip(b, r);
                after_chip(b, r, chip);
            }
            4 => {
                let chip = ai(b, r).attack.chip;
                after_chip(b, r, chip);
            }
            // The buster.
            2 => {
                leave_idle(b, r);
                let action = buster_routine(b, r);
                set_attack(b, r, action, 1);
            }
            // Nothing, or a step started.
            _ => {}
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
        let shot = super::role_action(b, crate::content::ActionRole::ForcedChargedShot);
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
        let sticky = ai(b, r).charge_shot.is_some_and(|w| b.content.weapon(w).sticky);
        let kind = if sticky { 2 } else { 1 };
        return set_attack(b, r, action, kind);
    }
    // EXE5's Chaos Unison releases (0x080F034E, 0x080F0382).
    if f & request::CHAOS_SUCCESS != 0 {
        return chaos_success(b, r);
    }
    if f & request::CHAOS_FAILURE != 0 {
        return chaos_failure(b, r);
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
    // EXE5's no-charge drive (0x080F03E4): the side's systems take the step
    // a navi with the no-charge state would take (DarkInvs: EXE5's
    // auto battle AI, 0x0802B4AC), and reset their drive's state for one
    // without it (0x0802C03A); a system that answers has decided.
    if b.game_rules().intake.no_charge_drive && b.systems_controller_answer(side as u8, r).is_some() {
        return;
    }
    let dir = held_direction(b, r);
    if dir != 0 {
        return start_move(b, r, dir);
    }
    if ai(b, r).requests & (request::TURN_L | request::TURN_R) != 0 {
        let turn = super::role_action(b, crate::content::ActionRole::Turn);
        return set_attack(b, r, turn, 4);
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
        // sub_801DA48(2): the console's chip icons show, whichever navi
        // this is.
        for hud in &mut b.chip_hud {
            hud.icons = true;
        }
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
    let side = b.objects.get(r).alliance as usize & 1;
    b.chip_hud[side].window = false;
}

/// `sub_802E4E4`: the specials' requests start them: the SELECT special
/// (the own-gauges mode's), and the Cross special (DarkInvs'
/// auto-battle: 0x1E0 ticks, invulnerable meanwhile, its controller's
/// state cleared).
fn start_specials(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    if ai(b, r).requests & request::SELECT_SPECIAL != 0 {
        ai_mut(b, r).requests &= !request::SELECT_SPECIAL;
        b.sides[side].select_special = 1;
        clear_special_selection(b, r);
    }
    // A takeover asked for (EXE6's Cross special): the side's systems start
    // it (`takeover_requested`).
    if ai(b, r).requests & request::TAKEOVER != 0 {
        ai_mut(b, r).requests &= !request::TAKEOVER;
        clear_special_selection(b, r);
        b.systems_takeover_requested(side as u8, r);
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

/// EXE5's 0x080F034E: a Chaos Unison charge released in its window: the
/// chaos level rises (at most 4) and the soul's chaos weapon fires
/// (0x0800F338: the side's statistic 0 counts it), an attack of kind 5
/// (the charge stays armed).
fn chaos_success(b: &mut Battle, r: ObjectRef) {
    leave_idle(b, r);
    let a = ai_mut(b, r);
    a.chaos.level = (a.chaos.level + 1).min(4);
    let side = b.objects.get(r).alliance;
    b.bump_side_stat(side, 0, 1);
    let weapon = ai(b, r).chaos.weapon;
    let action = weapon_slot_routine(b, r, weapon);
    set_attack(b, r, action, CHAOS_WEAPON_KIND);
}

/// The attack kinds of the chaos releases (EXE5's `set_attack` slots 5 and
/// 6): the failure's end disarms the charge (`end_attack`).
pub(crate) const CHAOS_WEAPON_KIND: u8 = 5;
pub(crate) const CHAOS_FAILURE_KIND: u8 = 6;

/// EXE5's 0x080F0382: released out of the window: uninterruptible, the
/// chaos failure (EXE5's action 0x39, the role `chaos_failure`).
fn chaos_failure(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).status |= status::UNINTERRUPTIBLE;
    leave_idle(b, r);
    let failure = super::role_action(b, crate::content::ActionRole::ChaosFailure);
    set_attack(b, r, failure, CHAOS_FAILURE_KIND);
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
fn buster_routine(b: &mut Battle, r: ObjectRef) -> super::NaviAction {
    weapon_slot_routine(b, r, ai(b, r).buster)
}

/// A weapon slot's routine; an empty slot (0xFF) reads past
/// `off_80117D4` (the input decoding never requests one).
fn weapon_slot_routine(b: &mut Battle, r: ObjectRef, weapon: Option<WeaponHandle>) -> super::NaviAction {
    let Some(weapon) = weapon else { panic!("weapon routine 0xff reads past off_80117D4") };
    weapon_routine(b, r, weapon)
}

/// `off_80117D4[routine]`: set up a weapon's attack variables and name
/// its action: the weapon's `setup`.
pub(super) fn weapon_routine(b: &mut Battle, r: ObjectRef, weapon: WeaponHandle) -> super::NaviAction {
    use nettai_content_api::{ActionHandle, HookCall, Registry, Value};
    if let Some(setup) = b.content.defs.weapon(weapon).setup {
        let instant = b.content.defs.weapon(weapon).instant;
        let action = match crate::behavior::call_hook(b, setup, HookCall::Weapon { navi: r }) {
            Value::Def(Registry::Action, h) => super::NaviAction::Content(ActionHandle(h)),
            // A chip the weapon loaded as its attack: what the chip's use
            // starts (its action, or the dimming, navi or instant chips'
            // action), as `loc_80126EA` returns the chip record's action.
            Value::Def(Registry::Chip, h) => super::chip_use::chip_action(b, r, Some(nettai_content_api::ChipHandle(h))),
            // A weapon with an instant effect of its own names no action:
            // the instant chips' action runs it.
            Value::Nil if instant.is_some() => super::EngineAction::InstantChip.into(),
            // A routine that ended the attack itself (`object_exitAttackState`)
            // returns false: the original returns the idle state's number
            // (8), which the attack then runs, and the attack update ends
            // it on the next tick (`sub_80EAF36`'s end path); the navi is
            // idle here at once, which no frame tells apart (the patch
            // cards' invisibility taken by Rush, the chip lab's
            // jp/cards/combos/invisible-charge-rush).
            Value::Bool(false) => super::NaviAction::Idle,
            v => panic!("weapon {:?} names {v:?}, not an action", b.content.defs.weapon(weapon).key),
        };
        // A weapon's instant effect: a chip's (it runs, and the navi
        // idles), or one no chip has (TenguCross's wind), after which the
        // navi waits. (Not when the routine ended the attack.)
        if let Some(f) = instant
            && action != super::NaviAction::Idle
        {
            use super::actions::instant::Effect;
            let waits = b.content.defs.weapon(weapon).instant_waits;
            ai_mut(b, r).attack.instant = Some(if waits { Effect::RunsThenWaits(f) } else { Effect::Runs(f) });
        }
        return action;
    }
    let w = b.content.weapon(weapon);
    // An A-charge that is its chip has no routine (the original's entry is
    // `nullsub_44`): started as a weapon, the game starts whatever action
    // the register it called through holds. (Forms name them only as
    // A-charges, which `chip_use` handles before calling here.)
    assert!(w.charged_chip.is_some(), "a weapon has a setup or is its chip");
    panic!("weapon {:?} is no routine (nullsub_44, off_80117D4): the game starts an action from a stale register", w.key)
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
    // loc_80F05F2: its console's chip window goes, intercepted or not.
    let side = b.objects.get(r).alliance as usize & 1;
    b.chip_hud[side].window = false;
    if intercepted(b, r, chip) {
        exit_attack_state(b, r);
        return;
    }
    let a = ai_mut(b, r);
    a.status &= !status::CONTROLLABLE;
    a.status |= status::CHIP_IN_PROGRESS;
    // The other player's console shows the chip's name (sub_801EB18),
    // unless it is a cut-in chip (its telop does) or the invalid chip
    // (the original's 0x185).
    let (used, damage, bonus) = {
        let a = &ai(b, r).attack;
        (a.chip, a.damage, a.extra)
    };
    if let Some(used) = used
        && chip.is_some_and(|c| b.content.chip(c).flags.0 & crate::content::ChipFlags::DIMMING == 0)
        && !b.roles().is_chip(crate::content::ChipRole::Invalid, used)
    {
        let side = b.objects.get(r).alliance;
        b.show_used_chip(side, used, damage, bonus);
    }
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
    intercepted_by(b, r, chip, true)
}

/// `sub_8010740` alone: the opponent's Rush (a weapon that fires a chip
/// asks it, the patch cards' invisibility).
pub(super) fn rush_intercepts(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) -> bool {
    intercepted_by(b, r, chip, false)
}

/// The opponent's Beat (`sub_80106C0`, when `beat`) or Rush (`sub_8010740`)
/// takes the chip.
fn intercepted_by(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>, beat: bool) -> bool {
    use crate::content::{ChipClass, ExtraChipFlags};
    if !is_link(b) {
        return false;
    }
    // The attack's chip; none reads as the pack's chip 0.
    let record = b.chip_field(chip).clone();
    let other = b.objects.get(r).alliance ^ 1;
    let support = match b.stats[other as usize].support {
        Some(opp) if beat && opp.beat && matches!(record.class, ChipClass::Mega | ChipClass::Giga) => {
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

/// The NaviCust supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Support {
    Rush,
    Beat,
    Tango,
}

impl Support {
    /// The support's name in the controller's `support` state field (the
    /// original's first parameter: 0, 1, 2).
    fn name(self) -> &'static str {
        match self {
            Support::Rush => "rush",
            Support::Beat => "beat",
            Support::Tango => "tango",
        }
    }

    /// The role of the chip the controller's telop names (its +0x30).
    fn telop_chip(self) -> crate::content::ChipRole {
        match self {
            Support::Rush => crate::content::ChipRole::Rush,
            Support::Beat => crate::content::ChipRole::Beat,
            Support::Tango => crate::content::ChipRole::Tango,
        }
    }
}


/// `sub_80E90FE`, then `sub_800BF16(side, 1, controller)`: `support`'s
/// controller on `host`'s panel, its side's, and a dimming its side starts
/// that no one can cut in on (`host` its user). The controller's state
/// says which support comes (its `support` field; the original's first
/// parameter) and, for Rush, the chip he eats (`eaten`; the original's
/// third and fourth parameters).
fn summon_support(b: &mut Battle, host: ObjectRef, support: Support, chip: Option<ChipHandle>) {
    let h = b.objects.get(host);
    let (panel, side) = (h.panel, h.alliance);
    // (The chip Rush eats, which the controller's state carries.)
    let chip_value = |h: Option<ChipHandle>| {
        h.map_or(nettai_content_api::Value::Nil, |h| nettai_content_api::Value::Def(nettai_content_api::Registry::Chip, h.0))
    };
    let eaten = chip_value(chip.filter(|_| support == Support::Rush));
    let telop = chip_value(b.roles().try_chip(support.telop_chip()));
    // The spawn's position is the caller's r1..r3: the host's panel row
    // and two zeros.
    let pos = crate::object::Vec3 { x: panel.y as i32, y: 0, z: 0 };
    let kind = b.roles().kind(crate::content::KindRole::Support);
    let controller = crate::kinds::spawn(b, kind, nettai_content_api::SpawnAt::AfterCurrent, pos, [0; 4]);
    if let Some(c) = controller {
        let o = b.objects.get_mut(c);
        o.panel = panel;
        o.element = 0;
        o.related[0] = Some(host);
        o.alliance = side;
        o.damage = 0;
        o.stamina = 0;
        crate::behavior::set_state_field(b, c, "telop_chip", telop);
        // The same for the presentation.
        let named = b.roles().try_chip(support.telop_chip());
        b.objects.get_mut(c).telop_chip = named.map(|chip| crate::hud::TelopChip { chip: Some(chip), ..Default::default() });
        crate::behavior::set_state_variant(b, c, "support", support.name());
        crate::behavior::set_state_field(b, c, "eaten", eaten);
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

/// `sub_8010332`: ticks of lag at the end of a move (4 for MegaMan, but
/// in a form with its own: EXE5's ShadowSoul's 0, 0x0800E0D2).
pub(super) fn move_lag(b: &Battle, r: ObjectRef) -> u16 {
    if super::battle_mode(b) == 9 {
        return 1;
    }
    let s = stats(b, r);
    if super::is_megaman(b, r) {
        return super::form_of(b, r).move_lag.unwrap_or(4) as u16;
    }
    b.content.navi(s.navi).move_lag[s.navi_variant as usize] as u16
}
