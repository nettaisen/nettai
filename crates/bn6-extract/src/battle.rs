//! The battle data: everything the engine's `Content` holds, read from the
//! ROM into its typed form (bn6-content writes it as the pack's TOML
//! files). The ROM layouts are decoded here and nowhere else; each table's
//! address and record layout is noted where it is read.

use crate::{Rom, iwram, lz77, u32at};
use bn6_battle::actor::ActorType;
use bn6_battle::content::*;
use bn6_battle::field::PanelType;
use bn6_battle::setup::{ActorEntry, ActorKind, ActorList, ActorListId, BattleSettings};
use std::collections::BTreeMap;

/// The ROM's battle data.
pub fn content(rom: &Rom) -> Content {
    let actor_lists = actor_lists(rom);
    let mut chips = chips(rom);
    attach_gun_del_sol(rom, &mut chips);
    attach_sp_damage(rom, &mut chips);
    attach_program_advances(rom, &mut chips);
    attach_modifiers(&mut chips);
    Content {
        chips,
        navis: navis(rom),
        forms: forms(rom),
        rules: rules(rom, &actor_lists),
        objects: objects(rom),
        effects: effect_table(rom, 0x080E_0398, 0x6C),
        sparks: effect_table(rom, 0x080E_0804, 16),
        regions: regions(rom),
        panel_layouts: panel_layouts(rom),
        animations: animations(rom),
        // Scripts come from the source overlay (content.rs), not the ROM.
        weapons: Vec::new(),
        scripts: Default::default(),
    }
}

// ---- Chips -------------------------------------------------------------------------

/// ChipDataArr: 0x2C-byte records (docs/engine/chips.md §1.2).
const CHIP_TABLE: u32 = 0x0802_1DA8;
const CHIP_RECORD: u32 = 0x2C;
const CHIP_COUNT: u32 = 411;

fn chip_name(rom: &Rom, id: u32) -> String {
    const NAMES0: u32 = 0x086E_A94C;
    const NAMES1: u32 = 0x086E_B354;
    if id <= 0xFF { crate::archive_string(rom, NAMES0, id) } else { crate::archive_string(rom, NAMES1, id & 0xFF) }
}

fn element(v: u8) -> Element {
    [Element::Null, Element::Fire, Element::Aqua, Element::Elec, Element::Wood][v as usize]
}

fn chips(rom: &Rom) -> Vec<ChipData> {
    (0..CHIP_COUNT)
        .map(|id| {
            let r = CHIP_TABLE + id * CHIP_RECORD;
            let b = |o: u32| rom.u8(r + o);
            let flags = b(0x09);
            assert_eq!(flags & 0x20, 0, "chip {id:#x}: flag 0x20 (unused by every chip) is set");
            ChipData {
                id: id as ChipId,
                name: chip_name(rom, id),
                codes: rom.bytes(r, 4).iter().filter(|&&c| c != 0xFF).map(|&c| ChipCode(c)).collect(),
                element: element(b(0x04)),
                rarity: b(0x05),
                family: ChipFamily::from_number(b(0x06)).unwrap_or_else(|| panic!("chip {id:#x}: family {}", b(0x06))),
                class: [ChipClass::Standard, ChipClass::Mega, ChipClass::Giga, ChipClass::Special, ChipClass::ProgramAdvance]
                    [b(0x07) as usize],
                mb: b(0x08),
                flags: ChipFlags(flags),
                hit_param: b(0x0A),
                action: b(0x0B),
                subtype: b(0x0C),
                // +0x0D and +0x0E have no reader in the game; not extracted.
                beast_lockon: match b(0x0F) {
                    0 => false,
                    1 => true,
                    v => panic!("chip {id:#x}: beast lock-on byte {v}"),
                },
                params: rom.bytes(r + 0x10, 4).try_into().unwrap(),
                lockout: b(0x14),
                library_index: b(0x15),
                extra_flags: ExtraChipFlags(b(0x16)),
                lockon_mode: b(0x17),
                sort_key: rom.u16(r + 0x18),
                damage: rom.u16(r + 0x1A),
                library_number: rom.u16(r + 0x1C),
                slot_in_limit: b(0x1E),
                dark_substitute: (b(0x1F) != 0xFF).then_some(b(0x1F)),
                sp_damage: None,
                modifier: None,
                program_advances: Vec::new(),
                gun_del_sol: None,
                script: None,
            }
        })
        .collect()
}

