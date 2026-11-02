//! The custom screen by pointer (docs/design/custom-screen-actions.md §5), in each game, played from the widescreen
//! test's match files (tests/widescreen; EXE6's with Crosses, tests/custom_pointer): at 240 by 160 and at a filled window's size, a click at each target the
//! renderer reports (`Renderer::targets`, the window at rest) is on that target (`Renderer::target_at`), and through
//! the pointer (`nettai_frontend::pointer`) and the player's layer it gives the target's choice: a chip its pick, OK,
//! a button its press, a pick its take-back, a form list entry its choice; the form list's tab opens the list. Then
//! played through `Player::point`: a click picks a chip, a click on its pick takes it back, a click on OK sends.

use nettai_battle::custom::layer::{Gesture, ListStage, Target};
use nettai_battle::custom::{Phase, SlotKind};
use nettai_battle::{PlayerTick, ScreenChoice};
use nettai_frontend::driver::{InFight, LivePlayer, Opponent};
use nettai_frontend::game::{Game, RomSource};
use nettai_frontend::player::Player;
use nettai_frontend::pointer::{PointerButton, PointerEvent, PointerKind, ScreenPointer};
use nettai_render::compose::Size;
use nettai_render::textlayer::Rect;
use nettai_render::{Consoles, Enhancements};
use std::path::{Path, PathBuf};

/// The original's size, and a filled window's (wider and taller, an odd row and column over).
const SIZES: [(usize, usize); 2] = [(240, 160), (347, 199)];

fn here() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn game(name: &str) -> Game {
    let roms = RomSource::from_env().unwrap_or_else(|e| panic!("{e}"));
    Game::load(&roms, Some(&here().join("../../content")), name, &[]).unwrap_or_else(|e| panic!("{e}"))
}

fn player(game: &Game, file: &str) -> Player {
    let text = std::fs::read_to_string(here().join("tests").join(file)).unwrap();
    let m = nettai_match::parse(&game.content, &text).unwrap_or_else(|p| panic!("{file}: {p:?}"));
    let mut live = LivePlayer::new(nettai_match::Set::of(&game.content, &m, m.seed.unwrap_or(1)));
    live.set_opponent(Opponent { in_fight: InFight::AttacksAtRandom, ..live.opponent() });
    let graphics = game.graphics(nettai_assets::BASE_LANGUAGE).unwrap_or_else(|e| panic!("{e}"));
    let mut renderer = graphics.renderer(Enhancements::ALL, None);
    renderer.consoles = Consoles::One;
    Player::with(renderer, None, None, Box::new(live))
}

/// Draw the frame at `size` (the targets are the last frame's).
fn draw(p: &mut Player, (w, h): (usize, usize)) {
    p.renderer().size = Size { width: w, height: h };
    p.frame();
}

fn targets(p: &mut Player) -> Vec<(Target, Rect)> {
    p.renderer().targets(0).collect()
}

/// Tick with nothing held until the local screen chooses and the frame drawn at `size` has targets.
fn to_choosing(p: &mut Player, size: (usize, usize), label: &str) {
    for _ in 0..600 {
        let choosing = p.battle().presented(0).is_some_and(|s| s.screen.phase == Phase::Choosing);
        draw(p, size);
        if choosing && !targets(p).is_empty() && p.layers()[0].is_some_and(|l| !l.is_up()) {
            return;
        }
        assert!(p.tick(0), "{label}: the battle stopped");
    }
    panic!("{label}: no custom screen to choose on");
}

/// The middle of a box, in the frame's pixels.
fn middle(r: Rect) -> (f32, f32) {
    (r.x as f32 + r.w as f32 / 2.0, r.y as f32 + r.h as f32 / 2.0)
}

/// A click at `(x, y)` through the pointer and a copy of the local layer: the target it was on, the tick's choice,
/// and the layer after.
fn click(p: &mut Player, (x, y): (f32, f32)) -> (Option<Target>, Option<ScreenChoice>, nettai_battle::custom::Layer) {
    let mut layer = *p.layers()[0].expect("the local layer");
    let mut pointer = ScreenPointer::default();
    let renderer = p.renderer();
    let at = |x: f32, y: f32| renderer.target_at(0, x as i32, y as i32);
    let on = at(x, y);
    let e = |kind| PointerEvent { kind, button: PointerButton::Primary, touch: false, x, y };
    for g in [pointer.event(e(PointerKind::Down), at), pointer.event(e(PointerKind::Up), at)].into_iter().flatten() {
        layer.point(g);
    }
    let mut choice = None;
    for _ in 0..2 {
        choice = choice.or(layer.input(PlayerTick::default(), p.battle()).choice);
    }
    (on, choice, layer)
}

/// What a click on `t` chooses.
fn expected(p: &Player, t: Target) -> Option<ScreenChoice> {
    let screen = p.battle().presented(0).unwrap().screen;
    match t {
        Target::Slot(s) => match screen.slots[s as usize].kind {
            SlotKind::Chip { .. } | SlotKind::Offered(_) => Some(ScreenChoice::Pick(s)),
            SlotKind::Ok => Some(ScreenChoice::Ok),
            SlotKind::Button { .. } => Some(ScreenChoice::Press(s)),
            SlotKind::Empty | SlotKind::Hidden => None,
        },
        Target::Pick(_) => Some(ScreenChoice::TakeBack),
        Target::Form(k) => Some(ScreenChoice::ChooseForm(k)),
        Target::FormTab => None,
    }
}

