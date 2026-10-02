//! A content pack: a folder of open-format files holding a game's assets,
//! which loads into the data the engine, the frontend and the audio use.
//!
//! ```text
//! content.toml           the manifest
//! assets.toml            the asset index: every asset by name (crate::names)
//! graphics/              sprites (with their animation timing), field, backgrounds, HUD
//! sound/                 songs, instruments, samples (see crate::sound)
//! ```
//!
//! Sprites, backgrounds, songs and the HUD's mugshots, banners and chip
//! icons are written under their names ([`crate::names`]); each file holds
//! its number, which the importers read.
//!
//! The battle content is not in a pack: it is a content root's definitions
//! (crate::root, content/bn6 in this repository), which name the pack's
//! assets. [`load_battle`] puts the two together.
//!
//! Everything loads straight from these files ([`load_battle`],
//! [`import_graphics`], [`import_sound`]); there is no derived binary.

use crate::names::AssetNames;
use crate::report::Report;
use crate::{custom, hud, sound, sprite, stage};
use nettai_assets::Bundle;
use m4a::SoundBank;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MANIFEST: &str = "content.toml";
pub const FORMAT: &str = "nettai-content";
pub const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub name: String,
    /// The game whose assets it holds (`bn6`): the name a content root's
    /// `assets` gives it (docs/design/rules-in-luau.md §7.2). A pack
    /// written before packs said their game has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graphics: Option<GraphicsManifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<SoundManifest>,
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
    manifest_of(name, None, graphics, sound)
}

