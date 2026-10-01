//! A small hand-authored content set for tests and examples.
//!
//! Everything here is made up: a MegaMan-like navi and its base form,
//! a few chips that use GunDelSol, two dimming chips (one grabs a column)
//! and ten navi chips, rocks,
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
use crate::setup::{ActorEntry, ActorKind, ActorList, ActorListId, StageSettings, effects};
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
/// Instant chips (action 0x1C, ids 0x40 and up): one that fills the custom
/// gauge (FullCust's effect).
pub const FULL_GAUGE: ChipId = 0x40;
/// Instant chips: a plus chip used on its own (subtype 3, the plus chips'
/// effect by number) and one that syncs the navi (subtype 13).
pub const PLUS: ChipId = 0x41;
pub const SYNC: ChipId = 0x43;
/// Instant chips whose effects spawn objects: a boomerang (subtype 1),
/// lances (4), fists (8), worms (12), flame hooks (14), a falling fist
/// (19) and a golem (21).
pub const BOOMERANG: ChipId = 0x44;
pub const LANCE: ChipId = 0x45;
pub const FIST: ChipId = 0x46;
pub const WORM: ChipId = 0x47;
pub const FLAME_HOOK: ChipId = 0x48;
pub const JUSTICE: ChipId = 0x49;
pub const GOLEM: ChipId = 0x4A;
/// Standard chip actions (ids 0x100 and up): a CrakShot (action 0x22,
/// subtype 0: the panel ahead).
pub const CRACK: ChipId = 0x100;
/// A Reflector (action 0x2B, subtype 0): guards for 30 ticks.
pub const MIRROR: ChipId = 0x101;
/// A recovery chip (action 0x20): heals 40 HP.
pub const MEND: ChipId = 0x102;
/// The thrown chips (action 0x12): a bomb (subtype 0), a seed that
/// poisons panels (subtype 12), a flash bomb (subtype 14) and a bug bomb
/// (subtype 7).
pub const BOMB: ChipId = 0x103;
pub const SEED: ChipId = 0x104;
pub const FLASH: ChipId = 0x105;
pub const BUG: ChipId = 0x106;
/// A chip that sends bees (action 0x39, RskyHny's).
pub const BEES: ChipId = 0x107;
/// A chip that sends an elec dragon (action 0x51, subtype 1).
pub const DRAGON: ChipId = 0x108;
/// A sword (action 0x13, subtype 1: a column of three panels ahead).
pub const BLADE: ChipId = 0x109;
/// A step sword (the same, with its first parameter set: it steps two
/// panels ahead first).
pub const STEP_BLADE: ChipId = 0x10a;
/// A strike at stunned or grounded opponents (action 0x49, subtype 2).
pub const STUN_BLADE: ChipId = 0x10b;
/// A dimming chip (action 0x15, subtype 6) that places a rock (variant 1)
/// in front of its user.
pub const CUBE: ChipId = 0x08;
/// A trap chip (action 0x15, subtype 20, Param1 3: no object).
pub const TRAP: ChipId = 0x09;
/// Navi-changing dimming chips (action 0x15, subtype 38): the buster and
/// shoes boost (Param1 0), and a new charged shot (Param1 2: weapon
/// routine 1).
pub const BOOST: ChipId = 0x0A;
pub const ARM: ChipId = 0x0B;
/// A dimming chip (action 0x15, subtype 25) that slows the custom gauge.
pub const SLOW_GAUGE: ChipId = 0x0C;
// Dimming chip subtypes 10, 11, 14 and ElemTrap's (20).
/// An element trap (action 0x15, subtype 20, Param1 0: the trap object).
pub const ELEM_TRAP: ChipId = 0x30;
/// Time bombs (action 0x15, subtype 10): variant 0 and 1.
pub const TIME_BOMB: ChipId = 0x31;
pub const TIME_BOMB_PLUS: ChipId = 0x32;
/// A mine (action 0x15, subtype 11).
pub const MINE: ChipId = 0x33;
// Navi chips.
/// A navi chip (action 0x1B, subtype 16: the elements navi).
pub const ELEMENTS: ChipId = 0x110;
/// A navi chip (action 0x1B, subtype 7: the water navi).
pub const SPOUT: ChipId = 0x111;
/// A navi chip (action 0x1B, subtype 2: the heat navi).
pub const HEAT: ChipId = 0x112;
/// A navi chip (action 0x1B, subtype 3: the elec navi).
pub const ELEC: ChipId = 0x113;
/// A navi chip (action 0x1B, subtype 4: the slash navi).
pub const SLASH: ChipId = 0x114;
/// A navi chip (action 0x1B, subtype 6: the charge navi).
pub const CHARGE: ChipId = 0x115;
/// A navi chip (action 0x1B, subtype 8: the tomahawk navi).
pub const TOMAHAWK: ChipId = 0x116;
/// A navi chip (action 0x1B, subtype 9: the tengu navi).
pub const TENGU: ChipId = 0x117;
/// A navi chip (action 0x1B, subtype 12: the blast navi).
pub const BLAST: ChipId = 0x118;
/// A navi chip (action 0x1B, subtype 26: the shooting navi, Bass's).
pub const BASS: ChipId = 0x119;
/// A navi chip (action 0x1B, subtype 25: the sun-and-moon navi).
pub const SUN_MOON: ChipId = 0x11a;

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

/// The content set, shared (defined once per process: the define phase
/// runs every module).
pub fn content() -> Arc<Content> {
    shared().0.clone()
}

/// The content set and its hash, made once.
fn shared() -> &'static (Arc<Content>, crate::content::ContentHash) {
    static SHARED: std::sync::OnceLock<(Arc<Content>, crate::content::ContentHash)> = std::sync::OnceLock::new();
    SHARED.get_or_init(|| {
        let c = make().defined();
        let hash = c.hash();
        (Arc::new(c), hash)
    })
}

