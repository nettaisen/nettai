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

/// A chip's family: one of its game's (the rules' `elements.families`,
/// [`super::ChipFamilies`]), by its number there, the number the game's
/// chip records hold and its pack's custom-screen icons are by. It gives
/// the chip's attacks their secondary elements (`Rules::family_elements`)
/// and keys the forms' chip bonuses and charged chips. In a content file,
/// the family's name, which the game's families say the number of (a
/// definition's names are read while its game's content is defined:
/// [`reading_families`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChipFamily(pub u8);

thread_local! {
    /// The families of the game whose definitions are being read (the
    /// rules' `elements.families`), which a family's name is one of.
    static READING: std::cell::RefCell<Option<super::ChipFamilies>> = const { std::cell::RefCell::new(None) };
}

/// `f`, with a family's name read as one of `families` (the game's, as its
/// content is defined).
pub fn reading_families<T>(families: &super::ChipFamilies, f: impl FnOnce() -> T) -> T {
    let before = READING.with(|r| r.replace(Some(families.clone())));
    let out = f();
    READING.with(|r| *r.borrow_mut() = before);
    out
}

/// The non-elemental family of the game whose definitions are being read:
/// the family of a chip that names none.
pub(crate) fn reading_non_elemental() -> Option<ChipFamily> {
    READING.with(|r| r.borrow().as_ref().map(|f| f.non_elemental))
}

impl Serialize for ChipFamily {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.0)
    }
}

impl<'de> Deserialize<'de> for ChipFamily {
    /// A family's name (one of the game's being read), or its number.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<ChipFamily, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Named {
            Number(u8),
            Name(String),
        }
        match Named::deserialize(d)? {
            Named::Number(n) => Ok(ChipFamily(n)),
            Named::Name(name) => READING.with(|r| match r.borrow().as_ref() {
                Some(families) => families.by_name(&name).ok_or_else(|| {
                    serde::de::Error::custom(format!("{name:?} is none of its game's chip families ({})", families.names().join(", ")))
                }),
                None => Err(serde::de::Error::custom(format!("chip family {name:?}: no game's families are being read"))),
            }),
        }
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
    /// A dark chip as the custom screen treats it: the cursor starts on it,
    /// and while it rests on it the screen darkens and the music quiets
    /// (`sub_802806C`, `sub_802A2B0`); its window frame is the dark one. No
    /// EXE6 chip has it (docs/engine/unverified.md).
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
    /// Canceled by the opponent's Rush (a NaviCust support).
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

/// What the rules ask of particular chips (docs/design/
/// content-model-v2.md §7.5): the cases the original tells by a chip's
/// place in its chip table. In a content file, a list of names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChipTraits(pub u16);

impl ChipTraits {
    /// The Beast rush doesn't chain it as the next chip (`sub_800FC30`:
    /// the variable swords).
    pub const NO_CHAIN: u16 = 0x01;
    /// It hits harder while its user's barrier holds (`sub_800F1DC`: the
    /// AuraHeds and StreamHd).
    pub const AURA_BONUS: u16 = 0x02;
    /// The other side can't cut in on the dimming it starts (`sub_800BF16`
    /// with the chip's cut-in rule: the chips past the Program Advances).
    pub const NO_CUT_IN: u16 = 0x04;
    /// SlashCross charges it though its family isn't Sword (`sub_8013236`:
    /// the elemental swords).
    pub const ELEMENT_SWORD: u16 = 0x08;
    /// It goes with any selection, and the selection's code and chip rules
    /// leave it out (`sub_8028E4C`, `sub_8028EC8`: EXE6's BeastOut chip).
    pub const GOES_WITH_ANY: u16 = 0x100;
    /// The custom screen's chip window shows "???" for its damage on a
    /// copy of code A, and its damage on any other: the original compares
    /// the whole chip word, number and code, with the chip's number, so
    /// only code A (0) matches (`sub_80284E2`: Muramasa, whose only code
    /// is M; EXE5's 0x080243C0: Muramasa, CustSwrd and the three CusVolts,
    /// of which CusVolt1 comes as an A).
    pub const HIDES_DAMAGE_AS_A: u16 = 0x400;
    pub(crate) const NAMES: &[(u32, &str)] = &[
        (0x01, "no_chain"),
        (0x02, "aura_bonus"),
        (0x04, "no_cut_in"),
        (0x08, "element_sword"),
        (0x100, "goes_with_any"),
        (0x400, "hides_damage_as_a"),
    ];

    pub fn has(self, bit: u16) -> bool {
        self.0 & bit != 0
    }

}

serde_flags!(ChipTraits, u16);

