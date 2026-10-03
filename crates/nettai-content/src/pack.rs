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
/// content root `content` (`crate::root::content()` for BN6's) and the roots it
/// requires, with the assets of the pack `assets` (its asset index,
/// `Content::assets`, and its sprites' animation timing), defined
/// (`Content::define`). [`load_battle_packs`] loads several packs.
pub fn load_battle(content: &Path, assets: &Path) -> Result<(nettai_battle::Content, Report), Report> {
    load_battle_packs(content, &[assets.to_path_buf()])
}

/// [`load_battle`] with the assets of several packs (docs/design/
/// rules-in-luau.md §7.4): one a game.
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

/// [`load_battle`]'s content before the define phase: the modules of every
/// folder of the content directory `content`, the asset index and the
/// sprite timing.
pub fn battle_content(content: &Path, assets: &Path) -> Result<(nettai_battle::Content, Report), Report> {
    battle_content_packs(content, &[assets.to_path_buf()])
}

/// [`battle_content`] with several packs: each says its game (a pack that
/// says none is taken as BN6's, with a warning: packs extracted before they
/// said it); two packs of one game are refused. A game's folder whose
/// modules name its assets loads only with its game's pack (content/bn5
/// with BN5's), else it is left out, with a warning. Asset names are in
/// full, their pack's game first (`bn6:bomb`).
pub fn battle_content_packs(content: &Path, packs: &[PathBuf]) -> Result<(nettai_battle::Content, Report), Report> {
    let mut report = Report::default();
    let Some(roots) = crate::root::read_all(content, &mut report) else { return Err(report) };
    let Some(games) = pack_games(packs, &mut report) else { return Err(report) };
    battle_content_of(roots, &games, report)
}

/// Whether the folder `r` names assets of its own game's pack (`bn5:...` in
/// content/bn5): a game's folder does, and loads only with that pack; a
/// folder of behavior only (content/common) names none and needs no pack.
fn needs_pack(r: &crate::root::Root) -> bool {
    let own_assets = format!("(\"{}{}", r.manifest.name, nettai_content_api::keys::SEPARATOR);
    r.modules.values().any(|m| m.contains("asset.") && m.contains(&own_assets))
}

/// The battle content of the folders `roots` with the packs `games` (each
/// pack's game and directory), before the define phase: a folder whose
/// game's pack isn't among them ([`needs_pack`]) is left out, with a
/// warning.
fn battle_content_of(
    mut roots: Vec<crate::root::Root>,
    games: &[(String, PathBuf)],
    mut report: Report,
) -> Result<(nettai_battle::Content, Report), Report> {
    roots.retain(|r| {
        let game = &r.manifest.name;
        if games.iter().any(|(g, _)| g == game) || !needs_pack(r) {
            return true;
        }
        report.warn(
            r.dir.display().to_string(),
            format!("no {game} pack is loaded: {game}'s content is left out (extract its pack to play it)"),
        );
        false
    });
    if report.has_errors() {
        return Err(report);
    }
    let mut indices = Vec::new();
    let mut timings = Vec::new();
    for (game, path) in games {
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
        strings.merge(r.strings);
        scripts.add_root(r.manifest, r.modules);
    }
    let c = nettai_battle::Content { assets, animations, scripts, strings, ..Default::default() };
    Ok((c, report))
}

/// The game of a pack whose manifest says none: BN6's (packs extracted
/// before they said it).
pub const UNSAID_GAME: &str = "bn6";

