//! What a save or a recording states of a player as EXE4's rules' setup
//! facts (content/exe4/rules/init.luau): its NaviCust's programs, its Mod
//! Cards, its auto battle data.

use crate::Compat;
use crate::save::{AUTO_BATTLE_EMPTY, AUTO_BATTLE_PATTERN, AUTO_BATTLE_PATTERNS, Part};
use nettai_battle::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{Registry, Value};

/// A NaviCust list (the save's, in its order) as the setup's
/// `navicust_programs`: each part's program and color (compat's
/// navicust.toml), its center on the engine's 7x7 grid (the save's column
/// and row, from 0 on the 5x5 board, one more), its quarter turns and its
/// compression.
pub fn navicust<'c>(content: &'c Content, compat: &Compat, parts: &[Option<Part>]) -> Result<Vec<Fact<'c>>, String> {
    let mut out = Vec::new();
    for p in parts.iter().flatten() {
        let Some((key, color)) = compat.navicust_part(p.id)? else { continue };
        let program = content.defs.entry_in("navicust_programs", key).ok_or_else(|| format!("the content has no NaviCust program {key}"))?;
        if p.column > 4 || p.row > 4 || p.rotation > 3 {
            return Err(format!("NaviCust part {:#04x} at column {}, row {}, turned {}: off EXE4's 5x5 board", p.id, p.column, p.row, p.rotation));
        }
        let colors = match content.defs.definitions.get(Registry::Entry, &content.defs.entry(program).key).map(|d| d.spec.field("colors")) {
            Some(nettai_content_api::Data::List(colors)) => colors.iter().filter_map(|c| c.str()).collect(),
            _ => Vec::new(),
        };
        let color = colors.get(color as usize).copied().ok_or_else(|| format!("NaviCust part {:#04x}: {key} has no color {color}", p.id))?;
        out.push(Fact::Record(vec![
            ("program", Fact::Value(Value::Def(Registry::Entry, program.0))),
            ("color", Fact::Name(color)),
            ("x", Fact::Value(Value::Int(p.column as i64 + 1))),
            ("y", Fact::Value(Value::Int(p.row as i64 + 1))),
            ("rotation", Fact::Value(Value::Int(p.rotation as i64))),
            ("compressed", Fact::Value(Value::Bool(p.compressed))),
        ]));
    }
    Ok(out)
}

/// A player's auto battle data's places (a save's block's 42 halfwords, or
/// those a recording carries of it) as the setup's `auto_battle_places`,
/// place for place to the last one that isn't empty: a chip by its number
/// (`chip` gives the content's, `{ chip }`), a pattern entry by its
/// record's number from 1 (`{ pattern }`), a 0 (`{ zero = true }`), an empty
/// place `{}`. What a setup can't state of it is left empty and comes back
/// apart, said: a chip's number `chip` has none for, an entry for a pattern
/// record past the block's eight.
pub fn auto_battle<'c>(places: &[u16], chip: impl Fn(u16) -> Option<nettai_content_api::ChipHandle>) -> (Vec<Fact<'c>>, Vec<String>) {
    let mut left_out = Vec::new();
    let mut nameless: Vec<u16> = Vec::new();
    let mut out: Vec<Fact> = Vec::new();
    for (i, &h) in places.iter().enumerate() {
        out.push(Fact::Record(match h {
            AUTO_BATTLE_EMPTY => Vec::new(),
            0 => vec![("zero", Fact::Value(Value::Bool(true)))],
            h if h & AUTO_BATTLE_PATTERN != 0 => match (h & !AUTO_BATTLE_PATTERN) as usize {
                n if n < AUTO_BATTLE_PATTERNS => vec![("pattern", Fact::Value(Value::Int(n as i64 + 1)))],
                n => {
                    left_out.push(format!("place {} of the auto battle data names pattern {}, past its eight: left empty", i + 1, n + 1));
                    Vec::new()
                }
            },
            h => match chip(h) {
                Some(c) => vec![("chip", Fact::Value(Value::Def(Registry::Chip, c.0)))],
                None => {
                    if !nameless.contains(&h) {
                        nameless.push(h);
                    }
                    Vec::new()
                }
            },
        }));
    }
    let used = out.iter().rposition(|p| !matches!(p, Fact::Record(f) if f.is_empty())).map_or(0, |i| i + 1);
    out.truncate(used);
    if !nameless.is_empty() {
        let numbers: Vec<String> = nameless.iter().map(|n| format!("{n:#05x}")).collect();
        left_out.push(format!("the auto battle data holds chip numbers the content has no chip for ({}): their places are left empty", numbers.join(", ")));
    }
    (out, left_out)
}

/// A save's patch card slots switched on (by slot: a card's number, none for an
/// empty slot) as the setup's `patch_cards`: each card and its slot. The
/// numbers of cards the content hasn't (their effects wait) come back apart.
pub fn patch_cards<'c>(content: &'c Content, compat: &Compat, slots: &[Option<u8>]) -> Result<(Vec<Fact<'c>>, Vec<u8>), String> {
    let mut out = Vec::new();
    let mut waiting = Vec::new();
    for (slot, n) in slots.iter().enumerate() {
        let Some(n) = *n else { continue };
        let Some(key) = compat.patch_card(n) else {
            waiting.push(n);
            continue;
        };
        let card = content.defs.entry_in("patch_cards", key).ok_or_else(|| format!("the content has no patch card {key}"))?;
        out.push(Fact::Record(vec![
            ("card", Fact::Value(Value::Def(Registry::Entry, card.0))),
            ("slot", Fact::Value(Value::Int(slot as i64))),
        ]));
    }
    Ok((out, waiting))
}
