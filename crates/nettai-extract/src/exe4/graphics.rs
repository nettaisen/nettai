//! EXE4's graphics: the battle sprites, the chips' pictures and icons, the
//! fonts and the HUD's text lines, the field, the backgrounds, in the
//! pack's typed form (nettai-assets); the HUD's and the custom screen's in
//! hud.rs and custom.rs. The routines that read them are EXE6's or EXE5's,
//! the same or nearly (docs/design/exe4-map.md §3), or EXE4's own (§14);
//! each block is stated by where the game's own pointer to it is in each of
//! the four ROMs (`Place`: those routines' literals; the addresses in the
//! comments are Red Sun US's), so any ROM gives what it holds: what the
//! four have alike from whichever is there, each release's words from a
//! ROM of that release, a version's own chips from a ROM of that version.

use crate::decode::{BackgroundDescriptor, background_picture_by, gfx_anims, speeding_scroll, tiles};
use crate::exe4::rom::{Place, Rom, Roms, Source, Version};
use nettai_assets::*;
use exe4_compat::Compat;
use nettai_content::names::AssetNames;

/// The language the Japanese ROMs' lettering is.
pub const LANGUAGE: &str = "ja";

/// The sprite list (EXE6 `SpritePointersList`; Red Sun US's 0x0802793C):
/// per category, a table of sprite archive pointers.
const SPRITE_LIST: Place = Place::pointer(0x0800_28BC, 0x0800_289C);
/// The categories of the sprite list with battle sprites (byte offsets
/// 0x00..=0x14, as EXE5's and EXE6's).
const BATTLE_CATEGORIES: usize = 6;

/// The chip records (0x2C bytes: +0x20 the icon, +0x24 the picture, +0x28
/// its palette; the chip accessor's literal, Red Sun US's table 0x080197EC).
const CHIPS: Place = Place::pointer(0x0801_90DC, 0x0801_901C);

/// A chip's picture: 7x6 tiles.
const PICTURE_BYTES: usize = 0x540;
/// A chip's icon: 2x2 tiles.
const ICON_BYTES: usize = 0x80;

/// The 8x16 font (0x40 bytes a glyph; what follows its glyphs is other
/// data; Red Sun US's 0x0868DF5C), which the chip's name is drawn with
/// (0x08015078) in the HP box's palette. The US fonts draw the first 0x70;
/// the Japanese fonts their encoding's first page and the second page up to
/// `FONT_GLYPHS`.
const FONT: Place = Place::pointer(0x0801_51F4, 0x0801_5190);
const FONT_GLYPHS: usize = 0x1A0;
/// The dialogue font (16x12 glyphs, 0x60 bytes each; 0x08694F5C) and its
/// advances (a word a glyph; 0x080515E0): the chatbox's literals, at an
/// address of their own in each ROM.
const DIALOGUE_FONT: Place = Place::pointer_in_each([0x0803_1BEC, 0x0803_1BF0, 0x0803_1B00, 0x0803_1B04]);
const DIALOGUE_ADVANCES: Place = Place::pointer_in_each([0x0804_E294, 0x0804_E29C, 0x0804_E184, 0x0804_E18C]);
const DIALOGUE_GLYPHS: usize = 0x1C0;
/// The HUD's text lines (EXE6 `TextScript86F0374`'s counterpart;
/// 0x087491CC), which end in 0xE5 (EXE5's and EXE6's 0xE6).
const TEXTS: Place = Place::pointer(0x0801_543C, 0x0801_53D8);
const TEXT_END: u8 = 0xE5;

/// The version chips (compat/chips.toml's `version`): a version's own giga
/// chip, its art from its own version's ROM.
fn version_of(c: &Compat, id: u16) -> Option<Version> {
    c.chip_key(id).and_then(|k| c.chip_entry(k)).and_then(|e| e.version).map(
        |v| match v {
            exe4_compat::Version::RedSun => Version::RedSun,
            exe4_compat::Version::BlueMoon => Version::BlueMoon,
        },
    )
}

