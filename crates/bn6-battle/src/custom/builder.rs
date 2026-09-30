//! Building a player's hand from their selection (`sub_8029110`): the
//! invalid-chip rule, Program Advances, and the modifier chips that fold
//! into the chip before them. See docs/engine/custom-screen.md §5.

use super::folder::FolderChip;
use super::library::Library;
use crate::content::{ChipClass, ChipCode, ChipFlags, ChipId, ChipModifier, PaRecipe};
use crate::hand::{ChipHand, NO_CHIP};

/// `ChipHand::modifiers` bits.
pub mod modifier_bits {
    /// The chip is the folder's Regular chip.
    pub const REGULAR: u8 = 0x01;
    /// WhiCapsl folded in: the chip paralyzes.
    pub const PARALYZE: u8 = 0x02;
    /// Uninstll folded in: the chip uninstalls.
    pub const UNINSTALL: u8 = 0x04;
}

/// Program Advances a player has formed this round (once each), by
/// `result - 0x140`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ProgramAdvancesUsed(pub u32);

impl ProgramAdvancesUsed {
    const FIRST: ChipId = 0x140;

    /// `sub_8029652`: spend a Program Advance; false if it was spent.
    fn spend(&mut self, result: ChipId) -> bool {
        let bit = 1 << (result - Self::FIRST);
        let fresh = self.0 & bit == 0;
        self.0 |= bit;
        fresh
    }
}

/// A picked chip as the builder takes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pick {
    /// The chip as it counts (after `screen::checked`).
    pub chip: FolderChip,
    pub regular: bool,
}

/// One entry of the hand being built (the work area `dword_2033000`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Entry {
    id: ChipId,
    damage: u16,
    bonus: u16,
    modifiers: u8,
}

const EMPTY: Entry = Entry { id: NO_CHIP, damage: 0, bonus: 0, modifiers: 0 };

/// What the builder made.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Built {
    pub hand: ChipHand,
    /// The Program Advance that formed (its animation lists the picks).
    pub program_advance: Option<FormedAdvance>,
}

/// A Program Advance formed at OK: what its animation shows
/// (`sub_802B6F2`'s arguments).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormedAdvance {
    pub chip: ChipId,
    /// Chips picked (all listed).
    pub picks: u8,
    /// Where the recipe's chips start among them, and how many.
    pub start: u8,
    pub len: u8,
}

/// Build a hand from the picked chips, in pick order (`sub_8029110`),
/// with each chip's damage from `damage`. `turn`: the screen's number in
/// the round (1 = first).
pub fn build(
    picks: &[Pick],
    turn: u8,
    pa_used: &mut ProgramAdvancesUsed,
    library: &dyn Library,
    damage: impl Fn(ChipId) -> u16,
) -> Built {
    let mut entries = [EMPTY; 6];
    let mut raw = [NO_CHIP; 6];
    for (i, p) in picks.iter().enumerate() {
        raw[i] = p.chip.packed();
        entries[i] = Entry {
            id: p.chip.id,
            damage: damage(p.chip.id),
            bonus: 0,
            modifiers: if p.regular { modifier_bits::REGULAR } else { 0 },
        };
    }
    let mut program_advance = None;
    if !picks.is_empty() {
        if let Some((result, start, len)) = find_program_advance(&raw[..picks.len()], pa_used, library) {
            // sub_80292CC: the recipe's chips become the Program Advance;
            // it is the Regular chip if one of them was.
            let regular = entries[start..start + len].iter().fold(0, |m, e| m | (e.modifiers & modifier_bits::REGULAR));
            entries[start] = Entry { id: result, damage: damage(result), bonus: 0, modifiers: entries[start].modifiers | regular };
            // The entries after the recipe move up behind it, up to and
            // with the end marker; what lay past it stays.
            shift_up(&mut entries, start + 1, start + len);
            program_advance =
                Some(FormedAdvance { chip: result, picks: picks.len() as u8, start: start as u8, len: len as u8 });
        }
        fold_modifiers(&mut entries, library);
    }
    let hand = ChipHand {
        cursor: 0,
        ids: entries.map(|e| e.id),
        damage: entries.map(|e| e.damage),
        attack_bonus: entries.map(|e| e.bonus),
        charge_bonus: [0; 6],
        selection: raw,
        turn: [turn.wrapping_sub(1); 6],
        modifiers: entries.map(|e| e.modifiers),
    };
    Built { hand, program_advance }
}