/// Attachment kinds (`byte_80B8BD4`, attachment object #5): five-byte rows
/// of sprite category and index, palette, lift, attach point (0 = none).
fn attachment(rom: &Rom, id: u8) -> AttachmentKind {
    let b = rom.bytes(0x080B_8BD4 + 5 * id as u32, 5);
    AttachmentKind {
        id,
        sprite: SpriteId { category: b[0], index: b[1] },
        palette: b[2],
        lift: b[3] as i8,
        attach_point: (b[4] != 0).then_some(b[4]),
    }
}

const ATTACHMENTS: u8 = 52;

/// GunDelSol (action 0x37, `sub_80EDAE0`) by level, the chip's subtype:
/// its gun is attachment `7 + level`, it fires for `dword_80EDBC8[level]`
/// ticks, and its beam looks are `byte_80EDBB8[sun][level]`.
fn attach_gun_del_sol(rom: &Rom, chips: &mut [ChipData]) {
    for c in chips.iter_mut().filter(|c| c.action == 0x37) {
        let level = c.subtype as u32;
        assert!(level < 4, "GunDelSol chip {:#x} of level {level}", c.id);
        let look = |sun: u32| {
            let a = 0x080E_DBB8 + 2 * (level + 4 * sun);
            SunBeamLook { look: rom.u8(a), palette: rom.u8(a + 1) }
        };
        c.gun_del_sol = Some(GunDelSol {
            firing_ticks: rom.u8(0x080E_DBC8 + level) as u16,
            beam: look(0),
            beam_in_sun: look(1),
            gun: attachment(rom, 7 + level as u8),
        });
    }
}

/// SP navi chips: damage formula n + 1 (1..=18, `sub_8010AE4`) reads row
/// n of `byte_8020E54` (11 halfwords by deletion-time step).
fn attach_sp_damage(rom: &Rom, chips: &mut [ChipData]) {
    let mut rows_used = [false; 18];
    for c in chips.iter_mut() {
        let Some(n) = c.damage.checked_sub(1001).filter(|&n| n < 18) else { continue };
        rows_used[n as usize] = true;
        c.sp_damage = Some((0..11).map(|k| rom.u16(0x0802_0E54 + 0x16 * n as u32 + 2 * k)).collect());
    }
    assert!(rows_used.iter().all(|&u| u), "every SP damage row has its chip");
}

/// The Program Advances (`off_802BCB0`, null-terminated): pointers to
/// records of the chip count (u8), the matcher (u8: 0 a code run of one
/// chip, `sub_80295C8`; 4 a sequence, `sub_802961A`), the result chip
/// (u16) and the chips (u16 each; one for a code run), in the order tried.
fn attach_program_advances(rom: &Rom, chips: &mut [ChipData]) {
    for (order, pa) in program_advance_records(rom).into_iter().enumerate() {
        chips[pa.result as usize].program_advances.push(ProgramAdvanceRecipe { order: order as u8, recipe: pa.recipe });
    }
}

/// The Program Advance table's records, in order.
fn program_advance_records(rom: &Rom) -> Vec<ProgramAdvance> {
    const TABLE: u32 = 0x0802_BCB0;
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        let p = u32at(rom, TABLE + 4 * i);
        if !(0x0800_0000..0x0900_0000).contains(&p) {
            break;
        }
        let (count, result) = (rom.u8(p), rom.u16(p + 2));
        let recipe = match rom.u8(p + 1) {
            0 => PaRecipe::CodeRun { chip: rom.u16(p + 4), count },
            4 => PaRecipe::Sequence((0..count as u32).map(|k| rom.u16(p + 4 + 2 * k)).collect()),
            k => panic!("Program Advance record {p:#x}: matcher {k}"),
        };
        out.push(ProgramAdvance { result, recipe });
        i += 1;
    }
    out
}

/// The modifier chips `sub_8029224` folds into the chip before them.
fn attach_modifiers(chips: &mut [ChipData]) {
    for (id, m) in [
        (0xC0, ChipModifier::AttackPlus),
        (0xC1, ChipModifier::NaviPlus),
        (0xC3, ChipModifier::AttackPlus),
        (0xB8, ChipModifier::Paralyze),
        (0xB9, ChipModifier::Uninstall),
    ] {
        chips[id].modifier = Some(m);
    }
}

// ---- Navis and forms ------------------------------------------------------------------

