//! Reading the battle's pictures' letters and digits as a pack is made
//! (`nettai_assets::letters`, docs/design/text-rendering.md §12): where a
//! picture's letters stand, how they look (palette indices: the face's
//! colors down the capitals, a rim all round or a shadow right and down),
//! a banner's bar, a lettered picture's lines, and the pictures with their
//! letters taken off. Extraction runs [`read`] once; a frontend applies
//! what it found and reads no picture itself.
//!
//! Nothing here is a game's: every look is the pack's pictures', so a game
//! whose pictures look otherwise is read otherwise. What [`read`] doesn't
//! find where every game has letters (a set of digits with no face, a Cross
//! name whose letters it can't find) it says, one line each.

use nettai_assets::letters::{
    BannerLetters, CountDigits, Shape, TextAlign, CountLetters, CustomLetters, Digits, FormNameLetters, GaugeWord, GlyphLetters, HelpLetters, HelpLine,
    HudLetters, HudWords, LabelLetters, Letters, PictureLetters, VersionLetters, WordLetters,
};
use nettai_assets::{Bundle, CustomScreen, Hud, MapEntry, Palette, Tiles, VersionPictures, Versioned, WindowLabel};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

    /// The tiles of glyphs side by side, as [`Picture::glyphs`] reads them.
    pub fn to_glyphs(&self) -> Tiles {
        let mut tiles = Tiles::default();
        for c in 0..self.w / 8 {
            for half in 0..2 {
                let tile: Vec<u8> = (0..64).map(|i| self.pixels[(8 * half + i / 8) * self.w + 8 * c + i % 8]).collect();
                tiles.push(&tile);
            }
        }
        tiles
    }
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
    letters_with(p, ground, capitals, rim, shadow)
}

/// How the letters of a picture look ([`letters`]) with their outline
/// given: a rim's color, or a shadow's, or none.
pub fn letters_with(p: &Picture, ground: impl Fn(u8) -> bool, capitals: Capitals, rim: Option<u8>, shadow: Option<u8>) -> Option<Letters> {
    let ink = |x: i32, y: i32| p.at(x, y).is_some_and(|v| !ground(v));
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
    let span = [*inked.first()? as i16, (inked.last()? + 1) as i16];
    Some(Letters { fill, rim, shadow, top: top as u8, cap: (bottom - top + 1) as u8, span })
}

/// A banner's bar ([`bar_shapes`]), read off its picture: the colors of its
/// first cell (8 columns), where that cell is a bar's end, every row of it
/// with pixels unbroken from them to the bar's last color on the row
/// (letters stand on the bar, so nothing between is clear); none for a
/// picture with no bar (the Japanese Program Advance's words, whose first
/// cell is a letter's). Its slant, its ends and its length are the
/// picture's: each row's first and last pixel of the bar's colors. With it,
/// the bar's colors.
pub fn bar(p: &Picture) -> Option<(Vec<Shape>, Vec<u8>)> {
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
    let slant = steps.get(steps.len() / 2).copied().unwrap_or(0).max(0);
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
    let (first, last) = (ends[0], ends[ends.len() - 1]);
    let geometry = BarGeometry {
        top: top as f32,
        bottom: (bottom + 1) as f32,
        left: last.0 as f32 - slant as f32 / 2.0,
        left_most: ends.iter().map(|e| e.0).max()? as f32,
        right: first.1 as f32 + slant as f32 / 2.0,
        right_least: ends.iter().map(|e| e.1).min()? as f32,
        slant: slant as f32,
    };
    let bar = bar_shapes(&geometry, [top_color, body, bottom_color, left, right]);
    Some((bar, colors))
}

/// A banner bar's outline: its top row and one past its bottom one; its
/// left end where it meets the bottom edge (it slants up to the right from
/// there) and the farthest right it goes (upright above); its right end
/// where it meets the top edge (it slants down to the left) and the
/// farthest left it goes (upright below); and how far an end leans a row.
#[derive(Clone, Copy, Debug)]
pub struct BarGeometry {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub left_most: f32,
    pub right: f32,
    pub right_least: f32,
    pub slant: f32,
}

impl BarGeometry {
    fn left_at(&self, y: f32) -> f32 {
        (self.left + self.slant * (self.bottom - y)).min(self.left_most)
    }

    fn right_at(&self, y: f32) -> f32 {
        (self.right - self.slant * (y - self.top)).max(self.right_least)
    }

    /// An end's corners down its rows (each end bends once, where it
    /// stands upright): the left's, or the right's.
    fn edge(&self, right: bool) -> Vec<[f32; 2]> {
        let at = |y: f32| if right { self.right_at(y) } else { self.left_at(y) };
        let mut ys = vec![self.top];
        if self.slant > 0.0 {
            let bend = if right { self.top + (self.right - self.right_least) / self.slant } else { self.bottom - (self.left_most - self.left) / self.slant };
            if bend > self.top && bend < self.bottom {
                ys.push(bend);
            }
        }
        ys.push(self.bottom);
        ys.into_iter().map(|y| [at(y), y]).collect()
    }
}

/// A bar's shapes (`colors`: its top line's, its body's, its bottom
/// line's, its left end's and its right end's, palette indices), each line
/// and end a frame pixel wide, as the pictures' are: the body (the whole
/// bar), then over it the right end, the bottom line, the top line and the
/// left end (where two meet, the later's color: left, top, bottom, right).
pub fn bar_shapes(g: &BarGeometry, [top, body, bottom, left, right]: [u8; 5]) -> Vec<Shape> {
    const LINE: f32 = 1.0;
    let (l, r) = (g.edge(false), g.edge(true));
    let whole: Vec<[f32; 2]> = l.iter().copied().chain(r.iter().rev().copied()).collect();
    // A band of rows of the bar (`from..to`), clipped from the whole.
    let rows = |from: f32, to: f32| -> Vec<[f32; 2]> { clip_rows(&clip_rows(&whole, from, true), to, false) };
    let strip = |edge: &[[f32; 2]], dx: f32| -> Vec<[f32; 2]> { edge.iter().copied().chain(edge.iter().rev().map(|p| [p[0] + dx, p[1]])).collect() };
    let shape = |points: Vec<[f32; 2]>, color: u8| Shape { points: points.iter().map(|p| p.map(|v| (v * Shape::UNIT).round() as i16)).collect(), color };
    vec![
        shape(whole.clone(), body),
        shape(strip(&r, -LINE), right),
        shape(rows(g.bottom - LINE, g.bottom), bottom),
        shape(rows(g.top, g.top + LINE), top),
        shape(strip(&l, LINE), left),
    ]
}

/// A polygon cut at row `y`, keeping what is below it (`below`) or above.
fn clip_rows(points: &[[f32; 2]], y: f32, below: bool) -> Vec<[f32; 2]> {
    let keep = |p: &[f32; 2]| if below { p[1] >= y } else { p[1] <= y };
    let mut out = Vec::new();
    for k in 0..points.len() {
        let (a, b) = (points[k], points[(k + 1) % points.len()]);
        if keep(&a) {
            out.push(a);
        }
        if keep(&a) != keep(&b) {
            let t = (y - a[1]) / (b[1] - a[1]);
            out.push([a[0] + t * (b[0] - a[0]), y]);
        }
    }
    out
}

/// The lines of a picture's lettering, top to bottom: runs of rows with
/// letters on them (pixels not `ground`), a clear row apart ([`line_rows`]:
/// two lines a mark joins are told apart by the lines' pitch). Each line's
/// baseline is under the last row most of its letters (runs of columns)
/// reach, so a descender doesn't move it; its color is the one most of its
/// pixels are in `palette` (the index of its first pixel of that color).
pub fn lines(p: &Picture, ground: impl Fn(u8) -> bool, palette: &Palette) -> Vec<HelpLine> {
    let ink = |x: i32, y: i32| p.at(x, y).is_some_and(|v| !ground(v));
    let inked: Vec<bool> = (0..p.h as i32).map(|y| (0..p.w as i32).any(|x| ink(x, y))).collect();
    let mut out = Vec::new();
    for (top, end) in line_rows(&inked) {
        let rows = top as i32..end as i32;
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
        let baseline = seen.iter().max_by_key(|s| (s.1, s.0)).map_or(end as i32, |s| s.0 + 1);
        // (Each color with its first index and how many pixels.)
        let mut colors: Vec<(u16, u8, usize)> = Vec::new();
        for r in rows.clone() {
            for x in first..=last {
                if let Some(v) = p.at(x, r).filter(|&v| !ground(v)) {
                    let c = palette[v as usize & 15] & 0x7FFF;
                    match colors.iter_mut().find(|s| s.0 == c) {
                        Some(s) => s.2 += 1,
                        None => colors.push((c, v, 1)),
                    }
                }
            }
        }
        let color = colors.iter().max_by_key(|s| (s.2, s.0)).map_or(0, |s| s.1);
        out.push(HelpLine { top: top as u8, baseline: baseline as u8, span: [first as u8, (last + 1) as u8], color });
    }
    out
}

/// The rows of a picture's lines (each its first and one past its last),
/// from which of its rows have letters on them (`inked`): the runs of such
/// rows a clear row apart, but where a run's next one starts two of the
/// lines' pitches or more after it (the pitch: the most common step from a
/// run's first row to the next's, with three runs or more), the run is that
/// many lines at that pitch: a mark of the lower line's letters (a voiced
/// mark) reached the line above and left no clear row between (EXE4's
/// Japanese OK picture: チップが over えらばれて).
pub fn line_rows(inked: &[bool]) -> Vec<(usize, usize)> {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut y = 0;
    while y < inked.len() {
        if !inked[y] {
            y += 1;
            continue;
        }
        let top = y;
        while y < inked.len() && inked[y] {
            y += 1;
        }
        runs.push((top, y));
    }
    let mut steps: Vec<usize> = runs.windows(2).map(|w| w[1].0 - w[0].0).collect();
    steps.sort_unstable();
    let Some(&pitch) = steps.get(steps.len() / 2).filter(|_| steps.len() >= 2) else { return runs };
    let mut out = Vec::new();
    for (k, &(top, end)) in runs.iter().enumerate() {
        let step = runs.get(k + 1).map(|next| next.0 - top);
        let n = step.map_or(1, |s| (s + pitch / 2) / pitch.max(1));
        match step {
            Some(step) if pitch > 0 && step + 1 >= 2 * pitch && n >= 2 && end - top > pitch => {
                for i in 0..n {
                    let (from, to) = (top + i * step / n, (top + (i + 1) * step / n).min(end));
                    if from < to {
                        out.push((from, to));
                    }
                }
            }
            _ => out.push((top, end)),
        }
    }
    out
}

