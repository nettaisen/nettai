//! A side from an EXE6 save (exe6-compat's `save`): what the game's rules
//! take of it, each written by its setup field's name.

use crate::Side;
use exe6_compat::save::Save;
use nettai_battle::content::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{Registry, Value};

impl Side {
    /// Take the version, the unlocks, the navi code's level and the SP
    /// deletion times from an EXE6 save, the side of an EXE6 match: its
    /// version (`version`), whether it has Beast Out (`beast_out`) and its
    /// Crosses (`crosses`: those of its version's five the save's flags
    /// own, in the Cross numbers' order, as the side's navi lists them;
    /// none for a navi that doesn't change form); a link navi's stats are
    /// the round's to build from its level. What is worth saying about it
    /// (what the side keeps), or why the save can't be read.
    pub fn import_exe6_save(&mut self, content: &Content, save: &Save) -> Result<Vec<String>, String> {
        let level = save.navi_level()?;
        let mut notes = Vec::new();
        let unlocks = save.unlocks();
        self.set_fact(content, "version", &[Fact::Name(save.version().name())])?;
        self.set_fact(content, "beast_out", &[Fact::Value(Value::Bool(unlocks.beast_out))])?;
        let owned: Vec<Fact> = unlocks.owned_crosses(content, self.navi(content)).iter().map(|f| Fact::Value(Value::Def(Registry::Form, f.0))).collect();
        self.set_fact(content, "crosses", &owned)?;
        // The level is the save's operated navi's; a link navi always has
        // one (it exists through its code).
        let link_navi = !content.navi(self.navi(content)).changes_form();
        match level {
            None if link_navi => notes.push("the save received no navi code: the side's link navi keeps its level".into()),
            _ => self.set_level(content, level)?,
        }
        if save.navi() != 0 && !link_navi {
            notes.push("the save operates a link navi: its level is MegaMan's here".into());
        }
        // The SP times, by the chip compat names each slot the game reads by
        // (the save's halfwords past them, unused, left out).
        let times = exe6_compat::codec::Ids::new(content, exe6_compat::Compat::exe6()).sp_times(&save.sp_times());
        self.facts.set_sp_times(content, &times)?;
        Ok(notes)
    }
}

#[cfg(test)]
mod tests {
    use crate::facts::Stated;
    use crate::testing::exe6_content;
    use exe6_compat::save::testing::file;
    use exe6_compat::GameVersion;
    use nettai_battle::rules::Fact;
    use exe6_compat::codec::SpTimes;

    /// A Falzar save without Beast Out, owning TomahawkCross and
    /// GroundCross, operating ProtoMan from his level-5 code: a MegaMan side
    /// takes the game, the unlocks (its Crosses are those two, in place of
    /// the five a random side had), the level (MegaMan from a code) and the
    /// SP times of the rules' slots.
    #[test]
    fn a_save_gives_the_game_unlocks_level_and_times() {
        let content = exe6_content();
        let mut times: SpTimes = std::array::from_fn(|i| 600 + i as u16);
        times[19] = 0xFFFF;
        let save = file(GameVersion::Falzar, false, [false, true, false, true, false], 11, Some(5), &times);
        let mut m = crate::pick::live(&content, "exe6", 1, None).unwrap();
        m.sides[0].set_fact(&content, "version", &[Fact::Name("gregar")]).unwrap();
        assert_eq!(m.sides[0].facts.form_list(&content).len(), 5);
        let notes = m.import_save(&content, 0, &save).unwrap();
        let s = &m.sides[0];
        assert_eq!((s.version(&content), s.level(&content)), (Some("falzar"), Some(5)));
        assert_eq!(s.facts.get(&content, "beast_out"), Some(Stated::Flag(false)));
        // (Falzar's second and fourth, by Cross number: the list holds them
        // from its front.)
        let list: Vec<&str> = s.facts.form_list(&content).iter().map(|&f| crate::ids::local(&content.defs.form(f).key)).collect();
        assert_eq!(list, ["tomahawkcross", "groundcross"]);
        // (By the chips of the slots the game reads, in the slots' order.)
        let times: Vec<(&str, u16)> = s.facts.sp_times(&content).iter().map(|&(c, f)| (crate::ids::local(&content.defs.chip(c).key), f)).collect();
        assert_eq!((times.len(), times[0], times[17]), (18, ("heatman-sp", 600), ("colonel-sp", 617)));
        assert_eq!(notes, ["the save operates a link navi: its level is MegaMan's here"]);
        assert!(crate::check_match(&content, &m).is_empty(), "{:?}", crate::check_match(&content, &m));
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
        assert!(crate::check_match(&content, &m).is_empty(), "{:?}", crate::check_match(&content, &m));
    }

    /// A link navi keeps its level when the save received no code; its
    /// stats follow the save's game.
    #[test]
    fn a_link_navi_keeps_its_level_without_a_code() {
        let content = exe6_content();
        let protoman = crate::ids::navi(&content, "exe6", "protoman").unwrap();
        let mut m = crate::pick::live(&content, "exe6", 1, None).unwrap();
        let s = &mut m.sides[1];
        s.set_navi(&content, protoman).unwrap();
        s.set_fact(&content, "crosses", &[]).unwrap();
        s.set_level(&content, Some(7)).unwrap();
        let notes = m.import_save(&content, 1, &file(GameVersion::Gregar, true, [true; 5], 0, None, &[0; 20])).unwrap();
        let s = &m.sides[1];
        assert_eq!((s.level(&content), s.version(&content)), (Some(7), Some("gregar")));
        // (His round's stats: his level's, of the save's game.)
        let b = crate::check::start(&content, &m).unwrap();
        assert_eq!((b.stats[1].version, b.stats[1].max_hp), (0, 1230));
        assert_eq!(notes, ["the save received no navi code: the side's link navi keeps its level"]);
        assert!(m.import_save(&content, 1, b"not a save").is_err());
    }
}
