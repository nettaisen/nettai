//! Extraction recipes (docs/design/extraction-recipes.md): each game says in
//! Luau how its assets are extracted from its ROMs (its pack's `extract/`,
//! with the shared part in its support packs' `extract/`), declared in its
//! manifest's `[extract]`. The recipe reads the ROMs through the primitives
//! (`prims`) and returns the model (`model`): the graphics, the asset names
//! and which ROMs the sound is read from. The host here runs it, in a VM of
//! its own (`nettai_luau::tool`), then makes the pack as every extraction
//! does: the check that every named asset is there, the letters, the
//! element icons' frame, the hand-drawn frames, the sound bank, the files.
//!
//! A recipe is read from a content directory (a bundle's, or the
//! repository's content/) whole: its inputs (every file under its game's
//! `extract/` and its support packs', and the files its `reads` names) are
//! held in memory, so what it runs on is what its fingerprint digests.

mod model;
mod prims;

use crate::rom::Rom;
use crate::shapes::{Drawings, SHAPES};
use crate::{Error, Extraction, GRAPHICS_SHARE, RomRead, RomSet, elements};
use nettai_content::{names::AssetNames, pack, sound::SongVersions};
use nettai_content_api::packs::{self, Extract, Modules, PackKind, PackManifest};
use nettai_content_api::keys;
use nettai_luau::mlua::Value;
use nettai_luau::tool::{self, ToolOptions};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

/// The primitives this nettai has (§3.12): rises with every release that
/// adds a primitive, changes what one returns, or lets the codec take a
/// field it didn't. A recipe declares the lowest it runs on.
pub const PRIMITIVES: u32 = 1;

/// A game's extraction recipe, read from a content directory.
#[derive(Clone, Debug)]
pub struct Recipe {
    game: String,
    extract: Extract,
    /// The game's pack and the support packs it depends on, in load order.
    packs: Vec<PackManifest>,
    /// Every input by its path in the bundle (`exe6/extract/hud.luau`,
    /// `exelib/extract/shapes/banners/bar-104.svg`, `exe6/compat/assets.toml`).
    files: Arc<BTreeMap<String, Vec<u8>>>,
}

