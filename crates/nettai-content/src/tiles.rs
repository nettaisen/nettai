//! 8x8 tiles laid out in an image, and back.
//!
//! A tile block is stored as an indexed image in which each tile sits at a
//! fixed place given by a [`Layout`], so that the image looks like what the
//! tiles draw: a grid for background tiles, 8x16 glyphs for fonts, 2x2
//! icons, 4x2 mugshots.

use crate::image::Indexed;
use nettai_assets::Tiles;
use serde::{Deserialize, Serialize};

/// Where tile `i` of a block goes in its image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// Tiles left to right, `columns` a row.
    Grid { columns: u32 },
    /// Blocks of `width` x `height` tiles (tiles within a block row by
    /// row, as a sprite of that size reads them), `columns` blocks a row.
    Blocks { width: u32, height: u32, columns: u32 },
}

impl Layout {
    /// The top-left pixel of tile `i`.
    pub fn place(self, i: u32) -> (u32, u32) {
        match self {
            Layout::Grid { columns } => ((i % columns) * 8, (i / columns) * 8),
            Layout::Blocks { width, height, columns } => {
                let per = width * height;
                let (b, j) = (i / per, i % per);
                let (bx, by) = (b % columns, b / columns);
                ((bx * width + j % width) * 8, (by * height + j / width) * 8)
            }
        }
    }

    /// Image size for `n` tiles.
    pub fn size(self, n: u32) -> (u32, u32) {
        match self {
            Layout::Grid { columns } => (columns * 8, n.div_ceil(columns).max(1) * 8),
            Layout::Blocks { width, height, columns } => {
                let blocks = n.div_ceil(width * height).max(1);
                (columns.min(blocks) * width * 8, blocks.div_ceil(columns) * height * 8)
            }
        }
    }
}

/// Draw tiles into an image: tile `i` at `layout.place(i)` in palette row
/// `row(i)`.
pub fn draw(img: &mut Indexed, tiles: &Tiles, layout: Layout, row: impl Fn(usize) -> u8) {
    for i in 0..tiles.len() {
        let (x0, y0) = layout.place(i as u32);
        let t = tiles.get(i).unwrap();
        let r = row(i) << 4;
        for (k, &v) in t.iter().enumerate() {
            let px = if v == 0 { r } else { r | v };
            img.set(x0 + (k % 8) as u32, y0 + (k / 8) as u32, px);
        }
    }
}

/// An image of `tiles`, `palette` its PNG palette.
pub fn image(tiles: &Tiles, layout: Layout, palette: Vec<[u8; 3]>, row: impl Fn(usize) -> u8) -> Indexed {
    let (w, h) = layout.size(tiles.len() as u32);
    let mut img = Indexed::new(w, h, palette);
    draw(&mut img, tiles, layout, row);
    img
}

/// What reading tiles back from an image found wrong.
#[derive(Debug, Default)]
pub struct ReadProblems {
    /// Tiles whose pixels use more than one palette row (only the index
    /// within a row is kept).
    pub mixed_rows: Vec<usize>,
    /// The image is too small for the tiles.
    pub too_small: bool,
}

/// Read `n` tiles back: each pixel's index within its palette row.
pub fn read(img: &Indexed, n: usize, layout: Layout) -> (Tiles, ReadProblems) {
    let mut out = Tiles { pixels: Vec::with_capacity(n * Tiles::TILE) };
    let mut problems = ReadProblems::default();
    let (w, h) = layout.size(n as u32);
    if img.width < w || img.height < h {
        problems.too_small = true;
        out.pixels.resize(n * Tiles::TILE, 0);
        return (out, problems);
    }
    for i in 0..n {
        let (x0, y0) = layout.place(i as u32);
        let mut row = None;
        for k in 0..64u32 {
            let v = img.get(x0 + k % 8, y0 + k / 8);
            if v & 15 != 0 {
                match row {
                    None => row = Some(v >> 4),
                    Some(r) if r != v >> 4
                        && problems.mixed_rows.last() != Some(&i) => {
                            problems.mixed_rows.push(i);
                        }
                    _ => {}
                }
            }
            out.pixels.push(v & 15);
        }
    }
    (out, problems)
}

