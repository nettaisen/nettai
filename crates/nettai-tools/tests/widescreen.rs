//! The enhanced mode's wide and tall frames (docs/frontend.md §3, "Larger than the original") in each game, played
//! from its match file beside this test (tests/widescreen): at 240 by 160 a frame is the one the renderer drew
//! before it had a size, and the original mode's frames are 240 by 160 whatever the size; at wider and taller
//! sizes each piece is where its two anchors put it (the HUD's against its edges, the field and its sprites in the
//! middle, the background across), in a fight, on the custom screen (its window at the top left), with the
//! chatbox up and on a screen of both.

use nettai_battle::input::keys;
use nettai_frontend::driver::{InFight, LivePlayer, Opponent};
use nettai_frontend::game::{Game, RomSource};
use nettai_frontend::player::Player;
use nettai_render::compose::{Anchor, BACKDROP_DEPTH, HEIGHT, Size, WIDTH, depth_key};
use nettai_render::textlayer::TextItem;
use nettai_render::vfont::{Role, TextRenderer};
use nettai_render::{Consoles, Frame, GraphicsMode};
use std::path::{Path, PathBuf};

/// The sizes the placement is checked at: 16:9 at the frame's height, a phone held sideways (an odd column left
/// over), and a short window on a wide screen (more columns than a sprite's 9-bit X spans); a frame taller than
/// the original's (an odd row left over), one taller and wider, and one held upright (taller than a sprite's
/// 8-bit Y spans).
const SIZES: [(usize, usize); 6] = [(284, 160), (347, 160), (1374, 160), (240, 213), (347, 199), (240, 427)];

/// The keys each match is played with: a chip picked and sent on the first custom screen, then moves and
/// attacks in the fight (the opponent attacks at random).
const KEYS: &[(u32, u16)] = &[(120, keys::A), (200, keys::START), (215, keys::A), (470, keys::UP), (520, keys::A), (640, keys::DOWN), (700, keys::A), (800, keys::A)];

/// The keys that open a chip's description on the first custom screen (the chatbox).
const CHAT_KEYS: &[(u32, u16)] = &[(100, keys::R), (180, keys::R), (190, keys::A), (260, keys::START)];

fn here() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Game `name`'s content (the repository's) and its assets, extracted from the ROMs.
fn game(name: &str) -> Game {
    let roms = RomSource::from_env().unwrap_or_else(|e| panic!("{e}"));
    Game::load(&roms, Some(&here().join("../../content")), name, &[]).unwrap_or_else(|e| panic!("{e}"))
}

/// The match file's set played live in `mode`, showing `consoles`.
fn player(game: &Game, file: &str, mode: GraphicsMode, consoles: Consoles) -> Player {
    let text = std::fs::read_to_string(here().join("tests/widescreen").join(file)).unwrap();
    let m = nettai_match::parse(&game.content, &text).unwrap_or_else(|p| panic!("{file}: {p:?}"));
    let mut live = LivePlayer::new(nettai_match::Set::of(&game.content, &m, m.seed.unwrap_or(1)));
    live.set_opponent(Opponent { in_fight: InFight::AttacksAtRandom, ..live.opponent() });
    let graphics = game.graphics(nettai_assets::BASE_LANGUAGE).unwrap_or_else(|e| panic!("{e}"));
    let font = nettai_frontend::game::font(mode, None, nettai_assets::BASE_LANGUAGE).unwrap_or_else(|e| panic!("{e}"));
    let mut renderer = graphics.renderer(mode, font.clone());
    renderer.consoles = consoles;
    let mut player = Player::with(renderer, font.map(TextRenderer::new), None, Box::new(live));
    let backgrounds = nettai_render::graphics::set_backgrounds(&player.battle().setup);
    player.renderer().graphics().wait_backgrounds(&game.content, backgrounds);
    player
}

/// Play on to tick `tick` (the keys as `KEYS` holds them, a tick each).
fn play_to(player: &mut Player, tick: u32) {
    play_keys_to(player, KEYS, tick);
}

/// Play on to tick `tick`, `keys` held a tick each.
fn play_keys_to(player: &mut Player, keys: &[(u32, u16)], tick: u32) {
    while (player.ticks() as u32) < tick {
        let next = player.ticks() as u32 + 1;
        let held = keys.iter().filter(|(at, _)| *at == next).fold(0, |k, (_, b)| k | b);
        assert!(player.tick(held), "the battle stopped before tick {tick}");
    }
}

/// The frame now, at `size`.
fn frame_at(player: &mut Player, size: Size) -> Frame {
    player.renderer().size = size;
    let frame = player.frame();
    player.renderer().size = Size::ORIGINAL;
    frame
}

/// What won a pixel, by its depth key: the field layer, the background layer or the backdrop (the stage), a
/// sprite, or another layer (the HUD's, the custom screen's names, the chatbox's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Won {
    Stage,
    Sprite,
    Hud,
}

