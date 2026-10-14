//! A build: one player's side of a match of a game, made and kept to be
//! played (nettai's build screens). What those screens ask of the game is
//! here, with no screen's code and no word a player reads:
//!
//! - **The game's builds** ([`Builds`], read from the game's own data, its
//!   rules/builds module): the versions a build is one of, in the order
//!   every list shows them, and what each sets of a side; a new build of a
//!   version ([`Builds::new_build`]: the game's navi that changes form, the
//!   version, the folder a new save of the game starts with); a build's
//!   other choices, each of a few presets that set their facts together
//!   ([`Choice`]: EXE4's and EXE5's light or dark MegaMan; a navi the choice
//!   isn't of has none of it); a list whose entries go in slots ([`Slots`]:
//!   EXE4's patch cards, each in its own); and the facts no build states, kept at the
//!   rules' defaults ([`Builds::as_built`]: every build plays at the most
//!   its game allows).
//! - **A build checked** ([`Checked`]): what is wrong with it, typed
//!   ([`crate::Wrong`]: what a match's checks say of any side, and what the
//!   rules hold a build to besides, their `build_check`), and its round set
//!   up, which the other questions are
//!   asked of: the folder's view ([`Checked::folder_view`]), the overview
//!   ([`Checked::overview`]) and the face its navi starts the round with
//!   ([`Checked::face`]).
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
use nettai_content_api::{ChipHandle, Data, DataKey, EntryHandle, FieldType, FormHandle, NaviHandle, Registry, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// A version of a game a build is one of (EXE6's Gregar, EXE5's Team
/// ProtoMan, EXE4's Red Sun): its id, by which a screen words it (the game's
/// strings' text table [`TEXT_TABLE`] names it: [`Version::text_key`]), the
/// color a screen draws its tag in (0xRRGGBB: the version's own, Red Sun's
/// red and Blue Moon's blue), and the facts it sets of a side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub id: String,
    pub color: u32,
    set: Data,
}

impl Version {
    /// The key of the version's name in the game's strings' text table
    /// [`TEXT_TABLE`].
    pub fn text_key(&self) -> String {
        format!("version.{}", self.id)
    }

    /// Whether the version sets fact `name` of a side (a problem with that
    /// fact is the version's: EXE6's `version`, EXE4's and EXE5's souls).
    pub fn sets(&self, name: &str) -> bool {
        fields(&self.set).iter().any(|(n, _)| *n == name)
    }
}

/// The game's strings' text table that names what a build is made of: each
/// version (`version.<id>`) and each preset of each choice
/// (`<choice>.<preset>`).
pub const TEXT_TABLE: &str = "builds";

/// One of a choice's presets: its id, by which a screen words it, the least
/// value of the choice's fact that is this preset's (the game's line, where
/// a number tells the presets apart), and the facts it sets together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preset {
    pub id: String,
    from: Option<i64>,
    set: Data,
}

/// A choice a build makes beside its version, of a few presets, each of
/// which sets its facts together (EXE4's and EXE5's light or dark MegaMan:
/// the light/dark value and the base HP): its id, by which a screen words
/// it, and its presets in the order a screen offers them, the first a new
/// build's. A choice may be one navi's alone (`navi`): a build of another
/// navi has none of it, and its facts are the rules' defaults there. A build
/// is on a preset by the fact that tells them apart (`fact`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub id: String,
    pub presets: Vec<Preset>,
    navi: Option<NaviHandle>,
    fact: String,
}

impl Choice {
    /// The preset named `id`.
    pub fn preset(&self, id: &str) -> Option<&Preset> {
        self.presets.iter().find(|p| p.id == id)
    }

    /// The key of preset `preset`'s name in the game's strings' text table
    /// [`TEXT_TABLE`].
    pub fn text_key(&self, preset: &Preset) -> String {
        format!("{}.{}", self.id, preset.id)
    }

    /// The facts the choice's presets set, by name, each once.
    fn facts(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.presets.iter().flat_map(|p| fields(&p.set)).map(|(name, _)| name).collect();
        names.sort();
        names.dedup();
        names
    }
}

/// A preset a build was put on that changed it ([`Builds::settle`]): the
/// choice, the preset, and the value the choice's fact had (a number's: a
/// save's light/dark value of 100 is the dark preset's), for an import to
/// say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Taken {
    pub choice: String,
    pub preset: String,
    pub was: Option<i64>,
}

/// A list of a build whose entries go in slots (EXE4's patch cards: six
/// catalog slots, each shown under its label, a card belonging in one): the
/// setup field, and the slots' names by number, as the game shows them. An
/// entry's own slot is its definition's ([`Slots::own`]); a build keeps
/// each entry in its own (the rules' `build_check`: [`crate::Wrong::Slot`]),
/// so putting one in puts it there ([`Slots::put`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slots {
    pub field: String,
    pub names: Vec<String>,
    /// The fields of the list's records that hold the entry and its slot,
    /// and the field of an entry's data that says its own slot.
    entry: String,
    slot: String,
    own: String,
}

impl Slots {
    /// The slot `entry` belongs in (from 0), if its definition says one.
    pub fn own(&self, content: &Content, entry: EntryHandle) -> Option<usize> {
        let key = &content.defs.entry(entry).key;
        let own = content.defs.definitions.get(Registry::Entry, key)?.spec.field(&self.own).int()?;
        usize::try_from(own).ok()
    }

    /// The name the game shows slot `slot` under ("0A").
    pub fn name(&self, slot: usize) -> Option<&str> {
        self.names.get(slot).map(String::as_str)
    }

    /// The field of an entry's data that names a slot by its number (its own
    /// slot; in a table of its data, another slot it names: a two-part
    /// card's other half's).
    pub fn slot_field(&self) -> &str {
        &self.own
    }

    /// `side`'s list: each entry and the slot it is in, in the list's order.
    fn stated(&self, content: &Content, side: &Side) -> Vec<(EntryHandle, usize)> {
        let Some(Stated::List(items)) = side.facts.get(content, &self.field) else { return Vec::new() };
        let of = |record: &Stated| -> Option<(EntryHandle, usize)> {
            let Stated::Record(fields) = record else { return None };
            let get = |name: &str| fields.iter().find(|(k, _)| k == name).map(|(_, v)| v);
            match (get(&self.entry)?, get(&self.slot)?) {
                (Stated::Def(Registry::Entry, Some(h)), Stated::Number(n)) => Some((EntryHandle(*h), usize::try_from(*n).ok()?)),
                _ => None,
            }
        };
        items.iter().filter_map(of).collect()
    }

    /// What each of the slots holds in `side`'s build, by the slot's number
    /// (as many as the slots have names; an entry in a slot past them is in
    /// none of these, and the build's check says so).
    pub fn held(&self, content: &Content, side: &Side) -> Vec<Option<EntryHandle>> {
        let mut out = vec![None; self.names.len()];
        for (entry, slot) in self.stated(content, side) {
            if let Some(place) = out.get_mut(slot) {
                *place = Some(entry);
            }
        }
        out
    }

    fn state(&self, content: &Content, side: &mut Side, mut list: Vec<(EntryHandle, usize)>) -> Result<(), String> {
        list.sort_by_key(|&(_, slot)| slot);
        let records: Vec<Fact> = list
            .iter()
            .map(|&(entry, slot)| {
                Fact::Record(vec![(self.entry.as_str(), Fact::Value(Value::Def(Registry::Entry, entry.0))), (self.slot.as_str(), Fact::Value(Value::Int(slot as i64)))])
            })
            .collect();
        side.set_fact(content, &self.field, &records)
    }

    /// Put `entry` in `side`'s build: in its own slot, in place of what that
    /// slot held (and out of any other slot it sat in). The slot it went
    /// into. An error: its definition says no slot.
    pub fn put(&self, content: &Content, side: &mut Side, entry: EntryHandle) -> Result<usize, String> {
        let own = self.own(content, entry).ok_or_else(|| format!("{} says no slot it belongs in", content.defs.entry(entry).key))?;
        let mut list = self.stated(content, side);
        list.retain(|&(e, slot)| e != entry && slot != own);
        list.push((entry, own));
        self.state(content, side, list)?;
        Ok(own)
    }