/// What is behind a small picture's letters (a chip code in its cell): its
/// most common color and the colors that fill a whole side of it (a cell's
/// frame), where letters don't reach. (A letter may touch a side: the chip
/// window's codes fill their cell's width.)
pub fn ground_of(p: &Picture) -> Vec<u8> {
    let mut count = [0usize; 16];
    p.pixels.iter().for_each(|&v| count[v as usize & 15] += 1);
    let mut ground = vec![(0..16u8).max_by_key(|&c| count[c as usize]).unwrap_or(0)];
    if p.w == 0 || p.h == 0 {
        return ground;
    }
    let at = |x: usize, y: usize| p.pixels[y * p.w + x];
    let sides: [Vec<u8>; 4] = [
        (0..p.w).map(|x| at(x, 0)).collect(),
        (0..p.w).map(|x| at(x, p.h - 1)).collect(),
        (0..p.h).map(|y| at(0, y)).collect(),
        (0..p.h).map(|y| at(p.w - 1, y)).collect(),
    ];
    for side in sides {
        if side.iter().all(|&v| v == side[0]) && !ground.contains(&side[0]) {
            ground.push(side[0]);
        }
    }
    ground
}

/// What is behind the letters of a picture of the HUD layer's cells: its
/// most common color, and the colors of its last row (the gauge's line).
pub fn cells_ground(p: &Picture) -> Vec<u8> {
    let mut count = [0usize; 16];
    p.pixels.iter().for_each(|&v| count[v as usize & 15] += 1);
    let mut ground = vec![(0..16u8).max_by_key(|&c| count[c as usize]).unwrap_or(0)];
    for &v in &p.pixels[(p.h - 1) * p.w..] {
        if !ground.contains(&v) {
            ground.push(v);
        }
    }
    ground
}

/// Letters a picture draws with outlines in its own edge's color (EXE6's
/// Cross names on their bars): which pixels are theirs (`mask`), and the
/// outline's color.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Enclosed {
    pub mask: Vec<bool>,
    pub outline: u8,
}

/// The letters a picture draws in outlines of its edge's color ([`Enclosed`]):
/// the edge's color is the one most of its edge pixels are; the outlines are
/// its pixels of that color off the edge (a letter's outline may touch it,
/// as EXE6's Japanese トマホーク and スラッシュ do); the letters are the
/// outlines and what they enclose, in the outlines' box: what isn't reached
/// from the box's edge without crossing one. None if it has none.
pub fn enclosed_letters(p: &Picture) -> Option<Enclosed> {
    let (w, h) = (p.w as i32, p.h as i32);
    let at = |x: i32, y: i32| p.pixels[(y * w + x) as usize];
    let edge = |x: i32, y: i32| x == 0 || y == 0 || x == w - 1 || y == h - 1;
    let mut count = [0usize; 16];
    for y in 0..h {
        for x in (0..w).filter(|&x| edge(x, y)) {
            count[at(x, y) as usize & 15] += 1;
        }
    }
    let outline = (0..16u8).max_by_key(|&c| count[c as usize])?;
    let near = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    // (The edge itself only: an outline joined to it is a letter's.)
    let joined: Vec<bool> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| edge(x, y)).collect();
    let inner: Vec<(i32, i32)> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| at(x, y) == outline && !joined[(y * w + x) as usize]).collect();
    let (x0, x1) = (inner.iter().map(|p| p.0).min()?, inner.iter().map(|p| p.0).max()?);
    let (y0, y1) = (inner.iter().map(|p| p.1).min()?, inner.iter().map(|p| p.1).max()?);
    // What the box's edge reaches without crossing an outline.
    let inside = |x: i32, y: i32| (x0..=x1).contains(&x) && (y0..=y1).contains(&y);
    let mut reached = vec![false; p.pixels.len()];
    let mut stack: Vec<(i32, i32)> = (y0..=y1)
        .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
        .filter(|&(x, y)| (x == x0 || x == x1 || y == y0 || y == y1) && at(x, y) != outline)
        .collect();
    while let Some((x, y)) = stack.pop() {
        let i = (y * w + x) as usize;
        if reached[i] {
            continue;
        }
        reached[i] = true;
        for (dx, dy) in near {
            let (nx, ny) = (x + dx, y + dy);
            if inside(nx, ny) && at(nx, ny) != outline && !reached[(ny * w + nx) as usize] {
                stack.push((nx, ny));
            }
        }
    }
    let mask = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let i = (y * w + x) as usize;
            inside(x, y) && (if at(x, y) == outline { !joined[i] } else { !reached[i] })
        })
        .collect();
    Some(Enclosed { mask, outline })
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
            if let Some(v) = nearest(row, x, &ground) {
                out.pixels[y * p.w + x] = v;
            }
        }
    }
    out
}

/// The pixel of `row` nearest `x` that `source` takes (the left's where
/// both are as near).
fn nearest(row: &[u8], x: usize, source: impl Fn(u8) -> bool) -> Option<u8> {
    let left = (0..x).rev().find(|&i| source(row[i])).map(|i| (x - i, row[i]));
    let right = (x + 1..row.len()).find(|&i| source(row[i])).map(|i| (i - x, row[i]));
    match (left, right) {
        (Some(l), Some(r)) => Some(if r.0 < l.0 { r.1 } else { l.1 }),
        (l, r) => l.or(r).map(|v| v.1),
    }
}

/// The look of `count` glyphs of 8x16 from `first` of `tiles` (a set of
/// digits), `ground` behind them: the box their faces fill in their cells
/// (the faces' columns of all the glyphs together, their capitals' rows),
/// and their paint.
pub fn digits(tiles: &Tiles, first: usize, count: usize, ground: impl Fn(u8) -> bool) -> Option<Digits> {
    let p = Picture::glyphs(tiles, first, count);
    let l = letters(&p, &ground, Capitals::All)?;
    let line = l.rim.or(l.shadow);
    let face = |x: usize, y: usize| {
        let v = p.pixels[y * p.w + x];
        !ground(v) && Some(v) != line
    };
    let columns: Vec<usize> = (0..8).filter(|&x| (0..count).any(|c| (0..p.h).any(|y| face(8 * c + x, y)))).collect();
    let (x0, x1) = (*columns.first()?, *columns.last()?);
    Some(Digits { face: [x0 as u8, l.top, (x1 + 1 - x0) as u8, l.cap], fill: l.fill, rim: l.rim, shadow: l.shadow })
}

/// A letter in its cell (a chip code): its look, on its cell's own colors
/// ([`ground_of`]: its most common and those filling a whole side), and the
/// cell with it taken off; none for a cell with no letter.
fn cell_letter(p: &Picture) -> Option<(Digits, Picture)> {
    let ground = ground_of(p);
    let is_ground = |v: u8| ground.contains(&v);
    let l = letters(p, is_ground, Capitals::All)?;
    let line = l.rim.or(l.shadow);
    let face = |x: usize, y: usize| {
        let v = p.pixels[y * p.w + x];
        !is_ground(v) && Some(v) != line
    };
    let xs: Vec<usize> = (0..p.w).filter(|&x| (0..p.h).any(|y| face(x, y))).collect();
    let ys: Vec<usize> = (0..p.h).filter(|&y| (0..p.w).any(|x| face(x, y))).collect();
    let (x0, x1, y0, y1) = (*xs.first()?, *xs.last()?, *ys.first()?, *ys.last()?);
    let look = Digits { face: [x0 as u8, y0 as u8, (x1 + 1 - x0) as u8, (y1 + 1 - y0) as u8], fill: l.fill, rim: l.rim, shadow: l.shadow };
    Some((look, erased(p, is_ground)))
}

/// Read every picture's letters and digits of `bundle` into its `letters`
/// (the HUD's and the custom screen's, in each language it has), and say
/// what it can't find where every game has it.
pub fn read(bundle: &mut Bundle) -> Vec<String> {
    let mut problems = Vec::new();
    let own = bundle.hud.language().to_string();
    let mut say = |lang: &str, what: String| problems.push(if lang == own { what } else { format!("{lang}: {what}") });
    if !bundle.hud.tiles.is_empty() {
        bundle.hud.letters = hud_letters(&bundle.hud, &mut |w| say(&own, w));
    }
    if !bundle.custom.is_empty() {
        bundle.custom.letters = custom_letters(&bundle.custom, &mut |w| say(&own, w));
    }
    let languages: Vec<String> = bundle.hud.languages.iter().map(|(l, _)| l.clone()).collect();
    for lang in languages {
        let Ok(other) = bundle.clone().in_language(&lang) else {
            say(&lang, "the pack's lettering in this language doesn't swap in".into());
            continue;
        };
        let words = hud_words(&other.hud, &mut |w| say(&lang, w));
        if let Some((_, l)) = bundle.hud.languages.iter_mut().find(|(l, _)| *l == lang) {
            l.letters = words;
        }
        if !other.custom.is_empty() {
            let pictures = picture_letters(&other.custom, &mut |w| say(&lang, w));
            if let Some((_, l)) = bundle.custom.languages.iter_mut().find(|(l, _)| *l == lang) {
                l.letters = pictures;
            }
        }
    }
    // (The box every form list name is centered in.)
    let c = &mut bundle.custom;
    let all = std::iter::once(&c.letters.pictures).chain(c.languages.iter().map(|(_, l)| &l.letters));
    let spans: Vec<[u8; 2]> = all
        .flat_map(|p| std::iter::once(&p.versioned.base).chain(p.versioned.versions.iter().map(|(_, v)| v)))
        .flat_map(|v| v.form_names.iter().flatten().map(|f| f.columns))
        .collect();
    c.letters.form_name_box = spans.iter().map(|s| s[0]).min().zip(spans.iter().map(|s| s[1]).max()).map(|(a, b)| [a, b]);
    // (The blanks as the pack keeps them: one base a picture.)
    problems.extend(settle(bundle).into_iter().map(|n| format!("note: {n}")));
    problems
}

/// The HUD's letters and digits ([`HudLetters`]).
pub fn hud_letters(h: &Hud, say: &mut dyn FnMut(String)) -> HudLetters {
    let mut need = |what: &str, found: Option<Digits>| {
        if found.is_none() {
            say(format!("no digits found in {what}"));
        }
        found
    };
    let blank = Picture::glyphs(&h.tiles, Hud::HP_BLANK, 1);
    let hp_digits = need("the HP box's digits", digits(&h.tiles, 0, 10, |v| blank.pixels.contains(&v)));
    let clear = |v: u8| v == 0;
    let damage_digits = need("the damage digits", digits(&h.tiles, Hud::DAMAGE_DIGIT, 10, clear));
    let plus = need("the damage's '+'", digits(&h.tiles, Hud::PLUS_GLYPH, 1, clear));
    let times = need("the damage's \"x2\"", digits(&h.tiles, Hud::TIMES_GLYPH, 2, clear));
    let enemy_digits = [0, 1, 2].map(|k| need(&format!("the HP digits of color {k}"), digits(&h.enemy_digits[k], 0, 10, clear)));
    let banner_digits = need("the banners' digits", digits(&h.banner_digits, 0, 10, clear));
    let telop = h.layout.telop.as_ref().and_then(|t| {
        let set = [
            digits(&t.glyphs, 0, 10, clear),
            digits(&t.glyphs, nettai_assets::TelopLook::PLUS, 1, clear),
            digits(&t.glyphs, nettai_assets::TelopLook::TIMES, 2, clear),
        ];
        match set {
            [Some(d), Some(p), Some(t)] => Some([d, p, t]),
            _ => {
                say("no digits found in the telop's glyphs".into());
                None
            }
        }
    });
    let pause = letters(&Picture::glyphs(&h.pause, 0, 5), clear, Capitals::First);
    if pause.is_none() && !h.pause.is_empty() {
        say("no letters found in PAUSE".into());
    }
    let counts = count_letters(&h.counts);
    if h.counts.len() > 1 && counts.counts.iter().all(Option::is_none) {
        say("no digits found in the emotion window's counts".into());
    }
    HudLetters {
        hp_digits,
        damage_digits,
        plus,
        times,
        enemy_digits,
        banner_digits,
        telop,
        pause,
        counts,
        words: hud_words(h, say),
    }
}

