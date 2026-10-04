//! The content's scripts: the Luau modules of the packs a load reads
//! (docs/design/content-model-v2.md §4.0), each named by its pack and its
//! path in it (`bn6:chips/minibomb/init`, `exelib:swords/slash`), which
//! live next to what they define.
//!
//! Each pack has a manifest (`manifest.toml`: its name, its kind and the
//! support packs it depends on), and a game pack a top module
//! (`<game>/init.luau`) that requires what the game has. The define phase
//! runs each game's top module and what it requires, nothing else, and
//! holds it to the whole truth: what a game has is defined by a module its
//! init.luau requires itself. A module requires only its own pack's modules
//! and those of the support packs its pack depends on. Every id a module
//! writes is local to its game (`minibomb`).
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
    /// that depends on every support pack the content holds; without a top
    /// module among them (`init`), one that requires every one of them
    /// ([`Scripts::init_for`]). (Tests and tools that hold a game's modules
    /// in memory; content/'s packs have their own.)
    pub fn add_game(&mut self, game: &str, mut modules: BTreeMap<String, String>) {
        let depends = self.packs.iter().filter(|p| p.kind == PackKind::Support).map(|p| p.id.clone()).collect();
        if !modules.contains_key(packs::INIT) {
            let init = Scripts::init_for(&modules);
            modules.insert(packs::INIT.to_string(), init);
        }
        self.add_dir(game, modules);
        self.set_manifest(PackManifest { id: game.to_string(), kind: PackKind::Game, depends });
    }

    /// Support pack `id`'s modules (by path in its directory), with its
    /// manifest; the games the content holds depend on it.
    pub fn add_support(&mut self, id: &str, modules: BTreeMap<String, String>) {
        self.add_dir(id, modules);
        for p in self.packs.iter_mut().filter(|p| p.kind == PackKind::Game) {
            if !p.depends.iter().any(|u| u == id) {
                p.depends.push(id.to_string());
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

    /// A top module (`init.luau`'s source) for a game whose modules are
    /// `modules` (by path in its directory): it requires every one, those
    /// that define what a game has under their groups
    /// ([`nettai_content_api::GAME_LISTS`]) and the rest under `also`, so
    /// every one loads; each group in path order, a folder by its name.
    pub fn init_for(modules: &BTreeMap<String, String>) -> String {
        let mut listed = std::collections::BTreeSet::new();
        let mut out = String::from("return {\n");
        let group = |name: &str, paths: Vec<&String>, out: &mut String| {
            if paths.is_empty() {
                return;
            }
            out.push_str(&format!("    {name} = {{\n"));
            for path in paths {
                out.push_str(&format!("        require(\"@self/{}\"),\n", keys::listed_as(path)));
            }
            out.push_str("    },\n");
        };
        for (list, registries) in nettai_content_api::GAME_LISTS {
            let paths: Vec<&String> = modules.iter().filter(|(_, source)| registries.iter().any(|r| defines(source, r.name()))).map(|(p, _)| p).collect();
            listed.extend(paths.iter().map(|p| (*p).clone()));
            group(list, paths, &mut out);
        }
        group("also", modules.keys().filter(|p| !listed.contains(*p) && p.as_str() != packs::INIT).collect(), &mut out);
        out.push_str("}\n");
        out
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

    /// What is wrong with the packs: a bad manifest, a pack twice, a game
    /// pack depended on or a cycle, a game without its top module.
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
            if let Some(m) = p.entry()
                && !self.modules.contains_key(&m)
            {
                return Err(format!(
                    "{}.luau: game pack {} has no top module: its {}.luau requires what the game has",
                    keys::module_path(&m),
                    p.id,
                    packs::INIT
                ));
            }
        }
        Ok(())
    }

    /// The modules game `game`'s top module requires itself, by name (a
    /// folder by its name): what the game has (`packs::required_by_init`).
    pub fn required_by_init(&self, game: &str) -> Result<Vec<String>, String> {
        match self.modules.get(&packs::top_module(game)) {
            Some(init) => packs::required_by_init(game, init),
            None => Err(format!("{game}/{}.luau: game pack {game} has no top module", packs::INIT)),
        }
    }

    /// The modules as a runtime loads them, with the bytecode compiled
    /// from them: the define phase starts from each game's top module.
    pub fn pack(&self) -> nettai_luau::Pack {
        let modules = self.modules.iter().map(|(k, v)| (k.clone(), v.clone()));
        let entries = self.packs.iter().filter_map(|p| p.entry()).collect();
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
        ("rules/turns", "return define.system { id = 'turns', state = { n = 'u8' } }"),
        ("rules/ruleset", "return define.ruleset { systems = { require('./turns') } }"),
        ("cards", "return define.record('card', { power = 1 })"),
        ("chips/cannon", "return define.record('chip-ish', { power = 3 })"),
    ];

    /// docs/design/content-model-v2.md §4.0: content holds one game pack
    /// (a match plays one game), every id written in full, its game first;
    /// one game pack requires nothing of another's.
    #[test]
    fn content_holds_one_game() {
        let mix: &[(&str, &str)] = &[
            ("rules/extra", "return define.system { id = 'extra' }"),
            ("rules/ruleset", "return define.ruleset { systems = { require('./extra') } }"),
            ("cards", "return define.record('card', { power = 2 })"),
        ];
        let c = content(vec![folder("game", GAME)]).unwrap();
        let d = &c.defs;
        assert_eq!((c.game(), d.game.as_str()), ("game", "game"));
        let keys: Vec<&str> = d.systems.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["turns"]);
        assert_eq!(d.ruleset().map(|r| r.systems.len()), Some(1), "the game's one ruleset");
        // Lookups are exact, of the one game's.
        assert!(d.record("cards#1").is_some());
        assert_eq!(d.record("game:cards#1"), None);
        // Two game packs are two contents.
        let e = content(vec![folder("mix", mix), folder("game", GAME)]).unwrap_err();
        assert!(e.contains("content holds one game, and these are mix and game"), "{e}");
        // One game pack requires nothing of another's.
        let mut c = Content::default();
        c.scripts.add_dir("game", GAME.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect());
        c.scripts.set_manifest(PackManifest { id: "game".into(), kind: PackKind::Game, ..Default::default() });
        c.scripts.add_game("mix", [("rules/ruleset".to_string(), "return define.ruleset { systems = { require('@game/rules/turns') } }".to_string())].into());
        c.scripts.packs.retain(|p| p.id == "mix");
        let e = c.define().unwrap_err().message;
        assert!(e.contains("mix/rules/ruleset.luau: require(\"@game/rules/turns\"): game is no pack") || e.contains("game is a game pack"), "{e}");
    }

    #[test]
    fn what_the_namespace_refuses() {
        let e = content(vec![folder("game", &[("rules/turns", "return define.system { id = 'game:turns' }")])]).unwrap_err();
        assert!(e.contains("\"game:turns\" is not a valid id"), "{e}");
        // A section is the ruleset's field by the engine's name.
        let e = content(vec![folder("game", &[("rules/x", "return define.ruleset { Pools = { actor = 16 } }")])]).unwrap_err();
        assert!(e.contains("`Pools` is no field of a ruleset"), "{e}");
        let e = content(vec![folder("game", &[("rules/x", "return define.ruleset { pools = { actor = 0, attack = 32, effect = 32 } }")])]).unwrap_err();
        assert!(e.contains("game:rules/x.luau: ruleset: pools: a pool holds 1 to"), "{e}");
        let e = content(vec![folder("game", &[("rules/x", "return define.ruleset { pools = { actor = 'many', attack = 32, effect = 32 } }")])]).unwrap_err();
        assert!(e.contains("ruleset: pools.actor: invalid type"), "{e}");
        let e = content(vec![folder("Game", GAME)]).unwrap_err();
        assert!(e.contains("not lowercase words"), "{e}");
    }

    /// The user: "there should only be one ruleset per game". A game
    /// defines one, with no name and no variants: a second, an `id`, a
    /// `stock` flag and a variant's `base`, `add` and `remove` are refused,
    /// each by what it is.
    #[test]
    fn a_game_has_one_ruleset() {
        let with = |more: &[(&str, &str)]| -> Result<Content, String> {
            let mut modules = GAME.to_vec();
            modules.retain(|(p, _)| !more.iter().any(|(q, _)| q == p));
            modules.extend_from_slice(more);
            content(vec![folder("game", &modules)])
        };
        let c = with(&[]).unwrap();
        let d = &c.defs;
        let names: Vec<&str> = d.ruleset_systems().iter().map(|&h| d.system(h).key.as_str()).collect();
        assert_eq!(names, ["turns"]);
        assert_eq!(d.definitions.of(nettai_content_api::Registry::Ruleset)[0].key, nettai_content_api::RULESET_KEY);
        // Content without one: its sides have no systems.
        let none = content(vec![folder("game", &[("cards", "return define.record('card', {})")])]).unwrap();
        assert!(none.defs.ruleset().is_none() && none.defs.ruleset_systems().is_empty());
        // A second.
        let e = with(&[("rules/other", "return define.ruleset { systems = {} }")]).unwrap_err();
        assert!(e.contains("a game has one ruleset: game/rules/other.luau defines one, and game/rules/ruleset.luau another"), "{e}");
        let bad = |source: &str| -> String { with(&[("rules/ruleset", source)]).unwrap_err() };
        let cases = [
            ("return define.ruleset { id = 'stock', systems = {} }", "define.ruleset takes no `id`: a game has one ruleset"),
            ("return define.ruleset { stock = true, systems = {} }", "`stock`: a game has one ruleset"),
            ("return define.ruleset { base = {}, systems = {} }", "`base`: a game has one ruleset, and no variants of it"),
            ("return define.ruleset { add = {} }", "`add`: a game has one ruleset, and no variants of it"),
            ("return define.ruleset { remove = {} }", "`remove`: a game has one ruleset, and no variants of it"),
            ("return define.ruleset { systems = {}, game = 'game' }", "`game` is no field of a ruleset"),
            ("local t = require('./turns')\nreturn define.ruleset { systems = { t, t } }", "`systems` lists system turns twice"),
            ("return define.ruleset { systems = { 3 } }", "`systems` lists system definitions"),
        ];
        for (source, want) in cases {
            let e = bad(source);
            assert!(e.contains(want), "{source}: {e}");
        }
    }

    /// docs/design/content-model-v2.md §4.0: a game's top module (its
    /// init.luau) is the whole truth about what the game has. The modules
    /// it doesn't reach don't load; a definition of what a game has in a
    /// module it doesn't require itself, a game without one, a support
    /// pack's definition of what a game has: refused. The order of its
    /// requires moves no key and no handle.
    #[test]
    fn a_games_init_requires_what_it_has() {
        let modules: BTreeMap<String, String> = [
            ("rules/turns", "return define.system { id = 'turns' }"),
            ("rules/init", "return define.ruleset { systems = { require('@self/turns') } }"),
            ("chips/cannon", "return define.record('card', { power = 3 })"),
            ("chips/sword/init", "return define.chip { id = 'sword', instant = function(u) end, parts = { define.record('part', {}), require('@self/edge') } }"),
            ("chips/sword/edge", "return define.record('part', { long = true })"),
            ("lib/pa", "return { chip = require('../chips/sword'), record = define.record('pa', {}) }"),
            ("never", "error('a module no init reaches never loads')"),
        ]
        .into_iter()
        .map(|(p, s)| (p.to_string(), s.to_string()))
        .collect();
        let with_init = |init: Option<&str>| -> Result<Content, String> {
            let mut c = Content::default();
            c.scripts.add_dir("game", modules.clone());
            if let Some(init) = init {
                c.scripts.modules.insert(packs::top_module("game"), init.to_string());
            }
            c.scripts.set_manifest(PackManifest::parse("id = \"game\"\nkind = \"game\"\n", "game/manifest.toml")?);
            c.define().map_err(|e| e.message)?;
            Ok(c)
        };
        let c = with_init(Some("return { rules = require('@self/rules') }")).unwrap();
        assert!(c.defs.ruleset().is_some());
        assert_eq!(c.defs.record("chips/cannon#1"), None, "unreached, unloaded");
        // `also`: a module loaded for what it defines, which nothing else requires.
        let all = "return { rules = require('@self/rules'), chips = { require('@self/chips/sword') }, also = { require('@self/chips/cannon'), require('@self/lib/pa') } }";
        let c = with_init(Some(all)).unwrap();
        assert!(c.defs.record("chips/cannon#1").is_some() && c.defs.chip_by_key("sword").is_some());
        // The order of the requires is the load order, and nothing more: the
        // same definitions under the same keys, so the same handles.
        let turned = "return { also = { require('@self/lib/pa'), require('@self/chips/cannon') }, chips = { require('@self/chips/sword') }, rules = require('@self/rules') }";
        let d = with_init(Some(turned)).unwrap();
        assert_eq!(c.defs.definitions, d.defs.definitions);
        let refused = |init: Option<&str>, said: &str| {
            let e = with_init(init).expect_err(said);
            assert!(e.contains(said), "{said}: {e}");
        };
        // Reached, but not required by the init itself.
        refused(
            Some("return { rules = require('@self/rules'), also = { require('@self/lib/pa') } }"),
            "game/chips/sword/init.luau: chip sword is game's, and game/init.luau doesn't require chips/sword",
        );
        refused(Some("return { rules = require('@self/rules'), also = { require('@self/gone') } }"), "no module game:gone.luau");
        refused(Some("return { rules = require('./rules') }"), "leaves pack game");
        refused(None, "game/init.luau: game pack game has no top module");
        // A support pack defines nothing a game has.
        let mut c = Content::default();
        c.scripts.add_support("lib", [("x".to_string(), "return define.ruleset {}".to_string())].into());
        c.scripts.add_game("game", [("chips/y".to_string(), "local _ = require('@lib/x')\nreturn define.record('y', {})".to_string())].into());
        let e = c.define().unwrap_err().message;
        assert!(e.contains("lib/x.luau: ruleset ruleset: support pack lib defines nothing a game has"), "{e}");
    }
}
