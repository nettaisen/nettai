//! An Aseprite view of a sprite, for editing whole frames.
//!
//! `sprite.aseprite` shows every animation frame as it is drawn: one
//! layer per hardware part (part 0, the shadow, at the bottom), one tag per
//! animation, frame durations in milliseconds that convert back to the
//! exact tick counts, indexed colour with the sprite's 16 palette rows.
//! Parts that show the same tiles at the same place are linked cels, so an
//! edit reaches every frame that shares them, as on the hardware.
//!
//! It is a view, synced by command: [`export`] writes it from the pack's
//! files, [`import`] reads the pixels, palette, durations and loop flags
//! back into a sprite whose layouts (part geometry) and structure come from
//! sprite.json and animations.json. Pixels drawn outside a part's rectangle
//! are reported, and tiles that frames share must stay the same in each.
//!
//! The file format follows Aseprite's published specification
//! (ase-file-specs.md): header, frames of chunks (layers 0x2004, cels
//! 0x2005, palette 0x2019, tags 0x2018, user data 0x2020).

use crate::report::Report;
use bn6_assets::{Palette, SpriteSheet, Tiles};
use std::collections::HashMap;

/// Frames per second of the game (durations convert through it).
const FPS: f64 = 59.7275;

pub fn ms_from_ticks(t: u8) -> u16 {
    (t as f64 * 1000.0 / FPS).round() as u16
}

/// Ticks for a duration, and whether the duration was exactly a tick
/// count's.
pub fn ticks_from_ms(ms: u16) -> (u8, bool) {
    let t = (ms as f64 * FPS / 1000.0).round().clamp(1.0, 255.0) as u8;
    (t, ms_from_ticks(t) == ms)
}

