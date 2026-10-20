//! The game's numbers in the enhanced mode: each digit (and the '+', the
//! "x" of "x2", the '?' marks) drawn in a bundled font, Saira at width 75
//! and weight 700 (fonts/saira, SIL Open Font License 1.1), at the output's
//! resolution, as tall as the box the game's own digit picture fills in its
//! cell (docs/design/text-rendering.md §12.4). The numbers were Bungee first
//! (the user, 2026-10-19: "i like bungee"), whose wide digits had to be
//! squeezed to fit the game's places; the user switched to Teko 600, a
//! condensed face that fits them as it is, on 2026-10-20 ("can you use
//! teko 600 without squeezing?"), and the same day to Saira 75/700 ("use
//! saira 75/700"), condensed too and with tabular figures, which Teko
//! lacks. Bungee stays the HUD's lettering font ([`font`]: the words on the
//! buttons, the window's labels, the chip codes).
//!
//! A set of the game's number glyphs (the HP box's digits, the enemies' in
//! each color, the damage digits, the chip window's, a telop's, the
//! banners') says how its numbers look: the box its faces fill in their
//! 8x16 cells, the face's colors down its rows, and its outline (a rim all
//! round, or a shadow right and down), as the pack has read them off the
//! glyphs (`nettai_assets::letters::Digits`). Each character is drawn in
//! the face box's height, set as the font sets it (`laid`): one shaped run
//! with the font's kerning and its tabular figures (each digit the one
//! advance: a number doesn't move as it changes, and its columns line up),
//! the run placed against the end the picture's number sits against (its
//! right, its left, its middle, or its first ink after a chip's name).
//! Every number has the same spacing for its height.

use crate::textlayer::{Rect, TextItem, TextSink};
use nettai_assets::Palette;
use nettai_assets::letters::Digits;

/// Bungee: the HUD's lettering font (its words in capitals, the chip codes).
pub const FONT: &[u8] = include_bytes!("../fonts/bungee/Bungee-Regular.ttf");

/// The HUD's lettering font, parsed once.
pub fn font() -> &'static std::sync::Arc<crate::vfont::VectorFont> {
    static FONT_PARSED: std::sync::LazyLock<std::sync::Arc<crate::vfont::VectorFont>> =
        std::sync::LazyLock::new(|| std::sync::Arc::new(crate::vfont::VectorFont::from_bytes(FONT, "Bungee (bundled)").expect("the bundled numbers font parses")));
    &FONT_PARSED
}

/// Saira: the numbers' font.
pub const DIGITS_FONT: &[u8] = include_bytes!("../fonts/saira/Saira-VariableFont_wdth,wght.ttf");

/// The numbers' font, parsed once.
pub fn digits_font() -> &'static std::sync::Arc<crate::vfont::VectorFont> {
    static FONT_PARSED: std::sync::LazyLock<std::sync::Arc<crate::vfont::VectorFont>> =
        std::sync::LazyLock::new(|| std::sync::Arc::new(crate::vfont::VectorFont::from_bytes(DIGITS_FONT, "Saira (bundled)").expect("the bundled numbers font parses")));
    &FONT_PARSED
}

/// Which font a [`Fitted`] is drawn in: the numbers' (Saira) or the
/// lettering's (Bungee: the chip codes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Digits,
    Letters,
}

impl Face {
    pub fn font(self) -> &'static std::sync::Arc<crate::vfont::VectorFont> {
        match self {
            Face::Digits => digits_font(),
            Face::Letters => font(),
        }
    }

    /// Its variable axes' settings.
    pub fn axes(self) -> &'static [(&'static str, f32)] {
        match self {
            Face::Digits => &[("wdth", 75.0), ("wght", 700.0)],
            Face::Letters => &[],
        }
    }
}

