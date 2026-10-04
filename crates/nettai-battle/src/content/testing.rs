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
//! overlay), read from the repository, and its own modules
//! (testdata/content): the test chips are definitions there, made-up
//! records whose uses are BN6's builders and actions, so the tests run the
//! real scripts on data they can reason about. (The BN6 modules it loads
//! bring their own chip definitions too; tests name either by key.)

use super::*;
use crate::field::{PanelType, pflags};

use std::sync::Arc;

// The test chips (testdata/content/chips/test/chips.luau), by key: made-up
// records whose uses are BN6's builders and actions.
/// Three GunDelSol levels and an EX (two columns).
pub const SUN_GUN_1: &str = "test/sun-gun-1";
pub const SUN_GUN_2: &str = "test/sun-gun-2";
pub const SUN_GUN_3: &str = "test/sun-gun-3";
pub const SUN_GUN_EX: &str = "test/sun-gun-ex";
/// A dimming chip (the invisibility freeze).
pub const VEIL: &str = "test/veil";
/// A navi chip (the eraser navi).
pub const ERASER: &str = "test/eraser";
/// Instant chips: a plus chip used on its own, fists (FireHit's) and flame
/// hooks (FlmHook's).
pub const PLUS: &str = "test/plus";
pub const FIST: &str = "test/fist";
pub const FLAME_HOOK: &str = "test/flame-hook";
/// The thrown chips: a bomb, a seed that poisons panels, a flash bomb and
/// a bug bomb.
pub const BOMB: &str = "test/bomb";
pub const SEED: &str = "test/seed";
pub const FLASH: &str = "test/flash";
pub const BUG: &str = "test/bug";
/// A sword (a column of three panels ahead).
pub const BLADE: &str = "test/blade";
/// A step sword (the same, after a step two panels ahead).
pub const STEP_BLADE: &str = "test/step-blade";
/// A strike at stunned or grounded opponents.
pub const STUN_BLADE: &str = "test/stun-blade";
/// A blank chip that is the AntiNavi trap when a side's defensive-chip
/// record holds it.
pub const ANTI_NAVI: &str = "test/anti-navi";
/// A trap chip that sets no object.
pub const TRAP: &str = "test/trap";
/// An element trap (the trap object).
pub const ELEM_TRAP: &str = "test/elem-trap";
/// Time bombs: the plain one and the big one.
pub const TIME_BOMB: &str = "test/time-bomb";
pub const TIME_BOMB_PLUS: &str = "test/time-bomb-plus";
// Navi chips: the elements navi, the water navi, the heat, elec, slash,
// charge, tomahawk, tengu and blast navis, the shooting navi (Bass's) and
// the sun-and-moon navi.
pub const ELEMENTS: &str = "test/elements";
pub const SPOUT: &str = "test/spout";
pub const HEAT: &str = "test/heat";
pub const ELEC: &str = "test/elec";
pub const SLASH: &str = "test/slash";
pub const CHARGE: &str = "test/charge";
pub const TOMAHAWK: &str = "test/tomahawk";
pub const TENGU: &str = "test/tengu";
pub const BLAST: &str = "test/blast";
pub const BASS: &str = "test/shooter";
pub const SUN_MOON: &str = "test/sun-moon";
/// The link navis' own chips: BN6's HeatPres, DElecSwd, RSlash, EDeletBm,
/// VolcChrg, DripShwr, ETomahwk, FTornado, RC Brakr and DustBrk's actions
/// (navis/<navi>/chip.luau), as chips of made-up damage. Any navi can use
/// them here.
pub const LINK_CHIPS: [&str; 10] = [
    "test/heatpres",
    "test/delecswd",
    "test/rslash",
    "test/edeletbm",
    "test/volcchrg",
    "test/dripshwr",
    "test/etomahwk",
    "test/ftornado",
    "test/rc-brakr",
    "test/dustbrk",
];
/// A link navi (the content's navi 1; AI index 4, whose actor record has
/// no hooks).
pub const LINK_NAVI: &str = "test/link-navi";
/// MegaMan, the navi that changes form.
pub const MEGAMAN: &str = "megaman";

/// The test stages (testdata/content/stages/test.luau), link battles on
/// the plain field: two navis, side 1's placed first (the usual netbattle
/// order); side 0's first; two navis with two rocks, one on each side; and
/// two navis with three boulders (the field has two stage slots).
pub const LINK_BATTLE: &str = "test/link-battle";
pub const LINK_BATTLE_SIDE0_FIRST: &str = "test/link-battle-side0-first";
pub const ROCK_BATTLE: &str = "test/rock-battle";
pub const BOULDER_BATTLE: &str = "test/boulder-battle";

/// The test stages' music's song (the asset `test-stage-music`).
pub const STAGE_SONG: u16 = 0x16;

/// The navi's sprite (base form), as the test pack holds it.
pub const NAVI_SPRITE: PackSprite = PackSprite { category: 0, index: 0 };

/// The test pack's sprite `id` as the test content names it (its handle).
pub fn sprite(id: PackSprite) -> SpriteId {
    let c = content();
    let pack = c.assets.pack(ROOT).expect("the test pack");
    SpriteId(c.assets.sprite_handle(pack, id).unwrap_or_else(|| panic!("the test pack has no sprite {id}")))
}

/// Sprite `id` as its pack holds it (one sprite may have several names, so
/// several handles: tests compare what the pack holds).
pub fn pack_sprite(c: &Content, id: SpriteId) -> PackSprite {
    c.assets.sprite(id.0).unwrap_or_else(|| panic!("no sprite has handle {}", id.0)).id
}

/// Add pack `game`'s assets (`index`, its sprites' timing `sprites`) to the
/// test content: its asset names over both packs, its animations (an
/// undefined content: `Content::define` after).
pub fn add_pack(c: &mut Content, game: &str, index: nettai_content_api::PackIndex, sprites: std::collections::BTreeMap<PackSprite, Vec<Vec<AnimFrame>>>) {
    c.assets = nettai_content_api::AssetNames::of_packs(vec![(ROOT.to_string(), pack_index()), (game.to_string(), index)]);
    c.animations = animations(&c.assets);
    let pack = c.assets.pack(game).expect("the pack just added");
    c.animations.add_pack(&c.assets, pack, &sprites, &Default::default());
}

/// The test pack's song `n` as the test content names it (its handle).
pub fn sound(n: u16) -> crate::sound::SoundId {
    let c = content();
    let pack = c.assets.pack(ROOT).expect("the test pack");
    crate::sound::SoundId(c.assets.sound_handle(pack, n).unwrap_or_else(|| panic!("the test pack has no song {n:#x}")))
}

/// The test content's asset `name` of `kind` (`test-navi`): its handle.
pub fn asset_named(c: &Content, kind: nettai_content_api::AssetKind, name: &str) -> u16 {
    c.assets.handle(kind, name).unwrap_or_else(|| panic!("the test content has no {kind} {name:?}"))
}

/// The test content's sprite `name` (its handle).
pub fn sprite_named(c: &Content, name: &str) -> SpriteId {
    SpriteId(asset_named(c, nettai_content_api::AssetKind::Sprite, name))
}

/// The test stages' music.
pub fn stage_music() -> crate::sound::SoundId {
    sound(STAGE_SONG)
}
/// Where the navi holds a gun.
pub const GUN_POINT: AttachPoint = AttachPoint { x: 20, y: 16 };

// Collision type bits by what they mean (field-collision-damage.md §3.3),
// which panels' flags carry too. (The test content's collision types are
// testdata/content/rules/ruleset.luau's.)
const BODY: [u32; 2] = [0x0800_0000, 0x0400_0000];
const OTHER_BODY: [u32; 2] = [0x0200_0000, 0x0100_0000];
const NEUTRAL: u32 = 0x0080_0000;
const PLAYER: [u32; 2] = [0x0040_0000, 0x0020_0000];
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

/// A round on this content on stage `stage` (its key), both navis with
/// `stats`: RNG seed 1, side 0's perspective, no set score, no folders.
pub fn round_setup(stage: &str, stats: crate::setup::NaviStats) -> crate::setup::RoundSetup {
    let (content, hash) = shared();
    crate::setup::RoundSetup {
        content: *hash,
        settings: crate::setup::BattleSettings::on(content, content.stage_by_key(stage)),
        ruleset: None,
        navi_stats: [stats; 2],
        rng: 1,
        local_side: 0,
        score: Default::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        players: Default::default(),
        link_delay: 0,
    }
}

/// A navi with `hp` HP and nothing else of note.
pub fn stats(hp: u16) -> crate::setup::NaviStats {
    crate::setup::NaviStats { hp, max_hp: hp, max_base_hp: hp, ..megaman_on(&content()) }
}

/// The link navi's stats (`LINK_NAVI`), by `content`'s handles.
pub fn link_navi_on(content: &Content) -> crate::setup::NaviStats {
    crate::setup::NaviStats { navi: content.navi_by_key(LINK_NAVI), ..megaman_on(content) }
}

/// Stats with nothing of note but MegaMan in his base form, by `content`'s
/// handles.
pub fn megaman_on(content: &Content) -> crate::setup::NaviStats {
    let navi = content.navi_by_key(MEGAMAN);
    let base = content.base_form_for(navi);
    crate::setup::NaviStats {
        navi,
        form: base,
        starting_form: base,
        ..Default::default()
    }
}

/// The chip `key` in the content (its handle there).
pub fn chip_in(content: &Content, key: &str) -> nettai_content_api::ChipHandle {
    content.defs.chip_by_key(key).unwrap_or_else(|| panic!("the test content defines no chip {key:?}"))
}

/// The chip `key` in the shared test content.
pub fn chip_handle(key: &str) -> nettai_content_api::ChipHandle {
    chip_in(&content(), key)
}

/// The weapon `content` defines as `key`, as a weapon slot holds it.
pub fn weapon_in(content: &Content, key: &str) -> Option<nettai_content_api::WeaponHandle> {
    Some(content.weapon_by_key(key))
}

