//! The screens (docs/app.md §2): each is one Slint file (`crates/nettai-ui/ui/screens/<name>.slint`: what is
//! drawn, and the screen's view, the global its data is set in) and one module here (what it does).
//!
//! A screen never handles keys or wires callbacks. Its controls have names; the shell finds which one a key,
//! a pad or a click means (`crate::nav`) and calls the screen's [`Screen`] with the name: `activate("ready",
//! 0)`, `step("rounds", 0, 1)`. The screen changes its state, sets its view's properties, and asks the
//! [`Shell`] for what is not its own: another screen, the selected game, a dialog, a battle.

use crate::battle::BattleEvent;
use crate::input::{Action, KeyPress};
use crate::shell::Shell;
use std::time::Instant;

pub mod builds;
pub mod menu;
pub mod play;
pub mod replays;
pub mod settings;
pub mod training;

/// The screens a player goes between.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenId {
    Menu,
    Play,
    Training,
    Builds,
    Replays,
    Settings,
}

impl ScreenId {
    pub const ALL: [ScreenId; 6] = [ScreenId::Menu, ScreenId::Play, ScreenId::Training, ScreenId::Builds, ScreenId::Replays, ScreenId::Settings];

    /// The UI's name for it.
    pub fn view(self) -> nettai_ui::Screen {
        match self {
            ScreenId::Menu => nettai_ui::Screen::Menu,
            ScreenId::Play => nettai_ui::Screen::Play,
            ScreenId::Training => nettai_ui::Screen::Training,
            ScreenId::Builds => nettai_ui::Screen::Builds,
            ScreenId::Replays => nettai_ui::Screen::Replays,
            ScreenId::Settings => nettai_ui::Screen::Settings,
        }
    }

    /// Its name in the screenshot pass's files and in the focus's memory.
    pub fn name(self) -> &'static str {
        match self {
            ScreenId::Menu => "menu",
            ScreenId::Play => "play",
            ScreenId::Training => "training",
            ScreenId::Builds => "builds",
            ScreenId::Replays => "replays",
            ScreenId::Settings => "settings",
        }
    }
}

/// Every screen's module, in [`ScreenId::ALL`]'s order.
pub fn all() -> Vec<Box<dyn Screen>> {
    vec![
        Box::new(menu::Menu::default()),
        Box::new(play::Play::default()),
        Box::new(training::Training::default()),
        Box::new(builds::Builds::default()),
        Box::new(replays::Replays::default()),
        Box::new(settings::Settings::default()),
    ]
}

/// What a screen does. Every method but [`Screen::show`] has a default that does nothing, so a screen writes
/// only what it has.
///
/// `id` and `index` are a control's name and index as its Slint gives them (`nav: "build"; nav-index: i`); a
/// `ListView`'s row is the list's name and the row's index.
pub trait Screen {
    /// The control the focus starts on, the first time the screen is shown (after that it is where it was).
    fn main(&self) -> &'static str {
        "back"
    }

    /// The screen is shown, or something it shows changed under it (the selected game, the language, the
    /// window turned upright): set the view from the state.
    fn show(&mut self, shell: &mut Shell);

    /// Another screen is shown in its place.
    fn hide(&mut self, _shell: &mut Shell) {}

    /// Enter, a click or a tap on a control.
    fn activate(&mut self, _shell: &mut Shell, _id: &str, _index: i32) {}

    /// Left or Right on a stepper, a select or a two-way switch (`by` -1 or 1), or a click on its arrow.
    fn step(&mut self, _shell: &mut Shell, _id: &str, _index: i32, _by: i32) {}

    /// A choice made outright: a click on one side of a two-way switch (0 or 1), a slider set to a level, a
    /// row of a select's list.
    fn choose(&mut self, _shell: &mut Shell, _id: &str, _index: i32, _choice: i32) {}

    /// A field's text: each change as it is typed (`done` false), then the text it is left with (confirmed,
    /// or the old one restored by Esc). The screen keeps what it wants of it and sets its view's property,
    /// which the field then shows (a name cut to its length, a code in capitals).
    fn edit(&mut self, _shell: &mut Shell, _id: &str, _index: i32, _text: &str, _done: bool) {}

    /// The focus moved to a control (a list's row: the overview beside a list follows it).
    fn focused(&mut self, _shell: &mut Shell, _id: &str, _index: i32) {}

    /// A key pressed that the screen may want for itself, before the menus' keys are read from it (a letter
    /// that acts on the focused row, a key being bound). True if it was the screen's.
    fn key(&mut self, _shell: &mut Shell, _key: &KeyPress) -> bool {
        false
    }

    /// A pad's button pressed, as [`Screen::key`].
    fn button(&mut self, _shell: &mut Shell, _button: gamepad_facade::Button) -> bool {
        false
    }

    /// An action the shell has no use of its own for here: previous and next (the shoulders, the keys bound to
    /// L and R: the shell changes the game with them unless the screen takes them), delete, the pad's north
    /// button. True if the screen took it.
    fn action(&mut self, _shell: &mut Shell, _action: Action) -> bool {
        false
    }

    /// Esc or Back. True if the screen went back within itself (from a lobby to its first view); false, and
    /// the shell goes back to the menu.
    fn back(&mut self, _shell: &mut Shell) -> bool {
        false
    }

    /// A dialog this screen asked (`Shell::ask`, by its `tag`) was answered with its button `choice`.
    fn answered(&mut self, _shell: &mut Shell, _tag: &str, _choice: usize) {}

    /// A file was dropped on the window while this screen showed.
    fn dropped(&mut self, _shell: &mut Shell, _path: &std::path::Path) {}

    /// A battle this screen started: it began, it ended; or the replay it asked for is ready to watch.
    fn battle(&mut self, _shell: &mut Shell, _event: BattleEvent) {}

    /// Every frame, shown or not (a lobby's link is polled while another screen shows).
    fn frame(&mut self, _shell: &mut Shell, _now: Instant, _shown: bool) {}
}
