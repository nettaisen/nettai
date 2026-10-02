//! The battle graphics, decoded from the ROM into their typed form (see the
//! nettai-assets crate); nettai-content writes them as the pack's images and
//! JSON.

use crate::{Rom, lz77, u32at};
use nettai_assets::*;
use std::collections::HashMap;

/// The battle graphics of a ROM; `names` gives the chip icons their keys
/// and the font its characters.
pub fn bundle(rom: &Rom, names: &nettai_content::names::AssetNames) -> Bundle {
    Bundle {
        sprites: sprites(rom),
        field: field(rom),
        backgrounds: backgrounds(rom),
        hud: crate::hud::hud(rom, names),
        custom: crate::custom::custom(rom, names),
    }
}

/// Bytes starting at a ROM address.
fn rom_from(rom: &Rom, a: u32, max: usize) -> Vec<u8> {
    let o = (a & 0x01FF_FFFF) as usize;
    rom.0[o..(o + max).min(rom.0.len())].to_vec()
}

fn map_entries(rom: &Rom, a: u32, n: usize) -> Vec<MapEntry> {
    (0..n as u32).map(|i| MapEntry::from_gba(rom.u16(a + 2 * i))).collect()
}

// ---- Sprites -------------------------------------------------------------------

/// `SpritePointersList`: per category, a table of sprite archive pointers
/// (bit 31 = LZ77 compressed).
const SPRITE_LIST: u32 = 0x0803_1CC4;
/// Categories with battle sprites (the byte offsets 0x00..=0x14).
const BATTLE_CATEGORIES: u32 = 6;

fn sprites(rom: &Rom) -> Vec<SpriteSheet> {
    let cats: Vec<u32> = (0..10).map(|i| u32at(rom, SPRITE_LIST + 4 * i)).collect();
    let mut out = Vec::new();
    for (ci, &c) in cats.iter().enumerate().take(BATTLE_CATEGORIES as usize) {
        let next = cats.iter().copied().filter(|&s| s > c).min().unwrap_or(c + 0x400);
        for idx in 0..((next - c) / 4).min(256) {
            let p = u32at(rom, c + 4 * idx);
            let data = if p & 0x8000_0000 != 0 {
                // A compressed archive starts with its own size word
                // (`sprite_decompress` hands out the data after it).
                match lz77(rom, p & 0x7FFF_FFFF) {
                    Some(d) if d.len() > 4 => d[4..].to_vec(),
                    _ => continue,
                }
            } else if (0x0800_0000..0x0900_0000).contains(&p) {
                rom_from(rom, p, 0x8_0000)
            } else {
                continue;
            };
            if let Some(s) = sprite_sheet(&data, (ci * 4) as u8, idx as u8) {
                out.push(s);
            }
        }
    }
    out.sort_by_key(|s| (s.category, s.index));
    out
}

/// Size of a sprite part by (shape, size).
fn part_size(shape: u8, size: u8) -> (u8, u8) {
    const SIZES: [[(u8, u8); 4]; 3] = [
        [(8, 8), (16, 16), (32, 32), (64, 64)],
        [(16, 8), (32, 8), (32, 16), (64, 32)],
        [(8, 16), (8, 32), (16, 32), (32, 64)],
    ];
    SIZES[(shape as usize).min(2)][size as usize & 3]
}

/// The most palettes kept of a sprite's palette block.
const MAX_PALETTES: usize = 64;

