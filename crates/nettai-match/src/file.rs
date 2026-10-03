//! The match file: a [`Match`] as TOML, by content key (docs/frontend.md
//! §6).
//!
//! ```toml
//! seed = 42                          # optional: the setup's and battle's seed
//!
//! [arena]                            # the stage's game decides the battle's data
//! stage = "bn6:netbattle-43"
//! background = "honeycomb"           # optional: else the stage's own
//! later = [                          # optional: the set's later rounds (else the first's)
//!     { stage = "bn6:netbattle-12", background = "code" },
//!     { stage = "bn6:netbattle-7" },
//! ]
//!
//! [left]                             # you, side 0; then [right]
//! ruleset = "bn6:bn6"                # optional: else the content's stock ruleset
//! navi = "bn6:megaman"
//! game = "falzar"                    # or "gregar"
//! crosses = ["bn6:heatcross", "bn6:spoutcross"]   # optional: else the game's own five
//! cards = [{ card = "bn6:canodumb" }, { card = "bn6:shadow", on = false }]
//!
//! [left.folder]
//! chips = ["bn6:cannon A", "bn6:cannon A", ...]   # 30, each "key code"
//! regular = 4                        # optional: an entry, counting from 0
//! tags = [5, 6]                      # optional
//!
//! [left.stats]                       # optional: over the navi's fresh stats (crate::stats)
//! hp = 1000
//! regular_memory = 50
//!
//! [left.navicust]                    # optional: the NaviCust, which the rules compile
//! expansions = 2                     # optional: the board's (else the largest)
//! programs = [                       # in the list's order; x, y the center on the 7x7 grid
//!     { program = "bn6:suprarmr", color = "red", x = 3, y = 3, rotation = 1, compressed = true },
//! ]
//! ```

use crate::{Arena, Match, Place, Side, stats};
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::folder::FOLDER_SIZE;
use nettai_battle::custom::{CrossList, FolderChip, GameVersion, SavedFolder};
use nettai_battle::navicust::{NaviCust, PlacedProgram};
use nettai_battle::patch_cards::InstalledCard;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchFile {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruleset: Option<String>,
    pub navi: String,
    #[serde(default = "falzar", skip_serializing_if = "is_falzar")]
    pub game: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bug_frags: Option<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub emotion_window_glitch: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crosses: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<CardFile>,
    pub folder: FolderFile,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub stats: BTreeMap<String, toml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navicust: Option<NaviCustFile>,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderFile {
    pub chips: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<[u8; 2]>,
}

/// The game's name in a file.
pub fn game_name(g: GameVersion) -> &'static str {
    match g {
        GameVersion::Gregar => "gregar",
        GameVersion::Falzar => "falzar",
    }
}

/// A folder entry as a file writes it: the chip's key and its code.
pub fn chip_entry(content: &Content, c: FolderChip) -> String {
    format!("{} {}", content.defs.chip(c.id).key, c.code.letter())
}

/// Each key a file names, resolved: the match, or every problem with it.
pub fn resolve(content: &Content, f: &MatchFile) -> Result<Match, Vec<String>> {
    let mut problems = Vec::new();
    let arena = resolve_arena(content, &f.arena, &mut problems);
    let left = resolve_side(content, &f.left, "left", &mut problems);
    let right = resolve_side(content, &f.right, "right", &mut problems);
    match (arena, left, right) {
        (Some(arena), Some(left), Some(right)) if problems.is_empty() => Ok(Match { seed: f.seed, arena, sides: [left, right] }),
        _ => Err(problems),
    }
}

fn resolve_place(content: &Content, stage: &str, background: &Option<String>, at: &str, problems: &mut Vec<String>) -> Option<Place> {
    let stage = match content.defs.stage_by_key(stage) {
        Some(s) => Some(s),
        None => {
            problems.push(format!("{at}: no stage {stage:?}"));
            None
        }
    };
    if let Some(b) = background
        && crate::background(content, b).is_none()
    {
        problems.push(format!("{at}: no background {b:?}"));
    }
    Some(Place { stage: stage?, background: background.clone() })
}

