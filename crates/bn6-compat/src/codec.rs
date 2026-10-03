//! The game's setup records, decoded into the engine's types and encoded
//! back: the one place that knows their layout (what traces, real saves
//! and link data carry). Only the bytes the engine uses are modeled; each
//! decoder notes the bytes it leaves out.
//!
//! The engine's state holds handles; the records hold the original's
//! numbers (chip ids, navi and form numbers, weapon routines, battle
//! settings indices). [`Ids`] maps one to the other through compat's keys.

use crate::{Compat, Game};
use nettai_battle::content::{ChipCode, Content};
use nettai_battle::custom::folder::FOLDER_SIZE;
use nettai_battle::custom::{BattleFolder, FolderChip};
use nettai_battle::hand::ChipHand;
use nettai_battle::navicust::{NaviCust, PlacedProgram};
use nettai_battle::patch_cards::{InstalledCard, PatchCards};
use nettai_battle::setup::{
    BattleSettings, GaugeSpeed, NaviCustBugs, NaviStats, NaviWeapons, SpTimes, Stage, Supports,
};
use nettai_battle::transform::TransformRequest;
use nettai_content_api::{ChipHandle, FormHandle, NaviCustProgramHandle, NaviHandle, PatchCardHandle, RecordHandle, StageHandle, WeaponHandle};

// ---- Numbers and handles ----------------------------------------------------------

/// A chip id (0..=0x19A in BN6): its place in the original's chip table,
/// which folders, hands, saves and link data carry.
pub type ChipId = u16;

