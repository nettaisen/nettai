//! What a match must be to be played: the checks a match file passes when
//! it loads, a netplay offer when it arrives (`check_side`), and the editor
//! shows as they fail. Each problem is said, with where it is.
//!
//! - **The arena**: the content's game, its link battle stages
//!   (`crate::link_battle_stages`), and backgrounds its pack has.
//! - **A side**: its navi, chips, patch cards, NaviCust programs and souls
//!   are the match's game's, its stats' forms the content's; its
//!   auto battle data what the game can hold (`crate::auto_battle`); a Cross list
//!   only with rules that have the forms system,
//!   each a Cross of the navi's, at most five, none twice; patch cards only
//!   with rules that have the patch-cards system, each installed once, at
//!   most [`MAX_CARDS`], their MB together at most [`CARD_MB`] (EXE6's menu
//!   adds no card past 80 MB, `0x08141868`); a NaviCust only for MegaMan
//!   under rules with the navicust system, on its board (`check_navicust`);
//!   the folder by its own game's rules (the game's rules'
//!   `folder_check`, `crate::folders`: EXE6's folder editor's), on the stats
//!   the round set up (the NaviCust's and the patch cards' folder limits:
//!   the original's folder editor and its link battle check read the stats
//!   the reload made).

use crate::folders;
use crate::{Arena, Match, Place, Side, ids};
use nettai_battle::Battle;
use nettai_battle::content::Content;
use nettai_battle::patch_cards::MAX_CARDS;
use nettai_battle::setup::NaviStats;
use std::sync::Arc;

/// The installed patch cards' MB together at most (EXE6's).
pub const CARD_MB: u32 = 80;

fn check_place(content: &Content, game: &str, p: &Place, at: &str, out: &mut Vec<String>) {
    if p.stage.index() >= content.defs.stages.len() || !ids::in_game(content, game, &content.defs.stage(p.stage).key) {
        out.push(format!("{at}: a stage {game} hasn't"));
        return;
    }
    if !crate::link_battle_stages(content, game).contains(&p.stage) {
        out.push(format!("{at}: {} is no link battle stage", ids::local(&content.defs.stage(p.stage).key)));
    }
    if let Some(b) = &p.background
        && crate::background(content, game, b).is_none()
    {
        out.push(crate::no_background(at, game, b));
    }
}

/// What is wrong with an arena.
pub fn check_arena(content: &Content, a: &Arena) -> Vec<String> {
    let mut out = Vec::new();
    let games = ids::games(content);
    if !games.contains(&a.game) {
        return vec![format!("no game {:?} (the content's are {})", a.game, games.join(", "))];
    }
    check_place(content, &a.game, &a.first, "arena", &mut out);
    for (i, p) in a.later.iter().enumerate() {
        check_place(content, &a.game, p, &format!("arena: later round {}", i + 2), &mut out);
    }
    out
}

