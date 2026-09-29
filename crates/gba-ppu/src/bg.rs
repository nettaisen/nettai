// Derived from mGBA (src/gba/renderers/software-mode0.c, software-bg.c and
// software-private.h), Copyright (c) 2013-2015 Jeffrey Pfau, licensed under
// the Mozilla Public License, v. 2.0. This file is therefore also under the
// MPL-2.0: https://mozilla.org/MPL/2.0/.

//! Background layers: text (mode 0/1), affine (mode 1/2) and bitmap
//! (mode 3/4/5) backgrounds.
//!
//! Each function draws one background into the current window segment
//! (`Ppu::start..Ppu::end`) of the scanline buffer. The text renderer
//! mirrors mGBA's tile-at-a-time structure (partial first tile, whole
//! tiles, partial last tile) rather than working per pixel, because mGBA's
//! output depends on it in a few corner cases: tile data past 0x10000 is
//! read for 4bpp partial tiles but skipped for whole ones, and a
//! horizontally flipped partial first tile in a segment narrower than the
//! tile is drawn from the wrong end.

use crate::Ppu;
use crate::color::{brighten, darken};
use crate::composite::*;
use crate::state::{Background, BlendEffect};
use crate::{NORMAL, VARIANT, read_u16, read_u32};

const MAP_HFLIP: u16 = 0x0400;
const MAP_VFLIP: u16 = 0x0800;

/// How one background's pixels are composited within the current window
/// segment (mGBA: the locals set up by `BACKGROUND_BITMAP_INIT` and
/// `PREPARE_OBJWIN`).
#[derive(Clone, Copy)]
pub(crate) struct BgCompositor {
    flags: u32,
    objwin_flags: u32,
    /// Palette for pixels outside the OBJ window (brightness-adjusted when
    /// the layer is a 1st target of brighten/darken in this window).
    palette: usize,
    /// Palette for pixels inside the OBJ window.
    objwin_palette: usize,
    objwin_slow_path: bool,
    objwin_force_enable: bool,
    objwin_only: bool,
    /// The layer is a 2nd target, so it may blend with what's in front.
    blend: bool,
}

impl BgCompositor {
    /// Whether a pixel with buffer value `current` may be drawn, given the
    /// layer's per-window enables inside and outside the OBJ window.
    #[inline]
    fn visible(&self, current: u32) -> bool {
        self.objwin_force_enable || (current & FLAG_OBJWIN == 0) == self.objwin_only
    }
}

/// Text-mode geometry of the scanline being drawn.
#[derive(Clone, Copy)]
struct TextLine {
    /// Halfword index into VRAM of the map row for this scanline (mGBA:
    /// `yBase`).
    y_base: u32,
    /// 512-pixel-wide map (screen size 1 or 3).
    wide: bool,
    char_base: u32,
    in_y: i32,
}

impl TextLine {
    /// Map entry for map x coordinate `local_x` (any value; wraps at the map
    /// width through masking, including negative values).
    #[inline]
    fn map_entry(&self, vram: &[u8], local_x: i32) -> u16 {
        let mut x_base = (local_x & 0xF8) as u32;
        if self.wide {
            x_base += ((local_x & 0x100) << 5) as u32;
        }
        read_u16(vram, ((self.y_base + (x_base >> 3)) << 1) as usize)
    }

    /// Row within the tile, after vertical flip.
    #[inline]
    fn tile_row(&self, map: u16) -> u32 {
        let row = (self.in_y & 7) as u32;
        if map & MAP_VFLIP != 0 { 7 - row } else { row }
    }

    #[inline]
    fn char_base_16(&self, map: u16) -> u32 {
        self.char_base + (u32::from(map & 0x3FF) << 5) + (self.tile_row(map) << 2)
    }

    #[inline]
    fn char_base_256(&self, map: u16) -> u32 {
        self.char_base + (u32::from(map & 0x3FF) << 6) + (self.tile_row(map) << 3)
    }
}

#[inline]
fn map_palette(map: u16) -> u32 {
    u32::from(map >> 12) << 4
}

