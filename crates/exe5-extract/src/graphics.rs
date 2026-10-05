//! The battle graphics, decoded from the US Team ProtoMan ROM into their
//! typed form (nettai-assets), with Team Colonel's own chips. The routines
//! that read them are EXE6's, the same or nearly (the verification
//! workspace's tools/exe5 maps them); the addresses are where those routines'
//! literals point in this ROM.
//!
//! What an EXE5 pack has: the battle sprites, the field (panels, edges,
//! cycling palettes), the battle backgrounds, the chips' pictures and icons;
//! the HUD (hud.rs: its fonts, banners, faces and the chatbox) and the
//! custom screen (custom.rs), in EXE6's formats with what EXE5 lays out
//! otherwise said (`CustomLayout`, the faces' own boxes, the soul button).

use crate::rom::{Rom, Roms, Version};
use nettai_assets::*;
use nettai_content::names::AssetNames;
use std::collections::HashMap;

/// `SpritePointersList` (EXE6 0x08031CC4): per category, a table of sprite
/// archive pointers. Categories with battle sprites are the byte offsets
/// 0x00..=0x14, as in EXE6.
const SPRITE_LIST: u32 = 0x0803_2728;
const BATTLE_CATEGORIES: usize = 6;

/// The field tiles (`sub_80075CA`'s, decompressed to VRAM 0x06001460) and
/// the background palettes 1..=8 its transfer list copies.
const FIELD_TILES: u32 = 0x086F_3BA0;
const FIELD_PALETTES: u32 = 0x086F_6970;
/// Panel blocks: 32 bytes (5x3 map entries) per 6 * type + 3 * owner + row
/// - 1 (`sub_800C01C`'s table). EXE5 has 11 panel types (EXE6 13).
const PANEL_BLOCKS: u32 = 0x086F_5CB0;
const PANEL_TYPES: u32 = 11;
/// The highlighted panel block (`sub_800C0BA`'s): EXE5 has one, which it
/// draws for both highlights (EXE6 a table of two); the pack has it as both,
/// what EXE5 draws for each.
const HIGHLIGHT_BLOCK: u32 = 0x086F_64F0;
/// The front edges by owner (`sub_800C100`'s).
const FRONT_EDGES: u32 = 0x086F_6510;
/// The panel palette animations `sub_800C192`'s counterpart runs.
const PANEL_PALETTE_ANIMS: u32 = 0x0800_A7AC;
/// The palette buffer's background palette 0 (EXE6 0x03001960).
const PALETTE_BUFFER: u32 = 0x0300_3960;

/// The backgrounds' load data by background id (EXE6 `off_8080F98`), their
/// scroll callbacks (16 bytes each, the second scrolls BG1) and their GFX
/// animation lists.
const BACKGROUNDS: u32 = 0x0808_C578;
const BACKGROUND_COUNT: u32 = 29;
const BACKGROUND_SCROLL: u32 = 0x0808_C32C;
const BACKGROUND_ANIMS: u32 = 0x0808_C96C;
/// The scroll callbacks and their counters' steps (1/16 pixel a frame): the
/// same code as EXE6's `BGScrollCB_*`. EXE5's own at 0x080019EC follows the
/// joypad (a background the player scrolls) and 0x08001A24 does nothing;
/// both are drawn still.
const SCROLLERS: [(u32, (i32, i32)); 5] = [
    (0x0800_1936, (-8, -4)), // EXE6 BGScrollCB_BG1Diagonal3to2Scroll
    (0x0800_196E, (0, -4)),  // BGScrollCB_BG1UpScroll
    (0x0800_1992, (0, 4)),   // BGScrollCB_BG1DownScroll
    (0x0800_19B6, (8, 0)),   // a fast right scroll (EXE6 has a slow one)
    (0x0800_19C8, (-8, 0)),  // BGScrollCB_BG1FastLeftScroll
];

/// The chip records (0x2C bytes, as EXE6's): +0x20 the icon, +0x24 the
/// picture, +0x28 its palette. Team Colonel's table is 4 bytes lower.
const CHIP_DATA: [u32; 2] = [0x0801_E214, 0x0801_E210];
pub const CHIP_COUNT: u32 = 368;

