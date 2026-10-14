//! The text layer's font and its drawing: a TrueType or OpenType outline
//! font, static or variable, laid out in the frame's pixels (shaped with
//! the font's kerning) and rasterized with antialiasing at the output's
//! resolution, over the scaled frame (swash). docs/design/text-rendering.md,
//! "As built".
//!
//! The bundled font is Murecho (fonts/murecho, SIL Open Font License 1.1):
//! a sans with Latin, Greek, Cyrillic, kana and some 2,300 kanji, in one
//! variable file with a weight axis. A role's text has one size (`Role`):
//! a string too wide for the original's box is narrowed to it, never made
//! smaller; a font with a width axis (`--font`) draws it in a narrower cut
//! of itself before narrowing it.

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

/// Murecho, its variable cut (weights 100 to 900; 1.4 MB): the font file,
/// for a host that draws with it otherwise.
pub const BUNDLED: &[u8] = include_bytes!("../fonts/murecho/Murecho-VariableFont_wght.ttf");

/// The narrowest width a font with a width axis condenses a name to
/// before it narrows it (the axis's "condensed").
const NARROWEST: f32 = 75.0;

/// Whether a character is drawn by the font's kana and kanji metrics (kana,
/// kanji, their punctuation, full-width forms): see [`Face`].
fn cjk(c: char) -> bool {
    matches!(c, '\u{3000}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}' | '\u{FF00}'..='\u{FFEF}')
}

/// The full-size katakana whose ink the font's kana box is measured from
/// ([`Face`]): no voiced, small or long forms.
const KANA: &str = "アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン";
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
const STACK_GAP: f32 = 0.1;
const STACK_STRETCH: f32 = 1.55;
const STACK_WEIGHT: f32 = 100.0;
/// The shadow's offset, right and down, in frame pixels (the original's
/// is one pixel).
const SHADOW: f32 = 0.5;
/// Glyphs smaller than this (pixels an em at the output) are hinted.
const HINT_BELOW: u32 = 40;
/// Glyph images kept before the cache starts over.
const CACHE_GLYPHS: usize = 4096;

/// What a string is drawn as: which of the original's fonts it stands in
/// for, which sets its size, its place in its box and its weight. A role's
/// size is fixed: every string of it is drawn at it, and one too wide for
/// its box is narrowed, never made smaller. No glyph is cut at its box's
/// rows: a role whose cells something follows right under them is drawn
/// so that its descenders end in them (docs/design/text-rendering.md
/// §9.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// The 8x16 font's strings with nothing of the original's under their
    /// cells (the telop, the Program Advance's names, the HUD's lines): a
    /// cell 16 rows high whose capitals, digits and kana take rows 4 to
    /// 13; bold, with a shadow. A descender (Murecho's reach 3.2 pixels
    /// under the baseline, the original's 1) is drawn past the cells.
    Cell,
    /// The 8x16 font's strings whose cells end where something else starts
    /// (the chip window's name over the chip's picture, the hand window's
    /// on the screen's last rows, the enemy names on their bar): drawn at
    /// 90% of [`Role::Cell`]'s size, 1.6 pixels higher, so that the deepest
    /// descender and its shadow end in the cells' 16 rows at every scale,
    /// as the original's do.
    FittedCell,
    /// The dialogue font's (the chatbox): lines 12 rows high whose
    /// capitals, digits and kana take rows 1 to 10 (the Japanese font's;
    /// the US font's capitals rows 0 to 10).
    Dialogue,
}

#[derive(Clone, Copy, Debug)]
struct Style {
    /// Capital height, in frame pixels; the kana's too: kana and kanji are
    /// drawn at the size that makes the font's kana box ([`Face`]) this
    /// high, centered on the capitals, as the original's kana stand in its
    /// capitals' rows.
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
            // Murecho's deepest descender (g) is 0.323 of its capitals'
            // height under the baseline: 2.9 pixels at 9, so with the
            // shadow (at most 1 pixel, at 1x; 2/3 at 3x) and the output
            // pixels' rounding it ends above row 16 at 1x to 6x (the test
            // `a_fitted_cells_descenders_end_in_its_cells`).
            Role::FittedCell => Style { cap: 9.0, baseline: 12.4, weight: 700.0, margin: 1.0 },
            // (The chatbox's lines are cut where its sprites end, which
            // leaves a third line's descenders room: `chatbox::draw`.)
            Role::Dialogue => Style { cap: 9.0, baseline: 10.5, weight: 300.0, margin: 0.0 },
        }
    }
}

