//! Fitting a traced picture's boundary lines with the shapes the pixels
//! are a rasterization of: each line between the places three colors meet
//! (junctions) as the fewest straight segments that pass within half a
//! pixel of its pixels' corners, each fitted over its whole length and set
//! to a clean slope (level, upright, 45 degrees, the pixel ratios 1:2, 1:3,
//! 2:3...) and a quarter pixel's place; the short segments of a rounded
//! corner between two straight edges an arc tangent to both (a radius in
//! half pixels), of a closed curve a circle. Each line is fitted once and
//! both its sides use it, and the lines meeting at a junction meet at one
//! point, so the regions still meet with no gap and no overlap.

/// A boundary line to fit: its pixel corners from start to end (a closed
/// one's last is its first), and whether it keeps them (against a hole).
pub struct Chain {
    pub corners: Vec<(i32, i32)>,
    pub closed: bool,
    pub fixed: bool,
}

type P = [f64; 2];

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn add(a: P, b: P) -> P {
    [a[0] + b[0], a[1] + b[1]]
}
fn mul(a: P, k: f64) -> P {
    [a[0] * k, a[1] * k]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: P, b: P) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn len(a: P) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: P) -> P {
    let l = len(a);
    if l == 0.0 { a } else { mul(a, 1.0 / l) }
}
fn pt(c: (i32, i32)) -> P {
    [c.0 as f64, c.1 as f64]
}

/// A fitted line: a point on it and its direction (unit, along the chain).
#[derive(Clone, Copy, Debug)]
struct Line {
    p: P,
    d: P,
}

impl Line {
    fn distance(&self, q: P) -> f64 {
        cross(self.d, sub(q, self.p)).abs()
    }

    fn meet(&self, o: &Line) -> Option<P> {
        let den = cross(self.d, o.d);
        if den.abs() < 0.08 {
            return None;
        }
        let t = cross(sub(o.p, self.p), o.d) / den;
        Some(add(self.p, mul(self.d, t)))
    }

    fn project(&self, q: P) -> P {
        add(self.p, mul(self.d, dot(sub(q, self.p), self.d)))
    }
}

/// The clean slopes a fitted line may take (directions with whole steps
/// of up to four pixels).
fn clean_directions() -> Vec<P> {
    let mut out = Vec::new();
    for a in -4i32..=4 {
        for b in -4i32..=4 {
            if (a, b) == (0, 0) || gcd(a.abs(), b.abs()) != 1 {
                continue;
            }
            out.push(unit([a as f64, b as f64]));
        }
    }
    out
}

fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// The largest angle a fitted line is turned by to a clean slope.
const SNAP_ANGLE: f64 = 0.07;
/// How far from a line its pixels' corners may be (a line within half a
/// pixel of each corner rasterizes back to those pixels).
const TOLERANCE: f64 = 0.55;
/// A run this long is an edge's: it may be in a slant's segment only with
/// two or more steps across it (one step between two long runs is a step).
const LONG_RUN: i32 = 3;
/// A segment this long or shorter, turning less than `SMOOTH_TURN` at a
/// corner, is a curve's.
const SHORT_SEGMENT: f64 = 3.0;
const SMOOTH_TURN: f64 = 70.0 * std::f64::consts::PI / 180.0;
/// Turns this small or smaller, three or more in a row one way, are a
/// curve's however long its segments.
const BEND_TURN: f64 = 50.0 * std::f64::consts::PI / 180.0;

