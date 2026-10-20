//! The battle's frames as shapes in a pack (`nettai_assets::shapes`): an SVG
//! file a drawing (`shapes/NAME.svg` beside the pictures), its shapes
//! filled one over another in the file's order, each even-odd: `rect`
//! (with `rx` a rounded one), `circle`, `polygon` and `path` (absolute M,
//! L, H, V, A and Z), in frame pixels from the drawing's top left, the
//! `viewBox` the picture's size. Each names its color as data:
//! `data-color`, the palette index, and `data-palette`, the palette row
//! where the picture is laid in several (a window map's); its `fill` is
//! that color in the picture's first palette, so the file looks as the
//! picture does where it is opened, and is for viewing only. What SVG
//! can't say (which cells of a map a drawing stands in for) is in
//! `hud.json` and `custom.json`.

use crate::report::Report;
use nettai_assets::shapes::{CustomShapes, Drawing, GaugeCells, HudShapes, MapDrawing, MapPart, Region, Step};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::path::Path;

/// A drawing as an SVG file, its regions' fills their colors in `view`
/// (palette row, index).
pub fn to_svg(d: &Drawing, view: &dyn Fn(u8, u8) -> u16) -> String {
    let [w, h] = d.size;
    let mut s = String::new();
    let _ = writeln!(s, "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {w} {h}\">", 8 * w as u32, 8 * h as u32);
    for r in &d.regions {
        let [cr, cg, cb] = crate::image::rgb(view(r.palette, r.color));
        let palette = if r.palette != 0 { format!(" data-palette=\"{}\"", r.palette) } else { String::new() };
        let _ = writeln!(
            s,
            "<path data-color=\"{}\"{palette} fill=\"#{cr:02x}{cg:02x}{cb:02x}\" fill-rule=\"evenodd\" d=\"{}\"/>",
            r.color,
            path_text(&r.path)
        );
    }
    for t in &d.words {
        let [cr, cg, cb] = crate::image::rgb(view(t.palette, t.color));
        let [x, y, w, h] = t.rect;
        let mut extra = String::new();
        if t.palette != 0 {
            let _ = write!(extra, " data-palette=\"{}\"", t.palette);
        }
        if let Some(r) = t.rim {
            let _ = write!(extra, " data-rim=\"{r}\"");
        }
        if let Some(r) = t.shadow {
            let _ = write!(extra, " data-shadow=\"{r}\"");
        }
        let _ = writeln!(
            s,
            "<text data-words=\"{}\" data-box=\"{} {} {} {}\" data-color=\"{}\"{extra} fill=\"#{cr:02x}{cg:02x}{cb:02x}\" x=\"{}\" y=\"{}\" font-size=\"{}\">{}</text>",
            t.key,
            number(x),
            number(y),
            number(w),
            number(h),
            t.color,
            number(x),
            number(y + h),
            number(h * 4 / 3),
            t.key
        );
    }
    s.push_str("</svg>\n");
    s
}

fn number(v: i32) -> String {
    let f = v as f64 / Drawing::UNIT as f64;
    let mut t = format!("{f:.6}");
    while t.ends_with('0') {
        t.pop();
    }
    if t.ends_with('.') {
        t.pop();
    }
    if t == "-0" { "0".into() } else { t }
}

fn path_text(path: &[Step]) -> String {
    let mut s = String::new();
    for step in path {
        if !s.is_empty() {
            s.push(' ');
        }
        match *step {
            Step::Move([x, y]) => {
                let _ = write!(s, "M{} {}", number(x), number(y));
            }
            Step::Line([x, y]) => {
                let _ = write!(s, "L{} {}", number(x), number(y));
            }
            Step::Arc { radius, large, sweep, to: [x, y] } => {
                let r = number(radius);
                let _ = write!(s, "A{r} {r} 0 {} {} {} {}", large as u8, sweep as u8, number(x), number(y));
            }
            Step::Close => s.push('Z'),
        }
    }
    s
}

