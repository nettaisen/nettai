//! What a side brings that its game's rules take: its facts.
//!
//! A fact is a field of the setup of the game's rules, as the
//! content declares it (`setup = { version = { "gregar", "falzar" },
//! crosses = "form[5]" }`, `setup = { karma = "u16" }`): its name, its
//! type, its default (`setup_defaults`; zero without one, and an enum or a
//! list of definitions unstated). A side holds its facts as those systems' setup blocks
//! ([`Facts`]), which the round's setup hands the engine as they are. A
//! field several systems declare is one fact, written into each.
//!
//! This crate names no game's fact. Which facts there are, what a side
//! that says nothing has, and which a round can't start without (an enum
//! or a list of definitions nothing states: no variant is assumed, and an
//! empty list is a statement) are the content's; a match file states a fact under its
//! field's name, and the editor shows one by its type. Three the engine
//! itself knows by role (`PlayerFact`: the version, Beast Out, the form
//! list), and where this crate needs one of those (the version's place in a
//! navi's stats, the forms a navi's list may hold) it asks by the role.
//!
//! The games' own (EXE6's `version`, `crosses`, `beast_out`,
//! `bug_frags`; EXE5's `karma`, `souls`, `soul_unison`, `chaos_unison`) are
//! documented where they are declared: content/exe6/rules and
//! content/exe5/rules.

use crate::{Folder, Side, ids};
use nettai_battle::content::{ChipCode, Content, PlayerFact};
use nettai_battle::custom::FolderChip;
use nettai_battle::rules::{self, Fact, SetupFact};
use nettai_content_api::{Block, ChipHandle, FieldType, FieldValue, FormHandle, NaviHandle, Registry, Value};

/// A side's facts: its setup of its game's rules, one block (none on a
/// content without rules).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Facts(Option<Block>);

/// A fact a side of the content's game takes: a setup field's name and type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field<'c> {
    pub name: &'c str,
    pub ty: &'c FieldType,
}

/// The facts a side of the content's game takes: each field of its rules'
/// setup, in its names' order (a schema sorts them).
pub fn fields(content: &Content) -> Vec<Field<'_>> {
    let Some(rules) = content.defs.rules() else { return Vec::new() };
    content.defs.schema(rules.setup).fields().iter().map(|f| Field { name: &f.name, ty: &f.ty }).collect()
}

/// The fact named `name`, if the game's rules take it.
pub fn field<'c>(content: &'c Content, name: &str) -> Option<Field<'c>> {
    fields(content).into_iter().find(|f| f.name == name)
}

/// Whether the game's rules' setup declares field `field` (a side takes
/// that fact).
pub fn takes(content: &Content, field: &str) -> bool {
    self::field(content, field).is_some()
}

/// The facts' names in a phrase, for a message: "crosses, version,
/// beast_out".
pub fn names_phrase(content: &Content) -> String {
    let names: Vec<&str> = fields(content).iter().map(|f| f.name).collect();
    if names.is_empty() { "none".to_string() } else { names.join(", ") }
}

/// A fact's value, read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stated {
    Flag(bool),
    Number(i64),
    /// An enum's variant, by its name; none: nothing stated.
    Variant(Option<String>),
    /// A definition of a registry, by its handle; none.
    Def(Registry, Option<u16>),
    /// An array's elements.
    List(Vec<Stated>),
    /// A number or none (a `u8?`: a navi code's level).
    Optional(Option<i64>),
    /// A record's fields, by name, in their order.
    Record(Vec<(String, Stated)>),
    /// A chip code, by its letter (`*` too); none: no code.
    Code(Option<char>),
    /// A value no match states (an object, a vector, an asset).
    Other,
}

impl Stated {
    /// The definitions a list holds (or the one a field names), by their
    /// handles, in order, its holes left out.
    pub fn defs(&self) -> Vec<u16> {
        match self {
            Stated::Def(_, Some(h)) => vec![*h],
            Stated::List(items) => items.iter().flat_map(Stated::defs).collect(),
            _ => Vec::new(),
        }
    }
}

