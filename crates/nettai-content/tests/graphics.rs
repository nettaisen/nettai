//! Graphics packs on a synthetic bundle (no game data): export, import back
//! byte for byte, the run-time timing loader, the derived cache, and what
//! the importer says about files ordinary tools damaged.

use nettai_assets::*;
use nettai_content::image::Indexed;
use nettai_content::report::{Level, Report};
use nettai_content::{pack, timing};
use std::path::{Path, PathBuf};

fn tiles(n: usize, seed: u8) -> Tiles {
    Tiles { pixels: (0..n * Tiles::TILE).map(|i| ((i as u32 * 7 + seed as u32 * 13) % 16) as u8).collect() }
}

fn palette(seed: u16) -> Palette {
    std::array::from_fn(|i| (i as u16 * 0x0421 + seed * 0x0103) & 0x7FFF)
}

fn entry(tile: u16, palette: u8, hflip: bool, vflip: bool) -> MapEntry {
    MapEntry { tile, hflip, vflip, palette }
}

fn part(tile: u16, x: i8, y: i8, w: u8, h: u8, hflip: bool, palette: u8) -> SpritePart {
    SpritePart { tile, x, y, width: w, height: h, hflip, vflip: !hflip && w == h, palette }
}

/// A sprite with overlapping parts, flips, palette offsets, a shared tile
/// set, colours with bit 15 set (as the game's trailing palette data has)
/// and frame flags with a cue bit.
fn sprite(category: u8, index: u8) -> SpriteSheet {
    let mut pals: Vec<Palette> = (0..16).map(palette).collect();
    pals[15][3] |= 0x8000;
    SpriteSheet {
        category,
        index,
        tilesets: vec![tiles(4 + 16 + 2, 1), tiles(4 + 8, 2)],
        palette_sets: vec![pals],
        part_lists: vec![
            // Shadow, body, an arm over the body.
            vec![part(0, -16, -4, 32, 8, false, 0), part(4, -16, -32, 32, 32, false, 0), part(20, -4, -20, 16, 8, true, 1)],
            vec![part(0, -16, -4, 32, 8, false, 0), part(4, -8, -32, 16, 32, true, 2)],
            vec![part(0, -16, -4, 32, 8, false, 0), part(4, -8, -32, 16, 16, false, 0)],
        ],
        animations: vec![
            vec![SpriteFrame { tileset: 0, palette_set: 0, parts: 0, duration: 3, flags: 0x80 }],
            vec![
                SpriteFrame { tileset: 1, palette_set: 0, parts: 1, duration: 2, flags: 0x04 },
                SpriteFrame { tileset: 1, palette_set: 0, parts: 2, duration: 250, flags: 0xC0 },
            ],
        ],
    }
}

