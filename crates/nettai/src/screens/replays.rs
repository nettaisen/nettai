//! Replays (docs/design/app-ui.md §6): the sets recorded, and the viewer. Not built yet: the screen has its
//! header and Back. The files and their reading are `crate::replays`.

use crate::screens::Screen;
use crate::shell::Shell;

#[derive(Default)]
pub struct Replays;

impl Screen for Replays {
    fn show(&mut self, _shell: &mut Shell) {}
}