/// The value at `place` of a setup's block, read: a record's fields, an
/// array's elements, a list's (its length's), a value.
fn stated_at(block: &Block, place: nettai_content_api::Place) -> Stated {
    match place.ty() {
        FieldType::Record(fields) => {
            Stated::Record(fields.fields().iter().enumerate().map(|(i, f)| (f.name.clone(), stated_at(block, place.field_at(i)))).collect())
        }
        FieldType::Array(..) | FieldType::List(..) => {
            let n = block.len_at(place).unwrap_or(0);
            Stated::List((0..n).filter_map(|k| place.elem(k)).map(|p| stated_at(block, p)).collect())
        }
        ty => stated_of(ty, block.get_at(place)),
    }
}

/// `src`'s value at `place` into `dst` (blocks of one layout), whole.
fn copy(dst: &mut Block, src: &Block, place: nettai_content_api::Place) {
    match place.ty() {
        FieldType::Record(fields) => {
            for i in 0..fields.fields().len() {
                copy(dst, src, place.field_at(i));
            }
        }
        FieldType::Array(..) | FieldType::List(..) => {
            let n = src.len_at(place).unwrap_or(0);
            if let FieldType::List(..) = place.ty() {
                let _ = dst.set_len_at(place, n);
            }
            for k in 0..place.capacity().unwrap_or(0) {
                if let Some(p) = place.elem(k) {
                    copy(dst, src, p);
                }
            }
        }
        _ => {
            let _ = dst.set_at(place, src.get_at(place).load());
        }
    }
}

/// Whether a value `f` of type `ty` fits it: its numbers within their
/// types (the engine's store would wrap one), a list's elements, a record's
/// fields.
fn in_range(ty: &FieldType, f: &Fact) -> Result<(), String> {
    match (ty, f) {
        (_, Fact::Value(Value::Int(n))) => match range(ty) {
            Some((least, most, what)) if !(least..=most).contains(n) => Err(format!("{n} is past a {what} ({least} to {most})")),
            _ => Ok(()),
        },
        (FieldType::Array(elem, _) | FieldType::List(elem, _), Fact::List(items)) => items.iter().try_for_each(|x| in_range(elem, x)),
        (FieldType::Record(fields), Fact::Record(entries)) => entries.iter().try_for_each(|(name, x)| match fields.index_of(name) {
            Some(i) => in_range(&fields.field(i).ty, x),
            None => Ok(()),
        }),
        _ => Ok(()),
    }
}

fn stated_of(ty: &FieldType, v: FieldValue) -> Stated {
    match (ty, v) {
        (_, FieldValue::Bool(b)) => Stated::Flag(b),
        (_, FieldValue::OptionalU8(v)) => Stated::Optional(v.map(i64::from)),
        (FieldType::Enum(names), FieldValue::Enum(i)) => Stated::Variant(names.get(i as usize).cloned()),
        (FieldType::Ref(registry, _), FieldValue::Ref(r)) => Stated::Def(*registry, r.map(|(_, h)| h)),
        (_, FieldValue::Code(c)) => Stated::Code(c.map(char::from)),
        (_, FieldValue::U8(_) | FieldValue::U16(_) | FieldValue::U32(_) | FieldValue::I8(_) | FieldValue::I16(_) | FieldValue::I32(_)) => {
            match v.load() {
                Value::Int(i) => Stated::Number(i),
                _ => Stated::Other,
            }
        }
        _ => Stated::Other,
    }
}

impl Facts {
    /// A setup's block as facts (a test's, the binary's).
    pub(crate) fn of_block(block: Block) -> Facts {
        Facts(Some(block))
    }

    /// What a side that says nothing has: the rules' defaults.
    pub fn defaults(content: &Content) -> Facts {
        Facts(rules::default_setup(content))
    }

