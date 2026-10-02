//! The text layer's font and its drawing: a TrueType or OpenType outline
//! font, static or variable, laid out in the frame's pixels (shaped with
//! the font's kerning) and rasterized with antialiasing at the output's
//! resolution, over the scaled frame (swash). docs/design/text-rendering.md,
//! "As built".
//!
//! The bundled font is Murecho (fonts/murecho, SIL Open Font License 1.1):
//! a sans with Latin, Greek, Cyrillic, kana and some 2,300 kanji, in one
//! variable file with a weight axis. A font with a width axis (`--font`)
//! draws a name too wide for the original's box in a narrower cut of
//! itself before squeezing it.

use crate::compose::{HEIGHT, WIDTH, apply_fade, to_rgb};
use crate::present::Placement;
use crate::textlayer::{Align, Rect, TextItem};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use swash::scale::image::Image;
use swash::scale::{Render, ScaleContext, Source};
use swash::shape::ShapeContext;
use swash::zeno::{Format, Transform, Vector};
use swash::{CacheKey, FontRef, GlyphId, Setting};

/// Murecho, its variable cut (weights 100 to 900; 1.4 MB).
const BUNDLED: &[u8] = include_bytes!("../fonts/murecho/Murecho-VariableFont_wght.ttf");

/// The narrowest width a font with a width axis condenses a name to
/// before it squeezes it (the axis's "condensed").
const NARROWEST: f32 = 75.0;
/// A string is squeezed horizontally down to this before it is made
/// smaller; one with kana or kanji down to `WIDE_SQUEEZE`.
const SQUEEZE: f32 = 0.85;
const WIDE_SQUEEZE: f32 = 0.7;

/// Whether a character is a wide one (kana, kanji, full-width forms).
fn wide(c: char) -> bool {
    matches!(c, '\u{3000}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}' | '\u{FF00}'..='\u{FFEF}')
}
/// A button's letter (Ⓐ drawn by the text layer) against the text's size,
/// and its ring's diameter and stroke against the capitals' height.
const BUTTON_LETTER: f32 = 0.68;
const BUTTON_RING: f32 = 1.15;
const BUTTON_STROKE: f32 = 0.1;
/// A stacked mark's letters (EX): the gap between them against the
/// capitals' height (each letter's capitals take half of the rest), how
/// much wider than the font's they are drawn (the game's stacked letters
/// are as wide as its others, half as high), and the weight added to the
/// text's so their strokes hold at that size.
const STACK_GAP: f32 = 0.14;
const STACK_STRETCH: f32 = 1.3;
const STACK_WEIGHT: f32 = 100.0;
/// The shadow's offset, right and down, in frame pixels (the original's
/// is one pixel).
const SHADOW: f32 = 0.5;
/// Glyphs smaller than this (pixels an em at the output) are hinted.
const HINT_BELOW: u32 = 40;
/// Glyph images kept before the cache starts over.
const CACHE_GLYPHS: usize = 4096;

/// What a string is drawn as: which of the original's fonts it stands in
/// for, which sets its size, its place in its box and its weight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// The 8x16 font's strings (names, the telop, the HUD's lines): a cell
    /// 16 rows high whose capitals take rows 4 to 13; bold, with a shadow.
    Cell,
    /// The dialogue font's (the chatbox): lines 12 rows high whose
    /// capitals take rows 0 to 10.
    Dialogue,
}

#[derive(Clone, Copy, Debug)]
struct Style {
    /// Capital height, in frame pixels.
    cap: f32,
    /// The baseline, down from the box's top, in frame pixels.
    baseline: f32,
    /// The weight axis's value (a static font has its own).
    weight: f32,
    /// What the string leaves free at its box's right, in frame pixels:
    /// the space the original's glyphs keep before what follows them (a
    /// name's damage digits).
    margin: f32,
}

