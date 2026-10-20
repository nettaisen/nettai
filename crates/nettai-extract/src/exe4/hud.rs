//! The HUD's graphics: what EXE4's HUD tasks (0x08014D10's table at 0x08014D34,
//! EXE6 `sub_801BF64`'s counterpart) draw and copy to VRAM, read from those
//! tasks and the routines that start them. EXE4's HUD is laid out as EXE6's
//! (the HUD layer's tiles from 0x130 in EXE6's order: the HP digits, the
//! box, the damage digits, '?', '+'; the gauge's from 0x80 in EXE6's order),
//! its emotion window as EXE5's (a face brings its box, a soul's face shows
//! a count), and it has no link navis' faces.
//!
//! Each block is stated by where the game's own pointer to it is in each of
//! the four ROMs (`Place`: one address in both versions' code of a release,
//! but for the chatbox's); the routines that load them are named by Red Sun
//! US's addresses, and the blocks' addresses in the comments are Red Sun
//! US's. So any ROM gives what it holds: what the four have alike from
//! whichever is there, the English words from a US ROM, a version's souls'
//! faces from a ROM of that version.

use crate::exe4::rom::{Place, Rom, Roms, Source, Version};
use nettai_assets::{
    BannerLayout, Chatbox, ChipIcon, Hud, HudLayout, MapEntry, Palette, TelopLook, TelopTiles, Tiles,
    palettes_from_bytes,
};
use nettai_content::names::AssetNames;

