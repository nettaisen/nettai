//! Custom sprites: an indexed PNG of a cell per frame, each cut into hardware-sprite parts after its shadow.

use super::picture::Indexed;
use nettai_assets::{Palette, SpriteFrame, SpritePart, SpriteSheet, Tiles};
use nettai_content_api::CustomSprite;
use std::collections::HashMap;

/// The shapes a hardware sprite (a part) has, width by height in pixels.
const SHAPES: [(u8, u8); 12] = [(8, 8), (16, 8), (8, 16), (16, 16), (32, 8), (8, 32), (32, 16), (16, 32), (32, 32), (64, 32), (32, 64), (64, 64)];

/// How far apart a frame's parts are cut: each square this wide of its picture is a part at most (a part of the
/// fewest pixels that holds that square's).
const BLOCK: usize = 32;

/// The most parts a frame has, its shadow too (the sprite draw's mask: nettai-render's objects).
const MOST_PARTS: usize = 32;

/// Sprite `def`, made from its picture, the PNG `bytes` (and its shadow's, `shadow`), for a sprite whose animations
/// have `frames` frames each: each frame a cell of the picture, cut into parts (its shadow first, an empty one where
/// the sprite has none), its tiles each part's in turn, row by row; frames alike share their tiles and parts. Its
/// palettes are the picture's, 16 colors each. Its numbers are its caller's to give.
pub(super) fn sprite(bytes: &[u8], shadow: Option<&[u8]>, def: &CustomSprite, frames: &[usize]) -> Result<SpriteSheet, String> {
    let picture = Indexed::read(bytes)?;
    picture.colors_below(16)?;
    let count: usize = frames.iter().sum();
    if count == 0 {
        return Err("the sprite's animations have no frames".to_string());
    }
    let (w, h) = match def.frame {
        Some((w, h)) => (usize::from(w), usize::from(h)),
        None if picture.width % count == 0 => (picture.width / count, picture.height),
        None => {
            return Err(format!(
                "{} pixels wide, not one row of {count} frames (the animations' frames) as wide as each other: say each cell's `frame = {{ width, height }}`",
                picture.width
            ));
        }
    };
    if picture.width % w != 0 || picture.height % h != 0 {
        return Err(format!("{}x{} pixels, not whole {w}x{h} cells", picture.width, picture.height));
    }
    let columns = picture.width / w;
    let cells = columns * (picture.height / h);
    if cells < count {
        return Err(format!("{cells} cells of {w}x{h}, and the animations have {count} frames"));
    }
    let cell = |k: usize| picture.crop(k % columns * w, k / columns * h, k % columns * w + w, k / columns * h + h);
    if let Some(k) = (count..cells).find(|&k| cell(k).pixels.iter().any(|&p| p != 0)) {
        return Err(format!("cell {k} isn't blank, and the animations have {count} frames (cells 0 to {})", count - 1));
    }
    let origin = def.origin.map_or((w as i32 / 2, h as i32), |(x, y)| (i32::from(x), i32::from(y)));
    let palettes = picture.palettes();
    let (shadow_part, shadow_tiles) = match shadow {
        Some(bytes) => shadow_of(bytes, &palettes[0]).map_err(|e| format!("its shadow: {e}"))?,
        None => (SpritePart { width: 8, height: 8, ..Default::default() }, vec![[0; 64]]),
    };
    let mut sheet = SpriteSheet { palette_sets: vec![palettes], ..Default::default() };
    let mut known: HashMap<Vec<u8>, u16> = HashMap::new();
    let mut each = Vec::with_capacity(count);
    for k in 0..count {
        let cell = cell(k);
        let n = match known.get(&cell.pixels) {
            Some(&n) => n,
            None => {
                let mut parts = vec![shadow_part];
                let mut tiles = Tiles::default();
                for t in &shadow_tiles {
                    tiles.push(t);
                }
                cut(&cell, origin, &mut parts, &mut tiles).map_err(|e| format!("frame {k} (cell {k}): {e}"))?;
                let n = sheet.tilesets.len() as u16;
                sheet.tilesets.push(tiles);
                sheet.part_lists.push(parts);
                known.insert(cell.pixels, n);
                n
            }
        };
        each.push(SpriteFrame { tileset: n, palette_set: 0, parts: n });
    }
    let mut each = each.into_iter();
    sheet.animations = frames.iter().map(|&n| each.by_ref().take(n).collect()).collect();
    Ok(sheet)
}

