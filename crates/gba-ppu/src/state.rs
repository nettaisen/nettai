// Derived from mGBA (src/gba/renderers/video-software.c and
// include/mgba/internal/gba/renderers/video-software.h), Copyright (c)
// 2013-2015 Jeffrey Pfau, licensed under the Mozilla Public License, v. 2.0.
// This file is therefore also under the MPL-2.0: https://mozilla.org/MPL/2.0/.

//! Decoded register state and the register write handlers that fill it
//! (mGBA: `GBAVideoSoftwareRendererWriteVideoRegister`).

use crate::Ppu;
use crate::regs::*;

/// `bg[n].enabled` value at which a background is drawn. mGBA models the
/// hardware's delay in enabling a layer mid-frame by counting up to this
/// value one scanline at a time (see [`Ppu::enable_bg`]).
pub(crate) const ENABLED_MAX: i32 = 4;

/// Per-background state (mGBA: `GBAVideoSoftwareBackground`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Background {
    pub index: u32,
    /// 0: disabled; 1..=3: being enabled; [`ENABLED_MAX`]: drawn;
    /// negative: just disabled (re-enabling is immediate).
    pub enabled: i32,
    pub priority: u32,
    pub char_base: u32,
    pub mosaic: bool,
    /// 256-color (8bpp) tiles.
    pub multipalette: bool,
    pub screen_base: u32,
    /// Affine wraparound (BGCNT bit 13).
    pub overflow: bool,
    pub size: u32,
    pub target1: bool,
    pub target2: bool,
    /// Horizontal scroll (text mode).
    pub x: u16,
    /// Vertical scroll (text mode).
    pub y: u16,
    /// Affine reference point as written to BGxX/BGxY.
    pub refx: i32,
    pub refy: i32,
    /// PA, PB, PC, PD.
    pub dx: i16,
    pub dmx: i16,
    pub dy: i16,
    pub dmy: i16,
    /// Internal (latched) reference point for the current scanline.
    pub sx: i32,
    pub sy: i32,
    /// Compositing flags inside the current window segment, and for pixels
    /// inside the OBJ window (see `Ppu::update_flags`).
    pub flags: u32,
    pub objwin_flags: u32,
    pub objwin_force_enable: bool,
    pub objwin_only: bool,
    /// Draw with the brightness-adjusted palette.
    pub variant: bool,
}

impl Background {
    pub fn new(index: u32) -> Background {
        Background {
            index,
            dx: 256,
            dmy: 256,
            ..Background::default()
        }
    }
}

/// WININ/WINOUT control byte plus the window's precedence (0 = WIN0,
/// 1 = WIN1, 2 = OBJ window, 3 = outside).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WindowControl {
    pub packed: u8,
    pub priority: i8,
}

impl WindowControl {
    pub fn with_priority(priority: i8) -> WindowControl {
        WindowControl {
            packed: 0,
            priority,
        }
    }

    pub fn bg_enable(self, index: usize) -> bool {
        self.packed & (1 << index) != 0
    }

    pub fn obj_enable(self) -> bool {
        self.packed & 0x10 != 0
    }

    pub fn blend_enable(self) -> bool {
        self.packed & 0x20 != 0
    }
}

/// One edge pair of WINxH or WINxV. `end` is exclusive.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WindowRegion {
    pub start: u8,
    pub end: u8,
}

/// WIN0 or WIN1 (mGBA: `struct WindowN`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WindowN {
    pub h: WindowRegion,
    pub v: WindowRegion,
    pub control: WindowControl,
    /// Whether the current scanline is inside the vertical range. Toggled
    /// as the scanline counter passes `v.start` and `v.end`.
    pub on: bool,
}

/// A horizontal segment of the scanline sharing one window control; the
/// segment spans from the previous segment's `end_x` to this one's.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Window {
    pub end_x: u8,
    pub control: WindowControl,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BlendEffect {
    #[default]
    None,
    Alpha,
    Brighten,
    Darken,
}

impl BlendEffect {
    pub fn is_brightness(self) -> bool {
        matches!(self, BlendEffect::Brighten | BlendEffect::Darken)
    }
}

