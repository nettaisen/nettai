use nettai_content::{pack, report::Report};
use nettai_extract::{Game, RomSet, extract};
use std::path::Path;
use std::sync::Arc;

/// What the content audit (nettai-tool's `--audit-content`) finds missing
/// of `game`'s content drawn and heard with the pack at `dir`: every
/// lookup a frontend makes of everything the content defines, in each
/// language it has strings in. A pack of placeholders has to answer them
/// all, as a pack of every ROM does.
fn audit(game: Game, dir: &Path) -> Vec<String> {
    use nettai_frontend::game::{Found, Game as Loaded};
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    // (A packs directory with no pack in it: the pack is the one given.)
    let none = tempfile::tempdir().unwrap();
    let found = Found::find(none.path(), &[dir.to_path_buf()]).unwrap_or_else(|e| panic!("{e}"));
    let loaded = Loaded::load(&found, Some(&content), game.id()).unwrap_or_else(|e| panic!("{e}"));
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

/// The banners that aren't plain, by game: each one's number / 4 and its
/// kind (`BannerLayout::kind`: 1 with a number, 2 held until removed, 3 a
/// telop's, 4 the judge's), as the ROMs' records state them
/// (`every_source_subset` holds a pack of all four ROMs to this list). How
/// a banner behaves is the game's, whichever ROMs a pack is built from:
/// only its picture may be a placeholder.
fn banner_kinds(game: Game) -> Vec<(usize, u8)> {
    let mut kinds = vec![(0x0C / 4, 1), (0x20 / 4, 1), (0x24 / 4, 2), (0x28 / 4, 4), (0x30 / 4, 1), (0x34 / 4, 2)];
    match game {
        Game::Exe4 => {}
        Game::Exe5 => kinds.extend([(0x3C / 4, 1), (0x4C / 4, 3), (0x50 / 4, 3), (0x90 / 4, 1)]),
        Game::Exe6 => kinds.extend([(0x3C / 4, 1), (0x4C / 4, 3), (0x50 / 4, 3), (0x60 / 4, 1)]),
    }
    kinds
}

/// Hold a pack's banners to `banner_kinds`, in its own words and in
/// Japanese: the kinds, a number's place for the banners with one and for
/// no other, and no glyphs for a telop's.
fn check_banner_kinds(game: Game, graphics: &nettai_assets::Bundle, what: &str) {
    for lang in ["en", "ja"] {
        let hud = graphics.clone().in_language(lang).unwrap().hud;
        let stated: Vec<(usize, u8)> = hud.banners.iter().enumerate().filter(|(_, b)| b.kind != 0).map(|(id, b)| (id, b.kind)).collect();
        assert_eq!(stated, banner_kinds(game), "{} {what} ({lang}): the banners' kinds", game.id());
        for (id, b) in hud.banners.iter().enumerate() {
            assert_eq!(b.number_at.is_some(), b.kind == 1, "{} {what} ({lang}): banner {:#04x}'s number", game.id(), 4 * id);
            assert_eq!(b.glyphs.is_empty(), b.kind == 3, "{} {what} ({lang}): banner {:#04x}'s glyphs", game.id(), 4 * id);
        }
    }
}

/// EXE4's pack from no ROMs: placeholders under compat's names, in the
/// game's own shape (its screen's own emblem and cursor colors, where the
/// other games' take a navi's), written and read back.
#[test]
fn an_empty_set_makes_an_exe4_pack() {
    let assets = extract(Game::Exe4, &RomSet::default()).unwrap();
    assert_eq!(assets.missing_roms.len(), 4);
    assert!(assets.placeholders.iter().any(|p| p == "sprite/megaman"));
    assert!(assets.graphics.custom.buttons.iter().any(|(n, b)| n == "soul" && b.place.is_some()));
    assert!(assets.graphics.custom.window_emblem.is_some() && assets.graphics.custom.cursor_palette.is_some());
    assets.graphics.clone().in_language("ja").unwrap().in_language("en").unwrap();
    let dir = tempfile::tempdir().unwrap();
    assets.write(dir.path()).unwrap();
}

/// A pack of no ROMs answers every lookup the content audit makes, in each
/// game: a placeholder stands wherever a frontend looks (EXE4's fonts with
/// every character a ROM's have). And its banners are the kinds the ROMs'
/// are: a placeholder is a picture, not another behavior.
#[test]
fn empty_sets_pass_the_content_audit() {
    for game in [Game::Exe4, Game::Exe5, Game::Exe6] {
        let assets = extract(game, &RomSet::default()).unwrap();
        check_banner_kinds(game, &assets.graphics, "from no ROMs");
        let dir = tempfile::tempdir().unwrap();
        assets.write(dir.path()).unwrap();
        let problems = audit(game, dir.path());
        assert!(problems.is_empty(), "{}: {problems:#?}", game.id());
    }
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

/// Optional local regression: put the twelve original images in this
/// directory, named by their game codes (B4WE.gba, B4BE.gba, B4WJ.gba,
/// B4BJ.gba, BRBE.gba, BRKE.gba, BRBJ.gba, BRKJ.gba, BR6E.gba, BR5E.gba,
/// BR6J.gba, BR5J.gba). Every subset of a game's four ROMs, the empty one
/// too, makes a pack that writes, reads back, loads a battle and passes the
/// content audit: a missing ROM's assets are placeholders, never an error.
/// Each pack's banners are the kinds a pack of all four ROMs has
/// (`banner_kinds`).
/// No copyrighted fixtures are needed for the normal tests.
#[test]
#[ignore = "requires local ROMs via NETTAI_EXTRACT_ROM_DIR"]
fn every_source_subset() {
    let dir = std::path::PathBuf::from(
        std::env::var_os("NETTAI_EXTRACT_ROM_DIR").expect("NETTAI_EXTRACT_ROM_DIR"),
    );
    for game in [Game::Exe4, Game::Exe5, Game::Exe6] {
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
            check_banner_kinds(game, &assets.graphics, &format!("{mask:04b}"));
            let out = tempfile::tempdir().unwrap();
            assets
                .write(out.path())
                .unwrap_or_else(|e| panic!("{} {mask:04b}: {e}", game.id()));
            let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
            pack::load_battle(&content, out.path())
                .unwrap_or_else(|e| panic!("{} {mask:04b}: {e}", game.id()));
            let problems = audit(game, out.path());
            assert!(problems.is_empty(), "{} {mask:04b}: {problems:#?}", game.id());
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
