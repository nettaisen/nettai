//! The ruleset's tables: data no single entity owns.

use super::{BannerId, ChipFamily, CustomScreenLayout, PanelCondition, PanelOffset, SecondaryElements};
use crate::field::PanelType;
use serde::{Deserialize, Serialize};

/// Global rules: element weakness, collision types, panels, banners,
/// statuses and the Beast Out lock-on.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rules {
    /// Extra damage multiplier by the receiver's element, then the
    /// hitter's (0 null, 1 fire, 2 aqua, 3 elec, 4 wood, 5 the drain
    /// element).
    pub element_weakness: [[u8; 6]; 6],
    /// The secondary elements each chip family adds to its attacks, by
    /// family.
    pub family_elements: [SecondaryElements; 13],
    pub panels: PanelRules,
    /// Banners that stay up until removed.
    pub holding_banners: Vec<BannerId>,
    /// The HP bug's drain period by bug level.
    pub hp_bug_periods: [u8; 8],
    /// What the charge rules read for an empty hand's chip.
    pub empty_hand: EmptyHandChip,
    /// Ticks of recovery after a buster shot, by Rapid stat, then by open
    /// panels ahead (0..=5).
    pub buster_recovery: Vec<[u8; 6]>,
    /// The deletion times (BCD hours:minutes:seconds.hundredths) at which
    /// an SP navi chip's damage steps down (`DamageFormula::SpNavi`).
    pub sp_deletion_times: Vec<u32>,
    /// The SP navis whose deletion times a round's setup carries, in its
    /// order (`RoundSetup::sp_times`): an SP navi chip's formula names its
    /// slot by these names.
    pub sp_slots: Vec<String>,
    /// The Cross special's chips (`sub_802D5A8`): a row by the hundreds of
    /// the navi's base max HP (the first row up to 199, the last from its
    /// place on), each chip by key.
    pub cross_special: Vec<Vec<SpecialChip>>,
    /// The sine table (`math_sinTable`, which `math_cosTable` continues):
    /// 256 steps a turn, 1.0 = 0x100, over a turn and a half, so that the
    /// cosine of step `a` is entry `a + 64`.
    pub sine: Vec<i16>,
    /// Pushes by hit-modifier bit (+5 with 0x80).
    pub push_vectors: [SlideVector; 10],
    /// Ice slides by the direction the navi last moved.
    pub ice_vectors: [SlideVector; 6],
    /// A bubbled navi's height, by bubble timer.
    pub bubble_bob: [i8; 32],
    pub lockon: Lockon,
    pub berserk: BerserkRules,
    /// The custom screen's slot layout.
    pub custom_screen: CustomScreenLayout,
}

/// One of the Cross special's chips (`sub_802D4F0`): the chip its
/// controller uses, by key, with another chip's damage where the original
/// takes it from one (the last row's LifeSrd strikes with VarSwrd's).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecialChip {
    pub chip: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_of: Option<String>,
}

impl Rules {
    /// Whether a banner stays up until removed.
    pub fn banner_holds(&self, id: BannerId) -> bool {
        self.holding_banners.contains(&id)
    }

