//! The custom screen's graphics: the chip selection window the original
//! draws on the HUD layer between turns, with its sprites.
//!
//! The window is a 15x20 tile map whose tiles come from blocks the battle
//! loads at fixed tile numbers of the HUD layer's character block (the
//! window frame from tile 1, ...) and from what the screen copies there as
//! it runs (the chip under the cursor: its art, name, code, element and
//! damage; the dealt chips' icons and codes; the picked chips' icons). The
//! frontend's custom screen (nettai-render `custom`) composes those tile
//! numbers itself, so the blocks here keep the tile numbers they load at.

use crate::{MapEntry, Palette, Tiles, Versioned};

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
    /// The region whose ROMs the picture comes from, when not the pack's
    /// own (BN6: `"jp"`, a chip the US release cut and left a placeholder
    /// picture for). A console of another region shows something else.
    pub region: Option<String>,
    /// The game version whose ROM the chip's picture and icon are from,
    /// for a chip only its own version's ROM draws: a version's own chip
    /// (BN6's and BN5's version Giga chips, BN5's Phoenix and DethPhnx),
    /// which the other version's ROM draws as its counterpart, and BN6's
    /// Gregar and Falzar chips (each ROM has its own beast in both). A
    /// console of the other version shows another picture there.
    pub version: Option<String>,
}

/// The chip window's pictures for the slots that aren't chips (Beast Out's
/// is the version's: `VersionPictures`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotPictures {
    /// OK with nothing picked yet, and with chips picked ("sending").
    pub ok: Picture,
    pub ok_picked: Picture,
    /// ChpShufl's re-deal button and DustCross's scrap button.
    pub redeal: Picture,
    pub scrap: Picture,
    /// The slot kind battle mode 1 has (a Beast Out without the button's
    /// art).
    pub other: Picture,
}

/// What a game version's custom screen shows of its own: its Beast and its
/// Crosses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VersionPictures {
    /// The Beast Out button's picture in the chip window, with its palettes.
    pub beast_out: Picture,
    pub beast_out_palettes: Vec<Palette>,
    /// The Beast Out button (4x2 each): selectable, unavailable, battle
    /// mode 1's, none.
    pub beast_buttons: Tiles,
    /// The navis' emblems (2x2 each; MegaMan's are the version's).
    pub emblems: Tiles,
    /// The Cross window's names (9x2 each): the version's five Crosses on
    /// the cursor's row, then on the others' (`dword_86E7DCC`).
    pub cross_names: Tiles,
    /// Background palette 10 in the Cross window: the Cross under the
    /// cursor's, then a used one's (`dword_86E944C`).
    pub cross_palettes: Vec<Palette>,
}

/// Where the screen's blocks go among the HUD layer's tile numbers, which
/// its window map and patch list count with: BN6's (the default, a pack
/// that says none), or a game's whose window is laid out otherwise (BN5's
/// smaller frame puts everything after it lower).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomLayout {
    /// The picked-chip column's cells and the late turns' block.
    pub column_cells: u16,
    pub turn_limit: u16,
    /// The chip window's name (8 cells of 2 tiles), picture (7x6), code
    /// (1x2), element icon (2x2) and damage (3 cells): the patch list's
    /// first runs.
    pub name: u16,
    pub art: u16,
    pub code: u16,
    pub element: u16,
    pub digits: u16,
    /// The slots (6 tiles each, as their kinds take them) and the special
    /// button after them (slot 11's: Beast Out's 4x2 in BN6, the soul
    /// button's 3x2 in BN5), then the picked column's icons.
    pub slots: u16,
    pub column_icons: u16,
    /// The enemy names' bar, and the Cross window's names.
    pub name_bar: u16,
    pub cross_names: u16,
    /// The color a hidden slot's tiles and a slot's blank code are filled
    /// with (BN6's `byte_802A700`: 1).
    pub slot_blank: u8,
    /// The cursor over OK and over the special slot under it.
    pub ok_cursor: CursorPlace,
    pub special_cursor: CursorPlace,
    /// Whether the chip window shows the re-deal button's uses left, a
    /// digit in the damage's last cell (BN5's Shuffle, 0x080245F2; BN6's
    /// `sub_80287A4` reads the count and draws nothing).
    pub button_uses: bool,
}

/// Where the cursor's corners go over a slot (`sub_8028820`): the slot's
/// place (`jt_802886C`'s routines) and the four 8x8 corners of each of its
/// two frames, each (y, x, hflip, vflip) from the place less 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CursorPlace {
    pub x: i16,
    pub y: i16,
    pub corners: [[(i8, i8, bool, bool); 4]; 2],
}

