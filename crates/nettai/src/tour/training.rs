//! The screenshot pass's steps for Training (docs/app.md §13): the entry (with no build, with builds, the focus a
//! control apart, a build changed, the builds' list, a list of more builds than it shows at once, a build that
//! can't be used), the two cards, the battle, the
//! training menu (a value changed, the focus a row apart, each Battle row a value apart, the input display, a
//! recording made and played, the rows another screen adds), and the motion probe's bursts of what Training
//! moves.
//!
//! The pass has no builds of the player's to go by (its pictures don't depend on what they have made): the
//! entry lists a new build of each of the game's versions and one that can't be used
//! (`crate::screens::training`, while the app is held still).

use super::{Burst, SETTLE, Step, Tour, compare, flush_burst, write};
use crate::input::Action;
use crate::nav::{Key, Rect};
use crate::screens::ScreenId;
use crate::screens::training;
use crate::shell::App;
use nettai_battle::input::keys;
use nettai_ui::Theme;
use slint::ComponentHandle;
use std::time::Instant;

/// What a shot may differ from the one kept before it in.
#[derive(Clone, Copy, PartialEq)]
enum Differs {
    /// It isn't compared.
    Unheld,
    /// Inside the control the cursor was on and the one it is on.
    Cursors,
    /// That, and what shows the builds chosen: the cards and the arena.
    Builds,
}

/// Where the entry shows the builds chosen, apart from its rows: beside the rows' column, or on a phone between
/// the rows and START TRAINING (in the UI's units, from where those controls are).
fn builds_shown(app: &App) -> Option<Rect> {
    let registry = app.shell.nav.registry();
    let registry = registry.borrow();
    let (start, dummys) = (registry.rect(&Key::new("start", 0))?, registry.rect(&Key::new("dummys", 0))?);
    // (`compare` lets a pair differ 12 around each: these stop 13 short of the controls.)
    Some(if app.shell.scale.phone {
        Rect { x: 0.0, y: dummys.bottom() + 13.0, w: 10_000.0, h: start.y - dummys.bottom() - 26.0 }
    } else {
        Rect { x: start.right() + 13.0, y: dummys.y - 56.0, w: 10_000.0, h: start.bottom() - dummys.y + 56.0 }
    })
}

/// A shot, as `tour::walk`'s: the texts' check started again, the picture written; compared with the shot kept
/// before it (`differs`), and kept for the next (`keep`).
fn shot(add: &mut dyn FnMut(u64, Step), name: &'static str, lang: &'static str, size: &'static str, differs: Differs, keep: bool) {
    let tag = format!("{name}-{lang}-{size}");
    let t = tag.clone();
    add(
        SETTLE,
        Box::new(move |ui, _, tour| {
            *tour.tag.borrow_mut() = t.clone();
            ui.global::<Theme>().set_audit(false);
        }),
    );
    add(90, Box::new(|ui, _, _| ui.global::<Theme>().set_audit(true)));
    add(
        90,
        Box::new(move |ui, app, tour| {
            ui.global::<Theme>().set_audit(false);
            let Some(picture) = write(ui, tour, &tag) else { return };
            let cursor = app.borrow().shell.nav.cursor();
            if differs != Differs::Unheld
                && let Some((first, before, was)) = tour.first.borrow_mut().take()
            {
                let also = if differs == Differs::Builds { builds_shown(&app.borrow()) } else { None };
                compare(ui, tour, &first, &before, &tag, &picture, &[was, cursor, also]);
            }
            if keep {
                *tour.first.borrow_mut() = Some((tag.clone(), picture, cursor));
            }
        }),
    );
}

/// What the walk saw of the battle that no picture says (a navi's HP, the gauge, a deal, the dummy's buttons), in
/// `training.txt` in the pass's folder: said, not checked.
fn say(tour: &Tour, said: &str) {
    use std::io::Write;
    eprint!("tour: {said}");
    if let Ok(mut file) = std::fs::File::options().create(true).append(true).open(tour.dir.join("training.txt")) {
        let _ = file.write_all(said.as_bytes());
    }
}

