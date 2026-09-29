//! A scanline software renderer for the Game Boy Advance PPU.
//!
//! [`Ppu::render_frame`] turns a snapshot of VRAM, palette RAM, OAM and the
//! video I/O registers into a 240x160 frame. Registers may optionally change
//! between scanlines (for H-blank DMA and H-blank interrupt effects) through
//! a per-line callback.
//!
//! ```
//! use gba_ppu::{Ppu, regs, HEIGHT, IO_SIZE, OAM_SIZE, PALETTE_SIZE, VRAM_SIZE, WIDTH};
//!
//! let mut io = [0u8; IO_SIZE];
//! let vram = vec![0u8; VRAM_SIZE].try_into().unwrap();
//! let mut palette = [0u8; PALETTE_SIZE];
//! let oam = [0u8; OAM_SIZE];
//! palette[0..2].copy_from_slice(&0x001Fu16.to_le_bytes()); // red backdrop
//! io[regs::DISPCNT..][..2].copy_from_slice(&0x0100u16.to_le_bytes()); // mode 0, BG0 on
//!
//! // Scroll BG0 by one more pixel on each scanline.
//! let mut skew = |y: usize, io: &mut [u8; IO_SIZE]| {
//!     io[regs::BG0HOFS..][..2].copy_from_slice(&(y as u16).to_le_bytes());
//! };
//! let mut frame = vec![0u32; WIDTH * HEIGHT].try_into().unwrap();
//! Ppu::new().render_frame(&io, &vram, &palette, &oam, Some(&mut skew), &mut frame);
//! assert_eq!(frame[0], 0xFF0000);
//! ```
//!
//! # Accuracy
//!
//! The renderer is a port of mGBA's software renderer
//! (`src/gba/renderers/video-software.c` and friends) and aims to be
//! pixel-identical to it, including its corner-case behavior, for the mGBA
//! configuration used by mgba-rs (`COLOR_16_BIT`: colors are blended in
//! BGR555). [`Ppu::render_frame_bgr555`] yields exactly the framebuffer
//! mGBA produces; [`Ppu::render_frame`] converts it the way mgba-rs does
//! (see [`bgr555_to_rgb888`]). It has been checked against mGBA's C
//! renderer on thousands of randomized scenes, with and without per-line
//! register changes.
//!
//! Differences from running mGBA itself come from the snapshot interface:
//!
//! - VRAM, palette RAM and OAM are fixed for the whole frame; mid-frame
//!   writes to them are not modeled.
//! - Register writes are only seen at scanline granularity, and a register
//!   counts as written when its value differs from the previous line's. In
//!   particular, rewriting an affine reference point (`BGxX`/`BGxY`) with
//!   the value it already had does not re-latch it.
//! - State that mGBA carries from one frame to the next is derived from the
//!   frame's own registers: backgrounds enabled in the initial DISPCNT are
//!   fully enabled from line 0, and WIN0/WIN1 start the frame in the
//!   vertical state they would have if WINxV had been unchanged during the
//!   previous frame. (The one exception, the stale window precedence mGBA
//!   uses for OBJ-window sprites when no window is enabled, is carried in
//!   the [`Ppu`] between frames.)
//! - mGBA skips scanlines whose inputs are unchanged since the previous
//!   frame, which is only visible when that happens during forced blank
//!   (affine reference points then advance) or while a background is being
//!   enabled mid-frame. This renderer always draws every scanline, matching
//!   mGBA whenever VRAM, OAM or palette RAM changed since the last frame.
//!
//! # License
//!
//! The rendering modules are derived from mGBA, Copyright (c) 2013-2025
//! Jeffrey Pfau, and are licensed under the Mozilla Public License 2.0 (see
//! each file's header).

mod bg;
mod color;
mod composite;
mod obj;
pub mod regs;
mod scanline;
mod state;

pub use color::bgr555_to_rgb888;

use obj::Sprite;
use state::{Background, BlendEffect, Window, WindowControl, WindowN};

