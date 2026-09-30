//! The game's setup records, decoded into the engine's types and encoded
//! back: the one place that knows their layout (what traces, real saves
//! and link data carry). Only the bytes the engine uses are modeled; each
//! decoder notes the bytes it leaves out.

use bn6_battle::content::Content;
use bn6_battle::custom::folder::FOLDER_SIZE;
use bn6_battle::custom::{BattleFolder, FolderChip};
use bn6_battle::hand::ChipHand;
use bn6_battle::setup::{
    BattleSettings, Form, GaugeSpeed, Navi, NaviCustBugs, NaviStats, NaviWeapons, SpTimes, Stage, SupportNavis,
};
use bn6_battle::transform::TransformRequest;

// ---- Navi stats ----------------------------------------------------------------

/// A navi's in-battle stats from the game's 0x64-byte NaviStats block.
pub fn navi_stats(b: &[u8; 0x64]) -> NaviStats {
    let u16at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let flag = |i: usize| b[i] != 0;
    NaviStats {
        attack: b[0x01],
        rapid: b[0x02],
        charge: b[0x03],
        first_barrier: b[0x06],
        gauge_speed: match b[0x08] {
            0 => GaugeSpeed::Normal,
            1 => GaugeSpeed::Fast,
            2 => GaugeSpeed::Slow,
            v => panic!("gauge speed {v}"),
        },
        reg_up: b[0x09],
        custom_level: b[0x0A],
        mega_level: b[0x0B],
        giga_level: b[0x0C],
        support: (b[0x0D] != 0xFF).then(|| SupportNavis {
            rush: b[0x0D] & 1 != 0,
            beat: b[0x0D] & 2 != 0,
            tango: b[0x0D] & 4 != 0,
        }),
        mood: b[0x0E],
        element: b[0x10],
        starting_form: Form(b[0x17]),
        float_shoes: flag(0x1B),
        air_shoes: flag(0x1C),
        undershirt: flag(0x1D),
        super_armor: flag(0x23),
        version: b[0x20],
        beast_out_counter: b[0x21],
        sun: flag(0x22),
        navi: Navi(b[0x29]),
        navi_variant: b[0x2B],
        form: Form(b[0x2C]),
        folder: b[0x2D],
        folder_reg: [b[0x2E], b[0x2F]],
        max_base_hp: u16at(0x3E),
        hp: u16at(0x40),
        max_hp: u16at(0x42),
        chip_recovery: u16at(0x50),
        folder_tags: [[b[0x56], b[0x57]], [b[0x58], b[0x59]]],
        chip_shuffle: flag(0x60),
        number_open: b[0x61] == 1,
        weapons: NaviWeapons {
            buster: b[0x04],
            charge_shot: b[0x05],
            back_special: b[0x07],
            a_charge: b[0x39],
            mode9_a: b[0x44],
            buster_shot: b[0x4D],
            charge_shot_kind: b[0x4F],
        },
        bugs: NaviCustBugs {
            auto_step: b[0x11],
            panel_trail_kind: b[0x12],
            panel_trail_level: b[0x13],
            buster_blanks: b[0x14],
            buster_charged: b[0x15],
            hit_status: b[0x16],
            hp_drain: b[0x18],
            custom_drain: b[0x19],
            battle_start: b[0x1A],
            emotion: b[0x24],
            processing: b[0x31],
            starting_damage: b[0x3D],
            status_immunity: flag(0x52),
            custom_damage: u16at(0x54),
            hand_shrink_turn: b[0x63],
        },
    }
}

