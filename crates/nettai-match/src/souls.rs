//! A side's souls: BN5's Soul Unison (docs/design/bn5-map.md §5), the
//! souls the player has, which the custom screen's soul button offers.
//!
//! The original's soul button (0x08024B28) offers, for the last chip
//! picked, the soul of its family (the table 0x08024BE0, a family a soul)
//! when the save has it: the version's table 0x08024BF0 gives each soul's
//! event flag, Team ProtoMan's souls 1 to 6 flags 2 to 7 and Team
//! Colonel's 7 to 12 flags 8 to 0x0D, the other version's none (0xFF: never
//! offered); a dark chip's Chaos Unison needs event flag 0x236 too; and a
//! soul given this round isn't offered again. The engine ports that check
//! (`nettai_battle::custom::screen`'s `update_soul`), with what the save
//! has as the setup's `SoulUnlocks`: the button (event flag 0), the souls
//! owned, Chaos Unison.
//!
//! A side lists the souls it has (none listed: every soul the content
//! has), of either version: nettai's extension, as a Cross list may name
//! either game's Crosses (the user: "allow all souls to be selected
//! regardless of game"). A real save holds its own version's six alone;
//! the save import reads them (`Side::import_bn5_save`). A soul whose chip
//! family the folder never holds never comes up.

use crate::Side;
use nettai_battle::content::{Content, FormKind};
use nettai_battle::custom::SoulUnlocks;
use nettai_content_api::FormHandle;

/// The system that keeps a soul's turns (BN5's): a side lists souls only
/// under a ruleset with it.
pub const SOULS_SYSTEM: &str = "souls";

/// Every soul the content has (forms of kind soul), in handle order: of
/// every game, either version.
pub fn all(content: &Content) -> Vec<FormHandle> {
    (0..content.defs.forms.len() as u16).map(FormHandle).filter(|&f| content.form(f).kind == FormKind::Soul).collect()
}

/// The souls `side` has: its list, else every soul.
pub fn owned(content: &Content, side: &Side) -> Vec<FormHandle> {
    side.souls.clone().unwrap_or_else(|| all(content))
}

/// What the side's save has of Soul Unison, for its round: the souls it
/// has (each by its number), and with any, the soul button and Chaos
/// Unison (a finished save's event flags 0 and 0x236). None without the
/// souls system.
pub fn unlocks(content: &Content, side: &Side) -> SoulUnlocks {
    if !side.has_system(content, SOULS_SYSTEM) {
        return SoulUnlocks::default();
    }
    let owned = owned(content, side).iter().filter_map(|&f| content.form(f).soul.as_ref().map(|s| 1u16 << (s.number & 15))).fold(0, |a, b| a | b);
    SoulUnlocks { button: owned != 0, owned, chaos: owned != 0, turn_bonus: 0 }
}

/// What is wrong with a side's soul list: a list under a ruleset without
/// the souls system, a form that is no soul, a soul twice. (Either
/// version's souls are fine.)
pub fn check(content: &Content, side: &Side) -> Vec<String> {
    let Some(list) = &side.souls else { return Vec::new() };
    let mut out = Vec::new();
    if !side.has_system(content, SOULS_SYSTEM) {
        out.push("a soul list, but the ruleset has no Soul Unison (no souls system)".into());
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
