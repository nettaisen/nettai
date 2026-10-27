//! The battle's frames as shapes in a pack (`nettai_assets::shapes`): an SVG
//! file a drawing (`shapes/NAME.svg` beside the pictures), read and drawn
//! by usvg and resvg, in frame pixels from the drawing's top left, the
//! `viewBox` the picture's size. Its colors are the palette's at the time
//! it is drawn: each shape (or a group round it) names its fill as data,
//! `data-color`, the palette index, and its outline's, `data-stroke`, with
//! `data-palette`, the palette row where the picture is laid in several (a
//! window map's), on it or round it; the file's own `fill` and `stroke` are
//! those colors in the picture's first palette, so the file looks as the
//! picture does where it is opened, and are for viewing only. A `text` with
//! `data-words` is words written on it (`Words`), drawn by the text layer,
//! not as the file's. What SVG can't say (which cells of a map a drawing
//! stands in for) is in `hud.json` and `custom.json`.

use nettai_assets::shapes::{Drawing, Words};
pub use resvg::tiny_skia;
use resvg::usvg::{self, roxmltree};

/// The elements that draw: each fills (or outlines) in a palette entry.
const SHAPES: [&str; 7] = ["rect", "circle", "ellipse", "line", "polyline", "polygon", "path"];

/// A data attribute's palette number (none: it hasn't the attribute).
fn number_of(node: roxmltree::Node, name: &str) -> Result<Option<u8>, String> {
    match node.attribute(name) {
        Some(t) => match t.trim().parse::<u8>() {
            Ok(c) if c <= 15 => Ok(Some(c)),
            _ => Err(format!("{name} {t:?} isn't a palette entry (0 to 15)")),
        },
        None => Ok(None),
    }
}

/// A data attribute's palette number on `node` or the nearest element round
/// it that has one.
fn inherited(node: roxmltree::Node, name: &str) -> Result<Option<u8>, String> {
    for n in node.ancestors().filter(|n| n.is_element()) {
        if let Some(v) = number_of(n, name)? {
            return Ok(Some(v));
        }
    }
    Ok(None)
}

/// A drawing read from its SVG file's text: the `viewBox`'s size, the
/// palette entries its shapes name, its words; checked as usvg reads it,
/// every shape filled or outlined in a palette entry. The text is kept as
/// its source.
pub fn from_svg(text: &str) -> Result<Drawing, String> {
    let doc = roxmltree::Document::parse(text).map_err(|e| e.to_string())?;
    let svg = doc.root_element();
    if svg.tag_name().name() != "svg" {
        return Err("no svg element".into());
    }
    let view = svg.attribute("viewBox").ok_or("the svg has no viewBox")?;
    let v: Vec<f64> = view.split_whitespace().map(|t| t.parse::<f64>().map_err(|_| format!("viewBox {view:?}"))).collect::<Result<_, _>>()?;
    let [0.0, 0.0, w, h] = v[..] else { return Err(format!("viewBox {view:?}: not 0 0 width height")) };
    if w.fract() != 0.0 || h.fract() != 0.0 || w < 1.0 || h < 1.0 || w > 65535.0 || h > 65535.0 {
        return Err(format!("viewBox {view:?}: not whole pixels"));
    }
    let mut colors: Vec<(u8, u8)> = Vec::new();
    let mut words = Vec::new();
    let mut add = |p: u8, c: u8| {
        if !colors.contains(&(p, c)) {
            colors.push((p, c));
        }
    };
    for node in svg.descendants().filter(|n| n.is_element()) {
        let name = node.tag_name().name();
        if name == "text" && node.has_attribute("data-words") {
            words.push(words_of(node)?);
            continue;
        }
        let palette = inherited(node, "data-palette")?.unwrap_or(0);
        if let Some(c) = number_of(node, "data-color")? {
            add(palette, c);
        }
        if let Some(c) = number_of(node, "data-stroke")? {
            add(palette, c);
        }
        if let Some(c) = number_of(node, "data-stop-color")? {
            add(palette, c);
        }
        // (A clip's or a definition's shapes are drawn only through what
        // uses them.)
        let defined = node.ancestors().any(|n| matches!(n.tag_name().name(), "defs" | "clipPath" | "mask" | "symbol"));
        if !SHAPES.contains(&name) || defined {
            continue;
        }
        // (A shape's paint is a palette entry's or none, never its own.)
        let paint = |attr: &str, data: &str| -> Result<(), String> {
            let named = inherited(node, data)?.is_some();
            // (A gradient's paint is its stops', each a palette entry's:
            // `data-stop-color`.)
            let none = node.ancestors().filter(|n| n.is_element()).find_map(|n| n.attribute(attr)).is_some_and(|v| v.trim() == "none" || v.trim().starts_with("url("));
            let painted = node.ancestors().filter(|n| n.is_element()).any(|n| n.has_attribute(attr)) || attr == "fill";
            if painted && !named && !none {
                return Err(format!("a {name} with no {data} ({attr}=\"none\" for none)"));
            }
            Ok(())
        };
        paint("fill", "data-color")?;
        paint("stroke", "data-stroke")?;
    }
    let hash = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut h);
        h.finish()
    };
    let d = Drawing { size: [w as u16, h as u16], source: text.into(), hash, colors, words };
    usvg::Tree::from_xmltree(&doc, &options(String::new())).map_err(|e| e.to_string())?;
    Ok(d)
}

