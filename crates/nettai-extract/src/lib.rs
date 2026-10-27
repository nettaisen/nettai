//! Extract EXE4/EXE5/EXE6 assets without launching a process.
//!
//! A game whose content declares an extraction recipe (its manifest's
//! `[extract]`, docs/design/extraction-recipes.md) is extracted by its recipe
//! in Luau ([`Recipe`]), from every ROM the recipe declares; the others still
//! by the Rust here, which the recipes replace game by game. ROMs are
//! identified by their headers, in any order. The same API drives the
//! `nettai-extract` command.
//!
//! A ROM is given as its bytes (`RomSet::insert`), from wherever the host has it: nettai reads its ROMs out of
//! an installed Legacy Collection's archives into memory and writes none to disk. [`recognize`] tells a ROM's
//! game from its header alone; [`extract_reporting`] and [`Extraction::write_reporting`] say how far they are as
//! they go; [`fingerprint`] tells apart the packs two contents' tables write.
//!
//! The names a pack's assets are written under, and the numbers the extraction finds some of them by, are the
//! content's: its game's compat tables ([`Tables`], `<game>/compat/assets.toml` and the tables it names), read
//! from the content at run time, so content that names a new asset has its pack extracted again with it, with
//! no new build of the extractor (docs/design/content-deployment.md §5, "Asset packs").
//!
//! ```no_run
//! use nettai_extract::{extract, Game, RomSet, Tables};
//! let tables = Tables::of_content(Game::Exe6, std::path::Path::new("content"))?;
//! let mut roms = RomSet::default();
//! roms.insert(nettai_platform::fs::read("falzar.gba")?)?;
//! let assets = extract(&tables, &roms)?;
//! // Use assets.graphics / assets.sound directly, or embed the open-format files.
//! let files = assets.files()?;
//! assets.write(std::path::Path::new("new-pack"))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod cli;
mod decode;
mod elements;
mod exe4;
mod exe5;
mod exe6;
mod placeholders;
mod recipe;
mod rom;
pub mod shapes;
mod sound;
mod sprite;

pub use recipe::{PRIMITIVES, Recipe};

use nettai_content::{names::AssetNames, pack, report::Report, sound::SongVersions};
use rom::Rom;
use std::{collections::BTreeMap, fmt, path::Path, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    Exe4,
    Exe5,
    Exe6,
}

impl Game {
    /// Every game the extractor makes a pack of.
    pub const ALL: [Game; 3] = [Game::Exe4, Game::Exe5, Game::Exe6];

    /// The game whose id ([`Game::id`]: a content's game, `exe6`) is `id`, if the extractor makes its pack.
    pub fn named(id: &str) -> Option<Game> {
        Game::ALL.into_iter().find(|g| g.id() == id)
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Exe4 => "exe4",
            Self::Exe5 => "exe5",
            Self::Exe6 => "exe6",
        }
    }
    /// Preferred base US ROM, other US version, then the corresponding JP ROMs.
    pub fn codes(self) -> [&'static str; 4] {
        match self {
            Self::Exe4 => ["B4WE", "B4BE", "B4WJ", "B4BJ"],
            Self::Exe5 => ["BRBE", "BRKE", "BRBJ", "BRKJ"],
            Self::Exe6 => ["BR6E", "BR5E", "BR6J", "BR5J"],
        }
    }
}

/// How much of a ROM [`recognize`] reads: its header, up to and with the game code at 0xAC.
pub const HEADER_LEN: usize = 0xB0;

/// The game a ROM is of, and its game code (`BR6E`), by its header: the code at 0xAC. `header` may be the ROM's
/// first [`HEADER_LEN`] bytes alone, so that a host reading ROMs out of archives can sort them by game before it
/// reads one whole. None: no ROM of a game the extractor takes. (Whether a ROM is whole is
/// [`RomSet::insert`]'s to say.)
pub fn recognize(header: &[u8]) -> Option<(Game, &'static str)> {
    let code = header.get(0xAC..HEADER_LEN)?;
    Game::ALL.into_iter().find_map(|game| game.codes().into_iter().find(|c| c.as_bytes() == code).map(|c| (game, c)))
}

/// A game's compat tables (`<content>/<game>/compat`): what the Rust extraction of the game reads, the names its
/// assets are written under, which a frontend finds them by (`assets.toml`, a chip's icon by its id, a navi's
/// emblem by its key, what each of the fonts' glyphs draws), and the numbers it reads some by (the panels' types, a
/// version's own chips, the backgrounds several numbers share). And what a save is imported by.
#[derive(Clone, Debug)]
pub enum Compat {
    Exe4(Box<exe4_compat::Compat>),
    Exe5(Box<exe5_compat::Compat>),
    Exe6(Box<exe6_compat::Compat>),
}

