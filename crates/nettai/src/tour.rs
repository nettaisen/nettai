//! The screenshot pass (`NETTAI_TOUR=<folder>`; docs/app.md §13): the app walks its own screens, in each
//! language bundled and at a desktop's and an upright phone's size, and writes the window as each shows to a PNG
//! (`Window::take_snapshot`: the window's own renderer's picture). The pictures are for comparing with the
//! mockups by eye. Two things it checks itself, writes to `report.txt` in the folder, and fails on (the
//! process ends with status 1):
//!
//! - **Nothing shifts.** Some shots come in pairs a step apart: the focus moved to the next control, or a value
//!   changed under it. The two pictures may differ only inside the controls the step touched (and the cursor's
//!   brackets around them); a pixel that differs anywhere else is something that moved.
//! - **Nothing is cut.** While it runs, every label, button and heading says when its text doesn't fit its
//!   box (the UI's `Audit`). The pseudo-locale makes every string about 40% longer, so a layout sized for
//!   English alone shows up there.
//!
//! The app is held still for it: no animation, no sound device, the menu's reel stopped on one picture, the
//! settings at their defaults and never written. `NETTAI_TOUR_LANGS=en,ja,pseudo` picks the languages,
//! `NETTAI_TOUR_SIZES=desktop,phone` the sizes, `NETTAI_TOUR_GAME=<game>` the game selected.
//!
//! **The motion probe** (`NETTAI_TOUR_MOTION=1`, with the same folder) is the pass with things moving: a
//! still picture can't show an animation, so it takes a burst of shots through each (a screen leaving and the
//! next arriving, the cursor's glide, a value sliding, a press, a list opening, a page scrolling, a dialog,
//! the cards and the wipe into a battle, a menu over it), each named for the milliseconds since the animation
//! began. Two things it checks itself and fails on:
//!
//! - **Motion changes no size.** In every shot of a burst, each control on screen is the size it has in the
//!   burst's last, when everything has come to rest. (Where it is may differ: that is what moves.)
//! - **A press acts at once.** A move pressed in the same instant as a screen change lands on the new screen's
//!   controls, and a direction held through a scrolling page moves the focus at every repeat, whatever is
//!   still sliding.

use crate::errors::UserError;
use crate::input::Action;
use crate::nav::{Key, Rect};
use crate::screens::ScreenId;
use crate::shell::App;
use nettai_ui::{AppWindow, Audit, BattleKind, Theme};
use slint::{ComponentHandle, PhysicalSize, Rgba8Pixel, SharedPixelBuffer, Timer};
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

// A screen's own steps (`<screen>::steps`, and `<screen>::motion` for the motion probe), each in a file of its
// own, called from `walk` and `motion_walk` with one line.
mod training;

/// How long a step is given to be drawn before the next: a few frames.
const SETTLE: u64 = 260;

/// The sizes walked, in points: the reference window, and a phone held upright.
const SIZES: [(&str, u32, u32); 2] = [("desktop", 1280, 800), ("phone", 390, 800)];

/// What the pass keeps as it goes.
struct Tour {
    dir: PathBuf,
    /// The shot being arranged (`menu-en-desktop`): what a cut text is reported under.
    tag: RefCell<String>,
    /// The first shot of a pair, and where the cursor was in it.
    first: RefCell<Option<(String, SharedPixelBuffer<Rgba8Pixel>, Option<Rect>)>>,
    /// The texts cut, each with the shot it was cut in.
    cut: RefCell<BTreeSet<(String, String)>>,
    /// The pairs compared, and those that differ where they shouldn't.
    pairs: Cell<u32>,
    moved: RefCell<Vec<String>>,
    shots: Cell<u32>,
    /// What the next step waits for, beyond its time (a language's graphics, loaded on a thread).
    until: RefCell<Option<Until>>,
    /// The motion probe's burst under way: when it began, and each shot with its time and the controls' sizes.
    burst: RefCell<Option<Burst>>,
    /// What the motion probe found wrong, and what it saw of the focus.
    resized: RefCell<Vec<String>>,
    late: RefCell<Vec<String>>,
    seen: RefCell<Vec<String>>,
}

