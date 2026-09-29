//! Battle sprites as a part atlas, part layouts and animation timing.
//!
//! A sprite is a folder (`graphics/sprites/CC-II/`, category and index in
//! hex) of three files:
//!
//! - `atlas.png`: every part image the sprite draws, once each, as an
//!   8-bit indexed PNG. Its palette is the sprite's palette set (16 rows of
//!   16 colours; `sprite_setPalette` picks the row). A part image is the
//!   part's tiles as stored (unflipped); its pixels display in the row of
//!   the palette offset it is first drawn with.
//! - `sprite.json`: the tile sets (which atlas rectangle fills which tiles)
//!   and the frame layouts (the hardware sprites a frame is built from:
//!   first tile, size, offset from the object, flips, palette offset). The
//!   first part of a layout is the object's shadow.
//! - `animations.json`: per animation, its frames' durations in ticks and
//!   flag bytes, and the tile set and layout each frame draws. This is the
//!   simulation's data (effects and attacks end on it), readable without
//!   touching any image ([`crate::timing`]).
//!
//! Parts are kept rather than baked into whole frames: in 40% of BN6's
//! frames parts overlap with different opaque pixels (an arm over a body),
//! so a flattened frame loses pixels that show when a part is hidden, the
//! shadow is drawn on the ground, or a part is culled.

use crate::image::{self, Indexed};
use crate::report::Report;
use crate::tiles;
use bn6_assets::{Palette, SpriteFrame, SpritePart, SpriteSheet, Tiles};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

pub const FORMAT: &str = "bn6-content/sprite";
pub const ANIMATIONS_FORMAT: &str = "bn6-content/animations";
pub const VERSION: u32 = 1;

/// Atlas width in pixels (wider parts widen it).
const ATLAS_WIDTH: u32 = 256;
/// Space between parts in the atlas.
const GAP: u32 = 8;

/// The folder name of a sprite.
pub fn folder_name(category: u8, index: u8) -> String {
    format!("{category:02x}-{index:02x}")
}

// ---- Documents ------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
pub struct SpriteDoc {
    pub format: String,
    pub version: u32,
    /// SpriteId: category, index.
    pub sprite: [u8; 2],
    pub atlas: String,
    /// Palette rows the atlas palette holds (palette set 0).
    pub palette_rows: usize,
    /// Hashes of the atlas palette as exported (in order, sorted): tells a
    /// re-sorted or truncated palette from an edited one.
    pub palette_fingerprint: String,
    /// Palette entries (row, index) whose BGR555 value has bit 15 set,
    /// which the hardware ignores and a PNG can't hold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palette_high_bits: Vec<[u8; 2]>,
    /// Palette sets after the first, as BGR555 hex per row.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_palette_sets: Vec<Vec<Vec<String>>>,
    pub tilesets: Vec<TilesetDoc>,
    pub layouts: Vec<Vec<PartDoc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TilesetDoc {
    pub tiles: usize,
    pub regions: Vec<RegionDoc>,
}

/// An atlas rectangle that fills tiles `tile..` of its tile set, row by row.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionDoc {
    pub tile: u16,
    pub size: [u8; 2],
    pub at: [u32; 2],
}

