//! Reading the battle's frames as shapes as a pack is made
//! (`nettai_assets::shapes`, docs/design/text-rendering.md §13): the HUD's
//! HP box, its gauge's frame and cells, the emotion window's boxes and its
//! frame apart from the faces, the enemy names' bar, and the custom
//! window's frame (each map's), its column's cells, its slots' boxes and
//! its cursors, and the banners' bars. Each picture is traced and its
//! boundaries fitted with the shapes they rasterize ([`trace`], [`fit`]) as the screen
//! lays it: a map's tiles as its entries place and flip them, with the
//! cells something else is laid in (a patch, a button, the column, the
//! names) holes in it, whose edges stay exact; the pictures with letters
//! off where the enhanced mode draws their words.
//!
//! Nothing here is a game's: the packs' pictures, maps and layouts say what
//! is drawn where. What can't be read as every game has it is said.

mod fit;
mod trace;

use trace::{Grid, Label, Region, trace};
use nettai_assets::shapes::{CustomShapes, Drawing, GaugeCells, HudShapes, MapDrawing, MugshotFrame};
use nettai_assets::{Bundle, CustomScreen, Hud, MapEntry, PatchList, Tiles};

/// A picture being laid: each pixel's label.
struct Canvas {
    w: usize,
    h: usize,
    cells: Vec<Label>,
}

impl Canvas {
    fn new(w: usize, h: usize, fill: Label) -> Canvas {
        Canvas { w, h, cells: vec![fill; w * h] }
    }

    /// An 8x8 tile at (x, y), flipped as `e` says, in palette row `palette`.
    fn tile(&mut self, t: &[u8], x: usize, y: usize, hflip: bool, vflip: bool, palette: u8) {
        for (i, &v) in t.iter().enumerate().take(64) {
            let (tx, ty) = (i % 8, i / 8);
            let (dx, dy) = (if hflip { 7 - tx } else { tx }, if vflip { 7 - ty } else { ty });
            let (px, py) = (x + dx, y + dy);
            if px < self.w && py < self.h {
                self.cells[py * self.w + px] = if v & 15 == 0 { Label::Clear } else { Label::Color(palette, v & 15) };
            }
        }
    }

    /// Cell (column, row) a hole.
    fn hole(&mut self, col: usize, row: usize) {
        for y in 8 * row..(8 * row + 8).min(self.h) {
            for x in 8 * col..(8 * col + 8).min(self.w) {
                self.cells[y * self.w + x] = Label::Hole;
            }
        }
    }

    fn grid(self, outside: [Label; 4]) -> Grid {
        Grid { w: self.w, h: self.h, cells: self.cells, outside }
    }
}

const CLEAR: [Label; 4] = [Label::Clear; 4];
const HOLES: [Label; 4] = [Label::Hole; 4];

/// A traced picture as a drawing: its regions' paths in the drawing's
/// unit.
fn drawing(w: usize, h: usize, regions: Vec<Region>) -> Drawing {
    use nettai_assets::shapes::Step;
    let unit = Drawing::UNIT as f64;
    let at = |p: [f64; 2]| [(p[0] * unit).round() as i32, (p[1] * unit).round() as i32];
    Drawing {
        size: [w as u16, h as u16],
        regions: regions
            .into_iter()
            .map(|r| {
                let mut path = Vec::new();
                for ring in &r.rings {
                    path.push(Step::Move(at(ring.start)));
                    for s in &ring.segs {
                        path.push(match *s {
                            fit::Seg::Line(p) => Step::Line(at(p)),
                            fit::Seg::Arc { to, radius, sweep, large } => Step::Arc { radius: (radius * unit).round() as i32, large, sweep, to: at(to) },
                        });
                    }
                    path.push(Step::Close);
                }
                nettai_assets::shapes::Region { palette: r.palette, color: r.color, path }
            })
            .collect(),
    }
}