/// usvg's options for a drawing, with `style` over its own.
fn options(style: String) -> usvg::Options<'static> {
    usvg::Options { style_sheet: Some(style), ..Default::default() }
}

/// The style sheet that puts `rgb`'s colors into `d`'s shapes (each of its
/// palette entries'), leaves its words out and (`crisp`) draws each pixel
/// whole, in its center's color.
fn style(d: &Drawing, rgb: &dyn Fn(u8, u8) -> [u8; 3], crisp: bool) -> String {
    let mut s = String::from("[data-words] { display: none }\n");
    if crisp {
        s.push_str("* { shape-rendering: crispEdges }\n");
    }
    // (The rows' own after the unnamed one's, so they win.)
    let mut entries = d.colors.clone();
    entries.sort_by_key(|&(p, _)| p != 0);
    for (p, c) in entries {
        let [r, g, b] = rgb(p, c);
        for (data, paint) in [("data-color", "fill"), ("data-stroke", "stroke"), ("data-stop-color", "stop-color")] {
            let rule = format!("{{ {paint}: #{r:02x}{g:02x}{b:02x} }}\n");
            if p == 0 {
                s += &format!("[{data}=\"{c}\"] {rule}");
            } else {
                s += &format!("[data-palette=\"{p}\"][{data}=\"{c}\"] {rule}[data-palette=\"{p}\"] [{data}=\"{c}\"] {rule}");
            }
        }
    }
    s
}

/// `d` drawn over `pixmap` through `transform` (from its frame pixels to
/// the pixmap's), its shapes in the colors `rgb` gives its palette entries
/// (row, index), smoothed or (`crisp`) each pixel whole.
pub fn paint(d: &Drawing, rgb: &dyn Fn(u8, u8) -> [u8; 3], transform: tiny_skia::Transform, crisp: bool, pixmap: &mut tiny_skia::PixmapMut) -> Result<(), String> {
    let tree = usvg::Tree::from_str(&d.source, &options(style(d, rgb, crisp))).map_err(|e| e.to_string())?;
    // (Its frame pixels, not the size the file is viewed at.)
    let view = tree.size();
    let transform = transform.pre_scale(d.size[0] as f32 / view.width(), d.size[1] as f32 / view.height());
    resvg::render(&tree, transform, pixmap);
    Ok(())
}

/// SVG files read into trees ([`Parsed::of`]) and drawings rasterized
/// ([`Parsed::raster`]) in this process: a frontend does both when a pack is
/// loaded (and rasterizes again when its output's scale changes), never
/// while it plays.
pub fn work_done() -> (u64, u64) {
    (PARSES.load(std::sync::atomic::Ordering::Relaxed), RASTERS.load(std::sync::atomic::Ordering::Relaxed))
}

