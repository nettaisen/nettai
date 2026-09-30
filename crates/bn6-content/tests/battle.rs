//! The battle data of a pack, on the engine's hand-authored test content
//! (no game data): written, read back as the same `Content`, readable and
//! editable as TOML, and checked for broken ids and references.

use bn6_battle::content::{ChipModifier, PaRecipe, ProgramAdvanceRecipe, testing};
use bn6_content::battle;
use bn6_content::pack;
use bn6_content::report::{Level, Report};
use std::path::{Path, PathBuf};

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bn6-content-battle-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The test content with a few things only BN6-like content has: a
/// Program Advance, a modifier chip, a link navi's own chip.
fn content() -> bn6_battle::Content {
    let mut c = testing::build();
    c.chips[testing::SUN_GUN_EX as usize].program_advances.push(ProgramAdvanceRecipe {
        order: 0,
        recipe: PaRecipe::CodeRun { chip: testing::SUN_GUN_1, count: 3 },
    });
    c.chips[testing::SUN_GUN_EX as usize].program_advances.push(ProgramAdvanceRecipe {
        order: 1,
        recipe: PaRecipe::Sequence(vec![testing::SUN_GUN_1, testing::SUN_GUN_2, testing::SUN_GUN_3]),
    });
    c.chips[0].modifier = Some(ChipModifier::AttackPlus);
    // The registries hold the records: define again.
    c.define().unwrap();
    c
}

fn write(dir: &Path, c: &bn6_battle::Content) {
    let mut files = vec![pack::manifest("test", None, false, true)];
    files.extend(battle::export(c));
    files.extend(battle::export_timing(c));
    pack::write_files(dir, &files).unwrap();
}

fn load(dir: &Path) -> (Option<bn6_battle::Content>, Report) {
    let mut r = Report::default();
    let c = battle::load(dir, &mut r);
    (c, r)
}

#[test]
fn battle_data_reads_back_as_the_same_content() {
    let dir = temp("roundtrip");
    let c = content();
    write(&dir, &c);
    let (back, report) = load(&dir);
    assert!(!report.has_errors(), "{report}");
    // The files hold the data and the scripts; defining what they read
    // back gives the same registries (the define phase is deterministic).
    // (Asset names aren't battle data: the asset index gives them,
    // docs/design/content-model-v2.md §9.2.)
    let mut back = back.unwrap();
    back.assets = c.assets.clone();
    back.define().unwrap();
    assert_eq!(bn6_content::verify::compare_battle(&c, &back), Vec::<String>::new());
    assert_eq!(back, c);
    assert_eq!(back.hash(), c.hash());
    assert_eq!(back.program_advances().len(), 2);
    // By owner: a chip's folder, a navi's, the rules and registries.
    assert!(dir.join("chips/003-sungun3/chip.toml").is_file());
    assert!(dir.join("navis/00-megaman/navi.toml").is_file());
    assert!(dir.join("navis/00-megaman/forms/00-base/form.toml").is_file());
    assert!(dir.join("rules/collision.toml").is_file());
    assert!(dir.join("registries/effects.toml").is_file());
}

#[test]
fn files_are_plain_toml_with_names_and_hex_ids() {
    let dir = temp("text");
    write(&dir, &content());
    let chip = std::fs::read_to_string(dir.join("chips/003-sungun3/chip.toml")).unwrap();
    for line in ["id = 0x03", "name = \"SunGun3\"", "codes = [\"A\", \"*\"]", "family = \"null\"", "action = 0x37", "[gun_del_sol]", "firing_ticks = 96"] {
        assert!(chip.contains(line), "{line:?} in\n{chip}");
    }
    let ex = std::fs::read_to_string(dir.join("chips/004-sungunx/chip.toml")).unwrap();
    assert!(ex.contains("[[program_advance]]") && ex.contains("sequence = [0x01, 0x02, 0x03]"), "{ex}");
    let collision = std::fs::read_to_string(dir.join("rules/collision.toml")).unwrap();
    assert!(collision.starts_with("# Collision types"), "{collision}");
    assert!(collision.contains("side0 = 0x08410080"), "{collision}");
}

/// The loader fills the content's asset names from the pack's index.
#[test]
fn the_loader_names_the_assets_from_the_index() {
    let dir = temp("index");
    let c = content();
    write(&dir, &c);
    pack::write_files(&dir, &vec![bn6_content::names::index_file(&c.assets)]).unwrap();
    let (loaded, report) = pack::load_battle(&dir).unwrap_or_else(|r| panic!("{r}"));
    assert!(!report.has_errors(), "{report}");
    assert_eq!(loaded.assets, c.assets);
    assert!(loaded.assets.sounds.contains_key("test-tick"));
}

#[test]
fn an_edit_comes_through() {
    let dir = temp("edit");
    write(&dir, &content());
    let path = dir.join("chips/003-sungun3/chip.toml");
    let text = std::fs::read_to_string(&path).unwrap().replace("firing_ticks = 96", "firing_ticks = 55");
    std::fs::write(&path, text).unwrap();
    let (back, report) = load(&dir);
    assert!(!report.has_errors(), "{report}");
    let mut back = back.unwrap();
    back.define().unwrap();
    assert_eq!(back.chip(back.chip_numbered(testing::SUN_GUN_3).unwrap()).gun_del_sol.unwrap().firing_ticks, 55);
    assert_ne!(back.hash(), content().hash(), "the content's identity changes with it");
}

#[test]
fn broken_ids_and_references_are_reported_by_file() {
    let dir = temp("broken");
    write(&dir, &content());
    // A second chip claiming id 3, and a battle settings entry naming an
    // actor list that doesn't exist.
    let text = std::fs::read_to_string(dir.join("chips/003-sungun3/chip.toml")).unwrap();
    std::fs::create_dir_all(dir.join("chips/003-copy")).unwrap();
    std::fs::write(dir.join("chips/003-copy/chip.toml"), text).unwrap();
    let stages = dir.join("rules/stages.toml");
    let text = std::fs::read_to_string(&stages).unwrap().replacen("actor_list = 0", "actor_list = 9", 1);
    std::fs::write(&stages, text).unwrap();
    let (back, report) = load(&dir);
    assert!(back.is_none());
    let errors: Vec<String> = report.issues.iter().filter(|i| i.level == Level::Error).map(|i| i.to_string()).collect();
    assert!(errors.iter().any(|e| e.contains("chips/003-sungun3/chip.toml") && e.contains("chip 0x3 is also in")), "{errors:?}");
    assert!(errors.iter().any(|e| e.contains("rules/stages.toml") && e.contains("actor list 9 doesn't exist")), "{errors:?}");
}

#[test]
fn a_gap_in_the_chips_is_an_error() {
    let dir = temp("gap");
    write(&dir, &content());
    std::fs::remove_dir_all(dir.join("chips/002-sungun2")).unwrap();
    let (back, report) = load(&dir);
    assert!(back.is_none());
    assert!(report.issues.iter().any(|i| i.level == Level::Error && i.message.contains("0x2 is missing")), "{report}");
}