/// A drawing read from its SVG file's text: the `viewBox`'s size and each
/// shape (`rect`, `circle`, `polygon`, `path`) in order, with its
/// `data-color` and `data-palette`; the text kept as its source.
pub fn from_svg(text: &str) -> Result<Drawing, String> {
    let svg = element(text, "svg").ok_or("no svg element")?;
    let view = attribute(svg, "viewBox").ok_or("the svg has no viewBox")?;
    let v: Vec<f64> = view.split_whitespace().map(|t| t.parse::<f64>().map_err(|_| format!("viewBox {view:?}"))).collect::<Result<_, _>>()?;
    let [_, _, w, h] = v[..] else { return Err(format!("viewBox {view:?}")) };
    let mut regions = Vec::new();
    let mut words = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find('<') {
        let end = rest[i..].find('>').ok_or("a tag isn't closed")? + i;
        let tag = &rest[i..=end];
        rest = &rest[end + 1..];
        let name: String = tag[1..].chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        if name == "text" {
            words.push(words_of(tag)?);
            continue;
        }
        let path = match name.as_str() {
            "rect" => rect_path(tag)?,
            "circle" => circle_path(tag)?,
            "polygon" => polygon_path(tag)?,
            "path" => parse_path(attribute(tag, "d").ok_or("a path has no d")?)?,
            _ => continue,
        };
        let color: u8 = attribute(tag, "data-color").ok_or_else(|| format!("a {name} has no data-color"))?.parse().map_err(|_| "a data-color isn't a palette index")?;
        let palette: u8 = match attribute(tag, "data-palette") {
            Some(p) => p.parse().map_err(|_| "a data-palette isn't a palette row")?,
            None => 0,
        };
        if color > 15 || palette > 15 {
            return Err(format!("palette entry {palette}:{color} past a palette's 16"));
        }
        regions.push(Region { palette, color, path });
    }
    Ok(Drawing { size: [w as u16, h as u16], regions, words, source: text.to_string() })
}

/// A `text` element's words: `data-words` the string's key, `data-box` the
/// box its capitals fill (x y width height), `data-color` (and
/// `data-palette`) the face's color, `data-rim` and `data-shadow` the
/// outline's and the shadow's; its `x`, `y`, size and contents are for
/// viewing only.
fn words_of(tag: &str) -> Result<nettai_assets::shapes::Words, String> {
    let key = attribute(tag, "data-words").ok_or("a text has no data-words")?.to_string();
    let b = attribute(tag, "data-box").ok_or("a text has no data-box")?;
    let v: Vec<i32> = b.split_whitespace().map(|t| t.parse::<f64>().map(|v| (v * Drawing::UNIT as f64).round() as i32).map_err(|_| format!("data-box {b:?}"))).collect::<Result<_, _>>()?;
    let [x, y, w, h] = v[..] else { return Err(format!("data-box {b:?} isn't four numbers")) };
    let index = |name: &str| -> Result<Option<u8>, String> {
        match attribute(tag, name) {
            Some(t) => match t.parse::<u8>() {
                Ok(c) if c <= 15 => Ok(Some(c)),
                _ => Err(format!("{name} {t:?} isn't a palette index")),
            },
            None => Ok(None),
        }
    };
    let color = index("data-color")?.ok_or("a text has no data-color")?;
    let palette = index("data-palette")?.unwrap_or(0);
    Ok(nettai_assets::shapes::Words { key, rect: [x, y, w, h], palette, color, rim: index("data-rim")?, shadow: index("data-shadow")? })
}

fn unit_of(tag: &str, name: &str) -> Result<i32, String> {
    let t = attribute(tag, name).ok_or_else(|| format!("no {name}"))?;
    let v: f64 = t.parse().map_err(|_| format!("{name} {t:?} isn't a number"))?;
    Ok((v * Drawing::UNIT as f64).round() as i32)
}

