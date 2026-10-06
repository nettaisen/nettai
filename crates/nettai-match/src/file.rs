//! The match file: a [`Match`] as TOML (docs/frontend.md §6). It names its
//! game once, at the top; everything else is a name in that game's
//! namespace (`cannon`, `megaman`: `crate::ids`), and a name the game
//! hasn't is said as such ("no chip "x" in exe6"). There is no way to name
//! another game's.
//!
//! ```toml
//! game = "exe6"                       # the match's game: everything below is its
//! seed = 42                          # optional: the setup's and battle's seed
//!
//! [arena]
//! stage = "netbattle-43"
//! background = "lans-hp"             # optional: else the stage's own
//! later = [                          # optional: the set's later rounds (else the first's)
//!     { stage = "netbattle-12", background = "undernet" },
//!     { stage = "netbattle-7" },
//! ]
//!
//! [left]                             # you, side 0; then [right]
//! navi = "megaman"
//! level = 7                          # optional: the navi code's level, 0-14 (else a link navi's 0, MegaMan none)
//! cards = [{ card = "canodumb" }, { card = "shadow", on = false }]
//! version = "falzar"                 # the side's facts: what its game's rules take, each under its setup field's
//! crosses = ["heatcross", "spoutcross"]   # name (crate::facts). EXE6's: version (gregar or falzar) and crosses (up
//! beast_out = false                  # to five, of either version; [] none), which a side states (none is assumed);
//! bug_frags = 9                      # beast_out (else unlocked), bug_frags (else 0)
//! hp = 1000                          # what the save brings to the stats: the base HP (else 100), the Regular
//! reg_up = 50                        # memory (else the fresh stats' 4), the sun (else none); the rules derive the rest
//! folder = [                         # 30 [chip, code] pairs ([] an empty entry, while it's being made)
//!     ["cannon", "A"],
//!     ["cannon", "A"],
//! ]
//! regular = 4                        # optional: the Regular chip's entry, counting from 0
//! tags = [5, 6]                      # optional: the tag chips' entries
//!
//! [left.sp_times]                    # optional: SP navi deletion times, mm:ss.cc (else the fastest)
//! "sp/heatman" = "00:12.34"
//!
//! [left.navicust]                    # optional: the NaviCust, which the rules compile
//! expansions = 2                     # optional: the board's (else the largest)
//! programs = [                       # in the list's order; x, y the center on the 7x7 grid
//!     { program = "suprarmr", color = "red", x = 3, y = 3, rotation = 1, compressed = true },
//! ]
//!
//! # An EXE5 side's facts ([left] of game = "exe5"; a fact left out is its rules' default):
//! level = 3                          # a team navi's level, 0 to 6: its damage rows' (its HP is the save's `hp`)
//! karma = 100                        # the light/dark value (default 500, a fresh save's; dark under 470)
//! souls = ["protosoul"]              # the souls it has, either version's (default: every soul)
//! soul_unison = false                # no soul button (the save's event flag 0; default true)
//! chaos_unison = false               # no Chaos Unison (the save's event flag 0x236; default true)
//!
//! [left.auto_battle]               # optional: what a navi in auto battle plays from the side's save, whole (none: nothing learned)
//! first = [{}, {}, {}]               # the data's places 1 to 3: each a chip, a pattern record's number, 0 or {} (an empty place)
//! standard = ["sword", "sword", {}, ...]   # places 4 to 27, all 24: the game writes its player's most used standard chips
//! mega = ["protoman", {}, {}, {}, {}]      # places 28 to 32: mega chips
//! giga = "crossdiv"                  # place 33: a giga chip
//! patterns = [1, {}, {}, {}, {}, {}, {}, {}]   # places 34 to 41: pattern records, by number (1 to 8)
//! program_advance = {}               # place 42: a program advance
//! records = [                        # the eight pattern records: a place from its target, five chip places, a score
//!     { dx = -2, dy = 0, chips = ["sword", "wideswrd", {}, {}, {}], score = 10 },
//!     { dx = 0, dy = 0, chips = [0, 0, 0, 0, 0], score = 0 },   # (and six more)
//! ]
//! ```

use crate::auto_battle::{self, ChipPlace, AutoBattle, Entry, Record};
use crate::facts::Stated;
use crate::{Arena, Facts, Folder, Match, Place, Side, ids};
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::folder::FOLDER_SIZE;
use nettai_battle::custom::FolderChip;
use nettai_battle::rules::Fact;
use nettai_content_api::{FieldType, Value};
use nettai_battle::navicust::{NaviCust, PlacedProgram};
use nettai_battle::patch_cards::InstalledCard;
use nettai_battle::setup::SpTimes;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchFile {
    pub game: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    pub arena: ArenaFile,
    pub left: SideFile,
    pub right: SideFile,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArenaFile {
    pub stage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub later: Option<Vec<PlaceFile>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceFile {
    pub stage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

/// A side as a file states it: the engine's parts under their own keys, and
/// every other key a fact of the game's rules, under its setup field's name
/// (`crate::facts`: a key no system declares is refused when the side is
/// resolved, with the facts the game takes).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SideFile {
    pub navi: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// The facts, in the order the file states them (written in the rules'
    /// systems' order).
    #[serde(flatten)]
    pub facts: FactsFile,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<CardFile>,
    /// The folder's entries, each `[chip, code]` (`[]` empty, while it is
    /// being made).
    pub folder: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<[u8; 2]>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sp_times: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navicust: Option<NaviCustFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_battle: Option<AutoBattleFile>,
}

/// A side's facts as a file states them: each a key and its value, in
/// order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FactsFile(pub Vec<(String, toml::Value)>);

impl Serialize for FactsFile {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for FactsFile {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<FactsFile, D::Error> {
        struct Entries;
        impl<'de> serde::de::Visitor<'de> for Entries {
            type Value = FactsFile;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a side's facts, each a key and its value")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<FactsFile, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = map.next_entry::<String, toml::Value>()? {
                    out.push(entry);
                }
                Ok(FactsFile(out))
            }
        }
        deserializer.deserialize_map(Entries)
    }
}

/// A player's auto battle data (`crate::auto_battle`), whole: its six
/// lists by name, each every one of its places in order (an entry a chip's
/// name, a number 1 to 8 for that pattern record, `0` for a place holding
/// 0, or `{}` for an empty one; the two lists of one place are that entry
/// alone), and its eight pattern records.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoBattleFile {
    pub first: Vec<toml::Value>,
    pub standard: Vec<toml::Value>,
    pub mega: Vec<toml::Value>,
    pub giga: toml::Value,
    pub patterns: Vec<toml::Value>,
    pub program_advance: toml::Value,
    pub records: Vec<RecordFile>,
}

impl AutoBattleFile {
    /// The entries the file gives the list named `name`
    /// (`auto_battle::LISTS`), in its places' order.
    pub fn entries(&self, name: &str) -> &[toml::Value] {
        match name {
            "first" => &self.first,
            "standard" => &self.standard,
            "mega" => &self.mega,
            "giga" => std::slice::from_ref(&self.giga),
            "patterns" => &self.patterns,
            "program_advance" => std::slice::from_ref(&self.program_advance),
            _ => &[],
        }
    }
}

