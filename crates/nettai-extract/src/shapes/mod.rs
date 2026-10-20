//! The battle's frames as shapes, drawn by hand (`shapes/GAME/` beside this
//! crate's sources: SVG files in frame pixels, each shape naming its
//! palette entry) and checked against the ROM's pictures as a pack is made
//! (`nettai_assets::shapes`, docs/design/text-rendering.md §13): the banners'
//! bars, the HUD's HP box, its gauge's frame and cells, the emotion window's
//! boxes and its frame apart from the faces, the enemy names' bar, and the
//! custom window's frame (each map's), its column's cells, its slots' boxes
//! and its cursors. They are the one input of extraction that isn't a
//! ROM's: their shapes are an artist's redrawing of the pictures; their
//! colors stay the ROM's palettes, at run time.
//!
//! Each picture is laid here as the screen lays it (a map's tiles as its
//! entries place and flip them, the cells something else is laid in a hole,
//! the pictures with letters off where the enhanced mode draws their
//! words), and its drawing checked: its size is the picture's, its colors
//! are entries the picture uses, and filled at the picture's pixels'
//! centers it gives about the picture's colors (each drawing's share of
//! matching pixels is noted; a drawing far off is a problem). A picture
//! with no drawing stays a picture, and that is said.

use nettai_assets::shapes::{CustomShapes, Drawing, GaugeCells, HudShapes, MapDrawing, MugshotFrame};
use nettai_assets::{Bundle, Chatbox, CustomScreen, Hud, MapEntry, PatchList, Tiles};

/// The hand-drawn SVG files (`shapes/`), by path from there.
mod drawn {
    include!(concat!(env!("OUT_DIR"), "/drawings.rs"));
}

/// What a pixel of a picture is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Label {
    /// Transparent.
    Clear,
    /// Not the picture's: something else is drawn there (a chip's picture
    /// in the window); a drawing may draw there or not.
    Hole,
    /// A color: a palette row (for a picture laid in several) and an index.
    Color(u8, u8),
}

/// A picture as laid: its labels row by row.
#[derive(Clone, Debug)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<Label>,
}

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

    /// An 8x8 tile at (x, y), flipped, in palette row `palette`.
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

    fn grid(self) -> Grid {
        Grid { w: self.w, h: self.h, cells: self.cells }
    }
}

/// A drawing's share of a picture's pixels it must give at their centers,
/// under which it is a problem.
const MATCH: f32 = 0.85;

/// The drawings of one game read and checked, and what was found.
pub struct Shaper {
    game: String,
    views: Views,
    notes: Vec<String>,
    /// (With `NETTAI_SHAPES_SOURCES` set to a folder: each picture written
    /// there as an SVG of its pixels, with its drawing beside where it has
    /// one, to draw from and look at.)
    sources: Option<std::path::PathBuf>,
    kept: Vec<(String, Grid, Option<Drawing>)>,
}

impl Shaper {
    pub fn new(game: &str, views: Views) -> Shaper {
        let sources = std::env::var_os("NETTAI_SHAPES_SOURCES").map(std::path::PathBuf::from);
        Shaper { game: game.to_string(), views, notes: Vec::new(), sources, kept: Vec::new() }
    }

    /// The hand-drawn file of `name` for this game (`GAME/NAME.svg`), else
    /// one another game's file says is the same (`GAME/NAME.same`, naming
    /// it).
    fn file(&self, name: &str) -> Option<(String, &'static str)> {
        let own = format!("{}/{name}", self.game);
        let find = |path: &str| drawn::DRAWINGS.iter().find(|(p, _)| *p == path).map(|(_, t)| *t);
        if let Some(t) = find(&format!("{own}.svg")) {
            return Some((format!("{own}.svg"), t));
        }
        let same = find(&format!("{own}.same"))?.trim().to_string();
        find(&same).map(|t| (same.clone(), t))
    }

