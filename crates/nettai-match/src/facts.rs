//! What a side's save brings that the game's rules' systems take by a field's
//! name (S6c's facts, `PlayerSetup::set_fact`), besides EXE6's (the game,
//! the Crosses, Beast Out: exe6-compat's `Unlocks`): EXE5's karma, souls,
//! Soul Unison and Chaos Unison. A match file and a netplay offer carry them
//! as the side's own keys (`karma`, `souls`, `soul_unison`, `chaos_unison`);
//! the round's setup writes each into whichever of the side's systems
//! declares the field (EXE5's light and dark system's `karma`, its souls
//! system's the rest), and a game with none takes none.
//!
//! **Karma** is EXE5's light/dark value (NaviStats +0x44), 0 to 1000: a
//! fresh save's 500 (0x08010C00) is the default. Under 470 a dark MegaMan
//! (mood 0, the dark face and palette, dark chips, no soul button); 499 or
//! under clears holy panels; under 500 worried at the start; 1000 the
//! brightest (Tango's light templates).
//!
//! **Souls** (EXE5's Soul Unison) are the souls the side has, which the
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
//! the soul button at all (the souls system's, content/exe5/rules/souls), and
//! a dark chip's Chaos Unison. A finished save has both (the default; the
//! souls system's `setup_defaults` too); the save import reads them.

use crate::{Arena, Side, ids};
use nettai_battle::content::Content;
use nettai_battle::custom::PlayerSetup;
use nettai_battle::rules::Fact;
use nettai_content_api::{ChipHandle, FormHandle, Registry, Value};

/// The setup fields the facts go into (EXE6's game version is S6c's:
/// exe6-compat's `Unlocks::write`, its cross and beast systems').
pub const KARMA_FIELD: &str = "karma";
pub const SOULS_FIELD: &str = "souls";
/// EXE5's Soul Unison and Chaos Unison (the save's event flags 0 and 0x236).
pub const SOUL_UNISON_FIELD: &str = "soul_unison";
pub const CHAOS_UNISON_FIELD: &str = "chaos_unison";
pub const VERSION_FIELD: &str = "version";
/// What a save unlocks on the custom screen, and a side's Cross list (EXE6's
/// cross and beast systems' setups): the Crosses of the side's version it
/// owns, by their number; Beast Out; the forms its form list offers
/// instead.
pub const CROSSES_FIELD: &str = "crosses";
pub const BEAST_OUT_FIELD: &str = "beast_out";
pub const CROSS_LIST_FIELD: &str = "cross_list";
/// EXE6's bug frags: its dark-chips system's setup (a dark chip spends one).
pub const BUG_FRAGS_FIELD: &str = "bug_frags";

/// The versions a side of the game states one of, by the names its rules
/// declare, in their order: the names of the engine's version fact
/// (`PlayerFact::Version`, the `version` enum of the first of the ruleset's
/// systems whose setup declares it: EXE6's cross system's "gregar" and
/// "falzar", the original's order). None: the rules take no version. Tools
/// go by the order: they list and pick the versions in it, and a version's
/// place is its number in a navi's stats (`crate::version_byte`).
pub fn versions(content: &Content) -> &[String] {
    let defs = &content.defs;
    let Some((slot, field)) = defs.fact_field(nettai_battle::content::PlayerFact::Version) else { return &[] };
    match &defs.schema(defs.system(defs.ruleset_systems()[slot]).setup).field(field).ty {
        nettai_content_api::FieldType::Enum(names) => names,
        _ => &[],
    }
}

/// The game's versions in a phrase, for a message: "gregar or falzar".
pub fn versions_phrase(content: &Content) -> String {
    versions(content).join(" or ")
}

/// A version's name as a tool shows it: its declared name with a capital
/// ("Falzar").
pub fn version_title(name: &str) -> String {
    let mut letters = name.chars();
    letters.next().map_or(String::new(), |first| first.to_uppercase().chain(letters).collect())
}

/// Why a game whose rules take no version has none to state.
pub fn no_versions(game: &str) -> String {
    format!("its versions play alike, so a match of {game} is of neither")
}

/// The most karma there is.
pub const MAX_KARMA: u16 = 1000;

