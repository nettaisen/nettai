//! The custom screen's graphics, from the US Team ProtoMan ROM. EXE5's
//! screen is EXE6's shared screen (`sub_8026A28`'s counterpart): the same
//! window map and patch list (EXE5's special button, the soul button under
//! OK, is 3x2 where EXE6's Beast Out button is 4x2), the same slots and
//! picked column, with its own frame (0x44 tiles to EXE6's 0x87, so every
//! block after it loads lower: `CustomLayout`), its own pictures and EXE5's
//! element icons (13 to EXE6's 11). It has no Cross window and no Beast Out:
//! its buttons (`CustomScreen::buttons`) are the soul button under OK and,
//! over slots 8 and 9, Shuffle's re-deal and Arm Change.
//! Read where the counterparts of the EXE6 routines exe6-extract reads load
//! them (the HUD's load list at 0x0801B54C, the window's opening, the chip
//! window's 0x080242FA and 0x080244F8 on, the emblems' 0x08023F68).

use crate::hud::{palette, tiles};
use crate::rom::{Rom, Roms, Version};
use nettai_assets::{
    ButtonPictures, ButtonSets, ChipArt, CursorPlace, CustomLayout, CustomScreen, Emblem, MapEntry, MapPatch, PatchList, Picture, SlotPictures, Tiles,
    VersionPictures, Versioned,
};
use nettai_content::names::AssetNames;

/// The window frame's tiles, loaded at tile 1 (0x44 tiles), the picked
/// column's cells (at 0x47), the late turns' block (0x4B) and the enemy
/// names' bar (0x1B6): the HUD's load list.
const WINDOW_TILES: (u32, usize) = (0x086F_7DB0, 0x880);
const COLUMN_CELLS: (u32, usize) = (0x086F_8630, 0x80);
const TURN_LIMIT: (u32, usize) = (0x0873_CDF8, 0x1C0);
const NAME_BAR: (u32, usize) = (0x086F_B74C, 0x80);
/// The window's 15x20 map (one: EXE5 has no Cross tab) and its patch list,
/// from tile 0x59.
const WINDOW_MAP: u32 = 0x086F_B90C;
const WINDOW_PATCHES: (u32, u16) = (0x0802_3938, 0x59);
const MAP_CELLS: u32 = 15 * 20;
/// Where the blocks load (EXE6's, all lower by the frame's 0x42 tiles, the
/// column's icons by 0x44: the special button is two tiles narrower); the
/// hidden slots' fill (0x08025D44: solid 2, EXE6's solid 1); the cursor
/// over OK (0x08024704: at (0x58, 0x70), EXE6's (0x5B, 0x6E)), its corners
/// read from `CURSOR_CORNERS`.
const LAYOUT: CustomLayout = CustomLayout {
    column_cells: 0x47,
    turn_limit: 0x4B,
    name: 0x59,
    art: 0x69,
    code: 0x93,
    element: 0x95,
    digits: 0x99,
    slots: 0x9F,
    column_icons: 0xE1,
    name_bar: 0x1B6,
    cross_names: 0,
    slot_blank: 2,
    ok_cursor: CursorPlace { x: 0x58, y: 0x70, corners: [[(0, 0, false, false); 4]; 2] },
};
/// The cursor over the soul button (0x08024734: at (0x58, 0x88)), its
/// corners read from `CURSOR_CORNERS`; and over a button two slots wide in
/// the slots' row (Shuffle's and Arm Change's, over slots 8 and 9: EXE6's
/// `sub_8028820`'s counterpart).
const SOUL_CURSOR: CursorPlace = CursorPlace { x: 0x58, y: 0x88, corners: [[(0, 0, false, false); 4]; 2] };
const ROW_BUTTON_CURSOR: CursorPlace = CursorPlace {
    x: 0x38,
    y: 0x80,
    corners: [
        [(2, 2, false, false), (2, 0x1C, true, false), (0x14, 0x1C, true, true), (0x14, 2, false, true)],
        [(4, 4, false, false), (4, 0x1A, true, false), (0x12, 0x1A, true, true), (0x12, 4, false, true)],
    ],
};
/// The cursor's corners over OK and over the soul button (`sub_80288D0`'s
/// and `sub_8028904`'s counterparts' tables: four words a frame, y in the
/// low half, x and the flips (0x1000 h, 0x2000 v) in the high).
const CURSOR_CORNERS: (u32, u32) = (0x0802_4714, 0x0802_4744);

