//! The content in content/bn6 (the BN6 source overlay) type-checks against
//! its API definitions, and misuse of the API is a type error.

use std::path::Path;

fn pack() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/bn6")
}

#[test]
fn the_content_pack_type_checks_against_the_core_api() {
    let (checked, problems) = bn6_content_check::check_pack(&pack()).unwrap();
    assert!(checked >= 16, "found the pack's modules ({checked})");
    assert!(problems.is_empty(), "type errors:\n{}", problems.join("\n"));
}

/// The engine's test pack (crates/bn6-battle/testdata/pack) uses the v2
/// API throughout: it type-checks against the same definitions.
#[test]
fn the_v2_test_pack_type_checks() {
    let mut checker = bn6_content_check::PackChecker::new(&bn6_content_check::definitions(&pack()).unwrap()).unwrap();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bn6-battle/testdata/pack");
    let modules = bn6_content_check::modules(&dir).unwrap();
    assert!(modules.len() >= 5, "{} modules", modules.len());
    let mut problems = Vec::new();
    for (path, source) in &modules {
        problems.extend(checker.check(path, source).unwrap());
    }
    assert!(problems.is_empty(), "type errors:\n{}", problems.join("\n"));
}

#[test]
fn misuse_of_the_v2_api_is_a_type_error() {
    let mut checker = bn6_content_check::PackChecker::new(&bn6_content_check::definitions(&pack()).unwrap()).unwrap();
    for (bad, why) in [
        ("local _ = define.kind { id = 'x', pool = 'water', update = function(me: Object) end }", "not a pool"),
        ("local _ = define.kind { id = 'x', pool = 'attack' }", "a kind without its update"),
        ("local _ = asset.sprite(3)", "an asset by number"),
        ("local _ = define.effect { sprite = 'bomb' }", "a sprite by string"),
        ("local function f(me: Object) me:set_attack('shot', 1) end", "an action by name"),
        ("local function f(me: Object) local _ = battle.spawn(me, me.pos) end", "an object for a kind"),
        ("local _ = define.region { panels = { 'front' } }", "a panel that isn't { dx, dy }"),
        ("local _ = define.roles { actions = { anti_damage_counter = 3 } }", "a role that isn't an action"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
}

#[test]
fn the_numeric_api_that_is_gone_is_a_type_error() {
    let mut checker = bn6_content_check::PackChecker::new(&bn6_content_check::definitions(&pack()).unwrap()).unwrap();
    for (bad, why) in [
        ("local _ = battle.spawn(\"attack\", 8)", "a spawn by pool and index"),
        ("local _ = battle.spawn_kind(\"rock\")", "a spawn by name"),
        ("local function f(me: Object) local _ = me:param(1) end", "a spawn parameter"),
        ("local function f(me: Object) local _ = me:attack_param(1) end", "an attack parameter"),
        ("local function f(me: Object) local _ = me.index end", "a kind by index"),
        ("local function f(me: Object) local _ = me.variant end", "the attack's variant"),
        ("local function f(me: Object) local _: number = me.chip end", "the attack's chip by number"),
        ("local function f(me: Object) me:set_attack(0x12, 2) end", "an action by number"),
        ("local function f(me: Object) me.sprite:load(\"0c-01\") end", "a sprite by number"),
        ("local _ = battle.hand_chip(0, 0)", "a hand's chip by number"),
        ("local function f(me: Object) dimming.show_navi_telop(me, 0x123) end", "a telop's chip by number"),
        ("battle.set_linked(0, { chip = 0x123, bonus = 0, damage = 0 })", "a linked record's chip by number"),
        ("local _ = define.roles { actions = { turn = { legacy = { action = 0x3B } } } }", "a role by action number"),
        ("local _ = define.roles { kinds = { support = { legacy = { kind = \"support\" } } } }", "a role by kind key"),
        ("local _ = data.rules.sine[1]", "a rule table by number"),
        ("local function f(me: Object) local _ = battle.effect(me.pos, 3) end", "an effect by number"),
        ("local function f(me: Object) local _ = me.name_id end", "an object's NameID"),
        ("local _ = battle.navi_record(0x1A0)", "an actor record by NameID"),
        ("local function f(me: Object) me:death_hook(0x1A0) end", "a death hook by NameID"),
        ("local _ = battle.attach_point(0x1A0, 3, 0, 0)", "an attach point by NameID"),
        ("local function f(me: Object) me:setup_collision(4, 5, 0) end", "collision types by number"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
}

#[test]
fn misuse_of_the_core_api_is_a_type_error() {
    let mut checker = bn6_content_check::PackChecker::new(&bn6_content_check::definitions(&pack()).unwrap()).unwrap();
    for (bad, why) in [
        ("local function f(me: Object) me.anmi = 3 end", "misspelled field"),
        ("local function f(me: Object) me:set_lifecycle(\"running\") end", "not a lifecycle state"),
        ("local function f(me: Object) me.sprite.shadow = \"soft\" end", "not a shadow"),
        ("local function f(me: Object) local _ = me.pos + 1 end", "Vec3 plus a number"),
        ("local _ = battle.spawn(\"projectile\", 3)", "not a pool"),
        ("local function f(me: Object) me:set_status(\"usingaction\", true) end", "not a status flag"),
        ("local function f(me: Object) local _ = me:held(\"x\") end", "not a button"),
        ("field.set_type(1, 1, \"lava\")", "not a panel type"),
        ("local function f(me: Object) me:set_status_timer(\"stun\", 3) end", "not a status timer"),
        ("local _ = data.chips[1]", "not a data field"),
        ("local function f(me: Object) me.drag_step = \"sliding\" end", "not a drag step"),
        (
            "local function f(me: Object) battle.afterimage(me, me.pos, { anim = 0, flip = 0, color_shader = 0, lifetime = 1, shadow = \"soft\" }) end",
            "an afterimage's shadow that isn't one",
        ),
        ("local function f(me: Object) local _ = obstacle.react(me, \"shatters\") end", "not an obstacle crush"),
        ("local function f(me: Object) local _: \"gone\" = obstacle.removal(me) end", "not an obstacle removal"),
        ("local _: ProjectileShot = { kind = 0, damage = 1 }", "a shot without its height"),
        ("local function f(): SideSpecial return \"beast\" end", "not a side special"),
        (
            "local function f(me: Object) battle.form_overlay(me, { sprite = asset.sprite(\"x\"), stepping = \"dimmed\" }) end",
            "not a form overlay stepping",
        ),
        // Subtypes 8, 17, 18 (Wind, Anubis, Otenko) and the obstacle framework.
        ("local function f(me: Object) obstacle.take_hits(me, \"shoved\") end", "not an obstacle push"),
        ("local function f(me: Object) local _ = obstacle.react(me, \"breaks\", \"never\") end", "not a dimming hold"),
        ("local function f(me: Object) battle.set_wind(me, 0, \"chip\") end", "not a wind source"),
        // Dimming chip subtypes 2, 3, 5, 15 and 27.
        ("field.blink(1, 1, \"lava\", 0)", "a blink to not a panel type"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
}
