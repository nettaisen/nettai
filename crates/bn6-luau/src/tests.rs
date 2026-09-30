//! The define phase and loading, on small packs written here.

use super::*;

#[test]
fn relative_paths_resolve_within_the_pack() {
    assert_eq!(resolve("chips/00f-gundels1/chip", "../../objects/sun-beam/sun_beam").unwrap(), "objects/sun-beam/sun_beam");
    assert_eq!(resolve("(pack)", "./lib/slot").unwrap(), "lib/slot");
    assert!(resolve("lib/slot", "../../x").is_err());
    assert!(resolve("lib/slot", "objects/x").is_err());
}

fn pack(modules: &[(&str, &str)]) -> Pack {
    Pack::new(modules.iter().map(|(p, s)| (p.to_string(), s.to_string())))
}

fn define_pack(modules: &[(&str, &str)]) -> Result<Definitions, String> {
    define(&pack(modules), &Data::Nil, &AssetNames::default(), Options::default()).map(|(d, _)| d).map_err(|e| e.message)
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
    assert_eq!(chip.module, "chips/minibomb/chip");
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
    ];
    for (source, want) in cases {
        let e = define_pack(&[("chips/x/chip", source)]).unwrap_err();
        assert!(e.contains(want), "{source}: {e}");
    }
    let e = define_pack(&[("a", "return define.chip { id = 'x' }"), ("b", "return define.chip { id = 'x' }")]).unwrap_err();
    assert!(e.contains("chip \"x\" is defined twice: in a.luau and in b.luau"), "{e}");
}

/// Asset names for these tests.
fn names() -> AssetNames {
    let mut a = AssetNames::default();
    a.sprites.insert("bomb".into(), bn6_content_api::SpriteId { category: 0x0C, index: 2 });
    a.sprites.insert("explosion".into(), bn6_content_api::SpriteId { category: 0x0C, index: 1 });
    a.sounds.insert("throw".into(), 0x1A6);
    a
}

fn define_named(modules: &[(&str, &str)]) -> Result<Definitions, String> {
    define(&pack(modules), &Data::Nil, &names(), Options::default()).map(|(d, _)| d).map_err(|e| e.message)
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
    assert_eq!(e.spec.field("sprite"), &Data::Asset(bn6_content_api::AssetKind::Sprite, "explosion".into()));
    assert_eq!(e.spec.field("sound"), &Data::Asset(bn6_content_api::AssetKind::Sound, "throw".into()));
    // An unknown name is an error naming the module; so is a resolver
    // called after loading.
    let e = define_named(&[("chips/x/chip", "return { s = asset.sprite('bom') }")]).unwrap_err();
    assert!(e.contains("chips/x/chip: no sprite is named \"bom\""), "{e}");
    let e = define_named(&[("m", "return { f = function() return asset.sound('throw') end }")]);
    assert!(e.is_ok(), "calling it later is the runtime's error, not the define phase's");
}

#[test]
fn the_roles_are_one_definition() {
    let d = define_named(&[
        ("lib/counter", "return define.action { id = 'counter', state = {}, update = function(me, s) end }"),
        ("rules/roles", "return define.roles { actions = { anti_damage_counter = require('../lib/counter') } }"),
    ])
    .unwrap();
    let roles = d.get(Registry::Roles, "roles").expect("keyed roles");
    assert_eq!(roles.spec.field("actions").field("anti_damage_counter"), &Data::Ref(Registry::Action, "counter".into()));
    let e = define_named(&[("a", "return define.roles {}"), ("b", "return define.roles {}")]).unwrap_err();
    assert!(e.contains("roles \"roles\" is defined twice"), "{e}");
}

#[test]
fn definitions_are_frozen_and_definers_close_after_loading() {
    let p = pack(&[(
        "m",
        "local d = define.record('r', { n = 1 })\nreturn { d = d, late = function() return define.record('r', {}) end }",
    )]);
    let (lua, defined, modules, _, _) = open(&p, &Data::Nil, &AssetNames::default(), Options::default()).unwrap();
    assert!(defined.tables.iter().all(|t| t.is_readonly()));
    let LuaValue::Table(m) = &modules["m"] else { panic!("a table") };
    let late: Function = m.get("late").unwrap();
    BUDGET.with(|b| b.set(1000));
    let e = late.call::<LuaValue>(()).unwrap_err().to_string();
    assert!(e.contains("definitions are made while content loads"), "{e}");
    drop(lua);
}

#[test]
fn a_plan_binds_definition_slots_and_module_exports() {
    let mut modules = BOMBS.to_vec();
    modules.push(("objects/old/old", "return { state = { t = 'u8' }, update = function(me) end }"));
    let p = pack(&modules);
    let (definitions, compiled) = define(&p, &Data::Nil, &AssetNames::default(), Options::default()).unwrap();
    assert_eq!(compiled.len(), modules.len(), "every module compiled");
    let p = p.with_compiled(compiled);
    let handles = (0..definitions.defs.len() as u16).collect();
    let plan = BindPlan {
        functions: vec![
            FnSource::slot(Registry::Kind, "bomb", "update"),
            FnSource::slot(Registry::Action, "minibomb/action", "update"),
            FnSource::export("objects/old/old", "update"),
        ],
        schemas: Vec::new(),
        definitions,
        handles,
        entries: Vec::new(),
        assets: AssetNames::default(),
    };
    assert!(LuauContent::load(&p, &plan, &Data::Nil, Options::default()).is_ok());
    // A plan made from other definitions is refused.
    let mut stale = plan.clone();
    stale.definitions.defs.pop();
    stale.handles.pop();
    let e = LuauContent::load(&p, &stale, &Data::Nil, Options::default()).err().expect("refused").message;
    assert!(e.contains("define something other"), "{e}");
    // A slot that isn't a function is refused.
    let mut wrong = plan.clone();
    wrong.functions.push(FnSource::slot(Registry::Chip, "minibomb", "name"));
    let e = LuauContent::load(&p, &wrong, &Data::Nil, Options::default()).err().expect("refused").message;
    assert!(e.contains("chip minibomb: `name` is string, not a function"), "{e}");
}
