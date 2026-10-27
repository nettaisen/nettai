//! The original mode's strings where the graphics can't spell the language shown
//! (Chinese, which no GBA ROM has a font for; every game's graphics have both
//! releases' fonts, English's and Japanese's), drawn at their own size
//! into the 240x160 frame in pixel fonts: Fusion Pixel Bold 12px for the
//! 8x16 font's strings (chip names, enemy names, the telop) and Fusion
//! Pixel 12px for the chatbox's (fonts/fusion-bold-pixel,
//! fonts/fusion-pixel; docs/design/text-rendering.md §15).
//!
//! The places that draw text lay the strings out as the enhanced mode does
//! (a [`TextSink`](crate::textlayer::TextSink) that measures with
//! [`PixelText::width`], boxes as wide as the text); [`PixelText::draw`]
//! then draws each item where it is, a pixel of a glyph a pixel of the
//! frame, in the item's face color with the original's shadow right and
//! below, faded as the item is and hidden where something in front of it
//! won the pixel. A string is never narrowed. The hanzi end on the row the
//! original's letters end on (on the bar of an enemy's name, in the
//! telop's band). What the game's own font has of a string's plain
//! characters (digits, Latin letters, ASCII's signs) is the game's glyph, as
//! tall as the original's (the pixel fonts' are half-width, smaller); so is
//! a mark of the game's (the stacked EX, SP, DS...: characters of the
//! Private Use Area, §10.5), never the fonts' own glyphs there, which are
//! other things.

use crate::compose::{WIDTH, apply_fade};
use crate::render::Frame;
use crate::textlayer::{Align, Rect, TextItem};
use crate::vfont::{Cut, Role};
use nettai_assets::{DialogueFont, Hud};
use std::collections::HashMap;
use std::rc::Rc;
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::Format;
use swash::{CacheKey, FontRef, GlyphId};

/// Both fonts' pixels an em: their glyphs lie on a 12-pixel grid (1200
/// units an em, 100 a pixel), so at this size a glyph's pixels are whole.
const SIZE: f32 = 12.0;

/// Where a glyph's shadow falls from each of its pixels: right, below, and
/// both (the 8x16 font's own shadow).
const SHADOW: [(i32, i32); 3] = [(1, 0), (0, 1), (1, 1)];

/// Whether `c` is of the Private Use Area: a mark of the game's, which the
/// pack draws.
fn private(c: char) -> bool {
    ('\u{E000}'..='\u{F8FF}').contains(&c)
}

/// Whether the game's own glyph draws `c` where it has one: a mark of the
/// game's, or a plain character (ASCII's).
fn packs(c: char) -> bool {
    private(c) || c.is_ascii_graphic() || c == ' '
}

/// One of the two fonts.
struct Face {
    data: &'static [u8],
    offset: u32,
    key: CacheKey,
    /// Where its hanzi's ink ends, in pixels above the baseline (below it:
    /// negative).
    bottom: f32,
}

impl Face {
    fn new(data: &'static [u8], scaler: &mut ScaleContext) -> Face {
        let f = FontRef::from_index(data, 0).expect("the bundled pixel font parses");
        let mut s = scaler.builder(f).size(SIZE).build();
        let ink = s.scale_outline(f.charmap().map('国')).map(|o| o.bounds());
        let bottom = ink.map_or(0.0, |b| b.min.y);
        Face { data, offset: f.offset, key: f.key, bottom }
    }

    fn font(&self) -> FontRef<'_> {
        FontRef { data: self.data, offset: self.offset, key: self.key }
    }
}

/// A glyph's pixels: 0 clear, 1 its face, 2 its shadow (the game's glyphs
/// draw their own; the fonts' are face alone).
struct Bitmap {
    /// Its first column, right of the pen.
    left: i32,
    /// Its first row: above the baseline (a font's glyph), or down from the
    /// box's top (the game's, drawn where the original's glyph is).
    top: i32,
    w: usize,
    h: usize,
    /// How far it moves the pen (the game's: its cell, or the chatbox
    /// font's advance).
    advance: i32,
    px: Vec<u8>,
}

