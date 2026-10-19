//! The game's numbers in the enhanced mode: each digit (and the '+', the
//! "x" of "x2", the '?' marks) drawn in a bundled font, Bungee (fonts/bungee,
//! SIL Open Font License 1.1), at the output's resolution, fitted to the box
//! the game's own digit picture fills in its cell (the user, 2026-10-19: "i
//! like bungee"; docs/design/text-rendering.md §12.4).
//!
//! A set of the game's number glyphs (the HP box's digits, the enemies' in
//! each color, the damage digits, the chip window's, a telop's, the
//! banners') says how its numbers look: the box its faces fill in their
//! 8x16 cells, the face's colors down its rows, and its outline (a rim all
//! round, or a shadow right and down), read off the glyphs as the lettering
//! is ([`crate::lettering::letters`]). Each character is drawn in the
//! glyph's cell, its ink stretched to fill that box, so a number's digits
//! stand where the picture's did, each the same width: a number doesn't
//! move as it changes, and its columns line up. Nothing new is in the pack.

use crate::lettering::{Capitals, Picture};
use crate::textlayer::{Rect, TextItem, TextSink};
use nettai_assets::{Palette, Tiles};

/// Bungee: the numbers' font.
pub const FONT: &[u8] = include_bytes!("../fonts/bungee/Bungee-Regular.ttf");

/// The numbers' font, parsed once.
pub fn font() -> &'static std::sync::Arc<crate::vfont::VectorFont> {
    static FONT_PARSED: std::sync::LazyLock<std::sync::Arc<crate::vfont::VectorFont>> =
        std::sync::LazyLock::new(|| std::sync::Arc::new(crate::vfont::VectorFont::from_bytes(FONT, "Bungee (bundled)").expect("the bundled numbers font parses")));
    &FONT_PARSED
}

/// A game cell's width: a character a cell.
pub const CELL: i32 = 8;

/// Characters of a number drawn in the numbers' font
/// ([`TextItem::fitted`]): each one's face box (x, y, width, height in
/// frame pixels from the item's box's top left), and the paint.
#[derive(Clone, Debug, PartialEq)]
pub struct Fitted {
    pub glyphs: Vec<(char, [f32; 4])>,
    /// The face's colors down the boxes' rows (BGR555), its rim's and its
    /// shadow's.
    pub fill: Vec<u16>,
    pub rim: Option<u16>,
    pub shadow: Option<u16>,
}

/// How a set of the game's number glyphs looks ([`NumberSet::of`]).
#[derive(Clone, Debug, PartialEq)]
pub struct NumberSet {
    /// The box its faces fill in their cells: x, y, width, height.
    pub face: [i32; 4],
    pub fill: Vec<u16>,
    pub rim: Option<u16>,
    pub shadow: Option<u16>,
}

impl NumberSet {
    /// The set of `count` glyphs of 8x16 from `first` of `tiles` (glyph k
    /// is tiles 2k and 2k + 1), `ground` behind them (their cells'
    /// background), in `palette`.
    pub fn of(tiles: &Tiles, first: usize, count: usize, ground: impl Fn(u8) -> bool, palette: &Palette) -> Option<NumberSet> {
        let p = Picture::glyphs(tiles, first, count);
        let l = crate::lettering::letters(&p, &ground, Capitals::All)?;
        let line = l.rim.or(l.shadow);
        let face = |x: usize, y: usize| {
            let v = p.pixels[y * p.w + x];
            !ground(v) && Some(v) != line
        };
        // The faces' columns in their cells, all the cells' together.
        let columns: Vec<usize> = (0..8).filter(|&x| (0..count).any(|c| (0..p.h).any(|y| face(8 * c + x, y)))).collect();
        let (x0, x1) = (*columns.first()? as i32, *columns.last()? as i32);
        let color = |c: u8| palette[c as usize & 15];
        Some(NumberSet {
            face: [x0, l.top as i32, x1 + 1 - x0, l.cap as i32],
            fill: l.fill.iter().map(|&c| color(c)).collect(),
            rim: l.rim.map(color),
            shadow: l.shadow.map(color),
        })
    }

    /// `chars` in this set's look, a character a cell from the left.
    pub fn fitted(&self, chars: &str) -> Fitted {
        let [x, y, w, h] = self.face.map(|v| v as f32);
        let glyphs = chars.chars().enumerate().filter(|(_, c)| *c != ' ').map(|(i, c)| (c, [(CELL * i as i32) as f32 + x, y, w, h])).collect();
        Fitted { glyphs, fill: self.fill.clone(), rim: self.rim, shadow: self.shadow }
    }
}

/// Numbers and signs in their sets' looks set one after another from
/// (x, y), a character an 8-pixel cell as the game's glyphs stand: an item
/// for each part. None where the text layer draws no numbers (the original
/// mode), or the font lacks a character.
pub fn items(text: &TextSink, parts: &[(String, &NumberSet)], (x, y): (i32, i32)) -> Option<Vec<TextItem>> {
    if !text.draws_numbers() {
        return None;
    }
    let mut out = Vec::new();
    let mut at = x;
    for (chars, set) in parts {
        if !font().covers(chars) {
            return None;
        }
        let n = chars.chars().count() as i32;
        out.push(TextItem::fitted(set.fitted(chars), Rect::new(at, y, CELL * n, 16)));
        at += CELL * n;
    }
    Some(out)
}

/// The character the font draws for the game's "x" of "x2".
pub fn times() -> &'static str {
    if font().covers("×") { "×" } else { "X" }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A set of two digits with a rim: its face box is the faces' columns
    /// and rows, in their cells; its colors the palette's.
    #[test]
    fn a_sets_face_box_and_paint() {
        // Two glyphs: a face of color 1 in columns 1 to 6, rows 4 to 11, a
        // rim of 6 around it.
        let mut tiles = Tiles { pixels: vec![0; 4 * Tiles::TILE] };
        for c in 0..2 {
            for y in 3..13 {
                for x in 0..8 {
                    let v = if (4..12).contains(&y) && (1..7).contains(&x) { 1 } else if (3..13).contains(&y) { 6 } else { 0 };
                    let (tile, row) = (2 * c + y / 8, y % 8);
                    tiles.pixels[tile * Tiles::TILE + row * 8 + x] = v;
                }
            }
        }
        let palette: Palette = std::array::from_fn(|i| i as u16 * 0x421);
        let set = NumberSet::of(&tiles, 0, 2, |v| v == 0, &palette).expect("a set");
        assert_eq!(set.face, [1, 4, 6, 8]);
        assert_eq!((set.rim, set.shadow), (Some(palette[6]), None));
        assert!(set.fill.iter().all(|&c| c == palette[1]));
        let f = set.fitted("10");
        assert_eq!(f.glyphs, vec![('1', [1.0, 4.0, 6.0, 8.0]), ('0', [9.0, 4.0, 6.0, 8.0])]);
    }

    #[test]
    fn the_font_has_the_numbers_and_signs() {
        assert!(font().covers("0123456789+?"));
        assert!(font().covers(times()));
    }
}
