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
