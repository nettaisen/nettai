//! Battle sprites as a part atlas, part layouts and animation timing.
//!
//! A sprite is a folder (`graphics/sprites/CC-II/`, category and index in
//! hex) of three files:
//!
//! - `atlas.png`: every part image the sprite draws, as an 8-bit indexed
//!   PNG. Its palette is the sprite's palette set (16 rows of 16 colors;
//!   `sprite_setPalette` picks the row); a part's pixels display in the row
//!   of its palette offset. Each tile set is drawn as the pose it makes,
//!   its parts where and as (mirrored) a frame draws them, the tiles of
//!   parts that would cover each other moved apart, poses lined up in a
//!   grid (see `figure`).
//! - `sprite.json`: the tile sets (which atlas rectangle fills which tiles)
//!   and the frame layouts (the hardware sprites a frame is built from:
//!   first tile, size, offset from the object, flips, palette offset). The
//!   first part of a layout is the object's shadow.
//! - `animations.json`: per animation, its frames' durations in ticks and
//!   flag bytes, and the tile set and layout each frame draws. This is the
//!   simulation's data (effects and attacks end on it), readable without
//!   touching any image ([`crate::timing`]).
//!
//! Parts are kept rather than baked into whole frames: in 40% of EXE6's
//! frames parts overlap with different opaque pixels (an arm over a body),
//! so a flattened frame loses pixels that show when a part is hidden, the
//! shadow is drawn on the ground, or a part is culled.

use crate::image::{self, Indexed};
use crate::report::Report;
use crate::tiles;
use nettai_assets::{Palette, SpriteFrame, SpritePart, SpriteSheet, Tiles};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

pub const FORMAT: &str = "nettai-content/sprite";
pub const ANIMATIONS_FORMAT: &str = "nettai-content/animations";
pub const VERSION: u32 = 1;

/// Space between poses in the atlas, and above a pose's undrawn tiles.
const GAP: u32 = 8;
/// The farthest a part moves along one way to clear the others.
const MAX_SHIFT: i32 = 256;
/// How much larger than the poses' own room a grid of equal cells may be.
const GRID_SLACK: i64 = 4;
/// The palette rows an indexed image holds (256 colors).
pub const ATLAS_PALETTE_ROWS: usize = 16;

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
    /// Palette rows the atlas palette holds (palette set 0's first rows:
    /// a PNG palette holds 16).
    pub palette_rows: usize,
    /// Hashes of the atlas palette as exported (in order, sorted): tells a
    /// re-sorted or truncated palette from an edited one.
    pub palette_fingerprint: String,
    /// Palette entries (row, index) whose BGR555 value has bit 15 set,
    /// which the hardware ignores and a PNG can't hold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palette_high_bits: Vec<[u8; 2]>,
    /// Palette set 0's rows after the atlas palette's, as BGR555 hex (a
    /// navi's palettes for its other forms, say).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub more_palette_rows: Vec<Vec<String>>,
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

