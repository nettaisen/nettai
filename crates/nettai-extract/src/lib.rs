//! A game's assets, extracted from its ROMs in memory: its graphics, its sound and the index of their names, for a
//! frontend to draw and play them (docs/design/extraction-recipes.md).
//!
//! A game's content says how its assets are extracted: its recipe in Luau (`extract/`) states its ROMs, every one
//! required (a complete Battle Network Legacy Collection's), reads them through the primitives here and returns the
//! assets in typed form; the host here finishes them ([`Recipe`]). ROMs are told by the game code in their headers
//! ([`header_code`]), in any order, and read from where they are ([`RomSource`]: the Legacy Collection installed, or
//! ROM files); none is written anywhere. [`extract`] is the whole of it in one call:
//!
//! ```no_run
//! use nettai_extract::{RomSource, extract};
//! let roms = RomSource::from_env()?; // $NETTAI_ROMS, else the Legacy Collection where Steam installed it
//! let assets = extract(std::path::Path::new("content"), "exe6", &roms)?;
//! // assets.graphics (a nettai_assets::Bundle), assets.sound (an m4a::SoundBank), assets.index (names to numbers)
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Its steps are public too: [`Recipe::of_content`], [`RomSource::read`] (the recipe's ROMs into a [`RomSet`]),
//! [`Recipe::extract`]. A frontend loads a game with it (nettai-frontend's `game::Game::load`), and checks the assets
//! against the content (`nettai_content::load::asset_numbers`, [`Extraction::timing_differences`]: `nettai-tool
//! --audit-content`). The crate is a library alone.

mod elements;
mod gfx;
mod recipe;
mod rom;
pub mod shapes;
mod sound;
pub mod source;
mod sprite;

pub use recipe::Recipe;
pub use sound::SongVersions;
pub use source::{Install, RomSource};

use nettai_content::names::AssetNames;
use nettai_content_api::AssetIndex;
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

/// Game `game`'s assets, extracted from `roms` by its recipe in the content directory `content` (a bundle's, or the
/// repository's content/): [`Recipe::of_content`], [`RomSource::read`] and [`Recipe::extract`] in one call.
pub fn extract(content: &Path, game: &str, roms: &RomSource) -> Result<Extraction, Error> {
    let recipe = Recipe::of_content(content, game)?;
    recipe.extract(&roms.read(&recipe)?)
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
    /// Each sheet's timing as the ROMs have it (`sprite::Sheet`), by sprite: for
    /// [`Extraction::timing_differences`] alone (the content times its sprites).
    timing: BTreeMap<(u8, u8), Vec<Vec<(u8, u8)>>>,
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

    /// Where content times its sprites otherwise than the ROMs (the `nettai-extract` check): for every sprite it
    /// defines (`sprites`, by handle, its names `names`, its numbers here `numbers`), each animation's
    /// frames' durations and whether it loops (the flag 0x40 on its last frame, 0x80 ending it), and the places of
    /// the parts it states (`SpriteDef::parts`) against the sheet's layouts. Each difference names its sprite and
    /// animation; none, the content is the ROMs' timing.
    pub fn timing_differences(
        &self,
        names: &nettai_content_api::AssetNames,
        sprites: &[nettai_content_api::SpriteDef],
        numbers: &nettai_content_api::AssetNumbers,
    ) -> Vec<String> {
        timing_differences(&self.timing, &self.graphics, names, sprites, numbers)
    }
}

/// [`Extraction::timing_differences`] of sheets timed `timing` (by sprite) whose graphics are `graphics`.
fn timing_differences(
    timing: &BTreeMap<(u8, u8), Vec<Vec<(u8, u8)>>>,
    graphics: &nettai_assets::Bundle,
    names: &nettai_content_api::AssetNames,
    sprites: &[nettai_content_api::SpriteDef],
    numbers: &nettai_content_api::AssetNumbers,
) -> Vec<String> {
    const LAST: u8 = 0x80;
    const LOOP: u8 = 0x40;
    // (The flags a frame has by its place: the last's 0x80, with 0x40 where its animation loops.)
    fn flags(k: usize, n: usize, looping: bool) -> u8 {
        match (k + 1 == n, looping) {
            (false, _) => 0,
            (true, false) => LAST,
            (true, true) => LAST | LOOP,
        }
    }
    fn ticks(d: impl Iterator<Item = u8>) -> String {
        d.map(|t| t.to_string()).collect::<Vec<_>>().join(", ")
    }
    let mut out = Vec::new();
    for (h, def) in sprites.iter().enumerate() {
        let name = names.sprites.get(h).map_or("?", String::as_str);
        let Some(id) = numbers.sprites.get(h) else { continue };
        let Some(rom) = timing.get(&(id.category, id.index)) else {
            out.push(format!("sprite {name}: no ROM gave its sheet"));
            continue;
        };
        if def.animations.len() != rom.len() {
            out.push(format!("sprite {name}: the content's {} animations, the ROM's {}", def.animations.len(), rom.len()));
        }
        for (a, (ours, theirs)) in def.animations.iter().zip(rom).enumerate() {
            let n = ours.frames.len();
            let same = n == theirs.len() && ours.frames.iter().zip(theirs).enumerate().all(|(k, (&d, &rom))| rom == (d, flags(k, n, ours.looping)));
            if same {
                continue;
            }
            let looping = theirs.last().is_some_and(|&(_, f)| f & LOOP != 0);
            let mut rom_said = format!("{}{}", ticks(theirs.iter().map(|f| f.0)), if looping { " looping" } else { "" });
            for (k, &(_, f)) in theirs.iter().enumerate() {
                if f != flags(k, theirs.len(), looping) {
                    rom_said += &format!(", frame {k}'s flags {f:#04x}");
                }
            }
            out.push(format!(
                "sprite {name} animation {a}: the content's {}{}, the ROM's {rom_said}",
                ticks(ours.frames.iter().copied()),
                if ours.looping { " looping" } else { "" }
            ));
        }
        let Some(sheet) = graphics.sprite(id.category, id.index) else { continue };
        for (&n, by_anim) in &def.parts {
            if by_anim.len() != sheet.animations.len() {
                out.push(format!("sprite {name}: part {n}'s places in {} animations, the sheet has {}", by_anim.len(), sheet.animations.len()));
            }
            for (a, (ours, frames)) in by_anim.iter().zip(&sheet.animations).enumerate() {
                let theirs: Vec<Option<(i8, i8)>> =
                    frames.iter().map(|f| sheet.part_lists.get(f.parts as usize)?.get(n as usize).map(|p| (p.x, p.y))).collect();
                if ours.iter().copied().map(Some).collect::<Vec<_>>() != theirs {
                    out.push(format!("sprite {name} animation {a}: part {n} is at {ours:?} in the content, and at {theirs:?} in the ROM's layouts"));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_assets::{SpriteFrame, SpritePart, SpriteSheet};
    use nettai_content_api::{AssetNumbers, SpriteAnimation, SpriteDef, SpriteNumber};

    /// A sprite the content times otherwise than the ROMs is named, with the animation: its durations, its loop,
    /// a flag the content can't state, and a part's place.
    #[test]
    fn content_timed_otherwise_than_the_roms_is_named() {
        let part = |x, y| SpritePart { x, y, width: 8, height: 8, ..Default::default() };
        let sheet = SpriteSheet {
            category: 0x0C,
            index: 2,
            part_lists: vec![vec![part(0, 0), part(3, -4)]],
            animations: vec![vec![SpriteFrame::default(); 2], vec![SpriteFrame::default()]],
            ..Default::default()
        };
        let graphics = nettai_assets::Bundle { sprites: vec![sheet], ..Default::default() };
        let timing = BTreeMap::from([((0x0C, 2), vec![vec![(3, 0), (4, 0xC0)], vec![(6, 0x80)]])]);
        let names = nettai_content_api::AssetNames { sprites: vec!["bomb".into()], ..Default::default() };
        let numbers = AssetNumbers { sprites: vec![SpriteNumber { category: 0x0C, index: 2 }], ..Default::default() };
        let def = |animations: &[(&[u8], bool)], part: Vec<Vec<(i8, i8)>>| SpriteDef {
            animations: animations.iter().map(|&(frames, looping)| SpriteAnimation { frames: frames.to_vec(), looping }).collect(),
            parts: BTreeMap::from([(1, part)]),
        };
        let differences = |d: SpriteDef, t: &BTreeMap<_, _>| timing_differences(t, &graphics, &names, &[d], &numbers);
        let same = def(&[(&[3, 4], true), (&[6], false)], vec![vec![(3, -4); 2], vec![(3, -4)]]);
        assert_eq!(differences(same.clone(), &timing), Vec::<String>::new());
        let other = def(&[(&[3, 5], true), (&[6], true)], vec![vec![(3, -4); 2], vec![(0, 0)]]);
        assert_eq!(
            differences(other, &timing),
            [
                "sprite bomb animation 0: the content's 3, 5 looping, the ROM's 3, 4 looping",
                "sprite bomb animation 1: the content's 6 looping, the ROM's 6",
                "sprite bomb animation 1: part 1 is at [(0, 0)] in the content, and at [Some((3, -4))] in the ROM's layouts",
            ]
        );
        let cue = BTreeMap::from([((0x0C, 2), vec![vec![(3, 4), (4, 0xC0)], vec![(6, 0x80)]])]);
        assert_eq!(differences(same, &cue), ["sprite bomb animation 0: the content's 3, 4 looping, the ROM's 3, 4 looping, frame 0's flags 0x04"]);
    }
}
