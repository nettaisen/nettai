//! Names in a match: once the arena chooses the game, everything else a
//! match names (its stages and backgrounds, each side's navi,
//! chips, Crosses, souls, patch cards, NaviCust programs, weapons and
//! records) is a local name in that game's namespace (`cannon`), looked up
//! only there. There is no way to write another game's name: a name the
//! game hasn't is the ordinary "no chip `x` in exe6". The one exception is
//! a background, which may be any of the background pool's, every game's
//! (docs/app.md §1, "The battle's background"): another game's is written
//! with its game, `exe4:acdc-area` ([`background`]).
//!
//! Every lookup goes through [`key`] (a game and a local name to the
//! content's key: the name itself, when the content is that game's), and
//! every name written through [`local`]: the content keys its definitions
//! by their local names (docs/design/content-model-v2.md §4.0), and a
//! support pack's anonymous definitions by their module (`exelib:regions#57`),
//! which no match names.

use nettai_battle::content::Content;
use nettai_content_api::{
    ChipHandle, EntryHandle, FormHandle, NaviHandle, RecordHandle, Registry, StageHandle, WeaponHandle, keys,
};

/// The content's key of the definition named `name` in `game`: the name,
/// when `content` is that game's; none in another game's content.
pub fn key<'n>(content: &Content, game: &str, name: &'n str) -> Option<&'n str> {
    (content.game() == game && keys::root_of(name).is_none()).then_some(name)
}

/// The name a match writes for the definition keyed `key`: the key, for a
/// definition of the game ([`in_game`]; its key is its name), and a
/// support pack's anonymous one without its pack.
pub fn local(key: &str) -> &str {
    keys::local(key)
}

/// Whether `content`'s definition keyed `key` is one a match of `game` can
/// name: `content` is that game's, and the definition is the game's, keyed
/// by its name (not a support pack's anonymous one, keyed by its module).
pub fn in_game(content: &Content, game: &str, key: &str) -> bool {
    content.game() == game && keys::root_of(key).is_none()
}

pub fn chip(content: &Content, game: &str, name: &str) -> Option<ChipHandle> {
    key(content, game, name).and_then(|k| content.defs.chip_by_key(k))
}

pub fn navi(content: &Content, game: &str, name: &str) -> Option<NaviHandle> {
    key(content, game, name).and_then(|k| content.defs.navi_by_key(k))
}

pub fn form(content: &Content, game: &str, name: &str) -> Option<FormHandle> {
    key(content, game, name).and_then(|k| content.defs.form_by_key(k))
}

pub fn stage(content: &Content, game: &str, name: &str) -> Option<StageHandle> {
    key(content, game, name).and_then(|k| content.defs.stage_by_key(k))
}

/// Entry `name` of `game`'s collection `collection` (the game's root's
/// `patch_cards`, say).
pub fn entry(content: &Content, game: &str, collection: &str, name: &str) -> Option<EntryHandle> {
    key(content, game, name).and_then(|k| content.defs.entry_in(collection, k))
}


pub fn weapon(content: &Content, game: &str, name: &str) -> Option<WeaponHandle> {
    key(content, game, name).and_then(|k| content.defs.weapon_by_key(k))
}

pub fn record(content: &Content, game: &str, name: &str) -> Option<RecordHandle> {
    key(content, game, name).and_then(|k| content.defs.record(k))
}

/// The content's key of definition `h` of `registry`, for the registries a
/// side's fact may hold a definition of (a form, a chip, a navi, a stage, a
/// weapon, an entry of one of the game's collections); none: the content
/// has no such definition, or the registry is none of those.
pub fn key_of(content: &Content, registry: Registry, h: u16) -> Option<&str> {
    let defs = &content.defs;
    let i = h as usize;
    Some(match registry {
        Registry::Form if i < defs.forms.len() => defs.form(FormHandle(h)).key.as_str(),
        Registry::Chip if i < defs.chips.len() => defs.chip(ChipHandle(h)).key.as_str(),
        Registry::Navi if i < defs.navis.len() => defs.navi(NaviHandle(h)).key.as_str(),
        Registry::Stage if i < defs.stages.len() => defs.stage(StageHandle(h)).key.as_str(),
        Registry::Weapon if i < defs.weapons.len() => defs.weapon(WeaponHandle(h)).key.as_str(),
        Registry::Entry if i < defs.entries.len() => defs.entry(EntryHandle(h)).key.as_str(),
        _ => return None,
    })
}

/// The name a match writes for definition `h` of `registry`: an entry's
/// id in its collection, else the definition's local name ([`local`]).
pub fn name_of(content: &Content, registry: Registry, h: u16) -> Option<&str> {
    match registry {
        Registry::Entry => Some(content.defs.entries.get(h as usize)?.id()),
        _ => key_of(content, registry, h).map(local),
    }
}