/// The battle's HUD load list (0x08015A0C, EXE6 `off_801ECB4`'s
/// counterpart, which 0x080159F4 queues): the HUD layer's tiles to tile
/// 0x130 (the HP digits, the box's, the damage digits, '?', '+', ':'), then
/// the 8x16 font's '×' (glyph 0x41) and '2' (glyph 3) to tiles 0x162 and
/// 0x164 (the doubled chip's "×2", 0x08015190).
const HUD_TILES: Place = Place::pointer(0x0801_5A18, 0x0801_5974);
const HUD_FIRST_TILE: u16 = 0x130;
const TIMES_GLYPH: Place = Place::pointer(0x0801_5A30, 0x0801_598C);
const TWO_GLYPH: Place = Place::pointer(0x0801_5A3C, 0x0801_5998);
/// The gauge's tiles (0x08016824 and 0x08016842: to tile 0x80), its frame
/// (0x08015F1C: 18x2 at column 6, row 0, with "CUSTOM" over it) and the HP
/// box (0x08015EFC: 6x2 where the HUD block's +7 and +8 put it). The
/// gauge's tiles, "CUSTOM" among them, are the same in the four ROMs.
const GAUGE_TILES: Place = Place::pointer(0x0801_686C, 0x0801_67C0);
const GAUGE_BYTES: usize = 0x380;
const GAUGE_FIRST_TILE: u16 = 0x80;
const GAUGE_FRAME: Place = Place::pointer(0x0801_5F64, 0x0801_5EB8);
const HP_BOX: Place = Place::pointer(0x0801_5F18, 0x0801_5E6C);
/// Background palette 13, the HP box's, by color (normal, healing, hurt or
/// low: 0x08014D98 copies + 0x20 a color); the gauge's palette (9) is the
/// first (0x08015F1C).
const HUD_PALETTES: Place = Place::pointer(0x0801_5A24, 0x0801_5980);
/// The opponent's HP digits (0x08014EB8: a pointer a digit, 0x08014F9C;
/// one color, the digits of 0x08707580 on), a sprite in palette 14, which
/// the load list's first fills (0x08707800, to 0x03002800: EXE5's
/// 0x086CBA48, EXE6's `byte_86B7AC0`'s place).
const ENEMY_DIGITS: Place = Place::pointer(0x0801_4F9C, 0x0801_4F3C);
const ENEMY_PALETTE: Place = Place::pointer(0x0801_5A0C, 0x0801_5968);
/// The chip icons' sprite palette (the load list's to 0x03002780: EXE5's
/// 0x0874AAF8's place) and the hidden chip's icon (the load list's to
/// sprite tile 0x35C).
const ICON_PALETTE: Place = Place::pointer(0x0801_5A84, 0x0801_59E0);
const HIDDEN_ICON: Place = Place::pointer(0x0801_5A9C, 0x0801_59F8);
/// The emotion window (0x08014B78): picture n < 5 (MegaMan's faces) at
/// FACES + 0x180 n (the 32x16 face, then its 16x16 box), n >= 5 (the
/// version's souls) at SOUL_FACES + 0x100 (n - 5), beside the count box
/// showing c at COUNTS + 0x80 (5 - c); picture n's palette at PALETTES +
/// 0x20 n (sprite palette 12), white while Full Synchro (3) blinks in.
/// MegaMan's five faces, the counts and the first five palettes are the
/// same in the four ROMs; the souls' faces and palettes are each version's
/// own six (0x08014B78's literals).
const FACES: Place = Place::pointer(0x0801_4C54, 0x0801_4BF4);
const SOUL_FACES: Place = Place::pointer(0x0801_4C58, 0x0801_4BF8);
const COUNTS: Place = Place::pointer(0x0801_4CB4, 0x0801_4C54);
const FACE_PALETTES: Place = Place::pointer(0x0801_4C68, 0x0801_4C08);
/// The faces MegaMan has of his own (normal, angry, worried, Full Synchro,
/// dark), and each version's souls.
const OWN_FACES: u32 = 5;
const SOULS: u32 = 6;
/// The highest count the window's box shows.
const COUNT_MAX: u32 = 5;
/// The dialogue font (16x12 glyphs of 0x60 bytes) and its advances: the
/// extractor's graphics.rs reads them (`dialogue_font`).
/// "PAUSE" (0x080166BC's list, 0x080166D0, to sprite tile 0x360: a 32x16
/// sprite's eight tiles and an 8x16 one's two), drawn by 0x0801554C at
/// (100, 64) in palette 14.
const PAUSE: Place = Place::pointer(0x0801_66D0, 0x0801_6624);
/// The banners (0x0801617E: a pointer a banner, by its number,
/// `BANNERS`): a record's halfword place (x, then y), its kind
/// byte, then 20 glyph pointers (no filler: EXE4 reads twenty), then a
/// kind-1 banner's number place; the banner font's digits (a pointer a
/// digit, the tenth the blank: a zeroed block) and the banners' palette (to
/// 0x030027A0, sprite palette 11). 41 banners: EXE6's first fifteen in
/// EXE6's places, then EXE4's navis' wins and deletions. The records and
/// their glyphs are each release's words; the digits and the palette are
/// the same in the four ROMs.
const BANNERS: Place = Place::pointer(0x0801_6264, 0x0801_61B8);
const BANNER_DIGITS: Place = Place::pointer(0x0801_626C, 0x0801_61C0);
const BANNER_PALETTE: Place = Place::pointer(0x0801_6274, 0x0801_61C8);
pub(crate) const BANNER_COUNT: u32 = 41;
/// The banners that aren't plain, as the US ROMs' records have them (the
/// kind, the place, a kind-1 banner's number's place), for a HUD of
/// placeholders (`placeholders::banners`): the turn's and the round's
/// numbered banners, the Program Advance's two that hold, and the judge's
/// (`JUDGE_KIND`). EXE4's telops aren't banners' (`TELOP_PLACES`).
pub(crate) const BANNER_RECORDS: [crate::placeholders::BannerRecord; 6] = [
    (0x0C / 4, 1, (48, 64), Some((104, 64))),
    (0x20 / 4, 1, (68, 64), Some((140, 64))),
    (0x24 / 4, 2, (16, 24), None),
    (0x28 / 4, 4, (40, 16), None),
    (0x30 / 4, 1, (40, 64), Some((112, 64))),
    (0x34 / 4, 2, (136, 24), None),
];
/// "BUSY..." (7x2 tiles: 0x080166F0's list, 0x08016710, to tile 0xAC, with
/// the banners' palette as background palette 10), drawn by 0x08015624 at
/// column 22, row 4 while the other player is still choosing; each
/// release's words.
const WAITING: Place = Place::pointer(0x0801_6710, 0x0801_6664);
const WAITING_BYTES: usize = 0x1C0;
/// The warning marker (0x08008424, EXE6 `sub_800AE90`'s counterpart: a
/// 16x16 sprite at sprite tile 0x360, its second frame 4 tiles on, in
/// palette 13): its two frames and its palette, which the objects that show
/// it load (0x080E3FC8's list).
const WARNING: Place = Place::pointer(0x0801_DE74, 0x0801_DDB4);
const WARNING_PALETTE: Place = Place::pointer(0x0802_0B24, 0x0802_0A64);
/// "PAUSE"'s place (0x0801554C: its 32x16 sprite at x 100, y 64), and the
/// priority the HP numbers under objects are drawn at (0x08014EB8's
/// 0xE730: 1).
const PAUSE_AT: (u8, u8) = (100, 64);
const HP_NUMBER_PRIORITY: u8 = 1;
/// The HUD's layer's priority: BG3's (the battle's video init's table at
/// 0x08006AD4: BG3CNT 0x1F00, priority 0; the custom screen's 0x1F08 too),
/// over the HP numbers where the custom screen is.
const HUD_PRIORITY: u8 = 0;
/// A message (0x08015FE8: "COUNTER HIT!", text line 14, rendered 14 glyphs
/// wide by 0x080162CC): its map from column 8 of row 2, 14 columns wide.
const MESSAGE_AT: (u8, u8, u8) = (8, 2, 14);
/// The telops (0x0801650C, which lays out the user's on the banner block
/// and the other player's on the second block): fifteen glyphs centered
/// from x 0 on the user's console and from x 120 on the other's, at y 32;
/// after the chip's name the HUD layer's damage digits (TELOP_DIGITS' ten
/// pointers, then a blank and '+') and the font's '×' and '2' (0x08016680's
/// list), in their own palette (to 0x030027A0, sprite palette 11, which the
/// banners' shares).
const TELOP_PLACES: [(u8, u8); 2] = [(0, 32), (120, 32)];
const TELOP_DIGITS: Place = Place::pointer(0x0801_6644, 0x0801_6598);
const TELOP_PALETTE: Place = Place::pointer(0x0801_6660, 0x0801_65B4);
/// Where the telops' sprite tiles go among the banners': a banner's glyphs
/// from tile 0x360 (0x0801617E's transfers to 0x06016C00, five 32x16
/// sprites drawn by 0x080151FC); a telop's name from 0x360 on the left and
/// 0x384 on the right (0x0801662A's 0x06016C00 and 0x06017080; 0x08015488
/// draws a telop from x 104 on from 0x360 + 36); the '×' and '2' pieces
/// into 0x380 to 0x383 (0x08016680's list, whichever telop); "PAUSE"
/// loads the banners' palette, 0x08750E40, into sprite palette 11 with its
/// tiles (0x080166D0's list).
const TELOP_TILES: TelopTiles = TelopTiles { banner: 0x360, names: [0x360, 0x384], signs: 0x380, pause_loads_banner_palette: true };
/// The chatbox (0x0804E3B4, `chatbox_runScript`'s counterpart): the box's
/// twenty tiles (to BG0's tile 0x2AC, which its maps count from) and
/// palette, the text's palette; the box's maps (0x0804E384: a kind's four
/// opening steps, 30x8 entries each); the key-wait arrow's three 16x16
/// frames.
const CHATBOX_TILES: Place = Place::pointer_in_each([0x0804_E484, 0x0804_E48C, 0x0804_E374, 0x0804_E37C]);
const CHATBOX_TILE_COUNT: usize = 20;
const CHATBOX_FIRST_TILE: u16 = 0x2AC;
const CHATBOX_PALETTE: Place = Place::pointer_in_each([0x0804_E490, 0x0804_E498, 0x0804_E380, 0x0804_E388]);
const CHATBOX_TEXT_PALETTE: Place = Place::pointer_in_each([0x0804_E498, 0x0804_E4A0, 0x0804_E388, 0x0804_E390]);
const CHATBOX_BOXES: Place = Place::pointer_in_each([0x0804_E37C, 0x0804_E384, 0x0804_E26C, 0x0804_E274]);
const CHATBOX_ARROW: Place = Place::pointer_in_each([0x0804_E5C8, 0x0804_E5D0, 0x0804_E4B8, 0x0804_E4C0]);
/// Where the chatbox's text starts (0x0804E3B4's +0x18, +0x19: 0x3F, 0x6D;
/// EXE6's 0x33, 0x6C). The portrait's place is EXE6's.
const CHATBOX_TEXT: (u8, u8) = (0x3F, 0x6D);
/// The Japanese games' (Red Sun Japan's 0x0804E2A4, the same routine: 0x33,
/// 0x6C, EXE6's place; the US games moved it twelve pixels right and one
/// down). Its arrow's place is the same.
pub(crate) const CHATBOX_TEXT_JA: (u8, u8) = (0x33, 0x6C);
/// The key-wait arrow's place (0x0804E3B4's +0x1A, +0x1B), EXE6's message
/// box's.
const CHATBOX_ARROW_AT: (u8, u8) = (0xE2, 0x8D);

