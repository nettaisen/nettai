//! The game's numbers in the enhanced mode: each digit (and the '+', the
//! "x" of "x2", the '?' marks) drawn in a font made of the game's own
//! digits: Nettai Digits (fonts/nettai-digits: the HP box's digits, the
//! HUD's '?', '+' and "x", traced from their pixels with sharp corners) and,
//! for the numbered banners, Nettai Banner Digits (fonts/nettai-banner-digits:
//! the banners' own drawing), at the output's resolution
//! (docs/design/text-rendering.md §12.4). The numbers were Bungee first (the
//! user, 2026-10-19: "i like bungee"), then Teko 600 and Saira 75/700 (the
//! user, 2026-10-20: "use saira 75/700"); on 2026-10-21 the user asked for
//! the game's own ("could you trace a vector font of the game's numbers?",
//! "you should just create the font yourself as an otf and commit it", "the
//! font should have sharp edges and not rounded", "can it not just be the
//! same base font? and exclude the variants that don't fit", the banners'
//! "actually a different font", then "replace them outright"). The chip
//! codes are fonts of the game's codes the same way, Nettai Chip Codes (the
//! chip window's) and Nettai Slot Codes (the slots'; the user: "you should
//! trace ABCDEFGHIJKLMNOPQRSTUVWXYZ* for the chip codes font like you did
//! digits", §12.11). Bungee stays the HUD's lettering font ([`font`]: the
//! words on the buttons, the window's labels).
//!
//! The digit fonts keep the game's metrics: a glyph pixel is 100 units, each
//! glyph in the game's 8-pixel cell (its advance), its face 6 pixels wide a
//! pixel in from the cell's left. A set of the game's number glyphs (the HP
//! box's digits, the enemies' in each color, the damage digits, the chip
//! window's, a telop's, the banners') says how its numbers look: the box its
//! faces fill in their 8x16 cells, the face's colors down its rows, and its
//! outline (a rim all round, or a shadow right and down), as the pack has
//! read them off the glyphs (`nettai_assets::letters::Digits`). Each
//! character is drawn a glyph pixel a frame pixel across and as tall as its
//! set's face box (the user, of the game's own sets: "aren't the fonts just
//! stretched or boldened or condensed or whatever?": the enemies' 8 pixels
//! tall, the chip window's 11, at one width), set as the font sets it
//! (`laid`: one run, each digit the one advance, a game cell), the run's
//! faces placed where the picture's are (against its right, its left, its
//! middle) or its first ink after a chip's name. So a number has the
//! original's spacing and box.

use crate::textlayer::{Rect, TextItem, TextSink};
use nettai_assets::Palette;
use nettai_assets::letters::Digits;

/// Bungee: the HUD's lettering font (its words in capitals: the window's
/// labels).
pub const FONT: &[u8] = include_bytes!("../fonts/bungee/Bungee-Regular.ttf");

/// The HUD's lettering font, parsed once.
pub fn font() -> &'static std::sync::Arc<crate::vfont::VectorFont> {
    static FONT_PARSED: std::sync::LazyLock<std::sync::Arc<crate::vfont::VectorFont>> =
        std::sync::LazyLock::new(|| std::sync::Arc::new(crate::vfont::VectorFont::from_bytes(FONT, "Bungee (bundled)").expect("the bundled numbers font parses")));
    &FONT_PARSED
}

/// Nettai Digits: the numbers' font (the game's own digits).
pub const DIGITS_FONT: &[u8] = include_bytes!("../fonts/nettai-digits/NettaiDigits-Regular.otf");

/// The numbers' font, parsed once.
pub fn digits_font() -> &'static std::sync::Arc<crate::vfont::VectorFont> {
    static FONT_PARSED: std::sync::LazyLock<std::sync::Arc<crate::vfont::VectorFont>> =
        std::sync::LazyLock::new(|| std::sync::Arc::new(crate::vfont::VectorFont::digits_from_bytes(DIGITS_FONT, "Nettai Digits (bundled)").expect("the bundled numbers font parses")));
    &FONT_PARSED
}