/// Screen width in pixels.
pub const WIDTH: usize = 240;
/// Screen height in pixels.
pub const HEIGHT: usize = 160;
/// Size of the I/O register block (`0x0400_0000..0x0400_0400`).
pub const IO_SIZE: usize = 0x400;
/// Size of VRAM (`0x0600_0000..0x0601_8000`).
pub const VRAM_SIZE: usize = 0x18000;
/// Size of palette RAM (`0x0500_0000..0x0500_0400`).
pub const PALETTE_SIZE: usize = 0x400;
/// Size of OAM (`0x0700_0000..0x0700_0400`).
pub const OAM_SIZE: usize = 0x400;

/// Index of the unmodified palette in [`Ppu::palettes`].
const NORMAL: usize = 0;
/// Index of the brightness-adjusted palette in [`Ppu::palettes`].
const VARIANT: usize = 1;

/// Slots in the window segment list. mGBA never needs more than five;
/// the spare slots keep segment insertion's shifting in bounds.
const WINDOW_SLOTS: usize = 8;

/// Per-scanline register override callback: called with the scanline
/// number and a mutable copy of the I/O registers before that scanline is
/// drawn. Changes persist to later scanlines of the same frame.
pub type LineIo<'a> = &'a mut dyn FnMut(usize, &mut [u8; IO_SIZE]);

/// GBA PPU renderer state.
///
/// Holds the decoded register state, internal affine reference points,
/// window state and scratch buffers. Rendering doesn't allocate.
pub struct Ppu {
    // Decoded register state (mGBA: `GBAVideoSoftwareRenderer`).
    dispcnt: u16,
    /// Green swap (mGBA: "stereo").
    stereo: bool,
    bg: [Background; 4],
    target1_obj: bool,
    target1_bd: bool,
    target2_obj: bool,
    target2_bd: bool,
    blend_effect: BlendEffect,
    /// The variant palette must be recomputed before the next scanline.
    blend_dirty: bool,
    /// EVA, EVB and EVY, clamped to 16.
    blda: u32,
    bldb: u32,
    bldy: u32,
    mosaic: u16,
    win_n: [WindowN; 2],
    winout: WindowControl,
    objwin: WindowControl,

    // Per-scanline state.
    /// Window control of the segment being drawn.
    current_window: WindowControl,
    /// Window segments of the scanline, left to right. Deliberately not
    /// reset between frames: like mGBA, the first segment's precedence is
    /// left stale on lines without windows.
    windows: [Window; WINDOW_SLOTS],
    n_windows: usize,
    /// Current segment, `start..end`.
    start: usize,
    end: usize,
    /// Scanline buffer: BGR555 colors tagged with compositing flags (see
    /// the `composite` module).
    row: [u32; WIDTH],
    /// Sprites of the scanline, before merging into `row`.
    sprite_layer: [u32; WIDTH],
    sprite_cycles_remaining: i32,
    /// Some sprite pixel needs the post-compositing passes.
    force_target1: bool,
    /// The next scanline to be drawn (0 during vblank).
    next_y: usize,
    /// Finished scanline.
    line: [u16; WIDTH],

    // Per-frame state.
    /// Palette RAM as BGR555, unmodified and brightness-adjusted (for layers
    /// that are a 1st target of brighten/darken).
    palettes: [[u16; 512]; 2],
    sprites: [Sprite; 128],
    oam_max: usize,
    /// Video registers as last applied, to detect per-line writes.
    regs: [u16; regs::BLDY / 2 + 1],
}

impl Default for Ppu {
    fn default() -> Ppu {
        Ppu::new()
    }
}

impl Ppu {
    pub fn new() -> Ppu {
        let mut ppu = Ppu {
            dispcnt: 0,
            stereo: false,
            bg: [Background::default(); 4],
            target1_obj: false,
            target1_bd: false,
            target2_obj: false,
            target2_bd: false,
            blend_effect: BlendEffect::None,
            blend_dirty: false,
            blda: 0,
            bldb: 0,
            bldy: 0,
            mosaic: 0,
            win_n: [WindowN::default(); 2],
            winout: WindowControl::default(),
            objwin: WindowControl::default(),
            current_window: WindowControl::default(),
            windows: [Window::default(); WINDOW_SLOTS],
            n_windows: 0,
            start: 0,
            end: 0,
            row: [0; WIDTH],
            sprite_layer: [0; WIDTH],
            sprite_cycles_remaining: 0,
            force_target1: false,
            next_y: 0,
            line: [0; WIDTH],
            palettes: [[0; 512]; 2],
            sprites: [Sprite::default(); 128],
            oam_max: 0,
            regs: [0; regs::BLDY / 2 + 1],
        };
        ppu.reset_registers();
        ppu
    }

