//! How the creator lays out a build: from the game's rules' setup schema
//! alone, the creator's to decide (the rules declare no views). Every setup
//! field a player states gets its type's view:
//!
//! - the navi and each fact of one value (a flag, a number, an enum's
//!   variants): a row of the NAVI tab;
//! - the folder (the engine's folder facts: its entries, its Regular and
//!   tag chips): the FOLDER tab;
//! - a few definitions (`form[5]`, `form[16]`): a checklist tab, of what
//!   `nettai_match::facts::offered` offers the side;
//! - a list of definitions (`schema.list("patch_cards", 32)`): a tab of
//!   its entries, added in the library's order, ordered and taken out, with
//!   their data's `mb` in all where each has one, and a card's lines;
//! - a list of `{ <definition>, frames }`: a time each (`mm:ss.cc`);
//! - the pieces placed on a board, where the data fit the grid
//!   (`crate::builds::grid`), and the auto battle data where they fit its
//!   view (`crate::builds::auto`): tabs of their own;
//! - any other list: shown as it stands, not edited.
//!
//! The facts a build in the app never states are left out ([`at_defaults`]:
//! what a save brings to the navi's stats, the SP navi times, the auto
//! battle data's records, Chaos Unison): every build plays at the most its
//! game allows, and states only what a player chooses ([`as_built`]). A
//! fact stated as a preset ([`presets`]: EXE5's light or dark MegaMan) is a
//! row of its presets, and the facts they set besides are left out too.

use crate::builds::{auto, grid, presets};
use nettai_battle::content::{Content, PlayerFact};
use nettai_content_api::{Data, FieldType, Registry};
use nettai_match::Side;
use nettai_match::facts::{self, Field};
use std::collections::BTreeMap;

/// A tab of a build, by what it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tab {
    /// The navi, the facts of one value, the build's own actions.
    Navi,
    Folder,
    /// A few definitions, each checked or not.
    Checklist(String),
    /// The pieces on the board.
    Grid,
    /// A list of definitions; `total`: a number field of its entries' data
    /// whose total it shows.
    Entries { field: String, total: Option<String> },
    /// A list of `{ <definition>, frames }`.
    Times(String),
    /// The auto battle data's places.
    Places,
    /// Any other list.
    Other(String),
}

impl Tab {
    /// Its name, for the window: its field's, else its kind's.
    pub fn key(&self) -> &str {
        match self {
            Tab::Navi => "navi",
            Tab::Folder => "folder",
            Tab::Grid => grid::PIECES_FIELD,
            Tab::Places => auto::PLACES_FIELD,
            Tab::Checklist(f) | Tab::Entries { field: f, .. } | Tab::Times(f) | Tab::Other(f) => f,
        }
    }

    /// The setup fields it shows (their problems are its; a preset's row's,
    /// those of the facts its presets set too).
    pub fn fields(&self, content: &Content, rows: &[String]) -> Vec<String> {
        let role = |r: PlayerFact| content.defs.fact_name(r).map(str::to_string);
        match self {
            Tab::Navi => {
                let set = rows.iter().flat_map(|r| presets::others(content.game(), r));
                role(PlayerFact::Navi).into_iter().chain(rows.iter().cloned()).chain(set).collect()
            }
            Tab::Folder => [PlayerFact::Folder, PlayerFact::RegularChip, PlayerFact::TagChips].into_iter().filter_map(role).collect(),
            Tab::Grid => vec![grid::SIZE_FIELD.into(), grid::PIECES_FIELD.into()],
            Tab::Places => vec![auto::PLACES_FIELD.into()],
            Tab::Checklist(f) | Tab::Entries { field: f, .. } | Tab::Times(f) | Tab::Other(f) => vec![f.clone()],
        }
    }
}

/// What modules `names` of `game`'s rules returned as the content loaded,
/// as data (a function in them left out, nothing called): read from a
/// round of a random match of the game, none where none starts or the
/// content has no such module.
pub fn modules(content: &std::sync::Arc<Content>, game: &str, names: &[&str]) -> Vec<Option<Data>> {
    let battle = nettai_match::pick::live(content, game, 0, None).ok().and_then(|m| nettai_match::check::start(content, &m).ok());
    names.iter().map(|n| battle.as_ref().and_then(|b| b.module_data(&format!("{game}:{n}"))).and_then(Result::ok)).collect()
}

