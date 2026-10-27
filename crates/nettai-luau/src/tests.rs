//! The define phase and loading, on small packs written here.

use super::*;
use nettai_content_api::Data;

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn relative_paths_resolve_within_the_pack() {
    let r = keys::resolve;
    assert_eq!(r("exe6:chips/gundels/beam", "../../objects/sun-beam/sun_beam").unwrap(), "exe6:objects/sun-beam/sun_beam");
    // A folder's init.luau is the folder as a module: beside it is `./`,
    // inside it `@self/`.
    assert_eq!(r("exe6:chips/gundels/init", "../objects/sun-beam/sun_beam").unwrap(), "exe6:objects/sun-beam/sun_beam");
    assert_eq!(r("exe6:chips/gundels/init", "@self/beam").unwrap(), "exe6:chips/gundels/beam");
    assert_eq!(r("exe6:(pack)", "./lib/slot").unwrap(), "exe6:lib/slot");
    assert!(r("exe6:lib/slot", "../../x").is_err());
    assert!(r("exe6:lib/slot", "objects/x").is_err());
    // A pack's top: its own, or another's (which `packs::check_require` may refuse).
    assert_eq!(r("exe6:chips/x/chip", "@exe6/lib/slot").unwrap(), "exe6:lib/slot");
    assert_eq!(r("exe6:chips/x/chip", "@exelib/swords/slash").unwrap(), "exelib:swords/slash");
    assert!(r("exe5:rules/definition", "@exe6/../x").is_err());
}

/// docs/design/content-model-v2.md §4.0: a game requires itself and the
/// support packs it depends on; a support pack itself and the support packs
/// it depends on; no pack another game's.
#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn a_require_reaches_only_its_pack_and_the_support_packs_it_depends_on() {
    use nettai_content_api::{PackKind, PackManifest};
    let manifest = |id: &str, kind: PackKind, depends: &[&str]| PackManifest {
        id: id.into(),
        kind,
        depends: depends.iter().map(|u| u.to_string()).collect(),
    };
    let packs = [manifest("lib", PackKind::Support, &[]), manifest("a", PackKind::Game, &["lib"]), manifest("b", PackKind::Game, &["lib"])];
    let load = |entry: &str, modules: &[(&str, &str)]| {
        let pack = Pack::new(modules.iter().map(|(n, s)| (n.to_string(), s.to_string())))
            .with_entries(vec![entry.to_string()])
            .with_packs(packs.clone());
        define(&pack, Options::default()).map(|_| ()).map_err(|e| e.message)
    };
    let lib = ("lib:y", "return {}");
    load("a:x", &[("a:x", "local _ = require('@lib/y')\nreturn {}"), lib]).unwrap();
    let e = load("a:x", &[("a:x", "local _ = require('@b/y')\nreturn {}"), ("b:y", "return {}")]).unwrap_err();
    assert!(e.contains("a/x.luau: require(\"@b/y\"): b is a game pack"), "game to game: {e}");
    let e = load("lib:y", &[("lib:y", "local _ = require('@a/x')\nreturn {}"), ("a:x", "return {}")]).unwrap_err();
    assert!(e.contains("lib/y.luau: require(\"@a/x\"): a is a game pack"), "support to game: {e}");
}

