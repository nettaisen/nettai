//! Using the next chip in the hand (`sub_800FB54` and `sub_80127C0`):
//! read the hand entry, fill the attack variables from the chip data and
//! the hand, apply the damage modifiers, and start the chip's action. See
//! chips.md §2.6.4 and §2.7.

use super::{Emotion, ai, ai_mut, emotion, flag1, navi_record, set_attack, set_mood, stats};
use crate::actor::{ActorType, request};
use crate::battle::Battle;
use crate::collision::f1;
use crate::content::{ChipFamily, ChipFlags, ChipId, Element};
use crate::object::ObjectRef;
use crate::setup::Navi;

/// The Beast forms' claw, the action weapon routine 0x1E names.
const BEAST_CLAW: u8 = 0x52;

/// Damage-word flag bits a chip use can add (see `oBattleObject_Damage`).
mod damage_flags {
    /// Double damage (Full Synchro, anger, some crosses).
    pub const DOUBLE: u16 = 0x8000;
    /// Paralyzes (a folded WhiCapsl).
    pub const PARALYZE: u16 = 0x4000;
    /// Uninstalls (a folded Uninstll).
    pub const UNINSTALL: u16 = 0x2000;
    /// Erases a cross (EraseCross family chips).
    pub const ERASE_CROSS: u16 = 0x1000;
}

/// `sub_800FB54`: when a chip request is up (and not sliding), take the
/// next chip and start its action. Returns the chip used.
pub(super) fn use_chip(b: &mut Battle, r: ObjectRef) -> Option<ChipId> {
    let requested = ai(b, r).requests & (request::CHIP | request::CHARGED_CHIP | request::ALT_CHIP);
    if flag1(b, r) & f1::SLIDING != 0 || requested == 0 {
        return None;
    }
    if requested & request::CHARGED_CHIP != 0 {
        return Some(use_charged_chip(b, r));
    }
    if requested & request::ALT_CHIP != 0 {
        panic!("Battle Chip Gate slot-in chips (sub_800EE26) are not implemented yet");
    }
    use_next_chip(b, r, 0);
    ai_mut(b, r).requests &= !(request::CHIP | request::CHARGED_CHIP | request::ALT_CHIP);
    Some(ai(b, r).attack.chip_id)
}

/// `loc_800FBEE`: use the next chip (`sub_80127C0(charged)`) and start its
/// action; in a Beast form, or from a special source, the chip's Beast Out
/// lock-on applies.
fn use_next_chip(b: &mut Battle, r: ObjectRef, charged: u8) {
    let action = prepare(b, r, charged);
    set_attack(b, r, action, 2);
    let form = stats(b, r).form;
    let content = b.content.clone();
    let a = &mut ai_mut(b, r).attack;
    if a.special_source != 0 || form.is_beast() {
        a.beast_lockon = content.chip(a.chip_id).beast_lockon as u8;
    }
}

/// `sub_800FB54`'s charged path (request 8): the form's A-charge routine
/// decides what the charged chip becomes. Returns the attack's chip id,
/// which that path leaves 0 for Null-family chips.
fn use_charged_chip(b: &mut Battle, r: ObjectRef) -> ChipId {
    let chip = hand_entry(b, r).chip;
    if chip == crate::hand::NO_CHIP {
        panic!("a charged chip with an empty hand reads past the chip table");
    }
    let family = b.content.chip(chip).family;
    let routine = if family == ChipFamily::Null {
        ai_mut(b, r).attack.chip_id = 0;
        ai(b, r).alt_a_charge
    } else {
        ai(b, r).a_charge
    };
    match routine {
        // GroundCross's A-charge: `sub_8012CB2` (the content pack's script
        // for routine 0x18, which `off_80117D4` leaves a nullsub) drops rocks
        // on the enemies first; what it leaves in r0 is the use's charge
        // argument.
        0x18 => {
            let charged = super::idle::weapon_routine(b, r, 0x18);
            use_next_chip(b, r, charged);
        }
        // No charge routine: the chip is used, with the register that held
        // the chip's family as the charge argument (the Null family's was
        // cleared with the chip id).
        0xFF => {
            let charged = if family == ChipFamily::Null { 0 } else { family as u8 };
            use_next_chip(b, r, charged);
        }
        // The forms whose A-charge is the chip's charged use.
        0x05 | 0x0D | 0x1F | 0x20 | 0x29 | 0x2D => use_next_chip(b, r, 1),
        _ => {
            ai_mut(b, r).attack.charged = 0;
            let action = super::idle::weapon_routine(b, r, routine);
            set_attack(b, r, action, 2);
            let form = stats(b, r).form;
            // The Beast forms' claw (weapon 0x1E) and SlashCross Beast's
            // charged sword run inside the Beast Out rush.
            if action == BEAST_CLAW || (action == 0x41 && form.0 == 0x0F) {
                ai_mut(b, r).attack.beast_lockon = 1;
            }
        }
    }
    ai_mut(b, r).requests &= !(request::CHIP | request::CHARGED_CHIP | request::ALT_CHIP);
    ai(b, r).attack.chip_id
}

