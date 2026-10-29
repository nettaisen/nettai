//! Extraction recipes (docs/design/extraction-recipes.md): each game says in
//! Luau how its assets are extracted from its ROMs (its pack's `extract/`,
//! with the shared part in its support packs' `extract/`). Its root, what
//! its `init.luau` returns, gives the recipe as its `extract`
//! (`require("@self/extract")`), so the recipe loads with the content: the
//! define phase reads its modules, and names the one the root gives
//! (`nettai_battle::content::Scripts::recipe`). That module returns a
//! table: the ROMs it reads (`roms`, by their header codes) and the
//! function that reads them (`extract`) through the primitives (`prims`)
//! and returns the model (`model`): the graphics, the asset names and which
//! ROMs the sound is read from. The host here runs it, in a VM of its own
//! (`nettai_luau::tool`), then finishes the assets as every extraction
//! does: the check that every named asset is there, the letters, the
//! element icons' frame, the hand-drawn frames, the sound bank, the index
//! of names.
//!
//! A recipe is made from what its content's load read ([`Recipe::new`];
//! `nettai_content::load::recipe` makes it of a content): its modules, as
//! their text was then, and its other inputs (the drawings under its
//! packs' `extract/`, its game's `compat/` files), read from the packs'
//! folders as it is made and held in memory, so what it runs on is what it
//! was read as. Its ROMs' codes come from its table as it is made, before
//! any ROM is read.

mod model;
mod prims;

use crate::rom::Rom;
use crate::shapes::{Drawings, SHAPES};
use crate::{Error, Extraction, RomRead, RomSet, elements};
use crate::names::AssetNames;
use crate::sound::SongVersions;
use nettai_content_api::packs::{self, Modules, PackKind, PackManifest};
use nettai_content_api::keys;
use nettai_luau::mlua::Value;
use nettai_luau::tool::{self, ToolOptions};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The folder of a game's pack whose files its recipe reads with `data` (docs/design/extraction-recipes.md §3.9):
/// its compat tables, the original's numbers (docs/design/content-model-v2.md §4.0).
const COMPAT: &str = "compat";

/// The share of an extraction's time its graphics take, the sound the rest ([`Recipe::extract_reporting`]). (Not
/// measured apart: the whole is about a tenth of a second for a game's four ROMs in a release build.)
const GRAPHICS_SHARE: f32 = 0.6;

/// A game's extraction recipe, as its content's load read it.
#[derive(Clone, Debug)]
pub struct Recipe {
    game: String,
    /// Its ROMs' header codes, as its table states them ([`Recipe::roms`]).
    roms: Vec<String>,
    /// Its top module, the one the game's root gives (`exe6:extract/init`).
    entry: String,
    /// The game's pack and the support packs it depends on, in load order.
    packs: Vec<PackManifest>,
    /// Its modules by name, each with the text the content's load read (`exe6:extract/hud`,
    /// `exelib:extract/place`).
    modules: Arc<BTreeMap<String, String>>,
    /// Its other inputs by their path in the bundle: the drawings (`exelib/extract/shapes/banners/bar-104.svg`) and
    /// the game's compat tables (`exe6/compat/assets.toml`).
    files: Arc<BTreeMap<String, Vec<u8>>>,
}

impl Recipe {
    /// Game `game`'s recipe, as its content's load read it: `entry`, the module its root's `extract` is
    /// (`exe6:extract/init`), and `modules` (by name, each with the text the load read: those under `extract/`
    /// are the recipe's, and the rest is passed over), of the packs `packs` (the game's and the support packs it
    /// depends on) in the folders `dirs`, where its other inputs are read now: the drawings under each pack's
    /// `extract/`, and the game's `compat/` files. Its ROMs are its table's: its modules run here to give it,
    /// without ROMs.
    pub fn new(
        game: &str,
        packs: &[PackManifest],
        modules: impl IntoIterator<Item = (String, String)>,
        entry: &str,
        dirs: &packs::Dirs,
    ) -> Result<Recipe, Error> {
        let said = |e: String| Error(format!("{game}'s recipe: {e}"));
        let all: BTreeMap<String, PackManifest> = packs.iter().map(|p| (p.id.clone(), p.clone())).collect();
        let packs: Vec<PackManifest> = packs::with_depends(&all, game).map_err(said)?.into_iter().cloned().collect();
        let own = packs.last().expect("with_depends ends with the pack");
        if own.kind != PackKind::Game {
            return Err(said(format!("{game} is a support pack, not a game")));
        }
        if keys::root_of(entry) != Some(game) || !packs::is_extract_module(entry) {
            return Err(said(format!("{}.luau is no module of {game}'s {}/ folder", keys::module_path(entry), packs::EXTRACT)));
        }
        let modules: BTreeMap<String, String> = modules.into_iter().filter(|(name, _)| packs::is_extract_module(name)).collect();
        let mut trees: Vec<(String, std::path::PathBuf)> = Vec::new();
        for p in &packs {
            let Some(dir) = dirs.0.get(&p.id) else { continue };
            trees.push((format!("{}/{}", p.id, packs::EXTRACT), dir.join(packs::EXTRACT)));
            if p.id == game {
                trees.push((format!("{game}/{COMPAT}"), dir.join(COMPAT)));
            }
        }
        let mut files = BTreeMap::new();
        for (under, root) in trees {
            let mut dirs = vec![root.clone()];
            while let Some(dir) = dirs.pop() {
                let Ok(entries) = nettai_platform::fs::read_dir(&dir) else { continue };
                for path in entries.flatten().map(|e| e.path()) {
                    // (A hidden file, a system's folder note, is no input; nor is a module, which the load read.)
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    if name.starts_with('.') {
                        continue;
                    }
                    if path.is_dir() {
                        dirs.push(path);
                        continue;
                    }
                    if name.ends_with(".luau") {
                        continue;
                    }
                    let rel = path.strip_prefix(&root).expect("walked under the root");
                    let rel = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                    let bytes = nettai_platform::fs::read(&path).map_err(|e| said(format!("{}: {e}", path.display())))?;
                    files.insert(format!("{under}/{rel}"), bytes);
                }
            }
        }
        let mut recipe =
            Recipe { game: game.to_string(), roms: Vec::new(), entry: entry.to_string(), packs, modules: Arc::new(modules), files: Arc::new(files) };
        let vm = recipe.vm().map_err(said)?;
        recipe.roms = model::declaration(vm.entry()).map_err(said)?.roms;
        Ok(recipe)
    }

