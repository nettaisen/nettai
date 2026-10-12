//! HUD graphics for the pack's graphics. Everything here is uncompressed in
//! the ROM; the HUD tasks (`sub_801BF64`) copy it to VRAM as needed.
//!
//! Each block is stated by where the game's own pointer to it is (`Place`:
//! a routine's literal or a load list's entry, at the same address in both
//! versions' code where one pair of addresses is given), so any of the four
//! ROMs gives what it holds. The names in backticks are the US Falzar
//! ROM's.

pub(crate) use crate::decode::{banner_at, tiles};
use crate::exe6::{Place, Rom, Roms, Source, Version, u32at};
use nettai_assets::{
    BannerLayout, ChipIcon, DialogueFont, Hud, MapEntry, NaviMugshot, Palette, Tiles,
    palettes_from_bytes,
};
use nettai_content::names::AssetNames;

/// HUD layer tiles 0x1A0..=0x1D1: HP digits and blank, the box border,
/// damage digits, '+' (`dword_86E1638`, the battle's HUD load list's,
/// `off_801ECB4`: twelve bytes a block, its source first).
const HUD_TILES: Place = Place::pointer(0x0801_ECC0, 0x0801_F0E4);
/// Tiles 0x1D2..=0x1D5: the '×' and '2' of a doubled chip (the list's
/// next two blocks, two glyphs of the 8x16 font: the Japanese font has its
/// '×' at another number).
const TIMES_GLYPH: Place = Place::pointer(0x0801_ECD8, 0x0801_F0FC);
const TWO_GLYPH: Place = Place::pointer(0x0801_ECE4, 0x0801_F108);
/// Gauge tiles 0x222..=0x23D (`sub_801DED0`'s `dword_86E489C`), with the
/// gauge's word on them: each release's own.
pub(crate) const GAUGE_TILES: Place = Place::pointer(0x0801_DF6C, 0x0801_E390);
pub(crate) const GAUGE_BYTES: usize = 0x380;
/// Background palette 9 (gauge) and 13 (HP box: + 0x20 per color state),
/// `dword_86E1C78` (`sub_801DED0`'s too).
const HUD_PALETTES: Place = Place::pointer(0x0801_DF80, 0x0801_E3A4);
/// The initial HP box (6x2) and gauge frame (18x2) maps (`byte_801EDFC`,
/// `byte_801ED6C`).
const HP_BOX: Place = Place::pointer(0x0801_E05C, 0x0801_E480);
const GAUGE_FRAME: Place = Place::pointer(0x0801_DEBC, 0x0801_E2E0);
/// The 8x16 font (glyph k = 0x40 bytes; `dword_86B7AE0`, the literal
/// beside the text lines').
pub(crate) const FONT: Place = Place::pointer(0x0801_D1D0, 0x0801_D5F4);
pub(crate) const FONT_GLYPHS: u32 = 0xE0;
/// The HUD's text lines (`TextScript86F0374`): a table of 16-bit offsets,
/// then each line's glyphs up to its end mark.
pub(crate) const TEXTS: Place = Place::pointer(0x0801_D1C4, 0x0801_D5E8);
const TEXT_END: u8 = 0xE6;
/// The chip records (`ChipDataArr_8021DA8`, the chip accessor's literal):
/// 0x2C bytes a chip; +0x20 its icon, +0x24 its picture, +0x28 the
/// picture's palette.
pub(crate) const CHIPS: Place = Place::pointer(0x0802_1AB0, 0x0802_1EC4);
const CHIP_COUNT: u32 = 411;
/// The opponent's HP digits by color (`dword_86E0AB8`, `dword_86E0D38`,
/// `dword_86E0FB8`), and their sprite palette (`byte_86B7AC0`, the load
/// list's first block).
const ENEMY_DIGITS: [Place; 3] = [
    Place::pointer(0x0801_D854, 0x0801_DC78),
    Place::pointer(0x0801_D880, 0x0801_DCA4),
    Place::pointer(0x0801_D8AC, 0x0801_DCD0),
];
const ENEMY_PALETTE: Place = Place::pointer(0x0801_ECB4, 0x0801_F0D8);
/// A hidden chip's icon (`dword_872CE94`), and the icons' palette (the
/// load list's `dword_872CFD4`).
const HIDDEN_ICON: Place = Place::pointer(0x0801_C160, 0x0801_C57C);
const ICON_PALETTE: Place = Place::pointer(0x0801_ED20, 0x0801_F144);
/// Mugshot graphics by emotion (`off_801CD08`) and palettes
/// (`dword_872F114`). The first five are MegaMan's own in both versions;
/// the rest are the ROM's version's (`gregar::OWN_FACES`).
const MUGSHOTS: Place = Place::pointer(0x0801_CD04, 0x0801_D128);
const MUGSHOT_COUNT: u32 = 23;
const MUGSHOT_PALETTES: Place = Place::pointer(0x0801_CD74, 0x0801_D198);
/// Count box with n = 10 - i at + 0x80 * i (`dword_872E994`), and the
/// plain box (`dword_872D914`).
const COUNTS: Place = Place::pointer(0x0801_CDC4, 0x0801_D1E8);
const COUNT_BOX: Place = Place::pointer(0x0801_CD64, 0x0801_D188);
/// The link navis' mugshots (`sub_801CC34`): six faces of 0x100 bytes
/// (`dword_872D094`), the box beside them (`dword_872D014`), two palettes
/// a face (normal, Full Synchro; `dword_872D694`). The first five are the
/// ROM's version's navis (`gregar::OWN_NAVI_FACES`), the sixth ProtoMan in
/// both. (Which face each navi shows, `byte_801CDDC`, is its
/// definition's.)
const NAVI_MUGSHOTS: Place = Place::pointer(0x0801_CDCC, 0x0801_D1F0);
const NAVI_MUGSHOT_COUNT: u32 = 6;
const NAVI_BOX: Place = Place::pointer(0x0801_CDD4, 0x0801_D1F8);
const NAVI_MUGSHOT_PALETTES: Place = Place::pointer(0x0801_CDD0, 0x0801_D1F4);
/// The dialogue font (`byte_86ACD60`: 16x12 glyphs, 0x60 bytes each, a
/// row of eight bytes, the left pixel in each byte's low nibble) and its
/// advances (`byte_8043CA4`): the one-byte glyphs below 0xE4 and the
/// two-byte codes' (E4 00 - E4 E8) after them. (The chatbox's code, past
/// where the two versions' code parts: `off_803B638`, `off_8045ED8`.)
pub(crate) const DIALOGUE_FONT: Place = Place::pointer_in_each([0x0803_B638, 0x0803_B60C, 0x0803_C6B4, 0x0803_C688]);
pub(crate) const DIALOGUE_ADVANCES: Place = Place::pointer_in_each([0x0804_5ED8, 0x0804_5EA8, 0x0804_6F20, 0x0804_6EF0]);
const DIALOGUE_GLYPHS: u32 = 0xE4 + 0xE9;
/// The chatbox (`chatbox_runScript`): the box's eleven tiles (to BG0's
/// tile 0x2E4, which its maps count from; `off_804046C`) and palette
/// (`off_804047C`), the text's palette (the arrow's too; `off_8040484`);
/// the box's maps (`spritePtrArr8045CEC`: eight pointers a kind, the
/// opening steps 0-3 first, 30x8 entries each); the key-wait arrow's three
/// 16x16 frames (`chatbox_804082C`'s `off_8040888`).
const CHATBOX_TILES: Place = Place::pointer_in_each([0x0804_046C, 0x0804_0440, 0x0804_16A4, 0x0804_1678]);
const CHATBOX_TILE_COUNT: usize = 11;
const CHATBOX_FIRST_TILE: u16 = 0x2E4;
const CHATBOX_PALETTE: Place = Place::pointer_in_each([0x0804_047C, 0x0804_0450, 0x0804_16B4, 0x0804_1688]);
const CHATBOX_TEXT_PALETTE: Place = Place::pointer_in_each([0x0804_0484, 0x0804_0458, 0x0804_16BC, 0x0804_1690]);
const CHATBOX_BOXES: Place = Place::pointer_in_each([0x0804_036C, 0x0804_0340, 0x0804_15A4, 0x0804_1578]);
/// The message box (`E8 00`) and the description box (`E8 06 01`).
const CHATBOX_KINDS: u32 = 2;
const CHATBOX_ARROW: Place = Place::pointer_in_each([0x0804_0888, 0x0804_085C, 0x0804_1AC0, 0x0804_1A94]);