/// A round on this content with battle settings `settings`, both navis
/// with `stats`: RNG seed 1, side 0's perspective, no set score, no
/// folders.
pub fn round_setup(settings: u8, stats: crate::setup::NaviStats) -> crate::setup::RoundSetup {
    let (content, hash) = shared();
    crate::setup::RoundSetup {
        content: *hash,
        settings: crate::setup::BattleSettings::on(content, content.stage_numbered(settings)),
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
    crate::setup::NaviStats { hp, max_hp: hp, max_base_hp: hp, ..megaman_on(&content()) }
}

/// Stats with nothing of note but MegaMan in his base form, by `content`'s
/// handles.
pub fn megaman_on(content: &Content) -> crate::setup::NaviStats {
    let base = content.form_numbered(crate::setup::Form::NONE);
    crate::setup::NaviStats {
        navi: content.navi_numbered(crate::setup::Navi::MEGAMAN),
        form: base,
        starting_form: base,
        ..Default::default()
    }
}

/// The chip with number `id` in the content (its handle there).
pub fn chip_in(content: &Content, id: ChipId) -> bn6_content_api::ChipHandle {
    content.chip_numbered(id).unwrap_or_else(|| panic!("chip {id:#x} is not in the test content"))
}

/// The chip with number `id` in the shared test content.
pub fn chip_handle(id: ChipId) -> bn6_content_api::ChipHandle {
    chip_in(&content(), id)
}

/// The chip the shared test content defines as `key`.
pub fn defined_chip(key: &str) -> bn6_content_api::ChipHandle {
    content().defs.chip_by_key(key).unwrap_or_else(|| panic!("the test content defines no chip {key:?}"))
}

/// Weapon routine `n` in the content (none for 0xFF).
pub fn weapon_in(content: &Content, n: u8) -> Option<bn6_content_api::WeaponHandle> {
    (n != 0xFF).then(|| content.weapon_numbered(n))
}

/// Weapon routine `n` in the shared test content (none for 0xFF).
pub fn weapon(n: u8) -> Option<bn6_content_api::WeaponHandle> {
    weapon_in(&content(), n)
}

/// The test pack's ticker chips and tick shot weapon (`with_test_pack`),
/// by key: setups reach them by handle.
pub const TICKER_1: &str = "test/ticker1";
pub const TICKER_2: &str = "test/ticker2";
pub const TICKER_3: &str = "test/ticker3";
/// BN6's AreaGrab and PanelGrab (chips/areagrab, chips/panlgrab): dimming
/// chips content defines, which grab a column and a panel.
pub const AREA_GRAB: &str = "areagrab";
pub const PANEL_GRAB: &str = "panlgrab";
/// BN6's BusterUp, Atk+10 and Navi+20 (chips/busterup, chips/atk-10,
/// chips/navi-20): instant chips content defines.
pub const BUSTER_UP: &str = "busterup";
pub const ATTACK_10: &str = "atk-10";
pub const NAVI_20: &str = "navi-20";
pub const TICK_SHOT: &str = "test/tick-shot";

/// The content model v2 test pack (crates/bn6-battle/testdata/pack):
/// definitions the engine's tests run.
const TEST_PACK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/pack");

/// Every `.luau` module under `dir`, by path without `.luau`.
pub fn modules_under(dir: &str) -> std::collections::BTreeMap<String, String> {
    fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut std::collections::BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if let Some(name) = path.to_str().and_then(|s| s.strip_suffix(".luau"))
                && !name.ends_with(".d")
            {
                let rel = std::path::Path::new(name).strip_prefix(root).expect("under the root");
                let key = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                out.insert(key, std::fs::read_to_string(&path).expect("a module"));
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(std::path::Path::new(dir), std::path::Path::new(dir), &mut out);
    out
}

/// The content set with the test pack's modules (under `test/`), defined.
/// Its kind, actions, chips and weapon run by handle, with no numbers:
/// hands and navi stats hold them (`TICKER_1`, `TICKER_2`, `TICK_SHOT`).
pub fn with_test_pack() -> Content {
    let mut c = make();
    for (path, source) in modules_under(TEST_PACK) {
        c.scripts.modules.insert(format!("test/{path}"), source);
    }
    c.define().unwrap_or_else(|e| panic!("content error: {e}"));
    c
}

/// The content set with stage `stage`'s record changed by `f` (its music,
/// say).
pub fn restaged(stage: u8, f: impl FnOnce(&mut StageSettings)) -> Content {
    let mut c = build();
    let h = c.stage_numbered(stage);
    f(&mut c.defs.stages[h.index()].record);
    c
}

/// The content set, defined (a copy: tests change it; one that changes
/// its scripts defines it again, `Content::define`).
pub fn build() -> Content {
    (*content()).clone()
}

/// The content set, not yet defined.
fn make() -> Content {
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
        assets: assets(),
        defs: Default::default(),
    }
}

/// A synthetic asset index for `modules`: every name they give an
/// `asset.<kind>("...")` call, each a made-up asset of its own (nothing
/// ROM-derived; for tests that load modules the test content doesn't
/// list).
pub fn asset_names_used(modules: &std::collections::BTreeMap<String, String>) -> bn6_content_api::AssetNames {
    use bn6_content_api::AssetKind;
    let mut a = bn6_content_api::AssetNames::default();
    let mut n = 0u16;
    for source in modules.values() {
        for kind in AssetKind::ALL {
            for piece in source.split(&format!("asset.{kind}(")).skip(1) {
                let piece = piece.trim_start();
                let Some(quote) = piece.chars().next().filter(|c| matches!(c, '"' | '\'')) else { continue };
                let Some(name) = piece[1..].split(quote).next() else { continue };
                n += 1;
                let id = n;
                match kind {
                    AssetKind::Sprite => {
                        a.sprites.entry(name.into()).or_insert(SpriteId { category: 0x7F, index: id as u8 });
                    }
                    AssetKind::Sound => {
                        a.sounds.entry(name.into()).or_insert(id);
                    }
                    AssetKind::Banner => {
                        a.banners.entry(name.into()).or_insert(id as u8);
                    }
                    AssetKind::Background => {
                        a.backgrounds.entry(name.into()).or_insert(id as u8);
                    }
                    AssetKind::Mugshot => {
                        a.mugshots.entry(name.into()).or_insert(id as u8);
                    }
                }
            }
        }
    }
    a
}

/// The asset names the test content has: the BN6 names its modules use
/// (with BN6's numbers), a few made-up ones for the test pack, and a
/// placeholder.
fn assets() -> bn6_content_api::AssetNames {
    let mut a = bn6_content_api::AssetNames::default();
    let sprite = |c, i| SpriteId { category: c, index: i };
    for (name, id) in [
        ("test-burst", sprite(0x14, 0)),
        ("test-spark", sprite(0x14, 1)),
        ("sprite-14-02", sprite(0x14, 2)),
        ("bomb", sprite(0x0C, 0x02)),
        ("flash-bomb", sprite(0x10, 0x52)),
        ("black-bomb", sprite(0x0C, 0x24)),
        ("energy-burst", sprite(0x14, 0x12)),
        ("sword", sprite(0x0C, 0x00)),
        ("explosion", sprite(0x14, 0x00)),
        ("rising-bubble", sprite(0x14, 0x02)),
        ("puff", sprite(0x14, 0x0D)),
        ("grab-shot", sprite(0x0C, 0x13)),
        ("copy-mark", sprite(0x14, 0x05)),
        ("fire-sword", sprite(0x0C, 0x36)),
        ("aqua-sword", sprite(0x0C, 0x37)),
        ("elec-sword", sprite(0x0C, 0x38)),
        ("sword-slash", sprite(0x0C, 0x14)),
        ("big-slash", sprite(0x0C, 0x15)),
        ("cross-slash", sprite(0x10, 0x41)),
        ("reflected-shot", sprite(0x14, 0x04)),
        ("eraseman", sprite(0x08, 0x04)),
        ("buster-up", sprite(0x14, 0x1B)),
        ("erase-mark", sprite(0x10, 0x50)),
        ("erase-beam", sprite(0x10, 0x51)),
        ("impact", sprite(0x14, 0x01)),
        ("bat-impact", sprite(0x14, 0x07)),
        ("shot-impact", sprite(0x14, 0x0C)),
        ("shell-burst", sprite(0x14, 0x11)),
        ("beast-shot", sprite(0x0C, 0x21)),
        ("bow", sprite(0x0C, 0x2A)),
        ("lil-boiler", sprite(0x04, 0x0D)),
        ("voodoo-doll", sprite(0x0C, 0x34)),
    ] {
        a.sprites.insert(name.into(), id);
    }
    for (name, id) in [
        ("test-tick", 0x1A6),
        ("minibomb-throw", 0xB2),
        ("hit-bomb-1", 0x70),
        ("panel-poison", 0x90),
        ("freeze", 0x118),
        ("elmnt-man-3", 0x11B),
        ("flash", 0x1BD),
        ("bug-bomb-land", 0x115),
        ("land", 0xC0),
        ("burst", 0xC3),
        ("energy-burst", 0xBB),
        ("sword-swing", 0xB0),
        ("big-sword-swing", 0xCE),
        ("grab-shot", 0xA2),
        ("grab-shot-2", 0xA1),
        ("appear", 0x94),
        ("erase-man", 0x10E),
        ("erase-man-2", 0xBA),
        ("hub", 0x119),
        ("bonus", 0x157),
        ("twang", 0x18A),
        ("boiler-erupt", 0x184),
        ("boiler-steam", 0x185),
        ("err-select-91", 0x91),
        ("hit-bomb-0", 0x6F),
    ] {
        a.sounds.insert(name.into(), id);
    }
    a
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
                ("chips/eraseman/mark", "chips/eraseman/mark"),
                ("chips/eraseman/beam", "chips/eraseman/beam"),
                ("chips/eraseman/navi", "chips/eraseman/navi"),
                ("chips/0ec-eraseman/chip", "chips/0ec-eraseman/chip"),
                ("lib/dimming", "lib/dimming"),
                ("lib/grab/shot", "lib/grab/shot"),
                ("lib/grab/controller", "lib/grab/controller"),
                ("chips/areagrab/chip", "chips/areagrab/chip"),
                ("chips/panlgrab/chip", "chips/panlgrab/chip"),
                ("objects/dust-ball/dust_ball", "objects/dust-ball/dust_ball"),
                ("objects/falling-rock/falling_rock", "objects/falling-rock/falling_rock"),
                ("objects/rock-chip/rock_chip", "objects/rock-chip/rock_chip"),
                ("objects/projectile/projectile", "objects/projectile/projectile"),
                ("objects/projectile/variants", "objects/projectile/variants"),
                ("objects/flying-shot/flying_shot", "objects/flying-shot/flying_shot"),
                ("lib/buster", "lib/buster"),
                ("lib/weapon", "lib/weapon"),
                ("objects/element-pillar/element_pillar", "objects/element-pillar/element_pillar"),
                ("objects/aqua-surge/aqua_surge", "objects/aqua-surge/aqua_surge"),
                ("objects/whirlwind/whirlwind", "objects/whirlwind/whirlwind"),
                ("objects/dash-hit/dash_hit", "objects/dash-hit/dash_hit"),
                ("objects/erase-drop/erase_drop", "objects/erase-drop/erase_drop"),
                ("objects/lunge-slash/lunge_slash", "objects/lunge-slash/lunge_slash"),
                ("objects/hit-flash/hit_flash", "objects/hit-flash/hit_flash"),
                ("objects/charge-wave/charge_wave", "objects/charge-wave/charge_wave"),
                ("objects/junk-shot/junk_shot", "objects/junk-shot/junk_shot"),
                ("objects/absorbed-obstacle/absorbed_obstacle", "objects/absorbed-obstacle/absorbed_obstacle"),
                ("chips/0ae-fullcust/chip", "chips/0ae-fullcust/chip"),
                ("lib/instant/plus", "lib/instant/plus"),
                ("chips/0c0-atk-10/chip", "chips/0c0-atk-10/chip"),
                ("chips/atk-10/chip", "chips/atk-10/chip"),
                ("chips/navi-20/chip", "chips/navi-20/chip"),
                ("chips/busterup/chip", "chips/busterup/chip"),
                ("chips/11d-synctrgr/chip", "chips/11d-synctrgr/chip"),
                ("objects/boomerang/boomerang", "objects/boomerang/boomerang"),
                ("objects/lance/lance", "objects/lance/lance"),
                ("objects/fire-hit/fire_hit", "objects/fire-hit/fire_hit"),
                ("objects/sand-worm/sand_worm", "objects/sand-worm/sand_worm"),
                ("objects/sand-hole/sand_hole", "objects/sand-hole/sand_hole"),
                ("objects/sand-spray/sand_spray", "objects/sand-spray/sand_spray"),
                ("objects/flame-hook/flame_hook", "objects/flame-hook/flame_hook"),
                ("objects/flame-hook-fire/flame_hook_fire", "objects/flame-hook-fire/flame_hook_fire"),
                ("objects/justice-one/justice_one", "objects/justice-one/justice_one"),
                ("objects/golem/golem", "objects/golem/golem"),
                ("lib/element", "lib/element"),
                ("lib/projectile", "lib/projectile"),
                ("lib/sword", "lib/sword"),
                ("objects/gust/gust", "objects/gust/gust"),
                ("objects/sword-wave/sword_wave", "objects/sword-wave/sword_wave"),
                ("objects/erase-ray/erase_ray", "objects/erase-ray/erase_ray"),
                ("objects/reflector-shield/reflector_shield", "objects/reflector-shield/reflector_shield"),
                ("objects/reflected-shot/reflected_shot", "objects/reflected-shot/reflected_shot"),
                ("chips/008-mirror/chip", "chips/083-rflectr1/chip"),
                ("chips/009-mend/chip", "chips/09a-recov10/chip"),
                ("lib/regions", "lib/regions"),
                ("lib/effects", "lib/effects"),
                ("lib/sparks", "lib/sparks"),
                ("rules/collision", "rules/collision"),
                ("lib/trajectory", "lib/trajectory"),
                ("lib/hp", "lib/hp"),
                // The bombs and seeds (content model v2): the chips' own
                // actions, which the test chips reach through the numbered
                // registration's module.
                ("lib/bombs/throw", "lib/bombs/throw"),
                ("lib/bombs/bomb", "lib/bombs/bomb"),
                ("lib/bombs/slash", "lib/bombs/slash"),
                ("lib/bombs/seed", "lib/bombs/seed"),
                ("chips/minibomb/chip", "chips/minibomb/chip"),
                ("chips/bigbomb/chip", "chips/bigbomb/chip"),
                ("chips/energbom/chips", "chips/energbom/chips"),
                ("chips/energbom/burst", "chips/energbom/burst"),
                ("chips/flshbom/chips", "chips/flshbom/chips"),
                ("chips/flshbom/bomb", "chips/flshbom/bomb"),
                ("chips/blkbomb/chip", "chips/blkbomb/chip"),
                ("chips/blkbomb/bomb", "chips/blkbomb/bomb"),
                ("chips/bugbomb/chip", "chips/bugbomb/chip"),
                ("chips/bugbomb/bomb", "chips/bugbomb/bomb"),
                ("chips/grasseed/chip", "chips/grasseed/chip"),
                ("chips/iceseed/chip", "chips/iceseed/chip"),
                ("chips/poisseed/chip", "chips/poisseed/chip"),
                ("chips/lilbolr/chips", "chips/lilbolr/chips"),
                ("chips/lilbolr/boiler", "chips/lilbolr/boiler"),
                ("chips/lilbolr/layer", "chips/lilbolr/layer"),
                ("chips/vdoll/chip", "chips/vdoll/chip"),
                ("chips/vdoll/doll", "chips/vdoll/doll"),
                ("chips/vdoll/curse", "chips/vdoll/curse"),
                ("chips/vdoll/sparkles", "chips/vdoll/sparkles"),
                ("chips/00a-bomb/chip", "chips/036-minibomb/chip"),
                ("chips/00e-bees/chip", "chips/025-rskyhny1/chip"),
                ("objects/honey-bee/honey_bee", "objects/honey-bee/honey_bee"),
                ("chips/00f-dragon/chip", "chips/02e-heatdrgn/chip"),
                ("lib/dragon", "lib/dragon"),
                ("objects/dragon-head/dragon_head", "objects/dragon-head/dragon_head"),
                ("objects/dragon-body/dragon_body", "objects/dragon-body/dragon_body"),
                // The swords (content model v2): the chips' own slashes and
                // strikes, which the test chips reach through the numbered
                // registrations' modules.
                ("lib/swords/parts", "lib/swords/parts"),
                ("lib/swords/slash", "lib/swords/slash"),
                ("lib/swords/strike", "lib/swords/strike"),
                ("chips/sword/chip", "chips/sword/chip"),
                ("chips/wideswrd/chip", "chips/wideswrd/chip"),
                ("chips/longswrd/chip", "chips/longswrd/chip"),
                ("chips/wideblde/chip", "chips/wideblde/chip"),
                ("chips/longblde/chip", "chips/longblde/chip"),
                ("chips/lifesrd/chip", "chips/lifesrd/chip"),
                ("chips/drksword/chip", "chips/drksword/chip"),
                ("chips/muramasa/chip", "chips/muramasa/chip"),
                ("chips/ftrsword/chip", "chips/ftrsword/chip"),
                ("chips/crosswrd/chip", "chips/crosswrd/chip"),
                ("chips/dbldream/chip", "chips/dbldream/chip"),
                ("chips/fireswrd/chip", "chips/fireswrd/chip"),
                ("chips/aquaswrd/chip", "chips/aquaswrd/chip"),
                ("chips/elecswrd/chip", "chips/elecswrd/chip"),
                ("chips/bambswrd/chip", "chips/bambswrd/chip"),
                ("chips/stepswrd/chip", "chips/stepswrd/chip"),
                ("chips/stepswrd/protoman", "chips/stepswrd/protoman"),
                ("chips/mchnswrd/chip", "chips/mchnswrd/chip"),
                ("chips/elemswrd/chip", "chips/elemswrd/chip"),
                ("chips/assnswrd/chip", "chips/assnswrd/chip"),
                ("chips/010-blade/chip", "chips/047-sword/chip"),
                ("chips/012-stunblade/chip", "chips/056-mchnswrd/chip"),
                ("objects/invisible/invisible", "objects/invisible/invisible"),
                ("objects/rock/rock", "objects/rock/rock"),
                ("objects/rock-cube/rock_cube", "objects/rock-cube/rock_cube"),
                ("objects/rock-debris/rock_debris", "objects/rock-debris/rock_debris"),
                ("objects/trap-chip/trap_chip", "objects/trap-chip/trap_chip"),
                ("objects/navi-boost/navi_boost", "objects/navi-boost/navi_boost"),
                ("objects/gauge-speed/gauge_speed", "objects/gauge-speed/gauge_speed"),
                // Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
                ("objects/rising-bubble/rising_bubble", "objects/rising-bubble/rising_bubble"),
                // Dimming chip subtypes 10, 11, 14 and ElemTrap's (20).
                ("lib/panels", "lib/panels"),
                ("objects/elem-trap/elem_trap", "objects/elem-trap/elem_trap"),
                ("objects/elem-trap-strike/elem_trap_strike", "objects/elem-trap-strike/elem_trap_strike"),
                ("objects/panel-bursts/panel_bursts", "objects/panel-bursts/panel_bursts"),
                ("objects/time-bom/time_bom", "objects/time-bom/time_bom"),
                ("objects/countdown-bomb/countdown_bomb", "objects/countdown-bomb/countdown_bomb"),
                ("objects/mine/mine", "objects/mine/mine"),
                ("objects/land-mine/land_mine", "objects/land-mine/land_mine"),
                ("chips/059-crakshot/chip", "chips/059-crakshot/chip"),
                ("objects/crack-shot/crack_shot", "objects/crack-shot/crack_shot"),
                ("objects/elmnt-man/elmnt_man", "objects/elmnt-man/elmnt_man"),
                ("objects/meteor/meteor", "objects/meteor/meteor"),
                ("objects/elmnt-ice/elmnt_ice", "objects/elmnt-ice/elmnt_ice"),
                ("objects/elmnt-bolt/elmnt_bolt", "objects/elmnt-bolt/elmnt_bolt"),
                ("objects/elmnt-vine/elmnt_vine", "objects/elmnt-vine/elmnt_vine"),
                ("objects/spout-man/spout_man", "objects/spout-man/spout_man"),
                ("objects/spout-ball/spout_ball", "objects/spout-ball/spout_ball"),
                ("objects/spout-splash/spout_splash", "objects/spout-splash/spout_splash"),
                ("objects/spout-pillar/spout_pillar", "objects/spout-pillar/spout_pillar"),
                ("objects/spout-geyser/spout_geyser", "objects/spout-geyser/spout_geyser"),
                ("objects/spout-mark/spout_mark", "objects/spout-mark/spout_mark"),
                ("objects/heat-man/heat_man", "objects/heat-man/heat_man"),
                ("objects/heat-flame/heat_flame", "objects/heat-flame/heat_flame"),
                ("objects/elec-man/elec_man", "objects/elec-man/elec_man"),
                ("objects/elec-thunder/elec_thunder", "objects/elec-thunder/elec_thunder"),
                ("objects/slash-man/slash_man", "objects/slash-man/slash_man"),
                ("objects/slash-wave/slash_wave", "objects/slash-wave/slash_wave"),
                ("objects/charge-man/charge_man", "objects/charge-man/charge_man"),
                ("objects/charge-car/charge_car", "objects/charge-car/charge_car"),
                ("objects/tomahawk-man/tomahawk_man", "objects/tomahawk-man/tomahawk_man"),
                ("objects/tengu-man/tengu_man", "objects/tengu-man/tengu_man"),
                ("objects/blast-man/blast_man", "objects/blast-man/blast_man"),
                ("objects/blast-fire/blast_fire", "objects/blast-fire/blast_fire"),
                ("objects/bass/bass", "objects/bass/bass"),
                ("objects/panel-strike/panel_strike", "objects/panel-strike/panel_strike"),
                ("objects/sun-moon/sun_moon", "objects/sun-moon/sun_moon"),
                ("objects/sun-meteor/sun_meteor", "objects/sun-meteor/sun_meteor"),
                ("objects/moon-beam/moon_beam", "objects/moon-beam/moon_beam"),
                ("objects/drill/drill", "objects/drill/drill"),
                ("objects/thunder-column/thunder_column", "objects/thunder-column/thunder_column"),
            ];
            let weapons = weapons().into_iter().map(|w| {
                let module = w.script;
                (module.clone(), module)
            });
            let modules = modules.iter().map(|&(to, from)| (to.to_string(), from.to_string())).chain(weapons);
            Scripts::new(modules.map(|(to, from)| (to, read(&from))).collect())
        })
        .clone()
}