    /// `name`'s drawing read, its size and colors checked against its
    /// picture `grid` (none: it has none, or it isn't the picture's).
    fn read(&mut self, name: &str, grid: &Grid) -> Option<Drawing> {
        let (path, text) = match self.file(name) {
            Some(f) => f,
            None => {
                self.notes.push(format!("no hand-drawn {name}: it stays a picture"));
                return None;
            }
        };
        // (Its fills its colors in the picture's first palette, as the pack
        // keeps it, and read so.)
        let colored = nettai_content::shapes::recolor(text, &|p, k| self.views.color(name, p, k));
        let d = match nettai_content::shapes::from_svg(text).and_then(|_| nettai_content::shapes::from_svg(&colored)) {
            Ok(d) => d,
            Err(e) => {
                self.notes.push(format!("{path}: {e}: it stays a picture"));
                return None;
            }
        };
        if d.size != [grid.w as u16, grid.h as u16] {
            self.notes.push(format!("{path}: {}x{}, the picture {}x{}: it stays a picture", d.size[0], d.size[1], grid.w, grid.h));
            return None;
        }
        let used: std::collections::BTreeSet<Label> = grid.cells.iter().copied().filter(|l| matches!(l, Label::Color(..))).collect();
        let words = d.words.iter().flat_map(|t| [Some(t.color), t.rim, t.shadow].into_iter().flatten().map(move |c| (t.palette, c)));
        for (p, c) in d.colors.iter().copied().chain(words) {
            if c == 0 || !used.contains(&Label::Color(p, c)) {
                self.notes.push(format!("{path}: color {p}:{c} isn't one the picture uses: it stays a picture"));
                return None;
            }
        }
        Some(d)
    }

    /// `name`'s drawing, read and checked against its picture `grid`
    /// (none: it has none, or it isn't the picture's): how much of the
    /// picture it gives is noted.
    fn drawing(&mut self, name: &str, grid: Grid) -> Option<Drawing> {
        let d = self.read(name, &grid);
        if let Some(d) = &d {
            let share = matching(&grid, d);
            let line = format!("{name}: {:.1}% of its pixels", 100.0 * share);
            self.notes.push(if share < MATCH { format!("{line}, far from the picture") } else { format!("note: {line}") });
        }
        if self.sources.is_some() {
            self.kept.push((name.to_string(), grid, d.clone()));
        }
        d
    }

    /// What was found, and the pictures and their drawings written where
    /// asked, each in `view`'s colors by its name.
    pub fn finish(mut self, view: &dyn Fn(&str, u8, u8) -> u16) -> Vec<String> {
        if let Some(dir) = &self.sources {
            let _ = std::fs::create_dir_all(dir);
            for (name, g, d) in self.kept.drain(..) {
                let file = name.replace('/', "-");
                let _ = std::fs::write(dir.join(format!("{file}.svg")), beside(&name, &g, d.as_ref(), view));
                // (Its pixels as text, a character each: its palette index
                // in hex, '.' clear, '#' a hole; a row a line.)
                let text: String = g
                    .cells
                    .chunks(g.w)
                    .map(|row| {
                        row.iter()
                            .map(|l| match l {
                                Label::Clear => '.',
                                Label::Hole => '#',
                                Label::Color(_, c) => char::from_digit(*c as u32, 16).unwrap_or('?'),
                            })
                            .collect::<String>()
                            + "\n"
                    })
                    .collect();
                let _ = std::fs::write(dir.join(format!("{file}.txt")), text);
            }
        }
        self.notes
    }
}

/// `d` drawn at the centers of its pixels: each one's palette entry.
fn raster(d: &Drawing) -> Vec<Label> {
    nettai_content::shapes::labels(d).into_iter().map(|l| l.map_or(Label::Clear, |(p, c)| Label::Color(p, c))).collect()
}

/// Whether pixel `at` (its center) is under words written on `d` (their
/// box and their outline's width round it, a pixel if they have none): the picture's letters
/// there aren't the drawing's to match.
fn under_words(d: &Drawing, at: [f32; 2]) -> bool {
    let u = Drawing::UNIT;
    d.words.iter().any(|t| {
        let [x, y, w, h] = t.rect.map(|v| v as f32 / u);
        // (Slanted, the box leans right going up: wider by that.)
        let lean = (t.slant as f32 / 100.0 * h).abs();
        let m = if t.rim.is_some() { t.rim_width as f32 / 100.0 + 0.5 } else { 1.0 };
        at[0] >= x - m && at[0] <= x + w + lean + m && at[1] >= y - m && at[1] <= y + h + m
    })
}

/// The share of `grid`'s pixels in `cells` (8x8 each) that `d` gives at
/// their centers (words' boxes aside).
fn matching_in(grid: &Grid, d: &Drawing, cells: &[[u8; 2]]) -> f32 {
    let got = raster(d);
    let (mut total, mut same) = (0usize, 0usize);
    for &[cx, cy] in cells {
        for i in 0..64 {
            let k = (8 * cy as usize + i / 8) * grid.w + 8 * cx as usize + i % 8;
            if under_words(d, [(k % grid.w) as f32 + 0.5, (k / grid.w) as f32 + 0.5]) {
                continue;
            }
            total += 1;
            same += (got[k] == grid.cells[k]) as usize;
        }
    }
    if total == 0 { 1.0 } else { same as f32 / total as f32 }
}

