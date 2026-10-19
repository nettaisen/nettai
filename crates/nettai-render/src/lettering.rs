//! The game-style lettering of the battle's pictures, read off the pictures
//! themselves, so that the enhanced mode can draw the words in the
//! lettering font in their place (docs/design/text-rendering.md §12).
//!
//! A picture with words (a banner, "PAUSE", the waiting label, the damage
//! digits) says how its letters look: where their capitals stand, the
//! colors of their face down the rows, and an outline around it, a rim
//! all round or a shadow right and down. [`letters`] reads that off the
//! picture's palette indices, given what is behind the letters (the
//! picture's ground); [`bar`] reads a banner's bar. Nothing here is a
//! game's: every look is the pack's pictures', so a game whose pictures
//! look otherwise is drawn otherwise. The words themselves are the
//! content's strings (`strings::DisplayText::lettering`).

use crate::textlayer::{Bar, BarColors, Ink};
use crate::vfont::Lettering;
use nettai_assets::{Palette, Tiles};

/// A picture's palette indices, `w` by `h`, row by row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Picture {
    pub w: usize,
    pub h: usize,
    pub pixels: Vec<u8>,
}

impl Picture {
    /// `cells` glyphs of 8x16 side by side from glyph `first` (glyph k is
    /// tiles 2k, its top, and 2k + 1).
    pub fn glyphs(tiles: &Tiles, first: usize, cells: usize) -> Picture {
        let (w, h) = (8 * cells, 16);
        let mut pixels = vec![0; w * h];
        for c in 0..cells {
            for half in 0..2 {
                let Some(t) = tiles.get(2 * (first + c) + half) else { continue };
                for (i, &v) in t.iter().enumerate() {
                    pixels[(8 * half + i / 8) * w + 8 * c + i % 8] = v;
                }
            }
        }
        Picture { w, h, pixels }
    }

    /// Tiles laid `columns` a row, `rows` rows, from tile `first`.
    pub fn grid(tiles: &Tiles, first: usize, columns: usize, rows: usize) -> Picture {
        let (w, h) = (8 * columns, 8 * rows);
        let mut pixels = vec![0; w * h];
        for k in 0..columns * rows {
            let Some(t) = tiles.get(first + k) else { continue };
            for (i, &v) in t.iter().enumerate() {
                pixels[(8 * (k / columns) + i / 8) * w + 8 * (k % columns) + i % 8] = v;
            }
        }
        Picture { w, h, pixels }
    }

    pub fn at(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return None;
        }
        Some(self.pixels[y as usize * self.w + x as usize])
    }

    /// Its part from (x, y), `w` by `h`.
    pub fn crop(&self, x: usize, y: usize, w: usize, h: usize) -> Picture {
        let mut pixels = Vec::with_capacity(w * h);
        for row in y..y + h {
            for col in x..x + w {
                pixels.push(self.at(col as i32, row as i32).unwrap_or(0));
            }
        }
        Picture { w, h, pixels }
    }

    /// The tiles it is made of again, laid as [`Picture::grid`] reads them.
    pub fn to_grid(&self) -> Tiles {
        let (columns, rows) = (self.w / 8, self.h / 8);
        let mut tiles = Tiles::default();
        for k in 0..columns * rows {
            let tile: Vec<u8> = (0..64).map(|i| self.pixels[(8 * (k / columns) + i / 8) * self.w + 8 * (k % columns) + i % 8]).collect();
            tiles.push(&tile);
        }
        tiles
    }
}

/// How a picture's letters look: palette indices, rows and columns of the
/// picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Letters {
    /// The face's color on each of the capitals' rows, top to bottom.
    pub fill: Vec<u8>,
    /// The outline's color, all round the face (a rim) or right and down
    /// of it (a shadow).
    pub rim: Option<u8>,
    pub shadow: Option<u8>,
    /// The capitals' first row, and how many rows they take.
    pub top: u8,
    pub cap: u8,
    /// The letters' columns, their outline included: the first, and one
    /// past the last.
    pub span: (i32, i32),
}

/// Where a picture's capitals are read from ([`letters`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capitals {
    /// Its first letter's (a word's: its lower case letters and its
    /// punctuation stand otherwise).
    First,
    /// All its letters' (a set of digits).
    All,
}

