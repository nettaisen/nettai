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
//! [[round]]                          # the set's rounds, one table each, in order: as many as it has
//! stage = "netbattle-43"             # (1 to 99; three: the original's triple battle, best of three).
//! background = "lans-hp"             # A part a round leaves out, or a round left empty, is picked
//!                                    # from the seed (as the game picks a link battle's)
//! [[round]]
//! stage = "netbattle-12"
//!
//! [[round]]
//!
//! [left]                             # you, side 0; then [right]: the side's facts, what its game's rules take,
//! navi = "megaman"                   # each under its setup field's name (crate::facts); one left out is the
//! level = 7                          # rules' default. The navi (which a side states), the navi code's level (0-14;
//! version = "falzar"                 # else the navi's last: a link navi's 14, MegaMan's none), EXE6's version
//! crosses = ["heatcross", "spoutcross"]   # (gregar or falzar, which a side states) and crosses (up to five, of
//! beast_out = false                  # either version; [] none), beast_out (else unlocked), bug_frags (else 0),
//! bug_frags = 9                      # what the save brings to the stats: the base HP (else 1000), the Regular
//! hp = 1000                          # memory (else 50), the sun (else true); the rules derive the rest
//! reg_up = 50
//! folder = [                         # a list of records, a table each, a line each (a field left out: false, none)
//!     { chip = "cannon", code = "A" },   # up to 30 entries ({} an empty one, while it's being made)
//!     { chip = "cannon", code = "A" },
//! ]
//! regular_chip = 4                   # the Regular chip's entry, from 0 (else none)
//! tag_chips = [5, 6]                 # the tag chips' entries (else none)
//! patch_cards = ["canodumb", "shadow"]  # in the order they apply (else none)
//! sp_times = [                       # SP navi deletion times, by SP chip, in frames (else every SP navi in no time)
//!     { chip = "heatman-sp", frames = 741 },
//! ]
//! navicust_expansions = 2            # the NaviCust's board (else the largest)
//! navicust_programs = [              # its programs in the list's order; x, y the center on the 7x7 grid
//!     { program = "suprarmr", color = "red", x = 3, y = 3, rotation = 1, compressed = true },
//! ]
//!
//! # An EXE5 side's facts ([left] of game = "exe5"; a fact left out is its rules' default):
//! level = 3                          # a team navi's level, 0 to 6 (else 6): its damage rows' (its HP the story's)
//! karma = 100                        # the light/dark value (default 500, a fresh save's; dark under 470)
//! souls = ["protosoul"]              # the souls it has, either version's (default: every soul; none, no soul button)
//! chaos_unison = false               # no Chaos Unison (the save's event flag 0x236; default true)
//!
//! auto_battle_places = [           # EXE5's: what a navi in auto battle plays from the side's save, its block's 42
//!     {}, {}, {},                    # places to the last that isn't empty: each a chip, a pattern record by number
//!     { chip = "sword" },            # (1 to 8), a 0 (`zero`) or {} (empty); the game writes its player's most used
//!     { pattern = 1 },               # standard chips from place 4, mega 28, giga 33, patterns 34, a program
//! ]                                  # advance 42 (else none)
//! auto_battle_records = [            # the eight pattern records: a place from its target, five chip places, a score
//!     { dx = -2, dy = 0, chips = [{ chip = "sword" }, { chip = "wideswrd" }], score = 10 },
//! ]                                  # (else every record zeros: nothing learned)
//! ```

use crate::facts::Stated;
use crate::{Facts, Match, RoundSettings, Side, ids};
use nettai_battle::content::{ChipCode, Content, PlayerFact};
use nettai_battle::rules::Fact;
use nettai_content_api::{FieldType, Value};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchFile {
    pub game: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    /// The rounds, a `[[round]]` table each (a file that lists none is
    /// refused when it is resolved).
    #[serde(default, rename = "round")]
    pub rounds: Vec<RoundFile>,
    pub left: SideFile,
    pub right: SideFile,
}

/// A round as a file states it: each part left out is picked from the
/// seed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoundFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

/// A side as a file states it: each key a fact of the game's rules, under
/// its setup field's name (`crate::facts`: the navi, the folder, EXE6's
/// version, EXE5's auto battle data...; a key the rules' setup doesn't
/// declare is refused when the side is resolved, with the facts the game
/// takes).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SideFile {
    /// The facts, in the order the file states them (written the navi
    /// first, then in the setup's fields' order).
    #[serde(flatten)]
    pub facts: FactsFile,
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

/// What names nothing in `game`: "no chip "x" in exe6", and the game's
/// names of that kind when they are few enough to read.
fn unknown(what: &str, name: &str, game: &str, have: &[&str]) -> String {
    if have.is_empty() || have.len() > 40 {
        format!("no {what} {name:?} in {game}")
    } else {
        format!("no {what} {name:?} in {game} ({game}'s are {})", have.join(", "))
    }
}

