// Derived from mGBA (src/gba/renderers/software-obj.c, common.c and
// video-software.c), Copyright (c) 2013-2019 Jeffrey Pfau, licensed under the
// Mozilla Public License, v. 2.0. This file is therefore also under the
// MPL-2.0: https://mozilla.org/MPL/2.0/.

//! Sprites (OBJs).
//!
//! Sprites are rendered in two passes per scanline. First, every sprite on
//! the line is drawn in OAM order into a separate sprite layer
//! (`Ppu::sprite_layer`), resolving sprite-vs-sprite priority there; OBJ
//! window sprites instead mark the scanline buffer. Then, while compositing,
//! the sprite layer is merged into the scanline buffer once per priority
//! level, ahead of the backgrounds of the same priority.

use crate::Ppu;
use crate::composite::*;
use crate::state::BlendEffect;
use crate::{NORMAL, VARIANT, read_u16};

/// Width and height by `shape * 4 + size`; shape 3 is invalid.
const OBJ_SIZES: [(i32, i32); 16] = [
    (8, 8),
    (16, 16),
    (32, 32),
    (64, 64),
    (16, 8),
    (32, 8),
    (32, 16),
    (64, 32),
    (8, 16),
    (8, 32),
    (16, 32),
    (32, 64),
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 0),
];

/// OBJ rendering time per scanline, in cycles; sprites past the budget are
/// dropped. H-blank interval free (DISPCNT bit 5) shortens it.
const OBJ_LENGTH: i32 = 1210;
const OBJ_HBLANK_FREE_LENGTH: i32 = 954;

const ATTR0_AFFINE: u16 = 0x0100;
const ATTR0_DISABLE_OR_DOUBLE: u16 = 0x0200;
const ATTR0_MOSAIC: u16 = 0x1000;
const ATTR0_256_COLOR: u16 = 0x2000;
const ATTR1_HFLIP: u16 = 0x1000;
const ATTR1_VFLIP: u16 = 0x2000;

const OBJ_MODE_SEMITRANSPARENT: u16 = 1;
const OBJ_MODE_OBJWIN: u16 = 2;

/// A sprite that may be visible this frame (mGBA:
/// `GBAVideoRendererSprite`), with its vertical extent resolved.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Sprite {
    a: u16,
    b: u16,
    c: u16,
    /// First scanline (negative if the sprite wraps from the bottom).
    y: i32,
    end_y: i32,
    /// Rendering cycles the sprite costs on each line it covers.
    cycles: i32,
    index: i32,
}

impl Sprite {
    fn size(&self) -> (i32, i32) {
        OBJ_SIZES[usize::from((self.a >> 14) * 4 + (self.b >> 14))]
    }

    fn obj_y(&self) -> i32 {
        i32::from(self.a & 0xFF)
    }

    /// Signed 9-bit X.
    fn obj_x(&self) -> i32 {
        (i32::from(self.b & 0x1FF) << 23) >> 23
    }

    fn mode(&self) -> u16 {
        (self.a >> 10) & 3
    }

    fn priority(&self) -> u32 {
        u32::from((self.c >> 10) & 3)
    }

    fn is_256_color(&self) -> bool {
        self.a & ATTR0_256_COLOR != 0
    }
}

/// How a sprite's pixels are written (mGBA: the `SPRITE_DRAW_PIXEL_*`
/// variants).
#[derive(Clone, Copy, PartialEq, Eq)]
enum DrawKind {
    Normal,
    /// Normal sprite while the OBJ window is enabled with a different blend
    /// enable: pixels already marked as OBJ window use `objwin_palette`.
    NormalObjwin,
    /// OBJ window sprite: marks the scanline buffer instead of drawing.
    Objwin,
}

/// Everything needed to fetch and draw one sprite's texels.
struct SpriteDraw {
    kind: DrawKind,
    flags: u32,
    /// `(palette selector, base index)`.
    palette: (usize, usize),
    objwin_palette: (usize, usize),
    is_256: bool,
    char_base: u32,
    /// Row stride in bytes per 8 pixel rows.
    stride: u32,
    mask_lo: u32,
    mask_hi: u32,
}

