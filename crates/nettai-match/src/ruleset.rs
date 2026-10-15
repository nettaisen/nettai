//! A ruleset: which builds may play a match (docs/design/app-ui.md §2.59;
//! docs/design/rules-in-luau.md, "Rulesets"; docs/design/versions.md §2.2,
//! §2.3). Its name, whether each build must be one version's (Versions: one
//! per build, or mixed), and for each of its game's categories the
//! definitions it doesn't allow: what may be played, and nothing else
//! (closed builds is the lobby's setting beside it: the user, "i think
//! closed setup should not be part of the ruleset actually"). It says
//! nothing of how a round plays, so the simulation never reads it.
//!
//! - **The game's categories and presets** ([`Rulesets`], read from the
//!   game's own data, its rules/rulesets module): a category is a fact of
//!   the rules' setup that names definitions, by the fact's name; its
//!   registry, the field of a record that names one and its items are the
//!   fact's type's ([`Category`]). The presets are the game's rulesets
//!   ready-made, in order, the first a player's until they pick another.
//! - **The game's groups** ([`Group`]): each version of the game as what a
//!   save of it can own that the other's can't (docs/design/versions.md
//!   §2.2). A group is a version of the game's builds data (its id, its
//!   color, its name in the builds' strings), and its members are the chips
//!   and forms whose own `version` names it (EXE6's Giga chips, Crosses and
//!   Beasts), and the definitions the version states (EXE4's and EXE5's
//!   souls and Giga chips) that no other version does. A category's items in
//!   a group are its quick choice, and a ruleset of one version per build
//!   holds every build to one group.
//! - **A ruleset** ([`Ruleset`]): what it doesn't name is allowed, so a
//!   definition the game gains later is allowed by every ruleset until one
//!   holds it back. As text (a file, the lobby's wire) it names its items by
//!   their names in the game ([`Ruleset::banned_names`], [`Ruleset::of_names`]).
//! - **A build under it** ([`Ruleset::problems`]; `build::Checked::under`):
//!   one problem for each definition a category's fact of the build names
//!   that the ruleset doesn't allow ([`Wrong::NotAllowed`]), and with one
//!   version per build one where what it holds is more than one group's
//!   ([`Wrong::Mixed`]), read from what the build holds, never from a fact
//!   that says its version. Nothing is taken out of a build.
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

/// What a ruleset may hold back of a build: one fact of the rules' setup
/// that names definitions (a navi, a list of chips in a record with their
/// codes, a list of patch cards, a list of forms), and its items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Category {
    /// The fact's name, which is the category's too.
    pub fact: String,
    /// The registry its definitions are of (an entry's collection in `of`).
    pub registry: Registry,
    pub of: Option<String>,
    /// The definitions a build can hold of it, by handle: the navis a side
    /// can play, the chips a folder lists, the forms the fact offers, else
    /// the registry's definitions of the game.
    pub items: Vec<u16>,
}

impl Category {
    /// Its key in the game's strings' [`TEXT_TABLE`].
    pub fn text_key(&self) -> String {
        format!("category.{}", self.fact)
    }
}

/// A version of the game as what only its saves can own (docs/design/versions.md
/// §2.2): its id and color (the builds data's version's), and its members.
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

    /// Whether definition `h` of `registry` is the group's.
    pub fn has(&self, registry: Registry, h: u16) -> bool {
        self.members.contains(&(registry, h))
    }
}

/// A ruleset's name: a preset's id (worded in each player's language), or
/// the name a player gave their own.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Name {
    Preset(String),
    Own(String),
}

impl Name {
    /// A preset's key in the game's strings' [`TEXT_TABLE`] (none of a
    /// player's own).
    pub fn text_key(&self) -> Option<String> {
        match self {
            Name::Preset(id) => Some(format!("preset.{id}")),
            Name::Own(_) => None,
        }
    }
}

/// A ruleset: its name, whether each build must be one group's, and for each category (by its fact's name) the
/// definitions it doesn't allow, by handle. A category that holds nothing
/// back isn't in `banned`, so two rulesets that allow the same are equal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ruleset {
    pub name: Name,
    /// Versions, one per build: what each build holds of the groups is one
    /// group's (a cartridge's). Off: mixed, the allowlists alone say what
    /// may be played.
    pub one_per_build: bool,
    banned: BTreeMap<String, BTreeSet<u16>>,
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