/// Tiles laid `columns` a row from tile `first`, traced, with `outside`
/// past its sides; none where it draws nothing.
fn tiles_drawing(name: &str, tiles: &Tiles, first: usize, columns: usize, rows: usize, outside: [Label; 4]) -> Option<Drawing> {
    let mut c = Canvas::new(8 * columns, 8 * rows, Label::Clear);
    for k in 0..columns * rows {
        let t = tiles.get(first + k)?;
        c.tile(t, 8 * (k % columns), 8 * (k / columns), false, false, 0);
    }
    shaped(name, c.grid(outside))
}

/// A picture traced and fitted ([`trace`]) as a drawing; none where it
/// draws nothing. (With `NETTAI_SHAPES_DEBUG` set to a folder, each is kept
/// to be written there beside its pixels: [`write_debug`].)
fn shaped(name: &str, grid: Grid) -> Option<Drawing> {
    let regions = trace(&grid);
    if regions.is_empty() {
        return None;
    }
    let d = drawing(grid.w, grid.h, regions);
    if std::env::var_os("NETTAI_SHAPES_DEBUG").is_some()
        && let Ok(mut kept) = DEBUG.lock()
    {
        kept.push((name.to_string(), grid, d.clone()));
    }
    Some(d)
}

type Kept = Vec<(String, Grid, Drawing)>;
static DEBUG: std::sync::Mutex<Kept> = std::sync::Mutex::new(Vec::new());

/// The drawings kept ([`shaped`]) written to `NETTAI_SHAPES_DEBUG`'s folder,
/// each an SVG of its pixels (holes hatched) beside its shapes, in the
/// colors `view` gives each drawing by its name.
fn write_debug(view: &dyn Fn(&str, u8, u8) -> u16) {
    let Some(dir) = std::env::var_os("NETTAI_SHAPES_DEBUG").map(std::path::PathBuf::from) else { return };
    let _ = std::fs::create_dir_all(&dir);
    let Ok(mut kept) = DEBUG.lock() else { return };
    for (name, g, d) in kept.drain(..) {
        let color = |p: u8, c: u8| {
            let [r, g, b] = nettai_content::image::rgb(view(&name, p, c));
            format!("#{r:02x}{g:02x}{b:02x}")
        };
        let (w, h) = (g.w, g.h);
        let mut out = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"-1 -1 {} {}\">\n", (2 * w + 3) * 8, (h + 2) * 8, 2 * w + 3, h + 2);
        out += &format!("<rect x=\"-1\" y=\"-1\" width=\"{}\" height=\"{}\" fill=\"#808080\"/>\n", 2 * w + 3, h + 2);
        for y in 0..h {
            for x in 0..w {
                let fill = match g.cells[y * w + x] {
                    Label::Clear => continue,
                    Label::Hole => "#ff00ff".to_string(),
                    Label::Color(p, c) => color(p, c),
                };
                out += &format!("<rect x=\"{x}\" y=\"{y}\" width=\"1.02\" height=\"1.02\" fill=\"{fill}\"/>\n");
            }
        }
        let svg = nettai_content::shapes::to_svg(&d, &|p, c| view(&name, p, c));
        out += &format!("<g transform=\"translate({} 0)\">\n", w + 1);
        for line in svg.lines().filter(|l| l.starts_with("<path")) {
            out += line;
            out.push('\n');
        }
        out += "</g>\n";
        out += "</svg>\n";
        let _ = std::fs::write(dir.join(format!("{}.svg", name.replace('/', "-"))), out);
    }
}

/// 8x16 glyphs (tile 2k its top, 2k + 1 its bottom), `cells` side by side
/// from glyph `first`.
fn glyphs_drawing(name: &str, tiles: &Tiles, first: usize, cells: usize, outside: [Label; 4]) -> Option<Drawing> {
    let mut c = Canvas::new(8 * cells, 16, Label::Clear);
    for k in 0..cells {
        c.tile(tiles.get(2 * (first + k))?, 8 * k, 0, false, false, 0);
        c.tile(tiles.get(2 * (first + k) + 1)?, 8 * k, 8, false, false, 0);
    }
    shaped(name, c.grid(outside))
}

