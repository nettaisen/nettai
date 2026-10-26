//! Signed distance fields of glyph outlines, for the text layer
//! (`vfont`): each glyph's field is made once, at [`RES`] texels an em
//! whatever the output's size, and drawn at any size by sampling it at
//! each output pixel (docs/design/text-rendering.md §9.10).
//!
//! The field is computed from the outline itself, not from a picture of
//! it: the outline's curves are cut into straight segments (much finer than
//! a texel), each texel's sign is whether its center is inside them (the
//! nonzero rule, row by row), and its distance is the least to any segment
//! of the outline's edge, computed for the texels within [`SPREAD`] of each
//! segment only. So the distances are exact to the segments, at a cost of
//! the outline's length times the spread. A variable font's contours
//! overlap (an e can be a bowl and a bar): where they do, the parts of
//! its segments inside the glyph are found (cut where they cross another
//! segment, each piece kept if it has the glyph on one side only) and left
//! out of the distances.

use std::cell::OnceCell;
use swash::zeno::{Vector, Verb};

/// Texels an em. The text's largest glyphs are 13.7 frame pixels an em
/// (the cell role's), 178 output pixels at 13x: a texel is 1.4 output
/// pixels there, 1 at 9x, half one at 5x.
pub const RES: f32 = 128.0;
/// How far from the outline the field holds distances, in texels (past
/// it, it holds this): more than half an output pixel from 2x up (a
/// texel is a fifth of an output pixel at 2x, the cell role's). At 1x the
/// antialiasing ramp narrows to fit (0.6 pixels wide where it is 1;
/// [`Sdf::draw`]).
pub const SPREAD: f32 = 3.0;
/// How finely the outline's curves are cut, in texels: the most a segment
/// strays from its curve.
const FLATNESS: f32 = 0.1;
/// The stored distances' unit: 256ths of a texel.
const UNIT: f32 = 256.0;

/// What a field is drawn into ([`Sdf::draw`]).
pub trait Paint {
    /// An output pixel, covered `a` 255ths (1 to 255).
    fn pixel(&mut self, x: i32, y: i32, a: u8);
    /// Output pixels `xa` to one before `xb` of row `y`, covered whole.
    fn span(&mut self, y: i32, xa: i32, xb: i32) {
        for x in xa..xb {
            self.pixel(x, y, 255);
        }
    }
}

impl<F: FnMut(i32, i32, u8)> Paint for F {
    fn pixel(&mut self, x: i32, y: i32, a: u8) {
        self(x, y, a)
    }
}

/// A straight piece of an outline: from, to.
type Segment = (Vector, Vector);

/// A glyph's signed distance field: a grid of texels over its outline and
/// [`SPREAD`] around it, in the outline's coordinates (texels, y up, the
/// glyph's origin at 0).
#[derive(Clone, Debug)]
pub struct Sdf {
    /// The grid's left and top edges: texel (i, j)'s center is at
    /// (`left + i + 0.5`, `top - j - 0.5`).
    pub left: f32,
    pub top: f32,
    pub w: usize,
    pub h: usize,
    /// How far from the outline it holds distances, in texels.
    pub spread: f32,
    /// Each texel's distance to the outline in [`UNIT`]s, negative inside,
    /// at most [`SPREAD`] either way; rows from the top.
    data: Vec<i16>,
    /// Where its texels are all deep inside or far outside the glyph, by
    /// how deep ([`DEEP`]), for drawing them a run at a time: each made
    /// the first time a size wants it.
    levels: [OnceCell<Runs>; DEEP.len()],
}

/// How far from the edge, in texels, a cell's texels all are for it to be
/// deep inside or far outside, at each level of [`Runs`]: a size draws by
/// the first that holds its antialiasing ramp (half an output pixel), the
/// first from 5x up (a texel half an output pixel or more), the second at
/// every size (the ramp is never wider than the spread).
const DEEP: [f32; 2] = [1.0, SPREAD];

