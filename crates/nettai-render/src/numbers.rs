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
//! round, or a shadow right and down), as the pack has read them off the
//! glyphs (`nettai_assets::letters::Digits`). Each character is drawn in
//! the glyph's cell, its ink stretched to fill that box, so a number's
//! digits stand where the picture's did, each the same width: a number
//! doesn't move as it changes, and its columns line up. The numbers after
//! a chip's name are set as the font sets them, tabular, as tall as the
//! name's capitals ([`after_words`]).

use crate::textlayer::{Rect, TextItem, TextSink};
use nettai_assets::Palette;
use nettai_assets::letters::Digits;

/// Bungee: the numbers' font.
pub const FONT: &[u8] = include_bytes!("../fonts/bungee/Bungee-Regular.ttf");

/// The numbers' font, parsed once.
pub fn font() -> &'static std::sync::Arc<crate::vfont::VectorFont> {
    static FONT_PARSED: std::sync::LazyLock<std::sync::Arc<crate::vfont::VectorFont>> =
        std::sync::LazyLock::new(|| std::sync::Arc::new(crate::vfont::VectorFont::from_bytes(FONT, "Bungee (bundled)").expect("the bundled numbers font parses")));
    &FONT_PARSED
}

/// A set of digits' characters (and the chip window's '?').
pub const DIGITS: &str = "0123456789?";

/// The chip codes' characters.
pub const CODES: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ*";

impl NumberSet {
    /// The set as the set of `chars` (a sign's: '+', "x2").
    pub fn with_set(self, chars: &str) -> NumberSet {
        NumberSet { set: chars.into(), ..self }
    }

    /// The set as one family with other sets beside it (`Fitted::family`).
    pub fn with_family(self, family: Vec<(String, f32)>) -> NumberSet {
        NumberSet { family, ..self }
    }
}

/// A game cell's width: a character a cell.
pub const CELL: i32 = 8;

/// Characters of a number drawn in the numbers' font
/// ([`TextItem::fitted`]): each one's face box (x, y, width, height in
/// frame pixels from the item's box's top left), the characters of its set
/// (`set`: the glyphs are drawn at one scale, the set's tallest ink the
/// boxes' height, at their own widths, narrowed alike where the set's
/// widest would be wider than its box, each centered in its box), and the
/// paint.
#[derive(Clone, Debug, PartialEq)]
pub struct Fitted {
    pub glyphs: Vec<(char, [f32; 4])>,
    pub set: String,
    /// Other sets drawn beside it as one family (the chip window's code and
    /// its damage), each with its boxes' width: the family is narrowed alike,
    /// by the least any of its sets needs.
    pub family: Vec<(String, f32)>,
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
    /// Its characters (`Fitted::set`), and its family's (`Fitted::family`).
    pub set: String,
    pub family: Vec<(String, f32)>,
    pub fill: Vec<u16>,
    pub rim: Option<u16>,
    pub shadow: Option<u16>,
}

impl NumberSet {
    /// A set's look (the pack's, read off its glyphs) in `palette`.
    pub fn of(d: &Digits, palette: &Palette) -> NumberSet {
        let color = |c: u8| palette[c as usize & 15];
        NumberSet {
            face: d.face.map(|v| v as i32),
            set: DIGITS.into(),
            family: Vec::new(),
            fill: d.fill.iter().map(|&c| color(c)).collect(),
            rim: d.rim.map(color),
            shadow: d.shadow.map(color),
        }
    }

    /// `chars` in this set's look, a character a cell from the left.
    pub fn fitted(&self, chars: &str) -> Fitted {
        let [x, y, w, h] = self.face.map(|v| v as f32);
        let glyphs = chars.chars().enumerate().filter(|(_, c)| *c != ' ').map(|(i, c)| (c, [(CELL * i as i32) as f32 + x, y, w, h])).collect();
        Fitted { glyphs, set: self.set.clone(), family: self.family.clone(), fill: self.fill.clone(), rim: self.rim, shadow: self.shadow }
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

/// A character's advance in the numbers' font, over its capitals' height.
fn advance(c: char) -> f32 {
    let f = font().font();
    let cap = f.metrics(&[]).cap_height.max(1.0);
    f.glyph_metrics(&[]).advance_width(f.charmap().map(c)) / cap
}

/// The advance every digit takes, over the capitals' height: the widest
/// digit's, so a number's digits stand in columns and it doesn't shift as
/// it changes.
fn digit_advance() -> f32 {
    static ADVANCE: std::sync::LazyLock<f32> = std::sync::LazyLock::new(|| ('0'..='9').map(advance).fold(0.0, f32::max));
    *ADVANCE
}

/// Numbers after a name's words (a chip's damage, its bonus's '+', its
/// "x2"), in their sets' looks, set as the font sets them from `x` (frame
/// pixels) on cells' rows `y`: their capitals `cap` tall standing on
/// `baseline` (down from the row's top), as the name's beside them are;
/// each digit the digits' one advance, a sign its own. Where they would
/// run past `limit` (where the original's numbers ended), all are narrowed
/// alike to end there. An item for each part, and where they end. None
/// where the text layer draws no numbers, or the font lacks a character.
pub fn after_words(text: &TextSink, parts: &[(String, &NumberSet)], (x, y): (f32, i32), (cap, baseline): (f32, f32), limit: Option<f32>) -> Option<(Vec<TextItem>, f32)> {
    if !text.draws_numbers() || parts.iter().any(|(chars, _)| !font().covers(chars)) {
        return None;
    }
    let pitch = |c: char| if c.is_ascii_digit() { digit_advance() } else { advance(c) } * cap;
    let natural: f32 = parts.iter().flat_map(|(chars, _)| chars.chars()).map(pitch).sum();
    let k = match limit {
        Some(l) if x + natural > l && natural > 0.0 => ((l - x) / natural).max(0.5),
        _ => 1.0,
    };
    let mut out = Vec::new();
    let mut at = x;
    for (chars, set) in parts {
        let left = at.floor();
        let mut glyphs = Vec::new();
        for c in chars.chars() {
            let w = pitch(c) * k;
            glyphs.push((c, [at - left, baseline - cap, w, cap]));
            at += w;
        }
        let rect = Rect::new(left as i32, y, (at - left).ceil() as i32 + 1, 16);
        let f = Fitted { glyphs, set: set.set.clone(), family: Vec::new(), fill: set.fill.clone(), rim: set.rim, shadow: set.shadow };
        out.push(TextItem::fitted(f, rect));
    }
    Some((out, at))
}

/// The character the font draws for the game's "x" of "x2".
pub fn times() -> &'static str {
    if font().covers("×") { "×" } else { "X" }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A set of digits with a rim: its face box, its colors the palette's,
    /// a digit a cell.
    #[test]
    fn a_sets_face_box_and_paint() {
        let palette: Palette = std::array::from_fn(|i| i as u16 * 0x421);
        let set = NumberSet::of(&Digits { face: [1, 4, 6, 8], fill: vec![1; 8], rim: Some(6), shadow: None }, &palette);
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
