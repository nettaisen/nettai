//! Builds (docs/design/app-ui.md §6): the player's builds of the selected game, and the editor. Not built yet:
//! the screen has its header and Back. What it will call is `crate::builds` (the files, the edits, the rules'
//! layout, the presets, a save's import, the pictures).

use crate::screens::Screen;
use crate::shell::Shell;

#[derive(Default)]
pub struct Builds;

impl Screen for Builds {
    fn show(&mut self, _shell: &mut Shell) {}
}
