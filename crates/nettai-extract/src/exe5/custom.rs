//! The custom screen's graphics. EXE5's
//! screen is EXE6's shared screen (`sub_8026A28`'s counterpart): the same
//! window map and patch list (EXE5's special button, the soul button under
//! OK, is 3x2 where EXE6's Beast Out button is 4x2), the same slots and
//! picked column, with its own frame (0x44 tiles to EXE6's 0x87, so every
//! block after it loads lower: `CustomLayout`), its own pictures and EXE5's
//! element icons (13 to EXE6's 11). It has no Cross window and no Beast Out:
//! its buttons (`CustomScreen::buttons`) are the soul button under OK and,
//! over slots 8 and 9, Shuffle's re-deal and Arm Change.
//! Read where the counterparts of the EXE6 routines the EXE6 module reads load
//! them (the HUD's load list at 0x0801B54C, the window's opening, the chip
//! window's 0x080242FA and 0x080244F8 on, the emblems' 0x08023F68): each
//! block by where the game's own pointer to it is in each of the four ROMs
//! (`Place`), so any ROM gives what it holds. The window and its parts are
//! the same in the four; the chip window's worded pictures and the soul
//! button's label are each release's own; a team's emblems its version's.

use crate::decode::patches;
use crate::exe5::hud::palette;
use crate::exe5::rom::{Place, Rom, Roms, Source, Version};
use nettai_assets::{
    ButtonPictures, ButtonSets, ChipArt, CursorPlace, CustomLayout, CustomScreen, Emblem, MapEntry,
    Palette, PatchList, Picture, SlotPictures, Tiles, VersionPictures, Versioned, WindowLabel,
};
use nettai_content::names::AssetNames;

/// The window frame's tiles, loaded at tile 1 (0x44 tiles), the picked
/// column's cells (at 0x47), the late turns' block (0x4B) and the enemy
/// names' bar (0x1B6): the HUD's load list.
const WINDOW_TILES: (Place, usize) = (Place::pointer_in_each([0x0801_B588, 0x0801_B584, 0x0801_B554, 0x0801_B550]), 0x880);
const COLUMN_CELLS: (Place, usize) = (Place::pointer_in_each([0x0801_B594, 0x0801_B590, 0x0801_B560, 0x0801_B55C]), 0x80);
const TURN_LIMIT: (Place, usize) = (Place::pointer_in_each([0x0801_B5D0, 0x0801_B5CC, 0x0801_B59C, 0x0801_B598]), 0x1C0);
const NAME_BAR: (Place, usize) = (Place::pointer_in_each([0x0801_B5E8, 0x0801_B5E4, 0x0801_B5B4, 0x0801_B5B0]), 0x80);
/// The window's 15x20 map (one: EXE5 has no Cross tab) and its patch list,
/// from tile 0x59.
const WINDOW_MAP: Place = Place::pointer_in_each([0x0802_2D48, 0x0802_2D44, 0x0802_2D04, 0x0802_2D00]);
const WINDOW_PATCHES: (Place, u16) = (Place::pointer_in_each([0x0802_2D50, 0x0802_2D4C, 0x0802_2D0C, 0x0802_2D08]), 0x59);
const MAP_CELLS: u32 = 15 * 20;
/// Where the blocks load (EXE6's, all lower by the frame's 0x42 tiles, the
/// column's icons by 0x44: the special button is two tiles narrower); the
/// hidden slots' fill (0x08025D44: solid 2, EXE6's solid 1); the cursor
/// over OK (0x08024704: at (0x58, 0x70), EXE6's (0x5B, 0x6E)), its corners
/// read from `CURSOR_CORNERS`.
const LAYOUT: CustomLayout = CustomLayout {
    column_cells: 0x47,
    turn_limit: 0x4B,
    // (0x08025970's map: palette 9.)
    turn_limit_palette: 9,
    name: 0x59,
    art: 0x69,
    code: 0x93,
    element: 0x95,
    digits: 0x99,
    slots: 0x9F,
    column_icons: 0xE1,
    name_bar: 0x1B6,
    form_names: 0,
    slot_blank: 2,
    detail_blank: 8,
    empty_palette: None,
    ok_cursor: CursorPlace {
        x: 0x58,
        y: 0x70,
        corners: [[(0, 0, false, false); 4]; 2],
    },
    regular: (2, 100),
    column_columns: [11, 14],
    column_row: 3,
    form_names_at: [1, 1],
};
/// The cursor over the soul button (0x08024734: at (0x58, 0x88)), its
/// corners read from `CURSOR_CORNERS`; and over a button two slots wide in
/// the slots' row (Shuffle's and Arm Change's, over slots 8 and 9: EXE6's
/// `sub_8028820`'s counterpart).
const SOUL_CURSOR: CursorPlace = CursorPlace {
    x: 0x58,
    y: 0x88,
    corners: [[(0, 0, false, false); 4]; 2],
};
const ROW_BUTTON_CURSOR: CursorPlace = CursorPlace {
    x: 0x38,
    y: 0x80,
    corners: [
        [
            (2, 2, false, false),
            (2, 0x1C, true, false),
            (0x14, 0x1C, true, true),
            (0x14, 2, false, true),
        ],
        [
            (4, 4, false, false),
            (4, 0x1A, true, false),
            (0x12, 0x1A, true, true),
            (0x12, 4, false, true),
        ],
    ],
};
/// The cursor's corners over OK and over the soul button (`sub_80288D0`'s
/// and `sub_8028904`'s counterparts' tables: four words a frame, y in the
/// low half, x and the flips (0x1000 h, 0x2000 v) in the high).
const CURSOR_CORNERS: (Place, Place) = (Place::pointer_in_each([0x0802_4710, 0x0802_4714, 0x0802_46CC, 0x0802_46D0]), Place::pointer_in_each([0x0802_4740, 0x0802_4744, 0x0802_46FC, 0x0802_4700]));

