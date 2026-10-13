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

/// What a ROM holds, for telling whether a picture is in it: every 32
/// bytes at a word boundary (a tile, four bits a pixel; the dialogue font's
/// rows pack alike; a palette's sixteen colors), of the image and of each
/// LZ77 block in it, as hashes in order. The image's are there once more
/// without each color's top bit, which the hardware ignores and a pack's
/// palette may not keep.
struct Held(Vec<u64>);

fn hash32(bytes: &[u8]) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    h.write(bytes);
    h.finish()
}

/// A GBA BIOS LZ77 block at `at`, if one that decodes whole is there.
fn lz77(rom: &[u8], at: usize) -> Option<Vec<u8>> {
    let header = u32::from_le_bytes(rom.get(at..at + 4)?.try_into().ok()?);
    let size = (header >> 8) as usize;
    if header & 0xFF != 0x10 || size == 0 || size > 0x40000 {
        return None;
    }
    let (mut out, mut o) = (Vec::with_capacity(size), at + 4);
    while out.len() < size {
        let flags = *rom.get(o)?;
        o += 1;
        for bit in 0..8 {
            if out.len() >= size {
                break;
            }
            if flags & (0x80 >> bit) == 0 {
                out.push(*rom.get(o)?);
                o += 1;
                continue;
            }
            let (a, b) = (*rom.get(o)? as usize, *rom.get(o + 1)? as usize);
            o += 2;
            let from = out.len().checked_sub(((a & 15) << 8 | b) + 1)?;
            for k in 0..((a >> 4) + 3).min(size - out.len()) {
                out.push(out[from + k]);
            }
        }
    }
    Some(out)
}

impl Held {
    fn of(rom: &[u8]) -> Held {
        let mut hashes = Vec::new();
        let mut add = |data: &[u8]| hashes.extend((0..data.len().saturating_sub(31)).step_by(4).map(|o| hash32(&data[o..o + 32])));
        add(rom);
        let colors: Vec<u8> = rom.iter().enumerate().map(|(i, &b)| if i % 2 == 1 { b & 0x7F } else { b }).collect();
        add(&colors);
        for at in (0..rom.len()).step_by(4) {
            if let Some(block) = lz77(rom, at) {
                add(&block);
            }
        }
        hashes.sort_unstable();
        hashes.dedup();
        Held(hashes)
    }

    /// Whether the ROM has every tile of `pixels` (a palette index a
    /// pixel, 64 a tile) and each of `palettes`.
    fn has(&self, pixels: &[u8], palettes: &[nettai_assets::Palette]) -> bool {
        let there = |bytes: &[u8]| self.0.binary_search(&hash32(bytes)).is_ok();
        pixels.chunks_exact(64).all(|tile| there(&tile.chunks_exact(2).map(|p| p[0] | p[1] << 4).collect::<Vec<u8>>()))
            && palettes.iter().all(|p| there(&p.iter().flat_map(|c| c.to_le_bytes()).collect::<Vec<u8>>()))
    }
}

/// A picture of a pack as `every_source_subset` holds it to the ROMs:
/// whether it has no pixels at all, whether it is a placeholder, which of
/// the game's four ROMs hold its pixels and the palettes that are its own
/// (a bit a ROM), a digest of its pixels, and one of all that is read with
/// it (its palette, its map, its place).
#[derive(Clone, Copy)]
struct Seen {
    empty: bool,
    placeholder: bool,
    held: u8,
    pixels: u64,
    all: u64,
}

