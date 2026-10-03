//! BN5's records in the engine's terms: the 0x60-byte NaviStats (with the
//! light/dark value), the field's panels, the chip blocks.
//!
//! What needs BN5's content (its navis', forms' and weapons' handles, its
//! chips' handles) is kept as the original's number, or a chip's compat
//! key: BN5's content has no root to give handles until rules-in-luau.md's
//! R. docs/design/bn5-map.md §13 lists every field that has no engine
//! counterpart.

use crate::Compat;
use nettai_battle::field::PanelType;
use nettai_battle::setup::{GaugeSpeed, Supports};

/// A side's NaviStats block: 0x60 bytes in BN5 (BN6's is 0x64).
pub const NAVI_STATS: usize = 0x60;
/// A panel record: 0x24 bytes in BN5 (BN6's 0x20). The traces carry each
/// panel's type (+2) and owner (+3) only.
pub const PANEL_RECORD: usize = 0x24;
/// A side's chip block: 0x50 bytes, BN6's layout.
pub const CHIP_BLOCK: usize = 0x50;

/// BN5's light/dark value (NaviStats +0x44, a halfword: bn5-map.md §6.1):
/// Tango's finished Team ProtoMan save has 500, its Team Colonel save 0,
/// its light netplay templates 1000. The engine has no counterpart (BN5's
/// light-and-dark system, not built).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LightDark(pub u16);

impl LightDark {
    /// The chips' threshold (0x08010118): a light-only chip needs at least
    /// this, a dark-only one less.
    pub const LIGHT_CHIPS: u16 = 470;
    /// The holy panels' threshold (0x08017136): at or below it, a holy
    /// panel under the navi turns Normal every tick.
    pub const CLEARS_HOLY: u16 = 499;

    /// Whether this MegaMan may use a chip whose record names `megaman`
    /// (+0x15: 0 either, 1 light, 2 dark).
    pub fn may_use(self, megaman: u8) -> bool {
        match megaman {
            1 => self.0 >= Self::LIGHT_CHIPS,
            2 => self.0 < Self::LIGHT_CHIPS,
            _ => true,
        }
    }

    /// Whether a holy panel this navi stands on turns Normal.
    pub fn clears_holy(self) -> bool {
        self.0 <= Self::CLEARS_HOLY
    }
}

/// A side's in-battle stats from BN5's 0x60-byte NaviStats block. The
/// fields named as the engine's `NaviStats` are at BN6's offsets, as the
/// pairing of BN5's accessor calls with BN6's found them (bn5-map.md §3.3:
/// `fields.py navistats`); the navi, the form and the weapons are the
/// original's numbers (BN5's content has no handles yet); the rest of the
/// block is kept as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NaviStats {
    pub attack: u8,
    pub rapid: u8,
    pub charge: u8,
    /// The first barrier's type (+0x06: 0 none; Team ProtoMan's FstBarr 1).
    pub first_barrier: u8,
    pub gauge_speed: GaugeSpeed,
    pub reg_up: u8,
    pub custom_level: u8,
    pub mega_level: u8,
    pub giga_level: u8,
    pub support: Option<Supports>,
    pub mood: u8,
    pub element: u8,
    pub float_shoes: bool,
    pub air_shoes: bool,
    pub undershirt: bool,
    pub super_armor: bool,
    /// The operated navi's number (+0x29; 0 MegaMan).
    pub navi: u8,
    pub navi_variant: u8,
    /// The form's number (+0x2C: BN5's souls, bn5-map.md §5).
    pub form: u8,
    pub max_base_hp: u16,
    pub hp: u16,
    pub max_hp: u16,
    /// The weapon routine bytes (+0x04, +0x05, +0x07, +0x39): BN6's buster
    /// (+0x04) is BN5's +0x39 and BN6's +0x39 BN5's +0x04 in the one call
    /// each that pairs them (to confirm), so they are kept by offset.
    pub weapon_bytes: [u8; 4],
    /// BN5's own: the light/dark value.
    pub light_dark: LightDark,
    /// BN5's own (+0x4C): Hub Style, which patch card 111 (0x6F) sets when
    /// installed and on (0x08138214): 1, else 0.
    pub hub_style: u8,
    /// The whole block.
    pub raw: [u8; NAVI_STATS],
}