/// The weapon the shared test content defines as `key`, as a weapon slot
/// holds it.
pub fn weapon(key: &str) -> Option<nettai_content_api::WeaponHandle> {
    weapon_in(&content(), key)
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
/// BN6's instant chips content defines whose effects fill the gauge, sync
/// the navi, and spawn objects: FullCust, SyncTrgr, Boomer, Lance,
/// SandWrm1, JustcOne, GolmHit1 (chips/fullcust ... chips/golmhit).
pub const FULL_CUST: &str = "fullcust";
pub const SYNC_TRIGGER: &str = "synctrgr";
pub const BOOMER: &str = "boomer";
pub const LANCE: &str = "lance";
pub const SAND_WORM: &str = "sandwrm1";
pub const JUSTICE_ONE: &str = "justcone";
pub const GOLEM_HIT: &str = "golmhit1";
/// BN6's RockCube (chips/rockcube): a dimming chip content defines, which
/// places a rock in front of its user.
pub const ROCK_CUBE: &str = "rockcube";
pub const TICK_SHOT: &str = "test/tick-shot";
/// BN6's CrakShot, Rflectr1 and Recov50 (chips/crakshot, chips/rflectr,
/// chips/recov): standard chips content defines, which dig up the panel
/// ahead, guard and reflect, and heal.
pub const CRAK_SHOT: &str = "crakshot";
pub const REFLECTOR_1: &str = "rflectr1";
pub const RECOV_50: &str = "recov50";
/// BN6's SloGauge and Mine (chips/slogauge, chips/mine): dimming chips
/// content defines, which slow the custom gauge and lay a mine.
pub const SLOW_GAUGE: &str = "slogauge";
pub const MINE: &str = "mine";
/// BN6's RskyHny2 and ElecDrgn (chips/rskyhny, chips/elecdrgn): chips
/// content defines, which send bees and an elec dragon.
pub const BEES: &str = "rskyhny2";
pub const DRAGON: &str = "elecdrgn";
/// BN6's Gregar and Falzar (chips/gregar, chips/falzar: the Japanese ROMs'
/// giga cut-in chips), which summon the cyber beasts.
pub const GREGAR: &str = "gregar";
pub const FALZAR: &str = "falzar";

/// The test content's game: its own modules and the BN6 modules it borrows
/// are one folder, `test`, whose ids are `test:...` (the borrowed modules'
/// `bn6:` ids and asset names read as `test:` ones: `borrowed`).
pub const ROOT: &str = "test";

/// The content model v2 test pack (crates/nettai-battle/testdata/pack):
/// definitions the engine's tests run.
const TEST_PACK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/pack");

/// The test content's own modules (its roles).
const TEST_CONTENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/content");

/// The support pack (content/exelib) in `scripts`: what BN6's modules
/// require by `@exelib/...`, for content that loads them.
pub fn add_shared(scripts: &mut Scripts) {
    scripts.add_support(EXELIB_PACK, modules_under(EXELIB));
}

/// A manifest for game `game`, whose modules `scripts` holds under its
/// name, that loads every one of them (`Scripts::add_game`'s) and uses the
/// support packs `scripts` holds: for tests that load a game's directory as
/// it is.
pub fn add_index(scripts: &mut Scripts, game: &str) {
    let prefix = format!("{game}{}", nettai_content_api::keys::SEPARATOR);
    let own: std::collections::BTreeMap<String, String> =
        scripts.modules.iter().filter_map(|(name, source)| Some((name.strip_prefix(&prefix)?.to_string(), source.clone()))).collect();
    let uses = scripts.packs.iter().filter(|p| p.kind == nettai_content_api::PackKind::Support).map(|p| p.id.clone()).collect();
    scripts.set_manifest(Scripts::manifest_of(game, &own, uses));
}

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
    // The test pack brings its own roles (the stock ruleset's `roles`).
    c.scripts.modules.insert(Scripts::name(ROOT, "rules/roles"), "return require(\"../test/rules/roles\")\n".to_string());
    for (path, source) in modules_under(TEST_PACK) {
        c.scripts.modules.insert(Scripts::name(ROOT, &format!("test/{path}")), source);
    }
    // (Its index, written again for these modules.)
    add_index(&mut c.scripts, ROOT);
    c.define().unwrap_or_else(|e| panic!("content error: {e}"));
    c
}

/// The content set with stage `stage` changed by `f` (its music, say).
pub fn restaged(stage: &str, f: impl FnOnce(&mut StageData)) -> Content {
    let mut c = build();
    let h = c.stage_by_key(stage);
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
    let assets = assets();
    Content {
        // (The navis and the base form are definitions:
        // testdata/content/navis/test.luau.)
        base_rules: rules(),
        rules: Default::default(),
        animations: animations(&assets),
        scripts: scripts(),
        assets,
        defs: Default::default(),
        strings: strings(),
    }
}

