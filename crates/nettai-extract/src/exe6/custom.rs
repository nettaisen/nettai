//! The custom screen's graphics. Everything is uncompressed in the ROM:
//! the battle's HUD load list (`off_801ECB4`) puts the window's tiles and
//! palettes in place at the battle's start, and the screen copies the rest
//! as it runs (`sub_80284E2` the chip window, `sub_8028250` the slots,
//! `off_802A744` its sprites).
//!
//! Each block is stated by where the game's own pointer to it is (`Place`,
//! as the HUD's are), so any of the four ROMs gives what it holds: the
//! window and its parts are the same in the four, the chip window's worded
//! pictures each release's own, and a version's own pictures its ROMs'.

use crate::decode::patches;
use crate::exe6::hud::CHIPS;
use crate::exe6::{Place, Rom, Roms, Source, Version, u32at};
use nettai_assets::{
    ButtonPictures, ButtonSets, ChipArt, CursorPlace, CustomLayout, CustomScreen, Emblem, MapEntry,
    Palette, Picture, SlotPictures, Tiles, VersionPictures, Versioned, palettes_from_bytes,
};
use nettai_content::names::AssetNames;

/// The window frame's tiles, loaded at tile 1 (0x87 tiles; the battle's
/// HUD load list's, `off_801ECB4`, as the three blocks after it are).
const WINDOW_TILES: (Place, usize) = (Place::pointer(0x0801_ECF0, 0x0801_F114), 0x10E0);
/// The picked-chip column's cells, at tile 0x89.
const COLUMN_CELLS: (Place, usize) = (Place::pointer(0x0801_ECFC, 0x0801_F120), 0x80);
/// The late turns' block, at tile 0x8D (`sub_8029D34`).
const TURN_LIMIT: (Place, usize) = (Place::pointer(0x0801_ED38, 0x0801_F15C), 0x1C0);
/// The enemy names' bar, at tile 0x1D6 (`sub_801E4F4`).
const NAME_BAR: (Place, usize) = (Place::pointer(0x0801_ED50, 0x0801_F174), 0x80);
/// The window's 15x20 maps: without and with the Cross tab (`sub_8026840`),
/// and the patch list laid over them from tile 0x9B.
const WINDOW_MAPS: [Place; 2] = [Place::pointer(0x0802_69A0, 0x0802_6DB0), Place::pointer(0x0802_69A4, 0x0802_6DB4)];
const WINDOW_PATCHES: (Place, u16) = (Place::pointer(0x0802_69AC, 0x0802_6DBC), 0x9B);
/// The Cross window's maps (`sub_8027834`: three opening steps, then one to
/// five Crosses) and its patch list, from tile 0xE1.
const CROSS_MAPS: (Place, u32) = (Place::pointer(0x0802_78E0, 0x0802_7CF4), 8);
const CROSS_PATCHES: (Place, u16) = (Place::pointer(0x0802_7904, 0x0802_7D18), 0xE1);
const MAP_CELLS: u32 = 15 * 20;
/// Background palette 9 by the chip window's class (`byte_86E587C`), and
/// palettes 11, 12 and 14 (the HUD load list).
const FRAME_PALETTES: (Place, usize) = (Place::pointer(0x0802_8818, 0x0802_8C2C), 4);
const ICON_PALETTE: Place = Place::pointer(0x0801_ED08, 0x0801_F12C);
const GRAY_PALETTE: Place = Place::pointer(0x0801_ED14, 0x0801_F138);
const OTHER_PALETTE: Place = Place::pointer(0x0801_ED2C, 0x0801_F150);
/// ChipData (`hud::CHIPS`): 0x2C bytes a chip; +0x24 its picture, +0x28
/// the picture's palette.
const CHIP_COUNT: u32 = 411;
const PICTURE_BYTES: usize = 0x540;
/// The chip window's other pictures and their palettes (the literals of
/// `sub_80286D4`, `sub_8028754`, `sub_80287A4`, `sub_802877C`): each
/// release's own words.
pub(crate) const OK: (Place, Place) = (Place::pointer(0x0802_8708, 0x0802_8B1C), Place::pointer(0x0802_8714, 0x0802_8B28));
pub(crate) const OK_PICKED: (Place, Place) = (Place::pointer(0x0802_870C, 0x0802_8B20), Place::pointer(0x0802_8718, 0x0802_8B2C));
pub(crate) const OTHER: (Place, Place) = (Place::pointer(0x0802_8774, 0x0802_8B88), Place::pointer(0x0802_8778, 0x0802_8B8C));
pub(crate) const REDEAL: (Place, Place) = (Place::pointer(0x0802_87EC, 0x0802_8C00), Place::pointer(0x0802_87F8, 0x0802_8C0C));
pub(crate) const SCRAP: (Place, Place) = (Place::pointer(0x0802_879C, 0x0802_8BB0), Place::pointer(0x0802_87A0, 0x0802_8BB4));
/// Chip codes (8x16, `dword_86E2E98`), element icons and their colors
/// (`dword_86E3598`, `dword_86E3B18`: six colors each), damage digits
/// (`dword_86E411C`).
const CODES: (Place, usize) = (Place::pointer(0x0802_8640, 0x0802_8A54), 28);
const ELEMENTS: (Place, usize) = (Place::pointer(0x0802_8648, 0x0802_8A5C), 11);
const ELEMENT_COLORS: Place = Place::pointer(0x0802_8680, 0x0802_8A94);
const DIGITS: (Place, usize) = (Place::pointer(0x0802_8650, 0x0802_8A64), 11);
/// The slots' codes (16x8, `dword_86E591C`), the empty slot's icon, and the
/// re-deal and scrap buttons (`dword_86E441C`, `dword_86E4D9C`; Beast
/// Out's, `byte_86E79CC`, is the version's).
const SLOT_CODES: (Place, usize) = (Place::pointer(0x0802_8230, 0x0802_8644), 28);
const EMPTY_ICON: Place = Place::pointer(0x0802_81C0, 0x0802_85D4);
const REDEAL_BUTTONS: (Place, usize) = (Place::pointer(0x0802_835C, 0x0802_8770), 0x480);
/// DustCross's has a fourth state, pressed (`sub_8028340`: 0x180 bytes a
/// state).
const SCRAP_BUTTONS: (Place, usize) = (Place::pointer(0x0802_8360, 0x0802_8774), 0x600);
/// Sprites: the cursor's corner (`off_802A744`: the list is four bytes
/// earlier in the Gregar ROMs), the navis' emblems and their palettes by
/// navi (`sub_802812C`), the Regular chip's frame (`sub_802899C`).
const CURSOR: (Place, usize) = (Place::pointer_in_each([0x0802_A744, 0x0802_A740, 0x0802_AB4C, 0x0802_AB48]), 0x40);
/// The Cross window's cursor (`off_802A744`'s fourth block, to sprite tile
/// 0x392).
const CROSS_CURSOR: (Place, usize) = (Place::pointer_in_each([0x0802_A768, 0x0802_A764, 0x0802_AB70, 0x0802_AB6C]), 0x80);
/// Sprite palette 14, which the battle loads (`byte_86A5D40`, by the load
/// list `off_81271C8`).
const CROSS_CURSOR_PALETTE: Place = Place::pointer_in_each([0x0812_7224, 0x0812_9000, 0x0812_F200, 0x0813_0FC8]);
/// `sub_802812C`: which of its ROM's seven emblems a navi shows, and in
/// which of the palettes (`byte_802819C`, `byte_802818C`); the pictures
/// (`dword_86F5834`) and the palettes (`byte_86E56FC`).
const EMBLEM_OF: Place = Place::pointer(0x0802_8198, 0x0802_85AC);
const EMBLEM_PALETTE_OF: Place = Place::pointer(0x0802_8188, 0x0802_859C);
const EMBLEMS: Place = Place::pointer(0x0802_8180, 0x0802_8594);
const EMBLEM_PALETTES: Place = Place::pointer(0x0802_8184, 0x0802_8598);
const LINK_NAVIS: u8 = 12;
/// The link navis whose own emblems only a version's ROMs have (navis 1
/// to 5, HeatMan to ChargeMan: Gregar's; 6 to 10, SpoutMan to GroundMan:
/// Falzar's); MegaMan's and ProtoMan's are the same in every ROM.
const GREGAR_NAVIS: std::ops::RangeInclusive<u8> = 1..=5;
const FALZAR_NAVIS: std::ops::RangeInclusive<u8> = 6..=10;