/// What is wrong with a side of a match on `arena` (a sound one:
/// `check_arena`) that needs no battle to see.
pub fn check_side_alone(content: &Content, arena: &Arena, s: &Side) -> Vec<String> {
    let mut out = Vec::new();
    let defs = &content.defs;
    let game = arena.game.as_str();
    // Everything the side names is the game's.
    let of_game = |key: &str| ids::in_game(content, game, key);
    if s.navi.index() >= defs.navis.len() || !of_game(&defs.navi(s.navi).key) {
        out.push(format!("a navi {game} hasn't"));
        return out;
    }
    let st = &s.stats;
    if st.navi != s.navi {
        out.push("the stats are another navi's".into());
    }
    if [st.form, st.starting_form].iter().any(|f| f.index() >= defs.forms.len()) {
        out.push("the stats name a form the content hasn't".into());
    }
    let weapons = [st.weapons.buster, st.weapons.charge_shot, st.weapons.back_special, st.weapons.a_charge, st.weapons.mode9_a];
    if weapons.iter().flatten().any(|w| w.index() >= defs.weapons.len()) {
        out.push("the stats name a weapon the content hasn't".into());
    }
    let records = [st.first_barrier, st.weapons.buster_shot, st.weapons.charge_shot_kind];
    if records.iter().flatten().any(|r| r.index() >= defs.records.len()) {
        out.push("the stats name a record the content hasn't".into());
    }
    // The navi code's level: 0 to 14, and a link navi always has one (it
    // exists only through its code); MegaMan may have none.
    // (A navi with a story, EXE5's team navis: its own levels, which its
    // damage rows are read at.)
    let last = crate::story::max_level(content, s.navi);
    match (s.navi_level, last) {
        (Some(l), Some(last)) if l > last => {
            out.push(format!("level {l}: {}'s level is 0 to {last}", crate::names::navi(content, s.navi)))
        }
        (None, Some(last)) => out.push(format!("{} has no level (0 to {last})", crate::names::navi(content, s.navi))),
        (Some(l), _) if l > nettai_battle::custom::MAX_NAVI_LEVEL => {
            out.push(format!("level {l}: a navi code's level is 0 to {}", nettai_battle::custom::MAX_NAVI_LEVEL))
        }
        (None, None) if !content.navi(s.navi).changes_form() => {
            out.push(format!("{} has no level: a link navi exists only through its navi code", crate::names::navi(content, s.navi)))
        }
        _ => {}
    }
    // The auto battle data: what the game can hold of it, where the
    // game has auto battle.
    out.extend(s.auto_battle.check(content, game));
    let foreign = |c: nettai_content_api::ChipHandle| c.index() >= defs.chips.len() || !of_game(&defs.chip(c).key);
    // The karma and the souls.
    out.extend(crate::facts::check(content, arena, s));
    // The folder's chips, before its rules.
    if let Some((i, _)) = s.folder.chips.iter().enumerate().find(|(_, c)| c.is_some_and(|c| foreign(c.id))) {
        out.push(format!("folder entry {i}: a chip {game} hasn't"));
        return out;
    }
    // The Crosses.
    if let Some(list) = &s.crosses {
        if !crate::ruleset_has_system(content, crate::FORMS_SYSTEM) {
            out.push(format!("a Cross list, but {game} has no Crosses (no forms system)"));
        }
        let forms: Vec<_> = list.forms().collect();
        if forms.iter().any(|f| f.index() >= defs.forms.len()) {
            out.push("a Cross the content hasn't".into());
        } else {
            match crate::navi_crosses(content, s.navi) {
                None => out.push(format!("a Cross list, but {} doesn't change form", crate::names::navi(content, s.navi))),
                Some(own) => {
                    for (i, &f) in forms.iter().enumerate() {
                        if !own.contains(&f) {
                            out.push(format!("{} is no Cross of {}'s", crate::names::form(content, f), crate::names::navi(content, s.navi)));
                        }
                        if forms[..i].contains(&f) {
                            out.push(format!("{} is in the Cross list twice", crate::names::form(content, f)));
                        }
                    }
                }
            }
        }
    }
    // The patch cards.
    if !s.cards.is_empty() {
        if !crate::ruleset_has_system(content, crate::PATCH_CARDS_SYSTEM) {
            out.push(format!("patch cards, but {game} has no patch-cards system"));
        }
        if s.cards.len() > MAX_CARDS {
            out.push(format!("{} patch cards installed: a list holds {MAX_CARDS}", s.cards.len()));
        }
        if s.cards.iter().any(|c| c.card.index() >= defs.patch_cards.len() || !of_game(&defs.patch_card(c.card).key)) {
            out.push(format!("a patch card {game} hasn't"));
        } else {
            for (i, c) in s.cards.iter().enumerate() {
                if s.cards[..i].iter().any(|d| d.card == c.card) {
                    out.push(format!("the patch card {} is installed twice", crate::names::patch_card(content, c.card)));
                }
            }
            let mb: u32 = s.cards.iter().map(|c| defs.patch_card(c.card).mb as u32).sum();
            if mb > CARD_MB {
                out.push(format!("the patch cards are {mb} MB, past {CARD_MB}"));
            }
        }
    }
    // The NaviCust.
    if let Some(n) = &s.navicust {
        out.extend(check_navicust(content, arena, s, n));
    }
    // The folder's chips are the content's (its rules wait for the round).
    let in_folder = |i: u8| s.folder.has(i);
    if !s.folder.regular.is_none_or(in_folder) || !s.folder.tags.is_none_or(|(a, b)| in_folder(a) && in_folder(b)) {
        out.push("folder: the Regular or tag chips aren't chips of the folder".into());
    }
    out
}