/// A cursor's corners from a table of them.
fn cursor_corners(rom: &Rom, a: u32) -> [[(i8, i8, bool, bool); 4]; 2] {
    std::array::from_fn(|f| {
        std::array::from_fn(|k| {
            let w = rom.u32(a + 16 * f as u32 + 4 * k as u32);
            let hi = (w >> 16) as u16;
            (
                w as u16 as i8,
                (hi & 0xFF) as i8,
                hi & 0x1000 != 0,
                hi & 0x2000 != 0,
            )
        })
    })
}
/// Background palette 9 by the chip window's class (standard, mega, giga,
/// dark), and palettes 11, 12 and 14 (the HUD's load list).
const FRAME_PALETTES: (Place, u32) = (Place::pointer_in_each([0x0802_464C, 0x0802_4650, 0x0802_4608, 0x0802_460C]), 4);
const ICON_PALETTE: Place = Place::pointer_in_each([0x0801_B5A0, 0x0801_B59C, 0x0801_B56C, 0x0801_B568]);
const GRAY_PALETTE: Place = Place::pointer_in_each([0x0801_B5AC, 0x0801_B5A8, 0x0801_B578, 0x0801_B574]);
const OTHER_PALETTE: Place = Place::pointer_in_each([0x0801_B5C4, 0x0801_B5C0, 0x0801_B590, 0x0801_B58C]);
/// The chip window's other pictures and their palettes (EXE6 `sub_80286D4`'s
/// OK, `sub_8028754`'s, `sub_80287A4`'s and `sub_802877C`'s counterparts:
/// 0x080244F8, 0x08024578, 0x080245C8, 0x080245A0). OK's, the other's and
/// the re-deal button's have words, each release's own; Arm Change's (the
/// scrap button's place) has none and is the same in the four ROMs.
pub(crate) const OK: (Place, Place) = (Place::pointer_in_each([0x0802_452C, 0x0802_4530, 0x0802_44E8, 0x0802_44EC]), Place::pointer_in_each([0x0802_4538, 0x0802_453C, 0x0802_44F4, 0x0802_44F8]));
pub(crate) const OK_PICKED: (Place, Place) = (Place::pointer_in_each([0x0802_4530, 0x0802_4534, 0x0802_44EC, 0x0802_44F0]), Place::pointer_in_each([0x0802_453C, 0x0802_4540, 0x0802_44F8, 0x0802_44FC]));
pub(crate) const OTHER: (Place, Place) = (Place::pointer_in_each([0x0802_4598, 0x0802_459C, 0x0802_4554, 0x0802_4558]), Place::pointer_in_each([0x0802_459C, 0x0802_45A0, 0x0802_4558, 0x0802_455C]));
pub(crate) const REDEAL: (Place, Place) = (Place::pointer_in_each([0x0802_4620, 0x0802_4624, 0x0802_45DC, 0x0802_45E0]), Place::pointer_in_each([0x0802_462C, 0x0802_4630, 0x0802_45E8, 0x0802_45EC]));
pub(crate) const SCRAP: (Place, Place) = (Place::pointer_in_each([0x0802_45C0, 0x0802_45C4, 0x0802_457C, 0x0802_4580]), Place::pointer_in_each([0x0802_45C4, 0x0802_45C8, 0x0802_4580, 0x0802_4584]));
const PICTURE_BYTES: usize = 0x540;
/// Chip codes (8x16), element icons (2x2) and the six colors each brings,
/// damage digits (0-9, '?'): `sub_80284E2`'s counterpart.
const CODES: (Place, usize) = (Place::pointer_in_each([0x0802_4464, 0x0802_4468, 0x0802_4420, 0x0802_4424]), 28);
const ELEMENTS: Place = Place::pointer_in_each([0x0802_446C, 0x0802_4470, 0x0802_4428, 0x0802_442C]);
const ELEMENT_COLORS: Place = Place::pointer_in_each([0x0802_44A4, 0x0802_44A8, 0x0802_4460, 0x0802_4464]);
/// The icons and colors are by the chip record's family byte (+6), EXE5's
/// order (its first 13 are icons; past them the routine reads on into what
/// follows, as the pack does); the pack has them by the engine's family
/// numbers (`ChipFamily`, EXE6's order with EXE5's recovery and invisible
/// after it): for each, EXE5's family byte.
const FAMILY_BYTES: [u32; 15] = [
    0,  // fire
    1,  // aqua
    2,  // elec
    3,  // wood
    5,  // plus
    6,  // sword
    8,  // cursor
    9,  // summon (EXE5's obstacles)
    10, // wind
    11, // break
    12, // null
    13, // program advance
    14, // special
    4,  // recovery
    7,  // invisible
];
const DIGITS: (Place, usize) = (Place::pointer_in_each([0x0802_4474, 0x0802_4478, 0x0802_4430, 0x0802_4434]), 11);
/// The slots' codes (16x8), the empty slot's icon.
const SLOT_CODES: (Place, usize) = (Place::pointer_in_each([0x0802_405C, 0x0802_4060, 0x0802_4018, 0x0802_401C]), 28);
const EMPTY_ICON: Place = Place::pointer_in_each([0x0802_3FEC, 0x0802_3FF0, 0x0802_3FA8, 0x0802_3FAC]);
/// The re-deal and scrap buttons over slots 8 and 9 (EXE6 `sub_80282D2`'s
/// and `sub_8028340`'s counterparts: 12 tiles a state; the shared screen's,
/// which a netbattle doesn't offer).
const REDEAL_BUTTONS: (Place, usize) = (Place::pointer_in_each([0x0802_4178, 0x0802_417C, 0x0802_4134, 0x0802_4138]), 0x480);
const SCRAP_BUTTONS: (Place, usize) = (Place::pointer_in_each([0x0802_417C, 0x0802_4180, 0x0802_4138, 0x0802_413C]), 0x600);
/// The soul button (slot 11, EXE6's Beast Out's place): its tiles, 3x2 a
/// state (selectable, unavailable, pressed; the HUD's load list's to tile
/// 0xDB), and its picture in the chip window with a palette a state
/// (EXE6 `sub_802871C`'s counterpart, 0x08024540: by the slot's state).
/// The tiles have the button's label, each release's own ("UNITE" in the
/// US ROMs).
pub(crate) const SOUL_BUTTONS: (Place, usize) = (Place::pointer_in_each([0x0802_41BC, 0x0802_41C0, 0x0802_4178, 0x0802_417C]), 0x240);
/// The names EXE5's rules give their buttons (Soul Unison's `soul`,
/// SearchSoul's Shuffle as `redeal`, ColonelSoul's `arm_change`:
/// content/exe5/rules/souls and the souls' own folders), which the pack has
/// their looks by.
pub(crate) const SOUL_BUTTON: &str = "soul";
pub(crate) const REDEAL_BUTTON: &str = "redeal";
pub(crate) const ARM_CHANGE_BUTTON: &str = "arm_change";
/// Arm Change's chip over its button (0x08025508: the sprite's x 0x45, y
/// 0x84).
const ARM_CHANGE_HELD_AT: (i16, i16) = (0x45, 0x84);
const SOUL_PICTURE: Place = Place::pointer_in_each([0x0802_4570, 0x0802_4574, 0x0802_452C, 0x0802_4530]);
const SOUL_PALETTES: (Place, u32) = (Place::pointer_in_each([0x0802_4574, 0x0802_4578, 0x0802_4530, 0x0802_4534]), 2);
/// The souls' icons in the picked column, by soul number (13: Chaos Unison's; the
/// column's 0x08024010 from 0x0802341C), which the soul choice (state 9,
/// 0x0802330C) first flies as a sprite in this palette (sprite palette 13).
const SOUL_ICONS: (Place, usize) = (Place::pointer_in_each([0x0802_402C, 0x0802_4030, 0x0802_3FE8, 0x0802_3FEC]), 14 * 0x80);
/// (A version's own: Team Colonel's ROMs' has another color 9, the icons'
/// outline.)
const SOUL_ICON_PALETTE: Place = Place::pointer_in_each([0x0802_3354, 0x0802_3350, 0x0802_3310, 0x0802_330C]);
/// Sprites: the cursor's corner (two frames), the Regular chip's frame.
const CURSOR: (Place, usize) = (Place::pointer_in_each([0x0802_5D88, 0x0802_5D8C, 0x0802_5D44, 0x0802_5D48]), 0x40);
const REGULAR: (Place, usize) = (Place::pointer_in_each([0x0802_47D0, 0x0802_47D4, 0x0802_478C, 0x0802_4790]), 0x400);
/// The emblems (0x08023F68, EXE6 `sub_802812C`'s counterpart): a ROM's
/// seven pictures (four tiles each: MegaMan's, then its own team's six
/// navis'), the eight palettes, and which palette each of the thirteen
/// navis shows (Team ProtoMan US's 0x08023FC4). MegaMan's picture, the
/// palettes and the table are the same in the four ROMs.
const EMBLEMS: Place = Place::pointer_in_each([0x0802_3FB8, 0x0802_3FBC, 0x0802_3F74, 0x0802_3F78]);
const EMBLEM_PALETTES: Place = Place::pointer_in_each([0x0802_3FBC, 0x0802_3FC0, 0x0802_3F78, 0x0802_3F7C]);
const EMBLEM_PALETTE_OF: Place = Place::pointer_in_each([0x0802_3FC0, 0x0802_3FC4, 0x0802_3F7C, 0x0802_3F80]);
/// The navis that operate (NaviStats +0x29): MegaMan (0), Team ProtoMan's
/// six (1 to 6) and Team Colonel's (7 to 12).
const NAVI_COUNT: u8 = 13;
const TEAM_NAVIS: u8 = 6;
/// The Program Advance animation's names' colors (EXE6 `byte_802BA48`'s
/// counterpart: three sets of four).
const ADVANCE_NAME_COLORS: (Place, u32) = (Place::pointer_in_each([0x0802_7D90, 0x0802_7D94, 0x0802_7D4C, 0x0802_7D50]), 3);