/// Decode a battle sprite archive (the layout `sprite_loadAnimationData`
/// reads for sprites loaded with flag 0x80). After a 4-byte header, offsets
/// are relative to `base`: a table of animation offsets; each animation is
/// 0x14-byte frames (tileset, palettes, mini-animation and part-table
/// offsets, then duration at +0x10 and flags at +0x12, 0x80 ending it).
fn sprite_sheet(data: &[u8], category: u8, index: u8) -> Option<SpriteSheet> {
    const BASE: usize = 4;
    let rd32 = |o: usize| data.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let count = rd32(BASE)? / 4;
    if count == 0 || count > 256 {
        return None;
    }
    let mut sheet = SpriteSheet { category, index, ..Default::default() };
    // Where the archive's blocks start (the animations' frames and the
    // blocks they name): a palette block runs up to the next one.
    let mut blocks: Vec<usize> = Vec::new();
    for a in 0..count {
        let mut f = BASE + rd32(BASE + 4 * a)?;
        blocks.push(f);
        for _ in 0..=512 {
            blocks.extend([rd32(f)?, rd32(f + 4)?, rd32(f + 8)?, rd32(f + 12)?].map(|o| BASE + o));
            if *data.get(f + 0x12)? & 0x80 != 0 {
                break;
            }
            f += 0x14;
        }
    }
    let mut tilesets: HashMap<usize, u16> = HashMap::new();
    let mut palsets: HashMap<usize, u16> = HashMap::new();
    let mut parts: HashMap<usize, u16> = HashMap::new();
    for a in 0..count {
        let mut f = BASE + rd32(BASE + 4 * a)?;
        let mut frames = Vec::new();
        loop {
            let (t, p, m, o) = (rd32(f)?, rd32(f + 4)?, rd32(f + 8)?, rd32(f + 12)?);
            let (duration, flags) = (*data.get(f + 0x10)?, *data.get(f + 0x12)?);
            let tileset = match tilesets.get(&t) {
                Some(&i) => i,
                None => {
                    let size = rd32(BASE + t)?;
                    let tiles = Tiles::from_4bpp(data.get(BASE + t + 4..BASE + t + 4 + size)?);
                    sheet.tilesets.push(tiles);
                    let i = (sheet.tilesets.len() - 1) as u16;
                    tilesets.insert(t, i);
                    i
                }
            };
            let palette_set = match palsets.get(&p) {
                Some(&i) => i,
                None => {
                    // The palette index is the object's choice plus the
                    // frame's offset: keep every palette up to the
                    // archive's next block (a navi's has its other forms'
                    // after its own), and at least 16 where the data goes
                    // on (the game reads whatever follows).
                    let start = BASE + p + 4;
                    let avail = data.len().saturating_sub(start) / 32;
                    let end = blocks.iter().copied().filter(|&b| b > BASE + p).min();
                    let count = end.map_or(16, |end| ((end - start) / 32).clamp(16, MAX_PALETTES));
                    let pals = palettes_from_bytes(&data[start..start + 32 * avail.min(count)]);
                    sheet.palette_sets.push(pals);
                    let i = (sheet.palette_sets.len() - 1) as u16;
                    palsets.insert(p, i);
                    i
                }
            };
            // The first frame of the frame's first mini-animation picks
            // the part list.
            let mini = BASE + m;
            let list_index = *data.get(mini + rd32(mini)?)? as usize;
            let table = BASE + o;
            let list = table + rd32(table + 4 * list_index)?;
            let parts_index = match parts.get(&list) {
                Some(&i) => i,
                None => {
                    let mut v = Vec::new();
                    let mut e = list;
                    while *data.get(e)? != 0xFF && v.len() < 128 {
                        let b = data.get(e..e + 5)?;
                        let (width, height) = part_size(b[4] & 3, b[3] & 3);
                        v.push(SpritePart {
                            tile: b[0] as u16,
                            x: b[1] as i8,
                            y: b[2] as i8,
                            width,
                            height,
                            hflip: b[3] & 0x40 != 0,
                            vflip: b[3] & 0x80 != 0,
                            palette: b[4] >> 4,
                        });
                        e += 5;
                    }
                    sheet.part_lists.push(v);
                    let i = (sheet.part_lists.len() - 1) as u16;
                    parts.insert(list, i);
                    i
                }
            };
            frames.push(SpriteFrame { tileset, palette_set, parts: parts_index, duration, flags });
            if flags & 0x80 != 0 || frames.len() > 512 {
                break;
            }
            f += 0x14;
        }
        sheet.animations.push(frames);
    }
    Some(sheet)
}

// ---- Field -------------------------------------------------------------------