    /// The game whose recipe it is (`exe6`).
    pub fn game(&self) -> &str {
        &self.game
    }

    /// Its ROMs' header codes, every one required, in the order its table
    /// states them (`roms`, which its places read an address by: a ROM's
    /// slot, from 1).
    pub fn roms(&self) -> &[String] {
        &self.roms
    }

    /// An empty set of the game's ROMs, to give [`Recipe::extract`]: it takes the codes the recipe states.
    pub fn rom_set(&self) -> RomSet {
        RomSet::new(&self.game, &self.roms)
    }

    /// The drawings it makes the frames with (its pack's `extract/shapes/`
    /// and its support packs').
    pub fn drawings(&self) -> Drawings {
        let files = self.files.iter().filter(|(p, _)| p.split_once('/').is_some_and(|(_, rest)| rest.starts_with(&format!("{SHAPES}/"))));
        Drawings::new(&self.game, files.filter_map(|(p, b)| Some((p.clone(), String::from_utf8(b.clone()).ok()?))))
    }

    /// Extract the game's assets from `roms`, which hold every ROM the
    /// recipe declares ([`crate::RomSource::read`] reads them), in memory.
    pub fn extract(&self, roms: &RomSet) -> Result<Extraction, Error> {
        self.extract_reporting(roms, &mut |_| {})
    }