fn resolve_arena(content: &Content, a: &ArenaFile, problems: &mut Vec<String>) -> Option<Arena> {
    let first = resolve_place(content, &a.stage, &a.background, "arena", problems);
    let later = match &a.later {
        None => first.clone().map(|p| [p.clone(), p]),
        Some(list) if list.len() == 2 => {
            let l: Vec<Option<Place>> = list
                .iter()
                .enumerate()
                .map(|(i, p)| resolve_place(content, &p.stage, &p.background, &format!("arena: later round {}", i + 2), problems))
                .collect();
            Some([l[0].clone()?, l[1].clone()?])
        }
        Some(list) => {
            problems.push(format!("arena: later names {} places; a set's later rounds are two", list.len()));
            None
        }
    };
    Some(Arena { first: first?, later: later? })
}

fn resolve_side(content: &Content, s: &SideFile, at: &str, problems: &mut Vec<String>) -> Option<Side> {
    let start = problems.len();
    let mut say = |p: String| problems.push(format!("{at}: {p}"));
    let ruleset = match &s.ruleset {
        None => None,
        Some(k) => match content.defs.ruleset_by_key(k) {
            Some(r) => Some(r),
            None => {
                let have: Vec<&str> = content.defs.rulesets.iter().map(|r| r.key.as_str()).collect();
                say(format!("no ruleset {k:?} (the content's are {})", have.join(", ")));
                None
            }
        },
    };
    let navi = content.defs.navi_by_key(&s.navi);
    if navi.is_none() {
        let have: Vec<&str> = content.defs.navis.iter().map(|n| n.key.as_str()).collect();
        say(format!("no navi {:?} (the content's are {})", s.navi, have.join(", ")));
    }
    let game = match s.game.as_str() {
        "falzar" => GameVersion::Falzar,
        "gregar" => GameVersion::Gregar,
        g => {
            say(format!("no game {g:?} (falzar or gregar)"));
            GameVersion::Falzar
        }
    };
    let crosses = s.crosses.as_ref().map(|list| {
        let forms: Vec<_> = list
            .iter()
            .filter_map(|k| {
                let f = content.defs.form_by_key(k);
                if f.is_none() {
                    say(format!("no Cross {k:?}"));
                }
                f
            })
            .collect();
        if forms.len() > nettai_battle::custom::screen::CROSSES {
            say(format!("{} Crosses: a Cross window offers {}", forms.len(), nettai_battle::custom::screen::CROSSES));
            CrossList::default()
        } else {
            CrossList::new(&forms)
        }
    });
    let mut cards = Vec::new();
    for c in &s.cards {
        match content.defs.patch_card_by_key(&c.card) {
            Some(card) => cards.push(InstalledCard { card, enabled: c.on }),
            None => say(format!("no patch card {:?}", c.card)),
        }
    }
    let folder = resolve_folder(content, &s.folder, &mut say);
    let navicust = match &s.navicust {
        None => None,
        Some(n) => {
            let mut parts = Vec::with_capacity(n.programs.len());
            for (i, p) in n.programs.iter().enumerate() {
                let Some(program) = content.defs.navicust_program_by_key(&p.program) else {
                    say(format!("navicust program {}: no NaviCust program {:?}", i + 1, p.program));
                    continue;
                };
                let def = content.defs.navicust_program(program);
                let Some(color) = def.colors.iter().position(|c| *c == p.color) else {
                    say(format!("navicust program {}: {} comes in {}, not {:?}", i + 1, def.key, def.colors.join(", "), p.color));
                    continue;
                };
                parts.push(PlacedProgram { program, color: color as u8, x: p.x, y: p.y, rotation: p.rotation, compressed: p.compressed });
            }
            let expansions = n.expansions.unwrap_or_else(|| {
                let rules = &content.rules_of(content.defs.ruleset_game(ruleset)).navicust;
                rules.boards.len().saturating_sub(1) as u8
            });
            match NaviCust::new(&parts, expansions) {
                Ok(n) => Some(n),
                Err(e) => {
                    say(e);
                    None
                }
            }
        }
    };
    let navi = navi?;
    let mut stats = Side::base_stats(content, navi, game);
    for p in stats::apply(content, &s.stats, &mut stats) {
        say(format!("stats: {p}"));
    }
    let stats = crate::starting(content, stats, game);
    if problems.len() > start {
        return None;
    }
    Some(Side {
        ruleset,
        navi,
        game,
        stats,
        emotion_window_glitch: s.emotion_window_glitch,
        folder: folder?,
        crosses,
        cards,
        navi_level: s.level.unwrap_or(0),
        bug_frags: s.bug_frags.unwrap_or(0),
        navicust,
    })
}

