//! Custom assets (docs/design/asset-formats.md §7): what a game's content makes from files of its own rather than
//! its ROMs (`Content::custom_assets`, which its modules define: `asset.background("my-stage", { file =
//! "./my_stage.png" })`). Each is made into the model the ROMs' assets are extracted into and added to the game's
//! assets after them, under its name, numbered past the ROMs' ([`add`]); a frontend then draws it as it draws theirs.
//! Its pictures are GBA-shaped, as the ROMs' are: indexed, 16 colors to a palette, cut into 8x8 tiles.

use nettai_assets::{Background, Bundle, MapEntry, Tiles};
use nettai_content_api::{AssetIndex, CustomBackground, keys, packs};
use std::collections::HashMap;

/// The tile number a custom background's tiles start at: past every number a GBA map entry reaches (10 bits), so
/// past every game's field's tiles, which a stage looks up after the background's (nettai-render's `Stage::tile`),
/// whichever game's field it is drawn with (the background pool).
pub const BACKGROUND_FIRST_TILE: u16 = 0x400;

/// Add content `c`'s custom assets to its game's assets as its ROMs gave them: their graphics `graphics` and their
/// index `index`. An error names the asset and what is wrong with it: its file isn't there or isn't what its kind
/// takes, or the ROMs have an asset by its name.
pub fn add(c: &nettai_battle::Content, graphics: &mut Bundle, index: &mut AssetIndex) -> Result<(), String> {
    let dirs = &c.scripts.dirs.0;
    for (name, def) in &c.custom_assets.backgrounds {
        let at = format!("{}.luau: background {name:?}", keys::module_path(&def.module));
        if index.backgrounds.contains_key(name) {
            return Err(format!("{at}: the ROMs have a background of that name (a custom one takes a name of its own)"));
        }
        let bg = read(dirs, &def.file).and_then(|bytes| background(&bytes, def)).map_err(|e| format!("{at}: {}: {e}", def.file))?;
        let id = u8::try_from(graphics.backgrounds.len()).map_err(|_| format!("{at}: a game has at most 256 backgrounds"))?;
        graphics.backgrounds.push(Some(bg));
        index.backgrounds.insert(name.clone(), id);
    }
    Ok(())
}

/// File `file` of the content's packs (`exe6:backgrounds/my_stage.png`), read.
fn read(dirs: &packs::Dirs, file: &str) -> Result<Vec<u8>, String> {
    let path = dirs.asset_file(file).ok_or_else(|| format!("the content has no folder for its pack {:?}", keys::root_of(file).unwrap_or(file)))?;
    nettai_files::read(&path).map_err(|e| format!("{e}"))
}

/// A picture as an indexed PNG holds it: its size, each pixel's color number (row by row), and its palette's
/// colors as BGR555.
struct Indexed {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
    colors: Vec<u16>,
}

impl Indexed {
    /// The PNG `bytes`, which must be indexed (a palette's color numbers, of any bit depth).
    fn read(bytes: &[u8]) -> Result<Indexed, String> {
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
    fn whole_tiles(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 || self.width % 8 != 0 || self.height % 8 != 0 {
            return Err(format!("{}x{} pixels, not whole 8x8 tiles (each side a multiple of 8)", self.width, self.height));
        }
        Ok(())
    }

    /// Whether every pixel is one of the palette's first `count` colors.
    fn colors_below(&self, count: u8) -> Result<(), String> {
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
    fn tile(&self, tx: usize, ty: usize) -> [u8; 64] {
        std::array::from_fn(|i| self.pixels[(ty * 8 + i / 8) * self.width + tx * 8 + i % 8])
    }

    /// Its first 16 colors as a palette (a color it hasn't is black).
    fn palette(&self) -> nettai_assets::Palette {
        std::array::from_fn(|i| self.colors.get(i).copied().unwrap_or(0))
    }
}

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
fn background(bytes: &[u8], def: &CustomBackground) -> Result<Background, String> {
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
mod tests {
    use super::*;

    /// An indexed PNG of `width` x `height` pixels of `bits` bits each, the color numbers `pixels` (row by row), its
    /// palette `colors`.
    pub(crate) fn indexed_png(width: u32, height: u32, bits: png::BitDepth, pixels: &[u8], colors: &[[u8; 3]]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, width, height);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(bits);
        enc.set_palette(colors.concat());
        let per_byte = 8 / bits as usize;
        let line = (width as usize).div_ceil(per_byte);
        let mut data = vec![0u8; line * height as usize];
        for (i, &p) in pixels.iter().enumerate() {
            let (x, y) = (i % width as usize, i / width as usize);
            data[y * line + x / per_byte] |= p << (8 - bits as usize * (x % per_byte + 1));
        }
        enc.write_header().unwrap().write_image_data(&data).unwrap();
        out
    }

    fn def() -> CustomBackground {
        CustomBackground { module: "g:backgrounds/x".into(), file: "g:backgrounds/x.png".into(), scroll: (16, -8) }
    }

    /// A 16x8 picture: a tile, then the same tile flipped across.
    fn mirrored() -> Vec<u8> {
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

    /// A content's custom backgrounds come after its ROMs', each numbered past them under its name, read from its
    /// pack's folder; a name the ROMs have, or a file that isn't there, is said with the module that defines it.
    #[test]
    fn custom_backgrounds_come_after_the_roms() {
        let dir = std::env::temp_dir().join(format!("nettai-custom-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("backgrounds")).unwrap();
        std::fs::write(dir.join("backgrounds/x.png"), indexed_png(16, 8, png::BitDepth::Four, &mirrored(), &[[0, 0, 0]; 5])).unwrap();
        let mut c = nettai_battle::Content::default();
        c.scripts.dirs.0 = packs::Dirs([("g".to_string(), dir.clone())].into());
        c.custom_assets.backgrounds.insert("sky".into(), def());
        let mut graphics = Bundle { backgrounds: vec![Some(Background::default()), None], ..Default::default() };
        let mut index = AssetIndex::default();
        index.backgrounds.insert("comp".into(), 0);
        add(&c, &mut graphics, &mut index).unwrap();
        assert_eq!(index.backgrounds["sky"], 2, "past the ROMs' numbers, holes too");
        assert_eq!(graphics.background(2).map(|b| (b.map_width, b.first_tile)), Some((2, BACKGROUND_FIRST_TILE)));
        // A name the ROMs have.
        let mut index = AssetIndex::default();
        index.backgrounds.insert("sky".into(), 0);
        let e = add(&c, &mut Bundle::default(), &mut index).unwrap_err();
        assert_eq!(e, "g/backgrounds/x.luau: background \"sky\": the ROMs have a background of that name (a custom one takes a name of its own)");
        // A file that isn't there.
        c.custom_assets.backgrounds.get_mut("sky").unwrap().file = "g:backgrounds/gone.png".into();
        let e = add(&c, &mut Bundle::default(), &mut AssetIndex::default()).unwrap_err();
        assert!(e.starts_with("g/backgrounds/x.luau: background \"sky\": g:backgrounds/gone.png: "), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
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
