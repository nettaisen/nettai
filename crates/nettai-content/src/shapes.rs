//! The battle's frames as shapes in a pack (`nettai_assets::shapes`): an SVG
//! file a drawing (`shapes/NAME.svg` beside the pictures), each region a
//! `path` filled even-odd. A path names its color as data: `data-color`,
//! the palette index, and `data-palette`, the palette row where the
//! picture is laid in several (a window map's); its `fill` is that color in
//! the picture's first palette, so the file looks as the picture does
//! where it is opened, and is for viewing only. The paths' steps are SVG's
//! absolute M, L, A and Z, in frame pixels from the drawing's top left; the
//! `viewBox` is the picture's size. What SVG can't say (which cells of a
//! map a drawing stands in for) is in `hud.json` and `custom.json`.

use crate::report::Report;
use nettai_assets::shapes::{CustomShapes, Drawing, GaugeCells, HudShapes, MapDrawing, Region, Step};
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

/// A drawing read from its SVG file's text ([`to_svg`]'s subset: the
/// `viewBox`'s size, each `path`'s `data-color`, `data-palette` and `d` of
/// absolute M, L, A and Z, each ring closed).
pub fn from_svg(text: &str) -> Result<Drawing, String> {
    let svg = element(text, "svg").ok_or("no svg element")?;
    let view = attribute(svg, "viewBox").ok_or("the svg has no viewBox")?;
    let v: Vec<f64> = view.split_whitespace().map(|t| t.parse::<f64>().map_err(|_| format!("viewBox {view:?}"))).collect::<Result<_, _>>()?;
    let [_, _, w, h] = v[..] else { return Err(format!("viewBox {view:?}")) };
    let mut regions = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("<path") {
        let end = rest[i..].find('>').ok_or("a path isn't closed")? + i;
        let tag = &rest[i..=end];
        rest = &rest[end + 1..];
        let color: u8 = attribute(tag, "data-color").ok_or("a path has no data-color")?.parse().map_err(|_| "a data-color isn't a palette index")?;
        let palette: u8 = match attribute(tag, "data-palette") {
            Some(p) => p.parse().map_err(|_| "a data-palette isn't a palette row")?,
            None => 0,
        };
        if color > 15 || palette > 15 {
            return Err(format!("palette entry {palette}:{color} past a palette's 16"));
        }
        let d = attribute(tag, "d").ok_or("a path has no d")?;
        regions.push(Region { palette, color, path: parse_path(d)? });
    }
    Ok(Drawing { size: [w as u16, h as u16], regions })
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
    while let Some(cmd) = it.next() {
        match cmd.as_str() {
            "M" => {
                if open {
                    return Err("a ring isn't closed".into());
                }
                out.push(Step::Move([unit(it.next())?, unit(it.next())?]));
                open = true;
            }
            "L" => out.push(Step::Line([unit(it.next())?, unit(it.next())?])),
            "A" => {
                let radius = unit(it.next())?;
                let ry = unit(it.next())?;
                let _rotation = it.next();
                if ry != radius {
                    return Err("an arc isn't of a circle".into());
                }
                let large = flag(it.next())?;
                let sweep = flag(it.next())?;
                out.push(Step::Arc { radius, large, sweep, to: [unit(it.next())?, unit(it.next())?] });
            }
            "Z" => {
                if !open {
                    return Err("a Z closes no ring".into());
                }
                out.push(Step::Close);
                open = false;
            }
            other => return Err(format!("a path step {other:?} (M, L, A and Z only)")),
        }
        if !matches!(out.first(), Some(Step::Move(_))) {
            return Err("a path starts with a step other than M".into());
        }
    }
    if open {
        return Err("a ring isn't closed".into());
    }
    Ok(out)
}

/// What a pack's files are written into: (path, contents).
pub type Files = Vec<(String, Vec<u8>)>;

/// Write `d` as `shapes/NAME.svg` into `files`; its file's name.
pub fn write(files: &mut Files, name: &str, d: &Drawing, view: &dyn Fn(u8, u8) -> u16) -> String {
    let file = format!("shapes/{name}.svg");
    files.push((file.clone(), to_svg(d, view).into_bytes()));
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

/// A map's drawing in a pack: its file and the cells it stands in for.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MapShapeDoc {
    pub file: String,
    pub cells: Vec<[u8; 2]>,
}

