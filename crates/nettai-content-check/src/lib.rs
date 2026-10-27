//! Type-check the content (docs/design/scripting.md §3.3;
//! docs/design/content-model-v2.md §4.0), each pack against its
//! declarations (the engine's core.d.luau, the support packs' it depends
//! on, its own; `packs::declarations`), with Luau's own analysis (strict
//! mode, the new solver), in process; and check that each game has its top
//! module (init.luau) and that every module's literal requires name a
//! module that is there, each one its pack may make (a pack requires only
//! itself and the support packs it depends on): each found as a load finds
//! it (`packs::find`), in every file of the packs, loaded or not.
//!
//! Each module is checked on its own and `require` is typed `any`; an
//! editor running luau-lsp with a `--definitions=` for each declaration
//! resolves requires and checks across modules too.
//!
//! A pack's extraction recipe (its `extract/` modules,
//! docs/design/extraction-recipes.md §4.5) checks against the recipes'
//! declarations (nettai/extract.d.luau and the `.d.luau` files under its
//! support packs' and its own `extract/`), the content against the rest:
//! neither sees the other's API. A content module requires no recipe
//! module, a recipe module nothing but recipe modules, and a recipe's
//! `data` reads only what its manifest's `[extract]` names.

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
/// (`packs::declarations`), the recipes' aside.
pub fn definitions(dir: &Path, pack: &str) -> Result<String, String> {
    use nettai_content_api::packs;
    let all = packs::packs(dir)?.into_iter().map(|p| (p.id.clone(), p)).collect();
    concatenated(&packs::declarations(dir, &all, pack)?)
}

/// The declarations pack `pack`'s recipe (its `extract/` modules) checks
/// against (`packs::extract_declarations`).
pub fn extract_definitions(dir: &Path, pack: &str) -> Result<String, String> {
    use nettai_content_api::packs;
    let all = packs::packs(dir)?.into_iter().map(|p| (p.id.clone(), p)).collect();
    concatenated(&packs::extract_declarations(dir, &all, pack)?)
}

fn concatenated(files: &[std::path::PathBuf]) -> Result<String, String> {
    let mut defs = String::new();
    for f in files {
        defs += &std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        defs += "\n";
    }
    Ok(defs)
}

/// What the packs of content `dir` refuse or can't find: a folder of
/// content/ that is no pack (no manifest), a module outside the packs, a
/// manifest that depends on what it can't, a game without its top module
/// (init.luau), and in every module (loaded or not) a require of no module
/// and a require across packs that the packs refuse (`packs::find`, as a
/// load finds a module); each with the path that names it.
pub fn reach(dir: &Path) -> Result<Vec<Problem>, String> {
    reach_of(dir, None)
}