/// `sub_800FC30`: the Beast Out rush chains the next chip, starting its
/// action (inside the rush again). Not the claw's chips 0x52/0x53, dimming
/// chips, or an empty hand. True if it did.
pub(super) fn chain_next_chip(b: &mut Battle, r: ObjectRef) -> bool {
    let chip = hand_entry(b, r).chip;
    if chip == crate::hand::NO_CHIP || chip == 0x52 || chip == 0x53 {
        return false;
    }
    if b.content.chip(chip).flags.has(ChipFlags::DIMMING) {
        return false;
    }
    let action = prepare(b, r, 0);
    set_attack(b, r, action, 2);
    ai_mut(b, r).attack.beast_lockon = 1;
    true
}

/// The hand entry at the cursor (`sub_800EDD0`, player branch).
struct HandEntry {
    chip: ChipId,
    damage: u16,
    /// Atk+ and charge bonuses plus any form or aura bonus.
    extra: u16,
    /// Modifier flags folded into the entry (bit 1 paralyze, bit 2
    /// uninstall).
    modifiers: u8,
}

fn hand_entry(b: &Battle, r: ObjectRef) -> HandEntry {
    let o = b.objects.get(r);
    if navi_record(b, r).actor_type != ActorType::Player {
        panic!("chip use by a non-player (sub_800EDD0) is not implemented yet");
    }
    let hand = &b.hands[o.alliance as usize];
    let i = hand.cursor as usize;
    let chip = hand.ids[i];
    let extra = hand.attack_bonus[i].wrapping_add(chip_bonus(b, r, chip)).wrapping_add(hand.charge_bonus[i]);
    HandEntry { chip, damage: hand.damage[i], extra, modifiers: hand.modifiers[i] }
}

/// `sub_80127C0(charged)`: fill the attack variables for the next chip and
/// name its action. `charged` is the A-charge's argument (0 for a plain
/// use).
fn prepare(b: &mut Battle, r: ObjectRef, charged: u8) -> u8 {
    let e = hand_entry(b, r);
    let content = b.content.clone();
    let cd = content.chip(e.chip);
    if cd.dark_substitute.is_some() {
        panic!("dark chip substitution (sub_8010D58) is not implemented yet");
    }
    let action = load_attack(b, r, e.chip);
    let a = &mut ai_mut(b, r).attack;
    a.charged = charged;
    // The hand's damage replaces the chip data's.
    a.damage = e.damage;
    let bonus = charge_bonus(b, r, charged);
    if bonus != 0 {
        let a = &mut ai_mut(b, r).attack;
        a.damage = a.damage.wrapping_add(bonus);
        b.play_sound(crate::sound::SoundId(BOOST_SOUND));
    }
    ai_mut(b, r).attack.extra = e.extra;
    let damage = ai(b, r).attack.damage;
    let (damage, boost) = double_damage(b, r, e.chip, damage, charged);
    ai_mut(b, r).attack.damage = damage;
    let side = b.objects.get(r).alliance;
    match boost {
        Some(Boost::FullSynchro) => {
            set_mood(b, side, 0x80);
            b.play_sound(crate::sound::SoundId(BOOST_SOUND));
        }
        Some(Boost::Anger) => {
            super::status::end_anger(b, r);
            b.play_sound(crate::sound::SoundId(BOOST_SOUND));
        }
        Some(Boost::Element | Boost::BeastOver) | None => {}
    }
    let mut damage = ai(b, r).attack.damage;
    // sub_8012C34
    if e.modifiers & 2 != 0 {
        damage |= damage_flags::PARALYZE;
    }
    if e.modifiers & 4 != 0 {
        damage |= damage_flags::UNINSTALL;
    }
    // sub_8012C4A
    if deals_damage(cd.flags) && cd.family == ChipFamily::Null && matches!(stats(b, r).form.0, 4 | 0x10) {
        damage |= damage_flags::ERASE_CROSS;
    }
    ai_mut(b, r).attack.damage = damage;
    heal_on_use(b, r, e.chip);
    if cd.flags.has(ChipFlags::NAVI) {
        b.bump_side_stat(side, 6, 1);
    }
    // sub_800B79A: some dark chips cost the user when used.
    if (0x11E..=0x122).contains(&e.chip) {
        panic!("dark chip side effects (sub_800B79A) are not implemented yet");
    }
    action
}

