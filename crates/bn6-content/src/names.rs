//! The names a pack's assets are written under (docs/design/
//! content-model-v2.md §6.3): sprites, backgrounds and songs by name, the
//! HUD's mugshots, banners and chip icons by name. The extractor fills
//! them from BN6's compat/assets.toml; what they don't name is written
//! under a placeholder (`sprite-0c-2d`, `sound-10e`), so nothing is lost.
//! Every file also holds its number (a sprite's `sprite.json`, a song's
//! header...), which the importers read: a pack's names are free.

use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetNames {
    /// Sprites by (category, index).
    pub sprites: BTreeMap<(u8, u8), String>,
    /// Song-table entries.
    pub songs: BTreeMap<u16, String>,
    pub backgrounds: BTreeMap<u8, String>,
    /// Mugshots by their index in the mugshot table.
    pub mugshots: BTreeMap<u8, String>,
    /// Banners by banner id (a multiple of 4).
    pub banners: BTreeMap<u8, String>,
    /// Chip icons, by chip id: the chip's key.
    pub chips: BTreeMap<u16, String>,
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
}
