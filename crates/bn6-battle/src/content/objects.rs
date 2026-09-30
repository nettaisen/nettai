//! Object kinds' data.

use super::{AttachmentKind, Element, SpriteId};
use serde::{Deserialize, Serialize};

/// Data of the object kinds that have their own.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ObjectData {
    /// Attachment kinds by number (attachment object #5's first
    /// parameter): those the chips declare, and the rest.
    pub attachments: Vec<AttachmentKind>,
    /// Rocks by variant (the rock's first parameter).
    pub rocks: Vec<RockKind>,
    /// The sprite an absorbed obstacle flies with, by obstacle kind.
    pub absorbed_sprites: Vec<SpriteId>,
    /// Body overlays (actor object #0x56) by variant.
    pub body_overlays: Vec<BodyOverlay>,
    /// The sun beam's sprites by look (`SunBeamLook::look`).
    pub sun_beam_looks: Vec<SpriteId>,
    /// Sword waves (attack object #0x96) by kind, its first parameter.
    pub sword_waves: Vec<SwordWave>,
    /// Hit zones (attack object #0x8B) by kind, its first parameter.
    pub hit_zones: Vec<HitZone>,
    /// The object kinds scripts implement, by name (see `content::scripts`).
    pub kinds: Vec<super::ObjectKind>,
}

impl ObjectData {
    /// Attachment kind `kind`.
    pub fn attachment(&self, kind: u8) -> &AttachmentKind {
        self.attachments.get(kind as usize).unwrap_or_else(|| panic!("attachment kind {kind:#x} is not in the content"))
    }

    /// Rock variant `variant`.
    pub fn rock(&self, variant: u8) -> &RockKind {
        self.rocks.get(variant as usize).unwrap_or_else(|| panic!("rock variant {variant} is not in the content"))
    }

    /// The sprite of absorbed obstacle kind `kind`.
    pub fn absorbed_sprite(&self, kind: u8) -> SpriteId {
        *self.absorbed_sprites.get(kind as usize).unwrap_or_else(|| panic!("absorbed obstacle kind {kind} is not in the content"))
    }

    /// Body overlay `variant`.
    pub fn body_overlay(&self, variant: u8) -> &BodyOverlay {
        self.body_overlays.get(variant as usize).unwrap_or_else(|| panic!("body overlay {variant} is not in the content"))
    }

    /// A sun beam look's sprite.
    pub fn sun_beam_sprite(&self, look: u8) -> SpriteId {
        *self.sun_beam_looks.get(look as usize).unwrap_or_else(|| panic!("sun beam look {look} is not in the content"))
    }
}

/// A kind of rock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RockKind {
    /// The variant number.
    pub id: u8,
    /// The rock's standing animation.
    pub anim: u8,
    pub hp: u16,
    pub element: Element,
    /// Palette of the debris it breaks into.
    pub debris_palette: u8,
    /// Sound it breaks with.
    pub break_sound: u16,
    pub name_id: u16,
}

/// A kind of sword wave (attack object #0x96, `byte_80D7F4C`): what
/// SlashCross's charged slashes send along the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwordWave {
    /// The kind number.
    pub id: u8,
    /// Its collision types (what it is, what it hits) and hit modifier.
    pub self_type: u8,
    pub target_type: u8,
    pub hit_mod: u8,
    pub region: u8,
    pub sprite: SpriteId,
    pub anim: u8,
    /// It animates, restarting its animation after the last frame.
    pub animates: bool,
    /// It highlights the panels its region covers.
    pub highlight: bool,
    /// Panel centers it passes before it ends.
    pub reach: u8,
    /// Its shadow is on the ground (else drawn with the sprite).
    pub ground_shadow: bool,
    pub palette: u8,
    /// The status its hit inflicts (0: none).
    pub status: u8,
    /// Pixels (16.16) it moves forward per tick.
    pub speed: i32,
}

/// A kind of hit zone (attack object #0x8B, `byte_80D6914`): a hit region
/// that stays on a panel for a while, such as EraseCross's beam's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitZone {
    /// The kind number.
    pub id: u8,
    /// Its collision types (what it is, what it hits) and hit modifier.
    pub self_type: u8,
    pub target_type: u8,
    pub hit_mod: u8,
    pub hit_effect: u8,
    pub region: u8,
    /// The status its hit inflicts (0: none).
    pub status: u8,
    /// The bug it gives and its argument (0: none).
    pub bug: u8,
    pub bug_arg: u8,
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