/// A pattern record: its place from its target (`dx` columns toward the
/// auto-battling navi's enemies, `dy` rows down), its five chip places (each a
/// chip's name, `0` or `{}`, an empty one) and its score.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordFile {
    pub dx: i8,
    pub dy: i8,
    pub chips: Vec<toml::Value>,
    pub score: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviCustFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expansions: Option<u8>,
    #[serde(default)]
    pub programs: Vec<ProgramFile>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramFile {
    pub program: String,
    pub color: String,
    pub x: u8,
    pub y: u8,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub compressed: bool,
}

fn is_zero(n: &u8) -> bool {
    *n == 0
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardFile {
    pub card: String,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub on: bool,
}

fn yes() -> bool {
    true
}

fn is_yes(b: &bool) -> bool {
    *b
}

/// A folder entry as a file writes it: the chip's name and its code.
pub fn chip_entry(content: &Content, c: FolderChip) -> [String; 2] {
    [ids::local(&content.defs.chip(c.id).key).to_string(), c.code.letter().to_string()]
}

/// What names nothing in `game`: "no chip "x" in exe6", and the game's
/// names of that kind when they are few enough to read.
fn unknown(what: &str, name: &str, game: &str, have: &[&str]) -> String {
    if have.is_empty() || have.len() > 40 {
        format!("no {what} {name:?} in {game}")
    } else {
        format!("no {what} {name:?} in {game} ({game}'s are {})", have.join(", "))
    }
}

/// The local names of `game`'s definitions among `keys`.
fn names_in<'k>(content: &Content, game: &str, keys: impl Iterator<Item = &'k str>) -> Vec<&'k str> {
    keys.filter(|k| ids::in_game(content, game, k)).map(ids::local).collect()
}

/// Each name a file says, resolved in its game: the match, or every
/// problem with it.
pub fn resolve(content: &Content, f: &MatchFile) -> Result<Match, Vec<String>> {
    let mut problems = Vec::new();
    let games = ids::games(content);
    if !games.contains(&f.game) {
        return Err(vec![format!("no game {:?} (the content's are {})", f.game, games.join(", "))]);
    }
    let arena = resolve_arena(content, &f.game, &f.arena, &mut problems);
    let Some(arena) = arena else { return Err(problems) };
    let left = resolve_side(content, &arena.game, &f.left, "left", &mut problems);
    let right = resolve_side(content, &arena.game, &f.right, "right", &mut problems);
    match (left, right) {
        (Some(left), Some(right)) if problems.is_empty() => Ok(Match { seed: f.seed, arena, sides: [left, right] }),
        _ => Err(problems),
    }
}

fn resolve_place(content: &Content, game: &str, stage: &str, background: &Option<String>, at: &str, problems: &mut Vec<String>) -> Option<Place> {
    let stage = match ids::stage(content, game, stage) {
        Some(s) => Some(s),
        None => {
            problems.push(format!("{at}: {}", unknown("stage", stage, game, &[])));
            None
        }
    };
    if let Some(b) = background
        && crate::background(content, game, b).is_none()
    {
        problems.push(crate::no_background(at, game, b));
    }
    Some(Place { stage: stage?, background: background.clone() })
}

/// A file's arena, of `game`.
pub fn resolve_arena(content: &Content, game: &str, a: &ArenaFile, problems: &mut Vec<String>) -> Option<Arena> {
    let first = resolve_place(content, game, &a.stage, &a.background, "arena", problems);
    let later = match &a.later {
        None => first.clone().map(|p| [p.clone(), p]),
        Some(list) if list.len() == 2 => {
            let l: Vec<Option<Place>> = list
                .iter()
                .enumerate()
                .map(|(i, p)| resolve_place(content, game, &p.stage, &p.background, &format!("arena: later round {}", i + 2), problems))
                .collect();
            Some([l[0].clone()?, l[1].clone()?])
        }
        Some(list) => {
            problems.push(format!("arena: later names {} places; a set's later rounds are two", list.len()));
            None
        }
    };
    Some(Arena { game: game.to_string(), first: first?, later: later? })
}

/// A file's side of a match of `game`, each name the game's.
pub fn resolve_side(content: &Content, game: &str, s: &SideFile, at: &str, problems: &mut Vec<String>) -> Option<Side> {
    let start = problems.len();
    let mut say = |p: String| problems.push(format!("{at}: {p}"));
    let navi = ids::navi(content, game, &s.navi);
    if navi.is_none() {
        let have = names_in(content, game, content.defs.navis.iter().map(|n| n.key.as_str()));
        say(unknown("navi", &s.navi, game, &have));
    }
    // The facts: each key that is none of the side's own parts, a field of
    // a system's setup, its value read by the field's type. A fact the file
    // leaves out is the rules' default (an enum without one, unstated: the
    // checks say a round needs it).
    let mut facts = Facts::defaults(content);
    for (key, value) in &s.facts.0 {
        let Some(field) = crate::facts::field(content, key) else {
            say(crate::facts::no_field(content, key));
            continue;
        };
        match fact_values(content, game, field.ty, value).and_then(|values| {
            let values: Vec<Fact> = values.into_iter().map(Fact::Value).collect();
            facts.set(content, key, &values)
        }) {
            Ok(()) => {}
            Err(e) => say(format!("{key}: {e}")),
        }
    }
    let mut cards = Vec::new();
    for c in &s.cards {
        match ids::patch_card(content, game, &c.card) {
            Some(card) => cards.push(InstalledCard { card, enabled: c.on }),
            None => say(unknown("patch card", &c.card, game, &[])),
        }
    }
    let folder = resolve_folder(content, game, s, &mut say);
    let navicust = match &s.navicust {
        None => None,
        Some(n) => {
            let mut parts = Vec::with_capacity(n.programs.len());
            for (i, p) in n.programs.iter().enumerate() {
                let Some(program) = ids::navicust_program(content, game, &p.program) else {
                    say(format!("navicust program {}: {}", i + 1, unknown("NaviCust program", &p.program, game, &[])));
                    continue;
                };
                let def = content.defs.navicust_program(program);
                let Some(color) = def.colors.iter().position(|c| *c == p.color) else {
                    say(format!("navicust program {}: {} comes in {}, not {:?}", i + 1, p.program, def.colors.join(", "), p.color));
                    continue;
                };
                parts.push(PlacedProgram { program, color: color as u8, x: p.x, y: p.y, rotation: p.rotation, compressed: p.compressed });
            }
            let expansions = n.expansions.unwrap_or_else(|| crate::navicust_rules(content).boards.len().saturating_sub(1) as u8);
            match NaviCust::new(&parts, expansions) {
                Ok(n) => Some(n),
                Err(e) => {
                    say(e);
                    None
                }
            }
        }
    };
    // The SP navis' deletion times, by the rules' slot names.
    let slots = crate::sp_slots(content);
    let mut sp_times = SpTimes::default();
    for (name, time) in &s.sp_times {
        let Some(i) = slots.iter().position(|x| x == name) else {
            say(format!("sp_times: no SP navi slot {name:?} (the rules' are {})", slots.join(", ")));
            continue;
        };
        match crate::sp_times::parse(time) {
            Ok(frames) => sp_times.0[i] = frames,
            Err(e) => say(format!("sp_times: {name}: {e}")),
        }
    }
    let auto_battle = s.auto_battle.as_ref().map(|c| resolve_auto_battle(content, game, c, &mut say)).unwrap_or_default();
    let navi = navi?;
    // (No level: a link navi's 0, MegaMan's none; the checks hold it.)
    let navi_level = s.level.or_else(|| crate::default_navi_level(content, navi));
    if problems.len() > start {
        return None;
    }
    Some(Side { navi, folder: folder?, cards, navi_level, sp_times, navicust, auto_battle, facts })
}

