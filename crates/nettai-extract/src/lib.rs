//! Extract a game's assets from its ROMs, without launching a process.
//!
//! A game's content says how its assets are extracted (docs/design/extraction-recipes.md): its recipe in Luau
//! (`extract/`) states its ROMs, every one required (a complete Battle Network Legacy Collection's), reads them
//! through the primitives here and returns the assets in typed form; the host here makes the pack of them
//! ([`Recipe`]). ROMs are told by the game code in their headers ([`header_code`]), in any order. The same API
//! drives the `nettai-extract` command.
//!
//! A ROM is given as its bytes ([`RomSet::insert`]), from wherever the host has it: nettai reads its ROMs out of an
//! installed Legacy Collection's archives into memory and writes none to disk. [`Recipe::extract_reporting`] and
//! [`Extraction::write_reporting`] say how far they are as they go; [`Recipe::fingerprint`] names the packs a
//! recipe makes.
//!
//! ```no_run
//! use nettai_extract::Recipe;
//! let recipe = Recipe::of_content(std::path::Path::new("content"), "exe6")?;
//! let mut roms = recipe.rom_set();
//! for rom in ["falzar.gba", "gregar.gba", "falzar-jp.gba", "gregar-jp.gba"] {
//!     roms.insert(nettai_platform::fs::read(rom)?)?;
//! }
//! let assets = recipe.extract(&roms)?;
//! // Use assets.graphics / assets.sound directly, or embed the open-format files.
//! let files = assets.files()?;
//! assets.write(std::path::Path::new("new-pack"))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod cli;
mod elements;
mod gfx;
mod recipe;
mod rom;
pub mod shapes;
mod sound;
mod sprite;

pub use recipe::Recipe;

use nettai_content::{names::AssetNames, pack, report::Report, sound::SongVersions};
use rom::Rom;
use std::{collections::BTreeMap, fmt, path::Path, sync::Arc};

/// How much of a ROM [`header_code`] reads: its header, up to and with the game code at 0xAC.
pub const HEADER_LEN: usize = 0xB0;

/// A ROM's game code (`BR6E`), by its header: the four letters and digits at 0xAC. `header` may be the ROM's first
/// [`HEADER_LEN`] bytes alone, so that a host reading ROMs out of archives can match them against the recipes'
/// declarations ([`Recipe::roms`]) before it reads one whole. None: no game code there.
pub fn header_code(header: &[u8]) -> Option<&str> {
    let code = std::str::from_utf8(header.get(0xAC..HEADER_LEN)?).ok()?;
    code.bytes().all(|b| b.is_ascii_alphanumeric()).then_some(code)
}

#[derive(Debug)]
pub struct Error(pub(crate) String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}

/// A game's ROMs, owned, for its recipe ([`Recipe::rom_set`]): each of the codes the recipe states at most once,
/// each recognized by its header's code. A ROM missing is one not inserted; the recipe refuses to run without every
/// one.
pub struct RomSet {
    game: String,
    codes: Vec<String>,
    roms: BTreeMap<String, Arc<Rom>>,
}
impl RomSet {
    /// An empty set of game `game`'s ROMs, whose codes are `codes`.
    fn new(game: &str, codes: &[String]) -> RomSet {
        RomSet { game: game.to_string(), codes: codes.to_vec(), roms: BTreeMap::new() }
    }

    /// Add a ROM, by its bytes: refused if its header's code isn't one of the game's, or if the set has one of its
    /// code already.
    pub fn insert(&mut self, bytes: Vec<u8>) -> Result<(), Error> {
        let Some(code) = header_code(&bytes).filter(|c| self.codes.iter().any(|d| d == c)) else {
            return Err(Error(format!(
                "not a ROM of {} ({}): expected an unmodified ROM of {}",
                self.game,
                header_code(&bytes).map_or_else(|| "no game code".to_string(), |c| format!("game code {c}")),
                self.codes.join(", ")
            )));
        };
        let code = code.to_string();
        if self.roms.contains_key(&code) {
            return Err(Error(format!("duplicate ROM for {code}")));
        }
        self.roms.insert(code, Arc::new(Rom(bytes)));
        Ok(())
    }

    /// Whether the set has the ROM whose code is `code`.
    pub fn contains(&self, code: &str) -> bool {
        self.roms.contains_key(code)
    }

