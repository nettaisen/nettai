//! The HUD's graphics: what EXE5's HUD tasks copy to VRAM, by the
//! counterparts of the EXE6 routines the EXE6 module reads them by (the
//! verification workspace's tools/exe5 pairs them). EXE5's HUD is EXE6's but
//! for where its blocks load (its HUD layer's tiles from 0x180, EXE6's
//! 0x1A0; the gauge from 0x202, EXE6's 0x222) and its emotion window: a face
//! brings its box (pictures 0-4 and 11-15), or shows a count beside it (the
//! souls', 5-10), and the faces' palettes run on past the pictures for
//! Chaos Unison's (exe5-map.md §11).
//!
//! Each block is stated by where the game's own pointer to it is in each of
//! the four ROMs (`Place`; the addresses in the comments are the US Team
//! ProtoMan ROM's blocks'), so any ROM gives what it holds: what the four
//! have alike from whichever is there, the English words from a US ROM, a
//! team's own faces from a ROM of that version.

pub(crate) use crate::decode::{banner_at, tiles};
use crate::exe5::rom::{Place, Rom, Roms, Source, Version};
use nettai_assets::{
    Chatbox, ChipIcon, DialogueFont, Hud, MapEntry, NaviMugshot, Palette, Tiles,
    palettes_from_bytes,
};
use nettai_content::names::AssetNames;

