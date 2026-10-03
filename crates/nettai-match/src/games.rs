//! Matches across games (docs/frontend.md §6), on every game's content
//! (`testing::every_game`): BN5's folder rules on a BN5 side (its stock
//! ruleset's folder system, content/bn5/rules/folder), a BN6 side's folder
//! holding BN5 chips under BN6's rules (a mixed folder), and a BN5 side
//! against a BN6 side, each played a few hundred ticks.

use crate::testing::every_game;
use crate::{Match, check_match};
use nettai_battle::content::Content;
use std::sync::Arc;

/// A match file's text: the content's first link battle stage, and the
/// sides `left` and `right` (each a side's table body).
fn match_text(content: &Content, left: &str, right: &str) -> String {
    let stage = &content.defs.stage(crate::link_battle_stages(content)[0]).key;
    format!("[arena]\nstage = \"{stage}\"\n\n[left]\n{left}\n\n[right]\n{right}\n")
}

/// A side's table body: its ruleset and navi, the folder's chips (each
/// "key code"), and `more` lines in the folder's table (`regular = 3`).
fn side(ruleset: &str, navi: &str, chips: &[&str], more: &str) -> String {
    let chips: Vec<String> = chips.iter().map(|c| format!("\"{c}\"")).collect();
    format!("ruleset = \"{ruleset}\"\nnavi = \"{navi}\"\n\n[{{side}}.folder]\nchips = [{}]\n{more}", chips.join(", "))
}

/// The match these sides make, or every problem its checks find.
fn parse(content: &Arc<Content>, left: &str, right: &str) -> Result<Match, Vec<String>> {
    let text = match_text(content, &left.replace("{side}", "left"), &right.replace("{side}", "right"));
    crate::parse(content, &text)
}

/// A folder of Tango's BN5 saves (their first): Standard chips only, four
/// copies at most.
const TANGO_BN5: [&str; 30] = [
    "bn5:cannon A", "bn5:cannon A", "bn5:cannon B", "bn5:cannon B", "bn5:airshot *", "bn5:airshot *", "bn5:airshot *",
    "bn5:vulcan1 D", "bn5:vulcan1 D", "bn5:vulcan1 D", "bn5:minibomb B", "bn5:minibomb B", "bn5:minibomb L", "bn5:minibomb L",
    "bn5:sword S", "bn5:sword S", "bn5:sword S", "bn5:sword S", "bn5:wideswrd S", "bn5:wideswrd S", "bn5:busterup *",
    "bn5:crakout *", "bn5:crakout *", "bn5:recov10 A", "bn5:recov10 A", "bn5:recov10 L", "bn5:recov10 L", "bn5:areagrab S",
    "bn5:attck-10 *", "bn5:attck-10 *",
];

/// A BN6 folder: the live navi's kind of folder, Standard chips.
const BN6: [&str; 30] = [
    "bn6:cannon A", "bn6:cannon A", "bn6:cannon B", "bn6:airshot *", "bn6:airshot *", "bn6:vulcan1 D", "bn6:vulcan1 D",
    "bn6:minibomb B", "bn6:minibomb B", "bn6:minibomb L", "bn6:sword S", "bn6:sword S", "bn6:sword S", "bn6:wideswrd S",
    "bn6:wideswrd S", "bn6:recov10 A", "bn6:recov10 A", "bn6:recov10 L", "bn6:areagrab S", "bn6:areagrab S", "bn6:spreadr1 L",
    "bn6:spreadr1 L", "bn6:longswrd S", "bn6:longswrd S", "bn6:crakshot A", "bn6:crakshot A", "bn6:widesht P", "bn6:widesht P",
    "bn6:cannon C", "bn6:recov30 L",
];

/// A BN5 side by BN5's stock rules, with this folder.
fn bn5(chips: &[&str], more: &str) -> String {
    side("bn5:stock", "bn5:megaman", chips, more)
}

