//! The background pool (docs/app.md §1, "The battle's background"): every background of the content's games, one
//! pool a round may be drawn on whatever its game. The user: "is it possible to use e.g. exe4 backgrounds in exe5?
//! as in if they all came from the same pool of backgrounds?", then "sure do that".
//!
//! A round's background is a [`BackgroundId`], a handle over the round's backgrounds: the game's own first (its
//! asset table's, `Content::assets`: what its modules name, `asset.background("comp")`, at their asset handles),
//! then the pool's others, each another game's (or one of the game's own that its modules don't name), in the
//! pool's order. No tick reads a background (the renderer alone does), so the pool is
//! presentation: it is left out of `Content::hash`, as the strings are, and a game's content is the same whatever
//! other games stand beside it. Two peers compare their pools on their own ([`BackgroundPool::identity`]).

use super::{BackgroundId, Content};
use nettai_content_api::AssetKind;

/// Every background of the content's games, each by its game and its name in its game's pack: the games in their
/// ids' order, each game's backgrounds in their names' (byte) order, each once. What each game's content names
/// (its own language's `[backgrounds]`, read by the loader), so the same whatever games' ROMs a player has.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackgroundPool {
    /// Each background's key, `<game>:<name>`, in the pool's order.
    keys: Vec<String>,
}

/// Left out of `Content::hash` (presentation; see the module docs).
impl std::hash::Hash for BackgroundPool {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// The key the pool gives `game`'s background `name`: `<game>:<name>` (`exe4:acdc-area`).
pub fn pool_key(game: &str, name: &str) -> String {
    format!("{game}:{name}")
}

impl BackgroundPool {
    /// The pool of `backgrounds`, each its game and its name in its game, put in the pool's order.
    pub fn new(backgrounds: impl IntoIterator<Item = (String, String)>) -> BackgroundPool {
        let mut pairs: Vec<(String, String)> = backgrounds.into_iter().collect();
        pairs.sort();
        pairs.dedup();
        BackgroundPool { keys: pairs.iter().map(|(game, name)| pool_key(game, name)).collect() }
    }

    /// Each background's game and name, in the pool's order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.keys.iter().map(|k| split(k))
    }

    /// Each background's key (`<game>:<name>`), in the pool's order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.keys.iter().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The pool's identity, which two peers compare: a stable hash of its backgrounds, in order.
    pub fn identity(&self) -> u64 {
        crate::digest::stable_hash(&self.keys)
    }
}

/// A pool key's game and name.
fn split(key: &str) -> (&str, &str) {
    key.split_once(':').expect("a pool key is <game>:<name>")
}

impl Content {
    /// The game's own backgrounds: its asset table's, handles `0..` this.
    fn own_backgrounds(&self) -> usize {
        self.assets.backgrounds.len()
    }

    /// The round's backgrounds past the game's own (handles from [`Content::own_backgrounds`] on): the pool's
    /// that the game's asset table hasn't, each its key, in the pool's order.
    fn other_backgrounds(&self) -> impl Iterator<Item = &str> {
        let game = self.game();
        self.background_pool.keys().filter(move |k| {
            let (g, name) = split(k);
            !(g == game && self.assets.handle(AssetKind::Background, name).is_some())
        })
    }

    /// The handle of game `game`'s background `name`: the game's own by its asset table, another's by the pool;
    /// none where neither has it.
    pub fn background_handle(&self, game: &str, name: &str) -> Option<BackgroundId> {
        if game == self.game()
            && let Some(h) = self.assets.handle(AssetKind::Background, name)
        {
            return Some(BackgroundId(h));
        }
        let own = self.own_backgrounds();
        self.other_backgrounds().position(|k| split(k) == (game, name)).map(|k| BackgroundId((own + k) as u16))
    }

    /// Whether `id` is one of the game's own backgrounds (its asset table's, drawn from its graphics by its name
    /// there).
    pub fn is_own_background(&self, id: BackgroundId) -> bool {
        (id.0 as usize) < self.own_backgrounds()
    }

    /// The pool's key (`<game>:<name>`) of background `id` past the game's own; none for one of the game's own
    /// and past the pool.
    pub fn other_background(&self, id: BackgroundId) -> Option<&str> {
        let k = (id.0 as usize).checked_sub(self.own_backgrounds())?;
        self.other_backgrounds().nth(k)
    }