/// Where between each two rows of texels (j and j + 1) the texels are all
/// deep inside the glyph or near its edge, by runs of the cells between
/// them (cell i is between texels i and i + 1); between runs they are all
/// far outside. Row j's runs are `runs[starts[j]..starts[j + 1]]`.
#[derive(Clone, Debug)]
struct Runs {
    runs: Vec<Run>,
    starts: Vec<u32>,
}

/// Cells, `start` to one past `end`: deep inside, or near the edge.
#[derive(Clone, Copy, Debug)]
struct Run {
    start: u16,
    end: u16,
    inside: bool,
}

impl Runs {
    /// A field's runs at a depth (in texels).
    fn of(data: &[i16], w: usize, h: usize, deep: f32) -> Runs {
        let limit = (deep * UNIT) as i16;
        let (mut runs, mut starts) = (Vec::new(), Vec::with_capacity(h));
        // Each cell's kind: 0 far outside, 1 deep inside, 2 near the edge.
        let mut kinds = vec![0u8; w - 1];
        for j in 0..h - 1 {
            starts.push(runs.len() as u32);
            let (upper, lower) = data[j * w..(j + 2) * w].split_at(w);
            let corners = upper[..w - 1].iter().zip(&upper[1..]).zip(lower[..w - 1].iter().zip(&lower[1..]));
            for (k, ((&a, &b), (&c, &d))) in kinds.iter_mut().zip(corners) {
                let (most, least) = (a.max(b).max(c.max(d)), a.min(b).min(c.min(d)));
                *k = 2 - (most <= -limit) as u8 - 2 * (least >= limit) as u8;
            }
            let mut i = 0;
            while i < w - 1 {
                let k = kinds[i];
                let end = kinds[i..].iter().position(|&n| n != k).map_or(w - 1, |n| i + n);
                if k != 0 {
                    runs.push(Run { start: i as u16, end: end as u16, inside: k == 1 });
                }
                i = end;
            }
        }
        starts.push(runs.len() as u32);
        Runs { runs, starts }
    }

    fn row(&self, j: usize) -> &[Run] {
        &self.runs[self.starts[j] as usize..self.starts[j + 1] as usize]
    }
}

impl Sdf {
    /// The field of an outline (`points` and `verbs` as swash gives them,
    /// scaled to [`RES`] texels an em); none for an outline with no area
    /// (a space).
    pub fn of(points: &[Vector], verbs: &[Verb]) -> Option<Sdf> {
        Sdf::of_spread(points, verbs, SPREAD)
    }