/// The offsets [`NaviStats`] names (the rest of the block has no engine
/// field, or BN5's meaning isn't read: bn5-map.md §13).
pub const NAMED_OFFSETS: &[usize] = &[
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x10, 0x1B, 0x1C, 0x1D, 0x23,
    0x29, 0x2B, 0x2C, 0x39, 0x3E, 0x3F, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45,
];

/// A side's stats from its NaviStats block.
pub fn navi_stats(b: &[u8; NAVI_STATS]) -> Result<NaviStats, String> {
    let u16at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let flag = |i: usize| b[i] != 0;
    Ok(NaviStats {
        attack: b[0x01],
        rapid: b[0x02],
        charge: b[0x03],
        first_barrier: b[0x06],
        gauge_speed: match b[0x08] {
            0 => GaugeSpeed::Normal,
            1 => GaugeSpeed::Fast,
            2 => GaugeSpeed::Slow,
            v => return Err(format!("NaviStats+0x08: gauge speed {v}")),
        },
        reg_up: b[0x09],
        custom_level: b[0x0A],
        mega_level: b[0x0B],
        giga_level: b[0x0C],
        support: (b[0x0D] != 0xFF).then(|| Supports {
            rush: b[0x0D] & 1 != 0,
            beat: b[0x0D] & 2 != 0,
            tango: b[0x0D] & 4 != 0,
        }),
        mood: b[0x0E],
        element: b[0x10],
        float_shoes: flag(0x1B),
        air_shoes: flag(0x1C),
        undershirt: flag(0x1D),
        super_armor: flag(0x23),
        navi: b[0x29],
        navi_variant: b[0x2B],
        form: b[0x2C],
        max_base_hp: u16at(0x3E),
        hp: u16at(0x40),
        max_hp: u16at(0x42),
        weapon_bytes: [b[0x04], b[0x05], b[0x07], b[0x39]],
        light_dark: LightDark(u16at(0x44)),
        hub_style: b[0x4C],
        raw: *b,
    })
}

/// A panel as the traces record it ((type, owner) by BN5's numbers), in the
/// engine's terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panel {
    /// BN5's panel type number.
    pub number: u8,
    /// The engine's panel type; none for BN5's metal and sea panels.
    pub kind: Option<PanelType>,
    pub alliance: u8,
}

/// A panel from its BN5 type number and owner.
pub fn panel(compat: &Compat, number: u8, alliance: u8) -> Result<Panel, String> {
    if alliance > 1 {
        return Err(format!("panel owner {alliance}"));
    }
    Ok(Panel { number, kind: compat.panel_type(number)?, alliance })
}

/// A chip in a hand or a selection: its id and, where compat has it, its
/// qualified key (`bn5:cannon`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip {
    pub id: u16,
    pub key: Option<String>,
}

/// A side's chip block (0x50 bytes, BN6's layout: `ChipHand`'s fields), the
/// chips by id and compat key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChipHand {
    pub cursor: u8,
    pub ids: [Option<Chip>; 6],
    pub damage: [u16; 6],
    pub attack_bonus: [u16; 6],
    pub charge_bonus: [u16; 6],
    /// The picks: the chip and its code.
    pub selection: [Option<(Chip, u8)>; 6],
    pub turn: [u8; 6],
    pub modifiers: [u8; 6],
}

/// A chip block.
pub fn chip_hand(compat: &Compat, b: &[u8]) -> Result<ChipHand, String> {
    if b.len() != CHIP_BLOCK {
        return Err(format!("a chip block of {:#x} bytes", b.len()));
    }
    let u16s = |off: usize| -> [u16; 6] { std::array::from_fn(|i| u16::from_le_bytes([b[off + 2 * i], b[off + 2 * i + 1]])) };
    let chip = |id: u16| Chip { id, key: compat.chip(id) };
    Ok(ChipHand {
        cursor: b[0],
        ids: u16s(0x02).map(|v| (v != 0xFFFF).then(|| chip(v))),
        damage: u16s(0x0E),
        attack_bonus: u16s(0x1A),
        charge_bonus: u16s(0x26),
        selection: u16s(0x32).map(|v| (v != 0xFFFF).then(|| (chip(v & 0x1FF), (v >> 9) as u8))),
        turn: b[0x3E..0x44].try_into().unwrap(),
        modifiers: b[0x44..0x4A].try_into().unwrap(),
    })
}
