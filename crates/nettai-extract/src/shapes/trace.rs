//! Pixel art as filled shapes, for the enhanced graphics to draw the
//! battle's frames at the output's resolution (docs/design/text-rendering.md
//! §13): a picture of palette indices traced once, as a pack is made, into
//! regions of one color each, their outlines polygons in frame pixels.
//!
//! The regions are a partition: each boundary between two colors is one
//! line, simplified once and shared by both sides, so the shapes meet with
//! no gap and no overlap. A boundary keeps the pixels' outline but where
//! the pixels step: a run of short steps that keep one direction (a slant,
//! a curve, a lone corner pixel taken off) between two straight edges (or
//! the picture's places where three colors meet) becomes the line through
//! the steps' middles, met by the edges on either side, as the banners'
//! bars' ends are (`letters::bar`). That line leaves as much of each color
//! on either side as the steps did. A single step in a long edge, a bump, a
//! dot and every edge against a [`Label::Hole`] keep their pixels' outline.
//! Two pixels of one color that touch only at a corner (a line a pixel wide
//! at 45 degrees) are one region through that corner where that color is
//! the sparser of the two around it, so a diagonal line is a slanted band.
//!
//! Nothing here is a game's: the pictures say what is drawn.

use super::fit;

/// What a pixel of a traced picture is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Label {
    /// Transparent: what is behind shows. Its edges are a shape's outline
    /// and are simplified as any other.
    Clear,
    /// Not the picture's: something else is drawn there (a chip's picture
    /// in the window, a neighbor cell's piece). An edge against it keeps
    /// its pixels' outline exactly.
    Hole,
    /// A color: a palette row (for a picture laid in several) and an index.
    Color(u8, u8),
}

/// A picture to trace: its labels row by row, and what is past each of
/// its sides (top, right, bottom, left).
#[derive(Clone, Debug)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<Label>,
    pub outside: [Label; 4],
}

impl Grid {
    /// The label at (x, y), past the sides what is there (a corner past two
    /// sides: a hole if either is).
    fn at(&self, x: i32, y: i32) -> Label {
        let inside_x = x >= 0 && (x as usize) < self.w;
        let inside_y = y >= 0 && (y as usize) < self.h;
        if inside_x && inside_y {
            return self.cells[y as usize * self.w + x as usize];
        }
        let mut sides = Vec::new();
        if y < 0 {
            sides.push(self.outside[0]);
        }
        if x >= self.w as i32 {
            sides.push(self.outside[1]);
        }
        if y >= self.h as i32 {
            sides.push(self.outside[2]);
        }
        if x < 0 {
            sides.push(self.outside[3]);
        }
        if sides.contains(&Label::Hole) { Label::Hole } else { sides[0] }
    }
}

/// A traced region: a color (palette row, index) and its outline's rings
/// (each a closed path of lines and arcs, in frame pixels from the
/// picture's top left), filled even-odd.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub palette: u8,
    pub color: u8,
    pub rings: Vec<fit::Path>,
}

/// The four ways from a corner of the pixel grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Dir {
    N,
    E,
    S,
    W,
}

impl Dir {
    fn step(self) -> (i32, i32) {
        match self {
            Dir::N => (0, -1),
            Dir::E => (1, 0),
            Dir::S => (0, 1),
            Dir::W => (-1, 0),
        }
    }

    fn back(self) -> Dir {
        match self {
            Dir::N => Dir::S,
            Dir::E => Dir::W,
            Dir::S => Dir::N,
            Dir::W => Dir::E,
        }
    }

    fn all() -> [Dir; 4] {
        [Dir::N, Dir::E, Dir::S, Dir::W]
    }
}

struct Tracer<'g> {
    g: &'g Grid,
}

/// How a corner of the grid passes the boundary through it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Corner {
    /// No boundary.
    None,
    /// Where lines end: three colors meet, or a side's labels change.
    Junction,
    /// A boundary passes: each way out pairs with one other (its index in
    /// `Dir::all()`).
    Through([Option<Dir>; 4]),
}