/// The original's numbers for a content's identities, and back. A number
/// names compat's key, and a definition of that key is the thing (a chip
/// content defines, reached from a recorded folder; a navi, a form, a
/// weapon). Back, a definition gives compat's number for its key. The
/// engine has no number for any of them.
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

    /// The pack BN6's numbers are of: compat's game's pack in the
    /// content (docs/design/rules-in-luau.md §7.4: the engine knows assets
    /// by handle; the original's numbers are its pack's).
    pub fn pack(&self) -> nettai_content_api::PackId {
        let root = &self.compat.root;
        self.content.assets.pack(root).unwrap_or_else(|| panic!("the content loads no pack of {root}"))
    }

    /// The original's number for asset `h` of `kind` (BN6's pack's).
    pub fn asset_number(&self, kind: nettai_content_api::AssetKind, h: u16) -> u16 {
        let a = self.content.assets.number(kind, h).unwrap_or_else(|| panic!("no {kind} has handle {h}"));
        assert_eq!(a.pack, self.pack(), "{kind} {h} is no asset of BN6's pack");
        a.id
    }

    /// The handle of BN6's `kind` asset numbered `n`.
    pub fn asset(&self, kind: nettai_content_api::AssetKind, n: u16) -> u16 {
        self.content.assets.number_handle(kind, self.pack(), n).unwrap_or_else(|| panic!("the pack has no {kind} {n:#x}"))
    }

    /// The background numbered `n` (a settings record's byte).
    pub fn background(&self, n: u8) -> nettai_battle::content::BackgroundId {
        nettai_battle::content::BackgroundId(self.asset(nettai_content_api::AssetKind::Background, n as u16))
    }

    /// A background's number.
    pub fn background_number(&self, b: nettai_battle::content::BackgroundId) -> u8 {
        self.asset_number(nettai_content_api::AssetKind::Background, b.0) as u8
    }

    /// A sound's song-table number.
    pub fn sound_number(&self, s: nettai_battle::sound::SoundId) -> u16 {
        self.asset_number(nettai_content_api::AssetKind::Sound, s.0)
    }

    /// A definition's key as compat writes it; a definition of another
    /// root has no BN6 number.
    fn key<'k>(&self, key: &'k str) -> &'k str {
        self.compat.compat_key(self.content, key).unwrap_or_else(|| panic!("{key} is no definition of BN6's content: it has no BN6 number"))
    }

    /// The chip with this id.
    pub fn chip(&self, id: ChipId) -> ChipHandle {
        let key = self.compat.chip_key(id).unwrap_or_else(|| panic!("chips.toml has no chip {id:#x}"));
        self.content.defs.chip_by_key(key).unwrap_or_else(|| panic!("the content defines no chip {key:?} (chip {id:#x})"))
    }

    /// A chip's id.
    pub fn chip_id(&self, h: ChipHandle) -> ChipId {
        let key = self.key(&self.content.defs.chip(h).key);
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
        let key = self.compat.navi_key(navi).unwrap_or_else(|| panic!("navis.toml has no navi {navi:#x}"));
        self.content.defs.navi_by_key(key).unwrap_or_else(|| panic!("the content defines no navi {key:?} (navi {navi:#x})"))
    }

    /// A navi's number.
    pub fn navi_number(&self, h: NaviHandle) -> u8 {
        let key = self.key(&self.content.defs.navi(h).key);
        self.compat.navis.get(key).map(|n| n.navi).unwrap_or_else(|| panic!("navis.toml has no {key:?}"))
    }

    /// MegaMan's form with this number.
    pub fn form(&self, form: u8) -> FormHandle {
        let key = self.compat.form_key(form).unwrap_or_else(|| panic!("forms.toml has no form {form:#x}"));
        self.content.defs.form_by_key(key).unwrap_or_else(|| panic!("the content defines no form {key:?} (form {form:#x})"))
    }

    /// A form's number.
    pub fn form_number(&self, h: FormHandle) -> u8 {
        let key = self.key(&self.content.defs.form(h).key);
        self.compat.forms.get(key).map(|f| f.form).unwrap_or_else(|| panic!("forms.toml has no {key:?}"))
    }

    /// The weapon a routine number names; none for 0xFF.
    pub fn weapon(&self, routine: u8) -> Option<WeaponHandle> {
        if routine == 0xFF {
            return None;
        }
        let key = self
            .compat
            .weapon_key(routine)
            .unwrap_or_else(|| panic!("weapons.toml has no weapon routine {routine:#x}"));
        let h = self
            .content
            .defs
            .weapon_by_key(key)
            .unwrap_or_else(|| panic!("the content has no weapon {key:?} (weapon routine {routine:#x})"));
        Some(h)
    }

    /// A weapon's routine number (the first compat gives it); 0xFF for
    /// none.
    pub fn weapon_number(&self, w: Option<WeaponHandle>) -> u8 {
        let Some(w) = w else { return 0xFF };
        let key = self.key(&self.content.defs.weapon(w).key);
        let numbers = self.compat.weapons.get(key).unwrap_or_else(|| panic!("weapons.toml has no {key:?}"));
        *numbers.first().unwrap_or_else(|| panic!("weapons.toml gives {key:?} no number"))
    }

    /// The projectile variant a shot program's byte names (a row of
    /// `off_80C4C78`); none for 0, no program.
    pub fn shot_program(&self, row: u8) -> Option<RecordHandle> {
        if row == 0 {
            return None;
        }
        let key = self
            .compat
            .records
            .projectile_variants
            .iter()
            .find(|(_, n)| **n == row)
            .map(|(k, _)| k.as_str())
            .unwrap_or_else(|| panic!("records.toml has no projectile variant {row:#x}"));
        let h = self
            .content
            .defs
            .record(key)
            .unwrap_or_else(|| panic!("the content has no projectile variant {key:?} (row {row:#x})"));
        Some(h)
    }

    /// The patch card a save's card list names by its number (compat
    /// patch-cards.toml).
    pub fn patch_card(&self, number: u8) -> PatchCardHandle {
        let key = self
            .compat
            .patch_cards
            .iter()
            .find(|(_, n)| **n == number)
            .map(|(k, _)| k.as_str())
            .unwrap_or_else(|| panic!("patch-cards.toml has no patch card {number}"));
        self.content.defs.patch_card_by_key(key).unwrap_or_else(|| panic!("the content has no patch card {key:?} (number {number})"))
    }

    /// The NaviCust program a part id names (its number, `id >> 2`) and its
    /// color (the variant, `id & 3`, the definition's color in that
    /// place); none for 0, no part.
    pub fn navicust_part(&self, id: u8) -> Option<(NaviCustProgramHandle, u8)> {
        if id == 0 {
            return None;
        }
        let number = id >> 2;
        let key = self
            .compat
            .navicust
            .programs
            .iter()
            .find(|(_, n)| **n == number)
            .map(|(k, _)| k.as_str())
            .unwrap_or_else(|| panic!("navicust.toml has no program {number} (part id {id:#x})"));
        let h = self.content.defs.navicust_program_by_key(key).unwrap_or_else(|| panic!("the content has no NaviCust program {key:?}"));
        Some((h, id & 3))
    }

    /// A placed program's part id.
    pub fn navicust_part_id(&self, program: NaviCustProgramHandle, color: u8) -> u8 {
        let key = self.key(&self.content.defs.navicust_program(program).key);
        let n = self.compat.navicust.programs.get(key).unwrap_or_else(|| panic!("navicust.toml has no {key:?}"));
        n * 4 + color
    }

    /// A patch card's number.
    pub fn patch_card_number(&self, h: PatchCardHandle) -> u8 {
        let key = self.key(&self.content.defs.patch_card(h).key);
        *self.compat.patch_cards.get(key).unwrap_or_else(|| panic!("patch-cards.toml has no patch card {key:?}"))
    }

    /// The barrier a first-barrier byte names (NaviStats+0x06, the
    /// barrier type `sub_801A7CC` takes); none for 0.
    pub fn barrier(&self, ty: u8) -> Option<RecordHandle> {
        if ty == 0 {
            return None;
        }
        let key = self
            .compat
            .records
            .barriers
            .iter()
            .find(|(_, n)| **n == ty)
            .map(|(k, _)| k.as_str())
            .unwrap_or_else(|| panic!("records.toml has no barrier type {ty:#x}"));
        Some(self.content.defs.record(key).unwrap_or_else(|| panic!("the content has no barrier {key:?} (type {ty:#x})")))
    }

    /// A first-barrier byte; 0 for none.
    pub fn barrier_type(&self, r: Option<RecordHandle>) -> u8 {
        let Some(r) = r else { return 0 };
        let key = self.key(&self.content.defs.records[r.index()].key);
        *self.compat.records.barriers.get(key).unwrap_or_else(|| panic!("records.toml has no barrier {key:?}"))
    }

    /// A shot program's byte; 0 for none.
    pub fn shot_program_number(&self, r: Option<RecordHandle>) -> u8 {
        let Some(r) = r else { return 0 };
        let key = self.key(&self.content.defs.records[r.index()].key);
        *self
            .compat
            .records
            .projectile_variants
            .get(key)
            .unwrap_or_else(|| panic!("records.toml has no projectile variant {key:?}"))
    }

    /// The stage a battle settings index names.
    pub fn stage(&self, settings: u8) -> StageHandle {
        let key = self.compat.stage_key(settings).unwrap_or_else(|| panic!("stages.toml has no battle settings {settings:#x}"));
        self.content
            .defs
            .stage_by_key(key)
            .unwrap_or_else(|| panic!("the content has no stage {key:?} (battle settings {settings:#x})"))
    }

    /// A stage's battle settings index.
    pub fn stage_index(&self, h: StageHandle) -> u8 {
        let key = self.key(&self.content.defs.stage(h).key);
        let e = self.compat.stages.get(key).unwrap_or_else(|| panic!("stages.toml has no {key:?}"));
        *e.settings.first().unwrap_or_else(|| panic!("stages.toml gives {key:?} no settings"))
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
        first_barrier: ids.barrier(b[0x06]),
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
        chip_drops: b[0x26],
        encounters: b[0x28],
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
            buster_shot: ids.shot_program(b[0x4D]),
            charge_shot_kind: ids.shot_program(b[0x4F]),
            back_special_damage: u16at(0x48),
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
    b[0x06] = ids.barrier_type(s.first_barrier);
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
    b[0x26] = s.chip_drops;
    b[0x28] = s.encounters;
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
    b[0x4D] = ids.shot_program_number(w.buster_shot);
    b[0x4F] = ids.shot_program_number(w.charge_shot_kind);
    put16(&mut b, 0x48, w.back_special_damage);
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

// ---- Patch cards -------------------------------------------------------------------

/// A Japanese save's card list (its bytes: the number, bit 7 when switched
/// off) as a player's installed patch cards.
pub fn patch_cards(list: &[u8], ids: &Ids) -> Result<PatchCards, String> {
    let cards: Vec<InstalledCard> = list.iter().map(|&b| InstalledCard { card: ids.patch_card(b & 0x7F), enabled: b & 0x80 == 0 }).collect();
    PatchCards::new(&cards)
}

// ---- The NaviCust -------------------------------------------------------------------

/// A save's NaviCust (BN6: the list at 0x02004190, 0x31 parts of 8 bytes:
/// +0 the part id, +3 the center's column, +4 its row, +5 the quarter turns
/// clockwise), on a board with `expansions` (key item 0x71's count); a part
/// is compressed when `compressed` says so of its part id (event flag 0x2660
/// + the id, which `sub_813B7A0` reads). The list's empty entries (id 0)
/// are left out, the others kept in order.
pub fn navicust(list: &[u8], expansions: u8, compressed: impl Fn(u8) -> bool, ids: &Ids) -> Result<NaviCust, String> {
    let mut parts = Vec::new();
    for e in list.chunks_exact(8) {
        let Some((program, color)) = ids.navicust_part(e[0]) else { continue };
        parts.push(PlacedProgram { program, color, x: e[3], y: e[4], rotation: e[5], compressed: compressed(e[0]) });
    }
    NaviCust::new(&parts, expansions)
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

/// The music byte of battle settings that start no music (`PlayMusic` of
/// it stops the music).
const NO_MUSIC: u8 = 0x63;

/// Netbattle settings from the Falzar game's 16-byte BattleSettings record
/// ([`battle_settings_of`] for another game's).
pub fn battle_settings(b: &[u8], ids: &Ids) -> BattleSettings {
    battle_settings_of(Game::Falzar, b, ids)
}

/// Netbattle settings from `game`'s 16-byte BattleSettings record: the
/// stage whose record it is, with the record's background and effects.
/// Byte 0 names the panel layout by number and bytes 12..16 the actor list
/// by its address in `game`'s ROM (compat's stages have Falzar's;
/// games.toml says where Gregar's are); the stage is the first, by key,
/// with that layout and actor list whose music, mode, battle number and
/// panel pattern match. Byte 1 (read by `GetBattleSettingsUnk01`, outside
/// the battle simulation) and byte 7 (no reader found) are not kept.
pub fn battle_settings_of(game: Game, b: &[u8], ids: &Ids) -> BattleSettings {
    let content = ids.content;
    let address = u32::from_le_bytes(b[12..16].try_into().unwrap());
    let games = &ids.compat.games;
    let stage = ids
        .compat
        .stages
        .iter()
        .filter(|(_, e)| e.layout == b[0] && games.actor_list(game, e.actor_list) == address)
        .find_map(|(key, _)| {
            let h = content.defs.stage_by_key(key)?;
            let r = content.stage(h);
            let music = r.music.map_or(NO_MUSIC as u16, |m| ids.sound_number(m));
            ((music, r.mode, r.battle_number, r.panel_pattern) == (b[2] as u16, b[3], b[5], b[6])).then_some(h)
        })
        .unwrap_or_else(|| panic!("no stage has the battle settings {b:02x?}"));
    BattleSettings {
        stage,
        background: ids.background(b[4]),
        effects: u32::from_le_bytes(b[8..12].try_into().unwrap()),
    }
}

/// The init exchange's two stage pairs (`byte_203CA50`: settings index,
/// then background, per round).
pub fn later_stages(b: &[u8], ids: &Ids) -> [Stage; 2] {
    [Stage { stage: ids.stage(b[0]), background: ids.background(b[1]) }, Stage { stage: ids.stage(b[2]), background: ids.background(b[3]) }]
}

/// A player's SP navi deletion times (`byte_203EB00`, 0x28 bytes).
pub fn sp_times(b: &[u8]) -> SpTimes {
    SpTimes(std::array::from_fn(|i| u16::from_le_bytes([b[2 * i], b[2 * i + 1]])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_battle::content::testing;

    /// The left navi's stats at the start of the machgun replay.
    const MACHGUN_P0: &str = "08000000000100ff00320505010080000000ff00000000000000000101000001010301000000001f0000000a0000ffffff0000000000000000ff00000000e803e803e8030000010000000a0000000000000000000000ffffffffffff0000000000000000";

    fn bytes(hex: &str) -> [u8; 0x64] {
        let v: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        v.try_into().unwrap()
    }

    /// The test content.
    fn content() -> &'static Content {
        static CONTENT: std::sync::OnceLock<Content> = std::sync::OnceLock::new();
        CONTENT.get_or_init(testing::build)
    }

    /// BN6's compat, with the test content's chips under the numbers
    /// these tests' records use.
    fn ids() -> Ids<'static> {
        static COMPAT: std::sync::OnceLock<Compat> = std::sync::OnceLock::new();
        let compat = COMPAT.get_or_init(|| {
            let mut compat = Compat::bn6_for(content()).clone();
            compat.chips = [
                (0x00, "test:test/blank"),
                (0x03, testing::SUN_GUN_3),
                (0x05, testing::VEIL),
                (0x41, testing::PLUS),
                (0x100, testing::BOMB),
            ]
            .into_iter()
            .map(|(id, key)| (key.to_string(), crate::ChipEntry { id, ..Default::default() }))
            .collect();
            compat
        });
        Ids::new(content(), compat)
    }

    #[test]
    fn navi_stats_decode() {
        let ids = ids();
        let s = navi_stats(&bytes(MACHGUN_P0), &ids);
        assert_eq!((s.hp, s.max_hp, s.max_base_hp), (1000, 1000, 1000));
        assert_eq!(nettai_content_api::keys::local(&ids.content.defs.navi(s.navi).key), "megaman");
        assert_eq!(s.form, ids.content.base_form());
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
            // A byte that names content by the original's number (a navi,
            // a form, a weapon, a shot program) decodes only where the
            // content has it.
            let decoded = |v: u8| {
                std::panic::catch_unwind(|| {
                    let mut raw = bytes(MACHGUN_P0);
                    raw[offset as usize] = v;
                    navi_stats(&raw, &Ids::new(content(), Compat::bn6_for(content())))
                })
                .ok()
            };
            // (A value that doesn't decode is read as naming content: the
            // byte is modeled.)
            let modeled = values.iter().any(|&v| decoded(v).is_none_or(|d| d != base));
            // The engine has no numbers for weapons, shot programs, first
            // barriers, forms or navis: a bug code can only clear those
            // bytes (a form's to the base form), and can't write a navi's.
            let by_handle = |v: u8| match offset {
                0x04 | 0x05 | 0x07 | 0x39 | 0x44 => v != 0xFF,
                0x06 | 0x4D | 0x4F | 0x17 | 0x2C => v != 0,
                0x29 => true,
                _ => false,
            };
            for &value in values {
                let result = std::panic::catch_unwind(move || {
                    let mut s = base;
                    s.set_byte_by_bug_code(offset, value, content());
                    s
                });
                match (result, decoded(value)) {
                    (Ok(s), Some(d)) if modeled && s != d => problems.push(format!("{offset:#x} = {value:#x}: {s:?}")),
                    (Ok(_), _) if !modeled => problems.push(format!("{offset:#x} isn't modeled but is accepted")),
                    (Ok(_), _) if by_handle(value) => problems.push(format!("{offset:#x} = {value:#x} names content by number but is accepted")),
                    (Err(_), Some(_)) if modeled && !by_handle(value) => {
                        problems.push(format!("{offset:#x} = {value:#x} is modeled but refused"))
                    }
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

    /// A number compat names a definition for reaches the definition, and
    /// back.
    #[test]
    fn numbers_reach_definitions_by_key() {
        let c = testing::with_test_pack();
        let mut compat = Compat { root: "test".into(), ..Compat::default() };
        compat.chips.insert("test:test/ticker1".into(), crate::ChipEntry { id: 0x36, ..Default::default() });
        compat.weapons.insert("test:test/tick-shot".into(), vec![0x2E, 0x2F]);
        let ids = Ids::new(&c, &compat);
        let ticker = c.defs.chip_by_key("test:test/ticker1").unwrap();
        assert_eq!(ids.chip(0x36), ticker);
        assert_eq!(ids.chip_id(ticker), 0x36);
        let shot = c.defs.weapon_by_key("test:test/tick-shot").unwrap();
        assert_eq!((ids.weapon(0x2E), ids.weapon(0x2F)), (Some(shot), Some(shot)));
        assert_eq!(ids.weapon_number(Some(shot)), 0x2E);
        let mut raw = [0xFF; 2 * FOLDER_SIZE];
        raw[..2].copy_from_slice(&[0x36, 0x06]);
        let folder = battle_folder(&raw, false, &ids);
        assert_eq!(folder.chips[0], Some(FolderChip::new(ticker, ChipCode(3))));
    }
}