/// Where a font's kana stand, at a weight: the median top and bottom of
/// the ink of its full-size katakana ([`KANA`]), over the em, up from the
/// baseline. A Japanese font draws its kana and kanji in its em box, which
/// sits lower than its capitals and is taller (Murecho's bold kana are 1.1
/// times its capitals' height and reach 0.05 em under the baseline), where
/// the original's fonts draw kana in their capitals' rows; the text layer
/// draws them at the size that makes this box the capitals' height,
/// centered on the capitals. One measure of the font's, so every kana and
/// kanji of a string, and of every string, shares one size and one
/// baseline with its Latin.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Face {
    top: f32,
    bottom: f32,
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
        // (EXE5's: its DS navi chips.)
        '\u{E008}' => Mark::Stacked('D', 'S'),
        _ => return None,
    })
}

/// A stacked mark's letters, side by side (U+E002 is `EX`), for text where
/// it can't be drawn (a terminal's).
pub fn stacked_letters(c: char) -> Option<[char; 2]> {
    match mark(c)? {
        Mark::Stacked(a, b) => Some([a, b]),
        Mark::Button(_) => None,
    }
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

/// A button mark's ring, in frame pixels as `Placed`: its center and outer
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
    /// How much it is narrowed to fit its box (1: not at all).
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
    /// The font's kana box by weight (16ths), once measured (none: the
    /// font has no kana).
    faces: HashMap<i32, Option<Face>>,
    made: Made,
}

/// What a text renderer has made so far that it didn't have: each is the
/// work of the first time a string is drawn in a box of some width, or a
/// glyph at some size. A string drawn again costs neither.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Made {
    /// Strings laid out (shaped and fitted to their box).
    pub layouts: u64,
    /// Glyph images drawn from the font's outlines.
    pub glyphs: u64,
}

impl TextRenderer {
    pub fn new(font: std::sync::Arc<VectorFont>) -> TextRenderer {
        TextRenderer {
            font,
            scaler: ScaleContext::new(),
            shaper: ShapeContext::new(),
            layouts: HashMap::new(),
            glyphs: HashMap::new(),
            faces: HashMap::new(),
            made: Made::default(),
        }
    }

    pub fn font(&self) -> &VectorFont {
        &self.font
    }

