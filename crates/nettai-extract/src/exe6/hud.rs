//! HUD graphics for the pack's graphics. Everything here is uncompressed in
//! the ROM; the HUD tasks (`sub_801BF64`) copy it to VRAM as needed.

pub(crate) use crate::decode::{banner_at, tiles};
use crate::exe6::{Rom, u32at};
use nettai_assets::{
    BannerLayout, ChipIcon, DialogueFont, Hud, MapEntry, NaviMugshot, Palette, Tiles,
    palettes_from_bytes,
};
use nettai_content::names::AssetNames;

/// HUD layer tiles 0x1A0..=0x1D1: HP digits and blank, the box border,
/// damage digits, '+' (list `off_801ECB4`).
const HUD_TILES: u32 = 0x086E_1638;
/// Tiles 0x1D2..=0x1D5: the '×' and '2' of a doubled chip.
const TIMES_GLYPH: u32 = 0x086B_A120;
const TWO_GLYPH: u32 = 0x086B_7BA0;
/// Gauge tiles 0x222..=0x23D (`sub_801DED0`).
const GAUGE_TILES: u32 = 0x086E_489C;
/// Background palette 9 (gauge) and 13 (HP box: + 0x20 per color state).
const HUD_PALETTES: u32 = 0x086E_1C78;
/// The initial HP box (6x2) and gauge frame (18x2) maps.
const HP_BOX: u32 = 0x0801_EDFC;
const GAUGE_FRAME: u32 = 0x0801_ED6C;
/// The 8x16 font (glyph k = 0x40 bytes).
const FONT: u32 = 0x086B_7AE0;
pub(crate) const FONT_GLYPHS: u32 = 0xE0;
/// The HUD's text lines (`TextScript86F0374`): a table of 16-bit offsets,
/// then each line's glyphs up to its end mark.
const TEXTS: u32 = 0x086F_0374;
const TEXT_END: u8 = 0xE6;
/// ChipData: 0x2C bytes per chip; +0x20 icon pointer.
const CHIP_COUNT: u32 = 411;
/// The opponent's HP digits by color, and their sprite palette.
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
/// The dialogue font (`byte_86ACD60`: 16x12 glyphs, 0x60 bytes each, a
/// row of eight bytes, the left pixel in each byte's low nibble) and its
/// advances (`byte_8043CA4`): the one-byte glyphs below 0xE4 and the
/// two-byte codes' (E4 00 - E4 E8) after them.
const DIALOGUE_FONT: u32 = 0x086A_CD60;
const DIALOGUE_ADVANCES: u32 = 0x0804_3CA4;
const DIALOGUE_GLYPHS: u32 = 0xE4 + 0xE9;
/// The chatbox (`chatbox_runScript`): the box's eleven tiles (to BG0's
/// tile 0x2E4, which its maps count from) and palette, the text's palette
/// (the arrow's too); the box's maps (`spritePtrArr8045CEC`: eight
/// pointers a kind, the opening steps 0-3 first, 30x8 entries each); the
/// key-wait arrow's three 16x16 frames (`chatbox_804082C`).
const CHATBOX_TILES: u32 = 0x086B_EB20;
const CHATBOX_TILE_COUNT: usize = 11;
const CHATBOX_FIRST_TILE: u16 = 0x2E4;
const CHATBOX_PALETTE: u32 = 0x086B_EC80;
const CHATBOX_TEXT_PALETTE: u32 = 0x086B_7AC0;
const CHATBOX_BOXES: u32 = 0x0804_5CEC;
/// The message box (`E8 00`) and the description box (`E8 06 01`).
const CHATBOX_KINDS: u32 = 2;
const CHATBOX_ARROW: u32 = 0x086A_4740;

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

/// The HUD's text lines at `at` (`TEXTS` in the US ROMs): a line with
/// anything but glyphs in it (a text command) is cut there.
pub(crate) fn texts(rom: &Rom, at: u32) -> Vec<Vec<u16>> {
    let offset = |i: u32| rom.u16(at + 2 * i) as u32;
    (0..offset(0) / 2)
        .map(|i| {
            let line = rom.bytes(at + offset(i), 0x40);
            line.iter()
                .take_while(|&&c| c != TEXT_END && (c as u32) < FONT_GLYPHS)
                .map(|&c| c as u16)
                .collect()
        })
        .collect()
}

