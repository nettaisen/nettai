//! The custom screen by pointer (docs/design/custom-screen-actions.md §5), in each game, played from the widescreen
//! test's match files (tests/widescreen; EXE6's with Crosses, tests/custom_pointer): at 240 by 160 and at a filled window's size, a click at each target the
//! renderer reports (`Renderer::targets`, the window at rest) is on that target (`Renderer::target_at`), and through
//! the pointer (`nettai_frontend::pointer`) and the player's layer it gives the target's choice: a chip its pick, OK,
//! a button its press (or the release of the chip it holds), a pick its `Unpick`, a form list entry its choice; the
//! form list's tab opens the list, and its exit label (EXE6's B:EXIT) closes it. Then played through
//! `Player::point`: clicks pick two chips, a click on the first pick takes it alone back (the second moving up, the
//! cursor to its slot), the tab opens the form list and closes it, so does its exit label and a click off its window,
//! a click on OK sends; in EXE6 a click on the Cross chosen un-chooses it, the picks kept.

use nettai_battle::input::Keys;
use nettai_battle::custom::layer::{ListStage, Target};
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
        assert!(p.tick(Keys::empty()), "{label}: the battle stopped");
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
    let e = |kind| PointerEvent { kind, button: PointerButton::Primary, x, y };
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
            SlotKind::Button { .. } if screen.hold.is_some_and(|h| screen.button_slot(s) == Some(h.button)) => Some(ScreenChoice::Release(s)),
            SlotKind::Button { .. } => Some(ScreenChoice::Press(s)),
            SlotKind::Empty | SlotKind::Hidden => None,
        },
        Target::Pick(k) => Some(ScreenChoice::Unpick(k)),
        Target::Form(k) => Some(ScreenChoice::ChooseForm(k)),
        Target::FormTab | Target::FormExit => None,
    }
}

