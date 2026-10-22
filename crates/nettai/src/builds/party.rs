//! A build's party (EXE5's party battle, EXE5 DS's: docs/design/exe5ds-map.md §4, §11), as any screen reads it: its
//! shape from the setup's schema (a list of records, each naming a navi and listing a collection's entries, the
//! programs), its partners read and written, a program's color, and the overview's rows of the partners. The Party tab
//! edits it (`crate::screens::builds::party`); the build overview shows it ([`sheet_rows`]).

use nettai_battle::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{EntryHandle, FieldType, NaviHandle, Registry, Value};
use nettai_match::build::{Budget, Overview};
use nettai_match::facts::Stated;

/// The setup field of the party.
pub const FIELD: &str = "party";

/// The field of a program's definition that names its color.
const COLOR: &str = "color";

/// The party's shape, as the setup's schema says it.
#[derive(Clone)]
pub struct Shape {
    /// How many partners the list holds.
    pub room: usize,
    /// The record's field naming the partner's navi.
    pub navi: String,
    /// The record's field listing its programs, their collection, and how many it holds.
    pub programs: String,
    pub collection: String,
    pub program_room: usize,
}

impl Shape {
    pub fn of(content: &Content) -> Option<Shape> {
        let field = nettai_match::facts::field(content, FIELD)?;
        let FieldType::List(elem, room) = field.ty else { return None };
        let FieldType::Record(fields) = &**elem else { return None };
        let navi = fields.fields().iter().find(|f| matches!(f.ty, FieldType::Ref(Registry::Navi, _)))?.name.clone();
        let (programs, collection, program_room) = fields.fields().iter().find_map(|f| match &f.ty {
            FieldType::List(e, n) => match &**e {
                FieldType::Ref(Registry::Entry, Some(c)) => Some((f.name.clone(), c.clone(), *n as usize)),
                _ => None,
            },
            _ => None,
        })?;
        Some(Shape { room: *room as usize, navi, programs, collection, program_room })
    }
}

/// A partner as the build holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Partner {
    pub navi: Option<NaviHandle>,
    pub programs: Vec<EntryHandle>,
}

/// Program `h`'s color as a NaviCust program's is drawn (its definition's `color`, a name of the NaviCust's colors:
/// `crate::builds::grid::PALETTE`).
pub fn tone(content: &Content, h: EntryHandle) -> slint::Color {
    let key = &content.defs.entry(h).key;
    let name = content.defs.definitions.get(Registry::Entry, key).and_then(|d| d.spec.field(COLOR).str().map(str::to_string));
    crate::builds::grid::tone(name.as_deref())
}

/// The build's partners, in order.
pub fn partners(content: &Content, side: &nettai_match::Side, shape: &Shape) -> Vec<Partner> {
    let Some(Stated::List(items)) = side.facts.get(content, FIELD) else { return Vec::new() };
    items
        .iter()
        .map(|item| {
            let Stated::Record(fields) = item else { return Partner::default() };
            let get = |name: &str| fields.iter().find(|(n, _)| n == name).map(|(_, v)| v);
            let navi = match get(&shape.navi) {
                Some(Stated::Def(Registry::Navi, Some(h))) => Some(NaviHandle(*h)),
                _ => None,
            };
            let programs = match get(&shape.programs) {
                Some(Stated::List(xs)) => xs.iter().filter_map(|x| if let Stated::Def(Registry::Entry, Some(h)) = x { Some(EntryHandle(*h)) } else { None }).collect(),
                _ => Vec::new(),
            };
            Partner { navi, programs }
        })
        .collect()
}

/// Write the build's partners: whether the facts took them.
pub fn write(content: &Content, side: &mut nettai_match::Side, shape: &Shape, partners: &[Partner]) -> bool {
    let records: Vec<Fact> = partners
        .iter()
        .map(|p| {
            let navi = p.navi.map_or(Fact::Value(Value::Nil), |n| Fact::Value(Value::Def(Registry::Navi, n.0)));
            let programs = Fact::List(p.programs.iter().map(|h| Fact::Value(Value::Def(Registry::Entry, h.0))).collect());
            Fact::Record(vec![(shape.navi.as_str(), navi), (shape.programs.as_str(), programs)])
        })
        .collect();
    match side.set_fact(content, FIELD, &records) {
        Ok(()) => true,
        Err(why) => {
            crate::errors::log("changing a build's party", &why);
            false
        }
    }
}


/// Each partner's gauge, by its entry: the overview's budget of it.
pub fn budgets(content: &Content, side: &nettai_match::Side, overview: Option<&Overview>) -> Vec<Option<Budget>> {
    let Some(o) = overview else { return Vec::new() };
    let n = match side.facts.get(content, FIELD) {
        Some(Stated::List(xs)) => xs.len(),
        _ => 0,
    };
    (0..n).map(|i| o.budget_of(FIELD, i).cloned()).collect()
}

/// The blocks partner `p` fills of its gauge, each in its program's color, the programs in their order.
pub fn tones(content: &Content, p: &Partner, budget: Option<&Budget>) -> Vec<slint::Color> {
    p.programs.iter().flat_map(|&h| std::iter::repeat_n(tone(content, h), budget.map_or(0, |b| b.cost_of(content, h)) as usize)).collect()
}

/// The build overview's rows of `side`'s partners (none: no party, or a game whose setup takes none): each its face
/// and name, and its PartyCust programs by name (the user: "there's no need to show the slot gauge or hp for the party
/// navi").
pub fn sheet_rows(kit: &crate::builds::Kit, names: &crate::games::Names, side: &nettai_match::Side) -> Vec<nettai_ui::SheetPartner> {
    let content = kit.content();
    let Some(shape) = Shape::of(content) else { return Vec::new() };
    partners(content, side, &shape)
        .iter()
        .filter_map(|p| {
            let navi = p.navi?;
            Some(nettai_ui::SheetPartner {
                face: kit.pictures.navi(navi),
                name: names.navi_long(navi).into(),
                version: kit.version_tag_of(names, Registry::Navi, &nettai_match::ruleset::Item::of(navi.0)),
                programs: crate::builds::overview::items(p.programs.iter().map(|&h| names.entry(h))),
            })
        })
        .collect()
}
