//! A content root: the definitions and modules that make the battle
//! content, as a checkout holds them (BN6's is content/bn6 in this
//! repository; docs/design/content-model-v2.md §4, docs/design/
//! rules-in-luau.md §7.2).
//!
//! ```text
//! root.toml                       the manifest: its name (its namespace: the keys its
//!                                 modules define are qualified with it, `bn6:minibomb`),
//!                                 whose pack its asset names resolve in, the roots it requires
//! **/*.luau                       the modules: what they define (chips, navis, forms, weapons,
//!                                 stages, rules...) and the code that runs it
//! *.d.luau                        its own API definitions, for editors and the checker (the
//!                                 engine's are content/nettai's)
//! compat/                         the original's numbers by key: tools' data, not content
//! locales/<language>.toml         display text by key, one table a language: the own
//!                                 language's (en) is the content's strings, the others a
//!                                 frontend's (crate::locale)
//! ```
//!
//! [`read`] reads one, [`read_all`] one and the roots it requires (each a
//! sibling directory named as the root: content/bn6 beside content/mix).
//! The assets the definitions name (`asset.sprite`) come from an extracted
//! pack's asset index; `crate::pack::load_battle` puts the two together.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use nettai_battle::content::RootManifest;

use crate::report::Report;

/// This repository's BN6 content.
const BN6: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");

/// The manifest's file in a root.
pub const MANIFEST: &str = "root.toml";

/// The BN6 content root: `$BN6_CONTENT`, else this repository's
/// content/bn6.
pub fn bn6() -> PathBuf {
    std::env::var_os("BN6_CONTENT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(BN6))
}

/// The engine's API declarations for content (content/nettai beside the
/// roots): `dir`'s sibling `nettai`.
pub fn engine_declarations(dir: &Path) -> PathBuf {
    dir.parent().unwrap_or(Path::new(".")).join("nettai")
}

/// What a content root holds.
#[derive(Clone, Debug, Default)]
pub struct Root {
    pub manifest: RootManifest,
    /// Where it was read from.
    pub dir: PathBuf,
    /// Modules by path in the root, without `.luau` (not the `.d.luau`
    /// definitions).
    pub modules: BTreeMap<String, String>,
    /// The content's own language's strings (`locales/en.toml`) as the
    /// root writes them (unqualified): its display text, which the define
    /// phase counts the chatbox's timing from (`Content::strings`).
    pub strings: crate::locale::Strings,
}

/// The manifest of the root in `dir`.
pub fn read_manifest(dir: &Path) -> Result<RootManifest, String> {
    let path = dir.join(MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e} (is this a content root?)", path.display()))?;
    let m: RootManifest = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    m.check().map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(m)
}

/// Read the content root in `dir`.
pub fn read(dir: &Path, report: &mut Report) -> Option<Root> {
    let manifest = match read_manifest(dir) {
        Ok(m) => m,
        Err(e) => {
            report.error(MANIFEST, e);
            return None;
        }
    };
    let mut root = Root { manifest, dir: dir.to_path_buf(), ..Default::default() };
    let mut paths = Vec::new();
    walk(dir, dir, &mut paths);
    for rel in paths {
        if rel.starts_with("compat/") || rel.starts_with("locales/") || rel.ends_with(".d.luau") || rel == MANIFEST {
            continue;
        }
        let full = dir.join(&rel);
        let Some(module) = rel.strip_suffix(".luau") else {
            report.error(&rel, "a content root holds modules and the API's definitions");
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
        Ok(None) => report.warn(format!("{}/{}.toml", crate::locale::DIR, crate::locale::OWN), "the content has no strings: its chips, navis and forms show by their keys"),
        Err(e) => report.error(format!("{}/{}.toml", crate::locale::DIR, crate::locale::OWN), e),
    }
    (!report.has_errors()).then_some(root)
}

/// The directories of the root in `dir` and the roots it requires, its
/// own first, each required root once (a required root is the sibling
/// directory of its name).
pub fn dirs(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(d) = pending.pop() {
        let m = read_manifest(&d)?;
        if let Some((_, other)) = out.iter().find(|(n, _)| *n == m.name) {
            if other != &d {
                return Err(format!("two roots are named {}: {} and {}", m.name, other.display(), d.display()));
            }
            continue;
        }
        let parent = d.parent().unwrap_or(Path::new(".")).to_path_buf();
        for r in m.requires.iter().rev() {
            if !out.iter().any(|(n, _)| n == r) {
                pending.push(parent.join(r));
            }
        }
        out.push((m.name, d));
    }
    Ok(out.into_iter().map(|(_, d)| d).collect())
}

/// The directories of the roots in `dirs` and the roots they require, each
/// root once, the first's own first (the home).
pub fn dirs_of(dirs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    for d in dirs {
        for d in self::dirs(d)? {
            let name = read_manifest(&d)?.name;
            match out.iter().find(|(n, _)| *n == name) {
                Some((_, other)) if *other != d => {
                    return Err(format!("two roots are named {name}: {} and {}", other.display(), d.display()));
                }
                Some(_) => {}
                None => out.push((name, d)),
            }
        }
    }
    Ok(out.into_iter().map(|(_, d)| d).collect())
}

/// The roots beside the root in `dir` (the other folders of its parent
/// with a root manifest), each with its manifest, by folder name.
pub fn beside(dir: &Path) -> Vec<(RootManifest, PathBuf)> {
    let parent = dir.parent().unwrap_or(Path::new("."));
    let own = std::fs::canonicalize(dir).ok();
    let mut out: Vec<(RootManifest, PathBuf)> = std::fs::read_dir(parent)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join(MANIFEST).is_file() && std::fs::canonicalize(p).ok() != own)
        .filter_map(|p| read_manifest(&p).ok().map(|m| (m, p)))
        .collect();
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

/// Read the content root in `dir` and every root it requires, its own
/// first.
pub fn read_all(dir: &Path, report: &mut Report) -> Option<Vec<Root>> {
    read_many(&[dir.to_path_buf()], report)
}

/// Read the content roots in `dirs` and every root they require, each
/// once, the first's own first (the home: `dirs_of`).
pub fn read_many(dirs: &[PathBuf], report: &mut Report) -> Option<Vec<Root>> {
    let dirs = match dirs_of(dirs) {
        Ok(d) => d,
        Err(e) => {
            report.error(MANIFEST, e);
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
