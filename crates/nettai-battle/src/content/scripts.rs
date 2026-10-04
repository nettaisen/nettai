//! The content's scripts: the Luau modules of the packs a load reads
//! (docs/design/content-model-v2.md §4.0), each named by its pack and its
//! path in it (`bn6:chips/minibomb/chip`, `exelib:swords/slash`), which
//! live next to what they define.
//!
//! Each pack has a manifest (`manifest.toml`): a game pack's lists its
//! definitions that a person picks or a ruleset names, and the support
//! packs it uses; the define phase loads each pack's listed modules and
//! what they require, nothing else, and holds a game pack's lists to the
//! whole truth. A module requires only its own pack's modules and those of
//! the support packs its pack uses. Every id a module writes is in full,
//! its game first (`bn6:minibomb`).
//!
//! [`Content::define`](super::Content::define) turns what the modules
//! define into what the script runtime binds (`content::defs`). Nothing in
//! the engine says which kind, action or hook is a script: whatever the
//! content defines runs as content, and the engine's own Rust runs the rest.

use std::collections::BTreeMap;

use nettai_content_api::{PackKind, PackManifest, keys, packs};

/// The content's Luau modules.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Scripts {
    /// Source text by module name: its pack, then its path in the pack
    /// without `.luau` (`bn6:objects/sun-beam/sun_beam`, `keys::module_name`).
    pub modules: BTreeMap<String, String>,
    /// The packs the content loads (their manifests), in load order: the
    /// support packs, then the games. None: every module loads (modules a
    /// test makes alone).
    pub packs: Vec<PackManifest>,
    /// Their bytecode, as the define phase compiled it (a runtime then
    /// skips the compiler).
    pub compiled: CompiledModules,
}

impl Scripts {
    /// The modules under content/'s directory `dir`, by path in it, without
    /// bytecode (tests and tools that hold modules in memory).
    pub fn dir(dir: &str, modules: BTreeMap<String, String>) -> Scripts {
        let mut s = Scripts::default();
        s.add_dir(dir, modules);
        s
    }

    /// Add the modules under content/'s directory `dir`, by path in it.
    pub fn add_dir(&mut self, dir: &str, modules: BTreeMap<String, String>) {
        for (path, source) in modules {
            self.modules.insert(Scripts::name(dir, &path), source);
        }
    }

    /// The game packs the content loads, in order.
    pub fn games(&self) -> Vec<String> {
        self.packs.iter().filter(|p| p.kind == PackKind::Game).map(|p| p.id.clone()).collect()
    }

    /// The manifest of pack `id`, if the content loads it.
    pub fn manifest(&self, id: &str) -> Option<&PackManifest> {
        self.packs.iter().find(|p| p.id == id)
    }

    /// Game `game`'s modules (by path in its directory), with a manifest
    /// that loads every one of them and lists the definitions it must, and
    /// uses every support pack the content holds (tests and tools that hold
    /// a game's modules in memory; content/'s packs have their own).
    pub fn add_game(&mut self, game: &str, modules: BTreeMap<String, String>) {
        let uses = self.packs.iter().filter(|p| p.kind == PackKind::Support).map(|p| p.id.clone()).collect();
        let manifest = Scripts::manifest_of(game, &modules, uses);
        self.add_dir(game, modules);
        self.set_manifest(manifest);
    }

    /// Support pack `id`'s modules (by path in its directory), with its
    /// manifest; the games the content holds use it.
    pub fn add_support(&mut self, id: &str, modules: BTreeMap<String, String>) {
        self.add_dir(id, modules);
        for p in self.packs.iter_mut().filter(|p| p.kind == PackKind::Game) {
            if !p.uses.iter().any(|u| u == id) {
                p.uses.push(id.to_string());
            }
        }
        self.set_manifest(PackManifest { id: id.to_string(), kind: PackKind::Support, ..Default::default() });
    }

    /// Pack `manifest`'s, in place of one of its name (support packs before
    /// the games).
    pub fn set_manifest(&mut self, manifest: PackManifest) {
        self.packs.retain(|p| p.id != manifest.id);
        let at = if manifest.kind == PackKind::Support {
            self.packs.iter().position(|p| p.kind == PackKind::Game).unwrap_or(self.packs.len())
        } else {
            self.packs.len()
        };
        self.packs.insert(at, manifest);
    }

    /// A manifest for game `game` whose modules are `modules` (by path in
    /// its directory), using `uses`: it lists those that define what a
    /// game lists under their lists ([`nettai_content_api::GAME_LISTS`]),
    /// and the rest under `also`, so every one loads.
    pub fn manifest_of(game: &str, modules: &BTreeMap<String, String>, uses: Vec<String>) -> PackManifest {
        let mut m = PackManifest { id: game.to_string(), kind: PackKind::Game, uses, ..Default::default() };
        let mut listed = std::collections::BTreeSet::new();
        for (list, registries) in nettai_content_api::GAME_LISTS {
            let out = m.definitions.list_mut(list).expect("a game's list");
            for (path, source) in modules {
                if registries.iter().any(|r| defines(source, r.name())) {
                    out.push(path.clone());
                    listed.insert(path.clone());
                }
            }
        }
        m.definitions.also = modules.keys().filter(|p| !listed.contains(*p)).cloned().collect();
        m
    }