impl Role {
    fn style(self) -> Style {
        match self {
            Role::Cell => Style { cap: 10.0, baseline: 14.0, weight: 700.0, margin: 1.0 },
            Role::Dialogue => Style { cap: 9.0, baseline: 10.5, weight: 300.0, margin: 0.0 },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Axis {
    min: f32,
    default: f32,
    max: f32,
}

/// A font file the text layer draws with.
pub struct VectorFont {
    data: Cow<'static, [u8]>,
    offset: u32,
    key: CacheKey,
    width: Option<Axis>,
    weight: Option<Axis>,
    /// Capital height over the em.
    cap: f32,
    pub name: String,
}

impl std::fmt::Debug for VectorFont {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("VectorFont").field("name", &self.name).finish()
    }
}

impl VectorFont {
    /// The font the frontend comes with.
    pub fn bundled() -> VectorFont {
        VectorFont::from_data(Cow::Borrowed(BUNDLED), "Murecho (bundled)".into()).expect("the bundled font parses")
    }

    /// A TrueType or OpenType font file (its first face).
    pub fn load(path: &Path) -> Result<VectorFont, String> {
        let data = std::fs::read(path).map_err(|e| format!("can't read the font {}: {e}", path.display()))?;
        VectorFont::from_data(Cow::Owned(data), path.display().to_string())
    }

    fn from_data(data: Cow<'static, [u8]>, name: String) -> Result<VectorFont, String> {
        let f = FontRef::from_index(&data, 0).ok_or_else(|| format!("{name} is not a TrueType or OpenType font"))?;
        let (offset, key) = (f.offset, f.key);
        let axis = |tag: &[u8; 4]| {
            let tag = swash::tag_from_bytes(tag);
            f.variations()
                .find(|v| v.tag() == tag)
                .map(|v| Axis { min: v.min_value(), default: v.default_value(), max: v.max_value() })
        };
        let (width, weight) = (axis(b"wdth"), axis(b"wght"));
        let m = f.metrics(&[]);
        let cap = if m.cap_height > 0.0 { m.cap_height / m.units_per_em as f32 } else { 0.7 };
        if f.charmap().map('A') == 0 {
            return Err(format!("{name} has no Latin letters"));
        }
        Ok(VectorFont { data, offset, key, width, weight, cap, name })
    }

    fn font(&self) -> FontRef<'_> {
        FontRef { data: &self.data, offset: self.offset, key: self.key }
    }

    /// Whether the text layer can draw every character of `text`: the font
    /// has it, or it is a mark the layer draws of letters the font has
    /// ([`mark`]); spaces always.
    pub fn covers(&self, text: &str) -> bool {
        let cmap = self.font().charmap();
        let has = |c: char| c.is_whitespace() || cmap.map(c) != 0;
        text.chars().all(|c| has(c) || mark(c).is_some_and(|m| m.letters().into_iter().all(has)))
    }
}

/// A mark of the game's the text layer draws itself when the font has no
/// glyph for it (docs/design/text-rendering.md §10.5: the game's marks are
/// characters, those Unicode has none for in the Private Use Area).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    /// A button: its letter in a ring (Ⓐ, Ⓑ, Ⓛ, Ⓡ).
    Button(char),
    /// Two letters stacked in one cell, the first on top, as the game's
    /// glyph has them (the 8x16 font's RV, BX, EX, SP and FZ, the dialogue
    /// font's MB).
    Stacked(char, char),
}

impl Mark {
    fn letters(self) -> Vec<char> {
        match self {
            Mark::Button(c) => vec![c],
            Mark::Stacked(a, b) => vec![a, b],
        }
    }
}

/// How a mark is drawn, if the text layer draws it (the bat, `End` and the
/// rest have no drawing: a string with one the font lacks is drawn in the
/// game's font).
fn mark(c: char) -> Option<Mark> {
    Some(match c {
        'Ⓐ' => Mark::Button('A'),
        'Ⓑ' => Mark::Button('B'),
        'Ⓛ' => Mark::Button('L'),
        'Ⓡ' => Mark::Button('R'),
        '\u{E000}' => Mark::Stacked('R', 'V'),
        '\u{E001}' => Mark::Stacked('B', 'X'),
        '\u{E002}' => Mark::Stacked('E', 'X'),
        '\u{E003}' => Mark::Stacked('S', 'P'),
        '\u{E004}' => Mark::Stacked('F', 'Z'),
        '\u{E005}' => Mark::Stacked('M', 'B'),
        _ => return None,
    })
}

