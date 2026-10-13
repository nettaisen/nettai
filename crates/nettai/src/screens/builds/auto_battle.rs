//! The auto battle tab of a build's editor (docs/design/app-ui.md §6, "Auto battle"), in a game whose setup
//! takes auto battle data. Not built yet: the tab is in the bar, with nothing in it. What it will call is
//! `crate::builds::auto` (the data's lists and places) and `crate::builds::import::auto_battle_of_save`.
//! `super::editor` says what a tab is given.

use crate::builds::auto;
use crate::screens::builds::editor::{Draft, Tab};
use crate::shell::Shell;
use nettai_battle::Content;
use nettai_ui::EditorTab;

#[derive(Default)]
pub struct AutoBattle;

impl Tab for AutoBattle {
    fn id(&self) -> EditorTab {
        EditorTab::AutoBattle
    }

    /// The data's places (its records are none of a build's: the rules' default, nothing learned).
    fn fields(&self, _content: &Content) -> Vec<String> {
        vec![auto::PLACES_FIELD.into()]
    }

    fn own_navi_only(&self) -> bool {
        true
    }

    fn main(&self) -> &'static str {
        "tab"
    }

    fn show(&mut self, _shell: &mut Shell, _draft: &mut Draft) {}
}
