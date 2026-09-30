//! A small hand-authored content set for tests and examples.
//!
//! Everything here is made up: a MegaMan-like navi and its base form,
//! a few chips that use GunDelSol, two dimming chips (one grabs a column)
//! and a navi chip, rocks,
//! sprites with short animations, and rules written from the engine's own
//! flag semantics (docs/engine/field-collision-damage.md). It is not BN6's
//! data, which comes only from a content pack extracted from the user's ROM
//! (`bn6-extract content`), and its numbers are chosen for tests, not taken
//! from the game. It has just what battles of two such navis need:
//! stepping, the chips below, custom screens, rocks and the round's flow.
//!
//! Its scripts are this repository's BN6 scripts (content/bn6, the source
//! overlay: this project's own code, not game data), read from the
//! repository and registered under this content's names, so the tests run
//! the real scripts on made-up data.

use super::*;
use crate::actor::ActorType;
use bn6_content_api::Pool;
use crate::field::{PanelType, pflags};
use crate::setup::{ActorEntry, ActorKind, ActorList, ActorListId, BattleSettings, effects};
use std::sync::Arc;

/// Chips: three GunDelSol levels (action 0x37 with subtypes 0..=2) and
/// an EX (subtype 3, two columns).
pub const SUN_GUN_1: ChipId = 0x01;
pub const SUN_GUN_2: ChipId = 0x02;
pub const SUN_GUN_3: ChipId = 0x03;
pub const SUN_GUN_EX: ChipId = 0x04;
/// A dimming (action 0x15, subtype 1: the invisibility freeze).
pub const VEIL: ChipId = 0x05;
/// A navi chip (action 0x1B, subtype 5: the eraser navi).
pub const ERASER: ChipId = 0x06;
/// A dimming chip (action 0x15, subtype 0) that grabs a column.
pub const GRAB: ChipId = 0x07;
/// A Reflector (action 0x2B, subtype 0): guards for 30 ticks.
pub const MIRROR: ChipId = 0x08;
/// A recovery chip (action 0x20): heals 40 HP.
pub const MEND: ChipId = 0x09;
/// The thrown chips (action 0x12): a bomb (subtype 0), a seed that
/// poisons panels (subtype 12), a flash bomb (subtype 14) and a bug bomb
/// (subtype 7).
pub const BOMB: ChipId = 0x0A;
pub const SEED: ChipId = 0x0B;
pub const FLASH: ChipId = 0x0C;
pub const BUG: ChipId = 0x0D;
/// A chip that sends bees (action 0x39, RskyHny's).
pub const BEES: ChipId = 0x0E;
/// A chip that sends an elec dragon (action 0x51, subtype 1).
pub const DRAGON: ChipId = 0x0F;
/// A sword (action 0x13, subtype 1: a column of three panels ahead).
pub const BLADE: ChipId = 0x10;
/// A step sword (the same, with its first parameter set: it steps two
/// panels ahead first).
pub const STEP_BLADE: ChipId = 0x11;
/// A strike at stunned or grounded opponents (action 0x49, subtype 2).
pub const STUN_BLADE: ChipId = 0x12;
/// A dimming chip (action 0x15, subtype 6) that places a rock (variant 1)
/// in front of its user.
pub const CUBE: ChipId = 0x13;
/// A trap chip (action 0x15, subtype 20, Param1 3: no object).
pub const TRAP: ChipId = 0x14;

/// Actor lists: two navis, side 1's first (the usual netbattle order)...
pub const TWO_NAVIS: ActorListId = ActorListId(0);
/// ...side 0's first...
pub const TWO_NAVIS_SIDE0_FIRST: ActorListId = ActorListId(1);
/// ...and two navis with two rocks, one on each side.
pub const NAVIS_AND_ROCKS: ActorListId = ActorListId(2);

/// Battle settings: a link battle on the plain field with `TWO_NAVIS`,
/// the same with `TWO_NAVIS_SIDE0_FIRST`, and with `NAVIS_AND_ROCKS`.
pub const LINK_BATTLE: u8 = 0;
pub const LINK_BATTLE_SIDE0_FIRST: u8 = 1;
pub const ROCK_BATTLE: u8 = 2;

/// The navi's sprite (base form).
pub const NAVI_SPRITE: SpriteId = SpriteId { category: 0, index: 0 };
/// Where the navi holds a gun.
pub const GUN_POINT: AttachPoint = AttachPoint { x: 20, y: 16 };

// Collision type bits by what they mean (field-collision-damage.md §3.3).
const ATTACK: [u32; 2] = [0x8000_0000, 0x4000_0000];
const OBJECT: [u32; 2] = [0x2000_0000, 0x1000_0000];
const BODY: [u32; 2] = [0x0800_0000, 0x0400_0000];
const OTHER_BODY: [u32; 2] = [0x0200_0000, 0x0100_0000];
const NEUTRAL: u32 = 0x0080_0000;
const PLAYER: [u32; 2] = [0x0040_0000, 0x0020_0000];
const FLOATING: u32 = 0x0010_0000;
const BLOCKER: u32 = 0x0008_0000;
const WHILE_DIMMED: u32 = 0x0001_0000;
const REACHES_FLOATING: u32 = 0x0080;
const BREAKS: u32 = 0x0002;
// A panel flag every panel type has.
const ON_FIELD: u32 = 0x0001_0000;