fn resolve_folder(content: &Content, f: &FolderFile, say: &mut impl FnMut(String)) -> Option<SavedFolder> {
    if f.chips.len() != FOLDER_SIZE {
        say(format!("the folder has {} chips; a folder is {FOLDER_SIZE}", f.chips.len()));
        return None;
    }
    let mut chips = Vec::with_capacity(FOLDER_SIZE);
    for (i, entry) in f.chips.iter().enumerate() {
        let parsed = entry.rsplit_once(' ').and_then(|(key, code)| {
            let mut letters = code.chars();
            let code = letters.next().and_then(ChipCode::from_letter).filter(|_| letters.next().is_none())?;
            Some((key.trim(), code))
        });
        let Some((key, code)) = parsed else {
            say(format!("folder entry {i}: {entry:?} is not \"<chip> <code>\" (a code is A-Z or *)"));
            continue;
        };
        match content.defs.chip_by_key(key) {
            Some(id) => chips.push(FolderChip::new(id, code)),
            None => say(format!("folder entry {i}: no chip {key:?}")),
        }
    }
    let chips: [FolderChip; FOLDER_SIZE] = chips.try_into().ok()?;
    Some(SavedFolder { chips, regular: f.regular, tags: f.tags.map(|[a, b]| (a, b)) })
}