/// A tile block stored as an image, as a JSON document describes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileImage {
    pub file: String,
    pub layout: Layout,
    pub tiles: usize,
    /// Rows of the image's palette that are data: `[first, count]`. The
    /// other rows only color the image for viewing; edits there are
    /// ignored.
    #[serde(default)]
    pub palettes: [usize; 2],
    /// The palette as exported (see [`crate::image::palette_fingerprint`]).
    pub fingerprint: String,
}

/// An image of `tiles` with palette rows `rows` (BGR555), of which
/// `owned` = [first, count] are data; tile `i` shows in row `row_of(i)`.
pub fn export_image(
    file: &str,
    tiles: &Tiles,
    layout: Layout,
    rows: &[nettai_assets::Palette],
    owned: [usize; 2],
    row_of: impl Fn(usize) -> u8,
) -> (Vec<u8>, TileImage) {
    let palette = crate::image::palette_rgb(rows);
    let img = image(tiles, layout, palette.clone(), row_of);
    let doc = TileImage {
        file: file.into(),
        layout,
        tiles: tiles.len(),
        palettes: owned,
        fingerprint: crate::image::palette_fingerprint(&palette),
    };
    (img.to_png(), doc)
}

/// Read a tile image back: its tiles and its data palettes.
pub fn import_image(
    dir: &std::path::Path,
    prefix: &str,
    doc: &TileImage,
    report: &mut crate::report::Report,
) -> Option<(Tiles, Vec<nettai_assets::Palette>)> {
    let name = format!("{prefix}/{}", doc.file);
    let img = match Indexed::load(&dir.join(&doc.file)) {
        Ok(i) => i,
        Err(e) => {
            report.error(&name, e);
            return None;
        }
    };
    let [first, count] = doc.palettes;
    let mut palettes = Vec::new();
    if count > 0 {
        if let Some(why) = crate::image::palette_change(&doc.fingerprint, &img.palette, (first + count) * 16) {
            report.error(&name, why);
            return None;
        }
        let (rows, off_grid) = crate::image::palette_rows(&img.palette, first + count);
        if !off_grid.iter().all(|&i| i < first * 16) {
            report.warn(&name, "some palette colors aren't GBA colors (5 bits a channel) and were rounded");
        }
        palettes = rows[first..].to_vec();
    } else if crate::image::palette_fingerprint(&img.palette) != doc.fingerprint {
        report.note(&name, "this image's palette is only for viewing; color edits here are ignored");
    }
    let (t, problems) = read(&img, doc.tiles, doc.layout);
    if problems.too_small {
        report.error(&name, format!("the image is smaller than {} tiles in its layout need", doc.tiles));
        return None;
    }
    if !problems.mixed_rows.is_empty() {
        report.warn(
            &name,
            format!(
                "tiles {:?} use colors from more than one palette row; only each color's place in its row is kept",
                &problems.mixed_rows[..problems.mixed_rows.len().min(8)]
            ),
        );
    }
    Some((t, palettes))
}

/// A map entry as text: `tile:palette`, then `:h`, `:v` or `:hv` if flipped.
pub fn entry_text(e: &nettai_assets::MapEntry) -> String {
    let flip = match (e.hflip, e.vflip) {
        (false, false) => "",
        (true, false) => ":h",
        (false, true) => ":v",
        (true, true) => ":hv",
    };
    format!("{}:{}{flip}", e.tile, e.palette)
}

