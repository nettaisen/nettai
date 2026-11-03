//! A game's assets, extracted from its ROMs in memory: its graphics, its sound and the index of their names, for a
//! frontend to draw and play them (docs/design/extraction-recipes.md).
//!
//! A game's content says how its assets are extracted: its recipe in Luau (`extract/`, which the game's root gives as
//! its `extract`, so it loads with the content) states its ROMs, every one required (a complete Battle Network Legacy
//! Collection's), reads them through the primitives here and returns the assets in typed form; the host here
//! finishes them ([`Recipe`]). ROMs are told by the game code in their headers ([`header_code`]), in any order, and
//! read from where they are ([`RomSource`]: the Legacy Collection installed, or ROM files); none is written anywhere.
//!
//! A recipe is made from what the content's load read ([`Recipe::new`]; `nettai_content::load::recipe` makes it of a
//! defined content, and `nettai_content::load::extract` is the whole of it in one call: the content loaded, its
//! recipe run on the ROMs). Then [`RomSource::read`] reads the recipe's ROMs into a [`RomSet`] and
//! [`Recipe::extract`] extracts them. A frontend loads a game with it (nettai-frontend's `game::Game::load`), and
//! checks the assets against the content (`nettai_content::load::asset_numbers`: `nettai-tool --audit-content`).
//! A sprite sheet carries its frames' timing, the ROMs' (the battle counts a sprite's steps; drawing finds the
//! frame). The crate is a library alone.

#![forbid(unsafe_code)]

mod elements;
mod gfx;
pub mod letters;
mod names;
mod recipe;
mod rom;
pub mod shapes;
mod sound;
pub mod source;
mod sprite;

pub use recipe::Recipe;
pub use sound::SongVersions;
pub use source::{Install, RomSource};

use crate::names::AssetNames;
use nettai_content_api::AssetIndex;
use rom::Rom;
use std::{collections::BTreeMap, fmt, sync::Arc};

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

/// A game's assets, extracted from its ROMs ([`Recipe::extract`]), in memory: nothing is written anywhere.
pub struct Extraction {
    /// The game's id (`exe6`).
    pub game: String,
    pub graphics: nettai_assets::Bundle,
    pub sound: m4a::SoundBank,
    /// The songs each version of the game has its own of (EXE5's Team Colonel's).
    pub sound_versions: SongVersions,
    /// Every asset by its name, with its number in `graphics` and `sound`: what content names its assets by
    /// (`nettai_content::load::asset_numbers`).
    pub index: AssetIndex,
    /// What extraction noticed that isn't wrong enough to fail it.
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
    /// The sprites, songs and backgrounds the assets have under a number
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
}
