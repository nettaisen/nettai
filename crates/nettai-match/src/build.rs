//! A build: one player's side of a match of a game, made and kept to be
//! played (nettai's build screens). What those screens ask of the game is
//! here, with no screen's code and no word a player reads:
//!
//! - **The game's builds** ([`Builds`], read from the game's own data, its
//!   rules/builds module): the versions a build is one of, in the order
//!   every list shows them, and what each sets of a side; a new build of a
//!   version ([`Builds::new_build`]: the game's navi that changes form, the
//!   version, the folder a new save of the game starts with); and the facts
//!   no build states, kept at the rules' defaults ([`Builds::as_built`]:
//!   every build plays at the most its game allows).
//! - **A build checked** ([`Checked`]): what is wrong with it, typed
//!   ([`crate::Wrong`]), and its round set up, which the other questions are
//!   asked of: the folder's view ([`Checked::folder_view`]) and the overview
//!   ([`Checked::overview`]).
//! - **The overview** ([`Overview`]): each number a round starts the navi
//!   with, as the navi alone has it and as the build's installed entries
//!   leave it (the NaviCust's programs, the patch cards); what gives the
//!   navi its abilities; its bugs, each by its name's key in the game's
//!   strings and its level; and the budgets of the build's lists (the patch
//!   cards' MB).
//! - **A chip, for a list** ([`ChipInfo`]): its family (whose icon the game
//!   shows as its element), class, MB, attack, codes and place in the
//!   library's order; rows of a chip in a code, sorted ([`sorted`]).
//!
//! This module names no game and no game's fact: the versions, the facts
//! they set and the folder are the game's data; the numbers are the engine's
//! stats; a name is a key into the content's strings.

use crate::check::{self, Problem};
use crate::facts::{self, Stated};
use crate::folders::{self, View};
use crate::{Count, Folder, Match, Side, Wrong, ids};
use nettai_battle::Battle;
use nettai_battle::content::{ChipClass, ChipCode, ChipFamily, ChipFlags, Content, Element, PlayerFact};
use nettai_battle::custom::FolderChip;
use nettai_battle::custom::folder::FOLDER_SIZE;
use nettai_battle::rules::Fact;
use nettai_battle::setup::NaviStats;
use nettai_content_api::{ChipHandle, Data, DataKey, EntryHandle, FieldType, Registry, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// A version of a game a build is one of (EXE6's Gregar, EXE5's Team
/// ProtoMan, EXE4's Red Sun): its id, by which a screen words it, and the
/// facts it sets of a side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub id: String,
    set: Data,
}

/// What a game says of its builds (its rules/builds module's data).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Builds {
    game: String,
    /// The versions, in the order every list shows them.
    pub versions: Vec<Version>,
    at_defaults: Vec<String>,
    new_set: Data,
    /// The folder a new save of the game starts with.
    pub new_folder: Folder,
}

/// The module of a game's rules that holds its builds' data.
const MODULE: &str = "rules/builds";

/// A table's fields, by name (none of another kind of value).
fn fields(d: &Data) -> Vec<(&str, &Data)> {
    match d {
        Data::Map(entries) => entries.iter().filter_map(|(k, v)| if let DataKey::Str(k) = k { Some((k.as_str(), v)) } else { None }).collect(),
        _ => Vec::new(),
    }
}

/// A list's items (none of another kind of value).
fn items(d: &Data) -> &[Data] {
    match d {
        Data::List(items) => items,
        _ => &[],
    }
}