static PARSES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static RASTERS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// A drawing's file read for rasterizing, once: a tree for each three of its
/// palette entries (`Drawing::colors`), those three in pure red, green and
/// blue and every other one black, so one raster of each gives each entry's
/// share of every pixel.
pub struct Parsed {
    trees: Vec<usvg::Tree>,
    size: [u16; 2],
    entries: usize,
}

/// A drawing rasterized at a scale ([`Parsed::raster`]), `w` by `h`
/// output pixels from the drawing's top left: its pixels' coverage
/// (`alpha`, over the box `area` holds: x, y, width, height, the rest
/// clear) and each palette entry's share of them (premultiplied, out of
/// 255, over the entry's own box: a pixel's color is the sum of each
/// entry's color by its share).
pub struct Raster {
    pub w: usize,
    pub h: usize,
    pub area: [usize; 4],
    pub alpha: Vec<u8>,
    pub shares: Vec<Share>,
}

/// A palette entry's share of a raster's pixels, over its box (x, y,
/// width, height, in the raster).
pub struct Share {
    pub area: [usize; 4],
    pub v: Vec<u8>,
}

impl Share {
    /// Its share of pixel (x, y) of the raster.
    pub fn at(&self, x: usize, y: usize) -> u8 {
        let [x0, y0, w, h] = self.area;
        if x < x0 || y < y0 || x >= x0 + w || y >= y0 + h {
            return 0;
        }
        self.v[(y - y0) * w + x - x0]
    }
}

/// The box of the pixels `v` (`w` a row) isn't 0 in, and those pixels.
fn cropped(v: &[u8], w: usize, h: usize) -> ([usize; 4], Vec<u8>) {
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (y, row) in v.chunks_exact(w).enumerate().take(h) {
        let Some(first) = row.iter().position(|&a| a != 0) else { continue };
        let last = row.iter().rposition(|&a| a != 0).unwrap_or(first);
        (x0, y0, x1, y1) = (x0.min(first), y0.min(y), x1.max(last + 1), y1.max(y + 1));
    }
    if x0 >= x1 {
        return ([0, 0, 0, 0], Vec::new());
    }
    let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0));
    for y in y0..y1 {
        out.extend_from_slice(&v[y * w + x0..y * w + x1]);
    }
    ([x0, y0, x1 - x0, y1 - y0], out)
}

impl Parsed {
    pub fn of(d: &Drawing) -> Result<Parsed, String> {
        let basis = [[255, 0, 0], [0, 255, 0], [0, 0, 255]];
        let groups = d.colors.len().div_ceil(3).max(1);
        let mut trees = Vec::with_capacity(groups);
        for g in 0..groups {
            let rgb = |p: u8, c: u8| {
                let k = d.colors.iter().position(|&e| e == (p, c)).unwrap_or(usize::MAX);
                if k / 3 == g { basis[k % 3] } else { [0, 0, 0] }
            };
            PARSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            trees.push(usvg::Tree::from_str(&d.source, &options(style(d, &rgb, false))).map_err(|e| e.to_string())?);
        }
        Ok(Parsed { trees, size: d.size, entries: d.colors.len() })
    }

    /// The drawing rasterized at `scale` output pixels a frame pixel
    /// (`vscale` times that down), smoothed.
    pub fn raster(&self, scale: f32, vscale: f32) -> Raster {
        RASTERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (sx, sy) = (scale, scale * vscale);
        let w = (self.size[0] as f32 * sx).ceil().max(1.0) as usize;
        let h = (self.size[1] as f32 * sy).ceil().max(1.0) as usize;
        let mut alpha = vec![0u8; w * h];
        let mut shares: Vec<Share> = Vec::with_capacity(self.entries);
        for (g, tree) in self.trees.iter().enumerate() {
            let Some(mut pixmap) = tiny_skia::Pixmap::new(w as u32, h as u32) else { break };
            let view = tree.size();
            let t = tiny_skia::Transform::from_scale(sx * self.size[0] as f32 / view.width(), sy * self.size[1] as f32 / view.height());
            resvg::render(tree, t, &mut pixmap.as_mut());
            if g == 0 {
                for (a, p) in alpha.iter_mut().zip(pixmap.data().chunks_exact(4)) {
                    *a = p[3];
                }
            }
            for c in 0..3 {
                if 3 * g + c >= self.entries {
                    break;
                }
                let data = pixmap.data();
                let v: Vec<u8> = data.iter().skip(c).step_by(4).copied().collect();
                let (area, v) = cropped(&v, w, h);
                shares.push(Share { area, v });
            }
        }
        let (area, alpha) = cropped(&alpha, w, h);
        Raster { w, h, area, alpha, shares }
    }