/// Reverses the order of the eight 4bpp pixels in a tile row.
#[inline]
fn reverse_nibbles(data: u32) -> u32 {
    let data = data.swap_bytes();
    ((data & 0xF0F0_F0F0) >> 4) | ((data & 0x0F0F_0F0F) << 4)
}

/// Replicates a 4bpp pixel into all eight nibbles.
#[inline]
fn splat_nibble(data: u32) -> u32 {
    let data = data & 0xF;
    let data = data | (data << 4);
    let data = data | (data << 8);
    data | (data << 16)
}

/// Affine/bitmap iteration state (mGBA: `BACKGROUND_BITMAP_INIT`).
struct BitmapInit {
    /// Texture coordinates of the pixel before the segment start.
    x: i32,
    y: i32,
    /// Horizontal mosaic size minus one (0 without mosaic).
    mosaic_h: i32,
    mosaic_wait: i32,
    /// With mosaic: texture coordinates of the mosaic block the segment
    /// starts in.
    local_x: i32,
    local_y: i32,
}

impl Ppu {
    pub(crate) fn bg_compositor(&self, index: usize) -> BgCompositor {
        let bg = &self.bg[index];
        let objwin_slow_path = self.objwin_enabled();
        let objwin_palette = if objwin_slow_path
            && bg.target1
            && self.objwin.blend_enable()
            && self.blend_effect.is_brightness()
        {
            VARIANT
        } else {
            NORMAL
        };
        BgCompositor {
            flags: bg.flags,
            objwin_flags: bg.objwin_flags,
            palette: if bg.variant { VARIANT } else { NORMAL },
            objwin_palette,
            objwin_slow_path,
            objwin_force_enable: bg.objwin_force_enable,
            objwin_only: bg.objwin_only,
            blend: bg.flags & FLAG_TARGET_2 != 0,
        }
    }

    /// Composites BG palette entry `index` at `x` (mGBA: `COMPOSITE_16_*` /
    /// `COMPOSITE_256_*`).
    #[inline]
    fn composite_bg_index(&mut self, x: usize, index: usize, c: &BgCompositor) {
        let current = self.row[x];
        // Behind a reblended sprite pixel the layer is drawn unadjusted: the
        // brightness effect is applied to the sprite after compositing.
        let reblend_behind = current & (FLAG_IS_BACKGROUND | FLAG_REBLEND) == FLAG_REBLEND;
        let (eva, evb) = (self.blda, self.bldb);
        if c.objwin_slow_path {
            if !c.visible(current) {
                return;
            }
            let (color, flags) = if current & FLAG_OBJWIN != 0 {
                (self.palettes[c.objwin_palette][index], c.objwin_flags)
            } else if reblend_behind {
                (self.palettes[NORMAL][index], c.flags)
            } else {
                (self.palettes[c.palette][index], c.flags)
            };
            let color = u32::from(color) | flags;
            self.row[x] = if c.blend {
                blend_objwin(color, current, eva, evb)
            } else {
                no_blend_objwin(color, current)
            };
        } else {
            let palette = if reblend_behind { NORMAL } else { c.palette };
            let color = u32::from(self.palettes[palette][index]) | c.flags;
            self.row[x] = if c.blend {
                blend_no_objwin(color, current, eva, evb)
            } else {
                no_blend_no_objwin(color, current)
            };
        }
    }

    /// Draws one indexed pixel; `pixel_data` 0 is transparent (mGBA:
    /// `BACKGROUND_DRAW_PIXEL_16/256`).
    #[inline]
    fn draw_bg_pixel(&mut self, x: i32, pixel_data: u32, palette_data: u32, c: &BgCompositor) {
        let x = x as usize;
        if pixel_data != 0 && is_writable(self.row[x]) {
            self.composite_bg_index(x, (palette_data | pixel_data) as usize, c);
        }
    }

