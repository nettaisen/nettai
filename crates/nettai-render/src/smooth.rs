//! The enhanced mode's smooth motion (`Renderer::motion`): what the original
//! itself moves, in whole frame pixels or in coarse steps, drawn between
//! them at the output's resolution, over the same frames. Nothing moves
//! that the original holds still, and nothing comes in that the original
//! shows at once.
//!
//! - **The custom gauge's fill** (`hud::gauge_sub`): its end where the
//!   counter is, not at its whole pixels.
//! - **The squashed and turned sprites** ([`Overlay`]): the banners' and
//!   the telops' squash and stretch, and the custom screen's emblem as it
//!   spins, read through the original's own scale or matrix on each tick at
//!   the output's resolution instead of the frame's, which picks whole rows
//!   and texels of them. Their curves and timing are the game's.
//! - **The custom screen's slide** ([`Lift`], [`Overlay::fine`]): the
//!   camera moves the field down 1.5 pixels a tick with the window, and the
//!   projection takes its whole pixels, so the field and every sprite on it
//!   step 1, 2, 1, 2 pixels. The field layer is drawn again a pixel up and
//!   shown on the output rows past the camera's half pixel; the sprites
//!   are drawn at their true places (overlays, the part of a pixel the
//!   projection rounds off: `objects::project_fine`).
//! - (The background's glide is `Frame::glide`, `present`.)
//!
//! A position between two whole pixels is drawn twice: the frame as ever,
//! and a [`Nudge`], the frame drawn again with what moves a pixel on;
//! `present` shows on each output pixel the one its place in its frame
//! pixel falls in (left of the part of a pixel the motion is past its
//! whole pixels: the nudged one). The two differ only where the moving
//! thing is, so the rest of the frame stays as it is.
//!
//! **Deterministic.** Every place is the battle's state's: the gauge's
//! counter, a banner's step and timer, the emblem's matrix, the camera's and the
//! objects' 16.16 places. Nothing reads a clock and nothing is kept in the
//! simulation, its snapshot or its digest; a rollback that draws a frame
//! again, a replay and a video draw it the same. The frame's own 240x160
//! pixels (`Frame::pixels`) are a whole-pixel picture of the same, as the
//! original's are.

use crate::compose::{Affine, Fade, HEIGHT, SpritePart, WIDTH, apply_fade, to_rgb};
use crate::present::Placement;

/// The enhanced mode's motion in a drawing of the frame: on or off (the
/// original mode, Reduce motion), and whether this drawing is the nudged
/// one (the gauge's fill a pixel longer) or the lifted one (the field
/// layer a pixel up: [`Lift`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Smooth {
    pub on: bool,
    pub nudged: bool,
    pub lifted: bool,
}

/// The frame drawn again with what moves between whole pixels a pixel
/// right, and how far past its whole pixels it is, in sixteenths of a
/// pixel: `present` shows these pixels on the output pixels whose place in
/// their frame pixel is left of `sub` (the frame's elsewhere).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Nudge {
    pub pixels: Vec<u16>,
    pub depth: Vec<u32>,
    pub sub: u8,
}

/// Whether the output column `dx` (from the frame's left edge, of
/// `place.width`) shows the nudged frame.
pub(crate) fn nudged_column(nudge: &Nudge, dx: i32, place: &Placement) -> bool {
    ((dx as i64 * 16 * WIDTH as i64 / place.width as i64) % 16) < nudge.sub as i64
}

/// The frame drawn again with the field layer a pixel up, and the part of
/// a pixel (16.16) the camera's Y is past its whole pixels: the field's
/// true place is that much above the frame's (the projection takes the
/// camera's whole pixels: the custom screen's slide moves it 1.5 pixels a
/// tick). `present` shows these pixels on the output rows whose middle,
/// that much further down, falls in the next frame row ([`lifted_row`]).
/// Only the field layer differs: the sprites on it are overlays at their
/// own true places ([`Overlay`]), left out of both.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lift {
    pub pixels: Vec<u16>,
    pub depth: Vec<u32>,
    pub sub: i32,
}

/// Whether the output row `dy` (from the frame's top edge, of
/// `place.height`) shows the lifted frame: its middle, in 16.16 frame
/// rows, plus the lift's part of a pixel passes into the next row. (An
/// overlay reads its texel at the output pixel's middle alike, so the
/// field and the sprites on it step on the same output rows.)
pub(crate) fn lifted_row(lift: &Lift, dy: i32, place: &Placement) -> bool {
    let middle = ((2 * dy as i64 + 1) * HEIGHT as i64 * 0x10000) / (2 * place.height as i64);
    (middle & 0xFFFF) + lift.sub as i64 >= 0x10000
}