/// What a pack of a game is made by, as its content has it: its compat tables ([`Compat`]), its drawings
/// ([`shapes::Drawings`], `<game>/extract/shapes/`), and its extraction recipe where its manifest declares one
/// ([`Recipe`]), which the extraction then runs.
#[derive(Clone, Debug)]
pub struct Tables {
    compat: Compat,
    drawings: shapes::Drawings,
    recipe: Option<Recipe>,
}

impl Tables {
    /// `game`'s tables in the content directory `content` (a bundle's, or the repository's content/), read.
    pub fn of_content(game: Game, content: &Path) -> Result<Tables, Error> {
        let said = |e: String| Error(format!("{}'s compat tables in {}: {e}", game.id(), content.display()));
        let compat = match game {
            Game::Exe4 => Compat::Exe4(Box::new(exe4_compat::Compat::of_content(content).map_err(said)?)),
            Game::Exe5 => Compat::Exe5(Box::new(exe5_compat::Compat::of_content(content).map_err(said)?)),
            Game::Exe6 => Compat::Exe6(Box::new(exe6_compat::Compat::of_content(content).map_err(said)?)),
        };
        let manifest = nettai_content_api::packs::read(content, game.id()).map_err(|e| Error(format!("{}: {e}", content.display())))?;
        let recipe = match manifest.extract {
            Some(_) => Some(Recipe::of_content(content, game.id())?),
            None => None,
        };
        let drawings = shapes::Drawings::of_content(content, game.id()).map_err(|e| Error(format!("{}'s drawings in {}: {e}", game.id(), content.display())))?;
        Ok(Tables { compat, drawings, recipe })
    }

    /// The game they are of.
    pub fn game(&self) -> Game {
        match self.compat {
            Compat::Exe4(_) => Game::Exe4,
            Compat::Exe5(_) => Game::Exe5,
            Compat::Exe6(_) => Game::Exe6,
        }
    }

    /// The game's compat tables.
    pub fn compat(&self) -> &Compat {
        &self.compat
    }

    /// The game's extraction recipe, if its content declares one.
    pub fn recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    /// The names the pack's assets are written under.
    fn names(&self) -> AssetNames {
        match &self.compat {
            Compat::Exe4(c) => exe4::names::asset_names(c),
            Compat::Exe5(c) => exe5::names::asset_names(c),
            Compat::Exe6(c) => exe6::names::asset_names(c),
        }
    }

    /// The field's panel types' names by number.
    fn panel_names(&self) -> Vec<String> {
        match &self.compat {
            Compat::Exe4(c) => exe4::graphics::panel_names(c),
            Compat::Exe5(c) => exe5::graphics::panel_names(c),
            Compat::Exe6(_) => exe6::graphics::panel_names(),
        }
    }

    /// What else of them the extraction reads, which changes what a pack holds: EXE4's version chips and its
    /// text's first control byte, EXE5's backgrounds by their numbers.
    fn read_by_number(&self) -> String {
        match &self.compat {
            Compat::Exe4(c) => {
                let versions: Vec<(&String, &Option<exe4_compat::Version>)> = c.chips.iter().map(|(k, e)| (k, &e.version)).collect();
                format!("{versions:?} {}", c.text.first_control)
            }
            Compat::Exe5(c) => format!("{:?}", c.assets.backgrounds),
            Compat::Exe6(_) => String::new(),
        }
    }
}

