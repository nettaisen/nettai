//! A side from a BN5 save file (bn5-compat's `save`): its karma (the
//! light/dark value) and the souls it has. (Its folder, NaviCust and stats
//! are a later import's.) `Side::import_save` comes here for a save that
//! isn't BN6's.

use crate::Side;
use bn5_compat::save::Save;
use nettai_battle::content::Content;

impl Side {
    /// Take the karma and the souls from the BN5 save in `file` (a .sav's
    /// bytes, or a raw save image as Tango's netplay templates hold): the
    /// souls its version's flags give (the content's BN5 souls of those
    /// numbers; none without Soul Unison) as the side's soul list. What is
    /// worth saying about it, or why the file isn't a BN5 save.
    pub fn import_bn5_save(&mut self, content: &Content, file: &[u8]) -> Result<Vec<String>, String> {
        let save = Save::read(file).or_else(|e| Save::from_image(file).map_err(|_| e))?;
        let mut notes = Vec::new();
        self.karma = save.light_dark();
        if !crate::facts::takes(content, self, crate::facts::KARMA_FIELD) {
            notes.push("the side's ruleset has no light and dark MegaMan: the save's karma is kept, unused".into());
        }
        let numbers = if save.soul_unison() { save.souls() } else { Vec::new() };
        let all = crate::facts::all_souls(content);
        let mut souls = Vec::new();
        for n in numbers {
            let of = all.iter().copied().find(|&f| {
                content.form(f).soul.as_ref().is_some_and(|s| s.number == n)
                    && content.game() == bn5_compat::ROOT
            });
            match of {
                Some(f) => souls.push(f),
                None => notes.push(format!("the save has soul {n}, which the content hasn't")),
            }
        }
        self.souls = Some(souls);
        Ok(notes)
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::every_game;

    /// A Team ProtoMan save, dark, with every soul flag set: its karma and
    /// its version's souls the content has (ProtoSoul among them), through
    /// the save import (`import_save`, which takes a save that isn't BN6's
    /// as BN5's).
    #[test]
    fn a_bn5_save_gives_the_karma_and_souls() {
        let content = every_game();
        let mut image = vec![0u8; bn5_compat::save::IMAGE_SIZE];
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOB 20041006 US");
        image[0x29F8] = 0xFF;
        image[0x29F9] = 0xFF;
        image[0x52A8 + 0x44..0x52A8 + 0x46].copy_from_slice(&100u16.to_le_bytes());
        let mut m = crate::Match::empty(&content).unwrap();
        let s = &mut m.sides[0];
        s.ruleset = content.defs.ruleset_by_key("bn5:stock");
        s.navi = content.defs.navi_by_key("bn5:megaman").unwrap();
        let notes = s.import_save(&content, &image).unwrap();
        assert_eq!(s.karma, 100);
        let souls: Vec<&str> = s.souls.as_ref().unwrap().iter().map(|&f| content.defs.form(f).key.as_str()).collect();
        assert!(souls.contains(&"bn5:protosoul") && !souls.contains(&"bn5:colonelsoul"), "{souls:?}");
        // (Its six souls: those the content hasn't yet are said.)
        assert_eq!(notes.len() + souls.len(), 6, "{notes:?}");
        let e = s.import_save(&content, b"not a save").unwrap_err();
        assert!(e.contains("BN6") && e.contains("BN5"), "{e}");
    }
}
