//! Training (docs/design/app-ui.md §6): you against a dummy set up from a menu while in training. Not built
//! yet: the screen has its header and Back, and the training menu what leaves a battle.
//!
//! What is here is what a battle's owner needs at the least, and a way into a battle for development and the
//! screenshot pass ([`start_development_battle`], `NETTAI_TRAINING=<game>`): a picked match against a dummy that
//! stands still. The screen's own way in (two builds, START TRAINING) replaces it.

use crate::battle::{BattleEvent, BattleStart};
use crate::builds::presets;
use crate::games::Names;
use crate::input::{Action, KeyPress};
use crate::screens::{Screen, ScreenId};
use crate::shell::Shell;
use nettai_frontend::driver::{LivePlayer, Opponent};
use nettai_ui::{BattleKind, TrainingView, VersusCard};
use slint::ComponentHandle;
use slint::winit_030::winit::keyboard::KeyCode;
use std::time::Instant;

#[derive(Default)]
pub struct Training {
    /// `NETTAI_TRAINING` was answered.
    started: bool,
}

impl Screen for Training {
    fn show(&mut self, _shell: &mut Shell) {}

    fn activate(&mut self, shell: &mut Shell, id: &str, _index: i32) {
        match id {
            "menu.resume" => shell.close_menu(),
            "menu.quit" => shell.end_battle(),
            _ => {}
        }
    }

    /// R or F5 in a training battle: the set again from its start.
    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        if !matches!(key.code, KeyCode::KeyR | KeyCode::F5) {
            return false;
        }
        restart(shell)
    }

    /// The pad's north button, as R.
    fn action(&mut self, shell: &mut Shell, action: Action) -> bool {
        action == Action::Alternate && restart(shell)
    }

    fn button(&mut self, shell: &mut Shell, button: gamepad_facade::Button) -> bool {
        button == gamepad_facade::Button::North && restart(shell)
    }

    fn battle(&mut self, shell: &mut Shell, event: BattleEvent) {
        if matches!(event, BattleEvent::Finished(_) | BattleEvent::Stopped) {
            shell.end_battle();
        }
    }

    fn frame(&mut self, shell: &mut Shell, _now: Instant, _shown: bool) {
        // `NETTAI_TRAINING=<game>`: straight into a training battle of it, once it is ready.
        if !self.started
            && let Ok(game) = std::env::var("NETTAI_TRAINING")
            && shell.game() == Some(game.as_str())
            && shell.ready().is_some()
        {
            self.started = true;
            shell.go(ScreenId::Training);
            start_development_battle(shell, crate::screens::menu::clock_seed());
        }
    }
}

/// The training battle's set again from its start, if one is on.
fn restart(shell: &mut Shell) -> bool {
    let Some(b) = shell.battle.as_mut().filter(|b| b.owner == ScreenId::Training && b.playing()) else { return false };
    b.stage.player.restart();
    b.stage.stale = true;
    true
}

/// Start a training battle of the selected game: a match picked from `seed`, against a dummy that stands still
/// and picks its first chips.
pub fn start_development_battle(shell: &mut Shell, seed: u32) {
    let Some((game, ready)) = shell.ready() else { return };
    let content = ready.content();
    let m = match nettai_match::pick::live(content, &game, seed, None) {
        Ok(m) => m,
        Err(why) => return crate::errors::log("picking a training match", &why),
    };
    let graphics = ready.graphics(shell.lang);
    let names = Names::of(content, &graphics);
    let ui = shell.ui();
    let view = ui.global::<TrainingView>();
    let card = |role: slint::SharedString, name: &str, side: &nettai_match::Side| VersusCard {
        role,
        name: name.into(),
        build: Default::default(),
        version: presets::version(content, &game, side).into(),
        navi: names.navi(side.navi(content)).into(),
        progress: 0.0,
    };
    let cards = [card(view.get_you(), &shell.settings.name, &m.sides[0]), card(view.get_dummy(), "", &m.sides[1])];
    let navi = |side: usize| names.navi(m.sides[side].navi(content));
    view.set_matchup(format!("{} vs {}", navi(0), navi(1)).into());
    let driver = LivePlayer::against(nettai_match::Set::of(content, &m, seed), Opponent::default(), seed as u64);
    shell.start_battle(BattleStart {
        owner: ScreenId::Training,
        kind: BattleKind::Training,
        game: game.clone(),
        set: nettai_match::Set::of(content, &m, seed),
        driver: Box::new(driver),
        viewer: 0,
        recorder: None,
        cards: Some(cards),
        full: true,
        gate: None,
    });
}
