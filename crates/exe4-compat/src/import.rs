//! A side from an EXE4 save file (exe4-compat's `save`), whole: MegaMan (the
//! one navi an EXE4 save operates), its equipped folder with its Regular chip,
//! the NaviCust's programs as placed, what the save brings to the stats (the
//! base HP and the Regular memory), and from MegaMan's NaviStats block his
//! light/dark value, the patch cards switched on (a card whose effects
//! aren't ported yet, docs/design/exe4-map.md §18, said and left out), and
//! Double Soul and the souls it has (its version's, by their flags: the
//! content has no version), and the player's auto battle data (what the
//! dark soul of a dark MegaMan plays). The import is the compat
//! boundary's: a save is the original's bytes, and a side its game's facts
//! (nettai's build creator picks the game's import: `builds::import`).

use crate::save::Save;
use nettai_battle::content::{ChipCode, Content};
use nettai_content_api::Registry;
use nettai_battle::custom::FolderChip;
use nettai_battle::rules::Fact;
use nettai_content_api::Value;
use nettai_match::{Folder, Side, ids};

/// Double Soul's event flag (0x0801E0B4).
const DOUBLE_SOUL: u16 = 0x14;

/// The EXE4 save in `file` (a .sav's bytes, or a raw save image as Tango's
/// netplay templates hold), or why it is none. (A raw image says neither its
/// version nor its region, and the import reads neither: it is read as Red
/// Sun US.)
pub fn read(file: &[u8]) -> Result<Save, String> {
    Save::read(file).or_else(|e| Save::from_image(file, crate::Version::RedSun, false).map_err(|_| e))
}

