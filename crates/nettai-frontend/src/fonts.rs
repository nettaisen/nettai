//! The game's two fonts, as the frontend draws strings with them. Every
//! string the custom screen draws goes through these helpers (the HUD's
//! older sites join them later), so that a later step can change what is
//! behind them without touching the places that draw text
//! (docs/design/text-rendering.md).
//!
//! - The 8x16 font ([`cell_text`]): one glyph a cell of 8x16 pixels,
//!   chip names, the enemy names, the Program Advance's names.

use nettai_assets::{Hud, Tiles};

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
}
