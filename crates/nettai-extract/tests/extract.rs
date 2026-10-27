mod common;

use common::{content, full_set, games, tables};
use nettai_content::{pack, report::Report};
use nettai_extract::{Game, RomSet, Tables, extract};
use std::path::Path;
use std::sync::Arc;

/// What the content audit (nettai-tool's `--audit-content`) finds missing
/// of `game`'s content drawn and heard with the pack at `dir`: every
/// lookup a frontend makes of everything the content defines, in each
/// language it has strings in.
fn audit(game: Game, dir: &Path) -> Vec<String> {
    use nettai_frontend::game::{Found, Game as Loaded};
    // (A packs directory with no pack in it: the pack is the one given.)
    let none = tempfile::tempdir().unwrap();
    let found = Found::find(none.path(), &[dir.to_path_buf()]).unwrap_or_else(|e| panic!("{e}"));
    let loaded = Loaded::load(&found, Some(&content()), game.id(), &[]).unwrap_or_else(|e| panic!("{e}"));
    let bundles = loaded.packs.iter().map(|p| pack::load_graphics(p).unwrap_or_else(|r| panic!("{r}")).0).collect();
    let banks = loaded.sound().unwrap_or_else(|e| panic!("{e}")).banks;
    let own = nettai_content::locale::OWN;
    let mut languages = vec![(own.to_string(), None)];
    for lang in nettai_content::locale::languages(&loaded.dir).into_iter().filter(|l| l != own) {
        let strings = nettai_content::locale::load_for(&loaded.dir, std::slice::from_ref(&loaded.name), &lang).unwrap();
        languages.push((lang, strings.map(Arc::new)));
    }
    nettai_tools::content_audit::audit(&loaded.content, bundles, loaded.own, Some(&banks), &languages).problems
}

/// Each game's pack of its full set (by its recipe, where its content declares one) loads with its content and
/// answers every lookup the content audit makes: an asset the content names that the pack lacks fails it. And a
/// battle's definitions load against it (`pack::load_battle`, the CLI's `--content`), and it is written as
/// `write` writes it, with nothing named a placeholder.
#[test]
#[ignore = "requires local ROMs via NETTAI_EXTRACT_ROM_DIR"]
fn full_sets_pass_the_content_audit() {
    for game in games() {
        let assets = extract(&tables(game), &full_set(game)).unwrap_or_else(|e| panic!("{e}"));
        assert!(assets.placeholders.is_empty() && assets.missing_roms.is_empty(), "{}: {:?}", game.id(), assets.placeholders);
        let dir = tempfile::tempdir().unwrap();
        assets.write(dir.path()).unwrap();
        let (_, report) = pack::load_battle(&content(), dir.path()).unwrap_or_else(|e| panic!("{}: {e}", game.id()));
        assert!(!report.has_errors(), "{}: {report}", game.id());
        let problems = audit(game, dir.path());
        assert!(problems.is_empty(), "{}: {problems:#?}", game.id());
        // Writing a second time cannot leave stale graphics from an earlier pack.
        assert!(assets.write(dir.path()).unwrap_err().to_string().contains("not empty"));
    }
}

#[test]
fn rejects_unknown_truncated_and_duplicate_roms() {
    let mut roms = RomSet::default();
    assert!(roms.insert(Vec::new()).is_err());
    let mut short = vec![0; 0xb0];
    short[0xac..0xb0].copy_from_slice(b"BR6E");
    assert!(roms.insert(short).unwrap_err().to_string().contains("8 MiB"));
    let mut bytes = vec![0; 8 * 1024 * 1024];
    bytes[0xac..0xb0].copy_from_slice(b"BR6E");
    roms.insert(bytes.clone()).unwrap();
    assert!(roms.contains("BR6E"));
    assert!(roms.insert(bytes).unwrap_err().to_string().contains("duplicate"));
}

