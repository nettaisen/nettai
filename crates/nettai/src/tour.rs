//! A tour of the app, for its screenshots (`NETTAI_TOUR=<folder>`): it
//! walks every screen by itself, in each language bundled and at a desktop's
//! and a phone's size, and writes the window as each shows to a PNG
//! (`Window::take_snapshot`: the window's own renderer's picture). The
//! battle is a real one: a random match of the first game ready, the stand-in
//! with 1 HP so the set is decided at the first hits, played by the title's
//! demo buttons.
//!
//! `NETTAI_TOUR_LANGS` names the languages (by code, comma-separated; all of
//! them by default).

use crate::app::App;
use crate::{AppWindow, NavAction, Screen};
use slint::{ComponentHandle, LogicalSize, Timer};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

type Step = (Duration, Box<dyn Fn(&AppWindow, &Rc<RefCell<App>>)>);

/// Walk the screens, writing each to `dir`, then quit.
pub fn start(ui: &AppWindow, app: &Rc<RefCell<App>>, dir: PathBuf) {
    let _ = std::fs::create_dir_all(&dir);
    let languages: Vec<&'static str> = match std::env::var("NETTAI_TOUR_LANGS") {
        Ok(list) => crate::lang::LANGUAGES.iter().map(|(c, _)| *c).filter(|c| list.split(',').any(|l| l == *c)).collect(),
        Err(_) => crate::lang::LANGUAGES.iter().map(|(c, _)| *c).collect(),
    };
    let mut steps: Vec<Step> = Vec::new();
    let mut add = |wait: u64, f: Box<dyn Fn(&AppWindow, &Rc<RefCell<App>>)>| steps.push((Duration::from_millis(wait), f));
    let shot = |dir: &PathBuf, name: String| {
        let path = dir.join(format!("{name}.png"));
        Box::new(move |ui: &AppWindow, _: &Rc<RefCell<App>>| write(ui, &path)) as Box<dyn Fn(&AppWindow, &Rc<RefCell<App>>)>
    };
    // (`NETTAI_TOUR_ONLY=builds`: the builds' screens alone.)
    let only = std::env::var("NETTAI_TOUR_ONLY").ok();
    let walks = |part: &str| only.as_deref().is_none_or(|o| o.split(',').any(|p| p == part));
    // (The games load first.)
    add(if only.is_some() { 12000 } else { 5000 }, Box::new(|ui, _| ui.window().set_size(LogicalSize::new(1280.0, 800.0))));
    for (size, w, h) in [("desktop", 1280.0, 800.0), ("phone", 390.0, 844.0)] {
        add(600, Box::new(move |ui, _| ui.window().set_size(LogicalSize::new(w, h))));
        for &lang in &languages {
            let tag = |screen: &str| format!("{screen}-{lang}-{size}");
            add(400, Box::new(move |_, app| app.borrow_mut().set_language(lang)));
            if walks("builds") {
                add(100, Box::new(|_, app| app.borrow_mut().tour_build()));
                add(900, shot(&dir, tag("builds")));
                add(100, Box::new(|_, app| app.borrow_mut().builds_activate(2)));
                add(900, shot(&dir, tag("build-navi")));
                for t in 1..8 {
                    add(100, Box::new(move |_, app| app.borrow_mut().build_tab(t)));
                    add(500, Box::new(move |ui, app| {
                        let (key, lang) = {
                            let a = app.borrow();
                            (a.builds.editor.as_ref().and_then(|e| e.layout.tabs.get(t).map(|t| t.key().to_string())), a.lang)
                        };
                        if let Some(key) = key {
                            write(ui, &dir_of(ui, &key, t, lang));
                        }
                    }));
                }
                // The folder's strip, the board with a piece in hand.
                add(100, Box::new(|_, app| {
                    let mut app = app.borrow_mut();
                    app.build_tab(1);
                    app.build_act(0, 3, -1);
                    app.build_act(0, 3, -1);
                }));
                add(500, shot(&dir, tag("build-folder-strip")));
                add(100, Box::new(|_, app| {
                    let mut app = app.borrow_mut();
                    let grid = app.builds.editor.as_ref().and_then(|e| e.layout.tabs.iter().position(|t| *t == crate::builds::layout::Tab::Grid));
                    if let Some(g) = grid {
                        app.build_tab(g);
                        app.build_act(1, 4, 0);
                        app.build_grid_hover(2, 2);
                    }
                }));
                add(500, shot(&dir, tag("build-grid-held")));
                add(100, Box::new(|_, app| app.borrow_mut().go(Screen::Builds)));
                add(900, Box::new(|_, _| {}));
                // The build chosen in Play, and in the lobby linked directly.
                add(100, Box::new(|ui, app| {
                    let mut app = app.borrow_mut();
                    app.enter(Screen::Play);
                    let game = app.builds_game_index();
                    app.play_game(game);
                    app.play_build(1);
                    ui.set_play_cursor(2);
                }));
                add(900, shot(&dir, tag("play-build")));
                add(100, Box::new(|ui, app| {
                    let mut app = app.borrow_mut();
                    app.enter(Screen::Lobby);
                    app.lobby_mode(2);
                    app.lobby_build(1);
                    ui.set_lobby_cursor(4);
                }));
                add(900, shot(&dir, tag("lobby-direct")));
                if only.is_some() {
                    continue;
                }
            }
            add(300, Box::new(|ui, app| {
                app.borrow_mut().enter(Screen::Title);
                ui.set_title_started(false);
            }));
            add(900, shot(&dir, tag("title")));
            add(100, Box::new(|ui, _| ui.invoke_nav(NavAction::Confirm)));
            add(2500, shot(&dir, tag("menu")));
            add(100, Box::new(|ui, app| {
                ui.set_title_cursor(0);
                app.borrow_mut().enter(Screen::Play);
            }));
            add(500, shot(&dir, tag("play-none")));
            add(100, Box::new(|ui, app| {
                let first = app.borrow().first_ready();
                if let Some(i) = first {
                    app.borrow_mut().play_game(i);
                    ui.set_play_cursor(2);
                }
            }));
            add(700, shot(&dir, tag("play")));
            add(100, Box::new(|_, app| app.borrow_mut().tour_battle()));
            add(200, Box::new(|_, app| app.borrow_mut().enter(Screen::Battle)));
            add(6000, shot(&dir, tag("battle")));
            add(100, Box::new(|_, app| app.borrow_mut().battle_pause()));
            add(700, shot(&dir, tag("battle-paused")));
            add(100, Box::new(|_, app| {
                let mut app = app.borrow_mut();
                app.battle_resume();
                app.tour_fast();
            }));
            add(25000, shot(&dir, tag("battle-result")));
            add(100, Box::new(|_, app| app.borrow_mut().enter(Screen::Replays)));
            add(2500, shot(&dir, tag("replays")));
            add(100, Box::new(|_, app| app.borrow_mut().enter(Screen::Lobby)));
            add(300, Box::new(|ui, app| {
                let ja = app.borrow().lang == "ja";
                app.borrow_mut().set_name(if ja { "ネット" } else { "Lan" });
                ui.set_lobby_cursor(2);
            }));
            add(700, shot(&dir, tag("lobby")));
            add(100, Box::new(|_, app| app.borrow_mut().enter(Screen::Settings)));
            add(700, shot(&dir, tag("settings")));
        }
    }
    add(200, Box::new(|_, _| {
        let _ = slint::quit_event_loop();
    }));
    run(ui.as_weak(), app.clone(), Rc::new(steps), 0);
}