/// The training battle's state for `training.txt`: each navi's HP, the custom gauge, and what the custom screens
/// dealt (each side's first chips, by key) beside the top of its folder as saved.
fn state(app: &App, lang: &str, size: &str, when: &str) -> String {
    let Some(b) = app.shell.battle.as_ref() else { return format!("{lang} {size} {when}: no battle\n") };
    let battle = b.stage.player.battle();
    let hp = |side: u8| battle.player(side).map_or((0, 0), |r| (battle.objects.get(r).hp, battle.objects.get(r).max_hp));
    let key = |c: nettai_battle::custom::FolderChip| format!("{}:{}", battle.content.defs.chip(c.id).key, c.code.0);
    let mut out = format!(
        "{lang} {size} {when}: tick {}, HP {:?} {:?}, gauge {:#x}{}",
        b.stage.player.ticks(),
        hp(0),
        hp(1),
        battle.gauge.value,
        if battle.round.flags & nettai_battle::battle::battle_flags::GAUGE_FULL != 0 { " (full)" } else { "" },
    );
    if let Some(live) = b.stage.player.live() {
        out += &format!(", options {:?}, dummy {:?}, recorded {} ticks{}", live.training(), live.opponent().in_fight, live.recorded(), if live.recording_now() { " (recording)" } else { "" });
        for side in 0..2 {
            let s = &battle.custom.sides[side];
            if let Some(screen) = s.screen.as_ref().filter(|_| battle.round.mode == nettai_battle::battle::mode::CUSTOM) {
                let dealt: Vec<String> = screen.dealt(&s.folder).map(|(_, c)| key(c)).collect();
                let top: Vec<String> = live.set().folders()[side].chips.iter().take(dealt.len()).map(|c| key(*c)).collect();
                out += &format!("\n  side {side} dealt {dealt:?}\n  its folder's top {top:?}");
            }
        }
    }
    out + "\n"
}

/// A step that says the battle's state (`state`).
fn tell(lang: &'static str, size: &'static str, when: &'static str) -> Step {
    Box::new(move |_, app, tour| say(tour, &state(&app.borrow(), lang, size, when)))
}

fn act(action: Action) -> Step {
    Box::new(move |_, app, _| app.borrow_mut().act(action))
}

fn focus(id: &'static str, index: i32) -> Step {
    Box::new(move |_, app, _| app.borrow_mut().shell.nav.focus(id, index))
}

/// What the pass has changed of Training's remembered choices, forgotten: each walk starts from none.
fn forget(app: &mut App) {
    app.shell.settings.remembered.retain(|key, _| !key.starts_with("training."));
}

