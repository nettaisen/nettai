//! `nettai-content check`'s report on definitions, on the engine's test pack
//! and on EXE6's content.

use nettai_battle::content::{Scripts, testing};
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
    let roles = c.scripts.module_mut(testing::ROOT, "test/rules/roles").expect("the test pack's roles");
    assert!(roles.contains("    sparks = ruleset.sparks,\n"));
    *roles = roles.replace("    sparks = ruleset.sparks,\n", "    sparks = { plain = ruleset.sparks.plain },\n");
    // (The test game's index loads every module of it: written again with the new ones.)
    testing::add_index(&mut c.scripts, testing::ROOT);
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
    testing::add_index(&mut c.scripts, testing::ROOT);
    c.define().unwrap();
    // (The test content's own types share rows with EXE6's, whose module
    // it has too: only these are looked at.)
    let twins: Vec<_> = nettai_content::lint::duplicate_collision_types(&c).into_iter().filter(|(row, _)| *row >= 0xFE).collect();
    assert_eq!(twins, [(0xFE, vec![("one".to_string(), "test:lib/one".to_string()), ("two".to_string(), "test:lib/two".to_string())])]);
    let mut r = Report::default();
    nettai_content::lint::definitions(&c, &mut r);
    let errors: Vec<&str> = r.issues.iter().filter(|i| i.level == Level::Error).map(|i| i.message.as_str()).collect();
    assert!(errors.iter().any(|e| e.contains("collision type one is row 0xfe") && e.contains("two (test:lib/two.luau)")), "{errors:?}");
}

/// The repository's content directory.
const CONTENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");

/// content/'s games `games` (their packs: a load reads each module as it
/// requires it) on made-up asset indices, with their strings: not defined.
fn read(games: &[&str]) -> nettai_battle::Content {
    let mut r = Report::default();
    let games: Vec<String> = games.iter().map(|g| g.to_string()).collect();
    let read = nettai_content::index::read(std::path::Path::new(CONTENT), &games, &mut r).unwrap_or_else(|| panic!("{r}"));
    let mut c = nettai_battle::Content::default();
    c.scripts = read.scripts();
    c.strings = read.strings;
    c.assets = testing::asset_names_for(&c.scripts);
    c
}

