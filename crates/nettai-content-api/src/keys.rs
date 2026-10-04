//! Keys and module names (docs/design/content-model-v2.md §4.0).
//!
//! An id is local to its game (`minibomb`): a match plays one game, and
//! nothing needs a prefix to tell games apart (the user: "no i don't want
//! qualified ids since you can't cross between games anymore"); an asset's
//! name is its pack's (`bomb`). The engine's own entries keep their
//! `engine/...` keys.
//!
//! A module is named by its pack and its path in it:
//! `bn6:chips/minibomb/init` is content/bn6/chips/minibomb/init.luau,
//! `exelib:swords/slash` content/exelib/swords/slash.luau ([`module_name`],
//! [`module_path`]; the packs, `crate::packs`). A folder's main module is
//! its `init.luau`, and the folder's name stands for it (`bn6:chips/minibomb`,
//! `require("../minibomb")`, a game's `require("@self/chips/minibomb")`:
//! [`init_of`], [`listed_as`]), as Luau's own requires read a folder. A
//! game pack's own init.luau is the game: `bn6:init` (`packs::INIT`).

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

/// A module name's path in its pack: `chips/cannon/init` of
/// `bn6:chips/cannon/init` (a key as it is).
pub fn local(key: &str) -> &str {
    key.split_once(SEPARATOR).map_or(key, |(_, k)| k)
}

/// The module name of the script at `path` in content/ (without `.luau`):
/// its pack, then its path in the pack (`bn6/chips/cannon/init` is
/// `bn6:chips/cannon/init`).
pub fn module_name(path: &str) -> String {
    match path.split_once('/') {
        Some((first, rest)) => format!("{first}{SEPARATOR}{rest}"),
        None => path.to_string(),
    }
}

/// The module a folder's name stands for, when no module has the name
/// itself: its `init` (`bn6:rules` is `bn6:rules/init`, content/bn6/rules/
/// init.luau), as a require names it.
pub fn init_of(name: &str) -> String {
    format!("{name}/init")
}

/// How a require names a module: a folder's `init` by the folder (`rules`
/// for `rules/init`), any other by its path.
pub fn listed_as(path: &str) -> &str {
    path.strip_suffix("/init").unwrap_or(path)
}

/// The path in content/ of module `name` ([`module_name`]'s inverse).
pub fn module_path(name: &str) -> String {
    match name.split_once(SEPARATOR) {
        Some((first, rest)) => format!("{first}/{rest}"),
        None => name.to_string(),
    }
}

