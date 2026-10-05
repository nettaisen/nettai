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
//! background = "honeycomb"           # optional: else the stage's own
//! later = [                          # optional: the set's later rounds (else the first's)
//!     { stage = "netbattle-12", background = "code" },
//!     { stage = "netbattle-7" },
//! ]
//!
//! [left]                             # you, side 0; then [right]
//! navi = "megaman"
//! version = "falzar"                 # optional: or "gregar" (else falzar)
//! level = 7                          # optional: the navi code's level, 0-14 (else a link navi's 0, MegaMan none)
//! crosses = ["heatcross", "spoutcross"]   # optional: else the version's own five
//! beast_out = false                  # optional: else Beast Out is unlocked
//! cards = [{ card = "canodumb" }, { card = "shadow", on = false }]
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
//! [left.stats]                       # optional: over the navi's fresh stats, a link navi's at its level (crate::stats)
//! hp = 1000
//! regular_memory = 50
//!
//! [left.navicust]                    # optional: the NaviCust, which the rules compile
//! expansions = 2                     # optional: the board's (else the largest)
//! programs = [                       # in the list's order; x, y the center on the 7x7 grid
//!     { program = "suprarmr", color = "red", x = 3, y = 3, rotation = 1, compressed = true },
//! ]
//!
//! # In an EXE5 match ([left] of game = "exe5") a side may say besides:
//! karma = 100                        # optional: the light/dark value, 0 to 1000 (default 500; dark under 470)
//! souls = ["protosoul"]              # optional: the souls it has, either version's (none: every soul)
//! soul_unison = false                # optional: no soul button (the save's event flag 0; default true)
//! chaos_unison = false               # optional: no Chaos Unison (the save's event flag 0x236; default true)
//!
//! [left.computer_navi]               # optional: what a computer navi plays from the side's save (none: nothing learned)
//! first = ["cannon"]                 # up to 3 entries it plays first, each a chip or a pattern
//! rest = [                           # up to 39 more; a chip named several times is played that much more often
//!     "sword",
//!     "sword",
//!     { dx = -2, dy = 0, chips = ["sword", "wideswrd"] },   # a pattern: a place from its target, up to 5 chips
//! ]
//! ```

use crate::computer_navi::{ComputerNavi, Pattern, Play};
use crate::{Arena, Folder, Match, Place, Side, ids, stats};
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::folder::FOLDER_SIZE;
use crate::CrossList;
use nettai_battle::custom::{FolderChip, GameVersion};
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SideFile {
    pub navi: String,
    #[serde(default = "falzar", skip_serializing_if = "is_falzar")]
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bug_frags: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crosses: Option<Vec<String>>,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub beast_out: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<CardFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub karma: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub souls: Option<Vec<String>>,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub soul_unison: bool,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub chaos_unison: bool,
    /// The folder's entries, each `[chip, code]` (`[]` empty, while it is
    /// being made).
    pub folder: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<[u8; 2]>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sp_times: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub stats: BTreeMap<String, toml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navicust: Option<NaviCustFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computer_navi: Option<ComputerNaviFile>,
}

/// A player's computer-navi data (`crate::computer_navi`): the entries a
/// computer navi plays first and the rest, each a chip's name (a string) or
/// a pattern (a table: [`PatternFile`]).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerNaviFile {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub first: Vec<toml::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rest: Vec<toml::Value>,
}

/// A pattern: its place from its target (`dx` columns toward the computer
/// navi's enemies, `dy` rows down) and its chips' names.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternFile {
    pub dx: i8,
    pub dy: i8,
    pub chips: Vec<String>,
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

fn falzar() -> String {
    "falzar".into()
}