    /// [`Sdf::of`], holding distances `spread` texels from the outline (at
    /// least [`SPREAD`]): a field an outline is drawn from, as far out as
    /// the outline's width ([`Sdf::draw_offset`]).
    pub fn of_spread(points: &[Vector], verbs: &[Verb], spread: f32) -> Option<Sdf> {
        let spread = spread.max(SPREAD);
        let segments = flatten(points, verbs);
        if segments.is_empty() {
            return None;
        }
        let (mut lo, mut hi) = (Vector::new(f32::INFINITY, f32::INFINITY), Vector::new(f32::NEG_INFINITY, f32::NEG_INFINITY));
        for &(p, _) in &segments {
            lo = Vector::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Vector::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        if hi.x <= lo.x || hi.y <= lo.y {
            return None;
        }
        let left = lo.x.floor() - spread;
        let top = hi.y.ceil() + spread;
        let w = (hi.x.ceil() + spread - left) as usize;
        let h = (top - (lo.y.floor() - spread)) as usize;
        let bands = Bands::new(&segments, top, h);

        // Signs: where the outline crosses each row's line through its
        // texels' centers, and which way; inside where the winding isn't
        // 0. A crossing from inside to inside is a segment within the
        // glyph: the contours overlap.
        let mut inside: Vec<(usize, usize, usize)> = Vec::new();
        let mut overlap = false;
        let mut crossings: Vec<(f32, i32)> = Vec::new();
        for (j, row) in bands.rows.iter().enumerate() {
            let y = top - j as f32 - 0.5;
            crossings.clear();
            for &k in row {
                let (p, q) = segments[k as usize];
                if (p.y <= y && y < q.y) || (q.y <= y && y < p.y) {
                    crossings.push((p.x + (y - p.y) * (q.x - p.x) / (q.y - p.y), if q.y > p.y { 1 } else { -1 }));
                }
            }
            crossings.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
            let (mut winding, mut from) = (0, 0.0);
            for &(x, dir) in &crossings {
                let after = winding + dir;
                if winding == 0 && after != 0 {
                    from = x;
                } else if winding != 0 && after == 0 {
                    let first = (from - left - 0.5).ceil().max(0.0) as usize;
                    let end = ((x - left - 0.5).ceil().max(0.0) as usize).min(w);
                    if first < end {
                        inside.push((j, first, end));
                    }
                } else {
                    overlap = true;
                }
                winding = after;
            }
        }
        let edge = if overlap { edge(&segments, &bands) } else { segments };

        // Distances: each segment's to the texels within the spread of it,
        // row by row (a row's are those within the spread of the part of
        // the segment within the spread of the row).
        let mut near = vec![spread * spread; w * h];
        for &(p, q) in &edge {
            let d = Vector::new(q.x - p.x, q.y - p.y);
            let len2 = d.x * d.x + d.y * d.y;
            let inv = if len2 > 0.0 { 1.0 / len2 } else { 0.0 };
            let (dxi, idy) = (d.x * inv, if d.y != 0.0 { 1.0 / d.y } else { 0.0 });
            let j0 = ((top - 0.5 - p.y.max(q.y) - spread).ceil().max(0.0)) as usize;
            let j1 = ((top - 0.5 - p.y.min(q.y) + spread).floor().max(0.0) as usize).min(h - 1);
            for j in j0..=j1 {
                let y = top - j as f32 - 0.5;
                let cy = y - p.y;
                let (xa, xb) = if d.y == 0.0 {
                    (p.x.min(q.x), p.x.max(q.x))
                } else {
                    let ta = ((cy - spread) * idy).clamp(0.0, 1.0);
                    let tb = ((cy + spread) * idy).clamp(0.0, 1.0);
                    let (xa, xb) = (p.x + ta * d.x, p.x + tb * d.x);
                    (xa.min(xb), xa.max(xb))
                };
                let i0 = ((xa - spread - left - 0.5).ceil().max(0.0)) as usize;
                let i1 = ((xb + spread - left - 0.5).floor().max(0.0) as usize).min(w - 1);
                if i0 > i1 {
                    continue;
                }
                let base = cy * d.y * inv;
                let cx0 = left + i0 as f32 + 0.5 - p.x;
                for (k, n) in near[j * w + i0..=j * w + i1].iter_mut().enumerate() {
                    let cx = cx0 + k as f32;
                    let t = (cx * dxi + base).clamp(0.0, 1.0);
                    let (ex, ey) = (cx - t * d.x, cy - t * d.y);
                    *n = n.min(ex * ex + ey * ey);
                }
            }
        }
        let mut data: Vec<i16> = near.iter().map(|&n| (n.sqrt() * UNIT).round() as i16).collect();
        for &(j, first, end) in &inside {
            for v in &mut data[j * w + first..j * w + end] {
                *v = -*v;
            }
        }
        Some(Sdf { left, top, w, h, spread, data, levels: Default::default() })
    }

    /// The bytes it takes.
    pub fn size(&self) -> usize {
        self.data.len() * 2 + std::mem::size_of::<Sdf>()
    }

    /// Draw it: `paint` each output pixel it covers, and each run of them
    /// it covers whole. The glyph's origin is at output (`ox`, `oy`) (y
    /// down), a texel `sx` output pixels wide and `sy` high; only the
    /// output pixels in `bounds` (left, top, right, bottom; exclusive) are
    /// drawn. Each pixel's center samples the field bilinearly; its
    /// distance in output pixels (across the field's gradient, so a
    /// narrowed or squashed glyph is antialiased as much in each
    /// direction) is ramped linearly over one output pixel, the coverage
    /// of a straight edge. Runs of texels deep inside the glyph are
    /// painted whole and those far outside skipped, unsampled.
    pub fn draw(&self, ox: f32, oy: f32, sx: f32, sy: f32, bounds: (i32, i32, i32, i32), paint: &mut impl Paint) {
        let (w, h) = (self.w, self.h);
        // The output pixels whose centers fall between the outermost
        // texels' centers.
        let x0 = ((ox + (self.left + 0.5) * sx - 0.5).ceil() as i32).max(bounds.0);
        let x1 = ((ox + (self.left + w as f32 - 0.5) * sx - 0.5).floor() as i32 + 1).min(bounds.2);
        let y0 = ((oy - (self.top - 0.5) * sy - 0.5).ceil() as i32).max(bounds.1);
        let y1 = ((oy - (self.top - h as f32 + 0.5) * sy - 0.5).floor() as i32 + 1).min(bounds.3);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        // Half the ramp, in output pixels: half an output pixel, or less
        // where the spread can't hold it.
        let fine = sx.min(sy);
        let half = 0.5f32.min(0.9 * SPREAD * fine);
        let ramp = 0.5 / half / UNIT;
        // Past this distance a pixel is wholly in or out (a texel of
        // distance is at least `fine` output pixels).
        let sure = half / fine * UNIT;
        // Neither narrowed nor squashed: a texel of distance is `sx` output
        // pixels whichever way.
        let even = (sx - sy).abs() <= 1e-4 * sx;
        let (isx, isy) = (1.0 / sx, 1.0 / sy);
        let (wl, hl) = ((w - 1) as f32, (h - 1) as f32);
        // The narrowest runs whose deep insides are wholly in and whose
        // cells between are wholly out at this size.
        let deep = [DEEP[0], self.spread];
        let l = (0..deep.len()).find(|&l| sure <= deep[l] * UNIT).unwrap_or(deep.len() - 1);
        let level = self.levels[l].get_or_init(|| Runs::of(&self.data, w, h, deep[l]));
        // The first pixel whose center is in cell `i` or after.
        let column = |i: u16| (ox + (self.left + 0.5 + i as f32) * sx - 0.5).ceil() as i32;
        for y in y0..y1 {
            let v = (self.top - (oy - (y as f32 + 0.5)) * isy - 0.5).clamp(0.0, hl);
            let j = (v as usize).min(h - 2);
            let fv = v - j as f32;
            let (upper, lower) = self.data[j * w..(j + 2) * w].split_at(w);
            for run in level.row(j) {
                let (xa, xb) = (column(run.start).max(x0), column(run.end).min(x1));
                if run.inside {
                    if xa < xb {
                        paint.span(y, xa, xb);
                    }
                    continue;
                }
                let mut u = (xa as f32 + 0.5 - ox) * isx - self.left - 0.5;
                for x in xa..xb {
                    let uc = u.clamp(0.0, wl);
                    u += isx;
                    let i = (uc as usize).min(w - 2);
                    let fu = uc - i as f32;
                    let (a, b, c, d) = (upper[i] as f32, upper[i + 1] as f32, lower[i] as f32, lower[i + 1] as f32);
                    let top = a + (b - a) * fu;
                    let dist = top + (c + (d - c) * fu - top) * fv;
                    if dist >= sure {
                        continue;
                    }
                    if dist <= -sure {
                        paint.pixel(x, y, 255);
                        continue;
                    }
                    // The output pixels a texel of distance is across the
                    // field's gradient.
                    let px = if even {
                        sx
                    } else {
                        let gu = (b - a) + ((d - c) - (b - a)) * fv;
                        let gv = (c - a) + ((d - b) - (c - a)) * fu;
                        let g2 = gu * gu + gv * gv;
                        if g2 > 1.0 { (g2 / ((gu * isx).powi(2) + (gv * isy).powi(2))).sqrt() } else { fine }
                    };
                    let k = (0.5 - dist * px * ramp).clamp(0.0, 1.0);
                    // (Down: an edge on a pixel's edge, off by the field's
                    // rounding, leaves the pixel past it unlit.)
                    let k = (k * 255.0) as u8;
                    if k > 0 {
                        paint.pixel(x, y, k);
                    }
                }
            }
        }
    }
}

impl Sdf {
    /// Draw it grown by `offset` output pixels all round (an outline as wide,
    /// even on every side, the glyph's own shape out that far: the field
    /// must hold distances that far, [`Sdf::of_spread`]), as
    /// [`Sdf::draw`] draws it: each pixel's center sampled, its distance in
    /// output pixels across the field's gradient, ramped over one pixel.
    pub fn draw_offset(&self, ox: f32, oy: f32, sx: f32, sy: f32, offset: f32, bounds: (i32, i32, i32, i32), paint: &mut impl Paint) {
        let (w, h) = (self.w, self.h);
        let x0 = ((ox + (self.left + 0.5) * sx - 0.5).ceil() as i32).max(bounds.0);
        let x1 = ((ox + (self.left + w as f32 - 0.5) * sx - 0.5).floor() as i32 + 1).min(bounds.2);
        let y0 = ((oy - (self.top - 0.5) * sy - 0.5).ceil() as i32).max(bounds.1);
        let y1 = ((oy - (self.top - h as f32 + 0.5) * sy - 0.5).floor() as i32 + 1).min(bounds.3);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let fine = sx.min(sy);
        let (isx, isy) = (1.0 / sx, 1.0 / sy);
        let (wl, hl) = ((w - 1) as f32, (h - 1) as f32);
        for y in y0..y1 {
            let v = (self.top - (oy - (y as f32 + 0.5)) * isy - 0.5).clamp(0.0, hl);
            let j = (v as usize).min(h - 2);
            let fv = v - j as f32;
            let (upper, lower) = self.data[j * w..(j + 2) * w].split_at(w);
            for x in x0..x1 {
                let u = ((x as f32 + 0.5 - ox) * isx - self.left - 0.5).clamp(0.0, wl);
                let i = (u as usize).min(w - 2);
                let fu = u - i as f32;
                let (a, b, c, d) = (upper[i] as f32, upper[i + 1] as f32, lower[i] as f32, lower[i + 1] as f32);
                let top = a + (b - a) * fu;
                let dist = (top + (c + (d - c) * fu - top) * fv) / UNIT;
                // The output pixels a texel of distance is across the
                // field's gradient.
                let gu = (b - a) + ((d - c) - (b - a)) * fv;
                let gv = (c - a) + ((d - b) - (c - a)) * fu;
                let g2 = gu * gu + gv * gv;
                let px = if g2 > 1.0 { (g2 / ((gu * isx).powi(2) + (gv * isy).powi(2))).sqrt() } else { fine };
                let k = (0.5 - (dist * px - offset)).clamp(0.0, 1.0);
                let k = (k * 255.0) as u8;
                if k > 0 {
                    paint.pixel(x, y, k);
                }
            }
        }
    }
}

#[cfg(test)]
impl Sdf {
    pub fn value(&self, i: usize, j: usize) -> i16 {
        self.data[j * self.w + i]
    }