/// A character laid out: where its glyph goes, from the string's first pen
/// and its box's top.
struct Piece {
    x: i32,
    y: i32,
    glyph: Option<Rc<Bitmap>>,
    /// A font's glyph, which takes the shadow; the game's has its own.
    shadowed: bool,
}

/// A string laid out.
struct Laid {
    pieces: Vec<Piece>,
    width: i32,
}

/// The game's own glyphs the strings take: its fonts' marks of the game's
/// and plain characters ([`packs`]), copied when the pack changes.
#[derive(Default)]
struct Own {
    pack: usize,
    cells: HashMap<char, Rc<Bitmap>>,
    dialogue: HashMap<char, Rc<Bitmap>>,
}

/// The original mode's pixel fonts in one language's cut, their glyphs
/// drawn as they are first wanted.
pub struct PixelText {
    text: Face,
    cells: Face,
    scaler: ScaleContext,
    glyphs: HashMap<(bool, GlyphId), Option<Rc<Bitmap>>>,
    own: Own,
}

impl PixelText {
    /// The fonts in `cut`'s forms (the language shown's: [`Cut::of`]); none
    /// in the Japanese cut's, English's and Japanese's, which every game's graphics
    /// spells in its own fonts. Fusion Pixel 12px draws the chatbox's
    /// strings, its bold build the 8x16 font's ([`crate::bundled`] holds
    /// both).
    pub fn new(cut: Cut) -> Option<PixelText> {
        let traditional = match cut {
            Cut::Japanese => return None,
            Cut::Simplified => false,
            Cut::Traditional => true,
        };
        let mut scaler = ScaleContext::new();
        let text = Face::new(crate::bundled::fusion_pixel(false, traditional), &mut scaler);
        let cells = Face::new(crate::bundled::fusion_pixel(true, traditional), &mut scaler);
        Some(PixelText { text, cells, scaler, glyphs: HashMap::new(), own: Own::default() })
    }

