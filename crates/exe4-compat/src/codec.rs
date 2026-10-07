//! EXE4's records in the engine's terms: the 0x40-byte NaviStats, the
//! field's panels, the chip blocks, the link record.
//!
//! What needs EXE4's content (its navis', forms' and weapons' handles, its
//! chips' handles) is kept as the original's number, or a chip's compat key;
//! the recordings' decode (`trace`) turns them into the engine's.
//! docs/design/exe4-map.md §3.3 says what each NaviStats field is.

use crate::Compat;

/// A side's NaviStats block: 0x40 bytes in EXE4 (EXE5's 0x60, EXE6's 0x64).
pub const NAVI_STATS: usize = 0x40;
/// A side's chip block: 0x50 bytes, EXE6's layout.
pub const CHIP_BLOCK: usize = 0x50;

/// A side's stats from EXE4's 0x40-byte NaviStats block, each field at
/// EXE4's offset (exe4-map.md §3.3: the defaults a new block gets,
/// 0x0800D6BE; the patch cards' effects, which set the block's bytes by
/// offset; the paired reads of EXE6's and EXE5's routines). What isn't
/// read yet stays in `raw`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NaviStats {
    /// +0x00: the mood (a new block's 0x99; the round's start sets it from
    /// the light/dark value).
    pub mood: u8,
    /// +0x01 to +0x04: SprArmr's super armor, FlotShoe's, AirShoes' and
    /// UnderSht's (the NaviCust's, 0x08041A50's handlers).
    pub super_armor: bool,
    pub float_shoes: bool,
    pub air_shoes: bool,
    pub undershirt: bool,
    /// +0x05, +0x06, +0x07: the buster's attack, rapid and charge levels
    /// (EXE6's +0x01 to +0x03).
    pub attack: u8,
    pub rapid: u8,
    pub charge: u8,
    /// +0x08: the buster's blank count (a draw of 1 to 16 no greater than it
    /// fires nothing: the NaviCust's and patch cards' bug).
    pub buster_blanks: u8,
    /// +0x09, +0x0A: the B button's and the charged weapon's routines (the
    /// table at 0x0800CA7C: 0 the buster, 1 the charged shot).
    pub buster_weapon: u8,
    pub charged_weapon: u8,
    /// +0x0B: BustPack's weapon level (0 to 2), which the charged shot reads.
    pub weapon_level: u8,
    /// +0x0C: B+Left's routine (patch card 0x0C; 0xFF none).
    pub back_special: Option<u8>,
    /// +0x0D: the move bug (0xFF confused at the start, 0x10 and 0x20 steps
    /// of its own right and left; 0 none).
    pub move_bug: u8,
    /// +0x0E, +0x0F: the HP drain and the custom gauge drain bugs (EXE6's
    /// +0x18, +0x19).
    pub hp_drain: u8,
    pub custom_drain: u8,
    /// +0x12: the custom level, the chips dealt (patch card 0x12, up to 8).
    pub custom_level: u8,
    /// +0x13, +0x14: the Mega and Giga chip limits.
    pub mega_level: u8,
    pub giga_level: u8,
    /// +0x18: the supports (patch card 0x18, Triple Supporter; EXE6's +0x0D),
    /// as the block has it: what its bits are is to read.
    pub supports: u8,
    /// +0x1B: the panel a step leaves (patch card 0x1B: 1 broken, 3 cracked, 5
    /// metal, 9 holy, EXE4's panel numbers; 0xFF none).
    pub panel_trail: Option<u8>,
    /// +0x1F: Full Synchro at the round's start (patch card 0x1F; EXE6's
    /// +0x0F).
    pub full_synchro: bool,
    /// +0x21: the aura the round starts with (patch card 0x21: 2 Barrier100, 3
    /// Barrier200, 6 LifeAura; 0 none; EXE6's +0x06).
    pub aura: u8,
    /// +0x23: the navi's number (MegaMan's 0).
    pub navi: u8,
    /// +0x24: the soul a round starts in (patch card 0x24; 0 none, his base
    /// form).
    pub soul: u8,
    /// +0x25: the move lag's column (0x0800C208's table, by the navi).
    pub move_lag_column: u8,
    /// +0x27: MegaMan's color (patch card 0x27).
    pub color: u8,
    /// +0x28: All Guard (patch card 0x28).
    pub all_guard: bool,
    /// +0x30, +0x32, +0x34: HP, max HP and the base max HP (before the
    /// NaviCust's and the patch cards': the save's 0x21CA).
    pub hp: u16,
    pub max_hp: u16,
    pub max_base_hp: u16,
    /// +0x36: the light/dark value (a halfword: 500 a new block's; Tango's
    /// dark netbattle save 460, its light ones 1000).
    pub light_dark: u16,
    /// The whole block.
    pub raw: [u8; NAVI_STATS],
}

