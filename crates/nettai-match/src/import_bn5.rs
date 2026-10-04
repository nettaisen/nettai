//! A side from a BN5 save file (bn5-compat's `save`): its karma (the
//! light/dark value) and the souls it has. (Its folder, NaviCust and stats
//! are a later import's.) `Match::import_save` comes here for a save that
//! isn't BN6's.

use crate::{Arena, Side};
use bn5_compat::save::Save;
use nettai_battle::content::Content;

/// The BN5 save in `file` (a .sav's bytes, or a raw save image as Tango's
/// netplay templates hold), or why it is none.
pub(crate) fn read(file: &[u8]) -> Result<Save, String> {
    Save::read(file).or_else(|e| Save::from_image(file).map_err(|_| e))
}

impl Side {
    /// Take the karma and the souls from a BN5 save, the side of a match
    /// on `arena` (a BN5 one): the souls its version's flags give (the
    /// game's souls of those numbers; none without Soul Unison) as the
    /// side's soul list. What is worth saying about it.
    pub fn import_bn5_save(&mut self, content: &Content, arena: &Arena, save: &Save) -> Vec<String> {
        let mut notes = Vec::new();
        self.karma = save.light_dark();
        if !crate::facts::takes(content, arena.ruleset, crate::facts::KARMA_FIELD) {
            notes.push("the match's ruleset has no light and dark MegaMan: the save's karma is kept, unused".into());
        }
        let numbers = if save.soul_unison() { save.souls() } else { Vec::new() };
        let all = crate::facts::all_souls(content, &arena.game);
        let mut souls = Vec::new();
        for n in numbers {
            match all.iter().copied().find(|&f| content.form(f).soul.as_ref().is_some_and(|s| s.number == n)) {
                Some(f) => souls.push(f),
                None => notes.push(format!("the save has soul {n}, which {} hasn't", arena.game)),
            }
        }
        self.souls = Some(souls);
        notes
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::{bn5_content, bn6_content};

    /// A Team ProtoMan save, dark, with every soul flag set: its karma and
    /// its version's souls BN5 has (ProtoSoul among them), through the save
    /// import (`import_save`, which takes a save that isn't BN6's as BN5's,
    /// into a match of BN5's).
    #[test]
    fn a_bn5_save_gives_the_karma_and_souls() {
        let content = bn5_content();
        let mut image = vec![0u8; bn5_compat::save::IMAGE_SIZE];
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOB 20041006 US");
        image[0x29F8] = 0xFF;
        image[0x29F9] = 0xFF;
        image[0x52A8 + 0x44..0x52A8 + 0x46].copy_from_slice(&100u16.to_le_bytes());
        assert_eq!(crate::save_game(&image), Ok("bn5"));
        // Into a BN6 match on BN5's content (a frontend loads the save's
        // game's): the match becomes BN5's.
        let mut m = crate::Match::empty(&bn6_content(), "bn6").unwrap();
        m.seed = Some(9);
        let notes = m.import_save(&content, 0, &image).unwrap();
        assert_eq!((m.arena.game.as_str(), m.seed), ("bn5", Some(9)));
        assert_eq!(notes[0], "a bn5 save: the match is now bn5's, both sides new");
        // On BN6's content alone, a BN5 save makes no match.
        let mut six = crate::Match::empty(&bn6_content(), "bn6").unwrap();
        let e = six.import_save(&bn6_content(), 0, &image).unwrap_err();
        assert!(e.contains("a bn5 save, but the content is bn6's"), "{e}");
        let s = &m.sides[0];
        assert_eq!(crate::ids::local(&content.defs.navi(s.navi).key), "megaman");
        assert!(crate::ids::in_game("bn5", &content.defs.navi(s.navi).key));
        assert_eq!(s.karma, 100);
        let souls: Vec<&str> = s.souls.as_ref().unwrap().iter().map(|&f| crate::ids::local(&content.defs.form(f).key)).collect();
        assert!(souls.contains(&"protosoul") && !souls.contains(&"colonelsoul"), "{souls:?}");
        // (Its six souls: those the game hasn't yet are said.)
        assert_eq!(notes.len() - 1 + souls.len(), 6, "{notes:?}");
        assert_eq!(m.sides[1], crate::Match::empty(&content, "bn5").unwrap().sides[1]);
        let e = m.import_save(&content, 0, b"not a save").unwrap_err();
        assert!(e.contains("BN6") && e.contains("BN5"), "{e}");
    }
}