fn bundle() -> Bundle {
    let field = Field {
        tiles: tiles(6, 3),
        first_tile: 10,
        palettes: (1..=8).map(palette).collect(),
        first_palette: 1,
        palette_anims: vec![PaletteAnim { slot: 2, frames: vec![(palette(20), 16), (palette(21), 8)], initial_timer: 14 }],
        panels: (0..13 * 6).map(|i| std::array::from_fn(|k| entry(10 + (k % 6) as u16, 1 + (i % 8) as u8, k % 2 == 0, false))).collect(),
        front_edges: [std::array::from_fn(|k| entry(11, 1, false, k == 0)), std::array::from_fn(|_| entry(12, 5, true, true))],
        highlights: [std::array::from_fn(|_| entry(13, 3, false, false)), std::array::from_fn(|_| entry(14, 7, false, false))],
    };
    let background = Background {
        tiles: tiles(5, 4),
        first_tile: 1,
        map: (0..16).map(|i| entry(1 + (i % 5) as u16, (i % 3) as u8, i % 2 == 1, i % 4 == 0)).collect(),
        map_width: 4,
        map_height: 4,
        palette: Some(palette(9)),
        scroll: (-8, 4),
        anims: vec![
            GfxAnim {
                target: AnimTarget::Tiles { first: 2, count: 3 },
                frames: vec![
                    GfxAnimFrame { tiles: tiles(3, 5), palettes: vec![], delay: 64 },
                    GfxAnimFrame { tiles: tiles(3, 6), palettes: vec![], delay: 1 },
                ],
                repeat_from: Some(1),
            },
            GfxAnim {
                target: AnimTarget::Palettes { first: 0, count: 1 },
                frames: vec![GfxAnimFrame { tiles: Tiles::default(), palettes: vec![palette(30)], delay: 12 }],
                repeat_from: None,
            },
        ],
    };
    let hud = Hud {
        tiles: tiles(6, 7),
        first_tile: 0x1A0,
        gauge_tiles: tiles(3, 8),
        gauge_first_tile: 0x222,
        hp_palettes: [palette(40), palette(41), palette(42)],
        gauge_palette: palette(40),
        hp_box: (0..12).map(|i| entry(0x1A0 + i, 13, false, false)).collect(),
        gauge_frame: (0..36).map(|i| entry(0x222 + i % 3, 9, i % 2 == 0, false)).collect(),
        font: tiles(8, 9),
        font_chars: [" ", "0", "A", "[EX]"].map(String::from).to_vec(),
        enemy_digits: [tiles(20, 10), tiles(20, 11), tiles(20, 12)],
        enemy_palette: palette(43),
        chip_icons: vec![
            ChipIcon { key: "cannon".into(), tiles: tiles(4, 13) },
            ChipIcon { key: "chip-001".into(), tiles: Tiles::default() },
            ChipIcon { key: "sword".into(), tiles: tiles(4, 14) },
        ],
        hidden_icon: tiles(4, 15),
        icon_palette: palette(44),
        mugshots: vec![(tiles(8, 16), palette(45)), (tiles(8, 17), palette(46))],
        counts: (0..11).map(|i| tiles(4, 18 + i)).collect(),
        count_box: tiles(4, 29),
        navi_mugshots: vec![NaviMugshot { tiles: tiles(8, 30), palettes: [palette(47), palette(48)] }],
        navi_box: tiles(4, 31),
        pause: tiles(10, 32),
        texts: vec![vec![1, 2, 3], vec![]],
        banners: vec![
            BannerLayout { x: 52, y: 64, kind: 1, glyphs: tiles(40, 30), number_at: Some((104, 64)) },
            BannerLayout { x: 0, y: 32, kind: 3, glyphs: Tiles::default(), number_at: None },
        ],
        banner_digits: tiles(22, 31),
        banner_palette: palette(47),
        waiting: tiles(16, 32),
        waiting_palette: palette(47),
        warning: tiles(8, 33),
        warning_palette: palette(49),
    };
    Bundle { sprites: vec![sprite(0, 1), sprite(0x14, 0x3A)], field, backgrounds: vec![Some(background), None, None], hud, custom: custom() }
}

/// A version's own pictures.
fn own(seed: u8) -> VersionPictures {
    VersionPictures {
        beast_out: Picture { tiles: tiles(42, seed), palette: palette(seed as u16) },
        beast_out_palettes: vec![palette(seed as u16), palette(seed as u16 + 1)],
        beast_buttons: tiles(32, seed + 2),
        emblems: tiles(8, seed + 3),
        cross_names: tiles(36, seed + 4),
        cross_palettes: vec![palette(seed as u16 + 5), palette(seed as u16 + 6)],
    }
}

/// A custom screen's graphics: every block and table with some content.
fn custom() -> CustomScreen {
    let map = |seed: u16| (0..300u16).map(|i| entry((i * 3 + seed) % 0x1A0, (i % 4) as u8 + 9, i % 7 == 0, i % 11 == 0)).collect();
    let picture = |seed: u8| Picture { tiles: tiles(42, seed), palette: palette(seed as u16) };
    let patch = |x, y, w, h, by_column| MapPatch { x, y, width: w, height: h, palette: 9, by_column };
    CustomScreen {
        window_tiles: tiles(0x87, 40),
        column_cells: tiles(4, 41),
        turn_limit: tiles(14, 42),
        name_bar: tiles(4, 43),
        window_maps: vec![map(0), map(1)],
        window_patches: PatchList { first_tile: 0x9B, patches: vec![patch(2, 1, 8, 2, true), patch(2, 3, 7, 6, false)] },
        cross_maps: vec![map(2), map(3), map(4)],
        cross_patches: PatchList { first_tile: 0xE1, patches: vec![patch(1, 13, 2, 2, false)] },
        frame_palettes: (50..54).map(palette).collect(),
        icon_palette: palette(55),
        grey_palette: palette(56),
        other_palette: palette(57),
        chip_art: vec![
            ChipArt { key: "cannon".into(), picture: picture(60) },
            ChipArt { key: "no-picture".into(), picture: Picture::default() },
        ],
        pictures: SlotPictures {
            ok: picture(61),
            ok_picked: picture(62),
            redeal: picture(65),
            scrap: picture(66),
            other: picture(67),
        },
        codes: tiles(56, 70),
        elements: tiles(8, 71),
        element_colours: vec![[1, 2, 3, 4, 5, 6], [0x7FFF, 0, 0x1F, 0x3E0, 0x7C00, 0x2108]],
        digits: tiles(22, 72),
        slot_codes: tiles(56, 73),
        empty_icon: tiles(4, 74),
        redeal_buttons: tiles(36, 76),
        scrap_buttons: tiles(48, 77),
        versioned: Versioned { base: own(63), versions: vec![("gregar".into(), own(90))] },
        cursor: tiles(2, 78),
        cross_cursor: tiles(4, 84),
        cross_cursor_palette: palette(85),
        emblem_palettes: vec![palette(80), palette(81)],
        emblem_of: vec![0, 1, 1],
        emblem_palette_of: vec![1, 0, 0],
        regular: tiles(32, 82),
        advance_name_colours: vec![[0, 0x7FFF, 0x14A5, 0], [0, 0x43F0, 0x14A5, 0]],
    }
}