/// Each pack's game (a pack that says none is taken as [`UNSAID_GAME`]'s,
/// with a warning), refusing two packs of one game; none when a manifest
/// can't be read.
fn pack_games(packs: &[PathBuf], report: &mut Report) -> Option<Vec<(String, PathBuf)>> {
    let mut games: Vec<(String, PathBuf)> = Vec::new();
    for path in packs {
        let m = read_manifest(path, report)?;
        let game = match m.game {
            Some(g) => g,
            None => {
                report.warn(
                    MANIFEST,
                    format!("{}: the pack says no game; it is taken as {UNSAID_GAME}'s (extract it again to record its game)", path.display()),
                );
                UNSAID_GAME.to_string()
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
    let games = pack_games(packs, &mut Report::default()).unwrap_or_default();
    c.assets.packs.iter().filter_map(|g| games.iter().find(|(game, _)| game == g).map(|(_, p)| p.clone())).collect()
}

// ---- Finding packs -------------------------------------------------------------

/// Where the extractors write their packs (`data/content/bn6`,
/// `data/content/bn5`), and where a frontend finds them.
pub const PACKS: &str = "data/content";

/// The packs directory: `$NETTAI_PACKS`, else `data/content`.
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
/// says (a pack that says none, written before packs said their game, by
/// its folder's name, with a warning). None in a missing `dir`.
pub fn discover(dir: &Path, report: &mut Report) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    for (folder, path) in subdirs(dir) {
        if !path.join(MANIFEST).is_file() {
            continue;
        }
        let Some(m) = read_manifest(&path, report) else { continue };
        let game = m.game.unwrap_or_else(|| {
            report.warn(MANIFEST, format!("{}: the pack says no game; it is taken as {folder}'s (extract it again to record its game)", path.display()));
            folder.clone()
        });
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
/// another) in place of the one of its game, or beside them. `$BN6_PACK`,
/// a BN6 pack's directory, is the first override, kept for the tools that
/// still set it (deprecated: `--pack`, or `$NETTAI_PACKS` for the
/// directory). None when an override isn't a pack.
pub fn find(dir: &Path, overrides: &[PathBuf], report: &mut Report) -> Option<Vec<Found>> {
    let env = std::env::var_os("BN6_PACK").map(PathBuf::from);
    let overrides: Vec<PathBuf> = env.into_iter().chain(overrides.iter().cloned()).collect();
    find_with(dir, &overrides, report)
}

/// [`find`] with these overrides alone.
fn find_with(dir: &Path, overrides: &[PathBuf], report: &mut Report) -> Option<Vec<Found>> {
    let mut found = discover(dir, report);
    for path in overrides {
        let m = read_manifest(path, report)?;
        let game = m.game.unwrap_or_else(|| {
            let folder = path.file_name().map_or("bn6".into(), |f| f.to_string_lossy().into_owned());
            report.warn(MANIFEST, format!("{}: the pack says no game; it is taken as {folder}'s (extract it again to record its game)", path.display()));
            folder
        });
        found.retain(|f| f.game != game);
        found.push(Found { game, dir: path.clone() });
    }
    found.sort_by(|a, b| a.game.cmp(&b.game));
    Some(found)
}

/// How to write the pack of `game`, for a message.
pub fn extract_command(game: &str, dir: &Path) -> String {
    let dir = dir.join(game);
    match game {
        "bn6" => format!("cargo run --release -p bn6-extract -- content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> {}", dir.display()),
        "bn5" => format!("cargo run --release -p bn5-extract -- content <protoman-us> <colonel-us> <protoman-jp> <colonel-jp> {}", dir.display()),
        _ => format!("the {game} extractor's `content` command, into {}", dir.display()),
    }
}

/// The battle content a frontend plays, and the packs it loaded
/// ([`load_found`]).
pub struct Loaded {
    pub content: nettai_battle::Content,
    /// The loaded packs' directories, by `PackId`.
    pub packs: Vec<PathBuf>,
    /// The loaded content folders' directories, by name.
    pub roots: Vec<PathBuf>,
    /// The content's folders that aren't loaded, each with why: their
    /// game's pack isn't found, or the content doesn't load with them.
    pub left_out: Vec<(String, String)>,
    pub report: Report,
}

/// Why folder `game` can't load: no pack of its game is found.
fn no_pack(game: &str, found: &[Found]) -> String {
    let packs = found.iter().map(|f| format!("{} ({})", f.game, f.dir.display())).collect::<Vec<_>>();
    let had = if packs.is_empty() { "none".to_string() } else { packs.join(", ") };
    format!(
        "the content folder {game} draws on {game}'s assets, and no pack of {game} is found in {} (found: {had}); write it with `{}`, or name its directory with --pack",
        packs_dir().display(),
        extract_command(game, &packs_dir()),
    )
}

/// Load the battle content a frontend plays (docs/frontend.md §1) with
/// every pack found ([`find`]): every folder of the content directory
/// `content` (`--content`, else [`crate::root::content`]: one namespace,
/// docs/design/rules-in-luau.md §7.2) whose game's pack is found (a folder
/// of behavior only needs none). A folder whose pack isn't found is left
/// out, and said why with the command that writes the pack; so is a folder
/// the content doesn't define with when the rest define without it (one
/// game's play never fails for another game's folder).
pub fn load_found(content: Option<&Path>, found: &[Found]) -> Result<Loaded, Report> {
    let dir = content.map(Path::to_path_buf).unwrap_or_else(crate::root::content);
    let mut report = Report::default();
    let Some(folders) = crate::root::read_all(&dir, &mut report) else { return Err(report) };
    let games: Vec<(String, PathBuf)> = found.iter().map(|f| (f.game.clone(), f.dir.clone())).collect();
    let mut left_out: Vec<(String, String)> = Vec::new();
    let mut loadable: Vec<crate::root::Root> = Vec::new();
    for f in folders {
        let game = f.manifest.name.clone();
        if needs_pack(&f) && !games.iter().any(|(g, _)| *g == game) {
            left_out.push((game.clone(), no_pack(&game, found)));
        } else {
            loadable.push(f);
        }
    }
    if !loadable.iter().any(needs_pack) {
        for (_, why) in &left_out {
            report.error(dir.display().to_string(), why.clone());
        }
        if !report.has_errors() {
            report.error(dir.display().to_string(), "no content folder names any asset: there is nothing to play");
        }
        return Err(report);
    }
    let load = |roots: Vec<crate::root::Root>| -> Result<(nettai_battle::Content, Report, Vec<PathBuf>, Vec<PathBuf>), Report> {
        let dirs: Vec<PathBuf> = roots.iter().map(|r| r.dir.clone()).collect();
        let (mut c, mut report) = battle_content_of(roots, &games, Report::default())?;
        if let Err(e) = c.define() {
            report.error(dir.display().to_string(), e.message);
            return Err(report);
        }
        let packs = c.assets.packs.iter().filter_map(|g| games.iter().find(|(game, _)| game == g).map(|(_, p)| p.clone())).collect();
        Ok((c, report, packs, dirs))
    };
    let loaded = match load(loadable.clone()) {
        Ok(l) => l,
        Err(r) => {
            // A folder without which the rest define is left out, said why
            // (a game's folder that another's requires can't be).
            let mut out = None;
            for (i, f) in loadable.iter().enumerate().filter(|(_, f)| needs_pack(f)) {
                let rest: Vec<crate::root::Root> = loadable.iter().enumerate().filter(|&(j, _)| j != i).map(|(_, r)| r.clone()).collect();
                if !rest.iter().any(needs_pack) {
                    continue;
                }
                if let Ok(l) = load(rest) {
                    let why = r.issues.iter().find(|i| i.level == crate::report::Level::Error).map_or("it doesn't load".to_string(), |i| i.to_string());
                    left_out.push((f.manifest.name.clone(), why));
                    out = Some(l);
                    break;
                }
            }
            match out {
                Some(l) => l,
                None => return Err(r),
            }
        }
    };
    let (content, report, packs, roots) = loaded;
    Ok(Loaded { content, packs, roots, left_out, report })
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
    fn packs(test: &str, packs: &[(&str, Option<&str>)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nettai-content-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (folder, game) in packs {
            let (name, text) = manifest_of(folder, *game, None, false);
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
        let dir = packs("found", &[("bn6", Some("bn6")), ("five", Some("bn5")), ("old", None)]);
        let mut r = Report::default();
        let found = discover(&dir, &mut r);
        let games: Vec<(&str, &str)> =
            found.iter().map(|f| (f.game.as_str(), f.dir.file_name().unwrap().to_str().unwrap())).collect();
        assert_eq!(games, [("bn6", "bn6"), ("bn5", "five"), ("old", "old")]);
        assert!(!r.has_errors() && r.issues.len() == 1, "a pack that says no game is its folder's, with a warning: {:?}", r.issues);
        let other = packs("override", &[("mine", Some("bn6"))]);
        let found = find_with(&dir, &[other.join("mine")], &mut Report::default()).unwrap();
        let bn6 = found.iter().find(|f| f.game == "bn6").unwrap();
        assert_eq!(bn6.dir, other.join("mine"));
        assert_eq!(found.len(), 3);
        // None in a directory that isn't there.
        assert!(discover(&dir.join("missing"), &mut Report::default()).is_empty());
        // Two packs of one game: an error.
        let two = packs("two", &[("a", Some("bn6")), ("b", Some("bn6"))]);
        let mut r = Report::default();
        assert_eq!(discover(&two, &mut r).len(), 1);
        assert!(r.has_errors());
        for d in [dir, other, two] {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}
