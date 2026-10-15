//! A ruleset: which builds may play a match (docs/design/app-ui.md §2.59;
//! docs/design/rules-in-luau.md, "Rulesets"; docs/design/versions.md §2.2,
//! §2.3). Its name, whether each build must be one version's (Versions: one
//! per build, or mixed), and for each of its game's categories the items
//! it doesn't allow: what may be played, and nothing else (closed builds is
//! the lobby's setting beside it: the user, "i think closed setup should
//! not be part of the ruleset actually"). It says nothing of how a round
//! plays, so the simulation never reads it.
//!
//! - **The game's categories and presets** ([`Rulesets`], read from the
//!   game's own data, its rules/rulesets module): a category is a fact of
//!   the rules' setup that names definitions, by the fact's name; its
//!   registry, the field of a record that names one and its items are the
//!   fact's type's ([`Category`]). An item is a definition, or one variant
//!   of one where the fact's record chooses one (its `variant`: EXE5's and
//!   EXE6's Bass-Cross MegaMan card, each version's, as a NaviCust
//!   program's color is chosen; [`Item`]). The presets are the game's
//!   rulesets ready-made, in order, the first a player's until they pick
//!   another.
//! - **The game's groups** ([`Group`]): each version of the game as what a
//!   save of it can own that the other's can't (docs/design/versions.md
//!   §2.2). A group is one of the game's builds data's (`build::Group`: its
//!   id, its color, its name in the builds' strings), and its members are
//!   the chips and forms whose own `version` names it (EXE6's Giga chips,
//!   Crosses and Beast forms, the Cybeasts among them; EXE4's and EXE5's
//!   souls and Giga chips) and the variants of its name (Bass-Cross
//!   MegaMan's). A category's items in a group are its quick choice, and a
//!   ruleset of one version per build holds every build to one group.
//! - **A ruleset** ([`Ruleset`]): what it doesn't name is allowed, so a
//!   definition the game gains later is allowed by every ruleset until one
//!   holds it back. As text (a file, the lobby's wire) it names its items by
//!   their names in the game, a variant after its definition's
//!   (`bass-cross-megaman/gregar`: [`Ruleset::banned_names`],
//!   [`Ruleset::of_names`]).
//! - **A build under it** ([`Ruleset::problems`]; `build::Checked::under`):
//!   one problem for each item a category's fact of the build names that
//!   the ruleset doesn't allow ([`Wrong::NotAllowed`]), and with one version
//!   per build one where what it holds is more than one group's
//!   ([`Wrong::Mixed`]), read from what the build holds. Nothing is taken
//!   out of a build.
//!
//! This module names no game, no category, no version and no fact: they are
//! the game's data, and a fact's definitions are read by its type.

use crate::build::Builds;
use crate::check::Problem;
use crate::facts::{self, Stated};
use crate::{Side, Wrong, ids};
use nettai_battle::content::{Content, PlayerFact};
use nettai_content_api::{ChipHandle, Data, DataKey, FieldType, FormHandle, Registry};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The text table of the game's strings that names the categories
/// (`category.<fact>`) and the presets (`preset.<id>`). (A group is named
/// as its version is: `build::TEXT_TABLE`'s `version.<id>`.)
pub const TEXT_TABLE: &str = "rulesets";

/// The module of a game's rules that holds its rulesets' data.
const MODULE: &str = "rules/rulesets";

/// The field of a fact's record that chooses its definition's variant (a
/// patch card's: Bass-Cross MegaMan's version), whose variants are the
/// versions the definition's data names ([`variants_of`]).
pub const VARIANT: &str = "variant";

/// The field of a definition's data that says the version a part of it is
/// the variant of that version's (a patch card's effect: an effect only
/// Gregar's Bass-Cross MegaMan has says `version = "gregar"`).
const VARIANT_OF: &str = "version";

/// What a category holds back: a definition of its registry, by handle, or
/// one variant of one (a card each version's ROM gives effects of its own:
/// the variant chosen for it in a build, by its name, a version's id).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Item {
    pub def: u16,
    pub variant: Option<String>,
}

impl Item {
    /// The definition `def` itself.
    pub fn of(def: u16) -> Item {
        Item { def, variant: None }
    }
}

/// What a ruleset may hold back of a build: one fact of the rules' setup
/// that names definitions (a navi, a list of chips in a record with their
/// codes, a list of patch cards, a form, a list of forms), and its items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Category {
    /// The fact's name, which is the category's too.
    pub fact: String,
    /// The registry its definitions are of (an entry's collection in `of`).
    pub registry: Registry,
    pub of: Option<String>,
    /// The items a build can hold of it: the navis a side can play, the
    /// chips a folder lists, the forms the fact offers, else the registry's
    /// definitions of the game; a definition with variants, where the fact's
    /// record chooses one, each of its variants in its place.
    pub items: Vec<Item>,
}

impl Category {
    /// Its key in the game's strings' [`TEXT_TABLE`].
    pub fn text_key(&self) -> String {
        format!("category.{}", self.fact)
    }
}

/// A version of the game as what only its saves can own (docs/design/versions.md
/// §2.2): its id and color (the builds data's group's), and its members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub color: u32,
    members: BTreeSet<(Registry, u16)>,
}

impl Group {
    /// Its name's key in the game's strings' builds table
    /// (`build::TEXT_TABLE`), its version's.
    pub fn text_key(&self) -> String {
        format!("version.{}", self.id)
    }