/// The background layer's depth key.
fn background_key() -> u32 {
    depth_key(3, 2, 0)
}

fn won(depth: u32) -> Won {
    let field = depth_key(2, 3, 0);
    match depth {
        d if d == field || d == background_key() || d == BACKDROP_DEPTH => Won::Stage,
        d if (d >> 16) & 0xFF == 0 => Won::Sprite,
        _ => Won::Hud,
    }
}

/// A piece of the frame to find where its anchor puts it: the HUD layer's pixels in a box of the original's
/// screen (x, y, w, h).
struct Piece {
    what: &'static str,
    rect: [usize; 4],
    anchor: Anchor,
}

/// Each of `pieces`' HUD layer pixels in the 240 by 160 frame `narrow` is the large frame's where its anchor
/// puts it, color and depth key; and each piece has some.
fn pieces_placed(narrow: &Frame, large: &Frame, pieces: &[Piece], label: &str) {
    let w = large.width();
    for p in pieces {
        let [x0, y0, pw, ph] = p.rect;
        let (ox, oy) = p.anchor.origin(large.size);
        let mut seen = 0;
        for y in y0..y0 + ph {
            for x in x0..x0 + pw {
                let i = y * WIDTH + x;
                if won(narrow.depth[i]) != Won::Hud {
                    continue;
                }
                seen += 1;
                let j = (y + oy as usize) * w + x + ox as usize;
                assert_eq!((large.pixels[j], large.depth[j]), (narrow.pixels[i], narrow.depth[i]), "{label}: {} at ({x}, {y})", p.what);
            }
        }
        assert!(seen > 0, "{label}: no pixel of {} in its box", p.what);
    }
}

/// The large frame's text items are the 240 by 160 frame's, in order, each moved where its anchor puts it; and
/// the items `expected` names (by their words, and a box of the original's screen their box's top left is in)
/// keep to the anchors it says.
fn items_placed(narrow: &Frame, large: &Frame, expected: &[(&str, [i32; 4], Anchor)], label: &str) {
    assert_eq!(narrow.text.len(), large.text.len(), "{label}: the text items");
    for (a, b) in narrow.text.iter().zip(&large.text) {
        assert_eq!(a.text, b.text, "{label}");
        let (ox, oy) = b.anchor.origin(large.size);
        assert_eq!(b.rect, a.rect.offset(ox, oy), "{label}: {:?} kept to {:?}", b.text, b.anchor);
    }
    let found = |a: &TextItem, (words, [x, y, rw, rh]): (&str, [i32; 4])| a.text == words && (x..x + rw).contains(&a.rect.x) && (y..y + rh).contains(&a.rect.y);
    for &(words, rect, anchor) in expected {
        let k = narrow.text.iter().position(|a| found(a, (words, rect))).unwrap_or_else(|| panic!("{label}: no {words:?} in {rect:?}: {:?}", narrow.text.iter().map(|t| (&t.text, t.rect)).collect::<Vec<_>>()));
        assert_eq!(large.text[k].anchor, anchor, "{label}: {words:?}");
    }
}

/// The field and the sprites on it are the 240 by 160 frame's moved to the middle: of the stage's and the
/// sprites' pixels in a box of the original's screen (x, y, w, h), where the large frame shows no HUD at the
/// moved place, nearly all match (what a HUD piece kept to an edge uncovers or covers differs; a sprite drawn
/// elsewhere leaves the stage in its place).
fn field_centered(narrow: &Frame, large: &Frame, [x0, y0, bw, bh]: [usize; 4], label: &str) {
    let w = large.width();
    let (mx, my) = Anchor::CENTER.origin(large.size);
    let (mut same, mut all) = (0, 0);
    for y in y0..y0 + bh {
        for x in x0..x0 + bw {
            let (i, j) = (y * WIDTH + x, (y + my as usize) * w + x + mx as usize);
            if won(narrow.depth[i]) == Won::Hud || won(large.depth[j]) == Won::Hud {
                continue;
            }
            all += 1;
            same += (narrow.pixels[i] == large.pixels[j]) as usize;
        }
    }
    assert!(all > 1000 && same * 100 >= all * 99, "{label}: {same} of {all} of the field's pixels where they were, moved to the middle");
}