/// A shot of a burst: the milliseconds since it began, the picture, and each control's size.
type BurstShot = (u128, SharedPixelBuffer<Rgba8Pixel>, Vec<(Key, (f32, f32))>);

struct Burst {
    name: &'static str,
    began: Instant,
    shots: Vec<BurstShot>,
}

type Step = Box<dyn Fn(&AppWindow, &Rc<RefCell<App>>, &Rc<Tour>)>;
type Until = Box<dyn Fn(&App) -> bool>;

/// Whether the pass found something wrong (the process's status).
pub fn failed() -> bool {
    FAILED.with(Cell::get)
}

thread_local! {
    static FAILED: Cell<bool> = const { Cell::new(false) };
}

/// Walk the screens, writing each to `dir`, then quit.
pub fn start(ui: &AppWindow, app: &Rc<RefCell<App>>, dir: PathBuf) {
    let _ = std::fs::create_dir_all(&dir);
    let tour = Rc::new(Tour {
        dir,
        tag: Default::default(),
        first: Default::default(),
        cut: Default::default(),
        pairs: Cell::new(0),
        moved: Default::default(),
        shots: Cell::new(0),
        until: Default::default(),
        burst: Default::default(),
        resized: Default::default(),
        late: Default::default(),
        seen: Default::default(),
    });
    let t = tour.clone();
    ui.global::<Audit>().on_clipped(move |text| {
        t.cut.borrow_mut().insert((t.tag.borrow().clone(), text.to_string()));
    });
    if let Ok(game) = std::env::var("NETTAI_TOUR_GAME") {
        app.borrow_mut().shell.select_game(&game);
    }
    let asked = |var: &str| std::env::var(var).ok().map(|list| list.split(',').map(str::to_string).collect::<Vec<_>>());
    let languages: Vec<&'static str> = crate::lang::LANGUAGES.iter().map(|(c, _)| *c).filter(|c| asked("NETTAI_TOUR_LANGS").is_none_or(|l| l.iter().any(|a| a == c))).collect();
    let sizes: Vec<(&str, u32, u32)> = SIZES.into_iter().filter(|(name, ..)| asked("NETTAI_TOUR_SIZES").is_none_or(|l| l.iter().any(|a| a == name))).collect();

    let mut steps: Vec<(u64, Step)> = Vec::new();
    if std::env::var_os("NETTAI_TOUR_MOTION").is_some() {
        {
            let mut app = app.borrow_mut();
            app.shell.probe_motion = true;
            app.shell.apply_motion();
        }
        motion_walk(&mut steps);
        steps.push((SETTLE, Box::new(|_, _, tour| finish(tour))));
        return when_ready(ui.as_weak(), app.clone(), tour, Rc::new(steps), 0);
    }
    for (size, width, height) in sizes {
        steps.push((
            SETTLE * 3,
            Box::new(move |ui, _, _| {
                // (In the display's pixels: the window's own scale is the app's to set.)
                let os = slint::winit_030::WinitWindowAccessor::with_winit_window(ui.window(), |w| w.scale_factor()).unwrap_or(1.0);
                ui.window().set_size(PhysicalSize::new((width as f64 * os).round() as u32, (height as f64 * os).round() as u32));
            }),
        ));
        for &lang in &languages {
            walk(&mut steps, lang, size);
        }
    }
    steps.push((SETTLE, Box::new(|_, _, tour| finish(tour))));
    when_ready(ui.as_weak(), app.clone(), tour, Rc::new(steps), 0);
}

