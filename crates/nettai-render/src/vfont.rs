//! The text layer's font and its drawing: a TrueType or OpenType outline
//! font, static or variable, laid out in the frame's pixels (shaped with
//! the font's kerning) and drawn with antialiasing at the output's
//! resolution, over the scaled frame, from each glyph's signed distance
//! field, made once whatever the output's size (swash's outlines, `sdf`).
//! docs/design/text-rendering.md, "As built".
//!
//! The bundled font is Noto Sans CJK (fonts/noto-sans-cjk, SIL Open Font
//! License 1.1), one design for every language with each region's hanzi
//! forms, in three cuts ([`Cut`]: Japanese for English and Japanese,
//! Simplified and Traditional Chinese), each one variable file with a weight
//! axis (the user, 2026-10-25: "you should use one unified font for all
//! languages so it's not inconsistent"). A role's text has one size
//! (`Role`): a string too wide for the original's box is narrowed to it,
//! never made smaller; a font with a width axis (`--font`) draws it in a
//! narrower cut of itself before narrowing it.
//!
//! The game-style lettering (the banners, the HUD's labels, the chip
//! window's help pictures, the damage numbers: [`Role::Lettering`]) is drawn
//! in the same family, bold and narrowed (it was M PLUS 1 Code, which has no
//! Chinese), in the cut of the text's language ([`lettering_font_of`]). Its
//! strings take the original's paint ([`Ink`](crate::textlayer::Ink)): a
//! fill that changes down the capitals, a rim. A drawing of a frame
//! ([`Drawn`](crate::textlayer::Drawn)) is filled with analytic coverage,
//! its shapes one over another.
//! docs/design/text-rendering.md §12.

use crate::compose::{Fade, HEIGHT, WIDTH, apply_fade, to_rgb};
use crate::present::Placement;
use crate::sdf::{Paint, RES, Sdf};
use crate::textlayer::{Align, Rect, TextItem};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use swash::scale::ScaleContext;
use swash::shape::ShapeContext;
use swash::zeno::Vector;
use swash::{CacheKey, FontRef, GlyphId, Setting};

/// A cut of the bundled font, Noto Sans CJK: the forms of the hanzi a
/// language writes (its Latin and kana are the same in each).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Cut {
    /// Noto Sans JP: English and Japanese.
    #[default]
    Japanese,
    /// Noto Sans SC: Simplified Chinese.
    Simplified,
    /// Noto Sans TC: Traditional Chinese (Taiwan's forms).
    Traditional,
}

impl Cut {
    /// The cut a language's text is drawn in, by its code (`zh-Hans`,
    /// `zh-Hant`; any other, English's and Japanese's, the Japanese cut).
    pub fn of(language: &str) -> Cut {
        match language {
            "zh-Hans" => Cut::Simplified,
            "zh-Hant" => Cut::Traditional,
            _ => Cut::Japanese,
        }
    }

    /// The font file (Google Fonts' variable cut, weights 100 to 900: fonts/noto-sans-cjk), for a host that
    /// draws with it otherwise too: the program holds it packed, and unpacks it the first time it is wanted
    /// ([`crate::bundled`]).
    pub fn data(self) -> &'static [u8] {
        crate::bundled::noto_sans_cjk(self)
    }

    /// Its family's name.
    pub fn family(self) -> &'static str {
        match self {
            Cut::Japanese => "Noto Sans JP",
            Cut::Simplified => "Noto Sans SC",
            Cut::Traditional => "Noto Sans TC",
        }
    }
}

/// A cut of the bundled font, parsed once.
pub fn bundled_font(cut: Cut) -> &'static std::sync::Arc<VectorFont> {
    static FONTS: std::sync::LazyLock<[std::sync::Arc<VectorFont>; 3]> = std::sync::LazyLock::new(|| {
        [Cut::Japanese, Cut::Simplified, Cut::Traditional].map(|c| std::sync::Arc::new(VectorFont::bundled_cut(c)))
    });
    &FONTS[cut as usize]
}

/// The game-style lettering's font ([`Role::Lettering`]) where the text is
/// in `cut`: the bundled font's same cut.
pub fn lettering_font_of(cut: Cut) -> &'static std::sync::Arc<VectorFont> {
    bundled_font(cut)
}

/// The game-style lettering's font of English and Japanese text.
pub fn lettering_font() -> &'static std::sync::Arc<VectorFont> {
    lettering_font_of(Cut::Japanese)
}

/// The narrowest width a font with a width axis condenses a name to
/// before it narrows it (the axis's "condensed").
const NARROWEST: f32 = 75.0;

/// Whether a character is drawn by the font's kana and kanji metrics (kana,
/// kanji, their punctuation, full-width forms): see [`Face`].
fn cjk(c: char) -> bool {
    matches!(c, '\u{3000}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}' | '\u{FF00}'..='\u{FFEF}')
}

/// The letters whose ink a role with short descenders measures the font's
/// deepest descender from ([`Shorten`]).
const DESCENDERS: &str = "gjpqy";
/// The full-size katakana whose ink the font's kana box is measured from
/// ([`Face`]): no voiced, small or long forms.
const KANA: &str = "アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン";
/// A button's letter (Ⓐ drawn by the text layer) against the text's size,
/// and its ring's diameter and stroke against the capitals' height.
const BUTTON_LETTER: f32 = 0.68;
pub const BUTTON_RING: f32 = 1.15;
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
/// The most the shadow's offset is once rounded to output pixels (at
/// least one), in frame pixels: one, at 1x.
const SHADOW_MOST: f32 = 1.0;
/// The bytes of glyph fields kept before the cache starts over (a glyph's
/// is 10 to 35 KB).
const CACHE_BYTES: usize = 64 << 20;

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
    /// 13; bold, with a shadow. A descender (the text font's reach 3 pixels
    /// under the baseline, the original's 1) is drawn past the cells.
    Cell,
    /// The 8x16 font's strings whose cells end where something else starts
    /// (the chip window's name over the chip's picture, the hand window's
    /// on the screen's last rows, the enemy names on their bar): as
    /// [`Role::Cell`], at its size on its baseline, but with short
    /// descenders: what a glyph's outline has under the baseline is drawn
    /// shorter, so that the descenders take one row and their shadow the
    /// next, the cells' last, as the original's do.
    FittedCell,
    /// The dialogue font's (the chatbox): lines 12 rows high whose
    /// capitals, digits and kana take rows 1 to 10 (the Japanese font's;
    /// the US font's capitals rows 0 to 10).
    Dialogue,
    /// The game-style lettering of a picture the text layer draws in its
    /// place (a banner's words, "PAUSE", a damage number): in the lettering
    /// font ([`lettering_font`]), its capitals where the picture's are.
    Lettering(Lettering),
}

/// How a picture's lettering is drawn ([`Role::Lettering`]), from what the
/// picture shows (`nettai_assets::letters::Letters`): where its capitals stand, and how
/// heavy and how wide its glyphs are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Lettering {
    /// The capitals' first row in the box, and how many rows they take.
    pub top: u8,
    pub cap: u8,
    /// The weight axis's value.
    pub weight: u16,
    /// How wide the glyphs are drawn, in hundredths of the font's own
    /// width (the banners' tall, narrow capitals: [`Lettering::NARROW`]).
    pub width: u8,
    /// Which bundled font draws it.
    pub font: LetteringFont,
}

/// The font a picture's lettering is drawn in ([`Lettering::font`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LetteringFont {
    /// The lettering font ([`lettering_font_of`]: the bundled font's cut of
    /// the text's language).
    #[default]
    Display,
    /// The text font (the text layer's own: the bundled font's cut of the
    /// language, or `--font`'s).
    Text,
    /// The numbers' font (`numbers::font`: Bungee), whose capitals the HUD's
    /// small labels are drawn in (the window's labels, CUSTOM).
    Numbers,
}

/// Which of the text renderer's fonts draws a glyph: the text font, the
/// lettering font or the numbers' font.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FontId {
    Text,
    Lettering,
    Numbers,
}

impl LetteringFont {
    fn id(self) -> FontId {
        match self {
            LetteringFont::Display => FontId::Lettering,
            LetteringFont::Text => FontId::Text,
            LetteringFont::Numbers => FontId::Numbers,
        }
    }
}

impl Lettering {
    /// The width the banners' and the labels' words are drawn at: the
    /// game's capitals are tall and narrow (the user's audition,
    /// 2026-10-18: "ok do that").
    pub const NARROW: u8 = 78;
    /// The weight of the game's display lettering: bold (M PLUS 1 Code's
    /// heaviest, which the lettering was drawn in until the bundled font
    /// took it).
    pub const BOLD: u16 = 700;

    /// Bold and narrowed, capitals `cap` rows high from row `top`.
    pub fn display(top: u8, cap: u8) -> Lettering {
        Lettering { top, cap, weight: Lettering::BOLD, width: Lettering::NARROW, font: LetteringFont::Display }
    }

    /// Bold at the font's own width: the small labels whose letters are as
    /// wide as they are high (CUSTOM, L or R, the help pictures' titles, the
    /// window's labels).
    pub fn wide(top: u8, cap: u8) -> Lettering {
        Lettering { top, cap, weight: Lettering::BOLD, width: 100, font: LetteringFont::Display }
    }
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
    /// Short descenders: how far under the baseline the deepest of them
    /// reaches, in frame pixels (its shadow takes the row under it). What
    /// a glyph's outline has under the baseline is drawn that much
    /// shorter, measured from the font's deepest descender ([`Shorten`]).
    /// None: the font's descenders, whole.
    descender: Option<f32>,
    /// The rows of the original's box for a string (its cells, a line).
    rows: f32,
    /// How wide its glyphs are drawn against the font's own (the
    /// lettering's narrowing; 1 for the rest).
    narrow: f32,
    /// The font that draws it (the lettering's: [`Role::Lettering`]).
    font: FontId,
}