/// A `rect`'s path (`rx`: its corners rounded).
fn rect_path(tag: &str) -> Result<Vec<Step>, String> {
    let (x, y, w, h) = (unit_of(tag, "x").unwrap_or(0), unit_of(tag, "y").unwrap_or(0), unit_of(tag, "width")?, unit_of(tag, "height")?);
    let r = unit_of(tag, "rx").unwrap_or(0).min(w / 2).min(h / 2);
    if r <= 0 {
        return Ok(vec![Step::Move([x, y]), Step::Line([x + w, y]), Step::Line([x + w, y + h]), Step::Line([x, y + h]), Step::Close]);
    }
    let arc = |to: [i32; 2]| Step::Arc { radius: r, large: false, sweep: true, to };
    Ok(vec![
        Step::Move([x + r, y]),
        Step::Line([x + w - r, y]),
        arc([x + w, y + r]),
        Step::Line([x + w, y + h - r]),
        arc([x + w - r, y + h]),
        Step::Line([x + r, y + h]),
        arc([x, y + h - r]),
        Step::Line([x, y + r]),
        arc([x + r, y]),
        Step::Close,
    ])
}

/// A `circle`'s path: two half circles.
fn circle_path(tag: &str) -> Result<Vec<Step>, String> {
    let (cx, cy, r) = (unit_of(tag, "cx")?, unit_of(tag, "cy")?, unit_of(tag, "r")?);
    let arc = |to: [i32; 2]| Step::Arc { radius: r, large: false, sweep: true, to };
    Ok(vec![Step::Move([cx + r, cy]), arc([cx - r, cy]), arc([cx + r, cy]), Step::Close])
}

/// A `polygon`'s path.
fn polygon_path(tag: &str) -> Result<Vec<Step>, String> {
    let points = attribute(tag, "points").ok_or("a polygon has no points")?;
    let v: Vec<i32> = points
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<f64>().map(|v| (v * Drawing::UNIT as f64).round() as i32).map_err(|_| format!("{t:?} isn't a number")))
        .collect::<Result<_, _>>()?;
    if v.len() < 6 || !v.len().is_multiple_of(2) {
        return Err("a polygon needs three points or more".into());
    }
    let mut out = vec![Step::Move([v[0], v[1]])];
    out.extend(v[2..].chunks(2).map(|c| Step::Line([c[0], c[1]])));
    out.push(Step::Close);
    Ok(out)
}

/// `text` (a drawing's SVG) with each shape's `fill` its color in `view`.
pub fn recolor(text: &str, view: &dyn Fn(u8, u8) -> u16) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find('<') {
        out.push_str(&rest[..i]);
        let Some(end) = rest[i..].find('>').map(|e| e + i) else { break };
        let tag = &rest[i..=end];
        rest = &rest[end + 1..];
        let color = attribute(tag, "data-color").and_then(|c| c.parse::<u8>().ok());
        let palette = attribute(tag, "data-palette").and_then(|c| c.parse::<u8>().ok()).unwrap_or(0);
        match color {
            Some(c) => {
                let [r, g, b] = crate::image::rgb(view(palette, c));
                let fill = format!("#{r:02x}{g:02x}{b:02x}");
                let tag = match attribute(tag, "fill") {
                    Some(old) => tag.replacen(&format!(" fill=\"{old}\""), &format!(" fill=\"{fill}\""), 1),
                    None => tag.replacen(" data-color=", &format!(" fill=\"{fill}\" data-color="), 1),
                };
                out.push_str(&tag);
            }
            None => out.push_str(tag),
        }
    }
    out.push_str(rest);
    out
}

fn element<'t>(text: &'t str, name: &str) -> Option<&'t str> {
    let i = text.find(&format!("<{name}"))?;
    let end = text[i..].find('>')? + i;
    Some(&text[i..=end])
}

fn attribute<'t>(tag: &'t str, name: &str) -> Option<&'t str> {
    let key = format!(" {name}=\"");
    let i = tag.find(&key)? + key.len();
    let end = tag[i..].find('"')? + i;
    Some(&tag[i..end])
}

