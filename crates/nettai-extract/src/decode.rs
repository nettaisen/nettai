//! Binary graphics formats shared by both games; address tables stay in each game.
use crate::rom::Rom;
use nettai_assets::*;
use std::collections::HashMap;

/// A BG1 scroll callback that speeds up, read from its code (EXE5's
/// 0x080019EC, EXE4's the scrollers' +0xB8): it loads its counter, takes
/// `movs r3, #a; lsls r3, r3, #b` from it, holds it at `movs r3, #c; lsls
/// r3, r3, #d; negs r3, r3`, stores it, and adds the counter's arithmetic
/// shift right 16 to BG1's offset (`ldrh r3, [r1, #o]; adds r3, r3, r2`:
/// the picture moves the counter's way) or takes it away (`subs`: the
/// other way); +0x10 the x offset, +0x12 the y. One that first asks for a
/// battle flag (`push {lr}; bl ...; movs r1, #flag; tst r0, r1; beq`) is
/// `from_battle` (EXE5's flag 0x40, which the battle's intro sets on the
/// battle's first frame); any other flag, or other code, is none.
pub fn speeding_scroll(rom: &Rom, at: u32) -> Option<Speeding> {
    let op = |i: u32| rom.u16(at + 2 * i);
    let movs = |o: u16, rd: u16| (o >> 11 == 0b00100 && (o >> 8) & 7 == rd).then_some((o & 0xFF) as i32);
    let lsls = |o: u16, rd: u16| (o >> 11 == 0 && o & 7 == rd && (o >> 3) & 7 == rd).then_some(((o >> 6) & 31) as u32);
    // (The flag test: push {lr}, a bl's two halves, movs r1, #flag, tst r0, r1, beq.)
    let from_battle = op(0) == 0xB500;
    let mut i = 0;
    if from_battle {
        if op(1) >> 11 != 0b11110 || op(2) >> 11 != 0b11111 || movs(op(3), 1) != Some(0x40) || op(4) != 0x4208 || op(5) >> 8 != 0xD0 {
            return None;
        }
        i = 6;
    }
    // ldr r1, [pc, #n] (the counter); ldr r2, [r1, #0].
    if op(i) >> 8 != 0x49 || op(i + 1) != 0x680A {
        return None;
    }
    let step = movs(op(i + 2), 3)? << lsls(op(i + 3), 3)?;
    // subs r2, r2, r3; then the top, negated.
    if op(i + 4) != 0x1AD2 {
        return None;
    }
    let top = movs(op(i + 5), 3)? << lsls(op(i + 6), 3)?;
    if op(i + 7) != 0x425B {
        return None;
    }
    // cmp r2, r3; bge; adds r2, r3, #0; str r2, [r1, #0]; asrs r2, r2, #16;
    // mov r1, sl; ldr r1, [r1, #8] (the render info); then BG1's offset.
    let (load, change) = (op(i + 15), op(i + 16));
    if load >> 11 != 0b10001 || load & 0x3F != (1 << 3) | 3 || op(i + 12) != 0x1412 {
        return None;
    }
    let sign = match change {
        0x189B => -1, // adds r3, r3, r2: the counter's way, which falls
        0x1A9B => 1,  // subs r3, r3, r2
        _ => return None,
    };
    let (step, top) = (sign * step, sign * top);
    match ((load >> 6) & 31) * 2 {
        0x10 => Some(Speeding { step: (step, 0), top: (top, 0), from_battle }),
        0x12 => Some(Speeding { step: (0, step), top: (0, top), from_battle }),
        _ => None,
    }
}

pub fn gfx_anims(rom: &Rom, list: u32, palette_buffer: u32) -> Vec<GfxAnim> {
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
        if let Some(anim) = gfx_anim(rom, p, palette_buffer) {
            out.push(anim);
        }
        a += 4;
    }
    out
}