/// A glyph laid out: frame pixels from the box's left and up from the
/// baseline.
#[derive(Clone, Copy, Debug)]
struct Placed {
    id: GlyphId,
    x: f32,
    y: f32,
    /// The em size, in frame pixels.
    size: f32,
    width: Option<f32>,
    weight: Option<f32>,
    /// Drawn this much wider (a stacked mark's letters).
    stretch: f32,
    /// The string's character it draws (what the chatbox's printing
    /// counts).
    unit: usize,
}

/// A button mark's ring, in frame pixels as `Placed`: its centre and outer
/// radius.
#[derive(Clone, Copy, Debug)]
struct Ring {
    x: f32,
    y: f32,
    radius: f32,
    unit: usize,
}

/// A string laid out to fit its box.
#[derive(Clone, Debug, Default)]
struct Layout {
    glyphs: Vec<Placed>,
    rings: Vec<Ring>,
    width: f32,
    /// How far the baseline moves down (a string made smaller stays
    /// centred on its capitals).
    drop: f32,
    /// A static font's horizontal squeeze.
    xscale: f32,
}

/// A glyph image's cache key: sizes in 64ths of an output pixel, axes in
/// 16ths, scales in 1024ths, the pen's subpixel place in quarters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GlyphKey {
    id: GlyphId,
    size: u32,
    width: i32,
    weight: i32,
    xscale: i32,
    yscale: i32,
    sub: u8,
}

/// Lays out and draws text items at the output's resolution, keeping the
/// layouts and glyph images it made.
pub struct TextRenderer {
    font: std::sync::Arc<VectorFont>,
    scaler: ScaleContext,
    shaper: ShapeContext,
    layouts: HashMap<(String, Role, i32), Layout>,
    glyphs: HashMap<GlyphKey, Option<Image>>,
}

impl TextRenderer {
    pub fn new(font: std::sync::Arc<VectorFont>) -> TextRenderer {
        TextRenderer { font, scaler: ScaleContext::new(), shaper: ShapeContext::new(), layouts: HashMap::new(), glyphs: HashMap::new() }
    }

    pub fn font(&self) -> &VectorFont {
        &self.font
    }

    /// The width a string takes, in frame pixels, once fitted into a box
    /// `room` wide.
    pub fn fitted_width(&mut self, text: &str, role: Role, room: i32) -> f32 {
        self.layout(text, role, room).width
    }

    fn layout(&mut self, text: &str, role: Role, room: i32) -> &Layout {
        let key = (text.to_string(), role, room);
        if !self.layouts.contains_key(&key) {
            if self.layouts.len() > 1024 {
                self.layouts.clear();
            }
            let l = self.fit(text, role, (room as f32 - role.style().margin).max(1.0));
            self.layouts.insert(key.clone(), l);
        }
        &self.layouts[&key]
    }

    /// Lay `text` out within `room` frame pixels: at the role's size and
    /// the font's normal width if it fits; else, with a width axis, in
    /// narrower cuts down to [`NARROWEST`]; else squeezed horizontally down
    /// to [`SQUEEZE`] ([`WIDE_SQUEEZE`] for a string with kana or kanji,
    /// whose glyphs are an em wide where the original's cells are half
    /// that); else that, smaller. Never wider than the room.
    fn fit(&mut self, text: &str, role: Role, room: f32) -> Layout {
        let st = role.style();
        let size = st.cap / self.font.cap;
        let weight = self.font.weight.map(|a| st.weight.clamp(a.min, a.max));
        let mut width = self.font.width.map(|a| a.default.clamp(a.min, a.max));
        let mut l = self.shape(text, size, width, weight);
        if l.width <= room || l.width <= 0.0 {
            return l;
        }
        if let Some(a) = self.font.width {
            let narrowest = NARROWEST.clamp(a.min, a.default);
            for k in 1..=8 {
                width = Some(a.default + (narrowest - a.default) * k as f32 / 8.0);
                l = self.shape(text, size, width, weight);
                if l.width <= room {
                    return l;
                }
            }
        }
        let least = if text.chars().any(wide) { WIDE_SQUEEZE } else { SQUEEZE };
        let x = room / l.width;
        let (xscale, f) = if x >= least { (x, 1.0) } else { (least, x / least) };
        if f < 1.0 {
            l = self.shape(text, size * f, width, weight);
            l.drop = st.cap * (1.0 - f) / 2.0;
        }
        for g in &mut l.glyphs {
            g.x *= xscale;
        }
        for r in &mut l.rings {
            r.x *= xscale;
        }
        l.width = (l.width * xscale).min(room);
        l.xscale = xscale;
        l
    }

