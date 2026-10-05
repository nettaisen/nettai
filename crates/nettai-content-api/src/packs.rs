//! Packs (docs/design/content-model-v2.md §4.0): the content is a set of
//! packs, each a folder of content/ with a manifest (`manifest.toml`) that
//! says what it is:
//!
//! ```toml
//! id = "exe6"
//! kind = "game"          # or "support"
//! depends = ["exelib"]   # the support packs it requires from
//! ```
//!
//! A game pack is what a match plays; a support pack (exelib) is shared
//! behavior game packs depend on, with no definitions of what a game has.
//! A pack requires only its own modules and those of the support packs its
//! manifest depends on; a support pack depends only on support packs,
//! without cycles ([`load_order`], [`check_require`]).
//!
//! What a game has is what its top module requires: `<game>/init.luau`
//! ([`INIT`], the pack as a module, as a folder's init.luau is the folder).
//! It returns nothing: it requires the game's rules and its folders, and
//! each folder's init.luau requires the folder's modules that define the
//! game's chips, navis, forms, stages, patch cards and NaviCust programs,
//! and those that define what only an id names. A definition is made as
//! its module loads:
//!
//! ```luau
//! -- exe6/init.luau
//! require("@self/rules")
//! require("@self/chips")
//! require("@self/navis")
//! -- exe6/chips/init.luau
//! require("@self/airshot")
//! require("@self/cannon")
//! ```
//!
//! A load of a game runs its top module, and what that requires, in turn,
//! is what loads (a support pack has no such module: its modules load when
//! a game requires them). The define phase holds the inits to the whole
//! truth: a definition of what a game has is made by a module its folder's
//! init.luau requires itself, a folder the game's init.luau requires itself
//! ([`listed_by`], [`required_by`]).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::keys;

/// A pack's manifest, in its folder.
pub const MANIFEST: &str = "manifest.toml";

/// A game pack's top module, by path in the pack: `<game>/init.luau`, which
/// requires what the game has.
pub const INIT: &str = "init";

/// The engine's API declarations' folder in content/ (no pack), whose
/// declarations every pack's modules check against first.
pub const ENGINE: &str = "nettai";

/// What a pack is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackKind {
    /// A game: what a match plays (EXE6, EXE5).
    #[default]
    Game,
    /// Behavior games share (exelib), defining nothing a game has.
    Support,
}

/// A pack's manifest: what the pack is, and nothing of what it holds (a
/// game's init.luau requires that).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    /// Its name, its folder's (`exe6`).
    pub id: String,
    pub kind: PackKind,
    /// The support packs it requires modules of, by name.
    #[serde(default)]
    pub depends: Vec<String>,
}

impl PackManifest {
    /// Parse a manifest (`at` names it in messages) and check it alone: its
    /// name and what it depends on.
    pub fn parse(text: &str, at: &str) -> Result<PackManifest, String> {
        let m: PackManifest = toml::from_str(text).map_err(|e| format!("{at}: {e}"))?;
        if !keys::valid_root_name(&m.id) {
            return Err(format!("{at}: id {:?} is no pack's name (lowercase words in -)", m.id));
        }
        for (i, u) in m.depends.iter().enumerate() {
            if u == &m.id {
                return Err(format!("{at}: {} depends on itself", m.id));
            }
            if m.depends[..i].contains(u) {
                return Err(format!("{at}: depends on {u} twice"));
            }
        }
        Ok(m)
    }

    /// Module `path` of this pack, by name (`exe6:chips/cannon/init`).
    pub fn module(&self, path: &str) -> String {
        format!("{}{}{path}", self.id, keys::SEPARATOR)
    }

    /// The module a load of this pack starts from, by name: a game's top
    /// module (`exe6:init`); a support pack's none (its modules load when a
    /// game requires them).
    pub fn entry(&self) -> Option<String> {
        (self.kind == PackKind::Game).then(|| top_module(&self.id))
    }
}

/// Game pack `id`'s top module, by name (`exe6:init`).
pub fn top_module(id: &str) -> String {
    format!("{id}{}{INIT}", keys::SEPARATOR)
}