/// Nettai Banner Digits: the numbered banners' font (the game's banner
/// digits, another drawing).
pub const BANNER_DIGITS_FONT: &[u8] = include_bytes!("../fonts/nettai-banner-digits/NettaiBannerDigits-Regular.otf");

/// The banners' numbers' font, parsed once.
pub fn banner_digits_font() -> &'static std::sync::Arc<crate::vfont::VectorFont> {
    static FONT_PARSED: std::sync::LazyLock<std::sync::Arc<crate::vfont::VectorFont>> = std::sync::LazyLock::new(|| {
        std::sync::Arc::new(crate::vfont::VectorFont::digits_from_bytes(BANNER_DIGITS_FONT, "Nettai Banner Digits (bundled)").expect("the bundled banner numbers font parses"))
    });
    &FONT_PARSED
}

/// Nettai Chip Codes: the chip window's codes' font (the game's big chip
/// codes, A to Z and '*', 7 by 11 pixels).
pub const CHIP_CODES_FONT: &[u8] = include_bytes!("../fonts/nettai-chip-codes/NettaiChipCodes-Regular.otf");

/// The chip window's codes' font, parsed once.
pub fn chip_codes_font() -> &'static std::sync::Arc<crate::vfont::VectorFont> {
    static FONT_PARSED: std::sync::LazyLock<std::sync::Arc<crate::vfont::VectorFont>> =
        std::sync::LazyLock::new(|| std::sync::Arc::new(crate::vfont::VectorFont::from_bytes(CHIP_CODES_FONT, "Nettai Chip Codes (bundled)").expect("the bundled chip codes font parses")));
    &FONT_PARSED
}

/// Nettai Slot Codes: the slots' codes' font (the game's small chip codes
/// under the slots' icons, 6 by 5 pixels).
pub const SLOT_CODES_FONT: &[u8] = include_bytes!("../fonts/nettai-slot-codes/NettaiSlotCodes-Regular.otf");

/// The slots' codes' font, parsed once.
pub fn slot_codes_font() -> &'static std::sync::Arc<crate::vfont::VectorFont> {
    static FONT_PARSED: std::sync::LazyLock<std::sync::Arc<crate::vfont::VectorFont>> =
        std::sync::LazyLock::new(|| std::sync::Arc::new(crate::vfont::VectorFont::from_bytes(SLOT_CODES_FONT, "Nettai Slot Codes (bundled)").expect("the bundled slot codes font parses")));
    &FONT_PARSED
}

/// Which font a [`Fitted`] is drawn in, each a font of the game's own
/// glyphs: the numbers' (Nettai Digits), the banners' numbers' (Nettai
/// Banner Digits), the chip window's codes' (Nettai Chip Codes) or the
/// slots' (Nettai Slot Codes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Digits,
    BannerDigits,
    ChipCodes,
    SlotCodes,
}

impl Face {
    pub fn font(self) -> &'static std::sync::Arc<crate::vfont::VectorFont> {
        match self {
            Face::Digits => digits_font(),
            Face::BannerDigits => banner_digits_font(),
            Face::ChipCodes => chip_codes_font(),
            Face::SlotCodes => slot_codes_font(),
        }
    }

    /// The characters its metrics are read off (its digits, or its codes).
    fn chars(self) -> &'static str {
        match self {
            Face::Digits | Face::BannerDigits => "0123456789",
            Face::ChipCodes | Face::SlotCodes => CODES,
        }
    }

    /// Whether its glyphs stand where the game's stand down their face box
    /// (a code font: its face's top to the baseline the box's height, '*'
    /// at its own rows), rather than each centered down it (the digits, all
    /// one height but the signs).
    pub fn rows_as_drawn(self) -> bool {
        matches!(self, Face::ChipCodes | Face::SlotCodes)
    }

    /// Its game metrics.
    pub fn game(self) -> GameMetrics {
        static DIGITS: std::sync::LazyLock<GameMetrics> = std::sync::LazyLock::new(|| GameMetrics::of(Face::Digits));
        static BANNER: std::sync::LazyLock<GameMetrics> = std::sync::LazyLock::new(|| GameMetrics::of(Face::BannerDigits));
        static CHIP: std::sync::LazyLock<GameMetrics> = std::sync::LazyLock::new(|| GameMetrics::of(Face::ChipCodes));
        static SLOT: std::sync::LazyLock<GameMetrics> = std::sync::LazyLock::new(|| GameMetrics::of(Face::SlotCodes));
        match self {
            Face::Digits => *DIGITS,
            Face::BannerDigits => *BANNER,
            Face::ChipCodes => *CHIP,
            Face::SlotCodes => *SLOT,
        }
    }
}

