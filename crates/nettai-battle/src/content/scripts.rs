//! The content's scripts: its folders' Luau modules, which live next to what
//! they define (`chips/minibomb/chip.luau`, `chips/rockcube/rock.luau`,
//! `navis/megaman/weapons/absorb/weapon.luau`, `lib/...`).
//!
//! Content is one namespace (docs/design/rules-in-luau.md, the flat
//! namespace): every content folder loads (content/bn6, content/bn5...), a
//! module is named by its folder and its path in it
//! (`bn6:chips/minibomb/chip`), and every id a module writes is in full,
//! its game first (`bn6:minibomb`).
//!
//! [`Content::define`](super::Content::define) turns what the modules
//! define into what the script runtime binds (`content::defs`). Nothing in
//! the engine says which kind, action or hook is a script: whatever the
//! content defines runs as content, and the engine's own Rust runs the rest.

use std::collections::BTreeMap;

use nettai_content_api::keys;

/// A content folder: its name, the part of its modules' names before the
/// path (`bn6`). A game's folder is named as the game, whose ids its
/// modules write (`bn6:...`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootManifest {
    pub name: String,
}

impl RootManifest {
    /// The folder named `name`.
    pub fn named(name: &str) -> RootManifest {
        RootManifest { name: name.to_string() }
    }

    /// The game pack a game folder's assets are in (its name).
    pub fn assets(&self) -> &str {
        &self.name
    }

    /// What is wrong with it, if anything.
    pub fn check(&self) -> Result<(), String> {
        if !keys::valid_root_name(&self.name) {
            return Err(format!("folder name {:?} is not lowercase words in - (and not \"engine\")", self.name));
        }
        Ok(())
    }
}

/// The content's Luau modules.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Scripts {
    /// Source text by module name: its root's name and its path in the
    /// root without `.luau` (`bn6:objects/sun-beam/sun_beam`).
    pub modules: BTreeMap<String, String>,
    /// The roots the modules come from: the content's own first (its
    /// home: the root the content was loaded from), then those it
    /// requires.
    pub roots: Vec<RootManifest>,
    /// Their bytecode, as the define phase compiled it (a runtime then
    /// skips the compiler).
    pub compiled: CompiledModules,
}

impl Scripts {
    /// One root's modules, by path in the root, without bytecode.
    pub fn root(manifest: RootManifest, modules: BTreeMap<String, String>) -> Scripts {
        let mut s = Scripts::default();
        s.add_root(manifest, modules);
        s
    }

    /// Add a root's modules, by path in the root.
    pub fn add_root(&mut self, manifest: RootManifest, modules: BTreeMap<String, String>) {
        for (path, source) in modules {
            self.modules.insert(Scripts::name(&manifest.name, &path), source);
        }
        self.roots.push(manifest);
    }

    /// The module name of path `path` in root `root`.
    pub fn name(root: &str, path: &str) -> String {
        format!("{root}{}{path}", keys::SEPARATOR)
    }

    /// Module `path` of folder `folder`, to change (tests and tools).
    pub fn module_mut(&mut self, folder: &str, path: &str) -> Option<&mut String> {
        self.modules.get_mut(&Scripts::name(folder, path))
    }

    /// What is wrong with the roots: a manifest's, a root named twice, a
    /// root required that isn't here, a module of no root.
    pub fn check_roots(&self) -> Result<(), String> {
        for (i, r) in self.roots.iter().enumerate() {
            r.check()?;
            if self.roots[..i].iter().any(|o| o.name == r.name) {
                return Err(format!("two folders are named {}", r.name));
            }
        }
        if let Some(m) = self.modules.keys().find(|m| !keys::root_of(m).is_some_and(|r| self.roots.iter().any(|o| o.name == r))) {
            return Err(format!("module {m} is in no loaded folder"));
        }
        Ok(())
    }

    /// The modules as a runtime loads them, with the bytecode compiled
    /// from them.
    pub fn pack(&self) -> nettai_luau::Pack {
        let modules = self.modules.iter().map(|(k, v)| (k.clone(), v.clone()));
        nettai_luau::Pack::new(modules).with_compiled(self.compiled.0.clone())
    }
}