    /// Renders one full frame as `0x00RRGGBB` pixels, row-major.
    ///
    /// `io` is the I/O register block as the game wrote it (little-endian
    /// halfwords at `address - 0x0400_0000`); see [`regs`] for offsets.
    /// If `line_io` is given, it is called before each scanline `y` in
    /// `0..160` with a copy of the registers that it may modify; changes
    /// carry over to later scanlines.
    pub fn render_frame(
        &mut self,
        io: &[u8; IO_SIZE],
        vram: &[u8; VRAM_SIZE],
        palette: &[u8; PALETTE_SIZE],
        oam: &[u8; OAM_SIZE],
        line_io: Option<LineIo<'_>>,
        out: &mut [u32; WIDTH * HEIGHT],
    ) {
        self.render(io, vram, palette, oam, line_io, |y, line| {
            for (dst, &color) in out[y * WIDTH..][..WIDTH].iter_mut().zip(line) {
                *dst = bgr555_to_rgb888(color);
            }
        });
    }

    /// Like [`Ppu::render_frame`], but outputs raw BGR555 colors
    /// (`r | g << 5 | b << 10`), exactly as mGBA's `COLOR_16_BIT`
    /// framebuffer holds them.
    pub fn render_frame_bgr555(
        &mut self,
        io: &[u8; IO_SIZE],
        vram: &[u8; VRAM_SIZE],
        palette: &[u8; PALETTE_SIZE],
        oam: &[u8; OAM_SIZE],
        line_io: Option<LineIo<'_>>,
        out: &mut [u16; WIDTH * HEIGHT],
    ) {
        self.render(io, vram, palette, oam, line_io, |y, line| {
            out[y * WIDTH..][..WIDTH].copy_from_slice(line);
        });
    }

    fn render(
        &mut self,
        io: &[u8; IO_SIZE],
        vram: &[u8; VRAM_SIZE],
        palette: &[u8; PALETTE_SIZE],
        oam: &[u8; OAM_SIZE],
        mut line_io: Option<LineIo<'_>>,
        mut emit: impl FnMut(usize, &[u16; WIDTH]),
    ) {
        self.begin_frame(io, palette, oam);
        let mut line_regs = *io;
        for y in 0..HEIGHT {
            if let Some(callback) = line_io.as_mut() {
                callback(y, &mut line_regs);
                // Writes land after the previous line was drawn.
                self.next_y = y;
                self.apply_changed_registers(&line_regs);
            }
            self.draw_scanline(y, vram, oam);
            emit(y, &self.line);
        }
    }

    /// Latches the frame's initial state, as mGBA's renderer would hold it
    /// after vblank with these registers written.
    fn begin_frame(
        &mut self,
        io: &[u8; IO_SIZE],
        palette: &[u8; PALETTE_SIZE],
        oam: &[u8; OAM_SIZE],
    ) {
        self.reset_registers();
        // Writes during vblank enable backgrounds immediately and load the
        // affine reference points.
        self.next_y = 0;
        for address in regs::VIDEO_REGISTERS {
            let value = read_u16(io, address);
            self.regs[address / 2] = value;
            self.write_register(address, value);
        }
        for (i, color) in self.palettes[NORMAL].iter_mut().enumerate() {
            *color = read_u16(palette, i * 2) & 0x7FFF;
        }
        self.update_palettes();
        self.blend_dirty = false;
        for win in &mut self.win_n {
            win.on = WindowN::steady_state_on(win.v);
        }
        self.clean_oam(oam);
    }

    /// Applies every video register whose value changed since it was last
    /// applied.
    fn apply_changed_registers(&mut self, io: &[u8; IO_SIZE]) {
        for address in regs::VIDEO_REGISTERS {
            let value = read_u16(io, address);
            if self.regs[address / 2] != value {
                self.regs[address / 2] = value;
                self.write_register(address, value);
            }
        }
    }
}

#[inline]
pub(crate) fn read_u16(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([buf[offset], buf[offset + 1]])
}

#[inline]
pub(crate) fn read_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}