/// A fresh directory for one test.
fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bn6-content-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write_pack(dir: &Path, b: &Bundle) {
    let mut files = vec![pack::manifest("test", Some(b), false)];
    files.extend(pack::export_graphics(b, &nettai_content::names::AssetNames::default()));
    pack::write_files(dir, &files).unwrap();
}

fn import(dir: &Path) -> (Option<Bundle>, Report) {
    let mut r = Report::default();
    let b = pack::import_graphics(dir, &mut r);
    (b, r)
}

#[test]
fn graphics_read_back_exactly() {
    let dir = temp("roundtrip");
    let b = bundle();
    write_pack(&dir, &b);
    let (back, report) = import(&dir);
    assert!(!report.has_errors() && report.count(Level::Warning) == 0, "{report}");
    assert_eq!(back.unwrap(), b);
    // The background map is a Tiled map; the sprite's timing a file of its own.
    assert!(dir.join("graphics/backgrounds/background-00/map.tmj").is_file());
    assert!(!dir.join("graphics/backgrounds/background-01").exists());
    let anims = std::fs::read_to_string(dir.join("graphics/sprites/sprite-00-01/animations.json")).unwrap();
    assert!(anims.contains(r#"{"ticks":250,"flags":["last","loop"],"tileset":1,"layout":2}"#), "{anims}");
    assert!(anims.contains(r#""flags":[4]"#));
}

/// Assets are written under the names given them; each file holds its
/// number, so the names are free.
#[test]
fn named_assets_read_back_by_their_numbers() {
    let dir = temp("named");
    let b = bundle();
    let mut names = nettai_content::names::AssetNames::default();
    for (i, s) in b.sprites.iter().enumerate() {
        names.sprites.insert((s.category, s.index), format!("sprite-named-{i}"));
    }
    names.backgrounds.insert(0, "clouds".into());
    names.mugshots.insert(0, "megaman".into());
    let mut files = vec![pack::manifest("test", Some(&b), false)];
    files.extend(pack::export_graphics(&b, &names));
    pack::write_files(&dir, &files).unwrap();
    assert!(dir.join("graphics/sprites/sprite-named-0/sprite.json").is_file());
    assert!(dir.join("graphics/backgrounds/clouds/background.json").is_file());
    assert!(dir.join("graphics/hud/mugshots/megaman.png").is_file());
    let (back, report) = import(&dir);
    assert!(!report.has_errors(), "{report}");
    assert_eq!(back.unwrap(), b);
}

/// The asset index lists every asset under the name it is written under
/// (placeholders for the rest), and reads back.
#[test]
fn the_asset_index_lists_every_asset_by_name() {
    use nettai_content_api::{AssetKind, AssetNames, SpriteId};
    let dir = temp("index");
    let b = bundle();
    let mut names = nettai_content::names::AssetNames::default();
    let first = &b.sprites[0];
    names.sprites.insert((first.category, first.index), "bomb".into());
    names.backgrounds.insert(0, "clouds".into());
    names.songs.insert(0x63, "no-music".into());
    names.songs.insert(1, "throw".into());
    // A banner the HUD doesn't draw is named all the same.
    names.banners.insert(0xA0, "heatman-win".into());
    let index = names.index(&b, &[0, 1, 2].into());
    assert_eq!(index.sprites["bomb"], SpriteId { category: first.category, index: first.index });
    assert_eq!(index.sprites.len(), b.sprites.len());
    let sounds: Vec<(&str, u16)> = index.sounds.iter().map(|(k, &v)| (k.as_str(), v)).collect();
    assert_eq!(sounds, [("no-music", 0x63), ("sound-000", 0), ("sound-002", 2), ("throw", 1)]);
    assert_eq!(index.banners.get("heatman-win"), Some(&0xA0));
    assert_eq!(index.backgrounds.get("clouds"), Some(&0));
    assert_eq!(index.banners.len(), b.hud.banners.len() + 1);
    assert_eq!(index.mugshots.len(), b.hud.mugshots.len() + b.hud.navi_mugshots.len());
    assert!(AssetNames::is_placeholder(AssetKind::Sound, "sound-002"));
    pack::write_files(&dir, &vec![nettai_content::names::index_file(&index)]).unwrap();
    let mut r = Report::default();
    assert_eq!(nettai_content::names::read_index(&dir, &mut r), Some(index), "{r}");
    // Without an index a pack names no assets.
    let mut r = Report::default();
    assert_eq!(nettai_content::names::read_index(&temp("no-index"), &mut r), Some(AssetNames::default()));
    assert_eq!(r.count(Level::Note), 1);
}

#[test]
fn timing_loads_without_images() {
    let dir = temp("timing");
    let b = bundle();
    write_pack(&dir, &b);
    for s in &b.sprites {
        std::fs::remove_file(dir.join(format!("graphics/sprites/sprite-{:02x}-{:02x}/atlas.png", s.category, s.index))).unwrap();
    }
    let mut r = Report::default();
    let t = timing::load(&dir, &mut r).unwrap();
    assert_eq!(t.animation(0x14, 0x3A, 1).unwrap().iter().map(|f| (f.ticks, f.flags)).collect::<Vec<_>>(), vec![(2, 0x04), (250, 0xC0)]);
    assert!(nettai_content::verify::compare_timing(&t, &b).is_empty());
}

#[test]
fn loading_reads_the_files_as_they_are() {
    let dir = temp("load");
    let b = bundle();
    write_pack(&dir, &b);
    let (first, _) = pack::load_graphics(&dir).unwrap();
    assert_eq!(first, b);
    // Recolour one sprite colour: the next load has it.
    let atlas = dir.join("graphics/sprites/sprite-00-01/atlas.png");
    let mut img = Indexed::load(&atlas).unwrap();
    img.palette[5] = [255, 255, 255];
    img.save(&atlas).unwrap();
    let (second, _) = pack::load_graphics(&dir).unwrap();
    assert_eq!(second.sprites[0].palette_sets[0][0][5], 0x7FFF);
}

#[test]
fn edits_in_an_image_editor_come_through() {
    let dir = temp("edit");
    let b = bundle();
    write_pack(&dir, &b);
    // Paint one pixel of the first part (tile 0) with colour 9 of the
    // part's palette row, as an editor would.
    let atlas = dir.join("graphics/sprites/sprite-00-01/atlas.png");
    let mut img = Indexed::load(&atlas).unwrap();
    img.set(3, 2, 9);
    img.save(&atlas).unwrap();
    let (back, report) = import(&dir);
    assert!(!report.has_errors(), "{report}");
    let t = &back.unwrap().sprites[0].tilesets[0];
    assert_eq!(t.get(0).unwrap()[2 * 8 + 3], 9);
}

#[test]
fn damaged_palettes_are_refused_with_a_reason() {
    let dir = temp("damage");
    let b = bundle();
    write_pack(&dir, &b);
    let atlas = dir.join("graphics/sprites/sprite-00-01/atlas.png");
    let good = std::fs::read(&atlas).unwrap();

    // Re-sorted palette (pixels remapped so the picture looks the same).
    let mut img = Indexed::from_png(&good).unwrap();
    img.palette.swap(1, 2);
    for p in img.pixels.iter_mut() {
        *p = match *p {
            1 => 2,
            2 => 1,
            x => x,
        };
    }
    img.save(&atlas).unwrap();
    let (back, r) = import(&dir);
    assert!(back.is_none() && r.issues.iter().any(|i| i.message.contains("re-sorted")), "{r}");

    // Unused colours dropped.
    let mut img = Indexed::from_png(&good).unwrap();
    img.palette.truncate(40);
    for p in img.pixels.iter_mut() {
        *p %= 40;
    }
    img.save(&atlas).unwrap();
    let (_, r) = import(&dir);
    assert!(r.issues.iter().any(|i| i.message.contains("dropped entries")), "{r}");

    // Saved as RGB.
    let rgb = {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, 2, 2);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&[0; 12]).unwrap();
        out
    };
    std::fs::write(&atlas, rgb).unwrap();
    let (_, r) = import(&dir);
    assert!(r.issues.iter().any(|i| i.message.contains("indexed")), "{r}");

    // A colour off the GBA's 5-bit grid: accepted, rounded, warned.
    let mut img = Indexed::from_png(&good).unwrap();
    img.palette[7] = [255, 1, 128];
    img.save(&atlas).unwrap();
    let (back, r) = import(&dir);
    assert!(back.is_some() && r.issues.iter().any(|i| i.level == Level::Warning && i.message.contains("rounded")), "{r}");
}

#[test]
fn an_aseprite_view_edits_whole_frames() {
    use nettai_content::aseprite::{self, AseFile, CelContent};
    let s = sprite(0, 1);
    let bytes = aseprite::export(&s);
    let mut r = Report::default();
    let back = aseprite::import(&bytes, &s, "view", &mut r).unwrap();
    assert!(r.issues.is_empty(), "{r}");
    assert_eq!(back, s);

    let ase = AseFile::from_bytes(&bytes).unwrap();
    assert_eq!((ase.frames.len(), ase.layers.len(), ase.tags.len()), (3, 3, 2));
    assert_eq!(ase.frames[2].duration_ms, 4186, "250 ticks");
    // Frames 1 and 2 draw the same shadow from the same tiles: a linked cel.
    assert!(ase.frames[2].cels.iter().any(|c| c.layer == 0 && c.content == CelContent::Linked(1)));

    // Paint the shadow in frame 1: the shared tiles change for both frames.
    let mut edited = ase.clone();
    let cel = edited.frames[1].cels.iter_mut().find(|c| c.layer == 0).unwrap();
    let CelContent::Image { pixels, .. } = &mut cel.content else { panic!() };
    pixels[0] = 0x0B;
    // Frame 2 lasts 110 ms now, and animation 1 plays once (the tag's
    // name says so; its repeat count is only the editor's preview).
    assert_eq!(ase.tags[1].name, "anim 01 loop");
    edited.frames[2].duration_ms = 110;
    edited.tags[1].name = "anim 01".into();
    edited.tags[1].repeat = 1;
    let mut r = Report::default();
    let back = aseprite::import(&edited.to_bytes(), &s, "view", &mut r).unwrap();
    assert!(!r.has_errors() && r.count(Level::Warning) == 0, "{r}");
    assert_eq!(back.tilesets[1].get(0).unwrap()[0], 0x0B);
    assert_eq!((back.animations[1][1].duration, back.animations[1][1].flags), (7, 0x80));
    assert!(r.issues.iter().any(|i| i.message.contains("110 ms is 7 ticks")));
    // An editor that doesn't keep repeat counts (repeat 0 everywhere).
    let mut old = ase.clone();
    old.tags.iter_mut().for_each(|t| t.repeat = 0);
    let mut r = Report::default();
    let back = aseprite::import(&old.to_bytes(), &s, "view", &mut r).unwrap();
    assert_eq!(back.animations[0][0].flags, 0x80, "still plays once");
    assert!(r.issues.iter().any(|i| i.message.contains("its name decides")), "{r}");

    // Frames 1 and 2 also share the body's tiles in different shapes (not
    // linked): editing only one of them is refused.
    let mut edited = ase.clone();
    let cel = edited.frames[2].cels.iter_mut().find(|c| c.layer == 1).unwrap();
    let CelContent::Image { pixels, .. } = &mut cel.content else { panic!("frame 2's body isn't linked") };
    for p in pixels.iter_mut() {
        *p = 0x01;
    }
    let mut r = Report::default();
    assert!(aseprite::import(&edited.to_bytes(), &s, "view", &mut r).is_none());
    assert!(r.issues.iter().any(|i| i.message.contains("several frames share")), "{r}");
}

#[test]
fn tiled_maps_refuse_what_the_gba_cannot_do() {
    let dir = temp("tiled");
    write_pack(&dir, &bundle());
    let map = dir.join("graphics/backgrounds/background-00/map.tmj");
    let text = std::fs::read_to_string(&map).unwrap();
    // Rotate the first cell (Tiled's diagonal flip bit).
    let first = text.split("\"data\": [").nth(1).unwrap().trim_start().split(',').next().unwrap().to_string();
    let rotated = (first.parse::<u32>().unwrap() | 0x2000_0000).to_string();
    std::fs::write(&map, text.replacen(&format!("[\n        {first},"), &format!("[\n        {rotated},"), 1)).unwrap();
    let (_, r) = import(&dir);
    assert!(r.issues.iter().any(|i| i.message.contains("rotated")), "{r}");
}
