//! The ruleset's tables: data no single entity owns.

use super::{BannerId, ChipFamily, CustomScreenLayout, PanelCondition, PanelOffset, SecondaryElements};
use crate::field::PanelType;
use crate::setup::{ActorList, BattleSettings};
use serde::{Deserialize, Serialize};

/// Global rules: element weakness, collision types, panels, stages,
/// banners, statuses, weapons and the Beast Out lock-on.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rules {
    /// Extra damage multiplier by the receiver's element, then the
    /// hitter's (0 null, 1 fire, 2 aqua, 3 elec, 4 wood, 5 the drain
    /// element).
    pub element_weakness: [[u8; 6]; 6],
    /// The secondary elements each chip family adds to its attacks, by
    /// family.
    pub family_elements: [SecondaryElements; 13],
    /// Collision type flags by collision type, for side 0 and side 1
    /// (`sub_801A0BA`). A reacts to B when A's target flags meet B's self
    /// flags.
    pub collision_types: Vec<[u32; 2]>,
    /// Whole-field hit regions: region `0x80 + i` covers every panel that
    /// meets condition `i`.
    pub field_regions: Vec<PanelCondition>,
    pub panels: PanelRules,
    pub stages: Stages,
    /// Banners that stay up until removed.
    pub holding_banners: Vec<BannerId>,
    /// Status effects by status byte: group `(status >> 4) - 1`, entry
    /// `status & 0xF`.
    pub status_effects: Vec<[StatusEffect; 16]>,
    /// The HP bug's drain period by bug level.
    pub hp_bug_periods: [u8; 8],
    /// Weapon routines by number (`off_80117D4`): their charge times.
    pub weapons: Vec<WeaponRoutine>,
    /// Ticks of recovery after a buster shot, by Rapid stat, then by open
    /// panels ahead (0..=5).
    pub buster_recovery: Vec<[u8; 6]>,
    /// The deletion times (BCD hours:minutes:seconds.hundredths) at which
    /// an SP navi chip's damage steps down (`ChipData::sp_damage`).
    pub sp_deletion_times: Vec<u32>,
    /// Pushes by hit-modifier bit (+5 with 0x80).
    pub push_vectors: [SlideVector; 10],
    /// Ice slides by the direction the navi last moved.
    pub ice_vectors: [SlideVector; 6],
    /// A bubbled navi's height, by bubble timer.
    pub bubble_bob: [i8; 32],
    pub lockon: Lockon,
    /// The custom screen's slot layout.
    pub custom_screen: CustomScreenLayout,
    /// The sine table (`math_sinTable`, 1.0 = 0x100) by angle (256 a
    /// turn), with 64 entries more: `math_cosTable` is the same table 64
    /// entries on.
    pub sine: Vec<i16>,
}

impl Rules {
    /// Collision type flags for `alliance`.
    pub fn collision_type(&self, index: u8, alliance: u8) -> u32 {
        let t = self.collision_types.get(index as usize).unwrap_or_else(|| panic!("collision type {index:#x} is not in the content"));
        t[alliance as usize & 1]
    }

    /// Whether a banner stays up until removed.
    pub fn banner_holds(&self, id: BannerId) -> bool {
        self.holding_banners.contains(&id)
    }

    /// The secondary elements a chip family adds.
    pub fn family_elements(&self, family: ChipFamily) -> SecondaryElements {
        self.family_elements[family as usize]
    }

    /// The status effect of a status byte; None outside the table.
    pub fn status_effect(&self, status: u8) -> Option<StatusEffect> {
        let group = (status >> 4).checked_sub(1)?;
        self.status_effects.get(group as usize).map(|g| g[(status & 0xF) as usize])
    }

    /// Ticks to a full charge for a charge routine at a Charge stat. A
    /// Charge past 4 reads the next routine's times, as in the game.
    pub fn charge_threshold(&self, routine: u8, charge: u8) -> u16 {
        let i = routine as usize * 5 + charge as usize;
        self.weapons[i / 5].charge_ticks[i % 5]
    }

    /// Ticks of recovery after a buster shot at a Rapid stat with `open`
    /// open panels ahead (counted up to 5). A Rapid past the table reads
    /// on into the next row, as in the game.
    pub fn buster_recovery(&self, rapid: u8, open: u8) -> u8 {
        let i = rapid as usize * 6 + open.min(5) as usize;
        self.buster_recovery[i / 6][i % 6]
    }
}

/// Panel rules.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelRules {
    /// By panel type (`PanelType as usize`).
    pub types: Vec<PanelTypeRule>,
    /// Whether each panel shows at the start of a round, `[y][x]`.
    pub start_visible: [[bool; 8]; 5],
    /// Whether each panel draws its front edge, `[y][x]`.
    pub front_edges: [[bool; 8]; 5],
    /// What a panel must be to step onto (`tbl_800E660`), in dash mode
    /// (`byte_8010388`), and ignoring ownership (`byte_800E6C8`).
    pub step: StepRuleSet,
    pub dash_step: StepRuleSet,
    pub any_side_step: StepRuleSet,
}

