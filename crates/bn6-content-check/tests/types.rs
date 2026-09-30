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
        ("local _ = data.chips[1].gun_del_sol.firing_tick", "not a data field"),
        ("local function f(me: Object) me.drag_step = \"sliding\" end", "not a drag step"),
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
}