/// Where a tab of the creator's picture goes: by the tab, the language and
/// the window's size.
fn dir_of(ui: &AppWindow, key: &str, tab: usize, lang: &str) -> PathBuf {
    let dir = std::env::var_os("NETTAI_TOUR").map(PathBuf::from).unwrap_or_default();
    let size = if (ui.window().size().width as f32) < 800.0 * ui.window().scale_factor() { "phone" } else { "desktop" };
    dir.join(format!("build-{tab}-{key}-{lang}-{size}.png"))
}

fn run(ui: slint::Weak<AppWindow>, app: Rc<RefCell<App>>, steps: Rc<Vec<Step>>, at: usize) {
    let Some((wait, _)) = steps.get(at) else { return };
    Timer::single_shot(*wait, move || {
        let Some(window) = ui.upgrade() else { return };
        (steps[at].1)(&window, &app);
        run(ui, app, steps, at + 1);
    });
}

/// The window, as it is drawn, to a PNG.
fn write(ui: &AppWindow, path: &std::path::Path) {
    let Ok(shot) = ui.window().take_snapshot() else {
        eprintln!("tour: can't take a snapshot for {}", path.display());
        return;
    };
    let file = match std::fs::File::create(path) {
        Ok(f) => std::io::BufWriter::new(f),
        Err(e) => return eprintln!("tour: {}: {e}", path.display()),
    };
    let mut enc = png::Encoder::new(file, shot.width(), shot.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let written = enc.write_header().and_then(|mut w| w.write_image_data(shot.as_bytes()));
    match written {
        Ok(()) => eprintln!("tour: {}", path.display()),
        Err(e) => eprintln!("tour: {}: {e}", path.display()),
    }
}
