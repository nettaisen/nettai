//! The screenshot pass's steps through Replays (docs/app.md §13): the list, the selected replay, a search, a
//! file that doesn't read (selected as the screen arrives from the menu), the delete question, the export's
//! dialog (idle, under way, failed), the two cards, the viewer (a focus move and a second of playback apart,
//! each a pair), the other console, the input display with the other console's picture in the corner, the
//! export's dialog over the viewer (one side, both sides), a replay that fails as it is watched, and on a phone
//! the selected replay's sheet.
//!
//! The pass has replays of its own, made here the first time through (`crate::replays::folder` is the pass's
//! folder while it runs): three sets of the selected game played by themselves and recorded, written again
//! under other names, days and kinds of match, with a few files that don't read beside them. So its pictures
//! don't depend on what the player has recorded.
//!
//! A delete is carried out too, of a file made for it; `replays.txt` in the pass's folder says whether the file
//! went and the list followed (it is said there, and isn't one of the pass's own checks).
//!
//! `NETTAI_TOUR_EXPORT=1` adds a real export, once: the first replay's last round at twice the frame, to an
//! MP4 in the pass's folder, through `ffmpeg`; and a second one stopped part way. `export.txt` there says how
//! each went and what was left in the folder.

use super::{SETTLE, Step, Tour, compare, write};
use crate::errors::UserError;
use crate::input::{Action, KeyPress};
use crate::nav::{Key, Rect};
use crate::screens::ScreenId;
use crate::shell::{App, Shell};
use nettai_battle::Content;
use nettai_battle::battle::mode;
use nettai_frontend::driver::{InFight, LivePlayer, OnCustomScreen, Opponent, bot_buttons};
use nettai_frontend::replay::{Info, MatchKind, Recorder, Replay};
use nettai_frontend::session::Session;
use nettai_ui::{AppWindow, ExportState, ReplaysView, Theme};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, Model};
use std::io::Write;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// The most ticks a set made for the pass is played for.
const MOST_TICKS: u64 = 30_000;

/// The seeds of the sets made for the pass. (Of EXE6's they make a set of three rounds won 2–1 in eight minutes,
/// and two of two rounds, won and lost, in four.)
const SEEDS: [u32; 3] = [29, 11, 33];

/// A sink a recording is written to, to be read back.
#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A set of `game` played by itself from `seed` (both navis pressing at random) for `most` ticks at the most,
/// recorded.
fn record(content: &Arc<Content>, game: &str, seed: u32, most: u64) -> Result<Replay, String> {
    let m = nettai_match::Match { seed: Some(seed), ..nettai_match::pick::live(content, game, seed, None)? };
    let sink = Sink::default();
    let opponent = Opponent { in_fight: InFight::AttacksAtRandom, on_custom_screen: OnCustomScreen::PicksFirst };
    let mut s = Session::new(Box::new(LivePlayer::against(nettai_match::Set::of(content, &m, seed), opponent, seed as u64)));
    s.record(Recorder::new(Box::new(sink.clone()), content, &m, &Info::of(MatchKind::Offline, 0))?)?;
    // (The left navi's own presses: a direction, the buster and a chip now and then, from the seed.)
    let (mut random, mut held) = (seed as u64 | 1, 0u16);
    while s.ticks < most {
        use nettai_battle::input::keys;
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        if random % 4 == 0 {
            held = [0, keys::UP, keys::DOWN, keys::LEFT, keys::RIGHT][(random >> 8) as usize % 5];
            held |= if (random >> 16) % 3 == 0 { keys::A } else { 0 } | if (random >> 24) % 2 == 0 { keys::B } else { 0 };
        }
        let buttons = if s.battle.round.mode == mode::CUSTOM { bot_buttons(&s.battle, 0, s.ticks as u32) } else { held };
        if !s.step(buttons) {
            break;
        }
    }
    drop(s);
    let bytes = sink.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
    Replay::read(&bytes).map_err(|e| e.to_string())
}

/// `replay` as a file's bytes, said to be `info`'s, its input cut after `ticks` ticks (none: all of it).
fn written(replay: &Replay, info: &Info, ticks: Option<usize>) -> Vec<u8> {
    let mut writer = nettai_replay::Writer::new(Vec::new(), &replay.head, &replay.match_bytes, info).expect("a vector takes it");
    for tick in &replay.ticks[..ticks.unwrap_or(replay.ticks.len()).min(replay.ticks.len())] {
        writer.tick(tick).expect("a vector takes it");
    }
    writer.into_inner()
}

/// A time of day in October 2026, in the machine's own time, as seconds since 1970.
fn october(day: u32, hour: u32, minute: u32) -> u64 {
    use chrono::TimeZone;
    chrono::Local.with_ymd_and_hms(2026, 10, day, hour, minute, 0).single().map_or(0, |t| t.timestamp() as u64)
}