/// EXE6's content defines without an error in its definitions (each
/// collision type once).
#[test]
fn exe6_content_has_no_definition_errors() {
    let mut c = read(&["exe6"]);
    c.define().unwrap_or_else(|e| panic!("content/exe6: {e}"));
    let mut r = Report::default();
    nettai_content::lint::definitions(&c, &mut r);
    let errors: Vec<String> =
        r.issues.iter().filter(|i| i.level == Level::Error).map(|i| format!("{}: {}", i.file, i.message)).collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

/// The strings tables (content/<game>/locales) name only definitions the content
/// has, and every chip, navi and form of EXE6 has its name in the own
/// language's; so do the library orders (content/<game>/library.toml). The loader reads them apart from the modules; the own
/// language's strings shape the records (a description's lines), and the
/// hash covers that shape alone: text that keeps it changes nothing.
#[test]
fn the_strings_name_the_contents_definitions_and_only_their_shape_is_hashed() {
    let dir = std::path::Path::new(CONTENT);
    let langs = nettai_content::locale::languages(dir);
    assert!(langs.contains(&"en".to_string()) && langs.contains(&"ja".to_string()), "{langs:?}");
    let base = read(&["exe6"]);
    assert!(base.scripts.modules.keys().all(|m| !m.contains("locales/")), "a strings table read as a module");
    let define = |strings: nettai_content::locale::Strings| {
        let mut c = base.clone();
        c.strings = strings;
        c.define().unwrap_or_else(|e| panic!("content/: {e}"));
        c
    };
    let c = define(base.strings.clone());
    let mut r = Report::default();
    nettai_content::locale::check_games(dir, &c, &mut r);
    let mut five = read(&["exe5"]);
    five.define().unwrap_or_else(|e| panic!("content/exe5: {e}"));
    nettai_content::locale::check_games(dir, &five, &mut r);
    // The library orders (content/<game>/library.toml) name the games' own
    // definitions, each once, in sections their classes fit.
    nettai_content::library::check_games(dir, &c, &mut r);
    nettai_content::library::check_games(dir, &five, &mut r);
    for game in ["exe5", "exe6"] {
        assert!(nettai_content::library::load(&dir.join(game)).unwrap().is_some(), "content/{game} has no library.toml");
    }
    // What the check refuses: a chip of another class, a key twice, a key
    // nothing has.
    use nettai_content::library::{Library, Section};
    let wrong = Library {
        chips: vec![(Section::Mega, vec!["cannon".into(), "roll".into(), "roll".into(), "nosuch".into()])],
        navicust: vec!["nosuch".into()],
        patch_cards: Vec::new(),
    };
    assert_eq!(
        nettai_content::library::check(&wrong, &c.defs),
        [
            "chips.mega: cannon is a Standard chip",
            "chips.mega: roll is in chips.mega already",
            "chips.mega: no chip has the key nosuch",
            "navicust: nothing has the key nosuch",
        ]
    );
    let errors: Vec<String> = r.issues.iter().filter(|i| i.level == Level::Error).map(|i| format!("{}: {}", i.file, i.message)).collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    // The own strings' shape is in the records: MagPanel's one line.
    let magpanl = c.defs.chip_by_key("magpanl").expect("magpanl");
    assert_eq!(c.chip(magpanl).description_lines, 1);
    // Other text of the same shape: the same content.
    let mut renamed = base.strings.clone();
    for s in renamed.chips.values_mut() {
        s.name = Some("Renamed".into());
    }
    assert_eq!(define(renamed).hash(), c.hash());
    // Another shape: another content.
    let mut reshaped = base.strings.clone();
    reshaped.chips.get_mut("magpanl").unwrap().description = Some("one\ntwo".into());
    assert_ne!(define(reshaped).hash(), c.hash());
}

/// docs/design/content-model-v2.md §4.0: content is one game, its ids
/// local: EXE5's content has `cannon`, EXE5's own, and a name written with a
/// game (`exe6:cannon`, `exe5:cannon`) names nothing. Two games are two
/// contents.
#[test]
fn a_game_loads_alone_under_its_names() {
    let mut c = read(&["exe5"]);
    c.define().unwrap_or_else(|e| panic!("{e}"));
    let d = &c.defs;
    assert_eq!((c.game(), d.game.as_str()), ("exe5", "exe5"));
    assert!(d.chip_by_key("cannon").is_some(), "an id is local to its game");
    assert_eq!(d.chip_by_key("exe6:cannon"), None, "EXE6's chips are another content's"); // (written in full)
    assert_eq!(d.chip_by_key("exe5:cannon"), None, "an id is written without its game"); // (written in full)
    assert!(d.ruleset().is_some_and(|r| !r.systems.is_empty()), "EXE5's one ruleset");
    assert_eq!(c.strings.chip("cannon").and_then(|s| s.name.as_deref()), Some("Cannon"));
    let mut both = read(&["exe6", "exe5"]);
    let e = both.define().unwrap_err().message;
    assert!(e.contains("content holds one game"), "{e}");
}

/// docs/design/content-model-v2.md §4.0: the order of a game's inits'
/// requires is the order its modules load in, and nothing more. Each
/// game's requires turned round (its top module's: its rules last; each
/// folder's init's: its chips from the last to the first) define the same
/// definitions under the same keys, in the same places: no handle moves.
/// (An anonymous key counts its own module's definitions, whenever the
/// module loads.)
#[test]
fn the_order_of_a_games_requires_moves_no_key_and_no_handle() {
    use nettai_content_api::packs;
    for game in ["exe6", "exe5"] {
        let mut c = read(&[game]);
        let mut turned = c.clone();
        c.define().unwrap_or_else(|e| panic!("content/{game}: {e}"));
        // The game's indexes among what the load read: its top module, and
        // each init that is nothing but requires. Turned round, they stand
        // in for the folders' own.
        let top = packs::top_module(game);
        assert_eq!(packs::requires(&c.scripts.modules[&top])[0], "@self/rules", "{game}/init.luau");
        let indexes: Vec<&String> = c.scripts.modules.iter().filter(|(m, s)| m.starts_with(game) && packs::is_index(s)).map(|(m, _)| m).collect();
        assert!(indexes.len() >= 5 && indexes.contains(&&top), "{game}'s indexes: {indexes:?}");
        let mut count = 0;
        for index in indexes {
            let mut requires = packs::requires(&c.scripts.modules[index]);
            count += requires.len();
            requires.reverse();
            let lines: Vec<String> = requires.iter().map(|r| format!("require(\"{r}\")\n")).collect();
            turned.scripts.modules.insert(index.clone(), lines.concat());
        }
        assert!(count > 300, "{game}'s inits: {count} requires");
        turned.define().unwrap_or_else(|e| panic!("content/{game}, turned round: {e}"));
        assert!(c.defs.definitions.defs.len() > 2000, "{game}: {} definitions", c.defs.definitions.defs.len());
        assert_eq!(c.defs.definitions, turned.defs.definitions, "{game}");
        assert_eq!(c.scripts.modules.keys().collect::<Vec<_>>(), turned.scripts.modules.keys().collect::<Vec<_>>(), "{game}: the same modules read");
        assert_eq!(c.defs.chips.len(), turned.defs.chips.len());
        for key in ["cannon", "minibomb"] {
            assert_eq!(c.defs.chip_by_key(key), turned.defs.chip_by_key(key), "{game}: {key}'s handle");
        }
        assert_ne!(c.hash(), turned.hash(), "the hash covers the modules' text, an init.luau's too");
    }
}

/// docs/design/content-model-v2.md §4.0: what loads is what the games'
/// top modules (their init.luau) require, in turn, each module read as its
/// require is reached. Each game reads none of the other's, and both read
/// the support pack's modules their requires reach, no other.
#[test]
fn a_load_reads_what_its_games_inits_reach() {
    let (mut six, mut five) = (read(&["exe6"]), read(&["exe5"]));
    assert!(six.scripts.modules.is_empty() && five.scripts.modules.is_empty(), "nothing is read before the load");
    six.define().unwrap_or_else(|e| panic!("content/exe6: {e}"));
    five.define().unwrap_or_else(|e| panic!("content/exe5: {e}"));
    assert!(six.scripts.modules.keys().all(|m| !m.starts_with("exe5")), "EXE6 reads none of EXE5's");
    assert!(five.scripts.modules.keys().all(|m| !m.starts_with("exe6")), "EXE5 reads none of EXE6's");
    for c in [&six, &five] {
        assert!(c.scripts.modules.contains_key(&nettai_content_api::packs::top_module(c.game())), "its top module");
        assert!(c.scripts.modules.keys().any(|m| m.starts_with("exelib:")), "the support pack's modules its modules require");
        assert_eq!(c.scripts.packs[0].id, "exelib", "the support pack first");
    }
}

/// docs/design/rules-in-luau.md R2: EXE5's ruleset and rule sections
/// (content/exe5/rules) are its game's, beside EXE6's: its pools (16 actors),
/// its banners, its element tables. With them, the EXE5 chips the port has
/// given uses (docs/design/exe5-map.md §15.6); one without a use yet isn't
/// required by its init.luau.
#[test]
fn exe5s_rules_are_its_games() {
    let mut c = read(&["exe5"]);
    c.define().unwrap_or_else(|e| panic!("{e}"));
    let mut exe6 = read(&["exe6"]);
    exe6.define().unwrap_or_else(|e| panic!("{e}"));
    let d = &c.defs;
    assert!(d.ruleset().is_some(), "EXE5's ruleset");
    let (six, five) = (exe6.rules(), c.rules());
    assert_eq!(five.pools.slots(), [16, 32, 32]);
    assert_eq!(six.pools.slots(), [32, 32, 32]);
    // EXE5's tables where they are EXE6's, and where they aren't.
    assert_eq!(five.element_weakness, six.element_weakness);
    assert_eq!(five.sine, six.sine);
    assert_eq!(five.holding_banners.len(), 3);
    assert_eq!(five.hp_bug_periods, six.hp_bug_periods);
    // The ported chips: EXE5's own, apart from EXE6's of the same key.
    for key in ["cannon", "minibomb", "energbom", "panlgrab", "antiswrd", "holypanl", "fullcust"] {
        assert!(d.chip_by_key(&format!("{key}")).is_some(), "{key}");
    }
    // docs/design/exe5-map.md §15.3 items 10 and 11: EXE5's chips leave
    // their action on the use frame, and AntiNavi's sparkle sits on the
    // panel's center.
    assert!(five.chip_use.leave_on_use && !six.chip_use.leave_on_use);
    assert_eq!((five.chip_use.anti_navi_sparkle.dy, five.chip_use.anti_navi_sparkle.z), (0, 16));
    assert_eq!((six.chip_use.anti_navi_sparkle.dy, six.chip_use.anti_navi_sparkle.z), (16, 32));
    // Item 9: no EXE5 module that uses exelib's makers names an EXE5 collision
    // type that tests EXE6's 0x80 self bit (EXE5's own row 0x3D, `probe`,
    // does).
    assert_eq!(nettai_content::lint::self_bit_targets(&c), vec![]);
    let text = "local slash = require(\"@exelib/swords/slash\")\nlocal collision = require(\"../../rules/collision\")\n\
                return slash.chip { hits = collision.probe }";
    c.scripts.modules.insert("exe5:chips/probing/chip".into(), text.into());
    let found = nettai_content::lint::self_bit_targets(&c);
    assert_eq!(found, vec![("probe".to_string(), "exe5:chips/probing/chip".to_string(), "exelib".to_string())]);
}