/// A palette (the hardware ignores bit 15 of a color).
pub(crate) fn palette(rom: &Rom, a: u32) -> Palette {
    if !rom.is_present() {
        return crate::placeholders::PALETTE;
    }
    palettes_from_bytes(rom.bytes(a, 32))[0].map(|c| c & 0x7FFF)
}

pub(crate) fn tiles(rom: &Rom, a: u32, len: usize) -> Tiles {
    crate::decode::tiles(rom, a, len)
}

fn map(rom: &Rom, a: u32, n: u32) -> Vec<MapEntry> {
    (0..n).map(|i| MapEntry::from_gba(rom.u16(a + 2 * i))).collect()
}

/// The damage judge's banner (number 0x28, which 0x080163EE shows as it
/// turns on the HUD's element 0x200: 0x080152EA draws "VS" at column 14,
/// row 5, the one side's damage in columns 9 to 12 and the other's from
/// column 17, as EXE6's `sub_801D048` does): the judge's layout, which adds
/// the two numbers (`nettai_assets::BannerLayout::kind` 4). Its record's
/// own kind byte, 2 (held until removed), says nothing of them.
const JUDGE_BANNER: u32 = 0x28 / 4;
const JUDGE_KIND: u8 = 4;

/// A source's banner `id` (the banner block's number / 4), in its release's
/// words.
pub(crate) fn banner(src: Source, id: u32) -> BannerLayout {
    let rom = src.rom;
    let p = rom.u32(src.at(BANNERS) + 4 * id);
    let place = rom.u16(p);
    let kind = if id == JUDGE_BANNER { JUDGE_KIND } else { rom.u8(p + 2) };
    let mut glyphs = Tiles::default();
    for g in 0..20 {
        let t = tiles(rom, rom.u32(p + 4 + 4 * g), 0x40);
        glyphs.push(t.get(0).unwrap());
        glyphs.push(t.get(1).unwrap());
    }
    let number_at = (kind == 1).then(|| {
        let n = rom.u16(p + 4 + 4 * 20);
        (n as u8, (n >> 8) as u8)
    });
    BannerLayout { x: place as u8, y: (place >> 8) as u8, kind, glyphs, number_at, bar: None, logo: None }
}