/// The chips with art in the pack: the named ones' table, and past it the
/// ones the content names (the invalid chip, 0x185: its record's art is
/// the blank icon and picture the unused records have, what EXE5 draws).
fn chip_ids(names: &AssetNames) -> impl Iterator<Item = u32> + '_ {
    (0..CHIP_COUNT).chain(names.chips.keys().map(|&id| id as u32).filter(|&id| id >= CHIP_COUNT))
}
const PICTURE_BYTES: usize = 0x540;

/// Everything the pack draws with: `names` gives the chips their keys, the
/// fonts their characters and the faces their names.
pub fn bundle(roms: &Roms, names: &AssetNames) -> Bundle {
    let rom = &roms.protoman;
    let mut hud = crate::hud::hud(roms, names, chip_icons(roms, names));
    let mut custom = crate::custom::custom(roms, names, chip_art(roms, names));
    // The US ROMs' words are English; the Japanese ROMs' lettering is the
    // pack's Japanese (`lettering`).
    let ja = crate::lettering::LANGUAGE.to_string();
    hud.languages.push((ja.clone(), crate::lettering::hud(roms, &hud, names)));
    hud.language = nettai_assets::BASE_LANGUAGE.into();
    custom.languages.push((ja, crate::lettering::custom(roms)));
    Bundle { sprites: sprites(rom), field: field(rom), backgrounds: backgrounds(rom), hud, custom }
}

use crate::hud::{palette, tiles};

fn map_entries(rom: &Rom, a: u32, n: usize) -> Vec<MapEntry> {
    (0..n as u32).map(|i| MapEntry::from_gba(rom.u16(a + 2 * i))).collect()
}

// ---- Sprites -------------------------------------------------------------------

/// The Japanese ROMs' sprite lists (Team of Blues, Team of Colonel).
const JP_SPRITE_LISTS: [u32; 2] = [0x0803_26C0, 0x0803_26C4];

/// The battle sprites' archives in a ROM whose sprite list is at `list`, by
/// (category, index).
fn archives(rom: &Rom, list: u32) -> Vec<((u8, u8), Vec<u8>)> {
    let cats: Vec<u32> = (0..10).map(|i| rom.u32(list + 4 * i)).collect();
    let mut out = Vec::new();
    for (ci, &c) in cats.iter().enumerate().take(BATTLE_CATEGORIES) {
        let next = cats.iter().copied().filter(|&s| s > c).min().unwrap_or(c + 0x400);
        for idx in 0..((next - c) / 4).min(256) {
            if let Some(data) = crate::sprite::archive(rom, rom.u32(c + 4 * idx)) {
                out.push((((ci * 4) as u8, idx as u8), data));
            }
        }
    }
    out
}

/// The battle sprites (the two US ROMs have the same ones).
fn sprites(rom: &Rom) -> Vec<SpriteSheet> {
    let mut out: Vec<SpriteSheet> =
        archives(rom, SPRITE_LIST).into_iter().filter_map(|((c, i), data)| crate::sprite::sheet(&data, c, i)).collect();
    out.sort_by_key(|s| (s.category, s.index));
    out
}

/// The sprites a Japanese ROM draws otherwise than the US Team ProtoMan's
/// (one, 14-17, the "BLOCK!" label a blocked hit shows in a mode no
/// netbattle is, which says ブロック! there): the pack keeps the US's, which
/// the US release localized rather than cut, and no content draws it.
pub fn japanese_differences(roms: &Roms) -> Vec<(u8, u8)> {
    // Decoded, not raw: an uncompressed archive is read up to a limit, past
    // its end, where the ROMs differ.
    let decode = |rom: &Rom, list: u32| -> HashMap<(u8, u8), Option<SpriteSheet>> {
        archives(rom, list).into_iter().map(|((c, i), data)| ((c, i), crate::sprite::sheet(&data, c, i))).collect()
    };
    let us = decode(&roms.protoman, SPRITE_LIST);
    let mut out = Vec::new();
    for (v, list) in [Version::ProtoMan, Version::Colonel].into_iter().zip(JP_SPRITE_LISTS) {
        for (id, sheet) in decode(roms.jp(v), list) {
            if us.get(&id) != Some(&sheet) && !out.contains(&id) {
                out.push(id);
            }
        }
    }
    out.sort();
    out
}

// ---- Field -----------------------------------------------------------------------

