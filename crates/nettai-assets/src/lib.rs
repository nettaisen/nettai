//! The graphics a battle frontend draws with, in typed form: what a
//! content pack's graphics (nettai-content) load into. Nothing ROM-derived is
//! checked in; `nettai-extract exe6` writes the pack from the user's copies
//! of the games (both US ROMs).
//!
//! Colors are the GBA's 15-bit BGR555. Tiles are 8x8 with one palette index
//! per pixel, where index 0 is transparent.

pub mod custom;
pub mod lettering;
pub use lettering::{BASE_LANGUAGE, ButtonLettering, CustomLettering, HudLettering};
pub use custom::{
    ButtonPictures, ButtonSets, ChipArt, CursorPlace, CustomLayout, CustomScreen, Emblem, MapPatch, PatchList, Picture, SlotPictures, VersionPictures,
};

/// Assets a game version has its own of: the base game's (`base_version`,
/// an EXE6 pack's "falzar"), and other versions' that differ, by version name
/// ("gregar", from the second ROM). A console of a version shows its own,
/// else the base's. In a pack each is its own asset, named with its
/// version (`cross-names-falzar`, `cross-names-gregar`); one that no other
/// version has its own of is named without.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Versioned<T> {
    pub base: T,
    pub base_version: String,
    pub versions: Vec<(String, T)>,
}

impl<T> Versioned<T> {
    pub fn new(base_version: &str, base: T) -> Versioned<T> {
        Versioned { base, base_version: base_version.into(), versions: Vec::new() }
    }

    /// A file or asset name with its version's suffix, when versions
    /// differ (`name` alone otherwise): the base's for `None`.
    pub fn name(&self, name: &str, version: Option<&str>) -> String {
        if self.versions.is_empty() {
            name.into()
        } else {
            format!("{name}-{}", version.unwrap_or(&self.base_version))
        }
    }

    /// The version's own, else the base's.
    pub fn get(&self, version: &str) -> &T {
        self.version(version).unwrap_or(&self.base)
    }

    /// The version's own, if it has its own.
    pub fn version(&self, version: &str) -> Option<&T> {
        self.versions.iter().find(|(v, _)| v == version).map(|(_, t)| t)
    }
}

/// Everything the frontend draws with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bundle {
    /// Battle sprites, sorted by (category, index).
    pub sprites: Vec<SpriteSheet>,
    pub field: Field,
    /// Battle backgrounds by the battle settings' background id.
    pub backgrounds: Vec<Option<Background>>,
    pub hud: Hud,
    /// The custom screen's (empty in a pack that predates it).
    pub custom: CustomScreen,
}

impl Bundle {
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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

/// A 16-color palette.
pub type Palette = [u16; 16];

/// Decode little-endian BGR555 palettes.
pub fn palettes_from_bytes(data: &[u8]) -> Vec<Palette> {
    data.chunks_exact(32)
        .map(|c| std::array::from_fn(|i| u16::from_le_bytes([c[2 * i], c[2 * i + 1]])))
        .collect()
}

/// A background map entry: which tile, flipped how, in which palette.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    /// The region whose ROMs the sprite comes from, when not the pack's
    /// own (EXE6: `"jp"`, a sprite the US release cut and left a
    /// placeholder in). A console of another region shows something else
    /// there.
    pub region: Option<String>,
}

/// One animation frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    /// The panel types the field draws, by the engine's number for each
    /// (`PanelType as u8`), in the order of their blocks in `panels`: EXE6's
    /// 13 in the engine's order, EXE5's 11 in EXE5's (its metal, lava and sea
    /// among them). docs/design/rules-in-luau.md §7.4.
    pub panel_types: Vec<u8>,
    /// A panel's 5x3 block by `6 * k + 3 * owner + row - 1`, where `k` is
    /// the type's place in `panel_types` and `row` the field row (1..=3).
    pub panels: Vec<[MapEntry; 15]>,
    /// The 5x1 edge under a front-row panel, by owner.
    pub front_edges: [[MapEntry; 5]; 2],
    /// Highlighted panel blocks (a chip's target), by highlight - 1: EXE6
    /// has two; a field may have one (a highlight it lacks is another
    /// pack's, or drawn as a fallback).
    pub highlights: Vec<[MapEntry; 15]>,
}

impl Field {
    /// Whether the field draws panel type `kind` (the engine's number).
    pub fn draws(&self, kind: u8) -> bool {
        self.panel_types.contains(&kind)
    }

