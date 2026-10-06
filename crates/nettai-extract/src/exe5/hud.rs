//! The HUD's graphics, from the US Team ProtoMan ROM (Team Colonel's own
//! faces from the US Team Colonel ROM): what EXE5's HUD tasks copy to VRAM,
//! by the counterparts of the EXE6 routines the EXE6 module reads them by (the
//! verification workspace's tools/exe5 pairs them). EXE5's HUD is EXE6's but
//! for where its blocks load (its HUD layer's tiles from 0x180, EXE6's
//! 0x1A0; the gauge from 0x202, EXE6's 0x222) and its emotion window: a face
//! brings its box (pictures 0-4 and 11-15), or shows a count beside it (the
//! souls', 5-10), and the faces' palettes run on past the pictures for
//! Chaos Unison's (exe5-map.md §11).

pub(crate) use crate::decode::{banner_at, tiles};
use crate::exe5::rom::{Rom, Roms, Version};
use nettai_assets::{
    Chatbox, ChipIcon, DialogueFont, Hud, MapEntry, NaviMugshot, Palette, Tiles,
    palettes_from_bytes,
};
use nettai_content::names::AssetNames;

/// The battle's HUD load list (EXE6 `off_801ECB4`, at 0x0801B54C):
/// the HUD layer's tiles to tile 0x180 (the HP box, HP and damage digits,
/// '+', "????"), then the 8x16 font's '×' and '2' (tiles 0x1B2, 0x1B4).
const HUD_TILES: u32 = 0x086F_76B0;
const HUD_FIRST_TILE: u16 = 0x180;
const TIMES_GLYPH: u32 = 0x086C_DF68;
const TWO_GLYPH: u32 = 0x086C_BB28;
/// The gauge's tiles (EXE6 `sub_801DED0`'s counterpart, 0x0801A7F4: to
/// tile 0x202).
const GAUGE_TILES: u32 = 0x086F_A2CC;
pub(crate) const GAUGE_BYTES: usize = 0x380;
const GAUGE_FIRST_TILE: u16 = 0x202;
/// Background palettes 9 (the gauge) and 13 (the HP box: + 0x20 per color
/// state).
const HUD_PALETTES: u32 = 0x086F_7CF0;
/// The HP box (6x2) and gauge frame (18x2) maps (EXE6 `sub_801E03E`'s and
/// `sub_801DDF6`'s counterparts).
const HP_BOX: u32 = 0x0801_B688;
const GAUGE_FRAME: u32 = 0x0801_B5F8;
/// The 8x16 font (glyph k = 0x40 bytes; EXE6 `dword_86B7AE0`), drawn with
/// the HP box's first palette.
pub(crate) const FONT: u32 = 0x086C_BA68;
pub(crate) const FONT_GLYPHS: usize = 0xE0;
/// The opponent's HP digits by color (EXE6 `off_801D854`'s tables, at
/// 0x0801A1A4) and their sprite palette (the load list's first).
const ENEMY_DIGITS: [u32; 3] = [0x086F_6B30, 0x086F_6DB0, 0x086F_7030];
const ENEMY_PALETTE: u32 = 0x086C_BA48;
/// The chip icons' palette (sprite palette 4: the HUD's load list's to
/// 0x03003690) and the hidden chip's icon.
const ICON_PALETTE: u32 = 0x0874_AAF8;
const HIDDEN_ICON: u32 = 0x0874_A9B8;
/// The emotion window's faces (0x0801968E): pictures 0-4 at FACES and 11-15
/// at FACES_DARK, 0x180 bytes each (the 32x16 face, then its 16x16 box);
/// 5-10 (the version's souls) at SOUL_FACES, 0x100 bytes each, beside the
/// count box showing 10 - n at COUNTS + 0x80 n. The palettes: picture n's
/// at PALETTES + 0x20 n; a soul's in Chaos Unison n + 11 (+ 0x160).
struct FaceAddresses {
    faces: u32,
    soul_faces: u32,
    faces_dark: u32,
    counts: u32,
    palettes: u32,
}