    /// The setup of the rules, as a round's setup carries it.
    pub fn block(&self) -> Option<&Block> {
        self.0.as_ref()
    }

    /// Whether these are the content's game's: a block of its rules'
    /// setup (none, for content without rules).
    pub fn fit(&self, content: &Content) -> bool {
        match (&self.0, content.defs.rules()) {
            (Some(b), Some(r)) => b.id() == r.setup,
            (None, None) => true,
            _ => false,
        }
    }

    /// Write fact `field`: one value for a field, an element each for an
    /// array (the rest zero), an enum's by its name (`Fact::Name`). An
    /// error names what is wrong: the game's rules take no such field, a
    /// value isn't the field's, or a number is past its type (the
    /// engine's own store wraps one to the width). Nothing is written then.
    pub fn set(&mut self, content: &Content, field: &str, values: &[Fact]) -> Result<(), String> {
        let Some(declared) = self::field(content, field) else { return Err(no_field(content, field)) };
        let of = match declared.ty {
            FieldType::Array(elem, _) | FieldType::List(elem, _) => elem,
            ty => ty,
        };
        for v in values {
            in_range(of, v)?;
        }
        let mut block = self.0.clone();
        match rules::set_fact(&mut block, content, field, values)? {
            false => Err(no_field(content, field)),
            true => {
                self.0 = block;
                Ok(())
            }
        }
    }

    /// Fact `field` back at what a side that says nothing has.
    pub fn reset(&mut self, content: &Content, field: &str) {
        let (Some(rules), Some(block)) = (content.defs.rules(), self.0.as_mut()) else { return };
        let default = rules.setup_block();
        let schema = content.defs.schema(rules.setup);
        let Some(i) = schema.index_of(field) else { return };
        if !default.stated(schema, i) {
            block.unstate(schema, i);
        } else {
            copy(block, &default, schema.place(i));
        }
    }

    /// Fact `field`, as the engine reads one ([`SetupFact`]).
    pub fn fact<'a>(&'a self, content: &'a Content, field: &str) -> Option<SetupFact<'a>> {
        rules::fact_in(self.0.as_ref()?, content, field)
    }

    /// Fact `field`'s value; none: the game's rules take no such fact.
    pub fn get(&self, content: &Content, field: &str) -> Option<Stated> {
        let fact = self.fact(content, field)?;
        Some(match fact.ty() {
            ty if !fact.stated() => match ty {
                FieldType::Enum(_) => Stated::Variant(None),
                _ => Stated::Other,
            },
            _ => stated_at(fact.block(), fact.place()),
        })
    }

    /// The SP navi deletion times (the rules' fact the engine knows as
    /// `PlayerFact::SpTimes`): each SP chip and its frames, in the list's
    /// order; none where the game's rules take no such fact.
    pub fn sp_times(&self, content: &Content) -> Vec<(ChipHandle, u16)> {
        let Some(Stated::List(items)) = content.defs.fact_field(PlayerFact::SpTimes).and(self.get(content, PlayerFact::SpTimes.name())) else {
            return Vec::new();
        };
        items
            .iter()
            .filter_map(|item| {
                let Stated::Record(fields) = item else { return None };
                let chip = fields.iter().find_map(|(n, v)| match (n.as_str(), v) {
                    ("chip", Stated::Def(Registry::Chip, Some(h))) => Some(ChipHandle(*h)),
                    _ => None,
                })?;
                let frames = fields.iter().find_map(|(n, v)| match (n.as_str(), v) {
                    ("frames", Stated::Number(f)) => Some(*f as u16),
                    _ => None,
                })?;
                Some((chip, frames))
            })
            .collect()
    }