/// The banner font's digits (a pointer a glyph), and the blank as glyph 10.
fn banner_digits(src: Source) -> Tiles {
    let (rom, table) = (src.rom, src.at(BANNER_DIGITS));
    let mut t = Tiles::default();
    for d in 0..=10 {
        let g = tiles(rom, rom.u32(table + 4 * d), 0x40);
        t.push(g.get(0).unwrap());
        t.push(g.get(1).unwrap());
    }
    t
}

/// "BUSY..." in a source's release's words.
pub(crate) fn waiting(src: Source) -> Tiles {
    tiles(src.rom, src.at(WAITING), WAITING_BYTES)
}

/// An emotion-window face of a source (4x2 tiles) and the box it brings
/// (2x2; none for a soul's, which shows a count), with its palette.
fn face(src: Source, picture: u32) -> ((Tiles, Palette), Tiles) {
    let own = picture < OWN_FACES;
    if !src.is_present() {
        let none = crate::placeholders::tiles;
        return ((none(8), crate::placeholders::PALETTE), if own { none(4) } else { Tiles::default() });
    }
    let rom = src.rom;
    let colors = palette(rom, src.at(FACE_PALETTES) + 0x20 * picture);
    if own {
        let at = src.at(FACES) + 0x180 * picture;
        ((tiles(rom, at, 0x100), colors), tiles(rom, at + 0x100, 0x80))
    } else {
        ((tiles(rom, src.at(SOUL_FACES) + 0x100 * (picture - OWN_FACES), 0x100), colors), Tiles::default())
    }
}

