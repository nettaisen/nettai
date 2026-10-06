//! EXE5's auto battle data, as a match states it for a side: what a
//! navi in auto battle plays from that player's save.
//!
//! **What it is.** An EXE5 save keeps a block of 0xE0 bytes for its player
//! (save +0x554C, the toolkit's +0x78; `nettai_battle::auto_battle`): 42
//! places, each a chip, a pattern record's number, a 0 or empty, and eight
//! pattern records, each a place by a target (`dx` columns toward the
//! auto-battling navi's enemies, `dy` rows), five chip places and the pattern's
//! score. A navi in auto battle plays it (content/exe5/rules/auto_battle): the
//! Dark MegaMan a failed Chaos Unison brings plays the data of the player
//! whose Chaos Unison failed, and a navi under DarkInvs its own side's. It
//! plays the entries in order, the played one going last: three times in
//! four the first, and otherwise, or with none, it steps into an enemy's
//! row and fires its buster.
//!
//! **Where it comes from.** The game learns it from its own player
//! alone. A battle counts each chip the player uses (0x0802C1FC, by the
//! chip's class: a standard or mega chip 1 or 3 a use, a giga chip or a
//! program advance 1) and remembers each run of chips used from one column
//! within 30 ticks of each other, two or more, with where its first hits
//! landed from there (0x0802C294, 0x0802C3E2, 0x0802C2DC: a pattern of up
//! to five chips, eight a battle, each starting at a score of 10). The
//! battle's end (0x0802C540) scores the patterns (16 kept: one seen again
//! gains 5, the others lose 1, not under 1) and writes the block from the
//! counts, its places in six lists ([`LISTS`]):
//!
//! - `first`, places 1 to 3: the three most counted standard chips of a
//!   second count (save +0x2340), which the battle's code never raises (its
//!   weight is always 0, 0x0802C220), so they are empty in every US save
//!   seen (one Japanese save has them filled: nothing here knows what
//!   filled them);
//! - `standard`, places 4 to 27: the sixteen most used standard chips, the
//!   two most used four times each, the next two twice, the rest once;
//! - `mega`, places 28 to 32: the five most used mega chips;
//! - `giga`, place 33: the most used giga chip;
//! - `patterns`, places 34 to 41: the pattern records that have a score, by
//!   number, from the first;
//! - `program_advance`, place 42: the most used program advance;
//!
//! then its eight records, the highest scored of the patterns it remembers
//! (a learned pattern's chip places past its chips are empty; a record its
//! learning never filled is zeros, [`Record::ZERO`]). So after any finished
//! battle a save's unused records are zeros
//! ([`AutoBattle::nothing_learned`] is what the write leaves of a player
//! it has learned nothing of: a new match's side, and what a random one
//! starts from). A block nothing has written is 0xFF throughout
//! ([`AutoBattle::default`]: the save's six other blocks are, and its
//! player's until its first battle ends): a side that states no data.
//!
//! **What a battle reads of it** is nearly all of it, so a match states
//! all of it:
//!
//! - *Every place.* As a battle starts each console sends its block
//!   shuffled (0x0802C7BE, `AutoBattleData::sent`): three swaps among the first
//!   three places, 39 swaps among the other 39 (each swap two places picked
//!   at random), the entries then packed to the front. That is no even
//!   shuffle: a place is in none of 39 swaps about one time in eight, so
//!   the entry in place 4 leads the sent list far more often than another,
//!   and an empty place between two entries changes what a seed sends.
//! - *A place holding 0* is no empty place: the send packs away only the
//!   empty ones (0xFFFF), so a 0 is sent as an entry, can come first, and
//!   is never played (a decision tests it as it does an empty place: a
//!   miss, then a buster run).
//! - *Every record, in its order.* The AI reads a pattern's chips to the
//!   first empty chip place with nothing else to end them (0x0802BCD6), so
//!   from a record whose five chip places are all filled it reads on: the
//!   score's low half, its high half, the next record's place bytes and
//!   that record's chip places, each as a chip's number. So a record's
//!   score, the record after it (named by an entry or not) and the records'
//!   order all show. A 0 in a chip place is played as chip 0. (To a navi
//!   that plays a pattern: with the games' own navis the test of a
//!   pattern's place, 0x0802BC48, always fails, so a pattern entry only
//!   costs the navi in auto battle a turn and its record never plays. A match
//!   states the records as the save has them all the same.)
//!
//! The lists are named for what the game writes there; any entry may stand
//! in any place, as in the block (a save made by hand can have a giga chip
//! where the game writes patterns).
//!
//! **What a match leaves out of the block**: the count at +0x54, which the
//! send writes, and the block's last eight bytes, which nothing reads. And
//! a chip number the game has no chip for can't be named.
//!
//! **What the game can't hold** is refused ([`AutoBattle::check`], and a
//! file's own reading): a list or a record with more or fewer entries than
//! it has places, a pattern number past the eighth record, a place from a
//! target or a score that doesn't fit its bytes, a chip the game hasn't,
//! and any of it for a game whose rules have no auto battle (EXE6).

