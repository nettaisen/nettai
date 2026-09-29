//! HUD graphics for the asset bundle. Everything here is uncompressed in
//! the ROM; the HUD tasks (`sub_801BF64`) copy it to VRAM as needed.

use crate::{Rom, u32at};
use bn6_assets::{BannerLayout, Hud, MapEntry, Palette, Tiles, palettes_from_bytes};

/// HUD layer tiles 0x1A0..=0x1D1: HP digits and blank, the box border,
/// damage digits, '+' (list `off_801ECB4`).
const HUD_TILES: u32 = 0x086E_1638;
/// Tiles 0x1D2..=0x1D5: the '×' and '2' of a doubled chip.
const TIMES_GLYPH: u32 = 0x086B_A120;
const TWO_GLYPH: u32 = 0x086B_7BA0;
/// Gauge tiles 0x222..=0x23D (`sub_801DED0`).
const GAUGE_TILES: u32 = 0x086E_489C;
/// Background palette 9 (gauge) and 13 (HP box: + 0x20 per colour state).
const HUD_PALETTES: u32 = 0x086E_1C78;
/// The initial HP box (6x2) and gauge frame (18x2) maps.
const HP_BOX: u32 = 0x0801_EDFC;
const GAUGE_FRAME: u32 = 0x0801_ED6C;
/// The 8x16 font (glyph k = 0x40 bytes).
const FONT: u32 = 0x086B_7AE0;
const FONT_GLYPHS: u32 = 0xA0;
/// Chip name archives (ids 0..=0xFF, then 0x100..).
const CHIP_NAMES: [u32; 2] = [0x086E_A94C, 0x086E_B354];
/// ChipData: 0x2C bytes per chip; +0x09 flags (bit 1: shows damage),
/// +0x20 icon pointer.
const CHIP_DATA: u32 = 0x0802_1DA8;
const CHIP_COUNT: u32 = 411;
/// The opponent's HP digits by colour, and their sprite palette.
const ENEMY_DIGITS: [u32; 3] = [0x086E_0AB8, 0x086E_0D38, 0x086E_0FB8];
const ENEMY_PALETTE: u32 = 0x086B_7AC0;
const HIDDEN_ICON: u32 = 0x0872_CE94;
const ICON_PALETTE: u32 = 0x0872_CFD4;
/// Mugshot graphics by emotion (`off_801CD08`) and palettes.
const MUGSHOTS: u32 = 0x0801_CD08;
const MUGSHOT_COUNT: u32 = 23;
const MUGSHOT_PALETTES: u32 = 0x0872_F114;
/// Count box with n = 10 - i at + 0x80 * i, and the plain box.
const COUNTS: u32 = 0x0872_E994;
const COUNT_BOX: u32 = 0x0872_D914;
/// Mugshot emotion by transformation (`byte_801E700`).
const FORM_EMOTIONS: u32 = 0x0801_E700;
/// Banner descriptors (`pt_801EF84`), their glyph filler, digits and
/// palette.
const BANNERS: u32 = 0x0801_EF84;
const BANNER_COUNT: u32 = 20;
const BANNER_FILLER: u32 = 0x0801_FDC0;
const BANNER_DIGITS: u32 = 0x086F_1DC0;
const BANNER_PALETTE: u32 = 0x086F_2900;

fn tiles(rom: &Rom, a: u32, len: usize) -> Tiles {
    Tiles::from_4bpp(rom.bytes(a, len))
}

fn palette(rom: &Rom, a: u32) -> Palette {
    palettes_from_bytes(rom.bytes(a, 32))[0]
}

fn map(rom: &Rom, a: u32, n: u32) -> Vec<MapEntry> {
    (0..n).map(|i| MapEntry::from_gba(rom.u16(a + 2 * i))).collect()
}

fn chip_name(rom: &Rom, id: u32) -> Vec<u8> {
    let archive = CHIP_NAMES[(id >> 8) as usize];
    let mut a = archive + rom.u16(archive + 2 * (id & 0xFF)) as u32;
    let mut name = Vec::new();
    while name.len() < 8 {
        let c = rom.u8(a);
        if c == 0xE6 {
            break;
        }
        name.push(c);
        a += 1;
    }
    name
}