/// `chips` with entries `at` replaced by `with`.
fn with(chips: &[&str], at: usize, with: &[&'static str]) -> Vec<String> {
    let mut v: Vec<String> = chips.iter().map(|s| s.to_string()).collect();
    for (i, c) in with.iter().enumerate() {
        v[at + i] = c.to_string();
    }
    v
}

fn refs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

/// BN5's MegaMan, on BN5's stock rules, with Tango's BN5 folder: the
/// rules accept it (the Regular chip too), against a BN6 side.
#[test]
fn a_bn5_folder_keeps_bn5s_rules() {
    let content = every_game();
    let right = side("bn6:stock", "bn6:megaman", &BN6, "");
    parse(&content, &bn5(&TANGO_BN5, ""), &right).unwrap_or_else(|p| panic!("{p:?}"));
    // A Regular chip within the fresh navi's Regular memory (4 MB: CrakOut).
    parse(&content, &bn5(&TANGO_BN5, "regular = 21"), &right).unwrap_or_else(|p| panic!("{p:?}"));
    // Three Mega chips, three dark chips (one of each), one Giga chip.
    let mixed = with(
        &TANGO_BN5,
        0,
        &["bn5:blizman-ds B", "bn5:cloudmn-ds C", "bn5:colonel C", "bn5:drksword Z", "bn5:darkthnd M", "bn5:darkwide T", "bn5:bass F"],
    );
    parse(&content, &bn5(&refs(&mixed), ""), &right).unwrap_or_else(|p| panic!("{p:?}"));
}

/// What BN5's folder rules refuse, each said with its rule.
#[test]
fn bn5s_rules_refuse() {
    let content = every_game();
    let right = side("bn6:stock", "bn6:megaman", &BN6, "");
    let refused = |chips: &[String], more: &str| -> Vec<String> {
        let e = parse(&content, &bn5(&refs(chips), more), &right).expect_err("refused");
        assert!(e.iter().all(|p| p.starts_with("left: folder: ")), "{e:?}");
        e
    };
    let says = |e: &[String], what: &str| assert!(e.iter().any(|p| p.contains(what)), "{what:?} not in {e:?}");
    // A fifth copy of a Standard chip (BN6 would take five 8 MB Cannons).
    let five = with(&TANGO_BN5, 4, &["bn5:cannon C"]);
    says(&refused(&five, ""), "5 copies of bn5:cannon (a standard chip), past 4");
    // Two copies of a Mega chip, of a Giga chip, of a dark chip.
    let two = with(&TANGO_BN5, 0, &["bn5:colonel C", "bn5:colonel C", "bn5:bass F", "bn5:bass F", "bn5:drksword Z", "bn5:drksword Z"]);
    let e = refused(&two, "");
    says(&e, "2 copies of bn5:colonel (a mega chip), past 1");
    says(&e, "2 copies of bn5:bass (a giga chip), past 1");
    says(&e, "2 copies of bn5:drksword (a dark chip), past 1");
    says(&e, "2 Giga chips, past the navi's 1");
    // Six Mega chips; four dark chips.
    let six = with(
        &TANGO_BN5,
        0,
        &["bn5:colonel C", "bn5:meddy M", "bn5:gyroman G", "bn5:knightmn K", "bn5:larkman S", "bn5:gridman F"],
    );
    says(&refused(&six, ""), "6 Mega chips, past the navi's 5");
    let four = with(&TANGO_BN5, 0, &["bn5:drksword Z", "bn5:darkthnd M", "bn5:darkwide T", "bn5:drklance W"]);
    says(&refused(&four, ""), "4 dark chips, past 3");
    // A chip past the pack (BatCan1), a Program Advance; a code the chip
    // doesn't come in.
    let pack = with(&TANGO_BN5, 0, &["bn5:batcan1 *", "bn5:lifesrd *"]);
    let e = refused(&pack, "");
    says(&e, "entry 0: bn5:batcan1 is no chip a folder can hold");
    says(&e, "entry 1: bn5:lifesrd is no chip a folder can hold");
    let code = with(&TANGO_BN5, 0, &["bn5:cannon Z"]);
    says(&refused(&code, ""), "entry 0: bn5:cannon doesn't come in code Z");
    // A Regular chip past the Regular memory (Cannon is 8 MB; a fresh
    // navi's memory is 4); tag chips, which BN5's folders haven't.
    says(&refused(&with(&TANGO_BN5, 0, &[]), "regular = 0"), "the Regular chip bn5:cannon is 8 MB, past the navi's 4");
    says(&refused(&with(&TANGO_BN5, 0, &[]), "tags = [4, 5]"), "tag chips, but BN5's folders have none");
}

/// A BN6 side's folder of BN5 chips (a mixed folder) is held to BN6's
/// rules, by the chips' records: five 8 MB Cannons of BN5's are BN6's
/// five copies, which BN5's own rules refuse.
#[test]
fn a_mixed_folder_keeps_its_sides_rules() {
    let content = every_game();
    let five = with(&TANGO_BN5, 4, &["bn5:cannon C"]);
    let mixed: Vec<String> = refs(&five).iter().take(15).map(|s| s.to_string()).chain(BN6[15..].iter().map(|s| s.to_string())).collect();
    let left = side("bn6:stock", "bn6:megaman", &refs(&mixed), "");
    let right = bn5(&TANGO_BN5, "");
    parse(&content, &left, &right).unwrap_or_else(|p| panic!("{p:?}"));
    // And BN6's limits: a sixth copy is past BN6's five.
    let six = with(&refs(&mixed), 5, &["bn5:cannon *"]);
    let e = parse(&content, &side("bn6:stock", "bn6:megaman", &refs(&six), ""), &right).expect_err("refused");
    assert!(e.iter().any(|p| p.contains("left: folder: 6 copies of bn5:cannon")), "{e:?}");
}

/// The chips a BN6 side's rules let a folder hold are every game's (a
/// BN5 Cannon among them), and a BN5 side's BN5's own without its BatCans
/// and Program Advances.
#[test]
fn every_games_chips_are_in_the_pool() {
    let content = every_game();
    let m = parse(&content, &side("bn6:stock", "bn6:megaman", &BN6, ""), &bn5(&TANGO_BN5, "")).unwrap();
    let mut b = crate::check::start(&content, &m).unwrap();
    let key = |h: nettai_content_api::ChipHandle| content.defs.chip(h).key.clone();
    let six: Vec<String> = crate::folders::pool(&content, &mut b, 0).into_iter().map(key).collect();
    let five: Vec<String> = crate::folders::pool(&content, &mut b, 1).into_iter().map(key).collect();
    for k in ["bn5:cannon", "bn6:cannon", "bn5:colonel", "bn5:bass"] {
        assert!(six.contains(&k.to_string()) && five.contains(&k.to_string()), "{k}");
    }
    for k in ["bn5:batcan1", "bn5:lifesrd", "bn5:invalid"] {
        assert!(!five.contains(&k.to_string()), "{k}");
    }
}

/// Play a match `ticks` ticks: the scenario's players (picking chips at
/// each custom screen, stepping about, firing and using them) on the
/// match's round. The chips each side used, by key, in order.
fn play(content: &Arc<Content>, m: &Match, ticks: usize) -> [Vec<String>; 2] {
    assert_eq!(check_match(content, m), Vec::<String>::new());
    let setup = m.round(content, 0x5EED);
    let tape = nettai_battle::scenario::record_on_content(setup.clone(), content.clone(), ticks, 11);
    let mut b = nettai_battle::Battle::new(setup, content.clone());
    let mut used: [Vec<String>; 2] = Default::default();
    let mut shown = [None, None];
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for side in 0..2 {
            let now = b.used_chips[side].map(|u| u.chip);
            if now.is_some() && b.used_chips[side].is_some_and(|u| u.ticks > 0) && now != shown[side] {
                used[side].push(content.defs.chip(now.unwrap()).key.clone());
            }
            shown[side] = now;
        }
    }
    used
}

/// A BN6 side with BN5 chips, against BN6's own, and a BN5 side against
/// a BN6 side: each plays 900 ticks (custom screens, chips used, a BN5
/// chip among them) without the engine stopping.
#[test]
fn mixed_and_cross_game_matches_play() {
    let content = every_game();
    let mixed: Vec<String> = TANGO_BN5.iter().take(15).map(|s| s.to_string()).chain(BN6[15..].iter().map(|s| s.to_string())).collect();
    let m = parse(&content, &side("bn6:stock", "bn6:megaman", &refs(&mixed), ""), &side("bn6:stock", "bn6:megaman", &BN6, "")).unwrap();
    let used = play(&content, &m, 900);
    assert!(used[0].iter().any(|k| k.starts_with("bn5:")), "the mixed side used a BN5 chip: {used:?}");
    let m = parse(&content, &bn5(&TANGO_BN5, ""), &side("bn6:stock", "bn6:megaman", &BN6, "")).unwrap();
    let used = play(&content, &m, 900);
    assert!(used[0].iter().any(|k| k.starts_with("bn5:")) && used[1].iter().any(|k| k.starts_with("bn6:")), "{used:?}");
}
