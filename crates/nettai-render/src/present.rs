//! Showing a frame at the output's size: the 240x160 picture scaled up,
//! then its text items drawn over it at the output's resolution (the
//! window and the PNG writer, [`write_png`], share this).
//!
//! The scaling policy: the picture takes the largest whole multiple of
//! 240x160 that fits the output, centered on black, so every frame pixel is
//! the same square of output pixels whatever the window's size; only an
//! output smaller than 240x160 gets a fractional (shrunk) picture. Text is
//! drawn at the same placement and scale.

use crate::compose::{HEIGHT, WIDTH, to_rgb};
use crate::render::Frame;
use crate::vfont::TextRenderer;
use std::path::Path;

/// Where the frame lands in the output: its top left, its size and the
/// output pixels a frame pixel takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: f32,
}

impl Placement {
    /// The frame in an output of `w` by `h` pixels, by the policy above.
    pub fn fit(w: usize, h: usize) -> Placement {
        let k = (w / WIDTH).min(h / HEIGHT);
        let (width, height, scale) = if k >= 1 {
            (WIDTH * k, HEIGHT * k, k as f32)
        } else {
            let s = (w as f32 / WIDTH as f32).min(h as f32 / HEIGHT as f32);
            (((WIDTH as f32 * s) as usize).max(1), ((HEIGHT as f32 * s) as usize).max(1), s)
        };
        Placement { x: (w - width.min(w)) as i32 / 2, y: (h - height.min(h)) as i32 / 2, width: width as i32, height: height as i32, scale }
    }

    /// The frame pixel an output pixel shows, if it shows one.
    pub fn frame_pixel(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let (dx, dy) = (x - self.x, y - self.y);
        if dx < 0 || dy < 0 || dx >= self.width || dy >= self.height {
            return None;
        }
        Some((dx * WIDTH as i32 / self.width, dy * HEIGHT as i32 / self.height))
    }
}

/// Draw `frame` into `out` (0RGB, `w` by `h` pixels): the picture scaled
/// as [`Placement::fit`] places it, black around it, and with `text` its
/// text items over it.
pub fn present(frame: &Frame, text: Option<&mut TextRenderer>, out: &mut [u32], w: usize, h: usize) -> Placement {
    let place = Placement::fit(w, h);
    out[..w * h].fill(0);
    let columns: Vec<usize> = (0..place.width).map(|dx| (dx * WIDTH as i32 / place.width) as usize).collect();
    for dy in 0..place.height {
        let fy = (dy * HEIGHT as i32 / place.height) as usize;
        let row = &frame.pixels[fy * WIDTH..(fy + 1) * WIDTH];
        let at = (place.y + dy) as usize * w + place.x as usize;
        for (o, &fx) in out[at..at + place.width as usize].iter_mut().zip(&columns) {
            *o = to_rgb(row[fx]);
        }
    }
    if let Some(t) = text
        && !frame.text.is_empty()
    {
        t.draw(&frame.text, &frame.depth, &place, out, w);
    }
    place
}

/// Draw a frame's text items into its own picture, at the frame's
/// resolution, and leave it none: the original mode's strings in a
/// language its pack has no fonts for (`Renderer::render`). A pixel no
/// item covers keeps its color.
pub fn draw_text(frame: &mut Frame, text: &mut TextRenderer) {
    if frame.text.is_empty() {
        return;
    }
    let before: Vec<u32> = frame.pixels.iter().map(|&p| to_rgb(p)).collect();
    let mut out = before.clone();
    text.draw(&frame.text, &frame.depth, &Placement::fit(WIDTH, HEIGHT), &mut out, WIDTH);
    for ((p, was), now) in frame.pixels.iter_mut().zip(&before).zip(&out) {
        if was != now {
            let five = |shift: u32| ((now >> (shift + 3)) & 0x1F) as u16;
            *p = five(16) | five(8) << 5 | five(0) << 10;
        }
    }
    frame.text.clear();
}