/// The modules module `module` (by name; source `source`) requires itself,
/// by name, in order, each once; a folder by its name (`exe6:chips/cannon`,
/// [`keys::listed_as`]). A require it can't resolve is an error naming it.
pub fn required_by(module: &str, source: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for written in requires(source) {
        let target = keys::resolve(module, &written).map_err(|e| format!("{}.luau: {e}", keys::module_path(module)))?;
        let target = keys::listed_as(&target).to_string();
        if !out.contains(&target) {
            out.push(target);
        }
    }
    Ok(out)
}

/// The module that lists game module `module` (by name), and the name it
/// requires it by: the init of the folder of the pack it is in
/// (`exe6:chips/init` requires `exe6:chips/cannon`, the module
/// `exe6:chips/cannon/init`); the game's top module for a module at the
/// pack's top and for a folder's own init (`exe6:init` requires
/// `exe6:rules`, the module `exe6:rules/init`, and `exe6:chips`). None for
/// the top module itself.
pub fn listed_by(module: &str) -> Option<(String, String)> {
    let pack = keys::root_of(module)?;
    let name = keys::listed_as(module);
    let local = keys::local(name);
    if local == INIT {
        return None;
    }
    Some(match local.split_once('/') {
        Some((folder, _)) => (format!("{pack}{}{folder}/{INIT}", keys::SEPARATOR), name.to_string()),
        None => (top_module(pack), name.to_string()),
    })
}

/// Whether module source `source` is an index: nothing but requires (and
/// comments), as a game's top module and its folders' inits are.
pub fn is_index(source: &str) -> bool {
    let code: String = source.lines().map(|l| l.split("--").next().unwrap_or("")).collect::<Vec<_>>().join("\n");
    let mut rest = code.as_str();
    let mut found = false;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            return found;
        }
        let Some(after) = rest.strip_prefix("require(") else { return false };
        let after = after.trim_start();
        let Some(quote) = after.chars().next().filter(|q| *q == '"' || *q == '\'') else { return false };
        let Some(end) = after[1..].find(quote) else { return false };
        let Some(close) = after[2 + end..].trim_start().strip_prefix(')') else { return false };
        found = true;
        rest = close;
    }
}

/// The modules an index's commented requires name (`-- require("@self/x")`:
/// a chip with no use yet, which the game doesn't load), as written.
pub fn commented_requires(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let Some(comment) = line.trim_start().strip_prefix("--") else { continue };
        let Some(rest) = comment.trim_start().strip_prefix("require(") else { continue };
        let Some(quote) = rest.chars().next().filter(|q| *q == '"' || *q == '\'') else { continue };
        if let Some(end) = rest[1..].find(quote) {
            out.push(rest[1..1 + end].to_string());
        }
    }
    out
}

/// Read pack `id`'s manifest in content `dir` (content/<id>/manifest.toml),
/// its id its folder's.
pub fn read(dir: &Path, id: &str) -> Result<PackManifest, String> {
    let at = format!("{id}/{MANIFEST}");
    let path = dir.join(id).join(MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e} (no pack {id} in {})", path.display(), dir.display()))?;
    let m = PackManifest::parse(&text, &at)?;
    if m.id != id {
        return Err(format!("{at}: id {:?} isn't its folder's name, {id}", m.id));
    }
    Ok(m)
}

/// Every pack of content `dir` (each folder with a manifest), by name.
pub fn packs(dir: &Path) -> Result<Vec<PackManifest>, String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e} (is this a content directory?)", dir.display()))?
        .flatten()
        .filter(|e| e.path().join(MANIFEST).is_file())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .collect();
    names.sort();
    names.iter().map(|n| read(dir, n)).collect()
}

/// The game packs of content `dir`, by name: the games it can play.
pub fn games(dir: &Path) -> Result<Vec<String>, String> {
    Ok(packs(dir)?.into_iter().filter(|p| p.kind == PackKind::Game).map(|p| p.id).collect())
}