/// The field tiles `sub_80075CA` decompresses to VRAM 0x06001460.
const FIELD_TILES: u32 = 0x086D_DBA0;
/// Background palettes 1..=8 at battle start (`dword_86E08F8`, copied to
/// the palette buffer's slot 1 by `sub_80075CA`'s transfer list).
const FIELD_PALETTES: u32 = 0x086E_08F8;
/// Panel blocks: 32 bytes (5x3 map entries) per 6 * type + 3 * owner + row - 1
/// (`byte_86DFA98`, read by `sub_800C01C`).
const PANEL_BLOCKS: u32 = 0x086D_FA98;
/// Highlighted panel blocks (`dword_86E0458`, `dword_86E0478`).
const HIGHLIGHT_BLOCKS: u32 = 0x086E_0458;
/// Front edges by owner (`byte_86E0498`, `sub_800C100`).
const FRONT_EDGES: u32 = 0x086E_0498;
/// Panel palette animations run by `sub_800C192` (`off_800C1DC`).
const PANEL_PALETTE_ANIMS: u32 = 0x0800_C1DC;
/// The palette buffer address of background palette 0.
const PALETTE_BUFFER: u32 = 0x0300_1960;

fn field(rom: &Rom) -> Field {
    let tiles = Tiles::from_4bpp(&lz77(rom, FIELD_TILES).expect("field tiles"));
    let palettes = palettes_from_bytes(rom.bytes(FIELD_PALETTES, 0x100));
    let block = |a: u32| -> [MapEntry; 15] { map_entries(rom, a, 15).try_into().unwrap() };
    let panels = (0..13 * 6).map(|i| block(PANEL_BLOCKS + 32 * i)).collect();
    let edge = |a: u32| -> [MapEntry; 5] { map_entries(rom, a, 5).try_into().unwrap() };
    let mut palette_anims = Vec::new();
    let mut e = PANEL_PALETTE_ANIMS;
    loop {
        let p = u32at(rom, e);
        if p == 0 {
            break;
        }
        let count = rom.u8(p + 1) as u32;
        let timer_slot = rom.u8(p + 2);
        let dest = u32at(rom, p + 4);
        let frames = (0..count)
            .map(|i| {
                let f = p + 8 + 8 * i;
                (palettes_from_bytes(rom.bytes(u32at(rom, f), 32))[0], u32at(rom, f + 4) as u8)
            })
            .collect();
        // `sub_800BF88` starts the timers (bytes 3, 5 .. 0xD of the
        // counter block) at 0xE, 0xD .. 9.
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
        panels,
        front_edges: [edge(FRONT_EDGES), edge(FRONT_EDGES + 32)],
        highlights: [block(HIGHLIGHT_BLOCKS), block(HIGHLIGHT_BLOCKS + 32)],
    }
}

// ---- Backgrounds ---------------------------------------------------------------

/// Background load data by background id (`off_8080F98`, `LoadBGAnimData`):
/// gfx source, gfx dest, tilemap source, tilemap dest offset, palette
/// source, palette dest, palette size.
const BACKGROUNDS: u32 = 0x0808_0F98;
const BACKGROUND_COUNT: u32 = 22;
/// Per background: scroll callbacks (`off_8080E34`, 16 bytes each; the
/// second one scrolls BG1).
const BACKGROUND_SCROLL: u32 = 0x0808_0E34;
/// Per background: its GFX animation list (`off_8081220`).
const BACKGROUND_ANIMS: u32 = 0x0808_1220;

/// Scroll callbacks and their counter steps (1/16 pixel per frame).
const SCROLLERS: [(u32, (i32, i32)); 5] = [
    (0x0800_19B4, (-8, -4)), // BGScrollCB_BG1Diagonal3to2Scroll
    (0x0800_19EC, (0, -4)),  // BGScrollCB_BG1UpScroll
    (0x0800_1A10, (0, 4)),   // BGScrollCB_BG1DownScroll
    (0x0800_1A34, (1, 0)),   // BGScrollCB_BG1SlowRightScroll
    (0x0800_1A58, (-8, 0)),  // BGScrollCB_BG1FastLeftScroll
];