    /// Write the SP navi deletion times (`PlayerFact::SpTimes`): each chip
    /// and its frames, in this order.
    pub fn set_sp_times(&mut self, content: &Content, times: &[(ChipHandle, u16)]) -> Result<(), String> {
        let records: Vec<Fact> = times
            .iter()
            .map(|&(c, f)| Fact::Record(vec![("chip", Fact::Value(Value::Def(Registry::Chip, c.0))), ("frames", Fact::Value(Value::Int(f as i64)))]))
            .collect();
        self.set(content, PlayerFact::SpTimes.name(), &records)
    }

    /// Whether fact `field` is what a side that says nothing has.
    pub fn is_default(&self, content: &Content, field: &str) -> bool {
        self.get(content, field) == Facts::defaults(content).get(content, field)
    }

    /// The fact the engine knows by `role`, where the game's rules take it.
    pub fn role<'a>(&'a self, content: &'a Content, role: PlayerFact) -> Option<SetupFact<'a>> {
        content.defs.fact_field(role)?;
        self.fact(content, role.name())
    }

    /// The side's version of the game, by the name its rules declare: the
    /// engine's version fact, where the rules take one and the side states
    /// it.
    pub fn version<'a>(&'a self, content: &'a Content) -> Option<&'a str> {
        self.role(content, PlayerFact::Version)?.name()
    }

    /// The forms the side has for its form list (the engine's form list
    /// fact: EXE6's Crosses), in order; none, a list not stated, or no such
    /// fact: empty.
    pub fn form_list(&self, content: &Content) -> Vec<FormHandle> {
        match self.role(content, PlayerFact::CrossList) {
            Some(fact) => (0..form_list_capacity(content)).filter_map(|k| fact.form(k)).collect(),
            None => Vec::new(),
        }
    }
}

/// The numbers a field of type `ty` holds, and the type's name: none for a
/// type that is no whole number's.
pub fn range(ty: &FieldType) -> Option<(i64, i64, &'static str)> {
    Some(match ty {
        FieldType::U8 => (0, u8::MAX as i64, "u8"),
        FieldType::U16 => (0, u16::MAX as i64, "u16"),
        FieldType::U32 => (0, u32::MAX as i64, "u32"),
        FieldType::I8 => (i8::MIN as i64, i8::MAX as i64, "i8"),
        FieldType::I16 => (i16::MIN as i64, i16::MAX as i64, "i16"),
        FieldType::I32 => (i32::MIN as i64, i32::MAX as i64, "i32"),
        FieldType::OptionalU8 => (0, u8::MAX as i64, "u8"),
        _ => return None,
    })
}

/// What is wrong with a fact the game's rules don't take.
pub fn no_field(content: &Content, field: &str) -> String {
    format!("no field {field:?} (a side of {} takes {})", content.game(), names_phrase(content))
}

/// The versions a side of the game states one of, by the names its rules
/// declare, in their order: the names of the engine's version fact
/// (`PlayerFact::Version`, an enum of the rules' setup: EXE6's cross
/// part's "gregar" and "falzar", the original's order). None: the rules
/// take no version. Tools go by the order: a version's place is its number
/// in a navi's stats (`crate::version_byte`).
pub fn versions(content: &Content) -> &[String] {
    let defs = &content.defs;
    let (Some(field), Some(rules)) = (defs.fact_field(PlayerFact::Version), defs.rules()) else { return &[] };
    match &defs.schema(rules.setup).field(field).ty {
        FieldType::Enum(names) => names,
        _ => &[],
    }
}

/// How many forms a side's form list has room for (the engine's form list
/// fact's elements: EXE6's Cross window's five), none when the game's
/// rules take none.
pub fn form_list_capacity(content: &Content) -> usize {
    match field(content, PlayerFact::CrossList.name()).filter(|_| content.defs.fact_field(PlayerFact::CrossList).is_some()) {
        Some(Field { ty: FieldType::Array(_, n), .. }) => *n as usize,
        _ => 0,
    }
}

