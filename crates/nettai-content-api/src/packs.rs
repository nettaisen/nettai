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
//! What a game has is what its top module returns, its root:
//! `<game>/init.luau` ([`INIT`], the pack as a module, as a folder's
//! init.luau is the folder). The root holds what a match names, by id, in
//! its sections ([`SECTIONS`], the core's: chips, navis, forms, stages) and
//! in collections of the game's own (any other key: EXE6's `patch_cards`,
//! `navicust_programs`, which the core reads as data,
//! `Registry::Entry`), and the game's rules; each section or collection is
//! the module that returns it, a folder's init that merges the tables of
//! the modules that define it. Loading has no side effect: the loader walks
//! from the root, and what it reaches is what the game has.
//!
//! ```luau
//! -- exe6/init.luau
//! return {
//!     chips = require("@self/chips"),
//!     navis = require("@self/navis"),
//!     rules = require("@self/rules"),
//! }
//! -- exe6/chips/init.luau
//! return merge {
//!     require("@self/airshot"),
//!     require("@self/cannon"),
//! }
//! -- exe6/chips/cannon/init.luau
//! local cannon: Chip = { ... }
//! return { cannon = cannon, hicannon = hicannon, ["m-cannon"] = m_cannon }
//! ```
//!
//! A load of a game runs its top module, and what that requires, in turn,
//! is what loads (a support pack has no such module: its modules load when
//! a game requires them). Nothing is read ahead: a `require` finds its
//! module when it runs ([`find`], the one place that knows how), and reads
//! it then from where the load's modules are ([`Modules`]: a content
//! directory's packs, modules held in memory, or both). A module nothing
//! requires never runs; the section inits are tools/content/index.py's,
//! written from the modules that are there ([`sections_of`]).

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

/// A pack's extraction recipe's folder (docs/design/extraction-recipes.md
/// §4.1): the Luau that says how the game's assets are extracted from its
/// ROMs, with the drawings it reads. Its modules run in a VM of their own
/// and require only modules under it (its own pack's and its support
/// packs'); no other module requires one of them (§2.4).
pub const EXTRACT: &str = "extract";

/// The engine's declarations of the recipes' API, in [`ENGINE`]: what an
/// [`EXTRACT`] module checks against, and no other module.
pub const EXTRACT_DECLARATIONS: &str = "extract.d.luau";

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
/// game's init.luau requires that; its recipe, `extract/init.luau`, says
/// what it extracts from, docs/design/extraction-recipes.md §4.2).
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

/// Where a load reads its modules from: by name (`exe6:chips/cannon/init`),
/// the text of each. A content directory's packs ([`Dirs`]), modules held
/// in memory (a map by name: a test's, a tool's stand-ins), or the first of
/// two that has it (a pair).
pub trait Modules {
    /// Module `name`'s text, if there is one.
    fn read(&self, name: &str) -> Option<String>;

    /// Whether there is a module `name`.
    fn has(&self, name: &str) -> bool {
        self.read(name).is_some()
    }
}

impl Modules for BTreeMap<String, String> {
    fn read(&self, name: &str) -> Option<String> {
        self.get(name).cloned()
    }

    fn has(&self, name: &str) -> bool {
        self.contains_key(name)
    }
}

impl<T: Modules + ?Sized> Modules for &T {
    fn read(&self, name: &str) -> Option<String> {
        (**self).read(name)
    }

    fn has(&self, name: &str) -> bool {
        (**self).has(name)
    }
}

impl<A: Modules, B: Modules> Modules for (A, B) {
    fn read(&self, name: &str) -> Option<String> {
        self.0.read(name).or_else(|| self.1.read(name))
    }

    fn has(&self, name: &str) -> bool {
        self.0.has(name) || self.1.has(name)
    }
}

/// Packs' folders on disk, by pack: module `<pack>:<path>` is the file
/// `<its folder>/<path>.luau` (content/'s packs: `exe6` in content/exe6).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dirs(pub BTreeMap<String, PathBuf>);

