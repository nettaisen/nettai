//! The shell (docs/app.md §2): what every screen sits in and asks things of.
//!
//! [`Shell`] is what a screen's module is handed: the window, the games and the selected one, the settings,
//! the sound, the focus, the battle on screen, and the ways to another screen, to a dialog, to a list of
//! choices, to a battle. [`App`] owns the shell and the screens, and carries what happens to the right one: a
//! key, a pad's button and a click all become the same few calls on the screen shown (`crate::screens::Screen`).
//!
//! The window's rendering is the app's clock: [`App::frame`] runs before each frame is drawn (docs/app.md §1).

use crate::battle::{Battle, BattleEvent, Loading};
use crate::errors::UserError;
use crate::games::{Games, Ready, State};
use crate::gl::GlPicture;
use crate::input::{Action, Bindings, KeyPress, Keys, Pad, menu_action};
use crate::nav::{Did, Direction, Key, Nav};
use crate::screens::{self, Screen, ScreenId};
use crate::settings::Settings;
use crate::sound::{Cue, Sound};
use nettai_battle::input::keys;
use nettai_ui::{AppWindow, ButtonKind, DialogButton, Theme};
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::event::{ElementState, KeyEvent};
use slint::winit_030::winit::keyboard::{KeyCode, PhysicalKey};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::rc::Rc;
use std::time::Instant;

/// A dialog a screen asks: its title, its message, its buttons. The screen hears the answer by `tag`
/// (`Screen::answered`).
pub struct Dialog {
    pub tag: &'static str,
    pub title: String,
    pub message: String,
    pub buttons: Vec<DialogButton>,
    /// The button the focus starts on (a deletion asks with Cancel focused), and the one Esc presses.
    pub focus: usize,
    pub cancel: usize,
}

/// A button of a dialog.
pub fn button(text: &str, kind: ButtonKind) -> DialogButton {
    DialogButton { text: text.into(), kind, danger: false }
}

/// The dialog open, and the screen that asked it (none: the shell's own, a failure's).
pub(crate) struct Asked {
    tag: &'static str,
    owner: Option<ScreenId>,
    cancel: usize,
}

/// A select's list open: the control it is of, and its screen.
struct Picking {
    owner: ScreenId,
    key: Key,
}

/// What a screen asks of the app that is done once the screen has returned.
enum Ask {
    Go(ScreenId),
    /// The screen shown sets its view again (the game, the language or the layout changed under it).
    Refresh,
    Quit,
}

/// How the window's layout is scaled (docs/design/app-ui.md §4, "Units"): the UI's lengths are units of a
/// 1280 by 800 window, or of a phone 390 wide when the window is upright, and the window's scale is set so
/// that one of Slint's `px` is one unit.
#[derive(Default)]
pub struct Scale {
    /// The display's pixels to a point (the system's scale factor).
    pub os: f32,
    /// Points to a unit.
    pub unit: f32,
    pub phone: bool,
    size: (u32, u32),
    /// The window in units, as the ground's hexagons were last drawn for.
    ground: (i32, i32),
}

pub struct Shell {
    ui: slint::Weak<AppWindow>,
    pub games: Games,
    pub settings: Settings,
    pub sound: Sound,
    pub nav: Nav,
    pub keys: Keys,
    pub pad: Pad,
    /// The battle's buttons as bound (the settings', read once).
    pub bindings: Bindings,
    /// The GBA buttons the on-screen pad holds.
    pub touch: u16,
    /// The language shown, by its code.
    pub lang: &'static str,
    /// The window's texture the picture is written into (femtovg's OpenGL); none: a new image each picture.
    pub gl: Option<GlPicture>,
    pub scale: Scale,
    /// Present the picture at the display's density (`NETTAI_PHYSICAL_PIXELS`).
    pub(crate) sharp: bool,
    /// The stage whose picture the texture holds.
    pub(crate) shown_stage: u64,
    /// The selected game.
    game: Option<String>,
    /// The game the screens were last shown with, once it was ready.
    announced: Option<String>,
    /// The frames the bar has shown the selected game's parts as read, before its runtime is loaded (which
    /// holds the window still: the bar is drawn first).
    runtime_due: u8,
    screen: ScreenId,
    asks: Vec<Ask>,
    pub(crate) dialog: Option<Asked>,
    picking: Option<Picking>,
    pub battle: Option<Battle>,
    pub(crate) loading: Option<Loading>,
    /// The screenshot pass holds things still: the two cards stay up whatever is ready, the loading bar is
    /// shown at a place of its choosing, and the menu's reel stops on one picture (`still`).
    pub hold_cards: bool,
    pub hold_loading: Option<f32>,
    pub still: bool,
    /// Ctrl, Alt or the system's key is held (the key is the system's, or a text field's).
    chord: bool,
    shift: bool,
    /// The input method is composing in a text field (its keys are its own).
    composing: bool,
    /// The settings aren't written (the screenshot pass changes them as it goes).
    pub keep_nothing: bool,
}

impl Shell {
    pub fn ui(&self) -> AppWindow {
        self.ui.upgrade().expect("the window outlives the app's state")
    }