    /// Shape `text` at `size` (frame pixels an em): runs of characters with
    /// the font's kerning; a mark the font has no glyph for as the layer
    /// draws it ([`mark`]): a button's letter, smaller, in a ring centred on
    /// the capitals, or a stacked mark's two letters, each half the
    /// capitals' height less a gap, the first above the second, filling
    /// one cell between the capitals' top and the baseline.
    fn shape(&mut self, text: &str, size: f32, width: Option<f32>, weight: Option<f32>) -> Layout {
        let font = self.font.font();
        let cmap = font.charmap();
        let cap = self.font.cap;
        let heavier = self.font.weight.zip(weight).map(|(a, w)| (w + STACK_WEIGHT).min(a.max));
        let settings = |weight: Option<f32>| {
            let mut s: Vec<Setting<f32>> = Vec::new();
            if let Some(w) = width {
                s.push(("wdth", w).into());
            }
            if let Some(w) = weight {
                s.push(("wght", w).into());
            }
            s
        };
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let drawn: Vec<Option<Mark>> = chars.iter().map(|&(_, c)| if cmap.map(c) == 0 { mark(c) } else { None }).collect();
        let mut out = Layout { xscale: 1.0, ..Layout::default() };
        let mut pen = 0.0f32;
        let gap = size * 0.05;
        let mut i = 0;
        while i < chars.len() {
            match drawn[i] {
                Some(Mark::Button(letter)) => {
                    let ring = cap * size * BUTTON_RING;
                    let small = size * BUTTON_LETTER;
                    let (glyphs, advance) = letter_glyphs(&mut self.shaper, font, letter, small, &settings(weight));
                    let (x, y) = (pen + gap + ring / 2.0, cap * size / 2.0);
                    for (id, gx, gy) in glyphs {
                        let (gx, gy) = (x - advance / 2.0 + gx, y - cap * small / 2.0 + gy);
                        out.glyphs.push(Placed { id, x: gx, y: gy, size: small, width, weight, stretch: 1.0, unit: i });
                    }
                    out.rings.push(Ring { x, y, radius: ring / 2.0, unit: i });
                    pen += ring + 2.0 * gap;
                    i += 1;
                }
                Some(Mark::Stacked(top, bottom)) => {
                    let between = cap * size * STACK_GAP;
                    let letter_cap = (cap * size - between) / 2.0;
                    let small = letter_cap / cap;
                    let s = settings(heavier);
                    let (top, top_w) = letter_glyphs(&mut self.shaper, font, top, small, &s);
                    let (bottom, bottom_w) = letter_glyphs(&mut self.shaper, font, bottom, small, &s);
                    let (top_w, bottom_w) = (top_w * STACK_STRETCH, bottom_w * STACK_STRETCH);
                    let cell = top_w.max(bottom_w);
                    let x0 = pen + gap;
                    for (glyphs, w, y) in [(top, top_w, letter_cap + between), (bottom, bottom_w, 0.0)] {
                        for (id, gx, gy) in glyphs {
                            let gx = x0 + (cell - w) / 2.0 + gx * STACK_STRETCH;
                            out.glyphs.push(Placed { id, x: gx, y: y + gy, size: small, width, weight: heavier, stretch: STACK_STRETCH, unit: i });
                        }
                    }
                    pen = x0 + cell + gap;
                    i += 1;
                }
                None => {
                    let j = (i + 1..chars.len()).find(|&k| drawn[k].is_some()).unwrap_or(chars.len());
                    let (start, end) = (chars[i].0, chars.get(j).map_or(text.len(), |c| c.0));
                    let s = settings(weight);
                    let mut b = self.shaper.builder(font).size(size).variations(s.iter().copied()).build();
                    b.add_str(&text[start..end]);
                    b.shape_with(|c| {
                        let at = start + c.source.start as usize;
                        let unit = chars.partition_point(|&(k, _)| k <= at).saturating_sub(1);
                        for g in c.glyphs {
                            out.glyphs.push(Placed { id: g.id, x: pen + g.x, y: g.y, size, width, weight, stretch: 1.0, unit });
                            pen += g.advance;
                        }
                    });
                    i = j;
                }
            }
        }
        out.width = pen;
        out
    }

