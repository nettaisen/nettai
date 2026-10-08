//! The HUD's graphics, from Red Sun US (Blue Moon's own souls' faces from
//! Blue Moon US): what EXE4's HUD tasks (0x08014D10's table at 0x08014D34,
//! EXE6 `sub_801BF64`'s counterpart) draw and copy to VRAM, read from those
//! tasks and the routines that start them. EXE4's HUD is laid out as EXE6's
//! (the HUD layer's tiles from 0x130 in EXE6's order: the HP digits, the
//! box, the damage digits, '?', '+'; the gauge's from 0x80 in EXE6's order),
//! its emotion window as EXE5's (a face brings its box, a soul's face shows
//! a count), and it has no link navis' faces.
//!
//! The addresses are Red Sun US's; the routines that load them are named
//! by Red Sun US's addresses too.

use crate::exe4::rom::{Addresses, Rom};
use nettai_assets::{
    BannerLayout, Chatbox, ChipIcon, DialogueFont, Hud, HudLayout, MapEntry, Palette, TelopLook, Tiles, palettes_from_bytes,
};
use nettai_content::names::AssetNames;

/// The battle's HUD load list (0x08015A0C, EXE6 `off_801ECB4`'s
/// counterpart, which 0x080159F4 queues): the HUD layer's tiles to tile
/// 0x130 (the HP digits, the box's, the damage digits, '?', '+', ':'), then
/// the 8x16 font's '×' (glyph 0x41) and '2' (glyph 3) to tiles 0x162 and
/// 0x164 (the doubled chip's "×2", 0x08015190).
const HUD_TILES: u32 = 0x0870_7C20;
const HUD_FIRST_TILE: u16 = 0x130;
const TIMES_GLYPH: u32 = 0x0868_EF9C;
const TWO_GLYPH: u32 = 0x0868_E01C;
/// The gauge's tiles (0x08016824 and 0x08016842: to tile 0x80), its frame
/// (0x08015F1C: 18x2 at column 6, row 0, with "CUSTOM" over it) and the HP
/// box (0x08015EFC: 6x2 where the HUD block's +7 and +8 put it).
const GAUGE_TILES: u32 = 0x0870_A280;
const GAUGE_BYTES: usize = 0x380;
const GAUGE_FIRST_TILE: u16 = 0x80;
const GAUGE_FRAME: u32 = 0x0801_6B44;
const HP_BOX: u32 = 0x0801_6B2C;
/// Background palette 13, the HP box's, by color (normal, healing, hurt or
/// low: 0x08014D98 copies + 0x20 a color); the gauge's palette (9) is the
/// first (0x08015F1C).
const HUD_PALETTES: u32 = 0x0870_8260;
/// The 8x16 font (glyph k = 0x40 bytes), which the chip's name is drawn
/// with (0x08015078) in the HP box's palette.
const FONT: u32 = 0x0868_DF5C;
/// The opponent's HP digits (0x08014EB8: a pointer a digit, 0x08014F9C;
/// one color, the digits of 0x08707580 on), a sprite in palette 14, which
/// the load list's first fills (0x08707800, to 0x03002800: EXE5's
/// 0x086CBA48, EXE6's `byte_86B7AC0`'s place).
const ENEMY_DIGITS: u32 = 0x0870_7580;
const ENEMY_PALETTE: u32 = 0x0870_7800;
/// The chip icons' sprite palette (the load list's to 0x03002780: EXE5's
/// 0x0874AAF8's place) and the hidden chip's icon (the load list's to
/// sprite tile 0x35C).
const ICON_PALETTE: u32 = 0x0874_8038;
const HIDDEN_ICON: u32 = 0x0874_7F78;
/// The emotion window (0x08014B78): picture n < 5 (MegaMan's faces) at
/// FACES + 0x180 n (the 32x16 face, then its 16x16 box), n >= 5 (the
/// version's souls) at SOUL_FACES + 0x100 (n - 5), beside the count box
/// showing c at COUNTS + 0x80 (5 - c); picture n's palette at PALETTES +
/// 0x20 n (sprite palette 12), white while Full Synchro (3) blinks in.
struct FaceAddresses {
    faces: u32,
    soul_faces: u32,
    counts: u32,
    palettes: u32,
}
const RED_SUN_FACES: FaceAddresses =
    FaceAddresses { faces: 0x0870_B160, soul_faces: 0x0870_B8E0, counts: 0x0870_BEE0, palettes: 0x0870_C1E0 };