/// A cursor's corners from a table of them.
fn cursor_corners(rom: &Rom, a: u32) -> [[(i8, i8, bool, bool); 4]; 2] {
    std::array::from_fn(|f| {
        std::array::from_fn(|k| {
            let w = rom.u32(a + 16 * f as u32 + 4 * k as u32);
            let hi = (w >> 16) as u16;
            (w as u16 as i8, (hi & 0xFF) as i8, hi & 0x1000 != 0, hi & 0x2000 != 0)
        })
    })
}
/// Background palette 9 by the chip window's class (standard, mega, giga,
/// dark), and palettes 11, 12 and 14 (the HUD's load list).
const FRAME_PALETTES: (u32, usize) = (0x086F_AF2C, 4);
const ICON_PALETTE: u32 = 0x0874_AB18;
const GRAY_PALETTE: u32 = 0x0874_AAD8;
const OTHER_PALETTE: u32 = 0x086F_AFAC;
/// The chip window's other pictures and their palettes (EXE6 `sub_80286D4`'s
/// OK, `sub_8028754`'s, `sub_80287A4`'s and `sub_802877C`'s counterparts:
/// 0x080244F8, 0x08024578, 0x080245C8, 0x080245A0).
const OK: (u32, u32) = (0x0873_1DA8, 0x0873_4DA8);
const OK_PICKED: (u32, u32) = (0x0873_1868, 0x0873_4D28);
const OTHER: (u32, u32) = (0x0873_1868, 0x0873_4D08);
const REDEAL: (u32, u32) = (0x0873_1DA8, 0x0873_4D88);
const SCRAP: (u32, u32) = (0x0874_F5B8, 0x0874_FB18);
const PICTURE_BYTES: usize = 0x540;
/// Chip codes (8x16), element icons (2x2) and the six colors each brings,
/// damage digits (0-9, '?'): `sub_80284E2`'s counterpart.
const CODES: (u32, usize) = (0x086F_86B0, 28);
const ELEMENTS: u32 = 0x086F_8DB0;
const ELEMENT_COLORS: u32 = 0x086F_9430;
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
const DIGITS: (u32, usize) = (0x086F_9B4C, 11);
/// The slots' codes (16x8), the empty slot's icon.
const SLOT_CODES: (u32, usize) = (0x086F_AFCC, 28);
const EMPTY_ICON: u32 = 0x086F_B6CC;
/// The re-deal and scrap buttons over slots 8 and 9 (EXE6 `sub_80282D2`'s
/// and `sub_8028340`'s counterparts: 12 tiles a state; the shared screen's,
/// which a netbattle doesn't offer).
const REDEAL_BUTTONS: (u32, usize) = (0x086F_9E4C, 0x480);
const SCRAP_BUTTONS: (u32, usize) = (0x086F_A7CC, 0x600);
/// The soul button (slot 11, EXE6's Beast Out's place): its tiles, 3x2 a
/// state (selectable, unavailable, pressed; the HUD's load list's to tile
/// 0xDB), and its picture in the chip window with a palette a state
/// (EXE6 `sub_802871C`'s counterpart, 0x08024540: by the slot's state).
const SOUL_BUTTONS: (u32, usize) = (0x086F_BB64, SOUL_BUTTON_BYTES);
pub(crate) const SOUL_BUTTON_BYTES: usize = 0x240;
/// The names EXE5's content registers its buttons under (the souls
/// system's `soul`, SearchSoul's Shuffle as `redeal`, ColonelSoul's
/// `arm_change`: content/exe5/rules/souls and the souls' own folders),
/// which the pack has their looks by.
pub(crate) const SOUL_BUTTON: &str = "soul";
pub(crate) const REDEAL_BUTTON: &str = "redeal";
pub(crate) const ARM_CHANGE_BUTTON: &str = "arm_change";
/// Arm Change's chip over its button (0x08025508: the sprite's x 0x45, y
/// 0x84).
const ARM_CHANGE_HELD_AT: (i16, i16) = (0x45, 0x84);
const SOUL_PICTURE: u32 = 0x0873_22E8;
const SOUL_PALETTES: (u32, u32) = (0x0873_4D48, 2);
/// The souls' icons in the picked column, by soul number (13: Chaos Unison's; the
/// column's 0x08024010 from 0x0802341C), which the soul choice (state 9,
/// 0x0802330C) first flies as a sprite in this palette (sprite palette 13).
const SOUL_ICONS: (u32, usize) = (0x0874_9FB8, 14 * 0x80);
const SOUL_ICON_PALETTE: u32 = 0x0874_AAB8;
/// Team Colonel's ROM's (its 0x08023308's): color 9, the icons' outline,
/// is another.
const COLONEL_SOUL_ICON_PALETTE: u32 = 0x0874_BDBC;
/// Sprites: the cursor's corner (two frames), the Regular chip's frame.
const CURSOR: (u32, usize) = (0x086F_ACEC, 0x40);
const REGULAR: (u32, usize) = (0x086F_72B0, 0x400);
/// The emblems (0x08023F68, EXE6 `sub_802812C`'s counterpart): a ROM's
/// seven pictures (four tiles each: MegaMan's, then its own team's six
/// navis'), the eight palettes, and which palette each of the thirteen
/// navis shows (0x08023FC4), by version.
struct Emblems {
    pictures: u32,
    palettes: u32,
    palette_of: u32,
}
const PROTOMAN_EMBLEMS: Emblems = Emblems { pictures: 0x0874_22B8, palettes: 0x086F_AE2C, palette_of: 0x0802_3FC4 };
const COLONEL_EMBLEMS: Emblems = Emblems { pictures: 0x0874_35BC, palettes: 0x086F_C0E8, palette_of: 0x0802_3FC8 };
/// The navis that operate (NaviStats +0x29): MegaMan (0), Team ProtoMan's
/// six (1 to 6) and Team Colonel's (7 to 12).
const NAVI_COUNT: u8 = 13;
const TEAM_NAVIS: u8 = 6;
const EMBLEM_PALETTE_COUNT: u32 = 8;
/// The Program Advance animation's names' colors (EXE6 `byte_802BA48`'s
/// counterpart: three sets of four).
const ADVANCE_NAME_COLORS: (u32, u32) = (0x0802_7DC8, 3);

