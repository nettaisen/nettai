//! A side from an EXE6 save (exe6-compat's `save`): what the game's rules
//! take of it, each written by its setup field's name.

use crate::{Folder, Side, ids};
use exe6_compat::save::Save;
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::FolderChip;
use nettai_battle::rules::Fact;
use nettai_content_api::{Registry, Value};

/// The EXE6 save in `file` (a .sav's bytes, or a raw save image as Tango's
/// netplay templates hold), or why it is none.
pub(crate) fn read(file: &[u8]) -> Result<Save, String> {
    Save::read(file).or_else(|e| Save::from_image(file).map_err(|_| e))
}

/// What a NaviStats block holds of what the save brings to the stats (EXE6's
/// rules/save): the base HP (HP Memories, +0x3E), the Regular memory
/// (RegUps, +0x09), fighting in the sun (+0x22).
const BASE_HP: usize = 0x3E;
const REG_UP: usize = 0x09;
const SUN: usize = 0x22;

impl Side {
    /// The side an EXE6 save gives, whole, of an EXE6 match: the navi it
    /// operates; its version (`version`), whether it has Beast Out
    /// (`beast_out`) and its Crosses (`crosses`: those of its version's
    /// five the save's flags own, in the Cross numbers' order, as the navi
    /// lists them; none for a navi that doesn't change form); its navi
    /// code's level; its equipped folder with its Regular and tag chips;
    /// MegaMan's NaviCust (its board's expansions, `navicust_expansions`,
    /// and its programs as placed, `navicust_programs`) and patch cards
    /// (`patch_cards`: those switched on; a link navi has neither); what it
    /// brings to the stats (`hp`, MegaMan's base HP; `reg_up` and `sun`, the
    /// operated navi's); its BugFrags (`bug_frags`); and its SP deletion
    /// times (`sp_times`). A link navi's stats are the round's to build from
    /// its level. What is worth saying about it, or why the save can't be
    /// read.
    pub fn import_exe6_save(&mut self, content: &Content, save: &Save) -> Result<Vec<String>, String> {
        let compat = exe6_compat::Compat::exe6();
        let mut notes = Vec::new();
        // The navi it operates.
        let key = compat.navi_key(save.navi()).ok_or_else(|| format!("the save operates navi {:#x}, which EXE6 hasn't", save.navi()))?;
        let navi = ids::navi(content, "exe6", key).ok_or_else(|| format!("the save operates {key}, which the content hasn't"))?;
        self.set_navi(content, navi)?;
        let link_navi = !content.navi(navi).changes_form();
        let level = save.navi_level()?;
        let unlocks = save.unlocks();
        self.set_fact(content, "version", &[Fact::Name(save.version().name())])?;
        self.set_fact(content, "beast_out", &[Fact::Value(Value::Bool(unlocks.beast_out))])?;
        let owned: Vec<Fact> = unlocks.owned_crosses(content, navi).iter().map(|f| Fact::Value(Value::Def(Registry::Form, f.0))).collect();
        self.set_fact(content, "crosses", &owned)?;
        // The level is the save's operated navi's; a link navi always has
        // one (it exists through its code).
        match level {
            None if link_navi => notes.push("the save operates a link navi without its navi code: the side keeps its level".into()),
            _ => self.set_level(content, level)?,
        }
        // Its equipped folder, by the chips' numbers' names.
        let f = save.folder();
        let mut chips = [None; 30];
        let mut nameless: Vec<u16> = Vec::new();
        // (Chip 0: an empty entry.)
        for (slot, &v) in chips.iter_mut().zip(&f.chips).filter(|(_, v)| **v & 0x1FF != 0) {
            match compat.chip_key(v & 0x1FF).and_then(|k| ids::chip(content, "exe6", k)) {
                Some(c) => *slot = Some(FolderChip::new(c, ChipCode((v >> 9) as u8))),
                None => nameless.push(v & 0x1FF),
            }
        }
        if !nameless.is_empty() {
            let numbers: Vec<String> = nameless.iter().map(|n| format!("{n:#05x}")).collect();
            notes.push(format!("the save's folder holds chip numbers exe6 has no chip for ({}): their entries are left empty", numbers.join(", ")));
        }
        self.set_folder(content, &Folder { chips, regular: f.regular, tags: f.tags.map(|[a, b]| (a, b)) })?;
        // MegaMan's NaviCust and patch cards.
        if link_navi {
            self.set_fact(content, "navicust_programs", &[])?;
            self.set_fact(content, "patch_cards", &[])?;
        } else {
            let ids = exe6_compat::codec::Ids::new(content, compat);
            let programs = exe6_compat::codec::navicust(save.navicust_list(), |id| save.compressed(id), &ids)?;
            self.set_fact(content, "navicust_expansions", &[Fact::Value(Value::Int(save.expansions() as i64))])?;
            self.set_fact(content, "navicust_programs", &programs)?;
            // (A card switched off does nothing in battle: none of the side's.)
            let mut cards = Vec::new();
            for &b in save.patch_cards().iter().filter(|&&b| b & 0x80 == 0) {
                let key = compat.patch_cards.iter().find(|(_, n)| **n == b & 0x7F).map(|(k, _)| k.as_str());
                match key.and_then(|k| content.defs.entry_in(exe6_compat::codec::PATCH_CARDS, k)) {
                    Some(card) => cards.push(Fact::Value(Value::Def(Registry::Entry, card.0))),
                    None => notes.push(format!("the save's patch card {} is none exe6 has: left out", b & 0x7F)),
                }
            }
            self.set_fact(content, "patch_cards", &cards)?;
        }
        // What the save brings to the stats: MegaMan's base HP (a link
        // navi's is its level's), the operated navi's Regular memory and sun.
        let stats = save.navi_stats();
        if !link_navi {
            let hp = u16::from_le_bytes([stats[BASE_HP], stats[BASE_HP + 1]]);
            self.set_fact(content, "hp", &[Fact::Value(Value::Int(hp as i64))])?;
        }
        self.set_fact(content, "reg_up", &[Fact::Value(Value::Int(stats[REG_UP] as i64))])?;
        self.set_fact(content, "sun", &[Fact::Value(Value::Bool(stats[SUN] != 0))])?;
        self.set_fact(content, "bug_frags", &[Fact::Value(Value::Int(save.bug_frags() as i64))])?;
        // The SP times, by the chip compat names each slot the game reads by
        // (the save's halfwords past them, unused, left out).
        let times = exe6_compat::codec::Ids::new(content, compat).sp_times(&save.sp_times());
        let records: Vec<Fact> = times
            .iter()
            .map(|&(c, f)| Fact::Record(vec![("chip", Fact::Value(Value::Def(Registry::Chip, c.0))), ("frames", Fact::Value(Value::Int(f as i64)))]))
            .collect();
        self.set_fact(content, "sp_times", &records)?;
        Ok(notes)
    }
}

