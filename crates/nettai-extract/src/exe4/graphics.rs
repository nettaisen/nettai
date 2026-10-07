//! EXE4's graphics: the battle sprites, the chips' pictures and icons, the
//! fonts and the HUD's text lines, the field, the backgrounds, in the
//! pack's typed form (nettai-assets); the HUD's and the custom screen's in
//! hud.rs and custom.rs. The routines that read them are EXE6's or EXE5's,
//! the same or nearly (docs/design/exe4-map.md §3), or EXE4's own (§14); the
//! addresses are where their literals point in each ROM (rom.rs).

use crate::decode::{BackgroundDescriptor, background_picture_by, gfx_anims, tiles};
use crate::exe4::rom::{Addresses, RED_SUN, Rom, Roms, Version};
use nettai_assets::*;
use nettai_content::names::AssetNames;

/// The language the Japanese ROMs' lettering is.
pub const LANGUAGE: &str = "ja";

/// The categories of the sprite list with battle sprites (byte offsets
/// 0x00..=0x14, as EXE5's and EXE6's).
const BATTLE_CATEGORIES: usize = 6;

/// A chip's picture: 7x6 tiles.
const PICTURE_BYTES: usize = 0x540;
/// A chip's icon: 2x2 tiles.
const ICON_BYTES: usize = 0x80;

/// The 8x16 font's glyphs (0x40 bytes each; what follows them is other
/// data). The US fonts draw the first 0x70; the Japanese fonts their
/// encoding's first page and the second page up to here.
const FONT_GLYPHS: usize = 0x1A0;
/// The dialogue font's glyphs (16x12, 0x60 bytes each).
const DIALOGUE_GLYPHS: usize = 0x1C0;
/// The HUD's text lines end in 0xE5 (EXE5's and EXE6's 0xE6).
const TEXT_END: u8 = 0xE5;

/// The version chips (compat/chips.toml's `version`): a version's own giga
/// chip, its art from its own version's ROM.
fn version_of(id: u16) -> Option<Version> {
    exe4_compat::Compat::exe4().chip_key(id).and_then(|k| exe4_compat::Compat::exe4().chip_entry(k)).and_then(|e| e.version).map(
        |v| match v {
            exe4_compat::Version::RedSun => Version::RedSun,
            exe4_compat::Version::BlueMoon => Version::BlueMoon,
        },
    )
}

/// Everything the pack draws with that EXE4's ROMs give so far: `names`
/// gives the chips their keys and the fonts their characters.
pub fn bundle(roms: &Roms, names: &AssetNames) -> Bundle {
    let mut sprites = roms.any().map(|(rom, a)| sprites(rom, a.sprite_list)).unwrap_or_default();
    sprites.extend(roms.any().map(|(rom, a)| portraits(rom, a.sprite_list, names)).unwrap_or_default());
    sprites.sort_by_key(|s| (s.category, s.index));
    // The HUD's art is Red Sun US's (exe4/hud.rs); without it the fonts and
    // text lines of a US ROM present, the rest placeholders.
    let mut hud = if roms.redsun.is_present() {
        let encoding = (names.glyphs.as_slice(), names.dialogue_glyphs.as_slice());
        super::hud::hud(roms.redsun, &RED_SUN, roms.bluemoon, names, chip_icons(roms, names), dialogue_font(roms.redsun, &RED_SUN, encoding))
    } else {
        let mut hud = match roms.us() {
            Some((rom, a)) => hud(rom, a, names),
            None => crate::placeholders::hud(names, super::hud::BANNER_COUNT as usize),
        };
        hud.chip_icons = chip_icons(roms, names);
        if let Some((rom, a)) = roms.any() {
            hud.icon_palette = palette(rom, rom.u32(a.icon_palette_pointer));
        }
        hud
    };
    let lettering = match roms.jp() {
        Some((rom, a)) => lettering(rom, a, &hud, names),
        None => crate::placeholders::lettering(&hud, names, LANGUAGE),
    };
    hud.languages.push((LANGUAGE.to_string(), lettering));
    hud.language = BASE_LANGUAGE.into();
    // The custom screen (exe4/custom.rs), Red Sun US's; its Japanese
    // lettering a Japanese ROM's.
    let mut custom = super::custom::custom(roms, chip_art(roms, names));
    let lettering = super::custom::lettering(roms, &custom);
    custom.languages.push((LANGUAGE.to_string(), lettering));
    // The backgrounds are the same in the four ROMs but for where they are.
    let backgrounds = roms.any().map(|(rom, a)| backgrounds(rom, a)).unwrap_or_default();
    // The field, the HUD's and the custom screen's art are Red Sun US's (the
    // base US ROM's, as the other games' packs take theirs).
    let field = if roms.redsun.is_present() { field(roms.redsun) } else { Field::default() };
    Bundle { sprites, field, backgrounds, hud, custom }
}