const PROTOMAN_FACES: FaceAddresses = FaceAddresses {
    faces: 0x0873_FD78,
    soul_faces: 0x0874_04F8,
    faces_dark: 0x0874_1078,
    counts: 0x0874_0AF8,
    palettes: 0x0874_17F8,
};
/// Team Colonel's (the same routine's literals, from 0x080197E8: its
/// pictures from 11 at 0x087412FC + 0x180 n).
const COLONEL_FACES: FaceAddresses = FaceAddresses {
    faces: 0x0874_107C,
    soul_faces: 0x0874_17FC,
    faces_dark: 0x0874_237C,
    counts: 0x0874_1DFC,
    palettes: 0x0874_2AFC,
};
const DARK_PALETTES: u32 = 11;
/// The team navis' faces (EXE6 `sub_801CC34`'s counterpart, 0x08019724:
/// navi n's at + 0x100 (n - 1), its version's six), the box beside them
/// (the same in both versions) and their two palettes each, normal and
/// Full Synchro. Team Colonel's routine (0x0801971C) reads its own six, navi
/// n's at + 0x100 (n - 7).
const NAVI_MUGSHOTS: u32 = 0x0874_1B38;
const NAVI_MUGSHOT_COUNT: u32 = 6;
const NAVI_BOX: u32 = 0x0874_1AB8;
const NAVI_MUGSHOT_PALETTES: u32 = 0x0874_2138;
const COLONEL_NAVI_MUGSHOTS: u32 = 0x0874_2E3C;
const COLONEL_NAVI_MUGSHOT_PALETTES: u32 = 0x0874_343C;
/// The dialogue font (16x12 glyphs of 0x60 bytes, up to the HUD font) and
/// its advances: a word a glyph in EXE5 (a byte in EXE6).
const DIALOGUE_FONT: u32 = 0x086C_14C8;
const DIALOGUE_ADVANCES: u32 = 0x0804_26B4;
/// The dialogue font's glyphs: it runs up to the 8x16 font (in every ROM).
pub(crate) const DIALOGUE_GLYPHS: usize = ((FONT - DIALOGUE_FONT) / 0x60) as usize;
/// "PAUSE" (EXE6 `off_801E188`'s): a 32x16 sprite's eight tiles and an 8x16
/// one's two.
const PAUSE: u32 = 0x086F_B7CC;
/// The HUD's text lines (EXE6 `TextScript86F0374`, at the counterparts' 0x0873ACF0).
const TEXTS: u32 = 0x0873_ACF0;
const TEXT_END: u8 = 0xE6;
/// The banners (EXE6 `pt_801EF84`: 49 records to EXE6's 47, the same layout:
/// a head word, then 20 glyph pointers that repeat the filler once it shows
/// up, then a kind-1 banner's number place), their glyph filler (EXE6
/// `byte_801FDC0`), the banner font's digits (ten glyphs from here; EXE6
/// `dword_86F1DC0`, by `off_801FD64`'s counterpart at 0x0801C698) and the
/// banners' palette (EXE6 `byte_86F2900`).
const BANNERS: u32 = 0x0801_B810;
pub(crate) const BANNER_COUNT: u32 = 49;
const BANNER_FILLER: u32 = 0x0801_C6F4;
const BANNER_DIGITS: u32 = 0x0873_C6B8;
const BANNER_PALETTE: u32 = 0x0873_D1F8;
/// "Cstmzing..." (8x2 tiles), the first of the three transfers before the
/// banners (EXE6 `off_801EF30`'s one, 0x200 bytes with the banners'
/// palette); the other two (0x240 and 0x2C0 bytes) are EXE5's own.
const WAITING: u32 = 0x0873_C938;
/// (The other two are what the screen says in a Team Battle between its
/// rounds and while a side changes its order: "Interval...", "Strat
/// Change...". A netbattle shows neither, and the pack has neither.)
/// The warning marker's two 16x16 frames and its palette (EXE6's
/// `dword_86E55FC` and `byte_86E56FC`, the same data).
const WARNING: u32 = 0x086F_AD2C;
const WARNING_PALETTE: u32 = 0x086F_AE2C;
/// The chatbox (`chatbox_runScript`'s counterpart, 0x0803EF18): the box's
/// twenty tiles (to BG0's tile 0x2EB, which its maps count from) and
/// palette, the text's palette; the box's maps (eight pointers a kind, the
/// opening steps 0-3 first, 30x8 entries each); the key-wait arrow's three
/// 16x16 frames.
const CHATBOX_TILES: u32 = 0x086D_2AA8;
const CHATBOX_TILE_COUNT: usize = 20;
const CHATBOX_FIRST_TILE: u16 = 0x2EB;
const CHATBOX_PALETTE: u32 = 0x086D_2D28;
const CHATBOX_TEXT_PALETTE: u32 = 0x086C_BA48;
const CHATBOX_BOXES: u32 = 0x0804_5864;
const CHATBOX_KINDS: u32 = 2;
const CHATBOX_ARROW: u32 = 0x086B_9028;

/// A palette (the hardware ignores bit 15 of a color; chip 0's placeholder
/// palette has it set, which an image can't hold).
pub(crate) fn palette(rom: &Rom, a: u32) -> Palette {
    if !rom.is_present() {
        return crate::placeholders::PALETTE;
    }
    palettes_from_bytes(rom.bytes(a, 32))[0].map(|c| c & 0x7FFF)
}