/// The line fitted to `points` (least squares), along `along`; its slope
/// made clean and its place a quarter pixel's where the corners `check`
/// stay within the tolerance of it.
fn fit_line(points: &[P], along: P, check: &[P]) -> Line {
    let n = points.len() as f64;
    let m = mul(points.iter().fold([0.0, 0.0], |a, &p| add(a, p)), 1.0 / n);
    let (mut sxx, mut sxy, mut syy) = (0.0, 0.0, 0.0);
    for &p in points {
        let q = sub(p, m);
        sxx += q[0] * q[0];
        sxy += q[0] * q[1];
        syy += q[1] * q[1];
    }
    let angle = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let mut d = [angle.cos(), angle.sin()];
    if points.len() < 2 || (sxx + syy) < 1e-9 {
        d = unit(along);
    }
    if dot(d, along) < 0.0 {
        d = mul(d, -1.0);
    }
    let free = Line { p: m, d };
    let ok = |l: &Line| check.iter().all(|&q| l.distance(q) <= TOLERANCE + 1e-9);
    // The nearest clean slope, its place the points' mean, then the nearest
    // quarter pixel (as the line's offset across its slope's grid).
    let mut best: Option<(f64, Line)> = None;
    for c in clean_directions() {
        let c = if dot(c, d) < 0.0 { mul(c, -1.0) } else { c };
        let turn = cross(d, c).abs().asin();
        if turn > SNAP_ANGLE {
            continue;
        }
        let normal = [-c[1], c[0]];
        // (The offset in the direction's whole steps: its normal scaled so
        // a pixel's corners are whole numbers on it.)
        let scale = c[0].abs().max(c[1].abs());
        let k = dot(normal, m) / scale;
        let k = (k * 4.0).round() / 4.0 * scale;
        let snapped = Line { p: add(mul(normal, k), mul(c, dot(c, m))), d: c };
        let l = if ok(&snapped) { snapped } else { Line { p: m, d: c } };
        if ok(&l) && best.as_ref().is_none_or(|b| turn < b.0) {
            best = Some((turn, l));
        }
    }
    best.map_or(free, |b| b.1)
}

/// The midpoints of the unit edges from corner `a` to corner `b` of a
/// chain's points.
fn midpoints(pts: &[(i32, i32)], a: usize, b: usize) -> Vec<P> {
    (a..b).map(|i| mul(add(pt(pts[i]), pt(pts[i + 1])), 0.5)).collect()
}

/// A piece of a fitted chain: a straight line, or an arc of a circle
/// (its center, its radius, the way it turns: positive, clockwise on the
/// screen).
#[derive(Clone, Copy, Debug)]
enum Elem {
    Line(Line),
    Circle(P, f64, bool),
}

/// A fitted chain's pieces, and between each two a corner (where they
/// meet, near its pixel corner) or, between two lines, a curve (an arc of
/// the radius tangent to both).
struct Fitted {
    elems: Vec<Elem>,
    /// For each boundary between `elems[k]` and `elems[k + 1]` (and, closed,
    /// between the last and the first): the pixel corner there, and the
    /// rounding's radius where two lines meet in a curve.
    joints: Vec<(P, Option<f64>)>,
    /// A circle in place of the whole chain (center, radius).
    circle: Option<(P, f64)>,
}

impl Fitted {
    /// The line at its start (or end), if it starts (ends) with one.
    fn end_line(&self, last: bool) -> Option<Line> {
        match if last { self.elems.last() } else { self.elems.first() } {
            Some(Elem::Line(l)) => Some(*l),
            _ => None,
        }
    }
}