// ---- Field ------------------------------------------------------------------------

/// The field's tiles (the field's load, 0x08006A40: decompressed to VRAM
/// 0x06001460, as EXE5's and EXE6's) and background palettes 1..=8 (its
/// transfer list, 0x08006A68, to the palette buffer's 0x03002A70).
const FIELD_TILES: u32 = 0x0870_4660;
const FIELD_PALETTES: u32 = 0x0870_73C0;
/// Panel blocks: 32 bytes (5x3 map entries) per 6 * type + 3 * owner + row
/// - 1 (0x080093FC, EXE6 `sub_800C01C`'s counterpart), for EXE4's 12 panel
/// types (0x0800A3A8's flag words).
const PANEL_BLOCKS: u32 = 0x0870_6640;
const PANEL_TYPES: u8 = 12;

/// EXE4's panel types' names by number (compat/panels.toml).
pub fn panel_names() -> Vec<String> {
    let c = exe4_compat::Compat::exe4();
    (0..PANEL_TYPES).map(|n| c.panel_name(n).unwrap_or_else(|e| panic!("EXE4 panel {n}: {e}")).to_string()).collect()
}
/// The highlighted panel block (0x0800948A, EXE6 `sub_800C0BA`'s): one,
/// which EXE4 draws for both highlights (it takes no highlight number).
const HIGHLIGHT_BLOCK: u32 = 0x0870_6F40;
/// The front edges by owner (0x080094C4: + 32 an owner).
const FRONT_EDGES: u32 = 0x0870_6F60;
/// The palette buffer's background palette 0 (the field's transfer list's
/// palette 1 is at + 0x20).
const FIELD_PALETTE_BUFFER: u32 = 0x0300_2A50;
/// The panel palettes that cycle (0x08009556, called as the field is drawn:
/// EXE6 `sub_800C192`'s counterpart, written out a palette at a time rather
/// than from a list): each a counter pair at 0x02037AB0 + 2k (its frame,
/// then a timer from 14, 13, 12, 11, 10 and 9, 0x08009120), a frame every 14
/// ticks, from a table of palette pointers (its first the palette the field
/// loads there) to a palette of the buffer.
const PANEL_PALETTE_ANIMS: [(u32, usize, u32); 6] = [
    (0x0800_9648, 10, 0x0300_2A70),
    (0x0800_9678, 10, 0x0300_2AF0),
    (0x0800_96A8, 6, 0x0300_2AB0),
    (0x0800_96C8, 6, 0x0300_2B30),
    (0x0800_96E8, 10, 0x0300_2AD0),
    (0x0800_9718, 10, 0x0300_2B50),
];
const PANEL_PALETTE_TICKS: u8 = 14;