/// Blue Moon US's (the same routine's literals: its own six souls).
const BLUE_MOON_FACES: FaceAddresses =
    FaceAddresses { faces: 0x0870_AC54, soul_faces: 0x0870_B3D4, counts: 0x0870_B9D4, palettes: 0x0870_BCD4 };
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
const PAUSE: u32 = 0x0870_CB40;
/// The HUD's text lines (EXE6 `TextScript86F0374`'s counterpart).
const TEXTS: u32 = 0x0874_91CC;
/// The banners (0x0801617E: a pointer a banner, by its number,
/// `Addresses::banners`): a record's halfword place (x, then y), its kind
/// byte, then 20 glyph pointers (no filler: EXE4 reads twenty), then a
/// kind-1 banner's number place; the banner font's digits (a pointer a
/// digit, the tenth the blank: a zeroed block) and the banners' palette (to
/// 0x030027A0, sprite palette 11). 41 banners: EXE6's first fifteen in
/// EXE6's places, then EXE4's navis' wins and deletions.
pub(crate) const BANNER_COUNT: u32 = 41;
/// "BUSY..." (7x2 tiles, `Addresses::waiting`: 0x080166F0's list,
/// 0x08016710, to tile 0xAC, with the banners' palette as background
/// palette 10), drawn by 0x08015624 at column 22, row 4 while the other
/// player is still choosing.
pub(crate) const WAITING_BYTES: usize = 0x1C0;
/// The warning marker (0x08008424, EXE6 `sub_800AE90`'s counterpart: a
/// 16x16 sprite at sprite tile 0x360, its second frame 4 tiles on, in
/// palette 13): its two frames and its palette, which the objects that show
/// it load (0x080E3FC8's list).
const WARNING: u32 = 0x0870_B060;
const WARNING_PALETTE: u32 = 0x0870_C3A0;
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
const TELOP_DIGITS: u32 = 0x0801_7A54;
const TELOP_PALETTE: u32 = 0x0875_0E80;
/// The chatbox (0x0804E3B4, `chatbox_runScript`'s counterpart): the box's
/// twenty tiles (to BG0's tile 0x2AC, which its maps count from) and
/// palette, the text's palette; the box's maps (0x0804E384: a kind's four
/// opening steps, 30x8 entries each); the key-wait arrow's three 16x16
/// frames.
const CHATBOX_TILES: u32 = 0x0875_6028;
const CHATBOX_TILE_COUNT: usize = 20;
const CHATBOX_FIRST_TILE: u16 = 0x2AC;
const CHATBOX_PALETTE: u32 = 0x0875_62A8;
const CHATBOX_TEXT_PALETTE: u32 = 0x0869_F4BC;
const CHATBOX_BOXES: u32 = 0x0804_E384;
const CHATBOX_ARROW: u32 = 0x0868_D8DC;
/// Where the chatbox's text starts (0x0804E3B4's +0x18, +0x19: 0x3F, 0x6D;
/// EXE6's 0x33, 0x6C). The portrait's place is EXE6's.
const CHATBOX_TEXT: (u8, u8) = (0x3F, 0x6D);
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

/// Banner `id` (the banner block's number / 4) of the table at `table`.
pub(crate) fn banner(rom: &Rom, table: u32, id: u32) -> BannerLayout {
    let p = rom.u32(table + 4 * id);
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
    BannerLayout { x: place as u8, y: (place >> 8) as u8, kind, glyphs, number_at }
}

/// The banner font's digits of the table at `table` (a pointer a glyph),
/// and the blank as glyph 10.
pub(crate) fn banner_digits(rom: &Rom, table: u32) -> Tiles {
    let mut t = Tiles::default();
    for d in 0..=10 {
        let g = tiles(rom, rom.u32(table + 4 * d), 0x40);
        t.push(g.get(0).unwrap());
        t.push(g.get(1).unwrap());
    }
    t
}

/// An emotion-window face (4x2 tiles) and the box it brings (2x2; none for
/// a soul's, which shows a count).
fn face(rom: &Rom, a: &FaceAddresses, picture: u32) -> (Tiles, Tiles) {
    if picture < OWN_FACES {
        let at = a.faces + 0x180 * picture;
        (tiles(rom, at, 0x100), tiles(rom, at + 0x100, 0x80))
    } else {
        (tiles(rom, a.soul_faces + 0x100 * (picture - OWN_FACES), 0x100), Tiles::default())
    }
}

/// The emotion window's faces, as compat/assets.toml numbers them (its
/// [mugshots]): MegaMan's five and Red Sun's souls in their own palettes
/// (the face table's eleven pictures), then Blue Moon's souls. Each with
/// the box it brings (`Hud::mugshot_boxes`).
fn mugshots(red_sun: &Rom, blue_moon: &Rom) -> (Vec<(Tiles, Palette)>, Vec<Tiles>) {
    let (mut faces, mut boxes) = (Vec::new(), Vec::new());
    let mut push = |rom: &Rom, a: &FaceAddresses, picture: u32| {
        let (t, b) = face(rom, a, picture);
        faces.push((t, palette(rom, a.palettes + 0x20 * picture)));
        boxes.push(b);
    };
    for picture in 0..OWN_FACES + SOULS {
        push(red_sun, &RED_SUN_FACES, picture);
    }
    for picture in OWN_FACES..OWN_FACES + SOULS {
        push(blue_moon, &BLUE_MOON_FACES, picture);
    }
    (faces, boxes)
}