/// Make the pass's replays in `dir`, if they aren't there.
pub(super) fn make(shell: &Shell, dir: &Path) -> Result<(), String> {
    if dir.join("made").exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let (game, ready) = shell.ready().ok_or("no game is ready")?;
    let set = |seed: u32| record(ready.content(), &game, seed, MOST_TICKS);
    let sets = [set(SEEDS[0])?, set(SEEDS[1])?, set(SEEDS[2])?];
    let save = |name: &str, bytes: &[u8], when: u64| -> Result<(), String> {
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        // (A file that doesn't read is listed by when it was written.)
        let file = std::fs::File::options().write(true).open(&path).map_err(|e| e.to_string())?;
        file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(when)).map_err(|e| e.to_string())
    };
    // (Which set, the side that recorded, the other player, when, online, open setups, cut after this part.)
    let me = shell.settings.name.clone();
    let listed: [(usize, u8, &str, (u32, u32, u32), bool, bool, Option<f32>); 15] = [
        (0, 0, "weenie", (11, 21, 42), true, true, None),
        (1, 1, "weenie", (11, 21, 30), true, true, None),
        (2, 0, "", (11, 20, 58), false, true, None),
        (0, 1, "moosh", (10, 23, 15), true, false, None),
        (1, 0, "moosh", (10, 22, 51), true, true, Some(0.45)),
        (2, 1, "", (9, 19, 4), false, true, None),
        (0, 0, "playerwithalongname", (8, 21, 20), true, true, None),
        (1, 1, "playerwithalongname", (8, 21, 2), true, true, None),
        (2, 0, "weenie", (7, 22, 10), true, true, None),
        (0, 1, "weenie", (7, 21, 48), true, true, None),
        (1, 0, "ネットバトラー", (6, 20, 15), true, true, None),
        (2, 1, "moosh", (5, 19, 30), true, true, None),
        (0, 0, "weenie", (4, 23, 1), true, true, None),
        (1, 1, "zed", (3, 18, 44), true, true, None),
        (2, 0, "zed", (2, 12, 5), true, true, None),
    ];
    for (i, (set, side, other, (day, hour, minute), online, open_setups, cut)) in listed.into_iter().enumerate() {
        let replay = &sets[set];
        let when = october(day, hour, minute);
        let mut names = [me.clone(), other.to_string()];
        if side == 1 {
            names.swap(0, 1);
        }
        let info = Info { when, side, kind: if online { MatchKind::Online } else { MatchKind::Offline }, open_setups, names };
        let ticks = cut.map(|part| (replay.ticks.len() as f32 * part) as usize);
        save(&format!("{i:02}.nettaireplay"), &written(replay, &info, ticks), when)?;
    }
    // Three that aren't played: of another layout, of other content, and no replay at all.
    save("old.nettaireplay", b"NTRP\x02another nettai's", october(9, 23, 40))?;
    let mut other = sets[0].clone();
    other.head.content ^= 1;
    let when = october(9, 12, 0);
    save("other.nettaireplay", &written(&other, &Info { when, names: [me, "weenie".into()], ..Info::of(MatchKind::Online, 0) }, None), when)?;
    save("notes.nettaireplay", b"not a replay", october(1, 8, 0))?;
    std::fs::write(dir.join("made"), b"").map_err(|e| e.to_string())
}

/// Whether a shot is one of a pair, which may differ only where its step touched.
#[derive(Clone, Copy, PartialEq)]
enum Pair {
    None,
    First,
    Second,
}

/// What a pair's second shot may differ in besides the cursors: nothing, or the picture and the bar with its
/// two times (a second of playback).
#[derive(Clone, Copy, PartialEq)]
enum Also {
    Nothing,
    Playback,
}

/// The picture's box and the bar's row, in the UI's units.
fn playback(ui: &AppWindow, app: &App) -> Vec<Option<Rect>> {
    let shell = ui.global::<nettai_ui::Shell>();
    let picture = Rect { x: shell.get_picture_room_x(), y: shell.get_picture_room_y(), w: shell.get_picture_room_width(), h: shell.get_picture_room_height() };
    // (The times are in boxes at the bar's ends.)
    let bar = app.shell.nav.registry().borrow().rect(&Key::new("seek", 0)).map(|r| Rect { x: r.x - 72.0, w: r.w + 144.0, ..r });
    vec![Some(picture), bar]
}

/// The replays' steps of one language at one size. `add` takes each step with the time it waits first.
pub(super) fn steps(add: &mut dyn FnMut(u64, Step), lang: &'static str, size: &'static str) {
    steps_to_queue(add, lang, size);
    queue_steps(add, lang, size);
}

/// A shot, as `super::walk` takes them: the texts' check started again, the picture written, a pair compared.
fn shot(add: &mut dyn FnMut(u64, Step), lang: &'static str, size: &'static str, name: &'static str, pair: Pair, also: Also) {
    {
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
                match pair {
                    Pair::None => {}
                    Pair::First => *tour.first.borrow_mut() = Some((tag.clone(), picture, cursor)),
                    Pair::Second => {
                        if let Some((first, before, was)) = tour.first.borrow_mut().take() {
                            let mut allowed = vec![was, cursor];
                            if also == Also::Playback {
                                allowed.extend(playback(ui, &app.borrow()));
                            }
                            compare(ui, tour, &first, &before, &tag, &picture, &allowed);
                        }
                    }
                }
            }),
        );
    }
}

