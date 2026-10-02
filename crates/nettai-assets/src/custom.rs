//! The custom screen's graphics: the chip selection window the original
//! draws on the HUD layer between turns, with its sprites.
//!
//! The window is a 15x20 tile map whose tiles come from blocks the battle
//! loads at fixed tile numbers of the HUD layer's character block (the
//! window frame from tile 1, ...) and from what the screen copies there as
//! it runs (the chip under the cursor: its art, name, code, element and
//! damage; the dealt chips' icons and codes; the picked chips' icons). The
//! frontend's custom screen (nettai-frontend `custom`) composes those tile
//! numbers itself, so the blocks here keep the tile numbers they load at.

use crate::{MapEntry, Palette, Tiles};

/// One run of a window map's patch list (`sub_8027CCC`): a `width` x
/// `height` block at column `x`, row `y` of the 15-column map takes
/// consecutive tile numbers, row by row (`by_column`: column by column), in
/// `palette`. The numbers run on from one patch to the next, from the
/// list's first tile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapPatch {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
    pub palette: u8,
    pub by_column: bool,
}

/// A 15x20 window map and the patches the screen lays over it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatchList {
    pub first_tile: u16,
    pub patches: Vec<MapPatch>,
}

/// A 56x48 picture in the chip window (7x6 tiles, row by row) with its
/// palette.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Picture {
    pub tiles: Tiles,
    pub palette: Palette,
}

/// A chip's picture, under its chip's key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChipArt {
    pub key: String,
    pub picture: Picture,
}

/// The chip window's pictures for the slots that aren't chips.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotPictures {
    /// OK with nothing picked yet, and with chips picked ("sending").
    pub ok: Picture,
    pub ok_picked: Picture,
    /// The Beast Out button's picture, with its palettes.
    pub beast_out: Picture,
    pub beast_out_palettes: Vec<Palette>,
    /// ChpShufl's re-deal button and DustCross's scrap button.
    pub redeal: Picture,
    pub scrap: Picture,
    /// The slot kind battle mode 1 has (a Beast Out without the button's
    /// art).
    pub other: Picture,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomScreen {
    // ---- The HUD layer's tiles, by the tile number they load at.
    /// The window frame, from tile 1.
    pub window_tiles: Tiles,
    /// The picked-chip column's cells (empty, filled; 1x2 each), from tile
    /// 0x89.
    pub column_cells: Tiles,
    /// The late turns' block (7x2), from tile 0x8D.
    pub turn_limit: Tiles,
    /// The bar behind the enemy names (its slanted end, then the bar; 1x2
    /// each), from tile 0x1D6.
    pub name_bar: Tiles,
    /// The window's map (15x20): without and with the Cross tab.
    pub window_maps: Vec<Vec<MapEntry>>,
    pub window_patches: PatchList,
    /// The Cross window's maps: its opening steps, then the window with
    /// one to five Crosses; and its patches.
    pub cross_maps: Vec<Vec<MapEntry>>,
    pub cross_patches: PatchList,
    // ---- Background palettes.
    /// The window's palette (9) by the chip under the cursor: standard,
    /// mega, giga, dark.
    pub frame_palettes: Vec<Palette>,
    /// The slot icons' palette (11), greyed out (12), and palette 14.
    pub icon_palette: Palette,
    pub grey_palette: Palette,
    pub other_palette: Palette,
    // ---- The chip window.
    pub chip_art: Vec<ChipArt>,
    pub pictures: SlotPictures,
    /// Chip codes as 8x16 glyphs (A..Z, '*', then one no chip has).
    pub codes: Tiles,
    /// Element icons (2x2) and the six colours each puts in palette 11
    /// from colour 10.
    pub elements: Tiles,
    pub element_colours: Vec<[u16; 6]>,
    /// Damage digits as 8x16 glyphs: 0-9, then '?'.
    pub digits: Tiles,
    // ---- The slots.
    /// A slot's code under its icon (2x1 tiles each), by code; the last is
    /// the empty slot's.
    pub slot_codes: Tiles,
    /// The icon of an empty or picked slot (2x2).
    pub empty_icon: Tiles,
    /// The Beast Out button (4x2 each): selectable, unavailable, battle
    /// mode 1's, none.
    pub beast_buttons: Tiles,
    /// ChpShufl's and DustCross's buttons over slots 8 and 9 (12 tiles
    /// each: two slots' icon and code), by state.
    pub redeal_buttons: Tiles,
    pub scrap_buttons: Tiles,
    // ---- Sprites.
    /// The cursor's corner (two frames, 8x8 each).
    pub cursor: Tiles,
    /// The navis' emblems (2x2 each) and their palettes (the cursor's
    /// too), and which a navi shows by its number.
    pub emblems: Tiles,
    pub emblem_palettes: Vec<Palette>,
    pub emblem_of: Vec<u8>,
    pub emblem_palette_of: Vec<u8>,
    /// The Regular chip's frame (two frames of 4x4).
    pub regular: Tiles,
}

impl CustomScreen {
    /// A chip's picture by its key.
    pub fn chip_art(&self, key: &str) -> Option<&Picture> {
        self.chip_art.iter().find(|a| a.key == key).map(|a| &a.picture).filter(|p| !p.tiles.is_empty())
    }

    /// Whether the pack has the custom screen's graphics (an older pack
    /// doesn't).
    pub fn is_empty(&self) -> bool {
        self.window_tiles.is_empty()
    }
}