/// The HUD's words in its language ([`HudWords`]).
pub fn hud_words(h: &Hud, say: &mut dyn FnMut(String)) -> HudWords {
    let banners = h.banners.iter().map(banner_letters).collect();
    let columns = (h.waiting.len() / 2).max(1);
    let waiting = letters(&Picture::grid(&h.waiting, 0, columns, 2), |v| v == 0, Capitals::First);
    if waiting.is_none() && !h.waiting.is_empty() {
        say("no letters found in the waiting label".into());
    }
    let l_or_r = [0u16, 1].map(|frame| {
        let first = Hud::GAUGE_L_OR_R as usize + 4 * frame as usize;
        // (The gauge's own tiles: the pack keeps them with the letters
        // taken off, `gauge_base`.)
        let tiles: Vec<&[u8]> = (0..4).filter_map(|i| h.gauge_tiles.get(first + i)).collect();
        if tiles.len() != 4 {
            return None;
        }
        let picture = Picture { w: 32, h: 8, pixels: tiles_row(&tiles) };
        let ground = cells_ground(&picture);
        let found = letters(&picture, |v| ground.contains(&v), Capitals::First);
        if found.is_none() {
            say(format!("no letters found in the full gauge's frame {frame}"));
        }
        Some(WordLetters { letters: found?, blank: erased(&picture, |v| ground.contains(&v)).to_grid() })
    });
    let gauge = gauge_word(h);
    if gauge.is_none() && !h.gauge_frame.is_empty() {
        say("no word found in the custom gauge's frame".into());
    }
    HudWords { banners, waiting, l_or_r, gauge }
}

/// Tiles side by side, as a picture's pixels (8 rows).
fn tiles_row(tiles: &[&[u8]]) -> Vec<u8> {
    (0..8).flat_map(|y| tiles.iter().flat_map(move |t| t[8 * y..8 * y + 8].iter().copied())).collect()
}

/// The custom gauge's frame's word: the cells of the frame's top row
/// between its corners that aren't its line's own (the row's most common
/// cell), their tiles with the letters taken off (their plate's color, the
/// cells' most common: `cells_ground`), and the letters.
fn gauge_word(h: &Hud) -> Option<GaugeWord> {
    let top = h.gauge_frame.get(..Hud::GAUGE_COLUMNS)?;
    let blank = *top.iter().max_by_key(|e| top.iter().filter(|o| o.tile == e.tile).count())?;
    let cells: Vec<usize> = (1..top.len() - 1).filter(|&i| top[i].tile != blank.tile).collect();
    if cells.is_empty() || cells.windows(2).any(|w| w[1] != w[0] + 1) {
        return None;
    }
    // (The gauge's own tiles: the pack keeps them with the letters taken
    // off, `gauge_base`.)
    let tiles: Vec<&[u8]> = cells.iter().map(|&i| h.gauge_tiles.get(top[i].tile.checked_sub(h.gauge_first_tile)? as usize)).collect::<Option<_>>()?;
    let picture = Picture { w: 8 * tiles.len(), h: 8, pixels: tiles_row(&tiles) };
    // (The word is what its cells draw over the frame's line: each cell is
    // the line's cell, the row's most common, where it draws the same, and
    // the word elsewhere, its letters and their outline. The letters stand
    // so close that their outlines merge into what looks like a tab: the
    // user, "the dark box is actually the outline of the text, it just
    // matches up so close". So the word's cells with it taken off are the
    // line's cell, and the letters are read off the word alone, their
    // outline a rim.)
    let line = h.layer_tile(blank.tile)?.to_vec();
    let line_row = Picture { w: picture.w, h: 8, pixels: tiles_row(&vec![line.as_slice(); tiles.len()]) };
    let word = Picture { pixels: picture.pixels.iter().zip(&line_row.pixels).map(|(&v, &l)| if v == l { 0 } else { v }).collect(), ..picture.clone() };
    let mut letters = letters(&word, |v| v == 0, Capitals::First)?;
    // (Where the letters stand on the line, their outline doesn't keep
    // them off the ground all round, EXE4's: it is their outline still, the
    // word's most common color.)
    if letters.rim.is_none() && letters.shadow.is_none() {
        let mut count = [0usize; 16];
        word.pixels.iter().filter(|&&v| v != 0).for_each(|&v| count[v as usize & 15] += 1);
        let outline = (1..16u8).max_by_key(|&c| count[c as usize])?;
        letters = letters_with(&word, |v| v == 0, Capitals::First, Some(outline), None)?;
    }
    let blank = line_row.to_grid();
    let align = TextAlign::of(letters.span, picture.w);
    Some(GaugeWord { cells: cells.into_iter().map(|c| c as u8).collect(), blank, letters, align })
}

/// A banner's letters ([`BannerLetters`]): none for one with no glyphs (a
/// telop's) or no letters (a logo).
fn banner_letters(layout: &nettai_assets::BannerLayout) -> Option<BannerLetters> {
    if layout.glyphs.is_empty() {
        return None;
    }
    let picture = Picture::glyphs(&layout.glyphs, 0, 20);
    let bar = bar(&picture);
    let bar_colors = bar.as_ref().map(|(_, c)| c.clone()).unwrap_or_default();
    let ground = |v: u8| v == 0 || bar_colors.contains(&v);
    let found = letters(&picture, ground, Capitals::First)?;
    // The letters' columns in the picture's columns `from..to`, where it
    // has letters there.
    let part = |from: i32, to: i32| -> Option<[i16; 2]> {
        let from = from.max(0);
        let cut = picture.crop(from as usize, 0, (to.min(picture.w as i32) - from).max(0) as usize, picture.h);
        let span = letters(&cut, ground, Capitals::First)?.span;
        Some([from as i16 + span[0], from as i16 + span[1]])
    };
    // Either side of the number, as one digit (moved 4 left) and as two.
    let (x, w) = (layout.x as i32, picture.w as i32);
    let sides = |nx: i32| {
        let cut = (nx - 4 - x, nx + 16 - x);
        [part(0, cut.0), part(cut.1, w)]
    };
    let (one_digit, two_digits) = match layout.number_at {
        Some((nx, _)) => (Some(sides(nx as i32 - 4)), Some(sides(nx as i32))),
        None => (None, None),
    };
    Some(BannerLetters { bar: bar.map(|(b, _)| b), letters: found, one_digit, two_digits })
}

/// The emotion window's counts' boxes ([`CountLetters`]): their box with no
/// digits, where the HUD has more than one count, and each count's digits.
fn count_letters(counts: &[Tiles]) -> CountLetters {
    let digits: Vec<Option<CountDigits>> = (0..counts.len()).map(|n| count_digits(counts, n)).collect();
    // (The blank box where a count's digits are drawn in a font.)
    let blank = if digits.iter().any(Option::is_some) { count_blank(counts).to_grid() } else { Tiles::default() };
    CountLetters { blank, counts: digits }
}

/// The empty box of the emotion window's counts (16x16 each): where they
/// all have one color, that; in their digits' place (the box around where
/// they differ), the box's own color around it.
fn count_blank(counts: &[Tiles]) -> Picture {
    let pictures: Vec<Picture> = count_boxes(counts).into_iter().map(|t| Picture::grid(&counts[t], 0, 2, 2)).collect();
    if pictures.is_empty() {
        return Picture { w: 16, h: 16, pixels: vec![0; 256] };
    }
    // (The digits' place: the box around the pixels the counts differ at, a
    // pixel every digit has included.)
    let differs: Vec<usize> = (0..256).filter(|&i| pictures.iter().any(|p| p.pixels[i] != pictures[0].pixels[i])).collect();
    let (x0, x1) = (differs.iter().map(|i| i % 16).min().unwrap_or(16), differs.iter().map(|i| i % 16).max().unwrap_or(0));
    let (y0, y1) = (differs.iter().map(|i| i / 16).min().unwrap_or(16), differs.iter().map(|i| i / 16).max().unwrap_or(0));
    let varies = |i: usize| (x0..=x1).contains(&(i % 16)) && (y0..=y1).contains(&(i / 16));
    // (The box's own color behind the digits: the one most of the pixels
    // around their place are, which every count has alike. Not the place's
    // own most common color: the digits fill most of it.)
    let mut seen = [0usize; 16];
    for i in (0..256).filter(|&i| !varies(i)) {
        let (x, y) = ((i % 16) as i32, (i / 16) as i32);
        let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            (0..16).contains(&nx) && (0..16).contains(&ny) && varies((ny * 16 + nx) as usize)
        });
        if near {
            seen[pictures[0].pixels[i] as usize & 15] += 1;
        }
    }
    let behind = (0..16u8).max_by_key(|&c| seen[c as usize]).unwrap_or(0);
    let pixels = (0..256).map(|i| if varies(i) { behind } else { pictures[0].pixels[i] }).collect();
    Picture { w: 16, h: 16, pixels }
}

/// The counts that are one box with a number in it: those whose edges (the
/// first and last rows and columns) are the first count's (EXE6's last two
/// are other boxes).
fn count_boxes(counts: &[Tiles]) -> Vec<usize> {
    let pictures: Vec<Picture> = counts.iter().map(|t| Picture::grid(t, 0, 2, 2)).collect();
    let edge = |p: &Picture| -> Vec<u8> { (0..256).filter(|i| i / 16 == 0 || i / 16 == 15 || i % 16 == 0 || i % 16 == 15).map(|i| p.pixels[i]).collect() };
    let Some(first) = pictures.first() else { return Vec::new() };
    (0..pictures.len()).filter(|&k| edge(&pictures[k]) == edge(first)).collect()
}

