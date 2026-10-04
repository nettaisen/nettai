//! Keys and module names (docs/design/content-model-v2.md §4.0).
//!
//! An id is local to its game (`minibomb`): a match plays one game, and
//! nothing needs a prefix to tell games apart (the user: "no i don't want
//! qualified ids since you can't cross between games anymore"); an asset's
//! name is its pack's (`bomb`). The engine's own entries keep their
//! `engine/...` keys.
//!
//! A module is named by its pack and its path in it:
//! `bn6:chips/minibomb/chip` is content/bn6/chips/minibomb/chip.luau,
//! `exelib:swords/slash` content/exelib/swords/slash.luau ([`module_name`],
//! [`module_path`]; the packs, `crate::packs`).

/// What the engine's own entries' keys start with (`engine/player`): they
/// belong to no game.
pub const ENGINE: &str = "engine/";

/// What separates a game's name from an id or module path in it.
pub const SEPARATOR: char = ':';

/// The pack a module name (or a support pack's anonymous key) belongs to
/// (`exelib` of `exelib:regions#57`); None for a game's key.
pub fn root_of(key: &str) -> Option<&str> {
    key.split_once(SEPARATOR).map(|(root, _)| root)
}

/// A module name's path in its pack: `chips/cannon/chips` of
/// `bn6:chips/cannon/chips` (a key as it is).
pub fn local(key: &str) -> &str {
    key.split_once(SEPARATOR).map_or(key, |(_, k)| k)
}

/// The module name of the script at `path` in content/ (without `.luau`):
/// its pack, then its path in the pack (`bn6/chips/cannon/chips` is
/// `bn6:chips/cannon/chips`).
pub fn module_name(path: &str) -> String {
    match path.split_once('/') {
        Some((first, rest)) => format!("{first}{SEPARATOR}{rest}"),
        None => path.to_string(),
    }
}

/// The path in content/ of module `name` ([`module_name`]'s inverse).
pub fn module_path(name: &str) -> String {
    match name.split_once(SEPARATOR) {
        Some((first, rest)) => format!("{first}/{rest}"),
        None => name.to_string(),
    }
}

/// Resolve a `require` path written in module `from`
/// (`bn6:rules/beast/system`): relative to its directory in its pack
/// (`./rush`, `../lib/slot`), which never leaves the pack, or a pack's
/// module from the pack's top (`@exelib/swords/slash`). The required
/// module's name. Whether `from`'s pack may require it is
/// `packs::check_require`'s.
pub fn resolve(from: &str, path: &str) -> Result<String, String> {
    let (pack, from_path) = from.split_once(SEPARATOR).ok_or_else(|| format!("module {from:?} names no pack"))?;
    let target = path.trim_end_matches(".luau");
    if let Some(rest) = target.strip_prefix('@') {
        let (to, p) = rest.split_once('/').ok_or_else(|| format!("require({path:?}): a pack's module is `@<pack>/<path>`"))?;
        if p.split('/').any(|s| s.is_empty() || s == "." || s == "..") {
            return Err(format!("require({path:?}): a path from a pack's top names its directories"));
        }
        return Ok(format!("{to}{SEPARATOR}{p}"));
    }
    if !(target.starts_with("./") || target.starts_with("../")) {
        return Err(format!("require({path:?}): content paths start with ./, ../ or @<pack>/"));
    }
    let mut parts: Vec<&str> = from_path.split('/').collect();
    parts.pop();
    for seg in target.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop().ok_or_else(|| format!("require({path:?}) from {from} leaves pack {pack} (`@<pack>/<path>` names another)"))?;
            }
            s => parts.push(s),
        }
    }
    if parts.is_empty() {
        return Err(format!("require({path:?}) from {from} names no module"));
    }
    Ok(format!("{pack}{SEPARATOR}{}", parts.join("/")))
}

/// Whether `name` is a valid game name: lowercase ASCII letters
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
    fn keys_and_module_names() {
        assert_eq!(root_of("exelib:regions#57"), Some("exelib"));
        assert_eq!(root_of("minibomb/action"), None);
        assert_eq!(local("bn6:chips/cannon/chips"), "chips/cannon/chips");
        assert_eq!(local("cannon"), "cannon");
        assert!(valid_root_name("bn6-souls"));
        assert!(!valid_root_name("engine"));
        assert!(!valid_root_name("BN6"));
        assert_eq!(module_name("bn6/chips/cannon/chips"), "bn6:chips/cannon/chips");
        assert_eq!(module_path("exelib:swords/slash"), "exelib/swords/slash");
        assert_eq!(resolve("bn6:rules/beast/system", "./rush"), Ok("bn6:rules/beast/rush".into()));
        assert_eq!(resolve("bn6:rules/beast/system", "../../lib/x"), Ok("bn6:lib/x".into()));
        assert_eq!(resolve("bn6:chips/cannon/chips", "@exelib/swords/slash.luau"), Ok("exelib:swords/slash".into()));
        assert!(resolve("bn6:chips/cannon/chips", "../../../exelib/cannon").unwrap_err().contains("leaves pack bn6"));
        assert!(resolve("x", "lib/x").is_err());
    }
}