impl Recipe {
    /// Game `game`'s recipe in the content directory `content`: its
    /// manifest's `[extract]`, its pack's and its support packs' `extract/`
    /// folders, and the files it reads.
    pub fn of_content(content: &Path, game: &str) -> Result<Recipe, Error> {
        let said = |e: String| Error(format!("{game}'s recipe in {}: {e}", content.display()));
        let all: BTreeMap<String, PackManifest> = packs::packs(content).map_err(said)?.into_iter().map(|p| (p.id.clone(), p)).collect();
        let packs: Vec<PackManifest> = packs::with_depends(&all, game).map_err(said)?.into_iter().cloned().collect();
        let own = packs.last().expect("with_depends ends with the pack");
        if own.kind != PackKind::Game {
            return Err(said(format!("{game} is a support pack, not a game")));
        }
        let extract = own.extract.clone().ok_or_else(|| said(format!("{game}/{} declares no [extract]", packs::MANIFEST)))?;
        let mut files = BTreeMap::new();
        for p in &packs {
            let root = content.join(&p.id).join(packs::EXTRACT);
            let mut dirs = vec![root.clone()];
            while let Some(dir) = dirs.pop() {
                let Ok(entries) = nettai_platform::fs::read_dir(&dir) else { continue };
                for path in entries.flatten().map(|e| e.path()) {
                    // (A hidden file, a system's folder note, is no input.)
                    if path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
                        continue;
                    }
                    if path.is_dir() {
                        dirs.push(path);
                        continue;
                    }
                    let rel = path.strip_prefix(&root).expect("walked under the root");
                    let rel = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                    let bytes = nettai_platform::fs::read(&path).map_err(|e| said(format!("{}: {e}", path.display())))?;
                    files.insert(format!("{}/{}/{rel}", p.id, packs::EXTRACT), bytes);
                }
            }
        }
        if !files.keys().any(|p| p.starts_with(&format!("{game}/{}/", packs::EXTRACT))) {
            return Err(said(format!("{game} has no {}/ folder", packs::EXTRACT)));
        }
        for read in &extract.reads {
            let path = content.join(game).join(read);
            let bytes = nettai_platform::fs::read(&path).map_err(|e| said(format!("{game}/{read}, which [extract] reads: {e}")))?;
            files.insert(format!("{game}/{read}"), bytes);
        }
        Ok(Recipe { game: game.to_string(), extract, packs, files: Arc::new(files) })
    }

    /// The game whose recipe it is (`exe6`).
    pub fn game(&self) -> &str {
        &self.game
    }

    /// Its ROMs' header codes, every one required, in the declaration's
    /// order.
    pub fn roms(&self) -> &[String] {
        &self.extract.roms
    }

    /// The lowest primitives version it runs on ([`PRIMITIVES`]).
    pub fn primitives(&self) -> u32 {
        self.extract.primitives
    }

    /// The drawings it makes the frames with (its pack's `extract/shapes/`
    /// and its support packs').
    pub fn drawings(&self) -> Drawings {
        let files = self.files.iter().filter(|(p, _)| p.split_once('/').is_some_and(|(_, rest)| rest.starts_with(&format!("{SHAPES}/"))));
        Drawings::new(&self.game, files.filter_map(|(p, b)| Some((p.clone(), String::from_utf8(b.clone()).ok()?))))
    }

    /// What names the packs it makes (§5.2): SHA-256, its first 16 hex
    /// digits, of the pack formats, this nettai's primitives, the game, its
    /// `[extract]` and each input in path order, its path and its bytes.
    /// The same bundle gives the same on every machine; a change to any
    /// input gives another.
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        let mut put = |b: &[u8]| {
            h.update((b.len() as u64).to_le_bytes());
            h.update(b);
        };
        put(b"nettai extraction recipe");
        put(format!("{:?}", pack::FORMATS).as_bytes());
        put(&PRIMITIVES.to_le_bytes());
        put(self.game.as_bytes());
        put(&self.extract.primitives.to_le_bytes());
        put(&(self.extract.roms.len() as u64).to_le_bytes());
        for code in &self.extract.roms {
            put(code.as_bytes());
        }
        put(&self.extract.size.to_le_bytes());
        put(&(self.extract.reads.len() as u64).to_le_bytes());
        for read in &self.extract.reads {
            put(read.as_bytes());
        }
        put(&(self.files.len() as u64).to_le_bytes());
        for (path, bytes) in self.files.iter() {
            put(path.as_bytes());
            put(bytes);
        }
        h.finalize()[..8].iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Extract the game's assets from `roms`, which hold every ROM the
    /// recipe declares.
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
    /// order read (`crate::extract_recording_reads`'), each with where in
    /// the recipe it was read.
    pub fn extract_recording_reads(&self, roms: &RomSet) -> Result<(Extraction, Vec<RomRead>), Error> {
        crate::recording_reads(roms, || self.extract(roms))
    }

    /// The ROMs as the recipe is given them, in its declaration's order,
    /// every one there and of the declared size.
    fn set(&self, roms: &RomSet) -> Result<Vec<prims::RomHandle>, Error> {
        let missing: Vec<&str> = self.extract.roms.iter().filter(|c| !roms.contains(c)).map(String::as_str).collect();
        if !missing.is_empty() {
            return Err(Error(format!(
                "{}: the Legacy Collection's ROMs are incomplete: {} missing (the game's are {})",
                self.game,
                missing.join(", "),
                self.extract.roms.join(", ")
            )));
        }
        let mut out = Vec::new();
        for (i, code) in self.extract.roms.iter().enumerate() {
            let rom = roms.rom(code).expect("checked above");
            if rom.0.len() != self.extract.size as usize {
                return Err(Error(format!("{code}: expected {} bytes, got {}", self.extract.size, rom.0.len())));
            }
            out.push(prims::RomHandle { rom, code: code.clone(), slot: i as u32 + 1 });
        }
        Ok(out)
    }

    fn run(&self, roms: &RomSet, progress: &mut dyn FnMut(f32)) -> Result<Extraction, Error> {
        let game = &self.game;
        if self.extract.primitives > PRIMITIVES {
            return Err(Error(format!(
                "{game}'s content needs a newer nettai: its recipe needs primitives {}, and this nettai has {PRIMITIVES}",
                self.extract.primitives
            )));
        }
        let set = self.set(roms)?;
        let recipe = |e: String| Error(format!("{game}'s recipe: {e}"));
        let data = prims::Data {
            game: game.clone(),
            files: self.extract.reads.iter().map(|r| (r.clone(), self.files[&format!("{game}/{r}")].clone().into())).collect(),
        };
        let entry = format!("{game}{}{}", keys::SEPARATOR, packs::EXTRACT);
        let vm = tool::open(Inputs(self.files.clone()), self.packs.clone(), &entry, packs::EXTRACT, ToolOptions::default(), |lua| {
            prims::install(lua, data)
        })
        .map_err(|e| recipe(e.to_string()))?;
        let set = prims::rom_set(vm.lua(), set).map_err(|e| recipe(e.to_string()))?;
        let value: Value = vm.call(set).map_err(|e| recipe(e.to_string()))?;
        let model::Extracted { names, mut graphics, sound } = model::extracted(&value).map_err(recipe)?;
        drop(value);
        drop(vm);
        complete(&mut graphics, &names).map_err(|e| Error(format!("{game}: {e}")))?;
        progress(GRAPHICS_SHARE);
        let mut warnings = Vec::new();
        warnings.extend(nettai_content::letters::read(&mut graphics).into_iter().map(|p| format!("letters: {p}")));
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
        progress(1.0);
        Ok(Extraction {
            game: game.clone(),
            graphics,
            sound: sound_bank,
            sound_versions,
            missing_roms: Vec::new(),
            warnings,
            placeholders: Vec::new(),
            names,
        })
    }
}

/// The modules of a recipe's inputs, by name (`exe6:extract/hud` is
/// `exe6/extract/hud.luau`).
struct Inputs(Arc<BTreeMap<String, Vec<u8>>>);

impl Modules for Inputs {
    fn read(&self, name: &str) -> Option<String> {
        let path = keys::module_path(name);
        if path.split('/').any(|s| s.is_empty() || s == "." || s == "..") {
            return None;
        }
        let text = String::from_utf8(self.0.get(&format!("{path}.luau"))?.clone()).ok()?;
        Some(if text.contains("\r\n") { text.replace("\r\n", "\n") } else { text })
    }

    fn has(&self, name: &str) -> bool {
        self.0.contains_key(&format!("{}.luau", keys::module_path(name)))
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