// ---- The file format ---------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AseFile {
    pub width: u16,
    pub height: u16,
    /// Bits per pixel: 8 indexed, 16 grey, 32 RGBA.
    pub depth: u16,
    pub transparent: u8,
    /// RGBA.
    pub palette: Vec<[u8; 4]>,
    pub layers: Vec<AseLayer>,
    pub frames: Vec<AseFrame>,
    pub tags: Vec<AseTag>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AseLayer {
    pub name: String,
    pub flags: u16,
    /// 0 image, 1 group, 2 tilemap.
    pub kind: u16,
    pub child_level: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AseFrame {
    pub duration_ms: u16,
    pub cels: Vec<AseCel>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AseCel {
    pub layer: u16,
    pub x: i16,
    pub y: i16,
    pub content: CelContent,
    pub text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CelContent {
    Image { width: u16, height: u16, pixels: Vec<u8> },
    /// The same cel as on this frame (same layer).
    Linked(u16),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AseTag {
    pub from: u16,
    pub to: u16,
    pub direction: u8,
    /// 0: not specified (loops in the editor), n: plays n times.
    pub repeat: u16,
    pub name: String,
}

struct W(Vec<u8>);

impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i16(&mut self, v: i16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn zeros(&mut self, n: usize) {
        self.0.extend(std::iter::repeat_n(0, n));
    }
    fn string(&mut self, s: &str) {
        self.u16(s.len() as u16);
        self.0.extend_from_slice(s.as_bytes());
    }
}

fn chunk(out: &mut Vec<u8>, kind: u16, body: Vec<u8>) {
    out.extend_from_slice(&((body.len() + 6) as u32).to_le_bytes());
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend(body);
}

fn user_text(out: &mut Vec<u8>, text: &str) {
    let mut w = W(Vec::new());
    w.u32(1);
    w.string(text);
    chunk(out, 0x2020, w.0);
}

impl AseFile {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut frames = Vec::new();
        for (f, frame) in self.frames.iter().enumerate() {
            let mut chunks = Vec::new();
            let mut n = 0u32;
            if f == 0 {
                let mut w = W(Vec::new());
                w.u32(self.palette.len() as u32);
                w.u32(0);
                w.u32(self.palette.len().saturating_sub(1) as u32);
                w.zeros(8);
                for c in &self.palette {
                    w.u16(0);
                    w.0.extend_from_slice(c);
                }
                chunk(&mut chunks, 0x2019, w.0);
                n += 1;
                for l in &self.layers {
                    let mut w = W(Vec::new());
                    w.u16(l.flags);
                    w.u16(l.kind);
                    w.u16(l.child_level);
                    w.u16(0);
                    w.u16(0);
                    w.u16(0);
                    w.u8(255);
                    w.zeros(3);
                    w.string(&l.name);
                    chunk(&mut chunks, 0x2004, w.0);
                    n += 1;
                }
                if !self.tags.is_empty() {
                    let mut w = W(Vec::new());
                    w.u16(self.tags.len() as u16);
                    w.zeros(8);
                    for t in &self.tags {
                        w.u16(t.from);
                        w.u16(t.to);
                        w.u8(t.direction);
                        w.u16(t.repeat);
                        w.zeros(6);
                        w.0.extend_from_slice(&[0, 0, 0, 0]);
                        w.string(&t.name);
                    }
                    chunk(&mut chunks, 0x2018, w.0);
                    n += 1;
                }
            }
            for c in &frame.cels {
                let mut w = W(Vec::new());
                w.u16(c.layer);
                w.i16(c.x);
                w.i16(c.y);
                w.u8(255);
                match &c.content {
                    CelContent::Linked(to) => {
                        w.u16(1);
                        w.i16(0);
                        w.zeros(5);
                        w.u16(*to);
                    }
                    CelContent::Image { width, height, pixels } => {
                        w.u16(2);
                        w.i16(0);
                        w.zeros(5);
                        w.u16(*width);
                        w.u16(*height);
                        w.0.extend(miniz_oxide::deflate::compress_to_vec_zlib(pixels, 9));
                    }
                }
                chunk(&mut chunks, 0x2005, w.0);
                n += 1;
                if let Some(t) = &c.text {
                    user_text(&mut chunks, t);
                    n += 1;
                }
            }
            let mut w = W(Vec::new());
            w.u32((chunks.len() + 16) as u32);
            w.u16(0xF1FA);
            w.u16(if n < 0xFFFF { n as u16 } else { 0xFFFF });
            w.u16(frame.duration_ms);
            w.zeros(2);
            w.u32(n);
            w.0.extend(chunks);
            frames.extend(w.0);
        }
        let mut h = W(Vec::new());
        h.u32((128 + frames.len()) as u32);
        h.u16(0xA5E0);
        h.u16(self.frames.len() as u16);
        h.u16(self.width);
        h.u16(self.height);
        h.u16(self.depth);
        h.u32(1); // layer opacity is valid
        h.u16(100);
        h.u32(0);
        h.u32(0);
        h.u8(self.transparent);
        h.zeros(3);
        h.u16(self.palette.len().min(0xFFFF) as u16);
        h.u8(1);
        h.u8(1);
        h.i16(0);
        h.i16(0);
        h.u16(8); // an 8x8 grid: the tiles
        h.u16(8);
        h.zeros(84);
        h.0.extend(frames);
        h.0
    }

    pub fn from_bytes(b: &[u8]) -> Result<AseFile, String> {
        let bad = |s: &str| format!("not a readable Aseprite file: {s}");
        let u16at = |o: usize| b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]])).ok_or_else(|| bad("truncated"));
        let u32at =
            |o: usize| b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| bad("truncated"));
        if b.len() < 128 || u16at(4)? != 0xA5E0 {
            return Err(bad("no Aseprite header"));
        }
        let nframes = u16at(6)?;
        let mut file = AseFile {
            width: u16at(8)?,
            height: u16at(10)?,
            depth: u16at(12)?,
            transparent: b[28],
            palette: Vec::new(),
            layers: Vec::new(),
            frames: Vec::new(),
            tags: Vec::new(),
        };
        let layer_uuids = u32at(14)? & 4 != 0;
        let mut at = 128;
        for _ in 0..nframes {
            let size = u32at(at)? as usize;
            if u16at(at + 4)? != 0xF1FA {
                return Err(bad("bad frame header"));
            }
            let old = u16at(at + 6)? as u32;
            let duration_ms = u16at(at + 8)?;
            let new = u32at(at + 12)?;
            let nchunks = if new != 0 { new } else { old };
            let mut frame = AseFrame { duration_ms, cels: Vec::new() };
            let mut c = at + 16;
            // What a user-data chunk attaches to.
            let mut last_is_cel = false;
            for _ in 0..nchunks {
                let len = u32at(c)? as usize;
                let kind = u16at(c + 4)?;
                let d = b.get(c + 6..c + len).ok_or_else(|| bad("truncated chunk"))?;
                let d16 = |o: usize| u16::from_le_bytes([d[o], d[o + 1]]);
                let string = |o: usize| -> (String, usize) {
                    let n = d16(o) as usize;
                    (String::from_utf8_lossy(&d[o + 2..o + 2 + n]).into_owned(), o + 2 + n)
                };
                last_is_cel = match kind {
                    0x2019 => {
                        let size = u32::from_le_bytes(d[0..4].try_into().unwrap()) as usize;
                        let (first, last) = (
                            u32::from_le_bytes(d[4..8].try_into().unwrap()) as usize,
                            u32::from_le_bytes(d[8..12].try_into().unwrap()) as usize,
                        );
                        file.palette.resize(size.max(file.palette.len()), [0, 0, 0, 255]);
                        let mut o = 20;
                        for entry in &mut file.palette[first..=last] {
                            let flags = d16(o);
                            *entry = [d[o + 2], d[o + 3], d[o + 4], d[o + 5]];
                            o += 6;
                            if flags & 1 != 0 {
                                o = string(o).1;
                            }
                        }
                        false
                    }
                    0x2004 => {
                        let (name, _) = string(16);
                        let _ = layer_uuids;
                        file.layers.push(AseLayer { name, flags: d16(0), kind: d16(2), child_level: d16(4) });
                        false
                    }
                    0x2005 => {
                        let layer = d16(0);
                        let (x, y) = (d16(2) as i16, d16(4) as i16);
                        let content = match d16(7) {
                            0 => {
                                let (w, h) = (d16(16), d16(18));
                                CelContent::Image { width: w, height: h, pixels: d[20..].to_vec() }
                            }
                            1 => CelContent::Linked(d16(16)),
                            2 => {
                                let (w, h) = (d16(16), d16(18));
                                let pixels = miniz_oxide::inflate::decompress_to_vec_zlib(&d[20..])
                                    .map_err(|_| bad("a cel's image doesn't decompress"))?;
                                CelContent::Image { width: w, height: h, pixels }
                            }
                            t => return Err(format!("cel type {t} (tilemap cels) isn't supported")),
                        };
                        frame.cels.push(AseCel { layer, x, y, content, text: None });
                        true
                    }
                    0x2018 => {
                        let n = d16(0) as usize;
                        let mut o = 10;
                        for _ in 0..n {
                            let (from, to, direction, repeat) = (d16(o), d16(o + 2), d[o + 4], d16(o + 5));
                            let (name, next) = string(o + 17);
                            file.tags.push(AseTag { from, to, direction, repeat, name });
                            o = next;
                        }
                        false
                    }
                    0x2020 => {
                        let flags = u32::from_le_bytes(d[0..4].try_into().unwrap());
                        if last_is_cel && flags & 1 != 0 {
                            frame.cels.last_mut().unwrap().text = Some(string(4).0);
                        }
                        last_is_cel
                    }
                    _ => false,
                };
                c += len;
            }
            file.frames.push(frame);
            at += size;
        }
        Ok(file)
    }
}

