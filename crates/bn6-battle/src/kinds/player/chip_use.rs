//! Using the next chip in the hand (`sub_800FB54` and `sub_80127C0`):
//! read the hand entry, fill the attack variables from the chip data and
//! the hand, apply the damage modifiers, and start the chip's action. See
//! chips.md §2.6.4 and §2.7.

use super::{Emotion, ai, ai_mut, emotion, flag1, form_of, navi_of, navi_record, set_attack, set_mood, stats};
use crate::actor::{ActorType, AttackVars, request};
use crate::battle::Battle;
use crate::collision::f1;
use crate::content::{ChipData, ChipFamily, ChipFlags, ChipTraits, Content, Element};
use crate::object::{ObjectRef, PanelPos, Vec3};
use crate::setup::Navi;
use bn6_content_api::ChipHandle;

/// Damage-word flag bits a chip use can add (see `oBattleObject_Damage`).
mod damage_flags {
    /// Double damage (Full Synchro, anger, some crosses).
    pub const DOUBLE: u16 = 0x8000;
    /// Paralyzes (a folded WhiCapsl, an ElecCross charged chip).
    pub const PARALYZE: u16 = 0x4000;
    /// Uninstalls (a folded Uninstll).
    pub const UNINSTALL: u16 = 0x2000;
    /// Erases a cross (EraseCross family chips).
    pub const ERASE_CROSS: u16 = 0x1000;
}

/// The Beast forms' claw, the action weapon routine 0x1E names.

/// The chip-use sound of a damage bonus (`SOUND_HIT_87`).
const BONUS_SOUND: crate::sound::SoundId = crate::sound::SoundId(0x87);

/// `sub_800FB54`: when a chip request is up (and not sliding), take the
/// next chip and start its action. Returns the attack's chip (the game
/// returns 0xFFFF for no use), itself none for the game's 0.
pub(super) fn use_chip(b: &mut Battle, r: ObjectRef) -> Option<Option<ChipHandle>> {
    let requested = ai(b, r).requests & (request::CHIP | request::CHARGED_CHIP | request::ALT_CHIP);
    if flag1(b, r) & f1::SLIDING != 0 || requested == 0 {
        return None;
    }
    let mut charge = 0;
    if requested & request::CHARGED_CHIP != 0 {
        // The form's A-charge routine decides what the charged chip does;
        // for the Null family the attack's chip id is cleared.
        let chip = hand_entry(b, r).chip;
        let null = super::null_family(b, chip);
        let routine = if null {
            ai_mut(b, r).attack.chip = None;
            ai(b, r).alt_a_charge
        } else {
            ai(b, r).a_charge
        };
        use crate::content::ChargedChip;
        match routine.map(|w| b.content.weapon(w).charged_chip) {
            // sub_8012CB2: GroundCross's rocks fall first; its leftover r0
            // (2 after a barrage, 0 without targets) marks the chip.
            Some(Some(ChargedChip::RockBarrage)) => charge = rock_barrage(b, r),
            // No routine: the chip is used with the register that held its
            // family byte as the argument (cleared with the chip id for the
            // Null family).
            None if null => {}
            None => {
                let Some(chip) = chip else {
                    panic!("a charged use of the empty hand without a charge routine needs its family byte (sub_800FB54)");
                };
                charge = b.content.chip(chip).family as u8;
            }
            // These forms' charged chips are the chip with a bonus.
            Some(Some(ChargedChip::Bonus)) => charge = 1,
            // The rest run a weapon routine instead of the chip.
            Some(None) => {
                let weapon = routine.expect("a weapon");
                ai_mut(b, r).attack.charged = 0;
                let action = super::idle::weapon_routine(b, r, weapon);
                set_attack(b, r, action, 2);
                let form = form_of(b, r);
                // The Beast forms' claw (weapon 0x1E) and SlashCross Beast's
                // charged sword run inside the Beast Out rush.
                use crate::content::ActionRole;
                let runs = |role| matches!(action, super::NaviAction::Content(h) if b.content.defs.roles.is_action(role, h));
                if runs(ActionRole::BeastClaw) || (runs(ActionRole::ChargedSword) && form.0 == 0x0F) {
                    ai_mut(b, r).attack.beast_lockon = 1;
                }
                ai_mut(b, r).requests &= !(request::CHIP | request::CHARGED_CHIP | request::ALT_CHIP);
                return Some(ai(b, r).attack.chip);
            }
        }
    }
    let action = prepare(b, r, charge);
    set_attack(b, r, action, 2);
    let form = form_of(b, r);
    let content = b.content.clone();
    let a = &mut ai_mut(b, r).attack;
    if a.special_source != 0 || form.is_beast() {
        a.beast_lockon = content.chip_field(a.chip).beast_lockon as u8;
    }
    ai_mut(b, r).requests &= !(request::CHIP | request::CHARGED_CHIP | request::ALT_CHIP);
    Some(ai(b, r).attack.chip)
}