impl SpriteDraw {
    /// Palette index of texel `(local_x, local_y)`, 0 for transparent.
    #[inline]
    fn texel(&self, vram: &[u8], local_x: i32, local_y: i32) -> u32 {
        let (x_base, y_base, shift) = if self.is_256 {
            (
                ((local_x & !7) * 8 + (local_x & 6)) as u32,
                ((local_y & !7) as u32).wrapping_mul(self.stride) + ((local_y & 7) * 8) as u32,
                (local_x & 1) << 3,
            )
        } else {
            (
                ((local_x & !7) * 4 + ((local_x >> 1) & 2)) as u32,
                ((local_y & !7) as u32).wrapping_mul(self.stride) + ((local_y & 7) * 4) as u32,
                (local_x & 3) << 2,
            )
        };
        let y_base = y_base.wrapping_add(self.mask_hi);
        let address =
            y_base.wrapping_add(x_base.wrapping_add(self.char_base) & self.mask_lo) & 0x7FFE;
        let data = u32::from(read_u16(vram, 0x10000 + address as usize)) >> shift;
        data & if self.is_256 { 0xFF } else { 0xF }
    }
}

impl Ppu {
    /// Collects the sprites that can appear this frame (mGBA:
    /// `GBAVideoRendererCleanOAM`).
    pub(crate) fn clean_oam(&mut self, oam: &[u8]) {
        self.oam_max = 0;
        for index in 0..128 {
            let entry = &oam[index * 8..];
            let mut sprite = Sprite {
                a: read_u16(entry, 0),
                b: read_u16(entry, 2),
                c: read_u16(entry, 4),
                index: index as i32,
                ..Sprite::default()
            };
            let affine = sprite.a & ATTR0_AFFINE != 0;
            if !affine && sprite.a & ATTR0_DISABLE_OR_DOUBLE != 0 {
                continue;
            }
            let (mut width, mut height) = sprite.size();
            let x = sprite.obj_x();
            if affine {
                if sprite.a & ATTR0_DISABLE_OR_DOUBLE != 0 {
                    width <<= 1;
                    height <<= 1;
                }
                sprite.cycles = 8 + width * 2 + x.min(0);
            } else {
                sprite.cycles = width - 2;
                if x < 0 {
                    if x + width < 0 {
                        continue;
                    }
                    sprite.cycles += x >> 1;
                }
            }
            let obj_y = sprite.obj_y();
            if obj_y >= 160 && obj_y + height < 228 {
                continue;
            }
            let obj_x = i32::from(sprite.b & 0x1FF);
            if obj_x >= 240 && obj_x + width < 512 {
                continue;
            }
            sprite.y = if obj_y + height > 256 {
                obj_y - 256
            } else {
                obj_y
            };
            sprite.end_y = sprite.y + height;
            self.sprites[self.oam_max] = sprite;
            self.oam_max += 1;
        }
    }

    /// Draws every sprite on scanline `y` into the sprite layer, returning
    /// a bitmask of the priorities that received pixels (mGBA:
    /// `GBAVideoSoftwareRendererPreprocessSpriteLayer`).
    pub(crate) fn preprocess_sprite_layer(&mut self, y: i32, vram: &[u8], oam: &[u8]) -> u32 {
        let mut sprite_layers = 0;
        if self.dispcnt & 0x1000 == 0 {
            return sprite_layers;
        }
        let mosaic_v = self.mosaic_obj_v() + 1;
        let mosaic_y = y - y % mosaic_v;
        let mut last_index = 0;
        for i in 0..self.oam_max {
            let sprite = self.sprites[i];
            let mut local_y = y;
            self.end = 0;
            // Scanning OAM costs cycles even for sprites that are skipped.
            self.sprite_cycles_remaining -= 2 * (sprite.index - last_index);
            last_index = sprite.index;
            if self.sprite_cycles_remaining <= 0 {
                break;
            }
            if y < sprite.y || y >= sprite.end_y {
                continue;
            }
            if sprite.a & ATTR0_MOSAIC != 0 && mosaic_v > 1 {
                local_y = mosaic_y;
                if local_y < sprite.y && sprite.y < 160 {
                    local_y = sprite.y;
                }
                if local_y >= (sprite.end_y & 0xFF) {
                    local_y = sprite.end_y - 1;
                }
            }
            for w in 0..self.n_windows {
                self.current_window = self.windows[w].control;
                self.start = self.end;
                self.end = usize::from(self.windows[w].end_x);
                if !self.current_window.obj_enable() && !self.objwin_enabled() {
                    continue;
                }
                if self.preprocess_sprite(&sprite, local_y, vram, oam) {
                    sprite_layers |= 1 << sprite.priority();
                }
            }
            self.sprite_cycles_remaining -= sprite.cycles;
        }
        sprite_layers
    }