    /// The ROM whose code is `code`, shared.
    fn rom(&self, code: &str) -> Option<Arc<Rom>> {
        self.roms.get(code).cloned()
    }
}

/// Decoded data and diagnostics. Extraction itself performs no filesystem writes.
pub struct Extraction {
    /// The game's id (`exe6`).
    pub game: String,
    pub graphics: nettai_assets::Bundle,
    pub sound: m4a::SoundBank,
    pub sound_versions: SongVersions,
    pub warnings: Vec<String>,
    names: AssetNames,
}

/// A range of a ROM an extraction read ([`Recipe::extract_recording_reads`]).
pub struct RomRead {
    /// The ROM's game code.
    pub rom: String,
    pub address: u32,
    pub len: u32,
    /// The ROM address the word read holds (a table's or a record's
    /// pointer: what the extraction reads at it is its target's).
    pub pointer: Option<u32>,
    /// A span read whole for a parser that takes what it needs of it (an
    /// uncompressed sprite archive): how far the parser went isn't known.
    pub open: bool,
    /// Where in the recipe it was read (`exe6:extract/hud.luau:212`).
    pub at: Option<String>,
}

/// `f` (an extraction of `roms`), and every range of `roms` it read, in the order read (reads of bytes that
/// follow one another are one read).
fn recording_reads<T>(roms: &RomSet, f: impl FnOnce() -> Result<T, Error>) -> Result<(T, Vec<RomRead>), Error> {
    let (extraction, reads) = rom::record_reads(f);
    let code = |at: usize| {
        roms.roms
            .iter()
            .find(|(_, r)| r.0.as_ptr() as usize == at)
            .map(|(code, _)| code.clone())
    };
    let reads = reads
        .into_iter()
        .filter_map(|r| {
            Some(RomRead {
                rom: code(r.rom)?,
                address: r.address,
                len: r.len,
                pointer: match r.kind {
                    rom::ReadKind::Pointer(p) => Some(p),
                    _ => None,
                },
                open: r.kind == rom::ReadKind::Open,
                at: r.at.map(|a| a.to_string()),
            })
        })
        .collect();
    Ok((extraction?, reads))
}

/// The numbers of the songs two ROM images' sound banks play otherwise (the
/// player, the priority, the reverb, the commands, or a voice a note of
/// theirs plays), or that only one has: for comparing two releases' ROMs
/// (the verification's cross-ROM sweep). The samples' and the voicegroups'
/// numbers, which are each bank's own order, don't count.
pub fn songs_played_otherwise(a: &[u8], b: &[u8]) -> Result<Vec<u16>, Error> {
    let bank = |rom: &[u8]| m4a::rom::extract(rom).map(|(bank, _)| bank).map_err(|e| Error(format!("reading sound: {e}")));
    Ok(sound::played_otherwise(&bank(a)?, &bank(b)?))
}

impl Extraction {
    /// The sprites, songs and backgrounds the pack has under a number
    /// (`sprite-cc-ii`, `sound-nnn`, `background-nn`): those
    /// compat/assets.toml names none of.
    pub fn unnamed(&self) -> Vec<String> {
        let sprites = self
            .graphics
            .sprites
            .iter()
            .filter(|s| !self.names.sprites.contains_key(&(s.category, s.index)))
            .map(|s| format!("sprite/{}", self.names.sprite(s.category, s.index)));
        let songs = self
            .sound
            .songs
            .iter()
            .enumerate()
            .filter(|(id, s)| s.is_some() && !self.names.songs.contains_key(&(*id as u16)))
            .map(|(id, _)| format!("sound/{}", self.names.song(id as u16)));
        let backgrounds = self
            .graphics
            .backgrounds
            .iter()
            .enumerate()
            .filter(|(id, b)| b.is_some() && !self.names.backgrounds.contains_key(&(*id as u8)))
            .map(|(id, _)| format!("background/{}", self.names.background(id as u8)));
        sprites.chain(songs).chain(backgrounds).collect()
    }