/// A block's tiles in a source: placeholders if the source isn't there.
pub(crate) fn tiles(src: Source, (place, len): (Place, usize)) -> Tiles {
    if !src.is_present() {
        return crate::placeholders::tiles(len / 32);
    }
    crate::exe5::hud::tiles(src.rom, src.at(place), len)
}

/// The first `n` palettes of a block in a source.
fn palettes(src: Source, (place, n): (Place, u32)) -> Vec<Palette> {
    if !src.is_present() {
        return vec![crate::placeholders::PALETTE; n as usize];
    }
    (0..n).map(|i| palette(src.rom, src.at(place) + 32 * i)).collect()
}

/// A picture of the chip window in a source, with its palette.
pub(crate) fn picture(src: Source, (gfx, pal): (Place, Place)) -> Picture {
    if !src.is_present() {
        return crate::placeholders::picture();
    }
    Picture {
        tiles: crate::exe5::hud::tiles(src.rom, src.at(gfx), PICTURE_BYTES),
        palette: palette(src.rom, src.at(pal)),
    }
}

/// The navis' emblems: navi n's picture is its ROM's nth (Team Colonel's
/// routine takes 6 off a team navi's number, 0x08023F9C: a ROM has
/// MegaMan's and its own team's six) and its palette the one the table
/// gives n, the same table and palettes in every ROM. The pack has each
/// navi's own, from a ROM that has it, under the navi's key (`names`): a
/// navi the content doesn't have yet has none.
fn emblems(roms: &Roms, names: &AssetNames) -> Vec<Emblem> {
    (0..NAVI_COUNT)
        .filter_map(|n| {
            let (src, picture) = match n {
                0 => (roms.any(), 0),
                _ if n <= TEAM_NAVIS => (roms.of(Version::ProtoMan), n),
                _ => (roms.of(Version::Colonel), n - TEAM_NAVIS),
            };
            if !src.is_present() {
                return None;
            }
            let rom = src.rom;
            let color = rom.u8(src.at(EMBLEM_PALETTE_OF) + n as u32) as u32;
            Some(Emblem {
                navi: names.navis.get(&n)?.clone(),
                tiles: crate::exe5::hud::tiles(rom, src.at(EMBLEMS) + 0x80 * picture as u32, 0x80),
                palette: palette(rom, src.at(EMBLEM_PALETTES) + 32 * color),
            })
        })
        .collect()
}