/// The content set, shared.
pub fn content() -> Arc<Content> {
    Arc::new(build())
}

/// A round on this content with battle settings `settings`, both navis
/// with `stats`: RNG seed 1, side 0's perspective, no set score, no
/// folders.
pub fn round_setup(settings: u8, stats: crate::setup::NaviStats) -> crate::setup::RoundSetup {
    let content = build();
    crate::setup::RoundSetup {
        content: content.hash(),
        settings: content.rules.stages.settings(settings),
        navi_stats: [stats; 2],
        rng: 1,
        local_side: 0,
        score: Default::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        sp_times: Default::default(),
        players: Default::default(),
        link_delay: 0,
    }
}

/// A navi with `hp` HP and nothing else of note.
pub fn stats(hp: u16) -> crate::setup::NaviStats {
    crate::setup::NaviStats { hp, max_hp: hp, max_base_hp: hp, ..Default::default() }
}

/// The content set.
pub fn build() -> Content {
    Content {
        chips: chips(),
        navis: vec![navi()],
        forms: vec![base_form()],
        rules: rules(),
        objects: objects(),
        effects: vec![EffectSprite { sprite: SpriteId { category: 0x14, index: 0 }, anim: 0, palette: 0 }; 0x70],
        sparks: vec![EffectSprite { sprite: SpriteId { category: 0x14, index: 1 }, anim: 0, palette: 0 }; 16],
        regions: regions(),
        panel_layouts: vec![PanelLayout { rows: [[PanelType::Normal; 6]; 3] }],
        animations: animations(),
        weapons: weapons(),
        scripts: scripts(),
    }
}

/// Where the BN6 scripts are (the source overlay in this repository).
const OVERLAY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");

/// The test content's scripts: modules of the BN6 overlay, by the paths
/// this content registers them under (the ones the overlay's modules
/// `require` stay where they are).
pub fn scripts() -> Scripts {
    static SCRIPTS: std::sync::OnceLock<Scripts> = std::sync::OnceLock::new();
    SCRIPTS
        .get_or_init(|| {
            let read = |path: &str| {
                let file = format!("{OVERLAY}/{path}.luau");
                std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"))
            };
            let modules = [
                ("lib/slot", "lib/slot"),
                ("objects/attachment/attachment", "objects/attachment/attachment"),
                ("objects/sun-beam/sun_beam", "objects/sun-beam/sun_beam"),
                ("chips/001-sungun1/chip", "chips/00f-gundels1/chip"),
                ("objects/erase-man/erase_man", "objects/erase-man/erase_man"),
                ("objects/erase-mark/erase_mark", "objects/erase-mark/erase_mark"),
                ("objects/erase-beam/erase_beam", "objects/erase-beam/erase_beam"),
                ("objects/area-grab/area_grab", "objects/area-grab/area_grab"),
                ("objects/grab-shot/grab_shot", "objects/grab-shot/grab_shot"),
                ("objects/dust-ball/dust_ball", "objects/dust-ball/dust_ball"),
                ("lib/buster", "lib/buster"),
                ("objects/reflector-shield/reflector_shield", "objects/reflector-shield/reflector_shield"),
                ("objects/reflected-shot/reflected_shot", "objects/reflected-shot/reflected_shot"),
                ("chips/008-mirror/chip", "chips/083-rflectr1/chip"),
                ("chips/009-mend/chip", "chips/09a-recov10/chip"),
                ("lib/region", "lib/region"),
                ("lib/trajectory", "lib/trajectory"),
                ("lib/hp", "lib/hp"),
                ("objects/bomb/bomb", "objects/bomb/bomb"),
                ("objects/bomb-slash/bomb_slash", "objects/bomb-slash/bomb_slash"),
                ("objects/energy-burst/energy_burst", "objects/energy-burst/energy_burst"),
                ("objects/seed/seed", "objects/seed/seed"),
                ("objects/flash-bomb/flash_bomb", "objects/flash-bomb/flash_bomb"),
                ("objects/bug-bomb/bug_bomb", "objects/bug-bomb/bug_bomb"),
                ("objects/smoke-puff/smoke_puff", "objects/smoke-puff/smoke_puff"),
                ("objects/black-bomb/black_bomb", "objects/black-bomb/black_bomb"),
                ("objects/panel-bursts/panel_bursts", "objects/panel-bursts/panel_bursts"),
                ("lib/panels", "lib/panels"),
                ("chips/00a-bomb/chip", "chips/036-minibomb/chip"),
                ("chips/00e-bees/chip", "chips/025-rskyhny1/chip"),
                ("objects/honey-bee/honey_bee", "objects/honey-bee/honey_bee"),
                ("chips/00f-dragon/chip", "chips/02e-heatdrgn/chip"),
                ("lib/dragon", "lib/dragon"),
                ("objects/dragon-head/dragon_head", "objects/dragon-head/dragon_head"),
                ("objects/dragon-body/dragon_body", "objects/dragon-body/dragon_body"),
                ("lib/sword", "lib/sword"),
                ("chips/010-blade/chip", "chips/047-sword/chip"),
                ("chips/012-stunblade/chip", "chips/056-mchnswrd/chip"),
                ("objects/invisible/invisible", "objects/invisible/invisible"),
                ("objects/rock/rock", "objects/rock/rock"),
                ("objects/rock-cube/rock_cube", "objects/rock-cube/rock_cube"),
                ("objects/rock-debris/rock_debris", "objects/rock-debris/rock_debris"),
                ("objects/trap-chip/trap_chip", "objects/trap-chip/trap_chip"),
            ];
            let weapons = weapons().into_iter().map(|w| {
                let module = w.script;
                (module.clone(), module)
            });
            let modules = modules.iter().map(|&(to, from)| (to.to_string(), from.to_string())).chain(weapons);
            Scripts { modules: modules.map(|(to, from)| (to, read(&from))).collect() }
        })
        .clone()
}