/// Count `n`'s digits in its box: each a single digit's box (the union of
/// the single digits'), or for a count of two digits its own two, apart by
/// the clear columns between them; and their paint. None for a count that
/// isn't one of the boxes, or a box with no number.
fn count_digits(counts: &[Tiles], n: usize) -> Option<CountDigits> {
    let chars = n.to_string();
    if !count_boxes(counts).contains(&n) {
        return None;
    }
    let blank = count_blank(counts);
    // A count's digits: its pixels that aren't the box's.
    let ink = |m: usize| -> Option<Picture> {
        let p = Picture::grid(counts.get(m)?, 0, 2, 2);
        let pixels = p.pixels.iter().zip(&blank.pixels).map(|(&v, &b)| if v == b { 0 } else { v }).collect();
        Some(Picture { pixels, ..p })
    };
    let bounds = |p: &Picture, (from, to): (usize, usize)| -> Option<[i32; 4]> {
        let inked = |x: usize, y: usize| (from..to).contains(&x) && p.pixels[y * 16 + x] != 0;
        let xs: Vec<usize> = (0..16).filter(|&x| (0..16).any(|y| inked(x, y))).collect();
        let ys: Vec<usize> = (0..16).filter(|&y| (0..16).any(|x| inked(x, y))).collect();
        let (x0, x1, y0, y1) = (*xs.first()?, *xs.last()?, *ys.first()?, *ys.last()?);
        Some([x0 as i32, y0 as i32, (x1 - x0 + 1) as i32, (y1 - y0 + 1) as i32])
    };
    let union = |a: [i32; 4], b: [i32; 4]| {
        let (x0, y0) = (a[0].min(b[0]), a[1].min(b[1]));
        [x0, y0, (a[0] + a[2]).max(b[0] + b[2]) - x0, (a[1] + a[3]).max(b[1] + b[3]) - y0]
    };
    let this = ink(n)?;
    if this.pixels.iter().all(|&v| v == 0) {
        return None;
    }
    let boxes: Vec<[i32; 4]> = if chars.len() == 1 {
        let single = count_boxes(counts).into_iter().filter(|&m| m.to_string().len() == 1).filter_map(|m| bounds(&ink(m)?, (0, 16))).reduce(union)?;
        vec![single]
    } else {
        // The count's own columns, in runs apart by clear ones.
        let columns: Vec<usize> = (0..16).filter(|&x| (0..16).any(|y| this.pixels[y * 16 + x] != 0)).collect();
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for c in columns {
            match runs.last_mut() {
                Some(r) if r.1 == c => r.1 = c + 1,
                _ => runs.push((c, c + 1)),
            }
        }
        if runs.len() != chars.len() {
            return None;
        }
        runs.iter().map(|&r| bounds(&this, r)).collect::<Option<_>>()?
    };
    let l = letters(&this, |v| v == 0, Capitals::All)?;
    Some(CountDigits { glyphs: boxes.into_iter().map(|b| b.map(|v| v as u8)).collect(), fill: l.fill, rim: l.rim, shadow: l.shadow })
}

/// The custom screen's letters and digits ([`CustomLetters`]).
pub fn custom_letters(c: &CustomScreen, say: &mut dyn FnMut(String)) -> CustomLetters {
    // The chip window's codes (8x16 glyphs) and the slots' (2x1 tiles).
    let glyphs = |tiles: &Tiles, picture: &dyn Fn(usize) -> Picture, back: &dyn Fn(&Picture) -> Tiles| {
        let mut out = GlyphLetters::default();
        for k in 0..tiles.len() / 2 {
            let p = picture(k);
            match cell_letter(&p) {
                Some((look, blank)) => {
                    out.glyphs.push(Some(look));
                    out.blank.pixels.extend(back(&blank).pixels);
                }
                None => {
                    out.glyphs.push(None);
                    out.blank.pixels.extend(back(&p).pixels);
                }
            }
        }
        // (A tile past the last glyph's stays: the blanks are the glyphs'
        // image, tile for tile.)
        out.blank.pixels.extend_from_slice(&tiles.pixels[out.blank.pixels.len().min(tiles.pixels.len())..]);
        out.face = GlyphLetters::face_of(&out.glyphs);
        out
    };
    let codes = glyphs(&c.codes, &|k| Picture::glyphs(&c.codes, k, 1), &Picture::to_glyphs);
    let slot_codes = glyphs(&c.slot_codes, &|k| Picture::grid(&c.slot_codes, 2 * k, 2, 1), &Picture::to_grid);
    // (The letters every game's code glyphs have: A to Z and '*'.)
    for (what, set) in [("the chip window's", &codes), ("the slots'", &slot_codes)] {
        let missing: Vec<usize> = (0..CODE_LETTERS.min(set.glyphs.len())).filter(|&k| set.glyphs[k].is_none()).collect();
        if !missing.is_empty() || set.glyphs.len() < CODE_LETTERS {
            say(format!("no letter found in {what} chip code glyphs {missing:?} (of {})", set.glyphs.len()));
        }
    }
    let ground = Picture::glyphs(&c.digits, 0, 1).at(0, 0);
    let digits = ground.and_then(|g| digits(&c.digits, 0, 10, |v| v == g));
    if digits.is_none() {
        say("no digits found in the chip window's damage digits".into());
    }
    let turn_limit = (c.turn_limit.len() >= 14)
        .then(|| {
            let picture = Picture::grid(&c.turn_limit, 0, 7, 2);
            let found = letters(&picture, |v| v == 0, Capitals::First)?;
            Some(WordLetters { letters: found, blank: erased(&picture, |v| v == 0).to_grid() })
        })
        .flatten();
    if turn_limit.is_none() && !c.turn_limit.is_empty() {
        say("no letters found in FINAL TURN".into());
    }
    let mut window_blank = c.window_tiles.clone();
    let labels = c
        .labels
        .iter()
        .map(|label| {
            let found = label_letters(c, label, &mut window_blank);
            if let Err(e) = &found {
                say(format!("window label {:?}: {e}", label.name));
            }
            found.ok()
        })
        .collect();
    CustomLetters { codes, slot_codes, digits, turn_limit, labels, window_blank, form_name_box: None, pictures: picture_letters(c, say) }
}

/// The chip codes with letters: A to Z, then '*'.
const CODE_LETTERS: usize = 27;

/// A window label's letters ([`LabelLetters`]), its tiles' letters taken off
/// in `window` (the window's tiles), as the window's maps lay them: on one
/// row, each map that shows them alike.
fn label_letters(c: &CustomScreen, label: &WindowLabel, window: &mut Tiles) -> Result<LabelLetters, String> {
    let range = label.first..=label.last;
    // Each map's cells that show the label's tiles, from the first's.
    let mut laid: Vec<Vec<(usize, usize, MapEntry)>> = Vec::new();
    for map in c.window_maps.iter().chain(&c.form_list_maps) {
        let cells: Vec<(usize, usize, MapEntry)> =
            map.iter().enumerate().filter(|(_, e)| range.contains(&e.tile)).map(|(i, e)| (i % MAP_COLUMNS, i / MAP_COLUMNS, *e)).collect();
        let (Some(col), Some(row)) = (cells.iter().map(|c| c.0).min(), cells.iter().map(|c| c.1).min()) else { continue };
        laid.push(cells.into_iter().map(|(x, y, e)| (x - col, y - row, e)).collect());
    }
    let Some(cells) = laid.first().cloned() else { return Err("no map shows its tiles".into()) };
    if laid.iter().any(|l| *l != cells) {
        return Err("the maps lay its tiles otherwise".into());
    }
    let w = cells.iter().map(|c| c.0).max().unwrap_or(0) + 1;
    if cells.iter().any(|c| c.1 != 0) {
        return Err("its tiles take more than a row".into());
    }
    let tile = |t: u16| window.get(t.checked_sub(CustomScreen::WINDOW_TILE)? as usize).map(<[u8]>::to_vec);
    let mut picture = Picture { w: 8 * w, h: 8, pixels: vec![0; 64 * w] };
    for &(x, _, e) in &cells {
        let t = tile(e.tile).ok_or("a tile past the window's")?;
        for (i, &v) in t.iter().enumerate() {
            let (px, py) = (i % 8, i / 8);
            let (px, py) = (if e.hflip { 7 - px } else { px }, if e.vflip { 7 - py } else { py });
            picture.pixels[py * picture.w + 8 * x + px] = v;
        }
    }
    let (top, bottom) = (label.top as usize, (label.bottom as usize).min(7));
    let is_letter = |v: u8| label.letters.contains(&v);
    let mut letters_only = picture.crop(0, top, picture.w, bottom + 1 - top);
    letters_only.pixels.iter_mut().for_each(|p| *p = if is_letter(*p) { *p } else { 0 });
    let found = letters(&letters_only, |v| v == 0, Capitals::First).ok_or("no letters in its colors")?;
    // Its letters taken off: each pixel the window's own color nearest it
    // on its row.
    let mut blank = picture.clone();
    for y in top..=bottom {
        let row: Vec<u8> = picture.pixels[y * picture.w..(y + 1) * picture.w].to_vec();
        for x in (0..picture.w).filter(|&x| is_letter(row[x])) {
            if let Some(n) = nearest(&row, x, |v| !is_letter(v)) {
                blank.pixels[y * picture.w + x] = n;
            }
        }
    }
    for &(x, _, e) in &cells {
        let t: Vec<u8> = (0..64)
            .map(|i| {
                let (px, py) = (i % 8, i / 8);
                let (px, py) = (if e.hflip { 7 - px } else { px }, if e.vflip { 7 - py } else { py });
                blank.pixels[py * blank.w + 8 * x + px]
            })
            .collect();
        let k = (e.tile - CustomScreen::WINDOW_TILE) as usize;
        window.pixels[k * Tiles::TILE..(k + 1) * Tiles::TILE].copy_from_slice(&t);
    }
    // The letters: runs of columns, and the rows their ink takes.
    let inked = |x: usize, y: usize| letters_only.pixels[y * letters_only.w + x] != 0;
    let rows: Vec<usize> = (0..letters_only.h).filter(|&y| (0..letters_only.w).any(|x| inked(x, y))).collect();
    let (Some(&row0), Some(&row1)) = (rows.first(), rows.last()) else { return Err("no letters in its colors".into()) };
    Ok(LabelLetters { letters: found, rows: [(top + row0) as u8, (top + row1) as u8] })
}

/// The window's maps' columns (15 a row).
const MAP_COLUMNS: usize = 15;

/// The custom screen's lettered pictures in its language
/// ([`PictureLetters`]).
pub fn picture_letters(c: &CustomScreen, say: &mut dyn FnMut(String)) -> PictureLetters {
    let mut need = |what: &str, p: &nettai_assets::Picture| {
        let found = help_letters(p);
        if found.is_none() && !p.tiles.is_empty() {
            say(format!("no lines found in the chip window's picture {what}"));
        }
        found
    };
    let ok = need("ok", &c.pictures.ok);
    let ok_picked = need("ok-picked", &c.pictures.ok_picked);
    let other = help_letters(&c.pictures.other);
    let buttons_of = |buttons: &[(String, nettai_assets::ButtonPictures)], say: &mut dyn FnMut(String)| -> Vec<(String, Option<HelpLetters>)> {
        buttons.iter().map(|(name, b)| (name.clone(), button_letters(name, b, say))).collect()
    };
    let buttons = buttons_of(&c.buttons, say);
    let version = |v: &VersionPictures, say: &mut dyn FnMut(String)| VersionLetters {
        buttons: v.buttons.iter().map(|(name, b)| (name.clone(), button_letters(name, b, say))).collect(),
        ..form_name_letters(v, say)
    };
    let base = version(&c.versioned.base, say);
    let versions = c.versioned.versions.iter().map(|(name, v)| (name.clone(), version(v, say))).collect();
    PictureLetters { ok, ok_picked, other, buttons, versioned: Versioned { base, base_version: c.versioned.base_version.clone(), versions } }
}