/// "PAUSE" (`off_801E188`): a 32x16 sprite's eight tiles and an 8x16
/// one's two.
const PAUSE: Place = Place::pointer(0x0801_E188, 0x0801_E5AC);
/// Banner descriptors (`pt_801EF84`: every banner up to the navis' win
/// and deletion banners; `off_801E928`), their glyph filler
/// (`off_801E958`), digits (`off_801FD64`) and palette (`off_801E938`).
pub(crate) const BANNERS: Place = Place::pointer(0x0801_E928, 0x0801_ED4C);
const BANNER_COUNT: u32 = 47;
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
    (0x60 / 4, 1, (40, 64), Some((56, 64))),
];
pub(crate) const BANNER_FILLER: Place = Place::pointer(0x0801_E958, 0x0801_ED7C);
const BANNER_DIGITS: Place = Place::pointer(0x0801_FD64, 0x0802_0178);
pub(crate) const BANNER_PALETTE: Place = Place::pointer(0x0801_E938, 0x0801_ED5C);
/// "Cstmzing..." (16 tiles, drawn on the HUD layer in palette 10 =
/// the banner palette; `off_801EF30`). The Japanese ROMs' is 14 tiles.
pub(crate) const WAITING: Place = Place::pointer(0x0801_EF30, 0x0801_F354);
/// The warning marker's two 16x16 frames (`dword_86E55FC`, which the gauge
/// chips' controller, VDoll's curse and LifeSync copy to sprite tiles
/// 0x3CA..=0x3D1; the custom screen's sprite list has them too,
/// `off_802A744`'s second block, which is the pointer read) and its
/// palette (`byte_86E56FC`, sprite palette 13: the first of the emblems'
/// palettes, `sub_802812C`'s literal).
const WARNING: Place = Place::pointer_in_each([0x0802_A750, 0x0802_A74C, 0x0802_AB58, 0x0802_AB54]);
const WARNING_PALETTE: Place = Place::pointer(0x0802_8184, 0x0802_8598);

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