/// A font's metrics as the game's: frame pixels a font unit across (its
/// glyphs' advance one game cell), and its glyphs' faces' insets in their
/// cell (frame pixels: from the cell's left to the faces' left, from the
/// faces' right to the cell's right), read off the font.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameMetrics {
    pub across: f32,
    pub inset: (f32, f32),
}

impl GameMetrics {
    fn of(face: Face) -> GameMetrics {
        let f = face.font().font();
        let cmap = f.charmap();
        let advance = f.glyph_metrics(&[]).advance_width(cmap.map(face.chars().chars().next().unwrap_or('0')));
        let across = CELL as f32 / advance.max(1.0);
        let inks: Vec<[f32; 4]> = face.chars().chars().filter_map(|c| ink_box(face, cmap.map(c))).collect();
        let left = inks.iter().map(|b| b[0]).fold(f32::MAX, f32::min);
        let right = inks.iter().map(|b| b[2]).fold(f32::MIN, f32::max);
        GameMetrics { across, inset: (left * across, (advance - right) * across) }
    }
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

    /// The set in another digit font (the banners').
    pub fn in_font(self, font: Face) -> NumberSet {
        NumberSet { font, ..self }
    }
}

/// A game cell's width: a character a cell.
pub const CELL: i32 = 8;

/// Characters of a number (or a chip code) drawn in a font of the game's
/// glyphs ([`TextItem::fitted`]): each one's face box (x, y, width, height
/// in frame pixels from the item's box's top left), the characters of its
/// set (`set`: the glyphs are drawn a glyph pixel a frame pixel across, at
/// their places in their faces, and down at one scale, the set's ink the
/// boxes' height), and the paint.
#[derive(Clone, Debug, PartialEq)]
pub struct Fitted {
    pub glyphs: Vec<(char, [f32; 4])>,
    pub set: String,
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
    /// Its font (a digit font: Nettai Digits, or the banners').
    pub font: Face,
    /// Its characters (`Fitted::set`).
    pub set: String,
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
            font: Face::Digits,
            set: DIGITS.into(),
            fill: d.fill.iter().map(|&c| color(c)).collect(),
            rim: d.rim.map(color),
            shadow: d.shadow.map(color),
        }
    }

    /// `chars` in this set's look, a character a cell from the left.
    pub fn fitted(&self, chars: &str) -> Fitted {
        let [x, y, w, h] = self.face.map(|v| v as f32);
        let glyphs = chars.chars().enumerate().filter(|(_, c)| *c != ' ').map(|(i, c)| (c, [(CELL * i as i32) as f32 + x, y, w, h])).collect();
        Fitted { glyphs, set: self.set.clone(), fill: self.fill.clone(), rim: self.rim, shadow: self.shadow, face: self.font, ids: Vec::new() }
    }
}

/// A glyph of a digit font's ink box, in font units: left, bottom, right,
/// top (none: no ink).
fn ink_box(face: Face, glyph: u16) -> Option<[f32; 4]> {
    let f = face.font().font();
    let mut ctx = swash::scale::ScaleContext::new();
    let mut scaler = ctx.builder(f).size(f.metrics(&[]).units_per_em as f32).build();
    scaler.scale_outline(glyph).map(|o| {
        let b = o.bounds();
        [b.min.x, b.min.y, b.max.x, b.max.y]
    })
}

