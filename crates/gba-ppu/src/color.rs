// Derived from mGBA (include/mgba-util/image.h and
// src/gba/renderers/software-private.h), Copyright (c) 2013-2025 Jeffrey Pfau,
// licensed under the Mozilla Public License, v. 2.0. This file is therefore
// also under the MPL-2.0: https://mozilla.org/MPL/2.0/.

//! Color arithmetic on BGR555 values.
//!
//! mGBA's renderer blends in its native `mColor` format. The mGBA build used
//! as our reference (mgba-rs) defines `COLOR_16_BIT` without `COLOR_5_6_5`,
//! so `mColor` is the GBA's own BGR555 (`r | g << 5 | b << 10`) and all
//! blending happens at 5 bits per channel. These functions reproduce that
//! arithmetic bit for bit, including its rounding: [`darken`] rounds the red
//! channel differently from green and blue because it operates on the
//! channels in place without shifting them down first.
//!
//! Inputs may carry renderer flag bits above bit 15; every function masks
//! them off and returns a plain color.

/// Brightness increase (BLDCNT effect 2): each channel moves `y`/16 of the
/// way toward white. `y` is BLDY, already clamped to 16.
pub(crate) fn brighten(color: u32, y: u32) -> u32 {
    let mut c = 0;
    let a = color & 0x1F;
    c |= (a + ((0x1F - a) * y) / 16) & 0x1F;
    let a = color & 0x3E0;
    c |= (a + ((0x3E0 - a) * y) / 16) & 0x3E0;
    let a = color & 0x7C00;
    c |= (a + ((0x7C00 - a) * y) / 16) & 0x7C00;
    c
}

/// Brightness decrease (BLDCNT effect 3): each channel moves `y`/16 of the
/// way toward black. `y` is BLDY, already clamped to 16.
pub(crate) fn darken(color: u32, y: u32) -> u32 {
    let mut c = 0;
    let a = color & 0x1F;
    c |= (a - (a * y) / 16) & 0x1F;
    let a = color & 0x3E0;
    c |= (a - (a * y) / 16) & 0x3E0;
    let a = color & 0x7C00;
    c |= (a - (a * y) / 16) & 0x7C00;
    c
}

/// Alpha blend: `min(31, (a * weight_a + b * weight_b) / 16)` per channel.
///
/// Weights are EVA/EVB, already clamped to 16. The three channels are
/// computed in one multiply by spreading them apart (green moves to bits
/// 21-25) so each has headroom for its overflow bit, exactly as mGBA does.
pub(crate) fn mix(weight_a: u32, color_a: u32, weight_b: u32, color_b: u32) -> u32 {
    let a = (color_a & 0x7C1F) | ((color_a & 0x3E0) << 16);
    let b = (color_b & 0x7C1F) | ((color_b & 0x3E0) << 16);
    let mut c = (a * weight_a + b * weight_b) / 16;
    if c & 0x0400_0000 != 0 {
        c = (c & !0x07E0_0000) | 0x03E0_0000;
    }
    if c & 0x0020 != 0 {
        c = (c & !0x003F) | 0x001F;
    }
    if c & 0x8000 != 0 {
        c = (c & !0xF800) | 0x7C00;
    }
    (c & 0x7C1F) | ((c >> 16) & 0x03E0)
}

/// Expands a BGR555 color to `0x00RRGGBB`.
///
/// Each 5-bit channel `c` becomes `c * 255 / 31`. This matches
/// `mgba::gba::bgr555_to_rgba8`, the conversion mgba-rs applies to the
/// `COLOR_16_BIT` framebuffer, so frames compare equal to the reference
/// harness's dumps. (mGBA's own 32-bit builds use `c << 3 | c >> 2`
/// instead, which differs for some values, and also blend at 8 bits per
/// channel.) Bit 15 is ignored.
pub const fn bgr555_to_rgb888(color: u16) -> u32 {
    let r = (color & 0x1F) as u32 * 0xFF / 0x1F;
    let g = ((color >> 5) & 0x1F) as u32 * 0xFF / 0x1F;
    let b = ((color >> 10) & 0x1F) as u32 * 0xFF / 0x1F;
    (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(r: u32, g: u32, b: u32) -> u32 {
        r | (g << 5) | (b << 10)
    }

    #[test]
    fn conversion_matches_mgba_rs() {
        assert_eq!(bgr555_to_rgb888(0), 0);
        assert_eq!(bgr555_to_rgb888(0x7FFF), 0xFFFFFF);
        assert_eq!(bgr555_to_rgb888(0xFFFF), 0xFFFFFF);
        assert_eq!(bgr555_to_rgb888(0x001F), 0xFF0000);
        assert_eq!(bgr555_to_rgb888(0x03E0), 0x00FF00);
        assert_eq!(bgr555_to_rgb888(0x7C00), 0x0000FF);
        // 4 * 255 / 31 = 32 (not 33 as with `c << 3 | c >> 2`).
        assert_eq!(bgr555_to_rgb888(rgb(4, 0, 0) as u16), 0x200000);
    }

    #[test]
    fn brighten_and_darken_endpoints() {
        let c = rgb(10, 20, 30);
        assert_eq!(brighten(c, 0), c);
        assert_eq!(brighten(c, 16), 0x7FFF);
        assert_eq!(darken(c, 0), c);
        assert_eq!(darken(c, 16), 0);
        // Flag bits above the color are dropped.
        assert_eq!(brighten(c | 0xFF00_0000, 0), c);
    }

    #[test]
    fn darken_rounds_red_down_and_green_blue_up() {
        // red: 7 - floor(7*3/16) = 6; green/blue: 7 - ceil(7*3/16) = 5.
        assert_eq!(darken(rgb(7, 7, 7), 3), rgb(6, 5, 5));
        // brighten rounds all channels down: 7 + floor(24*3/16) = 11.
        assert_eq!(brighten(rgb(7, 7, 7), 3), rgb(11, 11, 11));
    }

    #[test]
    fn mix_weights_and_clamps() {
        let a = rgb(31, 16, 1);
        let b = rgb(31, 8, 3);
        assert_eq!(mix(16, a, 0, b), a);
        assert_eq!(mix(0, a, 16, b), b);
        // 31*16 + 31*16 saturates; (16*8 + 8*8)/16 = 12; (1*8 + 3*8)/16 = 2.
        assert_eq!(mix(16, a, 16, b), rgb(31, 24, 4));
        assert_eq!(mix(8, a, 8, b), rgb(31, 12, 2));
    }
}
