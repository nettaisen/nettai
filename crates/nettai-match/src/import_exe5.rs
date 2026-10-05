//! A side from an EXE5 save file (exe5-compat's `save`): its karma (the
//! light/dark value), the souls it has, its Soul Unison and Chaos Unison,
//! how far its NaviCust's board is expanded (its ExpMemry), and its
//! computer-navi data (what a computer navi plays from it:
//! `crate::computer_navi`). (Its folder, the NaviCust's programs and its
//! stats are a later import's.) `Match::import_save` comes here for a save
//! that isn't EXE6's.

use crate::{Arena, ComputerNavi, Side, ids};
use exe5_compat::save::{ComputerNaviBlock, Save};
use nettai_battle::content::Content;
use nettai_battle::tactics::{Tactic, TacticPattern, Tactics};

/// A save's computer-navi data block as a side states it: each place's
/// entry, its chips by their numbers' names in `game`, a pattern entry with
/// its record and score (`ComputerNavi::of_block`). What a match doesn't
/// hold of it is left out and said: a chip `game` hasn't (and a pattern
/// with one), a place holding 0, an entry for a pattern past the block's
/// eight.
pub(crate) fn computer_navi(content: &Content, game: &str, block: &ComputerNaviBlock) -> (ComputerNavi, Vec<String>) {
    let mut notes = Vec::new();
    let compat = exe5_compat::Compat::exe5();
    let chip = |n: u16| compat.chip_key(n).and_then(|k| ids::chip(content, game, k));
    // The pattern records, those with a chip the game hasn't none.
    let patterns: Vec<Option<TacticPattern>> = block
        .patterns
        .iter()
        .map(|p| {
            let chips: Option<Vec<_>> = p.chips.iter().map(|&n| chip(n)).collect();
            chips.map(|chips| TacticPattern { dx: p.dx, dy: p.dy, chips, score: p.score })
        })
        .collect();
    let mut entries = Vec::with_capacity(block.places.len());
    for (i, &h) in block.places.iter().enumerate() {
        entries.push(match h {
            0xFFFF => Tactic::Empty,
            0 => Tactic::Nothing,
            h if h & 0x8000 != 0 => {
                let n = (h & 0x7FFF) as usize;
                match patterns.get(n) {
                    Some(Some(_)) => Tactic::Pattern(n as u8),
                    Some(None) => {
                        notes.push(format!("pattern {} of the save's computer-navi data has a chip {game} hasn't: left out", n + 1));
                        Tactic::Empty
                    }
                    None => {
                        notes.push(format!("place {} of the save's computer-navi data names pattern {}, past its eight: left out", i + 1, n + 1));
                        Tactic::Empty
                    }
                }
            }
            h => match chip(h) {
                Some(c) => Tactic::Chip(c),
                None => {
                    notes.push(format!("place {} of the save's computer-navi data holds chip {h:#05x}, which {game} hasn't: left out", i + 1));
                    Tactic::Empty
                }
            },
        });
    }
    let block = Tactics { entries, patterns: patterns.into_iter().map(Option::unwrap_or_default).collect() };
    let (data, more) = ComputerNavi::of_block(&block);
    notes.extend(more);
    (data, notes)
}

/// The EXE5 save in `file` (a .sav's bytes, or a raw save image as Tango's
/// netplay templates hold), or why it is none.
pub(crate) fn read(file: &[u8]) -> Result<Save, String> {
    Save::read(file).or_else(|e| Save::from_image(file).map_err(|_| e))
}

