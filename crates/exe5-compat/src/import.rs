//! A side from an EXE5 save file (exe5-compat's `save`), whole: the navi it
//! operates, its equipped folder with its Regular chip, MegaMan's NaviCust
//! (its board, by its ExpMemry, and its programs as placed) and patch cards
//! (those switched on), what it brings to the stats (MegaMan's base HP and
//! sun, the Regular memory), its karma (the light/dark value), the souls it has (none
//! without Soul Unison), its Chaos Unison, and its auto battle data (what a
//! navi in auto battle plays from it: EXE5's rules' `auto_battle_places` and
//! `auto_battle_records`); for a side that operates a team navi, the navi's
//! level, whose HP the story gives; and its SP deletion times. The import is
//! the compat boundary's: a save is the original's bytes, and a side its
//! game's facts (nettai-demo's `save_import` picks the game's import).

use nettai_match::{Folder, Side, ids};
use crate::save::{AUTO_BATTLE_EMPTY, AUTO_BATTLE_PATTERN, AutoBattleBlock, AutoBattlePattern, Save};
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::FolderChip;
use nettai_battle::rules::Fact;
use nettai_content_api::{Registry, Value};

/// The block's pattern records.
const RECORDS: usize = 8;

/// A pattern record nothing has written: 0xFF throughout.
const BLANK: AutoBattlePattern = AutoBattlePattern { dx: -1, dy: -1, chips: [AUTO_BATTLE_EMPTY; 5], score: 0xFFFF_FFFF };

/// A save's auto battle data block as `side`'s facts (EXE5's rules'
/// `auto_battle_places`, to its last place that isn't empty, and
/// `auto_battle_records`, to its last record that isn't blank), place for
/// place and record for record: a chip by its number's name in `game` (`{
/// chip }`), a pattern entry by its record's number from 1 (`{ pattern }`),
/// a 0 (`{ zero = true }`), an empty place `{}`. What a match can't state of
/// it is left empty and said: a chip number `game` has no chip for, and an
/// entry for a pattern record past the block's eight. (A chip the game's
/// rules can't play in auto battle comes in as the save has it: the game
/// writes none among the places, where the rules' `validate` refuses one,
/// and a record may hold one.) What is worth saying about it.
pub fn state_auto_battle(content: &Content, game: &str, block: &AutoBattleBlock, side: &mut Side) -> Vec<String> {
    let mut notes = Vec::new();
    let compat = crate::Compat::exe5();
    let mut nameless: Vec<u16> = Vec::new();
    let mut chip = |n: u16| {
        let c = compat.chip_key(n).and_then(|k| ids::chip(content, game, k));
        if c.is_none() && !nameless.contains(&n) {
            nameless.push(n);
        }
        c.map(|c| Fact::Value(Value::Def(Registry::Chip, c.0)))
    };
    let yes = || Fact::Value(Value::Bool(true));
    let mut places: Vec<Fact> = Vec::new();
    for (i, &h) in block.places.iter().enumerate() {
        places.push(Fact::Record(match h {
            AUTO_BATTLE_EMPTY => Vec::new(),
            0 => vec![("zero", yes())],
            h if h & AUTO_BATTLE_PATTERN != 0 => match (h & !AUTO_BATTLE_PATTERN) as usize {
                n if n < RECORDS => vec![("pattern", Fact::Value(Value::Int(n as i64 + 1)))],
                n => {
                    notes.push(format!("place {} of the save's auto battle data names pattern {}, past its eight: left empty", i + 1, n + 1));
                    Vec::new()
                }
            },
            h => chip(h).map_or_else(Vec::new, |c| vec![("chip", c)]),
        }));
    }
    let last = places.iter().rposition(|p| !matches!(p, Fact::Record(f) if f.is_empty())).map_or(0, |i| i + 1);
    places.truncate(last);
    let mut records: Vec<Fact> = Vec::new();
    for p in &block.patterns {
        let chips: Vec<Fact> = p
            .chips
            .iter()
            .map(|&h| {
                Fact::Record(match h {
                    AUTO_BATTLE_EMPTY => Vec::new(),
                    0 => vec![("zero", yes())],
                    h => chip(h).map_or_else(Vec::new, |c| vec![("chip", c)]),
                })
            })
            .collect();
        records.push(Fact::Record(vec![
            ("dx", Fact::Value(Value::Int(p.dx as i64))),
            ("dy", Fact::Value(Value::Int(p.dy as i64))),
            ("chips", Fact::List(chips)),
            ("score", Fact::Value(Value::Int(p.score as i64))),
        ]));
    }
    let last = block.patterns.iter().rposition(|p| *p != BLANK).map_or(0, |i| i + 1);
    records.truncate(last);
    if !nameless.is_empty() {
        let numbers: Vec<String> = nameless.iter().map(|n| format!("{n:#05x}")).collect();
        notes.push(format!("the save's auto battle data holds chip numbers {game} has no chip for ({}): their places are left empty", numbers.join(", ")));
    }
    let mut facts = side.facts.clone();
    match facts.set(content, "auto_battle_places", &places).and_then(|()| facts.set(content, "auto_battle_records", &records)) {
        Ok(()) => side.facts = facts,
        Err(e) => notes.push(format!("the save's auto battle data is left out: {e}")),
    }
    notes
}

