//! A content pack: a folder of open-format files holding a game's assets,
//! which loads into the data the engine, the frontend and the audio use.
//!
//! ```text
//! content.toml           the manifest
//! assets.toml            the asset index: every asset by name (crate::names)
//! graphics/              sprites (with the frames of their animations), field, backgrounds, HUD
//! sound/                 songs, instruments, samples (see crate::sound)
//! ```
//!
//! Sprites, backgrounds, songs and the HUD's mugshots, banners and chip
//! icons are written under their names ([`crate::names`]); each file holds
//! its number, which the importers read.
//!
//! The battle content is not in a pack: it is the content directory's games
//! (crate::index, content/ in this repository), whose modules name the
//! assets and define the sprites' timing. [`load_battle`] loads a game's
//! content alone; [`pack_assets`] says what a pack holds of it, for the
//! frontend that draws it and the audio that plays it.
//!
//! Everything loads straight from these files ([`load_battle`],
//! [`import_graphics`], [`import_sound`]); there is no derived binary.

use crate::names::AssetNames;
use crate::report::Report;
use crate::{custom, hud, sound, sprite, stage};
use nettai_assets::Bundle;
use nettai_content_api::PackAssets;
use m4a::SoundBank;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MANIFEST: &str = "content.toml";
pub const FORMAT: &str = "nettai-content";
pub const VERSION: u32 = 1;

/// Every format a pack's files are written in, with its version: the
/// manifest's, then its parts'. Each file says its own, and a reader refuses
/// another (a note to extract the pack again).
pub const FORMATS: [(&str, u32); 7] = [
    (FORMAT, VERSION),
    (sprite::FORMAT, sprite::VERSION),
    (sound::FORMAT, sound::VERSION),
    (custom::FORMAT, custom::VERSION),
    (hud::FORMAT, hud::VERSION),
    (stage::FIELD_FORMAT, stage::VERSION),
    (stage::BACKGROUND_FORMAT, stage::VERSION),
];

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
        write_file(root, path, data)?;
    }
    Ok(())
}

