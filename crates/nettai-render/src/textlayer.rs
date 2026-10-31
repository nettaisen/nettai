//! The text layer: the strings a frame shows that come from content or
//! vary (chip names, the telop, the chatbox's descriptions and messages,
//! the HUD's lines, the enemy names), drawn with a vector font at the
//! output's resolution over the scaled frame (docs/design/text-rendering.md,
//! "As built").
//!
//! In the original mode ([`GraphicsMode::Original`]) nothing changes: every
//! string is drawn into the 240x160 frame in the game's fonts, as the
//! comparison with the original needs. (With graphics that have no lettering
//! in the language shown, whose fonts are another language's, the original
//! mode lays the strings out here and draws them into the frame in pixel
//! fonts, at the frame's resolution: `pixeltext`, `Renderer::render`.) In
//! the enhanced mode
//! ([`GraphicsMode::Enhanced`]) the places that draw text ask a
//! [`TextSink`] first: for a string the font can draw they leave the glyphs
//! out of the frame and hand the sink a [`TextItem`] in the original's box
//! instead; the renderer works out each item's depth and fades, and the
//! window or the PNG writer draws the items after scaling (`present`,
//! `vfont`). Chip codes, the Crosses' names, buttons and logos
//! stay pixel art; the game-style lettering of the banners, the HUD's and
//! the custom window's labels and the chip window's help pictures is drawn
//! here too, in the lettering font ([`Role::Lettering`], `lettering`), and
//! the numbers in the numbers' font (`numbers`); and the frames drawn as
//! shapes in their pictures' place (`shapes`).

use crate::compose::{Anchor, Fade, HEIGHT, Size, WIDTH};
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
    /// A drawing of the battle's frames in place of its picture (the
    /// game's shapes: `crate::shapes`), filled at the output's resolution
    /// in its box: a banner's bar, a box, a window's frame.
    pub drawn: Option<Drawn>,
    /// How far past its box the item truly is, 16.16 frame pixels right and
    /// down: a sprite's true place between the frame's pixels
    /// (`objects::Fine`, the enhanced mode's motion).
    pub fine: (i32, i32),
    /// A number drawn in the numbers' font in place of the words, each
    /// character fitted to its game glyph's box (`numbers`).
    pub fitted: Option<crate::numbers::Fitted>,
    /// A lettering's slant, its rows leaning right going up: hundredths of
    /// a pixel across for each pixel up from its box's bottom (0: upright).
    pub slant: i16,
    /// Where it keeps to in a frame larger than the original's (the
    /// sink's, as it is pushed: `TextSink::anchor`).
    pub anchor: Anchor,
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
            drawn: None,
            fine: (0, 0),
            fitted: None,
            slant: 0,
            anchor: Anchor::CENTER,
        }
    }

    /// A number in the numbers' font (`numbers`), its characters' boxes
    /// from `rect`'s top left.
    pub fn fitted(number: crate::numbers::Fitted, rect: Rect) -> TextItem {
        let text: String = number.glyphs.iter().map(|g| g.0).collect();
        let face = number.fill.first().copied().unwrap_or(0);
        TextItem { fitted: Some(number), ..TextItem::new(text, Role::Cell, rect, face, None) }
    }

    /// The item laid out on the original's screen put in a frame of `size`
    /// where what keeps to its anchor goes: its box and its clip moved
    /// there, the whole screen's clip the whole frame's.
    pub fn placed(self, size: Size) -> TextItem {
        let (ox, oy) = self.anchor.origin(size);
        let clip = match self.clip == Rect::SCREEN {
            true => Rect::new(0, 0, size.width as i32, size.height as i32),
            false => self.clip.offset(ox, oy),
        };
        TextItem { rect: self.rect.offset(ox, oy), clip, ..self }
    }
}

