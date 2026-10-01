//! The game's setup records, decoded into the engine's types and encoded
//! back: the one place that knows their layout (what traces, real saves
//! and link data carry). Only the bytes the engine uses are modeled; each
//! decoder notes the bytes it leaves out.
//!
//! The engine's state holds handles; the records hold the original's
//! numbers (chip ids, navi and form numbers, weapon routines, battle
//! settings indices). [`Ids`] maps one to the other through compat's keys.

use crate::Compat;
use bn6_battle::content::{ChipCode, ChipId, Content};
use bn6_battle::custom::folder::FOLDER_SIZE;
use bn6_battle::custom::{BattleFolder, FolderChip};
use bn6_battle::hand::ChipHand;
use bn6_battle::setup::{
    BattleSettings, Form, GaugeSpeed, Navi, NaviCustBugs, NaviStats, NaviWeapons, SpTimes, Stage, Supports,
};
use bn6_battle::transform::TransformRequest;
use bn6_content_api::{ChipHandle, FormHandle, NaviHandle, StageHandle, WeaponHandle};

// ---- Numbers and handles ----------------------------------------------------------

/// The original's numbers for a content's identities, and back. A number
/// names compat's key, and a definition of that key is the thing (a chip
/// content defines, reached from a recorded folder); else the pack's
/// record with that number (its transitional key, `v1/chip-036`). Back, a
/// pack record gives its own number and a definition compat's for its key.
///
/// Numbers a setup can't hold panic: the records are the original's, and
/// the content is BN6's.
#[derive(Clone, Copy)]
pub struct Ids<'a> {
    pub content: &'a Content,
    pub compat: &'a Compat,
}