    // ---- The screens -------------------------------------------------------------------------------------

    /// The screen shown.
    pub fn screen(&self) -> ScreenId {
        self.screen
    }

    /// Go to a screen.
    pub fn go(&mut self, to: ScreenId) {
        self.asks.push(Ask::Go(to));
    }

    /// Have the screen shown set its view again.
    pub fn refresh(&mut self) {
        if !self.asks.iter().any(|a| matches!(a, Ask::Refresh)) {
            self.asks.push(Ask::Refresh);
        }
    }

    /// Leave the app.
    pub fn quit(&mut self) {
        self.asks.push(Ask::Quit);
    }

    // ---- The selected game -------------------------------------------------------------------------------

    /// The selected game's id (`exe6`), whether or not it has loaded.
    pub fn game(&self) -> Option<&str> {
        self.game.as_deref()
    }

    /// The selected game, loaded: its id and what was loaded of it. None while it loads (the loading screen
    /// is up then, and the screen is shown again once it is ready).
    pub fn ready(&self) -> Option<(String, Rc<Ready>)> {
        let id = self.game.clone()?;
        let ready = self.games.ready(&id)?;
        Some((id, ready))
    }

    /// A game's name as shown, from its id (`exe6`: EXE6).
    pub fn game_name(id: &str) -> String {
        id.to_ascii_uppercase()
    }

    /// Where the game can't change (inside a build, a lobby, a replay), its name stays in the header without
    /// its arrows.
    pub fn lock_game(&mut self, locked: bool) {
        self.ui().global::<nettai_ui::Shell>().set_game_locked(locked);
    }

    fn game_locked(&self) -> bool {
        self.ui().global::<nettai_ui::Shell>().get_game_locked()
    }

    /// Select a game: it loads if it hasn't, and the screen shown is shown again with it once it is ready.
    pub fn select_game(&mut self, id: &str) {
        if self.game.as_deref() == Some(id) {
            return;
        }
        self.game = Some(id.to_string());
        self.settings.game = Some(id.to_string());
        self.save_settings();
        self.load(id);
        self.ui().global::<nettai_ui::Shell>().set_game(Shell::game_name(id).into());
    }

    /// The game after (or before) the selected one, around from the last to the first; a game with no pack is
    /// passed over.
    pub fn step_game(&mut self, by: i32) {
        let games = self.games.playable();
        if games.len() < 2 || self.game_locked() {
            return;
        }
        let at = self.game.as_ref().and_then(|g| games.iter().position(|id| id == g)).unwrap_or(0) as i32;
        let next = games[(at + by).rem_euclid(games.len() as i32) as usize].clone();
        self.select_game(&next);
    }

    /// Start loading a game on its thread.
    fn load(&mut self, id: &str) {
        self.games.load(id, self.lang, |id, step| crate::with_app(|app| app.shell.games.step(&id, step)));
    }

    // ---- The settings and the language -------------------------------------------------------------------

    /// Keep the settings as they are now.
    pub fn save_settings(&self) {
        if self.keep_nothing {
            return;
        }
        if let Err(why) = self.settings.save() {
            crate::errors::log("keeping the settings", &why);
        }
    }

    /// Show the app and the battle in the language `code`: the app's strings from its catalog at once, the
    /// content's names from its strings table once each game's are loaded (on a thread: the screens are shown
    /// again then).
    pub fn set_language(&mut self, code: &'static str) {
        self.lang = code;
        crate::lang::select(code);
        self.ui().global::<Theme>().set_cjk(crate::lang::cjk(code));
        self.games.load_language(code, |id, lang, graphics| {
            crate::with_app(move |app| {
                app.shell.games.language(&id, &lang, graphics);
                if app.shell.lang == lang {
                    app.shell.show_language();
                }
            })
        });
        self.show_language();
    }

    /// The battle and the screen shown, in the language as far as it is loaded.
    fn show_language(&mut self) {
        if let Some(b) = &mut self.battle
            && let Some(ready) = self.games.ready(&b.game)
        {
            b.stage.player.set_language(&ready.graphics(self.lang));
            b.stage.stale = true;
        }
        self.refresh();
    }

    /// The battle's buttons bound anew: kept, and told to the hints.
    pub fn set_bindings(&mut self, bindings: Bindings) {
        self.settings.set_bindings(&bindings);
        self.bindings = bindings;
        self.keys.release();
        self.save_settings();
        self.show_bindings();
    }

    fn show_bindings(&self) {
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        shell.set_key_l(crate::input::key_cap(self.bindings.key(keys::L)).into());
        shell.set_key_r(crate::input::key_cap(self.bindings.key(keys::R)).into());
    }

    // ---- Dialogs and lists of choices --------------------------------------------------------------------

    /// Ask the player something: the screen shown hears the answer (`Screen::answered`).
    pub fn ask(&mut self, dialog: Dialog) {
        let owner = Some(self.battle.as_ref().map_or(self.screen, |b| b.owner));
        self.open_dialog(dialog, owner);
    }