/// `value` (a build's datum: a number, a flag, a variant's or a definition's
/// name, a list of definitions' names) as the facts to state of `field`.
fn facts_of<'v>(content: &Content, game: &str, field: &facts::Field, value: &'v Data) -> Result<Vec<Fact<'v>>, String> {
    let element = match field.ty {
        FieldType::List(e, _) | FieldType::Array(e, _) => e,
        ty => ty,
    };
    let one = |v: &'v Data| -> Result<Fact<'v>, String> {
        Ok(match (element, v) {
            (_, Data::Int(n)) => Fact::Value(Value::Int(*n)),
            (_, Data::Bool(b)) => Fact::Value(Value::Bool(*b)),
            (FieldType::Ref(registry, of), Data::Str(name)) => {
                let h = ids::handle_of(content, game, *registry, of.as_deref(), name).ok_or_else(|| format!("{}: {game} has no {registry} {name:?}", field.name))?;
                Fact::Value(Value::Def(*registry, h))
            }
            (_, Data::Str(name)) => Fact::Name(name),
            (_, other) => return Err(format!("{}: {other:?} is no value of a fact", field.name)),
        })
    };
    match value {
        Data::List(list) => list.iter().map(one).collect(),
        // (An empty Luau table reads as a table of no fields: an empty list.)
        Data::Map(m) if m.is_empty() => Ok(Vec::new()),
        v => Ok(vec![one(v)?]),
    }
}

/// State each fact of `set` (a table by the facts' names) of `side`.
fn state(content: &Content, game: &str, side: &mut Side, set: &Data) -> Result<(), String> {
    for (name, value) in fields(set) {
        let field = facts::field(content, name).ok_or_else(|| facts::no_field(content, name))?;
        side.set_fact(content, name, &facts_of(content, game, &field, value)?)?;
    }
    Ok(())
}

impl Builds {
    /// `game`'s builds, read from its rules/builds module as the content
    /// loaded it; an error names what the module lacks (or that the game
    /// has none).
    pub fn of(content: &Content, game: &str) -> Result<Builds, String> {
        crate::playable(content, game)?;
        let module = format!("{game}:{MODULE}");
        let data = nettai_battle::behavior::content_module_data(content, &module).ok_or_else(|| format!("{game} says nothing of its builds (no module {MODULE})"))??;
        let versions: Vec<Version> = items(data.field("versions"))
            .iter()
            .map(|v| match v.field("id") {
                Data::Str(id) => Ok(Version { id: id.clone(), set: v.field("set").clone() }),
                _ => Err(format!("{module}: a version has its `id`")),
            })
            .collect::<Result<_, _>>()?;
        if versions.is_empty() {
            return Err(format!("{module}: no `versions`"));
        }
        let at_defaults = items(data.field("at_defaults")).iter().filter_map(|f| if let Data::Str(f) = f { Some(f.clone()) } else { None }).collect();
        let new = data.field("new");
        let mut new_folder = Folder::EMPTY;
        let stated = items(new.field("folder"));
        if stated.len() != FOLDER_SIZE {
            return Err(format!("{module}: a new save's folder has {} chips, not {FOLDER_SIZE}", stated.len()));
        }
        for (slot, e) in new_folder.chips.iter_mut().zip(stated) {
            let (Data::Str(chip), Data::Str(code)) = (e.field("chip"), e.field("code")) else { return Err(format!("{module}: a folder's entry is a chip and its code")) };
            let id = ids::chip(content, game, chip).ok_or_else(|| format!("{module}: {game} has no chip {chip:?}"))?;
            let code = code.chars().next().and_then(ChipCode::from_letter).ok_or_else(|| format!("{module}: {code:?} is no chip code"))?;
            *slot = Some(FolderChip::new(id, code));
        }
        Ok(Builds { game: game.to_string(), versions, at_defaults, new_set: new.field("set").clone(), new_folder })
    }

    /// The game these are the builds of.
    pub fn game(&self) -> &str {
        &self.game
    }

    /// The version named `id`.
    pub fn version(&self, id: &str) -> Option<&Version> {
        self.versions.iter().find(|v| v.id == id)
    }

