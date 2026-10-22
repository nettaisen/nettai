//! The frames' drawings rasterized ahead of play (docs/design/
//! text-rendering.md §13.2): each drawing's file read once, and rasterized
//! once at the output's scale, when the pack is loaded (`Rasters::prepare`)
//! or before the first picture, and again, away from the frames, when the
//! scale changes (the old rasters, resampled, stand in until then). A
//! raster keeps each palette entry's share of every pixel, so a frame
//! colors it in the palette of the moment (a fade, a dimming, a flash, a
//! version's colors) without drawing the file again; what it colored is
//! kept while the colors hold. A drawing is laid at whole output pixels.

use nettai_assets::shapes::Drawing;
use nettai_content::shapes::{Parsed, Raster};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

/// Something painted at the output's scale, unplaced: each pixel's color
/// (0RGB) and coverage, `w` pixels a row, its top left (`x`, `y`) from
/// where it is laid.
#[derive(Default)]
pub struct Image {
    pub x: i32,
    pub y: i32,
    pub w: usize,
    pub h: usize,
    pub px: Vec<(u32, u8)>,
}

/// What a painted image is known by: a drawing colored, or a text item.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ImageKey {
    Drawing(TintKey),
    /// A text item by all that makes its pixels (`vfont`'s `draw_item`).
    Text(String),
}

/// The images painted, by what they are, the least recently used let go
/// first: one cache for the drawings and the text.
#[derive(Default)]
pub struct Images(Lru<ImageKey, Arc<Image>>);

impl Images {
    pub fn get(&mut self, k: &ImageKey) -> Option<Arc<Image>> {
        self.0.get(k)
    }

    pub fn put(&mut self, k: ImageKey, v: Arc<Image>) {
        let bytes = v.px.len() * 8 + 64;
        self.0.put(k, v, bytes);
    }
}

/// What a colored drawing is known by: the drawing, the raster's scale and
/// the size it is drawn at (bits), its flips, and its entries' colors.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TintKey {
    hash: u64,
    raster: u32,
    scale: u32,
    vscale: u32,
    flip: (bool, bool),
    colors: Vec<u32>,
}

/// Rasters by drawing (`Drawing::hash`).
type Made = HashMap<u64, Arc<Raster>>;

/// The rasters of the drawings a renderer draws.
#[derive(Default)]
pub struct Rasters {
    /// Each drawing known, its file read (once).
    parsed: HashMap<u64, Arc<Parsed>>,
    /// The rasters, all at `scale` (0: none made yet).
    scale: f32,
    rasters: Made,
    /// Rasters being made at another scale, away from the frames.
    pending: Option<(f32, Receiver<Made>)>,
    /// Drawings rasterized while a frame was drawn (none known before it).
    pub late: u64,
    /// The drawings last given (`prepare_all`).
    given: Option<Arc<Vec<Drawing>>>,
}

impl Rasters {
    /// Know `drawings` (each file read once, now) and rasterize them at
    /// `scale` (none: when the first frame says the scale).
    pub fn prepare(&mut self, drawings: &[&Drawing], scale: Option<f32>) {
        for d in drawings {
            self.know(d);
        }
        if let Some(s) = scale {
            self.rasterize_all(s);
        }
    }

    /// The packs' drawings (`Frame::drawings`) ready to draw at `scale`: read
    /// and rasterized the first time they are given (the first picture,
    /// or a preload's), rasterized again away from the frames at a new
    /// scale.
    pub fn prepare_all(&mut self, drawings: &Arc<Vec<Drawing>>, scale: f32) {
        if !self.given.as_ref().is_some_and(|g| Arc::ptr_eq(g, drawings)) {
            for d in drawings.iter() {
                self.know(d);
            }
            self.given = Some(drawings.clone());
            if self.scale == scale || self.scale == 0.0 {
                let missing: Vec<u64> = self.parsed.keys().filter(|h| !self.rasters.contains_key(h)).copied().collect();
                for h in missing {
                    let r = self.parsed[&h].raster(scale, 1.0);
                    self.rasters.insert(h, Arc::new(r));
                }
                self.scale = scale;
            }
        }
        self.at_scale(scale);
    }

    fn know(&mut self, d: &Drawing) -> Option<Arc<Parsed>> {
        if let Some(p) = self.parsed.get(&d.hash) {
            return Some(p.clone());
        }
        let p = Arc::new(Parsed::of(d).ok()?);
        self.parsed.insert(d.hash, p.clone());
        Some(p)
    }

    /// Every known drawing rasterized at `scale`, now.
    fn rasterize_all(&mut self, scale: f32) {
        if self.scale == scale && self.rasters.len() == self.parsed.len() {
            return;
        }
        self.rasters = self.parsed.iter().map(|(&h, p)| (h, Arc::new(p.raster(scale, 1.0)))).collect();
        self.scale = scale;
        self.pending = None;
    }

