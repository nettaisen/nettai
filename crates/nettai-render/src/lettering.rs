//! The game-style lettering of the battle's pictures, as the enhanced mode
//! draws their words in the lettering font in their place
//! (docs/design/text-rendering.md §12).
//!
//! A picture with words (a banner, "PAUSE", the waiting label, the damage
//! digits) has its letters' look in the pack (`nettai_assets::letters`,
//! read off the picture as the pack is made): where their capitals stand,
//! the colors of their face down the rows, and an outline around it, a rim
//! all round or a shadow right and down, as palette indices; a banner's bar
//! as a shape. Here that look is put in the palette the picture is drawn
//! in. Nothing here is a game's, and no picture is read: a game whose
//! pictures look otherwise has another look in its pack. The words
//! themselves are the content's strings (`strings::DisplayText::lettering`).

use crate::textlayer::{Filled, Ink};
use crate::vfont::Lettering;
use nettai_assets::Palette;
use nettai_assets::letters::{Letters, Shape};

/// The role words in `l`'s look are drawn in: the lettering font with its
/// capitals, at `lettering`'s weight and width.
pub fn role(l: &Letters, lettering: fn(u8, u8) -> Lettering) -> crate::vfont::Role {
    crate::vfont::Role::Lettering(lettering(l.top, l.cap))
}

/// `l`'s paint in `palette`.
pub fn ink(l: &Letters, palette: &Palette) -> Ink {
    Ink { fill: l.fill.iter().map(|&c| palette[c as usize & 15]).collect(), rim: l.rim.map(|c| palette[c as usize & 15]) }
}

/// `l`'s face's first color and its shadow's in `palette` (an item's
/// `face` and `shadow`).
pub fn face(l: &Letters, palette: &Palette) -> (u16, Option<u16>) {
    (palette[l.fill.first().copied().unwrap_or(1) as usize & 15], l.shadow.map(|c| palette[c as usize & 15]))
}

/// The look a picture's letters give the text drawn in their place: its
/// role (the lettering font, the capitals where the picture's are), its
/// paint and its face's and shadow's colors, in a palette.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub role: crate::vfont::Role,
    pub ink: Ink,
    pub face: u16,
    pub shadow: Option<u16>,
    /// The picture's letters' columns, their outline included.
    pub span: (i32, i32),
}

impl Look {
    /// A picture's letters' look in `palette`, drawn at `lettering`'s
    /// weight and width (`Lettering::display`, `Lettering::wide`).
    pub fn of(l: &Letters, palette: &Palette, lettering: fn(u8, u8) -> Lettering) -> Look {
        let (face, shadow) = face(l, palette);
        Look { role: role(l, lettering), ink: ink(l, palette), face, shadow, span: (l.span[0] as i32, l.span[1] as i32) }
    }

    /// `words` in this look in `rect` (its rows the picture's), aligned.
    pub fn item(&self, words: impl Into<String>, rect: crate::textlayer::Rect, align: crate::textlayer::Align) -> crate::textlayer::TextItem {
        let item = crate::textlayer::TextItem::new(words, self.role, rect, self.face, self.shadow);
        crate::textlayer::TextItem { align, ink: Some(self.ink.clone()), ..item }
    }

    /// The words standing where the picture's letters stand, the picture's
    /// top left at (x, y), `h` rows high: centered on its letters.
    pub fn in_place(&self, words: impl Into<String>, (x, y): (i32, i32), h: i32) -> crate::textlayer::TextItem {
        let rect = crate::textlayer::Rect::new(x + self.span.0, y, self.span.1 - self.span.0, h);
        self.item(words, rect, crate::textlayer::Align::Center)
    }
}

/// A picture's shapes (a banner's bar: the pack's) in `palette`, its top
/// left at (`x`, `y`) in frame pixels.
pub fn shapes(shapes: &[Shape], palette: &Palette, (x, y): (f32, f32)) -> Vec<Filled> {
    shapes
        .iter()
        .map(|s| Filled { points: s.points.iter().map(|p| [x + p[0] as f32 / Shape::UNIT, y + p[1] as f32 / Shape::UNIT]).collect(), color: palette[s.color as usize & 15] })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture's shapes in its palette, moved to its place.
    #[test]
    fn shapes_in_their_palette_and_place() {
        let palette: Palette = std::array::from_fn(|i| i as u16 * 0x421);
        let filled = shapes(&[Shape { points: vec![[0, 0], [16, 0], [16, 8]], color: 4 }], &palette, (10.0, 20.0));
        assert_eq!(filled, vec![Filled { points: vec![[10.0, 20.0], [12.0, 20.0], [12.0, 21.0]], color: palette[4] }]);
    }

    /// Letters' look in a palette: the face's colors down the rows, the
    /// rim's, the role's capitals.
    #[test]
    fn a_look_in_its_palette() {
        let l = Letters { fill: vec![8, 8, 7, 1, 1, 8], rim: Some(6), shadow: None, top: 2, cap: 6, span: [8, 16] };
        let palette: Palette = std::array::from_fn(|i| i as u16 * 0x421);
        let look = Look::of(&l, &palette, Lettering::display);
        assert_eq!(look.ink.fill, [8, 8, 7, 1, 1, 8].map(|c| palette[c]).to_vec());
        assert_eq!((look.ink.rim, look.face, look.shadow, look.span), (Some(palette[6]), palette[8], None, (8, 16)));
        assert_eq!(look.role, crate::vfont::Role::Lettering(Lettering::display(2, 6)));
    }
}
