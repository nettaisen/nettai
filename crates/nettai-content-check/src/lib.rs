//! Type-check the content (docs/design/scripting.md §3.3;
//! docs/design/content-model-v2.md §4.0), each pack against its
//! declarations (the engine's core.d.luau, the support packs' it depends
//! on, its own; `packs::declarations`), with Luau's own analysis (strict
//! mode, the new solver), in process; and check that each game has its top
//! module (init.luau), that the packs' requires name only modules that are
//! there, each require one its pack may make (a pack requires only itself
//! and the support packs it depends on), and that a game loads every module
//! of it that defines something (its inits require them).
//!
//! Each module is checked on its own and `require` is typed `any`; an
//! editor running luau-lsp with a `--definitions=` for each declaration
//! resolves requires and checks across modules too.

pub mod lints;

use std::path::Path;
use std::time::Duration;

/// A type error, as `path:line:col: message`.
pub type Problem = String;

/// A checker loaded with a pack's API definitions.
pub struct PackChecker {
    checker: luau_analyze::Checker,
}

impl PackChecker {
    /// Load the definitions (see [`definitions`]).
    pub fn new(definitions: &str) -> Result<PackChecker, String> {
        let mut checker = luau_analyze::Checker::new().map_err(|e| e.to_string())?;
        checker.add_definitions(definitions).map_err(|e| format!("definitions: {e}"))?;
        checker.add_definitions("declare function require(path: string): any").map_err(|e| e.to_string())?;
        Ok(PackChecker { checker })
    }

    /// The type errors in one module's source.
    pub fn check(&mut self, name: &str, source: &str) -> Result<Vec<Problem>, String> {
        let options = luau_analyze::CheckOptions {
            timeout: Some(Duration::from_secs(30)),
            module_name: Some(name),
            ..Default::default()
        };
        let result = self.checker.check_with_options(source, options).map_err(|e| e.to_string())?;
        if result.timed_out {
            return Err(format!("{name}: the type check timed out"));
        }
        Ok(result.errors().iter().map(|d| format!("{name}:{}:{}: {}", d.line + 1, d.col + 1, d.message)).collect())
    }
}

/// Every `.luau` module under `dir` (definition files excluded), as
/// (path relative to `dir`, source), in path order.
pub fn modules(dir: &Path) -> std::io::Result<Vec<(String, String)>> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else if path.extension().is_some_and(|e| e == "luau") && !path.to_string_lossy().ends_with(".d.luau") {
                let rel = path.strip_prefix(root).expect("walked under the root").to_string_lossy().replace('\\', "/");
                out.push((rel, std::fs::read_to_string(&path)?));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out)?;
    out.sort();
    Ok(out)
}

/// The declarations pack `pack` of content `dir` checks against: the
/// engine's, then the support packs' it depends on, then its own
/// (`packs::declarations`).
pub fn definitions(dir: &Path, pack: &str) -> Result<String, String> {
    use nettai_content_api::packs;
    let all = packs::packs(dir)?.into_iter().map(|p| (p.id.clone(), p)).collect();
    let mut defs = String::new();
    for f in packs::declarations(dir, &all, pack)? {
        defs += &std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        defs += "\n";
    }
    Ok(defs)
}

/// What the packs of content `dir` refuse or can't find: a folder of
/// content/ that is no pack (no manifest), a module outside the packs, a
/// manifest that depends on what it can't, a game without its top module
/// (init.luau), in every module (loaded or not) a require of no module
/// and a require across packs that the packs refuse
/// (`packs::check_require`), and a game's module that defines something
/// and that the game doesn't load ([`unloaded`]); each with the path that
/// names it.
pub fn reach(dir: &Path) -> Result<Vec<Problem>, String> {
    use nettai_content_api::{keys, packs};
    let mut problems = Vec::new();
    let mut all = std::collections::BTreeMap::new();
    let mut entries: Vec<std::path::PathBuf> =
        std::fs::read_dir(dir).map_err(|e| format!("{}: {e} (is this a content directory?)", dir.display()))?.flatten().map(|e| e.path()).collect();
    entries.sort();
    for entry in entries {
        let name = entry.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if name.starts_with('.') || name == packs::ENGINE {
            continue;
        }
        if entry.is_dir() {
            match packs::read(dir, &name) {
                Ok(m) => {
                    all.insert(name, m);
                }
                Err(e) => problems.push(if entry.join(packs::MANIFEST).is_file() {
                    e
                } else {
                    format!("{name}/: no {}; a folder of content/ is a pack", packs::MANIFEST)
                }),
            }
        } else if name.ends_with(".luau") {
            problems.push(format!("{name}: a module belongs to a pack (content/<pack>/...)"));
        }
    }
    for p in all.values() {
        if let Err(e) = packs::with_depends(&all, &p.id) {
            problems.push(e);
        }
        if let Some(m) = p.entry() {
            let file = format!("{}.luau", keys::module_path(&m));
            if !dir.join(&file).is_file() {
                problems.push(format!("{file}: game pack {} has no top module: its {}.luau requires what the game has", p.id, packs::INIT));
            }
        }
        let sources = modules(&dir.join(&p.id)).map_err(|e| e.to_string())?;
        if p.kind == nettai_content_api::PackKind::Game {
            problems.extend(unloaded(p, &sources));
        }
        for (path, source) in sources {
            let name = p.module(path.trim_end_matches(".luau"));
            let file = format!("{}/{path}", p.id);
            for written in packs::requires(&source) {
                match keys::resolve(&name, &written) {
                    Ok(target) => {
                        if let Err(e) = packs::check_require(&all, &name, &written, &target) {
                            problems.push(e);
                        } else if !dir.join(format!("{}.luau", keys::module_path(&target))).is_file()
                            && !dir.join(format!("{}.luau", keys::module_path(&keys::init_of(&target)))).is_file()
                        {
                            problems.push(format!("{file}: require({written:?}): no module {}.luau", keys::module_path(&target)));
                        }
                    }
                    Err(e) => problems.push(format!("{file}: {e}")),
                }
            }
            if p.kind == nettai_content_api::PackKind::Support {
                for (line, what) in lints::game_context(&source) {
                    problems.push(format!(
                        "{file}:{line}: support pack {} reaches for the game's context by itself, {what}; a maker takes it from the game's caller",
                        p.id
                    ));
                }
            }
        }
    }
    Ok(problems)
}