impl Dirs {
    /// The packs `packs` of content directory `dir`, each in its folder.
    pub fn of_content<'a>(dir: &Path, packs: impl IntoIterator<Item = &'a str>) -> Dirs {
        Dirs(packs.into_iter().map(|p| (p.to_string(), dir.join(p))).collect())
    }

    /// Module `name`'s file.
    pub fn file(&self, name: &str) -> Option<PathBuf> {
        let (pack, path) = name.split_once(keys::SEPARATOR)?;
        if path.split('/').any(|s| s.is_empty() || s == "." || s == "..") {
            return None;
        }
        Some(self.0.get(pack)?.join(format!("{path}.luau")))
    }

    /// Every module the folders hold, by name, in name order (definition
    /// files, `.d.luau`, aside): what a tool lists, never what a load
    /// reads.
    pub fn names(&self) -> Vec<String> {
        fn walk(pack: &str, root: &Path, dir: &Path, out: &mut Vec<String>) {
            let Ok(entries) = nettai_platform::fs::read_dir(dir) else { return };
            for path in entries.flatten().map(|e| e.path()) {
                if path.is_dir() {
                    walk(pack, root, &path, out);
                } else if let Some(stem) = path.to_str().and_then(|s| s.strip_suffix(".luau"))
                    && !stem.ends_with(".d")
                    && let Ok(rel) = Path::new(stem).strip_prefix(root)
                {
                    let rel = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                    out.push(format!("{pack}{}{rel}", keys::SEPARATOR));
                }
            }
        }
        let mut out = Vec::new();
        for (pack, dir) in &self.0 {
            walk(pack, dir, dir, &mut out);
        }
        out.sort();
        out
    }
}

impl Modules for Dirs {
    /// A module's source with CRLF line endings read as LF: the content's
    /// hash covers its sources byte for byte, and a file saved on Windows
    /// is the same content as one saved elsewhere.
    fn read(&self, name: &str) -> Option<String> {
        let text = nettai_platform::fs::read_to_string(self.file(name)?).ok()?;
        Some(if text.contains("\r\n") { text.replace("\r\n", "\n") } else { text })
    }

    fn has(&self, name: &str) -> bool {
        self.file(name).is_some_and(|f| nettai_platform::fs::is_file(&f))
    }
}

/// The module the name `name` stands for among `modules`: itself, or, a
/// folder's name, the folder's init (`exe6:rules` is `exe6:rules/init`,
/// rules/init.luau), as Luau reads a folder.
pub fn module(modules: &dyn Modules, name: &str) -> Option<String> {
    if modules.has(name) {
        return Some(name.to_string());
    }
    Some(keys::init_of(name)).filter(|init| modules.has(init))
}

/// The module a `require` of `written` in module `from` names, among
/// `modules`: where every require is resolved, a load's as it runs and a
/// check's of a module's text. Its path by Luau's rule ([`keys::resolve`]:
/// relative, `@self/`, `@<pack>/`), a folder's name for its init
/// ([`module`]); and whether `from`'s pack may require it
/// ([`check_require`]; `packs` empty: any, a test's modules alone). An
/// error names the requiring module and the path as written.
pub fn find(modules: &dyn Modules, packs: &BTreeMap<String, PackManifest>, from: &str, written: &str) -> Result<String, String> {
    let file = keys::module_path(from);
    let target = keys::resolve(from, written).map_err(|e| format!("{file}.luau: {e}"))?;
    if !packs.is_empty() {
        check_require(packs, from, written, &target)?;
    }
    module(modules, &target).ok_or_else(|| format!("{file}.luau: require({written:?}): no module {}.luau", keys::module_path(&target)))
}

/// A game's root's sections the core knows, each with the type its
/// definitions are written as in their modules (`local chip: Chip = { ...
/// }`): what a match names, by id. (A game's own collections are its
/// root's other keys.)
pub const SECTIONS: [(&str, &str); 4] = [("chips", "Chip"), ("navis", "NaviDef"), ("forms", "FormDef"), ("stages", "StageDef")];