    /// Whether item `item` of `registry` is the group's: a definition whose
    /// own `version` names it, or a variant of its name.
    pub fn has(&self, registry: Registry, item: &Item) -> bool {
        match &item.variant {
            Some(v) => *v == self.id,
            None => self.members.contains(&(registry, item.def)),
        }
    }
}

/// A ruleset's name: a preset's id (worded in each player's language), the
/// name a player gave their own, or the other player's (a lobby's host's,
/// which the joiner holds: a ruleset's name doesn't go over the wire, the
/// user: "ruleset name shouldn't travel to the opponent"; where its rules
/// are a preset's, it is that preset).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Name {
    Preset(String),
    Own(String),
    Theirs,
}

impl Name {
    /// A preset's key in the game's strings' [`TEXT_TABLE`], or the other
    /// player's (`theirs`: "Host's ruleset"); none of a player's own.
    pub fn text_key(&self) -> Option<String> {
        match self {
            Name::Preset(id) => Some(format!("preset.{id}")),
            Name::Theirs => Some("theirs".to_string()),
            Name::Own(_) => None,
        }
    }
}

/// A ruleset: its name, whether each build must be one group's, and for each category (by its fact's name) the
/// items it doesn't allow. A category that holds nothing back isn't in
/// `banned`, so two rulesets that allow the same are equal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ruleset {
    pub name: Name,
    /// Versions, one per build: what each build holds of the groups is one
    /// group's (a cartridge's). Off: mixed, the allowlists alone say what
    /// may be played.
    pub one_per_build: bool,
    banned: BTreeMap<String, BTreeSet<Item>>,
}

/// What a game says of its rulesets (its rules/rulesets module's data) and
/// its groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rulesets {
    game: String,
    /// The categories, in the order a ruleset's page lists them.
    pub categories: Vec<Category>,
    /// The groups, in the versions' order.
    pub groups: Vec<Group>,
    /// The rulesets ready-made, in order: the first is a player's until
    /// they pick another.
    pub presets: Vec<Ruleset>,
}

/// A list's items (none of another kind of value).
fn items(d: &Data) -> &[Data] {
    match d {
        Data::List(items) => items,
        _ => &[],
    }
}

/// What a fact names definitions by: its registry, the collection an entry
/// is of, and the field of a record that names one (none: the fact's
/// values are the definitions). None: the fact names no definitions.
pub fn definitions_of(content: &Content, fact: &str) -> Option<(Registry, Option<String>, Option<String>)> {
    let field = facts::field(content, fact)?;
    let element = match field.ty {
        FieldType::List(e, _) | FieldType::Array(e, _) => &**e,
        ty => ty,
    };
    match element {
        FieldType::Ref(registry, of) => Some((*registry, of.clone(), None)),
        FieldType::Record(fields) => {
            let mut named = fields.fields().iter().filter_map(|f| match &f.ty {
                FieldType::Ref(registry, of) => Some((*registry, of.clone(), Some(f.name.clone()))),
                _ => None,
            });
            let one = named.next()?;
            named.next().is_none().then_some(one)
        }
        _ => None,
    }
}

/// Whether fact `fact`'s records choose their definition's variant (a
/// [`VARIANT`] field: EXE5's and EXE6's patch cards).
fn chooses_variants(content: &Content, fact: &str) -> bool {
    let Some(field) = facts::field(content, fact) else { return false };
    let element = match field.ty {
        FieldType::List(e, _) | FieldType::Array(e, _) => &**e,
        ty => ty,
    };
    matches!(element, FieldType::Record(fields) if fields.index_of(VARIANT).is_some())
}

