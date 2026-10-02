//! `nettai-content check`'s report on definitions, on the engine's test pack
//! and on BN6's content.

use nettai_battle::content::testing;
use nettai_content::report::{Level, Report};

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
    // A role that names a definition, left out.
    let roles = c.scripts.modules.get_mut("test/rules/roles").expect("the test pack's roles");
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
    // (The test content's own types share rows with BN6's, whose module
    // it has too: only these are looked at.)
    let twins: Vec<_> = nettai_content::lint::duplicate_collision_types(&c).into_iter().filter(|(row, _)| *row >= 0xFE).collect();
    assert_eq!(twins, [(0xFE, vec![("one".to_string(), "lib/one".to_string()), ("two".to_string(), "lib/two".to_string())])]);
    let mut r = Report::default();
    nettai_content::lint::definitions(&c, &mut r);
    let errors: Vec<&str> = r.issues.iter().filter(|i| i.level == Level::Error).map(|i| i.message.as_str()).collect();
    assert!(errors.iter().any(|e| e.contains("collision type one is row 0xfe") && e.contains("two (lib/two.luau)")), "{errors:?}");
}

/// BN6's content defines without an error in its definitions (each
/// collision type once).
#[test]
fn bn6_content_has_no_definition_errors() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");
    let mut c = nettai_battle::Content::default();
    c.scripts.modules = testing::modules_under(dir);
    c.assets = testing::asset_names_used(&c.scripts.modules);
    c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
    let mut r = Report::default();
    nettai_content::lint::definitions(&c, &mut r);
    let errors: Vec<String> =
        r.issues.iter().filter(|i| i.level == Level::Error).map(|i| format!("{}: {}", i.file, i.message)).collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

/// BN6's strings tables in other languages (content/bn6/locale) name only
/// definitions BN6's content has, and stay out of the content: the content
/// root reader leaves them out, so they reach neither the define phase nor
/// `Content::hash()`.
#[test]
fn bn6_locale_tables_name_bn6_definitions_and_stay_out_of_the_content() {
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6"));
    let langs = nettai_content::locale::Strings::languages(dir);
    assert!(langs.contains(&"ja".to_string()), "{langs:?}");
    let mut r = Report::default();
    let root = nettai_content::root::read(dir, &mut r).expect("content/bn6 reads");
    assert!(root.modules.keys().all(|m| !m.starts_with("locale/")), "a strings table read as a module");
    let mut c = nettai_battle::Content::default();
    c.scripts.modules = root.modules;
    c.assets = testing::asset_names_used(&c.scripts.modules);
    c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
    let before = c.hash();
    nettai_content::locale::check_root(dir, &c, &mut r);
    let errors: Vec<String> = r.issues.iter().filter(|i| i.level == Level::Error).map(|i| format!("{}: {}", i.file, i.message)).collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    // A root with a table and one without define the same content.
    let tmp = std::env::temp_dir().join(format!("nettai-locale-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("locale")).unwrap();
    std::fs::write(tmp.join("a.luau"), "return define.record('thing', { n = 1 })").unwrap();
    let read = |r: &mut Report| nettai_content::root::read(&tmp, r).expect("the root reads").modules;
    let without = read(&mut Report::default());
    std::fs::write(tmp.join("locale/ja.toml"), "language = \"ja\"\n").unwrap();
    let mut r2 = Report::default();
    assert_eq!(read(&mut r2), without);
    assert!(!r2.has_errors(), "{r2:?}");
    let _ = std::fs::remove_dir_all(&tmp);
    assert_eq!(c.hash(), before);
}