/// The battle's HUD load list (EXE6 `off_801ECB4`, at 0x0801B54C):
/// the HUD layer's tiles to tile 0x180 (the HP box, HP and damage digits,
/// '+', "????"; 0x086F76B0), then the 8x16 font's '×' and '2' (tiles
/// 0x1B2, 0x1B4).
const HUD_TILES: Place = Place::pointer_in_each([0x0801_B558, 0x0801_B554, 0x0801_B524, 0x0801_B520]);
const HUD_FIRST_TILE: u16 = 0x180;
const TIMES_GLYPH: Place = Place::pointer_in_each([0x0801_B570, 0x0801_B56C, 0x0801_B53C, 0x0801_B538]);
const TWO_GLYPH: Place = Place::pointer_in_each([0x0801_B57C, 0x0801_B578, 0x0801_B548, 0x0801_B544]);
/// The gauge's tiles (EXE6 `sub_801DED0`'s counterpart, 0x0801A7F4: to
/// tile 0x202; 0x086FA2CC), the same in the four ROMs.
pub(crate) const GAUGE_TILES: Place = Place::pointer_in_each([0x0801_A86C, 0x0801_A864, 0x0801_A838, 0x0801_A830]);
pub(crate) const GAUGE_BYTES: usize = 0x380;
const GAUGE_FIRST_TILE: u16 = 0x202;
/// Background palettes 9 (the gauge) and 13 (the HP box: + 0x20 per color
/// state; 0x086F7CF0).
const HUD_PALETTES: Place = Place::pointer_in_each([0x0801_A7EC, 0x0801_A7E4, 0x0801_A7B8, 0x0801_A7B0]);
/// The HP box (6x2) and gauge frame (18x2) maps (EXE6 `sub_801E03E`'s and
/// `sub_801DDF6`'s counterparts).
const HP_BOX: Place = Place::pointer_in_each([0x0801_A968, 0x0801_A960, 0x0801_A934, 0x0801_A92C]);
const GAUGE_FRAME: Place = Place::pointer_in_each([0x0801_A7E4, 0x0801_A7DC, 0x0801_A7B0, 0x0801_A7A8]);
/// The 8x16 font (glyph k = 0x40 bytes; EXE6 `dword_86B7AE0`; 0x086CBA68),
/// drawn with the HP box's first palette.
pub(crate) const FONT: Place = Place::pointer_in_each([0x0801_9C58, 0x0801_9C50, 0x0801_9C2C, 0x0801_9C24]);
pub(crate) const FONT_GLYPHS: usize = 0xE0;
/// The opponent's HP digits by color (EXE6 `off_801D854`'s tables, at
/// 0x0801A1A4) and their sprite palette (the load list's first).
const ENEMY_DIGITS: [Place; 3] = [
    Place::pointer_in_each([0x0801_A1A4, 0x0801_A19C, 0x0801_A178, 0x0801_A170]),
    Place::pointer_in_each([0x0801_A1D0, 0x0801_A1C8, 0x0801_A1A4, 0x0801_A19C]),
    Place::pointer_in_each([0x0801_A1FC, 0x0801_A1F4, 0x0801_A1D0, 0x0801_A1C8]),
];
const ENEMY_PALETTE: Place = Place::pointer_in_each([0x0801_B54C, 0x0801_B548, 0x0801_B518, 0x0801_B514]);
/// The chip icons' palette (sprite palette 4: the HUD's load list's to
/// 0x03003690) and the hidden chip's icon.
const ICON_PALETTE: Place = Place::pointer_in_each([0x0801_B5B8, 0x0801_B5B4, 0x0801_B584, 0x0801_B580]);
const HIDDEN_ICON: Place = Place::pointer_in_each([0x0801_8C6C, 0x0801_8C64, 0x0801_8C40, 0x0801_8C38]);
/// The emotion window's faces (0x0801968E's literals): pictures 0-4 at
/// FACES and 11-15 at FACES_DARK (which is where an eleventh before them
/// would be: picture n at + 0x180 n), 0x180 bytes each (the 32x16 face,
/// then its 16x16 box); 5-10 (the version's souls) at SOUL_FACES, 0x100
/// bytes each, beside the count box showing 10 - n at COUNTS + 0x80 n. The
/// palettes: picture n's at FACE_PALETTES + 0x20 n; a soul's in Chaos
/// Unison n + 11 (+ 0x160).
///
/// Pictures 0-4 and the counts are the same in the four ROMs. The souls'
/// are each version's own six, with their palettes. Pictures 11-15 are the
/// same and four of their palettes differ by version in two or three
/// colors: the pack has Team ProtoMan's.
const FACES: Place = Place::pointer_in_each([0x0801_97F0, 0x0801_97E8, 0x0801_97C4, 0x0801_97BC]);
const SOUL_FACES: Place = Place::pointer_in_each([0x0801_97F8, 0x0801_97F0, 0x0801_97CC, 0x0801_97C4]);
const FACES_DARK: Place = Place::pointer_in_each([0x0801_97F4, 0x0801_97EC, 0x0801_97C8, 0x0801_97C0]);
const COUNTS: Place = Place::pointer_in_each([0x0801_985C, 0x0801_9854, 0x0801_9830, 0x0801_9828]);
const FACE_PALETTES: Place = Place::pointer_in_each([0x0801_9808, 0x0801_9800, 0x0801_97DC, 0x0801_97D4]);
const DARK_PALETTES: u32 = 11;
/// The team navis' faces (EXE6 `sub_801CC34`'s counterpart, 0x08019724:
/// navi n's at + 0x100 (n - 1), its version's six; Team Colonel's routine
/// reads its own six, navi n's at + 0x100 (n - 7)), the box beside them
/// (the same in both versions) and their two palettes each, normal and
/// Full Synchro.
const NAVI_MUGSHOTS: Place = Place::pointer_in_each([0x0801_9864, 0x0801_985C, 0x0801_9838, 0x0801_9830]);
const NAVI_MUGSHOT_COUNT: u32 = 6;
const NAVI_BOX: Place = Place::pointer_in_each([0x0801_986C, 0x0801_9864, 0x0801_9840, 0x0801_9838]);
const NAVI_MUGSHOT_PALETTES: Place = Place::pointer_in_each([0x0801_9868, 0x0801_9860, 0x0801_983C, 0x0801_9834]);
/// The dialogue font (16x12 glyphs of 0x60 bytes, up to the HUD font;
/// 0x086C14C8) and its advances: a word a glyph in EXE5 (a byte in EXE6;
/// 0x080426B4).
pub(crate) const DIALOGUE_FONT: Place = Place::pointer_in_each([0x0803_AA48, 0x0803_AA4C, 0x0803_A9C4, 0x0803_A9C8]);
pub(crate) const DIALOGUE_ADVANCES: Place = Place::pointer_in_each([0x0803_EBD4, 0x0803_EBD8, 0x0803_EAB8, 0x0803_EABC]);
/// The dialogue font's glyphs: it runs up to the 8x16 font (0xA5A0 bytes
/// in every ROM).
pub(crate) const DIALOGUE_GLYPHS: usize = 0xA5A0 / 0x60;
/// "PAUSE" (EXE6 `off_801E188`'s): a 32x16 sprite's eight tiles and an 8x16
/// one's two.
const PAUSE: Place = Place::pointer_in_each([0x0801_AA54, 0x0801_AA4C, 0x0801_AA20, 0x0801_AA18]);
/// The HUD's text lines (EXE6 `TextScript86F0374`, at the counterparts' 0x0873ACF0).
pub(crate) const TEXTS: Place = Place::pointer_in_each([0x0801_9C4C, 0x0801_9C44, 0x0801_9C20, 0x0801_9C18]);
const TEXT_END: u8 = 0xE6;
/// The banners (EXE6 `pt_801EF84`: 49 records to EXE6's 47, the same layout:
/// a head word, then 20 glyph pointers that repeat the filler once it shows
/// up, then a kind-1 banner's number place), their glyph filler (EXE6
/// `byte_801FDC0`), the banner font's digits (ten glyphs from here; EXE6
/// `dword_86F1DC0`, by `off_801FD64`'s counterpart at 0x0801C698) and the
/// banners' palette (EXE6 `byte_86F2900`).
pub(crate) const BANNERS: Place = Place::pointer_in_each([0x0801_B1C4, 0x0801_B1C0, 0x0801_B190, 0x0801_B18C]);
pub(crate) const BANNER_COUNT: u32 = 49;
/// The banners that aren't plain, as the US ROMs' records have them (the
/// kind, the place, a kind-1 banner's number's place), for a HUD of
/// placeholders (`placeholders::banners`): the turn's and the round's
/// numbered banners, the Program Advance's two that hold, the judge's, and
/// the telops' (the console's own and the other player's, with no glyphs:
/// a frontend draws the chip's name there).
const BANNER_RECORDS: [crate::placeholders::BannerRecord; 10] = [
    (0x0C / 4, 1, (48, 64), Some((104, 64))),
    (0x20 / 4, 1, (68, 64), Some((140, 64))),
    (0x24 / 4, 2, (12, 24), None),
    (0x28 / 4, 4, (40, 16), None),
    (0x30 / 4, 1, (40, 64), Some((112, 64))),
    (0x34 / 4, 2, (132, 24), None),
    (0x3C / 4, 1, (44, 64), Some((108, 64))),
    (0x4C / 4, 3, (0, 32), None),
    (0x50 / 4, 3, (120, 32), None),
    (0x90 / 4, 1, (40, 64), Some((56, 64))),
];
pub(crate) const BANNER_FILLER: Place = Place::pointer_in_each([0x0801_B1F4, 0x0801_B1F0, 0x0801_B1C0, 0x0801_B1BC]);
const BANNER_DIGITS: Place = Place::pointer_in_each([0x0801_C698, 0x0801_C694, 0x0801_C654, 0x0801_C650]);
pub(crate) const BANNER_PALETTE: Place = Place::pointer_in_each([0x0801_B1D4, 0x0801_B1D0, 0x0801_B1A0, 0x0801_B19C]);
/// "Cstmzing..." (8x2 tiles), the first of the three transfers before the
/// banners (EXE6 `off_801EF30`'s one, 0x200 bytes with the banners'
/// palette); the other two (0x240 and 0x2C0 bytes) are EXE5's own. The
/// Japanese ROMs' is seven tiles wide.
pub(crate) const WAITING: Place = Place::pointer_in_each([0x0801_B7BC, 0x0801_B7B8, 0x0801_B788, 0x0801_B784]);
/// (The other two are what the screen says in a Team Battle between its
/// rounds and while a side changes its order: "Interval...", "Strat
/// Change...". A netbattle shows neither, and the pack has neither.)
/// The warning marker's two 16x16 frames and its palette (EXE6's
/// `dword_86E55FC` and `byte_86E56FC`, the same data: the palette is the
/// first of the emblems').
const WARNING: Place = Place::pointer_in_each([0x0802_5D94, 0x0802_5D98, 0x0802_5D50, 0x0802_5D54]);
const WARNING_PALETTE: Place = Place::pointer_in_each([0x0802_3FBC, 0x0802_3FC0, 0x0802_3F78, 0x0802_3F7C]);
/// The chatbox (`chatbox_runScript`'s counterpart, 0x0803EF18): the box's
/// twenty tiles (to BG0's tile 0x2EB, which its maps count from) and
/// palette, the text's palette; the box's maps (eight pointers a kind, the
/// opening steps 0-3 first, 30x8 entries each); the key-wait arrow's three
/// 16x16 frames.
const CHATBOX_TILES: Place = Place::pointer_in_each([0x0803_F000, 0x0803_F004, 0x0803_EEE4, 0x0803_EEE8]);
const CHATBOX_TILE_COUNT: usize = 20;
const CHATBOX_FIRST_TILE: u16 = 0x2EB;
const CHATBOX_PALETTE: Place = Place::pointer_in_each([0x0803_F00C, 0x0803_F010, 0x0803_EEF0, 0x0803_EEF4]);
const CHATBOX_TEXT_PALETTE: Place = Place::pointer_in_each([0x0803_F014, 0x0803_F018, 0x0803_EEF8, 0x0803_EEFC]);
const CHATBOX_BOXES: Place = Place::pointer_in_each([0x0803_EF00, 0x0803_EF04, 0x0803_EDE4, 0x0803_EDE8]);
const CHATBOX_KINDS: u32 = 2;
const CHATBOX_ARROW: Place = Place::pointer_in_each([0x0803_F424, 0x0803_F428, 0x0803_F308, 0x0803_F30C]);

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