    /// The module name of path `path` in content/'s directory `dir`.
    pub fn name(dir: &str, path: &str) -> String {
        format!("{dir}{}{path}", keys::SEPARATOR)
    }

    /// Module `path` of content/'s directory `dir`, to change (tests and
    /// tools).
    pub fn module_mut(&mut self, dir: &str, path: &str) -> Option<&mut String> {
        self.modules.get_mut(&Scripts::name(dir, path))
    }

    /// What is wrong with the packs: a bad manifest, a pack twice, a use of
    /// a game pack or a cycle of uses, a listed module that isn't there.
    pub fn check_packs(&self) -> Result<(), String> {
        let mut all = BTreeMap::new();
        for p in &self.packs {
            let at = format!("{}/{}", p.id, packs::MANIFEST);
            if !keys::valid_root_name(&p.id) {
                return Err(format!("{at}: pack name {:?} is not lowercase words in - (and not \"engine\")", p.id));
            }
            if all.insert(p.id.clone(), p.clone()).is_some() {
                return Err(format!("{at}: pack {} is loaded twice", p.id));
            }
        }
        packs::load_order(&all, &self.games())?;
        for p in &self.packs {
            for m in p.entries() {
                if !self.modules.contains_key(&m) {
                    return Err(format!("{}/{}: lists {}, which isn't there", p.id, packs::MANIFEST, keys::module_path(&m)));
                }
            }
        }
        Ok(())
    }

    /// The modules as a runtime loads them, with the bytecode compiled
    /// from them: the define phase starts from each pack's listed modules.
    pub fn pack(&self) -> nettai_luau::Pack {
        let modules = self.modules.iter().map(|(k, v)| (k.clone(), v.clone()));
        let entries = self.packs.iter().flat_map(|p| p.entries()).collect();
        nettai_luau::Pack::new(modules).with_entries(entries).with_packs(self.packs.iter().cloned()).with_compiled(self.compiled.0.clone())
    }
}