    /// Empty slot `slot` of `side`'s build.
    pub fn clear(&self, content: &Content, side: &mut Side, slot: usize) -> Result<(), String> {
        let mut list = self.stated(content, side);
        list.retain(|&(_, s)| s != slot);
        self.state(content, side, list)
    }
}

/// What a game says of its builds (its rules/builds module's data).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Builds {
    game: String,
    /// The versions, in the order every list shows them.
    pub versions: Vec<Version>,
    /// A build's other choices, each of a few presets (none in a game
    /// without: EXE6).
    pub choices: Vec<Choice>,
    /// The build's list whose entries go in slots, in a game that has one
    /// (EXE4's patch cards).
    pub slots: Option<Slots>,
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
            .map(|v| match (v.field("id"), v.field("color").int().and_then(|c| u32::try_from(c).ok()).filter(|c| *c <= 0xFF_FFFF)) {
                (Data::Str(id), Some(color)) => Ok(Version { id: id.clone(), color, set: v.field("set").clone() }),
                _ => Err(format!("{module}: a version has its `id` and its `color` (0xRRGGBB)")),
            })
            .collect::<Result<_, _>>()?;
        if versions.is_empty() {
            return Err(format!("{module}: no `versions`"));
        }
        let mut choices: Vec<Choice> = Vec::new();
        for c in items(data.field("choices")) {
            let (Data::Str(id), Data::Str(fact)) = (c.field("id"), c.field("fact")) else { return Err(format!("{module}: a choice has its `id` and the `fact` that tells its presets apart")) };
            facts::field(content, fact).ok_or_else(|| format!("{module}: choice {id}: {}", facts::no_field(content, fact)))?;
            let navi = match c.field("navi") {
                Data::Nil => None,
                Data::Str(name) => Some(ids::navi(content, game, name).ok_or_else(|| format!("{module}: choice {id}: {game} has no navi {name:?}"))?),
                other => return Err(format!("{module}: choice {id}: its navi is a navi's name, not {other:?}")),
            };
            let presets: Vec<Preset> = items(c.field("presets"))
                .iter()
                .map(|p| match p.field("id") {
                    Data::Str(preset) => Ok(Preset { id: preset.clone(), from: p.field("from").int(), set: p.field("set").clone() }),
                    _ => Err(format!("{module}: choice {id}: a preset has its `id`")),
                })
                .collect::<Result<_, _>>()?;
            if presets.is_empty() {
                return Err(format!("{module}: choice {id} has no presets"));
            }
            choices.push(Choice { id: id.clone(), presets, navi, fact: fact.clone() });
        }
        let slots = match data.field("slots") {
            Data::Nil => None,
            s => {
                let text = |name: &str| match s.field(name) {
                    Data::Str(v) => Ok(v.clone()),
                    _ => Err(format!("{module}: `slots` has its `{name}`")),
                };
                let field = text("field")?;
                facts::field(content, &field).ok_or_else(|| format!("{module}: slots: {}", facts::no_field(content, &field)))?;
                let names: Vec<String> = items(s.field("names")).iter().filter_map(|n| if let Data::Str(n) = n { Some(n.clone()) } else { None }).collect();
                if names.is_empty() {
                    return Err(format!("{module}: `slots` names no slot"));
                }
                Some(Slots { field, names, entry: text("entry")?, slot: text("slot")?, own: text("own")? })
            }
        };
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
        Ok(Builds { game: game.to_string(), versions, choices, slots, at_defaults, new_set: new.field("set").clone(), new_folder })
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

    /// The choice named `id`.
    pub fn choice(&self, id: &str) -> Option<&Choice> {
        self.choices.iter().find(|c| c.id == id)
    }

    /// Whether `side` has choice `c`: any navi's, or its own navi's.
    fn offers(&self, content: &Content, side: &Side, c: &Choice) -> bool {
        c.navi.is_none_or(|n| side.stated_navi(content) == Some(n))
    }

    /// The choices `side` has, in the game's order: a choice of one navi
    /// alone is none of another navi's build (EXE5's light or dark is
    /// MegaMan's: a team navi's build has no such choice).
    pub fn choices_of(&self, content: &Content, side: &Side) -> Vec<&Choice> {
        self.choices.iter().filter(|c| self.offers(content, side, c)).collect()
    }

    /// The preset of choice `choice` that `side` is on: where a number tells
    /// the presets apart, the one with the highest line the side's value
    /// reaches (EXE5's light/dark value: 470 or more is light's, under it
    /// dark's); else the first whose facts are all the side's; else the
    /// first. None: the game has no such choice, or the side's navi none of
    /// it.
    pub fn preset_of(&self, content: &Content, side: &Side, choice: &str) -> Option<&Preset> {
        let c = self.choice(choice).filter(|c| self.offers(content, side, c))?;
        let on_the_line = match side.facts.get(content, &c.fact) {
            Some(Stated::Number(n) | Stated::Optional(Some(n))) => c.presets.iter().filter(|p| p.from.is_some_and(|from| from <= n)).max_by_key(|p| p.from),
            _ => None,
        };
        let exactly = || c.presets.iter().find(|p| self.sets(content, side, &p.set));
        on_the_line.or_else(exactly).or(c.presets.first())
    }

    /// Whether every fact of `set` is `side`'s already.
    fn sets(&self, content: &Content, side: &Side, set: &Data) -> bool {
        let mut as_it = side.clone();
        state(content, &self.game, &mut as_it, set).is_ok() && as_it.facts == side.facts
    }

    /// Put `side` on preset `preset` of choice `choice`: its facts, set
    /// together. An error: no such choice or preset, or the side's navi has
    /// none of the choice.
    pub fn take_preset(&self, content: &Content, side: &mut Side, choice: &str, preset: &str) -> Result<(), String> {
        let c = self.choice(choice).ok_or_else(|| format!("{} has no choice {choice:?}", self.game))?;
        if !self.offers(content, side, c) {
            return Err(format!("{choice} is no choice of the side's navi"));
        }
        let p = c.preset(preset).ok_or_else(|| {
            let known: Vec<&str> = c.presets.iter().map(|p| p.id.as_str()).collect();
            format!("{choice} has no preset {preset:?} (its are {})", known.join(", "))
        })?;
        let mut made = side.clone();
        state(content, &self.game, &mut made, &p.set)?;
        *side = made;
        Ok(())
    }

    /// Settle `side`'s choices, as a build read from a file or made from a
    /// save is settled: each choice it has, on the preset it is on
    /// ([`Builds::preset_of`]: a value that is neither preset's takes the
    /// one on its side of the game's line), with every fact that preset
    /// sets; and each choice its navi has none of, with its facts back at
    /// the rules' defaults. The presets that changed the side, for an import
    /// to say which it took.
    pub fn settle(&self, content: &Content, side: &mut Side) -> Vec<Taken> {
        let mut taken = Vec::new();
        for c in &self.choices {
            if !self.offers(content, side, c) {
                for name in c.facts() {
                    side.facts.reset(content, name);
                }
                continue;
            }
            let Some(p) = self.preset_of(content, side, &c.id) else { continue };
            let was = match side.facts.get(content, &c.fact) {
                Some(Stated::Number(n) | Stated::Optional(Some(n))) => Some(n),
                _ => None,
            };
            let before = side.facts.clone();
            let mut made = side.clone();
            if state(content, &self.game, &mut made, &p.set).is_ok() && made.facts != before {
                *side = made;
                taken.push(Taken { choice: c.id.clone(), preset: p.id.clone(), was });
            }
        }
        taken
    }

    /// Whether fact `name` is one no build states: always the rules'
    /// default (the game's `at_defaults`, and the facts of the engine's
    /// level and base HP roles; every build plays at the most its game
    /// allows). A fact a choice's presets set is stated by them (EXE5's
    /// base HP, with its light or dark MegaMan).
    pub fn never_stated(&self, content: &Content, name: &str) -> bool {
        let by_a_choice = self.choices.iter().any(|c| c.facts().contains(&name));
        !by_a_choice && (self.at_defaults.iter().any(|f| f == name) || matches!(facts::role_of(content, name), Some(PlayerFact::Level | PlayerFact::BaseHp)))
    }

    /// Set `side` to what every build is: the facts no build states back at
    /// the rules' defaults, a navi that must have a level at its last, and
    /// its choices settled ([`Builds::settle`]: an import that says which
    /// presets it took settles first, and reads them). The facts that
    /// changed, by name (a save's import says them).
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
        self.settle(content, side);
        facts::fields(content).into_iter().filter(|f| before.facts.get(content, f.name) != side.facts.get(content, f.name)).map(|f| f.name.to_string()).collect()
    }

    /// Make `side` a build of navi `navi`, as a build is: its level at its
    /// last, its form list its own of the side's version (none for a navi
    /// without), and its choices settled (one that isn't this navi's back at
    /// the rules' defaults: a team navi is neither light nor dark, and its
    /// HP its story's). What the side has installed stays as it is.
    pub fn set_navi(&self, content: &Content, side: &mut Side, navi: NaviHandle) -> Result<(), String> {
        let mut made = side.clone();
        made.set_navi(content, navi)?;
        made.state_own_forms(content);
        self.as_built(content, &mut made);
        *side = made;
        Ok(())
    }

    /// A new build of version `version`: the game's navi that changes form
    /// (MegaMan), the version, the folder a new save of the game starts
    /// with (no Regular or tag chip, as a new save's), the first preset of
    /// each of its choices (a light MegaMan), what the game says a new build
    /// states besides, and every other fact the rules' default (an empty
    /// NaviCust on the largest board, no patch cards). It keeps its game's
    /// rules ([`Checked::of`] finds nothing wrong with it).
    pub fn new_build(&self, content: &Content, version: &str) -> Result<Side, String> {
        let mut side = Side::fresh(content, &self.game)?;
        state(content, &self.game, &mut side, &self.new_set)?;
        self.take_version(content, &mut side, version)?;
        for c in self.choices_of(content, &side) {
            state(content, &self.game, &mut side, &c.presets[0].set)?;
        }
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

/// A build checked: what is wrong with it, and its round set up for the
/// checks (the side on both sides of a match of its game; `check::set_up`:
/// no folder dealt, so a folder being made is set up as it is), which the
/// folder's view and the overview are asked of. Made again after each edit. A build is held to
/// more than a match's side is: what a match's checks say of any side
/// (`check::problems`: the facts, the rules' `validate`), and what the rules
/// hold a player's build to besides (their `build_check`: what the game
/// would run and no save made by play holds). A netplay offer is a build:
/// it is checked so too.
pub struct Checked {
    game: String,
    /// What is wrong with the build, typed; empty: it can be played.
    pub problems: Vec<Problem>,
    battle: Option<Battle>,
}

impl Checked {
    /// `side` checked as a build of `game`: every problem nettai-match's
    /// checks and the game's rules find with it, of any side's and of a
    /// build's.
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
        match check::set_up(content, &m) {
            Ok(mut b) => {
                let mut problems = check::validated(&mut b, 0);
                problems.extend(check::built(&mut b, 0));
                Checked { game: game.to_string(), problems, battle: Some(b) }
            }
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

    /// The face the build's navi starts its round with (the round's first
    /// tick puts the navi on the field, whose start sets its mood: a dark
    /// MegaMan's is his dark face). None: the round doesn't start, or no
    /// navi is on the field by then.
    pub fn face(&self) -> Option<Face> {
        use nettai_battle::kinds::player;
        let mut b = self.battle.clone()?;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            b.tick(&Default::default(), Default::default());
            b.player(0)?;
            let stats = &b.stats[0];
            if !b.content.navi(stats.navi).changes_form() {
                return Some(Face::Navi(stats.navi));
            }
            let emotion = b.game_rules().emotion.name(player::emotion(&b, 0)).to_string();
            Some(Face::Form(stats.form, emotion, player::shows_face_variant(&b, 0)))
        }))
        .ok()
        .flatten()
    }

    /// The build's overview. None: the round doesn't start.
    pub fn overview(&mut self, content: &Arc<Content>, side: &Side) -> Option<Overview> {
        let b = self.battle.as_mut()?;
        let with = b.stats[0];
        // The navi alone: the side with nothing installed.
        let alone = Match::empty(content, &self.game).ok().and_then(|m| {
            let bare = without_entries(content, side);
            check::set_up(content, &Match { sides: [bare.clone(), bare], ..m }).ok().map(|b| b.stats[0])
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
        let extent = |d: &Data| match (d.field("level").int(), d.field("amount").int()) {
            (Some(level), _) => Some(Extent::Level(level.clamp(0, 255) as u8)),
            (None, Some(amount)) => Some(Extent::Amount { amount: u32::try_from(amount).ok()?, of: d.field("of").int().and_then(|of| u32::try_from(of).ok()) }),
            (None, None) => None,
        };
        let bugs = items(answer.field("bugs"))
            .iter()
            .filter_map(|d| match (d.field("name"), extent(d)) {
                (Data::Str(key), Some(extent)) => Some(Bug::Named { key: key.clone(), extent }),
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

/// The face the emotion window shows of a navi: its form's for an emotion
/// (by the emotion's name in the game's rules; of the form's second set
/// where the rules ask), or a navi's own where it doesn't change form.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Form(FormHandle, String, bool),
    Navi(NaviHandle),
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
/// table and how much of it there is, whoever brings it, the NaviCust or a
/// patch card; or as an installed entry's effect (a patch card's line: an
/// ordinary effect that the card marks as a drawback, which is no bug of its
/// own kind).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bug {
    Named { key: String, extent: Extent },
    From(Source),
}

/// How much of a named bug a build has: its level, or else its amount. The
/// game's rules say which (a game's rules/navicust, `card_bug`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    /// The bug's level: what the build's navi has is what the bug brings at
    /// that level. The NaviCust's bugs are at level 1 to 3; a patch card's
    /// is at the level whose work its own is (1, where the bug is only on or
    /// off); an HP drain's level is its own, 1 to 7, the NaviCust's and a
    /// card's together.
    Level(u8),
    /// The number a patch card's bug writes, where that is what no level of
    /// the bug brings: a number by itself (the damage as the custom screen
    /// opens), or one of `of` (the buster's blank shots of 16, the steps in
    /// 8 that leave a trail).
    Amount { amount: u32, of: Option<u32> },
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
    /// The navi's bugs, as the NaviCust and the patch cards together leave
    /// them: a bug that both bring is there once.
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

    /// A named bug at a level, and one with an amount.
    fn level(key: &str, level: u8) -> Bug {
        Bug::Named { key: key.into(), extent: Extent::Level(level) }
    }

    fn amount(key: &str, amount: u32, of: Option<u32>) -> Bug {
        Bug::Named { key: key.into(), extent: Extent::Amount { amount, of } }
    }

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

    /// A side without its patch cards, in each game (`Side::without_patch_cards`:
    /// what a match played with patch cards off plays). The list its rules
    /// keep them in is empty, whatever a card is there (EXE6's and EXE5's a
    /// card, EXE4's a card in its slot), and no other fact of the side
    /// changes. It is the build before it had the card, by what its round
    /// starts with (which the card had changed) and by its cards' MB, and
    /// nothing is wrong with it. A side with no cards is the same without
    /// them.
    #[test]
    fn a_side_without_its_patch_cards() {
        let entry = |h: EntryHandle| Fact::Value(Value::Def(Registry::Entry, h.0));
        for (content, game, versions) in games() {
            let builds = Builds::of(&content, game).unwrap();
            let plain = builds.new_build(&content, versions[0]).unwrap();
            let cards = match game {
                "exe4" => {
                    let custom2 = ids::entry(&content, game, facts::PATCH_CARDS, "017-custom2").unwrap();
                    vec![Fact::Record(vec![("card", entry(custom2)), ("slot", Fact::Value(Value::Int(0)))])]
                }
                "exe5" => patch_cards(&content, game, "quaker"),
                _ => patch_cards(&content, game, "canodumb"),
            };
            let mut with = plain.clone();
            with.set_fact(&content, facts::PATCH_CARDS, &cards).unwrap();
            assert_eq!(Checked::of(&content, game, &with).problems, Vec::new(), "{game}");
            let bare = with.without_patch_cards(&content);
            assert_ne!(bare, with, "{game}");
            // The list is empty, and every other fact is as it was.
            for f in facts::fields(&content) {
                let (was, is) = (with.facts.get(&content, f.name), bare.facts.get(&content, f.name));
                if f.name == facts::PATCH_CARDS {
                    assert_eq!(is, Some(Stated::List(Vec::new())), "{game}");
                } else {
                    assert_eq!(was, is, "{game}: {}", f.name);
                }
            }
            // The build before it had the card.
            let looked = |side: &Side| {
                let mut checked = Checked::of(&content, game, side);
                assert_eq!(checked.problems, Vec::new(), "{game}");
                let used = checked.overview(&content, side).unwrap().budget(facts::PATCH_CARDS).map(|b| b.count.used);
                (*checked.stats().unwrap(), used)
            };
            let (of_plain, of_with, of_bare) = (looked(&plain), looked(&with), looked(&bare));
            assert_eq!(of_bare, of_plain, "{game}");
            assert_ne!(of_with.0, of_plain.0, "{game}: the card changes what the round starts with");
            // (EXE4's cards go by their slots: its list has no MB to count.)
            assert!(of_with.1 > of_bare.1 || game == "exe4", "{game}: the cards' MB");
            // A side with none is the same without them, and taking them
            // out twice is taking them out.
            assert_eq!(looked(&plain.without_patch_cards(&content)), of_plain, "{game}");
            assert_eq!(bare.without_patch_cards(&content), bare, "{game}");
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

    /// A version's forms are its own: a new build of each has its version's
    /// souls (EXE4's and EXE5's, which the version sets) or Crosses (EXE6's,
    /// MegaMan's own forms of the version), and is told by them as that
    /// version; made the other version, it has the other's. In EXE4 and
    /// EXE5, whose rules take no version, a side with some souls of each, or
    /// every soul (the rules' default, a fresh side's), is of no version; in
    /// EXE6 the version is the rules' own fact, whatever the Crosses.
    #[test]
    fn each_version_has_its_own_forms() {
        let red = ["rollsoul", "gutssoul", "windsoul", "searchsoul", "firesoul", "thundersoul"];
        let blue = ["protosoul", "numbersoul", "metalsoul", "junksoul", "aquasoul", "woodsoul"];
        let protoman = ["protosoul", "gyrosoul", "searchsoul", "napalmsoul", "magnetsoul", "meddysoul"];
        let colonel = ["colonelsoul", "shadowsoul", "numbersoul", "tomahawksoul", "knightsoul", "toadsoul"];
        let gregar = ["heatcross", "eleccross", "slashcross", "erasecross", "chargecross"];
        let falzar = ["spoutcross", "tomahawkcross", "tengucross", "groundcross", "dustcross"];
        // (The game, the fact that lists the forms, the versions and each one's forms.)
        let games: [(Arc<Content>, &str, &str, [&str; 2], [&[&str]; 2]); 3] = [
            (exe4_content(), "exe4", "souls", ["redsun", "bluemoon"], [&red, &blue]),
            (exe5_content(), "exe5", "souls", ["protoman", "colonel"], [&protoman, &colonel]),
            (exe6_content(), "exe6", "crosses", ["gregar", "falzar"], [&gregar, &falzar]),
        ];
        for (content, game, fact, versions, forms) in games {
            let builds = Builds::of(&content, game).unwrap();
            let listed = |side: &Side| side.facts.get(&content, fact).map(|v| v.defs()).unwrap_or_default();
            let names = |side: &Side| {
                let mut names: Vec<String> = listed(side).iter().map(|&h| ids::local(&content.defs.form(FormHandle(h)).key).to_string()).collect();
                names.sort();
                names
            };
            let sorted = |list: &[&str]| {
                let mut list: Vec<String> = list.iter().map(|s| s.to_string()).collect();
                list.sort();
                list
            };
            for (v, version) in versions.iter().enumerate() {
                let mut side = builds.new_build(&content, version).unwrap();
                assert_eq!(names(&side), sorted(forms[v]), "{game} {version}");
                assert_eq!(builds.version_of(&content, &side), Some(*version), "{game}");
                assert_eq!(side.version(&content), (game == "exe6").then_some(*version), "{game}: the rules' own version");
                builds.take_version(&content, &mut side, versions[1 - v]).unwrap();
                assert_eq!(names(&side), sorted(forms[1 - v]), "{game} {version} made {}", versions[1 - v]);
                assert_eq!(builds.version_of(&content, &side), Some(versions[1 - v]), "{game}");
            }
            // Some of each version's: of neither.
            let mut mixed = builds.new_build(&content, versions[0]).unwrap();
            let mut list = listed(&mixed);
            list[0] = ids::form(&content, game, forms[1][0]).unwrap().0;
            let list: Vec<Fact> = list.iter().map(|&h| Fact::Value(Value::Def(Registry::Form, h))).collect();
            mixed.set_fact(&content, fact, &list).unwrap();
            if game == "exe6" {
                // (EXE6's version is its rules' own fact, not its Crosses.)
                assert_eq!(builds.version_of(&content, &mixed), Some(versions[0]), "{game}: a mix of Crosses");
            } else {
                assert_eq!(builds.version_of(&content, &mixed), None, "{game}: a mix of souls");
                assert_eq!(builds.version_of(&content, &Side::fresh(&content, game).unwrap()), None, "{game}: every soul");
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
        assert_eq!(o.bugs, [level("step_bug", 1)]);
        let text = |key: &str| content.strings.text(BUGS_TABLE, key);
        assert_eq!(text("step_bug"), Some("Movement Bug"));
        // A patch card's effect of the kind that writes the bug's stat takes
        // its place: BomBoy's fixes the step bug (no bug of it left; the card's
        // own bug line is its charged shot's, an ordinary effect marked as a
        // drawback, which stays the card's line), and BeastMan's is the step
        // bug again, the same bug by the same name. A fix is no ability.
        let card = |name: &str| ids::entry(&content, "exe6", "patch_cards", name).unwrap();
        let with = |cards: &str| {
            let mut side = side.clone();
            side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", cards)).unwrap();
            overview(&side)
        };
        let fixed = with("bomboy");
        assert_eq!(fixed.bugs, [Bug::From(Source::Effect(card("bomboy"), 1))]);
        assert_eq!(fixed.abilities, [Source::Effect(card("bomboy"), 0)]);
        assert_eq!(with("beastman").bugs, [level("step_bug", 1)]);
        // The NaviCust's HP bug (an HP program on the command line) is two
        // bugs: the HP drain in the fight, and a hit's HP bug. A card's
        // drain adds: EraseMan's 1 more makes the one bug level 2. Kettle's
        // 0 clears the drain and leaves the hit's, and its own bug is the
        // custom screen's damage, 40.
        set_navicust(&content, &mut side, &[placed("hp-100", "white", 3, 4)], 2);
        assert_eq!(overview(&side).bugs, [level("hp_drain", 1), level("hit_status.hp_bug", 1)]);
        assert_eq!((text("hp_drain"), text("hit_status.hp_bug"), text("custom_damage")), (Some("Battle HP Bug"), Some("Damage HP Bug"), Some("Custom Damage")));
        let with = |cards: &str| {
            let mut side = side.clone();
            side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", cards)).unwrap();
            overview(&side)
        };
        assert_eq!(with("eraseman").bugs, [level("hp_drain", 2), level("hit_status.hp_bug", 1)]);
        assert_eq!(with("kettle").bugs, [level("hit_status.hp_bug", 1), amount("custom_damage", 40, None)]);
        // Cragger's is the hit's HP bug again, and BombCorn's a hit's
        // confusion in its place.
        assert_eq!(with("cragger").bugs, [level("hp_drain", 1), level("hit_status.hp_bug", 1)]);
        assert_eq!(with("bombcorn").bugs, [level("hp_drain", 1), level("hit_status.confuse", 1)]);
        // A link navi's cards do nothing: nothing of them is listed but their MB.
        let mut link = side.clone();
        link.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", "beastman")).unwrap();
        set_navicust(&content, &mut link, &[], 2);
        builds.set_navi(&content, &mut link, ids::navi(&content, "exe6", "elecman").unwrap()).unwrap();
        let o = overview(&link);
        assert_eq!((o.abilities.len(), o.bugs.len(), o.budget("patch_cards").map(|b| b.count.used)), (0, 0, Some(33)));
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

    /// A patch card's bug is listed as the bug it is: by the name the
    /// NaviCust's bug of the same stat has, or by its own where only cards
    /// bring it, at its level or with its amount. An ordinary effect that a
    /// card marks as a drawback stays the card's line, and a bug slot's
    /// value that is no bug is an ability.
    #[test]
    fn a_cards_bug_is_listed_as_the_bug_it_is() {
        let content = exe6_content();
        let mut clean = Builds::of(&content, "exe6").unwrap().new_build(&content, "falzar").unwrap();
        set_navicust(&content, &mut clean, &[], 2);
        let card = |name: &str| ids::entry(&content, "exe6", "patch_cards", name).unwrap();
        let with = |cards: &str| {
            let mut side = clean.clone();
            side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", cards)).unwrap();
            Checked::of(&content, "exe6", &side).overview(&content, &side).unwrap().bugs
        };
        let text = |key: &str| content.strings.text(BUGS_TABLE, key);
        // On or off: level 1. DarkMan's custom damage is no level's: 50.
        assert_eq!(with("darkman"), [level("step_bug", 1), amount("custom_damage", 50, None)]);
        assert_eq!(with("volcano"), [level("emotion_bug", 1)]);
        assert_eq!((with("cragger"), text("hit_status.hp_bug")), (vec![level("hit_status.hp_bug", 1)], Some("Damage HP Bug")));
        assert_eq!((with("bombcorn"), text("hit_status.confuse")), (vec![level("hit_status.confuse", 1)], Some("Damage Confusion")));
        // The battle start's bug at 1 and at 2; ColorMan's other bug line is
        // its lost shield, the card's own.
        assert_eq!(with("count"), [level("status_bug", 2)]);
        assert_eq!(with("colorman"), [Bug::From(Source::Effect(card("colorman"), 2)), level("status_bug", 1)]);
        // The drains at their own levels: of two cards', the later is the
        // one that holds (as its write is).
        assert_eq!((with("japanman"), text("custom_hp_drain")), (vec![level("custom_hp_drain", 2)], Some("Custom HP Bug")));
        assert_eq!(with("eraseman"), [level("hp_drain", 1)]);
        // The cracked trail is the NaviCust's panel bug, but at 1 step in 8
        // where its level 1 is 2; grass is a trail only cards bring; and 4
        // blank shots of 16 are fewer than the buster bug's level 1.
        assert_eq!((with("dustman"), text("panel_trail.crack")), (vec![amount("panel_trail.crack", 1, Some(8))], Some("Panel Bug")));
        assert_eq!((with("spidy"), text("panel_trail.grass")), (vec![amount("panel_trail.grass", 2, Some(8))], Some("Movement Grass")));
        assert_eq!((with("knigtman"), text("buster_blanks")), (vec![amount("buster_blanks", 4, Some(16))], Some("Buster Blank")));
        // Two cards' bugs, and one card's fix of the other's: Kettle clears
        // EraseMan's drain (the later write), and brings its own.
        assert_eq!(with("eraseman, kettle"), [amount("custom_damage", 40, None)]);
        assert_eq!(with("kettle, eraseman"), [amount("custom_damage", 40, None), level("hp_drain", 1)]);

        // EXE5: the NaviCust's custom bug is the custom screen's damage, at
        // its level; a card's is the same bug with its own damage, in its
        // place (the card's write is the one that holds).
        let content = exe5_content();
        let mut clean = Builds::of(&content, "exe5").unwrap().new_build(&content, "colonel").unwrap();
        set_navicust(&content, &mut clean, &[], 2);
        let megfldr1 = ids::entry(&content, "exe5", "navicust_programs", "megfldr1").unwrap();
        let bugs = |side: &Side| {
            let mut checked = Checked::of(&content, "exe5", side);
            if checked.problems.is_empty() { checked.overview(&content, side).map(|o| (o.bugs, o.abilities)) } else { None }
        };
        // (MegFldr1 turned and somewhere off the command line: its bug alone.)
        let bugged = (0..4 * 49u8)
            .map(|at| {
                let mut side = clean.clone();
                set_navicust(&content, &mut side, &[PlacedProgram { program: megfldr1, color: 0, x: at % 7, y: at / 7 % 7, rotation: at / 49, compressed: false }], 2);
                side
            })
            .find(|side| bugs(side).is_some_and(|(bugs, _)| bugs == [level("custom_damage", 1)]))
            .expect("MegFldr1 placed off the command line brings the custom bug");
        let card = |name: &str| ids::entry(&content, "exe5", "patch_cards", name).unwrap();
        let with = |side: &Side, cards: &str| {
            let mut side = side.clone();
            side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe5", cards)).unwrap();
            bugs(&side).unwrap()
        };
        assert_eq!(with(&bugged, "bugtank").0, [amount("custom_damage", 50, None)]);
        // Cloudy clears it; its own bug line is its lost Super Armor.
        assert_eq!(with(&bugged, "cloudy").0, [Bug::From(Source::Effect(card("cloudy"), 1))]);
        // The buster bug's own shots (4 blank, 2 charged) are its level 1;
        // any others, the blank shots of 16.
        assert_eq!(with(&clean, "zomon").0, [level("buster_blanks", 1)]);
        assert_eq!(with(&clean, "junkman").0, [amount("buster_blanks", 4, Some(16))]);
        assert_eq!(with(&clean, "appley").0, [amount("buster_blanks", 1, Some(16)), level("emotion_bug", 1)]);
        let text = |key: &str| content.strings.text(BUGS_TABLE, key);
        assert_eq!((with(&clean, "lark").0, text("hit_status.blind")), (vec![level("hit_status.blind", 1)], Some("Damage Blindness")));
        assert_eq!((with(&clean, "whirly").0, text("panel_trail.water")), (vec![amount("panel_trail.water", 2, Some(8))], Some("Movement Water")));
        // BrushMan's holy trail is no bug: an ability, the card's line.
        let (bugs, abilities) = with(&clean, "brushman");
        assert_eq!((bugs, abilities.contains(&Source::Effect(card("brushman"), 2))), (Vec::new(), true));
    }

    /// Every bug a game has, whoever brings it, has its name in the game's
    /// strings (the `bugs` table), in each language the content has, and no
    /// name there is of no bug. The bugs are the NaviCust's (the rules' list,
    /// each under the names the rules give it) and those the game's patch
    /// cards bring (each card alone on a build). A bug a card brings is named
    /// by the key of the card effect's own line, where the game's cards have
    /// lines. The names are full names: one bug that the games' cards name
    /// alike reads alike in the three games.
    #[test]
    fn every_bug_has_its_name() {
        use std::collections::BTreeSet;
        let mut names: HashMap<(&str, String), String> = HashMap::new();
        for (content, game, versions) in games() {
            let programs = nettai_battle::behavior::content_module_data(&content, &format!("{game}:rules/navicust/programs")).unwrap().unwrap();
            let mut bugs: BTreeSet<String> = BTreeSet::new();
            for bug in items(programs.field("BUGS")) {
                let Data::Str(bug) = bug else { panic!("{bug:?}") };
                match programs.field("BUG_NAMES").field(bug) {
                    Data::List(named) => bugs.extend(named.iter().map(|n| n.field("name").str().unwrap().to_string())),
                    _ => drop(bugs.insert(bug.clone())),
                }
            }
            assert!(bugs.len() >= 10, "{game}");
            // The cards' bugs: each card alone on a build with no NaviCust.
            let builds = Builds::of(&content, game).unwrap();
            let mut clean = builds.new_build(&content, versions[0]).unwrap();
            clean.set_fact(&content, "navicust_programs", &[]).unwrap();
            let mut carded = 0;
            for card in content.defs.entries_of("patch_cards") {
                let mut side = clean.clone();
                match &builds.slots {
                    Some(slots) => drop(slots.put(&content, &mut side, card).unwrap()),
                    None => side.set_fact(&content, "patch_cards", &[Fact::Value(Value::Def(Registry::Entry, card.0))]).unwrap(),
                }
                let o = Checked::of(&content, game, &side).overview(&content, &side).unwrap();
                let named: Vec<&String> = o.bugs.iter().filter_map(|b| if let Bug::Named { key, .. } = b { Some(key) } else { None }).collect();
                carded += usize::from(!named.is_empty());
                for key in named {
                    let line = content.strings.text("patch_card_effects", key);
                    assert!(builds.slots.is_some() || line.is_some(), "{game}: the bug {key} is no card effect's line");
                    bugs.insert(key.clone());
                }
            }
            match game {
                "exe6" => assert_eq!(carded, 42),
                "exe5" => assert_eq!(carded, 31),
                _ => assert!(carded > 0),
            }
            for lang in ["en", "ja"] {
                let path = format!("{}/../../content/{game}/locales/{lang}.toml", env!("CARGO_MANIFEST_DIR"));
                let table: toml::Table = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
                let named: BTreeSet<String> = table[BUGS_TABLE].as_table().unwrap().keys().cloned().collect();
                assert_eq!(named, bugs, "{path}");
            }
            for bug in bugs {
                let name = content.strings.text(BUGS_TABLE, &bug).unwrap_or_else(|| panic!("{game}: no name for the bug {bug}"));
                assert!(name.split(' ').all(|word| word.starts_with(|c: char| c.is_uppercase())), "{game}: {name:?} is no full name in the others' style");
                names.insert((game, bug), name.to_string());
            }
        }
        let name = |game: &'static str, bug: &str| names[&(game, bug.to_string())].as_str();
        // The same bug by what it writes, the same name, whichever game has
        // it and whoever brings it there; and the three custom bugs, which
        // write three stats, three names.
        assert_eq!((name("exe6", "step_bug"), name("exe5", "step_bug"), name("exe4", "move")), ("Movement Bug", "Movement Bug", "Movement Bug"));
        assert_eq!((name("exe6", "hp_drain"), name("exe5", "hp_drain"), name("exe4", "hp")), ("Battle HP Bug", "Battle HP Bug", "Battle HP Bug"));
        assert_eq!((name("exe6", "hit_status.hp_bug"), name("exe5", "hit_status.hp_bug")), ("Damage HP Bug", "Damage HP Bug"));
        assert_eq!((name("exe6", "custom"), name("exe6", "custom_damage"), name("exe5", "custom_damage")), ("Custom Bug", "Custom Damage", "Custom Damage"));
        assert_eq!((name("exe6", "custom_hp_drain"), name("exe5", "custom_hp_drain"), name("exe4", "custom")), ("Custom HP Bug", "Custom HP Bug", "Custom HP Bug"));
        assert_eq!((name("exe6", "buster_blanks"), name("exe5", "panel_trail.crack"), name("exe4", "panel")), ("Buster Blank", "Panel Bug", "Panel Bug"));
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
        // Custom2 (the custom level +2, and its custom bug once) in its slot, 0A.
        side.set_fact(&content, "patch_cards", &[Fact::Record(vec![("card", entry(custom2)), ("slot", int(0))])]).unwrap();
        let mut checked = Checked::of(&content, "exe4", &side);
        assert_eq!(checked.problems, Vec::new());
        let o = checked.overview(&content, &side).unwrap();
        assert_eq!(o.custom.added(), 2);
        assert_eq!(o.abilities, [Source::Entry(undersht)]);
        assert_eq!(o.bugs, [level("custom", 1)]);
        assert_eq!((o.tag_memory, o.lists.len()), (None, 0));
        // Two cards in one slot: said of the second, typed.
        side.set_fact(&content, "patch_cards", &[Fact::Record(vec![("card", entry(custom2)), ("slot", int(0))]), Fact::Record(vec![("card", entry(custom2)), ("slot", int(0))])]).unwrap();
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

    /// EXE4's patch cards go in six slots, 0A to 0F, each card in its own:
    /// a card put in goes to its own slot in place of what was there, and a
    /// build with a card elsewhere (the seventh slot among them) has a typed
    /// problem, which a match's checks, as the game's reload, leave be.
    #[test]
    fn exe4s_cards_go_in_their_own_slots() {
        let content = exe4_content();
        let builds = Builds::of(&content, "exe4").unwrap();
        let slots = builds.slots.clone().unwrap();
        assert_eq!((slots.field.as_str(), slots.names.join(" ")), ("patch_cards", "0A 0B 0C 0D 0E 0F".to_string()));
        assert!(Builds::of(&exe5_content(), "exe5").unwrap().slots.is_none() && Builds::of(&exe6_content(), "exe6").unwrap().slots.is_none());
        // Every card states its slot, one of the six; the cards of slot 0A
        // and one of another.
        let cards = content.defs.entries_of("patch_cards");
        assert!(cards.iter().all(|&c| slots.own(&content, c).is_some_and(|s| s < 6)));
        let custom2 = ids::entry(&content, "exe4", "patch_cards", "017-custom2").unwrap();
        assert_eq!((slots.own(&content, custom2), slots.name(0)), (Some(0), Some("0A")));
        let another_0a = cards.iter().copied().find(|&c| c != custom2 && slots.own(&content, c) == Some(0)).unwrap();
        let elsewhere = cards.iter().copied().find(|&c| slots.own(&content, c) == Some(4)).unwrap();
        let mut side = builds.new_build(&content, "redsun").unwrap();
        assert_eq!(slots.held(&content, &side), vec![None; 6]);
        assert_eq!(slots.put(&content, &mut side, custom2), Ok(0));
        assert_eq!(slots.put(&content, &mut side, elsewhere), Ok(4));
        assert_eq!(slots.held(&content, &side), [Some(custom2), None, None, None, Some(elsewhere), None]);
        assert_eq!(Checked::of(&content, "exe4", &side).problems, Vec::new());
        // Another card of 0A takes Custom2's place; a slot is emptied.
        assert_eq!(slots.put(&content, &mut side, another_0a), Ok(0));
        assert_eq!(slots.held(&content, &side), [Some(another_0a), None, None, None, Some(elsewhere), None]);
        slots.clear(&content, &mut side, 4).unwrap();
        assert_eq!(slots.held(&content, &side), [Some(another_0a), None, None, None, None, None]);
        // A card out of its slot (a file's): a build's problem, with the
        // card, the slot it is in and its own; the seventh slot too. A
        // match's checks say nothing of either.
        let entry = |h: EntryHandle| Fact::Value(Value::Def(Registry::Entry, h.0));
        let stated = |side: &mut Side, at: i64| side.set_fact(&content, "patch_cards", &[Fact::Record(vec![("card", entry(custom2)), ("slot", Fact::Value(Value::Int(at)))])]).unwrap();
        for at in [3, 6] {
            stated(&mut side, at);
            let checked = Checked::of(&content, "exe4", &side);
            let got: Vec<(&Wrong, Option<&str>, Option<usize>)> = checked.problems.iter().map(|p| (&p.what, p.field.as_deref(), p.entry)).collect();
            assert_eq!(got, [(&Wrong::Slot { entry: custom2, slot: at as u32, own: 0 }, Some("patch_cards"), Some(0))], "slot {at}");
            assert!(!checked.ready());
            let m = Match { sides: [side.clone(), side.clone()], ..Match::empty(&content, "exe4").unwrap() };
            assert_eq!(check::problems(&content, &m), Vec::new(), "slot {at}: the reload runs a card in any of its seven slots");
        }
        // (A card of slot 0A as the held slots show it out of place: slot 3 holds it.)
        stated(&mut side, 3);
        assert_eq!(slots.held(&content, &side)[3], Some(custom2));
        // Put in, it goes back to its own.
        assert_eq!(slots.put(&content, &mut side, custom2), Ok(0));
        assert_eq!(Checked::of(&content, "exe4", &side).problems, Vec::new());
        // An eighth slot is no slot of a save's: any side's problem.
        stated(&mut side, 7);
        let checked = Checked::of(&content, "exe4", &side);
        assert_eq!(checked.problems.iter().map(|p| &p.what).collect::<Vec<_>>(), [&Wrong::Range { value: 7, least: 0, most: 6 }]);
    }

    /// A NaviCust bug and the patch card effect whose name it has are the
    /// same bug by what each writes: in EXE6, a build whose NaviCust brings
    /// the bug and a build with a card whose one bug effect is of that kind
    /// have the same bug stats changed (the NaviCust's HP bug changes two,
    /// one for each of its two names), and each is listed under the one
    /// name. Where they are listed alike, at the same level, the stats hold
    /// the same values; and a card that writes what no level of the bug does
    /// (DustMan's trail, KnightMan's blank shots) has an amount.
    #[test]
    fn a_bug_and_its_cards_effect_write_the_same_stat() {
        let content = exe6_content();
        let builds = Builds::of(&content, "exe6").unwrap();
        let clean = builds.new_build(&content, "gregar").unwrap();
        let stats = |side: &Side| *Checked::of(&content, "exe6", side).stats().unwrap();
        // The engine's bug stats, by name.
        let bug_stats = |s: &NaviStats| {
            let b = &s.bugs;
            [
                ("auto_step", b.auto_step as u32),
                ("panel_trail_kind", b.panel_trail_kind as u32),
                ("panel_trail_level", b.panel_trail_level as u32),
                ("buster_blanks", b.buster_blanks as u32),
                ("buster_charged", b.buster_charged as u32),
                ("hit_status", b.hit_status as u32),
                ("hp_drain", b.hp_drain as u32),
                ("custom_drain", b.custom_drain as u32),
                ("battle_start", b.battle_start as u32),
                ("emotion", b.emotion as u32),
                ("processing", b.processing as u32),
                ("starting_damage", b.starting_damage as u32),
                ("custom_damage", b.custom_damage as u32),
                ("hand_shrink_turn", b.hand_shrink_turn as u32),
            ]
        };
        let base = bug_stats(&stats(&clean));
        let changed = |side: &Side| -> Vec<&'static str> { bug_stats(&stats(side)).iter().zip(&base).filter(|(now, was)| now.1 != was.1).map(|(now, _)| now.0).collect() };
        let program = |name: &str| ids::entry(&content, "exe6", "navicust_programs", name).unwrap();
        // (The bug's name, a program placed so that the NaviCust brings it, a
        // card whose one bug effect is of the bug's kind, and the stats both
        // write.)
        for (key, placed, card, same) in [
            ("step_bug", ("undersht", 1, 2), "beastman", vec!["processing"]),
            ("emotion_bug", ("humor", 2, 5), "volcano", vec!["emotion"]),
            ("panel_trail.crack", ("flotshoe", 2, 5), "dustman", vec!["panel_trail_kind", "panel_trail_level"]),
            ("buster_blanks", ("attack-1", 5, 4), "knigtman", vec!["buster_blanks", "buster_charged"]),
            ("hp_drain", ("hp-100", 3, 4), "eraseman", vec!["hp_drain"]),
            ("hit_status.hp_bug", ("hp-100", 3, 4), "cragger", vec!["hit_status"]),
        ] {
            let mut by_navicust = clean.clone();
            set_navicust(&content, &mut by_navicust, &[PlacedProgram { program: program(placed.0), color: 0, x: placed.1, y: placed.2, rotation: 0, compressed: false }], 2);
            let mut by_card = clean.clone();
            by_card.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", card)).unwrap();
            let listed = |side: &Side| -> Vec<Bug> {
                let mut checked = Checked::of(&content, "exe6", side);
                assert_eq!(checked.problems, Vec::new(), "{key}");
                checked.overview(&content, side).unwrap().bugs.into_iter().filter(|b| matches!(b, Bug::Named { key: k, .. } if k == key)).collect()
            };
            let (of_navicust, of_card) = (listed(&by_navicust), listed(&by_card));
            assert_eq!((of_navicust.len(), of_card.len()), (1, 1), "{key}: the program placed and {card} each bring it, under the one name");
            let (wrote, card_wrote) = (changed(&by_navicust), changed(&by_card));
            assert_eq!(card_wrote, same, "{key}: {card}'s bug effect");
            if key == "hp_drain" || key == "hit_status.hp_bug" {
                assert_eq!(wrote, ["hit_status", "hp_drain"], "the HP bug is both");
            } else {
                assert_eq!(wrote, same, "{key}");
            }
            // (A bug that is only on or off is listed at the NaviCust's
            // count's level, whatever it is: the same stat at any.)
            let values = |side: &Side| -> Vec<u32> { bug_stats(&stats(side)).iter().filter(|(stat, _)| same.contains(stat)).map(|(_, value)| *value).collect() };
            let amount = matches!(of_card[0], Bug::Named { extent: Extent::Amount { .. }, .. });
            assert!(of_navicust != of_card || values(&by_navicust) == values(&by_card), "{key}: {of_navicust:?} twice, of other values");
            assert!(!amount || values(&by_navicust) != values(&by_card), "{key}: {of_card:?} writes what {of_navicust:?} does");
            assert_eq!(amount, ["panel_trail.crack", "buster_blanks"].contains(&key), "{key}: {of_card:?}");
        }
    }

    /// A side's number fact.
    fn number(content: &Content, side: &Side, fact: &str) -> Option<i64> {
        match side.facts.get(content, fact) {
            Some(Stated::Number(n)) => Some(n),
            _ => None,
        }
    }

    /// EXE5's and EXE4's light or dark MegaMan: one choice of two presets,
    /// light first, each the light/dark value and the base HP together (as
    /// Tango's saves have them). A new build is light; a dark one is ready
    /// too, with its 997 HP in the round's stats and the overview, and
    /// starts the round with another face. EXE6 has no such choice.
    #[test]
    fn megaman_is_light_or_dark() {
        // (The game, light's and dark's value and HP.)
        for (content, game, light, dark) in [(exe5_content(), "exe5", (1000, 1000), (0, 997)), (exe4_content(), "exe4", (1000, 1000), (460, 997))] {
            let builds = Builds::of(&content, game).unwrap();
            assert_eq!(builds.choices.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["light_dark"], "{game}");
            assert_eq!(builds.choices[0].presets.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["light", "dark"], "{game}");
            let mut side = builds.new_build(&content, &builds.versions[0].id.clone()).unwrap();
            let facts = |side: &Side| (number(&content, side, "karma").unwrap(), number(&content, side, "hp").unwrap());
            let on = |side: &Side| builds.preset_of(&content, side, "light_dark").map(|p| p.id.clone());
            assert_eq!((on(&side).as_deref(), facts(&side)), (Some("light"), light), "{game}");
            assert_eq!(builds.choices_of(&content, &side).len(), 1);
            assert!(!builds.never_stated(&content, "hp") && !builds.never_stated(&content, "karma"), "{game}: the choice states them");
            let light_face = Checked::of(&content, game, &side).face();
            builds.take_preset(&content, &mut side, "light_dark", "dark").unwrap();
            assert_eq!((on(&side).as_deref(), facts(&side)), (Some("dark"), dark), "{game}");
            // As built, a dark build stays dark: nothing to mend.
            assert_eq!(builds.as_built(&content, &mut side), Vec::<String>::new(), "{game}");
            let mut checked = Checked::of(&content, game, &side);
            assert_eq!(checked.problems, Vec::new(), "{game}");
            assert_eq!(checked.stats().map(|s| (s.max_base_hp, s.max_hp, s.hp)), Some((997, 997, 997)), "{game}");
            let o = checked.overview(&content, &side).unwrap();
            assert_eq!(o.hp, Stat { base: 997, value: 997 }, "{game}");
            let dark_face = checked.face();
            assert!(matches!((&light_face, &dark_face), (Some(Face::Form(..)), Some(Face::Form(..)))), "{game}: {light_face:?}, {dark_face:?}");
            assert_ne!(light_face, dark_face, "{game}: a dark MegaMan starts with his dark face");
            // The folder's rules are the same for both: a dark build's new
            // folder is as legal as a light one's.
            assert_eq!(checked.folder_view(&content, &side.folder(&content)).unwrap().problems, Vec::new());
            builds.take_preset(&content, &mut side, "light_dark", "light").unwrap();
            assert_eq!(facts(&side), light, "{game}");
            assert!(builds.take_preset(&content, &mut side, "light_dark", "grey").unwrap_err().contains("no preset"));
            assert!(builds.take_preset(&content, &mut side, "mood", "light").unwrap_err().contains("no choice"));
        }
        let six = exe6_content();
        let builds = Builds::of(&six, "exe6").unwrap();
        let side = builds.new_build(&six, "gregar").unwrap();
        assert!(builds.choices.is_empty() && builds.preset_of(&six, &side, "light_dark").is_none());
        assert!(matches!(Checked::of(&six, "exe6", &side).face(), Some(Face::Form(..))));
    }

    /// A build read from a file or made from a save whose light/dark value
    /// is neither preset's takes the preset on its side of the game's line
    /// (EXE5's 470, EXE4's 500), with that preset's HP, and the settling
    /// says which it took and the value it was.
    #[test]
    fn a_value_between_takes_its_side_of_the_line() {
        for (content, game, line, dark) in [(exe5_content(), "exe5", 470, (0, 997)), (exe4_content(), "exe4", 500, (460, 997))] {
            let builds = Builds::of(&content, game).unwrap();
            let new = builds.new_build(&content, &builds.versions[1].id.clone()).unwrap();
            let settled = |value: i64| {
                let mut side = new.clone();
                side.set_fact(&content, "karma", &[Fact::Value(Value::Int(value))]).unwrap();
                let on = builds.preset_of(&content, &side, "light_dark").map(|p| p.id.clone());
                let taken = builds.settle(&content, &mut side);
                (on, taken, number(&content, &side, "karma").unwrap(), number(&content, &side, "hp").unwrap())
            };
            let took = |preset: &str, was: i64| vec![Taken { choice: "light_dark".into(), preset: preset.into(), was: Some(was) }];
            assert_eq!(settled(line - 1), (Some("dark".into()), took("dark", line - 1), dark.0, dark.1), "{game}");
            assert_eq!(settled(line), (Some("light".into()), took("light", line), 1000, 1000), "{game}");
            assert_eq!(settled(100), (Some("dark".into()), took("dark", 100), dark.0, dark.1), "{game}");
            // A preset's own value: nothing to take.
            assert_eq!(settled(1000), (Some("light".into()), Vec::new(), 1000, 1000), "{game}");
            // A dark value with a light HP (a file's): the dark preset's HP.
            let mut mixed = new.clone();
            mixed.set_fact(&content, "karma", &[Fact::Value(Value::Int(dark.0))]).unwrap();
            assert_eq!(builds.settle(&content, &mut mixed), took("dark", dark.0), "{game}");
            assert_eq!(number(&content, &mixed, "hp"), Some(997), "{game}");
            // A side nothing was chosen for (the rules' default, a fresh
            // save's 500) is light's, and as built it is the light preset.
            let mut fresh = Side::fresh(&content, game).unwrap();
            let mut changed = builds.as_built(&content, &mut fresh);
            changed.sort();
            assert_eq!((changed, number(&content, &fresh, "karma")), (vec!["karma".to_string()], Some(1000)), "{game}");
        }
    }

    /// The choice is MegaMan's alone: a build of an EXE5 team navi has no
    /// light or dark (its light/dark value and its base HP the rules'
    /// defaults: a team navi's are its own block's and its story's), is
    /// ready, and is neither preset's; back on MegaMan it is light again.
    #[test]
    fn a_team_navi_is_neither_light_nor_dark() {
        let content = exe5_content();
        let builds = Builds::of(&content, "exe5").unwrap();
        let megaman = crate::first_navi(&content, "exe5").unwrap();
        let protoman = ids::navi(&content, "exe5", "protoman").unwrap();
        let mut side = builds.new_build(&content, "protoman").unwrap();
        builds.take_preset(&content, &mut side, "light_dark", "dark").unwrap();
        builds.set_navi(&content, &mut side, protoman).unwrap();
        assert_eq!(side.navi(&content), protoman);
        assert!(builds.choices_of(&content, &side).is_empty() && builds.preset_of(&content, &side, "light_dark").is_none());
        assert!(side.facts.is_default(&content, "karma") && side.facts.is_default(&content, "hp"));
        assert!(builds.take_preset(&content, &mut side, "light_dark", "dark").unwrap_err().contains("no choice of the side's navi"));
        assert_eq!(side.level(&content), crate::play_level(&content, protoman));
        // (Its version stays: the team's souls and Giga chips are the save's.)
        assert_eq!(builds.version_of(&content, &side), Some("protoman"));
        let mut checked = Checked::of(&content, "exe5", &side);
        assert_eq!(checked.problems, Vec::new());
        assert_eq!(checked.face(), Some(Face::Navi(protoman)));
        let o = checked.overview(&content, &side).unwrap();
        assert_eq!(o.hp.added(), 0);
        assert_eq!(builds.settle(&content, &mut side), Vec::new(), "nothing of the choice to settle");
        builds.set_navi(&content, &mut side, megaman).unwrap();
        assert_eq!(builds.preset_of(&content, &side, "light_dark").map(|p| p.id.as_str()), Some("light"));
        assert_eq!((number(&content, &side, "karma"), number(&content, &side, "hp")), (Some(1000), Some(1000)));
        // EXE6: a link navi's build has no Crosses, and MegaMan's has his
        // version's five again.
        let six = exe6_content();
        let builds = Builds::of(&six, "exe6").unwrap();
        let mut side = builds.new_build(&six, "falzar").unwrap();
        builds.set_navi(&six, &mut side, ids::navi(&six, "exe6", "elecman").unwrap()).unwrap();
        assert!(side.facts.form_list(&six).is_empty());
        assert_eq!(Checked::of(&six, "exe6", &side).problems, Vec::new());
        builds.set_navi(&six, &mut side, crate::first_navi(&six, "exe6").unwrap()).unwrap();
        assert_eq!(side.facts.form_list(&six).len(), 5);
        // What a build has installed stays through a change of navi, and the
        // rules refuse a NaviCust on a navi that compiles none: a screen that
        // keeps MegaMan's programs for a link navi's build keeps them aside,
        // or the build isn't ready. (Patch cards kept do nothing, and are
        // nothing wrong.)
        let undersht = ids::entry(&six, "exe6", "navicust_programs", "undersht").unwrap();
        set_navicust(&six, &mut side, &[PlacedProgram { program: undersht, color: 0, x: 1, y: 3, rotation: 0, compressed: false }], 2);
        side.set_fact(&six, "patch_cards", &patch_cards(&six, "exe6", "canodumb")).unwrap();
        builds.set_navi(&six, &mut side, ids::navi(&six, "exe6", "elecman").unwrap()).unwrap();
        let checked = Checked::of(&six, "exe6", &side);
        let got: Vec<(&Wrong, Option<&str>)> = checked.problems.iter().map(|p| (&p.what, p.field.as_deref())).collect();
        assert_eq!(got, [(&Wrong::NotTaken, Some("navicust_programs"))]);
    }
}