/// The test content's strings (testdata/content/locales/en.toml): its
/// chips' and navis' display text, whose shape the define phase
/// counts (the navis' no-running message: 19 and 12 characters).
pub fn strings() -> crate::content::strings::Strings {
    let file = format!("{TEST_CONTENT}/locales/en.toml");
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
    let s: crate::content::strings::Strings = toml::from_str(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
    s
}

/// A synthetic asset index for `modules`: every name of `game`'s pack they
/// give an `asset.<kind>("...")` call, each a made-up asset of its own
/// (nothing ROM-derived; for tests that load modules the test content
/// doesn't list).
pub fn asset_names_used(game: &str, modules: &std::collections::BTreeMap<String, String>) -> nettai_content_api::AssetNames {
    nettai_content_api::AssetNames::of_pack(game, pack_index_used(modules))
}

/// [`asset_names_used`] for the modules of `scripts`: the game's pack's (a
/// match plays one game; content without packs, a pack of no name).
pub fn asset_names_for(scripts: &Scripts) -> nettai_content_api::AssetNames {
    let game = scripts.games().first().cloned().unwrap_or_default();
    asset_names_used(&game, &scripts.modules)
}

/// [`asset_names_used`]'s pack index (its own, unqualified names).
pub fn pack_index_used(modules: &std::collections::BTreeMap<String, String>) -> nettai_content_api::PackIndex {
    use nettai_content_api::AssetKind;
    let mut a = nettai_content_api::PackIndex::default();
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
                        a.sprites.entry(name.into()).or_insert(PackSprite { category: 0x7F, index: id as u8 });
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
/// (with BN6's numbers where its tests look at them, made-up assets for
/// the rest), a few made-up ones for the test pack, and a placeholder.
fn assets() -> nettai_content_api::AssetNames {
    nettai_content_api::AssetNames::of_pack(ROOT, pack_index())
}

/// [`assets`]' pack index.
fn pack_index() -> nettai_content_api::PackIndex {
    let mut a = numbered_assets();
    let used = pack_index_used(&scripts().modules);
    for (name, id) in used.sprites {
        a.sprites.entry(name).or_insert(id);
    }
    for (name, id) in used.sounds {
        a.sounds.entry(name).or_insert(id);
    }
    for (name, id) in used.banners {
        a.banners.entry(name).or_insert(id);
    }
    a.backgrounds.extend(used.backgrounds);
    a.mugshots.extend(used.mugshots);
    a
}

/// The ids of the test content's sounds, music, sprites and banners for the
/// ruleset's roles (testdata/content/rules/ruleset.luau's assets, by role
/// name): what the tests look for in the cues and the HUD.
const ROLE_SOUNDS: &[(&str, u16)] = &[
    ("panel-crack", 0x97),
    ("panel-poison", 0x90),
    ("own-hit", 0x6b),
    ("hit", 0x6d),
    ("damage", 0x85),
    ("guard", 0x6e),
    ("counter-hit", 0x86),
    ("deleted", 0x6c),
    ("recovery", 0x8a),
    ("damage-bonus", 0x87),
    ("pause", 0x9f),
    ("gauge-full", 0x8f),
    ("low-hp", 0x84),
    ("cut-in", 0xa5),
    ("telop", 0x173),
    ("buster-charge", 0x71),
    ("buster-charged", 0x72),
    ("freeze", 0x118),
    ("bubble", 0x12d),
    ("bubble-pop", 0x124),
    ("confusion", 0x88),
    ("invisible", 0x93),
    ("appear", 0x94),
    ("arrive", 0x129),
    ("fade", 0x8e),
    ("obstacle-lift", 0x12a),
    ("obstacle-throw", 0x10c),
    ("cross-merge", 0x8c),
    ("form-change", 0xf7),
    ("cross-change", 0x8d),
    ("cross-change-chime", 0x77),
    ("beast-out", 0x100),
    ("gregar-roar", 0x1cc),
    ("falzar-roar", 0x1cd),
    ("beast-over-rumble", 0x19a),
    ("beast-over-burst", 0x12e),
    ("cross-special", 0x182),
    ("refused", 0x69),
    ("custom-open", 0x79),
    ("custom-cursor", 0x7f),
    ("custom-hide", 0x80),
    ("custom-pick", 0x81),
    ("custom-ok", 0x82),
    ("custom-back", 0x83),
    ("custom-cross-open", 0x7a),
    ("custom-cross-close", 0x7d),
    ("custom-cross-chosen", 0x92),
    ("custom-run-message", 0x7b),
    ("custom-description", 0x9c),
    ("custom-description-close", 0x9e),
    ("custom-beast-out-falzar", 0x193),
    ("custom-beast-out-gregar", 0x191),
    ("custom-beast-out-flash", 0xbc),
    ("custom-cancel", 0x1d2),
    ("custom-redeal", 0x182),
    ("custom-redeal-shuffle", 0x113),
    ("custom-scrap", 0x196),
    ("custom-scrap-done", 0x182),
    ("program-advance-part", 0x91),
    ("program-advance", 0x92),
];
const ROLE_MUSIC: &[(&str, u16)] = &[
    ("link-battle", 0x15),
    ("winner-special", 0x19),
    ("winner", 0x1f),
    ("loser", 0x1a),
];
const ROLE_SPRITES: &[(&str, (u8, u8))] = &[
    ("charge-glow", (0x14, 0x08)),
    ("charge-glow-a", (0x14, 0x15)),
    ("full-synchro-aura", (0x14, 0x16)),
    ("confusion", (0x14, 0x0b)),
    ("blindness", (0x14, 0x09)),
    ("immobilized", (0x10, 0x00)),
    ("ice", (0x14, 0x1c)),
    ("bubble", (0x0c, 0x20)),
    ("hit-marker", (0x14, 0x07)),
    ("eruption", (0x10, 0x24)),
    ("lockon-marker", (0x0c, 0x09)),
    ("idle-overlay", (0x10, 0x21)),
    ("beast-head", (0x0c, 0x0a)),
];
const ROLE_BANNERS: &[(&str, u8)] = &[
    ("round-start", 0x30),
    ("turn-start", 0xc),
    ("final-turn", 0x10),
    ("draw", 0x1c),
    ("judge", 0x28),
    ("telop", 0x4c),
    ("telop-remote", 0x50),
    ("program-advance", 0x24),
    ("program-advance-empty", 0x34),
];

/// The test content's assets with BN6's numbers.
fn numbered_assets() -> nettai_content_api::PackIndex {
    let mut a = nettai_content_api::PackIndex::default();
    for (role, id) in ROLE_SOUNDS {
        a.sounds.insert(format!("test-sound-{role}"), *id);
    }
    for (role, id) in ROLE_MUSIC {
        a.sounds.insert(format!("test-music-{role}"), *id);
    }
    for (role, (category, index)) in ROLE_SPRITES {
        a.sprites.insert(format!("test-sprite-{role}"), PackSprite { category: *category, index: *index });
    }
    for (role, id) in ROLE_BANNERS {
        a.banners.insert(format!("test-banner-{role}"), *id);
    }
    // The test stages' (testdata/content/stages/test.luau).
    a.sounds.insert("test-stage-music".into(), STAGE_SONG);
    a.backgrounds.insert("test-background".into(), 0);
    // The test navis' (testdata/content/navis/test.luau).
    a.sprites.insert("test-megaman".into(), PackSprite { category: 8, index: 0 });
    a.sprites.insert("test-navi".into(), NAVI_SPRITE);
    a.banners.insert("test-win".into(), 0x40);
    a.banners.insert("test-deleted".into(), 0x44);
    let sprite = |c, i| PackSprite { category: c, index: i };
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
        ("small-puff", sprite(0x14, 0x02)),
        ("puff", sprite(0x14, 0x0D)),
        ("burst", sprite(0x14, 0x0A)),
        ("rock-cubes", sprite(0x10, 0x00)),
        ("grab-shot", sprite(0x0C, 0x13)),
        ("hit-sparks", sprite(0x14, 0x05)),
        ("fire-sword", sprite(0x0C, 0x36)),
        ("aqua-sword", sprite(0x0C, 0x37)),
        ("elec-sword", sprite(0x0C, 0x38)),
        ("sword-slash", sprite(0x0C, 0x14)),
        ("big-slash", sprite(0x0C, 0x15)),
        ("cross-slash", sprite(0x10, 0x41)),
        ("pink-flash", sprite(0x14, 0x04)),
        ("muzzle-flash", sprite(0x0C, 0x06)),
        ("buster-arm", sprite(0x0C, 0x03)),
        ("eraseman", sprite(0x08, 0x04)),
        ("buster-up", sprite(0x14, 0x1B)),
        ("erase-mark", sprite(0x10, 0x50)),
        ("erase-beam", sprite(0x10, 0x51)),
        ("air-raid-plane", sprite(0x04, 0x18)),
        ("aura", sprite(0x0C, 0x07)),
        ("barrier", sprite(0x0C, 0x3D)),
        ("bubble", sprite(0x0C, 0x20)),
        ("instrument", sprite(0x04, 0x0A)),
        ("jet-flame", sprite(0x10, 0x28)),
        ("propeller", sprite(0x10, 0x3D)),
        ("sensor", sprite(0x04, 0x05)),
        ("wide-navi", sprite(0x08, 0x14)),
        ("summon-black", sprite(0x04, 0x1D)),
        ("flame-hook-fire", sprite(0x0C, 0x45)),
        ("impact", sprite(0x14, 0x01)),
        ("hit-marker", sprite(0x14, 0x07)),
        ("shot-impact", sprite(0x14, 0x0C)),
        ("shell-burst", sprite(0x14, 0x11)),
        ("beast-shot", sprite(0x0C, 0x21)),
        ("bow", sprite(0x0C, 0x2A)),
        ("lil-boiler", sprite(0x04, 0x0D)),
        ("voodoo-doll", sprite(0x0C, 0x34)),
        ("crack-shot", sprite(0x0C, 0x33)),
        ("dust", sprite(0x14, 0x0E)),
        ("reflector-shield", sprite(0x0C, 0x1B)),
        ("dummy-shield", sprite(0x04, 0x00)),
        ("megaman-navi", sprite(0x08, 0x00)),
        ("drill-arm", sprite(0x0C, 0x58)),
        ("burner", sprite(0x0C, 0x29)),
        ("heatcross-burner", sprite(0x0C, 0x1F)),
        ("flame", sprite(0x0C, 0x0E)),
        ("boomerang", sprite(0x10, 0x07)),
        ("boomerang-tomahawk", sprite(0x10, 0x57)),
        ("lance", sprite(0x0C, 0x44)),
        ("fire-hit", sprite(0x14, 0x1A)),
        ("sand-worm", sprite(0x04, 0x1A)),
        ("sand-hole", sprite(0x10, 0x48)),
        ("justice-one", sprite(0x0C, 0x62)),
        ("golem", sprite(0x10, 0x30)),
        ("dust-2", sprite(0x10, 0x2C)),
        ("rock-cubes", sprite(0x10, 0x00)),
        ("rock-debris", sprite(0x10, 0x01)),
        ("boulder", sprite(0x10, 0x08)),
        ("falling-rock", sprite(0x10, 0x05)),
        ("countdown-bomb", sprite(0x0C, 0x23)),
        ("heat-flame", sprite(0x10, 0x02)),
        ("follow-effect", sprite(0x10, 0x0E)),
        ("drip-shower", sprite(0x10, 0x22)),
        ("eagle-tomahawk", sprite(0x10, 0x2A)),
        ("tengu-tornado", sprite(0x10, 0x42)),
        ("volcano-rock", sprite(0x10, 0x55)),
        ("ground-drill", sprite(0x10, 0x4F)),
        ("dust-cloud", sprite(0x10, 0x59)),
        ("dustman", sprite(0x08, 0x0A)),
        ("swirl", sprite(0x0C, 0x28)),
        // The traps, mines, time bombs and navi-changing chips.
        ("spout-splash", sprite(0x0C, 0x1A)),
        ("lightning", sprite(0x14, 0x14)),
        ("land-mine", sprite(0x0C, 0x22)),
        ("blast", sprite(0x14, 0x13)),
        ("hub", sprite(0x14, 0x1E)),
        ("bug", sprite(0x14, 0x1F)),
        ("charge-glow-a", sprite(0x14, 0x15)),
        // The waves and pillars.
        ("slash-wave", sprite(0x10, 0x39)),
        ("charged-slash", sprite(0x10, 0x3B)),
        ("moon-blade", sprite(0x10, 0x3C)),
        ("element-pillar-flames", sprite(0x0C, 0x1C)),
        ("element-pillar-lightning", sprite(0x10, 0x32)),
        // The supports, and the barrier Tango's heal raises.
        ("rush", sprite(0x0C, 0x48)),
        ("beat", sprite(0x0C, 0x4B)),
        ("tango", sprite(0x0C, 0x4C)),
        ("tango-heal", sprite(0x0C, 0x4D)),
        ("heal", sprite(0x0C, 0x12)),
        // SunMoon and its meteors.
        ("moon-beam", sprite(0x0C, 0x64)),
        ("meteor", sprite(0x0C, 0x31)),
    ] {
        a.sprites.insert(name.into(), id);
    }
    for (name, id) in [
        ("test-tick", 0x1A6),
        ("minibomb-throw", 0xB2),
        ("hit-bomb-1", 0x70),
        ("panel-poison", 0x90),
        ("freeze", 0x118),
        ("grass", 0x11B),
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
        ("place", 0x112),
        ("falling-rock", 0xD9),
        ("erase-man", 0x10E),
        ("erase-man-2", 0xBA),
        ("hub", 0x119),
        ("buster-shot", 0x6A),
        ("junk-shot", 0xFF),
        ("gundels1", 0xF8),
        ("bonus", 0x157),
        ("barrier", 0x89),
        ("beast-over", 0x19A),
        ("bubble", 0x12D),
        ("bubble-pop", 0x124),
        ("bug", 0x143),
        ("bugfix-flash", 0xD1),
        ("cross-merge", 0x8C),
        ("discord", 0xA9),
        ("fanfare", 0xA8),
        ("invisible", 0x93),
        ("log-in", 0x77),
        ("panel-change", 0xA4),
        ("panel-change-tick", 0xA3),
        ("point-appear", 0x129),
        ("point-rise", 0x12A),
        ("silence", 0xAB),
        ("take-off", 0x1A9),
        ("timpani", 0xAA),
        ("tomahawk-man", 0x10A),
        ("flame-hook-fire", 0x158),
        ("twang", 0x18A),
        ("boiler-erupt", 0x184),
        ("boiler-steam", 0x185),
        ("err-select-91", 0x91),
        ("hit-bomb-0", 0x6F),
        ("crack-shot", 0xDA),
        ("bblstar1", 0xD8),
        ("follow-effect", 0xA0),
        ("wave", 0xC5),
        ("roar", 0x12B),
        ("cross-change", 0x8D),
        ("boomerang", 0xB7),
        ("fire-hit", 0xED),
        ("sand-worm", 0xE1),
        ("sand-worm-2", 0x1BE),
        ("justice-one", 0xC4),
        ("golem", 0x10D),
        ("golem-2", 0x188),
        ("place", 0x112),
        ("panel-crack", 0x97),
        ("falling-rock", 0xD9),
        ("form-change", 0xF7),
        ("follow-effect", 0xA0),
        ("drip-shower", 0x128),
        ("etomahwk", 0x10C),
        ("aqua-surge", 0xB8),
        ("volcano", 0x146),
        ("rslash", 0x164),
        ("dustbrk", 0xAD),
        ("dustbrk-2", 0x17B),
        ("rockfall", 0xE5),
        ("drill-spin", 0x1C0),
        // The traps, mines, time bombs, gauge and navi-changing chips.
        ("dimming-sparkle", 0xA5),
        ("target-move", 0x10F),
        ("spout-ball", 0x11D),
        ("beast-over-burst", 0x12E),
        ("hop", 0x113),
        ("tick", 0xC1),
        ("last", 0xC2),
        // The waves.
        ("ok-8b", 0x8B),
        ("aqua-needle-2", 0xB3),
        // The supports, and the barrier Tango's heal raises.
        ("bite", 0x122),
        ("set-down", 0x120),
        ("snatch", 0x126),
        ("arrive", 0x116),
        ("tango-land", 0xD4),
        ("heal", 0x8A),
        // SunMoon.
        ("sun-moon", 0x110),
        ("moon-beam", 0x111),
        ("blast-man", 0x17F),
    ] {
        a.sounds.insert(name.into(), id);
    }
    standard_chip_assets(&mut a);
    form_weapon_assets(&mut a);
    a
}

/// The asset names MegaMan's weapon definitions and the forms' kinds use
/// (content model v2, step 8e), with BN6's numbers.
fn form_weapon_assets(a: &mut nettai_content_api::PackIndex) {
    let sprite = |c, i| PackSprite { category: c, index: i };
    for (name, id) in [
        ("aqua-surge", sprite(0x10, 0x2E)),
        ("whirlwind", sprite(0x10, 0x44)),
        ("erase-ray", sprite(0x10, 0x4C)),
        ("erase-drop", sprite(0x10, 0x4D)),
        ("charge-wave", sprite(0x10, 0x09)),
        ("junk-shot", sprite(0x10, 0x56)),
        ("ground-drill-effect", sprite(0x0C, 0x2D)),
        ("slash-man-effect", sprite(0x10, 0x38)),
        ("groundman", sprite(0x08, 0x09)),
    ] {
        a.sprites.insert(name.into(), id);
    }
    for (name, id) in [
        ("aqua-needle-2", 0xB3),
        ("col-army-2", 0xB9),
        ("spout-beast-charge", 0xF4),
        ("moon-beam", 0x111),
        ("tomahawk-man", 0x10A),
        ("ground-beast-dash", 0x1BF),
        ("ground-beast-dash-2", 0x1C7),
        ("iron-shell", 0x187),
        ("beast-claw", 0x1C5),
        ("beast-claw-2", 0x1C6),
        ("charge-train", 0xE4),
        ("tenguman-nose", 0xFB),
        ("drill-launch", 0x14C),
        ("spin", 0xC7),
        ("drilarm", 0xF0),
    ] {
        a.sounds.insert(name.into(), id);
    }
}

/// The asset names the standard chip actions' modules use (content model
/// v2, step 8g), with BN6's numbers.
fn standard_chip_assets(a: &mut nettai_content_api::PackIndex) {
    let sprite = |c, i| PackSprite { category: c, index: i };
    for (name, id) in [
        ("gust", sprite(0x0C, 0x2E)),
        ("wind-rack", sprite(0x0C, 0x27)),
        ("tengu-fan", sprite(0x0C, 0x5C)),
        ("swirl", sprite(0x0C, 0x28)),
        ("thunder-doll", sprite(0x10, 0x12)),
        ("thunder-doll-hand", sprite(0x0C, 0x60)),
        ("gun-del-sol", sprite(0x0C, 0x3B)),
        ("sun-beam", sprite(0x0C, 0x3C)),
        ("sun-beam-ex", sprite(0x0C, 0x47)),
        ("hive", sprite(0x0C, 0x5E)),
        ("honey-bee", sprite(0x10, 0x31)),
        ("dragon", sprite(0x04, 0x10)),
    ] {
        a.sprites.insert(name.into(), id);
    }
    navi_chip_assets(a);
    for (name, id) in
        [("windrack", 0x11F), ("beast-over", 0x19A), ("sun-beam", 0xF9), ("buzz", 0x1A8), ("form-change", 0xF7)]
    {
        a.sounds.insert(name.into(), id);
    }
}

/// The asset names the navi chips' modules use (content model v2), with
/// BN6's numbers.
fn navi_chip_assets(a: &mut nettai_content_api::PackIndex) {
    let sprite = |c, i| PackSprite { category: c, index: i };
    for (name, id) in [
        ("bass-anly", sprite(0x08, 0x13)),
        ("lightning", sprite(0x14, 0x14)),
        ("blast-fire", sprite(0x08, 0x0C)),
        ("charge-car", sprite(0x10, 0x54)),
        ("chargeman", sprite(0x08, 0x05)),
        ("elecman", sprite(0x08, 0x02)),
        ("elmnt-ice", sprite(0x10, 0x0F)),
        ("elmnt-man", sprite(0x08, 0x10)),
        ("heatman", sprite(0x08, 0x01)),
        ("meteor", sprite(0x0C, 0x31)),
        ("moon-beam", sprite(0x0C, 0x64)),
        ("panel-strike", sprite(0x10, 0x26)),
        ("slash-man-effect", sprite(0x10, 0x38)),
        ("slash-wave", sprite(0x10, 0x39)),
        ("slashman", sprite(0x08, 0x03)),
        ("spout-geyser", sprite(0x10, 0x20)),
        ("spout-man-effect", sprite(0x10, 0x23)),
        ("spout-pillar", sprite(0x10, 0x1F)),
        ("spout-splash", sprite(0x0C, 0x1A)),
        ("spoutman", sprite(0x08, 0x06)),
        ("tenguman", sprite(0x08, 0x08)),
        ("tomahawkman", sprite(0x08, 0x07)),
        ("count", sprite(0x08, 0x16)),
        ("django", sprite(0x0C, 0x0F)),
        ("otenko", sprite(0x0C, 0x49)),
        ("dust-storm-mote", sprite(0x10, 0x10)),
    ] {
        a.sprites.insert(name.into(), id);
    }
    for (name, id) in [
        ("aqua-needle-2", 0xB3),
        ("beast-over-burst", 0x12E),
        ("blast-man", 0x17F),
        ("bubble", 0x12D),
        ("charge-man", 0xE3),
        ("col-army-2", 0xB9),
        ("elec-man", 0xC6),
        ("elmnt-man", 0x134),
        ("elmnt-man-2", 0x182),
        ("elmnt-man-4", 0x99),
        ("elmnt-vine", 0x181),
        ("moon-beam", 0x111),
        ("ok-8b", 0x8B),
        ("spout-ball", 0x11D),
        ("spout-man", 0x189),
        ("sun-moon", 0x110),
        ("tengu-man", 0x13C),
        ("tomahawk-man", 0x10A),
    ] {
        a.sounds.insert(name.into(), id);
    }
}

/// Where the BN6 scripts are (the source overlay in this repository).
const OVERLAY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");

/// The behavior BN5 and BN6 share (content/exelib), which BN6's modules
/// require by `@exelib/...`: a support pack beside the test content.
const EXELIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/exelib");

/// The support pack's name.
pub const EXELIB_PACK: &str = "exelib";

/// The test content's scripts: its own modules (testdata/content), these
/// modules of the BN6 overlay, and whatever they `require` of it.
pub fn scripts() -> Scripts {
    static SCRIPTS: std::sync::OnceLock<Scripts> = std::sync::OnceLock::new();
    SCRIPTS
        .get_or_init(|| {
            let read = |path: &str| {
                let file = format!("{OVERLAY}/{path}.luau");
                std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"))
            };
            let modules = [
                ("objects/attachment/attachment", "objects/attachment/attachment"),
                // GunDelSol: the chips, whose actions the SunGuns run.
                ("chips/gundels/beam", "chips/gundels/beam"),
                ("chips/gundels/action", "chips/gundels/action"),
                ("chips/gundels/chips", "chips/gundels/chips"),
                ("chips/eraseman/mark", "chips/eraseman/mark"),
                ("chips/eraseman/beam", "chips/eraseman/beam"),
                ("chips/eraseman/navi", "chips/eraseman/navi"),
                ("lib/grab/shot", "lib/grab/shot"),
                ("lib/grab/controller", "lib/grab/controller"),
                ("chips/areagrab/chip", "chips/areagrab/chip"),
                ("chips/panlgrab/chip", "chips/panlgrab/chip"),
                ("objects/falling-rock/falling_rock", "objects/falling-rock/falling_rock"),
                ("objects/falling-rock/chip", "objects/falling-rock/chip"),
                ("objects/projectile/projectile", "objects/projectile/projectile"),
                ("objects/projectile/variants", "objects/projectile/variants"),
                ("objects/flying-shot/flying_shot", "objects/flying-shot/flying_shot"),
                ("lib/buster", "lib/buster"),
                // MegaMan's buster, charged and blank shots and HeatCross's
                // charged shot are weapon definitions. (BN6's rules/roles
                // isn't here: the test pack fills the roles.)
                ("navis/megaman/weapons/blank-shot/weapon", "navis/megaman/weapons/blank-shot/weapon"),
                ("navis/megaman/weapons/charged-shot/weapon", "navis/megaman/weapons/charged-shot/weapon"),
                ("navis/megaman/weapons/buster/weapon", "navis/megaman/weapons/buster/weapon"),
                // Two of the buster's alias routines (its setup, their own
                // charge rows).
                ("navis/megaman/weapons/buster-2e/weapon", "navis/megaman/weapons/buster-2e/weapon"),
                ("navis/megaman/weapons/buster-82/weapon", "navis/megaman/weapons/buster-82/weapon"),
                ("navis/megaman/forms/heatcross/charge", "navis/megaman/forms/heatcross/charge"),
                ("lib/weapon", "lib/weapon"),
                ("objects/element-pillar/element_pillar", "objects/element-pillar/element_pillar"),
                // The form weapons content defines, with the kinds only they
                // spawn.
                (
                    "navis/megaman/weapons/falzar-beast-buster/weapon",
                    "navis/megaman/weapons/falzar-beast-buster/weapon",
                ),
                (
                    "navis/megaman/weapons/gregar-beast-buster/weapon",
                    "navis/megaman/weapons/gregar-beast-buster/weapon",
                ),
                ("navis/megaman/weapons/tengu-wind/weapon", "navis/megaman/weapons/tengu-wind/weapon"),
                ("navis/megaman/forms/spoutcross-beast/surge", "navis/megaman/forms/spoutcross-beast/surge"),
                ("navis/megaman/forms/spoutcross-beast/charge", "navis/megaman/forms/spoutcross-beast/charge"),
                ("navis/megaman/forms/tengucross-beast/whirlwind", "navis/megaman/forms/tengucross-beast/whirlwind"),
                ("navis/megaman/forms/tengucross-beast/charge", "navis/megaman/forms/tengucross-beast/charge"),
                ("navis/megaman/forms/eleccross/charge", "navis/megaman/forms/eleccross/charge"),
                ("navis/megaman/forms/eleccross/a-charge", "navis/megaman/forms/eleccross/a-charge"),
                ("navis/megaman/forms/tengucross/charge", "navis/megaman/forms/tengucross/charge"),
                ("navis/megaman/forms/dustcross/throw_absorbed", "navis/megaman/forms/dustcross/throw_absorbed"),
                (
                    "navis/megaman/forms/dustcross-beast/throw_absorbed",
                    "navis/megaman/forms/dustcross-beast/throw_absorbed",
                ),
                ("navis/megaman/forms/erasecross/ray", "navis/megaman/forms/erasecross/ray"),
                ("navis/megaman/forms/erasecross/charge", "navis/megaman/forms/erasecross/charge"),
                (
                    "navis/megaman/forms/erasecross-beast/erase_drop",
                    "navis/megaman/forms/erasecross-beast/erase_drop",
                ),
                ("navis/megaman/forms/erasecross-beast/drop", "navis/megaman/forms/erasecross-beast/drop"),
                ("navis/megaman/forms/tomahawkcross/charge", "navis/megaman/forms/tomahawkcross/charge"),
                (
                    "navis/megaman/forms/tomahawkcross-beast/throw",
                    "navis/megaman/forms/tomahawkcross-beast/throw",
                ),
                (
                    "navis/megaman/forms/slashcross-beast/lunge_slash",
                    "navis/megaman/forms/slashcross-beast/lunge_slash",
                ),
                ("navis/megaman/forms/slashcross-beast/lunge", "navis/megaman/forms/slashcross-beast/lunge"),
                ("navis/megaman/dash_hit", "navis/megaman/dash_hit"),
                ("navis/megaman/forms/groundcross-beast/dash", "navis/megaman/forms/groundcross-beast/dash"),
                ("navis/megaman/forms/groundcross/drill", "navis/megaman/forms/groundcross/drill"),
                ("navis/megaman/forms/chargecross/tackle", "navis/megaman/forms/chargecross/tackle"),
                (
                    "navis/megaman/forms/chargecross-beast/charge_wave",
                    "navis/megaman/forms/chargecross-beast/charge_wave",
                ),
                ("navis/megaman/forms/chargecross-beast/wave", "navis/megaman/forms/chargecross-beast/wave"),
                (
                    "navis/megaman/forms/dustcross-beast/junk_shot",
                    "navis/megaman/forms/dustcross-beast/junk_shot",
                ),
                ("navis/megaman/forms/dustcross-beast/scatter", "navis/megaman/forms/dustcross-beast/scatter"),
                ("navis/megaman/forms/dustcross/junk_ball", "navis/megaman/forms/dustcross/junk_ball"),
                ("navis/megaman/forms/dustcross/charge", "navis/megaman/forms/dustcross/charge"),
                ("navis/megaman/weapons/beast-claw/weapon", "navis/megaman/weapons/beast-claw/weapon"),
                ("navis/megaman/weapons/absorb/weapon", "navis/megaman/weapons/absorb/weapon"),
                (
                    "navis/megaman/forms/slashcross-beast/hit_flash",
                    "navis/megaman/forms/slashcross-beast/hit_flash",
                ),
                ("objects/absorbed-obstacle/absorbed_obstacle", "objects/absorbed-obstacle/absorbed_obstacle"),
                // The instant chips: BN6's definitions, and the effects
                // the test chips compose (the plus chips', FireHit's fist,
                // FlmHook's hook).
                ("lib/instant/plus", "lib/instant/plus"),
                // The chips the ruleset names: the custom screen's Beast Out
                // button, and what a selection it can't allow becomes.
                ("chips/beastout/chip", "chips/beastout/chip"),
                ("chips/invalid/chip", "chips/invalid/chip"),
                ("chips/atk-10/chip", "chips/atk-10/chip"),
                ("chips/navi-20/chip", "chips/navi-20/chip"),
                ("chips/busterup/chip", "chips/busterup/chip"),
                ("chips/fullcust/chip", "chips/fullcust/chip"),
                ("chips/synctrgr/chip", "chips/synctrgr/chip"),
                ("chips/boomer/boomerang", "chips/boomer/boomerang"),
                ("chips/boomer/chips", "chips/boomer/chips"),
                ("chips/lance/lance", "chips/lance/lance"),
                ("chips/lance/chip", "chips/lance/chip"),
                ("chips/firehit/fist", "chips/firehit/fist"),
                ("chips/sandwrm/worm", "chips/sandwrm/worm"),
                ("chips/sandwrm/hole", "chips/sandwrm/hole"),
                ("chips/sandwrm/spray", "chips/sandwrm/spray"),
                ("chips/sandwrm/chips", "chips/sandwrm/chips"),
                ("chips/flmhook/fire", "chips/flmhook/fire"),
                ("chips/flmhook/hook", "chips/flmhook/hook"),
                ("chips/justcone/strike", "chips/justcone/strike"),
                ("chips/justcone/chip", "chips/justcone/chip"),
                ("chips/golmhit/golem", "chips/golmhit/golem"),
                ("chips/golmhit/chips", "chips/golmhit/chips"),
                ("lib/projectile", "lib/projectile"),
                ("objects/gust/gust", "objects/gust/gust"),
                // WindRack's action, which TenguCross's charged shot swings
                // with its fan, and DolThdr's, which ElecCross's strikes
                // with (content model v2).
                ("lib/arm", "lib/arm"),
                ("chips/windrack/action", "chips/windrack/action"),
                ("chips/dolthdr/action", "chips/dolthdr/action"),
                ("chips/dolthdr/doll", "chips/dolthdr/doll"),
                // SlashCross's charged slash: its waves, the slashes the
                // swords name, its charged shot and A-charge; and the
                // Beast charged chips' pillars' weapons.
                (
                    "navis/megaman/forms/slashcross/sword_wave",
                    "navis/megaman/forms/slashcross/sword_wave",
                ),
                ("navis/megaman/forms/slashcross/slashes", "navis/megaman/forms/slashcross/slashes"),
                ("navis/megaman/forms/slashcross/charge", "navis/megaman/forms/slashcross/charge"),
                (
                    "navis/megaman/weapons/slash-a-charge/weapon",
                    "navis/megaman/weapons/slash-a-charge/weapon",
                ),
                ("navis/megaman/forms/heatcross-beast/charge", "navis/megaman/forms/heatcross-beast/charge"),
                ("navis/megaman/forms/eleccross-beast/charge", "navis/megaman/forms/eleccross-beast/charge"),
                // The Reflectors, the recovery chips and HeatCross's charged
                // shot's burner (content model v2).
                ("chips/rflectr/shield", "chips/rflectr/shield"),
                ("chips/rflectr/shot", "chips/rflectr/shot"),
                ("chips/rflectr/guard", "chips/rflectr/guard"),
                ("chips/rflectr/chips", "chips/rflectr/chips"),
                ("chips/recov/chips", "chips/recov/chips"),
                ("lib/burner/burn", "lib/burner/burn"),
                ("lib/burner/flame", "lib/burner/flame"),
                ("lib/effects", "lib/effects"),
                ("lib/sparks", "lib/sparks"),
                ("rules/collision", "rules/collision"),
                // BN6's Beast Out turns, a system of the test rules.
                ("rules/beast/system", "rules/beast/system"),
                ("rules/emotion/system", "rules/emotion/system"),
                ("rules/beast/rush", "rules/beast/rush"),
                ("rules/beast/berserk", "rules/beast/berserk"),
                // (Its chips are the test content's own: testdata's
                // rules/cross-special.luau.)
                ("rules/beast/cross-special", "rules/beast/cross-special"),
                // (Its tables are the test content's own, the same as BN6's.)
                ("rules/berserk", "rules/berserk"),
                ("lib/trajectory", "lib/trajectory"),
                ("lib/hp", "lib/hp"),
                // BN6's chip gate battle (battle flag 0x40), which the
                // Beast Out button and the beast buster test.
                ("lib/chip_gate_battle", "lib/chip_gate_battle"),
                // The bombs and seeds: the chips, whose actions the test
                // chips run.
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
                // RskyHny and the dragons (content model v2): definitions.
                ("chips/rskyhny/bee", "chips/rskyhny/bee"),
                ("chips/rskyhny/action", "chips/rskyhny/action"),
                ("chips/rskyhny/chips", "chips/rskyhny/chips"),
                ("lib/dragons/dragon", "lib/dragons/dragon"),
                ("lib/dragons/body", "lib/dragons/body"),
                ("lib/dragons/head", "lib/dragons/head"),
                ("lib/dragons/action", "lib/dragons/action"),
                ("chips/elecdrgn/chip", "chips/elecdrgn/chip"),
                // The swords: the chips, whose slashes and strikes the
                // test chips run.
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
                ("chips/mchnswrd/chip", "chips/mchnswrd/chip"),
                ("chips/elemswrd/chip", "chips/elemswrd/chip"),
                ("chips/assnswrd/chip", "chips/assnswrd/chip"),
                // Invisibl's hook, which the veil composes.
                ("chips/invisibl/controller", "chips/invisibl/controller"),
                ("chips/invisibl/chip", "chips/invisibl/chip"),
                ("chips/whicapsl-invisible/chip", "chips/whicapsl-invisible/chip"),
                // The field objects (content model v2): the rock with its
                // debris, RockCube and IceCube, and the stages' boulder.
                ("chips/rockcube/rock", "chips/rockcube/rock"),
                ("chips/rockcube/debris", "chips/rockcube/debris"),
                ("chips/rockcube/cube", "chips/rockcube/cube"),
                ("chips/rockcube/chips", "chips/rockcube/chips"),
                ("objects/boulder/boulder", "objects/boulder/boulder"),
                ("objects/encased-bubble/bubble", "objects/encased-bubble/bubble"),
                // The NaviCust supports (content model v2): the controller the
                // ruleset spawns by role, Rush, Beat, Tango and her heal, with
                // the barrier it raises.
                ("lib/barriers/visual", "lib/barriers/visual"),
                ("lib/barriers/barriers", "lib/barriers/barriers"),
                ("lib/supports/heal", "lib/supports/heal"),
                ("lib/supports/tango", "lib/supports/tango"),
                ("lib/supports/beat", "lib/supports/beat"),
                ("lib/supports/rush", "lib/supports/rush"),
                ("lib/supports/controller", "lib/supports/controller"),
                // The trap chips, the navi-changing chips and the gauge
                // chips: the test trap and boosts compose their hooks;
                // SloGauge is BN6's.
                ("lib/traps/controller", "lib/traps/controller"),
                ("chips/antinavi/chip", "chips/antinavi/chip"),
                ("chips/antidmg/chip", "chips/antidmg/chip"),
                ("chips/antiswrd/chip", "chips/antiswrd/chip"),
                ("chips/antirecv/chip", "chips/antirecv/chip"),
                ("chips/bodygrd/chip", "chips/bodygrd/chip"),
                // (HubBatc gives the NaviCust Shield as a B+Back special.)
                ("navis/megaman/weapons/shield/weapon", "navis/megaman/weapons/shield/weapon"),
                ("lib/navi-boost/controller", "lib/navi-boost/controller"),
                ("chips/darkinvs/chip", "chips/darkinvs/chip"),
                // HubBatc, and an arm chip with the weapon it makes the
                // charged shot.
                ("chips/hubbatc/chip", "chips/hubbatc/chip"),
                // (The chips that make their own weapon the charged shot,
                // with the chips' parts those weapons fire.)
                ("lib/navi-boost/charge", "lib/navi-boost/charge"),
                ("chips/puncharm/charge", "chips/puncharm/charge"),
                ("chips/puncharm/chip", "chips/puncharm/chip"),
                ("chips/bugrswrd/charge", "chips/bugrswrd/charge"),
                ("chips/bugrswrd/chip", "chips/bugrswrd/chip"),
                ("chips/thunder/ball", "chips/thunder/ball"),
                ("chips/thunder/shoot", "chips/thunder/shoot"),
                ("chips/thunder/chip", "chips/thunder/chip"),
                ("chips/bgdththd/charge", "chips/bgdththd/charge"),
                ("chips/bgdththd/chip", "chips/bgdththd/chip"),
                ("chips/aquandl/needle", "chips/aquandl/needle"),
                ("chips/aquandl/volley", "chips/aquandl/volley"),
                ("chips/aquandl/action", "chips/aquandl/action"),
                ("chips/aquandl/chips", "chips/aquandl/chips"),
                ("chips/needlarm/charge", "chips/needlarm/charge"),
                ("chips/needlarm/chip", "chips/needlarm/chip"),
                ("chips/elcpuls/pulse", "chips/elcpuls/pulse"),
                ("chips/elcpuls/action", "chips/elcpuls/action"),
                ("chips/puzzlarm/charge", "chips/puzzlarm/charge"),
                ("chips/puzzlarm/chip", "chips/puzzlarm/chip"),
                ("chips/boomrarm/charge", "chips/boomrarm/charge"),
                ("chips/boomrarm/chip", "chips/boomrarm/chip"),
                ("lib/gauge-speed/controller", "lib/gauge-speed/controller"),
                ("chips/slogauge/chip", "chips/slogauge/chip"),
                // Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
                ("objects/rising-bubble/rising_bubble", "objects/rising-bubble/rising_bubble"),
                // ElemTrap, the time bombs and Mine: BN6's definitions,
                // whose hooks the test traps and time bombs run.
                ("chips/elemtrap/trap", "chips/elemtrap/trap"),
                ("chips/elemtrap/strike", "chips/elemtrap/strike"),
                ("chips/elemtrap/chip", "chips/elemtrap/chip"),
                ("objects/panel-bursts/panel_bursts", "objects/panel-bursts/panel_bursts"),
                ("chips/timebom/controller", "chips/timebom/controller"),
                ("chips/timebom/countdown", "chips/timebom/countdown"),
                ("chips/timebom/chips", "chips/timebom/chips"),
                ("chips/mine/controller", "chips/mine/controller"),
                ("chips/mine/land_mine", "chips/mine/land_mine"),
                ("chips/mine/chip", "chips/mine/chip"),
                ("chips/crakshot/shot", "chips/crakshot/shot"),
                ("chips/crakshot/chips", "chips/crakshot/chips"),
                // The navi chips' navis: each navi and his kinds, which
                // the test navi chips summon.
                ("lib/navi-chips/navi", "lib/navi-chips/navi"),
                ("chips/elmntman/navi", "chips/elmntman/navi"),
                ("chips/elmntman/meteor", "chips/elmntman/meteor"),
                ("chips/elmntman/ice", "chips/elmntman/ice"),
                ("chips/elmntman/bolt", "chips/elmntman/bolt"),
                ("chips/elmntman/vine", "chips/elmntman/vine"),
                ("chips/spoutman/navi", "chips/spoutman/navi"),
                ("chips/spoutman/ball", "chips/spoutman/ball"),
                ("chips/spoutman/splash", "chips/spoutman/splash"),
                ("chips/spoutman/pillar", "chips/spoutman/pillar"),
                ("chips/spoutman/geyser", "chips/spoutman/geyser"),
                ("chips/spoutman/mark", "chips/spoutman/mark"),
                ("chips/heatman/navi", "chips/heatman/navi"),
                ("chips/heatman/flame", "chips/heatman/flame"),
                ("chips/elecman/navi", "chips/elecman/navi"),
                ("chips/elecman/thunder", "chips/elecman/thunder"),
                ("chips/slashman/navi", "chips/slashman/navi"),
                ("chips/slashman/wave", "chips/slashman/wave"),
                ("chips/chrgeman/navi", "chips/chrgeman/navi"),
                ("chips/chrgeman/car", "chips/chrgeman/car"),
                ("chips/tmhkman/navi", "chips/tmhkman/navi"),
                ("chips/tenguman/navi", "chips/tenguman/navi"),
                ("chips/blastman/navi", "chips/blastman/navi"),
                ("chips/blastman/fire", "chips/blastman/fire"),
                ("chips/bass/navi", "chips/bass/navi"),
                ("chips/sunmoon/sun", "chips/sunmoon/sun"),
                ("chips/sunmoon/meteor", "chips/sunmoon/meteor"),
                ("chips/sunmoon/moon_beam", "chips/sunmoon/moon_beam"),
                // The Japanese games' Count and Django chips, with their
                // navis, Count's lance and his rain (a dust storm).
                ("chips/count/chips", "chips/count/chips"),
                ("chips/django/chips", "chips/django/chips"),
                // The link navis' own chips (whose actions the test link
                // chips run) and their kinds.
                ("lib/link_chips", "lib/link_chips"),
                ("objects/follow-effect/follow_effect", "objects/follow-effect/follow_effect"),
                ("navis/heatman/chip", "navis/heatman/chip"),
                ("navis/elecman/chip", "navis/elecman/chip"),
                ("navis/slashman/chip", "navis/slashman/chip"),
                ("navis/slashman/riding_hit", "navis/slashman/riding_hit"),
                ("navis/eraseman/chip", "navis/eraseman/chip"),
                ("navis/chargeman/chip", "navis/chargeman/chip"),
                ("navis/chargeman/volcano_rock", "navis/chargeman/volcano_rock"),
                ("navis/spoutman/chip", "navis/spoutman/chip"),
                ("navis/spoutman/drip_shower", "navis/spoutman/drip_shower"),
                ("navis/tomahawkman/chip", "navis/tomahawkman/chip"),
                ("navis/tomahawkman/axe", "navis/tomahawkman/axe"),
                ("navis/tomahawkman/strike", "navis/tomahawkman/strike"),
                ("navis/tenguman/chip", "navis/tenguman/chip"),
                ("navis/tenguman/tornado", "navis/tenguman/tornado"),
                ("navis/dustman/chip", "navis/dustman/chip"),
                // Their charged attacks (weapon definitions), and the
                // drills GroundMan's throws.
                ("navis/heatman/charge", "navis/heatman/charge"),
                ("navis/elecman/charge", "navis/elecman/charge"),
                ("navis/slashman/charge", "navis/slashman/charge"),
                ("navis/eraseman/charge", "navis/eraseman/charge"),
                ("navis/chargeman/charge", "navis/chargeman/charge"),
                ("navis/spoutman/charge", "navis/spoutman/charge"),
                ("navis/tomahawkman/charge", "navis/tomahawkman/charge"),
                ("navis/tenguman/charge", "navis/tenguman/charge"),
                ("navis/groundman/charge", "navis/groundman/charge"),
                ("navis/dustman/charge", "navis/dustman/charge"),
                ("navis/groundman/drill", "navis/groundman/drill"),
                ("navis/dustman/clouds", "navis/dustman/clouds"),
                ("navis/groundman/chip", "navis/groundman/chip"),
                ("chips/grndman/drill", "chips/grndman/drill"),
                ("chips/grndman/rock", "chips/grndman/rock"),
                ("objects/panel-strike/panel_strike", "objects/panel-strike/panel_strike"),
                ("chips/drilarm/drill", "chips/drilarm/drill"),
                ("chips/dolthdr/column", "chips/dolthdr/column"),
                // The dimming chips of subtypes 4, 5, 9, 13, 26, 27, 28 and 36
                // (content model v2): the barriers, the panel chips, the
                // instruments, AirRaid, BugFix, ColorPt, Sensor and SumnBlk.
                ("lib/barriers/controller", "lib/barriers/controller"),
                ("chips/barrier/chips", "chips/barrier/chips"),
                ("chips/bblwrap/chip", "chips/bblwrap/chip"),
                ("chips/lifeaur/chip", "chips/lifeaur/chip"),
                ("chips/bugfix/chip", "chips/bugfix/chip"),
                ("chips/bugfix/controller", "chips/bugfix/controller"),
                ("chips/bugfix/glow", "chips/bugfix/glow"),
                ("objects/panel-changer/panel_changer", "objects/panel-changer/panel_changer"),
                ("lib/panel-chips/controller", "lib/panel-chips/controller"),
                ("chips/pnlretrn/chip", "chips/pnlretrn/chip"),
                ("chips/holypanl/chip", "chips/holypanl/chip"),
                ("chips/snctuary/chip", "chips/snctuary/chip"),
                ("chips/comingrd/chip", "chips/comingrd/chip"),
                ("chips/goingrd/chip", "chips/goingrd/chip"),
                ("lib/instruments/instrument", "lib/instruments/instrument"),
                ("lib/instruments/controller", "lib/instruments/controller"),
                ("chips/fanfare/chip", "chips/fanfare/chip"),
                ("chips/discord/chip", "chips/discord/chip"),
                ("chips/timpani/chip", "chips/timpani/chip"),
                ("chips/silence/chip", "chips/silence/chip"),
                ("chips/sensor/chips", "chips/sensor/chips"),
                ("chips/sensor/controller", "chips/sensor/controller"),
                ("chips/sensor/turret", "chips/sensor/turret"),
                ("chips/sensor/scanner", "chips/sensor/scanner"),
                ("chips/sensor/laser", "chips/sensor/laser"),
                ("chips/airraid/chips", "chips/airraid/chips"),
                ("chips/airraid/controller", "chips/airraid/controller"),
                ("chips/airraid/plane", "chips/airraid/plane"),
                ("chips/airraid/propeller", "chips/airraid/propeller"),
                ("chips/sumnblk/chips", "chips/sumnblk/chips"),
                ("chips/sumnblk/controller", "chips/sumnblk/controller"),
                ("chips/sumnblk/navi", "chips/sumnblk/navi"),
                ("chips/colorpt/chips", "chips/colorpt/chips"),
                ("chips/colorpt/controller", "chips/colorpt/controller"),
                ("chips/colorpt/point", "chips/colorpt/point"),
                // The Gregar and Falzar chips (the Japanese ROMs' routines).
                ("chips/gregar/chip", "chips/gregar/chip"),
                ("chips/falzar/chip", "chips/falzar/chip"),
            ];
            let modules = modules.iter().map(|&(to, from)| (to.to_string(), from.to_string()));
            let own = modules_under(TEST_CONTENT);
            let mut all: std::collections::BTreeMap<String, String> =
                modules.map(|(to, from)| (to, read(&from))).chain(own).collect();
            // What they require of the overlay comes with them (what they
            // require of content/exelib is the support pack's).
            let mut pending: Vec<String> = all.keys().cloned().collect();
            while let Some(module) = pending.pop() {
                for required in requires(&module, &all[&module]) {
                    if !all.contains_key(&required) {
                        all.insert(required.clone(), read(&required));
                        pending.push(required);
                    }
                }
            }
            let mut scripts = Scripts::default();
            add_shared(&mut scripts);
            scripts.add_game(ROOT, all);
            scripts
        })
        .clone()
}

/// The modules `source` (the module `module`) requires: each
/// `require("<relative path>")`, as a module path. (A require of another
/// pack, `@exelib/...`, is that pack's.)
fn requires(module: &str, source: &str) -> Vec<String> {
    let dir: Vec<&str> = module.split('/').collect();
    let dir = &dir[..dir.len() - 1];
    source
        .split("require(\"")
        .skip(1)
        .filter_map(|rest| rest.split_once("\")").map(|(path, _)| path))
        .filter(|path| !path.starts_with('@'))
        .map(|path| {
            let mut parts: Vec<&str> = dir.to_vec();
            for part in path.split('/') {
                match part {
                    "." => {}
                    ".." => {
                        parts.pop();
                    }
                    name => parts.push(name),
                }
            }
            parts.join("/")
        })
        .collect()
}

fn rules() -> Rules {
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
                // BN5's three (docs/design/bn5-map.md §15.2).
                PanelType::Metal => (pflags::SOLID | 0x200, None),
                PanelType::Lava => (pflags::SOLID | 0x1000, None),
                PanelType::Sea => (pflags::SOLID | 0x20000, None),
            };
            // Every panel type is on the field (the step sword looks for
            // this bit). The roads last 0x708 ticks, lava and sea 960 as
            // BN5's do; lava burns for 50, sea drains fire bodies and
            // holds a body that ends a move on it for 20 ticks, and metal
            // slides it as BN5's does.
            let road = t.is_road();
            PanelTypeRule {
                flags: flags | ON_FIELD,
                road_slide,
                trail_sound: None,
                expires: if road { Some(0x708) } else if matches!(t, PanelType::Lava | PanelType::Sea) { Some(960) } else { None },
                burn: (t == PanelType::Lava).then_some(50),
                drains: (t == PanelType::Sea).then_some(1),
                holds: (t == PanelType::Sea).then_some(20),
                submerges: t == PanelType::Sea,
                slide: (t == PanelType::Metal).then(metal_slide),
                cleared_by: match t {
                    PanelType::Grass => Some(1),
                    PanelType::Volcano | PanelType::Lava => Some(2),
                    PanelType::Metal => Some(4),
                    t if t.is_road() => Some(4),
                    _ => None,
                },
                named: true,
            }
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
        panels: PanelRules {
            types,
            start_visible,
            front_edges,
            step,
            dash_step: step,
            any_side_step: StepRuleSet { grounded: [solid(any_side); 2], floor_free: [any_side; 2] },
            mend: 0x258,
            mend_in_battle_mode_1: 0x1E0,
            numbers: Vec::new(),
            reservations: Default::default(),
        },
        holding_banners: vec![BannerId(0x24)],
        // (The statuses are testdata/content/rules/status.luau's.)
        hp_bug_periods: [0, 60, 50, 40, 30, 20, 10, 5],
        form_tick: true,
        flash_hides_on_clear: false,
        reactions: Default::default(),
        emotions: Default::default(),
        form_break: Default::default(),
        intake: Default::default(),
        empty_hand: EmptyHandChip { null_family: false, fire: false, flags: ChipFlags(0x10) },
        buster_recovery: vec![[5, 10, 15, 20, 25, 30], [4, 8, 12, 16, 20, 24], [3, 6, 9, 12, 15, 18], [2, 4, 6, 8, 10, 12], [1, 2, 3, 4, 5, 6]],
        chaos_cycle: Vec::new(),
        sp_deletion_times: vec![0x2000, 0x4000],
        flow: Default::default(),
        effects: Default::default(),
        chip_use: crate::content::ChipUseRules {
            leave_on_use: false,
            anti_navi_sparkle: crate::content::SparkleOffset { dy: 16, z: 32 },
        },
        // The SP navi chips BN6's modules bring: Count[SP].
        sp_slots: vec!["sp/count".into()],
        cross_special: Vec::new(),
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
        push_reading: Default::default(),
        hit_test: Default::default(),
        slide_speed: Default::default(),
        overlay_restart: Default::default(),
        stance_counter: Default::default(),
        // A triangle wave: 256 at a quarter turn, -256 at three quarters,
        // over a turn and a half.
        sine: (0..384)
            .map(|i: i16| {
                let t = i % 256;
                if t < 64 { t * 4 } else if t < 192 { 512 - t * 4 } else { t * 4 - 1024 }
            })
            .collect(),
        // (The modes are testdata/content/rules/lockon.luau's.)
        lockon: Lockon {
            column_shifts: vec![-1, -2],
            clear_path: [PanelCondition { require: 0, forbid: pflags::OCCUPIED }; 2],
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
        pools: Default::default(),
        navicust: Default::default(),
    }
}