/// The game's 0x64-byte NaviStats block of the modeled fields (the other
/// bytes are zero).
pub fn navi_stats_bytes(s: &NaviStats) -> [u8; 0x64] {
    let mut b = [0u8; 0x64];
    let put16 = |b: &mut [u8; 0x64], i: usize, v: u16| b[i..i + 2].copy_from_slice(&v.to_le_bytes());
    b[0x01] = s.attack;
    b[0x02] = s.rapid;
    b[0x03] = s.charge;
    b[0x06] = s.first_barrier;
    b[0x08] = s.gauge_speed as u8;
    b[0x09] = s.reg_up;
    b[0x0A] = s.custom_level;
    b[0x0B] = s.mega_level;
    b[0x0C] = s.giga_level;
    b[0x0D] = match s.support {
        None => 0xFF,
        Some(n) => n.rush as u8 | (n.beat as u8) << 1 | (n.tango as u8) << 2,
    };
    b[0x0E] = s.mood;
    b[0x10] = s.element;
    b[0x17] = s.starting_form.0;
    b[0x1B] = s.float_shoes as u8;
    b[0x1C] = s.air_shoes as u8;
    b[0x1D] = s.undershirt as u8;
    b[0x23] = s.super_armor as u8;
    b[0x20] = s.version;
    b[0x21] = s.beast_out_counter;
    b[0x22] = s.sun as u8;
    b[0x29] = s.navi.0;
    b[0x2B] = s.navi_variant;
    b[0x2C] = s.form.0;
    b[0x2D] = s.folder;
    b[0x2E] = s.folder_reg[0];
    b[0x2F] = s.folder_reg[1];
    put16(&mut b, 0x3E, s.max_base_hp);
    put16(&mut b, 0x40, s.hp);
    put16(&mut b, 0x42, s.max_hp);
    put16(&mut b, 0x50, s.chip_recovery);
    b[0x56] = s.folder_tags[0][0];
    b[0x57] = s.folder_tags[0][1];
    b[0x58] = s.folder_tags[1][0];
    b[0x59] = s.folder_tags[1][1];
    b[0x60] = s.chip_shuffle as u8;
    b[0x61] = s.number_open as u8;
    let w = &s.weapons;
    b[0x04] = w.buster;
    b[0x05] = w.charge_shot;
    b[0x07] = w.back_special;
    b[0x39] = w.a_charge;
    b[0x44] = w.mode9_a;
    b[0x4D] = w.buster_shot;
    b[0x4F] = w.charge_shot_kind;
    let g = &s.bugs;
    b[0x11] = g.auto_step;
    b[0x12] = g.panel_trail_kind;
    b[0x13] = g.panel_trail_level;
    b[0x14] = g.buster_blanks;
    b[0x15] = g.buster_charged;
    b[0x16] = g.hit_status;
    b[0x18] = g.hp_drain;
    b[0x19] = g.custom_drain;
    b[0x1A] = g.battle_start;
    b[0x24] = g.emotion;
    b[0x31] = g.processing;
    b[0x3D] = g.starting_damage;
    b[0x52] = g.status_immunity as u8;
    put16(&mut b, 0x54, g.custom_damage);
    b[0x63] = g.hand_shrink_turn;
    b
}

// ---- Folders, hands, transformations ---------------------------------------------

/// A battle folder from the game's 0x3C-byte encoding (`eBattleFolder`:
/// packed chips, 0xFFFF = empty).
pub fn battle_folder(b: &[u8], regular_pending: bool) -> BattleFolder {
    BattleFolder {
        chips: std::array::from_fn(|i| {
            let v = u16::from_le_bytes([b[2 * i], b[2 * i + 1]]);
            (v != 0xFFFF).then(|| FolderChip::from_packed(v))
        }),
        regular_pending,
    }
}

/// A battle folder's encoding.
pub fn battle_folder_bytes(f: &BattleFolder) -> [u8; 2 * FOLDER_SIZE] {
    let mut b = [0u8; 2 * FOLDER_SIZE];
    for (i, c) in f.chips.iter().enumerate() {
        let v = c.map_or(0xFFFF, FolderChip::packed);
        b[2 * i..2 * i + 2].copy_from_slice(&v.to_le_bytes());
    }
    b
}

/// A chip hand from the game's 0x50-byte chip block. Byte 1 is always 0
/// in battle (only the battle flag 0x40 mode's unreferenced routines use
/// it) and is not kept.
pub fn chip_hand(b: &[u8]) -> ChipHand {
    let u16s = |off: usize| -> [u16; 6] { std::array::from_fn(|i| u16::from_le_bytes([b[off + 2 * i], b[off + 2 * i + 1]])) };
    ChipHand {
        cursor: b[0],
        ids: u16s(0x02),
        damage: u16s(0x0E),
        attack_bonus: u16s(0x1A),
        charge_bonus: u16s(0x26),
        selection: u16s(0x32),
        turn: b[0x3E..0x44].try_into().unwrap(),
        modifiers: b[0x44..0x4A].try_into().unwrap(),
    }
}

/// A chip hand's 0x50-byte chip block.
pub fn chip_hand_bytes(h: &ChipHand) -> [u8; 0x50] {
    let mut b = [0u8; 0x50];
    b[0] = h.cursor;
    let mut put = |off: usize, v: &[u16; 6]| {
        for (i, x) in v.iter().enumerate() {
            b[off + 2 * i..off + 2 * i + 2].copy_from_slice(&x.to_le_bytes());
        }
    };
    put(0x02, &h.ids);
    put(0x0E, &h.damage);
    put(0x1A, &h.attack_bonus);
    put(0x26, &h.charge_bonus);
    put(0x32, &h.selection);
    b[0x3E..0x44].copy_from_slice(&h.turn);
    b[0x44..0x4A].copy_from_slice(&h.modifiers);
    b
}

/// A transformation request from the game's 0x10-byte record: +0 the
/// form, +4 the Cross change (0xFF = none for both). +1 and +3 are
/// custom-screen bookkeeping nothing in battle reads, and +8 names the
/// requesting navi object, which is always the side's player.
pub fn transform_request(b: &[u8]) -> TransformRequest {
    let opt = |v: u8| (v != 0xFF).then_some(v);
    TransformRequest { form: opt(b[0]).map(Form), cross_change: opt(b[4]) }
}