/// What tells apart the packs that two contents' tables (or two builds of the extractor) write of a game: a
/// digest, as 16 hex digits, of the versions of the pack's formats (`pack::FORMATS`) and of what the extraction
/// reads of the content's tables ([`Tables`]): the names the pack's assets are written under, which a frontend
/// finds them by, the panels' names and the numbers it reads some assets by. Two whose digests are the same write
/// packs that the same content loads; content that names an asset under another name, or one more, or a build
/// that writes a format of another version, has another. (A change to how an asset is decoded that names nothing
/// new doesn't change it.) A pack is kept under it (the app's `packs/<game>-<fingerprint>`).
pub fn fingerprint(tables: &Tables) -> String {
    if let Some(recipe) = &tables.recipe {
        return recipe.fingerprint();
    }
    let said = format!("{:?} {:?} {:?} {}", pack::FORMATS, tables.names(), tables.panel_names(), tables.read_by_number());
    // (FNV-1a, 64 bits: the same digest from every build and toolchain, which the standard library's hasher
    // doesn't promise.)
    let digest = said.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
    format!("{digest:016x}")
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

/// Owned ROM bytes, suitable for files, uploads, or an application's asset store.
/// A set can contain either or both games. Duplicate codes and unsupported or
/// truncated images are errors; a missing ROM is represented by not inserting it.
#[derive(Default)]
pub struct RomSet {
    roms: BTreeMap<String, Arc<Rom>>,
}
impl RomSet {
    pub fn insert(&mut self, bytes: Vec<u8>) -> Result<(), Error> {
        let code = bytes
            .get(0xAC..0xB0)
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or("");
        if ![Game::Exe4, Game::Exe5, Game::Exe6]
            .iter()
            .any(|g| g.codes().contains(&code))
        {
            return Err(Error(format!(
                "unsupported ROM game code {code:?}; expected an unmodified US or Japanese EXE4/EXE5/EXE6 ROM"
            )));
        }
        if bytes.len() != 8 * 1024 * 1024 {
            return Err(Error(format!(
                "{code}: expected an 8 MiB ROM, got {} bytes",
                bytes.len()
            )));
        }
        if self.roms.contains_key(code) {
            return Err(Error(format!("duplicate ROM for {code}")));
        }
        self.roms.insert(code.to_string(), Arc::new(Rom(bytes)));
        Ok(())
    }
    pub fn contains(&self, code: &str) -> bool {
        self.roms.contains_key(code)
    }
    fn get(&self, code: &str) -> &Rom {
        static MISSING: Rom = Rom(Vec::new());
        self.roms.get(code).map_or(&MISSING, |r| r)
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
    pub missing_roms: Vec<String>,
    pub warnings: Vec<String>,
    /// Named assets filled by the shared placeholder pass.
    pub placeholders: Vec<String>,
    names: AssetNames,
}

/// A range of a ROM an extraction read (`extract_recording_reads`).
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
    /// Where in a recipe it was read (`exe6:extract/hud.luau:212`); none for
    /// the Rust extraction's.
    pub at: Option<String>,
}

/// `extract`, and with it every range of the ROMs the extraction read, in
/// the order read (reads of bytes that follow one another are one read).
/// For comparing what two releases' ROMs hold where the extractor looks
/// (the verification's cross-ROM sweep); the sound bank's reads aren't in
/// it.
pub fn extract_recording_reads(tables: &Tables, roms: &RomSet) -> Result<(Extraction, Vec<RomRead>), Error> {
    recording_reads(roms, || extract(tables, roms))
}

/// `f` (an extraction of `roms`), and every range of `roms` it read, in the order read (reads of bytes that
/// follow one another are one read).
#[doc(hidden)]
pub fn recording_reads<T>(roms: &RomSet, f: impl FnOnce() -> Result<T, Error>) -> Result<(T, Vec<RomRead>), Error> {
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

/// Extract all available sources of the game of `tables` (its content's compat tables), its assets written
/// under the names they give. Even an empty set produces a usable placeholder pack. Malformed supplied data fails
/// extraction instead of being treated as a missing source. Existing game decoders' assertions are translated to
/// errors at this boundary (on targets with unwinding enabled).
pub fn extract(tables: &Tables, roms: &RomSet) -> Result<Extraction, Error> {
    extract_reporting(tables, roms, &mut |_| {})
}

/// [`extract`], telling `progress` how far it is as it goes (a fraction, 0 to 1, after each of its stretches:
/// the graphics, the sound, the checks), for a host that shows a bar.
pub fn extract_reporting(tables: &Tables, roms: &RomSet, progress: &mut dyn FnMut(f32)) -> Result<Extraction, Error> {
    if let Some(recipe) = &tables.recipe {
        return recipe.extract_reporting(roms, progress);
    }
    extract_in_rust_reporting(tables, roms, progress)
}

/// The Rust extraction of the game of `tables`, whether or not its content declares a recipe: for comparing the
/// two while a game moves into Luau (tests/recipes.rs).
#[doc(hidden)]
pub fn extract_in_rust(tables: &Tables, roms: &RomSet) -> Result<Extraction, Error> {
    extract_in_rust_reporting(tables, roms, &mut |_| {})
}

/// Whether the Rust extraction of `game` is still here (the games whose recipes aren't written yet, and those
/// being compared with theirs).
#[doc(hidden)]
pub fn has_rust_extraction(game: Game) -> bool {
    matches!(game, Game::Exe4 | Game::Exe5 | Game::Exe6)
}

fn extract_in_rust_reporting(tables: &Tables, roms: &RomSet, progress: &mut dyn FnMut(f32)) -> Result<Extraction, Error> {
    let game = tables.game();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| extract_inner(tables, roms, progress))).map_err(|panic| {
        let detail = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap_or("invalid asset data");
        Error(format!("{} ROM decoding failed: {detail}", game.id()))
    })?
}

/// The language of the Japanese ROMs' words, as each game's module names
/// its lettering.
const JAPANESE: &str = "ja";

