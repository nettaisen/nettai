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
//! The battle content is not in a pack: it is the content directory's games
//! (crate::index, content/ in this repository), whose definitions name the
//! packs' assets. [`load_battle`] puts the two together.
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
    /// The game whose assets it holds (`exe6`).
    pub game: String,
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

/// The manifest of `game`'s pack.
pub fn manifest(name: &str, game: &str, graphics: Option<&Bundle>, sound: bool) -> (String, Vec<u8>) {
    let m = Manifest {
        format: FORMAT.into(),
        version: VERSION,
        name: name.into(),
        game: game.to_string(),
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
    if m.format != FORMAT || m.version != VERSION {
        report.error(MANIFEST, format!("not a {FORMAT} pack of version {VERSION} (extract the pack again)"));
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
/// (`sound::SongVersions`: EXE5's Team Colonel's).
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

/// The battle content, for the engine (docs/design/content-model-v2.md
/// §4.0: one game a match): the game whose assets the asset pack `assets`
/// holds, of the content directory `content` (`crate::index::content()`:
/// the game's top module and what it requires, with the support packs it
/// depends on), with the asset pack's asset index (`Content::assets`)
/// and its sprites' animation timing, defined (`Content::define`).
pub fn load_battle(content: &Path, assets: &Path) -> Result<(nettai_battle::Content, Report), Report> {
    let (mut c, mut report) = battle_content(content, assets)?;
    // The define phase: what the modules define, the tables registration
    // by number reads, and the registries.
    if let Err(e) = c.define() {
        report.error(content.display().to_string(), e.message);
        return Err(report);
    }
    Ok((c, report))
}

/// [`load_battle`]'s content before the define phase: the game's modules,
/// the asset index and the sprite timing. The game is the asset pack's
/// (its manifest's). Asset names are the pack's own (`bomb`).
pub fn battle_content(content: &Path, assets: &Path) -> Result<(nettai_battle::Content, Report), Report> {
    let mut report = Report::default();
    let Some(game) = read_manifest(assets, &mut report).map(|m| m.game) else { return Err(report) };
    let Some(read) = crate::index::read(content, std::slice::from_ref(&game), &mut report) else { return Err(report) };
    battle_content_of(read, &game, assets, report)
}

/// The battle content of what `read` read (one game), with game `game`'s
/// asset pack `pack`, before the define phase.
fn battle_content_of(
    read: crate::index::Read,
    game: &str,
    pack: &Path,
    mut report: Report,
) -> Result<(nettai_battle::Content, Report), Report> {
    if report.has_errors() {
        return Err(report);
    }
    let Some(index) = crate::names::read_index(pack, &mut report) else { return Err(report) };
    let Some(timing) = crate::timing::load(pack, &mut report) else { return Err(report) };
    let assets = nettai_content_api::AssetNames::of_packs(vec![(game.to_string(), index)]);
    let animations = animations(&assets, vec![(game.to_string(), timing)]);
    let scripts = read.scripts();
    let c = nettai_battle::Content { assets, animations, scripts, strings: read.strings, ..Default::default() };
    Ok((c, report))
}

// ---- Finding packs -------------------------------------------------------------

/// Where the extractors write their packs (`data/exe6`,
/// `data/exe5`), and where a frontend finds them.
pub const PACKS: &str = "data";

/// The packs directory: `$NETTAI_PACKS`, else `data`.
pub fn packs_dir() -> PathBuf {
    std::env::var_os("NETTAI_PACKS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(PACKS))
}

/// A pack's directory and the game whose assets it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub game: String,
    pub dir: PathBuf,
}

/// Every pack in `dir`: each folder with a pack manifest, by the game it
/// says. None in a missing `dir`.
pub fn discover(dir: &Path, report: &mut Report) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    for (_, path) in subdirs(dir) {
        if !path.join(MANIFEST).is_file() {
            continue;
        }
        let Some(m) = read_manifest(&path, report) else { continue };
        let game = m.game;
        if let Some(other) = out.iter().find(|f| f.game == game) {
            report.error(MANIFEST, format!("two packs of {game}: {} and {} (keep one, or name one with --pack)", other.dir.display(), path.display()));
            continue;
        }
        out.push(Found { game, dir: path });
    }
    out
}

/// The packs a frontend loads from: those in `dir` (the packs directory,
/// [`packs_dir`]), with each of `overrides` (`--pack`, given again for
/// another) in place of the one of its game, or beside them. None when an
/// override isn't a pack.
pub fn find(dir: &Path, overrides: &[PathBuf], report: &mut Report) -> Option<Vec<Found>> {
    let mut found = discover(dir, report);
    for path in overrides {
        let game = read_manifest(path, report)?.game;
        found.retain(|f| f.game != game);
        found.push(Found { game, dir: path.clone() });
    }
    found.sort_by(|a, b| a.game.cmp(&b.game));
    Some(found)
}

/// How to write the asset pack of `game` into `dir`, for a frontend message.
/// The shared extractor identifies any supplied ROMs by their headers.
pub fn extract_command(game: &str, dir: &Path) -> String {
    format!("cargo run --release -p nettai-extract -- {game} {} <ROM ...>", dir.join(game).display())
}

/// A game of the content a frontend can offer (docs/frontend.md §1): a
/// game pack, and its asset pack if one is found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameChoice {
    /// The game pack's id (`exe6`).
    pub game: String,
    /// Its asset pack's directory, if found.
    pub pack: Option<PathBuf>,
    /// Why it can't be played, when its asset pack isn't found: the
    /// command that writes one.
    pub why_not: Option<String>,
}

/// The games of the content directory `content` (`--content`, else
/// [`crate::index::content`]) by id, each with its asset pack among
/// `found` ([`find`]), if there is one.
pub fn games(content: Option<&Path>, found: &[Found]) -> Result<Vec<GameChoice>, Report> {
    let dir = content.map(Path::to_path_buf).unwrap_or_else(crate::index::content);
    let mut report = Report::default();
    let ids = match crate::index::games(&dir) {
        Ok(g) => g,
        Err(e) => {
            report.error(dir.display().to_string(), e);
            return Err(report);
        }
    };
    Ok(ids
        .into_iter()
        .map(|game| {
            let pack = found.iter().find(|f| f.game == game).map(|f| f.dir.clone());
            let why_not = pack.is_none().then(|| no_pack(&game, found));
            GameChoice { game, pack, why_not }
        })
        .collect())
}

/// The battle content a frontend plays ([`load_game`]): one game.
pub struct Loaded {
    pub content: nettai_battle::Content,
    /// The game (`exe6`).
    pub game: String,
    /// Its asset pack's directory: the frontend's graphics and sound.
    pub pack: PathBuf,
    /// The content directory it came from (its game pack's strings tables
    /// are a frontend's other languages: `crate::locale::load_for`).
    pub dir: PathBuf,
    pub report: Report,
}

/// Why game `game` can't load: no pack of its game is found.
fn no_pack(game: &str, found: &[Found]) -> String {
    let packs = found.iter().map(|f| format!("{} ({})", f.game, f.dir.display())).collect::<Vec<_>>();
    let had = if packs.is_empty() { "none".to_string() } else { packs.join(", ") };
    format!(
        "the game {game} draws on {game}'s assets, and no pack of {game} is found in {} (found: {had}); write it with `{}` (README.md, \"Getting started\", lists each game's ROMs), or name its directory with --pack",
        packs_dir().display(),
        extract_command(game, &packs_dir()),
    )
}

/// Load game `game` of the content directory `content` (`--content`, else
/// [`crate::index::content`]) for a frontend to play (docs/frontend.md
/// §1; docs/design/content-model-v2.md §4.0: a match plays one game): its
/// game pack, the support packs it depends on and its asset pack among `found`
/// ([`find`]), defined. A game whose asset pack isn't found is an error
/// with the command that writes it.
pub fn load_game(content: Option<&Path>, game: &str, found: &[Found]) -> Result<Loaded, Report> {
    let dir = content.map(Path::to_path_buf).unwrap_or_else(crate::index::content);
    let mut report = Report::default();
    match crate::index::games(&dir) {
        Ok(g) if g.iter().any(|x| x == game) => {}
        Ok(g) => {
            report.error(dir.display().to_string(), format!("no game {game} in the content (its games: {})", g.join(", ")));
            return Err(report);
        }
        Err(e) => {
            report.error(dir.display().to_string(), e);
            return Err(report);
        }
    }
    let Some(pack) = found.iter().find(|f| f.game == game).map(|f| f.dir.clone()) else {
        report.error(dir.display().to_string(), no_pack(game, found));
        return Err(report);
    };
    let Some(read) = crate::index::read(&dir, &[game.to_string()], &mut report) else { return Err(report) };
    let (mut content, mut report) = battle_content_of(read, game, &pack, report)?;
    if let Err(e) = content.define() {
        report.error(dir.display().to_string(), e.message);
        return Err(report);
    }
    Ok(Loaded { content, game: game.to_string(), pack, dir, report })
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A packs directory under the system's temporary one, with a pack of
    /// each `(folder, game)` (a manifest alone).
    fn packs(test: &str, packs: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nettai-content-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (folder, game) in packs {
            let (name, text) = manifest(folder, game, None, false);
            std::fs::create_dir_all(dir.join(folder)).unwrap();
            std::fs::write(dir.join(folder).join(name), text).unwrap();
        }
        // (A folder that is no pack is passed over.)
        std::fs::create_dir_all(dir.join("notes")).unwrap();
        dir
    }

    /// Every pack in the packs directory, by its game; a pack given by
    /// directory stands in for the found one of its game.
    #[test]
    fn every_pack_is_found_by_its_game() {
        let dir = packs("found", &[("exe6", "exe6"), ("five", "exe5")]);
        let mut r = Report::default();
        let found = discover(&dir, &mut r);
        let games: Vec<(&str, &str)> =
            found.iter().map(|f| (f.game.as_str(), f.dir.file_name().unwrap().to_str().unwrap())).collect();
        assert_eq!(games, [("exe6", "exe6"), ("exe5", "five")]);
        assert!(r.issues.is_empty(), "{:?}", r.issues);
        let other = packs("override", &[("mine", "exe6")]);
        let found = find(&dir, &[other.join("mine")], &mut Report::default()).unwrap();
        let exe6 = found.iter().find(|f| f.game == "exe6").unwrap();
        assert_eq!(exe6.dir, other.join("mine"));
        assert_eq!(found.len(), 2);
        // A manifest that says no game is no pack's.
        std::fs::write(dir.join("five").join(MANIFEST), "format = \"nettai-content\"\nversion = 1\nname = \"x\"\n").unwrap();
        let mut r = Report::default();
        assert_eq!(discover(&dir, &mut r).len(), 1);
        assert!(r.has_errors(), "{:?}", r.issues);
        // None in a directory that isn't there.
        assert!(discover(&dir.join("missing"), &mut Report::default()).is_empty());
        // Two packs of one game: an error.
        let two = packs("two", &[("a", "exe6"), ("b", "exe6")]);
        let mut r = Report::default();
        assert_eq!(discover(&two, &mut r).len(), 1);
        assert!(r.has_errors());
        for d in [dir, other, two] {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}