impl Tracer<'_> {
    /// The labels either side of the edge from corner (x, y) going `d`:
    /// (left, right) as one walks it.
    fn sides(&self, x: i32, y: i32, d: Dir) -> (Label, Label) {
        let g = self.g;
        match d {
            // Going east along y: above is left.
            Dir::E => (g.at(x, y - 1), g.at(x, y)),
            Dir::W => (g.at(x - 1, y), g.at(x - 1, y - 1)),
            Dir::S => (g.at(x, y), g.at(x - 1, y)),
            Dir::N => (g.at(x - 1, y - 1), g.at(x, y - 1)),
        }
    }

    fn in_range(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x <= self.g.w as i32 && y <= self.g.h as i32
    }

    /// Whether the edge from (x, y) going `d` is a boundary (both its ends
    /// corners of the grid, its sides two labels).
    fn edge(&self, x: i32, y: i32, d: Dir) -> bool {
        let (dx, dy) = d.step();
        if !self.in_range(x, y) || !self.in_range(x + dx, y + dy) {
            return false;
        }
        let (l, r) = self.sides(x, y, d);
        l != r
    }

    fn corner(&self, x: i32, y: i32) -> Corner {
        let g = self.g;
        let ways: Vec<Dir> = Dir::all().into_iter().filter(|&d| self.edge(x, y, d)).collect();
        match ways.len() {
            0 => Corner::None,
            2 => {
                // (One pair of labels both ways, else a side's labels change
                // here.)
                let pair = |d: Dir| {
                    let (l, r) = self.sides(x, y, d);
                    if l < r { (l, r) } else { (r, l) }
                };
                if pair(ways[0]) != pair(ways[1]) {
                    return Corner::Junction;
                }
                let mut through = [None; 4];
                through[ways[0] as usize] = Some(ways[1]);
                through[ways[1] as usize] = Some(ways[0]);
                Corner::Through(through)
            }
            4 => {
                let (nw, ne, sw, se) = (g.at(x - 1, y - 1), g.at(x, y - 1), g.at(x - 1, y), g.at(x, y));
                let mut through = [None; 4];
                let pair = |through: &mut [Option<Dir>; 4], p: Dir, q: Dir| {
                    through[p as usize] = Some(q);
                    through[q as usize] = Some(p);
                };
                // How many of a color's pixels are around the corner.
                let count = |l: Label| {
                    let mut n = 0;
                    for yy in y - 4..y + 4 {
                        for xx in x - 4..x + 4 {
                            if xx >= 0 && yy >= 0 && (xx as usize) < g.w && (yy as usize) < g.h && g.at(xx, yy) == l {
                                n += 1;
                            }
                        }
                    }
                    n
                };
                // One color on one diagonal and two others on the other, the
                // first the sparsest of them (a line a pixel wide at 45
                // degrees between two colors): it joins through, a line
                // either side of it passing.
                let line = |l: Label, a: Label, b: Label| count(l) < count(a) && count(l) < count(b);
                if nw == se && ne != sw && nw != Label::Hole && ne != Label::Hole && sw != Label::Hole && line(nw, ne, sw) {
                    pair(&mut through, Dir::N, Dir::E);
                    pair(&mut through, Dir::S, Dir::W);
                    return Corner::Through(through);
                }
                if ne == sw && nw != se && ne != Label::Hole && nw != Label::Hole && se != Label::Hole && line(ne, nw, se) {
                    pair(&mut through, Dir::N, Dir::W);
                    pair(&mut through, Dir::S, Dir::E);
                    return Corner::Through(through);
                }
                if nw != se || ne != sw || nw == Label::Hole || ne == Label::Hole {
                    return Corner::Junction;
                }
                // The sparser of the two around the corner is the one that
                // joins through it.
                let (a, b) = (count(nw), count(ne));
                match a.cmp(&b) {
                    // NW and SE join: NE's corner and SW's turn.
                    std::cmp::Ordering::Less => {
                        pair(&mut through, Dir::N, Dir::E);
                        pair(&mut through, Dir::S, Dir::W);
                    }
                    std::cmp::Ordering::Greater => {
                        pair(&mut through, Dir::N, Dir::W);
                        pair(&mut through, Dir::S, Dir::E);
                    }
                    std::cmp::Ordering::Equal => return Corner::Junction,
                }
                Corner::Through(through)
            }
            _ => Corner::Junction,
        }
    }
}