/// The variants of definition `h` of `registry`: the versions its data's
/// parts name ([`VARIANT_OF`]), at any depth, each once, in the order they
/// first come (EXE6's Bass-Cross MegaMan's effects': gregar, falzar). None
/// for most.
pub fn variants_of(content: &Content, registry: Registry, h: u16) -> Vec<String> {
    fn walk(d: &Data, out: &mut Vec<String>) {
        match d {
            Data::Map(entries) => {
                for (k, v) in entries {
                    match (k, v) {
                        (DataKey::Str(k), Data::Str(name)) if k == VARIANT_OF => {
                            if !out.contains(name) {
                                out.push(name.clone());
                            }
                        }
                        _ => walk(v, out),
                    }
                }
            }
            Data::List(items) => items.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Some(def) = ids::key_of(content, registry, h).and_then(|key| content.defs.definitions.get(registry, key)) {
        walk(&def.spec, &mut out);
    }
    out
}

/// Item `item` of `registry` for a message: its definition's name, and its
/// variant's id after it ("Bass-Cross MegaMan (gregar)").
pub fn shown(content: &Content, registry: Registry, item: &Item) -> String {
    let name = ids::shown(content, registry, item.def);
    match &item.variant {
        Some(v) => format!("{name} ({v})"),
        None => name,
    }
}

/// The items `side`'s fact `fact` names, each with the entry of its list it
/// is first at (none: the fact is one value), in order, each once: a
/// definition, or where the record chooses its definition's variant and the
/// definition has variants, the variant chosen.
fn named_by(content: &Content, side: &Side, fact: &str, field: Option<&str>, registry: Registry) -> Vec<(Item, Option<usize>)> {
    let Some(value) = side.facts.get(content, fact) else { return Vec::new() };
    let one = |v: &Stated| -> Option<Item> {
        match (v, field) {
            (Stated::Def(_, Some(h)), None) => Some(Item::of(*h)),
            (Stated::Record(fields), Some(name)) => {
                let get = |n: &str| fields.iter().find(|(k, _)| k == n).map(|(_, v)| v);
                let Some(Stated::Def(_, Some(h))) = get(name) else { return None };
                let variant = match get(VARIANT) {
                    Some(Stated::Variant(Some(v))) if variants_of(content, registry, *h).contains(v) => Some(v.clone()),
                    _ => None,
                };
                Some(Item { def: *h, variant })
            }
            _ => None,
        }
    };
    let found: Vec<(Item, Option<usize>)> = match &value {
        Stated::List(list) => list.iter().enumerate().filter_map(|(i, v)| one(v).map(|h| (h, Some(i)))).collect(),
        v => one(v).map(|h| (h, None)).into_iter().collect(),
    };
    let mut seen = BTreeSet::new();
    found.into_iter().filter(|(h, _)| seen.insert(h.clone())).collect()
}

impl Ruleset {
    /// A ruleset that allows everything, named `name`, versions mixed.
    pub fn open(name: Name) -> Ruleset {
        Ruleset { name, one_per_build: false, banned: BTreeMap::new() }
    }

    /// Whether it allows what `other` does: the same Versions and the same
    /// items held back, whatever either is named.
    pub fn same_rules(&self, other: &Ruleset) -> bool {
        self.one_per_build == other.one_per_build && self.banned == other.banned
    }

    /// Whether it allows item `item` of category `fact`.
    pub fn allows_item(&self, fact: &str, item: &Item) -> bool {
        !self.banned.get(fact).is_some_and(|b| b.contains(item))
    }

    /// Whether it allows definition `h` of category `fact` itself (of a
    /// definition with variants, see [`Rulesets::allows_def`]).
    pub fn allows(&self, fact: &str, h: u16) -> bool {
        self.allows_item(fact, &Item::of(h))
    }

    /// The items of category `fact` it doesn't allow, in order.
    pub fn banned(&self, fact: &str) -> impl Iterator<Item = &Item> + '_ {
        self.banned.get(fact).into_iter().flatten()
    }

    /// Whether it holds anything back.
    pub fn bans_anything(&self) -> bool {
        !self.banned.is_empty()
    }

    /// Allow item `item` of category `fact`, or not.
    pub fn set_allowed(&mut self, fact: &str, item: &Item, allowed: bool) {
        if allowed {
            if let Some(b) = self.banned.get_mut(fact) {
                b.remove(item);
                if b.is_empty() {
                    self.banned.remove(fact);
                }
            }
        } else {
            self.banned.entry(fact.to_string()).or_default().insert(item.clone());
        }
    }

    /// Allow of `category` its items in `only` and none of the others.
    pub fn allow_only(&mut self, category: &Category, only: &[Item]) {
        self.banned.remove(&category.fact);
        for item in &category.items {
            if !only.contains(item) {
                self.set_allowed(&category.fact, item, false);
            }
        }
    }

    /// Allow of `category` the items of the groups named in `chosen` and
    /// none of the other groups' items; what is no group's stays as it was
    /// (a group's quick choice: Gregar's Crosses, each team's Giga chips).
    pub fn allow_groups(&mut self, rulesets: &Rulesets, category: &Category, chosen: &[&str]) {
        for g in &rulesets.groups {
            for item in rulesets.group_items(category, g) {
                self.set_allowed(&category.fact, &item, chosen.contains(&g.id.as_str()));
            }
        }
    }

    /// The groups of `category` it allows whole, where each of the
    /// category's groups is allowed whole or not at all (none: some group is
    /// allowed in part, or the category has no group's items).
    pub fn groups_chosen<'r>(&self, rulesets: &'r Rulesets, category: &Category) -> Option<Vec<&'r str>> {
        let mut chosen = Vec::new();
        let mut any = false;
        for g in &rulesets.groups {
            let of = rulesets.group_items(category, g);
            if of.is_empty() {
                continue;
            }
            any = true;
            let allowed = of.iter().filter(|h| self.allows_item(&category.fact, h)).count();
            if allowed == of.len() {
                chosen.push(g.id.as_str());
            } else if allowed != 0 {
                return None;
            }
        }
        any.then_some(chosen)
    }

    /// How many of `category`'s items it allows.
    pub fn allowed_count(&self, category: &Category) -> usize {
        category.items.iter().filter(|h| self.allows_item(&category.fact, h)).count()
    }

    /// What is wrong with `side` under this ruleset: for each category, a
    /// problem for each item the side's fact names that it doesn't allow, at
    /// the entry that first names it; and with one version per build, one
    /// where what the side holds is more than one group's. None: the ruleset
    /// lets the side play.
    pub fn problems(&self, rulesets: &Rulesets, content: &Content, side: &Side) -> Vec<Problem> {
        let mut out = Vec::new();
        let mut held: Vec<(Registry, Item)> = Vec::new();
        for category in &rulesets.categories {
            let Some((registry, _, field)) = definitions_of(content, &category.fact) else { continue };
            for (item, entry) in named_by(content, side, &category.fact, field.as_deref(), registry) {
                held.push((registry, item.clone()));
                if !self.allows_item(&category.fact, &item) {
                    let text = format!("{} isn't allowed ({})", shown(content, registry, &item), category.fact);
                    out.push(Problem { side: Some(0), field: Some(category.fact.clone()), entry, what: Wrong::NotAllowed { registry, item }, text });
                }
            }
        }
        if self.one_per_build {
            // (Each group with what the side holds of it; the one it holds
            // most of, the first in order where two tie, is the build's, and
            // what it holds of the others is listed.)
            let of: Vec<(&Group, Vec<(Registry, Item)>)> = rulesets
                .groups
                .iter()
                .map(|g| (g, held.iter().filter(|(r, h)| g.has(*r, h)).cloned().collect::<Vec<_>>()))
                .filter(|(_, items)| !items.is_empty())
                .collect();
            if of.len() > 1 {
                let most = of.iter().enumerate().max_by_key(|(i, (_, items))| (items.len(), std::cmp::Reverse(*i))).map(|(i, _)| i).unwrap_or(0);
                let groups: Vec<String> = of.iter().map(|(g, _)| g.id.clone()).collect();
                let items: Vec<(Registry, Item)> = of.iter().enumerate().filter(|(i, _)| *i != most).flat_map(|(_, (_, items))| items.iter().cloned()).collect();
                let listed: Vec<String> = items.iter().map(|(r, h)| shown(content, *r, h)).collect();
                let text = format!("mixes the versions {} ({})", groups.join(" and "), listed.join(", "));
                out.push(Problem { side: Some(0), field: None, entry: None, what: Wrong::Mixed { groups, items }, text });
            }
        }
        out
    }

    /// What it doesn't allow, by name: each category with anything held
    /// back, by its fact's name, with the names of its items in the game (a
    /// chip's `cannon`, an entry's in its collection; a variant after its
    /// definition's, `bass-cross-megaman/gregar`), in order. What a file and
    /// the lobby's wire state.
    pub fn banned_names(&self, content: &Content) -> Vec<(String, Vec<String>)> {
        self.banned
            .iter()
            .filter_map(|(fact, banned)| {
                let (registry, _, _) = definitions_of(content, fact)?;
                let name = |item: &Item| {
                    let def = ids::name_of(content, registry, item.def)?;
                    Some(match &item.variant {
                        Some(v) => format!("{def}/{v}"),
                        None => def.to_string(),
                    })
                };
                Some((fact.clone(), banned.iter().filter_map(name).collect()))
            })
            .collect()
    }

    /// A ruleset of the game of `rulesets` from its names
    /// ([`Ruleset::banned_names`]'s): an error names a fact that is no
    /// category of the game's or a name its registry hasn't.
    pub fn of_names(rulesets: &Rulesets, content: &Content, name: Name, one_per_build: bool, banned: &[(String, Vec<String>)]) -> Result<Ruleset, String> {
        let mut out = Ruleset { name, one_per_build, banned: BTreeMap::new() };
        for (fact, names) in banned {
            let category = rulesets.category(fact).ok_or_else(|| format!("{} has no ruleset category {fact:?}", rulesets.game))?;
            for name in names {
                let def = |n: &str| ids::handle_of(content, &rulesets.game, category.registry, category.of.as_deref(), n);
                let item = match def(name) {
                    Some(h) => Item::of(h),
                    None => {
                        let (n, v) = name.rsplit_once('/').ok_or_else(|| format!("{fact}: {} has no {:?}", rulesets.game, name))?;
                        let h = def(n).ok_or_else(|| format!("{fact}: {} has no {:?}", rulesets.game, n))?;
                        Item { def: h, variant: Some(v.to_string()) }
                    }
                };
                if !category.items.contains(&item) {
                    return Err(format!("{fact}: {name:?} is none of its items"));
                }
                out.set_allowed(fact, &item, false);
            }
        }
        Ok(out)
    }
}

