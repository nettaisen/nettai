//! The field's panels and the battle backgrounds.
//!
//! `graphics/field/`: `tiles.png` (the panel tiles; the image's palette
//! rows are the background palette slots, rows 1..=8 the panel palettes)
//! and `field.json` (the panel types it draws, by the engine's names;
//! panel blocks, edges and highlights as map entries
//! `tile:palette[:flip]`, and the cycling panel palettes).
//!
//! `graphics/backgrounds/NN/`: `tiles.png`, `map.tmj` (a Tiled map of the
//! tile map: gid = tile number + 1, flips as Tiled's flip bits),
//! `background.json` (scroll speed, animations) and, for tile
//! animations, `anim-K.png` (one block of tiles per frame).
//!
//! In both images, tile number n sits at place n (tiles below the first
//! loaded one are blank), so map entries and image positions agree.

use crate::report::Report;
use crate::sprite::read_json;
use crate::tiles::{self, Layout, TileImage};
use nettai_assets::{AnimTarget, Background, Field, GfxAnim, GfxAnimFrame, MapEntry, Palette, PaletteAnim, Tiles};
use nettai_battle::field::PanelType;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const FIELD_FORMAT: &str = "nettai-content/field";
pub const BACKGROUND_FORMAT: &str = "nettai-content/background";
pub const VERSION: u32 = 1;

const COLUMNS: u32 = 16;

/// Tiles `first..` padded with blank tiles in front, so tile number n is
/// image tile n.
fn from_zero(tiles: &Tiles, first: u16) -> Tiles {
    let mut pixels = vec![0; first as usize * Tiles::TILE];
    pixels.extend_from_slice(&tiles.pixels);
    Tiles { pixels }
}

fn after(tiles: Tiles, first: u16, report: &mut Report, file: &str) -> Tiles {
    let skip = first as usize * Tiles::TILE;
    if tiles.pixels[..skip.min(tiles.pixels.len())].iter().any(|&v| v != 0) {
        report.warn(file, format!("tiles before tile {first} aren't loaded by the game; what is drawn there is ignored"));
    }
    Tiles { pixels: tiles.pixels[skip.min(tiles.pixels.len())..].to_vec() }
}

/// A gray ramp for images without a palette of their own.
fn gray() -> Palette {
    std::array::from_fn(|i| {
        let v = (i as u16 * 2).min(31);
        v | v << 5 | v << 10
    })
}

/// The palette row each tile number is first drawn with.
fn first_rows(n: usize, entries: impl Iterator<Item = MapEntry>, default: u8) -> Vec<u8> {
    let mut rows = vec![None; n];
    for e in entries {
        if let Some(r) = rows.get_mut(e.tile as usize)
            && r.is_none()
        {
            *r = Some(e.palette);
        }
    }
    rows.into_iter().map(|r| r.unwrap_or(default)).collect()
}

