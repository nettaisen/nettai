//! The pictures the creator shows of a game's definitions, from its pack
//! as a frame draws them (`nettai_render::pictures`): a chip's icon and its
//! picture in the chip window, a form's face (a Cross's, a soul's) and a
//! navi's. Each is made the first time it is asked for, and kept.

use crate::games::Ready;
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle};
use nettai_render::packs::PackGraphics;
use nettai_render::pictures::{self, Image};
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A game's pictures, made as they are asked for.
pub struct Pictures {
    ready: Rc<Ready>,
    packs: PackGraphics,
    icons: RefCell<HashMap<ChipHandle, slint::Image>>,
    art: RefCell<HashMap<ChipHandle, slint::Image>>,
    faces: RefCell<HashMap<FormHandle, slint::Image>>,
    navis: RefCell<HashMap<NaviHandle, slint::Image>>,
}

/// An RGBA picture as the window shows one (none: an empty image).
fn image(p: Option<Image>) -> slint::Image {
    let Some(p) = p else { return slint::Image::default() };
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&p.rgba, p.width, p.height);
    slint::Image::from_rgba8(buffer)
}

/// What `get` makes of `key`, made once.
fn kept<K: std::hash::Hash + Eq + Copy>(map: &RefCell<HashMap<K, slint::Image>>, key: K, make: impl FnOnce() -> Option<Image>) -> slint::Image {
    if let Some(i) = map.borrow().get(&key) {
        return i.clone();
    }
    let i = image(make());
    map.borrow_mut().insert(key, i.clone());
    i
}

impl Pictures {
    pub fn new(ready: Rc<Ready>) -> Pictures {
        let packs = ready.loaded.graphics.packs();
        Pictures { ready, packs, icons: Default::default(), art: Default::default(), faces: Default::default(), navis: Default::default() }
    }

    /// A chip's 16x16 icon.
    pub fn icon(&self, chip: ChipHandle) -> slint::Image {
        kept(&self.icons, chip, || pictures::chip_icon(&self.packs.packs(), self.ready.content(), chip))
    }

    /// A chip's 56x48 picture.
    pub fn art(&self, chip: ChipHandle) -> slint::Image {
        kept(&self.art, chip, || pictures::chip_art(&self.packs.packs(), self.ready.content(), chip))
    }

    /// A form's 32x16 face.
    pub fn face(&self, form: FormHandle) -> slint::Image {
        kept(&self.faces, form, || pictures::form_face(&self.packs.packs(), self.ready.content(), form))
    }

    /// A navi's face: its own, else its starting form's.
    pub fn navi(&self, navi: NaviHandle) -> slint::Image {
        let content = self.ready.content();
        kept(&self.navis, navi, || {
            let packs = self.packs.packs();
            let own = nettai_render::lookups::navi_face(&packs, content, navi, &mut Default::default());
            own.and_then(|(tiles, palettes, _)| pictures::image(tiles, 4, 2, palettes.first()?)).or_else(|| {
                let form = nettai_match::Side::fresh_stats(content, navi).starting_form;
                pictures::form_face(&packs, content, form)
            })
        })
    }
}
