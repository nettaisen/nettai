//! The shell (docs/app.md §2): what every screen sits in and asks things of.
//!
//! [`Shell`] is what a screen's module is handed: the window, the games and the selected one, the settings,
//! the sound, the focus, the battle on screen, and the ways to another screen, to a dialog, to a list of
//! choices, to a battle. [`App`] owns the shell and the screens, and carries what happens to the right one: a
//! key, a pad's button and a click all become the same few calls on the screen shown (`crate::screens::Screen`).
//!
//! The window's rendering is the app's clock: [`App::frame`] runs before each frame is drawn (docs/app.md §1).

use crate::battle::{Battle, Loading};
use crate::errors::UserError;
use crate::games::{Games, Notice, Ready, State};
use crate::gl::GlPicture;
use crate::input::{Action, Bindings, KeyPress, Keys, Pad, menu_action};
use crate::nav::{Did, Direction, Key, Nav, Rect};
use crate::screens::{self, Screen, ScreenId};
use crate::settings::Settings;
use crate::sound::{Cue, Sound};
use nettai_battle::input::keys;
use nettai_ui::{AppWindow, ButtonKind, DialogButton, Theme};
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::event::{ElementState, KeyEvent, TouchPhase};
use slint::winit_030::winit::keyboard::{KeyCode, PhysicalKey};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

thread_local! {
    /// Each game's name in the language shown, by the game's id (`Shell::game_name`; `Shell::name_games` reads
    /// them).
    static GAME_NAMES: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

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

/// The shell's own question, no screen's: no Legacy Collection is found, and its first button chooses the
/// collection's folder (`Shell::ask_for_source`). And the control that does the same where no game can be played
/// (the loading screen's).
const SOURCE: &str = "source";
const CHOOSE_SOURCE: &str = "source.choose";

/// The most rows a select's list shows at once (`Shell::pick`): more scroll. And a row's height, the theme's.
const PICKER_ROWS: usize = 10;
const PICKER_ROW: f32 = 44.0;
/// The least width of a list of tags (`Shell::pick_tags`): the longest version's tag in the longest language
/// with the row's room at each side.
const TAG_LIST: f32 = 168.0;
/// The width of a menu of actions (`Shell::pick_menu`): its longest words in the longest language, with a row's
/// room at each side.
const MENU_WIDTH: f32 = 200.0;

/// A grid of pictures' sizes (`Shell::pick_grid`; the UI's `PickerLayer` lays it out by the same): its columns, a
/// cell's width (on a phone, the sheet's width shared by its three), the gap between cells, the room under a
/// cell's picture for its name, the padding around the cells (the scrollbar's 12 besides at the right, and 4
/// more), and the most rows it shows at once. A cell's picture is seven fifteenths as tall as it is wide (the
/// battle's field's band, 240 by 112).
const GRID_COLUMNS: usize = 5;
const GRID_PHONE_COLUMNS: usize = 3;
const GRID_CELL: f32 = 120.0;
const GRID_GAP: f32 = 8.0;
const GRID_PHONE_GAP: f32 = 6.0;
const GRID_NAME: f32 = 34.0;
const GRID_PAD: f32 = 8.0;
const GRID_RIGHT: f32 = 16.0;
const GRID_ROWS: usize = 5;
/// A grid's cell's control (each cell is focused, where a list's rows are its list's).
const GRID_CELL_KEY: &str = "pick.grid.cell";

/// What a list of choices is: a select's, a list of tags, a menu of actions whose rows from `apart` on (where
/// some) are set apart in the error color, or a grid of the choices' pictures.
enum Look {
    Select,
    Tags(Vec<slint::Color>),
    Menu { apart: Option<usize> },
    Grid(ModelRc<slint::Image>),
}

/// A grid's layout: its columns, a cell's width, and the gap between two cells.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Grid {
    columns: usize,
    cell: f32,
    gap: f32,
}

impl Grid {
    /// A grid on a phone's sheet `width` wide, or a desktop's.
    fn of(phone: bool, width: f32) -> Grid {
        let (columns, gap) = if phone { (GRID_PHONE_COLUMNS, GRID_PHONE_GAP) } else { (GRID_COLUMNS, GRID_GAP) };
        let cell = if phone { (width - GRID_PAD - GRID_RIGHT - (columns - 1) as f32 * gap) / columns as f32 } else { GRID_CELL };
        Grid { columns, cell, gap }
    }

    /// The width of its panel: its cells, their gaps and its padding.
    fn width(&self) -> f32 {
        self.columns as f32 * self.cell + (self.columns - 1) as f32 * self.gap + GRID_PAD + GRID_RIGHT
    }

    /// The height of its panel showing `rows` rows of cells.
    fn height(&self, rows: usize) -> f32 {
        rows as f32 * (self.cell * 7.0 / 15.0 + GRID_NAME + self.gap) - self.gap + 2.0 * GRID_PAD
    }

    /// The rows it shows at once of `count` choices, and whether it goes under its select: as many as it has, up
    /// to [`GRID_ROWS`] and to the whole rows there is room for `under` the select or `over` it, as a list's
    /// ([`picker_rows`]).
    fn rows(&self, count: usize, under: f32, over: f32) -> (usize, bool) {
        let fit = |room: f32| (1..=GRID_ROWS).rev().find(|&rows| self.height(rows) <= room).unwrap_or(1);
        let wanted = count.div_ceil(self.columns).clamp(1, GRID_ROWS);
        if wanted <= fit(under) || under >= over { (wanted.min(fit(under)), true) } else { (wanted.min(fit(over)), false) }
    }
}