/// The navis by navi number (NaviStats+0x29). The names are ours.
const NAVI_NAMES: [&str; 12] = [
    "MegaMan", "HeatMan", "ElecMan", "SlashMan", "EraseMan", "ChargeMan", "SpoutMan", "TomahawkMan", "TenguMan",
    "GroundMan", "DustMan", "Navi 11",
];

/// MegaMan's forms by form number (NaviStats+0x2C). The names are ours.
const FORM_NAMES: [&str; 25] = [
    "Base", "HeatCross", "ElecCross", "SlashCross", "EraseCross", "ChargeCross", "SpoutCross", "TomahawkCross",
    "TenguCross", "GroundCross", "DustCross", "Gregar Beast", "Falzar Beast", "HeatCross Beast", "ElecCross Beast",
    "SlashCross Beast", "EraseCross Beast", "ChargeCross Beast", "SpoutCross Beast", "TomahawkCross Beast",
    "TenguCross Beast", "GroundCross Beast", "DustCross Beast", "Gregar Beast Over", "Falzar Beast Over",
];

/// The first player NameID: MegaMan's (0x1A0); link navi n is 0x1A0 + n,
/// form f (1..=24) 0x1AB + f.
const FIRST_NAME: u16 = 0x1A0;

/// A player NameID's actor record (`byte_80182C4`, three bytes by NameID)
/// and its sprite attach points (`sub_8018810`: the actor's sprite, then
/// its 34 two-byte points).
fn name_record(rom: &Rom, name: u16) -> NameData {
    let rec = rom.bytes(0x0801_82C4 + 3 * name as u32, 3);
    let actor_type = [ActorType::Virus, ActorType::Navi, ActorType::Player][rec[1] as usize];
    let st = u32at(rom, u32at(rom, 0x0800_F230 + 4 * rec[1] as u32) + 4 * rec[2] as u32);
    let (cat, mut idx) = (rom.u8(st) as u32, rom.u8(st + 1) as u32);
    if cat == 0 {
        idx = rom.u8(0x0801_88B0 + idx) as u32;
    }
    let table = u32at(rom, 0x0801_88A0 + cat) + 0x44 * idx;
    let attach_points = (0..34)
        .map(|i| AttachPoint { x: rom.u8(table + 2 * i) as i8, y: rom.u8(table + 2 * i + 1) as i8 })
        .collect();
    NameData { id: name, version: rec[0], actor_type, ai_index: rec[2], attach_points }
}

fn secondary(v: u8) -> SecondaryElements {
    SecondaryElements(v)
}

/// Navis: sprites (`byte_800FCD5`, category 8), elements and weaknesses
/// (`byte_80108B8`, `byte_80108D1`, shared with the forms), buster bonus
/// (`byte_80126A4`), move lag (`byte_8020FE0`, 11 variants a row), the
/// netbattle banners (`byte_800A8EC`, `byte_800A8C8`), the height of a
/// Cross navi's image (`byte_80BC758`, 16.16 whole pixels) and the link
/// navis' own chips (`word_802A828`, `code << 9 | id` for navis 1..=11).
fn navis(rom: &Rom) -> Vec<NaviData> {
    (0..12u32)
        .map(|n| {
            let merge = u32at(rom, 0x080B_C758 + 4 * n) as i32;
            assert_eq!(merge & 0xFFFF, 0, "merge height {merge:#x} is not whole pixels");
            let own_chip = n.checked_sub(1).map(|i| {
                let packed = rom.u16(0x0802_A828 + 2 * i);
                CodedChip { chip: packed & 0x1FF, code: ChipCode((packed >> 9) as u8) }
            });
            NaviData {
                id: n as u8,
                name: NAVI_NAMES[n as usize].into(),
                sprite: SpriteId { category: 8, index: rom.u8(0x0800_FCD5 + n) },
                element: element(rom.u8(0x0801_08B8 + n)),
                weakness: secondary(rom.u8(0x0801_08D1 + n)),
                buster_bonus: rom.u8(0x0801_26A4 + n),
                move_lag: rom.bytes(0x0802_0FE0 + 11 * n, 11).to_vec(),
                win_banner: BannerId(rom.u8(0x0800_A8EC + n)),
                lose_banner: BannerId(rom.u8(0x0800_A8C8 + n)),
                merge_height: (merge >> 16) as i16,
                own_chip,
                name_record: Some(name_record(rom, FIRST_NAME + n as u16)),
            }
        })
        .collect()
}