/// Read the frames of `bundle` as shapes into its `shapes` (the banners'
/// bars into their layouts; and the
/// emotion window's frame apart from its faces: `Hud::mugshot_frame`), and
/// say what can't be read.
pub fn read(bundle: &mut Bundle) -> Vec<String> {
    let mut problems = Vec::new();
    if !bundle.hud.tiles.is_empty() {
        // The banners' bars, in each language's pictures.
        for (i, b) in bundle.hud.banners.iter_mut().enumerate() {
            b.bar = banner_bar(&format!("banner-{i}"), b);
        }
        for (lang, l) in &mut bundle.hud.languages {
            for (i, b) in l.banners.iter_mut().enumerate() {
                if let Some(b) = b {
                    b.bar = banner_bar(&format!("banner-{i}-{lang}"), b);
                }
            }
        }
        let (shapes, frame) = hud_shapes(&bundle.hud, &mut |p| problems.push(p));
        bundle.hud.shapes = shapes;
        bundle.hud.mugshot_frame = frame;
    }
    if !bundle.custom.is_empty() {
        bundle.custom.shapes = custom_shapes(&bundle.custom, &mut |p| problems.push(p));
    }
    // (The colors each is looked at in: its picture's first palette.)
    let (h, c) = (&bundle.hud, &bundle.custom);
    write_debug(&|name, p, k| {
        let k = k as usize & 15;
        let face = h.mugshots.first().map(|m| m.1).unwrap_or_default();
        let frame = c.frame_palettes.first().copied().unwrap_or_default();
        match name {
            n if n.starts_with("banner") => h.banner_palette[k],
            n if n.starts_with("gauge") => h.gauge_palette[k],
            "hp-box" | "name-bar-0" | "name-bar-1" => h.hp_palettes[0][k],
            n if n.starts_with("count") || n.starts_with("mugshot") || n == "navi-box" => face[k],
            "empty-icon" => c.icon_palette[k],
            n if n.starts_with("cursor") => c.cursor_palette.or_else(|| c.emblems.first().map(|e| e.palette)).unwrap_or_default()[k],
            n if n.starts_with("form-list-cursor") => c.form_list_cursor_palette[k],
            _ => match p {
                11 => c.icon_palette[k],
                13 => h.hp_palettes[0][k],
                14 | 15 => c.other_palette[k],
                _ => frame[k],
            },
        }
    });
    problems
}

/// A banner's bar ([`nettai_content::letters::bar_colors`]: none where its
/// picture has none): its picture with its letters off, each letter pixel
/// the bar's color nearest it on its row, traced.
fn banner_bar(name: &str, b: &nettai_assets::BannerLayout) -> Option<Drawing> {
    use nettai_content::letters::{Picture, bar_colors, erased};
    if b.glyphs.is_empty() {
        return None;
    }
    let cells = b.glyphs.len() / 2;
    let picture = Picture::glyphs(&b.glyphs, 0, cells);
    let colors = bar_colors(&picture)?;
    let bar = erased(&picture, |v| v == 0 || colors.contains(&v));
    let mut c = Canvas::new(bar.w, bar.h, Label::Clear);
    for (i, &v) in bar.pixels.iter().enumerate() {
        c.cells[i] = if v != 0 && colors.contains(&v) { Label::Color(0, v) } else { Label::Clear };
    }
    shaped(name, c.grid(CLEAR))
}

