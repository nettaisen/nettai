//! What a side's save brings that its ruleset's systems take by a field's
//! name (S6c's facts, `PlayerSetup::set_fact`), besides BN6's (the game,
//! the Crosses, Beast Out: bn6-compat's `Unlocks`): BN5's karma, souls,
//! Soul Unison and Chaos Unison. A match file and a netplay offer carry them
//! as the side's own keys (`karma`, `souls`, `soul_unison`, `chaos_unison`);
//! the round's setup writes each into whichever of the side's systems
//! declares the field (BN5's light and dark system's `karma`, its souls
//! system's the rest), and a ruleset with none takes none.
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
//! the match's game, of either version: none listed, every soul. A real
//! save holds its own version's six; the save import reads them.
//!
//! **Soul Unison and Chaos Unison** are the save's event flags 0 and 0x236:
//! the soul button at all (the souls system's, content/bn5/rules/souls), and
//! a dark chip's Chaos Unison. A finished save has both (the default; the
//! souls system's `setup_defaults` too); the save import reads them.

use crate::{Arena, Side, ids};
use nettai_battle::content::Content;
use nettai_battle::custom::PlayerSetup;
use nettai_battle::rules::Fact;
use nettai_content_api::{ChipHandle, FormHandle, Registry, RulesetHandle, Value};

/// The setup fields the facts go into (BN6's game version is S6c's:
/// bn6-compat's `Unlocks::write`, its cross and beast systems').
pub const KARMA_FIELD: &str = "karma";
pub const SOULS_FIELD: &str = "souls";
/// BN5's Soul Unison and Chaos Unison (the save's event flags 0 and 0x236).
pub const SOUL_UNISON_FIELD: &str = "soul_unison";
pub const CHAOS_UNISON_FIELD: &str = "chaos_unison";
pub const VERSION_FIELD: &str = "version";
/// BN6's bug frags: its dark-chips system's setup (a dark chip spends one).
pub const BUG_FRAGS_FIELD: &str = "bug_frags";

/// A fresh save's karma (0x08010C00), and the most there is.
pub const DEFAULT_KARMA: u16 = 500;
pub const MAX_KARMA: u16 = 1000;

/// Whether a system of `ruleset` declares setup field `field` (a side
/// under those rules takes that fact).
pub fn takes(content: &Content, _ruleset: RulesetHandle, field: &str) -> bool {
    content.defs.ruleset_systems().iter().any(|&h| content.defs.schema(content.defs.system(h).setup).index_of(field).is_some())
}

