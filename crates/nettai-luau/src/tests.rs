//! The define phase and loading, on small packs written here.

use super::*;
use nettai_content_api::Data;

#[test]
fn relative_paths_resolve_within_the_folder() {
    let r = resolve;
    assert_eq!(r("bn6:chips/gundels/chips", "../../objects/sun-beam/sun_beam").unwrap(), "bn6:objects/sun-beam/sun_beam");
    assert_eq!(r("bn6:(pack)", "./lib/slot").unwrap(), "bn6:lib/slot");
    assert!(r("bn6:lib/slot", "../../x").is_err());
    assert!(r("bn6:lib/slot", "objects/x").is_err());
    // A folder's top: its own, or any other (one namespace).
    assert_eq!(r("bn6:chips/x/chip", "@bn6/lib/slot").unwrap(), "bn6:lib/slot");
    assert_eq!(r("bn5:rules/ruleset", "@bn6/rules/beast/system").unwrap(), "bn6:rules/beast/system");
    assert_eq!(r("bn6:rules/ruleset", "@bn5/rules/ruleset").unwrap(), "bn5:rules/ruleset");
    assert!(r("bn5:rules/ruleset", "@bn6/../x").is_err());
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
    id = "test:bomb",
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
    id = "test:minibomb",
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
    define.chip { id = "test:flshbom1", name = "FlshBom1", action = THROW },
    define.chip { id = "test:flshbom2", name = "FlshBom2", action = THROW },
}
"#,
    ),
];

#[test]
fn definitions_get_keys_from_ids_owners_and_modules() {
    let d = define_pack(BOMBS).unwrap();
    let keys = |r: Registry| d.of(r).iter().map(|d| d.key.as_str()).collect::<Vec<_>>();
    // Each qualified with the module's root.
    assert_eq!(keys(Registry::Chip), ["test:flshbom1", "test:flshbom2", "test:minibomb"]);
    assert_eq!(keys(Registry::Kind), ["test:bomb"]);
    // An action nested in a chip made while the chip's module loaded takes
    // its key; the series' shared action, its first chip's.
    assert_eq!(keys(Registry::Action), ["test:flshbom1/action", "test:minibomb/action"]);
    // Nested deeper: the variant inside the action's arguments.
    assert_eq!(keys(Registry::Record), ["test:flshbom1/action/args/thrower", "test:minibomb/action/args/thrower"]);
    // Not nested in a keyed definition: its module and place.
    assert_eq!(keys(Registry::Effect), ["test:lib/bombs/bomb#1"]);
    // The throw's state table is one schema for both actions; the kind's
    // its own.
    assert_eq!(keys(Registry::Schema), ["action:test:flshbom1/action/state", "kind:test:bomb/state"]);
    let action = d.get(Registry::Action, "test:minibomb/action").unwrap();
    assert_eq!(action.spec.field("state"), &Data::Ref(Registry::Schema, "action:test:flshbom1/action/state".into()));
    assert_eq!(action.spec.field("update"), &Data::Function);
    let chip = d.get(Registry::Chip, "test:minibomb").unwrap();
    assert_eq!(chip.spec.field("action"), &Data::Ref(Registry::Action, "test:minibomb/action".into()));
    assert_eq!(chip.module, "test:chips/minibomb/chip");
    let record = d.get(Registry::Record, "test:minibomb/action/args/thrower").unwrap();
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
        ("return define.kind { id = 'test:Bomb', pool = 'attack' }", "not a valid id"),
        ("return define.chip { id = 'test:x', damage = 1.5 }", "1.5 is not an integer"),
        ("local t = {}\nt.me = t\nreturn define.chip { id = 'test:x', loop = t }", "contains itself"),
        ("return define.chip(3)", "takes a table"),
        ("local s = define.chip { id = 'test:x' }\nreturn define.chip(s)", "defined twice"),
        // The migration's markers are gone: neither the global nor the
        // field.
        ("return define.navi { id = 'test:x', legacy = { number = 1 } }", "takes no `legacy` field"),
        ("return define.navi { id = 'test:x', marker = legacy { number = 1 } }", "attempt to call a nil value"),
    ];
    for (source, want) in cases {
        let e = define_pack(&[("chips/x/chip", source)]).unwrap_err();
        assert!(e.contains(want), "{source}: {e}");
    }
    let e = define_pack(&[("a", "return define.chip { id = 'test:x' }"), ("b", "return define.chip { id = 'test:x' }")]).unwrap_err();
    assert!(e.contains("chip \"test:x\" is defined twice: in test:a.luau and in test:b.luau"), "{e}");
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
        "local SOUND = asset.sound('test:throw')\n\
         if asset.sprite('test:bomb') ~= asset.sprite('test:bomb') then error('one value per asset') end\n\
         return { explosion = define.effect { sprite = asset.sprite('test:explosion'), anim = 3, sound = SOUND } }",
    )])
    .unwrap();
    let e = &d.of(Registry::Effect)[0];
    assert_eq!(e.spec.field("sprite"), &Data::Asset(nettai_content_api::AssetKind::Sprite, "test:explosion".into()));
    assert_eq!(e.spec.field("sound"), &Data::Asset(nettai_content_api::AssetKind::Sound, "test:throw".into()));
    // An unknown name is an error naming the module; so is a resolver
    // called after loading.
    let e = define_named(&[("chips/x/chip", "return { s = asset.sprite('test:bom') }")]).unwrap_err();
    assert!(e.contains("chips/x/chip: no sprite is named \"test:bom\""), "{e}");
    let e = define_named(&[("m", "return { f = function() return asset.sound('test:throw') end }")]);
    assert!(e.is_ok(), "calling it later is the runtime's error, not the define phase's");
}