    /// Ready to draw at `scale`: the first frame rasterizes every known
    /// drawing; a new scale rasterizes them again away from the frames
    /// (drawn from the old ones, resampled, until they are made).
    fn at_scale(&mut self, scale: f32) {
        if self.scale == 0.0 {
            self.rasterize_all(scale);
            return;
        }
        if let Some((s, rx)) = &self.pending {
            if let Ok(made) = rx.try_recv() {
                let s = *s;
                self.rasters = made;
                self.scale = s;
                self.pending = None;
            } else if *s == scale {
                return;
            }
        }
        if self.scale == scale || self.pending.as_ref().is_some_and(|(s, _)| *s == scale) {
            return;
        }
        let parsed: Vec<(u64, Arc<Parsed>)> = self.parsed.iter().map(|(&h, p)| (h, p.clone())).collect();
        let (tx, rx) = std::sync::mpsc::channel();
        // (None started: the old scale's rasters stay, resampled.)
        let _ = nettai_platform::thread::spawn("rasters", move || {
            let made = parsed.into_iter().map(|(h, p)| (h, Arc::new(p.raster(scale, 1.0)))).collect();
            let _ = tx.send(made);
        });
        self.pending = Some((scale, rx));
    }

    /// `d` colored (`colors`: its entries' colors, 0RGB) and flipped, at
    /// `scale` output pixels a frame pixel, `vscale` times that down.
    pub fn tinted(&mut self, images: &mut Images, d: &Drawing, colors: &[u32], flip: (bool, bool), scale: f32, vscale: f32) -> Option<Arc<Image>> {
        self.at_scale(scale);
        if !self.rasters.contains_key(&d.hash) {
            // (A drawing no one said was coming: rasterized now.)
            let p = self.know(d)?;
            self.late += 1;
            self.rasters.insert(d.hash, Arc::new(p.raster(self.scale, 1.0)));
        }
        let raster = self.rasters.get(&d.hash)?.clone();
        let key = ImageKey::Drawing(TintKey { hash: d.hash, raster: self.scale.to_bits(), scale: scale.to_bits(), vscale: vscale.to_bits(), flip, colors: colors.to_vec() });
        if let Some(t) = images.get(&key) {
            return Some(t);
        }
        let mut t = tint(&raster, colors, flip);
        let (fx, fy) = (scale / self.scale, scale * vscale / self.scale);
        if fx != 1.0 || fy != 1.0 {
            t = resample(&t, fx, fy);
        }
        let t = Arc::new(t);
        images.put(key, t.clone());
        Some(t)
    }
}

/// A raster in its entries' colors, flipped as asked (the whole raster's
/// size, clear past its covered box).
fn tint(r: &Raster, colors: &[u32], (hflip, vflip): (bool, bool)) -> Image {
    let mut px = vec![(0u32, 0u8); r.w * r.h];
    let [ax, ay, aw, ah] = r.area;
    for y in ay..ay + ah {
        for x in ax..ax + aw {
            let a = r.alpha[(y - ay) * aw + x - ax];
            if a == 0 {
                continue;
            }
            let (mut rr, mut gg, mut bb) = (0u32, 0u32, 0u32);
            for (share, &c) in r.shares.iter().zip(colors) {
                let w = share.at(x, y) as u32;
                if w != 0 {
                    rr += w * ((c >> 16) & 0xFF);
                    gg += w * ((c >> 8) & 0xFF);
                    bb += w * (c & 0xFF);
                }
            }
            // (Premultiplied by the coverage, out of 255 twice: back to the
            // color under it.)
            let a32 = a as u32;
            let un = |v: u32| ((v + a32 / 2) / a32).min(255);
            let (ox, oy) = (if hflip { r.w - 1 - x } else { x }, if vflip { r.h - 1 - y } else { y });
            px[oy * r.w + ox] = ((un(rr) << 16) | (un(gg) << 8) | un(bb), a);
        }
    }
    Image { x: 0, y: 0, w: r.w, h: r.h, px }
}