    /// Open-format pack files, relative paths and bytes, for an application's own
    /// storage. This does not create directories or invoke the CLI.
    pub fn files(&self) -> Result<pack::Files, Error> {
        let mut files = vec![pack::manifest(
            &format!("{} battle assets", self.game),
            &self.game,
            Some(&self.graphics),
            true,
        )];
        files.extend(pack::export_graphics(&self.graphics, &self.names));
        let (sound, failures) =
            pack::export_sound_versions(&self.sound, &self.sound_versions, &self.names);
        if !failures.is_empty() {
            return Err(Error(format!("cannot export songs: {failures:?}")));
        }
        files.extend(sound);
        let songs = self
            .sound
            .songs
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_some())
            .map(|(i, _)| i as u16)
            .collect();
        files.push(nettai_content::names::index_file(
            &self.names.index(&self.graphics, &songs),
        ));
        let mut diagnostics = self.warnings.join("\n");
        // The assets written under a number, which no name in compat/assets.toml
        // gives them yet (content may not use them).
        for name in self.unnamed() {
            diagnostics.push_str(&format!("\nunnamed: {name}"));
        }
        files.push(("extraction.txt".into(), diagnostics.into_bytes()));
        let mut paths = std::collections::HashSet::new();
        for (path, _) in &files {
            if !paths.insert(path) {
                return Err(Error(format!("duplicate output path: {path}")));
            }
        }
        Ok(files)
    }

    /// Write to a new or empty directory and verify the graphics, index and sound
    /// read back. Refuses nonempty directories so stale assets cannot survive a
    /// later extraction. Use `files` for custom overwrite policies.
    pub fn write(&self, root: &Path) -> Result<(), Error> {
        self.write_reporting(root, &mut |_| {})
    }

    /// [`Extraction::write`], telling `progress` how far it is as it goes (a fraction, 0 to 1: the files made,
    /// each written, then read back), for a host that shows a bar. What it writes is `write`'s, byte for byte.
    pub fn write_reporting(&self, root: &Path, progress: &mut dyn FnMut(f32)) -> Result<(), Error> {
        if nettai_platform::fs::exists(root) && nettai_platform::fs::read_dir(root)?.next().is_some() {
            return Err(Error(format!(
                "{} is not empty; choose a new pack directory",
                root.display()
            )));
        }
        let files = self.files()?;
        // (The share of the time the files' making (their encoding) and their writing take, their reading back
        // the rest: about 40, 30 and 30 of 100 for each game's four ROMs, in a release build.)
        let (made, written) = (0.4, 0.7);
        progress(made);
        let total = files.iter().map(|(_, data)| data.len() as u64).sum::<u64>().max(1);
        let mut done = 0;
        for (path, data) in &files {
            pack::write_file(root, path, data)?;
            done += data.len() as u64;
            progress(made + (written - made) * (done as f64 / total as f64) as f32);
        }
        let mut report = Report::default();
        let back = pack::import_graphics(root, &mut report)
            .ok_or_else(|| Error(format!("graphics do not load:\n{report}")))?;
        if back != self.graphics {
            return Err(Error("graphics read back differently".into()));
        }
        let index = nettai_content::names::read_index(root, &mut report)
            .ok_or_else(|| Error(format!("index does not load:\n{report}")))?;
        let songs = self
            .sound
            .songs
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_some())
            .map(|(i, _)| i as u16)
            .collect();
        if index != self.names.index(&self.graphics, &songs) {
            return Err(Error("asset index reads back differently".into()));
        }
        let (bank, versions) = pack::import_sound_versions(root, &mut report)
            .ok_or_else(|| Error(format!("sound does not load:\n{report}")))?;
        let ids = |v: &SongVersions| {
            v.versions
                .iter()
                .map(|(n, songs)| (n.clone(), songs.keys().copied().collect::<Vec<_>>()))
                .collect::<Vec<_>>()
        };
        let song_ids = |b: &m4a::SoundBank| {
            b.songs
                .iter()
                .enumerate()
                .filter_map(|(id, s)| s.as_ref().map(|_| id))
                .collect::<Vec<_>>()
        };
        if song_ids(&bank) != song_ids(&self.sound)
            || bank.mixer != self.sound.mixer
            || bank.players != self.sound.players
            || bank.voicegroups.len() != self.sound.voicegroups.len()
            || ids(&versions) != ids(&self.sound_versions)
            || versions.base_version != self.sound_versions.base_version
        {
            return Err(Error("sound structure reads back differently".into()));
        }
        progress(1.0);
        Ok(())
    }
}