    fn glyph(&mut self, key: GlyphKey) -> Option<&Image> {
        if !self.glyphs.contains_key(&key) {
            if self.glyphs.len() >= CACHE_GLYPHS {
                self.glyphs.clear();
            }
            let font = self.font.font();
            let mut settings: Vec<Setting<f32>> = Vec::new();
            if self.font.width.is_some() {
                settings.push(("wdth", key.width as f32 / 16.0).into());
            }
            if self.font.weight.is_some() {
                settings.push(("wght", key.weight as f32 / 16.0).into());
            }
            let mut scaler = self.scaler.builder(font).size(key.size as f32 / 64.0).hint(key.size < HINT_BELOW * 64).variations(settings).build();
            let (xs, ys) = (key.xscale as f32 / 1024.0, key.yscale as f32 / 1024.0);
            let transform = ((xs, ys) != (1.0, 1.0)).then(|| Transform::scale(xs, ys));
            let image = Render::new(&[Source::Outline])
                .format(Format::Alpha)
                .offset(Vector::new(key.sub as f32 / 4.0, 0.0))
                .transform(transform)
                .render(&mut scaler, key.id);
            self.glyphs.insert(key, image);
        }
        self.glyphs[&key].as_ref()
    }

    /// Draw a frame's text items over its scaled picture in `out` (0RGB,
    /// `out_w` pixels a row), each where the frame's depth keys let it.
    pub fn draw(&mut self, items: &[TextItem], depth: &[u32], place: &Placement, out: &mut [u32], out_w: usize) {
        for item in items {
            self.draw_item(item, depth, place, out, out_w);
        }
    }

