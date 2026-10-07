//! The custom screen's graphics. Everything is uncompressed in the ROM:
//! the battle's HUD load list (`off_801ECB4`) puts the window's tiles and
//! palettes in place at the battle's start, and the screen copies the rest
//! as it runs (`sub_80284E2` the chip window, `sub_8028250` the slots,
//! `off_802A744` its sprites).

use crate::decode::patches;
use crate::exe6::{Rom, u32at};
use nettai_assets::{
    ButtonPictures, ButtonSets, ChipArt, CursorPlace, CustomLayout, CustomScreen, Emblem, MapEntry,
    Palette, Picture, SlotPictures, Tiles, VersionPictures, Versioned, palettes_from_bytes,
};
use nettai_content::names::AssetNames;

/// The window frame's tiles, loaded at tile 1 (0x87 tiles).
const WINDOW_TILES: (u32, usize) = (0x086E_1D38, 0x10E0);
/// The picked-chip column's cells, at tile 0x89.
const COLUMN_CELLS: (u32, usize) = (0x086E_2E18, 0x80);
/// The late turns' block, at tile 0x8D (`sub_8029D34`).
const TURN_LIMIT: (u32, usize) = (0x086F_2500, 0x1C0);
/// The enemy names' bar, at tile 0x1D6 (`sub_801E4F4`).
const NAME_BAR: (u32, usize) = (0x086E_609C, 0x80);
/// The window's 15x20 maps: without and with the Cross tab (`sub_8026840`),
/// and the patch list laid over them from tile 0x9B.
const WINDOW_MAPS: [u32; 2] = [0x086E_625C, 0x086E_64B4];
const WINDOW_PATCHES: (u32, u16) = (0x0802_7B2C, 0x9B);
/// The Cross window's maps (`sub_8027834`: three opening steps, then one to
/// five Crosses) and its patch list, from tile 0xE1.
const CROSS_MAPS: (u32, u32) = (0x086E_670C, 8);
const CROSS_PATCHES: (u32, u16) = (0x0802_7B4A, 0xE1);
const MAP_CELLS: u32 = 15 * 20;
/// Background palette 9 by the chip window's class (`byte_86E587C`), and
/// palettes 11, 12 and 14 (the HUD load list).
const FRAME_PALETTES: (u32, usize) = (0x086E_587C, 4);
const ICON_PALETTE: u32 = 0x0872_CFF4;
const GRAY_PALETTE: u32 = 0x0872_CFB4;
const OTHER_PALETTE: u32 = 0x086E_58FC;
/// ChipData: 0x2C bytes a chip; +0x24 its picture, +0x28 the picture's
/// palette.
const CHIP_COUNT: u32 = 411;
const PICTURE_BYTES: usize = 0x540;
/// The chip window's other pictures and their palettes (`sub_80286D4`,
/// `sub_802871C`, `sub_8028754`, `sub_80287A4`, `sub_802877C`).
const OK: (u32, u32) = (0x0872_2AF4, 0x0872_5874);
const OK_PICKED: (u32, u32) = (0x0872_25B4, 0x0872_57F4);
const OTHER: (u32, u32) = (0x0872_25B4, 0x0872_57D4);
const REDEAL: (u32, u32) = (0x0872_2AF4, 0x0872_5854);
const SCRAP: (u32, u32) = (0x0873_3E74, 0x0873_43D4);
/// Chip codes (8x16, `dword_86E2E98`), element icons and their colors
/// (`dword_86E3598`, `dword_86E3B18`: six colors each), damage digits
/// (`dword_86E411C`).
const CODES: (u32, usize) = (0x086E_2E98, 28);
const ELEMENTS: (u32, usize) = (0x086E_3598, 11);
const ELEMENT_COLORS: u32 = 0x086E_3B18;
const DIGITS: (u32, usize) = (0x086E_411C, 11);
/// The slots' codes (16x8, `dword_86E591C`), the empty slot's icon, and the
/// re-deal and scrap buttons (`dword_86E441C`, `dword_86E4D9C`; Beast
/// Out's, `byte_86E79CC`, is the version's).
const SLOT_CODES: (u32, usize) = (0x086E_591C, 28);
const EMPTY_ICON: u32 = 0x086E_601C;
const REDEAL_BUTTONS: (u32, usize) = (0x086E_441C, 0x480);
/// DustCross's has a fourth state, pressed (`sub_8028340`: 0x180 bytes a
/// state).
const SCRAP_BUTTONS: (u32, usize) = (0x086E_4D9C, 0x600);
/// Sprites: the cursor's corner (`off_802A744`), the navis' emblems and
/// their palettes by navi (`sub_802812C`), the Regular chip's frame
/// (`sub_802899C`).
const CURSOR: (u32, usize) = (0x086E_55BC, 0x40);
/// The Cross window's cursor (`off_802A744`'s fourth block, to sprite tile
/// 0x392).
const CROSS_CURSOR: (u32, usize) = (0x086E_57FC, 0x80);
/// Sprite palette 14, which the battle loads (`byte_86A5D40`).
const CROSS_CURSOR_PALETTE: u32 = 0x086A_5D40;
const EMBLEM_OF: u32 = 0x0802_819C;
const EMBLEM_PALETTE_OF: u32 = 0x0802_818C;
const LINK_NAVIS: u8 = 12;
/// The link navis whose own emblems the Gregar ROM has (navis 1 to 5,
/// HeatMan to ChargeMan: its version's); the others' are the Falzar ROM's.
const GREGAR_NAVIS: std::ops::RangeInclusive<u8> = 1..=5;

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
const REGULAR: (u32, usize) = (0x086E_1238, 0x400);
/// The Program Advance animation's names' colors (`byte_802BA48`: three
/// sets of four).
const ADVANCE_NAME_COLORS: (u32, u32) = (0x0802_BA48, 3);