use crate::ids;
use nettai_battle::content::{ChipClass, Content};
use nettai_battle::auto_battle::{AutoBattleData, AutoBattleEntry, MAX_ENTRIES, MAX_PATTERNS, PATTERN_CHIPS, PatternChip, PatternRecord};
use nettai_content_api::ChipHandle;

/// The block's places, and how many of them (the first) the send shuffles
/// apart from the rest.
pub const PLACES: usize = MAX_ENTRIES;
pub const FIRST: usize = 3;
/// The block's pattern records, and the chip places of a record.
pub const RECORDS: usize = MAX_PATTERNS;
pub const RECORD_CHIPS: usize = PATTERN_CHIPS;
/// The score a battle gives a pattern it has just seen (0x0802C4D0).
pub const NEW_SCORE: u32 = 10;

/// What the game writes into a list of the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holds {
    /// Chips of a class: its player's most used.
    Chips(ChipClass),
    /// The pattern records that have a score, by number.
    Patterns,
}

/// A list of the data: its name (a match file's key), the places it has
/// (from `start`, counting from 0), and what the game writes there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct List {
    pub name: &'static str,
    pub start: usize,
    pub len: usize,
    pub holds: Holds,
}

impl List {
    /// The list's places.
    pub fn places(&self) -> std::ops::Range<usize> {
        self.start..self.start + self.len
    }
}

/// The data's six lists, in the block's order (0x0802C692 to 0x0802C6E6).
pub const LISTS: [List; 6] = [
    List { name: "first", start: 0, len: FIRST, holds: Holds::Chips(ChipClass::Standard) },
    List { name: "standard", start: 3, len: 24, holds: Holds::Chips(ChipClass::Standard) },
    List { name: "mega", start: 27, len: 5, holds: Holds::Chips(ChipClass::Mega) },
    List { name: "giga", start: 32, len: 1, holds: Holds::Chips(ChipClass::Giga) },
    List { name: "patterns", start: 33, len: RECORDS, holds: Holds::Patterns },
    List { name: "program_advance", start: 41, len: 1, holds: Holds::Chips(ChipClass::ProgramAdvance) },
];
/// The lists by name.
pub const STANDARD: List = LISTS[1];
pub const MEGA: List = LISTS[2];
pub const GIGA: List = LISTS[3];
pub const PATTERNS: List = LISTS[4];
pub const PROGRAM_ADVANCE: List = LISTS[5];

/// The list place `place` (from 0) is in.
pub fn list_of(place: usize) -> &'static List {
    LISTS.iter().find(|l| l.places().contains(&place)).unwrap_or(&LISTS[LISTS.len() - 1])
}

/// How many times the game writes each of its player's sixteen most used
/// standard chips into the standard list (0x0802C790).
const STANDARD_TIMES: [usize; 16] = [4, 4, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];

/// One of the data's 42 places: empty (0xFFFF), a 0 (no chip, and no empty
/// place either: it is sent as an entry and never played), a chip, or a
/// pattern by its record's number (from 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Entry {
    #[default]
    Empty,
    Zero,
    Chip(ChipHandle),
    Pattern(u8),
}

/// One of a record's five chip places: empty (0xFFFF: the pattern's end), a
/// 0 (played as chip 0), or a chip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChipPlace {
    #[default]
    Empty,
    Zero,
    Chip(ChipHandle),
}

/// A pattern record, as the block has it: where the navi in auto battle stands
/// from its target (`dx` columns toward its enemies, so a negative one is
/// short of the target; `dy` rows, down the screen), its five chip places
/// (the chips it uses there, in order, to the first empty one), and its
/// score as the game's learning keeps it (a new pattern's is 10; one seen
/// again in a battle gains 5, the others lose 1), which the AI reads on
/// into from a record with no empty chip place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Record {
    pub dx: i8,
    pub dy: i8,
    pub chips: [ChipPlace; RECORD_CHIPS],
    pub score: u32,
}