/// MegaMan's weapon routines scripts implement: the BN6 overlay's buster
/// (and a routine that aliases it), charged shot, blank shot, GroundCross's
/// A-charge, the Beast claw, DustCross's charged shot, its obstacle
/// absorbing and the absorbed obstacle throw.
fn weapons() -> Vec<WeaponData> {
    let weapon = |id: u8, name: &str, action: Option<u8>, script: &str| WeaponData {
        id,
        name: name.into(),
        action,
        instant_chip: None,
        script: format!("navis/00-megaman/weapons/{script}"),
    };
    let mut weapons = vec![
        weapon(0x00, "Buster", Some(0x11), "00-buster/buster"),
        weapon(0x01, "Charged shot", Some(0x16), "01-charged-shot/charged_shot"),
        weapon(0x02, "Blank shot", Some(0x33), "02-blank-shot/blank_shot"),
        weapon(0x1B, "Tomahawk throw", Some(0x4E), "1b-tomahawk-throw/tomahawk_throw"),
        weapon(0x1E, "Beast claw", Some(0x52), "1e-beast-claw/beast_claw"),
        weapon(0x28, "Dust charge", Some(0x57), "28-dust-charge/dust_charge"),
        weapon(0x2A, "Absorb", Some(0x58), "2a-absorb/absorb"),
        weapon(0x2B, "Throw absorbed", None, "2b-throw-absorbed/throw_absorbed"),
        weapon(0x15, "EraseCross Beast drop", Some(0x46), "15-erase-beast-drop/erase_beast_drop"),
        weapon(0x17, "GroundCross Beast dash", Some(0x1A), "17-ground-beast-dash/ground_beast_dash"),
        weapon(0x1A, "SlashCross Beast lunge", Some(0x4C), "1a-slash-beast-lunge/slash_beast_lunge"),
        weapon(0x1C, "ChargeCross Beast wave", Some(0x4F), "1c-charge-beast-wave/charge_beast_wave"),
        weapon(0x1D, "DustCross Beast scatter", Some(0x50), "1d-dust-beast-scatter/dust_beast_scatter"),
        weapon(0x27, "ChargeCross tackle", Some(0x56), "27-charge-cross-tackle/charge_cross_tackle"),
        weapon(0x2E, "Buster", None, "00-buster/buster"),
        weapon(0x03, "Falzar Beast buster", Some(0x1E), "03-falzar-beast-buster/falzar_beast_buster"),
        weapon(0x04, "Gregar Beast buster", Some(0x1D), "04-gregar-beast-buster/gregar_beast_buster"),
        weapon(0x2C, "Beast throw absorbed", None, "2c-beast-throw-absorbed/beast_throw_absorbed"),
        weapon(0x07, "HeatCross Beast charge", Some(0x35), "07-heat-beast-charge/heat_beast_charge"),
        weapon(0x08, "SpoutCross Beast charge", Some(0x3A), "08-spout-beast-charge/spout_beast_charge"),
        weapon(0x09, "ElecCross Beast charge", Some(0x3C), "09-elec-beast-charge/elec_beast_charge"),
        weapon(0x0A, "TenguCross Beast charge", Some(0x3D), "0a-tengu-beast-charge/tengu_beast_charge"),
        weapon(0x06, "Heat charge", None, "06-heat-charge/heat_charge"),
        weapon(0x0B, "Elec charge", None, "0b-elec-charge/elec_charge"),
        weapon(0x0C, "Spout charge", None, "0c-spout-charge/spout_charge"),
        weapon(0x0F, "Tengu charge", None, "0f-tengu-charge/tengu_charge"),
        WeaponData { instant_chip: Some(0x14), ..weapon(0x10, "Tengu wind", None, "10-tengu-wind/tengu_wind") },
        weapon(0x11, "Slash A-charge", None, "11-slash-a-charge/slash_a_charge"),
        weapon(0x12, "Slash charge", Some(0x41), "12-slash-charge/slash_charge"),
        weapon(0x14, "Erase charge", Some(0x45), "14-erase-charge/erase_charge"),
        weapon(0x16, "Tomahawk charge", Some(0x4A), "16-tomahawk-charge/tomahawk_charge"),
        weapon(0x19, "Ground drill", Some(0x4D), "19-ground-drill/ground_drill"),
    ];
    // In id order, as a pack lists them.
    weapons.sort_by_key(|w| w.id);
    weapons
}