/// Fit a chain ([`Fitted`]), or none to keep its pixels' corners.
fn fit_chain(c: &Chain) -> Option<Fitted> {
    let mut pts = c.corners.clone();
    if c.fixed || pts.len() < 3 {
        return None;
    }
    if c.closed {
        pts.pop();
        // (A dot or a speck stays as its pixels.)
        let (x0, x1) = (pts.iter().map(|p| p.0).min()?, pts.iter().map(|p| p.0).max()?);
        let (y0, y1) = (pts.iter().map(|p| p.1).min()?, pts.iter().map(|p| p.1).max()?);
        if x1 - x0 <= 2 && y1 - y0 <= 2 {
            return None;
        }
    }
    let n = pts.len();
    let dir = |i: usize| -> (i32, i32) {
        let (a, b) = (pts[i % n], pts[(i + 1) % n]);
        (b.0 - a.0, b.1 - a.1)
    };
    // The corners: where the way changes (and an open chain's ends).
    let edges = if c.closed { n } else { n - 1 };
    let mut corner_at: Vec<usize> = (0..edges).filter(|&i| i == 0 && !c.closed || dir(i) != dir((i + edges - 1) % edges.max(1))).collect();
    if c.closed {
        if corner_at.len() < 3 {
            return None;
        }
        // (From the longest run's start.)
        let run = |k: usize| (corner_at[(k + 1) % corner_at.len()] + n - corner_at[k]) % n;
        let start = (0..corner_at.len()).max_by_key(|&k| (run(k), std::cmp::Reverse(k)))?;
        let s = corner_at[start];
        pts.rotate_left(s);
        corner_at = corner_at.iter().map(|&i| (i + n - s) % n).collect();
        corner_at.sort_unstable();
        pts.push(pts[0]);
        corner_at.push(n);
    } else {
        corner_at.push(n - 1);
    }
    let pts = pts;
    let k = corner_at.len();
    // The runs between corners: way and length.
    let runs: Vec<((i32, i32), i32)> = (0..k - 1)
        .map(|r| {
            let (a, b) = (pts[corner_at[r]], pts[corner_at[r + 1]]);
            let l = (b.0 - a.0).abs() + (b.1 - a.1).abs();
            (((b.0 - a.0).signum(), (b.1 - a.1).signum()), l)
        })
        .collect();
    // Whether corners a..b may be one straight segment.
    let straight = |a: usize, b: usize| -> bool {
        if b - a == 1 {
            return true;
        }
        let span = &runs[a..b];
        let (mut sx, mut sy) = (0, 0);
        for &((x, y), _) in span {
            if x != 0 {
                if sx != 0 && sx != x {
                    return false;
                }
                sx = x;
            }
            if y != 0 {
                if sy != 0 && sy != y {
                    return false;
                }
                sy = y;
            }
        }
        // (A short slant, of two or three runs, only where it turns no way
        // back with the runs either side: a pixel off a corner, not one
        // standing out.)
        if b - a <= 3 {
            let lo = a.saturating_sub(1);
            let hi = (b + 1).min(runs.len());
            if !monotone(&runs[lo..hi]) {
                return false;
            }
        }
        for &((x, _), l) in span {
            if l >= LONG_RUN {
                let across = span.iter().filter(|r| (r.0.0 != 0) != (x != 0)).count();
                if across < 2 {
                    return false;
                }
            }
        }
        let mids = midpoints(&pts, corner_at[a], corner_at[b]);
        let check: Vec<P> = (a..=b).map(|i| pt(pts[corner_at[i]])).collect();
        let along = sub(pt(pts[corner_at[b]]), pt(pts[corner_at[a]]));
        let l = fit_line(&mids, along, &[]);
        check.iter().all(|&q| l.distance(q) <= TOLERANCE)
    };
    // The fewest segments (then the least error), by dynamic programming.
    let mut best: Vec<Option<(usize, usize)>> = vec![None; k];
    best[0] = Some((0, 0));
    for b in 1..k {
        for a in (0..b).rev() {
            if !straight(a, b) {
                // (Once a span turns back, a longer one does too.)
                if b - a > 1 && !monotone(&runs[a..b]) {
                    break;
                }
                continue;
            }
            if let Some((count, _)) = best[a] {
                let c = count + 1;
                if best[b].is_none_or(|(bc, _)| c < bc) {
                    best[b] = Some((c, a));
                }
            }
        }
    }
    let mut vertices = vec![k - 1];
    while let Some(&v) = vertices.last() {
        if v == 0 {
            break;
        }
        vertices.push(best[v]?.1);
    }
    vertices.reverse();
    let segments = vertices.len() - 1;
    let lines: Vec<Line> = (0..segments)
        .map(|s| {
            let (a, b) = (corner_at[vertices[s]], corner_at[vertices[s + 1]]);
            let mids = midpoints(&pts, a, b);
            let check: Vec<P> = (vertices[s]..=vertices[s + 1]).map(|i| pt(pts[corner_at[i]])).collect();
            fit_line(&mids, sub(pt(pts[b]), pt(pts[a])), &check)
        })
        .collect();
    let seg_len = |s: usize| len(sub(pt(pts[corner_at[vertices[s + 1]]]), pt(pts[corner_at[vertices[s]]])));
    // The joints: between each two segments (closed: and the last and the
    // first), smooth where a short segment turns a little.
    let joints_n = if c.closed { segments } else { segments - 1 };
    let joint = |j: usize| -> (usize, usize) { (j, (j + 1) % segments) };
    let turn = |j: usize| {
        let (a, b) = joint(j);
        cross(lines[a].d, lines[b].d).atan2(dot(lines[a].d, lines[b].d))
    };
    // (And where three joints or more in a row turn one way a little each:
    // a curve whose segments are long, a large circle's.)
    let gentle = |j: usize| turn(j).abs() < BEND_TURN;
    let bend: Vec<bool> = (0..joints_n)
        .map(|j| {
            if !gentle(j) {
                return false;
            }
            let sign = turn(j) > 0.0;
            let alike = |k: usize| gentle(k) && (turn(k) > 0.0) == sign;
            let (mut lo, mut hi) = (j, j);
            while lo > 0 && alike(lo - 1) {
                lo -= 1;
            }
            while hi + 1 < joints_n && alike(hi + 1) {
                hi += 1;
            }
            hi - lo >= 2
        })
        .collect();
    let smooth: Vec<bool> = (0..joints_n)
        .map(|j| {
            let (a, b) = joint(j);
            bend[j] || turn(j).abs() < SMOOTH_TURN && seg_len(a).min(seg_len(b)) <= SHORT_SEGMENT
        })
        .collect();
    // A closed chain smooth all round: a circle.
    if c.closed && smooth.iter().all(|&s| s) {
        let mids = midpoints(&pts, 0, n);
        if let Some(circle) = fit_circle(&mids) {
            return Some(Fitted { elems: Vec::new(), joints: Vec::new(), circle: Some(circle) });
        }
    }
    // Each run of smooth joints is a curve: an arc tangent to the lines
    // either side of it where they are long and it turns less than a half
    // circle and a bit (a rounded corner), else a circle fitted to its
    // pixels over its short segments (a bend, a round part); the segments
    // inside it left out.
    let short = |s: usize| seg_len(s) <= SHORT_SEGMENT;
    let mut keep: Vec<bool> = vec![true; segments];
    let mut fillets: Vec<(usize, f64)> = Vec::new();
    let mut arcs: Vec<(usize, usize, P, f64, bool)> = Vec::new();
    let mut j = 0;
    while j < joints_n {
        if !smooth[j] {
            j += 1;
            continue;
        }
        // (A curve's segments are alike: one much longer than the rest is
        // a straight edge between two curves, a rounded rectangle's side.)
        let mut e = j;
        let mut shortest = f64::MAX;
        while e + 1 < joints_n && smooth[e + 1] {
            let inner = seg_len(joint(e).1);
            let shortest_next = shortest.min(inner);
            if inner > 3.0 && inner > 2.5 * shortest || shortest_next < f64::MAX && inner > 3.0 && shortest_next * 2.5 < inner {
                break;
            }
            shortest = shortest_next;
            e += 1;
        }
        let (before, after) = (joint(j).0, joint(e).1);
        let total: f64 = (j..=e).map(turn).sum();
        let deg = total.abs().to_degrees();
        let (a, b) = (corner_at[vertices[joint(j).1]], corner_at[vertices[joint(e).1]]);
        let mut done = false;
        // (Lines either side longer than the run's own segments.)
        let inner = (joint(j).1..=e).map(seg_len).fold(0.0, f64::max);
        let lined = |s: usize| seg_len(s) >= 2.0 && seg_len(s) > inner;
        if (12.0..=190.0).contains(&deg) && before != after && lined(before) && lined(after) {
            let samples = midpoints(&pts, a.min(b), a.max(b));
            // (A tangent point may go back along a segment as far as its
            // length, half where its other end is a curve too.)
            let other_smooth = |s: usize, at_end: bool| {
                let jj = if at_end { s } else { (s + joints_n.max(1) - 1) % joints_n.max(1) };
                (c.closed || if at_end { s < joints_n } else { s > 0 }) && jj < joints_n && smooth[jj] && jj != j && jj != e
            };
            let room = |s: usize, at_end: bool| seg_len(s) * if other_smooth(s, at_end) { 0.5 } else { 1.0 } + 1.0;
            if let Some(r) = fillet_radius(&lines[before], &lines[after], &samples, room(before, false), room(after, true)) {
                for s in joint(j).1..=e {
                    keep[s] = false;
                }
                // (Keyed by the joint's first kept segment.)
                fillets.push((before, r));
                done = true;
            }
        }
        if !done {
            // A circle over the run's segments and the short ones either
            // side of it, else over its own alone (its first and last
            // segments turn into lines that may not be the circle's).
            let wide = (if short(before) && before != after { before } else { joint(j).1 }, if short(after) && before != after { after } else { e });
            for (first, last) in [wide, (joint(j).1, e)] {
                if first > last {
                    continue;
                }
                let (from, to) = (corner_at[vertices[first]], corner_at[vertices[(last + 1) % segments.max(1)]]);
                let samples = if from < to { midpoints(&pts, from, to) } else { Vec::new() };
                // (How far the arc turns: its own joints'.)
                let bent: f64 = (first..last).map(|k| turn(k % joints_n.max(1))).sum();
                let fitted = fit_circle(&samples).filter(|_| bent.abs().to_degrees() >= 30.0);
                if let Some((center, r)) = fitted.filter(|_| (first..=last).all(|s| keep[s])) {
                    for s in first..=last {
                        keep[s] = false;
                    }
                    arcs.push((first, last, center, r, bent > 0.0));
                    break;
                }
            }
        }
        j = e + 1;
    }
    // The pieces in order: the kept segments' lines and the arcs.
    let mut elems: Vec<(usize, Elem)> = Vec::new();
    let mut s = 0;
    while s < segments {
        if let Some(&(first, last, center, r, way)) = arcs.iter().find(|a| a.0 == s) {
            elems.push((first, Elem::Circle(center, r, way)));
            s = last + 1;
            continue;
        }
        if keep[s] {
            elems.push((s, Elem::Line(lines[s])));
        }
        s += 1;
    }
    let mut joints = Vec::new();
    let count = elems.len();
    for (i, &(s, e)) in elems.iter().enumerate() {
        let Some(&(next, ne)) = elems.get(i + 1).or(if c.closed { elems.first() } else { None }) else { break };
        let radius = match (e, ne) {
            (Elem::Line(_), Elem::Line(_)) => fillets.iter().find(|f| f.0 == s).map(|f| f.1),
            _ => None,
        };
        // (The pixel corner where they meet: the next piece's first.)
        let at = pt(pts[corner_at[vertices[next]]]);
        let _ = count;
        joints.push((at, radius));
    }
    Some(Fitted { elems: elems.into_iter().map(|e| e.1).collect(), joints, circle: None })
}

