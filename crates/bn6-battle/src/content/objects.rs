//! Object kinds' data the engine reads.

use super::SpriteId;
use serde::{Deserialize, Serialize};

/// Data of the engine's object kinds that have their own.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ObjectData {
    /// Body overlays (actor object #0x56) by variant.
    pub body_overlays: Vec<BodyOverlay>,
}

impl ObjectData {
    /// Body overlay `variant`.
    pub fn body_overlay(&self, variant: u8) -> &BodyOverlay {
        self.body_overlays.get(variant as usize).unwrap_or_else(|| panic!("body overlay {variant} is not in the content"))
    }
}

/// A body overlay (actor object #0x56): a second sprite layered on a
/// navi, drawn in front of it in some of its animations and one pixel
/// further back in the others.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyOverlay {
    /// The variant number.
    pub id: u8,
    pub sprite: SpriteId,
    /// By the navi's animation: the overlay is drawn in front of it.
    pub in_front: Vec<bool>,
}
