//! The battle data: everything the engine's `Content` holds, read from the
//! ROM into its typed form (bn6-content writes it as the pack's TOML
//! files). The ROM layouts are decoded here and nowhere else; each table's
//! address and record layout is noted where it is read.

use crate::{Rom, iwram, lz77, u32at};
use bn6_battle::actor::ActorType;
use bn6_battle::content::*;
use bn6_battle::field::PanelType;
use bn6_battle::setup::{ActorEntry, ActorKind, ActorList, ActorListId, StageSettings};
use std::collections::BTreeMap;

/// The ROM's battle data.
pub fn content(rom: &Rom) -> Content {
    let actor_lists = actor_lists(rom);
    let mut chips = chips(rom);
    attach_gun_del_sol(rom, &mut chips);
    attach_recovery(rom, &mut chips);
    attach_swords(rom, &mut chips);
    attach_sp_damage(rom, &mut chips);
    attach_navi_damage(rom, &mut chips);
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
        assets: Default::default(),
        defs: Default::default(),
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

/// A chip's description, as R on the custom screen shows it
/// (`chip_getScript_8027D34`: `TextScriptChipDescriptions0` for chips up
/// to 0xFF, `TextScriptChipDesc1` after): its script opens the box at once
/// (`E8 06`), prints at speed 0 (`F1 00 00`) up to three lines (`E9`
/// between them) and waits for a key (`E7 01`). The lines are joined by
/// `\n`; a glyph the charset doesn't have reads as `?`.
fn chip_description(rom: &Rom, id: u32) -> Option<String> {
    const DESCRIPTIONS0: u32 = 0x086E_B8B8;
    const DESCRIPTIONS1: u32 = 0x086E_E0CC;
    let (archive, index) = if id <= 0xFF { (DESCRIPTIONS0, id) } else { (DESCRIPTIONS1, id & 0xFF) };
    if index >= rom.u16(archive) as u32 / 2 {
        return None;
    }
    let mut a = archive + rom.u16(archive + 2 * index) as u32;
    if rom.bytes(a, 2) != [0xE8, 0x06] || rom.bytes(a + 4, 3) != [0xF1, 0x00, 0x00] {
        return None;
    }
    a += 7;
    let mut text = String::new();
    loop {
        match rom.u8(a) {
            0xE9 => text.push('\n'),
            // A glyph of the second page.
            0xE4 => {
                text.push('?');
                a += 1;
            }
            b if (b as usize) < crate::CHARSET.len() => text.push_str(crate::CHARSET[b as usize]),
            b if b < 0xE4 => text.push('?'),
            // `FF`: the text is copied from the console's memory (DblBeast's,
            // Gregar's and Falzar's, which the game keeps outside their
            // scripts): the pack has none for them.
            0xFF => return None,
            // The key wait (or a command this reader doesn't know): the
            // text so far.
            _ => return Some(text),
        }
        a += 1;
    }
}

/// A navi's no-running message (L on the custom screen in a netbattle):
/// the characters in each of its lines. `TextScriptBattleRunDialog`'s
/// script 3 is MegaMan's and sends each link navi to its own script (`EF
/// 2F` and a script a navi, 0xFF for none); each shows the operator's
/// portrait (`F5`), opens the box (`E8 00`), prints the lines and waits for
/// A or B (`E7 00`).
fn run_message(rom: &Rom, navi: u32) -> Vec<u8> {
    const RUN_DIALOG: u32 = 0x086E_F78C;
    let script = |i: u32| RUN_DIALOG + rom.u16(RUN_DIALOG + 2 * i) as u32;
    let mut a = script(3);
    assert_eq!(rom.bytes(a, 2), [0xEF, 0x2F], "the no-running message doesn't start with the navi's jump");
    a = match rom.u8(a + 2 + navi) {
        0xFF => a + 14,
        own => script(own as u32),
    };
    assert_eq!(rom.u8(a), 0xF5, "navi {navi}'s no-running message: no portrait");
    assert_eq!(rom.bytes(a + 3, 2), [0xE8, 0x00], "navi {navi}'s no-running message: no box");
    a += 5;
    let mut lines = vec![0u8];
    loop {
        match rom.u8(a) {
            0xE9 => lines.push(0),
            0xE4 => {
                *lines.last_mut().unwrap() += 1;
                a += 1;
            }
            b if b < 0xE4 => *lines.last_mut().unwrap() += 1,
            0xE7 => {
                assert_eq!(rom.bytes(a + 1, 2), [0x00, 0xE6], "navi {navi}'s no-running message: its end");
                assert!(lines.len() <= 3, "navi {navi}'s no-running message has {} lines", lines.len());
                return lines;
            }
            b => panic!("navi {navi}'s no-running message: command {b:#x}"),
        }
        a += 1;
    }
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
                id: Some(id as ChipId),
                name: chip_name(rom, id),
                description: chip_description(rom, id),
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
                navi_damage: None,
                modifier: None,
                program_advances: Vec::new(),
                gun_del_sol: None,
                recovery: None,
                sword: None,
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
        assert!(level < 4, "GunDelSol chip {:#x} of level {level}", c.id.unwrap_or_default());
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

/// Recovery chips (action 0x20, `sub_80EC844`) heal `byte_80EC870[subtype]`
/// HP (ten halfwords; past them the routine reads its own code).
fn attach_recovery(rom: &Rom, chips: &mut [ChipData]) {
    for c in chips.iter_mut().filter(|c| c.action == 0x20) {
        assert!(c.subtype < 10, "recovery chip {:#x} of subtype {}", c.id.unwrap_or_default(), c.subtype);
        c.recovery = Some(rom.u16(0x080E_C870 + 2 * c.subtype as u32));
    }
}

/// Swords by their subtype: the blade (`byte_80EBB64`, 20 bytes, which
/// actions 0x13 and 0x49 both read) and action 0x13's slash (`sub_80EB862`:
/// the hit region's parameters `byte_80EBA18` and `byte_80EBA58`, four
/// bytes each, and the effect `byte_80EBAD8`, 16 rows each).
fn attach_swords(rom: &Rom, chips: &mut [ChipData]) {
    for c in chips.iter_mut().filter(|c| c.action == 0x13 || c.action == 0x49) {
        let v = c.subtype as u32;
        assert!(v < 20, "sword chip {:#x} of subtype {v}", c.id.unwrap_or_default());
        let slash = (c.action == 0x13).then(|| {
            assert!(v < 16, "action 0x13 chip {:#x} of subtype {v}", c.id.unwrap_or_default());
            let region = rom.bytes(0x080E_BA18 + 4 * v, 4);
            let hit = rom.bytes(0x080E_BA58 + 4 * v, 4);
            SwordSlash {
                region: region[0],
                hit_effect: region[1],
                target: region[2],
                self_type: region[3],
                hit_mod: hit[0],
                status: hit[1],
                bug: hit[2],
                bug_arg: hit[3],
                effect: rom.u8(0x080E_BAD8 + v),
            }
        });
        c.sword = Some(Sword { blade: rom.u8(0x080E_BB64 + v), slash });
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

/// Link navis' chips: damage formula n (24..=44, `sub_8010C50`) reads row
/// n - 23 of `byte_80212D4` (base, per buster level).
fn attach_navi_damage(rom: &Rom, chips: &mut [ChipData]) {
    for c in chips.iter_mut() {
        let Some(n) = c.damage.checked_sub(1023).filter(|n| (1..=21).contains(n)) else { continue };
        let row = 0x0802_12D4 + 2 * n as u32;
        c.navi_damage = Some(NaviChipDamage { base: rom.u8(row), per_level: rom.u8(row + 1) });
    }
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
/// A Program Advance record: the result chip and its recipe, by number.
struct PaRecord {
    result: ChipId,
    recipe: PaRecipe,
}

fn program_advance_records(rom: &Rom) -> Vec<PaRecord> {
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
        out.push(PaRecord { result, recipe });
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
                chip_bonus: navi_chip_bonus(rom, n as u8),
                run_message: run_message(rom, n),
                name_record: Some(name_record(rom, FIRST_NAME + n as u16)),
            }
        })
        .collect()
}

/// A link navi's chip bonus (`sub_800F09E`): its family (and whether
/// dimming chips of it count: only EraseMan's), and its row of
/// `byte_8021300` (15 bytes a row, by level).
fn navi_chip_bonus(rom: &Rom, navi: u8) -> Option<NaviChipBonus> {
    let (row, family, dimming_chips) = match navi {
        1 => (0, 0, false),
        2 => (1, 2, false),
        3 => (2, 5, false),
        4 => (3, 6, true),
        8 => (4, 8, false),
        9 => (5, 9, false),
        10 => (6, 9, false),
        _ => return None,
    };
    Some(NaviChipBonus {
        family: ChipFamily::from_number(family).expect("a family"),
        dimming_chips,
        by_level: rom.bytes(0x0802_1300 + 15 * row, 15).to_vec(),
    })
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
        empty_hand: empty_hand(rom),
        // By Rapid, then open panels ahead (`byte_80209CC`).
        buster_recovery: (0..5).map(|n| rom.bytes(0x0802_09CC + 6 * n, 6).try_into().unwrap()).collect(),
        // BCD times (`byte_8010B2C`).
        sp_deletion_times: (0..10).map(|i| u32at(rom, 0x0801_0B2C + 4 * i)).collect(),
        // `math_sinTable` through `math_cosTable`'s end: 384 halfwords.
        sine: (0..384).map(|i| rom.u16(0x0800_65E0 + 2 * i) as i16).collect(),
        // By hit-modifier bit (`byte_800E58C`, three bytes each) and by
        // collision direction (`byte_800E4E8`, four bytes each).
        push_vectors: std::array::from_fn(|i| slide(rom, 0x0800_E58C + 3 * i as u32)),
        ice_vectors: std::array::from_fn(|i| slide(rom, 0x0800_E4E8 + 4 * i as u32)),
        bubble_bob: std::array::from_fn(|i| rom.u8(0x0801_7868 + i as u32) as i8),
        lockon: lockon(rom),
        // The berserk controller's panel tables: the step conditions
        // (`byte_802D410` grounded, `byte_802D420` with AirShoe; 8 bytes
        // an alliance), an opponent's panel (`off_8109784`), what ends the
        // look behind an opponent (`byte_8015D78`) and the opposing
        // player's panel flag (`byte_80E74C4`).
        berserk: BerserkRules {
            step: StepRuleSet {
                grounded: [condition(rom, 0x0802_D410), condition(rom, 0x0802_D418)],
                floor_free: [condition(rom, 0x0802_D420), condition(rom, 0x0802_D428)],
            },
            opponent: [condition(rom, 0x0810_9784), condition(rom, 0x0810_978C)],
            blocking: [u32at(rom, 0x0801_5D78), u32at(rom, 0x0801_5D7C)],
            opposing_player: [u32at(rom, 0x080E_74C4), u32at(rom, 0x080E_74C8)],
        },
        custom_screen: custom_screen(rom),
        // Three bytes by NameID, 0..=0x1C3 (`byte_80182C4`).
        actor_records: (0..0x1C4)
            .map(|n| {
                let rec = rom.bytes(0x0801_82C4 + 3 * n, 3);
                let actor_type = [ActorType::Virus, ActorType::Navi, ActorType::Player][rec[1] as usize];
                NaviRecord { version: rec[0], actor_type, ai_index: rec[2] }
            })
            .collect(),
        // By form, the base form and the ten Crosses (`byte_80203EA`).
        cross_palettes: rom.bytes(0x0802_03EA, 11).to_vec(),
    }
}

/// What an empty hand's chip (0xFFFF) reads: the record 0xFFFF records
/// past the chip table, which is other ROM data (`getChip8021DA8` doesn't
/// check the id). Its family (+6), element (+4) and flags (+9) bytes.
fn empty_hand(rom: &Rom) -> EmptyHandChip {
    let r = CHIP_TABLE + 0xFFFF * CHIP_RECORD;
    EmptyHandChip {
        null_family: ChipFamily::from_number(rom.u8(r + 6)) == Some(ChipFamily::Null),
        fire: rom.u8(r + 4) == 1,
        flags: ChipFlags(rom.u8(r + 9)),
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

/// The Beast Out lock-on (`ho_8026554`): what each mode of `jt_8026584`
/// does with its lists (signed bytes, or byte pairs, up to 0x7F; each
/// routine loads its list through a literal-pool slot), the column shifts
/// tried when nothing fits (`byte_8026735`), the clear-path panel
/// condition (`byte_8026544`) and the charged sword's modes
/// (`byte_80EB028`).
fn lockon(rom: &Rom) -> Lockon {
    let list = |mut a: u32, pairs: bool| {
        let mut v = Vec::new();
        while rom.u8(a) != 0x7F {
            v.push((rom.u8(a) as i8, if pairs { rom.u8(a + 1) as i8 } else { 0 }));
            a += if pairs { 2 } else { 1 };
        }
        v
    };
    let offsets = |a: u32| list(a, true).into_iter().map(|(dx, dy)| PanelOffset { dx, dy }).collect::<Vec<_>>();
    let shifts = |a: u32| list(a, false).into_iter().map(|(s, _)| s).collect::<Vec<_>>();
    let near = |mode: u8, pool: u32, column_shifts: bool, prefers_middle_row: bool| LockonMode {
        mode,
        rule: LockonRule::Near,
        offsets: offsets(u32at(rom, pool)),
        column_shifts,
        prefers_middle_row,
        ..Default::default()
    };
    let modes = vec![
        // sub_802661C
        LockonMode { mode: 0, rule: LockonRule::Stay, ..Default::default() },
        // sub_8026622: byte_802673C, byte_802673A (which runs on into
        // byte_802673C) and the row shifts byte_8026730.
        LockonMode {
            mode: 1,
            rule: LockonRule::Row,
            offsets: offsets(u32at(rom, 0x0802_67EC)),
            same_row_offsets: offsets(u32at(rom, 0x0802_67F0)),
            row_shifts: shifts(u32at(rom, 0x0802_67F4)),
            ..Default::default()
        },
        // sub_8026650 .. sub_802669E: sub_80265D0, some then sub_80265FE.
        near(2, 0x0802_67FC, true, false),
        near(3, 0x0802_6800, true, true),
        near(4, 0x0802_6804, true, false),
        near(5, 0x0802_6808, true, false),
        near(6, 0x0802_680C, true, true),
        near(7, 0x0802_6810, true, false),
        near(8, 0x0802_6814, true, false),
        near(9, 0x0802_6818, true, true),
        // sub_80266AC: sub_80264A8 alone (a clear path, no shifts).
        LockonMode { clear_path: true, ..near(0x0A, 0x0802_681C, false, false) },
        // sub_80266BA: byte_80267A6, or from its second pair on when the
        // target is in the far column.
        LockonMode {
            far_column_offsets: Some(offsets(u32at(rom, 0x0802_6820) + 2)),
            ..near(0x0B, 0x0802_6820, true, false)
        },
        near(0x0C, 0x0802_6824, true, false),
        near(0x0D, 0x0802_6828, true, false),
        // sub_80266F2: sub_8026450 alone, then sub_80265FE.
        near(0x0E, 0x0802_682C, false, true),
        near(0x0F, 0x0802_6830, true, false),
        near(0x10, 0x0802_6834, true, true),
        near(0x11, 0x0802_6838, true, false),
        near(0x12, 0x0802_683C, true, false),
    ];
    Lockon {
        modes,
        column_shifts: shifts(u32at(rom, 0x0802_67E8)),
        clear_path: [condition(rom, 0x0802_6544), condition(rom, 0x0802_654C)],
        // Up to the literal pool that follows it.
        charged_sword_modes: rom.bytes(0x080E_B028, 0x14).to_vec(),
    }
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
            StageSettings {
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
    // Boomerang rows (`byte_80CA26C`, 12 bytes): +3 turns panels to grass,
    // +4 the speed along a row, +8 along the column.
    let boomerangs = (0..5u32)
        .map(|i| {
            let r = 0x080C_A26C + 12 * i;
            BoomerangKind {
                id: i as u8,
                speed: u32at(rom, r + 4) as i32,
                turn_speed: u32at(rom, r + 8) as i32,
                grass: rom.u8(r + 3) != 0,
            }
        })
        .collect();
    ObjectData {
        attachments: (0..ATTACHMENTS).map(|i| attachment(rom, i)).collect(),
        rocks,
        absorbed_sprites: (0..15).map(|i| sprite_at(0x080E_98C0 + 2 * i)).collect(),
        body_overlays,
        sun_beam_looks: (0..2).map(|i| sprite_at(0x080E_5C28 + 2 * i)).collect(),
        sword_waves: sword_waves(rom),
        boomerangs,
        projectiles: projectiles(rom),
        flying_shots: flying_shots(rom),
        name_looks: name_looks(rom),
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

/// An element byte: the primary element in the low bits, secondary bits
/// above.
fn element_byte(v: u8) -> (Element, SecondaryElements) {
    (element(v & 0x0F), secondary(v & 0xF0))
}

/// The projectile's kinds (attack object #0, `sub_80C4E58`): the 12-byte
/// records of `off_80C4C78` by its first parameter, 40 of them up to the
/// routine's code: collision self type, target type, hit modifier, element
/// byte, hit spark, sprite category (0xFF: not drawn) and index,
/// animation, the panel type its hit leaves (0xFF: none), status byte, bug
/// code and argument. The routine adds what it does by kind number
/// (`sub_80C4F02`): kinds 7 and 0x15 crack the panel they hit and 0x16
/// breaks it, whatever their panel type says; 0x22 and 0x24 leave theirs
/// for the left side's shots and a road the other way (0xC and 0xB) for
/// the right side's; 0xC bursts (`sub_80C5014`, `sub_80C5050`); 0x1D
/// climbs (`sub_80C5090`).
fn projectiles(rom: &Rom) -> Vec<ProjectileKind> {
    const TABLE: u32 = 0x080C_4C78;
    const COUNT: u32 = (0x080C_4E58 - TABLE) / 12;
    let panel_type = |v: u8| PanelType::ALL[v as usize];
    (0..COUNT)
        .map(|i| {
            let r = rom.bytes(TABLE + 12 * i, 12);
            let id = i as u8;
            let (element, secondary) = element_byte(r[3]);
            let hit_panel = match id {
                0x07 | 0x15 => Some(PanelHit::Crack),
                0x16 => Some(PanelHit::Break),
                0x22 => Some(PanelHit::SetType { left_side: panel_type(r[8]), right_side: PanelType::RoadRight }),
                0x24 => Some(PanelHit::SetType { left_side: panel_type(r[8]), right_side: PanelType::RoadLeft }),
                _ if r[8] != 0xFF => Some(PanelHit::SetType { left_side: panel_type(r[8]), right_side: panel_type(r[8]) }),
                _ => None,
            };
            let sprite = (r[5] != 0xFF).then_some(SpriteId { category: r[5], index: r[6] });
            ProjectileKind {
                id,
                self_type: r[0],
                target_type: r[1],
                hit_mod: r[2],
                element,
                secondary,
                hit_effect: r[4],
                sprite,
                anim: if sprite.is_some() { r[7] } else { 0 },
                status: r[9],
                bug: r[10],
                bug_arg: if r[10] != 0 { r[11] } else { 0 },
                hit_panel,
                bursts: id == 0x0C,
                climbs: id == 0x1D,
            }
        })
        .collect()
}

/// The flying shot's kinds (attack object #0xB, `sub_80C60A8`): the
/// 16-byte records of `byte_80C6038` by its first parameter, 7 of them up
/// to the routine's code: collision self type, target type, hit modifier,
/// element byte, hit spark, sprite category and index, animation,
/// highlight, range, shadow, status byte, speed (u32, 16.16). The routine
/// adds by kind number: kind 6 is a thrown obstacle, whose look its
/// spawner gives; kind 2 shows its spark at its panel's center
/// (`sub_801A100`) and sounds 0x18A when it sets off; kind 5 leaves effect
/// 7 when its range runs out.
fn flying_shots(rom: &Rom) -> Vec<FlyingShotKind> {
    const TABLE: u32 = 0x080C_6038;
    const COUNT: u32 = (0x080C_60A8 - TABLE) / 16;
    (0..COUNT)
        .map(|i| {
            let r = rom.bytes(TABLE + 16 * i, 16);
            let id = i as u8;
            let (element, secondary) = element_byte(r[3]);
            let flag = |v: u8| match v {
                0 => false,
                1 => true,
                v => panic!("flying shot {id}: flag byte {v:#x}"),
            };
            FlyingShotKind {
                id,
                self_type: r[0],
                target_type: r[1],
                hit_mod: r[2],
                element,
                secondary,
                hit_effect: r[4],
                sprite: SpriteId { category: r[5], index: r[6] },
                anim: r[7],
                highlight: flag(r[8]),
                range: r[9],
                shadow: flag(r[10]),
                status: r[11],
                speed: u32at(rom, TABLE + 16 * i + 12) as i32,
                obstacle: id == 6,
                panel_spark: id == 2,
                launch_sound: (id == 2).then_some(0x18A),
                end_effect: (id == 5).then_some(7),
            }
        })
        .collect()
}

/// A byte the object code only tests for zero, as a flag.
fn flag_byte(rom: &Rom, a: u32) -> bool {
    match rom.u8(a) {
        0 => false,
        1 => true,
        v => panic!("flag byte {v:#x} at {a:#x}"),
    }
}

/// The sword wave's kinds (`byte_80D7F4C`, 16 bytes each, read by
/// `sub_80D80B4`), up to the object's code: collision types and hit
/// modifier, region, sprite, animation, whether it animates, whether it
/// highlights, reach, shadow (low nibble) and palette (high), status,
/// speed.
fn sword_waves(rom: &Rom) -> Vec<SwordWave> {
    const TABLE: u32 = 0x080D_7F4C;
    const END: u32 = 0x080D_807C;
    (0..(END - TABLE) / 16)
        .map(|i| {
            let r = TABLE + 16 * i;
            let look = rom.u8(r + 10);
            assert!(look & 0xF <= 1, "sword wave {i} shadow nibble {look:#x}");
            SwordWave {
                id: i as u8,
                self_type: rom.u8(r),
                target_type: rom.u8(r + 1),
                hit_mod: rom.u8(r + 2),
                region: rom.u8(r + 3),
                sprite: SpriteId { category: rom.u8(r + 4), index: rom.u8(r + 5) },
                anim: rom.u8(r + 6),
                animates: flag_byte(rom, r + 7),
                highlight: flag_byte(rom, r + 8),
                reach: rom.u8(r + 9),
                ground_shadow: look & 0xF != 0,
                palette: look >> 4,
                status: rom.u8(r + 11),
                speed: u32at(rom, r + 12) as i32,
            }
        })
        .collect()
}

/// `byte_8021220`: how a field object looks by NameID, 5 bytes from
/// NameID 0xCD, for every NameID `sub_800F26C` reads it for (0xCD..=0xFF;
/// the entries past 0xEB are the bytes that follow the table).
fn name_looks(rom: &Rom) -> Vec<NameLook> {
    const TABLE: u32 = 0x0802_1220;
    (0xCDu16..=0xFF)
        .map(|name_id| {
            let a = TABLE + 5 * (name_id as u32 - 0xCD);
            let category = rom.u8(a);
            NameLook {
                name_id,
                sprite: (category != 0xFF).then(|| SpriteId { category, index: rom.u8(a + 1) }),
                anim: rom.u8(a + 2),
                palette: rom.u8(a + 3),
                shadow: rom.u8(a + 4) != 0,
            }
        })
        .collect()
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