/// The auto battle data of the EXE5 save in `file` (a .sav's bytes, or a
/// raw save image) as `side`'s, a side of a match of `game` (an editor
/// takes it alone): what the game has learned of the save's player, and
/// what is worth saying about it; or why the file gives none.
pub fn auto_battle_of_save(content: &Content, game: &str, file: &[u8], side: &mut Side) -> Result<Vec<String>, String> {
    if !nettai_match::facts::takes(content, "auto_battle_places") {
        return Err(format!("{game} has no auto battle"));
    }
    Ok(state_auto_battle(content, game, &read(file)?.auto_battle(), side))
}

/// The EXE5 save in `file` (a .sav's bytes, or a raw save image as Tango's
/// netplay templates hold), or why it is none.
pub fn read(file: &[u8]) -> Result<Save, String> {
    Save::read(file).or_else(|e| Save::from_image(file).map_err(|_| e))
}

/// Take the karma, the souls, Chaos Unison, the NaviCust board's
/// expansions and the auto battle data from an EXE5 save, the side of a
/// match of `game` (an EXE5 one): the souls its version's flags give (the
/// game's souls of those numbers) as the side's soul list, none if it hasn't
/// Soul Unison (event flag 0: the soul button), and its Chaos Unison (event
/// flag 0x236); the save's ExpMemry (key item 0x61's count) as the
/// expansions of the side's NaviCust, if it has one (its programs stay: one
/// off a smaller board is the match's check's to say); the save's auto
/// battle data (its first block, [`state_auto_battle`]: what the game has
/// learned of this player) as the side's; the save's SP navi deletion times
/// (`sp_times`, [`sp_times`]). What is worth saying about it.
pub fn import(content: &Content, game: &str, side: &mut Side, save: &Save) -> Vec<String> {
    let mut notes = Vec::new();
    let compat = crate::Compat::exe5();
    // The navi it operates.
    match compat.navi_key(save.navi()).and_then(|k| ids::navi(content, game, k)) {
        Some(navi) => {
            if let Err(e) = side.set_navi(content, navi) {
                notes.push(format!("the save's navi is left out: {e}"));
            }
        }
        None => notes.push(format!("the save operates navi {}, which {game} hasn't: the side keeps its navi", save.navi())),
    }
    let megaman = content.navi(side.navi(content)).forms.is_some();
    // (A fact the content's rules don't take is the save's alone: said,
    // and left out.)
    let mut left_out: Vec<String> = Vec::new();
    let mut state = |side: &mut Side, field: &str, values: &[Fact]| {
        if let Err(e) = side.set_fact(content, field, values) {
            left_out.push(format!("the save's {field} is left out: {e}"));
        }
    };
    state(side, "karma", &[Fact::Value(Value::Int(save.light_dark() as i64))]);
    state(side, "chaos_unison", &[Fact::Value(Value::Bool(save.chaos_unison()))]);
    // (A save's souls are by the original's number: compat names each
    // number's form. Without Soul Unison it has no soul button: no souls.)
    let mut souls = Vec::new();
    let mut missing = Vec::new();
    let has = if save.soul_unison() { save.souls() } else { Vec::new() };
    for n in has {
        match compat.form(n).and_then(|k| ids::form(content, game, k)) {
            Some(f) => souls.push(Fact::Value(Value::Def(Registry::Form, f.0))),
            None => missing.push(n),
        }
    }
    state(side, "souls", &souls);
    notes.extend(missing.iter().map(|n| format!("the save has soul {n}, which {game} hasn't")));
    // Its equipped folder, by the chips' numbers' names.
    let f = save.folder();
    let mut chips = [None; 30];
    let mut nameless: Vec<u16> = Vec::new();
    for (slot, &v) in chips.iter_mut().zip(&f.chips).filter(|(_, v)| **v & 0x1FF != 0) {
        match compat.chip_key(v & 0x1FF).and_then(|k| ids::chip(content, game, k)) {
            Some(c) => *slot = Some(FolderChip::new(c, ChipCode((v >> 9) as u8))),
            None => nameless.push(v & 0x1FF),
        }
    }
    if !nameless.is_empty() {
        let numbers: Vec<String> = nameless.iter().map(|n| format!("{n:#05x}")).collect();
        notes.push(format!("the save's folder holds chip numbers {game} has no chip for ({}): their entries are left empty", numbers.join(", ")));
    }
    if let Err(e) = side.set_folder(content, &Folder { chips, regular: f.regular, tags: None }) {
        notes.push(format!("the save's folder is left out: {e}"));
    }
    // MegaMan's NaviCust (its board, the save's ExpMemry: one past the
    // rules' is their `validate`'s to say) and patch cards; a team navi
    // has neither.
    if megaman {
        state(side, "navicust_expansions", &[Fact::Value(Value::Int(save.expansions() as i64))]);
        match crate::codec::navicust(content, compat, save.navicust_list(), |id| save.compressed(id)) {
            Ok(programs) => state(side, "navicust_programs", &programs),
            Err(e) => notes.push(format!("the save's NaviCust is left out: {e}")),
        }
        match crate::codec::patch_cards(content, compat, save.version(), &save.patch_cards()) {
            Ok(cards) => state(side, "patch_cards", &cards),
            Err(e) => notes.push(format!("the save's patch cards are left out: {e}")),
        }
    } else {
        state(side, "navicust_programs", &[]);
        state(side, "patch_cards", &[]);
    }
    // What the save brings to the stats: MegaMan's base HP (a team
    // navi's is its story's), the operated navi's Regular memory.
    if let Some(stats) = save.team_navi_stats(save.navi()) {
        if megaman {
            let hp = u16::from_le_bytes([stats[0x3E], stats[0x3F]]);
            state(side, "hp", &[Fact::Value(Value::Int(hp as i64))]);
            // (The sun, +0x22: MegaMan's, which the overworld writes.)
            state(side, "sun", &[Fact::Value(Value::Bool(stats[0x22] != 0))]);
        }
        state(side, "reg_up", &[Fact::Value(Value::Int(stats[0x09] as i64))]);
    }
    let (times, nameless) = sp_times(content, game, save);
    state(side, "sp_times", &times);
    notes.extend(nameless);
    notes.extend(left_out);
    if nettai_match::facts::takes(content, "auto_battle_places") {
        notes.extend(state_auto_battle(content, game, &save.auto_battle(), side));
    }
    notes.extend(team_navi(content, side, save));
    notes
}