fn map(rom: &Rom, a: u32, n: u32) -> Vec<MapEntry> {
    (0..n)
        .map(|i| MapEntry::from_gba(rom.u16(a + 2 * i)))
        .collect()
}

/// What glyph `k` draws: the content's name for it (EXE5's text encoding,
/// compat/text.toml), else its number in brackets.
fn glyph_name(names: &AssetNames, k: usize) -> String {
    names
        .glyphs
        .iter()
        .chain(&names.dialogue_glyphs)
        .nth(k)
        .cloned()
        .unwrap_or_else(|| format!("[{k:03x}]"))
}

/// The HUD's text lines at `at`: a line with anything but glyphs in it (a
/// text command) is cut there.
pub(crate) fn texts(rom: &Rom, at: u32) -> Vec<Vec<u16>> {
    let offset = |i: u32| rom.u16(at + 2 * i) as u32;
    (0..offset(0) / 2)
        .map(|i| {
            let line = rom.bytes(at + offset(i), 0x40);
            line.iter()
                .take_while(|&&c| c != TEXT_END && (c as usize) < FONT_GLYPHS)
                .map(|&c| c as u16)
                .collect()
        })
        .collect()
}

/// An emotion-window face (4x2 tiles) and the box it brings (2x2; none for
/// a soul's, which shows a count).
fn face(rom: &Rom, a: &FaceAddresses, picture: u32) -> (Tiles, Tiles) {
    let (at, boxed) = match picture {
        0..5 => (a.faces + 0x180 * picture, true),
        5..11 => (a.soul_faces + 0x100 * (picture - 5), false),
        _ => (a.faces_dark + 0x180 * (picture - 11), true),
    };
    let face = tiles(rom, at, 0x100);
    (
        face,
        if boxed {
            tiles(rom, at + 0x100, 0x80)
        } else {
            Tiles::default()
        },
    )
}

/// The emotion window's faces, as compat/assets.toml numbers them (its
/// [mugshots]): the face table's sixteen pictures in their own palettes,
/// the version's souls (5-10) in Chaos Unison's (+11), then Team Colonel's
/// own souls likewise. Each with the box it brings (`Hud::mugshot_boxes`).
fn mugshots(roms: &Roms) -> (Vec<(Tiles, Palette)>, Vec<Tiles>) {
    let (mut faces, mut boxes) = (Vec::new(), Vec::new());
    let mut push = |rom: &Rom, a: &FaceAddresses, picture: u32, palette_index: u32| {
        let (t, b) = face(rom, a, picture);
        faces.push((t, palette(rom, a.palettes + 0x20 * palette_index)));
        boxes.push(b);
    };
    let (protoman, colonel) = (roms.us(Version::ProtoMan), roms.us(Version::Colonel));
    for picture in 0..16 {
        push(protoman, &PROTOMAN_FACES, picture, picture);
    }
    for picture in 5..11 {
        push(protoman, &PROTOMAN_FACES, picture, picture + DARK_PALETTES);
    }
    for picture in 5..11 {
        push(colonel, &COLONEL_FACES, picture, picture);
    }
    for picture in 5..11 {
        push(colonel, &COLONEL_FACES, picture, picture + DARK_PALETTES);
    }
    (faces, boxes)
}

/// The dialogue font at `font` with its advances at `advances` (a word a
/// glyph), its glyphs drawing `chars` (the 8x16 font's characters, then the
/// dialogue font's past them; a glyph past them its number in brackets).
pub(crate) fn dialogue_font(
    rom: &Rom,
    (font, advances): (u32, u32),
    (cell, dialogue): (&[String], &[String]),
) -> DialogueFont {
    let name = |k: usize| {
        cell.iter()
            .chain(dialogue)
            .nth(k)
            .cloned()
            .unwrap_or_else(|| format!("[{k:03x}]"))
    };
    DialogueFont {
        pixels: rom
            .bytes(font, 0x60 * DIALOGUE_GLYPHS)
            .iter()
            .flat_map(|&b| [b & 15, b >> 4])
            .collect(),
        advances: (0..DIALOGUE_GLYPHS as u32)
            .map(|i| rom.u32(advances + 4 * i) as u8)
            .collect(),
        chars: (0..DIALOGUE_GLYPHS).map(name).collect(),
    }
}

