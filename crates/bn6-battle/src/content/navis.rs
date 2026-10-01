//! Navis and MegaMan's forms.

use super::{BannerId, CodedChip, Element, SecondaryElements, SpriteId};
use crate::actor::ActorType;
use bn6_content_api::WeaponHandle;
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
    /// A link navi's damage bonus on its family's chips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chip_bonus: Option<NaviChipBonus>,
    /// The no-running message the custom screen shows for the navi (L in
    /// a netbattle): the characters in each of its lines (up to three),
    /// which set how long it prints (docs/engine/custom-screen.md §3.5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_message: Vec<u8>,
    /// The navi's NameID and what goes with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_record: Option<NameData>,
    /// The weapons it comes with (`byte_80210DD`): what a Cross change
    /// gives its buttons. (Read from the definition by handle, not with
    /// the rest of the record.)
    #[serde(skip)]
    pub weapons: FormWeapons,
    /// The rest of what a Cross change brings it with, fresh; none for a
    /// navi no change can bring.
    #[serde(skip)]
    pub fresh: Option<FreshStats>,
    /// Its HP after a Cross change, by side (`byte_802DD88`).
    #[serde(skip)]
    pub cross_hp: Option<[u16; 2]>,
}

/// A navi's stats when a Cross change brings it fresh (`byte_80210DD`,
/// with its weapons): its HP, its body's programs, the first barrier, its
/// Mega and Giga levels, and the damage of its B+Back special.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FreshStats {
    pub hp: u16,
    pub super_armor: bool,
    pub float_shoes: bool,
    pub air_shoes: bool,
    pub undershirt: bool,
    pub first_barrier: u8,
    pub mega_level: u8,
    pub giga_level: u8,
    pub back_special_damage: u16,
}

/// A link navi's damage bonus on the damaging chips of its family
/// (`sub_800F09E`), by the navi's level (`byte_8021300`, 15 a navi).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviChipBonus {
    pub family: super::ChipFamily,
    /// Dimming chips of the family count too.
    #[serde(default)]
    pub dimming_chips: bool,
    pub by_level: Vec<u8>,
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
    /// (Read from the definition by handle, not with the rest of the
    /// record.)
    #[serde(skip)]
    pub weapons: FormWeapons,
    /// Added to the buster's damage.
    pub buster_bonus: u8,
    /// The form's NameID and what goes with it (the base form uses
    /// MegaMan's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_record: Option<NameData>,
}

/// A form's or navi's weapons, by the button that uses each (none: the
/// original's 0xFF).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FormWeapons {
    /// The A button in battle mode 9.
    pub mode9_a: Option<WeaponHandle>,
    /// The A-button charge (charged chips).
    pub a_charge: Option<WeaponHandle>,
    /// The B-button buster.
    pub buster: Option<WeaponHandle>,
    /// The charged shot.
    pub charge_shot: Option<WeaponHandle>,
    /// The B+Back special.
    pub back_special: Option<WeaponHandle>,
    /// The A-button charge for Null-family chips in Beast forms.
    pub alt_a_charge: Option<WeaponHandle>,
}

impl FormWeapons {
    /// The slots' names in a definition's `weapons`.
    pub const SLOTS: [&'static str; 6] = ["mode9_a", "a_charge", "buster", "charge_shot", "back_special", "alt_a_charge"];

    /// The slot named `slot`.
    pub fn slot_mut(&mut self, slot: &str) -> Option<&mut Option<WeaponHandle>> {
        Some(match slot {
            "mode9_a" => &mut self.mode9_a,
            "a_charge" => &mut self.a_charge,
            "buster" => &mut self.buster,
            "charge_shot" => &mut self.charge_shot,
            "back_special" => &mut self.back_special,
            "alt_a_charge" => &mut self.alt_a_charge,
            _ => return None,
        })
    }
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

/// A navi definition's `fresh`: what a Cross change brings it with.
pub(crate) fn read_fresh(d: &bn6_content_api::Definition) -> Result<Option<FreshStats>, bn6_content_api::ContentError> {
    use bn6_content_api::{ContentError, Data};
    let what = |m: String| ContentError::new(format!("{}.luau: navi {}: {m}", d.module, d.key));
    let fresh = match d.spec.field("fresh") {
        Data::Nil => return Ok(None),
        v @ Data::Map(_) => v,
        other => return Err(what(format!("`fresh` is {other:?}, not a table"))),
    };
    let number = |field: &str, max: i64| -> Result<i64, ContentError> {
        match fresh.field(field) {
            Data::Int(i) if (0..=max).contains(i) => Ok(*i),
            other => Err(what(format!("fresh.{field} is {other:?}, not a number up to {max}"))),
        }
    };
    let flag = |field: &str| -> Result<bool, ContentError> {
        match fresh.field(field) {
            Data::Nil => Ok(false),
            Data::Bool(b) => Ok(*b),
            other => Err(what(format!("fresh.{field} is {other:?}, not true or false"))),
        }
    };
    Ok(Some(FreshStats {
        hp: number("hp", 0xFFFF)? as u16,
        super_armor: flag("super_armor")?,
        float_shoes: flag("float_shoes")?,
        air_shoes: flag("air_shoes")?,
        undershirt: flag("undershirt")?,
        first_barrier: number("first_barrier", 0xFF)? as u8,
        mega_level: number("mega_level", 0xFF)? as u8,
        giga_level: number("giga_level", 0xFF)? as u8,
        back_special_damage: match fresh.field("back_special_damage") {
            Data::Nil => 0,
            _ => number("back_special_damage", 0xFFFF)? as u16,
        },
    }))
}

/// A navi definition's `cross_hp`: its HP after a Cross change, by side.
pub(crate) fn read_cross_hp(d: &bn6_content_api::Definition) -> Result<Option<[u16; 2]>, bn6_content_api::ContentError> {
    use bn6_content_api::{ContentError, Data};
    let what = || ContentError::new(format!("{}.luau: navi {}: `cross_hp` is two HP values, by side", d.module, d.key));
    match d.spec.field("cross_hp") {
        Data::Nil => Ok(None),
        Data::List(items) => match items.as_slice() {
            [Data::Int(a), Data::Int(b)] if (0..=0xFFFF).contains(a) && (0..=0xFFFF).contains(b) => Ok(Some([*a as u16, *b as u16])),
            _ => Err(what()),
        },
        _ => Err(what()),
    }
}