    fn open_dialog(&mut self, dialog: Dialog, owner: Option<ScreenId>) {
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        self.close_picker();
        shell.set_dialog_title(dialog.title.as_str().into());
        shell.set_dialog_message(dialog.message.as_str().into());
        shell.set_dialog_buttons(ModelRc::new(VecModel::from(dialog.buttons)));
        shell.set_dialog(true);
        self.dialog = Some(Asked { tag: dialog.tag, owner, cancel: dialog.cancel });
        self.keys.release();
        self.nav.close("dialog.");
        self.nav.open("dialog.", Key::new("dialog.button", dialog.focus as i32));
    }

    /// Say that something failed, in the player's words: a dialog with one button. What the code said of it
    /// is the caller's to log (`crate::errors::log`).
    pub fn fail(&mut self, error: UserError) {
        let ui = self.ui();
        let (title, message) = error.dialog(&ui);
        let ok = ui.global::<nettai_ui::Errors>().invoke_ok();
        self.sound.play(Cue::Refused);
        self.open_dialog(Dialog { tag: "", title, message, buttons: vec![button(&ok, ButtonKind::Main)], focus: 0, cancel: 0 }, None);
    }

    /// As [`Shell::fail`], with the screen's own button (BACK TO PLAY) and the answer told to it by `tag`.
    pub fn fail_with(&mut self, error: UserError, tag: &'static str, button_text: &str) {
        let (title, message) = error.dialog(&self.ui());
        self.sound.play(Cue::Refused);
        self.ask(Dialog { tag, title, message, buttons: vec![button(button_text, ButtonKind::Main)], focus: 0, cancel: 0 });
    }

    fn close_dialog(&mut self) -> Option<Asked> {
        let asked = self.dialog.take()?;
        self.ui().global::<nettai_ui::Shell>().set_dialog(false);
        self.nav.close("dialog.");
        Some(asked)
    }

    /// Open a select's choices as a list by it (on a phone, a sheet from the bottom): `id` and `index` are the
    /// select's own, and the screen hears the choice as `Screen::choose(id, index, choice)`.
    pub fn pick(&mut self, id: &str, index: i32, options: Vec<String>, chosen: usize) {
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        let key = Key::new(id, index);
        let by = self.nav.registry().borrow().rect(&key).unwrap_or_default();
        let height = options.len() as f32 * 44.0 + 2.0;
        let window = ui.window().size().to_logical(ui.window().scale_factor());
        // (Under the select, or over it where there is no room under.)
        let y = if by.bottom() + 4.0 + height <= window.height { by.bottom() + 4.0 } else { (by.y - 4.0 - height).max(0.0) };
        shell.set_picker_options(ModelRc::new(VecModel::from(options.into_iter().map(Into::into).collect::<Vec<slint::SharedString>>())));
        shell.set_picker_chosen(chosen as i32);
        shell.set_picker_x(by.x);
        shell.set_picker_y(y);
        shell.set_picker_width(by.w);
        shell.set_picker(true);
        self.picking = Some(Picking { owner: self.screen, key });
        self.nav.open("pick.", Key::new("pick.option", chosen as i32));
    }

    fn close_picker(&mut self) -> Option<Picking> {
        let picking = self.picking.take()?;
        self.ui().global::<nettai_ui::Shell>().set_picker(false);
        self.nav.close("pick.");
        Some(picking)
    }

    // ---- The system's ------------------------------------------------------------------------------------

    /// Copy `text` to the system's clipboard (an invite's code, an address).
    pub fn copy(&self, text: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        match arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_string())) {
            Ok(()) => {}
            Err(why) => crate::errors::log("copying to the clipboard", &why),
        }
    }

    /// Ask the player for a file to open, in the system's dialog: `kind` names what is asked for (in the
    /// language shown), `extensions` are those it may have. None: they chose none. The app's frames wait
    /// while the dialog is up.
    pub fn choose_file(&self, kind: &str, extensions: &[&str]) -> Option<std::path::PathBuf> {
        #[cfg(not(target_arch = "wasm32"))]
        return rfd::FileDialog::new().add_filter(kind, extensions).pick_file();
        #[cfg(target_arch = "wasm32")]
        None
    }

    /// Ask the player where to save a file named `name`, as [`Shell::choose_file`].
    pub fn choose_save(&self, kind: &str, extensions: &[&str], name: &str) -> Option<std::path::PathBuf> {
        #[cfg(not(target_arch = "wasm32"))]
        return rfd::FileDialog::new().add_filter(kind, extensions).set_file_name(name).save_file();
        #[cfg(target_arch = "wasm32")]
        None
    }

    // ---- The window's scale ------------------------------------------------------------------------------

    /// Fit the layout to the window: one unit is a 1280th of its width or an 800th of its height, whichever
    /// is less (upright, a 390th of its width), and the window's scale is set to that. True if the layout
    /// turned between the desktop's and the phone's.
    fn fit_window(&mut self) -> bool {
        let ui = self.ui();
        let window = ui.window();
        let size = window.size();
        if size.width == 0 || size.height == 0 {
            return false;
        }
        let os = window.with_winit_window(|w| w.scale_factor() as f32).unwrap_or(1.0).max(0.1);
        let (width, height) = (size.width as f32 / os, size.height as f32 / os);
        let phone = width < 0.8 * height;
        let unit = if phone { width / 390.0 } else { (width / 1280.0).min(height / 800.0) }.max(0.2);
        let factor = os * unit;
        let turned = phone != self.scale.phone;
        let resized = (size.width, size.height) != self.scale.size || (window.scale_factor() - factor).abs() > 0.0005;
        self.scale = Scale { os, unit, phone, size: (size.width, size.height), ground: self.scale.ground };
        if resized {
            // (Slint's own scale is the system's times the unit: its `px` are the UI's units.)
            window.dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged { scale_factor: factor });
            window.dispatch_event(slint::platform::WindowEvent::Resized { size: size.to_logical(factor) });
        }
        ui.global::<Theme>().set_phone(phone);
        // The ground's hexagons, for the window in units.
        let units = ((size.width as f32 / factor).ceil() as i32, (size.height as f32 / factor).ceil() as i32);
        if units != self.scale.ground {
            self.scale.ground = units;
            ui.global::<nettai_ui::Shell>().set_hexagons(hexagons(units.0, units.1).into());
        }
        turned
    }
}