/// Each name a file says, resolved in its game: the match, or every
/// problem with it.
pub fn resolve(content: &Content, f: &MatchFile) -> Result<Match, Vec<String>> {
    let mut problems = Vec::new();
    let games = ids::games(content);
    if !games.contains(&f.game) {
        return Err(vec![format!("no game {:?} (the content's are {})", f.game, games.join(", "))]);
    }
    let rounds = resolve_rounds(content, &f.game, &f.rounds, &mut problems);
    let Some(rounds) = rounds else { return Err(problems) };
    let left = resolve_side(content, &f.game, &f.left, "left", &mut problems);
    let right = resolve_side(content, &f.game, &f.right, "right", &mut problems);
    match (left, right) {
        (Some(left), Some(right)) if problems.is_empty() => Ok(Match { game: f.game.clone(), seed: f.seed, rounds, sides: [left, right] }),
        _ => Err(problems),
    }
}

fn resolve_round(content: &Content, game: &str, r: &RoundFile, at: &str, problems: &mut Vec<String>) -> Option<RoundSettings> {
    let stage = match &r.stage {
        None => None,
        Some(name) => match ids::stage(content, game, name) {
            Some(s) => Some(s),
            None => {
                problems.push(format!("{at}: {}", unknown("stage", name, game, &[])));
                return None;
            }
        },
    };
    if let Some(b) = &r.background
        && crate::background(content, game, b).is_none()
    {
        problems.push(crate::no_background(at, game, b));
    }
    Some(RoundSettings { stage, background: r.background.clone() })
}

/// What a file that lists no round is told.
pub const NO_ROUNDS: &str = "no [[round]]: a match file lists one [[round]] per round of its set (one left empty is picked from the seed; three for the original's triple battle)";

/// A file's rounds, of `game`.
pub fn resolve_rounds(content: &Content, game: &str, rounds: &[RoundFile], problems: &mut Vec<String>) -> Option<Vec<RoundSettings>> {
    if rounds.is_empty() {
        problems.push(NO_ROUNDS.to_string());
        return None;
    }
    let resolved: Vec<Option<RoundSettings>> =
        rounds.iter().enumerate().map(|(i, r)| resolve_round(content, game, r, &format!("round {}", i + 1), problems)).collect();
    resolved.into_iter().collect()
}

/// A file's side of a match of `game`, each name the game's.
pub fn resolve_side(content: &Content, game: &str, s: &SideFile, at: &str, problems: &mut Vec<String>) -> Option<Side> {
    let start = problems.len();
    let mut say = |p: String| problems.push(format!("{at}: {p}"));
    // The facts: each key a field of the rules' setup, its value read by the
    // field's type. A fact the file leaves out is the rules' default (an
    // enum without one, unstated; the navi, none: the checks say a round
    // needs them).
    let mut facts = Facts::defaults(content);
    for (key, value) in &s.facts.0 {
        let Some(field) = crate::facts::field(content, key) else {
            say(crate::facts::no_field(content, key));
            continue;
        };
        match fact_values(content, game, field.ty, value).and_then(|values| facts.set(content, key, &values)) {
            Ok(()) => {}
            Err(e) => say(format!("{key}: {e}")),
        }
    }
    let mut side = Side { facts };
    if side.stated_navi(content).is_none() && problems.len() == start {
        problems.push(format!("{at}: no navi: a side states its own"));
    }
    // (A level it leaves out: its navi's highest.)
    let level = content.defs.fact_name(PlayerFact::Level);
    if problems.len() == start
        && level.is_some_and(|l| !s.facts.0.iter().any(|(key, _)| key == l))
        && let Err(e) = side.state_play_level(content)
    {
        problems.push(format!("{at}: level: {e}"));
    }
    (problems.len() == start).then_some(side)
}

/// A fact's value as a file states it, read by the field's type `ty`: a
/// flag, a whole number that fits the type, an enum's variant by its name,
/// a definition by its name in `game` ("" none), an array's or a list's
/// elements from the first, a record's fields by name (a table). (The
/// values a fact's writer takes: one, or an array's or a list's elements.)
fn fact_values<'v>(content: &Content, game: &str, ty: &FieldType, v: &'v toml::Value) -> Result<Vec<Fact<'v>>, String> {
    match one(content, game, ty, v)? {
        Fact::List(items) => Ok(items),
        f => Ok(vec![f]),
    }
}