/// Everything the pack draws with that EXE4's ROMs give: `names` gives the
/// chips their keys and the fonts their characters, and the content's
/// compat `c` the chips' versions, the text's controls and the panels.
pub fn bundle(roms: &Roms, names: &AssetNames, c: &Compat) -> Bundle {
    // What the four ROMs have alike (the sprites, the backgrounds, the
    // field and most of the HUD and the custom screen) is whichever ROM's
    // is there.
    let art = roms.any();
    let mut sprites = if art.is_present() { sprites(art) } else { Vec::new() };
    if art.is_present() {
        sprites.extend(portraits(art, names));
    }
    sprites.sort_by_key(|s| (s.category, s.index));
    let mut hud = super::hud::hud(roms, names, c, chip_icons(roms, names, c));
    // (The Japanese games place the chatbox's text themselves: code, the
    // same with or without their ROMs.)
    let lettering = HudLettering { chatbox_text: Some(super::hud::CHATBOX_TEXT_JA), ..lettering(roms, &hud, names, c) };
    hud.languages.push((LANGUAGE.to_string(), lettering));
    hud.language = BASE_LANGUAGE.into();
    let mut custom = super::custom::custom(roms, chip_art(roms, names, c));
    let lettering = super::custom::lettering(roms, &custom);
    custom.languages.push((LANGUAGE.to_string(), lettering));
    let backgrounds = if art.is_present() { backgrounds(art) } else { Vec::new() };
    let field = if art.is_present() { field(art, c) } else { Field::default() };
    Bundle { sprites, field, backgrounds, hud, custom }
}

// ---- Field ------------------------------------------------------------------------

/// The field's tiles (the field's load, 0x08006A40: decompressed to VRAM
/// 0x06001460, as EXE5's and EXE6's) and background palettes 1..=8 (its
/// transfer list, 0x08006A68, to the palette buffer's 0x03002A70).
const FIELD_TILES: Place = Place::pointer(0x0800_6A94, 0x0800_6A74);
const FIELD_PALETTES: Place = Place::pointer(0x0800_6A68, 0x0800_6A48);
/// Panel blocks: 32 bytes (5x3 map entries) per 6 * type + 3 * owner + row
/// - 1 (0x080093FC, EXE6 `sub_800C01C`'s counterpart), for EXE4's 12 panel
/// types (0x0800A3A8's flag words).
const PANEL_BLOCKS: Place = Place::pointer(0x0800_944C, 0x0800_9418);
const PANEL_TYPES: u8 = 12;

/// EXE4's panel types' names by number (compat/panels.toml).
pub fn panel_names(c: &Compat) -> Vec<String> {
    (0..PANEL_TYPES).map(|n| c.panel_name(n).unwrap_or_else(|e| panic!("EXE4 panel {n}: {e}")).to_string()).collect()
}
/// The highlighted panel block (0x0800948A, EXE6 `sub_800C0BA`'s): one,
/// which EXE4 draws for both highlights (it takes no highlight number).
const HIGHLIGHT_BLOCK: Place = Place::pointer(0x0800_94BC, 0x0800_9488);
/// The front edges by owner (0x080094C4: + 32 an owner).
const FRONT_EDGES: Place = Place::pointer(0x0800_9530, 0x0800_94FC);
/// The palette buffer's background palette 0 (the field's transfer list's
/// palette 1 is at + 0x20).
const FIELD_PALETTE_BUFFER: u32 = 0x0300_2A50;
/// The panel palettes that cycle (0x08009556, called as the field is drawn:
/// EXE6 `sub_800C192`'s counterpart, written out a palette at a time rather
/// than from a list): each a counter pair at 0x02037AB0 + 2k (its frame,
/// then a timer from 14, 13, 12, 11, 10 and 9, 0x08009120), a frame every 14
/// ticks, from a table of palette pointers (its first the palette the field
/// loads there; Red Sun US's tables 0x08009648 on) to a palette of the
/// buffer.
const PANEL_PALETTE_ANIMS: [(Place, usize, u32); 6] = [
    (Place::pointer(0x0800_9644, 0x0800_9610), 10, 0x0300_2A70),
    (Place::pointer(0x0800_9674, 0x0800_9640), 10, 0x0300_2AF0),
    (Place::pointer(0x0800_96A4, 0x0800_9670), 6, 0x0300_2AB0),
    (Place::pointer(0x0800_96C4, 0x0800_9690), 6, 0x0300_2B30),
    (Place::pointer(0x0800_96E4, 0x0800_96B0), 10, 0x0300_2AD0),
    (Place::pointer(0x0800_9714, 0x0800_96E0), 10, 0x0300_2B50),
];
const PANEL_PALETTE_TICKS: u8 = 14;

