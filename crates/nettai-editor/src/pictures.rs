//! The packs' pictures the editor shows: each chip's icon and its custom
//! screen picture, as RGBA images. A chip's pictures are its own game's
//! pack's (the pack its root names its assets in), under its key there, as
//! the frontend draws them (`nettai_frontend::packs`). Everything the editor
//! reads of the packs' graphics is here.

use iced::widget::image::Handle;
use nettai_assets::{Bundle, Palette, Tiles};
use nettai_battle::Content;
use std::collections::HashMap;
use std::path::PathBuf;

/// A chip's pictures.
#[derive(Clone, Debug)]
pub struct ChipPictures {
    /// The 16x16 icon.
    pub icon: Option<Handle>,
    /// The custom screen's 56x48 picture.
    pub art: Option<Handle>,
}

/// The pictures of every chip the packs have, by the chip's key (as the
/// content keys it).
#[derive(Default)]
pub struct Pictures {
    chips: HashMap<String, ChipPictures>,
}

/// A BGR555 color as RGBA (index 0 of a palette is see-through).
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
    /// The pictures of `content`'s chips from `packs` (the packs the content
    /// loaded with; each chip's from its game's), or why not.
    pub fn load(content: &Content, packs: &[PathBuf]) -> Result<Pictures, String> {
        // Each pack's graphics, by the content's pack order.
        let mut bundles: Vec<Bundle> = Vec::new();
        for path in nettai_content::pack::pack_paths(content, packs) {
            let (b, _) = nettai_content::pack::load_graphics(&path)
                .map_err(|r| format!("can't load the graphics of {}: {} problems", path.display(), r.issues.len()))?;
            bundles.push(b);
        }
        let mut chips = HashMap::new();
        for d in &content.defs.chips {
            let game = content.scripts.roots.get(content.defs.root_of(&d.key).index()).map(|r| r.assets());
            let Some(pack) = game.and_then(|g| content.assets.pack(g)) else { continue };
            let Some(b) = bundles.get(pack.index()) else { continue };
            let local = nettai_content_api::keys::local(&d.key);
            let icon = b.hud.chip_icon(local).and_then(|t| image(t, 2, 2, &b.hud.icon_palette));
            let art = b.custom.chip_art(local).and_then(|a| image(&a.picture.tiles, 7, 6, &d.record.art_palette.unwrap_or(a.picture.palette)));
            chips.insert(d.key.clone(), ChipPictures { icon, art });
        }
        Ok(Pictures { chips })
    }

    /// The pictures of the chip with this key (as the content keys it).
    pub fn chip(&self, key: &str) -> Option<&ChipPictures> {
        self.chips.get(key)
    }
}
