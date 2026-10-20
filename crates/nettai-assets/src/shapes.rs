//! The battle's frames as filled shapes, for the enhanced graphics to draw
//! at the output's resolution (docs/design/text-rendering.md §13): the
//! banners' bars, the HUD's boxes and gauge, the emotion window's frame,
//! the enemy names' bar, the custom window's frame, its column's cells, its
//! slots' boxes and its cursors. Each is drawn by hand from its picture
//! (nettai-extract's `shapes/`), checked against the ROM's picture as a pack
//! is made and kept in the pack as an SVG file beside it. A frontend fills
//! the shapes, one over another in order, in the palette the picture is
//! drawn in at the time, where the picture would be; a picture with no
//! shapes here is drawn as it is.

use crate::Tiles;

/// A picture as shapes: its size in frame pixels, the SVG file it is (its
/// shapes drawn one over another as SVG draws them, each naming its color
/// as a palette entry: nettai-content's `shapes`), the palette entries its
/// shapes name (palette row, index; each once, in the file's order), and
/// the words written on it (over its shapes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drawing {
    pub size: [u16; 2],
    pub source: std::sync::Arc<str>,
    pub colors: Vec<(u8, u8)>,
    pub words: Vec<Words>,
}

/// Words written on a drawing, as text in a lettering font (the picture's
/// own letters taken off): the string's key in the game's strings, the box
/// its capitals fill (x, y, width, height in `Drawing::UNIT`s of a frame
/// pixel from the drawing's top left: their top to their baseline, the run
/// fitted to the width), the face's color, and the outline's and the
/// shadow's colors where it has them (palette indices, of row `palette`),
/// its slant (hundredths of a pixel right for each pixel up: 0, upright),
/// and whether it is in the lettering font (mixed case, kana) rather than
/// the numbers' (capitals).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    pub key: String,
    pub rect: [i32; 4],
    pub palette: u8,
    pub color: u8,
    pub rim: Option<u8>,
    pub shadow: Option<u8>,
    pub slant: i16,
    pub lettering: bool,
}

impl Drawing {
    /// The words' boxes' unit, over a frame pixel (their coordinates are
    /// whole numbers of it).
    pub const UNIT: f32 = 64.0;
}

/// A map's frame as shapes: drawings (each the map's size, its top left
/// the map's), each standing in for some of its cells, as (column, row):
/// cells whose pixels the drawing was drawn for, of the map's own tiles
/// that nothing lays over (the patches, the runs the screen lays itself, a
/// button, the emblem are holes in it). One drawing may serve several maps
/// where their cells are alike (the custom window with the Cross tab and
/// without), each map's own drawing its cells that differ.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapDrawing {
    pub parts: Vec<MapPart>,
}

/// A drawing of a map's ([`MapDrawing`]): its name (its file's: the map
/// it was drawn for), the drawing, the cells it stands in for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapPart {
    pub name: String,
    pub drawing: Drawing,
    pub cells: Vec<[u8; 2]>,
}

impl MapDrawing {
    /// Whether it stands in for cell (column, row).
    pub fn covers(&self, column: usize, row: usize) -> bool {
        self.parts.iter().any(|p| p.cells.iter().any(|c| c[0] as usize == column && c[1] as usize == row))
    }
}

/// The custom gauge's cells as shapes (8x8 each): its empty cell, its
/// filled cell, full (its frames), and the "L or R" cells (their frames,
/// 32x8 each, their letters off). `cut`: each cell filled by eighths is the
/// filled cell to that column and the empty one past it, so a fill may end
/// anywhere (else the fill is drawn as its pictures).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GaugeCells {
    pub empty: Drawing,
    pub filled: Drawing,
    pub full: Vec<Drawing>,
    pub l_or_r: Vec<Drawing>,
    pub cut: bool,
}

/// The HUD's frames as shapes (none: drawn as pictures).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudShapes {
    /// The HP box (6x2 cells) with its digits' cells blank.
    pub hp_box: Option<Drawing>,
    /// The gauge's frame (18x2 cells), its letters off, the fill's cells
    /// holes in it.
    pub gauge_frame: Option<MapDrawing>,
    pub gauge_cells: Option<GaugeCells>,
    /// The emotion window's boxes beside the face (2x2 tiles each): the
    /// count's box with no digit, the box without a count, the link navis'
    /// box, and each face's own box (by mugshot; none: it has none).
    pub count_blank: Option<Drawing>,
    pub count_box: Option<Drawing>,
    pub navi_box: Option<Drawing>,
    pub mugshot_boxes: Vec<Option<Drawing>>,
    /// The emotion window's frame around the face (4x2 tiles), color 1 its
    /// ring's (each face's own color: `MugshotFrame`).
    pub mugshot_frame: Option<Drawing>,
    /// The chatbox's box for each of its maps (`Chatbox::boxes`: by kind,
    /// its opening steps), as the custom window's.
    pub chatbox: Vec<Option<MapDrawing>>,
    /// The chatbox's key-wait arrow, its frames (16x16 each).
    pub chatbox_arrow: Vec<Drawing>,
}