    /// Text background (mGBA: `GBAVideoSoftwareRendererDrawBackgroundMode0`).
    pub(crate) fn draw_background_mode0(&mut self, index: usize, y: i32, vram: &[u8]) {
        let bg = self.bg[index];
        let c = self.bg_compositor(index);
        let start = self.start as i32;
        let end = self.end as i32;
        let in_x = (start + i32::from(bg.x)) & 0x1FF;
        let mut length = end - start;
        let mut y = y;
        if bg.mosaic {
            y -= y % (self.mosaic_bg_v() + 1);
        }
        let in_y = y + i32::from(bg.y);
        let mut y_base = (in_y & 0xF8) as u32;
        match bg.size {
            2 => y_base += (in_y & 0x100) as u32,
            3 => y_base += ((in_y & 0x100) << 1) as u32,
            _ => {}
        }
        let line = TextLine {
            y_base: (bg.screen_base >> 1) + (y_base << 2),
            wide: bg.size & 1 != 0,
            char_base: bg.char_base,
            in_y,
        };

        if bg.mosaic && self.mosaic_bg_h() != 0 {
            let mosaic_h = self.mosaic_bg_h() + 1;
            if bg.multipalette {
                self.text_mosaic_256(vram, &line, &c, in_x, length, mosaic_h);
            } else {
                self.text_mosaic_16(vram, &line, &c, in_x, length, mosaic_h);
            }
            return;
        }

        let mut out_x = start;
        let mut tile_x = 0;
        let tile_end = ((length + in_x) >> 3) - (in_x >> 3);

        // Partial first tile.
        if in_x & 7 != 0 {
            let map = line.map_entry(vram, tile_x * 8 + in_x);
            let mod8 = in_x & 7;
            let suffix_end = (out_x + 8 - mod8).min(end);
            if suffix_end == out_x {
                return;
            }
            if bg.multipalette {
                self.text_suffix_256(vram, &line, &c, map, in_x, out_x, suffix_end);
            } else {
                self.text_suffix_16(vram, &line, &c, map, mod8, out_x, suffix_end);
            }
            out_x = suffix_end;
            if tile_x < tile_end {
                tile_x += 1;
            }
            length -= suffix_end - start;
        }

        // Whole tiles.
        let mut pixel = out_x;
        out_x += (tile_end - tile_x) * 8;
        let mut local_x = (tile_x * 8 + in_x) & 0x1FF;
        while tile_x < tile_end {
            let map = line.map_entry(vram, local_x);
            local_x += 8;
            if bg.multipalette {
                self.text_tile_256(vram, &line, &c, map, pixel);
            } else {
                self.text_tile_16(vram, &line, &c, map, pixel);
            }
            pixel += 8;
            tile_x += 1;
        }

        // Partial last tile.
        if length & 7 != 0 {
            let map = line.map_entry(vram, tile_x * 8 + in_x);
            let mod8 = length & 7;
            if bg.multipalette {
                self.text_prefix_256(vram, &line, &c, map, mod8, out_x);
            } else {
                self.text_prefix_16(vram, &line, &c, map, mod8, out_x);
            }
        }
    }

    /// First, partial tile of a segment, 4bpp (mGBA:
    /// `DRAW_BACKGROUND_MODE_0_TILE_SUFFIX_16`). Unlike whole tiles, tile
    /// data past BG VRAM is read rather than skipped. The flipped case
    /// assumes the tile's right edge is `end`, which is wrong when the
    /// segment ends inside the tile; that mGBA quirk is kept.
    #[allow(clippy::too_many_arguments)]
    fn text_suffix_16(
        &mut self,
        vram: &[u8],
        line: &TextLine,
        c: &BgCompositor,
        map: u16,
        mod8: i32,
        mut out_x: i32,
        end: i32,
    ) {
        let palette_data = map_palette(map);
        let mut tile_data = read_u32(vram, line.char_base_16(map) as usize);
        if map & MAP_HFLIP == 0 {
            tile_data >>= 4 * mod8;
            while out_x < end {
                self.draw_bg_pixel(out_x, tile_data & 0xF, palette_data, c);
                tile_data >>= 4;
                out_x += 1;
            }
        } else {
            let start = self.start as i32;
            let mut x = end - 1;
            while x >= start {
                self.draw_bg_pixel(x, tile_data & 0xF, palette_data, c);
                tile_data >>= 4;
                x -= 1;
            }
        }
    }

