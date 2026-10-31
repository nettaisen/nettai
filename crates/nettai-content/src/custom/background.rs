//! Custom backgrounds: an indexed PNG of whole tiles, mapped across in one palette.

use super::picture::Indexed;
use nettai_assets::{Background, MapEntry, Tiles};
use nettai_content_api::CustomBackground;
use std::collections::HashMap;

/// The tile number a custom background's tiles start at: past every number a GBA map entry reaches (10 bits), so
/// past every game's field's tiles, which a stage looks up after the background's (nettai-render's `Stage::tile`),
/// whichever game's field it is drawn with (the background pool).
pub const BACKGROUND_FIRST_TILE: u16 = 0x400;

/// A tile flipped across (`h`) and down (`v`).
fn flipped(t: &[u8; 64], h: bool, v: bool) -> [u8; 64] {
    std::array::from_fn(|i| {
        let (x, y) = (i % 8, i / 8);
        t[if v { 7 - y } else { y } * 8 + if h { 7 - x } else { x }]
    })
}

/// Background `def`, made from its picture, the PNG `bytes`: an indexed PNG of whole tiles in at most 16 colors (color
/// 0 the backdrop), cut into tiles numbered from [`BACKGROUND_FIRST_TILE`], each tile once (one that is another
/// flipped is that one, flipped), mapped across the picture in palette 0, its colors.
pub(super) fn background(bytes: &[u8], def: &CustomBackground) -> Result<Background, String> {
    let picture = Indexed::read(bytes)?;
    picture.whole_tiles()?;
    picture.colors_below(16)?;
    let (columns, rows) = (picture.width / 8, picture.height / 8);
    let (map_width, map_height) = match (u16::try_from(columns), u16::try_from(rows)) {
        (Ok(w), Ok(h)) => (w, h),
        _ => return Err(format!("{}x{} pixels, more tiles across or down than a map has (65535)", picture.width, picture.height)),
    };
    let mut tiles = Tiles::default();
    // Each tile so far, and each of its flips, as the tile it is drawn with and how that tile is flipped.
    let mut known: HashMap<[u8; 64], (u16, bool, bool)> = HashMap::new();
    let mut map = Vec::with_capacity(columns * rows);
    for ty in 0..rows {
        for tx in 0..columns {
            let tile = picture.tile(tx, ty);
            let (n, hflip, vflip) = match known.get(&tile) {
                Some(&found) => found,
                None => {
                    let n = u16::try_from(tiles.len())
                        .ok()
                        .and_then(|k| BACKGROUND_FIRST_TILE.checked_add(k))
                        .ok_or_else(|| format!("more than {} different tiles", u16::MAX - BACKGROUND_FIRST_TILE + 1))?;
                    tiles.push(&tile);
                    for (h, v) in [(false, false), (true, false), (false, true), (true, true)] {
                        known.entry(flipped(&tile, h, v)).or_insert((n, h, v));
                    }
                    (n, false, false)
                }
            };
            map.push(MapEntry { tile: n, hflip, vflip, palette: 0 });
        }
    }
    Ok(Background {
        tiles,
        first_tile: BACKGROUND_FIRST_TILE,
        map,
        map_width,
        map_height,
        palette: Some(picture.palette()),
        scroll: def.scroll,
        speeding: None,
        anims: Vec::new(),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::custom::indexed_png;

    pub(crate) fn def() -> CustomBackground {
        CustomBackground { module: "g:backgrounds/x".into(), file: "g:backgrounds/x.png".into(), scroll: (16, -8) }
    }

    /// A 16x8 picture: a tile, then the same tile flipped across.
    pub(crate) fn mirrored() -> Vec<u8> {
        let mut pixels = vec![0u8; 16 * 8];
        for y in 0..8 {
            for x in 0..8 {
                let c = ((x + 2 * y) % 5) as u8;
                pixels[y * 16 + x] = c;
                pixels[y * 16 + 15 - x] = c;
            }
        }
        pixels
    }

    /// A picture's tiles are each kept once: a tile that is another flipped is drawn as that one, flipped; the
    /// palette is the PNG's colors as BGR555, and its scroll the definition's.
    #[test]
    fn a_background_keeps_each_tile_once() {
        let colors = [[0, 0, 0], [255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]];
        for bits in [png::BitDepth::Four, png::BitDepth::Eight] {
            let bg = background(&indexed_png(16, 8, bits, &mirrored(), &colors), &def()).unwrap();
            assert_eq!((bg.map_width, bg.map_height, bg.first_tile, bg.scroll), (2, 1, BACKGROUND_FIRST_TILE, (16, -8)));
            assert_eq!(bg.tiles.len(), 1, "{bits:?}: the second tile is the first flipped");
            assert_eq!(bg.map[0], MapEntry { tile: 0x400, hflip: false, vflip: false, palette: 0 });
            assert_eq!(bg.map[1], MapEntry { tile: 0x400, hflip: true, vflip: false, palette: 0 });
            assert_eq!(bg.tiles.get(0).unwrap()[..8], [0, 1, 2, 3, 4, 0, 1, 2]);
            let palette = bg.palette.unwrap();
            assert_eq!(palette[..5], [0, 0x001F, 0x03E0, 0x7C00, 0x7FFF]);
            assert_eq!(palette[5..], [0; 11]);
        }
    }

    /// What a background's picture can't be is said: not indexed, not whole tiles, a color past 15.
    #[test]
    fn a_background_says_what_its_picture_isnt() {
        let colors: Vec<[u8; 3]> = (0..32).map(|i| [i * 8, 0, 0]).collect();
        let err = |png: Vec<u8>| background(&png, &def()).unwrap_err();
        assert!(err(b"not a png".to_vec()).contains("not a PNG"));
        let mut rgb = Vec::new();
        let mut enc = png::Encoder::new(&mut rgb, 8, 8);
        enc.set_color(png::ColorType::Rgb);
        enc.write_header().unwrap().write_image_data(&[0; 8 * 8 * 3]).unwrap();
        assert!(err(rgb).contains("an indexed PNG"), "an RGB picture");
        assert!(err(indexed_png(12, 8, png::BitDepth::Eight, &[0; 96], &colors)).contains("12x8 pixels, not whole 8x8 tiles"));
        let mut pixels = [0u8; 64];
        pixels[8 * 3 + 5] = 16;
        assert_eq!(err(indexed_png(8, 8, png::BitDepth::Eight, &pixels, &colors)), "pixel (5, 3) is color 16 (a palette here has 16 colors, 0 to 15)");
    }
}