/// What a version's own custom screen shows (`VersionPictures`), at its
/// ROM's addresses (the code is at the same places in both US ROMs; the
/// data it points at moved).
struct VersionAddresses {
    beast_out: u32,
    beast_out_palettes: u32,
    beast_buttons: u32,
    /// `sub_802812C`: the seven emblems' pictures, and their palettes.
    emblems: u32,
    emblem_palettes: u32,
    /// `sub_8029D94`'s `dword_86E7DCC`: ten names of 0x240 bytes.
    form_names: u32,
    /// `sub_8029EAC`'s `dword_86E944C`: ten palettes.
    form_name_palettes: u32,
}

const FALZAR: VersionAddresses = VersionAddresses {
    beast_out: 0x0872_3034,
    beast_out_palettes: 0x0872_5814,
    beast_buttons: 0x086E_79CC,
    emblems: 0x086F_5834,
    emblem_palettes: 0x086E_56FC,
    form_names: 0x086E_7DCC,
    form_name_palettes: 0x086E_944C,
};

/// The US Gregar ROM's (`MEGAMAN6_GXXBR5E`), read off the same code's
/// literal pools.
const GREGAR: VersionAddresses = VersionAddresses {
    beast_out: 0x0872_0F70,
    beast_out_palettes: 0x0872_3750,
    beast_buttons: 0x086E_5950,
    emblems: 0x086F_3770,
    emblem_palettes: 0x086E_3680,
    form_names: 0x086E_5D50,
    form_name_palettes: 0x086E_73D0,
};

const BEAST_BUTTON_BYTES: usize = 0x400;
pub(crate) const FORM_NAMES: (usize, usize) = (10, 0x240);
const CROSS_PALETTE_COUNT: u32 = 10;

fn version_pictures(rom: &Rom, a: &VersionAddresses) -> VersionPictures {
    let palettes = |at: u32, n: u32| {
        (0..n)
            .map(|i| palette(rom, at + 32 * i))
            .collect::<Vec<_>>()
    };
    // The Beast Out button, the version's Beast's (`sub_8028250`: 4x2 a
    // set: selectable, unavailable, battle mode 1's, and the hidden
    // slot's), with its picture in the chip window. The picture's palette
    // is the first of two (`sub_802871C` takes the one the special slot's
    // +6 numbers, which EXE5's soul button sets for a Chaos Unison,
    // 0x08024540, and nothing in EXE6 sets): the pack leaves the second
    // out.
    let own = palette(rom, a.beast_out_palettes);
    let beast_out = ButtonPictures {
        width: 4,
        height: 2,
        tiles: tiles(rom, (a.beast_buttons, BEAST_BUTTON_BYTES)),
        sets: ButtonSets::Other,
        hidden: Some(3),
        cursor: SPECIAL_CURSOR,
        picture: Picture {
            tiles: tiles(rom, (a.beast_out, PICTURE_BYTES)),
            palette: own,
        },
        palettes: vec![own],
        ..ButtonPictures::default()
    };
    VersionPictures {
        buttons: vec![(BEAST_OUT_BUTTON.into(), beast_out)],
        form_names: tiles(rom, (a.form_names, FORM_NAMES.0 * FORM_NAMES.1)),
        form_name_palettes: palettes(a.form_name_palettes, CROSS_PALETTE_COUNT),
    }
}