/// An emotion-window face of a source (4x2 tiles) and the box it brings
/// (2x2; none for a soul's, which shows a count).
fn face(src: Source, picture: u32) -> (Tiles, Tiles) {
    let boxed = !(5..11).contains(&picture);
    if !src.is_present() {
        let none = crate::placeholders::tiles;
        return (none(8), if boxed { none(4) } else { Tiles::default() });
    }
    let at = match picture {
        0..5 => src.at(FACES) + 0x180 * picture,
        5..11 => src.at(SOUL_FACES) + 0x100 * (picture - 5),
        _ => src.at(FACES_DARK) + 0x180 * picture,
    };
    let face = tiles(src.rom, at, 0x100);
    (
        face,
        if boxed {
            tiles(src.rom, at + 0x100, 0x80)
        } else {
            Tiles::default()
        },
    )
}

/// The emotion window's faces, as compat/assets.toml numbers them (its
/// [mugshots]): the face table's sixteen pictures in their own palettes,
/// the version's souls (5-10) in Chaos Unison's (+11), then Team Colonel's
/// own souls likewise. Each with the box it brings (`Hud::mugshot_boxes`).
/// The table's sixteen are Team ProtoMan's: the first five from any ROM,
/// the souls from a ROM of that version, the last five from one where one
/// is given.
fn mugshots(roms: &Roms) -> (Vec<(Tiles, Palette)>, Vec<Tiles>) {
    let (mut faces, mut boxes) = (Vec::new(), Vec::new());
    let mut push = |src: Source, picture: u32, palette_index: u32| {
        let (t, b) = face(src, picture);
        let colors = if src.is_present() {
            palette(src.rom, src.at(FACE_PALETTES) + 0x20 * palette_index)
        } else {
            crate::placeholders::PALETTE
        };
        faces.push((t, colors));
        boxes.push(b);
    };
    let (protoman, colonel) = (roms.of(Version::ProtoMan), roms.of(Version::Colonel));
    for picture in 0..16 {
        let src = match picture {
            0..5 => roms.any(),
            5..11 => protoman,
            _ => roms.preferring(Version::ProtoMan),
        };
        push(src, picture, picture);
    }
    for picture in 5..11 {
        push(protoman, picture, picture + DARK_PALETTES);
    }
    for picture in 5..11 {
        push(colonel, picture, picture);
    }
    for picture in 5..11 {
        push(colonel, picture, picture + DARK_PALETTES);
    }
    (faces, boxes)
}

