//! The screenshot pass's steps for Practice (docs/app.md §13): PLAY's PRACTICE (the two cards, the battle with the
//! practice menu open over it as it starts), the menu's Dummy's build (a value apart, its list, a list of more
//! choices than it shows at once), the battle, the practice menu (a value changed, the focus a row apart, the
//! dummy that fights (its Action, and its Level under it), the arena's grid, the arena, the background and the
//! music changed under it, each Battle row a value apart, the input display, the rows another screen adds), EXIT
//! back to Play, the way to Builds' editor and back (the Dummy's build's EDIT), and the motion probe's bursts of
//! what Practice moves.
//!
//! The pass has no builds of the player's to go by (its pictures don't depend on what they have made): Play lists
//! builds of its own (`crate::screens::play::shelved`, while the app is held still). For the way to Builds it
//! keeps a new build of each version as files in its own builds folder, which Practice then lists as a player's
//! (`Shared::tour_files`).

use super::{Burst, SETTLE, Step, Tour, compare, flush_burst, write};
use crate::input::Action;
use crate::nav::Rect;
use crate::screens::ScreenId;
use crate::screens::practice;
use crate::shell::App;
use nettai_battle::input::keys;
use crate::builds::store;
use nettai_ui::{AppWindow, BuildsView, PracticeOption, Theme};
use slint::ComponentHandle;
use std::time::Instant;

/// What a shot may differ from the one kept before it in.
#[derive(Clone, Copy, PartialEq)]
enum Differs {
    /// It isn't compared.
    Unheld,
    /// Inside the control the cursor was on and the one it is on, and the game's own picture where the practice
    /// menu leaves it in view: the battle is held behind the menu, and its scenery (the background, the
    /// panels' own animations) moves on through the hold as the game's pause moves it.
    Picture,
}

/// Where the game's picture is in view beside the practice menu's panel, each box given exactly (`compare` lets a
/// pair differ 12 around each box it is given: these are 12 inside), in the UI's units. The panel is the menu's
/// (`MenuOverlay`, at the side: the theme's margin from the left, 24 from the top and bottom, at most 500 wide),
/// over everything under it: none of the picture under the panel is let differ, but where its cut corner shows it.
fn picture_beside_menu(ui: &AppWindow) -> Vec<Option<Rect>> {
    let shell = ui.global::<nettai_ui::Shell>();
    let picture = Rect {
        x: shell.get_picture_room_x() + shell.get_picture_x(),
        y: shell.get_picture_room_y() + shell.get_picture_y(),
        w: shell.get_picture_width(),
        h: shell.get_picture_height(),
    };
    let factor = ui.window().scale_factor();
    let window = ui.window().size();
    let (width, height) = (window.width as f32 / factor, window.height as f32 / factor);
    let margin = ui.global::<Theme>().get_margin();
    let panel = Rect { x: margin, y: 24.0, w: (width - 2.0 * margin).min(500.0), h: height - 48.0 };
    let exactly = |l: f32, t: f32, r: f32, b: f32| (r > l && b > t).then_some(Rect { x: l + 12.0, y: t + 12.0, w: r - l - 24.0, h: b - t - 24.0 });
    let (l, t, r, b) = (picture.x, picture.y, picture.right(), picture.bottom());
    let (inner_l, inner_r) = (l.max(panel.x), r.min(panel.right()));
    vec![
        // Left and right of the panel, the picture's whole height; above and below it, the panel's width.
        exactly(l, t, r.min(panel.x), b),
        exactly(l.max(panel.right()), t, r, b),
        exactly(inner_l, t, inner_r, b.min(panel.y)),
        exactly(inner_l, t.max(panel.bottom()), inner_r, b),
        // (And through the panel's cut corner, 18 at its bottom right: a dialog's or a menu's.)
        exactly(panel.right() - 18.0, panel.bottom() - 18.0, panel.right(), panel.bottom()),
    ]
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
                let mut allowed = vec![was, cursor];
                if differs == Differs::Picture {
                    allowed.extend(picture_beside_menu(ui));
                }
                compare(ui, tour, &first, &before, &tag, &picture, &allowed);
            }
            if keep {
                *tour.first.borrow_mut() = Some((tag.clone(), picture, cursor));
            }
        }),
    );
}