/// The emotion window's faces, as compat/assets.toml numbers them (its
/// [mugshots]): MegaMan's five and Red Sun's souls in their own palettes
/// (the face table's eleven pictures), then Blue Moon's souls. Each with
/// the box it brings (`Hud::mugshot_boxes`). MegaMan's are any ROM's, a
/// version's souls' a ROM's of that version.
fn mugshots(roms: &Roms) -> (Vec<(Tiles, Palette)>, Vec<Tiles>) {
    let own = (0..OWN_FACES).map(|picture| face(roms.any(), picture));
    let souls = |version: Version| (OWN_FACES..OWN_FACES + SOULS).map(move |picture| face(roms.of(version), picture));
    own.chain(souls(Version::RedSun)).chain(souls(Version::BlueMoon)).unzip()
}

fn chatbox(src: Source) -> Chatbox {
    let (rom, at) = (src.rom, |place: Place| src.at(place));
    let n = Chatbox::COLUMNS * Chatbox::ROWS;
    let map = |a: u32| -> Vec<MapEntry> {
        (0..n as u32)
            .map(|i| {
                let e = MapEntry::from_gba(rom.u16(a + 2 * i));
                MapEntry { tile: e.tile.wrapping_sub(CHATBOX_FIRST_TILE), ..e }
            })
            .collect()
    };
    // (The message box's four steps; EXE4 shows a chip's description in the
    // same box, the table's second kind being no maps: both kinds are it.)
    let message: [Vec<MapEntry>; 4] = std::array::from_fn(|step| map(rom.u32(at(CHATBOX_BOXES) + 4 * step as u32)));
    Chatbox {
        tiles: tiles(rom, at(CHATBOX_TILES), 0x20 * CHATBOX_TILE_COUNT),
        palette: palette(rom, at(CHATBOX_PALETTE)),
        boxes: vec![message.clone(), message],
        arrow: tiles(rom, at(CHATBOX_ARROW), 3 * 0x80),
        text_palette: palette(rom, at(CHATBOX_TEXT_PALETTE)),
    }
}

/// The telops' look (`TELOP_PLACES`): the digits and '+' by the telop's
/// table (its eleventh pointer the blank, which it draws nothing of), the
/// font's '×' and '2'. Where and how they are laid out is the game's code;
/// the glyphs are a ROM's (placeholders with none).
fn telop(src: Source) -> TelopLook {
    if !src.is_present() {
        let glyphs = crate::placeholders::tiles(2 * TelopLook::GLYPHS);
        return TelopLook { places: TELOP_PLACES, glyphs, palette: crate::placeholders::PALETTE, tiles: Some(TELOP_TILES) };
    }
    let rom = src.rom;
    let mut glyphs = Tiles::default();
    let mut push = |a: u32| {
        let t = tiles(rom, a, 0x40);
        glyphs.push(t.get(0).unwrap());
        glyphs.push(t.get(1).unwrap());
    };
    for d in (0..10).chain([11]) {
        push(rom.u32(src.at(TELOP_DIGITS) + 4 * d));
    }
    push(src.at(TIMES_GLYPH));
    push(src.at(TWO_GLYPH));
    TelopLook { places: TELOP_PLACES, glyphs, palette: palette(rom, src.at(TELOP_PALETTE)), tiles: Some(TELOP_TILES) }
}

