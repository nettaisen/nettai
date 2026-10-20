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
    /// For a picture with letters in it (`crate::letters`): the image is
    /// its base, the picture with its letters taken off, and this file its
    /// letters (a layer: as large and laid out alike, each pixel where the
    /// picture differs from the base its palette index as the image's are,
    /// or the entry after the palette's last, none, elsewhere).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letters: Option<String>,
    /// For another picture of one base (another language's, another panel
    /// picture's): the image is this picture's letters (a layer, as
    /// `letters`), over the base, which this file is ([`Layered`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
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
        letters: None,
        base: None,
    };
    (img.to_png(), doc)
}

/// How a picture with letters in it is kept ([`export_layered`]): its base
/// (the picture with its letters taken off, which one image holds for every
/// picture that shares it: `base_file`, the image another picture of it
/// wrote; none: this picture writes it). Every pixel where the picture
/// differs from it is its letters' (with what goes with them: the few
/// pixels where its own panel or a name's own bar departs from the shared
/// one), which the original mode draws and the enhanced mode, drawing the
/// base, doesn't.
pub struct Layered<'a> {
    pub base: &'a Tiles,
    pub base_file: Option<&'a str>,
}

/// A picture with letters in it as its base and a layer of its letters
/// ([`TileImage::letters`]): the pixels where the picture differs from the
/// base, each its palette index, and elsewhere the entry after the
/// palette's last (none). The base's own picture writes the base at `file`
/// and its letters beside (`NAME-letters.png`); another picture of the base
/// writes its letters at `file`, naming the base's. With nothing over its
/// own base, the picture alone, as [`export_image`].
pub fn export_layered(
    file: &str,
    tiles: &Tiles,
    l: &Layered,
    layout: Layout,
    rows: &[nettai_assets::Palette],
    owned: [usize; 2],
    row_of: impl Fn(usize) -> u8,
) -> (Vec<(String, Vec<u8>)>, TileImage) {
    let plain = || {
        let (png, doc) = export_image(file, tiles, layout, rows, owned, &row_of);
        (vec![(file.to_string(), png)], doc)
    };
    let n = tiles.pixels.len();
    // (A layer needs an entry past the palette's for "none".)
    if l.base.pixels.len() != n || rows.len() >= 16 {
        return plain();
    }
    if l.base_file.is_none() && l.base == tiles {
        return plain();
    }
    let palette = crate::image::palette_rgb(rows);
    let layered: Vec<[u8; 3]> = palette.iter().copied().chain([LETTERLESS]).collect();
    let pixels: Vec<Option<u8>> = (0..n).map(|i| (tiles.pixels[i] != l.base.pixels[i]).then_some(tiles.pixels[i])).collect();
    let letters = export_layer(file, &pixels, tiles.len(), layout, rows, [0, 0], &row_of).0;
    let stem = file.strip_suffix(".png").unwrap_or(file);
    let mut files = Vec::new();
    let doc = match l.base_file {
        // The base's own picture: the base at `file`, its letters beside.
        None => {
            let (png, doc) = export_image(file, l.base, layout, rows, owned, &row_of);
            files.push((file.to_string(), png));
            let name = format!("{stem}-letters.png");
            files.push((name.clone(), letters));
            TileImage { letters: Some(name), ..doc }
        }
        // Another picture of the base: its letters at `file`, in its
        // palette.
        Some(base) => {
            files.push((file.to_string(), letters));
            TileImage {
                file: file.into(),
                layout,
                tiles: tiles.len(),
                palettes: owned,
                fingerprint: crate::image::palette_fingerprint(&layered),
                letters: None,
                base: Some(base.to_string()),
            }
        }
    };
    (files, doc)
}

/// The color a letters image shows where it has no letter (for viewing).
const LETTERLESS: [u8; 3] = [255, 0, 255];