/// The side an EXE4 save gives, of a match of `game` (an EXE4 one), and
/// what is worth saying about it.
pub fn import(content: &Content, game: &str, side: &mut Side, save: &Save) -> Vec<String> {
    let mut notes = Vec::new();
    let compat = crate::Compat::exe4();
    match ids::navi(content, game, "megaman") {
        Some(navi) => {
            if let Err(e) = side.set_navi(content, navi) {
                notes.push(format!("the save's navi is left out: {e}"));
            }
        }
        None => notes.push(format!("{game} has no MegaMan: the side keeps its navi")),
    }
    // (A fact the content's rules don't take is the save's alone: said, and
    // left out.)
    let mut left_out: Vec<String> = Vec::new();
    let mut state = |side: &mut Side, field: &str, values: &[Fact]| {
        if let Err(e) = side.set_fact(content, field, values) {
            left_out.push(format!("the save's {field} is left out: {e}"));
        }
    };
    // His light/dark value (NaviStats +0x36: EXE4's rules/light_dark). (The
    // Full Synchro at the start, +0x1F, is patch card 45's.)
    let block = crate::codec::navi_stats(&save.navi_stats());
    state(side, "karma", &[Fact::Value(Value::Int(block.light_dark as i64))]);
    // Its wins less its losses against the other version's players (0x21D6:
    // EXE4's rules/sun_and_moon).
    state(side, "rival_wins", &[Fact::Value(Value::Int(save.rival_wins() as i64))]);
    // Its equipped folder, by the chips' numbers' names.
    let mut chips = [None; 30];
    let mut nameless: Vec<u16> = Vec::new();
    for (slot, c) in chips.iter_mut().zip(save.folder(save.equipped_folder())) {
        let Some(c) = c else { continue };
        match compat.chip_key(c.id).and_then(|k| ids::chip(content, game, k)) {
            Some(chip) => *slot = Some(FolderChip::new(chip, ChipCode(c.code))),
            None => nameless.push(c.id),
        }
    }
    if !nameless.is_empty() {
        let numbers: Vec<String> = nameless.iter().map(|n| format!("{n:#05x}")).collect();
        notes.push(format!("the save's folder holds chip numbers {game} has no chip for ({}): their entries are left empty", numbers.join(", ")));
    }
    let regular = save.regular_chip().map(|i| i as u8);
    if let Err(e) = side.set_folder(content, &Folder { chips, regular, tags: None }) {
        notes.push(format!("the save's folder is left out: {e}"));
    }
    // MegaMan's NaviCust.
    match crate::setup::navicust(content, compat, &save.navicust()) {
        Ok(programs) => state(side, "navicust_programs", &programs),
        Err(e) => notes.push(format!("the save's NaviCust is left out: {e}")),
    }
    // Its patch cards switched on, by slot (a card whose effects wait is said
    // and left out).
    let slots: Vec<Option<u8>> = save.patch_cards().iter().map(|c| c.filter(|c| c.on).map(|c| c.id)).collect();
    match crate::setup::patch_cards(content, compat, &slots) {
        Ok((cards, waiting)) => {
            state(side, "patch_cards", &cards);
            if !waiting.is_empty() {
                let numbers: Vec<String> = waiting.iter().map(|n| n.to_string()).collect();
                notes.push(format!("the save's patch cards {} are left out: their effects aren't ported yet", numbers.join(", ")));
            }
        }
        Err(e) => notes.push(format!("the save's patch cards are left out: {e}")),
    }
    // What the save brings to the stats: the base HP, the Regular memory,
    // fighting in the sun (his block's +0x29, which the overworld writes).
    state(side, "hp", &[Fact::Value(Value::Int(save.base_max_hp() as i64))]);
    state(side, "reg_up", &[Fact::Value(Value::Int(save.regular_memory() as i64))]);
    state(side, "sun", &[Fact::Value(Value::Bool(save.navi_stats()[0x29] != 0))]);
    // Double Soul (event flag 0x14) and the souls it has (EXE4's rules/souls):
    // its version's whose flags are set (`Version::soul_flag`, 0x08020018's
    // table), by the forms' numbers (records.toml's), in their order.
    state(side, "double_soul", &[Fact::Value(Value::Bool(save.event_flag(DOUBLE_SOUL)))]);
    let souls: Vec<Fact> = (1..=12u8)
        .filter(|&n| save.version.soul_flag(n).is_some_and(|f| save.event_flag(f)))
        .filter_map(|n| compat.form(n).and_then(|key| content.defs.form_by_key(key)))
        .map(|f| Fact::Value(Value::Def(Registry::Form, f.0)))
        .collect();
    state(side, "souls", &souls);
    // The player's auto battle data (EXE4's rules/auto_battle), by the
    // chips' numbers' names.
    let (places, unstated) = crate::setup::auto_battle(&save.auto_battle(), |n| compat.chip_key(n).and_then(|k| ids::chip(content, game, k)));
    state(side, "auto_battle_places", &places);
    notes.extend(unstated.into_iter().map(|s| format!("of the save: {s}")));
    notes.extend(left_out);
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::facts::Stated;
    use nettai_match::testing::exe4_content;

    /// A raw image: the game's name; its second folder equipped, a Cannon A
    /// then chip number 0x1FF (which no EXE4 chip has, left empty) and
    /// its Regular chip the first; AirShoes (program 12, in blue, its one color) on
    /// the command line's left end; patch card 16 (Panel Change) on in slot 0
    /// and card 12 (Buster Patch, whose B button waits) in slot 1, card 1 off
    /// in slot 2; MegaMan's block with
    /// the dark value 460; a base HP of 760 and a Regular memory of 10;
    /// Double Soul (event flag 0x14) and Red Sun's souls 1 and 3 (flags 0x17
    /// and 0x19, RollSoul and WindSoul), and Blue Moon's flag of soul 7
    /// (0x1D), which a Red Sun save's souls don't read.
    fn image() -> Vec<u8> {
        let mut img = vec![0; crate::save::IMAGE_SIZE];
        img[0x2208..0x2208 + 20].copy_from_slice(b"ROCKMANEXE4 20031022");
        img[0x2132] = 1;
        let folder = 0x262C + 2 * 30;
        img[folder..folder + 2].copy_from_slice(&1u16.to_le_bytes());
        img[folder + 2..folder + 4].copy_from_slice(&0x1FFu16.to_le_bytes());
        for i in 2..30 {
            img[folder + 2 * i..folder + 2 * i + 2].copy_from_slice(&[0xFF, 0xFF]);
        }
        img[0x214C] = 0;
        img[0x2148] = 6;
        img[0x21CA..0x21CC].copy_from_slice(&760u16.to_le_bytes());
        img[0x21D6..0x21D8].copy_from_slice(&35u16.to_le_bytes());
        img[0x4564..0x4564 + 6].copy_from_slice(&[12 << 2, 0, 0, 2, 0, 0]);
        img[0x464C..0x464C + 7].copy_from_slice(&[16, 12, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        img[0x4653..0x4653 + 7].copy_from_slice(&[0xFF, 0xFF, 1, 0xFF, 0xFF, 0xFF, 0xFF]);
        img[0x4E60 + 0x36..0x4E60 + 0x38].copy_from_slice(&460u16.to_le_bytes());
        img[0x5064..0x5064 + 84].fill(0xFF);
        img[0x5064..0x5064 + 8].copy_from_slice(&[0x01, 0x00, 0xFF, 0xFF, 0x03, 0x80, 0xFF, 0x01]);
        for flag in [0x14usize, 0x17, 0x19, 0x1D] {
            img[0x2248 + flag / 8] |= 0x80 >> (flag % 8);
        }
        img
    }

    /// Every patch card states its catalog slot as the game shows it, 0A
    /// to 0F (its definition's `slot`: a card's fact for what lists and
    /// files cards; nothing in a battle reads it). A pair's halves sit in
    /// the slots their handlers read for each other (0x080424F4 and
    /// 0x0804251C: Triple Supporter's in the save's slots 1 and 2).
    #[test]
    fn every_patch_card_states_its_catalog_slot() {
        let content = exe4_content();
        let slot = |h| {
            let key = &content.defs.entry(h).key;
            let card = content.defs.definitions.get(Registry::Entry, key).expect("its definition");
            card.spec.field("slot").str().map(str::to_string)
        };
        let cards = content.defs.entries_of("patch_cards");
        assert_eq!(cards.len(), 133);
        for &h in &cards {
            let s = slot(h);
            assert!(matches!(s.as_deref(), Some("0A" | "0B" | "0C" | "0D" | "0E" | "0F")), "{}: {s:?}", content.defs.entry(h).key);
        }
        let of = |id: &str| slot(content.defs.entry_in("patch_cards", id).expect("the card"));
        assert_eq!(of("017-custom2").as_deref(), Some("0A"));
        assert_eq!(of("074-triple-supporter-1-2").as_deref(), Some("0B"));
        assert_eq!(of("075-triple-supporter-2-2").as_deref(), Some("0C"));
        assert_eq!(of("028-pet-menu-color-change").as_deref(), Some("0F"));
    }

    #[test]
    fn an_exe4_save_gives_its_side() {
        let content = exe4_content();
        let save = read(&image()).unwrap();
        let mut side = Side::fresh(&content, "exe4").unwrap();
        let notes = import(&content, "exe4", &mut side, &save);
        let get = |field: &str| side.facts.get(&content, field);
        assert_eq!(
            (get("karma"), get("hp"), get("reg_up")),
            (Some(Stated::Number(460)), Some(Stated::Number(760)), Some(Stated::Number(10)))
        );
        assert_eq!(get("rival_wins"), Some(Stated::Number(35)));
        let Some(Stated::List(programs)) = get("navicust_programs") else { panic!("{:?}", get("navicust_programs")) };
        let Stated::Record(fields) = &programs[0] else { panic!("{programs:?}") };
        let field = |name: &str| fields.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
        assert_eq!(
            (programs.len(), field("x"), field("y"), field("color")),
            (1, Some(Stated::Number(1)), Some(Stated::Number(3)), Some(Stated::Variant(Some("blue".into()))))
        );
        let Some(Stated::List(cards)) = get("patch_cards") else { panic!("{:?}", get("patch_cards")) };
        assert_eq!(cards.len(), 2, "{cards:?}");
        let form = |key: &str| Stated::Def(Registry::Form, Some(content.form_by_key(key).0));
        assert_eq!(get("version"), None, "the content has no version");
        assert_eq!(
            (get("double_soul"), get("souls")),
            (
                Some(Stated::Flag(true)),
                Some(Stated::List([form("rollsoul"), form("windsoul")].into_iter().chain(std::iter::repeat_n(Stated::Def(Registry::Form, None), 10)).collect()))
            )
        );
        // Its auto battle data: a Cannon, an empty place, pattern 4, and a
        // chip number no chip has, left empty: to the last place that isn't.
        let Some(Stated::List(places)) = get("auto_battle_places") else { panic!("{:?}", get("auto_battle_places")) };
        let stated: Vec<&Stated> = places.iter().filter(|p| !matches!(p, Stated::Record(f) if f.iter().all(|(_, v)| matches!(v, Stated::Def(_, None) | Stated::Number(0) | Stated::Flag(false))))).collect();
        assert_eq!(stated.len(), 2, "{places:?}");
        assert_eq!(
            notes,
            [
                "the save's folder holds chip numbers exe4 has no chip for (0x1ff): their entries are left empty",
                "of the save: the auto battle data holds chip numbers the content has no chip for (0x1ff): their places are left empty",
            ]
        );
    }

    /// The e-Reader cards' chips in a save's folder (Duo, chip number 0x137,
    /// in its code D, and PrixPowr, 0x138, in its code G) come into the
    /// side's as any chip does: their records are every ROM's, and what the
    /// cards bring (their names, their pictures) the pack has whether or not
    /// the save holds the cards' data.
    #[test]
    fn the_e_reader_cards_chips_come_into_the_folder() {
        let content = exe4_content();
        let mut img = image();
        let folder = 0x262C + 2 * 30;
        img[folder + 2..folder + 4].copy_from_slice(&(0x137u16 | 3 << 9).to_le_bytes());
        img[folder + 4..folder + 6].copy_from_slice(&(0x138u16 | 6 << 9).to_le_bytes());
        let save = read(&img).unwrap();
        let mut side = Side::fresh(&content, "exe4").unwrap();
        let notes = import(&content, "exe4", &mut side, &save);
        let chips = side.folder(&content).chips;
        let named = |c: Option<FolderChip>| c.map(|c| (ids::local(&content.defs.chip(c.id).key).to_string(), c.code.letter()));
        assert_eq!(
            [named(chips[0]), named(chips[1]), named(chips[2])],
            [Some(("cannon".to_string(), 'A')), Some(("duo".to_string(), 'D')), Some(("prixpowr".to_string(), 'G'))]
        );
        assert!(!notes.iter().any(|n| n.contains("folder")), "{notes:?}");
    }
}