/// The steps of one language at one size.
fn walk(steps: &mut Vec<(u64, Step)>, lang: &'static str, size: &'static str) {
    let mut add = |wait: u64, step: Step| steps.push((wait, step));
    // A shot: the texts' check started again, the picture written (and kept, if `first` of a pair; compared
    // with the pair's first, if `second`, which may differ only inside `also` and the two cursors).
    #[derive(Clone, Copy, PartialEq)]
    enum Pair {
        None,
        First,
        Second,
    }
    let shot = |add: &mut dyn FnMut(u64, Step), name: &'static str, pair: Pair| {
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
                // (The audit is on for the shot alone: what is cut for a moment between two, as the window
                // changes its size or a screen arrives, isn't a shot's.)
                ui.global::<Theme>().set_audit(false);
                let Some(picture) = write(ui, tour, &tag) else { return };
                let cursor = app.borrow().shell.nav.cursor();
                match pair {
                    Pair::None => {}
                    Pair::First => *tour.first.borrow_mut() = Some((tag.clone(), picture, cursor)),
                    Pair::Second => {
                        if let Some((first, before, was)) = tour.first.borrow_mut().take() {
                            compare(ui, tour, &first, &before, &tag, &picture, &[was, cursor]);
                        }
                    }
                }
            }),
        );
    };
    let go = |to: ScreenId| -> Step {
        Box::new(move |_, app, _| {
            let mut app = app.borrow_mut();
            app.go_now(to);
            app.shell.nav.show_cursor(true, false);
        })
    };
    let act = |action: Action| -> Step { Box::new(move |_, app, _| app.borrow_mut().act(action)) };
    let focus = |id: &'static str, index: i32| -> Step { Box::new(move |_, app, _| app.borrow_mut().shell.nav.focus(id, index)) };

    add(
        SETTLE,
        Box::new(move |_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.set_name(if lang == "ja" { "ネット" } else { "bigfarts" });
            app.shell.set_language(lang);
            app.go_now(ScreenId::Menu);
            app.shell.nav.focus("play", 0);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    // (The language's graphics are loaded on a thread: the next step waits for them.)
    add(
        SETTLE,
        Box::new(move |_, _, tour| {
            *tour.until.borrow_mut() = Some(Box::new(move |app| app.shell.ready().is_some_and(|(_, ready)| ready.has_language(lang))));
        }),
    );
    add(SETTLE * 2, Box::new(|_, _, _| {}));

    // Loading, as the app starts.
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.hold_loading = Some(0.62)));
    shot(&mut add, "loading", Pair::None);
    add(SETTLE, Box::new(|ui, _, _| ui.global::<nettai_ui::Shell>().set_no_games(true)));
    shot(&mut add, "no-games", Pair::None);
    add(
        SETTLE,
        Box::new(|ui, app, _| {
            ui.global::<nettai_ui::Shell>().set_no_games(false);
            app.borrow_mut().shell.hold_loading = None;
        }),
    );

    // The main menu: the focus a step down, and the pad's hints.
    shot(&mut add, "menu", Pair::First);
    add(SETTLE, act(Action::Down));
    shot(&mut add, "menu-focus", Pair::Second);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.nav.show_cursor(true, true)));
    shot(&mut add, "menu-pad", Pair::None);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.nav.show_cursor(false, false)));
    shot(&mut add, "menu-pointer", Pair::None);

    // The four screens to be built.
    for (name, to) in [("play", ScreenId::Play), ("training", ScreenId::Training), ("builds", ScreenId::Builds), ("replays", ScreenId::Replays)] {
        add(SETTLE, go(to));
        shot(&mut add, name, Pair::None);
    }
    training::steps(&mut add, lang, size);

    // Settings: the focus a step down; a value changed under the focus; a field being edited; a select's list;
    // the groups down the page.
    add(SETTLE, go(ScreenId::Settings));
    add(SETTLE, focus("set.name", 0));
    shot(&mut add, "settings", Pair::First);
    add(SETTLE, act(Action::Down));
    shot(&mut add, "settings-focus", Pair::Second);
    add(SETTLE, focus("set.delay", 0));
    shot(&mut add, "settings-delay", Pair::First);
    add(SETTLE, Box::new(|_, app, _| (0..6).for_each(|_| app.borrow_mut().act(Action::Right))));
    shot(&mut add, "settings-delay-10", Pair::Second);
    add(SETTLE, Box::new(|_, app, _| (0..6).for_each(|_| app.borrow_mut().act(Action::Left))));
    add(SETTLE, focus("set.picture", 0));
    shot(&mut add, "settings-size", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(&mut add, "settings-size-1", Pair::Second);
    add(SETTLE, act(Action::Left));
    add(SETTLE, focus("set.name", 0));
    add(SETTLE, act(Action::Confirm));
    shot(&mut add, "settings-editing", Pair::None);
    add(SETTLE, act(Action::Back));
    add(SETTLE, focus("set.language", 0));
    add(SETTLE, act(Action::Confirm));
    shot(&mut add, "settings-list", Pair::None);
    add(SETTLE, act(Action::Back));
    add(SETTLE, focus("set.key", 4));
    shot(&mut add, "settings-controls", Pair::None);
    add(SETTLE, act(Action::Confirm));
    shot(&mut add, "settings-binding", Pair::None);
    add(SETTLE, focus("set.folder", 0));
    shot(&mut add, "settings-about", Pair::None);

    // A failure that ends what the player was doing.
    add(SETTLE, go(ScreenId::Play));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.fail(UserError::ConnectionLost)));
    shot(&mut add, "dialog", Pair::None);
    add(SETTLE, act(Action::Confirm));

    // Before a battle, the battle, and the menus over it.
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.go_now(ScreenId::Training);
            app.shell.hold_cards = true;
            crate::screens::training::start_development_battle(&mut app.shell, 7);
        }),
    );
    shot(&mut add, "versus", Pair::None);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.hold_cards = false));
    add(5000, Box::new(|_, _, _| {}));
    shot(&mut add, "battle", Pair::None);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.open_menu()));
    shot(&mut add, "battle-training-menu", Pair::None);
    // (An online match's corner and menu, shown over the training battle: there is no link here.)
    add(
        SETTLE,
        Box::new(|ui, app, _| {
            app.borrow_mut().shell.close_menu();
            let shell = ui.global::<nettai_ui::Shell>();
            shell.set_connection_shown(true);
            shell.set_connection_level(3);
            shell.set_connection_ping(42);
        }),
    );
    shot(&mut add, "battle-connection", Pair::None);
    add(
        SETTLE,
        Box::new(|ui, app, _| {
            app.borrow_mut().shell.open_menu();
            ui.global::<nettai_ui::Shell>().set_battle(BattleKind::Online);
        }),
    );
    shot(&mut add, "battle-match-menu", Pair::None);
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.end_battle();
            app.go_now(ScreenId::Menu);
        }),
    );
}