/// A layer of a picture's (what lies over what is under it: a Cross's
/// portrait over its name's bar, the name's letters over both) as an image
/// of `pixels` (in tile order, as many as `tiles` tiles have): each pixel
/// its palette index or, where the layer has none, the entry after the
/// palette's last. Its palette's rows `owned` are data, as
/// [`export_image`]'s.
pub fn export_layer(
    file: &str,
    pixels: &[Option<u8>],
    tiles: usize,
    layout: Layout,
    rows: &[nettai_assets::Palette],
    owned: [usize; 2],
    row_of: impl Fn(usize) -> u8,
) -> (Vec<u8>, TileImage) {
    let palette: Vec<[u8; 3]> = crate::image::palette_rgb(rows).into_iter().chain([LETTERLESS]).collect();
    let none = (palette.len() - 1) as u8;
    let (w, h) = layout.size(tiles as u32);
    let mut img = Indexed::new(w, h, palette.clone());
    img.pixels.iter_mut().for_each(|p| *p = none);
    for i in 0..tiles {
        let (x0, y0) = layout.place(i as u32);
        let r = row_of(i) << 4;
        for k in 0..Tiles::TILE {
            if let Some(Some(v)) = pixels.get(i * Tiles::TILE + k) {
                img.set(x0 + (k % 8) as u32, y0 + (k / 8) as u32, r | (v & 15));
            }
        }
    }
    let doc = TileImage {
        file: file.into(),
        layout,
        tiles,
        palettes: owned,
        fingerprint: crate::image::palette_fingerprint(&palette),
        letters: None,
        base: None,
    };
    (img.to_png(), doc)
}

/// Read a layer back ([`export_layer`]): its pixels (none where it has
/// none) and its data palettes.
pub fn import_layer(
    dir: &std::path::Path,
    prefix: &str,
    doc: &TileImage,
    report: &mut crate::report::Report,
) -> Option<(Vec<Option<u8>>, Vec<nettai_assets::Palette>)> {
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
        palettes = crate::image::palette_rows(&img.palette, first + count).0[first..].to_vec();
    }
    let (w, h) = doc.layout.size(doc.tiles as u32);
    if img.width < w || img.height < h {
        report.error(&name, format!("the layer is smaller than the {} tiles it lies over", doc.tiles));
        return None;
    }
    // (Its palette's last entry is none.)
    let none = (img.palette.len().saturating_sub(1)) as u8;
    let mut out = vec![None; doc.tiles * Tiles::TILE];
    for i in 0..doc.tiles {
        let (x0, y0) = doc.layout.place(i as u32);
        for k in 0..Tiles::TILE {
            let v = img.get(x0 + (k % 8) as u32, y0 + (k / 8) as u32);
            if v < none {
                out[i * Tiles::TILE + k] = Some(v & 15);
            }
        }
    }
    Some((out, palettes))
}

/// Read a tile image back: its tiles and its data palettes.
pub fn import_image(
    dir: &std::path::Path,
    prefix: &str,
    doc: &TileImage,
    report: &mut crate::report::Report,
) -> Option<(Tiles, Vec<nettai_assets::Palette>)> {
    let (tiles, _, palettes) = import_lettered(dir, prefix, doc, report)?;
    Some((tiles, palettes))
}

/// Read a tile image back, with its letters ([`TileImage::letters`],
/// [`TileImage::base`]): the picture (its letters over its base), the
/// picture as the enhanced mode shows it (its base), and its data palettes.
/// A picture with no letters is both.
pub fn import_lettered(
    dir: &std::path::Path,
    prefix: &str,
    doc: &TileImage,
    report: &mut crate::report::Report,
) -> Option<(Tiles, Tiles, Vec<nettai_assets::Palette>)> {
    let (base, palettes, letters) = match (&doc.base, &doc.letters) {
        // Another picture of the base: its letters at `file`, in its
        // palette (its data palettes are its letters' image's).
        (Some(base_file), _) => {
            let base_doc = TileImage { file: base_file.clone(), palettes: [0, 0], base: None, letters: None, ..doc.clone() };
            let (base, _) = import_base(dir, prefix, &base_doc, report)?;
            let (letters, palettes) = import_layer(dir, prefix, &TileImage { base: None, ..doc.clone() }, report)?;
            (base, palettes, letters)
        }
        (None, Some(file)) => {
            let (base, palettes) = import_base(dir, prefix, doc, report)?;
            let (letters, _) = import_layer(dir, prefix, &TileImage { file: file.clone(), palettes: [0, 0], ..doc.clone() }, report)?;
            (base, palettes, letters)
        }
        (None, None) => {
            let (base, palettes) = import_base(dir, prefix, doc, report)?;
            return Some((base.clone(), base, palettes));
        }
    };
    let mut tiles = base.clone();
    crate::letters::lay(&mut tiles, &letters);
    Some((tiles, base, palettes))
}