/// How the letters of a picture look, `ground` saying which indices are
/// behind them (its transparent pixels, a bar's colors); none if it has
/// no letters.
pub fn letters(p: &Picture, ground: impl Fn(u8) -> bool, capitals: Capitals) -> Option<Letters> {
    let ink = |x: i32, y: i32| p.at(x, y).is_some_and(|v| !ground(v));
    let near = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    // The outline is the color most of the letters' edge (their pixels
    // next to the ground) is.
    let mut edge = [0usize; 16];
    let mut total = 0;
    for y in 0..p.h as i32 {
        for x in 0..p.w as i32 {
            if !ink(x, y) {
                continue;
            }
            total += 1;
            if near.iter().any(|&(dx, dy)| !ink(x + dx, y + dy)) {
                edge[p.at(x, y).unwrap_or(0) as usize & 15] += 1;
            }
        }
    }
    if total == 0 {
        return None;
    }
    // The outline: of the colors of the letters' edge (their pixels next
    // to the ground), most first, the one the rest (the face) is kept from
    // the ground by: all round (a rim), or only right and down (a shadow).
    let is = |x: i32, y: i32, c: u8| ink(x, y) && p.at(x, y) == Some(c);
    let outline = |c: u8| {
        let (mut face, mut face_edge, mut before, mut after) = (0usize, 0usize, 0usize, 0usize);
        for y in 0..p.h as i32 {
            for x in 0..p.w as i32 {
                if !ink(x, y) || is(x, y, c) {
                    continue;
                }
                face += 1;
                if near.iter().any(|&(dx, dy)| !ink(x + dx, y + dy)) {
                    face_edge += 1;
                }
                before += is(x - 1, y, c) as usize + is(x, y - 1, c) as usize;
                after += is(x + 1, y, c) as usize + is(x, y + 1, c) as usize;
            }
        }
        let enough = face * 5 >= total;
        if enough && face_edge * 4 <= face {
            Some((Some(c), None))
        } else if enough && after > 0 && 2 * after > 3 * before {
            Some((None, Some(c)))
        } else {
            None
        }
    };
    let mut colors: Vec<u8> = (0..16u8).filter(|&c| edge[c as usize] > 0).collect();
    colors.sort_by_key(|&c| std::cmp::Reverse((edge[c as usize], c)));
    let (rim, shadow) = colors.iter().take(2).find_map(|&c| outline(c)).unwrap_or((None, None));
    let line = rim.or(shadow);
    let is_face = |x: i32, y: i32| ink(x, y) && (line.is_none() || p.at(x, y) != line);
    // The capitals' rows.
    let columns: Vec<i32> = (0..p.w as i32).filter(|&x| (0..p.h as i32).any(|y| is_face(x, y))).collect();
    let first = *columns.first()?;
    let letter: Vec<i32> = match capitals {
        Capitals::All => columns.clone(),
        Capitals::First => {
            let end = columns.windows(2).find(|w| w[1] != w[0] + 1).map_or(*columns.last()?, |w| w[0]);
            (first..=end).collect()
        }
    };
    let rows: Vec<i32> = (0..p.h as i32).filter(|&y| letter.iter().any(|&x| is_face(x, y))).collect();
    let (top, bottom) = (*rows.first()?, *rows.last()?);
    // Each row's face color: the one most of the face's pixels on it are.
    let row_color = |y: i32| {
        let mut count = [0usize; 16];
        for x in 0..p.w as i32 {
            if is_face(x, y) {
                count[p.at(x, y).unwrap_or(0) as usize & 15] += 1;
            }
        }
        (0..16).filter(|&c| count[c] > 0).max_by_key(|&c| (count[c], c)).map(|c| c as u8)
    };
    let mut fill: Vec<Option<u8>> = (top..=bottom).map(row_color).collect();
    // (A row of the capitals with no face of its own takes its neighbor's.)
    for i in 1..fill.len() {
        if fill[i].is_none() {
            fill[i] = fill[i - 1];
        }
    }
    for i in (0..fill.len().saturating_sub(1)).rev() {
        if fill[i].is_none() {
            fill[i] = fill[i + 1];
        }
    }
    let fill: Vec<u8> = fill.into_iter().map(|c| c.unwrap_or(1)).collect();
    let inked: Vec<i32> = (0..p.w as i32).filter(|&x| (0..p.h as i32).any(|y| ink(x, y))).collect();
    let span = (*inked.first()?, inked.last()? + 1);
    Some(Letters { fill, rim, shadow, top: top as u8, cap: (bottom - top + 1) as u8, span })
}

impl Letters {
    /// The role its words are drawn in: the lettering font with its
    /// capitals, at `lettering`'s weight and width.
    pub fn role(&self, lettering: fn(u8, u8) -> Lettering) -> crate::vfont::Role {
        crate::vfont::Role::Lettering(lettering(self.top, self.cap))
    }