/// A colored drawing resized `fx` by `fy` times (bilinear, its colors by
/// their coverage).
fn resample(t: &Image, fx: f32, fy: f32) -> Image {
    let w = ((t.w as f32 * fx).ceil() as usize).max(1);
    let h = ((t.h as f32 * fy).ceil() as usize).max(1);
    let at = |x: i64, y: i64| -> [f32; 4] {
        if x < 0 || y < 0 || x as usize >= t.w || y as usize >= t.h {
            return [0.0; 4];
        }
        let (c, a) = t.px[y as usize * t.w + x as usize];
        let a = a as f32;
        [((c >> 16) & 0xFF) as f32 * a, ((c >> 8) & 0xFF) as f32 * a, (c & 0xFF) as f32 * a, a]
    };
    let mut px = vec![(0u32, 0u8); w * h];
    for y in 0..h {
        let sy = (y as f32 + 0.5) / fy - 0.5;
        let (y0, ty) = (sy.floor() as i64, sy - sy.floor());
        for x in 0..w {
            let sx = (x as f32 + 0.5) / fx - 0.5;
            let (x0, tx) = (sx.floor() as i64, sx - sx.floor());
            let mut v = [0f32; 4];
            for (dx, dy, k) in [(0, 0, (1.0 - tx) * (1.0 - ty)), (1, 0, tx * (1.0 - ty)), (0, 1, (1.0 - tx) * ty), (1, 1, tx * ty)] {
                let p = at(x0 + dx, y0 + dy);
                for c in 0..4 {
                    v[c] += p[c] * k;
                }
            }
            if v[3] < 0.5 {
                continue;
            }
            let un = |c: f32| (c / v[3]).round().clamp(0.0, 255.0) as u32;
            px[y * w + x] = ((un(v[0]) << 16) | (un(v[1]) << 8) | un(v[2]), v[3].round().min(255.0) as u8);
        }
    }
    Image { x: 0, y: 0, w, h, px }
}

/// A cache of at most `BUDGET` bytes, the least recently used let go first.
pub struct Lru<K, V> {
    map: HashMap<K, (V, u64, usize)>,
    tick: u64,
    bytes: usize,
}

impl<K, V> Default for Lru<K, V> {
    fn default() -> Self {
        Lru { map: HashMap::new(), tick: 0, bytes: 0 }
    }
}

/// What the images kept may take: a custom screen, the HUD, the chatbox and
/// their text at a 4K output's scale, several times over.
const BUDGET: usize = 256 << 20;

impl<K: std::hash::Hash + Eq + Clone, V: Clone> Lru<K, V> {
    fn get(&mut self, k: &K) -> Option<V> {
        self.tick += 1;
        let tick = self.tick;
        self.map.get_mut(k).map(|e| {
            e.1 = tick;
            e.0.clone()
        })
    }

    fn put(&mut self, k: K, v: V, bytes: usize) {
        self.tick += 1;
        while self.bytes + bytes > BUDGET && !self.map.is_empty() {
            let Some(old) = self.map.iter().min_by_key(|(_, e)| e.1).map(|(k, _)| k.clone()) else { break };
            if let Some(e) = self.map.remove(&old) {
                self.bytes -= e.2;
            }
        }
        self.bytes += bytes;
        if let Some(e) = self.map.insert(k, (v, self.tick, bytes)) {
            self.bytes -= e.2;
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Drawing {
        nettai_content::shapes::from_svg("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 8 8\"><rect width=\"8\" height=\"8\" data-color=\"1\"/><rect x=\"2\" y=\"2\" width=\"4\" height=\"4\" data-color=\"2\"/></svg>").unwrap()
    }

    /// Rasterized once, colored in the colors of the moment; at a new scale
    /// drawn from the old raster resampled until the new one is made away
    /// from the frames, then from that.
    #[test]
    fn colored_from_one_raster_and_made_again_at_a_new_scale() {
        let d = square();
        let mut r = Rasters::default();
        let mut i = Images::default();
        r.prepare_all(&Arc::new(vec![d.clone()]), 2.0);
        let t = r.tinted(&mut i, &d, &[0xFF0000, 0x00FF00], (false, false), 2.0, 1.0).unwrap();
        assert_eq!((t.w, t.h), (16, 16));
        assert_eq!(t.px[0], (0xFF0000, 255));
        assert_eq!(t.px[8 * 16 + 8], (0x00FF00, 255));
        let t = r.tinted(&mut i, &d, &[0x0000FF, 0xFFFFFF], (false, false), 2.0, 1.0).unwrap();
        assert_eq!((t.px[0], t.px[8 * 16 + 8]), ((0x0000FF, 255), (0xFFFFFF, 255)));
        assert_eq!(r.late, 0);
        let t = r.tinted(&mut i, &d, &[0xFF0000, 0x00FF00], (false, false), 4.0, 1.0).unwrap();
        assert_eq!((t.w, t.h), (32, 32), "resampled meanwhile");
        let deadline = nettai_platform::time::Instant::now() + std::time::Duration::from_secs(10);
        while r.scale != 4.0 && nettai_platform::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
            r.tinted(&mut i, &d, &[0xFF0000, 0x00FF00], (false, false), 4.0, 1.0);
        }
        assert_eq!(r.scale, 4.0, "made again away from the frames");
        assert_eq!(r.late, 0);
        assert_eq!(r.parsed.len(), 1, "read once");
    }
}