/// The image of a tile image alone (no letters over it) and its data
/// palettes.
fn import_base(
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

    /// A picture with letters, in two languages: one base, each language's
    /// letters over it (a letter of index 0 among them, over a pixel that
    /// isn't; the second's a pixel off its words, its own panel's); each
    /// read back as its picture and as the enhanced mode shows it (its
    /// base).
    #[test]
    fn a_lettered_picture_is_a_shared_base_and_its_layers() {
        let base = Tiles { pixels: vec![3; 2 * Tiles::TILE] };
        let mut own = base.clone();
        own.pixels[5] = 0;
        own.pixels[70] = 9;
        let mut other = base.clone();
        other.pixels[6] = 4;
        other.pixels[100] = 7;
        let layout = Layout::Grid { columns: 2 };
        let palette: nettai_assets::Palette = std::array::from_fn(|i| i as u16 * 0x421);
        let (mut files, doc) = export_layered("x.png", &own, &Layered { base: &base, base_file: None }, layout, &[palette], [0, 1], |_| 0);
        assert_eq!(files.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(), ["x.png", "x-letters.png"]);
        assert_eq!(doc.letters.as_deref(), Some("x-letters.png"));
        let (more, other_doc) = export_layered("x-ja.png", &other, &Layered { base: &base, base_file: Some("x.png") }, layout, &[palette], [0, 1], |_| 0);
        assert_eq!(more.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(), ["x-ja.png"]);
        assert_eq!(other_doc.base.as_deref(), Some("x.png"));
        files.extend(more);
        let dir = std::env::temp_dir().join(format!("nettai-lettered-{}", std::process::id()));
        nettai_fs::create_dir_all(&dir).unwrap();
        for (f, bytes) in &files {
            nettai_fs::write(dir.join(f), bytes).unwrap();
        }
        let mut report = crate::report::Report::default();
        let (picture, shown, palettes) = import_lettered(&dir, "t", &doc, &mut report).expect("read");
        assert_eq!((picture, shown, palettes), (own.clone(), base.clone(), vec![palette]));
        let (picture, shown, palettes) = import_lettered(&dir, "t", &other_doc, &mut report).expect("read");
        assert_eq!((picture, shown, palettes), (other.clone(), base.clone(), vec![palette]));
        assert!(!report.has_errors(), "{report}");
        // With no letters, the picture alone.
        let (files, doc) = export_layered("y.png", &base, &Layered { base: &base, base_file: None }, layout, &[palette], [0, 1], |_| 0);
        assert_eq!((files.len(), doc.letters), (1, None));
        let _ = nettai_fs::remove_dir_all(&dir);
    }

    /// A layer's pixels, and none where it has none, read back as written,
    /// with the palette rows it owns.
    #[test]
    fn a_layer_round_trips() {
        let pixels: Vec<Option<u8>> = (0..3 * Tiles::TILE).map(|i| (i % 5 != 0).then_some((i % 16) as u8)).collect();
        let layout = Layout::Blocks { width: 1, height: 1, columns: 1 };
        let rows: Vec<nettai_assets::Palette> = (0..3).map(|r| std::array::from_fn(|i| (r * 16 + i) as u16 * 0x21)).collect();
        let (png, doc) = export_layer("l.png", &pixels, 3, layout, &rows, [0, 3], |i| i as u8);
        let dir = std::env::temp_dir().join(format!("nettai-layer-{}", std::process::id()));
        nettai_fs::create_dir_all(&dir).unwrap();
        nettai_fs::write(dir.join("l.png"), png).unwrap();
        let mut report = crate::report::Report::default();
        assert_eq!(import_layer(&dir, "t", &doc, &mut report), Some((pixels, rows)));
        assert!(!report.has_errors(), "{report}");
        let _ = nettai_fs::remove_dir_all(&dir);
    }

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