    /// First, partial tile of a segment, 8bpp (mGBA:
    /// `DRAW_BACKGROUND_MODE_0_TILE_SUFFIX_256`). Like the 4bpp version, it
    /// assumes the segment runs to the tile's right edge.
    #[allow(clippy::too_many_arguments)]
    fn text_suffix_256(
        &mut self,
        vram: &[u8],
        line: &TextLine,
        c: &BgCompositor,
        map: u16,
        in_x: i32,
        mut out_x: i32,
        end: i32,
    ) {
        let mut char_base = line.char_base_256(map);
        let end2 = end - 4;
        if map & MAP_HFLIP == 0 {
            let mut shift = in_x & 3;
            if char_base < 0x10000 && end2 > out_x {
                let mut tile_data = read_u32(vram, char_base as usize) >> (8 * shift);
                shift = 0;
                while out_x < end2 {
                    self.draw_bg_pixel(out_x, tile_data & 0xFF, 0, c);
                    tile_data >>= 8;
                    out_x += 1;
                }
            }
            if char_base < 0x10000 {
                let mut tile_data = read_u32(vram, char_base as usize + 4) >> (8 * shift);
                while out_x < end {
                    self.draw_bg_pixel(out_x, tile_data & 0xFF, 0, c);
                    tile_data >>= 8;
                    out_x += 1;
                }
            }
        } else {
            let first = out_x;
            let start = self.start as i32;
            let mut x = end - 1;
            if char_base < 0x10000 && end2 > first {
                let mut tile_data = read_u32(vram, char_base as usize);
                while x >= end2 {
                    self.draw_bg_pixel(x, tile_data & 0xFF, 0, c);
                    tile_data >>= 8;
                    x -= 1;
                }
                char_base += 4;
            }
            if char_base < 0x10000 {
                let mut tile_data = read_u32(vram, char_base as usize);
                while x >= start {
                    self.draw_bg_pixel(x, tile_data & 0xFF, 0, c);
                    tile_data >>= 8;
                    x -= 1;
                }
            }
        }
    }

    /// A whole 4bpp tile at `pixel..pixel + 8`. Tiles whose data would lie
    /// past BG VRAM (0x10000) are skipped.
    fn text_tile_16(
        &mut self,
        vram: &[u8],
        line: &TextLine,
        c: &BgCompositor,
        map: u16,
        pixel: i32,
    ) {
        let char_base = line.char_base_16(map);
        if char_base >= 0x10000 {
            return;
        }
        let palette_data = map_palette(map);
        let mut tile_data = read_u32(vram, char_base as usize);
        if map & MAP_HFLIP != 0 {
            tile_data = reverse_nibbles(tile_data);
        }
        if tile_data == 0 {
            return;
        }
        for x in pixel..pixel + 8 {
            self.draw_bg_pixel(x, tile_data & 0xF, palette_data, c);
            tile_data >>= 4;
        }
    }

    /// A whole 8bpp tile at `pixel..pixel + 8`.
    fn text_tile_256(
        &mut self,
        vram: &[u8],
        line: &TextLine,
        c: &BgCompositor,
        map: u16,
        pixel: i32,
    ) {
        let char_base = line.char_base_256(map) as usize;
        if char_base >= 0x10000 {
            return;
        }
        // Each half of the tile row is one word; flipping swaps the words
        // and draws each right to left.
        let (left, right) = if map & MAP_HFLIP == 0 {
            (read_u32(vram, char_base), read_u32(vram, char_base + 4))
        } else {
            (read_u32(vram, char_base + 4), read_u32(vram, char_base))
        };
        let flip = map & MAP_HFLIP != 0;
        for (half, mut tile_data) in [left, right].into_iter().enumerate() {
            if tile_data == 0 {
                continue;
            }
            let base = pixel + 4 * half as i32;
            for i in 0..4 {
                let x = if flip { base + 3 - i } else { base + i };
                self.draw_bg_pixel(x, tile_data & 0xFF, 0, c);
                tile_data >>= 8;
            }
        }
    }