/// Every target's middle is on it, and a click there gives its choice; the tab opens the form list where it is
/// offered, whose entries a click chooses and whose exit label closes it.
fn each_target_chooses(p: &mut Player, size: (usize, usize), label: &str) -> usize {
    let found = targets(p);
    let mut checked = 0;
    for &(t, r) in &found {
        let (on, choice, layer) = click(p, middle(r));
        assert_eq!(on, Some(t), "{label} {size:?}: the middle of {t:?}'s box {r:?}");
        if t == Target::FormTab {
            let offered = p.battle().form_list(0).is_some_and(|l| l.count > 0);
            assert_eq!(layer.list_stage().is_some(), offered, "{label}: the tab opens the form list where it is offered");
        } else if t == Target::FormExit {
            assert!(matches!(layer.list_stage(), Some(ListStage::Closing(_))), "{label} {size:?}: a click on B:EXIT closes the form list");
            assert_eq!(choice, None, "{label} {size:?}: B:EXIT chooses nothing");
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
        p.point(PointerEvent { kind, button: PointerButton::Primary, x, y });
    }
}

/// A click at the middle of `t`'s box, through the player.
fn click_on(p: &mut Player, t: Target) {
    let at = middle(box_of(p, t));
    point_click(p, at);
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
        assert!(!found.iter().any(|(t, _)| *t == Target::FormExit), "{label}: no exit label while the form list is down");
        let checked = each_target_chooses(&mut p, size, &label);
        // A click on the first chip picks it; its pick is then a target, whose click takes it back.
        let chip = middle(box_of(&mut p, Target::Slot(0)));
        point_click(&mut p, chip);
        p.tick(Keys::empty());
        draw(&mut p, size);
        assert_eq!(p.battle().presented(0).unwrap().screen.selection(), [0], "{label}: a click picks");
        assert_eq!(p.layers()[0].unwrap().cursor, 0);
        let checked = checked + each_target_chooses(&mut p, size, &label);
        // A second chip that goes with it, then a click on the first pick: it alone is taken back.
        let screen = p.battle().presented(0).unwrap().screen;
        let second = (1..10u8).find(|&s| {
            let slot = screen.slots[s as usize];
            matches!(slot.kind, SlotKind::Chip { .. }) && slot.state == nettai_battle::custom::SlotState::Selectable
        });
        if let Some(second) = second {
            click_on(&mut p, Target::Slot(second));
            p.tick(Keys::empty());
            draw(&mut p, size);
            assert_eq!(p.battle().presented(0).unwrap().screen.selection(), [0, second], "{label}: a second click picks");
            click_on(&mut p, Target::Pick(0));
            p.tick(Keys::empty());
            draw(&mut p, size);
            assert_eq!(p.battle().presented(0).unwrap().screen.selection(), [second], "{label}: a click on the first pick takes it alone back");
            assert_eq!(p.layers()[0].unwrap().cursor, 0, "{label}: the cursor on the slot the pick went back to");
        }
        while !p.battle().presented(0).unwrap().screen.selection().is_empty() {
            click_on(&mut p, Target::Pick(0));
            p.tick(Keys::empty());
            draw(&mut p, size);
        }
        // The form list's tab, where the game has one: a click opens the list; its entries are the targets, each
        // choosing its form; the tab again closes it.
        draw(&mut p, size);
        if let Some(tab) = targets(&mut p).into_iter().find(|(t, _)| *t == Target::FormTab).map(|(_, r)| middle(r)) {
            point_click(&mut p, tab);
            let up = (0..60).any(|_| {
                p.tick(Keys::empty());
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
                p.tick(Keys::empty());
                p.layers()[0].unwrap().list_stage().is_none()
            });
            assert!(closed, "{label}: the tab again closes the form list");
            // Opened again, its exit label (B:EXIT, at the window's foot) closes it.
            draw(&mut p, size);
            let tab = middle(box_of(&mut p, Target::FormTab));
            point_click(&mut p, tab);
            let up = (0..60).any(|_| {
                p.tick(Keys::empty());
                draw(&mut p, size);
                matches!(p.layers()[0].unwrap().list_stage(), Some(ListStage::Up(n)) if n > 1)
            });
            assert!(up, "{label}: the tab opens the form list again");
            let exit = box_of(&mut p, Target::FormExit);
            point_click(&mut p, middle(exit));
            let closed = (0..60).any(|_| {
                p.tick(Keys::empty());
                p.layers()[0].unwrap().list_stage().is_none()
            });
            assert!(closed, "{label}: B:EXIT closes the form list");
            // Opened again, each entry's row spans the window as its exit label does, and a click off the window (on
            // the first chip) closes it, picking nothing.
            draw(&mut p, size);
            let tab = middle(box_of(&mut p, Target::FormTab));
            point_click(&mut p, tab);
            let up = (0..60).any(|_| {
                p.tick(Keys::empty());
                draw(&mut p, size);
                matches!(p.layers()[0].unwrap().list_stage(), Some(ListStage::Up(n)) if n > 1)
            });
            assert!(up, "{label}: the tab opens the form list a third time");
            let (entry, exit) = (box_of(&mut p, Target::Form(0)), box_of(&mut p, Target::FormExit));
            assert_eq!((entry.x, entry.w), (exit.x, exit.w), "{label}: an entry's row spans the window");
            point_click(&mut p, chip);
            let closed = (0..60).any(|_| {
                p.tick(Keys::empty());
                p.layers()[0].unwrap().list_stage().is_none()
            });
            assert!(closed, "{label}: a click off the window closes the form list");
            assert!(p.battle().presented(0).unwrap().screen.selection().is_empty(), "{label}: a click off the window picks nothing");
            draw(&mut p, size);
        }
        // A click on OK sends the hand.
        let ok = middle(box_of(&mut p, Target::Slot(10)));
        point_click(&mut p, ok);
        p.tick(Keys::empty());
        p.tick(Keys::empty());
        assert_ne!(p.battle().presented(0).map(|s| s.screen.phase), Some(Phase::Choosing), "{label}: a click on OK sends");
        println!("{label}: {checked} targets clicked; the first of two picks taken back: {}", second.is_some());
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

/// Opens the form list by its tab and waits until it is up.
fn open_form_list(p: &mut Player, size: (usize, usize), label: &str) {
    click_on(p, Target::FormTab);
    let up = (0..60).any(|_| {
        p.tick(Keys::empty());
        draw(p, size);
        matches!(p.layers()[0].unwrap().list_stage(), Some(ListStage::Up(n)) if n > 1)
    });
    assert!(up, "{label}: the tab opens the form list");
}

/// EXE6: a click on the Cross chosen un-chooses it, the picks kept (the user: "unselecting a cross shouldn't remove
/// all chips").
#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn exe6_unchoosing_a_cross_keeps_the_picks() {
    let game = game("exe6");
    let size = SIZES[0];
    let label = "exe6";
    let mut p = player(&game, "custom_pointer/m6.toml");
    to_choosing(&mut p, size, label);
    click_on(&mut p, Target::Slot(0));
    p.tick(Keys::empty());
    draw(&mut p, size);
    assert_eq!(p.battle().presented(0).unwrap().screen.selection(), [0]);
    open_form_list(&mut p, size, label);
    click_on(&mut p, Target::Form(0));
    // The Cross's window plays; then the screen chooses again.
    let chosen = (0..120).any(|_| {
        p.tick(Keys::empty());
        draw(&mut p, size);
        let s = p.battle().presented(0).unwrap().screen;
        s.phase == Phase::Choosing && s.form.is_some() && p.layers()[0].unwrap().list_stage().is_none()
    });
    assert!(chosen, "{label}: a click on an entry chooses its Cross");
    assert_eq!(p.battle().form_list(0).and_then(|l| l.chosen), Some(0));
    open_form_list(&mut p, size, label);
    let entry = middle(box_of(&mut p, Target::Form(0)));
    let (_, choice, _) = click(&mut p, entry);
    assert_eq!(choice, Some(ScreenChoice::Unchoose), "{label}: a click on the Cross chosen");
    click_on(&mut p, Target::Form(0));
    p.tick(Keys::empty());
    p.tick(Keys::empty());
    let s = p.battle().presented(0).unwrap().screen;
    assert_eq!((s.selection(), s.form), (&[0u8][..], None), "{label}: the Cross un-chosen, the pick kept");
    assert_eq!(p.battle().form_list(0).and_then(|l| l.chosen), None);
}

/// The places two frames of the same battle differ: their pixels' (a pixel each) and their text items' (each one's
/// box, in either frame and not the other).
fn differences(a: &nettai_render::Frame, b: &nettai_render::Frame) -> Vec<Rect> {
    let w = a.size.width;
    let mut out: Vec<Rect> = (0..a.pixels.len())
        .filter(|&i| a.pixels[i] != b.pixels[i])
        .map(|i| Rect::new((i % w) as i32, (i / w) as i32, 1, 1))
        .collect();
    let only = |x: &nettai_render::Frame, y: &nettai_render::Frame| -> Vec<Rect> {
        x.text.iter().filter(|t| !y.text.contains(t)).map(|t| t.rect).collect()
    };
    out.extend(only(a, b));
    out.extend(only(b, a));
    out
}

/// The cursor on a pick by pointer (the user: "when mousing over the stack of chips you should show the cursor
/// there too"; now a press on the pick puts it there): drawn about the pick's icon, inside the pick's box, the slot's cursor gone; nothing else of the
/// frame changes. Each game, at both sizes.
#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn the_cursor_on_a_pick_is_drawn_there() {
    for (name, file) in [("exe6", "custom_pointer/m6.toml"), ("exe5", "widescreen/m5.toml"), ("exe4", "widescreen/m4.toml")] {
        let game = game(name);
        for size in SIZES {
            let label = format!("{name} at {}x{}", size.0, size.1);
            let mut p = player(&game, file);
            to_choosing(&mut p, size, &label);
            click_on(&mut p, Target::Slot(0));
            p.tick(Keys::empty());
            draw(&mut p, size);
            let pick = box_of(&mut p, Target::Pick(0));
            let cursor = p.layers()[0].unwrap().cursor;
            let slot = box_of(&mut p, Target::Slot(cursor));
            let on_slot = *p.layers()[0].unwrap();
            let mut on_pick = on_slot;
            on_pick.pick_cursor = Some(0);
            let battle = p.battle().clone();
            let a = p.renderer().render(&battle, [Some(&on_slot), None]);
            let b = p.renderer().render(&battle, [Some(&on_pick), None]);
            let near = |r: Rect, o: Rect| r.x < o.x + o.w + 4 && o.x - 4 < r.x + r.w && r.y < o.y + o.h + 4 && o.y - 4 < r.y + r.h;
            let changed = differences(&a, &b);
            assert!(changed.iter().any(|&r| near(r, pick)), "{label}: the cursor drawn at the pick {pick:?}");
            let stray: Vec<Rect> = changed.into_iter().filter(|&r| !near(r, pick) && !near(r, slot)).collect();
            assert!(stray.is_empty(), "{label}: only the cursor's places change, not {stray:?}");
        }
    }
}