fn block(rom: &Rom, (a, len): (u32, usize)) -> Tiles {
    tiles(rom, a, len)
}

pub(crate) fn picture(rom: &Rom, (gfx, pal): (u32, u32)) -> Picture {
    Picture { tiles: tiles(rom, gfx, PICTURE_BYTES), palette: palette(rom, pal) }
}

/// A patch list: six bytes a patch (x, y, width, height, palette, mode),
/// ending with 0xFF.
fn patches(rom: &Rom, (mut a, first_tile): (u32, u16)) -> PatchList {
    let mut patches = Vec::new();
    while rom.u8(a) != 0xFF {
        let b = rom.bytes(a, 6);
        patches.push(MapPatch { x: b[0], y: b[1], width: b[2], height: b[3], palette: b[4], by_column: b[5] == 1 });
        a += 6;
    }
    PatchList { first_tile, patches }
}

/// The navis' emblems: navi n's picture is its ROM's nth (Team Colonel's
/// routine takes 6 off a team navi's number, 0x08023F9C: a ROM has
/// MegaMan's and its own team's six) and its palette the one the table
/// gives n, the same table and palettes in both ROMs. The pack has each
/// navi's own, from the ROM that has it, under the navi's key (`names`): a
/// navi the content doesn't have yet has none.
fn emblems(roms: &Roms, names: &AssetNames) -> Vec<Emblem> {
    let (protoman, colonel) = (&roms.protoman, roms.us(Version::Colonel));
    let palettes = |rom: &Rom, e: &Emblems| (0..EMBLEM_PALETTE_COUNT).map(|i| palette(rom, e.palettes + 0x20 * i)).collect::<Vec<_>>();
    let colors = palettes(protoman, &PROTOMAN_EMBLEMS);
    let palette_of = protoman.bytes(PROTOMAN_EMBLEMS.palette_of, NAVI_COUNT as usize);
    assert_eq!(
        (&colors, palette_of),
        (&palettes(colonel, &COLONEL_EMBLEMS), colonel.bytes(COLONEL_EMBLEMS.palette_of, NAVI_COUNT as usize)),
        "the versions' emblems' colors"
    );
    assert_eq!(tiles(protoman, PROTOMAN_EMBLEMS.pictures, 0x80), tiles(colonel, COLONEL_EMBLEMS.pictures, 0x80), "MegaMan's emblem in each version");
    (0..NAVI_COUNT)
        .filter_map(|n| {
            let (rom, e, picture) = if n <= TEAM_NAVIS { (protoman, &PROTOMAN_EMBLEMS, n) } else { (colonel, &COLONEL_EMBLEMS, n - TEAM_NAVIS) };
            Some(Emblem {
                navi: names.navis.get(&n)?.clone(),
                tiles: tiles(rom, e.pictures + 0x80 * picture as u32, 0x80),
                palette: colors[palette_of[n as usize] as usize],
            })
        })
        .collect()
}

/// A button two slots wide in the slots' row (twelve tiles a state, the
/// two slots' icons and codes), with its picture in the chip window.
fn row_button(rom: &Rom, buttons: (u32, usize), details: (u32, u32)) -> ButtonPictures {
    let picture = picture(rom, details);
    ButtonPictures {
        width: 2,
        height: 3,
        tiles: block(rom, buttons),
        sets: ButtonSets::Each,
        cursor: ROW_BUTTON_CURSOR,
        palettes: vec![picture.palette],
        picture,
        ..ButtonPictures::default()
    }
}