/// Write a frame as an RGB PNG, scaled up by an integer factor, with its
/// text items drawn at that scale by `text` ([`present`]).
pub fn write_png(path: &Path, frame: &Frame, scale: usize, text: Option<&mut TextRenderer>) -> std::io::Result<()> {
    let scale = scale.max(1);
    let (w, h) = (WIDTH * scale, HEIGHT * scale);
    let mut out = vec![0u32; w * h];
    present(frame, text, &mut out, w, h);
    write_rgb_png(path, &out, w, h)
}

/// Write 0RGB pixels, `w` by `h`, as an RGB PNG.
pub fn write_rgb_png(path: &Path, out: &[u32], w: usize, h: usize) -> std::io::Result<()> {
    let rgb: Vec<u8> = out.iter().flat_map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]).collect();
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(std::io::Error::other)?;
    writer.write_image_data(&rgb).map_err(std::io::Error::other)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_multiples_centered_and_shrunk_only_below_one() {
        assert_eq!(Placement::fit(960, 640), Placement { x: 0, y: 0, width: 960, height: 640, scale: 4.0 });
        // A window between multiples: the largest that fits, centered.
        let p = Placement::fit(1000, 700);
        assert_eq!((p.x, p.y, p.width, p.height, p.scale), (20, 30, 960, 640, 4.0));
        assert_eq!(p.frame_pixel(20, 30), Some((0, 0)));
        assert_eq!(p.frame_pixel(23, 33), Some((0, 0)));
        assert_eq!(p.frame_pixel(24, 33), Some((1, 0)));
        assert_eq!(p.frame_pixel(19, 30), None);
        assert_eq!(p.frame_pixel(980, 30), None);
        // Smaller than the frame: shrunk to fit.
        let p = Placement::fit(120, 100);
        assert_eq!((p.x, p.y, p.width, p.height, p.scale), (0, 10, 120, 80, 0.5));
        assert_eq!(p.frame_pixel(119, 89), Some((238, 158)));
    }

    #[test]
    fn a_frame_scales_by_whole_pixels() {
        let mut pixels = vec![0u16; WIDTH * HEIGHT];
        pixels[0] = 0x001F;
        pixels[WIDTH + 1] = 0x7C00;
        let frame = Frame { pixels, depth: vec![0; WIDTH * HEIGHT], text: Vec::new() };
        let (w, h) = (2 * WIDTH + 10, 2 * HEIGHT);
        let mut out = vec![0xABCDEF; w * h];
        present(&frame, None, &mut out, w, h);
        assert_eq!(out[0], 0, "the border is black");
        assert_eq!(out[5], to_rgb(0x001F));
        assert_eq!(out[w + 6], to_rgb(0x001F));
        assert_eq!(out[2 * w + 7], to_rgb(0x7C00));
        assert_eq!(out[7], to_rgb(0));
    }

    /// The original mode's strings in a language the pack has no fonts for
    /// are drawn into the frame itself: its pixels take the text in the
    /// item's colors within the item's box, nothing else changes, and the
    /// frame carries no item for a window to draw again.
    #[test]
    fn text_drawn_into_the_frame_leaves_none_to_draw() {
        use crate::textlayer::{Rect, TextItem};
        use crate::vfont::{Role, VectorFont};
        let (face, shadow) = (0x7FFF, 0x2108);
        let item = TextItem::new("Cannon", Role::Cell, Rect::new(16, 8, 64, 16), face, Some(shadow));
        let mut frame = Frame { pixels: vec![0x1234; WIDTH * HEIGHT], depth: vec![u32::MAX; WIDTH * HEIGHT], text: vec![item] };
        draw_text(&mut frame, &mut TextRenderer::new(std::sync::Arc::new(VectorFont::bundled())));
        assert!(frame.text.is_empty());
        let changed: Vec<usize> = (0..WIDTH * HEIGHT).filter(|&i| frame.pixels[i] != 0x1234).collect();
        assert!(changed.len() > 40, "the text is drawn ({} pixels)", changed.len());
        assert!(changed.iter().all(|&i| (16..16 + 64 + 2).contains(&(i % WIDTH)) && (8..8 + 16 + 2).contains(&(i / WIDTH))));
        assert!(frame.pixels.contains(&face), "the face's color, whole where a glyph covers a pixel");
    }
}
