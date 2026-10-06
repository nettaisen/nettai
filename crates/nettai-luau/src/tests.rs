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
        text: Vec::new(),
    };
    let packs = [manifest("lib", PackKind::Support, &[]), manifest("a", PackKind::Game, &["lib"]), manifest("b", PackKind::Game, &["lib"])];
    let load = |entry: &str, modules: &[(&str, &str)]| {
        let pack = Pack::new(modules.iter().map(|(n, s)| (n.to_string(), s.to_string())))
            .with_entries(vec![entry.to_string()])
            .with_packs(packs.clone());
        define(&pack, &AssetNames::default(), Options::default()).map(|_| ()).map_err(|e| e.message)
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
        text: Vec::new(),
    };
    let packs = [manifest("lib", PackKind::Support, &[]), manifest("game", PackKind::Game, &["lib"])];
    let load = |modules: &[(&str, &str)]| {
        let pack = Pack::new(modules.iter().map(|(n, s)| (n.to_string(), s.to_string())))
            .with_entries(vec![modules[0].0.to_string()])
            .with_packs(packs.clone());
        define(&pack, &names(), Options::default()).map(|_| ()).map_err(|e| e.message)
    };
    // A game's module names its assets, defines, and reaches its systems'
    // library; and passes a look to the support pack's maker.
    let maker = "return function(look: any) return new.effect { sprite = look.sprite, anim = 0 } end";
    load(&[
        (
            "game:x",
            "local make = require('@lib/m')\nlocal _ = rules.state\nreturn make({ sprite = asset.sprite('bomb') })",
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
        ("game:x", "local s = asset.sprite('bomb')\nreturn require('@lib/m')"),
        ("lib:m", "return asset.sprite('bomb')"),
    ])
    .unwrap_err();
    refused(&e, "asset", "lib:m");
    // In a support pack's function, though a game's module calls it while
    // it loads: a function keeps its module's environment.
    let e = load(&[
        ("game:x", "local make = require('@lib/m')\nreturn make()"),
        ("lib:m", "return function() return new.effect { sprite = asset.sprite('bomb'), anim = 0 } end"),
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
    let e = define(&pack(&[("loop", "while true do end")]), &AssetNames::default(), options).unwrap_err();
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
    define(&pack(modules), &AssetNames::default(), Options::default()).map(|(d, _)| d).map_err(|e| e.message)
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

/// Asset names for these tests: the `test` pack's.
fn names() -> AssetNames {
    let mut a = nettai_content_api::PackIndex::default();
    a.sprites.insert("bomb".into(), nettai_content_api::PackSprite { category: 0x0C, index: 2 });
    a.sprites.insert("explosion".into(), nettai_content_api::PackSprite { category: 0x0C, index: 1 });
    a.sounds.insert("throw".into(), 0x1A6);
    AssetNames::of_pack("test", a)
}

fn define_named(modules: &[(&str, &str)]) -> Result<Definitions, String> {
    define(&pack(modules), &names(), Options::default()).map(|(d, _)| d).map_err(|e| e.message)
}

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn assets_resolve_by_name_while_content_loads() {
    let d = define_named(&[(
        "lib/effects",
        "local SOUND = asset.sound('throw')\n\
         if asset.sprite('bomb') ~= asset.sprite('bomb') then error('one value per asset') end\n\
         return { explosion = new.effect { sprite = asset.sprite('explosion'), anim = 3, sound = SOUND } }",
    )])
    .unwrap();
    let e = &d.of(Registry::Effect)[0];
    assert_eq!(e.spec.field("sprite"), &Data::Asset(nettai_content_api::AssetKind::Sprite, "explosion".into()));
    assert_eq!(e.spec.field("sound"), &Data::Asset(nettai_content_api::AssetKind::Sound, "throw".into()));
    // An unknown name is an error naming the module; so is a resolver
    // called after loading.
    let e = define_named(&[("chips/x/chip", "return { s = asset.sprite('bom') }")]).unwrap_err();
    assert!(e.contains("chips/x/chip: no sprite is named \"bom\""), "{e}");
    let e = define_named(&[("m", "return { f = function() return asset.sound('throw') end }")]);
    assert!(e.is_ok(), "calling it later is the runtime's error, not the define phase's");
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
    let (lua, defined, modules, _, _) = open(&p, &AssetNames::default(), Options::default()).unwrap();
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
    let (lua, _, modules, _, _) = open(&p, &AssetNames::default(), Options::default()).unwrap();
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
    let (definitions, compiled) = define(&p, &AssetNames::default(), Options::default()).unwrap();
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
        assets: AssetNames::default(),
    };
    assert!(LuauContent::load(&p, &plan, Options::default()).is_ok());
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

/// docs/design/content-model-v2.md §4.0: content names an asset as its
/// game's asset pack does (`bomb`); a name the pack hasn't is refused.
#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn assets_are_named_as_their_pack_names_them() {
    let mut test = nettai_content_api::PackIndex::default();
    test.sprites.insert("bomb".into(), nettai_content_api::PackSprite { category: 0x0C, index: 2 });
    let names = AssetNames::of_pack("test", test);
    assert_eq!(names.names(nettai_content_api::AssetKind::Sprite), ["bomb"]);
    let sprite = |source: &str| -> Result<Data, String> {
        let p = Pack::root("test", [("m".to_string(), source.to_string())]);
        define(&p, &names, Options::default())
            .map(|(d, _)| d.of(Registry::Effect)[0].spec.field("sprite").clone())
            .map_err(|e| e.message)
    };
    let effect = |name: &str| format!("return new.effect {{ sprite = asset.sprite('{name}'), anim = 0 }}");
    let asset = |n: &str| Data::Asset(nettai_content_api::AssetKind::Sprite, n.into());
    assert_eq!(sprite(&effect("bomb")), Ok(asset("bomb")));
    let e = sprite(&effect("nope")).unwrap_err();
    assert!(e.contains("no sprite is named \"nope\" in the game's asset pack"), "{e}");
}