impl Record {
    /// A record nothing has written: 0xFF throughout.
    pub const BLANK: Record = Record { dx: -1, dy: -1, chips: [ChipPlace::Empty; RECORD_CHIPS], score: 0xFFFF_FFFF };
    /// A record of zeros: what the game's write leaves of one its learning
    /// never filled.
    pub const ZERO: Record = Record { dx: 0, dy: 0, chips: [ChipPlace::Zero; RECORD_CHIPS], score: 0 };

    /// The chip places the AI reads within the record: to the first empty
    /// one.
    pub fn played(&self) -> &[ChipPlace] {
        let end = self.chips.iter().position(|c| *c == ChipPlace::Empty).unwrap_or(RECORD_CHIPS);
        &self.chips[..end]
    }
}

/// A player's auto battle data: the block's 42 places and its eight
/// pattern records, in order. The default is a block nothing has written
/// (every place empty, every record blank), a save's that has never
/// finished a battle: the navi in auto battle only fires its buster.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AutoBattle {
    pub places: [Entry; PLACES],
    pub records: [Record; RECORDS],
}

impl Default for AutoBattle {
    fn default() -> AutoBattle {
        AutoBattle { places: [Entry::Empty; PLACES], records: [Record::BLANK; RECORDS] }
    }
}

/// Whether the game's rules have auto battle (they drive navis no player
/// controls: a side takes the data).
pub fn has(content: &Content) -> bool {
    content.defs.rules().is_some_and(|r| r.navi_state.is_some())
}

/// Why a navi in auto battle can't play `chip`, if it can't: what the
/// game's rules say of it (their `unplayable_in_auto_battle`, from their own data:
/// EXE5's chips whose positioning class the original has no routine for,
/// where it crashes). A match's check refuses such a chip among the data's
/// 42 places (not in a pattern record, which never plays).
pub fn unplayable(content: &Content, chip: ChipHandle) -> Option<&str> {
    content.defs.unplayable_in_auto_battle(chip)
}

impl AutoBattle {
    /// Whether it is a block nothing has written (the default: what a side
    /// that states no data has).
    pub fn is_blank(&self) -> bool {
        *self == AutoBattle::default()
    }

    /// What the game's battle end writes (0x0802C540) for a player it has
    /// learned nothing of: every place empty, every record zeros. The
    /// navi in auto battle only fires its buster, as from a blank block.
    pub fn nothing_learned() -> AutoBattle {
        AutoBattle { places: [Entry::Empty; PLACES], records: [Record::ZERO; RECORDS] }
    }

    /// How many places aren't empty.
    pub fn entries(&self) -> usize {
        self.places.iter().filter(|e| **e != Entry::Empty).count()
    }

    /// A list's places.
    pub fn list(&self, list: &List) -> &[Entry] {
        &self.places[list.places()]
    }

