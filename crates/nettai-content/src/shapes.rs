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

use crate::report::Report;
use nettai_assets::shapes::{CustomShapes, Drawing, GaugeCells, HudShapes, MapDrawing, MapPart, Words};
pub use resvg::tiny_skia;
use resvg::usvg::{self, roxmltree};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
        // (A clip's or a definition's shapes are drawn only through what
        // uses them.)
        let defined = node.ancestors().any(|n| matches!(n.tag_name().name(), "defs" | "clipPath" | "mask" | "symbol"));
        if !SHAPES.contains(&name) || defined {
            continue;
        }
        // (A shape's paint is a palette entry's or none, never its own.)
        let paint = |attr: &str, data: &str| -> Result<(), String> {
            let named = inherited(node, data)?.is_some();
            let none = node.ancestors().filter(|n| n.is_element()).find_map(|n| n.attribute(attr)).is_some_and(|v| v.trim() == "none");
            let painted = node.ancestors().filter(|n| n.is_element()).any(|n| n.has_attribute(attr)) || attr == "fill";
            if painted && !named && !none {
                return Err(format!("a {name} with no {data} ({attr}=\"none\" for none)"));
            }
            Ok(())
        };
        paint("fill", "data-color")?;
        paint("stroke", "data-stroke")?;
    }
    let d = Drawing { size: [w as u16, h as u16], source: text.into(), colors, words };
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
        for (data, paint) in [("data-color", "fill"), ("data-stroke", "stroke")] {
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

/// Which of `d`'s pixels its shapes cover at their centers (row by row,
/// its size).
pub fn coverage(d: &Drawing) -> Vec<bool> {
    labels(d).into_iter().map(|l| l.is_some()).collect()
}

/// A `text` element's words: `data-words` the string's key, `data-box` the
/// box its capitals fill (x y width height), `data-color` (and
/// `data-palette`) the face's color, `data-rim` and `data-shadow` the
/// outline's and the shadow's, `data-slant` its slant (pixels right for
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
    let lettering = match node.attribute("data-font") {
        None | Some("numbers") => false,
        Some("lettering") => true,
        Some(t) => return Err(format!("data-font {t:?} isn't numbers or lettering")),
    };
    Ok(Words { key, rect: [x, y, w, h], palette, color, rim: number_of(node, "data-rim")?, shadow: number_of(node, "data-shadow")?, slant, lettering })
}

/// `text` (a drawing's SVG) with each element's `fill` and `stroke` that
/// name a palette entry (`data-color`, `data-stroke`) its color in `view`
/// (palette row, index); as it was if it doesn't read.
pub fn recolor(text: &str, view: &dyn Fn(u8, u8) -> u16) -> String {
    let Ok(doc) = roxmltree::Document::parse(text) else { return text.to_string() };
    let hex = |p: u8, c: u8| {
        let [r, g, b] = crate::image::rgb(view(p, c));
        format!("#{r:02x}{g:02x}{b:02x}")
    };
    // (Each element's start tag, rewritten.)
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        let palette = inherited(node, "data-palette").ok().flatten().unwrap_or(0);
        let fill = number_of(node, "data-color").ok().flatten().map(|c| ("fill", hex(palette, c)));
        let stroke = number_of(node, "data-stroke").ok().flatten().map(|c| ("stroke", hex(palette, c)));
        if fill.is_none() && stroke.is_none() {
            continue;
        }
        let start = node.range().start;
        let Some(end) = start_tag_end(text, start) else { continue };
        let mut tag = text[start..end].to_string();
        for (attr, value) in [fill, stroke].into_iter().flatten() {
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

/// What a pack's files are written into: (path, contents).
pub type Files = Vec<(String, Vec<u8>)>;

/// Write `d` as `shapes/NAME.svg` into `files`; its file's name.
pub fn write(files: &mut Files, name: &str, d: &Drawing) -> String {
    let file = format!("shapes/{name}.svg");
    // (As it was read: extraction set its fills for viewing.)
    files.push((file.clone(), d.source.as_bytes().to_vec()));
    file
}

/// Read a drawing's SVG file from `dir` (an error said where it isn't
/// read).
pub fn read(dir: &Path, prefix: &str, file: &str, report: &mut Report) -> Option<Drawing> {
    let name = format!("{prefix}/{file}");
    let text = match std::fs::read_to_string(dir.join(file)) {
        Ok(t) => t,
        Err(e) => {
            report.error(&name, e.to_string());
            return None;
        }
    };
    match from_svg(&text) {
        Ok(d) => Some(d),
        Err(e) => {
            report.error(&name, e);
            None
        }
    }
}

/// A map's drawings in a pack: each one's file and the cells it stands in
/// for.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MapShapeDoc {
    pub parts: Vec<MapPartDoc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MapPartDoc {
    pub file: String,
    pub cells: Vec<[u8; 2]>,
}

/// A map's drawings written (each file once however many maps use it).
fn write_map(files: &mut Files, m: &MapDrawing) -> MapShapeDoc {
    MapShapeDoc {
        parts: m
            .parts
            .iter()
            .map(|p| {
                let file = format!("shapes/{}.svg", p.name);
                if !files.iter().any(|(f, _)| *f == file) {
                    write(files, &p.name, &p.drawing);
                }
                MapPartDoc { file, cells: p.cells.clone() }
            })
            .collect(),
    }
}

/// A map's drawings read back.
fn read_map(dir: &Path, prefix: &str, d: &MapShapeDoc, report: &mut Report) -> MapDrawing {
    MapDrawing {
        parts: d
            .parts
            .iter()
            .map(|p| MapPart {
                name: p.file.trim_start_matches("shapes/").trim_end_matches(".svg").to_string(),
                drawing: read(dir, prefix, &p.file, report).unwrap_or_default(),
                cells: p.cells.clone(),
            })
            .collect(),
    }
}

/// The gauge's cells' drawings (`nettai_assets::shapes::GaugeCells`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GaugeCellsDoc {
    pub empty: String,
    pub filled: String,
    pub full: Vec<String>,
    pub l_or_r: Vec<String>,
    pub cut: bool,
}

/// The HUD's drawings' files (`nettai_assets::shapes::HudShapes`).
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct HudShapesDoc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp_box: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gauge_frame: Option<MapShapeDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gauge_cells: Option<GaugeCellsDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count_blank: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count_box: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navi_box: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mugshot_boxes: Vec<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mugshot_frame: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chatbox: Vec<Option<MapShapeDoc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chatbox_arrow: Vec<String>,
}