/// BN5's metal slide (0x0800C920, 0x0800C9C0): by the direction of the
/// move, the steps tried in turn (forward, back, up, down: dx toward the
/// front).
pub fn metal_slide() -> crate::content::PanelSlide {
    let (f, b, u, d) = (Some((1, 0)), Some((-1, 0)), Some((0, -1)), Some((0, 1)));
    crate::content::PanelSlide { tries: [[None; 4], [f, u, b, d], [b, d, f, u], [u, b, d, f], [d, f, u, b], [d, f, u, b]] }
}

/// The custom screen's grid: five chip slots on top, five below, OK at
/// the top row's right end and a special button under it; the last two
/// bottom slots start hidden. A neighbor that is missing is looked for
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

/// Animation timing for the sprites the tests' battles show: the test
/// pack's, keyed as `assets` names them.
fn animations(assets: &nettai_content_api::AssetNames) -> Animations {
    let mut out = Animations::default();
    let pack = assets.pack(ROOT).expect("the test pack");
    out.add_pack(assets, pack, &pack_animations(), &Default::default());
    out
}

/// The test pack's sprites' timing, by the pack's own ids.
fn pack_animations() -> std::collections::BTreeMap<PackSprite, Vec<Vec<AnimFrame>>> {
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
    // The gun: out, firing, away; the sun beams.
    sprites.insert(PackSprite { category: 0x0C, index: 0x3B }, vec![vec![f(3, 0), f(3, LAST)], vec![f(2, 0), f(2, LAST | LOOP)], once(4)]);
    for index in [0x3C, 0x47] {
        sprites.insert(PackSprite { category: 0x0C, index }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    }
    // The first attachment rows' sprite (fillers).
    sprites.insert(PackSprite { category: 0x0C, index: 0x01 }, vec![once(4)]);
    // Rocks: rising, then standing.
    sprites.insert(
        PackSprite { category: 0x10, index: 0 },
        vec![vec![f(3, 0), f(3, 0), f(3, LAST)], vec![f(30, LAST | LOOP)], vec![f(30, LAST | LOOP)]],
    );
    sprites.insert(PackSprite { category: 0x10, index: 1 }, vec![once(6), once(6), once(6), once(6)]);
    // The stages' boulder.
    sprites.insert(PackSprite { category: 0x10, index: 8 }, vec![vec![f(30, LAST | LOOP)]]);
    // The eraser navi (standing, appearing, leaving, raising, slashing), its
    // marks and its slash.
    let mut eraser = vec![once(4); 0x13];
    eraser[0] = vec![f(8, 0), f(8, LAST | LOOP)];
    eraser[0x12] = vec![f(4, 0), f(40, LAST)];
    sprites.insert(PackSprite { category: 8, index: 4 }, eraser);
    sprites.insert(PackSprite { category: 0x10, index: 0x50 }, vec![vec![f(4, 0), f(4, LAST | LOOP)]]);
    sprites.insert(PackSprite { category: 0x10, index: 0x51 }, vec![vec![f(3, 0), f(3, LAST | LOOP)]; 3]);
    // The elements navi (appearing, leaving, winding up, attacking, a vine)
    // and its overlay; its meteor, ice and bolt.
    let mut elements = vec![once(4); 0x13];
    elements[0] = vec![f(8, 0), f(8, LAST | LOOP)];
    sprites.insert(PackSprite { category: 8, index: 0x10 }, elements);
    sprites.insert(PackSprite { category: 8, index: 0x11 }, vec![once(4); 0x20]);
    sprites.insert(PackSprite { category: 0x0C, index: 0x31 }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    sprites.insert(PackSprite { category: 0x10, index: 0x0F }, vec![once(4), vec![f(4, 0), f(4, LAST | LOOP)]]);
    sprites.insert(PackSprite { category: 0x14, index: 0x14 }, vec![vec![f(3, 0), f(3, LAST | LOOP)]]);
    // The shooting navi (rising, raising his arm, shooting) and his cape
    // (his animation + 0x14), his shots' bursts; the sun-and-moon navi
    // and its moonlight.
    let mut shooter = vec![once(4); 0x21];
    shooter[0] = vec![f(8, 0), f(8, LAST | LOOP)];
    shooter[0x0C] = vec![f(4, 0), f(4, LAST | LOOP)];
    sprites.insert(PackSprite { category: 8, index: 0x13 }, shooter);
    sprites.insert(PackSprite { category: 0x10, index: 0x26 }, vec![vec![f(3, 0), f(3, LAST)]]);
    sprites.insert(PackSprite { category: 0x0C, index: 0x64 }, vec![vec![f(8, 0), f(8, LAST | LOOP)]; 5]);
    // Count (appearing, standing, raising his arms, lowering them,
    // leaving; his lance, animation 0xC) and the rain's motes; Django
    // (riding 6, his bike 7, appearing 1, slashing 5, leaving 2; his gun 8
    // and 9, CrosOver's).
    let mut count = vec![once(4); 0x0D];
    count[0] = vec![f(8, 0), f(8, LAST | LOOP)];
    count[0x0C] = vec![f(3, 0), f(3, LAST | LOOP)];
    sprites.insert(PackSprite { category: 8, index: 0x16 }, count);
    sprites.insert(PackSprite { category: 0x10, index: 0x10 }, vec![vec![f(3, 0), f(3, LAST | LOOP)]]);
    let mut django = vec![once(4); 10];
    django[0] = vec![f(8, 0), f(8, LAST | LOOP)];
    django[6] = vec![f(4, 0), f(4, LAST | LOOP)];
    django[7] = vec![f(4, 0), f(4, LAST | LOOP)];
    sprites.insert(PackSprite { category: 0x0C, index: 0x0F }, django);
    // Otenko's statue: the puff it appears in, then Otenko.
    sprites.insert(
        PackSprite { category: 0x0C, index: 0x49 },
        vec![vec![f(2, 0), f(2, 0), f(2, LAST)], vec![f(8, 0), f(8, 0), f(8, 0), f(8, LAST | LOOP)]],
    );
    // The water navi, his ball, splash, pillar, geyser and marks, and his
    // layer.
    let mut spout = vec![once(4); 0x16];
    spout[0] = vec![f(8, 0), f(8, LAST | LOOP)];
    sprites.insert(PackSprite { category: 8, index: 6 }, spout);
    sprites.insert(PackSprite { category: 0x0C, index: 0x23 }, vec![once(4), vec![f(2, 0), f(2, LAST | LOOP)]]);
    sprites.insert(PackSprite { category: 0x0C, index: 0x1A }, vec![vec![f(5, 0), f(5, LAST | LOOP)]]);
    sprites.insert(PackSprite { category: 0x10, index: 0x1F }, vec![vec![f(3, 0), f(3, LAST | LOOP)]; 4]);
    sprites.insert(PackSprite { category: 0x10, index: 0x20 }, vec![vec![f(3, 0), f(3, LAST | LOOP)]; 3]);
    sprites.insert(PackSprite { category: 0x10, index: 0x21 }, vec![vec![f(6, 0), f(6, LAST | LOOP)]]);
    // The buster's muzzle flash, and its arm (by form).
    sprites.insert(PackSprite { category: 0x0C, index: 0x06 }, vec![vec![f(2, 0), f(2, LAST)]]);
    sprites.insert(PackSprite { category: 0x0C, index: 0x03 }, vec![vec![f(30, LAST | LOOP)]; 0x19]);
    // The junk ball: rolling, bursting.
    let mut junk = vec![once(4); 0x1B];
    junk[0x19] = vec![f(4, 0), f(4, LAST | LOOP)];
    junk[0x1A] = vec![f(10, 0), f(20, LAST)];
    sprites.insert(PackSprite { category: 8, index: 0x0A }, junk);
    // The Reflector's shield (up, fading, by look) and its wave.
    sprites.insert(PackSprite { category: 0x0C, index: 0x1B }, vec![vec![f(8, LAST | LOOP)], vec![f(7, 0), f(7, LAST)]]);
    sprites.insert(PackSprite { category: 0x14, index: 0x04 }, vec![vec![f(2, 0), f(3, LAST)]]);
    // The swords' blade, swinging.
    sprites.insert(PackSprite { category: 0x0C, index: 0x08 }, vec![vec![f(3, 0), f(3, 0), f(8, LAST)]]);
    // The arrow.
    sprites.insert(PackSprite { category: 0x0C, index: 0x21 }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    // The grab shot: falling, landing.
    sprites.insert(PackSprite { category: 0x0C, index: 0x13 }, vec![vec![f(8, LAST | LOOP)], vec![f(3, 0), f(3, LAST)]]);
    // GroundCross's falling rock (0) and its chunks (1).
    sprites.insert(PackSprite { category: 0x10, index: 5 }, vec![vec![f(8, LAST | LOOP)], vec![f(4, LAST | LOOP)]]);
    // The Beast charged chips' pillars (flames, lightning: rising, dying
    // down), surge (rising, ebbing, falling) and whirlwind.
    for index in [0x1C] {
        sprites.insert(PackSprite { category: 0x0C, index }, vec![vec![f(4, LAST | LOOP)], vec![f(2, 0), f(2, LAST)]]);
    }
    sprites.insert(PackSprite { category: 0x10, index: 0x32 }, vec![vec![f(4, LAST | LOOP)], vec![f(2, 0), f(2, LAST)]]);
    sprites.insert(PackSprite { category: 0x10, index: 0x2E }, vec![vec![f(6, LAST | LOOP)]; 3]);
    sprites.insert(PackSprite { category: 0x10, index: 0x44 }, vec![vec![f(3, 0), f(3, LAST | LOOP)]]);
    // The gust, the sword waves, EraseCross's beam (opening, beaming,
    // closing) and the drill arm (attachment 0x20).
    sprites.insert(PackSprite { category: 0x0C, index: 0x2E }, vec![vec![f(4, 0), f(4, LAST | LOOP)]]);
    sprites.insert(PackSprite { category: 0x0C, index: 0x14 }, vec![vec![f(3, 0), f(3, LAST)]]);
    sprites.insert(PackSprite { category: 0x10, index: 0x4C }, vec![vec![f(3, 0), f(3, LAST)], vec![f(8, LAST | LOOP)], once(4)]);
    sprites.insert(PackSprite { category: 0x0C, index: 0x20 }, vec![once(4), vec![f(4, 0), f(4, LAST | LOOP)]]);
    // The hive (closed, open), a bee, and a dragon's animations.
    sprites.insert(PackSprite { category: 0x0C, index: 0x5E }, vec![vec![f(30, LAST | LOOP)], vec![f(4, 0), f(30, LAST)]]);
    sprites.insert(PackSprite { category: 0x10, index: 0x31 }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    sprites.insert(PackSprite { category: 0x04, index: 0x10 }, vec![vec![f(6, LAST | LOOP)]; 8]);
    // The crack shot: flying.
    sprites.insert(PackSprite { category: 0x0C, index: 0x33 }, vec![vec![f(2, 0), f(2, LAST | LOOP)]]);
    // Effects and sparks.
    sprites.insert(PackSprite { category: 0x14, index: 0 }, vec![vec![f(3, 0), f(3, 0), f(3, LAST)]]);
    sprites.insert(PackSprite { category: 0x14, index: 1 }, vec![vec![f(2, 0), f(2, LAST)]]);
    // The rising bubble.
    sprites.insert(PackSprite { category: 0x14, index: 2 }, vec![once(4), vec![f(4, 0), f(4, 0), f(4, LAST)]]);
    // Dimming chip subtypes 10, 11, 14 and ElemTrap's (20): the countdown
    // bomb (rising, standing; twice), the mine, the guardian statue
    // (standing, striking).
    let rise_and_stand = vec![vec![f(3, 0), f(3, LAST)], vec![f(20, LAST | LOOP)]];
    sprites.insert(PackSprite { category: 0x0C, index: 0x23 }, [rise_and_stand.clone(), rise_and_stand].concat());
    sprites.insert(PackSprite { category: 0x0C, index: 0x22 }, vec![vec![f(4, 0), f(4, LAST | LOOP)]]);
    sprites.insert(PackSprite { category: 0x0C, index: 0x35 }, vec![vec![f(20, LAST | LOOP)], vec![f(4, 0), f(8, LAST)]]);
    sprites
}