/// A button two slots wide in the slots' row (twelve tiles a state, the
/// two slots' icons and codes), with its picture in the chip window: the
/// states from `art`, the picture from `words`.
fn row_button(art: Source, words: Source, buttons: (Place, usize), details: (Place, Place)) -> ButtonPictures {
    if !art.is_present() {
        return crate::placeholders::button(2, 3);
    }
    let picture = picture(words, details);
    ButtonPictures {
        width: 2,
        height: 3,
        tiles: tiles(art, buttons),
        sets: ButtonSets::Each,
        cursor: ROW_BUTTON_CURSOR,
        palettes: vec![picture.palette],
        picture,
        ..ButtonPictures::default()
    }
}

/// The soul button: its states' tiles have its label (`words`), its picture
/// and its souls' icons are the same in every ROM (`art`), and the icons'
/// palette is each version's own (the pack's is Team ProtoMan's, with Team
/// Colonel's beside it where it is another).
fn soul_button(roms: &Roms) -> ButtonPictures {
    let (art, words) = (roms.any(), roms.us());
    if !art.is_present() {
        return crate::placeholders::button(3, 2);
    }
    let icon_palette = |src: Source| palette(src.rom, src.at(SOUL_ICON_PALETTE));
    let own = icon_palette(roms.preferring(Version::ProtoMan));
    let colonel = roms.of(Version::Colonel);
    ButtonPictures {
        width: 3,
        height: 2,
        tiles: tiles(words, SOUL_BUTTONS),
        // (0x08024540: gray when unavailable or picked.)
        sets: ButtonSets::Other,
        cursor: CursorPlace {
            corners: cursor_corners(art.rom, art.at(CURSOR_CORNERS.1)),
            ..SOUL_CURSOR
        },
        picture: picture(art, (SOUL_PICTURE, SOUL_PALETTES.0)),
        palettes: palettes(art, SOUL_PALETTES),
        icons: tiles(art, SOUL_ICONS),
        icon_palette: own,
        icon_palettes: if colonel.is_present() && icon_palette(colonel) != own {
            vec![(Version::Colonel.name().into(), icon_palette(colonel))]
        } else {
            Vec::new()
        },
        ..ButtonPictures::default()
    }
}