/// The field from Red Sun US: its panel blocks by EXE4's numbers, each type
/// by the name content gives that number (compat/panels.toml).
fn field(rom: &Rom) -> Field {
    let panel_types = panel_names();
    let palette_anims = PANEL_PALETTE_ANIMS
        .iter()
        .enumerate()
        .map(|(k, &(table, count, dest))| PaletteAnim {
            slot: ((dest - FIELD_PALETTE_BUFFER) / 32) as u8,
            frames: (0..count as u32).map(|i| (palette(rom, rom.u32(table + 4 * i)), PANEL_PALETTE_TICKS)).collect(),
            initial_timer: PANEL_PALETTE_TICKS - k as u8,
        })
        .collect();
    crate::decode::field_with(
        rom,
        crate::decode::FieldAddresses {
            tiles: FIELD_TILES,
            palettes: FIELD_PALETTES,
            panels: PANEL_BLOCKS,
            highlights: [HIGHLIGHT_BLOCK; 2],
            edges: FRONT_EDGES,
            palette_anims: 0,
            palette_buffer: FIELD_PALETTE_BUFFER,
        },
        panel_types,
        palette_anims,
    )
}

/// A palette (the hardware ignores bit 15 of a color).
fn palette(rom: &Rom, a: u32) -> Palette {
    palettes_from_bytes(rom.bytes(a, 32))[0].map(|c| c & 0x7FFF)
}

// ---- Backgrounds --------------------------------------------------------------------

/// EXE4's battle backgrounds (0 to 26: the loader 0x08085430 takes the game
/// state's +0x0F, else the battle settings' +5, else the map's from
/// 0x08085BAC).
const BACKGROUND_COUNT: u32 = 27;
/// EXE4's descriptor (0x08026234, 0x08026266): the tiles' archive, its
/// buffer, their VRAM destination, the map's archive, its buffer and
/// destination, the palette's source.
const DESCRIPTOR: BackgroundDescriptor = BackgroundDescriptor { gfx: 0, dest: 8, map: 0x0C, palette: 0x18 };
/// The palette buffer's background palette 0 (the descriptors' palette
/// destination; EXE5's 0x03003960).
const PALETTE_BUFFER: u32 = 0x0300_2A50;
/// The BG1 scroll callbacks by their offset from the first
/// (`Addresses::scrollers`), and their counters' steps (1/16 pixel a frame):
/// EXE6's `BGScrollCB_*` and EXE5's in EXE4's order. +0xB8 (Red Sun US's
/// 0x08001F88) speeds a left scroll up by 1/16 pixel a frame each frame to 4
/// pixels a frame: drawn at that speed from the start (background 0x17; the
/// pack's scroll is a speed).
const SCROLLERS: [(u32, (i32, i32)); 7] = [
    (0x00, (0, 0)),   // returns (EXE6 nullsub_35)
    (0x02, (-8, -4)), // EXE6 BGScrollCB_BG1Diagonal3to2Scroll
    (0x4C, (0, -4)),  // BGScrollCB_BG1UpScroll
    (0x5E, (0, 4)),   // BGScrollCB_BG1DownScroll
    (0x94, (-8, 0)),  // BGScrollCB_BG1FastLeftScroll
    (0xB8, (-64, 0)), // the speeding left scroll, at its top speed
    (0xE4, (0, 0)),   // returns (EXE5's 0x08001A24)
];

fn backgrounds(rom: &Rom, a: &Addresses) -> Vec<Option<Background>> {
    (0..BACKGROUND_COUNT)
        .map(|id| {
            let (tiles, first_tile, map, map_width, map_height, palette) =
                background_picture_by(rom, rom.u32(a.backgrounds + 4 * id), DESCRIPTOR)?;
            let cb = (rom.u32(a.background_scroll + 16 * id + 4) & !1).wrapping_sub(a.scrollers);
            let scroll = SCROLLERS.iter().find(|(o, _)| *o == cb).map(|(_, v)| *v).unwrap_or((0, 0));
            let anims = gfx_anims(rom, rom.u32(a.background_anims + 4 * id), PALETTE_BUFFER);
            Some(Background { tiles, first_tile, map, map_width, map_height, palette, scroll, speeding: None, anims })
        })
        .collect()
}