/// Sign-extends a 28-bit affine reference point.
fn sign_extend_28(value: u32) -> i32 {
    ((value << 4) as i32) >> 4
}

impl Ppu {
    /// Returns the decoded register state to what mGBA's renderer holds
    /// right after `reset`, before any register is written.
    ///
    /// The window segment list is deliberately left alone: mGBA leaves the
    /// first segment's precedence stale when no window is enabled, and that
    /// stale value decides whether OBJ-window sprites are processed.
    pub(crate) fn reset_registers(&mut self) {
        self.dispcnt = 0x0080;
        self.stereo = false;
        self.target1_obj = false;
        self.target1_bd = false;
        self.target2_obj = false;
        self.target2_bd = false;
        self.blend_effect = BlendEffect::None;
        self.blend_dirty = false;
        self.blda = 0;
        self.bldb = 0;
        self.bldy = 0;
        self.win_n = [
            WindowN {
                control: WindowControl::with_priority(0),
                ..WindowN::default()
            },
            WindowN {
                control: WindowControl::with_priority(1),
                ..WindowN::default()
            },
        ];
        self.objwin = WindowControl::with_priority(2);
        self.winout = WindowControl::with_priority(3);
        self.mosaic = 0;
        self.bg = [0, 1, 2, 3].map(Background::new);
    }

    /// Applies a write of `value` to the video register at byte offset
    /// `address`, masking and decoding it the way mGBA does.
    pub(crate) fn write_register(&mut self, address: usize, value: u16) {
        match address {
            DISPCNT => {
                self.dispcnt = value & 0xFFF7;
                self.update_dispcnt();
            }
            GREENSWP => self.stereo = value & 1 != 0,
            BG0CNT | BG1CNT => self.write_bgcnt((address - BG0CNT) / 2, value & 0xDFFF),
            BG2CNT | BG3CNT => self.write_bgcnt((address - BG0CNT) / 2, value),
            BG0HOFS..=BG3VOFS => {
                let bg = &mut self.bg[(address - BG0HOFS) / 4];
                if address & 2 == 0 {
                    bg.x = value & 0x1FF;
                } else {
                    bg.y = value & 0x1FF;
                }
            }
            BG2PA..=0x3E => {
                let bg = &mut self.bg[2 + (address - BG2PA) / 0x10];
                let value32 = u32::from(value);
                match (address - BG2PA) % 0x10 {
                    0x0 => bg.dx = value as i16,
                    0x2 => bg.dmx = value as i16,
                    0x4 => bg.dy = value as i16,
                    0x6 => bg.dmy = value as i16,
                    // Writing either half of a reference point reloads the
                    // internal one immediately, even mid-frame.
                    0x8 => {
                        bg.refx = ((bg.refx as u32 & 0xFFFF_0000) | value32) as i32;
                        bg.sx = bg.refx;
                    }
                    0xA => {
                        bg.refx = sign_extend_28((bg.refx as u32 & 0xFFFF) | (value32 << 16));
                        bg.sx = bg.refx;
                    }
                    0xC => {
                        bg.refy = ((bg.refy as u32 & 0xFFFF_0000) | value32) as i32;
                        bg.sy = bg.refy;
                    }
                    _ => {
                        bg.refy = sign_extend_28((bg.refy as u32 & 0xFFFF) | (value32 << 16));
                        bg.sy = bg.refy;
                    }
                }
            }
            BLDCNT => self.write_bldcnt(value),
            BLDALPHA => {
                self.blda = u32::from(value & 0x1F).min(0x10);
                self.bldb = u32::from((value >> 8) & 0x1F).min(0x10);
            }
            BLDY => {
                let bldy = u32::from(value & 0x1F).min(0x10);
                if self.bldy != bldy {
                    self.bldy = bldy;
                    self.blend_dirty = true;
                }
            }
            WIN0H | WIN1H => {
                let h = &mut self.win_n[(address - WIN0H) / 2].h;
                h.end = value as u8;
                h.start = (value >> 8) as u8;
                if h.start > 240 && h.start > h.end {
                    h.start = 0;
                }
                if h.end > 240 {
                    h.end = 240;
                    if h.start > 240 {
                        h.start = 240;
                    }
                }
            }
            WIN0V | WIN1V => {
                let v = &mut self.win_n[(address - WIN0V) / 2].v;
                v.end = value as u8;
                v.start = (value >> 8) as u8;
            }
            WININ => {
                self.win_n[0].control.packed = (value & 0x3F) as u8;
                self.win_n[1].control.packed = ((value >> 8) & 0x3F) as u8;
            }
            WINOUT => {
                self.winout.packed = (value & 0x3F) as u8;
                self.objwin.packed = ((value >> 8) & 0x3F) as u8;
            }
            MOSAIC => self.mosaic = value,
            _ => {}
        }
    }

