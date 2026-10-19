//! The text layer's font and its drawing: a TrueType or OpenType outline
//! font, static or variable, laid out in the frame's pixels (shaped with
//! the font's kerning) and drawn with antialiasing at the output's
//! resolution, over the scaled frame, from each glyph's signed distance
//! field, made once whatever the output's size (swash's outlines, `sdf`).
//! docs/design/text-rendering.md, "As built".
//!
//! The bundled font is Murecho (fonts/murecho, SIL Open Font License 1.1):
//! a sans with Latin, Greek, Cyrillic, kana and some 2,300 kanji, in one
//! variable file with a weight axis. A role's text has one size (`Role`):
//! a string too wide for the original's box is narrowed to it, never made
//! smaller; a font with a width axis (`--font`) draws it in a narrower cut
//! of itself before narrowing it.
//!
//! The game-style lettering (the banners, the HUD's labels, the chip
//! window's help pictures, the damage numbers: [`Role::Lettering`]) is drawn
//! in a second bundled font, M PLUS 1 Code (fonts/mplus1code, SIL Open Font
//! License 1.1), whose square capitals, narrowed, are near the game's; it
//! has kana and kanji too. Its strings take the original's paint
//! ([`Ink`](crate::textlayer::Ink)): a fill that changes down the capitals,
//! a rim, and a banner's bar under them ([`Bar`](crate::textlayer::Bar)).
//! docs/design/text-rendering.md §12.

use crate::compose::{HEIGHT, WIDTH, apply_fade, to_rgb};
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

/// Murecho, its variable cut (weights 100 to 900; 1.4 MB): the font file,
/// for a host that draws with it otherwise.
pub const BUNDLED: &[u8] = include_bytes!("../fonts/murecho/Murecho-VariableFont_wght.ttf");

/// M PLUS 1 Code, its variable cut (weights 100 to 700; 3.9 MB): the
/// game-style lettering's font ([`Role::Lettering`]).
pub const LETTERING: &[u8] = include_bytes!("../fonts/mplus1code/MPLUS1Code-VariableFont_wght.ttf");