/// The root's field that is the game's rules.
pub const RULES: &str = "rules";

/// The sections module source `source` gives definitions of: those a
/// top-level local of its is written as (`local chip: Chip = {`, a series'
/// `local chips: { [string]: Chip } = {`, or one declared to be one further
/// on: `local numtrap: Chip`), in [`SECTIONS`] order. tools/content/index.py
/// reads a game's modules the same way to write its section inits, and a
/// game held in memory gets its root from it (a test's).
pub fn sections_of(source: &str) -> Vec<&'static str> {
    let mut found = Vec::new();
    for line in source.lines() {
        let Some(rest) = line.strip_prefix("local ") else { continue };
        let Some((name, rest)) = rest.split_once(':') else { continue };
        if name.trim().is_empty() || !name.trim().chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let rest = rest.trim_start();
        let (ty, rest) = match rest.strip_prefix('{') {
            Some(map) => {
                let Some(map) = map.trim_start().strip_prefix("[string]") else { continue };
                let Some(map) = map.trim_start().strip_prefix(':') else { continue };
                let map = map.trim_start();
                let end = map.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(map.len());
                let Some(after) = map[end..].trim_start().strip_prefix('}') else { continue };
                (&map[..end], after)
            }
            None => {
                let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
                (&rest[..end], &rest[end..])
            }
        };
        let rest = rest.split("--").next().unwrap_or("").trim();
        let made = rest.is_empty() || rest.strip_prefix('=').is_some_and(|r| r.trim_start().starts_with('{'));
        if let Some(&(section, _)) = SECTIONS.iter().find(|(_, t)| *t == ty)
            && made
            && !found.contains(&section)
        {
            found.push(section);
        }
    }
    found.sort_by_key(|s| SECTIONS.iter().position(|(x, _)| x == s));
    found
}

/// Read pack `id`'s manifest in content `dir` (content/<id>/manifest.toml),
/// its id its folder's.
pub fn read(dir: &Path, id: &str) -> Result<PackManifest, String> {
    let at = format!("{id}/{MANIFEST}");
    let path = dir.join(id).join(MANIFEST);
    let text = nettai_platform::fs::read_to_string(&path).map_err(|e| format!("{}: {e} (no pack {id} in {})", path.display(), dir.display()))?;
    let m = PackManifest::parse(&text, &at)?;
    if m.id != id {
        return Err(format!("{at}: id {:?} isn't its folder's name, {id}", m.id));
    }
    Ok(m)
}

/// Every pack of content `dir` (each folder with a manifest), by name.
pub fn packs(dir: &Path) -> Result<Vec<PackManifest>, String> {
    let mut names: Vec<String> = nettai_platform::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e} (is this a content directory?)", dir.display()))?
        .flatten()
        .filter(|e| nettai_platform::fs::is_file(e.path().join(MANIFEST)))
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
        None => Err(format!("{at}: {target} is no pack of this load ({own} requires only itself and the support packs it depends on)")),
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
/// its line comments): for a tool that reads a module's text (the content
/// check, which checks every file's requires, loaded or not; a test that
/// rewrites an index). A load scans nothing: its `require` runs.
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

/// The API declarations pack `id`'s content modules check against: the
/// engine's (nettai/, but its [`EXTRACT_DECLARATIONS`]), then each support
/// pack it depends on (in load order), then its own; each folder's
/// `.d.luau` files in path order, but those under its [`EXTRACT`] folder.
/// (A declaration is global to the modules checked against it; Luau's
/// .luaurc names none.)
pub fn declarations(dir: &Path, all: &BTreeMap<String, PackManifest>, id: &str) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<PathBuf> = declaration_files(&dir.join(ENGINE)).into_iter().filter(|f| !is_extract_declarations(f)).collect();
    if out.is_empty() {
        return Err(format!("{}: the engine's declarations ({ENGINE}/core.d.luau) aren't there", dir.display()));
    }
    for q in with_depends(all, id)? {
        let root = dir.join(&q.id);
        out.extend(declaration_files(&root).into_iter().filter(|f| !f.starts_with(root.join(EXTRACT))));
    }
    Ok(out)
}