/// What is wrong with a side's NaviCust: it is MegaMan's (the navi that
/// changes form: EXE6 compiles the PET's own navi's alone) under rules with
/// the navicust system; the stats set besides are only what the save keeps
/// through its compile; each program in one of its colors, on the board of
/// its expansions (EXE6's `sub_813BB00`: every cell it covers on the board or
/// its frame, not all on the frame), over no other (`sub_813BB68`); copies
/// of a program in one color all compressed or not (the save keeps it by
/// program and color: event flag 0x2660 + the part id).
pub fn check_navicust(content: &Content, arena: &Arena, s: &Side, n: &nettai_battle::navicust::NaviCust) -> Vec<String> {
    use nettai_battle::content::NaviCustRules;
    use nettai_battle::navicust::{SIZE, cells};
    let mut out = Vec::new();
    let defs = &content.defs;
    if !crate::ruleset_has_system(content, crate::NAVICUST_SYSTEM) {
        out.push(format!("a NaviCust, but {} has no navicust system", arena.game));
    }
    if content.navi(s.navi).forms.is_none() {
        out.push(format!("a NaviCust, but {}'s stats aren't a NaviCust's (only MegaMan's compiles)", crate::names::navi(content, s.navi)));
    }
    for name in s.stats_block(content).keys() {
        if !crate::stats::SAVE_FIELDS.contains(&name.as_str()) {
            out.push(format!("stats: {name} is the NaviCust's (with a NaviCust the stats set only {})", crate::stats::SAVE_FIELDS.join(", ")));
        }
    }
    let rules = crate::navicust_rules(content);
    let Some(board) = rules.board(n.expansions) else {
        out.push(format!("a NaviCust with {} expansions: the game's board has {} sizes", n.expansions, rules.boards.len()));
        return out;
    };
    let mut grid = [[None; SIZE]; SIZE];
    let mut compression: Vec<((u16, u8), bool)> = Vec::new();
    for (i, p) in n.iter().enumerate() {
        if p.program.index() >= defs.navicust_programs.len() || !ids::in_game(content, &arena.game, &defs.navicust_program(p.program).key) {
            out.push(format!("navicust program {}: a program {} hasn't", i + 1, arena.game));
            continue;
        }
        let def = defs.navicust_program(p.program);
        let name = crate::names::navicust_program(content, p.program);
        if p.color as usize >= def.colors.len() {
            out.push(format!("navicust program {} ({name}): a color it doesn't come in", i + 1));
        }
        if p.rotation > 3 {
            out.push(format!("navicust program {} ({name}): turned {} quarters (0 to 3)", i + 1, p.rotation));
        }
        let shape = def.placed_shape(p.compressed, p.rotation);
        if !NaviCustRules::fits(board, &shape, p.x, p.y) {
            out.push(format!("navicust program {} ({name}) at ({}, {}): off the board", i + 1, p.x, p.y));
            continue;
        }
        for (x, y) in cells(&shape, p.x, p.y) {
            let cell = &mut grid[y as usize][x as usize];
            if let Some(j) = *cell {
                out.push(format!("navicust program {} ({name}) is over program {j}", i + 1));
                break;
            }
            *cell = Some(i + 1);
        }
        let key = (p.program.0, p.color);
        match compression.iter().find(|(k, _)| *k == key) {
            Some(&(_, c)) if c != p.compressed => {
                out.push(format!("navicust program {} ({name}): compressed and not (a save compresses every copy of a program in one color)", i + 1))
            }
            Some(_) => {}
            None => compression.push((key, p.compressed)),
        }
    }
    out
}

