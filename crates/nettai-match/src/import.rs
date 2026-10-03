//! A side from a BN6 save file (bn6-compat's `save`): what S6c's setup
//! takes of it, the game, what it unlocks on the custom screen, the navi
//! code's level and the SP navi deletion times. (The folder, NaviCust,
//! patch cards and stats are a later import's.)

use crate::{CrossList, Side};
use bn6_compat::save::Save;
use nettai_battle::content::Content;

impl Side {
    /// Take the game, the unlocks, the navi code's level and the SP
    /// deletion times from the save in `file` (a .sav's bytes): the version's
    /// Crosses it owns become the side's Cross list (none when it owns all
    /// five: the game's own), and a link navi's stats follow its level and
    /// game as the save's reload gives them. What is worth saying about it
    /// (what the side keeps), or why the file isn't a save.
    pub fn import_save(&mut self, content: &Content, file: &[u8]) -> Result<Vec<String>, String> {
        let save = Save::read(file)?;
        let level = save.navi_level()?;
        let mut notes = Vec::new();
        self.game = save.version();
        let unlocks = save.unlocks();
        self.beast_out = unlocks.beast_out;
        self.crosses = match content.navi(self.navi).forms.as_ref() {
            Some(forms) if !unlocks.crosses.iter().all(|&c| c) => {
                let own = &forms.of(self.game).crosses;
                let owned: Vec<_> = own.iter().zip(unlocks.crosses).filter(|(_, o)| *o).map(|(&f, _)| f).collect();
                Some(CrossList::new(&owned))
            }
            _ => None,
        };
        // The level is the save's operated navi's; a link navi always has
        // one (it exists through its code).
        let link_navi = !content.navi(self.navi).changes_form();
        match level {
            None if link_navi => notes.push("the save received no navi code: the side's link navi keeps its level".into()),
            _ => self.navi_level = level,
        }
        if save.navi() != 0 && !link_navi {
            notes.push("the save operates a link navi: its level is MegaMan's here".into());
        }
        // The SP times of the rules' slots (the save's halfwords past them,
        // unused, read as 0xFFFF: a match file holds the slots alone).
        let slots = crate::sp_slots(content, self.ruleset).len();
        self.sp_times = save.sp_times();
        for t in self.sp_times.0.iter_mut().skip(slots) {
            *t = 0;
        }
        // The stats: the game's (NaviStats+0x20), a link navi's at its level.
        self.stats.version = crate::version_byte(self.game);
        if let Some(s) = self.reloaded(content) {
            self.stats = s;
        }
        Ok(notes)
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::bn6_content;
    use bn6_compat::save::testing::file;
    use nettai_battle::custom::GameVersion;
    use nettai_battle::setup::SpTimes;

    /// A Falzar save without Beast Out, owning TomahawkCross and
    /// GroundCross, operating ProtoMan from his level-5 code: a MegaMan side
    /// takes the game, the unlocks (its Cross list those two), the level
    /// (MegaMan from a code) and the SP times of the rules' slots.
    #[test]
    fn a_save_gives_the_game_unlocks_level_and_times() {
        let content = bn6_content();
        let mut times = SpTimes(std::array::from_fn(|i| 600 + i as u16));
        times.0[19] = 0xFFFF;
        let save = file(GameVersion::Falzar, false, [false, true, false, true, false], 11, Some(5), &times);
        let mut m = crate::draw::live(&content, 1, None).unwrap();
        m.sides[0].game = GameVersion::Gregar;
        let notes = m.sides[0].import_save(&content, &save).unwrap();
        let s = &m.sides[0];
        assert_eq!((s.game, s.beast_out, s.navi_level, s.stats.version), (GameVersion::Falzar, false, Some(5), 1));
        let list: Vec<&str> = s.crosses.unwrap().forms().map(|f| content.defs.form(f).key.as_str()).collect();
        assert_eq!(list, ["bn6:tomahawkcross", "bn6:groundcross"]);
        assert_eq!((s.sp_times.0[0], s.sp_times.0[17], s.sp_times.0[18], s.sp_times.0[19]), (600, 617, 0, 0));
        assert_eq!(notes, ["the save operates a link navi: its level is MegaMan's here"]);
        assert!(crate::check_match(&content, &m).is_empty(), "{:?}", crate::check_match(&content, &m));
        // Every Cross owned: the game's own five.
        let all = file(GameVersion::Gregar, true, [true; 5], 0, None, &SpTimes::default());
        m.sides[0].import_save(&content, &all).unwrap();
        assert_eq!((m.sides[0].crosses, m.sides[0].navi_level, m.sides[0].beast_out), (None, None, true));
    }

    /// A link navi keeps its level when the save received no code; its
    /// stats follow the save's game.
    #[test]
    fn a_link_navi_keeps_its_level_without_a_code() {
        let content = bn6_content();
        let protoman = content.defs.navi_by_key("bn6:protoman").unwrap();
        let mut m = crate::draw::live(&content, 1, None).unwrap();
        let s = &mut m.sides[1];
        s.navi = protoman;
        s.crosses = None;
        s.navi_level = Some(7);
        s.stats = crate::Side::save_base(&content, protoman, s.game, Some(7));
        let notes = s.import_save(&content, &file(GameVersion::Gregar, true, [true; 5], 0, None, &SpTimes::default())).unwrap();
        assert_eq!((s.navi_level, s.game, s.stats.version), (Some(7), GameVersion::Gregar, 0));
        assert_eq!(notes, ["the save received no navi code: the side's link navi keeps its level"]);
        assert!(s.import_save(&content, b"not a save").is_err());
    }
}