/// A sprite's shadow, the PNG `bytes`: its part (centered on the object's place) and its tiles, its colors those of
/// the sprite's first palette `palette` (color 0 clear).
fn shadow_of(bytes: &[u8], palette: &Palette) -> Result<(SpritePart, Vec<[u8; 64]>), String> {
    let picture = Indexed::read(bytes)?;
    let shape = SHAPES.iter().find(|&&(w, h)| (usize::from(w), usize::from(h)) == (picture.width, picture.height));
    let Some(&(width, height)) = shape else {
        return Err(format!("{}x{} pixels, not a shape a hardware sprite has (8x8, 16x8, 8x16, ... 64x64)", picture.width, picture.height));
    };
    // (Each color number of its as the sprite's: color 0 clear, any other the first of the sprite's colors it is.)
    let mut numbers = vec![0u8; picture.colors.len().max(1)];
    for (i, n) in numbers.iter_mut().enumerate().skip(1) {
        *n = match palette.iter().skip(1).position(|&c| c == picture.colors[i]) {
            Some(p) => p as u8 + 1,
            None if !picture.pixels.contains(&(i as u8)) => 0,
            None => return Err(format!("color {i} ({:#06x}) isn't one of the sprite's first 16", picture.colors[i])),
        };
    }
    let mapped = Indexed { pixels: picture.pixels.iter().map(|&p| numbers.get(usize::from(p)).copied().unwrap_or(0)).collect(), ..picture };
    let tiles = (0..usize::from(height) / 8).flat_map(|ty| (0..usize::from(width) / 8).map(move |tx| (tx, ty))).map(|(tx, ty)| mapped.tile(tx, ty)).collect();
    let part = SpritePart { tile: 0, x: -(width as i32 / 2) as i8, y: -(height as i32 / 2) as i8, width, height, ..Default::default() };
    Ok((part, tiles))
}

