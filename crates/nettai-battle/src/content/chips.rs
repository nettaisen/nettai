//! Chips: the chip record.

use super::flags::serde_flags;
use super::{ChipModifier, Element, ProgramAdvanceRecipe};
use serde::{Deserialize, Serialize};

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
    /// A dark chip as the custom screen treats it: the cursor starts on it,
    /// and while it rests on it the screen darkens and the music quiets
    /// (`sub_802806C`, `sub_802A2B0`); its window frame is the dark one. No
    /// BN6 chip has it (docs/engine/unverified.md).
    pub const DARK: u8 = 0x20;
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
        (0x20, "dark"),
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
    /// The menus class it with the modifier chips (WhiCapsl, Uninstll and
    /// the plus chips, which the custom screen folds into the chip before
    /// them; the battle reads `ChipData::modifier`).
    pub const MODIFIER: u8 = 0x40;
    /// Costs no slot-in gauge (Battle Chip Gate slot-in only).
    pub const FREE_SLOT_IN: u8 = 0x80;
    pub(crate) const NAMES: &[(u32, &str)] = &[(0x02, "rush_cancels"), (0x40, "modifier"), (0x80, "free_slot_in")];

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }
}

serde_flags!(ExtraChipFlags, u8);

/// What the ruleset asks of particular chips (docs/design/
/// content-model-v2.md §7.5): the cases the original tells by a chip's
/// place in its chip table. In a content file, a list of names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChipTraits(pub u8);

impl ChipTraits {
    /// The Beast rush doesn't chain it as the next chip (`sub_800FC30`:
    /// the variable swords).
    pub const NO_CHAIN: u8 = 0x01;
    /// It hits harder while its user's barrier holds (`sub_800F1DC`: the
    /// AuraHeds and StreamHd).
    pub const AURA_BONUS: u8 = 0x02;
    /// The other side can't cut in on the dimming it starts (`sub_800BF16`
    /// with the chip's cut-in rule: the chips past the Program Advances).
    pub const NO_CUT_IN: u8 = 0x04;
    /// SlashCross charges it though its family isn't Sword (`sub_8013236`:
    /// the elemental swords).
    pub const ELEMENT_SWORD: u8 = 0x08;
    /// AntiNavi turns it back though it has no `navi` flag (the navi
    /// chips' block of the chip table, `sub_800BDB2`: the US games' records
    /// of Django's chips, which lack it; BN6's content takes the Japanese
    /// games', which have it, so no BN6 chip has this).
    pub const NAVI_SLOT: u8 = 0x10;
    /// Its navi heals: the other side's armed AntiRecv springs instead of
    /// it coming (`sub_80E192C`: Roll's chips).
    pub const HEALS: u8 = 0x20;
    /// Its user stays on the field while its navi acts: the navi chip's
    /// controller warps it neither out nor back in (`sub_80E1830`: BigHook,
    /// the original's navi 0x17).
    pub const USER_STAYS: u8 = 0x40;
    /// Its navi brings the user back itself: the controller warps the
    /// user out and not back in (`sub_80E18F8`: Roll's chips, the
    /// original's navi 0).
    pub const NAVI_RETURNS_USER: u8 = 0x80;
    pub(crate) const NAMES: &[(u32, &str)] = &[
        (0x01, "no_chain"),
        (0x02, "aura_bonus"),
        (0x04, "no_cut_in"),
        (0x08, "element_sword"),
        (0x10, "navi_slot"),
        (0x20, "heals"),
        (0x40, "user_stays"),
        (0x80, "navi_returns_user"),
    ];

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }
}

serde_flags!(ChipTraits, u8);

/// A trap chip: what the defensive-chip record that holds it catches (the
/// ruleset's side of the trap chips, docs/engine/dimming-chips.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trap {
    /// AntiDmg: a hit of 10 or more (`sub_801056A`).
    AntiDamage,
    /// AntiSwrd: a sword hit.
    AntiSword,
    /// BodyGrd: as AntiDmg, with its own counter.
    BodyGuard,
    /// AntiNavi: the other side's navi chip turns back.
    AntiNavi,
    /// AntiRecv: the other side's heal.
    AntiRecovery,
}