/// Wait, before the next step, for what the view says.
fn until(what: fn(&ReplaysView) -> bool) -> Step {
    Box::new(move |_, _, tour: &Rc<Tour>| {
        *tour.until.borrow_mut() = Some(Box::new(move |app: &App| what(&app.shell.ui().global::<ReplaysView>())));
    })
}

/// What a click or a key on one of the screen's own controls does.
fn activate(id: &'static str, index: i32) -> Step {
    Box::new(move |_, app, _| app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| s.activate(shell, id, index)))
}

/// The pass's replays made, Replays shown, and the list once each is played out and the first one's still is made.
fn arrive(add: &mut dyn FnMut(u64, Step)) {
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            let mut app = app.borrow_mut();
            if let Err(why) = make(&app.shell, &tour.dir.join("replays")) {
                eprintln!("tour: the replays couldn't be made: {why}");
            }
            // (A copy a pass stopped part way left.)
            let _ = std::fs::remove_file(tour.dir.join(CHANGED));
            let _ = std::fs::remove_file(tour.dir.join(QUEUED));
            app.go_now(ScreenId::Replays);
            app.shell.nav.focus("list", 0);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    // (Each wait is a minute at the most, and playing fifteen sets out takes longer than one on a busy machine:
    // a shot of a list whose scores are still coming in isn't the same twice.)
    for _ in 0..6 {
        add(SETTLE, until(|view| view.get_listed() && view.get_still() && view.get_rows().iter().all(|row| !row.score.is_empty() || !row.reason.is_empty())));
    }
    add(SETTLE, Box::new(|_, _, _| {}));
}

/// A replay of the queue whose file goes, in the pass's folder while the queue's steps run.
const QUEUED: &str = "replays/queued.nettaireplay";

/// The play queue's steps (docs/design/app-ui.md §2.91; `NETTAI_TOUR_ONLY=queue` takes them alone): a replay's menu
/// (on a phone its sheet, with ADD TO QUEUE), five replays added (one PLAY NEXT, one whose file then goes), the
/// Queue tab with an entry chosen and with the missing one chosen, on a phone an entry's sheet, CLEAR's question,
/// and the viewer playing the queue, then its next (the missing entry passed by).
pub(super) fn queue_steps(add: &mut dyn FnMut(u64, Step), lang: &'static str, size: &'static str) {
    let phone = size == "phone";
    let shot = |add: &mut dyn FnMut(u64, Step), name: &'static str| self::shot(add, lang, size, name, Pair::None, Also::Nothing);
    let act = |action: Action| -> Step { Box::new(move |_, app, _| app.borrow_mut().act(action)) };
    let focus = |id: &'static str, index: i32| -> Step { Box::new(move |_, app, _| app.borrow_mut().shell.nav.focus(id, index)) };
    // (A choice of the replay's menu, by its place: 1 ADD TO QUEUE, 2 PLAY NEXT.)
    let menu = |choice: i32| -> Step { Box::new(move |_, app, _| app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| s.choose(shell, "more", 0, choice))) };
    arrive(add);
    // The queue, empty from a pass before; and a copy of the newest replay, which sorts first.
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            let made = std::fs::copy(tour.dir.join("replays/00.nettaireplay"), tour.dir.join(QUEUED));
            if let Err(why) = made {
                eprintln!("tour: the replay to queue couldn't be made: {why}");
            }
            let mut app = app.borrow_mut();
            app.with_screen(ScreenId::Replays, |s, shell| {
                s.activate(shell, "tab", 0);
                s.answered(shell, "clear", 1);
            });
            app.go_now(ScreenId::Menu);
            app.go_now(ScreenId::Replays);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    add(SETTLE, until(|view| view.get_listed() && view.get_count() == 19));
    add(SETTLE, focus("list", 0));
    add(SETTLE, until(|view| view.get_still()));
    if phone {
        add(SETTLE, activate("list", 0));
        add(SETTLE, until(|view| view.get_still()));
        shot(add, "queue-replay-sheet");
        add(SETTLE, act(Action::Back));
    } else {
        add(SETTLE, activate("more", 0));
        shot(add, "queue-replay-menu");
        add(SETTLE, act(Action::Back));
    }
    for row in 0..4 {
        add(SETTLE, focus("list", row));
        add(SETTLE, menu(1));
    }
    add(SETTLE, focus("list", 5));
    add(SETTLE, menu(2));
    // (The copy goes: its entry is missing.)
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            let _ = std::fs::remove_file(tour.dir.join(QUEUED));
            let mut app = app.borrow_mut();
            app.go_now(ScreenId::Menu);
            app.go_now(ScreenId::Replays);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    add(SETTLE, until(|view| view.get_listed() && view.get_count() == 18));
    add(SETTLE, activate("tab", 1));
    add(SETTLE, focus("queue", 2));
    add(SETTLE, until(|view| view.get_still() && view.get_queue().iter().all(|row| !row.score.is_empty() || !row.reason.is_empty())));
    shot(add, "queue");
    add(SETTLE, focus("queue", 1));
    shot(add, "queue-missing");
    if phone {
        add(SETTLE, activate("queue", 2));
        add(SETTLE, until(|view| view.get_still()));
        shot(add, "queue-sheet");
        add(SETTLE, act(Action::Back));
    }
    add(SETTLE, activate("clear", 0));
    shot(add, "queue-clear");
    add(SETTLE, act(Action::Back));
    // The viewer playing the queue from its start, then its next (past the missing entry).
    add(SETTLE, activate("watch-queue", 0));
    add(SETTLE, until(|view| view.get_watching()));
    add(SETTLE * 2, focus("next-replay", 0));
    shot(add, "viewer-queue");
    add(SETTLE, activate("next-replay", 0));
    add(SETTLE, until(|view| view.get_watching() && view.get_up_next().row_count() == 2));
    add(SETTLE * 2, focus("seek", 0));
    shot(add, "viewer-queue-next");
    add(SETTLE, act(Action::Back));
    add(SETTLE, until(|view| !view.get_watching()));
    shot(add, "queue-after");
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.with_screen(ScreenId::Replays, |s, shell| {
                s.answered(shell, "clear", 1);
                s.activate(shell, "tab", 0);
            });
            app.go_now(ScreenId::Menu);
        }),
    );
}