/// A pack's manifest that says its game.
pub fn manifest_of(name: &str, game: Option<&str>, graphics: Option<&Bundle>, sound: bool) -> (String, Vec<u8>) {
    let m = Manifest {
        format: FORMAT.into(),
        version: VERSION,
        name: name.into(),
        game: game.map(str::to_string),
        graphics: graphics.map(|b| GraphicsManifest { background_slots: b.backgrounds.len() }),
        sound: sound.then_some(SoundManifest {}),
    };
    let text = format!(
        "# A content pack: a game's graphics and sound in open formats, by name.\n{}",
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
    files.extend(custom::export(&b.custom).into_iter().map(|(n, d)| (format!("graphics/custom/{n}"), d)));
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
    let mut sprites: Vec<nettai_assets::SpriteSheet> = Vec::new();
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
    let mut backgrounds: Vec<Option<nettai_assets::Background>> = vec![None; g.background_slots];
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
    let custom = custom::import(&root.join("graphics/custom"), "graphics/custom", report)?;
    if report.has_errors() {
        return None;
    }
    Some(Bundle { sprites, field, backgrounds, hud, custom })
}

pub fn export_sound(bank: &SoundBank, names: &AssetNames) -> (Files, Vec<(m4a::SongId, String)>) {
    let (files, failures) = sound::export(bank, names);
    (files.into_iter().map(|(n, d)| (format!("sound/{n}"), d)).collect(), failures)
}

pub fn import_sound(root: &Path, report: &mut Report) -> Option<SoundBank> {
    import_sound_versions(root, report).map(|(bank, _)| bank)
}

/// [`export_sound`], with the songs game versions have their own of
/// (`sound::SongVersions`: BN5's Team Colonel's).
pub fn export_sound_versions(
    bank: &SoundBank,
    versions: &sound::SongVersions,
    names: &AssetNames,
) -> (Files, Vec<(m4a::SongId, String)>) {
    let (files, failures) = sound::export_with_versions(bank, versions, names);
    (files.into_iter().map(|(n, d)| (format!("sound/{n}"), d)).collect(), failures)
}

/// [`import_sound`], with the songs game versions have their own of.
pub fn import_sound_versions(root: &Path, report: &mut Report) -> Option<(SoundBank, sound::SongVersions)> {
    read_manifest(root, report)?;
    let got = sound::import_with_versions(&root.join("sound"), "sound", report)?;
    (!report.has_errors()).then_some(got)
}

// ---- Loading -------------------------------------------------------------------

/// The battle content, for the engine: the definitions and modules of the
/// content root `content` (`crate::root::bn6()` for BN6's) and the roots it
/// requires, with the assets of the pack `assets` (its asset index,
/// `Content::assets`, and its sprites' animation timing), defined
/// (`Content::define`). [`load_battle_packs`] loads several packs.
pub fn load_battle(content: &Path, assets: &Path) -> Result<(nettai_battle::Content, Report), Report> {
    load_battle_packs(content, &[assets.to_path_buf()])
}

/// [`load_battle`] with the assets of several packs (docs/design/
/// rules-in-luau.md §7.4): one a game, each root's `assets` among them.
pub fn load_battle_packs(content: &Path, packs: &[PathBuf]) -> Result<(nettai_battle::Content, Report), Report> {
    let (mut c, mut report) = battle_content_packs(content, packs)?;
    // The define phase: what the modules define, the tables registration
    // by number reads, and the registries.
    if let Err(e) = c.define() {
        report.error(content.display().to_string(), e.message);
        return Err(report);
    }
    Ok((c, report))
}

/// [`load_battle`]'s content before the define phase: the modules of the
/// root in `content` and of the roots it requires, the asset index and the
/// sprite timing.
pub fn battle_content(content: &Path, assets: &Path) -> Result<(nettai_battle::Content, Report), Report> {
    battle_content_packs(content, &[assets.to_path_buf()])
}

/// [`battle_content`] with several packs: each says its game (a pack that
/// says none is taken as the content's own root's assets, with a warning);
/// two packs of one game are refused, and every root's `assets` must be
/// loaded. Asset names are qualified with their pack's game (`bn6:bomb`).
pub fn battle_content_packs(content: &Path, packs: &[PathBuf]) -> Result<(nettai_battle::Content, Report), Report> {
    let mut report = Report::default();
    let Some(roots) = crate::root::read_all(content, &mut report) else { return Err(report) };
    let home_assets = roots.first().map(|r| r.manifest.assets().to_string()).unwrap_or_default();
    let Some(games) = pack_games(packs, &home_assets, &mut report) else { return Err(report) };
    for r in &roots {
        if !games.iter().any(|(g, _)| g == r.manifest.assets()) {
            report.error(
                crate::root::MANIFEST,
                format!("root {}'s assets are {}'s, and no pack of {} is loaded", r.manifest.name, r.manifest.assets(), r.manifest.assets()),
            );
        }
    }
    if report.has_errors() {
        return Err(report);
    }
    let mut indices = Vec::new();
    let mut timings = Vec::new();
    for (game, path) in &games {
        let Some(index) = crate::names::read_index(path, &mut report) else { return Err(report) };
        let Some(timing) = crate::timing::load(path, &mut report) else { return Err(report) };
        indices.push((game.clone(), index));
        timings.push((game.clone(), timing));
    }
    let assets = nettai_content_api::AssetNames::of_packs(indices);
    let animations = animations(&assets, timings);
    let mut scripts = nettai_battle::content::Scripts::default();
    let mut strings = crate::locale::Strings::default();
    for r in roots {
        strings.merge(r.strings.qualified(&r.manifest.name));
        scripts.add_root(r.manifest, r.modules);
    }
    let c = nettai_battle::Content { assets, animations, scripts, strings, ..Default::default() };
    Ok((c, report))
}

/// Each pack's game (a pack that says none is taken as `home_assets`, with
/// a warning), refusing two packs of one game; none when a manifest can't
/// be read.
fn pack_games(packs: &[PathBuf], home_assets: &str, report: &mut Report) -> Option<Vec<(String, PathBuf)>> {
    let mut games: Vec<(String, PathBuf)> = Vec::new();
    for path in packs {
        let m = read_manifest(path, report)?;
        let game = match m.game {
            Some(g) => g,
            None => {
                report.warn(
                    MANIFEST,
                    format!("{}: the pack says no game; it is taken as {home_assets}'s (extract it again to record its game)", path.display()),
                );
                home_assets.to_string()
            }
        };
        if games.iter().any(|(g, _)| *g == game) {
            report.error(MANIFEST, format!("two packs of {game} are loaded"));
        }
        games.push((game, path.clone()));
    }
    Some(games)
}

/// The directories of the packs `c` was loaded from (`packs`, as given to
/// [`load_battle_packs`]) in its pack order, by `PackId`: the frontend's
/// graphics and sound of each.
pub fn pack_paths(c: &nettai_battle::Content, packs: &[PathBuf]) -> Vec<PathBuf> {
    let home_assets = c.scripts.roots.first().map(|r| r.assets().to_string()).unwrap_or_default();
    let games = pack_games(packs, &home_assets, &mut Report::default()).unwrap_or_default();
    c.assets.packs.iter().filter_map(|g| games.iter().find(|(game, _)| game == g).map(|(_, p)| p.clone())).collect()
}

/// Every sprite's animation timing, from its pack's `animations.json`s, and
/// its frames' part offsets, from its `sprite.json`s: by sprite handle (each
/// name a pack gives a sprite is a handle of it).
fn animations(assets: &nettai_content_api::AssetNames, timings: Vec<(String, crate::timing::Timing)>) -> nettai_battle::content::Animations {
    use nettai_battle::content::{AnimFrame, PackSprite, SpriteParts};
    let mut out = nettai_battle::content::Animations::default();
    for (game, t) in timings {
        let Some(pack) = assets.pack(&game) else { continue };
        let mut sprites = std::collections::BTreeMap::new();
        let mut parts = std::collections::BTreeMap::new();
        for (&(category, index), anims) in &t.sprites {
            let id = PackSprite { category, index };
            // (A sprite without its `sprite.json` has no parts.)
            if let Some(layouts) = t.layouts.get(&(category, index)) {
                let layouts = layouts.iter().map(|l| l.iter().map(|p| (p[0], p[1])).collect()).collect();
                let frame_layouts = anims.iter().map(|a| a.iter().map(|f| f.layout).collect()).collect();
                parts.insert(id, SpriteParts { frame_layouts, layouts });
            }
            let anims: Vec<Vec<AnimFrame>> =
                anims.iter().map(|a| a.iter().map(|f| AnimFrame { duration: f.ticks, flags: f.flags }).collect()).collect();
            sprites.insert(id, anims);
        }
        out.add_pack(assets, pack, &sprites, &parts);
    }
    out
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