    /// Every chip the data names: its places', then its records'.
    pub fn chips(&self) -> impl Iterator<Item = ChipHandle> + '_ {
        let places = self.places.iter().filter_map(|e| match e {
            Entry::Chip(c) => Some(*c),
            _ => None,
        });
        let records = self.records.iter().flat_map(|r| r.chips.iter()).filter_map(|c| match c {
            ChipPlace::Chip(c) => Some(*c),
            _ => None,
        });
        places.chain(records)
    }

    /// The block the engine plays (its type, in place order): each place
    /// its entry, each record as it is.
    pub fn data(&self) -> AutoBattleData {
        let entries = self
            .places
            .iter()
            .map(|e| match *e {
                Entry::Empty => AutoBattleEntry::Empty,
                Entry::Zero => AutoBattleEntry::Nothing,
                Entry::Chip(c) => AutoBattleEntry::Chip(c),
                Entry::Pattern(n) => AutoBattleEntry::Pattern(n),
            })
            .collect();
        let patterns = self
            .records
            .iter()
            .map(|r| PatternRecord {
                dx: r.dx,
                dy: r.dy,
                chips: r.chips.map(|c| match c {
                    ChipPlace::Empty => PatternChip::Empty,
                    ChipPlace::Zero => PatternChip::Nothing,
                    ChipPlace::Chip(c) => PatternChip::Chip(c),
                }),
                score: r.score,
            })
            .collect();
        AutoBattleData { entries, patterns }
    }

    /// The data as the game writes it at a battle's end (0x0802C540) from
    /// how often its player has used each chip (`uses`: a chip and its
    /// count, each chip once), having learned no pattern: the sixteen most
    /// used standard chips (the two most used four times each, the next two
    /// twice, the rest once), the five most used mega chips, the most used
    /// giga chip and the most used program advance, each list in its
    /// places; nothing in the first three places (the US games' second
    /// count stays 0) or the patterns'; every record zeros. Chips used
    /// equally often come as the game's sort leaves them (0x0814301C: the
    /// higher chip number first).
    ///
    /// The one place outside the save import that asks exe5-compat
    /// anything: the tie-break is by the original's chip numbers, which is
    /// the game's own order by definition and compat's to know.
    pub fn learned(content: &Content, uses: &[(ChipHandle, u32)]) -> AutoBattle {
        let number = |c: ChipHandle| exe5_compat::Compat::exe5().chip_entry(ids::local(&content.defs.chip(c).key)).map_or(c.0, |e| e.id);
        let most = |class: ChipClass| -> Vec<ChipHandle> {
            let mut list: Vec<(u32, u16, ChipHandle)> =
                uses.iter().filter(|(c, n)| *n > 0 && content.chip(*c).class == class).map(|&(c, n)| (n, number(c), c)).collect();
            list.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
            list.into_iter().map(|x| x.2).collect()
        };
        let mut out = AutoBattle::nothing_learned();
        let mut write = |list: &List, chips: &mut dyn Iterator<Item = ChipHandle>| {
            for (place, chip) in list.places().zip(chips) {
                out.places[place] = Entry::Chip(chip);
            }
        };
        let standard = most(ChipClass::Standard);
        write(&STANDARD, &mut standard.iter().zip(STANDARD_TIMES).flat_map(|(&chip, times)| std::iter::repeat_n(chip, times)));
        write(&MEGA, &mut most(ChipClass::Mega).into_iter());
        write(&GIGA, &mut most(ChipClass::Giga).into_iter());
        write(&PROGRAM_ADVANCE, &mut most(ChipClass::ProgramAdvance).into_iter());
        out
    }

    /// The data the game would have learned from a player who used each
    /// chip of `folder` once ([`AutoBattle::learned`], each chip counted
    /// as often as the folder holds it).
    pub fn of_folder(content: &Content, folder: &crate::Folder) -> AutoBattle {
        let mut uses: Vec<(ChipHandle, u32)> = Vec::new();
        for c in folder.chips() {
            match uses.iter_mut().find(|(id, _)| *id == c.id) {
                Some((_, n)) => *n += 1,
                None => uses.push((c.id, 1)),
            }
        }
        AutoBattle::learned(content, &uses)
    }

    /// What is wrong with the data for a side of a match of `game`: what
    /// the game can't hold (the lists' and the records' sizes are the
    /// type's), and each chip among its 42 places that its rules can't play
    /// ([`unplayable`]: the original crashes when its navi in auto battle
    /// goes to play one, and the game's own writer never puts one there),
    /// with where it is. A record's chip place may hold one: the game
    /// writes them there (a run takes any chip used), and a record never
    /// plays.
    pub fn check(&self, content: &Content, game: &str) -> Vec<String> {
        let mut out = Vec::new();
        if self.is_blank() {
            return out;
        }
        if !has(content) {
            out.push(format!("auto battle data, but {game} has no auto battle (its rules drive no navi)"));
        }
        for (i, e) in self.places.iter().enumerate() {
            if let Entry::Pattern(n) = e
                && *n as usize >= RECORDS
            {
                out.push(format!("place {} of the auto battle data names pattern {}; it has {RECORDS} pattern records", i + 1, *n as u16 + 1));
            }
        }
        let foreign = |c: ChipHandle| c.index() >= content.defs.chips.len() || !ids::in_game(content, game, &content.defs.chip(c).key);
        if self.chips().any(foreign) {
            out.push(format!("the auto battle data names a chip {game} hasn't"));
        }
        let why = |c: ChipHandle| if foreign(c) { None } else { unplayable(content, c) };
        for (i, e) in self.places.iter().enumerate() {
            if let Entry::Chip(c) = e
                && let Some(why) = why(*c)
            {
                let list = list_of(i);
                let (entry, name) = (i - list.start + 1, crate::names::chip(content, *c));
                out.push(format!("place {} of the auto battle data (`{}`, entry {entry}) holds {name}: {why}", i + 1, list.name));
            }
        }
        out
    }

    /// The data in a line, for the terminal: each list that has entries by
    /// its name, its entries by theirs up to its last (a chip in
    /// neighboring places once with its count, an empty place as `-`, a 0
    /// as `0`, a pattern as its record's number, from 1, its place from its
    /// target and its chips).
    pub fn describe(&self, content: &Content) -> String {
        let chip = |c: &ChipPlace| match c {
            ChipPlace::Empty => "-".to_string(),
            ChipPlace::Zero => "0".to_string(),
            ChipPlace::Chip(c) => crate::names::chip(content, *c).to_string(),
        };
        let mut lists = Vec::new();
        for list in &LISTS {
            let places = self.list(list);
            let Some(last) = places.iter().rposition(|e| *e != Entry::Empty) else { continue };
            let mut seen: Vec<(String, usize)> = Vec::new();
            for e in &places[..=last] {
                let name = match e {
                    Entry::Empty => "-".to_string(),
                    Entry::Zero => "0".to_string(),
                    Entry::Chip(c) => crate::names::chip(content, *c).to_string(),
                    Entry::Pattern(n) => match self.records.get(*n as usize) {
                        Some(r) => {
                            let chips: Vec<String> = r.played().iter().map(chip).collect();
                            format!("pattern {} [{}: {}]", *n as u16 + 1, place(r.dx, r.dy), chips.join(", "))
                        }
                        None => format!("pattern {}", *n as u16 + 1),
                    },
                };
                // (Only neighbors are counted together: the places' order shows.)
                match seen.last_mut() {
                    Some((n, times)) if *n == name && matches!(e, Entry::Chip(_)) => *times += 1,
                    _ => seen.push((name, 1)),
                }
            }
            let items: Vec<String> = seen.into_iter().map(|(name, n)| if n > 1 { format!("{name} x{n}") } else { name }).collect();
            lists.push(format!("{} {}", list.name.replace('_', " "), items.join(", ")));
        }
        if lists.is_empty() { "nothing learned (its buster alone)".into() } else { lists.join("; ") }
    }
}