fn field(rom: &Rom) -> Field {
    let tiles = Tiles::from_4bpp(&rom.lz77(FIELD_TILES).expect("the field's tiles"));
    let palettes = palettes_from_bytes(rom.bytes(FIELD_PALETTES, 0x100));
    let block = |a: u32| -> [MapEntry; 15] { map_entries(rom, a, 15).try_into().unwrap() };
    let edge = |a: u32| -> [MapEntry; 5] { map_entries(rom, a, 5).try_into().unwrap() };
    let mut palette_anims = Vec::new();
    let mut e = PANEL_PALETTE_ANIMS;
    loop {
        let p = rom.u32(e);
        if p == 0 {
            break;
        }
        let count = rom.u8(p + 1) as u32;
        let timer_slot = rom.u8(p + 2);
        let dest = rom.u32(p + 4);
        let frames = (0..count)
            .map(|i| {
                let f = p + 8 + 8 * i;
                (palettes_from_bytes(rom.bytes(rom.u32(f), 32))[0], rom.u32(f + 4) as u8)
            })
            .collect();
        // The timers start at 0xE, 0xD .. by their counter byte (EXE6's
        // `sub_800BF88`).
        let initial_timer = 0xE - (timer_slot.saturating_sub(3) / 2);
        palette_anims.push(PaletteAnim { slot: ((dest - PALETTE_BUFFER) / 32) as u8, frames, initial_timer });
        e += 4;
    }
    Field {
        tiles,
        first_tile: (0x1460 / 32) as u16,
        palettes,
        first_palette: 1,
        palette_anims,
        // (EXE5's types in its own order, each by the engine's number:
        // content/exe5/compat/panels.toml.)
        panel_types: (0..PANEL_TYPES as u8)
            .map(|n| match exe5_compat::Compat::exe5().panel_type(n) {
                Ok(Some(t)) => t as u8,
                other => panic!("EXE5's panel type {n} is no engine panel type ({other:?})"),
            })
            .collect(),
        panels: (0..PANEL_TYPES * 6).map(|i| block(PANEL_BLOCKS + 32 * i)).collect(),
        front_edges: [edge(FRONT_EDGES), edge(FRONT_EDGES + 32)],
        highlights: vec![block(HIGHLIGHT_BLOCK), block(HIGHLIGHT_BLOCK)],
    }
}

// ---- Backgrounds -------------------------------------------------------------------

/// A background's picture from its load data at `d`: its tiles, the tile
/// number they load at, its map with its size, and its palette.
type BackgroundPicture = (Tiles, u16, Vec<MapEntry>, u16, u16, Option<Palette>);

fn background_picture(rom: &Rom, d: u32) -> Option<BackgroundPicture> {
    if d == 0 {
        return None;
    }
    let gfx = rom.u32(d);
    if gfx == 0 {
        return None;
    }
    let size = rom.u32(gfx) as usize * 4;
    let raw = rom.lz77(gfx + rom.u32(gfx + 4))?;
    let tiles = Tiles::from_4bpp(&raw[..size.min(raw.len())]);
    let first_tile = ((rom.u32(d + 4) & 0xFFFF) / 32) as u16;
    let map_src = rom.u32(d + 8);
    let (w, h) = (rom.u8(map_src) as u16, rom.u8(map_src + 1) as u16);
    let map_raw = rom.lz77(map_src + 0xC)?;
    let map = (0..(w * h) as usize).map(|i| MapEntry::from_gba(u16::from_le_bytes([map_raw[2 * i], map_raw[2 * i + 1]]))).collect();
    let pal_src = rom.u32(d + 0x10);
    let palette = (pal_src != 0).then(|| palettes_from_bytes(rom.bytes(pal_src + 4, 32))[0]);
    Some((tiles, first_tile, map, w, h, palette))
}

fn backgrounds(rom: &Rom) -> Vec<Option<Background>> {
    (0..BACKGROUND_COUNT)
        .map(|id| {
            let (tiles, first_tile, map, map_width, map_height, palette) = background_picture(rom, rom.u32(BACKGROUNDS + 4 * id))?;
            let cb = rom.u32(BACKGROUND_SCROLL + 16 * id + 4) & !1;
            let scroll = SCROLLERS.iter().find(|(a, _)| *a == cb).map(|(_, v)| *v).unwrap_or((0, 0));
            let anims = gfx_anims(rom, rom.u32(BACKGROUND_ANIMS + 4 * id));
            Some(Background { tiles, first_tile, map, map_width, map_height, palette, scroll, anims })
        })
        .collect()
}

