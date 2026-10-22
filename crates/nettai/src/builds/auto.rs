//! Auto battle data (EXE5's): what a navi in auto battle plays from the
//! side's save, the Dark MegaMan a failed Chaos Unison brings and the
//! side's own navi under DarkInvs. The creator's own view, chosen by the
//! data's names: the setup's `auto_battle_places` (42 places, each
//! `{ chip }`, `{ pattern }` from 1, `{ zero = true }` or `{}` empty) and
//! `auto_battle_records` (eight records, `{ dx, dy, chips, score }`), and
//! the game's module `rules/auto_battle/block` as it loaded: its `PLACES`,
//! `RECORDS`, `RECORD_CHIPS` and `LISTS` (each `{ name, start, len }`, its
//! start from 1, and `own_class` where the rules hold the list to chips of
//! the class it is named for: their `validate` refuses a chip of another
//! there, and the editor offers that class's chips for it).
//!
//! A build in the app states only the places, each a chip (or a program
//! advance), a 0 or empty: its records are the rules' default, nothing
//! learned (the game's builds data's `at_defaults`: `nettai_match::build`),
//! so no place points at one ([`drop_patterns`]).

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

/// The game's strings' text table of the lists' names.
pub const LISTS_TABLE: &str = "auto_battle_lists";

pub const PLACES: usize = 42;
pub const RECORDS: usize = 8;
pub const RECORD_CHIPS: usize = 5;
pub const PLACES_FIELD: &str = "auto_battle_places";
pub const RECORDS_FIELD: &str = "auto_battle_records";

/// What a list of the data holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Holds {
    /// Chips of one class alone (the list's `own_class`, the class its name
    /// says: the rules refuse another there).
    Chips(ChipClass),
    /// Pattern entries (the game writes nothing else there).
    Patterns,
    /// A chip of any class.
    Any,
}