/// The names EXE6's rules give their buttons (content/exe6/rules/init.luau:
/// ChpShufl's `redeal`, DustCross's `scrap`, Beast Out's `beast_out`),
/// which the pack has their looks by.
pub(crate) const REDEAL_BUTTON: &str = "redeal";
pub(crate) const SCRAP_BUTTON: &str = "scrap";
const BEAST_OUT_BUTTON: &str = "beast_out";

/// Where the blocks load among the HUD layer's tile numbers
/// (`sub_8026840`, `sub_8028250`, `byte_8029DF8`), and the cursor over OK
/// (`sub_80288D0`, `byte_80288E4`).
const LAYOUT: CustomLayout = CustomLayout {
    column_cells: 0x89,
    turn_limit: 0x8D,
    turn_limit_palette: 9,
    name: 0x9B,
    art: 0xAB,
    code: 0xD5,
    element: 0xD7,
    digits: 0xDB,
    slots: 0xE1,
    column_icons: 0x125,
    name_bar: 0x1D6,
    form_names: 0x139,
    slot_blank: 1,
    detail_blank: 8,
    empty_palette: None,
    ok_cursor: CursorPlace {
        x: 0x58 + 3,
        y: 0x70 - 2,
        corners: [
            [
                (2, 1, false, false),
                (2, 0x16, true, false),
                (0x14, 0x16, true, true),
                (0x14, 1, false, true),
            ],
            [
                (4, 3, false, false),
                (4, 0x14, true, false),
                (0x12, 0x14, true, true),
                (0x12, 3, false, true),
            ],
        ],
    },
    regular: (2, 100),
};
/// The cursor over the special slot's button (Beast Out: `sub_8028904`,
/// `byte_8028918`), and over a button two slots wide in the slots' row (the
/// re-deal and scrap buttons, over slots 8 and 9: `sub_8028820`).
const SPECIAL_CURSOR: CursorPlace = CursorPlace {
    x: 0x58 + 3,
    y: 0x88 - 1,
    corners: [
        [
            (2, 1, false, false),
            (2, 0x16, true, false),
            (0xE, 0x16, true, true),
            (0xE, 1, false, true),
        ],
        [
            (3, 2, false, false),
            (3, 0x15, true, false),
            (0xD, 0x15, true, true),
            (0xD, 2, false, true),
        ],
    ],
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
const REGULAR: (Place, usize) = (Place::pointer(0x0802_89D8, 0x0802_8DEC), 0x400);
/// The Program Advance animation's names' colors (`byte_802BA48`: three
/// sets of four).
const ADVANCE_NAME_COLORS: (Place, u32) = (Place::pointer(0x0802_BA10, 0x0802_BE20), 3);

/// What a version's own custom screen shows (`VersionPictures`): the
/// same code's literals in every ROM, each ROM holding its own version's.
/// The Beast Out button's picture and its palettes (`sub_802871C`'s
/// literals: the BeastOut chip's record's picture) and its states
/// (`sub_8028250`'s `byte_86E79CC`); the forms' names (`sub_8029D94`'s
/// `dword_86E7DCC`: ten names of 0x240 bytes, in the release's words) and
/// their palettes (`sub_8029EAC`'s `dword_86E944C`: ten).
const BEAST_OUT: (Place, Place) = (Place::pointer(0x0802_874C, 0x0802_8B60), Place::pointer(0x0802_8750, 0x0802_8B64));
const BEAST_BUTTONS: (Place, usize) = (Place::pointer(0x0802_83A0, 0x0802_87B4), 0x400);
pub(crate) const FORM_NAMES: (Place, usize) = (Place::pointer(0x0802_9DD8, 0x0802_A1EC), 10 * 0x240);
const FORM_NAME_PALETTES: (Place, u32) = (Place::pointer(0x0802_9EC0, 0x0802_A2D4), 10);

/// A version's own pictures, from a ROM of that version (the Gregar ROMs'
/// Beast Out pictures differ in two pixels, `jp`: the US ROM's where both
/// are given); the forms' names in English are its US ROM's alone.
fn version_pictures(roms: &Roms, version: Version) -> VersionPictures {
    let (art, words) = (roms.of(version), roms.us_of(version));
    // The Beast Out button, the version's Beast's (`sub_8028250`: 4x2 a
    // set: selectable, unavailable, battle mode 1's, and the hidden
    // slot's), with its picture in the chip window. The picture's palette
    // is the first of two (`sub_802871C` takes the one the special slot's
    // +6 numbers, which EXE5's soul button sets for a Chaos Unison,
    // 0x08024540, and nothing in EXE6 sets): the pack leaves the second
    // out.
    let picture = picture(art, BEAST_OUT);
    let beast_out = ButtonPictures {
        width: 4,
        height: 2,
        tiles: tiles(art, BEAST_BUTTONS),
        sets: ButtonSets::Other,
        hidden: Some(3),
        cursor: SPECIAL_CURSOR,
        palettes: vec![picture.palette],
        picture,
        ..ButtonPictures::default()
    };
    VersionPictures {
        buttons: vec![(BEAST_OUT_BUTTON.into(), beast_out)],
        form_names: tiles(words, FORM_NAMES),
        form_name_palettes: palettes(art, FORM_NAME_PALETTES),
    }
}

/// A button two slots wide in the slots' row (`sub_8028250`: twelve tiles
/// a state, the two slots' icons and codes), with its picture in the chip
/// window: the states from `art`, the worded picture from `words`.
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

/// The navis' emblems (`sub_802812C`): navi n's picture is the ROM's
/// `byte_802819C[n]`th and its palette the `byte_802818C[n]`th, the same
/// tables and palettes in every ROM. A ROM has pictures for MegaMan, its
/// own version's five link navis and ProtoMan, and shows the other
/// version's five with its own five's pictures (a Falzar console's HeatMan
/// has SpoutMan's, in HeatMan's colors). The pack has each navi's own, from
/// a ROM that has it, under the navi's key (`names`): a navi compat
/// doesn't name has none.
fn emblems(roms: &Roms, names: &AssetNames) -> Vec<Emblem> {
    (0..LINK_NAVIS)
        .filter_map(|n| {
            let src = if GREGAR_NAVIS.contains(&n) {
                roms.of(Version::Gregar)
            } else if FALZAR_NAVIS.contains(&n) {
                roms.of(Version::Falzar)
            } else {
                roms.any()
            };
            if !src.is_present() {
                return None;
            }
            let rom = src.rom;
            let picture = rom.u8(src.at(EMBLEM_OF) + n as u32) as u32;
            let color = rom.u8(src.at(EMBLEM_PALETTE_OF) + n as u32) as u32;
            Some(Emblem {
                navi: names.navis.get(&n)?.clone(),
                tiles: crate::exe6::hud::tiles(rom, src.at(EMBLEMS) + 0x80 * picture, 0x80),
                palette: palette_at(rom, src.at(EMBLEM_PALETTES) + 32 * color),
            })
        })
        .collect()
}

/// A block's tiles in a source: placeholders if the source isn't there.
pub(crate) fn tiles(src: Source, (place, len): (Place, usize)) -> Tiles {
    if !src.is_present() {
        return crate::placeholders::tiles(len / 32);
    }
    crate::exe6::hud::tiles(src.rom, src.at(place), len)
}

/// A palette as the hardware shows it: bit 15 of a color is ignored
/// (the pictures of the chips the US release cut hold 0xCCCC).
fn palette_at(rom: &Rom, a: u32) -> Palette {
    palettes_from_bytes(rom.bytes(a, 32))[0].map(|c| c & 0x7FFF)
}

/// The first `n` palettes of a block in a source.
fn palettes(src: Source, (place, n): (Place, u32)) -> Vec<Palette> {
    if !src.is_present() {
        return vec![crate::placeholders::PALETTE; n as usize];
    }
    (0..n).map(|i| palette_at(src.rom, src.at(place) + 32 * i)).collect()
}

/// A picture of the chip window in a source, with its palette.
pub(crate) fn picture(src: Source, (gfx, pal): (Place, Place)) -> Picture {
    if !src.is_present() {
        return crate::placeholders::picture();
    }
    picture_at(src.rom, (src.at(gfx), src.at(pal)))
}

fn picture_at(rom: &Rom, (gfx, pal): (u32, u32)) -> Picture {
    Picture {
        tiles: crate::exe6::hud::tiles(rom, gfx, PICTURE_BYTES),
        palette: palette_at(rom, pal),
    }
}

fn map(rom: &Rom, a: u32) -> Vec<MapEntry> {
    (0..MAP_CELLS)
        .map(|i| MapEntry::from_gba(rom.u16(a + 2 * i)))
        .collect()
}

fn rom_pointer(p: u32) -> bool {
    (0x0800_0000..0x0A00_0000).contains(&p)
}

/// The custom screen's graphics; `names` gives the chips' pictures their
/// chips' keys. The window and its parts are whichever ROM's is there
/// (`drawn`), the chip window's worded pictures a US ROM's, each version's
/// own pictures a ROM's of that version (`version_pictures`); a chip's
/// picture is its US ROM's where there is one (`gregar::chip_source`), and
/// a Japanese ROM's for the chips the US ROMs have a placeholder for
/// (`jp::CHIP_PICTURES`). What no ROM given holds is a placeholder.
pub fn custom(roms: &Roms, names: &AssetNames) -> CustomScreen {
    let (art, words) = (roms.any(), roms.us());
    let mut versioned = Versioned::new(Version::Falzar.name(), version_pictures(roms, Version::Falzar));
    versioned
        .versions
        .push((Version::Gregar.name().into(), version_pictures(roms, Version::Gregar)));
    let c = CustomScreen {
        layout: LAYOUT,
        buttons: vec![
            (REDEAL_BUTTON.into(), row_button(art, words, REDEAL_BUTTONS, REDEAL)),
            (SCRAP_BUTTON.into(), row_button(art, words, SCRAP_BUTTONS, SCRAP)),
        ],
        versioned,
        chip_art: (0..CHIP_COUNT)
            .map(|id| chip_art(roms, names, id))
            .collect(),
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

/// `c` with what the four ROMs have alike, read from `src`.
fn drawn(src: Source, c: CustomScreen) -> CustomScreen {
    let (rom, at) = (src.rom, |place: Place| src.at(place));
    let glyphs = |(place, n): (Place, usize)| tiles(src, (place, 0x40 * n));
    CustomScreen {
        window_tiles: tiles(src, WINDOW_TILES),
        column_cells: tiles(src, COLUMN_CELLS),
        turn_limit: tiles(src, TURN_LIMIT),
        name_bar: tiles(src, NAME_BAR),
        window_maps: WINDOW_MAPS.iter().map(|&place| map(rom, at(place))).collect(),
        window_patches: patches(rom, (at(WINDOW_PATCHES.0), WINDOW_PATCHES.1)),
        form_list_maps: (0..CROSS_MAPS.1)
            .map(|i| map(rom, at(CROSS_MAPS.0) + 2 * MAP_CELLS * i))
            .collect(),
        form_list_patches: patches(rom, (at(CROSS_PATCHES.0), CROSS_PATCHES.1)),
        frame_palettes: palettes(src, (FRAME_PALETTES.0, FRAME_PALETTES.1 as u32)),
        icon_palette: palette_at(rom, at(ICON_PALETTE)),
        gray_palette: palette_at(rom, at(GRAY_PALETTE)),
        other_palette: palette_at(rom, at(OTHER_PALETTE)),
        codes: glyphs(CODES),
        elements: tiles(src, (ELEMENTS.0, 0x80 * ELEMENTS.1)),
        element_colors: (0..ELEMENTS.1 as u32)
            .map(|e| std::array::from_fn(|i| rom.u16(at(ELEMENT_COLORS) + 12 * e + 2 * i as u32)))
            .collect(),
        digits: glyphs(DIGITS),
        slot_codes: glyphs(SLOT_CODES),
        empty_icon: tiles(src, (EMPTY_ICON, 0x80)),
        cursor: tiles(src, CURSOR),
        form_list_cursor: tiles(src, CROSS_CURSOR),
        form_list_cursor_palette: palette_at(rom, at(CROSS_CURSOR_PALETTE)),
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

/// Chip `id`'s picture: the Falzar ROMs', the Gregar ROMs' for the five
/// they have right (`gregar::RIGHT_IN_GREGAR`), the US ROM's before the
/// Japanese one's (`gregar::chip_source`); a Japanese ROM's alone for the
/// chips the US ROMs have a placeholder for (`jp::CHIP_PICTURES`). A version's own chip's
/// is marked with its version (`gregar::chip_version`): a console of the
/// other version shows its counterpart's there. A cut chip whose
/// palette no ROM holds (the Gregar and Falzar chips': a Card e+ gift's,
/// kept in the save) gets a black one; its definition gives the palette
/// (`art_palette`).
fn chip_art(roms: &crate::exe6::Roms, names: &AssetNames, id: u32) -> ChipArt {
    let key = names.chip_icon(id as u16);
    // (A cut chip's picture is the Japanese ROMs': each Japanese ROM has its
    // own beast in the Gregar and Falzar chips, the pack each chip's own.
    // What a console shows otherwise there is the verification's to know.)
    if let Some((rom, gfx, pal)) = crate::exe6::jp::chip_picture(roms, id) {
        let tiles = crate::exe6::hud::tiles(rom, gfx, PICTURE_BYTES);
        let palette = pal.map_or([0; 16], |p| palette_at(rom, p));
        return ChipArt {
            key,
            picture: Picture { tiles, palette },
            version: None,
        };
    }
    // A cut chip with no Japanese source must not reuse the US purple block.
    if crate::exe6::jp::CHIP_PICTURES.contains(&id) {
        return ChipArt {
            key,
            ..Default::default()
        };
    }
    let version = crate::exe6::gregar::chip_version(id).map(|v| v.name().to_string());
    let src = crate::exe6::gregar::chip_source(roms, id);
    if !src.is_present() {
        return ChipArt {
            key,
            version,
            ..Default::default()
        };
    }
    let rom = src.rom;
    let record = src.at(CHIPS) + 0x2C * id;
    let (gfx, pal) = (u32at(rom, record + 0x24), u32at(rom, record + 0x28));
    let picture = if rom_pointer(gfx) && rom_pointer(pal) {
        picture_at(rom, (gfx, pal))
    } else {
        Picture::default()
    };
    ChipArt { key, picture, version }
}