/// MegaMan's weapon routines scripts implement: the BN6 overlay's buster,
/// charged shot, blank shot, DustCross's charged shot and the absorbed
/// obstacle throw.
fn weapons() -> Vec<WeaponData> {
    let weapon = |id: u8, name: &str, action: Option<u8>, script: &str| WeaponData {
        id,
        name: name.into(),
        action,
        script: format!("navis/00-megaman/weapons/{script}"),
    };
    vec![
        weapon(0x00, "Buster", None, "00-buster/buster"),
        weapon(0x01, "Charged shot", None, "01-charged-shot/charged_shot"),
        weapon(0x02, "Blank shot", Some(0x33), "02-blank-shot/blank_shot"),
        weapon(0x28, "Dust charge", Some(0x57), "28-dust-charge/dust_charge"),
        weapon(0x2B, "Throw absorbed", None, "2b-throw-absorbed/throw_absorbed"),
    ]
}

/// The object kinds scripts implement, by name (in name order, as a pack
/// lists them).
fn kinds() -> Vec<ObjectKind> {
    let kind = |name: &str, pool, index, script: &str| ObjectKind {
        name: name.into(),
        pool,
        index,
        script: script.into(),
        scratch_position: false,
        scratch_z_fraction: false,
        actor_list_entry: None,
    };
    let mut kinds = vec![
        kind("attachment", Pool::Actor, 0x05, "objects/attachment/attachment"),
        kind("sun-beam", Pool::Effect, 0x48, "objects/sun-beam/sun_beam"),
        kind("erase-man", Pool::Actor, 0x15, "objects/erase-man/erase_man"),
        kind("erase-mark", Pool::Effect, 0x62, "objects/erase-mark/erase_mark"),
        kind("erase-beam", Pool::Attack, 0xC3, "objects/erase-beam/erase_beam"),
        ObjectKind { scratch_position: true, ..kind("area-grab", Pool::Effect, 0x03, "objects/area-grab/area_grab") },
        kind("grab-shot", Pool::Attack, 0x0F, "objects/grab-shot/grab_shot"),
        ObjectKind { scratch_z_fraction: true, ..kind("dust-ball", Pool::Attack, 0xB0, "objects/dust-ball/dust_ball") },
        kind("reflector-shield", Pool::Attack, 0x2B, "objects/reflector-shield/reflector_shield"),
        kind("reflected-shot", Pool::Attack, 0x2F, "objects/reflected-shot/reflected_shot"),
        kind("bomb", Pool::Attack, 0x08, "objects/bomb/bomb"),
        kind("bomb-slash", Pool::Attack, 0x0A, "objects/bomb-slash/bomb_slash"),
        kind("energy-burst", Pool::Attack, 0x11, "objects/energy-burst/energy_burst"),
        kind("seed", Pool::Attack, 0x4F, "objects/seed/seed"),
        kind("flash-bomb", Pool::Attack, 0xA4, "objects/flash-bomb/flash_bomb"),
        kind("bug-bomb", Pool::Attack, 0xA5, "objects/bug-bomb/bug_bomb"),
        kind("smoke-puff", Pool::Effect, 0x14, "objects/smoke-puff/smoke_puff"),
        kind("honey-bee", Pool::Attack, 0x74, "objects/honey-bee/honey_bee"),
        kind("dragon-head", Pool::Attack, 0xC9, "objects/dragon-head/dragon_head"),
        kind("dragon-body", Pool::Attack, 0xC8, "objects/dragon-body/dragon_body"),
        ObjectKind { scratch_position: true, ..kind("invisible", Pool::Effect, 0x5D, "objects/invisible/invisible") },
        ObjectKind { actor_list_entry: Some(8), ..kind("rock", Pool::Attack, 0x59, "objects/rock/rock") },
        ObjectKind { scratch_position: true, ..kind("rock-cube", Pool::Effect, 0x37, "objects/rock-cube/rock_cube") },
        kind("rock-debris", Pool::Effect, 0x38, "objects/rock-debris/rock_debris"),
        ObjectKind { scratch_position: true, ..kind("trap-chip", Pool::Effect, 0x2A, "objects/trap-chip/trap_chip") },
    ];
    kinds.sort_by(|a, b| a.name.cmp(&b.name));
    kinds
}

