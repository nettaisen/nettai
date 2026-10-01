//! Buttons to requests (`sub_8012FC8`) and the buster charge
//! (`sub_8012EBC`). See objects-and-player.md §M3.3, §B3 and §B4.

use super::{ai, ai_mut, battle_mode, form_of, navi_of, next_chip, per_player_gauges, stats};
use crate::actor::{request, status};
use crate::battle::{Battle, battle_flags};
use bn6_content_api::ChipHandle;
use crate::input::keys;
use crate::object::ObjectRef;

/// `sub_8012E74`: every tick before anything else. Nothing is decoded
/// while paused; once the battle is over the pad reads as released.
pub(super) fn update(b: &mut Battle, r: ObjectRef) {
    if b.is_battle_over() {
        ai_mut(b, r).pad = Default::default();
        return;
    }
    if b.paused {
        return;
    }
    decode(b, r);
    accumulate_charge(b, r);
}

/// `sub_800A772`: chips are enabled for this side (intro bit 0x04/0x08)
/// and no chip lockout runs.
pub(super) fn chips_enabled(b: &Battle, r: ObjectRef) -> bool {
    let bit = if b.objects.get(r).alliance == 0 { 0x04 } else { 0x08 };
    ai(b, r).lockout == 0 && b.round.intro_bits & bit != 0
}

/// `sub_8012F3E`: charging (and raising the hold flags) is allowed: no
/// attack request pending, charging not blocked, and idle.
pub(super) fn may_charge(b: &Battle, r: ObjectRef) -> bool {
    let a = ai(b, r);
    // Every attack request except B+Back (0x1000002F).
    a.requests & (request::ATTACKS & !request::BACK_SPECIAL | request::MODE9_A) == 0
        && a.status & status::NO_CHARGE == 0
        && a.status & status::CONTROLLABLE != 0
}

/// `sub_801336C`: the A button charges (a charged-chip form).
pub(super) fn a_chargeable(b: &Battle, r: ObjectRef) -> bool {
    let a = ai(b, r);
    if a.a_charge.is_none() && a.alt_a_charge.is_none() {
        return false;
    }
    let Some(chip) = next_chip(b, r) else { return false };
    if !may_charge(b, r) {
        return false;
    }
    chip_charges(b, r, chip)
}

/// `sub_8013236`: whether chip `id` charges on A in the navi's form: its
/// attack family matches the form (damaging, not dimming chips; any
/// Null-family chip in Beast Out).
fn chip_charges(b: &Battle, r: ObjectRef, chip: ChipHandle) -> bool {
    use crate::content::{ChipFamily as F, ChipFlags};
    let id = b.content.chip_number(chip);
    if id.is_some_and(|id| id >= 0x190) {
        return false;
    }
    let c = b.content.chip(chip);
    let (family, form) = (c.family, form_of(b, r).0);
    let damaging = c.flags.has(ChipFlags::HAS_DAMAGE) && !c.flags.has(ChipFlags::DIMMING);
    let charges = (form == 2 && family == F::Null && damaging)
        || (matches!(form, 3 | 0xF) && (id.is_some_and(|id| (0x4C..=0x4F).contains(&id)) || family == F::Sword) && damaging)
        || ((0x0B..=0x16).contains(&form) && family == F::Null)
        || (matches!(form, 7 | 0x13) && family == F::Wood && damaging)
        || (matches!(form, 6 | 0x12) && family == F::Aqua && damaging)
        || (matches!(form, 9 | 0x15) && family == F::Break && damaging)
        || (matches!(form, 5 | 0x11) && family == F::Fire && damaging);
    if charges {
        return true;
    }
    // The link navis' own charged chips: from a navi level
    // (`sub_800F49E`; 0xFF: none), ChargeMan's, SpoutMan's, TomahawkMan's
    // and navi 0xB's damaging chips of their family (MegaMan has none).
    let level = b.navi_levels[b.objects.get(r).alliance as usize];
    if level == 0xFF || !damaging {
        return false;
    }
    let (own, i) = match navi_of(b, r).0 {
        5 => (F::Fire, 0),
        6 => (F::Aqua, 1),
        7 => (F::Wood, 2),
        0xB => (F::Sword, 3),
        _ => return false,
    };
    family == own && level >= LINK_NAVI_CHARGE_LEVELS[i]
}

/// `byte_8021369`: the navi level from which ChargeMan, SpoutMan,
/// TomahawkMan and navi 0xB charge their family's chips.
///
/// Game data held in the engine for now: it belongs with those navis in
/// the content (to move there with the content model's next version).
const LINK_NAVI_CHARGE_LEVELS: [u8; 4] = [3, 11, 11, 11];

