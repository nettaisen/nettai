//! What a side brings that its game's rules take: its facts.
//!
//! A fact is a field of the setup of a system of the game's ruleset, as
//! the content declares it (`setup = { version = { "gregar", "falzar" },
//! cross_list = "form[5]" }`, `setup = { karma = "u16" }`): its name, its
//! type, its default (`setup_defaults`; zero without one, and an enum
//! unstated). A side holds its facts as those systems' setup blocks
//! ([`Facts`]), which the round's setup hands the engine as they are. A
//! field several systems declare is one fact, written into each.
//!
//! This crate names no game's fact. Which facts there are, what a side
//! that says nothing has, and which a round can't start without (an enum
//! nothing states) are the content's; a match file states a fact under its
//! field's name, and the editor shows one by its type. Three the engine
//! itself knows by role (`PlayerFact`: the version, Beast Out, the form
//! list), and where this crate needs one of those (the version's place in a
//! navi's stats, the forms a navi's list may hold) it asks by the role.
//!
//! The games' own (EXE6's `version`, `crosses`, `cross_list`, `beast_out`,
//! `bug_frags`; EXE5's `karma`, `souls`, `soul_unison`, `chaos_unison`) are
//! documented where they are declared: content/exe6/rules and
//! content/exe5/rules.

use crate::{Arena, Side, ids};
use nettai_battle::content::{Content, PlayerFact};
use nettai_battle::rules::{self, Fact, SetupFact};
use nettai_content_api::{ChipHandle, ContentState, FieldType, FieldValue, FormHandle, Registry, Value};

/// A side's facts: a setup block for each system of its game's ruleset, in
/// the ruleset's order (none on a content without a ruleset).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Facts(Vec<ContentState>);

/// A fact a side of the content's game takes: a setup field's name and type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field<'c> {
    pub name: &'c str,
    pub ty: &'c FieldType,
}

/// The facts a side of the content's game takes: each setup field of its
/// ruleset's systems once, in the systems' order and each one's fields' (a
/// setup's fields are in their names' order: a schema sorts them).
pub fn fields(content: &Content) -> Vec<Field<'_>> {
    let mut out: Vec<Field> = Vec::new();
    for &h in crate::systems(content) {
        for f in content.defs.schema(content.defs.system(h).setup).fields() {
            if !out.iter().any(|o| o.name == f.name) {
                out.push(Field { name: &f.name, ty: &f.ty });
            }
        }
    }
    out
}

/// The fact named `name`, if the game's rules take it.
pub fn field<'c>(content: &'c Content, name: &str) -> Option<Field<'c>> {
    fields(content).into_iter().find(|f| f.name == name)
}

/// Whether a system of the game's rules declares setup field `field` (a
/// side takes that fact).
pub fn takes(content: &Content, field: &str) -> bool {
    self::field(content, field).is_some()
}

/// The facts' names in a phrase, for a message: "version, crosses,
/// cross_list".
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

