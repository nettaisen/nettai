//! `bn6-content check`'s report on definitions, on the engine's test pack
//! and on BN6's content.

use bn6_battle::content::testing;
use bn6_content::report::{Level, Report};

#[test]
fn unfilled_roles_and_single_owner_kinds_are_reported() {
    let mut c = testing::with_test_pack();
    // A kind under objects/ that only one chip folder uses.
    c.scripts.modules.insert(
        "objects/held/held".into(),
        "return { kind = define.kind { id = 'held', pool = 'effect', update = function(me) end } }".into(),
    );
    c.scripts.modules.insert(
        "chips/holder/chip".into(),
        "local held = require('../../objects/held/held')\n\
         return define.record('holder', { kind = held.kind })"
            .into(),
    );
    c.define().unwrap();
    let mut r = Report::default();
    bn6_content::lint::definitions(&c, &mut r);
    let warnings: Vec<String> =
        r.issues.iter().filter(|i| i.level == Level::Warning).map(|i| format!("{}: {}", i.file, i.message)).collect();
    assert!(warnings.iter().any(|w| w.contains("actions.body_guard_counter is not filled")), "{warnings:?}");
    assert!(!warnings.iter().any(|w| w.contains("anti_damage_counter")), "the test pack fills it: {warnings:?}");
    assert!(warnings.iter().any(|w| w.starts_with("objects/held/held.luau: only chips/holder uses")), "{warnings:?}");
}

#[test]
fn a_collision_type_defined_twice_is_an_error() {
    let mut c = testing::with_test_pack();
    for (module, key) in [("lib/one", "one"), ("lib/two", "two"), ("lib/other", "other")] {
        // (Rows no type of the test content is.)
        let row_offset = if key == "other" { 0x7F8 } else { 0x7F0 };
        c.scripts.modules.insert(
            module.into(),
            format!("return define.collision {{ id = '{key}', side0 = 0x80, side1 = 0x80, row_offset = {row_offset} }}"),
        );
    }
    c.define().unwrap();
    assert_eq!(
        bn6_content::lint::duplicate_collision_types(&c),
        [(0xFE, vec![("one".to_string(), "lib/one".to_string()), ("two".to_string(), "lib/two".to_string())])]
    );
    let mut r = Report::default();
    bn6_content::lint::definitions(&c, &mut r);
    let errors: Vec<&str> = r.issues.iter().filter(|i| i.level == Level::Error).map(|i| i.message.as_str()).collect();
    assert!(matches!(errors[..], [e] if e.contains("collision type one is row 0xfe") && e.contains("two (lib/two.luau)")), "{errors:?}");
}

/// BN6's content defines without an error in its definitions (each
/// collision type once).
#[test]
fn bn6_content_has_no_definition_errors() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");
    let mut c = bn6_battle::Content::default();
    c.scripts.modules = testing::modules_under(dir);
    c.assets = testing::asset_names_used(&c.scripts.modules);
    c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
    let mut r = Report::default();
    bn6_content::lint::definitions(&c, &mut r);
    let errors: Vec<String> =
        r.issues.iter().filter(|i| i.level == Level::Error).map(|i| format!("{}: {}", i.file, i.message)).collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