/// A pack's pictures by name, in each of its languages (the worded ones
/// differ): the sprites, the field, the backgrounds, and the HUD's and the
/// custom screen's parts, each with what goes with it. Parts read from
/// different ROMs (a button's states and its worded picture) are apart.
fn pictures(graphics: &nettai_assets::Bundle, held: &[Held]) -> std::collections::BTreeMap<String, Seen> {
    use nettai_assets::Palette;
    use std::fmt::{Debug, Write};
    struct Digest(std::collections::hash_map::DefaultHasher);
    impl Write for Digest {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            std::hash::Hasher::write(&mut self.0, s.as_bytes());
            Ok(())
        }
    }
    let mut out = std::collections::BTreeMap::new();
    let mut add = |name: String, pixels: &[&[u8]], palettes: &[Palette], all: &dyn Debug| {
        let pixels: Vec<u8> = pixels.concat();
        // (A placeholder has the checkerboard: all of it, or the part a
        // missing ROM would give, as EXE4's UNITE button's label beside
        // the window's fill. A placeholder font is all of one color.)
        let checkerboard = |tile: &[u8]| tile.iter().enumerate().all(|(i, &p)| p == if ((i % 8) / 4 + (i / 8) / 4) % 2 == 0 { 1 } else { 2 });
        let placeholder = !pixels.is_empty()
            && (pixels.chunks_exact(64).any(checkerboard) || (name.ends_with("dialogue-font") && pixels.iter().all(|&p| p == 1)));
        let mut digest = Digest(Default::default());
        write!(digest, "{all:?}").unwrap();
        let held = held.iter().enumerate().filter(|(_, h)| h.has(&pixels, palettes)).fold(0, |m, (i, _)| m | 1 << i);
        let seen = Seen { empty: pixels.is_empty(), placeholder, held, pixels: hash32(&pixels), all: std::hash::Hasher::finish(&digest.0) };
        assert!(out.insert(name.clone(), seen).is_none(), "{name} twice");
    };
    for s in &graphics.sprites {
        let tiles: Vec<&[u8]> = s.tilesets.iter().map(|t| &t.pixels[..]).collect();
        add(format!("sprite/{:02x}-{:02x}", s.category, s.index), &tiles, &[], s);
    }
    add("field".into(), &[&graphics.field.tiles.pixels], &[], &graphics.field);
    for (id, b) in graphics.backgrounds.iter().enumerate() {
        if let Some(b) = b {
            add(format!("background/{id:02x}"), &[&b.tiles.pixels], &[], b);
        }
    }
    for lang in ["en", "ja"] {
        let b = graphics.clone().in_language(lang).unwrap();
        let (h, c) = (&b.hud, &b.custom);
        // (`NONE`: a picture drawn in palettes that aren't its own.)
        const NONE: &[Palette] = &[];
        let mut add = |name: &str, pixels: &[&[u8]], palettes: &[Palette], all: &dyn Debug| add(format!("{lang}:{name}"), pixels, palettes, all);
        add("hud/tiles", &[&h.tiles.pixels], NONE, &(&h.tiles, h.first_tile, &h.hp_palettes, &h.gauge_palette, &h.hp_box, &h.gauge_frame, &h.layout.pause));
        add("hud/gauge", &[&h.gauge_tiles.pixels], NONE, &(&h.gauge_tiles, h.gauge_first_tile));
        add("hud/font", &[&h.font.pixels], NONE, &(&h.font, &h.font_chars, &h.texts));
        add("hud/dialogue-font", &[&h.dialogue_font.pixels], NONE, &h.dialogue_font);
        let digits: Vec<&[u8]> = h.enemy_digits.iter().map(|t| &t.pixels[..]).collect();
        add("hud/enemy-digits", &digits, NONE, &(&h.enemy_digits, &h.enemy_palette));
        add("hud/hidden-icon", &[&h.hidden_icon.pixels], NONE, &(&h.hidden_icon, &h.icon_palette));
        for icon in &h.chip_icons {
            add(&format!("hud/chip-icon/{}", icon.key), &[&icon.tiles.pixels], NONE, icon);
        }
        for (i, m) in h.mugshots.iter().enumerate() {
            add(&format!("hud/mugshot/{i:02x}"), &[&m.0.pixels], &[m.1], m);
        }
        for (i, m) in h.mugshot_boxes.iter().enumerate() {
            add(&format!("hud/mugshot-box/{i:02x}"), &[&m.pixels], NONE, m);
        }
        let counts: Vec<&[u8]> = h.counts.iter().chain([&h.count_box]).map(|t| &t.pixels[..]).collect();
        add("hud/counts", &counts, NONE, &(&h.counts, &h.count_box));
        for (i, m) in h.navi_mugshots.iter().enumerate() {
            add(&format!("hud/navi-mugshot/{i:02x}"), &[&m.tiles.pixels], &m.palettes, m);
        }
        add("hud/navi-box", &[&h.navi_box.pixels], NONE, &h.navi_box);
        add("hud/pause", &[&h.pause.pixels], NONE, &h.pause);
        for (i, banner) in h.banners.iter().enumerate() {
            add(&format!("hud/banner/{:02x}", 4 * i), &[&banner.glyphs.pixels], NONE, banner);
        }
        add("hud/banner-digits", &[&h.banner_digits.pixels], NONE, &h.banner_digits);
        add("hud/waiting", &[&h.waiting.pixels], NONE, &(&h.waiting, &h.waiting_palette, &h.banner_palette));
        add("hud/warning", &[&h.warning.pixels], NONE, &(&h.warning, &h.warning_palette));
        add("hud/chatbox", &[&h.chatbox.tiles.pixels, &h.chatbox.arrow.pixels], NONE, &(&h.chatbox, h.layout.chatbox_text, h.layout.chatbox_arrows));
        if let Some(telop) = &h.layout.telop {
            add("hud/telop", &[&telop.glyphs.pixels], NONE, telop);
        }
        add(
            "custom/window",
            &[&c.window_tiles.pixels],
            NONE,
            &(
                (&c.layout, &c.window_tiles, &c.window_maps, &c.window_patches, &c.form_list_maps, &c.form_list_patches),
                (&c.frame_palettes, &c.icon_palette, &c.gray_palette, &c.other_palette, &c.advance_name_colors),
            ),
        );
        add("custom/column-cells", &[&c.column_cells.pixels], NONE, &c.column_cells);
        add("custom/turn-limit", &[&c.turn_limit.pixels], NONE, &c.turn_limit);
        add("custom/name-bar", &[&c.name_bar.pixels], NONE, &c.name_bar);
        add("custom/codes", &[&c.codes.pixels, &c.slot_codes.pixels], NONE, &(&c.codes, &c.slot_codes));
        add("custom/elements", &[&c.elements.pixels], NONE, &(&c.elements, &c.element_colors, &c.element_sprite));
        add("custom/digits", &[&c.digits.pixels], NONE, &c.digits);
        add("custom/empty-icon", &[&c.empty_icon.pixels], NONE, &c.empty_icon);
        add("custom/cursor", &[&c.cursor.pixels], NONE, &(&c.cursor, &c.cursor_palette));
        add("custom/form-list-cursor", &[&c.form_list_cursor.pixels], NONE, &(&c.form_list_cursor, &c.form_list_cursor_palette));
        add("custom/regular", &[&c.regular.pixels], NONE, &c.regular);
        if let Some(emblem) = &c.window_emblem {
            add("custom/window-emblem", &[&emblem.tiles.pixels], NONE, emblem);
        }
        for (name, p) in [("ok", &c.pictures.ok), ("ok-picked", &c.pictures.ok_picked), ("other", &c.pictures.other)] {
            add(&format!("custom/picture/{name}"), &[&p.tiles.pixels], &[p.palette], p);
        }
        for art in &c.chip_art {
            add(&format!("custom/chip-art/{}", art.key), &[&art.picture.tiles.pixels], &[art.picture.palette], art);
        }
        for emblem in &c.emblems {
            add(&format!("custom/emblem/{}", emblem.navi), &[&emblem.tiles.pixels], &[emblem.palette], emblem);
        }
        let versions: Vec<_> = std::iter::once((&c.versioned.base_version, &c.versioned.base)).chain(c.versioned.versions.iter().map(|(v, p)| (v, p))).collect();
        for (version, own) in &versions {
            add(&format!("custom/{version}/form-names"), &[&own.form_names.pixels], NONE, &own.form_names);
        }
        // A button's states (which may have its label), the icons it gives
        // and its picture in the chip window, which has the words.
        let mut button = |at: &str, name: &str, b: &nettai_assets::ButtonPictures, with: &dyn Debug| {
            let states = (b.width, b.height, &b.tiles, &b.sets, b.hidden, &b.cursor, b.uses_digit, b.held_at, &b.place);
            add(&format!("custom/{at}button/{name}"), &[&b.tiles.pixels], NONE, &(states, with));
            add(&format!("custom/{at}button/{name}/icons"), &[&b.icons.pixels], NONE, &(&b.icons, &b.icon_palette, &b.icon_palettes));
            // (The first of its palettes is its picture's own, in the
            // pack's own language.)
            add(&format!("custom/{at}button/{name}/picture"), &[&b.picture.tiles.pixels], &[b.picture.palette], &(&b.picture, b.palettes.get(1..)));
        };
        for (name, b) in &c.buttons {
            button("", name, b, &());
        }
        for (version, own) in versions {
            // (The forms' names' palettes are the version's as its buttons
            // are; the names, above, have the words.)
            for (k, (name, b)) in own.buttons.iter().enumerate() {
                let palettes = (k == 0).then_some(&own.form_name_palettes);
                button(&format!("{version}/"), name, b, &palettes);
            }
        }
    }
    out
}