/// One hardware sprite of a frame.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartDoc {
    pub tile: u16,
    pub size: [u8; 2],
    pub offset: [i8; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flip: Option<Flip>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub palette: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flip {
    #[serde(rename = "h")]
    H,
    #[serde(rename = "v")]
    V,
    #[serde(rename = "hv")]
    HV,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnimationsDoc {
    pub format: String,
    pub version: u32,
    pub sprite: [u8; 2],
    pub animations: Vec<Vec<FrameDoc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrameDoc {
    /// How many ticks the frame shows.
    pub ticks: u8,
    /// Flag bits: "last" (0x80: the animation ends here), "loop" (0x40:
    /// with "last", it starts over), other bits by value.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<FlagDoc>,
    pub tileset: u16,
    pub layout: u16,
    #[serde(default, skip_serializing_if = "is_zero16")]
    pub palettes: u16,
}

fn is_zero16(v: &u16) -> bool {
    *v == 0
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(untagged)]
pub enum FlagDoc {
    Named(FlagName),
    Bits(u8),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FlagName {
    Last,
    Loop,
}

pub const FLAG_LAST: u8 = 0x80;
pub const FLAG_LOOP: u8 = 0x40;

pub fn flags_doc(flags: u8) -> Vec<FlagDoc> {
    let mut out = Vec::new();
    if flags & FLAG_LAST != 0 {
        out.push(FlagDoc::Named(FlagName::Last));
    }
    if flags & FLAG_LOOP != 0 {
        out.push(FlagDoc::Named(FlagName::Loop));
    }
    let rest = flags & !(FLAG_LAST | FLAG_LOOP);
    if rest != 0 {
        out.push(FlagDoc::Bits(rest));
    }
    out
}

pub fn flags_byte(flags: &[FlagDoc]) -> u8 {
    flags.iter().fold(0, |acc, f| {
        acc | match f {
            FlagDoc::Named(FlagName::Last) => FLAG_LAST,
            FlagDoc::Named(FlagName::Loop) => FLAG_LOOP,
            FlagDoc::Bits(b) => *b,
        }
    })
}

// ---- Export ----------------------------------------------------------------

/// The files of a sprite folder: name and contents.
pub fn export(sheet: &SpriteSheet) -> Vec<(String, Vec<u8>)> {
    let set0: Vec<Palette> = sheet.palette_sets.first().cloned().unwrap_or_default();
    let palette = image::palette_rgb(&set0);
    let mut high_bits = Vec::new();
    for (r, row) in set0.iter().enumerate() {
        for (i, &c) in row.iter().enumerate() {
            if c & 0x8000 != 0 {
                high_bits.push([r as u8, i as u8]);
            }
        }
    }
    // Regions per tile set, in order of first use, each with the palette
    // offset it is first drawn with.
    let mut regions: Vec<Vec<(u16, u8, u8, u8)>> = vec![Vec::new(); sheet.tilesets.len()];
    let mut known: HashMap<(u16, u16, u8, u8), ()> = HashMap::new();
    for anim in &sheet.animations {
        for f in anim {
            for p in &sheet.part_lists[f.parts as usize] {
                if known.insert((f.tileset, p.tile, p.width, p.height), ()).is_none() {
                    regions[f.tileset as usize].push((p.tile, p.width, p.height, p.palette));
                }
            }
        }
    }
    // Tiles no part draws get a grid region of their own so nothing is lost.
    for (t, set) in sheet.tilesets.iter().enumerate() {
        let mut covered = vec![false; set.len()];
        for &(tile, w, h, _) in &regions[t] {
            for i in tile as usize..tile as usize + (w as usize / 8) * (h as usize / 8) {
                if let Some(c) = covered.get_mut(i) {
                    *c = true;
                }
            }
        }
        for (i, c) in covered.iter().enumerate() {
            if !c {
                regions[t].push((i as u16, 8, 8, 0));
            }
        }
    }
    // Shelf packing: each tile set starts a new shelf.
    let width = regions.iter().flatten().map(|r| r.1 as u32).max().unwrap_or(8).max(ATLAS_WIDTH);
    let mut placed: Vec<Vec<RegionDoc>> = Vec::new();
    let mut y = 0;
    for set in &regions {
        let (mut x, mut shelf) = (0, 0);
        let mut docs = Vec::new();
        for &(tile, w, h, _) in set {
            if x + w as u32 > width {
                y += shelf + GAP;
                x = 0;
                shelf = 0;
            }
            docs.push(RegionDoc { tile, size: [w, h], at: [x, y] });
            x += w as u32 + GAP;
            shelf = shelf.max(h as u32);
        }
        y += shelf + GAP;
        placed.push(docs);
    }
    let height = y.saturating_sub(GAP).max(8);
    let mut atlas = Indexed::new(width, height, palette.clone());
    for (t, set) in placed.iter().enumerate() {
        let tiles = &sheet.tilesets[t];
        for (r, doc) in set.iter().enumerate() {
            let row = regions[t][r].3;
            draw_region(&mut atlas, tiles, doc, row);
        }
    }
    let doc = SpriteDoc {
        format: FORMAT.into(),
        version: VERSION,
        sprite: [sheet.category, sheet.index],
        atlas: "atlas.png".into(),
        palette_rows: set0.len(),
        palette_fingerprint: image::palette_fingerprint(&palette),
        palette_high_bits: high_bits,
        extra_palette_sets: sheet.palette_sets[1.min(sheet.palette_sets.len())..]
            .iter()
            .map(|set| set.iter().map(|row| row.iter().map(|c| format!("{c:#06x}")).collect()).collect())
            .collect(),
        tilesets: sheet
            .tilesets
            .iter()
            .zip(placed)
            .map(|(t, regions)| TilesetDoc { tiles: t.len(), regions })
            .collect(),
        layouts: sheet.part_lists.iter().map(|l| l.iter().map(part_doc).collect()).collect(),
    };
    let anims = AnimationsDoc {
        format: ANIMATIONS_FORMAT.into(),
        version: VERSION,
        sprite: [sheet.category, sheet.index],
        animations: sheet
            .animations
            .iter()
            .map(|a| {
                a.iter()
                    .map(|f| FrameDoc {
                        ticks: f.duration,
                        flags: flags_doc(f.flags),
                        tileset: f.tileset,
                        layout: f.parts,
                        palettes: f.palette_set,
                    })
                    .collect()
            })
            .collect(),
    };
    vec![
        ("atlas.png".into(), atlas.to_png()),
        ("sprite.json".into(), sprite_json(&doc).into_bytes()),
        ("animations.json".into(), animations_json(&anims).into_bytes()),
    ]
}

fn part_doc(p: &SpritePart) -> PartDoc {
    PartDoc {
        tile: p.tile,
        size: [p.width, p.height],
        offset: [p.x, p.y],
        flip: match (p.hflip, p.vflip) {
            (false, false) => None,
            (true, false) => Some(Flip::H),
            (false, true) => Some(Flip::V),
            (true, true) => Some(Flip::HV),
        },
        palette: p.palette,
    }
}

/// Draw a region's tiles (a `w` x `h` sprite's tiles, row by row) in
/// palette row `row`.
fn draw_region(atlas: &mut Indexed, tiles: &Tiles, r: &RegionDoc, row: u8) {
    let (w, h) = (r.size[0] as u32, r.size[1] as u32);
    let per_row = w / 8;
    for ty in 0..h / 8 {
        for tx in 0..per_row {
            let Some(t) = tiles.get(r.tile as usize + (ty * per_row + tx) as usize) else { continue };
            for (k, &v) in t.iter().enumerate() {
                let px = if v == 0 { row << 4 } else { (row << 4) | v };
                atlas.set(r.at[0] + tx * 8 + (k % 8) as u32, r.at[1] + ty * 8 + (k / 8) as u32, px);
            }
        }
    }
}

/// JSON with one part, region or frame per line.
fn sprite_json(d: &SpriteDoc) -> String {

    let mut s = String::new();
    s.push_str("{\n");
    writeln!(s, "  \"format\": {},", compact(&d.format)).unwrap();
    writeln!(s, "  \"version\": {},", d.version).unwrap();
    writeln!(s, "  \"sprite\": {},", compact(&d.sprite)).unwrap();
    writeln!(s, "  \"atlas\": {},", compact(&d.atlas)).unwrap();
    writeln!(s, "  \"palette_rows\": {},", d.palette_rows).unwrap();
    writeln!(s, "  \"palette_fingerprint\": {},", compact(&d.palette_fingerprint)).unwrap();
    if !d.palette_high_bits.is_empty() {
        writeln!(s, "  \"palette_high_bits\": {},", compact(&d.palette_high_bits)).unwrap();
    }
    if !d.extra_palette_sets.is_empty() {
        writeln!(s, "  \"extra_palette_sets\": {},", compact(&d.extra_palette_sets)).unwrap();
    }
    s.push_str("  \"tilesets\": [\n");
    for (i, t) in d.tilesets.iter().enumerate() {
        writeln!(s, "    {{ \"tiles\": {}, \"regions\": [", t.tiles).unwrap();
        list(&mut s, "      ", &t.regions);
        let comma = if i + 1 < d.tilesets.len() { "," } else { "" };
        writeln!(s, "    ] }}{comma}").unwrap();
    }
    s.push_str("  ],\n  \"layouts\": [\n");
    for (i, l) in d.layouts.iter().enumerate() {
        s.push_str("    [\n");
        list(&mut s, "      ", l);
        let comma = if i + 1 < d.layouts.len() { "," } else { "" };
        writeln!(s, "    ]{comma}").unwrap();
    }
    s.push_str("  ]\n}\n");
    s
}

fn animations_json(d: &AnimationsDoc) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    writeln!(s, "  \"format\": {},", serde_json::to_string(&d.format).unwrap()).unwrap();
    writeln!(s, "  \"version\": {},", d.version).unwrap();
    writeln!(s, "  \"sprite\": {},", serde_json::to_string(&d.sprite).unwrap()).unwrap();
    s.push_str("  \"animations\": [\n");
    for (i, a) in d.animations.iter().enumerate() {
        s.push_str("    [\n");
        list(&mut s, "      ", a);
        let comma = if i + 1 < d.animations.len() { "," } else { "" };
        writeln!(s, "    ]{comma}").unwrap();
    }
    s.push_str("  ]\n}\n");
    s
}

fn list<T: Serialize>(s: &mut String, indent: &str, items: &[T]) {
    for (i, it) in items.iter().enumerate() {
        let comma = if i + 1 < items.len() { "," } else { "" };
        writeln!(s, "{indent}{}{comma}", serde_json::to_string(it).unwrap()).unwrap();
    }
}

fn compact<T: Serialize + ?Sized>(v: &T) -> String {
    serde_json::to_string(v).unwrap()
}

// ---- Import ----------------------------------------------------------------

/// The OAM sprite shapes.
const SHAPES: [(u8, u8); 12] =
    [(8, 8), (16, 16), (32, 32), (64, 64), (16, 8), (32, 8), (32, 16), (64, 32), (8, 16), (8, 32), (16, 32), (32, 64)];

/// Read a sprite folder back. `name` is the folder's path in the pack, for
/// messages. Returns `None` when an error makes it unusable (the report
/// says why).
pub fn import(dir: &Path, name: &str, report: &mut Report) -> Option<SpriteSheet> {
    let file = |f: &str| format!("{name}/{f}");
    let doc: SpriteDoc = read_json(&dir.join("sprite.json"), &file("sprite.json"), report)?;
    let anims: AnimationsDoc = read_json(&dir.join("animations.json"), &file("animations.json"), report)?;
    if doc.format != FORMAT || doc.version > VERSION {
        report.error(file("sprite.json"), format!("not a {FORMAT} file of version {VERSION} or older"));
        return None;
    }
    if anims.format != ANIMATIONS_FORMAT || anims.version > VERSION {
        report.error(file("animations.json"), format!("not a {ANIMATIONS_FORMAT} file of version {VERSION} or older"));
        return None;
    }
    if anims.sprite != doc.sprite {
        report.error(file("animations.json"), "names a different sprite than sprite.json");
    }
    let atlas = match Indexed::load(&dir.join(&doc.atlas)) {
        Ok(a) => a,
        Err(e) => {
            report.error(file(&doc.atlas), e);
            return None;
        }
    };
    // Palette set 0 from the atlas palette.
    if let Some(why) = image::palette_change(&doc.palette_fingerprint, &atlas.palette, doc.palette_rows * 16) {
        report.error(file(&doc.atlas), why);
        return None;
    }
    let (mut set0, off_grid) = image::palette_rows(&atlas.palette, doc.palette_rows);
    if !off_grid.is_empty() {
        report.warn(
            file(&doc.atlas),
            format!(
                "{} palette colours aren't GBA colours (5 bits a channel) and were rounded: entries {:?}",
                off_grid.len(),
                &off_grid[..off_grid.len().min(8)]
            ),
        );
    }
    for &[r, i] in &doc.palette_high_bits {
        if let Some(row) = set0.get_mut(r as usize) {
            row[i as usize & 15] |= 0x8000;
        }
    }
    let mut palette_sets = vec![set0];
    for (k, set) in doc.extra_palette_sets.iter().enumerate() {
        let mut rows = Vec::new();
        for row in set {
            let mut p = [0u16; 16];
            for (i, c) in row.iter().enumerate().take(16) {
                match u16::from_str_radix(c.trim_start_matches("0x"), 16) {
                    Ok(v) => p[i] = v,
                    Err(_) => report.error(file("sprite.json"), format!("palette set {}: {c:?} isn't a hex colour", k + 1)),
                }
            }
            rows.push(p);
        }
        palette_sets.push(rows);
    }
    // Tile sets from their atlas regions.
    let mut tilesets = Vec::with_capacity(doc.tilesets.len());
    for (t, set) in doc.tilesets.iter().enumerate() {
        let mut pixels = vec![0u8; set.tiles * Tiles::TILE];
        let mut written: Vec<Option<usize>> = vec![None; set.tiles];
        for (r, reg) in set.regions.iter().enumerate() {
            let (w, h) = (reg.size[0] as u32, reg.size[1] as u32);
            if w % 8 != 0 || h % 8 != 0 || reg.at[0] + w > atlas.width || reg.at[1] + h > atlas.height {
                report.error(file("sprite.json"), format!("tileset {t} region {r}: {w}x{h} at {:?} is outside the atlas", reg.at));
                continue;
            }
            let mut region_img = Indexed::new(w, h, Vec::new());
            for y in 0..h {
                for x in 0..w {
                    region_img.set(x, y, atlas.get(reg.at[0] + x, reg.at[1] + y));
                }
            }
            let n = (w / 8 * h / 8) as usize;
            let (read, problems) = tiles::read(&region_img, n, tiles::Layout::Grid { columns: w / 8 });
            if !problems.mixed_rows.is_empty() {
                report.warn(
                    file(&doc.atlas),
                    format!(
                        "tileset {t} region {r} (at {:?}) mixes colours from different palette rows; \
                         only each colour's place within its row is kept",
                        reg.at
                    ),
                );
            }
            for k in 0..n {
                let slot = reg.tile as usize + k;
                if slot >= set.tiles {
                    report.error(file("sprite.json"), format!("tileset {t} region {r} runs past the tileset's {} tiles", set.tiles));
                    break;
                }
                let tile = read.get(k).unwrap();
                let dst = &mut pixels[slot * Tiles::TILE..(slot + 1) * Tiles::TILE];
                if let Some(other) = written[slot]
                    && dst != tile
                {
                    report.error(
                        file(&doc.atlas),
                        format!("tileset {t}: regions {other} and {r} both draw tile {slot} but differ; make them agree"),
                    );
                }
                dst.copy_from_slice(tile);
                written[slot] = Some(r);
            }
        }
        let blank = written.iter().filter(|w| w.is_none()).count();
        if blank > 0 {
            report.warn(file("sprite.json"), format!("tileset {t}: {blank} tiles aren't in any region and stay blank"));
        }
        tilesets.push(Tiles { pixels });
    }
    // Layouts.
    let mut part_lists = Vec::with_capacity(doc.layouts.len());
    for (l, layout) in doc.layouts.iter().enumerate() {
        let mut parts = Vec::with_capacity(layout.len());
        for (i, p) in layout.iter().enumerate() {
            let (w, h) = (p.size[0], p.size[1]);
            if !SHAPES.contains(&(w, h)) {
                report.error(file("sprite.json"), format!("layout {l} part {i}: {w}x{h} isn't a hardware sprite size"));
            }
            if p.palette > 15 {
                report.error(file("sprite.json"), format!("layout {l} part {i}: palette offset {} isn't 0..=15", p.palette));
            }
            let (hflip, vflip) = match p.flip {
                None => (false, false),
                Some(Flip::H) => (true, false),
                Some(Flip::V) => (false, true),
                Some(Flip::HV) => (true, true),
            };
            parts.push(SpritePart { tile: p.tile, x: p.offset[0], y: p.offset[1], width: w, height: h, hflip, vflip, palette: p.palette });
        }
        if parts.len() > 32 {
            report.warn(file("sprite.json"), format!("layout {l} has {} parts; only the first 32 are drawn", parts.len()));
        }
        part_lists.push(parts);
    }
    // Animations.
    let mut animations = Vec::with_capacity(anims.animations.len());
    for (a, frames) in anims.animations.iter().enumerate() {
        let mut out = Vec::with_capacity(frames.len());
        for (i, f) in frames.iter().enumerate() {
            let flags = flags_byte(&f.flags);
            let last = i + 1 == frames.len();
            if last && flags & FLAG_LAST == 0 {
                report.warn(file("animations.json"), format!("animation {a}: the last frame has no \"last\" flag; the animation runs past its end"));
            }
            if !last && flags & FLAG_LAST != 0 {
                report.warn(file("animations.json"), format!("animation {a} frame {i} has \"last\"; the frames after it never play"));
            }
            if f.ticks == 0 {
                report.warn(file("animations.json"), format!("animation {a} frame {i} lasts 0 ticks"));
            }
            if f.tileset as usize >= tilesets.len() || f.layout as usize >= part_lists.len() || f.palettes as usize >= palette_sets.len() {
                report.error(file("animations.json"), format!("animation {a} frame {i} names a tileset, layout or palette set that doesn't exist"));
                continue;
            }
            out.push(SpriteFrame { tileset: f.tileset, palette_set: f.palettes, parts: f.layout, duration: f.ticks, flags });
        }
        animations.push(out);
    }
    Some(SpriteSheet { category: doc.sprite[0], index: doc.sprite[1], tilesets, palette_sets, part_lists, animations })
}

pub(crate) fn read_json<T: serde::de::DeserializeOwned>(path: &Path, name: &str, report: &mut Report) -> Option<T> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            report.error(name, format!("can't read: {e}"));
            return None;
        }
    };
    match serde_json::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            report.error(name, format!("invalid: {e}"));
            None
        }
    }
}