/// [`fact_values`]' value at a place of type `ty`.
fn one<'v>(content: &Content, game: &str, ty: &FieldType, v: &'v toml::Value) -> Result<Fact<'v>, String> {
    Ok(match ty {
        FieldType::Bool => Fact::Value(v.as_bool().map(Value::Bool).ok_or_else(|| format!("{v} is neither true nor false"))?),
        // (A number past its type is the facts' writer's to refuse.)
        ty if crate::facts::range(ty).is_some() => Fact::Value(v.as_integer().map(Value::Int).ok_or_else(|| format!("{v} is no whole number"))?),
        FieldType::Enum(names) => match v.as_str().map(|n| names.iter().position(|x| x == n)) {
            Some(Some(i)) => Fact::Value(Value::Int(i as i64)),
            _ => return Err(format!("no {v} ({})", names.join(" or "))),
        },
        FieldType::Ref(registry, of) => Fact::Value(match v.as_str() {
            Some("") => Value::Nil,
            Some(name) => ids::handle_of(content, game, *registry, of.as_deref(), name).map(|h| Value::Def(*registry, h)).ok_or_else(|| {
                let have: Vec<&str> = ids::all_of(content, *registry, of.as_deref())
                    .into_iter()
                    .filter(|&h| ids::key_of(content, *registry, h).is_some_and(|k| ids::in_game(content, game, k)))
                    .filter_map(|h| ids::name_of(content, *registry, h))
                    .collect();
                // (An entry by its collection's name: "no patch_cards `x`".)
                let what = match (registry, of) {
                    (nettai_content_api::Registry::Entry, Some(c)) => c.clone(),
                    _ => registry.to_string(),
                };
                unknown(&what, name, game, &have)
            })?,
            None => return Err(format!("{v} is no {}'s name", ty)),
        }),
        FieldType::Code => Fact::Value(match v.as_str() {
            Some("") => Value::Nil,
            Some(code) => {
                let mut letters = code.chars();
                match letters.next().and_then(ChipCode::from_letter).filter(|_| letters.next().is_none()) {
                    Some(c) => Value::Code(c.letter() as u8),
                    None => return Err(format!("{code:?} is no code (A-Z or *)")),
                }
            }
            None => return Err(format!("{v} is no code (A-Z or *)")),
        }),
        FieldType::Array(..) | FieldType::List(..) => {
            let (elem, n) = match ty {
                FieldType::Array(elem, n) => (elem, *n as usize),
                FieldType::List(elem, n) => (elem, *n as usize),
                _ => unreachable!("an array or a list"),
            };
            let Some(items) = v.as_array() else { return Err(format!("{v} is no list")) };
            if items.len() > n {
                return Err(format!("{} entries, it holds {n}", items.len()));
            }
            Fact::List(
                items.iter().enumerate().map(|(k, item)| one(content, game, elem, item).map_err(|e| format!("[{}]: {e}", k + 1))).collect::<Result<_, _>>()?,
            )
        }
        FieldType::Record(fields) => {
            let Some(table) = v.as_table() else { return Err(format!("{v} is no table of fields")) };
            let mut out = Vec::with_capacity(table.len());
            for (name, x) in table {
                let Some(i) = fields.index_of(name) else {
                    let names: Vec<&str> = fields.fields().iter().map(|f| f.name.as_str()).collect();
                    return Err(format!("no field `{name}` ({})", names.join(", ")));
                };
                out.push((name.as_str(), one(content, game, &fields.field(i).ty, x).map_err(|e| format!("{name}: {e}"))?));
            }
            Fact::Record(out)
        }
        other => return Err(format!("a match states no {other:?}")),
    })
}

/// A fact's value as a file writes it: a flag, a number, an enum's
/// variant's name, a definition's name, a code's letter, a list's entries
/// (a list of definitions up to its last one; a hole in it, ""), a
/// record's fields (one holding nothing, false, none or 0, left out). None:
/// nothing to write (an enum nothing states, no definition, no code).
fn fact_toml(content: &Content, value: &Stated) -> Option<toml::Value> {
    Some(match value {
        Stated::Flag(b) => toml::Value::Boolean(*b),
        Stated::Number(n) => toml::Value::Integer(*n),
        Stated::Variant(name) => toml::Value::String(name.clone()?),
        Stated::Def(registry, h) => toml::Value::String(ids::name_of(content, *registry, (*h)?)?.to_string()),
        Stated::List(items) => {
            let last = items.iter().rposition(|v| !matches!(v, Stated::Def(_, None))).map_or(0, |i| i + 1);
            toml::Value::Array(items[..last].iter().map(|v| fact_toml(content, v).unwrap_or_else(|| toml::Value::String(String::new()))).collect())
        }
        Stated::Optional(n) => toml::Value::Integer((*n)?),
        Stated::Record(fields) => toml::Value::Table(
            fields
                .iter()
                .filter(|(_, v)| !matches!(v, Stated::Flag(false) | Stated::Number(0)))
                .filter_map(|(name, v)| Some((name.clone(), fact_toml(content, v)?)))
                .collect(),
        ),
        Stated::Code(c) => toml::Value::String((*c)?.to_string()),
        Stated::Other => return None,
    })
}

/// A side as a file writes it, each name its game's.
pub fn side_file(content: &Content, s: &Side) -> SideFile {
    // The facts that aren't the rules' defaults, the navi first, then in the
    // setup's fields' order (a level whenever the side has one).
    let navi = PlayerFact::Navi.name();
    let mut fields = crate::facts::fields(content);
    fields.sort_by_key(|f| f.name != navi);
    SideFile {
        facts: FactsFile(
            fields
                .into_iter()
                .filter(|f| !s.facts.is_default(content, f.name))
                .filter_map(|f| Some((f.name.to_string(), fact_toml(content, &s.facts.get(content, f.name)?)?)))
                .collect(),
        ),
    }
}

