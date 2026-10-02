//! Palette-indexed PNG files and colour conversions.
//!
//! Content images are 8-bit indexed PNGs. Their palette (PLTE) holds up to
//! 16 rows of 16 colours; a pixel's value is `row * 16 + index`, where
//! `index` is the GBA's 4-bit colour index (0 = transparent) and `row` the
//! 16-colour palette it displays with. Entry 0 of every row is marked
//! transparent (tRNS), so editors show transparency as the game does.
//!
//! Colours are the GBA's BGR555 widened to 8 bits per channel as
//! `v << 3 | v >> 2` (the renderer's own conversion), so every GBA colour
//! has exactly one RGB value and back.

use std::io::BufWriter;
use std::path::Path;

/// An indexed image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Indexed {
    pub width: u32,
    pub height: u32,
    /// One palette index per pixel, row-major.
    pub pixels: Vec<u8>,
    /// RGB per palette entry.
    pub palette: Vec<[u8; 3]>,
}

impl Indexed {
    pub fn new(width: u32, height: u32, palette: Vec<[u8; 3]>) -> Indexed {
        Indexed { width, height, pixels: vec![0; (width * height) as usize], palette }
    }

    pub fn get(&self, x: u32, y: u32) -> u8 {
        self.pixels[(y * self.width + x) as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, v: u8) {
        self.pixels[(y * self.width + x) as usize] = v;
    }

    pub fn to_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(BufWriter::new(&mut out), self.width, self.height);
            enc.set_color(png::ColorType::Indexed);
            enc.set_depth(png::BitDepth::Eight);
            let mut plte = Vec::with_capacity(self.palette.len() * 3);
            for c in &self.palette {
                plte.extend_from_slice(c);
            }
            if plte.is_empty() {
                plte.extend_from_slice(&[0, 0, 0]);
            }
            enc.set_palette(plte);
            // Entry 0 of each 16-colour row is transparent.
            let trns: Vec<u8> = (0..self.palette.len().max(1)).map(|i| if i % 16 == 0 { 0 } else { 255 }).collect();
            enc.set_trns(trns);
            enc.set_compression(png::Compression::Best);
            let mut w = enc.write_header().expect("PNG header");
            w.write_image_data(&self.pixels).expect("PNG data");
        }
        out
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, self.to_png())
    }

    /// Read an indexed PNG of any bit depth. Other colour types are refused
    /// with a message that says how to fix the file.
    pub fn from_png(bytes: &[u8]) -> Result<Indexed, String> {
        let mut dec = png::Decoder::new(bytes);
        dec.set_transformations(png::Transformations::IDENTITY);
        let mut reader = dec.read_info().map_err(|e| format!("not a readable PNG: {e}"))?;
        let info = reader.info();
        if info.color_type != png::ColorType::Indexed {
            return Err(format!(
                "saved as {:?}, not as an indexed (palette) image: the colour indices the game uses are gone. \
                 Re-save it in indexed mode with the original palette (e.g. GIMP: Image > Mode > Indexed, \
                 'use custom palette'; Aseprite keeps indexed mode on its own)",
                info.color_type
            ));
        }
        let palette: Vec<[u8; 3]> =
            info.palette.as_ref().map(|p| p.as_chunks::<3>().0.iter().map(|c| [c[0], c[1], c[2]]).collect()).unwrap_or_default();
        let depth = info.bit_depth as u32;
        let (width, height) = (info.width, info.height);
        let mut buf = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut buf).map_err(|e| format!("corrupt PNG data: {e}"))?;
        let stride = frame.line_size;
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for y in 0..height as usize {
            let line = &buf[y * stride..(y + 1) * stride];
            for x in 0..width as usize {
                let v = match depth {
                    8 => line[x],
                    4 => (line[x / 2] >> (4 - 4 * (x % 2))) & 0xF,
                    2 => (line[x / 4] >> (6 - 2 * (x % 4))) & 0x3,
                    1 => (line[x / 8] >> (7 - (x % 8))) & 0x1,
                    d => return Err(format!("unsupported bit depth {d}")),
                };
                pixels.push(v);
            }
        }
        Ok(Indexed { width, height, pixels, palette })
    }

    pub fn load(path: &Path) -> Result<Indexed, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        Indexed::from_png(&bytes)
    }
}