/// The Japanese ROMs' backgrounds' load data (Team of Blues, Team of
/// Colonel).
const JP_BACKGROUNDS: [u32; 2] = [0x0808_BEC4, 0x0808_BF34];

/// The backgrounds whose picture a Japanese ROM has another of than the US
/// Team ProtoMan's (one, 0x05: the US ROMs' goldfish, the Japanese ROMs'
/// bubbles in the dark). No netbattle shows it, and the pack keeps the
/// US's.
pub fn japanese_backgrounds(roms: &Roms) -> Vec<u8> {
    let us = &roms.protoman;
    (0..BACKGROUND_COUNT)
        .filter(|&id| {
            let own = background_picture(us, us.u32(BACKGROUNDS + 4 * id));
            [Version::ProtoMan, Version::Colonel].into_iter().zip(JP_BACKGROUNDS).any(|(v, table)| {
                let rom = roms.jp(v);
                background_picture(rom, rom.u32(table + 4 * id)) != own
            })
        })
        .map(|id| id as u8)
        .collect()
}

/// A GFX animation list: pointers to animation scripts, ended by a negative
/// word or 0.
fn gfx_anims(rom: &Rom, list: u32) -> Vec<GfxAnim> {
    let mut out = Vec::new();
    if list == 0 || !rom.contains(list) {
        return out;
    }
    let mut a = list;
    loop {
        let p = rom.u32(a);
        if p & 0x8000_0000 != 0 || p == 0 || !rom.contains(p) {
            break;
        }
        if let Some(anim) = gfx_anim(rom, p) {
            out.push(anim);
        }
        a += 4;
    }
    out
}

/// One GFX animation script (EXE6's `LoadGFXAnim`, the same here): a 12-byte
/// head (param0, param1, command, index, param2, param3), then entries of
/// (data, delay) with the control words 0 (end), 1 (loop) and 2 (jump).
/// Commands 0 (palette copy) and 4 (4-bit tile copy) are decoded.
fn gfx_anim(rom: &Rom, p: u32) -> Option<GfxAnim> {
    let (param0, param1) = (rom.u32(p), rom.u32(p + 4));
    let command = rom.u8(p + 8);
    let param2 = rom.u8(p + 10);
    let target = match command {
        0 => {
            let first = param0.checked_sub(PALETTE_BUFFER)? / 32;
            if first >= 16 {
                return None;
            }
            AnimTarget::Palettes { first: first as u8, count: (param1 / 32) as u8 }
        }
        4 => AnimTarget::Tiles { first: ((param1 & 0xFFFF) / 32) as u16, count: param2 as u16 },
        _ => return None,
    };
    let mut frames = Vec::new();
    let mut seen: HashMap<u32, usize> = HashMap::new();
    let mut loop_to = 0usize;
    let mut e = p + 12;
    let repeat_from = loop {
        if frames.len() > 512 {
            break None;
        }
        match rom.u32(e) {
            0 => break None,
            1 => break Some(loop_to),
            2 => {
                let dest = rom.u32(e + 4);
                if let Some(&i) = seen.get(&dest) {
                    break Some(i);
                }
                loop_to = frames.len();
                e = dest;
                continue;
            }
            data => {
                seen.insert(e, frames.len());
                let delay = rom.u32(e + 4) as u16;
                let frame = match target {
                    AnimTarget::Palettes { count, .. } => GfxAnimFrame {
                        palettes: palettes_from_bytes(rom.bytes(data, 32 * count as usize)),
                        delay,
                        ..Default::default()
                    },
                    AnimTarget::Tiles { count, .. } => {
                        // Each u16: a tile of the source (bits 0-9) and its
                        // flips (10, 11).
                        let mut tiles = Tiles::default();
                        for i in 0..count as u32 {
                            let w = rom.u16(data + 2 * i);
                            let src = Tiles::from_4bpp(rom.bytes(param0 + 32 * (w & 0x3FF) as u32, 32));
                            let t = src.get(0).unwrap();
                            let (h, v) = (w & 0x400 != 0, w & 0x800 != 0);
                            let flipped: Vec<u8> = (0..64)
                                .map(|k| {
                                    let (x, y) = (k % 8, k / 8);
                                    t[(if v { 7 - y } else { y }) * 8 + if h { 7 - x } else { x }]
                                })
                                .collect();
                            tiles.push(&flipped);
                        }
                        GfxAnimFrame { tiles, delay, ..Default::default() }
                    }
                    AnimTarget::Nothing => unreachable!(),
                };
                frames.push(frame);
                e += 8;
            }
        }
    };
    Some(GfxAnim { target, frames, repeat_from })
}