/// The share of `grid`'s pixels (holes and words' boxes aside) that `d`,
/// filled one shape over another, gives at their centers.
pub fn matching(grid: &Grid, d: &Drawing) -> f32 {
    let got = raster(d);
    let (mut total, mut same) = (0usize, 0usize);
    for y in 0..grid.h {
        for x in 0..grid.w {
            let want = grid.cells[y * grid.w + x];
            if want == Label::Hole || under_words(d, [x as f32 + 0.5, y as f32 + 0.5]) {
                continue;
            }
            total += 1;
            same += (got.get(y * grid.w + x) == Some(&want)) as usize;
        }
    }
    if total == 0 { 1.0 } else { same as f32 / total as f32 }
}

/// A picture's pixels (holes magenta) and its drawing beside, as an SVG.
fn beside(name: &str, g: &Grid, d: Option<&Drawing>, view: &dyn Fn(&str, u8, u8) -> u16) -> String {
    let color = |p: u8, c: u8| {
        let [r, gg, b] = nettai_content::image::rgb(view(name, p, c));
        format!("#{r:02x}{gg:02x}{b:02x}")
    };
    let (w, h) = (g.w, g.h);
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"-1 -1 {} {}\">\n<rect x=\"-1\" y=\"-1\" width=\"{}\" height=\"{}\" fill=\"#808080\"/>\n",
        (2 * w + 3) * 8,
        (h + 2) * 8,
        2 * w + 3,
        h + 2,
        2 * w + 3,
        h + 2
    );
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
    if let Some(d) = d {
        // (Its file's contents, in the view's colors.)
        let svg = nettai_content::shapes::recolor(&d.source, &|p, c| view(name, p, c));
        let inner = svg.find("<svg").and_then(|i| svg[i..].find('>').map(|e| i + e + 1)).zip(svg.rfind("</svg>")).map_or("", |(a, b)| &svg[a..b]);
        out += &format!("<g transform=\"translate({} 0)\">\n{inner}\n</g>\n", w + 1);
    }
    out += "</svg>\n";
    out
}

/// Tiles laid `columns` a row from tile `first`, `name`'s picture.
fn tiles_grid(tiles: &Tiles, first: usize, columns: usize, rows: usize) -> Option<Grid> {
    let mut c = Canvas::new(8 * columns, 8 * rows, Label::Clear);
    for k in 0..columns * rows {
        c.tile(tiles.get(first + k)?, 8 * (k % columns), 8 * (k / columns), false, false, 0);
    }
    Some(c.grid())
}

/// 8x16 glyphs (tile 2k its top, 2k + 1 its bottom), `cells` side by side
/// from glyph `first`.
fn glyphs_grid(tiles: &Tiles, first: usize, cells: usize) -> Option<Grid> {
    let mut c = Canvas::new(8 * cells, 16, Label::Clear);
    for k in 0..cells {
        c.tile(tiles.get(2 * (first + k))?, 8 * k, 0, false, false, 0);
        c.tile(tiles.get(2 * (first + k) + 1)?, 8 * k, 8, false, false, 0);
    }
    Some(c.grid())
}

impl Shaper {
    fn tiles(&mut self, name: &str, tiles: &Tiles, first: usize, columns: usize, rows: usize) -> Option<Drawing> {
        let g = tiles_grid(tiles, first, columns, rows)?;
        if g.cells.iter().all(|&l| l == Label::Clear) {
            return None;
        }
        self.drawing(name, g)
    }

    fn glyphs(&mut self, name: &str, tiles: &Tiles, first: usize, cells: usize) -> Option<Drawing> {
        let g = glyphs_grid(tiles, first, cells)?;
        self.drawing(name, g)
    }
}