/// How an overlay's box reads its texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// As it is, a texel a pixel (a part drawn at a true place between the
    /// frame's pixels: `Overlay::fine`).
    Plain,
    /// Rows step by `n / 256` a screen row about the box's middle (the
    /// banners' squash: `SpritePart::vscale`).
    Squashed(i32),
    /// Through an affine matrix about the box's center (`SpritePart::affine`).
    Turned(Affine),
}

/// A sprite part the enhanced mode's motion draws at the output's
/// resolution instead of into the frame: one the original squashes or
/// turns, whose rows or texels the frame's pixels would pick coarsely, or
/// one whose true place is between the frame's pixels (its `fine`). Its
/// texture's colors (faded as the frame's sprites are), its box in frame
/// pixels, how the box reads the texture, and its depth key: `present`
/// draws it over the scaled frame where nothing in front of it won the
/// pixel, as the frame would have drawn it there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overlay {
    pub x: i32,
    pub y: i32,
    /// The box (an affine part's doubled box is twice the texture).
    pub width: i32,
    pub height: i32,
    /// The texture, `texture_width` texels a row, flipped as the part is;
    /// none where it is clear.
    pub texture_width: i32,
    pub texels: Vec<Option<u16>>,
    pub reading: Reading,
    pub depth: u32,
    /// Its true place less `x` and `y`, 16.16 pixels right and down
    /// (`objects::Fine`).
    pub fine: (i32, i32),
}

impl Overlay {
    /// The overlay of `part`, the frame's sprite of depth key `depth`
    /// whose true place is `fine` off its own, its colors through the
    /// sprites' fade then the screen's (`fades`); none for a part the frame
    /// draws as it is (neither squashed nor turned and at its true place,
    /// or blended or in mosaic, which the frame's composition mixes).
    pub fn of(part: &SpritePart, depth: u32, fades: [Fade; 2], fine: (i32, i32)) -> Option<Overlay> {
        if part.alpha.is_some() || part.mosaic.is_some() {
            return None;
        }
        let reading = match (part.affine, part.vscale) {
            (Some(m), _) => Reading::Turned(m),
            (None, Some(n)) => Reading::Squashed(n),
            (None, None) if fine != (0, 0) => Reading::Plain,
            (None, None) => return None,
        };
        let (w, h) = (part.width as i32, part.height as i32);
        let per_row = (part.width / 8) as usize;
        let mut texels = Vec::with_capacity((w * h) as usize);
        for row in 0..h {
            for col in 0..w {
                // (The flips are the texture's: the frame flips the texel it
                // reads, for a squashed part after the row's step.)
                let (tx, ty) = (if part.hflip { w - 1 - col } else { col } as usize, if part.vflip { h - 1 - row } else { row } as usize);
                let index = part.tiles.get(part.first_tile + (ty / 8) * per_row + tx / 8).map_or(0, |t| t[(ty % 8) * 8 + tx % 8]);
                texels.push((index != 0).then(|| apply_fade(apply_fade(part.palette[index as usize] & 0x7FFF, fades[0]), fades[1])));
            }
        }
        let (bw, bh) = match reading {
            Reading::Turned(m) if m.double => (2 * w, 2 * h),
            _ => (w, h),
        };
        // (Its place wrapped as the hardware's.)
        let x = if part.x as i32 + bw > 0x200 { part.x as i32 - 0x200 } else { part.x as i32 };
        let y = if part.y as i32 + bh > 0x100 { part.y as i32 - 0x100 } else { part.y as i32 };
        Some(Overlay { x, y, width: bw, height: bh, texture_width: w, texels, reading, depth, fine })
    }

    /// The texel an output pixel shows, by its top left corner and its
    /// middle in the box (frame pixels from the box's top left,
    /// continuous), as the frame reads the texel at a whole pixel's.
    fn texel(&self, (cx, cy): (f64, f64), (mx, my): (f64, f64)) -> Option<u16> {
        let (w, h) = (self.texture_width, self.texels.len() as i32 / self.texture_width);
        // (The frame reads a squashed or turned part at its pixel's top
        // left; so does this, at the output pixel's: drawn at the frame's
        // own size it reads the same texels, and larger, a whole scale's
        // rows and texels on the frame pixels they are on. Read half a frame
        // pixel above and left of the output pixel's middle, as it once
        // was, the part was drawn up to that much down and right of the
        // frame's, its last row past its box and cut: EXE4's banners, whose
        // glyphs' outlines reach their last row.) A corner is a whole output
        // pixel over the scale, which a float holds only near: what it
        // reads is taken a hair up, as the hardware's whole steps are exact.
        let floor = |v: f64| (v + 1e-9).floor() as i32;
        let (tx, ty) = match self.reading {
            Reading::Plain => (mx.floor() as i32, my.floor() as i32),
            Reading::Squashed(n) => (mx.floor() as i32, floor(n as f64 * (cy - (h / 2) as f64) / 256.0) + h / 2),
            Reading::Turned(m) => {
                let (dx, dy) = (cx - (self.width / 2) as f64, cy - (self.height / 2) as f64);
                let tx = floor((m.pa as f64 * dx + m.pb as f64 * dy) / 256.0) + w / 2;
                let ty = floor((m.pc as f64 * dx + m.pd as f64 * dy) / 256.0) + h / 2;
                (tx, ty)
            }
        };
        if !(0..w).contains(&tx) || !(0..h).contains(&ty) {
            return None;
        }
        self.texels[(ty * w + tx) as usize]
    }
}