impl CustomLayout {
    /// BN6's (`sub_8026840`, `sub_8028250`, `byte_8029DF8`; the cursor over
    /// OK `sub_80288D0` and `byte_80288E4`, over Beast Out `sub_8028904`
    /// and `byte_8028918`).
    pub const BN6: CustomLayout = CustomLayout {
        column_cells: 0x89,
        turn_limit: 0x8D,
        name: 0x9B,
        art: 0xAB,
        code: 0xD5,
        element: 0xD7,
        digits: 0xDB,
        slots: 0xE1,
        column_icons: 0x125,
        name_bar: 0x1D6,
        cross_names: 0x139,
        slot_blank: 1,
        ok_cursor: CursorPlace {
            x: 0x58 + 3,
            y: 0x70 - 2,
            corners: [
                [(2, 1, false, false), (2, 0x16, true, false), (0x14, 0x16, true, true), (0x14, 1, false, true)],
                [(4, 3, false, false), (4, 0x14, true, false), (0x12, 0x14, true, true), (0x12, 3, false, true)],
            ],
        },
        special_cursor: CursorPlace {
            x: 0x58 + 3,
            y: 0x88 - 1,
            corners: [
                [(2, 1, false, false), (2, 0x16, true, false), (0xE, 0x16, true, true), (0xE, 1, false, true)],
                [(3, 2, false, false), (3, 0x15, true, false), (0xD, 0x15, true, true), (0xD, 2, false, true)],
            ],
        },
        button_uses: false,
    };
}

impl Default for CustomLayout {
    fn default() -> CustomLayout {
        CustomLayout::BN6
    }
}

/// A button the screen draws by its name (a system's button,
/// docs/design/rules-in-luau.md §4.8, or BN5's soul button): its tiles in
/// the slots' row (`width` x `height` tiles a state, row by row: selectable,
/// unavailable, then the game's others) and its picture in the chip window
/// with that picture's palettes by state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ButtonPictures {
    pub width: u8,
    pub height: u8,
    pub tiles: Tiles,
    pub picture: Picture,
    pub palettes: Vec<Palette>,
    /// The icons (2x2 each) the picked column shows for what the button
    /// gives, by its number (BN5's souls by their number, 13 Chaos Unison's:
    /// 0x08024010's table), and the palette of the sprite the icon flies as
    /// (BN5's state 9).
    pub icons: Tiles,
    pub icon_palette: Palette,
    /// The flying icon's palette on a console of another game version,
    /// where it differs, by version (BN5: Team Colonel's has another
    /// outline color, its ROM's 0x0874BDBC).
    pub icon_palettes: Vec<(String, Palette)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomScreen {
    /// Where the blocks below go among the layer's tile numbers.
    pub layout: CustomLayout,
    /// The buttons drawn by name that the fields below don't hold (BN5's
    /// "soul"), by name.
    pub buttons: Vec<(String, ButtonPictures)>,
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
    /// The slot icons' palette (11), grayed out (12), and palette 14.
    pub icon_palette: Palette,
    pub gray_palette: Palette,
    pub other_palette: Palette,
    // ---- The chip window.
    pub chip_art: Vec<ChipArt>,
    pub pictures: SlotPictures,
    /// Chip codes as 8x16 glyphs (A..Z, '*', then one no chip has).
    pub codes: Tiles,
    /// Element icons (2x2) and the six colors each puts in palette 11
    /// from color 10.
    pub elements: Tiles,
    pub element_colors: Vec<[u16; 6]>,
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
    /// ChpShufl's and DustCross's buttons over slots 8 and 9 (12 tiles
    /// each: two slots' icon and code), by state (the scrap button's
    /// fourth is its pressed look).
    pub redeal_buttons: Tiles,
    pub scrap_buttons: Tiles,
    /// The pictures by game version.
    pub versioned: Versioned<VersionPictures>,
    // ---- Sprites.
    /// The cursor's corner (two frames, 8x8 each).
    pub cursor: Tiles,
    /// The Cross window's cursor: its corner and its edge, in two frames
    /// (8x8 each; `dword_86E57FC`, sprite tile 0x392), and its palette
    /// (sprite palette 14, the battle's).
    pub cross_cursor: Tiles,
    pub cross_cursor_palette: Palette,
    /// The emblems' palettes (the cursor's too), and which emblem and
    /// palette a navi shows by its number.
    pub emblem_palettes: Vec<Palette>,
    pub emblem_of: Vec<u8>,
    pub emblem_palette_of: Vec<u8>,
    /// The Regular chip's frame (two frames of 4x4).
    pub regular: Tiles,
    /// The Program Advance animation's names' first four colors
    /// (background palette 10), three sets it steps through
    /// (`byte_802BA48`).
    pub advance_name_colors: Vec<[u16; 4]>,
    /// The other languages' pictures with words (`crate::lettering`).
    pub languages: Vec<(String, crate::CustomLettering)>,
}

impl CustomScreen {
    /// A chip's picture by its key (with the region it comes from).
    pub fn chip_art(&self, key: &str) -> Option<&ChipArt> {
        self.chip_art.iter().find(|a| a.key == key).filter(|a| !a.picture.tiles.is_empty())
    }

    /// Whether the bundle has no custom screen graphics (a pack always
    /// has them; a bundle made in a test may not).
    pub fn is_empty(&self) -> bool {
        self.window_tiles.is_empty()
    }

    /// The button named `name` (`buttons`).
    pub fn button(&self, name: &str) -> Option<&ButtonPictures> {
        self.buttons.iter().find(|(n, _)| n == name).map(|(_, b)| b)
    }
}