    /// Resets the per-scanline sprite cycle budget.
    pub(crate) fn reset_sprite_cycles(&mut self) {
        self.sprite_cycles_remaining = if self.dispcnt & 0x20 != 0 {
            OBJ_HBLANK_FREE_LENGTH
        } else {
            OBJ_LENGTH
        };
    }

    /// Draws one sprite's row `y` within the current window segment (mGBA:
    /// `GBAVideoSoftwareRendererPreprocessSprite`).
    fn preprocess_sprite(&mut self, sprite: &Sprite, y: i32, vram: &[u8], oam: &[u8]) -> bool {
        let (width, height) = sprite.size();
        let start = self.start as i32;
        let end = self.end as i32;
        let window_blend = self.current_window.blend_enable();
        let mut flags = sprite.priority() << OFFSET_PRIORITY;
        if (window_blend && self.target1_obj && self.blend_effect == BlendEffect::Alpha)
            || sprite.mode() == OBJ_MODE_SEMITRANSPARENT
        {
            flags |= FLAG_TARGET_1;
        }
        if sprite.mode() == OBJ_MODE_OBJWIN {
            flags |= FLAG_OBJWIN;
            // OBJ window sprites only act where no higher-precedence window
            // (WIN0/WIN1) covers the pixel.
            if self.current_window.priority < self.objwin.priority {
                return false;
            }
        }
        let x = sprite.obj_x();
        let mapping_1d = self.dispcnt & 0x40 != 0;
        let tile = u32::from(sprite.c & 0x3FF);
        let align = u32::from(sprite.is_256_color() && !mapping_1d);
        let char_base = (tile & !align) * 0x20;
        // In bitmap modes the lower half of OBJ VRAM holds the frame buffer.
        if self.mode() >= 3 && tile < 512 {
            return false;
        }

        // Meant as "the OBJ window's blend enable differs from this
        // window's", but mGBA compares `GetBlendEnable` (0/1) with
        // `IsBlendEnable` (0/0x20), so it holds whenever either is set.
        let objwin_slow_path =
            self.objwin_enabled() && (self.objwin.blend_enable() || window_blend);
        let mut variant = self.target1_obj && window_blend && self.blend_effect.is_brightness();
        if sprite.mode() == OBJ_MODE_SEMITRANSPARENT
            || (self.target1_obj && self.blend_effect == BlendEffect::Alpha)
            || objwin_slow_path
        {
            let target2 = self.target2_bd || self.bg.iter().any(|bg| bg.target2 && bg.enabled != 0);
            if target2 {
                // Blend against whatever ends up behind; any brightness
                // effect is applied afterwards (see `postprocess_buffer`).
                self.force_target1 = true;
                flags |= FLAG_REBLEND;
                variant = false;
            } else {
                flags &= !FLAG_TARGET_1;
            }
        }

        let mut palette = (NORMAL, 0x100);
        let mut objwin_palette = palette;
        if variant {
            palette.0 = VARIANT;
            if self.objwin.blend_enable() {
                objwin_palette = palette;
            }
        }
        if !sprite.is_256_color() {
            let bank = usize::from(sprite.c >> 12) << 4;
            palette.1 += bank;
            objwin_palette.1 += bank;
        }
        let kind = if flags & FLAG_OBJWIN != 0 {
            DrawKind::Objwin
        } else if objwin_slow_path {
            DrawKind::NormalObjwin
        } else {
            DrawKind::Normal
        };
        let draw = SpriteDraw {
            kind,
            flags,
            palette,
            objwin_palette,
            is_256: sprite.is_256_color(),
            char_base,
            stride: if mapping_1d {
                (width >> u32::from(!sprite.is_256_color())) as u32
            } else {
                0x80
            },
            mask_lo: if mapping_1d { 0x7FFE } else { 0x3FE },
            mask_hi: if mapping_1d { 0 } else { char_base & 0x7C00 },
        };
        let mut in_y = y - sprite.obj_y();
        // OBJ window sprites ignore horizontal mosaic but still get the
        // mosaic-extended width.
        let mosaic_h = if sprite.a & ATTR0_MOSAIC != 0 {
            self.mosaic_obj_h() + 1
        } else {
            1
        };
        let mosaic_loop = mosaic_h > 1 && kind != DrawKind::Objwin;

        if sprite.a & ATTR0_AFFINE != 0 {
            let double = i32::from(sprite.a & ATTR0_DISABLE_OR_DOUBLE != 0);
            let total_width = width << double;
            let total_height = height << double;
            let matrix = &oam[usize::from((sprite.b >> 9) & 0x1F) * 32..];
            let [pa, pb, pc, pd] =
                [6, 14, 22, 30].map(|offset| i32::from(read_u16(matrix, offset) as i16));
            if in_y < 0 {
                in_y += 256;
            }
            let mut out_x = x.max(start);
            let mut condition = (x + total_width).min(end);
            if mosaic_h > 1 && condition != end && condition % mosaic_h != 0 {
                condition += mosaic_h - condition % mosaic_h;
            }
            // mGBA can run past the scanline here; it has no pixels there.
            let condition = condition.min(240);

            // Texture coordinates (20.8) of the pixel before `out_x`: the
            // matrix maps screen offsets from the bounding box's center to
            // texture offsets from the sprite's center.
            let in_x = out_x - x;
            let mut x_accum = pa * (in_x - 1 - (total_width >> 1))
                + pb * (in_y - (total_height >> 1))
                + (width << 7);
            let mut y_accum = pc * (in_x - 1 - (total_width >> 1))
                + pd * (in_y - (total_height >> 1))
                + (height << 7);

            // Skip ahead to just before the row enters the texture, first
            // horizontally, then vertically. The texture is convex, so the
            // row covers it in one run and drawing stops at the first texel
            // outside it.
            if pa != 0 {
                let n = if (x_accum >> 8) < 0 {
                    (-x_accum - 1) / pa
                } else if (x_accum >> 8) >= width {
                    ((width << 8) - x_accum) / pa
                } else {
                    0
                };
                x_accum += pa * n;
                y_accum += pc * n;
                out_x += n;
            }
            if pc != 0 {
                let n = if (y_accum >> 8) < 0 {
                    (-y_accum - 1) / pc
                } else if (y_accum >> 8) >= height {
                    ((height << 8) - y_accum) / pc
                } else {
                    0
                };
                x_accum += pa * n;
                y_accum += pc * n;
                out_x += n;
            }
            if out_x < start || out_x >= condition {
                return false;
            }

            let outside = |local_x: i32, local_y: i32| {
                (local_x as u32) & !(width as u32 - 1) != 0
                    || (local_y as u32) & !(height as u32 - 1) != 0
            };
            if mosaic_loop {
                let mut local_x = x_accum >> 8;
                let mut local_y = y_accum >> 8;
                while out_x < condition {
                    x_accum += pa;
                    y_accum += pc;
                    if out_x % mosaic_h == 0 {
                        local_x = x_accum >> 8;
                        local_y = y_accum >> 8;
                    }
                    if !outside(local_x, local_y) {
                        let texel = draw.texel(vram, local_x, local_y);
                        self.draw_sprite_pixel(&draw, out_x as usize, texel);
                    }
                    out_x += 1;
                }
            } else {
                while out_x < condition {
                    x_accum += pa;
                    y_accum += pc;
                    let local_x = x_accum >> 8;
                    let local_y = y_accum >> 8;
                    if outside(local_x, local_y) {
                        break;
                    }
                    let texel = draw.texel(vram, local_x, local_y);
                    self.draw_sprite_pixel(&draw, out_x as usize, texel);
                    out_x += 1;
                }
            }
        } else {
            let mut out_x = x.max(start);
            let mut condition = x + width;
            if mosaic_h > 1 && condition % mosaic_h != 0 {
                condition += mosaic_h - condition % mosaic_h;
            }
            if sprite.obj_y() + height - 256 >= 0 {
                in_y += 256;
            }
            if sprite.b & ATTR1_VFLIP != 0 {
                in_y = height - in_y - 1;
            }
            let condition = condition.min(end);
            let (mut in_x, x_offset) = if sprite.b & ATTR1_HFLIP != 0 {
                (width - (out_x - x) - 1, -1)
            } else {
                (out_x - x, 1)
            };
            while out_x < condition {
                let local_x = if mosaic_loop {
                    (in_x - x_offset * (out_x % mosaic_h)).clamp(0, width - 1)
                } else {
                    in_x
                };
                let texel = draw.texel(vram, local_x, in_y);
                self.draw_sprite_pixel(&draw, out_x as usize, texel);
                out_x += 1;
                in_x += x_offset;
            }
        }
        true
    }

