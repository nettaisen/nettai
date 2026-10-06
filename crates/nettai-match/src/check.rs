//! What a match must be to be played: the checks a match file passes when
//! it loads, a netplay offer when it arrives (`check_side`), and the editor
//! shows as they fail. Each problem is said, with where it is.
//!
//! - **The arena**: the content's game, stages a match of it may name (its
//!   rules' `link_pick.match_stages`, `crate::link_battle_stages`), and
//!   backgrounds its pack has.
//! - **A side**: its navi stated and the match's game's; its facts its game's
//!   rules' (`crate::facts::check`: an enum the rules require stated, each
//!   definition they name, at any depth, the game's, a definition once in its
//!   list, the engine's form list forms of the navi's own lists).
//! - **What its game's rules say of it** (their `validate`, `Battle::validate`),
//!   once the round is set up (the stats its rules built: the NaviCust's and
//!   the patch cards' folder limits, which the original's folder editor and
//!   its link battle check read): EXE6's and EXE5's the navi's level, the
//!   base HP, the patch cards, the NaviCust on its board, the folder by its
//!   game's folder rules (content/exe6/rules, content/exe5/rules). This crate
//!   knows none of those: it reports what the rules say.

use crate::{Arena, Folder, Match, Place, Side, ids};
use nettai_battle::Battle;
use nettai_battle::content::Content;
use nettai_battle::setup::NaviStats;
use std::sync::Arc;

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
/// `check_arena`) that needs no battle to see: its navi, its auto battle
/// data, its facts as its game's rules declare them.
pub fn check_side_alone(content: &Content, arena: &Arena, s: &Side) -> Vec<String> {
    let mut out = Vec::new();
    let defs = &content.defs;
    let game = arena.game.as_str();
    // (A side's facts of another game's rules say nothing of this game's.)
    if !s.facts.fit(content) {
        return crate::facts::check(content, arena, s);
    }
    match s.stated_navi(content) {
        None => {
            out.push("no navi: a side states its own".into());
            return out;
        }
        Some(n) if n.index() >= defs.navis.len() || !ids::in_game(content, game, &defs.navi(n).key) => {
            out.push(format!("a navi {game} hasn't"));
            return out;
        }
        Some(_) => {}
    }
    // The facts its game's rules take.
    out.extend(crate::facts::check(content, arena, s));
    out
}