/// The object kinds scripts implement, by name (in name order, as a pack
/// lists them).
fn kinds() -> Vec<ObjectKind> {
    let kind = |name: &str, pool, index, script: &str| ObjectKind {
        name: name.into(),
        pool,
        index,
        script: script.into(),
        actor_list_entry: None,
    };
    let mut kinds = vec![
        kind("sun-beam", Pool::Effect, 0x48, "objects/sun-beam/sun_beam"),
        kind("dust-ball", Pool::Attack, 0xB0, "objects/dust-ball/dust_ball"),
        kind("element-pillar", Pool::Attack, 0x61, "objects/element-pillar/element_pillar"),
        kind("aqua-surge", Pool::Attack, 0x76, "objects/aqua-surge/aqua_surge"),
        kind("whirlwind", Pool::Attack, 0x81, "objects/whirlwind/whirlwind"),
        kind("dash-hit", Pool::Attack, 0xAF, "objects/dash-hit/dash_hit"),
        kind("erase-drop", Pool::Attack, 0xA1, "objects/erase-drop/erase_drop"),
        kind("lunge-slash", Pool::Attack, 0xB1, "objects/lunge-slash/lunge_slash"),
        kind("hit-flash", Pool::Effect, 0x73, "objects/hit-flash/hit_flash"),
        kind("charge-wave", Pool::Attack, 0xC4, "objects/charge-wave/charge_wave"),
        kind("junk-shot", Pool::Attack, 0xC5, "objects/junk-shot/junk_shot"),
        kind("absorbed-obstacle", Pool::Effect, 0x87, "objects/absorbed-obstacle/absorbed_obstacle"),
        kind("boomerang", Pool::Attack, 0x32, "objects/boomerang/boomerang"),
        kind("lance", Pool::Attack, 0x6F, "objects/lance/lance"),
        kind("fire-hit", Pool::Attack, 0x5B, "objects/fire-hit/fire_hit"),
        kind("sand-worm", Pool::Attack, 0xCB, "objects/sand-worm/sand_worm"),
        kind("sand-hole", Pool::Actor, 0x1C, "objects/sand-hole/sand_hole"),
        kind("sand-spray", Pool::Attack, 0xCC, "objects/sand-spray/sand_spray"),
        kind("flame-hook", Pool::Effect, 0x8C, "objects/flame-hook/flame_hook"),
        kind("flame-hook-fire", Pool::Attack, 0xCA, "objects/flame-hook-fire/flame_hook_fire"),
        kind("justice-one", Pool::Attack, 0xAE, "objects/justice-one/justice_one"),
        kind("golem", Pool::Effect, 0x3F, "objects/golem/golem"),
        kind("falling-rock", Pool::Attack, 0x1D, "objects/falling-rock/falling_rock"),
        kind("gust", Pool::Attack, 0x49, "objects/gust/gust"),
        kind("sword-wave", Pool::Attack, 0x96, "objects/sword-wave/sword_wave"),
        kind("erase-ray", Pool::Attack, 0x9D, "objects/erase-ray/erase_ray"),
        kind("reflector-shield", Pool::Attack, 0x2B, "objects/reflector-shield/reflector_shield"),
        kind("reflected-shot", Pool::Attack, 0x2F, "objects/reflected-shot/reflected_shot"),
        kind("honey-bee", Pool::Attack, 0x74, "objects/honey-bee/honey_bee"),
        kind("dragon-head", Pool::Attack, 0xC9, "objects/dragon-head/dragon_head"),
        kind("dragon-body", Pool::Attack, 0xC8, "objects/dragon-body/dragon_body"),
        kind("invisible", Pool::Effect, 0x5D, "objects/invisible/invisible"),
        ObjectKind { actor_list_entry: Some(8), ..kind("rock", Pool::Attack, 0x59, "objects/rock/rock") },
        kind("rock-cube", Pool::Effect, 0x37, "objects/rock-cube/rock_cube"),
        kind("rock-debris", Pool::Effect, 0x38, "objects/rock-debris/rock_debris"),
        kind("trap-chip", Pool::Effect, 0x2A, "objects/trap-chip/trap_chip"),
        kind("rock-chip", Pool::Effect, 0x09, "objects/rock-chip/rock_chip"),
        kind("navi-boost", Pool::Effect, 0x84, "objects/navi-boost/navi_boost"),
        kind("gauge-speed", Pool::Effect, 0x1C, "objects/gauge-speed/gauge_speed"),
        kind("rising-bubble", Pool::Effect, 0x14, "objects/rising-bubble/rising_bubble"),
        // Dimming chip subtypes 10, 11, 14 and ElemTrap's (20).
        kind("elem-trap", Pool::Attack, 0x4D, "objects/elem-trap/elem_trap"),
        kind("elem-trap-strike", Pool::Effect, 0x2B, "objects/elem-trap-strike/elem_trap_strike"),
        kind("panel-bursts", Pool::Effect, 0x24, "objects/panel-bursts/panel_bursts"),
        kind("time-bom", Pool::Effect, 0x27, "objects/time-bom/time_bom"),
        kind("countdown-bomb", Pool::Attack, 0x4B, "objects/countdown-bomb/countdown_bomb"),
        kind("mine", Pool::Effect, 0x29, "objects/mine/mine"),
        kind("land-mine", Pool::Attack, 0x4C, "objects/land-mine/land_mine"),
        kind("crack-shot", Pool::Attack, 0x33, "objects/crack-shot/crack_shot"),
        kind("elmnt-man", Pool::Actor, 0x10, "objects/elmnt-man/elmnt_man"),
        kind("meteor", Pool::Attack, 0x8D, "objects/meteor/meteor"),
        kind("elmnt-ice", Pool::Attack, 0x8E, "objects/elmnt-ice/elmnt_ice"),
        kind("elmnt-bolt", Pool::Attack, 0xB8, "objects/elmnt-bolt/elmnt_bolt"),
        kind("elmnt-vine", Pool::Attack, 0xB9, "objects/elmnt-vine/elmnt_vine"),
        kind("spout-man", Pool::Actor, 0x09, "objects/spout-man/spout_man"),
        kind("spout-ball", Pool::Attack, 0x22, "objects/spout-ball/spout_ball"),
        kind("spout-splash", Pool::Attack, 0x23, "objects/spout-splash/spout_splash"),
        kind("spout-pillar", Pool::Effect, 0x2D, "objects/spout-pillar/spout_pillar"),
        kind("spout-geyser", Pool::Attack, 0x17, "objects/spout-geyser/spout_geyser"),
        kind("spout-mark", Pool::Effect, 0x2E, "objects/spout-mark/spout_mark"),
        kind("heat-man", Pool::Actor, 0x07, "objects/heat-man/heat_man"),
        kind("heat-flame", Pool::Attack, 0x26, "objects/heat-flame/heat_flame"),
        kind("elec-man", Pool::Actor, 0x08, "objects/elec-man/elec_man"),
        kind("elec-thunder", Pool::Attack, 0x64, "objects/elec-thunder/elec_thunder"),
        kind("slash-man", Pool::Actor, 0x0D, "objects/slash-man/slash_man"),
        kind("slash-wave", Pool::Attack, 0x62, "objects/slash-wave/slash_wave"),
        kind("charge-man", Pool::Actor, 0x16, "objects/charge-man/charge_man"),
        kind("charge-car", Pool::Attack, 0xAC, "objects/charge-car/charge_car"),
        kind("tomahawk-man", Pool::Actor, 0x0A, "objects/tomahawk-man/tomahawk_man"),
        kind("tengu-man", Pool::Actor, 0x0C, "objects/tengu-man/tengu_man"),
        kind("blast-man", Pool::Actor, 0x06, "objects/blast-man/blast_man"),
        kind("blast-fire", Pool::Attack, 0x21, "objects/blast-fire/blast_fire"),
        kind("bass", Pool::Actor, 0x4F, "objects/bass/bass"),
        kind("panel-strike", Pool::Attack, 0x09, "objects/panel-strike/panel_strike"),
        kind("sun-moon", Pool::Actor, 0x24, "objects/sun-moon/sun_moon"),
        kind("sun-meteor", Pool::Attack, 0xB5, "objects/sun-meteor/sun_meteor"),
        kind("moon-beam", Pool::Attack, 0xB6, "objects/moon-beam/moon_beam"),
        kind("drill", Pool::Attack, 0x71, "objects/drill/drill"),
        kind("thunder-column", Pool::Attack, 0x8B, "objects/thunder-column/thunder_column"),
    ];
    kinds.sort_by(|a, b| a.name.cmp(&b.name));
    kinds
}

