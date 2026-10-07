//! The custom screen's graphics, from Red Sun US. EXE4's screen is laid out
//! as EXE5's (the window on the HUD layer's left, the chip window's name,
//! picture, code and damage, two rows of five slots, OK and the UNITE
//! button under it, the picked column) by its own code: the screen's open
//! (0x0801DC28: the window's map 0x0870CC80 and its patch list 0x0801DD90,
//! from tile 0x9C), the HUD's load list (0x08015A0C: the frame, the
//! column's cells, the UNITE button's tiles, the emblem's, "FINAL TURN"),
//! the chip window (0x0801FC40), the slots (0x0801F5EC on) and the
//! screen's sprites (0x0801DE68's list: the cursor's corners, the
//! warning's tiles, sprite palette 13). What EXE4 draws otherwise than EXE5 is said in the pack
//! (`CustomScreen`): its element icon is a sprite in a palette of its own
//! (0x0801EECC); its cursor and the Regular chip's frame have a palette of
//! their own (sprite palette 13: 0x0801EE38's and 0x0801EF00's attributes
//! 0xD360, 0xD362) where EXE5's take the navi's emblem's;
//! the emblem over the picked column is the window's own (0x08020028: an
//! orb drawn by frames on the map, which turns as a chip is picked), no
//! navi's sprite; the UNITE button is drawn at its own place on the map
//! (0x0801FF14), not among the slots' patches; and the window has one
//! frame color for every chip (0x0801DC28 loads it once).

use crate::exe4::hud::{palette, tiles};
use crate::exe4::rom::{Rom, Roms, Version};
use nettai_assets::{
    ButtonPictures, ButtonPlace, ButtonSets, ChipArt, CursorPlace, CustomLayout, CustomScreen, ElementSprite, MapEntry, PatchList, Picture, SlotPictures,
    Tiles, VersionPictures, Versioned, WindowEmblem,
};