/// What a round can't start without, of the game's facts: the enums and
/// the lists of definitions no default states (EXE6's `version`, one of two
/// names, and its `crosses`, a list that may be empty), in the facts'
/// order.
pub fn required(content: &Content) -> Vec<Field<'_>> {
    let defaults = Facts::defaults(content);
    fields(content).into_iter().filter(|f| defaults.get(content, f.name) == Some(Stated::Variant(None))).collect()
}

/// What a required fact may be stated as, for a message: an enum's
/// variants ("gregar or falzar").
pub fn may_be(field: &Field) -> String {
    match field.ty {
        FieldType::Enum(names) => names.join(" or "),
        other => other.to_string(),
    }
}

/// What is wrong with a side's facts in a match of `game`: facts that aren't the
/// game's; an enum or a list of definitions nothing states (a round starts
/// with none assumed: an empty list is stated as one); a
/// definition the content hasn't, or of another game than the match's; a
/// definition twice in a list, or an empty entry before one (a list is
/// filled from the front: a gap states what no save has); a form in the
/// side's form list that is none of its navi's lists'.
pub fn check(content: &Content, game: &str, side: &Side) -> Vec<String> {
    let mut out = Vec::new();
    if !side.facts.fit(content) {
        out.push(format!("the side's facts aren't {game}'s rules' (a side of {game} takes {})", names_phrase(content)));
        return out;
    }
    for f in fields(content) {
        let Some(value) = side.facts.get(content, f.name) else { continue };
        let of_game = |registry: Registry, h: u16| ids::key_of(content, registry, h).is_some_and(|key| ids::in_game(content, game, key));
        // (A definition a record names, at any depth: the game's.)
        if let Stated::List(items) = &value
            && items.iter().any(|v| matches!(v, Stated::Record(_)))
        {
            if let Some((r, _)) = named(&value).into_iter().find(|&(r, h)| !of_game(r, h)) {
                out.push(format!("{}: a {r} {game} hasn't", f.name));
            }
            continue;
        }
        match &value {
            Stated::Variant(None) => {
                out.push(format!("no {}: a side of {game} states its own ({}); none is assumed", f.name, may_be(&f)));
            }
            Stated::Def(registry, Some(h)) if !of_game(*registry, *h) => out.push(format!("{}: a {registry} {game} hasn't", f.name)),
            Stated::List(items) => {
                let defs: Vec<(Registry, u16)> = items.iter().filter_map(|v| if let Stated::Def(r, Some(h)) = v { Some((*r, *h)) } else { None }).collect();
                if defs.iter().any(|&(r, h)| !of_game(r, h)) {
                    out.push(format!("{}: a {} {game} hasn't", f.name, defs[0].0));
                    continue;
                }
                for (i, &(r, h)) in defs.iter().enumerate() {
                    if defs[..i].contains(&(r, h)) {
                        out.push(format!("{}: {} is there twice", f.name, ids::shown(content, r, h)));
                    }
                }
                let after_a_gap = items.iter().skip_while(|v| !matches!(v, Stated::Def(_, None))).find_map(|v| match v {
                    Stated::Def(r, Some(h)) => Some((*r, *h)),
                    _ => None,
                });
                if let Some((r, h)) = after_a_gap {
                    out.push(format!("{}: an empty entry before {} (a list is filled from the front)", f.name, ids::shown(content, r, h)));
                }
            }
            _ => {}
        }
    }
    // The form list: forms of the side's navi's own lists.
    let listed = side.facts.form_list(content);
    if !listed.is_empty() && !out.iter().any(|p| p.starts_with(PlayerFact::CrossList.name())) {
        let name = PlayerFact::CrossList.name();
        match crate::navi_forms(content, side.navi(content)) {
            None => out.push(format!("{name}: {} doesn't change form", crate::names::navi(content, side.navi(content)))),
            Some(own) => {
                for &f in &listed {
                    if !own.contains(&f) {
                        out.push(format!("{name}: {} is no form of {}'s lists", crate::names::form(content, f), crate::names::navi(content, side.navi(content))));
                    }
                }
            }
        }
    }
    out
}

