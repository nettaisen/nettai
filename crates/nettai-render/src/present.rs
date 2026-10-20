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
    // (The output columns that show the nudged frame: `smooth::Nudge`.)
    let nudged: Vec<bool> = match &frame.nudge {
        Some(n) => (0..place.width).map(|dx| crate::smooth::nudged_column(n, dx, &place)).collect(),
        None => Vec::new(),
    };
    for dy in 0..place.height {
        let fy = (dy * HEIGHT as i32 / place.height) as usize;
        // (The output rows that show the lifted frame: `smooth::Lift`. The
        // gauge's nudge is on the others.)
        let lifted = frame.lift.as_ref().filter(|l| crate::smooth::lifted_row(l, dy, &place));
        let (pixels, depth) = match lifted {
            Some(l) => (&l.pixels, &l.depth),
            None => (&frame.pixels, &frame.depth),
        };
        let nudge = frame.nudge.as_ref().filter(|_| lifted.is_none());
        let row = &pixels[fy * WIDTH..(fy + 1) * WIDTH];
        let at = (place.y + dy) as usize * w + place.x as usize;
        let line = &mut out[at..at + place.width as usize];
        for (o, &fx) in line.iter_mut().zip(&columns) {
            *o = to_rgb(row[fx]);
        }
        if let Some(n) = nudge {
            let moved = &n.pixels[fy * WIDTH..(fy + 1) * WIDTH];
            for ((o, &fx), _) in line.iter_mut().zip(&columns).zip(&nudged).filter(|(_, n)| **n) {
                *o = to_rgb(moved[fx]);
            }
        }
        if let Some(g) = &frame.glide {
            let depth = &depth[fy * WIDTH..(fy + 1) * WIDTH];
            glide_row(g, depth, nudge, &place, &nudged, dy, fy, line);
        }
    }
    // (The overlays' depth keys on the output pixels they draw, for the text
    // layer's items behind them: an emblem spinning over a window drawn as
    // shapes.)
    let mut cover: Vec<u32> = Vec::new();
    if !frame.overlays.is_empty() {
        cover = vec![u32::MAX; out.len()];
        crate::smooth::draw_overlays_covering(&frame.overlays, &frame.depth, frame.nudge.as_ref(), frame.lift.as_ref(), &place, out, w, Some(&mut cover));
    }
    if let Some(t) = text
        && !frame.text.is_empty()
    {
        // (The packs' drawings rasterized at this scale before the first
        // is drawn: once, or again away from the frames when it changes.)
        if let Some(d) = &frame.drawings {
            t.rasters.prepare_all(d, place.scale);
        }
        t.draw_over(&frame.text, &frame.depth, (!cover.is_empty()).then_some(&cover[..]), &place, out, w);
    }
    place
}