/// The replays' steps up to the queue's (`queue_steps`).
fn steps_to_queue(add: &mut dyn FnMut(u64, Step), lang: &'static str, size: &'static str) {
    let phone = size == "phone";
    let shot = |add: &mut dyn FnMut(u64, Step), name: &'static str, pair: Pair, also: Also| self::shot(add, lang, size, name, pair, also);
    let act = |action: Action| -> Step { Box::new(move |_, app, _| app.borrow_mut().act(action)) };
    let focus = |id: &'static str, index: i32| -> Step { Box::new(move |_, app, _| app.borrow_mut().shell.nav.focus(id, index)) };
    let key = |code: KeyCode, times: u32| -> Step {
        Box::new(move |_, app, _| {
            app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| {
                for _ in 0..times {
                    s.key(shell, &KeyPress { code, text: None, repeat: false, shift: false });
                }
            })
        })
    };

    arrive(add);
    shot(add, "replays", Pair::None, Also::Nothing);
    if phone {
        // A phone: the selected replay's sheet.
        add(SETTLE, activate("list", 0));
        add(SETTLE, until(|view| view.get_still()));
        shot(add, "replays-sheet", Pair::First, Also::Nothing);
        add(SETTLE, act(Action::Down));
        shot(add, "replays-sheet-focus", Pair::Second, Also::Nothing);
        add(SETTLE, act(Action::Back));
    } else {
        // What can be done with it: the focus a step along.
        add(SETTLE, focus("watch", 0));
        shot(add, "replays-actions", Pair::First, Also::Nothing);
        add(SETTLE, act(Action::Right));
        shot(add, "replays-actions-focus", Pair::Second, Also::Nothing);
    }
    // A search; a file that doesn't read; the delete question.
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| s.edit(shell, "search", 0, "moo", true))));
    add(SETTLE, focus("search", 0));
    // (Another replay is selected: its own still is waited for, here and below.)
    add(SETTLE, until(|view| view.get_still()));
    shot(add, "replays-search", Pair::None, Also::Nothing);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| s.edit(shell, "search", 0, "", true))));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| s.choose(shell, "kind", 0, 1))));
    shot(add, "replays-online", Pair::None, Also::Nothing);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| s.choose(shell, "kind", 0, 0))));
    // A file that doesn't read, selected as the screen arrives from the menu: its box is empty, with nothing of
    // the menu's reel in it (the window's picture is the reel's until something else is shown in it).
    add(SETTLE, focus("list", 5));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().go_now(ScreenId::Menu)));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().go_now(ScreenId::Replays)));
    shot(add, "replays-unreadable", Pair::None, Also::Nothing);
    add(SETTLE, focus("list", 1));
    add(SETTLE, until(|view| view.get_still()));
    add(SETTLE, act(Action::Delete));
    shot(add, "replays-delete", Pair::None, Also::Nothing);
    add(SETTLE, act(Action::Back));
    delete(add, until);

    // The export's dialog: as it opens; as an export under way and a failed one look (set on the view: no
    // export runs here); then closed.
    add(SETTLE, focus("list", 0));
    add(SETTLE, until(|view| view.get_still()));
    add(SETTLE, key(KeyCode::KeyE, 1));
    shot(add, "replays-export", Pair::First, Also::Nothing);
    add(SETTLE, act(Action::Left));
    shot(add, "replays-export-focus", Pair::Second, Also::Nothing);
    add(
        SETTLE,
        Box::new(|ui, app, _| {
            let view = ui.global::<ReplaysView>();
            view.set_export_state(ExportState::Running);
            view.set_export_progress(0.41);
            view.set_export_round(2);
            view.set_export_rounds(3);
            app.borrow_mut().shell.nav.focus("export.cancel", 0);
        }),
    );
    shot(add, "replays-exporting", Pair::None, Also::Nothing);
    add(
        SETTLE,
        Box::new(|ui, app, _| {
            let view = ui.global::<ReplaysView>();
            view.set_export_state(ExportState::Failed);
            view.set_export_error(UserError::NoFfmpeg.line(ui).into());
            app.borrow_mut().shell.nav.focus("export.go", 0);
        }),
    );
    shot(add, "replays-export-failed", Pair::None, Also::Nothing);
    add(SETTLE, act(Action::Back));
    if lang == "en" && !phone && std::env::var_os("NETTAI_TOUR_EXPORT").is_some() {
        export(add);
        shot(add, "replays-exported", Pair::None, Also::Nothing);
        add(SETTLE, act(Action::Back));
        stop_export(add);
        add(SETTLE, Box::new(|ui, app, _| if ui.global::<ReplaysView>().get_exporting() { app.borrow_mut().act(Action::Back) }));
    }

    // The two cards, then the viewer, held a little way into the second round: the focus a step down; a second
    // of playback on; the other console; its export.
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.hold_cards = true;
            app.shell.nav.focus("list", 0);
            app.with_screen(ScreenId::Replays, |s, shell| s.activate(shell, if shell.scale.phone { "sheet.watch" } else { "watch" }, 0));
        }),
    );
    shot(add, "replays-cards", Pair::None, Also::Nothing);
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.hold_cards = false));
    add(SETTLE, until(|view| view.get_watching()));
    add(SETTLE * 2, focus("seek", 0));
    shot(add, "viewer", Pair::First, Also::Nothing);
    add(SETTLE, act(Action::Down));
    shot(add, "viewer-focus", Pair::Second, Also::Nothing);
    add(SETTLE, focus("seek", 0));
    shot(add, "viewer-at", Pair::First, Also::Nothing);
    add(SETTLE, key(KeyCode::Period, 60));
    shot(add, "viewer-second-on", Pair::Second, Also::Playback);
    // (The bar is a setting: taken, stepped, let go.)
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, act(Action::Right));
    add(SETTLE, act(Action::Confirm));
    shot(add, "viewer-5s-on", Pair::None, Also::Nothing);
    // (The ⋯ menu's rows, chosen as a click on them is heard: P2 is its second, EXPORT VIDEO its last.)
    let menu = |row: i32| -> Step { Box::new(move |_, app, _| app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| s.choose(shell, "more", 0, row))) };
    add(SETTLE, menu(1));
    shot(add, "viewer-other-side", Pair::None, Also::Nothing);
    // Both builds (the match's setups were open), the other console's player's at the left.
    add(SETTLE, key(KeyCode::KeyB, 1));
    shot(add, "viewer-builds", Pair::None, Also::Nothing);
    add(SETTLE, act(Action::Back));
    // The input display and the other console's picture in the corner (I and P), on together, then off.
    add(SETTLE, key(KeyCode::KeyI, 1));
    add(SETTLE, key(KeyCode::KeyP, 1));
    shot(add, "viewer-overlays", Pair::None, Also::Nothing);
    add(SETTLE, key(KeyCode::KeyI, 1));
    add(SETTLE, key(KeyCode::KeyP, 1));
    // The export, from the viewer: its Range the whole match, NOW taking where the viewer is as the End.
    add(SETTLE, menu(9));
    add(SETTLE, activate("export.end-now", 0));
    shot(add, "viewer-export", Pair::None, Also::Nothing);
    // Both consoles side by side.
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| s.choose(shell, "export.sides", 0, 1))));
    shot(add, "viewer-export-both", Pair::None, Also::Nothing);
    add(SETTLE, act(Action::Back));
    add(SETTLE, act(Action::Back));
    // The viewer of a match whose setups weren't open: BUILDS unavailable.
    add(SETTLE, focus("list", 3));
    add(SETTLE, until(|view| view.get_still()));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().with_screen(ScreenId::Replays, |s, shell| s.activate(shell, if shell.scale.phone { "sheet.watch" } else { "watch" }, 0))));
    add(SETTLE, until(|view| view.get_watching()));
    add(SETTLE * 2, focus("seek", 0));
    shot(add, "viewer-setups-hidden", Pair::None, Also::Nothing);
    add(SETTLE, act(Action::Back));
    watch_failed(add, until, phone);
    shot(add, "replays-watch-failed", Pair::None, Also::Nothing);
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            let mut app = app.borrow_mut();
            app.act(Action::Back);
            let _ = std::fs::remove_file(tour.dir.join(CHANGED));
            app.go_now(ScreenId::Menu);
        }),
    );
}