    /// Writes one sprite texel into the sprite layer (mGBA:
    /// `SPRITE_DRAW_PIXEL_*`).
    ///
    /// A sprite pixel replaces an earlier one only if its priority is
    /// strictly better. Transparent texels of a better-priority sprite still
    /// lend their priority (and blend flags) to an opaque pixel already
    /// there, a hardware quirk mGBA reproduces.
    #[inline]
    fn draw_sprite_pixel(&mut self, draw: &SpriteDraw, out_x: usize, texel: u32) {
        const INHERITED: u32 = FLAG_ORDER_MASK | FLAG_REBLEND | FLAG_TARGET_1;
        let current = self.sprite_layer[out_x];
        let inherit = |current: u32| (current & !INHERITED) | (draw.flags & INHERITED);
        match draw.kind {
            DrawKind::Objwin => {
                if texel != 0 {
                    self.row[out_x] |= FLAG_OBJWIN;
                } else if current != FLAG_UNWRITTEN && (current & FLAG_ORDER_MASK) > draw.flags {
                    self.sprite_layer[out_x] = inherit(current);
                }
            }
            DrawKind::Normal | DrawKind::NormalObjwin => {
                if (current & FLAG_ORDER_MASK) <= draw.flags {
                    return;
                }
                if texel != 0 {
                    let (palette, base) = if draw.kind == DrawKind::NormalObjwin
                        && self.row[out_x] & FLAG_OBJWIN != 0
                    {
                        draw.objwin_palette
                    } else {
                        draw.palette
                    };
                    self.sprite_layer[out_x] =
                        u32::from(self.palettes[palette][base + texel as usize]) | draw.flags;
                } else if current != FLAG_UNWRITTEN {
                    self.sprite_layer[out_x] = inherit(current);
                }
            }
        }
    }