/// The ground's field of hexagons over a window of `width` by `height` units, as a path: a tile 42 by 72.75
/// holds one hexagon and the stem to the one under it, and each tile draws the edges its neighbors don't.
fn hexagons(width: i32, height: i32) -> String {
    use std::fmt::Write as _;
    const TILE: (f32, f32) = (42.0, 72.75);
    let (columns, rows) = ((width as f32 / TILE.0).ceil() as i32 + 1, (height as f32 / TILE.1).ceil() as i32 + 1);
    let mut path = String::new();
    for row in 0..rows {
        for column in 0..columns {
            let (x, y) = (column as f32 * TILE.0, row as f32 * TILE.1);
            // The left edge up to the top and down the right shoulder; the two lower edges; the stem.
            let _ = write!(
                path,
                "M{:.2} {:.2}V{:.2}L{:.2} {:.2}L{:.2} {:.2}M{:.2} {:.2}L{:.2} {:.2}L{:.2} {:.2}M{:.2} {:.2}V{:.2}",
                x,
                y + 36.38,
                y + 12.12,
                x + 21.0,
                y,
                x + 42.0,
                y + 12.12,
                x,
                y + 36.38,
                x + 21.0,
                y + 48.5,
                x + 42.0,
                y + 36.38,
                x + 21.0,
                y + 48.5,
                y + 72.75,
            );
        }
    }
    path
}

/// What the UI's callbacks tell the app (`main::wire`).
pub enum NavInput {
    /// A click or a tap on a control.
    Activate(String, i32),
    /// A click on a stepper's arrow.
    Step(String, i32, i32),
    /// A choice clicked.
    Choose(String, i32, i32),
    /// The pointer went down on a control.
    Point(String, i32),
    /// A field's edit began with this text, changed to this text, ended on this text.
    EditBegan(String),
    Edited(String),
    EditDone(String),
    /// The on-screen pad: a GBA button's bit number, held or let go.
    Touch(i32, bool),
}

pub struct App {
    pub shell: Shell,
    /// The screens' modules, by [`ScreenId::ALL`]'s order. One is taken out while it is called.
    screens: Vec<Option<Box<dyn Screen>>>,
}

impl App {
    pub fn new(ui: &AppWindow, still: bool) -> App {
        let settings = if still { Settings::default() } else { Settings::load() };
        let lang = crate::lang::initial(settings.language.as_deref());
        crate::lang::select(lang);
        ui.global::<Theme>().set_cjk(crate::lang::cjk(lang));
        let mut sound = if still { Sound::silent() } else { Sound::open() };
        sound.set_levels(settings.music, settings.effects);
        let bindings = settings.bindings();
        let shell = Shell {
            ui: ui.as_weak(),
            games: Games::find(),
            settings,
            sound,
            nav: Nav::new(),
            keys: Keys::default(),
            pad: Pad::start(),
            bindings,
            touch: 0,
            lang,
            gl: None,
            scale: Scale { os: 1.0, unit: 1.0, ..Scale::default() },
            sharp: std::env::var_os("NETTAI_PHYSICAL_PIXELS").is_some(),
            shown_stage: 0,
            game: None,
            announced: None,
            runtime_due: 0,
            screen: ScreenId::Menu,
            asks: Vec::new(),
            dialog: None,
            picking: None,
            battle: None,
            loading: None,
            hold_cards: false,
            hold_loading: None,
            still,
            chord: false,
            shift: false,
            composing: false,
            keep_nothing: still,
        };
        App { shell, screens: screens::all().into_iter().map(Some).collect() }
    }

