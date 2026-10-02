//! The names a pack's assets are written under (docs/design/
//! content-model-v2.md §6.3): sprites, backgrounds and songs by name, the
//! HUD's mugshots, banners and chip icons by name. The extractor fills
//! them from BN6's compat/assets.toml; what they don't name is written
//! under a placeholder (`sprite-0c-2d`, `sound-10e`), so nothing is lost.
//! Every file also holds its number (a sprite's `sprite.json`, a song's
//! header...), which the importers read: a pack's names are free.
//!
//! The pack's asset index ([`INDEX`], `assets.toml` at its root) lists
//! every asset content can name, by kind and name, with the engine's
//! identity for it (a sprite's category and index, a sound's number, a
//! banner's, background's or mugshot's id). The loader fills
//! `Content::assets` from it, so `asset.sprite("bomb")` resolves on a real
//! pack; placeholders are listed too (content may not use one, §6.3).

use crate::report::Report;
use nettai_assets::Bundle;
use nettai_content_api::SpriteId;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// The asset index's file in a pack.
pub const INDEX: &str = "assets.toml";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetNames {
    /// Sprites by (category, index).
    pub sprites: BTreeMap<(u8, u8), String>,
    /// Song-table entries.
    pub songs: BTreeMap<u16, String>,
    pub backgrounds: BTreeMap<u8, String>,
    /// Mugshots by their number: the emotion window's by their index in
    /// its table, the link navis' faces from `nettai_assets::NAVI_MUGSHOTS`.
    pub mugshots: BTreeMap<u8, String>,
    /// Banners by banner id (a multiple of 4).
    pub banners: BTreeMap<u8, String>,
    /// Chip icons, by chip id: the chip's key.
    pub chips: BTreeMap<u16, String>,
    /// What each glyph of the game's text font draws (the game's text
    /// encoding), for the HUD's font; and what the dialogue font draws past
    /// them.
    pub glyphs: Vec<String>,
    pub dialogue_glyphs: Vec<String>,
    /// The same for the fonts of other languages the pack has lettering in
    /// (`nettai_assets::HudLettering`), by language: (the 8x16 font's, the
    /// dialogue font's past them).
    pub language_glyphs: BTreeMap<String, (Vec<String>, Vec<String>)>,
}

impl AssetNames {
    pub fn sprite(&self, category: u8, index: u8) -> String {
        self.sprites.get(&(category, index)).cloned().unwrap_or_else(|| format!("sprite-{category:02x}-{index:02x}"))
    }

    pub fn song(&self, id: u16) -> String {
        self.songs.get(&id).cloned().unwrap_or_else(|| format!("sound-{id:03x}"))
    }

    pub fn background(&self, id: u8) -> String {
        self.backgrounds.get(&id).cloned().unwrap_or_else(|| format!("background-{id:02x}"))
    }

    pub fn mugshot(&self, index: u8) -> String {
        self.mugshots.get(&index).cloned().unwrap_or_else(|| format!("mugshot-{index:02x}"))
    }

    pub fn banner(&self, id: u8) -> String {
        self.banners.get(&id).cloned().unwrap_or_else(|| format!("banner-{id:02x}"))
    }

    pub fn chip_icon(&self, chip: u16) -> String {
        self.chips.get(&chip).cloned().unwrap_or_else(|| format!("chip-{chip:03x}"))
    }

    /// The asset index of a pack with these graphics and these song-table
    /// entries: every asset these names name (whether or not the pack has
    /// its image or song: a banner the HUD doesn't draw is still the
    /// engine's banner), and the pack's other sprites, backgrounds,
    /// banners, mugshots and songs under their placeholders.
    pub fn index(&self, graphics: &Bundle, songs: &std::collections::BTreeSet<u16>) -> nettai_content_api::AssetNames {
        let mut a = nettai_content_api::AssetNames::default();
        let sprites = self.sprites.keys().copied().chain(graphics.sprites.iter().map(|s| (s.category, s.index)));
        for (category, index) in sprites {
            a.sprites.insert(self.sprite(category, index), SpriteId { category, index });
        }
        for &id in self.songs.keys().chain(songs) {
            a.sounds.insert(self.song(id), id);
        }
        let present = |v: &[Option<nettai_assets::Background>]| {
            v.iter().enumerate().filter(|(_, b)| b.is_some()).map(|(i, _)| i as u8).collect::<Vec<_>>()
        };
        for id in self.backgrounds.keys().copied().chain(present(&graphics.backgrounds)) {
            a.backgrounds.insert(self.background(id), id);
        }
        for id in self.banners.keys().copied().chain((0..graphics.hud.banners.len()).map(|i| 4 * i as u8)) {
            a.banners.insert(self.banner(id), id);
        }
        let faces = (0..graphics.hud.navi_mugshots.len() as u8).map(|i| nettai_assets::NAVI_MUGSHOTS + i);
        for i in self.mugshots.keys().copied().chain(0..graphics.hud.mugshots.len() as u8).chain(faces) {
            a.mugshots.insert(self.mugshot(i), i);
        }
        a
    }
}