/// An atlas rectangle that fills tiles `tile..` of its tile set, row by
/// row (after undoing `flip`). Regions may hold the same tiles where they
/// agree.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionDoc {
    pub tile: u16,
    pub size: [u8; 2],
    pub at: [u32; 2],
    /// The atlas shows the region mirrored, as a frame draws it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flip: Option<Flip>,
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
    // The atlas's palette holds the first rows; the rest go in the document.
    let (set0, more) = set0.split_at(set0.len().min(ATLAS_PALETTE_ROWS));
    let palette = image::palette_rgb(set0);
    let mut high_bits = Vec::new();
    for (r, row) in set0.iter().enumerate() {
        for (i, &c) in row.iter().enumerate() {
            if c & 0x8000 != 0 {
                high_bits.push([r as u8, i as u8]);
            }
        }
    }
    // Regions per tile set, in order of first use, each with the palette
    // offset it is drawn with and where and how that part draws: every
    // part of the first layout that draws the tile set (a part drawn again
    // is a copy), then the parts only later layouts draw.
    let mut regions: Vec<Vec<Region>> = vec![Vec::new(); sheet.tilesets.len()];
    let mut known: HashMap<(u16, u16, u8, u8), ()> = HashMap::new();
    // The layouts each tile set is drawn with, in order of first use.
    let mut layouts: Vec<Vec<u16>> = vec![Vec::new(); sheet.tilesets.len()];
    for anim in &sheet.animations {
        for f in anim {
            let seen = &mut layouts[f.tileset as usize];
            let rank = seen.iter().position(|&l| l == f.parts).unwrap_or_else(|| {
                seen.push(f.parts);
                seen.len() - 1
            });
            for (k, p) in sheet.part_lists[f.parts as usize].iter().enumerate() {
                let new = known.insert((f.tileset, p.tile, p.width, p.height), ()).is_none();
                let region = Region {
                    tile: p.tile,
                    w: p.width,
                    h: p.height,
                    row: p.palette,
                    offset: Some([p.x, p.y]),
                    flip: (p.hflip, p.vflip),
                    later: rank > 0,
                    copy: !new,
                    part: k,
                };
                let set = &mut regions[f.tileset as usize];
                if (new || rank == 0) && !set.iter().any(|r| r.same_draw(&region)) {
                    set.push(region);
                }
            }
        }
    }
    // Tiles no part draws get a grid region of their own so nothing is lost.
    for (t, set) in sheet.tilesets.iter().enumerate() {
        let mut covered = vec![false; set.len()];
        for r in &regions[t] {
            for i in r.tile as usize..r.tile as usize + (r.w as usize / 8) * (r.h as usize / 8) {
                if let Some(c) = covered.get_mut(i) {
                    *c = true;
                }
            }
        }
        for (i, c) in covered.iter().enumerate() {
            if !c {
                regions[t].push(Region {
                    tile: i as u16,
                    w: 8,
                    h: 8,
                    row: 0,
                    offset: None,
                    flip: (false, false),
                    later: false,
                    copy: false,
                    part: 0,
                });
            }
        }
    }
    let figures: Vec<Vec<Piece>> = regions.iter().zip(&sheet.tilesets).map(|(set, tiles)| figure(set, tiles)).collect();
    let (origins, width, height) = sheet_cells(&figures);
    let mut atlas = Indexed::new(width, height, palette.clone());
    let mut placed: Vec<Vec<RegionDoc>> = Vec::new();
    for (t, pieces) in figures.iter().enumerate() {
        let [ox, oy] = origins[t];
        let mut docs = Vec::new();
        for piece in pieces {
            let at = [(ox + piece.at.x) as u32, (oy + piece.at.y) as u32];
            // Clear pixels never cover a color another piece holds there.
            for (k, &v) in piece_pixels(&sheet.tilesets[t], piece).iter().enumerate() {
                let (x, y) = (at[0] + k as u32 % piece.at.w as u32, at[1] + k as u32 / piece.at.w as u32);
                if v & 15 != 0 || atlas.get(x, y) & 15 == 0 {
                    atlas.set(x, y, v);
                }
            }
            docs.push(RegionDoc { tile: piece.tile, size: [piece.at.w as u8, piece.at.h as u8], at, flip: flip_doc(piece.flip) });
        }
        placed.push(docs);
    }
    let doc = SpriteDoc {
        format: FORMAT.into(),
        version: VERSION,
        sprite: [sheet.category, sheet.index],
        atlas: "atlas.png".into(),
        palette_rows: set0.len(),
        palette_fingerprint: image::palette_fingerprint(&palette),
        palette_high_bits: high_bits,
        more_palette_rows: more.iter().map(|row| row.iter().map(|c| format!("{c:#06x}")).collect()).collect(),
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

/// A region to place: a part's tiles, the palette row it shows in, and
/// where the part draws from the object (none for tiles no part draws).
#[derive(Clone)]
struct Region {
    tile: u16,
    w: u8,
    h: u8,
    row: u8,
    offset: Option<[i8; 2]>,
    /// Drawn mirrored (h, v).
    flip: (bool, bool),
    /// Only layouts after the tile set's first draw it.
    later: bool,
    /// Another region holds the same tiles (the part is drawn again): it
    /// shows where it fits and is left out elsewhere.
    copy: bool,
    /// Its place in the layout that first draws it: 0 is the shadow, at
    /// the back; of the others the first is in front.
    part: usize,
}

impl Region {
    fn same_draw(&self, o: &Region) -> bool {
        (self.tile, self.w, self.h, self.offset, self.flip) == (o.tile, o.w, o.h, o.offset, o.flip)
    }
}

/// Tiles of a region that stay side by side in the atlas: `at`'s size
/// holds them row by row from `tile`, mirrored by `flip`.
#[derive(Clone, Copy, Debug)]
struct Piece {
    tile: u16,
    row: u8,
    at: Rect,
    flip: (bool, bool),
}

fn flip_doc((h, v): (bool, bool)) -> Option<Flip> {
    match (h, v) {
        (false, false) => None,
        (true, false) => Some(Flip::H),
        (false, true) => Some(Flip::V),
        (true, true) => Some(Flip::HV),
    }
}

/// A rectangle in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Rect {
    fn union(self, o: Rect) -> Rect {
        let (x, y) = (self.x.min(o.x), self.y.min(o.y));
        Rect { x, y, w: (self.x + self.w).max(o.x + o.w) - x, h: (self.y + self.h).max(o.y + o.h) - y }
    }
}

