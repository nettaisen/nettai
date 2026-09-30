//! Cross data: the body overlays a Cross navi wears, and the look of a
//! Cross navi's image merging with MegaMan. The tables are in
//! `cross_generated`.

use super::SpriteId;
use super::cross_generated as tables;
use crate::setup::Navi;

/// A body overlay (actor object #0x56): a second sprite layered on a
/// navi, drawn in front of it in some of its animations and one pixel
/// further back in the others.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyOverlay {
    pub sprite: SpriteId,
    /// By the navi's animation: the overlay is drawn in front of it.
    pub in_front: &'static [bool],
}

/// Body overlay `variant`.
pub fn body_overlay(variant: u8) -> BodyOverlay {
    tables::BODY_OVERLAYS[variant as usize]
}

/// Extra height, in whole pixels, of `navi`'s image as it merges with
/// MegaMan.
pub fn merge_height(navi: Navi) -> i16 {
    tables::MERGE_HEIGHTS[navi.index()]
}