// ---- Battle settings, stages, SP times ---------------------------------------------

/// Netbattle settings from the game's 16-byte BattleSettings record. Bytes
/// 12..16 name the actor list by its original address, which `content`
/// resolves. Byte 1 (read by `GetBattleSettingsUnk01`, outside the battle
/// simulation) and byte 7 (no reader found) are not kept.
pub fn battle_settings(b: &[u8], content: &Content) -> BattleSettings {
    let address = u32::from_le_bytes(b[12..16].try_into().unwrap());
    let actors = content
        .rules
        .stages
        .actor_list_at(address)
        .unwrap_or_else(|| panic!("battle settings name an unknown actor list {address:#010x}"));
    BattleSettings {
        layout: b[0],
        music: b[2],
        mode: b[3],
        background: b[4],
        battle_number: b[5],
        panel_pattern: b[6],
        effects: u32::from_le_bytes(b[8..12].try_into().unwrap()),
        actors,
    }
}

/// The init exchange's two stage pairs (`byte_203CA50`: settings index,
/// then background, per round).
pub fn later_stages(b: &[u8]) -> [Stage; 2] {
    [Stage { settings: b[0], background: b[1] }, Stage { settings: b[2], background: b[3] }]
}

/// A player's SP navi deletion times (`byte_203EB00`, 0x28 bytes).
pub fn sp_times(b: &[u8]) -> SpTimes {
    SpTimes(std::array::from_fn(|i| u16::from_le_bytes([b[2 * i], b[2 * i + 1]])))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The left navi's stats at the start of the machgun replay.
    const MACHGUN_P0: &str = "08000000000100ff00320505010080000000ff00000000000000000101000001010301000000001f0000000a0000ffffff0000000000000000ff00000000e803e803e8030000010000000a0000000000000000000000ffffffffffff0000000000000000";

    fn bytes(hex: &str) -> [u8; 0x64] {
        let v: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        v.try_into().unwrap()
    }

    #[test]
    fn navi_stats_decode() {
        let s = navi_stats(&bytes(MACHGUN_P0));
        assert_eq!((s.hp, s.max_hp, s.max_base_hp), (1000, 1000, 1000));
        assert_eq!(s.navi, Navi::MEGAMAN);
        assert_eq!(s.form, Form::NONE);
        assert!(s.float_shoes && s.air_shoes && !s.undershirt && !s.super_armor);
        assert_eq!(s.mood, 0x80);
        assert_eq!(s.support, Some(SupportNavis::default()));
        assert_eq!((s.weapons.buster, s.weapons.charge_shot, s.weapons.back_special), (0, 1, 0xFF));
        assert_eq!(s.beast_out_counter, 3);
    }

    #[test]
    fn navi_stats_encode_round_trips() {
        let s = navi_stats(&bytes(MACHGUN_P0));
        assert_eq!(navi_stats(&navi_stats_bytes(&s)), s);
    }

    /// A bug code sets the stat byte it names: the engine's setter does
    /// what setting that byte of the game's block does, and refuses the
    /// bytes the engine doesn't model.
    #[test]
    fn bug_codes_set_the_byte_they_name() {
        let base = navi_stats(&bytes(MACHGUN_P0));
        let mut problems = Vec::new();
        std::panic::set_hook(Box::new(|_| {}));
        for offset in 1..0x64u8 {
            let values: &[u8] = if offset == 0x08 { &[0, 1, 2] } else { &[0, 1, 2, 0x7F, 0xFF] };
            let decoded = |v: u8| {
                let mut raw = bytes(MACHGUN_P0);
                raw[offset as usize] = v;
                navi_stats(&raw)
            };
            let modeled = values.iter().any(|&v| decoded(v) != base);
            for &value in values {
                let result = std::panic::catch_unwind(move || {
                    let mut s = base;
                    s.set_byte_by_bug_code(offset, value);
                    s
                });
                match result {
                    Ok(s) if modeled && s != decoded(value) => problems.push(format!("{offset:#x} = {value:#x}: {s:?}")),
                    Ok(_) if !modeled => problems.push(format!("{offset:#x} isn't modeled but is accepted")),
                    Err(_) if modeled => problems.push(format!("{offset:#x} = {value:#x} is modeled but refused")),
                    _ => {}
                }
            }
        }
        let _ = std::panic::take_hook();
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn hands_round_trip() {
        let mut b = [0u8; 0x50];
        for (i, x) in b.iter_mut().enumerate() {
            *x = (i * 7) as u8;
        }
        b[1] = 0;
        b[0x4A..].fill(0);
        assert_eq!(chip_hand_bytes(&chip_hand(&b)), b);
    }
}