    /// Last, partial tile of a segment, 4bpp (mGBA:
    /// `DRAW_BACKGROUND_MODE_0_TILE_PREFIX_16`): tile pixels `0..mod8` at
    /// `out_x..end`. Tile data past BG VRAM is read, not skipped.
    fn text_prefix_16(
        &mut self,
        vram: &[u8],
        line: &TextLine,
        c: &BgCompositor,
        map: u16,
        mod8: i32,
        mut out_x: i32,
    ) {
        let end = self.end as i32;
        debug_assert!(out_x >= self.start as i32);
        let palette_data = map_palette(map);
        let mut tile_data = read_u32(vram, line.char_base_16(map) as usize);
        if map & MAP_HFLIP == 0 {
            while out_x < end {
                self.draw_bg_pixel(out_x, tile_data & 0xF, palette_data, c);
                tile_data >>= 4;
                out_x += 1;
            }
        } else {
            // Draws a full tile width leftward; pixels beyond the segment
            // have been shifted out and are transparent.
            tile_data >>= 4 * (8 - mod8);
            let stop = (end - 8).max(-1);
            let mut x = end - 1;
            while x > stop {
                self.draw_bg_pixel(x, tile_data & 0xF, palette_data, c);
                tile_data >>= 4;
                x -= 1;
            }
        }
    }

    /// Last, partial tile of a segment, 8bpp (mGBA:
    /// `DRAW_BACKGROUND_MODE_0_TILE_PREFIX_256`).
    fn text_prefix_256(
        &mut self,
        vram: &[u8],
        line: &TextLine,
        c: &BgCompositor,
        map: u16,
        mod8: i32,
        mut out_x: i32,
    ) {
        let mut char_base = line.char_base_256(map) as usize;
        if char_base >= 0x10000 {
            return;
        }
        let end = self.end as i32;
        let first_half = mod8 - 4;
        if map & MAP_HFLIP == 0 {
            if first_half > 0 {
                let mut tile_data = read_u32(vram, char_base);
                while out_x < end - first_half {
                    self.draw_bg_pixel(out_x, tile_data & 0xFF, 0, c);
                    tile_data >>= 8;
                    out_x += 1;
                }
                char_base += 4;
            }
            let mut tile_data = read_u32(vram, char_base);
            while out_x < end {
                self.draw_bg_pixel(out_x, tile_data & 0xFF, 0, c);
                tile_data >>= 8;
                out_x += 1;
            }
        } else {
            let mut shift = (8 - mod8) & 3;
            let first = out_x;
            let mut x = end - 1;
            if first_half > 0 {
                let mut tile_data = read_u32(vram, char_base) >> (8 * shift);
                while x >= first + 4 {
                    self.draw_bg_pixel(x, tile_data & 0xFF, 0, c);
                    tile_data >>= 8;
                    x -= 1;
                }
                shift = 0;
            }
            let mut tile_data = read_u32(vram, char_base + 4) >> (8 * shift);
            while x >= first {
                self.draw_bg_pixel(x, tile_data & 0xFF, 0, c);
                tile_data >>= 8;
                x -= 1;
            }
        }
    }

    /// Where the pixel carried into a segment that starts inside a mosaic
    /// block comes from (mGBA: the `mosaicWait` prologue of
    /// `DRAW_BACKGROUND_MODE_0_MOSAIC_*`). Returns the map entry and the
    /// offset of the block's first pixel relative to its tile.
    ///
    /// When the block starts more than one tile back (mosaic sizes above 9),
    /// mGBA stays in the current tile with a negative offset; callers
    /// reproduce the resulting out-of-range shift with `wrapping_shr`,
    /// which is what that C code does on x86 and ARM.
    fn mosaic_carry_source(
        vram: &[u8],
        line: &TextLine,
        in_x: i32,
        mosaic_h: i32,
        mosaic_wait: i32,
    ) -> (u16, i32) {
        let x = in_x & 7;
        let mut base_x = x - (mosaic_h - mosaic_wait);
        if base_x < 0 {
            let disturb_x = (16 + base_x) >> 3;
            let map = line.map_entry(vram, in_x - (disturb_x << 3));
            base_x -= disturb_x << 3;
            (map, base_x)
        } else {
            (line.map_entry(vram, in_x), base_x)
        }
    }

