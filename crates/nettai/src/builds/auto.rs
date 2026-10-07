//! Auto battle data (EXE5's): what a navi in auto battle plays from the
//! side's save, the Dark MegaMan a failed Chaos Unison brings and the
//! side's own navi under DarkInvs. The creator's own view, chosen by the
//! data's names: the setup's `auto_battle_places` (42 places, each
//! `{ chip }`, `{ pattern }` from 1, `{ zero = true }` or `{}` empty) and
//! `auto_battle_records` (eight records, `{ dx, dy, chips, score }`), and
//! the game's module `rules/auto_battle/block` as it loaded: its `PLACES`,
//! `RECORDS`, `RECORD_CHIPS` and `LISTS` (each `{ name, start, len }`, its
//! start from 1). A list named for a chip class holds what the game writes
//! there. Any chip may stand in any place, as in a save.
//!
//! A build in the app states only the places, each a chip (or a program
//! advance), a 0 or empty: its records are the rules' default, nothing
//! learned (`crate::builds::layout::at_defaults`), so no place points at
//! one ([`drop_patterns`]).

use nettai_battle::Content;
use nettai_battle::content::ChipClass;
use nettai_battle::rules::Fact;
use nettai_content_api::{ChipHandle, Registry, Value};
use nettai_match::Side;
use nettai_match::facts::Stated;
use serde::Deserialize;
use serde::de::IntoDeserializer;

/// The game's module that states the block's layout.
pub const MODULE: &str = "rules/auto_battle/block";

pub const PLACES: usize = 42;
pub const RECORDS: usize = 8;
pub const RECORD_CHIPS: usize = 5;
pub const PLACES_FIELD: &str = "auto_battle_places";
pub const RECORDS_FIELD: &str = "auto_battle_records";

/// What the game writes into a list of the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holds {
    Chips(ChipClass),
    Patterns,
    Any,
}

/// A list of the data: its name, its places (from `start`, from 0), and
/// what the game writes there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    pub name: String,
    pub start: usize,
    pub len: usize,
    pub holds: Holds,
}

impl List {
    pub fn places(&self) -> std::ops::Range<usize> {
        self.start..self.start + self.len
    }
}

/// The block's lists, as the game's module states them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    pub lists: Vec<List>,
}

impl Layout {
    /// The layout `block` (the game's module, as data) states, where it is
    /// the one this view draws: 42 places in lists, eight records of five
    /// chips; and only where the game's setup takes the data.
    pub fn read(content: &Content, block: Option<&nettai_content_api::Data>) -> Option<Layout> {
        if !(nettai_match::facts::takes(content, PLACES_FIELD) && nettai_match::facts::takes(content, RECORDS_FIELD)) {
            return None;
        }
        let d = block?;
        let size = |k: &str| d.field(k).int().map(|n| n as usize);
        if (size("PLACES"), size("RECORDS"), size("RECORD_CHIPS")) != (Some(PLACES), Some(RECORDS), Some(RECORD_CHIPS)) {
            return None;
        }
        let nettai_content_api::Data::List(items) = d.field("LISTS") else { return None };
        let mut lists = Vec::new();
        for l in items {
            let name = l.field("name").str()?.to_string();
            let (start, len) = (l.field("start").int()? as usize, l.field("len").int()? as usize);
            let holds = match name.as_str() {
                "patterns" => Holds::Patterns,
                n => {
                    let class: Result<ChipClass, serde::de::value::Error> = ChipClass::deserialize(n.into_deserializer());
                    class.map_or(Holds::Any, Holds::Chips)
                }
            };
            lists.push(List { name, start: start.checked_sub(1)?, len, holds });
        }
        let mut next = 0;
        for l in &lists {
            if l.start != next {
                return None;
            }
            next += l.len;
        }
        (next == PLACES).then_some(Layout { lists })
    }

    /// The list place `place` (from 0) is in.
    pub fn list_of(&self, place: usize) -> Option<&List> {
        self.lists.iter().find(|l| l.places().contains(&place))
    }
}

/// Whether setup field `name` is one of the data's (this view shows them).
pub fn owns(layout: Option<&Layout>, name: &str) -> bool {
    layout.is_some() && matches!(name, PLACES_FIELD | RECORDS_FIELD)
}

/// One of the 42 places: empty, a 0 (no chip, and no empty place either),
/// a chip, or a pattern by its record's number (from 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Entry {
    #[default]
    Empty,
    Zero,
    Chip(ChipHandle),
    Pattern(u8),
}

/// One of a record's five chip places.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChipPlace {
    #[default]
    Empty,
    Zero,
    Chip(ChipHandle),
}

/// A pattern record: where the navi in auto battle stands from its target
/// (`dx` columns toward its enemies, `dy` rows down), its five chip places,
/// and its score.
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
    #[cfg(test)]
    pub const ZERO: Record = Record { dx: 0, dy: 0, chips: [ChipPlace::Zero; RECORD_CHIPS], score: 0 };
}

/// A side's data: its 42 places and eight records.
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

impl AutoBattle {
    /// What the game's battle end writes of a player it has learned nothing
    /// of: every place empty, every record zeros.
    #[cfg(test)]
    pub fn nothing_learned() -> AutoBattle {
        AutoBattle { places: [Entry::Empty; PLACES], records: [Record::ZERO; RECORDS] }
    }

    pub fn entries(&self) -> usize {
        self.places.iter().filter(|e| **e != Entry::Empty).count()
    }