/// The window frame's tiles (to tile 1, 62 tiles), the picked column's
/// cells (tile 0x43), "FINAL TURN" (tile 0x70): the HUD's load list.
const WINDOW_TILES: (u32, usize) = (0x0870_8320, 0x7C0);
const COLUMN_CELLS: (u32, usize) = (0x0870_8AE0, 0x80);
const TURN_LIMIT: (u32, usize) = (0x0875_0C80, 0x1C0);
/// The window's 15x20 map and its patch list, from tile 0x9C (0x0801DC28).
const WINDOW_MAP: u32 = 0x0870_CC80;
const WINDOW_PATCHES: (u32, u16) = (0x0801_DD90, 0x9C);
const MAP_CELLS: u32 = 15 * 20;
/// Where the blocks load (the patch list's runs: the name from 0x9C, the
/// picture 0xAC, the code 0xD6, the damage 0xD8, the slots 0xDE, the
/// picked column's icons 0x11A); no element tiles (a sprite) and no enemy
/// names' bar; a hidden slot's fill solid 1 (0x08020EDC, as EXE6's), a
/// blank code or damage cell's solid 7 (0x08020E5C: 0x0801FD08, 0x0801FE2A);
/// the empty icon in palette 9 (the slots' patches' palette, which
/// 0x0801FB00 leaves on an empty slot and sets on a picked one;
/// 0x0801FB6E's on an empty cell of the picked column); the cursor over OK
/// at column 11, row 14 (0x08020DBC: (0x58, 0x70)), its corners
/// `OK_CORNERS`.
const LAYOUT: CustomLayout = CustomLayout {
    column_cells: 0x43,
    turn_limit: 0x70,
    name: 0x9C,
    art: 0xAC,
    code: 0xD6,
    element: 0,
    digits: 0xD8,
    slots: 0xDE,
    column_icons: 0x11A,
    name_bar: 0,
    form_names: 0,
    slot_blank: 1,
    detail_blank: 7,
    empty_palette: Some(9),
    ok_cursor: CursorPlace { x: 0x58, y: 0x70, corners: [[(0, 0, false, false); 4]; 2] },
};
/// The cursor's corners (two frames of four words: y in the low half, x
/// and the flips, 0x1000 h and 0x2000 v, in the high): over OK
/// (0x0801EDA4's table), over the UNITE button (0x0801EE08's), over the
/// SHUFFLE button (0x0801EDD4's).
const OK_CORNERS: u32 = 0x0801_EDB4;
const UNITE_CORNERS: u32 = 0x0801_EE18;
const ROW_BUTTON_CORNERS: u32 = 0x0801_EDE8;
/// The cursor over the UNITE button (0x08020DD0: column 11, row 17) and
/// over SHUFFLE (0x0801EDD4: slot 8's place, a slot's (2 * column + 1, 3 *
/// row + 13) cells, 0x08020D9A).
const UNITE_CURSOR: (i16, i16) = (11 * 8, 17 * 8);
const ROW_BUTTON_CURSOR: (i16, i16) = (56, 128);
/// The window's palette 9 (0x0801DC28: once, for every chip), palettes 11
/// and 12 (the slots' icons, grayed: the load list's 0x40 bytes) and 14.
const FRAME_PALETTE: u32 = 0x0870_C340;
const ICON_PALETTE: u32 = 0x0874_7FF8;
const GRAY_PALETTE: u32 = 0x0874_8018;
const OTHER_PALETTE: u32 = 0x0870_C380;
/// The chip window's pictures for OK (0x0801FDFE: nothing picked, "NO DATA
/// SELECTED"; picked, "CHIP DATA TRANSMISSION") and the UNITE button's and
/// the SHUFFLE button's (0x0801FE44, 0x0801FE78), with their palettes.
const OK: (u32, u32) = (0x0873_CD78, 0x0874_00B8);
const OK_PICKED: (u32, u32) = (0x0873_C838, 0x0874_00B8);
const UNITE_PICTURE: (u32, u32) = (0x0873_D2B8, 0x0874_00D8);
const SHUFFLE_PICTURE: (u32, u32) = (0x0873_D7F8, 0x0874_00B8);
const PICTURE_BYTES: usize = 0x540;
/// Chip codes (8x16: A..Z, '*', then one no chip has), damage digits (0-9,
/// '?'), the element icons (2x2, 13 of them: a byte past them shows the
/// last, 0x0801FCC4) and their palette (sprite palette 11, the icon a
/// sprite at (24, 80) from the window's left edge: 0x0801EECC).
const CODES: (u32, usize) = (0x0870_8B60, 28);
const DIGITS: (u32, usize) = (0x0870_98E0, 11);
const ELEMENTS: u32 = 0x0870_9260;
const ELEMENT_COUNT: u32 = 13;
const ELEMENT_PALETTE: u32 = 0x0870_9DE0;
const ELEMENT_SPRITE: (i16, i16) = (24, 80);
/// The icons are by the chip record's element byte (+7), EXE5's order; the
/// pack has them by the engine's family numbers (`ChipFamily`): for each,
/// EXE4's byte (the program advance's and the special family's, past the
/// icons, show the last, null's).
const FAMILY_BYTES: [u32; 15] = [
    0,  // fire
    1,  // aqua
    2,  // elec
    3,  // wood
    5,  // plus
    6,  // sword
    8,  // cursor
    9,  // summon
    10, // wind
    11, // break
    12, // null
    12, // program advance
    12, // special
    4,  // recovery
    7,  // invisible
];
/// The slots' codes (16x8: 0x0801F5CC, by code, 27 of them; a code past
/// them, the empty slot's, the fill solid 1: 0x08020EDC) and the empty
/// slot's icon (0x0801F59C).
const SLOT_CODES: (u32, usize) = (0x0870_C3C0, 27);
const EMPTY_ICON: u32 = 0x0870_CAC0;
/// The UNITE button (0x0801FF14: 3x2 at column 11, row 17, in palette 9,
/// by its entries at 0x0801FF54's table): its tiles, on offer and gray (the
/// load list's to tile 0x52: 0x52 on offer, 0x58 gray, when unavailable or
/// a soul is chosen), and the window's fill tile it shows with no button
/// (0x2A, the window frame's).
const UNITE_TILES: (u32, usize) = (0x0874_8058, 0x180);
const UNITE_PLACE: ButtonPlace = ButtonPlace { x: 11, y: 17, first_tile: 0x52, palette: 9 };
const WINDOW_FILL_TILE: usize = 0x2A;
/// The SHUFFLE button over slots 8 and 9 (0x0801F6C2: twelve tiles a
/// state, by the screen's +0x13).
const SHUFFLE_TILES: (u32, usize) = (0x0870_9E00, 0x480);
/// The names EXE4's rules give their buttons (as EXE5's: the soul button's
/// `soul`, Shuffle's `redeal`), which the pack has their looks by.
pub(crate) const SOUL_BUTTON: &str = "soul";
pub(crate) const REDEAL_BUTTON: &str = "redeal";
/// The emblem over the picked column (0x08020028): 2x3 cells at column 12,
/// row 0, its tiles at 0x5E (the load list's), its four frames' entries
/// and the steps of its turn (0x08020078: a frame's entries, 1 a step that
/// holds the last, 0 the end).
const EMBLEM_TILES: (u32, usize) = (0x0870_9BA0, 0x240);
const EMBLEM_FIRST_TILE: u16 = 0x5E;
const EMBLEM_STEPS: u32 = 0x0802_0078;
const EMBLEM_SIZE: (u8, u8) = (2, 3);
const EMBLEM_AT: (u8, u8) = (12, 0);
/// Sprites: the cursor's corner (two frames), its palette and the Regular
/// chip's frame's (sprite palette 13, 0x0801DE68's list), the Regular
/// chip's frame (two frames of 4x4: 0x0801EF00).
const CURSOR: (u32, usize) = (0x0870_B020, 0x40);
const CURSOR_PALETTE: u32 = 0x0870_C360;
const REGULAR: (u32, usize) = (0x0870_7820, 0x400);
/// The Program Advance animation's names' colors (0x0801E684's table:
/// three sets of four).
const ADVANCE_NAME_COLORS: (u32, u32) = (0x0801_EB58, 3);