/// The definitions a value names, at any depth, each by its registry and
/// handle.
fn named(value: &Stated) -> Vec<(Registry, u16)> {
    match value {
        Stated::Def(r, Some(h)) => vec![(*r, *h)],
        Stated::List(items) => items.iter().flat_map(named).collect(),
        Stated::Record(fields) => fields.iter().flat_map(|(_, v)| named(v)).collect(),
        _ => Vec::new(),
    }
}

/// A fact's value in words, for a description: a flag as yes or no, a
/// definition by its name, a list's entries (the empty ones left out).
pub fn shown(content: &Content, value: &Stated) -> String {
    match value {
        Stated::Flag(b) => if *b { "yes" } else { "no" }.to_string(),
        Stated::Number(n) => n.to_string(),
        Stated::Variant(v) => v.clone().unwrap_or_else(|| "not stated".to_string()),
        Stated::Def(r, Some(h)) => ids::shown(content, *r, *h),
        Stated::Def(_, None) => "none".to_string(),
        Stated::Optional(Some(n)) => n.to_string(),
        Stated::Optional(None) => "none".to_string(),
        Stated::List(items) => {
            let all: Vec<String> = items.iter().filter(|v| !matches!(v, Stated::Def(_, None))).map(|v| shown(content, v)).collect();
            if all.is_empty() { "none".to_string() } else { all.join(", ") }
        }
        Stated::Record(fields) => {
            let all: Vec<String> = fields.iter().map(|(name, v)| format!("{name} {}", shown(content, v))).collect();
            format!("({})", all.join(", "))
        }
        Stated::Code(c) => c.map_or_else(|| "none".to_string(), String::from),
        Stated::Other => "?".to_string(),
    }
}

impl Side {
    /// The side's navi (the engine's navi fact), where it states one.
    pub fn stated_navi(&self, content: &Content) -> Option<NaviHandle> {
        match self.facts.role(content, PlayerFact::Navi)?.value() {
            FieldValue::Ref(Some((_, h))) => Some(NaviHandle(h)),
            _ => None,
        }
    }

    /// The side's navi: every side states one (a new side its game's first,
    /// a match file its own: the checks refuse one without).
    pub fn navi(&self, content: &Content) -> NaviHandle {
        self.stated_navi(content).expect("a side states its navi (the engine's navi fact)")
    }

    /// State the side's navi. An error where the game's rules take no navi
    /// fact.
    pub fn set_navi(&mut self, content: &Content, navi: NaviHandle) -> Result<(), String> {
        self.set_fact(content, PlayerFact::Navi.name(), &[Fact::Value(Value::Def(Registry::Navi, navi.0))])
    }

    /// The side's folder (the engine's folder facts: its entries, its
    /// Regular and tag chips): 30 entries, those past the list's empty, as
    /// is an entry with no chip or no code.
    pub fn folder(&self, content: &Content) -> Folder {
        let mut folder = Folder::EMPTY;
        if let Some(fact) = self.facts.role(content, PlayerFact::Folder) {
            let (block, place) = (fact.block(), fact.place());
            let fields = match place.ty() {
                FieldType::List(elem, _) => match &**elem {
                    FieldType::Record(fields) => fields.index_of("chip").zip(fields.index_of("code")),
                    _ => None,
                },
                _ => None,
            };
            if let Some((chip, code)) = fields {
                let n = block.len_at(place).unwrap_or(0);
                for (k, slot) in folder.chips.iter_mut().enumerate().take(n) {
                    let Some(entry) = place.elem(k) else { continue };
                    let id = match block.get_at(entry.field_at(chip)) {
                        FieldValue::Ref(Some((_, h))) => Some(ChipHandle(h)),
                        _ => None,
                    };
                    let code = match block.get_at(entry.field_at(code)) {
                        FieldValue::Code(Some(c)) => ChipCode::from_letter(char::from(c)),
                        _ => None,
                    };
                    *slot = id.zip(code).map(|(id, code)| FolderChip::new(id, code));
                }
            }
        }
        folder.regular = match self.facts.role(content, PlayerFact::RegularChip).map(|f| f.value()) {
            Some(FieldValue::OptionalU8(r)) => r,
            _ => None,
        };
        folder.tags = match self.facts.get(content, PlayerFact::TagChips.name()).filter(|_| content.defs.fact_field(PlayerFact::TagChips).is_some()) {
            Some(Stated::List(items)) => match items[..] {
                [Stated::Number(a), Stated::Number(b)] => Some((a as u8, b as u8)),
                _ => None,
            },
            _ => None,
        };
        folder
    }

