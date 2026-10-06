//! A side from a save file, into a match of the save's game
//! ([`Match::import_save`]): from an EXE6 save (exe6-compat's `save`) what
//! S6c's setup takes of it, the version, what it unlocks on the custom
//! screen, the navi code's level and the SP navi deletion times; from an EXE5
//! one, its karma, its souls, its NaviCust board's expansions and its
//! auto battle data (`import_exe5`). (The folder, the NaviCust's programs, patch cards and
//! stats are a later import's.) What a save says that its game's rules take
//! is written as the side's facts, each by its setup field's name: a
//! boundary names its own game's (EXE6's `version`, `beast_out` and
//! `crosses`, the list of the Crosses the save's flags own; EXE5's `karma`,
//! `souls`, `soul_unison` and `chaos_unison`).
//!
//! A boundary with the compat crates: a save file is the original's own
//! bytes, which each game's compat crate reads (`exe6_compat::save`,
//! `exe5_compat::save`), and which game's a file is picks the reader. This
//! module, `import_exe5` and the two uses in `auto_battle` (a save's block,
//! and the original's chip numbers as the game's own tie-break) are all
//! this crate has of compat.

use crate::{Match, Side};
use exe6_compat::save::Save;
use nettai_battle::content::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{Registry, Value};

/// The game of the save in `file` (a .sav's bytes, or a raw EXE5 save
/// image): `exe6` or `exe5`, or why it is neither's.
pub fn save_game(file: &[u8]) -> Result<&'static str, String> {
    match Save::read(file) {
        Ok(_) => Ok(exe6_compat::ROOT),
        Err(six) => crate::import_exe5::read(file).map(|_| exe5_compat::ROOT).map_err(|five| format!("{six}; {five}")),
    }
}

impl Match {
    /// Fill side `side` from the save in `file` (a .sav's bytes, or a raw
    /// EXE5 save image) of the content's game ([`save_game`]: a frontend
    /// loads that game's first): the match is the save's game's, so a save
    /// of another game than the match's makes the match a new one of that
    /// game (`Match::empty`, the seed kept) first. What is worth saying
    /// about it, or why the file is no save of the content's game.
    pub fn import_save(&mut self, content: &Content, side: usize, file: &[u8]) -> Result<Vec<String>, String> {
        let game = save_game(file)?;
        let five = if game == exe5_compat::ROOT { Some(crate::import_exe5::read(file)?) } else { None };
        let mut notes = Vec::new();
        if self.game != game {
            let seed = self.seed;
            *self = Match::empty(content, game).map_err(|e| format!("an {game} save, but {e}"))?;
            self.seed = seed;
            notes.push(format!("an {game} save: the match is now {game}'s, both sides new"));
        }
        let game = self.game.clone();
        let s = &mut self.sides[side];
        notes.extend(match five {
            Some(save) => s.import_exe5_save(content, &game, &save),
            None => s.import_exe6_save(content, &Save::read(file).expect("read above"))?,
        });
        Ok(notes)
    }
}

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