/// A chip record with the fields tests don't care about filled in.
fn chip(id: ChipId, name: &str, action: u8, subtype: u8) -> ChipData {
    ChipData {
        id,
        name: name.into(),
        codes: vec![ChipCode(0), ChipCode::ASTERISK],
        element: Element::Null,
        rarity: 0,
        family: ChipFamily::Null,
        class: ChipClass::Standard,
        mb: 10,
        flags: ChipFlags(ChipFlags::STANDARD_LIBRARY),
        hit_param: 0,
        action,
        subtype,
        beast_lockon: false,
        params: [0; 4],
        lockout: 0,
        extra_flags: ExtraChipFlags::default(),
        lockon_mode: 0,
        damage: 0,
        library_number: id,
        library_index: id as u8,
        sort_key: id,
        slot_in_limit: 3,
        dark_substitute: None,
        sp_damage: None,
        modifier: None,
        program_advances: Vec::new(),
        gun_del_sol: None,
        recovery: None,
        sword: None,
        script: None,
    }
}

fn sun_gun(id: ChipId, name: &str, level: u8, firing_ticks: u16) -> ChipData {
    let beam_look = if level < 3 { 0 } else { 1 };
    ChipData {
        beast_lockon: true,
        lockon_mode: 1,
        gun_del_sol: Some(GunDelSol {
            firing_ticks,
            beam: SunBeamLook { look: beam_look, palette: 0 },
            beam_in_sun: SunBeamLook { look: beam_look, palette: 1 },
            gun: AttachmentKind {
                id: 1 + level,
                sprite: SpriteId { category: 0x0C, index: 0x01 },
                palette: level,
                lift: 0,
                attach_point: Some(3),
            },
        }),
        script: Some("chips/001-sungun1/chip".into()),
        ..chip(id, name, 0x37, level)
    }
}

/// A sword chip of `action` holding blade 7; action 0x13's slash hits a
/// column of three panels.
fn blade(id: ChipId, name: &str, action: u8, subtype: u8, step: bool) -> ChipData {
    let slash = SwordSlash {
        region: 4,
        hit_effect: 0xFF,
        target: 5,
        self_type: 7,
        hit_mod: 3,
        status: 0,
        bug: 0,
        bug_arg: 0,
        effect: 0x16,
    };
    ChipData {
        flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
        family: ChipFamily::Sword,
        hit_param: 30,
        params: [step as u8, 0, 0, 0],
        damage: 80,
        sword: Some(Sword { blade: 7, slash: (action == 0x13).then_some(slash) }),
        script: Some(if action == 0x13 { "chips/010-blade/chip" } else { "chips/012-stunblade/chip" }.into()),
        ..chip(id, name, action, subtype)
    }
}

fn chips() -> Vec<ChipData> {
    vec![
        ChipData { class: ChipClass::Special, codes: vec![], ..chip(0, "Blank", 0, 0) },
        sun_gun(SUN_GUN_1, "SunGun1", 0, 48),
        sun_gun(SUN_GUN_2, "SunGun2", 1, 72),
        sun_gun(SUN_GUN_3, "SunGun3", 2, 96),
        sun_gun(SUN_GUN_EX, "SunGunX", 3, 96),
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::STANDARD_LIBRARY),
            extra_flags: ExtraChipFlags(ExtraChipFlags::RUSH_CANCELS),
            family: ChipFamily::Plus,
            script: Some("objects/invisible/invisible".into()),
            ..chip(VEIL, "Veil", 0x15, 1)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            family: ChipFamily::Cursor,
            class: ChipClass::Mega,
            hit_param: 100,
            params: [16, 0, 0, 0],
            damage: 60,
            script: Some("objects/erase-man/erase_man".into()),
            ..chip(ERASER, "Eraser", 0x1B, 5)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::STANDARD_LIBRARY),
            hit_param: 100,
            params: [1, 0, 0, 0],
            damage: 10,
            script: Some("objects/area-grab/area_grab".into()),
            ..chip(GRAB, "Grab", 0x15, 0)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
            hit_param: 20,
            params: [30, 0, 0, 0],
            damage: 50,
            script: Some("chips/008-mirror/chip".into()),
            ..chip(MIRROR, "Mirror", 0x2B, 0)
        },
        ChipData { recovery: Some(40), script: Some("chips/009-mend/chip".into()), ..chip(MEND, "Mend", 0x20, 1) },
        thrown(BOMB, "Bomb", 0, [0, 0, 0, 0], 50),
        thrown(SEED, "Seed", 12, [0, 0, 0, 0], 10),
        thrown(FLASH, "Flash", 14, [1, 0, 0, 0], 40),
        thrown(BUG, "Bug", 7, [0, 0, 0, 0], 0),
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
            element: Element::Wood,
            hit_param: 30,
            params: [1, 0, 0, 0],
            damage: 20,
            script: Some("chips/00e-bees/chip".into()),
            ..chip(BEES, "Bees", 0x39, 0)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
            element: Element::Elec,
            hit_param: 30,
            damage: 100,
            script: Some("chips/00f-dragon/chip".into()),
            ..chip(DRAGON, "Dragon", 0x51, 1)
        },
        blade(BLADE, "Blade", 0x13, 1, false),
        blade(STEP_BLADE, "StepBld", 0x13, 1, true),
        blade(STUN_BLADE, "StunBld", 0x49, 2, false),
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::STANDARD_LIBRARY),
            hit_param: 100,
            params: [1, 0, 0, 0],
            damage: 200,
            script: Some("objects/rock-cube/rock_cube".into()),
            ..chip(CUBE, "Cube", 0x15, 6)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::STANDARD_LIBRARY),
            params: [3, 0, 0, 0],
            script: Some("objects/trap-chip/trap_chip".into()),
            ..chip(TRAP, "Trap", 0x15, 20)
        },
    ]
}

