mod common;

use common::{content, full_set, games, recipe};
use nettai_content::{pack, report::Report};
use nettai_extract::Recipe;
use std::path::Path;
use std::sync::Arc;

/// What the content audit (nettai-tool's `--audit-content`) finds missing
/// of `game`'s content drawn and heard with the pack at `dir`: every
/// lookup a frontend makes of everything the content defines, in each
/// language it has strings in.
fn audit(game: &str, dir: &Path) -> Vec<String> {
    use nettai_frontend::game::{Found, Game as Loaded};
    // (A packs directory with no pack in it: the pack is the one given.)
    let none = tempfile::tempdir().unwrap();
    let found = Found::find(none.path(), &[dir.to_path_buf()]).unwrap_or_else(|e| panic!("{e}"));
    let loaded = Loaded::load(&found, Some(&content()), game, &[]).unwrap_or_else(|e| panic!("{e}"));
    let bundles = loaded.packs.iter().map(|p| pack::load_graphics(p).unwrap_or_else(|r| panic!("{r}")).0).collect();
    let banks = loaded.sound().unwrap_or_else(|e| panic!("{e}")).banks;
    let own = nettai_content::locale::OWN;
    let mut languages = vec![(own.to_string(), None)];
    for lang in nettai_content::locale::languages(&loaded.dir).into_iter().filter(|l| l != own) {
        let strings = nettai_content::locale::load_for(&loaded.dir, std::slice::from_ref(&loaded.name), &lang).unwrap();
        languages.push((lang, strings.map(Arc::new)));
    }
    nettai_tools::content_audit::audit(&loaded.content, bundles, loaded.own, &loaded.assets, Some(&banks), &languages).problems
}