/// The offsets [`NaviStats`] names (the rest of the block isn't read yet:
/// exe4-map.md §3.3).
pub const NAMED_OFFSETS: &[usize] = &[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x12, 0x13, 0x14, 0x18, 0x1B, 0x1F, 0x21, 0x23, 0x24, 0x25, 0x27,
    0x28, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
];

/// A side's stats from its NaviStats block.
pub fn navi_stats(b: &[u8; NAVI_STATS]) -> NaviStats {
    let u16at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let unless_ff = |i: usize| (b[i] != 0xFF).then_some(b[i]);
    NaviStats {
        mood: b[0x00],
        super_armor: b[0x01] != 0,
        float_shoes: b[0x02] != 0,
        air_shoes: b[0x03] != 0,
        undershirt: b[0x04] != 0,
        attack: b[0x05],
        rapid: b[0x06],
        charge: b[0x07],
        buster_blanks: b[0x08],
        buster_weapon: b[0x09],
        charged_weapon: b[0x0A],
        weapon_level: b[0x0B],
        back_special: unless_ff(0x0C),
        move_bug: b[0x0D],
        hp_drain: b[0x0E],
        custom_drain: b[0x0F],
        custom_level: b[0x12],
        mega_level: b[0x13],
        giga_level: b[0x14],
        supports: b[0x18],
        panel_trail: unless_ff(0x1B),
        full_synchro: b[0x1F] != 0,
        aura: b[0x21],
        navi: b[0x23],
        soul: b[0x24],
        move_lag_column: b[0x25],
        color: b[0x27],
        all_guard: b[0x28] != 0,
        hp: u16at(0x30),
        max_hp: u16at(0x32),
        max_base_hp: u16at(0x34),
        light_dark: u16at(0x36),
        raw: *b,
    }
}

/// A panel as the recordings have it: EXE4's panel type number (+0) and its
/// owner (+1). Which of the engine's panel types a number is, is the
/// game's rules' to say (`panels.numbers`, which the round runs on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panel {
    pub number: u8,
    pub alliance: u8,
}

/// A panel from its EXE4 type number and owner. (EXE4 has 12 panel types,
/// 0x0800A3A8.)
pub fn panel(number: u8, alliance: u8) -> Result<Panel, String> {
    if alliance > 1 {
        return Err(format!("panel owner {alliance}"));
    }
    if number > 11 {
        return Err(format!("panel type {number}: EXE4's are 0 to 11"));
    }
    Ok(Panel { number, alliance })
}

/// A chip in a hand or a selection: its id and, where compat has it, its
/// key (`cannon`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip {
    pub id: u16,
    pub key: Option<String>,
}

/// A side's chip block (0x50 bytes at 0x02035CB0 + 0x50 side, EXE6's
/// layout: `ChipHand`'s fields), the chips by id and compat key.
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

/// EXE4's link record (exe4-map.md §18 item 23, exe4-against-exe3.md §4.5):
/// what each console sends the other as a netbattle's battle starts, built
/// at 0x0203BD40 by 0x08008708 and received at 0x0203E390 (side 0's) and
/// 0x0203E490 (side 1's). Its design is EXE3's: the sender's RNG2 and its
/// MegaMan's stat block in it, side 0's RNG2 seeding the battle's
/// (0x080087DA reads 0x0203E394).
pub const LINK_RECORD: usize = 0xC4;
const LINK_MAGIC: u32 = 0x1234_5678;