/// A chip record with the fields tests don't care about filled in.
fn chip(id: ChipId, name: &str, action: u8, subtype: u8) -> ChipData {
    ChipData {
        id: Some(id),
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
        navi_damage: None,
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

/// Chip ids up to here exist (the ids no test uses are blanks).
const CHIP_IDS: ChipId = 0x120;

/// The chips, by id (the content looks chips up by index). Dimming chips
/// of the subtypes other scripts implement take ids 0x10 and up.
fn chips() -> Vec<ChipData> {
    let blank = |id| ChipData { class: ChipClass::Special, codes: vec![], ..chip(id, "Blank", 0, 0) };
    let mut all: Vec<ChipData> = (0..CHIP_IDS).map(blank).collect();
    for c in named_chips() {
        let id = c.id.expect("a numbered chip") as usize;
        all[id] = c;
    }
    all
}

fn named_chips() -> Vec<ChipData> {
    vec![
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
            script: Some("chips/0ec-eraseman/chip".into()),
            ..chip(ERASER, "Eraser", 0x1B, 5)
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
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::STANDARD_LIBRARY),
            script: Some("objects/navi-boost/navi_boost".into()),
            ..chip(BOOST, "Boost", 0x15, 38)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::STANDARD_LIBRARY),
            params: [2, 1, 0, 0],
            script: Some("objects/navi-boost/navi_boost".into()),
            ..chip(ARM, "Arm", 0x15, 38)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::STANDARD_LIBRARY),
            script: Some("objects/gauge-speed/gauge_speed".into()),
            ..chip(SLOW_GAUGE, "SlowGauge", 0x15, 25)
        },
        // Dimming chip subtypes 10, 11, 14 and ElemTrap's (20).
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
            damage: 40,
            script: Some("objects/trap-chip/trap_chip".into()),
            ..chip(ELEM_TRAP, "ElemTrap", 0x15, 20)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
            hit_param: 100,
            damage: 50,
            script: Some("objects/time-bom/time_bom".into()),
            ..chip(TIME_BOMB, "TimeBomb", 0x15, 10)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
            hit_param: 100,
            params: [1, 0, 0, 0],
            damage: 70,
            script: Some("objects/time-bom/time_bom".into()),
            ..chip(TIME_BOMB_PLUS, "TimeBomb+", 0x15, 10)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::DIMMING | ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
            hit_param: 100,
            damage: 60,
            script: Some("objects/mine/mine".into()),
            ..chip(MINE, "Mine", 0x15, 11)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
            hit_param: 30,
            damage: 40,
            script: Some("chips/059-crakshot/chip".into()),
            ..chip(CRACK, "Crack", 0x22, 0)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            class: ChipClass::Mega,
            hit_param: 100,
            params: [4, 0, 0, 0],
            damage: 50,
            script: Some("objects/elmnt-man/elmnt_man".into()),
            ..chip(ELEMENTS, "Elements", 0x1B, 16)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            element: Element::Aqua,
            class: ChipClass::Mega,
            damage: 40,
            script: Some("objects/spout-man/spout_man".into()),
            ..chip(SPOUT, "Spout", 0x1B, 7)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            class: ChipClass::Mega,
            params: [10, 0, 0, 0],
            damage: 40,
            script: Some("objects/heat-man/heat_man".into()),
            ..chip(HEAT, "Heat", 0x1B, 2)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            class: ChipClass::Mega,
            params: [10, 0, 0, 0],
            damage: 40,
            script: Some("objects/elec-man/elec_man".into()),
            ..chip(ELEC, "Elec", 0x1B, 3)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            class: ChipClass::Mega,
            params: [10, 0, 0, 0],
            damage: 40,
            script: Some("objects/slash-man/slash_man".into()),
            ..chip(SLASH, "Slash", 0x1B, 4)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            class: ChipClass::Mega,
            params: [10, 0, 0, 0],
            damage: 40,
            script: Some("objects/charge-man/charge_man".into()),
            ..chip(CHARGE, "Charge", 0x1B, 6)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            class: ChipClass::Mega,
            params: [10, 0, 0, 0],
            damage: 40,
            script: Some("objects/tomahawk-man/tomahawk_man".into()),
            ..chip(TOMAHAWK, "Tomahawk", 0x1B, 8)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            class: ChipClass::Mega,
            params: [10, 0, 0, 0],
            damage: 40,
            script: Some("objects/tengu-man/tengu_man".into()),
            ..chip(TENGU, "Tengu", 0x1B, 9)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            class: ChipClass::Mega,
            params: [10, 0, 0, 0],
            damage: 40,
            script: Some("objects/blast-man/blast_man".into()),
            ..chip(BLAST, "Blast", 0x1B, 12)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            class: ChipClass::Giga,
            damage: 30,
            script: Some("objects/bass/bass".into()),
            ..chip(BASS, "Shooter", 0x1B, 26)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::NAVI | ChipFlags::LIBRARY),
            class: ChipClass::Giga,
            damage: 90,
            script: Some("objects/sun-moon/sun_moon".into()),
            ..chip(SUN_MOON, "SunMoon", 0x1B, 25)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::STANDARD_LIBRARY),
            lockout: 20,
            script: Some("chips/0ae-fullcust/chip".into()),
            ..chip(FULL_GAUGE, "FullGage", 0x1C, 5)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::STANDARD_LIBRARY),
            family: ChipFamily::Plus,
            damage: 10,
            script: Some("chips/0c0-atk-10/chip".into()),
            ..chip(PLUS, "Plus", 0x1C, 3)
        },
        ChipData {
            flags: ChipFlags(ChipFlags::STANDARD_LIBRARY),
            script: Some("chips/11d-synctrgr/chip".into()),
            ..chip(SYNC, "Sync", 0x1C, 13)
        },
        spawning(BOOMERANG, "Boomer", 1, [0, 0, 0, 0], "objects/boomerang/boomerang"),
        spawning(LANCE, "Lance", 4, [0, 0, 0, 0], "objects/lance/lance"),
        spawning(FIST, "Fist", 8, [0, 3, 0, 0], "objects/fire-hit/fire_hit"),
        spawning(WORM, "Worm", 12, [0, 0, 0, 0], "objects/sand-worm/sand_worm"),
        spawning(FLAME_HOOK, "FlmHook", 14, [0, 1, 0, 0], "objects/flame-hook/flame_hook"),
        spawning(JUSTICE, "Justice", 19, [0, 0, 0, 0], "objects/justice-one/justice_one"),
        spawning(GOLEM, "Golem", 21, [0, 0, 0, 0], "objects/golem/golem"),
    ]
}