// ---- Sprites ------------------------------------------------------------------------

/// The battle sprites of the sprite list at `list` (the four ROMs have the
/// same ones), by (category, index).
fn sprites(rom: &Rom, list: u32) -> Vec<SpriteSheet> {
    let cats: Vec<u32> = (0..10).map(|i| rom.u32(list + 4 * i)).collect();
    let mut out = Vec::new();
    for (ci, &c) in cats.iter().enumerate().take(BATTLE_CATEGORIES) {
        let next = cats.iter().copied().filter(|&s| s > c).min().unwrap_or(c + 0x400);
        for idx in 0..((next - c) / 4).min(256) {
            if let Some(data) = crate::sprite::archive(rom, rom.u32(c + 4 * idx))
                && let Some(sheet) = crate::sprite::sheet(&data, (ci * 4) as u8, idx as u8)
            {
                out.push(sheet);
            }
        }
    }
    out.sort_by_key(|s| (s.category, s.index));
    out
}

/// The portraits' category (the sprite list's byte offset 0x20: the
/// mugshot table 0x08028038, which 0x080028F2 loads from, EXE6's
/// `mugshotSpritePtrs`).
const PORTRAITS: u8 = 0x20;

/// The portraits content names (the chatbox's speakers: a no-running
/// message's `F4 00 n`, 0x0804F750). The four ROMs have the same pictures:
/// any one's.
fn portraits(rom: &Rom, list: u32, names: &AssetNames) -> Vec<SpriteSheet> {
    let table = rom.u32(list + PORTRAITS as u32);
    names
        .sprites
        .keys()
        .filter(|(c, _)| *c == PORTRAITS)
        .filter_map(|&(category, index)| {
            let data = crate::sprite::archive(rom, rom.u32(table + 4 * index as u32))?;
            crate::sprite::portrait_sheet(&data, category, index)
        })
        .collect()
}

// ---- The HUD's lettering ------------------------------------------------------------

/// What glyph `k` of a font draws: the encoding's name for it, else its
/// number in brackets.
pub(super) fn glyph_names((cell, dialogue): (&[String], &[String]), n: usize) -> Vec<String> {
    (0..n).map(|k| cell.iter().chain(dialogue).nth(k).cloned().unwrap_or_else(|| format!("[{k:03x}]"))).collect()
}

/// The dialogue font at `a`, its glyphs drawing the encoding's characters.
fn dialogue_font(rom: &Rom, a: &Addresses, encoding: (&[String], &[String])) -> DialogueFont {
    DialogueFont {
        pixels: rom.bytes(a.dialogue_font, 0x60 * DIALOGUE_GLYPHS).iter().flat_map(|&b| [b & 15, b >> 4]).collect(),
        advances: (0..DIALOGUE_GLYPHS as u32).map(|i| rom.u32(a.dialogue_advances + 4 * i) as u8).collect(),
        chars: glyph_names(encoding, DIALOGUE_GLYPHS),
    }
}

/// The HUD's text lines at `at`: a line with anything but glyphs in it (a
/// text command) is cut there.
pub(super) fn texts(rom: &Rom, at: u32, first_control: u8) -> Vec<Vec<u16>> {
    let offset = |i: u32| rom.u16(at + 2 * i) as u32;
    (0..offset(0) / 2)
        .map(|i| {
            rom.bytes(at + offset(i), 0x40)
                .iter()
                .take_while(|&&c| c != TEXT_END && c < first_control)
                .map(|&c| c as u16)
                .collect()
        })
        .collect()
}

/// The 8x16 font's glyphs an encoding names (no more than the font has).
pub(super) fn font_glyphs(encoding: (&[String], &[String])) -> usize {
    (encoding.0.len() + encoding.1.len()).min(FONT_GLYPHS)
}