/// The background reaches all four edges of a large frame: where the 240 by 160 frame's first or last column
/// shows the stage, so does the large frame's on that row, but where a piece kept to the top or the bottom
/// covers it there; the background layer wins pixels of its first and last rows and columns wherever the 240 by
/// 160 frame's do; and somewhere it does.
fn background_across(narrow: &Frame, large: &Frame, label: &str) {
    let (w, h) = (large.width(), large.height());
    let my = Anchor::CENTER.origin(large.size).1 as usize;
    let mut seen = 0;
    for y in 0..HEIGHT {
        for (x, wx) in [(0, 0), (WIDTH - 1, w - 1)] {
            let at = (y + my) * w + wx;
            if won(narrow.depth[y * WIDTH + x]) == Won::Stage && (h == HEIGHT || won(large.depth[at]) != Won::Hud) {
                seen += 1;
                assert_eq!(won(large.depth[at]), Won::Stage, "{label}: column {wx}, row {}", y + my);
            }
        }
    }
    assert!(seen > 0, "{label}: no stage at the edges");
    if h == HEIGHT || !narrow.depth.contains(&background_key()) {
        return;
    }
    // (Over the rows above the original's and those under them: a row of a background may be clear, the backdrop
    // under it.)
    let background = |rows: std::ops::Range<usize>| (rows.start * w..rows.end * w).filter(|&i| large.depth[i] == background_key()).count();
    for (edge, rows) in [("top", 0..my), ("bottom", my + HEIGHT..h)] {
        assert!(background(rows) > 0, "{label}: no background at the {edge} edge");
    }
}

/// The checks of one moment of a game: the frame at 240 by 160 is the renderer's without a size; the original
/// mode's is 240 by 160 whatever the size; at each of `SIZES` the pieces, the items, the field and the
/// background are where they go.
struct Moment<'a> {
    label: &'a str,
    pieces: &'a [Piece],
    items: &'a [(&'a str, [i32; 4], Anchor)],
    /// Where the field shows on the original's screen (none: nowhere much).
    field: Option<[usize; 4]>,
}

fn check(player: &mut Player, m: &Moment) {
    let narrow = player.frame();
    let at_240 = frame_at(player, Size::ORIGINAL);
    assert!(at_240.pixels == narrow.pixels && at_240.depth == narrow.depth && at_240.text == narrow.text, "{}: at 240 by 160", m.label);
    assert_eq!(narrow.size, Size::ORIGINAL);
    for (width, height) in SIZES {
        let large = frame_at(player, Size::new(width, height));
        assert_eq!((large.width(), large.height(), large.pixels.len()), (width, height, width * height), "{}", m.label);
        let label = format!("{} at {width}x{height}", m.label);
        pieces_placed(&narrow, &large, m.pieces, &label);
        items_placed(&narrow, &large, m.items, &label);
        if let Some(field) = m.field {
            field_centered(&narrow, &large, field, &label);
        }
        background_across(&narrow, &large, &label);
    }
}

/// The original mode draws the original's 240 by 160, whatever size the renderer is given.
fn original_stays(game: &Game, file: &str, tick: u32) {
    let mut p = player(game, file, GraphicsMode::Original, Consoles::One);
    play_to(&mut p, tick);
    let narrow = p.frame();
    let large = frame_at(&mut p, Size::new(SIZES[4].0, SIZES[4].1));
    assert_eq!(large.size, Size::ORIGINAL);
    assert!(narrow.pixels == large.pixels && narrow.depth == large.depth, "{file}: the original mode at tick {tick}");
}

/// The custom screen: its window docked to the left edge at the top (its 160 rows, not stretched), the HP box
/// and emotion window beside it.
const CUSTOM: &[Piece] = &[Piece { what: "the custom window", rect: [0, 0, 120, 160], anchor: Anchor::TOP_LEFT }];

/// The field: beside the custom window, lowered with it; in a fight, between the HUD's top and bottom lines.
const CUSTOM_FIELD: [usize; 4] = [128, 80, 112, 80];
const FIGHT_FIELD: [usize; 4] = [0, 70, 240, 70];

/// With a chip's description up, its chatbox's lines keep to the middle with the field, both ways: the first
/// frame its first line has printed some of.
fn chatbox_with_the_field(game: &Game, file: &str, label: &str) {
    let mut p = player(game, file, GraphicsMode::Enhanced, Consoles::One);
    let printed = |frame: &Frame| frame.text.iter().any(|t| t.role == Role::Dialogue && t.shown.is_some_and(|n| n > 0));
    let mut narrow = p.frame();
    while !printed(&narrow) {
        assert!(p.ticks() < 300, "{label}: no chatbox by tick 300");
        let next = p.ticks() as u32 + 1;
        play_keys_to(&mut p, CHAT_KEYS, next);
        narrow = p.frame();
    }
    for (width, height) in SIZES {
        let large = frame_at(&mut p, Size::new(width, height));
        let label = format!("{label} at {width}x{height}");
        items_placed(&narrow, &large, &[], &label);
        for t in large.text.iter().filter(|t| t.role == Role::Dialogue) {
            assert_eq!(t.anchor, Anchor::CENTER, "{label}: {:?}", t.text);
        }
    }
}

