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

/// A navi's emblem over the picked column (`sub_8029C08`), under its navi's
/// key: its 2x2 tiles and its palette, which is sprite palette 11 while
/// the navi's console has the screen up (the cursor and the Regular chip's
/// frame are drawn in it too: `sub_802812C`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Emblem {
    pub navi: String,
    pub tiles: Tiles,
    pub palette: Palette,
}

/// A chip's picture, under its chip's key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChipArt {
    pub key: String,
    pub picture: Picture,
    /// The game version whose ROM the chip's picture and icon are from,
    /// for a chip only its own version's ROM draws: a version's own chip
    /// (EXE6's and EXE5's version Giga chips, EXE5's Phoenix and DethPhnx),
    /// which the other version's ROM draws as its counterpart. A console of
    /// the other version shows another picture there.
    pub version: Option<String>,
}

/// The chip window's pictures for the slots that are neither chips nor
/// buttons (a button's is its own: `ButtonPictures::picture`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotPictures {
    /// OK with nothing picked yet, and with chips picked ("sending").
    pub ok: Picture,
    pub ok_picked: Picture,
    /// The slot kind battle mode 1 has (a Beast Out without the button's
    /// art).
    pub other: Picture,
}

/// What a game version's custom screen shows of its own: its buttons (its
/// Beast's) and its Crosses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VersionPictures {
    /// The buttons the version has its own look of, by name (EXE6's
    /// "beast_out": the version's Beast).
    pub buttons: Vec<(String, ButtonPictures)>,
    /// The form list window's names (9x2 each): the version's forms on the
    /// cursor's row, then on the others' (EXE6's Cross window's five
    /// Crosses, `dword_86E7DCC`).
    pub form_names: Tiles,
    /// Background palette 10 in the form list window: the form under the
    /// cursor's, then a used one's (EXE6's, `dword_86E944C`).
    pub form_name_palettes: Vec<Palette>,
}

impl VersionPictures {
    /// The version's own look of the button named `name`.
    pub fn button(&self, name: &str) -> Option<&ButtonPictures> {
        self.buttons.iter().find(|(n, _)| n == name).map(|(_, b)| b)
    }
}

/// Where the screen's blocks go among the HUD layer's tile numbers, which
/// its window map and patch list count with: a game's own (EXE5's smaller
/// frame puts everything after it lower than EXE6's). A pack says its
/// game's; the default is no layout (zeros: an empty bundle's), never a
/// game's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CustomLayout {
    /// The picked-chip column's cells and the late turns' block.
    pub column_cells: u16,
    pub turn_limit: u16,
    /// The late turns' block's palette ("FINAL TURN": EXE6's and EXE5's 9,
    /// `sub_8029D34`'s map; EXE4's 14, 0x0801EE9C's).
    pub turn_limit_palette: u8,
    /// The chip window's name (8 cells of 2 tiles), picture (7x6), code
    /// (1x2), element icon (2x2) and damage (3 cells): the patch list's
    /// first runs.
    pub name: u16,
    pub art: u16,
    pub code: u16,
    pub element: u16,
    pub digits: u16,
    /// The slots (6 tiles each, as their kinds take them) and the special
    /// button after them (slot 11's: Beast Out's 4x2 in EXE6, the soul
    /// button's 3x2 in EXE5), then the picked column's icons.
    pub slots: u16,
    pub column_icons: u16,
    /// The enemy names' bar, and the form list window's names.
    pub name_bar: u16,
    pub form_names: u16,
    /// The color a hidden slot's tiles and a slot's blank code are filled
    /// with (EXE6's `byte_802A700`: 1).
    pub slot_blank: u8,
    /// The color the chip window's blank code and damage cells are filled
    /// with (EXE6's `sub_802869E` and EXE5's: 8; EXE4's 0x08020E5C: 7).
    pub detail_blank: u8,
    /// The palette the empty icon is drawn in where a slot or a cell of
    /// the picked column has no chip: an empty or picked slot's, an empty
    /// cell's (EXE4's 9: 0x0801FB00's and 0x0801FB6E's tables). None: the
    /// patch list's (EXE6's and EXE5's).
    pub empty_palette: Option<u8>,
    /// The cursor over OK (a button's is its own: `ButtonPictures::cursor`).
    pub ok_cursor: CursorPlace,
}

/// Where the cursor's corners go over a slot (`sub_8028820`): the slot's
/// place (`jt_802886C`'s routines) and the four 8x8 corners of each of its
/// two frames, each (y, x, hflip, vflip) from the place less 3.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorPlace {
    pub x: i16,
    pub y: i16,
    pub corners: [[(i8, i8, bool, bool); 4]; 2],
}

/// Which of a button's tile sets a slot in a state shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSets {
    /// One a state: selectable, unavailable, picked (EXE6's re-deal and
    /// scrap buttons).
    #[default]
    Each,
    /// The second for unavailable and picked alike (EXE6's Beast Out
    /// button, EXE5's soul button).
    Other,
    /// The second for unavailable alone (EXE5's Arm Change, 0x0802415A:
    /// picked, it looks on offer, the chip it holds drawn over it).
    Unavailable,
}