    /// The bytes a raster of it takes.
    pub fn bytes(r: &Raster) -> usize {
        r.alpha.len() + r.shares.iter().map(|s| s.v.len()).sum::<usize>()
    }
}

/// The palette entry `d` shows at each of its pixels' centers (row by row,
/// its size; none: clear).
pub fn labels(d: &Drawing) -> Vec<Option<(u8, u8)>> {
    let (w, h) = (d.size[0] as u32, d.size[1] as u32);
    let Some(mut pixmap) = tiny_skia::Pixmap::new(w, h) else { return Vec::new() };
    // (Each entry its own color: its place in the list.)
    let mark = |k: usize| [(k + 1) as u8, ((k + 1) >> 8) as u8, 0x5A];
    let rgb = |p: u8, c: u8| mark(d.colors.iter().position(|&e| e == (p, c)).unwrap_or(0));
    if paint(d, &rgb, tiny_skia::Transform::identity(), true, &mut pixmap.as_mut()).is_err() {
        return vec![None; (w * h) as usize];
    }
    pixmap
        .pixels()
        .iter()
        .map(|px| {
            if px.alpha() == 0 {
                return None;
            }
            let k = (px.red() as usize | (px.green() as usize) << 8).checked_sub(1)?;
            d.colors.get(k).copied()
        })
        .collect()
}

/// Which of `d`'s pixels its shapes cover wholly (row by row, its size): a
/// pixel an edge crosses is left out, so a tile pixel drawn under the shapes
/// there can't show its square past their cut or curve (the user, of a
/// chatbox tab's light cut over the art's one-pixel step: "why does it look
/// like there's such a weird cut out here?").
pub fn coverage(d: &Drawing) -> Vec<bool> {
    let (w, h) = (d.size[0] as u32, d.size[1] as u32);
    let Some(mut pixmap) = tiny_skia::Pixmap::new(w, h) else { return Vec::new() };
    if paint(d, &|_, _| [0, 0, 0], tiny_skia::Transform::identity(), false, &mut pixmap.as_mut()).is_err() {
        return vec![false; (w * h) as usize];
    }
    pixmap.pixels().iter().map(|px| px.alpha() == 255).collect()
}

/// A `text` element's words: `data-words` the string's key, `data-box` the
/// box its capitals fill (x y width height), `data-color` (and
/// `data-palette`) the face's color, `data-rim` and `data-shadow` the
/// outline's and the shadow's, `data-rim-width` the outline's width (in
/// pixels; 1 if not given), `data-slant` its slant (pixels right for
/// each one up), `data-font` its font (`numbers`, the default, or
/// `lettering`); its `x`, `y`, size and contents are for viewing only.
fn words_of(node: roxmltree::Node) -> Result<Words, String> {
    let key = node.attribute("data-words").unwrap_or_default().to_string();
    let b = node.attribute("data-box").ok_or("a text has no data-box")?;
    let v: Vec<i32> = b.split_whitespace().map(|t| t.parse::<f64>().map(|v| (v * Drawing::UNIT as f64).round() as i32).map_err(|_| format!("data-box {b:?}"))).collect::<Result<_, _>>()?;
    let [x, y, w, h] = v[..] else { return Err(format!("data-box {b:?} isn't four numbers")) };
    let color = number_of(node, "data-color")?.ok_or("a text has no data-color")?;
    let palette = inherited(node, "data-palette")?.unwrap_or(0);
    let slant = match node.attribute("data-slant") {
        Some(t) => (t.parse::<f64>().map_err(|_| format!("data-slant {t:?} isn't a number"))? * 100.0).round() as i16,
        None => 0,
    };
    let rim_width = match node.attribute("data-rim-width") {
        Some(t) => match t.parse::<f64>() {
            Ok(v) if (0.25..=4.0).contains(&v) => (v * 100.0).round() as u16,
            _ => return Err(format!("data-rim-width {t:?} isn't a width from 0.25 to 4 pixels")),
        },
        None => 100,
    };
    let lettering = match node.attribute("data-font") {
        None | Some("numbers") => false,
        Some("lettering") => true,
        Some(t) => return Err(format!("data-font {t:?} isn't numbers or lettering")),
    };
    Ok(Words { key, rect: [x, y, w, h], palette, color, rim: number_of(node, "data-rim")?, rim_width, shadow: number_of(node, "data-shadow")?, slant, lettering })
}

