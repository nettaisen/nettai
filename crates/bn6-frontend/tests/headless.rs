//! Headless rendering of a live battle with a small synthetic asset set
//! (no game data needed): the field, the navis and their shadows land
//! where the original draws them, and PNGs come out.

use bn6_assets::{Bundle, Field, MapEntry, SpriteFrame, SpritePart, SpriteSheet, Tiles};
use bn6_battle::content::testing;
use bn6_frontend::driver::{LivePlayer, live_setup};
use bn6_frontend::{Renderer, Session, headless};
use std::collections::BTreeSet;

const RED: u16 = 0x001F;
const BLUE: u16 = 0x7C00;
const SHADOW: u16 = 0x0421;
const BODY: u16 = 0x03E0;

fn solid(n: usize, index: u8) -> Tiles {
    Tiles { pixels: vec![index; n * Tiles::TILE] }
}

fn assets() -> Bundle {
    // MegaMan's battle sprite: every animation is one looping frame of a
    // 32x16 shadow and a 16x32 body standing on the anchor.
    let mut tiles = solid(8, 1);
    tiles.pixels.extend(solid(8, 2).pixels);
    let part = |tile, x, y, width, height| SpritePart { tile, x, y, width, height, hflip: false, vflip: false, palette: 0 };
    let mut palette = [0u16; 16];
    palette[1] = SHADOW;
    palette[2] = BODY;
    let frame = SpriteFrame { tileset: 0, palette_set: 0, parts: 0, duration: 60, flags: 0xC0 };
    let megaman = SpriteSheet {
        category: 0,
        index: 0,
        tilesets: vec![tiles],
        palette_sets: vec![vec![palette]],
        part_lists: vec![vec![part(0, -16, -8, 32, 16), part(8, -8, -32, 16, 32)]],
        animations: vec![vec![frame]; 32],
    };
    // Panels: one solid tile, palette 1 for the left side's, 5 for the
    // right side's.
    let block = |owner: usize| [MapEntry { tile: 0xA3, hflip: false, vflip: false, palette: 1 + 4 * owner as u8 }; 15];
    let mut palettes = vec![[0u16; 16]; 8];
    palettes[0][1] = RED;
    palettes[4][1] = BLUE;
    let field = Field {
        tiles: solid(1, 1),
        first_tile: 0xA3,
        palettes,
        first_palette: 1,
        panels: (0..13 * 6).map(|i| block((i / 3) % 2)).collect(),
        front_edges: [[MapEntry::default(); 5]; 2],
        highlights: [[MapEntry::default(); 15]; 2],
        ..Field::default()
    };
    Bundle { sprites: vec![megaman], field, ..Bundle::default() }
}

fn pixel(png: &std::path::Path, x: usize, y: usize) -> [u8; 3] {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(png).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (240, 160));
    let o = (y * 240 + x) * 3;
    [buf[o], buf[o + 1], buf[o + 2]]
}

fn rgb(c: u16) -> [u8; 3] {
    let v = bn6_frontend::compose::to_rgb(c);
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

#[test]
fn renders_a_live_battle_to_png() {
    let assets = assets();
    let mut renderer = Renderer::new(&assets);
    // A live battle on the engine's hand-authored test content.
    let content = testing::content();
    let settings = bn6_battle::BattleSettings::on(&content, content.stage_by_key(testing::LINK_BATTLE));
    // (Folders of GunDelS3 N: the test content has it.)
    let setup = live_setup(&content, settings, &[("gundels3", 13)], 1);
    let session = Session::new(Box::new(LivePlayer::new(setup, content.clone())));
    let out = std::env::temp_dir().join(format!("bn6-frontend-test-{}", std::process::id()));
    let wanted: BTreeSet<u32> = [1, 100].into_iter().collect();
    let mut log = |s: &str| panic!("{s}");
    let written = headless::render_frames(&mut renderer, vec![session], &wanted, &out, 1, &mut log).unwrap();
    assert_eq!(written, vec![1, 100]);

    // Tick 1: the screen starts fully faded to white.
    assert_eq!(pixel(&out.join("frame_00001.png"), 120, 80), [255, 255, 255]);

    // Tick 100: the intro is over and the custom screen is up (nobody
    // pressed OK), so the field and the navis are 15 pixels lower. Panels:
    // 40x24 blocks from (0, 87).
    let f = out.join("frame_00100.png");
    assert_eq!(pixel(&f, 20, 115), rgb(RED), "left panels are the left side's");
    assert_eq!(pixel(&f, 220, 115), rgb(BLUE), "right panels are the right side's");
    // The left navi stands on panel (2, 2): anchor (60, 123).
    assert_eq!(pixel(&f, 60, 105), rgb(BODY), "body above the anchor");
    assert_eq!(pixel(&f, 45, 117), rgb(SHADOW), "shadow on the ground");
    // The right navi faces left on panel (5, 2): anchor (180, 123).
    assert_eq!(pixel(&f, 180, 105), rgb(BODY));
    // Above the field is the backdrop (no background in this asset set).
    assert_eq!(pixel(&f, 120, 40), [0, 0, 0]);
    std::fs::remove_dir_all(&out).ok();
}