/// The motion probe's steps: at the desktop's size, in English.
fn motion_walk(steps: &mut Vec<(u64, Step)>) {
    let mut add = |wait: u64, step: Step| steps.push((wait, step));
    // A burst: `trigger` starts an animation, and a shot is taken at each of `at` (milliseconds after it); the
    // shots are written, and their controls' sizes compared, once the last is taken.
    let burst = |add: &mut dyn FnMut(u64, Step), name: &'static str, at: &[u64], trigger: Step| {
        add(
            SETTLE * 2,
            Box::new(move |ui, app, tour| {
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
    };
    let go = |to: ScreenId| -> Step { Box::new(move |_, app, _| app.borrow_mut().go_now(to)) };
    let act = |action: Action| -> Step { Box::new(move |_, app, _| app.borrow_mut().act(action)) };
    let focus = |id: &'static str, index: i32| -> Step { Box::new(move |_, app, _| app.borrow_mut().shell.nav.focus(id, index)) };

    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.set_name("bigfarts");
            app.go_now(ScreenId::Menu);
            app.shell.nav.focus("play", 0);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    burst(&mut add, "cursor-glide", &[0, 30, 60, 100, 180], act(Action::Down));
    burst(&mut add, "screen-to-settings", &[0, 40, 80, 130, 170, 220, 300, 450], go(ScreenId::Settings));
    burst(&mut add, "screen-back-to-menu", &[0, 40, 80, 130, 170, 220, 300, 450], go(ScreenId::Menu));

    // A press in the same instant as a screen change: Settings is asked for, and Down pressed twice before a
    // frame has passed. The focus must end two controls down the new screen (Name, Language, Picture size).
    add(
        SETTLE * 2,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.go_now(ScreenId::Settings);
            app.act(Action::Down);
            app.act(Action::Down);
        }),
    );
    add(
        SETTLE * 2,
        Box::new(|_, app, tour| {
            let focus = app.borrow().shell.nav.focused().map(|k| k.id.clone()).unwrap_or_default();
            tour.seen.borrow_mut().push(format!("Settings asked for and Down pressed twice at once: the focus is on {focus:?}"));
            if focus != "set.picture" {
                tour.late.borrow_mut().push(format!("two presses made with a screen change ended on {focus:?}, not on the second control down the new screen (set.picture)"));
            }
        }),
    );

    // A direction held down a scrolling page: every repeat moves the focus, in the same call.
    add(SETTLE, focus("set.name", 0));
    for repeat in 0..14 {
        add(
            45,
            Box::new(move |_, app, tour| {
                let mut app = app.borrow_mut();
                let before = app.shell.nav.focused().cloned();
                app.act(Action::Down);
                let after = app.shell.nav.focused().cloned();
                let name = |k: &Option<crate::nav::Key>| k.as_ref().map(|k| format!("{}[{}]", k.id, k.index)).unwrap_or_default();
                tour.seen.borrow_mut().push(format!("Down held, repeat {repeat}: {} to {}", name(&before), name(&after)));
                if before == after {
                    tour.late.borrow_mut().push(format!("Down held, repeat {repeat}: the focus stayed on {}", name(&before)));
                }
            }),
        );
    }

    add(SETTLE, focus("set.delay", 0));
    burst(&mut add, "stepper-slide", &[0, 30, 60, 100, 180], act(Action::Right));
    add(SETTLE, focus("set.text", 0));
    burst(&mut add, "switch-slide", &[0, 40, 80, 180], act(Action::Right));
    add(SETTLE, focus("set.reset", 0));
    burst(&mut add, "button-press", &[0, 30, 70, 120, 220], act(Action::Confirm));
    add(SETTLE, focus("set.language", 0));
    burst(&mut add, "list-opens", &[0, 30, 60, 140], act(Action::Confirm));
    add(SETTLE, act(Action::Back));
    add(SETTLE, focus("set.name", 0));
    burst(&mut add, "page-scrolls", &[0, 30, 60, 100, 200], focus("set.folder", 0));
    burst(&mut add, "dialog", &[0, 30, 60, 100, 160, 300], Box::new(|_, app, _| app.borrow_mut().shell.fail(UserError::ConnectionLost)));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, go(ScreenId::Training));
    burst(
        &mut add,
        "battle-entered",
        &[0, 40, 90, 160, 300, 480, 560, 640, 720, 800, 900, 1100],
        Box::new(|_, app, _| crate::screens::training::start_development_battle(&mut app.borrow_mut().shell, 7)),
    );
    add(1500, Box::new(|_, _, _| {}));
    burst(&mut add, "menu-over-battle", &[0, 40, 90, 160, 260], Box::new(|_, app, _| app.borrow_mut().shell.open_menu()));
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.end_battle();
            app.go_now(ScreenId::Menu);
        }),
    );
    training::motion(&mut add);
}