/// The file of the replay that fails as it is watched ([`watch_failed`]), in the pass's folder while its steps
/// run.
const CHANGED: &str = "replays/changed.nettaireplay";

/// A replay that fails as it is watched, up to its shot: its file, a copy of the first's, damaged after the
/// list read it (its size and its time as they were, so that the list keeps it as playable), and WATCH pressed
/// as the screen arrives from the menu, whose reel the window's picture was last. The error is over the list,
/// the replay's box empty: nothing of the reel, nor of another replay's still. (The copy is removed after the
/// shot.)
fn watch_failed(add: &mut dyn FnMut(u64, Step), until: impl Fn(fn(&ReplaysView) -> bool) -> Step, phone: bool) {
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            let copied = std::fs::copy(tour.dir.join("replays/00.nettaireplay"), tour.dir.join(CHANGED));
            let made = copied.and_then(|_| std::fs::File::options().write(true).open(tour.dir.join(CHANGED))?.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(october(12, 9, 0))));
            if let Err(why) = made {
                eprintln!("tour: the replay to damage couldn't be made: {why}");
            }
            // (The folder is listed again as the screen shows.)
            let mut app = app.borrow_mut();
            app.go_now(ScreenId::Menu);
            app.go_now(ScreenId::Replays);
        }),
    );
    add(SETTLE, until(|view| view.get_listed() && view.get_count() == 19));
    // (It is the newest.)
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.nav.focus("list", 0)));
    add(SETTLE, until(|view| view.get_still()));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().go_now(ScreenId::Menu)));
    add(
        SETTLE,
        Box::new(move |_, app, tour| {
            if let Err(why) = damage(&tour.dir.join(CHANGED)) {
                eprintln!("tour: the replay couldn't be damaged: {why}");
            }
            let mut app = app.borrow_mut();
            app.go_now(ScreenId::Replays);
            app.with_screen(ScreenId::Replays, |s, shell| s.activate(shell, if phone { "sheet.watch" } else { "watch" }, 0));
        }),
    );
}