    /// Draw the marks and plain characters from `hud`'s fonts (the game's graphics
    /// whose frame this is).
    pub fn use_pack(&mut self, hud: &Hud) {
        let pack = hud as *const Hud as usize;
        if self.own.pack == pack {
            return;
        }
        let mark = |name: &String| {
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if packs(c) => Some(c),
                _ => None,
            }
        };
        let mut cells = HashMap::new();
        for (k, name) in hud.font_chars.iter().enumerate() {
            let (Some(c), Some(top), Some(bottom)) = (mark(name), hud.font.get(2 * k), hud.font.get(2 * k + 1)) else { continue };
            // (An 8x16 cell: its top tile, then its bottom one, 8x8 each.)
            let px = top.chunks(8).chain(bottom.chunks(8)).flatten().map(|&v| [0, 1, 2][(v as usize & 15).min(2)]).collect();
            cells.entry(c).or_insert_with(|| Rc::new(Bitmap { left: 0, top: 0, w: 8, h: 16, advance: 8, px }));
        }
        let font = &hud.dialogue_font;
        let (ink, ground) = (font.ink(), font.ground());
        let mut dialogue = HashMap::new();
        for (k, name) in font.chars.iter().enumerate() {
            let (Some(c), Some(pixels), Some(&advance)) = (mark(name), font.glyph(k), font.advances.get(k)) else { continue };
            let w = (advance as usize).clamp(8, DialogueFont::WIDTH);
            let px = pixels
                .chunks(DialogueFont::WIDTH)
                .flat_map(|row| &row[..w])
                .map(|&v| if v == ink { 1 } else if v == 0 || v == ground { 0 } else { 2 })
                .collect();
            dialogue.entry(c).or_insert_with(|| Rc::new(Bitmap { left: 0, top: 0, w, h: DialogueFont::HEIGHT, advance: advance as i32, px }));
        }
        self.own = Own { pack, cells, dialogue };
    }

    /// Whether the fonts draw every character of `text`, and the game's graphics every
    /// mark of the game's in it (spaces always).
    pub fn covers(&self, text: &str) -> bool {
        let has = |f: &Face, c: char| f.font().charmap().map(c) != 0;
        text.chars().all(|c| match private(c) {
            true => self.own.cells.contains_key(&c) || self.own.dialogue.contains_key(&c),
            false => c.is_whitespace() || (has(&self.text, c) && has(&self.cells, c)),
        })
    }

    /// How wide `text` is drawn in `role`'s font, in frame pixels.
    pub fn width(&mut self, text: &str, role: Role) -> i32 {
        self.lay(text, role).width
    }

    /// Where `text`'s ink starts and ends, from its first pen (none without
    /// ink).
    pub fn ink_extent(&mut self, text: &str, role: Role) -> Option<(f32, f32)> {
        let laid = self.lay(text, role);
        let ink = laid.pieces.iter().filter_map(|p| p.glyph.as_ref().map(|g| (p.x + g.left, p.x + g.left + g.w as i32)));
        ink.fold(None, |r, (a, b)| Some(r.map_or((a, b), |(x, y): (i32, i32)| (x.min(a), y.max(b))))).map(|(a, b)| (a as f32, b as f32))
    }

    /// Draw the frame's text items into its pixels, back to front, and
    /// leave it none.
    pub fn draw(&mut self, frame: &mut Frame) {
        let mut items = std::mem::take(&mut frame.text);
        items.sort_by_key(|item| std::cmp::Reverse(item.depth));
        for item in &items {
            self.draw_item(item, frame);
        }
    }

    fn face(&self, role: Role) -> &Face {
        if role == Role::Dialogue { &self.text } else { &self.cells }
    }

    fn lay(&mut self, text: &str, role: Role) -> Laid {
        let cells = role != Role::Dialogue;
        // The baseline, down from the box's top: the hanzi's ink ends on the
        // row the role's capitals end on (the row above its baseline).
        let baseline = role.baseline().ceil() as i32 + self.face(role).bottom.round() as i32;
        let mut pen = 0;
        let mut pieces = Vec::new();
        for c in text.chars() {
            let own = if cells { &self.own.cells } else { &self.own.dialogue };
            if let Some(g) = own.get(&c) {
                pieces.push(Piece { x: pen, y: g.top, glyph: Some(g.clone()), shadowed: false });
                pen += g.advance;
                continue;
            }
            if private(c) {
                continue;
            }
            let font = self.face(role).font();
            let id = font.charmap().map(c);
            let advance = font.glyph_metrics(&[]).scale(SIZE).advance_width(id).round() as i32;
            let glyph = self.glyph(cells, id);
            let y = glyph.as_ref().map_or(0, |g| baseline - g.top);
            pieces.push(Piece { x: pen, y, glyph, shadowed: true });
            pen += advance;
        }
        Laid { pieces, width: pen }
    }

    /// A font's glyph's pixels, drawn the first time it is wanted (none for
    /// a glyph with no ink).
    fn glyph(&mut self, cells: bool, id: GlyphId) -> Option<Rc<Bitmap>> {
        if let Some(g) = self.glyphs.get(&(cells, id)) {
            return g.clone();
        }
        let face = if cells { &self.cells } else { &self.text };
        let mut s = self.scaler.builder(face.font()).size(SIZE).build();
        let image = Render::new(&[Source::Outline]).format(Format::Alpha).render(&mut s, id);
        let g = image.filter(|i| i.placement.width > 0 && i.placement.height > 0).map(|i| {
            let p = i.placement;
            let px = i.data.iter().map(|&a| u8::from(a >= 128)).collect();
            Rc::new(Bitmap { left: p.left, top: p.top, w: p.width as usize, h: p.height as usize, advance: 0, px })
        });
        self.glyphs.insert((cells, id), g.clone());
        g
    }

    fn draw_item(&mut self, item: &TextItem, frame: &mut Frame) {
        let laid = self.lay(&item.text, item.role);
        let r = item.rect;
        let x0 = r.x
            + match item.align {
                Align::Left => 0,
                Align::Center => ((r.w - laid.width) as f32 / 2.0).round() as i32,
                Align::Right => r.w - laid.width,
            };
        let shown: Vec<&Piece> = laid.pieces.iter().take(item.shown.unwrap_or(usize::MAX)).collect();
        // The string's pixels on a canvas round its glyphs, a pixel more
        // right and down for the shadow: 1 face, 2 shadow.
        let placed = shown.iter().filter_map(|p| p.glyph.as_ref().map(|g| (x0 + p.x + g.left, r.y + p.y, g.w as i32, g.h as i32)));
        let Some((cx, cy, cx1, cy1)) = placed.fold(None, |b: Option<(i32, i32, i32, i32)>, (x, y, w, h)| {
            Some(b.map_or((x, y, x + w, y + h), |(a, b, c, d)| (a.min(x), b.min(y), c.max(x + w), d.max(y + h))))
        }) else {
            return;
        };
        let (cw, ch) = ((cx1 - cx + 1) as usize, (cy1 - cy + 1) as usize);
        let mut canvas = vec![0u8; cw * ch];
        let at = |x: i32, y: i32| (y - cy) as usize * cw + (x - cx) as usize;
        for p in &shown {
            let Some(g) = &p.glyph else { continue };
            for (k, &v) in g.px.iter().enumerate() {
                let (x, y) = (x0 + p.x + g.left + (k % g.w) as i32, r.y + p.y + (k / g.w) as i32);
                let c = &mut canvas[at(x, y)];
                if v == 1 || (v == 2 && *c == 0) {
                    *c = v;
                }
            }
        }
        if item.shadow.is_some() {
            for p in shown.iter().filter(|p| p.shadowed) {
                let Some(g) = &p.glyph else { continue };
                for (k, _) in g.px.iter().enumerate().filter(|&(_, &v)| v == 1) {
                    let (x, y) = (x0 + p.x + g.left + (k % g.w) as i32, r.y + p.y + (k / g.w) as i32);
                    for (dx, dy) in SHADOW {
                        let c = &mut canvas[at(x + dx, y + dy)];
                        if *c == 0 {
                            *c = 2;
                        }
                    }
                }
            }
        }
        let clip = item.clip.intersect(Rect::SCREEN);
        let fade = |c: u16| apply_fade(apply_fade(c, item.fades[0]), item.fades[1]);
        let colors = [0, fade(item.face), item.shadow.map_or(0, fade)];
        let mut put = |x: i32, y: i32, v: u8| {
            if v == 0 || (v == 2 && item.shadow.is_none()) || x < clip.x || x >= clip.x + clip.w || y < clip.y || y >= clip.y + clip.h {
                return;
            }
            let i = y as usize * WIDTH + x as usize;
            if frame.depth.get(i).is_some_and(|&d| d < item.depth) {
                return;
            }
            frame.pixels[i] = colors[v as usize];
        };
        match item.vscale {
            // The squash: each of the box's rows shows the canvas's row the
            // original's sprite rows step to about the box's middle
            // (`compose`'s sprites), none past the box.
            Some(n) => {
                for row in 0..r.h {
                    let from = ((n * (row - r.h / 2)) >> 8) + r.h / 2;
                    let sy = r.y + from;
                    if !(0..r.h).contains(&from) || sy < cy || sy >= cy + ch as i32 {
                        continue;
                    }
                    for x in cx..cx + cw as i32 {
                        put(x, r.y + row, canvas[at(x, sy)]);
                    }
                }
            }
            None => {
                for (k, &v) in canvas.iter().enumerate() {
                    put(cx + (k % cw) as i32, cy + (k / cw) as i32, v);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::HEIGHT;

    fn frame(items: Vec<TextItem>) -> Frame {
        Frame { pixels: vec![0x1234; WIDTH * HEIGHT], depth: vec![u32::MAX; WIDTH * HEIGHT], text: items, ..Frame::default() }
    }

    /// A string goes into the frame's own pixels, in its face and shadow
    /// colors, round its box; nothing else changes, and the frame keeps no
    /// item for a window to draw again.
    #[test]
    fn a_string_is_drawn_into_the_frame_in_its_colors() {
        let mut p = PixelText::new(Cut::Simplified).unwrap();
        let (face, shadow) = (0x7FFF, 0x2108);
        let mut f = frame(vec![TextItem::new("加农炮 40", Role::FittedCell, Rect::new(16, 8, 64, 16), face, Some(shadow))]);
        p.draw(&mut f);
        assert!(f.text.is_empty());
        let changed: Vec<usize> = (0..WIDTH * HEIGHT).filter(|&i| f.pixels[i] != 0x1234).collect();
        assert!(changed.len() > 60, "the text is drawn ({} pixels)", changed.len());
        assert!(changed.iter().all(|&i| (16..16 + 64 + 2).contains(&(i % WIDTH)) && (6..8 + 16 + 2).contains(&(i / WIDTH))));
        assert!(changed.iter().all(|&i| f.pixels[i] == face || f.pixels[i] == shadow));
        assert!(f.pixels.contains(&face) && f.pixels.contains(&shadow));
    }

    /// The box is as wide as the text in its role's font, and the chatbox's
    /// font is the narrower: the 8x16 font's strings are in the bold build.
    #[test]
    fn a_box_is_as_wide_as_its_text() {
        let mut p = PixelText::new(Cut::Traditional).unwrap();
        let bold = p.width("加農砲", Role::FittedCell);
        let regular = p.width("加農砲", Role::Dialogue);
        assert!(bold > regular && regular > 0, "{bold} {regular}");
        assert!(p.covers("加農砲Ⓑ") && !p.covers("\u{E002}"), "a mark of the game's is the game's");
        let (i0, i1) = p.ink_extent("加農砲", Role::FittedCell).unwrap();
        assert!(i0 >= 0.0 && i1 <= bold as f32);
    }

    /// English and Japanese have no pixel fonts: every game's graphics spell them in
    /// its own.
    #[test]
    fn only_chinese_has_pixel_fonts() {
        assert!(PixelText::new(Cut::Japanese).is_none());
    }

    /// A mark of the game's (the stacked EX) is the game's own 8x16 glyph,
    /// a cell wide, never the font's glyph of that character.
    #[test]
    fn a_mark_is_the_packs_glyph() {
        use nettai_assets::Tiles;
        let mut font = Tiles { pixels: vec![0; 2 * Tiles::TILE] };
        font.pixels.fill(1);
        let hud = Hud { font, font_chars: vec!["\u{E002}".into()], ..Hud::default() };
        let mut p = PixelText::new(Cut::Simplified).unwrap();
        assert!(!p.covers("冲锋人\u{E002}"));
        p.use_pack(&hud);
        assert!(p.covers("冲锋人\u{E002}"));
        assert_eq!(p.width("冲锋人\u{E002}", Role::FittedCell), p.width("冲锋人", Role::FittedCell) + 8);
    }

    /// Where something in front of the text won a pixel, the text leaves it;
    /// the clip cuts it; the chatbox printing shows only its first
    /// characters.
    #[test]
    fn depth_clip_and_printing_hide_what_they_hide() {
        let mut p = PixelText::new(Cut::Simplified).unwrap();
        let item = TextItem::new("进行攻击", Role::Dialogue, Rect::new(40, 100, 160, 12), 0x0421, None);
        let drawn = |item: TextItem, depth: u32| {
            let mut f = frame(vec![item]);
            f.depth.fill(depth);
            let mut p = PixelText::new(Cut::Simplified).unwrap();
            p.draw(&mut f);
            (0..WIDTH * HEIGHT).filter(|&i| f.pixels[i] != 0x1234).collect::<Vec<usize>>()
        };
        assert!(drawn(TextItem { depth: 5, ..item.clone() }, 4).is_empty(), "behind what won the pixels");
        let whole = drawn(item.clone(), u32::MAX);
        let clipped = drawn(TextItem { clip: Rect::new(40, 100, 20, 12), ..item.clone() }, u32::MAX);
        assert!(!clipped.is_empty() && clipped.iter().all(|&i| (40..60).contains(&(i % WIDTH))) && clipped.len() < whole.len());
        let two = drawn(TextItem { shown: Some(2), ..item.clone() }, u32::MAX);
        let width = p.width("进行", Role::Dialogue);
        assert!(!two.is_empty() && two.iter().all(|&i| ((i % WIDTH) as i32) < 40 + width + 1) && two.len() < whole.len());
    }
}
