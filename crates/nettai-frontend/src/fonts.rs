//! The game's two fonts, as the frontend draws strings with them. Every
//! string the frontend draws (the HUD's, the custom screen's, the
//! chatbox's) goes through these helpers, so that a later step can change
//! what is behind them without touching the places that draw text
//! (docs/design/text-rendering.md).
//!
//! - The 8x16 font ([`cell_text`]): one glyph a cell of 8x16 pixels,
//!   chip names, the enemy names, the Program Advance's names.
//! - The dialogue font ([`dialogue_text`]): proportional 16x12 glyphs,
//!   the chatbox's descriptions and messages.

use crate::compose::Layer;
use nettai_assets::{DialogueFont, Hud, Palette, Tiles};

/// A string in the 8x16 font, laid into tiles as `renderTextGfx_8045F8C`
/// does: `cells` glyph cells of two tiles each (top, then bottom), the
/// string's glyphs first and spaces after; every pixel's colour index
/// plus `shift` (the original adds 0, 4, 8 or 12 to each pixel, so a
/// non-zero shift paints the font's blank pixels too). Glyphs past
/// `cells` are cut.
pub fn cell_text(hud: &Hud, glyphs: &[u16], cells: usize, shift: u8) -> Tiles {
    let mut out = Tiles { pixels: Vec::with_capacity(cells * 2 * Tiles::TILE) };
    for k in 0..cells {
        let g = glyphs.get(k).copied().unwrap_or(0) as usize;
        for half in 0..2 {
            match hud.font.get(2 * g + half) {
                Some(t) => out.pixels.extend(t.iter().map(|&v| v.wrapping_add(shift) & 15)),
                None => out.pixels.extend(std::iter::repeat_n(shift & 15, Tiles::TILE)),
            }
        }
    }
    out
}

/// The 8x16 font's glyphs for a string (`Hud::glyphs`), and the characters
/// it has none for.
pub fn cell_glyphs(hud: &Hud, text: &str) -> (Vec<u16>, Vec<char>) {
    hud.glyphs(text)
}

/// Glyphs of the 8x16 font on a tile layer, as `cell_text` lays them
/// (`cells` cells, spaces after the glyphs), the first cell's top left at
/// (x, y).
pub fn draw_cell_text(layer: &mut Layer, hud: &Hud, glyphs: &[u16], cells: usize, palette: &Palette, x: i32, y: i32) {
    let text = cell_text(hud, glyphs, cells, 0);
    for k in 0..cells {
        for half in 0..2 {
            if let Some(t) = text.get(2 * k + half) {
                layer.draw_tile(t, palette, x + 8 * k as i32, y + 8 * half as i32, false, false);
            }
        }
    }
}

/// Where a glyph of the 8x16 font is drawn from as a sprite: its two
/// tiles (top, then bottom) from this one of these.
pub fn cell_glyph(hud: &Hud, glyph: u16) -> (&Tiles, usize) {
    (&hud.font, 2 * glyph as usize)
}

/// A line in the dialogue font, composed as the chatbox composes its line
/// buffer (`sub_3006F8C`): into `image`, palette indices `stride` pixels a
/// row, from row `row` down, each glyph's 16x12 pixels at the pen and
/// OR'd with what is there, the pen moving by the glyph's advance. A
/// glyph's pixels past its advance are cut, but never its first eight (the
/// original masks only the glyph row's second word). Pixels past the
/// image's edge are dropped. Returns the pen's place after the line.
pub fn dialogue_text(font: &DialogueFont, glyphs: &[u16], image: &mut [u8], stride: usize, row: usize) -> usize {
    let (w, h) = (DialogueFont::WIDTH, DialogueFont::HEIGHT);
    let mut pen = 0;
    for &g in glyphs {
        let (Some(pixels), Some(&advance)) = (font.glyph(g as usize), font.advances.get(g as usize)) else { continue };
        let shown = (advance as usize).max(8).min(w);
        for y in 0..h {
            for x in 0..shown {
                let (px, py) = (pen + x, row + y);
                if px < stride && py * stride + px < image.len() {
                    image[py * stride + px] |= pixels[y * w + x];
                }
            }
        }
        pen += advance as usize;
    }
    pen
}

/// The dialogue font's glyphs for a string (`DialogueFont::glyphs`), and
/// the characters it has none for.
pub fn dialogue_glyphs(font: &DialogueFont, text: &str) -> (Vec<u16>, Vec<char>) {
    font.glyphs(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_text_pads_with_spaces_and_shifts_every_pixel() {
        // Glyph 0 (space) blank, glyph 1 all colour 1.
        let mut font = Tiles { pixels: vec![0; 4 * Tiles::TILE] };
        font.pixels[2 * Tiles::TILE..].fill(1);
        let hud = Hud { font, font_chars: vec![" ".into(), "A".into()], ..Hud::default() };
        let (glyphs, missing) = cell_glyphs(&hud, "A");
        assert_eq!((glyphs.clone(), missing), (vec![1], vec![]));
        let t = cell_text(&hud, &glyphs, 2, 8);
        assert_eq!(t.len(), 4);
        assert!(t.get(0).unwrap().iter().all(|&v| v == 9));
        assert!(t.get(2).unwrap().iter().all(|&v| v == 8), "the padding is the shifted blank");
        assert!(cell_text(&hud, &glyphs, 2, 0).get(3).unwrap().iter().all(|&v| v == 0));
    }

    #[test]
    fn dialogue_text_ors_glyphs_at_the_pen_and_cuts_past_the_advance() {
        // Glyph 0: every pixel 1, advance 6 (drawn 8 wide); glyph 1: every
        // pixel 2, advance 10.
        let n = DialogueFont::WIDTH * DialogueFont::HEIGHT;
        let mut pixels = vec![1; n];
        pixels.extend(vec![2; n]);
        let font = DialogueFont { pixels, advances: vec![6, 10], chars: vec!["a".into(), "b".into()] };
        let (glyphs, missing) = dialogue_glyphs(&font, "ab");
        assert_eq!((glyphs.clone(), missing), (vec![0, 1], vec![]));
        let mut image = vec![0; 32 * 13];
        assert_eq!(dialogue_text(&font, &glyphs, &mut image, 32, 1), 16);
        let row = |y: usize| image[32 * y..32 * (y + 1)].to_vec();
        assert!(row(0).iter().all(|&v| v == 0));
        let mut want = vec![1; 6];
        want.extend([3, 3]);
        want.extend([2; 8]);
        want.extend([0; 16]);
        assert_eq!(row(1), want);
        assert_eq!(row(12), want);
    }
}