// ---- A sprite as an Aseprite file ----------------------------------------------------------

/// The canvas: where the sprite's anchor sits and its size, from every
/// part of every layout its frames use.
fn canvas(s: &SpriteSheet) -> (i32, i32, u16, u16) {
    let (mut x0, mut y0, mut x1, mut y1) = (0i32, 0i32, 1i32, 1i32);
    for a in &s.animations {
        for f in a {
            for p in &s.part_lists[f.parts as usize] {
                x0 = x0.min(p.x as i32);
                y0 = y0.min(p.y as i32);
                x1 = x1.max(p.x as i32 + p.width as i32);
                y1 = y1.max(p.y as i32 + p.height as i32);
            }
        }
    }
    (-x0, -y0, (x1 - x0) as u16, (y1 - y0) as u16)
}

/// Source tile pixel (tile index, pixel) of a part's displayed pixel.
fn source(p: &bn6_assets::SpritePart, u: u32, v: u32) -> (usize, usize) {
    let (w, h) = (p.width as u32, p.height as u32);
    let sx = if p.hflip { w - 1 - u } else { u };
    let sy = if p.vflip { h - 1 - v } else { v };
    let tile = p.tile as usize + ((sy / 8) * (w / 8) + sx / 8) as usize;
    (tile, ((sy % 8) * 8 + sx % 8) as usize)
}