/// The field: its panel blocks by EXE4's numbers, each type by the name
/// content gives that number (compat/panels.toml).
fn field(src: Source, c: &Compat) -> Field {
    let rom = src.rom;
    let panel_types = panel_names(c);
    let palette_anims = PANEL_PALETTE_ANIMS
        .iter()
        .enumerate()
        .map(|(k, &(table, count, dest))| PaletteAnim {
            slot: ((dest - FIELD_PALETTE_BUFFER) / 32) as u8,
            frames: (0..count as u32).map(|i| (palette(rom, rom.u32(src.at(table) + 4 * i)), PANEL_PALETTE_TICKS)).collect(),
            initial_timer: PANEL_PALETTE_TICKS - k as u8,
        })
        .collect();
    crate::decode::field_with(
        rom,
        crate::decode::FieldAddresses {
            tiles: src.at(FIELD_TILES),
            palettes: src.at(FIELD_PALETTES),
            panels: src.at(PANEL_BLOCKS),
            highlights: [src.at(HIGHLIGHT_BLOCK); 2],
            edges: src.at(FRONT_EDGES),
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
/// 0x08085BAC): their load descriptors by background id (the loader's
/// literal; Red Sun US's 0x0808563C), their scroll records (16 bytes each:
/// two callbacks, the second BG1's, an argument and display bits;
/// 0x08085488) and their GFX animation lists (0x08085A50; EXE5's
/// 0x0808C578, 0x0808C32C, 0x0808C96C). The same backgrounds in the four
/// ROMs, the literals at an address of their own in each.
const BACKGROUNDS: Place = Place::pointer_in_each([0x0808_5638, 0x0808_5644, 0x0808_5684, 0x0808_5690]);
const BACKGROUND_SCROLL: Place = Place::pointer_in_each([0x0808_5484, 0x0808_5490, 0x0808_54D0, 0x0808_54DC]);
const BACKGROUND_ANIMS: Place = Place::pointer_in_each([0x0808_5480, 0x0808_548C, 0x0808_54CC, 0x0808_54D8]);
const BACKGROUND_COUNT: u32 = 27;
/// The first of the scroll callbacks (a routine that returns; the ones
/// after it at the same offsets in the four ROMs): code, which the scroll
/// records point into, so its own address is stated.
const SCROLLERS: Place = Place::stated([0x0800_1ED0, 0x0800_1ED0, 0x0800_1EB0, 0x0800_1EB0]);
/// EXE4's descriptor (0x08026234, 0x08026266): the tiles' archive, its
/// buffer, their VRAM destination, the map's archive, its buffer and
/// destination, the palette's source.
const DESCRIPTOR: BackgroundDescriptor = BackgroundDescriptor { gfx: 0, dest: 8, map: 0x0C, palette: 0x18 };
/// The palette buffer's background palette 0 (the descriptors' palette
/// destination; EXE5's 0x03003960).
const PALETTE_BUFFER: u32 = 0x0300_2A50;
/// The BG1 scroll callbacks by their offset from the first
/// (`SCROLLERS`), and their counters' steps (1/16 pixel a frame):
/// EXE6's `BGScrollCB_*` and EXE5's in EXE4's order. +0xB8 (Red Sun US's
/// 0x08001F88, background 0x17's) speeds a left scroll up from the
/// background's load, by 1/16 pixel a frame each frame to 4 pixels a frame:
/// a `speeding` scroll, read from its code (`speeding_scroll`).
const SCROLLER_STEPS: [(u32, (i32, i32)); 6] = [
    (0x00, (0, 0)),   // returns (EXE6 nullsub_35)
    (0x02, (-8, -4)), // EXE6 BGScrollCB_BG1Diagonal3to2Scroll
    (0x4C, (0, -4)),  // BGScrollCB_BG1UpScroll
    (0x5E, (0, 4)),   // BGScrollCB_BG1DownScroll
    (0x94, (-8, 0)),  // BGScrollCB_BG1FastLeftScroll
    (0xE4, (0, 0)),   // returns (EXE5's 0x08001A24)
];

fn backgrounds(src: Source) -> Vec<Option<Background>> {
    let rom = src.rom;
    let (loads, scrolls, anim_lists, scrollers) = (src.at(BACKGROUNDS), src.at(BACKGROUND_SCROLL), src.at(BACKGROUND_ANIMS), src.at(SCROLLERS));
    (0..BACKGROUND_COUNT)
        .map(|id| {
            let (tiles, first_tile, map, map_width, map_height, palette) =
                background_picture_by(rom, rom.u32(loads + 4 * id), DESCRIPTOR)?;
            let callback = rom.u32(scrolls + 16 * id + 4) & !1;
            let cb = callback.wrapping_sub(scrollers);
            let scroll = SCROLLER_STEPS.iter().find(|(o, _)| *o == cb).map(|(_, v)| *v).unwrap_or((0, 0));
            let speeding = speeding_scroll(rom, callback);
            let anims = gfx_anims(rom, rom.u32(anim_lists + 4 * id), PALETTE_BUFFER);
            Some(Background { tiles, first_tile, map, map_width, map_height, palette, scroll, speeding, anims })
        })
        .collect()
}

// ---- Sprites ------------------------------------------------------------------------

/// The battle sprites of a ROM's sprite list (the four ROMs have the same
/// ones), by (category, index).
fn sprites(src: Source) -> Vec<SpriteSheet> {
    let (rom, list) = (src.rom, src.at(SPRITE_LIST));
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
fn portraits(src: Source, names: &AssetNames) -> Vec<SpriteSheet> {
    let rom = src.rom;
    let table = rom.u32(src.at(SPRITE_LIST) + PORTRAITS as u32);
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

/// A source's dialogue font, its glyphs drawing the encoding's characters.
pub(super) fn dialogue_font(src: Source, encoding: (&[String], &[String])) -> DialogueFont {
    let (rom, font, advances) = (src.rom, src.at(DIALOGUE_FONT), src.at(DIALOGUE_ADVANCES));
    DialogueFont {
        pixels: rom.bytes(font, 0x60 * DIALOGUE_GLYPHS).iter().flat_map(|&b| [b & 15, b >> 4]).collect(),
        advances: (0..DIALOGUE_GLYPHS as u32).map(|i| rom.u32(advances + 4 * i) as u8).collect(),
        chars: glyph_names(encoding, DIALOGUE_GLYPHS),
    }
}

/// A source's HUD text lines: a line with anything but glyphs in it (a
/// text command) is cut there.
pub(super) fn texts(src: Source, c: &Compat) -> Vec<Vec<u16>> {
    let (rom, at) = (src.rom, src.at(TEXTS));
    let first_control = c.text.first_control;
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
fn font_glyphs(encoding: (&[String], &[String])) -> usize {
    (encoding.0.len() + encoding.1.len()).min(FONT_GLYPHS)
}

/// A source's 8x16 font: the glyphs its release's encoding names, with
/// their characters.
pub(super) fn font(src: Source, encoding: (&[String], &[String])) -> (Tiles, Vec<String>) {
    let n = font_glyphs(encoding);
    (tiles(src.rom, src.at(FONT), 0x40 * n), glyph_names(encoding, n))
}

/// An encoding's 8x16 font with no ROM to read it from: a placeholder glyph
/// for every character the ROM's font would have (`glyph_names`, the cell
/// font's and what it shares of the dialogue font's), so a pack of fewer
/// ROMs has a glyph for the same names a full one has.
pub(super) fn placeholder_font(encoding: (&[String], &[String])) -> (Tiles, Vec<String>) {
    let n = font_glyphs(encoding);
    (crate::placeholders::tiles(2 * n), glyph_names(encoding, n))
}

/// The HUD's Japanese lettering, from a Japanese ROM: its fonts, its text
/// lines, its banners and "BUSY...". Placeholders without one, but what has
/// no words and is the pack's own (`base`): the gauge's tiles ("CUSTOM" in
/// every ROM) and the banners' palette.
fn lettering(roms: &Roms, base: &Hud, names: &AssetNames, c: &Compat) -> HudLettering {
    let (cell, dialogue) = names.language_glyphs.get(LANGUAGE).cloned().unwrap_or_default();
    let encoding = (cell.as_slice(), dialogue.as_slice());
    let own = |l: HudLettering| HudLettering {
        gauge_tiles: base.gauge_tiles.clone(),
        banner_palette: base.banner_palette,
        waiting_palette: base.waiting_palette,
        ..l
    };
    let jp = roms.jp();
    if !jp.is_present() {
        let (font, font_chars) = placeholder_font(encoding);
        return own(HudLettering { font, font_chars, ..crate::placeholders::lettering(base, names, LANGUAGE) });
    }
    let (font, font_chars) = font(jp, encoding);
    own(HudLettering {
        font,
        font_chars,
        dialogue_font: dialogue_font(jp, encoding),
        texts: texts(jp, c),
        // The banners whose words differ, each where it starts (a longer
        // name further left); not what kind of banner it is.
        banners: base
            .banners
            .iter()
            .enumerate()
            .map(|(id, own)| {
                let b = super::hud::banner(jp, id as u32);
                (b != *own).then_some(b)
            })
            .collect(),
        // ("カスタム中…", seven tiles wide as the US's "BUSY...".)
        waiting: super::hud::waiting(jp),
        ..crate::placeholders::lettering(base, names, LANGUAGE)
    })
}

// ---- Chips -----------------------------------------------------------------------------

/// A chip's picture and icon. Every ROM has every chip's, a version's own
/// chips' too: a ROM of the chip's version is read where one is given
/// (any other chip: a Red Sun ROM, as four chips' icons aren't the same in
/// a Blue Moon ROM and the pack has Red Sun's), else whichever ROM is
/// there. An e-Reader card's chip's record points at RAM for them: its
/// icon is the ROM's all the same, its picture the extractor's own
/// (cards.rs).
fn chip_media(roms: &Roms, c: &Compat, id: u16) -> (Picture, Tiles) {
    let src = roms.preferring(version_of(c, id).unwrap_or(Version::RedSun));
    if !src.is_present() {
        return (Picture::default(), Tiles::default());
    }
    let (rom, table) = (src.rom, src.at(CHIPS));
    if let Some(media) = super::cards::media(rom, table, id, ICON_BYTES) {
        return media;
    }
    let r = table + 0x2C * id as u32;
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
fn chip_art(roms: &Roms, names: &AssetNames, c: &Compat) -> Vec<ChipArt> {
    names
        .chips
        .keys()
        .map(|&id| ChipArt { key: names.chip_icon(id), picture: chip_media(roms, c, id).0, version: version_of(c, id).map(|v| v.name().into()) })
        .collect()
}

/// The chips' icons, each under its chip's key.
fn chip_icons(roms: &Roms, names: &AssetNames, c: &Compat) -> Vec<ChipIcon> {
    names.chips.keys().map(|&id| ChipIcon { key: names.chip_icon(id), tiles: chip_media(roms, c, id).1 }).collect()
}