/// A team's six navis' faces, from a ROM of its version.
fn navi_mugshots(src: Source) -> impl Iterator<Item = NaviMugshot> {
    (0..NAVI_MUGSHOT_COUNT).map(move |n| {
        if !src.is_present() {
            return NaviMugshot {
                tiles: crate::placeholders::tiles(8),
                palettes: [crate::placeholders::PALETTE; 2],
            };
        }
        NaviMugshot {
            tiles: tiles(src.rom, src.at(NAVI_MUGSHOTS) + 0x100 * n, 0x100),
            palettes: std::array::from_fn(|k| palette(src.rom, src.at(NAVI_MUGSHOT_PALETTES) + 0x40 * n + 0x20 * k as u32)),
        }
    })
}

/// A source's dialogue font with its advances (a word a glyph), its glyphs
/// drawing `chars` (the 8x16 font's characters, then the dialogue font's
/// past them; a glyph past them its number in brackets).
pub(crate) fn dialogue_font(src: Source, (cell, dialogue): (&[String], &[String])) -> DialogueFont {
    let (rom, font, advances) = (src.rom, src.at(DIALOGUE_FONT), src.at(DIALOGUE_ADVANCES));
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

fn chatbox(src: Source) -> Chatbox {
    let (rom, at) = (src.rom, |place: Place| src.at(place));
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
        tiles: tiles(rom, at(CHATBOX_TILES), 0x20 * CHATBOX_TILE_COUNT),
        palette: palette(rom, at(CHATBOX_PALETTE)),
        boxes: (0..CHATBOX_KINDS)
            .map(|kind| {
                std::array::from_fn(|step| {
                    map(rom.u32(at(CHATBOX_BOXES) + 4 * (8 * kind + step as u32)))
                })
            })
            .collect(),
        arrow: tiles(rom, at(CHATBOX_ARROW), 3 * 0x80),
        text_palette: palette(rom, at(CHATBOX_TEXT_PALETTE)),
    }
}

