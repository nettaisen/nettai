//! The sprite and portrait archive format shared by EXE5 and EXE6.

use crate::rom::Rom;
use nettai_assets::*;
use std::collections::HashMap;

/// A sprite archive at `p` in a sprite list (bit 31: LZ77 compressed, its
/// first word then its size): the data `sprite_loadAnimationData` reads.
pub fn archive(rom: &Rom, p: u32) -> Option<Vec<u8>> {
    if p & 0x8000_0000 != 0 {
        match rom.lz77(p & 0x7FFF_FFFF) {
            Some(d) if d.len() > 4 => Some(d[4..].to_vec()),
            _ => None,
        }
    } else if rom.contains(p) {
        Some(rom.from(p, 0x8_0000).to_vec())
    } else {
        None
    }
}

/// The most palettes kept of a sprite's palette block.
const MAX_PALETTES: usize = 64;

/// A sprite archive's sheet. After a 4-byte header, offsets are relative to
/// `BASE`: a table of animation offsets; each animation is 0x14-byte frames
/// (tileset, palettes, mini-animation and part-table offsets, then the
/// duration at +0x10 and the flags at +0x12, 0x80 ending it).
pub fn sheet(data: &[u8], category: u8, index: u8) -> Option<SpriteSheet> {
    const BASE: usize = 4;
    let rd32 = |o: usize| {
        data.get(o..o + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
    };
    let count = rd32(BASE)? / 4;
    if count == 0 || count > 256 {
        return None;
    }
    let mut sheet = SpriteSheet {
        category,
        index,
        ..Default::default()
    };
    // Where the archive's blocks start: a palette block runs up to the next.
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
                    sheet.tilesets.push(Tiles::from_4bpp(
                        data.get(BASE + t + 4..BASE + t + 4 + size)?,
                    ));
                    let i = (sheet.tilesets.len() - 1) as u16;
                    tilesets.insert(t, i);
                    i
                }
            };
            let palette_set = match palsets.get(&p) {
                Some(&i) => i,
                None => {
                    // Every palette up to the archive's next block, and at
                    // least 16 where the data goes on.
                    let start = BASE + p + 4;
                    let avail = data.len().saturating_sub(start) / 32;
                    let end = blocks.iter().copied().filter(|&b| b > BASE + p).min();
                    let count = end.map_or(16, |end| {
                        (end.saturating_sub(start) / 32).clamp(16, MAX_PALETTES)
                    });
                    sheet.palette_sets.push(palettes_from_bytes(
                        data.get(start..start + 32 * avail.min(count))?,
                    ));
                    let i = (sheet.palette_sets.len() - 1) as u16;
                    palsets.insert(p, i);
                    i
                }
            };
            // The first frame of the frame's first mini-animation picks the
            // part list.
            let mini = BASE + m;
            let list_index = *data.get(mini + rd32(mini)?)? as usize;
            let table = BASE + o;
            let list = table + rd32(table + 4 * list_index)?;
            let parts_index = match parts.get(&list) {
                Some(&i) => i,
                None => {
                    sheet.part_lists.push(part_list(data, list)?);
                    let i = (sheet.part_lists.len() - 1) as u16;
                    parts.insert(list, i);
                    i
                }
            };
            frames.push(SpriteFrame {
                tileset,
                palette_set,
                parts: parts_index,
                duration,
                flags,
            });
            if flags & 0x80 != 0 || frames.len() > 512 {
                break;
            }
            f += 0x14;
        }
        sheet.animations.push(frames);
    }
    Some(sheet)
}

/// A portrait's sheet (the chatbox's speaker: the archive loaded without
/// the sprite flag 0x80, as EXE6's): one frame, whose mini-animations are
/// its faces (still, idle, talking); each becomes an animation of the
/// sheet, and each of its entries (part list, duration, flags) a frame.
pub fn portrait_sheet(data: &[u8], category: u8, index: u8) -> Option<SpriteSheet> {
    const BASE: usize = 4;
    let rd32 = |o: usize| {
        data.get(o..o + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
    };
    // Animation 0's frame (the table's first offset is also its size).
    let f = BASE + rd32(BASE).filter(|&o| o != 0)?;
    let (t, p, m, o) = (rd32(f)?, rd32(f + 4)?, rd32(f + 8)?, rd32(f + 12)?);
    let mut sheet = SpriteSheet {
        category,
        index,
        ..Default::default()
    };
    let size = rd32(BASE + t)?;
    sheet.tilesets.push(Tiles::from_4bpp(
        data.get(BASE + t + 4..BASE + t + 4 + size)?,
    ));
    let start = BASE + p + 4;
    let avail = data.len().saturating_sub(start) / 32;
    sheet.palette_sets.push(palettes_from_bytes(
        data.get(start..start + 32 * avail.min(16))?,
    ));
    let (mini, table) = (BASE + m, BASE + o);
    let mut lists: HashMap<usize, u16> = HashMap::new();
    for a in 0..(rd32(mini)? / 4).min(16) {
        let mut e = mini + rd32(mini + 4 * a)?;
        let mut frames = Vec::new();
        loop {
            let (list_index, duration, flags) =
                (*data.get(e)? as usize, *data.get(e + 1)?, *data.get(e + 2)?);
            let list = table + rd32(table + 4 * list_index)?;
            let parts = match lists.get(&list) {
                Some(&i) => i,
                None => {
                    sheet.part_lists.push(part_list(data, list)?);
                    let i = (sheet.part_lists.len() - 1) as u16;
                    lists.insert(list, i);
                    i
                }
            };
            frames.push(SpriteFrame {
                tileset: 0,
                palette_set: 0,
                parts,
                duration,
                flags,
            });
            if flags & 0x80 != 0 || frames.len() > 64 {
                break;
            }
            e += 3;
        }
        sheet.animations.push(frames);
    }
    Some(sheet)
}

/// A part list: five bytes a part (tile, x, y, size and flips, shape and
/// palette offset), up to 0xFF.
fn part_list(data: &[u8], list: usize) -> Option<Vec<SpritePart>> {
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
    Some(v)
}

/// The size of a sprite part by (shape, size).
fn part_size(shape: u8, size: u8) -> (u8, u8) {
    const SIZES: [[(u8, u8); 4]; 3] = [
        [(8, 8), (16, 16), (32, 32), (64, 64)],
        [(16, 8), (32, 8), (32, 16), (64, 32)],
        [(8, 16), (8, 32), (16, 32), (32, 64)],
    ];
    SIZES[(shape as usize).min(2)][size as usize & 3]
}
