//! Names in a match: once the arena chooses the game, everything else a
//! match names (its stages and backgrounds, each side's ruleset, navi,
//! chips, Crosses, souls, patch cards, NaviCust programs, weapons and
//! records) is a local name in that game's namespace (`cannon`), looked up
//! only there. There is no way to write another game's name: a name the
//! game hasn't is the ordinary "no chip `x` in bn6".
//!
//! Every lookup goes through [`key`] (a game and a local name to the
//! content's key), and every name written through [`local`]: the content
//! keys its definitions in full today (`bn6:cannon`), and when it keys them
//! by local name, these two are what changes.

use nettai_battle::content::Content;
use nettai_content_api::{
    AssetKind, ChipHandle, FormHandle, NaviCustProgramHandle, NaviHandle, PatchCardHandle, RecordHandle, RulesetHandle, StageHandle,
    WeaponHandle, keys,
};

/// The content's key of the definition named `name` in `game`.
pub fn key(game: &str, name: &str) -> String {
    format!("{game}{}{name}", keys::SEPARATOR)
}

/// The name a match writes for the definition keyed `key` (its game's).
pub fn local(key: &str) -> &str {
    keys::local(key)
}

/// Whether the definition keyed `key` is one of `game`'s.
pub fn in_game(game: &str, key: &str) -> bool {
    keys::root_of(key) == Some(game)
}

pub fn chip(content: &Content, game: &str, name: &str) -> Option<ChipHandle> {
    content.defs.chip_by_key(&key(game, name))
}

pub fn navi(content: &Content, game: &str, name: &str) -> Option<NaviHandle> {
    content.defs.navi_by_key(&key(game, name))
}

pub fn ruleset(content: &Content, game: &str, name: &str) -> Option<RulesetHandle> {
    content.defs.ruleset_by_key(&key(game, name))
}

pub fn form(content: &Content, game: &str, name: &str) -> Option<FormHandle> {
    content.defs.form_by_key(&key(game, name))
}

pub fn stage(content: &Content, game: &str, name: &str) -> Option<StageHandle> {
    content.defs.stage_by_key(&key(game, name))
}

pub fn patch_card(content: &Content, game: &str, name: &str) -> Option<PatchCardHandle> {
    content.defs.patch_card_by_key(&key(game, name))
}

pub fn navicust_program(content: &Content, game: &str, name: &str) -> Option<NaviCustProgramHandle> {
    content.defs.navicust_program_by_key(&key(game, name))
}

pub fn weapon(content: &Content, game: &str, name: &str) -> Option<WeaponHandle> {
    content.defs.weapon_by_key(&key(game, name))
}

pub fn record(content: &Content, game: &str, name: &str) -> Option<RecordHandle> {
    content.defs.record(&key(game, name))
}

/// A background of `game`'s pack by name.
pub fn background(content: &Content, game: &str, name: &str) -> Option<nettai_battle::content::BackgroundId> {
    content.assets.handle(AssetKind::Background, &key(game, name)).map(nettai_battle::content::BackgroundId)
}

/// The names of `game`'s backgrounds, in order.
pub fn backgrounds<'c>(content: &'c Content, game: &str) -> Vec<&'c str> {
    content.assets.backgrounds.keys().filter(|k| in_game(game, k)).map(|k| local(k)).collect()
}

/// The games a match on `content` can be of: its game, when it has a stock
/// ruleset and a link battle stage (a content holds one game).
pub fn games(content: &Content) -> Vec<String> {
    let game = content.game();
    let playable = content.defs.stock_ruleset().is_some() && !crate::link_battle_stages(content, game).is_empty();
    if playable { vec![game.to_string()] } else { Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A match's lookups see only its game: a name is its game's (`cannon`
    /// in bn6 is BN6's Cannon), and a name the game hasn't, whatever it is
    /// (another game's, written in full), is none.
    #[test]
    fn lookups_see_only_their_game() {
        let content = crate::testing::bn6_content();
        let six = chip(&content, "bn6", "cannon").unwrap();
        assert_eq!(local(&content.defs.chip(six).key), "cannon");
        assert!(form(&content, "bn6", "heatcross").is_some());
        assert_eq!(chip(&content, "bn5", "cannon"), None);
        assert_eq!(chip(&content, "bn6", "bn6:cannon"), None);
        assert_eq!(ruleset(&content, "bn6", "stock").map(|r| content.defs.ruleset(r).key.as_str()), Some("stock"));
        assert_eq!(games(&content), ["bn6"]);
    }
}