/// Training's steps of one language at one size.
pub(super) fn steps(add: &mut dyn FnMut(u64, Step), lang: &'static str, size: &'static str) {
    // The entry with no build of the game: the one button.
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            forget(&mut app);
            app.shell.training.tour_no_builds = true;
            app.go_now(ScreenId::Training);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    shot(add, "training-none", lang, size, Differs::Unheld, false);

    // The entry: the focus a control up from START TRAINING; the dummy's build stepped to the next; the builds'
    // list; a build that can't be used.
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.training.tour_no_builds = false;
            app.with_screen(ScreenId::Training, |screen, shell| screen.show(shell));
            app.shell.nav.focus("start", 0);
        }),
    );
    shot(add, "training-entry", lang, size, Differs::Unheld, true);
    add(SETTLE, act(Action::Up));
    shot(add, "training-entry-focus", lang, size, Differs::Cursors, true);
    add(SETTLE, act(Action::Right));
    shot(add, "training-entry-dummy", lang, size, Differs::Builds, false);
    add(SETTLE, act(Action::Confirm));
    shot(add, "training-entry-list", lang, size, Differs::Unheld, false);
    add(SETTLE, act(Action::Back));
    // A list of more choices than it shows at once (the shell's, for any select: twenty-four names where the
    // builds' three were): it opens with the chosen one in view, the focus moves a row without the list
    // moving, and its last row is scrolled to in a list of the same height.
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.pick("dummys", 0, (1..=24).map(|n| format!("Build {n:02}")).collect(), 17)));
    shot(add, "training-entry-long-list", lang, size, Differs::Unheld, true);
    add(SETTLE, act(Action::Up));
    shot(add, "training-entry-long-list-focus", lang, size, Differs::Cursors, false);
    add(SETTLE, act(Action::End));
    shot(add, "training-entry-long-list-end", lang, size, Differs::Unheld, false);
    add(SETTLE, act(Action::Back));
    add(SETTLE, act(Action::Right));
    shot(add, "training-entry-unusable", lang, size, Differs::Unheld, false);
    add(SETTLE, act(Action::Left));

    // START TRAINING: the two cards, then the battle.
    add(SETTLE, focus("start", 0));
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.hold_cards = true;
            app.act(Action::Confirm);
        }),
    );
    shot(add, "training-versus", lang, size, Differs::Unheld, false);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.hold_cards = false));
    add(4000, Box::new(|_, _, _| {}));
    shot(add, "training-battle", lang, size, Differs::Unheld, false);

    // The training menu: a value changed under the focus, the focus a row down, the input display turned on.
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.open_menu()));
    shot(add, "training-menu", lang, size, Differs::Unheld, true);
    add(SETTLE, act(Action::Right));
    shot(add, "training-menu-value", lang, size, Differs::Cursors, true);
    add(SETTLE, act(Action::Down));
    shot(add, "training-menu-focus", lang, size, Differs::Cursors, false);
    // Each of the Battle rows, and its value changed (at once, on the battle behind the menu: the HP refilled,
    // the gauge full, the folders put in their own order). (Their places in the menu: `training::ROWS`'s.)
    for (name, changed, index) in [
        ("training-menu-your-hp", "training-menu-your-hp-value", 5),
        ("training-menu-dummys-hp", "training-menu-dummys-hp-value", 6),
        ("training-menu-gauge", "training-menu-gauge-value", 7),
        ("training-menu-draw", "training-menu-draw-value", 8),
    ] {
        add(SETTLE, focus("menu.rows.option", index));
        shot(add, name, lang, size, Differs::Unheld, true);
        add(SETTLE, act(Action::Right));
        shot(add, changed, lang, size, Differs::Cursors, false);
    }
    // The input display, the menu's last row (on a phone, the rows scrolled to it).
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.nav.focus("menu.rows.option", 10);
            app.act(Action::Right);
        }),
    );
    shot(add, "training-menu-inputs", lang, size, Differs::Unheld, false);
    // (The display over the battle, a few buttons held.)
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.close_menu();
            app.shell.keys.held = keys::LEFT | keys::B | keys::R;
        }),
    );
    shot(add, "training-inputs", lang, size, Differs::Unheld, false);

    // What the Battle rows set, driven and drawn: the dummy shot (a charged shot and three more), its HP down;
    // a rest, its HP full again; the gauge full the while; L, and the custom screen deals the folder from its top.
    let hold = |held: u16| -> Step { Box::new(move |_, app, _| app.borrow_mut().shell.keys.held = held) };
    add(SETTLE, hold(keys::B));
    add(1500, hold(0));
    for _ in 0..3 {
        add(250, hold(keys::B));
        add(120, hold(0));
    }
    add(150, tell(lang, size, "after the shots"));
    shot(add, "training-refill-hit", lang, size, Differs::Unheld, false);
    add(2000, tell(lang, size, "after a rest"));
    shot(add, "training-refill-full", lang, size, Differs::Unheld, false);
    add(SETTLE, hold(keys::L));
    add(150, hold(0));
    add(1200, tell(lang, size, "the custom screen"));
    shot(add, "training-draw-deal", lang, size, Differs::Unheld, false);
    // (Its OK: START to it, and A.)
    add(SETTLE, hold(keys::START));
    add(200, hold(0));
    add(150, hold(keys::A));
    add(150, hold(0));
    add(2000, tell(lang, size, "the fight again"));

    // A recording: the row, chosen (the menu closes, the player's buttons drive the dummy, RECORDING over the
    // battle), Start (the dummy plays it), and the menu again (the recording's length, the dummy's Action).
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.keys.held = 0;
            app.shell.open_menu();
            app.shell.nav.focus("menu.rows.option", 3);
        }),
    );
    shot(add, "training-menu-recording", lang, size, Differs::Unheld, false);
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.act(Action::Confirm);
            app.shell.keys.held = keys::UP | keys::B;
        }),
    );
    add(1500, Box::new(|_, app, _| app.borrow_mut().shell.keys.held = keys::DOWN));
    add(1000, tell(lang, size, "recording"));
    shot(add, "training-recording", lang, size, Differs::Unheld, false);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.keys.held = keys::START));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.keys.held = 0));
    // (The dummy's buttons, a quarter of a second apart, through two of its loops: what it plays.)
    for _ in 0..16 {
        add(
            250,
            Box::new(move |_, app, tour| {
                let app = app.borrow();
                let Some(b) = app.shell.battle.as_ref() else { return };
                let battle = b.stage.player.battle();
                let at = battle.player(1).map(|r| battle.objects.get(r).panel);
                say(tour, &format!("{lang} {size} playing: tick {}, the dummy holds {:#05x}, on {at:?}\n", b.stage.player.ticks(), battle.custom.sides[1].joypad.held));
            }),
        );
    }
    shot(add, "training-recording-played", lang, size, Differs::Unheld, false);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.open_menu()));
    shot(add, "training-menu-recorded", lang, size, Differs::Unheld, false);

    // CHANGE BUILDS: the entry again.
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.keys.held = 0;
            app.shell.open_menu();
            app.with_screen(ScreenId::Training, |screen, shell| screen.activate(shell, "menu.change", 0));
        }),
    );
    shot(add, "training-entry-again", lang, size, Differs::Unheld, false);

    // A session another screen starts, with rows of its own at the menu's top (Play's, beside a waiting invite:
    // here it fills the window, there being no such screen to show it in).
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            forget(&mut app);
            app.go_now(ScreenId::Play);
            let Some((game, ready)) = app.shell.ready() else { return };
            let Some(build) = training::new_builds(ready.content(), &game).ok().and_then(|b| b.into_iter().next()) else { return };
            let rows = vec!["Copy invite code".to_string(), "Cancel invite".to_string()];
            let start = training::Start { full: true, seed: 7, ..training::Start::beside(&game, build, ScreenId::Play, "tour", rows) };
            let _ = training::start(&mut app.shell, start);
        }),
    );
    add(1500, Box::new(|_, _, _| {}));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.open_menu()));
    shot(add, "training-menu-added", lang, size, Differs::Unheld, false);
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            if training::running(&app.shell) {
                training::end(&mut app.shell);
            }
            app.go_now(ScreenId::Menu);
        }),
    );
}

