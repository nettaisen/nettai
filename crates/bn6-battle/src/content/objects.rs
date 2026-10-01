//! Object kinds' data the engine reads.

use super::SpriteId;
use serde::{Deserialize, Serialize};

/// Data of the engine's object kinds that have their own.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ObjectData {
    /// Body overlays (actor object #0x56) by variant.
    pub body_overlays: Vec<BodyOverlay>,
    /// How a field object looks by its NameID (`byte_8021220`, NameIDs
    /// 0xCD..=0xFF), as `sub_800F26C` gives it: what DustMan throws.
    pub name_looks: Vec<NameLook>,
}

impl ObjectData {
    /// Body overlay `variant`.
    pub fn body_overlay(&self, variant: u8) -> &BodyOverlay {
        self.body_overlays.get(variant as usize).unwrap_or_else(|| panic!("body overlay {variant} is not in the content"))
    }
}

/// How a field object looks, by NameID (`byte_8021220`, 5 bytes a NameID
/// from 0xCD: sprite category and index, animation, palette, shadow).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameLook {
    pub name_id: u16,
    /// None where the table's category byte is 0xFF (`sub_800F26C`'s "no
    /// look").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite: Option<SpriteId>,
    pub anim: u8,
    pub palette: u8,
    /// Drawn with a shadow (the fifth byte nonzero).
    pub shadow: bool,
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