    /// [`Recipe::extract`], telling `progress` how far it is as it goes (a
    /// fraction, 0 to 1).
    pub fn extract_reporting(&self, roms: &RomSet, progress: &mut dyn FnMut(f32)) -> Result<Extraction, Error> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run(roms, progress))).map_err(|panic| {
            let detail = panic.downcast_ref::<String>().map(String::as_str).or_else(|| panic.downcast_ref::<&str>().copied()).unwrap_or("invalid asset data");
            Error(format!("{} ROM decoding failed: {detail}", self.game))
        })?
    }

    /// [`Recipe::extract`], and every range of the ROMs it read, in the
    /// order read (reads of bytes that follow one another are one read),
    /// each with where in the recipe it was read: for comparing what two
    /// releases' ROMs hold where the recipe looks (the verification's
    /// cross-ROM sweep). The sound bank's reads aren't in it.
    pub fn extract_recording_reads(&self, roms: &RomSet) -> Result<(Extraction, Vec<RomRead>), Error> {
        crate::recording_reads(roms, || self.extract(roms))
    }

    /// The ROMs as the recipe is given them, in its table's order, every
    /// one there.
    fn set(&self, roms: &RomSet) -> Result<Vec<prims::RomHandle>, Error> {
        let missing: Vec<&str> = self.roms.iter().filter(|c| !roms.contains(c)).map(String::as_str).collect();
        if !missing.is_empty() {
            return Err(Error(format!(
                "{}: the Legacy Collection's ROMs are incomplete: {} missing (the game's are {})",
                self.game,
                missing.join(", "),
                self.roms.join(", ")
            )));
        }
        let set = self.roms.iter().enumerate();
        Ok(set.map(|(i, code)| prims::RomHandle { rom: roms.rom(code).expect("checked above"), code: code.clone(), slot: i as u32 + 1 }).collect())
    }

    /// The recipe's VM: the primitives, `data` (its game's `compat/` files,
    /// by path in the pack), and its top module loaded.
    fn vm(&self) -> Result<tool::Tool, String> {
        let game = &self.game;
        let compat = format!("{game}/{COMPAT}/");
        let files = self.files.iter().filter(|(p, _)| p.starts_with(&compat));
        let data = prims::Data { game: game.clone(), files: files.map(|(p, b)| (p[game.len() + 1..].to_string(), b.clone().into())).collect() };
        tool::open(Inputs(self.modules.clone()), self.packs.clone(), &self.entry, packs::EXTRACT, ToolOptions::default(), |lua| prims::install(lua, data))
            .map_err(|e| e.to_string())
    }

    fn run(&self, roms: &RomSet, progress: &mut dyn FnMut(f32)) -> Result<Extraction, Error> {
        let game = &self.game;
        let set = self.set(roms)?;
        let recipe = |e: String| Error(format!("{game}'s recipe: {e}"));
        let vm = self.vm().map_err(recipe)?;
        let model::Declaration { extract, .. } = model::declaration(vm.entry()).map_err(recipe)?;
        let set = prims::rom_set(vm.lua(), set).map_err(|e| recipe(e.to_string()))?;
        let value: Value = extract.call(set).map_err(|e| recipe(e.to_string()))?;
        let model::Extracted { names, mut graphics, timing, sound } = model::extracted(&value).map_err(recipe)?;
        drop((value, extract));
        drop(vm);
        complete(&mut graphics, &names).map_err(|e| Error(format!("{game}: {e}")))?;
        progress(GRAPHICS_SHARE);
        let mut warnings = Vec::new();
        warnings.extend(crate::letters::read(&mut graphics).into_iter().map(|p| format!("letters: {p}")));
        warnings.extend(elements::split(&mut graphics.custom).into_iter().map(|p| format!("element icons: {p}")));
        warnings.extend(crate::shapes::read(&mut graphics, &self.drawings(), &|i| names.banner(4 * i as u8)).into_iter().map(|p| format!("frames: {p}")));
        let bank = |(code, rom): &(String, Arc<Rom>)| -> Result<m4a::SoundBank, Error> {
            let (bank, failures) = m4a::rom::extract(&rom.0).map_err(|e| Error(format!("{code}: reading sound: {e}")))?;
            match failures.first() {
                Some((id, e)) => Err(Error(format!("{code}: song {:#05x} doesn't read: {e}", id.0))),
                None => Ok(bank),
            }
        };
        let mut sound_bank = bank(&sound.from)?;
        let mut sound_versions = SongVersions::default();
        for (name, from) in &sound.versions {
            let own = crate::sound::own_songs(&mut sound_bank, &bank(from)?);
            if !own.is_empty() {
                sound_versions.base_version = sound.base_version.clone();
                sound_versions.versions.push((name.clone(), own));
            }
        }
        progress(0.95);
        sound_bank.validate().map_err(|e| Error(e.to_string()))?;
        let songs = sound_bank.songs.iter().enumerate().filter(|(_, s)| s.is_some()).map(|(i, _)| i as u16).collect();
        let index = names.index(&graphics, &songs);
        progress(1.0);
        Ok(Extraction {
            game: game.clone(),
            graphics,
            sound: sound_bank,
            sound_versions,
            index,
            warnings,
            names,
            timing,
        })
    }
}

/// A recipe's modules, by name (`exe6:extract/hud`), as its content's load read them.
struct Inputs(Arc<BTreeMap<String, String>>);

impl Modules for Inputs {
    fn read(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }

    fn has(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }
}

/// What the host does to the model a recipe returns before the passes
/// that read it (§3.10, step 1): each language's buttons sorted by name and
/// the sprites by number; and every asset the names name there (a sprite,
/// the field, a background, a mugshot, a chip's picture and icon, a navi's
/// emblem), else an error naming the first missing.
fn complete(graphics: &mut nettai_assets::Bundle, names: &AssetNames) -> Result<(), String> {
    for (_, l) in &mut graphics.custom.languages {
        l.buttons.sort_by(|a, b| a.0.cmp(&b.0));
    }
    graphics.sprites.sort_by_key(|s| (s.category, s.index));
    let missing = |what: String| Err(format!("{what} is named in the asset names and no ROM gave it"));
    for (&(category, index), name) in &names.sprites {
        if graphics.sprite(category, index).is_none() {
            return missing(format!("sprite/{name}"));
        }
    }
    if graphics.field.tiles.is_empty() {
        return Err("the recipe gave no field".to_string());
    }
    for (&id, name) in &names.backgrounds {
        if graphics.background(id).is_none() {
            return missing(format!("background/{name}"));
        }
    }
    for (&id, name) in &names.mugshots {
        let there = match id.checked_sub(nettai_assets::NAVI_MUGSHOTS) {
            Some(n) => (n as usize) < graphics.hud.navi_mugshots.len(),
            None => (id as usize) < graphics.hud.mugshots.len(),
        };
        if !there {
            return missing(format!("mugshot/{name}"));
        }
    }
    for key in names.chips.values() {
        if !graphics.custom.chip_art.iter().any(|a| &a.key == key && !a.picture.tiles.is_empty()) {
            return missing(format!("chip-art/{key}"));
        }
        if !graphics.hud.chip_icons.iter().any(|a| &a.key == key && !a.tiles.is_empty()) {
            return missing(format!("chip-icon/{key}"));
        }
    }
    for name in names.navis.values() {
        if !graphics.custom.emblems.iter().any(|e| &e.navi == name) {
            return missing(format!("emblem/{name}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