    /// Make `side` a build of version `version`: the facts the version sets
    /// (its souls, its Giga chips, the game's own version fact), and the
    /// side's navi's own forms of it where the game's rules list a navi's
    /// forms by version (the engine's form list). An error: no such version,
    /// or a fact its game's rules don't take.
    pub fn take_version(&self, content: &Content, side: &mut Side, version: &str) -> Result<(), String> {
        let v = self.version(version).ok_or_else(|| {
            let known: Vec<&str> = self.versions.iter().map(|v| v.id.as_str()).collect();
            format!("{} has no version {version:?} (its are {})", self.game, known.join(", "))
        })?;
        let mut made = side.clone();
        state(content, &self.game, &mut made, &v.set)?;
        made.state_own_forms(content);
        *side = made;
        Ok(())
    }

    /// The version `side` is a build of: the first whose facts are all the
    /// side's; none where it is of none (a side no version made, a file's
    /// with another's mix).
    pub fn version_of(&self, content: &Content, side: &Side) -> Option<&str> {
        self.versions
            .iter()
            .find(|v| {
                let mut as_it = side.clone();
                state(content, &self.game, &mut as_it, &v.set).is_ok() && fields(&v.set).iter().all(|(name, _)| as_it.facts.get(content, name) == side.facts.get(content, name))
            })
            .map(|v| v.id.as_str())
    }

    /// Whether fact `name` is one no build states: always the rules'
    /// default (the game's `at_defaults`, and the facts of the engine's
    /// level and base HP roles; every build plays at the most its game
    /// allows).
    pub fn never_stated(&self, content: &Content, name: &str) -> bool {
        self.at_defaults.iter().any(|f| f == name) || matches!(facts::role_of(content, name), Some(PlayerFact::Level | PlayerFact::BaseHp))
    }

    /// Set `side` to what every build is: the facts no build states back at
    /// the rules' defaults, and a navi that must have a level at its last.
    /// The facts that changed, by name (a save's import says them).
    pub fn as_built(&self, content: &Content, side: &mut Side) -> Vec<String> {
        let before = side.clone();
        for f in facts::fields(content) {
            if self.never_stated(content, f.name) {
                side.facts.reset(content, f.name);
            }
        }
        if let Some(navi) = side.stated_navi(content)
            && content.defs.fact_name(PlayerFact::Level).is_some()
        {
            let _ = side.set_level(content, crate::play_level(content, navi));
        }
        facts::fields(content).into_iter().filter(|f| before.facts.get(content, f.name) != side.facts.get(content, f.name)).map(|f| f.name.to_string()).collect()
    }

    /// A new build of version `version`: the game's navi that changes form
    /// (MegaMan), the version, the folder a new save of the game starts
    /// with (no Regular or tag chip, as a new save's), what the game says a
    /// new build states besides, and every other fact the rules' default
    /// (an empty NaviCust on the largest board, no patch cards). It keeps
    /// its game's rules ([`Checked::of`] finds nothing wrong with it).
    pub fn new_build(&self, content: &Content, version: &str) -> Result<Side, String> {
        let mut side = Side::fresh(content, &self.game)?;
        state(content, &self.game, &mut side, &self.new_set)?;
        self.take_version(content, &mut side, version)?;
        side.set_folder(content, &self.new_folder)?;
        self.as_built(content, &mut side);
        Ok(side)
    }

    /// The chips only some versions' saves hold, each with those versions
    /// (ids, in the versions' order): what a screen says of a chip the
    /// folder's rules refuse as another version's ("Gregar only"). Asked of
    /// the rules, a new build of each version at a time.
    pub fn version_chips(&self, content: &Arc<Content>) -> Result<HashMap<ChipHandle, Vec<String>>, String> {
        let mut holders: HashMap<ChipHandle, Vec<String>> = HashMap::new();
        let mut exclusive: Vec<ChipHandle> = Vec::new();
        for v in &self.versions {
            let side = self.new_build(content, &v.id)?;
            let mut checked = Checked::of(content, &self.game, &side);
            let view = checked.folder_view(content, &Folder::EMPTY).ok_or_else(|| format!("{}'s rules give no view of a folder", self.game))?;
            for id in (0..content.defs.chips.len() as u16).map(ChipHandle).filter(|&id| view.lists(id)) {
                match view.refusal(id) {
                    Some(Wrong::ChipOfAnotherVersion { .. }) => exclusive.push(id),
                    _ => holders.entry(id).or_default().push(v.id.clone()),
                }
            }
        }
        holders.retain(|id, _| exclusive.contains(id));
        Ok(holders)
    }
}