fn block(rom: &Rom, (a, len): (u32, usize)) -> Tiles {
    tiles(rom, a, len)
}

fn picture(rom: &Rom, (gfx, pal): (u32, u32)) -> Picture {
    Picture { tiles: tiles(rom, gfx, PICTURE_BYTES), palette: palette(rom, pal) }
}

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

/// The emblem's frames and steps (0x08020078: a pointer a step, a frame's
/// six entries, 1 a hold, 0 the end; the frames in the order they first
/// show).
fn window_emblem(rom: &Rom) -> WindowEmblem {
    let (w, h) = EMBLEM_SIZE;
    let mut frames: Vec<u32> = Vec::new();
    let mut steps = Vec::new();
    for k in 0.. {
        let p = rom.u32(EMBLEM_STEPS + 4 * k);
        match p {
            0 => break,
            1 => steps.push(None),
            p => {
                let f = frames.iter().position(|&q| q == p).unwrap_or_else(|| {
                    frames.push(p);
                    frames.len() - 1
                });
                steps.push(Some(f as u8));
            }
        }
    }
    WindowEmblem {
        x: EMBLEM_AT.0,
        y: EMBLEM_AT.1,
        width: w,
        height: h,
        first_tile: EMBLEM_FIRST_TILE,
        tiles: block(rom, EMBLEM_TILES),
        frames: frames.iter().map(|&p| (0..(w as u32 * h as u32)).map(|i| MapEntry::from_gba(rom.u16(p + 2 * i))).collect()).collect(),
        // (The first step is the open's draw of the first frame: the rest
        // the turn's, from step 1.)
        steps: steps.into_iter().skip(1).collect(),
    }
}

