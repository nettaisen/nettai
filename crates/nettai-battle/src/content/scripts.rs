//! The content's scripts: its roots' Luau modules, which live next to what
//! they define (`chips/minibomb/chip.luau`, `chips/rockcube/rock.luau`,
//! `navis/megaman/weapons/absorb/weapon.luau`, `lib/...`).
//!
//! Content comes from roots (docs/design/rules-in-luau.md §7.2): a
//! directory of modules with a manifest ([`RootManifest`]) whose `name` is
//! its namespace. A module is named by its root and its path in the root
//! (`bn6:chips/minibomb/chip`), and every key it defines is qualified with
//! its root (`bn6:minibomb`).
//!
//! [`Content::define`](super::Content::define) turns what the modules
//! define into what the script runtime binds (`content::defs`). Nothing in
//! the engine says which kind, action or hook is a script: whatever the
//! content defines runs as content, and the engine's own Rust runs the rest.

use std::collections::BTreeMap;

use nettai_content_api::keys;

/// A content root's manifest (its `root.toml`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootManifest {
    /// Its namespace: the keys its modules define are qualified with it. A
    /// game root's is its game (`bn6`).
    pub name: String,
    /// The pack its asset names resolve in, by default its name (a game
    /// root's assets are its game's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<String>,
    /// The roots whose modules its modules may `require`
    /// (`require("@bn6/rules/cross/system")`). A game root requires none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
}

impl RootManifest {
    /// A root named `name` that requires nothing, with its own assets.
    pub fn named(name: &str) -> RootManifest {
        RootManifest { name: name.to_string(), assets: None, requires: Vec::new() }
    }

    /// The pack its asset names resolve in.
    pub fn assets(&self) -> &str {
        self.assets.as_deref().unwrap_or(&self.name)
    }