/// A button the screen draws by its name (a button of the rules,
/// docs/design/rules-in-luau.md §4.8: the name its content registers it
/// under). Its tiles among the slots': `width` x `height` a cell (a slot's
/// 2x3, its icon then its code; the special slot's own size), a set its
/// cells', set after set (selectable, unavailable, then the game's others;
/// `sets` says which a state shows). And its picture in the chip window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ButtonPictures {
    pub width: u8,
    pub height: u8,
    pub tiles: Tiles,
    pub sets: ButtonSets,
    /// The set its slot shows while the button isn't there (EXE6's Beast
    /// Out button's fourth); none: the window's fill.
    pub hidden: Option<u8>,
    /// The cursor over it (`sub_8028820`).
    pub cursor: CursorPlace,
    /// Its picture in the chip window, and that picture's palettes: its
    /// own first, then the others its game has for it (EXE5's soul
    /// button's second is a Chaos Unison's).
    pub picture: Picture,
    pub palettes: Vec<Palette>,
    /// Whether the chip window shows its uses left, a digit in the
    /// damage's last cell (EXE5's Shuffle, 0x080245F2; EXE6's
    /// `sub_80287A4` reads the count and draws nothing).
    pub uses_digit: bool,
    /// Where the icon of the chip it holds is drawn, a sprite over it
    /// (EXE5's Arm Change, 0x080254F4).
    pub held_at: Option<(i16, i16)>,
    /// The icons (2x2 each) the picked column shows for what the button
    /// gives, by its number (EXE5's souls by their number, 13 Chaos Unison's:
    /// 0x08024010's table), and the palette of the sprite the icon flies as
    /// (EXE5's state 9).
    pub icons: Tiles,
    pub icon_palette: Palette,
    /// The flying icon's palette on a console of another game version,
    /// where it differs, by version (EXE5: Team Colonel's has another
    /// outline color, its ROM's 0x0874BDBC).
    pub icon_palettes: Vec<(String, Palette)>,
    /// Where the button is drawn when no patch of the window's places it:
    /// its cells on the window's map at a column and row, from its own
    /// tile numbers, in a palette of the window's (EXE4's UNITE button,
    /// 0x0801FF14: 3x2 at column 11, row 17, from tile 0x52, in palette 9).
    /// None: the patch list's place in the slots' run, its tiles among the
    /// slots'.
    pub place: Option<ButtonPlace>,
}

/// A button's own place on the window (`ButtonPictures::place`): the
/// column and row of its first cell, the tile number its tiles load at,
/// and the window's palette its cells are in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonPlace {
    pub x: u8,
    pub y: u8,
    pub first_tile: u16,
    pub palette: u8,
}

/// The chip window's element icon drawn as a sprite (EXE4's, 0x0801EECC: a
/// 16x16 sprite at the window's column 3, row 10, in a palette of its
/// own), rather than as tiles of the window's map (EXE6's and EXE5's, in
/// the window's palette 11 with each element's colors).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ElementSprite {
    /// Its place from the window's left edge (which slides with it) and
    /// the screen's top.
    pub x: i16,
    pub y: i16,
    pub palette: Palette,
}

/// An emblem the screen draws on the window's map (EXE4's, 0x08020028:
/// the orb over the picked column, 2x3 cells at column 12, row 0), by
/// frames: its tiles from `first_tile`, its frames' map entries (`width`
/// x `height` each, row by row), and the steps its turn goes through after
/// a pick (`Screen::look`'s step: a frame, or a step that holds the last;
/// the first frame at rest).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowEmblem {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
    pub first_tile: u16,
    pub tiles: Tiles,
    pub frames: Vec<Vec<MapEntry>>,
    pub steps: Vec<Option<u8>>,
}

impl WindowEmblem {
    /// The frame shown at a turn's step (from 1; 0: at rest): the step's
    /// own, or the last a step before it showed.
    pub fn frame(&self, step: usize) -> usize {
        if step == 0 {
            return 0;
        }
        self.steps.iter().take(step).rev().find_map(|f| *f).map_or(0, |f| f as usize)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomScreen {
    /// Where the blocks below go among the layer's tile numbers.
    pub layout: CustomLayout,
    /// The buttons, by name (a version's own look of one is its
    /// `VersionPictures`').
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
    /// The form list window's maps: its opening steps, then the window
    /// with one to five forms (EXE6's Cross window); and its patches.
    pub form_list_maps: Vec<Vec<MapEntry>>,
    pub form_list_patches: PatchList,
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
    /// The pictures by game version.
    pub versioned: Versioned<VersionPictures>,
    // ---- Sprites.
    /// The cursor's corner (two frames, 8x8 each).
    pub cursor: Tiles,
    /// The form list window's cursor: its corner and its edge, in two frames
    /// (8x8 each; `dword_86E57FC`, sprite tile 0x392), and its palette
    /// (sprite palette 14, the battle's).
    pub form_list_cursor: Tiles,
    pub form_list_cursor_palette: Palette,
    /// The navis' emblems, by navi key.
    pub emblems: Vec<Emblem>,
    /// The Regular chip's frame (two frames of 4x4).
    pub regular: Tiles,
    /// The Program Advance animation's names' first four colors
    /// (background palette 10), three sets it steps through
    /// (`byte_802BA48`).
    pub advance_name_colors: Vec<[u16; 4]>,
    /// The other languages' pictures with words (`crate::lettering`).
    pub languages: Vec<(String, crate::CustomLettering)>,
    /// The chip window's element icon as a sprite (EXE4's); none: as the
    /// window's tiles (`layout.element`).
    pub element_sprite: Option<ElementSprite>,
    /// The palette of the cursor and the Regular chip's frame where it is
    /// their own (EXE4's sprite palette 13, 0x0870C360); none: the navi's
    /// emblem's (EXE6's and EXE5's sprite palette 11).
    pub cursor_palette: Option<Palette>,
    /// The emblem the screen draws on its window by frames (EXE4's), in
    /// place of a navi's emblem sprite (`emblems`).
    pub window_emblem: Option<WindowEmblem>,
}

impl CustomScreen {
    /// A chip's picture by its key.
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

    /// The emblem of the navi with key `navi`.
    pub fn emblem(&self, navi: &str) -> Option<&Emblem> {
        self.emblems.iter().find(|e| e.navi == navi)
    }
}
