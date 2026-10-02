//! The pack's pictures the editor shows: each chip's icon and its custom
//! screen picture, by the chip's key, as RGBA images. Everything the editor
//! reads of a pack's graphics is here, so a change to how packs load (several
//! packs, one per game root) is a change to this module alone.

use iced::widget::image::Handle;
use nettai_assets::{Palette, Tiles};
use std::collections::HashMap;
use std::path::Path;

/// A chip's pictures.
#[derive(Clone, Debug)]
pub struct ChipPictures {
    /// The 16x16 icon.
    pub icon: Option<Handle>,
    /// The custom screen's 56x48 picture.
    pub art: Option<Handle>,
}

/// The pictures of every chip the pack has, by chip key.
#[derive(Default)]
pub struct Pictures {
    chips: HashMap<String, ChipPictures>,
}

/// A BGR555 colour as RGBA (index 0 of a palette is see-through).
fn rgba(c: u16) -> [u8; 4] {
    let five = |v: u16| ((v & 31) << 3 | (v & 31) >> 2) as u8;
    [five(c), five(c >> 5), five(c >> 10), 0xFF]
}

/// `tiles` laid out row-major, `w` by `h` tiles, in `palette`.
fn image(tiles: &Tiles, w: usize, h: usize, palette: &Palette) -> Option<Handle> {
    if tiles.len() < w * h {
        return None;
    }
    let (pw, ph) = (w * 8, h * 8);
    let mut out = vec![0u8; pw * ph * 4];
    for t in 0..w * h {
        let tile = tiles.get(t)?;
        let (tx, ty) = (t % w * 8, t / w * 8);
        for (i, &p) in tile.iter().enumerate() {
            if p == 0 {
                continue;
            }
            let (x, y) = (tx + i % 8, ty + i / 8);
            out[(y * pw + x) * 4..][..4].copy_from_slice(&rgba(palette[p as usize]));
        }
    }
    Some(Handle::from_rgba(pw as u32, ph as u32, out))
}

impl Pictures {
    /// The pictures of the pack in `pack` (its graphics), or why not.
    pub fn load(pack: &Path) -> Result<Pictures, String> {
        let (bundle, _report) = nettai_content::pack::load_graphics(pack)
            .map_err(|r| format!("can't load the graphics of {}: {}", pack.display(), r.issues.len()))?;
        let hud = &bundle.hud;
        let mut chips: HashMap<String, ChipPictures> = HashMap::new();
        for icon in &hud.chip_icons {
            let handle = image(&icon.tiles, 2, 2, &hud.icon_palette);
            chips.entry(icon.key.clone()).or_insert(ChipPictures { icon: None, art: None }).icon = handle;
        }
        for art in &bundle.custom.chip_art {
            let handle = image(&art.picture.tiles, 7, 6, &art.picture.palette);
            chips.entry(art.key.clone()).or_insert(ChipPictures { icon: None, art: None }).art = handle;
        }
        Ok(Pictures { chips })
    }

    /// The pictures of the chip with this key (as content keys it).
    pub fn chip(&self, key: &str) -> Option<&ChipPictures> {
        self.chips.get(key).or_else(|| self.chips.get(nettai_content_api::keys::local(key)))
    }
}