fn stated_of(ty: &FieldType, v: FieldValue) -> Stated {
    match (ty, v) {
        (_, FieldValue::Bool(b)) => Stated::Flag(b),
        (FieldType::Enum(names), FieldValue::Enum(i)) => Stated::Variant(names.get(i as usize).cloned()),
        (FieldType::Ref(registry, _), FieldValue::Ref(r)) => Stated::Def(*registry, r.map(|(_, h)| h)),
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
    /// What a side that says nothing has: each system's defaults.
    pub fn defaults(content: &Content) -> Facts {
        Facts(rules::default_blocks(content))
    }

    /// The setup blocks, as a round's setup carries them.
    pub fn blocks(&self) -> &[ContentState] {
        &self.0
    }

    /// Facts from a player's setup blocks (a save's, a recording's).
    pub fn of_blocks(blocks: Vec<ContentState>) -> Facts {
        Facts(blocks)
    }

    /// Whether these are the content's game's: a block for each system of
    /// its ruleset, of that system's setup.
    pub fn fit(&self, content: &Content) -> bool {
        let systems = crate::systems(content);
        self.0.len() == systems.len() && self.0.iter().zip(systems).all(|(b, &h)| b.id() == content.defs.system(h).setup)
    }

    /// Write fact `field`: one value for a field, an element each for an
    /// array (the rest zero), an enum's by its name (`Fact::Name`). An
    /// error names what is wrong: no system of the game's rules takes the
    /// field, a value isn't the field's, or a number is past its type (the
    /// engine's own store wraps one to the width). Nothing is written then.
    pub fn set(&mut self, content: &Content, field: &str, values: &[Fact]) -> Result<(), String> {
        let Some(declared) = self::field(content, field) else { return Err(no_field(content, field)) };
        let of = match declared.ty {
            FieldType::Array(elem, _) => elem,
            ty => ty,
        };
        for v in values {
            if let (Fact::Value(Value::Int(n)), Some((least, most, what))) = (v, range(of))
                && !(least..=most).contains(n)
            {
                return Err(format!("{n} is past a {what} ({least} to {most})"));
            }
        }
        let mut blocks = self.0.clone();
        match rules::set_fact(&mut blocks, content, field, values)? {
            0 => Err(no_field(content, field)),
            _ => {
                self.0 = blocks;
                Ok(())
            }
        }
    }

    /// Fact `field` back at what a side that says nothing has.
    pub fn reset(&mut self, content: &Content, field: &str) {
        let defaults = rules::default_blocks(content);
        for ((block, default), &h) in self.0.iter_mut().zip(&defaults).zip(crate::systems(content)) {
            let schema = content.defs.schema(content.defs.system(h).setup);
            let Some(i) = schema.index_of(field) else { continue };
            match &schema.field(i).ty {
                FieldType::Array(_, n) => {
                    for k in 0..*n as usize {
                        if let Some(v) = default.get_elem(schema, i, k) {
                            let _ = block.set_elem(schema, i, k, v.load());
                        }
                    }
                }
                _ if !default.stated(schema, i) => block.unstate(schema, i),
                _ => {
                    let _ = block.set(schema, i, default.get(schema, i).load());
                }
            }
        }
    }

    /// Fact `field`, as the engine reads one ([`SetupFact`]).
    pub fn fact<'a>(&'a self, content: &'a Content, field: &str) -> Option<SetupFact<'a>> {
        rules::fact_in(&self.0, content, field)
    }

    /// Fact `field`'s value; none: the game's rules take no such fact.
    pub fn get(&self, content: &Content, field: &str) -> Option<Stated> {
        let fact = self.fact(content, field)?;
        Some(match fact.ty() {
            FieldType::Array(elem, n) => Stated::List((0..*n as usize).filter_map(|k| fact.elem(k)).map(|v| stated_of(elem, v)).collect()),
            ty if !fact.stated() => match ty {
                FieldType::Enum(_) => Stated::Variant(None),
                _ => Stated::Other,
            },
            ty => stated_of(ty, fact.value()),
        })
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

    /// The forms the side's form list offers in place of its version's own
    /// (the engine's form list fact), in order; none listed, or no such
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
        _ => return None,
    })
}

/// What is wrong with a fact no system of the game's rules takes.
pub fn no_field(content: &Content, field: &str) -> String {
    format!("no field {field:?} (a side of {} takes {})", content.game(), names_phrase(content))
}

