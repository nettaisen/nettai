//! A content pack: a folder of open-format files that loads into the data
//! the engine, the frontend and the audio use.
//!
//! ```text
//! content.toml           the manifest
//! chips/ navis/ objects/ rules/ registries/   the battle data (see crate::battle)
//! graphics/              sprites, field, backgrounds, HUD
//! sound/                 songs, instruments, samples (see crate::sound)
//! ```
//!
//! Sprites, backgrounds, songs and the HUD's mugshots, banners and chip
//! icons are written under their names ([`crate::names`]); each file holds
//! its number, which the importers read.
//!
//! Everything loads straight from these files ([`load_battle`],
//! [`import_graphics`], [`import_sound`]); there is no derived binary.

use crate::names::AssetNames;
use crate::report::Report;
use crate::{hud, sound, sprite, stage};
use bn6_assets::Bundle;
use m4a::SoundBank;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MANIFEST: &str = "content.toml";
pub const FORMAT: &str = "bn6-content";
pub const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graphics: Option<GraphicsManifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<SoundManifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battle: Option<BattleManifest>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphicsManifest {
    /// Background ids 0..slots (a folder of `graphics/backgrounds` each,
    /// whose `background.json` gives its id; an id without a folder has no
    /// background).
    pub background_slots: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SoundManifest {}

/// The pack has battle data (chips, navis, rules... see crate::battle).
#[derive(Serialize, Deserialize, Debug)]
pub struct BattleManifest {}

/// Files as (path in the pack, contents).
pub type Files = Vec<(String, Vec<u8>)>;

pub fn write_files(root: &Path, files: &Files) -> std::io::Result<()> {
    for (path, data) in files {
        let p = root.join(path);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, data)?;
    }
    Ok(())
}

pub fn manifest(name: &str, graphics: Option<&Bundle>, sound: bool, battle: bool) -> (String, Vec<u8>) {
    let m = Manifest {
        format: FORMAT.into(),
        version: VERSION,
        name: name.into(),
        graphics: graphics.map(|b| GraphicsManifest { background_slots: b.backgrounds.len() }),
        sound: sound.then_some(SoundManifest {}),
        battle: battle.then_some(BattleManifest {}),
    };
    let text = format!(
        "# A content pack: a battle's data, graphics and sound in open formats.\n{}",
        toml::to_string_pretty(&m).unwrap()
    );
    (MANIFEST.into(), text.into_bytes())
}

pub fn read_manifest(root: &Path, report: &mut Report) -> Option<Manifest> {
    let text = match std::fs::read_to_string(root.join(MANIFEST)) {
        Ok(t) => t,
        Err(e) => {
            report.error(MANIFEST, format!("can't read: {e} (is this a content pack?)"));
            return None;
        }
    };
    let m: Manifest = match toml::from_str(&text) {
        Ok(m) => m,
        Err(e) => {
            report.error(MANIFEST, format!("invalid: {e}"));
            return None;
        }
    };
    if m.format != FORMAT || m.version > VERSION {
        report.error(MANIFEST, format!("not a {FORMAT} pack of version {VERSION} or older"));
        return None;
    }
    Some(m)
}

// ---- Graphics -------------------------------------------------------------------

/// Run `f` over `items` on all cores, keeping order.
fn parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = std::thread::available_parallelism().map_or(4, |n| n.get()).min(items.len().max(1));
    let chunk = items.len().div_ceil(n).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = items.chunks(chunk).map(|c| s.spawn(|| c.iter().map(&f).collect::<Vec<R>>())).collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

pub fn export_graphics(b: &Bundle, names: &AssetNames) -> Files {
    let mut files = Files::new();
    let sprites = parallel(&b.sprites, |s| {
        let dir = format!("graphics/sprites/{}", names.sprite(s.category, s.index));
        sprite::export(s).into_iter().map(|(n, d)| (format!("{dir}/{n}"), d)).collect::<Files>()
    });
    files.extend(sprites.into_iter().flatten());
    files.extend(stage::export_field(&b.field).into_iter().map(|(n, d)| (format!("graphics/field/{n}"), d)));
    for (id, bg) in b.backgrounds.iter().enumerate() {
        if let Some(bg) = bg {
            let dir = format!("graphics/backgrounds/{}", names.background(id as u8));
            files.extend(stage::export_background(bg, id as u8).into_iter().map(|(n, d)| (format!("{dir}/{n}"), d)));
        }
    }
    files.extend(hud::export(&b.hud, names).into_iter().map(|(n, d)| (format!("graphics/hud/{n}"), d)));
    files
}