/// What the walk saw of the battle that no picture says (a navi's HP, the gauge, a deal, the dummy's buttons), in
/// `practice.txt` in the pass's folder: said, not checked.
fn say(tour: &Tour, said: &str) {
    use std::io::Write;
    eprint!("tour: {said}");
    if let Ok(mut file) = std::fs::File::options().create(true).append(true).open(tour.dir.join("practice.txt")) {
        let _ = file.write_all(said.as_bytes());
    }
}

/// The practice battle's state for `practice.txt`: each navi's HP, the custom gauge, and what the custom screens
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
        out += &format!(", options {:?}, dummy {:?}", live.training(), live.opponent().in_fight);
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

/// The focus on the practice menu's row of `option`, where the menu shows it as the step runs (the dummy's
/// Action has rows under it for some of its values).
fn focus_row(option: PracticeOption) -> Step {
    Box::new(move |_, app, _| {
        let mut app = app.borrow_mut();
        let at = practice::row_of(&app.shell, option).expect("a row the menu shows");
        app.shell.nav.focus("menu.rows.option", at);
    })
}

/// The pass's own builds of the selected game, gone (the folder is the pass's: `paths::keep_builds_in`).
fn clear_builds(app: &App) {
    if let Some(game) = app.shell.game() {
        let _ = std::fs::remove_dir_all(store::folder(game));
    }
}

/// A step that checks the walk is where its next picture says it is: if it isn't, the report says so (as a text
/// the pass counts against the picture, so the pass fails), a picture of something else passing for it otherwise.
fn expect(is: impl Fn(&App, &AppWindow) -> bool + 'static, what: &'static str) -> Step {
    Box::new(move |ui, app, _| {
        if !is(&app.borrow(), ui) {
            ui.global::<nettai_ui::Audit>().invoke_clipped(format!("(the walk isn't at {what}: the next picture is of something else)").into());
        }
    })
}

/// What the pass has changed of Practice's choices, forgotten, and the visit over: each walk starts from none, on
/// the defaults.
pub(super) fn forget(app: &mut App) {
    app.shell.practice.visit.leave();
}

/// PLAY's PRACTICE pressed, the cards held: Play shown, its first build chosen (the pass's own, or with `files` the
/// builds folder's), and PRACTICE confirmed.
fn practice_from_play(app: &mut App, files: bool) {
    forget(app);
    app.shell.practice.tour_files = files;
    app.go_now(ScreenId::Play);
    app.shell.nav.show_cursor(true, false);
    app.shell.hold_cards = true;
    app.shell.nav.focus("practice", 0);
    app.act(Action::Confirm);
}

