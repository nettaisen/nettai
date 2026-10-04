//! The match file: a [`Match`] as TOML (docs/frontend.md §6). It names its
//! game once, at the top; everything else is a name in that game's
//! namespace (`cannon`, `megaman`: `crate::ids`), and a name the game
//! hasn't is said as such ("no chip "x" in bn6"). There is no way to name
//! another game's.
//!
//! ```toml
//! game = "bn6"                       # the match's game: everything below is its
//! ruleset = "stock"                  # optional: both sides' rules (else the game's stock)
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
//! [left.tactics]                     # optional: BN5's computer-navi data, the save's (none: empty)
//! entries = ["cannon", "pattern 1", "nothing", "empty"]   # up to 42: a chip, a pattern by its number, 0, 0xFFFF
//! patterns = [{ dx = 1, dy = 0, chips = ["sword", "wideswrd"] }]   # up to 8, each up to 6 chips
//!
//! # In a BN5 match ([left] of game = "bn5") a side may say besides:
//! karma = 100                        # optional: the light/dark value, 0 to 1000 (default 500; dark under 470)
//! souls = ["protosoul"]              # optional: the souls it has, either version's (none: every soul)
//! ```

use crate::{Arena, Folder, Match, Place, Side, ids, stats};
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::folder::FOLDER_SIZE;
use crate::CrossList;
use nettai_battle::custom::{FolderChip, GameVersion};
use nettai_battle::navicust::{NaviCust, PlacedProgram};
use nettai_battle::patch_cards::InstalledCard;
use nettai_battle::setup::SpTimes;
use nettai_battle::tactics::{Tactic, TacticPattern, Tactics};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchFile {
    pub game: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruleset: Option<String>,
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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub emotion_window_glitch: bool,
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
    pub tactics: Option<TacticsFile>,
}

/// A player's tactics (`nettai_battle::tactics`): the entries, each a chip's
/// name, `pattern N` (from 1), `nothing` (a save's 0) or `empty` (0xFFFF), and
/// the patterns.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TacticsFile {
    #[serde(default)]
    pub entries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patterns: Vec<PatternFile>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternFile {
    pub dx: i8,
    pub dy: i8,
    #[serde(default)]
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

/// What names nothing in `game`: "no chip "x" in bn6", and the game's
/// names of that kind when they are few enough to read.
fn unknown(what: &str, name: &str, game: &str, have: &[&str]) -> String {
    if have.is_empty() || have.len() > 40 {
        format!("no {what} {name:?} in {game}")
    } else {
        format!("no {what} {name:?} in {game} ({game}'s are {})", have.join(", "))
    }
}

/// The local names of `game`'s definitions among `keys`.
fn names_in<'k>(game: &str, keys: impl Iterator<Item = &'k str>) -> Vec<&'k str> {
    keys.filter(|k| ids::in_game(game, k)).map(ids::local).collect()
}

/// Each name a file says, resolved in its game: the match, or every
/// problem with it.
pub fn resolve(content: &Content, f: &MatchFile) -> Result<Match, Vec<String>> {
    let mut problems = Vec::new();
    let games = ids::games(content);
    if !games.contains(&f.game) {
        return Err(vec![format!("no game {:?} (the content's are {})", f.game, games.join(", "))]);
    }
    let Some(ruleset) = resolve_ruleset(content, &f.game, f.ruleset.as_deref(), &mut problems) else {
        return Err(problems);
    };
    let arena = resolve_arena(content, &f.game, ruleset, &f.arena, &mut problems);
    let Some(arena) = arena else { return Err(problems) };
    let left = resolve_side(content, &arena.game, &f.left, "left", &mut problems);
    let right = resolve_side(content, &arena.game, &f.right, "right", &mut problems);
    match (left, right) {
        (Some(left), Some(right)) if problems.is_empty() => Ok(Match { seed: f.seed, arena, sides: [left, right] }),
        _ => Err(problems),
    }
}