fn monotone(span: &[((i32, i32), i32)]) -> bool {
    let (mut sx, mut sy) = (0, 0);
    for &((x, y), _) in span {
        if x != 0 {
            if sx != 0 && sx != x {
                return false;
            }
            sx = x;
        }
        if y != 0 {
            if sy != 0 && sy != y {
                return false;
            }
            sy = y;
        }
    }
    true
}

fn fit_circle_any(points: &[P]) -> Option<(P, f64)> {
    let n = points.len() as f64;
    if n < 3.0 {
        return None;
    }
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy, mut sxz, mut syz, mut sz) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for p in points {
        let z = p[0] * p[0] + p[1] * p[1];
        sx += p[0];
        sy += p[1];
        sxx += p[0] * p[0];
        syy += p[1] * p[1];
        sxy += p[0] * p[1];
        sxz += p[0] * z;
        syz += p[1] * z;
        sz += z;
    }
    let m = [[sxx, sxy, sx], [sxy, syy, sy], [sx, sy, n]];
    let v = [sxz, syz, sz];
    let det = |m: [[f64; 3]; 3]| m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let d = det(m);
    if d.abs() < 1e-9 {
        return None;
    }
    let col = |k: usize| {
        let mut mm = m;
        for r in 0..3 {
            mm[r][k] = v[r];
        }
        det(mm) / d
    };
    let (a, b, c) = (col(0), col(1), col(2));
    let center = [a / 2.0, b / 2.0];
    Some((center, (c + dot(center, center)).sqrt()))
}

