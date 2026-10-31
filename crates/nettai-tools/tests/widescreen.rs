//! The enhanced mode's wide frames (docs/frontend.md §3, "Wider than the original") in each game, played from its
//! match file beside this test (tests/widescreen): at 240 columns a frame is the one the renderer drew before it
//! had a width, and the original mode's frames are 240 columns whatever the width; at two wider widths each
//! piece is where its anchor puts it (the HUD's against its edge, the field and its sprites in the middle, the
//! background across), in a fight, on the custom screen and on a screen of both.

use nettai_battle::input::keys;
use nettai_frontend::driver::{InFight, LivePlayer, Opponent};
use nettai_frontend::game::{Game, RomSource};
use nettai_frontend::player::Player;
use nettai_render::compose::{Anchor, BACKDROP_DEPTH, HEIGHT, Size, WIDTH, depth_key};
use nettai_render::textlayer::TextItem;
use nettai_render::vfont::TextRenderer;
use nettai_render::{Consoles, Frame, GraphicsMode};
use std::path::{Path, PathBuf};

/// The widths the placement is checked at: 16:9 at the frame's height, a phone held sideways (an odd column
/// left over), and a short window on a wide screen (more columns than a sprite's 9-bit X spans).
const WIDTHS: [usize; 3] = [284, 347, 1374];

/// The keys each match is played with: a chip picked and sent on the first custom screen, then moves and
/// attacks in the fight (the opponent attacks at random).
const KEYS: &[(u32, u16)] = &[(120, keys::A), (200, keys::START), (215, keys::A), (470, keys::UP), (520, keys::A), (640, keys::DOWN), (700, keys::A), (800, keys::A)];

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
    while (player.ticks() as u32) < tick {
        let next = player.ticks() as u32 + 1;
        let held = KEYS.iter().filter(|(at, _)| *at == next).fold(0, |k, (_, b)| k | b);
        assert!(player.tick(held), "the battle stopped before tick {tick}");
    }
}