fn chatbox(rom: &Rom) -> Chatbox {
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
    let message: [Vec<MapEntry>; 4] = std::array::from_fn(|step| map(rom.u32(CHATBOX_BOXES + 4 * step as u32)));
    Chatbox {
        tiles: tiles(rom, CHATBOX_TILES, 0x20 * CHATBOX_TILE_COUNT),
        palette: palette(rom, CHATBOX_PALETTE),
        boxes: vec![message.clone(), message],
        arrow: tiles(rom, CHATBOX_ARROW, 3 * 0x80),
        text_palette: palette(rom, CHATBOX_TEXT_PALETTE),
    }
}

/// The telops' look (`TELOP_PLACES`): the digits and '+' by the telop's
/// table (its eleventh pointer the blank, which it draws nothing of), the
/// font's '×' and '2'.
fn telop(rom: &Rom) -> TelopLook {
    let mut glyphs = Tiles::default();
    let mut push = |a: u32| {
        let t = tiles(rom, a, 0x40);
        glyphs.push(t.get(0).unwrap());
        glyphs.push(t.get(1).unwrap());
    };
    for d in (0..10).chain([11]) {
        push(rom.u32(TELOP_DIGITS + 4 * d));
    }
    push(TIMES_GLYPH);
    push(TWO_GLYPH);
    TelopLook { places: TELOP_PLACES, glyphs, palette: palette(rom, TELOP_PALETTE) }
}

/// The HUD's graphics from Red Sun US (`rom`, its addresses `a`; Blue Moon
/// US's souls' faces from `blue_moon`); `names` the fonts' characters;
/// `chip_icons` the chips' (graphics.rs); the fonts and the text lines are
/// the caller's.
pub fn hud(
    rom: &Rom,
    a: &Addresses,
    blue_moon: &Rom,
    names: &AssetNames,
    chip_icons: Vec<ChipIcon>,
    dialogue_font: DialogueFont,
) -> Hud {
    let mut hud_tiles = tiles(rom, HUD_TILES, 0x640);
    for g in [TIMES_GLYPH, TWO_GLYPH] {
        let t = tiles(rom, g, 0x40);
        hud_tiles.push(t.get(0).unwrap());
        hud_tiles.push(t.get(1).unwrap());
    }
    // "PAUSE" as five glyphs: the first four are the 32x16 sprite's
    // columns (its tiles go row by row), the fifth the 8x16 sprite.
    let pause_tiles = tiles(rom, PAUSE, 0x140);
    let mut pause = Tiles::default();
    for (top, bottom) in [(0, 4), (1, 5), (2, 6), (3, 7), (8, 9)] {
        pause.push(pause_tiles.get(top).unwrap());
        pause.push(pause_tiles.get(bottom).unwrap());
    }
    let (mugshots, mugshot_boxes) = mugshots(rom, blue_moon);
    let digits = tiles(rom, ENEMY_DIGITS, 0x40 * 10);
    let encoding = (names.glyphs.as_slice(), names.dialogue_glyphs.as_slice());
    let n = super::graphics::font_glyphs(encoding);
    Hud {
        tiles: hud_tiles,
        first_tile: HUD_FIRST_TILE,
        gauge_tiles: tiles(rom, GAUGE_TILES, GAUGE_BYTES),
        gauge_first_tile: GAUGE_FIRST_TILE,
        hp_palettes: std::array::from_fn(|i| palette(rom, HUD_PALETTES + 0x20 * i as u32)),
        gauge_palette: palette(rom, HUD_PALETTES),
        hp_box: map(rom, HP_BOX, 12),
        gauge_frame: map(rom, GAUGE_FRAME, 36),
        font: tiles(rom, FONT, 0x40 * n),
        font_chars: super::graphics::glyph_names(encoding, n),
        // (One color: EXE4's numbers don't change color as HP drops or rises.)
        enemy_digits: std::array::from_fn(|_| digits.clone()),
        enemy_palette: palette(rom, ENEMY_PALETTE),
        chip_icons,
        hidden_icon: tiles(rom, HIDDEN_ICON, 0x80),
        icon_palette: palette(rom, ICON_PALETTE),
        mugshots,
        // The count boxes: 0..=5.
        counts: (0..=COUNT_MAX).map(|c| tiles(rom, RED_SUN_FACES.counts + 0x80 * (COUNT_MAX - c), 0x80)).collect(),
        // (EXE4 has no box without a count: its faces bring their own.)
        count_box: Tiles::default(),
        mugshot_boxes,
        // (EXE4 has no link navis.)
        navi_mugshots: Vec::new(),
        navi_box: Tiles::default(),
        pause,
        texts: super::graphics::texts(rom, TEXTS, exe4_compat::Compat::exe4().text.first_control),
        banners: (0..BANNER_COUNT).map(|id| banner(rom, a.banners, id)).collect(),
        banner_digits: banner_digits(rom, a.banner_digits),
        banner_palette: palette(rom, a.banner_palette),
        waiting: tiles(rom, a.waiting, WAITING_BYTES),
        waiting_palette: palette(rom, a.banner_palette),
        warning: tiles(rom, WARNING, 0x100),
        warning_palette: palette(rom, WARNING_PALETTE),
        dialogue_font,
        chatbox: chatbox(rom),
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
            telop: Some(telop(rom)),
        },
        language: String::new(),
        languages: Vec::new(),
    }
}