/// A build checked: what is wrong with it, and its round set up (the side on
/// both sides of a match of its game), which the folder's view and the
/// overview are asked of. Made again after each edit.
pub struct Checked {
    game: String,
    /// What is wrong with the build, typed; empty: it can be played.
    pub problems: Vec<Problem>,
    battle: Option<Battle>,
}

impl Checked {
    /// `side` checked as a build of `game`: every problem nettai-match's
    /// checks and the game's rules find with it.
    pub fn of(content: &Arc<Content>, game: &str, side: &Side) -> Checked {
        let none = |problems: Vec<Problem>| Checked { game: game.to_string(), problems, battle: None };
        let m = match Match::empty(content, game) {
            Ok(m) => Match { sides: [side.clone(), side.clone()], ..m },
            Err(e) => return none(vec![Problem { side: None, field: None, entry: None, what: Wrong::Match, text: e }]),
        };
        let alone: Vec<Problem> = check::side_alone(content, game, side).into_iter().map(|p| Problem { side: Some(0), ..p }).collect();
        if !alone.is_empty() {
            return none(alone);
        }
        match check::start(content, &m) {
            Ok(mut b) => Checked { game: game.to_string(), problems: check::validated(&mut b, 0, side), battle: Some(b) },
            Err(e) => none(vec![Problem { side: Some(0), field: None, entry: None, what: Wrong::Internal, text: e }]),
        }
    }

    /// Whether the build can be played.
    pub fn ready(&self) -> bool {
        self.problems.is_empty()
    }

    /// The stats the build's round starts its navi with (none: the round
    /// doesn't start).
    pub fn stats(&self) -> Option<&NaviStats> {
        self.battle.as_ref().map(|b| &b.stats[0])
    }

    /// The folder rules' view of `folder` on the build's limits
    /// ([`View::of`]): asked once for a folder as it stands, then read for
    /// every row of a list. None: the round doesn't start.
    pub fn folder_view(&mut self, content: &Content, folder: &Folder) -> Option<View> {
        View::of(content, &self.game, self.battle.as_mut()?, 0, folder)
    }

    /// What the rules say of `folder`'s Regular and tag chips
    /// ([`folders::mark_problems`]): a folder editor asks it of the folder
    /// with a mark as it would be.
    pub fn mark_problems(&mut self, folder: &Folder) -> Vec<Problem> {
        self.battle.as_mut().map(|b| folders::mark_problems(b, 0, folder)).unwrap_or_default()
    }

