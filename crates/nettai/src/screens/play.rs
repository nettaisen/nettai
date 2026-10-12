//! Play (docs/design/app-ui.md §6): online play. Not built yet: the screen has its header and Back, and the
//! match menu what leaves a match. The link and the lobby it will drive are `crate::lobby` and
//! `crate::netplay`.

use crate::battle::BattleEvent;
use crate::errors::UserError;
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
        match event {
            BattleEvent::Started | BattleEvent::Watching(_) => {}
            BattleEvent::Finished(_) => shell.end_battle(),
            BattleEvent::Stopped(reason) => {
                shell.end_battle();
                if let Some(error) = UserError::of_stop(reason) {
                    shell.fail(error);
                }
            }
        }
    }
}
