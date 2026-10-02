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
//!   adds no card past 80 MB, `0x08141868`); the folder by BN6's rules
//!   (`crate::folders`), its Mega, Giga and Regular limits the navi's
//!   stats once the round has set them up (the NaviCust's and the patch
//!   cards' work: the original's folder editor and its link battle check
//!   read the stats the reload made).

use crate::folders::{self, FolderLimits};
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
        out.push(format!("{at}: no background {b:?}"));
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
    // The folder's chips, before its rules.
    if let Some((i, _)) = s.folder.chips.iter().enumerate().find(|(_, c)| c.id.index() >= defs.chips.len()) {
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
    // The folder's own rules (the limits wait for the round's stats).
    let unlimited = FolderLimits { mega: u8::MAX, giga: u8::MAX, regular_mb: u8::MAX };
    for v in folders::violations(content, &s.folder, unlimited) {
        out.push(format!("folder: {v}"));
    }
    out
}

/// What the stats a side's round starts with allow its folder.
fn check_limits(content: &Content, s: &Side, stats: &NaviStats) -> Vec<String> {
    let limits = FolderLimits::of(stats);
    let unlimited = FolderLimits { mega: u8::MAX, giga: u8::MAX, regular_mb: u8::MAX };
    let own: Vec<String> = folders::violations(content, &s.folder, unlimited);
    folders::violations(content, &s.folder, limits).into_iter().filter(|v| !own.contains(v)).map(|v| format!("folder: {v}")).collect()
}

/// The stats each side's round starts with: the setup's, after the rules
/// have set the round up (each side's NaviCust and patch cards), or why the
/// round doesn't start.
pub fn round_stats(content: &Arc<Content>, m: &Match) -> Result<[NaviStats; 2], String> {
    let setup = m.round(content, m.seed.unwrap_or(0));
    let started = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Battle::new(setup, content.clone())));
    match started {
        Ok(b) => Ok(b.stats),
        Err(e) => {
            let why = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
            Err(format!("the round doesn't start: {}", why.unwrap_or_else(|| "the engine stopped".into())))
        }
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
    match round_stats(content, &m) {
        Ok(stats) => out.extend(check_limits(content, s, &stats[0])),
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
    match round_stats(content, m) {
        Ok(stats) => {
            for ((s, at), st) in m.sides.iter().zip(sides).zip(&stats) {
                out.extend(check_limits(content, s, st).into_iter().map(|p| format!("{at}: {p}")));
            }
        }
        Err(e) => out.push(e),
    }
    out
}