/// A tile set's regions as the pose they draw, from the object's place,
/// in pieces: each part where and as the first layout that draws the tile
/// set draws it, the largest first, then the parts it draws again, then
/// the shadow, then the parts later layouts swap in. Parts share atlas
/// pixels where both are clear or alike; a pixel two parts hold in
/// different colors (an arm over the body, clear corners over a leg)
/// can't be shared, so of a part that doesn't fit, the tiles that fit
/// stay and the others go together the least distance that frees them,
/// out from the pose's middle or along an axis (the shadow and swapped-in
/// parts go whole); a part drawn again shows only where it fits. A part's
/// tiles that stay side by side are one piece again. Tiles no part draws
/// follow in rows under the pose.
fn figure(regions: &[Region], tiles: &Tiles) -> Vec<Piece> {
    struct Unit {
        region: usize,
        tx: usize,
        ty: usize,
        x: i32,
        y: i32,
        pixels: Vec<u8>,
    }
    let mut units: Vec<Unit> = Vec::new();
    for (i, r) in regions.iter().enumerate() {
        let Some([ox, oy]) = r.offset else { continue };
        let (columns, rows) = (r.w as usize / 8, r.h as usize / 8);
        for ty in 0..rows {
            for tx in 0..columns {
                let piece = Piece { tile: r.tile + (ty * columns + tx) as u16, row: r.row, at: Rect { x: 0, y: 0, w: 8, h: 8 }, flip: r.flip };
                let (sx, sy) = (if r.flip.0 { columns - 1 - tx } else { tx }, if r.flip.1 { rows - 1 - ty } else { ty });
                let (x, y) = (ox as i32 + 8 * sx as i32, oy as i32 + 8 * sy as i32);
                units.push(Unit { region: i, tx, ty, x, y, pixels: piece_pixels(tiles, &piece) });
            }
        }
    }
    let at = |u: &Unit, p: usize, (dx, dy): (i32, i32)| (u.x + dx + p as i32 % 8, u.y + dy + p as i32 / 8);
    let fits = |held: &HashMap<(i32, i32), u8>, u: &Unit, d: (i32, i32)| {
        u.pixels.iter().enumerate().all(|(p, &v)| held.get(&at(u, p, d)).is_none_or(|&h| h == v || (h & 15 == 0 && v & 15 == 0)))
    };
    let hold = |held: &mut HashMap<(i32, i32), u8>, u: &Unit, d: (i32, i32)| {
        for (p, &v) in u.pixels.iter().enumerate() {
            let px = held.entry(at(u, p, d)).or_insert(v);
            if v & 15 != 0 {
                *px = v;
            }
        }
    };
    // The pose's middle, doubled to stay whole.
    let bounds = units.iter().map(|u| Rect { x: u.x, y: u.y, w: 8, h: 8 }).reduce(Rect::union);
    let (mx, my) = bounds.map_or((0, 0), |b| (2 * b.x + b.w, 2 * b.y + b.h));
    // The first layout's picture as the game draws it: the shadow at the
    // back, the other parts front first.
    let mut back_to_front: Vec<usize> = (0..regions.len()).filter(|&i| regions[i].offset.is_some() && !regions[i].later).collect();
    back_to_front.sort_by_key(|&i| if regions[i].part == 0 { 0 } else { usize::MAX - regions[i].part });
    let mut picture: HashMap<(i32, i32), u8> = HashMap::new();
    for &i in &back_to_front {
        for u in units.iter().filter(|u| u.region == i) {
            for (p, &v) in u.pixels.iter().enumerate() {
                if v & 15 != 0 {
                    picture.insert(at(u, p, (0, 0)), v);
                }
            }
        }
    }
    let place = |order: &[usize]| {
        let mut held: HashMap<(i32, i32), u8> = HashMap::new();
        let mut shift: Vec<Option<(i32, i32)>> = vec![None; units.len()];
            for &i in order {
                let r = &regions[i];
                let own: Vec<usize> = (0..units.len()).filter(|&n| units[n].region == i).collect();
                if r.copy || (!r.later && r.part != 0) || own.iter().all(|&n| fits(&held, &units[n], (0, 0))) {
                    for &n in &own {
                        if fits(&held, &units[n], (0, 0)) {
                            hold(&mut held, &units[n], (0, 0));
                            shift[n] = Some((0, 0));
                        }
                    }
                }
                let moved: Vec<usize> = own.into_iter().filter(|&n| shift[n].is_none()).collect();
                if r.copy || moved.is_empty() {
                    continue;
                }
                let b = moved.iter().map(|&n| Rect { x: units[n].x, y: units[n].y, w: 8, h: 8 }).reduce(Rect::union).unwrap();
                let (dx, dy) = ((2 * b.x + b.w - mx) as f64, (2 * b.y + b.h - my) as f64);
                let l = dx.abs().max(dy.abs());
                let out = if l == 0.0 { (0.0, 1.0) } else { (dx / l, dy / l) };
                let step = |(dx, dy): (f64, f64), t: i32| ((dx * t as f64).round() as i32, (dy * t as f64).round() as i32);
                let free = |d| moved.iter().all(|&n| fits(&held, &units[n], d));
                // The nearest place along each way; out from the middle wins ties.
                let d = [out, (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)]
                    .into_iter()
                    .filter_map(|w| (1..=MAX_SHIFT).find(|&t| free(step(w, t))).map(|t| (t, step(w, t))))
                    .min_by_key(|&(t, _)| t)
                    .map_or_else(|| step(out, (MAX_SHIFT + 1..).find(|&t| free(step(out, t))).unwrap()), |(_, d)| d);
                for &n in &moved {
                    hold(&mut held, &units[n], d);
                    shift[n] = Some(d);
                }
            }
        (shift, held)
    };
    // How far a placing is from the picture: pixels the picture shows
    // that it doesn't hold there, and colors it holds where the picture
    // shows none.
    let misses = |held: &HashMap<(i32, i32), u8>| {
        picture.iter().filter(|(p, v)| held.get(p) != Some(v)).count()
            + held.iter().filter(|(p, v)| **v & 15 != 0 && !picture.contains_key(p)).count()
    };
    // The parts that fit first keep their place: of the orders the largest
    // first, the front first and the back first, the one nearest the
    // picture. The shadow, the parts drawn again and the swapped-in parts
    // come after in each, as they are first drawn.
    let drawn: Vec<usize> = (0..regions.len()).filter(|&i| regions[i].offset.is_some()).collect();
    let rest = |i: usize| (regions[i].later, regions[i].part == 0, regions[i].copy);
    let mut largest = drawn.clone();
    largest.sort_by_key(|&i| (rest(i), std::cmp::Reverse(regions[i].w as u32 * regions[i].h as u32)));
    let mut front = drawn.clone();
    front.sort_by_key(|&i| (rest(i), regions[i].part));
    let mut back = drawn;
    back.sort_by_key(|&i| (rest(i), std::cmp::Reverse(regions[i].part)));
    let (shift, _) = [largest, front, back].iter().map(|o| place(o)).min_by_key(|(_, held)| misses(held)).unwrap();
    // Each part's tiles as pieces: the whole part where its tiles moved
    // alike, else bands of whole rows, else runs along a row.
    let mut pieces = Vec::new();
    for (i, r) in regions.iter().enumerate() {
        let Some([ox, oy]) = r.offset else { continue };
        let (columns, rows) = (r.w as usize / 8, r.h as usize / 8);
        let mut d: Vec<Option<(i32, i32)>> = vec![None; columns * rows];
        for (u, s) in units.iter().zip(&shift) {
            if u.region == i {
                d[u.ty * columns + u.tx] = *s;
            }
        }
        // Tiles [tx, tx + n) of rows [ty, ty + m), moved by `s`, as drawn.
        let mut piece = |tx: usize, ty: usize, n: usize, m: usize, (sx, sy): (i32, i32)| {
            let x = if r.flip.0 { r.w as usize - 8 * (tx + n) } else { 8 * tx };
            let y = if r.flip.1 { r.h as usize - 8 * (ty + m) } else { 8 * ty };
            let at = Rect { x: ox as i32 + x as i32 + sx, y: oy as i32 + y as i32 + sy, w: 8 * n as i32, h: 8 * m as i32 };
            pieces.push(Piece { tile: r.tile + (ty * columns + tx) as u16, row: r.row, at, flip: r.flip });
        };
        let mut ty = 0;
        while ty < rows {
            let row = &d[ty * columns..(ty + 1) * columns];
            if let Some(s) = row[0].filter(|_| row.iter().all(|&s| s == row[0])) {
                // As many whole rows as move alike.
                let mut m = 1;
                while ty + m < rows && d[(ty + m) * columns..(ty + m + 1) * columns].iter().all(|&e| e == Some(s)) {
                    m += 1;
                }
                piece(0, ty, columns, m, s);
                ty += m;
                continue;
            }
            let mut tx = 0;
            while tx < columns {
                let mut n = 1;
                while tx + n < columns && row[tx + n] == row[tx] {
                    n += 1;
                }
                if let Some(s) = row[tx] {
                    piece(tx, ty, n, 1, s);
                }
                tx += n;
            }
            ty += 1;
        }
    }
    // The undrawn tiles, side by side in tile order under the pose.
    let below = pieces.iter().map(|p| p.at).reduce(Rect::union);
    let (left, top) = below.map_or((0, 0), |b| (b.x, b.y + b.h + GAP as i32));
    let columns = below.map_or(0, |b| b.w / 8).max(8);
    for (k, r) in regions.iter().filter(|r| r.offset.is_none()).enumerate() {
        let k = k as i32;
        let at = Rect { x: left + 8 * (k % columns), y: top + 8 * (k / columns), w: 8, h: 8 };
        pieces.push(Piece { tile: r.tile, row: r.row, at, flip: (false, false) });
    }
    pieces
}