/// `sub_8013396`: the B button charges.
fn b_chargeable(b: &Battle, r: ObjectRef) -> bool {
    ai(b, r).charge_shot.is_some() && may_charge(b, r)
}

/// `sub_8012FC8`: raise requests from this tick's buttons.
fn decode(b: &mut Battle, r: ObjectRef) {
    let f0 = ai(b, r).requests;
    if b.is_dimmed() {
        // Only a dimming chip can cut in.
        if !chips_enabled(b, r) || f0 & request::CUT_IN != 0 || next_chip(b, r).is_none() {
            return;
        }
        if ai(b, r).dimmed_pad.pressed & keys::A != 0 {
            ai_mut(b, r).requests |= request::CUT_IN;
        }
        return;
    }
    if per_player_gauges(b) && ai(b, r).pad.pressed & keys::SELECT != 0 {
        let side = b.objects.get(r).alliance as usize;
        if b.sides[side].gauge >= 0x1500 {
            ai_mut(b, r).requests |= request::SELECT_SPECIAL;
            return;
        }
        // Otherwise its player hears that it can't.
        b.play_sound_for(b.objects.get(r).alliance, crate::sound::SoundId::CANT_JACK_IN);
    }
    if battle_mode(b) != 1 && decode_turn(b, r) {
        return;
    }
    decode_holds(b, r);
    decode_back_special(b, r);
    decode_buster(b, r, f0);
    if battle_mode(b) == 9 && ai(b, r).pad.pressed & keys::A != 0 {
        ai_mut(b, r).requests |= request::MODE9_A;
    }
    decode_chip(b, r, f0);
}

/// L/R: turn around where turning is enabled; otherwise, with a full
/// custom gauge, ask for the custom screen (and decode nothing else this
/// tick: returns true).
fn decode_turn(b: &mut Battle, r: ObjectRef) -> bool {
    let pressed = ai(b, r).pad.pressed;
    let lr = keys::L | keys::R;
    if ai(b, r).status & status::CAN_TURN == 0 {
        if b.round.flags & battle_flags::GAUGE_FULL != 0 && pressed & lr != 0 {
            b.set_flags(battle_flags::CUSTOM_REQUESTED);
            return true;
        }
        return false;
    }
    if pressed & lr != 0 {
        let flipped = b.objects.get(r).flip != 0;
        let a = ai_mut(b, r);
        if flipped {
            a.requests = (a.requests | request::TURN_L) & !request::TURN_R;
        } else {
            a.requests = (a.requests | request::TURN_R) & !request::TURN_L;
        }
    }
    false
}

/// The hold flags: which button is charging (A for charged chips, B for
/// the charged shot). Pressing the other button switches.
fn decode_holds(b: &mut Battle, r: ObjectRef) {
    let (held, pressed, f) = {
        let a = ai(b, r);
        (a.pad.held, a.pad.pressed, a.requests)
    };
    let set = |b: &mut Battle, bits: u32| ai_mut(b, r).requests |= bits;
    let clear = |b: &mut Battle, bits: u32| ai_mut(b, r).requests &= !bits;
    if f & request::HOLDS == 0 {
        if a_chargeable(b, r) && held & keys::A != 0 {
            set(b, request::A_HELD);
        } else if b_chargeable(b, r) && held & keys::B != 0 {
            set(b, request::B_HELD);
        }
    } else if f & request::A_HELD != 0 {
        if pressed & keys::B != 0 {
            clear(b, request::A_HELD);
            set(b, request::B_HELD);
        } else if !(a_chargeable(b, r) && held & keys::A != 0) {
            clear(b, request::HOLDS);
        }
    } else if a_chargeable(b, r) && pressed & keys::A != 0 {
        clear(b, request::B_HELD);
        set(b, request::A_HELD);
    } else if !(b_chargeable(b, r) && held & keys::B != 0) {
        clear(b, request::HOLDS);
    }
}

/// B then Back within 8 ticks, for navis with a B+Back special.
fn decode_back_special(b: &mut Battle, r: ObjectRef) {
    let a = ai(b, r);
    if a.back_special.is_none() || a.back_special_cooldown != 0 {
        return;
    }
    let (pressed, held) = (a.pad.pressed, a.pad.held);
    let mut window = a.back_special_window;
    if window == 0 {
        if pressed & keys::B == 0 {
            return;
        }
        window = 8;
    }
    let back = if b.objects.get(r).flip != 0 { keys::RIGHT } else { keys::LEFT };
    let a = ai_mut(b, r);
    if pressed & back != 0 && held & keys::B != 0 {
        a.requests |= request::BACK_SPECIAL;
        a.back_special_window = 0;
    } else {
        a.back_special_window = window - 1;
    }
}