/// Read the frames of `bundle` (of `game`, `exe6`...) as shapes into its
/// `shapes` (the banners' bars into their layouts, by their names in
/// `banner_name`; and the emotion window's frame apart from its faces:
/// `Hud::mugshot_frame`), and say what was found.
pub fn read(bundle: &mut Bundle, game: &str, banner_name: &dyn Fn(usize) -> String) -> Vec<String> {
    let mut sh = Shaper::new(game, Views::of(bundle));
    let mut problems = Vec::new();
    if !bundle.hud.tiles.is_empty() {
        // The banners' bars, in each language's pictures (a bar the same as
        // one drawn already is that one's).
        let mut done: Vec<(Grid, Option<Drawing>)> = Vec::new();
        let mut bar = |sh: &mut Shaper, name: String, b: &nettai_assets::BannerLayout| -> Option<Drawing> {
            let g = banner_grid(b)?;
            if let Some((_, d)) = done.iter().find(|(o, _)| o.cells == g.cells && o.w == g.w) {
                return d.clone();
            }
            let d = sh.drawing(&format!("banners/{name}"), g.clone());
            done.push((g, d.clone()));
            d
        };
        // (A banner with no bar its words stand on, the Program Advance's: the
        // whole picture, where it has a drawing, a logo.)
        let logo = |sh: &mut Shaper, name: String, b: &nettai_assets::BannerLayout| -> Option<Drawing> {
            let file = format!("banners/{name}");
            if b.glyphs.is_empty() || banner_grid(b).is_some() || sh.file(&file).is_none() {
                return None;
            }
            let picture = nettai_content::letters::Picture::glyphs(&b.glyphs, 0, b.glyphs.len() / 2);
            let mut c = Canvas::new(picture.w, picture.h, Label::Clear);
            for (i, &v) in picture.pixels.iter().enumerate() {
                c.cells[i] = if v != 0 { Label::Color(0, v) } else { Label::Clear };
            }
            sh.drawing(&file, c.grid())
        };
        for (i, b) in bundle.hud.banners.iter_mut().enumerate() {
            b.bar = bar(&mut sh, banner_name(i), b);
            b.logo = logo(&mut sh, banner_name(i), b);
        }
        for (lang, l) in &mut bundle.hud.languages {
            for (i, b) in l.banners.iter_mut().enumerate() {
                if let Some(b) = b {
                    b.bar = bar(&mut sh, format!("{}-{lang}", banner_name(i)), b);
                    b.logo = logo(&mut sh, format!("{}-{lang}", banner_name(i)), b);
                }
            }
        }
        let (shapes, frame) = hud_shapes(&bundle.hud, &mut sh, &mut |p| problems.push(p));
        bundle.hud.shapes = shapes;
        bundle.hud.mugshot_frame = frame;
    }
    if !bundle.custom.is_empty() {
        bundle.custom.shapes = custom_shapes(&bundle.custom, &mut sh, &mut |p| problems.push(p));
    }
    // (The colors each is looked at in: its picture's first palette.)
    let views = Views::of(bundle);
    problems.extend(sh.finish(&|name, p, k| views.color(name, p, k)));
    problems
}

/// The colors the drawings are looked at in, by their names: each
/// picture's first palette.
pub struct Views {
    banner: nettai_assets::Palette,
    gauge: nettai_assets::Palette,
    hp: nettai_assets::Palette,
    face: nettai_assets::Palette,
    navi: nettai_assets::Palette,
    icon: nettai_assets::Palette,
    cursor: nettai_assets::Palette,
    form_cursor: nettai_assets::Palette,
    picture: nettai_assets::Palette,
    form_name: nettai_assets::Palette,
    chatbox: nettai_assets::Palette,
    chatbox_text: nettai_assets::Palette,
    frame: nettai_assets::Palette,
    other: nettai_assets::Palette,
}

impl Views {
    pub fn of(bundle: &Bundle) -> Views {
        let (h, c) = (&bundle.hud, &bundle.custom);
        Views {
            banner: h.banner_palette,
            gauge: h.gauge_palette,
            hp: h.hp_palettes[0],
            face: h.mugshots.first().map(|m| m.1).unwrap_or_default(),
            navi: h.navi_mugshots.first().map(|m| m.palettes[0]).unwrap_or_default(),
            icon: c.icon_palette,
            cursor: c.cursor_palette.or_else(|| c.emblems.first().map(|e| e.palette)).unwrap_or_default(),
            form_cursor: c.form_list_cursor_palette,
            picture: c.pictures.ok.palette,
            chatbox: h.chatbox.palette,
            chatbox_text: h.chatbox.text_palette,
            form_name: c.versioned.base.form_name_palettes.first().copied().unwrap_or_default(),
            frame: c.frame_palettes.first().copied().unwrap_or_default(),
            other: c.other_palette,
        }
    }

    /// The color drawing `name` (`hud/hp-box`, `banners/battle-start`...)
    /// shows its palette entry (row `p`, index `k`) in.
    pub fn color(&self, name: &str, p: u8, k: u8) -> u16 {
        let k = k as usize & 15;
        match name {
            n if n.starts_with("banner") => self.banner[k],
            n if n.starts_with("hud/gauge") => self.gauge[k],
            "hud/hp-box" | "custom/name-bar-0" | "custom/name-bar-1" => self.hp[k],
            "hud/navi-box" => self.navi[k],
            n if n.starts_with("hud/count") || n.starts_with("hud/mugshot") => self.face[k],
            n if n.starts_with("hud/chatbox-arrow") => self.chatbox_text[k],
            n if n.starts_with("hud/chatbox") => self.chatbox[k],
            n if n.starts_with("custom/cursor") || n.starts_with("custom/regular") => self.cursor[k],
            n if n.starts_with("custom/form-list-cursor") => self.form_cursor[k],
            n if n.starts_with("custom/picture") => self.picture[k],
            "custom/form-name-bar" => self.form_name[k],
            _ => match p {
                11 => self.icon[k],
                13 => self.hp[k],
                14 | 15 => self.other[k],
                _ => self.frame[k],
            },
        }
    }
}