pub(crate) fn palette(rom: &Rom, a: u32) -> Palette {
    if !rom.is_present() {
        return crate::placeholders::PALETTE;
    }
    palettes_from_bytes(rom.bytes(a, 32))[0]
}

fn map(rom: &Rom, a: u32, n: u32) -> Vec<MapEntry> {
    (0..n)
        .map(|i| MapEntry::from_gba(rom.u16(a + 2 * i)))
        .collect()
}

fn banner(rom: &Rom, id: u32) -> BannerLayout {
    banner_at(rom, (BANNERS, BANNER_FILLER), id)
}

/// An emotion-window face of a ROM, its mugshot palettes at `palettes`.
fn mugshot(rom: &Rom, palettes: u32, e: u32) -> (Tiles, Palette) {
    if !rom.is_present() {
        return (crate::placeholders::tiles(8), crate::placeholders::PALETTE);
    }
    (
        tiles(rom, u32at(rom, MUGSHOTS + 4 * e), 0x100),
        palette(rom, palettes + 0x20 * e),
    )
}

/// A link navi's face of a ROM's table.
fn navi_mugshot(rom: &Rom, (faces, palettes): (u32, u32), n: u32) -> NaviMugshot {
    NaviMugshot {
        tiles: tiles(rom, faces + 0x100 * n, 0x100),
        palettes: std::array::from_fn(|k| palette(rom, palettes + 0x40 * n + 0x20 * k as u32)),
    }
}

/// The HUD's graphics; `names` gives the chip icons their chips' keys and
/// the font its characters. Gregar's own faces and five chips' icons are
/// the Gregar ROM's (`gregar`).
pub fn hud(roms: &crate::exe6::Roms, names: &AssetNames) -> Hud {
    let (rom, gregar) = (roms.falzar, roms.gregar);
    if !rom.is_present() {
        let mut h = crate::placeholders::hud(names, BANNER_COUNT as usize);
        h.chip_icons = chip_icons(roms, names);
        h.mugshots = (0..MUGSHOT_COUNT)
            .map(|_| (crate::placeholders::tiles(8), crate::placeholders::PALETTE))
            .chain(
                crate::exe6::gregar::OWN_FACES
                    .map(|e| mugshot(gregar, crate::exe6::gregar::MUGSHOT_PALETTES, e)),
            )
            .collect();
        h.navi_mugshots = (0..NAVI_MUGSHOT_COUNT)
            .map(|_| NaviMugshot {
                tiles: crate::placeholders::tiles(8),
                palettes: [crate::placeholders::PALETTE; 2],
            })
            .chain((0..crate::exe6::gregar::OWN_NAVI_FACES).map(|n| {
                navi_mugshot(
                    gregar,
                    (
                        crate::exe6::gregar::NAVI_MUGSHOTS,
                        crate::exe6::gregar::NAVI_MUGSHOT_PALETTES,
                    ),
                    n,
                )
            }))
            .collect();
        return h;
    }
    let mut hud_tiles = tiles(rom, HUD_TILES, 0x640);
    for g in [TIMES_GLYPH, TWO_GLYPH] {
        let t = tiles(rom, g, 0x40);
        hud_tiles.push(t.get(0).unwrap());
        hud_tiles.push(t.get(1).unwrap());
    }
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
        font_chars: names
            .glyphs
            .iter()
            .take(FONT_GLYPHS as usize)
            .cloned()
            .collect(),
        enemy_digits: ENEMY_DIGITS.map(|a| tiles(rom, a, 0x40 * 10)),
        enemy_palette: palette(rom, ENEMY_PALETTE),
        chip_icons: chip_icons(roms, names),
        hidden_icon: tiles(rom, HIDDEN_ICON, 0x80),
        icon_palette: palette(rom, ICON_PALETTE),
        // The Falzar ROM's, then Gregar's own (`gregar::OWN_FACES`).
        mugshots: (0..MUGSHOT_COUNT)
            .map(|e| mugshot(rom, MUGSHOT_PALETTES, e))
            .chain(
                crate::exe6::gregar::OWN_FACES
                    .map(|e| mugshot(gregar, crate::exe6::gregar::MUGSHOT_PALETTES, e)),
            )
            .collect(),
        counts: (0..=10u32)
            .map(|n| tiles(rom, COUNTS + 0x80 * (10 - n), 0x80))
            .collect(),
        count_box: tiles(rom, COUNT_BOX, 0x80),
        mugshot_boxes: Vec::new(),
        // The Falzar ROM's six, then Gregar's own five.
        navi_mugshots: (0..NAVI_MUGSHOT_COUNT)
            .map(|n| navi_mugshot(rom, (NAVI_MUGSHOTS, NAVI_MUGSHOT_PALETTES), n))
            .chain((0..crate::exe6::gregar::OWN_NAVI_FACES).map(|n| {
                navi_mugshot(
                    gregar,
                    (
                        crate::exe6::gregar::NAVI_MUGSHOTS,
                        crate::exe6::gregar::NAVI_MUGSHOT_PALETTES,
                    ),
                    n,
                )
            }))
            .collect(),
        navi_box: tiles(rom, NAVI_BOX, 0x80),
        dialogue_font: dialogue_font(
            rom,
            (DIALOGUE_FONT, DIALOGUE_ADVANCES),
            (&names.glyphs, &names.dialogue_glyphs),
        ),
        pause,
        texts: texts(rom, TEXTS),
        banners: (0..BANNER_COUNT).map(|id| banner(rom, id)).collect(),
        banner_digits,
        banner_palette: palette(rom, BANNER_PALETTE),
        waiting: tiles(rom, WAITING, 0x200),
        waiting_palette: palette(rom, BANNER_PALETTE),
        warning: tiles(rom, WARNING, 0x100),
        warning_palette: palette(rom, WARNING_PALETTE),
        chatbox: chatbox(rom),
        layout: nettai_assets::HudLayout::default(),
        language: String::new(),
        languages: Vec::new(),
    }
}