/// [`reach`], of packs `only` (by id) alone when given: what content/
/// holds besides them isn't theirs to answer for.
fn reach_of(dir: &Path, only: Option<&[&str]>) -> Result<Vec<Problem>, String> {
    use nettai_content_api::{keys, packs};
    let named = |id: &str| only.is_none_or(|o| o.contains(&id));
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
                Err(_) if !named(&name) => {}
                Err(e) => problems.push(if entry.join(packs::MANIFEST).is_file() {
                    e
                } else {
                    format!("{name}/: no {}; a folder of content/ is a pack", packs::MANIFEST)
                }),
            }
        } else if only.is_none() && name.ends_with(".luau") {
            problems.push(format!("{name}: a module belongs to a pack (content/<pack>/...)"));
        }
    }
    let files = packs::Dirs::of_content(dir, all.keys().map(String::as_str));
    if let Some(missing) = only.into_iter().flatten().find(|id| !all.contains_key(**id)) {
        problems.push(format!("no pack {missing} in {}", dir.display()));
    }
    for p in all.values().filter(|p| named(&p.id)) {
        if let Err(e) = packs::with_depends(&all, &p.id) {
            problems.push(e);
        }
        if let Some(m) = p.entry()
            && packs::module(&files, &m).is_none()
        {
            let file = format!("{}.luau", keys::module_path(&m));
            problems.push(format!("{file}: game pack {} has no top module: its {}.luau requires what the game has", p.id, packs::INIT));
        }
        for (path, source) in modules(&dir.join(&p.id)).map_err(|e| e.to_string())? {
            let name = p.module(path.trim_end_matches(".luau"));
            let file = format!("{}/{path}", p.id);
            let recipe = packs::is_extract_module(&name);
            for written in packs::requires(&source) {
                match packs::find(&files, &all, &name, &written) {
                    Err(e) => problems.push(e),
                    Ok(target) if recipe && !packs::is_extract_module(&target) => problems.push(format!(
                        "{file}: require({written:?}): {}.luau isn't a recipe's: an {}/ module requires only {}/ modules (its own pack's and its support packs')",
                        keys::module_path(&target),
                        packs::EXTRACT,
                        packs::EXTRACT
                    )),
                    Ok(target) if !recipe && packs::is_extract_module(&target) => problems.push(format!(
                        "{file}: require({written:?}): {}.luau is a recipe's: content requires no {}/ module (a recipe runs in a VM of its own)",
                        keys::module_path(&target),
                        packs::EXTRACT
                    )),
                    Ok(_) => {}
                }
            }
            if recipe {
                for (line, read) in lints::data_reads(&source) {
                    let declared = p.extract.as_ref().is_some_and(|e| e.reads.iter().any(|r| r == &read));
                    if p.kind == nettai_content_api::PackKind::Support {
                        problems.push(format!(
                            "{file}:{line}: data({read:?}): a support pack's recipe code reads no data of its own; the game's recipe passes it what it reads"
                        ));
                    } else if !declared {
                        problems.push(format!("{file}:{line}: data({read:?}): {}/{} doesn't name it in [extract] reads", p.id, packs::MANIFEST));
                    }
                }
                continue;
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
    check(dir, None)
}

/// [`check_content`] of packs `only` of content `dir`, by id (a game and
/// the support packs it depends on: what a bundle of it holds,
/// docs/design/content-deployment.md §3): their modules, and what they
/// refuse or can't find. The rest of `dir` isn't looked at but for the
/// manifests a require across packs is judged by.
pub fn check_packs(dir: &Path, only: &[&str]) -> Result<(usize, Vec<Problem>), String> {
    check(dir, Some(only))
}

fn check(dir: &Path, only: Option<&[&str]>) -> Result<(usize, Vec<Problem>), String> {
    let (mut n, mut problems) = (0, Vec::new());
    for p in nettai_content_api::packs::packs(dir)?.into_iter().filter(|p| only.is_none_or(|o| o.contains(&p.id.as_str()))) {
        let mut checker = PackChecker::new(&definitions(dir, &p.id)?)?;
        // (The recipe's checker, made when the pack has a recipe module.)
        let mut recipes: Option<PackChecker> = None;
        let name = &p.id;
        for (path, source) in modules(&dir.join(name)).map_err(|e| e.to_string())? {
            let full = format!("{name}/{path}");
            if path.starts_with(&format!("{}/", nettai_content_api::packs::EXTRACT)) {
                let recipe = match &mut recipes {
                    Some(c) => c,
                    None => recipes.insert(PackChecker::new(&extract_definitions(dir, &p.id)?)?),
                };
                problems.extend(recipe.check(&full, &source)?);
                n += 1;
                continue;
            }
            problems.extend(checker.check(&full, &source)?);
            // (The lints read a module's path in its pack: rules/ holds a game's rules.)
            problems.extend(lints::lints(&path, &source).into_iter().map(|p| format!("{name}/{p}")));
            n += 1;
        }
    }
    problems.extend(reach_of(dir, only)?);
    Ok((n, problems))
}

/// The games' locks (`depends.lock`, docs/design/content-deployment.md §3)
/// against the support packs checked out: what keeps a game of content
/// `dir` (of packs `only`, when given) from going with them
/// (`packs::lock_problems`), and, as notes, the support packs that are no
/// git checkout of their own (as a bundle's folders aren't), whose commits
/// can't be seen, so the locks on them aren't checked.
pub fn locks(dir: &Path, only: Option<&[&str]>) -> Result<(Vec<Problem>, Vec<String>), String> {
    use nettai_content_api::{PackKind, packs};
    let all: std::collections::BTreeMap<String, packs::PackManifest> = packs::packs(dir)?.into_iter().map(|p| (p.id.clone(), p)).collect();
    let (mut problems, mut notes) = (Vec::new(), Vec::new());
    let mut heads = std::collections::BTreeMap::new();
    for game in all.values().filter(|p| p.kind == PackKind::Game && only.is_none_or(|o| o.contains(&p.id.as_str()))) {
        // (A manifest's depends that can't be followed is reach's to report.)
        let Ok(wanted) = packs::locked(&all, &game.id) else { continue };
        for pack in &wanted {
            if heads.contains_key(pack) || notes.iter().any(|n: &String| n.starts_with(&format!("{pack}/"))) {
                continue;
            }
            match checkout(&dir.join(pack)) {
                Ok(commit) => {
                    heads.insert(pack.clone(), commit);
                }
                Err(why) => notes.push(format!("{pack}/: {why}; the locks on it ({}) aren't checked", packs::LOCK)),
            }
        }
        match packs::read_lock(dir, &game.id) {
            Ok(lock) => problems.extend(packs::lock_problems(&game.id, &wanted, &lock, &heads)),
            Err(e) => problems.push(e),
        }
    }
    Ok((problems, notes))
}

/// The commit checked out in folder `dir`, if it is a git checkout of its
/// own (a submodule's or a clone's, with its own `.git`); else why not.
fn checkout(dir: &Path) -> Result<String, String> {
    if !dir.join(".git").exists() {
        return Err("no git checkout of its own".to_string());
    }
    let out = std::process::Command::new("git").arg("-C").arg(dir).args(["rev-parse", "HEAD"]).output().map_err(|e| format!("git can't be run ({e})"))?;
    if !out.status.success() {
        return Err(format!("git rev-parse HEAD: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