/// The buster: on B release (B press without a charge shot; B held for
/// some rapid busters). A full B charge from last tick makes it a charged
/// shot.
fn decode_buster(b: &mut Battle, r: ObjectRef, f0: u32) {
    let form = form_of(b, r);
    let a = ai(b, r);
    if a.buster.is_none() || f0 & (request::BUSTER | request::CHARGED_SHOT) != 0 {
        return;
    }
    // A buster that fires while B is held (the Beast busters, the Beast
    // form's throw).
    let edge = if a.buster.is_some_and(|w| b.content.weapon(w).held) {
        if matches!(form.0, 0x14 | 0x16) && a.requests & request::BACK_SPECIAL != 0 {
            return;
        }
        if a.requests & request::ALT_CHIP != 0 {
            return;
        }
        a.pad.held
    } else if a.charge_shot.is_some() {
        a.pad.released
    } else {
        a.pad.pressed
    };
    if edge & keys::B == 0 {
        return;
    }
    let bit = if a.charge_source == 2 && a.charge_level == 2 { request::CHARGED_SHOT } else { request::BUSTER };
    ai_mut(b, r).requests |= bit;
}

/// A chip: on A press (A release when A charges). A full A charge makes
/// it a charged chip.
fn decode_chip(b: &mut Battle, r: ObjectRef, f0: u32) {
    if !chips_enabled(b, r) || f0 & (request::CHIP | request::CHARGED_CHIP) != 0 || next_chip(b, r).is_none() {
        return;
    }
    let edge = if a_chargeable(b, r) { ai(b, r).pad.released } else { ai(b, r).pad.pressed };
    if edge & keys::A == 0 {
        return;
    }
    let a = ai_mut(b, r);
    let bit = if a.charge_source == 1 && a.charge_level == 2 { request::CHARGED_CHIP } else { request::CHIP };
    a.requests |= bit;
}

/// `sub_8012EBC`: count the charge while a hold flag is up: level 1 from
/// 10 ticks, 2 (full) at the routine's threshold.
fn accumulate_charge(b: &mut Battle, r: ObjectRef) {
    if b.is_dimmed() {
        return;
    }
    if !may_charge(b, r) {
        super::reset_charge_counters(b, r);
        return;
    }
    let threshold = charge_threshold(b, r, ai(b, r).charge_source);
    let f = ai(b, r).requests;
    if f & request::HOLDS == 0 {
        super::reset_charge_counters(b, r);
        return;
    }
    let a = ai_mut(b, r);
    let source = if f & request::A_HELD != 0 { 1 } else { 2 };
    let old = a.charge_source;
    a.charge_source = source;
    if old != source && old != 0 {
        a.charge_counter = 0;
        a.charge_level = 0;
    }
    let count = a.charge_counter as u32 + 1;
    a.charge_counter = count as u8;
    a.charge_level = if count < 10 {
        0
    } else if count < threshold as u32 {
        1
    } else {
        a.charge_counter = threshold as u8;
        2
    };
}

/// `sub_8012F62`: ticks to a full charge for the charge routine of
/// `source` (the charge shot's for B, the A-charge's otherwise) at the
/// navi's Charge stat; 0xFF without a routine.
fn charge_threshold(b: &Battle, r: ObjectRef, source: u8) -> u16 {
    let s = stats(b, r);
    let a = ai(b, r);
    let routine = if source == 2 {
        a.charge_shot
    } else if form_of(b, r).is_beast() && uses_alt_a_charge(b, r) {
        a.alt_a_charge
    } else {
        a.a_charge
    };
    let Some(routine) = routine else { return 0xFF };
    // The weapon's own charge times, by Charge stat (those past its row
    // the next routine's, as the game reads them).
    let w = b.content.weapon(routine);
    match w.charge_ticks.get(s.charge as usize) {
        Some(&ticks) => ticks,
        None => panic!("content error: weapon {:?} has no charge time at Charge {}", w.key, s.charge),
    }
}

/// Whether the next chip is of the Null family, which uses the
/// alternative A-charge routine in Beast forms. The game's empty-hand
/// check tests flags a `ldr` doesn't set (never equal), so an empty hand
/// reads chip 0xFFFF's record, past the table (`Rules::empty_hand`).
fn uses_alt_a_charge(b: &Battle, r: ObjectRef) -> bool {
    super::null_family(b, next_chip(b, r))
}
