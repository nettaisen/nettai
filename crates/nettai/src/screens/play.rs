//! Play (docs/design/app-ui.md §6): online play. Not built yet: the screen has its header and Back, and the
//! match menu what leaves a match. The link and the lobby it will drive are `crate::lobby` and
//! `crate::netplay`.

use crate::battle::BattleEvent;
use crate::screens::Screen;
use crate::shell::Shell;

#[derive(Default)]
pub struct Play;

impl Screen for Play {
    fn show(&mut self, _shell: &mut Shell) {}

    fn activate(&mut self, shell: &mut Shell, id: &str, _index: i32) {
        match id {
            "menu.resume" => shell.close_menu(),
            "menu.leave" => shell.end_battle(),
            _ => {}
        }
    }

    fn battle(&mut self, shell: &mut Shell, event: BattleEvent) {
        if matches!(event, BattleEvent::Finished(_) | BattleEvent::Stopped) {
            shell.end_battle();
        }
    }
}
