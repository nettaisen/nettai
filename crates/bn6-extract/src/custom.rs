//! The custom screen's graphics. Everything is uncompressed in the ROM:
//! the battle's HUD load list (`off_801ECB4`) puts the window's tiles and
//! palettes in place at the battle's start, and the screen copies the rest
//! as it runs (`sub_80284E2` the chip window, `sub_8028250` the slots,
//! `off_802A744` its sprites).

use crate::{Rom, u32at};
use nettai_assets::{
    ChipArt, CustomScreen, MapEntry, MapPatch, Palette, PatchList, Picture, SlotPictures, Tiles, VersionPictures, Versioned,
    palettes_from_bytes,
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
const GREY_PALETTE: u32 = 0x0872_CFB4;
const OTHER_PALETTE: u32 = 0x086E_58FC;
/// ChipData: 0x2C bytes a chip; +0x24 its picture, +0x28 the picture's
/// palette.
const CHIP_DATA: u32 = 0x0802_1DA8;
const CHIP_COUNT: u32 = 411;
const PICTURE_BYTES: usize = 0x540;
/// The chip window's other pictures and their palettes (`sub_80286D4`,
/// `sub_802871C`, `sub_8028754`, `sub_80287A4`, `sub_802877C`).
const OK: (u32, u32) = (0x0872_2AF4, 0x0872_5874);
const OK_PICKED: (u32, u32) = (0x0872_25B4, 0x0872_57F4);
const OTHER: (u32, u32) = (0x0872_25B4, 0x0872_57D4);
const REDEAL: (u32, u32) = (0x0872_2AF4, 0x0872_5854);
const SCRAP: (u32, u32) = (0x0873_3E74, 0x0873_43D4);
/// Chip codes (8x16, `dword_86E2E98`), element icons and their colours
/// (`dword_86E3598`, `dword_86E3B18`: six colours each), damage digits
/// (`dword_86E411C`).
const CODES: (u32, usize) = (0x086E_2E98, 28);
const ELEMENTS: (u32, usize) = (0x086E_3598, 11);
const ELEMENT_COLOURS: u32 = 0x086E_3B18;
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
const EMBLEM_PALETTES: (u32, usize) = (0x086E_56FC, 7);
const EMBLEM_OF: u32 = 0x0802_819C;
const EMBLEM_PALETTE_OF: u32 = 0x0802_818C;
const LINK_NAVIS: usize = 12;
const REGULAR: (u32, usize) = (0x086E_1238, 0x400);

/// What a version's own custom screen shows (`VersionPictures`), at its
/// ROM's addresses (the code is at the same places in both US ROMs; the
/// data it points at moved).
struct VersionAddresses {
    beast_out: u32,
    beast_out_palettes: u32,
    beast_buttons: u32,
    emblems: u32,
    /// `sub_8029D94`'s `dword_86E7DCC`: ten names of 0x240 bytes.
    cross_names: u32,
    /// `sub_8029EAC`'s `dword_86E944C`: ten palettes.
    cross_palettes: u32,
}

const FALZAR: VersionAddresses = VersionAddresses {
    beast_out: 0x0872_3034,
    beast_out_palettes: 0x0872_5814,
    beast_buttons: 0x086E_79CC,
    emblems: 0x086F_5834,
    cross_names: 0x086E_7DCC,
    cross_palettes: 0x086E_944C,
};

/// The US Gregar ROM's (`MEGAMAN6_GXXBR5E`), read off the same code's
/// literal pools.
const GREGAR: VersionAddresses = VersionAddresses {
    beast_out: 0x0872_0F70,
    beast_out_palettes: 0x0872_3750,
    beast_buttons: 0x086E_5950,
    emblems: 0x086F_3770,
    cross_names: 0x086E_5D50,
    cross_palettes: 0x086E_73D0,
};

const BEAST_OUT_PALETTE_COUNT: u32 = 2;
const BEAST_BUTTON_BYTES: usize = 0x400;
const EMBLEM_COUNT: usize = 7;
const CROSS_NAMES: (usize, usize) = (10, 0x240);
const CROSS_PALETTE_COUNT: u32 = 10;

fn version_pictures(rom: &Rom, a: &VersionAddresses) -> VersionPictures {
    let palettes = |at: u32, n: u32| (0..n).map(|i| palette(rom, at + 32 * i)).collect::<Vec<_>>();
    VersionPictures {
        beast_out: Picture { tiles: tiles(rom, (a.beast_out, PICTURE_BYTES)), palette: palette(rom, a.beast_out_palettes) },
        beast_out_palettes: palettes(a.beast_out_palettes, BEAST_OUT_PALETTE_COUNT),
        beast_buttons: tiles(rom, (a.beast_buttons, BEAST_BUTTON_BYTES)),
        emblems: tiles(rom, (a.emblems, 0x80 * EMBLEM_COUNT)),
        cross_names: tiles(rom, (a.cross_names, CROSS_NAMES.0 * CROSS_NAMES.1)),
        cross_palettes: palettes(a.cross_palettes, CROSS_PALETTE_COUNT),
    }
}

fn tiles(rom: &Rom, (a, len): (u32, usize)) -> Tiles {
    Tiles::from_4bpp(rom.bytes(a, len))
}

/// A palette as the hardware shows it: bit 15 of a colour is ignored
/// (the pictures of the chips the US release cut hold 0xCCCC).
fn palette(rom: &Rom, a: u32) -> Palette {
    palettes_from_bytes(rom.bytes(a, 32))[0].map(|c| c & 0x7FFF)
}

fn picture(rom: &Rom, (gfx, pal): (u32, u32)) -> Picture {
    Picture { tiles: tiles(rom, (gfx, PICTURE_BYTES)), palette: palette(rom, pal) }
}

fn map(rom: &Rom, a: u32) -> Vec<MapEntry> {
    (0..MAP_CELLS).map(|i| MapEntry::from_gba(rom.u16(a + 2 * i))).collect()
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

fn rom_pointer(p: u32) -> bool {
    (0x0800_0000..0x0A00_0000).contains(&p)
}

/// The custom screen's graphics; `names` gives the chips' pictures their
/// chips' keys. With the Gregar ROM, a Gregar console's own pictures too
/// (where they differ).
pub fn custom(rom: &Rom, gregar: Option<&Rom>, names: &AssetNames) -> CustomScreen {
    let mut versioned = Versioned::new(version_pictures(rom, &FALZAR));
    if let Some(g) = gregar {
        let own = version_pictures(g, &GREGAR);
        if own != versioned.base {
            versioned.versions.push(("gregar".into(), own));
        }
    }
    let glyphs = |(a, n): (u32, usize)| tiles(rom, (a, 0x40 * n));
    let palettes = |(a, n): (u32, usize)| (0..n as u32).map(|i| palette(rom, a + 32 * i)).collect::<Vec<_>>();
    CustomScreen {
        window_tiles: tiles(rom, WINDOW_TILES),
        column_cells: tiles(rom, COLUMN_CELLS),
        turn_limit: tiles(rom, TURN_LIMIT),
        name_bar: tiles(rom, NAME_BAR),
        window_maps: WINDOW_MAPS.iter().map(|&a| map(rom, a)).collect(),
        window_patches: patches(rom, WINDOW_PATCHES),
        cross_maps: (0..CROSS_MAPS.1).map(|i| map(rom, CROSS_MAPS.0 + 2 * MAP_CELLS * i)).collect(),
        cross_patches: patches(rom, CROSS_PATCHES),
        frame_palettes: palettes(FRAME_PALETTES),
        icon_palette: palette(rom, ICON_PALETTE),
        grey_palette: palette(rom, GREY_PALETTE),
        other_palette: palette(rom, OTHER_PALETTE),
        chip_art: (0..CHIP_COUNT)
            .map(|id| {
                let record = CHIP_DATA + 0x2C * id;
                let (gfx, pal) = (u32at(rom, record + 0x24), u32at(rom, record + 0x28));
                let picture = if rom_pointer(gfx) && rom_pointer(pal) { picture(rom, (gfx, pal)) } else { Picture::default() };
                ChipArt { key: names.chip_icon(id as u16), picture }
            })
            .collect(),
        pictures: SlotPictures {
            ok: picture(rom, OK),
            ok_picked: picture(rom, OK_PICKED),
            redeal: picture(rom, REDEAL),
            scrap: picture(rom, SCRAP),
            other: picture(rom, OTHER),
        },
        codes: glyphs(CODES),
        elements: tiles(rom, (ELEMENTS.0, 0x80 * ELEMENTS.1)),
        element_colours: (0..ELEMENTS.1 as u32)
            .map(|e| std::array::from_fn(|i| rom.u16(ELEMENT_COLOURS + 12 * e + 2 * i as u32)))
            .collect(),
        digits: glyphs(DIGITS),
        slot_codes: glyphs(SLOT_CODES),
        empty_icon: tiles(rom, (EMPTY_ICON, 0x80)),
        redeal_buttons: tiles(rom, REDEAL_BUTTONS),
        scrap_buttons: tiles(rom, SCRAP_BUTTONS),
        versioned,
        cursor: tiles(rom, CURSOR),
        cross_cursor: tiles(rom, CROSS_CURSOR),
        cross_cursor_palette: palette(rom, CROSS_CURSOR_PALETTE),
        emblem_palettes: palettes(EMBLEM_PALETTES),
        emblem_of: rom.bytes(EMBLEM_OF, LINK_NAVIS).to_vec(),
        emblem_palette_of: rom.bytes(EMBLEM_PALETTE_OF, LINK_NAVIS).to_vec(),
        regular: tiles(rom, REGULAR),
    }
}
