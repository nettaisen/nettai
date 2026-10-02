//! Qualified keys (docs/design/rules-in-luau.md §7.2).
//!
//! Content loads from roots: a content directory with a manifest whose
//! `name` is its namespace (a game root's is its game, `bn6`). Inside a
//! root, modules, compat and locale tables write keys unqualified, as
//! `minibomb`; the loader qualifies every definition's key with its root,
//! `bn6:minibomb`, so roots of several games load together without their
//! keys meeting. The engine's own entries keep their `engine/...` keys,
//! outside every root.
//!
//! A module is named the same way: `bn6:chips/minibomb/chip` is
//! content/bn6/chips/minibomb/chip.luau.

/// What the engine's own entries' keys start with (`engine/player`): they
/// belong to no root.
pub const ENGINE: &str = "engine/";

/// What separates a root's name from a key or module path in it.
pub const SEPARATOR: char = ':';

/// `key` of root `root`: `bn6:minibomb`. An engine key stays as it is.
pub fn qualify(root: &str, key: &str) -> String {
    if key.starts_with(ENGINE) { key.to_string() } else { format!("{root}{SEPARATOR}{key}") }
}

/// The root a qualified key or module name belongs to (`bn6` of
/// `bn6:minibomb`); None for an engine key or an unqualified one.
pub fn root_of(key: &str) -> Option<&str> {
    key.split_once(SEPARATOR).map(|(root, _)| root)
}

/// A key as its root writes it: `minibomb` of `bn6:minibomb` (an engine or
/// unqualified key as it is).
pub fn local(key: &str) -> &str {
    key.split_once(SEPARATOR).map_or(key, |(_, k)| k)
}

/// Whether `key` is qualified (or the engine's): a key a lookup takes as it
/// is, never resolving it in a root.
pub fn is_qualified(key: &str) -> bool {
    key.contains(SEPARATOR) || key.starts_with(ENGINE)
}

/// Whether a definition's (qualified) key `defined` is what `key` names:
/// the same key, or, unqualified, its key in its root.
pub fn names(defined: &str, key: &str) -> bool {
    defined == key || (!is_qualified(key) && local(defined) == key)
}

/// Whether `name` is a valid root name: lowercase ASCII letters and digits
/// in `-`-separated words (`bn6`, `bn6-souls`), and not `engine`.
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
        assert!(names("bn6:cannon", "cannon"));
        assert!(names("bn6:cannon", "bn6:cannon"));
        assert!(!names("bn6:cannon", "bn5:cannon"));
        assert!(valid_root_name("bn6-souls"));
        assert!(!valid_root_name("engine"));
        assert!(!valid_root_name("BN6"));
    }
}