/// MegaMan's forms: sprites (`byte_800FCBC`, category 0), elements and
/// weaknesses (shared with the navis), weapon routines (`byte_8020354`, six
/// bytes a form: mode 9 A, A charge, buster, charged shot, B+Back, the
/// alternative A charge) and buster bonus (`off_80126B0`).
fn forms(rom: &Rom) -> Vec<FormData> {
    (0..25u32)
        .map(|f| {
            let w = rom.bytes(0x0802_0354 + 6 * f, 6);
            FormData {
                id: f as u8,
                name: FORM_NAMES[f as usize].into(),
                sprite: SpriteId { category: 0, index: rom.u8(0x0800_FCBC + f) },
                element: element(rom.u8(0x0801_08B8 + f)),
                weakness: secondary(rom.u8(0x0801_08D1 + f)),
                weapons: FormWeapons {
                    mode9_a: w[0],
                    a_charge: w[1],
                    buster: w[2],
                    charge_shot: w[3],
                    back_special: w[4],
                    alt_a_charge: w[5],
                },
                buster_bonus: rom.u8(0x0801_26B0 + f),
                name_record: (f != 0).then(|| name_record(rom, FIRST_NAME + 11 + f as u16)),
            }
        })
        .collect()
}

// ---- Rules -------------------------------------------------------------------------------

fn slide(rom: &Rom, a: u32) -> SlideVector {
    SlideVector { dx: rom.u8(a) as i8, dy: rom.u8(a + 1) as i8, tiles: rom.u8(a + 2) }
}

fn condition(rom: &Rom, a: u32) -> PanelCondition {
    PanelCondition { require: u32at(rom, a), forbid: u32at(rom, a + 4) }
}

/// A step-rule table: [floor-free][alliance], 8 bytes a condition.
fn step_rules(rom: &Rom, a: u32) -> StepRuleSet {
    StepRuleSet {
        grounded: [condition(rom, a), condition(rom, a + 8)],
        floor_free: [condition(rom, a + 16), condition(rom, a + 24)],
    }
}

fn rules(rom: &Rom, actor_lists: &(Vec<u32>, Vec<ActorList>)) -> Rules {
    // The weakness table is 28 bytes the game indexes as receiver * 5 +
    // hitter (`byte_3007444`): hitter element 5 reads the next receiver's
    // first entry.
    let w = rom.bytes(iwram(0x0300_7444), 28);
    let element_weakness = std::array::from_fn(|r| std::array::from_fn(|h| w.get(r * 5 + h).copied().unwrap_or(0)));
    // Families past the last (13..16) have no chips.
    let family_elements = std::array::from_fn(|f| secondary(rom.u8(0x0801_29E4 + f as u32)));
    let collision_types = (0..89).map(|i| [u32at(rom, 0x0801_9C7C + 8 * i), u32at(rom, 0x0801_9C80 + 8 * i)]).collect();
    let field_regions = (0..9).map(|i| condition(rom, 0x0801_9C34 + 8 * i)).collect();
    // Panel types: flag bits by type (`word_3007924`); the roads carry a
    // navi by `byte_800E538` (four bytes: dx, dy, panels).
    let types = PanelType::ALL
        .iter()
        .map(|&t| PanelTypeRule {
            flags: u32at(rom, iwram(0x0300_7924) + 4 * t as u32),
            road_slide: t.is_road().then(|| slide(rom, 0x0800_E538 + 4 * (t as u32 - PanelType::RoadUp as u32))),
        })
        .collect();
    let grid = |a: u32| std::array::from_fn(|y| std::array::from_fn(|x| rom.u8(a + 8 * y as u32 + x as u32) != 0));
    let panels = PanelRules {
        types,
        start_visible: grid(0x0800_C590),
        front_edges: grid(0x0800_C5B8),
        step: step_rules(rom, 0x0800_E660),
        dash_step: step_rules(rom, 0x0801_0388),
        any_side_step: step_rules(rom, 0x0800_E6C8),
    };
    Rules {
        element_weakness,
        family_elements,
        collision_types,
        field_regions,
        panels,
        stages: stages(rom, actor_lists),
        holding_banners: holding_banners(rom),
        status_effects: status_effects(rom),
        hp_bug_periods: rom.bytes(0x0801_02A4, 8).try_into().unwrap(),
        // Charge thresholds by routine (`byte_8020404`, five halfwords a
        // routine).
        weapons: (0..50)
            .map(|r| WeaponRoutine { charge_ticks: std::array::from_fn(|c| rom.u16(0x0802_0404 + 10 * r + 2 * c as u32)) })
            .collect(),
        // By Rapid, then open panels ahead (`byte_80209CC`).
        buster_recovery: (0..5).map(|n| rom.bytes(0x0802_09CC + 6 * n, 6).try_into().unwrap()).collect(),
        // BCD times (`byte_8010B2C`).
        sp_deletion_times: (0..10).map(|i| u32at(rom, 0x0801_0B2C + 4 * i)).collect(),
        // By hit-modifier bit (`byte_800E58C`, three bytes each) and by
        // collision direction (`byte_800E4E8`, four bytes each).
        push_vectors: std::array::from_fn(|i| slide(rom, 0x0800_E58C + 3 * i as u32)),
        ice_vectors: std::array::from_fn(|i| slide(rom, 0x0800_E4E8 + 4 * i as u32)),
        bubble_bob: std::array::from_fn(|i| rom.u8(0x0801_7868 + i as u32) as i8),
        lockon: lockon(rom),
        custom_screen: custom_screen(rom),
    }
}

