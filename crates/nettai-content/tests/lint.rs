//! `nettai-content check`'s report on definitions, on the engine's test pack
//! and on BN6's content.

use nettai_battle::content::{RootManifest, Scripts, testing};
use nettai_content::report::{Level, Report};

/// The test content's module `path`'s name.
fn module(path: &str) -> String {
    Scripts::name(testing::ROOT, path)
}

#[test]
fn unfilled_roles_and_single_owner_kinds_are_reported() {
    let mut c = testing::with_test_pack();
    // A kind under objects/ that only one chip folder uses.
    c.scripts.modules.insert(
        module("objects/held/held"),
        "return { kind = define.kind { id = 'held', pool = 'effect', update = function(me) end } }".into(),
    );
    c.scripts.modules.insert(
        module("chips/holder/chip"),
        "local held = require('../../objects/held/held')\n\
         return define.record('holder', { kind = held.kind })"
            .into(),
    );
    // A role that names a definition, left out.
    let roles = c.scripts.home_module_mut("test/rules/roles").expect("the test pack's roles");
    assert!(roles.contains("    sparks = ruleset.sparks,\n"));
    *roles = roles.replace("    sparks = ruleset.sparks,\n", "    sparks = { plain = ruleset.sparks.plain },\n");
    c.define().unwrap();
    let mut r = Report::default();
    nettai_content::lint::definitions(&c, &mut r);
    let warnings: Vec<String> =
        r.issues.iter().filter(|i| i.level == Level::Warning).map(|i| format!("{}: {}", i.file, i.message)).collect();
    assert!(warnings.iter().any(|w| w.contains("actions.body_guard_counter is not filled")), "{warnings:?}");
    assert!(!warnings.iter().any(|w| w.contains("anti_damage_counter")), "the test pack fills it: {warnings:?}");
    assert!(warnings.iter().any(|w| w.contains("sparks.guard is not filled")), "{warnings:?}");
    assert!(!warnings.iter().any(|w| w.contains("sparks.plain") || w.contains("effects.") || w.contains("collision.")), "{warnings:?}");
    assert!(warnings.iter().any(|w| w.starts_with("test:objects/held/held.luau: only test:chips/holder uses")), "{warnings:?}");
}

#[test]
fn a_collision_type_defined_twice_is_an_error() {
    let mut c = testing::with_test_pack();
    for (path, key) in [("lib/one", "one"), ("lib/two", "two"), ("lib/other", "other")] {
        // (Rows no type of the test content is.)
        let row_offset = if key == "other" { 0x7F8 } else { 0x7F0 };
        c.scripts.modules.insert(
            module(path),
            format!("return define.collision {{ id = '{key}', side0 = 0x80, side1 = 0x80, row_offset = {row_offset} }}"),
        );
    }
    c.define().unwrap();
    // (The test content's own types share rows with BN6's, whose module
    // it has too: only these are looked at.)
    let twins: Vec<_> = nettai_content::lint::duplicate_collision_types(&c).into_iter().filter(|(row, _)| *row >= 0xFE).collect();
    assert_eq!(twins, [(0xFE, vec![("test:one".to_string(), "test:lib/one".to_string()), ("test:two".to_string(), "test:lib/two".to_string())])]);
    let mut r = Report::default();
    nettai_content::lint::definitions(&c, &mut r);
    let errors: Vec<&str> = r.issues.iter().filter(|i| i.level == Level::Error).map(|i| i.message.as_str()).collect();
    assert!(errors.iter().any(|e| e.contains("collision type test:one is row 0xfe") && e.contains("test:two (test:lib/two.luau)")), "{errors:?}");
}

/// BN6's content defines without an error in its definitions (each
/// collision type once).
#[test]
fn bn6_content_has_no_definition_errors() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");
    let mut c = nettai_battle::Content::default();
    c.scripts = Scripts::root(RootManifest::named("bn6"), testing::modules_under(dir));
    c.assets = testing::asset_names_used(&c.scripts.modules);
    c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
    let mut r = Report::default();
    nettai_content::lint::definitions(&c, &mut r);
    let errors: Vec<String> =
        r.issues.iter().filter(|i| i.level == Level::Error).map(|i| format!("{}: {}", i.file, i.message)).collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

/// BN6's strings tables (content/bn6/locales) name only definitions BN6's
/// content has, and every chip, navi and form has its name in the own
/// language's. The root reader leaves them out of the modules; the own
/// language's strings shape the records (a description's lines), and the
/// hash covers that shape alone: text that keeps it changes nothing.
#[test]
fn bn6_strings_name_bn6_definitions_and_only_their_shape_is_hashed() {
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6"));
    let langs = nettai_content::locale::languages(dir);
    assert!(langs.contains(&"en".to_string()) && langs.contains(&"ja".to_string()), "{langs:?}");
    let mut r = Report::default();
    let root = nettai_content::root::read(dir, &mut r).expect("content/bn6 reads");
    assert!(root.modules.keys().all(|m| !m.starts_with("locales/")), "a strings table read as a module");
    let define = |strings: nettai_content::locale::Strings| {
        let mut c = nettai_battle::Content::default();
        c.scripts = Scripts::root(root.manifest.clone(), root.modules.clone());
        c.assets = testing::asset_names_used(&c.scripts.modules);
        c.strings = strings.qualified("bn6");
        c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
        c
    };
    let c = define(root.strings.clone());
    nettai_content::locale::check_root(dir, &c, &mut r);
    let errors: Vec<String> = r.issues.iter().filter(|i| i.level == Level::Error).map(|i| format!("{}: {}", i.file, i.message)).collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    // The own strings' shape is in the records: MagPanel's one line.
    let magpanl = c.defs.chip_by_key("magpanl").expect("magpanl");
    assert_eq!(c.chip(magpanl).description_lines, 1);
    // Other text of the same shape: the same content.
    let mut renamed = root.strings.clone();
    for s in renamed.chips.values_mut() {
        s.name = Some("Renamed".into());
    }
    assert_eq!(define(renamed).hash(), c.hash());
    // Another shape: another content.
    let mut reshaped = root.strings.clone();
    reshaped.chips.get_mut("magpanl").unwrap().description = Some("one\ntwo".into());
    assert_ne!(define(reshaped).hash(), c.hash());
}