/// The motion probe's bursts of what Training moves: the entry arriving, a build stepped (the arena follows a
/// moment after), the battle entered from START TRAINING, the training menu sliding in, a row's value sliding.
pub(super) fn motion(add: &mut dyn FnMut(u64, Step)) {
    // (A burst, as `tour::motion_walk`'s: `trigger` starts something moving, and a shot is taken at each of
    // `at`, milliseconds after it.)
    fn burst(add: &mut dyn FnMut(u64, Step), name: &'static str, at: &[u64], trigger: Step) {
        add(
            SETTLE * 2,
            Box::new(move |ui, app, tour: &std::rc::Rc<Tour>| {
                trigger(ui, app, tour);
                *tour.burst.borrow_mut() = Some(Burst { name, began: Instant::now(), shots: Vec::new() });
            }),
        );
        let mut last = 0;
        for &ms in at {
            add(
                ms - last,
                Box::new(|ui, app, tour| {
                    let Ok(picture) = ui.window().take_snapshot() else { return };
                    let sizes = app.borrow().shell.nav.registry().borrow().sizes();
                    if let Some(burst) = tour.burst.borrow_mut().as_mut() {
                        burst.shots.push((burst.began.elapsed().as_millis(), picture, sizes));
                    }
                }),
            );
            last = ms;
        }
        add(10, Box::new(|_, _, tour| flush_burst(tour)));
    }
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            forget(&mut app);
            app.go_now(ScreenId::Menu);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    burst(add, "training-arrives", &[0, 40, 80, 130, 170, 220, 300, 450], Box::new(|_, app, _| app.borrow_mut().go_now(ScreenId::Training)));
    add(SETTLE, focus("dummys", 0));
    burst(add, "training-build-steps", &[0, 40, 90, 150, 200, 260, 400], act(Action::Right));
    // (A long list: opening on a row far down it, then scrolled to its last.)
    burst(
        add,
        "training-list-opens",
        &[0, 40, 90, 150, 220, 320],
        Box::new(|_, app, _| app.borrow_mut().shell.pick("dummys", 0, (1..=24).map(|n| format!("Build {n:02}")).collect(), 17)),
    );
    burst(add, "training-list-scrolls", &[0, 40, 90, 150, 220], act(Action::End));
    add(SETTLE, act(Action::Back));
    add(SETTLE, focus("start", 0));
    burst(add, "training-entered", &[0, 40, 90, 160, 300, 480, 560, 640, 720, 800, 900, 1100], act(Action::Confirm));
    add(1500, Box::new(|_, _, _| {}));
    burst(add, "training-menu-opens", &[0, 40, 90, 160, 260], Box::new(|_, app, _| app.borrow_mut().shell.open_menu()));
    burst(add, "training-menu-value", &[0, 30, 60, 100, 180], act(Action::Right));
    burst(add, "training-menu-closes", &[0, 40, 90, 160], act(Action::Back));
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            forget(&mut app);
            training::end(&mut app.shell);
            app.go_now(ScreenId::Menu);
        }),
    );
}