/// How a chip's damage is worked out when it isn't a fixed number
/// (`off_80109DC`, by the original's formula number). In a content file,
/// `damage = { formula = "hp_lost" }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "formula", rename_all = "snake_case", deny_unknown_fields)]
pub enum DamageFormula {
    /// The opponent's HP, at most 500 (`sub_8010A90`; formula 0).
    OpponentHp,
    /// An SP navi chip's: by how long its user took to delete that SP navi
    /// (`sub_8010AE4`; formulas 1 to 18). `slot`: the SP navi, one of the
    /// rules' `sp_slots`; `by_time`: the damage by deletion-time step
    /// (`Rules::sp_deletion_times`).
    SpNavi { slot: String, by_time: Vec<u16> },
    /// By how full the custom gauge is (`sub_8010B78`; formula 19).
    Gauge,
    /// The HP its user has lost, at most 500 (`sub_8010BD0`; formula 20).
    HpLost,
    /// The last two digits of its user's HP (`sub_8010BF0`; formula 21).
    HpLastDigits,
    /// Half the opponent's max HP, at most 999 (`sub_8010C06`; formula 22).
    HalfOpponentMaxHp,
    /// A link navi's chip's: `base`, plus `per_level` for each level of
    /// its user's buster attack up to 5 (`sub_8010C50`, a row of
    /// `byte_80212D4`; formulas 23 to 44).
    NaviLevel { base: u8, per_level: u8 },
}

/// One battle chip's record (docs/engine/chips.md §1.2): what the ruleset
/// reads of it. Its use (an action, or a dimming, navi or instant hook) is
/// its definition's (`ChipDef::usage`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipData {
    pub name: String,
    /// The description the custom screen shows (R), its lines apart by
    /// `\n`. The battle reads only how many lines it has
    /// (`description_lines`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
    /// In Beast Out, the chip's attack goes through the Beast rush.
    pub beast_lockon: bool,
    /// Input lockout after the attack ends, in ticks.
    pub lockout: u8,
    #[serde(default)]
    pub extra_flags: ExtraChipFlags,
    /// The Beast Out rush's lock-on mode for it (`Content::lockon`); none:
    /// the navi stays where it is.
    #[serde(default, with = "lockon_handle", skip_serializing_if = "Option::is_none")]
    pub lockon_mode: Option<nettai_content_api::LockonHandle>,
    /// Base damage (0 for a chip whose damage is a `formula`).
    pub damage: u16,
    /// How the damage is worked out, for a chip whose damage isn't fixed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<DamageFormula>,
    /// What the ruleset asks of this chip in particular.
    #[serde(default)]
    pub traits: ChipTraits,
    /// A trap chip: what it catches as the side's defensive chip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trap: Option<Trap>,
    /// A dark chip's cost: what using it adds to its user's HP bug, which
    /// stops at 7 (`sub_800B79A`).
    #[serde(default)]
    pub hp_bug: u8,
    /// Library number, index within the library, and alphabetical sort
    /// key (menus only).
    pub library_number: u16,
    pub library_index: u8,
    pub sort_key: u16,
    /// Uses per battle through the Battle Chip Gate's slot-in.
    pub slot_in_limit: u8,
    /// A dark chip's substitute: the chip (by key) its user gets instead
    /// with no bug frag left (`sub_8010D58`). A chip with one costs a bug
    /// frag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dark_substitute: Option<String>,
    /// What the chip does to the chip picked before it, as a modifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier: Option<ChipModifier>,
    /// The Program Advances that make this chip, their ingredients by key.
    #[serde(default, rename = "program_advance", skip_serializing_if = "Vec::is_empty")]
    pub program_advances: Vec<ProgramAdvanceRecipe>,
}

impl ChipData {
    /// Lines of the chip's description: the custom screen's description
    /// box takes keys a tick later for each line after the first
    /// (docs/engine/custom-screen.md §3.5). A chip without one counts as
    /// three, what nearly every chip has.
    pub fn description_lines(&self) -> u8 {
        self.description.as_ref().map_or(3, |d| d.split('\n').count().clamp(1, 3) as u8)
    }
}

/// A lock-on mode's handle in a record's data form: its index, or nothing.
mod lockon_handle {
    use nettai_content_api::LockonHandle;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(h: &Option<LockonHandle>, s: S) -> Result<S::Ok, S::Error> {
        h.map(|h| h.0).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<LockonHandle>, D::Error> {
        Ok(Option::<u16>::deserialize(d)?.map(LockonHandle))
    }
}