/// The HUD's frames ([`HudShapes`]), and the emotion window's frame apart
/// from its faces.
pub fn hud_shapes(h: &Hud, say: &mut dyn FnMut(String)) -> (HudShapes, Option<MugshotFrame>) {
    let mut s = HudShapes::default();
    // A HUD layer's tile by its number, the gauge's with its letters off.
    let gauge_base = nettai_content::letters::gauge_base(&h.gauge_tiles, h.gauge_first_tile, &h.gauge_frame, &h.letters.words);
    let layer_tile = |tile: u16| -> Option<&[u8]> {
        match tile.checked_sub(h.gauge_first_tile).filter(|&i| (i as usize) < gauge_base.len()) {
            Some(i) => gauge_base.get(i as usize),
            None => h.layer_tile(tile),
        }
    };
    // The HP box, its digits' cells the box's blank glyph.
    if h.hp_box.len() == 12 {
        let mut c = Canvas::new(48, 16, Label::Clear);
        let mut ok = true;
        for (i, e) in h.hp_box.iter().enumerate() {
            let (col, row) = (i % 6, i / 6);
            let tile = if (1..5).contains(&col) { Some(h.first_tile + 2 * Hud::HP_BLANK as u16 + row as u16) } else { Some(e.tile) };
            match tile.and_then(layer_tile) {
                Some(t) => c.tile(t, 8 * col, 8 * row, e.hflip && !(1..5).contains(&col), e.vflip && !(1..5).contains(&col), 0),
                None => ok = false,
            }
        }
        if ok {
            s.hp_box = shaped("hp-box", c.grid(CLEAR));
        } else {
            say("the HP box has tiles the HUD layer lacks: it stays a picture".into());
        }
    }
    // The gauge's frame, its fill's cells holes.
    let columns = Hud::GAUGE_COLUMNS;
    if h.gauge_frame.len() == 2 * columns {
        let mut c = Canvas::new(8 * columns, 16, Label::Clear);
        let mut cells = Vec::new();
        for (i, e) in h.gauge_frame.iter().enumerate() {
            let (col, row) = (i % columns, i / columns);
            if row == Hud::GAUGE_FILL_ROW && (Hud::GAUGE_FILL_COLUMN..Hud::GAUGE_FILL_COLUMN + Hud::GAUGE_FILL_CELLS).contains(&col) {
                c.hole(col, row);
                continue;
            }
            if let Some(t) = layer_tile(e.tile) {
                c.tile(t, 8 * col, 8 * row, e.hflip, e.vflip, 0);
            }
            cells.push([col as u8, row as u8]);
        }
        s.gauge_frame = shaped("gauge-frame", c.grid(CLEAR)).map(|drawing| MapDrawing { drawing, cells });
    }
    s.gauge_cells = gauge_cells(h, &gauge_base, say);
    // The emotion window's boxes (beside the face: their left side meets
    // it).
    let beside = [Label::Clear, Label::Clear, Label::Clear, Label::Hole];
    s.count_blank = tiles_drawing("count-blank", &h.letters.counts.blank, 0, 2, 2, beside);
    s.count_box = tiles_drawing("count-box", &h.count_box, 0, 2, 2, beside);
    s.navi_box = tiles_drawing("navi-box", &h.navi_box, 0, 2, 2, beside);
    s.mugshot_boxes = h.mugshot_boxes.iter().enumerate().map(|(k, b)| tiles_drawing(&format!("mugshot-box-{k}"), b, 0, 2, 2, beside)).collect();
    if s.mugshot_boxes.iter().all(Option::is_none) {
        s.mugshot_boxes.clear();
    }
    let frame = mugshot_frame(h, say);
    if let Some(f) = &frame {
        // (The ring alone: the faces' pixels holes, so its edges against
        // them stay exact; the face's right side meets its box.)
        let mut c = Canvas::new(32, 16, Label::Clear);
        let faces: Vec<&Tiles> = h.mugshots.iter().map(|(t, _)| t).chain(h.navi_mugshots.iter().map(|m| &m.tiles)).collect();
        for k in 0..8 {
            let (x0, y0) = (8 * (k % 4), 8 * (k / 4));
            for i in 0..64 {
                let (x, y) = (x0 + i % 8, y0 + i / 8);
                let ring = f.ring.pixels[k * 64 + i] != 0;
                let drawn = faces.iter().any(|t| t.pixels.get(k * 64 + i).is_some_and(|&v| v != 0));
                c.cells[y * 32 + x] = if ring { Label::Color(0, 1) } else if drawn { Label::Hole } else { Label::Clear };
            }
        }
        s.mugshot_frame = shaped("mugshot-frame", c.grid([Label::Clear, Label::Hole, Label::Clear, Label::Clear]));
    }
    (s, frame)
}