impl Side {
    /// Take the karma, the souls, Soul Unison, the NaviCust board's
    /// expansions and the computer-navi data from an EXE5 save, the side of
    /// a match on `arena` (an EXE5 one): the souls its version's flags give
    /// (the game's souls of those numbers) as the side's soul list, and its
    /// Soul Unison and Chaos Unison (event flags 0 and 0x236); the save's
    /// ExpMemry (key item 0x61's count) as the expansions of the side's
    /// NaviCust, if it has one (its programs stay: one off a smaller board
    /// is the match's check's to say); the save's computer-navi data (its
    /// first block, [`computer_navi`]: what the game has learned of this
    /// player) as the side's. What is worth saying about it.
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
        for n in numbers {
            match all.iter().copied().find(|&f| content.form(f).soul.as_ref().is_some_and(|s| s.number == n)) {
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
        if crate::computer_navi::has(content) {
            let (data, more) = computer_navi(content, &arena.game, &save.computer_navi());
            self.computer_navi = data;
            notes.extend(more);
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
        // (Its computer-navi data: nothing learned.)
        image[0x554C..0x554C + 0xE0].fill(0xFF);
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

    /// A save's computer-navi data becomes the side's: each place's entry
    /// in its place, chips by their numbers' names, a pattern entry its
    /// record with its score; a save that has learned nothing gives none;
    /// what a match doesn't hold is said.
    #[test]
    fn a_exe5_save_gives_the_computer_navi_data() {
        use crate::computer_navi::{Pattern, Play};
        let content = exe5_content();
        let chip = |name: &str| Play::Chip(crate::ids::chip(&content, "exe5", name).unwrap());
        let number = |name: &str| exe5_compat::Compat::exe5().chip_entry(name).unwrap().id;
        let mut image = vec![0u8; exe5_compat::save::IMAGE_SIZE];
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOK 20041006 US");
        image[0x554C..0x554C + 0xE0].fill(0xFF);
        let put = |image: &mut [u8], at: usize, halves: &[u16]| {
            for (i, h) in halves.iter().enumerate() {
                image[0x554C + at + i * 2..0x554C + at + i * 2 + 2].copy_from_slice(&h.to_le_bytes());
            }
        };
        // As a battle's end writes it: place 2 (of the first three), the
        // most used standard chips from place 4, a pattern in place 34.
        put(&mut image, 2, &[number("areagrab")]);
        put(&mut image, 6, &[number("lance"), number("lance"), number("lance"), number("lance"), number("sidebub3")]);
        put(&mut image, 66, &[0x8000]);
        image[0x554C + 0x58..0x554C + 0x5A].copy_from_slice(&[0xFD, 0x01]);
        put(&mut image, 0x5A, &[number("sword"), number("wideswrd")]);
        put(&mut image, 0x64, &[7, 0]);
        let mut m = crate::Match::empty(&content, "exe5").unwrap();
        let notes = m.import_save(&content, 1, &image).unwrap();
        let sword = crate::ids::chip(&content, "exe5", "sword").unwrap();
        let wideswrd = crate::ids::chip(&content, "exe5", "wideswrd").unwrap();
        let data = &m.sides[1].computer_navi;
        let mut want = crate::ComputerNavi::default();
        want.places[1] = Some(chip("areagrab"));
        for i in 3..7 {
            want.places[i] = Some(chip("lance"));
        }
        want.places[7] = Some(chip("sidebub3"));
        want.places[33] = Some(Play::Pattern(Pattern { dx: -3, dy: 1, chips: vec![sword, wideswrd], score: Some(7) }));
        assert_eq!(*data, want);
        assert!(!notes.iter().any(|n| n.contains("computer-navi")), "{notes:?}");
        assert!(m.sides[0].computer_navi.is_empty());
        assert!(!crate::check_match(&content, &m).iter().any(|p| p.contains("computer navi")), "{:?}", crate::check_match(&content, &m));
        // What a match doesn't hold: a chip number the game hasn't, a place
        // holding 0, a pattern past the eighth. A full record comes with
        // its score.
        put(&mut image, 8, &[0x1FF, 0, 0x8009, 0x8001]);
        image[0x554C + 0x68..0x554C + 0x6A].copy_from_slice(&[0xFF, 0x00]);
        put(&mut image, 0x6A, &[number("sword"); 5]);
        put(&mut image, 0x74, &[12, 0]);
        let notes = m.import_save(&content, 1, &image).unwrap();
        for said in ["place 5 of the save's computer-navi data holds chip 0x1ff, which exe5 hasn't", "names pattern 10, past its eight", "1 of the computer-navi data's places hold no chip (0)"] {
            assert!(notes.iter().any(|n| n.contains(said)), "{said}: {notes:?}");
        }
        assert_eq!(notes.iter().filter(|n| n.contains("computer-navi")).count(), 3, "{notes:?}");
        let data = &m.sides[1].computer_navi;
        assert_eq!(data.places[3..8], [Some(chip("lance")), None, None, None, Some(Play::Pattern(Pattern { dx: -1, dy: 0, chips: vec![sword; 5], score: Some(12) }))]);
        assert!(!crate::check_match(&content, &m).iter().any(|p| p.contains("computer navi")), "{:?}", crate::check_match(&content, &m));
        // A save that has learned nothing.
        image[0x554C..0x554C + 0xE0].fill(0xFF);
        m.import_save(&content, 1, &image).unwrap();
        assert!(m.sides[1].computer_navi.is_empty());
    }
}