fn backgrounds(rom: &Rom) -> Vec<Option<Background>> {
    (0..BACKGROUND_COUNT)
        .map(|id| {
            let d = u32at(rom, BACKGROUNDS + 4 * id);
            if d == 0 {
                return None;
            }
            let gfx = u32at(rom, d);
            if gfx == 0 {
                return None;
            }
            let size = u32at(rom, gfx) as usize * 4;
            let raw = lz77(rom, gfx + u32at(rom, gfx + 4))?;
            let tiles = Tiles::from_4bpp(&raw[..size.min(raw.len())]);
            let first_tile = ((u32at(rom, d + 4) & 0xFFFF) / 32) as u16;
            let map_src = u32at(rom, d + 8);
            let (w, h) = (rom.u8(map_src) as u16, rom.u8(map_src + 1) as u16);
            let map_raw = lz77(rom, map_src + 0xC)?;
            let map = (0..(w * h) as usize)
                .map(|i| MapEntry::from_gba(u16::from_le_bytes([map_raw[2 * i], map_raw[2 * i + 1]])))
                .collect();
            let pal_src = u32at(rom, d + 0x10);
            let palette = (pal_src != 0).then(|| palettes_from_bytes(rom.bytes(pal_src + 4, 32))[0]);
            let cb = u32at(rom, BACKGROUND_SCROLL + 16 * id + 4) & !1;
            let scroll = SCROLLERS.iter().find(|(a, _)| *a == cb).map(|(_, v)| *v).unwrap_or((0, 0));
            let anims = gfx_anims(rom, u32at(rom, BACKGROUND_ANIMS + 4 * id));
            Some(Background { tiles, first_tile, map, map_width: w, map_height: h, palette, scroll, anims })
        })
        .collect()
}

/// A GFX animation list (`LoadGFXAnims`): pointers to animation scripts,
/// ended by a negative word.
fn gfx_anims(rom: &Rom, list: u32) -> Vec<GfxAnim> {
    let mut out = Vec::new();
    if list == 0 {
        return out;
    }
    let mut a = list;
    loop {
        let p = u32at(rom, a);
        if p & 0x8000_0000 != 0 || p == 0 {
            break;
        }
        if let Some(anim) = gfx_anim(rom, p) {
            out.push(anim);
        }
        a += 4;
    }
    out
}

/// One GFX animation script (`LoadGFXAnim`/`ProcessGFXAnims`): a 12-byte
/// head (param0, param1, command, index, param2, param3), then entries of
/// (data, delay) with the control words 0 (end), 1 (loop) and 2 (jump).
/// Commands 0 (palette copy: dest, size) and 4 (4-bit tile copy: source,
/// dest, tiles, buffer) are decoded; others are skipped.
fn gfx_anim(rom: &Rom, p: u32) -> Option<GfxAnim> {
    let (param0, param1) = (u32at(rom, p), u32at(rom, p + 4));
    let command = rom.u8(p + 8);
    let param2 = rom.u8(p + 10);
    let target = match command {
        0 => {
            // Palette copy to the palette buffer.
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
        match u32at(rom, e) {
            0 => break None,
            1 => break Some(loop_to),
            2 => {
                let dest = u32at(rom, e + 4);
                if let Some(&i) = seen.get(&dest) {
                    break Some(i);
                }
                loop_to = frames.len();
                e = dest;
                continue;
            }
            data => {
                seen.insert(e, frames.len());
                let delay = u32at(rom, e + 4) as u16;
                let frame = match target {
                    AnimTarget::Palettes { count, .. } => GfxAnimFrame {
                        palettes: palettes_from_bytes(rom.bytes(data, 32 * count as usize)),
                        delay,
                        ..Default::default()
                    },
                    AnimTarget::Tiles { count, .. } => {
                        // Each u16: bits 0-9 a tile of the source, bits
                        // 10-11 how it is flipped.
                        let mut tiles = Tiles::default();
                        for i in 0..count as u32 {
                            let w = rom.u16(data + 2 * i);
                            let src = Tiles::from_4bpp(rom.bytes(param0 + 32 * (w & 0x3FF) as u32, 32));
                            let t = src.get(0).unwrap();
                            let (h, v) = (w & 0x400 != 0, w & 0x800 != 0);
                            let flipped: Vec<u8> = (0..64)
                                .map(|k| {
                                    let (x, y) = (k % 8, k / 8);
                                    let sx = if h { 7 - x } else { x };
                                    let sy = if v { 7 - y } else { y };
                                    t[sy * 8 + sx]
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
