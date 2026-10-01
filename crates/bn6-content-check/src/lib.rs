//! Type-check a Luau content pack (docs/design/scripting.md §3.3) against
//! the content API definitions in its `core.d.luau`, with Luau's own
//! analysis (strict mode, the new solver), in process.
//!
//! Each module is checked on its own and `require` is typed `any`; an
//! editor running luau-lsp with `--definitions=<pack>/core.d.luau`
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

/// The pack's definitions: the API (`core.d.luau`) and, if present, the
/// pack's shared types (`types.d.luau`).
pub fn definitions(dir: &Path) -> Result<String, String> {
    let mut defs = std::fs::read_to_string(dir.join("core.d.luau")).map_err(|e| format!("core.d.luau: {e}"))?;
    if let Ok(types) = std::fs::read_to_string(dir.join("types.d.luau")) {
        defs += "\n";
        defs += &types;
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
