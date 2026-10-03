//! What a match must be to be played: the checks a match file passes when
//! it loads, a netplay offer when it arrives (`check_side`), and the editor
//! shows as they fail. Each problem is said, with where it is.
//!
//! - **The arena**: link battle stages (`crate::link_battle_stages`), and
//!   backgrounds the pack has.
//! - **A side**: its ruleset, navi, stats' forms and patch cards are the
//!   content's; a Cross list only with a ruleset that has the forms system,
//!   each a Cross of the navi's, at most five, none twice; patch cards only
//!   with a ruleset that has the patch-cards system, each installed once, at
//!   most [`MAX_CARDS`], their MB together at most [`CARD_MB`] (BN6's menu
//!   adds no card past 80 MB, `0x08141868`); a NaviCust only for MegaMan
//!   under rules with the navicust system, on its board (`check_navicust`);
//!   the folder by its own game's rules (the side's ruleset's
//!   `folder_check`, `crate::folders`: BN6's folder editor's), on the stats
//!   the round set up (the NaviCust's and the patch cards' folder limits:
//!   the original's folder editor and its link battle check read the stats
//!   the reload made).

use crate::folders;
use crate::{Arena, Match, Place, Side};
use nettai_battle::Battle;
use nettai_battle::content::Content;
use nettai_battle::patch_cards::MAX_CARDS;
use nettai_battle::setup::NaviStats;
use std::sync::Arc;

/// The installed patch cards' MB together at most (BN6's).
pub const CARD_MB: u32 = 80;

fn check_place(content: &Content, p: &Place, at: &str, out: &mut Vec<String>) {
    if p.stage.index() >= content.defs.stages.len() {
        out.push(format!("{at}: a stage the content hasn't"));
        return;
    }
    if !crate::link_battle_stages(content).contains(&p.stage) {
        out.push(format!("{at}: {} is no link battle stage", content.defs.stage(p.stage).key));
    }
    if let Some(b) = &p.background
        && crate::background(content, b).is_none()
    {
        out.push(crate::no_background(at, b));
    }
}

/// What is wrong with an arena.
pub fn check_arena(content: &Content, a: &Arena) -> Vec<String> {
    let mut out = Vec::new();
    check_place(content, &a.first, "arena", &mut out);
    for (i, p) in a.later.iter().enumerate() {
        check_place(content, p, &format!("arena: later round {}", i + 2), &mut out);
    }
    out
}