/// An instant chip whose effect spawns an object: made-up damage.
fn spawning(id: ChipId, name: &str, subtype: u8, params: [u8; 4], script: &str) -> ChipData {
    ChipData {
        flags: ChipFlags(ChipFlags::HAS_DAMAGE | ChipFlags::STANDARD_LIBRARY),
        hit_param: 0x30,
        params,
        damage: 40,
        lockout: 20,
        script: Some(script.into()),
        ..chip(id, name, 0x1C, subtype)
    }
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
        chip_bonus: None,
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
    for t in [0x04, 0x06, 0x07, 0x0A, 0x0B, 0x12, 0x15, 0x16, 0x2C, 0x32, 0x48] {
        collision_types[t] = attack;
    }
    collision_types[0x2A] = collision_types[0x05];
    // The Crosses' attacks (sword waves, hit zones, gusts, drills).
    for t in [0x06, 0x07, 0x1E, 0x4A] {
        collision_types[t] = attack;
    }
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
            // Dimming chip subtypes 10, 11, 14 and ElemTrap's (20): side 0's and
            // side 1's navi's panels (0x84, 0x85).
            PanelCondition { require: PLAYER[0], forbid: 0 },
            PanelCondition { require: PLAYER[1], forbid: 0 },
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
        empty_hand: EmptyHandChip { null_family: false, fire: false, flags: ChipFlags(0x10) },
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
            // Mode 1 next to the target with the column shifts; made-up
            // modes for the Crosses' tests: beside the target or diagonally
            // behind it (2), a panel or two in front of it (0xB, only two
            // away in the far column), and the claw's (0xC); the rest stay.
            modes: (0..=0xC)
                .map(|mode| {
                    let near = |offsets: Vec<PanelOffset>| LockonMode { mode, rule: LockonRule::Near, offsets, ..Default::default() };
                    let off = |dx, dy| PanelOffset { dx, dy };
                    match mode {
                        1 => LockonMode { column_shifts: true, ..near(vec![off(-1, 0)]) },
                        2 => near(vec![off(-1, 0), off(-1, 1)]),
                        0xB => LockonMode { far_column_offsets: Some(vec![off(-2, 0)]), ..near(vec![off(-1, 0), off(-2, 0)]) },
                        0xC => near(vec![off(-1, 0)]),
                        _ => LockonMode { mode, rule: LockonRule::Stay, ..Default::default() },
                    }
                })
                .collect(),
            column_shifts: vec![-1, -2],
            clear_path: [PanelCondition { require: 0, forbid: pflags::OCCUPIED }; 2],
            charged_sword_modes: vec![1; 0x13],
        },
        berserk: BerserkRules {
            step,
            opponent: [
                PanelCondition { require: BODY[1], forbid: 0 },
                PanelCondition { require: BODY[0], forbid: 0 },
            ],
            blocking: [NEUTRAL | OTHER_BODY[1], NEUTRAL | OTHER_BODY[0]],
            opposing_player: [PLAYER[1], PLAYER[0]],
        },
        custom_screen: custom_screen_layout(),
        actor_records: Vec::new(),
        cross_palettes: (0..11).collect(),
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
    let settings = |actors| StageSettings {
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
        // The elements navi's overlay (variant 0x0F).
        body_overlays: (0..0x10)
            .map(|id| BodyOverlay { id, sprite: SpriteId { category: 8, index: 0x11 }, in_front: vec![true; 0x20] })
            .collect(),
        sun_beam_looks: vec![SpriteId { category: 0x0C, index: 0x10 }, SpriteId { category: 0x0C, index: 0x11 }],
        boomerangs: (0..5).map(|id| BoomerangKind { id, speed: 0x8_0000, turn_speed: 0x6_0000, grass: id < 3 }).collect(),
        projectiles: projectiles(),
        flying_shots: flying_shots(),
        sword_waves: (0..0x13).map(sword_wave).collect(),
        // Made-up looks for NameIDs 0xCD..=0xFF (0xCF has none).
        name_looks: (0xCD..=0xFF)
            .map(|name_id| NameLook {
                name_id,
                sprite: (name_id != 0xCF).then_some(SpriteId { category: 0x10, index: 0 }),
                anim: 1,
                palette: 0,
                shadow: true,
            })
            .collect(),
        kinds: kinds(),
        shock_waves: (0..16).map(|id| ShockWave { id, sprite: SpriteId { category: 0x10, index: 3 }, anim: 1, ticks: 6, panel: None }).collect(),
    }
}