/// The definitions `side`'s fact `fact` names, each with the entry of its
/// list it is first at (none: the fact is one value), in order, each once.
fn named_by(content: &Content, side: &Side, fact: &str, field: Option<&str>) -> Vec<(u16, Option<usize>)> {
    let Some(value) = side.facts.get(content, fact) else { return Vec::new() };
    let one = |v: &Stated| -> Option<u16> {
        match (v, field) {
            (Stated::Def(_, Some(h)), None) => Some(*h),
            (Stated::Record(fields), Some(name)) => fields.iter().find(|(n, _)| n == name).and_then(|(_, v)| if let Stated::Def(_, Some(h)) = v { Some(*h) } else { None }),
            _ => None,
        }
    };
    let found: Vec<(u16, Option<usize>)> = match &value {
        Stated::List(list) => list.iter().enumerate().filter_map(|(i, v)| one(v).map(|h| (h, Some(i)))).collect(),
        v => one(v).map(|h| (h, None)).into_iter().collect(),
    };
    let mut seen = BTreeSet::new();
    found.into_iter().filter(|(h, _)| seen.insert(*h)).collect()
}

impl Ruleset {
    /// A ruleset that allows everything, named `name`, versions mixed.
    pub fn open(name: Name) -> Ruleset {
        Ruleset { name, one_per_build: false, banned: BTreeMap::new() }
    }

    /// Whether it allows definition `h` of category `fact`.
    pub fn allows(&self, fact: &str, h: u16) -> bool {
        !self.banned.get(fact).is_some_and(|b| b.contains(&h))
    }