    /// Horizontally mosaicked 4bpp text background: each block repeats the
    /// pixel at its left edge.
    fn text_mosaic_16(
        &mut self,
        vram: &[u8],
        line: &TextLine,
        c: &BgCompositor,
        in_x: i32,
        mut length: i32,
        mosaic_h: i32,
    ) {
        let mut pixel = self.start as i32;
        let mut mosaic_wait = (mosaic_h - pixel + 240 * mosaic_h) % mosaic_h;
        let mut carry_data = 0u32;
        let mut palette_data = 0u32;
        let mut x = in_x & 7;
        if mosaic_wait != 0 {
            let (map, base_x) = Self::mosaic_carry_source(vram, line, in_x, mosaic_h, mosaic_wait);
            let char_base = line.char_base_16(map);
            if char_base >= 0x10000 {
                carry_data = 0;
            } else {
                palette_data = map_palette(map);
                let shift = if map & MAP_HFLIP == 0 {
                    4 * base_x
                } else {
                    4 * (7 - base_x)
                };
                carry_data =
                    splat_nibble(read_u32(vram, char_base as usize).wrapping_shr(shift as u32));
            }
        }
        let mut local_x = in_x;
        while length != 0 {
            let map = line.map_entry(vram, local_x);
            local_x += 8;
            let char_base = line.char_base_16(map);
            let mut tile_data = carry_data;
            while x < 8 && length != 0 {
                if mosaic_wait == 0 {
                    if char_base >= 0x10000 {
                        // mGBA keeps drawing the stale `tile_data` here.
                        carry_data = 0;
                    } else {
                        palette_data = map_palette(map);
                        let shift = if map & MAP_HFLIP == 0 {
                            x * 4
                        } else {
                            (7 - x) * 4
                        };
                        tile_data = splat_nibble(read_u32(vram, char_base as usize) >> shift);
                        carry_data = tile_data;
                    }
                    mosaic_wait = mosaic_h;
                }
                mosaic_wait -= 1;
                self.draw_bg_pixel(pixel, tile_data & 0xF, palette_data, c);
                tile_data >>= 4;
                pixel += 1;
                x += 1;
                length -= 1;
            }
            x = 0;
        }
    }

    /// Horizontally mosaicked 8bpp text background.
    fn text_mosaic_256(
        &mut self,
        vram: &[u8],
        line: &TextLine,
        c: &BgCompositor,
        in_x: i32,
        mut length: i32,
        mosaic_h: i32,
    ) {
        /// Pixel `x` of an 8bpp tile row, honoring horizontal flip.
        fn texel(vram: &[u8], char_base: u32, map: u16, x: i32) -> u32 {
            let char_base = char_base as usize;
            let data = if map & MAP_HFLIP == 0 {
                if x >= 4 {
                    read_u32(vram, char_base + 4) >> ((x - 4) * 8)
                } else {
                    read_u32(vram, char_base) >> (x * 8)
                }
            } else if x >= 4 {
                read_u32(vram, char_base) >> ((7 - x) * 8)
            } else {
                read_u32(vram, char_base + 4) >> ((3 - x) * 8)
            };
            data & 0xFF
        }

        let mut pixel = self.start as i32;
        let mut mosaic_wait = (mosaic_h - pixel + 240 * mosaic_h) % mosaic_h;
        let mut carry_data = 0u32;
        let mut x = in_x & 7;
        if mosaic_wait != 0 {
            // mGBA picks the carried block's tile but then reads pixel `x`
            // (the segment start's position) instead of the block's first
            // pixel from it.
            let (map, _) = Self::mosaic_carry_source(vram, line, in_x, mosaic_h, mosaic_wait);
            let char_base = line.char_base_256(map);
            carry_data = if char_base >= 0x10000 {
                0
            } else {
                texel(vram, char_base, map, x)
            };
        }
        let mut local_x = in_x;
        while length != 0 {
            let map = line.map_entry(vram, local_x);
            local_x += 8;
            let char_base = line.char_base_256(map);
            let mut tile_data = carry_data;
            while x < 8 && length != 0 {
                if mosaic_wait == 0 {
                    if char_base >= 0x10000 {
                        carry_data = 0;
                    } else {
                        tile_data = texel(vram, char_base, map, x);
                        carry_data = tile_data;
                    }
                    mosaic_wait = mosaic_h;
                }
                tile_data |= tile_data << 8;
                mosaic_wait -= 1;
                self.draw_bg_pixel(pixel, tile_data & 0xFF, 0, c);
                tile_data >>= 8;
                pixel += 1;
                x += 1;
                length -= 1;
            }
            x = 0;
        }
    }