/// The color drawing `name` is looked at in for its palette entry (row `p`,
/// index `k`): its picture's first palette's.
pub fn view(bundle: &Bundle, name: &str, p: u8, k: u8) -> u16 {
    Views::of(bundle).color(name, p, k)
}

/// A banner's bar's picture ([`nettai_content::letters::bar_colors`]: none
/// where its picture has none): its picture with its letters off, each
/// letter pixel the bar's color nearest it on its row.
fn banner_grid(b: &nettai_assets::BannerLayout) -> Option<Grid> {
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
    Some(c.grid())
}

/// The HUD's frames ([`HudShapes`]), and the emotion window's frame apart
/// from its faces.
pub fn hud_shapes(h: &Hud, sh: &mut Shaper, say: &mut dyn FnMut(String)) -> (HudShapes, Option<MugshotFrame>) {
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
            s.hp_box = sh.drawing("hud/hp-box", c.grid());
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
        s.gauge_frame = sh.drawing("hud/gauge-frame", c.grid()).map(|drawing| MapDrawing { parts: vec![nettai_assets::shapes::MapPart { name: "gauge-frame".into(), drawing, cells }] });
    }
    s.gauge_cells = gauge_cells(h, &gauge_base, sh, say);
    // The emotion window's boxes beside the face.
    s.count_blank = sh.tiles("hud/count-blank", &h.letters.counts.blank, 0, 2, 2);
    s.count_box = sh.tiles("hud/count-box", &h.count_box, 0, 2, 2);
    s.navi_box = sh.tiles("hud/navi-box", &h.navi_box, 0, 2, 2);
    s.mugshot_boxes = h.mugshot_boxes.iter().enumerate().map(|(k, b)| sh.tiles(&format!("hud/mugshot-box-{k}"), b, 0, 2, 2)).collect();
    if s.mugshot_boxes.iter().all(Option::is_none) {
        s.mugshot_boxes.clear();
    }
    let frame = mugshot_frame(h, say);
    if let Some(f) = &frame {
        // (The ring alone, the faces' pixels holes.)
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
        s.mugshot_frame = sh.drawing("hud/mugshot-frame", c.grid());
    }
    // The chatbox's box, each of its maps: its kinds' (the message box,
    // the description box), each at its opening steps.
    let g = &h.chatbox;
    let mut maps: Vec<NamedMap> = Vec::new();
    for (kind, steps) in g.boxes.iter().enumerate() {
        for (step, map) in steps.iter().enumerate() {
            let name = format!("hud/chatbox-{kind}-{step}");
            if map.len() != Chatbox::COLUMNS * Chatbox::ROWS {
                maps.push((name, None));
                continue;
            }
            let mut c = Canvas::new(8 * Chatbox::COLUMNS, 8 * Chatbox::ROWS, Label::Clear);
            let mut cells = Vec::new();
            for (i, e) in map.iter().enumerate() {
                let (x, y) = (i % Chatbox::COLUMNS, i / Chatbox::COLUMNS);
                if let Some(t) = g.tiles.get(e.tile as usize) {
                    c.tile(t, 8 * x, 8 * y, e.hflip, e.vflip, 0);
                    cells.push([x as u8, y as u8]);
                }
            }
            maps.push((name, Some((c.grid(), cells))));
        }
    }
    if maps.iter().any(|(n, _)| sh.file(n).is_some()) {
        s.chatbox = map_drawings(&maps, sh);
    }
    // Its key-wait arrow, its frames (2x2 tiles each).
    let arrow: Vec<Option<Drawing>> = (0..g.arrow.len() / 4).map(|k| sh.tiles(&format!("hud/chatbox-arrow-{k}"), &g.arrow, 4 * k, 2, 2)).collect();
    s.chatbox_arrow = all(arrow);
    (s, frame)
}