    /// The secondary elements a chip family adds.
    pub fn family_elements(&self, family: ChipFamily) -> SecondaryElements {
        self.family_elements[family as usize]
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
    /// The sound a NaviCust panel trail makes turning a panel into the
    /// type (`byte_8013D44`; none: silent).
    pub trail_sound: Option<crate::sound::SoundId>,
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

/// A status effect (`define.status`): the requests it raises, its duration
/// and its timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusEffect {
    /// Request bits (`ObjectFlags2`).
    pub requests: u32,
    pub duration: u16,
    pub timer: StatusTimer,
    /// The hit that lands it doesn't flinch or flash its target: applying
    /// it drops those requests (`sub_801A554`: the freezing statuses,
    /// the original's bytes 0x50 to 0x55).
    #[serde(default)]
    pub cancels_flinch: bool,
    /// A counter hit that lands it keeps it rather than paralyzing
    /// (`sub_800EB26`: the bubbling statuses, the original's bytes 0x60 to
    /// 0x65).
    #[serde(default)]
    pub survives_counter: bool,
}

/// The chip record an empty hand reads. A hand with no chip left holds
/// chip 0xFFFF, and some of the game's checks look it up in the chip table
/// without testing for that (`getChip8021DA8(0xFFFF)`), reading whatever
/// ROM data lies 0xFFFF records past the table. It is the same in both
/// versions; these are the bytes the engine's readers look at, decoded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyHandChip {
    /// Its family byte is the Null family's (`sub_8012F62`: in the Beast
    /// forms the alternative A-charge routine's threshold applies).
    pub null_family: bool,
    /// Its element byte is Fire's (`sub_80F0608`, ChargeCross).
    pub fire: bool,
    /// Its flags byte.
    pub flags: super::ChipFlags,
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

/// The panels Beast Over's berserk controller (`sub_802D322`) looks at,
/// each by alliance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BerserkRules {
    /// Where its steps may land (`byte_802D410`, and `byte_802D420` with
    /// AirShoe).
    pub step: StepRuleSet,
    /// A panel with an opponent on it (`off_8109784`, `sub_810971A`).
    pub opponent: [PanelCondition; 2],
    /// Panel flags that end the look behind an opponent (`byte_8015D78`,
    /// `sub_8015CC0`).
    pub blocking: [u32; 2],
    /// The panel flag of the opposing player (`byte_80E74C4`,
    /// `sub_80E7486`).
    pub opposing_player: [u32; 2],
}

/// The Beast Out lock-on: where the Beast rush attacks from (`ho_8026554`,
/// by the chip's lock-on mode, a definition: `Content::lockon`). What the
/// modes share.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Lockon {
    /// Column shifts toward the user tried, in order, when no panel next
    /// to the target fits (`byte_8026735`).
    pub column_shifts: Vec<i8>,
    /// What every panel between the chosen one and the target must be for
    /// the modes that need a clear path, by alliance (`byte_8026544`).
    pub clear_path: [PanelCondition; 2],
}

/// How a lock-on mode picks the panel to attack from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockonRule {
    /// Stay on the navi's own panel (`sub_802661C`, mode 0).
    #[default]
    Stay,
    /// Along the target's row, counting from the navi's own column
    /// (`sub_8026622`): `offsets` (or `same_row_offsets` when the target
    /// stands in the navi's row), then the same in the rows `row_shifts`
    /// away from the target's.
    Row,
    /// Next to the target (`sub_8026450`): the first panel of `offsets`
    /// the navi can stand on, then with the column shifts if
    /// `column_shifts` (`sub_80265D0`), and only with a clear path to the
    /// target if `clear_path` (`sub_80264A8`).
    Near,
}

/// A lock-on mode (`define.lockon`; an entry of the original's
/// `jt_8026584`). Offsets are relative to where the rule counts from, dx
/// toward the user's front; a panel past the target's column never fits.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockonMode {
    pub rule: LockonRule,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offsets: Vec<PanelOffset>,
    /// `Row`: the offsets when the target is in the navi's row.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub same_row_offsets: Vec<PanelOffset>,
    /// `Row`: the rows tried after the target's, relative to it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_shifts: Vec<i8>,
    /// `Near`: the offsets when the target stands in the column farthest
    /// ahead of the user (`sub_80266BA`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub far_column_offsets: Option<Vec<PanelOffset>>,
    /// `Near`: try the column shifts too.
    #[serde(default)]
    pub column_shifts: bool,
    /// `Near`: every panel from the chosen one up to the target's column
    /// must meet `Lockon::clear_path`.
    #[serde(default)]
    pub clear_path: bool,
    /// Afterwards, the middle row of the chosen column is taken if the
    /// navi can stand there (`sub_80265FE`).
    #[serde(default)]
    pub prefers_middle_row: bool,
}