/// The custom screen's graphics; `names` gives the navis' emblems their
/// navis' keys, `chip_art` is the chips' pictures (graphics.rs).
pub fn custom(roms: &Roms, names: &AssetNames, chip_art: Vec<ChipArt>) -> CustomScreen {
    let rom = &roms.protoman;
    let palettes = |(a, n): (u32, usize)| (0..n as u32).map(|i| palette(rom, a + 32 * i)).collect::<Vec<_>>();
    let glyphs = |(a, n): (u32, usize)| tiles(rom, a, 0x40 * n);
    let soul = ButtonPictures {
        width: 3,
        height: 2,
        tiles: block(rom, SOUL_BUTTONS),
        // (0x08024540: gray when unavailable or picked.)
        sets: ButtonSets::Other,
        cursor: CursorPlace { corners: cursor_corners(rom, CURSOR_CORNERS.1), ..SOUL_CURSOR },
        picture: picture(rom, (SOUL_PICTURE, SOUL_PALETTES.0)),
        palettes: (0..SOUL_PALETTES.1).map(|i| palette(rom, SOUL_PALETTES.0 + 0x20 * i)).collect(),
        icons: block(rom, SOUL_ICONS),
        icon_palette: palette(rom, SOUL_ICON_PALETTE),
        icon_palettes: {
            let colonel = palette(roms.us(Version::Colonel), COLONEL_SOUL_ICON_PALETTE);
            if colonel == palette(rom, SOUL_ICON_PALETTE) { Vec::new() } else { vec![(Version::Colonel.name().into(), colonel)] }
        },
        ..ButtonPictures::default()
    };
    // SearchSoul's Shuffle (EXE6's re-deal button's counterpart, 0x080245C8:
    // its uses left in the chip window, 0x080245F2) and ColonelSoul's Arm
    // Change (0x0802415A and 0x080245A0, in EXE6's scrap button's place:
    // picked, it looks on offer, the chip it holds drawn over it).
    let redeal = ButtonPictures { uses_digit: true, ..row_button(rom, REDEAL_BUTTONS, REDEAL) };
    let arm_change =
        ButtonPictures { sets: ButtonSets::Unavailable, held_at: Some(ARM_CHANGE_HELD_AT), ..row_button(rom, SCRAP_BUTTONS, SCRAP) };
    let map = |a: u32| -> Vec<MapEntry> { (0..MAP_CELLS).map(|i| MapEntry::from_gba(rom.u16(a + 2 * i))).collect() };
    CustomScreen {
        layout: CustomLayout { ok_cursor: CursorPlace { corners: cursor_corners(rom, CURSOR_CORNERS.0), ..LAYOUT.ok_cursor }, ..LAYOUT },
        buttons: vec![(SOUL_BUTTON.into(), soul), (REDEAL_BUTTON.into(), redeal), (ARM_CHANGE_BUTTON.into(), arm_change)],
        window_tiles: block(rom, WINDOW_TILES),
        column_cells: block(rom, COLUMN_CELLS),
        turn_limit: block(rom, TURN_LIMIT),
        name_bar: block(rom, NAME_BAR),
        window_maps: vec![map(WINDOW_MAP)],
        window_patches: patches(rom, WINDOW_PATCHES),
        cross_maps: Vec::new(),
        cross_patches: PatchList::default(),
        frame_palettes: palettes(FRAME_PALETTES),
        icon_palette: palette(rom, ICON_PALETTE),
        gray_palette: palette(rom, GRAY_PALETTE),
        other_palette: palette(rom, OTHER_PALETTE),
        chip_art,
        pictures: SlotPictures {
            ok: picture(rom, OK),
            ok_picked: picture(rom, OK_PICKED),
            other: picture(rom, OTHER),
        },
        codes: glyphs(CODES),
        elements: Tiles { pixels: FAMILY_BYTES.iter().flat_map(|&b| tiles(rom, ELEMENTS + 0x80 * b, 0x80).pixels).collect() },
        element_colors: FAMILY_BYTES.iter().map(|&b| std::array::from_fn(|i| rom.u16(ELEMENT_COLORS + 12 * b + 2 * i as u32) & 0x7FFF)).collect(),
        digits: glyphs(DIGITS),
        slot_codes: glyphs(SLOT_CODES),
        empty_icon: tiles(rom, EMPTY_ICON, 0x80),
        // (No version shows anything of its own here: the base's name is
        // the game's base version's.)
        versioned: Versioned::new(Version::ProtoMan.name(), VersionPictures::default()),
        cursor: block(rom, CURSOR),
        cross_cursor: Tiles::default(),
        cross_cursor_palette: [0; 16],
        emblems: emblems(roms, names),
        regular: block(rom, REGULAR),
        advance_name_colors: (0..ADVANCE_NAME_COLORS.1)
            .map(|i| std::array::from_fn(|k| rom.u16(ADVANCE_NAME_COLORS.0 + 8 * i + 2 * k as u32) & 0x7FFF))
            .collect(),
        languages: Vec::new(),
    }
}
