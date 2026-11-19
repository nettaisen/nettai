//! A small hand-authored content set for tests and examples.
//!
//! Everything here is made up: a MegaMan-like navi and its base form,
//! a few chips that use GunDelSol, two dimming chips (one grabs a column)
//! and ten navi chips, rocks,
//! sprites with short animations, and rules written from the engine's own
//! flag semantics (docs/engine/field-collision-damage.md). It is not EXE6's
//! data (content/exe6's), and its numbers are chosen for tests, not taken
//! from the game. It has just what battles of two such navis need:
//! stepping, the chips below, custom screens, rocks and the round's flow.
//!
//! Its sprites are named as any content's are (`asset.sprite(name)`): its
//! own (testdata/content/sprites.luau), and EXE6's that the EXE6 modules
//! name. Its asset handles are the names its modules give; the numbers a
//! frontend's test draws them by are made up ([`asset_index`],
//! [`asset_numbers`]).
//!
//! Its scripts are this repository's EXE6 scripts (content/exe6, the source
//! overlay), read from the repository, and its own modules
//! (testdata/content): the test chips are definitions there, made-up
//! records whose uses are EXE6's builders and actions, so the tests run the
//! real scripts on data they can reason about. (The EXE6 modules it loads
//! bring their own chip definitions too; tests name either by key.)

use super::*;

use std::sync::Arc;

// The test chips (testdata/content/chips/test/init.luau), by key: made-up
// records whose uses are EXE6's builders and actions.
/// Three GunDelSol levels and an EX (two columns).
pub const SUN_GUN_1: &str = "test/sun_gun_1";
pub const SUN_GUN_2: &str = "test/sun_gun_2";
pub const SUN_GUN_3: &str = "test/sun_gun_3";
pub const SUN_GUN_EX: &str = "test/sun_gun_ex";
/// A dimming chip (the invisibility freeze).
pub const VEIL: &str = "test/veil";
/// A navi chip (the eraser navi).
pub const ERASER: &str = "test/eraser";
/// Instant chips: a plus chip used on its own, fists (FireHit's) and flame
/// hooks (FlmHook's).
pub const PLUS: &str = "test/plus";
pub const FIST: &str = "test/fist";
pub const FLAME_HOOK: &str = "test/flame_hook";
/// The thrown chips: a bomb, a seed that poisons panels, a flash bomb and
/// a bug bomb.
pub const BOMB: &str = "test/bomb";
pub const SEED: &str = "test/seed";
pub const FLASH: &str = "test/flash";
pub const BUG: &str = "test/bug";
/// A sword (a column of three panels ahead).
pub const BLADE: &str = "test/blade";
/// A step sword (the same, after a step two panels ahead).
pub const STEP_BLADE: &str = "test/step_blade";
/// A strike at stunned or grounded opponents.
pub const STUN_BLADE: &str = "test/stun_blade";
/// A blank chip that is the AntiNavi trap when a side's defensive-chip
/// record holds it.
pub const ANTI_NAVI: &str = "test/anti_navi";
/// A trap chip that sets no object.
pub const TRAP: &str = "test/trap";
/// An element trap (the trap object).
pub const ELEM_TRAP: &str = "test/elem_trap";
/// Time bombs: the plain one and the big one.
pub const TIME_BOMB: &str = "test/time_bomb";
pub const TIME_BOMB_PLUS: &str = "test/time_bomb_plus";
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
pub const SUN_MOON: &str = "test/sun_moon";
/// The link navis' own chips: EXE6's HeatPres, DElecSwd, RSlash, EDeletBm,
/// VolcChrg, DripShwr, ETomahwk, FTornado, RC Brakr and DustBrk's actions
/// (navis/<navi>/chip.luau), as chips of made-up damage. Any navi can use
/// them here.
pub const LINK_CHIPS: [&str; 10] = [
    "test/heat_press",
    "test/dash_elec_sword",
    "test/rolling_slash",
    "test/eraser_delete_beam",
    "test/volcano_charge",
    "test/drip_shower",
    "test/eagle_tomahawk",
    "test/fuujin_tornado",
    "test/rock_crush_breaker",
    "test/dust_break",
];
/// A link navi (the content's navi 1; AI index 4, whose actor record has
/// no hooks).
pub const LINK_NAVI: &str = "test/link_navi";
/// A link navi with hooks of its own (AI index 5): a tick, an idle and a
/// post-init hook (testdata/content/navis/test.luau).
pub const HOOKED_NAVI: &str = "test/hooked_navi";
/// MegaMan, the navi that changes form.
pub const MEGAMAN: &str = "megaman";

/// The test stages (testdata/content/stages/test.luau), link battles on
/// the plain field: two navis, side 1's placed first (the usual netbattle
/// order); side 0's first; two navis with two rocks, one on each side; and
/// two navis with three boulders (the field has two stage slots).
pub const LINK_BATTLE: &str = "test/link_battle";
pub const LINK_BATTLE_SIDE0_FIRST: &str = "test/link_battle_side0_first";
pub const ROCK_BATTLE: &str = "test/rock_battle";
pub const BOULDER_BATTLE: &str = "test/boulder_battle";

/// The test stages' music's song (the asset `test_stage_music`).
pub const STAGE_SONG: u16 = 0x16;

/// The navi's sprite (base form), by its made-up number.
pub const NAVI_SPRITE: SpriteNumber = SpriteNumber { category: 0, index: 0 };

/// The test content's assets by their made-up numbers ([`asset_index`]: EXE6's numbers where tests look at them,
/// made-up ones for the rest): what a frontend's test draws them by.
pub fn asset_numbers(c: &Content) -> AssetNumbers {
    AssetNumbers::new(&c.assets, &asset_index_of(c)).unwrap_or_else(|missing| panic!("the test assets have no {}", missing.join(", ")))
}

/// Sprite `id`'s made-up number (several names may be one sprite: tests compare the numbers).
pub fn sprite_number(c: &Content, id: SpriteId) -> SpriteNumber {
    asset_numbers(c).sprite(id.0).unwrap_or_else(|| panic!("no sprite has handle {}", id.0))
}

/// The test assets' song `n` as the test content names it (its handle).
pub fn sound(n: u16) -> crate::sound::SoundId {
    let c = content();
    let numbers = asset_numbers(&c);
    crate::sound::SoundId(numbers.sounds.iter().position(|&s| s == n).unwrap_or_else(|| panic!("the test content names no song {n:#x}")) as u16)
}

/// The test content's asset `name` of `kind` (`test_navi`): its handle.
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

/// The test content's panel type named `name` (its rules' `panels.numbers`:
/// EXE6's thirteen, then magnet, lava and sea).
pub fn panel(name: &str) -> crate::field::PanelType {
    content().rules().panels.named(name).unwrap_or_else(|| panic!("the test content has no panel type {name:?}"))
}

/// The content set, shared (defined once per process: the define phase
/// runs every module).
pub fn content() -> Arc<Content> {
    shared().0.clone()
}

/// The content set with its rules (testdata/content/rules/init.luau) patched:
/// each `(from, to)` replaces the first `from` in the definition's text.
/// Defined once per patch list: a game has one definition of its rules, so a
/// test that plays by other rules plays another content. Its definitions are
/// the content set's but the rules, so every handle is the same; a round's
/// setup names it by its hash (`RoundSetup::content`).
pub fn with_rules(patches: &[(&str, &str)]) -> Arc<Content> {
    static MADE: std::sync::Mutex<Vec<(Vec<(String, String)>, Arc<Content>)>> = std::sync::Mutex::new(Vec::new());
    let key: Vec<(String, String)> = patches.iter().map(|&(a, b)| (a.to_string(), b.to_string())).collect();
    let mut made = MADE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, c)) = made.iter().find(|(k, _)| *k == key) {
        return c.clone();
    }
    let mut c = make();
    let module = c.scripts.module_mut(ROOT, "rules/init").expect("the test content's rules");
    for (from, to) in patches {
        assert!(module.contains(from), "rules/init.luau has no `{from}`");
        *module = module.replacen(from, to, 1);
    }
    let c = Arc::new(c.defined());
    made.push((key, c.clone()));
    c
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

/// The version the test rounds' players are of: this content plays by
/// EXE6's rules/beast, whose setup takes one, and a round states it (none
/// is assumed: `SideRules::for_player`).
pub const VERSION: &str = "falzar";