impl HudShapes {
    pub fn is_empty(&self) -> bool {
        *self == HudShapes::default()
    }
}

/// The emotion window's frame, apart from the faces: its ring (index 1
/// where it is, 4x2 tiles as a face), and each face's ring's color with the
/// face without it, by mugshot (the link navis' apart); none: its face
/// covers the ring in places and keeps it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MugshotFrame {
    pub ring: Tiles,
    pub faces: Vec<Option<(u8, Tiles)>>,
    pub navi_faces: Vec<Option<(u8, Tiles)>>,
}

impl MugshotFrame {
    /// Face `number` (`Hud::mugshot`'s) without its ring, and the ring's
    /// color, where it is apart.
    pub fn face(&self, number: u8) -> Option<(u8, &Tiles)> {
        let f = match number.checked_sub(crate::NAVI_MUGSHOTS) {
            Some(i) => self.navi_faces.get(i as usize),
            None => self.faces.get(number as usize),
        };
        f.and_then(|f| f.as_ref()).map(|(c, t)| (*c, t))
    }

    /// `face` with the ring in color `color` over it.
    pub fn with_ring(&self, face: &Tiles, color: u8) -> Tiles {
        let mut out = face.clone();
        for (p, &r) in out.pixels.iter_mut().zip(&self.ring.pixels) {
            if r != 0 {
                *p = color;
            }
        }
        out
    }

    /// `face` without the ring, and the ring's color, if every pixel of
    /// the ring is one color in it (none: the face covers it in places).
    pub fn split(&self, face: &Tiles) -> Option<(u8, Tiles)> {
        if face.pixels.len() != self.ring.pixels.len() {
            return None;
        }
        let mut colors = face.pixels.iter().zip(&self.ring.pixels).filter(|(_, r)| **r != 0).map(|(p, _)| *p);
        let color = colors.next()?;
        if color == 0 || colors.any(|c| c != color) {
            return None;
        }
        let mut out = face.clone();
        for (p, &r) in out.pixels.iter_mut().zip(&self.ring.pixels) {
            if r != 0 {
                *p = 0;
            }
        }
        Some((color, out))
    }
}

/// The custom screen's frames as shapes (none: drawn as pictures).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomShapes {
    /// The window's frame for each of its maps (`window_maps`), and for
    /// each of the form list window's (`form_list_maps`).
    pub window_maps: Vec<Option<MapDrawing>>,
    pub form_list_maps: Vec<Option<MapDrawing>>,
    /// The picked column's cells (empty, filled; 8x16 each, as their tiles
    /// are, unflipped).
    pub column_cells: Vec<Drawing>,
    /// An empty slot's box (16x16) and a slot's code's plate with no
    /// letter (16x8).
    pub empty_icon: Option<Drawing>,
    pub slot_code: Option<Drawing>,
    /// The cursor's corner (its frames) and the form list cursor's tiles
    /// (8x8 each, unflipped).
    pub cursor: Vec<Drawing>,
    pub form_list_cursor: Vec<Drawing>,
    /// The enemy names' bar: its slanted end, its cell (8x16 each).
    pub name_bar: Vec<Drawing>,
    /// The buttons' looks, by the name their content registers them under:
    /// each set's drawing (a cell its pictures' width and height in tiles,
    /// as many side by side as the button takes), its words on it.
    pub buttons: Vec<(String, Vec<Drawing>)>,
    /// The chip window's pictures for OK and the buttons with their
    /// letters off (7x6 tiles each), by the picture's name (`ok`,
    /// `ok-picked`, a button's), a language's own as `NAME-LANG`.
    pub pictures: Vec<(String, Drawing)>,
    /// The bar the form list's names lie on (9x2 tiles), their portraits
    /// over it kept as pictures.
    pub form_name_bar: Option<Drawing>,
    /// The Regular chip's frame (32x32), its frames.
    pub regular: Vec<Drawing>,
}

impl CustomShapes {
    pub fn is_empty(&self) -> bool {
        *self == CustomShapes::default()
    }
}
