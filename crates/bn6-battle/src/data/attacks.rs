//! Chip-attack data: the row types of `attacks_generated` and typed
//! lookups.

use super::attacks_generated as tables;
use super::player::SecondaryElements;
use super::SpriteId;

/// What an attachment (attachment object #5) looks like and where it sits
/// on its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachmentKind {
    pub sprite: SpriteId,
    pub palette: u8,
    /// Pixels the attachment is raised by (subtracted from its y and z).
    pub lift: i8,
    /// The owner's sprite attach point it follows (None: the owner's
    /// origin).
    pub attach_point: Option<u8>,
}

/// Which sun beam sprite (`SUN_BEAM_SPRITES`) and palette GunDelSol shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SunBeamLook {
    pub sprite: u8,
    pub palette: u8,
}

/// The secondary elements a chip family adds to its attacks.
pub fn family_elements(family: u8) -> SecondaryElements {
    tables::FAMILY_ELEMENTS[family as usize]
}

/// An attachment kind (its first spawn parameter).
pub fn attachment(kind: u8) -> AttachmentKind {
    tables::ATTACHMENTS[kind as usize]
}

/// GunDelSol's firing ticks at `level` (the chip's subtype).
pub fn gun_del_sol_firing_ticks(level: u8) -> u16 {
    tables::GUN_DEL_SOL_FIRING_TICKS[level as usize] as u16
}

/// GunDelSol's sun beam at `level`, fighting in the sun or not.
pub fn gun_del_sol_beam(sun: bool, level: u8) -> SunBeamLook {
    tables::GUN_DEL_SOL_BEAMS[sun as usize][level as usize]
}

/// A sun beam sprite.
pub fn sun_beam_sprite(look: u8) -> SpriteId {
    tables::SUN_BEAM_SPRITES[look as usize]
}