/// The numbers' font's normalized coordinates at its axes' settings.
fn coords() -> Vec<swash::NormalizedCoord> {
    let f = digits_font().font();
    f.variations().normalized_coords(Face::Digits.axes().iter().map(|&(t, v)| swash::Setting { tag: swash::tag_from_str_lossy(t), value: v })).collect()
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
    /// Its font.
    pub face: Face,
    /// Each glyph's glyph id, where its run was shaped (a tabular figure);
    /// none: its character's.
    pub ids: Vec<u16>,
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
        Fitted { glyphs, set: self.set.clone(), family: self.family.clone(), fill: self.fill.clone(), rim: self.rim, shadow: self.shadow, face: Face::Digits, ids: Vec::new() }
    }
}

/// A glyph of the numbers' font's ink box, in font units: left, bottom,
/// right, top (none: no ink).
fn ink_box(glyph: u16) -> Option<[f32; 4]> {
    let f = digits_font().font();
    let mut ctx = swash::scale::ScaleContext::new();
    let mut scaler = ctx.builder(f).size(f.metrics(&coords()).units_per_em as f32).variations(Face::Digits.axes().iter().copied()).build();
    scaler.scale_outline(glyph).map(|o| {
        let b = o.bounds();
        [b.min.x, b.min.y, b.max.x, b.max.y]
    })
}

/// `text` shaped in the numbers' font (kerned, its tabular figures), in font
/// units: each glyph's character, glyph, offset and advance.
fn shaped(text: &str) -> Vec<(char, u16, f32, f32)> {
    let f = digits_font().font();
    let upem = f.metrics(&coords()).units_per_em as f32;
    let mut ctx = swash::shape::ShapeContext::new();
    let mut shaper = ctx.builder(f).size(upem).variations(Face::Digits.axes().iter().copied()).features([("kern", 1u16), ("tnum", 1)]).build();
    shaper.add_str(text);
    let chars: Vec<char> = text.chars().collect();
    let starts: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    let mut out = Vec::new();
    shaper.shape_with(|cluster| {
        let at = starts.iter().position(|&i| i == cluster.source.start as usize).unwrap_or(0);
        for g in cluster.glyphs {
            out.push((chars.get(at).copied().unwrap_or(' '), g.id, g.x, g.advance));
        }
    });
    out
}

/// Where a run of numbers is placed: its advances' left edge, right edge or
/// middle at this x (frame pixels), or its first glyph's ink's left edge
/// (`LeftInk`: after words, ink to ink).
#[derive(Clone, Copy, Debug)]
pub enum Anchor {
    Left(f32),
    Right(f32),
    Center(f32),
    LeftInk(f32),
}

/// Where the picture's number of `n` glyphs of `set` from cell column `x`
/// (frame pixels) has its ink: its first face's left and its last's right.
pub fn picture_span(set: &NumberSet, x: i32, n: usize) -> (f32, f32) {
    let [fx, _, fw, _] = set.face.map(|v| v as f32);
    (x as f32 + fx, x as f32 + (CELL * n.saturating_sub(1) as i32) as f32 + fx + fw)
}