/// A button two slots wide in the slots' row (`sub_8028250`: twelve tiles
/// a state, the two slots' icons and codes), with its picture in the chip
/// window.
fn row_button(rom: &Rom, buttons: (u32, usize), details: (u32, u32)) -> ButtonPictures {
    let picture = picture(rom, details);
    ButtonPictures {
        width: 2,
        height: 3,
        tiles: tiles(rom, buttons),
        sets: ButtonSets::Each,
        cursor: ROW_BUTTON_CURSOR,
        palettes: vec![picture.palette],
        picture,
        ..ButtonPictures::default()
    }
}

/// The navis' emblems (`sub_802812C`): navi n's picture is the ROM's
/// `byte_802819C[n]`th and its palette the `byte_802818C[n]`th, the same
/// tables and palettes in both ROMs. A ROM has pictures for MegaMan, its
/// own version's five link navis and ProtoMan, and shows the other
/// version's five with its own five's pictures (a Falzar console's HeatMan
/// has SpoutMan's, in HeatMan's colors). The pack has each navi's own, from
/// the ROM that has it, under the navi's key (`names`): a navi compat
/// doesn't name has none.
fn emblems(roms: &crate::exe6::Roms, names: &AssetNames) -> Vec<Emblem> {
    (0..LINK_NAVIS)
        .filter_map(|n| {
            let (rom, a) = if GREGAR_NAVIS.contains(&n) {
                (roms.gregar, &GREGAR)
            } else {
                (roms.falzar, &FALZAR)
            };
            if !rom.is_present() {
                return None;
            }
            let picture = rom.u8(EMBLEM_OF + n as u32) as u32;
            let color = rom.u8(EMBLEM_PALETTE_OF + n as u32) as u32;
            Some(Emblem {
                navi: names.navis.get(&n)?.clone(),
                tiles: tiles(rom, (a.emblems + 0x80 * picture, 0x80)),
                palette: palette(rom, a.emblem_palettes + 32 * color),
            })
        })
        .collect()
}

fn tiles(rom: &Rom, (a, len): (u32, usize)) -> Tiles {
    crate::exe6::hud::tiles(rom, a, len)
}

/// A palette as the hardware shows it: bit 15 of a color is ignored
/// (the pictures of the chips the US release cut hold 0xCCCC).
fn palette(rom: &Rom, a: u32) -> Palette {
    if !rom.is_present() {
        return crate::placeholders::PALETTE;
    }
    palettes_from_bytes(rom.bytes(a, 32))[0].map(|c| c & 0x7FFF)
}