/// The asset index's file.
pub fn index_file(index: &nettai_content_api::AssetNames) -> (String, Vec<u8>) {
    let mut s = String::from(
        "# The pack's assets by name (docs/design/content-model-v2.md §6.3): what content\n\
         # names (asset.sprite(\"bomb\")) and what the engine knows it as. Sprites are\n\
         # \"category-index\" in hex; sounds are song-table numbers; banners, backgrounds and\n\
         # mugshots their ids. Numbered placeholders (sprite-0c-2d) are assets nobody has\n\
         # named yet.\n",
    );
    let key = |k: &str| if k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') { k.to_string() } else { format!("{k:?}") };
    s += "\n[sprites]\n";
    for (name, id) in &index.sprites {
        let _ = writeln!(s, "{} = \"{:02x}-{:02x}\"", key(name), id.category, id.index);
    }
    let numbers = |s: &mut String, table: &str, m: &BTreeMap<String, u16>, digits: usize| {
        let _ = write!(s, "\n[{table}]\n");
        for (name, n) in m {
            let _ = writeln!(s, "{} = 0x{n:0digits$X}", key(name));
        }
    };
    let wide = |m: &BTreeMap<String, u8>| m.iter().map(|(k, &v)| (k.clone(), v as u16)).collect::<BTreeMap<_, _>>();
    numbers(&mut s, "sounds", &index.sounds, 3);
    numbers(&mut s, "banners", &wide(&index.banners), 2);
    numbers(&mut s, "backgrounds", &wide(&index.backgrounds), 2);
    numbers(&mut s, "mugshots", &wide(&index.mugshots), 2);
    (INDEX.to_string(), s.into_bytes())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexFile {
    #[serde(default)]
    sprites: BTreeMap<String, String>,
    #[serde(default)]
    sounds: BTreeMap<String, u16>,
    #[serde(default)]
    banners: BTreeMap<String, u8>,
    #[serde(default)]
    backgrounds: BTreeMap<String, u8>,
    #[serde(default)]
    mugshots: BTreeMap<String, u8>,
}

/// A pack's asset index; empty (with a note) for a pack without one, whose
/// content can name no asset.
pub fn read_index(root: &Path, report: &mut Report) -> Option<nettai_content_api::AssetNames> {
    let path = root.join(INDEX);
    if !path.is_file() {
        report.note(INDEX, "the pack has no asset index: content can name no asset");
        return Some(Default::default());
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            report.error(INDEX, format!("can't read: {e}"));
            return None;
        }
    };
    let f: IndexFile = match toml::from_str(&text) {
        Ok(f) => f,
        Err(e) => {
            report.error(INDEX, format!("invalid: {e}"));
            return None;
        }
    };
    let mut a = nettai_content_api::AssetNames {
        sounds: f.sounds,
        banners: f.banners,
        backgrounds: f.backgrounds,
        mugshots: f.mugshots,
        ..Default::default()
    };
    let mut ok = true;
    for (name, id) in f.sprites {
        let parse = |s: &str| u8::from_str_radix(s, 16).ok();
        match id.split_once('-').and_then(|(c, i)| Some((parse(c)?, parse(i)?))) {
            Some((category, index)) => {
                a.sprites.insert(name, SpriteId { category, index });
            }
            None => {
                report.error(INDEX, format!("sprite {name} is {id:?}, not \"category-index\" in hex"));
                ok = false;
            }
        }
    }
    ok.then_some(a)
}