impl<'a> Ids<'a> {
    pub fn new(content: &'a Content, compat: &'a Compat) -> Ids<'a> {
        Ids { content, compat }
    }

    /// The chip with this id.
    pub fn chip(&self, id: ChipId) -> ChipHandle {
        let defs = &self.content.defs;
        self.compat
            .chip_key(id)
            .and_then(|k| defs.chip_by_key(k))
            .or_else(|| defs.chip_numbered(id))
            .unwrap_or_else(|| panic!("chip {id:#x} is neither defined nor in the pack"))
    }

    /// A chip's id.
    pub fn chip_id(&self, h: ChipHandle) -> ChipId {
        // (A numbered record, which only the engine's test content has, is
        // its own number.)
        if let Some(id) = self.content.defs.chip(h).record.id {
            return id;
        }
        let key = &self.content.defs.chip(h).key;
        self.compat.chips.get(key).map(|c| c.id).unwrap_or_else(|| panic!("chips.toml has no {key:?}"))
    }

    /// A chip field: `none` for no chip.
    pub fn chip_field(&self, v: u16, none: u16) -> Option<ChipHandle> {
        (v != none).then(|| self.chip(v))
    }

    /// A chip field's value: `none` for no chip.
    pub fn chip_field_id(&self, h: Option<ChipHandle>, none: u16) -> u16 {
        h.map_or(none, |h| self.chip_id(h))
    }

    /// A folder chip from the game's packed halfword (`code << 9 | id`).
    pub fn folder_chip(&self, v: u16) -> FolderChip {
        FolderChip::new(self.chip(v & 0x1FF), ChipCode((v >> 9) as u8))
    }

    /// A folder chip's packed halfword.
    pub fn packed(&self, c: FolderChip) -> u16 {
        (c.code.0 as u16) << 9 | self.chip_id(c.id)
    }

    /// The navi with this number.
    pub fn navi(&self, navi: u8) -> NaviHandle {
        let defs = &self.content.defs;
        self.compat
            .navi_key(navi)
            .and_then(|k| defs.navi_by_key(k))
            .or_else(|| defs.navi_numbered(Navi(navi)))
            .unwrap_or_else(|| panic!("navi {navi:#x} is neither defined nor in the pack"))
    }

    /// A navi's number.
    pub fn navi_number(&self, h: NaviHandle) -> u8 {
        // The engine's navis are the pack's, by number (content defines
        // none of its own yet).
        self.content.defs.navi(h).record.id
    }

    /// MegaMan's form with this number.
    pub fn form(&self, form: u8) -> FormHandle {
        let defs = &self.content.defs;
        self.compat
            .form_key(form)
            .and_then(|k| defs.form_by_key(k))
            .or_else(|| defs.form_numbered(Form(form)))
            .unwrap_or_else(|| panic!("form {form:#x} is neither defined nor in the pack"))
    }

    /// A form's number.
    pub fn form_number(&self, h: FormHandle) -> u8 {
        // MegaMan's forms are the pack's, by number.
        self.content.defs.form(h).record.id
    }

    /// The weapon a routine number names; none for 0xFF.
    pub fn weapon(&self, routine: u8) -> Option<WeaponHandle> {
        if routine == 0xFF {
            return None;
        }
        let defs = &self.content.defs;
        let h = self
            .compat
            .weapon_key(routine)
            .and_then(|k| defs.weapon_by_key(k))
            .or_else(|| defs.weapon_numbered(routine))
            .unwrap_or_else(|| panic!("weapon routine {routine:#x} is neither defined nor in the pack"));
        Some(h)
    }

    /// A weapon's routine number (the first compat gives a weapon content
    /// defines); 0xFF for none.
    pub fn weapon_number(&self, w: Option<WeaponHandle>) -> u8 {
        let Some(w) = w else { return 0xFF };
        if let Some(n) = self.content.weapon_number(w) {
            return n;
        }
        let key = &self.content.defs.weapon(w).key;
        let numbers = self.compat.weapons.get(key).unwrap_or_else(|| panic!("weapons.toml has no {key:?}"));
        *numbers.first().unwrap_or_else(|| panic!("weapons.toml gives {key:?} no number"))
    }

    /// The stage a battle settings index names.
    pub fn stage(&self, settings: u8) -> StageHandle {
        let defs = &self.content.defs;
        self.compat
            .stage_key(settings)
            .and_then(|k| defs.stage_by_key(k))
            .or_else(|| defs.stage_numbered(settings))
            .unwrap_or_else(|| panic!("battle settings {settings:#x} are neither defined nor in the pack"))
    }

    /// A stage's battle settings index.
    pub fn stage_index(&self, h: StageHandle) -> u8 {
        let def = self.content.defs.stage(h);
        if let Some(n) = def.number {
            return n;
        }
        let e = self.compat.stages.get(&def.key).unwrap_or_else(|| panic!("stages.toml has no {:?}", def.key));
        *e.settings.first().unwrap_or_else(|| panic!("stages.toml gives {:?} no settings", def.key))
    }
}

// ---- Navi stats ----------------------------------------------------------------

