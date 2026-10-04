//! The define phase and loading, on small packs written here.

use super::*;
use nettai_content_api::Data;

#[test]
fn relative_paths_resolve_within_the_pack() {
    let r = keys::resolve;
    assert_eq!(r("bn6:chips/gundels/chips", "../../objects/sun-beam/sun_beam").unwrap(), "bn6:objects/sun-beam/sun_beam");
    assert_eq!(r("bn6:(pack)", "./lib/slot").unwrap(), "bn6:lib/slot");
    assert!(r("bn6:lib/slot", "../../x").is_err());
    assert!(r("bn6:lib/slot", "objects/x").is_err());
    // A pack's top: its own, or another's (which `packs::check_require` may refuse).
    assert_eq!(r("bn6:chips/x/chip", "@bn6/lib/slot").unwrap(), "bn6:lib/slot");
    assert_eq!(r("bn6:chips/x/chip", "@exelib/swords/slash").unwrap(), "exelib:swords/slash");
    assert!(r("bn5:rules/ruleset", "@bn6/../x").is_err());
}

/// docs/design/content-model-v2.md §4.0: a game requires itself and the
/// support packs it uses; a support pack itself and the support packs it
/// uses; no pack another game's.
#[test]
fn a_require_reaches_only_its_pack_and_the_support_packs_it_uses() {
    use nettai_content_api::{PackKind, PackManifest};
    let manifest = |id: &str, kind: PackKind, uses: &[&str]| PackManifest {
        id: id.into(),
        kind,
        uses: uses.iter().map(|u| u.to_string()).collect(),
        ..Default::default()
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
/// without the playing game's context (`asset`, `system`), which reaches
/// them only as their callers' arguments; a game's modules have it.
#[test]
fn a_support_pack_has_no_game_context() {
    use nettai_content_api::{PackKind, PackManifest};
    let manifest = |id: &str, kind: PackKind, uses: &[&str]| PackManifest {
        id: id.into(),
        kind,
        uses: uses.iter().map(|u| u.to_string()).collect(),
        ..Default::default()
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
    let maker = "return function(look: any) return define.effect { sprite = look.sprite, anim = 0 } end";
    load(&[
        (
            "game:x",
            "local make = require('@lib/m')\nlocal _ = system.state\nreturn make({ sprite = asset.sprite('bomb') })",
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
    let e = load(&[("game:x", "return require('@lib/m')"), ("lib:m", "return system.side")]).unwrap_err();
    refused(&e, "system", "lib:m");
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
        ("lib:m", "return function() return define.effect { sprite = asset.sprite('bomb'), anim = 0 } end"),
    ])
    .unwrap_err();
    refused(&e, "asset", "lib:m");
}

/// `system.state_of` is a game's rules' (its API module's): a module
/// outside rules/ calling it is refused, naming the module (As built S8).
#[test]
fn another_systems_state_is_for_the_games_rules() {
    let call = "local s = system.state_of(0, define.system { id = 'x' })\nreturn {}";
    let e = define_named(&[("chips/x/chip", call)]).unwrap_err();
    assert!(e.contains("test:chips/x/chip: system.state_of is a game's rules'"), "{e}");
    // (Under rules/ it passes the guard: here no battle runs.)
    let e = define_named(&[("rules/api", call)]).unwrap_err();
    assert!(!e.contains("is a game's rules'") && e.contains("only reachable while content runs"), "{e}");
}

fn pack(modules: &[(&str, &str)]) -> Pack {
    Pack::root("test", modules.iter().map(|(p, s)| (p.to_string(), s.to_string())))
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
    return define.action {
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
local EXPLOSION = define.effect { anim = 0 }
bomb.kind = define.kind {
    id = "bomb",
    pool = "attack",
    state = { variant = "u16" },
    update = function(me: any) return EXPLOSION end,
}
function bomb.variant(v: any): any
    return define.record("bomb-variant", v)
end
return bomb
"#,
    ),
    (
        "chips/minibomb/chip",
        r#"--!strict
local throw = require("../../lib/bombs/throw")
local bomb = require("../../lib/bombs/bomb")
return define.chip {
    id = "minibomb",
    name = "MiniBomb",
    codes = { "B", "L", "R", "*" },
    action = throw.action { held = 4, thrower = bomb.variant { palette = 0 } },
}
"#,
    ),
    (
        "chips/flshbom/chips",
        r#"--!strict
local throw = require("../../lib/bombs/throw")
local bomb = require("../../lib/bombs/bomb")
local THROW = throw.action { held = 0x2E, thrower = bomb.variant { palette = 1 } }
return {
    define.chip { id = "flshbom1", name = "FlshBom1", action = THROW },
    define.chip { id = "flshbom2", name = "FlshBom2", action = THROW },
}
"#,
    ),
];

#[test]
fn definitions_get_keys_from_ids_owners_and_modules() {
    let d = define_pack(BOMBS).unwrap();
    let keys = |r: Registry| d.of(r).iter().map(|d| d.key.as_str()).collect::<Vec<_>>();
    // Each qualified with the module's root.
    assert_eq!(keys(Registry::Chip), ["flshbom1", "flshbom2", "minibomb"]);
    assert_eq!(keys(Registry::Kind), ["bomb"]);
    // An action nested in a chip made while the chip's module loaded takes
    // its key; the series' shared action, its first chip's.
    assert_eq!(keys(Registry::Action), ["flshbom1/action", "minibomb/action"]);
    // Nested deeper: the variant inside the action's arguments.
    assert_eq!(keys(Registry::Record), ["flshbom1/action/args/thrower", "minibomb/action/args/thrower"]);
    // Not nested in a keyed definition: its module and place.
    assert_eq!(keys(Registry::Effect), ["lib/bombs/bomb#1"]);
    // The throw's state table is one schema for both actions; the kind's
    // its own.
    assert_eq!(keys(Registry::Schema), ["action:flshbom1/action/state", "kind:bomb/state"]);
    let action = d.get(Registry::Action, "minibomb/action").unwrap();
    assert_eq!(action.spec.field("state"), &Data::Ref(Registry::Schema, "action:flshbom1/action/state".into()));
    assert_eq!(action.spec.field("update"), &Data::Function);
    let chip = d.get(Registry::Chip, "minibomb").unwrap();
    assert_eq!(chip.spec.field("action"), &Data::Ref(Registry::Action, "minibomb/action".into()));
    assert_eq!(chip.module, "test:chips/minibomb/chip");
    let record = d.get(Registry::Record, "minibomb/action/args/thrower").unwrap();
    assert_eq!(record.record_type.as_deref(), Some("bomb-variant"));
}

#[test]
fn the_define_phase_reads_the_same_whatever_the_order() {
    let a = define_pack(BOMBS).unwrap();
    let mut reversed = BOMBS.to_vec();
    reversed.reverse();
    assert_eq!(define_pack(&reversed).unwrap(), a);
    assert_eq!(define_pack(BOMBS).unwrap(), a);
}

#[test]
fn definition_mistakes_are_load_errors() {
    let cases: &[(&str, &str)] = &[
        ("return define.chip { name = 'x' }", "needs an `id`"),
        ("return define.kind { id = 'Bomb', pool = 'attack' }", "not a valid id"),
        ("return define.chip { id = 'x', damage = 1.5 }", "1.5 is not an integer"),
        ("local t = {}\nt.me = t\nreturn define.chip { id = 'x', loop = t }", "contains itself"),
        ("return define.chip(3)", "takes a table"),
        ("local s = define.chip { id = 'x' }\nreturn define.chip(s)", "defined twice"),
        // The migration's markers are gone: neither the global nor the
        // field.
        ("return define.navi { id = 'x', legacy = { number = 1 } }", "takes no `legacy` field"),
        ("return define.navi { id = 'x', marker = legacy { number = 1 } }", "attempt to call a nil value"),
    ];
    for (source, want) in cases {
        let e = define_pack(&[("chips/x/chip", source)]).unwrap_err();
        assert!(e.contains(want), "{source}: {e}");
    }
    let e = define_pack(&[("a", "return define.chip { id = 'x' }"), ("b", "return define.chip { id = 'x' }")]).unwrap_err();
    assert!(e.contains("chip \"x\" is defined twice: in test:a.luau and in test:b.luau"), "{e}");
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

#[test]
fn assets_resolve_by_name_while_content_loads() {
    let d = define_named(&[(
        "lib/effects",
        "local SOUND = asset.sound('throw')\n\
         if asset.sprite('bomb') ~= asset.sprite('bomb') then error('one value per asset') end\n\
         return { explosion = define.effect { sprite = asset.sprite('explosion'), anim = 3, sound = SOUND } }",
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

/// A game's rules are one definition, its stock ruleset (rules/init.luau):
/// its rule sections and its roles are plain tables in it, whose
/// definitions are references (docs/design/content-model-v2.md §3.8).
#[test]
fn a_games_rules_are_one_ruleset() {
    let d = define_named(&[
        ("lib/counter", "return define.action { id = 'counter', state = {}, update = function(me, s) end }"),
        ("rules/roles", "return { actions = { anti_damage_counter = require('../lib/counter') } }"),
        ("rules/pools", "return { actor = 32, attack = 32, effect = 32 }"),
        (
            "rules/init",
            "return define.ruleset { id = 'stock', stock = true, systems = {}, pools = require('./pools'), roles = require('./roles') }",
        ),
    ])
    .unwrap();
    let stock = d.get(Registry::Ruleset, "stock").expect("the stock ruleset");
    assert_eq!(stock.spec.field("roles").field("actions").field("anti_damage_counter"), &Data::Ref(Registry::Action, "counter".into()));
    assert_eq!(stock.spec.field("pools").field("actor"), &Data::Int(32));
    // No definer makes a section or the roles apart from it.
    for definer in ["rules('pools', {})", "roles {}"] {
        let source = format!("return define.{definer}");
        let e = define_named(&[("m", source.as_str())]).unwrap_err();
        assert!(e.contains("attempt to call a nil value"), "define.{definer}: {e}");
    }
}

#[test]
fn definitions_are_frozen_and_definers_close_after_loading() {
    let p = pack(&[(
        "m",
        "local d = define.record('r', { n = 1 })\nreturn { d = d, late = function() return define.record('r', {}) end }",
    )]);
    let (lua, defined, modules, _, _) = open(&p, &AssetNames::default(), Options::default()).unwrap();
    assert!(defined.tables.iter().all(|t| t.is_readonly()));
    let LuaValue::Table(m) = &modules["test:m"] else { panic!("a table") }; // (written in full)
    let late: Function = m.get("late").unwrap();
    BUDGET.with(|b| b.set(1000));
    let e = late.call::<LuaValue>(()).unwrap_err().to_string();
    assert!(e.contains("definitions are made while content loads"), "{e}");
    drop(lua);
}

/// Coverage (src/coverage.rs): the modules whose code ran while recording,
/// on this thread; nothing when not recording.
#[test]
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

#[test]
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
#[test]
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
    let effect = |name: &str| format!("return define.effect {{ sprite = asset.sprite('{name}'), anim = 0 }}");
    let asset = |n: &str| Data::Asset(nettai_content_api::AssetKind::Sprite, n.into());
    assert_eq!(sprite(&effect("bomb")), Ok(asset("bomb")));
    let e = sprite(&effect("nope")).unwrap_err();
    assert!(e.contains("no sprite is named \"nope\" in the game's asset pack"), "{e}");
}