    /// The app starts: on the main menu, the game last used loading first.
    pub fn start(&mut self) {
        let ui = self.shell.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        shell.set_version(format!("v{}", env!("CARGO_PKG_VERSION")).into());
        shell.set_player(self.shell.settings.name.as_str().into());
        shell.set_packs_folder(nettai_content::pack::packs_dir().display().to_string().into());
        if ui.window().is_fullscreen() != self.shell.settings.full_screen {
            ui.window().set_fullscreen(self.shell.settings.full_screen);
        }
        self.shell.show_bindings();
        let games = self.shell.games.playable();
        let first = self.shell.settings.game.clone().filter(|g| games.contains(g)).or_else(|| games.first().cloned());
        match first {
            Some(id) => {
                self.shell.game = Some(id.clone());
                self.shell.load(&id);
                shell.set_game(Shell::game_name(&id).into());
            }
            None => shell.set_no_games(true),
        }
        shell.set_screen(ScreenId::Menu.view());
        let main = self.screen(ScreenId::Menu).map_or("back", |s| s.main());
        self.shell.nav.enter(ScreenId::Menu.name(), main);
        self.call(ScreenId::Menu, |s, shell| s.show(shell));
        self.settle();
    }

    fn slot(id: ScreenId) -> usize {
        ScreenId::ALL.iter().position(|s| *s == id).expect("every screen is listed")
    }

    fn screen(&self, id: ScreenId) -> Option<&dyn Screen> {
        self.screens[App::slot(id)].as_deref()
    }

    /// Call screen `id`'s module with the shell. (None: it is the one calling, which the shell never does.)
    fn call<R>(&mut self, id: ScreenId, f: impl FnOnce(&mut dyn Screen, &mut Shell) -> R) -> Option<R> {
        let mut screen = self.screens[App::slot(id)].take()?;
        let r = f(screen.as_mut(), &mut self.shell);
        self.screens[App::slot(id)] = Some(screen);
        Some(r)
    }

    /// The screen that hears what happens: the battle's owner while one is on, else the screen shown.
    fn hearer(&self) -> ScreenId {
        self.shell.battle.as_ref().map_or(self.shell.screen, |b| b.owner)
    }

    /// Do what the screens asked of the app while they were called.
    fn settle(&mut self) {
        while !self.shell.asks.is_empty() {
            for ask in std::mem::take(&mut self.shell.asks) {
                match ask {
                    Ask::Go(to) => self.show_screen(to),
                    Ask::Refresh => {
                        let shown = self.shell.screen;
                        self.call(shown, |s, shell| s.show(shell));
                    }
                    Ask::Quit => {
                        let _ = slint::quit_event_loop();
                    }
                }
            }
        }
    }

    fn show_screen(&mut self, to: ScreenId) {
        let from = self.shell.screen;
        if from == to {
            return;
        }
        self.call(from, |s, shell| s.hide(shell));
        self.shell.close_picker();
        self.shell.screen = to;
        self.shell.lock_game(false);
        self.shell.ui().global::<nettai_ui::Shell>().set_screen(to.view());
        let main = self.screen(to).map_or("back", |s| s.main());
        self.shell.nav.enter(to.name(), main);
        self.call(to, |s, shell| s.show(shell));
    }

    /// Go to a screen at once (the screenshot pass, and the development variables).
    pub fn go_now(&mut self, to: ScreenId) {
        self.shell.go(to);
        self.settle();
    }

    /// Call the screen shown (the screenshot pass sets a screen up through its own module).
    pub fn with_screen(&mut self, id: ScreenId, f: impl FnOnce(&mut dyn Screen, &mut Shell)) {
        self.call(id, f);
        self.settle();
    }

    // ---- What happens ------------------------------------------------------------------------------------

    /// What a key did to the focused control, carried to whose it is.
    fn did(&mut self, did: Did) {
        match did {
            Did::Nothing => {}
            Did::Activate(key) => {
                self.shell.sound.play(Cue::Pick);
                match key.id.as_str() {
                    "dialog.button" => self.answer(key.index as usize),
                    "pick.option" => self.picked(Some(key.index)),
                    "pick.close" => self.picked(None),
                    "back" => self.back(),
                    "pad.menu" => self.shell.open_menu(),
                    "game" | "" => {}
                    id => {
                        let hearer = self.hearer();
                        self.call(hearer, |s, shell| s.activate(shell, id, key.index));
                    }
                }
            }
            Did::Step(key, by) => {
                self.shell.sound.play(Cue::Cursor);
                if key.id == "game" {
                    self.shell.step_game(by);
                } else {
                    let hearer = self.hearer();
                    self.call(hearer, |s, shell| s.step(shell, &key.id, key.index, by));
                }
            }
            Did::Edit { key, text, done } => {
                let hearer = self.hearer();
                self.call(hearer, |s, shell| s.edit(shell, &key.id, key.index, &text, done));
            }
        }
        self.settle();
    }

    /// A dialog's button: the dialog closes, and the screen that asked hears which.
    fn answer(&mut self, choice: usize) {
        let Some(asked) = self.shell.close_dialog() else { return };
        if let Some(owner) = asked.owner {
            self.call(owner, |s, shell| s.answered(shell, asked.tag, choice));
        }
    }

    /// A choice of a select's list (none: the list closed without one).
    fn picked(&mut self, choice: Option<i32>) {
        let Some(picking) = self.shell.close_picker() else { return };
        if let Some(choice) = choice {
            self.call(picking.owner, |s, shell| s.choose(shell, &picking.key.id, picking.key.index, choice));
        }
    }