/// The app's own data on its builds, by game (`crates/nettai/builds.toml`):
/// the facts a build never states, and those it states as presets.
#[derive(serde::Deserialize)]
pub struct Config {
    at_defaults: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub presets: BTreeMap<String, BTreeMap<String, Vec<presets::Preset>>>,
}

pub fn config() -> &'static Config {
    static CONFIG: std::sync::OnceLock<Config> = std::sync::OnceLock::new();
    CONFIG.get_or_init(|| toml::from_str(include_str!("../../builds.toml")).expect("builds.toml reads"))
}

/// Whether fact `name` is one a build in the app never states, always the
/// rules' default: the engine's level and base HP, and the facts the app
/// lists for the game (`builds.toml`: the rest of what a save brings to the
/// navi's stats, the SP navi times, the auto battle data's records), where
/// no preset sets it ([`presets`]: EXE5's base HP, with its light or dark
/// MegaMan).
pub fn at_defaults(content: &Content, game: &str, name: &str) -> bool {
    let listed = config().at_defaults.get(game).is_some_and(|l| l.iter().any(|f| f == name));
    (matches!(facts::role_of(content, name), Some(PlayerFact::Level | PlayerFact::BaseHp)) || listed) && !presets::set_by_another(game, name)
}

/// Whether a player states the fact in the creator (one stated as a preset
/// among them, those its presets set besides not).
pub fn stated_by_player(content: &Content, game: &str, name: &str) -> bool {
    !at_defaults(content, game, name) && !presets::set_by_another(game, name)
}

/// What setting a side to what a build in the app is changed.
#[derive(Clone, Debug, Default)]
pub struct Built {
    /// The facts set to the rules' defaults, by name (a place of the auto
    /// battle data that pointed at a record emptied, a level stated).
    pub reset: Vec<String>,
    /// The facts set to a preset's.
    pub taken: Vec<presets::Taken>,
    /// Whether any fact changed.
    pub changed: bool,
}

/// Set the side to what a build in the app is: its facts at defaults the
/// rules' defaults, a navi that must have a level at its last, where the
/// auto battle data's records are at defaults no place pointing at one, and
/// each fact stated as a preset the preset it is on.
pub fn as_built(content: &Content, game: &str, side: &mut Side) -> Built {
    as_built_with(content, game, side, None)
}

/// [`as_built`], `hint` (a save's version or team) naming the preset where
/// one has that name.
pub fn as_built_with(content: &Content, game: &str, side: &mut Side, hint: Option<&str>) -> Built {
    let before = side.clone();
    for f in facts::fields(content) {
        if at_defaults(content, game, f.name) {
            side.facts.reset(content, f.name);
        }
    }
    if at_defaults(content, game, auto::RECORDS_FIELD) {
        auto::drop_patterns(content, side);
    }
    let level = content.defs.fact_name(PlayerFact::Level).filter(|l| at_defaults(content, game, l));
    if let (Some(_), Some(navi)) = (level, side.stated_navi(content))
        && nettai_match::level_required(content, navi)
        && side.level(content).is_none()
    {
        let _ = side.set_level(content, nettai_match::play_level(content, navi).or(Some(0)));
    }
    let taken = presets::settle(content, game, side, hint);
    let changed: Vec<String> = facts::fields(content)
        .into_iter()
        .filter(|f| before.facts.get(content, f.name) != side.facts.get(content, f.name))
        .map(|f| f.name.to_string())
        .collect();
    let preset = |name: &str| presets::of(game, name).is_some() || presets::set_by_another(game, name);
    Built { changed: !changed.is_empty(), reset: changed.into_iter().filter(|n| !preset(n)).collect(), taken }
}

/// The type of a list's element, else the type itself.
pub fn element(ty: &FieldType) -> &FieldType {
    match ty {
        FieldType::List(e, _) | FieldType::Array(e, _) => e,
        t => t,
    }
}

/// A list's room.
pub fn room(ty: &FieldType) -> usize {
    match ty {
        FieldType::List(_, n) => *n as usize,
        FieldType::Array(_, n) => *n as usize,
        _ => 0,
    }
}