/// Whether module source `source` calls `define.<registry>` (outside its
/// comments).
fn defines(source: &str, registry: &str) -> bool {
    let call = format!("define.{registry}");
    source.lines().map(|l| l.split("--").next().unwrap_or("")).any(|code| {
        code.match_indices(&call).any(|(i, _)| {
            let next = code[i + call.len()..].chars().next();
            !next.is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
    })
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

    type Game = (String, BTreeMap<String, String>);

    fn folder(name: &str, modules: &[(&str, &str)]) -> Game {
        (name.to_string(), modules.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect())
    }

    /// Content of these game packs (each with a manifest that loads all its
    /// modules), defined.
    fn content(games: Vec<Game>) -> Result<Content, String> {
        let mut c = Content::default();
        for (name, modules) in games {
            c.scripts.add_game(&name, modules);
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

    /// docs/design/rules-in-luau.md, the flat namespace: game packs load
    /// together; every id is written in full, its game first.
    #[test]
    fn game_packs_load_together() {
        let mix: &[(&str, &str)] = &[
            ("rules/extra", "return define.system { id = 'mix:extra' }"),
            ("rules/ruleset", "return define.ruleset { id = 'mix:stock', stock = true, systems = { require('./extra') } }"),
            ("cards", "return define.record('card', { power = 2 })"),
        ];
        let c = content(vec![folder("mix", mix), folder("game", GAME)]).unwrap();
        let d = &c.defs;
        assert_eq!(d.roots, ["game", "mix"], "the games, by name");
        let keys: Vec<&str> = d.systems.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["game:turns", "mix:extra"]);
        let mix_rules = d.ruleset_by_key("mix:stock").unwrap();
        assert_eq!(d.ruleset(mix_rules).systems.len(), 1);
        assert_eq!(d.stock_ruleset_of("mix"), Some(mix_rules));
        assert_eq!(d.stock_ruleset_of("game"), d.ruleset_by_key("game:stock"));
        // Lookups are exact.
        assert_eq!(d.ruleset_by_key("test:stock"), None);
        assert!(d.record("game:cards#1").is_some() && d.record("mix:cards#1").is_some());
        assert_eq!(d.record("test:cards#1"), None);
        // A definition's game is its id's prefix.
        assert_eq!(d.root_of("mix:extra"), d.root_id("mix"));
        assert_eq!(d.root_of("engine/player"), None);
        // One game pack requires nothing of another's.
        let e = content(vec![
            folder("mix", &[("rules/ruleset", "return define.ruleset { id = 'mix:stock', stock = true, systems = { require('@game/rules/turns') } }")]),
            folder("game", GAME),
        ])
        .unwrap_err();
        assert!(e.contains("mix/rules/ruleset.luau: require(\"@game/rules/turns\"): game is a game pack"), "{e}");
    }

    #[test]
    fn what_the_namespace_refuses() {
        let e = content(vec![folder("game", &[("rules/turns", "return define.system { id = 'turns' }")])]).unwrap_err();
        assert!(e.contains("\"turns\" names no game: write it in full (\"game:turns\")"), "{e}");
        let e = content(vec![folder("game", &[("rules/x", "return define.rules('pools', { actor = 16 })")])]).unwrap_err();
        assert!(e.contains("section name \"pools\" names no game"), "{e}");
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
    /// it names one. (Its modules are the base's pack's: a game pack
    /// requires nothing of another's.)
    #[test]
    fn a_mix_changes_its_base() {
        let mix: &[(&str, &str)] = &[
            ("rules/extra", "return define.system { id = 'mix:extra' }"),
            (
                "rules/mixes",
                "local game = require('./ruleset')\n\
                 local turns = require('./turns')\n\
                 local extra = require('./extra')\n\
                 return {\n\
                   define.ruleset { id = 'mix:plus', base = game, add = { extra } },\n\
                   define.ruleset { id = 'mix:minus', base = game, remove = { turns } },\n\
                 }",
            ),
        ];
        let with = |more: &[(&str, &str)]| -> Result<Content, String> {
            let mut modules = GAME.to_vec();
            modules.extend_from_slice(more);
            content(vec![folder("game", &modules)])
        };
        let c = with(mix).unwrap();
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
        let bad = |source: &str| -> String { with(&[("rules/bad", source)]).unwrap_err() };
        let base = "local game = require('./ruleset')\nlocal turns = require('./turns')\n";
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
        let e = bad("return define.ruleset { id = 'mix:x', base = require('./ruleset'), remove = { define.system { id = 'mix:y' } } }");
        assert!(e.contains("which its base doesn't have"), "{e}");
        // A mix that names its game is that game's.
        let named = with(&[("rules/ok", &format!("{base}return define.ruleset {{ id = 'mix:x', systems = {{ turns }}, game = 'game' }}"))])
            .unwrap()
            .defs;
        assert_eq!(named.ruleset(named.ruleset_by_key("mix:x").unwrap()).game, named.root_id("game").unwrap());
    }

    /// docs/design/content-model-v2.md §4.0: a game pack's manifest is the
    /// whole truth about its definitions of what a game lists. The modules
    /// it doesn't reach don't load; a definition it doesn't list, one under
    /// the wrong list, a listed module that isn't there, a support pack's
    /// definition of what a game lists: refused.
    #[test]
    fn a_game_packs_manifest_lists_its_definitions() {
        let modules: BTreeMap<String, String> = [
            ("rules/turns", "return define.system { id = 'game:turns' }"),
            ("rules/ruleset", "return define.ruleset { id = 'game:stock', stock = true, systems = { require('./turns') } }"),
            ("chips/cannon", "return define.record('card', { power = 3 })"),
            ("never", "error('a module no manifest reaches never loads')"),
        ]
        .into_iter()
        .map(|(p, s)| (p.to_string(), s.to_string()))
        .collect();
        let with_manifest = |definitions: &str| -> Result<Content, String> {
            let mut c = Content::default();
            c.scripts.add_dir("game", modules.clone());
            let m = PackManifest::parse(&format!("id = \"game\"\nkind = \"game\"\n[definitions]\n{definitions}"), "game/manifest.toml")?;
            c.scripts.set_manifest(m);
            c.define().map_err(|e| e.message)?;
            Ok(c)
        };
        let c = with_manifest("rules = [\"rules/ruleset\"]").unwrap();
        assert!(c.defs.ruleset_by_key("game:stock").is_some());
        assert_eq!(c.defs.record("game:chips/cannon#1"), None, "unreached, unloaded");
        // `also`: a module loaded for what it defines, which nothing listed requires.
        let c = with_manifest("rules = [\"rules/ruleset\"]\nalso = [\"chips/cannon\"]").unwrap();
        assert!(c.defs.record("game:chips/cannon#1").is_some());
        let refused = |definitions: &str, said: &str| {
            let e = with_manifest(definitions).expect_err(said);
            assert!(e.contains(said), "{said}: {e}");
        };
        refused("also = [\"rules/ruleset\"]", "game/rules/ruleset.luau: ruleset game:stock is game's, and game/manifest.toml doesn't list rules/ruleset in `rules`");
        refused("chips = [\"rules/ruleset\"]", "game/manifest.toml: `chips` lists rules/ruleset, which defines no chip");
        refused("rules = [\"rules/ruleset\", \"gone\"]", "game/manifest.toml: lists game/gone, which isn't there");
        // A support pack defines nothing a game lists.
        let mut c = Content::default();
        c.scripts.add_support("lib", [("x".to_string(), "return define.ruleset { id = 'lib:r' }".to_string())].into());
        c.scripts.add_game("game", [("rules/ruleset".to_string(), "local _ = require('@lib/x')\nreturn define.ruleset { id = 'game:stock', stock = true }".to_string())].into());
        let e = c.define().unwrap_err().message;
        assert!(e.contains("lib/x.luau: ruleset lib:r: support pack lib defines nothing a game lists"), "{e}");
    }
}
