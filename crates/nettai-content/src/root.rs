//! The content: every content folder (content/bn6, content/bn5...) loads,
//! one namespace (docs/design/rules-in-luau.md, the flat namespace; the
//! user: "maybe you should just have it all in a flat namespace and then
//! in the chip ids directly have bn6:cannon or whatever"). A folder holds:
//!
//! ```text
//! **/*.luau                       the modules: what they define (chips, navis, forms, weapons,
//!                                 stages, rules...) and the code that runs it; every id
//!                                 written in full (`bn6:minibomb`), every asset name too
//! *.d.luau                        its own API definitions, for editors and the checker (the
//!                                 engine's are content/nettai's)
//! compat/                         the original's numbers by id: tools' data, not content
//! locales/<language>.toml         display text by id, one table a language: the own
//!                                 language's (en) is the content's strings, the others a
//!                                 frontend's (crate::locale)
//! ```
//!
//! A folder of behavior only (content/common: modules other folders
//! require, no assets of its own, no definitions to name) needs neither
//! compat nor locales.
//!
//! [`read`] reads one folder, [`read_all`] every folder of a content
//! directory (content/nettai, the engine's declarations, aside). The assets
//! the definitions name (`asset.sprite("bn6:bomb")`) come from the
//! extracted packs' asset indices; `crate::pack::load_battle` puts the two
//! together.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use nettai_battle::content::RootManifest;

use crate::report::Report;

/// This repository's content directory.
const CONTENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");

/// The engine's declarations' folder in a content directory.
pub const DECLARATIONS: &str = "nettai";