/// A path's steps from its `d` (absolute M, L, A, Z; each ring closed).
fn parse_path(d: &str) -> Result<Vec<Step>, String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
            tokens.push(ch.to_string());
        } else if ch == ' ' || ch == ',' || ch == '\n' || ch == '\t' {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
        } else if ch == '-' && !cur.is_empty() {
            tokens.push(std::mem::take(&mut cur));
            cur.push(ch);
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    let mut it = tokens.into_iter().peekable();
    let unit = |t: Option<String>| -> Result<i32, String> {
        let t = t.ok_or("a path ends in the middle of a step")?;
        let v: f64 = t.parse().map_err(|_| format!("{t:?} isn't a number"))?;
        Ok((v * Drawing::UNIT as f64).round() as i32)
    };
    let flag = |t: Option<String>| -> Result<bool, String> {
        match t.as_deref() {
            Some("0") => Ok(false),
            Some("1") => Ok(true),
            other => Err(format!("{other:?} isn't an arc's flag")),
        }
    };
    let mut out = Vec::new();
    let mut open = false;
    let (mut last, mut start) = ([0, 0], [0, 0]);
    while let Some(cmd) = it.next() {
        match cmd.as_str() {
            "M" => {
                if open {
                    out.push(Step::Close);
                }
                let p = [unit(it.next())?, unit(it.next())?];
                last = p;
                start = p;
                out.push(Step::Move(p));
                open = true;
            }
            "L" => {
                let p = [unit(it.next())?, unit(it.next())?];
                last = p;
                out.push(Step::Line(p));
            }
            "H" => {
                let p = [unit(it.next())?, last[1]];
                last = p;
                out.push(Step::Line(p));
            }
            "V" => {
                let p = [last[0], unit(it.next())?];
                last = p;
                out.push(Step::Line(p));
            }
            "A" => {
                let radius = unit(it.next())?;
                let ry = unit(it.next())?;
                let _rotation = it.next();
                if ry != radius {
                    return Err("an arc isn't of a circle".into());
                }
                let large = flag(it.next())?;
                let sweep = flag(it.next())?;
                let to = [unit(it.next())?, unit(it.next())?];
                last = to;
                out.push(Step::Arc { radius, large, sweep, to });
            }
            "Z" => {
                if !open {
                    return Err("a Z closes no ring".into());
                }
                out.push(Step::Close);
                last = start;
                open = false;
            }
            other => return Err(format!("a path step {other:?} (M, L, H, V, A and Z only)")),
        }
        if !matches!(out.first(), Some(Step::Move(_))) {
            return Err("a path starts with a step other than M".into());
        }
    }
    if open {
        out.push(Step::Close);
    }
    Ok(out)
}

/// What a pack's files are written into: (path, contents).
pub type Files = Vec<(String, Vec<u8>)>;