pub(crate) fn picture(rom: &Rom, (gfx, pal): (u32, u32)) -> Picture {
    Picture {
        tiles: tiles(rom, (gfx, PICTURE_BYTES)),
        palette: palette(rom, pal),
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
/// chips' keys. A Gregar console's own pictures are the US Gregar ROM's,
/// where they differ; the pictures of the chips the US release cut are the
/// Japanese ROMs' (`jp::CHIP_PICTURES`).
pub fn custom(roms: &crate::exe6::Roms, names: &AssetNames) -> CustomScreen {
    let (rom, gregar) = (&roms.falzar, &roms.gregar);
    let mut versioned = Versioned::new("falzar", version_pictures(rom, &FALZAR));
    let own = version_pictures(gregar, &GREGAR);
    if own != versioned.base || !rom.is_present() || !gregar.is_present() {
        versioned.versions.push(("gregar".into(), own));
    }
    if !rom.is_present() {
        let mut c = crate::placeholders::custom();
        c.layout = LAYOUT;
        c.buttons = vec![
            (REDEAL_BUTTON.into(), crate::placeholders::button(2, 3)),
            (SCRAP_BUTTON.into(), crate::placeholders::button(2, 3)),
        ];
        c.versioned = versioned;
        c.chip_art = (0..CHIP_COUNT)
            .map(|id| chip_art(roms, names, id))
            .collect();
        c.emblems = emblems(roms, names);
        return c;
    }
    let glyphs = |(a, n): (u32, usize)| tiles(rom, (a, 0x40 * n));
    let palettes = |(a, n): (u32, usize)| {
        (0..n as u32)
            .map(|i| palette(rom, a + 32 * i))
            .collect::<Vec<_>>()
    };
    CustomScreen {
        layout: LAYOUT,
        buttons: vec![
            (
                REDEAL_BUTTON.into(),
                row_button(rom, REDEAL_BUTTONS, REDEAL),
            ),
            (SCRAP_BUTTON.into(), row_button(rom, SCRAP_BUTTONS, SCRAP)),
        ],
        window_tiles: tiles(rom, WINDOW_TILES),
        column_cells: tiles(rom, COLUMN_CELLS),
        turn_limit: tiles(rom, TURN_LIMIT),
        name_bar: tiles(rom, NAME_BAR),
        window_maps: WINDOW_MAPS.iter().map(|&a| map(rom, a)).collect(),
        window_patches: patches(rom, WINDOW_PATCHES),
        form_list_maps: (0..CROSS_MAPS.1)
            .map(|i| map(rom, CROSS_MAPS.0 + 2 * MAP_CELLS * i))
            .collect(),
        form_list_patches: patches(rom, CROSS_PATCHES),
        frame_palettes: palettes(FRAME_PALETTES),
        icon_palette: palette(rom, ICON_PALETTE),
        gray_palette: palette(rom, GRAY_PALETTE),
        other_palette: palette(rom, OTHER_PALETTE),
        chip_art: (0..CHIP_COUNT)
            .map(|id| chip_art(roms, names, id))
            .collect(),
        pictures: SlotPictures {
            ok: picture(rom, OK),
            ok_picked: picture(rom, OK_PICKED),
            other: picture(rom, OTHER),
        },
        codes: glyphs(CODES),
        elements: tiles(rom, (ELEMENTS.0, 0x80 * ELEMENTS.1)),
        element_colors: (0..ELEMENTS.1 as u32)
            .map(|e| std::array::from_fn(|i| rom.u16(ELEMENT_COLORS + 12 * e + 2 * i as u32)))
            .collect(),
        digits: glyphs(DIGITS),
        slot_codes: glyphs(SLOT_CODES),
        empty_icon: tiles(rom, (EMPTY_ICON, 0x80)),
        versioned,
        cursor: tiles(rom, CURSOR),
        form_list_cursor: tiles(rom, CROSS_CURSOR),
        form_list_cursor_palette: palette(rom, CROSS_CURSOR_PALETTE),
        emblems: emblems(roms, names),
        regular: tiles(rom, REGULAR),
        advance_name_colors: (0..ADVANCE_NAME_COLORS.1)
            .map(|i| {
                std::array::from_fn(|k| {
                    rom.u16(ADVANCE_NAME_COLORS.0 + 8 * i + 2 * k as u32) & 0x7FFF
                })
            })
            .collect(),
        languages: Vec::new(),
        element_sprite: None,
        cursor_palette: None,
        window_emblem: None,
    }
}

/// Chip `id`'s picture: the US Falzar ROM's, the US Gregar ROM's for the
/// five it has right (`gregar::RIGHT_IN_GREGAR`), a Japanese ROM's for the
/// chips the US release cut (`jp::CHIP_PICTURES`). A version's own chip's
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
    if let Some((rom, gfx, pal, _)) = crate::exe6::jp::chip_picture(roms, id) {
        let tiles = tiles(rom, (gfx, PICTURE_BYTES));
        let palette = pal.map_or([0; 16], |p| palette(rom, p));
        return ChipArt {
            key,
            picture: Picture { tiles, palette },
            version: None,
        };
    }
    // A cut chip with no Japanese source must not reuse the US purple block.
    if crate::exe6::jp::CHIP_PICTURES
        .iter()
        .any(|(chip, _)| *chip == id)
    {
        return ChipArt {
            key,
            ..Default::default()
        };
    }
    let (rom, table) = crate::exe6::gregar::chip_source(roms, id);
    if !rom.is_present() {
        return ChipArt {
            key,
            version: crate::exe6::gregar::chip_version(id).map(String::from),
            ..Default::default()
        };
    }
    let record = table + 0x2C * id;
    let (gfx, pal) = (u32at(rom, record + 0x24), u32at(rom, record + 0x28));
    let picture = if rom_pointer(gfx) && rom_pointer(pal) {
        picture(rom, (gfx, pal))
    } else {
        Picture::default()
    };
    ChipArt {
        key,
        picture,
        version: crate::exe6::gregar::chip_version(id).map(String::from),
    }
}