/// `sub_800FC30`: the Beast Out rush chains the next chip, starting its
/// action (inside the rush again). Not the chips with the `no_chain` trait
/// (the variable swords), dimming chips, or an empty hand. True if it did.
pub(super) fn chain_next_chip(b: &mut Battle, r: ObjectRef) -> bool {
    let Some(chip) = hand_entry(b, r).chip else { return false };
    if b.content.chip(chip).traits.has(ChipTraits::NO_CHAIN) {
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

/// The hand entry at the cursor (`sub_800EDD0`).
#[derive(Clone, Copy, Debug)]
struct HandEntry {
    /// None: the empty hand (the game's 0xFFFF).
    chip: Option<ChipHandle>,
    damage: u16,
    /// Atk+ and charge bonuses plus any form or aura bonus.
    extra: u16,
    /// Modifier flags folded into the entry (bit 1 paralyze, bit 2
    /// uninstall).
    modifiers: u8,
}

/// `sub_800EDD0`: a player's next chip from its side's hand; anything else
/// uses the chip it carries, with no damage, no bonus, and as modifiers the
/// low byte the caller left in r4 (`sub_800FB54`'s chip requests).
fn hand_entry(b: &Battle, r: ObjectRef) -> HandEntry {
    let o = b.objects.get(r);
    if navi_record(b, r).actor_type != ActorType::Player {
        let requests = ai(b, r).requests & (request::CHIP | request::CHARGED_CHIP);
        return HandEntry { chip: carried_chip(b, r), damage: 0, extra: 0, modifiers: requests as u8 };
    }
    let hand = &b.hands[o.alliance as usize];
    let i = hand.cursor as usize;
    let chip = hand.ids[i];
    let extra = hand.attack_bonus[i].wrapping_add(chip_bonus(b, r, chip)).wrapping_add(hand.charge_bonus[i]);
    HandEntry { chip, damage: hand.damage[i], extra, modifiers: hand.modifiers[i] }
}

/// The chip an object other than a player carries: its zeroed chip field,
/// the zeroed chip (nothing else sets it).
fn carried_chip(b: &Battle, r: ObjectRef) -> Option<ChipHandle> {
    b.objects.get(r).chip.or_else(|| b.content.zeroed_chip())
}

/// The record of a hand entry's chip; the empty hand's (0xFFFF) is past
/// the chip table.
fn entry_record(content: &Content, chip: Option<ChipHandle>) -> &ChipData {
    let Some(chip) = chip else { panic!("the empty hand's chip (0xFFFF) reads past the chip table (sub_80127C0)") };
    content.chip(chip)
}

/// `sub_80127C0(charge)`: fill the attack variables for the next chip (or
/// the dark chip's substitute) and name its action. `charge` marks the
/// attack as charged (AIAttackVars+4): 1 for the forms' charged chips, 2
/// after GroundCross's rocks.
pub(super) fn prepare(b: &mut Battle, r: ObjectRef, charge: u8) -> super::NaviAction {
    let slot_in = ai(b, r).requests & request::ALT_CHIP != 0;
    prepare_from(b, r, charge, slot_in)
}

/// `sub_80127C0(0)` as the counter cut-in calls it (`sub_8017AB4`): into
/// a scratch copy of the attack variables (a 0x50-byte stack buffer in the
/// game, r7), so the navi's own attack and action are untouched. The
/// cut-in's r4 is the AI index times 4, never the slot-in bit, so the
/// chip always comes from the hand. Every other side effect of a use still
/// happens (the Full Synchro and anger doubling, the heal on use, the navi
/// chip count, a dark chip's cost). Returns the chip's action and the
/// filled variables.
pub(super) fn prepare_detached(b: &mut Battle, r: ObjectRef) -> (super::NaviAction, AttackVars) {
    let own = ai(b, r).attack.clone();
    let action = prepare_from(b, r, 0, false);
    let scratch = std::mem::replace(&mut ai_mut(b, r).attack, own);
    (action, scratch)
}

/// `sub_80127C0`, reading the slot-in chip (`sub_800EE26`) when `slot_in`
/// (the caller's r4 bit 0x10000), else the hand (`sub_800EDD0`).
fn prepare_from(b: &mut Battle, r: ObjectRef, charge: u8, slot_in: bool) -> super::NaviAction {
    if slot_in {
        ai_mut(b, r).attack.special_source = 1;
    }
    let mut e = if slot_in { slot_in_entry(b, r) } else { hand_entry(b, r) };
    if let Some(sub) = dark_substitute(b, r, e.chip) {
        e = sub;
    }
    let content = b.content.clone();
    let cd = entry_record(&content, e.chip);
    load_attack(b, r, e.chip);
    let a = &mut ai_mut(b, r).attack;
    a.charged = charge;
    // The hand's damage replaces the chip data's.
    a.damage = e.damage;
    // sub_8012C7C: the Cross's bonus on a charged chip.
    let bonus = charged_bonus(b, r, charge);
    if bonus != 0 {
        let a = &mut ai_mut(b, r).attack;
        let add = if bonus == 0xFF { a.damage } else { bonus };
        a.damage = a.damage.wrapping_add(add);
        b.play_sound(BONUS_SOUND);
    }
    ai_mut(b, r).attack.extra = e.extra;
    let (damage, boost) = double_damage(b, r, e.chip, ai(b, r).attack.damage, charge);
    ai_mut(b, r).attack.damage = damage;
    let side = b.objects.get(r).alliance;
    match boost {
        Some(Boost::FullSynchro) => {
            set_mood(b, side, 0x80);
            b.play_sound(BONUS_SOUND);
        }
        Some(Boost::Anger) => {
            super::status::end_anger(b, r);
            b.play_sound(BONUS_SOUND);
        }
        Some(Boost::Cross | Boost::BeastOver) | None => {}
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
    if deals_damage(cd.flags) && cd.family == ChipFamily::Null && matches!(form_of(b, r).0, 4 | 0x10) {
        damage |= damage_flags::ERASE_CROSS;
    }
    ai_mut(b, r).attack.damage = damage;
    heal_on_use(b, r, e.chip);
    if cd.flags.has(ChipFlags::NAVI) {
        b.bump_side_stat(side, 6, 1);
    }
    dark_chip_side_effect(b, r, e.chip);
    chip_action(b, r, e.chip)
}

/// The action chip `chip` starts by its usage (none: the zeroed chip,
/// which a zeroed chip field reads): its own action, or the engine's action
/// for its kind of use (which calls its hook); an instant chip's effect
/// goes into the attack.
pub(super) fn chip_action(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) -> super::NaviAction {
    use crate::content::ChipUsage;
    use super::actions::instant::Effect;
    use super::{EngineAction as E, NaviAction as A};
    let content = b.content.clone();
    let chip = content.chip_or_zeroed(chip);
    match content.defs.chip(chip).usage {
        ChipUsage::Action(h) => A::Content(h),
        ChipUsage::Dimming(_) => A::Engine(E::DimmingChip),
        ChipUsage::Navi(_) => A::Engine(E::NaviChip),
        ChipUsage::Instant(f) => {
            ai_mut(b, r).attack.instant = Some(Effect::Runs(f));
            A::Engine(E::InstantChip)
        }
    }
}

/// A chip that deals damage and isn't a dimming chip (the condition every
/// damage bonus shares).
fn deals_damage(flags: ChipFlags) -> bool {
    flags.has(ChipFlags::HAS_DAMAGE) && !flags.has(ChipFlags::DIMMING)
}

/// `sub_8010D58`: a dark chip (one with a substitute) costs a bug frag; with
/// none left the player gets the substitute instead (the chip's own:
/// `off_8010D84`, Sword, Thunder, Recov10, Invisibl and Atk+10 for the five
/// dark chips), through `sub_800EF02`, with its own damage and bonus and no
/// modifiers.
fn dark_substitute(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) -> Option<HandEntry> {
    // (The empty hand's chip reads past the chip table, as for the record.)
    entry_record(&b.content, chip);
    let sub = b.content.chip_links(chip.expect("a chip")).dark_substitute?;
    let side = b.objects.get(r).alliance as usize;
    if b.bug_frags[side] >= 1 {
        // sub_800F4B2 (the local player's save loses one too).
        b.bug_frags[side] -= 1;
        return None;
    }
    let chip = Some(sub);
    // sub_800EF02: anything but a player keeps the chip it carries.
    if navi_record(b, r).actor_type != ActorType::Player {
        return Some(HandEntry { chip: carried_chip(b, r), damage: 0, extra: 0, modifiers: 0 });
    }
    let damage = crate::hand::chip_damage(b, chip, b.objects.get(r).alliance);
    let extra = chip_bonus(b, r, chip);
    Some(HandEntry { chip, damage, extra, modifiers: 0 })
}

/// `sub_800B79A`: the dark chips worsen the user's HP bug (NaviStats+0x18,
/// at most 7) by their `hp_bug`: DrkSword +2, DarkThnd +1, DrkRecov to 7,
/// DarkInvs nothing, DarkPlus +4.
fn dark_chip_side_effect(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) {
    let cost = chip.map_or(0, |h| b.content.chip(h).hp_bug);
    if cost == 0 {
        return;
    }
    let s = &mut super::stats_mut(b, r).bugs.hp_drain;
    *s = s.saturating_add(cost).min(7);
}

/// `sub_800EE26`: the battle flag 0x40 mode's special chip (the side
/// state's), paid for from the side's gauge (`sub_800EE98`), with the
/// side's stored bonuses spent.
fn slot_in_entry(b: &mut Battle, r: ObjectRef) -> HandEntry {
    let side = b.objects.get(r).alliance as usize;
    if navi_record(b, r).actor_type != ActorType::Player {
        return HandEntry { chip: carried_chip(b, r), damage: 0, extra: 0, modifiers: 0 };
    }
    // The zeroed field is the zeroed chip.
    let chip = b.sides[side].special_chip.or_else(|| b.content.zeroed_chip());
    pay_for_special_chip(b, side, chip);
    let extra = chip_bonus(b, r, chip);
    let content = b.content.clone();
    let cd = entry_record(&content, chip);
    ai_mut(b, r).attack.beast_lockon = cd.beast_lockon as u8;
    let damage = crate::hand::chip_damage(b, chip, side as u8);
    let mut extra = extra;
    let s = &mut b.sides[side];
    if cd.flags.has(ChipFlags::HAS_DAMAGE) {
        extra = extra.wrapping_add(std::mem::take(&mut s.special_attack_bonus));
    }
    if cd.flags.has(ChipFlags::NAVI) {
        extra = extra.wrapping_add(std::mem::take(&mut s.special_navi_bonus));
    }
    HandEntry { chip, damage, extra, modifiers: 0 }
}

/// `sub_800EE98`: the special chip costs gauge by its class (0x1500
/// standard, 0x2A00 mega, 0x4000 giga; the rest cost nothing), unless its
/// second flag byte's top bit is set.
fn pay_for_special_chip(b: &mut Battle, side: usize, chip: Option<ChipHandle>) {
    // (sub_802E830 and the local player's HUD and sound.)
    let cd = entry_record(&b.content, chip);
    if cd.extra_flags.0 & 0x80 != 0 {
        return;
    }
    let cost: u16 = match cd.class {
        crate::content::ChipClass::Standard => 0x1500,
        crate::content::ChipClass::Mega => 0x2A00,
        crate::content::ChipClass::Giga => 0x4000,
        _ => 0xFFFF,
    };
    // (The cost is also stored at word_200F3C4 indexed by twice the side
    // state's address: a store into video memory.)
    let s = &mut b.sides[side];
    s.gauge = s.gauge.saturating_sub(cost);
}

/// `sub_80126E4`: the attack variables from the chip data (its action is
/// [`chip_action`]'s). (The game also counts the use per side, for a
/// report only the battle-flag 0x40 mode reads.)
fn load_attack(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) {
    let content = b.content.clone();
    let cd = entry_record(&content, chip);
    let side = b.objects.get(r).alliance;
    let damage = crate::hand::chip_damage(b, chip, side);
    let a = &mut ai_mut(b, r).attack;
    a.chip = chip;
    // (The original copies the record's subtype and parameter bytes: a
    // chip has neither here; what its action needs is its definition's.)
    a.params = [0; 4];
    a.damage = damage;
    a.hit_param = cd.hit_param as u16;
    a.lockout = cd.lockout;
    a.extra = 0;
    a.variant = 0;
    a.element = cd.element as u8 | content.rules.family_elements(cd.family).0;
    a.charged = 0;
}

/// `sub_800EF34`: the damage bonus MegaMan's form gives a chip (a link
/// navi's own, `sub_800F09E`), then the aura bonus (`sub_800F1DC`).
fn chip_bonus(b: &Battle, r: ObjectRef, chip: Option<ChipHandle>) -> u16 {
    let bonus = if navi_of(b, r) != Navi::MEGAMAN { link_navi_bonus(b, r, chip) } else { form_bonus(b, r, chip) };
    bonus + aura_bonus(b, r, chip)
}

/// `sub_800EF34`'s MegaMan part: by form, a family's damaging chips get
/// more (the Beast forms' Null chips outside battle mode 1).
fn form_bonus(b: &Battle, r: ObjectRef, chip: Option<ChipHandle>) -> u16 {
    let form = form_of(b, r);
    let Some(chip) = chip else { return 0 };
    let cd = b.content.chip(chip);
    let damaging = cd.flags.has(ChipFlags::HAS_DAMAGE);
    let family_bonus = |family: ChipFamily, bonus: u16| (deals_damage(cd.flags) && cd.family == family).then_some(bonus);
    let form_bonus = match form.0 {
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
        let beast = (0x0B..=0x16).contains(&form.0);
        (beast && deals_damage(cd.flags) && cd.family == ChipFamily::Null && super::battle_mode(b) != 1).then_some(30)
    };
    form_bonus.or_else(beast_bonus).unwrap_or(0)
}

/// `sub_800F09E`: a link navi's bonus on its family's damaging chips, by
/// its level (`sub_800F49E`; 0xFF: none).
fn link_navi_bonus(b: &Battle, r: ObjectRef, chip: Option<ChipHandle>) -> u16 {
    let Some(chip) = chip else { return 0 };
    let s = stats(b, r);
    let Some(bonus) = &b.content.navi(s.navi).chip_bonus else { return 0 };
    let cd = b.content.chip(chip);
    let fits = cd.flags.has(ChipFlags::HAS_DAMAGE)
        && cd.family == bonus.family
        && (bonus.dimming_chips || !cd.flags.has(ChipFlags::DIMMING));
    let level = b.navi_levels[b.objects.get(r).alliance as usize];
    if !fits || level == 0xFF {
        return 0;
    }
    *bonus.by_level.get(level as usize).unwrap_or_else(|| panic!("link navi level {level} reads past the bonus table (sub_800F09E)"))
        as u16
}

/// `sub_800F1DC`: StreamHd and the AuraHed chips hit harder while the
/// user's barrier holds.
fn aura_bonus(b: &Battle, r: ObjectRef, chip: Option<ChipHandle>) -> u16 {
    if !chip.is_some_and(|h| b.content.chip(h).traits.has(ChipTraits::AURA_BONUS)) {
        return 0;
    }
    let c = super::coll(b, r);
    let up = (1..0x10).contains(&c.barrier) && (c.barrier != 8 || c.barrier_hp != 0);
    if up { 50 } else { 0 }
}

/// `sub_8012C7C`: a charged chip's bonus in ElecCross (it paralyzes) and
/// SlashCross (+40), and their Beasts.
fn charged_bonus(b: &Battle, r: ObjectRef, charge: u8) -> u16 {
    if charge == 0 {
        return 0;
    }
    match form_of(b, r).0 {
        2 | 0x0E => damage_flags::PARALYZE,
        3 | 0x0F => 0x28,
        _ => 0,
    }
}

/// Why a chip's damage doubled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Boost {
    /// Full Synchro (spent by the use).
    FullSynchro,
    /// Anger (spent by the use).
    Anger,
    /// A Cross's element on a charged (or fully A-charged) chip.
    Cross,
    /// Beast Over's Null chips.
    BeastOver,
}

/// `sub_8012A38`: whether the use doubles the chip's damage.
fn double_damage(b: &Battle, r: ObjectRef, chip: Option<ChipHandle>, damage: u16, charge: u8) -> (u16, Option<Boost>) {
    let cd = entry_record(&b.content, chip);
    if !cd.flags.has(ChipFlags::HAS_DAMAGE) {
        return (damage, None);
    }
    let boost = match emotion(b, b.objects.get(r).alliance) {
        Emotion::FullSynchro => Some(Boost::FullSynchro),
        Emotion::Angry => Some(Boost::Anger),
        _ if cross_doubles(b, r, chip, charge) => Some(Boost::Cross),
        _ if beast_over_doubles(b, r, chip) => Some(Boost::BeastOver),
        _ => None,
    };
    let damage = if boost.is_some() { damage | damage_flags::DOUBLE } else { damage };
    (damage, boost)
}

/// `sub_8012AFA`, `sub_8012B4E`, `sub_8012BA2`: TomahawkMan's element
/// (Wood) in TomahawkCross, SpoutMan's (Aqua) in SpoutCross, and navi
/// 0xB's family 5, doubled when the chip is charged or the A charge is
/// full.
fn cross_doubles(b: &Battle, r: ObjectRef, chip: Option<ChipHandle>, charge: u8) -> bool {
    let cd = entry_record(&b.content, chip);
    let (navi, form) = (navi_of(b, r).0, form_of(b, r).0);
    let fits = if navi == 7 || matches!(form, 7 | 0x13) {
        cd.element == Element::Wood
    } else if navi == 6 || matches!(form, 6 | 0x12) {
        cd.element == Element::Aqua
    } else if navi == 0x0B {
        cd.family == ChipFamily::from_number(5).expect("family 5")
    } else {
        return false;
    };
    if !fits || !deals_damage(cd.flags) {
        return false;
    }
    let a = ai(b, r);
    charge != 0 || (a.charge_level == 2 && a.charge_source == 1)
}

/// `sub_8012ABC`: Beast Over doubles its Null chips (not in battle mode 1).
fn beast_over_doubles(b: &Battle, r: ObjectRef, chip: Option<ChipHandle>) -> bool {
    if super::battle_mode(b) == 1 || !matches!(form_of(b, r).0, 0x17 | 0x18) {
        return false;
    }
    let cd = entry_record(&b.content, chip);
    deals_damage(cd.flags) && cd.family == ChipFamily::Null
}

/// The heal some uses give (`sub_800E2FC(heal, 0)`): the NaviCust
/// chip-recovery stat, plus a twentieth of the base HP (rounded up) for
/// non-dimming aqua chips in the aqua crosses; a recovery effect (#6) and
/// its sound.
fn heal_on_use(b: &mut Battle, r: ObjectRef, chip: Option<ChipHandle>) {
    let s = *stats(b, r);
    let cd = entry_record(&b.content, chip);
    let cross = matches!(form_of(b, r).0, 6 | 0x12) && cd.element == Element::Aqua && !cd.flags.has(ChipFlags::DIMMING);
    let heal = if cross { (s.max_base_hp as u32 + 0x13) / 0x14 } else { 0 };
    let total = s.chip_recovery as u32 + heal;
    if total as u16 == 0 {
        return;
    }
    super::intake::add_hp(b, r, total);
    let pos = b.objects.get(r).pos;
    crate::kinds::effect::spawn(b, pos, 6, 0, 0, 0);
    b.play_sound(crate::sound::SoundId(0x8A));
}

// ---- GroundCross's charged chip ----------------------------------------------

/// `sub_8012CB2`: rocks fall on up to three of the opponents (by panel, a
/// shared panel moved to another free one on their side), for 30 plus 20
/// per buster level up to 5, then the camera shakes (20 ticks at magnitude
/// 2). Returns what the game
/// leaves in r0: 2 after a barrage, 0 without targets.
fn rock_barrage(b: &mut Battle, r: ObjectRef) -> u8 {
    let side = b.objects.get(r).alliance;
    let opp = side ^ 1;
    // object_getEnemyByNameRange: the other side's viruses (NameID
    // 0..=0xBA) then its navis (0x100..=0x1C3), in actor-list order.
    let actors = b.round.alive_actors[opp as usize];
    let mut targets: Vec<ObjectRef> =
        actors.iter().flatten().copied().filter(|&o| b.objects.get(o).name_id <= 0xBA).collect();
    targets.extend(actors.iter().flatten().copied().filter(|&o| (0x100..=0x1C3).contains(&b.objects.get(o).name_id)));
    if targets.is_empty() {
        return 0;
    }
    let at: Vec<PanelPos> = targets.iter().map(|&o| b.objects.get(o).panel).collect();
    // sub_8012D24: always three: one target thrice, two with the second
    // twice, three each once; with four the list starts at the fourth
    // (and runs two bytes into the stack frame above, a side of the game
    // only a four-actor side reaches), which is repeated.
    let mut panels = match at.len() {
        1 => [at[0]; 3],
        2 => [at[0], at[1], at[1]],
        3 => [at[0], at[1], at[2]],
        _ => [at[3]; 3],
    };
    // sub_8012D70: a panel shared with an earlier one moves elsewhere.
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        if panels[i] == panels[j] {
            let p = panels[i];
            b.reserve_panel(r, p.x, p.y);
            panels[j] = elsewhere(b, r);
            b.unreserve_panel(r, p.x, p.y);
        }
    }
    let damage = 0x1E + 0x14 * super::idle::buster_damage(b, r).min(5);
    let flip = b.objects.get(r).flip;
    for (n, p) in panels.iter().enumerate() {
        // sub_80C7F20: Param1 counts down 3, 2, 1.
        let params = [3 - n as u8, 0, 0, 0];
        let kind = b.content.defs.roles.kind(crate::content::KindRole::FallingRock);
        if let Some(rock) = crate::kinds::spawn(b, kind, bn6_content_api::SpawnAt::AfterCurrent, Vec3::default(), params) {
            let o = b.objects.get_mut(rock);
            o.panel = *p;
            o.element = 0x10;
            o.damage = damage;
            o.stamina = 0x8A;
            o.alliance = side;
            o.flip = flip;
        }
    }
    // camera_initShakeEffect_80302a8(2, 0x14).
    b.shake_camera(2, 0x14);
    2
}

/// `sub_8012DB8`: a random free panel on the other side
/// (`sub_8015C94` with `byte_8012DD4`), or none (x 0).
fn elsewhere(b: &mut Battle, r: ObjectRef) -> PanelPos {
    let o = b.objects.get(r);
    let (own, alliance) = (o.panel, o.alliance);
    // Side 0 looks for side 1's panels, unreserved; side 1 for its own
    // opponent's (no alliance bit).
    let (require, forbid) = if alliance == 0 { (0x20, 0x80) } else { (0, 0xA0) };
    let mut found = Vec::new();
    // object_getPanelsExceptCurrentFiltered: rows 3..1, columns 6..1.
    for y in (1..=3).rev() {
        for x in (1..=6).rev() {
            let f = b.field.flags(x, y);
            if f & forbid == 0 && f & require == require && !(x == own.x && y == own.y) {
                found.push(PanelPos { x, y });
            }
        }
    }
    if found.is_empty() {
        // sub_8015C94 falls back to the user's own panel.
        return own;
    }
    let i = b.rng.next_positive() % found.len() as u32;
    found[i as usize]
}