/// How the game-style lettering is painted, from the picture it stands in
/// for (`nettai_assets::letters::Letters`): the face's colors down the capitals, and the
/// rim around them.
#[derive(Clone, Debug, PartialEq)]
pub struct Ink {
    /// The face's color on each of the capitals' rows, top to bottom (BGR555,
    /// before the fades), as the picture has them: drawn blended between
    /// the rows' middles, so the bands run smoothly at the output's
    /// resolution. One color: a flat face.
    pub fill: Vec<u16>,
    /// The rim's color (BGR555): an outline around the face, as the
    /// picture's is. None: no rim.
    pub rim: Option<u16>,
    /// The rim's width in frame pixels (the pictures' lettering's: 1).
    pub rim_width: f32,
    /// Each of the picture's rows its own color, the bands as sharp as the
    /// picture's rather than blended (the custom gauge's words: the user,
    /// "remove the gradient on the custom bar in enhanced mode").
    pub stepped: bool,
}

/// A drawing in place of a picture ([`TextItem::drawn`]): its pieces,
/// drawn one over another in order; the 8x8 cells of its box it may draw in
/// (none: all of them); the frame pixels of its box it keeps off (a
/// picture's over it there: its width and a flag a pixel, row by row; none:
/// none); and a hash of the pieces, to know a drawing painted before.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    pub pieces: Vec<Piece>,
    pub cells: Option<Vec<[u8; 2]>>,
    pub off: Option<(usize, Vec<bool>)>,
    pub hash: u64,
}

/// A drawing in a [`Drawn`]: the drawing (its words apart), the color of
/// each of its palette entries (`Drawing::colors`; BGR555, before the
/// fades), flipped within its box (horizontally, vertically), its top left
/// at `at` (frame pixels from the item's box's), cut to its columns
/// `from`..`to` (its own frame pixels, after the flip; none: all of it).
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub drawing: nettai_assets::shapes::Drawing,
    pub colors: Vec<u16>,
    pub flip: (bool, bool),
    pub at: (f32, f32),
    pub cut: Option<(f32, f32)>,
}

/// What a text item is drawn among, for its depth and its fades.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plane {
    /// The HUD layer (BG3), which the custom screen's shake moves.
    Hud,
    /// The other console's HUD layer on a screen of both (`crate::seat`:
    /// the right player's pieces), which its own console's shake moves.
    OtherHud,
    /// BG0, in front of everything (the enemy names).
    Bg0,
    /// The other console's BG0 on a screen of both (its custom screen's
    /// enemy names).
    OtherBg0,
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
    /// The original mode's pixel fonts, where its graphics can't spell the
    /// language shown: what takes the strings and measures them
    /// (`pixeltext`).
    pixels: Option<&'f std::cell::RefCell<crate::pixeltext::PixelText>>,
    items: Vec<(Plane, TextItem)>,
    next_tag: u32,
    /// The display text the frame shows for content: the player's
    /// language's strings, else the content's own.
    pub strings: crate::strings::DisplayText<'f>,
    /// The game-style lettering goes to the text layer too (the enhanced
    /// mode's: [`TextSink::takes_lettering`]).
    lettering: bool,
    /// Where the items pushed keep to in a frame larger than the original's
    /// (`TextItem::anchor`).
    pub anchor: Anchor,
}

impl<'f> TextSink<'f> {
    pub fn new(mode: GraphicsMode, font: Option<&'f VectorFont>) -> TextSink<'f> {
        let lettering = mode == GraphicsMode::Enhanced && font.is_some();
        TextSink { mode, font, measure: None, pixels: None, items: Vec::new(), next_tag: 0, strings: Default::default(), lettering, anchor: Anchor::CENTER }
    }

    /// With the game-style lettering left to the pictures however the
    /// strings are drawn (`on` false: the original mode's frame, whose
    /// strings graphics without the language's lettering have drawn by the
    /// vector font: `Renderer::render`).
    pub fn with_lettering(self, on: bool) -> TextSink<'f> {
        TextSink { lettering: on && self.lettering, ..self }
    }