// ---- Field -------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct FieldDoc {
    pub format: String,
    pub version: u32,
    pub tiles: TileImage,
    pub first_tile: u16,
    /// Palette rows 1..=8 of `tiles.png` are the panel palettes; this is
    /// the first row and how many.
    pub palette_rows: [u8; 2],
    pub palette_anims: Vec<PaletteAnimDoc>,
    /// The panel types the field draws, by the engine's names, in the
    /// order of their blocks (docs/design/rules-in-luau.md §7.4).
    pub panel_types: Vec<PanelType>,
    /// 5x3 blocks by 6 * the type's place in `panel_types` + 3 * owner +
    /// row - 1.
    pub panels: Vec<Vec<String>>,
    pub front_edges: Vec<Vec<String>>,
    /// One or two: by highlight - 1.
    pub highlights: Vec<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaletteAnimDoc {
    /// The background palette slot it replaces.
    pub slot: u8,
    /// Ticks until the first switch.
    pub initial_timer: u8,
    pub frames: Vec<PaletteFrameDoc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaletteFrameDoc {
    pub ticks: u8,
    pub colors: Vec<String>,
}

pub fn export_field(f: &Field) -> Vec<(String, Vec<u8>)> {
    let all = from_zero(&f.tiles, f.first_tile);
    let mut rows = vec![gray(); 16];
    for (i, p) in f.palettes.iter().enumerate() {
        rows[f.first_palette as usize + i] = *p;
    }
    let entries = f.panels.iter().flatten().chain(f.front_edges.iter().flatten()).chain(f.highlights.iter().flatten());
    let row_of = first_rows(all.len(), entries.copied(), f.first_palette);
    let (png, image) = tiles::export_image(
        "tiles.png",
        &all,
        Layout::Grid { columns: COLUMNS },
        &rows,
        [f.first_palette as usize, f.palettes.len()],
        |i| row_of[i],
    );
    let texts = |b: &[MapEntry]| b.iter().map(tiles::entry_text).collect::<Vec<_>>();
    let doc = FieldDoc {
        format: FIELD_FORMAT.into(),
        version: VERSION,
        tiles: image,
        first_tile: f.first_tile,
        palette_rows: [f.first_palette, f.palettes.len() as u8],
        palette_anims: f
            .palette_anims
            .iter()
            .map(|a| PaletteAnimDoc {
                slot: a.slot,
                initial_timer: a.initial_timer,
                frames: a.frames.iter().map(|(p, t)| PaletteFrameDoc { ticks: *t, colors: tiles::palette_text(p) }).collect(),
            })
            .collect(),
        panel_types: f.panel_types.iter().filter_map(|&t| PanelType::ALL.get(t as usize).copied()).collect(),
        panels: f.panels.iter().map(|b| texts(b)).collect(),
        front_edges: f.front_edges.iter().map(|b| texts(b)).collect(),
        highlights: f.highlights.iter().map(|b| texts(b)).collect(),
    };
    vec![("tiles.png".into(), png), ("field.json".into(), json_lines(&doc))]
}

pub fn import_field(dir: &Path, prefix: &str, report: &mut Report) -> Option<Field> {
    let name = format!("{prefix}/field.json");
    let doc: FieldDoc = read_json(&dir.join("field.json"), &name, report)?;
    if doc.format != FIELD_FORMAT || doc.version != VERSION {
        report.error(&name, format!("not a {FIELD_FORMAT} file of version {VERSION} (extract the pack again)"));
        return None;
    }
    let (all, palettes) = tiles::import_image(dir, prefix, &doc.tiles, report)?;
    let tiles = after(all, doc.first_tile, report, &format!("{prefix}/{}", doc.tiles.file));
    let blocks = |v: &[Vec<String>], n: usize, report: &mut Report| -> Vec<Vec<MapEntry>> {
        v.iter()
            .map(|b| {
                if b.len() != n {
                    report.error(&name, format!("a block has {n} entries, not {}", b.len()));
                }
                (0..n)
                    .map(|i| {
                        b.get(i).map(|s| tiles::parse_entry(s)).unwrap_or(Ok(MapEntry::default())).unwrap_or_else(|e| {
                            report.error(&name, e);
                            MapEntry::default()
                        })
                    })
                    .collect()
            })
            .collect()
    };
    let panels: Vec<[MapEntry; 15]> =
        blocks(&doc.panels, 15, report).into_iter().map(|b| b.try_into().unwrap()).collect();
    let edges = blocks(&doc.front_edges, 5, report);
    let lights = blocks(&doc.highlights, 15, report);
    if edges.len() != 2 || !(1..=2).contains(&lights.len()) {
        report.error(&name, "there are two front edges, and one highlight or two");
        return None;
    }
    if 6 * doc.panel_types.len() != panels.len() {
        report.error(&name, format!("{} panel types need {} blocks, not {}", doc.panel_types.len(), 6 * doc.panel_types.len(), panels.len()));
        return None;
    }
    let panel_types: Vec<u8> = doc.panel_types.iter().map(|&t| t as u8).collect();
    let palette_anims = doc
        .palette_anims
        .iter()
        .map(|a| PaletteAnim {
            slot: a.slot,
            initial_timer: a.initial_timer,
            frames: a.frames.iter().map(|f| (tiles::parse_palette(&f.colors, report, &name), f.ticks)).collect(),
        })
        .collect();
    Some(Field {
        tiles,
        first_tile: doc.first_tile,
        palettes,
        first_palette: doc.palette_rows[0],
        palette_anims,
        panel_types,
        panels,
        front_edges: [edges[0].clone().try_into().unwrap(), edges[1].clone().try_into().unwrap()],
        highlights: lights.into_iter().map(|b| b.try_into().unwrap()).collect(),
    })
}

/// Pretty JSON, but arrays of scalars on one line.
pub(crate) fn json_lines<T: Serialize>(v: &T) -> Vec<u8> {
    let value = serde_json::to_value(v).unwrap();
    let mut s = String::new();
    write_value(&mut s, &value, 0);
    s.push('\n');
    s.into_bytes()
}

fn write_value(s: &mut String, v: &serde_json::Value, depth: usize) {
    use serde_json::Value;
    let pad = "  ".repeat(depth + 1);
    match v {
        Value::Array(items) if items.iter().all(|x| !x.is_array() && !x.is_object()) => {
            s.push_str(&serde_json::to_string(v).unwrap());
        }
        Value::Array(items) => {
            s.push_str("[\n");
            for (i, x) in items.iter().enumerate() {
                s.push_str(&pad);
                write_value(s, x, depth + 1);
                s.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
            }
            s.push_str(&"  ".repeat(depth));
            s.push(']');
        }
        Value::Object(map) if map.values().all(|x| !x.is_object() && !x.is_array()) && map.len() <= 4 => {
            s.push_str(&serde_json::to_string(v).unwrap());
        }
        Value::Object(map) => {
            s.push_str("{\n");
            for (i, (k, x)) in map.iter().enumerate() {
                s.push_str(&pad);
                s.push_str(&serde_json::to_string(k).unwrap());
                s.push_str(": ");
                write_value(s, x, depth + 1);
                s.push_str(if i + 1 < map.len() { ",\n" } else { "\n" });
            }
            s.push_str(&"  ".repeat(depth));
            s.push('}');
        }
        _ => s.push_str(&serde_json::to_string(v).unwrap()),
    }
}

// ---- Backgrounds -----------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct BackgroundDoc {
    pub format: String,
    pub version: u32,
    /// The background's id (battle settings' background byte).
    pub id: u8,
    pub tiles: TileImage,
    pub first_tile: u16,
    /// The Tiled map of the tile map.
    pub map: String,
    /// Whether the background brings its own palette 0 (row 0 of the
    /// tiles image); otherwise that row is only for viewing.
    pub palette: bool,
    /// Scroll per frame in 1/16 pixel.
    pub scroll: [i32; 2],
    pub anims: Vec<AnimDoc>,
    /// The region whose ROMs the picture comes from, where another
    /// region's have another (`Background::region`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnimDoc {
    /// What each frame replaces: tiles `[first, count]` or palettes
    /// `[first, count]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiles: Option<[u16; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palettes: Option<[u8; 2]>,
    /// The frame playback returns to after the last (none: it stays on
    /// the last).
    pub repeat_from: Option<usize>,
    /// Tile frames: one block of `count` tiles per frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<TileImage>,
    pub frames: Vec<AnimFrameDoc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnimFrameDoc {
    pub ticks: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palettes: Vec<Vec<String>>,
}

pub fn export_background(bg: &Background, id: u8) -> Vec<(String, Vec<u8>)> {
    let all = from_zero(&bg.tiles, bg.first_tile);
    let rows = vec![bg.palette.unwrap_or_else(gray)];
    let row_of = first_rows(all.len(), bg.map.iter().copied(), 0);
    let (png, image) = tiles::export_image(
        "tiles.png",
        &all,
        Layout::Grid { columns: COLUMNS },
        &rows,
        [0, bg.palette.is_some() as usize],
        |i| row_of[i].min(rows.len() as u8 - 1),
    );
    let mut files = vec![("tiles.png".to_string(), png)];
    let (iw, ih) = Layout::Grid { columns: COLUMNS }.size(all.len() as u32);
    files.push(("map.tmj".into(), tiled_map(bg, iw, ih, all.len())));
    let mut anims = Vec::new();
    for (k, a) in bg.anims.iter().enumerate() {
        let mut doc = AnimDoc { tiles: None, palettes: None, repeat_from: a.repeat_from, image: None, frames: Vec::new() };
        match a.target {
            AnimTarget::Tiles { first, count } => {
                doc.tiles = Some([first, count]);
                // One block per frame, 16 tiles wide.
                let w = (count as u32).clamp(1, COLUMNS);
                let h = (count as u32).div_ceil(w).max(1);
                let per = (w * h) as usize;
                let mut t = Tiles::default();
                for f in &a.frames {
                    let mut px = f.tiles.pixels.clone();
                    px.resize(per * Tiles::TILE, 0);
                    t.pixels.extend(px);
                }
                let file = format!("anim-{k}.png");
                let layout = Layout::Blocks { width: w, height: h, columns: 1 };
                let (png, img) = tiles::export_image(&file, &t, layout, &rows, [0, 0], |_| 0);
                files.push((file, png));
                doc.image = Some(img);
            }
            AnimTarget::Palettes { first, count } => doc.palettes = Some([first, count]),
            AnimTarget::Nothing => {}
        }
        doc.frames = a
            .frames
            .iter()
            .map(|f| AnimFrameDoc { ticks: f.delay, palettes: f.palettes.iter().map(tiles::palette_text).collect() })
            .collect();
        anims.push(doc);
    }
    let doc = BackgroundDoc {
        format: BACKGROUND_FORMAT.into(),
        version: VERSION,
        id,
        tiles: image,
        first_tile: bg.first_tile,
        map: "map.tmj".into(),
        palette: bg.palette.is_some(),
        scroll: [bg.scroll.0, bg.scroll.1],
        anims,
        region: bg.region.clone(),
    };
    files.push(("background.json".into(), json_lines(&doc)));
    files
}

/// A background and its id.
pub fn import_background(dir: &Path, prefix: &str, report: &mut Report) -> Option<(u8, Background)> {
    let name = format!("{prefix}/background.json");
    let doc: BackgroundDoc = read_json(&dir.join("background.json"), &name, report)?;
    if doc.format != BACKGROUND_FORMAT || doc.version != VERSION {
        report.error(&name, format!("not a {BACKGROUND_FORMAT} file of version {VERSION} (extract the pack again)"));
        return None;
    }
    let (all, palettes) = tiles::import_image(dir, prefix, &doc.tiles, report)?;
    let tiles = after(all, doc.first_tile, report, &format!("{prefix}/{}", doc.tiles.file));
    let (map, width, height) = read_tiled_map(&dir.join(&doc.map), &format!("{prefix}/{}", doc.map), report)?;
    let mut anims = Vec::new();
    for (k, a) in doc.anims.iter().enumerate() {
        let target = match (a.tiles, a.palettes) {
            (Some([first, count]), None) => AnimTarget::Tiles { first, count },
            (None, Some([first, count])) => AnimTarget::Palettes { first, count },
            (None, None) => AnimTarget::Nothing,
            _ => {
                report.error(&name, format!("animation {k} replaces both tiles and palettes"));
                continue;
            }
        };
        let mut frame_tiles = vec![Tiles::default(); a.frames.len()];
        if let (AnimTarget::Tiles { count, .. }, Some(img)) = (target, &a.image) {
            let Layout::Blocks { width: w, height: h, .. } = img.layout else {
                report.error(&name, format!("animation {k}'s image must use a blocks layout"));
                continue;
            };
            let per = (w * h) as usize;
            let (t, _) = tiles::import_image(dir, prefix, img, report)?;
            if t.len() < per * a.frames.len() {
                report.error(&name, format!("animation {k}'s image has fewer blocks than frames"));
                continue;
            }
            for (f, ft) in frame_tiles.iter_mut().enumerate() {
                let start = f * per * Tiles::TILE;
                ft.pixels = t.pixels[start..start + count as usize * Tiles::TILE].to_vec();
            }
        }
        let frames = a
            .frames
            .iter()
            .zip(frame_tiles)
            .map(|(f, tiles)| GfxAnimFrame {
                tiles,
                palettes: f.palettes.iter().map(|p| tiles::parse_palette(p, report, &name)).collect(),
                delay: f.ticks,
            })
            .collect();
        anims.push(GfxAnim { target, frames, repeat_from: a.repeat_from });
    }
    let bg = Background {
        tiles,
        first_tile: doc.first_tile,
        map,
        map_width: width,
        map_height: height,
        palette: doc.palette.then(|| palettes.first().copied().unwrap_or_default()),
        scroll: (doc.scroll[0], doc.scroll[1]),
        anims,
        region: doc.region.clone(),
    };
    Some((doc.id, bg))
}

// ---- Tiled maps -------------------------------------------------------------------

const FLIP_H: u32 = 0x8000_0000;
const FLIP_V: u32 = 0x4000_0000;
const FLIP_D: u32 = 0x2000_0000;
const FLIP_HEX: u32 = 0x1000_0000;

/// A Tiled JSON map (orthogonal, 8x8 tiles) of a background's map: one
/// tile layer, the tileset being the tiles image (gid = tile + 1), flips as
/// Tiled's flip bits, palettes (when any isn't 0) as the layer's
/// `palettes` property, one hex digit per cell.
fn tiled_map(bg: &Background, image_w: u32, image_h: u32, tile_count: usize) -> Vec<u8> {
    let data: Vec<u32> = bg
        .map
        .iter()
        .map(|e| (e.tile as u32 + 1) | if e.hflip { FLIP_H } else { 0 } | if e.vflip { FLIP_V } else { 0 })
        .collect();
    let mut layer = serde_json::json!({
        "id": 1, "name": "tiles", "type": "tilelayer", "x": 0, "y": 0,
        "width": bg.map_width, "height": bg.map_height, "opacity": 1, "visible": true,
        "data": data,
    });
    if bg.map.iter().any(|e| e.palette != 0) {
        let digits: String = bg.map.iter().map(|e| char::from_digit(e.palette as u32, 16).unwrap()).collect();
        layer["properties"] = serde_json::json!([{ "name": "palettes", "type": "string", "value": digits }]);
    }
    let map = serde_json::json!({
        "type": "map", "version": "1.10", "tiledversion": "1.10.2",
        "orientation": "orthogonal", "renderorder": "right-down", "infinite": false,
        "width": bg.map_width, "height": bg.map_height, "tilewidth": 8, "tileheight": 8,
        "nextlayerid": 2, "nextobjectid": 1,
        "layers": [layer],
        "tilesets": [{
            "firstgid": 1, "name": "tiles", "image": "tiles.png",
            "imagewidth": image_w, "imageheight": image_h,
            "tilewidth": 8, "tileheight": 8, "tilecount": tile_count, "columns": COLUMNS,
            "margin": 0, "spacing": 0,
        }],
    });
    let mut s = serde_json::to_string_pretty(&map).unwrap();
    // One map row per line.
    s = compact_data(&s, bg.map_width as usize);
    s.push('\n');
    s.into_bytes()
}

/// Rewrite a pretty-printed "data" array with `width` numbers a line.
fn compact_data(s: &str, width: usize) -> String {
    let Some(start) = s.find("\"data\": [") else { return s.to_string() };
    let open = start + "\"data\": [".len();
    let close = open + s[open..].find(']').unwrap();
    let nums: Vec<&str> = s[open..close].split(',').map(str::trim).filter(|x| !x.is_empty()).collect();
    let indent = " ".repeat(8);
    let rows: Vec<String> = nums.chunks(width.max(1)).map(|r| format!("{indent}{}", r.join(", "))).collect();
    format!("{}\n{}\n      {}", &s[..open], rows.join(",\n"), &s[close..])
}

fn read_tiled_map(path: &Path, name: &str, report: &mut Report) -> Option<(Vec<MapEntry>, u16, u16)> {
    let v: serde_json::Value = read_json(path, name, report)?;
    if v["infinite"].as_bool() == Some(true) {
        report.error(name, "an infinite map; make it finite (Map > Map Properties > Infinite off)");
        return None;
    }
    let (w, h) = (v["width"].as_u64()? as usize, v["height"].as_u64()? as usize);
    let tilesets = v["tilesets"].as_array()?;
    if tilesets.len() != 1 || tilesets[0]["firstgid"].as_u64() != Some(1) {
        report.error(name, "the map must use exactly its one tileset (tiles.png), first gid 1");
        return None;
    }
    let layers = v["layers"].as_array()?;
    let tile_layers: Vec<&serde_json::Value> = layers.iter().filter(|l| l["type"] == "tilelayer").collect();
    if tile_layers.len() != 1 {
        report.error(name, "the map must have exactly one tile layer (a GBA background is one layer)");
        return None;
    }
    let layer = tile_layers[0];
    if layer["encoding"].as_str().is_some_and(|e| e != "csv") || layer["compression"].as_str().is_some_and(|c| !c.is_empty()) {
        report.error(name, "the tile layer is stored base64/compressed; set Map > Map Properties > Tile Layer Format to CSV");
        return None;
    }
    let data: Vec<u32> = layer["data"].as_array()?.iter().map(|x| x.as_u64().unwrap_or(0) as u32).collect();
    if data.len() != w * h {
        report.error(name, format!("the layer has {} cells for a {w}x{h} map", data.len()));
        return None;
    }
    let palettes: Vec<u8> = layer["properties"]
        .as_array()
        .and_then(|p| p.iter().find(|x| x["name"] == "palettes"))
        .and_then(|p| p["value"].as_str())
        .map(|s| s.chars().map(|c| c.to_digit(16).unwrap_or(0) as u8).collect())
        .unwrap_or_default();
    let mut map = Vec::with_capacity(data.len());
    for (i, &gid) in data.iter().enumerate() {
        if gid & (FLIP_D | FLIP_HEX) != 0 {
            report.error(name, format!("cell {} is rotated; the GBA can only flip tiles horizontally and vertically", i));
        }
        let id = gid & 0x0FFF_FFFF;
        if id == 0 {
            report.error(name, format!("cell ({}, {}) is empty; every cell of a GBA map names a tile", i % w, i / w));
        }
        map.push(MapEntry {
            tile: id.saturating_sub(1) as u16,
            hflip: gid & FLIP_H != 0,
            vflip: gid & FLIP_V != 0,
            palette: palettes.get(i).copied().unwrap_or(0),
        });
    }
    Some((map, w as u16, h as u16))
}