/// A fact's value as a file states it, read by the field's type `ty`: a
/// flag, a whole number that fits the type, an enum's variant by its name,
/// a definition by its name in `game` ("" none), an array's elements from
/// the first.
fn fact_values(content: &Content, game: &str, ty: &FieldType, v: &toml::Value) -> Result<Vec<Value>, String> {
    let one = |ty: &FieldType, v: &toml::Value| -> Result<Value, String> {
        match ty {
            FieldType::Bool => v.as_bool().map(Value::Bool).ok_or_else(|| format!("{v} is neither true nor false")),
            // (A number past its type is the facts' writer's to refuse.)
            ty if crate::facts::range(ty).is_some() => v.as_integer().map(Value::Int).ok_or_else(|| format!("{v} is no whole number")),
            FieldType::Enum(names) => match v.as_str().map(|n| names.iter().position(|x| x == n)) {
                Some(Some(i)) => Ok(Value::Int(i as i64)),
                _ => Err(format!("no {v} ({})", names.join(" or "))),
            },
            FieldType::Ref(registry, _) => match v.as_str() {
                Some("") => Ok(Value::Nil),
                Some(name) => {
                    ids::handle_of(content, game, *registry, name).map(|h| Value::Def(*registry, h)).ok_or_else(|| unknown(&registry.to_string(), name, game, &[]))
                }
                None => Err(format!("{v} is no {registry}'s name")),
            },
            other => Err(format!("a match states no {other:?}")),
        }
    };
    match (ty, v) {
        (FieldType::Array(elem, n), toml::Value::Array(items)) if items.len() <= *n as usize => items.iter().map(|item| one(elem, item)).collect(),
        (FieldType::Array(_, n), toml::Value::Array(items)) => Err(format!("{} entries, it holds {n}", items.len())),
        (FieldType::Array(..), _) => Err(format!("{v} is no list")),
        (ty, v) => Ok(vec![one(ty, v)?]),
    }
}

/// A fact's value as a file writes it: a flag, a number, an enum's
/// variant's name, a definition's name, a list's entries (a list of
/// definitions up to its last one; a hole in it, ""). None: nothing to
/// write (an enum nothing states, no definition).
fn fact_toml(content: &Content, value: &Stated) -> Option<toml::Value> {
    Some(match value {
        Stated::Flag(b) => toml::Value::Boolean(*b),
        Stated::Number(n) => toml::Value::Integer(*n),
        Stated::Variant(name) => toml::Value::String(name.clone()?),
        Stated::Def(registry, h) => toml::Value::String(ids::local(ids::key_of(content, *registry, (*h)?)?).to_string()),
        Stated::List(items) => {
            let last = items.iter().rposition(|v| !matches!(v, Stated::Def(_, None))).map_or(0, |i| i + 1);
            toml::Value::Array(items[..last].iter().map(|v| fact_toml(content, v).unwrap_or_else(|| toml::Value::String(String::new()))).collect())
        }
        Stated::Unlisted | Stated::Other => return None,
    })
}

/// A file's auto battle data: each list's entries into its places, and
/// its records. What is no entry, what names nothing in `game`, and a list
/// or a record that doesn't state each of its places are said; such an
/// entry is left empty.
fn resolve_auto_battle(content: &Content, game: &str, c: &AutoBattleFile, say: &mut impl FnMut(String)) -> AutoBattle {
    let mut out = AutoBattle::default();
    let chip = |at: &str, n: &str, say: &mut dyn FnMut(String)| {
        let c = ids::chip(content, game, n);
        if c.is_none() {
            say(format!("{at}: {}", unknown("chip", n, game, &[])));
        }
        c
    };
    for list in &auto_battle::LISTS {
        let entries = c.entries(list.name);
        if entries.len() != list.len {
            say(format!("auto_battle: {} states {} places; it has {}, each stated ({{}} an empty one)", list.name, entries.len(), list.len));
        }
        for (i, e) in entries.iter().take(list.len).enumerate() {
            let at = if list.len == 1 { format!("auto_battle: {}", list.name) } else { format!("auto_battle: {} entry {}", list.name, i + 1) };
            out.places[list.start + i] = match e {
                toml::Value::String(n) => chip(&at, n, say).map_or(Entry::Empty, Entry::Chip),
                // (A number: a pattern record's, from 1; 0 is the place
                // that holds 0.)
                toml::Value::Integer(0) => Entry::Zero,
                toml::Value::Integer(n) if (1..=auto_battle::RECORDS as i64).contains(n) => Entry::Pattern((n - 1) as u8),
                toml::Value::Table(t) if t.is_empty() => Entry::Empty,
                other => {
                    say(format!(
                        "{at}: {other} is no entry (a chip's name, a pattern record's number 1 to {}, 0 for a place holding no chip, or {{}} for an empty place)",
                        auto_battle::RECORDS
                    ));
                    Entry::Empty
                }
            };
        }
    }
    if c.records.len() != auto_battle::RECORDS {
        say(format!("auto_battle: records states {} pattern records; the data has {}, each stated", c.records.len(), auto_battle::RECORDS));
    }
    for (n, r) in c.records.iter().take(auto_battle::RECORDS).enumerate() {
        let at = format!("auto_battle: record {}", n + 1);
        if r.chips.len() != auto_battle::RECORD_CHIPS {
            say(format!("{at}: chips states {} places; a record has {}, each stated ({{}} an empty one)", r.chips.len(), auto_battle::RECORD_CHIPS));
        }
        let mut chips = [ChipPlace::Empty; auto_battle::RECORD_CHIPS];
        for (place, e) in chips.iter_mut().zip(&r.chips) {
            *place = match e {
                toml::Value::String(n) => chip(&at, n, say).map_or(ChipPlace::Empty, ChipPlace::Chip),
                toml::Value::Integer(0) => ChipPlace::Zero,
                toml::Value::Table(t) if t.is_empty() => ChipPlace::Empty,
                other => {
                    say(format!("{at}: {other} is no chip place (a chip's name, 0, or {{}} for an empty place)"));
                    ChipPlace::Empty
                }
            };
        }
        out.records[n] = Record { dx: r.dx, dy: r.dy, chips, score: r.score };
    }
    out
}

