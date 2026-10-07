//! A side from an EXE4 save file (exe4-compat's `save`), whole: MegaMan (the
//! one navi an EXE4 save operates), its equipped folder with its Regular chip,
//! the NaviCust's programs as placed, what the save brings to the stats (the
//! base HP and the Regular memory), and from MegaMan's NaviStats block his
//! light/dark value, the patch cards switched on (a card whose effects
//! aren't ported yet, docs/design/exe4-map.md §18, said and left out), its
//! version, and Double Soul and the souls it has. The import is the compat
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
/// Each version's first soul's event flag, its others' following in its
/// souls' order (0x08020018's tables by version: Red Sun's 0x17 to 0x1C,
/// Blue Moon's 0x1D to 0x22).
const SOUL_FLAGS: [u16; 2] = [0x17, 0x1D];

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
    // What the save brings to the stats: the base HP, the Regular memory.
    state(side, "hp", &[Fact::Value(Value::Int(save.base_max_hp() as i64))]);
    state(side, "reg_up", &[Fact::Value(Value::Int(save.regular_memory() as i64))]);
    // Its version; Double Soul (event flag 0x14) and the souls it has (EXE4's
    // rules/souls): of the forms MegaMan lists for the version, in order,
    // those whose flags are set (0x0801FFD4's table: Red Sun's souls'
    // 0x17 to 0x1C, Blue Moon's 0x1D to 0x22).
    let version = save.version.name();
    state(side, "version", &[Fact::Name(version)]);
    state(side, "double_soul", &[Fact::Value(Value::Bool(save.event_flag(DOUBLE_SOUL)))]);
    let navi = side.navi(content);
    let listed = content.navi(navi).forms.as_ref().map_or(&[][..], |f| f.listed(version));
    let first = SOUL_FLAGS[save.version as usize];
    let souls: Vec<Fact> = (listed.iter().enumerate())
        .filter(|&(k, _)| save.event_flag(first + k as u16))
        .map(|(_, f)| Fact::Value(Value::Def(Registry::Form, f.0)))
        .collect();
    state(side, "souls", &souls);
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
        img[0x4564..0x4564 + 6].copy_from_slice(&[12 << 2, 0, 0, 2, 0, 0]);
        img[0x464C..0x464C + 7].copy_from_slice(&[16, 12, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        img[0x4653..0x4653 + 7].copy_from_slice(&[0xFF, 0xFF, 1, 0xFF, 0xFF, 0xFF, 0xFF]);
        img[0x4E60 + 0x36..0x4E60 + 0x38].copy_from_slice(&460u16.to_le_bytes());
        for flag in [0x14usize, 0x17, 0x19, 0x1D] {
            img[0x2248 + flag / 8] |= 0x80 >> (flag % 8);
        }
        img
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
        let Some(Stated::List(programs)) = get("navicust_programs") else { panic!("{:?}", get("navicust_programs")) };
        let Stated::Record(fields) = &programs[0] else { panic!("{programs:?}") };
        let field = |name: &str| fields.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
        assert_eq!(
            (programs.len(), field("x"), field("y"), field("color")),
            (1, Some(Stated::Number(1)), Some(Stated::Number(3)), Some(Stated::Variant(Some("blue".into()))))
        );
        let Some(Stated::List(cards)) = get("patch_cards") else { panic!("{:?}", get("patch_cards")) };
        assert_eq!(cards.len(), 1, "{cards:?}");
        let form = |key: &str| Stated::Def(Registry::Form, Some(content.form_by_key(key).0));
        assert_eq!(
            (get("version"), get("double_soul"), get("souls")),
            (
                Some(Stated::Variant(Some("redsun".into()))),
                Some(Stated::Flag(true)),
                Some(Stated::List([form("rollsoul"), form("windsoul")].into_iter().chain(std::iter::repeat_n(Stated::Def(Registry::Form, None), 4)).collect()))
            )
        );
        assert_eq!(
            notes,
            [
                "the save's folder holds chip numbers exe4 has no chip for (0x1ff): their entries are left empty",
                "the save's patch cards 12 are left out: their effects aren't ported yet",
            ]
        );
    }
}
