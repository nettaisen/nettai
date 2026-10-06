//! How the editor lays out a side's setup beyond its own panes (the arena,
//! the navi, the folder, the stats, the auto battle data): from the game's
//! rules' setup schema alone, the editor's to decide. Every setup field
//! gets its type's view:
//!
//! - a value: a row on the navi pane (`crate::editor::facts::rows`: a flag,
//!   a number, an enum's variants);
//! - a few definitions (`form[5]`): a checklist pane (`crate::editor::facts`);
//! - any other list: a pane of its own ([`List`]: rows, added from what
//!   the game has in its library's order, removed and reordered up to its
//!   room; a record's fields as columns, each by its type's view).
//!
//! The richer views are the editor's own, each chosen by field and data
//! names and the data's shape, never by a game's name; where the data
//! doesn't fit one, the field has its generic view:
//!
//! - the SP navi deletion times (the engine's role `sp_times`, a list of
//!   records of a definition and `frames`): a row each, its frames a time
//!   as the games show one, `mm:ss.cc`;
//! - a list of entries whose data hold an `mb` (the patch cards): their
//!   total beside the count;
//! - the NaviCust (`crate::editor::navicust`): the board's size a pick,
//!   the programs placed on the board a grid.
//!
//! Whether a side's setup is right is the rules' `validate`'s: the editor
//! shows each problem beside the row, entry or piece it is tied to.

use crate::editor::facts::title;
use nettai_battle::content::{Content, PlayerFact};
use nettai_content_api::{Data, FieldType, Registry};
use nettai_match::facts;

/// A pane of a side's setup: its key (`--tab`'s name for it: its field's,
/// or `navicust`), its title, and its facts' views.
#[derive(Clone, Debug, PartialEq)]
pub struct Pane {
    pub key: String,
    pub title: String,
    pub fields: Vec<FieldPane>,
}

/// A fact's view in a pane: the setup field, its label, its view. Its
/// path (the field's name) is what an edit names.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldPane {
    pub field: String,
    pub label: String,
    pub view: View,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum View {
    Flag,
    /// A number (`time`: frames as `mm:ss.cc`).
    Number { time: bool },
    Pick(Pick),
    List(List),
    Grid(Grid),
}

/// One of a few: an enum's variants, a definition, or numbers by name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pick {
    /// Numbers by name (the board's sizes); none: the field's own choices.
    pub choices: Vec<(i64, String)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct List {
    /// The rows stay (each a definition's, named; its other fields edited).
    pub fixed: bool,
    /// A number field of its entries' data whose total it shows.
    pub total: Option<String>,
    /// A record's fields' views, by field; a field not named, its type's.
    pub columns: Vec<(String, View)>,
}

/// Pieces placed on a board: a list of records, its fields by what they
/// are, and the setup field holding the board's size.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub piece: String,
    pub x: String,
    pub y: String,
    pub rotation: String,
    pub color: String,
    pub toggles: Vec<String>,
    pub size: String,
}

/// What modules `names` of `game`'s rules (`rules/navicust/board`) returned
/// as the content loaded, as data (a function in them left out, nothing
/// called): read from a round of a random match of the game (one that
/// starts), none where none starts or the content has no such module.
pub fn modules(content: &std::sync::Arc<Content>, game: &str, names: &[&str]) -> Vec<Option<Data>> {
    let battle = nettai_match::pick::live(content, game, 0, None).ok().and_then(|m| nettai_match::check::start(content, &m).ok());
    names.iter().map(|n| battle.as_ref().and_then(|b| b.module_data(&format!("{game}:{n}"))).and_then(Result::ok)).collect()
}

/// The type of a list's element, else the type itself.
pub fn element(ty: &FieldType) -> &FieldType {
    match ty {
        FieldType::List(e, _) | FieldType::Array(e, _) => e,
        t => t,
    }
}

/// A record field's type, of a list of records.
pub fn record_field<'t>(ty: &'t FieldType, name: &str) -> Option<&'t FieldType> {
    let FieldType::Record(fields) = element(ty) else { return None };
    fields.index_of(name).map(|i| &fields.field(i).ty)
}

