//! The Auto battle pane (EXE5's): what a navi in auto battle plays from the
//! side's save, the Dark MegaMan the side's failed Chaos Unison brings and
//! the side's own navi under DarkInvs. The editor's own view, chosen by the
//! data's names (`crate::editor::layout`): the setup's
//! `auto_battle_places` (42 places, each `{ chip }`, `{ pattern }` from 1,
//! `{ zero = true }` or `{}` empty) and `auto_battle_records` (eight
//! records, `{ dx, dy, chips, score }`), and the game's module
//! `rules/auto_battle/block` as it loaded: its `PLACES`, `RECORDS`,
//! `RECORD_CHIPS` and `LISTS` (each `{ name, start, len }`, its start from
//! 1). A list named for a chip class holds what the game writes there.
//!
//! It shows the save's block whole, as a match states it: on the left its
//! 42 places, in the six lists the game writes them in (the first three,
//! the standard chips, the mega chips, the giga chip, the patterns, the
//! program advance), each place a chip, a pattern record's number, a 0 or
//! empty; in the middle its eight pattern records, each its place from the
//! target, its five chip places and its score (a plain table: with the
//! games' navis a pattern entry only costs the navi in auto battle a turn, and
//! its record's contents don't play); on the right the game's chips,
//! searched, where a chip is put into the selected place (of the 42, or of
//! a record), as the folder pane fills a folder. Any entry may stand
//! in any place, as in a save; one the game wouldn't write in its list is
//! quietly marked. The data can be taken from an EXE5 save.

use crate::editor::app::{Choice, Editor, Msg};
use crate::editor::view::{DIM, GREEN, RED, SIDES, class_letter, class_name, heading, icon};
use iced::widget::{Column, Row, button, checkbox, column, image, pick_list, row, scrollable, space, text, text_input};
use iced::{Alignment, Color, Element, Length};
use nettai_battle::Content;
use nettai_battle::content::{ChipClass, ChipFlags};
use nettai_battle::rules::Fact;
use nettai_content_api::{ChipHandle, Registry, Value};
use nettai_match::Side;
use nettai_match::facts::Stated;

// ---- The data, as the setup states it --------------------------------------------------

/// The game's module that states the block's layout.
pub const MODULE: &str = "rules/auto_battle/block";

/// The block's places, its pattern records, and a record's chip places.
pub const PLACES: usize = 42;
pub const RECORDS: usize = 8;
pub const RECORD_CHIPS: usize = 5;
/// The score a battle gives a pattern it has just seen (0x0802C4D0).
const NEW_SCORE: u32 = 10;
/// The setup's fields.
const PLACES_FIELD: &str = "auto_battle_places";
const RECORDS_FIELD: &str = "auto_battle_records";

/// What the game writes into a list of the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holds {
    /// Chips of a class: its player's most used.
    Chips(ChipClass),
    /// The pattern records that have a score, by number.
    Patterns,
    /// Nothing the game says.
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
    /// The layout `block` (the game's rules/auto_battle/block, as data)
    /// states, where it is the one this pane draws: 42 places in lists, eight
    /// records of five chips.
    pub fn read(block: Option<&nettai_content_api::Data>) -> Option<Layout> {
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
                n => serde_json::from_value::<ChipClass>(serde_json::Value::String(n.to_string())).map_or(Holds::Any, Holds::Chips),
            };
            lists.push(List { name, start: start.checked_sub(1)?, len, holds });
        }
        // (The lists are the 42 places, in order.)
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

    /// The place a pane starts on: the standard chips' first, else the first.
    pub fn first_place(&self) -> usize {
        self.lists.iter().find(|l| l.holds == Holds::Chips(ChipClass::Standard) && l.len > 1).map_or(0, |l| l.start)
    }
}

/// Whether setup field `name` is one of the data's (the pane shows them,
/// not the setup's generic views): where the game's block fits the pane.
pub fn owns(layout: Option<&Layout>, name: &str) -> bool {
    layout.is_some() && matches!(name, PLACES_FIELD | RECORDS_FIELD)
}

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

/// One of a record's five chip places: empty (the pattern's end), a 0
/// (played as chip 0), or a chip.
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
    pub const ZERO: Record = Record { dx: 0, dy: 0, chips: [ChipPlace::Zero; RECORD_CHIPS], score: 0 };

    /// The chip places the AI reads within the record: to the first empty
    /// one.
    pub fn played(&self) -> &[ChipPlace] {
        let end = self.chips.iter().position(|c| *c == ChipPlace::Empty).unwrap_or(RECORD_CHIPS);
        &self.chips[..end]
    }
}