/// The game-style lettering's font, parsed once.
pub fn lettering_font() -> &'static std::sync::Arc<VectorFont> {
    static FONT: std::sync::LazyLock<std::sync::Arc<VectorFont>> = std::sync::LazyLock::new(|| {
        let font = VectorFont::from_data(Cow::Borrowed(LETTERING), "M PLUS 1 Code (bundled)".into());
        std::sync::Arc::new(font.expect("the bundled lettering font parses"))
    });
    &FONT
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
/// deepest descender from ([`Shorten`]; Murecho's: g, 0.235 em).
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
    /// 13; bold, with a shadow. A descender (Murecho's reach 3.2 pixels
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
    /// The lettering font ([`lettering_font`]: M PLUS 1 Code).
    #[default]
    Display,
    /// The text font (the text layer's own: Murecho, or `--font`'s).
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
    /// The weight of the game's display lettering (the font's heaviest).
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
            // Murecho's g, 3.2 pixels deep at this size, is drawn 1 deep
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

    /// Whether it is drawn in the lettering font.
    pub fn lettering(self) -> bool {
        matches!(self, Role::Lettering(_))
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

    /// A font file compiled in.
    pub fn from_bytes(data: &'static [u8], name: &str) -> Result<VectorFont, String> {
        VectorFont::from_data(Cow::Borrowed(data), name.into())
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
    painted: HashMap<String, Option<Painted>>,
    /// The numbers' font's glyphs' fields and their ink's bounds (texels:
    /// left, bottom, right, top), by character (`numbers`).
    number_glyphs: HashMap<char, Option<(Sdf, [f32; 4])>>,
    made: Made,
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
}

impl TextRenderer {
    pub fn new(font: std::sync::Arc<VectorFont>) -> TextRenderer {
        TextRenderer {
            font,
            lettering: lettering_font().clone(),
            numbers: crate::numbers::font().clone(),
            scaler: ScaleContext::new(),
            shaper: ShapeContext::new(),
            layouts: HashMap::new(),
            glyphs: HashMap::new(),
            glyph_bytes: 0,
            faces: HashMap::new(),
            descenders: HashMap::new(),
            painted: HashMap::new(),
            number_glyphs: HashMap::new(),
            made: Made::default(),
        }
    }

    pub fn font(&self) -> &VectorFont {
        &self.font
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
        self.made
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
                Sdf::of(outline.points(), outline.verbs())
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
        // The frame pixel each output column and row of the picture shows.
        let columns: Vec<i32> = (0..place.width).map(|dx| place.frame_pixel(place.x + dx, place.y).map_or(0, |p| p.0)).collect();
        let rows: Vec<i32> = (0..place.height).map(|dy| place.frame_pixel(place.x, place.y + dy).map_or(0, |p| p.1)).collect();
        let frame = FrameMap { x: place.x, y: place.y, columns: &columns, rows: &rows };
        for item in items {
            self.draw_item(item, depth, place, &frame, out, out_w);
        }
    }

    fn draw_item(&mut self, item: &TextItem, depth: &[u32], place: &Placement, frame: &FrameMap, out: &mut [u32], out_w: usize) {
        let layout = if item.fitted.is_some() { Layout::default() } else { self.layout(&item.text, item.role, item.rect.w).clone() };
        if layout.glyphs.is_empty() && item.bar.is_none() && item.fitted.is_none() {
            return;
        }
        let s = place.scale;
        let st = item.role.style();
        // (A sprite's true place, past its whole pixels.)
        let (fine_x, fine_y) = (item.fine.0 as f32 / 65536.0, item.fine.1 as f32 / 65536.0);
        let x0 = item.rect.x as f32
            + fine_x
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
        let baseline = baseline + fine_y;
        let clip = item.clip.intersect(Rect::SCREEN);
        if clip.is_empty() {
            return;
        }
        let color = |c: u16| to_rgb(apply_fade(apply_fade(c, item.fades[0]), item.fades[1]));
        let mut target = Target { depth, frame, out, out_w, clip, key: item.depth };
        // The output pixels the clip shows (a pixel more each way: `Target`
        // tests each against the clip's frame pixels).
        let bounds = (
            place.x + (clip.x as f32 * s).floor() as i32 - 1,
            place.y + (clip.y as f32 * s).floor() as i32 - 1,
            place.x + ((clip.x + clip.w) as f32 * s).ceil() as i32 + 1,
            place.y + ((clip.y + clip.h) as f32 * s).ceil() as i32 + 1,
        );
        // Short descenders: how far under the text's baseline its ink may
        // reach (the box's bottom, less the shadow's offset at its most),
        // in frame pixels.
        let floor = (item.rect.y + item.rect.h) as f32 - (item.rect.y as f32 + st.baseline) - SHADOW_MOST;
        let at = Laid { x0, baseline, k, floor, shown: item.shown.unwrap_or(usize::MAX) };
        let offset = (SHADOW * s).round().max(1.0) as i32;
        if let Some(f) = &item.fitted {
            // A number in the numbers' font, painted once for what it is and
            // where.
            let key = format!("fitted|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}", f, item.rect, item.fades, item.vscale, item.fine, place, bounds);
            if !self.painted.contains_key(&key) {
                if self.painted.len() >= PAINTED_KEPT {
                    self.painted.clear();
                }
                let painted = self.paint_fitted(f, item.rect, (k, mid), (fine_x, fine_y), place, bounds, &color);
                self.painted.insert(key.clone(), painted);
            }
            if let Some(p) = &self.painted[&key] {
                for (i, &(rgb, a)) in p.px.iter().enumerate().filter(|(_, px)| px.1 > 0) {
                    target.blend(p.x + (i % p.w) as i32, p.y + (i / p.w) as i32, rgb, a);
                }
            }
            return;
        }
        if let Some(ink) = &item.ink {
            // The lettering's paint, painted once for what it is and where:
            // its bar, then its glyphs' coverage, the rim around it, the
            // shadow and the face's fill drawn from it.
            let key = format!(
                "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
                item.text, item.role, item.rect, item.align, item.face, item.shadow, item.fades, item.vscale, item.shown, ink, item.bar, item.fine, place, bounds
            );
            if !self.painted.contains_key(&key) {
                if self.painted.len() >= PAINTED_KEPT {
                    self.painted.clear();
                }
                let painted = self.paint_inked(item, ink, &layout, st, &at, place, bounds, (k, mid), (fine_x, fine_y), &color);
                self.painted.insert(key.clone(), painted);
            }
            if let Some(p) = &self.painted[&key] {
                for (i, &(rgb, a)) in p.px.iter().enumerate().filter(|(_, px)| px.1 > 0) {
                    target.blend(p.x + (i % p.w) as i32, p.y + (i / p.w) as i32, rgb, a);
                }
            }
            return;
        }
        let passes: Vec<(u32, i32)> = item.shadow.map(|c| (color(c), offset)).into_iter().chain([(color(item.face), 0)]).collect();
        for (rgb, off) in passes {
            self.paint_layout(&layout, st, &at, place, bounds, off, &mut Pen { target: &mut target, rgb });
        }
    }

    /// A lettering item painted ([`Painted`]): its bar, and its glyphs' coverage once (a [`Mask`]), the rim
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
        (fine_x, fine_y): (f32, f32),
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
            let margin = 2.0 + 2.0 * RIM;
            let (left, right) = (x0.min(item.rect.x as f32) - margin, (x0 + layout.width).max((item.rect.x + item.rect.w) as f32) + margin);
            let (top, bottom) = (baseline - (st.cap * 1.6 + margin) * k, baseline + (st.cap * 0.6 + margin) * k);
            let area = (
                ((place.x as f32 + left * s).floor() as i32).max(bounds.0),
                ((place.y as f32 + top * s).floor() as i32).max(bounds.1),
                ((place.x as f32 + right * s).ceil() as i32).min(bounds.2),
                ((place.y as f32 + bottom * s).ceil() as i32).min(bounds.3),
            );
            let bar = item.bar.as_ref().map(|bar| (bar, bar_area(bar, (k, mid + fine_y), (fine_x, fine_y), place, bounds)));
            let all = match bar {
                Some((_, b)) => (area.0.min(b.0), area.1.min(b.1), area.2.max(b.2), area.3.max(b.3)),
                None => area,
            };
            if all.0 >= all.2 || all.1 >= all.3 {
                return None;
            }
            let mut out = Painted::new(all);
            if let Some((bar, b)) = bar {
                draw_bar(bar, b, (k, mid + fine_y), (fine_x, fine_y), place, &mut out, color);
            }
            if area.0 >= area.2 || area.1 >= area.3 {
                return Some(out);
            }
            let mut mask = Mask::new(area);
            self.paint_layout(layout, st, at, place, area, 0, &mut mask);
            // (A frame pixel all round, as the picture's; heavier right and
            // down where the picture's is.)
            let heavier = if ink.heavier { (RIM_HEAVIER * s).round() as i32 } else { 0 };
            let rim = ink.rim.map(|c| (color(c), mask.dilated(RIM * s, heavier)));
            let fill: Vec<u32> = if ink.fill.is_empty() { vec![color(item.face)] } else { ink.fill.iter().map(|&c| color(c)).collect() };
            let shadow = item.shadow.map(color);
            for y in area.1..area.3 {
                // The face's color on this output row: the picture's row
                // its middle falls on, blended between rows' middles.
                let row = unsquash((y as f32 + 0.5 - place.y as f32) / s) - cap_top - 0.5;
                let face = fill_color(&fill, row);
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
        for &(c, [bx, by, bw, bh]) in &f.glyphs {
            if !self.number_glyphs.contains_key(&c) {
                let font = crate::numbers::font().font();
                let mut scaler = self.scaler.builder(font).size(RES).build();
                let glyph = scaler.scale_outline(font.charmap().map(c)).and_then(|o| {
                    let b = o.bounds();
                    Some((Sdf::of(o.points(), o.verbs())?, [b.min.x, b.min.y, b.max.x, b.max.y]))
                });
                self.made.glyphs += 1;
                self.number_glyphs.insert(c, glyph);
            }
            let Some((field, [x0, y0, x1, y1])) = &self.number_glyphs[&c] else { continue };
            let out_left = place.x as f32 + (left + bx) * s;
            let (out_top, out_bottom) = (place.y as f32 + squash(top + by) * s, place.y as f32 + squash(top + by + bh) * s);
            let sx = bw * s / (x1 - x0).max(1e-3);
            let sy = (out_bottom - out_top) / (y1 - y0).max(1e-3);
            field.draw(out_left - x0 * sx, out_top + y1 * sy, sx, sy, area, &mut mask);
        }
        let mut out = Painted::new(area);
        let one = s.round().max(1.0) as i32;
        // (A shadow as the names' beside them: the text layer's, half a frame
        // pixel right and down.)
        let drop = (SHADOW * s).round().max(1.0) as i32;
        let rim = f.rim.map(|c| (color(c), mask.dilated_square(one)));
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
/// pixel, and where the picture's is heavier right and down (`Ink::heavier`)
/// that again half a pixel there.
const RIM: f32 = 1.0;
const RIM_HEAVIER: f32 = 0.5;

/// The fill's color at `row` (the capitals' rows from the first's middle,
/// continuous): blended between the two rows it falls between, the first's
/// or the last's past them.
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
const PAINTED_KEPT: usize = 32;

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

/// The output pixels a bar covers ([`draw_bar`]), within `bounds`.
fn bar_area(bar: &crate::textlayer::Bar, (k, mid): (f32, f32), (fine_x, fine_y): (f32, f32), place: &Placement, bounds: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    let s = place.scale;
    let squash = |y: f32| mid + (y - mid) * k;
    let (top, bottom) = (squash(bar.top + fine_y), squash(bar.bottom + fine_y));
    let left = bar.left_at(bar.bottom).min(bar.left_at(bar.top)) + fine_x;
    let right = bar.right_at(bar.top).max(bar.right_at(bar.bottom)) + fine_x;
    (
        ((place.x as f32 + left * s).floor() as i32).max(bounds.0),
        ((place.y as f32 + top * s).floor() as i32).max(bounds.1),
        ((place.x as f32 + right * s).ceil() as i32).min(bounds.2),
        ((place.y as f32 + bottom * s).ceil() as i32).min(bounds.3),
    )
}

/// A banner's bar ([`Bar`](crate::textlayer::Bar)) at the output's
/// resolution, over `area` ([`bar_area`]): each output pixel's coverage and
/// color from 4x4 points of it, read through the item's squash (`k` about
/// `mid`).
fn draw_bar(
    bar: &crate::textlayer::Bar,
    (x0, y0, x1, y1): (i32, i32, i32, i32),
    (k, mid): (f32, f32),
    (fine_x, fine_y): (f32, f32),
    place: &Placement,
    target: &mut Painted,
    color: &impl Fn(u16) -> u32,
) {
    const N: usize = 4;
    let s = place.scale;
    let colors = bar.colors;
    let rgbs: Vec<(u16, u32)> = [colors.top, colors.body, colors.bottom, colors.left, colors.right].iter().map(|&c| (c, color(c))).collect();
    let rgb = |c: u16| rgbs.iter().find(|r| r.0 == c).map_or(0, |r| r.1);
    for y in y0..y1 {
        // The points' rows in the bar (unsquashed).
        let rows: [f32; N] = std::array::from_fn(|j| {
            let fy = (y as f32 + (j as f32 + 0.5) / N as f32 - place.y as f32) / s;
            mid + (fy - mid) / k - fine_y
        });
        for x in x0..x1 {
            let (mut sum, mut n) = ([0u32; 3], 0u32);
            for &ty in &rows {
                for i in 0..N {
                    let fx = (x as f32 + (i as f32 + 0.5) / N as f32 - place.x as f32) / s - fine_x;
                    if let Some(c) = bar.color_at(fx, ty) {
                        let c = rgb(c);
                        sum[0] += (c >> 16) & 0xFF;
                        sum[1] += (c >> 8) & 0xFF;
                        sum[2] += c & 0xFF;
                        n += 1;
                    }
                }
            }
            if n == 0 {
                continue;
            }
            let avg = ((sum[0] + n / 2) / n) << 16 | ((sum[1] + n / 2) / n) << 8 | (sum[2] + n / 2) / n;
            target.blend(x, y, avg, ((n * 255 + (N * N) as u32 / 2) / (N * N) as u32) as u8);
        }
    }
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

    fn at(&self, x: i32, y: i32) -> u8 {
        let (dx, dy) = (x - self.x, y - self.y);
        if dx < 0 || dy < 0 || dx as usize >= self.w || dy as usize >= self.h {
            return 0;
        }
        self.a[dy as usize * self.w + dx as usize]
    }

    /// The coverage grown by a square `reach` output pixels each way (the
    /// game's rims are a pixel's square around the face).
    fn dilated_square(&self, reach: i32) -> Mask {
        let half = reach.max(0) as usize;
        let mut wide = vec![0u8; self.w * self.h];
        for y in 0..self.h {
            row_max(&self.a[y * self.w..(y + 1) * self.w], half, &mut wide[y * self.w..(y + 1) * self.w]);
        }
        let mut a = vec![0u8; self.w * self.h];
        for y in 0..self.h as i32 {
            for from in (y - reach).max(0)..(y + reach + 1).min(self.h as i32) {
                let (dst, src) = (y as usize * self.w, from as usize * self.w);
                for x in 0..self.w {
                    a[dst + x] = a[dst + x].max(wide[src + x]);
                }
            }
        }
        Mask { a, ..*self }
    }

    /// The coverage grown by a disk of `radius` output pixels, and that
    /// again `heavier` pixels right and down: a rim's.
    fn dilated(&self, radius: f32, heavier: i32) -> Mask {
        let r = radius.max(1.0);
        let reach = r.floor() as i32;
        // Each row's coverage widened by each half-width the disk has.
        let halves: Vec<usize> = (-reach..=reach).map(|dy| (r * r - (dy * dy) as f32).max(0.0).sqrt().floor() as usize).collect();
        let mut widened: HashMap<usize, Vec<u8>> = HashMap::new();
        for &half in &halves {
            widened.entry(half).or_insert_with(|| {
                let mut out = vec![0; self.w * self.h];
                for y in 0..self.h {
                    row_max(&self.a[y * self.w..(y + 1) * self.w], half, &mut out[y * self.w..(y + 1) * self.w]);
                }
                out
            });
        }
        let mut grown = vec![0u8; self.w * self.h];
        for y in 0..self.h as i32 {
            let row = &mut grown[y as usize * self.w..(y as usize + 1) * self.w];
            for (k, &half) in halves.iter().enumerate() {
                let from = y + k as i32 - reach;
                if from < 0 || from >= self.h as i32 {
                    continue;
                }
                let src = &widened[&half][from as usize * self.w..(from as usize + 1) * self.w];
                for (o, &v) in row.iter_mut().zip(src) {
                    *o = (*o).max(v);
                }
            }
        }
        let mut a = grown.clone();
        if heavier > 0 {
            let d = heavier as usize;
            for y in d..self.h {
                for x in d..self.w {
                    let v = grown[(y - d) * self.w + x - d];
                    let o = &mut a[y * self.w + x];
                    *o = (*o).max(v);
                }
            }
        }
        Mask { a, ..*self }
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

/// `out[x]` the most of `row[x - half..=x + half]` (van Herk's: blocks of
/// the window's width, their running maxima each way).
fn row_max(row: &[u8], half: usize, out: &mut [u8]) {
    if half == 0 {
        out.copy_from_slice(row);
        return;
    }
    let (n, m) = (row.len(), 2 * half + 1);
    let len = n + 2 * half;
    let p = |i: usize| if i >= half && i - half < n { row[i - half] } else { 0 };
    let mut ahead = vec![0u8; len];
    let mut behind = vec![0u8; len];
    for i in 0..len {
        ahead[i] = if i % m == 0 { p(i) } else { ahead[i - 1].max(p(i)) };
    }
    for i in (0..len).rev() {
        behind[i] = if i % m == m - 1 || i == len - 1 { p(i) } else { behind[i + 1].max(p(i)) };
    }
    for (x, o) in out.iter_mut().enumerate() {
        *o = behind[x].max(ahead[(x + m - 1).min(len - 1)]);
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
    fn blend(&mut self, x: i32, y: i32, rgb: u32, a: u8) {
        let (dx, dy) = ((x - self.frame.x) as usize, (y - self.frame.y) as usize);
        let (Some(&fx), Some(&fy)) = (self.frame.columns.get(dx), self.frame.rows.get(dy)) else { return };
        if fx < self.clip.x || fx >= self.clip.x + self.clip.w || fy < self.clip.y || fy >= self.clip.y + self.clip.h {
            return;
        }
        if self.depth.get(fy as usize * WIDTH + fx as usize).is_some_and(|&d| d < self.key) {
            return;
        }
        debug_assert!((fy as usize) < HEIGHT);
        let i = y as usize * self.out_w + x as usize;
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

    /// Output pixels `xa` to one before `xb` of row `y`, in `rgb` whole.
    fn fill(&mut self, y: i32, xa: i32, xb: i32, rgb: u32) {
        let f = self.frame;
        let Some(&fy) = f.rows.get((y - f.y) as usize) else { return };
        if fy < self.clip.y || fy >= self.clip.y + self.clip.h {
            return;
        }
        // (A frame without depth keys hides nothing.)
        let depth = self.depth.get(fy as usize * WIDTH..(fy as usize + 1) * WIDTH);
        let row = y as usize * self.out_w;
        let (xa, xb) = (xa.max(f.x), xb.min(f.x + f.columns.len() as i32));
        let (clip0, clip1) = (self.clip.x, self.clip.x + self.clip.w);
        for (x, &fx) in (xa..xb).zip(&f.columns[(xa - f.x).max(0) as usize..]) {
            if fx >= clip0 && fx < clip1 && depth.is_none_or(|d| d[fx as usize] >= self.key)
                && let Some(o) = self.out.get_mut(row + x as usize)
            {
                *o = rgb;
            }
        }
    }
}

/// A target in a color: what a glyph's field paints.
struct Pen<'t, 'o> {
    target: &'t mut Target<'o>,
    rgb: u32,
}

impl Paint for Pen<'_, '_> {
    fn pixel(&mut self, x: i32, y: i32, a: u8) {
        self.target.blend(x, y, self.rgb, a);
    }

    fn span(&mut self, y: i32, xa: i32, xb: i32) {
        self.target.fill(y, xa, xb, self.rgb);
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
        let frame = crate::render::Frame { pixels: vec![0; WIDTH * HEIGHT], depth, text: vec![item], glide: None, ..crate::render::Frame::default() };
        let (w, h) = (WIDTH * 2, HEIGHT * 2);
        let mut out = vec![0u32; w * h];
        crate::present::present(&frame, Some(&mut renderer()), &mut out, w, h);
        (0..w).map(|x| (0..32).any(|y| out[y * w + x] != 0)).collect()
    }

    /// The first and last output rows an item lights, white with a shadow
    /// over a black frame, at `scale` times the frame.
    fn lit_rows(item: TextItem, scale: usize) -> (usize, usize) {
        let frame = crate::render::Frame { pixels: vec![0; WIDTH * HEIGHT], depth: vec![u32::MAX; WIDTH * HEIGHT], text: vec![item], glide: None, ..crate::render::Frame::default() };
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
    /// and kanji start under their first. What reaches deeper than g (the
    /// cedilla, the low line, a kanji's foot) ends there too.
    #[test]
    fn a_fitted_cells_descenders_end_in_its_cells() {
        let at = Rect::new(16, 8, 64, 16);
        for scale in 1..=6 {
            let mut deepest = 0;
            for text in ["gjpqy", "Spreadr1", "BigBomb", "(Ⓐ),;", "攻撃熱斗", "ガンデルソル3", "バッドメディスン", "Éfl", "Çŋ_‗", "妹舞＿"] {
                let (first, last) = lit_rows(TextItem::new(text, Role::FittedCell, at, 0x7FFF, Some(0x2108)), scale);
                assert!(first >= at.y as usize * scale, "{text} at {scale}x: from output row {first}");
                assert!(last < (at.y + at.h) as usize * scale, "{text} at {scale}x: to output row {last}");
                deepest = deepest.max(last);
            }
            // (Not fitted further than it needs: the last row is reached.)
            assert!(deepest + 2 * scale > (at.y + at.h) as usize * scale, "{scale}x: to output row {deepest}");
        }
    }

    /// A fitted cell string stands where a cell string does, the
    /// original's: its capitals are the cell role's, the same pixels in
    /// rows 4 to 13 of the cells on the baseline at 14, at every scale.
    /// Only what is under the baseline differs: its descenders take one
    /// row (14) and their shadow the next (15), as the original's do,
    /// where the cell role's reach 3.2 rows.
    #[test]
    fn a_fitted_cells_capitals_stand_on_the_originals_baseline() {
        let at = Rect::new(16, 8, 64, 16);
        let baseline = at.y as usize + 14;
        for scale in 1..=6 {
            let shot = |text: &str, role: Role| {
                let item = TextItem::new(text, role, at, 0x7FFF, Some(0x2108));
                let frame = crate::render::Frame { pixels: vec![0; WIDTH * HEIGHT], depth: vec![u32::MAX; WIDTH * HEIGHT], text: vec![item], glide: None, ..crate::render::Frame::default() };
                let (w, h) = (WIDTH * scale, HEIGHT * scale);
                let mut out = vec![0u32; w * h];
                crate::present::present(&frame, Some(&mut renderer()), &mut out, w, h);
                out
            };
            // The capitals: the cell role's pixels, shadow and all (those
            // that reach no lower than the baseline: a round capital's
            // overshoot under it is shortened too).
            for text in ["HIEML", "TNKZ1", "AVFWX"] {
                assert!(shot(text, Role::FittedCell) == shot(text, Role::Cell), "{text} at {scale}x");
            }
            let rows = |text: &str, role: Role| lit_rows(TextItem::new(text, role, at, 0x7FFF, None), scale);
            let (top, bottom) = rows("HIEML", Role::FittedCell);
            assert_eq!((top, bottom + 1), ((at.y as usize + 4) * scale, baseline * scale), "{scale}x: the capitals' rows");
            // The descenders: one row under the baseline, where the cell
            // role's go three and more.
            let (_, low) = rows("gjpqy", Role::FittedCell);
            assert!(low >= baseline * scale && low < (baseline + 1) * scale, "{scale}x: descenders to output row {low}");
            let (_, whole) = rows("gjpqy", Role::Cell);
            assert!(whole >= (baseline + 3) * scale, "{scale}x: the cell role's to output row {whole}");
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
            let face = r.face(FontId::Text, Some(st.weight)).expect("Murecho has kana");
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

    /// A glyph's field draws what its outline covers: at 5x and 13x, Latin,
    /// kana and kanji (Murecho's e and kanji are overlapping contours),
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