/// What is wrong with a side that needs no battle to see.
pub fn check_side_alone(content: &Content, s: &Side) -> Vec<String> {
    let mut out = Vec::new();
    let defs = &content.defs;
    if s.ruleset.is_some_and(|r| r.index() >= defs.rulesets.len()) {
        out.push("a ruleset the content hasn't".into());
    }
    if s.navi.index() >= defs.navis.len() {
        out.push("a navi the content hasn't".into());
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
    match s.navi_level {
        Some(l) if l > nettai_battle::custom::MAX_NAVI_LEVEL => {
            out.push(format!("level {l}: a navi code's level is 0 to {}", nettai_battle::custom::MAX_NAVI_LEVEL))
        }
        None if !content.navi(s.navi).changes_form() => {
            out.push(format!("{} has no level: a link navi exists only through its navi code", crate::names::navi(content, s.navi)))
        }
        _ => {}
    }
    // The folder's chips, before its rules.
    if let Some((i, _)) = s.folder.chips.iter().enumerate().find(|(_, c)| c.is_some_and(|c| c.id.index() >= defs.chips.len())) {
        out.push(format!("folder entry {i}: a chip the content hasn't"));
        return out;
    }
    // The Crosses.
    if let Some(list) = &s.crosses {
        if !s.has_system(content, crate::FORMS_SYSTEM) {
            out.push("a Cross list, but the ruleset has no Crosses (no forms system)".into());
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
        if !s.has_system(content, crate::PATCH_CARDS_SYSTEM) {
            out.push("patch cards, but the ruleset has no patch-cards system".into());
        }
        if s.cards.len() > MAX_CARDS {
            out.push(format!("{} patch cards installed: a list holds {MAX_CARDS}", s.cards.len()));
        }
        if s.cards.iter().any(|c| c.card.index() >= defs.patch_cards.len()) {
            out.push("a patch card the content hasn't".into());
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
        out.extend(check_navicust(content, s, n));
    }
    // The folder's chips are the content's (its rules wait for the round).
    let in_folder = |i: u8| s.folder.has(i);
    if !s.folder.regular.is_none_or(in_folder) || !s.folder.tags.is_none_or(|(a, b)| in_folder(a) && in_folder(b)) {
        out.push("folder: the Regular or tag chips aren't chips of the folder".into());
    }
    out
}

/// What is wrong with a side's NaviCust: it is MegaMan's (the navi that
/// changes form: BN6 compiles the PET's own navi's alone) under rules with
/// the navicust system; the stats set besides are only what the save keeps
/// through its compile; each program in one of its colors, on the board of
/// its expansions (BN6's `sub_813BB00`: every cell it covers on the board or
/// its frame, not all on the frame), over no other (`sub_813BB68`); copies
/// of a program in one color all compressed or not (the save keeps it by
/// program and color: event flag 0x2660 + the part id).
pub fn check_navicust(content: &Content, s: &Side, n: &nettai_battle::navicust::NaviCust) -> Vec<String> {
    use nettai_battle::content::NaviCustRules;
    use nettai_battle::navicust::{SIZE, cells};
    let mut out = Vec::new();
    let defs = &content.defs;
    if !s.has_system(content, crate::NAVICUST_SYSTEM) {
        out.push("a NaviCust, but the ruleset has no navicust system".into());
    }
    if content.navi(s.navi).forms.is_none() {
        out.push(format!("a NaviCust, but {}'s stats aren't a NaviCust's (only MegaMan's compiles)", crate::names::navi(content, s.navi)));
    }
    for name in s.stats_block(content).keys() {
        if !crate::stats::SAVE_FIELDS.contains(&name.as_str()) {
            out.push(format!("stats: {name} is the NaviCust's (with a NaviCust the stats set only {})", crate::stats::SAVE_FIELDS.join(", ")));
        }
    }
    let rules = crate::navicust_rules(content, s);
    let Some(board) = rules.board(n.expansions) else {
        out.push(format!("a NaviCust with {} expansions: the game's board has {} sizes", n.expansions, rules.boards.len()));
        return out;
    };
    let mut grid = [[None; SIZE]; SIZE];
    let mut compression: Vec<((u16, u8), bool)> = Vec::new();
    for (i, p) in n.iter().enumerate() {
        if p.program.index() >= defs.navicust_programs.len() {
            out.push(format!("navicust program {}: a program the content hasn't", i + 1));
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

/// What is wrong with one side, as a netplay offer brings it: everything
/// [`check_side_alone`] sees, and its folder against the stats its round
/// starts with (the side on both sides of a round on the first link
/// battle stage).
pub fn check_side(content: &Arc<Content>, s: &Side) -> Vec<String> {
    let mut out = check_side_alone(content, s);
    if !out.is_empty() {
        return out;
    }
    let Some(&stage) = crate::link_battle_stages(content).first() else {
        return vec!["the content has no link battle stage".into()];
    };
    let m = Match { seed: None, arena: Arena::on(Place { stage, background: None }), sides: [s.clone(), s.clone()] };
    match start(content, &m) {
        Ok(mut b) => out.extend(folder_problems(&mut b, 0, s)),
        Err(e) => out.push(e),
    }
    out
}

/// What is wrong with a match, each problem with where it is.
pub fn check_match(content: &Arc<Content>, m: &Match) -> Vec<String> {
    let mut out = check_arena(content, &m.arena);
    let sides = ["left", "right"];
    for (s, at) in m.sides.iter().zip(sides) {
        out.extend(check_side_alone(content, s).into_iter().map(|p| format!("{at}: {p}")));
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

    /// A MegaMan with a NaviCust of BN6's programs: the navicust system
    /// compiles it into the stats the round starts with.
    fn compiled(parts: &[(&str, &str, u8, u8)]) -> (NaviStats, bool, Vec<String>) {
        compiled_at(parts, None)
    }

    /// [`compiled`], MegaMan from a navi code of `level`.
    fn compiled_at(parts: &[(&str, &str, u8, u8)], level: Option<u8>) -> (NaviStats, bool, Vec<String>) {
        let content = crate::testing::bn6_content();
        let mut m = crate::draw::live(&content, 3, None).unwrap();
        let s = &mut m.sides[0];
        s.navi_level = level;
        s.stats = crate::Side::base_stats(&content, s.navi, s.game);
        (s.stats.max_base_hp, s.stats.hp, s.stats.max_hp, s.stats.reg_up) = (600, 600, 600, 50);
        let placed: Vec<PlacedProgram> = parts
            .iter()
            .map(|&(key, color, x, y)| {
                let program = content.defs.navicust_program_by_key(key).unwrap();
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
        let (s, glitch, problems) = compiled(&[("bn6:undersht", "white", 1, 3), ("bn6:attack-1", "pink", 5, 2), ("bn6:hp-100", "white", 3, 2)]);
        assert_eq!(problems, Vec::<String>::new());
        assert!(s.undershirt && !glitch);
        assert_eq!((s.attack, s.max_hp, s.hp), (1, 700, 700));
        assert_eq!((s.bugs.buster_blanks, s.bugs.hp_drain), (0, 0));
        // Attack+1 on the command line: the buster bug (and it still works).
        let (s, glitch, _) = compiled(&[("bn6:undersht", "white", 1, 3), ("bn6:attack-1", "pink", 5, 4), ("bn6:hp-100", "white", 3, 2)]);
        assert!(glitch);
        assert_eq!((s.attack, s.bugs.buster_blanks, s.bugs.buster_charged), (1, 6, 1));
        // Attack+1 beside HP+100 of its color: each brings the other's bug.
        let (s, glitch, _) = compiled(&[("bn6:undersht", "white", 1, 3), ("bn6:attack-1", "pink", 5, 2), ("bn6:hp-100", "pink", 3, 2)]);
        assert!(glitch);
        assert_eq!((s.bugs.buster_blanks, s.bugs.hp_drain, s.bugs.hit_status), (6, 1, 3));
        // UnderSht off the command line: the step bug, and no UnderSht.
        let (s, glitch, _) = compiled(&[("bn6:undersht", "white", 1, 2)]);
        assert!(glitch && !s.undershirt);
        assert_eq!(s.bugs.processing, 1);
        // Over another, and off the board: said.
        let (_, _, problems) = compiled(&[("bn6:undersht", "white", 1, 3), ("bn6:attack-1", "pink", 1, 3)]);
        assert!(problems.iter().any(|p| p.contains("is over program 1")), "{problems:?}");
        let (_, _, problems) = compiled(&[("bn6:undersht", "white", 0, 1)]);
        assert!(problems.iter().any(|p| p.contains("off the board")), "{problems:?}");
    }

    /// MegaMan from a navi code gets his level's gains over what his
    /// NaviCust made (`reloadCurNaviStatBoosts`: `sub_8121154` after
    /// `sub_813C458`): its HP to the maximum and the HP the maximum, the
    /// buster's levels to 4, the custom level to 8, the Mega level to 10.
    #[test]
    fn megaman_from_a_navi_code_gets_his_levels_gains() {
        let content = crate::testing::bn6_content();
        let megaman = content.form_changing_navi().unwrap();
        let parts = [("bn6:undersht", "white", 1, 3), ("bn6:attack-1", "pink", 5, 2), ("bn6:hp-100", "white", 3, 2)];
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