/// Practice's steps of one language at one size.
pub(super) fn steps(add: &mut dyn FnMut(u64, Step), lang: &'static str, size: &'static str) {
    // PLAY's PRACTICE: straight into a practice battle on the build chosen there, the two cards first; then the
    // battle with the practice menu open over it as it starts, the focus on RESUME.
    add(SETTLE, Box::new(|_, app, _| practice_from_play(&mut app.borrow_mut(), false)));
    add(SETTLE, expect(|app, _| app.shell.screen() == ScreenId::Practice && practice::running(&app.shell), "Practice, its session on, from PLAY's PRACTICE"));
    shot(add, "practice-versus", lang, size, Differs::Unheld, false);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.hold_cards = false));
    add(4000, Box::new(|_, _, _| {}));
    add(
        SETTLE,
        expect(
            |app, _| app.shell.battle.as_ref().is_some_and(|b| b.menu_open()) && app.shell.nav.focused().is_some_and(|k| k.id == "menu.resume"),
            "the practice menu open as the battle starts, on RESUME",
        ),
    );
    shot(add, "practice-menu-start", lang, size, Differs::Unheld, false);

    // The Dummy's build: its row, a step along (the set starts over with the dummy on the next build, the menu
    // staying open), its list, and back to the player's own. A list of more choices than it shows at once (the
    // shell's, for any select: twenty-four names where the builds' few were): it opens with the chosen one in view,
    // the focus moves a row without the list moving, and its last row is scrolled to in a list of the same height.
    add(SETTLE, focus_row(PracticeOption::DummysBuild));
    shot(add, "practice-menu-dummys-build", lang, size, Differs::Unheld, true);
    // (A select: Enter opens its list, the next chosen from it.)
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Down));
    add(SETTLE, act(Action::Confirm));
    add(2500, Box::new(|_, _, _| {}));
    shot(add, "practice-menu-dummys-build-value", lang, size, Differs::Unheld, false);
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, expect(|_, ui| ui.global::<nettai_ui::Shell>().get_picker(), "the Dummy's build's list"));
    shot(add, "practice-menu-dummys-build-list", lang, size, Differs::Unheld, false);
    add(SETTLE, act(Action::Up));
    add(SETTLE, act(Action::Confirm));
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            let at = practice::row_of(&app.shell, PracticeOption::DummysBuild).expect("the Dummy's build's row");
            app.shell.pick("menu.rows.option", at, (1..=24).map(|n| format!("Build {n:02}")).collect(), 17);
        }),
    );
    shot(add, "practice-menu-long-list", lang, size, Differs::Unheld, true);
    add(SETTLE, act(Action::Up));
    shot(add, "practice-menu-long-list-focus", lang, size, Differs::Picture, false);
    add(SETTLE, act(Action::End));
    shot(add, "practice-menu-long-list-end", lang, size, Differs::Unheld, false);
    add(SETTLE, act(Action::Back));

    // RESUME: the battle.
    add(SETTLE, focus("menu.resume", 0));
    add(SETTLE, act(Action::Confirm));
    add(2500, Box::new(|_, _, _| {}));
    shot(add, "practice-battle", lang, size, Differs::Unheld, false);

    // The practice menu: a value changed under the focus, the focus a row down, the input display turned on. (The
    // game's picture beside the menu may differ in each pair: it is held, and its scenery moves on.)
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.open_menu()));
    // (It opens on RESUME: the focus is put on a row.)
    add(SETTLE, focus_row(PracticeOption::Action));
    // (A setting is taken to change: Enter, then the arrows; taken before the pair's first.)
    add(SETTLE, act(Action::Confirm));
    shot(add, "practice-menu", lang, size, Differs::Unheld, true);
    add(SETTLE, act(Action::Right));
    shot(add, "practice-menu-value", lang, size, Differs::Picture, true);
    add(SETTLE, act(Action::Down));
    shot(add, "practice-menu-focus", lang, size, Differs::Picture, false);
    // The dummy that fights: its Action Fights (one on from Moves around), and the Level row come under it (the
    // rows below it a row down: not compared); its Level Hard, the row a value apart. Then its Action back as it
    // was, and the Level row gone with it.
    add(SETTLE, focus_row(PracticeOption::Action));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Right));
    shot(add, "practice-menu-fights", lang, size, Differs::Unheld, false);
    add(SETTLE, focus_row(PracticeOption::Level));
    add(SETTLE, act(Action::Confirm));
    shot(add, "practice-menu-level", lang, size, Differs::Unheld, true);
    add(SETTLE, act(Action::Right));
    shot(add, "practice-menu-level-hard", lang, size, Differs::Picture, false);
    add(SETTLE, focus_row(PracticeOption::Action));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Left));
    shot(add, "practice-menu-moves-again", lang, size, Differs::Unheld, false);
    // The Arena row, and its value changed: the set starts over on the next arena behind the menu, which stays
    // open (the battle through the round's opening to its custom screen, where it waits).
    add(SETTLE, focus_row(PracticeOption::Arena));
    shot(add, "practice-menu-arena", lang, size, Differs::Unheld, false);
    // Its grid (the user: "arena selector should be like a dropdown that shows previews of all arenas"), open at
    // the arena chosen; and the cursor a cell on.
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, expect(|_, ui| ui.global::<nettai_ui::Shell>().get_picker_grid(), "the menu's arena grid"));
    shot(add, "practice-menu-arena-grid", lang, size, Differs::Unheld, true);
    add(SETTLE, act(Action::Right));
    shot(add, "practice-menu-arena-grid-focus", lang, size, Differs::Picture, false);
    add(SETTLE, act(Action::Confirm));
    add(2500, Box::new(|_, _, _| {}));
    shot(add, "practice-menu-arena-value", lang, size, Differs::Unheld, false);
    // The Background row, and its value changed: the battle goes on where it is, drawn on the next background
    // (the field, the HP and the round as they were).
    add(SETTLE, focus_row(PracticeOption::Background));
    shot(add, "practice-menu-background", lang, size, Differs::Unheld, false);
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Right));
    add(SETTLE, act(Action::Confirm));
    shot(add, "practice-menu-background-value", lang, size, Differs::Unheld, false);
    // Its grid, open at the background chosen; the one beside it chosen from it, the battle drawn on that one.
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, expect(|_, ui| ui.global::<nettai_ui::Shell>().get_picker_grid(), "the menu's background grid"));
    shot(add, "practice-menu-background-grid", lang, size, Differs::Unheld, false);
    let before = std::rc::Rc::new(std::cell::Cell::new(None));
    let was = before.clone();
    add(SETTLE, Box::new(move |_, app, _| was.set(practice::session_place(&app.borrow().shell))));
    add(SETTLE, act(Action::Right));
    add(SETTLE, act(Action::Confirm));
    add(
        SETTLE,
        expect(
            move |app, ui| !ui.global::<nettai_ui::Shell>().get_picker() && practice::session_place(&app.shell).zip(before.get()).is_some_and(|(now, was)| now[1] == was[1] + 1),
            "the background beside it chosen from the menu's grid",
        ),
    );
    shot(add, "practice-menu-background-chosen", lang, size, Differs::Unheld, false);
    // The Music row, and its value changed: the battle's music switched at once (nothing drawn changes but the
    // row's value); then the game's own again. Its list, open at the game's own, every game's themes in it once
    // each game's parts are read.
    add(SETTLE, Box::new(|_, _, tour| tour.wait_until(|app| app.shell.games.playable().iter().all(|id| app.shell.games.read(id).is_some()))));
    add(SETTLE, focus_row(PracticeOption::Music));
    shot(add, "practice-menu-music", lang, size, Differs::Unheld, true);
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Down));
    add(SETTLE, act(Action::Confirm));
    shot(add, "practice-menu-music-value", lang, size, Differs::Picture, false);
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Up));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, expect(|_, ui| ui.global::<nettai_ui::Shell>().get_picker(), "the menu's music list"));
    shot(add, "practice-menu-music-list", lang, size, Differs::Unheld, false);
    add(SETTLE, act(Action::Back));
    // Each of the other Battle rows, and its value changed (at once, on the battle behind the menu: the HP
    // refilled, the gauge full, the folders put in their own order).
    for (name, changed, option) in [
        ("practice-menu-your-hp", "practice-menu-your-hp-value", PracticeOption::YourHp),
        ("practice-menu-dummys-hp", "practice-menu-dummys-hp-value", PracticeOption::DummysHp),
        ("practice-menu-gauge", "practice-menu-gauge-value", PracticeOption::CustomGauge),
        ("practice-menu-draw", "practice-menu-draw-value", PracticeOption::ChipDraw),
    ] {
        add(SETTLE, focus_row(option));
        add(SETTLE, act(Action::Confirm));
        shot(add, name, lang, size, Differs::Unheld, true);
        add(SETTLE, act(Action::Right));
        shot(add, changed, lang, size, Differs::Picture, false);
    }
    // The input display, the menu's last row (on a phone, the rows scrolled to it).
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            let at = practice::row_of(&app.shell, PracticeOption::InputDisplay).expect("the input display's row");
            app.shell.nav.focus("menu.rows.option", at);
            app.act(Action::Confirm);
            app.act(Action::Right);
        }),
    );
    shot(add, "practice-menu-inputs", lang, size, Differs::Unheld, false);
    // (The display over the battle, a few buttons held.)
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.close_menu();
            app.shell.keys.held = keys::LEFT | keys::B | keys::R;
        }),
    );
    shot(add, "practice-inputs", lang, size, Differs::Unheld, false);

    // (The dummy stands still for the shots at it below, across from the player: its Action stepped back to the
    // first, and the set restarted from the menu, which puts both where a round starts them; then the round's
    // opening, to its custom screen. Moving around, it could be out of the shots' row. The menu's rows are there
    // a step after it opens.)
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.keys.held = 0;
            app.shell.open_menu();
        }),
    );
    add(SETTLE, focus_row(PracticeOption::Action));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Left));
    add(SETTLE, focus("menu.restart", 0));
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            app.borrow_mut().act(Action::Confirm);
            tour.wait_until(|app| app.shell.battle.as_ref().is_some_and(|b| b.stage.player.battle().round.mode == nettai_battle::battle::mode::CUSTOM));
        }),
    );

    // Through the custom screen to the fight (START to OK, A on it), and the menu opened there: the battle held
    // behind it, no PAUSED banner, and a moment later still held, its background moved on.
    for n in 0..16 {
        add(150, Box::new(move |_, app, _| app.borrow_mut().shell.keys.held = if n % 2 == 0 { keys::START } else { keys::A }));
    }
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.keys.held = 0));
    add(1500, Box::new(|_, app, _| app.borrow_mut().shell.open_menu()));
    shot(add, "practice-menu-paused", lang, size, Differs::Unheld, false);
    add(700, Box::new(|_, _, _| {}));
    shot(add, "practice-menu-paused-later", lang, size, Differs::Unheld, false);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.close_menu()));
    shot(add, "practice-resumed", lang, size, Differs::Unheld, false);

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
    shot(add, "practice-refill-hit", lang, size, Differs::Unheld, false);
    add(2000, tell(lang, size, "after a rest"));
    shot(add, "practice-refill-full", lang, size, Differs::Unheld, false);
    add(SETTLE, hold(keys::L));
    add(150, hold(0));
    add(1200, tell(lang, size, "the custom screen"));
    shot(add, "practice-draw-deal", lang, size, Differs::Unheld, false);
    // (Its OK: START to it, and A.)
    add(SETTLE, hold(keys::START));
    add(200, hold(0));
    add(150, hold(keys::A));
    add(150, hold(0));
    add(2000, tell(lang, size, "the fight again"));

    // EXIT: Play again, the build still chosen.
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.keys.held = 0;
            app.shell.open_menu();
        }),
    );
    // (The menu's controls are there a step after it opens.)
    add(SETTLE, focus("menu.exit", 0));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, expect(|app, _| app.shell.screen() == ScreenId::Play && !practice::running(&app.shell), "Play, back from Practice's EXIT"));
    shot(add, "practice-exit", lang, size, Differs::Unheld, false);

    // EDIT under the Dummy's build: the builds folder listing the pass's own files (a new build of each version,
    // kept as a player's are), PLAY's PRACTICE on the first; the dummy on the second; its EDIT focused; the Builds
    // editor it opens on that build, Back saying PRACTICE; and Back, the practice battle again with the menu open
    // over it, the dummy still on that build.
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            let Some((game, ready)) = app.shell.ready() else { return };
            clear_builds(&app);
            let content = ready.content();
            for b in practice::new_builds(content, &game).unwrap_or_default() {
                let build = store::Build::new(content, &b.name, b.side);
                if let Err(why) = store::write(&store::new_path(&game, &b.name), content, &game, &build) {
                    crate::errors::log("keeping the pass's build", &why);
                }
            }
            let first = store::list(&game).first().map(|b| b.name.clone()).unwrap_or_default();
            forget(&mut app);
            app.shell.practice.tour_files = true;
            app.shell.hold_cards = false;
            practice::open(&mut app.shell, &game, &first);
            app.go_now(ScreenId::Practice);
        }),
    );
    add(4000, Box::new(|_, _, _| {}));
    add(SETTLE, focus_row(PracticeOption::DummysBuild));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Down));
    add(SETTLE, act(Action::Confirm));
    add(2500, Box::new(|_, _, _| {}));
    add(SETTLE, focus_row(PracticeOption::EditDummy));
    shot(add, "practice-menu-edit", lang, size, Differs::Unheld, false);
    add(SETTLE, act(Action::Confirm));
    add(
        SETTLE,
        expect(
            |app, ui| {
                let view = ui.global::<BuildsView>();
                let second = app.shell.game().and_then(|game| store::list(game).get(1).map(|b| b.name.clone())).unwrap_or_default();
                app.shell.screen() == ScreenId::Builds && view.get_to_practice() && view.get_name() == second.as_str()
            },
            "Builds' editor on the dummy's build, Back going to Practice",
        ),
    );
    shot(add, "practice-builds-editor", lang, size, Differs::Unheld, false);
    add(SETTLE, act(Action::Back));
    add(4000, Box::new(|_, _, _| {}));
    add(
        SETTLE,
        expect(
            |app, _| app.shell.screen() == ScreenId::Practice && app.shell.battle.as_ref().is_some_and(|b| b.menu_open()) && practice::dummys_build(&app.shell) == Some(1),
            "the practice battle again, back from the editor, the menu open, the dummy on its build",
        ),
    );
    shot(add, "practice-menu-edited", lang, size, Differs::Unheld, false);
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.go_now(ScreenId::Play);
            clear_builds(&app);
            forget(&mut app);
            app.shell.practice.tour_files = false;
        }),
    );

    // A session another screen starts, with rows of its own at the menu's top (Play's, beside a waiting invite:
    // here it fills the window, there being no such screen to show it in).
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            forget(&mut app);
            app.go_now(ScreenId::Play);
            let Some((game, ready)) = app.shell.ready() else { return };
            let Some(build) = practice::new_builds(ready.content(), &game).ok().and_then(|b| b.into_iter().next()) else { return };
            let rows = vec!["Stop practice".to_string()];
            let start = practice::Start { full: true, seed: 7, exit: Some("Cancel invite".into()), ..practice::Start::beside(&game, build, ScreenId::Play, "tour", rows) };
            let _ = practice::start(&mut app.shell, start);
        }),
    );
    add(1500, Box::new(|_, _, _| {}));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.open_menu()));
    shot(add, "practice-menu-added", lang, size, Differs::Unheld, false);
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            if practice::running(&app.shell) {
                practice::end(&mut app.shell);
            }
            app.go_now(ScreenId::Menu);
        }),
    );
}

/// The motion probe's bursts of what Practice moves: the battle entered from PLAY's PRACTICE (the cards, the
/// jack-in, the menu sliding in as it starts), the practice menu closing and opening, a row's value sliding.
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
            app.go_now(ScreenId::Play);
            app.shell.nav.show_cursor(true, false);
            app.shell.nav.focus("practice", 0);
        }),
    );
    burst(add, "practice-entered", &[0, 40, 90, 160, 300, 480, 560, 640, 720, 800, 900, 1100, 1400, 1700], act(Action::Confirm));
    add(1500, Box::new(|_, _, _| {}));
    burst(add, "practice-menu-closes", &[0, 40, 90, 160], act(Action::Back));
    burst(add, "practice-menu-opens", &[0, 40, 90, 160, 260], Box::new(|_, app, _| app.borrow_mut().shell.open_menu()));
    add(SETTLE, focus_row(PracticeOption::Action));
    add(SETTLE, act(Action::Confirm));
    burst(add, "practice-menu-value", &[0, 30, 60, 100, 180], act(Action::Right));
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            forget(&mut app);
            practice::end(&mut app.shell);
            app.go_now(ScreenId::Menu);
        }),
    );
}