impl HudShapesDoc {
    pub fn is_empty(&self) -> bool {
        *self == HudShapesDoc::default()
    }
}

/// The custom screen's drawings' files (`nettai_assets::shapes::CustomShapes`).
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct CustomShapesDoc {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub window_maps: Vec<Option<MapShapeDoc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form_list_maps: Vec<Option<MapShapeDoc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_cells: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty_icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_code: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cursor: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form_list_cursor: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub name_bar: Vec<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub buttons: std::collections::BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub pictures: std::collections::BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_name_bar: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regular: Vec<String>,
}

impl CustomShapesDoc {
    pub fn is_empty(&self) -> bool {
        *self == CustomShapesDoc::default()
    }
}

/// The HUD's drawings written.
pub fn write_hud(files: &mut Files, s: &HudShapes) -> HudShapesDoc {
    let gauge_frame = s.gauge_frame.as_ref().map(|m| write_map(files, m));
    let chatbox = s.chatbox.iter().map(|m| m.as_ref().map(|m| write_map(files, m))).collect();
    let mut w = |name: &str, d: &Drawing| write(files, name, d);
    HudShapesDoc {
        hp_box: s.hp_box.as_ref().map(|d| w("hp-box", d)),
        gauge_frame,
        gauge_cells: s.gauge_cells.as_ref().map(|g| GaugeCellsDoc {
            empty: w("gauge-empty", &g.empty),
            filled: w("gauge-filled", &g.filled),
            full: g.full.iter().enumerate().map(|(k, d)| w(&format!("gauge-full-{k}"), d)).collect(),
            l_or_r: g.l_or_r.iter().enumerate().map(|(k, d)| w(&format!("gauge-l-or-r-{k}"), d)).collect(),
            cut: g.cut,
        }),
        count_blank: s.count_blank.as_ref().map(|d| w("count-blank", d)),
        count_box: s.count_box.as_ref().map(|d| w("count-box", d)),
        navi_box: s.navi_box.as_ref().map(|d| w("navi-box", d)),
        mugshot_boxes: s.mugshot_boxes.iter().enumerate().map(|(k, d)| d.as_ref().map(|d| w(&format!("mugshot-box-{k}"), d))).collect(),
        mugshot_frame: s.mugshot_frame.as_ref().map(|d| w("mugshot-frame", d)),
        chatbox,
        chatbox_arrow: s.chatbox_arrow.iter().enumerate().map(|(k, d)| w(&format!("chatbox-arrow-{k}"), d)).collect(),
    }
}