/// A BGR555 colour as RGB.
pub fn rgb(c: u16) -> [u8; 3] {
    let w = |v: u16| ((v << 3) | (v >> 2)) as u8;
    [w(c & 31), w((c >> 5) & 31), w((c >> 10) & 31)]
}

/// An RGB colour as BGR555, and whether it was exactly a GBA colour
/// (otherwise each channel is rounded to the nearest of the 32 levels).
pub fn bgr555(c: [u8; 3]) -> (u16, bool) {
    let ch = |v: u8| -> (u16, bool) {
        let n = ((v as u32 * 31 + 127) / 255) as u16;
        let exact = ((n << 3) | (n >> 2)) as u8 == v;
        (n, exact)
    };
    let (r, er) = ch(c[0]);
    let (g, eg) = ch(c[1]);
    let (b, eb) = ch(c[2]);
    (r | g << 5 | b << 10, er && eg && eb)
}

/// Palette rows as PNG palette entries.
pub fn palette_rgb(rows: &[[u16; 16]]) -> Vec<[u8; 3]> {
    rows.iter().flat_map(|p| p.iter().map(|&c| rgb(c))).collect()
}

/// PNG palette entries back to BGR555 rows (the last row padded with
/// black), with the indices of entries that weren't GBA colours.
pub fn palette_rows(entries: &[[u8; 3]], rows: usize) -> (Vec<[u16; 16]>, Vec<usize>) {
    let mut off_grid = Vec::new();
    let mut out = vec![[0u16; 16]; rows];
    for (i, &c) in entries.iter().enumerate().take(rows * 16) {
        let (v, exact) = bgr555(c);
        if !exact {
            off_grid.push(i);
        }
        out[i / 16][i % 16] = v;
    }
    (out, off_grid)
}

/// Hashes of a palette in order and sorted, to tell a reordered palette
/// from an edited one.
pub fn palette_fingerprint(entries: &[[u8; 3]]) -> String {
    let flat: Vec<u8> = entries.iter().flatten().copied().collect();
    let mut sorted = entries.to_vec();
    sorted.sort();
    let sorted: Vec<u8> = sorted.iter().flatten().copied().collect();
    format!("{}-{}", crate::report::stamp(&flat), crate::report::stamp(&sorted))
}

/// Explain how a palette differs from the one it was exported with.
pub fn palette_change(expected: &str, entries: &[[u8; 3]], needed: usize) -> Option<String> {
    let now = palette_fingerprint(entries);
    if now == expected {
        return None;
    }
    if entries.len() < needed {
        return Some(format!(
            "the palette has {} colours but the image needs {needed}: the editor dropped entries \
             (unused or duplicate colours). Colour indices have shifted; undo the save or re-export",
            entries.len()
        ));
    }
    let (_, sorted_now) = now.split_once('-').unwrap();
    let (_, sorted_then) = expected.split_once('-').unwrap_or(("", ""));
    if sorted_now == sorted_then {
        return Some(
            "the palette has the same colours in a different order: the editor re-sorted it. Pixels now \
             point at other indices, which breaks palette swaps; undo the save or turn off palette sorting"
                .into(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_round_trip_exactly() {
        for c in 0..0x8000u16 {
            assert_eq!(bgr555(rgb(c)), (c, true));
        }
        assert_eq!(bgr555([255, 0, 9]), (31 | 1 << 10, false));
    }

    #[test]
    fn png_round_trip() {
        let mut img = Indexed::new(3, 2, palette_rgb(&[[0x7FFF; 16], [0x001F; 16]]));
        img.set(1, 0, 17);
        img.set(2, 1, 5);
        let back = Indexed::from_png(&img.to_png()).unwrap();
        assert_eq!(back, img);
    }

    #[test]
    fn palette_changes_are_explained() {
        let p: Vec<[u8; 3]> = (0..16).map(|i| [i * 8, 0, 0]).collect();
        let fp = palette_fingerprint(&p);
        assert_eq!(palette_change(&fp, &p, 16), None);
        let mut q = p.clone();
        q.swap(1, 2);
        assert!(palette_change(&fp, &q, 16).unwrap().contains("re-sorted"));
        assert!(palette_change(&fp, &p[..8], 16).unwrap().contains("dropped"));
        let mut r = p.clone();
        r[3] = [255, 255, 255];
        assert_eq!(palette_change(&fp, &r, 16), None, "an edited colour is an edit, not damage");
    }
}