    /// The build's overview. None: the round doesn't start.
    pub fn overview(&mut self, content: &Arc<Content>, side: &Side) -> Option<Overview> {
        let b = self.battle.as_mut()?;
        let with = b.stats[0];
        // The navi alone: the side with nothing installed.
        let alone = Match::empty(content, &self.game).ok().and_then(|m| {
            let bare = without_entries(content, side);
            check::start(content, &Match { sides: [bare.clone(), bare], ..m }).ok().map(|b| b.stats[0])
        });
        let alone = alone.unwrap_or(with);
        let stat = |of: fn(&NaviStats) -> i32| Stat { base: of(&alone), value: of(&with) };
        let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.overview(0))).ok().flatten().unwrap_or(Data::Nil);
        let entry = |d: &Data| match d.field("entry") {
            Data::Ref(Registry::Entry, key) => content.defs.entries.iter().position(|e| e.key == *key).map(|i| EntryHandle(i as u16)),
            _ => None,
        };
        let source = |d: &Data| {
            let e = entry(d)?;
            Some(match d.field("effect").int() {
                Some(n) if n >= 1 => Source::Effect(e, n as usize - 1),
                _ => Source::Entry(e),
            })
        };
        let bugs = items(answer.field("bugs"))
            .iter()
            .filter_map(|d| match (d.field("name"), d.field("level").int()) {
                (Data::Str(key), Some(level)) => Some(Bug::Named { key: key.clone(), level: level.clamp(0, 255) as u8 }),
                _ => source(d).map(Bug::From),
            })
            .collect();
        let lists = items(answer.field("lists"))
            .iter()
            .filter_map(|d| {
                let (Data::Str(field), Data::Str(cost)) = (d.field("field"), d.field("cost")) else { return None };
                let count = Count { used: d.field("used").int()?.try_into().ok()?, limit: d.field("limit").int()?.try_into().ok()? };
                Some(Budget { field: field.clone(), cost: cost.clone(), count, once: matches!(d.field("once"), Data::Bool(true)) })
            })
            .collect();
        let tag_memory = self.folder_view(content, &side.folder(content)).and_then(|v| v.tag_memory);
        Some(Overview {
            hp: stat(|s| s.max_hp as i32),
            attack: stat(|s| s.attack as i32 + 1),
            rapid: stat(|s| s.rapid as i32 + 1),
            charge: stat(|s| s.charge as i32 + 1),
            custom: stat(|s| s.custom_level as i32),
            mega: stat(|s| s.mega_level as i32),
            giga: stat(|s| s.giga_level as i32),
            regular_memory: stat(|s| s.reg_up as i32),
            tag_memory,
            abilities: items(answer.field("abilities")).iter().filter_map(source).collect(),
            bugs,
            lists,
        })
    }
}

/// `side` with nothing installed: each of its lists of entries of the game's
/// collections (the NaviCust's programs, the patch cards) empty. What its
/// round starts the navi with is the navi's own, before what the build
/// installs.
fn without_entries(content: &Content, side: &Side) -> Side {
    let holds_entries = |ty: &FieldType| match ty {
        FieldType::Ref(Registry::Entry, _) => true,
        FieldType::Record(fields) => fields.fields().iter().any(|f| matches!(f.ty, FieldType::Ref(Registry::Entry, _))),
        _ => false,
    };
    let mut bare = side.clone();
    for f in facts::fields(content) {
        if let FieldType::List(elem, _) | FieldType::Array(elem, _) = f.ty
            && holds_entries(elem)
        {
            let _ = bare.set_fact(content, f.name, &[]);
        }
    }
    bare
}

/// A number a round starts the navi with: as the navi alone has it (`base`),
/// and as the build leaves it (`value`), what its installed entries added
/// the difference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stat {
    pub base: i32,
    pub value: i32,
}

impl Stat {
    /// What the build's installed entries added (or took).
    pub fn added(self) -> i32 {
        self.value - self.base
    }
}

/// What gives a navi an ability or a bug, of the entries a build installs:
/// an entry of one of the game's collections (a NaviCust program at work: a
/// screen shows its name), or one effect of an entry that lists its effects
/// (a patch card's line: its place in the entry's `effects`, from 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    Entry(EntryHandle),
    Effect(EntryHandle, usize),
}

/// A bug of a build's navi: by its name's key in the game's strings' `bugs`
/// table at its level (the NaviCust's, 1 to 3), or as an installed entry's
/// effect (a patch card's line, which the card shows as a bug).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bug {
    Named { key: String, level: u8 },
    From(Source),
}

/// The strings' text table a named bug's key is of.
pub const BUGS_TABLE: &str = "bugs";