    /// A side's data: its facts, the places past the list's empty, the
    /// records past it blank.
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
        if let Some(Stated::List(items)) = side.facts.get(content, PLACES_FIELD) {
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
        if let Some(Stated::List(items)) = side.facts.get(content, RECORDS_FIELD) {
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

    /// State the data as `side`'s facts: the places to the last that isn't
    /// empty, the records to the last that isn't blank.
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
        facts.set(content, PLACES_FIELD, &places)?;
        facts.set(content, RECORDS_FIELD, &records)?;
        side.facts = facts;
        Ok(())
    }
}

/// An edit of the places (each from 0).
#[derive(Clone, Copy, Debug)]
pub enum Edit {
    /// A chip (or a program advance) into a place.
    Put(usize, ChipHandle),
    Empty(usize),
    Zero(usize),
    /// Every place emptied.
    Clear,
}

/// Apply `edit` to the side's places: whether the side changed.
pub fn update(content: &Content, side: &mut Side, edit: Edit) -> bool {
    let mut data = AutoBattle::of_side(content, side);
    let mut set = |i: usize, e: Entry| data.places.get_mut(i).is_some_and(|p| std::mem::replace(p, e) != e);
    let changed = match edit {
        Edit::Put(i, chip) => set(i, Entry::Chip(chip)),
        Edit::Empty(i) => set(i, Entry::Empty),
        Edit::Zero(i) => set(i, Entry::Zero),
        Edit::Clear => (0..PLACES).fold(false, |changed, i| set(i, Entry::Empty) || changed),
    };
    changed && data.write(content, side).is_ok()
}

/// Empty the side's places that point at a pattern record (whose records
/// are the rules' default: nothing learned): whether any did.
pub fn drop_patterns(content: &Content, side: &mut Side) -> bool {
    if !nettai_match::facts::takes(content, PLACES_FIELD) {
        return false;
    }
    let mut data = AutoBattle::of_side(content, side);
    let mut dropped = false;
    for p in &mut data.places {
        if matches!(p, Entry::Pattern(_)) {
            *p = Entry::Empty;
            dropped = true;
        }
    }
    dropped && data.write(content, side).is_ok()
}

/// Whether the game would write `entry` in `list` (a quiet note where it
/// wouldn't: a save may hold it).
pub fn like_the_game(content: &Content, list: &List, entry: Entry) -> bool {
    match (list.holds, entry) {
        (_, Entry::Empty) | (Holds::Any, _) => true,
        (Holds::Chips(class), Entry::Chip(chip)) => content.chip(chip).class == class,
        (Holds::Patterns, Entry::Pattern(_)) => true,
        _ => false,
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::ids;

    /// The layout EXE5's block module states: six lists over the 42 places.
    #[test]
    fn exe5s_block_fits() {
        let content = nettai_match::testing::exe5_content();
        let block = crate::builds::layout::modules(&content, "exe5", &[MODULE]).remove(0);
        let layout = Layout::read(&content, block.as_ref()).expect("the block's layout");
        let names: Vec<(&str, usize, usize, Holds)> = layout.lists.iter().map(|l| (l.name.as_str(), l.start, l.len, l.holds)).collect();
        assert_eq!(
            names,
            [
                ("first", 0, 3, Holds::Any),
                ("standard", 3, 24, Holds::Chips(ChipClass::Standard)),
                ("mega", 27, 5, Holds::Chips(ChipClass::Mega)),
                ("giga", 32, 1, Holds::Chips(ChipClass::Giga)),
                ("patterns", 33, 8, Holds::Patterns),
                ("program_advance", 41, 1, Holds::Chips(ChipClass::ProgramAdvance)),
            ]
        );
        let six = nettai_match::testing::exe6_content();
        assert!(Layout::read(&six, block.as_ref()).is_none(), "EXE6's setup takes no such data");
    }

    #[test]
    fn edits_change_the_data() {
        let content = nettai_match::testing::exe5_content();
        let mut m = nettai_match::pick::live(&content, "exe5", 3, None).unwrap();
        let side = &mut m.sides[0];
        let (sword, cannon) = (ids::chip(&content, "exe5", "sword").unwrap(), ids::chip(&content, "exe5", "cannon").unwrap());
        assert_eq!(AutoBattle::of_side(&content, side), AutoBattle::nothing_learned());
        assert!(!update(&content, side, Edit::Clear));
        assert!(update(&content, side, Edit::Put(3, sword)) && update(&content, side, Edit::Put(4, cannon)));
        assert!(!update(&content, side, Edit::Put(4, cannon)));
        assert_eq!(AutoBattle::of_side(&content, side).places[3..5], [Entry::Chip(sword), Entry::Chip(cannon)]);
        assert!(update(&content, side, Edit::Zero(3)));
        assert_eq!(AutoBattle::of_side(&content, side).places[3], Entry::Zero);
        assert!(update(&content, side, Edit::Empty(3)));
        // A save's place that points at a pattern: dropped.
        let mut data = AutoBattle::of_side(&content, side);
        data.places[33] = Entry::Pattern(1);
        data.write(&content, side).unwrap();
        assert!(drop_patterns(&content, side) && !drop_patterns(&content, side));
        assert_eq!(AutoBattle::of_side(&content, side).places[33], Entry::Empty);
        assert!(update(&content, side, Edit::Clear));
        assert_eq!(AutoBattle::of_side(&content, side), AutoBattle::nothing_learned());
    }
}
