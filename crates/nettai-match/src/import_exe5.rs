//! A side from an EXE5 save file (exe5-compat's `save`): its karma (the
//! light/dark value), the souls it has, its Soul Unison and Chaos Unison,
//! how far its NaviCust's board is expanded (its ExpMemry), and its
//! auto battle data (what a navi in auto battle plays from it:
//! `crate::auto_battle`); for a side that operates a team navi, the
//! navi's level and HP. (Its folder, the NaviCust's programs and MegaMan's
//! stats are a later import's.) `Match::import_save` comes here for a save
//! that isn't EXE6's. (A boundary with exe5-compat, as `import` is with
//! exe6-compat: a save's bytes and the original's numbers in them.)

use crate::auto_battle::{ChipPlace, AutoBattle, Entry, RECORDS, Record};
use crate::{Side, ids};
use exe5_compat::save::{AUTO_BATTLE_EMPTY, AUTO_BATTLE_PATTERN, AutoBattleBlock, Save};
use nettai_battle::content::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{Registry, Value};

/// A save's auto battle data block as a side states it, place for place
/// and record for record: its chips by their numbers' names in `game`, a
/// pattern entry by its record's number. What a match can't state of it is
/// left empty and said: a chip number `game` has no chip for, and an entry
/// for a pattern record past the block's eight. (A chip the game's rules
/// can't play in auto battle comes in as the save has it: the game writes
/// none among the places, where a match's check refuses one, and a record
/// may hold one.)
pub(crate) fn auto_battle(content: &Content, game: &str, block: &AutoBattleBlock) -> (AutoBattle, Vec<String>) {
    let mut notes = Vec::new();
    let compat = exe5_compat::Compat::exe5();
    let mut nameless: Vec<u16> = Vec::new();
    let mut chip = |n: u16| {
        let c = compat.chip_key(n).and_then(|k| ids::chip(content, game, k));
        if c.is_none() && !nameless.contains(&n) {
            nameless.push(n);
        }
        c
    };
    let mut out = AutoBattle::default();
    for (i, &h) in block.places.iter().enumerate() {
        out.places[i] = match h {
            AUTO_BATTLE_EMPTY => Entry::Empty,
            0 => Entry::Zero,
            h if h & AUTO_BATTLE_PATTERN != 0 => match (h & !AUTO_BATTLE_PATTERN) as usize {
                n if n < RECORDS => Entry::Pattern(n as u8),
                n => {
                    notes.push(format!("place {} of the save's auto battle data names pattern {}, past its eight: left empty", i + 1, n + 1));
                    Entry::Empty
                }
            },
            h => chip(h).map_or(Entry::Empty, Entry::Chip),
        };
    }
    for (record, p) in out.records.iter_mut().zip(&block.patterns) {
        let chips = p.chips.map(|h| match h {
            AUTO_BATTLE_EMPTY => ChipPlace::Empty,
            0 => ChipPlace::Zero,
            h => chip(h).map_or(ChipPlace::Empty, ChipPlace::Chip),
        });
        *record = Record { dx: p.dx, dy: p.dy, chips, score: p.score };
    }
    if !nameless.is_empty() {
        let numbers: Vec<String> = nameless.iter().map(|n| format!("{n:#05x}")).collect();
        notes.push(format!("the save's auto battle data holds chip numbers {game} has no chip for ({}): their places are left empty", numbers.join(", ")));
    }
    (out, notes)
}

/// The EXE5 save in `file` (a .sav's bytes, or a raw save image as Tango's
/// netplay templates hold), or why it is none.
pub(crate) fn read(file: &[u8]) -> Result<Save, String> {
    Save::read(file).or_else(|e| Save::from_image(file).map_err(|_| e))
}