/// A player who brings nothing but their version ([`VERSION`]), where the
/// rules of `content` take one.
pub fn player(content: &Content) -> crate::custom::PlayerSetup {
    let mut p = crate::custom::PlayerSetup::default();
    p.set_fact(content, "version", &[crate::rules::Fact::Name(VERSION)]).expect("the test content's version");
    p
}

/// A round on this content on stage `stage` (its key), both navis with
/// `stats`: RNG seed 1, side 0's perspective, no set score, no folders,
/// the players of [`VERSION`].
pub fn round_setup(stage: &str, stats: crate::setup::NaviStats) -> crate::setup::RoundSetup {
    let (content, hash) = shared();
    crate::setup::RoundSetup {
        content: *hash,
        settings: crate::setup::BattleSettings::on(content, content.stage_by_key(stage)),
        navi_stats: [stats; 2],
        rng: 1,
        local_side: 0,
        score: Default::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        host_keys: crate::setup::HostKeys::NONE,
        custom_screen_limit: None,
        players: [player(content), player(content)],
    }
}

/// Play `setup` on `content`, another build of the test content (its own
/// rules and handles): the round names it, and its players' setups are
/// remade for its rules (a setup's block is its content's): of
/// [`VERSION`], and nothing else.
pub fn on(setup: &mut crate::setup::RoundSetup, content: &Content) {
    setup.content = content.hash();
    for p in &mut setup.players {
        p.rules = player(content).rules;
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
/// EXE6's AreaGrab and PanelGrab (chips/areagrab, chips/panlgrab): dimming
/// chips content defines, which grab a column and a panel.
pub const AREA_GRAB: &str = "area_grab";
pub const PANEL_GRAB: &str = "panel_grab";
/// EXE6's BusterUp, Atk+10 and Navi+20 (chips/busterup, chips/atk-10,
/// chips/navi-20): instant chips content defines.
pub const BUSTER_UP: &str = "buster_up";
pub const ATTACK_10: &str = "attack_10";
pub const NAVI_20: &str = "navi_20";
/// EXE6's instant chips content defines whose effects fill the gauge, sync
/// the navi, and spawn objects: FullCust, SyncTrgr, Boomer, Lance,
/// SandWrm1, JustcOne, GolmHit1 (chips/fullcust ... chips/golmhit).
pub const FULL_CUST: &str = "full_custom";
pub const SYNC_TRIGGER: &str = "synchro_trigger";
pub const BOOMER: &str = "boomerang";
pub const LANCE: &str = "lance";
pub const SAND_WORM: &str = "sand_worm_1";
pub const JUSTICE_ONE: &str = "justice_one";
pub const GOLEM_HIT: &str = "golem_hit_1";
/// EXE6's RockCube (chips/rockcube): a dimming chip content defines, which
/// places a rock in front of its user.
pub const ROCK_CUBE: &str = "rock_cube";
pub const TICK_SHOT: &str = "test/tick_shot";
/// EXE6's CrakShot, Rflectr1 and Recov50 (chips/crakshot, chips/rflectr,
/// chips/recov): standard chips content defines, which dig up the panel
/// ahead, guard and reflect, and heal.
pub const CRAK_SHOT: &str = "crack_shot";
pub const REFLECTOR_1: &str = "reflector_1";
pub const RECOV_50: &str = "recovery_50";
/// EXE6's SloGauge and Mine (chips/slogauge, chips/mine): dimming chips
/// content defines, which slow the custom gauge and lay a mine.
pub const SLOW_GAUGE: &str = "slow_gauge";
pub const MINE: &str = "mine";
/// EXE6's RskyHny2 and ElecDrgn (chips/rskyhny, chips/elecdrgn): chips
/// content defines, which send bees and an elec dragon.
pub const BEES: &str = "risky_honey_2";
pub const DRAGON: &str = "elec_dragon";
/// EXE6's Gregar and Falzar (chips/gregar, chips/falzar: the Japanese ROMs'
/// giga cut-in chips), which summon the cyber beasts.
pub const GREGAR: &str = "gregar";
pub const FALZAR: &str = "falzar";

/// The test content's game: its own modules and the EXE6 modules it borrows
/// are one game, `test` (its modules are `test:...`; what they define is
/// keyed by its own name, as in any game).
pub const ROOT: &str = "test";

/// The content model v2 test pack (crates/nettai-battle/testdata/pack):
/// definitions the engine's tests run.
const TEST_PACK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/pack");

/// The test content's own modules (its roles).
const TEST_CONTENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/content");

/// The support pack (content/exelib) in `scripts`: what EXE6's modules
/// require by `@exelib/...`, for content that loads them; each read from
/// its folder as a load requires it.
pub fn add_shared(scripts: &mut Scripts) {
    scripts.add_support_dir(EXELIB_PACK, EXELIB);
}

/// A manifest for game `game`, whose modules `scripts` holds under its
/// name, that depends on the support packs `scripts` holds; and a top
/// module that loads every one of them (`Scripts::init_for`, in place of
/// its own): for tests that load a game's directory as it is, or add
/// modules to a game.
pub fn add_index(scripts: &mut Scripts, game: &str) {
    use nettai_content_api::{PackKind, PackManifest, packs};
    let prefix = format!("{game}{}", nettai_content_api::keys::SEPARATOR);
    let own: std::collections::BTreeMap<String, String> =
        scripts.modules.iter().filter_map(|(name, source)| Some((name.strip_prefix(&prefix)?.to_string(), source.clone()))).collect();
    scripts.modules.insert(packs::top_module(game), Scripts::init_for(&own));
    scripts.hold(game, own.keys());
    let depends = scripts.packs.iter().filter(|p| p.kind == PackKind::Support).map(|p| p.id.clone()).collect();
    scripts.set_manifest(PackManifest { id: game.to_string(), kind: PackKind::Game, depends });
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
    // The test pack brings its own roles (the rules' `roles`).
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

/// The content set, not yet defined. (The navis and the base form are
/// definitions: testdata/content/navis/test.luau; its rules are its rules':
/// testdata/content/rules; its sprites its modules': testdata/content/
/// sprites.luau.)
fn make() -> Content {
    // (Its stages are drawn on its one background.)
    let presentation = nettai_content_api::Presentation {
        stage_background: Some("test_background".into()),
        backgrounds: vec!["test_background".into()],
        ..Default::default()
    };
    Content { scripts: scripts(), strings: strings(), presentation, ..Default::default() }
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

/// A made-up asset index for `modules`: every name they give an
/// `asset.<kind>("...")` call, each a made-up asset of its own (nothing
/// ROM-derived).
pub fn asset_index_used(modules: &std::collections::BTreeMap<String, String>) -> nettai_content_api::AssetIndex {
    use nettai_content_api::AssetKind;
    let mut a = nettai_content_api::AssetIndex::default();
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
                        a.sprites.entry(name.into()).or_insert(SpriteNumber { category: 0x7F, index: id as u8 });
                    }
                    AssetKind::Sound => {
                        // (Past the song tables' numbers, which tests look sounds up by.)
                        a.sounds.entry(name.into()).or_insert(0x400 + id);
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

/// The test content's made-up asset index: the EXE6 names its modules use
/// (with EXE6's numbers where its tests look at them, made-up assets for
/// the rest) and a few made-up ones for the test pack.
pub fn asset_index() -> nettai_content_api::AssetIndex {
    asset_index_with(&scripts().available())
}

/// [`asset_index`] for content `c` (a test content with modules of its own).
fn asset_index_of(c: &Content) -> nettai_content_api::AssetIndex {
    let mut modules = c.scripts.available();
    modules.extend(c.scripts.modules.iter().map(|(k, v)| (k.clone(), v.clone())));
    asset_index_with(&modules)
}

/// The test content's asset index, with names of `modules` too.
fn asset_index_with(modules: &std::collections::BTreeMap<String, String>) -> nettai_content_api::AssetIndex {
    let mut a = numbered_assets();
    let used = asset_index_used(modules);
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
/// rules' roles (testdata/content/rules/role_definitions.luau's assets, by role
/// name): what the tests look for in the cues and the HUD.
const ROLE_SOUNDS: &[(&str, u16)] = &[
    ("panel_crack", 0x97),
    ("panel_poison", 0x90),
    ("own_hit", 0x6b),
    ("hit", 0x6d),
    ("damage", 0x85),
    ("guard", 0x6e),
    ("counter_hit", 0x86),
    ("deleted", 0x6c),
    ("recovery", 0x8a),
    ("damage_bonus", 0x87),
    ("pause", 0x9f),
    ("gauge_full", 0x8f),
    ("low_hp", 0x84),
    ("cut_in", 0xa5),
    ("trap_mark", 0xa5),
    ("telop", 0x173),
    ("buster_charge", 0x71),
    ("buster_charged", 0x72),
    ("freeze", 0x118),
    ("bubble", 0x12d),
    ("bubble_pop", 0x124),
    ("confusion", 0x88),
    ("invisible", 0x93),
    ("appear", 0x94),
    ("arrive", 0x129),
    ("fade", 0x8e),
    ("obstacle_lift", 0x12a),
    ("obstacle_throw", 0x10c),
    ("cross_merge", 0x8c),
    ("form_change", 0xf7),
    ("cross_change", 0x8d),
    ("cross_change_chime", 0x77),
    ("beast_out", 0x100),
    ("gregar_roar", 0x1cc),
    ("falzar_roar", 0x1cd),
    ("beast_over_rumble", 0x19a),
    ("beast_over_burst", 0x12e),
    ("cross_special", 0x182),
    ("refused", 0x69),
    ("custom_open", 0x79),
    ("custom_cursor", 0x7f),
    ("custom_hide", 0x80),
    ("custom_pick", 0x81),
    ("custom_ok", 0x82),
    ("custom_back", 0x83),
    ("custom_cross_open", 0x7a),
    ("custom_cross_close", 0x7d),
    ("custom_cross_chosen", 0x92),
    ("custom_run_message", 0x7b),
    ("custom_description", 0x9c),
    ("custom_description_close", 0x9e),
    ("custom_beast_out_falzar", 0x193),
    ("custom_beast_out_gregar", 0x191),
    ("custom_beast_out_flash", 0xbc),
    ("custom_cancel", 0x1d2),
    ("custom_redeal", 0x182),
    ("custom_redeal_shuffle", 0x113),
    ("custom_scrap", 0x196),
    ("custom_scrap_done", 0x182),
    ("program_advance_part", 0x91),
    ("program_advance", 0x92),
    ("encounter", 0x78),
];
const ROLE_MUSIC: &[(&str, u16)] = &[
    ("link_battle", 0x15),
    ("winner_special", 0x19),
    ("winner", 0x1f),
    ("loser", 0x1a),
];
const ROLE_SPRITES: &[(&str, (u8, u8))] = &[
    ("charge_glow", (0x14, 0x08)),
    ("charge_glow_a", (0x14, 0x15)),
    ("full_synchro_aura", (0x14, 0x16)),
    ("confusion", (0x14, 0x0b)),
    ("blindness", (0x14, 0x09)),
    ("immobilized", (0x10, 0x00)),
    ("ice", (0x14, 0x1c)),
    ("bubble", (0x0c, 0x20)),
    ("hit_marker", (0x14, 0x07)),
    ("eruption", (0x10, 0x24)),
    ("lockon_marker", (0x0c, 0x09)),
    ("idle_overlay", (0x10, 0x21)),
    ("beast_head", (0x0c, 0x0a)),
];
const ROLE_BANNERS: &[(&str, u8)] = &[
    ("round_start", 0x30),
    ("turn_start", 0xc),
    ("final_turn", 0x10),
    ("draw", 0x1c),
    ("win", 0x04),
    ("win_judged", 0x14),
    ("lose_judged", 0x18),
    ("judge", 0x28),
    ("telop", 0x4c),
    ("telop_remote", 0x50),
    ("program_advance", 0x24),
    ("program_advance_empty", 0x34),
];

/// The test content's assets with EXE6's numbers.
fn numbered_assets() -> nettai_content_api::AssetIndex {
    let mut a = nettai_content_api::AssetIndex::default();
    for (role, id) in ROLE_SOUNDS {
        a.sounds.insert(format!("test_sound_{role}"), *id);
    }
    for (role, id) in ROLE_MUSIC {
        a.sounds.insert(format!("test_music_{role}"), *id);
    }
    for (role, (category, index)) in ROLE_SPRITES {
        a.sprites.insert(format!("test_sprite_{role}"), SpriteNumber { category: *category, index: *index });
    }
    for (role, id) in ROLE_BANNERS {
        a.banners.insert(format!("test_banner_{role}"), *id);
    }
    // The test stages' (testdata/content/stages/test.luau).
    a.sounds.insert("test_stage_music".into(), STAGE_SONG);
    a.backgrounds.insert("test_background".into(), 0);
    // The test navis' (testdata/content/navis/test.luau).
    a.sprites.insert("test_megaman".into(), SpriteNumber { category: 8, index: 0 });
    a.sprites.insert("test_navi".into(), NAVI_SPRITE);
    a.banners.insert("test_win".into(), 0x40);
    a.banners.insert("test_deleted".into(), 0x44);
    let sprite = |c, i| SpriteNumber { category: c, index: i };
    for (name, id) in [
        ("test_burst", sprite(0x14, 0)),
        ("test_spark", sprite(0x14, 1)),
        ("sprite-14-02", sprite(0x14, 2)),
        ("bomb", sprite(0x0C, 0x02)),
        ("flash_bomb", sprite(0x10, 0x52)),
        ("black_bomb", sprite(0x0C, 0x24)),
        ("energy_burst", sprite(0x14, 0x12)),
        ("sword", sprite(0x0C, 0x00)),
        ("explosion", sprite(0x14, 0x00)),
        ("small_puff", sprite(0x14, 0x02)),
        ("puff", sprite(0x14, 0x0D)),
        ("burst", sprite(0x14, 0x0A)),
        ("rock_cubes", sprite(0x10, 0x00)),
        ("grab_shot", sprite(0x0C, 0x13)),
        ("hit_sparks", sprite(0x14, 0x05)),
        ("fire_sword", sprite(0x0C, 0x36)),
        ("aqua_sword", sprite(0x0C, 0x37)),
        ("elec_sword", sprite(0x0C, 0x38)),
        ("sword_slash", sprite(0x0C, 0x14)),
        ("big_slash", sprite(0x0C, 0x15)),
        ("cross_slash", sprite(0x10, 0x41)),
        ("pink_flash", sprite(0x14, 0x04)),
        ("muzzle_flash", sprite(0x0C, 0x06)),
        ("buster_arm", sprite(0x0C, 0x03)),
        ("eraseman", sprite(0x08, 0x04)),
        ("buster_up", sprite(0x14, 0x1B)),
        ("erase_mark", sprite(0x10, 0x50)),
        ("erase_beam", sprite(0x10, 0x51)),
        ("air_raid_plane", sprite(0x04, 0x18)),
        ("aura", sprite(0x0C, 0x07)),
        ("barrier", sprite(0x0C, 0x3D)),
        ("bubble", sprite(0x0C, 0x20)),
        ("instrument", sprite(0x04, 0x0A)),
        ("jet_flame", sprite(0x10, 0x28)),
        ("propeller", sprite(0x10, 0x3D)),
        ("sensor", sprite(0x04, 0x05)),
        ("gregar_cybeast", sprite(0x08, 0x14)),
        ("summon_black", sprite(0x04, 0x1D)),
        ("flame_hook_fire", sprite(0x0C, 0x45)),
        ("impact", sprite(0x14, 0x01)),
        ("hit_marker", sprite(0x14, 0x07)),
        ("chip_destruction_spark", sprite(0x14, 0x0C)),
        ("shell_burst", sprite(0x14, 0x11)),
        ("beast_shot", sprite(0x0C, 0x21)),
        ("bow", sprite(0x0C, 0x2A)),
        ("lil_boiler", sprite(0x04, 0x0D)),
        ("voodoo_doll", sprite(0x0C, 0x34)),
        ("crack_shot", sprite(0x0C, 0x33)),
        ("dust", sprite(0x14, 0x0E)),
        ("reflector_shield", sprite(0x0C, 0x1B)),
        ("dummy_shield", sprite(0x04, 0x00)),
        ("megaman_navi", sprite(0x08, 0x00)),
        ("drill_arm", sprite(0x0C, 0x58)),
        ("burner", sprite(0x0C, 0x29)),
        ("heat_cross_burner", sprite(0x0C, 0x1F)),
        ("flame", sprite(0x0C, 0x0E)),
        ("boomerang", sprite(0x10, 0x07)),
        ("boomerang_tomahawk", sprite(0x10, 0x57)),
        ("lance", sprite(0x0C, 0x44)),
        ("fire_hit", sprite(0x14, 0x1A)),
        ("sand_worm", sprite(0x04, 0x1A)),
        ("sand_hole", sprite(0x10, 0x48)),
        ("justice_one", sprite(0x0C, 0x62)),
        ("golem", sprite(0x10, 0x30)),
        ("dust_2", sprite(0x10, 0x2C)),
        ("rock_cubes", sprite(0x10, 0x00)),
        ("rock_debris", sprite(0x10, 0x01)),
        ("boulder", sprite(0x10, 0x08)),
        ("falling_rock", sprite(0x10, 0x05)),
        ("countdown_bomb", sprite(0x0C, 0x23)),
        ("heat_flame", sprite(0x10, 0x02)),
        ("follow_effect", sprite(0x10, 0x0E)),
        ("drip_shower", sprite(0x10, 0x22)),
        ("eagle_tomahawk", sprite(0x10, 0x2A)),
        ("tengu_tornado", sprite(0x10, 0x42)),
        ("volcano_rock", sprite(0x10, 0x55)),
        ("ground_drill", sprite(0x10, 0x4F)),
        ("dust_cloud", sprite(0x10, 0x59)),
        ("dustman", sprite(0x08, 0x0A)),
        ("swirl", sprite(0x0C, 0x28)),
        // The traps, mines, time bombs and navi-changing chips.
        ("spout_splash", sprite(0x0C, 0x1A)),
        ("lightning", sprite(0x14, 0x14)),
        ("land_mine", sprite(0x0C, 0x22)),
        ("blast", sprite(0x14, 0x13)),
        ("hub", sprite(0x14, 0x1E)),
        ("bug", sprite(0x14, 0x1F)),
        ("charge_glow_a", sprite(0x14, 0x15)),
        // The waves and pillars.
        ("slash_wave", sprite(0x10, 0x39)),
        ("charged_slash", sprite(0x10, 0x3B)),
        ("moon_blade", sprite(0x10, 0x3C)),
        ("element_pillar_flames", sprite(0x0C, 0x1C)),
        ("element_pillar_lightning", sprite(0x10, 0x32)),
        // The supports, and the barrier Tango's heal raises.
        ("rush", sprite(0x0C, 0x48)),
        ("beat", sprite(0x0C, 0x4B)),
        ("tango", sprite(0x0C, 0x4C)),
        ("tango_heal", sprite(0x0C, 0x4D)),
        ("heal", sprite(0x0C, 0x12)),
        // SunMoon and its meteors.
        ("moon_beam", sprite(0x0C, 0x64)),
        ("meteor", sprite(0x0C, 0x31)),
    ] {
        a.sprites.insert(name.into(), id);
    }
    for (name, id) in [
        ("test_tick", 0x1A6),
        ("mini_bomb_throw", 0xB2),
        ("hit_bomb_1", 0x70),
        ("panel_poison", 0x90),
        ("freeze", 0x118),
        ("grass", 0x11B),
        ("flash", 0x1BD),
        ("bug_bomb_land", 0x115),
        ("land", 0xC0),
        ("burst", 0xC3),
        ("energy_burst", 0xBB),
        ("sword_swing", 0xB0),
        ("big_sword_swing", 0xCE),
        ("grab_shot", 0xA2),
        ("grab_shot_2", 0xA1),
        ("appear", 0x94),
        ("place", 0x112),
        ("falling_rock", 0xD9),
        ("erase_man", 0x10E),
        ("erase_man_2", 0xBA),
        ("hub", 0x119),
        ("buster_shot", 0x6A),
        ("junk_shot", 0xFF),
        ("gun_del_sol_1", 0xF8),
        ("bonus", 0x157),
        ("barrier", 0x89),
        ("beast_over", 0x19A),
        ("bubble", 0x12D),
        ("bubble_pop", 0x124),
        ("bug", 0x143),
        ("bug_fix_flash", 0xD1),
        ("cross_merge", 0x8C),
        ("discord", 0xA9),
        ("fanfare", 0xA8),
        ("invisible", 0x93),
        ("log_in", 0x77),
        ("panel_change", 0xA4),
        ("panel_change_tick", 0xA3),
        ("point_appear", 0x129),
        ("point_rise", 0x12A),
        ("silence", 0xAB),
        ("take_off", 0x1A9),
        ("timpani", 0xAA),
        ("tomahawk_man", 0x10A),
        ("flame_hook_fire", 0x158),
        ("twang", 0x18A),
        ("boiler_erupt", 0x184),
        ("boiler_steam", 0x185),
        ("err_select_91", 0x91),
        ("hit_bomb_0", 0x6F),
        ("crack_shot", 0xDA),
        ("bubble_star_1", 0xD8),
        ("follow_effect", 0xA0),
        ("wave", 0xC5),
        ("roar", 0x12B),
        ("cross_change", 0x8D),
        ("boomerang", 0xB7),
        ("fire_hit", 0xED),
        ("sand_worm", 0xE1),
        ("sand_worm_2", 0x1BE),
        ("justice_one", 0xC4),
        ("golem", 0x10D),
        ("golem_2", 0x188),
        ("place", 0x112),
        ("panel_crack", 0x97),
        ("falling_rock", 0xD9),
        ("form_change", 0xF7),
        ("follow_effect", 0xA0),
        ("drip_shower", 0x128),
        ("eagle_tomahawk", 0x10C),
        ("aqua_surge", 0xB8),
        ("volcano", 0x146),
        ("rolling_slash", 0x164),
        ("dust_break", 0xAD),
        ("dust_break_2", 0x17B),
        ("rockfall", 0xE5),
        ("drill_spin", 0x1C0),
        // The traps, mines, time bombs, gauge and navi-changing chips.
        ("dimming_sparkle", 0xA5),
        ("target_move", 0x10F),
        ("spout_ball", 0x11D),
        ("beast_over_burst", 0x12E),
        ("hop", 0x113),
        ("tick", 0xC1),
        ("last", 0xC2),
        // The waves.
        ("ok_8b", 0x8B),
        ("aqua_needle_2", 0xB3),
        // The supports, and the barrier Tango's heal raises.
        ("bite", 0x122),
        ("set_down", 0x120),
        ("snatch", 0x126),
        ("arrive", 0x116),
        ("tango_land", 0xD4),
        ("heal", 0x8A),
        // SunMoon.
        ("sun_moon", 0x110),
        ("moon_beam", 0x111),
        ("blast_man", 0x17F),
    ] {
        a.sounds.insert(name.into(), id);
    }
    standard_chip_assets(&mut a);
    form_weapon_assets(&mut a);
    a
}

/// The asset names MegaMan's weapon definitions and the forms' kinds use
/// (content model v2, step 8e), with EXE6's numbers.
fn form_weapon_assets(a: &mut nettai_content_api::AssetIndex) {
    let sprite = |c, i| SpriteNumber { category: c, index: i };
    for (name, id) in [
        ("aqua_surge", sprite(0x10, 0x2E)),
        ("whirlwind", sprite(0x10, 0x44)),
        ("erase_ray", sprite(0x10, 0x4C)),
        ("erase_drop", sprite(0x10, 0x4D)),
        ("charge_wave", sprite(0x10, 0x09)),
        ("junk_shot", sprite(0x10, 0x56)),
        ("ground_drill_effect", sprite(0x0C, 0x2D)),
        ("slash_man_effect", sprite(0x10, 0x38)),
        ("groundman", sprite(0x08, 0x09)),
    ] {
        a.sprites.insert(name.into(), id);
    }
    for (name, id) in [
        ("aqua_needle_2", 0xB3),
        ("col_army_2", 0xB9),
        ("spout_beast_charge", 0xF4),
        ("moon_beam", 0x111),
        ("tomahawk_man", 0x10A),
        ("ground_beast_dash", 0x1BF),
        ("ground_beast_dash_2", 0x1C7),
        ("iron_shell", 0x187),
        ("beast_claw", 0x1C5),
        ("beast_claw_2", 0x1C6),
        ("charge_train", 0xE4),
        ("tenguman_nose", 0xFB),
        ("drill_launch", 0x14C),
        ("spin", 0xC7),
        ("drill_arm", 0xF0),
    ] {
        a.sounds.insert(name.into(), id);
    }
}

/// The asset names the standard chip actions' modules use (content model
/// v2, step 8g), with EXE6's numbers.
fn standard_chip_assets(a: &mut nettai_content_api::AssetIndex) {
    let sprite = |c, i| SpriteNumber { category: c, index: i };
    for (name, id) in [
        ("gust", sprite(0x0C, 0x2E)),
        ("wind_rack", sprite(0x0C, 0x27)),
        ("tengu_fan", sprite(0x0C, 0x5C)),
        ("swirl", sprite(0x0C, 0x28)),
        ("thunder_doll", sprite(0x10, 0x12)),
        ("thunder_doll_hand", sprite(0x0C, 0x60)),
        ("gun_del_sol", sprite(0x0C, 0x3B)),
        ("sun_beam", sprite(0x0C, 0x3C)),
        ("sun_beam_ex", sprite(0x0C, 0x47)),
        ("hive", sprite(0x0C, 0x5E)),
        ("honey_bee", sprite(0x10, 0x31)),
        ("dragon", sprite(0x04, 0x10)),
    ] {
        a.sprites.insert(name.into(), id);
    }
    navi_chip_assets(a);
    for (name, id) in
        [("wind_racket", 0x11F), ("beast_over", 0x19A), ("sun_beam", 0xF9), ("buzz", 0x1A8), ("form_change", 0xF7)]
    {
        a.sounds.insert(name.into(), id);
    }
}

/// The asset names the navi chips' modules use (content model v2), with
/// EXE6's numbers.
fn navi_chip_assets(a: &mut nettai_content_api::AssetIndex) {
    let sprite = |c, i| SpriteNumber { category: c, index: i };
    for (name, id) in [
        ("bass_anly", sprite(0x08, 0x13)),
        ("lightning", sprite(0x14, 0x14)),
        ("blast_fire", sprite(0x08, 0x0C)),
        ("charge_car", sprite(0x10, 0x54)),
        ("chargeman", sprite(0x08, 0x05)),
        ("elecman", sprite(0x08, 0x02)),
        ("elmnt_ice", sprite(0x10, 0x0F)),
        ("elmnt_man", sprite(0x08, 0x10)),
        ("heatman", sprite(0x08, 0x01)),
        ("meteor", sprite(0x0C, 0x31)),
        ("moon_beam", sprite(0x0C, 0x64)),
        ("panel_strike", sprite(0x10, 0x26)),
        ("slash_man_effect", sprite(0x10, 0x38)),
        ("slash_wave", sprite(0x10, 0x39)),
        ("slashman", sprite(0x08, 0x03)),
        ("spout_geyser", sprite(0x10, 0x20)),
        ("spout_man_effect", sprite(0x10, 0x23)),
        ("spout_pillar", sprite(0x10, 0x1F)),
        ("spout_splash", sprite(0x0C, 0x1A)),
        ("aquaman", sprite(0x08, 0x06)),
        ("tenguman", sprite(0x08, 0x08)),
        ("tomahawkman", sprite(0x08, 0x07)),
        ("count", sprite(0x08, 0x16)),
        ("django", sprite(0x0C, 0x0F)),
        ("otenko", sprite(0x0C, 0x49)),
        ("dust_storm_mote", sprite(0x10, 0x10)),
    ] {
        a.sprites.insert(name.into(), id);
    }
    for (name, id) in [
        ("aqua_needle_2", 0xB3),
        ("beast_over_burst", 0x12E),
        ("blast_man", 0x17F),
        ("bubble", 0x12D),
        ("charge_man", 0xE3),
        ("col_army_2", 0xB9),
        ("elec_man", 0xC6),
        ("elmnt_man", 0x134),
        ("elmnt_man_2", 0x182),
        ("elmnt_man_4", 0x99),
        ("elmnt_vine", 0x181),
        ("moon_beam", 0x111),
        ("ok_8b", 0x8B),
        ("spout_ball", 0x11D),
        ("spout_man", 0x189),
        ("sun_moon", 0x110),
        ("tengu_man", 0x13C),
        ("tomahawk_man", 0x10A),
    ] {
        a.sounds.insert(name.into(), id);
    }
}

/// Where the EXE6 scripts are (the source overlay in this repository).
const OVERLAY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/exe6");

/// The behavior EXE5 and EXE6 share (content/exelib), which EXE6's modules
/// require by `@exelib/...`: a support pack beside the test content.
const EXELIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/exelib");

/// The support pack's name.
pub const EXELIB_PACK: &str = "exelib";

/// The test content's scripts: its own modules (testdata/content) and these
/// modules of the EXE6 overlay, in memory; whatever they `require` of the
/// overlay is read from it as the load reaches it (the game's folder is
/// content/exe6: a module the test content hasn't is EXE6's).
pub fn scripts() -> Scripts {
    static SCRIPTS: std::sync::OnceLock<Scripts> = std::sync::OnceLock::new();
    SCRIPTS
        .get_or_init(|| {
            let read = |path: &str| {
                let file = format!("{OVERLAY}/{path}.luau");
                std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"))
            };
            let modules = [
                ("objects/attachment/init", "objects/attachment/init"),
                // GunDelSol: the chips, whose actions the SunGuns run.
                ("chips/gun_del_sol/beam", "chips/gun_del_sol/beam"),
                ("chips/gun_del_sol/action", "chips/gun_del_sol/action"),
                ("chips/gun_del_sol/init", "chips/gun_del_sol/init"),
                ("chips/eraseman/mark", "chips/eraseman/mark"),
                ("chips/eraseman/beam", "chips/eraseman/beam"),
                ("chips/eraseman/navi", "chips/eraseman/navi"),
                ("lib/grab/shot", "lib/grab/shot"),
                ("lib/grab/controller", "lib/grab/controller"),
                ("chips/area_grab/init", "chips/area_grab/init"),
                ("chips/panel_grab/init", "chips/panel_grab/init"),
                ("objects/falling_rock/init", "objects/falling_rock/init"),
                ("objects/falling_rock/chip", "objects/falling_rock/chip"),
                ("objects/projectile/init", "objects/projectile/init"),
                ("objects/projectile/variants", "objects/projectile/variants"),
                ("objects/flying_shot/init", "objects/flying_shot/init"),
                ("lib/buster", "lib/buster"),
                // MegaMan's buster, charged and blank shots and HeatCross's
                // charged shot are weapon definitions. (EXE6's rules/roles
                // isn't here: the test pack fills the roles.)
                ("navis/megaman/weapons/blank_shot/init", "navis/megaman/weapons/blank_shot/init"),
                ("navis/megaman/weapons/charged_shot/init", "navis/megaman/weapons/charged_shot/init"),
                ("navis/megaman/weapons/buster/init", "navis/megaman/weapons/buster/init"),
                ("navis/megaman/forms/heat_cross/charge", "navis/megaman/forms/heat_cross/charge"),
                ("lib/weapon", "lib/weapon"),
                ("objects/element_pillar/init", "objects/element_pillar/init"),
                // The form weapons content defines, with the kinds only they
                // spawn.
                (
                    "navis/megaman/weapons/falzar_beast_buster/init",
                    "navis/megaman/weapons/falzar_beast_buster/init",
                ),
                (
                    "navis/megaman/weapons/gregar_beast_buster/init",
                    "navis/megaman/weapons/gregar_beast_buster/init",
                ),
                ("navis/megaman/weapons/tengu_wind/init", "navis/megaman/weapons/tengu_wind/init"),
                ("navis/megaman/forms/aqua_cross_beast/surge", "navis/megaman/forms/aqua_cross_beast/surge"),
                ("navis/megaman/forms/aqua_cross_beast/charge", "navis/megaman/forms/aqua_cross_beast/charge"),
                ("navis/megaman/forms/tengu_cross_beast/whirlwind", "navis/megaman/forms/tengu_cross_beast/whirlwind"),
                ("navis/megaman/forms/tengu_cross_beast/charge", "navis/megaman/forms/tengu_cross_beast/charge"),
                ("navis/megaman/forms/elec_cross/charge", "navis/megaman/forms/elec_cross/charge"),
                ("navis/megaman/forms/elec_cross/a_charge", "navis/megaman/forms/elec_cross/a_charge"),
                ("navis/megaman/forms/tengu_cross/charge", "navis/megaman/forms/tengu_cross/charge"),
                ("navis/megaman/forms/dust_cross/throw_absorbed", "navis/megaman/forms/dust_cross/throw_absorbed"),
                (
                    "navis/megaman/forms/dust_cross_beast/throw_absorbed",
                    "navis/megaman/forms/dust_cross_beast/throw_absorbed",
                ),
                ("navis/megaman/forms/erase_cross/ray", "navis/megaman/forms/erase_cross/ray"),
                ("navis/megaman/forms/erase_cross/charge", "navis/megaman/forms/erase_cross/charge"),
                (
                    "navis/megaman/forms/erase_cross_beast/erase_drop",
                    "navis/megaman/forms/erase_cross_beast/erase_drop",
                ),
                ("navis/megaman/forms/erase_cross_beast/drop", "navis/megaman/forms/erase_cross_beast/drop"),
                ("navis/megaman/forms/tomahawk_cross/charge", "navis/megaman/forms/tomahawk_cross/charge"),
                (
                    "navis/megaman/forms/tomahawk_cross_beast/throw",
                    "navis/megaman/forms/tomahawk_cross_beast/throw",
                ),
                (
                    "navis/megaman/forms/slash_cross_beast/lunge_slash",
                    "navis/megaman/forms/slash_cross_beast/lunge_slash",
                ),
                ("navis/megaman/forms/slash_cross_beast/lunge", "navis/megaman/forms/slash_cross_beast/lunge"),
                ("navis/megaman/dash_hit", "navis/megaman/dash_hit"),
                ("navis/megaman/forms/ground_cross_beast/dash", "navis/megaman/forms/ground_cross_beast/dash"),
                ("navis/megaman/forms/ground_cross/drill", "navis/megaman/forms/ground_cross/drill"),
                ("navis/megaman/forms/charge_cross/tackle", "navis/megaman/forms/charge_cross/tackle"),
                (
                    "navis/megaman/forms/charge_cross_beast/charge_wave",
                    "navis/megaman/forms/charge_cross_beast/charge_wave",
                ),
                ("navis/megaman/forms/charge_cross_beast/wave", "navis/megaman/forms/charge_cross_beast/wave"),
                (
                    "navis/megaman/forms/dust_cross_beast/junk_shot",
                    "navis/megaman/forms/dust_cross_beast/junk_shot",
                ),
                ("navis/megaman/forms/dust_cross_beast/scatter", "navis/megaman/forms/dust_cross_beast/scatter"),
                ("navis/megaman/forms/dust_cross/junk_ball", "navis/megaman/forms/dust_cross/junk_ball"),
                ("navis/megaman/forms/dust_cross/charge", "navis/megaman/forms/dust_cross/charge"),
                ("navis/megaman/weapons/beast_claw/init", "navis/megaman/weapons/beast_claw/init"),
                ("navis/megaman/weapons/absorb/init", "navis/megaman/weapons/absorb/init"),
                (
                    "navis/megaman/forms/slash_cross_beast/hit_flash",
                    "navis/megaman/forms/slash_cross_beast/hit_flash",
                ),
                ("objects/absorbed_obstacle/init", "objects/absorbed_obstacle/init"),
                // The instant chips: EXE6's definitions, and the effects
                // the test chips compose (the plus chips', FireHit's fist,
                // FlmHook's hook).
                ("lib/instant/plus", "lib/instant/plus"),
                // The chips the rules name: the custom screen's Beast Out
                // button, and what a selection it can't allow becomes.
                ("chips/beast_out/init", "chips/beast_out/init"),
                ("chips/invalid/init", "chips/invalid/init"),
                ("chips/attack_10/init", "chips/attack_10/init"),
                ("chips/navi_20/init", "chips/navi_20/init"),
                ("chips/buster_up/init", "chips/buster_up/init"),
                ("chips/full_custom/init", "chips/full_custom/init"),
                ("chips/synchro_trigger/init", "chips/synchro_trigger/init"),
                ("chips/boomerang/boomerang", "chips/boomerang/boomerang"),
                ("chips/boomerang/init", "chips/boomerang/init"),
                ("chips/lance/lance", "chips/lance/lance"),
                ("chips/lance/init", "chips/lance/init"),
                ("chips/fire_hit/fist", "chips/fire_hit/fist"),
                ("chips/sand_worm/worm", "chips/sand_worm/worm"),
                ("chips/sand_worm/hole", "chips/sand_worm/hole"),
                ("chips/sand_worm/spray", "chips/sand_worm/spray"),
                ("chips/sand_worm/init", "chips/sand_worm/init"),
                ("chips/flame_hook/fire", "chips/flame_hook/fire"),
                ("chips/flame_hook/hook", "chips/flame_hook/hook"),
                ("chips/justice_one/strike", "chips/justice_one/strike"),
                ("chips/justice_one/init", "chips/justice_one/init"),
                ("chips/golem_hit/golem", "chips/golem_hit/golem"),
                ("chips/golem_hit/init", "chips/golem_hit/init"),
                ("lib/projectile", "lib/projectile"),
                ("objects/gust/init", "objects/gust/init"),
                // WindRack's action, which TenguCross's charged shot swings
                // with its fan, and DolThdr's, which ElecCross's strikes
                // with (content model v2).
                ("lib/arm", "lib/arm"),
                ("chips/wind_racket/action", "chips/wind_racket/action"),
                ("chips/doll_thunder/action", "chips/doll_thunder/action"),
                ("chips/doll_thunder/doll", "chips/doll_thunder/doll"),
                // SlashCross's charged slash: its waves, the slashes the
                // swords name, its charged shot and A-charge; and the
                // Beast charged chips' pillars' weapons.
                (
                    "navis/megaman/forms/slash_cross/sword_wave",
                    "navis/megaman/forms/slash_cross/sword_wave",
                ),
                ("navis/megaman/forms/slash_cross/slashes", "navis/megaman/forms/slash_cross/slashes"),
                ("navis/megaman/forms/slash_cross/charge", "navis/megaman/forms/slash_cross/charge"),
                (
                    "navis/megaman/weapons/slash_a_charge/init",
                    "navis/megaman/weapons/slash_a_charge/init",
                ),
                ("navis/megaman/forms/heat_cross_beast/charge", "navis/megaman/forms/heat_cross_beast/charge"),
                ("navis/megaman/forms/elec_cross_beast/charge", "navis/megaman/forms/elec_cross_beast/charge"),
                // The Reflectors, the recovery chips and HeatCross's charged
                // shot's burner (content model v2).
                ("chips/reflector/shield", "chips/reflector/shield"),
                ("chips/reflector/shot", "chips/reflector/shot"),
                ("chips/reflector/guard", "chips/reflector/guard"),
                ("chips/reflector/init", "chips/reflector/init"),
                ("chips/recov/init", "chips/recov/init"),
                ("lib/burner/burn", "lib/burner/burn"),
                ("lib/burner/flame", "lib/burner/flame"),
                ("lib/effects", "lib/effects"),
                ("lib/sparks", "lib/sparks"),
                ("rules/collision", "rules/collision"),
                // EXE6's Beast Out turns, which the test rules run.
                ("rules/beast/init", "rules/beast/init"),
                ("rules/emotion/init", "rules/emotion/init"),
                ("rules/beast/rush", "rules/beast/rush"),
                ("rules/lockon_search", "rules/lockon_search"),
                ("rules/beast/berserk", "rules/beast/berserk"),
                // (Its chips are the test content's own: testdata's
                // rules/cross_special.luau.)
                ("rules/beast/cross_special", "rules/beast/cross_special"),
                // (Its tables are the test content's own, the same as EXE6's.)
                ("rules/berserk", "rules/berserk"),
                ("lib/trajectory", "lib/trajectory"),
                ("lib/hp", "lib/hp"),
                // EXE6's chip gate battle (battle flag 0x40), which the
                // Beast Out button and the beast buster test.
                ("lib/chip_gate_battle", "lib/chip_gate_battle"),
                // The bombs and seeds: the chips, whose actions the test
                // chips run.
                ("lib/bombs/throw", "lib/bombs/throw"),
                ("lib/bombs/bomb", "lib/bombs/bomb"),
                ("lib/bombs/seed", "lib/bombs/seed"),
                ("chips/mini_bomb/init", "chips/mini_bomb/init"),
                ("chips/big_bomb/init", "chips/big_bomb/init"),
                ("chips/energy_bomb/init", "chips/energy_bomb/init"),
                ("chips/energy_bomb/burst", "chips/energy_bomb/burst"),
                ("chips/flash_bomb/init", "chips/flash_bomb/init"),
                ("chips/flash_bomb/bomb", "chips/flash_bomb/bomb"),
                ("chips/black_bomb/init", "chips/black_bomb/init"),
                ("chips/black_bomb/bomb", "chips/black_bomb/bomb"),
                ("chips/bug_bomb/init", "chips/bug_bomb/init"),
                ("chips/bug_bomb/bomb", "chips/bug_bomb/bomb"),
                ("chips/grass_seed/init", "chips/grass_seed/init"),
                ("chips/ice_seed/init", "chips/ice_seed/init"),
                ("chips/poison_seed/init", "chips/poison_seed/init"),
                ("chips/little_boiler/init", "chips/little_boiler/init"),
                ("chips/little_boiler/boiler", "chips/little_boiler/boiler"),
                ("chips/little_boiler/layer", "chips/little_boiler/layer"),
                ("chips/voodoo_doll/init", "chips/voodoo_doll/init"),
                ("chips/voodoo_doll/doll", "chips/voodoo_doll/doll"),
                ("chips/voodoo_doll/curse", "chips/voodoo_doll/curse"),
                ("chips/voodoo_doll/sparkles", "chips/voodoo_doll/sparkles"),
                // RskyHny and the dragons (content model v2): definitions.
                ("chips/risky_honey/bee", "chips/risky_honey/bee"),
                ("chips/risky_honey/action", "chips/risky_honey/action"),
                ("chips/risky_honey/init", "chips/risky_honey/init"),
                ("lib/dragons/dragon", "lib/dragons/dragon"),
                ("lib/dragons/body", "lib/dragons/body"),
                ("lib/dragons/head", "lib/dragons/head"),
                ("lib/dragons/action", "lib/dragons/action"),
                ("chips/elec_dragon/init", "chips/elec_dragon/init"),
                // The swords: the chips, whose slashes and strikes the
                // test chips run.
                ("lib/swords/parts", "lib/swords/parts"),
                ("lib/swords/slash", "lib/swords/slash"),
                ("lib/swords/strike", "lib/swords/strike"),
                ("chips/sword/init", "chips/sword/init"),
                ("chips/wide_sword/init", "chips/wide_sword/init"),
                ("chips/long_sword/init", "chips/long_sword/init"),
                ("chips/wide_blade/init", "chips/wide_blade/init"),
                ("chips/long_blade/init", "chips/long_blade/init"),
                ("chips/life_sword/init", "chips/life_sword/init"),
                ("chips/dark_sword/init", "chips/dark_sword/init"),
                ("chips/muramasa_blade/init", "chips/muramasa_blade/init"),
                ("chips/fighter_sword/init", "chips/fighter_sword/init"),
                ("chips/cross_sword/init", "chips/cross_sword/init"),
                ("chips/double_dream/init", "chips/double_dream/init"),
                ("chips/fire_sword/init", "chips/fire_sword/init"),
                ("chips/aqua_sword/init", "chips/aqua_sword/init"),
                ("chips/elec_sword/init", "chips/elec_sword/init"),
                ("chips/bamboo_sword/init", "chips/bamboo_sword/init"),
                ("chips/step_sword/init", "chips/step_sword/init"),
                ("chips/machine_sword/init", "chips/machine_sword/init"),
                ("chips/element_sword/init", "chips/element_sword/init"),
                ("chips/assassin_sword/init", "chips/assassin_sword/init"),
                // Invisibl's hook, which the veil composes.
                ("chips/invisible/controller", "chips/invisible/controller"),
                ("chips/invisible/init", "chips/invisible/init"),
                ("chips/white_capsule_invisible/init", "chips/white_capsule_invisible/init"),
                // The field objects (content model v2): the rock with its
                // debris, RockCube and IceCube, and the stages' boulder.
                ("chips/rock_cube/rock", "chips/rock_cube/rock"),
                ("chips/rock_cube/debris", "chips/rock_cube/debris"),
                ("chips/rock_cube/cube", "chips/rock_cube/cube"),
                ("chips/rock_cube/init", "chips/rock_cube/init"),
                ("objects/boulder/init", "objects/boulder/init"),
                ("objects/encased_bubble/bubble", "objects/encased_bubble/bubble"),
                // The NaviCust supports (content model v2): the controller the
                // rules spawn by role, Rush, Beat, Tango and her heal, with
                // the barrier it raises.
                ("lib/barriers/visual", "lib/barriers/visual"),
                ("lib/barriers/init", "lib/barriers/init"),
                ("lib/supports/heal", "lib/supports/heal"),
                ("lib/supports/tango", "lib/supports/tango"),
                ("lib/supports/beat", "lib/supports/beat"),
                ("lib/supports/rush", "lib/supports/rush"),
                ("lib/supports/controller", "lib/supports/controller"),
                // The trap chips, the navi-changing chips and the gauge
                // chips: the test trap and boosts compose their hooks;
                // SloGauge is EXE6's.
                ("lib/traps/controller", "lib/traps/controller"),
                ("chips/anti_navi/init", "chips/anti_navi/init"),
                ("chips/anti_damage/init", "chips/anti_damage/init"),
                ("chips/anti_sword/init", "chips/anti_sword/init"),
                ("chips/anti_recovery/init", "chips/anti_recovery/init"),
                ("chips/body_guard/init", "chips/body_guard/init"),
                // (HubBatc gives the NaviCust Shield as a B+Back special.)
                ("navis/megaman/weapons/shield/init", "navis/megaman/weapons/shield/init"),
                ("lib/navi_boost/controller", "lib/navi_boost/controller"),
                ("chips/dark_invisible/init", "chips/dark_invisible/init"),
                // HubBatc, and an arm chip with the weapon it makes the
                // charged shot.
                ("chips/hub_batch/init", "chips/hub_batch/init"),
                // (The chips that make their own weapon the charged shot,
                // with the chips' parts those weapons fire.)
                ("lib/navi_boost/charge", "lib/navi_boost/charge"),
                ("chips/punch_arm/charge", "chips/punch_arm/charge"),
                ("chips/punch_arm/init", "chips/punch_arm/init"),
                ("chips/bug_rise_sword/charge", "chips/bug_rise_sword/charge"),
                ("chips/bug_rise_sword/init", "chips/bug_rise_sword/init"),
                ("chips/thunder/ball", "chips/thunder/ball"),
                ("chips/thunder/shoot", "chips/thunder/shoot"),
                ("chips/thunder/init", "chips/thunder/init"),
                ("chips/bug_death_thunder/charge", "chips/bug_death_thunder/charge"),
                ("chips/bug_death_thunder/init", "chips/bug_death_thunder/init"),
                ("chips/aqua_needle/needle", "chips/aqua_needle/needle"),
                ("chips/aqua_needle/volley", "chips/aqua_needle/volley"),
                ("chips/aqua_needle/action", "chips/aqua_needle/action"),
                ("chips/aqua_needle/init", "chips/aqua_needle/init"),
                ("chips/needle_arm/charge", "chips/needle_arm/charge"),
                ("chips/needle_arm/init", "chips/needle_arm/init"),
                ("chips/elec_pulse/pulse", "chips/elec_pulse/pulse"),
                ("chips/elec_pulse/action", "chips/elec_pulse/action"),
                ("chips/pulse_arm/charge", "chips/pulse_arm/charge"),
                ("chips/pulse_arm/init", "chips/pulse_arm/init"),
                ("chips/boomerang_arm/charge", "chips/boomerang_arm/charge"),
                ("chips/boomerang_arm/init", "chips/boomerang_arm/init"),
                ("lib/gauge_speed/controller", "lib/gauge_speed/controller"),
                ("chips/slow_gauge/init", "chips/slow_gauge/init"),
                // Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
                ("objects/rising_bubble/init", "objects/rising_bubble/init"),
                // ElemTrap, the time bombs and Mine: EXE6's definitions,
                // whose hooks the test traps and time bombs run.
                ("chips/element_trap/trap", "chips/element_trap/trap"),
                ("chips/element_trap/strike", "chips/element_trap/strike"),
                ("chips/element_trap/init", "chips/element_trap/init"),
                ("objects/panel_bursts/init", "objects/panel_bursts/init"),
                ("chips/time_bomb_plus/controller", "chips/time_bomb_plus/controller"),
                ("chips/time_bomb_plus/countdown", "chips/time_bomb_plus/countdown"),
                ("chips/time_bomb_plus/init", "chips/time_bomb_plus/init"),
                ("chips/mine/controller", "chips/mine/controller"),
                ("chips/mine/land_mine", "chips/mine/land_mine"),
                ("chips/mine/init", "chips/mine/init"),
                ("chips/crack_shot/shot", "chips/crack_shot/shot"),
                ("chips/crack_shot/init", "chips/crack_shot/init"),
                // The navi chips' navis: each navi and his kinds, which
                // the test navi chips summon.
                ("lib/navi_chips/navi", "lib/navi_chips/navi"),
                // (Each test navi chip's own controller.)
                ("lib/navi_chips/chip", "lib/navi_chips/chip"),
                ("chips/elementman/navi", "chips/elementman/navi"),
                ("chips/elementman/meteor", "chips/elementman/meteor"),
                ("chips/elementman/ice", "chips/elementman/ice"),
                ("chips/elementman/bolt", "chips/elementman/bolt"),
                ("chips/elementman/vine", "chips/elementman/vine"),
                ("chips/aquaman/navi", "chips/aquaman/navi"),
                ("chips/aquaman/ball", "chips/aquaman/ball"),
                ("chips/aquaman/splash", "chips/aquaman/splash"),
                ("chips/aquaman/pillar", "chips/aquaman/pillar"),
                ("chips/aquaman/geyser", "chips/aquaman/geyser"),
                ("chips/aquaman/mark", "chips/aquaman/mark"),
                ("chips/heatman/navi", "chips/heatman/navi"),
                ("chips/heatman/flame", "chips/heatman/flame"),
                ("chips/elecman/navi", "chips/elecman/navi"),
                ("chips/elecman/thunder", "chips/elecman/thunder"),
                ("chips/slashman/navi", "chips/slashman/navi"),
                ("chips/slashman/wave", "chips/slashman/wave"),
                ("chips/chargeman/navi", "chips/chargeman/navi"),
                ("chips/chargeman/car", "chips/chargeman/car"),
                ("chips/tomahawkman/navi", "chips/tomahawkman/navi"),
                ("chips/tenguman/navi", "chips/tenguman/navi"),
                ("chips/blastman/navi", "chips/blastman/navi"),
                ("chips/blastman/fire", "chips/blastman/fire"),
                ("chips/bass/navi", "chips/bass/navi"),
                ("chips/sun_and_moon/sun", "chips/sun_and_moon/sun"),
                ("chips/sun_and_moon/meteor", "chips/sun_and_moon/meteor"),
                ("chips/sun_and_moon/moon_beam", "chips/sun_and_moon/moon_beam"),
                // The Japanese games' Count and Django chips, with their
                // navis, Count's lance and his rain (a dust storm).
                ("chips/count/init", "chips/count/init"),
                ("chips/django/init", "chips/django/init"),
                // The link navis' own chips (whose actions the test link
                // chips run) and their kinds.
                ("lib/link_chips", "lib/link_chips"),
                ("navis/heatman/chip", "navis/heatman/chip"),
                ("navis/elecman/chip", "navis/elecman/chip"),
                ("navis/slashman/chip", "navis/slashman/chip"),
                ("navis/slashman/riding_hit", "navis/slashman/riding_hit"),
                ("navis/eraseman/chip", "navis/eraseman/chip"),
                ("navis/chargeman/chip", "navis/chargeman/chip"),
                ("navis/chargeman/volcano_rock", "navis/chargeman/volcano_rock"),
                ("navis/aquaman/chip", "navis/aquaman/chip"),
                ("navis/aquaman/drip_shower", "navis/aquaman/drip_shower"),
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
                ("navis/aquaman/charge", "navis/aquaman/charge"),
                ("navis/tomahawkman/charge", "navis/tomahawkman/charge"),
                ("navis/tenguman/charge", "navis/tenguman/charge"),
                ("navis/groundman/charge", "navis/groundman/charge"),
                ("navis/dustman/charge", "navis/dustman/charge"),
                ("navis/groundman/drill", "navis/groundman/drill"),
                ("navis/dustman/clouds", "navis/dustman/clouds"),
                ("navis/groundman/chip", "navis/groundman/chip"),
                ("chips/groundman/drill", "chips/groundman/drill"),
                ("chips/groundman/rock", "chips/groundman/rock"),
                ("objects/panel_strike/init", "objects/panel_strike/init"),
                ("chips/drill_arm/drill", "chips/drill_arm/drill"),
                ("chips/doll_thunder/column", "chips/doll_thunder/column"),
                // The dimming chips of subtypes 4, 5, 9, 13, 26, 27, 28 and 36
                // (content model v2): the barriers, the panel chips, the
                // instruments, AirRaid, BugFix, ColorPt, Sensor and SumnBlk.
                ("lib/barriers/controller", "lib/barriers/controller"),
                ("chips/barrier/init", "chips/barrier/init"),
                ("chips/bubble_wrap/init", "chips/bubble_wrap/init"),
                ("chips/life_aura/init", "chips/life_aura/init"),
                ("chips/bug_fix/init", "chips/bug_fix/init"),
                ("chips/bug_fix/controller", "chips/bug_fix/controller"),
                ("chips/bug_fix/glow", "chips/bug_fix/glow"),
                ("objects/panel_changer/init", "objects/panel_changer/init"),
                ("lib/panel_chips/controller", "lib/panel_chips/controller"),
                ("chips/panel_return/init", "chips/panel_return/init"),
                ("chips/holy_panel/init", "chips/holy_panel/init"),
                ("chips/sanctuary/init", "chips/sanctuary/init"),
                ("chips/coming_road/init", "chips/coming_road/init"),
                ("chips/going_road/init", "chips/going_road/init"),
                ("lib/instruments/instrument", "lib/instruments/instrument"),
                ("lib/instruments/controller", "lib/instruments/controller"),
                ("chips/fanfare/init", "chips/fanfare/init"),
                ("chips/discord/init", "chips/discord/init"),
                ("chips/timpani/init", "chips/timpani/init"),
                ("chips/silence/init", "chips/silence/init"),
                ("chips/sensor/init", "chips/sensor/init"),
                ("chips/sensor/controller", "chips/sensor/controller"),
                ("chips/sensor/turret", "chips/sensor/turret"),
                ("chips/sensor/scanner", "chips/sensor/scanner"),
                ("chips/sensor/laser", "chips/sensor/laser"),
                ("chips/air_raid/init", "chips/air_raid/init"),
                ("chips/air_raid/controller", "chips/air_raid/controller"),
                ("chips/air_raid/plane", "chips/air_raid/plane"),
                ("chips/air_raid/propeller", "chips/air_raid/propeller"),
                ("chips/summon_black/init", "chips/summon_black/init"),
                ("chips/summon_black/controller", "chips/summon_black/controller"),
                ("chips/summon_black/navi", "chips/summon_black/navi"),
                ("chips/color_point/init", "chips/color_point/init"),
                ("chips/color_point/controller", "chips/color_point/controller"),
                ("chips/color_point/point", "chips/color_point/point"),
                // The Gregar and Falzar chips (the Japanese ROMs' routines).
                ("chips/gregar/init", "chips/gregar/init"),
                ("chips/falzar/init", "chips/falzar/init"),
            ];
            let modules = modules.iter().map(|&(to, from)| (to.to_string(), from.to_string()));
            let own = modules_under(TEST_CONTENT);
            let all: std::collections::BTreeMap<String, String> = modules.map(|(to, from)| (to, read(&from))).chain(own).collect();
            // (What they require of the overlay is read as it is required;
            // what they require of content/exelib is the support pack's.)
            let mut scripts = Scripts::default();
            add_shared(&mut scripts);
            scripts.add_game(ROOT, all);
            scripts.read_from(ROOT, OVERLAY);
            scripts
        })
        .clone()
}

/// The test content's rules: what its rules state
/// (testdata/content/rules: a made-up game's, every rule its own choice;
/// where games differ it plays as EXE6 does, with EXE5's three panel types
/// beside EXE6's). For a test that wants a rule's value as the test content
/// has it, or that makes a content of a few modules and small rules of
/// its own and gives it these as its Rust tables (`Content::base_rules`).
pub fn rules() -> Rules {
    content().rules().clone()
}

/// Side `side`'s bug frags in battle `b`: the rules' state (the test
/// content's rules have EXE6's rules/dark_chips).
pub fn bug_frags(b: &crate::Battle, side: u8) -> u32 {
    let (schema, state) = b.rules_state(side).expect("the test content's rules");
    match state.get(schema, schema.index_of("bug_frags").expect("its bug frags")) {
        nettai_content_api::FieldValue::U32(n) => n,
        other => panic!("bug frags {other:?}"),
    }
}

/// Give side `side` `n` bug frags in battle `b` (the rules' state).
pub fn set_bug_frags(b: &mut crate::Battle, side: u8, n: u32) {
    let content = b.content.clone();
    let schema = content.defs.schema(content.defs.rules().expect("the rules").state);
    let i = schema.index_of("bug_frags").expect("its bug frags");
    let state = b.rules[side as usize].state.as_mut().expect("the side's rules");
    state.set(schema, i, nettai_content_api::Value::Int(n as i64)).expect("a count of bug frags");
}
