//! The text layer: the strings a frame shows that come from content or
//! vary (chip names, the telop, the chatbox's descriptions and messages,
//! the HUD's lines, the enemy names), drawn with a vector font at the
//! output's resolution over the scaled frame (docs/design/text-rendering.md,
//! "As built").
//!
//! In the original mode ([`GraphicsMode::Original`]) nothing changes: every
//! string is drawn into the 240x160 frame in the game's fonts, as the
//! comparison with the original needs. (With a pack that has no lettering
//! in the language shown, whose fonts are another language's, the original
//! mode draws the strings with the vector font into the frame, at the
//! frame's resolution: `Renderer::render`.) In the enhanced mode
//! ([`GraphicsMode::Enhanced`]) the places that draw text ask a
//! [`TextSink`] first: for a string the font can draw they leave the glyphs
//! out of the frame and hand the sink a [`TextItem`] in the original's box
//! instead; the renderer works out each item's depth and fades, and the
//! window or the PNG writer draws the items after scaling (`present`,
//! `vfont`). Chip codes, the Crosses' names, buttons and logos
//! stay pixel art; the game-style lettering of the banners, the HUD's and
//! the custom window's labels and the chip window's help pictures is drawn
//! here too, in the lettering font ([`Role::Lettering`], `lettering`), and
//! the numbers in the numbers' font (`numbers`).

use crate::compose::{Fade, HEIGHT, WIDTH};
pub use crate::render::GraphicsMode;
use crate::vfont::{Role, VectorFont};

/// A rectangle in frame pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const SCREEN: Rect = Rect { x: 0, y: 0, w: WIDTH as i32, h: HEIGHT as i32 };

    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    /// The part of both (empty when they don't meet).
    pub fn intersect(self, o: Rect) -> Rect {
        let (x0, y0) = (self.x.max(o.x), self.y.max(o.y));
        let (x1, y1) = ((self.x + self.w).min(o.x + o.w), (self.y + self.h).min(o.y + o.h));
        Rect { x: x0, y: y0, w: (x1 - x0).max(0), h: (y1 - y0).max(0) }
    }

    pub fn offset(self, dx: i32, dy: i32) -> Rect {
        Rect { x: self.x + dx, y: self.y + dy, ..self }
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0 || self.h <= 0
    }
}

/// Where a string sits in its box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// A string for the text layer: what to draw, in which role, in which box
/// of the frame, and how the frame's composition treats it.
#[derive(Clone, Debug, PartialEq)]
pub struct TextItem {
    /// The string, as the content writes it: the game's marks are
    /// characters (Ⓐ, the stacked EX U+E002), which the font draws or the
    /// layer does (`vfont`'s marks).
    pub text: String,
    pub role: Role,
    /// The original's box for the string (its cells), in frame pixels: the
    /// text is fitted into its width and sits on the role's baseline in
    /// it.
    pub rect: Rect,
    pub align: Align,
    /// Nothing outside it is drawn (a window sliding in, the chatbox's
    /// sprites), in frame pixels.
    pub clip: Rect,
    /// Drawn where the frame's depth key is at least this
    /// (`compose::depth_key`): hidden where something in front of the
    /// text's layer or sprite won the pixel.
    pub depth: u32,
    /// The face and shadow colors (BGR555) of the palette the original
    /// draws the string with, before the fades.
    pub face: u16,
    pub shadow: Option<u16>,
    /// The fades that reach it: its layer's or the sprites', then the
    /// screen's.
    pub fades: [Fade; 2],
    /// Drawn on the HUD layer in background palette 14 or 15, past the
    /// custom screen's ranged fades (`Layer::past_ranged`).
    pub past_ranged: bool,
    /// The banners' squash and stretch: the original's texture rows step
    /// by `n / 256` a screen row about the box's middle, and the box's rows
    /// cut it.
    pub vscale: Option<i32>,
    /// How many characters show (the chatbox printing): the string is laid
    /// out whole, so what shows doesn't move as it grows.
    pub shown: Option<usize>,
    /// The game-style lettering's paint, where the picture it stands in for
    /// has more than one face color or a rim (`face` is then its first
    /// fill color).
    pub ink: Option<Ink>,
    /// A banner's bar, drawn under the words.
    pub bar: Option<Bar>,
    /// How far past its box the item truly is, 16.16 frame pixels right and
    /// down: a sprite's true place between the frame's pixels
    /// (`objects::Fine`, the enhanced mode's motion).
    pub fine: (i32, i32),
    /// A number drawn in the numbers' font in place of the words, each
    /// character fitted to its game glyph's box (`numbers`).
    pub fitted: Option<crate::numbers::Fitted>,
}

impl TextItem {
    pub fn new(text: impl Into<String>, role: Role, rect: Rect, face: u16, shadow: Option<u16>) -> TextItem {
        TextItem {
            text: text.into(),
            role,
            rect,
            align: Align::Left,
            clip: Rect::SCREEN,
            depth: 0,
            face,
            shadow,
            fades: [Fade::None; 2],
            past_ranged: false,
            vscale: None,
            shown: None,
            ink: None,
            bar: None,
            fine: (0, 0),
            fitted: None,
        }
    }

