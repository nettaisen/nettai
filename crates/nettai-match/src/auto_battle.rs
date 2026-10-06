//! EXE5's auto battle data, as a match states it for a side: what a
//! navi in auto battle plays from that player's save. A side states it as
//! two facts of EXE5's rules' setup (`auto_battle_places`, the block's 42
//! places, and `auto_battle_records`, its eight records:
//! content/exe5/rules/auto_battle/block.luau, which also sends it as the
//! round is set up and checks it, their `validate`); [`AutoBattle`] is a
//! typed view of them for the editor's pane and the game's learning
//! ([`AutoBattle::learned`]), which read and write a side's facts through
//! it ([`AutoBattle::of_side`], [`AutoBattle::write`]).
//!
//! **What it is.** An EXE5 save keeps a block of 0xE0 bytes for its player
//! (save +0x554C, the toolkit's +0x78): 42
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
//!   shuffled (0x0802C7BE, the rules' `send`): three swaps among the first
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
//! **What the game can't hold** is refused: by the facts' types (a list
//! longer than the block's, a place from a target or a score that doesn't
//! fit its bytes), the match's checks (a chip the game hasn't, any of it for
//! a game whose rules take no such facts, EXE6's), and the rules' `validate`
//! (a pattern number past the eighth record, a chip the AI can't play among
//! the places).

use crate::facts::Stated;
use crate::{Side, ids};
use nettai_battle::content::{ChipClass, Content};
use nettai_battle::rules::Fact;
use nettai_content_api::{ChipHandle, Registry, Value};

/// The block's places, and how many of them (the first) the send shuffles
/// apart from the rest (EXE5's rules' `block.PLACES`, `block.FIRST`).
pub const PLACES: usize = 42;
pub const FIRST: usize = 3;
/// The block's pattern records, and the chip places of a record.
pub const RECORDS: usize = 8;
pub const RECORD_CHIPS: usize = 5;
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

    /// A side's data: its facts (EXE5's rules' `auto_battle_places` and
    /// `auto_battle_records`), the places past the list's empty, the records
    /// past it blank; a side of a game whose rules take none, blank.
    pub fn of_side(content: &Content, side: &Side) -> AutoBattle {
        let mut out = AutoBattle::default();
        let field = |fields: &[(String, Stated)], name: &str| fields.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
        let chip = |v: Option<Stated>| match v {
            Some(Stated::Def(Registry::Chip, Some(h))) => Some(ChipHandle(h)),
            _ => None,
        };
        let number = |v: Option<Stated>| match v {
            Some(Stated::Number(n)) => n,
            _ => 0,
        };
        let flag = |v: Option<Stated>| v == Some(Stated::Flag(true));
        if let Some(Stated::List(items)) = side.facts.get(content, "auto_battle_places") {
            for (place, item) in out.places.iter_mut().zip(&items) {
                let Stated::Record(fields) = item else { continue };
                let pattern = number(field(fields, "pattern"));
                *place = match chip(field(fields, "chip")) {
                    Some(c) => Entry::Chip(c),
                    None if pattern > 0 => Entry::Pattern((pattern - 1) as u8),
                    None if flag(field(fields, "zero")) => Entry::Zero,
                    None => Entry::Empty,
                };
            }
        }
        if let Some(Stated::List(items)) = side.facts.get(content, "auto_battle_records") {
            for (record, item) in out.records.iter_mut().zip(&items) {
                let Stated::Record(fields) = item else { continue };
                let mut chips = [ChipPlace::Empty; RECORD_CHIPS];
                if let Some(Stated::List(places)) = field(fields, "chips") {
                    for (place, item) in chips.iter_mut().zip(&places) {
                        let Stated::Record(f) = item else { continue };
                        *place = match chip(field(f, "chip")) {
                            Some(c) => ChipPlace::Chip(c),
                            None if flag(field(f, "zero")) => ChipPlace::Zero,
                            None => ChipPlace::Empty,
                        };
                    }
                }
                let (dx, dy) = (number(field(fields, "dx")) as i8, number(field(fields, "dy")) as i8);
                *record = Record { dx, dy, chips, score: number(field(fields, "score")) as u32 };
            }
        }
        out
    }

    /// State the data as `side`'s facts (`auto_battle_places` to its last
    /// place that isn't empty, `auto_battle_records` to its last record
    /// that isn't blank). An error where the game's rules take no such facts.
    pub fn write(&self, content: &Content, side: &mut Side) -> Result<(), String> {
        let def = |c: ChipHandle| Fact::Value(Value::Def(Registry::Chip, c.0));
        let yes = || Fact::Value(Value::Bool(true));
        let last = self.places.iter().rposition(|e| *e != Entry::Empty).map_or(0, |i| i + 1);
        let places: Vec<Fact> = self.places[..last]
            .iter()
            .map(|e| {
                Fact::Record(match *e {
                    Entry::Empty => Vec::new(),
                    Entry::Zero => vec![("zero", yes())],
                    Entry::Chip(c) => vec![("chip", def(c))],
                    Entry::Pattern(n) => vec![("pattern", Fact::Value(Value::Int(n as i64 + 1)))],
                })
            })
            .collect();
        let last = self.records.iter().rposition(|r| *r != Record::BLANK).map_or(0, |i| i + 1);
        let records: Vec<Fact> = self.records[..last]
            .iter()
            .map(|r| {
                let chips: Vec<Fact> = r
                    .chips
                    .iter()
                    .map(|c| {
                        Fact::Record(match *c {
                            ChipPlace::Empty => Vec::new(),
                            ChipPlace::Zero => vec![("zero", yes())],
                            ChipPlace::Chip(c) => vec![("chip", def(c))],
                        })
                    })
                    .collect();
                Fact::Record(vec![
                    ("dx", Fact::Value(Value::Int(r.dx as i64))),
                    ("dy", Fact::Value(Value::Int(r.dy as i64))),
                    ("chips", Fact::List(chips)),
                    ("score", Fact::Value(Value::Int(r.score as i64))),
                ])
            })
            .collect();
        let mut facts = side.facts.clone();
        facts.set(content, "auto_battle_places", &places)?;
        facts.set(content, "auto_battle_records", &records)?;
        side.facts = facts;
        Ok(())
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
    use crate::testing::exe5_content;

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