/// The content directory: `$NETTAI_CONTENT`, else this repository's
/// content/.
pub fn content() -> PathBuf {
    std::env::var_os("NETTAI_CONTENT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(CONTENT))
}

/// The engine's API declarations for content (content/nettai).
pub fn engine_declarations(content: &Path) -> PathBuf {
    content.join(DECLARATIONS)
}

/// What a content folder holds.
#[derive(Clone, Debug, Default)]
pub struct Root {
    /// Its name (the folder's).
    pub manifest: RootManifest,
    /// Where it was read from.
    pub dir: PathBuf,
    /// Modules by path in the folder, without `.luau` (not the `.d.luau`
    /// definitions).
    pub modules: BTreeMap<String, String>,
    /// The content's own language's strings (`locales/en.toml`), ids in
    /// full: its display text, which the define phase counts the chatbox's
    /// timing from (`Content::strings`).
    pub strings: crate::locale::Strings,
}

impl Root {
    /// Leave out the chips whose module names no use yet (`define.chip`'s
    /// `action`, `dimming`, `navi` or `instant`: a port that hasn't written
    /// them, BN5's), with every module of their folders: the define phase
    /// refuses a chip without one, and the rest of the folder plays without
    /// them. So is every chip folder one of whose modules requires a module
    /// left out, in turn (a Program Advance naming an unported ingredient,
    /// a chip borrowing an unported chip's module): it can't load without
    /// it. (Anything else that requires one stops the define phase, which
    /// says so.) The folders' keys as the folder writes them
    /// (`chips/<key>/chip`), in order. The content is then exactly what
    /// loaded, and so is its hash (netplay's handshake).
    pub fn leave_out_unported(&mut self) -> Vec<String> {
        fn names_a_use(module: &str) -> bool {
            module.lines().any(|l| {
                let l = l.strip_prefix("    ").unwrap_or("");
                ["action", "dimming", "navi", "instant"]
                    .iter()
                    .any(|f| l.strip_prefix(f).is_some_and(|r| r.trim_start().starts_with('=')))
            })
        }
        // A module's chip folder (`chips/<key>/...`).
        fn chip_folder(path: &str) -> Option<&str> {
            let rest = path.strip_prefix("chips/")?;
            rest.split_once('/').map(|(key, _)| key)
        }
        let mut out: std::collections::BTreeSet<String> = self
            .modules
            .iter()
            .filter_map(|(path, text)| Some(path.strip_prefix("chips/")?.strip_suffix("/chip")?).filter(|_| !names_a_use(text)))
            .map(str::to_string)
            .collect();
        if out.is_empty() {
            return Vec::new();
        }
        // What each module requires in this folder.
        let requires: Vec<(&str, Vec<String>)> =
            self.modules.iter().map(|(path, text)| (path.as_str(), requires_in(&self.manifest.name, path, text))).collect();
        loop {
            let more: Vec<String> = requires
                .iter()
                .filter_map(|(path, targets)| {
                    let key = chip_folder(path)?;
                    let needs = targets.iter().any(|t| chip_folder(t).is_some_and(|k| out.contains(k)));
                    (needs && !out.contains(key)).then(|| key.to_string())
                })
                .collect();
            if more.is_empty() {
                break;
            }
            out.extend(more);
        }
        self.modules.retain(|path, _| !chip_folder(path).is_some_and(|k| out.contains(k)));
        out.into_iter().collect()
    }
}

/// The modules `module` (a path in the folder named `root`) requires in its
/// own folder, by path: each `require("...")` it writes, relative to it
/// (`./x`, `../lib/x`) or from the folder's top (`@<root>/lib/x`). (What it
/// requires of other folders isn't this folder's.)
fn requires_in(root: &str, module: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("require(") {
        rest = &rest[at + "require(".len()..];
        let Some(quote) = rest.chars().next().filter(|q| *q == '"' || *q == '\'') else { continue };
        let Some(end) = rest[1..].find(quote) else { break };
        let path = rest[1..1 + end].trim_end_matches(".luau");
        if let Some(top) = path.strip_prefix('@') {
            if let Some(p) = top.strip_prefix(root).and_then(|p| p.strip_prefix('/')) {
                out.push(p.to_string());
            }
        } else if path.starts_with("./") || path.starts_with("../") {
            let mut parts: Vec<&str> = module.split('/').collect();
            parts.pop();
            for seg in path.split('/') {
                match seg {
                    "." | "" => {}
                    ".." => {
                        parts.pop();
                    }
                    s => parts.push(s),
                }
            }
            out.push(parts.join("/"));
        }
    }
    out
}

/// The folder in `dir`'s name.
pub fn folder_name(dir: &Path) -> Result<RootManifest, String> {
    let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let m = RootManifest::named(&name);
    m.check().map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(m)
}

/// Read the content folder in `dir`.
pub fn read(dir: &Path, report: &mut Report) -> Option<Root> {
    let manifest = match folder_name(dir) {
        Ok(m) => m,
        Err(e) => {
            report.error(dir.display().to_string(), e);
            return None;
        }
    };
    let mut root = Root { manifest, dir: dir.to_path_buf(), ..Default::default() };
    let mut paths = Vec::new();
    walk(dir, dir, &mut paths);
    for rel in paths {
        if rel.starts_with("compat/") || rel.starts_with("locales/") || rel.ends_with(".d.luau") {
            continue;
        }
        let full = dir.join(&rel);
        let Some(module) = rel.strip_suffix(".luau") else {
            report.error(&rel, "a content folder holds modules and the API's definitions");
            continue;
        };
        match std::fs::read_to_string(&full) {
            Ok(text) => {
                root.modules.insert(module.to_string(), text);
            }
            Err(e) => report.error(&rel, format!("can't read: {e}")),
        }
    }
    if root.modules.is_empty() {
        report.error(dir.display().to_string(), "no modules here");
        return None;
    }
    match crate::locale::load(dir, crate::locale::OWN) {
        Ok(Some(s)) => root.strings = s,
        // (A folder of behavior only has no locales.)
        Ok(None) if !dir.join(crate::locale::DIR).is_dir() => {}
        Ok(None) => report.warn(format!("{}/{}.toml", crate::locale::DIR, crate::locale::OWN), "the content has no strings: its chips, navis and forms show by their keys"),
        Err(e) => report.error(format!("{}/{}.toml", crate::locale::DIR, crate::locale::OWN), e),
    }
    (!report.has_errors()).then_some(root)
}

/// The content folders of the content directory `content`, by name (its
/// declarations' folder, content/nettai, aside).
pub fn dirs(content: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(content).map_err(|e| format!("{}: {e} (is this a content directory?)", content.display()))?;
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| p.file_name().is_some_and(|n| n != DECLARATIONS && !n.to_string_lossy().starts_with('.')))
        .collect();
    out.sort();
    if out.is_empty() {
        return Err(format!("{}: no content folders", content.display()));
    }
    Ok(out)
}

/// Read every content folder of the content directory `content`.
pub fn read_all(content: &Path, report: &mut Report) -> Option<Vec<Root>> {
    let dirs = match dirs(content) {
        Ok(d) => d,
        Err(e) => {
            report.error(content.display().to_string(), e);
            return None;
        }
    };
    let mut roots = Vec::new();
    for d in dirs {
        roots.push(read(&d, report)?);
    }
    Some(roots)
}

/// Every file under `dir`, as paths relative to `root` with `/`, sorted
/// (hidden files left out).
fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, out);
        } else {
            let rel = path.strip_prefix(root).expect("walked under the root");
            out.push(rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"));
        }
    }
}