    fn bitmap_init(&self, bg: &Background, in_y: i32) -> BitmapInit {
        let start = self.start as i32;
        let (dx, dy) = (i32::from(bg.dx), i32::from(bg.dy));
        let mut init = BitmapInit {
            x: bg.sx + (start - 1) * dx,
            y: bg.sy + (start - 1) * dy,
            mosaic_h: 0,
            mosaic_wait: 0,
            local_x: 0,
            local_y: 0,
        };
        if bg.mosaic {
            let mosaic_v = self.mosaic_bg_v() + 1;
            let mosaic_h = self.mosaic_bg_h() + 1;
            init.mosaic_wait = (mosaic_h - start + 240 * mosaic_h) % mosaic_h;
            let start_x = start - start % mosaic_h;
            init.mosaic_h = mosaic_h - 1;
            // Vertical mosaic rewinds the reference point to the first
            // scanline of the block, assuming it advanced by PB/PD per line.
            let rewind = in_y % mosaic_v;
            init.local_x = -rewind * i32::from(bg.dmx);
            init.local_y = -rewind * i32::from(bg.dmy);
            init.x += init.local_x;
            init.y += init.local_y;
            init.local_x += bg.sx + start_x * dx;
            init.local_y += bg.sy + start_x * dy;
        }
        init
    }

    /// Affine (rotation/scaling) background (mGBA:
    /// `GBAVideoSoftwareRendererDrawBackgroundMode2`).
    ///
    /// Screen pixel `x` samples texture coordinate `(sx + x * PA, sy + x *
    /// PC)` (20.8 fixed point), where `(sx, sy)` is the internal reference
    /// point for this scanline. Outside the map the layer is transparent,
    /// or wraps when BGCNT bit 13 is set. Maps are 8bpp with one-byte
    /// entries.
    pub(crate) fn draw_background_mode2(&mut self, index: usize, in_y: i32, vram: &[u8]) {
        let bg = self.bg[index];
        let c = self.bg_compositor(index);
        let mask = (0x8000 << bg.size) - 1;
        let BitmapInit {
            mut x,
            mut y,
            mosaic_h,
            mut mosaic_wait,
            mut local_x,
            mut local_y,
        } = self.bitmap_init(&bg, in_y);
        let (dx, dy) = (i32::from(bg.dx), i32::from(bg.dy));
        let fetch = |local_x: i32, local_y: i32| -> u32 {
            let map_index = (local_x >> 11) as u32 + ((((local_y >> 7) & 0x7F0) as u32) << bg.size);
            let map = u32::from(vram[(bg.screen_base + map_index) as usize]);
            let texel =
                (map << 6) + (((local_y & 0x700) >> 5) as u32) + (((local_x & 0x700) >> 8) as u32);
            u32::from(vram[(bg.char_base + texel) as usize])
        };

        let mosaic = mosaic_h > 1;
        let mut pixel_data = 0;
        if mosaic && (bg.overflow || (x | y) & !mask == 0) {
            local_x &= mask;
            local_y &= mask;
            pixel_data = fetch(local_x, local_y);
        }
        for out_x in self.start..self.end {
            x += dx;
            y += dy;
            let current = self.row[out_x];
            if !mosaic || mosaic_wait == 0 {
                if bg.overflow {
                    local_x = x & mask;
                    local_y = y & mask;
                } else {
                    if (x | y) & !mask != 0 {
                        continue;
                    }
                    local_x = x;
                    local_y = y;
                }
                pixel_data = fetch(local_x, local_y);
                if mosaic {
                    mosaic_wait = mosaic_h;
                }
            } else {
                mosaic_wait -= 1;
            }
            if pixel_data != 0 && is_writable(current) {
                self.composite_bg_index(out_x, pixel_data as usize, &c);
            }
        }
    }

