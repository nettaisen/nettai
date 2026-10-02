//! A content root: the definitions and modules that make the battle
//! content, as a checkout holds them (BN6's is content/bn6 in this
//! repository; docs/design/content-model-v2.md §4).
//!
//! ```text
//! **/*.luau                       the modules: what they define (chips, navis, forms, weapons,
//!                                 stages, rules...) and the code that runs it
//! *.d.luau                        the API's definitions, for editors and the checker
//! compat/                         the original's numbers by key: tools' data, not content
//! locales/<language>.toml         display text by key, one table a language: the own
//!                                 language's (en) is the content's strings, the others a
//!                                 frontend's (crate::locale)
//! ```
//!
//! [`read`] reads one. The assets the definitions name (`asset.sprite`)
//! come from an extracted pack's asset index; `crate::pack::load_battle`
//! puts the two together.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::report::Report;

/// This repository's BN6 content.
const BN6: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");

/// The BN6 content root: `$BN6_CONTENT`, else this repository's
/// content/bn6.
pub fn bn6() -> PathBuf {
    std::env::var_os("BN6_CONTENT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(BN6))
}

/// What a content root holds.
#[derive(Clone, Debug, Default)]
pub struct Root {
    /// Modules by path without `.luau` (not the `.d.luau` definitions).
    pub modules: BTreeMap<String, String>,
    /// The content's own language's strings (`locales/en.toml`): its
    /// display text, which the define phase counts the chatbox's timing
    /// from (`Content::strings`).
    pub strings: crate::locale::Strings,
}

/// Read the content root in `dir`.
pub fn read(dir: &Path, report: &mut Report) -> Option<Root> {
    let mut root = Root::default();
    let mut paths = Vec::new();
    walk(dir, dir, &mut paths);
    if paths.is_empty() {
        report.error(dir.display().to_string(), "no content here");
        return None;
    }
    for rel in paths {
        if rel.starts_with("compat/") || rel.starts_with("locales/") || rel.ends_with(".d.luau") {
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
    match crate::locale::load(dir, crate::locale::OWN) {
        Ok(Some(s)) => root.strings = s,
        Ok(None) => report.warn(format!("{}/{}.toml", crate::locale::DIR, crate::locale::OWN), "the content has no strings: its chips, navis and forms show by their keys"),
        Err(e) => report.error(format!("{}/{}.toml", crate::locale::DIR, crate::locale::OWN), e),
    }
    (!report.has_errors()).then_some(root)
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