/// The share of an extraction's time its graphics take, the sound the rest ([`extract_reporting`]). (Not
/// measured apart: the whole is a few hundredths of a second for a game's four ROMs in a release build, and a
/// pack's writing, with its encoding, nearly all of its time.)
pub(crate) const GRAPHICS_SHARE: f32 = 0.6;

fn extract_inner(tables: &Tables, roms: &RomSet, progress: &mut dyn FnMut(f32)) -> Result<Extraction, Error> {
    let game = tables.game();
    let names = tables.names();
    let codes = game.codes();
    let sources = codes.map(|code| roms.get(code));
    let missing_roms = codes
        .iter()
        .filter(|code| !roms.contains(code))
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let mut warnings = missing_roms
        .iter()
        .map(|c| format!("missing {c}: unavailable assets use generated placeholders"))
        .collect::<Vec<_>>();
    let mut graphics = match &tables.compat {
        Compat::Exe4(c) => exe4::graphics::bundle(&rom::Roms(sources), &names, c),
        Compat::Exe5(c) => exe5::graphics::bundle(&rom::Roms(sources), &names, c),
        Compat::Exe6(_) => exe6::graphics::bundle(&rom::Roms(sources), &names),
    };
    let panel_names = tables.panel_names();
    let mut placeholders = placeholders::complete(&mut graphics, &names, &panel_names);
    progress(GRAPHICS_SHARE);
    // A pack has the words of the releases its ROMs are of (`sources`: the
    // two US ROMs, then their Japanese ones). With Japanese ROMs alone its
    // own language is Japanese; with no Japanese ROM it has no Japanese
    // lettering. A frontend set to a language the pack lacks draws the
    // pack's own (`Bundle::for_language`), so no language is a set of
    // placeholders.
    let (us, jp) = (sources[..2].iter().any(|r| r.is_present()), sources[2..].iter().any(|r| r.is_present()));
    let mut graphics = match (us, jp) {
        (true, true) => graphics,
        (false, true) => graphics.in_language(JAPANESE).map_err(Error)?.without_language(nettai_assets::BASE_LANGUAGE),
        _ => graphics.without_language(JAPANESE),
    };
    // The pictures' letters and digits, read once here for the enhanced
    // graphics (a frontend reads no picture): what isn't found where every
    // game has it is said (a placeholder picture has none).
    warnings.extend(nettai_content::letters::read(&mut graphics).into_iter().map(|p| format!("letters: {p}")));
    // The element icons apart from their frame.
    warnings.extend(elements::split(&mut graphics.custom).into_iter().map(|p| format!("element icons: {p}")));
    // The frames as shapes, read once here for the enhanced graphics (and
    // the emotion window's frame apart from its faces).
    warnings.extend(shapes::read(&mut graphics, &tables.drawings, &|i| names.banner(4 * i as u8)).into_iter().map(|p| format!("frames: {p}")));
    // (`sources`: the two US ROMs, then their Japanese ones. A US ROM's
    // bank where there is one: EXE5's hit, song 0x6D, is louder in the
    // Japanese ROMs, volume 127 for 112.)
    let (mut sound, missing_songs) = if let Some(i) = sources.iter().position(|r| r.is_present()) {
        let (bank, failures) =
            m4a::rom::extract(&sources[i].0).map_err(|e| Error(format!("reading sound: {e}")))?;
        warnings.extend(
            failures
                .iter()
                .map(|(id, e)| format!("song {:#05x}: {e}; using silence", id.0)),
        );
        (
            bank,
            failures.iter().map(|(id, _)| id.0).collect::<Vec<_>>(),
        )
    } else {
        (
            placeholders::sound_bank(),
            names.songs.keys().copied().collect(),
        )
    };
    let sound_versions = if game == Game::Exe5 && sources[0].is_present() && sources[1].is_present()
    {
        let (colonel, failures) = m4a::rom::extract(&sources[1].0)
            .map_err(|e| Error(format!("reading Colonel sound: {e}")))?;
        warnings.extend(
            failures
                .iter()
                .map(|(id, e)| format!("Colonel song {:#05x}: {e}", id.0)),
        );
        sound::colonel_songs(&mut sound, &colonel)
    } else {
        SongVersions::default()
    };
    placeholders.extend(placeholders::songs(&mut sound, &names, &missing_songs));
    progress(0.95);
    sound.validate().map_err(|e| Error(e.to_string()))?;
    progress(1.0);
    Ok(Extraction {
        game: game.id().to_string(),
        graphics,
        sound,
        sound_versions,
        missing_roms,
        warnings,
        placeholders,
        names,
    })
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
        for name in &self.placeholders {
            diagnostics.push_str(&format!("\nplaceholder: {name}"));
        }
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
    /// later extraction from fewer ROMs. Use `files` for custom overwrite policies.
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