/// A ROM is told by its header alone, the first `HEADER_LEN` bytes: its game and its code, for each of the
/// games' four; anything else is no game's. And a game is found by its id.
#[test]
fn a_rom_is_recognized_by_its_header() {
    for game in Game::ALL {
        assert_eq!(Game::named(game.id()), Some(game));
        for code in game.codes() {
            let mut header = vec![0; nettai_extract::HEADER_LEN];
            header[0xac..0xb0].copy_from_slice(code.as_bytes());
            assert_eq!(nettai_extract::recognize(&header), Some((game, code)));
        }
    }
    let mut other = vec![0; nettai_extract::HEADER_LEN];
    other[0xac..0xb0].copy_from_slice(b"A6BE");
    assert_eq!(nettai_extract::recognize(&other), None);
    assert_eq!(nettai_extract::recognize(&other[..0xae]), None, "a header cut short");
    assert_eq!(Game::named("exe3"), None);
}

/// Each game's packs have a fingerprint of their own, the same each time it is asked for.
#[test]
fn each_game_has_its_own_fingerprint() {
    let print = |game| nettai_extract::fingerprint(&tables(game));
    let prints: Vec<String> = Game::ALL.into_iter().map(print).collect();
    assert!(prints.iter().all(|p| p.len() == 16 && p.chars().all(|c| c.is_ascii_hexdigit())), "{prints:?}");
    assert_eq!(prints, Game::ALL.map(print));
    assert!(prints[0] != prints[1] && prints[1] != prints[2] && prints[0] != prints[2], "{prints:?}");
}

/// The names are the content's, read when the extraction runs: content whose `assets.toml` names an asset
/// otherwise has another fingerprint. Tables that don't read are an error that names the folder.
#[test]
fn the_names_are_the_contents() {
    let copy = tempfile::tempdir().unwrap();
    // (What the tables read of the content: the game's manifest, compat and extract/, and exelib's.)
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
    let same = Tables::of_content(Game::Exe6, copy.path()).unwrap();
    assert_eq!(nettai_extract::fingerprint(&same), nettai_extract::fingerprint(&tables(Game::Exe6)));
    let assets = compat.join("assets.toml");
    let text = std::fs::read_to_string(&assets).unwrap().replacen("\ndummy-shield = ", "\nwooden-shield = ", 1);
    std::fs::write(&assets, text).unwrap();
    let renamed = Tables::of_content(Game::Exe6, copy.path()).unwrap();
    assert_ne!(nettai_extract::fingerprint(&renamed), nettai_extract::fingerprint(&same));
    std::fs::remove_file(&assets).unwrap();
    let e = Tables::of_content(Game::Exe6, copy.path()).unwrap_err().to_string();
    assert!(e.contains("assets.toml") && e.contains(&copy.path().display().to_string()), "{e}");
}

/// The extraction and the writing say how far they are, from 0 to 1 and never back; and what the writing that
/// reports writes is `write`'s, byte for byte.
#[test]
#[ignore = "requires local ROMs via NETTAI_EXTRACT_ROM_DIR"]
fn an_extraction_reports_how_far_it_is() {
    let mut said = Vec::new();
    let assets = nettai_extract::extract_reporting(&tables(Game::Exe6), &full_set(Game::Exe6), &mut |f| said.push(f)).unwrap();
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
        let a = extract(&tables(game), &full_set(game)).unwrap();
        assert!(a.placeholders.is_empty(), "{}: {:?}", game.id(), a.placeholders);
        if game == Game::Exe6 {
            assert!(a.sound.songs[0x63].is_none(), "stop-music is intentionally empty");
        }
        let mut report = Report::default();
        let b = pack::import_graphics(&packs.join(game.id()), &mut report).unwrap_or_else(|| panic!("{report}"));
        assert!(a.graphics.sprites == b.sprites, "{} sprites", game.id());
        assert!(a.graphics.field == b.field, "{} field", game.id());
        assert!(a.graphics.backgrounds == b.backgrounds, "{} backgrounds", game.id());
        assert!(a.graphics.hud == b.hud, "{} HUD", game.id());
        assert!(a.graphics.custom == b.custom, "{} custom screen", game.id());
    }
}
