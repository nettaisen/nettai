//! Custom backgrounds: an indexed PNG of whole tiles, mapped across in one palette; a picture of several, one below
//! another, is an animation of its tiles and its colors.

use super::picture::Indexed;
use nettai_assets::{AnimTarget, Background, GfxAnim, GfxAnimFrame, MapEntry, Palette, Speeding, Tiles};
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

/// One place of the map: its tile in each picture, in turn.
type Run = Vec<[u8; 64]>;

/// Background `def`, made from its picture, the PNG `bytes`: as many pictures as `def.frames` has durations (one
/// without them), one below another, each of whole tiles in the colors of one 16 of the PNG's palette (color 0 of
/// each the backdrop). Each place of the map has its tiles in turn, a run: each run is kept once (one that is
/// another flipped is that one, flipped), the runs that stay the same first, from tile number
/// [`BACKGROUND_FIRST_TILE`]; the first picture is the background, and where the pictures differ an animation
/// replaces the tiles of the runs that change, or palette 0 (the colors each picture is in), on the pictures'
/// schedule. Its scroll is `def.scroll`, or with `def.speed_up` one that speeds up to it from standing still.
pub(super) fn background(bytes: &[u8], def: &CustomBackground) -> Result<Background, String> {
    let picture = Indexed::read(bytes)?;
    let count = def.frames.len().max(1);
    if picture.height % count != 0 {
        return Err(format!("{} pixels tall, not {count} pictures one below another (its `frames`) as tall as each other", picture.height));
    }
    let height = picture.height / count;
    let pictures: Vec<Indexed> = (0..count).map(|f| picture.crop(0, f * height, picture.width, (f + 1) * height)).collect();
    pictures[0].whole_tiles().map_err(|e| if count > 1 { format!("each picture is {e}") } else { e })?;
    let palettes = picture.palettes();
    let mut colors: Vec<Palette> = Vec::with_capacity(count);
    for (f, p) in pictures.iter().enumerate() {
        let group = p.group().map_err(|e| if count > 1 { format!("picture {f}: {e}") } else { e })?;
        colors.push(palettes.get(group).copied().unwrap_or([0; 16]));
    }
    let (columns, rows) = (picture.width / 8, height / 8);
    let (map_width, map_height) = match (u16::try_from(columns), u16::try_from(rows)) {
        (Ok(w), Ok(h)) => (w, h),
        _ => return Err(format!("{}x{height} pixels, more tiles across or down than a map has (65535)", picture.width)),
    };
    // Each run once, in the order first met, and each place's: its run and how that run is flipped.
    let mut runs: Vec<Run> = Vec::new();
    let mut known: HashMap<Run, (usize, bool, bool)> = HashMap::new();
    let mut places = Vec::with_capacity(columns * rows);
    for ty in 0..rows {
        for tx in 0..columns {
            let run: Run = pictures.iter().map(|p| p.tile(tx, ty).map(|c| c % 16)).collect();
            let place = match known.get(&run) {
                Some(&found) => found,
                None => {
                    let n = runs.len();
                    for (h, v) in [(false, false), (true, false), (false, true), (true, true)] {
                        known.entry(run.iter().map(|t| flipped(t, h, v)).collect()).or_insert((n, h, v));
                    }
                    runs.push(run);
                    (n, false, false)
                }
            };
            places.push(place);
        }
    }
    // (The runs that stay the same, then those that change: an animation replaces the last ones' tiles.)
    let changes = |r: &Run| r.iter().any(|t| *t != r[0]);
    let mut order: Vec<usize> = (0..runs.len()).collect();
    order.sort_by_key(|&n| changes(&runs[n]));
    let mut number = vec![0u16; runs.len()];
    for (k, &n) in order.iter().enumerate() {
        number[n] = u16::try_from(k)
            .ok()
            .and_then(|k| BACKGROUND_FIRST_TILE.checked_add(k))
            .ok_or_else(|| format!("more than {} different tiles", u16::MAX - BACKGROUND_FIRST_TILE + 1))?;
    }
    let mut tiles = Tiles::default();
    for &n in &order {
        tiles.push(&runs[n][0]);
    }
    let map = places.into_iter().map(|(n, hflip, vflip)| MapEntry { tile: number[n], hflip, vflip, palette: 0 }).collect();
    let still = order.iter().take_while(|&&n| !changes(&runs[n])).count();
    let repeat_from = def.looping.then_some(0);
    let delay = |f: usize| def.frames.get(f).copied().unwrap_or(1);
    let mut anims = Vec::new();
    if still < order.len() {
        let frames = (0..count)
            .map(|f| {
                let mut t = Tiles::default();
                for &n in &order[still..] {
                    t.push(&runs[n][f]);
                }
                GfxAnimFrame { tiles: t, delay: delay(f), ..Default::default() }
            })
            .collect();
        let first = BACKGROUND_FIRST_TILE + still as u16;
        anims.push(GfxAnim { target: AnimTarget::Tiles { first, count: (order.len() - still) as u16 }, frames, repeat_from });
    }
    if colors.iter().any(|c| *c != colors[0]) {
        let frames = colors.iter().enumerate().map(|(f, c)| GfxAnimFrame { palettes: vec![*c], delay: delay(f), ..Default::default() }).collect();
        anims.push(GfxAnim { target: AnimTarget::Palettes { first: 0, count: 1 }, frames, repeat_from });
    }
    // (A scroll that speeds up: `scroll` is its top speed, in 1/16 pixel, the model's in 1/65536; it gets there in
    // `speed_up` frames, a step at least 1 a frame.)
    let speeding = def.speed_up.map(|frames| {
        let axis = |v: i32| {
            let top = v * 0x1000;
            let step = top / i32::from(frames);
            (if step == 0 { top.signum() } else { step }, top.abs())
        };
        let ((sx, tx), (sy, ty)) = (axis(def.scroll.0), axis(def.scroll.1));
        Speeding { step: (sx, sy), top: (tx, ty), from_battle: false }
    });
    Ok(Background {
        tiles,
        first_tile: BACKGROUND_FIRST_TILE,
        map,
        map_width,
        map_height,
        palette: Some(colors[0]),
        scroll: if speeding.is_some() { (0, 0) } else { def.scroll },
        speeding,
        anims,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::custom::indexed_png;

    pub(crate) fn def() -> CustomBackground {
        CustomBackground { source: "g/backgrounds.toml".into(), file: "g:backgrounds/x.png".into(), scroll: (16, -8), ..Default::default() }
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

    /// A picture of several, one below another, is an animation: the places whose tiles stay the same are tiles of
    /// their own, the places whose tiles change come last and an animation replaces their tiles; each picture is in
    /// the colors of the 16 its pixels use, and an animation replaces palette 0 where they differ.
    #[test]
    fn pictures_one_below_another_animate() {
        let colors: Vec<[u8; 3]> = (0..32).map(|i| [i * 8, 0, 0]).collect();
        // Two 16x8 pictures: the left tile 1s in both, the right tile 2s then 3s (in the second 16's colors: 18, 19).
        let pixels: Vec<u8> = (0..16 * 16).map(|i| match (i % 16 < 8, i / 16 < 8) { (true, true) => 1, (false, true) => 2, (true, false) => 17, (false, false) => 19 }).collect();
        let two = CustomBackground { frames: vec![10, 20], looping: true, ..def() };
        let bg = background(&indexed_png(16, 16, png::BitDepth::Eight, &pixels, &colors), &two).unwrap();
        assert_eq!((bg.map_width, bg.map_height), (2, 1));
        assert_eq!(bg.map.iter().map(|e| e.tile).collect::<Vec<_>>(), [0x400, 0x401], "the tile that changes last");
        assert_eq!((bg.tiles.get(0).unwrap(), bg.tiles.get(1).unwrap()), (&[1; 64][..], &[2; 64][..]));
        let tiles = &bg.anims[0];
        assert_eq!(tiles.target, AnimTarget::Tiles { first: 0x401, count: 1 });
        assert_eq!(tiles.frames.iter().map(|f| (f.tiles.get(0).unwrap()[0], f.delay)).collect::<Vec<_>>(), [(2, 10), (3, 20)]);
        assert_eq!(tiles.repeat_from, Some(0));
        let shown = &bg.anims[1];
        assert_eq!(shown.target, AnimTarget::Palettes { first: 0, count: 1 });
        assert_eq!(shown.frames.iter().map(|f| f.palettes[0][1]).collect::<Vec<_>>(), [nettai_assets::bgr555([8, 0, 0]), nettai_assets::bgr555([136, 0, 0])]);
        assert_eq!(bg.palette.unwrap()[2], nettai_assets::bgr555([16, 0, 0]));
        // A still one in the second 16's colors is drawn in them; a speed-up is a speeding scroll to its `scroll`.
        let still = background(&indexed_png(8, 8, png::BitDepth::Eight, &[16 + 3; 64], &colors), &CustomBackground { speed_up: Some(60), ..def() }).unwrap();
        assert_eq!((still.palette.unwrap()[3], still.anims.len()), (nettai_assets::bgr555([152, 0, 0]), 0));
        assert_eq!(still.scroll, (0, 0));
        assert_eq!(still.speeding, Some(Speeding { step: (1092, -546), top: (0x1_0000, 0x8000), from_battle: false }));
    }

    /// What a background's picture can't be is said: not indexed, not whole tiles, colors of two 16s, not as many
    /// pictures as its frames.
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
        pixels[1] = 4;
        assert_eq!(
            err(indexed_png(8, 8, png::BitDepth::Eight, &pixels, &colors)),
            "its colors are of two 16s of the palette: color 4 at (1, 0) (colors 0 to 15) and color 16 at (5, 3) (colors 16 to 31); a picture is in one"
        );
        let three = CustomBackground { frames: vec![1, 1, 1], ..def() };
        assert_eq!(background(&indexed_png(8, 16, png::BitDepth::Eight, &[0; 128], &colors), &three).unwrap_err(), "16 pixels tall, not 3 pictures one below another (its `frames`) as tall as each other");
        let two = CustomBackground { frames: vec![1, 1], ..def() };
        let e = background(&indexed_png(8, 16, png::BitDepth::Eight, &[[1; 64], [17; 64]].concat().iter().enumerate().map(|(i, &c)| if i == 70 { 2 } else { c }).collect::<Vec<_>>(), &colors), &two).unwrap_err();
        assert!(e.starts_with("picture 1: its colors are of two 16s"), "{e}");
    }
}