impl Role {
    fn style(self) -> Style {
        let plain = Style { cap: 10.0, baseline: 14.0, weight: 700.0, margin: 1.0, descender: None, rows: 16.0, narrow: 1.0, font: FontId::Text };
        match self {
            Role::Cell => plain,
            // The original's descenders take row 14, their shadow row 15:
            // A g, some 3 pixels deep at this size, is drawn 1 deep
            // (the test `a_fitted_cells_descenders_end_in_its_cells`).
            Role::FittedCell => Style { descender: Some(1.0), ..plain },
            // (The chatbox's lines are cut where its sprites end, which
            // leaves a third line's descenders room: `chatbox::draw`.)
            Role::Dialogue => Style { cap: 9.0, baseline: 10.5, weight: 300.0, margin: 0.0, rows: 12.0, ..plain },
            Role::Lettering(l) => Style {
                cap: l.cap as f32,
                baseline: (l.top + l.cap) as f32,
                weight: l.weight as f32,
                margin: 0.0,
                descender: None,
                rows: 16.0,
                narrow: l.width.max(1) as f32 / 100.0,
                font: l.font.id(),
            },
        }
    }

    /// Its capitals' height, in frame pixels.
    pub fn cap(self) -> f32 {
        self.style().cap
    }

    /// Its baseline, in frame pixels down from its box's top: its capitals
    /// end on the row above it (where the original mode's pixel fonts end
    /// their hanzi: `pixeltext`).
    pub fn baseline(self) -> f32 {
        self.style().baseline
    }

    /// Whether it is drawn in the lettering font.
    pub fn lettering(self) -> bool {
        matches!(self, Role::Lettering(_))
    }
}

/// Where a font's kana stand, at a weight: the median top and bottom of
/// the ink of its full-size katakana ([`KANA`]), over the em, up from the
/// baseline. A Japanese font draws its kana and kanji in its em box, which
/// sits lower than its capitals and is taller (the text font's bold kana are some 1.1
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
    /// The bundled font's cut it is (none: a font file of the player's).
    cut: Option<Cut>,
}

impl std::fmt::Debug for VectorFont {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("VectorFont").field("name", &self.name).finish()
    }
}

impl VectorFont {
    /// The font the frontend comes with, its Japanese cut (English's and
    /// Japanese's).
    pub fn bundled() -> VectorFont {
        VectorFont::bundled_cut(Cut::Japanese)
    }

    /// The font the frontend comes with, the cut of the language `language`
    /// (its code: `ja`, `zh-Hans`).
    pub fn bundled_for(language: &str) -> VectorFont {
        VectorFont::bundled_cut(Cut::of(language))
    }

    fn bundled_cut(cut: Cut) -> VectorFont {
        let font = VectorFont::from_data(Cow::Borrowed(cut.data()), format!("{} (bundled)", cut.family()));
        VectorFont { cut: Some(cut), ..font.expect("the bundled font parses") }
    }

    /// The bundled font's cut it is, or the one whose lettering goes with it
    /// (a font of the player's: the Japanese).
    pub fn cut(&self) -> Cut {
        self.cut.unwrap_or_default()
    }

    /// Whether it is the bundled font (a cut of it), not a font file of the
    /// player's.
    pub fn is_bundled(&self) -> bool {
        self.cut.is_some()
    }

    /// A TrueType or OpenType font file (its first face).
    pub fn load(path: &Path) -> Result<VectorFont, String> {
        let data = nettai_platform::fs::read(path).map_err(|e| format!("can't read the font {}: {e}", path.display()))?;
        VectorFont::from_data(Cow::Owned(data), path.display().to_string())
    }

    /// A font file compiled in.
    pub fn from_bytes(data: &'static [u8], name: &str) -> Result<VectorFont, String> {
        VectorFont::from_data(Cow::Borrowed(data), name.into())
    }

    /// A font of digits alone (the numbers' fonts: `numbers`), which has no
    /// letters.
    pub fn digits_from_bytes(data: &'static [u8], name: &str) -> Result<VectorFont, String> {
        VectorFont::from_data_having(Cow::Borrowed(data), name.into(), '0', "digits")
    }

    fn from_data(data: Cow<'static, [u8]>, name: String) -> Result<VectorFont, String> {
        VectorFont::from_data_having(data, name, 'A', "Latin letters")
    }

    fn from_data_having(data: Cow<'static, [u8]>, name: String, probe: char, what: &str) -> Result<VectorFont, String> {
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
        if f.charmap().map(probe) == 0 {
            return Err(format!("{name} has no {what}"));
        }
        Ok(VectorFont { data, offset, key, width, weight, cap, name, cut: None })
    }

    pub(crate) fn font(&self) -> FontRef<'_> {
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

    /// Whether `c` is a button the text layer draws as its letter in a
    /// ring ([`BUTTON_RING`] times the capitals' height): the font has no
    /// glyph of its own for it.
    pub fn draws_ring(&self, c: char) -> bool {
        matches!(mark(c), Some(Mark::Button(_))) && self.font().charmap().map(c) == 0
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
    /// The font whose glyph it is.
    font: FontId,
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
    /// Its role's short descenders: what its glyphs have under the
    /// baseline is drawn this many times as high ([`Shorten`]).
    shorten: Option<f32>,
}

/// A glyph field's cache key: what changes its outline's shape (the
/// variable font's axes, in 16ths, and the short descenders). Its size,
/// its narrowing and its squash are the drawing's ([`Sdf::draw`]), so one
/// field draws the glyph at every size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GlyphKey {
    id: GlyphId,
    /// The font whose glyph it is.
    font: FontId,
    width: i32,
    weight: i32,
    shorten: Option<Shorten>,
    /// How far out its field holds distances, in texels (0: the spread an
    /// edge's antialiasing needs, `sdf::SPREAD`; more: an outline's,
    /// [`TextRenderer::paint_layout_grown`]).
    spread: u8,
}

impl GlyphKey {
    /// A laid out glyph's, squashed `k` times, its role's short descenders
    /// `shorten` reaching at most `floor` frame pixels under the text's
    /// baseline.
    fn of(g: &Placed, shorten: Option<f32>, k: f32, floor: f32) -> GlyphKey {
        // The text's baseline and its floor, in the glyph's outline's
        // texels, up from its origin.
        let texels = |frame: f32| (frame / (g.size * k) * RES * 64.0).round() as i32;
        GlyphKey {
            id: g.id,
            font: g.font,
            width: g.width.map_or(0, |w| (w * 16.0).round() as i32),
            weight: g.weight.map_or(0, |w| (w * 16.0).round() as i32),
            shorten: shorten.map(|f| Shorten { factor: (f * 1024.0).round() as i32, baseline: texels(-g.y * k), floor: texels(-g.y * k - floor) }),
            spread: 0,
        }
    }
}

/// Short descenders (a role's `Style::descender`): the part of a glyph's
/// outline under the text's baseline is scaled vertically about it, so a
/// descender ends where the original's does; the rest of the glyph is the
/// font's, whole and at its size. One factor for every glyph of the role,
/// from the font's deepest descender ([`DESCENDERS`]), so its descenders
/// stay alike; a glyph that reaches deeper still (a cedilla, the low line)
/// is shortened just as far as ends it in the box's rows, its shadow
/// included. Done to the outline before its field is made, so the glyph
/// is as sharp as any at every scale. In the field's texels ([`RES`] an
/// em), up from the glyph's origin, in 64ths; the factor in 1024ths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Shorten {
    factor: i32,
    /// The text's baseline.
    baseline: i32,
    /// The lowest the outline may reach: the box's last row's bottom, less
    /// the shadow's offset at its most ([`SHADOW_MOST`]).
    floor: i32,
}

impl Shorten {
    fn apply(self, points: &mut [Vector]) {
        let (baseline, floor) = (self.baseline as f32 / 64.0, self.floor as f32 / 64.0);
        let squeeze = |points: &mut [Vector], k: f32| {
            for p in points.iter_mut().filter(|p| p.y < baseline) {
                p.y = baseline + (p.y - baseline) * k;
            }
        };
        squeeze(points, self.factor as f32 / 1024.0);
        // (The outline's points bound its curves.)
        let lowest = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        if lowest < floor && floor < baseline {
            squeeze(points, (baseline - floor) / (baseline - lowest));
        }
    }
}

/// Lays out and draws text items at the output's resolution, keeping the
/// layouts and glyph fields it made.
pub struct TextRenderer {
    font: std::sync::Arc<VectorFont>,
    /// The game-style lettering's ([`Role::Lettering`]), and the numbers'
    /// (the labels' lettering in it).
    lettering: std::sync::Arc<VectorFont>,
    numbers: std::sync::Arc<VectorFont>,
    scaler: ScaleContext,
    shaper: ShapeContext,
    layouts: HashMap<(String, Role, i32), Layout>,
    /// Strings' ink extents ([`TextRenderer::ink_extent`]), each measured
    /// once: a static lettering's words (a window label's) are measured
    /// the first frame they show and kept.
    extents: HashMap<(String, Role, i32), Option<(f32, f32)>>,
    glyphs: HashMap<GlyphKey, Option<Sdf>>,
    /// The bytes the fields in `glyphs` take.
    glyph_bytes: usize,
    /// Each font's kana box by weight (16ths), once measured (none: the
    /// font has no kana); the lettering font's under `true`.
    faces: HashMap<(FontId, i32), Option<Face>>,
    /// Each font's deepest descender by weight (16ths), in ems under the
    /// baseline.
    descenders: HashMap<(FontId, i32), f32>,
    /// The lettering items painted (their bar, rim, shadow and fill) at an
    /// output's size and place, by all that changes their pixels: a banner
    /// holding still is painted once.
    /// What it painted, unplaced, by what it is (its drawings colored, its
    /// text items painted), the least recently used let go first.
    images: crate::rasters::Images,
    /// The numbers' font's glyphs' fields and their ink's bounds (texels:
    /// left, bottom, right, top), by character (`numbers`).
    /// (By the face, its weight (rounded; 0: a font without the axis) and
    /// the glyph.)
    number_glyphs: HashMap<(crate::numbers::Face, u16, u16), Option<(Sdf, [f32; 4])>>,
    made: Made,
    /// The frames' drawings, rasterized ahead (`rasters`).
    pub rasters: crate::rasters::Rasters,
}