    /// The 5x3 block of panel type `kind` (the engine's number) for an
    /// owner (0 the viewer's) and a row (1..=3), if the field draws the
    /// type and has the block.
    pub fn panel(&self, kind: u8, owner: usize, row: u8) -> Option<&[MapEntry; 15]> {
        let k = self.panel_types.iter().position(|&t| t == kind)?;
        self.panels.get(6 * k + 3 * owner + (row as usize).checked_sub(1)?)
    }
}

/// A palette that cycles through frames.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PaletteAnim {
    /// The background palette it replaces.
    pub slot: u8,
    /// Each frame's palette and how many frames it shows.
    pub frames: Vec<(Palette, u8)>,
    /// Picks until the first switch (to frame 1).
    pub initial_timer: u8,
}

// ---- Backgrounds -------------------------------------------------------------

/// A battle background: a scrolling, possibly animated tile map.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    /// The region whose ROMs the picture comes from, where another
    /// region's have another (EXE5's 0x05: `"us"`, the goldfish; the
    /// Japanese ROMs' is the bubbles alone). A console of another region
    /// shows something else.
    pub region: Option<String>,
}

/// A graphics animation: tiles or palettes replaced on a schedule.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GfxAnim {
    pub target: AnimTarget,
    pub frames: Vec<GfxAnimFrame>,
    /// Where playback continues after the last frame (None = it stops on
    /// the last frame).
    pub repeat_from: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimTarget {
    /// Replaces tiles `first..first + count`.
    Tiles { first: u16, count: u16 },
    /// Replaces background palettes `first..first + count`.
    Palettes { first: u8, count: u8 },
    #[default]
    Nothing,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GfxAnimFrame {
    pub tiles: Tiles,
    pub palettes: Vec<Palette>,
    /// Frames this one shows for.
    pub delay: u16,
}

// ---- HUD -----------------------------------------------------------------------

/// The HUD's graphics. "Glyphs" are 8x16: glyph k is tiles 2k (top) and
/// 2k + 1 (bottom).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hud {
    /// The HUD layer's tiles: `tiles[0]` is tile number `first_tile` of the
    /// map entries below (the HP box, HP and damage digits, '+', '×2').
    pub tiles: Tiles,
    pub first_tile: u16,
    /// The custom gauge's tiles, from tile number `gauge_first_tile`.
    pub gauge_tiles: Tiles,
    pub gauge_first_tile: u16,
    /// HP box colors: normal, healing, hurt or low.
    pub hp_palettes: [Palette; 3],
    pub gauge_palette: Palette,
    /// The HP box (6x2) and the gauge frame (18x2) as first placed.
    pub hp_box: Vec<MapEntry>,
    pub gauge_frame: Vec<MapEntry>,
    /// The 8x16 text font, which chip names are drawn with: glyph k draws
    /// `font_chars[k]`.
    pub font: Tiles,
    /// What each glyph of the font draws, as text (the game's text
    /// encoding; the game's marks are characters, one in the Private Use
    /// Area for a glyph Unicode has none for: the stacked EX is U+E002).
    /// Content's names are written in these.
    pub font_chars: Vec<String>,
    /// The opponent's HP digits by color (normal, dropping, rising):
    /// glyph d is digit d.
    pub enemy_digits: [Tiles; 3],
    pub enemy_palette: Palette,
    /// Chip icons (2x2 tiles), each under its chip's key, in the pack's
    /// order (a chip the pack numbers has its number's place), and the
    /// icon of an opponent's hidden chip.
    pub chip_icons: Vec<ChipIcon>,
    pub hidden_icon: Tiles,
    pub icon_palette: Palette,
    /// Mugshots (4x2 tiles) and their palettes by emotion.
    pub mugshots: Vec<(Tiles, Palette)>,
    /// The count box beside the mugshot (2x2 tiles) showing 0..=10, and
    /// without a number.
    pub counts: Vec<Tiles>,
    pub count_box: Tiles,
    /// The box a face brings for beside it (2x2 tiles), by mugshot: EXE5's
    /// faces have their own (all but its souls', which show a count); none
    /// (empty, or past the list: EXE6's) shows the count box or a count.
    pub mugshot_boxes: Vec<Tiles>,
    /// The link navis' faces (a ROM holds its own version's navis' and
    /// ProtoMan's or Colonel's). Which face a form or a navi shows is its
    /// definition's (`mugshot`), by number (see [`NAVI_MUGSHOTS`]).
    pub navi_mugshots: Vec<NaviMugshot>,
    /// The box beside a link navi's mugshot (2x2 tiles), where MegaMan's
    /// count is.
    pub navi_box: Tiles,
    /// "PAUSE": five glyphs, drawn with the opponents' HP digits' palette.
    pub pause: Tiles,
    /// The HUD's text lines, each as glyphs of the font
    /// (`TextScript86F0374`): the multiple deletions (0, 1, 19), "TIME
    /// UP!" (3), the turn timer's seconds 1-10 (4-13), "COUNTER HIT!" (14)
    /// and the custom screen's own.
    pub texts: Vec<Vec<u16>>,
    /// Banners by banner id / 4.
    pub banners: Vec<BannerLayout>,
    /// The banner font's digits (glyph d is digit d; glyph 10 is blank).
    pub banner_digits: Tiles,
    pub banner_palette: Palette,
    /// "Cstmzing..." (8x2 tiles), shown while the opponent is still on the
    /// custom screen, and its palette.
    pub waiting: Tiles,
    pub waiting_palette: Palette,
    /// The warning marker (`sub_800AE90`: a blinking arrow over the custom
    /// gauge or a place on the field): two 16x16 frames of 2x2 tiles, and
    /// its palette.
    pub warning: Tiles,
    pub warning_palette: Palette,
    /// The dialogue font (the chatbox's).
    pub dialogue_font: DialogueFont,
    /// The chatbox's box and key-wait arrow.
    pub chatbox: Chatbox,
    /// The language the lettering above is in (the fonts, the text lines,
    /// the banners' and "Cstmzing..."'s words; empty: `BASE_LANGUAGE`),
    /// and the other languages' the pack has (`lettering`).
    pub language: String,
    pub languages: Vec<(String, HudLettering)>,
}