/// Bytecode compiled from [`Scripts::modules`]: derived from them (each
/// module's with the source it was compiled from, which loading checks),
/// so content equality and the content hash leave it out.
#[derive(Clone, Default)]
pub struct CompiledModules(pub nettai_luau::Compiled);

impl std::fmt::Debug for CompiledModules {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CompiledModules({})", self.0.len())
    }
}

impl PartialEq for CompiledModules {
    fn eq(&self, _: &CompiledModules) -> bool {
        true
    }
}

impl Eq for CompiledModules {}

impl std::hash::Hash for CompiledModules {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;

    type Root = (RootManifest, BTreeMap<String, String>);

    fn folder(name: &str, modules: &[(&str, &str)]) -> Root {
        (RootManifest::named(name), modules.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect())
    }

    /// Content of `folders`, defined.
    fn content(folders: Vec<Root>) -> Result<Content, String> {
        let mut c = Content::default();
        for (m, modules) in folders {
            c.scripts.add_root(m, modules);
        }
        c.define().map_err(|e| e.message)?;
        Ok(c)
    }

    const GAME: &[(&str, &str)] = &[
        ("rules/turns", "return define.system { id = 'game:turns', state = { n = 'u8' } }"),
        ("rules/ruleset", "return define.ruleset { id = 'game:stock', stock = true, systems = { require('./turns') } }"),
        ("cards", "return define.record('card', { power = 1 })"),
        ("chips/cannon", "return define.record('chip-ish', { power = 3 })"),
    ];