/// A thrown chip (action 0x12) of `subtype`.
fn thrown(id: ChipId, name: &str, subtype: u8, params: [u8; 4], damage: u16) -> ChipData {
    ChipData {
        flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
        hit_param: 30,
        params,
        damage,
        script: Some("chips/00a-bomb/chip".into()),
        ..chip(id, name, 0x12, subtype)
    }
}

fn navi() -> NaviData {
    let mut attach_points = vec![AttachPoint { x: 4, y: 24 }; 34];
    attach_points[3] = GUN_POINT;
    NaviData {
        id: 0,
        name: "MegaMan".into(),
        sprite: SpriteId { category: 8, index: 0 },
        element: Element::Null,
        weakness: SecondaryElements::default(),
        buster_bonus: 1,
        move_lag: vec![4; 11],
        win_banner: BannerId(0x40),
        lose_banner: BannerId(0x44),
        merge_height: 0,
        own_chip: None,
        name_record: Some(NameData { id: 0x1A0, version: 0, actor_type: ActorType::Player, ai_index: 0, attach_points }),
    }
}

fn base_form() -> FormData {
    FormData {
        id: 0,
        name: "Base".into(),
        sprite: NAVI_SPRITE,
        element: Element::Null,
        weakness: SecondaryElements::default(),
        weapons: FormWeapons {
            mode9_a: 0xFF,
            a_charge: 0xFF,
            buster: 0,
            charge_shot: 1,
            back_special: 0xFF,
            alt_a_charge: 0xFF,
        },
        buster_bonus: 0,
        name_record: None,
    }
}