/// The chatbox's graphics (`chatbox_runScript`'s transfers): the box's
/// tiles and palette, its maps by kind and opening step, the key-wait
/// arrow's frames, and the palette the text and the arrow draw with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Chatbox {
    /// The box's tiles; its maps' entries count from the first.
    pub tiles: Tiles,
    pub palette: Palette,
    /// The box's maps (`spritePtrArr8045CEC`): by kind (0 the message box,
    /// 1 the description box), its four opening steps (0 to 3, open),
    /// each `Chatbox::COLUMNS` x `Chatbox::ROWS` entries row by row.
    pub boxes: Vec<[Vec<MapEntry>; 4]>,
    /// The arrow's three 16x16 frames (2x2 tiles each).
    pub arrow: Tiles,
    pub text_palette: Palette,
}

impl Chatbox {
    pub const COLUMNS: usize = 30;
    pub const ROWS: usize = 8;

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}

/// Where the link navis' faces (`Hud::navi_mugshots`) start among the
/// mugshots' numbers; the emotion window's (`Hud::mugshots`) start at 0.
pub const NAVI_MUGSHOTS: u8 = 0x80;

impl Hud {
    /// Mugshot `number`: its tiles and palettes (an emotion window face
    /// has one; a link navi's two, normal and Full Synchro).
    pub fn mugshot(&self, number: u8) -> Option<(&Tiles, &[Palette])> {
        match number.checked_sub(NAVI_MUGSHOTS) {
            Some(i) => self.navi_mugshots.get(i as usize).map(|m| (&m.tiles, &m.palettes[..])),
            None => self.mugshots.get(number as usize).map(|(t, p)| (t, std::slice::from_ref(p))),
        }
    }

    /// The box emotion-window face `number` brings for beside it, if it
    /// has its own (`mugshot_boxes`).
    pub fn mugshot_box(&self, number: u8) -> Option<&Tiles> {
        self.mugshot_boxes.get(number as usize).filter(|t| !t.is_empty())
    }
}

/// The dialogue font (the chatbox's, `byte_86ACD60`): glyphs of 16x12
/// pixels, each with its advance (`byte_8043CA4`), and what each draws:
/// the game's text encoding's glyphs, then bytes 0xE0-0xE3 and the
/// two-byte codes E4 00 on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DialogueFont {
    /// A palette index a pixel, glyph after glyph, row by row (0 is
    /// clear; the text's color is added to the others).
    pub pixels: Vec<u8>,
    pub advances: Vec<u8>,
    pub chars: Vec<String>,
}

impl DialogueFont {
    pub const WIDTH: usize = 16;
    pub const HEIGHT: usize = 12;

    pub fn len(&self) -> usize {
        self.advances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.advances.is_empty()
    }

    /// Glyph `i`'s pixels (16x12).
    pub fn glyph(&self, i: usize) -> Option<&[u8]> {
        let n = Self::WIDTH * Self::HEIGHT;
        self.pixels.get(n * i..n * (i + 1))
    }

