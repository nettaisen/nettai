//! A side from a save file, into a match of the save's game
//! ([`Match::import_save`]): each game's own import ([`exe6`], [`exe5`])
//! writes what its save says that its game's rules take as the side's
//! facts, each by its setup field's name: a boundary names its own game's.
//!
//! A boundary with the compat crates: a save file is the original's own
//! bytes, which each game's compat crate reads (`exe6_compat::save`,
//! `exe5_compat::save`), and which game's a file is picks the reader. These
//! modules are all this crate has of compat.

pub mod exe5;
pub mod exe6;

use crate::Match;
use nettai_battle::content::Content;

/// The game of the save in `file` (a .sav's bytes, or a raw save image as
/// Tango's netplay templates hold): `exe6` or `exe5`, or why it is neither's.
pub fn save_game(file: &[u8]) -> Result<&'static str, String> {
    match exe6::read(file) {
        Ok(_) => Ok(exe6_compat::ROOT),
        Err(six) => exe5::read(file).map(|_| exe5_compat::ROOT).map_err(|five| format!("{six}; {five}")),
    }
}

impl Match {
    /// Fill side `side` from the save in `file` (a .sav's bytes, or a raw
    /// save image) of the content's game ([`save_game`]: a frontend
    /// loads that game's first): the match is the save's game's, so a save
    /// of another game than the match's makes the match a new one of that
    /// game (`Match::empty`, the seed kept) first. What is worth saying
    /// about it, or why the file is no save of the content's game.
    pub fn import_save(&mut self, content: &Content, side: usize, file: &[u8]) -> Result<Vec<String>, String> {
        let game = save_game(file)?;
        let five = if game == exe5_compat::ROOT { Some(exe5::read(file)?) } else { None };
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
            None => s.import_exe6_save(content, &exe6::read(file)?)?,
        });
        Ok(notes)
    }
}