/// The gauge's cells ([`GaugeCells`]): each traced alone, its sides exact
/// (its neighbors and the frame meet them).
fn gauge_cells(h: &Hud, base: &Tiles, say: &mut dyn FnMut(String)) -> Option<GaugeCells> {
    let cell = |name: String, k: u16| tiles_drawing(&name, base, k as usize, 1, 1, HOLES).unwrap_or_default();
    if base.len() < (Hud::GAUGE_L_OR_R + 8) as usize {
        say("the gauge has fewer tiles than its cells, its full frames and its L or R take: they stay pictures".into());
        return None;
    }
    let steps: Vec<Drawing> = (0..8).map(|k| cell(format!("gauge-step-{k}"), k)).collect();
    let filled = cell("gauge-filled".into(), Hud::GAUGE_FILLED);
    let full = (0..4).map(|k| cell(format!("gauge-full-{k}"), Hud::GAUGE_FULL + k)).collect();
    // L or R: its letters off where the pack has them.
    let l_or_r = (0..2)
        .map(|f| {
            let blank = h.letters.words.l_or_r[f].as_ref().map(|w| w.blank.clone());
            let first = (Hud::GAUGE_L_OR_R + 4 * f as u16) as usize;
            let tiles = blank.unwrap_or_else(|| Tiles { pixels: base.pixels[first * Tiles::TILE..(first + 4) * Tiles::TILE].to_vec() });
            tiles_drawing(&format!("gauge-l-or-r-{f}"), &tiles, 0, 4, 1, HOLES).unwrap_or_default()
        })
        .collect();
    // Whether each cell filled by eighths is the filled cell to that column
    // and the empty one past it.
    let (empty, full_cell) = (base.get(0)?, base.get(Hud::GAUGE_FILLED as usize)?);
    let cut = (1..8usize).all(|k| base.get(k).is_some_and(|t| (0..64).all(|i| t[i] == if i % 8 < k { full_cell[i] } else { empty[i] })));
    if !cut {
        say("the gauge's cells filled by eighths aren't its filled cell cut short: its fill ends on whole pixels".into());
    }
    Some(GaugeCells { steps, filled, full, l_or_r, cut })
}

/// The emotion window's frame apart from its faces: the pixels every face
/// shows in its frame's color (the color most of its pictures' edge pixels
/// are), where nearly all of them do; each face apart from it where all its
/// ring is that color. What a face keeps is said.
fn mugshot_frame(h: &Hud, say: &mut dyn FnMut(String)) -> Option<MugshotFrame> {
    let faces: Vec<&Tiles> = h.mugshots.iter().map(|(t, _)| t).chain(h.navi_mugshots.iter().map(|m| &m.tiles)).filter(|t| t.len() == 8).collect();
    if faces.len() < 2 {
        return None;
    }
    // (A face's pixel at (x, y) of its 32x16, tiles row by row.)
    let at = |t: &Tiles, x: usize, y: usize| t.pixels[((y / 8) * 4 + x / 8) * 64 + (y % 8) * 8 + x % 8];
    let frame_color = |t: &Tiles| -> Option<u8> {
        let mut count = [0usize; 16];
        for x in 0..32 {
            for y in [0, 15] {
                count[at(t, x, y) as usize & 15] += 1;
            }
        }
        for y in 0..16 {
            for x in [0, 31] {
                count[at(t, x, y) as usize & 15] += 1;
            }
        }
        (1..16).max_by_key(|&c| (count[c], c)).filter(|&c| count[c] > 0).map(|c| c as u8)
    };
    let colors: Vec<Option<u8>> = faces.iter().map(|t| frame_color(t)).collect();
    let mut ring = Tiles { pixels: vec![0; 8 * Tiles::TILE] };
    let mut any = false;
    for y in 0..16 {
        for x in 0..32 {
            let n = faces.iter().zip(&colors).filter(|(t, c)| c.is_some_and(|c| at(t, x, y) == c)).count();
            if n * 10 >= faces.len() * 9 {
                ring.pixels[((y / 8) * 4 + x / 8) * 64 + (y % 8) * 8 + x % 8] = 1;
                any = true;
            }
        }
    }
    if !any {
        say("the emotion window's faces share no frame: it stays in each face".into());
        return None;
    }
    let mut frame = MugshotFrame { ring, ..MugshotFrame::default() };
    let mut kept = Vec::new();
    frame.faces = h.mugshots.iter().enumerate().map(|(i, (t, _))| frame.split(t).or_else(|| {
        kept.push(format!("{i:#04x}"));
        None
    })).collect();
    frame.navi_faces = h.navi_mugshots.iter().enumerate().map(|(i, m)| frame.split(&m.tiles).or_else(|| {
        kept.push(format!("{:#04x}", nettai_assets::NAVI_MUGSHOTS as usize + i));
        None
    })).collect();
    if !kept.is_empty() {
        say(format!("note: the emotion window's faces {} cover its frame in places and keep it (drawn as their pictures)", kept.join(", ")));
    }
    Some(frame)
}

