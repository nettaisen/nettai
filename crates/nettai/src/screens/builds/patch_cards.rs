//! The patch cards tab of a build's editor (docs/design/app-ui.md §6, "Patch cards"). Not built yet: the tab
//! is in the bar, with nothing in it. What it will call is `crate::builds::cards` (a card's lines),
//! `crate::builds::edit` (a list's edits) and nettai-match's `build` (`Overview::budget`, `Budget::refusal`,
//! `Builds::slots`). `super::editor` says what a tab is given.

use crate::screens::builds::editor::{Draft, Tab};
use crate::shell::Shell;
use nettai_battle::Content;
use nettai_ui::EditorTab;

/// The setup field of the cards installed.
pub const FIELD: &str = "patch_cards";

#[derive(Default)]
pub struct PatchCards;

impl Tab for PatchCards {
    fn id(&self) -> EditorTab {
        EditorTab::PatchCards
    }

    fn fields(&self, _content: &Content) -> Vec<String> {
        vec![FIELD.into()]
    }

    fn own_navi_only(&self) -> bool {
        true
    }

    fn main(&self) -> &'static str {
        "tab"
    }

    fn show(&mut self, _shell: &mut Shell, _draft: &mut Draft) {}
}