fn gfx_anim(rom: &Rom, p: u32, palette_buffer: u32) -> Option<GfxAnim> {
    let (param0, param1) = (rom.u32(p), rom.u32(p + 4));
    let command = rom.u8(p + 8);
    let param2 = rom.u8(p + 10);
    let target = match command {
        0 => {
            let first = param0.checked_sub(palette_buffer)? / 32;
            if first >= 16 {
                return None;
            }
            AnimTarget::Palettes {
                first: first as u8,
                count: (param1 / 32) as u8,
            }
        }
        4 => AnimTarget::Tiles {
            first: ((param1 & 0xFFFF) / 32) as u16,
            count: param2 as u16,
        },
        // A palette transform (EXE4's 0x080024BC): its mode (0 adds, 4 takes
        // away), over the palettes shown (the buffer's copy 0x200 on, which
        // 0x0800258C makes each frame and shifts).
        0xC => {
            let first = param1.checked_sub(palette_buffer + 0x200)? / 32;
            if first >= 16 || !matches!(param0, 0 | 4) {
                return None;
            }
            AnimTarget::PaletteShift { first: first as u8, count: param2, darken: param0 == 4 }
        }
        _ => return None,
    };
    let mut frames = Vec::new();
    let mut seen: HashMap<u32, usize> = HashMap::new();
    let mut jumps = std::collections::HashSet::new();
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
                if !jumps.insert(e) {
                    return None;
                }
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
                    // (The color, its word's top bit set.)
                    AnimTarget::PaletteShift { .. } => GfxAnimFrame { shift: (data & 0x7FFF) as u16, delay, ..Default::default() },
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
                            let src =
                                Tiles::from_4bpp(rom.bytes(param0 + 32 * (w & 0x3FF) as u32, 32));
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
                        GfxAnimFrame {
                            tiles,
                            delay,
                            ..Default::default()
                        }
                    }
                    AnimTarget::Nothing => unreachable!(),
                };
                frames.push(frame);
                e += 8;
            }
        }
    };
    Some(GfxAnim {
        target,
        frames,
        repeat_from,
    })
}

pub fn tiles(rom: &Rom, a: u32, len: usize) -> Tiles {
    if !rom.is_present() {
        return crate::placeholders::tiles(len / 32);
    }
    Tiles::from_4bpp(rom.bytes(a, len))
}

pub fn banner_at(rom: &Rom, (banners, filler): (u32, u32), id: u32) -> BannerLayout {
    let p = rom.u32(banners + 4 * id);
    let head = rom.u32(p);
    let (x, y, kind) = (head as u8, (head >> 8) as u8, (head >> 16) as u8);
    let mut glyphs = Tiles::default();
    let mut number_at = None;
    // Kind 3 (the telops) has no glyphs; kind 4 (the judge's) has them
    // like the plain ones.
    if kind <= 2 || kind == 4 {
        // 20 glyph pointers; once the filler shows up it repeats.
        let mut q = p + 4;
        for _ in 0..20 {
            let g = rom.u32(q);
            let t = tiles(rom, g, 0x40);
            glyphs.push(t.get(0).unwrap());
            glyphs.push(t.get(1).unwrap());
            if g != filler {
                q += 4;
            }
        }
        if rom.u32(q) == filler {
            q += 4;
        }
        if kind == 1 {
            let n = rom.u32(q);
            number_at = Some((n as u8, (n >> 8) as u8));
        }
    }
    BannerLayout {
        x,
        y,
        kind,
        glyphs,
        number_at,
    }
}

pub fn patches(rom: &Rom, (mut a, first_tile): (u32, u16)) -> PatchList {
    let mut patches = Vec::new();
    while rom.u8(a) != 0xFF {
        let b = rom.bytes(a, 6);
        patches.push(MapPatch {
            x: b[0],
            y: b[1],
            width: b[2],
            height: b[3],
            palette: b[4],
            by_column: b[5] == 1,
        });
        a += 6;
    }
    PatchList {
        first_tile,
        patches,
    }
}

/// Addresses of the common battle-field format.
pub struct FieldAddresses {
    pub tiles: u32,
    pub palettes: u32,
    pub panels: u32,
    pub highlights: [u32; 2],
    pub edges: u32,
    pub palette_anims: u32,
    pub palette_buffer: u32,
}
pub fn field(rom: &Rom, a: FieldAddresses, panel_types: Vec<String>) -> Field {
    let palette_anims = palette_anims(rom, a.palette_anims, a.palette_buffer);
    field_with(rom, a, panel_types, palette_anims)
}

/// The panel palette animations of a list in EXE6's and EXE5's form
/// (`sub_800C192`'s): a word list of records, each its frame count, timer
/// slot and destination, then a palette and a duration a frame.
pub fn palette_anims(rom: &Rom, list: u32, palette_buffer: u32) -> Vec<PaletteAnim> {
    let mut palette_anims = Vec::new();
    let mut e = list;
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
                (
                    palettes_from_bytes(rom.bytes(rom.u32(f), 32))[0],
                    rom.u32(f + 4) as u8,
                )
            })
            .collect();
        // The timers start at 0xE, 0xD .. by their counter byte (EXE6's
        // `sub_800BF88`).
        let initial_timer = 0xE - (timer_slot.saturating_sub(3) / 2);
        palette_anims.push(PaletteAnim {
            slot: ((dest - palette_buffer) / 32) as u8,
            frames,
            initial_timer,
        });
        e += 4;
    }
    palette_anims
}