/// Damage the replay at `path` where nothing shows it: its first tick's record made one no writer writes, its
/// size and its time as they were.
fn damage(path: &Path) -> Result<(), String> {
    let mut bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let was = std::fs::metadata(path).and_then(|m| m.modified()).map_err(|e| e.to_string())?;
    let replay = Replay::read(&bytes).map_err(|e| e.to_string())?;
    // (Where its input starts: after what a file of no ticks holds.)
    let start = written(&replay, &replay.info, Some(0)).len();
    *bytes.get_mut(start).ok_or("the replay has no input")? |= 0x20;
    std::fs::write(path, &bytes).map_err(|e| e.to_string())?;
    std::fs::File::options().write(true).open(path).and_then(|f| f.set_modified(was)).map_err(|e| e.to_string())
}

/// `NETTAI_TOUR_EXPORT`: a real export of the first replay's last round, at twice the frame, into the pass's
/// folder; `export.txt` there says what came of it.
fn export(add: &mut dyn FnMut(u64, Step)) {
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            let mut app = app.borrow_mut();
            app.shell.settings.remembered.insert("replays.export-folder".into(), tour.dir.display().to_string());
            app.shell.nav.focus("list", 0);
            app.with_screen(ScreenId::Replays, |s, shell| {
                s.key(shell, &KeyPress { code: KeyCode::KeyE, text: None, repeat: false, shift: false });
                s.step(shell, "export.size", 0, -1);
                s.choose(shell, "export.part", 0, i32::MAX);
                s.activate(shell, "export.go", 0);
            });
        }),
    );
    // (Each wait is a minute at the most: a round takes a while on a busy machine.)
    for _ in 0..5 {
        add(
            SETTLE,
            Box::new(|_, _, tour| {
                *tour.until.borrow_mut() = Some(Box::new(|app: &App| app.shell.ui().global::<ReplaysView>().get_export_state() != ExportState::Running));
            }),
        );
    }
    add(
        SETTLE,
        Box::new(|ui, _, tour| {
            let view = ui.global::<ReplaysView>();
            let made = videos(tour);
            let said = format!("the export ended {:?}; its line: {:?}; saved as {:?}\nin the folder: {made:?}\n", view.get_export_state(), view.get_export_error(), view.get_export_path());
            say(tour, "export.txt", &said);
        }),
    );
}

/// Add `said` to the pass's file `name`, and to the terminal.
fn say(tour: &Tour, name: &str, said: &str) {
    eprint!("tour: {said}");
    if let Ok(mut file) = std::fs::File::options().create(true).append(true).open(tour.dir.join(name)) {
        let _ = file.write_all(said.as_bytes());
    }
}

/// The videos and the parts of videos in the pass's folder, each with its size.
fn videos(tour: &Tour) -> Vec<String> {
    let files = std::fs::read_dir(&tour.dir).into_iter().flatten().flatten().map(|e| e.path());
    files.filter(|p| p.extension().is_some_and(|x| x == "mp4" || x == "part")).map(|p| format!("{} ({} bytes)", p.display(), p.metadata().map_or(0, |m| m.len()))).collect()
}