/// A press held still on a chip (a finger's, the mouse's alike) describes it.
#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn a_long_press_describes() {
    let game = game("exe6");
    let mut p = player(&game, "custom_pointer/m6.toml");
    to_choosing(&mut p, SIZES[0], "exe6");
    let chip = middle(box_of(&mut p, Target::Slot(2)));
    let e = |kind| PointerEvent { kind, button: PointerButton::Primary, x: chip.0, y: chip.1 };
    p.point(e(PointerKind::Down));
    let described = (0..nettai_frontend::pointer::LONG_PRESS + 5).any(|_| {
        p.tick(Keys::empty());
        p.layers()[0].unwrap().chatbox().is_some()
    });
    assert!(described, "a long press describes");
    assert_eq!(p.layers()[0].unwrap().cursor, 2, "the press put the cursor there");
    p.point(e(PointerKind::Up));
    p.tick(Keys::empty());
    assert!(p.battle().presented(0).unwrap().screen.selection().is_empty(), "its letting go picks nothing");
}

/// The mouse moved off the targets (the user: "when the mouse cursor isn't hovering over anything on the custom screen
/// the selection should be empty"; "can you blank it"): the cursor on nothing, none drawn, no element icon, the chip
/// window empty; back over a chip, all of them again. Each game, at both sizes.
#[test]
#[ignore = "requires the original ROMs: $NETTAI_ROMS, else the Legacy Collection installed through Steam"]
fn the_pointer_away_shows_nothing_selected() {
    for (name, file) in [("exe6", "custom_pointer/m6.toml"), ("exe5", "widescreen/m5.toml"), ("exe4", "widescreen/m4.toml")] {
        let game = game(name);
        for size in SIZES {
            let label = format!("{name} at {}x{}", size.0, size.1);
            let mut p = player(&game, file);
            to_choosing(&mut p, size, &label);
            let chip = middle(box_of(&mut p, Target::Slot(0)));
            let off = (size.0 as f32 - 2.0, size.1 as f32 - 2.0);
            assert_eq!(p.renderer().target_at(0, off.0 as i32, off.1 as i32), None, "{label}: nothing at {off:?}");
            let moved = |p: &mut Player, (x, y): (f32, f32)| {
                p.point(PointerEvent { kind: PointerKind::Moved, button: PointerButton::Primary, x, y });
                p.tick(Keys::empty());
                draw(p, size);
            };
            moved(&mut p, chip);
            let on = *p.layers()[0].unwrap();
            assert!(!on.away && !on.chip_window.empty && on.look.drawn.cursor.is_some(), "{label}: over a chip");
            let on_frame = p.frame();
            moved(&mut p, off);
            let away = *p.layers()[0].unwrap();
            assert!(away.away && away.chip_window.empty, "{label}: away");
            assert_eq!((away.look.drawn.cursor, away.look.drawn.element), (None, false), "{label}: no cursor, no element icon");
            // The chip window empty: the same layer with the chip's window draws another.
            let battle = p.battle().clone();
            let mut shown = away;
            shown.chip_window = on.chip_window;
            let a = p.renderer().render(&battle, [Some(&away), None]);
            let b = p.renderer().render(&battle, [Some(&shown), None]);
            assert!(!differences(&a, &b).is_empty(), "{label}: the chip window emptied");
            assert!(!differences(&on_frame, &a).is_empty(), "{label}: the frame changed");
            moved(&mut p, chip);
            let back = *p.layers()[0].unwrap();
            assert!(!back.away && !back.chip_window.empty && back.look.drawn.cursor.is_some(), "{label}: over a chip again");
        }
    }
}