/// A list of a build whose entries cost against a budget (the patch cards'
/// MB): the setup field, the field of an entry's data that is its cost,
/// what the list's entries cost together against the most they may, and
/// whether an entry goes into it once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Budget {
    pub field: String,
    pub cost: String,
    pub count: Count,
    pub once: bool,
}

/// Why an entry can't go into a list of a build now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryRefusal {
    /// The list holds it, and holds an entry once.
    Installed,
    /// It costs `needs`, and the list has `free` left.
    Needs { needs: u32, free: u32 },
}

impl Budget {
    /// What `entry` costs (its data's cost field; none: it costs nothing).
    pub fn cost_of(&self, content: &Content, entry: EntryHandle) -> u32 {
        let key = &content.defs.entry(entry).key;
        let cost = content.defs.definitions.get(Registry::Entry, key).and_then(|d| d.spec.field(&self.cost).int());
        cost.and_then(|n| u32::try_from(n).ok()).unwrap_or(0)
    }

    /// Why `entry` can't go into `side`'s list now; none: it can.
    pub fn refusal(&self, content: &Content, side: &Side, entry: EntryHandle) -> Option<EntryRefusal> {
        let held = side.facts.get(content, &self.field).map(|v| named(&v)).unwrap_or_default();
        if self.once && held.contains(&entry.0) {
            return Some(EntryRefusal::Installed);
        }
        let (needs, free) = (self.cost_of(content, entry), self.count.free());
        (needs > free).then_some(EntryRefusal::Needs { needs, free })
    }
}

/// The entries a list's value names, at any depth.
fn named(value: &Stated) -> Vec<u16> {
    match value {
        Stated::Def(Registry::Entry, Some(h)) => vec![*h],
        Stated::List(items) => items.iter().flat_map(named).collect(),
        Stated::Record(fields) => fields.iter().flat_map(|(_, v)| named(v)).collect(),
        _ => Vec::new(),
    }
}

/// What a build comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overview {
    /// The navi's HP.
    pub hp: Stat,
    /// The buster's attack, rapid and charge, as the games show them (1 to
    /// 5; the stats hold one less).
    pub attack: Stat,
    pub rapid: Stat,
    pub charge: Stat,
    /// The custom screen's chips.
    pub custom: Stat,
    /// The folder's Mega and Giga chip limits.
    pub mega: Stat,
    pub giga: Stat,
    /// The Regular memory, in MB.
    pub regular_memory: Stat,
    /// The tag memory, in MB, in a game whose folders have tag chips.
    pub tag_memory: Option<u32>,
    /// What gives the navi its abilities.
    pub abilities: Vec<Source>,
    /// The navi's bugs.
    pub bugs: Vec<Bug>,
    /// The budgets of the build's lists (the patch cards' MB).
    pub lists: Vec<Budget>,
}

impl Overview {
    /// The budget of the build's list `field`, if it has one.
    pub fn budget(&self, field: &str) -> Option<&Budget> {
        self.lists.iter().find(|b| b.field == field)
    }
}

/// A mark of a folder's entry: its Regular chip, or one of its two tag
/// chips, in their order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    Regular,
    Tag1,
    Tag2,
}

/// The mark of `folder`'s entry `entry`, if it has one.
pub fn mark(folder: &Folder, entry: usize) -> Option<Mark> {
    if folder.regular.is_some_and(|r| r as usize == entry) {
        return Some(Mark::Regular);
    }
    match folder.tags {
        Some((a, _)) if a as usize == entry => Some(Mark::Tag1),
        Some((_, b)) if b as usize == entry => Some(Mark::Tag2),
        _ => None,
    }
}

/// What a list shows of a chip: its family (the one whose icon the game
/// shows as the chip's element: its number, by which the pack's icons are,
/// and its name in the game's rules), its element, its class and whether it
/// is a dark chip, its MB, its attack (0: none, or one the chip works out),
/// the codes it comes in, and its place in the game's library's order (the
/// section and the place in it; none: no library lists it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChipInfo {
    pub chip: ChipHandle,
    pub family: ChipFamily,
    pub family_name: Option<String>,
    pub element: Element,
    pub class: ChipClass,
    pub dark: bool,
    pub mb: u8,
    pub attack: u16,
    pub codes: Vec<ChipCode>,
    pub library: Option<(usize, usize)>,
}