/// Banners that hold until removed: types 2 and 4 in `pt_801EF84`
/// records (byte 2), by banner id / 4.
fn holding_banners(rom: &Rom) -> Vec<BannerId> {
    let base = 0x0801_EF84;
    let mut out = Vec::new();
    for i in 0.. {
        let p = u32at(rom, base + 4 * i);
        if !(0x0800_0000..0x0900_0000).contains(&p) {
            break;
        }
        if matches!(rom.u8(p + 2), 2 | 4) {
            out.push(BannerId(4 * i as u8));
        }
    }
    out
}

/// Status effects (`off_80209EC`): per group a pointer to 8-byte entries
/// {u32 request bits, u16 duration, u8 collision-field offset}. Entries
/// past a group's end read the following bytes, as in the game.
fn status_effects(rom: &Rom) -> Vec<[StatusEffect; 16]> {
    let timer = |off: u8| match off {
        0x1C => StatusTimer::Paralyze,
        0x1E => StatusTimer::Confuse,
        0x20 => StatusTimer::Blind,
        0x22 => StatusTimer::Immobilize,
        0x24 => StatusTimer::Flash,
        0x26 => StatusTimer::Submerged,
        0x28 => StatusTimer::Invulnerable,
        0x2A => StatusTimer::Freeze,
        0x2C => StatusTimer::Bubble,
        0x0A => StatusTimer::CollisionPanel,
        o => StatusTimer::Other(o),
    };
    (0..6)
        .map(|g| {
            let p = u32at(rom, 0x0802_09EC + 4 * g);
            std::array::from_fn(|n| {
                let a = p + 8 * n as u32;
                StatusEffect { requests: u32at(rom, a), duration: rom.u16(a + 4), timer: timer(rom.u8(a + 6)) }
            })
        })
        .collect()
}

/// Beast Out lock-on searches (`ho_8026554`, `jt_8026584`): for the modes
/// that look for a panel near the target (`sub_80265D0`), the offsets
/// tried and whether the middle row is taken afterwards (`sub_80265FE`),
/// and the column shifts tried when nothing fits (`byte_8026735`).
fn lockon(rom: &Rom) -> Lockon {
    // A list of signed bytes, or byte pairs, up to 0x7F.
    let list = |mut a: u32, pairs: bool| {
        let mut v = Vec::new();
        while rom.u8(a) != 0x7F {
            v.push((rom.u8(a) as i8, if pairs { rom.u8(a + 1) as i8 } else { 0 }));
            a += if pairs { 2 } else { 1 };
        }
        v
    };
    let column_shifts = list(u32at(rom, 0x0802_67E8), false).into_iter().map(|(s, _)| s).collect();
    // (mode, literal-pool slot of its offset list, prefers the middle row)
    const MODES: [(u8, u32, bool); 12] = [
        (0x02, 0x0802_67FC, false),
        (0x03, 0x0802_6800, true),
        (0x04, 0x0802_6804, false),
        (0x05, 0x0802_6808, false),
        (0x06, 0x0802_680C, true),
        (0x07, 0x0802_6810, false),
        (0x08, 0x0802_6814, false),
        (0x09, 0x0802_6818, true),
        (0x0C, 0x0802_6824, false),
        (0x0D, 0x0802_6828, false),
        (0x0F, 0x0802_6830, false),
        (0x10, 0x0802_6834, true),
    ];
    let searches = MODES
        .iter()
        .map(|&(mode, pool, prefers_middle_row)| LockonSearch {
            mode,
            offsets: list(u32at(rom, pool), true).into_iter().map(|(dx, dy)| PanelOffset { dx, dy }).collect(),
            prefers_middle_row,
        })
        .collect();
    Lockon { searches, column_shifts }
}

