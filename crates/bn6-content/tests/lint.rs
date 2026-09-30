//! `bn6-content check`'s report on definitions, on the engine's test pack.

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