/// Numbers and signs in their sets' looks set as one run of the numbers'
/// font on cells' rows `y`, with the font's own metrics (shaped, kerned, its
/// tabular figures: every digit one advance), each part at its set's face
/// box's height and rows, never squeezed; placed by `anchor`. An item for
/// each part, and where the run's ink starts and ends. None where the text
/// layer draws no numbers (the original mode), or the font lacks a
/// character.
pub fn laid(text: &TextSink, parts: &[(String, &NumberSet)], y: i32, anchor: Anchor) -> Option<(Vec<TextItem>, (f32, f32))> {
    if !text.draws_numbers() || parts.iter().any(|(chars, _)| !digits_font().covers(chars)) {
        return None;
    }
    let f = digits_font().font();
    // (Each part's frame pixels a font unit: its set's ink the box's
    // height, as `Fitted` draws it.)
    let unit = |set: &NumberSet| {
        let mut heights: Vec<f32> = set.set.chars().filter_map(|c| ink_box(f.charmap().map(c))).map(|b| b[3] - b[1]).collect();
        heights.sort_by(f32::total_cmp);
        set.face[3] as f32 / heights.get(heights.len() / 2).copied().unwrap_or(1.0).max(1e-3)
    };
    // (The whole run shaped as one string, each glyph its part's.)
    let whole: String = parts.iter().map(|(chars, _)| chars.replace(' ', "")).collect();
    let mut owner = Vec::new();
    for (k, (chars, set)) in parts.iter().enumerate() {
        let u = unit(set);
        owner.extend(chars.chars().filter(|c| *c != ' ').map(|_| (k, u)));
    }
    let glyphs = shaped(&whole);
    // (Each glyph's ink, from the run's start, and the advances' total.)
    let mut inks: Vec<(usize, char, u16, f32, f32)> = Vec::new();
    let mut pen = 0.0;
    for (i, &(c, glyph, dx, a)) in glyphs.iter().enumerate() {
        let Some(&(k, u)) = owner.get(i) else { break };
        if let Some([x0, _, x1, _]) = ink_box(glyph) {
            inks.push((k, c, glyph, pen + (dx + x0) * u, (x1 - x0) * u));
        }
        pen += a * u;
    }
    let first_ink = inks.first().map_or(0.0, |g| g.3);
    let start = match anchor {
        Anchor::Left(x) => x,
        Anchor::Right(x) => x - pen,
        Anchor::Center(x) => x - pen / 2.0,
        Anchor::LeftInk(x) => x - first_ink,
    };
    let mut boxes: Vec<(Vec<(char, [f32; 4])>, Vec<u16>)> = vec![(Vec::new(), Vec::new()); parts.len()];
    let (mut ink_start, mut ink_end) = (f32::MAX, f32::MIN);
    for &(k, c, glyph, x, w) in &inks {
        let [_, top, _, h] = parts[k].1.face.map(|v| v as f32);
        boxes[k].0.push((c, [start + x, top, w, h]));
        boxes[k].1.push(glyph);
        ink_start = ink_start.min(start + x);
        ink_end = ink_end.max(start + x + w);
    }
    let mut out = Vec::new();
    for ((_, set), (glyphs, ids)) in parts.iter().zip(boxes) {
        let Some(left) = glyphs.iter().map(|g| g.1[0]).reduce(f32::min).map(f32::floor) else { continue };
        let right = glyphs.iter().map(|g| g.1[0] + g.1[2]).fold(left, f32::max);
        let glyphs = glyphs.into_iter().map(|(c, [x, top, w, h])| (c, [x - left, top, w, h])).collect();
        let rect = Rect::new(left as i32, y, (right - left).ceil() as i32 + 1, 16);
        let f = Fitted { glyphs, set: set.set.clone(), family: Vec::new(), fill: set.fill.clone(), rim: set.rim, shadow: set.shadow, face: Face::Digits, ids };
        out.push(TextItem::fitted(f, rect));
    }
    Some((out, (ink_start, ink_end)))
}

/// The character the font draws for the game's "x" of "x2".
pub fn times() -> &'static str {
    if digits_font().covers("×") { "×" } else { "X" }
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

    /// The numbers' font's tabular figures take effect through the run's
    /// shaping: every digit one advance.
    #[test]
    fn digits_are_tabular() {
        let advances: Vec<f32> = shaped("0123456789").iter().map(|g| g.3).collect();
        eprintln!("the digits' advances through the shaper (font units): {advances:?}");
        assert_eq!(advances.len(), 10);
        assert!(advances.iter().all(|&a| a == advances[0]), "{advances:?}");
        // (Without the feature they differ: the font's own figures are
        // proportional.)
        assert!(digits_font().font().features().any(|f| f.tag() == swash::tag_from_bytes(b"tnum")));
    }

    #[test]
    fn the_font_has_the_numbers_and_signs() {
        assert!(font().covers("0123456789+?"));
        assert!(font().covers(times()));
    }
}
