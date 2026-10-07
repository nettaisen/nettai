//! EXE4's graphics: the battle sprites, the chips' pictures and icons, the
//! fonts and the HUD's text lines, in the pack's typed form (nettai-assets).
//! The routines that read them are EXE6's or EXE5's, the same or nearly
//! (docs/design/exe4-map.md §3); the addresses are where their literals
//! point in each ROM (rom.rs). What isn't extracted yet is the placeholder
//! pass's (`super::NOT_YET`).

use crate::decode::tiles;
use crate::exe4::rom::{Addresses, Rom, Roms, Version};
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
    let sprites = roms.any().map(|(rom, a)| sprites(rom, a.sprite_list)).unwrap_or_default();
    let mut hud = match roms.us() {
        Some((rom, a)) => hud(rom, a, names),
        None => crate::placeholders::hud(names, 0),
    };
    hud.chip_icons = chip_icons(roms, names);
    // (No banners yet: a pack keeps the banners' palette with their pictures,
    // so it has none.)
    hud.banner_palette = Palette::default();
    if let Some((rom, a)) = roms.any() {
        hud.icon_palette = palette(rom, rom.u32(a.icon_palette_pointer));
    }
    let lettering = match roms.jp() {
        Some((rom, a)) => lettering(rom, a, &hud, names),
        None => crate::placeholders::lettering(&hud, names, LANGUAGE),
    };
    hud.languages.push((LANGUAGE.to_string(), lettering));
    hud.language = BASE_LANGUAGE.into();
    let mut custom = crate::placeholders::custom();
    custom.chip_art = chip_art(roms, names);
    // (The custom screen is a placeholder yet, its Japanese lettering too.)
    custom.languages.push((
        LANGUAGE.to_string(),
        CustomLettering { pictures: crate::placeholders::slot_pictures(), ..Default::default() },
    ));
    Bundle { sprites, field: Field::default(), backgrounds: Vec::new(), hud, custom }
}

/// A palette (the hardware ignores bit 15 of a color).
fn palette(rom: &Rom, a: u32) -> Palette {
    palettes_from_bytes(rom.bytes(a, 32))[0].map(|c| c & 0x7FFF)
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

// ---- The HUD's lettering ------------------------------------------------------------

/// What glyph `k` of a font draws: the encoding's name for it, else its
/// number in brackets.
fn glyph_names((cell, dialogue): (&[String], &[String]), n: usize) -> Vec<String> {
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
fn texts(rom: &Rom, at: u32, first_control: u8) -> Vec<Vec<u16>> {
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