/// `NETTAI_TOUR_EXPORT`, after the real export: a second one (the whole match, at six times the frame) stopped
/// once some of it is written. `export.txt` says how it ended and what is in the folder then: no file of it,
/// whole or in part.
fn stop_export(add: &mut dyn FnMut(u64, Step)) {
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            app.shell.nav.focus("list", 0);
            app.with_screen(ScreenId::Replays, |s, shell| {
                s.key(shell, &KeyPress { code: KeyCode::KeyE, text: None, repeat: false, shift: false });
                s.step(shell, "export.size", 0, 1);
                s.activate(shell, "export.go", 0);
            });
        }),
    );
    add(
        SETTLE,
        Box::new(|_, _, tour| {
            *tour.until.borrow_mut() = Some(Box::new(|app: &App| {
                let ui = app.shell.ui();
                let view = ui.global::<ReplaysView>();
                view.get_export_state() != ExportState::Running || view.get_export_progress() > 0.02
            }));
        }),
    );
    add(
        SETTLE,
        Box::new(|ui, app, tour| {
            let view = ui.global::<ReplaysView>();
            say(tour, "export.txt", &format!("a second export, at {:.0}% ({:?}), with these in the folder: {:?}\n", view.get_export_progress() * 100.0, view.get_export_state(), videos(tour)));
            // (CANCEL: only while it runs. With none running, Back would close the dialog.)
            if view.get_export_state() == ExportState::Running {
                app.borrow_mut().act(Action::Back);
            }
        }),
    );
    add(
        SETTLE,
        Box::new(|_, _, tour| {
            *tour.until.borrow_mut() = Some(Box::new(|app: &App| app.shell.ui().global::<ReplaysView>().get_export_state() != ExportState::Running));
        }),
    );
    add(
        SETTLE,
        Box::new(|ui, _, tour| {
            let view = ui.global::<ReplaysView>();
            say(tour, "export.txt", &format!("stopped, it ended {:?}; in the folder: {:?}\n", view.get_export_state(), videos(tour)));
        }),
    );
}