impl Rulesets {
    /// `game`'s rulesets, read from its rules/rulesets module as the content
    /// loaded it, and its groups, from its builds data; an error names what
    /// the module lacks (or that the game has none).
    pub fn of(content: &Arc<Content>, game: &str) -> Result<Rulesets, String> {
        crate::playable(content, game)?;
        let module = format!("{game}:{MODULE}");
        let data = nettai_battle::behavior::content_module_data(content, &module).ok_or_else(|| format!("{game} says nothing of its rulesets (no module {MODULE})"))??;
        let builds = Builds::of(content, game)?;
        let mut categories = Vec::new();
        for c in items(data.field("categories")) {
            let Data::Str(fact) = c.field("fact") else { return Err(format!("{module}: a category names its `fact`")) };
            let (registry, of, _) = definitions_of(content, fact).ok_or_else(|| format!("{module}: {fact} is no fact of {game}'s that names definitions"))?;
            let defs = category_items(content, game, &builds, fact, registry, of.as_deref())?;
            let variants = chooses_variants(content, fact);
            let items = defs
                .into_iter()
                .flat_map(|h| {
                    let of = if variants { variants_of(content, registry, h) } else { Vec::new() };
                    if of.is_empty() { vec![Item::of(h)] } else { of.into_iter().map(|v| Item { def: h, variant: Some(v) }).collect() }
                })
                .collect();
            categories.push(Category { fact: fact.clone(), registry, of, items });
        }
        let groups = groups(content, game, &builds);
        let mut out = Rulesets { game: game.to_string(), categories, groups, presets: Vec::new() };
        for p in items(data.field("presets")) {
            let Data::Str(id) = p.field("id") else { return Err(format!("{module}: a preset has its `id`")) };
            let mut ruleset = Ruleset::open(Name::Preset(id.clone()));
            ruleset.one_per_build = match p.field("versions") {
                Data::Str(v) if v == "one_per_build" => true,
                Data::Str(v) if v == "mixed" => false,
                _ => return Err(format!("{module}: preset {id}: its `versions` is \"one_per_build\" or \"mixed\"")),
            };
            if let Data::Map(only) = p.field("only") {
                for (fact, allowed) in only {
                    let DataKey::Str(fact) = fact else { continue };
                    let category = out.category(fact).ok_or_else(|| format!("{module}: preset {id}: no category {fact:?}"))?.clone();
                    let allowed: Vec<u16> = self::items(allowed)
                        .iter()
                        .map(|n| n.str().and_then(|n| ids::handle_of(content, game, category.registry, category.of.as_deref(), n)).ok_or_else(|| format!("{module}: preset {id}: {fact}: {n:?} is none of its items")))
                        .collect::<Result<_, _>>()?;
                    let only: Vec<Item> = category.items.iter().filter(|i| allowed.contains(&i.def)).cloned().collect();
                    ruleset.allow_only(&category, &only);
                }
            }
            out.presets.push(ruleset);
        }
        if out.presets.is_empty() {
            return Err(format!("{module}: no `presets`"));
        }
        Ok(out)
    }