/// The frame now, `width` columns wide.
fn frame_at(player: &mut Player, width: usize) -> Frame {
    player.renderer().size = Size::new(width, HEIGHT);
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

fn won(depth: u32) -> Won {
    let (field, background) = (depth_key(2, 3, 0), depth_key(3, 2, 0));
    match depth {
        d if d == field || d == background || d == BACKDROP_DEPTH => Won::Stage,
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

/// Each of `pieces`' HUD layer pixels in the 240-column frame `narrow` is the wide frame's where its anchor puts
/// it, color and depth key; and each piece has some.
fn pieces_placed(narrow: &Frame, wide: &Frame, pieces: &[Piece], label: &str) {
    let w = wide.width();
    for p in pieces {
        let [x0, y0, pw, ph] = p.rect;
        let origin = p.anchor.origin(Size::new(w, HEIGHT)).0 as usize;
        let mut seen = 0;
        for y in y0..y0 + ph {
            for x in x0..x0 + pw {
                let i = y * WIDTH + x;
                if won(narrow.depth[i]) != Won::Hud {
                    continue;
                }
                seen += 1;
                let j = y * w + x + origin;
                assert_eq!((wide.pixels[j], wide.depth[j]), (narrow.pixels[i], narrow.depth[i]), "{label}: {} at ({x}, {y}), {w} wide", p.what);
            }
        }
        assert!(seen > 0, "{label}: no pixel of {} in its box", p.what);
    }
}

/// The wide frame's text items are the 240-column frame's, in order, each moved where its anchor puts it; and
/// the items `expected` names (by their words, and a box of the original's screen their box's top left is in)
/// keep to the anchors it says.
fn items_placed(narrow: &Frame, wide: &Frame, expected: &[(&str, [i32; 4], Anchor)], label: &str) {
    let w = wide.width();
    assert_eq!(narrow.text.len(), wide.text.len(), "{label}: the text items");
    for (a, b) in narrow.text.iter().zip(&wide.text) {
        assert_eq!(a.text, b.text, "{label}");
        let (dx, dy) = b.anchor.origin(Size::new(w, HEIGHT));
        assert_eq!(b.rect, a.rect.offset(dx, dy), "{label}: {:?} kept to {:?}", b.text, b.anchor);
    }
    let found = |a: &TextItem, (words, [x, y, rw, rh]): (&str, [i32; 4])| a.text == words && (x..x + rw).contains(&a.rect.x) && (y..y + rh).contains(&a.rect.y);
    for &(words, rect, anchor) in expected {
        let k = narrow.text.iter().position(|a| found(a, (words, rect))).unwrap_or_else(|| panic!("{label}: no {words:?} in {rect:?}: {:?}", narrow.text.iter().map(|t| (&t.text, t.rect)).collect::<Vec<_>>()));
        assert_eq!(wide.text[k].anchor.horizontal, anchor.horizontal, "{label}: {words:?}");
    }
}

/// The field and the sprites on it are the 240-column frame's moved to the middle: of the stage's and the
/// sprites' pixels in a box of the original's screen (x, y, w, h), where the wide frame shows no HUD at the
/// moved place, nearly all match (what a HUD piece kept to an edge uncovers or covers differs; a sprite drawn
/// elsewhere leaves the stage in its place).
fn field_centered(narrow: &Frame, wide: &Frame, [x0, y0, bw, bh]: [usize; 4], label: &str) {
    let w = wide.width();
    let m = Anchor::CENTER.origin(Size::new(w, HEIGHT)).0 as usize;
    let (mut same, mut all) = (0, 0);
    for y in y0..y0 + bh {
        for x in x0..x0 + bw {
            let (i, j) = (y * WIDTH + x, y * w + x + m);
            if won(narrow.depth[i]) == Won::Hud || won(wide.depth[j]) == Won::Hud {
                continue;
            }
            all += 1;
            same += (narrow.pixels[i] == wide.pixels[j]) as usize;
        }
    }
    assert!(all > 1000 && same * 100 >= all * 99, "{label}: {same} of {all} of the field's pixels where they were, moved to the middle");
}

/// The background reaches both edges of a wide frame: where the 240-column frame's first or last column shows
/// the stage, so does the wide frame's; and somewhere it does.
fn background_across(narrow: &Frame, wide: &Frame, label: &str) {
    let w = wide.width();
    let mut seen = 0;
    for y in 0..HEIGHT {
        for (x, wx) in [(0, 0), (WIDTH - 1, w - 1)] {
            if won(narrow.depth[y * WIDTH + x]) == Won::Stage {
                seen += 1;
                assert_eq!(won(wide.depth[y * w + wx]), Won::Stage, "{label}: column {wx}, row {y}");
            }
        }
    }
    assert!(seen > 0, "{label}: no stage at the edges");
}

/// The checks of one moment of a game: the frame at 240 columns is the renderer's without a width; the original
/// mode's has 240 columns whatever the width; at each of `WIDTHS` the pieces, the items, the field and the
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
    let at_240 = frame_at(player, WIDTH);
    assert!(at_240.pixels == narrow.pixels && at_240.depth == narrow.depth && at_240.text == narrow.text, "{}: at 240 columns", m.label);
    assert_eq!(narrow.width(), WIDTH);
    for width in WIDTHS {
        let wide = frame_at(player, width);
        assert_eq!((wide.width(), wide.pixels.len()), (width, width * HEIGHT), "{}", m.label);
        let label = format!("{} at {width}", m.label);
        pieces_placed(&narrow, &wide, m.pieces, &label);
        items_placed(&narrow, &wide, m.items, &label);
        if let Some(field) = m.field {
            field_centered(&narrow, &wide, field, &label);
        }
        background_across(&narrow, &wide, &label);
    }
}

/// The original mode draws the original's 240 columns, whatever width the renderer is given.
fn original_stays(game: &Game, file: &str, tick: u32) {
    let mut p = player(game, file, GraphicsMode::Original, Consoles::One);
    play_to(&mut p, tick);
    let narrow = p.frame();
    let wide = frame_at(&mut p, WIDTHS[1]);
    assert_eq!(wide.width(), WIDTH);
    assert!(narrow.pixels == wide.pixels && narrow.depth == wide.depth, "{file}: the original mode at tick {tick}");
}

/// The custom screen: its window docked to the left edge, the HP box and emotion window beside it.
const CUSTOM: &[Piece] = &[Piece { what: "the custom window", rect: [0, 0, 120, 160], anchor: Anchor::LEFT }];

/// The field: beside the custom window, lowered with it; in a fight, between the HUD's top and bottom lines.
const CUSTOM_FIELD: [usize; 4] = [128, 80, 112, 80];
const FIGHT_FIELD: [usize; 4] = [0, 70, 240, 70];

#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn exe6_pieces_keep_to_their_edges() {
    let game = game("exe6");
    original_stays(&game, "m6.toml", 360);
    let mut p = player(&game, "m6.toml", GraphicsMode::Enhanced, Consoles::One);
    play_to(&mut p, 140);
    let names = [("MegaMan", [120, 0, 120, 16], Anchor::RIGHT)];
    check(&mut p, &Moment { label: "exe6 custom screen", pieces: CUSTOM, items: &names, field: Some(CUSTOM_FIELD) });
    play_to(&mut p, 360);
    let items = [
        ("980", [0, 0, 48, 16], Anchor::LEFT),
        ("CUSTOM", [48, 0, 144, 16], Anchor::CENTER),
        ("Cannon", [0, 140, 120, 20], Anchor::LEFT),
        ("AirShot", [120, 20, 120, 40], Anchor::RIGHT),
        ("1000", [120, 60, 120, 100], Anchor::CENTER),
    ];
    check(&mut p, &Moment { label: "exe6 fight", pieces: &[], items: &items, field: Some(FIGHT_FIELD) });
}

#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn exe5_pieces_keep_to_their_edges() {
    let game = game("exe5");
    original_stays(&game, "m5.toml", 360);
    let mut p = player(&game, "m5.toml", GraphicsMode::Enhanced, Consoles::One);
    play_to(&mut p, 140);
    // (The name plate and the custom screen's time left against the right edge.)
    let names = [("MegaMan", [120, 0, 120, 16], Anchor::RIGHT), ("59", [200, 0, 40, 40], Anchor::RIGHT)];
    check(&mut p, &Moment { label: "exe5 custom screen", pieces: CUSTOM, items: &names, field: Some(CUSTOM_FIELD) });
    play_to(&mut p, 360);
    let items = [
        ("1000", [0, 0, 48, 16], Anchor::LEFT),
        ("Sword", [0, 140, 120, 20], Anchor::LEFT),
        ("Boomer", [120, 20, 120, 40], Anchor::RIGHT),
    ];
    check(&mut p, &Moment { label: "exe5 fight", pieces: &[], items: &items, field: Some(FIGHT_FIELD) });
}