/// The circle fitted to points (least squares, Kåsa's): its center and
/// radius on half pixels where that fits them within the tolerance, else
/// on quarters, else as fitted; none where it doesn't fit them.
fn fit_circle(points: &[P]) -> Option<(P, f64)> {
    if points.len() < 6 {
        return None;
    }
    let (center, r) = fit_circle_any(points)?;
    let worst = |c: P, r: f64| points.iter().map(|&p| (len(sub(p, c)) - r).abs()).fold(0.0, f64::max);
    if worst(center, r) > 0.6 || r < 1.0 {
        return None;
    }
    for grid in [2.0, 4.0, 8.0] {
        let snap = |v: f64| (v * grid).round() / grid;
        let (c, rr) = ([snap(center[0]), snap(center[1])], snap(r));
        if worst(c, rr) <= 0.7 {
            return Some((c, rr));
        }
    }
    Some((center, r))
}

/// The radius (in half pixels) of the arc tangent to `a` (coming in) and
/// `b` (going out) that best fits `samples`, its tangent points within
/// `room_a` and `room_b` of where the lines meet (or, the lines running
/// back alongside each other, the half circle between them); none where
/// no arc fits.
fn fillet_radius(a: &Line, b: &Line, samples: &[P], room_a: f64, room_b: f64) -> Option<f64> {
    if a.meet(b).is_none() {
        // (Back alongside: a half circle's radius is half their distance.)
        if dot(a.d, b.d) > -0.9 {
            return None;
        }
        let r = (a.distance(b.p) / 2.0 * 2.0).round() / 2.0;
        return (r >= 0.5).then_some(r);
    }
    let mut best: Option<(f64, f64)> = None;
    let mut r = 0.5;
    while r <= 16.0 {
        if let Some((t1, t2, center, _)) = fillet(a, b, r) {
            let k = a.meet(b)?;
            if len(sub(k, t1)) <= room_a + 1e-6 && len(sub(t2, k)) <= room_b + 1e-6 {
                // (Each sample's distance to the curve: the arc, or the
                // lines past its ends.)
                let mut curve: Vec<P> = vec![t1];
                curve.extend(arc_points(center, t1, fillet(a, b, r)?.3));
                let err: f64 = samples
                    .iter()
                    .map(|&p| {
                        let on_lines = segment_distance(p, sub(t1, mul(a.d, 16.0)), t1).min(segment_distance(p, t2, add(t2, mul(b.d, 16.0))));
                        let on_arc = curve.windows(2).map(|w| segment_distance(p, w[0], w[1])).fold(f64::MAX, f64::min);
                        on_lines.min(on_arc).powi(2)
                    })
                    .sum();
                if best.is_none_or(|(_, e)| err < e - 1e-9) {
                    best = Some((r, err));
                }
            }
        }
        r += 0.5;
    }
    best.map(|b| b.0)
}