/// Whether `field` is a few definitions: a checklist.
pub fn is_checklist(field: &Field) -> bool {
    matches!(field.ty, FieldType::Array(elem, _) if matches!(**elem, FieldType::Ref(..)))
}

/// Whether a value of type `ty` is a row of its own: one value, or a few
/// flags.
fn is_row(ty: &FieldType) -> bool {
    match ty {
        FieldType::List(..) | FieldType::Record(_) | FieldType::Object | FieldType::Vec3 | FieldType::Asset(_) => false,
        FieldType::Array(elem, _) => **elem == FieldType::Bool,
        _ => true,
    }
}

/// The record field that names a list's definitions, where a list is one
/// of `{ <definition>, frames }`.
pub fn times_def(ty: &FieldType) -> Option<String> {
    let FieldType::Record(fields) = element(ty) else { return None };
    let frames = fields.index_of("frames").map(|i| &fields.field(i).ty);
    if fields.fields().len() != 2 || !frames.is_some_and(|t| facts::range(t).is_some()) {
        return None;
    }
    fields.fields().iter().find(|f| matches!(f.ty, FieldType::Ref(..))).map(|f| f.name.clone())
}

/// `field` where a list's entries are entries of a collection whose data
/// all hold it as a number (each patch card's `mb`).
fn entries_total(content: &Content, ty: &FieldType, field: &str) -> Option<String> {
    let FieldType::Ref(Registry::Entry, Some(collection)) = element(ty) else { return None };
    let entries = content.defs.entries_of(collection);
    let numbered = |h: &nettai_content_api::EntryHandle| {
        let key = &content.defs.entry(*h).key;
        content.defs.definitions.get(Registry::Entry, key).is_some_and(|d| matches!(d.spec.field(field), Data::Int(_)))
    };
    (!entries.is_empty() && entries.iter().all(numbered)).then(|| field.to_string())
}

/// What a build of the content's game is laid out as: its tabs, and the
/// facts on the NAVI tab's rows (in the setup's order).
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub tabs: Vec<Tab>,
    pub rows: Vec<String>,
}

/// The layout of `side`, a build of the content's game `game`: the grid's
/// and the auto battle data's tabs where their data fit (`grid`, `auto`).
pub fn layout(content: &Content, game: &str, side: &Side, grid: Option<&grid::GridData>, auto: Option<&auto::Layout>) -> Layout {
    let mut tabs = vec![Tab::Navi];
    if content.defs.fact_field(PlayerFact::Folder).is_some() {
        tabs.push(Tab::Folder);
    }
    let mut rows = Vec::new();
    let mut lists = Vec::new();
    let mut grid_tab = false;
    let changes_form = side.stated_navi(content).is_some_and(|n| content.navi(n).forms.is_some());
    for f in facts::fields(content) {
        let role = facts::role_of(content, f.name);
        if !stated_by_player(content, game, f.name) || matches!(role, Some(PlayerFact::Navi)) {
            continue;
        }
        if matches!(role, Some(PlayerFact::Folder | PlayerFact::RegularChip | PlayerFact::TagChips)) {
            continue;
        }
        // (The pieces on the board are the navi's that changes form alone:
        // another's side has none.)
        if grid.is_some() && (f.name == grid::SIZE_FIELD || f.name == grid::PIECES_FIELD) {
            if !grid_tab && changes_form {
                lists.push(Tab::Grid);
            }
            grid_tab = true;
            continue;
        }
        if auto::owns(auto, f.name) {
            if f.name == auto::PLACES_FIELD {
                lists.push(Tab::Places);
            }
            continue;
        }
        // (A fact stated as one of its presets is a row of them, whatever
        // its type: EXE5's souls, its team's.)
        if presets::of(game, f.name).is_some() {
            rows.push(f.name.to_string());
            continue;
        }
        if is_checklist(&f) {
            if facts::offered(content, game, side, &f).is_some_and(|o| !o.is_empty()) {
                tabs.push(Tab::Checklist(f.name.to_string()));
            }
            continue;
        }
        if is_row(f.ty) {
            rows.push(f.name.to_string());
            continue;
        }
        let tab = match f.ty {
            FieldType::List(elem, _) if matches!(**elem, FieldType::Ref(..)) => Tab::Entries { field: f.name.to_string(), total: entries_total(content, f.ty, "mb") },
            ty if times_def(ty).is_some() => Tab::Times(f.name.to_string()),
            _ => Tab::Other(f.name.to_string()),
        };
        lists.push(tab);
    }
    // (The checklists by the folder; then the grid, the lists, the times.)
    lists.sort_by_key(|t| match t {
        Tab::Grid => 0,
        Tab::Entries { .. } => 1,
        Tab::Places => 2,
        Tab::Times(_) => 3,
        _ => 4,
    });
    tabs.extend(lists);
    Layout { tabs, rows }
}