/// A source's banner `id`, in its release's words.
pub(crate) fn banner(src: Source, id: u32) -> nettai_assets::BannerLayout {
    banner_at(src.rom, (src.at(BANNERS), src.at(BANNER_FILLER)), id)
}

/// The HUD's graphics; `chip_icons` the chips' (graphics.rs), `names` the
/// fonts' characters. What the four ROMs have alike is whichever ROM's is
/// there (`drawn`), the English words a US ROM's (`worded`), a team's own
/// faces a ROM's of that version. What no ROM given holds is a
/// placeholder.
pub fn hud(roms: &Roms, names: &AssetNames, chip_icons: Vec<ChipIcon>) -> Hud {
    let mut h = crate::placeholders::hud(names, BANNER_COUNT as usize);
    h.banners = crate::placeholders::banners(BANNER_COUNT as usize, &BANNER_RECORDS);
    (h.mugshots, h.mugshot_boxes) = mugshots(roms);
    // The team navis' by navi number: Team ProtoMan's six, then Team
    // Colonel's.
    h.navi_mugshots = navi_mugshots(roms.of(Version::ProtoMan)).chain(navi_mugshots(roms.of(Version::Colonel))).collect();
    h.chip_icons = chip_icons;
    let art = roms.any();
    if art.is_present() {
        h = drawn(art, h);
    }
    let words = roms.us();
    if words.is_present() {
        h = worded(words, names, h);
    }
    h
}

