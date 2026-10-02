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
    c.assets = testing::asset_names_for(&c.scripts);
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
        c.assets = testing::asset_names_for(&c.scripts);
        c.strings = strings;
        c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
        c
    };
    let c = define(root.strings.clone());
    nettai_content::locale::check_root(dir, &c, &mut r);
    let errors: Vec<String> = r.issues.iter().filter(|i| i.level == Level::Error).map(|i| format!("{}: {}", i.file, i.message)).collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    // The own strings' shape is in the records: MagPanel's one line.
    let magpanl = c.defs.chip_by_key("bn6:magpanl").expect("bn6:magpanl");
    assert_eq!(c.chip(magpanl).description_lines, 1);
    // Other text of the same shape: the same content.
    let mut renamed = root.strings.clone();
    for s in renamed.chips.values_mut() {
        s.name = Some("Renamed".into());
    }
    assert_eq!(define(renamed).hash(), c.hash());
    // Another shape: another content.
    let mut reshaped = root.strings.clone();
    reshaped.chips.get_mut("bn6:magpanl").unwrap().description = Some("one\ntwo".into());
    assert_ne!(define(reshaped).hash(), c.hash());
}

/// docs/design/rules-in-luau.md, the flat namespace: BN5's folder
/// (content/bn5) loads beside BN6's, one namespace, every id in full:
/// `bn5:cannon` and `bn6:cannon` are two chips.
/// (BN5's chips have no use yet, which the define phase refuses: until the
/// BN5 port writes them, the refusal must be that, of a `bn5:` chip.)
#[test]
fn bn5_and_bn6_load_together_under_their_names() {
    let repo = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let mut r = Report::default();
    let bn6 = nettai_content::root::read(&repo.join("content/bn6"), &mut r).expect("content/bn6 reads");
    let bn5 = nettai_content::root::read(&repo.join("content/bn5"), &mut r).expect("content/bn5 reads");
    let mut c = nettai_battle::Content::default();
    c.strings = bn6.strings;
    c.strings.merge(bn5.strings);
    c.scripts = Scripts::root(bn6.manifest, bn6.modules);
    c.scripts.add_root(bn5.manifest, bn5.modules);
    c.assets = testing::asset_names_for(&c.scripts);
    if let Err(e) = c.define() {
        let e = e.message;
        assert!(e.starts_with("bn5:chips/") && e.contains(": chip bn5:") && e.contains("needs exactly one of `action`"), "{e}");
        return;
    }
    let d = &c.defs;
    assert_eq!(d.roots, ["bn5", "bn6"]);
    let (six, five) = (d.chip_by_key("bn6:cannon").expect("bn6:cannon"), d.chip_by_key("bn5:cannon").expect("bn5:cannon"));
    assert_ne!(six, five);
    assert_eq!(d.chip_by_key("cannon"), None, "an id is written in full");
    assert_eq!(d.stock_ruleset_of("bn6"), d.ruleset_by_key("bn6:stock"));
    assert_eq!(c.strings.chip("bn5:cannon").and_then(|s| s.name.as_deref()), Some("Cannon"));
}

/// Whether a chip module names its use (`define.chip`'s `action`, `dimming`,
/// `navi` or `instant`).
fn names_a_use(module: &str) -> bool {
    module.lines().any(|l| {
        let l = l.strip_prefix("    ").unwrap_or("");
        ["action", "dimming", "navi", "instant"].iter().any(|f| l.strip_prefix(f).is_some_and(|r| r.trim_start().starts_with('=')))
    })
}

/// docs/design/rules-in-luau.md R2: BN5's stock ruleset and rule sections
/// (content/bn5/rules) are its game's, beside BN6's: its pools (16 actors),
/// its banners, its element tables. With them, the BN5 chips the port has
/// given uses (docs/design/bn5-map.md §15.6), many of them BN6's code
/// (`require("@bn6/...")`); the rest, without a use yet, are left out (with
/// them the define phase stops at the first, above).
#[test]
fn bn5s_rules_are_its_games() {
    let repo = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let mut r = Report::default();
    let bn6 = nettai_content::root::read(&repo.join("content/bn6"), &mut r).expect("content/bn6 reads");
    let mut bn5 = nettai_content::root::read(&repo.join("content/bn5"), &mut r).expect("content/bn5 reads");
    let unported: Vec<String> = bn5
        .modules
        .iter()
        .filter_map(|(path, text)| Some(path.strip_prefix("chips/")?.strip_suffix("/chip")?).filter(|_| !names_a_use(text)))
        .map(|k| format!("chips/{k}/"))
        .collect();
    bn5.modules.retain(|path, _| !unported.iter().any(|k| path.starts_with(k.as_str())));
    let mut c = nettai_battle::Content {
        strings: bn6.strings,
        scripts: Scripts::root(bn6.manifest, bn6.modules),
        ..Default::default()
    };
    c.scripts.add_root(bn5.manifest, bn5.modules);
    // Each root's names in its own assets pack (BN5's in bn5's).
    c.assets = testing::asset_names_for(&c.scripts);
    c.define().unwrap_or_else(|e| panic!("{e}"));
    let d = &c.defs;
    let five = d.root_id("bn5").expect("the bn5 root");
    assert!(d.stock_ruleset_of("bn5").is_some(), "BN5's stock ruleset");
    assert_eq!(d.stock_ruleset_of("bn6"), d.ruleset_by_key("bn6:stock"));
    let (six, five) = (c.rules_of(d.root_id("bn6").expect("the bn6 game")), c.rules_of(five));
    assert_eq!(five.pools.slots(), [16, 32, 32]);
    assert_eq!(six.pools.slots(), [32, 32, 32]);
    // BN5's tables where they are BN6's, and where they aren't.
    assert_eq!(five.element_weakness, six.element_weakness);
    assert_eq!(five.sine, six.sine);
    assert_eq!(five.holding_banners.len(), 3);
    assert_eq!(five.hp_bug_periods, six.hp_bug_periods);
    // The ported chips: BN5's own, apart from BN6's of the same key.
    for key in ["cannon", "minibomb", "energbom", "panlgrab", "antiswrd", "holypanl", "fullcust"] {
        let (six, five) = (d.chip_by_key(&format!("bn6:{key}")), d.chip_by_key(&format!("bn5:{key}")));
        assert!(five.is_some() && six != five, "bn5:{key}");
    }
    // docs/design/bn5-map.md §15.3 items 10 and 11: BN5's chips leave
    // their action on the use frame, and AntiNavi's sparkle sits on the
    // panel's center.
    assert!(five.chip_use.leave_on_use && !six.chip_use.leave_on_use);
    assert_eq!((five.chip_use.anti_navi_sparkle.dy, five.chip_use.anti_navi_sparkle.z), (0, 16));
    assert_eq!((six.chip_use.anti_navi_sparkle.dy, six.chip_use.anti_navi_sparkle.z), (16, 32));
    // Item 9: no BN5 module that uses BN6's names a BN5 collision type
    // that tests BN6's 0x80 self bit (BN5's own row 0x3D, `probe`, does).
    assert_eq!(nettai_content::lint::self_bit_targets(&c), vec![]);
    let text = "local shot = require(\"@bn6/lib/shot\")\nlocal collision = require(\"../../rules/collision\")\n\
                return shot.chip { hits = collision.probe }";
    c.scripts.modules.insert("bn5:chips/probing/chip".into(), text.into());
    let found = nettai_content::lint::self_bit_targets(&c);
    assert_eq!(found, vec![("bn5:probe".to_string(), "bn5:chips/probing/chip".to_string(), "bn6".to_string())]);
}