/// A place in a library's order by a chip's key (a game's library.toml:
/// `nettai_content::library::Library::chip_rank`).
pub type LibraryRank<'a> = &'a dyn Fn(&str) -> Option<(usize, usize)>;

/// What a list shows of `chip`.
pub fn chip_info(content: &Content, chip: ChipHandle, rank: LibraryRank) -> ChipInfo {
    let record = content.chip(chip);
    ChipInfo {
        chip,
        family: record.family,
        family_name: content.rules().chip_families.name(record.family).map(str::to_string),
        element: record.element,
        class: record.class,
        dark: record.flags.has(ChipFlags::DARK),
        mb: record.mb,
        attack: record.damage,
        codes: record.codes.clone(),
        library: rank(ids::local(&content.defs.chip(chip).key)),
    }
}

/// The chips a folder's library lists for a side (`view`: every chip of the
/// game but those no folder holds; another version's is listed, and
/// refused), in the library's order, those no library lists after, by key.
pub fn library(content: &Content, game: &str, view: &View, rank: LibraryRank) -> Vec<ChipInfo> {
    let mut out: Vec<ChipInfo> = (0..content.defs.chips.len() as u16)
        .map(ChipHandle)
        .filter(|&id| ids::in_game(content, game, &content.defs.chip(id).key) && view.lists(id))
        .map(|id| chip_info(content, id, rank))
        .collect();
    out.sort_by(|a, b| by_library(content, a, b));
    out
}

fn by_library(content: &Content, a: &ChipInfo, b: &ChipInfo) -> std::cmp::Ordering {
    let key = |c: &ChipInfo| (c.library.is_none(), c.library, content.defs.chip(c.chip).key.clone());
    key(a).cmp(&key(b))
}

/// What a library's rows are sorted by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sort {
    /// The game's library's order.
    Library,
    /// The chips' names, as the caller gives them (in its language).
    Name,
    /// The code: A to Z, then `*`.
    Code,
    Mb,
    /// The family, by its number (the games' element icons' order).
    Element,
    Attack,
}

/// A library's rows: each chip of `chips` in each of its codes, sorted by
/// `sort` (`descending`: the other way), equal rows in the library's order
/// and then by code. `name`: a chip's name in the list's language (asked for
/// [`Sort::Name`] alone).
pub fn sorted(content: &Content, chips: &[ChipInfo], sort: Sort, descending: bool, name: &dyn Fn(ChipHandle) -> String) -> Vec<FolderChip> {
    let names: HashMap<ChipHandle, String> = if sort == Sort::Name { chips.iter().map(|c| (c.chip, name(c.chip).to_lowercase())).collect() } else { HashMap::new() };
    let mut rows: Vec<(&ChipInfo, ChipCode)> = chips.iter().flat_map(|c| c.codes.iter().map(move |&code| (c, code))).collect();
    rows.sort_by(|(a, x), (b, y)| {
        let first = match sort {
            Sort::Library => std::cmp::Ordering::Equal,
            Sort::Name => names[&a.chip].cmp(&names[&b.chip]),
            Sort::Code => x.0.cmp(&y.0),
            Sort::Mb => a.mb.cmp(&b.mb),
            Sort::Element => a.family.cmp(&b.family),
            Sort::Attack => a.attack.cmp(&b.attack),
        };
        let first = if descending { first.reverse() } else { first };
        first.then_with(|| by_library(content, a, b)).then_with(|| x.0.cmp(&y.0))
    });
    rows.into_iter().map(|(c, code)| FolderChip::new(c.chip, code)).collect()
}