/// A file side's folder: up to 30 entries, an empty one `[]` and those past
/// the last given empty (a folder being made; the checks say it isn't
/// whole), and its Regular and tag chips.
fn resolve_folder(content: &Content, game: &str, s: &SideFile, say: &mut impl FnMut(String)) -> Option<Folder> {
    if s.folder.len() > FOLDER_SIZE {
        say(format!("the folder has {} entries; a folder is {FOLDER_SIZE}", s.folder.len()));
        return None;
    }
    let mut folder = Folder { regular: s.regular, tags: s.tags.map(|[a, b]| (a, b)), ..Folder::EMPTY };
    let mut ok = true;
    for (i, entry) in s.folder.iter().enumerate() {
        let (name, code) = match entry.as_slice() {
            [] => continue,
            [name, code] => (name, code),
            _ => {
                say(format!("folder entry {i}: {entry:?} is not [chip, code] ([] an empty entry)"));
                ok = false;
                continue;
            }
        };
        let mut letters = code.chars();
        let Some(code) = letters.next().and_then(ChipCode::from_letter).filter(|_| letters.next().is_none()) else {
            say(format!("folder entry {i}: {code:?} is no code (A-Z or *)"));
            ok = false;
            continue;
        };
        match ids::chip(content, game, name) {
            Some(id) => folder.chips[i] = Some(FolderChip::new(id, code)),
            None => {
                say(format!("folder entry {i}: {}", unknown("chip", name, game, &[])));
                ok = false;
            }
        }
    }
    ok.then_some(folder)
}

/// A side as a file writes it, each name its game's.
pub fn side_file(content: &Content, s: &Side) -> SideFile {
    let name = |key: &str| ids::local(key).to_string();
    SideFile {
        navi: name(&content.defs.navi(s.navi).key),
        // (The navi's default level is left out.)
        level: s.navi_level.filter(|_| s.navi_level != crate::default_navi_level(content, s.navi)),
        // The facts that aren't the rules' defaults, in the rules' order.
        facts: FactsFile(
            crate::facts::fields(content)
                .into_iter()
                .filter(|f| !s.facts.is_default(content, f.name))
                .filter_map(|f| Some((f.name.to_string(), fact_toml(content, &s.facts.get(content, f.name)?)?)))
                .collect(),
        ),
        sp_times: crate::sp_times::named(crate::sp_slots(content), &s.sp_times)
            .into_iter()
            .map(|(name, frames)| (name, crate::sp_times::format(frames)))
            .collect(),
        cards: s.cards.iter().map(|c| CardFile { card: name(&content.defs.patch_card(c.card).key), on: c.enabled }).collect(),
        // An empty entry is [], and those after the last chip are left off.
        folder: {
            let last = s.folder.chips.iter().rposition(|c| c.is_some()).map_or(0, |i| i + 1);
            s.folder.chips[..last].iter().map(|c| c.map_or(Vec::new(), |c| chip_entry(content, c).to_vec())).collect()
        },
        regular: s.folder.regular,
        tags: s.folder.tags.map(|(a, b)| [a, b]),
        // (A block nothing has written is left out; any other is stated
        // whole.)
        auto_battle: (!s.auto_battle.is_blank()).then(|| {
            let chip = |c: nettai_content_api::ChipHandle| toml::Value::String(name(&content.defs.chip(c).key));
            let empty = || toml::Value::Table(Default::default());
            let entry = |e: &Entry| match *e {
                Entry::Empty => empty(),
                Entry::Zero => toml::Value::Integer(0),
                Entry::Chip(c) => chip(c),
                Entry::Pattern(n) => toml::Value::Integer(n as i64 + 1),
            };
            let list = |i: usize| -> Vec<toml::Value> { s.auto_battle.list(&auto_battle::LISTS[i]).iter().map(entry).collect() };
            let one = |i: usize| list(i).pop().unwrap_or_else(empty);
            let record = |r: &Record| RecordFile {
                dx: r.dx,
                dy: r.dy,
                chips: r
                    .chips
                    .iter()
                    .map(|c| match *c {
                        ChipPlace::Empty => empty(),
                        ChipPlace::Zero => toml::Value::Integer(0),
                        ChipPlace::Chip(c) => chip(c),
                    })
                    .collect(),
                score: r.score,
            };
            AutoBattleFile {
                first: list(0),
                standard: list(1),
                mega: list(2),
                giga: one(3),
                patterns: list(4),
                program_advance: one(5),
                records: s.auto_battle.records.iter().map(record).collect(),
            }
        }),
        navicust: s.navicust.map(|n| NaviCustFile {
            expansions: Some(n.expansions),
            programs: n
                .iter()
                .map(|p| {
                    let def = content.defs.navicust_program(p.program);
                    ProgramFile {
                        program: name(&def.key),
                        color: def.colors.get(p.color as usize).cloned().unwrap_or_default(),
                        x: p.x,
                        y: p.y,
                        rotation: p.rotation,
                        compressed: p.compressed,
                    }
                })
                .collect(),
        }),
    }
}

/// A match's arena as a file writes it (its places; the game is the
/// file's own key).
pub fn arena_file(content: &Content, a: &Arena) -> ArenaFile {
    let place = |p: &Place| PlaceFile { stage: ids::local(&content.defs.stage(p.stage).key).to_string(), background: p.background.clone() };
    let first = place(&a.first);
    let later = (a.later != [a.first.clone(), a.first.clone()]).then(|| a.later.iter().map(place).collect());
    ArenaFile { stage: first.stage, background: first.background, later }
}

/// A match as a file.
pub fn to_file(content: &Content, m: &Match) -> MatchFile {
    MatchFile {
        game: m.arena.game.clone(),
        seed: m.seed,
        arena: arena_file(content, &m.arena),
        left: side_file(content, &m.sides[0]),
        right: side_file(content, &m.sides[1]),
    }
}

/// A match file's text, resolved against `content` and checked
/// (`crate::check`): the match, or every problem with it.
pub fn parse(content: &std::sync::Arc<Content>, text: &str) -> Result<Match, Vec<String>> {
    let f: MatchFile = toml::from_str(text).map_err(|e| vec![e.to_string()])?;
    let m = resolve(content, &f)?;
    let problems = crate::check_match(content, &m);
    if problems.is_empty() { Ok(m) } else { Err(problems) }
}

/// The game a match file's text names (`game = "exe6"`), read before the
/// content it needs is loaded: or why it names none.
pub fn game_of(text: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct Game {
        game: Option<String>,
    }
    let g: Game = toml::from_str(text).map_err(|e| e.to_string())?;
    g.game.ok_or_else(|| "the match file names no game (game = \"exe6\" at its top)".into())
}

/// A match file's text.
pub fn write(content: &Content, m: &Match) -> String {
    let file = to_file(content, m);
    let body = toml::to_string_pretty(&file).expect("a match file serializes");
    format!("# A nettai match (docs/frontend.md §6): play it with `nettai-demo --match FILE`.\n\n{}", tidy(&body, &file))
}

/// The most entries of a fact's list a file writes on one line.
const SHORT_LIST: usize = 6;

/// `list`'s entries each on a line of its own.
fn a_line_each(list: &mut toml_edit::Array) {
    for entry in list.iter_mut() {
        entry.decor_mut().set_prefix("\n    ");
    }
    list.set_trailing_comma(true);
    list.set_trailing("\n");
}

/// An auto-battling navi's entry or a record's chip place as a file writes it: a
/// chip's name, a number (a pattern record's, or 0) or `{}`.
fn play_item(play: &toml::Value) -> toml_edit::Value {
    match play {
        toml::Value::Table(_) => toml_edit::InlineTable::new().into(),
        toml::Value::Integer(n) => (*n).into(),
        other => other.as_str().unwrap_or_default().into(),
    }
}

