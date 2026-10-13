//! The NaviCust tab of a build's editor (docs/design/app-ui.md §6, "NaviCust"). Not built yet: the tab is in
//! the bar, with nothing in it. What it will call is `crate::builds::grid` (the boards, the pieces' shapes, a
//! piece placed, turned and compressed). `super::editor` says what a tab is given.

use crate::builds::grid;
use crate::screens::builds::editor::{Draft, Tab};
use crate::shell::Shell;
use nettai_battle::Content;
use nettai_ui::EditorTab;

#[derive(Default)]
pub struct Navicust;

impl Tab for Navicust {
    fn id(&self) -> EditorTab {
        EditorTab::Navicust
    }

    /// The pieces placed, and the board they are on.
    fn fields(&self, _content: &Content) -> Vec<String> {
        vec![grid::PIECES_FIELD.into(), grid::SIZE_FIELD.into()]
    }

    fn own_navi_only(&self) -> bool {
        true
    }

    fn main(&self) -> &'static str {
        "tab"
    }

    fn show(&mut self, _shell: &mut Shell, _draft: &mut Draft) {}
}