    /// The glyphs of a string, each glyph's longest name first, and the
    /// characters none draws.
    pub fn glyphs(&self, text: &str) -> (Vec<u16>, Vec<char>) {
        let mut out = Vec::new();
        let mut missing = Vec::new();
        let mut rest = text;
        while let Some(c) = rest.chars().next() {
            let best = self
                .chars
                .iter()
                .enumerate()
                .filter(|(_, g)| !g.is_empty() && rest.starts_with(g.as_str()))
                .max_by_key(|(i, g)| (g.len(), std::cmp::Reverse(*i)));
            match best {
                Some((i, g)) => {
                    out.push(i as u16);
                    rest = &rest[g.len()..];
                }
                None => {
                    missing.push(c);
                    rest = &rest[c.len_utf8()..];
                }
            }
        }
        (out, missing)
    }
}

/// A link navi's mugshot (4x2 tiles) with its palettes: normal, angry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NaviMugshot {
    pub tiles: Tiles,
    pub palettes: [Palette; 2],
}

/// A chip's icon. Empty tiles: the chip has none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChipIcon {
    /// The chip's key (the icon's file name in the pack).
    pub key: String,
    pub tiles: Tiles,
}

impl Hud {
    /// The icon of the chip with this key; none if the chip has no icon.
    pub fn chip_icon(&self, key: &str) -> Option<&Tiles> {
        let icon = self.chip_icons.iter().find(|i| i.key == key)?;
        (!icon.tiles.is_empty()).then_some(&icon.tiles)
    }

    /// The font's glyphs for `text` (the longest glyph name that matches
    /// at each place), and the characters it has no glyph for.
    pub fn glyphs(&self, text: &str) -> (Vec<u16>, Vec<char>) {
        let (mut glyphs, mut missing) = (Vec::new(), Vec::new());
        let mut rest = text;
        while let Some(c) = rest.chars().next() {
            let best = self
                .font_chars
                .iter()
                .enumerate()
                .filter(|(_, g)| !g.is_empty() && rest.starts_with(g.as_str()))
                // The first of equally long names: the encoding has two
                // spaces and two hyphens, each pair drawn alike.
                .min_by_key(|(k, g)| (std::cmp::Reverse(g.len()), *k));
            match best {
                Some((k, g)) => {
                    glyphs.push(k as u16);
                    rest = &rest[g.len()..];
                }
                None => {
                    missing.push(c);
                    rest = &rest[c.len_utf8()..];
                }
            }
        }
        (glyphs, missing)
    }
}

/// A banner's text and where it sits.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BannerLayout {
    pub x: u8,
    pub y: u8,
    /// 0 plain, 1 with a number, 2 and 4 hold until removed; 3 (the
    /// telops) has no glyphs of its own: a frontend draws the chip's name
    /// with the font, and 4 (the judge's) adds two numbers to its glyphs.
    pub kind: u8,
    /// 20 glyphs, drawn as five 32x16 sprites.
    pub glyphs: Tiles,
    /// Where the number goes (kind 1).
    pub number_at: Option<(u8, u8)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A glyph name may be several characters (nettai-extract names a glyph
    /// its content has no character for by its number, `[0a3]`); EXE6's
    /// are one each, its marks too (U+E002 the stacked EX).
    #[test]
    fn text_takes_the_longest_glyph_names() {
        let hud = Hud {
            font_chars: [" ", "A", "[", "[0a3]", "n", " ", "\u{E002}"].map(String::from).to_vec(),
            ..Hud::default()
        };
        assert_eq!(hud.glyphs("An [0a3]"), (vec![1, 4, 0, 3], vec![]));
        assert_eq!(hud.glyphs("A?[0"), (vec![1, 2], vec!['?', '0']));
        assert_eq!(hud.glyphs("An\u{E002}"), (vec![1, 4, 6], vec![]));
    }

    #[test]
    fn chip_icons_are_found_by_key_then_number() {
        let icon = |key: &str, n: usize| ChipIcon { key: key.into(), tiles: Tiles { pixels: vec![1; n * Tiles::TILE] } };
        let hud = Hud { chip_icons: vec![icon("cannon", 4), icon("no-icon", 0), icon("sword", 4)], ..Hud::default() };
        assert_eq!(hud.chip_icon("sword"), Some(&hud.chip_icons[2].tiles));
        assert_eq!(hud.chip_icon("no-icon"), None);
        assert_eq!(hud.chip_icon("other"), None);
    }

    #[test]
    fn map_entries_decode() {
        let e = MapEntry::from_gba(0x54A7);
        assert_eq!(e, MapEntry { tile: 0xA7, hflip: true, vflip: false, palette: 5 });
    }
}