/// The distance from `p` to the segment from `a` to `b`.
fn segment_distance(p: P, a: P, b: P) -> f64 {
    let ab = sub(b, a);
    let t = if dot(ab, ab) > 0.0 { (dot(sub(p, a), ab) / dot(ab, ab)).clamp(0.0, 1.0) } else { 0.0 };
    len(sub(p, add(a, mul(ab, t))))
}

/// The arc of radius `r` tangent to `a` (coming in) and `b` (going out):
/// its tangent points, its center and its sweep (signed, radians).
fn fillet(a: &Line, b: &Line, r: f64) -> Option<(P, P, P, f64)> {
    let k = a.meet(b)?;
    let turn = cross(a.d, b.d).atan2(dot(a.d, b.d));
    let half = (std::f64::consts::PI - turn.abs()) / 2.0;
    let t = r / half.tan();
    let t1 = sub(k, mul(a.d, t));
    let t2 = add(k, mul(b.d, t));
    // (The center on the side the chain turns to.)
    let side = if turn > 0.0 { [-a.d[1], a.d[0]] } else { [a.d[1], -a.d[0]] };
    let center = add(t1, mul(side, r));
    Some((t1, t2, center, turn))
}

/// An arc's points from `from` about `center`, sweeping `sweep` radians,
/// a point each tenth of a right angle or less (`from` left out).
fn arc_points(center: P, from: P, sweep: f64) -> Vec<P> {
    let r = len(sub(from, center));
    let a0 = (from[1] - center[1]).atan2(from[0] - center[0]);
    let steps = ((sweep.abs() / (std::f64::consts::PI / 20.0)).ceil() as usize).max(1);
    (1..=steps)
        .map(|i| {
            let a = a0 + sweep * i as f64 / steps as f64;
            [center[0] + r * a.cos(), center[1] + r * a.sin()]
        })
        .collect()
}