/// How many souls a side under `ruleset` has room for (the `souls`
/// field's elements: BN5's souls system's 16), none when the rules take
/// none.
pub fn soul_capacity(content: &Content, _ruleset: RulesetHandle) -> usize {
    content
        .defs
        .ruleset_systems()
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

/// Every soul of `game` (its forms with a `soul`), in handle order, of
/// either version.
pub fn all_souls(content: &Content, game: &str) -> Vec<FormHandle> {
    (0..content.defs.forms.len() as u16)
        .map(FormHandle)
        .filter(|&f| content.form(f).soul.is_some() && ids::in_game(game, &content.defs.form(f).key))
        .collect()
}

/// The souls `side` of a match of `game` has: its list, else every soul.
pub fn owned_souls(content: &Content, game: &str, side: &Side) -> Vec<FormHandle> {
    side.souls.clone().unwrap_or_else(|| all_souls(content, game))
}

/// What is wrong with a side's karma and souls on `arena`: karma past
/// 1000, or other than the default under rules that take none; a soul list
/// under rules without souls, a form that is no soul of the game's, a soul
/// twice. (Either version's souls are fine.)
pub fn check(content: &Content, arena: &Arena, side: &Side) -> Vec<String> {
    let mut out = Vec::new();
    let ruleset = arena.ruleset;
    if side.karma > MAX_KARMA {
        out.push(format!("karma {}: the light/dark value is 0 to {MAX_KARMA}", side.karma));
    }
    if side.karma != DEFAULT_KARMA && !takes(content, ruleset, KARMA_FIELD) {
        out.push("karma, but the ruleset has no light and dark MegaMan (no system takes `karma`)".into());
    }
    for (on, field, what) in [
        (side.soul_unison, SOUL_UNISON_FIELD, "Soul Unison"),
        (side.chaos_unison, CHAOS_UNISON_FIELD, "Chaos Unison"),
    ] {
        if !on && !takes(content, ruleset, field) {
            out.push(format!("no {what}, but the ruleset has none (no system takes `{field}`)"));
        }
    }
    let Some(list) = &side.souls else { return out };
    if !takes(content, ruleset, SOULS_FIELD) {
        out.push("a soul list, but the ruleset has no Soul Unison (no system takes `souls`)".into());
    } else if list.len() > soul_capacity(content, ruleset) {
        out.push(format!("{} souls; the rules hold {}", list.len(), soul_capacity(content, ruleset)));
    }
    for (i, &f) in list.iter().enumerate() {
        if f.index() >= content.defs.forms.len() || !ids::in_game(&arena.game, &content.defs.form(f).key) {
            out.push(format!("a soul {} hasn't", arena.game));
            continue;
        }
        if content.form(f).soul.is_none() {
            out.push(format!("{} is no soul", crate::names::form(content, f)));
        }
        if list[..i].contains(&f) {
            out.push(format!("{} is in the soul list twice", crate::names::form(content, f)));
        }
    }
    out
}

/// Write `side`'s karma and souls into `player`'s setup, each into the
/// systems of the arena's rules that take it (none: nothing); with souls,
/// the save's Soul Unison and Chaos Unison (a finished save's event flags
/// 0 and 0x236).
pub fn write(content: &Content, arena: &Arena, side: &Side, player: &mut PlayerSetup) -> Result<(), String> {
    let game = arena.game.as_str();
    player.set_fact(content, KARMA_FIELD, &[Fact::Value(Value::Int(side.karma as i64))])?;
    // (BN6's: the dark-chips system's.)
    player.set_fact(content, BUG_FRAGS_FIELD, &[Fact::Value(Value::Int(side.bug_frags as i64))])?;
    if takes(content, arena.ruleset, SOULS_FIELD) {
        // (Every soul, as many as the rules hold.)
        let souls: Vec<Fact> = owned_souls(content, game, side)
            .iter()
            .take(soul_capacity(content, arena.ruleset))
            .map(|f| Fact::Value(Value::Def(Registry::Form, f.0)))
            .collect();
        player.set_fact(content, SOULS_FIELD, &souls)?;
    }
    // Soul Unison and Chaos Unison, where the rules take them (their
    // defaults: on, a finished save's).
    for (on, field) in [(side.soul_unison, SOUL_UNISON_FIELD), (side.chaos_unison, CHAOS_UNISON_FIELD)] {
        if takes(content, arena.ruleset, field) {
            player.set_fact(content, field, &[Fact::Value(Value::Bool(on))])?;
        }
    }
    Ok(())
}

impl Side {
    /// Whether a side under `ruleset` takes its version (Gregar or Falzar:
    /// BN6's cross and beast systems' `version`). BN5's rules don't.
    pub fn takes_game(content: &Content, ruleset: RulesetHandle) -> bool {
        takes(content, ruleset, VERSION_FIELD)
    }

    /// Whether the side's navi takes a navi code's level: its definition
    /// says what a level gives it (`levels`: BN6's MegaMan and link navis;
    /// BN5's MegaMan has none).
    pub fn takes_level(&self, content: &Content) -> bool {
        content.navi(self.navi).levels.is_some()
    }

    /// Whether a side takes SP navi deletion times (the game's rules'
    /// `sp_slots`: BN6's and BN5's, each their own SP navis).
    pub fn takes_sp_times(content: &Content) -> bool {
        !crate::sp_slots(content).is_empty()
    }

    /// The side, on rules `new` (of its game's, whose SP navis are the
    /// game's), without what they don't take: the Crosses, patch cards and
    /// NaviCust without their systems, the karma, souls, Soul Unison and
    /// Chaos Unison without theirs,
    /// and the version (back to Falzar) without `version`.
    pub fn fit_rules(&mut self, content: &Content, new: RulesetHandle) {
        let has = |system| crate::ruleset_has_system(content, new, system);
        if !has(crate::FORMS_SYSTEM) {
            self.crosses = None;
        }
        if !has(crate::PATCH_CARDS_SYSTEM) {
            self.cards.clear();
        }
        if !has(crate::NAVICUST_SYSTEM) {
            self.navicust = None;
        }
        if !takes(content, new, SOULS_FIELD) {
            self.souls = None;
        }
        if !takes(content, new, SOUL_UNISON_FIELD) {
            self.soul_unison = true;
        }
        if !takes(content, new, CHAOS_UNISON_FIELD) {
            self.chaos_unison = true;
        }
        if !takes(content, new, KARMA_FIELD) {
            self.karma = DEFAULT_KARMA;
        }
        if !Side::takes_game(content, new) {
            self.game = nettai_battle::custom::GameVersion::Falzar;
            self.stats.version = crate::version_byte(self.game);
        }
    }
}

/// The SP navi chip of the arena's game whose damage reads slot `slot` of
/// its rules, if the game has it: the slot's name in a tool.
pub fn sp_chip(content: &Content, arena: &Arena, slot: usize) -> Option<ChipHandle> {
    (0..content.defs.chips.len() as u16)
        .map(ChipHandle)
        .find(|&h| content.chip_links(h).sp_slot == Some(slot as u8) && ids::in_game(&arena.game, &content.defs.chip(h).key))
}