/// `items` laid out: a few on one line, more four to a line.
fn laid_out(mut items: toml_edit::Array) -> toml_edit::Array {
    if items.len() > auto_battle::RECORDS {
        for (i, item) in items.iter_mut().enumerate() {
            item.decor_mut().set_prefix(if i % 4 == 0 { "\n    " } else { " " });
        }
        items.set_trailing_comma(true);
        items.set_trailing("\n");
    }
    items
}

/// `body`, the pretty printer's text of `file`, as a person would lay it
/// out: a fact's short list on its line (`crosses = ["heatcross",
/// "eleccross"]`; a longer one an entry a line, as the printer has it),
/// each folder entry (`["cannon", "A"]`) on a line of its own (the
/// printer spreads every element of a nested array over lines), and the
/// auto-battling navi's data in the block's order, its lists on a line or four
/// entries to one, its records a line each.
fn tidy(body: &str, file: &MatchFile) -> String {
    let mut doc: toml_edit::DocumentMut = body.parse().expect("a match file parses");
    for (side, of) in [("left", &file.left), ("right", &file.right)] {
        for (key, _) in &of.facts.0 {
            let list = doc.get_mut(side).and_then(|s| s.get_mut(key)).and_then(|f| f.as_array_mut());
            if let Some(list) = list.filter(|l| l.len() <= SHORT_LIST) {
                list.set_trailing_comma(false);
                list.set_trailing("");
                for (i, v) in list.iter_mut().enumerate() {
                    v.decor_mut().set_prefix(if i == 0 { "" } else { " " });
                    v.decor_mut().set_suffix("");
                }
            }
        }
        if let Some(folder) = doc.get_mut(side).and_then(|s| s.get_mut("folder")).and_then(|f| f.as_array_mut()) {
            for entry in folder.iter_mut() {
                if let Some(pair) = entry.as_array_mut() {
                    pair.set_trailing_comma(false);
                    pair.set_trailing("");
                    for (i, v) in pair.iter_mut().enumerate() {
                        v.decor_mut().set_prefix(if i == 0 { "" } else { " " });
                        v.decor_mut().set_suffix("");
                    }
                }
            }
            a_line_each(folder);
        }
        // (The table written anew over the printer's, which makes tables of
        // the records and of the entries that are tables.)
        let (Some(data), Some(item)) = (&of.auto_battle, doc.get_mut(side).and_then(|s| s.get_mut("auto_battle"))) else {
            continue;
        };
        let mut table = toml_edit::Table::new();
        if let Some(position) = item.as_table().and_then(|t| t.position()) {
            table.set_position(position);
        }
        for list in &auto_battle::LISTS {
            match data.entries(list.name) {
                [entry] if list.len == 1 => table.insert(list.name, toml_edit::value(play_item(entry))),
                entries => table.insert(list.name, toml_edit::value(laid_out(entries.iter().map(play_item).collect()))),
            };
        }
        let mut records: toml_edit::Array = data
            .records
            .iter()
            .map(|r| {
                let mut record = toml_edit::InlineTable::new();
                record.insert("dx", (r.dx as i64).into());
                record.insert("dy", (r.dy as i64).into());
                record.insert("chips", r.chips.iter().map(play_item).collect::<toml_edit::Array>().into());
                record.insert("score", (r.score as i64).into());
                toml_edit::Value::from(record)
            })
            .collect();
        a_line_each(&mut records);
        table.insert("records", toml_edit::value(records));
        *item = toml_edit::Item::Table(table);
    }
    doc.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::exe6_content;

    /// Live play's random match, written as a match file and read back, is the same
    /// match, and starts the same round.
    #[test]
    fn a_random_match_writes_and_reads_back() {
        let content = exe6_content();
        for seed in 0..6 {
            let m = crate::pick::live(&content, "exe6", seed, None).unwrap();
            let text = write(&content, &m);
            let back = parse(&content, &text).unwrap_or_else(|e| panic!("seed {seed}: {e:?}\n{text}"));
            assert_eq!(back, m, "seed {seed}:\n{text}");
            assert_eq!(format!("{:?}", back.round(&content, seed)), format!("{:?}", m.round(&content, seed)));
        }
        let text = write(&content, &crate::pick::live(&content, "exe6", 3, None).unwrap());
        for line in ["game = \"exe6\"", "[arena]", "[left]", "folder = [\n    [\"", "\", \"", "[left.navicust]", "expansions = 2", "programs = []"] {
            assert!(text.contains(line), "{line}:\n{text}");
        }
        // (No stats: a side states none.)
        assert!(!text.contains("stats"), "{text}");
        // Every name is the game's own, written once with the game.
        assert!(!text.contains("exe6:") && !text.contains("ruleset"), "{text}");
        assert_eq!(game_of(&text).unwrap(), "exe6");
        assert!(game_of("[arena]\nstage = \"x\"\n").is_err());
    }

    /// A side's auto battle data (EXE5's) writes and reads back, whole:
    /// every place of its six lists (chips by name, a pattern by its
    /// record's number from 1, a 0, an empty place as `{}`) and its eight
    /// records;
    /// the round sends it; and what a file gets wrong is said, with where it
    /// is.
    #[test]
    fn auto_battle_data_writes_and_reads_back() {
        use auto_battle::{GIGA, MEGA, PATTERNS, PROGRAM_ADVANCE, STANDARD};
        use nettai_battle::auto_battle::{AutoBattleEntry, PatternRecord};
        let content = crate::testing::exe5_content();
        let mut m = crate::pick::live(&content, "exe5", 2, None).unwrap();
        let chip = |name: &str| ids::chip(&content, "exe5", name).unwrap();
        let mut data = AutoBattle::default();
        data.places[0] = Entry::Chip(chip("areagrab"));
        for (i, name) in ["sword", "sword", "sword", "sword", "cannon"].into_iter().enumerate() {
            data.places[STANDARD.start + i] = Entry::Chip(chip(name));
        }
        // (A 0 after them; an empty place before the second mega chip.)
        data.places[STANDARD.start + 5] = Entry::Zero;
        data.places[MEGA.start + 1] = Entry::Chip(chip("protoman"));
        data.places[GIGA.start] = Entry::Chip(chip("crossdiv"));
        data.places[PATTERNS.start] = Entry::Pattern(0);
        data.places[PATTERNS.start + 1] = Entry::Pattern(2);
        data.places[PROGRAM_ADVANCE.start] = Entry::Chip(chip("csmopris"));
        let places = |names: &[&str]| -> [ChipPlace; 5] { std::array::from_fn(|i| names.get(i).map_or(ChipPlace::Empty, |n| ChipPlace::Chip(chip(n)))) };
        data.records[0] = Record { dx: -2, dy: 1, chips: places(&["sword", "wideswrd"]), score: 7 };
        data.records[1] = Record::ZERO;
        data.records[2] = Record { dx: -1, dy: 0, chips: places(&["cannon"; 5]), score: 10 };
        m.sides[0].auto_battle = data;
        m.sides[1].auto_battle = AutoBattle::default();
        let text = write(&content, &m);
        let written = "[left.auto_battle]\n\
            first = [\"areagrab\", {}, {}]\n\
            standard = [\n    \"sword\", \"sword\", \"sword\", \"sword\",\n    \"cannon\", 0, {}, {},\n    {}, {}, {}, {},\n    {}, {}, {}, {},\n    {}, {}, {}, {},\n    {}, {}, {}, {},\n]\n\
            mega = [{}, \"protoman\", {}, {}, {}]\n\
            giga = \"crossdiv\"\n\
            patterns = [1, 3, {}, {}, {}, {}, {}, {}]\n\
            program_advance = \"csmopris\"\n\
            records = [\n    \
            { dx = -2, dy = 1, chips = [\"sword\", \"wideswrd\", {}, {}, {}], score = 7 },\n    \
            { dx = 0, dy = 0, chips = [0, 0, 0, 0, 0], score = 0 },\n    \
            { dx = -1, dy = 0, chips = [\"cannon\", \"cannon\", \"cannon\", \"cannon\", \"cannon\"], score = 10 },\n    \
            { dx = -1, dy = -1, chips = [{}, {}, {}, {}, {}], score = 4294967295 },\n";
        assert!(text.contains(written), "{text}");
        assert!(!text.contains("[right.auto_battle]") && !text.contains("[[left") && !text.contains("[left.auto_battle."), "{text}");
        assert!(text.find("[left.auto_battle]") > text.find("[left]") && text.find("[right]") > text.find("[left.auto_battle]"), "{text}");
        let back = parse(&content, &text).unwrap_or_else(|e| panic!("{e:?}\n{text}"));
        assert_eq!(back, m);
        // The round sends it: its twelve entries (the 0 one of them), the
        // one of the first three places still first; its eight records.
        let sent = &m.round(&content, 2).players[0].auto_battle;
        assert_eq!((sent.entries.len(), sent.entries[0]), (12, AutoBattleEntry::Chip(chip("areagrab"))));
        assert!(sent.entries.contains(&AutoBattleEntry::Nothing) && sent.entries.contains(&AutoBattleEntry::Pattern(2)));
        assert_eq!(sent.patterns.len(), 8);
        assert_eq!(sent.patterns[0], PatternRecord::of(-2, 1, &[chip("sword"), chip("wideswrd")], 7));
        assert_eq!((sent.patterns[1].score, sent.patterns[2].score, sent.patterns[3]), (0, 10, PatternRecord::UNUSED));
        assert!(m.round(&content, 2).players[1].auto_battle.entries.is_empty());
        // What a file gets wrong.
        let bad = |from: &str, to: &str| -> Vec<String> {
            assert!(text.contains(from), "{from}:\n{text}");
            parse(&content, &text.replacen(from, to, 1)).unwrap_err()
        };
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        let first = "first = [\"areagrab\", {}, {}]";
        has(bad(first, "first = [\"nothing-at-all\", {}, {}]"), "left: auto_battle: first entry 1: no chip \"nothing-at-all\" in exe5");
        has(bad(first, "first = [9, {}, {}]"), "left: auto_battle: first entry 1: 9 is no entry (a chip's name, a pattern record's number 1 to 8, 0 for");
        has(bad(first, "first = [-1, {}, {}]"), "left: auto_battle: first entry 1: -1 is no entry");
        has(bad(first, "first = [{ pattern = 1 }, {}, {}]"), "left: auto_battle: first entry 1: { pattern = 1 } is no entry");
        has(bad(first, "first = [\"areagrab\"]"), "left: auto_battle: first states 1 places; it has 3, each stated ({} an empty one)");
        has(bad(first, "first = [\"areagrab\", {}, {}, {}]"), "left: auto_battle: first states 4 places; it has 3");
        has(bad("giga = \"crossdiv\"", "giga = \"crosdiv\""), "left: auto_battle: giga: no chip \"crosdiv\" in exe5");
        has(bad("giga = \"crossdiv\"", "giga = [\"crossdiv\"]"), "left: auto_battle: giga: [\"crossdiv\"] is no entry");
        has(bad("giga = \"crossdiv\"\n", ""), "missing field `giga`");
        has(bad("patterns = [1, 3,", "patterns = [1, 9,"), "left: auto_battle: patterns entry 2: 9 is no entry");
        has(bad("dx = -2, dy = 1", "dx = -2, dy = 1, dz = 0"), "unknown field `dz`");
        has(bad("dx = -2, dy = 1", "dx = -200, dy = 1"), "invalid value");
        has(bad("\"wideswrd\", {}, {}, {}]", "\"wideswd\", {}, {}, {}]"), "left: auto_battle: record 1: no chip \"wideswd\" in exe5");
        has(bad("\"wideswrd\", {}, {}, {}]", "\"wideswrd\"]"), "left: auto_battle: record 1: chips states 2 places; a record has 5");
        has(bad("\"wideswrd\", {}, {}, {}]", "\"wideswrd\", 1, {}, {}]"), "left: auto_battle: record 1: 1 is no chip place (a chip's name, 0, or {} for an empty place)");
        has(bad("    { dx = 0, dy = 0, chips = [0, 0, 0, 0, 0], score = 0 },\n", ""), "left: auto_battle: records states 7 pattern records; the data has 8, each stated");
        has(bad(first, &format!("{first}\nrest = []")), "unknown field `rest`");
        // EXE6 has no auto battle: its match file takes no such data.
        let six = exe6_content();
        let good = write(&six, &crate::pick::live(&six, "exe6", 2, None).unwrap());
        assert!(!good.contains("auto_battle"), "{good}");
        let empties = |n: usize| vec!["{}"; n].join(", ");
        let blank = "{ dx = -1, dy = -1, chips = [{}, {}, {}, {}, {}], score = 4294967295 }";
        let stated = format!(
            "first = [{}]\nstandard = [\"cannon\", {}]\nmega = [{}]\ngiga = {{}}\npatterns = [{}]\nprogram_advance = {{}}\nrecords = [{}]\n",
            empties(3),
            empties(23),
            empties(5),
            empties(8),
            vec![blank; 8].join(", ")
        );
        let problems = parse(&six, &format!("{good}\n[left.auto_battle]\n{stated}")).unwrap_err();
        assert_eq!(problems, ["left: auto battle data, but exe6 has no auto battle (no auto-battle system)"]);
        // (And a block nothing has written, stated whole, is the side that
        // states none.)
        let nothing = stated.replacen("\"cannon\"", "{}", 1);
        assert_eq!(parse(&six, &format!("{good}\n[left.auto_battle]\n{nothing}")).unwrap(), parse(&six, &good).unwrap());
    }

    /// A version is stated where the game's rules take one, and nowhere
    /// else: a new EXE6 match's sides have none until each is given its own
    /// (the checks refuse the match: none is assumed), a random one's are
    /// picked and written; an EXE5 match has none, and its file takes no
    /// `version`: its rules declare no such fact. EXE6's Crosses likewise:
    /// a list a side states, an empty one for none, with nothing assumed.
    #[test]
    fn a_version_is_stated_where_the_game_takes_one() {
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        let six = exe6_content();
        let new = Match::empty(&six, "exe6").unwrap();
        assert!(new.sides.iter().all(|s| s.version(&six).is_none()));
        let problems = crate::check_match(&six, &new);
        for side in ["left", "right"] {
            has(problems.clone(), &format!("{side}: no version: a side of exe6 states its own (gregar or falzar); none is assumed"));
            has(problems.clone(), &format!("{side}: no crosses: a side of exe6 states its own (up to 5 forms, an empty list for none); none is assumed"));
        }
        assert!(!write(&six, &new).contains("version") && !write(&six, &new).contains("crosses"));
        // (Nor does the engine start its round: a player's setup states the
        // version, and nothing fills one in. With the Crosses stated, that
        // is what it says is missing.)
        let mut no_version = new.clone();
        for s in &mut no_version.sides {
            s.set_fact(&six, "crosses", &[]).unwrap();
        }
        let refused = crate::check::start(&six, &no_version).err().expect("no round without the versions");
        assert_eq!(refused, "the round doesn't start: a player's setup doesn't state the cross system's `version` (gregar or falzar): none is assumed");
        let picked = write(&six, &crate::pick::live(&six, "exe6", 1, None).unwrap());
        assert_eq!(picked.matches("\nversion = \"falzar\"\n").count() + picked.matches("\nversion = \"gregar\"\n").count(), 2, "{picked}");
        // A version is its name, one of those the game's rules declare
        // (their `version` field's, in its order): a side made in code
        // takes no other, as a file's doesn't.
        // (EXE6's come in the original's order, which numbers them: the
        // byte a navi's stats carry is the version's place.)
        assert_eq!(crate::facts::versions(&six), ["gregar", "falzar"]);
        // (EXE6's rules require two facts: the one enum and the one list
        // of definitions no default states.)
        assert_eq!(crate::facts::required(&six).iter().map(|f| f.name).collect::<Vec<_>>(), ["crosses", "version"]);
        let live = crate::pick::live(&six, "exe6", 1, None).unwrap();
        assert!(live.sides.iter().all(|s| crate::facts::versions(&six).iter().any(|v| Some(v.as_str()) == s.version(&six))));
        let mut odd = live.clone();
        let refused = odd.sides[0].set_fact(&six, "version", &[Fact::Name("azure")]).unwrap_err();
        assert_eq!(refused, "setup field `version` has no variant \"azure\"");
        assert_eq!(odd, live);
        // The Crosses: stated with the version, the round starts; left
        // out, neither the match nor the engine's round does; an empty list
        // is a statement (none), and written as one.
        let mut none = new.clone();
        for s in &mut none.sides {
            s.set_fact(&six, "version", &[Fact::Name("falzar")]).unwrap();
        }
        let refused = crate::check::start(&six, &none).err().expect("no round without the Crosses");
        assert_eq!(refused, "the round doesn't start: a player's setup doesn't state the cross system's `crosses` (up to 5 forms, an empty list for none): none is assumed");
        for s in &mut none.sides {
            s.set_fact(&six, "crosses", &[]).unwrap();
        }
        // (The navi's version byte, NaviStats+0x20, is the version's place
        // among those the rules declare: the battle's start sets it.)
        let b = crate::check::start(&six, &none).unwrap();
        assert_eq!((b.stats[0].version, b.stats[1].version), (1, 1));
        assert_eq!(write(&six, &none).matches("\ncrosses = []\n").count(), 2);
        // (And a side's own of its version, as a tool fills them in: the
        // version's five.)
        assert!(none.sides[0].state_own_forms(&six));
        let own: Vec<&str> = none.sides[0].facts.form_list(&six).iter().map(|&f| ids::local(&six.defs.form(f).key)).collect();
        assert_eq!(own, ["spoutcross", "tomahawkcross", "tengucross", "groundcross", "dustcross"]);
        assert!(!new.sides[0].clone().state_own_forms(&six), "no version yet: nothing to go by");
        // EXE5.
        let five = crate::testing::exe5_content();
        assert!(crate::facts::versions(&five).is_empty());
        let m = crate::pick::live(&five, "exe5", 1, None).unwrap();
        let text = write(&five, &m);
        assert!(!text.contains("version"), "{text}");
        assert!(crate::facts::required(&five).is_empty());
        let e = parse(&five, &text.replacen("navi = \"megaman\"", "navi = \"megaman\"\nversion = \"falzar\"", 1)).unwrap_err();
        assert_eq!(e, [format!("left: no field \"version\" (a side of exe5 takes {})", crate::facts::names_phrase(&five))]);
        let mut odd = m.clone();
        let refused = odd.sides[1].set_fact(&five, "version", &[Fact::Name("gregar")]).unwrap_err();
        assert_eq!(refused, format!("no field \"version\" (a side of exe5 takes {})", crate::facts::names_phrase(&five)));
        // (Nor are another game's facts a side's: an EXE6 side in an EXE5
        // match is said.)
        odd.sides[1].facts = live.sides[1].facts.clone();
        has(crate::check_match(&five, &odd), "right: the side's facts aren't exe5's rules'");
        // (The round an EXE5 match starts brings its players no version.)
        let b = crate::check::start(&five, &m).unwrap();
        assert!(b.fact(0, nettai_battle::content::PlayerFact::Version).is_none() && b.stats[0].version == 0);
    }

    /// What a file can get wrong is said, with where it is.
    #[test]
    fn problems_are_said() {
        let content = exe6_content();
        let picked = crate::pick::live(&content, "exe6", 1, None).unwrap();
        let good = write(&content, &picked);
        let bad = |from: &str, to: &str| -> Vec<String> {
            assert!(good.contains(from), "{from}");
            parse(&content, &good.replacen(from, to, 1)).unwrap_err()
        };
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        has(bad("navi = \"megaman\"", "navi = \"nobody\""), "left: no navi \"nobody\" in exe6");
        has(bad("navi = \"megaman\"", "navi = \"exe6:megaman\""), "left: no navi \"exe6:megaman\" in exe6"); // (written in full)
        // (No stats block: a side states what the save brings to them as
        // facts, and nothing else of them.)
        let stats = parse(&content, &format!("{good}\n[left.stats]\nhp = 1000\n")).unwrap_err();
        has(stats, "left: no field \"stats\"");
        has(bad("navi = \"megaman\"", "navi = \"megaman\"\nhp = 100000"), "left: hp: 100000 is past a u16");
        // No key takes the emotion window's glitch: the rules make it. (A
        // key that is none of a side's own parts is a fact of its game's
        // rules, by its setup field's name, or it is refused with those the
        // game takes.)
        has(
            bad("navi = \"megaman\"", "navi = \"megaman\"\nemotion_window_glitch = true"),
            "left: no field \"emotion_window_glitch\" (a side of exe6 takes hp, reg_up, sun, crosses, version, beast_out, bug_frags)",
        );
        let stage = good.lines().find(|l| l.starts_with("stage = ")).unwrap();
        has(bad(stage, "stage = \"moon\""), "arena: no stage \"moon\" in exe6");
        has(bad("game = \"exe6\"", "game = \"bn7\""), "no game \"bn7\"");
        // The version: one of the game's two, stated (none is assumed).
        let version = good.lines().find(|l| l.starts_with("version = ")).unwrap();
        has(bad(version, "version = \"azure\""), "left: version: no \"azure\" (gregar or falzar)");
        has(bad(version, "version = 1"), "left: version: no 1 (gregar or falzar)");
        // A fact's value is its field's type's: a flag, a whole number in
        // its range, a name of the game's, a list no longer than it holds.
        let stated = |line: &str| bad("navi = \"megaman\"", &format!("navi = \"megaman\"\n{line}"));
        has(stated("beast_out = 1"), "left: beast_out: 1 is neither true nor false");
        has(stated("bug_frags = -1"), "left: bug_frags: -1 is past a u32 (0 to 4294967295)");
        has(stated("bug_frags = \"many\""), "left: bug_frags: \"many\" is no whole number");
        let list = good.lines().find(|l| l.starts_with("crosses = ")).unwrap();
        has(
            bad(list, "crosses = [\"heatcross\", \"eleccross\", \"slashcross\", \"erasecross\", \"chargecross\", \"spoutcross\"]"),
            "left: crosses: 6 entries, it holds 5",
        );
        has(bad(list, "crosses = true"), "left: crosses: true is no list");
        has(bad(list, "crosses = [\"heatcros\"]"), "left: crosses: no form \"heatcros\" in exe6");
        has(bad(list, "crosses = [\"heatcross\", \"heatcross\"]"), "left: crosses: HeatCross is there twice");
        // (The Crosses are forms of the navi's own lists: a Cross's Beast
        // form is a form of EXE6's, and none of them.)
        has(bad(list, "crosses = [\"heatcross-beast\"]"), "left: crosses: heatcross-beast is no form of MegaMan's lists");
        // (Left out, they are missing: none is assumed. A list with a gap
        // states what no save has.)
        has(bad(&format!("{list}\n"), ""), "left: no crosses: a side of exe6 states its own (up to 5 forms, an empty list for none); none is assumed");
        has(bad(list, "crosses = [\"\", \"heatcross\"]"), "left: crosses: an empty entry before HeatCross (a list is filled from the front)");
        has(bad(&format!("{version}\n"), ""), "left: no version: a side of exe6 states its own (gregar or falzar); none is assumed");
        // Thirty copies of a chip.
        let mut m = picked.clone();
        m.sides[0].folder.chips = [m.sides[0].folder.chips[0]; 30];
        m.sides[0].folder.regular = None;
        has(crate::check_match(&content, &m), "left: folder: 30 copies of");
        // Mega chips past the navi's Mega level (MegaMan's fresh 5, with no
        // NaviCust program that raises it): ten of the game's.
        let mut m = picked.clone();
        let megas: Vec<_> = (0..content.defs.chips.len() as u16)
            .map(nettai_content_api::ChipHandle)
            .filter(|&c| content.chip(c).class == nettai_battle::content::ChipClass::Mega && !content.chip(c).codes.is_empty())
            .filter(|&c| ids::in_game(&content, "exe6", &content.defs.chip(c).key))
            .take(10)
            .collect();
        for (i, &c) in megas.iter().enumerate() {
            m.sides[1].folder.chips[i] = Some(FolderChip::new(c, content.chip(c).codes[0]));
        }
        m.sides[1].folder.regular = None;
        has(crate::check_match(&content, &m), "Mega chips, past the navi's 5");
        // Patch cards past 80 MB; Crosses for a navi without any.
        let mut m = picked.clone();
        m.sides[0].cards = crate::patch_cards(&content, "exe6", "canodumb,amonicul,coldbear,megalian,mettfire,kilplant").unwrap();
        has(crate::check_match(&content, &m), "left: the patch cards are");
        let mut m = picked.clone();
        let protoman = ids::navi(&content, "exe6", "protoman").unwrap();
        m.sides[1].navi = protoman;
        has(crate::check_match(&content, &m), "right: crosses: ProtoMan doesn't change form");
    }

    /// A game's rules have their systems, by name: EXE6's the forms system
    /// (its Crosses), and no key names another ruleset.
    #[test]
    fn a_game_is_its_rules() {
        let content = exe6_content();
        assert!(crate::ruleset_has_system(&content, crate::FORMS_SYSTEM));
        assert!(!crate::ruleset_has_system(&content, "souls"));
        let good = write(&content, &crate::pick::live(&content, "exe6", 1, None).unwrap());
        let e = parse(&content, &good.replacen("game = \"exe6\"\n", "game = \"exe6\"\nruleset = \"stock\"\n", 1)).unwrap_err();
        assert!(e[0].contains("unknown field `ruleset`"), "{e:?}");
    }

    /// The SP deletion times, Beast Out locked and a level write and read
    /// back; a navi's own default level (a link navi's 0, MegaMan's none) is
    /// left out, and a file without one reads as it.
    #[test]
    fn sp_times_beast_out_and_levels_write_and_read_back() {
        let content = exe6_content();
        let mut m = crate::pick::live(&content, "exe6", 2, None).unwrap();
        m.sides[0].set_fact(&content, "beast_out", &[Fact::Value(Value::Bool(false))]).unwrap();
        m.sides[0].navi_level = Some(3);
        m.sides[0].sp_times.0[0] = 721;
        m.sides[0].sp_times.0[11] = 1500;
        let protoman = ids::navi(&content, "exe6", "protoman").unwrap();
        m.sides[1].navi = protoman;
        m.sides[1].set_fact(&content, "crosses", &[]).unwrap();
        m.sides[1].navicust = None;
        m.sides[1].navi_level = Some(0);
        m.sides[1].folder.regular = None;
        let text = write(&content, &m);
        for line in ["beast_out = false", "level = 3", "[left.sp_times]", "\"sp/heatman\" = \"00:12.01\"", "\"sp/blastman\" = \"00:25.00\""] {
            assert!(text.contains(line), "{line}:\n{text}");
        }
        let right = &text[text.find("[right]").unwrap()..];
        assert!(!right.contains("level ="), "ProtoMan's level 0 is his default:\n{right}");
        assert_eq!(parse(&content, &text).unwrap(), m, "{text}");
        // No level: ProtoMan's 0, MegaMan's none.
        let no_level = text.replacen("level = 3\n", "", 1);
        let back = parse(&content, &no_level).unwrap();
        assert_eq!((back.sides[0].navi_level, back.sides[1].navi_level), (None, Some(0)));
        // A time that isn't one, a slot the rules lack.
        let bad = parse(&content, &text.replacen("\"00:12.01\"", "\"12:60.00\"", 1)).unwrap_err();
        assert!(bad.iter().any(|p| p.contains("sp_times: sp/heatman")), "{bad:?}");
        let bad = parse(&content, &text.replacen("\"sp/heatman\"", "\"sp/nobody\"", 1)).unwrap_err();
        assert!(bad.iter().any(|p| p.contains("no SP navi slot \"sp/nobody\"")), "{bad:?}");
    }

    /// A navi code's level is 0 to 14, and a link navi has one.
    #[test]
    fn the_level_is_checked() {
        let content = exe6_content();
        let mut m = crate::pick::live(&content, "exe6", 2, None).unwrap();
        m.sides[0].navi_level = Some(15);
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        has(crate::check_match(&content, &m), "left: level 15: a navi code's level is 0 to 14");
        let protoman = ids::navi(&content, "exe6", "protoman").unwrap();
        m.sides[0].navi_level = None;
        m.sides[1].navi = protoman;
        m.sides[1].set_fact(&content, "crosses", &[]).unwrap();
        m.sides[1].navi_level = None;
        let problems = crate::check_match(&content, &m);
        has(problems.clone(), "right: ProtoMan has no level: a link navi exists only through its navi code");
        assert!(!problems.iter().any(|p| p.starts_with("left")), "MegaMan without a code is fine: {problems:?}");
    }
}