/// The custom screen's graphics; `names` gives the navis' emblems their
/// navis' keys, `chip_art` is the chips' pictures (graphics.rs). The window
/// and its parts are whichever ROM's is there (`drawn`), the worded
/// pictures a US ROM's. What no ROM given holds is a placeholder.
pub fn custom(roms: &Roms, names: &AssetNames, chip_art: Vec<ChipArt>) -> CustomScreen {
    let (art, words) = (roms.any(), roms.us());
    // SearchSoul's Shuffle (EXE6's re-deal button's counterpart, 0x080245C8:
    // its uses left in the chip window, 0x080245F2) and ColonelSoul's Arm
    // Change (0x0802415A and 0x080245A0, in EXE6's scrap button's place:
    // picked, it looks on offer, the chip it holds drawn over it; its
    // picture has no words).
    let mut redeal = row_button(art, words, REDEAL_BUTTONS, REDEAL);
    let mut arm_change = row_button(art, art, SCRAP_BUTTONS, SCRAP);
    if art.is_present() {
        redeal.uses_digit = true;
        arm_change.sets = ButtonSets::Unavailable;
        arm_change.held_at = Some(ARM_CHANGE_HELD_AT);
    }
    let c = CustomScreen {
        layout: LAYOUT,
        buttons: vec![
            (SOUL_BUTTON.into(), soul_button(roms)),
            (REDEAL_BUTTON.into(), redeal),
            (ARM_CHANGE_BUTTON.into(), arm_change),
        ],
        // (No version shows anything of its own here: the base's name is
        // the game's base version's.)
        versioned: Versioned::new(Version::ProtoMan.name(), VersionPictures::default()),
        chip_art,
        pictures: SlotPictures {
            ok: picture(words, OK),
            ok_picked: picture(words, OK_PICKED),
            other: picture(words, OTHER),
        },
        emblems: emblems(roms, names),
        ..crate::placeholders::custom()
    };
    if art.is_present() { drawn(art, c) } else { c }
}