/// The save's SP navi deletion times as the rules' `sp_times` (a `{ chip,
/// frames }` each): the chip compat names each slot by (records.toml's
/// `sp_slots`), in the slots' order, with the save's frames as they are.
/// A time of 0xFFFF (its navi never deleted) is stated as 65535 frames:
/// the original's own, which its damage reads as the slowest time (the
/// chip's least damage), as the game's battle does; left out, the rules
/// would read it as deleted in no time. A slot whose chip `game` hasn't is
/// said and left out.
pub fn sp_times(content: &Content, game: &str, save: &Save) -> (Vec<Fact<'static>>, Vec<String>) {
    let compat = crate::Compat::exe5();
    let times = save.sp_times();
    let mut slots: Vec<(&String, u8)> = compat.records.sp_slots.iter().map(|(k, &n)| (k, n)).collect();
    slots.sort_by_key(|&(_, n)| n);
    let mut notes = Vec::new();
    let facts = slots
        .into_iter()
        .filter_map(|(key, n)| match ids::chip(content, game, key) {
            Some(c) => Some(Fact::Record(vec![("chip", Fact::Value(Value::Def(Registry::Chip, c.0))), ("frames", Fact::Value(Value::Int(times[n as usize] as i64)))])),
            None => {
                notes.push(format!("the save's SP time of {key} (slot {n}) is left out: {game} has no such chip"));
                None
            }
        })
        .collect();
    (facts, notes)
}