/// The gauge's cells ([`GaugeCells`]): all of them drawn, or none.
fn gauge_cells(h: &Hud, base: &Tiles, sh: &mut Shaper, say: &mut dyn FnMut(String)) -> Option<GaugeCells> {
    if base.len() < (Hud::GAUGE_L_OR_R + 8) as usize {
        say("the gauge has fewer tiles than its cells, its full frames and its L or R take: they stay pictures".into());
        return None;
    }
    let empty = sh.tiles("hud/gauge-empty", base, 0, 1, 1);
    let filled = sh.tiles("hud/gauge-filled", base, Hud::GAUGE_FILLED as usize, 1, 1);
    let full: Vec<Option<Drawing>> = (0..4).map(|k| sh.tiles(&format!("hud/gauge-full-{k}"), base, (Hud::GAUGE_FULL + k) as usize, 1, 1)).collect();
    // L or R: its letters off where the pack has them.
    let l_or_r: Vec<Option<Drawing>> = (0..2)
        .map(|f| {
            let blank = h.letters.words.l_or_r[f].as_ref().map(|w| w.blank.clone());
            let first = (Hud::GAUGE_L_OR_R + 4 * f as u16) as usize;
            let tiles = blank.unwrap_or_else(|| Tiles { pixels: base.pixels[first * Tiles::TILE..(first + 4) * Tiles::TILE].to_vec() });
            sh.tiles(&format!("hud/gauge-l-or-r-{f}"), &tiles, 0, 4, 1)
        })
        .collect();
    // Whether each cell filled by eighths is the filled cell to that column
    // and the empty one past it.
    let (empty_tile, full_cell) = (base.get(0)?, base.get(Hud::GAUGE_FILLED as usize)?);
    let cut = (1..8usize).all(|k| base.get(k).is_some_and(|t| (0..64).all(|i| t[i] == if i % 8 < k { full_cell[i] } else { empty_tile[i] })));
    if !cut {
        say("the gauge's cells filled by eighths aren't its filled cell cut short: its fill ends on whole pixels".into());
    }
    let full: Option<Vec<Drawing>> = full.into_iter().collect();
    let l_or_r: Option<Vec<Drawing>> = l_or_r.into_iter().collect();
    Some(GaugeCells { empty: empty?, filled: filled?, full: full?, l_or_r: l_or_r?, cut })
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
pub fn custom_shapes(c: &CustomScreen, sh: &mut Shaper, say: &mut dyn FnMut(String)) -> CustomShapes {
    let mut s = CustomShapes::default();
    let window = if c.letters.window_blank.len() == c.window_tiles.len() { &c.letters.window_blank } else { &c.window_tiles };
    let mut maps: Vec<NamedMap> = Vec::new();
    for (k, m) in c.window_maps.iter().enumerate() {
        maps.push((format!("custom/window-{k}"), window_grid(c, window, m, &c.window_patches, false, say)));
    }
    for (k, m) in c.form_list_maps.iter().enumerate() {
        maps.push((format!("custom/form-list-{k}"), window_grid(c, window, m, &c.form_list_patches, true, say)));
    }
    let mut drawn = map_drawings(&maps, sh);
    s.form_list_maps = drawn.split_off(c.window_maps.len());
    s.window_maps = drawn;
    // The column's cells, both looks; an empty slot's box; a code's plate.
    let column: Vec<Option<Drawing>> = (0..2).map(|k| sh.glyphs(&format!("custom/column-cell-{k}"), &c.column_cells, k, 1)).collect();
    s.column_cells = all(column);
    s.empty_icon = sh.tiles("custom/empty-icon", &c.empty_icon, 0, 2, 2);
    s.slot_code = slot_code(c, sh, say);
    // The cursors, their corners alone (the chip cursor's) or meeting each
    // other (the form list's).
    let cursor: Vec<Option<Drawing>> = (0..c.cursor.len()).map(|k| sh.tiles(&format!("custom/cursor-{k}"), &c.cursor, k, 1, 1)).collect();
    s.cursor = all(cursor);
    let tiles: Vec<Option<Drawing>> = (0..c.form_list_cursor.len()).map(|k| sh.tiles(&format!("custom/form-list-cursor-{k}"), &c.form_list_cursor, k, 1, 1)).collect();
    s.form_list_cursor = all(tiles);
    // The buttons' looks, each set's as its drawing's file has it (as many
    // cells side by side as its width takes), while the files go on.
    for (name, b) in &c.buttons {
        let (w, h) = (b.width as usize, b.height as usize);
        if w == 0 || h == 0 {
            continue;
        }
        let mut sets = Vec::new();
        for k in 0.. {
            let file = format!("custom/button-{name}-{k}");
            let Some((_, text)) = sh.file(&file) else { break };
            let cells = nettai_content::shapes::from_svg(text).map_or(1, |d| (d.size[0] as usize / (8 * w)).max(1));
            let mut canvas = Canvas::new(8 * w * cells, 8 * h, Label::Clear);
            let mut whole = true;
            for j in 0..cells {
                for t in 0..w * h {
                    match b.tiles.get((k * cells + j) * w * h + t) {
                        Some(tile) => canvas.tile(tile, 8 * (j * w + t % w), 8 * (t / w), false, false, 0),
                        None => whole = false,
                    }
                }
            }
            if !whole {
                say(format!("{file}: its button's pictures have no such set: its looks stay pictures"));
                sets.clear();
                break;
            }
            match sh.drawing(&file, canvas.grid()) {
                Some(d) => sets.push(d),
                // (Why is said where it is read.)
                None => {
                    sets.clear();
                    break;
                }
            }
        }
        if !sets.is_empty() {
            s.buttons.push((name.clone(), sets));
        }
    }
    // The chip window's pictures with their letters off, the pack's own
    // language's and each other language's.
    let langs = std::iter::once((String::new(), &c.letters.pictures)).chain(c.languages.iter().map(|(l, x)| (format!("-{l}"), &x.letters)));
    for (suffix, p) in langs {
        let named = [("ok", p.ok.as_ref()), ("ok-picked", p.ok_picked.as_ref()), ("other", p.other.as_ref())].into_iter().map(|(n, l)| (n.to_string(), l)).chain(p.buttons.iter().map(|(n, l)| (n.clone(), l.as_ref())));
        for (name, letters) in named {
            let Some(l) = letters else { continue };
            let file = format!("custom/picture-{name}{suffix}");
            if sh.file(&file).is_none() {
                continue;
            }
            if let Some(d) = tiles_grid(&l.blank, 0, 7, 6).and_then(|g| sh.drawing(&file, g)) {
                s.pictures.push((format!("{name}{suffix}"), d));
            }
        }
    }
    // The bar the form list's names lie on (every version's, where the
    // names come apart into it, their portraits and their letters).
    if let Some((_, layers)) = nettai_content::letters::form_names_layered(c).0.first() {
        s.form_name_bar = tiles_grid(&layers.bar, 0, CustomScreen::FORM_NAME_COLUMNS, CustomScreen::FORM_NAME_TILES / CustomScreen::FORM_NAME_COLUMNS).and_then(|g| sh.drawing("custom/form-name-bar", g));
    }
    // The Regular chip's frame, its frames (32x32 each).
    let regular: Vec<Option<Drawing>> = (0..c.regular.len() / 16).map(|k| sh.tiles(&format!("custom/regular-{k}"), &c.regular, 16 * k, 4, 4)).collect();
    s.regular = all(regular);
    // (By name, as a pack keeps them.)
    s.buttons.sort_by(|a, b| a.0.cmp(&b.0));
    s.pictures.sort_by(|a, b| a.0.cmp(&b.0));
    // The enemy names' bar: its end (the bar's cells right of it), its
    // cell (more of them either side).
    if c.name_bar.len() >= 4 {
        let end = sh.glyphs("custom/name-bar-0", &c.name_bar, 0, 1);
        let cell = sh.glyphs("custom/name-bar-1", &c.name_bar, 1, 1);
        if let (Some(end), Some(cell)) = (end, cell) {
            s.name_bar = vec![end, cell];
        }
    }
    s
}

/// Every one of a set of drawings, or none (a set is drawn whole or as its
/// pictures).
fn all(set: Vec<Option<Drawing>>) -> Vec<Drawing> {
    set.into_iter().collect::<Option<Vec<_>>>().unwrap_or_default()
}

/// A slot's code's plate: the blank of every lettered code (A to Z, `*`),
/// which must be one; none if they differ.
fn slot_code(c: &CustomScreen, sh: &mut Shaper, say: &mut dyn FnMut(String)) -> Option<Drawing> {
    let set = &c.letters.slot_codes;
    let blanks: Vec<&[u8]> = (0..27).filter(|&k| set.glyphs.get(k).is_some_and(Option::is_some)).filter_map(|k| set.blank.pixels.get(2 * k * Tiles::TILE..(2 * k + 2) * Tiles::TILE)).collect();
    let first = *blanks.first()?;
    if blanks.iter().any(|b| *b != first) {
        say("the slots' codes' plates differ with their letters off: they stay pictures".into());
        return None;
    }
    sh.tiles("custom/slot-code", &Tiles { pixels: first.to_vec() }, 0, 2, 1)
}

/// A window map's frame: its own tiles (the window's, letters off), each
/// cell something else is laid in a hole: the patches, the column's cells,
/// the form list's names (`form_list`), a button at its own place, the
/// screen's emblem, OK (the cells its cursor's corners take half of or
/// more), and any tile not the window's.
fn window_grid(c: &CustomScreen, window: &Tiles, map: &[MapEntry], patches: &PatchList, form_list: bool, say: &mut dyn FnMut(String)) -> Option<(Grid, Vec<[u8; 2]>)> {
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
    Some((canvas.grid(), cells))
}

/// A map by its drawing's name: its picture and the cells of its own, if it
/// can be read as one.
type NamedMap = (String, Option<(Grid, Vec<[u8; 2]>)>);

/// The maps' frames ([`MapDrawing`]), from the drawings of maps that have
/// one: each map's cells, each of the first drawing (the map's own, then
/// the others in order) that draws something there and was drawn for the
/// same pixels there.
fn map_drawings(maps: &[NamedMap], sh: &mut Shaper) -> Vec<Option<MapDrawing>> {
    // Each map's own drawing, with its pixels at their centers.
    let drawn: Vec<Option<(Drawing, Vec<Label>)>> = maps
        .iter()
        .map(|(name, m)| {
            let (grid, _) = m.as_ref()?;
            // (A map need not have its own: another's may draw all its cells.)
            sh.file(name)?;
            let d = sh.read(name, grid)?;
            let raster = raster(&d);
            Some((d, raster))
        })
        .collect();
    maps.iter()
        .enumerate()
        .map(|(k, (name, m))| {
            let (grid, cells) = m.as_ref()?;
            // (The map's own drawing first, then the others by how many of
            // their cells are this map's: the window most like it.)
            let alike = |o: usize| -> usize {
                let Some((other, other_cells)) = &maps[o].1 else { return 0 };
                cells.iter().filter(|c| other_cells.contains(c)).filter(|&&[cx, cy]| (0..64).all(|i| {
                    let at = (8 * cy as usize + i / 8) * grid.w + 8 * cx as usize + i % 8;
                    other.cells[at] == grid.cells[at]
                })).count()
            };
            let mut others: Vec<usize> = (0..maps.len()).filter(|&o| o != k).collect();
            others.sort_by_key(|&o| (std::cmp::Reverse(alike(o)), o));
            let order: Vec<usize> = std::iter::once(k).chain(others).collect();
            let mut parts: Vec<(usize, Vec<[u8; 2]>)> = Vec::new();
            let mut missing = 0;
            let mut missed: Vec<String> = Vec::new();
            for &[cx, cy] in cells {
                let cell = |g: &Grid| -> Vec<Label> { (0..64).map(|i| g.cells[(8 * cy as usize + i / 8) * g.w + 8 * cx as usize + i % 8]).collect() };
                let want = cell(grid);
                if want.iter().all(|&l| l == Label::Clear) {
                    continue;
                }
                let found = order.iter().copied().find(|&o| {
                    let (Some((_, raster)), Some((other, _))) = (&drawn[o], &maps[o].1) else { return false };
                    let draws = (0..64).any(|i| raster[(8 * cy as usize + i / 8) * grid.w + 8 * cx as usize + i % 8] != Label::Clear);
                    draws && cell(other) == want
                });
                match found {
                    Some(o) => match parts.iter_mut().find(|p| p.0 == o) {
                        Some(p) => p.1.push([cx, cy]),
                        None => parts.push((o, vec![[cx, cy]])),
                    },
                    None => {
                        missing += 1;
                        missed.push(format!("({cx}, {cy})"));
                    }
                }
            }
            if missing > 0 {
                let more = if missed.len() > 12 { format!(" and {} more", missed.len() - 12) } else { String::new() };
                sh.notes.push(format!("{name}: {missing} cells no drawing draws stay pictures: {}{more}", missed.iter().take(12).cloned().collect::<Vec<_>>().join(" ")));
            }
            let parts: Vec<nettai_assets::shapes::MapPart> = parts
                .into_iter()
                .filter_map(|(o, cells)| {
                    let (d, _) = drawn[o].as_ref()?;
                    let share = matching_in(grid, d, &cells);
                    let line = format!("{name}: {} for {} cells: {:.1}% of their pixels", maps[o].0, cells.len(), 100.0 * share);
                    sh.notes.push(if share < MATCH { format!("{line}, far from the picture") } else { format!("note: {line}") });
                    Some(nettai_assets::shapes::MapPart { name: maps[o].0.trim_start_matches("custom/").trim_start_matches("hud/").to_string(), drawing: d.clone(), cells })
                })
                .collect();
            if sh.sources.is_some() {
                let own = drawn[k].as_ref().map(|(d, _)| d.clone());
                sh.kept.push((name.clone(), grid.clone(), own));
            }
            (!parts.is_empty()).then_some(MapDrawing { parts })
        })
        .collect()
}
