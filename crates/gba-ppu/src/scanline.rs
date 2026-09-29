// Derived from mGBA (src/gba/renderers/video-software.c), Copyright (c)
// 2013-2015 Jeffrey Pfau, licensed under the Mozilla Public License, v. 2.0.
// This file is therefore also under the MPL-2.0: https://mozilla.org/MPL/2.0/.

//! Per-scanline driver (mGBA: `GBAVideoSoftwareRendererDrawScanline` and
//! helpers): window segmentation, layer ordering, and the final blending
//! passes.

use crate::Ppu;
use crate::color::{brighten, darken, mix};
use crate::composite::*;
use crate::state::{BlendEffect, ENABLED_MAX, WindowN, WindowRegion};
use crate::{HEIGHT, NORMAL, VARIANT, WIDTH};

/// Total scanlines per frame including vblank.
const VERTICAL_TOTAL: u8 = 228;

/// Forced blank shows white.
const WHITE: u16 = 0x7FFF;

impl WindowN {
    /// The vertical "inside" state a window has at the start of a frame
    /// when its registers held their current values through the previous
    /// frame.
    ///
    /// mGBA tracks `on` across frames, toggling it as scanlines pass
    /// `v.start`/`v.end` and adjusting it at vblank. Simulating one frame
    /// from `false` reaches that steady state, so frames can be rendered
    /// independently.
    pub(crate) fn steady_state_on(v: WindowRegion) -> bool {
        let mut on = false;
        for y in 0..HEIGHT {
            step_window_on(&mut on, v, y);
        }
        finish_frame_on(&mut on, v);
        on
    }
}

fn step_window_on(on: &mut bool, v: WindowRegion, y: usize) {
    if y == usize::from(v.start) {
        *on = true;
    }
    if y == usize::from(v.end) {
        *on = false;
    }
}

/// mGBA: the window part of `GBAVideoSoftwareRendererFinishFrame`.
fn finish_frame_on(on: &mut bool, v: WindowRegion) {
    let visible = HEIGHT as u8;
    if v.end >= visible && v.end < VERTICAL_TOTAL {
        *on = false;
    }
    if v.start >= visible && v.start < VERTICAL_TOTAL && v.start > v.end {
        *on = true;
    }
}

impl Ppu {
    /// Renders scanline `y` into `self.line`.
    pub(crate) fn draw_scanline(&mut self, y: usize, vram: &[u8], oam: &[u8]) {
        self.next_y = if y == HEIGHT - 1 { 0 } else { y + 1 };
        for win in &mut self.win_n {
            let v = win.v;
            step_window_on(&mut win.on, v, y);
        }

        if self.dispcnt & 0x80 != 0 {
            // Forced blank. Note that affine reference points don't advance.
            self.line.fill(WHITE);
            return;
        }

        self.preprocess_buffer();
        self.reset_sprite_cycles();
        let sprite_layers = self.preprocess_sprite_layer(y as i32, vram, oam);

        let mode = self.mode();
        self.end = 0;
        for w in 0..self.n_windows {
            self.start = self.end;
            self.end = usize::from(self.windows[w].end_x);
            self.current_window = self.windows[w].control;
            self.prepare_window();
            for priority in 0..4 {
                if sprite_layers & (1 << priority) != 0 {
                    self.postprocess_sprite(priority);
                }
                let y = y as i32;
                if self.layer_enabled(0, priority) && mode < 2 {
                    self.draw_background_mode0(0, y, vram);
                }
                if self.layer_enabled(1, priority) && mode < 2 {
                    self.draw_background_mode0(1, y, vram);
                }
                if self.layer_enabled(2, priority) {
                    match mode {
                        0 => self.draw_background_mode0(2, y, vram),
                        1 | 2 => self.draw_background_mode2(2, y, vram),
                        3 | 5 => self.draw_background_mode3_5(2, y, vram),
                        4 => self.draw_background_mode4(2, y, vram),
                        _ => {}
                    }
                }
                if self.layer_enabled(3, priority) {
                    match mode {
                        0 => self.draw_background_mode0(3, y, vram),
                        2 => self.draw_background_mode2(3, y, vram),
                        _ => {}
                    }
                }
            }
        }

        self.postprocess_buffer();

        // Affine reference points advance by (PB, PD) per drawn scanline.
        if mode != 0 {
            for bg in &mut self.bg[2..] {
                if bg.enabled == ENABLED_MAX {
                    bg.sx += i32::from(bg.dmx);
                    bg.sy += i32::from(bg.dmy);
                }
            }
        }
        for bg in &mut self.bg {
            if bg.enabled != 0 && bg.enabled < ENABLED_MAX {
                bg.enabled += 1;
            }
        }

        self.write_line();
    }