/// A step of a fitted path: a straight line to a point, or an arc of a
/// circle to it (its radius; turning the way of increasing angles, which
/// is clockwise on the screen; the larger of the two arcs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Line(P),
    Arc { to: P, radius: f64, sweep: bool, large: bool },
}

impl Seg {
    pub fn to(&self) -> P {
        match *self {
            Seg::Line(p) | Seg::Arc { to: p, .. } => p,
        }
    }
}

/// A fitted path: where it starts and its steps (a closed one's last ends
/// where it starts).
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub start: P,
    pub segs: Vec<Seg>,
}

impl Path {
    /// The same path walked the other way.
    pub fn reversed(&self) -> Path {
        let mut points = vec![self.start];
        points.extend(self.segs.iter().map(Seg::to));
        let n = self.segs.len();
        let segs = (0..n)
            .rev()
            .map(|i| match self.segs[i] {
                Seg::Line(_) => Seg::Line(points[i]),
                Seg::Arc { radius, sweep, large, .. } => Seg::Arc { to: points[i], radius, sweep: !sweep, large },
            })
            .collect();
        Path { start: points[n], segs }
    }

    pub fn end(&self) -> P {
        self.segs.last().map_or(self.start, Seg::to)
    }
}

/// The arc from `from` about `center`, sweeping `sweep` radians, as a step.
fn arc_seg(center: P, from: P, sweep: f64) -> Seg {
    let r = len(sub(from, center));
    let a0 = (from[1] - center[1]).atan2(from[0] - center[0]);
    let a = a0 + sweep;
    Seg::Arc { to: [center[0] + r * a.cos(), center[1] + r * a.sin()], radius: r, sweep: sweep > 0.0, large: sweep.abs() > std::f64::consts::PI }
}