/// A match as a file.
pub fn to_file(content: &Content, m: &Match) -> MatchFile {
    let place = |p: &Place| PlaceFile { stage: content.defs.stage(p.stage).key.clone(), background: p.background.clone() };
    let first = place(&m.arena.first);
    let later = (m.arena.later != [m.arena.first.clone(), m.arena.first.clone()]).then(|| m.arena.later.iter().map(place).collect());
    let side = |s: &Side| SideFile {
        ruleset: s.ruleset.map(|r| content.defs.ruleset(r).key.clone()),
        navi: content.defs.navi(s.navi).key.clone(),
        game: game_name(s.game).into(),
        level: (s.navi_level != 0).then_some(s.navi_level),
        bug_frags: (s.bug_frags != 0).then_some(s.bug_frags),
        emotion_window_glitch: s.emotion_window_glitch,
        crosses: s.crosses.map(|l| l.forms().map(|f| content.defs.form(f).key.clone()).collect()),
        cards: s.cards.iter().map(|c| CardFile { card: content.defs.patch_card(c.card).key.clone(), on: c.enabled }).collect(),
        folder: FolderFile {
            chips: s.folder.chips.iter().map(|&c| chip_entry(content, c)).collect(),
            regular: s.folder.regular,
            tags: s.folder.tags.map(|(a, b)| [a, b]),
        },
        stats: s.stats_block(content),
        navicust: s.navicust.map(|n| NaviCustFile {
            expansions: Some(n.expansions),
            programs: n
                .iter()
                .map(|p| {
                    let def = content.defs.navicust_program(p.program);
                    ProgramFile {
                        program: def.key.clone(),
                        color: def.colors.get(p.color as usize).cloned().unwrap_or_default(),
                        x: p.x,
                        y: p.y,
                        rotation: p.rotation,
                        compressed: p.compressed,
                    }
                })
                .collect(),
        }),
    };
    MatchFile {
        seed: m.seed,
        arena: ArenaFile { stage: first.stage, background: first.background, later },
        left: side(&m.sides[0]),
        right: side(&m.sides[1]),
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

/// A match file's text.
pub fn write(content: &Content, m: &Match) -> String {
    let body = toml::to_string_pretty(&to_file(content, m)).expect("a match file serializes");
    format!("# A nettai match (docs/frontend.md §6): play it with `nettai-frontend --match FILE`.\n\n{body}")
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
            let m = crate::draw::live(&content, seed, None).unwrap();
            let text = write(&content, &m);
            let back = parse(&content, &text).unwrap_or_else(|e| panic!("seed {seed}: {e:?}\n{text}"));
            assert_eq!(back, m, "seed {seed}:\n{text}");
            assert_eq!(format!("{:?}", back.round(&content, seed)), format!("{:?}", m.round(&content, seed)));
        }
        let text = write(&content, &crate::draw::live(&content, 3, None).unwrap());
        for line in ["[arena]", "[left]", "[right.folder]", "[left.stats]", "hp = 1000", "regular_memory = 50", "bn6:"] {
            assert!(text.contains(line), "{line}:\n{text}");
        }
    }

    /// What a file can get wrong is said, with where it is.
    #[test]
    fn problems_are_said() {
        let content = bn6_content();
        let drawn = crate::draw::live(&content, 1, None).unwrap();
        let good = write(&content, &drawn);
        let bad = |from: &str, to: &str| -> Vec<String> {
            assert!(good.contains(from), "{from}");
            parse(&content, &good.replacen(from, to, 1)).unwrap_err()
        };
        let has = |problems: Vec<String>, said: &str| assert!(problems.iter().any(|p| p.contains(said)), "{said}: {problems:?}");
        has(bad("navi = \"bn6:megaman\"", "navi = \"bn6:nobody\""), "left: no navi \"bn6:nobody\"");
        has(bad("hp = 1000", "hp = 100000"), "stats: hp takes a whole number");
        has(bad("hp = 1000", "hp = 1000\natack = 1"), "no stat \"atack\"");
        let stage = good.lines().find(|l| l.starts_with("stage = ")).unwrap();
        has(bad(stage, "stage = \"bn6:moon\""), "arena: no stage");
        // Thirty copies of a chip.
        let mut m = drawn.clone();
        m.sides[0].folder.chips = [m.sides[0].folder.chips[0]; 30];
        m.sides[0].folder.regular = None;
        has(crate::check_match(&content, &m), "left: folder: 30 copies of");
        // A Mega chip past the navi's Mega level.
        let mut m = drawn.clone();
        m.sides[1].stats.mega_level = 0;
        let megas = m.sides[1].folder.chips.iter().filter(|c| content.chip(c.id).class == nettai_battle::content::ChipClass::Mega).count();
        if megas > 0 {
            has(crate::check_match(&content, &m), "Mega chips, past the navi's 0");
        }
        // Patch cards past 80 MB; a Cross list for a navi without Crosses.
        let mut m = drawn.clone();
        m.sides[0].cards = crate::patch_cards(&content, "canodumb,amonicul,coldbear,megalian,mettfire,kilplant").unwrap();
        has(crate::check_match(&content, &m), "left: the patch cards are");
        let mut m = drawn.clone();
        let protoman = content.defs.navi_by_key("protoman").unwrap();
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
        let stock = content.defs.stock_ruleset();
        assert!(crate::ruleset_has_system(&content, stock, crate::FORMS_SYSTEM));
        assert!(!crate::ruleset_has_system(&content, Some(mix), crate::FORMS_SYSTEM));
    }
}