/// The swords' blade (attachment 7), held at the gun's point.
fn blade_kind() -> AttachmentKind {
    AttachmentKind { id: 7, sprite: SpriteId { category: 0x0C, index: 0x08 }, palette: 0, lift: 0, attach_point: Some(3) }
}

/// The projectile's kinds: a plain shot (0), one that cracks the panel it
/// hits (1), one that breaks it (2), one that turns it to grass (3), one
/// that lays a road away from its side (4), one that bursts (5), a charged
/// shot with a spark (6) and a drawn one that climbs (7).
fn projectiles() -> Vec<ProjectileKind> {
    let shot = |id| ProjectileKind {
        id,
        self_type: 0x04,
        target_type: 0x05,
        hit_mod: 0,
        element: Element::Null,
        secondary: SecondaryElements::default(),
        hit_effect: 0,
        sprite: None,
        anim: 0,
        status: 0,
        bug: 0,
        bug_arg: 0,
        hit_panel: None,
        bursts: false,
        climbs: false,
    };
    vec![
        shot(0),
        ProjectileKind { hit_panel: Some(PanelHit::Crack), ..shot(1) },
        ProjectileKind { hit_panel: Some(PanelHit::Break), ..shot(2) },
        ProjectileKind {
            hit_panel: Some(PanelHit::SetType { left_side: PanelType::Grass, right_side: PanelType::Grass }),
            ..shot(3)
        },
        ProjectileKind {
            hit_panel: Some(PanelHit::SetType { left_side: PanelType::RoadRight, right_side: PanelType::RoadLeft }),
            ..shot(4)
        },
        ProjectileKind { bursts: true, hit_mod: 3, ..shot(5) },
        ProjectileKind { hit_effect: 5, ..shot(6) },
        ProjectileKind { sprite: Some(SpriteId { category: 0x0C, index: 0x21 }), anim: 0, climbs: true, ..shot(7) },
    ]
}

/// The flying shot's kinds: arrows (0 to 5; 2 waits with a sound and
/// sparks over its panel, 5 leaves an effect) and the thrown obstacle (6).
fn flying_shots() -> Vec<FlyingShotKind> {
    let arrow = |id| FlyingShotKind {
        id,
        self_type: 0x04,
        target_type: 0x05,
        hit_mod: 0,
        element: Element::Null,
        secondary: SecondaryElements::default(),
        hit_effect: 5,
        sprite: SpriteId { category: 0x0C, index: 0x21 },
        anim: 0,
        shadow: false,
        speed: 8 << 16,
        range: 8,
        status: 0,
        highlight: false,
        obstacle: false,
        panel_spark: false,
        launch_sound: None,
        end_effect: None,
    };
    vec![
        arrow(0),
        arrow(1),
        FlyingShotKind { panel_spark: true, launch_sound: Some(0x18A), element: Element::Aqua, ..arrow(2) },
        arrow(3),
        arrow(4),
        FlyingShotKind { range: 2, end_effect: Some(7), ..arrow(5) },
        FlyingShotKind {
            obstacle: true,
            highlight: true,
            speed: 10 << 16,
            hit_mod: 3,
            secondary: SecondaryElements(SecondaryElements::BREAK),
            ..arrow(6)
        },
    ]
}