    /// What is wrong with it, if anything.
    pub fn check(&self) -> Result<(), String> {
        if !keys::valid_root_name(&self.name) {
            return Err(format!(
                "root name {:?} is not lowercase words in - (and not \"engine\")",
                self.name
            ));
        }
        if let Some(r) = self.requires.iter().find(|r| **r == self.name) {
            return Err(format!("root {} requires itself ({r})", self.name));
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

    /// The content's own root's name (None for content without scripts).
    pub fn home(&self) -> Option<&str> {
        self.roots.first().map(|r| r.name.as_str())
    }

    /// Module `path` of the content's own root, to change (tests and
    /// tools).
    pub fn home_module_mut(&mut self, path: &str) -> Option<&mut String> {
        let name = Scripts::name(self.home()?, path);
        self.modules.get_mut(&name)
    }

    /// What is wrong with the roots: a manifest's, a root named twice, a
    /// root required that isn't here, a module of no root.
    pub fn check_roots(&self) -> Result<(), String> {
        for (i, r) in self.roots.iter().enumerate() {
            r.check()?;
            if self.roots[..i].iter().any(|o| o.name == r.name) {
                return Err(format!("two roots are named {}", r.name));
            }
        }
        for r in &self.roots {
            if let Some(missing) = r.requires.iter().find(|q| !self.roots.iter().any(|o| &o.name == *q)) {
                return Err(format!("root {} requires root {missing}, which isn't loaded", r.name));
            }
        }
        if let Some(m) = self.modules.keys().find(|m| !keys::root_of(m).is_some_and(|r| self.roots.iter().any(|o| o.name == r))) {
            return Err(format!("module {m} is in no loaded root"));
        }
        Ok(())
    }

    /// The modules as a runtime loads them, with the bytecode compiled
    /// from them.
    pub fn pack(&self) -> nettai_luau::Pack {
        let modules = self.modules.iter().map(|(k, v)| (k.clone(), v.clone()));
        let mut pack = nettai_luau::Pack::new(modules).with_compiled(self.compiled.0.clone());
        for r in &self.roots {
            pack = pack.with_requires(&r.name, r.requires.clone());
        }
        pack
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

    fn root(name: &str, requires: &[&str], modules: &[(&str, &str)]) -> Root {
        let manifest = RootManifest { name: name.into(), assets: None, requires: requires.iter().map(|r| r.to_string()).collect() };
        (manifest, modules.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect())
    }

    /// Content of `roots`, the first its own, defined.
    fn content(roots: Vec<Root>) -> Result<Content, String> {
        let mut c = Content::default();
        for (m, modules) in roots {
            c.scripts.add_root(m, modules);
        }
        c.define().map_err(|e| e.message)?;
        Ok(c)
    }

    const GAME: &[(&str, &str)] = &[
        ("rules/turns", "return define.system { id = 'turns', state = { n = 'u8' } }"),
        ("rules/ruleset", "return define.ruleset { id = 'game', stock = true, systems = { require('./turns') } }"),
        ("cards", "return define.record('card', { power = 1 })"),
        ("chips/cannon", "return define.record('chip-ish', { power = 3 })"),
    ];

    /// docs/design/rules-in-luau.md §7.2: two roots load together, each
    /// one's keys qualified with its name; a root names another's modules
    /// only if it requires it.
    #[test]
    fn roots_load_together_under_their_names() {
        let mix: &[(&str, &str)] = &[
            // Its own ruleset, of the other root's system and its own.
            ("rules/extra", "return define.system { id = 'extra' }"),
            (
                "rules/ruleset",
                "return define.ruleset { id = 'mix', stock = true, systems = { require('@game/rules/turns'), require('./extra') } }",
            ),
            ("cards", "return define.record('card', { power = 2 })"),
        ];
        let c = content(vec![root("mix", &["game"], mix), root("game", &[], GAME)]).unwrap();
        let d = &c.defs;
        assert_eq!(d.roots, ["mix", "game"]);
        let keys: Vec<&str> = d.systems.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["game:turns", "mix:extra"]);
        let mix_rules = d.ruleset_by_key("mix:mix").unwrap();
        assert_eq!(d.ruleset(mix_rules).systems.len(), 2);
        // Each root's stock rules; the content's own root's are its stock.
        assert_eq!(d.stock_ruleset(), Some(mix_rules));
        assert_eq!(d.stock_ruleset_of("game"), d.ruleset_by_key("game"));
        // An unqualified key finds the one root's; one two roots define
        // must be qualified.
        assert_eq!(d.ruleset_by_key("mix"), Some(mix_rules));
        assert!(d.record("game:cards#1").is_some() && d.record("mix:cards#1").is_some());
        assert_eq!(d.record("cards#1"), None, "two roots define it");
        assert_eq!(d.record("chips/cannon#1"), d.record("game:chips/cannon#1"));
    }

    #[test]
    fn a_root_reaches_only_the_roots_it_requires() {
        let mix: &[(&str, &str)] = &[("rules/ruleset", "return define.ruleset { id = 'mix', systems = { require('@game/rules/turns') } }")];
        let e = content(vec![root("mix", &[], mix), root("game", &[], GAME)]).unwrap_err();
        assert!(e.contains("root mix doesn't require root game"), "{e}");
        let e = content(vec![root("mix", &["game"], mix)]).unwrap_err();
        assert!(e.contains("root mix requires root game, which isn't loaded"), "{e}");
        let e = content(vec![root("game", &[], GAME), root("game", &[], &[])]).unwrap_err();
        assert!(e.contains("two roots are named game"), "{e}");
        let e = content(vec![root("Game", &[], GAME)]).unwrap_err();
        assert!(e.contains("not lowercase words"), "{e}");
        // Two stock rulesets in one root.
        let two: &[(&str, &str)] = &[
            ("a", "return define.ruleset { id = 'a', stock = true }"),
            ("b", "return define.ruleset { id = 'b', stock = true }"),
        ];
        let e = content(vec![root("game", &[], two)]).unwrap_err();
        assert!(e.contains("root game has 2 stock rulesets"), "{e}");
    }
}