    /// A number in the numbers' font (`numbers`), its characters' boxes
    /// from `rect`'s top left.
    pub fn fitted(number: crate::numbers::Fitted, rect: Rect) -> TextItem {
        let text: String = number.glyphs.iter().map(|g| g.0).collect();
        let face = number.fill.first().copied().unwrap_or(0);
        TextItem { fitted: Some(number), ..TextItem::new(text, Role::Cell, rect, face, None) }
    }
}

/// How the game-style lettering is painted, from the picture it stands in
/// for (`lettering::Letters`): the face's colors down the capitals, and the
/// rim around them.
#[derive(Clone, Debug, PartialEq)]
pub struct Ink {
    /// The face's color on each of the capitals' rows, top to bottom (BGR555,
    /// before the fades), as the picture has them: drawn blended between
    /// the rows' middles, so the bands run smoothly at the output's
    /// resolution. One color: a flat face.
    pub fill: Vec<u16>,
    /// The rim's color (BGR555): an outline a frame pixel wide around the
    /// face, heavier right and down, as the picture's is. None: no rim.
    pub rim: Option<u16>,
}

/// A banner's bar, drawn as a shape at the output's resolution under the
/// words (`lettering::Bar::measure` reads it off the banner's picture): a
/// band from `top` to `bottom` whose ends slant at the picture's angle, a
/// line of its top color along its top and of its left color along its
/// left end, of its bottom color along its bottom and of its right color
/// along its right end, its body between. In frame pixels, before the
/// item's squash (which takes it with the words).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bar {
    pub top: f32,
    pub bottom: f32,
    /// The left end where it meets the bottom edge, and the farthest right
    /// it goes up (the picture's end stands upright near the top).
    pub left: f32,
    pub left_most: f32,
    /// The right end where it meets the top edge, and the farthest left it
    /// goes down (upright near the bottom).
    pub right: f32,
    pub right_least: f32,
    /// How far right the ends lean a row up.
    pub slant: f32,
    /// The lines' width, in frame pixels (the picture's: one).
    pub line: f32,
    /// Its colors (BGR555, before the fades).
    pub colors: BarColors,
}

/// A bar's colors ([`Bar`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarColors {
    pub top: u16,
    pub body: u16,
    pub bottom: u16,
    pub left: u16,
    pub right: u16,
}

impl Bar {
    /// The bar moved `dx` right and `dy` down.
    pub fn moved(self, dx: f32, dy: f32) -> Bar {
        Bar {
            top: self.top + dy,
            bottom: self.bottom + dy,
            left: self.left + dx,
            left_most: self.left_most + dx,
            right: self.right + dx,
            right_least: self.right_least + dx,
            ..self
        }
    }

    /// The left end's place on row `y` (a frame row, continuous).
    pub fn left_at(&self, y: f32) -> f32 {
        (self.left + self.slant * (self.bottom - y)).min(self.left_most)
    }

    /// The right end's place on row `y`.
    pub fn right_at(&self, y: f32) -> f32 {
        (self.right - self.slant * (y - self.top)).max(self.right_least)
    }

    /// The color at a point of the frame (continuous), if the bar covers it.
    pub fn color_at(&self, x: f32, y: f32) -> Option<u16> {
        if y < self.top || y >= self.bottom {
            return None;
        }
        let (l, r) = (self.left_at(y), self.right_at(y));
        if x < l || x >= r {
            return None;
        }
        let c = &self.colors;
        Some(if x < l + self.line {
            c.left
        } else if y < self.top + self.line {
            c.top
        } else if y >= self.bottom - self.line {
            c.bottom
        } else if x >= r - self.line {
            c.right
        } else {
            c.body
        })
    }
}

/// What a text item is drawn among, for its depth and its fades.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plane {
    /// The HUD layer (BG3), which the custom screen's shake moves.
    Hud,
    /// BG0, in front of everything (the enemy names).
    Bg0,
    /// Sprite parts queued with this tag (`SpriteList::insert_tagged`):
    /// the item is as deep as the frontmost of them.
    Sprite(u32),
}

/// Collects the frame's text items while it is drawn, and gives the
/// display text for content in the player's language (`strings`).
pub struct TextSink<'f> {
    pub mode: GraphicsMode,
    font: Option<&'f VectorFont>,
    measure: Option<&'f std::cell::RefCell<crate::vfont::TextRenderer>>,
    items: Vec<(Plane, TextItem)>,
    next_tag: u32,
    /// The display text the frame shows for content: the player's
    /// language's strings, else the content's own.
    pub strings: crate::strings::DisplayText<'f>,
    /// The game-style lettering goes to the text layer too (the enhanced
    /// mode's: [`TextSink::takes_lettering`]).
    lettering: bool,
}

