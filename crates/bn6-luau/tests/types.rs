//! The content pack type-checks (Luau strict mode, new solver) against the
//! API definitions in content/bn6/core.d.luau: misspelled fields, wrong
//! argument types and misuse of the core API fail here, before a battle
//! runs. Each module is checked on its own; `require` is typed `any` here
//! (an editor running luau-lsp resolves it and checks across modules too).

use std::path::Path;

#[test]
fn the_content_pack_type_checks_against_the_core_api() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/bn6");
    let defs = std::fs::read_to_string(root.join("core.d.luau")).unwrap();
    let mut checker = luau_analyze::Checker::new().unwrap();
    checker.add_definitions(&defs).expect("core.d.luau loads");
    checker.add_definitions("declare function require(path: string): any").unwrap();
    let pack = bn6_luau::Pack::from_dir(&root).unwrap();
    let mut problems = Vec::new();
    let mut checked = 0;
    for (path, source) in pack.modules() {
        eprintln!("checking {path}");
        let options = luau_analyze::CheckOptions { timeout: Some(std::time::Duration::from_secs(20)), ..Default::default() };
        let result = checker.check_with_options(source, options).unwrap();
        assert!(!result.timed_out, "{path}.luau: the type check timed out");
        checked += 1;
        for d in result.errors() {
            problems.push(format!("{path}.luau:{}:{}: {}", d.line + 1, d.col + 1, d.message));
        }
    }
    assert!(checked >= 7, "found the pack's modules");
    assert!(problems.is_empty(), "type errors:\n{}", problems.join("\n"));
}

#[test]
fn misuse_of_the_core_api_is_a_type_error() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/bn6");
    let defs = std::fs::read_to_string(root.join("core.d.luau")).unwrap();
    let mut checker = luau_analyze::Checker::new().unwrap();
    checker.add_definitions(&defs).unwrap();
    for (bad, why) in [
        ("local function f(me: Object) me.anmi = 3 end", "misspelled field"),
        ("local function f(me: Object) me:set_lifecycle(\"running\") end", "not a lifecycle state"),
        ("local function f(me: Object) me.sprite.shadow = \"soft\" end", "not a shadow"),
        ("local function f(me: Object) local v: Vec3 = me.pos + 1 end", "Vec3 plus a number"),
        ("local o = battle.spawn(\"projectile\", 3)", "not a pool"),
    ] {
        eprintln!("checking {why}");
        let options = luau_analyze::CheckOptions { timeout: Some(std::time::Duration::from_secs(20)), ..Default::default() };
        let result = checker.check_with_options(&format!("--!strict\n{bad}\n"), options).unwrap();
        assert!(!result.timed_out, "{why}: the type check timed out");
        assert!(!result.errors().is_empty(), "{why}: `{bad}` should not type-check");
    }
}