/// The versions a side of the game states one of, by the names its rules
/// declare, in their order: the names of the engine's version fact
/// (`PlayerFact::Version`, an enum of a system's setup: EXE6's cross
/// system's "gregar" and "falzar", the original's order). None: the rules
/// take no version. Tools go by the order: a version's place is its number
/// in a navi's stats (`crate::version_byte`).
pub fn versions(content: &Content) -> &[String] {
    let defs = &content.defs;
    let Some((slot, field)) = defs.fact_field(PlayerFact::Version) else { return &[] };
    match &defs.schema(defs.system(defs.ruleset_systems()[slot]).setup).field(field).ty {
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

/// What a round can't start without, of the game's facts: the enums no
/// default states (EXE6's `version`), with their variants.
pub fn required(content: &Content) -> Vec<(&str, &[String])> {
    let defaults = Facts::defaults(content);
    fields(content)
        .into_iter()
        .filter_map(|f| match f.ty {
            FieldType::Enum(names) if defaults.get(content, f.name) == Some(Stated::Variant(None)) => Some((f.name, names.as_slice())),
            _ => None,
        })
        .collect()
}

/// What is wrong with a side's facts on `arena`: facts that aren't the
/// game's; an enum nothing states (a round starts with none assumed); a
/// definition the content hasn't, or of another game than the match's; a
/// definition twice in a list; a form in the side's form list that is none
/// of its navi's lists'.
pub fn check(content: &Content, arena: &Arena, side: &Side) -> Vec<String> {
    let mut out = Vec::new();
    let game = arena.game.as_str();
    if !side.facts.fit(content) {
        out.push(format!("the side's facts aren't {game}'s rules' (a side of {game} takes {})", names_phrase(content)));
        return out;
    }
    for f in fields(content) {
        let Some(value) = side.facts.get(content, f.name) else { continue };
        let of_game = |registry: Registry, h: u16| ids::key_of(content, registry, h).is_some_and(|key| ids::in_game(content, game, key));
        match &value {
            Stated::Variant(None) => {
                let FieldType::Enum(names) = f.ty else { continue };
                out.push(format!("no {}: a side of {game} states its own ({}); none is assumed", f.name, names.join(" or ")));
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
            }
            _ => {}
        }
    }
    // The form list: forms of the side's navi's own lists.
    let listed = side.facts.form_list(content);
    if !listed.is_empty() && !out.iter().any(|p| p.starts_with(PlayerFact::CrossList.name())) {
        let name = PlayerFact::CrossList.name();
        match crate::navi_forms(content, side.navi) {
            None => out.push(format!("{name}: {} doesn't change form", crate::names::navi(content, side.navi))),
            Some(own) => {
                for &f in &listed {
                    if !own.contains(&f) {
                        out.push(format!("{name}: {} is no form of {}'s lists", crate::names::form(content, f), crate::names::navi(content, side.navi)));
                    }
                }
            }
        }
    }
    out
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
        Stated::List(items) => {
            let all: Vec<String> = items.iter().filter(|v| !matches!(v, Stated::Def(_, None))).map(|v| shown(content, v)).collect();
            if all.is_empty() { "none".to_string() } else { all.join(", ") }
        }
        Stated::Other => "?".to_string(),
    }
}

impl Side {
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

    /// Whether a side takes a version (the engine's version fact: EXE6's
    /// cross and beast systems' `version`, gregar or falzar). EXE5's rules
    /// don't: its two versions play alike, and a match of it states none.
    pub fn takes_version(content: &Content) -> bool {
        !versions(content).is_empty()
    }

    /// Whether the side's navi takes a level: its definition says what a
    /// level gives it (`levels`: EXE6's MegaMan and link navis, a navi
    /// code's; `story`: EXE5's team navis; EXE5's MegaMan has neither).
    pub fn takes_level(&self, content: &Content) -> bool {
        let navi = content.navi(self.navi);
        navi.levels.is_some() || navi.story.is_some()
    }

    /// Whether a side takes SP navi deletion times (the game's rules'
    /// `sp_slots`: EXE6's and EXE5's, each their own SP navis).
    pub fn takes_sp_times(content: &Content) -> bool {
        !crate::sp_slots(content).is_empty()
    }
}

/// The SP navi chip of the arena's game whose damage reads slot `slot` of
/// its rules, if the game has it: the slot's name in a tool.
pub fn sp_chip(content: &Content, arena: &Arena, slot: usize) -> Option<ChipHandle> {
    (0..content.defs.chips.len() as u16)
        .map(ChipHandle)
        .find(|&h| content.chip_links(h).sp_slot == Some(slot as u8) && ids::in_game(content, &arena.game, &content.defs.chip(h).key))
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
        return Some(crate::navi_forms(content, side.navi).unwrap_or_default().into_iter().map(|f| f.0).collect());
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
