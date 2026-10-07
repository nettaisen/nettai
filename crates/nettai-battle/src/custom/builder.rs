//! Building a player's hand from their selection (`sub_8029110`): the
//! invalid-chip rule, Program Advances, and the modifier chips that fold
//! into the chip before them. See docs/engine/custom-screen.md §5.

use super::folder::FolderChip;
use super::library::Library;
use crate::content::{ChipClass, ChipCode, ChipFlags, ChipModifier, Recipe};
use crate::hand::ChipHand;
use nettai_content_api::ChipHandle;

/// `ChipHand::modifiers` bits.
pub mod modifier_bits {
    /// The chip is the folder's Regular chip.
    pub const REGULAR: u8 = 0x01;
    /// WhiCapsl folded in: the chip paralyzes.
    pub const PARALYZE: u8 = 0x02;
    /// Uninstll folded in: the chip uninstalls (the damage word's 0x2000;
    /// EXE5's confusion, mixed in by its yellow capsule).
    pub const UNINSTALL: u8 = 0x04;
    /// EXE5's capsules (0x08023824, by 0x08010368 and 0x0800FFF6): the
    /// damage word's 0x1000 (its blindness), its 0x0800 (its HP bug), and
    /// the user healed a tenth of its HP at the use.
    pub const DAMAGE_1000: u8 = 0x08;
    pub const HEAL: u8 = 0x10;
    pub const DAMAGE_0800: u8 = 0x20;
}

/// Program Advances a player has formed this round (once each), by the
/// result's place among them (`Library::advance_index`; the original's bit
/// is the result's place in its chip table past the last Giga chip). The
/// content's Program Advances of every game have their own bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ProgramAdvancesUsed(pub u64);

impl ProgramAdvancesUsed {
    /// `sub_8029652`: spend a Program Advance; false if it was spent.
    fn spend(&mut self, index: u8) -> bool {
        let bit = 1u64 << index;
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
    /// The modifier bits a button mixed into the pick (the slot's `marks`:
    /// EXE5's capsules).
    pub marks: u8,
}

/// One entry of the hand being built (the work area `dword_2033000`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Entry {
    id: Option<ChipHandle>,
    damage: u16,
    bonus: u16,
    modifiers: u8,
}

const EMPTY: Entry = Entry { id: None, damage: 0, bonus: 0, modifiers: 0 };

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
    pub chip: ChipHandle,
    /// Chips picked (all listed).
    pub picks: u8,
    /// Where the recipe's chips start among them, and how many.
    pub start: u8,
    pub len: u8,
}