/// `sub_8029520`: the first Program Advance in the selection, trying each
/// start position in turn and the recipes in table order; one already
/// formed this round is passed over. Returns (result, start, length).
fn find_program_advance(raw: &[u16], used: &mut ProgramAdvancesUsed, library: &dyn Library) -> Option<(ChipId, usize, usize)> {
    let chips: Vec<FolderChip> = raw.iter().map(|&v| FolderChip::from_packed(v)).collect();
    for start in 0..chips.len().saturating_sub(2) {
        let rest = &chips[start..];
        for pa in library.program_advances() {
            let len = pa.recipe.len();
            if rest.len() < len || !recipe_matches(&pa.recipe, &rest[..len]) {
                continue;
            }
            if used.spend(pa.result) {
                return Some((pa.result, start, len));
            }
        }
    }
    None
}

fn recipe_matches(recipe: &PaRecipe, chips: &[FolderChip]) -> bool {
    match *recipe {
        PaRecipe::Sequence(ref ids) => chips.iter().zip(ids).all(|(c, &id)| c.id == id),
        PaRecipe::CodeRun { chip, .. } => chips.iter().all(|c| c.id == chip) && codes_run(chips),
    }
}

/// `sub_80295C8`'s code test: the codes go up by one in pick order, and
/// at most one `*` stands in for the code it needs.
fn codes_run(chips: &[FolderChip]) -> bool {
    let codes: Vec<i32> = chips.iter().map(|c| c.code.0 as i32).collect();
    let star = ChipCode::ASTERISK.0 as i32;
    let last = codes.len() - 1;
    // Walk back from the last chip, which a `*` makes the one after its
    // predecessor.
    let mut expected = if codes[last] == star && last > 0 { codes[last - 1] + 1 } else { codes[last] };
    let mut stars = 0;
    for i in (0..=last).rev() {
        if codes[i] == star {
            stars += 1;
        } else if codes[i] != expected {
            return false;
        }
        if i > 0 {
            expected -= 1;
        }
    }
    stars <= 1 && expected >= 0
}

/// `sub_8029224`: a modifier right after a chip it applies to folds into
/// it and leaves the hand. Chained modifiers stack; the first chip never
/// folds.
fn fold_modifiers(entries: &mut [Entry; 6], library: &dyn Library) {
    let mut i = 1;
    while i < entries.len() && entries[i].id != NO_CHIP {
        let Some(m) = library.chip(entries[i].id).modifier else {
            i += 1;
            continue;
        };
        let prev = library.chip(entries[i - 1].id).flags;
        let applies = match m {
            ChipModifier::AttackPlus => prev.has(ChipFlags::HAS_DAMAGE),
            ChipModifier::NaviPlus => prev.has(ChipFlags::NAVI),
            ChipModifier::Paralyze => prev.has(ChipFlags::HAS_DAMAGE),
            ChipModifier::Uninstall => prev.has(ChipFlags::HAS_DAMAGE) && !prev.has(ChipFlags::DIMMING),
        };
        if !applies {
            i += 1;
            continue;
        }
        match m {
            ChipModifier::AttackPlus | ChipModifier::NaviPlus => entries[i - 1].bonus += entries[i].damage,
            ChipModifier::Paralyze => entries[i - 1].modifiers |= modifier_bits::PARALYZE,
            ChipModifier::Uninstall => entries[i - 1].modifiers |= modifier_bits::UNINSTALL,
        }
        shift_up(entries, i, i + 1);
    }
}

/// The game's way of taking entries out: entries from `from` move to
/// `to` onward, one by one, up to and with the end marker; entries past
/// the end marker keep what they held, and each entry keeps its bonus
/// (only the chip, damage and modifiers move).
fn shift_up(entries: &mut [Entry; 6], to: usize, from: usize) {
    let (mut dst, mut src) = (to, from);
    loop {
        let e = entries[src];
        let bonus = entries[dst].bonus;
        entries[dst] = Entry { bonus, ..e };
        if e.id == NO_CHIP {
            return;
        }
        dst += 1;
        src += 1;
    }
}