/// A navi's in-battle stats from the game's 0x64-byte NaviStats block.
pub fn navi_stats(b: &[u8; 0x64], ids: &Ids) -> NaviStats {
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
        support: (b[0x0D] != 0xFF).then(|| Supports {
            rush: b[0x0D] & 1 != 0,
            beat: b[0x0D] & 2 != 0,
            tango: b[0x0D] & 4 != 0,
        }),
        mood: b[0x0E],
        element: b[0x10],
        starting_form: ids.form(b[0x17]),
        float_shoes: flag(0x1B),
        air_shoes: flag(0x1C),
        undershirt: flag(0x1D),
        super_armor: flag(0x23),
        version: b[0x20],
        beast_out_counter: b[0x21],
        sun: flag(0x22),
        navi: ids.navi(b[0x29]),
        navi_variant: b[0x2B],
        form: ids.form(b[0x2C]),
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
            buster: ids.weapon(b[0x04]),
            charge_shot: ids.weapon(b[0x05]),
            back_special: ids.weapon(b[0x07]),
            a_charge: ids.weapon(b[0x39]),
            mode9_a: ids.weapon(b[0x44]),
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
pub fn navi_stats_bytes(s: &NaviStats, ids: &Ids) -> [u8; 0x64] {
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
    b[0x17] = ids.form_number(s.starting_form);
    b[0x1B] = s.float_shoes as u8;
    b[0x1C] = s.air_shoes as u8;
    b[0x1D] = s.undershirt as u8;
    b[0x23] = s.super_armor as u8;
    b[0x20] = s.version;
    b[0x21] = s.beast_out_counter;
    b[0x22] = s.sun as u8;
    b[0x29] = ids.navi_number(s.navi);
    b[0x2B] = s.navi_variant;
    b[0x2C] = ids.form_number(s.form);
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
    b[0x04] = ids.weapon_number(w.buster);
    b[0x05] = ids.weapon_number(w.charge_shot);
    b[0x07] = ids.weapon_number(w.back_special);
    b[0x39] = ids.weapon_number(w.a_charge);
    b[0x44] = ids.weapon_number(w.mode9_a);
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
pub fn battle_folder(b: &[u8], regular_pending: bool, ids: &Ids) -> BattleFolder {
    BattleFolder {
        chips: std::array::from_fn(|i| {
            let v = u16::from_le_bytes([b[2 * i], b[2 * i + 1]]);
            (v != 0xFFFF).then(|| ids.folder_chip(v))
        }),
        regular_pending,
    }
}

/// A battle folder's encoding.
pub fn battle_folder_bytes(f: &BattleFolder, ids: &Ids) -> [u8; 2 * FOLDER_SIZE] {
    let mut b = [0u8; 2 * FOLDER_SIZE];
    for (i, c) in f.chips.iter().enumerate() {
        let v: u16 = c.map_or(0xFFFF, |c| ids.packed(c));
        b[2 * i..2 * i + 2].copy_from_slice(&v.to_le_bytes());
    }
    b
}

/// A chip hand from the game's 0x50-byte chip block (0xFFFF: no chip, in
/// the chips and the selection). Byte 1 is always 0 in battle (only the
/// battle flag 0x40 mode's unreferenced routines use it) and is not kept.
pub fn chip_hand(b: &[u8], ids: &Ids) -> ChipHand {
    let u16s = |off: usize| -> [u16; 6] { std::array::from_fn(|i| u16::from_le_bytes([b[off + 2 * i], b[off + 2 * i + 1]])) };
    ChipHand {
        cursor: b[0],
        ids: u16s(0x02).map(|v| ids.chip_field(v, 0xFFFF)),
        damage: u16s(0x0E),
        attack_bonus: u16s(0x1A),
        charge_bonus: u16s(0x26),
        selection: u16s(0x32).map(|v| (v != 0xFFFF).then(|| ids.folder_chip(v))),
        turn: b[0x3E..0x44].try_into().unwrap(),
        modifiers: b[0x44..0x4A].try_into().unwrap(),
    }
}

/// A chip hand's 0x50-byte chip block.
pub fn chip_hand_bytes(h: &ChipHand, ids: &Ids) -> [u8; 0x50] {
    let mut b = [0u8; 0x50];
    b[0] = h.cursor;
    let chips = h.ids.map(|c| ids.chip_field_id(c, 0xFFFF));
    let selection = h.selection.map(|c| c.map_or(0xFFFF, |c| ids.packed(c)));
    let mut put = |off: usize, v: &[u16; 6]| {
        for (i, x) in v.iter().enumerate() {
            b[off + 2 * i..off + 2 * i + 2].copy_from_slice(&x.to_le_bytes());
        }
    };
    put(0x02, &chips);
    put(0x0E, &h.damage);
    put(0x1A, &h.attack_bonus);
    put(0x26, &h.charge_bonus);
    put(0x32, &selection);
    b[0x3E..0x44].copy_from_slice(&h.turn);
    b[0x44..0x4A].copy_from_slice(&h.modifiers);
    b
}

/// A transformation request from the game's 0x10-byte record: +0 the
/// form, +4 the Cross change (0xFF = none for both). +1 and +3 are
/// custom-screen bookkeeping nothing in battle reads, and +8 names the
/// requesting navi object, which is always the side's player.
pub fn transform_request(b: &[u8], ids: &Ids) -> TransformRequest {
    let opt = |v: u8| (v != 0xFF).then_some(v);
    TransformRequest { form: opt(b[0]).map(|f| ids.form(f)), cross_change: opt(b[4]).map(|n| ids.navi(n)) }
}

// ---- Battle settings, stages, SP times ---------------------------------------------

/// Netbattle settings from the game's 16-byte BattleSettings record: the
/// stage whose record it is (the first whose layout, music, mode, battle
/// number, panel pattern and actor list match; bytes 12..16 name the actor
/// list by its original address, which `content` resolves), with the
/// record's background and effects. Byte 1 (read by
/// `GetBattleSettingsUnk01`, outside the battle simulation) and byte 7 (no
/// reader found) are not kept.
pub fn battle_settings(b: &[u8], ids: &Ids) -> BattleSettings {
    let content = ids.content;
    let address = u32::from_le_bytes(b[12..16].try_into().unwrap());
    let actors = content
        .rules
        .stages
        .actor_list_at(address)
        .unwrap_or_else(|| panic!("battle settings name an unknown actor list {address:#010x}"));
    let stage = content
        .defs
        .stages
        .iter()
        .position(|st| {
            let r = &st.record;
            (r.layout, r.music, r.mode, r.battle_number, r.panel_pattern, r.actors) == (b[0], b[2], b[3], b[5], b[6], actors)
        })
        .unwrap_or_else(|| panic!("no stage has the battle settings {b:02x?}"));
    BattleSettings {
        stage: StageHandle(stage as u16),
        background: b[4],
        effects: u32::from_le_bytes(b[8..12].try_into().unwrap()),
    }
}

/// The init exchange's two stage pairs (`byte_203CA50`: settings index,
/// then background, per round).
pub fn later_stages(b: &[u8], ids: &Ids) -> [Stage; 2] {
    [Stage { stage: ids.stage(b[0]), background: b[1] }, Stage { stage: ids.stage(b[2]), background: b[3] }]
}

/// A player's SP navi deletion times (`byte_203EB00`, 0x28 bytes).
pub fn sp_times(b: &[u8]) -> SpTimes {
    SpTimes(std::array::from_fn(|i| u16::from_le_bytes([b[2 * i], b[2 * i + 1]])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bn6_battle::content::testing;

    /// The left navi's stats at the start of the machgun replay.
    const MACHGUN_P0: &str = "08000000000100ff00320505010080000000ff00000000000000000101000001010301000000001f0000000a0000ffffff0000000000000000ff00000000e803e803e8030000010000000a0000000000000000000000ffffffffffff0000000000000000";

    fn bytes(hex: &str) -> [u8; 0x64] {
        let v: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        v.try_into().unwrap()
    }

    /// The test content with a navi and a form for every number (a stat
    /// byte can name any), defined.
    fn content() -> &'static Content {
        static CONTENT: std::sync::OnceLock<Content> = std::sync::OnceLock::new();
        CONTENT.get_or_init(|| {
            let mut c = testing::build();
            let (navi, form) = (c.navis[0].clone(), c.forms[0].clone());
            c.navis = (0..=0xFF).map(|id| bn6_battle::content::NaviData { id, ..navi.clone() }).collect();
            c.forms = (0..=0xFF).map(|id| bn6_battle::content::FormData { id, ..form.clone() }).collect();
            c.define().unwrap_or_else(|e| panic!("content error: {e}"));
            c
        })
    }

    fn ids() -> Ids<'static> {
        Ids::new(content(), Compat::bn6())
    }

    #[test]
    fn navi_stats_decode() {
        let ids = ids();
        let s = navi_stats(&bytes(MACHGUN_P0), &ids);
        assert_eq!((s.hp, s.max_hp, s.max_base_hp), (1000, 1000, 1000));
        assert_eq!(ids.navi_number(s.navi), Navi::MEGAMAN.0);
        assert_eq!(ids.form_number(s.form), Form::NONE.0);
        assert!(s.float_shoes && s.air_shoes && !s.undershirt && !s.super_armor);
        assert_eq!(s.mood, 0x80);
        assert_eq!(s.support, Some(Supports::default()));
        assert_eq!((s.weapons.buster, s.weapons.charge_shot, s.weapons.back_special), (ids.weapon(0), ids.weapon(1), None));
        assert_eq!(s.beast_out_counter, 3);
    }

    #[test]
    fn navi_stats_encode_round_trips() {
        let ids = ids();
        let s = navi_stats(&bytes(MACHGUN_P0), &ids);
        assert_eq!(navi_stats(&navi_stats_bytes(&s, &ids), &ids), s);
    }

    /// A bug code sets the stat byte it names: the engine's setter does
    /// what setting that byte of the game's block does, and refuses the
    /// bytes the engine doesn't model.
    #[test]
    fn bug_codes_set_the_byte_they_name() {
        let ids = ids();
        let base = navi_stats(&bytes(MACHGUN_P0), &ids);
        let mut problems = Vec::new();
        std::panic::set_hook(Box::new(|_| {}));
        for offset in 1..0x64u8 {
            let values: &[u8] = if offset == 0x08 { &[0, 1, 2] } else { &[0, 1, 2, 0x7F, 0xFF] };
            let decoded = |v: u8| {
                let mut raw = bytes(MACHGUN_P0);
                raw[offset as usize] = v;
                navi_stats(&raw, &ids)
            };
            let modeled = values.iter().any(|&v| decoded(v) != base);
            for &value in values {
                let result = std::panic::catch_unwind(move || {
                    let mut s = base;
                    s.set_byte_by_bug_code(offset, value, content());
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
        let ids = ids();
        let mut b = [0u8; 0x50];
        for (i, x) in b.iter_mut().enumerate() {
            *x = (i * 7) as u8;
        }
        b[1] = 0;
        b[0x4A..].fill(0);
        // Chips the content has (and empty entries), and picks with codes.
        let put = |b: &mut [u8; 0x50], off: usize, v: [u16; 6]| {
            for (i, x) in v.iter().enumerate() {
                b[off + 2 * i..off + 2 * i + 2].copy_from_slice(&x.to_le_bytes());
            }
        };
        put(&mut b, 0x02, [0x03, 0x41, 0x100, 0xFFFF, 0x00, 0x05]);
        put(&mut b, 0x32, [26 << 9 | 0x03, 0x41, 0xFFFF, 3 << 9 | 0x100, 0x0000, 0xFFFF]);
        let hand = chip_hand(&b, &ids);
        assert_eq!(hand.ids[3], None);
        assert_eq!(hand.selection[2], None);
        assert_eq!(chip_hand_bytes(&hand, &ids), b);
    }

    /// A number compat names a definition for reaches the definition; the
    /// pack's record otherwise, and back.
    #[test]
    fn numbers_reach_definitions_by_key() {
        let c = testing::with_test_pack();
        let mut compat = Compat::default();
        compat.chips.insert("test/ticker1".into(), crate::ChipEntry { id: 0x36, ..Default::default() });
        compat.weapons.insert("test/tick-shot".into(), vec![0x2E, 0x2F]);
        let ids = Ids::new(&c, &compat);
        let ticker = c.defs.chip_by_key("test/ticker1").unwrap();
        assert_eq!(ids.chip(0x36), ticker);
        assert_eq!(ids.chip_id(ticker), 0x36);
        assert_eq!(ids.chip(0x37), c.defs.chip_numbered(0x37).unwrap());
        assert_eq!(ids.chip_id(ids.chip(0x37)), 0x37);
        let shot = c.defs.weapon_by_key("test/tick-shot").unwrap();
        assert_eq!((ids.weapon(0x2E), ids.weapon(0x2F)), (Some(shot), Some(shot)));
        assert_eq!(ids.weapon_number(Some(shot)), 0x2E);
        assert_eq!(ids.weapon(0x01), c.defs.weapon_numbered(0x01));
        let mut raw = [0xFF; 2 * FOLDER_SIZE];
        raw[..2].copy_from_slice(&[0x36, 0x06]);
        let folder = battle_folder(&raw, false, &ids);
        assert_eq!(folder.chips[0], Some(FolderChip::new(ticker, ChipCode(3))));
    }
}