/// A source's banner `id`, in its release's words.
pub(crate) fn banner(src: Source, id: u32) -> BannerLayout {
    banner_at(src.rom, (src.at(BANNERS), src.at(BANNER_FILLER)), id)
}

/// An emotion-window face of a source's table.
fn mugshot(src: Source, e: u32) -> (Tiles, Palette) {
    if !src.is_present() {
        return (crate::placeholders::tiles(8), crate::placeholders::PALETTE);
    }
    let rom = src.rom;
    (
        tiles(rom, u32at(rom, src.at(MUGSHOTS) + 4 * e), 0x100),
        palette(rom, src.at(MUGSHOT_PALETTES) + 0x20 * e),
    )
}

/// A link navi's face of a source's table.
fn navi_mugshot(src: Source, n: u32) -> NaviMugshot {
    if !src.is_present() {
        return NaviMugshot {
            tiles: crate::placeholders::tiles(8),
            palettes: [crate::placeholders::PALETTE; 2],
        };
    }
    let rom = src.rom;
    NaviMugshot {
        tiles: tiles(rom, src.at(NAVI_MUGSHOTS) + 0x100 * n, 0x100),
        palettes: std::array::from_fn(|k| palette(rom, src.at(NAVI_MUGSHOT_PALETTES) + 0x40 * n + 0x20 * k as u32)),
    }
}