/// A piece's pixels as the atlas holds them, row by row: each color in
/// the piece's palette row (a tile past the tile set's end is clear),
/// mirrored as the piece is.
fn piece_pixels(tiles: &Tiles, piece: &Piece) -> Vec<u8> {
    let (w, h) = (piece.at.w as usize, piece.at.h as usize);
    let mut out = vec![piece.row << 4; w * h];
    for ty in 0..h / 8 {
        for tx in 0..w / 8 {
            let Some(t) = tiles.get(piece.tile as usize + ty * (w / 8) + tx) else { continue };
            for (k, &v) in t.iter().enumerate() {
                let (x, y) = (tx * 8 + k % 8, ty * 8 + k / 8);
                let (x, y) = (if piece.flip.0 { w - 1 - x } else { x }, if piece.flip.1 { h - 1 - y } else { y });
                out[y * w + x] = (piece.row << 4) | v;
            }
        }
    }
    out
}

/// Where each tile set's pose goes in the atlas (the pixel its object's
/// place falls on) and the atlas's size. Poses sit in a grid of equal
/// cells, each object at the same place in its cell, so poses line up;
/// where one pose is so much bigger than the rest that the grid would be
/// mostly empty, each pose takes only its own room instead, in rows.
fn sheet_cells(figures: &[Vec<Piece>]) -> (Vec<[i32; 2]>, u32, u32) {
    let gap = GAP as i32;
    let bounds: Vec<Option<Rect>> = figures.iter().map(|f| f.iter().map(|p| p.at).reduce(Rect::union)).collect();
    let cells: Vec<(usize, Rect)> = bounds.iter().enumerate().filter_map(|(t, b)| b.map(|b| (t, b))).collect();
    let mut origins = vec![[0, 0]; figures.len()];
    let Some(all) = cells.iter().map(|c| c.1).reduce(Rect::union) else {
        return (origins, 8, 8);
    };
    let n = cells.len() as i64;
    let own: i64 = cells.iter().map(|(_, b)| ((b.w + gap) * (b.h + gap)) as i64).sum();
    let (width, height) = if n * ((all.w + gap) * (all.h + gap)) as i64 <= GRID_SLACK * own {
        // About square: the fewest columns that make it at least as wide
        // as tall.
        let (cw, ch) = ((all.w + gap) as i64, (all.h + gap) as i64);
        let rows = |c: i64| (n + c - 1) / c;
        let columns = (1..=n).find(|&c| c * cw >= rows(c) * ch).unwrap_or(n);
        for (k, &(t, _)) in cells.iter().enumerate() {
            let (col, row) = (k as i64 % columns, k as i64 / columns);
            origins[t] = [col as i32 * (all.w + gap) - all.x, row as i32 * (all.h + gap) - all.y];
        }
        (columns as i32 * (all.w + gap) - gap, rows(columns) as i32 * (all.h + gap) - gap)
    } else {
        let row_width = (own.isqrt() as i32).max(cells.iter().map(|c| c.1.w + gap).max().unwrap_or(0));
        let (mut x, mut y, mut shelf, mut width) = (0, 0, 0, 0);
        for &(t, b) in &cells {
            if x > 0 && x + b.w > row_width {
                (x, y, shelf) = (0, y + shelf + gap, 0);
            }
            origins[t] = [x - b.x, y - b.y];
            width = width.max(x + b.w);
            x += b.w + gap;
            shelf = shelf.max(b.h);
        }
        (width, y + shelf)
    };
    (origins, width.max(8) as u32, height.max(8) as u32)
}