/// Cut frame picture `cell`, whose object stands at `origin` in it, into parts after `parts` (its shadow), their
/// tiles after `tiles`: each [`BLOCK`]-pixel square of its opaque pixels' box, from the box's top left, the
/// smallest shape that holds the square's opaque pixels, its pixels outside the square clear (so no pixel is drawn
/// twice).
fn cut(cell: &Indexed, origin: (i32, i32), parts: &mut Vec<SpritePart>, tiles: &mut Tiles) -> Result<(), String> {
    let Some((x0, y0, x1, y1)) = cell.opaque_within((0, 0, cell.width, cell.height)) else { return Ok(()) };
    for by in (y0..y1).step_by(BLOCK) {
        for bx in (x0..x1).step_by(BLOCK) {
            let block = (bx, by, (bx + BLOCK).min(x1), (by + BLOCK).min(y1));
            let Some((cx0, cy0, cx1, cy1)) = cell.opaque_within(block) else { continue };
            let &(width, height) = SHAPES
                .iter()
                .filter(|&&(w, h)| usize::from(w) >= cx1 - cx0 && usize::from(h) >= cy1 - cy0)
                .min_by_key(|&&(w, h)| u16::from(w) * u16::from(h))
                .expect("a block fits the largest shape");
            let place = |at: usize, from: i32, what: &str| {
                i8::try_from(at as i32 - from).map_err(|_| format!("a pixel {what} {} from the origin (a part is within 128 of it)", at as i32 - from))
            };
            let (x, y) = (place(cx0, origin.0, "across")?, place(cy0, origin.1, "down")?);
            if parts.len() == MOST_PARTS {
                return Err(format!("more than {} parts (each {BLOCK}x{BLOCK} square of its pixels is one)", MOST_PARTS - 1));
            }
            parts.push(SpritePart { tile: tiles.len() as u16, x, y, width, height, ..Default::default() });
            for ty in 0..usize::from(height) / 8 {
                for tx in 0..usize::from(width) / 8 {
                    let tile: [u8; 64] = std::array::from_fn(|i| {
                        let (px, py) = (cx0 + tx * 8 + i % 8, cy0 + ty * 8 + i / 8);
                        if px < block.2 && py < block.3 { cell.pixels[py * cell.width + px] } else { 0 }
                    });
                    tiles.push(&tile);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custom::indexed_png;

    fn sprite_def(frame: Option<(u16, u16)>, origin: Option<(i16, i16)>) -> CustomSprite {
        CustomSprite { module: "g:x".into(), file: "g:x.png".into(), frame, origin, shadow: None }
    }

    /// A picture of `cells` cells of `w` x `h` in a row, cell `k` drawn by `draw(k, x, y)`.
    fn strip(cells: usize, w: usize, h: usize, draw: impl Fn(usize, usize, usize) -> u8) -> Vec<u8> {
        let width = cells * w;
        let pixels: Vec<u8> = (0..width * h).map(|i| draw(i % width / w, i % width % w, i / width)).collect();
        let colors: Vec<[u8; 3]> = (0..32).map(|i| [i * 8, 255 - i * 8, 0]).collect();
        indexed_png(width as u32, h as u32, png::BitDepth::Eight, &pixels, &colors)
    }

    /// A sprite's frames are its picture's cells, in its animations' order, each cut into parts after its shadow (an
    /// empty part where it has none): each 32-pixel square of its pixels the smallest shape that holds them, placed
    /// from the origin (else the middle of the cell's bottom); frames alike share their tiles and parts, and each 16
    /// of the picture's colors is a palette.
    #[test]
    fn a_sprite_is_its_cells_cut_into_parts() {
        // Three 48x24 cells: a 40x10 bar at (4, 14) in the first and the last, a 8x8 dot at (20, 8) in the middle.
        let png = strip(3, 48, 24, |k, x, y| match k {
            1 => u8::from((20..28).contains(&x) && (8..16).contains(&y)) * 3,
            _ => u8::from((4..44).contains(&x) && (14..24).contains(&y)) * (1 + (x % 2) as u8),
        });
        let sheet = sprite(&png, None, &sprite_def(Some((48, 24)), None), &[2, 1]).unwrap();
        assert_eq!(sheet.animations.len(), 2);
        let frames: Vec<u16> = sheet.animations.iter().flatten().map(|f| f.tileset).collect();
        assert_eq!(frames, [0, 1, 0], "the first and last frames are alike");
        assert_eq!(sheet.palette_sets, [vec![std::array::from_fn(|i| nettai_assets::bgr555([i as u8 * 8, 255 - i as u8 * 8, 0])), std::array::from_fn(|i| nettai_assets::bgr555([(16 + i as u8) * 8, 255 - (16 + i as u8) * 8, 0]))]]);
        let bar = &sheet.part_lists[0];
        let shape = |p: &SpritePart| (p.tile, p.x, p.y, p.width, p.height);
        // The empty shadow, then the bar's 32 pixels and its last 8, from the origin (24, 24).
        assert_eq!(bar.iter().map(shape).collect::<Vec<_>>(), [(0, 0, 0, 8, 8), (1, -20, -10, 32, 16), (9, 12, -10, 8, 16)]);
        assert_eq!(sheet.tilesets[0].len(), 1 + 8 + 2);
        let dot = &sheet.part_lists[1];
        assert_eq!(dot.iter().map(shape).collect::<Vec<_>>(), [(0, 0, 0, 8, 8), (1, -4, -16, 8, 8)]);
        assert_eq!(sheet.tilesets[1].get(1).unwrap(), &[3; 64]);
        // A part starts at its first opaque row (the cell's 14th): its first row of tiles is the bar's rows 14 to 21,
        // its second the last two (22 and 23), the rest clear.
        assert_eq!(sheet.tilesets[0].get(1).unwrap(), &[1, 2].repeat(32)[..]);
        let second_row = sheet.tilesets[0].get(1 + 4).unwrap();
        assert_eq!(second_row[..16], [1, 2].repeat(8)[..]);
        assert_eq!(second_row[16..], [0; 48]);
        // One row of as many cells as frames needs no `frame`; another origin moves the parts.
        let sheet = sprite(&png, None, &sprite_def(None, Some((0, 0))), &[3]).unwrap();
        assert_eq!(sheet.part_lists[1][1].x, 20);
    }

    /// A sprite's shadow is its first part, centered on the object, in the sprite's colors.
    #[test]
    fn a_sprites_shadow_is_its_first_part() {
        let png = strip(1, 16, 16, |_, x, _| u8::from(x == 3) * 2);
        let colors: Vec<[u8; 3]> = (0..32).map(|i| [i * 8, 255 - i * 8, 0]).collect();
        let shadow = indexed_png(16, 8, png::BitDepth::Four, &[[0u8, 1].repeat(8), [1u8; 16].to_vec()].concat().repeat(4), &[[9, 9, 9], colors[2]]);
        let sheet = sprite(&png, Some(&shadow), &sprite_def(None, None), &[1]).unwrap();
        let parts = &sheet.part_lists[0];
        assert_eq!((parts[0].x, parts[0].y, parts[0].width, parts[0].height, parts[0].tile), (-8, -4, 16, 8, 0));
        assert_eq!(parts[1].tile, 2, "the body's tiles after the shadow's two");
        assert_eq!(sheet.tilesets[0].get(0).unwrap()[..10], [0, 2, 0, 2, 0, 2, 0, 2, 2, 2], "the shadow's color 1 is the sprite's 2");
        let odd = indexed_png(8, 8, png::BitDepth::Eight, &[1; 64], &[[0, 0, 0], [1, 2, 3]]);
        let e = sprite(&png, Some(&odd), &sprite_def(None, None), &[1]).unwrap_err();
        assert!(e.contains("its shadow: color 1 (0x0000) isn't one of the sprite's first 16"), "{e}");
        let e = sprite(&png, Some(&indexed_png(24, 8, png::BitDepth::Eight, &[0; 192], &[[0; 3]])), &sprite_def(None, None), &[1]).unwrap_err();
        assert!(e.contains("24x8 pixels, not a shape a hardware sprite has"), "{e}");
    }

    /// What a sprite's picture can't be is said.
    #[test]
    fn a_sprite_says_what_its_picture_isnt() {
        let png = strip(3, 16, 16, |k, x, _| u8::from(k == 2 && x == 0));
        let e = |frames: &[usize], frame| sprite(&png, None, &sprite_def(frame, None), frames).unwrap_err();
        assert!(e(&[5], None).contains("48 pixels wide, not one row of 5 frames"));
        assert_eq!(e(&[2], Some((16, 16))), "cell 2 isn't blank, and the animations have 2 frames (cells 0 to 1)");
        assert_eq!(e(&[4], Some((16, 16))), "3 cells of 16x16, and the animations have 4 frames");
        assert_eq!(e(&[3], Some((20, 16))), "48x16 pixels, not whole 20x16 cells");
        let far = strip(1, 200, 8, |_, x, _| u8::from(x == 199));
        let e = sprite(&far, None, &sprite_def(None, Some((0, 0))), &[1]).unwrap_err();
        assert_eq!(e, "frame 0 (cell 0): a pixel across 199 from the origin (a part is within 128 of it)");
    }
}