    /// Copies the scanline buffer's colors to `self.line`, applying green
    /// swap if enabled.
    fn write_line(&mut self) {
        let row = &self.row;
        if self.stereo {
            const GREEN: u32 = 0x03E0;
            const RED_BLUE: u32 = 0x7C1F;
            for x in (0..WIDTH).step_by(2) {
                self.line[x] = ((row[x] & RED_BLUE) | (row[x + 1] & GREEN)) as u16;
                self.line[x + 1] = ((row[x + 1] & RED_BLUE) | (row[x] & GREEN)) as u16;
            }
        } else {
            for (out, &pixel) in self.line.iter_mut().zip(row) {
                *out = (pixel & 0x7FFF) as u16;
            }
        }
    }

    /// mGBA: `TEST_LAYER_ENABLED`.
    fn layer_enabled(&self, index: usize, priority: u32) -> bool {
        let bg = &self.bg[index];
        bg.enabled == ENABLED_MAX
            && (self.current_window.bg_enable(index)
                || (self.objwin_enabled() && self.objwin.bg_enable(index)))
            && bg.priority == priority
    }

    /// Sets up the scanline: clears the sprite layer, splits the line into
    /// window segments, and fills the buffer with the backdrop (mGBA:
    /// `GBAVideoSoftwareRendererPreprocessBuffer`).
    fn preprocess_buffer(&mut self) {
        self.sprite_layer.fill(FLAG_UNWRITTEN);

        self.windows[0].end_x = WIDTH as u8;
        self.n_windows = 1;
        let win0 = self.dispcnt & 0x2000 != 0;
        let win1 = self.dispcnt & 0x4000 != 0;
        if win0 || win1 || self.objwin_enabled() {
            self.windows[0].control = self.winout;
            // WIN1 goes in first so WIN0, which has precedence, overwrites it.
            if win1 && self.win_n[1].on {
                self.break_window(self.win_n[1]);
            }
            if win0 && self.win_n[0].on {
                self.break_window(self.win_n[0]);
            }
        } else {
            // Everything enabled. The precedence is left stale, as in mGBA.
            self.windows[0].control.packed = 0xFF;
        }

        self.update_dispcnt();
        if self.blend_dirty {
            self.update_palettes();
            self.blend_dirty = false;
        }
        self.force_target1 = false;

        let mut x = 0;
        for w in 0..self.n_windows {
            let backdrop =
                FLAG_UNWRITTEN | FLAG_PRIORITY | FLAG_IS_BACKGROUND | self.backdrop_color(w);
            let end = usize::from(self.windows[w].end_x);
            // mGBA fills up to the next multiple of four before looking at
            // the segment end, so a segment can spill into the next.
            while x & 3 != 0 {
                self.row[x] = backdrop;
                x += 1;
            }
            while x < end {
                self.row[x] = backdrop;
                x += 1;
            }
        }
    }

    /// Backdrop color within window segment `w`: palette entry 0, brightness
    /// adjusted if the backdrop is a 1st target there.
    fn backdrop_color(&self, w: usize) -> u32 {
        let adjusted = self.target1_bd
            && self.blend_effect.is_brightness()
            && self.windows[w].control.blend_enable();
        u32::from(self.palettes[if adjusted { VARIANT } else { NORMAL }][0])
    }

    /// Final blending passes (mGBA: `GBAVideoSoftwareRendererPostprocessBuffer`):
    /// 1st-target pixels with only the backdrop behind them blend with it,
    /// and reblended sprite pixels get the brightness effect.
    fn postprocess_buffer(&mut self) {
        let any_target1 = self.force_target1 || self.bg.iter().any(|bg| bg.target1);
        if any_target1 && self.target2_bd {
            let mut x = 0;
            for w in 0..self.n_windows {
                let backdrop = self.backdrop_color(w);
                let end = usize::from(self.windows[w].end_x);
                while x < end {
                    let color = self.row[x];
                    if color & FLAG_TARGET_1 != 0 {
                        self.row[x] = mix(self.bldb, backdrop, self.blda, color);
                    }
                    x += 1;
                }
            }
        }
        if self.force_target1 && self.blend_effect.is_brightness() {
            let mut x = 0;
            let objwin_blend = self.objwin.blend_enable();
            for w in 0..self.n_windows {
                let end = usize::from(self.windows[w].end_x);
                let window_blend = self.windows[w].control.blend_enable();
                let mut mask = FLAG_REBLEND | FLAG_IS_BACKGROUND;
                let mut matched = FLAG_REBLEND;
                if self.objwin_enabled() && objwin_blend != window_blend {
                    mask |= FLAG_OBJWIN;
                    if objwin_blend {
                        matched |= FLAG_OBJWIN;
                    }
                } else if !window_blend {
                    x = end;
                    continue;
                }
                while x < end {
                    let color = self.row[x];
                    if color & mask == matched {
                        self.row[x] = if self.blend_effect == BlendEffect::Darken {
                            darken(color, self.bldy)
                        } else {
                            brighten(color, self.bldy)
                        };
                    }
                    x += 1;
                }
            }
        }
    }