impl Side {
    /// Take the karma, the souls, Soul Unison, the NaviCust board's
    /// expansions and the auto battle data from an EXE5 save, the side of
    /// a match of `game` (an EXE5 one): the souls its version's flags give
    /// (the game's souls of those numbers) as the side's soul list, and its
    /// Soul Unison and Chaos Unison (event flags 0 and 0x236); the save's
    /// ExpMemry (key item 0x61's count) as the expansions of the side's
    /// NaviCust, if it has one (its programs stay: one off a smaller board
    /// is the match's check's to say); the save's auto battle data (its
    /// first block, [`auto_battle`]: what the game has learned of this
    /// player) as the side's. What is worth saying about it.
    pub fn import_exe5_save(&mut self, content: &Content, game: &str, save: &Save) -> Vec<String> {
        let mut notes = Vec::new();
        // (A fact the content's rules don't take is the save's alone: said,
        // and left out.)
        let mut state = |side: &mut Side, field: &str, values: &[Fact]| {
            if let Err(e) = side.set_fact(content, field, values) {
                notes.push(format!("the save's {field} is left out: {e}"));
            }
        };
        state(self, "karma", &[Fact::Value(Value::Int(save.light_dark() as i64))]);
        state(self, "soul_unison", &[Fact::Value(Value::Bool(save.soul_unison()))]);
        state(self, "chaos_unison", &[Fact::Value(Value::Bool(save.chaos_unison()))]);
        // (A save's souls are by the original's number: compat names each
        // number's form.)
        let compat = exe5_compat::Compat::exe5();
        let mut souls = Vec::new();
        let mut missing = Vec::new();
        for n in save.souls() {
            match compat.form(n).and_then(|k| crate::ids::form(content, game, k)) {
                Some(f) => souls.push(Fact::Value(Value::Def(Registry::Form, f.0))),
                None => missing.push(n),
            }
        }
        state(self, "souls", &souls);
        notes.extend(missing.iter().map(|n| format!("the save has soul {n}, which {game} hasn't")));
        // The NaviCust's board, where the side has a NaviCust (its
        // `navicust_expansions` stated).
        let has_navicust = matches!(
            self.facts.fact(content, "navicust_expansions").map(|f| f.value()),
            Some(nettai_content_api::FieldValue::OptionalU8(Some(_)))
        );
        // (A board past the rules' is their `validate`'s to say.)
        if has_navicust && let Err(e) = self.set_fact(content, "navicust_expansions", &[Fact::Value(Value::Int(save.expansions() as i64))]) {
            notes.push(format!("the save's navicust_expansions is left out: {e}"));
        }
        if crate::auto_battle::has(content) {
            let (data, more) = auto_battle(content, game, &save.auto_battle());
            if let Err(e) = data.write(content, self) {
                notes.push(format!("the save's auto battle data is left out: {e}"));
            }
            notes.extend(more);
        }
        notes.extend(self.import_exe5_team_navi(content, save));
        notes
    }

    /// A side that operates a team navi (a navi with a story) takes the
    /// save's level (its story flags' count), which its HP is the story's
    /// at (EXE5's rules/save), and, where the save's version has the navi,
    /// the light/dark value of the navi's own block.
    fn import_exe5_team_navi(&mut self, content: &Content, save: &Save) -> Vec<String> {
        if content.navi(self.navi(content)).story.is_none() {
            return Vec::new();
        }
        let name = crate::names::navi(content, self.navi(content));
        let level = save.navi_level();
        if let Err(e) = self.set_level(content, Some(level)) {
            return vec![format!("{}: the save's level {level} is left out: {e}", crate::names::navi(content, self.navi(content)))];
        }
        let compat = exe5_compat::Compat::exe5();
        let key = crate::ids::local(&content.defs.navi(self.navi(content)).key);
        let block = compat.navi_number(key).and_then(|n| save.team_navi_stats(n));
        let mut notes = vec![format!("{name}: the save's level {level}")];
        match block.map(|b| exe5_compat::codec::navi_stats(&b)) {
            Some(Ok(b)) => {
                if let Err(e) = self.set_fact(content, "karma", &[Fact::Value(Value::Int(b.light_dark.0 as i64))]) {
                    notes.push(format!("{name}: its block's karma is left out: {e}"));
                }
            }
            Some(Err(e)) => notes.push(format!("{name}: the navi's block doesn't read ({e}): no karma of its own")),
            None => notes.push(format!("{name}: its version has no such navi: no karma of its own")),
        }
        notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        // (Its auto battle data: nothing learned.)
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
        assert_eq!((m.game.as_str(), m.seed), ("exe5", Some(9)));
        assert_eq!(notes[0], "an exe5 save: the match is now exe5's, both sides new");
        // On EXE6's content alone, an EXE5 save makes no match.
        let mut six = crate::Match::empty(&exe6_content(), "exe6").unwrap();
        let e = six.import_save(&exe6_content(), 0, &image).unwrap_err();
        assert!(e.contains("an exe5 save, but the content is exe6's"), "{e}");
        let s = &m.sides[0];
        assert_eq!(crate::ids::local(&content.defs.navi(s.navi(&content)).key), "megaman");
        assert!(crate::ids::in_game(&content, "exe5", &content.defs.navi(s.navi(&content)).key));
        use crate::facts::Stated;
        assert_eq!(s.facts.get(&content, "karma"), Some(Stated::Number(100)));
        let listed = s.facts.get(&content, "souls").unwrap().defs();
        let souls: Vec<&str> = listed.iter().map(|&f| crate::ids::local(&content.defs.form(nettai_content_api::FormHandle(f)).key)).collect();
        assert!(souls.contains(&"protosoul") && !souls.contains(&"colonelsoul"), "{souls:?}");
        // (Its six souls: those the game hasn't yet are said.)
        assert_eq!(notes.len() - 1 + souls.len(), 6, "{notes:?}");
        assert_eq!(m.sides[1], crate::Match::empty(&content, "exe5").unwrap().sides[1]);
        let expansions = |s: &crate::Side| crate::testing::navicust_expansions(&content, s);
        assert_eq!((expansions(s), expansions(&m.sides[1])), (Some(1), Some(2)));
        // More ExpMemry than the board has sizes: the side states it, and the
        // rules' `validate` says it is past the boards.
        image[0x3DB0 + 0x61] = 3;
        m.import_save(&content, 0, &image).unwrap();
        assert_eq!(crate::testing::navicust_expansions(&content, &m.sides[0]), Some(3));
        let problems = crate::check_match(&content, &m);
        assert!(problems.iter().any(|p| p.contains("a NaviCust with 3 expansions")), "{problems:?}");
        let e = m.import_save(&content, 0, b"not a save").unwrap_err();
        assert!(e.contains("EXE6") && e.contains("EXE5"), "{e}");
    }