/// A trap chip: what the defensive-chip record that holds it catches (the
/// rules' side of the trap chips, docs/engine/dimming-chips.md).
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
    /// By how full the custom gauge is (`sub_8010B78`; formula 19).
    Gauge,
    /// The HP its user has lost, at most `cap`: EXE6's 500 (`sub_8010BD0`;
    /// formula 20) when none, EXE5's 999 (its formula 46, 0x0800EA78).
    HpLost {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cap: Option<u16>,
    },
    /// The last two digits of its user's HP (`sub_8010BF0`; formula 21).
    HpLastDigits,
    /// Half the opponent's max HP, at most 999 (`sub_8010C06`; formula 22).
    HalfOpponentMaxHp,
    /// A link navi's chip's: `base`, plus `per_level` for each level of
    /// its user's buster attack up to 5 (`sub_8010C50`, a row of
    /// `byte_80212D4`; formulas 23 to 44).
    NaviLevel { base: u8, per_level: u8 },
    /// A function of the side and the chip gives it (`damage =
    /// function(side, chip)`), which the round's setup asks once for each
    /// side (`Battle::given`): EXE5's team navis' own chips (its formulas
    /// 50 to 72, 0x0800EAF8: a row of 0x0801D74F at the side's level, as
    /// EXE5's rules read the level, lib/navi_level); both games' SP navi
    /// chips (EXE6's formulas 1 to 18, `sub_8010AE4`, EXE5's 0x0800E8DE: the
    /// chip's `sp.by_time` at the side's deletion time for it,
    /// rules/sp_chips).
    #[serde(skip)]
    Given(nettai_content_api::FnId),
    /// EXE5's CusVolt (its formulas 73 to 75, 0x0800EB0E): `base` plus 100
    /// by the custom gauge's level (its value >> 7): 100 × level / 95 below
    /// 96, 100 to 126, none from 127 (full); the side's own gauge in the
    /// the own-gauges mode.
    GaugeLevel { base: u16 },
    /// By a count of its user's side, `by_count[n]` (the last entry for
    /// more), and `operation_battle` in EXE5's operation battle: EXE5's
    /// DS navi chips (formulas 23 to 44, 0x0800E9B0: the side's statistic
    /// 3), Roll SP's (formula 1, 0x0800E8B4: the side's holy panels),
    /// Django SP's (formula 22, 0x0800E98A: the turns before this one).
    Count {
        of: Counted,
        by_count: Vec<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        operation_battle: Option<u16>,
    },
}

/// What a `DamageFormula::Count` counts.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Counted {
    /// The side's statistics counter `n` (`sub_800AB3A`).
    SideStat(u8),
    /// The field's panels of this type the side owns
    /// (`object_dead_getPanelsTypeAllianceCount`).
    OwnPanels(crate::field::PanelType),
    /// The field's panels of this type, whichever side owns them
    /// (`object_dead_getPanelsTypeAllianceCount` for each side: EXE4's DS
    /// navi chips, 0x08019518, by the field's holes).
    Panels(crate::field::PanelType),
    /// The turns before this one: the turn number less one, as unsigned
    /// (turn 0 counts as the most).
    TurnsBefore,
}

/// One battle chip's record (docs/engine/chips.md §1.2): what the rules
/// reads of it. Its use (an action, or a dimming, navi or instant hook) is
/// its definition's (`ChipDef::usage`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipData {
    /// The lines of the description the custom screen shows (R; its text
    /// is the content's strings, `Content::strings`): the description box
    /// takes keys a tick later for each line after the first
    /// (docs/engine/custom-screen.md §3.5). A chip without one counts as
    /// three, what nearly every chip has. The define phase counts it from
    /// the content's own strings.
    #[serde(skip_deserializing, default = "super::strings::three_lines")]
    pub description_lines: u8,
    /// The palette of the chip's picture on the custom screen, 16 BGR555
    /// colors, for a chip whose palette no ROM holds (the pack's picture
    /// has a black one): presentation only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art_palette: Option<[u16; 16]>,
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
    /// Input lockout after the attack ends, in ticks.
    pub lockout: u8,
    #[serde(default)]
    pub extra_flags: ExtraChipFlags,
    /// Base damage (0 for a chip whose damage is a `formula`).
    pub damage: u16,
    /// How the damage is worked out, for a chip whose damage isn't fixed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<DamageFormula>,
    /// What the rules ask of this chip in particular.
    #[serde(default)]
    pub traits: ChipTraits,
    /// A trap chip: what it catches as the side's defensive chip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trap: Option<Trap>,
    /// Uses per battle through the Battle Chip Gate's slot-in.
    pub slot_in_limit: u8,
    /// What the chip does to the chip picked before it, as a modifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier: Option<ChipModifier>,
    /// The Program Advances that make this chip, their ingredients by key.
    #[serde(default, rename = "program_advance", skip_serializing_if = "Vec::is_empty")]
    pub program_advances: Vec<ProgramAdvanceRecipe>,
}