/// A boundary line between two labels: its labels (left, right as walked)
/// and its corners, from where it starts to where it ends (a closed one's
/// last corner is its first).
struct Line {
    left: Label,
    right: Label,
    closed: bool,
    corners: Vec<(i32, i32)>,
}

/// Trace `g` into its colors' regions (none for the clear or the holes),
/// in the order of their colors.
pub fn trace(g: &Grid) -> Vec<Region> {
    let g = &without_specks(g);
    let t = Tracer { g };
    let (w, h) = (g.w as i32, g.h as i32);
    let corners: Vec<Corner> = (0..=h).flat_map(|y| (0..=w).map(move |x| (x, y))).map(|(x, y)| t.corner(x, y)).collect();
    let corner = |x: i32, y: i32| corners[(y * (w + 1) + x) as usize];
    // Each edge walked once: by its corner and way, both ways marked.
    let mut walked = std::collections::HashSet::new();
    let mut lines = Vec::new();
    let walk = |x0: i32, y0: i32, d0: Dir, walked: &mut std::collections::HashSet<(i32, i32, Dir)>| -> Option<Line> {
        if walked.contains(&(x0, y0, d0)) || !t.edge(x0, y0, d0) {
            return None;
        }
        let (left, right) = t.sides(x0, y0, d0);
        let mut corners_out = vec![(x0, y0)];
        let (mut x, mut y, mut d) = (x0, y0, d0);
        loop {
            walked.insert((x, y, d));
            let (dx, dy) = d.step();
            let (nx, ny) = (x + dx, y + dy);
            walked.insert((nx, ny, d.back()));
            corners_out.push((nx, ny));
            match corner(nx, ny) {
                Corner::Through(through) => {
                    let Some(next) = through[d.back() as usize] else { break };
                    if (nx, ny, next) == (x0, y0, d0) {
                        return Some(Line { left, right, closed: true, corners: corners_out });
                    }
                    (x, y, d) = (nx, ny, next);
                }
                _ => break,
            }
        }
        Some(Line { left, right, closed: false, corners: corners_out })
    };
    // The lines from the junctions, then the closed ones.
    for y in 0..=h {
        for x in 0..=w {
            if corner(x, y) == Corner::Junction {
                for d in Dir::all() {
                    lines.extend(walk(x, y, d, &mut walked));
                }
            }
        }
    }
    for y in 0..=h {
        for x in 0..=w {
            for d in Dir::all() {
                lines.extend(walk(x, y, d, &mut walked));
            }
        }
    }
    let chains: Vec<fit::Chain> = lines
        .iter()
        .map(|l| fit::Chain { corners: l.corners.clone(), closed: l.closed, fixed: l.left == Label::Hole || l.right == Label::Hole })
        .collect();
    let fitted: Vec<(Label, Label, fit::Path, bool)> =
        lines.iter().zip(fit::fit_all(&chains)).map(|(l, path)| (l.left, l.right, path, l.closed)).collect();
    // Each color's rings: its lines joined end to end (even-odd fill makes
    // any joining at a corner the same area).
    let mut colors: Vec<Label> = fitted.iter().flat_map(|s| [s.0, s.1]).filter(|l| matches!(l, Label::Color(..))).collect();
    colors.sort();
    colors.dedup();
    let mut out = Vec::new();
    for c in colors {
        let mut rings = Vec::new();
        let mut open: Vec<fit::Path> = Vec::new();
        for (l, r, path, closed) in &fitted {
            if *l != c && *r != c {
                continue;
            }
            if *closed { rings.push(path.clone()) } else { open.push(path.clone()) }
        }
        rings.extend(join(open));
        let rings: Vec<fit::Path> = rings.into_iter().map(|r| without_collinear(&r)).filter(|r| r.segs.len() >= 2).collect();
        if let (Label::Color(palette, color), false) = (c, rings.is_empty()) {
            out.push(Region { palette, color, rings });
        }
    }
    out
}