/// A side's data: its 42 places and eight records. The default is a block
/// nothing has written (every place empty, every record blank).
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
    pub fn is_blank(&self) -> bool {
        *self == AutoBattle::default()
    }

    /// What the game's battle end writes of a player it has learned nothing
    /// of: every place empty, every record zeros.
    pub fn nothing_learned() -> AutoBattle {
        AutoBattle { places: [Entry::Empty; PLACES], records: [Record::ZERO; RECORDS] }
    }

    pub fn entries(&self) -> usize {
        self.places.iter().filter(|e| **e != Entry::Empty).count()
    }

    /// Every chip the data names: its places', then its records'.
    pub fn chips(&self) -> impl Iterator<Item = ChipHandle> + '_ {
        let places = self.places.iter().filter_map(|e| if let Entry::Chip(c) = e { Some(*c) } else { None });
        let records = self.records.iter().flat_map(|r| r.chips.iter()).filter_map(|c| if let ChipPlace::Chip(c) = c { Some(*c) } else { None });
        places.chain(records)
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

// ---- The pane ---------------------------------------------------------------------------

/// What is selected: one of the 42 places (from 0), or a chip place of a
/// record (the record, then the place, from 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selected {
    Place(usize),
    Chip(usize, usize),
}

/// What the pane keeps of its own: what is selected (none: the layout's
/// first place), and whether the chip list shows every chip (else those
/// the game writes in the selected place's list).
#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub selected: Option<Selected>,
    pub every_chip: bool,
}

impl State {
    /// What is selected, on `layout`.
    pub fn selected(&self, layout: &Layout) -> Selected {
        self.selected.unwrap_or(Selected::Place(layout.first_place()))
    }
}

#[derive(Clone, Debug)]
pub enum Edit {
    Select(Selected),
    /// A chip from the list: into the selected place, on to the next.
    Put(ChipHandle),
    /// The selected place emptied, or made a 0.
    Empty,
    Zero,
    /// The selected place (of the 42) made this pattern record's number.
    Pattern(u8),
    /// A record's place from its target, and its score as typed.
    Dx(usize, i8),
    Dy(usize, i8),
    Score(usize, String),
    /// A record set whole (zeros, or blank).
    Record(usize, Record),
    /// The data of an EXE5 save (the window asks which: `Editor::update`).
    FromSave,
    /// What the game writes of nothing learned (empty places, zeroed
    /// records), or no data at all (a block nothing has written).
    NothingLearned,
    NoData,
    EveryChip(bool),
}

/// Apply `edit` to the side's data (its facts, `AutoBattle::of_side`), on
/// `layout`: whether the match changed.
pub fn update(content: &Content, layout: &Layout, side: &mut Side, state: &mut State, edit: Edit) -> bool {
    let mut data = AutoBattle::of_side(content, side);
    let changed = edit_data(layout, &mut data, state, edit);
    changed && data.write(content, side).is_ok()
}