    fn draw_item(&mut self, item: &TextItem, depth: &[u32], place: &Placement, out: &mut [u32], out_w: usize) {
        let layout = self.layout(&item.text, item.role, item.rect.w).clone();
        if layout.glyphs.is_empty() {
            return;
        }
        let s = place.scale;
        let st = item.role.style();
        let x0 = item.rect.x as f32
            + match item.align {
                Align::Left => 0.0,
                Align::Centre => ((item.rect.w as f32 - layout.width) / 2.0).round(),
                Align::Right => item.rect.w as f32 - layout.width,
            };
        let baseline = item.rect.y as f32 + st.baseline + layout.drop;
        // The squash: about the box's middle, cut by its rows.
        let (k, baseline, clip) = match item.vscale {
            Some(n) if n > 0 => {
                let k = 256.0 / n as f32;
                let mid = item.rect.y as f32 + item.rect.h as f32 / 2.0;
                let rows = Rect::new(-(WIDTH as i32), item.rect.y, 3 * WIDTH as i32, item.rect.h);
                (k, mid + (baseline - mid) * k, item.clip.intersect(rows))
            }
            _ => (1.0, baseline, item.clip),
        };
        let clip = clip.intersect(Rect::SCREEN);
        if clip.is_empty() {
            return;
        }
        let shown = item.shown.unwrap_or(usize::MAX);
        let colour = |c: u16| to_rgb(apply_fade(apply_fade(c, item.fades[0]), item.fades[1]));
        let mut target = Target { depth, place, out, out_w, clip, key: item.depth };
        let offset = (SHADOW * s).round().max(1.0) as i32;
        let passes: Vec<(u32, i32)> = item.shadow.map(|c| (colour(c), offset)).into_iter().chain([(colour(item.face), 0)]).collect();
        for (rgb, off) in passes {
            for g in layout.glyphs.iter().filter(|g| g.unit < shown) {
                let gx = place.x as f32 + (x0 + g.x) * s;
                let gy = place.y as f32 + (baseline - g.y * k) * s;
                let ix = gx.floor();
                let sub = (((gx - ix) * 4.0) as u8).min(3);
                let key = GlyphKey {
                    id: g.id,
                    size: (g.size * s * 64.0).round() as u32,
                    width: g.width.map_or(0, |w| (w * 16.0).round() as i32),
                    weight: g.weight.map_or(0, |w| (w * 16.0).round() as i32),
                    xscale: (layout.xscale * g.stretch * 1024.0).round() as i32,
                    yscale: (k * 1024.0).round() as i32,
                    sub,
                };
                let Some(image) = self.glyph(key) else { continue };
                let (w, h) = (image.placement.width as i32, image.placement.height as i32);
                let (left, top) = (ix as i32 + image.placement.left + off, gy.round() as i32 - image.placement.top + off);
                for y in 0..h {
                    for x in 0..w {
                        let a = image.data[(y * w + x) as usize];
                        if a != 0 {
                            target.blend(left + x, top + y, rgb, a);
                        }
                    }
                }
            }
            // A button mark's ring: antialiased, its stroke inside its
            // outer edge, squeezed and squashed with the text.
            for r in layout.rings.iter().filter(|r| r.unit < shown) {
                let (rx, ry) = (r.radius * layout.xscale * s, r.radius * k * s);
                let stroke = (BUTTON_STROKE * st.cap * s).max(1.0);
                let cx = place.x as f32 + (x0 + r.x) * s + off as f32;
                let cy = place.y as f32 + (baseline - r.y * k) * s + off as f32;
                let mean = (rx + ry) / 2.0;
                for y in (cy - ry - 1.0).floor() as i32..=(cy + ry + 1.0).ceil() as i32 {
                    for x in (cx - rx - 1.0).floor() as i32..=(cx + rx + 1.0).ceil() as i32 {
                        let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                        // Output pixels out from the outer edge (inside: below 0).
                        let edge = ((dx * dx + dy * dy).sqrt() - 1.0) * mean;
                        let a = (0.5 - edge).clamp(0.0, 1.0) * (0.5 + edge + stroke).clamp(0.0, 1.0);
                        if a > 0.0 {
                            target.blend(x, y, rgb, (a * 255.0).round() as u8);
                        }
                    }
                }
            }
        }
    }
}

/// A letter's glyphs at `size` (frame pixels an em), each with its place
/// from the pen, and its advance.
fn letter_glyphs(shaper: &mut ShapeContext, font: FontRef, letter: char, size: f32, settings: &[Setting<f32>]) -> (Vec<(GlyphId, f32, f32)>, f32) {
    let mut b = shaper.builder(font).size(size).variations(settings.iter().copied()).build();
    b.add_str(letter.encode_utf8(&mut [0; 4]));
    let (mut glyphs, mut pen) = (Vec::new(), 0.0);
    b.shape_with(|c| {
        for g in c.glyphs {
            glyphs.push((g.id, pen + g.x, g.y));
            pen += g.advance;
        }
    });
    (glyphs, pen)
}

/// Where an item's pixels go: the output, through its clip and the frame's
/// depth keys.
struct Target<'o> {
    depth: &'o [u32],
    place: &'o Placement,
    out: &'o mut [u32],
    out_w: usize,
    clip: Rect,
    key: u32,
}

