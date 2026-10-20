//! The battle's frames as filled shapes, for the enhanced graphics to draw
//! at the output's resolution (docs/design/text-rendering.md §13): the
//! banners' bars, the HUD's boxes and gauge, the emotion window's frame,
//! the enemy names' bar, the custom window's frame, its column's cells, its
//! slots' boxes and its cursors. Each is fitted once from its picture as a
//! pack is made (nettai-extract) and kept in the pack as an SVG file beside
//! it. A frontend fills the shapes in the palette the picture is drawn in
//! at the time, where the picture would be; a picture with no shapes here
//! is drawn as it is.

use crate::Tiles;

/// A picture as shapes: its size in frame pixels, and its regions, one
/// color each, which meet with no gap and no overlap (a partition of what
/// the picture draws).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drawing {
    pub size: [u16; 2],
    pub regions: Vec<Region>,
}

impl Drawing {
    /// Its points' unit, over a frame pixel (their coordinates are whole
    /// numbers of it).
    pub const UNIT: f32 = 64.0;
}

/// A region of a drawing: its color (a palette index; of the palette row
/// `palette` where the picture is laid in several, a map's) and its
/// outline, a path in `Drawing::UNIT`s of a frame pixel from the
/// drawing's top left, filled even-odd.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Region {
    pub palette: u8,
    pub color: u8,
    pub path: Vec<Step>,
}

/// A step of a path, as SVG's: to a point, a straight line to it, an arc of
/// a circle to it (its radius, whether it is the larger of the two arcs,
/// whether it turns the way of increasing angles: clockwise on the screen),
/// and closing the ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Move([i32; 2]),
    Line([i32; 2]),
    Arc { radius: i32, large: bool, sweep: bool, to: [i32; 2] },
    Close,
}

/// A map's frame as shapes: the drawing (the map's top left its top left)
/// and the cells it stands in for, as (column, row): the map's own tiles
/// that nothing lays over (the patches, the runs the screen lays itself,
/// a button, the emblem are holes in it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapDrawing {
    pub drawing: Drawing,
    pub cells: Vec<[u8; 2]>,
}

impl MapDrawing {
    /// Whether it stands in for cell (column, row).
    pub fn covers(&self, column: usize, row: usize) -> bool {
        self.cells.iter().any(|c| c[0] as usize == column && c[1] as usize == row)
    }
}

/// The custom gauge's cells as shapes (8x8 each): its empty cell, its
/// cell filled by eighths (the first empty), filled, full (its frames),
/// and the "L or R" cells (their frames, 32x8 each, their letters off).
/// `cut`: each cell filled by eighths is the filled cell to that column and
/// the empty one past it, so a fill may end anywhere.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GaugeCells {
    pub steps: Vec<Drawing>,
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
}

impl CustomShapes {
    pub fn is_empty(&self) -> bool {
        *self == CustomShapes::default()
    }
}