/// The custom screen's graphics from Red Sun US (placeholders in its
/// layout without it); `chip_art` the chips' pictures (graphics.rs).
pub fn custom(roms: &Roms, chip_art: Vec<ChipArt>) -> CustomScreen {
    let rom = roms.redsun;
    if !rom.is_present() {
        let mut c = crate::placeholders::custom();
        c.layout = LAYOUT;
        let mut soul = crate::placeholders::button(3, 2);
        soul.place = Some(UNITE_PLACE);
        c.buttons = vec![(SOUL_BUTTON.into(), soul), (REDEAL_BUTTON.into(), crate::placeholders::button(2, 3))];
        c.versioned = Versioned::new(Version::RedSun.name(), VersionPictures::default());
        c.chip_art = chip_art;
        return c;
    }
    let glyphs = |(a, n): (u32, usize)| tiles(rom, a, 0x40 * n);
    let window_tiles = block(rom, WINDOW_TILES);
    // The UNITE button's sets: on offer, gray, and with no button the
    // window's fill tile in its six cells.
    let mut unite_tiles = block(rom, UNITE_TILES);
    if let Some(fill) = window_tiles.get(WINDOW_FILL_TILE - 1).map(<[u8]>::to_vec) {
        for _ in 0..6 {
            unite_tiles.push(&fill);
        }
    }
    let unite_picture = picture(rom, UNITE_PICTURE);
    let soul = ButtonPictures {
        width: 3,
        height: 2,
        tiles: unite_tiles,
        // (Gray when unavailable and when a soul is chosen.)
        sets: ButtonSets::Other,
        hidden: Some(2),
        cursor: CursorPlace { x: UNITE_CURSOR.0, y: UNITE_CURSOR.1, corners: cursor_corners(rom, UNITE_CORNERS) },
        palettes: vec![unite_picture.palette],
        picture: unite_picture,
        place: Some(UNITE_PLACE),
        ..ButtonPictures::default()
    };
    let shuffle_picture = picture(rom, SHUFFLE_PICTURE);
    let redeal = ButtonPictures {
        width: 2,
        height: 3,
        tiles: block(rom, SHUFFLE_TILES),
        sets: ButtonSets::Each,
        cursor: CursorPlace { x: ROW_BUTTON_CURSOR.0, y: ROW_BUTTON_CURSOR.1, corners: cursor_corners(rom, ROW_BUTTON_CORNERS) },
        palettes: vec![shuffle_picture.palette],
        picture: shuffle_picture,
        ..ButtonPictures::default()
    };
    // The slots' codes, then the empty slot's: the fill.
    let mut slot_codes = glyphs(SLOT_CODES);
    for _ in 0..2 {
        slot_codes.push(&[LAYOUT.slot_blank; Tiles::TILE]);
    }
    let map = (0..MAP_CELLS).map(|i| MapEntry::from_gba(rom.u16(WINDOW_MAP + 2 * i))).collect();
    let (patches, first_tile) = WINDOW_PATCHES;
    CustomScreen {
        layout: CustomLayout { ok_cursor: CursorPlace { corners: cursor_corners(rom, OK_CORNERS), ..LAYOUT.ok_cursor }, ..LAYOUT },
        buttons: vec![(SOUL_BUTTON.into(), soul), (REDEAL_BUTTON.into(), redeal)],
        window_tiles,
        column_cells: block(rom, COLUMN_CELLS),
        turn_limit: block(rom, TURN_LIMIT),
        name_bar: Tiles::default(),
        window_maps: vec![map],
        window_patches: crate::decode::patches(rom, (patches, first_tile)),
        form_list_maps: Vec::new(),
        form_list_patches: PatchList::default(),
        frame_palettes: vec![palette(rom, FRAME_PALETTE)],
        icon_palette: palette(rom, ICON_PALETTE),
        gray_palette: palette(rom, GRAY_PALETTE),
        other_palette: palette(rom, OTHER_PALETTE),
        chip_art,
        pictures: SlotPictures {
            ok: picture(rom, OK),
            ok_picked: picture(rom, OK_PICKED),
            // (EXE4 has no other kind of slot.)
            other: Picture::default(),
        },
        codes: glyphs(CODES),
        elements: Tiles {
            pixels: FAMILY_BYTES.iter().flat_map(|&b| tiles(rom, ELEMENTS + 0x80 * b.min(ELEMENT_COUNT - 1), 0x80).pixels).collect(),
        },
        element_colors: Vec::new(),
        digits: glyphs(DIGITS),
        slot_codes,
        empty_icon: tiles(rom, EMPTY_ICON, 0x80),
        // (No version shows anything of its own here.)
        versioned: Versioned::new(Version::RedSun.name(), VersionPictures::default()),
        cursor: block(rom, CURSOR),
        form_list_cursor: Tiles::default(),
        form_list_cursor_palette: [0; 16],
        emblems: Vec::new(),
        regular: block(rom, REGULAR),
        advance_name_colors: (0..ADVANCE_NAME_COLORS.1)
            .map(|i| std::array::from_fn(|k| rom.u16(ADVANCE_NAME_COLORS.0 + 8 * i + 2 * k as u32) & 0x7FFF))
            .collect(),
        languages: Vec::new(),
        element_sprite: Some(ElementSprite { x: ELEMENT_SPRITE.0, y: ELEMENT_SPRITE.1, palette: palette(rom, ELEMENT_PALETTE) }),
        cursor_palette: Some(palette(rom, CURSOR_PALETTE)),
        window_emblem: Some(window_emblem(rom)),
    }
}