/// The custom screen's slot grid (`dword_802A7CC`: four bytes a slot:
/// kind (0x0A a chip position, 0x01 OK, 0x0B hidden), vertical, left,
/// right) and the lists its neighbour fix-up scans (`sub_8027F42`: the
/// lists at `off_802A7FC`, `byte_802A804`, `byte_802A817`, `byte_802A81F`,
/// each slot's start in them at `byte_8027FD0` and `byte_8028028`).
fn custom_screen(rom: &Rom) -> CustomScreenLayout {
    let slots = std::array::from_fn(|s| {
        let b = rom.bytes(0x0802_A7CC + 4 * s as u32, 4);
        let kind = match b[0] {
            0x0A => TemplateSlot::ChipPosition,
            0x01 => TemplateSlot::Ok,
            0x0B => TemplateSlot::Hidden,
            k => panic!("custom screen slot {s}: kind {k}"),
        };
        SlotLayout { kind, vertical: b[1], left: b[2], right: b[3] }
    });
    let bytes = |a: u32, n: usize| rom.bytes(a, n).to_vec();
    CustomScreenLayout {
        slots,
        left_scan_top: bytes(0x0802_A7FC, 8),
        left_scan_bottom: bytes(0x0802_A804, 19),
        right_scan_top: bytes(0x0802_A817, 8),
        right_scan_bottom: bytes(0x0802_A81F, 7),
        left_scan_start: bytes(0x0802_7FD0, 12).try_into().unwrap(),
        right_scan_start: bytes(0x0802_8028, 12).try_into().unwrap(),
    }
}

// ---- Stages -----------------------------------------------------------------------------

/// `BattleSettingsList1`: 192 16-byte battle settings records; bytes
/// 12..16 point at the actor list.
const SETTINGS: u32 = 0x080B_0D88;
const SETTINGS_COUNT: u32 = 192;

/// Every actor list the battle settings refer to, by address (sorted), and
/// the lists. A list is 4-byte entries up to one whose byte 0 has high
/// nibble 0xF; the spawn loop (`sub_8007368`) reads kind = b0 >> 4,
/// side = b0 & 1, x = b1 & 7, y = b1 >> 4, and kind-specific bytes b2, b3.
fn actor_lists(rom: &Rom) -> (Vec<u32>, Vec<ActorList>) {
    let mut sources: Vec<u32> = (0..SETTINGS_COUNT).map(|i| u32at(rom, SETTINGS + 16 * i + 12)).collect();
    sources.sort_unstable();
    sources.dedup();
    let lists = sources
        .iter()
        .map(|&source| {
            let mut entries = Vec::new();
            let mut a = source;
            loop {
                let b = rom.bytes(a, 4);
                if b[0] >> 4 == 0xF {
                    break;
                }
                // Each kind's spawn routine reads only some of the bytes;
                // the rest must be clear so the entry says it all.
                let unused = |mask0: u8, used2: bool| {
                    assert!(
                        b[0] & mask0 == 0 && b[1] & 0x08 == 0 && (used2 || b[2] == 0) && b[3] == 0,
                        "actor entry {a:#010x} sets bytes its spawn routine ignores: {b:02x?}"
                    )
                };
                let kind = match b[0] >> 4 {
                    0 => {
                        unused(0x0E, false);
                        ActorKind::Navi
                    }
                    3 => {
                        unused(0x0F, false);
                        ActorKind::Object6E
                    }
                    8 => {
                        unused(0x0F, true);
                        ActorKind::Rock { variant: b[2] }
                    }
                    9 => {
                        unused(0x0F, true);
                        ActorKind::Object7D { variant: b[2] }
                    }
                    k => panic!("actor entry {a:#010x}: kind {k} has no ActorKind yet"),
                };
                entries.push(ActorEntry { kind, alliance: b[0] & 1, x: b[1] & 7, y: b[1] >> 4 });
                a += 4;
            }
            ActorList { original_address: source, entries }
        })
        .collect();
    (sources, lists)
}