fn key(p: [f64; 2]) -> (i64, i64) {
    ((p[0] * 64.0).round() as i64, (p[1] * 64.0).round() as i64)
}

/// The largest speck of a color (in pixels) that is a picture's shading of
/// an edge rather than a part of it.
const SPECK: usize = 3;

/// `g` with its edges' shading taken off: each speck of a color (a few
/// pixels) that lies between two or more others (the pixel art's
/// in-between colors on a slant's or a curve's steps, which a fitted shape
/// needs none of) made the color it shares most edge with. A speck inside
/// one color alone (a rivet, a hole) stays.
fn without_specks(g: &Grid) -> Grid {
    let mut out = g.clone();
    let (w, h) = (g.w, g.h);
    for _ in 0..3 {
        let mut seen = vec![false; w * h];
        let mut changed = false;
        for start in 0..w * h {
            let Label::Color(..) = out.cells[start] else { continue };
            if seen[start] {
                continue;
            }
            // The speck's pixels (eight-connected: a line a pixel wide at
            // 45 degrees is one), up to one more than a speck's.
            let label = out.cells[start];
            let mut comp = vec![start];
            seen[start] = true;
            let mut k = 0;
            let mut big = false;
            while k < comp.len() {
                let i = comp[k];
                k += 1;
                let (x, y) = ((i % w) as i32, (i / w) as i32);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if !seen[j] && out.cells[j] == label {
                        seen[j] = true;
                        comp.push(j);
                    }
                }
                if comp.len() > SPECK {
                    big = true;
                }
            }
            if big {
                continue;
            }
            // Its neighbors (inside the picture, holes aside) and the edges
            // it shares with each.
            let mut around: Vec<(Label, usize)> = Vec::new();
            let mut touches_hole = false;
            for &i in &comp {
                let (x, y) = ((i % w) as i32, (i / w) as i32);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let n = out.at(x + dx, y + dy);
                    if n == label {
                        continue;
                    }
                    if n == Label::Hole {
                        touches_hole = true;
                        continue;
                    }
                    match around.iter_mut().find(|a| a.0 == n) {
                        Some(a) => a.1 += 1,
                        None => around.push((n, 1)),
                    }
                }
            }
            if around.len() < 2 || touches_hole {
                continue;
            }
            let (to, _) = *around.iter().max_by_key(|a| (a.1, std::cmp::Reverse(a.0))).unwrap();
            for &i in &comp {
                out.cells[i] = to;
            }
            changed = true;
        }
        if !changed {
            break;
        }
    }
    out
}

/// Open lines joined end to end into rings.
fn join(mut open: Vec<fit::Path>) -> Vec<fit::Path> {
    let mut rings = Vec::new();
    while let Some(first) = open.pop() {
        let start = key(first.start);
        let mut ring = first;
        loop {
            let end = key(ring.end());
            if end == start {
                break;
            }
            let next = open.iter().position(|l| key(l.start) == end || key(l.end()) == end);
            let Some(i) = next else { break };
            let mut l = open.swap_remove(i);
            if key(l.start) != end {
                l = l.reversed();
            }
            ring.segs.extend(l.segs);
        }
        rings.push(ring);
    }
    rings
}

