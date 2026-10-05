//! Names in a match: once the arena chooses the game, everything else a
//! match names (its stages and backgrounds, each side's navi,
//! chips, Crosses, souls, patch cards, NaviCust programs, weapons and
//! records) is a local name in that game's namespace (`cannon`), looked up
//! only there. There is no way to write another game's name: a name the
//! game hasn't is the ordinary "no chip `x` in exe6".
//!
//! Every lookup goes through [`key`] (a game and a local name to the
//! content's key: the name itself, when the content is that game's), and
//! every name written through [`local`]: the content keys its definitions
//! by their local names (docs/design/content-model-v2.md §4.0), and a
//! support pack's anonymous definitions by their module (`exelib:regions#57`),
//! which no match names.

use nettai_battle::content::Content;
use nettai_content_api::{
    AssetKind, ChipHandle, FormHandle, NaviCustProgramHandle, NaviHandle, PatchCardHandle, RecordHandle, StageHandle,
    WeaponHandle, keys,
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

pub fn patch_card(content: &Content, game: &str, name: &str) -> Option<PatchCardHandle> {
    key(content, game, name).and_then(|k| content.defs.patch_card_by_key(k))
}

pub fn navicust_program(content: &Content, game: &str, name: &str) -> Option<NaviCustProgramHandle> {
    key(content, game, name).and_then(|k| content.defs.navicust_program_by_key(k))
}

pub fn weapon(content: &Content, game: &str, name: &str) -> Option<WeaponHandle> {
    key(content, game, name).and_then(|k| content.defs.weapon_by_key(k))
}

pub fn record(content: &Content, game: &str, name: &str) -> Option<RecordHandle> {
    key(content, game, name).and_then(|k| content.defs.record(k))
}

/// A background of `game`'s pack by name.
pub fn background(content: &Content, game: &str, name: &str) -> Option<nettai_battle::content::BackgroundId> {
    key(content, game, name).and_then(|k| content.assets.handle(AssetKind::Background, k)).map(nettai_battle::content::BackgroundId)
}

/// The name of background `id` in its game's pack (a match names a
/// background by it); none: no background of the content's game.
pub fn background_name(content: &Content, id: nettai_battle::content::BackgroundId) -> Option<&str> {
    content.assets.backgrounds.keys().find(|k| content.assets.handle(AssetKind::Background, k) == Some(id.0)).map(|k| local(k))
}

/// The names of `game`'s backgrounds, in order.
pub fn backgrounds<'c>(content: &'c Content, game: &str) -> Vec<&'c str> {
    (content.game() == game).then(|| content.assets.backgrounds.keys().map(|k| local(k)).collect()).unwrap_or_default()
}

/// The games a match on `content` can be of: its game, when it has a
/// ruleset and a link battle stage (a content holds one game).
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