/// Write a burst's shots (at half their size: they are many), and hold each to its last: every control the size
/// it is at rest.
fn flush_burst(tour: &Tour) {
    let Some(burst) = tour.burst.borrow_mut().take() else { return };
    let Some((_, _, rest)) = burst.shots.last() else { return };
    let rest: std::collections::HashMap<&Key, (f32, f32)> = rest.iter().map(|(k, s)| (k, *s)).collect();
    for (ms, picture, sizes) in &burst.shots {
        for (key, size) in sizes {
            // (A control not yet laid out has no size; one that is gone by the end has none to hold it to.)
            if let Some(at_rest) = rest.get(key)
                && size.0 > 0.0
                && ((size.0 - at_rest.0).abs() > 0.5 || (size.1 - at_rest.1).abs() > 0.5)
            {
                tour.resized.borrow_mut().push(format!("{} at {ms} ms: {}[{}] is {} by {}, and {} by {} at rest", burst.name, key.id, key.index, size.0, size.1, at_rest.0, at_rest.1));
            }
        }
        let (w, h) = (picture.width() / 2, picture.height() / 2);
        let mut half = Vec::with_capacity((w * h * 4) as usize);
        let pixels = picture.as_slice();
        for y in 0..h {
            for x in 0..w {
                let p = pixels[(y * 2 * picture.width() + x * 2) as usize];
                half.extend_from_slice(&[p.r, p.g, p.b, p.a]);
            }
        }
        let path = tour.dir.join(format!("motion-{}-{ms:04}ms.png", burst.name));
        let Ok(file) = std::fs::File::create(&path) else { continue };
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        if enc.write_header().and_then(|mut w| w.write_image_data(&half)).is_ok() {
            tour.shots.set(tour.shots.get() + 1);
        }
    }
}