/// What a text renderer has made so far that it didn't have: each is the
/// work of the first time a string is drawn in a box of some width, or a
/// glyph in some weight (at any size). A string drawn again costs neither.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Made {
    /// Strings laid out (shaped and fitted to their box).
    pub layouts: u64,
    /// Glyphs' signed distance fields made from the font's outlines.
    pub glyphs: u64,
    /// Drawings rasterized while a frame was drawn, none having said they
    /// were coming (`rasters`: a pack's are rasterized before the first
    /// picture, and again away from the frames when the scale changes).
    pub rasters: u64,
}

impl TextRenderer {
    pub fn new(font: std::sync::Arc<VectorFont>) -> TextRenderer {
        TextRenderer {
            font: font.clone(),
            lettering: lettering_font_of(font.cut()).clone(),
            numbers: crate::numbers::font().clone(),
            scaler: ScaleContext::new(),
            shaper: ShapeContext::new(),
            layouts: HashMap::new(),
            extents: HashMap::new(),
            glyphs: HashMap::new(),
            glyph_bytes: 0,
            faces: HashMap::new(),
            descenders: HashMap::new(),
            images: Default::default(),
            number_glyphs: HashMap::new(),
            made: Made::default(),
            rasters: Default::default(),
        }
    }

    pub fn font(&self) -> &VectorFont {
        &self.font
    }

    /// A renderer of the same font with nothing made yet: one for another
    /// thread, drawing the same frames the same (a video's frames presented
    /// on several, nettai-video).
    pub fn like(&self) -> TextRenderer {
        TextRenderer::new(self.font.clone())
    }

    /// The font a role's text is drawn in.
    fn font_of(&self, id: FontId) -> &VectorFont {
        match id {
            FontId::Text => &self.font,
            FontId::Lettering => &self.lettering,
            FontId::Numbers => &self.numbers,
        }
    }

    /// What it has made so far.
    pub fn made(&self) -> Made {
        Made { rasters: self.rasters.late, ..self.made }
    }

    /// Make the fields of the glyphs `text` is drawn with in `role` (in a
    /// box of the role's rows, unsquashed), if the font draws it: what the
    /// first picture to show it would make, made ahead (a host's loading,
    /// nettai-frontend's `preload`). One field draws its glyph at every
    /// size, so what is made here serves any window. Its lines are made
    /// one by one; a string the font can't draw whole is drawn in the
    /// game's font, and has none.
    pub fn prepare(&mut self, text: &str, role: Role) {
        let st = role.style();
        if !self.font_of(st.font).covers(text) {
            return;
        }
        let shorten = self.shorten(role);
        let floor = st.rows - st.baseline - SHADOW_MOST;
        for line in text.lines() {
            // (Fitted to no box: narrowing makes no other glyphs.)
            let layout = self.fit(line, role, f32::INFINITY);
            for g in layout.glyphs.iter().filter(|g| g.id != 0) {
                self.glyph(GlyphKey::of(g, shorten, 1.0, floor));
            }
        }
    }

    /// The width a string takes, in frame pixels, once fitted into a box
    /// `room` wide.
    pub fn fitted_width(&mut self, text: &str, role: Role, room: i32) -> f32 {
        self.layout(text, role, room).width
    }

    /// Where a string's ink starts and ends, in frame pixels from where its
    /// first glyph's pen starts, once fitted into a box `room` wide: its
    /// glyphs' outlines' bounds, without the font's side bearings (none:
    /// no ink).
    pub fn ink_extent(&mut self, text: &str, role: Role, room: i32) -> Option<(f32, f32)> {
        let key = (text.to_string(), role, room);
        if let Some(&e) = self.extents.get(&key) {
            return e;
        }
        let e = self.measure_ink(text, role, room);
        self.extents.insert(key, e);
        e
    }

    fn measure_ink(&mut self, text: &str, role: Role, room: i32) -> Option<(f32, f32)> {
        let layout = self.layout(text, role, room).clone();
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for g in &layout.glyphs {
            let vf: &VectorFont = match g.font {
                FontId::Text => &self.font,
                FontId::Lettering => &self.lettering,
                FontId::Numbers => &self.numbers,
            };
            let font = vf.font();
            let mut settings: Vec<Setting<f32>> = Vec::new();
            if let (Some(_), Some(w)) = (vf.width, g.width) {
                settings.push(("wdth", w).into());
            }
            if let (Some(_), Some(w)) = (vf.weight, g.weight) {
                settings.push(("wght", w).into());
            }
            let mut scaler = self.scaler.builder(font).size(RES).variations(settings).build();
            let Some(outline) = scaler.scale_outline(g.id) else { continue };
            let b = outline.bounds();
            if b.max.x <= b.min.x {
                continue;
            }
            let k = g.size * layout.xscale * g.stretch / RES;
            lo = lo.min(g.x + b.min.x * k);
            hi = hi.max(g.x + b.max.x * k);
        }
        (lo < hi).then_some((lo, hi))
    }