    /// State the side's folder: its entries to the last with a chip (an
    /// empty one `{}`), its Regular chip, its tag chips. An error where the
    /// game's rules take no folder, or the folder has tag chips and the
    /// rules take none (EXE5's).
    pub fn set_folder(&mut self, content: &Content, folder: &Folder) -> Result<(), String> {
        let last = folder.chips.iter().rposition(|c| c.is_some()).map_or(0, |i| i + 1);
        let entries: Vec<Fact> = folder.chips[..last]
            .iter()
            .map(|c| match c {
                Some(c) => Fact::Record(vec![
                    ("chip", Fact::Value(Value::Def(Registry::Chip, c.id.0))),
                    ("code", Fact::Value(Value::Code(c.code.letter() as u8))),
                ]),
                None => Fact::Record(Vec::new()),
            })
            .collect();
        let mut facts = self.facts.clone();
        facts.set(content, PlayerFact::Folder.name(), &entries)?;
        let regular = folder.regular.map_or(Value::Nil, |r| Value::Int(r as i64));
        if content.defs.fact_field(PlayerFact::RegularChip).is_some() {
            facts.set(content, PlayerFact::RegularChip.name(), &[Fact::Value(regular)])?;
        } else if folder.regular.is_some() {
            return Err(format!("a Regular chip, but {}'s rules take none", content.game()));
        }
        if content.defs.fact_field(PlayerFact::TagChips).is_some() {
            let tags: Vec<Fact> = folder.tags.map_or(Vec::new(), |(a, b)| vec![Fact::Value(Value::Int(a as i64)), Fact::Value(Value::Int(b as i64))]);
            facts.set(content, PlayerFact::TagChips.name(), &tags)?;
        } else if folder.tags.is_some() {
            return Err(format!("tag chips, but {}'s rules take none", content.game()));
        }
        self.facts = facts;
        Ok(())
    }

    /// The side's version of the game, where its rules take one and the
    /// side states it (`Facts::version`): its navi's version in the stats
    /// (NaviStats+0x20) goes by it.
    pub fn version<'a>(&'a self, content: &'a Content) -> Option<&'a str> {
        self.facts.version(content)
    }

    /// Write one of the side's facts (`Facts::set`).
    pub fn set_fact(&mut self, content: &Content, field: &str, values: &[Fact]) -> Result<(), String> {
        self.facts.set(content, field, values)
    }

    /// The side's navi's level (the engine's level fact: EXE6's navi
    /// code's, EXE5's team navi's), none where it states none or its rules
    /// take none.
    pub fn level(&self, content: &Content) -> Option<u8> {
        match self.facts.role(content, PlayerFact::Level)?.value() {
            FieldValue::OptionalU8(l) => l,
            _ => None,
        }
    }

    /// State the side's navi's level (none: none). An error where the
    /// game's rules take no level.
    pub fn set_level(&mut self, content: &Content, level: Option<u8>) -> Result<(), String> {
        let value = level.map_or(Value::Nil, |l| Value::Int(l as i64));
        self.set_fact(content, PlayerFact::Level.name(), &[Fact::Value(value)])
    }

