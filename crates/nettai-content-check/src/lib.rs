//! Type-check the content (docs/design/scripting.md §3.3;
//! docs/design/content-model-v2.md §4.0), each pack against its
//! declarations (the engine's core.d.luau, the support packs' it uses, its
//! own; `packs::declarations`), with Luau's own analysis (strict mode, the
//! new solver), in process; and check that the packs' manifests and
//! requires name only modules that are there, each require one its pack may
//! make (a pack requires only itself and the support packs it uses).
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
/// engine's, then the support packs it uses', then its own
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
/// manifest's bad uses, a listed or unported module that isn't there, and
/// in every module (listed or not) a require of no module and a require
/// across packs that the packs refuse (`packs::check_require`); each with
/// the path that names it.
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
        if let Err(e) = packs::with_uses(&all, &p.id) {
            problems.push(e);
        }
        for m in p.entries().iter().chain(p.unported().iter()) {
            let file = format!("{}.luau", keys::module_path(m));
            // (A folder names its `init` module.)
            let init = format!("{}.luau", keys::module_path(&keys::init_of(m)));
            if !dir.join(&file).is_file() && !dir.join(&init).is_file() {
                problems.push(format!("{}/{}: lists {}, and no module {file} is there", p.id, packs::MANIFEST, keys::local(m)));
            }
        }
        for (path, source) in modules(&dir.join(&p.id)).map_err(|e| e.to_string())? {
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
