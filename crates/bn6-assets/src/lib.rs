//! The graphics a battle frontend draws with, in typed form.
//!
//! `bn6-extract graphics <rom> <dir>` decodes them from the user's copy of the
//! game into `<dir>/bn6-assets.bin`. Nothing ROM-derived is checked in; the
//! frontend loads the bundle at start-up.
//!
//! Colours are the GBA's 15-bit BGR555. Tiles are 8x8 with one palette index
//! per pixel, where index 0 is transparent.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// The bundle's file name inside the asset directory.
pub const FILE_NAME: &str = "bn6-assets.bin";

/// Identifies the file and its layout version.
const MAGIC: &[u8; 8] = b"BN6GFX\x00\x04";

/// Everything the frontend draws with.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Bundle {
    /// Battle sprites, sorted by (category, index).
    pub sprites: Vec<SpriteSheet>,
    pub field: Field,
    /// Battle backgrounds by the battle settings' background id.
    pub backgrounds: Vec<Option<Background>>,
    pub hud: Hud,
}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    /// Not an asset bundle, or one written by a different version.
    Format(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "{e}"),
            LoadError::Format(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl Bundle {
    /// Encode the bundle.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.extend(bincode::serialize(self).expect("bundle serializes"));
        out
    }

    /// Decode a bundle written by [`Bundle::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Bundle, LoadError> {
        let body = bytes
            .strip_prefix(MAGIC.as_slice())
            .ok_or_else(|| LoadError::Format("not a bn6 asset bundle of this version; re-run `bn6-extract graphics`".into()))?;
        bincode::deserialize(body).map_err(|e| LoadError::Format(format!("corrupt asset bundle: {e}")))
    }

    /// Load `dir/bn6-assets.bin` (or `path` itself if it names a file).
    pub fn load(path: impl AsRef<Path>) -> Result<Bundle, LoadError> {
        let path = path.as_ref();
        let file = if path.is_dir() { path.join(FILE_NAME) } else { path.to_path_buf() };
        Bundle::from_bytes(&std::fs::read(&file).map_err(LoadError::Io)?)
    }

    /// Write `dir/bn6-assets.bin`.
    pub fn save(&self, dir: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(dir.as_ref())?;
        std::fs::write(dir.as_ref().join(FILE_NAME), self.to_bytes())
    }

    /// A battle sprite by (category, index).
    pub fn sprite(&self, category: u8, index: u8) -> Option<&SpriteSheet> {
        self.sprites
            .binary_search_by_key(&(category, index), |s| (s.category, s.index))
            .ok()
            .map(|i| &self.sprites[i])
    }

    /// A background by id.
    pub fn background(&self, id: u8) -> Option<&Background> {
        self.backgrounds.get(id as usize).and_then(|b| b.as_ref())
    }
}

/// 8x8 tiles, 64 palette indices each (row-major).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Tiles {
    pub pixels: Vec<u8>,
}

impl Tiles {
    pub const TILE: usize = 64;

    /// Decode 4-bit-per-pixel GBA tile data (low nibble first).
    pub fn from_4bpp(data: &[u8]) -> Tiles {
        let n = data.len() / 32;
        let mut pixels = Vec::with_capacity(n * Self::TILE);
        for b in &data[..n * 32] {
            pixels.push(b & 0xF);
            pixels.push(b >> 4);
        }
        Tiles { pixels }
    }

    pub fn len(&self) -> usize {
        self.pixels.len() / Self::TILE
    }

    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }

    pub fn get(&self, i: usize) -> Option<&[u8]> {
        self.pixels.get(i * Self::TILE..(i + 1) * Self::TILE)
    }

    pub fn push(&mut self, tile: &[u8]) {
        assert_eq!(tile.len(), Self::TILE);
        self.pixels.extend_from_slice(tile);
    }
}

/// A 16-colour palette.
pub type Palette = [u16; 16];

/// Decode little-endian BGR555 palettes.
pub fn palettes_from_bytes(data: &[u8]) -> Vec<Palette> {
    data.chunks_exact(32)
        .map(|c| std::array::from_fn(|i| u16::from_le_bytes([c[2 * i], c[2 * i + 1]])))
        .collect()
}