/// The API declarations pack `id`'s [`EXTRACT`] modules (its recipe's)
/// check against: the engine's [`EXTRACT_DECLARATIONS`], then the `.d.luau`
/// files under the `extract/` folder of each support pack it depends on (in
/// load order) and of its own, in path order: the recipes' API and none of
/// the battle's (docs/design/extraction-recipes.md §4.5).
pub fn extract_declarations(dir: &Path, all: &BTreeMap<String, PackManifest>, id: &str) -> Result<Vec<PathBuf>, String> {
    let engine = dir.join(ENGINE).join(EXTRACT_DECLARATIONS);
    if !nettai_platform::fs::is_file(&engine) {
        return Err(format!("{}: the engine's declarations of the recipes ({ENGINE}/{EXTRACT_DECLARATIONS}) aren't there", dir.display()));
    }
    let mut out = vec![engine];
    for q in with_depends(all, id)? {
        out.extend(declaration_files(&dir.join(&q.id).join(EXTRACT)));
    }
    Ok(out)
}

/// Whether `file` is the engine's declarations of the recipes.
fn is_extract_declarations(file: &Path) -> bool {
    file.file_name().is_some_and(|n| n == EXTRACT_DECLARATIONS)
}

/// The `.d.luau` files under `dir`, in path order.
fn declaration_files(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = nettai_platform::fs::read_dir(dir) else { return };
        let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, out);
            } else if path.to_string_lossy().ends_with(".d.luau") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out
}

/// Whether pack `id` of content `dir` has an extraction recipe: its
/// [`EXTRACT`] folder's top module, `extract/init.luau`, which returns the
/// recipe (docs/design/extraction-recipes.md §4.2).
pub fn has_recipe(dir: &Path, id: &str) -> bool {
    nettai_platform::fs::is_file(dir.join(id).join(EXTRACT).join(format!("{INIT}.luau")))
}

/// Whether module `name` (`exe6:extract/hud`) is a recipe's: under its
/// pack's [`EXTRACT`] folder.
pub fn is_extract_module(name: &str) -> bool {
    name.split_once(keys::SEPARATOR).is_some_and(|(_, path)| path == EXTRACT || path.starts_with(&format!("{EXTRACT}/")))
}

/// A game pack's lock, in its folder (docs/design/content-deployment.md
/// §3): the commit of each support pack it goes with, the ones it was
/// checked against, one line each (`exelib = "a73907b5..."`). It is no
/// content (nothing loads it), and travels in the game's bundle with the
/// pack's other files.
pub const LOCK: &str = "depends.lock";

/// The support packs game `id`'s lock names: those it depends on, and
/// theirs (what a bundle of it holds besides it), by id.
pub fn locked(all: &BTreeMap<String, PackManifest>, id: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = with_depends(all, id)?.into_iter().filter(|p| p.id != id).map(|p| p.id.clone()).collect();
    out.sort();
    Ok(out)
}

/// Game `id`'s lock in content `dir`: each support pack's commit (40 hex
/// digits), by id; none when the game has no lock.
pub fn read_lock(dir: &Path, id: &str) -> Result<BTreeMap<String, String>, String> {
    let path = dir.join(id).join(LOCK);
    if !nettai_platform::fs::exists(&path) {
        return Ok(BTreeMap::new());
    }
    let at = format!("{id}/{LOCK}");
    let text = nettai_platform::fs::read_to_string(&path).map_err(|e| format!("{at}: {e}"))?;
    let lock: BTreeMap<String, String> = toml::from_str(&text).map_err(|e| format!("{at}: {e}"))?;
    if let Some((pack, commit)) = lock.iter().find(|(_, c)| c.len() != 40 || !c.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))) {
        return Err(format!("{at}: {pack}'s commit is 40 lowercase hex digits, not {commit:?}"));
    }
    Ok(lock)
}