fn chatbox(rom: &Rom) -> Chatbox {
    let n = Chatbox::COLUMNS * Chatbox::ROWS;
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
    Chatbox {
        tiles: tiles(rom, CHATBOX_TILES, 0x20 * CHATBOX_TILE_COUNT),
        palette: palette(rom, CHATBOX_PALETTE),
        boxes: (0..CHATBOX_KINDS)
            .map(|kind| {
                std::array::from_fn(|step| {
                    map(rom.u32(CHATBOX_BOXES + 4 * (8 * kind + step as u32)))
                })
            })
            .collect(),
        arrow: tiles(rom, CHATBOX_ARROW, 3 * 0x80),
        text_palette: palette(rom, CHATBOX_TEXT_PALETTE),
    }
}

/// The HUD's graphics; `chip_icons` the chips' (graphics.rs), `names` the
/// fonts' characters.
pub fn hud(roms: &Roms, names: &AssetNames, chip_icons: Vec<ChipIcon>) -> Hud {
    let rom = roms.protoman;
    if !rom.is_present() {
        let mut h = crate::placeholders::hud(names, BANNER_COUNT as usize);
        (h.mugshots, h.mugshot_boxes) = mugshots(roms);
        h.chip_icons = chip_icons;
        return h;
    }
    let mut hud_tiles = tiles(rom, HUD_TILES, 0x640);
    for g in [TIMES_GLYPH, TWO_GLYPH] {
        let t = tiles(rom, g, 0x40);
        hud_tiles.push(t.get(0).unwrap());
        hud_tiles.push(t.get(1).unwrap());
    }
    // The banner digits, and the filler as glyph 10 (blank).
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
    let (mugshots, mugshot_boxes) = mugshots(roms);
    // The soul faces' count boxes: 0..=10.
    let counts = (0..=10u32)
        .map(|n| tiles(rom, PROTOMAN_FACES.counts + 0x80 * (10 - n), 0x80))
        .collect();
    Hud {
        tiles: hud_tiles,
        first_tile: HUD_FIRST_TILE,
        gauge_tiles: tiles(rom, GAUGE_TILES, GAUGE_BYTES),
        gauge_first_tile: GAUGE_FIRST_TILE,
        hp_palettes: std::array::from_fn(|i| palette(rom, HUD_PALETTES + 0x20 * i as u32)),
        gauge_palette: palette(rom, HUD_PALETTES),
        hp_box: map(rom, HP_BOX, 12),
        gauge_frame: map(rom, GAUGE_FRAME, 36),
        font: tiles(rom, FONT, 0x40 * FONT_GLYPHS),
        font_chars: (0..FONT_GLYPHS).map(|k| glyph_name(names, k)).collect(),
        enemy_digits: ENEMY_DIGITS.map(|a| tiles(rom, a, 0x40 * 10)),
        enemy_palette: palette(rom, ENEMY_PALETTE),
        chip_icons,
        hidden_icon: tiles(rom, HIDDEN_ICON, 0x80),
        icon_palette: palette(rom, ICON_PALETTE),
        mugshots,
        counts,
        // (EXE5 has no box without a count: its faces bring their own.)
        count_box: Tiles::default(),
        mugshot_boxes,
        // The team navis' by navi number: Team ProtoMan's six, then Team
        // Colonel's.
        navi_mugshots: [
            (rom, NAVI_MUGSHOTS, NAVI_MUGSHOT_PALETTES),
            (
                roms.us(Version::Colonel),
                COLONEL_NAVI_MUGSHOTS,
                COLONEL_NAVI_MUGSHOT_PALETTES,
            ),
        ]
        .into_iter()
        .flat_map(|(rom, faces, palettes)| {
            (0..NAVI_MUGSHOT_COUNT).map(move |n| NaviMugshot {
                tiles: tiles(rom, faces + 0x100 * n, 0x100),
                palettes: std::array::from_fn(|k| {
                    palette(rom, palettes + 0x40 * n + 0x20 * k as u32)
                }),
            })
        })
        .collect(),
        navi_box: tiles(rom, NAVI_BOX, 0x80),
        pause,
        texts: texts(rom, TEXTS),
        banners: (0..BANNER_COUNT)
            .map(|id| banner_at(rom, (BANNERS, BANNER_FILLER), id))
            .collect(),
        banner_digits,
        banner_palette: palette(rom, BANNER_PALETTE),
        waiting: tiles(rom, WAITING, 0x200),
        waiting_palette: palette(rom, BANNER_PALETTE),
        warning: tiles(rom, WARNING, 0x100),
        warning_palette: palette(rom, WARNING_PALETTE),
        dialogue_font: dialogue_font(
            rom,
            (DIALOGUE_FONT, DIALOGUE_ADVANCES),
            (&names.glyphs, &names.dialogue_glyphs),
        ),
        chatbox: chatbox(rom),
        language: String::new(),
        languages: Vec::new(),
    }
}
