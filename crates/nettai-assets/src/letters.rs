//! How the battle's pictures with letters and digits look, for the
//! enhanced graphics to draw their words and numbers in fonts in their
//! place (docs/design/text-rendering.md §12): read off the pictures once,
//! as a pack is made (`nettai_content::letters::read`), and kept in the
//! pack beside them. A frontend only applies what is here: where the
//! letters stand, their colors as palette indices (the palette is the
//! one the picture is drawn in at the time), and the picture's tiles with
//! the letters taken off. A picture with nothing here is drawn as it is.
//!
//! What a language changes (the banners, the waiting label, the gauge's
//! words, the chip window's lettered pictures, the form list's names) is in
//! [`HudWords`] and [`PictureLetters`], swapped with the pictures they are
//! read off (`Bundle::in_language`).

use crate::{Tiles, Versioned};
use serde::{Deserialize, Serialize};

/// How a picture's letters look, as palette indices: the face's color on
/// each of the capitals' rows (top to bottom), the outline's color all
/// round the face (a rim) or right and down of it (a shadow), the
/// capitals' first row and how many rows they take, and the letters'
/// columns with their outline (the first, one past the last).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Letters {
    pub fill: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rim: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<u8>,
    pub top: u8,
    pub cap: u8,
    pub span: [i16; 2],
}

/// How a set of the game's number glyphs (8x16 each) looks: the box its
/// faces fill in their cells (x, y, width, height: all the set's glyphs'
/// together) and their paint, as palette indices.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Digits {
    pub face: [u8; 4],
    pub fill: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rim: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<u8>,
}

/// A letter in its own cell (a chip code): the box its face fills (x, y,
/// width, height in the cell) and its paint.
pub type CellLetter = Digits;

/// A banner's bar as a shape: its first row and one past its last; its
/// first row's ends (the first column, one past the last) and its last
/// row's; the rightmost of its rows' first columns and the leftmost of
/// their ends; how far a row's first column is right of the next's; and
/// the colors (palette indices) of its top line, body, bottom line, left
/// end and right end.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BarShape {
    pub rows: [i16; 2],
    pub first_row: [i16; 2],
    pub last_row: [i16; 2],
    pub left_most: i16,
    pub right_least: i16,
    pub slant: i16,
    pub colors: [u8; 5],
}

/// A banner's letters: its bar (none: the picture has none), its letters
/// (on the bar's colors), and the columns its letters take, whole and, for
/// a numbered banner, either side of its number as one digit and as two
/// (`[before, after]`; none: no letters there).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BannerLetters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<BarShape>,
    pub letters: Letters,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub one_digit: Option<[Option<[i16; 2]>; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub two_digits: Option<[Option<[i16; 2]>; 2]>,
}

/// Words in a block of tiles, and the block with them taken off.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WordLetters {
    pub letters: Letters,
    pub blank: Tiles,
}

/// The custom gauge's frame's word: the cells of its top row that draw it
/// (from the frame's first), their tiles with the letters taken off, and
/// the letters (in the cells' picture, from the first cell).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GaugeWord {
    pub cells: Vec<u8>,
    pub blank: Tiles,
    pub letters: Letters,
}

/// The emotion window's count's digits in its box (16x16): each digit's
/// box, and their paint.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountDigits {
    pub glyphs: Vec<[u8; 4]>,
    pub fill: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rim: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<u8>,
}

/// The emotion window's counts: their box with no digits (2x2 tiles), and
/// each count's digits (none: a count drawn as its picture).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CountLetters {
    pub blank: Tiles,
    pub counts: Vec<Option<CountDigits>>,
}

/// A line of a lettered picture's words: its first row, the row under its
/// letters' baseline, its columns (the first, one past the last), and its
/// color (a palette index).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelpLine {
    pub top: u8,
    pub baseline: u8,
    pub span: [u8; 2],
    pub color: u8,
}

/// A chip window's lettered picture (7x6 tiles): its lines, and its tiles
/// with the letters taken off.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelpLetters {
    pub lines: Vec<HelpLine>,
    pub blank: Tiles,
}

/// A form list name's letters (9x2 tiles), in one of its looks: the
/// columns and rows their face takes (the first, the last), the face's
/// color on each of those rows, and the outline's.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormNameLetters {
    pub columns: [u8; 2],
    pub rows: [u8; 2],
    pub fill: Vec<u8>,
    pub rim: u8,
}