    /// Its paint in `palette`.
    pub fn ink(&self, palette: &Palette) -> Ink {
        Ink { fill: self.fill.iter().map(|&c| palette[c as usize & 15]).collect(), rim: self.rim.map(|c| palette[c as usize & 15]), bold: 0.0 }
    }

    /// Its face's first color and its shadow's in `palette` (an item's
    /// `face` and `shadow`).
    pub fn face(&self, palette: &Palette) -> (u16, Option<u16>) {
        (palette[self.fill.first().copied().unwrap_or(1) as usize & 15], self.shadow.map(|c| palette[c as usize & 15]))
    }
}

/// The look a picture's letters give the text drawn in their place: its
/// role (the lettering font, the capitals where the picture's are), its
/// paint and its face's and shadow's colors, in a palette.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub role: crate::vfont::Role,
    pub ink: Ink,
    pub face: u16,
    pub shadow: Option<u16>,
    /// The picture's letters' columns, their outline included.
    pub span: (i32, i32),
}

impl Look {
    /// A picture's letters' look in `palette`, drawn at `lettering`'s
    /// weight and width (`Lettering::display`, `Lettering::wide`).
    pub fn of(p: &Picture, ground: impl Fn(u8) -> bool, capitals: Capitals, palette: &Palette, lettering: fn(u8, u8) -> Lettering) -> Option<Look> {
        let l = letters(p, ground, capitals)?;
        let (face, shadow) = l.face(palette);
        Some(Look { role: l.role(lettering), ink: l.ink(palette), face, shadow, span: l.span })
    }

    /// `words` in this look in `rect` (its rows the picture's), aligned.
    pub fn item(&self, words: impl Into<String>, rect: crate::textlayer::Rect, align: crate::textlayer::Align) -> crate::textlayer::TextItem {
        let item = crate::textlayer::TextItem::new(words, self.role, rect, self.face, self.shadow);
        crate::textlayer::TextItem { align, ink: Some(self.ink.clone()), ..item }
    }

    /// The words standing where the picture's letters stand, the picture's
    /// top left at (x, y), `h` rows high: centered on its letters.
    pub fn in_place(&self, words: impl Into<String>, (x, y): (i32, i32), h: i32) -> crate::textlayer::TextItem {
        let rect = crate::textlayer::Rect::new(x + self.span.0, y, self.span.1 - self.span.0, h);
        self.item(words, rect, crate::textlayer::Align::Center)
    }
}

/// A banner's bar ([`Bar`]), read off its picture in `palette`: the colors
/// of its first cell (8 columns), where that cell is a bar's end, every
/// row of it with pixels unbroken from them to the bar's last color on the
/// row (letters stand on the bar, so nothing between is clear); none for a
/// picture with no bar (the Japanese Program Advance's words, whose first
/// cell is a letter's). Its slant, its ends and its length are the
/// picture's: each row's first and last pixel of the bar's colors.
pub fn bar(p: &Picture, palette: &Palette) -> Option<(Bar, Vec<u8>)> {
    let mut colors: Vec<u8> = Vec::new();
    for y in 0..p.h as i32 {
        for x in 0..8 {
            if let Some(v) = p.at(x, y).filter(|&v| v != 0)
                && !colors.contains(&v)
            {
                colors.push(v);
            }
        }
    }
    if colors.is_empty() {
        return None;
    }
    let of_bar = |x: i32, y: i32| p.at(x, y).is_some_and(|v| colors.contains(&v));
    let rows: Vec<i32> = (0..p.h as i32).filter(|&y| (0..8).any(|x| of_bar(x, y))).collect();
    let (top, bottom) = (*rows.first()?, *rows.last()?);
    let mut ends = Vec::new();
    for y in top..=bottom {
        let first = (0..p.w as i32).find(|&x| of_bar(x, y))?;
        let last = (0..p.w as i32).rev().find(|&x| of_bar(x, y))?;
        if (first..=last).any(|x| p.at(x, y) == Some(0)) {
            return None;
        }
        ends.push((first, last + 1));
    }
    // The slant: how far each row's first pixel is right of the next's,
    // most often.
    let mut steps: Vec<i32> = ends.windows(2).map(|w| w[0].0 - w[1].0).collect();
    steps.sort_unstable();
    let slant = steps.get(steps.len() / 2).copied().unwrap_or(0).max(0) as f32;
    let most = |count: &[usize; 16]| (0..16).max_by_key(|&c| (count[c], c)).unwrap_or(0) as u8;
    let tally = |pixels: &mut dyn Iterator<Item = (i32, i32)>| {
        let mut count = [0usize; 16];
        for (x, y) in pixels {
            if let Some(v) = p.at(x, y).filter(|_| of_bar(x, y)) {
                count[v as usize & 15] += 1;
            }
        }
        most(&count)
    };
    let row = |y: i32| (0..p.w as i32).map(move |x| (x, y));
    let top_color = tally(&mut row(top));
    let bottom_color = tally(&mut row(bottom));
    let middle = (top + 1)..bottom;
    let body = tally(&mut middle.clone().flat_map(row));
    let left = tally(&mut middle.clone().map(|y| (ends[(y - top) as usize].0, y)));
    let right = tally(&mut middle.clone().map(|y| (ends[(y - top) as usize].1 - 1, y)));
    let color = |c: u8| palette[c as usize & 15];
    let (first, last) = (ends[0], ends[ends.len() - 1]);
    let bar = Bar {
        top: top as f32,
        bottom: (bottom + 1) as f32,
        left: last.0 as f32 - slant / 2.0,
        left_most: ends.iter().map(|e| e.0).max()? as f32,
        right: first.1 as f32 + slant / 2.0,
        right_least: ends.iter().map(|e| e.1).min()? as f32,
        slant,
        line: 1.0,
        colors: BarColors { top: color(top_color), body: color(body), bottom: color(bottom_color), left: color(left), right: color(right) },
    };
    Some((bar, colors))
}