/// The round `m` starts, set up (each side's rules have set its stats:
/// the NaviCust, the patch cards), or why it doesn't start.
pub fn start(content: &Arc<Content>, m: &Match) -> Result<Battle, String> {
    // A folder being made has empty entries, and a round is set up with
    // whole ones: here they hold a stand-in (the folder's first chip, else
    // the content's first with a code). Its rules see its own entries
    // (`validate`), and say it isn't whole.
    let mut m = m.clone();
    for s in &mut m.sides {
        let folder = s.folder(content);
        if folder.saved().is_none() {
            let any = (0..content.defs.chips.len() as u16)
                .map(nettai_content_api::ChipHandle)
                .find_map(|id| content.chip(id).codes.first().map(|&code| nettai_battle::custom::FolderChip::new(id, code)));
            let filler = folder.chips().next().or(any).ok_or("the content has no chip with a code")?;
            s.set_folder(content, &Folder::from(folder.filled_with(filler)))?;
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

/// What side `side`'s rules say is wrong with its setup in `b`, the
/// round's battle (their `validate`): of the side's own setup, its folder as
/// it stands (the round was set up with a whole one: `start`).
fn validated(b: &mut Battle, side: usize, s: &Side) -> Vec<String> {
    b.setup.players[side].rules = s.facts.block().cloned();
    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.validate(side as u8)));
    match checked {
        Ok(problems) => problems.into_iter().map(|p| p.text).collect(),
        Err(_) => vec!["the rules stopped checking the side".into()],
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
        Ok(mut b) => out.extend(validated(&mut b, 0, s)),
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
                out.extend(validated(&mut b, side, s).into_iter().map(|p| format!("{at}: {p}")));
            }
        }
        Err(e) => out.push(e),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{PlacedProgram, set_navicust};

    /// A MegaMan with a NaviCust of EXE6's programs: rules/navicust
    /// compiles it into the stats the round starts with.
    fn compiled(parts: &[(&str, &str, u8, u8)]) -> (NaviStats, bool, Vec<String>) {
        compiled_at(parts, None)
    }

    /// [`compiled`], MegaMan from a navi code of `level`.
    fn compiled_at(parts: &[(&str, &str, u8, u8)], level: Option<u8>) -> (NaviStats, bool, Vec<String>) {
        let content = crate::testing::exe6_content();
        let mut m = crate::pick::live(&content, "exe6", 3, None).unwrap();
        let s = &mut m.sides[0];
        s.set_level(&content, level).unwrap();
        // (What the save brings: a base HP of 600, a Regular memory of 50.)
        use nettai_battle::rules::Fact;
        use nettai_content_api::Value;
        s.set_fact(&content, "hp", &[Fact::Value(Value::Int(600))]).unwrap();
        s.set_fact(&content, "reg_up", &[Fact::Value(Value::Int(50))]).unwrap();
        let placed: Vec<PlacedProgram> = parts
            .iter()
            .map(|&(name, color, x, y)| {
                let program = ids::navicust_program(&content, "exe6", name).unwrap();
                let color = content.defs.navicust_program(program).colors.iter().position(|c| c == color).unwrap() as u8;
                PlacedProgram { program, color, x, y, rotation: 0, compressed: false }
            })
            .collect();
        set_navicust(&content, s, &placed, 2);
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
        let m = crate::pick::live(&content, "exe5", 3, None).unwrap();
        assert_eq!(crate::testing::navicust_expansions(&content, &crate::Match::empty(&content, "exe5").unwrap().sides[0]), Some(2));
        let undersht = ids::navicust_program(&content, "exe5", "undersht").unwrap();
        // (UnderSht covers its center and the cell above it.)
        let with = |x: u8, y: u8, expansions: u8| {
            let mut m = m.clone();
            let part = PlacedProgram { program: undersht, color: 0, x, y, rotation: 0, compressed: false };
            set_navicust(&content, &mut m.sides[0], &[part], expansions);
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

    /// EXE5's HubBatc shares a board with no HP program. It has one place
    /// (the middle of the 5x5 board, the whole command line its own; no
    /// smaller board holds it), which leaves four corners of three cells in
    /// an L, and no HP program fits one: turned or not, compressed or not
    /// (none has a compressed shape of its own). So its bug, which halves
    /// what the HP programs add (0x08140248), has nothing to halve on a
    /// board a save can hold. The lab's recordings of that halving
    /// (docs/design/exe5-map.md §15.13) lay HP+500 over HubBatc, which the
    /// original's compile takes and its placing refuses (0x0813F2A4), as
    /// this check does. BugStop fits beside it only compressed, in a corner,
    /// off the command line, where it stops nothing.
    #[test]
    fn exe5s_hubbatc_shares_a_board_with_no_hp_program() {
        use nettai_battle::navicust::{SIZE, cells};
        let content = crate::testing::exe5_content();
        let m = crate::pick::live(&content, "exe5", 3, None).unwrap();
        let program = |name: &str| ids::navicust_program(&content, "exe5", name).unwrap();
        let problems = |parts: &[PlacedProgram], expansions: u8| -> Vec<String> {
            let mut m = m.clone();
            set_navicust(&content, &mut m.sides[0], parts, expansions);
            check_match(&content, &m).into_iter().filter(|p| p.contains("navicust") || p.contains("NaviCust")).collect()
        };
        let at = |program, x: u8, y: u8, rotation: u8, compressed: bool| PlacedProgram { program, color: 0, x, y, rotation, compressed };
        let everywhere = || (0..SIZE as u8).flat_map(|x| (0..SIZE as u8).map(move |y| (x, y)));
        let hub = program("hubbatc");
        // HubBatc: the middle of the largest board alone.
        for expansions in 0..3 {
            for (x, y) in everywhere() {
                let fits = problems(&[at(hub, x, y, 0, false)], expansions).is_empty();
                assert_eq!(fits, expansions == 2 && (x, y) == (3, 3), "{expansions} expansions, ({x}, {y})");
            }
        }
        let beside_hub = |part: PlacedProgram| problems(&[at(hub, 3, 3, 0, false), part], 2);
        // No HP program beside it.
        for name in ["hp-50", "hp-100", "hp-200", "hp-300", "hp-400", "hp-500"] {
            let hp = program(name);
            for (x, y) in everywhere() {
                for rotation in 0..4 {
                    for compressed in [false, true] {
                        let said = beside_hub(at(hp, x, y, rotation, compressed));
                        assert!(
                            said.iter().any(|p| p.contains("off the board") || p.contains("is over program 1")),
                            "{name} at ({x}, {y}) turned {rotation}, compressed {compressed}: {said:?}"
                        );
                    }
                }
            }
        }
        // (The lab's board: HP+500 over HubBatc's top.)
        assert!(beside_hub(at(program("hp-500"), 3, 2, 0, false)).iter().any(|p| p.contains("is over program 1")));
        // BugStop beside it: compressed alone, and never on the command line.
        let bugstop = program("bugstop");
        let line = crate::navicust_rules(&content).command_line as i32;
        let mut places = 0;
        for (x, y) in everywhere() {
            for rotation in 0..4 {
                assert!(!beside_hub(at(bugstop, x, y, rotation, false)).is_empty(), "BugStop as it is at ({x}, {y}) turned {rotation}");
                if beside_hub(at(bugstop, x, y, rotation, true)).is_empty() {
                    let shape = content.defs.navicust_program(bugstop).placed_shape(true, rotation);
                    assert!(cells(&shape, x, y).all(|(_, row)| row != line), "BugStop on the command line at ({x}, {y}) turned {rotation}");
                    places += 1;
                }
            }
        }
        assert!(places > 0, "compressed, BugStop fits a corner");
    }

    /// A setup with no NaviCust has its stats as a compile left them (a
    /// recording's): no setup field gives the emotion window's glitch, and
    /// the rules make it from the stats' NaviCust bugs (here the support
    /// bug, which the window's own count of bugs doesn't see).
    #[test]
    fn stats_set_directly_glitch_as_their_bugs_say() {
        let content = crate::testing::exe6_content();
        let m = crate::pick::live(&content, "exe6", 3, None).unwrap();
        let mut setup = m.round(&content, 3);
        // (No NaviCust: the rules' `navicust_expansions` none.)
        for p in &mut setup.players {
            let none = nettai_battle::rules::Fact::Value(nettai_content_api::Value::Nil);
            p.set_fact(&content, "navicust_expansions", &[none]).unwrap();
        }
        setup.navi_stats[0].support = None;
        let b = Battle::new(setup, content.clone());
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