    fn layout(&mut self, text: &str, role: Role, room: i32) -> &Layout {
        let key = (text.to_string(), role, room);
        if !self.layouts.contains_key(&key) {
            if self.layouts.len() > 1024 {
                self.layouts.clear();
            }
            let mut l = self.fit(text, role, (room as f32 - role.style().margin).max(1.0));
            l.shorten = self.shorten(role);
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
    /// (The lettering's glyphs are narrowed to its width first: `narrow`.)
    fn fit(&mut self, text: &str, role: Role, room: f32) -> Layout {
        let st = role.style();
        let (weight_axis, width_axis) = (self.font_of(st.font).weight, self.font_of(st.font).width);
        let weight = weight_axis.map(|a| st.weight.clamp(a.min, a.max));
        let mut width = width_axis.map(|a| a.default.clamp(a.min, a.max));
        let mut l = narrowed(self.shape(text, st, width, weight), st.narrow);
        if l.width <= room || l.width <= 0.0 {
            return l;
        }
        if let Some(a) = width_axis {
            let narrowest = NARROWEST.clamp(a.min, a.default);
            for k in 1..=8 {
                width = Some(a.default + (narrowest - a.default) * k as f32 / 8.0);
                l = narrowed(self.shape(text, st, width, weight), st.narrow);
                if l.width <= room {
                    return l;
                }
            }
        }
        let width = l.width;
        let mut l = narrowed(l, room / width);
        l.width = l.width.min(room);
        l
    }

    /// A role's short descenders' factor ([`Shorten`]): its descender's
    /// depth over the font's deepest descender's at the role's size (never
    /// more than 1).
    fn shorten(&mut self, role: Role) -> Option<f32> {
        let st = role.style();
        let depth = st.descender?;
        let vf: &VectorFont = match st.font {
            FontId::Text => &self.font,
            FontId::Lettering => &self.lettering,
            FontId::Numbers => &self.numbers,
        };
        let weight = vf.weight.map(|a| st.weight.clamp(a.min, a.max));
        let key = (st.font, weight.map_or(0, |w| (w * 16.0).round() as i32));
        let deepest = match self.descenders.get(&key) {
            Some(&d) => d,
            None => {
                let font = vf.font();
                let cmap = font.charmap();
                let upem = font.metrics(&[]).units_per_em as f32;
                let settings: Vec<Setting<f32>> = weight.map(|w| ("wght", w).into()).into_iter().collect();
                let mut scaler = self.scaler.builder(font).size(upem).variations(settings).build();
                let d = DESCENDERS
                    .chars()
                    .filter_map(|c| scaler.scale_outline(cmap.map(c)))
                    .map(|o| -o.bounds().min.y / upem)
                    .fold(0.0, f32::max);
                self.descenders.insert(key, d);
                d
            }
        };
        let px = deepest * st.cap / self.font_of(st.font).cap;
        Some(if px > depth { depth / px } else { 1.0 })
    }

    /// A font's kana box at `weight` ([`Face`]), measured once (the
    /// lettering font's, or the text font's).
    fn face(&mut self, id: FontId, weight: Option<f32>) -> Option<Face> {
        let key = (id, weight.map_or(0, |w| (w * 16.0).round() as i32));
        if let Some(&f) = self.faces.get(&key) {
            return f;
        }
        let vf: &VectorFont = match id {
            FontId::Text => &self.font,
            FontId::Lettering => &self.lettering,
            FontId::Numbers => &self.numbers,
        };
        let font = vf.font();
        let cmap = font.charmap();
        let upem = font.metrics(&[]).units_per_em as f32;
        let settings: Vec<Setting<f32>> = weight.filter(|_| vf.weight.is_some()).map(|w| ("wght", w).into()).into_iter().collect();
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
        let lettering = st.font;
        let size = st.cap / self.font_of(lettering).cap;
        // Kana and kanji: their size, and how far up from the baseline
        // their glyphs' origin goes.
        let (kana_size, kana_rise) = match self.face(lettering, weight) {
            Some(f) => {
                let k = st.cap / (f.top - f.bottom);
                (k, st.cap / 2.0 - k * (f.top + f.bottom) / 2.0)
            }
            None => (size, 0.0),
        };
        let vf: &VectorFont = match lettering {
            FontId::Text => &self.font,
            FontId::Lettering => &self.lettering,
            FontId::Numbers => &self.numbers,
        };
        let font = vf.font();
        let cmap = font.charmap();
        let cap = vf.cap;
        let heavier = vf.weight.zip(weight).map(|(a, w)| (w + STACK_WEIGHT).min(a.max));
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
                        out.glyphs.push(Placed { id, x: gx, y: gy, size: small, width, weight, stretch: 1.0, unit: i, font: lettering });
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
                            out.glyphs.push(Placed { id, x: gx, y: y + gy, size: small, width, weight: heavier, stretch: STACK_STRETCH, unit: i, font: lettering });
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
                            out.glyphs.push(Placed { id: g.id, x: pen + g.x, y: rise + g.y, size, width, weight, stretch: 1.0, unit, font: lettering });
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

    /// A glyph's field, made the first time it is wanted.
    fn glyph(&mut self, key: GlyphKey) -> Option<&Sdf> {
        if !self.glyphs.contains_key(&key) {
            if self.glyph_bytes >= CACHE_BYTES {
                self.glyphs.clear();
                self.glyph_bytes = 0;
            }
            let vf: &VectorFont = match key.font {
                FontId::Text => &self.font,
                FontId::Lettering => &self.lettering,
                FontId::Numbers => &self.numbers,
            };
            let font = vf.font();
            let mut settings: Vec<Setting<f32>> = Vec::new();
            if vf.width.is_some() {
                settings.push(("wdth", key.width as f32 / 16.0).into());
            }
            if vf.weight.is_some() {
                settings.push(("wght", key.weight as f32 / 16.0).into());
            }
            let mut scaler = self.scaler.builder(font).size(RES).variations(settings).build();
            let field = scaler.scale_outline(key.id).and_then(|mut outline| {
                if let Some(shorten) = key.shorten {
                    shorten.apply(outline.points_mut());
                }
                Sdf::of_spread(outline.points(), outline.verbs(), key.spread as f32)
            });
            self.glyph_bytes += field.as_ref().map_or(0, Sdf::size);
            self.made.glyphs += 1;
            self.glyphs.insert(key, field);
        }
        self.glyphs[&key].as_ref()
    }

    /// Draw a frame's text items over its scaled picture in `out` (0RGB,
    /// `out_w` pixels a row), each where the frame's depth keys let it.
    pub fn draw(&mut self, items: &[TextItem], depth: &[u32], place: &Placement, out: &mut [u32], out_w: usize) {
        self.draw_over(items, depth, None, place, out, out_w);
    }

    /// [`TextRenderer::draw`], each item hidden too where an overlay in
    /// front of it was drawn (`cover`: the overlays' depth keys on the
    /// output's pixels, `smooth::draw_overlays_covering`).
    pub fn draw_over(&mut self, items: &[TextItem], depth: &[u32], cover: Option<&[u32]>, place: &Placement, out: &mut [u32], out_w: usize) {
        // The frame pixel each output column and row of the picture shows.
        let columns: Vec<i32> = (0..place.width).map(|dx| place.frame_pixel(place.x + dx, place.y).map_or(0, |p| p.0)).collect();
        let rows: Vec<i32> = (0..place.height).map(|dy| place.frame_pixel(place.x, place.y + dy).map_or(0, |p| p.1)).collect();
        let frame = FrameMap { x: place.x, y: place.y, columns: &columns, rows: &rows, cover };
        // Back to front by depth, so one item in front of another covers it as the frame's own pixels do (the custom
        // screen's window drawn as shapes over an HP number under it); items of one depth in the order given.
        let mut order: Vec<&TextItem> = items.iter().collect();
        order.sort_by_key(|item| std::cmp::Reverse(item.depth));
        for item in order {
            self.draw_item(item, depth, place, &frame, out, out_w);
        }
    }

    fn draw_item(&mut self, item: &TextItem, depth: &[u32], place: &Placement, frame: &FrameMap, out: &mut [u32], out_w: usize) {
        let layout = if item.fitted.is_some() { Layout::default() } else { self.layout(&item.text, item.role, item.rect.w).clone() };
        if let Some(d) = &item.drawn {
            self.draw_drawn(item, d, depth, place, frame, out, out_w);
            return;
        }
        if layout.glyphs.is_empty() && item.fitted.is_none() {
            return;
        }
        let s = place.scale;
        let clip = item.clip.intersect(Rect::SCREEN);
        if clip.is_empty() {
            return;
        }
        // The output pixels the clip shows (a pixel more each way: `Target`
        // tests each against the clip's frame pixels).
        let bounds = (
            place.x + (clip.x as f32 * s).floor() as i32 - 1,
            place.y + (clip.y as f32 * s).floor() as i32 - 1,
            place.x + ((clip.x + clip.w) as f32 * s).ceil() as i32 + 1,
            place.y + ((clip.y + clip.h) as f32 * s).ceil() as i32 + 1,
        );
        // Painted once for what it is, unplaced (its box's top left at the
        // output's (0, 0), in its colors before the fades), and laid at
        // whole output pixels where it is, faded there.
        let (fine_x, fine_y) = (item.fine.0 as f32 / 65536.0, item.fine.1 as f32 / 65536.0);
        let ox = (place.x as f32 + s * (item.rect.x as f32 + fine_x)).round() as i32;
        let oy = (place.y as f32 + s * (item.rect.y as f32 + fine_y)).round() as i32;
        let key = crate::rasters::ImageKey::Text(format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|{}",
            item.text, item.role, (item.rect.w, item.rect.h), item.align, item.face, item.shadow, item.vscale, item.shown, item.ink, item.fitted, item.slant, s
        ));
        let image = match self.images.get(&key) {
            Some(i) => i,
            None => {
                let local = TextItem { rect: Rect::new(0, 0, item.rect.w, item.rect.h), fine: (0, 0), clip: Rect::new(0, 0, item.rect.w, item.rect.h), ..item.clone() };
                let lp = Placement { x: 0, y: 0, width: place.width, height: place.height, scale: s };
                let i = std::sync::Arc::new(self.paint_text(&local, &layout, &lp).map(crate::rasters::Image::from).unwrap_or_default());
                self.images.put(key, i.clone());
                i
            }
        };
        let mut target = Target { depth, frame, out, out_w, clip, key: item.depth };
        let fades = item.fades;
        let faded = |rgb: u32| if fades == [Fade::None, Fade::None] { rgb } else { fade_rgb(fade_rgb(rgb, fades[0]), fades[1]) };
        target.blit(&image, (ox, oy), bounds, faded);
    }

    /// A text item painted at the output's scale where `place` puts it,
    /// in its colors as they are before the fades: its numbers (`fitted`),
    /// its lettering (`ink`) or its string (a shadow under it), cropped to
    /// what it covers.
    fn paint_text(&mut self, item: &TextItem, layout: &Layout, place: &Placement) -> Option<Painted> {
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
        let mid = item.rect.y as f32 + item.rect.h as f32 / 2.0;
        let (k, baseline) = match item.vscale {
            Some(n) if n > 0 => {
                let k = 256.0 / n as f32;
                (k, mid + (baseline - mid) * k)
            }
            _ => (1.0, baseline),
        };
        let color = |c: u16| to_rgb(c);
        // (All it may cover: its box, its words past it, and eight frame
        // pixels round them, its box's height more up and down.)
        let (left, right) = (x0.min(0.0) - 8.0, (x0 + layout.width).max(item.rect.w as f32) + 8.0);
        let (top, bottom) = (-(item.rect.h as f32) - 8.0, 2.0 * item.rect.h as f32 + 8.0);
        let bounds = (
            place.x + (left * s).floor() as i32,
            place.y + (top * s).floor() as i32,
            place.x + (right * s).ceil() as i32,
            place.y + (bottom * s).ceil() as i32,
        );
        // Short descenders: how far under the text's baseline its ink may
        // reach (the box's bottom, less the shadow's offset at its most),
        // in frame pixels.
        let floor = (item.rect.y + item.rect.h) as f32 - (item.rect.y as f32 + st.baseline) - SHADOW_MOST;
        let at = Laid { x0, baseline, k, floor, shown: item.shown.unwrap_or(usize::MAX) };
        let offset = (SHADOW * s).round().max(1.0) as i32;
        if let Some(f) = &item.fitted {
            return self.paint_fitted(f, item.rect, (k, mid), (0.0, 0.0), place, bounds, &color);
        }
        if let Some(ink) = &item.ink {
            let mut painted = self.paint_inked(item, ink, layout, st, &at, place, bounds, (k, mid), (0.0, 0.0), &color);
            // (Slanted: each row moved right by the slant for how far up
            // from the box's bottom it is, at the output's resolution.)
            if item.slant != 0 {
                let bottom = place.y as f32 + (item.rect.y + item.rect.h) as f32 * s;
                painted = painted.map(|p| p.sheared(item.slant as f32 / 100.0, bottom));
            }
            return painted;
        }
        let mut painted = Painted::new(bounds);
        let passes: Vec<(u32, i32)> = item.shadow.map(|c| (color(c), offset)).into_iter().chain([(color(item.face), 0)]).collect();
        for (rgb, off) in passes {
            let mut pen = |x: i32, y: i32, a: u8| painted.blend(x, y, rgb, a);
            self.paint_layout(layout, st, &at, place, bounds, off, &mut pen);
        }
        painted.cropped()
    }

    /// A drawing ([`Drawn`](crate::textlayer::Drawn)) over the picture: its
    /// pieces drawn one over another by resvg at the output's resolution in
    /// its box (squashed about its middle as a banner's words are), painted
    /// once for what it is and where, then drawn through its clip, its cells
    /// and the frame's depth keys.
    #[allow(clippy::too_many_arguments)]
    fn draw_drawn(&mut self, item: &TextItem, d: &crate::textlayer::Drawn, depth: &[u32], place: &Placement, frame: &FrameMap, out: &mut [u32], out_w: usize) {
        let s = place.scale;
        let clip = item.clip.intersect(Rect::SCREEN);
        if clip.is_empty() {
            return;
        }
        let bounds = (
            place.x + (clip.x as f32 * s).floor() as i32,
            place.y + (clip.y as f32 * s).floor() as i32,
            place.x + ((clip.x + clip.w) as f32 * s).ceil() as i32,
            place.y + ((clip.y + clip.h) as f32 * s).ceil() as i32,
        );
        let (fine_x, fine_y) = (item.fine.0 as f32 / 65536.0, item.fine.1 as f32 / 65536.0);
        let (x0, y0) = (item.rect.x as f32 + fine_x, item.rect.y as f32 + fine_y);
        let mid = y0 + item.rect.h as f32 / 2.0;
        let k = match item.vscale {
            Some(n) if n > 0 => 256.0 / n as f32,
            _ => 1.0,
        };
        let color = |c: u16| to_rgb(apply_fade(apply_fade(c, item.fades[0]), item.fades[1]));
        // (Only in its cells, where it has them: the frame pixel each output
        // pixel shows, from its box's top left.)
        // (Its cells as a grid of flags, a cell a flag, to look up.)
        let grid_w = (item.rect.w.max(0) as usize).div_ceil(8);
        let grid: Option<Vec<bool>> = d.cells.as_ref().map(|cells| {
            let mut g = vec![false; grid_w * (item.rect.h.max(0) as usize).div_ceil(8)];
            for c in cells {
                if let Some(f) = g.get_mut(c[1] as usize * grid_w + c[0] as usize).filter(|_| (c[0] as usize) < grid_w) {
                    *f = true;
                }
            }
            g
        });
        // (Whether the frame pixel (`fx`, `fy`) an output pixel shows is in
        // them; everywhere, for a drawing with neither.)
        let everywhere = d.cells.is_none() && d.off.is_none();
        let in_cells = |fx: i32, fy: i32| -> bool {
            let (dx, dy) = (fx - item.rect.x, fy - item.rect.y);
            if let Some((w, off)) = &d.off
                && dx >= 0
                && dy >= 0
                && (dx as usize) < *w
                && off.get(dy as usize * w + dx as usize) == Some(&true)
            {
                return false;
            }
            let Some(grid) = &grid else { return true };
            let (cx, cy) = (dx.div_euclid(8), dy.div_euclid(8));
            cx >= 0 && cy >= 0 && (cx as usize) < grid_w && grid.get(cy as usize * grid_w + cx as usize) == Some(&true)
        };
        let mut target = Target { depth, frame, out, out_w, clip, key: item.depth };
        // Each piece from its raster, colored now, at whole output pixels.
        for piece in &d.pieces {
            let colors: Vec<u32> = piece.colors.iter().map(|&c| color(c)).collect();
            let Some(t) = self.rasters.tinted(&mut self.images, &piece.drawing, &colors, piece.flip, s, k) else { continue };
            let left = place.x as f32 + s * (x0 + piece.at.0);
            let top = place.y as f32 + s * (mid + (y0 + piece.at.1 - mid) * k);
            let (ox, oy) = (left.round() as i32, top.round() as i32);
            // (Cut to its columns `from`..`to`: each output column's share
            // of them.)
            let cut = |lx: usize| -> f32 {
                match piece.cut {
                    None => 1.0,
                    Some((from, to)) => {
                        let (a, b) = (from * s, to * s);
                        ((lx as f32 + 1.0).min(b) - (lx as f32).max(a)).clamp(0.0, 1.0)
                    }
                }
            };
            // (Its columns within the bounds; on each row the clip shows,
            // the frame row it is, once.)
            let columns = (bounds.0 - ox).clamp(0, t.w as i32) as usize..(bounds.2 - ox).clamp(0, t.w as i32) as usize;
            for ly in 0..t.h {
                let y = oy + ly as i32;
                if y < bounds.1 || y >= bounds.3 {
                    continue;
                }
                let Some(fy) = target.row(y) else { continue };
                for lx in columns.clone() {
                    let (rgb, a) = t.px[ly * t.w + lx];
                    if a == 0 {
                        continue;
                    }
                    let x = ox + lx as i32;
                    let a = match piece.cut {
                        None => a,
                        Some(_) => (a as f32 * cut(lx)).round() as u8,
                    };
                    if a == 0 {
                        continue;
                    }
                    let Some(fx) = target.column(x) else { continue };
                    if everywhere || in_cells(fx, fy) {
                        target.blend_at(x, y, (fx, fy), rgb, a);
                    }
                }
            }
        }
    }

    /// A lettering item painted ([`Painted`]): its glyphs' coverage once (a [`Mask`]), the rim
    /// around it, the shadow and the face's fill down the capitals drawn from it.
    #[allow(clippy::too_many_arguments)]
    fn paint_inked(
        &mut self,
        item: &TextItem,
        ink: &crate::textlayer::Ink,
        layout: &Layout,
        st: Style,
        at: &Laid,
        place: &Placement,
        bounds: (i32, i32, i32, i32),
        (k, mid): (f32, f32),
        (_, fine_y): (f32, f32),
        color: &impl Fn(u16) -> u32,
    ) -> Option<Painted> {
        let s = place.scale;
        let (x0, baseline) = (at.x0, at.baseline);
        let offset = (SHADOW * s).round().max(1.0) as i32;
        {
            let cap_top = item.rect.y as f32 + st.baseline - st.cap + fine_y;
            let unsquash = |fy: f32| mid + fine_y + (fy - mid - fine_y) / k;
            // (Its box and a margin, and the cap's height more up and down
            // for what stands past the capitals: accents, kana, descenders.)
            let margin = 2.0 + 2.0 * ink.rim_width;
            let (left, right) = (x0.min(item.rect.x as f32) - margin, (x0 + layout.width).max((item.rect.x + item.rect.w) as f32) + margin);
            let (top, bottom) = (baseline - (st.cap * 1.6 + margin) * k, baseline + (st.cap * 0.6 + margin) * k);
            let area = (
                ((place.x as f32 + left * s).floor() as i32).max(bounds.0),
                ((place.y as f32 + top * s).floor() as i32).max(bounds.1),
                ((place.x as f32 + right * s).ceil() as i32).min(bounds.2),
                ((place.y as f32 + bottom * s).ceil() as i32).min(bounds.3),
            );
            if area.0 >= area.2 || area.1 >= area.3 {
                return None;
            }
            let mut out = Painted::new(area);
            let mut mask = Mask::new(area);
            self.paint_layout(layout, st, at, place, area, 0, &mut mask);
            // (A frame pixel all round, even, as the pictures' are: each
            // glyph's own shape grown that far, `paint_layout_grown`.)
            let rim = match ink.rim {
                Some(c) => {
                    let mut grown = Mask::new(area);
                    self.paint_layout_grown(layout, st, at, place, area, ink.rim_width * s, &mut grown);
                    // (An outline wider than a pixel fills what the letters
                    // close in, as the pictures' do: ロ's inside.)
                    if ink.rim_width > RIM {
                        grown.fill_enclosed();
                    }
                    Some((color(c), grown))
                }
                None => None,
            };
            let fill: Vec<u32> = if ink.fill.is_empty() { vec![color(item.face)] } else { ink.fill.iter().map(|&c| color(c)).collect() };
            let shadow = item.shadow.map(color);
            for y in area.1..area.3 {
                // The face's color on this output row: the picture's row
                // its middle falls on, blended between rows' middles (or
                // that row's alone, `Ink::stepped`).
                let row = unsquash((y as f32 + 0.5 - place.y as f32) / s) - cap_top - 0.5;
                let face = if ink.stepped { row_color(&fill, row) } else { fill_color(&fill, row) };
                for x in area.0..area.2 {
                    if let Some((rgb, rim)) = &rim {
                        let a = rim.at(x, y);
                        if a > 0 {
                            out.blend(x, y, *rgb, a);
                        }
                    }
                    if let Some(rgb) = shadow {
                        let a = mask.at(x - offset, y - offset);
                        if a > 0 {
                            out.blend(x, y, rgb, a);
                        }
                    }
                    let a = mask.at(x, y);
                    if a > 0 {
                        out.blend(x, y, face, a);
                    }
                }
            }
            Some(out)
        }
    }

    /// A number in the numbers' font ([`crate::numbers`]), its boxes from
    /// `rect`'s top left, squashed `k` times about `mid` (a banner's number):
    /// each character's ink stretched to fill its box, then the rim around
    /// them (a frame pixel, square, as the game's is) or their shadow (a
    /// frame pixel right and down), and their face's colors down the boxes'
    /// rows.
    #[allow(clippy::too_many_arguments)]
    fn paint_fitted(
        &mut self,
        f: &crate::numbers::Fitted,
        rect: Rect,
        (k, mid): (f32, f32),
        (fine_x, fine_y): (f32, f32),
        place: &Placement,
        bounds: (i32, i32, i32, i32),
        color: &impl Fn(u16) -> u32,
    ) -> Option<Painted> {
        let s = place.scale;
        let (left, top) = (rect.x as f32 + fine_x, rect.y as f32 + fine_y);
        let mid = mid + fine_y;
        let squash = |y: f32| mid + (y - mid) * k;
        let area = (
            ((place.x as f32 + (left - 2.0) * s).floor() as i32).max(bounds.0),
            ((place.y as f32 + squash(top - 2.0) * s).floor() as i32).max(bounds.1),
            ((place.x as f32 + (left + rect.w as f32 + 2.0) * s).ceil() as i32).min(bounds.2),
            ((place.y as f32 + squash(top + rect.h as f32 + 2.0) * s).ceil() as i32).min(bounds.3),
        );
        if area.0 >= area.2 || area.1 >= area.3 {
            return None;
        }
        let mut mask = Mask::new(area);
        // (The rim: each glyph's own shape grown a frame pixel, even all
        // round, as the game's are.)
        let one = s.round().max(1.0) as i32;
        let mut grown = f.rim.map(|_| Mask::new(area));
        // The set's glyphs' ink, in texels: its glyphs' height (the median
        // of theirs: a digit's; a sign is let be), and their rows' top and
        // bottom (a code font's face: its letters' top to the baseline).
        let face = f.face;
        // (At the weight its face box gives it (`numbers::Face::weight`: the
        // digits' bars the game's 2 pixels however tall the box), its
        // outlines the axis's there.)
        let weight = f.glyphs.first().and_then(|g| face.weight(g.1[3]));
        let w = weight.map_or(0, |w| w.round() as u16);
        // (Each glyph by its glyph id where the run shaped it (`ids`), else
        // its character's.)
        let id = |c: char| face.font().font().charmap().map(c);
        let ids: Vec<u16> = f.glyphs.iter().enumerate().map(|(i, g)| f.ids.get(i).copied().unwrap_or_else(|| id(g.0))).collect();
        for g in f.set.chars().map(id).chain(ids.iter().copied()) {
            self.number_glyph_ink(face, weight, g);
        }
        let inks: Vec<[f32; 4]> = f.set.chars().filter_map(|c| self.number_glyphs.get(&(face, w, id(c))).and_then(|g| g.as_ref()).map(|(_, b)| *b)).collect();
        let mut heights: Vec<f32> = inks.iter().map(|b| b[3] - b[1]).collect();
        heights.sort_by(f32::total_cmp);
        let tallest = heights.get(heights.len() / 2).copied().or_else(|| ids.iter().filter_map(|g| self.number_glyphs.get(&(face, w, *g)).and_then(|g| g.as_ref()).map(|(_, b)| b[3] - b[1])).reduce(f32::max)).unwrap_or(1.0);
        let (rows_top, rows_bottom) = (inks.iter().map(|b| b[3]).fold(f32::MIN, f32::max), inks.iter().map(|b| b[1]).fold(f32::MAX, f32::min));
        // (A glyph pixel a frame pixel across, at its place in its face, the
        // box being its face's: its ink in texels, made at `RES` to the em,
        // a texel upem / RES font units.)
        let m = face.game();
        let across = m.across * face.font().font().metrics(&[]).units_per_em as f32 / RES;
        for (&(_, [bx, by, _, bh]), g) in f.glyphs.iter().zip(&ids) {
            let Some((field, [x0, y0, x1, y1])) = &self.number_glyphs[&(face, w, *g)] else { continue };
            // (Down, one scale for the set, the game's own sets being its
            // glyphs stretched (the enemies' digits 8 pixels tall, the chip
            // window's 11, at one width): a digit font's ink the box's
            // height, each glyph centered down its box; a code font's face,
            // its letters' top to the baseline, the box's height, each glyph
            // at its own rows in it ('*' where the game's stands).)
            let (h, gy) = if face.rows_as_drawn() && rows_top > rows_bottom {
                let scale = bh / (rows_top - rows_bottom);
                ((y1 - y0) * scale, by + (rows_top - y1) * scale)
            } else {
                let h = (y1 - y0) * bh / tallest.max(1e-3);
                (h, by + (bh - h) / 2.0)
            };
            let (w, gx) = ((x1 - x0) * across, bx - m.inset.0 + x0 * across);
            let out_left = place.x as f32 + (left + gx) * s;
            let (out_top, out_bottom) = (place.y as f32 + squash(top + gy) * s, place.y as f32 + squash(top + gy + h) * s);
            let sx = w * s / (x1 - x0).max(1e-3);
            let sy = (out_bottom - out_top) / (y1 - y0).max(1e-3);
            field.draw(out_left - x0 * sx, out_top + y1 * sy, sx, sy, area, &mut mask);
            if let Some(grown) = &mut grown {
                field.draw_offset(out_left - x0 * sx, out_top + y1 * sy, sx, sy, s, area, grown);
            }
        }
        let mut out = Painted::new(area);
        let _ = one;
        // (A shadow as the names' beside them: the text layer's, half a frame
        // pixel right and down.)
        let drop = (SHADOW * s).round().max(1.0) as i32;
        let rim = f.rim.zip(grown).map(|(c, grown)| (color(c), grown));
        let shadow = f.shadow.map(color);
        let fill: Vec<u32> = if f.fill.is_empty() { vec![0] } else { f.fill.iter().map(|&c| color(c)).collect() };
        let face_top = top + f.glyphs.first().map_or(0.0, |g| g.1[1]);
        for y in area.1..area.3 {
            let row = mid + ((y as f32 + 0.5 - place.y as f32) / s - mid) / k - face_top - 0.5;
            let face = fill_color(&fill, row);
            for x in area.0..area.2 {
                if let Some((rgb, rim)) = &rim {
                    let a = rim.at(x, y);
                    if a > 0 {
                        out.blend(x, y, *rgb, a);
                    }
                }
                if let Some(rgb) = shadow {
                    let a = mask.at(x - drop, y - drop);
                    if a > 0 {
                        out.blend(x, y, rgb, a);
                    }
                }
                let a = mask.at(x, y);
                if a > 0 {
                    out.blend(x, y, face, a);
                }
            }
        }
        Some(out)
    }

    /// A numbers' font glyph's ink's width and height at a weight, in texels
    /// (made once).
    fn number_glyph_ink(&mut self, face: crate::numbers::Face, weight: Option<f32>, id: u16) -> Option<(f32, f32)> {
        let key = (face, weight.map_or(0, |w| w.round() as u16), id);
        if !self.number_glyphs.contains_key(&key) {
            let font = face.font().font();
            let mut scaler = self.scaler.builder(font).size(RES).variations(crate::numbers::axes(weight)).build();
            let glyph = scaler.scale_outline(id).and_then(|o| {
                let b = o.bounds();
                Some((Sdf::of_spread(o.points(), o.verbs(), NUMBER_SPREAD)?, [b.min.x, b.min.y, b.max.x, b.max.y]))
            });
            self.made.glyphs += 1;
            self.number_glyphs.insert(key, glyph);
        }
        self.number_glyphs[&key].as_ref().map(|(_, [x0, y0, x1, y1])| (x1 - x0, y1 - y0))
    }

    /// Paint a layout's glyphs and rings (`at`: where, squashed how) into
    /// `paint`, `off` output pixels right and down, within `bounds`.
    #[allow(clippy::too_many_arguments)]
    fn paint_layout(&mut self, layout: &Layout, st: Style, at: &Laid, place: &Placement, bounds: (i32, i32, i32, i32), off: i32, paint: &mut impl Paint) {
        let (s, k) = (place.scale, at.k);
        for g in layout.glyphs.iter().filter(|g| g.unit < at.shown) {
            let Some(field) = self.glyph(GlyphKey::of(g, layout.shorten, k, at.floor)) else { continue };
            // The glyph's origin, and a texel's size, in output pixels:
            // across, where the layout puts it; down, on a whole output
            // row, so that the baseline and what stands on it are on a
            // pixel's edge (the chatbox's baseline is half a frame pixel
            // down, a kana's a part of one).
            let ox = place.x as f32 + (at.x0 + g.x) * s + off as f32;
            let oy = (place.y as f32 + (at.baseline - g.y * k) * s).round() + off as f32;
            let (sx, sy) = (g.size * s * layout.xscale * g.stretch / RES, g.size * s * k / RES);
            field.draw(ox, oy, sx, sy, bounds, paint);
        }
        // A button mark's ring: antialiased, its stroke inside its outer
        // edge, narrowed and squashed with the text.
        for r in layout.rings.iter().filter(|r| r.unit < at.shown) {
            let (rx, ry) = (r.radius * layout.xscale * s, r.radius * k * s);
            let stroke = (BUTTON_STROKE * st.cap * s).max(1.0);
            let cx = place.x as f32 + (at.x0 + r.x) * s + off as f32;
            let cy = place.y as f32 + (at.baseline - r.y * k) * s + off as f32;
            let mean = (rx + ry) / 2.0;
            let ys = ((cy - ry - 1.0).floor() as i32).max(bounds.1)..=((cy + ry + 1.0).ceil() as i32).min(bounds.3 - 1);
            for y in ys {
                for x in ((cx - rx - 1.0).floor() as i32).max(bounds.0)..=((cx + rx + 1.0).ceil() as i32).min(bounds.2 - 1) {
                    let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                    // Output pixels out from the outer edge (inside: below 0).
                    let edge = ((dx * dx + dy * dy).sqrt() - 1.0) * mean;
                    let a = (0.5 - edge).clamp(0.0, 1.0) * (0.5 + edge + stroke).clamp(0.0, 1.0);
                    if a > 0.0 {
                        paint.pixel(x, y, (a * 255.0).round() as u8);
                    }
                }
            }
        }
    }
}

impl TextRenderer {
    /// Paint a layout's glyphs and rings grown by `grow` output pixels all
    /// round (a rim as wide, even on every side: each glyph's field offset,
    /// `Sdf::draw_offset`, each ring wider out and in) into `paint`, within
    /// `bounds`.
    #[allow(clippy::too_many_arguments)]
    fn paint_layout_grown(&mut self, layout: &Layout, st: Style, at: &Laid, place: &Placement, bounds: (i32, i32, i32, i32), grow: f32, paint: &mut impl Paint) {
        let (s, k) = (place.scale, at.k);
        for g in layout.glyphs.iter().filter(|g| g.unit < at.shown) {
            let (sx, sy) = (g.size * s * layout.xscale * g.stretch / RES, g.size * s * k / RES);
            // (Its field as far out as the rim, in texels.)
            let spread = (grow / sx.min(sy).max(1e-3) + 2.0).ceil().clamp(crate::sdf::SPREAD, 120.0) as u8;
            let key = GlyphKey { spread, ..GlyphKey::of(g, layout.shorten, k, at.floor) };
            let Some(field) = self.glyph(key) else { continue };
            let ox = place.x as f32 + (at.x0 + g.x) * s;
            let oy = (place.y as f32 + (at.baseline - g.y * k) * s).round();
            field.draw_offset(ox, oy, sx, sy, grow, bounds, paint);
        }
        for r in layout.rings.iter().filter(|r| r.unit < at.shown) {
            let (rx, ry) = (r.radius * layout.xscale * s + grow, r.radius * k * s + grow);
            let stroke = (BUTTON_STROKE * st.cap * s).max(1.0) + 2.0 * grow;
            let cx = place.x as f32 + (at.x0 + r.x) * s;
            let cy = place.y as f32 + (at.baseline - r.y * k) * s;
            let mean = (rx + ry) / 2.0;
            let ys = ((cy - ry - 1.0).floor() as i32).max(bounds.1)..=((cy + ry + 1.0).ceil() as i32).min(bounds.3 - 1);
            for y in ys {
                for x in ((cx - rx - 1.0).floor() as i32).max(bounds.0)..=((cx + rx + 1.0).ceil() as i32).min(bounds.2 - 1) {
                    let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                    let edge = ((dx * dx + dy * dy).sqrt() - 1.0) * mean;
                    let a = (0.5 - edge).clamp(0.0, 1.0) * (0.5 + edge + stroke).clamp(0.0, 1.0);
                    if a > 0.0 {
                        paint.pixel(x, y, (a * 255.0).round() as u8);
                    }
                }
            }
        }
    }
}

/// Where a layout is drawn: its left and its baseline in frame pixels (the
/// squash applied), the squash, the short descenders' floor and how many
/// of its characters show.
struct Laid {
    x0: f32,
    baseline: f32,
    k: f32,
    floor: f32,
    shown: usize,
}

/// The lettering's rim, in frame pixels: as wide as the picture's, one
/// pixel, even all round (each glyph's field offset that far).
pub(crate) const RIM: f32 = 1.0;

/// How far out the numbers' glyphs' fields hold distances, in texels: a
/// frame pixel's rim at the smallest boxes the game's numbers fill (a
/// digit's ink some 90 texels high in a box of 4 rows: 23 texels a pixel).
const NUMBER_SPREAD: f32 = 40.0;

/// The fill's color at `row` (the capitals' rows from the first's middle,
/// continuous): blended between the two rows it falls between, the first's
/// or the last's past them.
/// The face's color of the picture's row that `row` (from the first row's
/// middle, as [`fill_color`] takes it) falls in: a band a row, no blend.
fn row_color(fill: &[u32], row: f32) -> u32 {
    let last = fill.len() - 1;
    fill[((row + 0.5).floor().max(0.0) as usize).min(last)]
}

fn fill_color(fill: &[u32], row: f32) -> u32 {
    let last = fill.len() - 1;
    let r = row.clamp(0.0, last as f32);
    let i = (r.floor() as usize).min(last);
    let (a, b) = (fill[i], fill[(i + 1).min(last)]);
    let t = r - i as f32;
    if t <= 0.0 || a == b {
        return a;
    }
    let mix = |shift: u32| {
        let (ca, cb) = (((a >> shift) & 0xFF) as f32, ((b >> shift) & 0xFF) as f32);
        ((ca + (cb - ca) * t).round() as u32) << shift
    };
    mix(16) | mix(8) | mix(0)
}

/// The lettering items painted kept ([`TextRenderer`]'s `painted`) before
/// they are made again.

/// A lettering item's pixels over an area of the output, painted once
/// (its bar, rim, shadow and fill, each over the one before), to draw over
/// the picture through its clip and the frame's depth: each pixel's color
/// and coverage.
#[derive(Clone, Debug)]
struct Painted {
    x: i32,
    y: i32,
    w: usize,
    px: Vec<(u32, u8)>,
}

impl Painted {
    fn new((x0, y0, x1, y1): (i32, i32, i32, i32)) -> Painted {
        let (w, h) = ((x1 - x0).max(0) as usize, (y1 - y0).max(0) as usize);
        Painted { x: x0, y: y0, w, px: vec![(0, 0); w * h] }
    }

    /// The paint sheared: each row moved right by `slant` output pixels for
    /// each one it is above row `bottom` (left below it), between pixels
    /// its two neighbors' coverage blended; the area widened to hold it.
    fn sheared(&self, slant: f32, bottom: f32) -> Painted {
        let h = if self.w == 0 { 0 } else { self.px.len() / self.w };
        let shift = |y: usize| slant * (bottom - (self.y as f32 + y as f32 + 0.5));
        let (lo, hi) = (0..h).map(shift).fold((0.0f32, 0.0f32), |(a, b), v| (a.min(v), b.max(v)));
        let (left, right) = (lo.floor() as i32, hi.ceil() as i32);
        let w = self.w + (right - left) as usize + 1;
        let mut out = Painted { x: self.x + left, y: self.y, w, px: vec![(0, 0); w * h] };
        for y in 0..h {
            let d = shift(y) - left as f32;
            let (whole, part) = (d.floor() as usize, d - d.floor());
            for x in 0..self.w {
                let (rgb, a) = self.px[y * self.w + x];
                if a == 0 {
                    continue;
                }
                // (Its coverage split between the two pixels it lands across.)
                let (a0, a1) = ((a as f32 * (1.0 - part)).round() as u8, (a as f32 * part).round() as u8);
                for (dx, aa) in [(0, a0), (1, a1)] {
                    if aa > 0 {
                        let i = y * w + x + whole + dx;
                        if let Some(o) = out.px.get_mut(i) {
                            // (Where two meet, their colors weighed by their coverage.)
                            let (wa, wb) = (o.1 as u32, aa as u32);
                            let mix = |sh: u32| ((((o.0 >> sh) & 0xFF) * wa + ((rgb >> sh) & 0xFF) * wb) / (wa + wb).max(1)) << sh;
                            *o = (mix(16) | mix(8) | mix(0), (wa + wb).min(255) as u8);
                        }
                    }
                }
            }
        }
        out
    }

    /// Cut to the pixels it covers (none: none).
    fn cropped(self) -> Option<Painted> {
        let h = if self.w == 0 { 0 } else { self.px.len() / self.w };
        let (mut x0, mut y0, mut x1, mut y1) = (self.w, h, 0, 0);
        for y in 0..h {
            let row = &self.px[y * self.w..(y + 1) * self.w];
            let Some(first) = row.iter().position(|p| p.1 > 0) else { continue };
            let last = row.iter().rposition(|p| p.1 > 0).unwrap_or(first);
            (x0, y0, x1, y1) = (x0.min(first), y0.min(y), x1.max(last + 1), y1.max(y + 1));
        }
        if x0 >= x1 {
            return None;
        }
        let px = (y0..y1).flat_map(|y| self.px[y * self.w + x0..y * self.w + x1].iter().copied()).collect();
        Some(Painted { x: self.x + x0 as i32, y: self.y + y0 as i32, w: x1 - x0, px })
    }

    /// `rgb` over the pixel at (x, y), `a` 255ths of it.
    fn blend(&mut self, x: i32, y: i32, rgb: u32, a: u8) {
        let (dx, dy) = (x - self.x, y - self.y);
        if dx < 0 || dy < 0 || dx as usize >= self.w {
            return;
        }
        let Some(p) = self.px.get_mut(dy as usize * self.w + dx as usize) else { return };
        if a == 255 || p.1 == 0 {
            *p = (rgb, a);
            return;
        }
        let (sa, da) = (a as f32 / 255.0, p.1 as f32 / 255.0);
        let oa = sa + da * (1.0 - sa);
        let mix = |shift: u32| {
            let (c, d) = (((rgb >> shift) & 0xFF) as f32, ((p.0 >> shift) & 0xFF) as f32);
            (((c * sa + d * da * (1.0 - sa)) / oa).round() as u32) << shift
        };
        *p = (mix(16) | mix(8) | mix(0), (oa * 255.0).round() as u8);
    }
}

impl From<Painted> for crate::rasters::Image {
    fn from(p: Painted) -> Self {
        let h = if p.w == 0 { 0 } else { p.px.len() / p.w };
        crate::rasters::Image { x: p.x, y: p.y, w: p.w, h, px: p.px }
    }
}

/// A color (0RGB, as `to_rgb` widens the frame's) faded as `apply_fade`
/// fades the frame's colors: each channel at the frame's five bits.
fn fade_rgb(rgb: u32, fade: Fade) -> u32 {
    if fade == Fade::None {
        return rgb;
    }
    let five = |shift: u32| ((rgb >> (shift + 3)) & 0x1F) as u16;
    to_rgb(apply_fade(five(16) | five(8) << 5 | five(0) << 10, fade))
}

/// An item's glyphs' coverage over an area of the output (the lettering's,
/// whose rim and fill are drawn from it).
struct Mask {
    x: i32,
    y: i32,
    w: usize,
    h: usize,
    a: Vec<u8>,
}

impl Mask {
    fn new((x0, y0, x1, y1): (i32, i32, i32, i32)) -> Mask {
        let (w, h) = ((x1 - x0).max(0) as usize, (y1 - y0).max(0) as usize);
        Mask { x: x0, y: y0, w, h, a: vec![0; w * h] }
    }

    /// Its uncovered pixels that are closed in (none of them reaches its
    /// edge through uncovered pixels) covered whole.
    fn fill_enclosed(&mut self) {
        let (w, h) = (self.w, self.h);
        let open = |a: u8| a < 128;
        let mut outside = vec![false; w * h];
        let mut stack: Vec<usize> = (0..w * h).filter(|&i| (i % w == 0 || i % w == w - 1 || i / w == 0 || i / w == h - 1) && open(self.a[i])).collect();
        for &i in &stack {
            outside[i] = true;
        }
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            let next = [(x > 0).then(|| i - 1), (x + 1 < w).then(|| i + 1), (y > 0).then(|| i - w), (y + 1 < h).then(|| i + w)];
            for j in next.into_iter().flatten() {
                if !outside[j] && open(self.a[j]) {
                    outside[j] = true;
                    stack.push(j);
                }
            }
        }
        for (a, out) in self.a.iter_mut().zip(outside) {
            if !out {
                *a = 255;
            }
        }
    }

    fn at(&self, x: i32, y: i32) -> u8 {
        let (dx, dy) = (x - self.x, y - self.y);
        if dx < 0 || dy < 0 || dx as usize >= self.w || dy as usize >= self.h {
            return 0;
        }
        self.a[dy as usize * self.w + dx as usize]
    }
}

impl Paint for Mask {
    fn pixel(&mut self, x: i32, y: i32, a: u8) {
        let (dx, dy) = (x - self.x, y - self.y);
        if dx < 0 || dy < 0 || dx as usize >= self.w || dy as usize >= self.h {
            return;
        }
        let o = &mut self.a[dy as usize * self.w + dx as usize];
        *o = (*o).max(a);
    }
}

/// A layout drawn `k` times as wide: its glyphs' places and its rings'
/// moved in (or out) from its left, its glyphs narrowed with them.
fn narrowed(mut l: Layout, k: f32) -> Layout {
    if k == 1.0 {
        return l;
    }
    for g in &mut l.glyphs {
        g.x *= k;
    }
    for r in &mut l.rings {
        r.x *= k;
    }
    l.width *= k;
    l.xscale *= k;
    l
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

/// The frame pixel each output column and row of the picture shows
/// (`Placement::frame_pixel`), from the picture's top left corner.
struct FrameMap<'f> {
    x: i32,
    y: i32,
    columns: &'f [i32],
    rows: &'f [i32],
    /// The overlays' depth keys on the output's pixels (none: no overlays).
    cover: Option<&'f [u32]>,
}

/// Where an item's pixels go: the output, through its clip and the frame's
/// depth keys.
struct Target<'o> {
    depth: &'o [u32],
    frame: &'o FrameMap<'o>,
    out: &'o mut [u32],
    out_w: usize,
    clip: Rect,
    key: u32,
}

