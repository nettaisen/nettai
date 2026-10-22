//! The battle's frames drawn as shapes in the enhanced mode
//! (docs/design/text-rendering.md §13): the pack's drawings
//! (`nettai_assets::shapes`, drawn by hand and checked as the pack is made)
//! put in the palette the picture they stand in for is drawn in at the
//! moment, flipped as its tiles are, as a text layer item over the picture's
//! place. Nothing here is a game's: a frame with no drawing is drawn as its
//! picture.

use crate::textlayer::{Drawn, Piece, Rect, TextItem};
use nettai_assets::shapes::Drawing;

/// `d` in the colors `color` gives its palette entries (row, index),
/// flipped within its box as `hflip` and `vflip` say, cut to the part
/// `from`..`to` of its width (its whole width: `None`).
pub fn drawn(d: &Drawing, color: impl Fn(u8, u8) -> u16, (hflip, vflip): (bool, bool), cut: Option<(f32, f32)>) -> Drawn {
    let piece = Piece {
        drawing: Drawing { words: Vec::new(), ..d.clone() },
        colors: d.colors.iter().map(|&(p, c)| color(p, c)).collect(),
        flip: (hflip, vflip),
        at: (0.0, 0.0),
        cut,
    };
    let pieces = vec![piece];
    Drawn { hash: hash_of(&pieces), pieces, cells: None, off: None }
}

/// Which of `d`'s frame pixels (row by row, its size) its shapes cover at
/// their centers: where a picture standing in for it must still be (for
/// what is drawn in front of or behind it), and nowhere else. Made once a
/// drawing (the chatbox's as its pack's graphics are set).
pub fn coverage(d: &Drawing) -> std::sync::Arc<Vec<bool>> {
    use std::sync::{Arc, Mutex, OnceLock};
    static MADE: OnceLock<Mutex<std::collections::HashMap<u64, Arc<Vec<bool>>>>> = OnceLock::new();
    let made = MADE.get_or_init(Default::default);
    if let Some(c) = nettai_platform::thread::lock(made).ok().and_then(|m| m.get(&d.hash).cloned()) {
        return c;
    }
    let c = Arc::new(nettai_content::shapes::coverage(d));
    if let Ok(mut m) = nettai_platform::thread::lock(made) {
        m.insert(d.hash, c.clone());
    }
    c
}

/// Several drawings as one (each moved by its offset in frame pixels),
/// drawn in order.
pub fn joined(parts: Vec<(Drawn, (f32, f32))>) -> Drawn {
    let pieces: Vec<Piece> =
        parts.into_iter().flat_map(|(d, (dx, dy))| d.pieces.into_iter().map(move |p| Piece { at: (p.at.0 + dx, p.at.1 + dy), ..p })).collect();
    Drawn { hash: hash_of(&pieces), pieces, cells: None, off: None }
}

/// `d` mirrored in its item's box `width` frame pixels wide: each piece
/// flipped at its mirrored place (its cut the mirror of its columns), the
/// cells it draws in and the pixels it keeps off mirrored.
pub fn mirrored(d: &Drawn, width: i32) -> Drawn {
    let pieces: Vec<Piece> = d
        .pieces
        .iter()
        .map(|p| {
            let pw = p.drawing.size[0] as f32;
            Piece {
                flip: (!p.flip.0, p.flip.1),
                at: (width as f32 - p.at.0 - pw, p.at.1),
                cut: p.cut.map(|(from, to)| (pw - to, pw - from)),
                ..p.clone()
            }
        })
        .collect();
    let columns = (width.max(0) as usize).div_ceil(8);
    let cells = d.cells.as_ref().map(|cells| cells.iter().map(|&[c, r]| [(columns - 1 - (c as usize).min(columns - 1)) as u8, r]).collect());
    let off = d.off.as_ref().map(|(w, flags)| {
        let to = width.max(0) as usize;
        let rows = flags.len() / (*w).max(1);
        let mut out = vec![false; to * rows];
        for y in 0..rows {
            for x in 0..(*w).min(to) {
                out[y * to + to - 1 - x] = flags[y * w + x];
            }
        }
        (to, out)
    });
    let mut out = Drawn { hash: 0, pieces, cells, off };
    out.hash = rehash(&out);
    out
}

/// A drawing's hash again, after its pieces changed (`Drawn::hash`: of its
/// pieces, as `drawn` and `joined` make it).
fn rehash(d: &Drawn) -> u64 {
    hash_of(&d.pieces)
}