/// Stages: the battle settings (bytes 0 layout, 2 music, 3 mode, 4
/// background, 5 battle number, 6 panel pattern, 8..12 effects; byte 1 is
/// read outside the battle and byte 7 has no reader) and the actor lists.
fn stages(rom: &Rom, (sources, lists): &(Vec<u32>, Vec<ActorList>)) -> Stages {
    let settings = (0..SETTINGS_COUNT)
        .map(|i| {
            let r = rom.bytes(SETTINGS + 16 * i, 16);
            let source = u32at(rom, SETTINGS + 16 * i + 12);
            BattleSettings {
                layout: r[0],
                music: r[2],
                mode: r[3],
                background: r[4],
                battle_number: r[5],
                panel_pattern: r[6],
                effects: u32at(rom, SETTINGS + 16 * i + 8),
                actors: ActorListId(sources.binary_search(&source).unwrap() as u8),
            }
        })
        .collect();
    Stages { settings, actor_lists: lists.clone() }
}

/// Panel layouts (`word_800D730`): three u32 rows (y 1..3), a nibble per
/// column (x 1..6), up to `0x0800E24C`.
fn panel_layouts(rom: &Rom) -> Vec<PanelLayout> {
    let n = (0x0800_E24C - 0x0800_D730) / 12;
    (0..n)
        .map(|i| PanelLayout {
            rows: std::array::from_fn(|y| {
                let w = u32at(rom, 0x0800_D730 + 12 * i + 4 * y as u32);
                std::array::from_fn(|x| PanelType::ALL[((w >> (4 * x)) & 0xF) as usize])
            }),
        })
        .collect()
}

/// Region shapes (`PanelOffsetListsPointerTable`): signed byte pairs up to
/// dx 0x7F; a null pointer is no panels.
fn regions(rom: &Rom) -> Vec<Vec<PanelOffset>> {
    (0..47)
        .map(|i| {
            let mut a = u32at(rom, 0x0801_9B78 + 4 * i);
            let mut v = Vec::new();
            if a >= 0x0800_0000 {
                while rom.u8(a) != 0x7F {
                    v.push(PanelOffset { dx: rom.u8(a) as i8, dy: rom.u8(a + 1) as i8 });
                    a += 2;
                }
            }
            v
        })
        .collect()
}

/// Effect rows: sprite category, index, animation, palette.
fn effect_table(rom: &Rom, base: u32, n: u32) -> Vec<EffectSprite> {
    (0..n)
        .map(|i| {
            let b = rom.bytes(base + 4 * i, 4);
            EffectSprite { sprite: SpriteId { category: b[0], index: b[1] }, anim: b[2], palette: b[3] }
        })
        .collect()
}

// ---- Objects ------------------------------------------------------------------------------

/// Object data: attachments (`byte_80B8BD4`), rocks (`byte_80CF934`),
/// absorbed obstacles' sprites (`byte_80E98C0`), body overlays
/// (`byte_80C4320` sprites and `off_80C42D4` depth tables) and the sun
/// beam's sprites (`dword_80E5C28`), shock waves (`byte_80C6B00`).
fn objects(rom: &Rom) -> ObjectData {
    // Rock rows: standing animation, (unused), HP / 2, debris palette,
    // break sound (u16), name id (u16). The rock's init (`sub_80CF974`)
    // turns HP / 2 = 0 into 1 HP and makes variants 3 and up aqua.
    let rocks = (0..4u32)
        .map(|i| {
            let r = 0x080C_F934 + 8 * i;
            let hp = match rom.u8(r + 2) as u16 * 2 {
                0 => 1,
                hp => hp,
            };
            RockKind {
                id: i as u8,
                anim: rom.u8(r),
                hp,
                element: if i >= 3 { Element::Aqua } else { Element::Null },
                debris_palette: rom.u8(r + 3),
                break_sound: rom.u16(r + 4),
                name_id: rom.u16(r + 6),
            }
        })
        .collect();
    let sprite_at = |a: u32| SpriteId { category: rom.u8(a), index: rom.u8(a + 1) };
    // The depth tables sit back to back and end where the pointer table
    // starts; an animation past a table's end reads the next one, so each
    // runs to the end of the block.
    const OVERLAY_DEPTHS: u32 = 0x080C_42D4;
    let body_overlays = (0..19u32)
        .map(|i| {
            let depths = u32at(rom, OVERLAY_DEPTHS + 4 * i);
            assert!(depths < OVERLAY_DEPTHS, "overlay depth table {depths:#x} outside the block");
            let in_front = rom
                .bytes(depths, (OVERLAY_DEPTHS - depths) as usize)
                .iter()
                .map(|&b| match b {
                    0 => false,
                    1 => true,
                    v => panic!("overlay depth byte {v:#x}"),
                })
                .collect();
            BodyOverlay { id: i as u8, sprite: sprite_at(0x080C_4320 + 2 * i), in_front }
        })
        .collect();
    ObjectData {
        attachments: (0..ATTACHMENTS).map(|i| attachment(rom, i)).collect(),
        rocks,
        absorbed_sprites: (0..15).map(|i| sprite_at(0x080E_98C0 + 2 * i)).collect(),
        body_overlays,
        sun_beam_looks: (0..2).map(|i| sprite_at(0x080E_5C28 + 2 * i)).collect(),
        kinds: Vec::new(),
        shock_waves: (0..16).map(|i| shock_wave(rom, i)).collect(),
    }
}