/// A pattern's place across the field, in words: "2 columns short of its
/// target" (`dx` is toward the auto-battling navi's enemies).
pub fn across(dx: i8) -> String {
    let columns = |n: u8| if n == 1 { "1 column".to_string() } else { format!("{n} columns") };
    match dx {
        0 => "its target's column".to_string(),
        d if d < 0 => format!("{} short of its target", columns(d.unsigned_abs())),
        d => format!("{} past its target", columns(d.unsigned_abs())),
    }
}

/// A pattern's place up and down the field, in words: "1 row up".
pub fn down(dy: i8) -> String {
    let rows = |n: u8| if n == 1 { "1 row".to_string() } else { format!("{n} rows") };
    match dy {
        0 => "its target's row".to_string(),
        d if d < 0 => format!("{} up", rows(d.unsigned_abs())),
        d => format!("{} down", rows(d.unsigned_abs())),
    }
}

/// A pattern's place in words: "2 columns short of its target, 1 row up".
pub fn place(dx: i8, dy: i8) -> String {
    format!("{}, {}", across(dx), down(dy))
}

/// The auto battle data of the EXE5 save in `file` (a .sav's bytes, or a
/// raw save image), for a side of a match of `game`: what the game has
/// learned of the save's player (`crate::import_exe5`), and what is worth
/// saying about it; or why the file gives none. (The save import's: whose
/// game's the file is picks compat's reader, as in `crate::import`.)
pub fn of_save(content: &Content, game: &str, file: &[u8]) -> Result<(AutoBattle, Vec<String>), String> {
    if !has(content) {
        return Err(format!("{game} has no auto battle"));
    }
    match crate::save_game(file)? {
        exe5_compat::ROOT => {
            let save = crate::import_exe5::read(file)?;
            Ok(crate::import_exe5::auto_battle(content, game, &save.auto_battle()))
        }
        other => Err(format!("a save of {other}, which keeps no auto battle data")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{exe5_content, exe6_content};
    use nettai_battle::Rng;

    fn chip(content: &Content, name: &str) -> ChipHandle {
        ids::chip(content, "exe5", name).unwrap_or_else(|| panic!("exe5 has no {name}"))
    }

    /// The data with these entries in these places (from 0), its records
    /// blank.
    fn data(entries: &[(usize, Entry)]) -> AutoBattle {
        let mut out = AutoBattle::default();
        for (place, e) in entries {
            out.places[*place] = *e;
        }
        out
    }

    /// A chip the game's rules can't play in auto battle (the content's
    /// own answer, from its data's positioning classes: a team navi's own
    /// chip, the chips past the library) is refused among the 42 places,
    /// with where it is and why; a pattern record may hold one (a save
    /// can), and a chip the AI plays is fine anywhere.
    #[test]
    fn a_chip_auto_battle_cant_play_is_refused_among_the_places() {
        let content = exe5_content();
        let (step, capsule, cannon) = (chip(&content, "stepswrd"), chip(&content, "pnkcapsl"), chip(&content, "cannon"));
        let why = "the original can't play it in auto battle (positioning class 255 is past the game's table of them: the game crashes)";
        assert_eq!((unplayable(&content, step), unplayable(&content, capsule), unplayable(&content, cannon)), (Some(why), Some(why), None));
        let mut d = data(&[(3, Entry::Chip(cannon)), (28, Entry::Chip(step)), (41, Entry::Chip(capsule))]);
        d.records = [Record::ZERO; RECORDS];
        d.records[2] = Record { dx: 1, dy: 0, chips: [ChipPlace::Chip(step), ChipPlace::Chip(cannon), ChipPlace::Empty, ChipPlace::Empty, ChipPlace::Empty], score: 10 };
        assert_eq!(
            d.check(&content, "exe5"),
            [
                format!("place 29 of the auto battle data (`mega`, entry 2) holds {}: {why}", crate::names::chip(&content, step)),
                format!("place 42 of the auto battle data (`program_advance`, entry 1) holds {}: {why}", crate::names::chip(&content, capsule)),
            ]
        );
        d.places[28] = Entry::Empty;
        d.places[41] = Entry::Empty;
        assert_eq!(d.check(&content, "exe5"), Vec::<String>::new(), "a record may hold one");
        // EXE6's rules have no auto battle: they say nothing of any chip.
        let six = exe6_content();
        assert!((0..six.defs.chips.len() as u16).all(|c| unplayable(&six, ChipHandle(c)).is_none()));
    }

    /// The lists are the block's 42 places, in order, none shared.
    #[test]
    fn the_lists_are_the_blocks_places() {
        let mut next = 0;
        for list in &LISTS {
            assert_eq!(list.start, next, "{}", list.name);
            next += list.len;
        }
        assert_eq!(next, PLACES);
        assert_eq!((STANDARD.name, MEGA.name, GIGA.name, PATTERNS.name, PROGRAM_ADVANCE.name), ("standard", "mega", "giga", "patterns", "program_advance"));
        assert_eq!((list_of(0).name, list_of(3).name, list_of(32).name, list_of(40).name, list_of(41).name), ("first", "standard", "giga", "patterns", "program_advance"));
        assert_eq!(STANDARD_TIMES.iter().sum::<usize>(), STANDARD.len);
        assert_eq!((Record::BLANK.played().len(), Record::ZERO.played().len()), (0, RECORD_CHIPS));
        assert!(AutoBattle::default().is_blank() && AutoBattle::default().entries() == 0);
    }

    /// Why a match states the places: the send's swaps are no even shuffle.
    /// The entry in place 4 leads what the other 39 places send far more
    /// often than one in 39 (a place is in none of the 39 swaps about one
    /// time in eight), where the empty places are changes what a seed
    /// sends, and a 0 is sent as an entry where an empty place is packed
    /// away.
    #[test]
    fn the_places_show_in_what_is_sent() {
        let content = exe5_content();
        let chips: Vec<ChipHandle> =
            (0..content.defs.chips.len() as u16).map(ChipHandle).filter(|&c| !content.chip(c).codes.is_empty()).take(39).collect();
        let mut full = AutoBattle::default();
        for (i, &c) in chips.iter().enumerate() {
            full.places[FIRST + i] = Entry::Chip(c);
        }
        let full = full.data();
        let seeds = 4000u32;
        let leads = (0..seeds).filter(|&s| full.sent(&mut Rng::new(s.wrapping_mul(0x9E37_79B9) ^ 0xA5A5_5A5A)).entries[0] == AutoBattleEntry::Chip(chips[0])).count();
        // (An even shuffle: about 100 of 4,000. The swaps: about 600.)
        assert!(leads > 400, "{leads} of {seeds}");
        // Two chips in places 4 and 5, or in places 28 and 42: the same
        // entries in the same order, sent differently by some seeds.
        let two = |a: usize, b: usize| data(&[(a, Entry::Chip(chips[0])), (b, Entry::Chip(chips[1]))]).data();
        let differ = (0..200u32).filter(|&s| two(3, 4).sent(&mut Rng::new(s)).entries != two(27, 41).sent(&mut Rng::new(s)).entries).count();
        assert!(differ > 20, "{differ} of 200 seeds");
        // A 0 is an entry of what is sent; an empty place isn't.
        let zero = data(&[(3, Entry::Zero), (9, Entry::Chip(chips[0]))]).data().sent(&mut Rng::new(7));
        assert_eq!(zero.entries.len(), 2);
        assert!(zero.entries.contains(&AutoBattleEntry::Nothing));
        assert_eq!(AutoBattle::default().data().sent(&mut Rng::new(3)).entries, Vec::new());
    }

    /// What the engine plays of the data: each place its entry, each record
    /// a pattern by its number, as it is.
    #[test]
    fn the_engine_plays_the_datas_places_and_records() {
        let content = exe5_content();
        let (cannon, sword) = (chip(&content, "cannon"), chip(&content, "sword"));
        let mut d = data(&[(1, Entry::Chip(cannon)), (5, Entry::Zero), (33, Entry::Pattern(2)), (34, Entry::Pattern(0)), (41, Entry::Chip(sword))]);
        d.records[0] = Record { dx: -2, dy: 1, chips: [ChipPlace::Chip(sword), ChipPlace::Chip(cannon), ChipPlace::Empty, ChipPlace::Chip(cannon), ChipPlace::Empty], score: 7 };
        d.records[2] = Record { dx: -1, dy: 0, chips: [ChipPlace::Chip(cannon); 5], score: 10 };
        d.records[3] = Record::ZERO;
        let block = d.data();
        assert_eq!(block.entries.len(), PLACES);
        let filled: Vec<(usize, AutoBattleEntry)> = block.entries.iter().copied().enumerate().filter(|(_, e)| *e != AutoBattleEntry::Empty).collect();
        assert_eq!(filled, [(1, AutoBattleEntry::Chip(cannon)), (5, AutoBattleEntry::Nothing), (33, AutoBattleEntry::Pattern(2)), (34, AutoBattleEntry::Pattern(0)), (41, AutoBattleEntry::Chip(sword))]);
        assert_eq!(block.patterns.len(), RECORDS);
        let places = [PatternChip::Chip(sword), PatternChip::Chip(cannon), PatternChip::Empty, PatternChip::Chip(cannon), PatternChip::Empty];
        assert_eq!(block.patterns[0], PatternRecord { dx: -2, dy: 1, chips: places, score: 7 });
        assert_eq!(block.patterns[2], PatternRecord::of(-1, 0, &[cannon; 5], 10));
        assert_eq!(block.patterns[3], PatternRecord { dx: 0, dy: 0, chips: [PatternChip::Nothing; 5], score: 0 });
        // (A blank record is the engine's unused one.)
        assert_eq!(block.patterns[1], PatternRecord::UNUSED);
        assert_eq!(AutoBattle::default().data().patterns, [PatternRecord::UNUSED; RECORDS]);
        assert_eq!((d.entries(), d.chips().count()), (5, 2 + 3 + 5));
        assert_eq!(d.check(&content, "exe5"), Vec::<String>::new());
    }

    /// What the game can't hold is said.
    #[test]
    fn what_the_game_cant_hold_is_refused() {
        let content = exe5_content();
        let cannon = chip(&content, "cannon");
        let has = |d: &AutoBattle, said: &str| {
            let problems = d.check(&content, "exe5");
            assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        };
        has(&data(&[(33, Entry::Pattern(8))]), "place 34 of the auto battle data names pattern 9; it has 8 pattern records");
        has(&data(&[(0, Entry::Chip(ChipHandle(u16::MAX)))]), "names a chip exe5 hasn't");
        let mut foreign = AutoBattle::default();
        foreign.records[7].chips[4] = ChipPlace::Chip(ChipHandle(u16::MAX));
        has(&foreign, "names a chip exe5 hasn't");
        // Every place filled, any class anywhere, every record full: the
        // block holds it. So are records of zeros, named or not.
        let mut full = AutoBattle { places: [Entry::Chip(cannon); PLACES], records: [Record { dx: -5, dy: 2, chips: [ChipPlace::Chip(cannon); 5], score: u32::MAX }; RECORDS] };
        full.places[33] = Entry::Pattern(7);
        assert_eq!(full.check(&content, "exe5"), Vec::<String>::new());
        let mut zeros = data(&[(3, Entry::Chip(cannon))]);
        zeros.records = [Record::ZERO; RECORDS];
        assert_eq!(zeros.check(&content, "exe5"), Vec::<String>::new());
        zeros.places[33] = Entry::Pattern(1);
        assert_eq!(zeros.check(&content, "exe5"), Vec::<String>::new());
        // EXE6 has no auto battle.
        let six = exe6_content();
        let cannon6 = ids::chip(&six, "exe6", "cannon").unwrap();
        let problems = data(&[(0, Entry::Chip(cannon6))]).check(&six, "exe6");
        assert_eq!(problems, ["auto battle data, but exe6 has no auto battle (its rules drive no navi)"]);
        assert!(AutoBattle::default().check(&six, "exe6").is_empty());
        assert!(super::has(&content) && !super::has(&six));
    }

    /// The game's own write of the block from its counts: sixteen standard
    /// chips by use (4, 4, 2, 2, then once each) from place 4, five megas
    /// from place 28, a giga in place 33 and a program advance in place 42,
    /// its records zeros; equal counts by the higher chip number.
    #[test]
    fn the_data_is_learned_as_the_game_writes_it() {
        let content = exe5_content();
        let of = |class: ChipClass| -> Vec<ChipHandle> {
            (0..content.defs.chips.len() as u16)
                .map(ChipHandle)
                .filter(|&c| content.chip(c).class == class && ids::in_game(&content, "exe5", &content.defs.chip(c).key))
                .collect()
        };
        let (standard, mega, giga, advances) = (of(ChipClass::Standard), of(ChipClass::Mega), of(ChipClass::Giga), of(ChipClass::ProgramAdvance));
        assert!(standard.len() >= 18 && mega.len() >= 6 && giga.len() >= 2 && advances.len() >= 2);
        // Eighteen standard chips used 18, 17, ... times, six megas, two
        // gigas, two program advances.
        let counted = |chips: &[ChipHandle]| -> Vec<(ChipHandle, u32)> { chips.iter().enumerate().map(|(i, &c)| (c, (chips.len() - i) as u32)).collect() };
        let uses = [counted(&standard[..18]), counted(&mega[..6]), counted(&giga[..2]), counted(&advances[..2])].concat();
        let d = AutoBattle::learned(&content, &uses);
        let mut want = AutoBattle::nothing_learned();
        let mut place = STANDARD.start;
        for (i, &c) in standard[..16].iter().enumerate() {
            for _ in 0..STANDARD_TIMES[i] {
                want.places[place] = Entry::Chip(c);
                place += 1;
            }
        }
        assert_eq!(place, MEGA.start);
        for (i, &c) in mega[..5].iter().enumerate() {
            want.places[MEGA.start + i] = Entry::Chip(c);
        }
        want.places[GIGA.start] = Entry::Chip(giga[0]);
        want.places[PROGRAM_ADVANCE.start] = Entry::Chip(advances[0]);
        assert_eq!(d, want);
        assert!(d.list(&LISTS[0]).iter().chain(d.list(&PATTERNS)).all(|e| *e == Entry::Empty));
        assert_eq!(d.check(&content, "exe5"), Vec::<String>::new());
        // Equal counts: the higher chip number first (Cannon is chip 1,
        // HiCannon 2). Two standard chips alone: places 4 to 11, the mega
        // chips' places empty.
        let (cannon, hicannon) = (chip(&content, "cannon"), chip(&content, "hicannon"));
        let tied = AutoBattle::learned(&content, &[(cannon, 2), (hicannon, 2)]);
        assert_eq!(tied.places[3..11], [[Entry::Chip(hicannon); 4], [Entry::Chip(cannon); 4]].concat());
        assert_eq!(tied.entries(), 8);
        // No chip used: what the write leaves of nothing learned, which is
        // no blank block (its records are zeros).
        assert_eq!(AutoBattle::learned(&content, &[(cannon, 0)]), AutoBattle::nothing_learned());
        assert!(!AutoBattle::nothing_learned().is_blank() && AutoBattle::nothing_learned().entries() == 0);
        assert_eq!(AutoBattle::nothing_learned().describe(&content), "nothing learned (its buster alone)");
    }

    /// The terminal's line: the lists with entries by name, an empty place
    /// before an entry and a 0 shown, a pattern by its record.
    #[test]
    fn the_data_is_described_by_its_lists() {
        let content = exe5_content();
        let (cannon, sword) = (chip(&content, "cannon"), chip(&content, "sword"));
        let mut d = data(&[(3, Entry::Chip(cannon)), (4, Entry::Chip(cannon)), (6, Entry::Chip(sword)), (7, Entry::Zero), (33, Entry::Pattern(1))]);
        d.records[1] = Record { dx: -2, dy: 1, chips: [ChipPlace::Chip(sword), ChipPlace::Zero, ChipPlace::Chip(cannon), ChipPlace::Empty, ChipPlace::Empty], score: 7 };
        assert_eq!(
            d.describe(&content),
            "standard Cannon x2, -, Sword, 0; patterns pattern 2 [2 columns short of its target, 1 row down: Sword, 0, Cannon]"
        );
        assert_eq!(AutoBattle::default().describe(&content), "nothing learned (its buster alone)");
        assert_eq!(place(0, 0), "its target's column, its target's row");
        assert_eq!(place(1, -2), "1 column past its target, 2 rows up");
    }
}
