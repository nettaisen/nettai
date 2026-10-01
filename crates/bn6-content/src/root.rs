//! A content root: the definitions and modules that make the battle
//! content, as a checkout holds them (BN6's is content/bn6 in this
//! repository; docs/design/content-model-v2.md §4).
//!
//! ```text
//! **/*.luau                       the modules: what they define (chips, navis, forms, weapons,
//!                                 stages, rules...) and the code that runs it
//! objects/NAME/object.toml        `[kind]`: the object kind a v1 module implements
//! *.d.luau                        the API's definitions, for editors and the checker
//! compat/                         the original's numbers by key: tools' data, not content
//! ```
//!
//! [`read`] reads one. The assets the definitions name (`asset.sprite`)
//! come from an extracted pack's asset index; `crate::pack::load_battle`
//! puts the two together.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bn6_battle::content::ObjectKind;
use serde::Deserialize;

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
    /// The object kinds v1 modules implement, by name.
    pub kinds: Vec<ObjectKind>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KindFile {
    kind: ObjectKind,
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
        if rel.starts_with("compat/") || rel.ends_with(".d.luau") {
            continue;
        }
        let full = dir.join(&rel);
        let folder = rel.rsplit_once('/').map_or("", |(f, _)| f).to_string();
        let text = || std::fs::read_to_string(&full).map_err(|e| format!("can't read: {e}"));
        let result: Result<(), String> = (|| {
            if let Some(module) = rel.strip_suffix(".luau") {
                root.modules.insert(module.to_string(), text()?);
            } else if rel.starts_with("objects/") && rel.ends_with("/object.toml") {
                let k: KindFile = toml::from_str(&text()?).map_err(|e| format!("invalid: {e}"))?;
                let name = folder.trim_start_matches("objects/").to_string();
                let script = script_module(&folder, &k.kind.script)?;
                root.kinds.push(ObjectKind { name, script, ..k.kind });
            } else {
                return Err("a content root holds modules, object kinds' object.toml and the API's definitions".into());
            }
            Ok(())
        })();
        if let Err(e) = result {
            report.error(&rel, e);
        }
    }
    root.kinds.sort_by(|a, b| a.name.cmp(&b.name));
    (!report.has_errors()).then_some(root)
}

/// The module a file's `script` names (a path relative to `folder`).
fn script_module(folder: &str, file: &str) -> Result<String, String> {
    let rel = file.strip_suffix(".luau").ok_or_else(|| format!("script {file:?} is not a .luau file"))?;
    let mut parts: Vec<&str> = folder.split('/').collect();
    for seg in rel.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop().ok_or_else(|| format!("script {file:?} leaves the content root"))?;
            }
            s => parts.push(s),
        }
    }
    Ok(parts.join("/"))
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