/// A shock wave's row (`byte_80C6B00`, by the wave's first parameter):
/// the sprite's index in category 0x10, the animation, the ticks, and the
/// panel type it leaves (0xFF none).
fn shock_wave(rom: &Rom, id: u8) -> ShockWave {
    let b = rom.bytes(0x080C_6B00 + 4 * id as u32, 4);
    let panel = match b[3] {
        0xFF => None,
        t => Some(*PanelType::ALL.get(t as usize).unwrap_or_else(|| panic!("shock wave {id}: panel type {t:#x}"))),
    };
    ShockWave { id, sprite: SpriteId { category: 0x10, index: b[0] }, anim: b[1], ticks: b[2], panel }
}

// ---- Sprite timing ---------------------------------------------------------------------

/// Animation timing of every battle sprite (`SpritePointersList`,
/// categories 0x00..=0x14): per animation, its frames' (duration, flags),
/// 0x14-byte frames with the duration at +0x10 and flags at +0x12. The
/// same data the graphics export writes to `animations.json`; the
/// extractor checks they agree.
fn animations(rom: &Rom) -> Animations {
    const LIST: u32 = 0x0803_1CC4;
    const BATTLE_CATEGORIES: usize = 6;
    let cats: Vec<u32> = (0..10).map(|i| u32at(rom, LIST + 4 * i)).collect();
    let mut sprites = BTreeMap::new();
    for (ci, &c) in cats.iter().enumerate().take(BATTLE_CATEGORIES) {
        let next = cats.iter().copied().filter(|&s| s > c).min().unwrap_or(c + 0x400);
        for idx in 0..((next - c) / 4).min(256) {
            let p = u32at(rom, c + 4 * idx);
            let data: Vec<u8> = if p & 0x8000_0000 != 0 {
                match lz77(rom, p & 0x7FFF_FFFF) {
                    Some(d) if d.len() > 4 => d[4..].to_vec(),
                    _ => continue,
                }
            } else if (0x0800_0000..0x0900_0000).contains(&p) {
                let o = (p & 0x01FF_FFFF) as usize;
                rom.0[o..(o + 0x80000).min(rom.0.len())].to_vec()
            } else {
                continue;
            };
            let rd32 = |o: usize| data.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
            let Some(first) = rd32(4) else { continue };
            let count = first / 4;
            if count == 0 || count > 256 {
                continue;
            }
            let mut anims = Vec::new();
            let mut ok = true;
            for a in 0..count as usize {
                let Some(off) = rd32(4 + 4 * a) else {
                    ok = false;
                    break;
                };
                let mut f = 4 + off as usize;
                let mut frames = Vec::new();
                loop {
                    let (Some(&duration), Some(&flags)) = (data.get(f + 0x10), data.get(f + 0x12)) else {
                        ok = false;
                        break;
                    };
                    frames.push(AnimFrame { duration, flags });
                    if flags & 0x80 != 0 || frames.len() > 512 {
                        break;
                    }
                    f += 0x14;
                }
                if !ok {
                    break;
                }
                anims.push(frames);
            }
            if ok {
                sprites.insert(SpriteId { category: (ci * 4) as u8, index: idx as u8 }, anims);
            }
        }
    }
    Animations { sprites }
}