/// A made-up sword wave: one panel of region, three panels of reach.
fn sword_wave(id: u8) -> SwordWave {
    SwordWave {
        id,
        self_type: 4,
        target_type: 5,
        hit_mod: 3,
        region: 1,
        sprite: SpriteId { category: 0x0C, index: 0x14 },
        anim: 0,
        animates: id % 2 == 0,
        highlight: id == 1,
        reach: 3,
        ground_shadow: false,
        palette: 0,
        status: 0,
        speed: 0x8_0000,
    }
}

fn regions() -> Vec<Vec<PanelOffset>> {
    let p = |dx, dy| PanelOffset { dx, dy };
    let mut v = vec![vec![p(0, 0)]; 0x2F];
    v[0] = Vec::new();
    v[2] = vec![p(0, 0), p(1, 0)];
    v[3] = vec![p(1, 0)];
    v[4] = vec![p(0, 0), p(0, -1), p(0, 1)];
    v[0x11] = vec![p(0, 0), p(0, -1), p(0, 1), p(1, 0), p(1, -1), p(1, 1)];
    // The Beast charged chips' pillars: here the panel in front and two
    // past it.
    v[0x1A] = vec![p(0, 0), p(2, 0)];
    // A block around the panel (the scatter's panel search).
    v[0x0F] = vec![p(0, 0), p(1, 0), p(-1, 0), p(0, -1), p(0, 1), p(1, -1), p(1, 1), p(-1, -1), p(-1, 1)];
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
    // The elements navi (appearing, leaving, winding up, attacking, a vine)
    // and its overlay; its meteor, ice and bolt.
    let mut elements = vec![once(4); 0x13];
    elements[0] = vec![f(8, 0), f(8, LAST | LOOP)];
    sprites.insert(SpriteId { category: 8, index: 0x10 }, elements);
    sprites.insert(SpriteId { category: 8, index: 0x11 }, vec![once(4); 0x20]);
    sprites.insert(SpriteId { category: 0x0C, index: 0x31 }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    sprites.insert(SpriteId { category: 0x10, index: 0x0F }, vec![once(4), vec![f(4, 0), f(4, LAST | LOOP)]]);
    sprites.insert(SpriteId { category: 0x14, index: 0x14 }, vec![vec![f(3, 0), f(3, LAST | LOOP)]]);
    // The shooting navi (rising, raising his arm, shooting) and his cape
    // (his animation + 0x14), his shots' bursts; the sun-and-moon navi
    // and its moonlight.
    let mut shooter = vec![once(4); 0x21];
    shooter[0] = vec![f(8, 0), f(8, LAST | LOOP)];
    shooter[0x0C] = vec![f(4, 0), f(4, LAST | LOOP)];
    sprites.insert(SpriteId { category: 8, index: 0x13 }, shooter);
    sprites.insert(SpriteId { category: 0x10, index: 0x26 }, vec![vec![f(3, 0), f(3, LAST)]]);
    sprites.insert(SpriteId { category: 0x0C, index: 0x64 }, vec![vec![f(8, 0), f(8, LAST | LOOP)]; 5]);
    // The water navi, his ball, splash, pillar, geyser and marks, and his
    // layer.
    let mut spout = vec![once(4); 0x16];
    spout[0] = vec![f(8, 0), f(8, LAST | LOOP)];
    sprites.insert(SpriteId { category: 8, index: 6 }, spout);
    sprites.insert(SpriteId { category: 0x0C, index: 0x23 }, vec![once(4), vec![f(2, 0), f(2, LAST | LOOP)]]);
    sprites.insert(SpriteId { category: 0x0C, index: 0x1A }, vec![vec![f(5, 0), f(5, LAST | LOOP)]]);
    sprites.insert(SpriteId { category: 0x10, index: 0x1F }, vec![vec![f(3, 0), f(3, LAST | LOOP)]; 4]);
    sprites.insert(SpriteId { category: 0x10, index: 0x20 }, vec![vec![f(3, 0), f(3, LAST | LOOP)]; 3]);
    sprites.insert(SpriteId { category: 0x10, index: 0x21 }, vec![vec![f(6, 0), f(6, LAST | LOOP)]]);
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
    // The arrow.
    sprites.insert(SpriteId { category: 0x0C, index: 0x21 }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    // The grab shot: falling, landing.
    sprites.insert(SpriteId { category: 0x0C, index: 0x13 }, vec![vec![f(8, LAST | LOOP)], vec![f(3, 0), f(3, LAST)]]);
    // GroundCross's falling rock (0) and its chunks (1).
    sprites.insert(SpriteId { category: 0x10, index: 5 }, vec![vec![f(8, LAST | LOOP)], vec![f(4, LAST | LOOP)]]);
    // The Beast charged chips' pillars (flames, lightning: rising, dying
    // down), surge (rising, ebbing, falling) and whirlwind.
    for index in [0x1C] {
        sprites.insert(SpriteId { category: 0x0C, index }, vec![vec![f(4, LAST | LOOP)], vec![f(2, 0), f(2, LAST)]]);
    }
    sprites.insert(SpriteId { category: 0x10, index: 0x32 }, vec![vec![f(4, LAST | LOOP)], vec![f(2, 0), f(2, LAST)]]);
    sprites.insert(SpriteId { category: 0x10, index: 0x2E }, vec![vec![f(6, LAST | LOOP)]; 3]);
    sprites.insert(SpriteId { category: 0x10, index: 0x44 }, vec![vec![f(3, 0), f(3, LAST | LOOP)]]);
    // The gust, the sword waves, EraseCross's beam (opening, beaming,
    // closing) and the drill arm (attachment 0x20).
    sprites.insert(SpriteId { category: 0x0C, index: 0x2E }, vec![vec![f(4, 0), f(4, LAST | LOOP)]]);
    sprites.insert(SpriteId { category: 0x0C, index: 0x14 }, vec![vec![f(3, 0), f(3, LAST)]]);
    sprites.insert(SpriteId { category: 0x10, index: 0x4C }, vec![vec![f(3, 0), f(3, LAST)], vec![f(8, LAST | LOOP)], once(4)]);
    sprites.insert(SpriteId { category: 0x0C, index: 0x20 }, vec![once(4), vec![f(4, 0), f(4, LAST | LOOP)]]);
    // The hive (closed, open), a bee, and a dragon's animations.
    sprites.insert(SpriteId { category: 0x0C, index: 0x5E }, vec![vec![f(30, LAST | LOOP)], vec![f(4, 0), f(30, LAST)]]);
    sprites.insert(SpriteId { category: 0x10, index: 0x31 }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    sprites.insert(SpriteId { category: 0x04, index: 0x10 }, vec![vec![f(6, LAST | LOOP)]; 8]);
    // The crack shot: flying.
    sprites.insert(SpriteId { category: 0x0C, index: 0x33 }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    // Effects and sparks.
    sprites.insert(SpriteId { category: 0x14, index: 0 }, vec![vec![f(3, 0), f(3, 0), f(3, LAST)]]);
    sprites.insert(SpriteId { category: 0x14, index: 1 }, vec![vec![f(2, 0), f(2, LAST)]]);
    // The rising bubble.
    sprites.insert(SpriteId { category: 0x14, index: 2 }, vec![once(4), vec![f(4, 0), f(4, 0), f(4, LAST)]]);
    // Dimming chip subtypes 10, 11, 14 and ElemTrap's (20): the countdown
    // bomb (rising, standing; twice), the mine, the guardian statue
    // (standing, striking).
    let rise_and_stand = vec![vec![f(3, 0), f(3, LAST)], vec![f(20, LAST | LOOP)]];
    sprites.insert(SpriteId { category: 0x0C, index: 0x23 }, [rise_and_stand.clone(), rise_and_stand].concat());
    sprites.insert(SpriteId { category: 0x0C, index: 0x22 }, vec![vec![f(4, 0), f(4, LAST | LOOP)]]);
    sprites.insert(SpriteId { category: 0x0C, index: 0x35 }, vec![vec![f(20, LAST | LOOP)], vec![f(4, 0), f(8, LAST)]]);
    Animations { sprites }
}
