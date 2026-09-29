//! Buttons to requests (`sub_8012FC8`) and the buster charge
//! (`sub_8012EBC`). See objects-and-player.md §M3.3, §B3 and §B4.

use super::{ai, ai_mut, battle_mode, is_mode_40, next_chip, stats};
use crate::actor::{request, status};
use crate::battle::{Battle, battle_flags};
use crate::hand::NO_CHIP;
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
    if a.a_charge == 0xFF && a.alt_a_charge == 0xFF {
        return false;
    }
    if !may_charge(b, r) || next_chip(b, r) == NO_CHIP {
        return false;
    }
    chip_charges(b, r, next_chip(b, r))
}

/// `sub_8013236`: whether chip `id` charges on A in the navi's form: its
/// attack family matches the form (damaging, not time-freeze chips; any
/// Null-family chip in Beast Out).
fn chip_charges(b: &Battle, r: ObjectRef, id: u16) -> bool {
    use crate::data::{ChipFlags, chip};
    if id >= 0x190 {
        return false;
    }
    let c = chip(id);
    let (family, form) = (c.family, stats(b, r).form.0);
    let damaging = c.flags.has(ChipFlags::HAS_DAMAGE) && !c.flags.has(ChipFlags::TIME_FREEZE);
    let charges = (form == 2 && family == 0xA && damaging)
        || (matches!(form, 3 | 0xF) && ((0x4C..=0x4F).contains(&id) || family == 5) && damaging)
        || ((0x0B..=0x16).contains(&form) && family == 0xA)
        || (matches!(form, 7 | 0x13) && family == 3 && damaging)
        || (matches!(form, 6 | 0x12) && family == 1 && damaging)
        || (matches!(form, 9 | 0x15) && family == 9 && damaging)
        || (matches!(form, 5 | 0x11) && family == 0 && damaging);
    if charges {
        return true;
    }
    // The link navis' own charged chips (`sub_800F49E`, `byte_8021369`);
    // MegaMan has none.
    if stats(b, r).navi != crate::setup::Navi::MEGAMAN {
        panic!("link navis' charged chips (sub_8013236) are not implemented yet");
    }
    false
}

/// `sub_8013396`: the B button charges.
fn b_chargeable(b: &Battle, r: ObjectRef) -> bool {
    ai(b, r).charge_shot != 0xFF && may_charge(b, r)
}

/// `sub_8012FC8`: raise requests from this tick's buttons.
fn decode(b: &mut Battle, r: ObjectRef) {
    let f0 = ai(b, r).requests;
    if b.is_time_stop() {
        // Only a time-stop counter chip can be requested.
        if !chips_enabled(b, r) || f0 & request::TIMESTOP_CHIP != 0 || next_chip(b, r) == NO_CHIP {
            return;
        }
        if ai(b, r).timestop_pad.pressed & keys::A != 0 {
            ai_mut(b, r).requests |= request::TIMESTOP_CHIP;
        }
        return;
    }
    if is_mode_40(b) && ai(b, r).pad.pressed & keys::SELECT != 0 {
        let side = b.objects.get(r).alliance as usize;
        if b.sides[side].gauge >= 0x1500 {
            ai_mut(b, r).requests |= request::SELECT_SPECIAL;
            return;
        }
        // Otherwise the local side hears SOUND_CANT_JACK_IN.
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

/// B then Back within 8 ticks (`AIData.Unk_13` window), for navis with a
/// B+Back special.
fn decode_back_special(b: &mut Battle, r: ObjectRef) {
    let a = ai(b, r);
    if a.back_special == 0xFF || a.unk_15 != 0 {
        return;
    }
    let (pressed, held) = (a.pad.pressed, a.pad.held);
    let mut window = a.unk_13;
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
        a.unk_13 = 0;
    } else {
        a.unk_13 = window - 1;
    }
}

/// The buster: on B release (B press without a charge shot; B held for
/// some rapid busters). A full B charge from last tick makes it a charged
/// shot.
fn decode_buster(b: &mut Battle, r: ObjectRef, f0: u32) {
    let form = stats(b, r).form;
    let a = ai(b, r);
    if a.buster == 0xFF || f0 & (request::BUSTER | request::CHARGED_SHOT) != 0 {
        return;
    }
    let edge = if matches!(a.buster, 3 | 4 | 0x2C) {
        if matches!(form.0, 0x14 | 0x16) && a.requests & request::BACK_SPECIAL != 0 {
            return;
        }
        if a.requests & request::ALT_CHIP != 0 {
            return;
        }
        a.pad.held
    } else if a.charge_shot != 0xFF {
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
    if !chips_enabled(b, r) || f0 & (request::CHIP | request::CHARGED_CHIP) != 0 || next_chip(b, r) == NO_CHIP {
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
    if b.is_time_stop() {
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
    } else if s.form.is_beast() && uses_alt_a_charge(b, r) {
        a.alt_a_charge
    } else {
        a.a_charge
    };
    if routine == 0xFF {
        return 0xFF;
    }
    crate::data::player::charge_threshold(routine, s.charge)
}

/// Whether the next chip is one of the family (0x0A) that uses the
/// alternative A-charge routine in Beast forms.
fn uses_alt_a_charge(b: &Battle, r: ObjectRef) -> bool {
    let id = next_chip(b, r);
    // The game's empty-hand check tests stale flags and falls through,
    // reading chip 0xFFFF's record past the table.
    if id == NO_CHIP {
        panic!("charge threshold with an empty hand reads past the chip table");
    }
    crate::data::chip(id).family == 0x0A
}