/// Each game's pack of its full set, made by its recipe, loads with its content and answers every lookup the
/// content audit makes: an asset the content names that the pack lacks fails it. And it has what the content names,
/// each sprite with the content's animations (`pack::pack_assets`, the CLI's `--check`), and it is written as `write`
/// writes it.
#[test]
#[ignore = "requires local ROMs via NETTAI_EXTRACT_ROM_DIR"]
fn full_sets_pass_the_content_audit() {
    for game in games() {
        let recipe = recipe(&game);
        let assets = recipe.extract(&full_set(&recipe)).unwrap_or_else(|e| panic!("{e}"));
        let dir = tempfile::tempdir().unwrap();
        assets.write(dir.path()).unwrap();
        let (loaded, _) = pack::load_battle(&content(), &game).unwrap_or_else(|e| panic!("{game}: {e}"));
        let (_, report) = pack::pack_assets(&loaded, dir.path()).unwrap_or_else(|e| panic!("{game}: {e}"));
        assert!(!report.has_errors(), "{game}: {report}");
        let problems = audit(&game, dir.path());
        assert!(problems.is_empty(), "{game}: {problems:#?}");
        // Writing a second time cannot leave stale graphics from an earlier pack.
        assert!(assets.write(dir.path()).unwrap_err().to_string().contains("not empty"));
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

/// Each game's packs have a fingerprint of their own, the same each time it is asked for.
#[test]
fn each_game_has_its_own_fingerprint() {
    let print = |game: &String| recipe(game).fingerprint();
    let all = games();
    let prints: Vec<String> = all.iter().map(print).collect();
    assert!(prints.iter().all(|p| p.len() == 16 && p.chars().all(|c| c.is_ascii_hexdigit())), "{prints:?}");
    assert_eq!(prints, all.iter().map(print).collect::<Vec<_>>());
    for (i, p) in prints.iter().enumerate() {
        assert!(!prints[..i].contains(p), "{prints:?}");
    }
}

/// The names are the content's, read when the extraction runs: content whose `assets.toml` names an asset
/// otherwise, or that has no `assets.toml`, has another fingerprint. A recipe that doesn't load is an error that
/// names the folder.
#[test]
fn the_names_are_the_contents() {
    let copy = tempfile::tempdir().unwrap();
    // (What the recipe reads of the content: the game's compat/ and extract/, and exelib's extract/; the manifests
    // say which packs they are.)
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
    for (pack, parts) in [("exe6", &["compat", "extract"][..]), ("exelib", &["extract"][..])] {
        std::fs::create_dir_all(copy.path().join(pack)).unwrap();
        std::fs::copy(content().join(pack).join("manifest.toml"), copy.path().join(pack).join("manifest.toml")).unwrap();
        for part in parts {
            if content().join(pack).join(part).is_dir() {
                copy_dir(&content().join(pack).join(part), &copy.path().join(pack).join(part));
            }
        }
    }
    let compat = copy.path().join("exe6/compat");
    let same = Recipe::of_content(copy.path(), "exe6").unwrap();
    assert_eq!(same.fingerprint(), recipe("exe6").fingerprint());
    let assets = compat.join("assets.toml");
    let text = std::fs::read_to_string(&assets).unwrap().replacen("\ndummy-shield = ", "\nwooden-shield = ", 1);
    std::fs::write(&assets, text).unwrap();
    let renamed = Recipe::of_content(copy.path(), "exe6").unwrap();
    assert_ne!(renamed.fingerprint(), same.fingerprint());
    std::fs::remove_file(&assets).unwrap();
    let gone = Recipe::of_content(copy.path(), "exe6").unwrap();
    assert!(gone.fingerprint() != renamed.fingerprint() && gone.fingerprint() != same.fingerprint());
    std::fs::write(copy.path().join("exe6/extract/names.luau"), "return {").unwrap();
    let e = Recipe::of_content(copy.path(), "exe6").unwrap_err().to_string();
    assert!(e.contains("syntax error") && e.contains(&copy.path().display().to_string()), "{e}");
}

/// The extraction and the writing say how far they are, from 0 to 1 and never back; and what the writing that
/// reports writes is `write`'s, byte for byte.
#[test]
#[ignore = "requires local ROMs via NETTAI_EXTRACT_ROM_DIR"]
fn an_extraction_reports_how_far_it_is() {
    let mut said = Vec::new();
    let six = recipe("exe6");
    let assets = six.extract_reporting(&full_set(&six), &mut |f| said.push(f)).unwrap();
    let (reported, plain) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    assets.write_reporting(reported.path(), &mut |f| said.push(f + 1.0)).unwrap();
    assets.write(plain.path()).unwrap();
    assert!(said.windows(2).all(|w| w[0] <= w[1]), "{said:?}");
    assert!(said.iter().all(|f| (0.0..=2.0).contains(f)), "{said:?}");
    assert!(said.contains(&1.0) && said.last() == Some(&2.0), "{said:?}");
    fn files(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        let mut out = Vec::new();
        let mut dirs = vec![root.to_path_buf()];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    out.push((path.strip_prefix(root).unwrap().to_path_buf(), std::fs::read(&path).unwrap()));
                }
            }
        }
        out.sort();
        out
    }
    let (a, b) = (files(reported.path()), files(plain.path()));
    assert!(!a.is_empty());
    assert!(a == b, "the two writings differ");
}

#[test]
#[ignore = "requires original ROMs and existing reference packs"]
fn full_sets_preserve_reference_graphics() {
    let packs = std::path::PathBuf::from(std::env::var_os("NETTAI_EXTRACT_REFERENCE_PACKS").expect("NETTAI_EXTRACT_REFERENCE_PACKS"));
    for game in games() {
        let recipe = recipe(&game);
        let a = recipe.extract(&full_set(&recipe)).unwrap();
        if game == "exe6" {
            assert!(a.sound.songs[0x63].is_none(), "stop-music is intentionally empty");
        }
        let mut report = Report::default();
        let b = pack::import_graphics(&packs.join(&game), &mut report).unwrap_or_else(|| panic!("{report}"));
        assert!(a.graphics.sprites == b.sprites, "{game} sprites");
        assert!(a.graphics.field == b.field, "{game} field");
        assert!(a.graphics.backgrounds == b.backgrounds, "{game} backgrounds");
        assert!(a.graphics.hud == b.hud, "{game} HUD");
        assert!(a.graphics.custom == b.custom, "{game} custom screen");
    }
}