/// docs/design/content-model-v2.md §4.0: a support pack's modules run
/// without the playing game's context (`asset`, `rules`), which reaches
/// them only as their callers' arguments; a game's modules have it.
#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn a_support_pack_has_no_game_context() {
    use nettai_content_api::{PackKind, PackManifest};
    let manifest = |id: &str, kind: PackKind, depends: &[&str]| PackManifest {
        id: id.into(),
        kind,
        depends: depends.iter().map(|u| u.to_string()).collect(),
    };
    let packs = [manifest("lib", PackKind::Support, &[]), manifest("game", PackKind::Game, &["lib"])];
    let load = |modules: &[(&str, &str)]| {
        let pack = Pack::new(modules.iter().map(|(n, s)| (n.to_string(), s.to_string())))
            .with_entries(vec![modules[0].0.to_string()])
            .with_packs(packs.clone());
        define(&pack, Options::default()).map(|_| ()).map_err(|e| e.message)
    };
    // A game's module names its assets, defines, and reaches its systems'
    // library; and passes a look to the support pack's maker.
    let maker = "return function(look: any) return new.effect { sprite = look.sprite, anim = 0 } end";
    load(&[
        (
            "game:x",
            "local make = require('@lib/m')\nlocal _ = rules.state\nreturn make({ sprite = asset.sprite('bomb', { { 3 } }) })",
        ),
        ("lib:m", maker),
    ])
    .unwrap();
    let refused = |e: &str, name: &str, module: &str| {
        assert!(e.contains(&format!("{module}: a support pack has no `{name}`")), "{module} naming `{name}`: {e}");
    };
    // At a support module's top.
    let e = load(&[("game:x", "return require('@lib/m')"), ("lib:m", "return asset.sprite('bomb')")]).unwrap_err();
    refused(&e, "asset", "lib:m");
    let e = load(&[("game:x", "return require('@lib/m')"), ("lib:m", "return rules.side")]).unwrap_err();
    refused(&e, "rules", "lib:m");
    // After a game's module has named the asset: Luau resolves a module's
    // globals against the VM's when it loads, so the support pack's must
    // be missing there too, not only in its environment.
    let e = load(&[
        ("game:x", "local s = asset.sound('throw')\nreturn require('@lib/m')"),
        ("lib:m", "return asset.sound('throw')"),
    ])
    .unwrap_err();
    refused(&e, "asset", "lib:m");
    // In a support pack's function, though a game's module calls it while
    // it loads: a function keeps its module's environment.
    let e = load(&[
        ("game:x", "local make = require('@lib/m')\nreturn make()"),
        ("lib:m", "return function() return new.effect { sprite = asset.sound('throw'), anim = 0 } end"),
    ])
    .unwrap_err();
    refused(&e, "asset", "lib:m");
}

/// `rules.state_of` is a game's rules' (its API module's): a module outside
/// rules/ calling it is refused, naming the module (As built S8).
#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn another_sides_rules_state_is_for_the_games_rules() {
    let call = "local s = rules.state_of(0)\nlocal _ = s.bug_frags\nreturn {}";
    let e = define_named(&[("chips/x/chip", call)]).unwrap_err();
    assert!(e.contains("test:chips/x/chip: rules.state_of is a game's rules'"), "{e}");
    // (Under rules/ it passes the guard: here no battle runs, so the read
    // of a field is refused.)
    let e = define_named(&[("rules/api", call)]).unwrap_err();
    assert!(!e.contains("is a game's rules'") && e.contains("only reachable while content runs"), "{e}");
}