/// The field at `a` (its `palette_anims` unread), with these panel palette
/// animations.
pub fn field_with(rom: &Rom, a: FieldAddresses, panel_types: Vec<String>, palette_anims: Vec<PaletteAnim>) -> Field {
    let tiles = Tiles::from_4bpp(&rom.lz77(a.tiles).expect("the field's tiles"));
    let palettes = palettes_from_bytes(rom.bytes(a.palettes, 0x100));
    let block = |a: u32| -> [MapEntry; 15] { map_entries(rom, a, 15).try_into().unwrap() };
    let edge = |a: u32| -> [MapEntry; 5] { map_entries(rom, a, 5).try_into().unwrap() };
    Field {
        tiles,
        first_tile: (0x1460 / 32) as u16,
        palettes,
        first_palette: 1,
        palette_anims,
        panels: (0..panel_types.len() as u32 * 6)
            .map(|i| block(a.panels + 32 * i))
            .collect(),
        panel_types,
        front_edges: [edge(a.edges), edge(a.edges + 32)],
        highlights: a.highlights.map(block).to_vec(),
    }
}
pub fn map_entries(rom: &Rom, a: u32, n: usize) -> Vec<MapEntry> {
    (0..n as u32)
        .map(|i| MapEntry::from_gba(rom.u16(a + 2 * i)))
        .collect()
}
type BackgroundPicture = (Tiles, u16, Vec<MapEntry>, u16, u16, Option<Palette>);

/// Where a background's load descriptor keeps what it copies: the tiles'
/// archive, their VRAM destination, the map's archive and the palette's
/// source, as byte offsets into the descriptor. EXE6's and EXE5's
/// (`DIRECT`) copy each straight to its place; EXE4's (0x08026234,
/// 0x08026266) decompresses into a buffer first, each source followed by
/// its buffer.
#[derive(Clone, Copy)]
pub struct BackgroundDescriptor {
    pub gfx: u32,
    pub dest: u32,
    pub map: u32,
    pub palette: u32,
}

impl BackgroundDescriptor {
    /// EXE6's (`off_8080F98`'s) and EXE5's.
    pub const DIRECT: Self = Self { gfx: 0, dest: 4, map: 8, palette: 0x10 };
}

pub fn background_picture(rom: &Rom, d: u32) -> Option<BackgroundPicture> {
    background_picture_by(rom, d, BackgroundDescriptor::DIRECT)
}

/// A background's picture from its descriptor at `d`, laid out as `at`.
pub fn background_picture_by(rom: &Rom, d: u32, at: BackgroundDescriptor) -> Option<BackgroundPicture> {
    if d == 0 {
        return None;
    }
    let gfx = rom.u32(d + at.gfx);
    if gfx == 0 {
        return None;
    }
    let size = rom.u32(gfx) as usize * 4;
    let raw = rom.lz77(gfx + rom.u32(gfx + 4))?;
    let tiles = Tiles::from_4bpp(&raw[..size.min(raw.len())]);
    let first_tile = ((rom.u32(d + at.dest) & 0xFFFF) / 32) as u16;
    let map_src = rom.u32(d + at.map);
    let (w, h) = (rom.u8(map_src) as u16, rom.u8(map_src + 1) as u16);
    let map_raw = rom.lz77(map_src + 0xC)?;
    let map = (0..(w * h) as usize)
        .map(|i| MapEntry::from_gba(u16::from_le_bytes([map_raw[2 * i], map_raw[2 * i + 1]])))
        .collect();
    let pal_src = rom.u32(d + at.palette);
    let palette = (pal_src != 0).then(|| palettes_from_bytes(rom.bytes(pal_src + 4, 32))[0]);
    Some((tiles, first_tile, map, w, h, palette))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animation_control_cycle_is_rejected() {
        let mut data = vec![0; 20];
        data[0..4].copy_from_slice(&0x0300_1960u32.to_le_bytes());
        data[4..8].copy_from_slice(&32u32.to_le_bytes());
        data[12..16].copy_from_slice(&2u32.to_le_bytes());
        data[16..20].copy_from_slice(&0x0800_000cu32.to_le_bytes());
        assert!(gfx_anim(&Rom(data), 0x0800_0000, 0x0300_1960).is_none());
    }
}
