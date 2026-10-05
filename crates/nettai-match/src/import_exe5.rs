//! A side from an EXE5 save file (exe5-compat's `save`): its karma (the
//! light/dark value), the souls it has, its Soul Unison and Chaos Unison,
//! and how far its NaviCust's board is expanded (its ExpMemry). (Its folder,
//! the NaviCust's programs and its stats are a later import's.)
//! `Match::import_save` comes here for a save that isn't EXE6's.

use crate::{Arena, Side};
use exe5_compat::save::Save;
use nettai_battle::content::Content;

/// The EXE5 save in `file` (a .sav's bytes, or a raw save image as Tango's
/// netplay templates hold), or why it is none.
pub(crate) fn read(file: &[u8]) -> Result<Save, String> {
    Save::read(file).or_else(|e| Save::from_image(file).map_err(|_| e))
}

impl Side {
    /// Take the karma, the souls, Soul Unison and the NaviCust board's
    /// expansions from an EXE5 save, the side of a match on `arena` (an EXE5
    /// one): the souls its version's flags give (the game's souls of those
    /// numbers) as the side's soul list, and its Soul Unison and Chaos
    /// Unison (event flags 0 and 0x236); the save's ExpMemry (key item
    /// 0x61's count) as the expansions of the side's NaviCust, if it has one
    /// (its programs stay: one off a smaller board is the match's check's to
    /// say). What is worth saying about it.
    pub fn import_exe5_save(&mut self, content: &Content, arena: &Arena, save: &Save) -> Vec<String> {
        let mut notes = Vec::new();
        self.karma = save.light_dark();
        if !crate::facts::takes(content, crate::facts::KARMA_FIELD) {
            notes.push(format!("{} has no light and dark MegaMan: the save's karma is kept, unused", arena.game));
        }
        let numbers = save.souls();
        self.soul_unison = save.soul_unison();
        self.chaos_unison = save.chaos_unison();
        let all = crate::facts::all_souls(content, &arena.game);
        let mut souls = Vec::new();
        // (A save's souls are by the original's number: compat names each
        // number's form.)
        let compat = exe5_compat::Compat::exe5();
        for n in numbers {
            match compat.form(n).and_then(|k| crate::ids::form(content, &arena.game, k)).filter(|f| all.contains(f)) {
                Some(f) => souls.push(f),
                None => notes.push(format!("the save has soul {n}, which {} hasn't", arena.game)),
            }
        }
        self.souls = Some(souls);
        if let Some(n) = &mut self.navicust {
            let (had, sizes) = (save.expansions(), crate::navicust_rules(content).boards.len());
            if (had as usize) < sizes {
                n.expansions = had;
            } else {
                notes.push(format!("the save has {had} ExpMemry, but the NaviCust's board has {sizes} sizes: the side's board is kept"));
            }
        }
        notes
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::{exe5_content, exe6_content};

    /// A Team ProtoMan save, dark, with every soul flag set and one
    /// ExpMemry: its karma, its version's souls EXE5 has (ProtoSoul among
    /// them) and its NaviCust's board (5x4, where a new side's is the
    /// largest), through the save import (`import_save`, which takes a save
    /// that isn't EXE6's as EXE5's, into a match of EXE5's).
    #[test]
    fn a_exe5_save_gives_the_karma_and_souls() {
        let content = exe5_content();
        let mut image = vec![0u8; exe5_compat::save::IMAGE_SIZE];
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOB 20041006 US");
        image[0x29F8] = 0xFF;
        image[0x29F9] = 0xFF;
        image[0x52A8 + 0x44..0x52A8 + 0x46].copy_from_slice(&100u16.to_le_bytes());
        image[0x3DB0 + 0x61] = 1;
        assert_eq!(crate::save_game(&image), Ok("exe5"));
        // Into an EXE6 match on EXE5's content (a frontend loads the save's
        // game's): the match becomes EXE5's.
        let mut m = crate::Match::empty(&exe6_content(), "exe6").unwrap();
        m.seed = Some(9);
        let notes = m.import_save(&content, 0, &image).unwrap();
        assert_eq!((m.arena.game.as_str(), m.seed), ("exe5", Some(9)));
        assert_eq!(notes[0], "an exe5 save: the match is now exe5's, both sides new");
        // On EXE6's content alone, an EXE5 save makes no match.
        let mut six = crate::Match::empty(&exe6_content(), "exe6").unwrap();
        let e = six.import_save(&exe6_content(), 0, &image).unwrap_err();
        assert!(e.contains("an exe5 save, but the content is exe6's"), "{e}");
        let s = &m.sides[0];
        assert_eq!(crate::ids::local(&content.defs.navi(s.navi).key), "megaman");
        assert!(crate::ids::in_game(&content, "exe5", &content.defs.navi(s.navi).key));
        assert_eq!(s.karma, 100);
        let souls: Vec<&str> = s.souls.as_ref().unwrap().iter().map(|&f| crate::ids::local(&content.defs.form(f).key)).collect();
        assert!(souls.contains(&"protosoul") && !souls.contains(&"colonelsoul"), "{souls:?}");
        // (Its six souls: those the game hasn't yet are said.)
        assert_eq!(notes.len() - 1 + souls.len(), 6, "{notes:?}");
        assert_eq!(m.sides[1], crate::Match::empty(&content, "exe5").unwrap().sides[1]);
        assert_eq!((s.navicust.map(|n| n.expansions), m.sides[1].navicust.map(|n| n.expansions)), (Some(1), Some(2)));
        // More ExpMemry than the board has sizes: said, the board kept.
        image[0x3DB0 + 0x61] = 3;
        let notes = m.import_save(&content, 0, &image).unwrap();
        assert!(notes.iter().any(|n| n.contains("3 ExpMemry")), "{notes:?}");
        assert_eq!(m.sides[0].navicust.map(|n| n.expansions), Some(1));
        let e = m.import_save(&content, 0, b"not a save").unwrap_err();
        assert!(e.contains("EXE6") && e.contains("EXE5"), "{e}");
    }

    /// The content names a soul by its form's id; the original's number for
    /// it is compat's (records.toml's forms). MegaMan lists his souls in
    /// the order of those numbers, 1 to 12: the order of the soul button's
    /// icons in a pack, which the frontend takes an icon by
    /// (nettai-render's `soul_place`).
    #[test]
    fn megamans_souls_are_in_the_order_of_their_numbers() {
        let content = exe5_content();
        let compat = exe5_compat::Compat::exe5();
        let megaman = content.defs.navi_by_key("megaman").unwrap();
        let souls = &content.navi(megaman).forms.as_ref().unwrap().souls;
        let numbers: Vec<Option<u8>> = souls.iter().map(|&f| compat.form_number(&content.defs.form(f).key)).collect();
        assert_eq!(numbers, (1..=12).map(Some).collect::<Vec<_>>());
        assert_eq!((compat.form(0), compat.form(7), compat.form(13)), (Some("base"), Some("colonelsoul"), None));
    }
}