/// Resolve a `require` path written in module `from`
/// (`bn6:rules/beast/rush`), by Luau's own rule, so an editor resolves it
/// the same:
/// - relative to its directory in its pack (`./berserk`, `../../lib/slot`),
///   which never leaves the pack;
/// - from a folder's `init.luau` (`bn6:rules/beast/init`), which is the
///   folder as a module, relative to the folder's place: `./forms` is the
///   folder beside it (rules/forms), and a module of its own folder is
///   `@self/rush`;
/// - a pack's module from the pack's top (`@exelib/swords/slash`).
///
/// The required module's name, as written: a folder's name stands for its
/// init ([`init_of`]), which the loaders resolve. Whether `from`'s pack may
/// require it is `packs::check_require`'s.
pub fn resolve(from: &str, path: &str) -> Result<String, String> {
    let (pack, from_path) = from.split_once(SEPARATOR).ok_or_else(|| format!("module {from:?} names no pack"))?;
    let target = path.trim_end_matches(".luau");
    // (An init is its folder: `rules/beast` of `rules/beast/init`.)
    let init = from_path == "init" || from_path.ends_with("/init");
    if let Some(own) = target.strip_prefix("@self/") {
        if !init {
            return Err(format!("require({path:?}) from {from}: `@self/` is a folder's init.luau's, for the modules of its folder"));
        }
        if own.split('/').any(|s| s.is_empty() || s == "." || s == "..") {
            return Err(format!("require({path:?}): a path of the folder names its directories"));
        }
        let folder = from_path.strip_suffix("init").unwrap_or(from_path);
        return Ok(format!("{pack}{SEPARATOR}{folder}{own}"));
    }
    if let Some(rest) = target.strip_prefix('@') {
        let (to, p) = rest.split_once('/').ok_or_else(|| format!("require({path:?}): a pack's module is `@<pack>/<path>`"))?;
        if p.split('/').any(|s| s.is_empty() || s == "." || s == "..") {
            return Err(format!("require({path:?}): a path from a pack's top names its directories"));
        }
        return Ok(format!("{to}{SEPARATOR}{p}"));
    }
    if !(target.starts_with("./") || target.starts_with("../")) {
        return Err(format!("require({path:?}): content paths start with ./, ../, @self/ or @<pack>/"));
    }
    let mut parts: Vec<&str> = from_path.split('/').collect();
    parts.pop();
    if init {
        // (From the folder's place, not from inside it; a pack's top
        // module's place is content/, outside the pack.)
        if parts.pop().is_none() {
            return Err(format!("require({path:?}) from {from}: leaves pack {pack} (a pack's init.luau requires its modules as `@self/...`)"));
        }
    }
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
        assert_eq!(local("bn6:chips/cannon/init"), "chips/cannon/init");
        assert_eq!(local("cannon"), "cannon");
        assert!(valid_root_name("bn6-souls"));
        assert!(!valid_root_name("engine"));
        assert!(!valid_root_name("BN6"));
        assert_eq!(module_name("bn6/chips/cannon/init"), "bn6:chips/cannon/init");
        assert_eq!(module_path("exelib:swords/slash"), "exelib/swords/slash");
        assert_eq!(resolve("bn6:rules/beast/rush", "./berserk"), Ok("bn6:rules/beast/berserk".into()));
        assert_eq!(resolve("bn6:rules/beast/rush", "../../lib/x"), Ok("bn6:lib/x".into()));
        assert_eq!(resolve("bn6:chips/cannon/action", "@exelib/swords/slash.luau"), Ok("exelib:swords/slash".into()));
        assert!(resolve("bn6:chips/cannon/action", "../../../exelib/cannon").unwrap_err().contains("leaves pack bn6"));
        assert!(resolve("bn6:chips/cannon/init", "../../exelib/cannon").unwrap_err().contains("leaves pack bn6"));
        // A folder's init is the folder as a module (Luau's rule): `./` is
        // beside the folder, `@self/` inside it.
        assert_eq!(resolve("bn6:chips/cannon/init", "./hicannon"), Ok("bn6:chips/hicannon".into()));
        assert_eq!(resolve("bn6:chips/cannon/init", "../lib/slot"), Ok("bn6:lib/slot".into()));
        assert_eq!(resolve("bn6:chips/cannon/init", "@self/action"), Ok("bn6:chips/cannon/action".into()));
        assert_eq!(resolve("bn6:rules/init", "@self/beast"), Ok("bn6:rules/beast".into()));
        // A pack's top module is the pack: its modules are `@self/`.
        assert_eq!(resolve("bn6:init", "@self/chips/cannon"), Ok("bn6:chips/cannon".into()));
        assert!(resolve("bn6:init", "./chips/cannon").unwrap_err().contains("leaves pack bn6"));
        assert_eq!(resolve("bn6:chips/cannon/action", "../cannon"), Ok("bn6:chips/cannon".into()));
        assert!(resolve("bn6:chips/cannon/action", "@self/x").unwrap_err().contains("is a folder's init.luau's"));
        assert_eq!((init_of("bn6:chips/cannon"), listed_as("chips/cannon/init")), ("bn6:chips/cannon/init".to_string(), "chips/cannon"));
        assert!(resolve("x", "lib/x").is_err());
    }
}
