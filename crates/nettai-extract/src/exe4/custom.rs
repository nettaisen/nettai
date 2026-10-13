//! The custom screen's graphics. EXE4's screen is laid out
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
//!
//! Each block is stated by where the game's own pointer to it is in each of
//! the four ROMs (`Place`, at one address in both versions' code of a
//! release; the addresses in the comments are Red Sun US's), so any ROM
//! gives what it holds. The window and its parts are the same in the four;
//! OK's and SHUFFLE's pictures and the UNITE button's label are each
//! release's words.

use crate::exe4::hud::palette;
use crate::exe4::rom::{Place, Rom, Roms, Source, Version};
use nettai_assets::{
    ButtonPictures, ButtonPlace, ButtonSets, ChipArt, CursorPlace, CustomLayout, CustomScreen, ElementSprite, MapEntry, PatchList, Picture, SlotPictures,
    Tiles, VersionPictures, Versioned, WindowEmblem,
};

/// The window frame's tiles (to tile 1, 62 tiles), the picked column's
/// cells (tile 0x43), "FINAL TURN" (tile 0x70): the HUD's load list.
const WINDOW_TILES: (Place, usize) = (Place::pointer(0x0801_5A48, 0x0801_59A4), 0x7C0);
const COLUMN_CELLS: (Place, usize) = (Place::pointer(0x0801_5A54, 0x0801_59B0), 0x80);
const TURN_LIMIT: (Place, usize) = (Place::pointer(0x0801_5AA8, 0x0801_5A04), 0x1C0);
/// The window's 15x20 map and its patch list, from tile 0x9C (0x0801DC28).
const WINDOW_MAP: Place = Place::pointer(0x0801_DE4C, 0x0801_DD8C);
const WINDOW_PATCHES: (Place, u16) = (Place::pointer(0x0801_DD8C, 0x0801_DCCC), 0x9C);
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
    // (0x0801EE6C's map at 0x0801EE9C: palette 14.)
    turn_limit_palette: 14,
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
    regular: (1, 100),
};
/// The cursor's corners (two frames of four words: y in the low half, x
/// and the flips, 0x1000 h and 0x2000 v, in the high): over OK
/// (0x0801EDA4's table), over the UNITE button (0x0801EE08's), over the
/// SHUFFLE button (0x0801EDD4's).
const OK_CORNERS: Place = Place::pointer(0x0801_EDB0, 0x0801_ECF0);
const UNITE_CORNERS: Place = Place::pointer(0x0801_EE14, 0x0801_ED54);
const ROW_BUTTON_CORNERS: Place = Place::pointer(0x0801_EDE4, 0x0801_ED24);
/// The cursor over the UNITE button (0x08020DD0: column 11, row 17) and
/// over SHUFFLE (0x0801EDD4: slot 8's place, a slot's (2 * column + 1, 3 *
/// row + 13) cells, 0x08020D9A).
const UNITE_CURSOR: (i16, i16) = (11 * 8, 17 * 8);
const ROW_BUTTON_CURSOR: (i16, i16) = (56, 128);
/// The window's palette 9 (0x0801DC28: once, for every chip), palettes 11
/// and 12 (the slots' icons, grayed: the load list's 0x40 bytes, the gray
/// one after the icons') and 14.
const FRAME_PALETTE: Place = Place::pointer(0x0801_DE38, 0x0801_DD78);
const ICON_PALETTE: Place = Place::pointer(0x0801_5A78, 0x0801_59D4);
const OTHER_PALETTE: Place = Place::pointer(0x0801_5A90, 0x0801_59EC);
/// The chip window's pictures for OK (0x0801FDFE: nothing picked, "NO DATA
/// SELECTED"; picked, "CHIP DATA TRANSMISSION") and the UNITE button's and
/// the SHUFFLE button's (0x0801FE44, 0x0801FE78), with their palettes.
/// OK's two and SHUFFLE's have words, each release's own, in one palette
/// (its own too); UNITE's is the same in the four ROMs.
const WORDED_PALETTE: Place = Place::pointer(0x0801_FE40, 0x0801_FD80);
const OK: (Place, Place) = (Place::pointer(0x0801_FE38, 0x0801_FD78), WORDED_PALETTE);
const OK_PICKED: (Place, Place) = (Place::pointer(0x0801_FE3C, 0x0801_FD7C), WORDED_PALETTE);
const UNITE_PICTURE: (Place, Place) = (Place::pointer(0x0801_FE70, 0x0801_FDB0), Place::pointer(0x0801_FE74, 0x0801_FDB4));
const SHUFFLE_PICTURE: (Place, Place) = (Place::pointer(0x0801_FEB8, 0x0801_FDF8), Place::pointer(0x0801_FEC4, 0x0801_FE04));
const PICTURE_BYTES: usize = 0x540;
/// Chip codes (8x16: A..Z, '*', then one no chip has), damage digits (0-9,
/// '?'), the element icons (2x2, 13 of them: a byte past them shows the
/// last, 0x0801FCC4) and their palette (sprite palette 11, the icon a
/// sprite at (24, 80) from the window's left edge: 0x0801EECC).
const CODES: (Place, usize) = (Place::pointer(0x0801_FD3C, 0x0801_FC7C), 28);
const DIGITS: (Place, usize) = (Place::pointer(0x0801_FD4C, 0x0801_FC8C), 11);
const ELEMENTS: Place = Place::pointer(0x0801_FD44, 0x0801_FC84);
const ELEMENT_COUNT: u32 = 13;
const ELEMENT_PALETTE: Place = Place::pointer(0x0801_E0E4, 0x0801_E024);
const ELEMENT_SPRITE: (i16, i16) = (24, 80);
/// The icons are by the chip record's element byte (+7), EXE4's own family
/// numbers, which its rules' families are (`elements.families`): 0 to 15,
/// those past the icons (13 none's, 14 the program advance's, 15 the
/// souls') showing the last, null's.
const FAMILIES: u32 = 16;
/// The slots' codes (16x8: 0x0801F5CC, by code; code 27 is the fill solid
/// 1, 0x08020EDC, and a code past it, the empty slot's, the table's 28th
/// glyph, dark as the empty icon) and the empty slot's icon (0x0801F59C).
const SLOT_CODES: (Place, usize) = (Place::pointer(0x0801_F5E4, 0x0801_F524), 28);
const EMPTY_ICON: Place = Place::pointer(0x0801_F5B4, 0x0801_F4F4);
/// The UNITE button (0x0801FF14: 3x2 at column 11, row 17, in palette 9,
/// by its entries at 0x0801FF54's table): its tiles, on offer and gray (the
/// load list's to tile 0x52: 0x52 on offer, 0x58 gray, when unavailable or
/// a soul is chosen), and the window's fill tile it shows with no button
/// (0x2A, the window frame's). The tiles have the button's label, each
/// release's own ("UNITE" in the US ROMs).
const UNITE_TILES: (Place, usize) = (Place::pointer(0x0801_5A60, 0x0801_59BC), 0x180);
/// The icons of the souls' chips, chip 0x160 (no soul) and 0x161 to 0x16C
/// (their records' +0x20, one after another; every ROM's alike), by the
/// offer: the soul's choice flies its soul's (0x0802084E: chip 0x160 plus
/// the screen's +0x0D) as a sprite in their palette (0x0802086C, sprite
/// palette 11, 0x08020DF4).
const SOUL_ICONS: (Place, usize) = (Place::pointer(0x0801_D48C, 0x0801_D3CC), 13 * 0x80);
const SOUL_ICON_PALETTE: Place = Place::pointer(0x0802_087C, 0x0802_07BC);
const UNITE_PLACE: ButtonPlace = ButtonPlace { x: 11, y: 17, first_tile: 0x52, palette: 9 };
const WINDOW_FILL_TILE: usize = 0x2A;
/// The SHUFFLE button over slots 8 and 9: 4x3 cells at column 7, row 16,
/// in palette 9, numbered on from slot 8's first tile (0x0801DD34: the
/// slots' tiles are from 0xDE, six a slot, 0x0801F680); its twelve tiles a
/// look, by the screen's +0x13 (0x0801F6C2): as it is, shuffling (0x080209BA)
/// and with no shuffle left (0x080209B8). The pack has a button's sets as
/// selectable, unavailable and picked: its looks 0, 2 and 1.
const SHUFFLE_TILES: (Place, usize) = (Place::pointer(0x0801_F718, 0x0801_F658), 0x480);
const SHUFFLE_PLACE: ButtonPlace = ButtonPlace { x: 7, y: 16, first_tile: 0x10E, palette: 9 };
const SHUFFLE_SIZE: (u8, u8) = (4, 3);
const SHUFFLE_SETS: [usize; 3] = [0, 2, 1];
/// The names EXE4's rules give their buttons (as EXE5's: the soul button's
/// `soul`, Shuffle's `redeal`), which the pack has their looks by.
pub(crate) const SOUL_BUTTON: &str = "soul";
pub(crate) const REDEAL_BUTTON: &str = "redeal";
/// The emblem over the picked column (0x08020028): 2x3 cells at column 12,
/// row 0, its tiles at 0x5E (the load list's), its four frames' entries
/// and the steps of its turn (0x08020078: a frame's entries, 1 a step that
/// holds the last, 0 the end).
const EMBLEM_TILES: (Place, usize) = (Place::pointer(0x0801_5A6C, 0x0801_59C8), 0x240);
const EMBLEM_FIRST_TILE: u16 = 0x5E;
const EMBLEM_STEPS: Place = Place::pointer(0x0802_0074, 0x0801_FFB4);
const EMBLEM_SIZE: (u8, u8) = (2, 3);
const EMBLEM_AT: (u8, u8) = (12, 0);
/// Sprites: the cursor's corner (two frames), its palette and the Regular
/// chip's frame's (sprite palette 13, 0x0801DE68's list), the Regular
/// chip's frame (two frames of 4x4: 0x0801EF00).
const CURSOR: (Place, usize) = (Place::pointer(0x0801_DE68, 0x0801_DDA8), 0x40);
const CURSOR_PALETTE: Place = Place::pointer(0x0801_DE80, 0x0801_DDC0);
const REGULAR: (Place, usize) = (Place::pointer(0x0801_EF40, 0x0801_EE80), 0x400);
/// The Program Advance animation's names' colors (0x0801E684's table:
/// three sets of four).
const ADVANCE_NAME_COLORS: (Place, u32) = (Place::pointer(0x0801_E684, 0x0801_E5C4), 3);