/// A chip that deals damage and isn't a dimming chip (the condition every
/// damage bonus shares).
fn deals_damage(flags: ChipFlags) -> bool {
    flags.has(ChipFlags::HAS_DAMAGE) && !flags.has(ChipFlags::DIMMING)
}

/// `sub_80126E4`: the attack variables from the chip data; returns the
/// chip's action. (The game also counts the use per side, for a report
/// only the battle-flag 0x40 mode reads.)
fn load_attack(b: &mut Battle, r: ObjectRef, chip: ChipId) -> u8 {
    let content = b.content.clone();
    let cd = content.chip(chip);
    let side = b.objects.get(r).alliance;
    let damage = crate::hand::chip_damage(b, chip, side);
    let a = &mut ai_mut(b, r).attack;
    a.chip_id = chip;
    a.params = cd.params;
    a.damage = damage;
    a.hit_param = cd.hit_param as u16;
    a.lockout = cd.lockout;
    a.extra = 0;
    a.variant = cd.subtype;
    a.element = cd.element as u8 | content.rules.family_elements(cd.family).0;
    a.charged = 0;
    cd.action
}

/// `sub_800EF34`: the damage bonus MegaMan's form gives a chip, then the
/// aura bonus (`sub_800F1DC`).
fn chip_bonus(b: &Battle, r: ObjectRef, chip: ChipId) -> u16 {
    let s = stats(b, r);
    if s.navi != Navi::MEGAMAN {
        panic!("link navi chip bonuses (sub_800F09E) are not implemented yet");
    }
    if chip == crate::hand::NO_CHIP {
        return 0;
    }
    let cd = b.content.chip(chip);
    let damaging = cd.flags.has(ChipFlags::HAS_DAMAGE);
    let family_bonus = |family: ChipFamily, bonus: u16| (deals_damage(cd.flags) && cd.family == family).then_some(bonus);
    let form_bonus = match s.form.0 {
        1 | 0x0D => family_bonus(ChipFamily::Fire, 50),
        2 | 0x0E => family_bonus(ChipFamily::Elec, 50),
        3 | 0x0F => family_bonus(ChipFamily::Sword, 50),
        8 | 0x14 => family_bonus(ChipFamily::Wind, 10),
        // Unlike the others, this one also boosts dimming chips.
        4 | 0x10 => (damaging && cd.family == ChipFamily::Cursor).then_some(30),
        9 | 0x15 => family_bonus(ChipFamily::Break, 10),
        _ => None,
    };
    let beast_bonus = || {
        let beast = (0x0B..=0x16).contains(&s.form.0);
        (beast && deals_damage(cd.flags) && cd.family == ChipFamily::Null && super::battle_mode(b) != 1).then_some(30)
    };
    let bonus = form_bonus.or_else(beast_bonus).unwrap_or(0);
    bonus + aura_bonus(b, r, chip)
}

/// `sub_800F1DC`: StreamHd and the AuraHed chips hit harder while the
/// user's barrier holds.
fn aura_bonus(b: &Battle, r: ObjectRef, chip: ChipId) -> u16 {
    if chip != 0x150 && !(0x5F..=0x61).contains(&chip) {
        return 0;
    }
    let c = super::coll(b, r);
    let up = (1..0x10).contains(&c.barrier) && (c.barrier != 8 || c.barrier_hp != 0);
    if up { 50 } else { 0 }
}