fn banner(rom: &Rom, id: u32) -> BannerLayout {
    let p = u32at(rom, BANNERS + 4 * id);
    let head = u32at(rom, p);
    let (x, y, kind) = (head as u8, (head >> 8) as u8, (head >> 16) as u8);
    let mut glyphs = Tiles::default();
    let mut number_at = None;
    if kind <= 2 {
        // 20 glyph pointers; once the filler shows up it repeats.
        let mut q = p + 4;
        for _ in 0..20 {
            let g = u32at(rom, q);
            let t = tiles(rom, g, 0x40);
            glyphs.push(t.get(0).unwrap());
            glyphs.push(t.get(1).unwrap());
            if g != BANNER_FILLER {
                q += 4;
            }
        }
        if u32at(rom, q) == BANNER_FILLER {
            q += 4;
        }
        if kind == 1 {
            let n = u32at(rom, q);
            number_at = Some((n as u8, (n >> 8) as u8));
        }
    }
    BannerLayout { x, y, kind, glyphs, number_at }
}

pub fn hud(rom: &Rom) -> Hud {
    let mut hud_tiles = tiles(rom, HUD_TILES, 0x640);
    for g in [TIMES_GLYPH, TWO_GLYPH] {
        let t = tiles(rom, g, 0x40);
        hud_tiles.push(t.get(0).unwrap());
        hud_tiles.push(t.get(1).unwrap());
    }
    let chip = |id: u32| CHIP_DATA + 0x2C * id;
    let mut banner_digits = tiles(rom, BANNER_DIGITS, 0x40 * 10);
    let blank = tiles(rom, BANNER_FILLER, 0x40);
    banner_digits.push(blank.get(0).unwrap());
    banner_digits.push(blank.get(1).unwrap());
    Hud {
        tiles: hud_tiles,
        first_tile: 0x1A0,
        gauge_tiles: tiles(rom, GAUGE_TILES, 0x380),
        gauge_first_tile: 0x222,
        hp_palettes: std::array::from_fn(|i| palette(rom, HUD_PALETTES + 0x20 * i as u32)),
        gauge_palette: palette(rom, HUD_PALETTES),
        hp_box: map(rom, HP_BOX, 12),
        gauge_frame: map(rom, GAUGE_FRAME, 36),
        font: tiles(rom, FONT, 0x40 * FONT_GLYPHS as usize),
        chip_names: (0..CHIP_COUNT).map(|id| chip_name(rom, id)).collect(),
        chip_shows_damage: (0..CHIP_COUNT).map(|id| rom.u8(chip(id) + 9) & 2 != 0).collect(),
        enemy_digits: ENEMY_DIGITS.map(|a| tiles(rom, a, 0x40 * 10)),
        enemy_palette: palette(rom, ENEMY_PALETTE),
        chip_icons: (0..CHIP_COUNT)
            .map(|id| {
                let p = u32at(rom, chip(id) + 0x20);
                if (0x0800_0000..0x0A00_0000).contains(&p) { tiles(rom, p, 0x80) } else { Tiles::default() }
            })
            .collect(),
        hidden_icon: tiles(rom, HIDDEN_ICON, 0x80),
        icon_palette: palette(rom, ICON_PALETTE),
        mugshots: (0..MUGSHOT_COUNT)
            .map(|e| (tiles(rom, u32at(rom, MUGSHOTS + 4 * e), 0x100), palette(rom, MUGSHOT_PALETTES + 0x20 * e)))
            .collect(),
        counts: (0..=10u32).map(|n| tiles(rom, COUNTS + 0x80 * (10 - n), 0x80)).collect(),
        count_box: tiles(rom, COUNT_BOX, 0x80),
        form_emotions: rom.bytes(FORM_EMOTIONS, 25).to_vec(),
        banners: (0..BANNER_COUNT).map(|id| banner(rom, id)).collect(),
        banner_digits,
        banner_palette: palette(rom, BANNER_PALETTE),
    }
}