/// A line of a picture's lettering ([`lines`]): its first row, the row
/// under its letters' baseline, its columns (the first, one past the last)
/// and its color (BGR555).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    pub top: i32,
    pub baseline: i32,
    pub span: (i32, i32),
    pub color: u16,
}

/// The lines of a picture's lettering, top to bottom: runs of rows with
/// letters on them (pixels not `ground`), a clear row apart. Each line's
/// baseline is under the last row most of its letters (runs of columns)
/// reach, so a descender doesn't move it; its color is the one most of its
/// pixels are, in `palette`.
pub fn lines(p: &Picture, ground: impl Fn(u8) -> bool, palette: &Palette) -> Vec<Line> {
    let ink = |x: i32, y: i32| p.at(x, y).is_some_and(|v| !ground(v));
    let inked: Vec<bool> = (0..p.h as i32).map(|y| (0..p.w as i32).any(|x| ink(x, y))).collect();
    let mut out = Vec::new();
    let mut y = 0;
    while y < p.h {
        if !inked[y] {
            y += 1;
            continue;
        }
        let top = y;
        while y < p.h && inked[y] {
            y += 1;
        }
        let rows = top as i32..y as i32;
        let columns: Vec<i32> = (0..p.w as i32).filter(|&x| rows.clone().any(|r| ink(x, r))).collect();
        let (Some(&first), Some(&last)) = (columns.first(), columns.last()) else { continue };
        // Each letter's last row, and the most common of them.
        let mut bottoms: Vec<i32> = Vec::new();
        let mut start = 0;
        for i in 1..=columns.len() {
            if i == columns.len() || columns[i] != columns[i - 1] + 1 {
                let letter = &columns[start..i];
                bottoms.extend(rows.clone().rev().find(|&r| letter.iter().any(|&x| ink(x, r))));
                start = i;
            }
        }
        let mut seen: Vec<(i32, usize)> = Vec::new();
        for b in bottoms {
            match seen.iter_mut().find(|s| s.0 == b) {
                Some(s) => s.1 += 1,
                None => seen.push((b, 1)),
            }
        }
        let baseline = seen.iter().max_by_key(|s| (s.1, s.0)).map_or(y as i32, |s| s.0 + 1);
        let mut colors: Vec<(u16, usize)> = Vec::new();
        for r in rows.clone() {
            for x in first..=last {
                if let Some(v) = p.at(x, r).filter(|&v| !ground(v)) {
                    let c = palette[v as usize & 15] & 0x7FFF;
                    match colors.iter_mut().find(|s| s.0 == c) {
                        Some(s) => s.1 += 1,
                        None => colors.push((c, 1)),
                    }
                }
            }
        }
        let color = colors.iter().max_by_key(|s| (s.1, s.0)).map_or(0, |s| s.0);
        out.push(Line { top: top as i32, baseline, span: (first, last + 1), color });
    }
    out
}