/// A glide's row of the output (`Glide`): where the frame's pixel under an
/// output pixel is the background's (or the backdrop's), the background a
/// part of a pixel on, at the output's resolution: each output pixel takes
/// the background pixel its place, in sixteenths of a frame pixel, plus the
/// glide's falls in. `depth` is the row's depth keys (the lifted frame's on
/// a lifted row); where the output shows the nudged frame, `nudged`, that
/// frame's pixel's.
#[allow(clippy::too_many_arguments)]
fn glide_row(g: &crate::render::Glide, depth: &[u32], nudge: Option<&crate::smooth::Nudge>, place: &Placement, nudged: &[bool], dy: i32, fy: usize, out: &mut [u32]) {
    use crate::compose::BACKDROP_DEPTH;
    use crate::render::Glide;
    let sixteenths = |d: i32, size: usize, span: i32, sub: u8| ((d as i64 * 16 * size as i64 / span as i64) as usize + sub as usize) / 16;
    let gy = sixteenths(dy, HEIGHT, place.height, g.sub.1).min(Glide::HEIGHT - 1);
    let moved = nudge.map(|n| &n.depth[fy * WIDTH..(fy + 1) * WIDTH]);
    let source = &g.pixels[gy * Glide::WIDTH..(gy + 1) * Glide::WIDTH];
    for (dx, o) in out.iter_mut().enumerate() {
        let fx = (dx as i32 * WIDTH as i32 / place.width) as usize;
        let depth = match moved {
            Some(m) if nudged[dx] => m,
            _ => depth,
        };
        if depth[fx] == g.depth || depth[fx] == BACKDROP_DEPTH {
            let gx = sixteenths(dx as i32, WIDTH, place.width, g.sub.0).min(Glide::WIDTH - 1);
            *o = to_rgb(source[gx]);
        }
    }
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
        let frame = Frame { pixels, depth: vec![0; WIDTH * HEIGHT], text: Vec::new(), glide: None, ..Frame::default() };
        let (w, h) = (2 * WIDTH + 10, 2 * HEIGHT);
        let mut out = vec![0xABCDEF; w * h];
        present(&frame, None, &mut out, w, h);
        assert_eq!(out[0], 0, "the border is black");
        assert_eq!(out[5], to_rgb(0x001F));
        assert_eq!(out[w + 6], to_rgb(0x001F));
        assert_eq!(out[2 * w + 7], to_rgb(0x7C00));
        assert_eq!(out[7], to_rgb(0));
    }

    /// A glide shows the background a part of a pixel on, at the output's
    /// resolution, where the frame shows the background or the backdrop;
    /// what is in front of it stays on the frame's pixels.
    #[test]
    fn a_glide_moves_the_background_by_parts_of_a_pixel() {
        use crate::compose::{BACKDROP_DEPTH, depth_key};
        use crate::render::Glide;
        // The background's columns colored by their x; the frame shows it
        // at its whole pixels, a sprite over its column 3.
        let bg_depth = depth_key(3, 2, 0);
        let mut pixels: Vec<u16> = (0..WIDTH * HEIGHT).map(|i| (i % WIDTH) as u16).collect();
        let mut depth = vec![bg_depth; WIDTH * HEIGHT];
        pixels[3] = 0x7FFF;
        depth[3] = depth_key(0, 0, 1);
        depth[4] = BACKDROP_DEPTH;
        let glide = Glide { pixels: (0..Glide::WIDTH * Glide::HEIGHT).map(|i| (i % Glide::WIDTH) as u16).collect(), sub: (8, 0), depth: bg_depth };
        let mut frame = Frame { pixels, depth, text: Vec::new(), glide: None, ..Frame::default() };
        let (w, h) = (2 * WIDTH, 2 * HEIGHT);
        let mut still = vec![0; w * h];
        present(&frame, None, &mut still, w, h);
        frame.glide = Some(glide);
        let mut moved = vec![0; w * h];
        present(&frame, None, &mut moved, w, h);
        let row = |out: &[u32]| out[..10].to_vec();
        assert_eq!(row(&still), [0, 0, 1, 1, 2, 2, 0x7FFF, 0x7FFF, 4, 4].map(to_rgb));
        // Half a pixel on: each output pixel a column further, half the time;
        // the sprite's stays; the backdrop's glides.
        assert_eq!(row(&moved), [0, 1, 1, 2, 2, 3, 0x7FFF, 0x7FFF, 4, 5].map(to_rgb));
    }

    /// A lift shows the lifted frame on the output rows whose middle, its
    /// part of a pixel further down, is in the next frame row: half a pixel
    /// at twice the frame's size, every second row; at three times, two of
    /// three; at four, two of four. A field whose top is at row 50 in the
    /// frame and 49 lifted shows from the output row a sprite's part half a
    /// pixel up from row 50 starts on.
    #[test]
    fn a_lift_steps_the_field_with_the_sprites_on_it() {
        use crate::smooth::{Lift, lifted_row};
        let lift = Lift { pixels: Vec::new(), depth: Vec::new(), sub: 0x8000 };
        for (scale, rows) in [(2, vec![false, true]), (3, vec![false, true, true]), (4, vec![false, false, true, true])] {
            let place = Placement::fit(scale * WIDTH, scale * HEIGHT);
            let seen: Vec<bool> = (0..scale as i32).map(|dy| lifted_row(&lift, 30 * scale as i32 + dy, &place)).collect();
            assert_eq!(seen, rows, "at {scale} times");
        }
        let (field, sky) = (0x03E0, 0x001F);
        let picture = |top: usize| (0..WIDTH * HEIGHT).map(|i| if i / WIDTH >= top { field } else { sky }).collect::<Vec<u16>>();
        let frame = Frame {
            pixels: picture(50),
            depth: vec![0; WIDTH * HEIGHT],
            lift: Some(Lift { pixels: picture(49), depth: vec![0; WIDTH * HEIGHT], sub: 0x8000 }),
            ..Frame::default()
        };
        let (w, h) = (3 * WIDTH, 3 * HEIGHT);
        let mut out = vec![0; w * h];
        present(&frame, None, &mut out, w, h);
        let first = (0..h).find(|&y| out[y * w] == to_rgb(field)).unwrap();
        // (The overlay's first row: its texel row 0 from the output row whose
        // middle is past 49.5 frame rows.)
        let part = (0..h).find(|&y| (y as f64 + 0.5) / 3.0 - 49.5 >= 0.0).unwrap();
        assert_eq!((first, part), (148, 148));
    }

    /// A nudge shows on the output pixels left of its part of a pixel, and
    /// the frame's own on the rest: a half pixel at twice the frame's size,
    /// the left output pixel of each; a quarter at four times, the first of
    /// four.
    #[test]
    fn a_nudge_shows_left_of_its_part_of_a_pixel() {
        use crate::smooth::Nudge;
        let (here, moved) = (0x001F, 0x7C00);
        let frame = |sub| Frame {
            pixels: vec![here; WIDTH * HEIGHT],
            depth: vec![0; WIDTH * HEIGHT],
            nudge: Some(Nudge { pixels: vec![moved; WIDTH * HEIGHT], depth: vec![0; WIDTH * HEIGHT], sub }),
            ..Frame::default()
        };
        for (scale, sub, row) in [(2, 8, vec![moved, here, moved, here]), (4, 4, vec![moved, here, here, here, moved])] {
            let (w, h) = (scale * WIDTH, scale * HEIGHT);
            let mut out = vec![0; w * h];
            present(&frame(sub), None, &mut out, w, h);
            assert_eq!(out[w..w + row.len()].to_vec(), row.iter().map(|&c| to_rgb(c)).collect::<Vec<_>>(), "at {scale} times");
        }
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
        let mut frame = Frame { pixels: vec![0x1234; WIDTH * HEIGHT], depth: vec![u32::MAX; WIDTH * HEIGHT], text: vec![item], glide: None, ..Frame::default() };
        draw_text(&mut frame, &mut TextRenderer::new(std::sync::Arc::new(VectorFont::bundled())));
        assert!(frame.text.is_empty());
        let changed: Vec<usize> = (0..WIDTH * HEIGHT).filter(|&i| frame.pixels[i] != 0x1234).collect();
        assert!(changed.len() > 40, "the text is drawn ({} pixels)", changed.len());
        assert!(changed.iter().all(|&i| (16..16 + 64 + 2).contains(&(i % WIDTH)) && (8..8 + 16 + 2).contains(&(i / WIDTH))));
        assert!(frame.pixels.contains(&face), "the face's color, whole where a glyph covers a pixel");
    }
}