fn part_doc(p: &SpritePart) -> PartDoc {
    PartDoc {
        tile: p.tile,
        size: [p.width, p.height],
        offset: [p.x, p.y],
        flip: flip_doc((p.hflip, p.vflip)),
        palette: p.palette,
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
    if !d.more_palette_rows.is_empty() {
        writeln!(s, "  \"more_palette_rows\": {},", compact(&d.more_palette_rows)).unwrap();
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
    if doc.format != FORMAT || doc.version != VERSION {
        report.error(file("sprite.json"), format!("not a {FORMAT} file of version {VERSION} (extract the pack again)"));
        return None;
    }
    if anims.format != ANIMATIONS_FORMAT || anims.version != VERSION {
        report.error(file("animations.json"), format!("not a {ANIMATIONS_FORMAT} file of version {VERSION} (extract the pack again)"));
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
                "{} palette colors aren't GBA colors (5 bits a channel) and were rounded: entries {:?}",
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
    let mut hex_rows = |set: &[Vec<String>], what: &str| -> Vec<Palette> {
        let mut rows = Vec::new();
        for row in set {
            let mut p = [0u16; 16];
            for (i, c) in row.iter().enumerate().take(16) {
                match u16::from_str_radix(c.trim_start_matches("0x"), 16) {
                    Ok(v) => p[i] = v,
                    Err(_) => report.error(file("sprite.json"), format!("{what}: {c:?} isn't a hex color")),
                }
            }
            rows.push(p);
        }
        rows
    };
    set0.extend(hex_rows(&doc.more_palette_rows, "palette set 0's later rows"));
    let mut palette_sets = vec![set0];
    for (k, set) in doc.extra_palette_sets.iter().enumerate() {
        palette_sets.push(hex_rows(set, &format!("palette set {}", k + 1)));
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
            let (hflip, vflip) = match reg.flip {
                None => (false, false),
                Some(Flip::H) => (true, false),
                Some(Flip::V) => (false, true),
                Some(Flip::HV) => (true, true),
            };
            let mut region_img = Indexed::new(w, h, Vec::new());
            for y in 0..h {
                for x in 0..w {
                    let (sx, sy) = (if hflip { w - 1 - x } else { x }, if vflip { h - 1 - y } else { y });
                    region_img.set(x, y, atlas.get(reg.at[0] + sx, reg.at[1] + sy));
                }
            }
            let n = (w / 8 * h / 8) as usize;
            let (read, problems) = tiles::read(&region_img, n, tiles::Layout::Grid { columns: w / 8 });
            if !problems.mixed_rows.is_empty() {
                report.warn(
                    file(&doc.atlas),
                    format!(
                        "tileset {t} region {r} (at {:?}) mixes colors from different palette rows; \
                         only each color's place within its row is kept",
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