pub fn export(s: &SpriteSheet) -> Vec<u8> {
    let (ax, ay, width, height) = canvas(s);
    let layers = s.part_lists.iter().map(Vec::len).max().unwrap_or(0).max(1);
    // (An indexed image's palette holds set 0's first 16 rows.)
    let mut set0: Vec<Palette> = s.palette_sets.first().cloned().unwrap_or_default();
    set0.truncate(crate::sprite::ATLAS_PALETTE_ROWS);
    let palette = crate::image::palette_rgb(&set0)
        .into_iter()
        .enumerate()
        .map(|(i, c)| [c[0], c[1], c[2], if i == 0 { 0 } else { 255 }])
        .collect();
    let mut frames = Vec::new();
    let mut tags = Vec::new();
    // Cels already written, per layer: (image, x, y) -> frame.
    type Placed = (Vec<u8>, i16, i16);
    let mut seen: Vec<HashMap<Placed, u16>> = vec![HashMap::new(); layers];
    for (a, anim) in s.animations.iter().enumerate() {
        let from = frames.len() as u16;
        for f in anim {
            let tiles = &s.tilesets[f.tileset as usize];
            let mut cels = Vec::new();
            for (k, p) in s.part_lists[f.parts as usize].iter().enumerate() {
                let (w, h) = (p.width as u32, p.height as u32);
                let mut pixels = vec![0u8; (w * h) as usize];
                for v in 0..h {
                    for u in 0..w {
                        let (t, i) = source(p, u, v);
                        let c = tiles.get(t).map_or(0, |t| t[i]);
                        pixels[(v * w + u) as usize] = if c == 0 { 0 } else { (p.palette << 4) | c };
                    }
                }
                let (x, y) = ((ax + p.x as i32) as i16, (ay + p.y as i32) as i16);
                let flip = match (p.hflip, p.vflip) {
                    (false, false) => "",
                    (true, false) => " flip h",
                    (false, true) => " flip v",
                    (true, true) => " flip hv",
                };
                let text = Some(format!(
                    "part {k}: tileset {} tile {} {}x{}{flip} palette {}",
                    f.tileset, p.tile, p.width, p.height, p.palette
                ));
                let key = (pixels, x, y);
                let content = match seen[k].get(&key) {
                    Some(&frame) => CelContent::Linked(frame),
                    None => {
                        seen[k].insert(key.clone(), frames.len() as u16);
                        CelContent::Image { width: w as u16, height: h as u16, pixels: key.0 }
                    }
                };
                cels.push(AseCel { layer: k as u16, x, y, content, text });
            }
            frames.push(AseFrame { duration_ms: ms_from_ticks(f.duration), cels });
        }
        let looping = anim.last().is_some_and(|f| f.flags & crate::sprite::FLAG_LOOP != 0);
        tags.push(AseTag {
            from,
            to: frames.len().saturating_sub(1) as u16,
            direction: 0,
            repeat: if looping { 0 } else { 1 },
            name: format!("anim {a:02}{}", if looping { " loop" } else { "" }),
        });
    }
    let layers = (0..layers)
        .map(|k| AseLayer {
            name: if k == 0 { "part 0 (shadow)".into() } else { format!("part {k}") },
            flags: 1 | 2 | 16, // visible, editable, prefer linked cels
            kind: 0,
            child_level: 0,
        })
        .collect();
    AseFile { width, height, depth: 8, transparent: 0, palette, layers, frames, tags }.to_bytes()
}