#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn exe4_pieces_keep_to_their_edges() {
    let game = game("exe4");
    original_stays(&game, "m4.toml", 360);
    let mut p = player(&game, "m4.toml", GraphicsMode::Enhanced, Consoles::One);
    play_to(&mut p, 140);
    check(&mut p, &Moment { label: "exe4 custom screen", pieces: CUSTOM, items: &[], field: Some(CUSTOM_FIELD) });
    // ("BUSY..." against the right edge while the other side chooses.)
    play_to(&mut p, 240);
    let busy = [("BUSY...", [176, 32, 64, 16], Anchor::RIGHT)];
    check(&mut p, &Moment { label: "exe4 waiting", pieces: &[], items: &busy, field: Some(FIGHT_FIELD) });
    play_to(&mut p, 360);
    let items = [("997", [0, 0, 48, 16], Anchor::LEFT), ("Sword", [0, 140, 120, 20], Anchor::LEFT)];
    check(&mut p, &Moment { label: "exe4 fight", pieces: &[], items: &items, field: Some(FIGHT_FIELD) });
}

/// On a screen of both the right console's pieces keep to the right edge: both custom windows docked to their
/// edges, both HP boxes and chip names in the corners.
#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn both_sides_keep_to_their_edges() {
    let game = game("exe6");
    let mut p = player(&game, "m6.toml", GraphicsMode::Enhanced, Consoles::Both);
    play_to(&mut p, 62);
    let windows = [
        Piece { what: "the left custom window", rect: [0, 0, 120, 160], anchor: Anchor::LEFT },
        Piece { what: "the right custom window", rect: [120, 0, 120, 160], anchor: Anchor::RIGHT },
    ];
    check(&mut p, &Moment { label: "both: both windows", pieces: &windows, items: &[], field: None });
    play_to(&mut p, 400);
    let items = [
        ("1000", [192, 0, 48, 16], Anchor::RIGHT),
        ("Cannon", [0, 140, 120, 20], Anchor::LEFT),
        ("AirShot", [120, 20, 120, 40], Anchor::RIGHT),
    ];
    check(&mut p, &Moment { label: "both: the fight", pieces: &[], items: &items, field: Some(FIGHT_FIELD) });
}