/// A background map entry: which tile, flipped how, in which palette.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapEntry {
    pub tile: u16,
    pub hflip: bool,
    pub vflip: bool,
    pub palette: u8,
}

impl MapEntry {
    /// From the GBA's text-mode map format.
    pub fn from_gba(e: u16) -> MapEntry {
        MapEntry { tile: e & 0x3FF, hflip: e & 0x400 != 0, vflip: e & 0x800 != 0, palette: (e >> 12) as u8 }
    }
}

// ---- Sprites ----------------------------------------------------------------

/// A battle sprite: its animations, and the tiles, palettes and part
/// layouts their frames draw with.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SpriteSheet {
    /// The engine's `SpriteId`.
    pub category: u8,
    pub index: u8,
    pub tilesets: Vec<Tiles>,
    /// Palette sets; a sprite draws with one palette of its frame's set,
    /// chosen by the object (`sprite_setPalette`) plus the frame's offset.
    pub palette_sets: Vec<Vec<Palette>>,
    pub part_lists: Vec<Vec<SpritePart>>,
    pub animations: Vec<Vec<SpriteFrame>>,
}

/// One animation frame.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpriteFrame {
    /// Index into `tilesets`.
    pub tileset: u16,
    /// Index into `palette_sets`.
    pub palette_set: u16,
    /// Index into `part_lists`.
    pub parts: u16,
    pub duration: u8,
    pub flags: u8,
}

/// One hardware sprite of a frame. The first part of every frame is the
/// object's shadow.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpritePart {
    /// First tile in the frame's tileset (the part's tiles follow in
    /// row-major order).
    pub tile: u16,
    /// Offset from the object's screen position.
    pub x: i8,
    pub y: i8,
    pub width: u8,
    pub height: u8,
    pub hflip: bool,
    pub vflip: bool,
    /// Added to the sprite's palette; the first part's offset applies to
    /// the whole frame.
    pub palette: u8,
}

// ---- Field ------------------------------------------------------------------

/// The battle field's panels.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Field {
    /// Panel tiles; `tiles[0]` is tile number `first_tile` of the map
    /// entries below.
    pub tiles: Tiles,
    pub first_tile: u16,
    /// Background palettes `first_palette..` at the start of a battle.
    pub palettes: Vec<Palette>,
    pub first_palette: u8,
    /// Panel palettes that cycle.
    pub palette_anims: Vec<PaletteAnim>,
    /// A panel's 5x3 block by `6 * type + 3 * owner + row - 1`, where
    /// `type` is the panel type (0..=12) and `row` the field row (1..=3).
    pub panels: Vec<[MapEntry; 15]>,
    /// The 5x1 edge under a front-row panel, by owner.
    pub front_edges: [[MapEntry; 5]; 2],
    /// Highlighted panel blocks (a chip's target), by highlight - 1.
    pub highlights: [[MapEntry; 15]; 2],
}

/// A palette that cycles through frames.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct PaletteAnim {
    /// The background palette it replaces.
    pub slot: u8,
    /// Each frame's palette and how many frames it shows.
    pub frames: Vec<(Palette, u8)>,
    /// Draws until the first switch (to frame 1).
    pub initial_timer: u8,
}

// ---- Backgrounds -------------------------------------------------------------

/// A battle background: a scrolling, possibly animated tile map.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Background {
    /// `tiles[0]` is tile number `first_tile`.
    pub tiles: Tiles,
    pub first_tile: u16,
    pub map: Vec<MapEntry>,
    pub map_width: u16,
    pub map_height: u16,
    /// Background palette 0.
    pub palette: Option<Palette>,
    /// Scroll per frame in 1/16 pixel (the game's scroll counters).
    pub scroll: (i32, i32),
    pub anims: Vec<GfxAnim>,
}