/// The HUD's graphics; `names` gives the chip icons their chips' keys and
/// the font its characters. What the four ROMs have alike is whichever
/// ROM's is there (`drawn`); the English words a US ROM's (`worded`); a
/// version's own faces and chips' icons a ROM's of that version. What no
/// ROM given holds is a placeholder.
pub fn hud(roms: &Roms, names: &AssetNames) -> Hud {
    use crate::exe6::gregar::{OWN_FACES, OWN_NAVI_FACES};
    let mut h = crate::placeholders::hud(names, BANNER_COUNT as usize);
    h.banners = crate::placeholders::banners(BANNER_COUNT as usize, &BANNER_RECORDS);
    h.chip_icons = chip_icons(roms, names);
    // A ROM's table of faces: MegaMan's own, which both versions have, then
    // its version's. The pack has the Falzar ROMs' table, then Gregar's own.
    let face = |e: u32| if OWN_FACES.contains(&e) { roms.of(Version::Falzar) } else { roms.any() };
    h.mugshots = (0..MUGSHOT_COUNT)
        .map(|e| mugshot(face(e), e))
        .chain(OWN_FACES.map(|e| mugshot(roms.of(Version::Gregar), e)))
        .collect();
    // Likewise the link navis': the version's five, then ProtoMan.
    let navi_face = |n: u32| if n < OWN_NAVI_FACES { roms.of(Version::Falzar) } else { roms.any() };
    h.navi_mugshots = (0..NAVI_MUGSHOT_COUNT)
        .map(|n| navi_mugshot(navi_face(n), n))
        .chain((0..OWN_NAVI_FACES).map(|n| navi_mugshot(roms.of(Version::Gregar), n)))
        .collect();
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
        first_tile: 0x1A0,
        gauge_first_tile: 0x222,
        hp_palettes: std::array::from_fn(|i| palette(rom, at(HUD_PALETTES) + 0x20 * i as u32)),
        gauge_palette: palette(rom, at(HUD_PALETTES)),
        hp_box: map(rom, at(HP_BOX), 12),
        gauge_frame: map(rom, at(GAUGE_FRAME), 36),
        enemy_digits: ENEMY_DIGITS.map(|a| tiles(rom, at(a), 0x40 * 10)),
        enemy_palette: palette(rom, at(ENEMY_PALETTE)),
        hidden_icon: tiles(rom, at(HIDDEN_ICON), 0x80),
        icon_palette: palette(rom, at(ICON_PALETTE)),
        counts: (0..=10u32)
            .map(|n| tiles(rom, at(COUNTS) + 0x80 * (10 - n), 0x80))
            .collect(),
        count_box: tiles(rom, at(COUNT_BOX), 0x80),
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
        gauge_tiles: tiles(rom, at(GAUGE_TILES), GAUGE_BYTES),
        font: tiles(rom, at(FONT), 0x40 * FONT_GLYPHS as usize),
        font_chars: names
            .glyphs
            .iter()
            .take(FONT_GLYPHS as usize)
            .cloned()
            .collect(),
        dialogue_font: dialogue_font(src, (&names.glyphs, &names.dialogue_glyphs)),
        texts: texts(rom, at(TEXTS)),
        banners: (0..BANNER_COUNT).map(|id| banner(src, id)).collect(),
        banner_palette: palette(rom, at(BANNER_PALETTE)),
        waiting: tiles(rom, at(WAITING), 0x200),
        waiting_palette: palette(rom, at(BANNER_PALETTE)),
        ..h
    }
}

/// A source's dialogue font with its advances, its glyphs drawing `chars`
/// (the 8x16 font's characters, then the dialogue font's past them).
pub(crate) fn dialogue_font(src: Source, (cell, dialogue): (&[String], &[String])) -> DialogueFont {
    let rom = src.rom;
    DialogueFont {
        pixels: rom
            .bytes(src.at(DIALOGUE_FONT), 0x60 * DIALOGUE_GLYPHS as usize)
            .iter()
            .flat_map(|&b| [b & 15, b >> 4])
            .collect(),
        advances: rom.bytes(src.at(DIALOGUE_ADVANCES), DIALOGUE_GLYPHS as usize).to_vec(),
        chars: cell
            .iter()
            .chain(dialogue)
            .take(DIALOGUE_GLYPHS as usize)
            .cloned()
            .collect(),
    }
}

fn chatbox(src: Source) -> nettai_assets::Chatbox {
    let (rom, at) = (src.rom, |place: Place| src.at(place));
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
        tiles: tiles(rom, at(CHATBOX_TILES), 0x20 * CHATBOX_TILE_COUNT),
        palette: palette(rom, at(CHATBOX_PALETTE)),
        boxes: (0..CHATBOX_KINDS)
            .map(|kind| {
                std::array::from_fn(|step| {
                    map(u32at(rom, at(CHATBOX_BOXES) + 4 * (8 * kind + step as u32)))
                })
            })
            .collect(),
        arrow: tiles(rom, at(CHATBOX_ARROW), 3 * 0x80),
        text_palette: palette(rom, at(CHATBOX_TEXT_PALETTE)),
    }
}

fn chip_icons(roms: &Roms, names: &AssetNames) -> Vec<ChipIcon> {
    (0..CHIP_COUNT)
        .map(|id| {
            let src = crate::exe6::gregar::chip_source(roms, id);
            let rom = src.rom;
            let icon = if rom.is_present() {
                let p = u32at(rom, src.at(CHIPS) + 0x2c * id + 0x20);
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