pub fn parse_entry(s: &str) -> Result<nettai_assets::MapEntry, String> {
    let mut it = s.split(':');
    let bad = || format!("{s:?} isn't a map entry (tile:palette[:h|:v|:hv])");
    let tile: u16 = it.next().and_then(|t| t.trim().parse().ok()).ok_or_else(bad)?;
    let palette: u8 = it.next().and_then(|t| t.trim().parse().ok()).ok_or_else(bad)?;
    let (hflip, vflip) = match it.next().map(str::trim) {
        None | Some("") => (false, false),
        Some("h") => (true, false),
        Some("v") => (false, true),
        Some("hv") | Some("vh") => (true, true),
        Some(_) => return Err(bad()),
    };
    if tile > 0x3FF || palette > 15 {
        return Err(format!("{s:?}: tiles go up to 1023 and palettes to 15"));
    }
    Ok(nettai_assets::MapEntry { tile, hflip, vflip, palette })
}

/// A color as text: `#rrggbb`, or `0xNNNN` (raw BGR555) when it has bits
/// an RGB value can't hold.
pub fn color_text(c: u16) -> String {
    if c & 0x8000 != 0 {
        return format!("{c:#06x}");
    }
    let [r, g, b] = crate::image::rgb(c);
    format!("#{r:02x}{g:02x}{b:02x}")
}

pub fn parse_color(s: &str) -> Result<(u16, bool), String> {
    if let Some(hex) = s.strip_prefix("0x") {
        return u16::from_str_radix(hex, 16).map(|v| (v, true)).map_err(|_| format!("{s:?} isn't a color"));
    }
    let hex = s.strip_prefix('#').filter(|h| h.len() == 6).ok_or_else(|| format!("{s:?} isn't a #rrggbb color"))?;
    let v = u32::from_str_radix(hex, 16).map_err(|_| format!("{s:?} isn't a #rrggbb color"))?;
    Ok(crate::image::bgr555([(v >> 16) as u8, (v >> 8) as u8, v as u8]))
}

pub fn palette_text(p: &nettai_assets::Palette) -> Vec<String> {
    p.iter().map(|&c| color_text(c)).collect()
}

pub fn parse_palette(v: &[String], report: &mut crate::report::Report, file: &str) -> nettai_assets::Palette {
    let mut p = [0u16; 16];
    if v.len() != 16 {
        report.error(file, format!("a palette has 16 colors, not {}", v.len()));
    }
    for (i, s) in v.iter().enumerate().take(16) {
        match parse_color(s) {
            Ok((c, exact)) => {
                if !exact {
                    report.warn(file, format!("{s} isn't a GBA color (5 bits a channel); rounded"));
                }
                p[i] = c;
            }
            Err(e) => report.error(file, e),
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_entries_and_colors_as_text() {
        let e = nettai_assets::MapEntry { tile: 700, hflip: true, vflip: true, palette: 5 };
        assert_eq!(entry_text(&e), "700:5:hv");
        assert_eq!(parse_entry("700:5:hv"), Ok(e));
        assert!(parse_entry("7:16").is_err());
        for c in [0u16, 0x7FFF, 0x1234, 0x8001] {
            assert_eq!(parse_color(&color_text(c)), Ok((c, true)));
        }
    }

    #[test]
    fn layouts_place_tiles() {
        let icons = Layout::Blocks { width: 2, height: 2, columns: 3 };
        assert_eq!(icons.place(0), (0, 0));
        assert_eq!(icons.place(3), (8, 8));
        assert_eq!(icons.place(4), (16, 0));
        assert_eq!(icons.place(12), (0, 16));
        assert_eq!(icons.size(13), (48, 32));
        assert_eq!(Layout::Grid { columns: 16 }.size(17), (128, 16));
    }

    #[test]
    fn tiles_round_trip_through_an_image() {
        let tiles = Tiles { pixels: (0..5 * 64).map(|i| (i * 7 % 16) as u8).collect() };
        let layout = Layout::Blocks { width: 1, height: 2, columns: 2 };
        let img = image(&tiles, layout, vec![[0; 3]; 48], |i| (i % 3) as u8);
        let (back, problems) = read(&img, 5, layout);
        assert_eq!(back, tiles);
        assert!(problems.mixed_rows.is_empty() && !problems.too_small);
    }
}