fn rules() -> Rules {
    // Collision types by what they are.
    let both = |f: &dyn Fn(usize) -> u32| [f(0), f(1)];
    let mut collision_types = vec![[0, 0]; 0x59];
    let attack = both(&|s| ATTACK[s] | REACHES_FLOATING);
    collision_types[0x01] = both(&|s| BODY[s] | PLAYER[s] | WHILE_DIMMED | REACHES_FLOATING);
    collision_types[0x10] = both(&|s| BODY[s] | PLAYER[s] | WHILE_DIMMED | REACHES_FLOATING | FLOATING);
    collision_types[0x02] = both(&|s| ATTACK[s ^ 1] | OBJECT[s ^ 1] | BODY[s ^ 1] | OTHER_BODY[s ^ 1] | NEUTRAL);
    collision_types[0x05] = both(&|s| OBJECT[s ^ 1] | BODY[s ^ 1] | OTHER_BODY[s ^ 1] | NEUTRAL);
    for t in [0x04, 0x07, 0x0A, 0x15, 0x16, 0x2C, 0x48] {
        collision_types[t] = attack;
    }
    collision_types[0x2A] = collision_types[0x05];
    collision_types[0x0E] = [NEUTRAL | BLOCKER | WHILE_DIMMED | REACHES_FLOATING | BREAKS; 2];
    collision_types[0x0F] = [ATTACK[0] | ATTACK[1] | BODY[0] | BODY[1] | BREAKS; 2];
    // Thrown things: a flash's hit, a set-down bomb (an object either side
    // can hit) and what it reacts to, a bug bomb and its target.
    collision_types[0x0B] = attack;
    collision_types[0x0C] = both(&|s| OBJECT[s] | NEUTRAL);
    collision_types[0x0D] = both(&|s| ATTACK[s ^ 1] | BODY[s ^ 1]);
    collision_types[0x4E] = both(&|s| OBJECT[s] | NEUTRAL);
    collision_types[0x14] = both(&|s| ATTACK[s ^ 1] | BODY[s ^ 1]);

    // Panels: what each type adds to a panel's flags word.
    let types = PanelType::ALL
        .iter()
        .map(|&t| {
            let (flags, road_slide) = match t {
                PanelType::Missing | PanelType::Broken => (0, None),
                PanelType::Normal => (pflags::SOLID, None),
                PanelType::Cracked => (pflags::SOLID | pflags::CRACKED, None),
                PanelType::Poison => (pflags::SOLID | 0x100, None),
                PanelType::Holy => (pflags::SOLID | 0x2000, None),
                PanelType::Grass => (pflags::SOLID | 0x400, None),
                PanelType::Ice => (pflags::SOLID | 0x800, None),
                PanelType::Volcano => (pflags::SOLID | 0x1000, None),
                PanelType::RoadUp => (pflags::SOLID | 0x200, Some(SlideVector { dx: 0, dy: -1, tiles: 1 })),
                PanelType::RoadDown => (pflags::SOLID | 0x200, Some(SlideVector { dx: 0, dy: 1, tiles: 1 })),
                PanelType::RoadLeft => (pflags::SOLID | 0x200, Some(SlideVector { dx: -1, dy: 0, tiles: 1 })),
                PanelType::RoadRight => (pflags::SOLID | 0x200, Some(SlideVector { dx: 1, dy: 0, tiles: 1 })),
            };
            // Every panel type is on the field (the step sword looks for
            // this bit).
            PanelTypeRule { flags: flags | ON_FIELD, road_slide }
        })
        .collect();
    // Steps: onto a free panel of one's own side, solid unless floor-free.
    let own_side = |s: usize| PanelCondition {
        require: if s == 1 { pflags::ALLIANCE_1 } else { 0 },
        forbid: pflags::OCCUPIED | if s == 0 { pflags::ALLIANCE_1 } else { 0 },
    };
    let solid = |c: PanelCondition| PanelCondition { require: c.require | pflags::SOLID, ..c };
    let step = StepRuleSet { grounded: [solid(own_side(0)), solid(own_side(1))], floor_free: [own_side(0), own_side(1)] };
    let any_side = PanelCondition { require: 0, forbid: pflags::OCCUPIED };
    let mut start_visible = [[false; 8]; 5];
    for row in &mut start_visible[1..4] {
        row[1..7].fill(true);
    }
    let mut front_edges = [[false; 8]; 5];
    front_edges[3][1..7].fill(true);

    let sides = PanelCondition { require: 0, forbid: 0 };
    Rules {
        element_weakness: {
            // Fire is weak to aqua, aqua to elec, elec to wood, wood to
            // fire.
            let mut w = [[0; 6]; 6];
            (w[1][2], w[2][3], w[3][4], w[4][1]) = (1, 1, 1, 1);
            w
        },
        family_elements: ChipFamily::ALL.map(|f| {
            SecondaryElements(match f {
                ChipFamily::Sword => SecondaryElements::SWORD,
                ChipFamily::Cursor => SecondaryElements::CURSOR,
                ChipFamily::Wind => SecondaryElements::WIND,
                ChipFamily::Break => SecondaryElements::BREAK,
                _ => 0,
            })
        }),
        collision_types,
        field_regions: vec![
            sides,
            PanelCondition { require: 0, forbid: pflags::ALLIANCE_1 },
            PanelCondition { require: pflags::ALLIANCE_1, forbid: 0 },
            PanelCondition { require: pflags::SOLID, forbid: 0 },
        ],
        panels: PanelRules {
            types,
            start_visible,
            front_edges,
            step,
            dash_step: step,
            any_side_step: StepRuleSet { grounded: [solid(any_side); 2], floor_free: [any_side; 2] },
        },
        stages: stages(),
        holding_banners: vec![BannerId(0x24)],
        status_effects: vec![[StatusEffect { requests: 0, duration: 60, timer: StatusTimer::Paralyze }; 16]; 6],
        hp_bug_periods: [0, 60, 50, 40, 30, 20, 10, 5],
        weapons: vec![WeaponRoutine { charge_ticks: [120, 100, 80, 60, 50] }; 0x30],
        buster_recovery: vec![[5, 10, 15, 20, 25, 30], [4, 8, 12, 16, 20, 24], [3, 6, 9, 12, 15, 18], [2, 4, 6, 8, 10, 12], [1, 2, 3, 4, 5, 6]],
        sp_deletion_times: vec![0x2000, 0x4000],
        push_vectors: [
            SlideVector { dx: 1, dy: 0, tiles: 6 },
            SlideVector { dx: -1, dy: 0, tiles: 6 },
            SlideVector { dx: 1, dy: 0, tiles: 1 },
            SlideVector { dx: -1, dy: 0, tiles: 1 },
            SlideVector::NONE,
            SlideVector { dx: 0, dy: -1, tiles: 1 },
            SlideVector { dx: 0, dy: 1, tiles: 1 },
            SlideVector { dx: 1, dy: 0, tiles: 2 },
            SlideVector { dx: -1, dy: 0, tiles: 2 },
            SlideVector::NONE,
        ],
        obstacle_push_vectors: [
            SlideVector { dx: -1, dy: 0, tiles: 6 },
            SlideVector { dx: 1, dy: 0, tiles: 6 },
            SlideVector { dx: -1, dy: 0, tiles: 1 },
            SlideVector { dx: 1, dy: 0, tiles: 1 },
        ],
        ice_vectors: [
            SlideVector::NONE,
            SlideVector { dx: 0, dy: -1, tiles: 1 },
            SlideVector { dx: 0, dy: 1, tiles: 1 },
            SlideVector { dx: -1, dy: 0, tiles: 1 },
            SlideVector { dx: 1, dy: 0, tiles: 1 },
            SlideVector::NONE,
        ],
        bubble_bob: std::array::from_fn(|i| [0, 1, 2, 3, 3, 2, 1, 0][i % 8] * if i < 16 { 1 } else { -1 }),
        // A triangle wave: 256 at a quarter turn, -256 at three quarters,
        // over a turn and a half.
        sine: (0..384)
            .map(|i: i16| {
                let t = i % 256;
                if t < 64 { t * 4 } else if t < 192 { 512 - t * 4 } else { t * 4 - 1024 }
            })
            .collect(),
        lockon: Lockon {
            searches: vec![LockonSearch { mode: 1, offsets: vec![PanelOffset { dx: -1, dy: 0 }], prefers_middle_row: false }],
            column_shifts: vec![-1, -2],
        },
        custom_screen: custom_screen_layout(),
    }
}