/// A fresh save's karma, which a side that states none has: the default
/// the game's rules give the `karma` field (EXE5's light and dark system's
/// `setup_defaults.karma`: 500, 0x08010C00), stated there alone. 0: the
/// rules take no karma (a side of such a game has none to state).
pub fn default_karma(content: &Content) -> u16 {
    crate::systems(content)
        .iter()
        .find_map(|&h| {
            let system = content.defs.system(h);
            let schema = content.defs.schema(system.setup);
            match system.setup_default.get(schema, schema.index_of(KARMA_FIELD)?) {
                nettai_content_api::FieldValue::U16(v) => Some(v),
                _ => None,
            }
        })
        .unwrap_or(0)
}

/// Whether a system of the game's rules declares setup field `field` (a
/// side takes that fact).
pub fn takes(content: &Content, field: &str) -> bool {
    crate::systems(content).iter().any(|&h| content.defs.schema(content.defs.system(h).setup).index_of(field).is_some())
}

/// How many forms a side's Cross list has room for (the `cross_list`
/// field's elements: EXE6's Cross window's five), none when the game's
/// rules take none.
pub fn cross_list_capacity(content: &Content) -> usize {
    capacity(content, CROSS_LIST_FIELD)
}

/// How many souls a side has room for (the `souls` field's elements:
/// EXE5's souls system's 16), none when the game's rules take none.
pub fn soul_capacity(content: &Content) -> usize {
    capacity(content, SOULS_FIELD)
}