    /// What it has made so far.
    pub fn made(&self) -> Made {
        self.made
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
            self.made.layouts += 1;
            self.layouts.insert(key.clone(), l);
        }
        &self.layouts[&key]
    }

    /// Lay `text` out within `room` frame pixels, at the role's size and on
    /// its baseline whatever its length: at the font's normal width if it
    /// fits; else, with a width axis, in narrower cuts down to
    /// [`NARROWEST`]; else narrowed horizontally to the room, its height
    /// and baseline unchanged. Never wider than the room, never smaller.
    fn fit(&mut self, text: &str, role: Role, room: f32) -> Layout {
        let st = role.style();
        let weight = self.font.weight.map(|a| st.weight.clamp(a.min, a.max));
        let mut width = self.font.width.map(|a| a.default.clamp(a.min, a.max));
        let mut l = self.shape(text, st, width, weight);
        if l.width <= room || l.width <= 0.0 {
            return l;
        }
        if let Some(a) = self.font.width {
            let narrowest = NARROWEST.clamp(a.min, a.default);
            for k in 1..=8 {
                width = Some(a.default + (narrowest - a.default) * k as f32 / 8.0);
                l = self.shape(text, st, width, weight);
                if l.width <= room {
                    return l;
                }
            }
        }
        let xscale = room / l.width;
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

    /// The font's kana box at `weight` ([`Face`]), measured once.
    fn face(&mut self, weight: Option<f32>) -> Option<Face> {
        let key = weight.map_or(0, |w| (w * 16.0).round() as i32);
        if let Some(&f) = self.faces.get(&key) {
            return f;
        }
        let font = self.font.font();
        let cmap = font.charmap();
        let upem = font.metrics(&[]).units_per_em as f32;
        let settings: Vec<Setting<f32>> = weight.filter(|_| self.font.weight.is_some()).map(|w| ("wght", w).into()).into_iter().collect();
        let mut scaler = self.scaler.builder(font).size(upem).variations(settings).build();
        let (mut tops, mut bottoms) = (Vec::new(), Vec::new());
        for c in KANA.chars() {
            let id = cmap.map(c);
            if id == 0 {
                continue;
            }
            if let Some(o) = scaler.scale_outline(id) {
                let b = o.bounds();
                if b.max.y > b.min.y {
                    tops.push(b.max.y / upem);
                    bottoms.push(b.min.y / upem);
                }
            }
        }
        let median = |v: &mut Vec<f32>| {
            v.sort_by(f32::total_cmp);
            v[v.len() / 2]
        };
        let f = (tops.len() >= KANA.chars().count() / 2).then(|| Face { top: median(&mut tops), bottom: median(&mut bottoms) });
        self.faces.insert(key, f);
        f
    }

    /// Shape `text` in a role's style: runs of characters with the font's
    /// kerning, Latin and the rest at the size that gives the capitals the
    /// role's height, kana and kanji at the size that gives the font's
    /// kana box that height too, raised or lowered onto the capitals'
    /// middle ([`Face`]); a mark the font has no glyph for as
    /// the layer draws it ([`mark`]): a button's letter, smaller, in a ring
    /// centered on the capitals, or a stacked mark's two letters, each half
    /// the capitals' height less a gap, the first above the second, filling
    /// one cell between the capitals' top and the baseline.
    fn shape(&mut self, text: &str, st: Style, width: Option<f32>, weight: Option<f32>) -> Layout {
        let size = st.cap / self.font.cap;
        // Kana and kanji: their size, and how far up from the baseline
        // their glyphs' origin goes.
        let (kana_size, kana_rise) = match self.face(weight) {
            Some(f) => {
                let k = st.cap / (f.top - f.bottom);
                (k, st.cap / 2.0 - k * (f.top + f.bottom) / 2.0)
            }
            None => (size, 0.0),
        };
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
                    // A run: up to the next mark the layer draws, to where
                    // kana and kanji start or end, or past a comma the next
                    // word follows at once.
                    let kana = cjk(chars[i].1);
                    let j = (i + 1..chars.len())
                        .find(|&k| drawn[k].is_some() || cjk(chars[k].1) != kana || tight_comma(&chars, k - 1))
                        .unwrap_or(chars.len());
                    let (start, end) = (chars[i].0, chars.get(j).map_or(text.len(), |c| c.0));
                    let (size, rise) = if kana { (kana_size, kana_rise) } else { (size, 0.0) };
                    let s = settings(weight);
                    let mut b = self.shaper.builder(font).size(size).variations(s.iter().copied()).build();
                    b.add_str(&text[start..end]);
                    b.shape_with(|c| {
                        let at = start + c.source.start as usize;
                        let unit = chars.partition_point(|&(k, _)| k <= at).saturating_sub(1);
                        for g in c.glyphs {
                            out.glyphs.push(Placed { id: g.id, x: pen + g.x, y: rise + g.y, size, width, weight, stretch: 1.0, unit });
                            pen += g.advance;
                        }
                    });
                    if tight_comma(&chars, j - 1) {
                        pen += letter_glyphs(&mut self.shaper, font, ' ', size, &s).1;
                    }
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
            self.made.glyphs += 1;
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
                Align::Center => ((item.rect.w as f32 - layout.width) / 2.0).round(),
                Align::Right => item.rect.w as f32 - layout.width,
            };
        let baseline = item.rect.y as f32 + st.baseline;
        // The squash: about the box's middle. (The original's stretch cuts
        // its glyphs at the sprite's rows; the text isn't cut.)
        let (k, baseline) = match item.vscale {
            Some(n) if n > 0 => {
                let k = 256.0 / n as f32;
                let mid = item.rect.y as f32 + item.rect.h as f32 / 2.0;
                (k, mid + (baseline - mid) * k)
            }
            _ => (1.0, baseline),
        };
        let clip = item.clip.intersect(Rect::SCREEN);
        if clip.is_empty() {
            return;
        }
        let shown = item.shown.unwrap_or(usize::MAX);
        let color = |c: u16| to_rgb(apply_fade(apply_fade(c, item.fades[0]), item.fades[1]));
        let mut target = Target { depth, place, out, out_w, clip, key: item.depth };
        let offset = (SHADOW * s).round().max(1.0) as i32;
        let passes: Vec<(u32, i32)> = item.shadow.map(|c| (color(c), offset)).into_iter().chain([(color(item.face), 0)]).collect();
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
            // outer edge, narrowed and squashed with the text.
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

/// Whether the character at `k` is a comma the next word follows at once
/// ("Lan,this is no time"): the US scripts write no space there, because
/// the original's fonts give a comma a capital's advance (the dialogue
/// font's 8 pixels) and the words stand apart; a font's comma is narrow, so
/// the text layer puts a space's width after it. (Not between digits:
/// "1,000".)
fn tight_comma(chars: &[(usize, char)], k: usize) -> bool {
    chars.get(k).is_some_and(|c| c.1 == ',') && chars.get(k + 1).is_some_and(|c| c.1.is_alphabetic())
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
        let frame = crate::render::Frame { pixels: vec![0; WIDTH * HEIGHT], depth, text: vec![item], glide: None };
        let (w, h) = (WIDTH * 2, HEIGHT * 2);
        let mut out = vec![0u32; w * h];
        crate::present::present(&frame, Some(&mut renderer()), &mut out, w, h);
        (0..w).map(|x| (0..32).any(|y| out[y * w + x] != 0)).collect()
    }

    /// The first and last output rows an item lights, white with a shadow
    /// over a black frame, at `scale` times the frame.
    fn lit_rows(item: TextItem, scale: usize) -> (usize, usize) {
        let frame = crate::render::Frame { pixels: vec![0; WIDTH * HEIGHT], depth: vec![u32::MAX; WIDTH * HEIGHT], text: vec![item], glide: None };
        let (w, h) = (WIDTH * scale, HEIGHT * scale);
        let mut out = vec![0u32; w * h];
        crate::present::present(&frame, Some(&mut renderer()), &mut out, w, h);
        let rows: Vec<usize> = (0..h).filter(|&y| out[y * w..(y + 1) * w].iter().any(|&p| p != 0)).collect();
        (rows[0], *rows.last().expect("something drawn"))
    }

    /// No glyph is cut at its box's rows: a cell string's descenders and
    /// a chatbox line's are drawn past them, a telop's through its squash
    /// and stretch too.
    #[test]
    fn descenders_are_drawn_past_the_cells() {
        let item = |role: Role, h: i32| TextItem::new("gjpqy", role, Rect::new(8, 8, 64, h), 0x7FFF, Some(0x2108));
        assert!(lit_rows(item(Role::Cell, 16), 2).1 > 2 * 24 + 1, "past the cells' 16 rows");
        assert!(lit_rows(item(Role::Dialogue, 12), 2).1 > 2 * 20, "past the dialogue font's 12 rows");
        // The banner's bounce stretches the telop 1.23 times about its
        // box's middle: its descenders go further still.
        let stretched = TextItem { vscale: Some(0xD0), ..item(Role::Cell, 16) };
        assert!(lit_rows(stretched, 2).1 > lit_rows(item(Role::Cell, 16), 2).1);
    }

    /// A fitted cell string, whatever its letters, stays in its cells at
    /// every scale, its shadow included: its deepest descenders end on the
    /// cells' last row, where the original's do, and its ascenders, kana
    /// and kanji start under their first.
    #[test]
    fn a_fitted_cells_descenders_end_in_its_cells() {
        let at = Rect::new(16, 8, 64, 16);
        for scale in 1..=6 {
            let mut deepest = 0;
            for text in ["gjpqy", "Spreadr1", "BigBomb", "(Ⓐ),;", "攻撃熱斗", "ガンデルソル3", "バッドメディスン", "Éfl"] {
                let (first, last) = lit_rows(TextItem::new(text, Role::FittedCell, at, 0x7FFF, Some(0x2108)), scale);
                assert!(first >= at.y as usize * scale, "{text} at {scale}x: from output row {first}");
                assert!(last < (at.y + at.h) as usize * scale, "{text} at {scale}x: to output row {last}");
                deepest = deepest.max(last);
            }
            // (Not fitted further than it needs: the last row is reached.)
            assert!(deepest + 2 * scale > (at.y + at.h) as usize * scale, "{scale}x: to output row {deepest}");
        }
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

    /// A comma the next word follows at once is drawn with a space after
    /// it, as the original's wide comma stands; one between digits isn't.
    #[test]
    fn a_comma_before_a_word_keeps_the_words_apart() {
        let mut r = renderer();
        for role in [Role::Dialogue, Role::Cell] {
            let w = |r: &mut TextRenderer, s: &str| r.fitted_width(s, role, 10_000);
            assert!((w(&mut r, "Lan,this") - w(&mut r, "Lan, this")).abs() < 0.01, "{role:?}");
            assert!(w(&mut r, "1,000") < w(&mut r, "1, 000") - 0.5);
            let l = r.layout("Lan,this", role, 10_000).clone();
            assert_eq!(l.glyphs.iter().map(|g| g.unit).collect::<Vec<_>>(), (0..8).collect::<Vec<_>>(), "a glyph a character");
        }
    }

    /// A string too wide for its box is narrowed to it, however much: its
    /// glyphs keep the role's size and baseline (the bundled font has no
    /// width axis).
    #[test]
    fn a_string_too_wide_is_narrowed_never_made_smaller() {
        let mut r = renderer();
        let natural = r.layout("WWWWWWWW", Role::Cell, 1000).clone();
        for k in [0.95, 0.6, 0.3] {
            let room = (natural.width * k) as i32;
            let l = r.layout("WWWWWWWW", Role::Cell, room).clone();
            let fits = (room as f32 - Role::Cell.style().margin) / natural.width;
            assert!((l.xscale - fits).abs() < 1e-4, "{k}: {} for {fits}", l.xscale);
            assert_eq!(l.glyphs.len(), natural.glyphs.len());
            for (g, n) in l.glyphs.iter().zip(&natural.glyphs) {
                assert_eq!((g.size, g.y), (n.size, n.y), "{k}");
                assert!((g.x - n.x * l.xscale).abs() < 1e-3);
            }
        }
    }

    /// Kana and kanji share one size and one baseline with the Latin of
    /// their role, in every string: the font's kana box is the capitals'
    /// height, centered on them, whatever the string's length or its box.
    #[test]
    fn kana_stand_on_the_capitals_at_one_size() {
        let mut r = renderer();
        for role in [Role::Cell, Role::FittedCell, Role::Dialogue] {
            let st = role.style();
            let face = r.face(Some(st.weight)).expect("Murecho has kana");
            assert!(face.bottom < 0.0 && face.top > r.font.cap, "{face:?}: lower and taller than the capitals");
            let mut sizes = Vec::new();
            for (text, room) in [("キャノン", 1000), ("バッドメディスン", 64), ("ロックマンEXE", 56), ("攻撃力✕2", 160), ("ガンデルソル3", 56)] {
                let l = r.layout(text, role, room).clone();
                let kana: Vec<&Placed> = l.glyphs.iter().filter(|g| text.chars().nth(g.unit).is_some_and(cjk)).collect();
                assert!(!kana.is_empty() && kana.iter().all(|g| g.id != 0), "{text}");
                for g in &kana {
                    let (top, bottom) = (g.y + g.size * face.top, g.y + g.size * face.bottom);
                    assert!((top - bottom - st.cap).abs() < 1e-3, "{text}: the kana box {bottom}..{top}");
                    assert!(((top + bottom) / 2.0 - st.cap / 2.0).abs() < 1e-3, "{text}: on the capitals' middle");
                    sizes.push(g.size);
                }
                // The Latin of the string: on the baseline, at the
                // capitals' size.
                for g in l.glyphs.iter().filter(|g| !text.chars().nth(g.unit).is_some_and(cjk)) {
                    assert_eq!((g.y, g.size), (0.0, st.cap / r.font.cap), "{text}");
                }
            }
            assert!(sizes.windows(2).all(|w| w[0] == w[1]), "{role:?}: one size");
        }
        let l = r.layout("熱斗", Role::Dialogue, 192).clone();
        assert_eq!(l.glyphs.len(), 2);
        assert!(l.glyphs.iter().all(|g| g.id != 0));
    }
}