/// The HUD's graphics; `names` the fonts' characters, `chip_icons` the
/// chips' (graphics.rs). What the four ROMs have alike is whichever ROM's
/// is there (`drawn`), the English words a US ROM's (`worded`), a version's
/// souls' faces a ROM's of that version. What no ROM given holds is a
/// placeholder; how the HUD is laid out is the game's code, whichever ROMs
/// are given.
pub fn hud(roms: &Roms, names: &AssetNames, chip_icons: Vec<ChipIcon>) -> Hud {
    let encoding = (names.glyphs.as_slice(), names.dialogue_glyphs.as_slice());
    let (font, font_chars) = super::graphics::placeholder_font(encoding);
    let (mugshots, mugshot_boxes) = mugshots(roms);
    let art = roms.any();
    let mut h = Hud {
        font,
        font_chars,
        banners: crate::placeholders::banners(BANNER_COUNT as usize, &BANNER_RECORDS),
        chip_icons,
        mugshots,
        mugshot_boxes,
        layout: HudLayout {
            pause: PAUSE_AT,
            hp_number_priority: HP_NUMBER_PRIORITY,
            hud_priority: HUD_PRIORITY,
            message: MESSAGE_AT,
            // (The damage judge's numbers, task 9 from 0x080163C8: from its
            // holding banner's hold, 0x08014AA8, until the judge's release,
            // 0x080163B6, slides the banner out and clears them.)
            judge_from_hold: true,
            chatbox_text: CHATBOX_TEXT,
            // (A description shows in the message box: its arrow's place.)
            chatbox_arrows: [CHATBOX_ARROW_AT; 2],
            telop: Some(telop(art)),
        },
        ..crate::placeholders::hud(names, 0)
    };
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
    // "PAUSE" as five glyphs: the first four are the 32x16 sprite's
    // columns (its tiles go row by row), the fifth the 8x16 sprite.
    let pause_tiles = tiles(rom, at(PAUSE), 0x140);
    let mut pause = Tiles::default();
    for (top, bottom) in [(0, 4), (1, 5), (2, 6), (3, 7), (8, 9)] {
        pause.push(pause_tiles.get(top).unwrap());
        pause.push(pause_tiles.get(bottom).unwrap());
    }
    // (The pointer is the first of a digit's each: the digits follow one
    // another from it.)
    let digits = tiles(rom, at(ENEMY_DIGITS), 0x40 * 10);
    let banner_palette = palette(rom, at(BANNER_PALETTE));
    Hud {
        tiles: hud_tiles,
        first_tile: HUD_FIRST_TILE,
        gauge_tiles: tiles(rom, at(GAUGE_TILES), GAUGE_BYTES),
        gauge_first_tile: GAUGE_FIRST_TILE,
        hp_palettes: std::array::from_fn(|i| palette(rom, at(HUD_PALETTES) + 0x20 * i as u32)),
        gauge_palette: palette(rom, at(HUD_PALETTES)),
        hp_box: map(rom, at(HP_BOX), 12),
        gauge_frame: map(rom, at(GAUGE_FRAME), 36),
        // (One color: EXE4's numbers don't change color as HP drops or rises.)
        enemy_digits: std::array::from_fn(|_| digits.clone()),
        enemy_palette: palette(rom, at(ENEMY_PALETTE)),
        hidden_icon: tiles(rom, at(HIDDEN_ICON), 0x80),
        icon_palette: palette(rom, at(ICON_PALETTE)),
        // The count boxes: 0..=5.
        counts: (0..=COUNT_MAX).map(|c| tiles(rom, at(COUNTS) + 0x80 * (COUNT_MAX - c), 0x80)).collect(),
        // (EXE4 has no box without a count: its faces bring their own.)
        count_box: Tiles::default(),
        // (EXE4 has no link navis.)
        navi_mugshots: Vec::new(),
        navi_box: Tiles::default(),
        pause,
        banner_digits: banner_digits(src),
        banner_palette,
        waiting_palette: banner_palette,
        warning: tiles(rom, at(WARNING), 0x100),
        warning_palette: palette(rom, at(WARNING_PALETTE)),
        chatbox: chatbox(src),
        ..h
    }
}

/// `h` with the words of `src`'s release (a US ROM's: the pack's own
/// language is English, and the Japanese ROMs' words are graphics.rs's
/// `lettering`).
fn worded(src: Source, names: &AssetNames, h: Hud) -> Hud {
    let encoding = (names.glyphs.as_slice(), names.dialogue_glyphs.as_slice());
    let (font, font_chars) = super::graphics::font(src, encoding);
    Hud {
        font,
        font_chars,
        dialogue_font: super::graphics::dialogue_font(src, encoding),
        texts: super::graphics::texts(src),
        banners: (0..BANNER_COUNT).map(|id| banner(src, id)).collect(),
        waiting: waiting(src),
        ..h
    }
}
