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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{PlacedProgram, exe4_content, exe5_content, exe6_content, patch_cards, program_colors, set_navicust};

    /// The three games, each with its versions in the order every list shows
    /// them.
    fn games() -> [(Arc<Content>, &'static str, [&'static str; 2]); 3] {
        [(exe4_content(), "exe4", ["redsun", "bluemoon"]), (exe5_content(), "exe5", ["protoman", "colonel"]), (exe6_content(), "exe6", ["gregar", "falzar"])]
    }

    /// Each game's versions are listed in the user's order, and a new build
    /// of each is MegaMan of that version with the folder a new save starts
    /// with, at the most its game allows; nothing is wrong with it.
    #[test]
    fn a_new_build_of_each_version_is_ready() {
        for (content, game, versions) in games() {
            let builds = Builds::of(&content, game).unwrap();
            assert_eq!(builds.versions.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(), versions, "{game}");
            assert_eq!(builds.new_folder.chips().count(), FOLDER_SIZE, "{game}");
            for version in versions {
                let side = builds.new_build(&content, version).unwrap();
                assert_eq!(Some(side.navi(&content)), crate::first_navi(&content, game), "{game} {version}");
                assert_eq!(builds.version_of(&content, &side), Some(version), "{game}");
                assert_eq!(side.folder(&content), builds.new_folder, "{game} {version}");
                let mut checked = Checked::of(&content, game, &side);
                assert_eq!(checked.problems, Vec::new(), "{game} {version}");
                assert!(checked.ready());
                let st = *checked.stats().unwrap();
                assert_eq!((st.max_hp, st.hp, st.reg_up, st.mega_level, st.giga_level), (1000, 1000, 50, 5, 1), "{game} {version}");
                // Nothing installed: nothing added, no ability, no bug.
                let o = checked.overview(&content, &side).unwrap();
                assert_eq!((o.hp, o.mega, o.giga, o.regular_memory), (Stat { base: 1000, value: 1000 }, Stat { base: 5, value: 5 }, Stat { base: 1, value: 1 }, Stat { base: 50, value: 50 }));
                assert_eq!((o.attack.value, o.rapid.value, o.charge.value), (1, 1, 1), "{game} {version}");
                assert_eq!((o.abilities.len(), o.bugs.len()), (0, 0), "{game} {version}");
                assert_eq!(o.tag_memory, (game == "exe6").then_some(60), "{game}");
                // Made again as built, it is the same: no fact to mend.
                let mut again = side.clone();
                assert_eq!(builds.as_built(&content, &mut again), Vec::<String>::new(), "{game} {version}");
            }
            assert!(builds.new_build(&content, "nowhere").unwrap_err().contains("no version"));
        }
    }

    /// A build of one version holds none of the other's Giga chips: the
    /// rules refuse one in its folder as another version's, and a build made
    /// the other version takes it.
    #[test]
    fn a_version_holds_its_own_giga_chips() {
        for (content, game, versions, own) in [
            (exe4_content(), "exe4", ["redsun", "bluemoon"], ["redsun", "bluemoon"]),
            (exe5_content(), "exe5", ["protoman", "colonel"], ["dethphnx", "phoenix"]),
            (exe6_content(), "exe6", ["gregar", "falzar"], ["colforce", "hubbatc"]),
        ] {
            let builds = Builds::of(&content, game).unwrap();
            let chip = |key: &str| ids::chip(&content, game, key).unwrap();
            for (v, version) in versions.iter().enumerate() {
                let mut side = builds.new_build(&content, version).unwrap();
                let mut folder = side.folder(&content);
                let theirs = chip(own[1 - v]);
                folder.chips[0] = Some(FolderChip::new(theirs, content.chip(theirs).codes[0]));
                side.set_folder(&content, &folder).unwrap();
                let checked = Checked::of(&content, game, &side);
                assert_eq!(checked.problems.len(), 1, "{game} {version}: {:?}", checked.problems);
                let p = &checked.problems[0];
                assert_eq!((&p.what, p.entry, p.field.as_deref()), (&Wrong::ChipOfAnotherVersion { chip: theirs }, Some(0), Some("folder")), "{game} {version}");
                // The other version, with the same folder, takes it.
                builds.take_version(&content, &mut side, versions[1 - v]).unwrap();
                assert_eq!(Checked::of(&content, game, &side).problems, Vec::new(), "{game} {version}");
                assert_eq!(builds.version_of(&content, &side), Some(versions[1 - v]));
            }
        }
    }

    /// EXE6's overview: what the NaviCust's programs and a patch card add to
    /// the navi's numbers, what gives it its abilities, its bugs by name and
    /// level, and the patch cards' MB.
    #[test]
    fn an_overview_says_what_a_build_comes_to() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let program = |name: &str| ids::entry(&content, "exe6", "navicust_programs", name).unwrap();
        let placed = |name: &str, color: &str, x: u8, y: u8| {
            let color = program_colors(&content, program(name)).iter().position(|c| *c == color).unwrap() as u8;
            PlacedProgram { program: program(name), color, x, y, rotation: 0, compressed: false }
        };
        let overview = |side: &Side| Checked::of(&content, "exe6", side).overview(&content, side).unwrap();
        // UnderSht on the command line, Attack+1 and HP+100 off it: no bug.
        let mut side = builds.new_build(&content, "falzar").unwrap();
        set_navicust(&content, &mut side, &[placed("undersht", "white", 1, 3), placed("attack-1", "pink", 5, 2), placed("hp-100", "white", 3, 2)], 2);
        let o = overview(&side);
        assert_eq!((o.hp, o.attack), (Stat { base: 1000, value: 1100 }, Stat { base: 1, value: 2 }));
        assert_eq!((o.hp.added(), o.attack.added(), o.mega.added()), (100, 1, 0));
        assert_eq!(o.abilities, [Source::Entry(program("undersht"))]);
        assert_eq!(o.bugs, Vec::new());
        // UnderSht off the command line: its bug at level 1, and no UnderSht.
        set_navicust(&content, &mut side, &[placed("undersht", "white", 1, 2)], 2);
        let o = overview(&side);
        assert_eq!(o.abilities, Vec::new());
        let [Bug::Named { key, level: 1 }] = &o.bugs[..] else { panic!("{:?}", o.bugs) };
        assert!(content.strings.text(BUGS_TABLE, key).is_some(), "{key}");
        // A patch card: its MB against the list's, its lines under the
        // abilities as its effects, and what it does to the numbers.
        set_navicust(&content, &mut side, &[], 2);
        side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", "canodumb")).unwrap();
        let canodumb = ids::entry(&content, "exe6", "patch_cards", "canodumb").unwrap();
        let o = overview(&side);
        let budget = o.budget("patch_cards").unwrap().clone();
        assert_eq!((budget.count, budget.once, budget.cost_of(&content, canodumb)), (Count { used: 10, limit: 80 }, true, 10));
        assert_eq!((o.hp, o.attack), (Stat { base: 1000, value: 960 }, Stat { base: 1, value: 2 }));
        assert!(matches!(o.abilities[..], [Source::Effect(card, _)] if card == canodumb), "{:?}", o.abilities);
        assert_eq!(budget.refusal(&content, &side, canodumb), Some(EntryRefusal::Installed));
        let other = ids::entry(&content, "exe6", "patch_cards", "amonicul").unwrap();
        assert_eq!(budget.refusal(&content, &side, other), None);
        let tight = Budget { count: Count { used: 70, limit: 80 }, ..budget };
        assert_eq!(tight.refusal(&content, &side, other), Some(EntryRefusal::Needs { needs: 16, free: 10 }));
    }

    /// Every bug a game's NaviCust can bring has its name in the game's
    /// strings (the `bugs` table, by the rules' name for the bug).
    #[test]
    fn every_bug_has_its_name() {
        for (content, game, _) in games() {
            let programs = nettai_battle::behavior::content_module_data(&content, &format!("{game}:rules/navicust/programs")).unwrap().unwrap();
            let bugs = items(programs.field("BUGS"));
            assert!(bugs.len() >= 10, "{game}");
            for bug in bugs {
                let Data::Str(name) = bug else { panic!("{bug:?}") };
                assert!(content.strings.text(BUGS_TABLE, name).is_some(), "{game}: no name for the bug {name}");
            }
        }
    }

    /// A chip's row, and a library's rows sorted.
    #[test]
    fn a_library_lists_and_sorts_its_chips() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let side = builds.new_build(&content, "gregar").unwrap();
        let view = Checked::of(&content, "exe6", &side).folder_view(&content, &side.folder(&content)).unwrap();
        // (A library's order for the test: three chips' places, the rest
        // none.)
        let rank = |key: &str| ["cannon", "hicannon", "m-cannon"].iter().position(|k| *k == key).map(|i| (0, i));
        let chip = |key: &str| ids::chip(&content, "exe6", key).unwrap();
        let cannon = chip_info(&content, chip("cannon"), &rank);
        assert_eq!((cannon.family_name.as_deref(), cannon.class, cannon.dark, cannon.attack, cannon.library), (Some("null"), ChipClass::Standard, false, 40, Some((0, 0))));
        assert_eq!(cannon.codes.iter().map(|c| c.letter()).collect::<String>(), "ABC*");
        assert_eq!(chip_info(&content, chip("firebrn1"), &rank).family_name.as_deref(), Some("fire"));
        let chips = library(&content, "exe6", &view, &rank);
        assert!(chips.len() > 250 && chips.iter().all(|c| view.lists(c.chip)));
        assert_eq!(chips[..3].iter().map(|c| c.chip).collect::<Vec<_>>(), [chip("cannon"), chip("hicannon"), chip("m-cannon")]);
        // The other version's Giga chips are listed (and refused).
        assert!(chips.iter().any(|c| c.chip == chip("falzar")) && matches!(view.refusal(chip("falzar")), Some(Wrong::ChipOfAnotherVersion { .. })));
        let name = |c: ChipHandle| crate::names::chip(&content, c).to_string();
        let rows = sorted(&content, &chips, Sort::Library, false, &name);
        assert_eq!(rows[..4].iter().map(|c| (c.id, c.code.letter())).collect::<Vec<_>>(), [(chip("cannon"), 'A'), (chip("cannon"), 'B'), (chip("cannon"), 'C'), (chip("cannon"), '*')]);
        assert_eq!(rows.len(), chips.iter().map(|c| c.codes.len()).sum::<usize>());
        let by = |sort: Sort, descending: bool| sorted(&content, &chips, sort, descending, &name);
        let mb = |c: &FolderChip| content.chip(c.id).mb;
        assert!(by(Sort::Mb, false).windows(2).all(|w| mb(&w[0]) <= mb(&w[1])));
        assert!(by(Sort::Mb, true).windows(2).all(|w| mb(&w[0]) >= mb(&w[1])));
        assert!(by(Sort::Attack, true).windows(2).all(|w| content.chip(w[0].id).damage >= content.chip(w[1].id).damage));
        assert!(by(Sort::Code, false).windows(2).all(|w| w[0].code.0 <= w[1].code.0));
        assert!(by(Sort::Element, false).windows(2).all(|w| content.chip(w[0].id).family <= content.chip(w[1].id).family));
        assert!(by(Sort::Name, false).windows(2).all(|w| name(w[0].id).to_lowercase() <= name(w[1].id).to_lowercase()));
        // The marks of a folder's entries.
        let folder = Folder { regular: Some(4), tags: Some((7, 2)), ..side.folder(&content) };
        assert_eq!([2, 4, 7, 9].map(|e| mark(&folder, e)), [Some(Mark::Tag2), Some(Mark::Regular), Some(Mark::Tag1), None]);
    }

    /// A build is set to what every build is: the facts no build states back
    /// at the rules' defaults, the rest as it was.
    #[test]
    fn a_build_is_as_built() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "gregar").unwrap();
        let before = side.clone();
        side.set_fact(&content, "hp", &[Fact::Value(Value::Int(600))]).unwrap();
        side.set_fact(&content, "sun", &[Fact::Value(Value::Bool(false))]).unwrap();
        assert!(builds.never_stated(&content, "hp") && !builds.never_stated(&content, "folder"));
        let mut changed = builds.as_built(&content, &mut side);
        changed.sort();
        assert_eq!(changed, ["hp", "sun"]);
        assert_eq!(side, before);
    }

    /// EXE4's overview (its own NaviCust and patch cards): a program at work
    /// is an ability's, a patch card's bug is counted with the NaviCust's
    /// and said by name and level, and its numbers are added.
    #[test]
    fn exe4s_overview_counts_its_cards_bugs() {
        let content = exe4_content();
        let builds = Builds::of(&content, "exe4").unwrap();
        let mut side = builds.new_build(&content, "bluemoon").unwrap();
        let undersht = ids::entry(&content, "exe4", "navicust_programs", "undersht").unwrap();
        let custom2 = ids::entry(&content, "exe4", "patch_cards", "017-custom2").unwrap();
        let entry = |h: EntryHandle| Fact::Value(Value::Def(Registry::Entry, h.0));
        let int = |n: i64| Fact::Value(Value::Int(n));
        // UnderSht on the command line (its two cells at (2, 2) and (2, 3)).
        let placed = Fact::Record(vec![
            ("program", entry(undersht)),
            ("color", Fact::Name("white")),
            ("x", int(2)),
            ("y", int(3)),
            ("rotation", int(0)),
            ("compressed", Fact::Value(Value::Bool(false))),
        ]);
        side.set_fact(&content, "navicust_programs", &[placed]).unwrap();
        // Custom2 (the custom level +2, and its custom bug once) in slot 3.
        side.set_fact(&content, "patch_cards", &[Fact::Record(vec![("card", entry(custom2)), ("slot", int(3))])]).unwrap();
        let mut checked = Checked::of(&content, "exe4", &side);
        assert_eq!(checked.problems, Vec::new());
        let o = checked.overview(&content, &side).unwrap();
        assert_eq!(o.custom.added(), 2);
        assert_eq!(o.abilities, [Source::Entry(undersht)]);
        assert_eq!(o.bugs, [Bug::Named { key: "custom".into(), level: 1 }]);
        assert_eq!((o.tag_memory, o.lists.len()), (None, 0));
        // Two cards in one slot: said of the second, typed.
        side.set_fact(&content, "patch_cards", &[Fact::Record(vec![("card", entry(custom2)), ("slot", int(3))]), Fact::Record(vec![("card", entry(custom2)), ("slot", int(3))])]).unwrap();
        let checked = Checked::of(&content, "exe4", &side);
        let got: Vec<(Wrong, Option<&str>, Option<usize>)> = checked.problems.iter().map(|p| (p.what.clone(), p.field.as_deref(), p.entry)).collect();
        assert_eq!(got, [(Wrong::Twice, Some("patch_cards"), Some(1))]);
    }

    /// EXE5's overview: a program at work, and its patch cards' MB.
    #[test]
    fn exe5s_overview() {
        let content = exe5_content();
        let builds = Builds::of(&content, "exe5").unwrap();
        let mut side = builds.new_build(&content, "colonel").unwrap();
        let undersht = ids::entry(&content, "exe5", "navicust_programs", "undersht").unwrap();
        set_navicust(&content, &mut side, &[PlacedProgram { program: undersht, color: 0, x: 1, y: 3, rotation: 0, compressed: false }], 2);
        let mut checked = Checked::of(&content, "exe5", &side);
        assert_eq!(checked.problems, Vec::new());
        let o = checked.overview(&content, &side).unwrap();
        assert_eq!((&o.abilities, &o.bugs), (&vec![Source::Entry(undersht)], &Vec::new()));
        assert_eq!(o.budget("patch_cards").map(|b| b.count), Some(Count { used: 0, limit: 80 }));
        // A program off the board: said of its entry, typed.
        set_navicust(&content, &mut side, &[PlacedProgram { program: undersht, color: 0, x: 6, y: 3, rotation: 0, compressed: false }], 2);
        let checked = Checked::of(&content, "exe5", &side);
        assert!(checked.problems.iter().any(|p| (&p.what, p.field.as_deref(), p.entry) == (&Wrong::OffBoard, Some("navicust_programs"), Some(0))), "{:?}", checked.problems);
    }
}