#[cfg(test)]
mod tests {
    use crate::facts::Stated;
    use crate::testing::exe6_content;
    use exe6_compat::GameVersion;
    use exe6_compat::codec::{Ids, SpTimes};
    use exe6_compat::save::testing::{file, file_with};
    use exe6_compat::save::{BUG_FRAGS, CARD_COUNT, CARDS, EVENT_FLAGS, EXP_MEMORY, FOLDERS, KEY_ITEMS, NAVI_STATS, NAVICUST};

    /// A Falzar save without Beast Out, owning TomahawkCross and
    /// GroundCross, MegaMan from his level-5 code, with a folder, a NaviCust
    /// of two programs (one compressed) on its 5x5 board, two patch cards
    /// (the second switched off), 1000 HP, 50 Regular memory, the sun,
    /// BugFrags and SP times: a side of it is all of that, the match's
    /// checks find nothing wrong with it, and its round starts.
    #[test]
    fn a_save_gives_a_whole_side() {
        let content = exe6_content();
        let compat = exe6_compat::Compat::exe6();
        let ids = Ids::new(&content, compat);
        let mut m = crate::pick::live(&content, "exe6", 1, None).unwrap();
        // The folder: a random side's, its chips by number.
        let folder = m.sides[1].folder(&content);
        let mut chips = Vec::new();
        for c in folder.chips() {
            chips.extend(ids.packed(c).to_le_bytes());
        }
        let mut stats = [0u8; 0x64];
        stats[0x3E..0x40].copy_from_slice(&1000u16.to_le_bytes());
        (stats[0x09], stats[0x22], stats[0x2D], stats[0x2E]) = (50, 1, 0, folder.regular.unwrap_or(0xFF));
        (stats[0x56], stats[0x57]) = folder.tags.unwrap_or((0xFF, 0xFF));
        // The NaviCust: SuprArmr (white) with its center at (2, 3), HP+50
        // (its second color) at (4, 2), compressed.
        let program = |name: &str| crate::ids::entry(&content, "exe6", "navicust_programs", name).unwrap();
        let (suprarmr, hp50) = (ids.navicust_part_id(program("suprarmr"), 0), ids.navicust_part_id(program("hp-50"), 1));
        let mut list = [0u8; 16];
        (list[0], list[3], list[4]) = (suprarmr, 2, 3);
        (list[8], list[11], list[12]) = (hp50, 4, 2);
        let compressed = 0x2660 + hp50 as usize;
        let flag = [0x80u8 >> (compressed & 7)];
        // The patch cards: CanoDumb on, the next card off.
        let card = |name: &str| ids.patch_card_number(crate::ids::entry(&content, "exe6", "patch_cards", name).unwrap());
        let cards = [card("canodumb"), card("canodumb") + 1 | 0x80];
        let mut times: SpTimes = std::array::from_fn(|i| 600 + i as u16);
        times[19] = 0xFFFF;
        let save = file_with(
            GameVersion::Falzar,
            false,
            [false, true, false, true, false],
            0,
            Some(5),
            &times,
            &[
                (FOLDERS, &chips),
                (NAVI_STATS, &stats),
                (NAVICUST, &list),
                (EVENT_FLAGS + compressed / 8, &flag),
                (KEY_ITEMS + EXP_MEMORY as usize, &[2]),
                (CARD_COUNT, &[2]),
                (CARDS, &cards),
                (BUG_FRAGS, &1234u32.to_le_bytes()),
            ],
        );
        let notes = m.import_save(&content, 0, &save).unwrap();
        assert_eq!(notes, Vec::<String>::new());
        let s = &m.sides[0];
        assert_eq!(crate::ids::local(&content.defs.navi(s.navi(&content)).key), "megaman");
        assert_eq!((s.version(&content), s.level(&content)), (Some("falzar"), Some(5)));
        assert_eq!(s.facts.get(&content, "beast_out"), Some(Stated::Flag(false)));
        // (Falzar's second and fourth, by Cross number: the list holds them
        // from its front.)
        let list: Vec<&str> = s.facts.form_list(&content).iter().map(|&f| crate::ids::local(&content.defs.form(f).key)).collect();
        assert_eq!(list, ["tomahawkcross", "groundcross"]);
        assert_eq!(s.folder(&content), folder);
        assert_eq!(crate::testing::navicust_expansions(&content, s), Some(2));
        let programs: Vec<(String, i64, i64, bool)> = match s.facts.get(&content, "navicust_programs") {
            Some(Stated::List(items)) => items
                .iter()
                .map(|p| {
                    let Stated::Record(f) = p else { panic!("{p:?}") };
                    let get = |n: &str| f.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone()).unwrap();
                    let Stated::Def(_, Some(h)) = get("program") else { panic!() };
                    let (Stated::Number(x), Stated::Number(y)) = (get("x"), get("y")) else { panic!() };
                    let name = content.defs.entry(nettai_content_api::EntryHandle(h)).id().to_string();
                    (name, x, y, get("compressed") == Stated::Flag(true))
                })
                .collect(),
            other => panic!("{other:?}"),
        };
        assert_eq!(programs, [("suprarmr".to_string(), 2, 3, false), ("hp-50".to_string(), 4, 2, true)]);
        let cards = s.facts.get(&content, "patch_cards").unwrap().defs();
        assert_eq!(cards, [crate::ids::entry(&content, "exe6", "patch_cards", "canodumb").unwrap().0]);
        for (field, want) in [("hp", Stated::Number(1000)), ("reg_up", Stated::Number(50)), ("sun", Stated::Flag(true)), ("bug_frags", Stated::Number(1234))] {
            assert_eq!(s.facts.get(&content, field), Some(want), "{field}");
        }
        // (The SP times by the chips of the slots the game reads, in the
        // slots' order.)
        let times = crate::testing::sp_times(&content, &s.facts);
        assert_eq!((times.len(), &times[0], &times[17]), (18, &("heatman-sp".to_string(), 600), &("colonel-sp".to_string(), 617)));
        assert_eq!(crate::check_match(&content, &m), Vec::<String>::new());
        assert!(crate::check::start(&content, &m).is_ok());
        // Every Cross owned: the game's own five, stated; and none owned,
        // an empty list, stated too.
        let all = file(GameVersion::Gregar, true, [true; 5], 0, None, &[0; 20]);
        m.import_save(&content, 0, &all).unwrap();
        let s = &m.sides[0];
        assert_eq!((s.version(&content), s.level(&content)), (Some("gregar"), None));
        assert_eq!(s.facts.form_list(&content), content.navi(s.navi(&content)).forms.as_ref().unwrap().listed("gregar"));
        assert!(s.facts.is_default(&content, "beast_out"));
        let none = file(GameVersion::Gregar, true, [false; 5], 0, None, &[0; 20]);
        m.import_save(&content, 0, &none).unwrap();
        assert_eq!(m.sides[0].facts.get(&content, "crosses").map(|v| v.defs()), Some(Vec::new()));
    }

    /// A save operating ProtoMan from his level-5 code: the side is
    /// ProtoMan at level 5, with no Crosses, NaviCust or patch cards (they
    /// are MegaMan's); without a code, he keeps the side's level, and his
    /// stats follow the save's game.
    #[test]
    fn a_save_operating_a_link_navi() {
        let content = exe6_content();
        let protoman = crate::ids::navi(&content, "exe6", "protoman").unwrap();
        let mut m = crate::pick::live(&content, "exe6", 1, None).unwrap();
        let notes = m.import_save(&content, 1, &file(GameVersion::Falzar, false, [true; 5], 11, Some(5), &[0; 20])).unwrap();
        let s = &m.sides[1];
        assert_eq!((s.navi(&content), s.level(&content), s.version(&content)), (protoman, Some(5), Some("falzar")));
        for field in ["crosses", "navicust_programs", "patch_cards"] {
            assert_eq!(s.facts.get(&content, field).map(|v| v.defs()), Some(Vec::new()), "{field}");
        }
        assert!(s.facts.is_default(&content, "hp"), "a link navi's HP is its level's");
        assert_eq!(notes, Vec::<String>::new());
        let s = &mut m.sides[1];
        s.set_level(&content, Some(7)).unwrap();
        let notes = m.import_save(&content, 1, &file(GameVersion::Gregar, true, [true; 5], 11, None, &[0; 20])).unwrap();
        let s = &m.sides[1];
        assert_eq!((s.level(&content), s.version(&content)), (Some(7), Some("gregar")));
        // (His round's stats: his level's, of the save's game.)
        let b = crate::check::start(&content, &m).unwrap();
        assert_eq!((b.stats[1].version, b.stats[1].max_hp), (0, 1230));
        assert_eq!(notes, ["the save operates a link navi without its navi code: the side keeps its level"]);
        assert!(m.import_save(&content, 1, b"not a save").is_err());
    }
}