/// Fit every chain and give each its path: the lines meeting at a junction
/// meet at one point there.
pub fn fit_all(chains: &[Chain]) -> Vec<Path> {
    use std::collections::BTreeMap;
    let fitted: Vec<Option<Fitted>> = chains.iter().map(fit_chain).collect();
    // Each junction's place: where its lines' ends best meet (the corner
    // itself where one of its lines keeps its pixels, or they don't meet
    // near it).
    let mut ends: BTreeMap<(i32, i32), Vec<Option<Line>>> = BTreeMap::new();
    for (c, f) in chains.iter().zip(&fitted) {
        if c.closed {
            continue;
        }
        let (first, last) = (c.corners[0], *c.corners.last().unwrap());
        let (a, b) = match f {
            Some(f) => (f.end_line(false), f.end_line(true)),
            None => (None, None),
        };
        ends.entry(first).or_default().push(a);
        ends.entry(last).or_default().push(b);
    }
    let place: BTreeMap<(i32, i32), P> = ends
        .iter()
        .map(|(&corner, lines)| {
            let at = pt(corner);
            if lines.iter().any(Option::is_none) {
                return (corner, at);
            }
            let lines: Vec<Line> = lines.iter().flatten().copied().collect();
            // Least squares: the point nearest all the lines.
            let (mut a11, mut a12, mut a22, mut b1, mut b2) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for l in &lines {
                let n = [-l.d[1], l.d[0]];
                let c = dot(n, l.p);
                a11 += n[0] * n[0];
                a12 += n[0] * n[1];
                a22 += n[1] * n[1];
                b1 += n[0] * c;
                b2 += n[1] * c;
            }
            let det = a11 * a22 - a12 * a12;
            let p = if det.abs() > 1e-6 { [(b1 * a22 - b2 * a12) / det, (a11 * b2 - a12 * b1) / det] } else { lines[0].project(at) };
            let p = if len(sub(p, at)) <= 1.0 { p } else { at };
            (corner, p)
        })
        .collect();
    chains
        .iter()
        .zip(&fitted)
        .map(|(c, f)| {
            let Some(f) = f else {
                let points: Vec<P> = c.corners.iter().map(|&q| pt(q)).collect();
                return Path { start: points[0], segs: points[1..].iter().map(|&p| Seg::Line(p)).collect() };
            };
            if let Some((center, r)) = f.circle {
                // (Turning the chain's way: two half circles.)
                let area: f64 = c.corners.windows(2).map(|w| cross(pt(w[0]), pt(w[1]))).sum();
                let half = if area >= 0.0 { std::f64::consts::PI } else { -std::f64::consts::PI };
                let from = [center[0] + r, center[1]];
                let first = arc_seg(center, from, half);
                let second = arc_seg(center, first.to(), half);
                let second = match second {
                    Seg::Arc { radius, sweep, large, .. } => Seg::Arc { to: from, radius, sweep, large },
                    s => s,
                };
                return Path { start: from, segs: vec![first, second] };
            }
            // The steps at each joint: what ends the piece before it (a line
            // to the corner, or to a rounding's start and along it; an arc
            // to the corner's place on its circle).
            let n = f.elems.len();
            let piece_end = |e: &Elem, to: P| -> (Seg, Option<P>) {
                match *e {
                    Elem::Line(_) => (Seg::Line(to), None),
                    Elem::Circle(center, r, way) => (Seg::Arc { to, radius: r, sweep: way, large: false }, Some(center)),
                }
            };
            let on = |e: &Elem, q: P| -> P {
                match *e {
                    Elem::Line(l) => l.project(q),
                    Elem::Circle(center, r, _) => add(center, mul(unit(sub(q, center)), r)),
                }
            };
            let joint = |k: usize| -> Vec<(Seg, Option<P>)> {
                let (a, b) = (&f.elems[k], &f.elems[(k + 1) % n]);
                let (corner, radius) = f.joints[k];
                if let (Elem::Line(la), Elem::Line(lb), Some(r)) = (a, b, radius) {
                    if let Some((t1, _, center, turn)) = fillet(la, lb, r) {
                        return vec![(Seg::Line(t1), None), (arc_seg(center, t1, turn), None)];
                    }
                    let mid = add(la.p, mul(sub(lb.p, la.p), 0.5));
                    let center = add(mid, mul(la.d, dot(sub(la.project(corner), mid), la.d)));
                    let t1 = la.project(center);
                    let turn = if cross(la.d, sub(lb.p, la.p)) > 0.0 { std::f64::consts::PI } else { -std::f64::consts::PI };
                    return vec![(Seg::Line(t1), None), (arc_seg(center, t1, turn), None)];
                }
                if let (Elem::Line(la), Elem::Line(lb)) = (a, b) {
                    return match la.meet(lb) {
                        Some(p) if len(sub(p, corner)) <= 1.5 => vec![(Seg::Line(p), None)],
                        _ => vec![(Seg::Line(la.project(corner)), None), (Seg::Line(lb.project(corner)), None)],
                    };
                }
                // (A line and an arc: where the arc's circle is nearest the
                // corner; the line meets it there.)
                let at = match (a, b) {
                    (Elem::Circle(..), _) => on(a, corner),
                    _ => on(b, corner),
                };
                vec![piece_end(a, at)]
            };
            let mut segs: Vec<Seg> = Vec::new();
            let mut from = if c.closed { None } else { Some(place[&c.corners[0]]) };
            let mut steps: Vec<(Seg, Option<P>)> = (0..f.joints.len()).flat_map(joint).collect();
            if !c.closed {
                let end = place[c.corners.last().unwrap()];
                steps.push(match f.elems.last() {
                    Some(e) => piece_end(e, end),
                    None => (Seg::Line(end), None),
                });
            }
            // (Each arc's way and size from where it starts.)
            let start = match from.take() {
                Some(p) => p,
                None => steps.last().map(|s| s.0.to()).unwrap_or([0.0, 0.0]),
            };
            let mut at = start;
            for (s, center) in steps {
                let s = match (s, center) {
                    // (A fitted circle's arc: as far round as its ends are,
                    // its way.)
                    (Seg::Arc { to, radius, sweep, .. }, Some(c)) => {
                        let (a0, a1) = ((at[1] - c[1]).atan2(at[0] - c[0]), (to[1] - c[1]).atan2(to[0] - c[0]));
                        let tau = std::f64::consts::TAU;
                        let span = if sweep { (a1 - a0).rem_euclid(tau) } else { (a0 - a1).rem_euclid(tau) };
                        Seg::Arc { to, radius, sweep, large: span > std::f64::consts::PI }
                    }
                    (s, _) => s,
                };
                at = s.to();
                segs.push(s);
            }
            Path { start, segs }
        })
        .collect()
}