#[test]
fn the_roles_are_one_definition() {
    let d = define_named(&[
        ("lib/counter", "return define.action { id = 'test:counter', state = {}, update = function(me, s) end }"),
        ("rules/roles", "return define.roles { id = 'test:roles', actions = { anti_damage_counter = require('../lib/counter') } }"),
    ])
    .unwrap();
    let roles = d.get(Registry::Roles, "test:roles").expect("keyed roles");
    assert_eq!(roles.spec.field("actions").field("anti_damage_counter"), &Data::Ref(Registry::Action, "test:counter".into()));
    let e = define_named(&[("a", "return define.roles { id = 'test:roles' }"), ("b", "return define.roles { id = 'test:roles' }")]).unwrap_err();
    assert!(e.contains("roles \"test:roles\" is defined twice"), "{e}");
}

#[test]
fn definitions_are_frozen_and_definers_close_after_loading() {
    let p = pack(&[(
        "m",
        "local d = define.record('r', { n = 1 })\nreturn { d = d, late = function() return define.record('r', {}) end }",
    )]);
    let (lua, defined, modules, _, _) = open(&p, &AssetNames::default(), Options::default()).unwrap();
    assert!(defined.tables.iter().all(|t| t.is_readonly()));
    let LuaValue::Table(m) = &modules["test:m"] else { panic!("a table") };
    let late: Function = m.get("late").unwrap();
    BUDGET.with(|b| b.set(1000));
    let e = late.call::<LuaValue>(()).unwrap_err().to_string();
    assert!(e.contains("definitions are made while content loads"), "{e}");
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
            FnSource::slot(Registry::Kind, "test:bomb", "update"),
            FnSource::slot(Registry::Action, "test:minibomb/action", "update"),
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
    wrong.functions.push(FnSource::slot(Registry::Chip, "test:minibomb", "name"));
    let e = LuauContent::load(&p, &wrong, Options::default()).err().expect("refused").message;
    assert!(e.contains("chip test:minibomb: `name` is string, not a function"), "{e}");
}

/// docs/design/rules-in-luau.md, the flat namespace: assets of several
/// packs load together, each name its pack's game first; content writes
/// every asset name in full, any loaded pack's, and an unqualified one is
/// refused.
#[test]
fn assets_are_named_in_full() {
    let mut other = nettai_content_api::PackIndex::default();
    other.sprites.insert("bomb".into(), nettai_content_api::PackSprite { category: 0x01, index: 2 });
    let mut test = nettai_content_api::PackIndex::default();
    test.sprites.insert("bomb".into(), nettai_content_api::PackSprite { category: 0x0C, index: 2 });
    let names = AssetNames::of_packs(vec![("test".into(), test), ("other".into(), other)]);
    assert_eq!(names.packs, ["other", "test"], "packs in their games' order");
    assert_eq!(names.names(nettai_content_api::AssetKind::Sprite), ["other:bomb", "test:bomb"]);
    let sprite = |source: &str| -> Result<Data, String> {
        let p = Pack::root("test", [("m".to_string(), source.to_string())]);
        define(&p, &names, Options::default())
            .map(|(d, _)| d.of(Registry::Effect)[0].spec.field("sprite").clone())
            .map_err(|e| e.message)
    };
    let effect = |name: &str| format!("return define.effect {{ sprite = asset.sprite('{name}'), anim = 0 }}");
    let asset = |n: &str| Data::Asset(nettai_content_api::AssetKind::Sprite, n.into());
    assert_eq!(sprite(&effect("test:bomb")), Ok(asset("test:bomb")));
    assert_eq!(sprite(&effect("other:bomb")), Ok(asset("other:bomb")), "any loaded pack's");
    let e = sprite(&effect("bomb")).unwrap_err();
    assert!(e.contains("write an asset's name in full"), "{e}");
}