#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn exe6_pieces_keep_to_their_edges() {
    let game = game("exe6");
    original_stays(&game, "m6.toml", 360);
    chatbox_with_the_field(&game, "m6.toml", "exe6 chatbox");
    let mut p = player(&game, "m6.toml", GraphicsMode::Enhanced, Consoles::One);
    play_to(&mut p, 140);
    let names = [("MegaMan", [120, 0, 120, 16], Anchor::TOP_RIGHT)];
    check(&mut p, &Moment { label: "exe6 custom screen", pieces: CUSTOM, items: &names, field: Some(CUSTOM_FIELD) });
    play_to(&mut p, 360);
    let items = [
        ("980", [0, 0, 48, 16], Anchor::TOP_LEFT),
        ("CUSTOM", [48, 0, 144, 16], Anchor::TOP),
        ("Cannon", [0, 140, 120, 20], Anchor::BOTTOM_LEFT),
        ("AirShot", [120, 20, 120, 40], Anchor::TOP_RIGHT),
        ("1000", [120, 60, 120, 100], Anchor::CENTER),
    ];
    check(&mut p, &Moment { label: "exe6 fight", pieces: &[], items: &items, field: Some(FIGHT_FIELD) });
}

#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn exe5_pieces_keep_to_their_edges() {
    let game = game("exe5");
    original_stays(&game, "m5.toml", 360);
    chatbox_with_the_field(&game, "m5.toml", "exe5 chatbox");
    let mut p = player(&game, "m5.toml", GraphicsMode::Enhanced, Consoles::One);
    play_to(&mut p, 140);
    // (The name plate and the custom screen's time left against the top right corner.)
    let names = [("MegaMan", [120, 0, 120, 16], Anchor::TOP_RIGHT), ("59", [200, 0, 40, 40], Anchor::TOP_RIGHT)];
    check(&mut p, &Moment { label: "exe5 custom screen", pieces: CUSTOM, items: &names, field: Some(CUSTOM_FIELD) });
    play_to(&mut p, 360);
    let items = [
        ("1000", [0, 0, 48, 16], Anchor::TOP_LEFT),
        ("Sword", [0, 140, 120, 20], Anchor::BOTTOM_LEFT),
        ("Boomer", [120, 20, 120, 40], Anchor::TOP_RIGHT),
    ];
    check(&mut p, &Moment { label: "exe5 fight", pieces: &[], items: &items, field: Some(FIGHT_FIELD) });
}

#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn exe4_pieces_keep_to_their_edges() {
    let game = game("exe4");
    original_stays(&game, "m4.toml", 360);
    chatbox_with_the_field(&game, "m4.toml", "exe4 chatbox");
    let mut p = player(&game, "m4.toml", GraphicsMode::Enhanced, Consoles::One);
    play_to(&mut p, 140);
    check(&mut p, &Moment { label: "exe4 custom screen", pieces: CUSTOM, items: &[], field: Some(CUSTOM_FIELD) });
    // ("BUSY..." against the top right corner while the other side chooses.)
    play_to(&mut p, 240);
    let busy = [("BUSY...", [176, 32, 64, 16], Anchor::TOP_RIGHT)];
    check(&mut p, &Moment { label: "exe4 waiting", pieces: &[], items: &busy, field: Some(FIGHT_FIELD) });
    play_to(&mut p, 360);
    let items = [("997", [0, 0, 48, 16], Anchor::TOP_LEFT), ("Sword", [0, 140, 120, 20], Anchor::BOTTOM_LEFT)];
    check(&mut p, &Moment { label: "exe4 fight", pieces: &[], items: &items, field: Some(FIGHT_FIELD) });
}

/// On a screen of both the right console's pieces keep to the right edge, as high as the left's: both custom
/// windows docked to their edges at the top, both HP boxes in the top corners and chip names in the bottom ones.
#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn both_sides_keep_to_their_edges() {
    let game = game("exe6");
    let mut p = player(&game, "m6.toml", GraphicsMode::Enhanced, Consoles::Both);
    play_to(&mut p, 62);
    let windows = [
        Piece { what: "the left custom window", rect: [0, 0, 120, 160], anchor: Anchor::TOP_LEFT },
        Piece { what: "the right custom window", rect: [120, 0, 120, 160], anchor: Anchor::TOP_RIGHT },
    ];
    check(&mut p, &Moment { label: "both: both windows", pieces: &windows, items: &[], field: None });
    play_to(&mut p, 400);
    let items = [
        ("1000", [192, 0, 48, 16], Anchor::TOP_RIGHT),
        ("Cannon", [0, 140, 120, 20], Anchor::BOTTOM_LEFT),
        ("AirShot", [120, 20, 120, 40], Anchor::TOP_RIGHT),
    ];
    check(&mut p, &Moment { label: "both: the fight", pieces: &[], items: &items, field: Some(FIGHT_FIELD) });
}