/// A window label ([`WindowLabel`]): its name, its tiles, its letters' rows
/// and colors.
fn label(name: &str, (first, last): (u16, u16), (top, bottom): (u8, u8), letters: &[u8]) -> WindowLabel {
    WindowLabel { name: name.into(), first, last, top, bottom, letters: letters.to_vec() }
}

/// `c` with what the four ROMs have alike, read from `src`.
fn drawn(src: Source, c: CustomScreen) -> CustomScreen {
    let (rom, at) = (src.rom, |place: Place| src.at(place));
    let glyphs = |(place, n): (Place, usize)| tiles(src, (place, 0x40 * n));
    let labels = vec![
        // BATTLE CHIP on the window's top frame (tiles 2 to 10, color 3 on
        // 6), CHIP SELECT over the slots (0x27 to 0x2D, 4 on 2).
        label("battle-chip", (0x02, 0x0A), (1, 5), &[3]),
        label("chip-select", (0x27, 0x2D), (3, 6), &[4]),
    ];
    let map = |a: u32| -> Vec<MapEntry> {
        (0..MAP_CELLS)
            .map(|i| MapEntry::from_gba(rom.u16(a + 2 * i)))
            .collect()
    };
    CustomScreen {
        layout: CustomLayout {
            ok_cursor: CursorPlace {
                corners: cursor_corners(rom, at(CURSOR_CORNERS.0)),
                ..LAYOUT.ok_cursor
            },
            ..LAYOUT
        },
        labels,
        window_tiles: tiles(src, WINDOW_TILES),
        column_cells: tiles(src, COLUMN_CELLS),
        turn_limit: tiles(src, TURN_LIMIT),
        name_bar: tiles(src, NAME_BAR),
        window_maps: vec![map(at(WINDOW_MAP))],
        window_patches: patches(rom, (at(WINDOW_PATCHES.0), WINDOW_PATCHES.1)),
        form_list_maps: Vec::new(),
        form_list_patches: PatchList::default(),
        frame_palettes: palettes(src, FRAME_PALETTES),
        icon_palette: palette(rom, at(ICON_PALETTE)),
        gray_palette: palette(rom, at(GRAY_PALETTE)),
        other_palette: palette(rom, at(OTHER_PALETTE)),
        codes: glyphs(CODES),
        elements: Tiles {
            pixels: FAMILY_BYTES
                .iter()
                .flat_map(|&b| crate::exe5::hud::tiles(rom, at(ELEMENTS) + 0x80 * b, 0x80).pixels)
                .collect(),
        },
        element_colors: FAMILY_BYTES
            .iter()
            .map(|&b| {
                std::array::from_fn(|i| rom.u16(at(ELEMENT_COLORS) + 12 * b + 2 * i as u32) & 0x7FFF)
            })
            .collect(),
        digits: glyphs(DIGITS),
        slot_codes: glyphs(SLOT_CODES),
        empty_icon: tiles(src, (EMPTY_ICON, 0x80)),
        cursor: tiles(src, CURSOR),
        form_list_cursor: Tiles::default(),
        form_list_cursor_palette: [0; 16],
        regular: tiles(src, REGULAR),
        advance_name_colors: (0..ADVANCE_NAME_COLORS.1)
            .map(|i| {
                std::array::from_fn(|k| {
                    rom.u16(at(ADVANCE_NAME_COLORS.0) + 8 * i + 2 * k as u32) & 0x7FFF
                })
            })
            .collect(),
        ..c
    }
}