    /// Texels whose distance differs from a neighbor's by more than the
    /// texel between them (and a little).
    pub fn jumps(&self) -> Vec<(usize, usize, i16)> {
        let mut out = Vec::new();
        for j in 1..self.h - 1 {
            for i in 1..self.w - 1 {
                let v = self.value(i, j);
                let n = [self.value(i - 1, j), self.value(i + 1, j), self.value(i, j - 1), self.value(i, j + 1)];
                if n.iter().any(|&m| (m as i32 - v as i32).abs() > (UNIT * 1.05) as i32) {
                    out.push((i, j, v));
                }
            }
        }
        out
    }
}

/// An outline's segments by the grid's rows they reach (row j's band is
/// `top - j - 1` to `top - j`), for the windings.
struct Bands {
    top: f32,
    rows: Vec<Vec<u32>>,
}

impl Bands {
    fn new(segments: &[Segment], top: f32, h: usize) -> Bands {
        let mut rows = vec![Vec::new(); h];
        for (k, &(p, q)) in segments.iter().enumerate() {
            let j0 = (top - p.y.max(q.y)).floor().max(0.0) as usize;
            let j1 = ((top - p.y.min(q.y)).floor().max(0.0) as usize).min(h - 1);
            for row in &mut rows[j0..=j1] {
                row.push(k as u32);
            }
        }
        Bands { top, rows }
    }