/// A fact's name as a title: `chaos_unison` as "Chaos unison".
pub fn title(name: &str) -> String {
    let spaced = name.replace('_', " ");
    let mut letters = spaced.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => spaced,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each game's build is laid out from its setup alone: EXE6's the navi
    /// (its version, Beast Out, bug frags), the folder, the Crosses, the
    /// grid and the patch cards with their MB; EXE5's the same with its
    /// souls and its auto battle data's places. What a save brings to the
    /// stats, the SP times and the auto battle records are none of a
    /// player's.
    #[test]
    fn each_games_build_is_laid_out() {
        for (content, game) in [(nettai_match::testing::exe6_content(), "exe6"), (nettai_match::testing::exe5_content(), "exe5")] {
            let modules = modules(&content, game, &[grid::MODULE, auto::MODULE]);
            let g = grid::read(&content, modules[0].as_ref());
            let a = auto::Layout::read(&content, modules[1].as_ref());
            let m = nettai_match::Match::empty(&content, game).unwrap();
            let l = layout(&content, game, &m.sides[0], g.as_ref(), a.as_ref());
            let keys: Vec<&str> = l.tabs.iter().map(Tab::key).collect();
            let rows: Vec<&str> = l.rows.iter().map(String::as_str).collect();
            match game {
                // (The Crosses and the souls are their version's and team's
                // presets': no tab of their own.)
                "exe6" => {
                    assert_eq!(keys, ["navi", "folder", "navicust_programs", "patch_cards"]);
                    assert_eq!(rows, ["beast_out", "bug_frags", "version"]);
                }
                _ => {
                    assert_eq!(keys, ["navi", "folder", "navicust_programs", "patch_cards", "auto_battle_places"]);
                    assert_eq!(rows, ["karma", "souls"]);
                }
            }
            assert!(matches!(&l.tabs.iter().find(|t| t.key() == "patch_cards"), Some(Tab::Entries { total: Some(t), .. }) if t == "mb"));
        }
    }

    /// A build is set to what every build in the app is: what a save brings
    /// to the stats and the SP times at the rules' defaults (said), a link
    /// navi at its last level, the rest as it was.
    #[test]
    fn a_build_is_as_built() {
        use nettai_battle::rules::Fact;
        use nettai_content_api::Value;
        let six = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&six, "exe6", 9, None).unwrap();
        let mut side = m.sides[0].clone();
        side.set_fact(&six, "hp", &[Fact::Value(Value::Int(600))]).unwrap();
        side.set_fact(&six, "sun", &[Fact::Value(Value::Bool(false))]).unwrap();
        let before = side.clone();
        let reset = as_built(&six, "exe6", &mut side).reset;
        assert!(reset.contains(&"hp".to_string()) && reset.contains(&"sun".to_string()), "{reset:?}");
        assert!(side.facts.is_default(&six, "hp") && side.facts.is_default(&six, "sun"));
        assert_eq!(side.folder(&six), before.folder(&six));
        assert!(!as_built(&six, "exe6", &mut side).changed, "once is enough");
        let protoman = nettai_match::ids::navi(&six, "exe6", "protoman").unwrap();
        crate::builds::edit::switch_navi(&six, &mut side, protoman);
        side.set_level(&six, None).unwrap();
        as_built(&six, "exe6", &mut side);
        let last = six.navi(protoman).levels.as_ref().unwrap().by_level.len() as u8 - 1;
        assert_eq!(side.level(&six), Some(last), "a link navi at its last level");
    }
}