/// A button's picture's lines, in its own palette; the same in its others
/// (a line's color by its index), or it is said.
fn button_letters(name: &str, b: &nettai_assets::ButtonPictures, say: &mut dyn FnMut(String)) -> Option<HelpLetters> {
    let found = help_letters(&b.picture);
    for (k, &palette) in b.palettes.iter().enumerate().skip(1) {
        if help_letters(&nettai_assets::Picture { palette, ..b.picture.clone() }) != found {
            say(format!("button {name:?}'s picture's lines differ in its palette {k}"));
        }
    }
    found
}

/// A chip window's lettered picture's lines ([`HelpLetters`]) in its
/// palette, its letters taken off (its panel's or its frame's color, by
/// color: its indices are its bit planes', several of them one color); none
/// for a picture with no lines, or one that isn't a panel with words: its
/// panel's and frame's colors less than two thirds of it (a button's art
/// with its name lettered over it: EXE6's TRASH CHUTE and Beast Out, EXE5's
/// soul and Arm Change, each under half; the panels are four fifths and
/// more).
pub fn help_letters(p: &nettai_assets::Picture) -> Option<HelpLetters> {
    if p.tiles.len() < CustomScreen::PICTURE_TILES {
        return None;
    }
    let picture = Picture::grid(&p.tiles, 0, CustomScreen::PICTURE_COLUMNS, CustomScreen::PICTURE_TILES / CustomScreen::PICTURE_COLUMNS);
    let palette = p.palette;
    let color = |v: u8| palette[v as usize & 15] & 0x7FFF;
    let frame = color(picture.at(0, 0)?);
    let mut count: std::collections::HashMap<u16, usize> = Default::default();
    picture.pixels.iter().for_each(|&v| *count.entry(color(v)).or_default() += 1);
    let (&panel, _) = count.iter().max_by_key(|(c, n)| (**n, **c))?;
    let ground = |v: u8| color(v) == frame || color(v) == panel;
    if 3 * picture.pixels.iter().filter(|&&v| ground(v)).count() < 2 * picture.pixels.len() {
        return None;
    }
    let lines = lines(&picture, ground, &palette);
    if lines.is_empty() {
        return None;
    }
    Some(HelpLetters { lines, blank: erased(&picture, ground).to_grid() })
}

/// A version's form list names' letters, by look, and the names with them
/// taken off ([`VersionLetters`]'s `form_names`, `form_names_blank`).
fn form_name_letters(v: &VersionPictures, say: &mut dyn FnMut(String)) -> VersionLetters {
    let (columns, tiles) = (CustomScreen::FORM_NAME_COLUMNS, CustomScreen::FORM_NAME_TILES);
    let rows = tiles / columns;
    let mut out = VersionLetters::default();
    for look in 0..v.form_names.len() / tiles {
        let picture = Picture::grid(&v.form_names, tiles * look, columns, rows);
        // (Where the letters are: the name's first look's, the same
        // letters, whose Cross's face draws no pixel of the outline's color,
        // as another look's grayed face does.)
        let number = if look >= CustomScreen::FORM_NAMES_UNDER_CURSOR { look - CustomScreen::FORM_NAMES_UNDER_CURSOR } else { look };
        let first_look = Picture::grid(&v.form_names, tiles * number, columns, rows);
        match form_name(&picture, &first_look) {
            Some((letters, blank)) => {
                out.form_names.push(Some(letters));
                out.form_names_blank.pixels.extend(blank.to_grid().pixels);
            }
            None => {
                say(format!("no letters found in form list name {look}"));
                out.form_names.push(None);
                out.form_names_blank.pixels.extend(picture.to_grid().pixels);
            }
        }
    }
    out
}

/// A form list name's letters in one look (`picture`), where they are in
/// its first look: their face's rows and columns, its color on each row,
/// the outline's; and the name with them taken off, each pixel the bar's
/// own color nearest it on its row (neither a letter's nor an outline's).
fn form_name(picture: &Picture, first_look: &Picture) -> Option<(FormNameLetters, Picture)> {
    let found = enclosed_letters(first_look).filter(|_| first_look.w == picture.w && first_look.h == picture.h)?;
    let w = picture.w;
    let is_face = |x: usize, y: usize| found.mask[y * w + x] && picture.pixels[y * w + x] != found.outline;
    let rows: Vec<usize> = (0..picture.h).filter(|&y| (0..w).any(|x| is_face(x, y))).collect();
    let columns: Vec<usize> = (0..w).filter(|&x| (0..picture.h).any(|y| is_face(x, y))).collect();
    let (row0, row1, col0, col1) = (*rows.first()?, *rows.last()?, *columns.first()?, *columns.last()?);
    // The face's color on each of its rows.
    let fill: Vec<u8> = (row0..=row1)
        .map(|y| {
            let mut seen = [0usize; 16];
            (0..w).filter(|&x| is_face(x, y)).for_each(|x| seen[picture.pixels[y * w + x] as usize & 15] += 1);
            (0..16).max_by_key(|&c| seen[c]).unwrap_or(0) as u8
        })
        .collect();
    let mut blank = picture.clone();
    for y in 0..picture.h {
        let row = &picture.pixels[y * w..(y + 1) * w];
        for x in (0..w).filter(|&x| found.mask[y * w + x]) {
            // (The bar's own pixels only: neither a letter's nor its edge's.)
            let left = (0..x).rev().find(|&i| !found.mask[y * w + i] && row[i] != found.outline).map(|i| (x - i, row[i]));
            let right = (x + 1..w).find(|&i| !found.mask[y * w + i] && row[i] != found.outline).map(|i| (i - x, row[i]));
            if let Some(n) = match (left, right) {
                (Some(l), Some(r)) => Some(if r.0 < l.0 { r.1 } else { l.1 }),
                (l, r) => l.or(r).map(|n| n.1),
            } {
                blank.pixels[y * w + x] = n;
            }
        }
    }
    let letters = FormNameLetters { columns: [col0 as u8, col1 as u8], rows: [row0 as u8, row1 as u8], fill, rim: found.outline };
    Some((letters, blank))
}

/// A picture with letters in it in one language, for its shared base
/// ([`shared_base`]): its file in the pack, its tiles, its tiles with its
/// letters taken off (as read: its blank), and the color its palette gives
/// index `v` at pixel `i` (in tile order).
pub struct Variant<'a> {
    pub file: String,
    pub tiles: &'a Tiles,
    pub blank: &'a Tiles,
    pub color: Box<dyn Fn(usize, u8) -> u16 + 'a>,
}

/// How the pack keeps one language's picture of a lettered picture: the
/// base all its languages share, the file that holds it (none: this
/// picture's own, the first language's), and which of its pixels are its
/// letters (`tiles::Layered`).
#[derive(Clone, Debug)]
pub struct LayeredPicture {
    pub base: Tiles,
    pub base_file: Option<String>,
    pub letters: Vec<bool>,
}

/// One base for a lettered picture's languages (the user: "i think the
/// blanks in en and ja should be the same anyway"), the first its own: each
/// language's letters (where its blank differs from it) taken off, every
/// other pixel the first's. Where the first has its letters, its blank's
/// pixel, or one that shows each language that has letters there as its
/// blank does, in its own palette, if one does; where only another language
/// has letters, that language's blank's. Each picture is then the base and
/// a layer of its own pixels where they differ (`tiles::export_layered`),
/// and the enhanced mode shows the base where its letters are, its own
/// pixels elsewhere: its blank, but where two languages' letters cross on a
/// pixel no one index shows as both blanks (those pixels, by language, are
/// counted: the second value). A language whose picture is another size
/// keeps a base of its own (its blank).
pub fn shared_base(variants: &[Variant]) -> (Vec<(String, LayeredPicture)>, Vec<(String, usize)>) {
    let Some(own) = variants.first() else { return (Vec::new(), Vec::new()) };
    let n = own.tiles.pixels.len();
    let alike = |v: &Variant| v.tiles.pixels.len() == n && v.blank.pixels.len() == n;
    let letters = |v: &Variant| -> Vec<bool> {
        if v.blank.pixels.len() != v.tiles.pixels.len() {
            return vec![false; v.tiles.pixels.len()];
        }
        v.tiles.pixels.iter().zip(&v.blank.pixels).map(|(a, b)| a != b).collect()
    };
    let masks: Vec<Vec<bool>> = variants.iter().map(letters).collect();
    let shared: Vec<usize> = (0..variants.len()).filter(|&k| k == 0 || alike(&variants[k])).collect();
    let mut base = Tiles { pixels: own.tiles.pixels.clone() };
    let mut crossed = vec![0usize; variants.len()];
    for i in 0..n {
        let with: Vec<usize> = shared.iter().copied().filter(|&k| masks[k][i]).collect();
        let Some(&first) = with.first() else { continue };
        // (An index that shows as each one's blank, in each one's palette.)
        let fits = |v: u8| with.iter().all(|&k| (variants[k].color)(i, v) == (variants[k].color)(i, variants[k].blank.pixels[i]));
        let candidates = with.iter().map(|&k| variants[k].blank.pixels[i]).chain(0..16);
        match candidates.into_iter().find(|&v| fits(v)) {
            Some(v) => base.pixels[i] = v,
            None => {
                base.pixels[i] = variants[first].blank.pixels[i];
                for &k in &with[1..] {
                    crossed[k] += 1;
                }
            }
        }
    }
    let mut out = Vec::new();
    for (k, v) in variants.iter().enumerate() {
        let picture = if shared.contains(&k) {
            LayeredPicture { base: base.clone(), base_file: (k > 0).then(|| own.file.clone()), letters: masks[k].clone() }
        } else {
            LayeredPicture { base: v.blank.clone(), base_file: None, letters: masks[k].clone() }
        };
        out.push((v.file.clone(), picture));
    }
    let crossed = variants.iter().zip(crossed).filter(|(_, c)| *c > 0).map(|(v, c)| (v.file.clone(), c)).collect();
    (out, crossed)
}