    /// docs/design/rules-in-luau.md, the flat namespace: the folders load
    /// together, one namespace; every id is written in full, its game
    /// first, and a module reaches any folder's modules.
    #[test]
    fn folders_load_together_one_namespace() {
        let mix: &[(&str, &str)] = &[
            // Its own ruleset, of the other folder's system and its own.
            ("rules/extra", "return define.system { id = 'mix:extra' }"),
            (
                "rules/ruleset",
                "return define.ruleset { id = 'mix:stock', stock = true, systems = { require('@game/rules/turns'), require('./extra') } }",
            ),
            ("cards", "return define.record('card', { power = 2 })"),
        ];
        let c = content(vec![folder("mix", mix), folder("game", GAME)]).unwrap();
        let d = &c.defs;
        assert_eq!(d.roots, ["game", "mix"], "the games, by name");
        let keys: Vec<&str> = d.systems.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["game:turns", "mix:extra"]);
        let mix_rules = d.ruleset_by_key("mix:stock").unwrap();
        assert_eq!(d.ruleset(mix_rules).systems.len(), 2);
        assert_eq!(d.stock_ruleset_of("mix"), Some(mix_rules));
        assert_eq!(d.stock_ruleset_of("game"), d.ruleset_by_key("game:stock"));
        // Lookups are exact.
        assert_eq!(d.ruleset_by_key("test:stock"), None);
        assert!(d.record("game:cards#1").is_some() && d.record("mix:cards#1").is_some());
        assert_eq!(d.record("test:cards#1"), None);
        // A definition's game is its id's prefix.
        assert_eq!(d.root_of("mix:extra"), d.root_id("mix"));
        assert_eq!(d.root_of("engine/player"), None);
    }

    #[test]
    fn what_the_namespace_refuses() {
        let e = content(vec![folder("game", &[("rules/turns", "return define.system { id = 'turns' }")])]).unwrap_err();
        assert!(e.contains("\"turns\" names no game: write it in full (\"game:turns\")"), "{e}");
        let e = content(vec![folder("game", &[("rules/x", "return define.rules('pools', { actor = 16 })")])]).unwrap_err();
        assert!(e.contains("section name \"pools\" names no game"), "{e}");
        let e = content(vec![folder("game", GAME), folder("game", &[])]).unwrap_err();
        assert!(e.contains("two folders are named game"), "{e}");
        let e = content(vec![folder("Game", GAME)]).unwrap_err();
        assert!(e.contains("not lowercase words"), "{e}");
        // Two stock rulesets in one game.
        let two: &[(&str, &str)] = &[
            ("a", "return define.ruleset { id = 'game:a', stock = true }"),
            ("b", "return define.ruleset { id = 'game:b', stock = true }"),
        ];
        let e = content(vec![folder("game", two)]).unwrap_err();
        assert!(e.contains("root game has 2 stock rulesets"), "{e}");
    }

    /// docs/design/rules-in-luau.md §2.2: a mix is its base's systems,
    /// less `remove`, with `add` after them; its game is its base's unless
    /// it names one.
    #[test]
    fn a_mix_changes_its_base() {
        let mix: &[(&str, &str)] = &[
            ("rules/extra", "return define.system { id = 'mix:extra' }"),
            (
                "rules/mixes",
                "local game = require('@game/rules/ruleset')\n\
                 local turns = require('@game/rules/turns')\n\
                 local extra = require('./extra')\n\
                 return {\n\
                   define.ruleset { id = 'mix:plus', base = game, add = { extra } },\n\
                   define.ruleset { id = 'mix:minus', base = game, remove = { turns } },\n\
                 }",
            ),
        ];
        let c = content(vec![folder("mix", mix), folder("game", GAME)]).unwrap();
        let d = &c.defs;
        let systems = |key: &str| -> Vec<&str> {
            let r = d.ruleset(d.ruleset_by_key(key).unwrap());
            r.systems.iter().map(|&h| d.system(h).key.as_str()).collect()
        };
        assert_eq!(systems("mix:plus"), ["game:turns", "mix:extra"]);
        assert!(systems("mix:minus").is_empty());
        let game = d.root_id("game").unwrap();
        assert_eq!(d.ruleset(d.ruleset_by_key("mix:plus").unwrap()).game, game, "its base's game");
        // What a ruleset refuses.
        let bad = |source: &str| -> String {
            let modules: &[(&str, &str)] = &[("rules/bad", source)];
            content(vec![folder("mix", modules), folder("game", GAME)]).unwrap_err()
        };
        let base = "local game = require('@game/rules/ruleset')\nlocal turns = require('@game/rules/turns')\n";
        let cases = [
            ("return define.ruleset { id = 'mix:x', stock = true, base = game }", "has no `base`"),
            ("return define.ruleset { id = 'mix:x', base = game, systems = { turns } }", "not `systems`"),
            ("return define.ruleset { id = 'mix:x', base = game, add = { turns } }", "which it has already"),
            ("return define.ruleset { id = 'mix:x', systems = {}, remove = { turns } }", "names none"),
            ("return define.ruleset { id = 'mix:x', systems = { turns } }", "no game's"),
            ("return define.ruleset { id = 'mix:x', systems = { turns }, game = 'nowhere' }", "no loaded root"),
            ("return define.ruleset { id = 'mix:x', systems = { turns }, game = 'mix' }", "which is no game's"),
        ];
        for (source, want) in cases {
            let e = bad(&format!("{base}{source}"));
            assert!(e.contains(want), "{source}: {e}");
        }
        let e = bad(
            "return define.ruleset { id = 'mix:x', base = require('@game/rules/ruleset'), remove = { define.system { id = 'mix:y' } } }",
        );
        assert!(e.contains("which its base doesn't have"), "{e}");
        // A mix that names its game is that game's.
        let named = bad_free(&format!("{base}return define.ruleset {{ id = 'mix:x', systems = {{ turns }}, game = 'game' }}"));
        assert_eq!(named.ruleset(named.ruleset_by_key("mix:x").unwrap()).game, named.root_id("game").unwrap());
    }

    /// The definitions of a `mix` folder holding `source` beside `game`.
    fn bad_free(source: &str) -> crate::content::Defs {
        let modules: &[(&str, &str)] = &[("rules/ok", source)];
        content(vec![folder("mix", modules), folder("game", GAME)]).unwrap().defs
    }
}