/// Build a hand from the picked chips, in pick order (`sub_8029110`),
/// with each chip's damage from `damage`. `turn`: the screen's number in
/// the round (1 = first). `own_gauges`: battle flag 0x40 (the
/// recipes only it tries count).
pub fn build(
    picks: &[Pick],
    turn: u8,
    pa_used: &mut ProgramAdvancesUsed,
    library: &dyn Library,
    own_gauges: bool,
    damage: impl Fn(ChipHandle) -> u16,
) -> Built {
    let mut entries = [EMPTY; 6];
    let mut raw = [None; 6];
    for (i, p) in picks.iter().enumerate() {
        raw[i] = Some(p.chip);
        entries[i] = Entry {
            id: Some(p.chip.id),
            damage: damage(p.chip.id),
            bonus: 0,
            modifiers: if p.regular { modifier_bits::REGULAR } else { 0 } | p.marks,
        };
    }
    let mut program_advance = None;
    let rules = library.layout().program_advances;
    if !picks.is_empty() {
        let chips: Vec<FolderChip> = picks.iter().map(|p| p.chip).collect();
        if let Some((result, start, len)) = find_program_advance(&chips, pa_used, library, own_gauges) {
            // sub_80292CC: the recipe's chips become the Program Advance;
            // it is the Regular chip if one of them was (EXE4's 0x0801F404
            // clears its flags: never).
            let modifiers = if rules.keeps_regular {
                let regular = entries[start..start + len].iter().fold(0, |m, e| m | (e.modifiers & modifier_bits::REGULAR));
                entries[start].modifiers | regular
            } else {
                0
            };
            entries[start] = Entry { id: Some(result), damage: damage(result), bonus: 0, modifiers };
            // The entries after the recipe move up behind it, up to and
            // with the end marker; what lay past it stays (EXE6's), or is
            // cleared to the selection's end (EXE4's 0x0801F41E).
            shift_up(&mut entries, start + 1, start + len);
            if rules.clears_past_end {
                let end = picks.len() - len + 1;
                for e in &mut entries[end..=picks.len().min(5)] {
                    *e = Entry { bonus: e.bonus, ..EMPTY };
                }
            }
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
/// formed this round is passed over where each forms once a round (EXE4's
/// 0x0801F390 keeps no record), and one only battle flag 0x40 tries
/// without it (EXE5's 0x080251DC: its full table with the flag, the
/// netbattles' without). Returns (result, start, length).
fn find_program_advance(
    chips: &[FolderChip],
    used: &mut ProgramAdvancesUsed,
    library: &dyn Library,
    own_gauges: bool,
) -> Option<(ChipHandle, usize, usize)> {
    let once = library.layout().program_advances.once_a_round;
    for start in 0..chips.len().saturating_sub(2) {
        let rest = &chips[start..];
        for pa in library.program_advances() {
            if pa.operation_battle_only && !own_gauges {
                continue;
            }
            let len = pa.recipe.len();
            if rest.len() < len || !recipe_matches(&pa.recipe, &rest[..len]) {
                continue;
            }
            if !once || used.spend(library.advance_index(pa.result)) {
                return Some((pa.result, start, len));
            }
        }
    }
    None
}

fn recipe_matches(recipe: &Recipe, chips: &[FolderChip]) -> bool {
    match *recipe {
        Recipe::Sequence(ref ids) => chips.iter().zip(ids).all(|(c, &id)| c.id == id),
        Recipe::CodeRun { chip, .. } => chips.iter().all(|c| c.id == chip) && codes_run(chips),
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
    while i < entries.len()
        && let Some(id) = entries[i].id
    {
        let Some(m) = library.chip(id).modifier else {
            i += 1;
            continue;
        };
        let prev = entries[i - 1].id.map_or(ChipFlags(0), |p| library.chip(p).flags);
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
        // (EXE4's 0x0801F17E: the modifier's Regular chip mark stays, on the
        // chip it folded into.)
        if library.layout().modifier_passes_regular {
            entries[i - 1].modifiers |= entries[i].modifiers & modifier_bits::REGULAR;
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
        if e.id.is_none() {
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
    for c in hand.selection.iter().map_while(|c| *c) {
        let count = match library.chip(c.id).class {
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
    use crate::custom::library::testing::ChipId;
    use super::*;
    use crate::custom::library::testing::{EVERY_CODE, TestLibrary, chip};
    use crate::content::{ChipData, ProgramAdvance};

    const NONE: u16 = 0xFFFF;

    /// Hand ids from made-up numbers (the test library's handles), NONE
    /// for an empty entry.
    fn ids<const N: usize>(v: [u16; N]) -> [Option<ChipHandle>; N] {
        v.map(|id| (id != NONE).then_some(ChipHandle(id)))
    }

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
                ProgramAdvance {
                    result: ChipHandle(0x141),
                    recipe: Recipe::Sequence(vec![ChipHandle(QUIET), ChipHandle(CANNON), ChipHandle(QUIET)]),
                    operation_battle_only: false,
                },
                ProgramAdvance {
                    result: ChipHandle(0x140),
                    recipe: Recipe::CodeRun { chip: ChipHandle(CANNON), count: 3 },
                    operation_battle_only: false,
                },
            ],
        )
    }

    /// A modifier chip (the ids are made up too).
    fn modifier(m: ChipModifier, damage: u16) -> ChipData {
        ChipData { modifier: Some(m), ..chip(ChipClass::Standard, EVERY_CODE, 0, damage) }
    }

    fn pick(id: ChipId, code: u8) -> Pick {
        Pick { chip: FolderChip::new(ChipHandle(id), ChipCode(code)), regular: false, marks: 0 }
    }

    fn built(picks: &[Pick]) -> Built {
        let lib = library();
        build(picks, 1, &mut ProgramAdvancesUsed::default(), &lib, false, |id| lib.chip(id).damage)
    }

    #[test]
    fn code_runs() {
        let run =
            |codes: &[u8]| codes_run(&codes.iter().map(|&c| FolderChip::new(ChipHandle(1), ChipCode(c))).collect::<Vec<_>>());
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
        let b = build(&picks, 2, &mut used, &lib, false, |id| lib.chip(id).damage);
        assert_eq!(b.program_advance.map(|p| (p.chip, p.picks)), Some((ChipHandle(0x140), 3)));
        // The end marker moves up behind the Program Advance; the third
        // part stays past it, as in the game.
        assert_eq!(b.hand.ids, ids([0x140, NONE, CANNON, NONE, NONE, NONE]));
        assert_eq!(b.hand.damage[0], 300);
        assert_eq!(b.hand.modifiers[0], modifier_bits::REGULAR);
        let picked = |code: u8| Some(FolderChip::new(ChipHandle(CANNON), ChipCode(code)));
        assert_eq!(b.hand.selection[..4], [picked(0), picked(1), picked(2), None]);
        assert_eq!(b.hand.turn, [1; 6]);
        let again = build(&picks, 3, &mut used, &lib, false, |id| lib.chip(id).damage);
        assert_eq!(again.program_advance, None);
        assert_eq!(again.hand.ids[..3], ids([CANNON; 3]));
    }

    #[test]
    fn program_advance_later_in_the_selection() {
        // The sequence 2, 1, 2 starting at the second pick; the chips
        // around it stay.
        let b = built(&[pick(CANNON, 5), pick(QUIET, 0), pick(CANNON, 9), pick(QUIET, 3), pick(CANNON, 1)]);
        assert_eq!(b.program_advance.map(|p| (p.chip, p.picks, p.start, p.len)), Some((ChipHandle(0x141), 5, 1, 3)));
        assert_eq!(b.hand.ids[..4], ids([CANNON, 0x141, CANNON, NONE]));
    }

    #[test]
    fn attack_plus_folds_into_a_damaging_chip() {
        // Atk+10 twice after a damaging chip stacks; after a chip without
        // damage it stays in the hand.
        let b = built(&[pick(CANNON, 0), pick(0xC0, 26), pick(0xC0, 26), pick(QUIET, 13), pick(0xC0, 26)]);
        assert_eq!(b.hand.ids, ids([CANNON, QUIET, 0xC0, NONE, NONE, NONE]));
        assert_eq!(b.hand.attack_bonus, [20, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn navi_plus_needs_a_navi_chip() {
        let b = built(&[pick(CANNON, 0), pick(0xC1, 26), pick(NAVI, 0), pick(0xC1, 26)]);
        assert_eq!(b.hand.ids[..4], ids([CANNON, 0xC1, NAVI, NONE]));
        assert_eq!(b.hand.attack_bonus[..3], [0, 0, 20]);
    }

    #[test]
    fn white_capsule_and_uninstall() {
        let b = built(&[pick(CANNON, 0), pick(0xB8, 26), pick(CANNON, 1), pick(0xB9, 6), pick(DIMMING, 0)]);
        assert_eq!(b.hand.ids[..4], ids([CANNON, CANNON, DIMMING, NONE]));
        assert_eq!(b.hand.modifiers[..2], [modifier_bits::PARALYZE, modifier_bits::UNINSTALL]);
        // Uninstll doesn't fold into a dimming.
        let b = built(&[pick(DIMMING, 0), pick(0xB9, 6)]);
        assert_eq!(b.hand.ids[..3], ids([DIMMING, 0xB9, NONE]));
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