fn is_falzar(s: &String) -> bool {
    s == "falzar"
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

/// The version's name in a file.
pub fn version_name(g: GameVersion) -> &'static str {
    match g {
        GameVersion::Gregar => "gregar",
        GameVersion::Falzar => "falzar",
    }
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
    let version = match s.version.as_str() {
        "falzar" => GameVersion::Falzar,
        "gregar" => GameVersion::Gregar,
        g => {
            say(format!("no version {g:?} (falzar or gregar)"));
            GameVersion::Falzar
        }
    };
    let crosses = s.crosses.as_ref().map(|list| {
        let forms: Vec<_> = list
            .iter()
            .filter_map(|n| {
                let f = ids::form(content, game, n);
                if f.is_none() {
                    say(unknown("Cross", n, game, &[]));
                }
                f
            })
            .collect();
        if forms.len() > exe6_compat::unlocks::CROSSES {
            say(format!("{} Crosses: a Cross window offers {}", forms.len(), exe6_compat::unlocks::CROSSES));
            CrossList::default()
        } else {
            CrossList::new(&forms)
        }
    });
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
    let computer_navi = s.computer_navi.as_ref().map(|c| resolve_computer_navi(content, game, c, &mut say)).unwrap_or_default();
    // The souls, by name.
    let souls = s.souls.as_ref().map(|names| {
        names
            .iter()
            .filter_map(|n| {
                let f = ids::form(content, game, n);
                if f.is_none() {
                    say(format!("souls: {}", unknown("soul", n, game, &[])));
                }
                f
            })
            .collect()
    });
    let navi = navi?;
    // (No level: a link navi's 0, MegaMan's none; the checks hold it.)
    let navi_level = s.level.or_else(|| crate::default_navi_level(content, navi));
    let mut stats = Side::save_base(content, navi, version, navi_level);
    for p in stats::apply(content, game, &s.stats, &mut stats) {
        say(format!("stats: {p}"));
    }
    let stats = crate::starting(content, stats, version);
    if problems.len() > start {
        return None;
    }
    Some(Side {
        navi,
        game: version,
        stats,
        folder: folder?,
        crosses,
        beast_out: s.beast_out,
        cards,
        navi_level,
        bug_frags: s.bug_frags.unwrap_or(0),
        sp_times,
        navicust,
        computer_navi,
        karma: s.karma.unwrap_or(crate::facts::DEFAULT_KARMA),
        souls,
        soul_unison: s.soul_unison,
        chaos_unison: s.chaos_unison,
    })
}

/// A file's computer-navi data: each entry a chip of `game` by name or a
/// pattern of them. What is neither, and what names nothing in `game`, is
/// said, and left out.
fn resolve_computer_navi(content: &Content, game: &str, c: &ComputerNaviFile, say: &mut impl FnMut(String)) -> ComputerNavi {
    let mut group = |name: &str, entries: &[toml::Value]| -> Vec<Play> {
        let mut out = Vec::with_capacity(entries.len());
        for (i, e) in entries.iter().enumerate() {
            let at = format!("computer_navi: {name} entry {}", i + 1);
            let mut chip = |n: &str| {
                let c = ids::chip(content, game, n);
                if c.is_none() {
                    say(format!("{at}: {}", unknown("chip", n, game, &[])));
                }
                c
            };
            match e {
                toml::Value::String(n) => out.extend(chip(n).map(Play::Chip)),
                toml::Value::Table(_) => match e.clone().try_into::<PatternFile>() {
                    // (A pattern missing a chip the game hasn't is another
                    // pattern: the whole entry is left out.)
                    Ok(p) => {
                        let chips: Vec<_> = p.chips.iter().map(|n| chip(n)).collect();
                        if chips.iter().all(Option::is_some) {
                            out.push(Play::Pattern(Pattern { dx: p.dx, dy: p.dy, chips: chips.into_iter().flatten().collect() }));
                        }
                    }
                    Err(e) => say(format!("{at}: a pattern is {{ dx, dy, chips }}: {}", e.message())),
                },
                other => say(format!("{at}: {other} is neither a chip's name nor a pattern ({{ dx, dy, chips }})")),
            }
        }
        out
    };
    ComputerNavi { first: group("first", &c.first), rest: group("rest", &c.rest) }
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
        version: version_name(s.game).into(),
        // (The navi's default level is left out.)
        level: s.navi_level.filter(|_| s.navi_level != crate::default_navi_level(content, s.navi)),
        bug_frags: (s.bug_frags != 0).then_some(s.bug_frags),
        crosses: s.crosses.map(|l| l.forms().map(|f| name(&content.defs.form(f).key)).collect()),
        beast_out: s.beast_out,
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
        // (Nothing learned is left out.)
        computer_navi: (!s.computer_navi.is_empty()).then(|| {
            let play = |p: &Play| match p {
                Play::Chip(c) => toml::Value::String(name(&content.defs.chip(*c).key)),
                Play::Pattern(p) => {
                    let chips = p.chips.iter().map(|&c| name(&content.defs.chip(c).key)).collect();
                    toml::Value::try_from(PatternFile { dx: p.dx, dy: p.dy, chips }).expect("a pattern serializes")
                }
            };
            ComputerNaviFile { first: s.computer_navi.first.iter().map(play).collect(), rest: s.computer_navi.rest.iter().map(play).collect() }
        }),
        stats: s.stats_block(content),
        karma: (s.karma != crate::facts::DEFAULT_KARMA).then_some(s.karma),
        souls: s.souls.as_ref().map(|l| l.iter().map(|&f| name(&content.defs.form(f).key)).collect()),
        soul_unison: s.soul_unison,
        chaos_unison: s.chaos_unison,
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
    format!("# A nettai match (docs/frontend.md §6): play it with `nettai-frontend --match FILE`.\n\n{}", tidy(&body, &file))
}

