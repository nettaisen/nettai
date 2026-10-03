//! Ids in full (docs/design/rules-in-luau.md, the flat namespace).
//!
//! The content is one namespace: every folder of content/ loads, and every
//! id is written in full, its game first (`bn6:minibomb`), in modules,
//! compat and locale tables alike; an asset's name likewise names its
//! pack's game (`bn6:bomb`). The engine's own entries keep their
//! `engine/...` keys, of no game.
//!
//! A module is named the same way: `bn6:chips/minibomb/chip` is
//! content/bn6/chips/minibomb/chip.luau.

/// What the engine's own entries' keys start with (`engine/player`): they
/// belong to no game.
pub const ENGINE: &str = "engine/";

/// What separates a game's name from an id or module path in it.
pub const SEPARATOR: char = ':';

/// `key` of game `game`, in full: `bn6:minibomb` (a pack's asset by its
/// name in the pack). An engine key stays as it is.
pub fn qualify(game: &str, key: &str) -> String {
    if key.starts_with(ENGINE) { key.to_string() } else { format!("{game}{SEPARATOR}{key}") }
}

/// The game an id or module name belongs to (`bn6` of `bn6:minibomb`);
/// None for an engine key or one not written in full.
pub fn root_of(key: &str) -> Option<&str> {
    key.split_once(SEPARATOR).map(|(root, _)| root)
}

/// An id's own part, after its game: `minibomb` of `bn6:minibomb` (a
/// pack's name for an asset; an engine key as it is).
pub fn local(key: &str) -> &str {
    key.split_once(SEPARATOR).map_or(key, |(_, k)| k)
}

/// Whether `key` is written in full (or is the engine's).
pub fn is_qualified(key: &str) -> bool {
    key.contains(SEPARATOR) || key.starts_with(ENGINE)
}

/// Whether `name` is a valid game (folder) name: lowercase ASCII letters
/// and digits in `-`-separated words (`bn6`, `bn6-souls`), and not
/// `engine`.
pub fn valid_root_name(name: &str) -> bool {
    name != "engine"
        && !name.is_empty()
        && name.split('-').all(|w| !w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_qualify_by_root() {
        assert_eq!(qualify("bn6", "minibomb"), "bn6:minibomb");
        assert_eq!(qualify("bn6", "engine/player"), "engine/player");
        assert_eq!(root_of("bn6:minibomb/action"), Some("bn6"));
        assert_eq!(root_of("engine/player"), None);
        assert_eq!(local("bn5:cannon"), "cannon");
        assert_eq!(local("cannon"), "cannon");
        assert!(valid_root_name("bn6-souls"));
        assert!(!valid_root_name("engine"));
        assert!(!valid_root_name("BN6"));
    }
}