/// `text` (a drawing's SVG) with each element's `fill` and `stroke` that
/// name a palette entry (`data-color`, `data-stroke`) its color in `view`
/// (palette row, index); as it was if it doesn't read.
pub fn recolor(text: &str, view: &dyn Fn(u8, u8) -> u16) -> String {
    let Ok(doc) = roxmltree::Document::parse(text) else { return text.to_string() };
    let hex = |p: u8, c: u8| {
        let [r, g, b] = nettai_assets::rgb(view(p, c));
        format!("#{r:02x}{g:02x}{b:02x}")
    };
    // (Each element's start tag, rewritten.)
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        let palette = inherited(node, "data-palette").ok().flatten().unwrap_or(0);
        let fill = number_of(node, "data-color").ok().flatten().map(|c| ("fill", hex(palette, c)));
        let stroke = number_of(node, "data-stroke").ok().flatten().map(|c| ("stroke", hex(palette, c)));
        let stop = number_of(node, "data-stop-color").ok().flatten().map(|c| ("stop-color", hex(palette, c)));
        if fill.is_none() && stroke.is_none() && stop.is_none() {
            continue;
        }
        let start = node.range().start;
        let Some(end) = start_tag_end(text, start) else { continue };
        let mut tag = text[start..end].to_string();
        for (attr, value) in [fill, stroke, stop].into_iter().flatten() {
            tag = set_attribute(&tag, attr, &value);
        }
        edits.push((start..end, tag));
    }
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (range, tag) in edits {
        out.push_str(&text[at..range.start]);
        out.push_str(&tag);
        at = range.end;
    }
    out.push_str(&text[at..]);
    out
}

/// Where the start tag at `start` ends (past its `>`).
fn start_tag_end(text: &str, start: usize) -> Option<usize> {
    let mut quote = None;
    for (i, ch) in text[start..].char_indices() {
        match (quote, ch) {
            (None, '"' | '\'') => quote = Some(ch),
            (Some(q), _) if ch == q => quote = None,
            (None, '>') => return Some(start + i + 1),
            _ => {}
        }
    }
    None
}