/// `list`'s entries each on a line of its own.
fn a_line_each(list: &mut toml_edit::Array) {
    for entry in list.iter_mut() {
        entry.decor_mut().set_prefix("\n    ");
    }
    list.set_trailing_comma(true);
    list.set_trailing("\n");
}

/// A computer navi's entry as a file writes it: a chip's name, or a
/// pattern on one line, its place then its chips.
fn play_item(play: &toml::Value) -> toml_edit::Value {
    match play {
        toml::Value::Table(p) => {
            let mut table = toml_edit::InlineTable::new();
            for key in ["dx", "dy"] {
                table.insert(key, p.get(key).and_then(toml::Value::as_integer).unwrap_or(0).into());
            }
            let chips: toml_edit::Array = p.get("chips").and_then(toml::Value::as_array).into_iter().flatten().filter_map(toml::Value::as_str).collect();
            table.insert("chips", chips.into());
            table.into()
        }
        other => other.as_str().unwrap_or_default().into(),
    }
}

/// `body`, the pretty printer's text of `file`, as a person would lay it
/// out: each folder entry (`["cannon", "A"]`) on a line of its own (the
/// printer spreads every element of a nested array over lines), and the
/// computer navi's entries a line each where there are more than a few, a
/// pattern on one line.
fn tidy(body: &str, file: &MatchFile) -> String {
    let mut doc: toml_edit::DocumentMut = body.parse().expect("a match file parses");
    for (side, of) in [("left", &file.left), ("right", &file.right)] {
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
        // (Written over the printer's, which makes a list of patterns
        // alone a table each.)
        let (Some(data), Some(table)) = (&of.computer_navi, doc.get_mut(side).and_then(|s| s.get_mut("computer_navi")).and_then(|c| c.as_table_mut())) else {
            continue;
        };
        table.set_implicit(false);
        for (name, plays) in [("first", &data.first), ("rest", &data.rest)] {
            if plays.is_empty() {
                continue;
            }
            let mut list: toml_edit::Array = plays.iter().map(play_item).collect();
            if plays.len() > crate::computer_navi::FIRST || plays.iter().any(|p| !p.is_str()) {
                a_line_each(&mut list);
            }
            table.insert(name, toml_edit::value(list));
        }
    }
    doc.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::exe6_content;

    /// Live play's draw, written as a match file and read back, is the same
    /// match, and starts the same round.
    #[test]
    fn a_drawn_match_writes_and_reads_back() {
        let content = exe6_content();
        for seed in 0..6 {
            let m = crate::draw::live(&content, "exe6", seed, None).unwrap();
            let text = write(&content, &m);
            let back = parse(&content, &text).unwrap_or_else(|e| panic!("seed {seed}: {e:?}\n{text}"));
            assert_eq!(back, m, "seed {seed}:\n{text}");
            assert_eq!(format!("{:?}", back.round(&content, seed)), format!("{:?}", m.round(&content, seed)));
        }
        let text = write(&content, &crate::draw::live(&content, "exe6", 3, None).unwrap());
        for line in ["game = \"exe6\"", "[arena]", "[left]", "folder = [\n    [\"", "\", \"", "[left.navicust]", "expansions = 2", "programs = []"] {
            assert!(text.contains(line), "{line}:\n{text}");
        }
        // (MegaMan at his fresh stats: no stats block.)
        assert!(!text.contains("[left.stats]") && !text.contains("[right.stats]"), "{text}");
        // Every name is the game's own, written once with the game.
        assert!(!text.contains("exe6:") && !text.contains("ruleset"), "{text}");
        assert_eq!(game_of(&text).unwrap(), "exe6");
        assert!(game_of("[arena]\nstage = \"x\"\n").is_err());
    }

    /// A side's computer-navi data (EXE5's) writes and reads back, chips by
    /// name and patterns as tables; the round sends it; and what a file gets
    /// wrong is said, with where it is.
    #[test]
    fn computer_navi_data_writes_and_reads_back() {
        use nettai_battle::tactics::{Tactic, TacticPattern};
        let content = crate::testing::exe5_content();
        let mut m = crate::draw::live(&content, "exe5", 2, None).unwrap();
        let chip = |name: &str| ids::chip(&content, "exe5", name).unwrap();
        let pattern = Pattern { dx: -2, dy: 1, chips: vec![chip("sword"), chip("wideswrd")] };
        m.sides[0].computer_navi = ComputerNavi {
            first: vec![Play::Chip(chip("cannon"))],
            rest: vec![Play::Chip(chip("sword")), Play::Pattern(pattern.clone()), Play::Chip(chip("sword"))],
        };
        m.sides[1].computer_navi = ComputerNavi::default();
        let text = write(&content, &m);
        for line in ["[left.computer_navi]", "first = [\"cannon\"]", "    \"sword\",\n", "    { dx = -2, dy = 1, chips = [\"sword\", \"wideswrd\"] },\n"] {
            assert!(text.contains(line), "{line}:\n{text}");
        }
        assert!(!text.contains("[right.computer_navi]") && !text.contains("tactics"), "{text}");
        let back = parse(&content, &text).unwrap_or_else(|e| panic!("{e:?}\n{text}"));
        assert_eq!(back, m);
        // Patterns alone (which the printer would make a table each) are a
        // list as any other.
        let mut patterns = m.clone();
        patterns.sides[1].computer_navi =
            ComputerNavi { first: vec![Play::Pattern(pattern.clone())], rest: vec![Play::Pattern(Pattern { dx: 0, dy: 0, chips: vec![chip("cannon")] })] };
        let listed = write(&content, &patterns);
        let lines = "[right.computer_navi]\nfirst = [\n    { dx = -2, dy = 1, chips = [\"sword\", \"wideswrd\"] },\n]\nrest = [\n    { dx = 0, dy = 0, chips = [\"cannon\"] },\n]\n";
        assert!(listed.contains(lines) && !listed.contains("[[right"), "{listed}");
        assert_eq!(parse(&content, &listed).unwrap_or_else(|e| panic!("{e:?}\n{listed}")), patterns);
        // The round sends it: the entry played first still first, the
        // others after it in an order the seed draws.
        let sent = &m.round(&content, 2).players[0].tactics;
        assert_eq!((sent.entries.len(), sent.entries[0]), (4, Tactic::Chip(chip("cannon"))));
        assert_eq!(sent.patterns, [TacticPattern { dx: -2, dy: 1, chips: pattern.chips.clone() }]);
        assert!(m.round(&content, 2).players[1].tactics.entries.is_empty());
        // What a file gets wrong.
        let bad = |from: &str, to: &str| -> Vec<String> {
            assert!(text.contains(from), "{from}:\n{text}");
            parse(&content, &text.replacen(from, to, 1)).unwrap_err()
        };
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        has(bad("first = [\"cannon\"]", "first = [\"nothing-at-all\"]"), "left: computer_navi: first entry 1: no chip \"nothing-at-all\" in exe5");
        has(bad("first = [\"cannon\"]", "first = [7]"), "left: computer_navi: first entry 1: 7 is neither a chip's name nor a pattern");
        has(bad("first = [\"cannon\"]", "first = [\"cannon\", \"cannon\", \"cannon\", \"cannon\"]"), "left: the computer navi plays 4 entries first");
        has(bad("dx = -2, dy = 1", "dx = -2, dy = 1, dz = 0"), "left: computer_navi: rest entry 2: a pattern is { dx, dy, chips }");
        has(bad("dx = -2, dy = 1", "dx = -2"), "left: computer_navi: rest entry 2: a pattern is { dx, dy, chips }");
        has(bad("chips = [\"sword\", \"wideswrd\"]", "chips = [\"sword\", \"wideswd\"]"), "left: computer_navi: rest entry 2: no chip \"wideswd\" in exe5");
        has(bad("dx = -2", "dx = -6"), "left: the computer navi's pattern 1 is -6 columns and 1 rows from its target");
        has(bad("chips = [\"sword\", \"wideswrd\"]", "chips = []"), "left: the computer navi's pattern 1 has no chips");
        has(bad("first = [\"cannon\"]", "first = [\"cannon\"]\nentries = []"), "unknown field `entries`");
        // EXE6 has no computer navis: its match file takes no such data.
        let six = exe6_content();
        let good = write(&six, &crate::draw::live(&six, "exe6", 2, None).unwrap());
        assert!(!good.contains("computer_navi"), "{good}");
        let problems = parse(&six, &format!("{good}\n[left.computer_navi]\nrest = [\"cannon\"]\n")).unwrap_err();
        assert_eq!(problems, ["left: computer-navi data, but exe6 has no computer navis (no computer-navi system)"]);
    }

    /// What a file can get wrong is said, with where it is.
    #[test]
    fn problems_are_said() {
        let content = exe6_content();
        let drawn = crate::draw::live(&content, "exe6", 1, None).unwrap();
        let good = write(&content, &drawn);
        let bad = |from: &str, to: &str| -> Vec<String> {
            assert!(good.contains(from), "{from}");
            parse(&content, &good.replacen(from, to, 1)).unwrap_err()
        };
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        has(bad("navi = \"megaman\"", "navi = \"nobody\""), "left: no navi \"nobody\" in exe6");
        has(bad("navi = \"megaman\"", "navi = \"exe6:megaman\""), "left: no navi \"exe6:megaman\" in exe6"); // (written in full)
        // (A stats block, after the sides' tables.)
        let stats = |block: &str| parse(&content, &format!("{good}\n[left.stats]\n{block}\n")).unwrap_err();
        has(stats("hp = 100000"), "stats: hp takes a whole number");
        has(stats("hp = 1000\natack = 1"), "no stat \"atack\"");
        // No key takes the emotion window's glitch: the rules make it.
        has(bad("navi = \"megaman\"", "navi = \"megaman\"\nemotion_window_glitch = true"), "unknown field `emotion_window_glitch`");
        let stage = good.lines().find(|l| l.starts_with("stage = ")).unwrap();
        has(bad(stage, "stage = \"moon\""), "arena: no stage \"moon\" in exe6");
        has(bad("game = \"exe6\"", "game = \"bn7\""), "no game \"bn7\"");
        has(bad("navi = \"megaman\"", "navi = \"megaman\"\nversion = \"azure\""), "no version \"azure\"");
        // Thirty copies of a chip.
        let mut m = drawn.clone();
        m.sides[0].folder.chips = [m.sides[0].folder.chips[0]; 30];
        m.sides[0].folder.regular = None;
        has(crate::check_match(&content, &m), "left: folder: 30 copies of");
        // A Mega chip past the navi's Mega level (its stats set directly:
        // no NaviCust).
        let mut m = drawn.clone();
        m.sides[1].navicust = None;
        m.sides[1].stats.mega_level = 0;
        let megas = m.sides[1].folder.chips().filter(|c| content.chip(c.id).class == nettai_battle::content::ChipClass::Mega).count();
        if megas > 0 {
            has(crate::check_match(&content, &m), "Mega chips, past the navi's 0");
        }
        // Patch cards past 80 MB; a Cross list for a navi without Crosses.
        let mut m = drawn.clone();
        m.sides[0].cards = crate::patch_cards(&content, "exe6", "canodumb,amonicul,coldbear,megalian,mettfire,kilplant").unwrap();
        has(crate::check_match(&content, &m), "left: the patch cards are");
        let mut m = drawn.clone();
        let protoman = ids::navi(&content, "exe6", "protoman").unwrap();
        m.sides[1].navi = protoman;
        m.sides[1].stats = crate::Side::base_stats(&content, protoman, m.sides[1].game);
        has(crate::check_match(&content, &m), "right: a Cross list, but ProtoMan doesn't change form");
    }

    /// A game's rules have their systems, by name: EXE6's the forms system
    /// (its Crosses), and no key names another ruleset.
    #[test]
    fn a_game_is_its_rules() {
        let content = exe6_content();
        assert!(crate::ruleset_has_system(&content, crate::FORMS_SYSTEM));
        assert!(!crate::ruleset_has_system(&content, "souls"));
        let good = write(&content, &crate::draw::live(&content, "exe6", 1, None).unwrap());
        let e = parse(&content, &good.replacen("game = \"exe6\"\n", "game = \"exe6\"\nruleset = \"stock\"\n", 1)).unwrap_err();
        assert!(e[0].contains("unknown field `ruleset`"), "{e:?}");
    }

    /// The SP deletion times, Beast Out locked and a level write and read
    /// back; a navi's own default level (a link navi's 0, MegaMan's none) is
    /// left out, and a file without one reads as it.
    #[test]
    fn sp_times_beast_out_and_levels_write_and_read_back() {
        let content = exe6_content();
        let mut m = crate::draw::live(&content, "exe6", 2, None).unwrap();
        m.sides[0].beast_out = false;
        m.sides[0].navi_level = Some(3);
        m.sides[0].sp_times.0[0] = 721;
        m.sides[0].sp_times.0[11] = 1500;
        let protoman = ids::navi(&content, "exe6", "protoman").unwrap();
        m.sides[1].navi = protoman;
        m.sides[1].crosses = None;
        m.sides[1].navicust = None;
        m.sides[1].navi_level = Some(0);
        m.sides[1].stats = crate::Side::save_base(&content, protoman, m.sides[1].game, Some(0));
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
        let mut m = crate::draw::live(&content, "exe6", 2, None).unwrap();
        m.sides[0].navi_level = Some(15);
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        has(crate::check_match(&content, &m), "left: level 15: a navi code's level is 0 to 14");
        let protoman = ids::navi(&content, "exe6", "protoman").unwrap();
        m.sides[0].navi_level = None;
        m.sides[1].navi = protoman;
        m.sides[1].crosses = None;
        m.sides[1].navi_level = None;
        m.sides[1].stats = crate::Side::base_stats(&content, protoman, m.sides[1].game);
        let problems = crate::check_match(&content, &m);
        has(problems.clone(), "right: ProtoMan has no level: a link navi exists only through its navi code");
        assert!(!problems.iter().any(|p| p.starts_with("left")), "MegaMan without a code is fine: {problems:?}");
    }
}