/// The HUD's drawings read back.
pub fn read_hud(dir: &Path, prefix: &str, d: &HudShapesDoc, report: &mut Report) -> HudShapes {
    let gauge_frame = d.gauge_frame.as_ref().map(|m| read_map(dir, prefix, m, report));
    let chatbox = d.chatbox.iter().map(|m| m.as_ref().map(|m| read_map(dir, prefix, m, report))).collect();
    let mut r = |file: &str| read(dir, prefix, file, report).unwrap_or_default();
    HudShapes {
        chatbox,
        hp_box: d.hp_box.as_deref().map(&mut r),
        gauge_frame,
        gauge_cells: d.gauge_cells.as_ref().map(|g| GaugeCells {
            empty: r(&g.empty),
            filled: r(&g.filled),
            full: g.full.iter().map(|f| r(f)).collect(),
            l_or_r: g.l_or_r.iter().map(|f| r(f)).collect(),
            cut: g.cut,
        }),
        count_blank: d.count_blank.as_deref().map(&mut r),
        count_box: d.count_box.as_deref().map(&mut r),
        navi_box: d.navi_box.as_deref().map(&mut r),
        mugshot_boxes: d.mugshot_boxes.iter().map(|f| f.as_deref().map(&mut r)).collect(),
        mugshot_frame: d.mugshot_frame.as_deref().map(&mut r),
        chatbox_arrow: d.chatbox_arrow.iter().map(|f| r(f)).collect(),
    }
}

/// The custom screen's drawings written.
pub fn write_custom(files: &mut Files, s: &CustomShapes) -> CustomShapesDoc {
    let maps = |files: &mut Files, m: &[Option<MapDrawing>]| -> Vec<Option<MapShapeDoc>> { m.iter().map(|m| m.as_ref().map(|m| write_map(files, m))).collect() };
    let window_maps = maps(files, &s.window_maps);
    let form_list_maps = maps(files, &s.form_list_maps);
    let mut w = |name: &str, d: &Drawing| write(files, name, d);
    let set = |w: &mut dyn FnMut(&str, &Drawing) -> String, name: &str, ds: &[Drawing]| -> Vec<String> { ds.iter().enumerate().map(|(k, d)| w(&format!("{name}-{k}"), d)).collect() };
    CustomShapesDoc {
        window_maps,
        form_list_maps,
        column_cells: set(&mut w, "column-cell", &s.column_cells),
        empty_icon: s.empty_icon.as_ref().map(|d| w("empty-icon", d)),
        slot_code: s.slot_code.as_ref().map(|d| w("slot-code", d)),
        cursor: set(&mut w, "cursor", &s.cursor),
        form_list_cursor: set(&mut w, "form-list-cursor", &s.form_list_cursor),
        name_bar: set(&mut w, "name-bar", &s.name_bar),
        buttons: s.buttons.iter().map(|(name, sets)| (name.clone(), set(&mut w, &format!("button-{name}"), sets))).collect(),
        pictures: s.pictures.iter().map(|(name, d)| (name.clone(), w(&format!("picture-{name}"), d))).collect(),
        form_name_bar: s.form_name_bar.as_ref().map(|d| w("form-name-bar", d)),
        regular: set(&mut w, "regular", &s.regular),
    }
}

/// The custom screen's drawings read back.
pub fn read_custom(dir: &Path, prefix: &str, d: &CustomShapesDoc, report: &mut Report) -> CustomShapes {
    let window_maps: Vec<Option<MapDrawing>> = d.window_maps.iter().map(|m| m.as_ref().map(|m| read_map(dir, prefix, m, report))).collect();
    let form_list_maps: Vec<Option<MapDrawing>> = d.form_list_maps.iter().map(|m| m.as_ref().map(|m| read_map(dir, prefix, m, report))).collect();
    let mut r = |file: &str| read(dir, prefix, file, report).unwrap_or_default();
    CustomShapes {
        window_maps,
        form_list_maps,
        column_cells: d.column_cells.iter().map(|f| r(f)).collect(),
        empty_icon: d.empty_icon.as_deref().map(&mut r),
        slot_code: d.slot_code.as_deref().map(&mut r),
        cursor: d.cursor.iter().map(|f| r(f)).collect(),
        form_list_cursor: d.form_list_cursor.iter().map(|f| r(f)).collect(),
        name_bar: d.name_bar.iter().map(|f| r(f)).collect(),
        buttons: d.buttons.iter().map(|(name, sets)| (name.clone(), sets.iter().map(|f| r(f)).collect())).collect(),
        pictures: d.pictures.iter().map(|(name, f)| (name.clone(), r(f))).collect(),
        form_name_bar: d.form_name_bar.as_deref().map(&mut r),
        regular: d.regular.iter().map(|f| r(f)).collect(),
    }
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