/// A side that operates a team navi (a navi with a story) takes the
/// save's level (its story flags' count), which its HP is the story's
/// at (EXE5's rules/save), and, where the save's version has the navi,
/// the light/dark value of the navi's own block.
fn team_navi(content: &Content, side: &mut Side, save: &Save) -> Vec<String> {
    // (MegaMan takes no level.)
    if content.navi(side.navi(content)).story.is_none() {
        return match side.set_level(content, None) {
            Ok(()) => Vec::new(),
            Err(e) => vec![format!("the side's level is left as it is: {e}")],
        };
    }
    let name = nettai_match::names::navi(content, side.navi(content));
    let level = save.navi_level();
    if let Err(e) = side.set_level(content, Some(level)) {
        return vec![format!("{}: the save's level {level} is left out: {e}", nettai_match::names::navi(content, side.navi(content)))];
    }
    let compat = crate::Compat::exe5();
    let key = ids::local(&content.defs.navi(side.navi(content)).key);
    let block = compat.navi_number(key).and_then(|n| save.team_navi_stats(n));
    let mut notes = vec![format!("{name}: the save's level {level}")];
    match block.map(|b| crate::codec::navi_stats(&b)) {
        Some(Ok(b)) => {
            if let Err(e) = side.set_fact(content, "karma", &[Fact::Value(Value::Int(b.light_dark.0 as i64))]) {
                notes.push(format!("{name}: its block's karma is left out: {e}"));
            }
        }
        Some(Err(e)) => notes.push(format!("{name}: the navi's block doesn't read ({e}): no karma of its own")),
        None => notes.push(format!("{name}: its version has no such navi: no karma of its own")),
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::testing::exe5_content;

    /// A Team ProtoMan save, dark, with every soul flag set and one
    /// ExpMemry: its karma, its version's souls EXE5 has (ProtoSoul among
    /// them; none without Soul Unison) and its NaviCust's board (5x4, where
    /// a new side's is the largest).
    #[test]
    fn a_exe5_save_gives_the_karma_and_souls() {
        let content = exe5_content();
        let mut image = vec![0u8; crate::save::IMAGE_SIZE];
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOB 20041006 US");
        // (Its auto battle data: nothing learned.)
        image[0x554C..0x554C + 0xE0].fill(0xFF);
        image[0x29F8] = 0xFF;
        image[0x29F9] = 0xFF;
        image[0x52A8 + 0x44..0x52A8 + 0x46].copy_from_slice(&100u16.to_le_bytes());
        image[0x3DB0 + 0x61] = 1;
        // (Its SP times: ProtoMan SP's slot 1 in 721 frames, GridMan SP's
        // 18 never; the rest in no time.)
        image[0x2670 + 2..0x2670 + 4].copy_from_slice(&721u16.to_le_bytes());
        image[0x2670 + 36..0x2670 + 38].copy_from_slice(&0xFFFFu16.to_le_bytes());
        let mut m = nettai_match::Match::empty(&content, "exe5").unwrap();
        let notes = import_file(&content, &mut m, 0, &image).unwrap();
        let s = &m.sides[0];
        let times = nettai_match::testing::sp_times(&content, &s.facts);
        assert_eq!((times.len(), &times[0], &times[1], &times[17]), (18, &("protomn-sp".to_string(), 721), &("gyroman-sp".to_string(), 0), &("gridman-sp".to_string(), 0xFFFF)));
        assert_eq!(nettai_match::ids::local(&content.defs.navi(s.navi(&content)).key), "megaman");
        assert!(nettai_match::ids::in_game(&content, "exe5", &content.defs.navi(s.navi(&content)).key));
        use nettai_match::facts::Stated;
        assert_eq!(s.facts.get(&content, "karma"), Some(Stated::Number(100)));
        let listed = s.facts.get(&content, "souls").unwrap().defs();
        let souls: Vec<&str> = listed.iter().map(|&f| nettai_match::ids::local(&content.defs.form(nettai_content_api::FormHandle(f)).key)).collect();
        assert!(souls.contains(&"protosoul") && !souls.contains(&"colonelsoul"), "{souls:?}");
        // (Its six souls: those the game hasn't yet are said.)
        assert_eq!(notes.len() + souls.len(), 6, "{notes:?}");
        // Without Soul Unison (event flag 0), its soul flags all the same:
        // no souls.
        let mut without = image.clone();
        without[0x29F8] &= !0x80;
        let mut m2 = nettai_match::Match::empty(&content, "exe5").unwrap();
        import_file(&content, &mut m2, 0, &without).unwrap();
        assert!(m2.sides[0].facts.get(&content, "souls").unwrap().defs().is_empty());
        assert_eq!(m.sides[1], nettai_match::Match::empty(&content, "exe5").unwrap().sides[1]);
        let expansions = |s: &nettai_match::Side| nettai_match::testing::navicust_expansions(&content, s);
        assert_eq!((expansions(s), expansions(&m.sides[1])), (Some(1), Some(2)));
        // More ExpMemry than the board has sizes: the side states it, and the
        // rules' `validate` says it is past the boards.
        image[0x3DB0 + 0x61] = 3;
        import_file(&content, &mut m, 0, &image).unwrap();
        assert_eq!(nettai_match::testing::navicust_expansions(&content, &m.sides[0]), Some(3));
        let problems = nettai_match::check_match(&content, &m);
        assert!(problems.iter().any(|p| p.contains("a NaviCust with 3 expansions")), "{problems:?}");
        let e = import_file(&content, &mut m, 0, b"not a save").unwrap_err();
        assert!(e.contains("EXE5"), "{e}");
    }

    /// A Team ProtoMan save operating ProtoMan, whose story is four flags
    /// along: the side is ProtoMan at its level (4), which his round's HP is
    /// the story's at (450, whatever his block's says), with his own block's
    /// light/dark value; from a Team Colonel save, which hasn't him, the
    /// level alone; and a save operating MegaMan gives a MegaMan side, who
    /// takes no level.
    #[test]
    fn a_exe5_save_gives_a_team_navi_its_level() {
        let content = exe5_content();
        let mut image = vec![0u8; crate::save::IMAGE_SIZE];
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
        // (The navi operated: ProtoMan, Team ProtoMan's first.)
        image[0x2941] = 1;
        let protoman = nettai_match::ids::navi(&content, "exe5", "protoman").unwrap();
        let mut m = nettai_match::Match::empty(&content, "exe5").unwrap();
        let notes = import_file(&content, &mut m, 0, &image).unwrap();
        let s = &m.sides[0];
        assert_eq!(s.navi(&content), protoman);
        for field in ["navicust_programs", "patch_cards"] {
            assert_eq!(s.facts.get(&content, field).map(|v| v.defs()), Some(Vec::new()), "{field}: MegaMan's");
        }
        assert_eq!(s.level(&content), Some(4));
        assert_eq!(s.facts.get(&content, "karma"), Some(nettai_match::facts::Stated::Number(519)));
        assert!(notes.iter().any(|n| n.contains("level 4")), "{notes:?}");
        assert_eq!(nettai_match::check::check_side_alone(&content, &m.game, s), Vec::<String>::new());
        let hp = |m: &nettai_match::Match| {
            let mut m = m.clone();
            m.sides[1] = m.sides[0].clone();
            nettai_match::check::round_stats(&content, &m).unwrap()[0].max_hp
        };
        assert_eq!(hp(&m), 450, "the story's at level 4");
        image[0x29E0..0x29E0 + 20].copy_from_slice(b"REXE5TOK 20041006 US");
        let notes = import_file(&content, &mut m, 0, &image).unwrap();
        assert_eq!((m.sides[0].level(&content), hp(&m)), (Some(4), 450));
        assert!(notes.iter().any(|n| n.contains("its version has no such navi")), "{notes:?}");
        // A save operating MegaMan: a MegaMan side, who takes no level.
        image[0x2941] = 0;
        import_file(&content, &mut m, 0, &image).unwrap();
        assert_eq!(nettai_match::ids::local(&content.defs.navi(m.sides[0].navi(&content)).key), "megaman");
        assert_eq!(m.sides[0].level(&content), None);
    }

    /// The content names a soul by its form's id; the original's number for
    /// it is compat's (records.toml's forms). MegaMan lists his souls in
    /// the order of those numbers, 1 to 12: the order of the soul button's
    /// icons in a pack, which the frontend takes an icon by
    /// (nettai-render's `soul_place`).
    #[test]
    fn megamans_souls_are_in_the_order_of_their_numbers() {
        let content = exe5_content();
        let compat = crate::Compat::exe5();
        let megaman = content.defs.navi_by_key("megaman").unwrap();
        let souls = &content.navi(megaman).forms.as_ref().unwrap().souls;
        let numbers: Vec<Option<u8>> = souls.iter().map(|&f| compat.form_number(&content.defs.form(f).key)).collect();
        assert_eq!(numbers, (1..=12).map(Some).collect::<Vec<_>>());
        assert_eq!((compat.form(0), compat.form(7), compat.form(13)), (Some("base"), Some("colonelsoul"), None));
    }

    /// The block side `side`'s auto battle facts state (the import's way
    /// back): a place past the list empty, a record past it blank.
    fn block_of(content: &Content, side: &Side) -> AutoBattleBlock {
        use nettai_match::facts::Stated;
        let number = |h: u16| crate::Compat::exe5().chip_entry(nettai_match::ids::local(&content.defs.chip(nettai_content_api::ChipHandle(h)).key)).unwrap().id;
        let get = |fields: &[(String, Stated)], name: &str| fields.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
        let half = |fields: &[(String, Stated)]| match (get(fields, "chip"), get(fields, "pattern"), get(fields, "zero")) {
            (Some(Stated::Def(_, Some(h))), _, _) => number(h),
            (_, Some(Stated::Number(n)), _) if n > 0 => AUTO_BATTLE_PATTERN | (n - 1) as u16,
            (_, _, Some(Stated::Flag(true))) => 0,
            _ => AUTO_BATTLE_EMPTY,
        };
        let mut out = AutoBattleBlock { places: [AUTO_BATTLE_EMPTY; 42], patterns: [BLANK; RECORDS] };
        if let Some(Stated::List(items)) = side.facts.get(content, "auto_battle_places") {
            for (place, item) in out.places.iter_mut().zip(&items) {
                if let Stated::Record(f) = item {
                    *place = half(f);
                }
            }
        }
        if let Some(Stated::List(items)) = side.facts.get(content, "auto_battle_records") {
            for (p, item) in out.patterns.iter_mut().zip(&items) {
                let Stated::Record(f) = item else { continue };
                let int = |name: &str| match get(f, name) {
                    Some(Stated::Number(n)) => n,
                    _ => 0,
                };
                let mut chips = [AUTO_BATTLE_EMPTY; 5];
                if let Some(Stated::List(places)) = get(f, "chips") {
                    for (c, item) in chips.iter_mut().zip(&places) {
                        if let Stated::Record(f) = item {
                            *c = half(f);
                        }
                    }
                }
                *p = AutoBattlePattern { dx: int("dx") as i8, dy: int("dy") as i8, chips, score: int("score") as u32 };
            }
        }
        out
    }

    /// A save's auto battle data becomes the side's, place for place and
    /// record for record: chips by their numbers' names, a pattern entry by
    /// its record's number, a 0 a 0; a block nothing has written gives no
    /// data; what a match can't state is said.
    #[test]
    fn a_exe5_save_gives_the_auto_battle_data() {
        let content = exe5_content();
        let number = |name: &str| crate::Compat::exe5().chip_entry(name).unwrap().id;
        let mut image = vec![0u8; crate::save::IMAGE_SIZE];
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
        let mut m = nettai_match::Match::empty(&content, "exe5").unwrap();
        let notes = import_file(&content, &mut m, 1, &image).unwrap();
        let mut want = AutoBattleBlock { places: [AUTO_BATTLE_EMPTY; 42], patterns: [BLANK; RECORDS] };
        want.places[1] = number("areagrab");
        for i in 3..7 {
            want.places[i] = number("lance");
        }
        want.places[7] = number("sidebub3");
        want.places[33] = AUTO_BATTLE_PATTERN;
        want.patterns[0] = AutoBattlePattern { dx: -3, dy: 1, chips: [number("sword"), number("wideswrd"), AUTO_BATTLE_EMPTY, AUTO_BATTLE_EMPTY, AUTO_BATTLE_EMPTY], score: 7 };
        want.patterns[1] = AutoBattlePattern { dx: 0, dy: 0, chips: [0; 5], score: 0 };
        assert_eq!(block_of(&content, &m.sides[1]), want);
        assert!(!notes.iter().any(|n| n.contains("auto battle")), "{notes:?}");
        // (The other side: a new match's, nothing learned: every record zeros.)
        let nothing = AutoBattleBlock { places: [AUTO_BATTLE_EMPTY; 42], patterns: [AutoBattlePattern { dx: 0, dy: 0, chips: [0; 5], score: 0 }; RECORDS] };
        assert_eq!(block_of(&content, &m.sides[0]), nothing);
        assert!(!nettai_match::check_match(&content, &m).iter().any(|p| p.contains("auto battle")), "{:?}", nettai_match::check_match(&content, &m));
        // What a match can't state: a chip number the game has no chip for
        // (among the places, and in a record), a pattern past the eighth.
        put(&mut image, 8, &[0x1FF, 0, 0x8009]);
        put(&mut image, 0x5E, &[0x1FE]);
        let notes = import_file(&content, &mut m, 1, &image).unwrap();
        let said: Vec<&String> = notes.iter().filter(|n| n.contains("auto battle")).collect();
        assert_eq!(said.len(), 2, "{notes:?}");
        assert!(said[0].contains("place 7 of the save's auto battle data names pattern 10, past its eight"), "{notes:?}");
        assert!(said[1].contains("holds chip numbers exe5 has no chip for (0x1ff, 0x1fe): their places are left empty"), "{notes:?}");
        let data = block_of(&content, &m.sides[1]);
        assert_eq!(data.places[3..8], [number("lance"), AUTO_BATTLE_EMPTY, 0, AUTO_BATTLE_EMPTY, number("sidebub3")]);
        assert_eq!(data.patterns[0].chips[2], AUTO_BATTLE_EMPTY);
        // A block nothing has written: no data stated.
        image[0x554C..0x554C + 0xE0].fill(0xFF);
        import_file(&content, &mut m, 1, &image).unwrap();
        assert_eq!(block_of(&content, &m.sides[1]), AutoBattleBlock { places: [AUTO_BATTLE_EMPTY; 42], patterns: [BLANK; RECORDS] });
        // The data alone, from a save's file (the editor's "From a save").
        let mut side = m.sides[0].clone();
        assert_eq!(auto_battle_of_save(&content, "exe5", &image, &mut side), Ok(Vec::new()));
        assert_eq!(block_of(&content, &side), block_of(&content, &m.sides[1]));
        assert!(auto_battle_of_save(&content, "exe5", b"not a save", &mut side).is_err());
    }

    /// What a match states of a block is the block: a block read into a
    /// side's facts and back is the same in every place and every record
    /// (all a battle reads of it: only its count and its last eight bytes
    /// aren't stated), and so is the side written to a match file and read
    /// back. Over blocks of each awkward shape: as a battle's end writes
    /// one; a pattern that fills its record, the record after it zeros, and
    /// another, the record after it blank; a 0 among the places and among a
    /// record's chips; pattern entries out of the records' order, in the
    /// first places, one twice, and records no entry names; a full block;
    /// one with zeroed records alone; a blank one.
    #[test]
    fn what_a_match_states_of_a_block_is_the_block() {
        let content = exe5_content();
        let n = |name: &str| crate::Compat::exe5().chip_entry(name).unwrap().id;
        let none = AUTO_BATTLE_EMPTY;
        let blank = BLANK;
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
            let mut m = nettai_match::Match::empty(&content, "exe5").unwrap();
            let notes = state_auto_battle(&content, "exe5", original, &mut m.sides[0]);
            assert_eq!(notes, Vec::<String>::new(), "{name}");
            assert_eq!(block_of(&content, &m.sides[0]), *original, "{name}");
            // (And as bytes, but for the count and the last eight.)
            assert_eq!(AutoBattleBlock::read(&original.bytes()), Ok(*original), "{name}");
            // Through a match file.
            state_auto_battle(&content, "exe5", &blocks[8].1, &mut m.sides[1]);
            let text = nettai_match::write(&content, &m);
            let file: nettai_match::file::MatchFile = toml::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}\n{text}"));
            let back = nettai_match::file::resolve(&content, &file).unwrap_or_else(|e| panic!("{name}: {e:?}\n{text}"));
            assert_eq!(back.sides[0], m.sides[0], "{name}:\n{text}");
            assert_eq!(block_of(&content, &back.sides[0]), *original, "{name}");
        }
    }

    /// A save's file imported into side `side` of `m` (an EXE5 match).
    fn import_file(content: &Content, m: &mut nettai_match::Match, side: usize, file: &[u8]) -> Result<Vec<String>, String> {
        let game = m.game.clone();
        Ok(import(content, &game, &mut m.sides[side], &read(file)?))
    }
}