/// The round `m` starts, set up (each side's rules have set its stats:
/// the NaviCust, the patch cards), or why it doesn't start.
pub fn start(content: &Arc<Content>, m: &Match) -> Result<Battle, String> {
    // A folder being made has empty entries, and a round is set up with
    // whole ones: here they hold a stand-in (the folder's first chip, else
    // the content's first with a code). Its rules see its own entries
    // (`folder_problems`), and say it isn't whole.
    let mut m = m.clone();
    for s in &mut m.sides {
        if s.folder.saved().is_none() {
            let any = (0..content.defs.chips.len() as u16)
                .map(nettai_content_api::ChipHandle)
                .find_map(|id| content.chip(id).codes.first().map(|&code| nettai_battle::custom::FolderChip::new(id, code)));
            let filler = s.folder.chips().next().or(any).ok_or("the content has no chip with a code")?;
            s.folder = s.folder.filled_with(filler).into();
        }
    }
    let setup = m.round(content, m.seed.unwrap_or(0));
    let started = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Battle::new(setup, content.clone())));
    started.map_err(|e| {
        let why = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
        format!("the round doesn't start: {}", why.unwrap_or_else(|| "the engine stopped".into()))
    })
}

/// The stats each side's round starts with: the setup's, after the rules
/// have set the round up (each side's NaviCust and patch cards), or why the
/// round doesn't start.
pub fn round_stats(content: &Arc<Content>, m: &Match) -> Result<[NaviStats; 2], String> {
    start(content, m).map(|b| b.stats)
}

/// What side `side`'s rules say of its folder in `b`, the round's battle.
fn folder_problems(b: &mut Battle, side: usize, s: &Side) -> Vec<String> {
    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| folders::problems(b, side as u8, &s.folder)));
    match checked {
        Ok(problems) => problems.into_iter().map(|p| format!("folder: {}", p.text)).collect(),
        Err(_) => vec!["folder: the rules stopped checking it".into()],
    }
}

/// What is wrong with one side of a match on `arena`, as a netplay offer
/// brings it: everything [`check_side_alone`] sees, and its folder against
/// the stats its round starts with (the side on both sides of a round on
/// the arena).
pub fn check_side(content: &Arc<Content>, arena: &Arena, s: &Side) -> Vec<String> {
    let mut out = check_arena(content, arena);
    if !out.is_empty() {
        return out;
    }
    out = check_side_alone(content, arena, s);
    if !out.is_empty() {
        return out;
    }
    let m = Match { seed: None, arena: arena.clone(), sides: [s.clone(), s.clone()] };
    match start(content, &m) {
        Ok(mut b) => out.extend(folder_problems(&mut b, 0, s)),
        Err(e) => out.push(e),
    }
    out
}