/// The HUD from a US ROM: its fonts and text lines (the rest, EXE4's own,
/// placeholders).
fn hud(rom: &Rom, a: &Addresses, names: &AssetNames) -> Hud {
    let encoding = (names.glyphs.as_slice(), names.dialogue_glyphs.as_slice());
    let n = font_glyphs(encoding);
    let first_control = exe4_compat::Compat::exe4().text.first_control;
    Hud {
        font: tiles(rom, a.font, 0x40 * n),
        font_chars: glyph_names(encoding, n),
        dialogue_font: dialogue_font(rom, a, encoding),
        texts: texts(rom, a.texts, first_control),
        ..crate::placeholders::hud(names, 0)
    }
}

/// The Japanese lettering from a Japanese ROM: its fonts and text lines.
fn lettering(rom: &Rom, a: &Addresses, base: &Hud, names: &AssetNames) -> HudLettering {
    let (cell, dialogue) = names.language_glyphs.get(LANGUAGE).cloned().unwrap_or_default();
    let encoding = (cell.as_slice(), dialogue.as_slice());
    let n = font_glyphs(encoding);
    let first_control = exe4_compat::Compat::exe4().text.first_control;
    HudLettering {
        font: tiles(rom, a.font, 0x40 * n),
        font_chars: glyph_names(encoding, n),
        dialogue_font: dialogue_font(rom, a, encoding),
        texts: texts(rom, a.texts, first_control),
        // The banners whose words differ, each where it starts (a longer
        // name further left); not what kind of banner it is.
        banners: base
            .banners
            .iter()
            .enumerate()
            .map(|(id, own)| {
                let b = super::hud::banner(rom, a.banners, id as u32);
                (b != *own).then_some(b)
            })
            .collect(),
        banner_palette: palette(rom, a.banner_palette),
        // ("カスタム中…", seven tiles wide as the US's "BUSY...".)
        waiting: tiles(rom, a.waiting, super::hud::WAITING_BYTES),
        waiting_palette: palette(rom, a.banner_palette),
        // (The gauge's tiles, "CUSTOM" among them, are the US ROMs'.)
        gauge_tiles: base.gauge_tiles.clone(),
        ..crate::placeholders::lettering(base, names, LANGUAGE)
    }
}

// ---- Chips -----------------------------------------------------------------------------

/// A chip's picture and icon, from its version's ROM (a version's own chip)
/// or Red Sun US's (any other; else whichever ROM is present).
fn chip_media(roms: &Roms, id: u16) -> (Picture, Tiles) {
    let source = match version_of(id) {
        Some(v) => roms.of(v),
        None => roms.any(),
    };
    let Some((rom, a)) = source else { return (Picture::default(), Tiles::default()) };
    let r = a.chip_table + 0x2C * id as u32;
    let (icon, gfx, pal) = (rom.u32(r + 0x20), rom.u32(r + 0x24), rom.u32(r + 0x28));
    let picture = if rom.contains(gfx) {
        Picture { tiles: tiles(rom, gfx, PICTURE_BYTES), palette: if rom.contains(pal) { palette(rom, pal) } else { Default::default() } }
    } else {
        Picture::default()
    };
    let icon = if rom.contains(icon) { tiles(rom, icon, ICON_BYTES) } else { Tiles::default() };
    (picture, icon)
}

/// The chips' pictures, each under its chip's key; a version's own chip's
/// marked with its version.
fn chip_art(roms: &Roms, names: &AssetNames) -> Vec<ChipArt> {
    names
        .chips
        .keys()
        .map(|&id| ChipArt { key: names.chip_icon(id), picture: chip_media(roms, id).0, version: version_of(id).map(|v| v.name().into()) })
        .collect()
}

/// The chips' icons, each under its chip's key.
fn chip_icons(roms: &Roms, names: &AssetNames) -> Vec<ChipIcon> {
    names.chips.keys().map(|&id| ChipIcon { key: names.chip_icon(id), tiles: chip_media(roms, id).1 }).collect()
}
