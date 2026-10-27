//! GFX animation lists (the primitive `gfx.anims`, docs/design/extraction-recipes.md §3.6): a background's
//! palette frames, tile frames (each tile flipped as its word says) and EXE4's palette transform, which the
//! command byte in the data selects.
use crate::rom::Rom;
use nettai_assets::*;
use std::collections::HashMap;

/// The animations listed at `list` (a pointer each, up to the first word that isn't one), the palette
/// animations' targets read against the palette buffer at `palette_buffer`.
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
