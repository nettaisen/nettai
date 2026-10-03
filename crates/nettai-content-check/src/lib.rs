//! Type-check a Luau content root (docs/design/scripting.md §3.3) against
//! the engine's content API definitions (content/nettai/core.d.luau, beside
//! the roots) and the root's own (its `*.d.luau`), with Luau's own analysis
//! (strict mode, the new solver), in process.
//!
//! Each module is checked on its own and `require` is typed `any`; an
//! editor running luau-lsp with `--definitions=content/nettai/core.d.luau
//! --definitions=<root>/types.d.luau` resolves requires and checks across
//! modules too.

pub mod lints;

use std::path::Path;
use std::time::Duration;

/// A type error, as `path:line:col: message`.
pub type Problem = String;

/// The folder of behavior every game's folder may require (content/common).
pub const COMMON: &str = "common";

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

/// The definitions a root's modules check against: the engine's (every
/// `*.d.luau` of content/nettai, the sibling `nettai` of `dir`), the shared
/// folder's (content/common's, the sibling `common` of `dir`, when there is
/// one: the types of the modules every game's folder requires), then the
/// root's own (its `*.d.luau`: BN6's shared types, `types.d.luau`), each in
/// name order.
pub fn definitions(dir: &Path) -> Result<String, String> {
    let parent = dir.parent().unwrap_or(Path::new("."));
    let engine = parent.join("nettai");
    let common = parent.join(COMMON);
    let mut folders = vec![engine.as_path()];
    if common.is_dir() && common.file_name() != dir.file_name() {
        folders.push(common.as_path());
    }
    folders.push(dir);
    let mut defs = String::new();
    for d in folders {
        let mut files: Vec<_> = std::fs::read_dir(d)
            .map_err(|e| format!("{}: {e}", d.display()))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().ends_with(".d.luau"))
            .collect();
        files.sort();
        if d == engine && files.is_empty() {
            return Err(format!("{}: the engine's API definitions (core.d.luau) aren't here", engine.display()));
        }
        for f in files {
            defs += &std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
            defs += "\n";
        }
    }
    Ok(defs)
}

/// Check a whole pack directory against its definitions, and lint it: the
/// number of modules checked and the problems found.
pub fn check_pack(dir: &Path) -> Result<(usize, Vec<Problem>), String> {
    let mut checker = PackChecker::new(&definitions(dir)?)?;
    let modules = modules(dir).map_err(|e| e.to_string())?;
    let mut problems = Vec::new();
    for (path, source) in &modules {
        problems.extend(checker.check(path, source)?);
        problems.extend(lints::lints(path, source));
    }
    Ok((modules.len(), problems))
}

/// Check every folder of the content directory `dir` (content/: one
/// namespace, docs/design/rules-in-luau.md) against the engine's
/// declarations and every folder's own (a module may use another folder's
/// types: BN5's BN6's), and lint it: the modules checked and the problems,
/// each module named by its folder and path.
pub fn check_content(dir: &Path) -> Result<(usize, Vec<Problem>), String> {
    let folders: Vec<std::path::PathBuf> = {
        let mut f: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir() && p.file_name().is_some_and(|n| n != "nettai" && !n.to_string_lossy().starts_with('.')))
            .collect();
        f.sort();
        f
    };
    let mut defs = own_definitions(&dir.join("nettai"))?;
    for folder in &folders {
        defs += &own_definitions(folder)?;
    }
    let mut checker = PackChecker::new(&defs)?;
    let (mut n, mut problems) = (0, Vec::new());
    for folder in &folders {
        let name = folder.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        for (path, source) in modules(folder).map_err(|e| e.to_string())? {
            let full = format!("{name}/{path}");
            problems.extend(checker.check(&full, &source)?);
            problems.extend(lints::lints(&path, &source).into_iter().map(|p| format!("{name}/{p}")));
            n += 1;
        }
    }
    Ok((n, problems))
}

/// A folder's own definitions (its `*.d.luau`), in name order.
fn own_definitions(dir: &Path) -> Result<String, String> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".d.luau"))
        .collect();
    files.sort();
    let mut defs = String::new();
    for f in files {
        defs += &std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        defs += "\n";
    }
    Ok(defs)
}