    /// State the side's form list (the engine's form list fact: EXE6's
    /// Crosses) as its navi's own of its version: the forms the navi lists
    /// for the version the side states (EXE6's version's five), or none for
    /// a navi that doesn't change form. What a tool writes for a person
    /// once the side's version is chosen, and a random match for a side it
    /// picks no list for: the rules assume nothing, so a round doesn't
    /// start until the list is stated. False, and nothing written: the
    /// game's rules take no form list, or the navi changes form and the
    /// side states no version yet.
    pub fn state_own_forms(&mut self, content: &Content) -> bool {
        if content.defs.fact_field(PlayerFact::CrossList).is_none() {
            return false;
        }
        let own: Vec<FormHandle> = match (content.navi(self.navi(content)).forms.as_ref(), self.version(content)) {
            (None, _) => Vec::new(),
            (Some(forms), Some(version)) => forms.listed(version).to_vec(),
            (Some(_), None) => return false,
        };
        let list: Vec<Fact> = own.iter().map(|f| Fact::Value(Value::Def(Registry::Form, f.0))).collect();
        self.set_fact(content, PlayerFact::CrossList.name(), &list).is_ok()
    }

    /// Whether a side takes a version (the engine's version fact: EXE6's
    /// cross and rules/beast `version`, gregar or falzar). EXE5's rules
    /// don't: its two versions play alike, and a match of it states none.
    pub fn takes_version(content: &Content) -> bool {
        !versions(content).is_empty()
    }

    /// Whether the side's navi takes a level: its definition says what a
    /// level gives it (`levels`: EXE6's MegaMan and link navis, a navi
    /// code's; `story`: EXE5's team navis; EXE5's MegaMan has neither).
    pub fn takes_level(&self, content: &Content) -> bool {
        let navi = content.navi(self.navi(content));
        navi.levels.is_some() || navi.story.is_some()
    }

    /// Whether a side takes SP navi deletion times (the game's rules' fact
    /// the engine knows as `PlayerFact::SpTimes`: EXE6's and EXE5's, each
    /// their own SP navis).
    pub fn takes_sp_times(content: &Content) -> bool {
        content.defs.fact_field(PlayerFact::SpTimes).is_some()
    }
}

/// The role the engine knows fact `name` by (`PlayerFact`), if it knows it.
pub fn role_of(content: &Content, name: &str) -> Option<PlayerFact> {
    PlayerFact::ALL.iter().copied().find(|r| r.name() == name && content.defs.fact_field(*r).is_some())
}

/// The definitions a tool offers for `side`'s list fact `field` (a list of
/// definitions of a registry), by their handles, in the list's order:
///
/// - the engine's form list: the forms of the side's navi's own lists, of
///   every version (EXE6's ten Crosses; none for a navi that doesn't
///   change form);
/// - else what the rules' default lists (what a side that says nothing
///   has: EXE5's twelve souls), and after them any other the side's list
///   holds;
/// - a list whose default holds nothing: the game's definitions of the
///   registry.
///
/// None for a fact that is no list of definitions.
pub fn offered(content: &Content, game: &str, side: &Side, field: &Field) -> Option<Vec<u16>> {
    let FieldType::Array(elem, _) = field.ty else { return None };
    let FieldType::Ref(registry, _) = **elem else { return None };
    if role_of(content, field.name) == Some(PlayerFact::CrossList) {
        return Some(crate::navi_forms(content, side.navi(content)).unwrap_or_default().into_iter().map(|f| f.0).collect());
    }
    let mut out = Facts::defaults(content).get(content, field.name).map(|v| v.defs()).unwrap_or_default();
    if out.is_empty() {
        out = (0..=u16::MAX).map_while(|h| ids::key_of(content, registry, h).map(|key| (h, key))).filter(|(_, key)| ids::in_game(content, game, key)).map(|(h, _)| h).collect();
    }
    for h in side.facts.get(content, field.name).map(|v| v.defs()).unwrap_or_default() {
        if !out.contains(&h) {
            out.push(h);
        }
    }
    Some(out)
}
