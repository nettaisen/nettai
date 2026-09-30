//! The content pack in content/bn6 type-checks against its API
//! definitions, and misuse of the API is a type error.

use std::path::Path;

fn pack() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/bn6")
}

#[test]
fn the_content_pack_type_checks_against_the_core_api() {
    let (checked, problems) = bn6_content_check::check_pack(&pack()).unwrap();
    assert!(checked >= 7, "found the pack's modules ({checked})");
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
    ] {
        let problems = checker.check(why, &format!("--!strict\n{bad}\n")).unwrap();
        assert!(!problems.is_empty(), "{why}: `{bad}` should not type-check");
    }
}