    /// The game these are the rulesets of.
    pub fn game(&self) -> &str {
        &self.game
    }

    /// The category of fact `fact`.
    pub fn category(&self, fact: &str) -> Option<&Category> {
        self.categories.iter().find(|c| c.fact == fact)
    }

    /// The group named `id`.
    pub fn group(&self, id: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// `category`'s items that are `group`'s, in the category's order.
    pub fn group_items(&self, category: &Category, group: &Group) -> Vec<Item> {
        category.items.iter().filter(|h| group.has(category.registry, h)).cloned().collect()
    }

    /// The group item `item` of `registry` is, if it is one's.
    pub fn group_of(&self, registry: Registry, item: &Item) -> Option<&Group> {
        self.groups.iter().find(|g| g.has(registry, item))
    }

    /// The groups with items in `category`, in order.
    pub fn groups_in(&self, category: &Category) -> Vec<&Group> {
        self.groups.iter().filter(|g| !self.group_items(category, g).is_empty()).collect()
    }

    /// Whether `ruleset` allows definition `h` of category `fact` in some
    /// way: itself, or one of its variants (a card a build can hold under
    /// it, of the variant it allows).
    pub fn allows_def(&self, ruleset: &Ruleset, fact: &str, h: u16) -> bool {
        match self.category(fact) {
            Some(c) => c.items.iter().filter(|i| i.def == h).any(|i| ruleset.allows_item(fact, i)) || !c.items.iter().any(|i| i.def == h),
            None => true,
        }
    }

    /// The variants of definition `h` of category `fact` that `ruleset`
    /// allows, in order (none: it has none).
    pub fn allowed_variants(&self, ruleset: &Ruleset, fact: &str, h: u16) -> Vec<String> {
        self.category(fact)
            .into_iter()
            .flat_map(|c| c.items.iter())
            .filter(|i| i.def == h && ruleset.allows_item(fact, i))
            .filter_map(|i| i.variant.clone())
            .collect()
    }

    /// The preset named `id`.
    pub fn preset(&self, id: &str) -> Option<&Ruleset> {
        self.presets.iter().find(|p| p.name == Name::Preset(id.to_string()))
    }

    /// `ruleset` named as this side names its rules: the preset whose rules
    /// they are, else the other player's ([`Name::Theirs`]). (A lobby's
    /// ruleset comes over the wire without its name.)
    pub fn named_here(&self, mut ruleset: Ruleset) -> Ruleset {
        ruleset.name = self.presets.iter().find(|p| p.same_rules(&ruleset)).map_or(Name::Theirs, |p| p.name.clone());
        ruleset
    }

    /// The first preset: a player's until they pick another.
    pub fn first(&self) -> &Ruleset {
        &self.presets[0]
    }
}

/// The game's groups (docs/design/versions.md §2.2): each group of its
/// builds data, with the chips and forms whose own `version` names it (a
/// variant of its name is its too: [`Group::has`]).
fn groups(content: &Content, game: &str, builds: &Builds) -> Vec<Group> {
    let mut groups: Vec<Group> = builds.groups.iter().map(|g| Group { id: g.id.clone(), color: g.color, members: BTreeSet::new() }).collect();
    let own = |registry: Registry, key: &str| content.defs.extension(registry, key, "version").and_then(Data::str).map(str::to_string);
    for h in 0..content.defs.chips.len() as u16 {
        let key = &content.defs.chip(ChipHandle(h)).key;
        if let Some(v) = own(Registry::Chip, key).filter(|_| ids::in_game(content, game, key))
            && let Some(g) = groups.iter_mut().find(|g| g.id == v)
        {
            g.members.insert((Registry::Chip, h));
        }
    }
    for h in 0..content.defs.forms.len() as u16 {
        let form = content.defs.form(FormHandle(h));
        if let Some(v) = form.record.version.clone().filter(|_| ids::in_game(content, game, &form.key))
            && let Some(g) = groups.iter_mut().find(|g| g.id == v)
        {
            g.members.insert((Registry::Form, h));
        }
    }
    groups
}

/// A category's definitions: those of `registry` a build of `game` can
/// hold in fact `fact`.
fn category_items(content: &Arc<Content>, game: &str, builds: &Builds, fact: &str, registry: Registry, of: Option<&str>) -> Result<Vec<u16>, String> {
    Ok(match registry {
        Registry::Navi => crate::navis(content, game).into_iter().map(|n| n.0).collect(),
        // (The chips a folder lists: every chip of the game but those no
        // folder holds, as the folder rules' view of a new build says.)
        Registry::Chip => {
            let side = builds.new_build(content, &builds.groups[0].id)?;
            let mut checked = crate::build::Checked::of(content, game, &side);
            let view = checked.folder_view(content, &crate::Folder::EMPTY).ok_or_else(|| format!("{game}'s rules have no folder view"))?;
            (0..content.defs.chips.len() as u16).filter(|&h| ids::in_game(content, game, &content.defs.chip(ChipHandle(h)).key) && view.lists(ChipHandle(h))).collect()
        }
        // (The forms a fact of forms offers, in the battle's order: the
        // build's choices of them.)
        Registry::Form if content.defs.fact_name(PlayerFact::FormList) == Some(fact) => crate::listed_forms(content, game)?.into_iter().map(|f| f.0).collect(),
        Registry::Form if facts::field(content, fact).is_some_and(|f| facts::offered_forms(content, &f).is_some_and(|o| !o.is_empty())) => {
            let side = builds.new_build(content, &builds.groups[0].id)?;
            builds.form_choices(content, &side).into_iter().find(|c| c.fact == fact).map(|c| c.forms.into_iter().map(|f| f.0).collect()).unwrap_or_default()
        }
        _ => {
            let defaults = crate::Facts::defaults(content).get(content, fact).map(|v| v.defs()).unwrap_or_default();
            if !defaults.is_empty() {
                defaults
            } else {
                ids::all_of(content, registry, of).into_iter().filter(|&h| ids::key_of(content, registry, h).is_some_and(|key| ids::in_game(content, game, key))).collect()
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::Checked;
    use crate::testing::{exe4_content, exe5_content, exe6_content, patch_cards};
    use nettai_battle::rules::Fact;
    use nettai_content_api::Value;

    /// Each game's categories, its groups and what they hold, and its
    /// presets, as its data says them.
    #[test]
    fn each_game_has_its_categories_groups_and_presets() {
        for (content, game, facts, forms, groups) in [
            (exe6_content(), "exe6", vec!["navi", "folder", "navicust_programs", "patch_cards", "cybeast", "crosses"], "crosses", ["gregar", "falzar"]),
            (exe5_content(), "exe5", vec!["navi", "folder", "navicust_programs", "patch_cards", "souls"], "souls", ["protoman", "colonel"]),
            (exe4_content(), "exe4", vec!["folder", "navicust_programs", "patch_cards", "souls"], "souls", ["redsun", "bluemoon"]),
        ] {
            let r = Rulesets::of(&content, game).unwrap();
            assert_eq!(r.categories.iter().map(|c| c.fact.as_str()).collect::<Vec<_>>(), facts, "{game}");
            for c in &r.categories {
                assert!(!c.items.is_empty(), "{game} {}", c.fact);
            }
            assert_eq!(r.groups.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(), groups, "{game}");
            // Each group's souls or Crosses (each version's own), and its own Giga chips.
            let category = r.category(forms).unwrap();
            let chips = r.category("folder").unwrap();
            for g in &r.groups {
                let own = r.group_items(category, g).len();
                assert_eq!(own, if game == "exe6" { 5 } else { 6 }, "{game} {}", g.id);
                assert!(r.group_items(chips, g).len() >= 5, "{game} {}", g.id);
            }
            // (A chip both versions hold is none's: EXE5's ChaosLrd, EXE4's Duo.)
            let both = match game {
                "exe5" => Some("chaoslrd"),
                "exe4" => Some("duo"),
                _ => None,
            };
            if let Some(both) = both {
                let h = crate::ids::chip(&content, game, both).unwrap().0;
                assert!(r.group_of(Registry::Chip, &Item::of(h)).is_none(), "{game} {both}");
            }
            // Bass-Cross MegaMan is two items, a variant of each group (EXE4 has no such card).
            let cards = r.category("patch_cards").unwrap();
            let variants: Vec<&Item> = cards.items.iter().filter(|i| i.variant.is_some()).collect();
            if game == "exe4" {
                assert!(variants.is_empty());
            } else {
                assert_eq!(variants.iter().map(|i| i.variant.as_deref().unwrap()).collect::<Vec<_>>(), groups, "{game}");
                for (g, v) in r.groups.iter().zip(&variants) {
                    assert_eq!(r.group_items(cards, g), [(*v).clone()], "{game}");
                }
            }
            // The presets: Masters class, one version per build; Senior class, without patch cards; Unrestricted.
            let names: Vec<Name> = r.presets.iter().map(|p| p.name.clone()).collect();
            let expected: Vec<Name> = ["masters", "senior", "unrestricted"].iter().map(|n| Name::Preset(n.to_string())).collect();
            assert_eq!(names, expected, "{game}");
            assert!(!r.first().bans_anything() && r.first().one_per_build);
            assert!(!r.preset("unrestricted").unwrap().one_per_build);
            assert_eq!(r.first().groups_chosen(&r, category), Some(groups.to_vec()));
            assert_eq!(r.first().groups_chosen(&r, chips), Some(groups.to_vec()));
            let senior = r.preset("senior").unwrap();
            assert!(senior.one_per_build && senior.allowed_count(cards) == 0);
            // A new build of each group plays under Masters class; one version's alone (the page's quick choice)
            // refuses the other's.
            let builds = Builds::of(&content, game).unwrap();
            for (v, other) in [(groups[0], groups[1]), (groups[1], groups[0])] {
                let side = builds.new_build(&content, v).unwrap();
                assert!(Checked::under(&content, game, &side, &r, r.first()).ready(), "{game} {v}");
                let mut alone = Ruleset::open(Name::Own(other.to_string()));
                alone.one_per_build = true;
                for c in r.categories.clone() {
                    alone.allow_groups(&r, &c, &[other]);
                }
                let problems = Checked::under(&content, game, &side, &r, &alone).problems;
                assert!(!problems.is_empty() && problems.iter().all(|p| [forms, "cybeast"].contains(&p.field.as_deref().unwrap_or_default())), "{game} {v}");
            }
        }
    }

    /// A build under a ruleset: each item it holds that the ruleset doesn't
    /// allow is a problem at its first entry, of its fact; as names a
    /// ruleset reads back the same.
    #[test]
    fn a_build_holding_what_a_ruleset_bans_has_a_problem_for_each() {
        let content = exe6_content();
        let r = Rulesets::of(&content, "exe6").unwrap();
        let builds = Builds::of(&content, "exe6").unwrap();
        let gregar = builds.new_build(&content, "gregar").unwrap();
        // Falzar's alone: Gregar's Cybeast and five Crosses aren't allowed.
        let mut falzar = Ruleset::open(Name::Own("Falzar".into()));
        falzar.one_per_build = true;
        for c in r.categories.clone() {
            falzar.allow_groups(&r, &c, &["falzar"]);
        }
        let problems = Checked::under(&content, "exe6", &gregar, &r, &falzar).problems;
        assert_eq!(problems.len(), 6);
        assert!(problems.iter().all(|p| ["crosses", "cybeast"].contains(&p.field.as_deref().unwrap()) && matches!(p.what, Wrong::NotAllowed { registry: Registry::Form, .. })));
        assert_eq!(problems.iter().map(|p| p.entry).collect::<Vec<_>>(), [None].into_iter().chain((0..5).map(Some)).collect::<Vec<_>>());
        // A chip held back: one problem however many copies, at the first.
        let cannon = crate::ids::chip(&content, "exe6", "cannon").unwrap();
        let mut no_cannon = Ruleset::open(Name::Own("No Cannon".into()));
        no_cannon.set_allowed("folder", &Item::of(cannon.0), false);
        let problems = no_cannon.problems(&r, &content, &gregar);
        assert_eq!(problems.len(), 1);
        assert_eq!((problems[0].entry, &problems[0].what), (Some(0), &Wrong::NotAllowed { registry: Registry::Chip, item: Item::of(cannon.0) }));
        // The folder's view under it: Cannon can't go in, and each entry of it is a problem.
        let mut checked = Checked::under(&content, "exe6", &gregar, &r, &no_cannon);
        let folder = gregar.folder(&content);
        let view = checked.folder_view(&content, &folder).unwrap();
        assert_eq!(view.refusal(cannon), Some(&Wrong::NotAllowed { registry: Registry::Chip, item: Item::of(cannon.0) }));
        let cannons: Vec<usize> = folder.chips.iter().enumerate().filter(|(_, c)| c.is_some_and(|c| c.id == cannon)).map(|(i, _)| i).collect();
        assert!(cannons.len() > 1 && cannons.iter().all(|&i| !view.entry_problems(&content, &folder, i).is_empty()));
        assert!(!matches!(Checked::of(&content, "exe6", &gregar).folder_view(&content, &folder).unwrap().refusal(cannon), Some(Wrong::NotAllowed { .. })));
        // As names, and back.
        let names = no_cannon.banned_names(&content);
        assert_eq!(names, vec![("folder".to_string(), vec!["cannon".to_string()])]);
        assert_eq!(Ruleset::of_names(&r, &content, no_cannon.name.clone(), false, &names).unwrap(), no_cannon);
        assert!(Ruleset::of_names(&r, &content, no_cannon.name.clone(), false, &[("hp".into(), vec![])]).is_err());
        assert!(Ruleset::of_names(&r, &content, no_cannon.name.clone(), false, &[("folder".into(), vec!["nothing".into()])]).is_err());
        // Allowed again, it holds nothing back, and equals an open one.
        no_cannon.set_allowed("folder", &Item::of(cannon.0), true);
        assert_eq!(no_cannon, Ruleset::open(Name::Own("No Cannon".into())));
        // Patch cards none: a build holding one has its problem, at its entry.
        let mut carded = gregar.clone();
        carded.set_fact(&content, crate::facts::PATCH_CARDS, &patch_cards(&content, "exe6", "starman")).unwrap();
        let problems = r.preset("senior").unwrap().problems(&r, &content, &carded);
        assert_eq!(problems.len(), 1);
        assert_eq!((problems[0].field.as_deref(), problems[0].entry), (Some("patch_cards"), Some(0)));
    }

    /// A card's variant is an item of its own (docs/design/versions.md
    /// §2.1): a ruleset bans Gregar's Bass-Cross MegaMan and not Falzar's, by
    /// name too, and the card is still one a build can hold under it, of
    /// the variant it allows.
    #[test]
    fn a_ruleset_bans_a_cards_variant() {
        let content = exe6_content();
        let r = Rulesets::of(&content, "exe6").unwrap();
        let builds = Builds::of(&content, "exe6").unwrap();
        let card = crate::ids::entry(&content, "exe6", crate::facts::PATCH_CARDS, "bass-cross-megaman").unwrap().0;
        let gregars = Item { def: card, variant: Some("gregar".into()) };
        let mut ruleset = Ruleset::open(Name::Own("No Gregar card".into()));
        ruleset.set_allowed("patch_cards", &gregars, false);
        assert!(r.allows_def(&ruleset, "patch_cards", card) && r.allowed_variants(&ruleset, "patch_cards", card) == ["falzar"]);
        let names = ruleset.banned_names(&content);
        assert_eq!(names, vec![("patch_cards".to_string(), vec!["bass-cross-megaman/gregar".to_string()])]);
        assert_eq!(Ruleset::of_names(&r, &content, ruleset.name.clone(), false, &names).unwrap(), ruleset);
        let mut side = builds.new_build(&content, "gregar").unwrap();
        side.set_fact(&content, crate::facts::PATCH_CARDS, &patch_cards(&content, "exe6", "bass-cross-megaman/gregar")).unwrap();
        let problems = ruleset.problems(&r, &content, &side);
        assert_eq!(problems.iter().map(|p| &p.what).collect::<Vec<_>>(), [&Wrong::NotAllowed { registry: Registry::Entry, item: gregars.clone() }]);
        side.set_fact(&content, crate::facts::PATCH_CARDS, &patch_cards(&content, "exe6", "bass-cross-megaman/falzar")).unwrap();
        assert!(ruleset.problems(&r, &content, &side).is_empty());
    }

    /// With one version per build, a build holding two groups' things is a
    /// problem that lists what is the other group's, read from what it holds:
    /// its Crosses, its Cybeast, its Bass-Cross MegaMan card's variant;
    /// mixed lets it be.
    #[test]
    fn one_version_per_build_refuses_a_build_of_two_groups() {
        let content = exe6_content();
        let r = Rulesets::of(&content, "exe6").unwrap();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "gregar").unwrap();
        // Two of Falzar's Crosses in place of Gregar's last two.
        let crosses = r.category("crosses").unwrap();
        let falzar = r.group_items(crosses, r.group("falzar").unwrap());
        let gregar = r.group_items(crosses, r.group("gregar").unwrap());
        let mixed: Vec<Fact> = gregar[..3].iter().chain(&falzar[..2]).map(|h| Fact::Value(Value::Def(Registry::Form, h.def))).collect();
        side.set_fact(&content, "crosses", &mixed).unwrap();
        let problems = r.first().problems(&r, &content, &side);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].what, Wrong::Mixed { groups: vec!["gregar".into(), "falzar".into()], items: falzar[..2].iter().map(|h| (Registry::Form, h.clone())).collect() });
        assert!(r.preset("unrestricted").unwrap().problems(&r, &content, &side).is_empty());
        // Gregar's Crosses alone with Falzar's Cybeast: it mixes them.
        let mut beast = builds.new_build(&content, "gregar").unwrap();
        beast.set_fact(&content, "cybeast", &[crate::facts::version_fact(&content, "falzar").unwrap()]).unwrap();
        let falzar_beast = crate::ids::form(&content, "exe6", "falzar-beast").unwrap().0;
        assert!(matches!(&r.first().problems(&r, &content, &beast)[..], [p] if p.what == Wrong::Mixed { groups: vec!["gregar".into(), "falzar".into()], items: vec![(Registry::Form, Item::of(falzar_beast))] }));
        // And Falzar's card with a Gregar build.
        let mut card = builds.new_build(&content, "gregar").unwrap();
        card.set_fact(&content, crate::facts::PATCH_CARDS, &patch_cards(&content, "exe6", "bass-cross-megaman/falzar")).unwrap();
        let h = crate::ids::entry(&content, "exe6", crate::facts::PATCH_CARDS, "bass-cross-megaman").unwrap().0;
        assert!(matches!(&r.first().problems(&r, &content, &card)[..], [p] if p.what == Wrong::Mixed { groups: vec!["gregar".into(), "falzar".into()], items: vec![(Registry::Entry, Item { def: h, variant: Some("falzar".into()) })] }));
        card.set_fact(&content, crate::facts::PATCH_CARDS, &patch_cards(&content, "exe6", "bass-cross-megaman/gregar")).unwrap();
        assert!(r.first().problems(&r, &content, &card).is_empty());
    }

    /// The navi a build is is held to the navis' category.
    #[test]
    fn a_navi_held_back_is_a_problem_of_the_navi() {
        let content = exe5_content();
        let r = Rulesets::of(&content, "exe5").unwrap();
        let navis = r.category("navi").unwrap();
        let builds = Builds::of(&content, "exe5").unwrap();
        let side = builds.new_build(&content, "protoman").unwrap();
        let navi = side.navi(&content);
        let mut ruleset = r.first().clone();
        ruleset.allow_only(navis, &navis.items.iter().filter(|n| n.def != navi.0).cloned().collect::<Vec<_>>());
        let problems = ruleset.problems(&r, &content, &side);
        assert_eq!(problems.len(), 1);
        assert_eq!((problems[0].field.as_deref(), problems[0].entry, &problems[0].what), (Some("navi"), None, &Wrong::NotAllowed { registry: Registry::Navi, item: Item::of(navi.0) }));
    }
}