/// A match's rounds as a file writes them (what each states).
pub fn rounds_file(content: &Content, rounds: &[RoundSettings]) -> Vec<RoundFile> {
    let round = |r: &RoundSettings| RoundFile { stage: r.stage.map(|s| ids::local(&content.defs.stage(s).key).to_string()), background: r.background.clone() };
    rounds.iter().map(round).collect()
}

/// A match as a file.
pub fn to_file(content: &Content, m: &Match) -> MatchFile {
    MatchFile {
        game: m.game.clone(),
        seed: m.seed,
        rounds: rounds_file(content, &m.rounds),
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
    format!("# A nettai match (docs/frontend.md §6): play it with `nettai-demo --match FILE`.\n\n{}", tidy(&body, &[("left", &file.left), ("right", &file.right)]))
}

/// A side alone as TOML, under the table `key` (`[side]`), laid out as a
/// match file lays out its sides: what a file of one side holds (a
/// player's build, under a header of its own), which [`resolve_side`]
/// reads back.
pub fn side_toml(content: &Content, key: &str, s: &Side) -> String {
    let side = side_file(content, s);
    let mut table = toml::map::Map::new();
    table.insert(key.to_string(), toml::Value::try_from(&side).expect("a side serializes"));
    let body = toml::to_string_pretty(&table).expect("a side serializes");
    tidy(&body, &[(key, &side)])
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

/// A value of a fact as one inline value: a table an inline table.
fn inline(v: &toml::Value) -> toml_edit::Value {
    match v {
        toml::Value::String(s) => s.as_str().into(),
        toml::Value::Integer(n) => (*n).into(),
        toml::Value::Boolean(b) => (*b).into(),
        toml::Value::Float(f) => (*f).into(),
        toml::Value::Array(items) => items.iter().map(inline).collect::<toml_edit::Array>().into(),
        toml::Value::Table(t) => {
            let mut table = toml_edit::InlineTable::new();
            for (k, x) in t {
                table.insert(k, inline(x));
            }
            table.into()
        }
        toml::Value::Datetime(d) => d.to_string().into(),
    }
}

/// `body`, the pretty printer's text of a file of `sides` (each its table's
/// name and the side), as a person would lay it out: a fact's short list on
/// its line (`crosses = ["heatcross", "eleccross"]`; a longer one an entry
/// a line, as the printer has it), and a list of records an inline table a
/// line, after the side's other values (the printer makes a table of each,
/// under its own header).
fn tidy(body: &str, sides: &[(&str, &SideFile)]) -> String {
    let mut doc: toml_edit::DocumentMut = body.parse().expect("a match file parses");
    for &(side, of) in sides {
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
        // A list of records: an inline table each, a line each (the printer
        // makes a table of each, under its own header).
        for (key, value) in &of.facts.0 {
            let toml::Value::Array(items) = value else { continue };
            if !items.iter().any(|v| v.is_table()) {
                continue;
            }
            let Some(table) = doc.get_mut(side).and_then(|s| s.as_table_mut()) else { continue };
            let mut list: toml_edit::Array = items.iter().map(inline).collect();
            if list.len() > 1 {
                a_line_each(&mut list);
            }
            // (After the side's other values: a long list last.)
            table.remove(key);
            table.insert(key, toml_edit::value(list));
        }
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
        for line in ["game = \"exe6\"", "[[round]]\nstage = \"netbattle-", "[left]", "navi = \"megaman\"", "folder = [\n    { chip = \"", "\", code = \""] {
            assert!(text.contains(line), "{line}:\n{text}");
        }
        // (No stats: a side states none.)
        assert!(!text.contains("stats"), "{text}");
        // Every name is the game's own, written once with the game.
        assert!(!text.contains("exe6:") && !text.contains("rules"), "{text}");
        assert_eq!(game_of(&text).unwrap(), "exe6");
        assert!(game_of("[[round]]\nstage = \"x\"\n").is_err());
    }

    /// A match file lists its rounds, a `[[round]]` table each, as many as
    /// the set has: each part it states is that round's, each it leaves out
    /// (or a round left empty) is picked from the seed, and a file that
    /// lists none is refused. Five rounds, some stated, write and read back
    /// and start the same round.
    #[test]
    fn a_match_file_lists_its_rounds() {
        let content = exe6_content();
        let live = crate::pick::live(&content, "exe6", 4, None).unwrap();
        let text = write(&content, &live);
        assert_eq!(text.matches("[[round]]").count(), 3, "{text}");
        let start = text.find("[[round]]").unwrap();
        let end = text.find("[left]").unwrap();
        let rounds = "[[round]]\n\n[[round]]\nstage = \"netbattle-12\"\n\n[[round]]\nbackground = \"undernet\"\n\n[[round]]\n\n[[round]]\n\n";
        let five = format!("{}{rounds}{}", &text[..start], &text[end..]);
        let m = parse(&content, &five).unwrap_or_else(|e| panic!("{e:?}\n{five}"));
        assert_eq!(m.rounds.len(), 5);
        assert_eq!(m.rounds[1].stage, Some(crate::link_stage(&content, "exe6", "netbattle-12").unwrap()));
        assert_eq!((m.rounds[2].stage, m.rounds[2].background.as_deref()), (None, Some("undernet")));
        assert_eq!(m.rounds[0], RoundSettings::default());
        assert_eq!(parse(&content, &write(&content, &m)).unwrap(), m);
        let setup = m.round(&content, 4);
        assert_eq!((setup.rounds(), setup.later_stages[0].stage), (5, m.rounds[1].stage.unwrap()));
        assert_eq!(setup.later_stages[1].background, crate::background(&content, "exe6", "undernet").unwrap());
        // (The first round, left empty, is the random match's of the seed.)
        assert_eq!(setup.settings, live.round(&content, 4).settings);
        let none = format!("{}{}", &text[..start], &text[end..]);
        assert_eq!(parse(&content, &none).unwrap_err(), [NO_ROUNDS]);
        let many = format!("{}{}{}", &text[..start], "[[round]]\n".repeat(crate::MAX_ROUNDS + 1), &text[end..]);
        assert_eq!(parse(&content, &many).unwrap_err(), ["100 rounds: a match has 1 to 99"]);
    }

    /// A side's auto battle data (EXE5's facts) writes and reads back: its
    /// places to the last that isn't empty (chips by name, a pattern by its
    /// record's number from 1, a 0, an empty place as `{}`) and its records;
    /// the round sends it; and what a file gets wrong is said, with where it
    /// is.
    #[test]
    fn auto_battle_data_writes_and_reads_back() {
        use nettai_battle::rules::Fact;
        use nettai_content_api::{Registry, Value};
        let content = crate::testing::exe5_content();
        let mut m = crate::pick::live(&content, "exe5", 2, None).unwrap();
        let chip = |name: &str| ids::chip(&content, "exe5", name).unwrap();
        let def = |name: &str| -> Fact<'static> { Fact::Value(Value::Def(Registry::Chip, chip(name).0)) };
        let int = |n: i64| -> Fact<'static> { Fact::Value(Value::Int(n)) };
        // Its 42 places: the first, the standard chips from place 4 (a 0
        // after them), an empty place before the second mega chip, a giga, two
        // patterns and a program advance.
        let mut places: Vec<Fact> = (0..42).map(|_| Fact::Record(Vec::new())).collect();
        places[0] = Fact::Record(vec![("chip", def("areagrab"))]);
        for (i, name) in ["sword", "sword", "sword", "sword", "cannon"].into_iter().enumerate() {
            places[3 + i] = Fact::Record(vec![("chip", def(name))]);
        }
        places[8] = Fact::Record(vec![("zero", Fact::Value(Value::Bool(true)))]);
        places[28] = Fact::Record(vec![("chip", def("protoman"))]);
        places[32] = Fact::Record(vec![("chip", def("crossdiv"))]);
        places[33] = Fact::Record(vec![("pattern", int(1))]);
        places[34] = Fact::Record(vec![("pattern", int(3))]);
        places[41] = Fact::Record(vec![("chip", def("csmopris"))]);
        let chips = |names: &[&str], zero: bool| -> Fact<'static> {
            Fact::List(
                (0..5)
                    .map(|i| match names.get(i) {
                        Some(n) => Fact::Record(vec![("chip", def(n))]),
                        None if zero => Fact::Record(vec![("zero", Fact::Value(Value::Bool(true)))]),
                        None => Fact::Record(Vec::new()),
                    })
                    .collect(),
            )
        };
        let record = |dx: i64, dy: i64, c: Fact<'static>, score: i64| -> Fact<'static> { Fact::Record(vec![("dx", int(dx)), ("dy", int(dy)), ("chips", c), ("score", int(score))]) };
        let records = vec![record(-2, 1, chips(&["sword", "wideswrd"], false), 7), record(0, 0, chips(&[], true), 0), record(-1, 0, chips(&["cannon"; 5], false), 10)];
        m.sides[0].set_fact(&content, "auto_battle_places", &places).unwrap();
        m.sides[0].set_fact(&content, "auto_battle_records", &records).unwrap();
        // (The other side: a block nothing has written.)
        m.sides[1].set_fact(&content, "auto_battle_places", &[]).unwrap();
        m.sides[1].set_fact(&content, "auto_battle_records", &[]).unwrap();
        let text = write(&content, &m);
        for line in [
            "auto_battle_places = [\n    { chip = \"areagrab\" },\n    {},\n    {},\n    { chip = \"sword\" },",
            "    { zero = true },\n",
            "    { pattern = 1 },\n    { pattern = 3 },\n",
            "    { chip = \"csmopris\" },\n]",
            "    { chips = [{ chip = \"sword\" }, { chip = \"wideswrd\" }, {}, {}, {}], dx = -2, dy = 1, score = 7 },\n",
            "    { chips = [{ zero = true }, { zero = true }, { zero = true }, { zero = true }, { zero = true }] },\n",
        ] {
            assert!(text.contains(line), "{line}:\n{text}");
        }
        let back = parse(&content, &text).unwrap_or_else(|e| panic!("{e:?}\n{text}"));
        assert_eq!(back, m);
        // The round sends it (EXE5's rules, as the round is set up): its
        // twelve entries (the 0 one of them), the one of the first three
        // places still first; the other side's none.
        let b = crate::check::start(&content, &m).unwrap();
        let sent = |side: u8| -> (usize, nettai_content_api::FieldValue) {
            let (schema, state) = b.rules_state(side).unwrap();
            let place = schema.place(schema.index_of("auto_battle_places").unwrap());
            let n = state.len_at(place).unwrap();
            let first = place.elem(0).map_or(nettai_content_api::FieldValue::Ref(None), |e| state.get_at(e.field("chip").unwrap()));
            (n, first)
        };
        let areagrab = nettai_content_api::FieldValue::Ref(Some((nettai_content_api::Registry::Chip, chip("areagrab").0)));
        assert_eq!(sent(0), (12, areagrab));
        assert_eq!(sent(1).0, 0);
        // What a file gets wrong.
        let bad = |from: &str, to: &str| -> Vec<String> {
            assert!(text.contains(from), "{from}:\n{text}");
            parse(&content, &text.replacen(from, to, 1)).unwrap_err()
        };
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        let first = "{ chip = \"areagrab\" }";
        has(bad(first, "{ chip = \"nothing-at-all\" }"), "left: auto_battle_places: [1]: chip: no chip \"nothing-at-all\" in exe5");
        has(bad(first, "{ pattern = 9 }"), "left: place 1 of the auto battle data names pattern 9; it has 8 pattern records");
        has(bad(first, "{ chip = \"areagrab\", pattern = 1 }"), "left: place 1 of the auto battle data (`first`, entry 1) is a chip, a pattern or a 0, not more");
        has(bad(first, "{ chip = \"stepswrd\" }"), "left: place 1 of the auto battle data (`first`, entry 1) holds");
        has(bad("dx = -2, dy = 1", "dx = -2, dy = 1, dz = 0"), "no field `dz` (chips, dx, dy, score)");
        has(bad("dx = -2, dy = 1", "dx = -200, dy = 1"), "-200 is past");
        // EXE6 has no auto battle: its sides take no such fact.
        let six = exe6_content();
        let good = write(&six, &crate::pick::live(&six, "exe6", 2, None).unwrap());
        assert!(!good.contains("auto_battle"), "{good}");
        let problems = parse(&six, &good.replacen("[left]\n", "[left]\nauto_battle_places = [{}]\n", 1)).unwrap_err();
        assert!(problems.iter().any(|p| p.starts_with("left: no field \"auto_battle_places\"")), "{problems:?}");
    }

    /// A version is stated where the game's rules take one, and nowhere
    /// else: a new EXE6 match's sides have none until each is given its own
    /// (the checks refuse the match: none is assumed), a random one's are
    /// picked and written; an EXE5 match has none, and its file takes no
    /// `version`: its rules declare no such fact. EXE6's Crosses are a list
    /// a side states, none when it states none.
    #[test]
    fn a_version_is_stated_where_the_game_takes_one() {
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        let six = exe6_content();
        let new = Match::empty(&six, "exe6").unwrap();
        assert!(new.sides.iter().all(|s| s.version(&six).is_none()));
        let problems = crate::check_match(&six, &new);
        for side in ["left", "right"] {
            has(problems.clone(), &format!("{side}: no version: a side of exe6 states its own (gregar or falzar); none is assumed"));
        }
        assert!(!problems.iter().any(|p| p.contains("crosses")), "no Crosses is none: {problems:?}");
        assert!(!write(&six, &new).contains("version") && !write(&six, &new).contains("crosses"));
        // (Nor does the engine start its round: a player's setup states the
        // version, and nothing fills one in.)
        let refused = crate::check::start(&six, &new).err().expect("no round without the versions");
        assert_eq!(refused, "the round doesn't start: a player's setup doesn't state the rules' `version` (gregar or falzar): none is assumed");
        let picked = write(&six, &crate::pick::live(&six, "exe6", 1, None).unwrap());
        assert_eq!(picked.matches("\nversion = \"falzar\"\n").count() + picked.matches("\nversion = \"gregar\"\n").count(), 2, "{picked}");
        // A version is its name, one of those the game's rules declare
        // (their `version` field's, in its order): a side made in code
        // takes no other, as a file's doesn't.
        // (EXE6's come in the original's order, which numbers them: the
        // byte a navi's stats carry is the version's place.)
        assert_eq!(crate::facts::versions(&six), ["gregar", "falzar"]);
        // (EXE6's rules require one fact: the one enum no default states.)
        assert_eq!(crate::facts::required(&six).iter().map(|f| f.name).collect::<Vec<_>>(), ["version"]);
        let live = crate::pick::live(&six, "exe6", 1, None).unwrap();
        assert!(live.sides.iter().all(|s| crate::facts::versions(&six).iter().any(|v| Some(v.as_str()) == s.version(&six))));
        let mut odd = live.clone();
        let refused = odd.sides[0].set_fact(&six, "version", &[Fact::Name("azure")]).unwrap_err();
        assert_eq!(refused, "setup field `version` has no variant \"azure\"");
        assert_eq!(odd, live);
        // The Crosses: left out, the round starts with none, and a file
        // leaves them out.
        let mut none = new.clone();
        for s in &mut none.sides {
            s.set_fact(&six, "version", &[Fact::Name("falzar")]).unwrap();
        }
        // (The navi's version is the side's version fact, which the round
        // carries: NaviStats+0x20 is compat's to write from it.)
        let b = crate::check::start(&six, &none).unwrap();
        let version = |side: u8| b.fact(side, nettai_battle::content::PlayerFact::Version).and_then(|f| f.name().map(str::to_string));
        assert_eq!((version(0), version(1)), (Some("falzar".to_string()), Some("falzar".to_string())));
        assert!(!write(&six, &none).contains("crosses"));
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
        assert!(b.fact(0, nettai_battle::content::PlayerFact::Version).is_none());
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
        has(bad("navi = \"megaman\"", "navi = \"nobody\""), "left: navi: no navi \"nobody\" in exe6");
        has(bad("navi = \"megaman\"", "navi = \"exe6:megaman\""), "left: navi: no navi \"exe6:megaman\" in exe6"); // (written in full)
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
            "left: no field \"emotion_window_glitch\" (a side of exe6 takes beast_out, bug_frags, crosses, folder, hp, level, navi, navicust_expansions, navicust_programs, patch_cards, reg_up, regular_chip, sp_times, sun, tag_chips, version)",
        );
        let stage = good.lines().find(|l| l.starts_with("stage = ")).unwrap();
        has(bad(stage, "stage = \"moon\""), "round 1: no stage \"moon\" in exe6");
        has(bad(stage, "stage = \"netbattle-43\"\nmoon = 1"), "unknown field `moon`");
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
        has(bad(list, "crosses = [\"heatcros\"]"), "left: crosses: [1]: no form \"heatcros\" in exe6");
        has(bad(list, "crosses = [\"heatcross\", \"heatcross\"]"), "left: crosses: HeatCross is there twice");
        // (The Crosses are forms of the navi's own lists: a Cross's Beast
        // form is a form of EXE6's, and none of them.)
        has(bad(list, "crosses = [\"heatcross-beast\"]"), "left: crosses: heatcross-beast is no form of MegaMan's lists");
        // (Left out, they are none. A list with a gap states what no save
        // has.)
        let none = parse(&content, &good.replacen(&format!("{list}\n"), "", 1)).unwrap();
        assert!(none.sides[0].facts.form_list(&content).is_empty());
        has(bad(list, "crosses = [\"\", \"heatcross\"]"), "left: crosses: an empty entry before HeatCross (a list is filled from the front)");
        has(bad(&format!("{version}\n"), ""), "left: no version: a side of exe6 states its own (gregar or falzar); none is assumed");
        // Thirty copies of a chip.
        let mut m = picked.clone();
        { let mut f = m.sides[0].folder(&content); f.chips = [m.sides[0].folder(&content).chips[0]; 30]; m.sides[0].set_folder(&content, &f).unwrap(); }
        { let mut f = m.sides[0].folder(&content); f.regular = None; m.sides[0].set_folder(&content, &f).unwrap(); }
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
            { let mut f = m.sides[1].folder(&content); f.chips[i] = Some(nettai_battle::custom::FolderChip::new(c, content.chip(c).codes[0])); m.sides[1].set_folder(&content, &f).unwrap(); }
        }
        { let mut f = m.sides[1].folder(&content); f.regular = None; m.sides[1].set_folder(&content, &f).unwrap(); }
        has(crate::check_match(&content, &m), "Mega chips, past the navi's 5");
        // Patch cards past 80 MB; Crosses for a navi without any.
        let mut m = picked.clone();
        let cards = crate::testing::patch_cards(&content, "exe6", "canodumb,amonicul,coldbear,megalian,mettfire,kilplant");
        m.sides[0].set_fact(&content, "patch_cards", &cards).unwrap();
        has(crate::check_match(&content, &m), "left: the patch cards are");
        let mut m = picked.clone();
        let protoman = ids::navi(&content, "exe6", "protoman").unwrap();
        m.sides[1].set_navi(&content, protoman).unwrap();
        has(crate::check_match(&content, &m), "right: crosses: ProtoMan doesn't change form");
    }

    /// A match plays its game's rules: EXE6's take a Cross list and no
    /// souls, and no key of a match names other rules.
    #[test]
    fn a_game_is_its_rules() {
        let content = exe6_content();
        assert!(content.defs.fact_field(nettai_battle::content::PlayerFact::FormList).is_some());
        assert!(!crate::facts::takes(&content, "souls"));
        let good = write(&content, &crate::pick::live(&content, "exe6", 1, None).unwrap());
        let e = parse(&content, &good.replacen("game = \"exe6\"\n", "game = \"exe6\"\nrules = \"stock\"\n", 1)).unwrap_err();
        assert!(e[0].contains("unknown field `rules`"), "{e:?}");
    }

    /// The SP deletion times, Beast Out locked and a level write and read
    /// back; a file that leaves the level out reads as the navi's highest
    /// (a link navi's 14, MegaMan's none).
    #[test]
    fn sp_times_beast_out_and_levels_write_and_read_back() {
        let content = exe6_content();
        let mut m = crate::pick::live(&content, "exe6", 2, None).unwrap();
        m.sides[0].set_fact(&content, "beast_out", &[Fact::Value(Value::Bool(false))]).unwrap();
        m.sides[0].set_level(&content, Some(3)).unwrap();
        let chip = |k: &str| ids::chip(&content, "exe6", k).unwrap();
        let time = |c: &str, frames: i64| Fact::Record(vec![("chip", Fact::Value(Value::Def(nettai_content_api::Registry::Chip, chip(c).0))), ("frames", Fact::Value(Value::Int(frames)))]);
        m.sides[0].facts.set(&content, "sp_times", &[time("heatman-sp", 721), time("blastmn-sp", 1500)]).unwrap();
        let protoman = ids::navi(&content, "exe6", "protoman").unwrap();
        m.sides[1].set_navi(&content, protoman).unwrap();
        m.sides[1].set_fact(&content, "crosses", &[]).unwrap();
        { let mut f = m.sides[1].folder(&content); f.regular = None; m.sides[1].set_folder(&content, &f).unwrap(); }
        assert_eq!(m.sides[1].level(&content), Some(14), "ProtoMan's highest");
        m.sides[1].set_level(&content, Some(5)).unwrap();
        let text = write(&content, &m);
        for line in ["beast_out = false", "level = 3", "chip = \"heatman-sp\"", "frames = 721", "chip = \"blastmn-sp\"", "frames = 1500"] {
            assert!(text.contains(line), "{line}:\n{text}");
        }
        let right = &text[text.find("[right]").unwrap()..];
        assert!(right.contains("level = 5"), "{right}");
        assert_eq!(parse(&content, &text).unwrap(), m, "{text}");
        // No level: MegaMan's none, ProtoMan's his highest.
        let no_level = text.replacen("level = 3\n", "", 1);
        assert_eq!(parse(&content, &no_level).unwrap().sides[0].level(&content), None);
        let no_level = format!("{}{}", &text[..text.find("[right]").unwrap()], right.replacen("level = 5\n", "", 1));
        assert_eq!(parse(&content, &no_level).unwrap().sides[1].level(&content), Some(14));
        // Frames past a u16, a chip the game lacks, a field the record lacks.
        let bad = parse(&content, &text.replacen("frames = 721", "frames = 70000", 1)).unwrap_err();
        assert!(bad.iter().any(|p| p.contains("sp_times: 70000 is past a u16")), "{bad:?}");
        let bad = parse(&content, &text.replacen("\"heatman-sp\"", "\"nobody\"", 1)).unwrap_err();
        assert!(bad.iter().any(|p| p.contains("sp_times: [1]: chip: no chip \"nobody\"")), "{bad:?}");
        let bad = parse(&content, &text.replacen("frames = 721", "time = 721", 1)).unwrap_err();
        assert!(bad.iter().any(|p| p.contains("no field `time` (chip, frames)")), "{bad:?}");
    }

    /// A navi code's level is 0 to 14, and a link navi has one.
    #[test]
    fn the_level_is_checked() {
        let content = exe6_content();
        let mut m = crate::pick::live(&content, "exe6", 2, None).unwrap();
        m.sides[0].set_level(&content, Some(15)).unwrap();
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        has(crate::check_match(&content, &m), "left: level 15: a navi code's level is 0 to 14");
        let protoman = ids::navi(&content, "exe6", "protoman").unwrap();
        m.sides[0].set_level(&content, None).unwrap();
        m.sides[1].set_navi(&content, protoman).unwrap();
        m.sides[1].set_fact(&content, "crosses", &[]).unwrap();
        m.sides[1].set_level(&content, None).unwrap();
        let problems = crate::check_match(&content, &m);
        has(problems.clone(), "right: ProtoMan has no level (0 to 14): a link navi exists only through its navi code");
        assert!(!problems.iter().any(|p| p.starts_with("left")), "MegaMan without a code is fine: {problems:?}");
    }
}