/// The pictures with letters in them, by file, as the pack keeps them
/// (`letters::shared_base`): the window's (its labels), the codes', FINAL
/// TURN's, and with a base each one's languages share, the chip window's
/// lettered pictures (OK's, the buttons', each version's) and the form
/// list's names (each version's).
pub fn custom_layered(c: &CustomScreen) -> (BTreeMap<String, LayeredPicture>, Vec<(String, usize)>) {
    let l = &c.letters;
    let vs = &c.versioned;
    let mut out = BTreeMap::new();
    let mut crossed = Vec::new();
    let mut add = |variants: Vec<Variant>| {
        let (layered, c) = shared_base(&variants);
        out.extend(layered);
        crossed.extend(c);
    };
    let flat = |p: Palette| Box::new(move |_: usize, v: u8| p[v as usize & 15]) as Box<dyn Fn(usize, u8) -> u16>;
    fn single<'a>(file: &str, tiles: &'a Tiles, blank: &'a Tiles) -> Vec<Variant<'a>> {
        vec![Variant { file: file.into(), tiles, blank, color: Box::new(|_, v| v as u16) }]
    }
    // (Each alone: its colors don't matter, one language.)
    let mut singles: Vec<Vec<Variant>> = vec![
        single("window.png", &c.window_tiles, &l.window_blank),
        single("codes.png", &c.codes, &l.codes.blank),
        single("slot-codes.png", &c.slot_codes, &l.slot_codes.blank),
    ];
    if let Some(t) = &l.turn_limit {
        singles.push(single("turn-limit.png", &c.turn_limit, &t.blank));
    }
    for v in singles {
        add(v);
    }
    let blank = |h: &'_ Option<nettai_assets::letters::HelpLetters>, tiles: &'_ Tiles| -> Tiles { h.as_ref().map_or_else(|| tiles.clone(), |h| h.blank.clone()) };
    let mut kept: Vec<Tiles> = Vec::new();
    // The chip window's pictures for OK, in each language.
    type Pick = fn(&nettai_assets::SlotPictures) -> &nettai_assets::Picture;
    type Letters = fn(&nettai_assets::letters::PictureLetters) -> &Option<nettai_assets::letters::HelpLetters>;
    let slots: [(&str, Pick, Letters); 3] = [
        ("ok", |p| &p.ok, |l| &l.ok),
        ("ok-picked", |p| &p.ok_picked, |l| &l.ok_picked),
        ("other", |p| &p.other, |l| &l.other),
    ];
    for (name, pick, letters) in slots {
        kept.clear();
        kept.push(blank(letters(&l.pictures), &pick(&c.pictures).tiles));
        for (_, lang) in &c.languages {
            kept.push(blank(letters(&lang.letters), &pick(&lang.pictures).tiles));
        }
        let mut variants = vec![Variant { file: format!("pictures/{name}.png"), tiles: &pick(&c.pictures).tiles, blank: &kept[0], color: flat(pick(&c.pictures).palette) }];
        for (k, (lang, lettering)) in c.languages.iter().enumerate() {
            let p = pick(&lettering.pictures);
            variants.push(Variant { file: format!("pictures/{name}-{lang}.png"), tiles: &p.tiles, blank: &kept[k + 1], color: flat(p.palette) });
        }
        add(variants);
    }
    // The buttons' pictures: the pack's (and a language's own), each
    // version's own.
    for (name, b) in &c.buttons {
        let own = blank(&l.pictures.buttons.iter().find(|(n, _)| n == name).and_then(|(_, h)| h.clone()), &b.picture.tiles);
        let langs: Vec<(String, nettai_assets::Picture, Tiles)> = c
            .languages
            .iter()
            .filter_map(|(lang, lettering)| {
                let (_, own) = lettering.buttons.iter().find(|(n, _)| n == name)?;
                let p = own.picture.clone()?;
                let h = lettering.letters.buttons.iter().find(|(n, _)| n == name).and_then(|(_, h)| h.clone());
                let blank = blank(&h, &p.tiles);
                Some((format!("pictures/{name}-{lang}.png"), p, blank))
            })
            .collect();
        let mut variants = vec![Variant { file: format!("pictures/{name}.png"), tiles: &b.picture.tiles, blank: &own, color: flat(b.picture.palette) }];
        for (file, p, blank) in &langs {
            variants.push(Variant { file: file.clone(), tiles: &p.tiles, blank, color: flat(p.palette) });
        }
        add(variants);
    }
    for (which, v) in std::iter::once((None, &vs.base)).chain(vs.versions.iter().map(|(n, v)| (Some(n.as_str()), v))) {
        let letters = l.pictures.versioned.get(which.unwrap_or(&vs.base_version));
        let letters = if which.is_none() { &l.pictures.versioned.base } else { letters };
        for (name, b) in &v.buttons {
            let own = blank(&letters.buttons.iter().find(|(n, _)| n == name).and_then(|(_, h)| h.clone()), &b.picture.tiles);
            add(vec![Variant { file: format!("pictures/{}.png", vs.name(name, which)), tiles: &b.picture.tiles, blank: &own, color: flat(b.picture.palette) }]);
        }
        // The form list's names, in each language: each look in its own
        // palette.
        let looks = v.form_names.len() / CustomScreen::FORM_NAME_TILES;
        let palettes = v.form_name_palettes.clone();
        let color = move || {
            let palettes = palettes.clone();
            Box::new(move |i: usize, c: u8| {
                let row = (i / (CustomScreen::FORM_NAME_TILES * Tiles::TILE)).min(looks.saturating_sub(1));
                palettes.get(row).map_or(c as u16, |p| p[c as usize & 15])
            }) as Box<dyn Fn(usize, u8) -> u16>
        };
        let version = which.unwrap_or(&vs.base_version).to_string();
        let mut variants = vec![Variant { file: format!("{}.png", vs.name("form-names", which)), tiles: &v.form_names, blank: &letters.form_names_blank, color: color() }];
        for (lang, lettering) in &c.languages {
            let Some((_, names)) = lettering.form_names.iter().find(|(n, _)| *n == version) else { continue };
            let theirs = lettering.letters.versioned.get(&version);
            variants.push(Variant { file: format!("{}-{lang}.png", vs.name("form-names", Some(&version))), tiles: names, blank: &theirs.form_names_blank, color: color() });
        }
        add(variants);
    }
    (out, crossed)
}

/// The HUD's pictures with letters in them, by file, as the pack keeps
/// them (`shared_base`): the gauge's (its words), its languages sharing
/// its base, and the counts' (their digits); with the pixels where two
/// languages' letters cross on no shared index, by file.
pub fn hud_layered(h: &Hud) -> (BTreeMap<String, LayeredPicture>, Vec<(String, usize)>) {
    let mut out = BTreeMap::new();
    let mut crossed = Vec::new();
    let gauge_bases: Vec<(String, &Tiles, Tiles)> = std::iter::once(("gauge.png".to_string(), &h.gauge_tiles, &h.letters.words))
        .chain(h.languages.iter().map(|(lang, l)| (format!("gauge-{lang}.png"), &l.gauge_tiles, &l.letters)))
        .map(|(file, tiles, words)| (file, tiles, gauge_base(tiles, h.gauge_first_tile, &h.gauge_frame, words)))
        .collect();
    let gauge_palette = h.gauge_palette;
    let variants = gauge_bases
        .iter()
        .map(|(file, tiles, blank)| Variant { file: file.clone(), tiles, blank, color: Box::new(move |_, v| gauge_palette[v as usize & 15]) })
        .collect::<Vec<_>>();
    let (layered, c) = shared_base(&variants);
    out.extend(layered);
    crossed.extend(c);
    let mut counts: Vec<&Tiles> = h.counts.iter().collect();
    counts.push(&h.count_box);
    let counts = Tiles { pixels: counts.iter().flat_map(|t| t.pixels.iter().copied()).collect() };
    let mut base = counts_base(&h.counts, &h.letters.counts);
    base.push(h.count_box.clone());
    let base = Tiles { pixels: base.iter().flat_map(|t| t.pixels.iter().copied()).collect() };
    let variants = vec![Variant { file: "counts.png".into(), tiles: &counts, blank: &base, color: Box::new(|_, v| v as u16) }];
    out.extend(shared_base(&variants).0);
    (out, crossed)
}

/// A picture as the enhanced mode shows it, kept as `l` says
/// ([`LayeredPicture`]): its base where its letters are, its own pixels
/// elsewhere.
pub fn shown(tiles: &Tiles, l: &LayeredPicture) -> Tiles {
    let mut out = l.base.clone();
    if out.pixels.len() != tiles.pixels.len() {
        return tiles.clone();
    }
    for (i, &v) in tiles.pixels.iter().enumerate() {
        if !l.letters.get(i).copied().unwrap_or(false) {
            out.pixels[i] = v;
        }
    }
    out
}

/// The pictures' blanks as the pack keeps them (`custom_layered`,
/// `hud_layered`: one base a picture for its languages), so that a bundle
/// read is the bundle written: each one the picture as the enhanced mode
/// shows it (`shown`). What it changes of a blank (the pixels where two
/// languages' letters cross and no index shows as both blanks), by
/// picture.
pub fn settle(bundle: &mut Bundle) -> Vec<String> {
    let mut notes = Vec::new();
    let mut say = |crossed: Vec<(String, usize)>| {
        notes.extend(crossed.into_iter().map(|(file, n)| format!("{file}: {n} pixels where its letters cross another language's show the shared base")));
    };
    // The gauge's words, in each language.
    let (map, crossed) = hud_layered(&bundle.hud);
    say(crossed);
    let h = &mut bundle.hud;
    let (first, frame) = (h.gauge_first_tile, h.gauge_frame.clone());
    if let Some(l) = map.get("gauge.png") {
        let s = shown(&h.gauge_tiles, l);
        h.letters.words = settled_words(&h.letters.words, &s, first, &frame);
    }
    for (lang, lettering) in &mut h.languages {
        if let Some(l) = map.get(&format!("gauge-{lang}.png")) {
            let s = shown(&lettering.gauge_tiles, l);
            lettering.letters = settled_words(&lettering.letters, &s, first, &frame);
        }
    }
    if bundle.custom.is_empty() {
        return notes;
    }
    let (map, crossed) = custom_layered(&bundle.custom);
    say(crossed);
    let c = &mut bundle.custom;
    let set = |h: &mut Option<HelpLetters>, tiles: &Tiles, file: &str| {
        if let (Some(h), Some(l)) = (h.as_mut(), map.get(file)) {
            h.blank = shown(tiles, l);
        }
    };
    set(&mut c.letters.pictures.ok, &c.pictures.ok.tiles, "pictures/ok.png");
    set(&mut c.letters.pictures.ok_picked, &c.pictures.ok_picked.tiles, "pictures/ok-picked.png");
    set(&mut c.letters.pictures.other, &c.pictures.other.tiles, "pictures/other.png");
    for (name, h) in &mut c.letters.pictures.buttons {
        if let Some((_, b)) = c.buttons.iter().find(|(n, _)| n == name) {
            set(h, &b.picture.tiles, &format!("pictures/{name}.png"));
        }
    }
    let vs = c.versioned.clone();
    let names = |which: Option<&str>| format!("{}.png", vs.name("form-names", which));
    let own_names: Vec<(String, Tiles)> = std::iter::once((vs.base_version.clone(), &vs.base))
        .chain(vs.versions.iter().map(|(n, v)| (n.clone(), v)))
        .map(|(version, v)| {
            let which = (version != vs.base_version || !vs.versions.is_empty()).then_some(version.as_str());
            let file = if version == vs.base_version { names(None) } else { names(which) };
            (version.clone(), map.get(&file).map_or_else(|| v.form_names.clone(), |l| shown(&v.form_names, l)))
        })
        .collect();
    for (version, blank) in &own_names {
        let v = if *version == c.letters.pictures.versioned.base_version {
            &mut c.letters.pictures.versioned.base
        } else {
            match c.letters.pictures.versioned.versions.iter_mut().find(|(n, _)| n == version) {
                Some((_, v)) => v,
                None => continue,
            }
        };
        if map.contains_key(&names(Some(version))) || map.contains_key(&names(None)) {
            v.form_names_blank = blank.clone();
        }
    }
    let own_buttons = c.letters.pictures.buttons.clone();
    for (lang, lettering) in &mut c.languages {
        let p = &mut lettering.letters;
        set(&mut p.ok, &lettering.pictures.ok.tiles, &format!("pictures/ok-{lang}.png"));
        set(&mut p.ok_picked, &lettering.pictures.ok_picked.tiles, &format!("pictures/ok-picked-{lang}.png"));
        set(&mut p.other, &lettering.pictures.other.tiles, &format!("pictures/other-{lang}.png"));
        for (name, h) in &mut p.buttons {
            match lettering.buttons.iter().find(|(n, _)| n == name).and_then(|(_, b)| b.picture.as_ref()) {
                // (Its own picture: its layer over the shared base.)
                Some(picture) => set(h, &picture.tiles, &format!("pictures/{name}-{lang}.png")),
                // (The pack's picture: as the pack's shows.)
                None => {
                    if let (Some(h), Some((_, Some(own)))) = (h.as_mut(), own_buttons.iter().find(|(n, _)| n == name)) {
                        h.blank = own.blank.clone();
                    }
                }
            }
        }
        for (version, own_blank) in &own_names {
            let v = if *version == p.versioned.base_version {
                &mut p.versioned.base
            } else {
                match p.versioned.versions.iter_mut().find(|(n, _)| n == version) {
                    Some((_, v)) => v,
                    None => continue,
                }
            };
            let file = format!("{}-{lang}.png", vs.name("form-names", Some(version)));
            match (lettering.form_names.iter().find(|(n, _)| n == version), map.get(&file)) {
                (Some((_, tiles)), Some(l)) => v.form_names_blank = shown(tiles, l),
                (None, _) => v.form_names_blank = own_blank.clone(),
                _ => {}
            }
        }
    }
    notes
}