    /// Esc, the pad's east button, the header's Back: out of what is open over the screen, else back within
    /// the screen, else to the menu.
    fn back(&mut self) {
        if self.shell.picking.is_some() {
            self.picked(None);
        } else if let Some(cancel) = self.shell.dialog.as_ref().map(|d| d.cancel) {
            self.answer(cancel);
        } else if self.shell.battle.as_ref().is_some_and(Battle::menu_open) {
            self.shell.close_menu();
        } else {
            let shown = self.shell.screen;
            let within = self.call(shown, |s, shell| s.back(shell)).unwrap_or(false);
            if !within && shown != ScreenId::Menu {
                self.shell.go(ScreenId::Menu);
            }
            if within || shown != ScreenId::Menu {
                self.shell.sound.play(Cue::Back);
            }
        }
        self.settle();
    }

    /// One of the menus' actions, from a key or a pad's button.
    pub fn act(&mut self, action: Action) {
        let ui = self.shell.ui();
        let over = self.shell.dialog.is_some() || self.shell.picking.is_some();
        match action {
            Action::Up | Action::Down | Action::Left | Action::Right => {
                let dir = match action {
                    Action::Up => Direction::Up,
                    Action::Down => Direction::Down,
                    Action::Left => Direction::Left,
                    _ => Direction::Right,
                };
                let before = self.shell.nav.focused().cloned();
                let did = self.shell.nav.go(&ui, dir);
                if self.shell.nav.focused() != before.as_ref() {
                    self.shell.sound.play(Cue::Cursor);
                }
                self.did(did);
            }
            Action::NextFocus | Action::PreviousFocus => {
                self.shell.nav.tab(&ui, action == Action::PreviousFocus);
                self.shell.sound.play(Cue::Cursor);
            }
            Action::PageUp => self.shell.nav.rows(&ui, -1, true),
            Action::PageDown => self.shell.nav.rows(&ui, 1, true),
            Action::Home => self.shell.nav.rows(&ui, i32::MIN, false),
            Action::End => self.shell.nav.rows(&ui, i32::MAX, false),
            Action::Confirm => {
                let did = self.shell.nav.confirm(&ui);
                self.did(did);
            }
            // (The pad's Start: nettai's menu in a battle, shut again by it; Enter's in a menu.)
            Action::Menu if self.shell.battle.as_ref().is_some_and(Battle::menu_open) && !over => self.shell.close_menu(),
            Action::Menu => {
                let did = self.shell.nav.confirm(&ui);
                self.did(did);
            }
            Action::Back => self.back(),
            Action::Previous | Action::Next | Action::Delete | Action::Alternate if !over => {
                let hearer = self.hearer();
                let taken = self.call(hearer, |s, shell| s.action(shell, action)).unwrap_or(false);
                // The shoulders change the game where the header offers it.
                if !taken && self.shell.battle.is_none() && self.shell.screen != ScreenId::Settings {
                    match action {
                        Action::Previous => self.did(Did::Step(Key::new("game", 0), -1)),
                        Action::Next => self.did(Did::Step(Key::new("game", 0), 1)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        self.settle();
    }

    /// What the UI's callbacks say: a click, an arrow, a field's edit.
    pub fn input(&mut self, input: NavInput) {
        let ui = self.shell.ui();
        match input {
            NavInput::Activate(id, index) => {
                let did = self.shell.nav.clicked(&ui, &id, index);
                self.did(did);
            }
            NavInput::Step(id, index, by) => self.did(Did::Step(Key::new(&id, index), by)),
            NavInput::Choose(id, index, choice) => {
                let hearer = self.hearer();
                self.call(hearer, |s, shell| s.choose(shell, &id, index, choice));
            }
            NavInput::Point(id, index) => {
                self.shell.nav.used_pointer();
                let did = self.shell.nav.pointed(&id, index);
                self.did(did);
            }
            NavInput::EditBegan(text) => {
                let did = self.shell.nav.edit_began(&text);
                self.did(did);
            }
            NavInput::Edited(text) => {
                let did = self.shell.nav.edited(&text);
                self.did(did);
            }
            NavInput::EditDone(text) => {
                self.shell.nav.edited(&text);
                let did = self.shell.nav.end_edit(true);
                self.did(did);
            }
            NavInput::Touch(bit, down) => {
                let bit = 1u16 << bit.clamp(0, 9);
                if down {
                    self.shell.touch |= bit;
                } else {
                    self.shell.touch &= !bit;
                }
            }
        }
        self.settle();
    }

    /// The modifiers held changed.
    pub fn modifiers(&mut self, shift: bool, chord: bool) {
        self.shell.shift = shift;
        self.shell.chord = chord;
    }

    /// The input method started or stopped composing.
    pub fn composing(&mut self, composing: bool) {
        self.shell.composing = composing;
    }

    /// The pointer moved, clicked or scrolled: the cursor hides.
    pub fn pointer(&mut self) {
        self.shell.nav.used_pointer();
    }

    /// The window lost the keyboard: nothing is held.
    pub fn unfocused(&mut self) {
        self.shell.keys.release();
    }

    /// A key of the window, before Slint sees it. True if it was the app's (Slint doesn't get it); false and
    /// it goes on to Slint (a text field being typed in, a shortcut of the system's).
    pub fn key(&mut self, event: &KeyEvent) -> bool {
        let now = Instant::now();
        let PhysicalKey::Code(code) = event.physical_key else { return false };
        // The battle's buttons follow the keys whatever has them, so none sticks.
        self.shell.keys.event(&self.shell.bindings, event, now);
        let editing = self.shell.nav.editing();
        if event.state != ElementState::Pressed {
            return !editing;
        }
        let press = KeyPress { code, text: event.text.as_ref().map(|t| t.to_string()), repeat: event.repeat, shift: self.shell.shift };
        // The battle has the keys: Esc is nettai's menu, a key bound to no button may be its owner's.
        if self.shell.battle_has_keys() {
            self.shell.nav.used_keys(false);
            if code == KeyCode::Escape {
                self.shell.open_menu();
            } else if self.shell.bindings.of_key(code).is_none() && !event.repeat {
                let hearer = self.hearer();
                self.call(hearer, |s, shell| s.key(shell, &press));
                self.settle();
            }
            return true;
        }
        // A text field being edited: the keys are its own, but those that leave it.
        if editing {
            if self.shell.composing {
                return false;
            }
            let ui = self.shell.ui();
            match code {
                KeyCode::Escape => {
                    let did = self.shell.nav.end_edit(false);
                    self.did(did);
                }
                KeyCode::ArrowUp | KeyCode::ArrowDown => {
                    let did = self.shell.nav.end_edit(true);
                    self.did(did);
                    self.shell.nav.go(&ui, if code == KeyCode::ArrowUp { Direction::Up } else { Direction::Down });
                }
                KeyCode::Tab => {
                    let did = self.shell.nav.end_edit(true);
                    self.did(did);
                    self.shell.nav.tab(&ui, self.shell.shift);
                }
                _ => return false,
            }
            self.shell.nav.used_keys(false);
            return true;
        }
        if self.shell.chord {
            return false;
        }
        // The screen may want the key for itself (a key being bound, a letter that acts on a row).
        let over = self.shell.dialog.is_some() || self.shell.picking.is_some();
        if !over {
            let hearer = self.hearer();
            let taken = self.call(hearer, |s, shell| s.key(shell, &press)).unwrap_or(false);
            self.settle();
            if taken {
                self.shell.nav.used_keys(false);
                return true;
            }
        }
        // A letter typed on a text field starts editing it.
        if self.shell.nav.on_field()
            && let Some(text) = press.text.as_deref().filter(|t| !t.is_empty() && t.chars().all(|c| !c.is_control()))
        {
            self.shell.nav.used_keys(false);
            self.shell.nav.begin_edit(&self.shell.ui(), text);
            return true;
        }
        let action = match code {
            _ if self.shell.bindings.of_key(code) == Some(keys::L) && menu_action(code, false).is_none() => Some(Action::Previous),
            _ if self.shell.bindings.of_key(code) == Some(keys::R) && menu_action(code, false).is_none() => Some(Action::Next),
            _ => menu_action(code, self.shell.shift),
        };
        let Some(action) = action else { return false };
        // (A key held repeats a move; a choice or a way back is pressed once.)
        if event.repeat && !matches!(action, Action::Up | Action::Down | Action::Left | Action::Right | Action::NextFocus | Action::PreviousFocus | Action::PageUp | Action::PageDown) {
            return true;
        }
        self.shell.nav.used_keys(false);
        self.act(action);
        true
    }

    // ---- The frame ---------------------------------------------------------------------------------------

    /// Before a frame is drawn at `now`: the window's scale, the pad, the games loading, the screens, the
    /// battle's ticks and its picture, the sound, the focus.
    pub fn frame(&mut self, now: Instant) {
        // (The last frame was drawn, if the renderer didn't say so.)
        self.drawn();
        if self.shell.fit_window() {
            self.shell.refresh();
        }
        self.read_pad(now);
        self.follow_games();
        let shown = self.shell.screen;
        for id in ScreenId::ALL {
            self.call(id, |s, shell| s.frame(shell, now, id == shown && shell.battle.as_ref().is_none_or(|b| !b.full)));
        }
        self.settle();
        // The battle: loading behind the cards, or running.
        if let Some(owner) = self.shell.load_battle(now) {
            self.call(owner, |s, shell| s.battle(shell, BattleEvent::Started));
            self.settle();
        }
        let mut samples = Vec::new();
        let (ticks, event) = self.shell.run_battle(now, &mut samples);
        if let Some((owner, event)) = event {
            self.call(owner, |s, shell| s.battle(shell, event));
            self.settle();
        }
        // The title music plays on the menu screens.
        let in_battle = self.shell.battle.is_some() || self.shell.loading.is_some();
        self.shell.sound.want_music(!in_battle && self.shell.ready().is_some());
        self.shell.sound.frame(now, ticks, &mut samples);
        // The one focus, and where it is.
        let ui = self.shell.ui();
        if let Some(key) = self.shell.nav.settle(&ui)
            && !matches!(key.id.as_str(), "back" | "game")
            && !key.id.starts_with("dialog.")
            && !key.id.starts_with("pick.")
        {
            let hearer = self.hearer();
            self.call(hearer, |s, shell| s.focused(shell, &key.id, key.index));
            self.settle();
        }
    }

    /// The frame was drawn.
    pub fn drawn(&mut self) {
        if let Some(b) = &mut self.shell.battle {
            b.stage.drawn();
        }
    }

    /// The pad's presses: the battle's (its bound buttons are read as held), a screen's, or the menus'.
    fn read_pad(&mut self, now: Instant) {
        let events = self.shell.pad.poll(&self.shell.bindings, now);
        if events.buttons.is_empty() && events.actions.is_empty() {
            return;
        }
        self.shell.nav.used_keys(true);
        if self.shell.battle_has_keys() {
            if let Some(at) = events.at {
                self.shell.keys.pressed.push(at);
            }
            // (The guide button is nettai's menu, as Esc is; a button bound to nothing may be the owner's.)
            for button in events.buttons {
                if button == gamepad_facade::Button::Guide {
                    self.shell.open_menu();
                } else if !self.shell.bindings.pad.contains(&button) {
                    let hearer = self.hearer();
                    self.call(hearer, |s, shell| s.button(shell, button));
                }
            }
            self.settle();
            return;
        }
        let over = self.shell.dialog.is_some() || self.shell.picking.is_some();
        let mut taken = false;
        if !over && !self.shell.nav.editing() {
            for button in &events.buttons {
                let hearer = self.hearer();
                taken |= self.call(hearer, |s, shell| s.button(shell, *button)).unwrap_or(false);
            }
            self.settle();
        }
        if taken {
            return;
        }
        for action in events.actions {
            // (A field being edited is left by the pad as by the keys: up and down confirm and move on.)
            if self.shell.nav.editing() {
                let keep = action != Action::Back;
                let did = self.shell.nav.end_edit(keep);
                self.did(did);
                if matches!(action, Action::Back | Action::Confirm | Action::Menu) {
                    continue;
                }
            }
            self.act(action);
        }
    }

    /// The games as they load: the bar while the selected one does, the screens shown again once it is ready,
    /// the others loaded behind it; one that can't load is passed over.
    fn follow_games(&mut self) {
        let ui = self.shell.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        if let Some(held) = self.shell.hold_loading {
            shell.set_loading(true);
            shell.set_loading_progress(held);
            return;
        }
        let Some(id) = self.shell.game.clone() else {
            shell.set_no_games(true);
            shell.set_loading(true);
            return;
        };
        match self.shell.games.get(&id).map(|e| &e.state) {
            Some(State::Ready(_)) if self.shell.games.ready(&id).is_none() => {
                // Its parts are read: the bar says so for two frames, then its runtime is loaded here, on the
                // thread that runs the battles (about a second, the window still).
                shell.set_loading(true);
                shell.set_loading_progress(self.shell.games.progress(&id));
                self.shell.runtime_due += 1;
                if self.shell.runtime_due > 2 {
                    self.shell.runtime_due = 0;
                    self.shell.games.load_runtime(&id);
                }
            }
            Some(State::Ready(ready)) => {
                shell.set_loading(false);
                shell.set_loading_progress(1.0);
                if self.shell.announced.as_deref() != Some(id.as_str()) {
                    let ready = ready.clone();
                    self.shell.announced = Some(id.clone());
                    self.shell.sound.use_game(&id, &ready.loaded);
                    self.shell.refresh();
                    // The others' parts, behind it.
                    for other in self.shell.games.playable() {
                        self.shell.load(&other);
                    }
                    self.settle();
                }
            }
            Some(State::Failed) | Some(State::NoPack) | None => {
                // It can't be played: the next that can, and the player told once.
                let games = self.shell.games.playable();
                self.shell.game = None;
                match games.first() {
                    Some(next) => self.shell.select_game(&next.clone()),
                    None => shell.set_no_games(true),
                }
                self.shell.fail(UserError::Other);
            }
            Some(State::NotLoaded) => self.shell.load(&id),
            Some(State::Loading(_)) => {
                shell.set_loading(true);
                shell.set_loading_progress(self.shell.games.progress(&id));
            }
        }
    }

    /// A key probe's press (`NETTAI_KEY_PROBE`): the right arrow, as a key event of the window would press it.
    pub fn probe(&mut self, down: bool) {
        if !self.shell.battle_has_keys() {
            return;
        }
        if down {
            if self.shell.keys.held & keys::RIGHT == 0 {
                self.shell.keys.pressed.push(Instant::now());
            }
            self.shell.keys.held |= keys::RIGHT;
        } else {
            self.shell.keys.held &= !keys::RIGHT;
        }
    }

    /// A file was dropped on the window: the screen shown may want it (a save, for a build of it).
    pub fn dropped(&mut self, path: &std::path::Path) {
        if self.shell.battle.is_some() || self.shell.dialog.is_some() {
            return;
        }
        let shown = self.shell.screen;
        self.call(shown, |s, shell| s.dropped(shell, path));
        self.settle();
    }
}
