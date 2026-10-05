//! A side from a save file, into a match of the save's game
//! ([`Match::import_save`]): from an EXE6 save (exe6-compat's `save`) what
//! S6c's setup takes of it, the version, what it unlocks on the custom
//! screen, the navi code's level and the SP navi deletion times; from an EXE5
//! one, its karma, its souls, its NaviCust board's expansions and its
//! auto battle data (`import_exe5`). (The folder, the NaviCust's programs, patch cards and
//! stats are a later import's.)

use crate::{CrossList, Match, Side};
use exe6_compat::save::Save;
use nettai_battle::content::Content;

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
        if self.arena.game != game {
            let seed = self.seed;
            *self = Match::empty(content, game).map_err(|e| format!("an {game} save, but {e}"))?;
            self.seed = seed;
            notes.push(format!("an {game} save: the match is now {game}'s, both sides new"));
        }
        let arena = self.arena.clone();
        let s = &mut self.sides[side];
        notes.extend(match five {
            Some(save) => s.import_exe5_save(content, &arena, &save),
            None => s.import_exe6_save(content, &Save::read(file).expect("read above"))?,
        });
        Ok(notes)
    }
}

impl Side {
    /// Take the version, the unlocks, the navi code's level and the SP
    /// deletion times from an EXE6 save, the side of an EXE6 match: the
    /// version's Crosses it owns become the side's Cross list
    /// (none when it owns all five: the version's own), and a link navi's
    /// stats follow its level and version as the save's reload gives them.
    /// What is worth saying about it (what the side keeps), or why the save
    /// can't be read.
    pub fn import_exe6_save(&mut self, content: &Content, save: &Save) -> Result<Vec<String>, String> {
        let level = save.navi_level()?;
        let mut notes = Vec::new();
        self.version = Some(save.version().name().to_string());
        let unlocks = save.unlocks();
        self.beast_out = unlocks.beast_out;
        self.crosses = match exe6_compat::forms::set(content, self.navi, save.version()) {
            Some(set) if !unlocks.crosses.iter().all(|&c| c) => {
                let own = &set.crosses;
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
        let slots = crate::sp_slots(content).len();
        self.sp_times = save.sp_times();
        for t in self.sp_times.0.iter_mut().skip(slots) {
            *t = 0;
        }
        // The stats: the game's (NaviStats+0x20), a link navi's at its level.
        self.stats.version = crate::version_byte(self.version.as_deref());
        if let Some(s) = self.reloaded(content) {
            self.stats = s;
        }
        Ok(notes)
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::exe6_content;
    use exe6_compat::save::testing::file;
    use exe6_compat::GameVersion;
    use nettai_battle::setup::SpTimes;

    /// A Falzar save without Beast Out, owning TomahawkCross and
    /// GroundCross, operating ProtoMan from his level-5 code: a MegaMan side
    /// takes the game, the unlocks (its Cross list those two), the level
    /// (MegaMan from a code) and the SP times of the rules' slots.
    #[test]
    fn a_save_gives_the_game_unlocks_level_and_times() {
        let content = exe6_content();
        let mut times = SpTimes(std::array::from_fn(|i| 600 + i as u16));
        times.0[19] = 0xFFFF;
        let save = file(GameVersion::Falzar, false, [false, true, false, true, false], 11, Some(5), &times);
        let mut m = crate::draw::live(&content, "exe6", 1, None).unwrap();
        m.sides[0].version = Some("gregar".into());
        let notes = m.import_save(&content, 0, &save).unwrap();
        let s = &m.sides[0];
        assert_eq!((s.version.as_deref(), s.beast_out, s.navi_level, s.stats.version), (Some("falzar"), false, Some(5), 1));
        let list: Vec<&str> = s.crosses.unwrap().forms().map(|f| crate::ids::local(&content.defs.form(f).key)).collect();
        assert_eq!(list, ["tomahawkcross", "groundcross"]);
        assert_eq!((s.sp_times.0[0], s.sp_times.0[17], s.sp_times.0[18], s.sp_times.0[19]), (600, 617, 0, 0));
        assert_eq!(notes, ["the save operates a link navi: its level is MegaMan's here"]);
        assert!(crate::check_match(&content, &m).is_empty(), "{:?}", crate::check_match(&content, &m));
        // Every Cross owned: the game's own five.
        let all = file(GameVersion::Gregar, true, [true; 5], 0, None, &SpTimes::default());
        m.import_save(&content, 0, &all).unwrap();
        assert_eq!((m.sides[0].crosses, m.sides[0].navi_level, m.sides[0].beast_out), (None, None, true));
    }

    /// A link navi keeps its level when the save received no code; its
    /// stats follow the save's game.
    #[test]
    fn a_link_navi_keeps_its_level_without_a_code() {
        let content = exe6_content();
        let protoman = crate::ids::navi(&content, "exe6", "protoman").unwrap();
        let mut m = crate::draw::live(&content, "exe6", 1, None).unwrap();
        let s = &mut m.sides[1];
        s.navi = protoman;
        s.crosses = None;
        s.navi_level = Some(7);
        s.stats = crate::Side::save_base(&content, protoman, s.version.as_deref(), Some(7));
        let notes = m.import_save(&content, 1, &file(GameVersion::Gregar, true, [true; 5], 0, None, &SpTimes::default())).unwrap();
        let s = &m.sides[1];
        assert_eq!((s.navi_level, s.version.as_deref(), s.stats.version), (Some(7), Some("gregar"), 0));
        assert_eq!(notes, ["the save received no navi code: the side's link navi keeps its level"]);
        assert!(m.import_save(&content, 1, b"not a save").is_err());
    }
}