/// Walk once the selected game is ready, or after five minutes.
fn when_ready(ui: slint::Weak<AppWindow>, app: Rc<RefCell<App>>, tour: Rc<Tour>, steps: Rc<Vec<(u64, Step)>>, waited: u32) {
    if app.borrow().shell.ready().is_some() || waited >= 600 {
        return run(ui, app, tour, steps, 0);
    }
    Timer::single_shot(Duration::from_millis(500), move || when_ready(ui, app, tour, steps, waited + 1));
}

fn run(ui: slint::Weak<AppWindow>, app: Rc<RefCell<App>>, tour: Rc<Tour>, steps: Rc<Vec<(u64, Step)>>, at: usize) {
    let Some((wait, _)) = steps.get(at) else { return };
    Timer::single_shot(Duration::from_millis(*wait), move || waited(ui, app, tour, steps, at, 0));
}

/// A step's time is up: it is run, once what the pass waits for has come (or after a minute).
fn waited(ui: slint::Weak<AppWindow>, app: Rc<RefCell<App>>, tour: Rc<Tour>, steps: Rc<Vec<(u64, Step)>>, at: usize, tries: u32) {
    let come = tour.until.borrow().as_ref().is_none_or(|until| app.try_borrow().is_ok_and(|app| until(&app)));
    if !come && tries < 600 {
        return Timer::single_shot(Duration::from_millis(100), move || waited(ui, app, tour, steps, at, tries + 1));
    }
    *tour.until.borrow_mut() = None;
    let Some(window) = ui.upgrade() else { return };
    (steps[at].1)(&window, &app, &tour);
    run(ui, app, tour, steps, at + 1);
}

/// The window, as it is drawn, to `<tag>.png`.
fn write(ui: &AppWindow, tour: &Tour, tag: &str) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let path = tour.dir.join(format!("{tag}.png"));
    let Ok(shot) = ui.window().take_snapshot() else {
        eprintln!("tour: can't take a snapshot for {}", path.display());
        return None;
    };
    let file = match std::fs::File::create(&path) {
        Ok(f) => std::io::BufWriter::new(f),
        Err(e) => {
            eprintln!("tour: {}: {e}", path.display());
            return None;
        }
    };
    let mut enc = png::Encoder::new(file, shot.width(), shot.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    match enc.write_header().and_then(|mut w| w.write_image_data(shot.as_bytes())) {
        Ok(()) => tour.shots.set(tour.shots.get() + 1),
        Err(e) => eprintln!("tour: {}: {e}", path.display()),
    }
    Some(shot)
}