impl Target<'_> {
    /// The frame row output row `y` shows, if the clip has it.
    fn row(&self, y: i32) -> Option<i32> {
        let fy = *self.frame.rows.get((y - self.frame.y) as usize)?;
        (fy >= self.clip.y && fy < self.clip.y + self.clip.h).then_some(fy)
    }

    /// The frame column output column `x` shows, if the clip has it.
    fn column(&self, x: i32) -> Option<i32> {
        let fx = *self.frame.columns.get((x - self.frame.x) as usize)?;
        (fx >= self.clip.x && fx < self.clip.x + self.clip.w).then_some(fx)
    }

    /// `image` laid with its origin at output pixel (`ox`, `oy`), its
    /// pixels within `bounds` (left, top, right, bottom) blended in, each
    /// color through `color`. (Its columns within the bounds, and each
    /// row's frame row, found once.)
    fn blit(&mut self, image: &crate::rasters::Image, (ox, oy): (i32, i32), bounds: (i32, i32, i32, i32), color: impl Fn(u32) -> u32) {
        let (left, top) = (ox + image.x, oy + image.y);
        let columns = (bounds.0 - left).clamp(0, image.w as i32) as usize..(bounds.2 - left).clamp(0, image.w as i32) as usize;
        for ly in 0..image.h {
            let y = top + ly as i32;
            if y < bounds.1 || y >= bounds.3 {
                continue;
            }
            let Some(fy) = self.row(y) else { continue };
            for lx in columns.clone() {
                let (rgb, a) = image.px[ly * image.w + lx];
                if a == 0 {
                    continue;
                }
                let x = left + lx as i32;
                if let Some(fx) = self.column(x) {
                    self.blend_at(x, y, (fx, fy), color(rgb), a);
                }
            }
        }
    }

    /// Blend `rgb` at coverage `a` into output pixel (`x`, `y`), which
    /// shows the frame pixel `(fx, fy)` in the clip ([`Target::row`],
    /// [`Target::column`]), where nothing in front of the item won it.
    fn blend_at(&mut self, x: i32, y: i32, (fx, fy): (i32, i32), rgb: u32, a: u8) {
        if self.depth.get(fy as usize * WIDTH + fx as usize).is_some_and(|&d| d < self.key) {
            return;
        }
        debug_assert!((fy as usize) < HEIGHT);
        let i = y as usize * self.out_w + x as usize;
        if self.frame.cover.and_then(|c| c.get(i)).is_some_and(|&d| d < self.key) {
            return;
        }
        let Some(o) = self.out.get_mut(i) else { return };
        if a == 255 {
            *o = rgb;
            return;
        }
        let a = a as u32;
        let mix = |shift: u32| {
            let (c, b) = ((rgb >> shift) & 0xFF, (*o >> shift) & 0xFF);
            ((c * a + b * (255 - a) + 127) / 255) << shift
        };
        *o = mix(16) | mix(8) | mix(0);
    }

}