/// The custom screen's frames ([`CustomShapes`]).
pub fn custom_shapes(c: &CustomScreen, say: &mut dyn FnMut(String)) -> CustomShapes {
    let mut s = CustomShapes::default();
    let window = if c.letters.window_blank.len() == c.window_tiles.len() { &c.letters.window_blank } else { &c.window_tiles };
    s.window_maps = c.window_maps.iter().enumerate().map(|(k, m)| window_drawing(&format!("window-{k}"), c, window, m, &c.window_patches, false, say)).collect();
    s.form_list_maps = c.form_list_maps.iter().enumerate().map(|(k, m)| window_drawing(&format!("form-list-{k}"), c, window, m, &c.form_list_patches, true, say)).collect();
    // The column's cells, both looks; an empty slot's box; a code's plate.
    s.column_cells = (0..2).filter_map(|k| glyphs_drawing(&format!("column-cell-{k}"), &c.column_cells, k, 1, HOLES)).collect();
    s.empty_icon = tiles_drawing("empty-icon", &c.empty_icon, 0, 2, 2, HOLES);
    s.slot_code = slot_code(c, say);
    // The cursors, their corners alone (the chip cursor's) or meeting each
    // other (the form list's).
    s.cursor = (0..c.cursor.len()).filter_map(|k| tiles_drawing(&format!("cursor-{k}"), &c.cursor, k, 1, 1, CLEAR)).collect();
    s.form_list_cursor = (0..c.form_list_cursor.len()).filter_map(|k| tiles_drawing(&format!("form-list-cursor-{k}"), &c.form_list_cursor, k, 1, 1, HOLES)).collect();
    // The enemy names' bar: its end (the bar's cells right of it), its
    // cell (more of them either side).
    if c.name_bar.len() >= 4 {
        let end = glyphs_drawing("name-bar-0", &c.name_bar, 0, 1, [Label::Clear, Label::Hole, Label::Clear, Label::Clear]);
        let cell = glyphs_drawing("name-bar-1", &c.name_bar, 1, 1, [Label::Clear, Label::Hole, Label::Clear, Label::Hole]);
        if let (Some(end), Some(cell)) = (end, cell) {
            s.name_bar = vec![end, cell];
        }
    }
    s
}

/// A slot's code's plate: the blank of every lettered code (A to Z, `*`),
/// which must be one; none if they differ.
fn slot_code(c: &CustomScreen, say: &mut dyn FnMut(String)) -> Option<Drawing> {
    let set = &c.letters.slot_codes;
    let blanks: Vec<&[u8]> = (0..27).filter(|&k| set.glyphs.get(k).is_some_and(Option::is_some)).filter_map(|k| set.blank.pixels.get(2 * k * Tiles::TILE..(2 * k + 2) * Tiles::TILE)).collect();
    let first = *blanks.first()?;
    if blanks.iter().any(|b| *b != first) {
        say("the slots' codes' plates differ with their letters off: they stay pictures".into());
        return None;
    }
    tiles_drawing("slot-code", &Tiles { pixels: first.to_vec() }, 0, 2, 1, HOLES)
}