/// A link record, as the battle's unpack (0x080087A8) reads it: the
/// sender's RNG2 (+0x04), its battle settings record's address (+0x08:
/// BattleState +0x3C), its MegaMan's NaviStats (+0x0C: +0x2A cleared in a
/// battle mode from 0x46 when event flag 0x1184 is set, +0x36 set to 500
/// when 0x08006570 says so, both as the sender built it). The rest (0x2C
/// bytes each from +0x4C and +0x78, 0x10 from +0xA4, two words at +0xB4 and
/// +0xB8, 8 bytes at +0xBC) goes past the battle's unpack, and stays in
/// `raw`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRecord {
    pub rng2: u32,
    pub settings: u32,
    pub navi_stats: NaviStats,
    pub raw: [u8; LINK_RECORD],
}

impl LinkRecord {
    /// A record from its bytes: refused when it doesn't start with the
    /// magic word (0x12345678) the sender writes.
    pub fn from_bytes(b: &[u8]) -> Result<LinkRecord, String> {
        let raw: [u8; LINK_RECORD] = b.try_into().map_err(|_| format!("a link record of {:#x} bytes, not {LINK_RECORD:#x}", b.len()))?;
        let word = |i: usize| u32::from_le_bytes(raw[i..i + 4].try_into().unwrap());
        if word(0) != LINK_MAGIC {
            return Err(format!("a link record starting {:#010x}, not the magic {LINK_MAGIC:#010x}", word(0)));
        }
        let stats: [u8; NAVI_STATS] = raw[0x0C..0x0C + NAVI_STATS].try_into().unwrap();
        Ok(LinkRecord { rng2: word(0x04), settings: word(0x08), navi_stats: navi_stats(&stats), raw })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lab's buster duel's side 0 as the battle starts (flow/buster-duel's
    /// setup line): a light save's MegaMan with HP 1000, the charged shot on
    /// B's charge, no B+Left, no trail.
    #[test]
    fn a_navi_stats_block_reads() {
        let hex = "000000000000000000000100ff000000200405050100001f000000ff0000000001000000000a00000001010000000000e803e803e803e8030000000000000000";
        let b: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        let s = navi_stats(&b.try_into().unwrap());
        assert_eq!((s.buster_weapon, s.charged_weapon, s.back_special, s.panel_trail), (0, 1, None, None));
        assert_eq!((s.custom_level, s.mega_level, s.giga_level), (5, 5, 1));
        assert_eq!((s.hp, s.max_hp, s.max_base_hp, s.light_dark), (1000, 1000, 1000, 1000));
        assert_eq!((s.navi, s.soul, s.aura, s.full_synchro), (0, 0, 0, false));
    }

    #[test]
    fn a_link_record_reads_and_wants_its_magic() {
        let mut r = vec![0u8; LINK_RECORD];
        r[..4].copy_from_slice(&LINK_MAGIC.to_le_bytes());
        r[4..8].copy_from_slice(&0x2EBB_6C64u32.to_le_bytes());
        r[8..12].copy_from_slice(&0x080F_C3FCu32.to_le_bytes());
        r[0x0C + 0x30..0x0C + 0x32].copy_from_slice(&760u16.to_le_bytes());
        let l = LinkRecord::from_bytes(&r).unwrap();
        assert_eq!((l.rng2, l.settings, l.navi_stats.hp), (0x2EBB_6C64, 0x080F_C3FC, 760));
        r[0] = 0;
        assert!(LinkRecord::from_bytes(&r).unwrap_err().contains("magic"));
        assert!(LinkRecord::from_bytes(&r[..0x40]).is_err());
    }

    #[test]
    fn a_chip_block_reads_with_exe6s_layout() {
        // custom/cannon's side 0 as the fight starts: Cannon A picked, its 40
        // damage.
        let mut b = vec![0u8; CHIP_BLOCK];
        b[0x02..0x0E].copy_from_slice(&[0x01, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        b[0x0E] = 0x28;
        b[0x32..0x3E].copy_from_slice(&[0x01, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        b[0x44] = 4;
        let h = chip_hand(Compat::exe4(), &b).unwrap();
        assert_eq!(h.ids[0].as_ref().and_then(|c| c.key.as_deref()), Some("cannon"));
        assert_eq!((h.damage[0], h.modifiers[0]), (40, 4));
        assert_eq!(h.selection[0].as_ref().map(|(c, code)| (c.id, *code)), Some((1, 0)));
        assert!(h.ids[1].is_none());
    }
}