    /// A Team ProtoMan save whose story is four flags along: a side that
    /// operates ProtoMan takes its level (4), which his round's HP is the
    /// story's at (450, whatever his block's says), and his own block's
    /// light/dark value; from a Team Colonel save, which hasn't him, the
    /// level alone.
    #[test]
    fn a_exe5_save_gives_a_team_navi_its_level() {
        let content = exe5_content();
        let mut image = vec![0u8; exe5_compat::save::IMAGE_SIZE];
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOB 20041006 US");
        // Event flags 0x300 to 0x303 (and 0x305, past the first clear one).
        image[0x29F8 + 0x60] = 0xF4;
        // His block, the first after MegaMan's: base, current and maximum HP,
        // then the light/dark value.
        let block = 0x52A8 + 0x60;
        image[block + 0x29] = 1;
        for (at, v) in [(0x3E, 470u16), (0x40, 123), (0x42, 470), (0x44, 519)] {
            image[block + at..block + at + 2].copy_from_slice(&v.to_le_bytes());
        }
        let protoman = crate::ids::navi(&content, "exe5", "protoman").unwrap();
        let mut m = crate::Match::empty(&content, "exe5").unwrap();
        let s = &mut m.sides[0];
        s.set_navi(&content, protoman).unwrap();
        s.set_level(&content, Some(0)).unwrap();
        let notes = m.import_save(&content, 0, &image).unwrap();
        let s = &m.sides[0];
        assert_eq!(s.level(&content), Some(4));
        assert_eq!(s.facts.get(&content, "karma"), Some(crate::facts::Stated::Number(519)));
        assert!(notes.iter().any(|n| n.contains("level 4")), "{notes:?}");
        assert_eq!(crate::check::check_side_alone(&content, &m.game, s), Vec::<String>::new());
        let hp = |m: &crate::Match| {
            let mut m = m.clone();
            m.sides[1] = m.sides[0].clone();
            crate::check::round_stats(&content, &m).unwrap()[0].max_hp
        };
        assert_eq!(hp(&m), 450, "the story's at level 4");
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOK 20041006 US");
        let notes = m.import_save(&content, 0, &image).unwrap();
        assert_eq!((m.sides[0].level(&content), hp(&m)), (Some(4), 450));
        assert!(notes.iter().any(|n| n.contains("its version has no such navi")), "{notes:?}");
        // A MegaMan side takes no level from the save.
        assert_eq!(m.sides[1].level(&content), None);
        m.import_save(&content, 1, &image).unwrap();
        assert_eq!(m.sides[1].level(&content), None);
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

    /// A save's auto battle data becomes the side's, place for place and
    /// record for record: chips by their numbers' names, a pattern entry by
    /// its record's number, a 0 a 0; a block nothing has written gives the
    /// side's default; what a match can't state is said.
    #[test]
    fn a_exe5_save_gives_the_auto_battle_data() {
        let content = exe5_content();
        let chip = |name: &str| crate::ids::chip(&content, "exe5", name).unwrap();
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
        // most used standard chips from place 4, a pattern in place 34, its
        // record the first; the second record zeros.
        put(&mut image, 2, &[number("areagrab")]);
        put(&mut image, 6, &[number("lance"), number("lance"), number("lance"), number("lance"), number("sidebub3")]);
        put(&mut image, 66, &[0x8000]);
        image[0x554C + 0x58..0x554C + 0x5A].copy_from_slice(&[0xFD, 0x01]);
        put(&mut image, 0x5A, &[number("sword"), number("wideswrd")]);
        put(&mut image, 0x64, &[7, 0]);
        image[0x554C + 0x68..0x554C + 0x78].fill(0);
        let mut m = crate::Match::empty(&content, "exe5").unwrap();
        let notes = m.import_save(&content, 1, &image).unwrap();
        let mut want = AutoBattle::default();
        want.places[1] = Entry::Chip(chip("areagrab"));
        for i in 3..7 {
            want.places[i] = Entry::Chip(chip("lance"));
        }
        want.places[7] = Entry::Chip(chip("sidebub3"));
        want.places[33] = Entry::Pattern(0);
        want.records[0] = Record {
            dx: -3,
            dy: 1,
            chips: [ChipPlace::Chip(chip("sword")), ChipPlace::Chip(chip("wideswrd")), ChipPlace::Empty, ChipPlace::Empty, ChipPlace::Empty],
            score: 7,
        };
        want.records[1] = Record::ZERO;
        assert_eq!(AutoBattle::of_side(&content, &m.sides[1]), want);
        assert!(!notes.iter().any(|n| n.contains("auto battle")), "{notes:?}");
        // (The other side: a new match's.)
        assert_eq!(AutoBattle::of_side(&content, &m.sides[0]), AutoBattle::nothing_learned());
        assert!(!crate::check_match(&content, &m).iter().any(|p| p.contains("auto battle")), "{:?}", crate::check_match(&content, &m));
        // What a match can't state: a chip number the game has no chip for
        // (among the places, and in a record), a pattern past the eighth.
        put(&mut image, 8, &[0x1FF, 0, 0x8009]);
        put(&mut image, 0x5E, &[0x1FE]);
        let notes = m.import_save(&content, 1, &image).unwrap();
        let said: Vec<&String> = notes.iter().filter(|n| n.contains("auto battle")).collect();
        assert_eq!(said.len(), 2, "{notes:?}");
        assert!(said[0].contains("place 7 of the save's auto battle data names pattern 10, past its eight"), "{notes:?}");
        assert!(said[1].contains("holds chip numbers exe5 has no chip for (0x1ff, 0x1fe): their places are left empty"), "{notes:?}");
        let data = AutoBattle::of_side(&content, &m.sides[1]);
        assert_eq!(data.places[3..8], [Entry::Chip(chip("lance")), Entry::Empty, Entry::Zero, Entry::Empty, Entry::Chip(chip("sidebub3"))]);
        assert_eq!(data.records[0].chips[2], ChipPlace::Empty);
        // A block nothing has written.
        image[0x554C..0x554C + 0xE0].fill(0xFF);
        m.import_save(&content, 1, &image).unwrap();
        assert!(AutoBattle::of_side(&content, &m.sides[1]).is_blank());
    }

    /// The block a side's data is, by number: the import's way back.
    fn block_of(content: &nettai_battle::content::Content, data: &AutoBattle) -> AutoBattleBlock {
        let number = |c: nettai_content_api::ChipHandle| exe5_compat::Compat::exe5().chip_entry(crate::ids::local(&content.defs.chip(c).key)).unwrap().id;
        AutoBattleBlock {
            places: data.places.map(|e| match e {
                Entry::Empty => AUTO_BATTLE_EMPTY,
                Entry::Zero => 0,
                Entry::Chip(c) => number(c),
                Entry::Pattern(n) => AUTO_BATTLE_PATTERN | n as u16,
            }),
            patterns: data.records.map(|r| exe5_compat::save::AutoBattlePattern {
                dx: r.dx,
                dy: r.dy,
                chips: r.chips.map(|c| match c {
                    ChipPlace::Empty => AUTO_BATTLE_EMPTY,
                    ChipPlace::Zero => 0,
                    ChipPlace::Chip(c) => number(c),
                }),
                score: r.score,
            }),
        }
    }

    /// What a match states of a block is the block: a block read into a
    /// side's data and written back is the same in every place and every
    /// record (all a battle reads of it: only its count and its last eight
    /// bytes aren't stated), and so is the data written to a match file and
    /// read back. Over blocks of each awkward shape: as a battle's end
    /// writes one; a pattern that fills its record, the record after it
    /// zeros, and another, the record after it blank; a 0 among the places
    /// and among a record's chips; pattern entries out of the records'
    /// order, in the first places, one twice, and records no entry names;
    /// a full block; one with zeroed records alone; a blank one.
    #[test]
    fn what_a_match_states_of_a_block_is_the_block() {
        use exe5_compat::save::AutoBattlePattern;
        let content = exe5_content();
        let n = |name: &str| exe5_compat::Compat::exe5().chip_entry(name).unwrap().id;
        let none = AUTO_BATTLE_EMPTY;
        let blank = AutoBattlePattern { dx: -1, dy: -1, chips: [none; 5], score: 0xFFFF_FFFF };
        let zero = AutoBattlePattern { dx: 0, dy: 0, chips: [0; 5], score: 0 };
        let block = |places: &[(usize, u16)], patterns: [AutoBattlePattern; 8]| {
            let mut out = AutoBattleBlock { places: [none; 42], patterns };
            for &(i, h) in places {
                out.places[i] = h;
            }
            out
        };
        let (lance, sword, cannon, wide) = (n("lance"), n("sword"), n("cannon"), n("wideswrd"));
        let two = AutoBattlePattern { dx: -3, dy: 1, chips: [lance, lance, none, none, none], score: 7 };
        let full = AutoBattlePattern { dx: -1, dy: 0, chips: [sword, wide, sword, wide, cannon], score: 12 };
        let long_score = AutoBattlePattern { dx: 2, dy: -2, chips: [cannon, 0, sword, none, cannon], score: 0x0123_4567 };
        // As a battle's end writes one: the standard chips (the most used
        // four times), megas, a giga, two patterns, a program advance.
        let mut written: Vec<(usize, u16)> = (3..7).map(|i| (i, lance)).chain((7..11).map(|i| (i, n("sidebub3")))).collect();
        written.extend([(11, n("magnum")), (12, n("magnum")), (13, cannon), (27, n("protoman")), (28, n("colonel")), (32, n("crossdiv"))]);
        written.extend([(33, 0x8000), (34, 0x8001), (41, n("csmopris"))]);
        let every: Vec<(usize, u16)> = (0..42).map(|i| (i, if i % 7 == 0 { 0x8000 | (i as u16 / 7) } else { [lance, sword, cannon, wide][i % 4] })).collect();
        let blocks = [
            ("written", block(&written, [two, full, zero, zero, zero, zero, zero, zero])),
            ("a full pattern, then zeros", block(&[(33, 0x8000)], [full, zero, blank, blank, blank, blank, blank, blank])),
            ("a full pattern, then a blank record", block(&[(33, 0x8000)], [full, blank, zero, zero, zero, zero, zero, zero])),
            ("a full pattern last", block(&[(33, 0x8007)], [zero, zero, zero, zero, zero, zero, zero, full])),
            ("a 0 among the places and in a record", block(&[(3, lance), (4, 0), (5, lance), (0, 0), (40, 0x8002)], [blank, blank, long_score, blank, blank, blank, blank, blank])),
            (
                "patterns out of order, in the first places, one twice, records unnamed",
                block(&[(0, 0x8005), (2, 0x8001), (12, 0x8005), (13, sword), (33, 0x8000)], [two, full, long_score, zero, blank, two, full, long_score]),
            ),
            ("full", block(&every, [full; 8])),
            ("zeroed records alone", block(&[], [zero; 8])),
            ("blank", block(&[], [blank; 8])),
        ];
        for (name, original) in &blocks {
            let (data, notes) = super::auto_battle(&content, "exe5", original);
            assert_eq!(notes, Vec::<String>::new(), "{name}");
            assert_eq!(block_of(&content, &data), *original, "{name}");
            // (And as bytes, but for the count and the last eight.)
            assert_eq!(AutoBattleBlock::read(&original.bytes()), Ok(*original), "{name}");
            // Through a match file.
            let mut m = crate::Match::empty(&content, "exe5").unwrap();
            data.write(&content, &mut m.sides[0]).unwrap();
            AutoBattle::default().write(&content, &mut m.sides[1]).unwrap();
            let text = crate::write(&content, &m);
            let file: crate::file::MatchFile = toml::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}\n{text}"));
            let back = crate::file::resolve(&content, &file).unwrap_or_else(|e| panic!("{name}: {e:?}\n{text}"));
            let back = AutoBattle::of_side(&content, &back.sides[0]);
            assert_eq!(back, data, "{name}:\n{text}");
            assert_eq!(block_of(&content, &back), *original, "{name}");
        }
        assert!(super::auto_battle(&content, "exe5", &blocks[8].1).0.is_blank());
    }
}