/// Every target's middle is on it, and a click there gives its choice; the tab opens the form list where it is
/// offered, whose entries a click chooses.
fn each_target_chooses(p: &mut Player, size: (usize, usize), label: &str) -> usize {
    let found = targets(p);
    let mut checked = 0;
    for &(t, r) in &found {
        let (on, choice, layer) = click(p, middle(r));
        assert_eq!(on, Some(t), "{label} {size:?}: the middle of {t:?}'s box {r:?}");
        if t == Target::FormTab {
            let offered = p.battle().form_list(0).is_some_and(|l| l.count > 0);
            assert_eq!(layer.list_stage().is_some(), offered, "{label}: the tab opens the form list where it is offered");
        } else {
            assert_eq!(choice, expected(p, t), "{label} {size:?}: a click on {t:?}");
        }
        checked += 1;
    }
    checked
}

/// A pointer's click through the player, in the frame's pixels.
fn point_click(p: &mut Player, (x, y): (f32, f32)) {
    for kind in [PointerKind::Moved, PointerKind::Down, PointerKind::Up] {
        p.point(PointerEvent { kind, button: PointerButton::Primary, touch: false, x, y });
    }
}

fn box_of(p: &mut Player, t: Target) -> Rect {
    targets(p).into_iter().find(|(u, _)| *u == t).unwrap_or_else(|| panic!("no {t:?} among the targets")).1
}

fn pointer_plays(name: &str, file: &str) {
    let game = game(name);
    for size in SIZES {
        let label = format!("{name} at {}x{}", size.0, size.1);
        let mut p = player(&game, file);
        to_choosing(&mut p, size, &label);
        let found = targets(&mut p);
        assert!(found.iter().any(|(t, _)| matches!(t, Target::Slot(10))), "{label}: OK is a target");
        assert!(found.iter().filter(|(t, _)| matches!(t, Target::Slot(s) if *s < 10)).count() >= 5, "{label}: the dealt chips are targets");
        // (The window keeps to the top left: its places are the original's.)
        assert_eq!(box_of(&mut p, Target::Slot(0)), Rect::new(8, 104, 16, 24), "{label}: the first chip's cell");
        let checked = each_target_chooses(&mut p, size, &label);
        // A click on the first chip picks it; its pick is then a target, whose click takes it back.
        let chip = middle(box_of(&mut p, Target::Slot(0)));
        point_click(&mut p, chip);
        p.tick(0);
        draw(&mut p, size);
        assert_eq!(p.battle().presented(0).unwrap().screen.selection(), [0], "{label}: a click picks");
        assert_eq!(p.layers()[0].unwrap().cursor, 0);
        let checked = checked + each_target_chooses(&mut p, size, &label);
        let pick = middle(box_of(&mut p, Target::Pick(0)));
        point_click(&mut p, pick);
        p.tick(0);
        assert!(p.battle().presented(0).unwrap().screen.selection().is_empty(), "{label}: a click on the pick takes it back");
        // The form list's tab, where the game has one: a click opens the list; its entries are the targets, each
        // choosing its form; the tab again closes it.
        draw(&mut p, size);
        if let Some(tab) = targets(&mut p).into_iter().find(|(t, _)| *t == Target::FormTab).map(|(_, r)| middle(r)) {
            point_click(&mut p, tab);
            let up = (0..60).any(|_| {
                p.tick(0);
                draw(&mut p, size);
                matches!(p.layers()[0].unwrap().list_stage(), Some(ListStage::Up(n)) if n > 1)
            });
            assert!(up, "{label}: the tab opens the form list");
            let entries = targets(&mut p).iter().filter(|(t, _)| matches!(t, Target::Form(_))).count();
            assert!(entries > 0, "{label}: the form list's entries are targets");
            each_target_chooses(&mut p, size, &label);
            let tab = middle(box_of(&mut p, Target::FormTab));
            point_click(&mut p, tab);
            let closed = (0..60).any(|_| {
                p.tick(0);
                p.layers()[0].unwrap().list_stage().is_none()
            });
            assert!(closed, "{label}: the tab again closes the form list");
            draw(&mut p, size);
        }
        // A click on OK sends the hand.
        let ok = middle(box_of(&mut p, Target::Slot(10)));
        point_click(&mut p, ok);
        p.tick(0);
        p.tick(0);
        assert_ne!(p.battle().presented(0).map(|s| s.screen.phase), Some(Phase::Choosing), "{label}: a click on OK sends");
        println!("{label}: {checked} targets clicked");
    }
}

#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn exe6_by_pointer() {
    pointer_plays("exe6", "custom_pointer/m6.toml");
}

#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn exe5_by_pointer() {
    pointer_plays("exe5", "widescreen/m5.toml");
}

#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn exe4_by_pointer() {
    pointer_plays("exe4", "widescreen/m4.toml");
}

/// A tap held still on a chip describes it; a gesture of the mouse's on the picture with no screen up does nothing.
#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn a_long_press_describes() {
    let game = game("exe6");
    let mut p = player(&game, "custom_pointer/m6.toml");
    to_choosing(&mut p, SIZES[0], "exe6");
    let chip = middle(box_of(&mut p, Target::Slot(2)));
    let e = |kind| PointerEvent { kind, button: PointerButton::Primary, touch: true, x: chip.0, y: chip.1 };
    p.point(e(PointerKind::Down));
    let described = (0..nettai_frontend::pointer::LONG_PRESS + 5).any(|_| {
        p.tick(0);
        p.layers()[0].unwrap().chatbox().is_some()
    });
    assert!(described, "a long press describes");
    assert_eq!(p.layers()[0].unwrap().cursor, 2, "the finger's down put the cursor there");
    p.point(e(PointerKind::Up));
    p.tick(0);
    assert!(p.battle().presented(0).unwrap().screen.selection().is_empty(), "its letting go picks nothing");
    let _ = Gesture::Click(None);
}