/// A language's HUD words with their blanks read off the gauge as the
/// enhanced mode shows it (`shown`).
fn settled_words(w: &HudWords, gauge: &Tiles, first: u16, frame: &[MapEntry]) -> HudWords {
    let doc = words_doc(w);
    words_from_doc(&doc, gauge, first, frame)
}

/// How the pack writes the letters (`hud.json`'s and `custom.json`'s
/// `letters`, and each language's): the plain values as they are. The
/// pictures with their letters taken off are no files of their own: a
/// lettered picture's image is that, with its letters in a layer of their
/// own beside it (`TileImage::letters`), and the blanks here are read off
/// those images ([`gauge_base`], [`counts_base`], `CustomBases`).
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct HudLettersDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp_digits: Option<Digits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_digits: Option<Digits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plus: Option<Digits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub times: Option<Digits>,
    pub enemy_digits: [Option<Digits>; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner_digits: Option<Digits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telop: Option<[Digits; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause: Option<Letters>,
    /// Each count's digits in the emotion window's box.
    pub counts: Vec<Option<CountDigits>>,
    pub words: HudWordsDoc,
}

/// `HudWords`.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct HudWordsDoc {
    pub banners: Vec<Option<BannerLetters>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting: Option<Letters>,
    pub l_or_r: [Option<Letters>; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gauge: Option<GaugeWordDoc>,
}

/// `GaugeWord`.
#[derive(Serialize, Deserialize, Debug)]
pub struct GaugeWordDoc {
    pub cells: Vec<u8>,
    pub letters: Letters,
    pub align: TextAlign,
}

/// `CustomLetters`.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CustomLettersDoc {
    pub codes: Vec<Option<Digits>>,
    pub slot_codes: Vec<Option<Digits>>,
    /// The boxes all the codes', and all the slots' codes', faces fill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codes_face: Option<[u8; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_codes_face: Option<[u8; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digits: Option<Digits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_limit: Option<Letters>,
    pub labels: Vec<Option<LabelLetters>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_name_box: Option<[u8; 2]>,
    pub pictures: PictureLettersDoc,
}

/// `PictureLetters`: each picture's lines; the versions' with the base's
/// first.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PictureLettersDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<Vec<HelpLine>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok_picked: Option<Vec<HelpLine>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other: Option<Vec<HelpLine>>,
    pub buttons: Vec<NamedLinesDoc>,
    pub versions: Vec<VersionLettersDoc>,
}

/// A button's picture's lines, by the button's name.
#[derive(Serialize, Deserialize, Debug)]
pub struct NamedLinesDoc {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<HelpLine>>,
}

/// `VersionLetters`, by its version's name.
#[derive(Serialize, Deserialize, Debug)]
pub struct VersionLettersDoc {
    pub version: String,
    pub buttons: Vec<NamedLinesDoc>,
    pub form_names: Vec<Option<FormNameLetters>>,
}

/// The HUD's letters as the pack writes them.
pub fn hud_doc(l: &HudLetters) -> HudLettersDoc {
    HudLettersDoc {
        hp_digits: l.hp_digits.clone(),
        damage_digits: l.damage_digits.clone(),
        plus: l.plus.clone(),
        times: l.times.clone(),
        enemy_digits: l.enemy_digits.clone(),
        banner_digits: l.banner_digits.clone(),
        telop: l.telop.clone(),
        pause: l.pause.clone(),
        counts: l.counts.counts.clone(),
        words: words_doc(&l.words),
    }
}

/// A language's HUD words as the pack writes them.
pub fn words_doc(w: &HudWords) -> HudWordsDoc {
    HudWordsDoc {
        banners: w.banners.clone(),
        waiting: w.waiting.clone(),
        l_or_r: [0, 1].map(|k| w.l_or_r[k].as_ref().map(|l| l.letters.clone())),
        gauge: w.gauge.as_ref().map(|g| GaugeWordDoc { cells: g.cells.clone(), letters: g.letters.clone(), align: g.align }),
    }
}

/// The gauge's tiles (`Hud::gauge_tiles`, from tile number `first`) with
/// the letters of its words taken off: the gauge's image in the pack.
pub fn gauge_base(gauge: &Tiles, first: u16, frame: &[MapEntry], w: &HudWords) -> Tiles {
    let mut base = gauge.clone();
    for (k, blank) in gauge_word_tiles(first, frame, w) {
        if let (Some(t), Some(range)) = (blank.get(k.1), base.pixels.get_mut(k.0 * Tiles::TILE..(k.0 + 1) * Tiles::TILE)) {
            range.copy_from_slice(t);
        }
    }
    base
}

/// Where the gauge's words' tiles are among the gauge's (each its index
/// there and its index in its word's blank tiles), with those blanks.
fn gauge_word_tiles<'w>(first: u16, frame: &[MapEntry], w: &'w HudWords) -> Vec<((usize, usize), &'w Tiles)> {
    let mut out = Vec::new();
    for (f, l) in w.l_or_r.iter().enumerate() {
        if let Some(l) = l {
            for i in 0..4 {
                out.push(((Hud::GAUGE_L_OR_R as usize + 4 * f + i, i), &l.blank));
            }
        }
    }
    if let Some(g) = &w.gauge {
        for (i, &c) in g.cells.iter().enumerate() {
            if let Some(e) = frame.get(c as usize)
                && let Some(k) = e.tile.checked_sub(first)
            {
                out.push(((k as usize, i), &g.blank));
            }
        }
    }
    out
}

/// A language's HUD words from the pack: their blanks from the gauge's
/// image (`gauge_base`, from tile number `first`).
pub fn words_from_doc(d: &HudWordsDoc, gauge_base: &Tiles, first: u16, frame: &[MapEntry]) -> HudWords {
    let tiles = |ks: &mut dyn Iterator<Item = usize>| Tiles { pixels: ks.flat_map(|k| gauge_base.get(k).unwrap_or(&[0; Tiles::TILE]).to_vec()).collect() };
    let l_or_r = [0usize, 1].map(|f| {
        d.l_or_r[f].as_ref().map(|letters| WordLetters { letters: letters.clone(), blank: tiles(&mut (0..4).map(|i| Hud::GAUGE_L_OR_R as usize + 4 * f + i)) })
    });
    let gauge = d.gauge.as_ref().map(|g| {
        let ks: Vec<usize> = g.cells.iter().filter_map(|&c| frame.get(c as usize)?.tile.checked_sub(first)).map(usize::from).collect();
        GaugeWord { cells: g.cells.clone(), blank: tiles(&mut ks.into_iter()), letters: g.letters.clone(), align: g.align }
    });
    HudWords { banners: d.banners.clone(), waiting: d.waiting.clone(), l_or_r, gauge }
}

/// The emotion window's counts with their digits taken off: the counts'
/// image in the pack (each a count's box with no digits where the count
/// has digits, the count's picture otherwise).
pub fn counts_base(counts: &[Tiles], l: &CountLetters) -> Vec<Tiles> {
    counts.iter().enumerate().map(|(n, t)| if l.counts.get(n).is_some_and(Option::is_some) { l.blank.clone() } else { t.clone() }).collect()
}

/// The HUD's letters from the pack: the gauge's words' blanks from the
/// gauge's image (`gauge_base`), the counts' blank box from the counts'
/// image (`counts_base`: a count with digits).
pub fn hud_from_doc(d: &HudLettersDoc, gauge_base: &Tiles, first: u16, frame: &[MapEntry], counts_base: &[Tiles]) -> HudLetters {
    let blank = d.counts.iter().position(Option::is_some).and_then(|n| counts_base.get(n)).cloned().unwrap_or_default();
    HudLetters {
        hp_digits: d.hp_digits.clone(),
        damage_digits: d.damage_digits.clone(),
        plus: d.plus.clone(),
        times: d.times.clone(),
        enemy_digits: d.enemy_digits.clone(),
        banner_digits: d.banner_digits.clone(),
        telop: d.telop.clone(),
        pause: d.pause.clone(),
        counts: CountLetters { blank, counts: d.counts.clone() },
        words: words_from_doc(&d.words, gauge_base, first, frame),
    }
}