/// Apply `edit` to `d`: whether it changed.
fn edit_data(layout: &Layout, d: &mut AutoBattle, state: &mut State, edit: Edit) -> bool {
    let set = |d: &mut AutoBattle, selected: Selected, place: Entry, chip: ChipPlace| match selected {
        Selected::Place(i) => d.places.get_mut(i).is_some_and(|p| std::mem::replace(p, place) != place),
        Selected::Chip(r, k) => d.records.get_mut(r).and_then(|r| r.chips.get_mut(k)).is_some_and(|p| std::mem::replace(p, chip) != chip),
    };
    match edit {
        Edit::Select(selected) => {
            state.selected = Some(selected);
            false
        }
        Edit::EveryChip(on) => {
            state.every_chip = on;
            false
        }
        Edit::Put(chip) => {
            let selected = state.selected(layout);
            let changed = set(d, selected, Entry::Chip(chip), ChipPlace::Chip(chip));
            // On to the next place, as one fills a folder.
            state.selected = Some(match selected {
                Selected::Place(i) => Selected::Place((i + 1).min(PLACES - 1)),
                Selected::Chip(r, k) => Selected::Chip(r, (k + 1).min(RECORD_CHIPS - 1)),
            });
            changed
        }
        Edit::Empty => set(d, state.selected(layout), Entry::Empty, ChipPlace::Empty),
        Edit::Zero => set(d, state.selected(layout), Entry::Zero, ChipPlace::Zero),
        Edit::Pattern(n) => match state.selected(layout) {
            Selected::Place(i) if (n as usize) < RECORDS => set(d, Selected::Place(i), Entry::Pattern(n), ChipPlace::Empty),
            _ => false,
        },
        Edit::Dx(r, dx) => d.records.get_mut(r).is_some_and(|r| std::mem::replace(&mut r.dx, dx) != dx),
        Edit::Dy(r, dy) => d.records.get_mut(r).is_some_and(|r| std::mem::replace(&mut r.dy, dy) != dy),
        // (Nothing typed is 0; what is no number is not taken.)
        Edit::Score(r, typed) => match (d.records.get_mut(r), if typed.trim().is_empty() { Ok(0) } else { typed.trim().parse::<u32>() }) {
            (Some(r), Ok(score)) => std::mem::replace(&mut r.score, score) != score,
            _ => false,
        },
        Edit::Record(r, record) => d.records.get_mut(r).is_some_and(|r| std::mem::replace(r, record) != record),
        Edit::NothingLearned => std::mem::replace(d, AutoBattle::nothing_learned()) != AutoBattle::nothing_learned(),
        Edit::NoData => std::mem::replace(d, AutoBattle::default()) != AutoBattle::default(),
        // (The window's: it asks for the file.)
        Edit::FromSave => false,
    }
}

/// A list's heading: what the game writes there, and its places.
fn list_title(list: &List) -> String {
    let what = match list.holds {
        Holds::Chips(ChipClass::Standard) => "Standard chips".to_string(),
        Holds::Chips(ChipClass::Mega) => "Mega chips".to_string(),
        Holds::Chips(ChipClass::Giga) => "Giga chip".to_string(),
        Holds::Chips(ChipClass::ProgramAdvance) => "Program advance".to_string(),
        Holds::Patterns => "Patterns".to_string(),
        _ => crate::editor::facts::title(&list.name),
    };
    if list.len == 1 { format!("{what} (place {})", list.start + 1) } else { format!("{what} (places {} to {})", list.start + 1, list.start + list.len) }
}

/// What the game writes of a class, in words.
fn class_words(class: ChipClass) -> &'static str {
    match class {
        ChipClass::Standard => "a standard chip",
        ChipClass::Mega => "a mega chip",
        ChipClass::Giga => "a giga chip",
        ChipClass::ProgramAdvance => "a program advance",
        _ => "another chip",
    }
}

/// The quiet note for an entry the game wouldn't write in its list (no
/// error: a save holds it).
fn unlike_the_game(content: &Content, list: &List, entry: Entry) -> Option<String> {
    match (list.holds, entry) {
        (_, Entry::Empty) | (Holds::Any, _) => None,
        (Holds::Chips(class), Entry::Chip(chip)) if content.chip(chip).class == class => None,
        (Holds::Patterns, Entry::Pattern(_)) => None,
        (Holds::Chips(class), _) => Some(format!("the game writes {} here", class_words(class))),
        (Holds::Patterns, _) => Some("the game writes a pattern here".into()),
    }
}

/// A record's chips in a line, as the AI reads them (to the first empty
/// place).
fn record_line(e: &Editor, r: &Record) -> String {
    let chips: Vec<String> = r
        .played()
        .iter()
        .map(|c| match c {
            ChipPlace::Chip(c) => e.names.chip(&e.content, *c),
            _ => "0".to_string(),
        })
        .collect();
    if chips.is_empty() { "no chips".to_string() } else { chips.join(", ") }
}

/// The chips an entry can be: the game's that a folder holds (those with a
/// code) and its program advances, which are what the game writes into the
/// data (its most used standard, mega and giga chips and program advance).
/// (One a navi in auto battle can't play among the 42 places, the rules'
/// `validate` says, with the match's other problems.)
fn chips(e: &Editor, _place: bool) -> Vec<ChipHandle> {
    let c = &e.content;
    (0..c.defs.chips.len() as u16)
        .map(ChipHandle)
        .filter(|&h| nettai_match::ids::in_game(c, e.m.game(), &c.defs.chip(h).key))
        .filter(|&h| {
            let d = c.chip(h);
            match d.class {
                ChipClass::Standard | ChipClass::Mega | ChipClass::Giga => !d.codes.is_empty(),
                ChipClass::ProgramAdvance => true,
                _ => false,
            }
        })
        .collect()
}

