//! Packs (docs/design/content-model-v2.md §4.0): the content is a set of
//! packs, each a folder of content/ with a manifest (`manifest.toml`):
//!
//! ```toml
//! id = "bn6"
//! kind = "game"          # or "support"
//! uses = ["exelib"]      # the support packs it requires from
//!
//! [definitions]          # its modules, by path in the pack
//! rules = ["rules/ruleset", "rules/roles"]
//! chips = ["chips/airshot/chip", "chips/cannon/chips"]
//! also = ["lib/instant/repair"]
//! unported = ["chips/x/chip"]
//! ```
//!
//! A game pack is what a match plays; a support pack (exelib) is shared
//! behavior game packs use, with no definitions of what a game lists. A
//! pack requires only its own modules and those of the support packs its
//! manifest uses; a support pack uses only support packs, without cycles
//! ([`load_order`], [`check_require`]). A load reads the manifests of its
//! games and their uses, loads each listed module (but the unported) and
//! what those require, nothing else; the define phase holds each game
//! pack's lists to the whole truth.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::keys;

/// A pack's manifest, in its folder.
pub const MANIFEST: &str = "manifest.toml";

/// The engine's API declarations' folder in content/ (no pack), whose
/// declarations every pack's modules check against first.
pub const ENGINE: &str = "nettai";

/// What a pack is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackKind {
    /// A game: what a match plays (BN6, BN5).
    #[default]
    Game,
    /// Behavior games share (exelib), defining nothing a game lists.
    Support,
}

/// A pack's manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    /// Its name, its folder's (`bn6`).
    pub id: String,
    pub kind: PackKind,
    /// The support packs it requires modules of, by name.
    #[serde(default)]
    pub uses: Vec<String>,
    #[serde(default)]
    pub definitions: PackDefinitions,
}

/// A pack's modules that define what a game lists (docs/design/
/// content-model-v2.md §4.0), each by its path in the pack, without
/// `.luau`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackDefinitions {
    /// Its stock ruleset (and with it its systems), its roles and its rule
    /// sections.
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub chips: Vec<String>,
    #[serde(default)]
    pub navis: Vec<String>,
    #[serde(default)]
    pub forms: Vec<String>,
    #[serde(default)]
    pub stages: Vec<String>,
    #[serde(default)]
    pub patch_cards: Vec<String>,
    /// Its NaviCust programs.
    #[serde(default)]
    pub navicust: Vec<String>,
    /// Modules loaded for what they define though nothing listed requires
    /// them (the alias routines, kinds and effects the original's tables
    /// number, which compat names).
    #[serde(default)]
    pub also: Vec<String>,
    /// Chips defined without a use yet: not loaded.
    #[serde(default)]
    pub unported: Vec<String>,
}

impl PackDefinitions {
    /// Its lists of what a game lists, by name, in [`crate::GAME_LISTS`]'
    /// order.
    pub fn lists(&self) -> [(&'static str, &[String]); 7] {
        [
            ("rules", &self.rules),
            ("chips", &self.chips),
            ("navis", &self.navis),
            ("forms", &self.forms),
            ("stages", &self.stages),
            ("patch_cards", &self.patch_cards),
            ("navicust", &self.navicust),
        ]
    }

    /// List `name` (one of [`Self::lists`]', or `also`), to fill.
    pub fn list_mut(&mut self, name: &str) -> Option<&mut Vec<String>> {
        Some(match name {
            "rules" => &mut self.rules,
            "chips" => &mut self.chips,
            "navis" => &mut self.navis,
            "forms" => &mut self.forms,
            "stages" => &mut self.stages,
            "patch_cards" => &mut self.patch_cards,
            "navicust" => &mut self.navicust,
            "also" => &mut self.also,
            _ => return None,
        })
    }

    /// The modules a load starts from: every list's (and `also`'s), each
    /// once, in list order; the unported aside.
    pub fn loaded(&self) -> Vec<&String> {
        let mut out: Vec<&String> = Vec::new();
        for (_, list) in self.lists() {
            for m in list {
                if !out.contains(&m) {
                    out.push(m);
                }
            }
        }
        for m in &self.also {
            if !out.contains(&m) {
                out.push(m);
            }
        }
        out
    }

    fn is_empty(&self) -> bool {
        self.lists().iter().all(|(_, l)| l.is_empty()) && self.also.is_empty() && self.unported.is_empty()
    }
}

impl PackManifest {
    /// Parse a manifest (`at` names it in messages) and check it alone: its
    /// name, its uses, a support pack's empty definitions.
    pub fn parse(text: &str, at: &str) -> Result<PackManifest, String> {
        let m: PackManifest = toml::from_str(text).map_err(|e| format!("{at}: {e}"))?;
        if !keys::valid_root_name(&m.id) {
            return Err(format!("{at}: id {:?} is no pack's name (lowercase words in -)", m.id));
        }
        for (i, u) in m.uses.iter().enumerate() {
            if u == &m.id {
                return Err(format!("{at}: {} uses itself", m.id));
            }
            if m.uses[..i].contains(u) {
                return Err(format!("{at}: uses {u} twice"));
            }
        }
        if m.kind == PackKind::Support && !m.definitions.is_empty() {
            return Err(format!(
                "{at}: support pack {} lists definitions; a support pack defines nothing a game lists (its makers take the game's ids)",
                m.id
            ));
        }
        Ok(m)
    }

