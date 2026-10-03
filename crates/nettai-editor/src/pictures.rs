//! The packs' pictures the editor shows: each chip's icon and its custom
//! screen picture, as images. A chip's pictures are drawn by nettai-render
//! as a frame draws them (`nettai_render::pictures`: its game's pack's,
//! under its key there). Everything the editor reads of the packs'
//! graphics is here.

use iced::widget::image::Handle;
use nettai_assets::Bundle;
use nettai_battle::Content;
use nettai_content_api::ChipHandle;
use nettai_render::packs::Packs;
use nettai_render::pictures::{self, Image};
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

fn handle(i: Image) -> Handle {
    Handle::from_rgba(i.width, i.height, i.rgba)
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
        if bundles.is_empty() {
            return Ok(Pictures::default());
        }
        // (The own pack only draws a chip whose game has none loaded.)
        let packs = Packs::new(bundles.iter().collect(), nettai_battle::content::PackId(0));
        let mut chips = HashMap::new();
        for (i, d) in content.defs.chips.iter().enumerate() {
            let chip = ChipHandle(i as u16);
            let icon = pictures::chip_icon(&packs, content, chip).map(handle);
            let art = pictures::chip_art(&packs, content, chip).map(handle);
            chips.insert(d.key.clone(), ChipPictures { icon, art });
        }
        Ok(Pictures { chips })
    }

    /// The pictures of the chip with this key (as the content keys it).
    pub fn chip(&self, key: &str) -> Option<&ChipPictures> {
        self.chips.get(key)
    }
}