/// The custom screen's letters as the pack writes them.
pub fn custom_doc(l: &CustomLetters) -> CustomLettersDoc {
    CustomLettersDoc {
        codes: l.codes.glyphs.clone(),
        slot_codes: l.slot_codes.glyphs.clone(),
        codes_face: l.codes.face,
        slot_codes_face: l.slot_codes.face,
        digits: l.digits.clone(),
        turn_limit: l.turn_limit.as_ref().map(|w| w.letters.clone()),
        labels: l.labels.clone(),
        form_name_box: l.form_name_box,
        pictures: pictures_doc(&l.pictures),
    }
}

/// A language's lettered pictures as the pack writes them.
pub fn pictures_doc(p: &PictureLetters) -> PictureLettersDoc {
    let lines = |h: &Option<HelpLetters>| h.as_ref().map(|h| h.lines.clone());
    let buttons = |b: &[(String, Option<HelpLetters>)]| b.iter().map(|(name, h)| NamedLinesDoc { name: name.clone(), lines: lines(h) }).collect();
    let version = |name: &str, v: &VersionLetters| VersionLettersDoc { version: name.to_string(), buttons: buttons(&v.buttons), form_names: v.form_names.clone() };
    let vs = &p.versioned;
    PictureLettersDoc {
        ok: lines(&p.ok),
        ok_picked: lines(&p.ok_picked),
        other: lines(&p.other),
        buttons: buttons(&p.buttons),
        versions: std::iter::once(version(&vs.base_version, &vs.base)).chain(vs.versions.iter().map(|(n, v)| version(n, v))).collect(),
    }
}

/// The custom screen's lettered images in the pack, their letters taken
/// off: what the letters' blanks are read from.
pub struct CustomBases<'a> {
    pub codes: &'a Tiles,
    pub slot_codes: &'a Tiles,
    pub window: &'a Tiles,
    pub turn_limit: &'a Tiles,
    pub pictures: PictureBases<'a>,
}

/// A language's lettered pictures' images, their letters taken off (its
/// own where it has them, else the pack's): the chip window's pictures,
/// the buttons' (by name), each version's buttons' and form list names.
pub struct PictureBases<'a> {
    pub ok: &'a Tiles,
    pub ok_picked: &'a Tiles,
    pub other: &'a Tiles,
    pub buttons: Vec<(&'a str, &'a Tiles)>,
    pub versions: Vec<(&'a str, Vec<(&'a str, &'a Tiles)>, &'a Tiles)>,
}

/// The custom screen's letters from the pack.
pub fn custom_from_doc(d: &CustomLettersDoc, b: &CustomBases) -> CustomLetters {
    CustomLetters {
        codes: GlyphLetters { face: d.codes_face, glyphs: d.codes.clone(), blank: b.codes.clone() },
        slot_codes: GlyphLetters { face: d.slot_codes_face, glyphs: d.slot_codes.clone(), blank: b.slot_codes.clone() },
        digits: d.digits.clone(),
        turn_limit: d.turn_limit.as_ref().map(|l| WordLetters { letters: l.clone(), blank: b.turn_limit.clone() }),
        labels: d.labels.clone(),
        window_blank: b.window.clone(),
        form_name_box: d.form_name_box,
        pictures: pictures_from_doc(&d.pictures, &b.pictures),
    }
}

/// A language's lettered pictures from the pack.
pub fn pictures_from_doc(d: &PictureLettersDoc, b: &PictureBases) -> PictureLetters {
    let help = |lines: &Option<Vec<HelpLine>>, base: Option<&Tiles>| {
        lines.as_ref().map(|lines| HelpLetters { lines: lines.clone(), blank: base.cloned().unwrap_or_default() })
    };
    fn find<'a>(list: &[(&str, &'a Tiles)], name: &str) -> Option<&'a Tiles> {
        list.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
    }
    let buttons = |doc: &[NamedLinesDoc], bases: &[(&str, &Tiles)]| -> Vec<(String, Option<HelpLetters>)> {
        doc.iter().map(|n| (n.name.clone(), help(&n.lines, find(bases, &n.name)))).collect()
    };
    let mut versions = d.versions.iter().map(|v| {
        let own = b.versions.iter().find(|(n, _, _)| *n == v.version);
        let letters = VersionLetters {
            buttons: buttons(&v.buttons, own.map_or(&[][..], |o| &o.1[..])),
            form_names: v.form_names.clone(),
            form_names_blank: own.map(|o| o.2.clone()).unwrap_or_default(),
        };
        (v.version.clone(), letters)
    });
    let (base_version, base) = versions.next().unwrap_or_default();
    PictureLetters {
        ok: help(&d.ok, Some(b.ok)),
        ok_picked: help(&d.ok_picked, Some(b.ok_picked)),
        other: help(&d.other, Some(b.other)),
        buttons: buttons(&d.buttons, &b.buttons),
        versioned: Versioned { base, base_version, versions: versions.collect() },
    }
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
        let (bar, colors) = bar(&p).expect("a bar");
        assert_eq!(colors.len(), 3, "{colors:?}");
        // Its body, right end, bottom line, top line and left end.
        assert_eq!(bar.iter().map(|s| s.color).collect::<Vec<_>>(), [4, 5, 5, 3, 3]);
        // The body: the left end slants to the picture's first column at the
        // bottom and stands upright from column 3 at the top (rows 3 to 8).
        let whole = &bar[0].points;
        assert!(whole.contains(&[24, 24]) && whole.contains(&[-4, 64]), "{whole:?}");
        let l = letters(&p, |v| v == 0 || colors.contains(&v), Capitals::First).expect("letters");
        assert_eq!((l.rim, l.shadow, l.top, l.cap), (Some(6), None, 2, 6));
        assert_eq!(l.fill, vec![8, 8, 7, 1, 1, 8]);
        assert_eq!(l.span, [8, 16]);
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
        assert!(bar(&p).is_none());
    }

    #[test]
    fn letters_erased_to_their_ground() {
        let p = picture(&["eee44eee", "e114411e", "e133311e"]);
        let ground = |v: u8| v == 1 || v == 14;
        assert_eq!(erased(&p, ground).pixels, picture(&["eeeeeeee", "e111111e", "e111111e"]).pixels);
    }

    /// A chip code's ground: its cell's most common color and a frame
    /// filling a side; a letter touching a side (the chip window's A fills
    /// its cell's width) stays a letter.
    #[test]
    fn a_codes_ground_is_its_cells_color_and_frame() {
        let slot = picture(&["38888883", "3888b883", "388b8b83", "38bbbbb3", "33888833", "33333333"]);
        assert_eq!(ground_of(&slot), vec![8, 3]);
        let window = picture(&["88888888", "88bbb888", "bbb8bbb8", "bbbbbbb8", "bb888bb8", "88888888"]);
        assert_eq!(ground_of(&window), vec![8]);
    }

    /// A Cross's name on its bar: letters outlined in the bar's edge color
    /// (d), apart from the bar's own edge, with what they enclose.
    #[test]
    fn a_names_enclosed_letters_are_found() {
        let p = picture(&[
            "dddddddddddd",
            "d9999999999d",
            "d99ddd9ddd9d",
            "d99d1d9d1d9d",
            "d99ddd9ddd9d",
            "d9999999999d",
            "dddddddddddd",
        ]);
        let found = enclosed_letters(&p).expect("letters");
        assert_eq!(found.outline, 13);
        let mask: Vec<usize> = (0..p.pixels.len()).filter(|&i| found.mask[i]).collect();
        let row = |y: usize, xs: &[usize]| xs.iter().map(|&x| y * 12 + x).collect::<Vec<_>>();
        let mut want = Vec::new();
        for y in 2..5 {
            want.extend(row(y, &[3, 4, 5, 7, 8, 9]));
        }
        assert_eq!(mask, want);
    }

    /// A letter whose outline touches the bar's edge is still a letter.
    #[test]
    fn a_letter_touching_the_edge_is_found() {
        let p = picture(&[
            "dddddddddddd",
            "d9999999999d",
            "d99ddd9ddddd",
            "d99d1d9d11dd",
            "d99ddd9ddddd",
            "d9999999999d",
            "dddddddddddd",
        ]);
        let found = enclosed_letters(&p).expect("letters");
        let mask: Vec<usize> = (0..p.pixels.len()).filter(|&i| found.mask[i]).collect();
        let mut want = Vec::new();
        for y in 2..5 {
            want.extend([3, 4, 5, 7, 8, 9, 10].map(|x| y * 12 + x));
        }
        assert_eq!(mask, want);
    }

    /// A set of two digits with a rim: its face box is the faces' columns
    /// and rows, in their cells; its paint the indices.
    #[test]
    fn a_digit_sets_face_box_and_paint() {
        // Two glyphs: a face of color 1 in columns 1 to 6, rows 4 to 11, a
        // rim of 6 around it.
        let mut tiles = Tiles { pixels: vec![0; 4 * Tiles::TILE] };
        for c in 0..2 {
            for y in 3..13 {
                for x in 0..8 {
                    let v = if (4..12).contains(&y) && (1..7).contains(&x) { 1 } else { 6 };
                    let (tile, row) = (2 * c + y / 8, y % 8);
                    tiles.pixels[tile * Tiles::TILE + row * 8 + x] = v;
                }
            }
        }
        let set = digits(&tiles, 0, 2, |v| v == 0).expect("a set");
        assert_eq!(set.face, [1, 4, 6, 8]);
        assert_eq!((set.rim, set.shadow), (Some(6), None));
        assert!(set.fill.iter().all(|&c| c == 1));
    }

    /// Lines a clear row apart, at their pitch; a run two pitches tall
    /// before the next line (a voiced mark joining two lines) is two lines
    /// at that pitch, the lower from its pitch's row; a run far from the
    /// next by the title's gap stays one.
    #[test]
    fn lines_a_mark_joins_are_split_at_their_pitch() {
        // (EXE4's Japanese OK picture: rows 1-8, 10-17, 19-36 and 39-47.)
        let mut inked = vec![false; 48];
        for (a, b) in [(1, 9), (10, 18), (19, 37), (39, 48)] {
            inked[a..b].iter_mut().for_each(|r| *r = true);
        }
        assert_eq!(line_rows(&inked), [(1, 9), (10, 18), (19, 29), (29, 37), (39, 48)]);
        // (EXE6's English OK picture: a title of two lines, a gap, two
        // lines of words.)
        let mut inked = vec![false; 48];
        for (a, b) in [(2, 10), (11, 19), (23, 31), (32, 40)] {
            inked[a..b].iter_mut().for_each(|r| *r = true);
        }
        assert_eq!(line_rows(&inked), [(2, 10), (11, 19), (23, 31), (32, 40)]);
    }

    /// A chip code's letter in its cell: its face's box and paint, and the
    /// cell with it taken off.
    #[test]
    fn a_code_is_read_off_its_cell() {
        let p = picture(&["88888888", "88bbb888", "8bb8bb88", "8bbbbb88", "8bb8bb88", "88888888"]);
        let (look, blank) = cell_letter(&p).expect("a letter");
        assert_eq!(look.face, [1, 1, 5, 4]);
        assert_eq!(look.fill, vec![11; 4]);
        assert!(blank.pixels.iter().all(|&v| v == 8));
    }
}