/// The folders under `dir`, by name.
fn subdirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| Some((e.file_name().into_string().ok()?, e.path())))
        .collect();
    v.sort();
    v
}

pub fn import_graphics(root: &Path, report: &mut Report) -> Option<Bundle> {
    let m = read_manifest(root, report)?;
    let Some(g) = m.graphics else {
        report.error(MANIFEST, "the pack has no graphics");
        return None;
    };
    let dirs: Vec<(String, PathBuf)> =
        subdirs(&root.join("graphics/sprites")).into_iter().filter(|(_, d)| d.join("sprite.json").is_file()).collect();
    let results = parallel(&dirs, |(folder, dir)| {
        let mut r = Report::default();
        let s = sprite::import(dir, &format!("graphics/sprites/{folder}"), &mut r);
        (folder.clone(), s, r)
    });
    let mut sprites: Vec<bn6_assets::SpriteSheet> = Vec::new();
    for (folder, s, r) in results {
        report.issues.extend(r.issues);
        if let Some(s) = s {
            if let Some(other) = sprites.iter().find(|o| (o.category, o.index) == (s.category, s.index)) {
                let id = sprite::folder_name(other.category, other.index);
                report.error(format!("graphics/sprites/{folder}/sprite.json"), format!("another folder is sprite {id} too"));
                continue;
            }
            sprites.push(s);
        }
    }
    sprites.sort_by_key(|s| (s.category, s.index));
    let field = stage::import_field(&root.join("graphics/field"), "graphics/field", report)?;
    let mut backgrounds: Vec<Option<bn6_assets::Background>> = vec![None; g.background_slots];
    for (folder, dir) in subdirs(&root.join("graphics/backgrounds")) {
        let name = format!("graphics/backgrounds/{folder}");
        let Some((id, bg)) = stage::import_background(&dir, &name, report) else { continue };
        match backgrounds.get_mut(id as usize) {
            Some(slot @ None) => *slot = Some(bg),
            Some(Some(_)) => report.error(format!("{name}/background.json"), format!("another folder is background {id} too")),
            None => report.error(format!("{name}/background.json"), format!("background {id} is past the manifest's {} slots", g.background_slots)),
        }
    }
    let hud = hud::import(&root.join("graphics/hud"), "graphics/hud", report)?;
    if report.has_errors() {
        return None;
    }
    Some(Bundle { sprites, field, backgrounds, hud })
}

pub fn export_sound(bank: &SoundBank, names: &AssetNames) -> (Files, Vec<(m4a::SongId, String)>) {
    let (files, failures) = sound::export(bank, names);
    (files.into_iter().map(|(n, d)| (format!("sound/{n}"), d)).collect(), failures)
}

pub fn import_sound(root: &Path, report: &mut Report) -> Option<SoundBank> {
    read_manifest(root, report)?;
    let bank = sound::import(&root.join("sound"), "sound", report)?;
    (!report.has_errors()).then_some(bank)
}

// ---- Loading -------------------------------------------------------------------

/// A pack's battle data, for the engine: loaded and defined
/// (`Content::define`).
pub fn load_battle(root: &Path) -> Result<(bn6_battle::Content, Report), Report> {
    let mut report = Report::default();
    let m = read_manifest(root, &mut report).ok_or_else(|| report.clone())?;
    if m.battle.is_none() {
        report.error(MANIFEST, "the pack has no battle data");
        return Err(report);
    }
    let Some(mut c) = crate::battle::load(root, &mut report) else { return Err(report) };
    // The define phase: what the scripts define, and the registries.
    if let Err(e) = c.define() {
        report.error("scripts", e.message);
        return Err(report);
    }
    Ok((c, report))
}

/// A pack's graphics, for a frontend.
pub fn load_graphics(root: &Path) -> Result<(Bundle, Report), Report> {
    let mut report = Report::default();
    match import_graphics(root, &mut report) {
        Some(b) => Ok((b, report)),
        None => Err(report),
    }
}

/// A pack's sound, for the audio.
pub fn load_sound(root: &Path) -> Result<(SoundBank, Report), Report> {
    let mut report = Report::default();
    match import_sound(root, &mut report) {
        Some(b) => Ok((b, report)),
        None => Err(report),
    }
}