/// What is wrong with a match, each problem with where it is.
pub fn check_match(content: &Arc<Content>, m: &Match) -> Vec<String> {
    let mut out = check_arena(content, &m.arena);
    if !out.is_empty() {
        return out;
    }
    let sides = ["left", "right"];
    for (s, at) in m.sides.iter().zip(sides) {
        out.extend(check_side_alone(content, &m.arena, s).into_iter().map(|p| format!("{at}: {p}")));
    }
    if !out.is_empty() {
        return out;
    }
    match start(content, m) {
        Ok(mut b) => {
            for (side, (s, at)) in m.sides.iter().zip(sides).enumerate() {
                out.extend(folder_problems(&mut b, side, s).into_iter().map(|p| format!("{at}: {p}")));
            }
        }
        Err(e) => out.push(e),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_battle::navicust::{NaviCust, PlacedProgram};

    /// A MegaMan with a NaviCust of EXE6's programs: the navicust system
    /// compiles it into the stats the round starts with.
    fn compiled(parts: &[(&str, &str, u8, u8)]) -> (NaviStats, bool, Vec<String>) {
        compiled_at(parts, None)
    }

    /// [`compiled`], MegaMan from a navi code of `level`.
    fn compiled_at(parts: &[(&str, &str, u8, u8)], level: Option<u8>) -> (NaviStats, bool, Vec<String>) {
        let content = crate::testing::exe6_content();
        let mut m = crate::draw::live(&content, "exe6", 3, None).unwrap();
        let s = &mut m.sides[0];
        s.navi_level = level;
        s.stats = crate::Side::base_stats(&content, s.navi, s.version);
        (s.stats.max_base_hp, s.stats.hp, s.stats.max_hp, s.stats.reg_up) = (600, 600, 600, 50);
        let placed: Vec<PlacedProgram> = parts
            .iter()
            .map(|&(name, color, x, y)| {
                let program = ids::navicust_program(&content, "exe6", name).unwrap();
                let color = content.defs.navicust_program(program).colors.iter().position(|c| c == color).unwrap() as u8;
                PlacedProgram { program, color, x, y, rotation: 0, compressed: false }
            })
            .collect();
        s.navicust = Some(NaviCust::new(&placed, 2).unwrap());
        let problems = check_match(&content, &m);
        let b = Battle::new(m.round(&content, 3), content.clone());
        (b.stats[0], b.consoles[0].emotion_window_glitch, problems)
    }

    #[test]
    fn a_navicust_compiles_into_the_stats() {
        // UnderSht on the command line, Attack+1 and HP+100 off it: no bug.
        let (s, glitch, problems) = compiled(&[("undersht", "white", 1, 3), ("attack-1", "pink", 5, 2), ("hp-100", "white", 3, 2)]);
        assert_eq!(problems, Vec::<String>::new());
        assert!(s.undershirt && !glitch);
        assert_eq!((s.attack, s.max_hp, s.hp), (1, 700, 700));
        assert_eq!((s.bugs.buster_blanks, s.bugs.hp_drain), (0, 0));
        // Attack+1 on the command line: the buster bug (and it still works).
        let (s, glitch, _) = compiled(&[("undersht", "white", 1, 3), ("attack-1", "pink", 5, 4), ("hp-100", "white", 3, 2)]);
        assert!(glitch);
        assert_eq!((s.attack, s.bugs.buster_blanks, s.bugs.buster_charged), (1, 6, 1));
        // Attack+1 beside HP+100 of its color: each brings the other's bug.
        let (s, glitch, _) = compiled(&[("undersht", "white", 1, 3), ("attack-1", "pink", 5, 2), ("hp-100", "pink", 3, 2)]);
        assert!(glitch);
        assert_eq!((s.bugs.buster_blanks, s.bugs.hp_drain, s.bugs.hit_status), (6, 1, 3));
        // UnderSht off the command line: the step bug, and no UnderSht.
        let (s, glitch, _) = compiled(&[("undersht", "white", 1, 2)]);
        assert!(glitch && !s.undershirt);
        assert_eq!(s.bugs.processing, 1);
        // Over another, and off the board: said.
        let (_, _, problems) = compiled(&[("undersht", "white", 1, 3), ("attack-1", "pink", 1, 3)]);
        assert!(problems.iter().any(|p| p.contains("is over program 1")), "{problems:?}");
        let (_, _, problems) = compiled(&[("undersht", "white", 0, 1)]);
        assert!(problems.iter().any(|p| p.contains("off the board")), "{problems:?}");
    }

    /// EXE5's board grows with its ExpMemry (4x4, 5x4, 5x5, no frame): a
    /// program in the fifth column fits from one expansion, one in the fifth
    /// row from two; off the board it is said, as the original's placing
    /// refuses it (0x0813F250). A new side's board is the largest, and the
    /// compile is the same on every board the programs fit.
    #[test]
    fn exe5s_board_grows_with_its_expansions() {
        let content = crate::testing::exe5_content();
        assert_eq!(crate::navicust_rules(&content).boards.len(), 3);
        let m = crate::draw::live(&content, "exe5", 3, None).unwrap();
        assert_eq!(crate::Match::empty(&content, "exe5").unwrap().sides[0].navicust.map(|n| n.expansions), Some(2));
        let undersht = ids::navicust_program(&content, "exe5", "undersht").unwrap();
        // (UnderSht covers its center and the cell above it.)
        let with = |x: u8, y: u8, expansions: u8| {
            let mut m = m.clone();
            let part = PlacedProgram { program: undersht, color: 0, x, y, rotation: 0, compressed: false };
            m.sides[0].navicust = Some(NaviCust::new(&[part], expansions).unwrap());
            m
        };
        let said = |x: u8, y: u8, expansions: u8| -> Vec<String> {
            check_match(&content, &with(x, y, expansions)).into_iter().filter(|p| p.contains("navicust") || p.contains("NaviCust")).collect()
        };
        let works = |x: u8, y: u8, expansions: u8| Battle::new(with(x, y, expansions).round(&content, 3), content.clone()).stats[0].undershirt;
        // On the command line (the engine's row 3) inside the 4x4: every board.
        for expansions in 0..3 {
            assert_eq!((said(1, 3, expansions), works(1, 3, expansions)), (Vec::new(), true), "{expansions} expansions");
        }
        // The fifth column (the engine's 5): from one expansion.
        assert!(said(5, 3, 0).iter().any(|p| p.contains("off the board")));
        for expansions in 1..3 {
            assert_eq!((said(5, 3, expansions), works(5, 3, expansions)), (Vec::new(), true), "{expansions} expansions");
        }
        // The fifth row (the engine's 5): from two (off the command line there).
        for expansions in 0..2 {
            assert!(said(1, 5, expansions).iter().any(|p| p.contains("off the board")), "{expansions} expansions");
        }
        assert_eq!((said(1, 5, 2), works(1, 5, 2)), (Vec::new(), false));
        // No frame: off the 5x5 is off every board; and a fourth size is none.
        assert!(said(6, 3, 2).iter().any(|p| p.contains("off the board")));
        assert!(said(1, 3, 3).iter().any(|p| p.contains("3 expansions")));
    }

    /// A side with no NaviCust has its stats as a compile left them: no
    /// match key or setup field gives the emotion window's glitch, and the
    /// rules make it from the stats' NaviCust bugs (here the support bug,
    /// which the window's own count of bugs doesn't see).
    #[test]
    fn stats_set_directly_glitch_as_their_bugs_say() {
        let content = crate::testing::exe6_content();
        let mut m = crate::draw::live(&content, "exe6", 3, None).unwrap();
        for s in &mut m.sides {
            s.navicust = None;
        }
        m.sides[0].stats.support = None;
        assert_eq!(check_match(&content, &m), Vec::<String>::new());
        let b = Battle::new(m.round(&content, 3), content.clone());
        assert!(b.consoles[0].emotion_window_glitch, "the support bug");
        assert!(!b.consoles[1].emotion_window_glitch, "no bug");
    }

    /// MegaMan from a navi code gets his level's gains over what his
    /// NaviCust made (`reloadCurNaviStatBoosts`: `sub_8121154` after
    /// `sub_813C458`): its HP to the maximum and the HP the maximum, the
    /// buster's levels to 4, the custom level to 8, the Mega level to 10.
    #[test]
    fn megaman_from_a_navi_code_gets_his_levels_gains() {
        let content = crate::testing::exe6_content();
        let megaman = content.form_changing_navi().unwrap();
        let parts = [("undersht", "white", 1, 3), ("attack-1", "pink", 5, 2), ("hp-100", "white", 3, 2)];
        let (base, _, problems) = compiled(&parts);
        assert_eq!(problems, Vec::<String>::new());
        for level in [0u8, 7, 14] {
            let (s, _, problems) = compiled_at(&parts, Some(level));
            assert_eq!(problems, Vec::<String>::new(), "level {level}");
            let g = content.navi(megaman).levels.as_ref().unwrap().by_level[level as usize];
            let up = |v: u8, n: u8, most: u8| (v + n).min(most);
            assert_eq!(
                (s.max_hp, s.hp, s.attack, s.rapid, s.charge, s.custom_level, s.mega_level),
                (
                    base.max_hp + g.hp,
                    base.max_hp + g.hp,
                    up(base.attack, g.attack, 4),
                    up(base.rapid, g.rapid, 4),
                    up(base.charge, g.charge, 4),
                    up(base.custom_level, g.custom_level, 8),
                    up(base.mega_level, g.mega_level, 10)
                ),
                "level {level}"
            );
        }
    }
}