/// The definitions of `registry` a field of it may hold, by handle: those
/// of the collection `of` names, for an entry (`patch_cards`); every one,
/// for the other registries [`key_of`] knows.
pub fn all_of(content: &Content, registry: Registry, of: Option<&str>) -> Vec<u16> {
    match (registry, of) {
        (Registry::Entry, Some(c)) => content.defs.entries_of(c).into_iter().map(|h| h.0).collect(),
        (Registry::Entry, None) => Vec::new(),
        _ => (0..=u16::MAX).map_while(|h| key_of(content, registry, h).map(|_| h)).collect(),
    }
}

/// The definition of `registry` named `name` in `game` (its handle), for
/// the registries [`key_of`] knows: an entry's in the collection `of` names.
pub fn handle_of(content: &Content, game: &str, registry: Registry, of: Option<&str>, name: &str) -> Option<u16> {
    match registry {
        Registry::Entry => entry(content, game, of?, name).map(|h| h.0),
        Registry::Form => form(content, game, name).map(|h| h.0),
        Registry::Chip => chip(content, game, name).map(|h| h.0),
        Registry::Navi => navi(content, game, name).map(|h| h.0),
        Registry::Stage => stage(content, game, name).map(|h| h.0),
        Registry::Weapon => weapon(content, game, name).map(|h| h.0),
        _ => None,
    }
}

/// Definition `h` of `registry` for a message: its name in the content's
/// own strings where it has one, else its key, else its number.
pub fn shown(content: &Content, registry: Registry, h: u16) -> String {
    match (registry, key_of(content, registry, h)) {
        (Registry::Form, Some(_)) => crate::names::form(content, FormHandle(h)).to_string(),
        (Registry::Chip, Some(_)) => crate::names::chip(content, ChipHandle(h)).to_string(),
        (Registry::Navi, Some(_)) => crate::names::navi(content, NaviHandle(h)).to_string(),
        (Registry::Entry, Some(_)) => crate::names::entry(content, EntryHandle(h)),
        (_, Some(key)) => local(key).to_string(),
        (_, None) => format!("{registry} {h}"),
    }
}

/// A background a match of `game` names `name`: one of `game`'s pack by its
/// name there (`acdc-area`), or one of the background pool's by its game and
/// its name in that game's pack (`exe4:acdc-area`: the one name a match
/// writes of another game's, docs/app.md §1, "The battle's background").
/// None: neither has it, or `content` isn't `game`'s.
pub fn background(content: &Content, game: &str, name: &str) -> Option<nettai_battle::content::BackgroundId> {
    if content.game() != game {
        return None;
    }
    match name.split_once(':') {
        Some((of, name)) => content.background_handle(of, name),
        None => content.background_handle(game, name),
    }
}

/// The name a match writes for background `id` ([`background`]): one of the
/// game's own by its name in the game's pack, one of the pool's others by its
/// game and its name in that game's pack (`exe4:acdc-area`); none: no such
/// background.
pub fn background_name(content: &Content, id: nettai_battle::content::BackgroundId) -> Option<&str> {
    match content.background_of(id)? {
        (game, name) if game == content.game() => Some(local(name)),
        _ => content.other_background(id),
    }
}

/// The names of `game`'s backgrounds, in order: those its modules name and
/// the others of its own the background pool has.
pub fn backgrounds<'c>(content: &'c Content, game: &str) -> Vec<&'c str> {
    if content.game() != game {
        return Vec::new();
    }
    let pool = content.background_pool.iter().filter(|(g, _)| *g == game).map(|(_, name)| name);
    let mut out: Vec<&str> = content.assets.backgrounds.iter().map(String::as_str).chain(pool).map(local).collect();
    out.sort();
    out.dedup();
    out
}

/// The names a match of the content's game writes for the background pool's
/// backgrounds ([`background_name`]), in the pool's order: every game's.
pub fn pool_backgrounds(content: &Content) -> Vec<&str> {
    content.pool_backgrounds().into_iter().filter_map(|id| background_name(content, id)).collect()
}

/// The games a match on `content` can be of: its game, when it has a
/// rules and a link battle stage (a content holds one game).
pub fn games(content: &Content) -> Vec<String> {
    let game = content.game();
    let playable = crate::playable(content, game).is_ok() && !crate::link_battle_stages(content, game).is_empty();
    if playable { vec![game.to_string()] } else { Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A match's lookups see only its game: a name is its game's (`cannon`
    /// in exe6 is EXE6's Cannon), and a name the game hasn't, whatever it is
    /// (another game's, written in full), is none.
    #[test]
    fn lookups_see_only_their_game() {
        let content = crate::testing::exe6_content();
        let six = chip(&content, "exe6", "cannon").unwrap();
        assert_eq!(local(&content.defs.chip(six).key), "cannon");
        assert!(form(&content, "exe6", "heatcross").is_some());
        assert_eq!(chip(&content, "exe5", "cannon"), None);
        assert_eq!(chip(&content, "exe6", "exe6:cannon"), None); // (written in full)
        assert_eq!(games(&content), ["exe6"]);
    }
}