/// `h` with what the four ROMs have alike, read from `src`.
fn drawn(src: Source, h: Hud) -> Hud {
    let (rom, at) = (src.rom, |place: Place| src.at(place));
    let mut hud_tiles = tiles(rom, at(HUD_TILES), 0x640);
    for g in [TIMES_GLYPH, TWO_GLYPH] {
        let t = tiles(rom, at(g), 0x40);
        hud_tiles.push(t.get(0).unwrap());
        hud_tiles.push(t.get(1).unwrap());
    }
    // The banner digits, and the filler as glyph 10 (blank).
    let mut banner_digits = tiles(rom, at(BANNER_DIGITS), 0x40 * 10);
    let blank = tiles(rom, at(BANNER_FILLER), 0x40);
    banner_digits.push(blank.get(0).unwrap());
    banner_digits.push(blank.get(1).unwrap());
    // "PAUSE" as five glyphs: the first four are the 32x16 sprite's
    // columns (its tiles go row by row), the fifth the 8x16 sprite.
    let pause_tiles = tiles(rom, at(PAUSE), 0x140);
    let mut pause = Tiles::default();
    for (top, bottom) in [(0, 4), (1, 5), (2, 6), (3, 7), (8, 9)] {
        pause.push(pause_tiles.get(top).unwrap());
        pause.push(pause_tiles.get(bottom).unwrap());
    }
    Hud {
        tiles: hud_tiles,
        first_tile: HUD_FIRST_TILE,
        gauge_tiles: tiles(rom, at(GAUGE_TILES), GAUGE_BYTES),
        gauge_first_tile: GAUGE_FIRST_TILE,
        hp_palettes: std::array::from_fn(|i| palette(rom, at(HUD_PALETTES) + 0x20 * i as u32)),
        gauge_palette: palette(rom, at(HUD_PALETTES)),
        hp_box: map(rom, at(HP_BOX), 12),
        gauge_frame: map(rom, at(GAUGE_FRAME), 36),
        enemy_digits: ENEMY_DIGITS.map(|a| tiles(rom, at(a), 0x40 * 10)),
        enemy_palette: palette(rom, at(ENEMY_PALETTE)),
        hidden_icon: tiles(rom, at(HIDDEN_ICON), 0x80),
        icon_palette: palette(rom, at(ICON_PALETTE)),
        // The soul faces' count boxes: 0..=10.
        counts: (0..=10u32)
            .map(|n| tiles(rom, at(COUNTS) + 0x80 * (10 - n), 0x80))
            .collect(),
        // (EXE5 has no box without a count: its faces bring their own.)
        count_box: Tiles::default(),
        navi_box: tiles(rom, at(NAVI_BOX), 0x80),
        pause,
        banner_digits,
        warning: tiles(rom, at(WARNING), 0x100),
        warning_palette: palette(rom, at(WARNING_PALETTE)),
        chatbox: chatbox(src),
        ..h
    }
}

/// `h` with the words of `src`'s release (a US ROM's: the pack's own
/// language is English, and the Japanese ROMs' words are `lettering`'s).
fn worded(src: Source, names: &AssetNames, h: Hud) -> Hud {
    let (rom, at) = (src.rom, |place: Place| src.at(place));
    Hud {
        font: tiles(rom, at(FONT), 0x40 * FONT_GLYPHS),
        font_chars: (0..FONT_GLYPHS).map(|k| glyph_name(names, k)).collect(),
        texts: texts(rom, at(TEXTS)),
        banners: (0..BANNER_COUNT).map(|id| banner(src, id)).collect(),
        banner_palette: palette(rom, at(BANNER_PALETTE)),
        waiting: tiles(rom, at(WAITING), 0x200),
        waiting_palette: palette(rom, at(BANNER_PALETTE)),
        dialogue_font: dialogue_font(src, (&names.glyphs, &names.dialogue_glyphs)),
        ..h
    }
}