/// A window map's frame: its own tiles (the window's, letters off), each
/// cell something else is laid in a hole: the patches, the column's cells,
/// the form list's names (`form_list`), a button at its own place, the
/// screen's emblem, OK (the cells its cursor's corners take half of or
/// more), and any tile not the window's.
fn window_drawing(name: &str, c: &CustomScreen, window: &Tiles, map: &[MapEntry], patches: &PatchList, form_list: bool, say: &mut dyn FnMut(String)) -> Option<MapDrawing> {
    const COLUMNS: usize = 15;
    const ROWS: usize = 20;
    if map.len() != COLUMNS * ROWS {
        say(format!("a window map has {} cells, not 15x20: it stays a picture", map.len()));
        return None;
    }
    let mut hole = vec![false; COLUMNS * ROWS];
    let mark = |x: usize, y: usize, hole: &mut Vec<bool>| {
        if x < COLUMNS && y < ROWS {
            hole[y * COLUMNS + x] = true;
        }
    };
    for p in &patches.patches {
        for y in p.y as usize..(p.y + p.height) as usize {
            for x in p.x as usize..(p.x + p.width) as usize {
                mark(x, y, &mut hole);
            }
        }
    }
    let l = &c.layout;
    for i in 0..5 {
        for &x in &l.column_columns {
            mark(x as usize, l.column_row as usize + 2 * i, &mut hole);
            mark(x as usize, l.column_row as usize + 2 * i + 1, &mut hole);
        }
    }
    if form_list {
        let [x0, y0] = l.form_names_at.map(usize::from);
        for slot in 0..5 {
            for i in 0..CustomScreen::FORM_NAME_TILES {
                mark(x0 + i % CustomScreen::FORM_NAME_COLUMNS, y0 + 2 * slot + i / CustomScreen::FORM_NAME_COLUMNS, &mut hole);
            }
        }
    }
    let buttons = c.buttons.iter().map(|(_, b)| b).chain(c.versioned.versions.iter().flat_map(|(_, v)| v.buttons.iter().map(|(_, b)| b)));
    for b in buttons.chain(c.versioned.base.buttons.iter().map(|(_, b)| b)) {
        if let Some(p) = b.place {
            for y in p.y as usize..p.y as usize + b.height as usize {
                for x in p.x as usize..p.x as usize + b.width as usize {
                    mark(x, y, &mut hole);
                }
            }
        }
    }
    if let Some(e) = &c.window_emblem {
        for y in e.y as usize..(e.y + e.height) as usize {
            for x in e.x as usize..(e.x + e.width) as usize {
                mark(x, y, &mut hole);
            }
        }
    }
    // OK: the cells its cursor's corners (8x8 each, the place less 3) take
    // half of or more.
    let o = &l.ok_cursor;
    let corners = o.corners[0];
    let (x0, x1) = (corners.iter().map(|k| o.x as i32 - 3 + k.1 as i32).min()?, corners.iter().map(|k| o.x as i32 - 3 + k.1 as i32 + 8).max()?);
    let (y0, y1) = (corners.iter().map(|k| o.y as i32 - 3 + k.0 as i32).min()?, corners.iter().map(|k| o.y as i32 - 3 + k.0 as i32 + 8).max()?);
    for y in 0..ROWS {
        for x in 0..COLUMNS {
            let over = |a0: i32, a1: i32, c0: i32| (a1.min(c0 + 8) - a0.max(c0)).max(0);
            if over(x0, x1, 8 * x as i32) >= 4 && over(y0, y1, 8 * y as i32) >= 4 {
                mark(x, y, &mut hole);
            }
        }
    }
    let mut canvas = Canvas::new(8 * COLUMNS, 8 * ROWS, Label::Clear);
    let mut cells = Vec::new();
    for (i, e) in map.iter().enumerate() {
        let (x, y) = (i % COLUMNS, i / COLUMNS);
        let own = e.tile.checked_sub(CustomScreen::WINDOW_TILE).and_then(|k| window.get(k as usize));
        match own {
            Some(t) if !hole[i] => {
                canvas.tile(t, 8 * x, 8 * y, e.hflip, e.vflip, e.palette & 15);
                cells.push([x as u8, y as u8]);
            }
            _ => canvas.hole(x, y),
        }
    }
    shaped(name, canvas.grid(CLEAR)).map(|drawing| MapDrawing { drawing, cells })
}