/// A list of the data: its name, its places (from `start`, from 0), and
/// what it holds.
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
    /// the one this view draws: 42 places in lists (where it states none,
    /// one list of them all), and where
    /// the game's setup takes pattern records, eight of five chips; and
    /// only where the game's setup takes the places.
    pub fn read(content: &Content, block: Option<&nettai_content_api::Data>) -> Option<Layout> {
        if !nettai_match::facts::takes(content, PLACES_FIELD) {
            return None;
        }
        let d = block?;
        let size = |k: &str| d.field(k).int().map(|n| n as usize);
        if size("PLACES") != Some(PLACES) {
            return None;
        }
        if nettai_match::facts::takes(content, RECORDS_FIELD) && (size("RECORDS"), size("RECORD_CHIPS")) != (Some(RECORDS), Some(RECORD_CHIPS)) {
            return None;
        }
        let items = match d.field("LISTS") {
            nettai_content_api::Data::List(items) => items.as_slice(),
            nettai_content_api::Data::Nil => return Some(Layout { lists: vec![List { name: String::new(), start: 0, len: PLACES, holds: Holds::Any }] }),
            _ => return None,
        };
        let mut lists = Vec::new();
        for l in items {
            let name = l.field("name").str()?.to_string();
            let (start, len) = (l.field("start").int()? as usize, l.field("len").int()? as usize);
            let holds = match (name.as_str(), matches!(l.field("own_class"), nettai_content_api::Data::Bool(true))) {
                ("patterns", _) => Holds::Patterns,
                (n, true) => {
                    let class: Result<ChipClass, serde::de::value::Error> = ChipClass::deserialize(n.into_deserializer());
                    Holds::Chips(class.ok()?)
                }
                (_, false) => Holds::Any,
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

/// List `list`'s name, as the game's strings have it (its id where they have none).
pub fn list_name(names: &crate::games::Names, list: &List) -> String {
    names.text(LISTS_TABLE, &list.name).unwrap_or_else(|| list.name.clone())
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
        // (A game whose setup states no records has none to write.)
        if nettai_match::facts::takes(content, RECORDS_FIELD) {
            facts.set(content, RECORDS_FIELD, &records)?;
        }
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
    /// What a place holds moved to another, the places between shifting one toward where it was (within one list).
    Move(usize, usize),
    /// What two places hold traded.
    Swap(usize, usize),
}

/// Apply `edit` to the side's places: whether the side changed.
pub fn update(content: &Content, side: &mut Side, edit: Edit) -> bool {
    let mut data = AutoBattle::of_side(content, side);
    let before = data.places;
    let places = &mut data.places;
    match edit {
        Edit::Put(i, chip) if i < PLACES => places[i] = Entry::Chip(chip),
        Edit::Empty(i) if i < PLACES => places[i] = Entry::Empty,
        Edit::Zero(i) if i < PLACES => places[i] = Entry::Zero,
        Edit::Clear => *places = [Entry::Empty; PLACES],
        Edit::Move(from, to) if from < to && to < PLACES => places[from..=to].rotate_left(1),
        Edit::Move(from, to) if to < from && from < PLACES => places[to..=from].rotate_right(1),
        Edit::Swap(a, b) if a.max(b) < PLACES => places.swap(a, b),
        _ => return false,
    }
    data.places != before && data.write(content, side).is_ok()
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

/// The chips a list held to chips of `class` is offered: every chip of
/// game `game` of that class (whether a folder holds it or not: a list of
/// Program Advances holds those no folder does).
pub fn of_class(content: &Content, game: &str, class: ChipClass) -> Vec<ChipHandle> {
    (0..content.defs.chips.len() as u16)
        .map(ChipHandle)
        .filter(|&id| nettai_match::ids::in_game(content, game, &content.defs.chip(id).key) && content.chip(id).class == class)
        .collect()
}


#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::ids;

    /// The layout EXE5's block module states: six lists over the 42 places.
    #[test]
    fn exe5s_block_fits() {
        let content = nettai_match::testing::exe5_content();
        let block = crate::builds::module_data(&content, "exe5", MODULE);
        let layout = Layout::read(&content, block.as_ref()).expect("the block's layout");
        let names: Vec<(&str, usize, usize, Holds)> = layout.lists.iter().map(|l| (l.name.as_str(), l.start, l.len, l.holds)).collect();
        assert_eq!(
            names,
            [
                ("first", 0, 3, Holds::Any),
                ("standard", 3, 24, Holds::Any),
                ("mega", 27, 5, Holds::Chips(ChipClass::Mega)),
                ("giga", 32, 1, Holds::Chips(ChipClass::Giga)),
                ("patterns", 33, 8, Holds::Patterns),
                ("program_advance", 41, 1, Holds::Chips(ChipClass::ProgramAdvance)),
            ]
        );
        let six = nettai_match::testing::exe6_content();
        assert!(Layout::read(&six, block.as_ref()).is_none(), "EXE6's setup takes no such data");
        // EXE4's block states EXE5's lists (its writer, 0x0800EFA8, writes them so).
        let four = nettai_match::testing::exe4_content();
        let block4 = crate::builds::module_data(&four, "exe4", MODULE);
        let layout4 = Layout::read(&four, block4.as_ref()).expect("EXE4's block's layout");
        assert_eq!(layout4, layout);
        let mut m = nettai_match::pick::live(&four, "exe4", 3, None).unwrap();
        let cannon = ids::chip(&four, "exe4", "cannon").unwrap();
        assert!(update(&four, &mut m.sides[0], Edit::Put(5, cannon)));
        assert_eq!(AutoBattle::of_side(&four, &m.sides[0]).places[5], Entry::Chip(cannon));
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
        // Moved down two, the places between shifting up; traded with another.
        assert!(update(&content, side, Edit::Put(3, sword)) && update(&content, side, Edit::Move(3, 5)));
        assert_eq!(AutoBattle::of_side(&content, side).places[3..6], [Entry::Chip(cannon), Entry::Empty, Entry::Chip(sword)]);
        assert!(update(&content, side, Edit::Swap(3, 5)) && !update(&content, side, Edit::Move(4, 4)));
        assert_eq!(AutoBattle::of_side(&content, side).places[3..6], [Entry::Chip(sword), Entry::Empty, Entry::Chip(cannon)]);
        assert!(update(&content, side, Edit::Move(5, 4)) && update(&content, side, Edit::Empty(3)));
        assert_eq!(AutoBattle::of_side(&content, side).places[3..6], [Entry::Empty, Entry::Chip(cannon), Entry::Empty]);
        // A save's place that points at a pattern: dropped.
        let mut data = AutoBattle::of_side(&content, side);
        data.places[33] = Entry::Pattern(1);
        data.write(&content, side).unwrap();
        assert!(drop_patterns(&content, side) && !drop_patterns(&content, side));
        assert_eq!(AutoBattle::of_side(&content, side).places[33], Entry::Empty);
        assert!(update(&content, side, Edit::Clear));
        assert_eq!(AutoBattle::of_side(&content, side), AutoBattle::nothing_learned());
    }

    /// The rules hold the mega, giga and program advance lists to chips of their class, each list's `own_class`: a chip
    /// of another class there is refused as one the place can't hold. Every chip the editor offers such a list
    /// ([`of_class`]: the game's chips of the class, the Program Advances among them, which no folder holds) the
    /// rules let stand there. The first and standard lists take a chip of any class. In EXE4 and EXE5 alike.
    #[test]
    fn a_list_held_to_a_class_takes_its_class_alone() {
        use nettai_match::Wrong;
        for (content, game) in [(nettai_match::testing::exe5_content(), "exe5"), (nettai_match::testing::exe4_content(), "exe4")] {
            let block = crate::builds::module_data(&content, game, MODULE);
            let layout = Layout::read(&content, block.as_ref()).expect("the block's layout");
            let mut m = nettai_match::pick::live(&content, game, 3, None).unwrap();
            let side = &mut m.sides[0];
            let problems = |side: &Side| -> Vec<(Wrong, Option<usize>)> {
                let checked = nettai_match::build::Checked::of(&content, game, side);
                checked.problems.into_iter().filter(|p| p.field.as_deref() == Some(PLACES_FIELD)).map(|p| (p.what, p.entry)).collect()
            };
            let held: Vec<(&List, ChipClass)> = layout
                .lists
                .iter()
                .filter_map(|l| match l.holds {
                    Holds::Chips(class) => Some((l, class)),
                    _ => None,
                })
                .collect();
            let names: Vec<(&str, ChipClass)> = held.iter().map(|(l, class)| (l.name.as_str(), *class)).collect();
            assert_eq!(names, [("mega", ChipClass::Mega), ("giga", ChipClass::Giga), ("program_advance", ChipClass::ProgramAdvance)], "{game}");
            let cannon = ids::chip(&content, game, "cannon").unwrap();
            for (list, class) in held {
                let offered = of_class(&content, game, class);
                assert!(offered.len() > 10, "{game} {}: {offered:?}", list.name);
                for batch in offered.chunks(list.len) {
                    for (place, &chip) in list.places().zip(batch) {
                        update(&content, side, Edit::Put(place, chip));
                    }
                    assert_eq!(problems(side), [], "{game} {}: {batch:?}", list.name);
                }
                // A standard chip there: refused.
                update(&content, side, Edit::Put(list.start, cannon));
                assert_eq!(problems(side), [(Wrong::ChipNotAllowed { chip: cannon }, Some(list.start))], "{game} {}", list.name);
                update(&content, side, Edit::Clear);
            }
            // The first and standard lists: a mega chip and a program advance.
            let (mega, advance) = (of_class(&content, game, ChipClass::Mega)[0], of_class(&content, game, ChipClass::ProgramAdvance)[0]);
            for (place, chip) in [(0, mega), (1, advance), (3, mega), (4, advance)] {
                update(&content, side, Edit::Put(place, chip));
            }
            assert_eq!(problems(side), [], "{game}");
        }
    }
}
