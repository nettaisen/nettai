use nettai_content::{pack, report::Report};
use nettai_extract::{Game, RomSet, extract};

#[test]
fn empty_sets_make_loadable_packs_and_language_variants() {
    for game in [Game::Exe5, Game::Exe6] {
        let assets = extract(game, &RomSet::default()).unwrap();
        assert_eq!(assets.missing_roms.len(), 4);
        assert!(!assets.placeholders.is_empty());
        assert!(!assets.graphics.sprites.is_empty());
        assets
            .graphics
            .clone()
            .in_language("ja")
            .unwrap()
            .in_language("en")
            .unwrap();
        assets.sound.validate().unwrap();
        let dir = tempfile::tempdir().unwrap();
        assets.write(dir.path()).unwrap();
        let mut report = Report::default();
        let (sound, _) = pack::import_sound_versions(dir.path(), &mut report).unwrap();
        sound.validate().unwrap();
        let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let (_, report) = pack::load_battle(&content, dir.path()).unwrap_or_else(|e| panic!("{e}"));
        assert!(!report.has_errors(), "{report}");
        // Writing a second time cannot leave stale graphics from an earlier pack.
        assert!(
            assets
                .write(dir.path())
                .unwrap_err()
                .to_string()
                .contains("not empty")
        );
    }
}

/// EXE4's pack from no ROMs: placeholders under compat's names, written and
/// read back. (Its content has no rules yet, so no battle loads with it.)
#[test]
fn an_empty_set_makes_an_exe4_pack() {
    let assets = extract(Game::Exe4, &RomSet::default()).unwrap();
    assert_eq!(assets.missing_roms.len(), 4);
    assert!(assets.placeholders.iter().any(|p| p == "sprite/megaman"));
    assert!(assets.warnings.iter().any(|w| w.contains("extracted yet")));
    assets.graphics.clone().in_language("ja").unwrap().in_language("en").unwrap();
    let dir = tempfile::tempdir().unwrap();
    assets.write(dir.path()).unwrap();
}

#[test]
fn rejects_unknown_truncated_and_duplicate_roms() {
    let mut roms = RomSet::default();
    assert!(roms.insert(Vec::new()).is_err());
    let mut short = vec![0; 0xb0];
    short[0xac..0xb0].copy_from_slice(b"BR6E");
    assert!(
        roms.insert(short)
            .unwrap_err()
            .to_string()
            .contains("8 MiB")
    );
    let mut bytes = vec![0; 8 * 1024 * 1024];
    bytes[0xac..0xb0].copy_from_slice(b"BR6E");
    roms.insert(bytes.clone()).unwrap();
    assert!(roms.contains("BR6E"));
    assert!(
        roms.insert(bytes)
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
    // Supplied but corrupt data is an error, not a successful placeholder pack.
    assert!(extract(Game::Exe6, &roms).is_err());
}

/// Optional local regression: put the eight original images in this directory,
/// named BRBE.gba, BRKE.gba, BRBJ.gba, BRKJ.gba, BR6E.gba, BR5E.gba, BR6J.gba,
/// BR5J.gba. No copyrighted fixtures are needed for the normal tests.
#[test]
#[ignore = "requires local ROMs via NETTAI_EXTRACT_ROM_DIR"]
fn every_source_subset() {
    let dir = std::path::PathBuf::from(
        std::env::var_os("NETTAI_EXTRACT_ROM_DIR").expect("NETTAI_EXTRACT_ROM_DIR"),
    );
    for game in [Game::Exe5, Game::Exe6] {
        let bytes = game
            .codes()
            .map(|c| std::fs::read(dir.join(format!("{c}.gba"))).unwrap());
        for mask in 0u32..16 {
            let mut roms = RomSet::default();
            for (i, b) in bytes.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    roms.insert(b.clone()).unwrap();
                }
            }
            eprintln!("{} sources {mask:04b}", game.id());
            let assets =
                extract(game, &roms).unwrap_or_else(|e| panic!("{} {mask:04b}: {e}", game.id()));
            assert_eq!(assets.missing_roms.len(), 4 - mask.count_ones() as usize);
            assets.graphics.clone().in_language("ja").unwrap();
            let out = tempfile::tempdir().unwrap();
            assets
                .write(out.path())
                .unwrap_or_else(|e| panic!("{} {mask:04b}: {e}", game.id()));
            let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
            pack::load_battle(&content, out.path())
                .unwrap_or_else(|e| panic!("{} {mask:04b}: {e}", game.id()));
        }
    }
}

#[test]
#[ignore = "requires original ROMs and existing reference packs"]
fn full_sets_preserve_reference_graphics() {
    let dir = std::path::PathBuf::from(
        std::env::var_os("NETTAI_EXTRACT_ROM_DIR").expect("NETTAI_EXTRACT_ROM_DIR"),
    );
    let packs = std::path::PathBuf::from(
        std::env::var_os("NETTAI_EXTRACT_REFERENCE_PACKS").expect("NETTAI_EXTRACT_REFERENCE_PACKS"),
    );
    for game in [Game::Exe5, Game::Exe6] {
        let mut roms = RomSet::default();
        for code in game.codes() {
            roms.insert(std::fs::read(dir.join(format!("{code}.gba"))).unwrap())
                .unwrap();
        }
        let a = extract(game, &roms).unwrap();
        assert!(
            a.placeholders.is_empty(),
            "{}: {:?}",
            game.id(),
            a.placeholders
        );
        if game == Game::Exe6 {
            assert!(
                a.sound.songs[0x63].is_none(),
                "stop-music is intentionally empty"
            );
        }
        let mut report = Report::default();
        let b = pack::import_graphics(&packs.join(game.id()), &mut report)
            .unwrap_or_else(|| panic!("{report}"));
        assert!(a.graphics.sprites == b.sprites, "{} sprites", game.id());
        assert!(a.graphics.field == b.field, "{} field", game.id());
        assert!(
            a.graphics.backgrounds == b.backgrounds,
            "{} backgrounds",
            game.id()
        );
        assert!(a.graphics.hud == b.hud, "{} HUD", game.id());
        assert!(a.graphics.custom == b.custom, "{} custom screen", game.id());
    }
}