/// The gauge's cells' drawings (`nettai_assets::shapes::GaugeCells`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GaugeCellsDoc {
    pub steps: Vec<String>,
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
    let mut w = |name: &str, d: &Drawing, view: &dyn Fn(u8, u8) -> u16| write(files, name, d, view);
    HudShapesDoc {
        hp_box: s.hp_box.as_ref().map(|d| w("hp-box", d, &hp)),
        gauge_frame: s.gauge_frame.as_ref().map(|m| MapShapeDoc { file: w("gauge-frame", &m.drawing, &gauge), cells: m.cells.clone() }),
        gauge_cells: s.gauge_cells.as_ref().map(|g| GaugeCellsDoc {
            steps: g.steps.iter().enumerate().map(|(k, d)| w(&format!("gauge-step-{k}"), d, &gauge)).collect(),
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
    }
}

/// The HUD's drawings read back.
pub fn read_hud(dir: &Path, prefix: &str, d: &HudShapesDoc, report: &mut Report) -> HudShapes {
    let mut r = |file: &str| read(dir, prefix, file, report).unwrap_or_default();
    HudShapes {
        hp_box: d.hp_box.as_deref().map(&mut r),
        gauge_frame: d.gauge_frame.as_ref().map(|m| MapDrawing { drawing: r(&m.file), cells: m.cells.clone() }),
        gauge_cells: d.gauge_cells.as_ref().map(|g| GaugeCells {
            steps: g.steps.iter().map(|f| r(f)).collect(),
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
    let mut w = |name: &str, d: &Drawing, view: &dyn Fn(u8, u8) -> u16| write(files, name, d, view);
    let maps = |name: &str, m: &[Option<MapDrawing>], w: &mut dyn FnMut(&str, &Drawing, &dyn Fn(u8, u8) -> u16) -> String| -> Vec<Option<MapShapeDoc>> {
        m.iter().enumerate().map(|(k, m)| m.as_ref().map(|m| MapShapeDoc { file: w(&format!("{name}-{k}"), &m.drawing, &window), cells: m.cells.clone() })).collect()
    };
    let window_maps = maps("window", &s.window_maps, &mut w);
    let form_list_maps = maps("form-list", &s.form_list_maps, &mut w);
    CustomShapesDoc {
        window_maps,
        form_list_maps,
        column_cells: s.column_cells.iter().enumerate().map(|(k, d)| w(&format!("column-cell-{k}"), d, &in_palette(frame))).collect(),
        empty_icon: s.empty_icon.as_ref().map(|d| w("empty-icon", d, &in_palette(c.icon_palette))),
        slot_code: s.slot_code.as_ref().map(|d| w("slot-code", d, &in_palette(frame))),
        cursor: s.cursor.iter().enumerate().map(|(k, d)| w(&format!("cursor-{k}"), d, &in_palette(cursor))).collect(),
        form_list_cursor: s.form_list_cursor.iter().enumerate().map(|(k, d)| w(&format!("form-list-cursor-{k}"), d, &in_palette(c.form_list_cursor_palette))).collect(),
        name_bar: s.name_bar.iter().enumerate().map(|(k, d)| w(&format!("name-bar-{k}"), d, &in_palette(hud_palette))).collect(),
    }
}

/// The custom screen's drawings read back.
pub fn read_custom(dir: &Path, prefix: &str, d: &CustomShapesDoc, report: &mut Report) -> CustomShapes {
    let mut r = |file: &str| read(dir, prefix, file, report).unwrap_or_default();
    let maps = |m: &[Option<MapShapeDoc>], r: &mut dyn FnMut(&str) -> Drawing| -> Vec<Option<MapDrawing>> {
        m.iter().map(|m| m.as_ref().map(|m| MapDrawing { drawing: r(&m.file), cells: m.cells.clone() })).collect()
    };
    let window_maps = maps(&d.window_maps, &mut r);
    let form_list_maps = maps(&d.form_list_maps, &mut r);
    CustomShapes {
        window_maps,
        form_list_maps,
        column_cells: d.column_cells.iter().map(|f| r(f)).collect(),
        empty_icon: d.empty_icon.as_deref().map(&mut r),
        slot_code: d.slot_code.as_deref().map(&mut r),
        cursor: d.cursor.iter().map(|f| r(f)).collect(),
        form_list_cursor: d.form_list_cursor.iter().map(|f| r(f)).collect(),
        name_bar: d.name_bar.iter().map(|f| r(f)).collect(),
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
        };
        let svg = to_svg(&d, &|_, c| c as u16 * 0x421);
        assert_eq!(from_svg(&svg), Ok(d));
    }

    /// A ring left open, an unknown step or an entry past a palette's is an
    /// error.
    #[test]
    fn a_bad_path_is_an_error() {
        let svg = |d: &str| format!("<svg viewBox=\"0 0 8 8\"><path data-color=\"1\" d=\"{d}\"/></svg>");
        assert!(from_svg(&svg("M0 0 L1 1")).is_err());
        assert!(from_svg(&svg("M0 0 Q1 1 2 2 Z")).is_err());
        assert!(from_svg("<svg viewBox=\"0 0 8 8\"><path data-color=\"16\" d=\"M0 0 L1 0 L1 1 Z\"/></svg>").is_err());
        assert!(from_svg(&svg("M0 0 L1 0 L1 1 Z")).is_ok());
    }
}
