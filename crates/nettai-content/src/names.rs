//! The names a game's assets are known by (docs/design/content-model-v2.md
//! §6.3): sprites, backgrounds and songs by name, the HUD's mugshots,
//! banners and chip icons by name. The recipe fills them from the game's
//! compat/assets.toml; what they don't name is known by a placeholder
//! (`sprite-0c-2d`, `sound-10e`), so nothing is lost.
//!
//! The asset index ([`AssetNames::index`]) lists every asset extracted by
//! kind and name, with the game's number for it (a sprite's category and
//! index, a sound's number, a banner's, background's or mugshot's id);
//! placeholders are listed too (content may not use one, §6.3). A frontend
//! finds the assets the content names in it (`nettai_content::load::asset_numbers`:
//! each content handle's number); the battle needs none of it.

use nettai_assets::Bundle;
use std::collections::BTreeMap;

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
    /// Navis by their number in the game: the navi's key, which its emblem
    /// is known by. A navi without one has no emblem.
    pub navis: BTreeMap<u8, String>,
    /// What each glyph of the game's text font draws (the game's text
    /// encoding), for the HUD's font; and what the dialogue font draws past
    /// them.
    pub glyphs: Vec<String>,
    pub dialogue_glyphs: Vec<String>,
    /// The same for the fonts of other languages the graphics have lettering in
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

    /// The asset index of these graphics and these song-table entries:
    /// every asset these names name (whether or not the graphics have its
    /// image or the bank its song: a banner the HUD doesn't draw is still
    /// the engine's banner), and the other sprites, backgrounds, banners,
    /// mugshots and songs under their placeholders.
    pub fn index(&self, graphics: &Bundle, songs: &std::collections::BTreeSet<u16>) -> nettai_content_api::AssetIndex {
        let mut a = nettai_content_api::AssetIndex::default();
        let sprites = self.sprites.keys().copied().chain(graphics.sprites.iter().map(|s| (s.category, s.index)));
        for (category, index) in sprites {
            a.sprites.insert(self.sprite(category, index), nettai_content_api::SpriteNumber { category, index });
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