/// Chips a player sent in a round, by class (`dword_20367E0`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ClassCounts {
    pub standard: u8,
    pub mega: u8,
    pub giga: u8,
}

/// `sub_802A4FC`: the classes of the chips a sent hand was picked from
/// (Program Advance parts and modifiers each count; invalid chips don't),
/// added to the round's counts. The Mega and Giga counts are what the
/// invalid-chip rule checks next time.
pub fn count_classes(hand: &ChipHand, uses: &mut ClassCounts, library: &dyn Library) {
    for &raw in hand.selection.iter().take_while(|&&v| v != NO_CHIP) {
        let count = match library.chip(raw & 0x1FF).class {
            ChipClass::Standard => &mut uses.standard,
            ChipClass::Mega => &mut uses.mega,
            ChipClass::Giga => &mut uses.giga,
            _ => continue,
        };
        *count = count.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custom::library::testing::{EVERY_CODE, TestLibrary, chip};
    use crate::content::{ChipData, ProgramAdvance};

    // Made-up chips: 1 a damaging chip, 2 one without damage, 3 a dimming
    // chip, 4 a navi chip, and the modifiers; the Program Advance 0x140
    // is three of chip 1 in a code run, 0x141 the sequence 2, 1, 2.
    const CANNON: ChipId = 1;
    const QUIET: ChipId = 2;
    const DIMMING: ChipId = 3;
    const NAVI: ChipId = 4;

    fn library() -> TestLibrary {
        let damaging = chip(ChipClass::Standard, EVERY_CODE, ChipFlags::HAS_DAMAGE, 40);
        TestLibrary::new(
            vec![
                (CANNON, damaging),
                (QUIET, chip(ChipClass::Standard, EVERY_CODE, 0, 0)),
                (DIMMING, chip(ChipClass::Standard, EVERY_CODE, ChipFlags::HAS_DAMAGE | ChipFlags::DIMMING, 30)),
                (NAVI, chip(ChipClass::Mega, EVERY_CODE, ChipFlags::NAVI, 100)),
                (0xC0, modifier(ChipModifier::AttackPlus, 10)),
                (0xC1, modifier(ChipModifier::NaviPlus, 20)),
                (0xB8, modifier(ChipModifier::Paralyze, 0)),
                (0xB9, modifier(ChipModifier::Uninstall, 0)),
                (0x140, chip(ChipClass::ProgramAdvance, EVERY_CODE, ChipFlags::HAS_DAMAGE, 300)),
                (0x141, chip(ChipClass::ProgramAdvance, EVERY_CODE, ChipFlags::HAS_DAMAGE, 200)),
            ],
            vec![
                ProgramAdvance { result: 0x141, recipe: PaRecipe::Sequence(vec![QUIET, CANNON, QUIET]) },
                ProgramAdvance { result: 0x140, recipe: PaRecipe::CodeRun { chip: CANNON, count: 3 } },
            ],
        )
    }

    /// A modifier chip (the ids are made up too).
    fn modifier(m: ChipModifier, damage: u16) -> ChipData {
        ChipData { modifier: Some(m), ..chip(ChipClass::Standard, EVERY_CODE, 0, damage) }
    }

    fn pick(id: ChipId, code: u8) -> Pick {
        Pick { chip: FolderChip::new(id, ChipCode(code)), regular: false }
    }

    fn built(picks: &[Pick]) -> Built {
        let lib = library();
        build(picks, 1, &mut ProgramAdvancesUsed::default(), &lib, |id| lib.chip(id).damage)
    }

    #[test]
    fn code_runs() {
        let run = |codes: &[u8]| codes_run(&codes.iter().map(|&c| FolderChip::new(1, ChipCode(c))).collect::<Vec<_>>());
        const STAR: u8 = 26;
        assert!(run(&[0, 1, 2]));
        assert!(run(&[0, STAR, 2]));
        assert!(run(&[STAR, 1, 2]));
        assert!(run(&[0, 1, STAR]));
        assert!(!run(&[STAR, 0, 1]));
        assert!(!run(&[0, 0, 0]));
        assert!(!run(&[1, 0, 2]));
        assert!(!run(&[0, STAR, STAR]));
    }

    #[test]
    fn program_advance_once_a_round() {
        // Chip 1 in codes A, B, C forms 0x140; the Regular chip among the
        // parts makes it the Regular chip.
        let mut picks = [pick(CANNON, 0), pick(CANNON, 1), pick(CANNON, 2)];
        picks[1].regular = true;
        let lib = library();
        let mut used = ProgramAdvancesUsed::default();
        let b = build(&picks, 2, &mut used, &lib, |id| lib.chip(id).damage);
        assert_eq!(b.program_advance.map(|p| (p.chip, p.picks)), Some((0x140, 3)));
        // The end marker moves up behind the Program Advance; the third
        // part stays past it, as in the game.
        assert_eq!(b.hand.ids, [0x140, NO_CHIP, CANNON, NO_CHIP, NO_CHIP, NO_CHIP]);
        assert_eq!(b.hand.damage[0], 300);
        assert_eq!(b.hand.modifiers[0], modifier_bits::REGULAR);
        assert_eq!(b.hand.selection[..4], [0x0001, 0x0201, 0x0401, NO_CHIP]);
        assert_eq!(b.hand.turn, [1; 6]);
        let again = build(&picks, 3, &mut used, &lib, |id| lib.chip(id).damage);
        assert_eq!(again.program_advance, None);
        assert_eq!(again.hand.ids[..3], [CANNON; 3]);
    }

    #[test]
    fn program_advance_later_in_the_selection() {
        // The sequence 2, 1, 2 starting at the second pick; the chips
        // around it stay.
        let b = built(&[pick(CANNON, 5), pick(QUIET, 0), pick(CANNON, 9), pick(QUIET, 3), pick(CANNON, 1)]);
        assert_eq!(b.program_advance.map(|p| (p.chip, p.picks, p.start, p.len)), Some((0x141, 5, 1, 3)));
        assert_eq!(b.hand.ids[..4], [CANNON, 0x141, CANNON, NO_CHIP]);
    }

    #[test]
    fn attack_plus_folds_into_a_damaging_chip() {
        // Atk+10 twice after a damaging chip stacks; after a chip without
        // damage it stays in the hand.
        let b = built(&[pick(CANNON, 0), pick(0xC0, 26), pick(0xC0, 26), pick(QUIET, 13), pick(0xC0, 26)]);
        assert_eq!(b.hand.ids, [CANNON, QUIET, 0xC0, NO_CHIP, NO_CHIP, NO_CHIP]);
        assert_eq!(b.hand.attack_bonus, [20, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn navi_plus_needs_a_navi_chip() {
        let b = built(&[pick(CANNON, 0), pick(0xC1, 26), pick(NAVI, 0), pick(0xC1, 26)]);
        assert_eq!(b.hand.ids[..4], [CANNON, 0xC1, NAVI, NO_CHIP]);
        assert_eq!(b.hand.attack_bonus[..3], [0, 0, 20]);
    }

    #[test]
    fn white_capsule_and_uninstall() {
        let b = built(&[pick(CANNON, 0), pick(0xB8, 26), pick(CANNON, 1), pick(0xB9, 6), pick(DIMMING, 0)]);
        assert_eq!(b.hand.ids[..4], [CANNON, CANNON, DIMMING, NO_CHIP]);
        assert_eq!(b.hand.modifiers[..2], [modifier_bits::PARALYZE, modifier_bits::UNINSTALL]);
        // Uninstll doesn't fold into a dimming.
        let b = built(&[pick(DIMMING, 0), pick(0xB9, 6)]);
        assert_eq!(b.hand.ids[..3], [DIMMING, 0xB9, NO_CHIP]);
    }

    #[test]
    fn classes_are_counted_from_the_picks() {
        let lib = library();
        let b = built(&[pick(CANNON, 0), pick(CANNON, 1), pick(CANNON, 2), pick(NAVI, 0)]);
        let mut uses = ClassCounts::default();
        count_classes(&b.hand, &mut uses, &lib);
        assert_eq!(uses, ClassCounts { standard: 3, mega: 1, giga: 0 });
    }
}