/// A graphics animation: tiles or palettes replaced on a schedule.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GfxAnim {
    pub target: AnimTarget,
    pub frames: Vec<GfxAnimFrame>,
    /// Where playback continues after the last frame (None = it stops on
    /// the last frame).
    pub repeat_from: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimTarget {
    /// Replaces tiles `first..first + count`.
    Tiles { first: u16, count: u16 },
    /// Replaces background palettes `first..first + count`.
    Palettes { first: u8, count: u8 },
    #[default]
    Nothing,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GfxAnimFrame {
    pub tiles: Tiles,
    pub palettes: Vec<Palette>,
    /// Frames this one shows for.
    pub delay: u16,
}

// ---- HUD -----------------------------------------------------------------------

/// The HUD's graphics. "Glyphs" are 8x16: glyph k is tiles 2k (top) and
/// 2k + 1 (bottom).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Hud {
    /// The HUD layer's tiles: `tiles[0]` is tile number `first_tile` of the
    /// map entries below (the HP box, HP and damage digits, '+', '×2').
    pub tiles: Tiles,
    pub first_tile: u16,
    /// The custom gauge's tiles, from tile number `gauge_first_tile`.
    pub gauge_tiles: Tiles,
    pub gauge_first_tile: u16,
    /// HP box colours: normal, healing, hurt or low.
    pub hp_palettes: [Palette; 3],
    pub gauge_palette: Palette,
    /// The HP box (6x2) and the gauge frame (18x2) as first placed.
    pub hp_box: Vec<MapEntry>,
    pub gauge_frame: Vec<MapEntry>,
    /// The 8x16 text font (the game's text codes).
    pub font: Tiles,
    /// Chip names in font codes, by chip id.
    pub chip_names: Vec<Vec<u8>>,
    /// Whether a chip's damage shows after its name, by chip id.
    pub chip_shows_damage: Vec<bool>,
    /// The opponent's HP digits by colour (normal, dropping, rising):
    /// glyph d is digit d.
    pub enemy_digits: [Tiles; 3],
    pub enemy_palette: Palette,
    /// Chip icons (2x2 tiles) by chip id, and the icon of an opponent's
    /// hidden chip.
    pub chip_icons: Vec<Tiles>,
    pub hidden_icon: Tiles,
    pub icon_palette: Palette,
    /// Mugshots (4x2 tiles) and their palettes by emotion.
    pub mugshots: Vec<(Tiles, Palette)>,
    /// The count box beside the mugshot (2x2 tiles) showing 0..=10, and
    /// without a number.
    pub counts: Vec<Tiles>,
    pub count_box: Tiles,
    /// A transformed navi's mugshot emotion by form.
    pub form_emotions: Vec<u8>,
    /// Banners by banner id / 4.
    pub banners: Vec<BannerLayout>,
    /// The banner font's digits (glyph d is digit d; glyph 10 is blank).
    pub banner_digits: Tiles,
    pub banner_palette: Palette,
    /// "Cstmzing..." (8x2 tiles), shown while the opponent is still on the
    /// custom screen, and its palette.
    pub waiting: Tiles,
    pub waiting_palette: Palette,
}

/// A banner's text and where it sits.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BannerLayout {
    pub x: u8,
    pub y: u8,
    /// 0 plain, 1 with a number, 2 and 4 hold until removed, 3 and 4 are
    /// drawn from text at run time (not extracted).
    pub kind: u8,
    /// 20 glyphs, drawn as five 32x16 sprites.
    pub glyphs: Tiles,
    /// Where the number goes (kind 1).
    pub number_at: Option<(u8, u8)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mut b = Bundle::default();
        b.sprites.push(SpriteSheet { category: 0, index: 1, ..Default::default() });
        b.field.tiles = Tiles::from_4bpp(&[0x21; 32]);
        let back = Bundle::from_bytes(&b.to_bytes()).unwrap();
        assert_eq!(back.sprites.len(), 1);
        assert_eq!(back.field.tiles.get(0).unwrap()[..2], [1, 2]);
        assert!(Bundle::from_bytes(b"nope").is_err());
    }

    #[test]
    fn map_entries_decode() {
        let e = MapEntry::from_gba(0x54A7);
        assert_eq!(e, MapEntry { tile: 0xA7, hflip: true, vflip: false, palette: 5 });
    }
}