/// One of a pack's files ([`write_files`] writes each so), its folders made as it needs them.
pub fn write_file(root: &Path, path: &str, data: &[u8]) -> std::io::Result<()> {
    let p = root.join(path);
    if let Some(d) = p.parent() {
        nettai_platform::fs::create_dir_all(d)?;
    }
    nettai_platform::fs::write(p, data)
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
    let text = match nettai_platform::fs::read_to_string(root.join(MANIFEST)) {
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

// ---- What was read ------------------------------------------------------------

thread_local! {
    /// How many times this thread read a part of a pack or of the content
    /// from disk ([`reads`]).
    static READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many times this thread has read a part of a pack or of the content
/// from its files so far: a pack's graphics ([`import_graphics`]), its
/// sound ([`import_sound_versions`]), a game's battle content
/// ([`battle_content`], [`load_game`]) or a language's strings
/// (`crate::locale::load_for`). Each is read whole, by a loader a host
/// calls: nothing reads a file as a battle runs, so a host that loaded its
/// game before a match sees the count stay where it was through the match
/// (docs/frontend.md §9).
pub fn reads() -> u64 {
    READS.with(|n| n.get())
}

/// Count a part read on this thread.
pub(crate) fn read_a_part() {
    READS.with(|n| n.set(n.get() + 1));
}

// ---- Graphics -------------------------------------------------------------------

/// Run `f` over `items` on all cores, keeping order.
fn parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    // (The browser's threads are Web Workers, started from the event loop: this runs on one already, the
    // extraction's, and goes through the items itself.)
    if cfg!(target_arch = "wasm32") {
        return items.iter().map(f).collect();
    }
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
    let mut v: Vec<(String, PathBuf)> = nettai_platform::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| Some((e.file_name().into_string().ok()?, e.path())))
        .collect();
    v.sort();
    v
}

pub fn import_graphics(root: &Path, report: &mut Report) -> Option<Bundle> {
    read_a_part();
    let m = read_manifest(root, report)?;
    let Some(g) = m.graphics else {
        report.error(MANIFEST, "the pack has no graphics");
        return None;
    };
    let dirs: Vec<(String, PathBuf)> =
        subdirs(&root.join("graphics/sprites")).into_iter().filter(|(_, d)| nettai_platform::fs::is_file(d.join("sprite.json"))).collect();
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
    read_a_part();
    read_manifest(root, report)?;
    let got = sound::import_with_versions(&root.join("sound"), "sound", report)?;
    (!report.has_errors()).then_some(got)
}

// ---- Loading -------------------------------------------------------------------

/// The battle content of game `game` of the content directory `content`
/// (`crate::index::content()`), for the engine (docs/design/
/// content-model-v2.md §4.0: one game a match): the game's top module and
/// what it requires, with the support packs it depends on, defined
/// (`Content::define`). Everything a battle reads is the content's, the
/// asset handles and the sprites' timing too: no asset pack is read
/// ([`pack_assets`] says what a pack holds of it, for drawing and sound).
pub fn load_battle(content: &Path, game: &str) -> Result<(nettai_battle::Content, Report), Report> {
    let (mut c, mut report) = battle_content(content, game)?;
    // The define phase: what the modules define, the tables registration
    // by number reads, and the registries.
    if let Err(e) = c.define() {
        report.error(content.display().to_string(), e.message);
        return Err(report);
    }
    Ok((c, report))
}

/// [`load_battle`]'s content before the define phase: the game's modules,
/// strings and background pool.
pub fn battle_content(content: &Path, game: &str) -> Result<(nettai_battle::Content, Report), Report> {
    let mut report = Report::default();
    let Some(read) = crate::index::read(content, &[game.to_string()], &mut report) else { return Err(report) };
    battle_content_of(read, report)
}

/// The battle content of what `read` read (one game), before the define
/// phase.
fn battle_content_of(read: crate::index::Read, report: Report) -> Result<(nettai_battle::Content, Report), Report> {
    read_a_part();
    if report.has_errors() {
        return Err(report);
    }
    let scripts = read.scripts();
    // (Every game's backgrounds, from the content: the pool a round may be
    // drawn on, past the game's own.)
    let background_pool = crate::locale::background_pool(&read.dir);
    let c = nettai_battle::Content { scripts, strings: read.strings, background_pool, ..Default::default() };
    Ok((c, report))
}

/// What the asset pack at `pack` holds of `content`'s assets, for whoever
/// draws and plays them (docs/frontend.md §1): each handle's number in the
/// pack. Drawing looks a sprite's frame up by the simulation's animation
/// and frame, so each sprite the content defines must be the pack's with as
/// many animations, each of as many frames (their durations may differ:
/// the content's time the battle, and a content can be changed as a mod
/// is); and every other asset the content names must be the pack's. A pack
/// that isn't so is an error naming each asset it lacks or has otherwise.
pub fn pack_assets(content: &nettai_battle::Content, pack: &Path) -> Result<(PackAssets, Report), Report> {
    let mut report = Report::default();
    let Some(index) = crate::names::read_index(pack, &mut report) else { return Err(report) };
    let Some(frames) = sprite::frame_counts(pack, &mut report) else { return Err(report) };
    let at = pack.display().to_string();
    let numbers = match PackAssets::new(&content.assets, &index) {
        Ok(n) => n,
        Err(missing) => {
            for m in missing {
                report.error(&at, format!("the pack has no {m}, which the content names"));
            }
            return Err(report);
        }
    };
    for (h, name) in content.assets.sprites.iter().enumerate() {
        let ours: Vec<usize> = content.animations.sprites.get(h).map_or(Vec::new(), |d| d.animations.iter().map(|a| a.frames.len()).collect());
        let id = numbers.sprites[h];
        let theirs: Vec<usize> = frames.get(&(id.category, id.index)).cloned().unwrap_or_default();
        if ours != theirs {
            report.error(
                &at,
                format!(
                    "sprite {name}: the content's animations have {} frames, and the pack's {} (drawing takes the pack's frame by the content's animation and frame)",
                    frame_counts(&ours),
                    frame_counts(&theirs)
                ),
            );
        }
    }
    if report.has_errors() {
        return Err(report);
    }
    Ok((numbers, report))
}

/// Animations' frame counts, for a message: `[3, 4]`.
fn frame_counts(counts: &[usize]) -> String {
    format!("{counts:?}")
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
        if !nettai_platform::fs::is_file(path.join(MANIFEST)) {
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
    /// What the pack holds of the content's assets ([`pack_assets`]).
    pub assets: PackAssets,
    /// The pack's own index: every asset it has, by its name there (what a
    /// frontend plays of the pack that no content names: the app's title
    /// music).
    pub index: nettai_content_api::PackIndex,
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
/// game pack and the support packs it depends on, defined, and what its
/// asset pack among `found` ([`find`]) holds of its assets
/// ([`pack_assets`]). A game whose asset pack isn't found is an error with
/// the command that writes it; one whose pack can't draw it, an error
/// naming what the pack lacks.
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
    let (mut content, mut report) = battle_content_of(read, report)?;
    if let Err(e) = content.define() {
        report.error(dir.display().to_string(), e.message);
        return Err(report);
    }
    let assets = match pack_assets(&content, &pack) {
        Ok((assets, r)) => {
            report.issues.extend(r.issues);
            assets
        }
        Err(r) => {
            report.issues.extend(r.issues);
            return Err(report);
        }
    };
    let Some(index) = crate::names::read_index(&pack, &mut report) else { return Err(report) };
    Ok(Loaded { content, game: game.to_string(), pack, assets, index, dir, report })
}

/// A pack's graphics, for a frontend.
pub fn load_graphics(root: &Path) -> Result<(Bundle, Report), Report> {
    let mut report = Report::default();
    match import_graphics(root, &mut report) {
        Some(b) => Ok((b, report)),
        None => Err(report),
    }
}

/// Backgrounds `names` of the pack at `root` alone (another game's, which a
/// round of the background pool draws: docs/app.md §1, "The battle's
/// background"), each by its name, read from its folder
/// (`graphics/backgrounds/<name>`). One the pack hasn't, or that doesn't
/// read, is left out, and what its reading said is in the report.
pub fn load_backgrounds<'n>(root: &Path, names: impl IntoIterator<Item = &'n str>) -> (std::collections::BTreeMap<String, nettai_assets::Background>, Report) {
    let mut report = Report::default();
    let mut out = std::collections::BTreeMap::new();
    for name in names {
        let dir = root.join("graphics/backgrounds").join(name);
        if !nettai_platform::fs::is_file(dir.join("background.json")) {
            continue;
        }
        let mut r = Report::default();
        if let Some((_, bg)) = stage::import_background(&dir, &format!("graphics/backgrounds/{name}"), &mut r)
            && !r.has_errors()
        {
            out.insert(name.to_string(), bg);
        }
        report.issues.extend(r.issues);
    }
    (out, report)
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
        let _ = nettai_platform::fs::remove_dir_all(&dir);
        for (folder, game) in packs {
            let (name, text) = manifest(folder, game, None, false);
            nettai_platform::fs::create_dir_all(dir.join(folder)).unwrap();
            nettai_platform::fs::write(dir.join(folder).join(name), text).unwrap();
        }
        // (A folder that is no pack is passed over.)
        nettai_platform::fs::create_dir_all(dir.join("notes")).unwrap();
        dir
    }

    /// A pack of the test content's assets, the made-up numbers of
    /// `nettai_battle::content::testing::pack_index`, each sprite's
    /// animations with `frames(name, animation)` frames (none: as the
    /// content's), in a folder under the system's temporary one.
    fn pack_of(test: &str, c: &nettai_battle::Content, frames: impl Fn(&str, usize) -> Option<usize>) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nettai-content-{test}-{}", std::process::id()));
        let _ = nettai_platform::fs::remove_dir_all(&dir);
        let index = nettai_battle::content::testing::pack_index();
        let (name, text) = crate::names::index_file(&index);
        write_file(&dir, &name, &text).unwrap();
        for (h, name) in c.assets.sprites.iter().enumerate() {
            let id = index.sprites[name];
            let animations: Vec<Vec<serde_json::Value>> = c.animations.sprites[h]
                .animations
                .iter()
                .enumerate()
                .map(|(i, a)| (0..frames(name, i).unwrap_or(a.frames.len())).map(|_| serde_json::json!({ "tileset": 0, "layout": 0 })).collect())
                .collect();
            // (The part of a sprite.json `sprite::frame_counts` reads.)
            let doc = serde_json::json!({ "format": sprite::FORMAT, "version": sprite::VERSION, "sprite": [id.category, id.index], "animations": animations });
            write_file(&dir, &format!("graphics/sprites/{name}/sprite.json"), doc.to_string().as_bytes()).unwrap();
        }
        dir
    }

    /// docs/frontend.md §1: a pack draws a content when it has every asset
    /// the content names, each sprite with the content's animations and as
    /// many frames in each (their durations the content's own); its numbers
    /// are the pack's. Else the error names what is wrong.
    #[test]
    fn a_pack_draws_the_content_it_has_the_assets_of() {
        let c = nettai_battle::content::testing::content();
        let navi = c.assets.handle(nettai_content_api::AssetKind::Sprite, "test-navi").unwrap();
        let dir = pack_of("pack-assets", &c, |_, _| None);
        let (assets, report) = pack_assets(&c, &dir).unwrap_or_else(|r| panic!("{r}"));
        assert!(!report.has_errors(), "{report}");
        assert_eq!(assets.sprite(navi), Some(nettai_battle::content::testing::NAVI_SPRITE));
        assert_eq!(assets, nettai_battle::content::testing::pack_assets(&c));
        // A sprite whose animation has another number of frames.
        let other = pack_of("pack-frames", &c, |name, i| (name == "test-navi" && i == 3).then_some(5));
        let r = pack_assets(&c, &other).unwrap_err();
        assert!(r.to_string().contains("sprite test-navi: the content's animations have"), "{r}");
        // An asset the pack hasn't.
        let mut index = nettai_battle::content::testing::pack_index();
        index.sounds.remove("test-stage-music");
        let (name, text) = crate::names::index_file(&index);
        write_file(&other, &name, &text).unwrap();
        let r = pack_assets(&c, &other).unwrap_err();
        assert!(r.to_string().contains("the pack has no sound test-stage-music, which the content names"), "{r}");
        for d in [dir, other] {
            let _ = nettai_platform::fs::remove_dir_all(d);
        }
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
        nettai_platform::fs::write(dir.join("five").join(MANIFEST), "format = \"nettai-content\"\nversion = 1\nname = \"x\"\n").unwrap();
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
            let _ = nettai_platform::fs::remove_dir_all(d);
        }
    }
}