/// Draw a frame's overlays into its scaled picture `out` (`w` pixels a
/// row), back to front, each where the frame's depth keys (the lifted
/// frame's or the nudged frame's, on the output pixels that show them) say
/// nothing in front of it won the pixel.
pub(crate) fn draw_overlays(overlays: &[Overlay], depth: &[u32], nudge: Option<&Nudge>, lift: Option<&Lift>, place: &Placement, out: &mut [u32], w: usize) {
    let s = place.scale as f64;
    for o in overlays.iter().rev() {
        let left = o.x as f64 + o.fine.0 as f64 / 65536.0;
        let top = o.y as f64 + o.fine.1 as f64 / 65536.0;
        // (The output pixels its box covers, within the frame's.)
        let x0 = ((left * s).floor() as i32).max(0);
        let x1 = (((left + o.width as f64) * s).ceil() as i32).min(place.width);
        let y0 = ((top * s).floor() as i32).max(0);
        let y1 = (((top + o.height as f64) * s).ceil() as i32).min(place.height);
        for dy in y0..y1 {
            let fy = (dy * HEIGHT as i32 / place.height) as usize;
            let (cy, my) = (dy as f64 / s - top, (dy as f64 + 0.5) / s - top);
            let lifted = lift.filter(|l| lifted_row(l, dy, place));
            for dx in x0..x1 {
                let Some(color) = o.texel((dx as f64 / s - left, cy), ((dx as f64 + 0.5) / s - left, my)) else { continue };
                let fx = (dx * WIDTH as i32 / place.width) as usize;
                let front = match (lifted, nudge) {
                    (Some(l), _) => l.depth[fy * WIDTH + fx],
                    (None, Some(n)) if nudged_column(n, dx, place) => n.depth[fy * WIDTH + fx],
                    _ => depth[fy * WIDTH + fx],
                };
                if front < o.depth {
                    continue;
                }
                out[(place.y + dy) as usize * w + (place.x + dx) as usize] = to_rgb(color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{BACKDROP_DEPTH, Fades, Layer, Palettes, compose_with_depth, sprite_depth};
    use nettai_assets::Tiles;

    /// A 16x16 glyph's part whose texel rows are colored by their row.
    fn rows_part(tiles: &Tiles, vscale: Option<i32>) -> SpritePart<'_> {
        SpritePart {
            x: 40,
            y: 30,
            width: 8,
            height: 16,
            tiles,
            first_tile: 0,
            hflip: false,
            vflip: false,
            palette: std::array::from_fn(|i| i as u16 * 0x21),
            priority: 0,
            alpha: None,
            mosaic: None,
            vscale,
            affine: None,
        }
    }

    fn row_tiles() -> Tiles {
        let mut pixels = vec![0u8; 2 * Tiles::TILE];
        for row in 0..16 {
            for col in 0..8 {
                pixels[(row / 8) * Tiles::TILE + (row % 8) * 8 + col] = 1 + row as u8 % 15;
            }
        }
        Tiles { pixels }
    }

    /// A part at its true place stays in the frame; one a whole pixel off
    /// it is, drawn at the frame's size, the part a pixel on; one half a
    /// pixel off, at twice the frame's size, is the part an output pixel on.
    #[test]
    fn a_part_between_pixels_is_drawn_where_it_truly_is() {
        let tiles = row_tiles();
        let part = rows_part(&tiles, None);
        assert!(Overlay::of(&part, sprite_depth(0, 0), [Fade::None; 2], (0, 0)).is_none());
        let empty = Layer { palettes: Palettes::Hud, ..Layer::new(3, 0) };
        let (_, depth) = compose_with_depth(0, &[&empty], &[], Fades::default());
        let drawn = |fine: (i32, i32), scale: usize| {
            let overlay = Overlay::of(&part, sprite_depth(0, 0), [Fade::None; 2], fine).unwrap();
            let place = Placement::fit(scale * WIDTH, scale * HEIGHT);
            let mut out = vec![to_rgb(0); scale * scale * WIDTH * HEIGHT];
            draw_overlays(&[overlay], &depth, None, None, &place, &mut out, scale * WIDTH);
            out
        };
        let framed = |part: &SpritePart| {
            let (pixels, _) = compose_with_depth(0, &[&empty], std::slice::from_ref(part), Fades::default());
            pixels.iter().map(|&c| to_rgb(c)).collect::<Vec<_>>()
        };
        let moved = SpritePart { x: part.x + 1, y: part.y + 1, ..part.clone() };
        assert!(drawn((0x10000, 0x10000), 1) == framed(&moved));
        // Half a pixel right and up, at twice the size: the part's own
        // pixels, scaled, an output pixel right and up.
        let (half, at_place) = (drawn((0x8000, -0x8000), 2), framed(&part));
        let w = 2 * WIDTH;
        for y in 50..100 {
            for x in 70..110 {
                assert_eq!(half[y * w + x], at_place[((y + 1) / 2) * WIDTH + (x - 1) / 2], "({x}, {y})");
            }
        }
    }

    /// Drawn at the frame's own size, an overlay is the part as the frame
    /// draws it: the same pixels, squashed and stretched alike.
    #[test]
    fn at_the_frames_size_an_overlay_is_the_frames_part() {
        let tiles = row_tiles();
        for n in [0x300, 0x200, 0x100, 0xD0, 0xE0] {
            let part = rows_part(&tiles, Some(n));
            let empty = Layer { palettes: Palettes::Hud, ..Layer::new(3, 0) };
            let (framed, _) = compose_with_depth(0, &[&empty], std::slice::from_ref(&part), Fades::default());
            let (bare, depth) = compose_with_depth(0, &[&empty], &[], Fades::default());
            let overlay = Overlay::of(&part, sprite_depth(0, 0), [Fade::None; 2], (0, 0)).unwrap();
            let place = Placement::fit(WIDTH, HEIGHT);
            let mut out: Vec<u32> = bare.iter().map(|&c| to_rgb(c)).collect();
            draw_overlays(&[overlay], &depth, None, None, &place, &mut out, WIDTH);
            let want: Vec<u32> = framed.iter().map(|&c| to_rgb(c)).collect();
            assert!(out == want, "vscale {n:#x}");
        }
    }

    /// At three times the frame, a squashed part's rows are read at the
    /// output's rows: more of its rows show than the frame's rows pick.
    #[test]
    fn at_three_times_a_squash_reads_the_outputs_rows() {
        let tiles = row_tiles();
        let part = rows_part(&tiles, Some(0x200));
        let overlay = Overlay::of(&part, sprite_depth(0, 0), [Fade::None; 2], (0, 0)).unwrap();
        let place = Placement::fit(3 * WIDTH, 3 * HEIGHT);
        let rows = |out: &[u32], w: usize| -> Vec<u32> {
            let mut seen: Vec<u32> = (0..3 * HEIGHT).map(|y| out[y * w + 3 * 42]).filter(|&c| c != 0).collect();
            seen.dedup();
            seen
        };
        let mut out = vec![0u32; 9 * WIDTH * HEIGHT];
        draw_overlays(std::slice::from_ref(&overlay), &vec![BACKDROP_DEPTH; WIDTH * HEIGHT], None, None, &place, &mut out, 3 * WIDTH);
        let mut one = vec![0u32; WIDTH * HEIGHT];
        draw_overlays(&[overlay], &vec![BACKDROP_DEPTH; WIDTH * HEIGHT], None, None, &Placement::fit(WIDTH, HEIGHT), &mut one, WIDTH);
        let frame_rows = {
            let mut seen: Vec<u32> = (0..HEIGHT).map(|y| one[y * WIDTH + 42]).filter(|&c| c != 0).collect();
            seen.dedup();
            seen
        };
        assert_eq!(frame_rows.len(), 8, "half its rows at the frame's size");
        assert!(rows(&out, 3 * WIDTH).len() > 12, "most of them at three times: {}", rows(&out, 3 * WIDTH).len());
    }

    /// Something in front of an overlay's sprite keeps its pixels.
    #[test]
    fn what_is_in_front_keeps_its_pixels() {
        let tiles = row_tiles();
        let part = rows_part(&tiles, Some(0x100));
        let overlay = Overlay { depth: sprite_depth(0, 3), ..Overlay::of(&part, 0, [Fade::None; 2], (0, 0)).unwrap() };
        let mut depth = vec![BACKDROP_DEPTH; WIDTH * HEIGHT];
        depth[35 * WIDTH + 42] = sprite_depth(0, 1);
        let mut out = vec![7u32; WIDTH * HEIGHT];
        draw_overlays(&[overlay], &depth, None, None, &Placement::fit(WIDTH, HEIGHT), &mut out, WIDTH);
        assert_eq!(out[35 * WIDTH + 42], 7);
        assert_ne!(out[35 * WIDTH + 43], 7);
    }
}
