//! What a side's save brings that its ruleset's systems take by a field's
//! name (S6c's facts, `PlayerSetup::set_fact`), besides BN6's (the game,
//! the Crosses, Beast Out: bn6-compat's `Unlocks`): BN5's karma and souls.
//! A match file and a netplay offer carry them as the side's own keys
//! (`karma`, `souls`); the round's setup writes each into whichever of the
//! side's systems declares the field (BN5's light and dark system's
//! `karma`, its souls system's `souls`), and a ruleset with none takes
//! none.
//!
//! **Karma** is BN5's light/dark value (NaviStats +0x44), 0 to 1000: a
//! fresh save's 500 (0x08010C00) is the default. Under 470 a dark MegaMan
//! (mood 0, the dark face and palette, dark chips, no soul button); 499 or
//! under clears holy panels; under 500 worried at the start; 1000 the
//! brightest (Tango's light templates).
//!
//! **Souls** (BN5's Soul Unison) are the souls the side has, which the
//! custom screen's soul button may offer. The original's button
//! (0x08024B28) offers the soul of the last chip's family (the table
//! 0x08024BE0) when the save has it: each version's table 0x08024BF0 gives
//! Team ProtoMan's souls 1 to 6 the event flags 2 to 7 and Team Colonel's 7
//! to 12 the flags 8 to 0x0D, the other version's none (never offered); a
//! dark chip's Chaos Unison needs flag 0x236 too; a soul given this round
//! isn't offered again. The engine ports that check on the souls owned (the
//! souls system's setup, as the save's flags). A side may have any soul of
//! the content, of either version or any game (the user: "allow all souls
//! to be selected regardless of game"): none listed, every soul. A real
//! save holds its own version's six; the save import reads them.

use crate::Side;
use nettai_battle::content::{Content, FormKind};
use nettai_battle::custom::PlayerSetup;
use nettai_battle::rules::Fact;
use nettai_content_api::{FormHandle, Registry, Value};

/// The setup fields the facts go into.
pub const KARMA_FIELD: &str = "karma";
pub const SOULS_FIELD: &str = "souls";

/// A fresh save's karma (0x08010C00), and the most there is.
pub const DEFAULT_KARMA: u16 = 500;
pub const MAX_KARMA: u16 = 1000;

/// Whether a system of `side`'s ruleset declares setup field `field` (the
/// side's rules take that fact).
pub fn takes(content: &Content, side: &Side, field: &str) -> bool {
    let Some(r) = side.ruleset_or_stock(content) else { return false };
    content.defs.ruleset(r).systems.iter().any(|&h| content.defs.schema(content.defs.system(h).setup).index_of(field).is_some())
}

/// How many souls the side's rules take (the `souls` field's elements:
/// BN5's souls system's 16), none when they take none.
pub fn soul_capacity(content: &Content, side: &Side) -> usize {
    let Some(r) = side.ruleset_or_stock(content) else { return 0 };
    content
        .defs
        .ruleset(r)
        .systems
        .iter()
        .filter_map(|&h| {
            let schema = content.defs.schema(content.defs.system(h).setup);
            match &schema.field(schema.index_of(SOULS_FIELD)?).ty {
                nettai_content_api::FieldType::Array(_, n) => Some(*n as usize),
                _ => None,
            }
        })
        .min()
        .unwrap_or(0)
}

/// Every soul the content has (forms of kind soul), in handle order: of
/// every game, either version.
pub fn all_souls(content: &Content) -> Vec<FormHandle> {
    (0..content.defs.forms.len() as u16).map(FormHandle).filter(|&f| content.form(f).kind == FormKind::Soul).collect()
}

/// The souls `side` has: its list, else every soul.
pub fn owned_souls(content: &Content, side: &Side) -> Vec<FormHandle> {
    side.souls.clone().unwrap_or_else(|| all_souls(content))
}

/// What is wrong with a side's karma and souls: karma past 1000, or other
/// than the default under rules that take none; a soul list under rules
/// without souls, a form that is no soul, a soul twice. (Either version's
/// souls are fine.)
pub fn check(content: &Content, side: &Side) -> Vec<String> {
    let mut out = Vec::new();
    if side.karma > MAX_KARMA {
        out.push(format!("karma {}: the light/dark value is 0 to {MAX_KARMA}", side.karma));
    }
    if side.karma != DEFAULT_KARMA && !takes(content, side, KARMA_FIELD) {
        out.push("karma, but the ruleset has no light and dark MegaMan (no system takes `karma`)".into());
    }
    let Some(list) = &side.souls else { return out };
    if !takes(content, side, SOULS_FIELD) {
        out.push("a soul list, but the ruleset has no Soul Unison (no system takes `souls`)".into());
    } else if list.len() > soul_capacity(content, side) {
        out.push(format!("{} souls; the rules hold {}", list.len(), soul_capacity(content, side)));
    }
    for (i, &f) in list.iter().enumerate() {
        if f.index() >= content.defs.forms.len() {
            out.push("a soul the content hasn't".into());
            continue;
        }
        if content.form(f).kind != FormKind::Soul {
            out.push(format!("{} is no soul", crate::names::form(content, f)));
        }
        if list[..i].contains(&f) {
            out.push(format!("{} is in the soul list twice", crate::names::form(content, f)));
        }
    }
    out
}

/// Write `side`'s karma and souls into `player`'s setup, each into the
/// systems that take it (none: nothing); with souls, the save's Soul
/// Unison and Chaos Unison (a finished save's event flags 0 and 0x236).
pub fn write(content: &Content, side: &Side, player: &mut PlayerSetup) -> Result<(), String> {
    // (A side without a ruleset plays by the default game's stock rules.)
    let game = crate::DEFAULT_GAME;
    player.set_fact(content, game, KARMA_FIELD, &[Fact::Value(Value::Int(side.karma as i64))])?;
    if takes(content, side, SOULS_FIELD) {
        // (Every soul, as many as the rules hold.)
        let souls: Vec<Fact> = owned_souls(content, side)
            .iter()
            .take(soul_capacity(content, side))
            .map(|f| Fact::Value(Value::Def(Registry::Form, f.0)))
            .collect();
        player.set_fact(content, game, SOULS_FIELD, &souls)?;
        player.souls.button = true;
        player.souls.chaos = true;
    }
    Ok(())
}