/// Pack `id` and the support packs it depends on, in turn, in load order:
/// each after the packs it depends on, `id` last. Refused: a pack that
/// isn't there, a game pack depended on, and a cycle (the chain named).
pub fn with_depends<'a>(all: &'a BTreeMap<String, PackManifest>, id: &str) -> Result<Vec<&'a PackManifest>, String> {
    fn visit<'a>(
        all: &'a BTreeMap<String, PackManifest>,
        id: &str,
        chain: &mut Vec<String>,
        out: &mut Vec<&'a PackManifest>,
    ) -> Result<(), String> {
        if out.iter().any(|p| p.id == id) {
            return Ok(());
        }
        let user = chain.last().cloned();
        if chain.iter().any(|c| c == id) {
            chain.push(id.to_string());
            return Err(format!("{}/{MANIFEST}: its `depends` make a cycle: {}", chain[0], chain.join(" depends on ")));
        }
        let Some(p) = all.get(id) else {
            return Err(match user {
                Some(u) => format!("{u}/{MANIFEST}: depends on {id}, which is no pack"),
                None => format!("no pack {id}"),
            });
        };
        if let Some(u) = &user
            && p.kind == PackKind::Game
        {
            return Err(format!("{u}/{MANIFEST}: depends on {id}, a game pack; a pack depends only on support packs"));
        }
        chain.push(id.to_string());
        for u in &p.depends {
            visit(all, u, chain, out)?;
        }
        chain.pop();
        out.push(p);
        Ok(())
    }
    let mut out = Vec::new();
    visit(all, id, &mut Vec::new(), &mut out)?;
    Ok(out)
}

/// The packs a load of games `games` reads, in load order: the support
/// packs they depend on ([`with_depends`]), then the games, in order. A
/// support pack among `games` is refused.
pub fn load_order<'a>(all: &'a BTreeMap<String, PackManifest>, games: &[String]) -> Result<Vec<&'a PackManifest>, String> {
    let mut supports: Vec<&PackManifest> = Vec::new();
    let mut out: Vec<&PackManifest> = Vec::new();
    for g in games {
        if all.get(g).is_some_and(|p| p.kind == PackKind::Support) {
            return Err(format!("{g}/{MANIFEST}: {g} is a support pack, not a game"));
        }
        for p in with_depends(all, g)? {
            let list = if p.kind == PackKind::Support { &mut supports } else { &mut out };
            if !list.iter().any(|q| q.id == p.id) {
                list.push(p);
            }
        }
    }
    supports.extend(out);
    Ok(supports)
}

/// May module `from` require module `to` (both by name; `written` as the
/// require wrote it)? A pack requires its own modules and those of the
/// support packs it depends on; no pack requires a game pack's but its own,
/// and a support pack requires no game's. The refusal names the module, the
/// require and the rule.
pub fn check_require(all: &BTreeMap<String, PackManifest>, from: &str, written: &str, to: &str) -> Result<(), String> {
    let (Some(own), Some(target)) = (keys::root_of(from), keys::root_of(to)) else {
        return Err(format!("{}.luau: require({written:?}): no pack's module", keys::module_path(from)));
    };
    if own == target {
        return Ok(());
    }
    let at = format!("{}.luau: require({written:?})", keys::module_path(from));
    let Some(p) = all.get(own) else { return Err(format!("{at}: {own} is no pack")) };
    match all.get(target) {
        None => Err(format!("{at}: {target} is no pack")),
        Some(t) if t.kind == PackKind::Game => Err(format!(
            "{at}: {target} is a game pack, which no other pack requires ({own} requires only itself and the support packs it depends on)"
        )),
        Some(_) if !p.depends.iter().any(|u| u == target) => {
            Err(format!("{at}: {own} doesn't depend on {target} (its manifest's `depends`: {})", p.depends.join(", ")))
        }
        Some(_) => Ok(()),
    }
}

