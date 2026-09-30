//! The slice's data. It comes from the content pack the battle runs on:
//! the engine fills [`Data`] from its `Content` when it loads this content
//! (`bn6_battle::behavior::Behaviors::rust`). No game data is compiled in.

use bn6_content_api::SpriteId;
use std::collections::BTreeMap;

/// What an attachment looks like and where it sits on its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachmentKind {
    /// Its number (the attachment's first spawn parameter).
    pub id: u8,
    pub sprite: SpriteId,
    pub palette: u8,
    /// Pixels the attachment is raised by (subtracted from its y and z).
    pub lift: i8,
    /// The owner's sprite attach point it follows (None: the owner's origin).
    pub attach_point: Option<u8>,
}

/// Which sun beam look (`Data::sun_beam_looks`) and palette GunDelSol
/// shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SunBeamLook {
    pub look: u8,
    pub palette: u8,
}

/// A GunDelSol chip's data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GunDelSol {
    /// Ticks of hits under the beam.
    pub firing_ticks: u16,
    /// The sun beam in the shade and in the sun.
    pub beam: SunBeamLook,
    pub beam_in_sun: SunBeamLook,
    /// The gun attached to the user.
    pub gun: AttachmentKind,
}

/// The data the slice reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Data {
    /// GunDelSol's data by chip id.
    pub gun_del_sol: BTreeMap<u16, GunDelSol>,
    /// Attachment kinds by number.
    pub attachments: Vec<AttachmentKind>,
    /// The sun beam's sprites by look.
    pub sun_beam_looks: Vec<SpriteId>,
}
