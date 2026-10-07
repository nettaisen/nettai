//! A side from an EXE4 save file (exe4-compat's `save`), whole: MegaMan (the
//! one navi an EXE4 save operates), its equipped folder with its Regular chip,
//! the NaviCust's programs as placed, what the save brings to the stats (the
//! base HP and the Regular memory), and from MegaMan's NaviStats block his
//! light/dark value and the Full Synchro at the start. The Mod Cards aren't
//! ported yet (docs/design/exe4-map.md §18): a card switched on is said and
//! left out. The import is the compat boundary's: a save is the original's
//! bytes, and a side its game's facts (nettai's build creator picks the
//! game's import: `builds::import`).

use crate::save::Save;
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::FolderChip;
use nettai_battle::rules::Fact;
use nettai_content_api::Value;
use nettai_match::{Folder, Side, ids};

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
    // His light/dark value and the Full Synchro at the start (NaviStats
    // +0x36, +0x1F: EXE4's rules/light_dark).
    let block = crate::codec::navi_stats(&save.navi_stats());
    state(side, "karma", &[Fact::Value(Value::Int(block.light_dark as i64))]);
    state(side, "full_synchro_start", &[Fact::Value(Value::Bool(block.full_synchro))]);
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
    // (The Mod Cards: not ported yet.)
    let on: Vec<String> = save.mod_cards().iter().flatten().filter(|c| c.on).map(|c| format!("{:#04x}", c.id)).collect();
    if !on.is_empty() {
        notes.push(format!("the save's Mod Cards ({}) are left out: EXE4's Mod Cards aren't ported yet", on.join(", ")));
    }
    // What the save brings to the stats: the base HP, the Regular memory.
    state(side, "hp", &[Fact::Value(Value::Int(save.base_max_hp() as i64))]);
    state(side, "reg_up", &[Fact::Value(Value::Int(save.regular_memory() as i64))]);
    notes.extend(left_out);
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::facts::Stated;
    use nettai_match::testing::exe4_content;

    /// A raw image: the game's name; its second folder equipped, a Cannon A
    /// then an AirShot (a chip EXE4's content hasn't yet, left empty) and
    /// its Regular chip the first; AirShoes (program 12, in blue, its one color) on
    /// the command line's left end; Mod Card 0x10 on; MegaMan's block with
    /// the dark value 460; a base HP of 760 and a Regular memory of 10.
    fn image() -> Vec<u8> {
        let mut img = vec![0; crate::save::IMAGE_SIZE];
        img[0x2208..0x2208 + 20].copy_from_slice(b"ROCKMANEXE4 20031022");
        img[0x2132] = 1;
        let folder = 0x262C + 2 * 30;
        img[folder..folder + 2].copy_from_slice(&1u16.to_le_bytes());
        img[folder + 2..folder + 4].copy_from_slice(&4u16.to_le_bytes());
        for i in 2..30 {
            img[folder + 2 * i..folder + 2 * i + 2].copy_from_slice(&[0xFF, 0xFF]);
        }
        img[0x214C] = 0;
        img[0x2148] = 6;
        img[0x21CA..0x21CC].copy_from_slice(&760u16.to_le_bytes());
        img[0x4564..0x4564 + 6].copy_from_slice(&[12 << 2, 0, 0, 2, 0, 0]);
        img[0x464C..0x464C + 6].copy_from_slice(&[0x10, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        img[0x4653..0x4653 + 6].fill(0xFF);
        img[0x4E60 + 0x36..0x4E60 + 0x38].copy_from_slice(&460u16.to_le_bytes());
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
        assert_eq!(
            notes,
            [
                "the save's folder holds chip numbers exe4 has no chip for (0x004): their entries are left empty",
                "the save's Mod Cards (0x10) are left out: EXE4's Mod Cards aren't ported yet",
            ]
        );
    }
}