impl PanelRules {
    /// The flag bits a panel type contributes to a panel's flags word
    /// (with the type itself in the low nibble).
    pub fn type_flags(&self, t: PanelType) -> u32 {
        t as u32 | self.types[t as usize].flags
    }

    /// Where a road panel carries a navi.
    pub fn road_slide(&self, t: PanelType) -> Option<SlideVector> {
        self.types[t as usize].road_slide
    }
}

/// What one panel type is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelTypeRule {
    /// Flag bits the type adds to a panel's flags word.
    pub flags: u32,
    /// For roads: where they carry a navi.
    pub road_slide: Option<SlideVector>,
}

/// Step rules by whether the object is floor-free (AirShoes, or standing
/// off solid ground), then by alliance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StepRuleSet {
    pub grounded: [PanelCondition; 2],
    pub floor_free: [PanelCondition; 2],
}

impl StepRuleSet {
    pub fn get(&self, floor_free: bool, alliance: u8) -> PanelCondition {
        let rules = if floor_free { &self.floor_free } else { &self.grounded };
        rules[alliance as usize & 1]
    }
}

/// Battle settings and the actor lists they spawn.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Stages {
    /// `BattleSettingsList1`, by index (a set's later rounds are drawn from
    /// it).
    pub settings: Vec<BattleSettings>,
    /// Actor lists by [`ActorListId`](crate::setup::ActorListId).
    pub actor_lists: Vec<ActorList>,
}

impl Stages {
    /// Battle settings entry `index`.
    pub fn settings(&self, index: u8) -> BattleSettings {
        *self.settings.get(index as usize).unwrap_or_else(|| panic!("battle settings {index:#x} are not in the content"))
    }

    /// An actor list.
    pub fn actor_list(&self, id: crate::setup::ActorListId) -> &ActorList {
        self.actor_lists.get(id.0 as usize).unwrap_or_else(|| panic!("actor list {} is not in the content", id.0))
    }

    /// The actor list the original's battle settings name by `address`
    /// (what link data and traces carry).
    pub fn actor_list_at(&self, address: u32) -> Option<crate::setup::ActorListId> {
        let i = self.actor_lists.iter().position(|l| l.original_address == address)?;
        Some(crate::setup::ActorListId(i as u8))
    }
}

/// A panel layout: panel types `[y - 1][x - 1]` over the playable 6x3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PanelLayout {
    pub rows: [[PanelType; 6]; 3],
}

/// The status timer a status effect sets (a collision field).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusTimer {
    Paralyze,
    Confuse,
    Blind,
    Immobilize,
    Flash,
    /// `collision::timer::SUBMERGED`.
    Submerged,
    Invulnerable,
    Freeze,
    Bubble,
    /// Table garbage (statuses past a group's end): the collision panel.
    CollisionPanel,
    /// Table garbage: another collision field, by the original's offset.
    Other(u8),
}

/// A status effect: the requests it raises, its duration and its timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusEffect {
    /// Request bits (`ObjectFlags2`).
    pub requests: u32,
    pub duration: u16,
    pub timer: StatusTimer,
}

/// A weapon routine's data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct WeaponRoutine {
    /// Ticks to a full charge, by Charge stat (0..=4).
    pub charge_ticks: [u16; 5],
}

/// A slide or push: a direction and how many panels. In a content file,
/// `{ dx, dy, panels }`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlideVector {
    pub dx: i8,
    pub dy: i8,
    /// 0 = none; 6 = until blocked.
    #[serde(rename = "panels")]
    pub tiles: u8,
}

impl SlideVector {
    pub const NONE: SlideVector = SlideVector { dx: 0, dy: 0, tiles: 0 };
}

/// The Beast Out lock-on: where the Beast rush attacks from.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Lockon {
    /// The lock-on modes that look for a panel near the target.
    pub searches: Vec<LockonSearch>,
    /// Column shifts toward the user tried, in order, when no panel next
    /// to the target fits.
    pub column_shifts: Vec<i8>,
}

impl Lockon {
    /// The search lock-on `mode` does (None for the modes that do
    /// something else).
    pub fn search(&self, mode: u8) -> Option<&LockonSearch> {
        self.searches.iter().find(|s| s.mode == mode)
    }
}

/// A lock-on mode that looks for a panel near the target to attack from.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockonSearch {
    /// The chips' lock-on mode (`ChipData::lockon_mode`).
    pub mode: u8,
    /// Panels tried, relative to the target, dx toward the user's front.
    pub offsets: Vec<PanelOffset>,
    /// Afterwards, the middle row of the chosen column is taken if free.
    pub prefers_middle_row: bool,
}