/// Whether the picture `name` of `game` is in no ROM at all: EXE4's two
/// e-Reader cards' chips' pictures, which are the cards' data and the
/// extractor's own (its exe4/cards.rs).
fn in_no_rom(game: Game, name: &str) -> bool {
    game == Game::Exe4 && ["custom/chip-art/duo", "custom/chip-art/prixpowr"].iter().any(|own| name.ends_with(own))
}

/// The ROMs of `game` (a bit each, in `Game::codes`' order) that have the
/// pixels of the picture `name` without having the picture: `Held` finds
/// its tiles there, and the game's own tables don't give it.
///
/// - A banner is glyphs of the banner font, put together by the banner's
///   record: each release's ROMs have the other's glyphs too (the font
///   was kept whole) and only their own release's records.
/// - EXE6's Japanese ROMs' sheet of the flames HeatMan wears has every
///   tile of the US ROMs' and a frame more: the US sheet, its throw of
///   three frames, isn't there.
fn unreached(game: Game, name: &str) -> u8 {
    const US: u8 = 0b0011;
    const JP: u8 = 0b1100;
    match (game, name) {
        _ if name.starts_with("en:hud/banner/") => JP,
        _ if name.starts_with("ja:hud/banner/") => US,
        (Game::Exe6, "sprite/10-19") => JP,
        _ => 0,
    }
}

