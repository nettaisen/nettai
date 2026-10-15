//! Builds (docs/design/app-ui.md §6): a build's editor. The builds themselves (+ NEW BUILD and its FROM A SAVE
//! FILE…, the chosen one's overview, its menu of DUPLICATE, RENAME and DELETE) are Play's
//! (`crate::screens::play`); Play's EDIT and Practice's open a build here.
//!
//! **The editor** (`editor`): the header's name, version and "✓ Ready to play", the tab bar, and the tab
//! shown. Each tab is a module here and a file in the UI's `screens/builds/`: [`navi`], [`folder`],
//! [`navicust`], [`patch_cards`] and [`auto_battle`].
//!
//! **Opened by another screen** ([`open`]: Play's EDIT and Practice's): Back goes straight back to that screen,
//! the build kept, and the header says where (‹ PLAY or ‹ PRACTICE).

use crate::builds::store;
use crate::builds::{self};
use crate::input::{Action, KeyPress};
use crate::screens::{Screen, ScreenId, opened_by};
use crate::shell::Shell;
use crate::sound::Cue;
use editor::Editor;
use nettai_ui::BuildsView;
use slint::ComponentHandle;

pub mod auto_battle;
pub mod editor;
pub mod folder;
pub mod navi;
pub mod navicust;
pub mod patch_cards;

#[derive(Default)]
pub struct Builds {
    editor: Option<Editor>,
    /// The screen that opened the editor ([`open`]), which Back goes back to.
    back_to: Option<ScreenId>,
}

/// Show the build at `at` of the selected game's builds (`store::list`'s order, which every list of them has) in
/// the editor for screen `from`, Back going straight back to it. The screen `from` is shown again as ever
/// (`Screen::show`): it reads the builds as they were left.
pub fn open(shell: &mut Shell, from: ScreenId, at: usize) {
    shell.go(ScreenId::Builds);
    shell.tell(ScreenId::Builds, &crate::screens::opened_tag(from), at);
}

impl Builds {
    /// The screen `from` opened this one ([`open`]) on the build at `at`: it is read and opened in the editor.
    /// One that isn't there or doesn't read as one (its file changed meanwhile) is refused, and the screen goes
    /// straight back.
    fn opened(&mut self, shell: &mut Shell, from: ScreenId, at: usize) {
        self.back_to = Some(from);
        let opened = builds::kit(shell).and_then(|kit| {
            let listed = store::list(&kit.game).into_iter().nth(at)?;
            let build = store::read(kit.content(), &kit.game, &listed.text).map_err(|why| crate::errors::log(&format!("reading the build {}", listed.path.display()), &why)).ok()?;
            Some(Editor::open(shell, kit, listed.path, build))
        });
        match opened {
            Some(editor) => {
                self.editor = Some(editor);
                self.show(shell);
            }
            None => {
                shell.sound.play(Cue::Refused);
                shell.go(from);
            }
        }
    }
}

impl Screen for Builds {
    fn main(&self) -> &'static str {
        "tab"
    }

    fn show(&mut self, shell: &mut Shell) {
        shell.ui().global::<BuildsView>().set_to_practice(self.back_to == Some(ScreenId::Practice));
        // (The editor's build is its game's: the game can't change under it.)
        if let Some(editor) = &mut self.editor {
            editor.show(shell);
        }
    }

    fn hide(&mut self, shell: &mut Shell) {
        if let Some(mut editor) = self.editor.take() {
            editor.close(shell);
        }
        self.back_to = None;
        shell.ui().global::<BuildsView>().set_to_practice(false);
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if let Some(editor) = &mut self.editor {
            editor.activate(shell, id, index);
        }
    }

    fn step(&mut self, shell: &mut Shell, id: &str, index: i32, by: i32) {
        if let Some(editor) = &mut self.editor {
            editor.step(shell, id, index, by);
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, index: i32, choice: i32) {
        if let Some(editor) = &mut self.editor {
            editor.choose(shell, id, index, choice);
        }
    }

    fn edit(&mut self, shell: &mut Shell, id: &str, index: i32, text: &str, done: bool) {
        if let Some(editor) = &mut self.editor {
            editor.edit(shell, id, index, text, done);
        }
    }

    fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if let Some(editor) = &mut self.editor {
            editor.focused(shell, id, index);
        }
    }

    fn hovered(&mut self, shell: &mut Shell, id: &str, index: i32, on: bool) {
        if let Some(editor) = &mut self.editor {
            editor.hovered(shell, id, index, on);
        }
    }

    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        self.editor.as_mut().is_some_and(|editor| editor.key(shell, key))
    }

    fn button(&mut self, shell: &mut Shell, button: gamepad_facade::Button) -> bool {
        self.editor.as_mut().is_some_and(|editor| editor.button(shell, button))
    }

    /// The shoulders change the tab.
    fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        self.editor.as_mut().is_some_and(|editor| editor.action(shell, action))
    }

    /// Back within the tab (out of its library, a piece put away), else straight back to the screen that opened
    /// the editor: leaving this one closes it, the build kept.
    fn back(&mut self, shell: &mut Shell) -> bool {
        if self.editor.as_mut().is_some_and(|editor| editor.back(shell)) {
            return true;
        }
        shell.go(self.back_to.unwrap_or(ScreenId::Play));
        true
    }

    fn answered(&mut self, shell: &mut Shell, tag: &str, choice: usize) {
        if let Some(from) = opened_by(tag) {
            return self.opened(shell, from, choice);
        }
        if let Some(editor) = &mut self.editor {
            editor.answered(shell, tag, choice);
        }
    }
}