/// The custom screen's grid: five chip slots on top, five below, OK at
/// the top row's right end and a special button under it; the last two
/// bottom slots start hidden. A neighbour that is missing is looked for
/// along its row, which wraps through OK (the top row) or the button
/// under it (the bottom row).
pub fn custom_screen_layout() -> CustomScreenLayout {
    let slot = |kind, vertical, left, right| SlotLayout { kind, vertical, left, right };
    let chip = TemplateSlot::ChipPosition;
    let hidden = TemplateSlot::Hidden;
    CustomScreenLayout {
        slots: [
            slot(chip, 5, 10, 1),
            slot(chip, 6, 0, 2),
            slot(chip, 7, 1, 3),
            slot(chip, 8, 2, 4),
            slot(chip, 9, 3, 10),
            slot(chip, 0, 11, 6),
            slot(chip, 1, 5, 7),
            slot(chip, 2, 6, 8),
            slot(hidden, 3, 7, 9),
            slot(hidden, 4, 8, 11),
            slot(TemplateSlot::Ok, 11, 4, 0),
            slot(hidden, 10, 9, 5),
        ],
        left_scan_top: vec![4, 3, 2, 1, 0, 10],
        left_scan_bottom: vec![9, 8, 7, 6, 5, 11, 10],
        right_scan_top: vec![0, 1, 2, 3, 4, 10],
        right_scan_bottom: vec![5, 6, 7, 8, 9, 11, 10],
        left_scan_start: [5, 4, 3, 2, 1, 5, 4, 3, 2, 1, 0, 0],
        right_scan_start: [1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 0, 0],
    }
}

fn stages() -> Stages {
    let navi = |alliance, x| ActorEntry { kind: ActorKind::Navi, alliance, x, y: 2 };
    let rock = |x, y| ActorEntry { kind: ActorKind::Rock { variant: 1 }, alliance: 0, x, y };
    let settings = |actors| BattleSettings {
        layout: 0,
        music: 0x16,
        mode: 0,
        background: 0,
        battle_number: 0,
        panel_pattern: 0x38,
        effects: effects::LINK,
        actors,
    };
    Stages {
        settings: vec![settings(TWO_NAVIS), settings(TWO_NAVIS_SIDE0_FIRST), settings(NAVIS_AND_ROCKS)],
        actor_lists: vec![
            ActorList { original_address: 1, entries: vec![navi(1, 5), navi(0, 2)] },
            ActorList { original_address: 2, entries: vec![navi(0, 2), navi(1, 5)] },
            ActorList { original_address: 3, entries: vec![navi(0, 1), navi(1, 6), rock(3, 3), rock(4, 1)] },
        ],
    }
}

fn objects() -> ObjectData {
    let gun = |id: u8| AttachmentKind {
        id,
        sprite: SpriteId { category: 0x0C, index: 0x01 },
        palette: id.saturating_sub(1),
        lift: 0,
        attach_point: (id != 0).then_some(3),
    };
    let rock = |id, anim, element| RockKind { id, anim, hp: 100, element, debris_palette: id, break_sound: 0x118, name_id: 0x100 };
    // The buster's muzzle flash and arm.
    let plain = |id, index| AttachmentKind { id, sprite: SpriteId { category: 0x0C, index }, palette: 0, lift: 0, attach_point: None };
    ObjectData {
        // Attachments are numbered without gaps: the swords' blade (7),
        // fillers up to the bee chip's hive (0x28), then what the thrown
        // chips hold (a seed at 0x24, the flash bomb at 0x2E).
        attachments: (0..5)
            .map(gun)
            .chain([plain(5, 0x06), plain(6, 0x03), blade_kind()])
            .chain((8..0x28).map(|id| if id == 0x24 { plain(id, 0x02) } else { plain(id, 0x06) }))
            .chain([plain(0x28, 0x5E)])
            .chain((0x29..0x2F).map(|id| plain(id, 0x02)))
            .collect(),
        rocks: vec![rock(0, 1, Element::Null), rock(1, 1, Element::Null), rock(2, 2, Element::Null), rock(3, 2, Element::Aqua)],
        absorbed_sprites: vec![SpriteId { category: 0x10, index: 0 }; 6],
        body_overlays: Vec::new(),
        sun_beam_looks: vec![SpriteId { category: 0x0C, index: 0x10 }, SpriteId { category: 0x0C, index: 0x11 }],
        kinds: kinds(),
    }
}

/// The swords' blade (attachment 7), held at the gun's point.
fn blade_kind() -> AttachmentKind {
    AttachmentKind { id: 7, sprite: SpriteId { category: 0x0C, index: 0x08 }, palette: 0, lift: 0, attach_point: Some(3) }
}

