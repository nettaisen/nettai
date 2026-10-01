//! Object kinds' data.

use super::{Element, SecondaryElements, SpriteId};
use crate::field::PanelType;
use serde::{Deserialize, Serialize};

/// Data of the object kinds that have their own.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ObjectData {
    /// Rocks by variant (the rock's first parameter).
    pub rocks: Vec<RockKind>,
    /// The sprite an absorbed obstacle flies with, by obstacle kind.
    pub absorbed_sprites: Vec<SpriteId>,
    /// Body overlays (actor object #0x56) by variant.
    pub body_overlays: Vec<BodyOverlay>,
    /// The sun beam's sprites by look (`SunBeamLook::look`).
    pub sun_beam_looks: Vec<SpriteId>,
    /// Boomerangs (attack object #0x32) by variant, its first parameter.
    pub boomerangs: Vec<BoomerangKind>,
    /// The projectile (attack object #0) by kind, its first parameter.
    pub projectiles: Vec<ProjectileKind>,
    /// The flying shot (attack object #0xB) by kind, its first parameter.
    pub flying_shots: Vec<FlyingShotKind>,
    /// Sword waves (attack object #0x96) by kind, its first parameter.
    pub sword_waves: Vec<SwordWave>,
    /// The object kinds scripts implement, by name (see `content::scripts`).
    pub kinds: Vec<super::ObjectKind>,
    /// Shock waves (attack object #0x16) by variant, its first parameter.
    pub shock_waves: Vec<ShockWave>,
}

impl ObjectData {
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

/// A shock wave's look and timing (attack object #0x16, `byte_80C6B00`):
/// the rolling wave of WaveArm, PwrWave and the viruses that send one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShockWave {
    /// The variant number.
    pub id: u8,
    pub sprite: SpriteId,
    pub anim: u8,
    /// Ticks on a panel before the next wave rolls on.
    pub ticks: u8,
    /// What it does to its panel as it comes: cracks it (`cracked`),
    /// breaks it (`broken`, or cracks it when something stands there), or
    /// turns it to another type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<crate::field::PanelType>,
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

/// A kind of boomerang (attack object #0x32, `byte_80CA26C`): how fast it
/// flies along a row and along the far column, and whether it turns the
/// other side's panels it crosses to grass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoomerangKind {
    /// The variant number.
    pub id: u8,
    /// Along a row and along the column, 16.16 pixels a tick.
    pub speed: i32,
    pub turn_speed: i32,
    pub grass: bool,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

fn no_secondary(v: &SecondaryElements) -> bool {
    v.0 == 0
}

/// A kind of projectile (attack object #0, `sub_80C4E58`): the shot the
/// buster, the cannons and many chips fire. It flies a panel every two
/// ticks and ends on the first thing it hits or when it leaves the field.
/// Its first parameter picks the kind (`off_80C4C78`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectileKind {
    /// The kind number.
    pub id: u8,
    /// Its collision types (the original's rows by number): what it is, what
    /// it hits; and its hit modifier.
    pub self_type: u8,
    pub target_type: u8,
    pub hit_mod: u8,
    pub element: Element,
    /// The secondary elements its hits carry.
    #[serde(default, skip_serializing_if = "no_secondary")]
    pub secondary: SecondaryElements,
    /// The hit spark it shows (0xFF: none).
    pub hit_effect: u8,
    /// Its sprite, if it is drawn (most are not), and the animation it
    /// plays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite: Option<SpriteId>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub anim: u8,
    /// The status byte its hits inflict (`Rules::status_effects`), 0 for
    /// none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub status: u8,
    /// The bug its hits give: the bug's code (0 for none) and argument.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bug: u8,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bug_arg: u8,
    /// What its hit does to the panel it hits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_panel: Option<PanelHit>,
    /// It bursts: when it hits, over the panels around it (`sub_80C5050`);
    /// when it leaves the field, over the last two columns it crossed
    /// (`sub_80C5014`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bursts: bool,
    /// Every panel it enters, it sits a pixel further down the field and
    /// a pixel higher than on the last (`sub_80C5090`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub climbs: bool,
}

/// What a projectile's hit does to the panel it hits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "effect", rename_all = "snake_case", deny_unknown_fields)]
pub enum PanelHit {
    /// `object_crackPanel`: crack it, or break it if it is cracked and
    /// nothing stands on it.
    Crack,
    /// `object_breakPanel_dup2`: break it, or crack it if something
    /// stands on it.
    Break,
    /// If it is solid, it becomes a panel of this type: the first for the
    /// left side's shots, the second for the right side's.
    SetType { left_side: PanelType, right_side: PanelType },
}

/// A kind of flying shot (attack object #0xB, `sub_80C60A8`): an arrow, a
/// Beast form's slash wave, a thrown obstacle. It flies at a steady speed
/// for a number of panels, and ends on the first thing it hits or when it
/// leaves the field. Its first parameter picks the kind (`byte_80C6038`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlyingShotKind {
    /// The kind number.
    pub id: u8,
    /// Its collision types (the original's rows by number): what it is, what
    /// it hits; and its hit modifier.
    pub self_type: u8,
    pub target_type: u8,
    pub hit_mod: u8,
    pub element: Element,
    #[serde(default, skip_serializing_if = "no_secondary")]
    pub secondary: SecondaryElements,
    /// The hit spark it shows (0xFF: none).
    pub hit_effect: u8,
    /// Its sprite and animation (a thrown obstacle's are the obstacle's).
    pub sprite: SpriteId,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub anim: u8,
    /// It draws a shadow.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shadow: bool,
    /// Pixels a tick (16.16) it flies forward.
    pub speed: i32,
    /// How many panel centers it passes before it ends.
    pub range: u8,
    /// The status byte its hits inflict (`Rules::status_effects`), 0 for
    /// none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub status: u8,
    /// It highlights the panels it is over.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub highlight: bool,
    /// It is a thrown obstacle: its look comes from its spawner (the
    /// obstacle's sprite and animation), not from this record.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub obstacle: bool,
    /// Its hit spark shows over the center of the panel it is on
    /// (`sub_801A100`) rather than where it is.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub panel_spark: bool,
    /// The sound when it sets off after its wait (its second parameter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_sound: Option<u16>,
    /// The one-shot effect it leaves on its panel when its range runs
    /// out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_effect: Option<u8>,
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