    /// Module `path` of this pack, by name (`bn6:chips/cannon/chips`).
    pub fn module(&self, path: &str) -> String {
        format!("{}{}{path}", self.id, keys::SEPARATOR)
    }

    /// The modules a load of this pack starts from, by name.
    pub fn entries(&self) -> Vec<String> {
        self.definitions.loaded().into_iter().map(|p| self.module(p)).collect()
    }

    /// The modules it leaves unported, by name.
    pub fn unported(&self) -> Vec<String> {
        self.definitions.unported.iter().map(|p| self.module(p)).collect()
    }
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

/// Pack `id` and the support packs it uses, in turn, in load order: each
/// after the packs it uses, `id` last. Refused: a pack that isn't there, a
/// use of a game pack, and a cycle of uses (the chain named).
pub fn with_uses<'a>(all: &'a BTreeMap<String, PackManifest>, id: &str) -> Result<Vec<&'a PackManifest>, String> {
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
            return Err(format!("{}/{MANIFEST}: its uses make a cycle: {}", chain[0], chain.join(" uses ")));
        }
        let Some(p) = all.get(id) else {
            return Err(match user {
                Some(u) => format!("{u}/{MANIFEST}: uses {id}, which is no pack"),
                None => format!("no pack {id}"),
            });
        };
        if let Some(u) = &user
            && p.kind == PackKind::Game
        {
            return Err(format!("{u}/{MANIFEST}: uses {id}, a game pack; a pack uses only support packs"));
        }
        chain.push(id.to_string());
        for u in &p.uses {
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
/// packs they use ([`with_uses`]), then the games, in order. A support pack
/// among `games` is refused.
pub fn load_order<'a>(all: &'a BTreeMap<String, PackManifest>, games: &[String]) -> Result<Vec<&'a PackManifest>, String> {
    let mut supports: Vec<&PackManifest> = Vec::new();
    let mut out: Vec<&PackManifest> = Vec::new();
    for g in games {
        if all.get(g).is_some_and(|p| p.kind == PackKind::Support) {
            return Err(format!("{g}/{MANIFEST}: {g} is a support pack, not a game"));
        }
        for p in with_uses(all, g)? {
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
/// support packs it uses; no pack requires a game pack's but its own, and a
/// support pack requires no game's. The refusal names the module, the
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
            "{at}: {target} is a game pack, which no other pack requires ({own} requires only itself and the support packs it uses)"
        )),
        Some(_) if !p.uses.iter().any(|u| u == target) => {
            Err(format!("{at}: {own} doesn't use {target} (its manifest's `uses`: {})", p.uses.join(", ")))
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
/// (nettai/), then each support pack it uses (in load order), then its own;
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
    for q in with_uses(all, id)? {
        walk(dir, &dir.join(&q.id), &mut out)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(id: &str, kind: PackKind, uses: &[&str]) -> (String, PackManifest) {
        (id.into(), PackManifest { id: id.into(), kind, uses: uses.iter().map(|u| u.to_string()).collect(), ..Default::default() })
    }

    #[test]
    fn a_manifest_reads() {
        let m = PackManifest::parse(
            "id = \"bn6\"\nkind = \"game\"\nuses = [\"exelib\"]\n\n[definitions]\nrules = [\"rules/ruleset\"]\nchips = [\"chips/cannon/chips\", \"chips/airshot/chip\"]\nalso = [\"lib/x\", \"chips/cannon/chips\"]\nunported = [\"chips/y/chip\"]\n",
            "bn6/manifest.toml",
        )
        .unwrap();
        assert_eq!((m.kind, m.uses.as_slice()), (PackKind::Game, ["exelib".to_string()].as_slice()));
        assert_eq!(m.entries(), ["bn6:rules/ruleset", "bn6:chips/cannon/chips", "bn6:chips/airshot/chip", "bn6:lib/x"]);
        assert_eq!(m.unported(), ["bn6:chips/y/chip"]);
        assert!(PackManifest::parse("id = \"x\"\nkind = \"support\"\n[definitions]\nchips = [\"a\"]\n", "x").unwrap_err().contains("defines nothing a game lists"));
        assert!(PackManifest::parse("id = \"x\"\nkind = \"game\"\nweapons = []\n", "x").is_err(), "no other field");
        assert!(PackManifest::parse("id = \"x\"\nkind = \"game\"\nuses = [\"x\"]\n", "x").unwrap_err().contains("uses itself"));
    }

    /// A game requires itself and the support packs it uses; a support pack
    /// itself and the support packs it uses; no pack another game's.
    #[test]
    fn a_pack_requires_itself_and_the_support_packs_it_uses() {
        let all: BTreeMap<String, PackManifest> = [
            pack("bn6", PackKind::Game, &["exelib"]),
            pack("bn5", PackKind::Game, &["exelib"]),
            pack("exelib", PackKind::Support, &["base"]),
            pack("base", PackKind::Support, &[]),
            pack("other", PackKind::Support, &[]),
        ]
        .into_iter()
        .collect();
        assert_eq!(check_require(&all, "bn6:chips/x", "./y", "bn6:chips/y"), Ok(()));
        assert_eq!(check_require(&all, "bn6:chips/x", "@exelib/y", "exelib:y"), Ok(()));
        assert_eq!(check_require(&all, "exelib:y", "@base/z", "base:z"), Ok(()));
        // Game to game.
        let e = check_require(&all, "bn6:chips/x", "@bn5/lib/y", "bn5:lib/y").unwrap_err();
        assert!(e.starts_with("bn6/chips/x.luau: require(\"@bn5/lib/y\"): bn5 is a game pack"), "{e}");
        // Support to game.
        let e = check_require(&all, "exelib:y", "@bn6/lib/z", "bn6:lib/z").unwrap_err();
        assert!(e.contains("bn6 is a game pack, which no other pack requires"), "{e}");
        // A support pack it doesn't use (the user's own uses don't count).
        assert!(check_require(&all, "bn6:chips/x", "@other/y", "other:y").unwrap_err().contains("bn6 doesn't use other"));
        assert!(check_require(&all, "bn6:chips/x", "@base/y", "base:y").unwrap_err().contains("bn6 doesn't use base"));
        // The load order: support packs first, each after what it uses.
        let ids = |v: Vec<&PackManifest>| v.into_iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(load_order(&all, &["bn6".into()]).unwrap()), ["base", "exelib", "bn6"]);
        // A game's use of a game, a load of a support pack as a game.
        let mut bad = all.clone();
        bad.insert("bn6".into(), pack("bn6", PackKind::Game, &["bn5"]).1);
        assert!(load_order(&bad, &["bn6".into()]).unwrap_err().contains("uses bn5, a game pack"));
        assert!(load_order(&all, &["exelib".into()]).unwrap_err().contains("a support pack, not a game"));
        // A support pack's use of a game.
        let mut bad = all.clone();
        bad.insert("base".into(), pack("base", PackKind::Support, &["bn5"]).1);
        assert!(load_order(&bad, &["bn6".into()]).unwrap_err().contains("base/manifest.toml: uses bn5, a game pack"));
        // A cycle of support packs, the chain named.
        let mut bad = all.clone();
        bad.insert("base".into(), pack("base", PackKind::Support, &["exelib"]).1);
        let e = load_order(&bad, &["bn6".into()]).unwrap_err();
        assert!(e.contains("its uses make a cycle: bn6 uses exelib uses base uses exelib"), "{e}");
    }
}
