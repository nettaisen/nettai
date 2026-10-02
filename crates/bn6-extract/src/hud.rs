//! HUD graphics for the pack's graphics. Everything here is uncompressed in
//! the ROM; the HUD tasks (`sub_801BF64`) copy it to VRAM as needed.

use crate::{Rom, u32at};
use bn6_assets::{BannerLayout, ChipIcon, Hud, MapEntry, NaviMugshot, Palette, Tiles, palettes_from_bytes};
use bn6_content::names::AssetNames;

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
const FONT_GLYPHS: u32 = 0xE0;
/// The HUD's text lines (`TextScript86F0374`): a table of 16-bit offsets,
/// then each line's glyphs up to its end mark.
const TEXTS: u32 = 0x086F_0374;
const TEXT_END: u8 = 0xE6;
/// ChipData: 0x2C bytes per chip; +0x20 icon pointer.
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
/// The link navis' mugshots (`sub_801CC34`): six faces of 0x100 bytes,
/// the box beside them, two palettes a face (normal, Full Synchro). (Which
/// face each navi shows, `byte_801CDDC`, is its definition's.)
const NAVI_MUGSHOTS: u32 = 0x0872_D094;
const NAVI_MUGSHOT_COUNT: u32 = 6;
const NAVI_BOX: u32 = 0x0872_D014;
const NAVI_MUGSHOT_PALETTES: u32 = 0x0872_D694;
/// "PAUSE" (`off_801E188`): a 32x16 sprite's eight tiles and an 8x16
/// one's two.
const PAUSE: u32 = 0x086E_611C;
/// Banner descriptors (`pt_801EF84`: every banner up to the navis' win
/// and deletion banners), their glyph filler, digits and palette.
const BANNERS: u32 = 0x0801_EF84;
const BANNER_COUNT: u32 = 47;
const BANNER_FILLER: u32 = 0x0801_FDC0;
const BANNER_DIGITS: u32 = 0x086F_1DC0;
const BANNER_PALETTE: u32 = 0x086F_2900;
/// "Cstmzing..." (16 tiles, drawn on the HUD layer in palette 10 =
/// the banner palette).
const WAITING: u32 = 0x086F_2040;
/// The warning marker's two 16x16 frames (`dword_86E55FC`, which the gauge
/// chips' controller, VDoll's curse and LifeSync copy to sprite tiles
/// 0x3CA..=0x3D1) and its palette (`byte_86E56FC`, sprite palette 13).
const WARNING: u32 = 0x086E_55FC;
const WARNING_PALETTE: u32 = 0x086E_56FC;

fn tiles(rom: &Rom, a: u32, len: usize) -> Tiles {
    Tiles::from_4bpp(rom.bytes(a, len))
}

/// The HUD's text lines: a line with anything but glyphs in it (a text
/// command) is cut there.
fn texts(rom: &Rom) -> Vec<Vec<u16>> {
    let offset = |i: u32| rom.u16(TEXTS + 2 * i) as u32;
    (0..offset(0) / 2)
        .map(|i| {
            let line = rom.bytes(TEXTS + offset(i), 0x40);
            line.iter().take_while(|&&c| c != TEXT_END && (c as u32) < FONT_GLYPHS).map(|&c| c as u16).collect()
        })
        .collect()
}

fn palette(rom: &Rom, a: u32) -> Palette {
    palettes_from_bytes(rom.bytes(a, 32))[0]
}

fn map(rom: &Rom, a: u32, n: u32) -> Vec<MapEntry> {
    (0..n).map(|i| MapEntry::from_gba(rom.u16(a + 2 * i))).collect()
}

fn banner(rom: &Rom, id: u32) -> BannerLayout {
    let p = u32at(rom, BANNERS + 4 * id);
    let head = u32at(rom, p);
    let (x, y, kind) = (head as u8, (head >> 8) as u8, (head >> 16) as u8);
    let mut glyphs = Tiles::default();
    let mut number_at = None;
    // Kind 3 (the telops) has no glyphs; kind 4 (the judge's) has them
    // like the plain ones.
    if kind <= 2 || kind == 4 {
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

/// The HUD's graphics; `names` gives the chip icons their chips' keys and
/// the font its characters.
pub fn hud(rom: &Rom, names: &AssetNames) -> Hud {
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
    // "PAUSE" as five glyphs: the first four are the 32x16 sprite's
    // columns (its tiles go row by row), the fifth the 8x16 sprite.
    let pause_tiles = tiles(rom, PAUSE, 0x140);
    let mut pause = Tiles::default();
    for (top, bottom) in [(0, 4), (1, 5), (2, 6), (3, 7), (8, 9)] {
        pause.push(pause_tiles.get(top).unwrap());
        pause.push(pause_tiles.get(bottom).unwrap());
    }
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
        font_chars: names.glyphs.iter().take(FONT_GLYPHS as usize).cloned().collect(),
        enemy_digits: ENEMY_DIGITS.map(|a| tiles(rom, a, 0x40 * 10)),
        enemy_palette: palette(rom, ENEMY_PALETTE),
        chip_icons: (0..CHIP_COUNT)
            .map(|id| {
                let p = u32at(rom, chip(id) + 0x20);
                let icon = if (0x0800_0000..0x0A00_0000).contains(&p) { tiles(rom, p, 0x80) } else { Tiles::default() };
                ChipIcon { key: names.chip_icon(id as u16), tiles: icon }
            })
            .collect(),
        hidden_icon: tiles(rom, HIDDEN_ICON, 0x80),
        icon_palette: palette(rom, ICON_PALETTE),
        mugshots: (0..MUGSHOT_COUNT)
            .map(|e| (tiles(rom, u32at(rom, MUGSHOTS + 4 * e), 0x100), palette(rom, MUGSHOT_PALETTES + 0x20 * e)))
            .collect(),
        counts: (0..=10u32).map(|n| tiles(rom, COUNTS + 0x80 * (10 - n), 0x80)).collect(),
        count_box: tiles(rom, COUNT_BOX, 0x80),
        navi_mugshots: (0..NAVI_MUGSHOT_COUNT)
            .map(|n| NaviMugshot {
                tiles: tiles(rom, NAVI_MUGSHOTS + 0x100 * n, 0x100),
                palettes: std::array::from_fn(|k| palette(rom, NAVI_MUGSHOT_PALETTES + 0x40 * n + 0x20 * k as u32)),
            })
            .collect(),
        navi_box: tiles(rom, NAVI_BOX, 0x80),
        pause,
        texts: texts(rom),
        banners: (0..BANNER_COUNT).map(|id| banner(rom, id)).collect(),
        banner_digits,
        banner_palette: palette(rom, BANNER_PALETTE),
        waiting: tiles(rom, WAITING, 0x200),
        waiting_palette: palette(rom, BANNER_PALETTE),
        warning: tiles(rom, WARNING, 0x100),
        warning_palette: palette(rom, WARNING_PALETTE),
    }
}
