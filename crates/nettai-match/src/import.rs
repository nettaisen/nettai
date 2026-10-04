//! A side from a save file, into a match of the save's game
//! ([`Match::import_save`]): from a BN6 save (bn6-compat's `save`) what
//! S6c's setup takes of it, the version, what it unlocks on the custom
//! screen, the navi code's level and the SP navi deletion times; from a BN5
//! one, its karma and souls (`import_bn5`). (The folder, NaviCust, patch
//! cards and stats are a later import's.)

use crate::{Arena, CrossList, Match, Side};
use bn6_compat::save::Save;
use nettai_battle::content::Content;

impl Match {
    /// Fill side `side` from the save in `file` (a .sav's bytes, or a raw
    /// BN5 save image): the match is the save's game's, so a save of
    /// another game's makes the match a new one of that game
    /// (`Match::empty`, the seed kept) first. What is worth saying about
    /// it, or why the file is no save of a game the content has.
    pub fn import_save(&mut self, content: &Content, side: usize, file: &[u8]) -> Result<Vec<String>, String> {
        let (game, six) = match Save::read(file) {
            Ok(save) => (bn6_compat::ROOT, Ok(save)),
            Err(e) => (bn5_compat::ROOT, Err(e)),
        };
        let five = match six {
            Ok(_) => None,
            Err(six) => Some(crate::import_bn5::read(file).map_err(|five| format!("{six}; {five}"))?),
        };
        let mut notes = Vec::new();
        if self.arena.game != game {
            let seed = self.seed;
            *self = Match::empty(content, game).map_err(|e| format!("a {game} save, but {e}"))?;
            self.seed = seed;
            notes.push(format!("a {game} save: the match is now {game}'s, both sides new"));
        }
        let arena = self.arena.clone();
        let s = &mut self.sides[side];
        notes.extend(match five {
            Some(save) => s.import_bn5_save(content, &arena, &save),
            None => s.import_bn6_save(content, &arena, &Save::read(file).expect("read above"))?,
        });
        Ok(notes)
    }
}

impl Side {
    /// Take the version, the unlocks, the navi code's level and the SP
    /// deletion times from a BN6 save, the side of a match on `arena` (a
    /// BN6 one): the version's Crosses it owns become the side's Cross list
    /// (none when it owns all five: the version's own), and a link navi's
    /// stats follow its level and version as the save's reload gives them.
    /// What is worth saying about it (what the side keeps), or why the save
    /// can't be read.
    pub fn import_bn6_save(&mut self, content: &Content, arena: &Arena, save: &Save) -> Result<Vec<String>, String> {
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
        let slots = crate::sp_slots(content, arena.ruleset).len();
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
        let mut m = crate::draw::live(&content, "bn6", 1, None).unwrap();
        m.sides[0].game = GameVersion::Gregar;
        let notes = m.import_save(&content, 0, &save).unwrap();
        let s = &m.sides[0];
        assert_eq!((s.game, s.beast_out, s.navi_level, s.stats.version), (GameVersion::Falzar, false, Some(5), 1));
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
        let content = bn6_content();
        let protoman = crate::ids::navi(&content, "bn6", "protoman").unwrap();
        let mut m = crate::draw::live(&content, "bn6", 1, None).unwrap();
        let s = &mut m.sides[1];
        s.navi = protoman;
        s.crosses = None;
        s.navi_level = Some(7);
        s.stats = crate::Side::save_base(&content, protoman, s.game, Some(7));
        let notes = m.import_save(&content, 1, &file(GameVersion::Gregar, true, [true; 5], 0, None, &SpTimes::default())).unwrap();
        let s = &m.sides[1];
        assert_eq!((s.navi_level, s.game, s.stats.version), (Some(7), GameVersion::Gregar, 0));
        assert_eq!(notes, ["the save received no navi code: the side's link navi keeps its level"]);
        assert!(m.import_save(&content, 1, b"not a save").is_err());
    }
}
