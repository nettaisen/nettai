//! A content pack: a folder of open-format files, and the binary data the
//! engine and frontend load, derived from it.
//!
//! ```text
//! content.toml           the manifest
//! graphics/              sprites, field, backgrounds, HUD (see graphics())
//! sound/                 songs, instruments, samples (see crate::sound)
//! ```
//!
//! The pack is the source; `bn6-assets.bin` and the sound bank are caches
//! built from it ([`load_graphics`], [`load_sound`]), keyed by a hash of
//! every source file and the importer's version, and rebuilt when either
//! changes.

use crate::report::{Report, stamp};
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphicsManifest {
    /// Background ids 0..slots (folders `graphics/backgrounds/NN`; a
    /// missing folder is an id without a background).
    pub background_slots: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SoundManifest {}

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

pub fn manifest(name: &str, graphics: Option<&Bundle>, sound: bool) -> (String, Vec<u8>) {
    let m = Manifest {
        format: FORMAT.into(),
        version: VERSION,
        name: name.into(),
        graphics: graphics.map(|b| GraphicsManifest { background_slots: b.backgrounds.len() }),
        sound: sound.then_some(SoundManifest {}),
    };
    let text = format!(
        "# A content pack: open formats the engine's data is built from.\n{}",
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

pub fn export_graphics(b: &Bundle) -> Files {
    let mut files = Files::new();
    let sprites = parallel(&b.sprites, |s| {
        let dir = format!("graphics/sprites/{}", sprite::folder_name(s.category, s.index));
        sprite::export(s).into_iter().map(|(n, d)| (format!("{dir}/{n}"), d)).collect::<Files>()
    });
    files.extend(sprites.into_iter().flatten());
    files.extend(stage::export_field(&b.field).into_iter().map(|(n, d)| (format!("graphics/field/{n}"), d)));
    for (id, bg) in b.backgrounds.iter().enumerate() {
        if let Some(bg) = bg {
            files.extend(stage::export_background(bg).into_iter().map(|(n, d)| (format!("graphics/backgrounds/{id:02}/{n}"), d)));
        }
    }
    files.extend(hud::export(&b.hud).into_iter().map(|(n, d)| (format!("graphics/hud/{n}"), d)));
    files
}

fn sprite_dirs(root: &Path) -> Vec<(u8, u8, PathBuf)> {
    let mut v: Vec<(u8, u8, PathBuf)> = std::fs::read_dir(root.join("graphics/sprites"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let (c, i) = name.split_once('-')?;
            Some((u8::from_str_radix(c, 16).ok()?, u8::from_str_radix(i, 16).ok()?, e.path()))
        })
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
    let dirs = sprite_dirs(root);
    let results = parallel(&dirs, |(c, i, dir)| {
        let mut r = Report::default();
        let name = format!("graphics/sprites/{}", sprite::folder_name(*c, *i));
        let s = sprite::import(dir, &name, &mut r);
        if let Some(s) = &s
            && (s.category, s.index) != (*c, *i)
        {
            r.error(format!("{name}/sprite.json"), "the sprite id doesn't match the folder name");
        }
        (s, r)
    });
    let mut sprites = Vec::new();
    for (s, r) in results {
        report.issues.extend(r.issues);
        sprites.extend(s);
    }
    let field = stage::import_field(&root.join("graphics/field"), "graphics/field", report)?;
    let mut backgrounds = Vec::new();
    for id in 0..g.background_slots {
        let dir = root.join(format!("graphics/backgrounds/{id:02}"));
        backgrounds.push(if dir.is_dir() {
            stage::import_background(&dir, &format!("graphics/backgrounds/{id:02}"), report)
        } else {
            None
        });
    }
    let hud = hud::import(&root.join("graphics/hud"), "graphics/hud", report)?;
    if report.has_errors() {
        return None;
    }
    Some(Bundle { sprites, field, backgrounds, hud })
}

pub fn export_sound(bank: &SoundBank) -> (Files, Vec<(m4a::SongId, String)>) {
    let (files, failures) = sound::export(bank);
    (files.into_iter().map(|(n, d)| (format!("sound/{n}"), d)).collect(), failures)
}

pub fn import_sound(root: &Path, report: &mut Report) -> Option<SoundBank> {
    read_manifest(root, report)?;
    let bank = sound::import(&root.join("sound"), "sound", report)?;
    (!report.has_errors()).then_some(bank)
}

// ---- Derived caches -----------------------------------------------------------------

/// A hash of every file under `dir` (paths and contents) and the importer.
pub fn source_key(dir: &Path) -> String {
    let mut files = Vec::new();
    fn walk(d: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) {
        for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, base, out);
            } else if let Ok(rel) = p.strip_prefix(base) {
                out.push((rel.to_string_lossy().replace('\\', "/"), p.clone()));
            }
        }
    }
    walk(dir, dir, &mut files);
    files.sort();
    let mut all = format!("{} {}\n", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).into_bytes();
    for (rel, p) in files {
        let data = std::fs::read(&p).unwrap_or_default();
        all.extend_from_slice(format!("{rel} {}\n", stamp(&data)).as_bytes());
    }
    stamp(&all)
}

/// Load `name` from `cache_dir` if it was built from the current sources,
/// else build it and store it (dropping older builds).
fn cached<T>(
    source: &Path,
    cache_dir: &Path,
    name: &str,
    build: impl FnOnce(&mut Report) -> Option<T>,
    to_bytes: impl Fn(&T) -> Vec<u8>,
    from_bytes: impl Fn(&[u8]) -> Option<T>,
) -> Result<(T, Report), Report> {
    let key = source_key(source);
    let file = cache_dir.join(format!("{name}-{key}.bin"));
    if let Ok(bytes) = std::fs::read(&file)
        && let Some(v) = from_bytes(&bytes)
    {
        return Ok((v, Report::default()));
    }
    let mut report = Report::default();
    let Some(v) = build(&mut report) else { return Err(report) };
    if std::fs::create_dir_all(cache_dir).is_ok() {
        for e in std::fs::read_dir(cache_dir).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with(&format!("{name}-")) && n.ends_with(".bin") {
                let _ = std::fs::remove_file(e.path());
            }
        }
        let _ = std::fs::write(&file, to_bytes(&v));
    }
    Ok((v, report))
}

/// The graphics bundle of a pack, from the cache when it is current.
pub fn load_graphics(root: &Path, cache_dir: &Path) -> Result<(Bundle, Report), Report> {
    cached(
        &root.join("graphics"),
        cache_dir,
        "graphics",
        |r| import_graphics(root, r),
        |b| b.to_bytes(),
        |d| Bundle::from_bytes(d).ok(),
    )
}

/// The sound bank of a pack, from the cache when it is current.
pub fn load_sound(root: &Path, cache_dir: &Path) -> Result<(SoundBank, Report), Report> {
    cached(
        &root.join("sound"),
        cache_dir,
        "sound",
        |r| import_sound(root, r),
        |b| b.to_bytes(),
        |d| SoundBank::from_bytes(d).ok(),
    )
}