/// Optional local regression: put the twelve original images in this
/// directory, named by their game codes (B4WE.gba, B4BE.gba, B4WJ.gba,
/// B4BJ.gba, BRBE.gba, BRKE.gba, BRBJ.gba, BRKJ.gba, BR6E.gba, BR5E.gba,
/// BR6J.gba, BR5J.gba). Every subset of a game's four ROMs, the empty one
/// too, makes a pack that writes, reads back, loads a battle and passes the
/// content audit: a missing ROM's assets are placeholders, never an error.
/// Each pack's banners are the kinds a pack of all four ROMs has
/// (`banner_kinds`).
///
/// And a pack has what its ROMs hold, which the ROMs' own bytes say
/// (`Held`): a picture is a placeholder only where no ROM given holds it
/// (as any pack has it), a real one is in a ROM given, and it is the pack
/// of all four's unless no ROM given holds that one or gives it alone (the
/// two really differ).
/// No copyrighted fixtures are needed for the normal tests.
#[test]
#[ignore = "requires local ROMs via NETTAI_EXTRACT_ROM_DIR"]
fn every_source_subset() {
    let dir = std::path::PathBuf::from(
        std::env::var_os("NETTAI_EXTRACT_ROM_DIR").expect("NETTAI_EXTRACT_ROM_DIR"),
    );
    // (NETTAI_EXTRACT_GAMES=exe6 runs one game's sixteen.)
    let only = std::env::var("NETTAI_EXTRACT_GAMES").ok();
    for game in [Game::Exe4, Game::Exe5, Game::Exe6] {
        if only.as_ref().is_some_and(|g| !g.split(',').any(|g| g == game.id())) {
            continue;
        }
        let bytes = game
            .codes()
            .map(|c| std::fs::read(dir.join(format!("{c}.gba"))).unwrap());
        let held: Vec<Held> = bytes.iter().map(|b| Held::of(b)).collect();
        let mut seen = Vec::new();
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
            seen.push(pictures(&assets.graphics, &held));
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
        let full = &seen[15];
        let mut wrong = Vec::new();
        for (mask, pack) in seen.iter().enumerate() {
            wrong.extend(pack.keys().filter(|name| !full.contains_key(*name)).map(|name| format!("{mask:04b} {name}: no picture of the pack of all four")));
            for (name, whole) in full {
                // The ROMs given, and those of them the game reaches the
                // picture's pixels in as that picture.
                let given = mask as u8;
                let reached = given & !unreached(game, name);
                // (A pack of fewer ROMs has no placeholder for a sheet no
                // name asks for: nothing there is held as a placeholder is.)
                let x = pack.get(name);
                if x.is_none_or(|x| x.placeholder) {
                    // No ROM given holds it, as any pack has it.
                    let real = |t: &usize| seen[*t].get(name).is_some_and(|v| !v.placeholder && !v.empty && v.held & reached != 0);
                    if let Some(of) = (0..16).find(real) {
                        let what = if x.is_some() { "a placeholder" } else { "not there" };
                        wrong.push(format!("{mask:04b} {name}: {what}, though a ROM given holds what the pack of {of:04b} has"));
                    }
                    continue;
                }
                let x = x.unwrap();
                if in_no_rom(game, name) {
                    continue;
                }
                if !x.empty && x.held & given == 0 {
                    wrong.push(format!("{mask:04b} {name}: in no ROM given"));
                } else if x.all != whole.all {
                    // The pack of all four's is another: no ROM given has
                    // its pixels, and none gives it alone.
                    if x.pixels != whole.pixels && !whole.empty && whole.held & reached != 0 {
                        wrong.push(format!("{mask:04b} {name}: not the pack of all four's, whose pixels a ROM given holds"));
                    } else if let Some(rom) = (0..4).find(|&r| mask & (1 << r) != 0 && seen[1 << r].get(name).is_some_and(|v| v.all == whole.all)) {
                        wrong.push(format!("{mask:04b} {name}: not the pack of all four's, which ROM {rom} gives alone"));
                    }
                }
            }
        }
        assert!(wrong.is_empty(), "{}: {} pictures\n{}", game.id(), wrong.len(), wrong.join("\n"));
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