/// The modules of game pack `p` (`sources`: its modules by path, with
/// their text) that define something (`define.<registry>`) and that the
/// game doesn't load: nothing its top module requires, in turn, requires
/// them (docs/design/content-model-v2.md §4.0: a game's inits require what
/// it has, and tools/content/index.py writes them). A chip with no use yet
/// is none: its folder's init names it in a commented require
/// (`-- require("@self/later")`), and what its folder holds waits with it.
pub fn unloaded(p: &nettai_content_api::PackManifest, sources: &[(String, String)]) -> Vec<Problem> {
    use nettai_content_api::{keys, packs};
    use std::collections::{BTreeMap, BTreeSet};
    let by_name: BTreeMap<String, &str> =
        sources.iter().map(|(path, source)| (p.module(path.trim_end_matches(".luau")), source.as_str())).collect();
    // (A folder names its init.)
    let module_of = |name: &str| -> Option<String> {
        if by_name.contains_key(name) {
            return Some(name.to_string());
        }
        Some(keys::init_of(name)).filter(|init| by_name.contains_key(init))
    };
    let Some(top) = p.entry().and_then(|m| module_of(&m)) else { return Vec::new() };
    let mut reached: BTreeSet<String> = BTreeSet::from([top.clone()]);
    let mut waiting: Vec<String> = Vec::new();
    let mut pending = vec![top];
    while let Some(module) = pending.pop() {
        let source = by_name[&module];
        for written in packs::requires(source) {
            let Some(target) = keys::resolve(&module, &written).ok().and_then(|t| module_of(&t)) else { continue };
            if reached.insert(target.clone()) {
                pending.push(target);
            }
        }
        if packs::is_index(source) {
            waiting.extend(packs::commented_requires(source).iter().filter_map(|w| keys::resolve(&module, w).ok()));
        }
    }
    let defines = |source: &str| {
        source.lines().map(|l| l.split("--").next().unwrap_or("")).any(|code| {
            code.match_indices("define.").any(|(i, _)| code[i + "define.".len()..].chars().next().is_some_and(|c| c.is_ascii_lowercase()))
        })
    };
    let mut out = Vec::new();
    for (name, source) in &by_name {
        let waits = waiting.iter().any(|w| name == w || name.strip_prefix(w.as_str()).is_some_and(|rest| rest.starts_with('/')));
        if !reached.contains(name) && !waits && defines(source) {
            out.push(format!(
                "{}.luau: it defines something, and {} doesn't load it: no init.luau requires it, in turn (tools/content/index.py, the verification workspace's, writes a game's inits from its modules; a chip with no use yet is a commented require in its folder's)",
                keys::module_path(name),
                p.id
            ));
        }
    }
    out
}

/// Check the content directory `dir` (content/: packs,
/// docs/design/content-model-v2.md §4.0), each pack against its
/// declarations, and lint it: every script of every pack, each named by
/// its path; then what the packs refuse or can't find ([`reach`]).
pub fn check_content(dir: &Path) -> Result<(usize, Vec<Problem>), String> {
    let (mut n, mut problems) = (0, Vec::new());
    for p in nettai_content_api::packs::packs(dir)? {
        let mut checker = PackChecker::new(&definitions(dir, &p.id)?)?;
        let name = &p.id;
        for (path, source) in modules(&dir.join(name)).map_err(|e| e.to_string())? {
            let full = format!("{name}/{path}");
            problems.extend(checker.check(&full, &source)?);
            // (The lints read a module's path in its pack: rules/ holds a game's rules.)
            problems.extend(lints::lints(&path, &source).into_iter().map(|p| format!("{name}/{p}")));
            n += 1;
        }
    }
    problems.extend(reach(dir)?);
    Ok((n, problems))
}