/// Read an Aseprite view back into `base` (the sprite as the pack's JSON
/// files have it): pixels, palette set 0, frame durations and loop flags.
pub fn import(bytes: &[u8], base: &SpriteSheet, file: &str, report: &mut Report) -> Option<SpriteSheet> {
    let ase = match AseFile::from_bytes(bytes) {
        Ok(a) => a,
        Err(e) => {
            report.error(file, e);
            return None;
        }
    };
    if ase.depth != 8 {
        report.error(file, "the sprite isn't in indexed colour mode any more; convert it back (Sprite > Color Mode > Indexed) with the original palette");
        return None;
    }
    let (ax, ay, _, _) = canvas(base);
    let total: usize = base.animations.iter().map(Vec::len).sum();
    if ase.frames.len() != total {
        report.error(file, format!("{} frames, but the animations have {total}: add or remove frames in animations.json first", ase.frames.len()));
        return None;
    }
    // Layers by part number, from their names (a layer can be moved).
    let mut part_layer: HashMap<usize, u16> = HashMap::new();
    for (i, l) in ase.layers.iter().enumerate() {
        if l.kind != 0 {
            report.error(file, format!("layer {:?} is a group or tilemap layer; parts are plain layers", l.name));
            return None;
        }
        let n = l.name.strip_prefix("part ").and_then(|r| r.split(' ').next()).and_then(|n| n.parse().ok());
        match n {
            Some(n) => {
                part_layer.insert(n, i as u16);
            }
            None => report.warn(file, format!("layer {:?} isn't a part (name it \"part N\"); ignored", l.name)),
        }
    }
    // Palette set 0 from the sprite's palette.
    let mut out = base.clone();
    let rgb: Vec<[u8; 3]> = ase.palette.iter().map(|c| [c[0], c[1], c[2]]).collect();
    let rows = out.palette_sets.first().map_or(0, Vec::len).min(crate::sprite::ATLAS_PALETTE_ROWS);
    let base_rgb = crate::image::palette_rgb(out.palette_sets.first().map_or(&[][..], |set| &set[..rows]));
    if let Some(why) = crate::image::palette_change(&crate::image::palette_fingerprint(&base_rgb), &rgb, rows * 16) {
        report.error(file, why);
        return None;
    }
    let (new_rows, off_grid) = crate::image::palette_rows(&rgb, rows);
    if !off_grid.is_empty() {
        report.warn(file, format!("{} palette colours aren't GBA colours and were rounded", off_grid.len()));
    }
    if let Some(set) = out.palette_sets.first_mut() {
        for (r, row) in set.iter_mut().enumerate().take(rows) {
            for (i, c) in row.iter_mut().enumerate() {
                *c = (*c & 0x8000) | new_rows[r][i];
            }
        }
    }
    // Pixels: every part of every frame writes its tiles; frames that share
    // tiles must agree.
    let mut written: Vec<Vec<Option<(usize, u8)>>> =
        out.tilesets.iter().map(|t| vec![None; t.pixels.len()]).collect();
    let mut tiles: Vec<Tiles> = out.tilesets.clone();
    let mut conflicts = 0;
    let mut outside = 0;
    let mut f = 0;
    for (a, anim) in base.animations.iter().enumerate() {
        for (i, fr) in anim.iter().enumerate() {
            let frame = &ase.frames[f];
            // A linked cel shares the image and place of its frame's cel.
            let cel_of = |layer: u16| -> Option<&AseCel> {
                let c = frame.cels.iter().find(|c| c.layer == layer)?;
                match c.content {
                    CelContent::Linked(to) => ase.frames.get(to as usize)?.cels.iter().find(|c| c.layer == layer),
                    CelContent::Image { .. } => Some(c),
                }
            };
            for (k, p) in base.part_lists[fr.parts as usize].iter().enumerate() {
                let cel = part_layer.get(&k).and_then(|&l| cel_of(l));
                let (w, h) = (p.width as u32, p.height as u32);
                let (px, py) = (ax + p.x as i32, ay + p.y as i32);
                let at = |u: u32, v: u32| -> u8 {
                    let Some(c) = cel else { return 0 };
                    let CelContent::Image { width, height, pixels } = &c.content else { return 0 };
                    let (cx, cy) = (px + u as i32 - c.x as i32, py + v as i32 - c.y as i32);
                    if cx < 0 || cy < 0 || cx >= *width as i32 || cy >= *height as i32 {
                        return 0;
                    }
                    pixels[(cy as usize) * (*width as usize) + cx as usize]
                };
                if let Some(c) = cel
                    && let CelContent::Image { width, height, pixels } = &c.content
                {
                    // Opaque pixels outside the part's rectangle.
                    for cy in 0..*height as i32 {
                        for cx in 0..*width as i32 {
                            let (u, v) = (c.x as i32 + cx - px, c.y as i32 + cy - py);
                            if pixels[(cy * *width as i32 + cx) as usize] != ase.transparent
                                && (u < 0 || v < 0 || u >= w as i32 || v >= h as i32)
                            {
                                outside += 1;
                            }
                        }
                    }
                }
                for v in 0..h {
                    for u in 0..w {
                        let (t, idx) = source(p, u, v);
                        let raw = at(u, v);
                        let c = if raw == ase.transparent { 0 } else { raw & 15 };
                        let slot = t * Tiles::TILE + idx;
                        let Some(dst) = tiles[fr.tileset as usize].pixels.get_mut(slot) else { continue };
                        match written[fr.tileset as usize][slot] {
                            Some((_, old)) if old != c => conflicts += 1,
                            _ => {}
                        }
                        written[fr.tileset as usize][slot] = Some((f, c));
                        *dst = c;
                    }
                }
            }
            // Duration and loop flag.
            let (ticks, exact) = ticks_from_ms(frame.duration_ms);
            if !exact {
                report.note(file, format!("animation {a} frame {i}: {} ms is {ticks} ticks, rounded", frame.duration_ms));
            }
            out.animations[a][i].duration = ticks;
            f += 1;
        }
        // The tag's name says whether the animation loops ("anim NN loop");
        // its repeat count only drives the editor's preview, and editors
        // before Aseprite 1.3 (and LibreSprite) don't keep it.
        let number = |t: &AseTag| t.name.split_whitespace().nth(1).and_then(|n| n.parse::<usize>().ok());
        match ase.tags.iter().find(|t| t.name.starts_with("anim ") && number(t) == Some(a)) {
            Some(tag) => {
                let looping = tag.name.split_whitespace().nth(2) == Some("loop");
                if looping != (tag.repeat == 0) {
                    report.warn(
                        file,
                        format!(
                            "tag {:?}: its name decides whether the animation loops ({}); its repeat setting ({}) disagrees",
                            tag.name,
                            if looping { "it does" } else { "it doesn't" },
                            tag.repeat
                        ),
                    );
                }
                let last = out.animations[a].last_mut().unwrap();
                if looping {
                    last.flags |= crate::sprite::FLAG_LOOP;
                } else {
                    last.flags &= !crate::sprite::FLAG_LOOP;
                }
            }
            None => report.warn(file, format!("no tag \"anim {a:02}\"; animation {a} keeps its loop flag")),
        }
    }
    if outside > 0 {
        report.warn(file, format!("{outside} pixels are outside their part's rectangle and aren't kept (parts can't grow in this view; change sprite.json)"));
    }
    if conflicts > 0 {
        report.error(
            file,
            format!("{conflicts} pixels of tiles that several frames share differ between those frames; edit linked cels, or make them agree"),
        );
        return None;
    }
    out.tilesets = tiles;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_survive_milliseconds() {
        for t in 1..=255u8 {
            assert_eq!(ticks_from_ms(ms_from_ticks(t)), (t, true));
        }
        assert_eq!(ticks_from_ms(50), (3, true));
        assert_eq!(ticks_from_ms(55), (3, false));
    }
}
