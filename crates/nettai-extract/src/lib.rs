//! Extract EXE4/EXE5/EXE6 assets without launching a process or reading a content tree.
//!
//! ROMs are identified by their headers, in any order. Missing sources produce
//! checkerboard graphics and silent songs, with omissions recorded in the result.
//! The same API drives `nettai-extract` command.
//!
//! ```no_run
//! use nettai_extract::{extract, Game, RomSet};
//! let mut roms = RomSet::default();
//! roms.insert(std::fs::read("falzar.gba")?)?;
//! let assets = extract(Game::Exe6, &roms)?;
//! // Use assets.graphics / assets.sound directly, or embed the open-format files.
//! let files = assets.files()?;
//! assets.write(std::path::Path::new("new-pack"))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod cli;
mod decode;
mod exe4;
mod exe5;
mod exe6;
mod placeholders;
mod rom;
mod sound;
mod sprite;

use nettai_content::{names::AssetNames, pack, report::Report, sound::SongVersions};
use rom::Rom;
use std::{collections::BTreeMap, fmt, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    Exe4,
    Exe5,
    Exe6,
}

impl Game {
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

#[derive(Debug)]
pub struct Error(String);
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
    roms: BTreeMap<String, Rom>,
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
        self.roms.insert(code.to_string(), Rom(bytes));
        Ok(())
    }
    pub fn contains(&self, code: &str) -> bool {
        self.roms.contains_key(code)
    }
    fn get(&self, code: &str) -> &Rom {
        static MISSING: Rom = Rom(Vec::new());
        self.roms.get(code).unwrap_or(&MISSING)
    }
}

/// Decoded data and diagnostics. Extraction itself performs no filesystem writes.
pub struct Extraction {
    pub game: Game,
    pub graphics: nettai_assets::Bundle,
    pub sound: m4a::SoundBank,
    pub sound_versions: SongVersions,
    pub missing_roms: Vec<String>,
    pub warnings: Vec<String>,
    /// Named assets filled by the shared placeholder pass.
    pub placeholders: Vec<String>,
    names: AssetNames,
}

/// Extract all available sources of `game`. Even an empty set produces a usable
/// placeholder pack. Malformed supplied data fails extraction instead of being
/// treated as a missing source. Existing game decoders' assertions are translated
/// to errors at this boundary (on targets with unwinding enabled).
pub fn extract(game: Game, roms: &RomSet) -> Result<Extraction, Error> {
    std::panic::catch_unwind(|| extract_inner(game, roms)).map_err(|panic| {
        let detail = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap_or("invalid asset data");
        Error(format!("{} ROM decoding failed: {detail}", game.id()))
    })?
}

fn extract_inner(game: Game, roms: &RomSet) -> Result<Extraction, Error> {
    let names = match game {
        Game::Exe4 => exe4::names::asset_names(),
        Game::Exe5 => exe5::names::asset_names(),
        Game::Exe6 => exe6::names::asset_names(),
    };
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
    let mut graphics = match game {
        Game::Exe4 => exe4::graphics::bundle(
            &exe4::rom::Roms {
                redsun: sources[0],
                bluemoon: sources[1],
                redsun_jp: sources[2],
                bluemoon_jp: sources[3],
            },
            &names,
        ),
        Game::Exe5 => exe5::graphics::bundle(
            &exe5::rom::Roms {
                protoman: sources[0],
                colonel: sources[1],
                protoman_jp: sources[2],
                colonel_jp: sources[3],
            },
            &names,
        ),
        Game::Exe6 => exe6::graphics::bundle(
            &exe6::Roms {
                falzar: sources[0],
                gregar: sources[1],
                falzar_jp: sources[2],
                gregar_jp: sources[3],
            },
            &names,
        ),
    };
    let panel_names = match game {
        Game::Exe4 => exe4::graphics::panel_names(),
        Game::Exe5 => exe5::graphics::panel_names(),
        Game::Exe6 => exe6::graphics::panel_names(),
    };
    let mut placeholders = placeholders::complete(&mut graphics, &names, &panel_names);
    let (mut sound, missing_songs) = if let Some(rom) = sources.iter().find(|r| r.is_present()) {
        let (bank, failures) =
            m4a::rom::extract(&rom.0).map_err(|e| Error(format!("reading sound: {e}")))?;
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
    sound.validate().map_err(|e| Error(e.to_string()))?;
    Ok(Extraction {
        game,
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
            &format!("{} battle assets", self.game.id()),
            self.game.id(),
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
        if root.exists() && std::fs::read_dir(root)?.next().is_some() {
            return Err(Error(format!(
                "{} is not empty; choose a new pack directory",
                root.display()
            )));
        }
        let files = self.files()?;
        pack::write_files(root, &files)?;
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
        Ok(())
    }
}