// ---- Chips -------------------------------------------------------------------------

/// A chip's record in a version's ROM.
fn record(v: Version, id: u32) -> u32 {
    CHIP_DATA[(v == Version::Colonel) as usize] + 0x2C * id
}

/// A chip's picture (tiles and palette), icon and whether its record has
/// them, in a version's ROM. A picture whose palette no ROM holds (the
/// e-Reader cards' LeadRaid and ChaosLrd: their records point at EWRAM,
/// 0x02001660 and 0x02001680, where the save's card data goes) gets a
/// black one; its definition gives the palette (`art_palette`), as EXE6's
/// gift chips' do.
fn chip_media(roms: &Roms, v: Version, id: u32) -> (Picture, Tiles) {
    let rom = roms.us(v);
    let r = record(v, id);
    let (icon, gfx, pal) = (rom.u32(r + 0x20), rom.u32(r + 0x24), rom.u32(r + 0x28));
    let picture = if rom.contains(gfx) {
        let palette = if rom.contains(pal) { palette(rom, pal) } else { Default::default() };
        Picture { tiles: tiles(rom, gfx, PICTURE_BYTES), palette }
    } else {
        Picture::default()
    };
    let icon = if rom.contains(icon) { tiles(rom, icon, 0x80) } else { Tiles::default() };
    (picture, icon)
}

/// The library flag of a chip's record (+0x09): set in the version whose
/// library lists the chip.
const LIBRARY: u8 = 0x40;

/// The version chips (12: each version's five Giga chips, 0x12D to 0x136,
/// and DethPhnx and Phoenix, 0x13A and 0x139), each with the version it
/// belongs to (the one whose library lists it: its record's library flag
/// there). A version's ROM holds the art of its own, and has it again at
/// the other version's counterparts (Team ProtoMan's ROM draws MetrKnuk
/// and CrossDiv as HolyDrem, OmegaRkt and BugCharg as BigHook, Phoenix as
/// DethPhnx), so these are the chips whose art differs between the two US
/// ROMs.
pub fn version_chips(roms: &Roms) -> Vec<(u32, Version)> {
    (0..CHIP_COUNT)
        .filter(|&id| chip_media(roms, Version::ProtoMan, id) != chip_media(roms, Version::Colonel, id))
        .map(|id| {
            let listed = |v: Version| roms.us(v).u8(record(v, id) + 9) & LIBRARY != 0;
            match (listed(Version::ProtoMan), listed(Version::Colonel)) {
                (true, false) => (id, Version::ProtoMan),
                (false, true) => (id, Version::Colonel),
                both => panic!("chip {id:#05x}'s art differs between the versions' ROMs, and their libraries list it {both:?}: one is its own"),
            }
        })
        .collect()
}

/// The version chip `id` belongs to; none for a chip of both.
fn version_of(versions: &[(u32, Version)], id: u32) -> Option<Version> {
    versions.iter().find(|(chip, _)| *chip == id).map(|&(_, v)| v)
}

/// The chips' pictures, each under its chip's key: a version chip's from
/// its own version's ROM, marked with it (a console of the other version
/// shows its counterpart's there); any other from Team ProtoMan's.
fn chip_art(roms: &Roms, names: &AssetNames) -> Vec<ChipArt> {
    let versions = version_chips(roms);
    chip_ids(names)
        .map(|id| {
            let own = version_of(&versions, id);
            let (picture, _) = chip_media(roms, own.unwrap_or(Version::ProtoMan), id);
            ChipArt { key: names.chip_icon(id as u16), picture, region: None, version: own.map(|v| v.name().into()) }
        })
        .collect()
}

/// The chips' icons, each under its chip's key, from the ROM its picture
/// is from.
fn chip_icons(roms: &Roms, names: &AssetNames) -> Vec<ChipIcon> {
    let versions = version_chips(roms);
    chip_ids(names)
        .map(|id| {
            let (_, icon) = chip_media(roms, version_of(&versions, id).unwrap_or(Version::ProtoMan), id);
            ChipIcon { key: names.chip_icon(id as u16), tiles: icon }
        })
        .collect()
}