    /// Background `id`'s game and its name in its game: one of the game's own (its asset table's: the
    /// content's game, the asset's name), or the pool's; none past them.
    pub fn background_of(&self, id: BackgroundId) -> Option<(&str, &str)> {
        if self.is_own_background(id) {
            return Some((self.game(), self.assets.name(AssetKind::Background, id.0)?));
        }
        self.other_background(id).map(split)
    }

    /// The pool's backgrounds by their handles, in the pool's order: what a round that picks its background
    /// from the pool picks from. (One of the game's own that its asset table has is at its own handle.)
    pub fn pool_backgrounds(&self) -> Vec<BackgroundId> {
        self.background_pool.iter().filter_map(|(game, name)| self.background_handle(game, name)).collect()
    }

    /// The pool's backgrounds of the content's own game alone, in the pool's order: what a round that picks its
    /// background from its game's part of the pool picks from.
    pub fn game_pool_backgrounds(&self) -> Vec<BackgroundId> {
        let own = self.game();
        self.background_pool.iter().filter(|(game, _)| *game == own).filter_map(|(game, name)| self.background_handle(game, name)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pool's order is the games' and then their names', each once; its identity is its backgrounds'.
    #[test]
    fn the_pool_is_in_its_games_order() {
        let pair = |g: &str, n: &str| (g.to_string(), n.to_string());
        let pool = BackgroundPool::new([pair("exe6", "comp"), pair("exe4", "undernet"), pair("exe4", "acdc-area"), pair("exe6", "comp")]);
        assert_eq!(pool.keys().collect::<Vec<_>>(), ["exe4:acdc-area", "exe4:undernet", "exe6:comp"]);
        assert_eq!(pool.iter().next(), Some(("exe4", "acdc-area")));
        let other = BackgroundPool::new([pair("exe4", "acdc-area"), pair("exe6", "comp")]);
        assert_ne!(pool.identity(), other.identity());
        assert_eq!(pool.identity(), BackgroundPool::new(pool.iter().map(|(g, n)| pair(g, n))).identity());
    }

    /// The game's own backgrounds keep their handles, the pool's others come after them, and the content's hash
    /// is the same whatever pool stands beside it.
    #[test]
    fn the_games_own_keep_their_handles() {
        let mut c = crate::content::testing::build();
        let game = c.game().to_string();
        let own: Vec<String> = c.assets.backgrounds.clone();
        assert!(!own.is_empty());
        let hash = c.hash();
        let before: Vec<Option<BackgroundId>> = own.iter().map(|n| c.background_handle(&game, n)).collect();
        let pair = |g: &str, n: &str| (g.to_string(), n.to_string());
        // (Games before the test content's and after it.)
        assert!(game.as_str() > "aaa" && game.as_str() < "zzz", "{game}");
        c.background_pool = BackgroundPool::new([pair("zzz", "far-area"), pair(&game, &own[0]), pair("aaa", "near-area")]);
        assert_eq!(c.hash(), hash, "the pool is presentation");
        assert_eq!(own.iter().map(|n| c.background_handle(&game, n)).collect::<Vec<_>>(), before, "the own keep their handles");
        let n = own.len() as u16;
        assert_eq!(c.background_handle("aaa", "near-area"), Some(BackgroundId(n)));
        assert_eq!(c.background_handle("zzz", "far-area"), Some(BackgroundId(n + 1)));
        assert_eq!(c.background_handle("zzz", "no-area"), None);
        assert_eq!(c.other_background(BackgroundId(n + 1)), Some("zzz:far-area"));
        assert_eq!(c.background_of(BackgroundId(n)), Some(("aaa", "near-area")));
        assert_eq!(c.other_background(BackgroundId(0)), None);
        assert_eq!(c.background_of(BackgroundId(n + 2)), None);
        assert!(c.is_own_background(BackgroundId(0)) && !c.is_own_background(BackgroundId(n)));
        assert_eq!(c.pool_backgrounds(), [BackgroundId(n), before[0].unwrap(), BackgroundId(n + 1)], "the pool's order: the games'");
    }
}
