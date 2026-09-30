//! A source overlay: hand-written content that goes into a pack next to
//! extracted data. BN6's is content/bn6 in this repository: the Luau
//! scripts that implement BN6's chips, weapons and object kinds (this
//! project's port of the game's routines, not ROM data), the files that
//! register them, and the API definitions for editors. It mirrors the
//! pack's layout:
//!
//! ```text
//! chips/NNN-name/chip.toml        only `script = "..."`: the chip's script
//! chips/NNN-name/*.luau
//! objects/NAME/object.toml        only `[kind]`: the object kind a script implements
//! objects/NAME/*.luau
//! navis/00-megaman/weapons/NN-name/weapon.toml   a weapon routine's script
//! lib/*.luau                      helpers scripts share
//! *.d.luau                        the API's definitions, copied as they are
//! ```
//!
//! Its compat/ folder (the original's numbers by content key,
//! docs/design/content-model-v2.md §6) is not content: no pack holds it.
//!
//! [`read`] reads one, [`Overlay::apply`] adds it to extracted content (a
//! chip's `script`, the kinds, the weapons, the modules), and
//! [`Overlay::files`] are the files copied as they are. `bn6-extract
//! content` does both.

use std::collections::BTreeMap;
use std::path::Path;

use bn6_battle::content::{Content, ObjectKind, WeaponData};
use serde::Deserialize;

use crate::battle::{chip_folder, script_module};
use crate::pack::Files;
use crate::report::Report;

/// What an overlay adds to a pack.
#[derive(Clone, Debug, Default)]
pub struct Overlay {
    /// Chips' scripts (module paths) by chip folder (`chips/00f-gundels1`).
    chip_scripts: Vec<(String, String)>,
    kinds: Vec<ObjectKind>,
    weapons: Vec<WeaponData>,
    modules: BTreeMap<String, String>,
    files: Files,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChipScript {
    script: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KindFile {
    kind: ObjectKind,
}

/// Read the overlay in `dir`.
pub fn read(dir: &Path, report: &mut Report) -> Option<Overlay> {
    let mut o = Overlay::default();
    let mut paths = Vec::new();
    walk(dir, dir, &mut paths);
    if paths.is_empty() {
        report.error(dir.display().to_string(), "no overlay here");
        return None;
    }
    for rel in paths {
        if rel.starts_with("compat/") {
            continue;
        }
        let full = dir.join(&rel);
        let folder = rel.rsplit_once('/').map_or("", |(f, _)| f).to_string();
        let text = || std::fs::read_to_string(&full).map_err(|e| format!("can't read: {e}"));
        let result: Result<(), String> = (|| {
            if let Some(module) = rel.strip_suffix(".luau").filter(|m| !m.ends_with(".d")) {
                o.modules.insert(module.to_string(), text()?);
            } else if rel.ends_with(".d.luau") {
                o.files.push((rel.clone(), text()?.into_bytes()));
            } else if rel.starts_with("chips/") && rel.ends_with("/chip.toml") {
                let c: ChipScript = toml::from_str(&text()?).map_err(|e| format!("invalid: {e}"))?;
                o.chip_scripts.push((folder.clone(), script_module(&folder, &c.script)?));
            } else if rel.starts_with("objects/") && rel.ends_with("/object.toml") {
                let k: KindFile = toml::from_str(&text()?).map_err(|e| format!("invalid: {e}"))?;
                let name = folder.trim_start_matches("objects/").to_string();
                let script = script_module(&folder, &k.kind.script)?;
                o.kinds.push(ObjectKind { name, script, ..k.kind });
            } else if rel.starts_with("navis/00-megaman/weapons/") && rel.ends_with("/weapon.toml") {
                let mut w: WeaponData = toml::from_str(&text()?).map_err(|e| format!("invalid: {e}"))?;
                w.script = script_module(&folder, &w.script)?;
                o.weapons.push(w);
            } else {
                return Err("an overlay holds scripts, the files that register them and the API's definitions".into());
            }
            Ok(())
        })();
        if let Err(e) = result {
            report.error(&rel, e);
        }
    }
    (!report.has_errors()).then_some(o)
}

/// Every file under `dir`, as paths relative to `root` with `/`, sorted.
fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk(root, &path, out);
        } else if !path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
            let rel = path.strip_prefix(root).expect("walked under the root");
            out.push(rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"));
        }
    }
}

impl Overlay {
    /// Add the overlay to `content`: each chip's script, the object kinds,
    /// the weapons and the modules. A chip folder the content doesn't have,
    /// or a kind or weapon it already has, is an error.
    pub fn apply(&self, content: &mut Content, report: &mut Report) {
        for (folder, module) in &self.chip_scripts {
            match content.chips.iter_mut().find(|c| chip_folder(c) == *folder) {
                Some(c) => c.script = Some(module.clone()),
                None => report.error(format!("{folder}/chip.toml"), "the content has no chip with this folder"),
            }
        }
        for k in &self.kinds {
            if content.objects.kinds.iter().any(|o| o.name == k.name) {
                report.error(format!("objects/{}/object.toml", k.name), "the content already has this kind");
                continue;
            }
            content.objects.kinds.push(k.clone());
        }
        content.objects.kinds.sort_by(|a, b| a.name.cmp(&b.name));
        for w in &self.weapons {
            if content.weapons.iter().any(|o| o.id == w.id) {
                report.error(format!("weapon {:#04x}", w.id), "the content already has this weapon routine");
                continue;
            }
            content.weapons.push(w.clone());
        }
        content.weapons.sort_by_key(|w| w.id);
        content.scripts.modules.extend(self.modules.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    /// The files copied into a pack as they are (the API's definitions).
    pub fn files(&self) -> &Files {
        &self.files
    }
}