/// A block's tiles in a source: placeholders if the source isn't there.
fn block(src: Source, (place, len): (Place, usize)) -> Tiles {
    if !src.is_present() {
        return crate::placeholders::tiles(len / 32);
    }
    crate::exe4::hud::tiles(src.rom, src.at(place), len)
}

/// A picture of the chip window in a source, with its palette.
fn picture(src: Source, (gfx, pal): (Place, Place)) -> Picture {
    if !src.is_present() {
        return crate::placeholders::picture();
    }
    Picture { tiles: crate::exe4::hud::tiles(src.rom, src.at(gfx), PICTURE_BYTES), palette: palette(src.rom, src.at(pal)) }
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
fn window_emblem(src: Source) -> WindowEmblem {
    let rom = src.rom;
    let (w, h) = EMBLEM_SIZE;
    let mut frames: Vec<u32> = Vec::new();
    let mut steps = Vec::new();
    for k in 0.. {
        let p = rom.u32(src.at(EMBLEM_STEPS) + 4 * k);
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
        tiles: block(src, EMBLEM_TILES),
        frames: frames.iter().map(|&p| (0..(w as u32 * h as u32)).map(|i| MapEntry::from_gba(rom.u16(p + 2 * i))).collect()).collect(),
        // (The first step is the open's draw of the first frame: the rest
        // the turn's, from step 1.)
        steps: steps.into_iter().skip(1).collect(),
    }
}

/// The custom screen's graphics; `chip_art` the chips' pictures
/// (graphics.rs). The window and its parts are whichever ROM's is there,
/// the worded pictures and the UNITE button's label a US ROM's
/// (placeholders without one); with no ROM at all, placeholders in the
/// screen's layout.
pub fn custom(roms: &Roms, chip_art: Vec<ChipArt>) -> CustomScreen {
    let (art, words) = (roms.any(), roms.us());
    if !art.is_present() {
        let mut c = crate::placeholders::custom();
        c.layout = LAYOUT;
        let mut soul = crate::placeholders::button(3, 2);
        soul.place = Some(UNITE_PLACE);
        let mut redeal = crate::placeholders::button(SHUFFLE_SIZE.0, SHUFFLE_SIZE.1);
        redeal.place = Some(SHUFFLE_PLACE);
        redeal.uses_digit = true;
        c.buttons = vec![(SOUL_BUTTON.into(), soul), (REDEAL_BUTTON.into(), redeal)];
        c.versioned = Versioned::new(Version::RedSun.name(), VersionPictures::default());
        c.chip_art = chip_art;
        // What EXE4's screen has of its own where the other games' take a
        // navi's: the element icon as a sprite, the cursor's colors and the
        // emblem over the picked column (one frame of it, at rest).
        let (w, h) = EMBLEM_SIZE;
        c.element_sprite = Some(ElementSprite { x: ELEMENT_SPRITE.0, y: ELEMENT_SPRITE.1, palette: crate::placeholders::PALETTE });
        c.elements = crate::placeholders::tiles(4 * FAMILIES as usize);
        c.element_colors = Vec::new();
        c.cursor_palette = Some(crate::placeholders::PALETTE);
        c.window_emblem = Some(WindowEmblem {
            x: EMBLEM_AT.0,
            y: EMBLEM_AT.1,
            width: w,
            height: h,
            first_tile: EMBLEM_FIRST_TILE,
            tiles: crate::placeholders::tiles(EMBLEM_TILES.1 / 0x20),
            frames: vec![(0..(w as u16 * h as u16)).map(|i| MapEntry { tile: EMBLEM_FIRST_TILE + i, palette: 9, ..Default::default() }).collect()],
            steps: Vec::new(),
        });
        return c;
    }
    let (rom, at) = (art.rom, |place: Place| art.at(place));
    let glyphs = |(place, n): (Place, usize)| block(art, (place, 0x40 * n));
    let window_tiles = block(art, WINDOW_TILES);
    // The UNITE button's sets: on offer, gray (both with its label), and
    // with no button the window's fill tile in its six cells.
    let mut unite_tiles = block(words, UNITE_TILES);
    if let Some(fill) = window_tiles.get(WINDOW_FILL_TILE - 1).map(<[u8]>::to_vec) {
        for _ in 0..6 {
            unite_tiles.push(&fill);
        }
    }
    let unite_picture = picture(art, UNITE_PICTURE);
    let soul = ButtonPictures {
        width: 3,
        height: 2,
        tiles: unite_tiles,
        // (Gray when unavailable and when a soul is chosen.)
        sets: ButtonSets::Other,
        hidden: Some(2),
        cursor: CursorPlace { x: UNITE_CURSOR.0, y: UNITE_CURSOR.1, corners: cursor_corners(rom, at(UNITE_CORNERS)) },
        palettes: vec![unite_picture.palette],
        picture: unite_picture,
        place: Some(UNITE_PLACE),
        icons: block(art, SOUL_ICONS),
        icon_palette: palette(rom, at(SOUL_ICON_PALETTE)),
        ..ButtonPictures::default()
    };
    let shuffle_picture = picture(words, SHUFFLE_PICTURE);
    let shuffle_looks = block(art, SHUFFLE_TILES);
    let mut shuffle_tiles = Tiles::default();
    let cells = SHUFFLE_SIZE.0 as usize * SHUFFLE_SIZE.1 as usize;
    for look in SHUFFLE_SETS {
        for k in 0..cells {
            if let Some(tile) = shuffle_looks.get(look * cells + k) {
                shuffle_tiles.push(tile);
            }
        }
    }
    let redeal = ButtonPictures {
        width: SHUFFLE_SIZE.0,
        height: SHUFFLE_SIZE.1,
        tiles: shuffle_tiles,
        sets: ButtonSets::Each,
        cursor: CursorPlace { x: ROW_BUTTON_CURSOR.0, y: ROW_BUTTON_CURSOR.1, corners: cursor_corners(rom, at(ROW_BUTTON_CORNERS)) },
        palettes: vec![shuffle_picture.palette],
        picture: shuffle_picture,
        // 0x0801FEA2: the shuffles left, in the damage's last cell.
        uses_digit: true,
        place: Some(SHUFFLE_PLACE),
        ..ButtonPictures::default()
    };
    // The slots' codes, the last the empty slot's.
    let slot_codes = glyphs(SLOT_CODES);
    let map = (0..MAP_CELLS).map(|i| MapEntry::from_gba(rom.u16(at(WINDOW_MAP) + 2 * i))).collect();
    let (patches, first_tile) = (at(WINDOW_PATCHES.0), WINDOW_PATCHES.1);
    CustomScreen {
        layout: CustomLayout { ok_cursor: CursorPlace { corners: cursor_corners(rom, at(OK_CORNERS)), ..LAYOUT.ok_cursor }, ..LAYOUT },
        buttons: vec![(SOUL_BUTTON.into(), soul), (REDEAL_BUTTON.into(), redeal)],
        window_tiles,
        column_cells: block(art, COLUMN_CELLS),
        turn_limit: block(art, TURN_LIMIT),
        name_bar: Tiles::default(),
        window_maps: vec![map],
        window_patches: crate::decode::patches(rom, (patches, first_tile)),
        form_list_maps: Vec::new(),
        form_list_patches: PatchList::default(),
        frame_palettes: vec![palette(rom, at(FRAME_PALETTE))],
        icon_palette: palette(rom, at(ICON_PALETTE)),
        gray_palette: palette(rom, at(ICON_PALETTE) + 0x20),
        other_palette: palette(rom, at(OTHER_PALETTE)),
        chip_art,
        pictures: SlotPictures {
            ok: picture(words, OK),
            ok_picked: picture(words, OK_PICKED),
            // (EXE4 has no other kind of slot.)
            other: Picture::default(),
        },
        codes: glyphs(CODES),
        elements: Tiles {
            pixels: (0..FAMILIES).flat_map(|b| crate::exe4::hud::tiles(rom, at(ELEMENTS) + 0x80 * b.min(ELEMENT_COUNT - 1), 0x80).pixels).collect(),
        },
        element_colors: Vec::new(),
        digits: glyphs(DIGITS),
        slot_codes,
        empty_icon: block(art, (EMPTY_ICON, 0x80)),
        // (No version shows anything of its own here.)
        versioned: Versioned::new(Version::RedSun.name(), VersionPictures::default()),
        cursor: block(art, CURSOR),
        form_list_cursor: Tiles::default(),
        form_list_cursor_palette: [0; 16],
        emblems: Vec::new(),
        regular: block(art, REGULAR),
        advance_name_colors: (0..ADVANCE_NAME_COLORS.1)
            .map(|i| std::array::from_fn(|k| rom.u16(at(ADVANCE_NAME_COLORS.0) + 8 * i + 2 * k as u32) & 0x7FFF))
            .collect(),
        languages: Vec::new(),
        element_sprite: Some(ElementSprite { x: ELEMENT_SPRITE.0, y: ELEMENT_SPRITE.1, palette: palette(rom, at(ELEMENT_PALETTE)) }),
        cursor_palette: Some(palette(rom, at(CURSOR_PALETTE))),
        window_emblem: Some(window_emblem(art)),
    }
}

/// What the Japanese ROMs show in words on the screen (the same routines'
/// literals: 0x0801FDFE's and 0x0801FE78's pictures, the load list's UNITE
/// button): OK's two pictures and SHUFFLE's in Japanese, with their
/// palette, and the UNITE button's tiles ("uni son" in two rows), from a
/// Japanese ROM (placeholders without one). The UNITE picture, the SHUFFLE
/// button's tiles and "FINAL TURN" have no words of a release's own: the
/// pack's. `base` is the pack's own screen (its UNITE button's fill set,
/// the window's, is kept in every language).
pub fn lettering(roms: &Roms, base: &CustomScreen) -> nettai_assets::CustomLettering {
    use nettai_assets::{ButtonLettering, CustomLettering};
    let jp = roms.jp();
    let mut unite = block(jp, UNITE_TILES);
    if let Some((_, own)) = base.buttons.iter().find(|(n, _)| n == SOUL_BUTTON) {
        for k in 12..18 {
            if let Some(t) = own.tiles.get(k) {
                unite.push(t);
            }
        }
    }
    CustomLettering {
        pictures: SlotPictures { ok: picture(jp, OK), ok_picked: picture(jp, OK_PICKED), other: Picture::default() },
        form_names: Vec::new(),
        buttons: vec![
            (SOUL_BUTTON.into(), ButtonLettering { tiles: Some(unite), picture: None }),
            (REDEAL_BUTTON.into(), ButtonLettering { tiles: None, picture: Some(picture(jp, SHUFFLE_PICTURE)) }),
        ],
    }
}