    /// Merges sprite-layer pixels of `priority` into the current window
    /// segment of the scanline buffer (mGBA:
    /// `GBAVideoSoftwareRendererPostprocessSprite`).
    pub(crate) fn postprocess_sprite(&mut self, priority: u32) {
        let flags = if self.target2_obj { FLAG_TARGET_2 } else { 0 };
        let (eva, evb) = (self.blda, self.bldb);
        let window_obj = self.current_window.obj_enable();
        // Which pixels may receive sprites: `None` for all, `Some(inside)`
        // for only those inside (or outside) the OBJ window.
        let objwin = self.objwin_enabled();
        let restrict = if objwin {
            let objwin_obj = self.objwin.obj_enable();
            match (objwin_obj, window_obj) {
                (false, false) => return,
                (false, true) => Some(false),
                (true, false) => Some(true),
                (true, true) => None,
            }
        } else if !window_obj {
            return;
        } else {
            None
        };
        for x in self.start..self.end {
            let color = self.sprite_layer[x] & !FLAG_OBJWIN;
            let current = self.row[x];
            if color & FLAG_UNWRITTEN == FLAG_UNWRITTEN
                || (color & FLAG_PRIORITY) >> OFFSET_PRIORITY != priority
            {
                continue;
            }
            if restrict.is_some_and(|inside| (current & FLAG_OBJWIN != 0) != inside) {
                continue;
            }
            self.row[x] = if objwin {
                blend_objwin(color | flags, current, eva, evb)
            } else {
                blend_no_objwin(color | flags, current, eva, evb)
            };
        }
    }
}