/// The rows a select's list of `options` choices shows at once, and whether it goes under its select: as many
/// as it has, up to [`PICKER_ROWS`] and to the whole rows there is room for, `under` the select or `over` it.
/// It goes under, unless it doesn't fit there and there is more room over.
fn picker_rows(options: usize, under: f32, over: f32) -> (usize, bool) {
    let fit = |room: f32| ((room - 2.0) / PICKER_ROW).floor().max(1.0) as usize;
    let wanted = options.clamp(1, PICKER_ROWS);
    if wanted <= fit(under) || under >= over { (wanted.min(fit(under)), true) } else { (wanted.min(fit(over)), false) }
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

/// The screen on screen is leaving: the one to show once it has, and when.
struct Swap {
    to: ScreenId,
    at: Instant,
}

/// What a screen asks of the app that is done once the screen has returned.
enum Ask {
    Go(ScreenId),
    /// The screen shown sets its view again (the game, the language or the layout changed under it).
    Refresh,
    /// A screen is told what another has for it (`Shell::tell`).
    Tell(ScreenId, String, usize),
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
    /// The window in units, as the ground's patterns were last drawn for.
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
    pub touch: crate::input::TouchPad,
    /// The on-screen pad's buttons as the UI was last told they are held.
    pad_shown: u16,
    /// The language shown, by its code.
    pub lang: &'static str,
    /// The window's texture the picture is written into (femtovg's OpenGL); none: a new image each picture.
    pub gl: Option<GlPicture>,
    pub scale: Scale,
    /// Present the picture at the display's density (`NETTAI_PHYSICAL_PIXELS`).
    pub(crate) sharp: bool,
    /// Which surface's picture the texture holds, and whether one is shown (docs/app.md §1).
    pub(crate) showing: crate::stage::Showing,
    /// Samples a screen played this frame (`Shell::play_samples`), and the ticks that made them.
    pub(crate) own_samples: Vec<[f32; 2]>,
    pub(crate) own_ticks: Option<u32>,
    /// The selected game.
    game: Option<String>,
    /// Each game's look, and the change of game's sweep and cross-fade under way (docs/app.md §10).
    pub(crate) looks: crate::look::Looks,
    /// The game the screens were last shown with, once it was ready.
    announced: Option<String>,
    /// The frames the bar has shown the selected game's parts as read, before its runtime is loaded (which
    /// holds the window still: the bar is drawn first).
    runtime_due: u8,
    screen: ScreenId,
    /// The screen shown is still the one before `screen`, leaving (docs/app.md §10).
    swap: Option<Swap>,
    /// Motion is turned down: the player's setting, else the system's.
    reduce_motion: bool,
    flights: i32,
    asks: Vec<Ask>,
    pub(crate) dialog: Option<Asked>,
    picking: Option<Picking>,
    pub battle: Option<Battle>,
    pub(crate) loading: Option<Loading>,
    /// The practice session on, if one is: any screen may start one (`crate::screens::practice::start`).
    pub practice: screens::practice::Shared,
    /// The screenshot pass holds things still: the two cards stay up whatever is ready, the loading bar is
    /// shown at a place of its choosing, and the menu's reel stops on one picture (`still`). Its motion probe
    /// lets things move all the same (`probe_motion`), and its showreel lets the battles play too, the menu's reel
    /// and a replay's viewer (`reel`: `tour/reel.rs`).
    pub hold_cards: bool,
    pub hold_loading: Option<f32>,
    pub still: bool,
    pub probe_motion: bool,
    pub reel: bool,
    /// A screen has something running that must keep its time whatever covers the window (a link to another
    /// player, before and between their battles): the app isn't napped while this is set, as it isn't while a
    /// battle runs (`awake.rs`).
    pub stay_awake: bool,
    awake: Option<crate::awake::Awake>,
    /// This frame is being drawn (the window's rendering runs it). When the window isn't drawn (minimized,
    /// hidden, covered), a timer runs the frames instead, and no picture is presented in them: there is no
    /// frame to show it in, and the window's OpenGL isn't the thread's.
    pub(crate) drawing: bool,
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

    /// Tell screen `to` that something it asked of another screen was answered: it hears it as a dialog's
    /// answer, `Screen::answered(tag, choice)`, once the screen calling has returned (a row it added to the
    /// practice menu was chosen: `crate::screens::practice`).
    pub fn tell(&mut self, to: ScreenId, tag: &str, choice: usize) {
        self.asks.push(Ask::Tell(to, tag.to_string(), choice));
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

    /// A game's name as shown, in the language shown: the game's own, from its locales (`exe6`: BN6 in English,
    /// EXE6 in Japanese; `nettai_content::locale::game_name`), as [`Shell::name_games`] read them. A game whose
    /// locales can't be read is shown by its id, in capitals.
    pub fn game_name(id: &str) -> String {
        GAME_NAMES.with(|names| names.borrow().get(id).cloned()).unwrap_or_else(|| id.to_ascii_uppercase())
    }

    /// Read each game's name in the language shown, for [`Shell::game_name`]: at start-up and when the language
    /// changes. From the content's locales alone (a game without its pack has its name too); the header's name
    /// is told again.
    fn name_games(&self) {
        let ids: Vec<&str> = self.games.list.iter().map(|g| g.id.as_str()).collect();
        Shell::read_game_names(&ids, self.lang);
        if let Some(id) = &self.game {
            self.ui().global::<nettai_ui::Shell>().set_game(Shell::game_name(id).into());
        }
    }

    /// Read games `ids`' names in `lang` (for [`Shell::game_name`]), their battle themes' (for
    /// `crate::music::name`) and their backgrounds' (for `crate::backgrounds::name`).
    pub(crate) fn read_game_names(ids: &[&str], lang: &str) {
        let dir = nettai_content::index::content();
        let names: HashMap<String, String> = ids.iter().filter_map(|id| nettai_content::locale::game_name(&dir, id, lang).map(|name| (id.to_string(), name))).collect();
        GAME_NAMES.with(|n| *n.borrow_mut() = names);
        crate::music::name_themes(ids.iter().copied(), lang);
        crate::backgrounds::name_backgrounds(ids.iter().copied(), lang);
    }

    /// Where the game can't change (inside a build, a lobby, a replay), its name stays in the header without
    /// its arrows.
    pub fn lock_game(&mut self, locked: bool) {
        self.ui().global::<nettai_ui::Shell>().set_game_locked(locked);
    }

    fn game_locked(&self) -> bool {
        self.ui().global::<nettai_ui::Shell>().get_game_locked()
    }

    /// Select a game: it loads if it hasn't, and the screen shown is shown again with it once it is ready. Its
    /// look sweeps in from the right.
    pub fn select_game(&mut self, id: &str) {
        self.select_game_from(id, 1);
    }

    /// [`Shell::select_game`], its look sweeping in from side `from` (-1 the left, 1 the right: the side of the
    /// arrow pressed).
    fn select_game_from(&mut self, id: &str, from: i32) {
        if self.game.as_deref() == Some(id) {
            return;
        }
        self.game = Some(id.to_string());
        self.settings.game = Some(id.to_string());
        self.save_settings();
        self.games.prefer(id);
        self.load(id);
        let ui = self.ui();
        ui.global::<nettai_ui::Shell>().set_game(Shell::game_name(id).into());
        self.looks.show(&ui, Some(id), from);
        ui.global::<nettai_ui::Shell>().set_no_games(false);
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
        self.select_game_from(&next, by.signum());
    }

    /// Start loading a game on its thread.
    fn load(&mut self, id: &str) {
        self.games.load(id, self.lang, |id, step| crate::with_app(|app| app.shell.games.step(&id, step)));
    }

    /// The extraction of game `id`'s pack says how far it is, or that it ended (`crate::packs`). A pack that
    /// lands behind the menu is loaded there; the selected game's loads as `App::follow_games` sees it.
    pub fn extracting(&mut self, id: &str, told: crate::packs::Told) {
        if self.games.extracting(id, told) && self.announced.is_some() {
            self.load(id);
        }
    }

    /// Say that a game has no pack and no Legacy Collection is found to extract it from: the error's dialog,
    /// with the way to choose the collection's folder (`App::choose_source`).
    pub fn ask_for_source(&mut self) {
        let ui = self.ui();
        let (title, message) = UserError::NoLegacyCollection.dialog(&ui);
        let errors = ui.global::<nettai_ui::Errors>();
        let buttons = vec![button(&errors.invoke_choose_folder(), ButtonKind::Main), button(&errors.invoke_ok(), ButtonKind::Normal)];
        self.sound.play(Cue::Refused);
        self.open_dialog(Dialog { tag: SOURCE, title, message, buttons, focus: 0, cancel: 1 }, None, true);
    }

    // ---- The settings and the language -------------------------------------------------------------------

    /// The player's name: kept, and shown where the shell shows it.
    pub fn set_name(&mut self, name: &str) {
        self.settings.name = crate::settings::clean_name(name);
        self.ui().global::<nettai_ui::Shell>().set_player(self.settings.name.as_str().into());
        self.save_settings();
    }

    /// The battle's music the player picked (`crate::music`), for this visit to Practice
    /// (`screens::practice::Visit`): Practice's own sessions play it (`BattleMusic::Picked`).
    pub fn battle_music(&self) -> crate::music::Music {
        self.practice.visit.music.clone()
    }

    /// Pick the battle's music, for the rest of this visit to Practice and the next battle the shell makes ready
    /// (a battle running goes on with what it has: `crate::music::retheme` changes it).
    pub fn set_battle_music(&mut self, music: &crate::music::Music) {
        self.practice.visit.music = music.clone();
    }

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
        self.name_games();
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
            b.stage.surface.stale = true;
        }
        self.refresh();
    }

    /// The Music and Sound effects levels, each 0 to 100: the menus' and the title music's, and the battle's
    /// own music and effects (a screen with a replay's viewer gives it `sound.battle_levels()` itself).
    pub fn set_sound_levels(&mut self, music: u32, effects: u32) {
        self.sound.set_levels(music, effects);
        let (music, effects) = self.sound.battle_levels();
        if let Some(b) = &mut self.battle {
            let music = if b.music == crate::battle::BattleMusic::None { 0.0 } else { music };
            b.stage.player.set_sound_levels(music, effects);
        }
    }

    /// Tell the UI which of the on-screen pad's buttons are held, if that changed: it draws them pressed.
    pub(crate) fn show_pad(&mut self) {
        let held = self.touch.held();
        if held != self.pad_shown {
            self.pad_shown = held;
            let buttons: Vec<bool> = (0..10).map(|bit| held & 1 << bit != 0).collect();
            self.ui().global::<nettai_ui::Shell>().set_pad_held(ModelRc::new(VecModel::from(buttons)));
        }
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
        // (The keys that confirm and go back are Enter and Esc; the pad's, the buttons bound to A and B.)
        shell.set_key_a("Enter".into());
        shell.set_key_b("Esc".into());
        shell.set_pad_a(crate::input::pad_face(self.bindings.confirm_button()));
        shell.set_pad_b(crate::input::pad_face(self.bindings.back_button()));
        shell.set_pad_delete(crate::input::pad_face(self.bindings.delete_button()));
    }

    // ---- Motion ------------------------------------------------------------------------------------------

    /// Whether motion is turned down (the player's Reduce motion, else the system's).
    pub fn reduces_motion(&self) -> bool {
        self.reduce_motion
    }

    /// Turn motion down or up, and keep the choice.
    pub fn set_reduce_motion(&mut self, reduce: bool) {
        self.settings.reduce_motion = Some(reduce);
        self.save_settings();
        self.apply_motion();
    }

    /// Tell the UI whether things move: not with motion turned down, and not in the screenshot pass (where not
    /// even what Reduce motion keeps does: the jack-in's fade).
    pub(crate) fn apply_motion(&mut self) {
        self.reduce_motion = self.settings.reduce_motion.unwrap_or_else(crate::settings::system_reduces_motion);
        let ui = self.ui();
        let theme = ui.global::<Theme>();
        theme.set_motion(if self.still { self.probe_motion } else { !self.reduce_motion });
        theme.set_still(self.still && !self.probe_motion);
    }

    /// Have something fly over the layout from one control to another (a chip from its library row to its
    /// slot): `image` at 16 by 16 with `text` beside it, for the theme's `settle`. Each is a control's name and
    /// index; nothing flies if either isn't on screen.
    pub fn fly(&mut self, from: (&str, i32), to: (&str, i32), image: slint::Image, text: &str) {
        let registry = self.nav.registry();
        let registry = registry.borrow();
        let (Some(a), Some(b)) = (registry.rect(&Key::new(from.0, from.1)), registry.rect(&Key::new(to.0, to.1))) else { return };
        let ui = self.ui();
        let fx = ui.global::<nettai_ui::Fx>();
        fx.set_fly_from_x(a.x + 8.0);
        fx.set_fly_from_y(a.y + (a.h - 20.0) / 2.0);
        fx.set_fly_to_x(b.x + 8.0);
        fx.set_fly_to_y(b.y + (b.h - 20.0) / 2.0);
        fx.set_fly_image(image);
        fx.set_fly_text(text.into());
        self.flights += 1;
        fx.set_fly_seq(self.flights);
    }

    /// A screen is leaving (docs/app.md §10): the screen shown is still the one before.
    pub(crate) fn leaving(&self) -> bool {
        self.swap.is_some()
    }

    /// The screen that was leaving has left (or is cut short): the next is shown.
    fn finish_swap(&mut self) {
        let Some(swap) = self.swap.take() else { return };
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        shell.set_screen(swap.to.view());
        shell.set_leaving(false);
        self.nav.hold(false);
    }

    // ---- Dialogs and lists of choices --------------------------------------------------------------------

    /// Ask the player something: the screen shown hears the answer (`Screen::answered`).
    pub fn ask(&mut self, dialog: Dialog) {
        let owner = Some(self.battle.as_ref().map_or(self.screen, |b| b.owner));
        self.open_dialog(dialog, owner, false);
    }

    fn open_dialog(&mut self, dialog: Dialog, owner: Option<ScreenId>, shake: bool) {
        self.finish_swap();
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        self.close_picker();
        shell.set_dialog_title(dialog.title.as_str().into());
        shell.set_dialog_message(dialog.message.as_str().into());
        shell.set_dialog_buttons(ModelRc::new(VecModel::from(dialog.buttons)));
        shell.set_dialog_shake(shake);
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
        self.open_dialog(Dialog { tag: "", title, message, buttons: vec![button(&ok, ButtonKind::Main)], focus: 0, cancel: 0 }, None, true);
    }

    /// As [`Shell::fail`], with the screen's own button (BACK TO PLAY) and the answer told to it by `tag`.
    pub fn fail_with(&mut self, error: UserError, tag: &'static str, button_text: &str) {
        let (title, message) = error.dialog(&self.ui());
        self.sound.play(Cue::Refused);
        let owner = Some(self.battle.as_ref().map_or(self.screen, |b| b.owner));
        self.open_dialog(Dialog { tag, title, message, buttons: vec![button(button_text, ButtonKind::Main)], focus: 0, cancel: 0 }, owner, true);
    }

    fn close_dialog(&mut self) -> Option<Asked> {
        let asked = self.dialog.take()?;
        self.ui().global::<nettai_ui::Shell>().set_dialog(false);
        self.nav.close("dialog.");
        Some(asked)
    }

    /// Open a select's choices as a list by it (on a phone, a sheet from the bottom): `id` and `index` are the
    /// select's own, and the screen hears the choice as `Screen::choose(id, index, choice)`. The list shows as
    /// many rows as it has, up to [`PICKER_ROWS`] and to what the window has room for by the select; more
    /// scroll, the chosen one in view as it opens.
    pub fn pick(&mut self, id: &str, index: i32, options: Vec<String>, chosen: usize) {
        self.open_picker(Key::new(id, index), &Key::new(id, index), options, Look::Select, chosen as i32);
    }

    /// Open a menu of actions (Play's build menu) as a select's list, by the control `at` (its ⋯ button, or the
    /// row a right click opened it on), [`MENU_WIDTH`] wide with its right edge at `at`'s: the screen hears the
    /// choice as `Screen::choose(id, index, choice)`. The rows from `apart` on are set apart under a line, in the
    /// error color (DELETE). No row is marked; the cursor starts on the first.
    pub fn pick_menu(&mut self, id: &str, index: i32, at: &Key, options: Vec<String>, apart: Option<usize>) {
        self.open_picker(Key::new(id, index), at, options, Look::Menu { apart }, -1);
    }

    /// Open a list of tags by a tag that opens a choice (the editor's version), as [`Shell::pick`] does a
    /// select's: each choice its text as a tag in its color. The list is at least [`TAG_LIST`] wide, the tag
    /// it opens from being narrower than its choices may be.
    pub fn pick_tags(&mut self, id: &str, index: i32, tags: Vec<nettai_ui::VersionTag>, chosen: usize) {
        let (options, tones) = tags.into_iter().map(|tag| (tag.text.to_string(), tag.tone)).unzip();
        self.open_picker(Key::new(id, index), &Key::new(id, index), options, Look::Tags(tones), chosen as i32);
    }

    /// Open a select's choices as a grid of their pictures by it (Practice's Arena and Background; on a phone, a
    /// sheet from the bottom), as [`Shell::pick`] opens a list: each choice its picture (`pictures`, which may
    /// fill in as they are drawn) over its name, five to a row (a phone's three), as many rows shown as there
    /// are, up to [`GRID_ROWS`] and to what the window has room for; more scroll, the chosen one in view.
    pub fn pick_grid(&mut self, id: &str, index: i32, options: Vec<String>, chosen: usize, pictures: ModelRc<slint::Image>) {
        self.open_picker(Key::new(id, index), &Key::new(id, index), options, Look::Grid(pictures), chosen as i32);
    }

    /// Open the list of `key`'s choices by the control `at`, `chosen` marked (-1: none, the cursor on the first).
    fn open_picker(&mut self, key: Key, at: &Key, options: Vec<String>, look: Look, chosen: i32) {
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        let by = self.nav.registry().borrow().rect(at).unwrap_or_default();
        let window = ui.window().size().to_logical(ui.window().scale_factor());
        let grid = matches!(look, Look::Grid(_)).then(|| Grid::of(self.scale.phone, window.width));
        // (A phone's sheet is at the window's foot, three fifths of it at the most.)
        let (under, over) = if self.scale.phone { (window.height * 0.6, 0.0) } else { (window.height - by.bottom() - 8.0, by.y - 8.0) };
        let (rows, under, height) = match grid {
            Some(grid) => {
                let (rows, under) = grid.rows(options.len(), under, over);
                (rows, under, grid.height(rows))
            }
            None => {
                let (rows, under) = picker_rows(options.len(), under, over);
                (rows, under, rows as f32 * PICKER_ROW + 2.0)
            }
        };
        let y = if under { by.bottom() + 4.0 } else { by.y - 4.0 - height };
        let (width, x) = match &look {
            Look::Select => (by.w, by.x),
            Look::Tags(_) => (by.w.max(TAG_LIST), by.x),
            Look::Menu { .. } => (MENU_WIDTH, by.right() - MENU_WIDTH),
            Look::Grid(_) => (grid.map_or(0.0, |g| g.width()), by.x),
        };
        let (columns, cell) = grid.map_or((GRID_COLUMNS, GRID_CELL), |g| (g.columns, g.cell));
        shell.set_picker_grid(grid.is_some());
        shell.set_picker_columns(columns as i32);
        shell.set_picker_cell(cell);
        shell.set_picker_pictures(match &look {
            Look::Grid(pictures) => pictures.clone(),
            _ => ModelRc::default(),
        });
        shell.set_picker_rows(rows as i32);
        shell.set_picker_options(ModelRc::new(VecModel::from(options.into_iter().map(Into::into).collect::<Vec<slint::SharedString>>())));
        shell.set_picker_menu(matches!(look, Look::Menu { .. }));
        shell.set_picker_apart(match &look {
            Look::Menu { apart: Some(apart) } => *apart as i32,
            _ => -1,
        });
        shell.set_picker_tones(ModelRc::new(VecModel::from(match look {
            Look::Tags(tones) => tones,
            _ => Vec::new(),
        })));
        shell.set_picker_chosen(chosen);
        // (Inside the window, a list wider than what it opens from.)
        shell.set_picker_x(x.min(window.width - width - 8.0).max(0.0));
        shell.set_picker_y(y);
        shell.set_picker_width(width);
        shell.set_picker(true);
        // (The battle's menu's list is its owner's, as the menu is: `App::hearer_of`.)
        let owner = match &self.battle {
            Some(b) if b.full || key.id.starts_with("menu.") => b.owner,
            _ => self.screen,
        };
        self.picking = Some(Picking { owner, key });
        let row = chosen.max(0);
        let focus = if grid.is_some() { GRID_CELL_KEY } else { "pick.option" };
        self.nav.open("pick.", Key::new(focus, row));
        // (The chosen row, scrolled into view once the list is there.)
        self.nav.focus(focus, row);
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
        // The ground's patterns, for the window in units.
        let units = ((size.width as f32 / factor).ceil() as i32, (size.height as f32 / factor).ceil() as i32);
        if units != self.scale.ground {
            self.scale.ground = units;
            ui.global::<nettai_ui::Shell>().set_patterns(crate::look::patterns(units.0, units.1));
        }
        turned
    }
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
    /// The UI laid one of the on-screen pad's buttons out: its bit number and its place.
    PadKey(i32, Rect),
}