    fn write_bgcnt(&mut self, index: usize, value: u16) {
        let bg = &mut self.bg[index];
        let value = u32::from(value);
        bg.priority = value & 3;
        bg.char_base = ((value >> 2) & 3) << 14;
        bg.mosaic = value & 0x40 != 0;
        bg.multipalette = value & 0x80 != 0;
        bg.screen_base = ((value >> 8) & 0x1F) << 11;
        bg.overflow = value & 0x2000 != 0;
        bg.size = value >> 14;
    }

    fn write_bldcnt(&mut self, value: u16) {
        let old_effect = self.blend_effect;
        for (i, bg) in self.bg.iter_mut().enumerate() {
            bg.target1 = value & (1 << i) != 0;
            bg.target2 = value & (0x100 << i) != 0;
        }
        self.blend_effect = match (value >> 6) & 3 {
            0 => BlendEffect::None,
            1 => BlendEffect::Alpha,
            2 => BlendEffect::Brighten,
            _ => BlendEffect::Darken,
        };
        self.target1_obj = value & 0x10 != 0;
        self.target1_bd = value & 0x20 != 0;
        self.target2_obj = value & 0x1000 != 0;
        self.target2_bd = value & 0x2000 != 0;
        if old_effect != self.blend_effect {
            self.blend_dirty = true;
        }
    }

    /// Updates each background's enable counter from DISPCNT (mGBA:
    /// `_enableBg`). A layer enabled during vblank (`next_y == 0`) appears
    /// at once; one enabled mid-frame only appears a few scanlines later.
    /// Disabling from the fully enabled state goes through a negative
    /// grace period during which re-enabling is immediate.
    pub(crate) fn update_dispcnt(&mut self) {
        for index in 0..4 {
            self.enable_bg(index, self.dispcnt & (0x100 << index) != 0);
        }
    }

    fn enable_bg(&mut self, index: usize, active: bool) {
        let was_active = self.bg[index].enabled;
        let enabled = &mut self.bg[index].enabled;
        if !active {
            if self.next_y == 0 || (was_active > 0 && was_active < ENABLED_MAX) {
                *enabled = 0;
            } else if was_active == ENABLED_MAX {
                *enabled = -2;
            }
        } else if was_active == 0 {
            *enabled = if self.next_y == 0 {
                ENABLED_MAX
            } else if self.dispcnt & 7 > 2 {
                2
            } else {
                1
            };
        } else if was_active < 0 {
            *enabled = ENABLED_MAX;
        }
    }

    pub(crate) fn mosaic_bg_h(&self) -> i32 {
        i32::from(self.mosaic & 0xF)
    }

    pub(crate) fn mosaic_bg_v(&self) -> i32 {
        i32::from((self.mosaic >> 4) & 0xF)
    }

    pub(crate) fn mosaic_obj_h(&self) -> i32 {
        i32::from((self.mosaic >> 8) & 0xF)
    }

    pub(crate) fn mosaic_obj_v(&self) -> i32 {
        i32::from(self.mosaic >> 12)
    }

    pub(crate) fn mode(&self) -> u16 {
        self.dispcnt & 7
    }

    pub(crate) fn objwin_enabled(&self) -> bool {
        self.dispcnt & 0x8000 != 0
    }
}