/// Compare a pair of shots: they may differ only inside `allowed` (the controls the step touched, in the UI's
/// units), each grown by the cursor's reach (its brackets are 5 outside a control, and glow 4 beyond).
fn compare(ui: &AppWindow, tour: &Tour, first: &str, before: &SharedPixelBuffer<Rgba8Pixel>, second: &str, after: &SharedPixelBuffer<Rgba8Pixel>, allowed: &[Option<Rect>]) {
    const REACH: f32 = 12.0;
    tour.pairs.set(tour.pairs.get() + 1);
    if (before.width(), before.height()) != (after.width(), after.height()) {
        tour.moved.borrow_mut().push(format!("{first} and {second} are of two sizes"));
        return;
    }
    let factor = ui.window().scale_factor();
    let allowed: Vec<(i64, i64, i64, i64)> = allowed
        .iter()
        .flatten()
        .map(|r| (((r.x - REACH) * factor).floor() as i64, ((r.y - REACH) * factor).floor() as i64, ((r.right() + REACH) * factor).ceil() as i64, ((r.bottom() + REACH) * factor).ceil() as i64))
        .collect();
    let width = before.width() as i64;
    // Where the two differ outside what the step touched: how many pixels, and the box around them.
    let (mut count, mut bounds) = (0u64, (i64::MAX, i64::MAX, 0i64, 0i64));
    for (i, (a, b)) in before.as_slice().iter().zip(after.as_slice()).enumerate() {
        if a == b {
            continue;
        }
        let (x, y) = (i as i64 % width, i as i64 / width);
        if allowed.iter().any(|&(l, t, r, b)| x >= l && x < r && y >= t && y < b) {
            continue;
        }
        count += 1;
        bounds = (bounds.0.min(x), bounds.1.min(y), bounds.2.max(x), bounds.3.max(y));
    }
    if count > 0 {
        let unit = |v: i64| (v as f32 / factor).round();
        tour.moved.borrow_mut().push(format!(
            "{first} and {second}: {count} pixels differ outside the controls the step touched, within x {} to {}, y {} to {} (units)",
            unit(bounds.0),
            unit(bounds.2),
            unit(bounds.1),
            unit(bounds.3)
        ));
    }
}

/// Write the report, and leave.
fn finish(tour: &Tour) {
    let mut report = String::new();
    let (cut, moved) = (tour.cut.borrow(), tour.moved.borrow());
    let (resized, late, seen) = (tour.resized.borrow(), tour.late.borrow(), tour.seen.borrow());
    let _ = writeln!(report, "{} shots, {} pairs compared.", tour.shots.get(), tour.pairs.get());
    if std::env::var_os("NETTAI_TOUR_MOTION").is_some() {
        let _ = writeln!(report, "\nControls whose size differed while something moved: {}", resized.len());
        for line in resized.iter() {
            let _ = writeln!(report, "  {line}");
        }
        let _ = writeln!(report, "\nPresses that didn't act at once: {}", late.len());
        for line in late.iter() {
            let _ = writeln!(report, "  {line}");
        }
        let _ = writeln!(report, "\nWhat the focus did:");
        for line in seen.iter() {
            let _ = writeln!(report, "  {line}");
        }
    }
    let _ = writeln!(report, "\nPairs that differ outside the controls their step touched: {}", moved.len());
    for line in moved.iter() {
        let _ = writeln!(report, "  {line}");
    }
    let _ = writeln!(report, "\nTexts cut by their boxes: {}", cut.len());
    for (tag, text) in cut.iter() {
        let _ = writeln!(report, "  {tag}: {text:?}");
    }
    let path = tour.dir.join("report.txt");
    if let Err(e) = std::fs::write(&path, &report) {
        eprintln!("tour: {}: {e}", path.display());
    }
    eprint!("tour: {report}");
    FAILED.with(|f| f.set(!cut.is_empty() || !moved.is_empty() || !resized.is_empty() || !late.is_empty()));
    let _ = slint::quit_event_loop();
}
