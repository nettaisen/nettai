//! Buttons to requests (`sub_8012FC8`) and the buster charge
//! (`sub_8012EBC`). See objects-and-player.md §M3.3, §B3 and §B4.

use super::{ai, ai_mut, battle_mode, form_of, navi_of, next_chip, own_gauges, stats};
use crate::actor::{request, status};
use crate::battle::{Battle, battle_flags};
use nettai_content_api::ChipHandle;
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
    match b.game_rules().effects.charge {
        crate::content::ChargeControls::HoldFlags => {
            decode(b, r);
            accumulate_charge(b, r);
        }
        crate::content::ChargeControls::PerButton => {
            decode_per_button(b, r);
            accumulate_per_button(b, r);
        }
    }
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
    use crate::content::{ChipFlags, ChipTraits};
    // (A link navi's own chip, the original's last block of chips, never
    // charges.)
    if b.content.chip_links(chip).own_chip_of.is_some() {
        return false;
    }
    let c = b.content.chip(chip);
    let family = c.family;
    let damaging = c.flags.has(ChipFlags::HAS_DAMAGE) && !c.flags.has(ChipFlags::DIMMING);
    // The form's `charged_chips`: a family's damaging chips (ElecCross's
    // Null, SlashCross's Sword and the element swords, TomahawkCross's
    // Wood, SpoutCross's Aqua, GroundCross's Break, ChargeCross's Fire),
    // and in Beast Out any Null chip; EXE5's souls' (0x0801090A) a family's
    // chips that are neither dimming nor dark chips (`plain`).
    let plain = !c.flags.has(ChipFlags::DIMMING) && !c.flags.has(ChipFlags::DARK);
    let charges = form_of(b, r).charged_chips.iter().any(|rule| {
        (family == rule.family || (rule.element_swords && c.traits.has(ChipTraits::ELEMENT_SWORD)))
            && (damaging || !rule.damaging)
            && (plain || !rule.plain)
    });
    if charges {
        return true;
    }
    // The navi's own `charged_chips` (MegaMan has none), where the content
    // gave its side them for the round (`crate::given`). EXE6's link navis:
    // from a navi level (`sub_800F49E`, `byte_8021369`: their `when`),
    // ChargeMan's, SpoutMan's, TomahawkMan's and ProtoMan's damaging chips of
    // their family. EXE5's team navis (0x0801090A's tests by navi, after the
    // souls'): NapalmMan's Fire, ToadMan's Aqua, MagnetMan's Elec and
    // TomahawkMan's Wood chips that are neither dimming nor dark chips,
    // whatever the level.
    let Some(own) = navi_of(b, r).charged_chips else { return false };
    if !b.given.navis[b.objects.get(r).alliance as usize & 1].charges {
        return false;
    }
    family == own.family && (damaging || !own.damaging) && (plain || !own.plain)
}

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
    if own_gauges(b) && ai(b, r).pad.pressed & keys::SELECT != 0 {
        let side = b.objects.get(r).alliance as usize;
        if b.sides[side].gauge >= 0x1500 {
            ai_mut(b, r).requests |= request::SELECT_SPECIAL;
            return;
        }
        // Otherwise its player hears that it can't.
        b.sound_for(b.objects.get(r).alliance, crate::content::SoundRole::Refused);
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
        // (A game whose fight reads the keys itself asks nothing here: the
        // flow's `custom_request`.)
        let asks = b.game_rules().flow.custom_request == crate::content::CustomRequest::NaviInput;
        if asks && b.round.flags & battle_flags::GAUGE_FULL != 0 && pressed & lr != 0 {
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
    let special_holds = form_of(b, r).traits.has(crate::content::FormTraits::SPECIAL_HOLDS_BUSTER);
    let a = ai(b, r);
    if a.buster.is_none() || f0 & (request::BUSTER | request::CHARGED_SHOT) != 0 {
        return;
    }
    // A buster that fires while B is held (the Beast busters, the Beast
    // form's throw).
    let edge = if a.buster.is_some_and(|w| b.content.weapon(w).held) {
        if special_holds && a.requests & request::BACK_SPECIAL != 0 {
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
    let bit = if a.charge_source != 2 || a.charge_level != 2 {
        request::BUSTER
    } else if a.b_charge_time.is_some() {
        // The rules' own B charge (EXE5's Chaos Unison, 0x0801086E: in the
        // cycle's window, the chaos weapon; out of it, the failure): they
        // hear of it now, and take it from idle.
        let side = b.objects.get(r).alliance;
        b.rules_charge_released(side, r);
        request::RULES_RELEASE
    } else {
        request::CHARGED_SHOT
    };
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
    a.charge_counter = count as u16;
    a.charge_level = if count < 10 {
        0
    } else if count < threshold as u32 {
        1
    } else {
        a.charge_counter = threshold;
        2
    };
}

// ---- A charge per button (EXE4's) -----------------------------------------------

/// EXE4's 0x0800BDE0: raise requests from this tick's buttons. Dimmed, only a
/// dimming chip can cut in (as `decode`'s). Else: the buster on B's release,
/// unless a buster or charged shot is asked already, charged with a full B
/// charge from last tick; B then Left within 8 ticks the B+Left special; a
/// chip on A's press (its release when the chip charges), charged with a
/// full A charge. (At each ask the original also copies both charge levels
/// to AIData +0x14 and +0x15, 0x0800BE48 and 0x0800BF10, which nothing
/// reads in the end: docs/design/exe4-map.md item 50.)
fn decode_per_button(b: &mut Battle, r: ObjectRef) {
    let f0 = ai(b, r).requests;
    if b.is_dimmed() {
        if !chips_enabled(b, r) || f0 & request::CUT_IN != 0 || next_chip(b, r).is_none() {
            return;
        }
        if ai(b, r).dimmed_pad.pressed & keys::A != 0 {
            ai_mut(b, r).requests |= request::CUT_IN;
        }
        return;
    }
    let (pressed, released, source, level) = {
        let a = ai(b, r);
        (a.pad.pressed, a.pad.released, a.charge_source, a.charge_level)
    };
    if f0 & (request::BUSTER | request::CHARGED_SHOT) == 0 && released & keys::B != 0 {
        let full = source == 2 && level == 2;
        let a = ai_mut(b, r);
        a.requests |= if full { request::CHARGED_SHOT } else { request::BUSTER };
    }
    // (0x0800BE50: a form's rapid presses, GutsSoul's. A press counts and
    // opens the window; the window running out drops the count; at the
    // count the forced charged shot is asked for every tick until it starts,
    // the window set to the count.)
    if let Some(rapid) = form_of(b, r).rapid_presses
        && f0 & request::FORCED_CHARGED_SHOT == 0
    {
        let a = ai_mut(b, r);
        if pressed & keys::B != 0 {
            a.rapid_presses = a.rapid_presses.wrapping_add(1);
            a.rapid_window = rapid.window;
        }
        if a.rapid_window != 0 {
            a.rapid_window -= 1;
            if a.rapid_window == 0 {
                a.rapid_presses = 0;
            }
        }
        if a.rapid_presses >= rapid.presses {
            a.rapid_window = a.rapid_presses;
            a.requests |= request::FORCED_CHARGED_SHOT;
        }
    }
    // (0x0800BE96: the B+Left special, its weapon's.)
    let a = ai(b, r);
    if a.back_special.is_some() && a.back_special_cooldown == 0 {
        let mut window = a.back_special_window;
        if window != 0 || pressed & keys::B != 0 {
            if window == 0 {
                window = 8;
            }
            let a = ai_mut(b, r);
            if pressed & keys::LEFT != 0 {
                a.requests |= request::BACK_SPECIAL;
                a.back_special_window = 0;
            } else {
                a.back_special_window = window - 1;
            }
        }
    }
    if !chips_enabled(b, r) || f0 & (request::CHIP | request::CHARGED_CHIP) != 0 || next_chip(b, r).is_none() {
        return;
    }
    let edge = if chip_charges_on_a(b, r) { ai(b, r).pad.released } else { pressed };
    if edge & keys::A == 0 {
        return;
    }
    let full = source == 1 && level == 2;
    let a = ai_mut(b, r);
    a.requests |= if full { request::CHARGED_CHIP } else { request::CHIP };
}

/// EXE4's 0x0800BBA4 and 0x0800BB50: the charge, by the button held. Not
/// dimmed: with an attack asked (0x2F) or the navi not to be controlled,
/// none. Else B held (after A's charge: B pressed) charges B, its count
/// going on while the navi has a charged shot (0 without); with a chip in
/// hand, A held (after B's charge: A pressed) charges A, its count going on
/// while the chip charges (0 if it doesn't); else none. A count is the
/// source's own (the other's drops to 0) and goes on to 510 (then 511 and
/// 510 by turns). The level: full at the source's threshold, else 1 from 10
/// ticks.
fn accumulate_per_button(b: &mut Battle, r: ObjectRef) {
    if b.is_dimmed() {
        return;
    }
    let (f, status, held, pressed, source) = {
        let a = ai(b, r);
        (a.requests, a.status, a.pad.held, a.pad.pressed, a.charge_source)
    };
    let asked = request::BUSTER | request::CHARGED_SHOT | request::CHIP | request::CHARGED_CHIP | request::FORCED_CHARGED_SHOT;
    if f & asked != 0 || status & status::CONTROLLABLE == 0 {
        return super::reset_charge_counters(b, r);
    }
    let chip = next_chip(b, r).is_some();
    // 0x0800BBDA: by the source charging last tick.
    let next = match source {
        0 if held & keys::B != 0 => 2,
        1 if pressed & keys::B != 0 => 2,
        0 | 1 if chip && held & keys::A != 0 => 1,
        2.. if chip && pressed & keys::A != 0 => 1,
        2.. if held & keys::B != 0 => 2,
        _ => return super::reset_charge_counters(b, r),
    };
    let counts = if next == 2 { ai(b, r).charge_shot.is_some() } else { chip_charges_on_a(b, r) };
    let a = ai_mut(b, r);
    let count = if a.charge_source == next { a.charge_counter } else { 0 };
    a.charge_source = next;
    a.charge_counter = match count + 1 {
        _ if !counts => 0,
        n if n >> 1 > 255 => 510,
        n => n,
    };
    let threshold = charge_threshold(b, r, next);
    let a = ai_mut(b, r);
    a.charge_level = if a.charge_counter >= threshold {
        2
    } else if a.charge_counter >= 10 {
        1
    } else {
        0
    };
}

/// EXE4's 0x0800BC78: the chip in hand charges on A (by the navi's A charge
/// and its form's charged chips, as `a_chargeable`'s test, without its
/// requests and status).
fn chip_charges_on_a(b: &Battle, r: ObjectRef) -> bool {
    let a = ai(b, r);
    if a.a_charge.is_none() && a.alt_a_charge.is_none() {
        return false;
    }
    next_chip(b, r).is_some_and(|chip| chip_charges(b, r, chip))
}

/// `sub_8012F62`: ticks to a full charge for the charge routine of
/// `source` (the charge shot's for B, the A-charge's otherwise) at the
/// navi's Charge stat; 0xFF without a routine.
fn charge_threshold(b: &Battle, r: ObjectRef, source: u8) -> u16 {
    let s = stats(b, r);
    let a = ai(b, r);
    // The rules' own B charge's time (EXE5's armed Chaos Unison charge,
    // 0x0801067E: the chaos weapon's by the chaos level).
    if source == 2 && let Some(ticks) = a.b_charge_time {
        return ticks as u16;
    }
    let routine = if source == 2 {
        a.charge_shot
    } else if form_of(b, r).traits.has(crate::content::FormTraits::ALT_CHARGE_TIME) && uses_alt_a_charge(b, r) {
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
