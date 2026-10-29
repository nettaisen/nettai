mod common;

use common::{content, full_set, games, recipe, roms};
use std::path::Path;
use std::sync::Arc;

/// Each game's assets, extracted from its full set by its recipe, load with its content (every asset the content
/// names is there, each sprite with the content's animations and frames: `nettai_content::load::asset_numbers`), its
/// content times its sprites as the ROMs do, and they answer every lookup the content audit (nettai-tool's
/// `--audit-content`) makes, in each language the content has strings in.
#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn full_sets_pass_the_content_audit() {
    use nettai_frontend::game::Game;
    for game in games() {
        let loaded = Game::load(&roms(), Some(&content()), &game, &[]).unwrap_or_else(|e| panic!("{game}: {e}\n{}", e.report));
        assert!(!loaded.report.has_errors(), "{game}: {}", loaded.report);
        assert_eq!(loaded.timing_differences, Vec::<String>::new(), "{game}: the content times its sprites otherwise than the ROMs");
        let own = nettai_content::locale::OWN;
        let mut languages = vec![(own.to_string(), None)];
        for lang in nettai_content::locale::languages(&loaded.dir).into_iter().filter(|l| l != own) {
            let strings = nettai_content::locale::load_for(&loaded.dir, std::slice::from_ref(&loaded.name), &lang).unwrap();
            languages.push((lang, strings.map(Arc::new)));
        }
        let found = nettai_tools::content_audit::audit(&loaded.content, (*loaded.graphics).clone(), &loaded.numbers, Some(&loaded.sound), &languages);
        assert!(found.problems.is_empty(), "{game}: {:#?}", found.problems);
    }
}

/// A game's set takes its own ROMs, each once, recognized by the codes in their headers: another game's, an image
/// with no game code and a second copy are refused.
#[test]
fn rejects_unknown_and_duplicate_roms() {
    let six = recipe("exe6");
    let mut roms = six.rom_set();
    assert!(roms.insert(Vec::new()).unwrap_err().to_string().contains("no game code"));
    let mut other = vec![0; 8 * 1024 * 1024];
    other[0xac..0xb0].copy_from_slice(b"BRBE");
    let e = roms.insert(other).unwrap_err().to_string();
    assert!(e.contains("not a ROM of exe6") && e.contains("BRBE") && e.contains("BR6E, BR5E, BR6J, BR5J"), "{e}");
    let mut bytes = vec![0; 8 * 1024 * 1024];
    bytes[0xac..0xb0].copy_from_slice(b"BR6E");
    roms.insert(bytes.clone()).unwrap();
    assert!(roms.contains("BR6E"));
    assert!(roms.insert(bytes).unwrap_err().to_string().contains("duplicate"));
}

/// A ROM's game code is read from its header alone, the first `HEADER_LEN` bytes: each game's four, as its recipe
/// states them; a header cut short, or with no letters and digits there, has none.
#[test]
fn a_roms_code_is_read_from_its_header() {
    for game in games() {
        for code in recipe(&game).roms() {
            let mut header = vec![0; nettai_extract::HEADER_LEN];
            header[0xac..0xb0].copy_from_slice(code.as_bytes());
            assert_eq!(nettai_extract::header_code(&header), Some(code.as_str()));
        }
    }
    let mut other = vec![0; nettai_extract::HEADER_LEN];
    other[0xac..0xb0].copy_from_slice(b"A6BE");
    assert_eq!(nettai_extract::header_code(&other), Some("A6BE"));
    assert_eq!(nettai_extract::header_code(&other[..0xae]), None, "a header cut short");
    assert_eq!(nettai_extract::header_code(&[0; nettai_extract::HEADER_LEN]), None, "no game code");
}

/// A recipe is the content's, as its load read it: a copy of the content's has the same ROMs; and what the recipe
/// runs is what the load read, though its file changes after.
#[test]
fn a_recipe_is_the_contents() {
    let copy = tempfile::tempdir().unwrap();
    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                copy_dir(&entry.path(), &to.join(entry.file_name()));
            } else {
                std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
            }
        }
    }
    for pack in ["exe6", "exelib"] {
        copy_dir(&content().join(pack), &copy.path().join(pack));
    }
    let (c, _) = nettai_content::load::load_battle(copy.path(), "exe6").unwrap_or_else(|r| panic!("{r}"));
    let same = nettai_content::load::recipe(&c).unwrap();
    assert_eq!(same.roms(), recipe("exe6").roms());
    std::fs::write(copy.path().join("exe6/extract/names.luau"), "return {").unwrap();
    assert_eq!(nettai_content::load::recipe(&c).unwrap().roms(), same.roms(), "the recipe's modules are those the load read");
    let e = nettai_content::load::load_battle(copy.path(), "exe6").err().unwrap().to_string();
    assert!(e.contains("syntax error") && e.contains(&copy.path().display().to_string()), "{e}");
}

/// An extraction says how far it is, from 0 to 1 and never back.
#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn an_extraction_reports_how_far_it_is() {
    let mut said = Vec::new();
    let six = recipe("exe6");
    six.extract_reporting(&full_set(&six), &mut |f| said.push(f)).unwrap();
    assert!(said.windows(2).all(|w| w[0] <= w[1]), "{said:?}");
    assert!(said.iter().all(|f| (0.0..=1.0).contains(f)), "{said:?}");
    assert_eq!(said.last(), Some(&1.0), "{said:?}");
}