impl<'f> TextSink<'f> {
    pub fn new(mode: GraphicsMode, font: Option<&'f VectorFont>) -> TextSink<'f> {
        let lettering = mode == GraphicsMode::Enhanced && font.is_some();
        TextSink { mode, font, measure: None, items: Vec::new(), next_tag: 0, strings: Default::default(), lettering }
    }

    /// With the game-style lettering left to the pictures however the
    /// strings are drawn (`on` false: the original mode's frame, whose
    /// strings a pack without the language's lettering has drawn by the
    /// vector font: `Renderer::render`).
    pub fn with_lettering(self, on: bool) -> TextSink<'f> {
        TextSink { lettering: on && self.lettering, ..self }
    }

    /// Whether a picture's lettering that says `text` goes to the text
    /// layer: in the enhanced mode, when the lettering font draws every
    /// character of it ([`crate::vfont::lettering_font`]); else the
    /// picture is drawn as the original draws it.
    pub fn takes_lettering(&self, text: &str) -> bool {
        self.lettering && !text.is_empty() && crate::vfont::lettering_font().covers(text)
    }

    /// Whether the game's numbers are drawn in the numbers' font (the
    /// enhanced mode's: `numbers`).
    pub fn draws_numbers(&self) -> bool {
        self.lettering
    }

    /// With a language's strings table (none: the content's own strings).
    pub fn with_language(self, strings: Option<&'f nettai_content::locale::Strings>) -> TextSink<'f> {
        TextSink { strings: crate::strings::DisplayText::new(strings), ..self }
    }

    /// With the layouts the text layer will use, so `fitted_width` can say
    /// how wide a string it takes is drawn.
    pub fn measuring(self, measure: Option<&'f std::cell::RefCell<crate::vfont::TextRenderer>>) -> TextSink<'f> {
        TextSink { measure, ..self }
    }

    /// How wide, in frame pixels, the text layer draws `text` in a box
    /// `room` wide (None without a measurer).
    pub fn fitted_width(&self, text: &str, role: Role, room: i32) -> Option<f32> {
        self.measure.map(|m| m.borrow_mut().fitted_width(text, role, room))
    }

    /// The original mode's: takes nothing.
    pub fn original() -> TextSink<'static> {
        TextSink::new(GraphicsMode::Original, None)
    }

    /// Whether `text` goes to the text layer: in the enhanced mode, when the
    /// font has every character of it (else the whole string is drawn in
    /// the game's font, as in the original mode: a string never mixes two
    /// fonts).
    pub fn takes(&self, text: &str) -> bool {
        self.mode == GraphicsMode::Enhanced && self.font.is_some_and(|f| f.covers(text))
    }

    /// A new tag for the sprite parts an item stands in for.
    pub fn tag(&mut self) -> u32 {
        self.next_tag += 1;
        self.next_tag
    }

    pub fn push(&mut self, plane: Plane, item: TextItem) {
        self.items.push((plane, item));
    }

    pub fn into_items(self) -> Vec<(Plane, TextItem)> {
        self.items
    }
}

/// A frame's text items, a line each: the words, the role, the box, the
/// depth key and how many of the box's frame pixels (within its clip)
/// something in front covers.
pub fn describe(frame: &crate::render::Frame) -> Vec<String> {
    frame
        .text
        .iter()
        .map(|t| {
            let area = t.rect.intersect(t.clip).intersect(Rect::SCREEN);
            let mut covered = 0;
            for y in area.y..area.y + area.h {
                for x in area.x..area.x + area.w {
                    if frame.depth.get(y as usize * WIDTH + x as usize).is_some_and(|&d| d < t.depth) {
                        covered += 1;
                    }
                }
            }
            let r = t.rect;
            let total = area.w.max(0) * area.h.max(0);
            let mut line = format!("text {:?} {:?} at ({}, {}) {}x{} depth {:#x}", t.text, t.role, r.x, r.y, r.w, r.h, t.depth);
            line += &format!(" covered {covered}/{total}");
            if let Some(n) = t.vscale {
                line += &format!(" vscale {n:#x}");
            }
            if let Some(n) = t.shown {
                line += &format!(" shown {n}");
            }
            if t.fades != [Fade::None; 2] {
                line += &format!(" fades {:?}", t.fades);
            }
            line
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_meet_or_not() {
        let a = Rect::new(0, 0, 10, 10);
        assert_eq!(a.intersect(Rect::new(5, 5, 10, 10)), Rect::new(5, 5, 5, 5));
        assert!(a.intersect(Rect::new(20, 0, 5, 5)).is_empty());
        assert_eq!("font".parse::<GraphicsMode>(), Ok(GraphicsMode::Enhanced));
        assert!("pixel".parse::<GraphicsMode>().is_err());
    }

    #[test]
    fn the_original_mode_takes_nothing() {
        let font = VectorFont::bundled();
        assert!(!TextSink::original().takes("Cannon"));
        assert!(TextSink::new(GraphicsMode::Enhanced, Some(&font)).takes("Cannon"));
        assert!(TextSink::new(GraphicsMode::Enhanced, Some(&font)).takes("ElecMan\u{E002}"));
    }
}