/// `text` shaped in a digit font, in font units: each glyph's character,
/// glyph, offset and advance.
fn shaped(face: Face, text: &str) -> Vec<(char, u16, f32, f32)> {
    let f = face.font().font();
    let upem = f.metrics(&[]).units_per_em as f32;
    let mut ctx = swash::shape::ShapeContext::new();
    let mut shaper = ctx.builder(f).size(upem).build();
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

/// Where a run of numbers is placed: its first face's left edge, its last
/// face's right edge or its faces' middle at this x (frame pixels: where the
/// picture's are, [`picture_span`]), or its first glyph's ink's left edge
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

/// Numbers and signs in their sets' looks set as one run of their digit
/// font on cells' rows `y`, with the font's own metrics (every glyph its
/// advance, a game cell), each glyph a glyph pixel a frame pixel across and
/// its part's set's face box tall, never squeezed; placed by `anchor`. An
/// item for each part (each glyph's box its face's: where the font's 6-pixel
/// face is in its cell), and where the run's ink starts and ends. None where
/// the text layer draws no numbers (the original mode), or a font lacks a
/// character.
pub fn laid(text: &TextSink, parts: &[(String, &NumberSet)], y: i32, anchor: Anchor) -> Option<(Vec<TextItem>, (f32, f32))> {
    if !text.draws_numbers() || parts.iter().any(|(chars, set)| !set.font.font().covers(chars)) {
        return None;
    }
    // (Each part shaped in its set's font, one after another.)
    struct Glyph {
        part: usize,
        c: char,
        glyph: u16,
        // (From the run's start, frame pixels: its face's left, and its ink's
        // left and width.)
        face: f32,
        ink: Option<(f32, f32)>,
    }
    let mut glyphs: Vec<Glyph> = Vec::new();
    let mut pen = 0.0;
    for (k, (chars, set)) in parts.iter().enumerate() {
        let metrics = set.font.game();
        let u = metrics.across;
        for (c, glyph, dx, a) in shaped(set.font, &chars.replace(' ', "")) {
            let origin = pen + dx * u;
            let ink = ink_box(set.font, glyph).map(|[x0, _, x1, _]| (origin + x0 * u, (x1 - x0) * u));
            glyphs.push(Glyph { part: k, c, glyph, face: origin + metrics.inset.0, ink });
            pen += a * u;
        }
    }
    let first = parts.first()?.1.font.game();
    let last = parts.last()?.1.font.game();
    let first_ink = glyphs.iter().find_map(|g| g.ink).map_or(0.0, |i| i.0);
    let (faces_left, faces_right) = (first.inset.0, pen - last.inset.1);
    let start = match anchor {
        Anchor::Left(x) => x - faces_left,
        Anchor::Right(x) => x - faces_right,
        Anchor::Center(x) => x - (faces_left + faces_right) / 2.0,
        Anchor::LeftInk(x) => x - first_ink,
    };
    let mut boxes: Vec<(Vec<(char, [f32; 4])>, Vec<u16>)> = vec![(Vec::new(), Vec::new()); parts.len()];
    let (mut ink_start, mut ink_end) = (f32::MAX, f32::MIN);
    for g in &glyphs {
        let Some((x, w)) = g.ink else { continue };
        let set = parts[g.part].1;
        let [_, top, face_w, h] = set.face.map(|v| v as f32);
        boxes[g.part].0.push((g.c, [start + g.face, top, face_w, h]));
        boxes[g.part].1.push(g.glyph);
        ink_start = ink_start.min(start + x);
        ink_end = ink_end.max(start + x + w);
    }
    let mut out = Vec::new();
    for ((_, set), (glyphs, ids)) in parts.iter().zip(boxes) {
        let Some(left) = glyphs.iter().map(|g| g.1[0]).reduce(f32::min).map(f32::floor) else { continue };
        let right = glyphs.iter().map(|g| g.1[0] + g.1[2]).fold(left, f32::max);
        let glyphs = glyphs.into_iter().map(|(c, [x, top, w, h])| (c, [x - left, top, w, h])).collect();
        let rect = Rect::new(left as i32, y, (right - left).ceil() as i32 + 1, 16);
        let f = Fitted { glyphs, set: set.set.clone(), fill: set.fill.clone(), rim: set.rim, shadow: set.shadow, face: set.font, ids };
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

    /// The digit fonts keep the game's metrics: every digit one advance, a
    /// game cell, and its face 6 frame pixels wide a pixel in from the
    /// cell's left.
    #[test]
    fn digits_are_a_cell_apart() {
        for face in [Face::Digits, Face::BannerDigits] {
            let shaped = shaped(face, "0123456789");
            let m = face.game();
            let cells: Vec<f32> = shaped.iter().map(|g| g.3 * m.across).collect();
            assert_eq!(cells, vec![CELL as f32; 10], "{face:?}");
            assert_eq!(m.inset, (1.0, 1.0), "{face:?}");
        }
    }

    /// The code fonts keep the game's metrics too: every code one advance, a
    /// game cell; the chip window's 7 frame pixels of face, half a pixel in,
    /// 11 tall; the slots' 6, a pixel in, 5 tall; each on the baseline, '*'
    /// within its letters' rows.
    #[test]
    fn codes_are_a_cell_apart() {
        for (face, inset, tall) in [(Face::ChipCodes, (0.5, 0.5), 11.0), (Face::SlotCodes, (1.0, 1.0), 5.0)] {
            let shaped = shaped(face, CODES);
            let m = face.game();
            let cells: Vec<f32> = shaped.iter().map(|g| g.3 * m.across).collect();
            assert_eq!(cells, vec![CELL as f32; CODES.len()], "{face:?}");
            assert_eq!(m.inset, inset, "{face:?}");
            let cmap = face.font().font().charmap();
            let rows = |c: char| ink_box(face, cmap.map(c)).map(|[_, y0, _, y1]| (y0 * m.across, y1 * m.across)).unwrap();
            assert_eq!(rows('A'), (0.0, tall), "{face:?}");
            let star = rows('*');
            assert!(star.0 >= 0.0 && star.1 <= tall, "{face:?}: {star:?}");
        }
    }

    /// A number laid where the picture's is lands on its faces: the HP box's
    /// four digits' faces from a cell's second column, a cell apart.
    #[test]
    fn a_laid_numbers_faces_are_the_pictures() {
        let palette: Palette = std::array::from_fn(|i| i as u16 * 0x421);
        let set = NumberSet::of(&Digits { face: [1, 3, 6, 10], fill: vec![1; 10], rim: None, shadow: None }, &palette);
        let text = TextSink::new(crate::textlayer::GraphicsMode::Enhanced, Some(font().as_ref()));
        let (a, b) = picture_span(&set, 16, 4);
        let (items, _) = laid(&text, &[("1234".to_string(), &set)], 0, Anchor::Right(b)).unwrap();
        let f = items[0].fitted.as_ref().unwrap();
        let x0 = items[0].rect.x as f32;
        let lefts: Vec<f32> = f.glyphs.iter().map(|g| x0 + g.1[0]).collect();
        assert_eq!(lefts, vec![a, a + 8.0, a + 16.0, a + 24.0]);
        assert!(f.glyphs.iter().all(|g| g.1[2] == 6.0 && g.1[3] == 10.0));
    }

    #[test]
    fn the_fonts_have_the_numbers_and_signs() {
        assert!(font().covers("0123456789+?"));
        assert!(digits_font().covers("0123456789+?"));
        assert!(digits_font().covers(times()));
        assert!(banner_digits_font().covers("0123456789"));
    }
}