/// A target in a color: what a glyph's field paints.
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

    /// What a text renderer has made (its layouts, its glyphs, the items it
    /// painted) changes nothing it draws: frames presented one after
    /// another by one renderer are each what a renderer like it with nothing
    /// made draws, in any order, at any size. (A video's frames are
    /// presented by several, a frame each: nettai-video's `presenters`.)
    #[test]
    fn a_renderer_draws_a_frame_the_same_whatever_it_drew_before() {
        use crate::compose::Fade;
        use crate::textlayer::{Align, Ink};
        let frames: Vec<crate::render::Frame> = (0..6)
            .map(|n| {
                let words = ["Cannon", "GunDelS3", "ElecMan\u{E002}", "Press Ⓐ", "ミテイ ロックマン", "gjpqy"];
                let mut items = vec![
                    TextItem::new(words[n], Role::Cell, Rect::new(8 + 4 * n as i32, 8, 64, 16), 0x7FFF, Some(0x2108)),
                    TextItem::new(words[(n + 2) % 6], Role::Dialogue, Rect::new(16, 120, 192, 12), 0x0421 * (n as u16 + 3), None),
                    TextItem { vscale: Some(0xC0 + 0x10 * n as i32), align: Align::Center, ..TextItem::new(words[(n + 1) % 6], Role::FittedCell, Rect::new(80, 40, 80, 16), 0x03FF, Some(0x0010)) },
                    TextItem { fades: [Fade::Black(n as u8 * 3), Fade::None], shown: Some(n + 1), ..TextItem::new(words[(n + 3) % 6], Role::Cell, Rect::new(120, 70, 96, 16), 0x7C00, Some(0x0000)) },
                    TextItem {
                        ink: Some(Ink { fill: vec![0x7FFF, 0x5294, 0x318C], rim: Some(0x0000), rim_width: 1.0, stepped: n % 2 == 0 }),
                        ..TextItem::new("COUNTER HIT!", Role::Cell, Rect::new(40, 90, 120, 16), 0x7FFF, Some(0x2108))
                    },
                ];
                items[1].depth = 7;
                let depth = (0..WIDTH * HEIGHT).map(|i| if (i / WIDTH + i % WIDTH) % 9 == 0 { 3 } else { u32::MAX }).collect();
                crate::render::Frame { pixels: vec![0x1234; WIDTH * HEIGHT], depth, text: items, ..crate::render::Frame::default() }
            })
            .collect();
        for scale in [2, 3, 4] {
            let (w, h) = (WIDTH * scale, HEIGHT * scale);
            let drawn = |frame: &crate::render::Frame, r: &mut TextRenderer| {
                let mut out = vec![0u32; w * h];
                crate::present::present(frame, Some(r), &mut out, w, h);
                out
            };
            let first = renderer();
            let alone: Vec<Vec<u32>> = frames.iter().map(|f| drawn(f, &mut first.like())).collect();
            assert!(alone.iter().all(|out| out.iter().any(|&p| p != crate::compose::to_rgb(0x1234))), "text drawn at {scale}x");
            let mut one = first.like();
            for (n, f) in frames.iter().enumerate().chain(frames.iter().enumerate().rev()) {
                assert!(drawn(f, &mut one) == alone[n], "frame {n} at {scale}x, after others");
            }
        }
    }

    /// A white item over a black frame at 2x, `depth` the frame's depth
    /// keys: which output columns of the box's rows it lit.
    fn lit_columns(item: TextItem, depth: Vec<u32>) -> Vec<bool> {
        let frame = crate::render::Frame { pixels: vec![0; WIDTH * HEIGHT], depth, text: vec![item], glide: None, ..crate::render::Frame::default() };
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

    /// A glyph's field draws what its outline covers: at 5x and 13x, Latin,
    /// kana and kanji (an e and kanji can be overlapping contours),
    /// each pixel's coverage is the outline's rasterized (zeno's), but for
    /// a little at corners.
    #[test]
    fn a_field_draws_what_its_outline_covers() {
        let r = renderer();
        let font = r.font.font();
        let mut context = ScaleContext::new();
        for scale in [5.0f32, 13.0] {
            let size = Role::Cell.style().cap / r.font.cap * scale;
            let mut fields = context.builder(font).size(RES).variations([("wght", 700.0f32)]).build();
            let outlines: Vec<_> = "eSWgBガ熱撃".chars().map(|c| fields.scale_outline(font.charmap().map(c)).unwrap()).collect();
            let mut zeno = context.builder(font).size(size).variations([("wght", 700.0f32)]).build();
            for (outline, c) in outlines.iter().zip("eSWgBガ熱撃".chars()) {
                let field = Sdf::of(outline.points(), outline.verbs()).unwrap();
                let (w, h) = (400usize, 400usize);
                let mut ours = vec![0u8; w * h];
                field.draw(100.0, 300.0, size / RES, size / RES, (0, 0, w as i32, h as i32), &mut |x: i32, y: i32, a| ours[y as usize * w + x as usize] = a);
                let image = swash::scale::Render::new(&[swash::scale::Source::Outline])
                    .format(swash::zeno::Format::Alpha)
                    .render(&mut zeno, font.charmap().map(c))
                    .unwrap();
                let mut theirs = vec![0u8; w * h];
                let p = image.placement;
                for y in 0..p.height as usize {
                    for x in 0..p.width as usize {
                        theirs[(300 - p.top + y as i32) as usize * w + (100 + p.left + x as i32) as usize] = image.data[y * p.width as usize + x];
                    }
                }
                let diffs: Vec<i32> = ours.iter().zip(&theirs).map(|(&a, &b)| (a as i32 - b as i32).abs()).filter(|&d| d > 0).collect();
                let inked = theirs.iter().filter(|&&a| a > 0).count();
                let (most, total) = (diffs.iter().copied().max().unwrap_or(0), diffs.iter().sum::<i32>());
                let far = diffs.iter().filter(|&&d| d > 64).count();
                // Off by under 1% of the ink in all (measured: 0.1 to
                // 0.6%); more than a quarter off at a few pixels of a
                // corner (at most 0.4% of the ink, 撃 at 5x), none by
                // more than 0.55.
                assert!((total as f32) < 0.01 * 255.0 * inked as f32, "{c} at {scale}x: {total} over {inked} inked");
                assert!(far * 200 <= inked && most < 160, "{c} at {scale}x: {far} pixels a quarter off, the most {most}");
            }
        }
    }

    /// A string's glyphs made ahead draw it at every size: drawing it in
    /// its role's box, narrowed or not, with its shadow, makes no more.
    #[test]
    fn prepared_glyphs_serve_every_size() {
        let mut r = renderer();
        let strings = [("BigBomb", Role::FittedCell), ("スプレッドガン1", Role::FittedCell), ("ComingRd", Role::Cell), ("敵に当たると\nダメージ gjpqy", Role::Dialogue)];
        for (text, role) in strings {
            r.prepare(text, role);
        }
        let made = r.made().glyphs;
        assert!(made > 20);
        let mut items = Vec::new();
        for (n, (text, role)) in strings.into_iter().enumerate() {
            for (k, line) in text.lines().enumerate() {
                let rows = role.style().rows as i32;
                items.push(TextItem::new(line, role, Rect::new(8, 8 + 16 * (n + k) as i32, 40, rows), 0x7FFF, Some(0x2108)));
            }
        }
        for scale in [1, 3, 7] {
            let frame = crate::render::Frame { pixels: vec![0; WIDTH * HEIGHT], depth: vec![u32::MAX; WIDTH * HEIGHT], text: items.clone(), glide: None, ..crate::render::Frame::default() };
            let (w, h) = (WIDTH * scale, HEIGHT * scale);
            let mut out = vec![0u32; w * h];
            crate::present::present(&frame, Some(&mut r), &mut out, w, h);
            assert_eq!(r.made().glyphs, made, "{scale}x");
        }
    }
}