/// The elements of the setup array `field`, of the first system of the
/// game's rules that declares it; 0: none does.
fn capacity(content: &Content, field: &str) -> usize {
    crate::systems(content)
        .iter()
        .filter_map(|&h| {
            let schema = content.defs.schema(content.defs.system(h).setup);
            match &schema.field(schema.index_of(field)?).ty {
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
        .filter(|&f| content.form(f).soul.is_some() && ids::in_game(content, game, &content.defs.form(f).key))
        .collect()
}

/// The souls `side` of a match of `game` has: its list, else every soul.
pub fn owned_souls(content: &Content, game: &str, side: &Side) -> Vec<FormHandle> {
    side.souls.clone().unwrap_or_else(|| all_souls(content, game))
}

/// What is wrong with a side's version, karma and souls on `arena`: no
/// version under rules that take one (nothing fills one in), or one under
/// rules that take none; karma past 1000, or other than the default under
/// rules that take none; a soul list under rules without souls, a form that
/// is no soul of the game's, a soul twice. (Either version's souls are
/// fine.)
pub fn check(content: &Content, arena: &Arena, side: &Side) -> Vec<String> {
    let mut out = Vec::new();
    match (side.version.as_deref(), Side::takes_version(content)) {
        (None, true) => out.push(format!("no version: a side of {} states its own ({}); none is assumed", arena.game, versions_phrase(content))),
        (Some(v), true) if !versions(content).iter().any(|name| name == v) => out.push(format!("no version {v:?} ({})", versions_phrase(content))),
        (Some(_), false) => out.push(format!("a version, but {} has none to state: {}", arena.game, no_versions(&arena.game))),
        _ => {}
    }
    if side.karma > MAX_KARMA {
        out.push(format!("karma {}: the light/dark value is 0 to {MAX_KARMA}", side.karma));
    }
    if side.karma != default_karma(content) && !takes(content, KARMA_FIELD) {
        out.push(format!("karma, but {} has no light and dark MegaMan (no system takes `karma`)", arena.game));
    }
    for (on, field, what) in [
        (side.soul_unison, SOUL_UNISON_FIELD, "Soul Unison"),
        (side.chaos_unison, CHAOS_UNISON_FIELD, "Chaos Unison"),
    ] {
        if !on && !takes(content, field) {
            out.push(format!("no {what}, but {} has none (no system takes `{field}`)", arena.game));
        }
    }
    let Some(list) = &side.souls else { return out };
    if !takes(content, SOULS_FIELD) {
        out.push(format!("a soul list, but {} has no Soul Unison (no system takes `souls`)", arena.game));
    } else if list.len() > soul_capacity(content) {
        out.push(format!("{} souls; the rules hold {}", list.len(), soul_capacity(content)));
    }
    for (i, &f) in list.iter().enumerate() {
        if f.index() >= content.defs.forms.len() || !ids::in_game(content, &arena.game, &content.defs.form(f).key) {
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

/// Write a player's version into `player`'s setup, and with it what a save
/// of that version unlocks on the custom screen, each fact by its name into
/// the systems that take it (EXE6's cross and beast systems): every Cross
/// of the version owned (a finished game's save), Beast Out as `beast_out`
/// says, and the Cross list `crosses` in place of the version's own, if
/// one is given.
pub fn write_version(content: &Content, player: &mut PlayerSetup, version: &str, beast_out: bool, crosses: Option<&crate::CrossList>) -> Result<(), String> {
    player.set_fact(content, VERSION_FIELD, &[Fact::Name(version)])?;
    let owned = vec![Fact::Value(Value::Bool(true)); capacity(content, CROSSES_FIELD)];
    player.set_fact(content, CROSSES_FIELD, &owned)?;
    player.set_fact(content, BEAST_OUT_FIELD, &[Fact::Value(Value::Bool(beast_out))])?;
    let list: Vec<Fact> = crosses.iter().flat_map(|l| l.forms()).map(|f| Fact::Value(Value::Def(Registry::Form, f.0))).collect();
    player.set_fact(content, CROSS_LIST_FIELD, &list)?;
    Ok(())
}

/// Write what `side` brings into `player`'s setup, each fact by its name
/// into the systems of the game's rules that take it (none: nothing):
/// - its version, where it states one of the rules' (a side without one
///   leaves it unstated: the rules that take one start no round, and the
///   match's checks say so first);
/// - what its save unlocks on the custom screen (EXE6's cross and beast
///   systems'): every Cross of its version owned, Beast Out as the side
///   says, and its Cross list, if it names one;
/// - its karma, bug frags and souls; with souls, the save's Soul Unison
///   and Chaos Unison (a finished save's event flags 0 and 0x236).
pub fn write(content: &Content, arena: &Arena, side: &Side, player: &mut PlayerSetup) -> Result<(), String> {
    // (No ruleset named: the game's own, its one.)
    let game = arena.game.as_str();
    if let Some(version) = side.version.as_deref().filter(|v| versions(content).iter().any(|name| name == v)) {
        write_version(content, player, version, side.beast_out, side.crosses.as_ref())?;
    }
    player.set_fact(content, KARMA_FIELD, &[Fact::Value(Value::Int(side.karma as i64))])?;
    // (EXE6's: the dark-chips system's.)
    player.set_fact(content, BUG_FRAGS_FIELD, &[Fact::Value(Value::Int(side.bug_frags as i64))])?;
    if takes(content, SOULS_FIELD) {
        // (Every soul, as many as the rules hold.)
        let souls: Vec<Fact> = owned_souls(content, game, side)
            .iter()
            .take(soul_capacity(content))
            .map(|f| Fact::Value(Value::Def(Registry::Form, f.0)))
            .collect();
        player.set_fact(content, SOULS_FIELD, &souls)?;
    }
    // Soul Unison and Chaos Unison, where the rules take them (their
    // defaults: on, a finished save's).
    for (on, field) in [(side.soul_unison, SOUL_UNISON_FIELD), (side.chaos_unison, CHAOS_UNISON_FIELD)] {
        if takes(content, field) {
            player.set_fact(content, field, &[Fact::Value(Value::Bool(on))])?;
        }
    }
    Ok(())
}

impl Side {
    /// Whether a side takes a version (one of [`versions`]: EXE6's cross
    /// and beast systems' `version`, gregar or falzar). EXE5's rules don't:
    /// its two versions play alike, and a match of it states none.
    pub fn takes_version(content: &Content) -> bool {
        !versions(content).is_empty()
    }

    /// Whether the side's navi takes a level: its definition says what a
    /// level gives it (`levels`: EXE6's MegaMan and link navis, a navi
    /// code's; `story`: EXE5's team navis; EXE5's MegaMan has neither).
    pub fn takes_level(&self, content: &Content) -> bool {
        let navi = content.navi(self.navi);
        navi.levels.is_some() || navi.story.is_some()
    }

    /// Whether a side takes SP navi deletion times (the game's rules'
    /// `sp_slots`: EXE6's and EXE5's, each their own SP navis).
    pub fn takes_sp_times(content: &Content) -> bool {
        !crate::sp_slots(content).is_empty()
    }
}

/// The SP navi chip of the arena's game whose damage reads slot `slot` of
/// its rules, if the game has it: the slot's name in a tool.
pub fn sp_chip(content: &Content, arena: &Arena, slot: usize) -> Option<ChipHandle> {
    (0..content.defs.chips.len() as u16)
        .map(ChipHandle)
        .find(|&h| content.chip_links(h).sp_slot == Some(slot as u8) && ids::in_game(content, &arena.game, &content.defs.chip(h).key))
}