pub struct App {
    pub shell: Shell,
    /// The screens' modules, by [`ScreenId::ALL`]'s order. One is taken out while it is called.
    screens: Vec<Option<Box<dyn Screen>>>,
    /// Presses that came before the controls they are for existed (a screen just shown): done at the next
    /// frames, in order, each with the frames it has waited.
    pending: Vec<(Action, u8)>,
    /// When the window last drew a frame.
    drawn_at: Option<Instant>,
    /// The motion probe's watch on a press: when it was made, and when the window's first frame after it began and
    /// when the app's work for it ended, before Slint drew it (docs/app.md §13).
    pub(crate) first_frame_after: Option<(Instant, Option<(Instant, Instant)>)>,
    /// `NETTAI_PLAY_STATS`: since when, the frames the window drew and the frames the timer ran since.
    clock_stats: Option<(Instant, u32, u32)>,
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
        // (The screenshot pass extracts nothing: it keeps nothing of its own in the player's data folder.)
        let games = Games::find(settings.legacy_collection.as_deref(), settings.game.as_deref(), !still);
        let looks = crate::look::Looks::new(ui, games.list.iter().map(|g| g.id.as_str()));
        let shell = Shell {
            ui: ui.as_weak(),
            games,
            settings,
            sound,
            nav: Nav::new(),
            keys: Keys::default(),
            pad: Pad::start(),
            bindings,
            touch: Default::default(),
            pad_shown: 0,
            lang,
            gl: None,
            scale: Scale { os: 1.0, unit: 1.0, ..Scale::default() },
            sharp: std::env::var_os("NETTAI_PHYSICAL_PIXELS").is_some(),
            showing: Default::default(),
            own_samples: Vec::new(),
            own_ticks: None,
            game: None,
            looks,
            announced: None,
            runtime_due: 0,
            screen: ScreenId::Menu,
            swap: None,
            reduce_motion: false,
            flights: 0,
            asks: Vec::new(),
            dialog: None,
            picking: None,
            battle: None,
            loading: None,
            practice: Default::default(),
            hold_cards: false,
            hold_loading: None,
            still,
            probe_motion: false,
            reel: false,
            stay_awake: false,
            awake: None,
            drawing: true,
            chord: false,
            shift: false,
            composing: false,
            keep_nothing: still,
        };
        App {
            shell,
            screens: screens::all().into_iter().map(Some).collect(),
            pending: Vec::new(),
            drawn_at: None,
            first_frame_after: None,
            clock_stats: std::env::var_os("NETTAI_PLAY_STATS").map(|_| (Instant::now(), 0, 0)),
        }
    }

    /// The app starts: on the main menu, the game last used loading first.
    pub fn start(&mut self) {
        let ui = self.shell.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        shell.set_version(format!("v{}", env!("CARGO_PKG_VERSION")).into());
        shell.set_player(self.shell.settings.name.as_str().into());
        shell.set_packs_folder(self.shell.games.packs.display().to_string().into());
        shell.set_extracts(self.shell.games.extracts);
        self.shell.games.start();
        if ui.window().is_fullscreen() != self.shell.settings.full_screen {
            ui.window().set_fullscreen(self.shell.settings.full_screen);
        }
        self.shell.apply_motion();
        self.shell.show_bindings();
        self.shell.name_games();
        let games = self.shell.games.playable();
        let first = self.shell.settings.game.clone().filter(|g| games.contains(g)).or_else(|| games.first().cloned());
        match first {
            Some(id) => {
                self.shell.game = Some(id.clone());
                self.shell.games.prefer(&id);
                self.shell.load(&id);
                shell.set_game(Shell::game_name(&id).into());
                // (In its look from the first frame.)
                self.shell.looks.show(&ui, Some(&id), 0);
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

    /// The screen that hears of the control `id`: the battle's owner for the battle's own (its menu's, whose
    /// names begin `menu.`, and everything while it fills the window), else the screen shown. A screen that
    /// shows a battle another owns in a `Picture` of its own (Play, the practice battle beside a waiting
    /// invite) keeps its own controls.
    fn hearer_of(&self, id: &str) -> ScreenId {
        match &self.shell.battle {
            Some(b) if b.full || id.starts_with("menu.") => b.owner,
            _ => self.shell.screen,
        }
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
                    Ask::Tell(to, tag, choice) => {
                        self.call(to, |s, shell| s.answered(shell, &tag, choice));
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
        // (A screen that was still leaving is gone at once.)
        self.shell.finish_swap();
        self.call(from, |s, shell| s.hide(shell));
        self.shell.close_picker();
        // (What was shown this frame is the screen before's.)
        self.shell.showing.reset();
        self.shell.screen = to;
        self.shell.lock_game(false);
        let ui = self.shell.ui();
        // Back's way: its screens leave and arrive the other way.
        ui.global::<Theme>().set_back(to == ScreenId::Menu);
        let main = self.screen(to).map_or("back", |s| s.main());
        self.shell.nav.enter(to.name(), main);
        self.call(to, |s, shell| s.show(shell));
        // On screen, the one before leaves first: its body slides and fades for the theme's `leave`, and
        // then this one is shown. What the player presses meanwhile cuts that short (`App::act`).
        let leave = ui.global::<Theme>().get_leave();
        if leave > 0 {
            ui.global::<nettai_ui::Shell>().set_leaving(true);
            self.shell.nav.hold(true);
            self.shell.swap = Some(Swap { to, at: crate::clock::now() + std::time::Duration::from_millis(leave as u64) });
        } else {
            ui.global::<nettai_ui::Shell>().set_screen(to.view());
        }
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
                    "pick.option" | GRID_CELL_KEY => self.picked(Some(key.index)),
                    "pick.close" => self.picked(None),
                    "back" => self.back(),
                    "pad.menu" => self.shell.open_menu(),
                    CHOOSE_SOURCE => self.choose_source(),
                    "game" | "" => {}
                    id => {
                        let hearer = self.hearer_of(id);
                        self.call(hearer, |s, shell| s.activate(shell, id, key.index));
                    }
                }
            }
            Did::Step(key, by) => {
                self.shell.sound.play(Cue::Cursor);
                // (The value slides the way the step went.)
                self.shell.ui().global::<nettai_ui::Nav>().set_step_dir(by.signum());
                if key.id == "game" {
                    self.shell.step_game(by);
                } else {
                    let hearer = self.hearer_of(&key.id);
                    self.call(hearer, |s, shell| s.step(shell, &key.id, key.index, by));
                }
            }
            Did::Edit { key, text, done } => {
                let hearer = self.hearer_of(&key.id);
                self.call(hearer, |s, shell| s.edit(shell, &key.id, key.index, &text, done));
            }
        }
        self.settle();
    }

    /// A dialog's button: the dialog closes, and the screen that asked hears which.
    fn answer(&mut self, choice: usize) {
        let Some(asked) = self.shell.close_dialog() else { return };
        match asked.owner {
            Some(owner) => {
                self.call(owner, |s, shell| s.answered(shell, asked.tag, choice));
            }
            None if asked.tag == SOURCE && choice == 0 => self.choose_source(),
            None => {}
        }
    }

    /// The player chooses the Legacy Collection's folder, in the system's dialog (the app's frames wait while it
    /// is up). A folder that holds the collection is kept in the settings, and the packs wanted are extracted
    /// from it; one that doesn't is said, and nothing is kept.
    fn choose_source(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        let Some(folder) = rfd::FileDialog::new().pick_folder() else { return };
        #[cfg(target_arch = "wasm32")]
        let Some(folder) = None::<std::path::PathBuf> else { return };
        let first = self.shell.game.clone();
        let extracting = match crate::bnlc::Install::at(&folder) {
            Some(install) => self.shell.games.extract_from(&install, first.as_deref()),
            None => 0,
        };
        if extracting == 0 {
            crate::errors::log("choosing the Legacy Collection's folder", &format_args!("{} holds no archive with a ROM of a game wanted", folder.display()));
            self.shell.ask_for_source();
            return;
        }
        self.shell.settings.legacy_collection = Some(folder);
        self.shell.save_settings();
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

    /// One of the menus' actions, from a key or a pad's button. It acts at once: a screen still leaving is
    /// gone, and if the controls it is for don't exist yet (the screen was shown this frame) it is done as
    /// soon as they do.
    pub fn act(&mut self, action: Action) {
        self.shell.finish_swap();
        if self.shell.nav.waiting() {
            if self.pending.len() < 8 {
                self.pending.push((action, 0));
            }
            return;
        }
        // A field being edited is left first: Back restores its text, anything else confirms it; a move then
        // moves on, and a press was the edit's own.
        if self.shell.nav.editing() {
            let did = self.shell.nav.end_edit(action != Action::Back);
            self.did(did);
            if matches!(action, Action::Back | Action::Confirm | Action::Menu) {
                return;
            }
        }
        let ui = self.shell.ui();
        let over = self.shell.dialog.is_some() || self.shell.picking.is_some();
        // (A page that scrolls with nothing in it to move to is the screen's to scroll: `Screen::scroll`.)
        if !over && matches!(action, Action::Up | Action::Down | Action::PageUp | Action::PageDown | Action::Home | Action::End) {
            let hearer = self.hearer();
            if self.call(hearer, |s, shell| s.scroll(shell, action)).unwrap_or(false) {
                self.settle();
                return;
            }
        }
        // (A move the screen makes itself, where the controls' places don't say where it goes: `Screen::route`.)
        if !over && matches!(action, Action::Up | Action::Down | Action::Left | Action::Right | Action::NextFocus | Action::PreviousFocus) {
            let hearer = self.hearer();
            if self.call(hearer, |s, shell| s.route(shell, action)).unwrap_or(false) {
                self.shell.sound.play(Cue::Cursor);
                self.settle();
                return;
            }
        }
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
                self.shell.nav.pressed(&ui);
                let did = self.shell.nav.confirm(&ui);
                self.did(did);
            }
            // (The pad's START: it shuts the battle's menu it opened. Elsewhere it does nothing: A confirms.)
            Action::Menu if self.shell.battle.as_ref().is_some_and(Battle::menu_open) && !over => self.shell.close_menu(),
            Action::Menu => {}
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
        // (A click on a screen that is leaving is on nothing: its controls are going.)
        if self.shell.swap.is_some() && matches!(input, NavInput::Activate(..) | NavInput::Step(..) | NavInput::Choose(..) | NavInput::Point(..)) {
            return;
        }
        let ui = self.shell.ui();
        match input {
            NavInput::Activate(id, index) => {
                let did = self.shell.nav.clicked(&ui, &id, index);
                self.did(did);
            }
            NavInput::Step(id, index, by) => self.did(Did::Step(Key::new(&id, index), by)),
            NavInput::Choose(id, index, choice) => {
                let hearer = self.hearer_of(&id);
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
                let entered = if let Did::Edit { key, .. } = &did { Some(key.clone()) } else { None };
                self.did(did);
                // (Enter, which is what ends an edit here: the field's purpose follows.)
                if let Some(key) = entered {
                    let hearer = self.hearer_of(&key.id);
                    self.call(hearer, |s, shell| s.submitted(shell, &key.id, key.index));
                    self.settle();
                }
            }
            // (The mouse on the on-screen pad: one pointer, through the UI. Fingers come by `touched`.)
            NavInput::Touch(bit, down) => {
                self.shell.touch.press(down.then_some(bit.clamp(0, 9) as u8));
                self.shell.show_pad();
            }
            NavInput::PadKey(bit, rect) => self.shell.touch.place(bit.clamp(0, 9) as u8, rect),
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

    /// The window's size or the display's scale changed: the layout is fitted to it before the next frame.
    pub fn resized(&mut self) {
        if self.shell.fit_window() {
            self.shell.refresh();
            self.settle();
        }
    }

    /// The pointer moved, clicked or scrolled: the cursor hides. False in the screenshot pass, which takes no
    /// pointer (a mouse left over its window would light what it is on).
    pub fn pointer(&mut self) -> bool {
        if self.shell.still {
            return false;
        }
        self.shell.nav.used_pointer();
        true
    }

    /// The window lost the keyboard: nothing is held.
    pub fn unfocused(&mut self) {
        self.shell.keys.release();
        self.shell.touch.clear();
        self.shell.show_pad();
    }

    /// A finger on the window (`main`'s winit hook, before Slint; `x` and `y` in the display's pixels): the
    /// on-screen pad's, if it came down on one of its buttons while the battle has them. True if it was the
    /// pad's (Slint doesn't get it); false and it goes on to Slint, as a pointer.
    pub fn touched(&mut self, finger: u64, phase: TouchPhase, x: f64, y: f64) -> bool {
        if !(self.shell.scale.phone && self.shell.battle_has_keys()) {
            self.shell.touch.clear();
            self.shell.show_pad();
            return false;
        }
        let unit = (self.shell.scale.os * self.shell.scale.unit).max(0.01);
        let (x, y) = (x as f32 / unit, y as f32 / unit);
        let before = self.shell.touch.held();
        let taken = match phase {
            TouchPhase::Started => self.shell.touch.down(finger, x, y),
            TouchPhase::Moved => self.shell.touch.moved(finger, x, y),
            TouchPhase::Ended | TouchPhase::Cancelled => self.shell.touch.up(finger),
        };
        // (A button newly held is a press, timed as a key's is.)
        if self.shell.touch.held() & !before != 0 {
            self.shell.keys.pressed.push(crate::clock::now());
            self.shell.nav.used_pointer();
        }
        self.shell.show_pad();
        taken
    }

    /// A key of the window, before Slint sees it. True if it was the app's (Slint doesn't get it); false and
    /// it goes on to Slint (a text field being typed in, a shortcut of the system's).
    pub fn key(&mut self, event: &KeyEvent) -> bool {
        let PhysicalKey::Code(code) = event.physical_key else { return false };
        self.key_of(code, event.state == ElementState::Pressed, event.text.as_ref().map(|t| t.to_string()), event.repeat)
    }

    /// [`App::key`], of a key's parts: what it is, whether it went down, the text it types, whether it is a
    /// held key's repeat. (The screenshot pass presses keys through here, as the window does.)
    pub fn key_of(&mut self, code: KeyCode, pressed: bool, text: Option<String>, repeat: bool) -> bool {
        let now = crate::clock::now();
        // The battle's buttons follow the keys whatever has them, so none sticks.
        self.shell.keys.key(&self.shell.bindings, code, pressed, repeat, now);
        let editing = self.shell.nav.editing();
        if !pressed {
            return !editing;
        }
        let press = KeyPress { code, text, repeat, shift: self.shell.shift };
        // The battle has the keys: Esc is nettai's menu, a key bound to no button may be its owner's.
        if self.shell.battle_has_keys() {
            self.shell.nav.used_keys(false);
            if code == KeyCode::Escape {
                self.shell.open_menu();
            } else if self.shell.bindings.of_key(code).is_none() && !repeat {
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
        let Some(action) = menu_action(code, self.shell.shift, &self.shell.bindings) else { return false };
        // (A key held repeats a move; a choice or a way back is pressed once.)
        if repeat && !matches!(action, Action::Up | Action::Down | Action::Left | Action::Right | Action::NextFocus | Action::PreviousFocus | Action::PageUp | Action::PageDown) {
            return true;
        }
        self.shell.nav.used_keys(false);
        self.act(action);
        true
    }

    // ---- The frame ---------------------------------------------------------------------------------------

    /// Before a frame is drawn at `now`: the window's scale, the pad, the games loading, the screens, the
    /// battle's ticks and its picture, the sound, the focus.
    /// A frame the window is about to draw: the app's clock (docs/app.md §1).
    pub fn frame(&mut self, now: Instant) {
        self.drawn_at = Some(now);
        let first = self.first_frame_after.as_ref().is_some_and(|(at, first)| first.is_none() && now > *at);
        self.shell.drawing = true;
        self.count(now, true);
        self.run(now);
        if first && let Some((_, seen)) = &mut self.first_frame_after {
            *seen = Some((now, Instant::now()));
        }
    }

    /// How long the window may go without drawing a frame before the timer runs the app's: three of a 60 Hz
    /// display's frames.
    const UNDRAWN: Duration = Duration::from_millis(50);

    /// The timer's turn (every 16 ms: `main.rs`): if the window hasn't drawn a frame for a while (it is
    /// minimized, hidden, covered or on another desktop: the system draws no frames for it) and something must
    /// go on all the same (a battle, the cards before one, a screen's link), the frame is run here, without
    /// its picture. So an online match keeps ticking and its link keeps being polled, and the other player
    /// doesn't see a stalled peer. True if it ran one. While the window draws, this does nothing: the frames
    /// are the window's, at its rate.
    pub fn idle(&mut self, now: Instant) -> bool {
        let wanted = self.shell.battle.is_some() || self.shell.loading_battle() || self.shell.stay_awake;
        if !wanted || self.drawn_at.is_some_and(|at| now.saturating_duration_since(at) < App::UNDRAWN) {
            return false;
        }
        self.shell.drawing = false;
        self.count(now, false);
        self.run(now);
        self.shell.drawing = true;
        true
    }

    /// `NETTAI_PLAY_STATS`: every two seconds, how many frames the window drew and how many the timer ran.
    fn count(&mut self, now: Instant, drawn: bool) {
        let Some((since, by_window, by_timer)) = &mut self.clock_stats else { return };
        if drawn {
            *by_window += 1;
        } else {
            *by_timer += 1;
        }
        if now.saturating_duration_since(*since) >= Duration::from_secs(2) {
            eprintln!("clock stats: {by_window} frames drawn by the window, {by_timer} run by the timer");
            (*since, *by_window, *by_timer) = (now, 0, 0);
        }
    }

    fn run(&mut self, now: Instant) {
        // (The last frame was drawn, if the renderer didn't say so.)
        self.drawn();
        self.shell.showing.reset();
        if self.shell.fit_window() {
            self.shell.refresh();
        }
        self.read_pad(now);
        self.follow_games();
        // (A change of game's colors a step further.)
        self.shell.looks.frame(&self.shell.ui(), now);
        let shown = self.shell.screen;
        for id in ScreenId::ALL {
            self.call(id, |s, shell| s.frame(shell, now, id == shown && shell.battle.as_ref().is_none_or(|b| !b.full)));
        }
        self.settle();
        // What must keep its time whatever covers the window: a battle, a screen's link, the screenshot pass.
        let awake = self.shell.battle.is_some() || self.shell.loading_battle() || self.shell.stay_awake || self.shell.still;
        if awake != self.shell.awake.is_some() {
            self.shell.awake = awake.then(crate::awake::Awake::new);
        }
        // The battle: loading behind the cards, or running.
        if let Some((owner, event)) = self.shell.load_battle(now) {
            self.call(owner, |s, shell| s.battle(shell, event));
            self.settle();
        }
        let mut samples = Vec::new();
        let (mut ticks, event) = self.shell.run_battle(now, &mut samples);
        // (What a screen played of its own this frame: a replay's viewer.)
        if let Some(own) = self.shell.own_ticks.take() {
            samples.append(&mut self.shell.own_samples);
            ticks = Some(ticks.unwrap_or(0) + own);
        }
        if let Some((owner, event)) = event {
            self.call(owner, |s, shell| s.battle(shell, event));
            self.settle();
        }
        for id in ScreenId::ALL {
            self.call(id, |s, shell| s.frame_end(shell, now));
        }
        self.settle();
        // The title music plays on the menu screens (not under a battle with music of its own, nor on the cards
        // before one).
        let silent = |m: crate::battle::BattleMusic| m == crate::battle::BattleMusic::None;
        let in_battle = self.shell.battle.as_ref().is_some_and(|b| !silent(b.music))
            || self.shell.loading.as_ref().is_some_and(|l| !silent(l.start.music))
            || (ticks.is_some() && !self.shell.battle.as_ref().is_some_and(|b| silent(b.music)));
        self.shell.sound.want_music(!in_battle && self.shell.ready().is_some());
        self.shell.sound.frame(now, ticks, &mut samples);
        // The screen that was leaving has left.
        if self.shell.swap.as_ref().is_some_and(|s| now >= s.at) {
            self.shell.finish_swap();
        }
        let ui = self.shell.ui();
        // The window's picture, only if something was shown in it this frame (`stage::Showing`): a screen with
        // nothing of its own shows its `Picture`'s ground, not what another presented last. (While a screen
        // leaves, it keeps what it had.)
        if self.shell.drawing
            && !self.shell.leaving()
            && let Some(shown) = self.shell.showing.tell()
        {
            ui.global::<nettai_ui::Shell>().set_picture_shown(shown);
        }
        // The one focus, and where it is.
        let moved = self.shell.nav.settle(&ui, now, ui.global::<Theme>().get_motion());
        // What was pressed before its controls existed, now that they do (or after a few frames without).
        if !self.pending.is_empty() && self.shell.swap.is_none() {
            let waiting = self.shell.nav.waiting();
            let pending = std::mem::take(&mut self.pending);
            for (action, waited) in pending {
                if !waiting {
                    self.act(action);
                } else if waited < 6 {
                    self.pending.push((action, waited + 1));
                }
            }
        }
        if let Some(key) = moved
            && !matches!(key.id.as_str(), "back" | "game")
            && !key.id.starts_with("dialog.")
            && !key.id.starts_with("pick.")
        {
            let hearer = self.hearer_of(&key.id);
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
            self.act(action);
        }
    }

    /// The games as they load: the bar while the selected one does, the screens shown again once it is ready,
    /// the others loaded behind it; one that can't load is passed over.
    fn follow_games(&mut self) {
        let ui = self.shell.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        // What the player is to be told of a game, once, by the game's name (the others go on loading, and
        // play): a pack that couldn't be loaded or extracted; no Legacy Collection to extract one from (where a
        // game can be played: else the loading screen says it, with the way to its folder). Not over another
        // dialog, nor over a battle.
        if self.shell.dialog.is_none()
            && self.shell.battle.is_none()
            && !self.shell.loading_battle()
            && let Some(notice) = self.shell.games.take_notice()
        {
            match notice {
                // (Another build's pack, which couldn't be made again for want of the collection.)
                Notice::LoadFailed(game) if self.shell.games.is_stale(&game) && !self.shell.games.found_source => self.shell.ask_for_source(),
                Notice::LoadFailed(game) => self.shell.fail(UserError::GameOutdated { game: Shell::game_name(&game) }),
                Notice::ExtractFailed(game) => self.shell.fail(UserError::ExtractFailed { game: Shell::game_name(&game) }),
                Notice::NoSource if !self.shell.games.playable().is_empty() => self.shell.ask_for_source(),
                Notice::NoSource => {}
            }
        }
        if let Some(held) = self.shell.hold_loading {
            shell.set_loading(true);
            shell.set_loading_progress(held);
            return;
        }
        let Some(id) = self.shell.game.clone() else {
            // (A game that can be played since: a pack being extracted from a folder just chosen.)
            match self.shell.games.playable().first() {
                Some(first) => self.shell.select_game(&first.clone()),
                None => {
                    shell.set_no_games(true);
                    shell.set_loading(true);
                }
            }
            return;
        };
        shell.set_extracting(matches!(self.shell.games.get(&id).map(|e| &e.state), Some(State::Extracting(_))));
        match self.shell.games.get(&id).map(|e| &e.state) {
            Some(State::Extracting(_)) => {
                shell.set_loading(true);
                shell.set_loading_progress(self.shell.games.progress(&id));
            }
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
            state @ (Some(State::Failed) | Some(State::NoPack) | None) => {
                // It can't be played: the next that can. One whose pack couldn't be loaded or extracted is told
                // above, by its name; one asked for that has no pack at all, as something gone wrong.
                let told = matches!(state, Some(State::Failed)) || (matches!(state, Some(State::NoPack)) && self.shell.games.extracts);
                let games = self.shell.games.playable();
                self.shell.game = None;
                match games.first() {
                    Some(next) => self.shell.select_game(&next.clone()),
                    None => shell.set_no_games(true),
                }
                if !told {
                    self.shell.fail(UserError::Other);
                }
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
                self.shell.keys.pressed.push(crate::clock::now());
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A select's list is as tall as its rows, up to ten and to the room the window has by the select: under
    /// it, or over it where it doesn't fit under and there is more room over. It is never taller than its room.
    #[test]
    fn a_selects_list_fits_the_window() {
        assert_eq!(picker_rows(3, 600.0, 100.0), (3, true));
        assert_eq!(picker_rows(24, 600.0, 100.0), (10, true), "more than ten scroll");
        assert_eq!(picker_rows(24, 300.0, 100.0), (6, true), "as many whole rows as there is room for");
        assert_eq!(picker_rows(24, 100.0, 500.0), (10, false), "over the select, where the room is");
        assert_eq!(picker_rows(24, 100.0, 200.0), (4, false));
        assert_eq!(picker_rows(3, 100.0, 500.0), (3, false));
        assert_eq!(picker_rows(2, 100.0, 500.0), (2, true), "what fits under goes under");
        assert_eq!(picker_rows(0, 10.0, 5.0), (1, true));
        for options in 0..40 {
            for (under, over) in [(700.0, 60.0), (60.0, 700.0), (380.0, 380.0), (150.0, 240.0), (800.0 * 0.6, 0.0)] {
                let (rows, is_under) = picker_rows(options, under, over);
                let room: f32 = if is_under { under } else { over };
                assert!((1..=PICKER_ROWS).contains(&rows) && rows <= options.max(1));
                assert!(rows as f32 * PICKER_ROW + 2.0 <= room, "{options} choices in {under} under, {over} over: {rows} rows");
            }
        }
    }
}