    /// Whether a picture's lettering that says `text` goes to the text
    /// layer: in the enhanced mode, when the lettering font draws every
    /// character of it ([`crate::vfont::lettering_font_of`] the text font's
    /// cut); else the picture is drawn as the original draws it.
    pub fn takes_lettering(&self, text: &str) -> bool {
        self.lettering && !text.is_empty() && self.lettering_font().covers(text)
    }

    /// The lettering's font: the bundled font's cut of the text font's.
    fn lettering_font(&self) -> &'static crate::vfont::VectorFont {
        crate::vfont::lettering_font_of(self.font.map(|f| f.cut()).unwrap_or_default())
    }

    /// Whether `text` is drawn as lettering in `font` (the enhanced mode's,
    /// where the font has every character).
    pub fn takes_lettering_in(&self, text: &str, font: crate::vfont::LetteringFont) -> bool {
        use crate::vfont::LetteringFont;
        match font {
            LetteringFont::Display => self.takes_lettering(text),
            LetteringFont::Text => self.takes(text, Role::Cell),
            LetteringFont::Numbers => self.lettering && !text.is_empty() && crate::numbers::font().covers(text),
        }
    }

    /// Whether a picture's lettering that says `text` goes to the text
    /// layer in `font` ([`TextSink::takes_lettering_in`]); where the frame
    /// letters pictures but the font lacks some of its characters, a
    /// problem (`what`'s words): the picture stays as it is.
    pub fn lettering_in(&self, text: &str, font: crate::vfont::LetteringFont, what: impl FnOnce() -> String, problems: &mut crate::audit::Problems) -> bool {
        if self.takes_lettering_in(text, font) {
            return true;
        }
        if let Some(missing) = self.lacks_in(text, font) {
            problems.note(format!("{}: the {font:?} font has no {missing:?} of its words {text:?}, so it stays a picture", what()));
        }
        false
    }

    /// The characters of `text` that `font` lacks, where the frame letters
    /// pictures (none: it has them all, or the frame letters none).
    pub fn lacks_in(&self, text: &str, font: crate::vfont::LetteringFont) -> Option<String> {
        use crate::vfont::LetteringFont;
        let covers: &dyn Fn(&str) -> bool = match font {
            _ if !self.lettering => return None,
            LetteringFont::Display => &|c| self.lettering_font().covers(c),
            LetteringFont::Numbers => &|c| crate::numbers::font().covers(c),
            LetteringFont::Text => &|c| self.font.is_some_and(|f| f.covers(c)),
        };
        let missing: String = text.chars().filter(|&c| !covers(&c.to_string())).collect();
        (!missing.is_empty()).then_some(missing)
    }

    /// Whether the game's numbers are drawn in the numbers' font (the
    /// enhanced mode's: `numbers`).
    pub fn draws_numbers(&self) -> bool {
        self.lettering
    }

    /// Whether the frames the game's graphics have drawings of are drawn as shapes (the
    /// enhanced mode's: `shapes`).
    pub fn draws_shapes(&self) -> bool {
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

    /// With the original mode's pixel fonts taking and measuring the
    /// strings (its graphics can't spell the language shown).
    pub fn with_pixels(self, pixels: Option<&'f std::cell::RefCell<crate::pixeltext::PixelText>>) -> TextSink<'f> {
        TextSink { pixels, ..self }
    }

    /// How wide, in frame pixels, the text layer draws `text` in a box
    /// `room` wide (None without a measurer). The pixel fonts never narrow
    /// a string: its own width.
    pub fn fitted_width(&self, text: &str, role: Role, room: i32) -> Option<f32> {
        if let Some(p) = self.pixels {
            return Some(p.borrow_mut().width(text, role) as f32);
        }
        self.measure.map(|m| m.borrow_mut().fitted_width(text, role, room))
    }

    /// Where `text`'s ink starts and ends in a box `room` wide, from its
    /// first glyph's pen (`TextRenderer::ink_extent`; None without a
    /// measurer, or no ink).
    pub fn ink_extent(&self, text: &str, role: Role, room: i32) -> Option<(f32, f32)> {
        if let Some(p) = self.pixels {
            return p.borrow_mut().ink_extent(text, role);
        }
        self.measure.and_then(|m| m.borrow_mut().ink_extent(text, role, room))
    }

    /// The original mode's: takes nothing.
    pub fn original() -> TextSink<'static> {
        TextSink::new(GraphicsMode::Original, None)
    }

    /// Whether `text`, to be drawn in `role`, goes to the text layer: in the
    /// enhanced mode, when the font (or the original mode's pixel font of
    /// the role) has every character of it (else the whole string is drawn
    /// in the game's font, as in the original mode: a string never mixes two
    /// fonts).
    pub fn takes(&self, text: &str, role: Role) -> bool {
        self.mode == GraphicsMode::Enhanced && (self.font.is_some_and(|f| f.covers(text)) || self.pixels.is_some_and(|p| p.borrow().covers(text, role)))
    }

    /// A new tag for the sprite parts an item stands in for.
    pub fn tag(&mut self) -> u32 {
        self.next_tag += 1;
        self.next_tag
    }

    /// Add an item, kept to where the sink's items keep to now.
    pub fn push(&mut self, plane: Plane, item: TextItem) {
        self.items.push((plane, TextItem { anchor: self.anchor, ..item }));
    }

    /// Where the items pushed from now on start ([`TextSink::move_since`]).
    pub fn mark(&self) -> usize {
        self.items.len()
    }

    /// Move the items pushed since `mark` `dx` frame pixels right, their
    /// boxes and their clips: a piece laid out, then put in its place.
    pub fn move_since(&mut self, mark: usize, dx: i32) {
        for (_, item) in self.items.iter_mut().skip(mark) {
            item.rect = item.rect.offset(dx, 0);
            item.clip = item.clip.offset(dx, 0);
        }
    }

    /// The items pushed since `mark`, with their planes, to put elsewhere.
    pub fn since(&mut self, mark: usize) -> impl Iterator<Item = &mut (Plane, TextItem)> {
        self.items.iter_mut().skip(mark)
    }

    /// The items pushed since `mark` each made into what `f` makes of it
    /// (none, itself put elsewhere, or several), on its plane.
    pub fn rework_since(&mut self, mark: usize, mut f: impl FnMut(Plane, TextItem) -> Vec<TextItem>) {
        let tail: Vec<(Plane, TextItem)> = self.items.drain(mark.min(self.items.len())..).collect();
        for (plane, item) in tail {
            self.items.extend(f(plane, item).into_iter().map(|item| (plane, item)));
        }
    }

    pub fn into_items(self) -> Vec<(Plane, TextItem)> {
        self.items
    }
}

/// A frame's text items, a line each: the words, the role, the box, the
/// depth key and how many of the box's frame pixels (within its clip)
/// something in front covers.
pub fn describe(frame: &crate::render::Frame) -> Vec<String> {
    let width = frame.width();
    frame
        .text
        .iter()
        .map(|t| {
            let area = t.rect.intersect(t.clip).intersect(Rect::new(0, 0, width as i32, frame.height() as i32));
            let mut covered = 0;
            for y in area.y..area.y + area.h {
                for x in area.x..area.x + area.w {
                    if frame.depth.get(y as usize * width + x as usize).is_some_and(|&d| d < t.depth) {
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
        assert!(!TextSink::original().takes("Cannon", Role::FittedCell));
        assert!(TextSink::new(GraphicsMode::Enhanced, Some(&font)).takes("Cannon", Role::FittedCell));
        assert!(TextSink::new(GraphicsMode::Enhanced, Some(&font)).takes("ElecMan\u{E002}", Role::FittedCell));
    }
}