fn regions() -> Vec<Vec<PanelOffset>> {
    let p = |dx, dy| PanelOffset { dx, dy };
    let mut v = vec![vec![p(0, 0)]; 0x2F];
    v[0] = Vec::new();
    v[2] = vec![p(0, 0), p(1, 0)];
    v[3] = vec![p(1, 0)];
    v[4] = vec![p(0, 0), p(0, -1), p(0, 1)];
    v[0x11] = vec![p(0, 0), p(0, -1), p(0, 1), p(1, 0), p(1, -1), p(1, 1)];
    v
}

/// Animation timing for the sprites the tests' battles show.
fn animations() -> Animations {
    const LAST: u8 = crate::object::sprite::FRAME_LAST;
    const LOOP: u8 = crate::object::sprite::FRAME_LOOP;
    let f = |duration, flags| AnimFrame { duration, flags };
    let once = |ticks| vec![f(ticks, LAST)];
    let mut navi = vec![once(4); 0x20];
    navi[0] = vec![f(16, LAST | LOOP)];
    navi[3] = vec![f(2, 0), f(3, LAST)];
    navi[4] = vec![f(1, 0), f(2, LAST)];
    navi[0x0A] = vec![f(6, 0), f(30, LAST | LOOP)];
    let mut sprites = std::collections::BTreeMap::new();
    sprites.insert(NAVI_SPRITE, navi);
    // The gun: out, firing, away.
    sprites.insert(SpriteId { category: 0x0C, index: 0x01 }, vec![vec![f(3, 0), f(3, LAST)], vec![f(2, 0), f(2, LAST | LOOP)], once(4)]);
    for index in [0x10, 0x11] {
        sprites.insert(SpriteId { category: 0x0C, index }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    }
    // Rocks: rising, then standing.
    sprites.insert(
        SpriteId { category: 0x10, index: 0 },
        vec![vec![f(3, 0), f(3, 0), f(3, LAST)], vec![f(30, LAST | LOOP)], vec![f(30, LAST | LOOP)]],
    );
    sprites.insert(SpriteId { category: 0x10, index: 1 }, vec![once(6), once(6), once(6), once(6)]);
    // The eraser navi (standing, appearing, leaving, raising, slashing), its
    // marks and its slash.
    let mut eraser = vec![once(4); 0x13];
    eraser[0] = vec![f(8, 0), f(8, LAST | LOOP)];
    eraser[0x12] = vec![f(4, 0), f(40, LAST)];
    sprites.insert(SpriteId { category: 8, index: 4 }, eraser);
    sprites.insert(SpriteId { category: 0x10, index: 0x50 }, vec![vec![f(4, 0), f(4, LAST | LOOP)]]);
    sprites.insert(SpriteId { category: 0x10, index: 0x51 }, vec![vec![f(3, 0), f(3, LAST | LOOP)]; 3]);
    // The buster's muzzle flash, and its arm (by form).
    sprites.insert(SpriteId { category: 0x0C, index: 0x06 }, vec![vec![f(2, 0), f(2, LAST)]]);
    sprites.insert(SpriteId { category: 0x0C, index: 0x03 }, vec![vec![f(30, LAST | LOOP)]; 0x19]);
    // The junk ball: rolling, bursting.
    let mut junk = vec![once(4); 0x1B];
    junk[0x19] = vec![f(4, 0), f(4, LAST | LOOP)];
    junk[0x1A] = vec![f(10, 0), f(20, LAST)];
    sprites.insert(SpriteId { category: 8, index: 0x0A }, junk);
    // The Reflector's shield (up, fading, by look) and its wave.
    sprites.insert(SpriteId { category: 0x0C, index: 0x1B }, vec![vec![f(8, LAST | LOOP)], vec![f(7, 0), f(7, LAST)]]);
    sprites.insert(SpriteId { category: 0x14, index: 0x04 }, vec![vec![f(2, 0), f(3, LAST)]]);
    // The swords' blade, swinging.
    sprites.insert(blade_kind().sprite, vec![vec![f(3, 0), f(3, 0), f(8, LAST)]]);
    // The grab shot: falling, landing.
    sprites.insert(SpriteId { category: 0x0C, index: 0x13 }, vec![vec![f(8, LAST | LOOP)], vec![f(3, 0), f(3, LAST)]]);
    // The hive (closed, open), a bee, and a dragon's animations.
    sprites.insert(SpriteId { category: 0x0C, index: 0x5E }, vec![vec![f(30, LAST | LOOP)], vec![f(4, 0), f(30, LAST)]]);
    sprites.insert(SpriteId { category: 0x10, index: 0x31 }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    sprites.insert(SpriteId { category: 0x04, index: 0x10 }, vec![vec![f(6, LAST | LOOP)]; 8]);
    // Effects and sparks.
    sprites.insert(SpriteId { category: 0x14, index: 0 }, vec![vec![f(3, 0), f(3, 0), f(3, LAST)]]);
    sprites.insert(SpriteId { category: 0x14, index: 1 }, vec![vec![f(2, 0), f(2, LAST)]]);
    Animations { sprites }
}