/// The literal paths module source `source` requires, as written (outside
/// its line comments).
pub fn requires(source: &str) -> Vec<String> {
    let code: String = source.lines().map(|l| l.split("--").next().unwrap_or("")).collect::<Vec<_>>().join("\n");
    let mut out = Vec::new();
    let mut rest = code.as_str();
    while let Some(at) = rest.find("require(") {
        rest = &rest[at + "require(".len()..];
        let Some(quote) = rest.chars().next().filter(|q| *q == '"' || *q == '\'') else { continue };
        let Some(end) = rest[1..].find(quote) else { break };
        out.push(rest[1..1 + end].to_string());
        rest = &rest[1 + end..];
    }
    out
}

/// The API declarations pack `id`'s modules check against: the engine's
/// (nettai/), then each support pack it depends on (in load order), then its own;
/// each folder's `.d.luau` files in path order. (A declaration is global
/// to the modules checked against it; Luau's .luaurc names none.)
pub fn declarations(dir: &Path, all: &BTreeMap<String, PackManifest>, id: &str) -> Result<Vec<PathBuf>, String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        let Ok(entries) = std::fs::read_dir(dir) else { return Ok(()) };
        let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(root, &path, out)?;
            } else if path.to_string_lossy().ends_with(".d.luau") {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(dir, &dir.join(ENGINE), &mut out)?;
    if out.is_empty() {
        return Err(format!("{}: the engine's declarations ({ENGINE}/core.d.luau) aren't there", dir.display()));
    }
    for q in with_depends(all, id)? {
        walk(dir, &dir.join(&q.id), &mut out)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(id: &str, kind: PackKind, depends: &[&str]) -> (String, PackManifest) {
        (id.into(), PackManifest { id: id.into(), kind, depends: depends.iter().map(|u| u.to_string()).collect() })
    }

    #[test]
    fn a_manifest_reads() {
        let m = PackManifest::parse("id = \"exe6\"\nkind = \"game\"\ndepends = [\"exelib\"]\n", "exe6/manifest.toml").unwrap();
        assert_eq!((m.kind, m.depends.as_slice()), (PackKind::Game, ["exelib".to_string()].as_slice()));
        assert_eq!(m.entry(), Some("exe6:init".to_string()));
        let m = PackManifest::parse("id = \"exelib\"\nkind = \"support\"\n", "exelib/manifest.toml").unwrap();
        assert_eq!((m.kind, m.entry()), (PackKind::Support, None));
        // What it holds isn't the manifest's.
        let e = PackManifest::parse("id = \"x\"\nkind = \"game\"\n[definitions]\nchips = [\"a\"]\n", "x/manifest.toml").unwrap_err();
        assert!(e.starts_with("x/manifest.toml: ") && e.contains("unknown field `definitions`"), "{e}");
        assert!(PackManifest::parse("id = \"x\"\nkind = \"game\"\nweapons = []\n", "x").is_err(), "no other field");
        assert!(PackManifest::parse("id = \"x\"\nkind = \"game\"\ndepends = [\"x\"]\n", "x").unwrap_err().contains("depends on itself"));
    }

    /// A module's own requires: folders by name, each once, in order; a
    /// line comment's aside. An index is nothing but requires, and its
    /// commented requires name what the game doesn't load yet.
    #[test]
    fn an_index_requires_what_its_folder_has() {
        let top = "--!strict\n-- The game.\nrequire(\"@self/rules\")\nrequire(\"@self/chips\")\n";
        assert_eq!(required_by("exe6:init", top).unwrap(), ["exe6:rules", "exe6:chips"]);
        let chips = "-- Chips.\nrequire(\"@self/cannon\")\n-- require(\"@self/later\")  -- no use yet\nrequire(\"@self/cannon/init\")\nrequire(\"@exelib/x/init\")\n";
        assert_eq!(required_by("exe6:chips/init", chips).unwrap(), ["exe6:chips/cannon", "exelib:x"]);
        assert_eq!(commented_requires(chips), ["@self/later"]);
        assert!(is_index(top) && is_index(chips));
        assert!(!is_index("local x = require(\"@self/x\")\n") && !is_index("return { require(\"@self/x\") }") && !is_index("-- nothing\n"));
        // (Beside a pack is outside it.)
        let e = required_by("exe6:init", "require(\"./exe5/chips/x\")").unwrap_err();
        assert!(e.starts_with("exe6/init.luau: ") && e.contains("leaves pack exe6"), "{e}");
        // Which init lists a module.
        let by = |m: &str| listed_by(m).map(|(i, n)| format!("{i} {n}"));
        assert_eq!(by("exe6:chips/cannon/init").as_deref(), Some("exe6:chips/init exe6:chips/cannon"));
        assert_eq!(by("exe6:navis/elecman/chip").as_deref(), Some("exe6:navis/init exe6:navis/elecman/chip"));
        assert_eq!(by("exe6:rules/init").as_deref(), Some("exe6:init exe6:rules"));
        assert_eq!(by("exe6:chips/init").as_deref(), Some("exe6:init exe6:chips"));
        assert_eq!(by("exe6:probe").as_deref(), Some("exe6:init exe6:probe"));
        assert_eq!(by("exe6:init"), None);
    }

    /// A game requires itself and the support packs it depends on; a
    /// support pack itself and the support packs it depends on; no pack
    /// another game's.
    #[test]
    fn a_pack_requires_itself_and_the_support_packs_it_depends_on() {
        let all: BTreeMap<String, PackManifest> = [
            pack("exe6", PackKind::Game, &["exelib"]),
            pack("exe5", PackKind::Game, &["exelib"]),
            pack("exelib", PackKind::Support, &["base"]),
            pack("base", PackKind::Support, &[]),
            pack("other", PackKind::Support, &[]),
        ]
        .into_iter()
        .collect();
        assert_eq!(check_require(&all, "exe6:chips/x", "./y", "exe6:chips/y"), Ok(()));
        assert_eq!(check_require(&all, "exe6:chips/x", "@exelib/y", "exelib:y"), Ok(()));
        assert_eq!(check_require(&all, "exelib:y", "@base/z", "base:z"), Ok(()));
        // Game to game.
        let e = check_require(&all, "exe6:chips/x", "@exe5/lib/y", "exe5:lib/y").unwrap_err();
        assert!(e.starts_with("exe6/chips/x.luau: require(\"@exe5/lib/y\"): exe5 is a game pack"), "{e}");
        // Support to game.
        let e = check_require(&all, "exelib:y", "@exe6/lib/z", "exe6:lib/z").unwrap_err();
        assert!(e.contains("exe6 is a game pack, which no other pack requires"), "{e}");
        // A support pack it doesn't depend on (what its own depend on doesn't
        // count).
        assert!(check_require(&all, "exe6:chips/x", "@other/y", "other:y").unwrap_err().contains("exe6 doesn't depend on other"));
        assert!(check_require(&all, "exe6:chips/x", "@base/y", "base:y").unwrap_err().contains("exe6 doesn't depend on base"));
        // The load order: support packs first, each after what it depends on.
        let ids = |v: Vec<&PackManifest>| v.into_iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(load_order(&all, &["exe6".into()]).unwrap()), ["base", "exelib", "exe6"]);
        // A game depending on a game, a load of a support pack as a game.
        let mut bad = all.clone();
        bad.insert("exe6".into(), pack("exe6", PackKind::Game, &["exe5"]).1);
        assert!(load_order(&bad, &["exe6".into()]).unwrap_err().contains("depends on exe5, a game pack"));
        assert!(load_order(&all, &["exelib".into()]).unwrap_err().contains("a support pack, not a game"));
        // A support pack depending on a game.
        let mut bad = all.clone();
        bad.insert("base".into(), pack("base", PackKind::Support, &["exe5"]).1);
        assert!(load_order(&bad, &["exe6".into()]).unwrap_err().contains("base/manifest.toml: depends on exe5, a game pack"));
        // A cycle of support packs, the chain named.
        let mut bad = all.clone();
        bad.insert("base".into(), pack("base", PackKind::Support, &["exelib"]).1);
        let e = load_order(&bad, &["exe6".into()]).unwrap_err();
        assert!(e.contains("its `depends` make a cycle: exe6 depends on exelib depends on base depends on exelib"), "{e}");
    }
}