impl Target<'_> {
    fn blend(&mut self, x: i32, y: i32, rgb: u32, a: u8) {
        let Some((fx, fy)) = self.place.frame_pixel(x, y) else { return };
        if fx < self.clip.x || fx >= self.clip.x + self.clip.w || fy < self.clip.y || fy >= self.clip.y + self.clip.h {
            return;
        }
        if self.depth.get(fy as usize * WIDTH + fx as usize).is_some_and(|&d| d < self.key) {
            return;
        }
        debug_assert!((fy as usize) < HEIGHT);
        let i = y as usize * self.out_w + x as usize;
        let Some(o) = self.out.get_mut(i) else { return };
        let a = a as u32;
        let mix = |shift: u32| {
            let (c, b) = ((rgb >> shift) & 0xFF, (*o >> shift) & 0xFF);
            ((c * a + b * (255 - a) + 127) / 255) << shift
        };
        *o = mix(16) | mix(8) | mix(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer() -> TextRenderer {
        TextRenderer::new(std::sync::Arc::new(VectorFont::bundled()))
    }

    /// A stacked mark (EX) is two letters of one size, one above the
    /// other, in one cell between the capitals' top and the baseline; it is
    /// one character of the string.
    #[test]
    fn a_stacked_mark_is_two_letters_in_one_cell() {
        let f = VectorFont::bundled();
        assert!(f.covers("ElecMan\u{E002}") && f.covers("Count\u{E003}"));
        let mut r = renderer();
        let l = r.layout("Count\u{E002}", Role::Cell, 1000).clone();
        let mark: Vec<&Placed> = l.glyphs.iter().filter(|g| g.unit == 5).collect();
        assert_eq!(mark.len(), 2, "E and X");
        let (top, bottom) = (mark[0], mark[1]);
        assert_eq!(top.size, bottom.size);
        assert_eq!(bottom.y, 0.0, "the X on the baseline");
        let cap = Role::Cell.style().cap;
        let letter = top.size * f.cap;
        assert!(top.y > letter && top.y + letter <= cap + 0.01, "the E above, under the capitals' top: {} {letter}", top.y);
        assert!(l.glyphs.iter().filter(|g| g.unit < 5).all(|g| g.size > top.size));
        assert!(l.rings.is_empty());
        // One cell: narrower than the two letters side by side would be.
        let side_by_side = r.fitted_width("CountEX", Role::Cell, 1000) - r.fitted_width("Count", Role::Cell, 1000);
        assert!(l.width - r.fitted_width("Count", Role::Cell, 1000) < side_by_side);
    }

    /// A button the font lacks (Ⓐ) is its letter in a ring the layer
    /// draws; one the font has (Ⓡ) and the symbols (✕, ○) are the font's.
    #[test]
    fn a_button_is_its_letter_in_a_ring_and_a_symbol_the_fonts() {
        let f = VectorFont::bundled();
        assert!(f.covers("Press Ⓐ for 3panl") && f.covers("Ⓑ+Left") && f.covers("攻撃力✕2 ○"));
        // A mark the layer has no drawing of, which the font lacks (the
        // bat): drawn in the game's font.
        assert!(!f.covers("\u{E006}"));
        let mut r = renderer();
        let a = r.layout("Ⓐ", Role::Dialogue, 192).clone();
        assert_eq!((a.glyphs.len(), a.rings.len()), (1, 1));
        let ring = a.rings[0];
        let cap = Role::Dialogue.style().cap;
        assert!((ring.y - cap / 2.0).abs() < 0.01 && ring.radius * 2.0 > cap, "around the capitals");
        for c in ["Ⓡ", "✕", "○"] {
            let l = r.layout(c, Role::Dialogue, 192).clone();
            assert_eq!((l.glyphs.len(), l.rings.len()), (1, 0), "{c}");
            assert!(l.glyphs[0].id != 0, "{c}");
        }
    }

    #[test]
    fn the_bundled_font_has_latin_kana_and_kanji() {
        let f = VectorFont::bundled();
        assert!(f.width.is_none() && f.weight.is_some());
        assert!(f.covers("GunDelS3 Cannon x2 TIME UP! COUNTER HIT!"));
        assert!(f.covers("ミテイ ガンデルソル3 熱斗 電脳獣 ロックマン"));
        // (Not every kanji: a string with one it lacks is drawn in the
        // game's font.)
        assert!(!f.covers("綾小路"));
    }

    #[test]
    fn a_string_never_leaves_its_box() {
        let mut r = renderer();
        for (text, room) in [("Cannon", 48), ("GunDelS3", 64), ("WWWWWWWW", 64), ("WWWWWWWWWWWWWWWW", 32), ("ElecMan\u{E002}", 64), ("TmhkMan\u{E003}", 64)] {
            let w = r.fitted_width(text, Role::Cell, room);
            assert!(w <= room as f32 + 0.01, "{text:?} is {w} in {room}");
            assert!(w > 0.0);
        }
        // A name with room to spare keeps its width: it isn't stretched.
        let natural = r.fitted_width("Cannon", Role::Cell, 1000);
        assert_eq!(r.fitted_width("Cannon", Role::Cell, natural as i32 + 20), natural);
    }

    /// A white item over a black frame at 2x, `depth` the frame's depth
    /// keys: which output columns of the box's rows it lit.
    fn lit_columns(item: TextItem, depth: Vec<u32>) -> Vec<bool> {
        let frame = crate::render::Frame { pixels: vec![0; WIDTH * HEIGHT], depth, text: vec![item] };
        let (w, h) = (WIDTH * 2, HEIGHT * 2);
        let mut out = vec![0u32; w * h];
        crate::present::present(&frame, Some(&mut renderer()), &mut out, w, h);
        (0..w).map(|x| (0..32).any(|y| out[y * w + x] != 0)).collect()
    }

    #[test]
    fn text_is_hidden_where_something_in_front_won_and_faded_with_its_layer() {
        use crate::compose::{BACKDROP_DEPTH, Fade, depth_key, sprite_depth};
        let item = TextItem {
            depth: depth_key(1, 4, 0),
            ..TextItem::new("MMMMMMMM", Role::Cell, Rect::new(0, 0, 64, 16), 0x7FFF, None)
        };
        let mut depth = vec![BACKDROP_DEPTH; WIDTH * HEIGHT];
        // A sprite in front over the box's left half; the layer itself
        // over the next quarter.
        for y in 0..16 {
            depth[y * WIDTH..y * WIDTH + 32].fill(sprite_depth(0, 0));
            depth[y * WIDTH + 32..y * WIDTH + 48].fill(depth_key(1, 4, 0));
        }
        let lit = lit_columns(item.clone(), depth.clone());
        assert!(!lit[..64].iter().any(|&l| l), "behind the sprite");
        assert!(lit[64..96].iter().any(|&l| l), "on its own layer");
        assert!(lit[96..128].iter().any(|&l| l), "over what is behind");
        // Its layer faded all the way to black: nothing shows.
        let dark = TextItem { fades: [Fade::Black(16), Fade::None], ..item };
        assert!(!lit_columns(dark, depth).iter().any(|&l| l));
    }

    #[test]
    fn a_squeeze_comes_before_a_smaller_size() {
        // (The bundled font has no width axis: it is squeezed.)
        let mut r = renderer();
        let wide = r.fitted_width("WWWWWWWW", Role::Cell, 1000);
        // Just too wide: squeezed, not made smaller.
        let l = r.layout("WWWWWWWW", Role::Cell, (wide * 0.95) as i32).clone();
        assert_eq!(l.drop, 0.0);
        assert!(l.xscale < 1.0 && l.xscale >= SQUEEZE);
        // Far too wide: squeezed all it may be, and smaller.
        let l = r.layout("WWWWWWWW", Role::Cell, (wide * 0.3) as i32).clone();
        assert!(l.drop > 0.0);
        assert_eq!(l.xscale, SQUEEZE);
    }

    #[test]
    fn kana_and_kanji_lay_out() {
        let mut r = renderer();
        let w = r.fitted_width("ガンデルソル3", Role::Cell, 64);
        assert!(w > 0.0 && w <= 64.0);
        let l = r.layout("熱斗", Role::Dialogue, 192).clone();
        assert_eq!(l.glyphs.len(), 2);
        assert!(l.glyphs.iter().all(|g| g.id != 0));
    }
}