/// A window label's letters: their look (`WindowLabel::letters`' colors
/// alone), the runs of columns they take (each a letter: the first, one
/// past the last) and the rows of the label's tiles their ink takes (the
/// first, the last).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelLetters {
    pub letters: Letters,
    pub groups: Vec<[u8; 2]>,
    pub rows: [u8; 2],
}

/// Glyphs of letters in cells (the chip codes): each glyph's letter (none:
/// a glyph with none), and the glyphs with their letters taken off, laid as
/// the glyphs are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlyphLetters {
    pub glyphs: Vec<Option<CellLetter>>,
    pub blank: Tiles,
}

/// The HUD's letters and digits (`Hud::letters`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudLetters {
    /// The HP box's digits (glyphs 0 to 9 of the HUD layer's tiles, on its
    /// blank glyph's colors).
    pub hp_digits: Option<Digits>,
    /// The damage digits, the '+' and the "x2" of the HUD layer's tiles
    /// (`Hud::DAMAGE_DIGIT`, `Hud::PLUS_GLYPH`, `Hud::TIMES_GLYPH`).
    pub damage_digits: Option<Digits>,
    pub plus: Option<Digits>,
    pub times: Option<Digits>,
    /// The opponents' HP digits, by color.
    pub enemy_digits: [Option<Digits>; 3],
    /// The banners' digits.
    pub banner_digits: Option<Digits>,
    /// A game's own telop's digits, '+' and "x2" (`HudLayout::telop`).
    pub telop: Option<[Digits; 3]>,
    /// "PAUSE".
    pub pause: Option<Letters>,
    /// The emotion window's counts.
    pub counts: CountLetters,
    /// The pack's own language's words.
    pub words: HudWords,
}

/// The HUD's letters in a language (`HudLetters::words`,
/// `HudLettering::letters`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudWords {
    /// By banner, as `Hud::banners` (none: no letters, or none of its own).
    pub banners: Vec<Option<BannerLetters>>,
    /// "Cstmzing...".
    pub waiting: Option<Letters>,
    /// The full gauge's "L or R", in its two frames.
    pub l_or_r: [Option<WordLetters>; 2],
    /// The gauge frame's word (CUSTOM).
    pub gauge: Option<GaugeWord>,
}

/// The custom screen's letters and digits (`CustomScreen::letters`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomLetters {
    /// The chip codes, the chip window's (`CustomScreen::codes`) and the
    /// slots' (`CustomScreen::slot_codes`).
    pub codes: GlyphLetters,
    pub slot_codes: GlyphLetters,
    /// The chip window's damage digits.
    pub digits: Option<Digits>,
    /// FINAL TURN (`CustomScreen::turn_limit`).
    pub turn_limit: Option<WordLetters>,
    /// The window's labels, as `CustomScreen::labels`, and the window's
    /// tiles with their letters taken off (as `CustomScreen::window_tiles`).
    pub labels: Vec<Option<LabelLetters>>,
    pub window_blank: Tiles,
    /// The pack's own language's lettered pictures.
    pub pictures: PictureLetters,
}

/// The custom screen's lettered pictures in a language
/// (`CustomLetters::pictures`, `CustomLettering::letters`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PictureLetters {
    /// The chip window's pictures for OK (as `SlotPictures`).
    pub ok: Option<HelpLetters>,
    pub ok_picked: Option<HelpLetters>,
    pub other: Option<HelpLetters>,
    /// The buttons' pictures, by name (as `CustomScreen::buttons`).
    pub buttons: Vec<(String, Option<HelpLetters>)>,
    /// By game version (as `CustomScreen::versioned`).
    pub versioned: Versioned<VersionLetters>,
}

impl PictureLetters {
    /// The letters of the picture of the button named `name`.
    pub fn button(&self, name: &str) -> Option<&HelpLetters> {
        self.buttons.iter().find(|(n, _)| n == name).and_then(|(_, b)| b.as_ref())
    }
}

/// A game version's own lettered pictures (as `VersionPictures`): its
/// buttons' pictures, its form list names (by look, as
/// `VersionPictures::form_names`), and those names with their letters taken
/// off.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VersionLetters {
    pub buttons: Vec<(String, Option<HelpLetters>)>,
    pub form_names: Vec<Option<FormNameLetters>>,
    pub form_names_blank: Tiles,
}

impl VersionLetters {
    /// The letters of the picture of the version's own button `name`.
    pub fn button(&self, name: &str) -> Option<&HelpLetters> {
        self.buttons.iter().find(|(n, _)| n == name).and_then(|(_, b)| b.as_ref())
    }
}
