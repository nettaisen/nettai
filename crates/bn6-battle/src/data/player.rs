//! Player-navi data: the row types of `player_generated` and typed
//! lookups by form, navi and NameID.

use super::player_generated as tables;
use super::{Element, SpriteId};
use crate::actor::ActorType;
use crate::setup::{Form, Navi};

/// Secondary-element bits (an attack's extra elements, or a navi's
/// weaknesses).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SecondaryElements(pub u8);

impl SecondaryElements {
    pub const BREAK: u8 = 0x10;
    pub const WIND: u8 = 0x20;
    pub const CURSOR: u8 = 0x40;
    pub const SWORD: u8 = 0x80;
}

/// A form's weapon routines (indices into the game's `off_80117D4`; 0xFF
/// = none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormWeapons {
    pub mode9_a: u8,
    pub a_charge: u8,
    pub buster: u8,
    pub charge_shot: u8,
    pub back_special: u8,
    pub alt_a_charge: u8,
}

/// The status timer a status effect sets (a CollisionData field).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusTimer {
    Paralyze,
    Confuse,
    Blind,
    Immobilize,
    Flash,
    /// CollisionData+0x26 (`collision::timer::SEMI_INTANGIBLE`).
    SemiIntangible,
    Invulnerable,
    Freeze,
    Bubble,
    /// Table garbage (status 0x66/0x67): the collision panel.
    CollisionPanel,
    /// Table garbage: another CollisionData offset.
    Other(u8),
}

/// A status effect: the requests it raises, its duration and its timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusEffect {
    /// ObjectFlags2 request bits.
    pub requests: u32,
    pub duration: u16,
    pub timer: StatusTimer,
}

/// An actor record (`sub_80182B4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NaviRecord {
    pub version: u8,
    pub actor_type: ActorType,
    /// Selects per-navi hooks and tables.
    pub ai_index: u8,
}

/// A sprite attach point, in pixels, x toward the facing side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachPoint {
    pub x: i8,
    pub y: i8,
}

/// A slide or push: a direction and how many panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlideVector {
    pub dx: i8,
    pub dy: i8,
    /// 0 = none; 6 = until blocked.
    pub tiles: u8,
}

impl SlideVector {
    pub const NONE: SlideVector = SlideVector { dx: 0, dy: 0, tiles: 0 };
}

/// MegaMan's battle sprite in `form`.
pub fn form_sprite(form: Form) -> SpriteId {
    tables::FORM_SPRITES[form.index()]
}

/// Another navi's battle sprite.
pub fn navi_sprite(navi: Navi) -> SpriteId {
    tables::NAVI_SPRITES[navi.index()]
}

/// MegaMan's element in `form` (the game shares one table between forms
/// and navis).
pub fn form_element(form: Form) -> Element {
    tables::ELEMENTS[form.index()]
}

/// Another navi's element.
pub fn navi_element(navi: Navi) -> Element {
    tables::ELEMENTS[navi.index()]
}

/// MegaMan's secondary-element weakness in `form` (shared with navis).
pub fn form_weakness(form: Form) -> SecondaryElements {
    tables::WEAKNESSES[form.index()]
}

/// Another navi's secondary-element weakness.
pub fn navi_weakness(navi: Navi) -> SecondaryElements {
    tables::WEAKNESSES[navi.index()]
}

/// A form's weapon routines.
pub fn form_weapons(form: Form) -> FormWeapons {
    tables::FORM_WEAPONS[form.index()]
}

/// Ticks to a full charge for a charge routine at a Charge stat. Rows are
/// five entries; a Charge past 4 reads into the next row, as in the game.
pub fn charge_threshold(routine: u8, charge: u8) -> u16 {
    let i = routine as usize * 5 + charge as usize;
    tables::CHARGE_THRESHOLDS[i / 5][i % 5]
}

/// Ticks of recovery after a buster shot at a Rapid stat with `open`
/// open panels ahead (counted up to 5).
pub fn buster_recovery(rapid: u8, open: u8) -> u8 {
    let i = rapid as usize * 6 + open.min(5) as usize;
    tables::BUSTER_RECOVERY[i / 6][i % 6]
}

/// A navi's move end lag (`byte_8020FE0`).
pub fn move_lag(navi: Navi, variant: u8) -> u8 {
    tables::MOVE_LAG[navi.index()][variant as usize]
}

/// The status effect for a status byte (group in the high nibble from 1,
/// entry in the low nibble); None outside the table.
pub fn status_effect(status: u8) -> Option<StatusEffect> {
    let group = (status >> 4).checked_sub(1)?;
    tables::STATUS_EFFECTS.get(group as usize).map(|g| g[(status & 0xF) as usize])
}

/// The actor record of a player NameID (0x1A0..=0x1C3).
pub fn navi_record(name_id: u16) -> NaviRecord {
    tables::NAVI_RECORDS[navi_name_index(name_id)]
}

/// A player NameID's sprite attach point `index` (`sub_8018810`).
pub fn attach_point(name_id: u16, index: usize) -> AttachPoint {
    tables::NAVI_ATTACH_POINTS[navi_name_index(name_id)][index]
}

fn navi_name_index(name_id: u16) -> usize {
    match name_id.checked_sub(0x1A0) {
        Some(i) if i < 36 => i as usize,
        _ => panic!("NameID {name_id:#x} is not a player navi"),
    }
}