/// A start tag with attribute `name` set to `value` (added after its name
/// where it hasn't it).
fn set_attribute(tag: &str, name: &str, value: &str) -> String {
    let key = format!(" {name}=\"");
    if let Some(i) = tag.find(&key) {
        let from = i + key.len();
        if let Some(len) = tag[from..].find('"') {
            return format!("{}{value}{}", &tag[..from], &tag[from + len..]);
        }
    }
    let name_end = tag[1..].find(|c: char| c.is_whitespace() || c == '/' || c == '>').map_or(tag.len(), |i| i + 1);
    format!("{} {name}=\"{value}\"{}", &tag[..name_end], &tag[name_end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="64" viewBox="0 0 16 8">
<!-- A comment > with a bracket -->
<rect x="0" y="0" width="16" height="8" fill="#000000" data-color="1"/>
<rect x="1" y="1" width="14" height="6" rx="2" data-color="2"/>
<g data-palette="9"><circle cx="8" cy="4" r="2" data-color="3"/></g>
<path d="M0 8 L4 4 L8 8 Z" fill="none" stroke="#000" stroke-width="1" data-stroke="4"/>
<text data-words="window.ok" data-box="1 1 6 4" data-color="5" x="1" y="5">OK</text>
</svg>"##;

    /// A drawing's size, the palette entries it names (a group's row
    /// reaching the shapes in it), its words; drawn at pixels' centers.
    #[test]
    fn a_drawing_reads_and_draws() {
        let d = from_svg(SVG).unwrap();
        assert_eq!(d.size, [16, 8]);
        assert_eq!(d.colors, [(0, 1), (0, 2), (9, 3), (0, 4)]);
        assert_eq!(d.words.len(), 1);
        assert_eq!((d.words[0].key.as_str(), d.words[0].color, d.words[0].rect), ("window.ok", 5, [64, 64, 384, 256]));
        let l = labels(&d);
        assert_eq!(l[0], Some((0, 1)), "the ground in the corner, past the rounded rectangle's curve");
        assert_eq!(l[2 * 16 + 2], Some((0, 2)));
        assert_eq!(l[4 * 16 + 8], Some((9, 3)), "the circle in its group's row");
        assert_eq!(l[4 * 16 + 4], Some((0, 4)), "the outline");
        assert!(coverage(&d).iter().all(|&c| c));
    }

    /// Only the pixels a shape covers wholly count as covered: a pixel a cut
    /// crosses doesn't, even with its center inside.
    #[test]
    fn coverage_is_whole_pixels() {
        let d = from_svg(r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 8 8"><path d="M0 1.5 L1.5 0 L8 0 L8 8 L0 8 Z" data-color="1"/></svg>"##).unwrap();
        let c = coverage(&d);
        assert!(!c[0], "the corner, outside the cut");
        assert!(!c[1] && !c[8], "the cut crosses these, their centers inside");
        assert!(c[2] && c[8 + 1] && c[2 * 8], "whole pixels past the cut");
        assert!(c[8 * 8 - 1]);
    }

    /// The viewing colors set in the file, each entry's in its row.
    #[test]
    fn a_drawing_recolored() {
        let colored = recolor(SVG, &|p, c| if p == 9 { 0x7FFF } else { c as u16 });
        assert!(colored.contains("<circle cx=\"8\" cy=\"4\" r=\"2\" data-color=\"3\" fill=\"#ffffff\"/>") || colored.contains("<circle fill=\"#ffffff\""), "{colored}");
        assert!(colored.contains("<rect x=\"0\" y=\"0\" width=\"16\" height=\"8\" fill=\"#080000\""), "{colored}");
        assert!(colored.contains("stroke=\"#210000\""), "{colored}");
        assert!(colored.contains("<!-- A comment > with a bracket -->"));
        assert_eq!(from_svg(&colored).unwrap().colors, from_svg(SVG).unwrap().colors);
    }

    /// A shape whose paint is its own, an entry past a palette's, or a file
    /// usvg can't read is an error.
    #[test]
    fn a_bad_drawing_is_an_error() {
        let svg = |body: &str| format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 8 8\">{body}</svg>");
        assert!(from_svg(&svg("<rect width=\"1\" height=\"1\"/>")).is_err(), "no color");
        assert!(from_svg(&svg("<rect width=\"1\" height=\"1\" data-color=\"16\"/>")).is_err());
        assert!(from_svg(&svg("<rect width=\"1\" height=\"1\" data-color=\"1\" stroke=\"#000\"/>")).is_err(), "a stroke of its own");
        assert!(from_svg(&svg("<rect width=\"1\" height=\"1\" data-color=\"1\"")).is_err());
        assert!(from_svg("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 8.5 8\"/>").is_err());
        assert!(from_svg(&svg("<rect width=\"1\" height=\"1\" data-color=\"1\"/>")).is_ok());
        let clipped = "<clipPath id=\"c\"><rect width=\"4\" height=\"4\"/></clipPath><rect width=\"8\" height=\"8\" clip-path=\"url(#c)\" data-color=\"1\"/>";
        assert!(from_svg(&svg(clipped)).is_ok(), "a clip's shapes have no colors");
        assert_eq!(labels(&from_svg(&svg(clipped)).unwrap()).iter().filter(|l| l.is_some()).count(), 16);
    }
}