/// A list's room.
pub fn room(ty: &FieldType) -> usize {
    match ty {
        FieldType::List(_, n) => *n as usize,
        FieldType::Array(_, n) => *n as usize,
        _ => 0,
    }
}

/// The view a value of type `ty` has.
pub fn default_view(ty: &FieldType) -> View {
    match ty {
        FieldType::Bool => View::Flag,
        FieldType::Enum(_) | FieldType::Ref(..) => View::Pick(Pick::default()),
        FieldType::List(..) | FieldType::Array(..) => View::List(List::default()),
        _ => View::Number { time: false },
    }
}

/// The panes of a side's setup of the content's game, with the NaviCust's
/// where its data fit one (`navicust`: what the game's NaviCust module and
/// programs say, `crate::editor::navicust::Data`).
pub fn layout(
    content: &Content,
    navicust: Option<&crate::editor::navicust::Data>,
    auto_battle: Option<&crate::editor::auto_battle::Layout>,
) -> Vec<Pane> {
    let grid = navicust.and_then(|n| crate::editor::navicust::pane(content, n));
    let in_grid: Vec<String> = grid.iter().flat_map(|p| p.fields.iter().map(|f| f.field.clone())).collect();
    let mut grid = grid;
    let mut out = Vec::new();
    for f in facts::fields(content) {
        // (The core's own panes': the navi, the folder and its marks; the
        // auto battle pane's.)
        let role = facts::role_of(content, f.name);
        if matches!(role, Some(PlayerFact::Navi | PlayerFact::Folder | PlayerFact::RegularChip | PlayerFact::TagChips)) || crate::editor::auto_battle::owns(auto_battle, f.name) {
            continue;
        }
        if in_grid.iter().any(|n| n == f.name) {
            out.extend(grid.take());
            continue;
        }
        // (A value is a row on the navi pane; a few definitions a checklist.)
        if !matches!(f.ty, FieldType::List(..) | FieldType::Array(..)) || crate::editor::facts::is_list(&f) {
            continue;
        }
        // (A list of `{ <definition>, frames }`: times, as a person reads
        // them; the SP navi deletion times, by the field's name.)
        let list = if times_fit(f.ty) {
            List { fixed: true, total: None, columns: vec![("frames".into(), View::Number { time: true })] }
        } else {
            List { total: entries_total(content, f.ty, "mb"), ..List::default() }
        };
        let title = if f.name == "sp_times" { "SP navi deletion times".to_string() } else { title(f.name) };
        out.push(Pane {
            key: f.name.to_string(),
            title: title.clone(),
            fields: vec![FieldPane { field: f.name.to_string(), label: title, view: View::List(list), path: f.name.to_string() }],
        });
    }
    out
}

/// Whether a list is one of `{ <definition>, frames }`.
fn times_fit(ty: &FieldType) -> bool {
    let FieldType::Record(fields) = element(ty) else { return false };
    let frames = fields.index_of("frames").map(|i| &fields.field(i).ty);
    fields.fields().len() == 2
        && frames.is_some_and(|t| facts::range(t).is_some())
        && fields.fields().iter().any(|f| matches!(f.ty, FieldType::Ref(..)))
}

/// `field` where a list's entries are entries of a collection whose data
/// hold it as a number (each patch card's `mb`).
fn entries_total(content: &Content, ty: &FieldType, field: &str) -> Option<String> {
    let FieldType::Ref(Registry::Entry, Some(collection)) = element(ty) else { return None };
    let entries = content.defs.entries_of(collection);
    let numbered = |h: &nettai_content_api::EntryHandle| {
        let key = &content.defs.entry(*h).key;
        content.defs.definitions.get(Registry::Entry, key).is_some_and(|d| matches!(d.spec.field(field), Data::Int(_)))
    };
    (!entries.is_empty() && entries.iter().all(numbered)).then(|| field.to_string())
}