fn hash_of(pieces: &[Piece]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for p in pieces {
        p.drawing.hash.hash(&mut h);
        p.colors.hash(&mut h);
        p.flip.hash(&mut h);
        for v in [p.at.0, p.at.1] {
            v.to_bits().hash(&mut h);
        }
        p.cut.map(|(a, b)| (a.to_bits(), b.to_bits())).hash(&mut h);
    }
    h.finish()
}

/// The words written on `d` (`Drawing::words`) as lettering items in the
/// numbers' font, from the drawing's top left: each the game's string for
/// its key, its capitals filling its box's height (its lines' share of it,
/// one under another), each run fitted to the box's width and centered in it, in the colors `color` gives (its palette
/// row, its index); none for words the strings or the font lack (noted).
pub fn words(
    d: &Drawing,
    color: impl Fn(u8, u8) -> u16,
    text: &crate::textlayer::TextSink,
    content: &nettai_battle::Content,
    problems: &mut crate::audit::Problems,
) -> Vec<TextItem> {
    use crate::vfont::{Lettering, LetteringFont, Role};
    let u = Drawing::UNIT;
    let mut out = Vec::new();
    for t in &d.words {
        let what = || format!("the words {:?} on a drawing", t.key);
        let Some(words) = text.strings.lettering(content, &t.key) else {
            problems.note(format!("{}: the strings have none: not drawn", what()));
            continue;
        };
        // (The numbers' font where it has every letter and the drawing
        // doesn't ask for the lettering font, else that: kana, mixed case.)
        let font = if !t.lettering && text.takes_lettering_in(words, LetteringFont::Numbers) { LetteringFont::Numbers } else { LetteringFont::Display };
        if !text.lettering_in(words, font, what, problems) {
            continue;
        }
        let [x, y, w, h] = t.rect.map(|v| v as f32 / u);
        // (Lines one under another, the box's height shared, a pixel apart.)
        let lines: Vec<&str> = words.lines().collect();
        let n = lines.len().max(1) as f32;
        let gap = if n > 1.0 { 1.0 } else { 0.0 };
        let face = color(t.palette, t.color);
        let shadow = t.shadow.map(|c| color(t.palette, c));
        for (k, line) in lines.iter().enumerate() {
            let line_top = y + k as f32 * (h + gap) / n;
            let (top, cap) = (line_top.round() as i32, ((h + gap) / n - gap).round().clamp(1.0, 255.0) as i32);
            let natural = Lettering { font, ..Lettering::wide(0, cap as u8) };
            let Some((ink0, ink1)) = text.ink_extent(line, Role::Lettering(natural), 10_000) else { continue };
            let width = (100.0 * w / (ink1 - ink0).max(1.0)).round().clamp(25.0, 255.0) as u8;
            let lettering = Lettering { width, ..natural };
            let rect = Rect::new(x.floor() as i32 - 2, top, w.ceil() as i32 + 4, cap);
            let item = TextItem::new(*line, Role::Lettering(lettering), rect, face, shadow);
            let ink = crate::textlayer::Ink { fill: vec![face], rim: t.rim.map(|c| color(t.palette, c)), rim_width: t.rim_width as f32 / 100.0, stepped: false };
            out.push(TextItem { align: crate::textlayer::Align::Center, ink: Some(ink), slant: t.slant, ..item });
        }
    }
    out
}

/// A text layer item of `d` with its box's top left at (x, y) in frame
/// pixels: its box the drawing's, nothing drawn past it.
pub fn item(d: Drawn, (x, y): (i32, i32), (w, h): (i32, i32)) -> TextItem {
    let rect = Rect::new(x, y, w, h);
    TextItem { drawn: Some(d), clip: rect, ..TextItem::new("", crate::vfont::Role::Cell, rect, 0, None) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(color: u8) -> Drawing {
        nettai_content::shapes::from_svg(&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 8 8\"><rect width=\"4\" height=\"8\" data-color=\"{color}\"/></svg>")).unwrap()
    }

    /// A drawing in its palette, flipped, cut, and joined with another moved.
    #[test]
    fn a_drawing_flipped_cut_and_joined() {
        let d = drawn(&square(3), |_, c| c as u16 * 10, (true, false), Some((0.0, 2.5)));
        assert_eq!((d.pieces[0].colors.as_slice(), d.pieces[0].flip, d.pieces[0].cut), ([30u16].as_slice(), (true, false), Some((0.0, 2.5))));
        let j = joined(vec![(d.clone(), (0.0, 0.0)), (drawn(&square(4), |_, c| c as u16, (false, false), Some((1.0, 2.0))), (8.0, 0.0))]);
        assert_eq!(j.pieces.len(), 2);
        assert_eq!((j.pieces[1].at, j.pieces[1].cut), ((8.0, 0.0), Some((1.0, 2.0))));
        assert_ne!(j.hash, d.hash);
    }
}