    /// The segments that reach the row band `y` is in.
    fn at(&self, y: f32) -> &[u32] {
        let j = (self.top - y).floor();
        if j < 0.0 || j as usize >= self.rows.len() { &[] } else { &self.rows[j as usize] }
    }

    /// The outline's winding number at a point: the crossings of the ray
    /// from it to the right, each 1 or -1 by its direction.
    fn winding(&self, segments: &[Segment], x: f32, y: f32) -> i32 {
        let mut winding = 0;
        for &k in self.at(y) {
            let (p, q) = segments[k as usize];
            if ((p.y <= y && y < q.y) || (q.y <= y && y < p.y)) && p.x + (y - p.y) * (q.x - p.x) / (q.y - p.y) > x {
                winding += if q.y > p.y { 1 } else { -1 };
            }
        }
        winding
    }
}

/// The parts of overlapping contours' segments that are the glyph's edge:
/// each segment cut where another crosses it, each piece kept if the glyph
/// is on one side of it only (its middle's windings a 64th of a texel to
/// either side, one 0 and one not).
fn edge(segments: &[Segment], bands: &Bands) -> Vec<Segment> {
    let cross = |a: Vector, b: Vector| a.x * b.y - a.y * b.x;
    // Where each segment is crossed, along it (segment, place): every
    // pair whose heights overlap tried once, swept up from the lowest.
    let mut cuts: Vec<(u32, f32)> = Vec::new();
    let span = |&(p, q): &Segment| (p.x.min(q.x), p.y.min(q.y), p.x.max(q.x), p.y.max(q.y));
    let boxes: Vec<(f32, f32, f32, f32)> = segments.iter().map(span).collect();
    let mut order: Vec<usize> = (0..segments.len()).collect();
    order.sort_unstable_by(|&a, &b| boxes[a].1.total_cmp(&boxes[b].1));
    for (n, &k) in order.iter().enumerate() {
        let bk = boxes[k];
        let (p, q) = segments[k];
        let d = Vector::new(q.x - p.x, q.y - p.y);
        for &o in &order[n + 1..] {
            let bo = boxes[o];
            if bo.1 > bk.3 {
                break;
            }
            if bo.0 > bk.2 || bo.2 < bk.0 {
                continue;
            }
            let (a, b) = segments[o];
            let e = Vector::new(b.x - a.x, b.y - a.y);
            let denom = cross(d, e);
            if denom.abs() < 1e-9 {
                continue;
            }
            let ap = Vector::new(a.x - p.x, a.y - p.y);
            let (t, u) = (cross(ap, e) / denom, cross(ap, d) / denom);
            let (inner, on) = (|t: f32| t > 1e-4 && t < 1.0 - 1e-4, |t: f32| (0.0..=1.0).contains(&t));
            if inner(t) && on(u) {
                cuts.push((k as u32, t));
            }
            if inner(u) && on(t) {
                cuts.push((o as u32, u));
            }
        }
    }
    cuts.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut places: Vec<f32> = Vec::new();
    let mut next = 0;
    let mut out = Vec::new();
    for (k, &(p, q)) in segments.iter().enumerate() {
        let d = Vector::new(q.x - p.x, q.y - p.y);
        places.clear();
        places.push(0.0);
        while next < cuts.len() && cuts[next].0 == k as u32 {
            places.push(cuts[next].1);
            next += 1;
        }
        places.push(1.0);
        let len = (d.x * d.x + d.y * d.y).sqrt();
        let n = Vector::new(-d.y / len / 64.0, d.x / len / 64.0);
        let mut run: Option<f32> = None;
        for pair in places.windows(2) {
            let (t0, t1) = (pair[0], pair[1]);
            if t1 - t0 < 1e-6 {
                continue;
            }
            let t = (t0 + t1) / 2.0;
            let m = Vector::new(p.x + t * d.x, p.y + t * d.y);
            let one = bands.winding(segments, m.x + n.x, m.y + n.y) != 0;
            let other = bands.winding(segments, m.x - n.x, m.y - n.y) != 0;
            match (one != other, run) {
                (true, None) => run = Some(t0),
                (false, Some(from)) => {
                    out.push((Vector::new(p.x + from * d.x, p.y + from * d.y), Vector::new(p.x + t0 * d.x, p.y + t0 * d.y)));
                    run = None;
                }
                _ => {}
            }
        }
        if let Some(from) = run {
            out.push((Vector::new(p.x + from * d.x, p.y + from * d.y), q));
        }
    }
    out
}


/// An outline's contours as straight segments, each closed.
fn flatten(points: &[Vector], verbs: &[Verb]) -> Vec<(Vector, Vector)> {
    let mut out = Vec::new();
    let (mut at, mut start, mut pen) = (0, Vector::ZERO, Vector::ZERO);
    let mut open = false;
    let line = |out: &mut Vec<(Vector, Vector)>, p: Vector, q: Vector| {
        if p != q {
            out.push((p, q));
        }
    };
    for verb in verbs {
        match verb {
            Verb::MoveTo => {
                if open {
                    line(&mut out, pen, start);
                }
                start = points[at];
                pen = start;
                at += 1;
                open = true;
            }
            Verb::LineTo => {
                line(&mut out, pen, points[at]);
                pen = points[at];
                at += 1;
            }
            Verb::QuadTo => {
                let (c, q) = (points[at], points[at + 1]);
                let dd = Vector::new(pen.x - 2.0 * c.x + q.x, pen.y - 2.0 * c.y + q.y);
                let n = pieces((dd.x * dd.x + dd.y * dd.y).sqrt() / 4.0);
                let p0 = pen;
                for k in 1..=n {
                    let t = k as f32 / n as f32;
                    let s = 1.0 - t;
                    let next = Vector::new(s * s * p0.x + 2.0 * s * t * c.x + t * t * q.x, s * s * p0.y + 2.0 * s * t * c.y + t * t * q.y);
                    line(&mut out, pen, next);
                    pen = next;
                }
                pen = q;
                at += 2;
            }
            Verb::CurveTo => {
                let (c1, c2, q) = (points[at], points[at + 1], points[at + 2]);
                let p0 = pen;
                let dd = |a: Vector, b: Vector, c: Vector| ((a.x - 2.0 * b.x + c.x).powi(2) + (a.y - 2.0 * b.y + c.y).powi(2)).sqrt();
                let n = pieces(0.75 * dd(p0, c1, c2).max(dd(c1, c2, q)));
                for k in 1..=n {
                    let t = k as f32 / n as f32;
                    let s = 1.0 - t;
                    let (a, b, c, d) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
                    let next = Vector::new(a * p0.x + b * c1.x + c * c2.x + d * q.x, a * p0.y + b * c1.y + c * c2.y + d * q.y);
                    line(&mut out, pen, next);
                    pen = next;
                }
                pen = q;
                at += 3;
            }
            Verb::Close => {
                if open {
                    line(&mut out, pen, start);
                }
                pen = start;
                open = false;
            }
        }
    }
    if open {
        line(&mut out, pen, start);
    }
    out
}

/// The pieces a curve is cut into for it to stray at most [`FLATNESS`]
/// from them: `deviation` is how far it strays from its chord.
fn pieces(deviation: f32) -> usize {
    ((deviation / FLATNESS).sqrt().ceil() as usize).clamp(1, 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(size: f32) -> (Vec<Vector>, Vec<Verb>) {
        let p = vec![Vector::new(0.0, 0.0), Vector::new(size, 0.0), Vector::new(size, size), Vector::new(0.0, size)];
        (p, vec![Verb::MoveTo, Verb::LineTo, Verb::LineTo, Verb::LineTo, Verb::Close])
    }

    /// A square's field: the distances to its edges, negative inside,
    /// at most the spread.
    #[test]
    fn a_squares_field_is_its_distances() {
        let (p, v) = square(40.0);
        let f = Sdf::of(&p, &v).unwrap();
        let at = |x: f32, y: f32| {
            let (i, j) = ((x - f.left - 0.5) as usize, (f.top - y - 0.5) as usize);
            f.data[j * f.w + i] as f32 / UNIT
        };
        assert_eq!(at(20.5, 20.5), -SPREAD, "deep inside: the spread");
        assert!((at(0.5, 20.5) + 0.5).abs() < 0.01, "{}", at(0.5, 20.5));
        assert!((at(-2.5, 20.5) - 2.5).abs() < 0.01);
        assert!((at(-1.5, -2.5) - (1.5f32.hypot(2.5))).abs() < 0.01);
        assert!((at(-SPREAD + 0.5, 20.5) - (SPREAD - 0.5)).abs() < 0.01);
        assert_eq!(at(-2.5, -2.5), SPREAD, "past the spread: the spread");
    }

    /// Two squares that overlap are one shape: the edges of each inside
    /// the other are no edge of it.
    #[test]
    fn overlapping_contours_are_one_shape() {
        let (mut p, mut v) = square(40.0);
        let (q, w) = square(40.0);
        p.extend(q.iter().map(|q| Vector::new(q.x + 20.0, q.y + 10.0)));
        v.extend(w);
        let f = Sdf::of(&p, &v).unwrap();
        let at = |x: f32, y: f32| f.data[(f.top - y - 0.5) as usize * f.w + (x - f.left - 0.5) as usize] as f32 / UNIT;
        // On the first square's right edge and the second's bottom, inside
        // both: deep inside the shape.
        assert_eq!(at(39.5, 30.5), -SPREAD);
        assert_eq!(at(30.5, 10.5), -SPREAD);
        // Their outer edges are its edges.
        assert!((at(0.5, 20.5) + 0.5).abs() < 0.01);
        assert!((at(59.5, 30.5) + 0.5).abs() < 0.01);
        assert!((at(30.5, 49.5) + 0.5).abs() < 0.01);
        assert!((at(41.5, 5.5) - 1.5).abs() < 0.01, "under the second, right of the first: {}", at(41.5, 5.5));
        assert!(f.jumps().is_empty());
    }

    /// Drawn at 1 output pixel a texel, a square's edges on pixel edges
    /// cover their pixels whole and none past them.
    #[test]
    fn a_square_on_pixel_edges_covers_its_pixels() {
        let (p, v) = square(8.0);
        let f = Sdf::of(&p, &v).unwrap();
        let mut lit = std::collections::HashMap::new();
        f.draw(10.0, 20.0, 1.0, 1.0, (0, 0, 100, 100), &mut |x, y, a| {
            lit.insert((x, y), a);
        });
        for y in 0..100 {
            for x in 0..100 {
                let inside = (10..18).contains(&x) && (12..20).contains(&y);
                assert_eq!(lit.get(&(x, y)).copied().unwrap_or(0), if inside { 255 } else { 0 }, "{x} {y}");
            }
        }
    }
}
