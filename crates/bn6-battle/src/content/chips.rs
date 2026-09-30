//! Chips: the chip record, and the data only one chip's action uses.

use super::flags::serde_flags;
use super::{ChipModifier, Element, ProgramAdvanceRecipe, SpriteId};
use serde::{Deserialize, Serialize};

/// A chip id (0..=0x19A in BN6).
pub type ChipId = u16;

/// A chip code: A-Z are 0-25, `*` is 26. In a content file, the letter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChipCode(pub u8);

impl ChipCode {
    pub const ASTERISK: ChipCode = ChipCode(26);

    pub fn letter(self) -> char {
        if self.0 == 26 { '*' } else { (b'A' + self.0) as char }
    }

    pub fn from_letter(c: char) -> Option<ChipCode> {
        match c {
            '*' => Some(ChipCode::ASTERISK),
            'A'..='Z' => Some(ChipCode(c as u8 - b'A')),
            _ => None,
        }
    }
}

impl Serialize for ChipCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&self.letter())
    }
}

impl<'de> Deserialize<'de> for ChipCode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<ChipCode, D::Error> {
        let s = String::deserialize(d)?;
        let mut chars = s.chars();
        match (chars.next().and_then(ChipCode::from_letter), chars.next()) {
            (Some(c), None) => Ok(c),
            _ => Err(serde::de::Error::custom(format!("{s:?} is not a chip code (A-Z or *)"))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChipClass {
    Standard,
    Mega,
    Giga,
    /// Not a folder chip (cross/beast attacks, internal chips).
    Special,
    ProgramAdvance,
}

/// A chip's icon family. It gives the chip's attacks their secondary
/// elements (`Rules::family_elements`) and keys the forms' chip bonuses
/// and charged chips.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChipFamily {
    Fire = 0,
    Aqua = 1,
    Elec = 2,
    Wood = 3,
    Plus = 4,
    Sword = 5,
    Cursor = 6,
    /// Obstacles and summons.
    Summon = 7,
    Wind = 8,
    Break = 9,
    Null = 10,
    ProgramAdvance = 11,
    /// The cross and beast attacks (chips 0x160..0x171).
    Special = 12,
}

impl ChipFamily {
    pub const ALL: [ChipFamily; 13] = [
        ChipFamily::Fire,
        ChipFamily::Aqua,
        ChipFamily::Elec,
        ChipFamily::Wood,
        ChipFamily::Plus,
        ChipFamily::Sword,
        ChipFamily::Cursor,
        ChipFamily::Summon,
        ChipFamily::Wind,
        ChipFamily::Break,
        ChipFamily::Null,
        ChipFamily::ProgramAdvance,
        ChipFamily::Special,
    ];

    /// The family with the original's number.
    pub fn from_number(n: u8) -> Option<ChipFamily> {
        ChipFamily::ALL.get(n as usize).copied()
    }
}

/// The chip record's flags. In a content file, a list of names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChipFlags(pub u8);

impl ChipFlags {
    /// A dimming chip: its use starts a dimming; it can counter while dimmed.
    pub const DIMMING: u8 = 0x01;
    /// Deals damage: shown on the banner, boostable by Atk+ and forms.
    pub const HAS_DAMAGE: u8 = 0x02;
    /// Navi chip: boosted by Navi+.
    pub const NAVI: u8 = 0x04;
    /// In the standard library (menus only).
    pub const STANDARD_LIBRARY: u8 = 0x08;
    /// Damage shown as variable (menus only).
    pub const DAMAGE_SHOWN_VARIABLE: u8 = 0x10;
    /// In a library (menus only).
    pub const LIBRARY: u8 = 0x40;
    /// Damage recomputed every tick while this is the next chip.
    pub const VARIABLE_DAMAGE: u8 = 0x80;
    pub(crate) const NAMES: &[(u32, &str)] = &[
        (0x01, "dimming"),
        (0x02, "has_damage"),
        (0x04, "navi"),
        (0x08, "standard_library"),
        (0x10, "damage_shown_variable"),
        (0x40, "library"),
        (0x80, "variable_damage"),
    ];

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }
}

serde_flags!(ChipFlags, u8);

/// The chip record's second flag byte. Bits without a name only sort
/// chips in menus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ExtraChipFlags(pub u8);

impl ExtraChipFlags {
    /// Cancelled by the opponent's Rush (a NaviCust support).
    pub const RUSH_CANCELS: u8 = 0x02;
    /// Costs no slot-in gauge (Battle Chip Gate slot-in only).
    pub const FREE_SLOT_IN: u8 = 0x80;
    pub(crate) const NAMES: &[(u32, &str)] = &[(0x02, "rush_cancels"), (0x80, "free_slot_in")];

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }
}

serde_flags!(ExtraChipFlags, u8);