/// A delete carried out: of a file made for it (no replay, so the last row), so that the pass's replays stay as
/// they are. `replays.txt` in the pass's folder says whether the file went and the list followed.
fn delete(add: &mut dyn FnMut(u64, Step), until: impl Fn(fn(&ReplaysView) -> bool) -> Step) {
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            let scrap = tour.dir.join("replays/scrap.nettaireplay");
            let made = std::fs::write(&scrap, b"to be deleted").and_then(|()| std::fs::File::options().write(true).open(&scrap)?.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(october(1, 7, 0))));
            if let Err(why) = made {
                say(tour, "replays.txt", &format!("the file to delete couldn't be made: {why}\n"));
            }
            // (The folder is listed again as the screen shows.)
            let mut app = app.borrow_mut();
            app.go_now(ScreenId::Menu);
            app.go_now(ScreenId::Replays);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    add(SETTLE, until(|view| view.get_count() == 19));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().shell.nav.focus("list", 0)));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().act(Action::End)));
    // The question, DELETE (Cancel is focused), Enter.
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().act(Action::Delete)));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().act(Action::Right)));
    add(SETTLE, Box::new(|_, app, _| app.borrow_mut().act(Action::Confirm)));
    add(SETTLE, until(|view| view.get_count() == 18));
    add(
        SETTLE,
        Box::new(|ui, app, tour| {
            let view = ui.global::<ReplaysView>();
            let there = tour.dir.join("replays/scrap.nettaireplay").exists();
            let focus = app.borrow().shell.nav.focused().map(|k| format!("{}[{}]", k.id, k.index)).unwrap_or_default();
            say(tour, "replays.txt", &format!("{}: a file deleted from the list: the file is {}, the list has {} (18 without it), the focus is on {focus}\n", tour.tag.borrow(), if there { "still there" } else { "gone" }, view.get_count()));
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replays::{Found, Library};
    use nettai_frontend::replay::{ReadError, Refused};

    /// Take what the library's thread finds until `enough` says so of all it found (a minute at the most).
    fn wait(library: &mut Library, enough: impl Fn(&[Found]) -> bool) {
        let (began, mut found) = (std::time::Instant::now(), Vec::new());
        while !enough(&found) {
            assert!(began.elapsed() < std::time::Duration::from_secs(60), "the replays' thread didn't answer");
            std::thread::sleep(std::time::Duration::from_millis(5));
            found.extend(library.poll());
        }
    }

    /// The cache's text once it keeps the outcomes of `files` and no others (a minute at the most). The thread
    /// writes it when it has nothing left to do: after its last answer, not with it, so a cache read as the last
    /// answer comes may be the one before.
    fn kept(cache: &std::path::Path, files: &[&crate::replays::Entry]) -> String {
        let want: Vec<String> = files.iter().map(|e| format!("file = \"{:016x}\"", e.digest.unwrap())).collect();
        let began = std::time::Instant::now();
        loop {
            let text = std::fs::read_to_string(cache).unwrap_or_default();
            if text.matches("[[played]]").count() == want.len() && want.iter().all(|w| text.contains(w.as_str())) {
                return text;
            }
            assert!(began.elapsed() < std::time::Duration::from_secs(60), "the cache doesn't keep {want:?} alone:\n{text}");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// A folder is listed on the library's thread, newest first, a file that doesn't read among the rest with
    /// why (and by when it was written), and other files left out. Each replay is then played out there: one
    /// whole, one cut short, and one of other content, which is told as refused and not as a failure. What was
    /// played out is kept: a second library with the same cache takes it from there, playing nothing; a file
    /// whose bytes changed is played out again.
    #[test]
    fn a_folder_is_listed_and_its_replays_played_out_off_the_thread() {
        let content = nettai_match::testing::exe6_content();
        let dir = std::env::temp_dir().join(format!("nettai-replays-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let replay = record(&content, "exe6", 7, 900).unwrap();
        let info = |when: u64, side: u8| Info { when, names: ["Lan".into(), "Chaud".into()], ..Info::of(MatchKind::Online, side) };
        let mut other = replay.clone();
        other.head.content ^= 1;
        std::fs::write(dir.join("first.nettaireplay"), written(&replay, &info(100, 0), None)).unwrap();
        std::fs::write(dir.join("second.nettaireplay"), written(&replay, &info(300, 1), Some(600))).unwrap();
        std::fs::write(dir.join("other.nettaireplay"), written(&other, &info(200, 0), None)).unwrap();
        std::fs::write(dir.join("old.nettaireplay"), b"NTRP\x02another nettai's").unwrap();
        std::fs::write(dir.join("notes.txt"), b"not a replay, and not named as one").unwrap();
        let cache = dir.join("kept/replays-played.toml");

        let mut library = Library::default();
        library.cache = Some(cache.clone());
        library.list(&dir);
        wait(&mut library, |found| found.iter().any(|f| matches!(f, Found::Listed)));
        let entries = library.entries.clone();
        let names: Vec<&str> = entries.iter().map(|e| e.path.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names, ["old.nettaireplay", "second.nettaireplay", "other.nettaireplay", "first.nettaireplay"]);
        assert!(matches!(entries[0].read, Err(ReadError::OtherLayout { layout: 2 })));
        let second = entries[1].read.as_ref().unwrap();
        assert_eq!((second.ticks, second.rounds.as_slice(), second.info.side, second.head.game.as_str()), (600, &[0][..], 1, "exe6"));

        let hash = content.hash().0;
        for entry in &entries[1..] {
            library.play(entry, &content, hash);
            // (Asked twice, it is played once.)
            library.play(entry, &content, hash);
        }
        wait(&mut library, |found| found.iter().filter(|f| matches!(f, Found::Played(_))).count() == 3);
        let played = |at: usize| library.played(&entries[at]).unwrap();
        assert!(matches!(played(1), Ok(out) if out.ticks == 600 && out.diverged.is_none() && out.result.is_none()), "{:?}", played(1));
        assert!(matches!(played(2), Err(Refused::OtherVersion(_))), "{:?}", played(2));
        assert!(matches!(played(3), Ok(out) if out.ticks == 900 && out.diverged.is_none()), "{:?}", played(3));
        assert!(library.played(&entries[0]).is_none());
        // (The two played out are kept, the refused one isn't.)
        kept(&cache, &[&entries[1], &entries[3]]);

        // Another run: the two that played are kept, the refused one is found refused again, and a file changed
        // since is played out anew.
        std::fs::write(dir.join("first.nettaireplay"), written(&replay, &info(100, 1), None)).unwrap();
        let mut again = Library::default();
        again.cache = Some(cache.clone());
        again.list(&dir);
        wait(&mut again, |found| found.iter().any(|f| matches!(f, Found::Listed)));
        let entries = again.entries.clone();
        let began = std::time::Instant::now();
        for entry in &entries[1..] {
            again.play(entry, &content, hash);
        }
        wait(&mut again, |found| found.iter().filter(|f| matches!(f, Found::Played(_))).count() == 3);
        assert!(matches!(again.played(&entries[1]), Some(Ok(out)) if out.ticks == 600));
        assert!(matches!(again.played(&entries[2]), Some(Err(Refused::OtherVersion(_)))));
        assert!(matches!(again.played(&entries[3]), Some(Ok(out)) if out.ticks == 900));
        // (The changed file's new outcome in the old one's place.)
        kept(&cache, &[&entries[1], &entries[3]]);
        eprintln!("the second listing's three answered in {:.2?}", began.elapsed());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file whose head reads names its game whatever is wrong after it, and is listed with that game's
    /// replays alone; one whose head doesn't read names none.
    #[test]
    fn a_file_that_doesnt_read_is_listed_with_the_game_its_head_names() {
        let content = nettai_match::testing::exe6_content();
        let dir = std::env::temp_dir().join(format!("nettai-replays-games-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let replay = record(&content, "exe6", 7, 60).unwrap();
        let bytes = written(&replay, &Info { when: 100, ..Info::of(MatchKind::Online, 0) }, None);
        let mut damaged = bytes.clone();
        damaged.push(0xE0);
        let mut other_game = replay.clone();
        other_game.head.game = "exe5".into();
        let mut other_damaged = written(&other_game, &Info { when: 100, ..Info::of(MatchKind::Online, 0) }, None);
        other_damaged.push(0xE0);
        std::fs::write(dir.join("damaged.nettaireplay"), &damaged).unwrap();
        std::fs::write(dir.join("other.nettaireplay"), &other_damaged).unwrap();
        std::fs::write(dir.join("old.nettaireplay"), b"NTRP\x02another nettai's").unwrap();
        let mut library = Library::default();
        library.list(&dir);
        wait(&mut library, |found| found.iter().any(|f| matches!(f, Found::Listed)));
        let of = |name: &str| library.entries.iter().find(|e| e.path.ends_with(name)).map(|e| (e.read.is_ok(), e.game.clone())).unwrap();
        assert_eq!(of("damaged.nettaireplay"), (false, Some("exe6".into())));
        assert_eq!(of("other.nettaireplay"), (false, Some("exe5".into())));
        assert_eq!(of("old.nettaireplay"), (false, None));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