/// A lock's text: one line per support pack, in id order.
pub fn lock_text(lock: &BTreeMap<String, String>) -> String {
    lock.iter().map(|(pack, commit)| format!("{pack} = \"{commit}\"\n")).collect()
}

/// What keeps game `id` from going with the support packs checked out:
/// its lock (`lock`) against support packs `wanted` ([`locked`]) and the
/// commits they are at (`heads`, by id). Each names the commits and the
/// fix.
pub fn lock_problems(id: &str, wanted: &[String], lock: &BTreeMap<String, String>, heads: &BTreeMap<String, String>) -> Vec<String> {
    let at = format!("{id}/{LOCK}");
    let fix = format!("check {id} against it, then `nettai-content lock {id}` and commit {LOCK} in {id}'s repository");
    let mut problems = Vec::new();
    for pack in wanted {
        match (lock.get(pack), heads.get(pack)) {
            (None, Some(head)) => problems.push(format!("{at}: locks no commit of {pack}, which is at {head}: {fix}")),
            (Some(locked), Some(head)) if locked != head => {
                problems.push(format!("{at}: locks {pack} at {locked}, and {pack} is at {head}: {fix}, or check out {pack} at {locked}"))
            }
            _ => {}
        }
    }
    for pack in lock.keys().filter(|p| !wanted.contains(p)) {
        problems.push(format!("{at}: locks {pack}, which {id} doesn't depend on: `nettai-content lock {id}` writes what it does"));
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_games_lock_names_its_support_packs_and_their_commits() {
        let all: BTreeMap<String, PackManifest> =
            [pack("g", PackKind::Game, &["lib"]), pack("lib", PackKind::Support, &["base"]), pack("base", PackKind::Support, &[])].into();
        assert_eq!(locked(&all, "g").unwrap(), ["base", "lib"]);
        let (a, b) = ("a".repeat(40), "b".repeat(40));
        let lock: BTreeMap<String, String> = [("base".to_string(), a.clone()), ("lib".to_string(), a.clone())].into();
        assert_eq!(lock_text(&lock), format!("base = \"{a}\"\nlib = \"{a}\"\n"));
        assert_eq!(toml::from_str::<BTreeMap<String, String>>(&lock_text(&lock)).unwrap(), lock);
        let wanted = locked(&all, "g").unwrap();
        assert!(lock_problems("g", &wanted, &lock, &lock).is_empty());
        // A support pack at another commit than the lock's: both commits and the fix.
        let heads: BTreeMap<String, String> = [("base".to_string(), a.clone()), ("lib".to_string(), b.clone())].into();
        let p = lock_problems("g", &wanted, &lock, &heads);
        assert!(p.len() == 1 && p[0].contains(&format!("locks lib at {a}, and lib is at {b}")) && p[0].contains("nettai-content lock g"), "{p:?}");
        // A support pack unlocked, and one locked that the game doesn't depend on.
        let lock: BTreeMap<String, String> = [("lib".to_string(), b.clone()), ("other".to_string(), b.clone())].into();
        let p = lock_problems("g", &wanted, &lock, &heads);
        assert!(p.iter().any(|p| p.contains("locks no commit of base")) && p.iter().any(|p| p.contains("locks other, which g doesn't")), "{p:?}");
        // A support pack whose checkout can't be seen isn't judged.
        assert!(lock_problems("g", &wanted, &lock, &BTreeMap::new()).iter().all(|p| p.contains("other")));
    }

    #[test]
    fn a_lock_reads_back_and_its_commits_are_full() {
        let dir = std::env::temp_dir().join(format!("nettai-lock-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("g")).unwrap();
        assert!(read_lock(&dir, "g").unwrap().is_empty(), "no lock");
        let c = "0123456789abcdef0123456789abcdef01234567";
        std::fs::write(dir.join("g").join(LOCK), format!("exelib = \"{c}\"\n")).unwrap();
        assert_eq!(read_lock(&dir, "g").unwrap(), [("exelib".to_string(), c.to_string())].into());
        std::fs::write(dir.join("g").join(LOCK), "exelib = \"a73907b5\"\n").unwrap();
        assert!(read_lock(&dir, "g").unwrap_err().contains("40 lowercase hex digits"));
        std::fs::remove_dir_all(&dir).ok();
    }

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

    /// A manifest says what the pack is and nothing more: what a game
    /// extracts from is its recipe's to say (`extract/init.luau`), and its
    /// locales' text tables are theirs.
    #[test]
    fn a_manifest_declares_no_recipe_and_no_text() {
        let head = "id = \"exe6\"\nkind = \"game\"\n";
        let e = PackManifest::parse(&format!("{head}[extract]\nroms = [\"BR6E\"]\n"), "exe6/manifest.toml").unwrap_err();
        assert!(e.contains("unknown field `extract`"), "{e}");
        let e = PackManifest::parse(&format!("{head}text = [\"game\"]\n"), "exe6/manifest.toml").unwrap_err();
        assert!(e.contains("unknown field `text`"), "{e}");
        assert!(is_extract_module("exe6:extract/hud") && is_extract_module("exelib:extract/place") && is_extract_module("exe6:extract"));
        assert!(!is_extract_module("exe6:extractor") && !is_extract_module("exe6:chips/extract/x"));
    }

    #[test]
    fn content_and_recipes_check_against_declarations_of_their_own() {
        let dir = std::env::temp_dir().join(format!("nettai-declarations-{}", std::process::id()));
        let file = |path: &str| {
            let p = dir.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, "").unwrap();
        };
        for f in ["nettai/core.d.luau", "nettai/extract.d.luau", "lib/types.d.luau", "lib/extract/place.d.luau", "g/types.d.luau", "g/extract/own.d.luau"] {
            file(f);
        }
        let all: BTreeMap<String, PackManifest> = [pack("lib", PackKind::Support, &[]), pack("g", PackKind::Game, &["lib"])].into();
        let names = |files: Vec<PathBuf>| files.iter().map(|f| f.strip_prefix(&dir).unwrap().to_string_lossy().replace('\\', "/")).collect::<Vec<_>>();
        assert_eq!(names(declarations(&dir, &all, "g").unwrap()), ["nettai/core.d.luau", "lib/types.d.luau", "g/types.d.luau"]);
        assert_eq!(names(extract_declarations(&dir, &all, "g").unwrap()), ["nettai/extract.d.luau", "lib/extract/place.d.luau", "g/extract/own.d.luau"]);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A module saved with CRLF line endings reads as the same source as
    /// one saved with LF: a Windows checkout plays the same content.
    #[test]
    fn a_crlf_module_reads_as_lf() {
        let dir = std::env::temp_dir().join(format!("nettai-crlf-{}", std::process::id()));
        nettai_platform::fs::create_dir_all(dir.join("g")).unwrap();
        nettai_platform::fs::write(dir.join("g/init.luau"), "--!strict\r\nreturn {}\r\n").unwrap();
        let read = Dirs(BTreeMap::from([("g".to_string(), dir.join("g"))])).read("g:init");
        nettai_platform::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(read.as_deref(), Some("--!strict\nreturn {}\n"));
    }

    /// A require is found in one place: among modules in memory, in packs'
    /// folders, or the first of the two that has it; a folder's name is its
    /// init; and what the packs refuse, a missing module and a path out of
    /// the pack each say where they are written.
    #[test]
    fn a_require_finds_its_module() {
        let memory: BTreeMap<String, String> =
            [("g:init", "require(\"@self/rules\")"), ("g:rules/init", "return {}"), ("g:rules/turns", "return {}"), ("lib:x", "return {}")]
                .into_iter()
                .map(|(n, s)| (n.to_string(), s.to_string()))
                .collect();
        let all: BTreeMap<String, PackManifest> = [pack("g", PackKind::Game, &["lib"]), pack("h", PackKind::Game, &[]), pack("lib", PackKind::Support, &[])].into_iter().collect();
        let found = |from: &str, written: &str| find(&memory, &all, from, written);
        assert_eq!(found("g:init", "@self/rules").as_deref(), Ok("g:rules/init"), "a folder's name is its init");
        assert_eq!(found("g:rules/init", "@self/turns").as_deref(), Ok("g:rules/turns"));
        assert_eq!(found("g:rules/turns", "../rules").as_deref(), Ok("g:rules/init"));
        assert_eq!(found("g:rules/turns", "@lib/x").as_deref(), Ok("lib:x"));
        assert_eq!(found("g:init", "@self/gone").unwrap_err(), "g/init.luau: require(\"@self/gone\"): no module g/gone.luau");
        assert!(found("g:init", "./rules").unwrap_err().starts_with("g/init.luau: require(\"./rules\") from g:init: leaves pack g"));
        assert!(found("lib:x", "@g/rules").unwrap_err().starts_with("lib/x.luau: require(\"@g/rules\"): g is a game pack"));
        assert!(found("g:init", "@h/x").unwrap_err().contains("h is a game pack, which no other pack requires"));
        // Without packs (a test's modules alone), any module.
        assert_eq!(find(&memory, &BTreeMap::new(), "g:init", "@lib/x").as_deref(), Ok("lib:x"));
        // Packs' folders, and memory before them (a tool's stand-in).
        let dir = std::env::temp_dir().join(format!("nettai-packs-{}", std::process::id()));
        nettai_platform::fs::create_dir_all(dir.join("g/chips/cannon")).unwrap();
        nettai_platform::fs::write(dir.join("g/chips/cannon/init.luau"), "return 1").unwrap();
        nettai_platform::fs::write(dir.join("g/chips/cannon/shot.luau"), "return 2").unwrap();
        nettai_platform::fs::write(dir.join("g/types.d.luau"), "").unwrap();
        let dirs = Dirs::of_content(&dir, ["g"]);
        assert_eq!(dirs.names(), ["g:chips/cannon/init", "g:chips/cannon/shot"]);
        assert_eq!(dirs.read("g:chips/cannon/shot").as_deref(), Some("return 2"));
        assert!(!dirs.has("g:chips/cannon") && !dirs.has("g:../g/chips/cannon/init") && !dirs.has("h:x"));
        assert_eq!(find(&dirs, &all, "g:chips/cannon/shot", "../cannon").as_deref(), Ok("g:chips/cannon/init"));
        let stand_in: BTreeMap<String, String> = [("g:chips/cannon/shot".to_string(), "return 3".to_string())].into();
        let both = (stand_in, dirs);
        assert_eq!(both.read("g:chips/cannon/shot").as_deref(), Some("return 3"));
        assert_eq!(both.read("g:chips/cannon/init").as_deref(), Some("return 1"));
        nettai_platform::fs::remove_dir_all(&dir).ok();
    }

    /// A module gives the sections whose type a top-level local of its is
    /// written as; its requires are what it names, outside comments.
    #[test]
    fn a_modules_sections_are_its_typed_locals() {
        let chips = "-- Chips.\nrequire(\"@self/cannon\")\n-- require(\"@self/later\")  -- no use yet\nrequire(\"@exelib/x/init\")\n";
        assert_eq!(requires(chips), ["@self/cannon", "@exelib/x/init"]);
        let series = "local x = 1\nlocal chips: { [string]: Chip } = {\n}\nlocal base: FormDef = {}\nreturn chips\n";
        assert_eq!(sections_of(series), ["chips", "forms"]);
        assert_eq!(sections_of("local numtrap: Chip\nnumtrap = {}\n"), ["chips"]);
        // Not a definition: a local of the type taken from elsewhere, or
        // inside a function.
        assert!(sections_of("local navi: NaviDef = protoman\n    local chip: Chip = {}\n").is_empty());
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
