//! Navis and MegaMan's forms.

use super::{BannerId, CodedChip, Element, SecondaryElements, SpriteId};
use crate::actor::ActorType;
use serde::{Deserialize, Serialize};

/// A navi (NaviStats' navi number): MegaMan (0) or a link navi.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviData {
    /// The navi number.
    pub id: u8,
    pub name: String,
    /// The battle sprite (MegaMan draws his form's instead).
    pub sprite: SpriteId,
    pub element: Element,
    pub weakness: SecondaryElements,
    /// Added to the buster's damage.
    pub buster_bonus: u8,
    /// Ticks of lag after a move, by navi variant (NaviStats' variant).
    pub move_lag: Vec<u8>,
    /// The netbattle result banners.
    pub win_banner: BannerId,
    pub lose_banner: BannerId,
    /// Extra height, in whole pixels, of the navi's image as it merges
    /// with MegaMan in a Cross.
    pub merge_height: i16,
    /// A link navi's own chip, offered on the custom screen once a round.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_chip: Option<CodedChip>,
    /// The navi's NameID and what goes with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_record: Option<NameData>,
}

/// One of MegaMan's forms: his base form (0), a Cross (1..=10), Beast Out
/// (0x0B, 0x0C), a Cross in Beast Out (0x0D..=0x16) or Beast Over (0x17,
/// 0x18).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormData {
    /// The form number.
    pub id: u8,
    pub name: String,
    pub sprite: SpriteId,
    pub element: Element,
    pub weakness: SecondaryElements,
    pub weapons: FormWeapons,
    /// Added to the buster's damage.
    pub buster_bonus: u8,
    /// The form's NameID and what goes with it (the base form uses
    /// MegaMan's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_record: Option<NameData>,
}

/// A form's weapon routines (numbers of the engine's weapon routines;
/// 0xFF = none).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormWeapons {
    /// The A button in battle mode 9.
    pub mode9_a: u8,
    /// The A-button charge (charged chips).
    pub a_charge: u8,
    /// The B-button buster.
    pub buster: u8,
    /// The charged shot.
    pub charge_shot: u8,
    /// The B+Back special.
    pub back_special: u8,
    /// The A-button charge for Null-family chips in Beast forms.
    pub alt_a_charge: u8,
}

/// What goes with a player NameID: the actor record and the sprite's
/// attach points.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameData {
    /// The NameID (0x1A0..=0x1C3 for BN6's player navis and forms).
    pub id: u16,
    /// The actor record (`sub_80182B4`).
    pub version: u8,
    pub actor_type: ActorType,
    /// Selects per-navi hooks and tables.
    pub ai_index: u8,
    /// Where things attach to the sprite (`sub_8018810`), by attach
    /// point.
    pub attach_points: Vec<AttachPoint>,
}

impl NameData {
    /// The actor record.
    pub fn record(&self) -> NaviRecord {
        NaviRecord { version: self.version, actor_type: self.actor_type, ai_index: self.ai_index }
    }
}

/// An actor record (`sub_80182B4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NaviRecord {
    pub version: u8,
    pub actor_type: ActorType,
    /// Selects per-navi hooks and tables.
    pub ai_index: u8,
}

/// A sprite attach point, in pixels, x toward the facing side. In a
/// content file, `[x, y]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttachPoint {
    pub x: i8,
    pub y: i8,
}

impl Serialize for AttachPoint {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        [self.x, self.y].serialize(s)
    }
}

impl<'de> Deserialize<'de> for AttachPoint {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<AttachPoint, D::Error> {
        let [x, y] = <[i8; 2]>::deserialize(d)?;
        Ok(AttachPoint { x, y })
    }
}