/// What the Japanese ROMs show in words on the screen, at their own
/// addresses (the same routines' literals: 0x0801FDFE's and 0x0801FE78's
/// pictures, the load list's UNITE button): OK's two pictures and
/// SHUFFLE's in Japanese, with their palette, and the UNITE button's tiles
/// ("uni son" in two rows). The UNITE picture, the SHUFFLE button's tiles
/// and "FINAL TURN" are the US ROMs'.
struct Lettering {
    ok: u32,
    ok_picked: u32,
    shuffle: u32,
    palette: u32,
    unite_tiles: u32,
}
const RED_SUN_JP: Lettering =
    Lettering { ok: 0x0873_AA38, ok_picked: 0x0873_A4F8, shuffle: 0x0873_B4B8, palette: 0x0873_DD78, unite_tiles: 0x0874_5D18 };
const BLUE_MOON_JP: Lettering =
    Lettering { ok: 0x0873_A574, ok_picked: 0x0873_A034, shuffle: 0x0873_AFF4, palette: 0x0873_D8B4, unite_tiles: 0x0874_5854 };

/// The screen's Japanese lettering from a Japanese ROM (Red Sun's first);
/// `base` the pack's own screen (its UNITE button's fill set is kept).
pub fn lettering(roms: &Roms, base: &CustomScreen) -> nettai_assets::CustomLettering {
    use nettai_assets::{ButtonLettering, CustomLettering};
    let source = [(roms.redsun_jp, &RED_SUN_JP), (roms.bluemoon_jp, &BLUE_MOON_JP)].into_iter().find(|(r, _)| r.is_present());
    let Some((rom, a)) = source else {
        // (As many tiles as the pack's own button has.)
        let soul_tiles = base.buttons.iter().find(|(n, _)| n == SOUL_BUTTON).map_or(18, |(_, b)| b.tiles.len());
        return CustomLettering {
            pictures: crate::placeholders::slot_pictures(),
            buttons: vec![
                (SOUL_BUTTON.into(), ButtonLettering { tiles: Some(crate::placeholders::tiles(soul_tiles)), picture: None }),
                (REDEAL_BUTTON.into(), ButtonLettering { tiles: None, picture: Some(crate::placeholders::picture()) }),
            ],
            ..Default::default()
        };
    };
    let mut unite = block(rom, (a.unite_tiles, UNITE_TILES.1));
    // (The fill set is the window's, the same in every language.)
    if let Some((_, own)) = base.buttons.iter().find(|(n, _)| n == SOUL_BUTTON) {
        for k in 12..18 {
            if let Some(t) = own.tiles.get(k) {
                unite.push(t);
            }
        }
    }
    CustomLettering {
        pictures: SlotPictures { ok: picture(rom, (a.ok, a.palette)), ok_picked: picture(rom, (a.ok_picked, a.palette)), other: Picture::default() },
        form_names: Vec::new(),
        buttons: vec![
            (SOUL_BUTTON.into(), ButtonLettering { tiles: Some(unite), picture: None }),
            (REDEAL_BUTTON.into(), ButtonLettering { tiles: None, picture: Some(picture(rom, (a.shuffle, a.palette))) }),
        ],
    }
}
