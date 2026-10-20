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
//! the face box's height, set as the font sets it (`laid`): each digit the
//! digits' one advance at that height (the widest digit's: a number doesn't
//! move as it changes, and its columns line up), a sign its own advance with
//! clear space either side, the run placed against the end the picture's
//! number sits against (its right, its left, its middle). Every number has
//! the same spacing for its height.

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

/// A character's advance in the numbers' font, over its capitals' height.
fn advance(c: char) -> f32 {
    let f = font().font();
    let cap = f.metrics(&[]).cap_height.max(1.0);
    f.glyph_metrics(&[]).advance_width(f.charmap().map(c)) / cap
}

/// A character's ink's width and height in the numbers' font, over its
/// capitals' height (a sign's box is its ink's height: `Fitted`).
fn ink(c: char) -> (f32, f32) {
    let f = font().font();
    let cap = f.metrics(&[]).cap_height.max(1.0);
    let mut ctx = swash::scale::ScaleContext::new();
    let mut scaler = ctx.builder(f).build();
    scaler.scale_outline(f.charmap().map(c)).map_or((0.0, 1.0), |o| {
        let b = o.bounds();
        ((b.max.x - b.min.x) / cap, ((b.max.y - b.min.y) / cap).max(1e-3))
    })
}

/// The advance every digit takes, over the capitals' height: the widest
/// digit's, so a number's digits stand in columns and it doesn't shift as
/// it changes.
fn digit_advance() -> f32 {
    static ADVANCE: std::sync::LazyLock<f32> = std::sync::LazyLock::new(|| ('0'..='9').map(advance).fold(0.0, f32::max));
    *ADVANCE
}

/// The clear space either side of a sign ('+', the "x" of "x2"), over the
/// digits' height beside it: more than its own advance leaves, so it never
/// touches a digit.
const SIGN_SPACE: f32 = 0.3;

/// Where a run of numbers is placed: its left edge, its right edge or its
/// middle at this x (frame pixels).
#[derive(Clone, Copy, Debug)]
pub enum Anchor {
    Left(f32),
    Right(f32),
    Center(f32),
}

/// Where the picture's number of `n` glyphs of `set` from cell column `x`
/// (frame pixels) has its ink: its first face's left and its last's right.
pub fn picture_span(set: &NumberSet, x: i32, n: usize) -> (f32, f32) {
    let [fx, _, fw, _] = set.face.map(|v| v as f32);
    (x as f32 + fx, x as f32 + (CELL * n.saturating_sub(1) as i32) as f32 + fx + fw)
}

/// Numbers and signs in their sets' looks laid as one run on cells' rows
/// `y` as the font sets them, each part at its set's face box's height
/// and rows: each digit the digits' one advance, a sign its own advance
/// and clear space either side; placed by `anchor`, and where the run
/// would be wider than `room` (left, right), narrowed alike to fit it. An
/// item for each part, and where the run starts and ends. None where the
/// text layer draws no numbers (the original mode), or the font lacks a
/// character.
pub fn laid(text: &TextSink, parts: &[(String, &NumberSet)], y: i32, anchor: Anchor, room: Option<(f32, f32)>) -> Option<(Vec<TextItem>, (f32, f32))> {
    if !text.draws_numbers() || parts.iter().any(|(chars, _)| !font().covers(chars)) {
        return None;
    }
    // (A sign's clear space by the run's digits' height, its tallest.)
    let tallest = parts.iter().map(|(_, set)| set.face[3] as f32).fold(0.0, f32::max);
    // (A sign is drawn its ink the box's height: its width its ink's at
    // that height; its clear space either side kept however the run is
    // narrowed.)
    let pad = SIGN_SPACE * tallest;
    let is_digit = |c: char| c.is_ascii_digit() || c == '?';
    let width = |c: char, h: f32| {
        if is_digit(c) {
            digit_advance() * h
        } else {
            let (w, ih) = ink(c);
            w / ih * h
        }
    };
    let chars = || parts.iter().flat_map(|(chars, set)| chars.chars().filter(|c| *c != ' ').map(move |c| (c, set.face[3] as f32)));
    let glyphs: f32 = chars().map(|(c, h)| width(c, h)).sum();
    let pads = 2.0 * pad * chars().filter(|(c, _)| !is_digit(*c)).count() as f32;
    let k = match room {
        Some((a, b)) if glyphs + pads > b - a && glyphs > 0.0 => ((b - a - pads) / glyphs).clamp(0.5, 1.0),
        _ => 1.0,
    };
    let total = glyphs * k + pads;
    let mut start = match anchor {
        Anchor::Left(x) => x,
        Anchor::Right(x) => x - total,
        Anchor::Center(x) => x - total / 2.0,
    };
    if let Some((a, b)) = room {
        start = start.min(b - total).max(a);
    }
    let mut out = Vec::new();
    let mut at = start;
    for (chars, set) in parts {
        let [_, top, _, h] = set.face.map(|v| v as f32);
        let left = at.floor();
        let mut glyphs = Vec::new();
        for c in chars.chars().filter(|c| *c != ' ') {
            let space = if is_digit(c) { 0.0 } else { pad };
            let w = width(c, h) * k;
            glyphs.push((c, [at - left + space, top, w, h]));
            at += w + 2.0 * space;
        }
        let rect = Rect::new(left as i32, y, (at - left).ceil() as i32 + 1, 16);
        let f = Fitted { glyphs, set: set.set.clone(), family: Vec::new(), fill: set.fill.clone(), rim: set.rim, shadow: set.shadow };
        out.push(TextItem::fitted(f, rect));
    }
    Some((out, (start, at)))
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