/// A ring without the corners where a straight line goes on straight.
fn without_collinear(ring: &fit::Path) -> fit::Path {
    let n = ring.segs.len();
    if n < 3 {
        return ring.clone();
    }
    let point = |i: usize| if i == 0 { ring.start } else { ring.segs[i - 1].to() };
    // Corner i is where step i - 1 ends (step n - 1 ends at the start).
    let mut keep = vec![true; n];
    for i in 0..n {
        let (prev, next) = (ring.segs[(i + n - 1) % n], ring.segs[i]);
        if let (fit::Seg::Line(b), fit::Seg::Line(c)) = (prev, next) {
            let a = point((i + n - 1) % n);
            let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
            let back = (b[0] - a[0]) * (c[0] - b[0]) + (b[1] - a[1]) * (c[1] - b[1]) < 0.0;
            if cross.abs() < 1e-6 && !back {
                keep[(i + n - 1) % n] = false;
            }
        }
    }
    let segs: Vec<fit::Seg> = ring.segs.iter().zip(&keep).filter(|(_, k)| **k).map(|(s, _)| *s).collect();
    let start = segs.last().map(fit::Seg::to).unwrap_or(ring.start);
    fit::Path { start, segs }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Label = Label::Color(0, 1);
    const B: Label = Label::Color(0, 2);
    const C: Label = Label::Clear;

    fn grid(rows: &[&str], outside: Label) -> Grid {
        let h = rows.len();
        let w = rows[0].len();
        let cells = rows
            .iter()
            .flat_map(|r| {
                r.chars().map(|c| match c {
                    '.' => C,
                    '#' => Label::Hole,
                    'a' => A,
                    'b' => B,
                    _ => panic!(),
                })
            })
            .collect();
        Grid { w, h, cells, outside: [outside; 4] }
    }

    /// A ring's points, its arcs in short lines.
    fn flatten(p: &fit::Path) -> Vec<[f64; 2]> {
        let mut out = vec![p.start];
        let mut at = p.start;
        for s in &p.segs {
            if let fit::Seg::Arc { to, radius, sweep, large } = *s {
                let (m, h) = ([(at[0] + to[0]) / 2.0, (at[1] + to[1]) / 2.0], ((to[0] - at[0]).hypot(to[1] - at[1])) / 2.0);
                let d = (radius * radius - h * h).max(0.0).sqrt();
                let u = [-(to[1] - at[1]) / (2.0 * h), (to[0] - at[0]) / (2.0 * h)];
                let sign = if large != sweep { 1.0 } else { -1.0 };
                let c = [m[0] + sign * d * u[0], m[1] + sign * d * u[1]];
                let (a0, mut a1) = ((at[1] - c[1]).atan2(at[0] - c[0]), (to[1] - c[1]).atan2(to[0] - c[0]));
                if sweep && a1 < a0 {
                    a1 += std::f64::consts::TAU;
                }
                if !sweep && a1 > a0 {
                    a1 -= std::f64::consts::TAU;
                }
                for k in 1..32 {
                    let a = a0 + (a1 - a0) * k as f64 / 32.0;
                    out.push([c[0] + radius * a.cos(), c[1] + radius * a.sin()]);
                }
            }
            out.push(s.to());
            at = s.to();
        }
        out
    }

    fn area(p: &fit::Path) -> f64 {
        let r = flatten(p);
        let n = r.len();
        (0..n).map(|i| r[i][0] * r[(i + 1) % n][1] - r[(i + 1) % n][0] * r[i][1]).sum::<f64>().abs() / 2.0
    }

    fn region(rs: &[Region], color: u8) -> &Region {
        rs.iter().find(|r| r.color == color).unwrap()
    }

    fn arcs(p: &fit::Path) -> usize {
        p.segs.iter().filter(|s| matches!(s, fit::Seg::Arc { .. })).count()
    }

    /// A rectangle stays one: its corners, its area.
    #[test]
    fn a_box_keeps_its_corners() {
        let r = trace(&grid(&["aaaa", "aaaa", "aaaa"], C));
        assert_eq!((r.len(), r[0].rings.len(), r[0].rings[0].segs.len()), (1, 1, 4));
        assert_eq!(area(&r[0].rings[0]), 12.0);
    }

    /// A corner pixel off a box is a rounded corner: an arc tangent to both
    /// edges, about as much taken off.
    #[test]
    fn a_corner_pixel_off_is_rounded() {
        let r = trace(&grid(&[".aaaaa", "aaaaaa", "aaaaaa", "aaaaaa"], C));
        let ring = &r[0].rings[0];
        assert_eq!(arcs(ring), 1, "{ring:?}");
        assert!((area(ring) - 23.0).abs() <= 0.6, "{}", area(ring));
    }

    /// One step in a long edge stays a step.
    #[test]
    fn a_single_step_stays() {
        let r = trace(&grid(&["aaaaaa....", "aaaaaaaaaa", "aaaaaaaaaa"], C));
        let ring = &r[0].rings[0];
        assert_eq!(area(ring), 26.0);
        assert_eq!(ring.segs.len(), 6, "{ring:?}");
    }

    /// A staircase is one straight slant, at its slope, as much of the shape
    /// on either side.
    #[test]
    fn steps_are_a_slant() {
        let r = trace(&grid(&["....aaaa", "...aaaaa", "..aaaaaa", ".aaaaaaa", "aaaaaaaa"], C));
        let ring = &r[0].rings[0];
        assert_eq!(ring.segs.len(), 4, "{ring:?}");
        assert!((area(ring) - 30.0).abs() < 0.3, "{}", area(ring));
        // (A slope of 1:2, its top a tread of it: a triangle.)
        let r = trace(&grid(&["......aa", "....aaaa", "..aaaaaa", "aaaaaaaa"], C));
        assert_eq!(r[0].rings[0].segs.len(), 3, "{:?}", r[0].rings[0]);
        assert!((area(&r[0].rings[0]) - 20.0).abs() < 0.5);
    }

    /// Two colors meet on one line, fitted once: their areas add up to the
    /// picture's.
    #[test]
    fn two_colors_share_their_line() {
        let r = trace(&grid(&["bbbbaaaa", "bbbaaaaa", "bbaaaaaa", "baaaaaaa", "aaaaaaaa"], C));
        let (a, b) = (region(&r, 1), region(&r, 2));
        let total = area(&a.rings[0]) + area(&b.rings[0]);
        assert!((total - 40.0).abs() < 0.01);
        assert!(a.rings[0].segs.len() <= 5 && b.rings[0].segs.len() <= 3, "{:?} {:?}", a.rings, b.rings);
    }

    /// An edge against a hole keeps its pixels.
    #[test]
    fn holes_keep_their_edges() {
        let r = trace(&grid(&["#aaaa", "##aaa", "###aa", "####a"], C));
        assert_eq!(area(&r[0].rings[0]), 10.0);
    }

    /// A line a pixel wide at 45 degrees is one slanted band.
    #[test]
    fn a_diagonal_line_is_a_band() {
        let r = trace(&grid(&["a.....", ".a....", "..a...", "...a..", "....a.", ".....a"], C));
        assert_eq!(r[0].rings.len(), 1, "{:?}", r[0].rings);
        assert!((area(&r[0].rings[0]) - 6.0).abs() < 0.8, "{}", area(&r[0].rings[0]));
    }

    /// A dot and a bump stay squares.
    #[test]
    fn dots_and_bumps_stay() {
        let r = trace(&grid(&[".....", "..a..", ".....", "bbbbb"], C));
        assert_eq!(region(&r, 1).rings[0].segs.len(), 4);
        let r = trace(&grid(&["..a....", "aaaaaaa", "aaaaaaa"], C));
        assert_eq!(region(&r, 1).rings[0].segs.len(), 8, "{:?}", region(&r, 1).rings[0]);
    }

    /// A round blob is a circle.
    #[test]
    fn a_disc_is_a_circle() {
        let r = trace(&grid(
            &["..........", "...aaaa...", "..aaaaaa..", ".aaaaaaaa.", ".aaaaaaaa.", ".aaaaaaaa.", ".aaaaaaaa.", "..aaaaaa..", "...aaaa...", ".........."],
            C,
        ));
        let ring = &r[0].rings[0];
        assert_eq!(arcs(ring), ring.segs.len(), "{ring:?}");
    }

    /// A ring of a color around another: two rings, filled even-odd.
    #[test]
    fn a_frame_has_a_hole() {
        let r = trace(&grid(&["aaaaa", "abbba", "abbba", "aaaaa"], C));
        assert_eq!(region(&r, 1).rings.len(), 2);
        assert_eq!(region(&r, 2).rings.len(), 1);
    }
}