/// A choice of a place from the target: the field's own (so many columns
/// or rows either way) and the record's, if it is another.
fn places_from(now: i8, most: i8, label: fn(i8) -> String) -> (Vec<Choice<i8>>, Option<Choice<i8>>) {
    let mut values: Vec<i8> = (-most..=most).collect();
    if !values.contains(&now) {
        values.push(now);
        values.sort();
    }
    let choices: Vec<Choice<i8>> = values.into_iter().map(|v| Choice { label: label(v), value: v }).collect();
    let picked = choices.iter().find(|x| x.value == now).cloned();
    (choices, picked)
}

/// The furthest a place can be from a target on a field of six columns and
/// three rows.
const MAX_DX: i8 = 5;
const MAX_DY: i8 = 2;

pub fn view(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let Some(layout) = e.auto_battle_layout.as_ref() else { return space().into() };
    let d = &AutoBattle::of_side(c, e.side(s));
    let state = e.auto_battle[s];
    let selected = state.selected(layout);
    let msg = move |edit: Edit| Msg::AutoBattle(s, edit);
    let pick = |selected: bool| if selected { button::secondary } else { button::text };

    // The 42 places, by list.
    let mut places = Column::new().spacing(1);
    for list in &layout.lists {
        places = places.push(text(list_title(list)).size(13).color(DIM));
        for i in list.places() {
            let entry = d.places[i];
            let number = text(format!("{:>2}", i + 1)).size(12).color(DIM).width(Length::Fixed(22.0));
            let blank = || space().width(16).height(16);
            let line = match entry {
                Entry::Chip(chip) => row![number, icon(e, chip), text(e.names.chip(c, chip)).size(14).width(Length::Fill)],
                Entry::Pattern(n) => {
                    let what = d.records.get(n as usize).map_or(String::new(), |r| record_line(e, r));
                    row![number, text("▸").size(14).width(Length::Fixed(16.0)), text(format!("Pattern {}: {what}", n as u16 + 1)).size(14).width(Length::Fill)]
                }
                Entry::Zero => row![number, blank(), text("0").size(14).width(Length::Fill)],
                Entry::Empty => row![number, blank(), text("(empty)").size(14).color(DIM).width(Length::Fill)],
            }
            .spacing(6)
            .align_y(Alignment::Center);
            // (Under the entry: the quiet note.)
            let under = |note: String, color| row![space().width(Length::Fixed(50.0)), text(note).size(11).color(color).width(Length::Fill)];
            let line: Element<Msg> = match unlike_the_game(c, list, entry) {
                Some(note) => column![line, under(note, DIM)].into(),
                None => line.into(),
            };
            let b = button(line).width(Length::Fill).padding([1, 4]).on_press(msg(Edit::Select(Selected::Place(i))));
            places = places.push(b.style(pick(selected == Selected::Place(i))));
        }
        places = places.push(space().height(Length::Fixed(6.0)));
    }
    let left = column![
        heading(format!("{}: auto battle", SIDES[s])),
        text(
            "What a navi in auto battle plays from this player's save: the Dark MegaMan their failed Chaos Unison brings, and their own navi \
             under DarkInvs. EXE5 learns it from the chips its player uses, and keeps it as these 42 places and eight pattern records."
        )
        .size(13)
        .color(DIM),
        row![
            button(text("From a save…").size(13)).on_press(msg(Edit::FromSave)).style(button::secondary),
            button(text("Nothing learned").size(13)).on_press(msg(Edit::NothingLearned)).style(button::secondary),
            button(text("No data").size(13)).on_press(msg(Edit::NoData)).style(button::secondary),
        ]
        .spacing(4)
        .wrap(),
        text(if d.is_blank() {
            "No data: a save that never finished a battle (the match file states none).".to_string()
        } else {
            format!("{} of {} places hold an entry", d.entries(), PLACES)
        })
        .size(13)
        .color(DIM),
        scrollable(row![places.width(Length::Fill), space().width(Length::Fixed(14.0))]).height(Length::Fill),
    ]
    .spacing(6);

    // The eight pattern records.
    let mut records = Column::new().spacing(8);
    for (n, r) in d.records.iter().enumerate() {
        let (across, dx) = places_from(r.dx, MAX_DX, across);
        let (down, dy) = places_from(r.dy, MAX_DY, down);
        let named: Vec<String> = d.places.iter().enumerate().filter(|(_, e)| **e == Entry::Pattern(n as u8)).map(|(i, _)| (i + 1).to_string()).collect();
        let named = if named.is_empty() { "no place names it".to_string() } else { format!("named by place {}", named.join(", ")) };
        let mut its = Row::new().spacing(2);
        let cant = Column::new();
        for (k, chip) in r.chips.iter().enumerate() {
            let label: Element<Msg> = match chip {
                ChipPlace::Chip(chip) => row![icon(e, *chip), text(e.names.chip(c, *chip)).size(13)].spacing(4).align_y(Alignment::Center).into(),
                ChipPlace::Zero => text("0").size(13).into(),
                ChipPlace::Empty => text("(empty)").size(13).color(DIM).into(),
            };
            let b = button(label).padding([1, 6]).on_press(msg(Edit::Select(Selected::Chip(n, k))));
            its = its.push(b.style(pick(selected == Selected::Chip(n, k))));
        }
        records = records.push(
            column![
                row![
                    text(format!("Pattern {}", n + 1)).size(14).width(Length::Fixed(70.0)),
                    text(named).size(11).color(DIM).width(Length::Fill),
                    button(text("zeros").size(11)).padding([1, 5]).on_press(msg(Edit::Record(n, Record::ZERO))).style(button::text),
                    button(text("blank").size(11)).padding([1, 5]).on_press(msg(Edit::Record(n, Record::BLANK))).style(button::text),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
                row![
                    pick_list(across, dx, move |x: Choice<i8>| msg(Edit::Dx(n, x.value))).text_size(12),
                    pick_list(down, dy, move |y: Choice<i8>| msg(Edit::Dy(n, y.value))).text_size(12),
                ]
                .spacing(6),
                its.wrap(),
                cant,
                row![
                    text("score").size(12).color(DIM),
                    text_input("0", &r.score.to_string()).size(12).on_input(move |t| msg(Edit::Score(n, t))).width(Length::Fixed(110.0)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            ]
            .spacing(3),
        );
    }
    let middle = column![
        text("The eight pattern records").size(16),
        text(format!(
            "A record is where the navi in auto battle would stand from its target, the chips it would use there in a row and how the game ranks \
             the pattern (a new one's score is {}). With the games' navis a pattern entry only ever costs the navi in auto battle a turn (the \
             game's test of the place always fails), so a record's contents don't play: they are the save's, kept as they are.",
            NEW_SCORE
        ))
        .size(12)
        .color(DIM),
        scrollable(row![records.width(Length::Fill), space().width(Length::Fixed(14.0))]).height(Length::Fill),
    ]
    .spacing(6);

    // What is selected.
    let selected_chip = match selected {
        Selected::Place(i) => match d.places.get(i) {
            Some(Entry::Chip(chip)) => Some(*chip),
            _ => None,
        },
        Selected::Chip(r, k) => match d.records.get(r).and_then(|r| r.chips.get(k)) {
            Some(ChipPlace::Chip(chip)) => Some(*chip),
            _ => None,
        },
    };
    let holds = match selected_chip {
        Some(chip) => e.names.chip(c, chip),
        None => match selected {
            Selected::Place(i) => match d.places.get(i) {
                Some(Entry::Pattern(n)) => format!("pattern {}", *n as u16 + 1),
                Some(Entry::Zero) => "0".into(),
                _ => "empty".into(),
            },
            Selected::Chip(r, k) => match d.records.get(r).and_then(|r| r.chips.get(k)) {
                Some(ChipPlace::Zero) => "0".into(),
                _ => "empty".into(),
            },
        },
    };
    let title = match selected {
        Selected::Place(i) => format!("Place {}: {holds}", i + 1),
        Selected::Chip(r, k) => format!("Pattern {}, chip {}: {holds}", r + 1, k + 1),
    };
    let picture: Element<Msg> = match selected_chip.and_then(|chip| e.pictures.chip(&c.defs.chip(chip).key)).and_then(|p| p.art.clone()) {
        Some(h) => image(h).width(112).height(96).filter_method(image::FilterMethod::Nearest).into(),
        None => space().width(112).height(96).into(),
    };
    let facts = selected_chip.map_or(String::new(), |chip| {
        let sd = c.chip(chip);
        format!("{} · {} MB · {} damage", class_name(sd.class), sd.mb, sd.damage)
    });
    let mut makes = row![
        button("Empty").on_press(msg(Edit::Empty)).style(button::secondary),
        button("0").on_press(msg(Edit::Zero)).style(button::secondary),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if let Selected::Place(i) = selected {
        let patterns: Vec<Choice<u8>> = (0..RECORDS as u8).map(|n| Choice { label: format!("pattern {}", n + 1), value: n }).collect();
        let now = match d.places.get(i) {
            Some(Entry::Pattern(n)) => patterns.iter().find(|x| x.value == *n).cloned(),
            _ => None,
        };
        makes = makes.push(pick_list(patterns, now, move |x: Choice<u8>| msg(Edit::Pattern(x.value))).placeholder("a pattern").text_size(13));
    }
    let about = row![
        picture,
        column![
            text(title).size(15),
            text(facts).size(13).color(DIM),
            makes,
            text("0: no chip, but no empty place: among the 42 it is sent and never played; in a pattern it is played as chip 0.").size(11).color(DIM),
        ]
        .spacing(4),
    ]
    .spacing(10);

    // The chips, searched: those the game writes in the place's list (a
    // record's chip place takes any), or every chip.
    let wanted = match (selected, state.every_chip) {
        (Selected::Place(i), false) => match layout.list_of(i).map(|l| l.holds) {
            Some(Holds::Chips(class)) => Some(class),
            _ => None,
        },
        _ => None,
    };
    let needle = e.search.to_lowercase();
    let mut pool: Vec<(String, ChipHandle)> = chips(e, matches!(selected, Selected::Place(_)))
        .into_iter()
        .filter(|&h| wanted.is_none_or(|class| c.chip(h).class == class))
        .map(|h| (e.names.chip(c, h), h))
        .filter(|(name, h)| needle.is_empty() || name.to_lowercase().contains(&needle) || nettai_match::ids::local(&c.defs.chip(*h).key).contains(&needle))
        .collect();
    e.order.chips(c, &mut pool);
    let mut choices = Column::new().spacing(1);
    for (name, h) in pool.into_iter().take(400) {
        let cd = c.chip(h);
        let times = d.chips().filter(|&x| x == h).count();
        let dark = cd.flags.has(ChipFlags::DARK);
        choices = choices.push(
            row![
                button(text("put").size(12)).padding([1, 6]).on_press(msg(Edit::Put(h))).style(button::secondary),
                icon(e, h),
                text(name).size(14).width(Length::Fill).color(if dark { RED } else { Color::BLACK }),
                text(format!("{} {} MB", class_letter(cd.class), cd.mb)).size(12).color(DIM).width(Length::Fixed(64.0)),
                text(if times > 0 { format!("×{times}") } else { String::new() }).size(12).color(GREEN).width(Length::Fixed(34.0)),
                space().width(Length::Fixed(12.0)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }
    let listed = match wanted {
        Some(class) => format!("Listed: what the game writes here, {}.", class_words(class)),
        None => "Listed: every chip.".to_string(),
    };
    let right = column![
        about,
        row![
            text_input("search chips", &e.search).on_input(Msg::Search),
            checkbox(state.every_chip).label("Every chip").on_toggle(move |on| msg(Edit::EveryChip(on))),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        text(format!(
            "{listed} A chip in several places is played that much more often. The places matter: the game shuffles the first three among \
             themselves and the others among themselves as a battle starts, but lightly, so the early places of each tend to be played first."
        ))
        .size(12)
        .color(DIM),
        scrollable(choices).height(Length::Fill),
    ]
    .spacing(6);
    row![left.width(Length::FillPortion(4)), middle.width(Length::FillPortion(5)), right.width(Length::FillPortion(5))].spacing(12).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_match::ids;

    /// The layout EXE5's block module states: six lists over the 42 places,
    /// the standard chips' first at place 4 (a pane starts there), the mega
    /// chips', the giga chip's, the patterns' and the program advance's.
    #[test]
    fn exe5s_block_fits_the_pane() {
        let content = nettai_match::testing::exe5_content();
        let block = crate::editor::layout::modules(&content, "exe5", &[MODULE]).remove(0);
        let layout = Layout::read(block.as_ref()).expect("the block's layout");
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
        assert_eq!(layout.first_place(), 3);
        assert!(Layout::read(None).is_none());
    }

    /// What the pane's edits do to a side's data: a chip put into the
    /// selected place and on to the next, a place made a pattern's number,
    /// a 0 or empty, a record's place, chips and score, a record set whole,
    /// and the data taken away.
    #[test]
    fn the_panes_edits_change_the_data() {
        let content = nettai_match::testing::exe5_content();
        let block = crate::editor::layout::modules(&content, "exe5", &[MODULE]).remove(0);
        let layout = Layout::read(block.as_ref()).unwrap();
        let mut m = nettai_match::pick::live(&content, "exe5", 3, None).unwrap();
        let side = &mut m.sides[0];
        let mut state = State::default();
        let (sword, cannon) = (ids::chip(&content, "exe5", "sword").unwrap(), ids::chip(&content, "exe5", "cannon").unwrap());
        let edit = |side: &mut Side, state: &mut State, e: Edit| update(&content, &layout, side, state, e);
        // (A random match states nothing learned.)
        assert_eq!(AutoBattle::of_side(&content, side), AutoBattle::nothing_learned());
        assert!(!edit(side, &mut state, Edit::NothingLearned));
        assert_eq!(state.selected(&layout), Selected::Place(3));
        assert!(edit(side, &mut state, Edit::Put(sword)) && edit(side, &mut state, Edit::Put(cannon)));
        assert_eq!((AutoBattle::of_side(&content, side).places[3], AutoBattle::of_side(&content, side).places[4], state.selected(&layout)), (Entry::Chip(sword), Entry::Chip(cannon), Selected::Place(5)));
        // Place 34 names pattern 2; its record's chips, place and score.
        edit(side, &mut state, Edit::Select(Selected::Place(33)));
        assert!(edit(side, &mut state, Edit::Pattern(1)) && !edit(side, &mut state, Edit::Pattern(8)));
        assert_eq!(AutoBattle::of_side(&content, side).places[33], Entry::Pattern(1));
        edit(side, &mut state, Edit::Select(Selected::Chip(1, 0)));
        assert!(edit(side, &mut state, Edit::Put(sword)));
        assert_eq!(state.selected(&layout), Selected::Chip(1, 1));
        assert!(edit(side, &mut state, Edit::Empty) && edit(side, &mut state, Edit::Zero) && !edit(side, &mut state, Edit::Pattern(2)));
        assert!(edit(side, &mut state, Edit::Dx(1, -2)) && edit(side, &mut state, Edit::Dy(1, 1)) && edit(side, &mut state, Edit::Score(1, " 12 ".into())));
        assert!(!edit(side, &mut state, Edit::Score(1, "twelve".into())) && !edit(side, &mut state, Edit::Dx(1, -2)));
        let places = [ChipPlace::Chip(sword), ChipPlace::Zero, ChipPlace::Zero, ChipPlace::Zero, ChipPlace::Zero];
        assert_eq!(AutoBattle::of_side(&content, side).records[1], Record { dx: -2, dy: 1, chips: places, score: 12 });
        assert!(edit(side, &mut state, Edit::Score(1, String::new())));
        assert_eq!(AutoBattle::of_side(&content, side).records[1].score, 0);
        assert!(edit(side, &mut state, Edit::Record(1, Record::BLANK)) && !edit(side, &mut state, Edit::Record(1, Record::BLANK)));
        assert_eq!(AutoBattle::of_side(&content, side).records[1], Record::BLANK);
        // A place made a 0, then empty.
        edit(side, &mut state, Edit::Select(Selected::Place(3)));
        assert!(edit(side, &mut state, Edit::Zero));
        assert_eq!(AutoBattle::of_side(&content, side).places[3], Entry::Zero);
        assert!(edit(side, &mut state, Edit::Empty) && !edit(side, &mut state, Edit::Empty));
        // A chip put into the last place stays there.
        edit(side, &mut state, Edit::Select(Selected::Place(PLACES - 1)));
        assert!(edit(side, &mut state, Edit::Put(cannon)));
        assert_eq!(state.selected(&layout), Selected::Place(PLACES - 1));
        // Nothing learned, then no data.
        assert!(edit(side, &mut state, Edit::NothingLearned));
        assert_eq!(AutoBattle::of_side(&content, side), AutoBattle::nothing_learned());
        assert!(edit(side, &mut state, Edit::NoData) && AutoBattle::of_side(&content, side).is_blank());
        assert!(!edit(side, &mut state, Edit::FromSave) && !edit(side, &mut state, Edit::EveryChip(true)) && state.every_chip);
    }
}