/// `game`'s ruleset named `name` (none: its stock one).
pub fn resolve_ruleset(content: &Content, game: &str, name: Option<&str>, problems: &mut Vec<String>) -> Option<nettai_content_api::RulesetHandle> {
    let found = match name {
        None => crate::stock_ruleset(content, game).map_err(|e| vec![e]),
        Some(n) => ids::ruleset(content, game, n).ok_or_else(|| {
            let have = names_in(game, content.defs.rulesets.iter().map(|r| r.key.as_str()));
            vec![unknown("ruleset", n, game, &have)]
        }),
    };
    found.map_err(|e| problems.extend(e)).ok()
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

/// A file's arena, of `game` by `ruleset`.
pub fn resolve_arena(content: &Content, game: &str, ruleset: nettai_content_api::RulesetHandle, a: &ArenaFile, problems: &mut Vec<String>) -> Option<Arena> {
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
    Some(Arena { game: game.to_string(), ruleset, first: first?, later: later? })
}

/// A file's side of a match of `game`, each name the game's.
pub fn resolve_side(content: &Content, game: &str, s: &SideFile, at: &str, problems: &mut Vec<String>) -> Option<Side> {
    let start = problems.len();
    let mut say = |p: String| problems.push(format!("{at}: {p}"));
    let navi = ids::navi(content, game, &s.navi);
    if navi.is_none() {
        let have = names_in(game, content.defs.navis.iter().map(|n| n.key.as_str()));
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
        if forms.len() > bn6_compat::unlocks::CROSSES {
            say(format!("{} Crosses: a Cross window offers {}", forms.len(), bn6_compat::unlocks::CROSSES));
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
    let tactics = s.tactics.as_ref().map(|t| resolve_tactics(content, game, t, &mut say)).unwrap_or_default();
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
        emotion_window_glitch: s.emotion_window_glitch,
        folder: folder?,
        crosses,
        beast_out: s.beast_out,
        cards,
        navi_level,
        bug_frags: s.bug_frags.unwrap_or(0),
        sp_times,
        navicust,
        tactics,
        karma: s.karma.unwrap_or(crate::facts::DEFAULT_KARMA),
        souls,
    })
}

/// A file's tactics: what names nothing in `game` is said, and left out.
fn resolve_tactics(content: &Content, game: &str, t: &TacticsFile, say: &mut impl FnMut(String)) -> Tactics {
    let chip = |name: &str, say: &mut dyn FnMut(String)| {
        let c = ids::chip(content, game, name);
        if c.is_none() {
            say(format!("tactics: {}", unknown("chip", name, game, &[])));
        }
        c
    };
    let mut entries = Vec::with_capacity(t.entries.len());
    for e in &t.entries {
        let e = e.trim();
        let entry = match e {
            "nothing" => Some(Tactic::Nothing),
            "empty" => Some(Tactic::Empty),
            _ => match e.strip_prefix("pattern ") {
                Some(n) => match n.trim().parse::<u8>() {
                    Ok(n) if n >= 1 => Some(Tactic::Pattern(n - 1)),
                    _ => {
                        say(format!("tactics: {e:?} is no pattern's number (from 1)"));
                        None
                    }
                },
                None => chip(e, say).map(Tactic::Chip),
            },
        };
        entries.extend(entry);
    }
    let patterns = t
        .patterns
        .iter()
        .map(|p| TacticPattern { dx: p.dx, dy: p.dy, chips: p.chips.iter().filter_map(|k| chip(k, say)).collect() })
        .collect();
    Tactics { entries, patterns }
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
        emotion_window_glitch: s.emotion_window_glitch,
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
        tactics: (s.tactics != Tactics::default()).then(|| TacticsFile {
            entries: s
                .tactics
                .entries
                .iter()
                .map(|e| match *e {
                    Tactic::Chip(c) => name(&content.defs.chip(c).key),
                    Tactic::Pattern(i) => format!("pattern {}", i as u16 + 1),
                    Tactic::Nothing => "nothing".into(),
                    Tactic::Empty => "empty".into(),
                })
                .collect(),
            patterns: s
                .tactics
                .patterns
                .iter()
                .map(|p| PatternFile { dx: p.dx, dy: p.dy, chips: p.chips.iter().map(|&c| name(&content.defs.chip(c).key)).collect() })
                .collect(),
        }),
        stats: s.stats_block(content),
        karma: (s.karma != crate::facts::DEFAULT_KARMA).then_some(s.karma),
        souls: s.souls.as_ref().map(|l| l.iter().map(|&f| name(&content.defs.form(f).key)).collect()),
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

/// A match's arena as a file writes it (its places; the game and the
/// ruleset are the file's own keys).
pub fn arena_file(content: &Content, a: &Arena) -> ArenaFile {
    let place = |p: &Place| PlaceFile { stage: ids::local(&content.defs.stage(p.stage).key).to_string(), background: p.background.clone() };
    let first = place(&a.first);
    let later = (a.later != [a.first.clone(), a.first.clone()]).then(|| a.later.iter().map(place).collect());
    ArenaFile { stage: first.stage, background: first.background, later }
}

/// The name a file gives `game`'s `ruleset`: none for the game's stock one.
pub fn ruleset_name(content: &Content, game: &str, ruleset: nettai_content_api::RulesetHandle) -> Option<String> {
    (crate::stock_ruleset(content, game).ok() != Some(ruleset)).then(|| ids::local(&content.defs.ruleset(ruleset).key).to_string())
}

/// A match as a file.
pub fn to_file(content: &Content, m: &Match) -> MatchFile {
    MatchFile {
        game: m.arena.game.clone(),
        ruleset: ruleset_name(content, &m.arena.game, m.arena.ruleset),
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

/// The game a match file's text names (`game = "bn6"`), read before the
/// content it needs is loaded: or why it names none.
pub fn game_of(text: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct Game {
        game: Option<String>,
    }
    let g: Game = toml::from_str(text).map_err(|e| e.to_string())?;
    g.game.ok_or_else(|| "the match file names no game (game = \"bn6\" at its top)".into())
}

/// A match file's text.
pub fn write(content: &Content, m: &Match) -> String {
    let body = toml::to_string_pretty(&to_file(content, m)).expect("a match file serializes");
    format!("# A nettai match (docs/frontend.md §6): play it with `nettai-frontend --match FILE`.\n\n{}", folders_inline(&body))
}

/// `body` with each folder entry (`["cannon", "A"]`) on a line of its
/// own: the pretty printer spreads every element of a nested array over
/// lines.
fn folders_inline(body: &str) -> String {
    let mut doc: toml_edit::DocumentMut = body.parse().expect("a match file parses");
    for side in ["left", "right"] {
        let Some(folder) = doc.get_mut(side).and_then(|s| s.get_mut("folder")).and_then(|f| f.as_array_mut()) else { continue };
        for entry in folder.iter_mut() {
            if let Some(pair) = entry.as_array_mut() {
                pair.set_trailing_comma(false);
                pair.set_trailing("");
                for (i, v) in pair.iter_mut().enumerate() {
                    v.decor_mut().set_prefix(if i == 0 { "" } else { " " });
                    v.decor_mut().set_suffix("");
                }
            }
            entry.decor_mut().set_prefix("\n    ");
        }
        folder.set_trailing_comma(true);
        folder.set_trailing("\n");
    }
    doc.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::bn6_content;

    /// Live play's draw, written as a match file and read back, is the same
    /// match, and starts the same round.
    #[test]
    fn a_drawn_match_writes_and_reads_back() {
        let content = bn6_content();
        for seed in 0..6 {
            let m = crate::draw::live(&content, "bn6", seed, None).unwrap();
            let text = write(&content, &m);
            let back = parse(&content, &text).unwrap_or_else(|e| panic!("seed {seed}: {e:?}\n{text}"));
            assert_eq!(back, m, "seed {seed}:\n{text}");
            assert_eq!(format!("{:?}", back.round(&content, seed)), format!("{:?}", m.round(&content, seed)));
        }
        let text = write(&content, &crate::draw::live(&content, "bn6", 3, None).unwrap());
        for line in ["game = \"bn6\"", "[arena]", "[left]", "folder = [\n    [\"", "\", \"", "[left.stats]", "hp = 1000", "regular_memory = 50"] {
            assert!(text.contains(line), "{line}:\n{text}");
        }
        // Every name is the game's own, written once with the game.
        assert!(!text.contains("bn6:") && !text.contains("ruleset"), "{text}");
        assert_eq!(game_of(&text).unwrap(), "bn6");
        assert!(game_of("[arena]\nstage = \"x\"\n").is_err());
    }

    /// A side's tactics (BN5's computer-navi data) write and read back, and
    /// what they get wrong is said.
    #[test]
    fn tactics_write_and_read_back() {
        let content = bn6_content();
        let mut m = crate::draw::live(&content, "bn6", 2, None).unwrap();
        let chip = |name: &str| ids::chip(&content, "bn6", name).unwrap();
        m.sides[0].tactics = Tactics {
            entries: vec![Tactic::Chip(chip("cannon")), Tactic::Pattern(0), Tactic::Nothing, Tactic::Empty],
            patterns: vec![TacticPattern { dx: 1, dy: -1, chips: vec![chip("sword"), chip("cannon")] }],
        };
        let text = write(&content, &m);
        assert!(text.contains("[left.tactics]") && text.contains("\"pattern 1\"") && !text.contains("[right.tactics]"), "{text}");
        let back = parse(&content, &text).unwrap_or_else(|e| panic!("{e:?}\n{text}"));
        assert_eq!(back, m);
        // The round sends them: packed, the empty place gone.
        let sent = &m.round(&content, 2).players[0].tactics;
        assert_eq!(sent.entries.len(), 3);
        assert_eq!(sent.patterns, m.sides[0].tactics.patterns);
        let bad = text.replacen("\"pattern 1\"", "\"pattern 2\"", 1);
        let problems = parse(&content, &bad).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("the tactics name pattern 2")), "{problems:?}");
        let bad = text.replacen("entries = [\n    \"cannon\"", "entries = [\n    \"nothing-at-all\"", 1);
        let problems = parse(&content, &bad).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("tactics: no chip \"nothing-at-all\" in bn6")), "{problems:?}\n{text}");
    }

    /// What a file can get wrong is said, with where it is.
    #[test]
    fn problems_are_said() {
        let content = bn6_content();
        let drawn = crate::draw::live(&content, "bn6", 1, None).unwrap();
        let good = write(&content, &drawn);
        let bad = |from: &str, to: &str| -> Vec<String> {
            assert!(good.contains(from), "{from}");
            parse(&content, &good.replacen(from, to, 1)).unwrap_err()
        };
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        has(bad("navi = \"megaman\"", "navi = \"nobody\""), "left: no navi \"nobody\" in bn6");
        has(bad("navi = \"megaman\"", "navi = \"bn6:megaman\""), "left: no navi \"bn6:megaman\" in bn6"); // (written in full)
        has(bad("hp = 1000", "hp = 100000"), "stats: hp takes a whole number");
        has(bad("hp = 1000", "hp = 1000\natack = 1"), "no stat \"atack\"");
        let stage = good.lines().find(|l| l.starts_with("stage = ")).unwrap();
        has(bad(stage, "stage = \"moon\""), "arena: no stage \"moon\" in bn6");
        has(bad("game = \"bn6\"", "game = \"bn7\""), "no game \"bn7\"");
        has(bad("navi = \"megaman\"", "navi = \"megaman\"\nversion = \"azure\""), "no version \"azure\"");
        // Thirty copies of a chip.
        let mut m = drawn.clone();
        m.sides[0].folder.chips = [m.sides[0].folder.chips[0]; 30];
        m.sides[0].folder.regular = None;
        has(crate::check_match(&content, &m), "left: folder: 30 copies of");
        // A Mega chip past the navi's Mega level.
        let mut m = drawn.clone();
        m.sides[1].stats.mega_level = 0;
        let megas = m.sides[1].folder.chips().filter(|c| content.chip(c.id).class == nettai_battle::content::ChipClass::Mega).count();
        if megas > 0 {
            has(crate::check_match(&content, &m), "Mega chips, past the navi's 0");
        }
        // Patch cards past 80 MB; a Cross list for a navi without Crosses.
        let mut m = drawn.clone();
        m.sides[0].cards = crate::patch_cards(&content, "bn6", "canodumb,amonicul,coldbear,megalian,mettfire,kilplant").unwrap();
        has(crate::check_match(&content, &m), "left: the patch cards are");
        let mut m = drawn.clone();
        let protoman = ids::navi(&content, "bn6", "protoman").unwrap();
        m.sides[1].navi = protoman;
        m.sides[1].stats = crate::Side::base_stats(&content, protoman, m.sides[1].game);
        has(crate::check_match(&content, &m), "right: a Cross list, but ProtoMan doesn't change form");
    }

    /// A ruleset without the forms system has no Crosses (the test
    /// content's mix).
    #[test]
    fn crosses_need_the_forms_system() {
        let content = nettai_battle::content::testing::content();
        let mix = content.defs.ruleset_by_key("test-mix").unwrap();
        let stock = content.defs.stock_ruleset().unwrap();
        assert!(crate::ruleset_has_system(&content, stock, crate::FORMS_SYSTEM));
        assert!(!crate::ruleset_has_system(&content, mix, crate::FORMS_SYSTEM));
    }

    /// The SP deletion times, Beast Out locked and a level write and read
    /// back; a navi's own default level (a link navi's 0, MegaMan's none) is
    /// left out, and a file without one reads as it.
    #[test]
    fn sp_times_beast_out_and_levels_write_and_read_back() {
        let content = bn6_content();
        let mut m = crate::draw::live(&content, "bn6", 2, None).unwrap();
        m.sides[0].beast_out = false;
        m.sides[0].navi_level = Some(3);
        m.sides[0].sp_times.0[0] = 721;
        m.sides[0].sp_times.0[11] = 1500;
        let protoman = ids::navi(&content, "bn6", "protoman").unwrap();
        m.sides[1].navi = protoman;
        m.sides[1].crosses = None;
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
        let content = bn6_content();
        let mut m = crate::draw::live(&content, "bn6", 2, None).unwrap();
        m.sides[0].navi_level = Some(15);
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        has(crate::check_match(&content, &m), "left: level 15: a navi code's level is 0 to 14");
        let protoman = ids::navi(&content, "bn6", "protoman").unwrap();
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