/// A picture with its letters taken off: each of their pixels the color
/// of the ground nearest it on its row (the left's where both are as
/// near), as the original fills a blank cell with its window's colors. A
/// row with no ground keeps its pixels.
pub fn erased(p: &Picture, ground: impl Fn(u8) -> bool) -> Picture {
    let mut out = p.clone();
    for y in 0..p.h {
        let row = &p.pixels[y * p.w..(y + 1) * p.w];
        for x in 0..p.w {
            if ground(row[x]) {
                continue;
            }
            let left = (0..x).rev().find(|&i| ground(row[i])).map(|i| (x - i, row[i]));
            let right = (x + 1..p.w).find(|&i| ground(row[i])).map(|i| (i - x, row[i]));
            let near = match (left, right) {
                (Some(l), Some(r)) => Some(if r.0 < l.0 { r.1 } else { l.1 }),
                (l, r) => l.or(r).map(|v| v.1),
            };
            if let Some(v) = near {
                out.pixels[y * p.w + x] = v;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(rows: &[&str]) -> Picture {
        let w = rows[0].len();
        let pixels = rows.iter().flat_map(|r| r.chars().map(|c| if c == '.' { 0 } else { c.to_digit(16).unwrap() as u8 })).collect();
        Picture { w, h: rows.len(), pixels }
    }

    /// A banner's letters (the ROM's B and A: a cream face paler in the
    /// middle, a dark rim) on its bar (a light top line, a blue body, a navy
    /// bottom line, ends slanting a pixel a row).
    #[test]
    fn a_banners_letters_and_bar_read_off_its_picture() {
        let p = picture(&[
            "................................",
            "........66666666................",
            "........68888866................",
            "...333336888888633333333333.....",
            "...444446776677644444444445.....",
            "..344444611661164444444445......",
            ".3444444611111164444444445......",
            "35555555688888865555555555......",
            "........66666666................",
            "................................",
        ]);
        let palette: Palette = std::array::from_fn(|i| i as u16 * 0x421);
        let (bar, colors) = bar(&p, &palette).expect("a bar");
        assert_eq!(colors.len(), 3, "{colors:?}");
        assert_eq!((bar.top, bar.bottom, bar.slant), (3.0, 8.0, 1.0));
        assert_eq!(bar.colors, BarColors { top: palette[3], body: palette[4], bottom: palette[5], left: palette[3], right: palette[5] });
        // Its left end slants to the picture's first column at the bottom,
        // and stands upright from column 3 at the top.
        assert_eq!((bar.left_at(7.5), bar.left_at(3.5)), (0.0, 3.0));
        let l = letters(&p, |v| v == 0 || colors.contains(&v), Capitals::First).expect("letters");
        assert_eq!((l.rim, l.shadow, l.top, l.cap), (Some(6), None, 2, 6));
        assert_eq!(l.fill, vec![8, 8, 7, 1, 1, 8]);
        assert_eq!(l.span, (8, 16));
    }

    /// The damage digits: an orange and yellow face with a dark shadow
    /// right and down, no rim.
    #[test]
    fn a_shadow_is_right_and_down_of_the_face() {
        let p = picture(&[
            "........", //
            ".aaaa2..",
            ".a22a2..",
            ".9229...",
            ".9999...",
            ".aa2aa2.",
            ".aaaaa2.",
            "..22222.",
        ]);
        let l = letters(&p, |v| v == 0, Capitals::All).expect("letters");
        assert_eq!((l.rim, l.shadow), (None, Some(2)));
        assert_eq!((l.top, l.cap), (1, 6));
        assert_eq!(l.fill, vec![10, 10, 9, 9, 10, 10]);
    }

    /// Letters with no outline: their face's colors.
    #[test]
    fn letters_with_no_outline() {
        let p = picture(&["......", ".1111.", ".3333.", ".1111.", "......"]);
        let l = letters(&p, |v| v == 0, Capitals::All).expect("letters");
        assert_eq!((l.rim, l.shadow, l.fill.clone()), (None, None, vec![1, 3, 1]));
    }

    /// A picture whose first cell is a letter's has no bar.
    #[test]
    fn no_bar_without_a_bars_end() {
        let p = picture(&["..6666....6666..", ".661166..661166.", ".611116..611116.", ".666666..666666."]);
        assert!(bar(&p, &[0; 16]).is_none());
    }

    #[test]
    fn letters_erased_to_their_ground() {
        let p = picture(&["eee44eee", "e114411e", "e133311e"]);
        let ground = |v: u8| v == 1 || v == 14;
        assert_eq!(erased(&p, ground).pixels, picture(&["eeeeeeee", "e111111e", "e111111e"]).pixels);
    }
}