/// The sound of a charge bonus or a doubled chip (`SOUND_HIT_87`).
const BOOST_SOUND: u16 = 0x87;

/// `sub_8012C7C`: what a charged use adds to the chip's damage in the
/// ElecCross forms (the paralyzing flag) and the SlashCross forms (40).
fn charge_bonus(b: &Battle, r: ObjectRef, charged: u8) -> u16 {
    if charged == 0 {
        return 0;
    }
    match stats(b, r).form.0 {
        2 | 0x0E => damage_flags::PARALYZE,
        3 | 0x0F => 0x28,
        _ => 0,
    }
}

/// Why a chip's damage doubled (`sub_8012A38`'s second result).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Boost {
    /// Full Synchro (1, spent by the use).
    FullSynchro,
    /// Anger (2, spent by the use).
    Anger,
    /// A charged chip of the element or family its cross favors (4 wood,
    /// 6 aqua or sword).
    Element,
    /// A Null-family chip in Beast Over (4).
    BeastOver,
}

/// `sub_8012A38`: whether the use doubles the chip's damage.
fn double_damage(b: &Battle, r: ObjectRef, chip: ChipId, damage: u16, charged: u8) -> (u16, Option<Boost>) {
    let cd = b.content.chip(chip);
    if !cd.flags.has(ChipFlags::HAS_DAMAGE) {
        return (damage, None);
    }
    let boost = match emotion(b, b.objects.get(r).alliance) {
        Emotion::FullSynchro => Some(Boost::FullSynchro),
        Emotion::Angry => Some(Boost::Anger),
        _ => cross_boost(b, r, chip, charged),
    };
    let damage = if boost.is_some() { damage | damage_flags::DOUBLE } else { damage };
    (damage, boost)
}

/// `sub_8012AFA`, `sub_8012B4E`, `sub_8012BA2` and `sub_8012ABC`, in that
/// order: a charged wood chip for TomahawkMan or TomahawkCross, a charged
/// aqua chip for SpoutMan or SpoutCross, a charged sword-family chip for
/// navi 0x0B, and a Null-family chip in Beast Over (outside battle mode 1).
/// "Charged" is the use's charge argument, or a full A charge held
/// (charge level 2 from A).
fn cross_boost(b: &Battle, r: ObjectRef, chip: ChipId, charged: u8) -> Option<Boost> {
    let s = stats(b, r);
    let (navi, form) = (s.navi.0, s.form.0);
    let cd = b.content.chip(chip);
    let damaging = cd.flags.has(ChipFlags::HAS_DAMAGE) && !cd.flags.has(ChipFlags::DIMMING);
    let a = ai(b, r);
    let charged = charged != 0 || (a.charge_level == 2 && a.charge_source == 1);
    let wood = (navi == 7 || matches!(form, 7 | 0x13)) && damaging && cd.element == Element::Wood && charged;
    let aqua = (navi == 6 || matches!(form, 6 | 0x12)) && damaging && cd.element == Element::Aqua && charged;
    let sword = navi == 0x0B && damaging && cd.family == ChipFamily::Sword && charged;
    let beast_over =
        super::battle_mode(b) != 1 && matches!(form, 0x17 | 0x18) && damaging && cd.family == ChipFamily::Null;
    if wood || aqua || sword {
        Some(Boost::Element)
    } else if beast_over {
        Some(Boost::BeastOver)
    } else {
        None
    }
}

/// The heal some uses give: the NaviCust chip-recovery stat, plus a
/// twentieth of the base HP for aqua chips in the aqua crosses.
fn heal_on_use(b: &Battle, r: ObjectRef, chip: ChipId) {
    let s = stats(b, r);
    let cd = b.content.chip(chip);
    let cross = matches!(s.form.0, 6 | 0x12) && cd.element == Element::Aqua && !cd.flags.has(ChipFlags::DIMMING);
    let heal = if cross { s.max_base_hp.div_ceil(0x14) } else { 0 };
    if s.chip_recovery.wrapping_add(heal) != 0 {
        panic!("healing on chip use (sub_800E2FC) is not implemented yet");
    }
}