    /// The definitions of category `fact` it doesn't allow, by handle, in
    /// handle order.
    pub fn banned(&self, fact: &str) -> impl Iterator<Item = u16> + '_ {
        self.banned.get(fact).into_iter().flatten().copied()
    }

    /// Whether it holds anything back.
    pub fn bans_anything(&self) -> bool {
        !self.banned.is_empty()
    }

    /// Allow definition `h` of category `fact`, or not.
    pub fn set_allowed(&mut self, fact: &str, h: u16, allowed: bool) {
        if allowed {
            if let Some(b) = self.banned.get_mut(fact) {
                b.remove(&h);
                if b.is_empty() {
                    self.banned.remove(fact);
                }
            }
        } else {
            self.banned.entry(fact.to_string()).or_default().insert(h);
        }
    }

    /// Allow of `category` its items in `only` and none of the others.
    pub fn allow_only(&mut self, category: &Category, only: &[u16]) {
        self.banned.remove(&category.fact);
        for &h in &category.items {
            if !only.contains(&h) {
                self.set_allowed(&category.fact, h, false);
            }
        }
    }

    /// Allow of `category` the items of the groups named in `chosen` and
    /// none of the other groups' items; what is no group's stays as it was
    /// (a group's quick choice: Gregar's Crosses, each team's Giga chips).
    pub fn allow_groups(&mut self, rulesets: &Rulesets, category: &Category, chosen: &[&str]) {
        for g in &rulesets.groups {
            for h in rulesets.group_items(category, g) {
                self.set_allowed(&category.fact, h, chosen.contains(&g.id.as_str()));
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
            let allowed = of.iter().filter(|&&h| self.allows(&category.fact, h)).count();
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
        category.items.iter().filter(|&&h| self.allows(&category.fact, h)).count()
    }

    /// What is wrong with `side` under this ruleset: for each category, a
    /// problem for each definition the side's fact names that it doesn't
    /// allow, at the entry that first names it; and with one version per
    /// build, one where what the side holds is more than one group's. None:
    /// the ruleset lets the side play.
    pub fn problems(&self, rulesets: &Rulesets, content: &Content, side: &Side) -> Vec<Problem> {
        let mut out = Vec::new();
        let mut held: Vec<(Registry, u16)> = Vec::new();
        for category in &rulesets.categories {
            let Some((registry, _, field)) = definitions_of(content, &category.fact) else { continue };
            for (h, entry) in named_by(content, side, &category.fact, field.as_deref()) {
                held.push((registry, h));
                if !self.allows(&category.fact, h) {
                    let text = format!("{} isn't allowed ({})", ids::shown(content, registry, h), category.fact);
                    out.push(Problem { side: Some(0), field: Some(category.fact.clone()), entry, what: Wrong::NotAllowed { registry, def: h }, text });
                }
            }
        }
        if self.one_per_build {
            // (Each group with what the side holds of it; the one it holds
            // most of, the first in order where two tie, is the build's, and
            // what it holds of the others is listed.)
            let of: Vec<(&Group, Vec<(Registry, u16)>)> = rulesets
                .groups
                .iter()
                .map(|g| (g, held.iter().copied().filter(|&(r, h)| g.has(r, h)).collect::<Vec<_>>()))
                .filter(|(_, items)| !items.is_empty())
                .collect();
            if of.len() > 1 {
                let most = of.iter().enumerate().max_by_key(|(i, (_, items))| (items.len(), std::cmp::Reverse(*i))).map(|(i, _)| i).unwrap_or(0);
                let groups: Vec<String> = of.iter().map(|(g, _)| g.id.clone()).collect();
                let items: Vec<(Registry, u16)> = of.iter().enumerate().filter(|(i, _)| *i != most).flat_map(|(_, (_, items))| items.iter().copied()).collect();
                let shown: Vec<String> = items.iter().map(|&(r, h)| ids::shown(content, r, h)).collect();
                let text = format!("mixes the versions {} ({})", groups.join(" and "), shown.join(", "));
                out.push(Problem { side: Some(0), field: None, entry: None, what: Wrong::Mixed { groups, items }, text });
            }
        }
        out
    }

    /// What it doesn't allow, by name: each category with anything held
    /// back, by its fact's name, with the names of its definitions in the
    /// game (a chip's `cannon`, an entry's in its collection), in handle
    /// order. What a file and the lobby's wire state.
    pub fn banned_names(&self, content: &Content) -> Vec<(String, Vec<String>)> {
        self.banned
            .iter()
            .filter_map(|(fact, banned)| {
                let (registry, _, _) = definitions_of(content, fact)?;
                Some((fact.clone(), banned.iter().filter_map(|&h| ids::name_of(content, registry, h).map(str::to_string)).collect()))
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
                let h = ids::handle_of(content, &rulesets.game, category.registry, category.of.as_deref(), name).ok_or_else(|| format!("{fact}: {} has no {:?}", rulesets.game, name))?;
                out.set_allowed(fact, h, false);
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
            let items = category_items(content, game, &builds, fact, registry, of.as_deref())?;
            categories.push(Category { fact: fact.clone(), registry, of, items });
        }
        let groups = groups(content, game, &builds)?;
        let mut out = Rulesets { game: game.to_string(), categories, groups, presets: Vec::new() };
        for p in items(data.field("presets")) {
            let Data::Str(id) = p.field("id") else { return Err(format!("{module}: a preset has its `id`")) };
            let mut ruleset = Ruleset::open(Name::Preset(id.clone()));
            ruleset.one_per_build = match p.field("versions") {
                Data::Str(v) if v == "one_per_build" => true,
                Data::Str(v) if v == "mixed" => false,
                _ => return Err(format!("{module}: preset {id}: its `versions` is \"one_per_build\" or \"mixed\"")),
            };
            let groups: Vec<&str> = items(p.field("groups")).iter().filter_map(Data::str).collect();
            if let Some(g) = groups.iter().find(|g| out.group(g).is_none()) {
                return Err(format!("{module}: preset {id}: no group {g:?}"));
            }
            if !groups.is_empty() {
                for category in out.categories.clone() {
                    ruleset.allow_groups(&out, &category, &groups);
                }
            }
            if let Data::Map(only) = p.field("only") {
                for (fact, allowed) in only {
                    let DataKey::Str(fact) = fact else { continue };
                    let category = out.category(fact).ok_or_else(|| format!("{module}: preset {id}: no category {fact:?}"))?.clone();
                    let allowed: Vec<u16> = self::items(allowed)
                        .iter()
                        .map(|n| n.str().and_then(|n| ids::handle_of(content, game, category.registry, category.of.as_deref(), n)).ok_or_else(|| format!("{module}: preset {id}: {fact}: {n:?} is none of its items")))
                        .collect::<Result<_, _>>()?;
                    ruleset.allow_only(&category, &allowed);
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
    pub fn group_items(&self, category: &Category, group: &Group) -> Vec<u16> {
        category.items.iter().copied().filter(|&h| group.has(category.registry, h)).collect()
    }

    /// The group definition `h` of `registry` is, if it is one's.
    pub fn group_of(&self, registry: Registry, h: u16) -> Option<&Group> {
        self.groups.iter().find(|g| g.has(registry, h))
    }

    /// The groups with items in `category`, in order.
    pub fn groups_in(&self, category: &Category) -> Vec<&Group> {
        self.groups.iter().filter(|g| !self.group_items(category, g).is_empty()).collect()
    }

    /// The preset named `id`.
    pub fn preset(&self, id: &str) -> Option<&Ruleset> {
        self.presets.iter().find(|p| p.name == Name::Preset(id.to_string()))
    }

    /// The first preset: a player's until they pick another.
    pub fn first(&self) -> &Ruleset {
        &self.presets[0]
    }
}

/// The game's groups (docs/design/versions.md §2.2): each version of its
/// builds data, with the chips and forms whose own `version` names it, and
/// the definitions the version states that no other version does.
fn groups(content: &Content, game: &str, builds: &Builds) -> Result<Vec<Group>, String> {
    let mut groups: Vec<Group> = builds.versions.iter().map(|v| Group { id: v.id.clone(), color: v.color, members: BTreeSet::new() }).collect();
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
    // (What each version states, by fact: a definition stated by one alone
    // is that one's; by several, none's.)
    let mut stated: BTreeMap<(Registry, u16), Vec<usize>> = BTreeMap::new();
    for (i, v) in builds.versions.iter().enumerate() {
        for (fact, names) in v.stated_names() {
            let Some((registry, of, _)) = definitions_of(content, fact) else { continue };
            for name in names {
                let h = ids::handle_of(content, game, registry, of.as_deref(), name).ok_or_else(|| format!("{game}'s version {}: {fact}: no {name:?}", v.id))?;
                stated.entry((registry, h)).or_default().push(i);
            }
        }
    }
    for (def, by) in stated {
        if let [only] = by[..] {
            groups[only].members.insert(def);
        }
    }
    Ok(groups)
}

/// A category's items: the definitions of `registry` a build of `game` can
/// hold in fact `fact`.
fn category_items(content: &Arc<Content>, game: &str, builds: &Builds, fact: &str, registry: Registry, of: Option<&str>) -> Result<Vec<u16>, String> {
    Ok(match registry {
        Registry::Navi => crate::navis(content, game).into_iter().map(|n| n.0).collect(),
        // (The chips a folder lists: every chip of the game but those no
        // folder holds, as the folder rules' view of a new build says.)
        Registry::Chip => {
            let side = builds.new_build(content, &builds.versions[0].id)?;
            let mut checked = crate::build::Checked::of(content, game, &side);
            let view = checked.folder_view(content, &crate::Folder::EMPTY).ok_or_else(|| format!("{game}'s rules have no folder view"))?;
            (0..content.defs.chips.len() as u16).filter(|&h| ids::in_game(content, game, &content.defs.chip(ChipHandle(h)).key) && view.lists(ChipHandle(h))).collect()
        }
        Registry::Form if content.defs.fact_name(PlayerFact::FormList) == Some(fact) => crate::listed_forms(content, game)?.into_iter().map(|f| f.0).collect(),
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
    use crate::testing::{exe4_content, exe5_content, exe6_content};

    /// Each game's categories, its groups and what they hold, and its
    /// presets, as its data says them.
    #[test]
    fn each_game_has_its_categories_groups_and_presets() {
        for (content, game, facts, forms, groups) in [
            (exe6_content(), "exe6", vec!["navi", "folder", "navicust_programs", "patch_cards", "crosses"], "crosses", ["gregar", "falzar"]),
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
                assert!(r.group_of(Registry::Chip, h).is_none(), "{game} {both}");
            }
            // The presets: Standard, one version per build; Mixed; each group alone; no patch cards.
            let names: Vec<Name> = r.presets.iter().map(|p| p.name.clone()).collect();
            let expected: Vec<Name> = ["standard", "mixed", groups[0], groups[1], "no_patch_cards"].iter().map(|n| Name::Preset(n.to_string())).collect();
            assert_eq!(names, expected, "{game}");
            assert!(!r.first().bans_anything() && r.first().one_per_build);
            assert!(!r.preset("mixed").unwrap().one_per_build);
            let one = r.preset(groups[0]).unwrap();
            assert_eq!(one.groups_chosen(&r, category), Some(vec![groups[0]]));
            assert_eq!(one.groups_chosen(&r, chips), Some(vec![groups[0]]));
            assert_eq!(r.first().groups_chosen(&r, category), Some(groups.to_vec()));
            let cards = r.category("patch_cards").unwrap();
            assert_eq!(r.preset("no_patch_cards").unwrap().allowed_count(cards), 0);
            // A new build of each version is one group's, and plays under Standard and its own version's preset.
            let builds = Builds::of(&content, game).unwrap();
            for (v, other) in [(groups[0], groups[1]), (groups[1], groups[0])] {
                let side = builds.new_build(&content, v).unwrap();
                assert!(Checked::under(&content, game, &side, &r, r.first()).ready(), "{game} {v}");
                assert!(Checked::under(&content, game, &side, &r, r.preset(v).unwrap()).ready(), "{game} {v}");
                let problems = Checked::under(&content, game, &side, &r, r.preset(other).unwrap()).problems;
                assert!(!problems.is_empty() && problems.iter().all(|p| p.field.as_deref() == Some(forms)), "{game} {v}");
            }
        }
    }

    /// A build under a ruleset: each definition it holds that the ruleset
    /// doesn't allow is a problem at its first entry, of its fact; as names
    /// a ruleset reads back the same.
    #[test]
    fn a_build_holding_what_a_ruleset_bans_has_a_problem_for_each() {
        let content = exe6_content();
        let r = Rulesets::of(&content, "exe6").unwrap();
        let builds = Builds::of(&content, "exe6").unwrap();
        let gregar = builds.new_build(&content, "gregar").unwrap();
        // Falzar's preset: Gregar's five Crosses aren't allowed.
        let problems = Checked::under(&content, "exe6", &gregar, &r, r.preset("falzar").unwrap()).problems;
        assert_eq!(problems.len(), 5);
        assert!(problems.iter().all(|p| p.field.as_deref() == Some("crosses") && matches!(p.what, Wrong::NotAllowed { registry: Registry::Form, .. })));
        assert_eq!(problems.iter().map(|p| p.entry).collect::<Vec<_>>(), (0..5).map(Some).collect::<Vec<_>>());
        // A chip held back: one problem however many copies, at the first.
        let cannon = crate::ids::chip(&content, "exe6", "cannon").unwrap();
        let mut no_cannon = Ruleset::open(Name::Own("No Cannon".into()));
        no_cannon.set_allowed("folder", cannon.0, false);
        let problems = no_cannon.problems(&r, &content, &gregar);
        assert_eq!(problems.len(), 1);
        assert_eq!((problems[0].entry, &problems[0].what), (Some(0), &Wrong::NotAllowed { registry: Registry::Chip, def: cannon.0 }));
        // As names, and back.
        let names = no_cannon.banned_names(&content);
        assert_eq!(names, vec![("folder".to_string(), vec!["cannon".to_string()])]);
        assert_eq!(Ruleset::of_names(&r, &content, no_cannon.name.clone(), false, &names).unwrap(), no_cannon);
        assert!(Ruleset::of_names(&r, &content, no_cannon.name.clone(), false, &[("hp".into(), vec![])]).is_err());
        assert!(Ruleset::of_names(&r, &content, no_cannon.name.clone(), false, &[("folder".into(), vec!["nothing".into()])]).is_err());
        // Allowed again, it holds nothing back, and equals an open one.
        no_cannon.set_allowed("folder", cannon.0, true);
        assert_eq!(no_cannon, Ruleset::open(Name::Own("No Cannon".into())));
        // Patch cards none: a build holding one has its problem, at its entry.
        let card = crate::ids::entry(&content, "exe6", crate::facts::PATCH_CARDS, "starman").unwrap();
        let mut carded = gregar.clone();
        carded.set_fact(&content, crate::facts::PATCH_CARDS, &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Def(Registry::Entry, card.0))]).unwrap();
        let problems = r.preset("no_patch_cards").unwrap().problems(&r, &content, &carded);
        assert_eq!(problems.len(), 1);
        assert_eq!((problems[0].field.as_deref(), problems[0].entry), (Some("patch_cards"), Some(0)));
    }

    /// With one version per build, a build holding two groups' things is a
    /// problem that lists what is the other group's, read from what it holds
    /// (its version fact says nothing here); mixed lets it be.
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
        let mixed: Vec<nettai_battle::rules::Fact> =
            gregar[..3].iter().chain(&falzar[..2]).map(|&h| nettai_battle::rules::Fact::Value(nettai_content_api::Value::Def(Registry::Form, h))).collect();
        side.set_fact(&content, "crosses", &mixed).unwrap();
        let problems = r.first().problems(&r, &content, &side);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].what, Wrong::Mixed { groups: vec!["gregar".into(), "falzar".into()], items: falzar[..2].iter().map(|&h| (Registry::Form, h)).collect() });
        assert!(r.preset("mixed").unwrap().problems(&r, &content, &side).is_empty());
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
        ruleset.allow_only(navis, &navis.items.iter().copied().filter(|&n| n != navi.0).collect::<Vec<_>>());
        let problems = ruleset.problems(&r, &content, &side);
        assert_eq!(problems.len(), 1);
        assert_eq!((problems[0].field.as_deref(), problems[0].entry, &problems[0].what), (Some("navi"), None, &Wrong::NotAllowed { registry: Registry::Navi, def: navi.0 }));
    }
}
