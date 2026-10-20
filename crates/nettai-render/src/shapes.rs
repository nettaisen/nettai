//! The battle's frames drawn as shapes in the enhanced mode
//! (docs/design/text-rendering.md §13): the pack's drawings
//! (`nettai_assets::shapes`, drawn by hand and checked as the pack is made)
//! put in the palette the picture they stand in for is drawn in at the
//! moment, flipped as its tiles are, as a text layer item over the picture's
//! place. Nothing here is a game's: a frame with no drawing is drawn as its
//! picture.

use crate::textlayer::{Drawn, Rect, TextItem};
use nettai_assets::shapes::{Drawing, Region};

/// `d` in the colors `color` gives each region (its palette row, its
/// index), flipped within its box as `hflip` and `vflip` say, cut to the part
/// of a pixel `from`..`to` of its width (its whole width: `None`).
pub fn drawn(d: &Drawing, color: impl Fn(&Region) -> u16, (hflip, vflip): (bool, bool), cut: Option<(f32, f32)>) -> Drawn {
    let (w, h) = (d.size[0] as f32, d.size[1] as f32);
    let regions: Vec<(Vec<Vec<[f32; 2]>>, u16)> = d
        .regions
        .iter()
        .map(|r| {
            let rings: Vec<Vec<[f32; 2]>> = r
                .rings()
                .into_iter()
                .map(|ring| ring.into_iter().map(|[x, y]| [if hflip { w - x } else { x }, if vflip { h - y } else { y }]).collect::<Vec<_>>())
                .map(|ring| match cut {
                    Some((a, b)) => clip_x(&clip_x(&ring, a, true), b, false),
                    None => ring,
                })
                .filter(|ring| ring.len() >= 3)
                .collect();
            (rings, color(r))
        })
        .filter(|(rings, _)| !rings.is_empty())
        .collect();
    let hash = hash_of(&regions);
    Drawn { regions, cells: None, off: None, hash }
}

/// Which of `d`'s frame pixels (row by row, its size) some region of it
/// covers at their centers: where a picture standing in for it must still
/// be (for what is drawn in front of or behind it), and nowhere else.
pub fn coverage(d: &Drawing) -> Vec<bool> {
    let (w, h) = (d.size[0] as usize, d.size[1] as usize);
    let rings: Vec<Vec<Vec<[f32; 2]>>> = d.regions.iter().map(|r| r.rings()).collect();
    (0..w * h).map(|i| rings.iter().any(|r| nettai_assets::shapes::inside(r, [(i % w) as f32 + 0.5, (i / w) as f32 + 0.5]))).collect()
}

/// Several drawings as one (each moved by its offset in frame pixels),
/// filled in order.
pub fn joined(parts: Vec<(Drawn, (f32, f32))>) -> Drawn {
    let regions: Vec<(Vec<Vec<[f32; 2]>>, u16)> = parts
        .into_iter()
        .flat_map(|(d, (dx, dy))| d.regions.into_iter().map(move |(rings, c)| (rings.into_iter().map(|r| r.into_iter().map(|[x, y]| [x + dx, y + dy]).collect()).collect(), c)))
        .collect();
    let hash = hash_of(&regions);
    Drawn { regions, cells: None, off: None, hash }
}

fn hash_of(regions: &[(Vec<Vec<[f32; 2]>>, u16)]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for (rings, c) in regions {
        c.hash(&mut h);
        for r in rings {
            r.len().hash(&mut h);
            for p in r {
                p[0].to_bits().hash(&mut h);
                p[1].to_bits().hash(&mut h);
            }
        }
    }
    h.finish()
}

/// A ring cut at column `x`, keeping what is right of it (`right`) or left.
fn clip_x(ring: &[[f32; 2]], x: f32, right: bool) -> Vec<[f32; 2]> {
    let keep = |p: &[f32; 2]| if right { p[0] >= x } else { p[0] <= x };
    let mut out = Vec::new();
    for k in 0..ring.len() {
        let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
        if keep(&a) {
            out.push(a);
        }
        if keep(&a) != keep(&b) {
            let t = (x - a[0]) / (b[0] - a[0]);
            out.push([x, a[1] + t * (b[1] - a[1])]);
        }
    }
    out
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
            let ink = crate::textlayer::Ink { fill: vec![face], rim: t.rim.map(|c| color(t.palette, c)) };
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
    use nettai_assets::shapes::Step;

    fn square(color: u8) -> Drawing {
        let u = Drawing::UNIT as i32;
        Drawing {
            size: [8, 8],
            regions: vec![Region { palette: 0, color, path: vec![Step::Move([0, 0]), Step::Line([4 * u, 0]), Step::Line([4 * u, 8 * u]), Step::Line([0, 8 * u]), Step::Close] }],
            words: Vec::new(),
            source: String::new(),
        }
    }

    /// A drawing in its palette, flipped across its box, cut to a part of its
    /// width.
    #[test]
    fn a_drawing_flipped_and_cut() {
        let d = drawn(&square(3), |r| r.color as u16 * 10, (true, false), None);
        assert_eq!(d.regions[0].1, 30);
        assert!(d.regions[0].0[0].iter().all(|p| p[0] >= 4.0), "{:?}", d.regions);
        let cut = drawn(&square(3), |_| 0, (false, false), Some((0.0, 2.5)));
        assert!(cut.regions[0].0[0].iter().all(|p| p[0] <= 2.5), "{:?}", cut.regions);
        let gone = drawn(&square(3), |_| 0, (false, false), Some((5.0, 8.0)));
        assert!(gone.regions.is_empty());
    }
}