/// One battle chip (docs/engine/chips.md §1.2), with the data that only
/// its action uses.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipData {
    /// The chip's number in the pack's table (its place in the original's
    /// chip table); a chip content defines has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ChipId>,
    pub name: String,
    /// Codes the chip comes in (up to four).
    #[serde(default)]
    pub codes: Vec<ChipCode>,
    pub element: Element,
    /// Stars minus one.
    pub rarity: u8,
    pub family: ChipFamily,
    pub class: ChipClass,
    /// Folder memory cost.
    pub mb: u8,
    #[serde(default)]
    pub flags: ChipFlags,
    /// Counter/stagger strength carried to the attack's hitbox (the high
    /// half of its damage word).
    pub hit_param: u8,
    /// The attack action the user performs.
    pub action: u8,
    /// Variant within the action (e.g. Cannon/HiCannon/M-Cannon = 0/1/2).
    pub subtype: u8,
    /// In Beast Out, the chip's attack goes through the Beast rush.
    pub beast_lockon: bool,
    /// Action-specific parameters.
    pub params: [u8; 4],
    /// Input lockout after the attack ends, in ticks.
    pub lockout: u8,
    #[serde(default)]
    pub extra_flags: ExtraChipFlags,
    /// Beast Out lock-on panel search (`Rules::lockon`).
    pub lockon_mode: u8,
    /// Base damage; 1000 and up select damage formula `damage - 1000`.
    pub damage: u16,
    /// Library number, index within the library, and alphabetical sort
    /// key (menus only).
    pub library_number: u16,
    pub library_index: u8,
    pub sort_key: u16,
    /// Uses per battle through the Battle Chip Gate's slot-in.
    pub slot_in_limit: u8,
    /// A dark chip's substitute when its user has no bug frags (an index
    /// into the dark-chip substitute list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dark_substitute: Option<u8>,
    /// An SP navi chip's damage by deletion-time step (damage formulas
    /// 1..=18; `Rules::sp_deletion_times`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sp_damage: Option<Vec<u16>>,
    /// A link navi's chip's damage (damage formulas 24..=44).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navi_damage: Option<NaviChipDamage>,
    /// What the chip does to the chip picked before it, as a modifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier: Option<ChipModifier>,
    /// The Program Advances that make this chip.
    #[serde(default, rename = "program_advance", skip_serializing_if = "Vec::is_empty")]
    pub program_advances: Vec<ProgramAdvanceRecipe>,
    /// GunDelSol's data (action 0x37).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gun_del_sol: Option<GunDelSol>,
    /// The HP a recovery chip restores (action 0x20).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<u16>,
    /// A sword's data (actions 0x13 and 0x49).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sword: Option<Sword>,
    /// The script that implements the chip's action, or its part of a
    /// generic one (see `content::scripts`): a module path in the pack
    /// (`chips/00f-gundels1/chip`); in the chip's file, a path relative to
    /// its folder (`chip.luau`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
}

/// What an attachment (attachment object #5) looks like and where it sits
/// on its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentKind {
    /// The attachment's number: its first spawn parameter, which the
    /// object state keeps.
    pub id: u8,
    pub sprite: SpriteId,
    pub palette: u8,
    /// Pixels the attachment is raised by (subtracted from its y and z).
    pub lift: i8,
    /// The owner's sprite attach point it follows (none: the owner's
    /// origin).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach_point: Option<u8>,
}

/// Which sun beam look (`ObjectData::sun_beam_looks`, the beam object's
/// first parameter) and palette GunDelSol shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SunBeamLook {
    pub look: u8,
    pub palette: u8,
}

/// A sword's per-chip data, by the chip's subtype: the blade its user holds
/// (actions 0x13 and 0x49) and, for action 0x13 (`sub_80EB776`), its slash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sword {
    /// The blade (`byte_80EBB64`): an attachment kind.
    pub blade: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slash: Option<SwordSlash>,
}

/// Action 0x13's slash: its one-tick hit region (`byte_80EBA18`, and
/// `byte_80EBA58` for what the hit does) and the effect that draws it
/// (`byte_80EBAD8`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwordSlash {
    /// Region shape, hit spark and collision types.
    pub region: u8,
    pub hit_effect: u8,
    pub target: u8,
    pub self_type: u8,
    /// Hit modifier, status effect, bug and its argument.
    pub hit_mod: u8,
    pub status: u8,
    pub bug: u8,
    pub bug_arg: u8,
    /// The effect (effect object #0) it shows on the panel ahead.
    pub effect: u8,
}

/// A link navi's chip's damage (`sub_8010C50`, a row of `byte_80212D4`):
/// `base`, plus `per_level` for each level of its user's buster attack
/// up to 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviChipDamage {
    pub base: u8,
    pub per_level: u8,
}

/// GunDelSol's per-chip data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GunDelSol {
    /// Ticks of hits under the beam.
    pub firing_ticks: u16,
    /// The sun beam, in the shade and in the sun.
    pub beam: SunBeamLook,
    pub beam_in_sun: SunBeamLook,
    /// The gun, attached to the user.
    pub gun: AttachmentKind,
}