/// Write `d` as `shapes/NAME.svg` into `files`; its file's name.
pub fn write(files: &mut Files, name: &str, d: &Drawing, view: &dyn Fn(u8, u8) -> u16) -> String {
    let file = format!("shapes/{name}.svg");
    // (A drawing read from its file is written as it was read: extraction
    // set its fills.)
    let text = if d.source.is_empty() { to_svg(d, view) } else { d.source.clone() };
    files.push((file.clone(), text.into_bytes()));
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
fn write_map(files: &mut Files, m: &MapDrawing, view: &dyn Fn(u8, u8) -> u16) -> MapShapeDoc {
    MapShapeDoc {
        parts: m
            .parts
            .iter()
            .map(|p| {
                let file = format!("shapes/{}.svg", p.name);
                if !files.iter().any(|(f, _)| *f == file) {
                    write(files, &p.name, &p.drawing, view);
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
}

impl CustomShapesDoc {
    pub fn is_empty(&self) -> bool {
        *self == CustomShapesDoc::default()
    }
}

/// The colors a drawing is viewed in: one palette's.
pub fn in_palette(p: nettai_assets::Palette) -> impl Fn(u8, u8) -> u16 {
    move |_, c| p[c as usize & 15]
}

/// The HUD's drawings written (their colors for viewing: the HP box's and
/// the gauge's palettes, the first face's).
pub fn write_hud(files: &mut Files, s: &HudShapes, h: &nettai_assets::Hud) -> HudShapesDoc {
    let hp = in_palette(h.hp_palettes[0]);
    let gauge = in_palette(h.gauge_palette);
    let face = in_palette(h.mugshots.first().map(|m| m.1).unwrap_or_default());
    let gauge_frame = s.gauge_frame.as_ref().map(|m| write_map(files, m, &gauge));
    let chat = in_palette(h.chatbox.palette);
    let chatbox = s.chatbox.iter().map(|m| m.as_ref().map(|m| write_map(files, m, &chat))).collect();
    let mut w = |name: &str, d: &Drawing, view: &dyn Fn(u8, u8) -> u16| write(files, name, d, view);
    HudShapesDoc {
        hp_box: s.hp_box.as_ref().map(|d| w("hp-box", d, &hp)),
        gauge_frame,
        gauge_cells: s.gauge_cells.as_ref().map(|g| GaugeCellsDoc {
            empty: w("gauge-empty", &g.empty, &gauge),
            filled: w("gauge-filled", &g.filled, &gauge),
            full: g.full.iter().enumerate().map(|(k, d)| w(&format!("gauge-full-{k}"), d, &gauge)).collect(),
            l_or_r: g.l_or_r.iter().enumerate().map(|(k, d)| w(&format!("gauge-l-or-r-{k}"), d, &gauge)).collect(),
            cut: g.cut,
        }),
        count_blank: s.count_blank.as_ref().map(|d| w("count-blank", d, &face)),
        count_box: s.count_box.as_ref().map(|d| w("count-box", d, &face)),
        navi_box: s.navi_box.as_ref().map(|d| w("navi-box", d, &in_palette(h.navi_mugshots.first().map(|m| m.palettes[0]).unwrap_or_default()))),
        mugshot_boxes: s
            .mugshot_boxes
            .iter()
            .enumerate()
            .map(|(k, d)| d.as_ref().map(|d| w(&format!("mugshot-box-{k}"), d, &in_palette(h.mugshots.get(k).map(|m| m.1).unwrap_or_default()))))
            .collect(),
        mugshot_frame: s.mugshot_frame.as_ref().map(|d| w("mugshot-frame", d, &face)),
        chatbox,
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
    }
}

/// The custom screen's drawings written (their colors for viewing: the
/// window's first palette for its frame, the other rows as the screen sets
/// them at rest).
pub fn write_custom(files: &mut Files, s: &CustomShapes, c: &nettai_assets::CustomScreen, hud_palette: nettai_assets::Palette) -> CustomShapesDoc {
    let frame = c.frame_palettes.first().copied().unwrap_or_default();
    let window = move |p: u8, k: u8| {
        let row = match p {
            11 => c.icon_palette,
            12 => c.gray_palette,
            13 => hud_palette,
            14 | 15 => c.other_palette,
            _ => frame,
        };
        row[k as usize & 15]
    };
    let cursor = c.cursor_palette.or_else(|| c.emblems.first().map(|e| e.palette)).unwrap_or_default();
    let maps = |files: &mut Files, m: &[Option<MapDrawing>]| -> Vec<Option<MapShapeDoc>> { m.iter().map(|m| m.as_ref().map(|m| write_map(files, m, &window))).collect() };
    let window_maps = maps(files, &s.window_maps);
    let form_list_maps = maps(files, &s.form_list_maps);
    let mut w = |name: &str, d: &Drawing, view: &dyn Fn(u8, u8) -> u16| write(files, name, d, view);
    CustomShapesDoc {
        window_maps,
        form_list_maps,
        column_cells: s.column_cells.iter().enumerate().map(|(k, d)| w(&format!("column-cell-{k}"), d, &in_palette(frame))).collect(),
        empty_icon: s.empty_icon.as_ref().map(|d| w("empty-icon", d, &in_palette(c.icon_palette))),
        slot_code: s.slot_code.as_ref().map(|d| w("slot-code", d, &in_palette(frame))),
        cursor: s.cursor.iter().enumerate().map(|(k, d)| w(&format!("cursor-{k}"), d, &in_palette(cursor))).collect(),
        form_list_cursor: s.form_list_cursor.iter().enumerate().map(|(k, d)| w(&format!("form-list-cursor-{k}"), d, &in_palette(c.form_list_cursor_palette))).collect(),
        name_bar: s.name_bar.iter().enumerate().map(|(k, d)| w(&format!("name-bar-{k}"), d, &in_palette(hud_palette))).collect(),
        buttons: s.buttons.iter().map(|(name, sets)| (name.clone(), sets.iter().enumerate().map(|(k, d)| w(&format!("button-{name}-{k}"), d, &in_palette(frame))).collect())).collect(),
        pictures: s.pictures.iter().map(|(name, d)| (name.clone(), w(&format!("picture-{name}"), d, &in_palette(c.pictures.ok.palette)))).collect(),
        form_name_bar: s.form_name_bar.as_ref().map(|d| w("form-name-bar", d, &in_palette(c.versioned.base.form_name_palettes.first().copied().unwrap_or_default()))),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A drawing written as SVG reads back the same.
    #[test]
    fn a_drawing_reads_back() {
        let u = Drawing::UNIT as i32;
        let d = Drawing {
            size: [8, 16],
            regions: vec![
                Region { palette: 0, color: 3, path: vec![Step::Move([0, 0]), Step::Line([8 * u, 0]), Step::Line([8 * u, 16 * u]), Step::Close] },
                Region {
                    palette: 9,
                    color: 12,
                    path: vec![Step::Move([u / 2, u]), Step::Arc { radius: 3 * u / 2, large: false, sweep: true, to: [2 * u, -u / 4] }, Step::Line([u / 8, 3]), Step::Close],
                },
            ],
            words: Vec::new(),
            source: String::new(),
        };
        let back = from_svg(&to_svg(&d, &|_, c| c as u16 * 0x421)).unwrap();
        assert_eq!((back.size, back.regions), (d.size, d.regions));
    }

    /// The shapes a hand-drawn file uses: a rectangle, a rounded one, a
    /// circle, a polygon, a path with H and V; each recolored in a palette.
    #[test]
    fn hand_drawn_shapes_read() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 8">
<rect x="0" y="0" width="16" height="8" fill="#000000" data-color="1"/>
<rect x="1" y="1" width="14" height="6" rx="2" data-color="2"/>
<circle cx="8" cy="4" r="2" data-color="3" data-palette="9"/>
<polygon points="0,8 4,4 8,8" data-color="4"/>
<path d="M1 1 H3 V3 Z" data-color="5"/>
</svg>"##;
        let d = from_svg(svg).unwrap();
        assert_eq!(d.size, [16, 8]);
        assert_eq!(d.regions.iter().map(|r| (r.palette, r.color)).collect::<Vec<_>>(), [(0, 1), (0, 2), (9, 3), (0, 4), (0, 5)]);
        assert!(nettai_assets::shapes::inside(&d.regions[2].rings(), [8.0, 4.0]));
        assert!(!nettai_assets::shapes::inside(&d.regions[1].rings(), [1.2, 1.2]), "a rounded corner is off the corner");
        let colored = recolor(svg, &|p, c| if p == 9 { 0x7FFF } else { c as u16 });
        assert!(colored.contains("fill=\"#ffffff\" data-color=\"3\""), "{colored}");
        assert!(colored.contains("<rect x=\"0\" y=\"0\" width=\"16\" height=\"8\" fill=\"#080000\""), "{colored}");
    }

    /// An unknown step or an entry past a palette's is an error.
    #[test]
    fn a_bad_shape_is_an_error() {
        let svg = |d: &str| format!("<svg viewBox=\"0 0 8 8\"><path data-color=\"1\" d=\"{d}\"/></svg>");
        assert!(from_svg(&svg("M0 0 Q1 1 2 2 Z")).is_err());
        assert!(from_svg("<svg viewBox=\"0 0 8 8\"><path data-color=\"16\" d=\"M0 0 L1 0 L1 1 Z\"/></svg>").is_err());
        assert!(from_svg("<svg viewBox=\"0 0 8 8\"><rect width=\"1\" height=\"1\"/></svg>").is_err());
        assert!(from_svg(&svg("M0 0 L1 0 L1 1 Z")).is_ok());
    }
}