    /// Mode 3 (240x160 direct color) and mode 5 (160x128 direct color, two
    /// frames) bitmaps. Direct colors have no transparency.
    pub(crate) fn draw_background_mode3_5(&mut self, index: usize, in_y: i32, vram: &[u8]) {
        let bg = self.bg[index];
        let c = self.bg_compositor(index);
        let (width, height, offset) = if self.mode() == 3 {
            (240, 160, 0)
        } else {
            (160, 128, if self.dispcnt & 0x10 != 0 { 0xA000 } else { 0 })
        };
        let load = |local_x: i32, local_y: i32| -> u32 {
            let texel = (local_x >> 8) + (local_y >> 8) * width;
            u32::from(read_u16(vram, offset + 2 * texel as usize) & 0x7FFF)
        };
        let in_bounds = |x: i32, y: i32| x >= 0 && y >= 0 && (x >> 8) < width && (y >> 8) < height;

        let BitmapInit {
            mut x,
            mut y,
            mosaic_h,
            mut mosaic_wait,
            local_x,
            local_y,
        } = self.bitmap_init(&bg, in_y);
        let (dx, dy) = (i32::from(bg.dx), i32::from(bg.dy));
        let mut color = u32::from(self.palettes[NORMAL][0]);
        if mosaic_wait != 0 && in_bounds(local_x, local_y) {
            color = load(local_x, local_y);
        }
        for out_x in self.start..self.end {
            x += dx;
            y += dy;
            if !in_bounds(x, y) && mosaic_wait == 0 {
                continue;
            }
            if mosaic_wait == 0 {
                color = load(x, y);
                mosaic_wait = mosaic_h;
            } else {
                mosaic_wait -= 1;
            }
            let current = self.row[out_x];
            // Note the inverted OBJ window test compared to the other modes;
            // mGBA's OBJ window support for direct-color modes is incomplete.
            if !c.objwin_slow_path || (current & FLAG_OBJWIN == 0) != bg.objwin_only {
                let flags = if current & FLAG_OBJWIN != 0 {
                    bg.objwin_flags
                } else {
                    bg.flags
                };
                let adjusted = if !bg.variant {
                    color
                } else if self.blend_effect == BlendEffect::Brighten {
                    brighten(color, self.bldy)
                } else {
                    darken(color, self.bldy)
                };
                self.row[out_x] = blend_objwin(adjusted | flags, current, self.blda, self.bldb);
            }
        }
    }

    /// Mode 4: 240x160 8bpp paletted bitmap, two frames.
    pub(crate) fn draw_background_mode4(&mut self, index: usize, in_y: i32, vram: &[u8]) {
        let bg = self.bg[index];
        let c = self.bg_compositor(index);
        let offset = if self.dispcnt & 0x10 != 0 { 0xA000 } else { 0 };
        let load = |x: i32, y: i32| -> usize {
            usize::from(vram[offset + ((x >> 8) + (y >> 8) * 240) as usize])
        };
        let in_bounds = |x: i32, y: i32| x >= 0 && y >= 0 && (x >> 8) < 240 && (y >> 8) < 160;

        let BitmapInit {
            mut x,
            mut y,
            mosaic_h,
            mut mosaic_wait,
            local_x,
            local_y,
        } = self.bitmap_init(&bg, in_y);
        let (dx, dy) = (i32::from(bg.dx), i32::from(bg.dy));
        let mut color = 0;
        if mosaic_wait != 0 && in_bounds(local_x, local_y) {
            color = load(local_x, local_y);
        }
        for out_x in self.start..self.end {
            x += dx;
            y += dy;
            if !in_bounds(x, y) && mosaic_wait == 0 {
                continue;
            }
            if mosaic_wait == 0 {
                color = load(x, y);
                mosaic_wait = mosaic_h;
            } else {
                mosaic_wait -= 1;
            }
            let current = self.row[out_x];
            if color == 0 || !is_writable(current) {
                continue;
            }
            if !c.objwin_slow_path {
                let value = u32::from(self.palettes[c.palette][color]) | c.flags;
                self.row[out_x] = blend_no_objwin(value, current, self.blda, self.bldb);
            } else if c.visible(current) {
                let (palette, flags) = if current & FLAG_OBJWIN != 0 {
                    (c.objwin_palette, c.objwin_flags)
                } else {
                    (c.palette, c.flags)
                };
                let value = u32::from(self.palettes[palette][color]) | flags;
                self.row[out_x] = blend_objwin(value, current, self.blda, self.bldb);
            }
        }
    }
}