/// The dialogue font at `font` with its advances at `advances`, its
/// glyphs drawing `chars` (the 8x16 font's characters, then the dialogue
/// font's past them).
pub(crate) fn dialogue_font(
    rom: &Rom,
    (font, advances): (u32, u32),
    (cell, dialogue): (&[String], &[String]),
) -> DialogueFont {
    DialogueFont {
        pixels: rom
            .bytes(font, 0x60 * DIALOGUE_GLYPHS as usize)
            .iter()
            .flat_map(|&b| [b & 15, b >> 4])
            .collect(),
        advances: rom.bytes(advances, DIALOGUE_GLYPHS as usize).to_vec(),
        chars: cell
            .iter()
            .chain(dialogue)
            .take(DIALOGUE_GLYPHS as usize)
            .cloned()
            .collect(),
    }
}

fn chatbox(rom: &Rom) -> nettai_assets::Chatbox {
    let n = nettai_assets::Chatbox::COLUMNS * nettai_assets::Chatbox::ROWS;
    let map = |a: u32| -> Vec<MapEntry> {
        (0..n as u32)
            .map(|i| {
                let e = MapEntry::from_gba(rom.u16(a + 2 * i));
                MapEntry {
                    tile: e.tile.wrapping_sub(CHATBOX_FIRST_TILE),
                    ..e
                }
            })
            .collect()
    };
    nettai_assets::Chatbox {
        tiles: tiles(rom, CHATBOX_TILES, 0x20 * CHATBOX_TILE_COUNT),
        palette: palette(rom, CHATBOX_PALETTE),
        boxes: (0..CHATBOX_KINDS)
            .map(|kind| {
                std::array::from_fn(|step| {
                    map(u32at(rom, CHATBOX_BOXES + 4 * (8 * kind + step as u32)))
                })
            })
            .collect(),
        arrow: tiles(rom, CHATBOX_ARROW, 3 * 0x80),
        text_palette: palette(rom, CHATBOX_TEXT_PALETTE),
    }
}

fn chip_icons(roms: &crate::exe6::Roms, names: &AssetNames) -> Vec<ChipIcon> {
    (0..CHIP_COUNT)
        .map(|id| {
            let (rom, table) = crate::exe6::gregar::chip_source(roms, id);
            let icon = if rom.is_present() {
                let p = u32at(rom, table + 0x2c * id + 0x20);
                if rom.contains(p) {
                    tiles(rom, p, 0x80)
                } else {
                    Tiles::default()
                }
            } else {
                Tiles::default()
            };
            ChipIcon {
                key: names.chip_icon(id as u16),
                tiles: icon,
            }
        })
        .collect()
}
