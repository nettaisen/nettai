//! A picture as an indexed PNG holds it (custom assets' pictures: GBA-shaped, a palette's color numbers).

use nettai_assets::Palette;

/// A picture as an indexed PNG holds it: its size, each pixel's color number (row by row), and its palette's
/// colors as BGR555.
pub(super) struct Indexed {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) pixels: Vec<u8>,
    pub(super) colors: Vec<u16>,
}

impl Indexed {
    /// The PNG `bytes`, which must be indexed (a palette's color numbers, of any bit depth).
    pub(super) fn read(bytes: &[u8]) -> Result<Indexed, String> {
        let mut decoder = png::Decoder::new(bytes);
        // (The color numbers as stored: no expansion to the palette's colors.)
        decoder.set_transformations(png::Transformations::IDENTITY);
        let mut reader = decoder.read_info().map_err(|e| format!("not a PNG ({e})"))?;
        let info = reader.info();
        if info.color_type != png::ColorType::Indexed {
            return Err(format!("an indexed PNG (a palette and each pixel's color number), not a {:?} one", info.color_type));
        }
        let colors: Vec<u16> = match &info.palette {
            Some(p) => p.chunks_exact(3).map(|c| nettai_assets::bgr555([c[0], c[1], c[2]])).collect(),
            None => return Err("an indexed PNG with no palette".to_string()),
        };
        let mut buf = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut buf).map_err(|e| format!("a PNG that doesn't decode ({e})"))?;
        let (width, height) = (frame.width as usize, frame.height as usize);
        let bits = frame.bit_depth as usize;
        let per_byte = 8 / bits;
        let mask = ((1u16 << bits) - 1) as u8;
        let mut pixels = Vec::with_capacity(width * height);
        for row in buf.chunks(frame.line_size).take(height) {
            for x in 0..width {
                // (The leftmost pixel in a byte's high bits.)
                let shift = 8 - bits * (x % per_byte + 1);
                pixels.push((row[x / per_byte] >> shift) & mask);
            }
        }
        Ok(Indexed { width, height, pixels, colors })
    }

    /// Whether each side is whole tiles (a multiple of 8 pixels), and not empty.
    pub(super) fn whole_tiles(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 || self.width % 8 != 0 || self.height % 8 != 0 {
            return Err(format!("{}x{} pixels, not whole 8x8 tiles (each side a multiple of 8)", self.width, self.height));
        }
        Ok(())
    }

    /// The 16 of the palette (0 the first 16 colors, 1 the next, ...) whose colors its pixels are: one for all of them
    /// (color 0 counts as any's first; with no other color, the first 16).
    pub(super) fn group(&self) -> Result<usize, String> {
        let mut found: Option<(usize, usize)> = None;
        for (i, &p) in self.pixels.iter().enumerate() {
            if p == 0 {
                continue;
            }
            let g = usize::from(p) / 16;
            match found {
                None => found = Some((g, i)),
                Some((other, at)) if other != g => {
                    let place = |i: usize| format!("({}, {})", i % self.width, i / self.width);
                    return Err(format!(
                        "its colors are of two 16s of the palette: color {} at {} (colors {} to {}) and color {p} at {} (colors {} to {}); a picture is in one",
                        self.pixels[at],
                        place(at),
                        16 * other,
                        16 * other + 15,
                        place(i),
                        16 * g,
                        16 * g + 15
                    ));
                }
                Some(_) => {}
            }
        }
        Ok(found.map_or(0, |(g, _)| g))
    }

    /// Whether every pixel is one of the palette's first `count` colors.
    pub(super) fn colors_below(&self, count: u8) -> Result<(), String> {
        match self.pixels.iter().position(|&p| p >= count) {
            Some(i) => Err(format!(
                "pixel ({}, {}) is color {} (a palette here has {count} colors, 0 to {})",
                i % self.width,
                i / self.width,
                self.pixels[i],
                count - 1
            )),
            None => Ok(()),
        }
    }

    /// The tile at column `tx`, row `ty` (in tiles): its 64 color numbers, row by row.
    pub(super) fn tile(&self, tx: usize, ty: usize) -> [u8; 64] {
        std::array::from_fn(|i| self.pixels[(ty * 8 + i / 8) * self.width + tx * 8 + i % 8])
    }

    /// Its colors as palettes, each 16 in turn (a color past its last is black): at least one.
    pub(super) fn palettes(&self) -> Vec<Palette> {
        let n = self.colors.len().div_ceil(16).max(1);
        (0..n).map(|p| std::array::from_fn(|i| self.colors.get(16 * p + i).copied().unwrap_or(0))).collect()
    }

    /// The rectangle `x0..x1`, `y0..y1` of it.
    pub(super) fn crop(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> Indexed {
        let pixels = (y0..y1).flat_map(|y| self.pixels[y * self.width + x0..y * self.width + x1].iter().copied()).collect();
        Indexed { width: x1 - x0, height: y1 - y0, pixels, colors: self.colors.clone() }
    }

    /// The smallest rectangle `x0..x1`, `y0..y1` of `x0..x1`, `y0..y1` that holds every pixel not color 0 there.
    pub(super) fn opaque_within(&self, (x0, y0, x1, y1): (usize, usize, usize, usize)) -> Option<(usize, usize, usize, usize)> {
        let mut found: Option<(usize, usize, usize, usize)> = None;
        for y in y0..y1 {
            for x in x0..x1 {
                if self.pixels[y * self.width + x] != 0 {
                    let (a, b, c, d) = found.unwrap_or((x, y, x + 1, y + 1));
                    found = Some((a.min(x), b.min(y), c.max(x + 1), d.max(y + 1)));
                }
            }
        }
        found
    }
}