    /// Recomputes the brightness-adjusted palette (mGBA: `_updatePalettes`).
    pub(crate) fn update_palettes(&mut self) {
        let [normal, variant] = &mut self.palettes;
        for (v, &n) in variant.iter_mut().zip(normal.iter()) {
            let n = u32::from(n);
            *v = match self.blend_effect {
                BlendEffect::Brighten => brighten(n, self.bldy),
                BlendEffect::Darken => darken(n, self.bldy),
                BlendEffect::None | BlendEffect::Alpha => n,
            } as u16;
        }
    }

    /// Inserts a window into the segment list, splitting it in two if it
    /// wraps around the right edge (mGBA: `_breakWindow`).
    fn break_window(&mut self, win: WindowN) {
        if win.h.end > WIDTH as u8 || win.h.end < win.h.start {
            let mut left = win;
            left.h.start = 0;
            let mut right = win;
            right.h.end = WIDTH as u8;
            self.break_window_inner(left);
            self.break_window_inner(right);
        } else {
            self.break_window_inner(win);
        }
    }

    /// mGBA: `_breakWindowInner`, ported as is. It can leave empty segments
    /// behind, and segment boundaries influence how mGBA draws text
    /// backgrounds and mosaic sprites, so the exact list matters.
    fn break_window_inner(&mut self, win: WindowN) {
        if win.h.end == 0 {
            return;
        }
        let mut start_x = 0;
        let mut active = 0;
        while active < self.n_windows {
            if win.h.start < self.windows[active].end_x {
                let old = self.windows[active];
                if win.h.start > start_x {
                    // Split the segment at the window's start.
                    self.insert_window(active);
                    self.windows[active].end_x = win.h.start;
                    active += 1;
                }
                self.windows[active].control = win.control;
                self.windows[active].end_x = win.h.end;
                if win.h.end >= old.end_x {
                    // Trim segments the window now covers.
                    active += 1;
                    while self.n_windows > active + 1 && win.h.end >= self.windows[active].end_x {
                        self.windows[active] = self.windows[active + 1];
                        self.n_windows -= 1;
                        active += 1;
                    }
                } else {
                    // Resume the split segment after the window.
                    active += 1;
                    self.insert_window(active);
                    self.windows[active] = old;
                }
                return;
            }
            start_x = self.windows[active].end_x;
            active += 1;
        }
    }

    /// Shifts segments `at..` right by one, duplicating segment `at`.
    fn insert_window(&mut self, at: usize) {
        let n = self.n_windows;
        self.windows.copy_within(at..n, at + 1);
        self.n_windows += 1;
    }

    /// Recomputes per-background window state for the current segment
    /// (mGBA: `GBAVideoSoftwareRendererPrepareWindow`).
    fn prepare_window(&mut self) {
        if self.objwin_enabled() {
            for (i, bg) in self.bg.iter_mut().enumerate() {
                bg.objwin_force_enable =
                    self.objwin.bg_enable(i) && self.current_window.bg_enable(i);
                bg.objwin_only = !self.objwin.bg_enable(i);
            }
        }
        let layers: &[usize] = match self.mode() {
            0 => &[0, 1, 2, 3],
            1 => &[0, 1, 2],
            2 => &[2, 3],
            3..=5 => &[2],
            _ => &[],
        };
        for &i in layers {
            if self.bg[i].enabled == ENABLED_MAX {
                self.update_flags(i);
            }
        }
    }

    /// Computes a background's compositing flags for the current segment
    /// and for OBJ window pixels (mGBA: `_updateFlags`).
    fn update_flags(&mut self, index: usize) {
        let blend_effect = self.blend_effect;
        let window_blend = self.current_window.blend_enable();
        let objwin_blend = self.objwin.blend_enable();
        // EVA 16 / EVB 0 alpha blending is a no-op, so mGBA doesn't blend.
        let trivial_alpha = self.blda == 0x10 && self.bldb == 0;
        let bg = &mut self.bg[index];
        let mut flags =
            (bg.priority << OFFSET_PRIORITY) | (bg.index << OFFSET_INDEX) | FLAG_IS_BACKGROUND;
        if bg.target2 {
            flags |= FLAG_TARGET_2;
        }
        let mut objwin_flags = flags;
        if blend_effect == BlendEffect::Alpha {
            if trivial_alpha {
                flags &= !FLAG_TARGET_2;
                objwin_flags &= !FLAG_TARGET_2;
            } else if bg.target1 {
                if window_blend {
                    flags |= FLAG_TARGET_1;
                }
                if objwin_blend {
                    objwin_flags |= FLAG_TARGET_1;
                }
            }
        }
        bg.flags = flags;
        bg.objwin_flags = objwin_flags;
        bg.variant = bg.target1 && window_blend && blend_effect.is_brightness();
    }
}