fn pack(modules: &[(&str, &str)]) -> Pack {
    Pack::root("test", modules.iter().map(|(p, s)| (p.to_string(), s.to_string())))
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn errors_unwind_and_leave_the_vm_usable() {
    let lua = sandbox::new_vm(false).unwrap();
    let fail = lua.create_function(|_, ()| -> mlua::Result<()> { Err(mlua::Error::runtime("host failure")) }).unwrap();
    lua.globals().set("fail", fail).unwrap();
    for _ in 0..32 {
        assert!(lua.load("error('script failure')").exec().unwrap_err().to_string().contains("script failure"));
        assert!(lua.load("fail()").exec().unwrap_err().to_string().contains("host failure"));
        let caught: bool = lua.load("local ok = pcall(fail); return not ok").eval().unwrap();
        assert!(caught);
        assert!(lua.load("local =").exec().is_err());
        assert_eq!(lua.load("return 6 * 7").eval::<i64>().unwrap(), 42);
        lua.gc_collect().unwrap();
    }
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn runaway_content_is_interrupted_and_a_new_pack_still_loads() {
    let options = Options { budget: 10, ..Options::default() };
    let e = define(&pack(&[("loop", "while true do end")]), options).unwrap_err();
    assert!(e.message.contains("ran past its budget"), "{}", e.message);
    assert!(define_pack(&[("ok", "return new.effect { anim = 0 }")]).is_ok());
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn definition_integers_keep_their_values_on_32_bit_targets() {
    let defs = define_pack(&[("numbers", "return new.record('numbers', { low = -2147483649, high = 4294967295, [4294967295] = 123 })")]).unwrap();
    let record = &defs.of(Registry::Record)[0].spec;
    assert_eq!(record.field("low"), &Data::Int(-2147483649));
    assert_eq!(record.field("high"), &Data::Int(4294967295));
    let Data::Map(fields) = record else { panic!("a record is a map") };
    assert!(fields.contains(&(nettai_content_api::data::Key::Int(4294967295), Data::Int(123))));
}

fn define_pack(modules: &[(&str, &str)]) -> Result<Definitions, String> {
    define(&pack(modules), Options::default()).map(|(d, _, _)| d).map_err(|e| e.message)
}

/// A family library, a kind, and chips composing them: MiniBomb's pattern
/// (docs/design/content-model-v2.md §5.1).
const BOMBS: &[(&str, &str)] = &[
    (
        "lib/bombs/throw",
        r#"--!strict
local throw = {}
-- One state for every throw action.
local STATE = { timer = "u16" }
function throw.action(spec: any): any
    return new.action {
        state = STATE,
        args = spec,
        update = function(me: any, s: any) end,
    }
end
return throw
"#,
    ),
    (
        "lib/bombs/bomb",
        r#"--!strict
local bomb = {}
local EXPLOSION = new.effect { anim = 0 }
bomb.kind = new.kind {
    id = "bomb",
    pool = "attack",
    state = { variant = "u16" },
    update = function(me: any) return EXPLOSION end,
}
function bomb.variant(v: any): any
    return new.record("bomb-variant", v)
end
return bomb
"#,
    ),
    (
        "chips/minibomb/init",
        r#"--!strict
local throw = require("../lib/bombs/throw")
local bomb = require("../lib/bombs/bomb")
local chip = {
    name = "MiniBomb",
    codes = { "B", "L", "R", "*" },
    action = throw.action { held = 4, thrower = bomb.variant { palette = 0 } },
}
return { minibomb = chip }
"#,
    ),
    (
        "chips/flshbom/init",
        r#"--!strict
local throw = require("../lib/bombs/throw")
local bomb = require("../lib/bombs/bomb")
local THROW = throw.action { held = 0x2E, thrower = bomb.variant { palette = 1 } }
return {
    flshbom1 = { name = "FlshBom1", action = THROW },
    flshbom2 = { name = "FlshBom2", action = THROW },
}
"#,
    ),
    // The game's root: its chips by id.
    (
        "init",
        r#"--!strict
local minibomb = require("@self/chips/minibomb")
local flshbom = require("@self/chips/flshbom")
return { chips = { minibomb = minibomb.minibomb, flshbom1 = flshbom.flshbom1, flshbom2 = flshbom.flshbom2 } }
"#,
    ),
];

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn definitions_get_keys_from_the_root_ids_modules_and_paths() {
    let d = define_pack(BOMBS).unwrap();
    let keys = |r: Registry| d.of(r).iter().map(|d| d.key.as_str()).collect::<Vec<_>>();
    // The root's: their ids there.
    assert_eq!(keys(Registry::Chip), ["flshbom1", "flshbom2", "minibomb"]);
    // A tagged table's id.
    assert_eq!(keys(Registry::Kind), ["bomb"]);
    // An action a chip holds: its path from the chip; the series' shared
    // action, from its first chip.
    assert_eq!(keys(Registry::Action), ["flshbom1/action", "minibomb/action"]);
    // Nested deeper: the variant inside the action's arguments.
    assert_eq!(keys(Registry::Record), ["flshbom1/action/args/thrower", "minibomb/action/args/thrower"]);
    // Captured by a function: the name it captures it by.
    assert_eq!(keys(Registry::Effect), ["bomb/update/EXPLOSION"]);
    // The throw's state table is one schema for both actions; the kind's
    // its own.
    assert_eq!(keys(Registry::Schema), ["action:flshbom1/action/state", "kind:bomb/state"]);
    let action = d.get(Registry::Action, "minibomb/action").unwrap();
    assert_eq!(action.spec.field("state"), &Data::Ref(Registry::Schema, "action:flshbom1/action/state".into()));
    assert_eq!(action.spec.field("update"), &Data::Function);
    let chip = d.get(Registry::Chip, "minibomb").unwrap();
    assert_eq!(chip.spec.field("action"), &Data::Ref(Registry::Action, "minibomb/action".into()));
    assert_eq!(chip.module, "test:chips/minibomb/init");
    let record = d.get(Registry::Record, "minibomb/action/args/thrower").unwrap();
    assert_eq!(record.record_type.as_deref(), Some("bomb-variant"));
}

/// A root's sections are the ones the packs' tools know (`packs::SECTIONS`,
/// by which tools/content/index.py and a held game's init find a
/// section's modules).
#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn a_roots_sections_are_the_packs() {
    let ours: Vec<&str> = define::SECTIONS.iter().map(|(s, _)| *s).collect();
    let packs: Vec<&str> = nettai_content_api::packs::SECTIONS.iter().map(|(s, _)| *s).collect();
    assert_eq!(ours, packs);
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn the_define_phase_reads_the_same_whatever_the_order() {
    let a = define_pack(BOMBS).unwrap();
    let mut reversed = BOMBS.to_vec();
    reversed.reverse();
    assert_eq!(define_pack(&reversed).unwrap(), a);
    assert_eq!(define_pack(BOMBS).unwrap(), a);
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn definition_mistakes_are_load_errors() {
    let cases: &[(&str, &str)] = &[
        ("return { chips = { X = {} } }", "\"X\" is not a valid id"),
        ("return new.kind { id = 'Bomb', pool = 'attack' }", "not a valid id"),
        ("return { chips = { x = { damage = 1.5 } } }", "1.5 is not an integer"),
        ("local t = {}\nt.me = t\nreturn { chips = { x = { loop = t } } }", "contains itself"),
        ("return new.kind(3)", "takes a table"),
        ("local s = new.kind { pool = 'attack' }\nreturn new.kind(s)", "tagged already"),
        // A root's definition is named by its key there, and is no tagged
        // table.
        ("return { chips = { x = { id = 'x' } } }", "a chip's id is its key in `chips`"),
        ("return { chips = { x = new.kind {} } }", "a kind (new.kind), not a chip"),
        // No definer: what a match names is a plain table in the root.
        ("return define.chip { id = 'x' }", "attempt to index nil"),
        // The migration's markers are gone: neither the global nor the
        // field.
        ("return new.kind { legacy = { number = 1 } }", "takes no `legacy` field"),
        ("return new.kind { marker = legacy { number = 1 } }", "attempt to call a nil value"),
    ];
    for (source, want) in cases {
        let e = define_pack(&[("chips/x/chip", source)]).expect_err(source);
        assert!(e.contains(want), "{source}: {e}");
    }
    let e = define_pack(&[("a", "return new.kind { id = 'x' }"), ("b", "return new.kind { id = 'x' }")]).unwrap_err();
    assert!(e.contains("kind \"x\" is two tables: one made in test/a.luau, one in test/b.luau"), "{e}");
    // What nothing reaches is no definition.
    let d = define_pack(&[("a", "local unreached = new.kind { id = 'x' }\nreturn {}")]).unwrap();
    assert!(d.of(Registry::Kind).is_empty());
}

fn define_named(modules: &[(&str, &str)]) -> Result<Definitions, String> {
    define_assets(modules).map(|(d, _)| d)
}

fn define_assets(modules: &[(&str, &str)]) -> Result<(Definitions, nettai_content_api::DefinedAssets), String> {
    define(&pack(modules), Options::default()).map(|(d, _, a)| (d, a)).map_err(|e| e.message)
}

/// docs/design/content-model-v2.md §6.3: content names its assets while it
/// loads, one value per asset, and defines its sprites (`asset.sprite(name,
/// animations)`) where their modules are; the define phase interns the
/// names, each kind's in byte order, and keeps each sprite's definition.
#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn assets_resolve_by_name_while_content_loads() {
    let (d, assets) = define_assets(&[
        ("sprites/explosion", "return asset.sprite('explosion', { { 3, 3 }, { 2, 2, 4, loop = true } })"),
        ("sprites/bomb", "return asset.sprite('bomb', {})"),
        (
            "lib/effects",
            "local SOUND = asset.sound('throw')\n\
             if asset.sound('throw') ~= SOUND then error('one value per asset') end\n\
             local _ = require('./../sprites/bomb')\n\
             return { explosion = new.effect { sprite = require('./../sprites/explosion'), anim = 1, sound = SOUND } }",
        ),
    ])
    .unwrap();
    let e = &d.of(Registry::Effect)[0];
    assert_eq!(e.spec.field("sprite"), &Data::Asset(nettai_content_api::AssetKind::Sprite, "explosion".into()));
    assert_eq!(e.spec.field("sound"), &Data::Asset(nettai_content_api::AssetKind::Sound, "throw".into()));
    assert_eq!(assets.names.sprites, ["bomb", "explosion"], "in byte order, whatever order they loaded in");
    assert_eq!(assets.names.sounds, ["throw"]);
    let anim = |frames: &[u8], looping| nettai_content_api::SpriteAnimation { frames: frames.to_vec(), looping };
    assert_eq!(assets.sprites[1].animations, [anim(&[3, 3], false), anim(&[2, 2, 4], true)]);
    assert!(assets.sprites[0].animations.is_empty() && assets.sprites[0].parts.is_empty());
    // A sprite is defined once, by its module.
    let e = define_named(&[("a", "return asset.sprite('bomb', { { 1 } })"), ("b", "return asset.sprite('bomb', { { 1 } })")]).unwrap_err();
    assert!(e.contains("sprite \"bomb\" is defined twice: in test/a.luau and in test/b.luau"), "{e}");
    let e = define_named(&[("chips/x/chip", "return { s = asset.sprite('bomb') }")]).unwrap_err();
    assert!(e.contains("chips/x/chip: sprite \"bomb\": asset.sprite(name, animations) takes the sprite's animations"), "{e}");
    // An animation is its frames' durations, 1 to 255, and `loop`.
    for (animations, wrong) in [
        ("{ { 0 } }", "frame 0 is 0, not 1 to 255"),
        ("{ { 3, 256 } }", "frame 1 is 256, not 1 to 255"),
        ("{ {} }", "animation 0 has no frame"),
        ("{ { 3, last = true } }", "holds its frames' durations (1, 2, ...) and `loop`, not last"),
        ("{ { 3, loop = 1 } }", "`loop` is a integer, not a boolean"),
        ("{ [2] = { 3 } }", "the animations are a list"),
    ] {
        let e = define_named(&[("m", &format!("return asset.sprite('bomb', {animations})"))]).unwrap_err();
        assert!(e.contains(wrong), "{animations}: {e}");
    }
    // Other assets are named alone.
    let e = define_named(&[("m", "return asset.sound('throw', {})")]).unwrap_err();
    assert!(e.contains("asset.sound takes a name alone"), "{e}");
    let e = define_named(&[("m", "return { f = function() return asset.sound('throw') end }")]);
    assert!(e.is_ok(), "calling it later is the runtime's error, not the define phase's");
}

/// A sprite states the parts content reads (`sprite:part_offset(n)`): by
/// part number, each animation's frames' offsets, as many as its frames.
#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn a_sprite_states_the_parts_content_reads() {
    let define_parts = |parts: &str| {
        define_assets(&[("m", &format!("return asset.sprite('plane', {{ {{ 6, loop = true }}, {{ 8, 2 }} }}, {{ parts = {parts} }})"))])
    };
    let (_, assets) = define_parts("{ [1] = { { { 18, -20 } }, { { 18, -20 }, { 14, -20 } } } }").unwrap();
    assert_eq!(assets.sprites[0].parts[&1], [vec![(18, -20)], vec![(18, -20), (14, -20)]]);
    let e = define_parts("{ [1] = { { { 18, -20 } }, { { 18, -20 } } } }").unwrap_err();
    assert!(e.contains("part 1, animation 1 has 1 offsets, and the animation 2 frames"), "{e}");
    let e = define_parts("{ [1] = { { { 18, -20 } } } }").unwrap_err();
    assert!(e.contains("part 1 has offsets for 1 animations, and the sprite has 2"), "{e}");
    let e = define_parts("{ [1] = { { { 200, 0 } }, { { 0, 0 }, { 0, 0 } } } }").unwrap_err();
    assert!(e.contains("offset is 200, not -128 to 127"), "{e}");
}

/// A game's rules are one definition, its rules (rules/init.luau), which
/// takes no id and is keyed `rules`:
/// its rule sections and its roles are plain tables in it, whose
/// definitions are references (docs/design/content-model-v2.md §3.8).
#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn a_games_rules_are_one_definition() {
    let d = define_named(&[
        ("lib/counter", "return new.action { id = 'counter', state = {}, update = function(me, s) end }"),
        ("rules/roles", "return { actions = { anti_damage_counter = require('../lib/counter') } }"),
        ("rules/pools", "return { actor = 32, attack = 32, effect = 32 }"),
        ("rules/init", "return { pools = require('@self/pools'), roles = require('@self/roles') }"),
        ("init", "return { rules = require('@self/rules') }"),
    ])
    .unwrap();
    let rules = d.get(Registry::Rules, nettai_content_api::RULESET_KEY).expect("the game's rules");
    assert_eq!(rules.module, "test:rules/init");
    assert_eq!(rules.spec.field("roles").field("actions").field("anti_damage_counter"), &Data::Ref(Registry::Action, "counter".into()));
    assert_eq!(rules.spec.field("pools").field("actor"), &Data::Int(32));
    // A plain table no root holds is no rules.
    let d = define_named(&[("rules/init", "return { pools = { actor = 1 } }")]).unwrap();
    assert!(d.get(Registry::Rules, nettai_content_api::RULESET_KEY).is_none());
    // A game has one.
    let e = define_named(&[("a", "return { rules = {} }"), ("b", "return { rules = {} }")]).unwrap_err();
    assert!(e.contains("a game has one rules definition"), "{e}");
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn definitions_are_frozen_and_tags_close_after_loading() {
    let p = pack(&[(
        "m",
        "local d = new.record('r', { n = 1 })\nreturn { d = d, late = function() return new.record('r', {}) end }",
    )]);
    let (lua, defined, modules, _, _) = open(&p, Options::default()).unwrap();
    assert!(defined.tables.iter().all(|t| t.is_readonly()));
    let LuaValue::Table(m) = &modules["test:m"] else { panic!("a table") }; // (written in full)
    let late: Function = m.get("late").unwrap();
    BUDGET.with(|b| b.set(1000));
    let e = late.call::<LuaValue>(()).unwrap_err().to_string();
    assert!(e.contains("tables are tagged while content loads"), "{e}");
    drop(lua);
}

/// Coverage (src/coverage.rs): the modules whose code ran while recording,
/// on this thread; nothing when not recording.
#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn coverage_records_the_modules_that_ran() {
    let p = pack(&[
        ("lib/twice", "return { twice = function(n) return n * 2 end, unused = function() return 0 end }"),
        ("m", "local lib = require('./lib/twice')\nreturn { f = function(n) for i = 1, 2 do n = lib.twice(n) end return n end }"),
        ("other", "return { g = function() return 1 end }"),
    ]);
    let (lua, _, modules, _, _) = open(&p, Options::default()).unwrap();
    let LuaValue::Table(m) = &modules["test:m"] else { panic!("a table") }; // (written in full)
    let f: Function = m.get("f").unwrap();
    BUDGET.with(|b| b.set(1000));
    assert_eq!(f.call::<i64>(3).unwrap(), 12);
    assert_eq!(coverage::take(), coverage::Ran::default(), "nothing recorded unless started");
    coverage::start();
    assert_eq!(f.call::<i64>(3).unwrap(), 12);
    let ran = coverage::take();
    assert_eq!(ran.modules.into_iter().collect::<Vec<_>>(), ["test:lib/twice", "test:m"]); // (written in full)
    // Stopped: the next call isn't recorded.
    assert_eq!(f.call::<i64>(3).unwrap(), 12);
    assert_eq!(coverage::take(), coverage::Ran::default());
    drop(lua);
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn a_plan_binds_definition_slots() {
    let modules = BOMBS.to_vec();
    let p = pack(&modules);
    let (definitions, compiled, assets) = define(&p, Options::default()).unwrap();
    assert_eq!(compiled.len(), modules.len(), "every module compiled");
    let p = p.with_compiled(compiled);
    let handles = (0..definitions.defs.len() as u16).collect();
    let plan = BindPlan {
        functions: vec![
            FnSource::slot(Registry::Kind, "bomb", "update"),
            FnSource::slot(Registry::Action, "minibomb/action", "update"),
        ],
        schemas: Vec::new(),
        definitions,
        handles,
        entries: Vec::new(),
        assets: assets.names,
    };
    assert!(LuauContent::load(&p, &plan, Options::default()).is_ok());
    // A plan made from other assets is refused.
    let mut other = plan.clone();
    other.assets.sounds.push("zzz".into());
    let e = LuauContent::load(&p, &other, Options::default()).err().expect("refused").message;
    assert!(e.contains("name other assets"), "{e}");
    // A plan made from other definitions is refused.
    let mut stale = plan.clone();
    stale.definitions.defs.pop();
    stale.handles.pop();
    let e = LuauContent::load(&p, &stale, Options::default()).err().expect("refused").message;
    assert!(e.contains("define something other"), "{e}");
    // A slot that isn't a function is refused.
    let mut wrong = plan.clone();
    wrong.functions.push(FnSource::slot(Registry::Chip, "minibomb", "name"));
    let e = LuauContent::load(&p, &wrong, Options::default()).err().expect("refused").message;
    assert!(e.contains("chip minibomb: `name` is string, not a function"), "{e}");
}
