// Derived from mGBA (src/gba/renderers/software-private.h and
// include/mgba/internal/gba/renderers/video-software.h), Copyright (c)
// 2013-2015 Jeffrey Pfau, licensed under the Mozilla Public License, v. 2.0.
// This file is therefore also under the MPL-2.0: https://mozilla.org/MPL/2.0/.

//! Layer compositing on the flag-tagged scanline buffer.
//!
//! Each entry of the scanline buffer (`Ppu::row`) holds a BGR555 color in
//! its low bits and compositing flags in its top byte:
//!
//! ```text
//! 31-30 priority   29-28 BG index   27 is background   26 reblend
//! 25 1st target    24 2nd target (incoming color) / in OBJ window (buffer)
//! ```
//!
//! Layers are drawn front to back: for each priority, sprites first, then
//! BG0..BG3. Because priority, BG index and the background bit sit at the
//! top, "is the new pixel in front?" is a single integer comparison. Only
//! the backdrop can ever be behind a newly drawn layer.
//!
//! Alpha blending happens when a 2nd-target layer lands directly behind a
//! 1st-target pixel; the result carries no flags, which makes the pixel
//! final. A layer landing behind a pixel that it can't blend with strips
//! the pixel's priority and target bits, so nothing further can blend
//! with it either. A 1st-target pixel with nothing behind it but the
//! backdrop keeps its flag and is blended with the backdrop at the end of
//! the scanline.

/// Priority bits (0 = front).
pub(crate) const FLAG_PRIORITY: u32 = 0xC000_0000;
pub(crate) const OFFSET_PRIORITY: u32 = 30;
pub(crate) const OFFSET_INDEX: u32 = 28;
pub(crate) const FLAG_IS_BACKGROUND: u32 = 0x0800_0000;
/// Initial value of empty sprite-layer pixels (and part of the backdrop).
pub(crate) const FLAG_UNWRITTEN: u32 = 0xFC00_0000;
/// Semi-transparent/alpha-blended sprite pixel whose brightness effect is
/// applied after compositing instead of via the palette.
pub(crate) const FLAG_REBLEND: u32 = 0x0400_0000;
pub(crate) const FLAG_TARGET_1: u32 = 0x0200_0000;
pub(crate) const FLAG_TARGET_2: u32 = 0x0100_0000;
/// Shares its bit with [`FLAG_TARGET_2`]: on buffer pixels it marks the OBJ
/// window, on incoming layer colors it marks a 2nd target.
pub(crate) const FLAG_OBJWIN: u32 = 0x0100_0000;
pub(crate) const FLAG_ORDER_MASK: u32 = 0xF800_0000;

/// What survives when a layer ends up behind an existing pixel.
const KEEP_BEHIND: u32 = 0x00FF_FFFF | FLAG_REBLEND | FLAG_OBJWIN;

/// A pixel can still be drawn behind (or blended with) unless it is a
/// finished, flagless color.
#[inline]
pub(crate) fn is_writable(pixel: u32) -> bool {
    pixel & 0xFE00_0000 != 0
}

/// Composites `color` (with its flags) against buffer pixel `current` when
/// the incoming layer may blend and the OBJ window is enabled.
#[inline]
pub(crate) fn blend_objwin(color: u32, current: u32, eva: u32, evb: u32) -> u32 {
    if color >= current {
        if current & FLAG_TARGET_1 != 0 && color & FLAG_TARGET_2 != 0 {
            crate::color::mix(eva, current, evb, color)
        } else {
            current & KEEP_BEHIND
        }
    } else {
        (color & !FLAG_TARGET_2) | (current & FLAG_OBJWIN)
    }
}

/// As [`blend_objwin`], without the OBJ window.
#[inline]
pub(crate) fn blend_no_objwin(color: u32, current: u32, eva: u32, evb: u32) -> u32 {
    if color >= current {
        if current & FLAG_TARGET_1 != 0 && color & FLAG_TARGET_2 != 0 {
            crate::color::mix(eva, current, evb, color)
        } else {
            current & KEEP_BEHIND
        }
    } else {
        color & !FLAG_TARGET_2
    }
}

/// Composites a layer that is not a 2nd target, with the OBJ window enabled.
#[inline]
pub(crate) fn no_blend_objwin(color: u32, current: u32) -> u32 {
    if color < current {
        color | (current & FLAG_OBJWIN)
    } else {
        current & KEEP_BEHIND
    }
}

/// Composites a layer that is not a 2nd target, without the OBJ window.
#[inline]
pub(crate) fn no_blend_no_objwin(color: u32, current: u32) -> u32 {
    if color >= current {
        current & KEEP_BEHIND
    } else {
        color
    }
}
