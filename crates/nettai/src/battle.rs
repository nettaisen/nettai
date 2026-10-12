//! A battle on screen (docs/app.md §1): the two cards while it loads, then the battle itself, with nettai's
//! menu over it when Start or Esc asks.
//!
//! A screen starts one with [`Shell::start_battle`] and stays its owner: the menu's controls are the owner's
//! (the training menu is Training's, the match menu Play's: their names begin `menu.`), and the owner hears
//! when the battle begins and ends ([`BattleEvent`]). The shell does the rest: it loads everything before the
//! first frame, runs the ticks due each frame with the buttons held, presents the picture, and takes the keys
//! for the battle while nothing is over it.
//!
//! The game's own pause never runs: outside the custom screen, START never reaches the game, and opens
//! nettai's menu instead.

use crate::errors::UserError;
use crate::games::Ready;
use crate::nav::Key;
use crate::screens::ScreenId;
use crate::shell::Shell;
use crate::stage::{self, Stage};
use nettai_battle::BattleResult;
use nettai_battle::input::keys;
use nettai_frontend::driver::Driver;
use nettai_frontend::game::TextMode;
use nettai_frontend::player::Player;
use nettai_frontend::replay::Recorder;
use nettai_render::vfont::TextRenderer;
use nettai_ui::{BattleKind, VersusCard};
use slint::ComponentHandle;
use std::time::{Duration, Instant};

/// How long the two cards show at the least, so they can be read.
const CARDS_SHOWN: Duration = Duration::from_millis(900);

/// What a screen makes for its battle while the cards show: what drives it (a live set against a dummy, a match
/// over a link, a replay), and the recorder it is recorded with, if it is.
pub struct Made {
    pub driver: Box<dyn Driver>,
    pub recorder: Option<Recorder>,
}

/// A battle to start.
pub struct BattleStart {
    /// The screen that starts it: its menu's controls and its events are that screen's.
    pub owner: ScreenId,
    /// Which menu Start opens.
    pub kind: BattleKind,
    /// The game it is of (loaded).
    pub game: String,
    /// The two cards shown while it loads, left and right; none: it starts with no cards (the training battle
    /// beside a waiting invite).
    pub cards: Option<[VersusCard; 2]>,
    /// The picture fills the window. Else the owner's screen shows it in a `Picture` of its own.
    pub full: bool,
    /// Makes the battle's driver. It runs while the cards show, so what it takes (reading the builds, setting
    /// the battle up) is not a stutter in the battle.
    pub make: Box<dyn FnOnce(&Shell, &Ready) -> Result<Made, UserError>>,
    /// Online: called each frame with how far this side has loaded (0 to 1), it tells the other side and says
    /// how far that one is. The battle starts when both are 1. None: nobody to wait for.
    pub gate: Option<Box<dyn FnMut(f32) -> f32>>,
}

/// What a battle's owner hears of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleEvent {
    /// Everything is loaded and its first frame is about to run.
    Started,
    /// The set is over: how it went for this player (none: nobody's).
    Finished(Option<BattleResult>),
    /// It stopped without ending (the other player left, the engine stopped: why is in the log).
    Stopped,
}

/// The battle on screen.
pub struct Battle {
    pub stage: Stage,
    pub owner: ScreenId,
    pub kind: BattleKind,
    pub game: String,
    pub full: bool,
    /// nettai's menu is over it.
    menu: bool,
    /// The owner has heard how it ended.
    over: bool,
    /// The buttons held at the last frame (START's press opens the menu once).
    held: u16,
}

impl Battle {
    /// Whether the battle has the keys: it runs, and nothing is over it.
    pub fn playing(&self) -> bool {
        !self.menu && !self.over
    }

    pub fn menu_open(&self) -> bool {
        self.menu
    }
}

/// A battle being loaded: the steps done, and what they made.
pub struct Loading {
    start: BattleStart,
    began: Instant,
    step: u8,
    made: Option<Made>,
    stage: Option<Stage>,
    theirs: f32,
}

impl Shell {
    /// A player of `driver` on `ready`'s game, shown in the language, its text as the settings say.
    pub fn player(&self, ready: &Ready, driver: Box<dyn Driver>, sound: bool) -> Player {
        let text = if self.settings.sharp_text { TextMode::Font } else { TextMode::Original };
        let graphics = ready.graphics(self.lang);
        let font = ready.loaded.font.clone().filter(|_| text == TextMode::Font);
        let renderer = graphics.renderer(text, font.clone());
        let audio = ready.loaded.sound.as_ref().filter(|_| sound).map(|s| nettai_audio::BattleAudio::with_banks(s.banks.clone(), s.songs.clone()));
        Player::with(renderer, font.map(TextRenderer::new), audio, driver)
    }

    /// Start a battle: the two cards while it loads, then the battle. The battle before it, if there is one,
    /// ends unheard.
    pub fn start_battle(&mut self, start: BattleStart) {
        self.battle = None;
        self.nav.close("menu.");
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        if let Some([left, right]) = &start.cards {
            shell.set_versus_left(left.clone());
            shell.set_versus_right(right.clone());
            shell.set_versus(true);
        }
        shell.set_battle(BattleKind::None);
        shell.set_battle_menu(false);
        self.loading = Some(Loading { start, began: Instant::now(), step: 0, made: None, stage: None, theirs: 0.0 });
    }

    /// A frame of the cards: one step of the loading, and the battle begun once everything is loaded, both
    /// sides are, and the cards were shown long enough. The owner, when the battle began.
    pub(crate) fn load_battle(&mut self, now: Instant) -> Option<ScreenId> {
        let mut loading = self.loading.take()?;
        let Some(ready) = self.games.ready(&loading.start.game) else {
            crate::errors::log("starting a battle", &format_args!("{} isn't loaded", loading.start.game));
            self.loading_failed(UserError::Other);
            return None;
        };
        // (One step a frame, so the bar is seen to move; the first frame only shows the cards.)
        match loading.step {
            0 => {}
            1 => {
                // (The closure is run once: a boxed one is taken out with a stand-in left in its place.)
                let make = std::mem::replace(&mut loading.start.make, Box::new(|_, _| Err(UserError::Other)));
                match make(self, &ready) {
                    Ok(made) => loading.made = Some(made),
                    Err(e) => {
                        self.loading_failed(e);
                        return None;
                    }
                }
            }
            2 => {
                let Made { driver, recorder } = loading.made.take().expect("made at step 1");
                let mut player = self.player(&ready, driver, true);
                if let Some(recorder) = recorder
                    && let Err(why) = player.record(recorder)
                {
                    crate::errors::log("recording the battle", &why);
                }
                loading.stage = Some(Stage::new(player));
            }
            3 => {
                // The first picture, at the size it will be shown: the text's glyphs are drawn once here,
                // not in the battle's first frames.
                let ui = self.ui();
                let size = ui.window().size();
                let fit = self.fit(size.width as f32, size.height as f32);
                if let Some(stage) = &mut loading.stage {
                    stage.warm(fit);
                }
            }
            _ => {}
        }
        loading.step = (loading.step + 1).min(5);
        let mine = [0.0, 0.0, 0.45, 0.8, 1.0, 1.0][loading.step as usize];
        if let Some(gate) = &mut loading.start.gate {
            loading.theirs = gate(mine);
        } else {
            loading.theirs = mine;
        }
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        if loading.start.cards.is_some() {
            let (mut left, mut right) = (shell.get_versus_left(), shell.get_versus_right());
            (left.progress, right.progress) = (mine, loading.theirs);
            shell.set_versus_left(left);
            shell.set_versus_right(right);
        }
        let shown_enough = loading.start.cards.is_none() || now.saturating_duration_since(loading.began) >= CARDS_SHOWN;
        if loading.step < 5 || loading.theirs < 1.0 || !shown_enough || self.hold_cards {
            self.loading = Some(loading);
            return None;
        }
        // The battle.
        let Loading { start, stage, .. } = loading;
        let mut stage = stage.expect("made at step 2");
        stage.hold();
        let owner = start.owner;
        shell.set_versus(false);
        shell.set_battle(start.kind);
        shell.set_battle_full(start.full);
        shell.set_battle_menu(false);
        shell.set_connection_shown(false);
        self.keys.pressed.clear();
        self.battle = Some(Battle { stage, owner, kind: start.kind, game: start.game, full: start.full, menu: false, over: false, held: 0 });
        Some(owner)
    }

    fn loading_failed(&mut self, error: UserError) {
        self.loading = None;
        self.ui().global::<nettai_ui::Shell>().set_versus(false);
        self.fail(error);
    }

    /// Whether a battle is loading (the cards show).
    pub fn loading_battle(&self) -> bool {
        self.loading.is_some()
    }

    /// Give up the battle being loaded (its owner asks: an invite canceled).
    pub fn cancel_loading(&mut self) {
        if self.loading.take().is_some() {
            self.ui().global::<nettai_ui::Shell>().set_versus(false);
        }
    }

    /// Whether the battle has the keys now.
    pub fn battle_has_keys(&self) -> bool {
        self.battle.as_ref().is_some_and(Battle::playing) && self.dialog.is_none()
    }

    /// Open nettai's menu over the battle. A battle that can stop (training, a replay) is paused under it; an
    /// online match keeps running.
    pub fn open_menu(&mut self) {
        let Some(b) = &mut self.battle else { return };
        if b.menu || b.over {
            return;
        }
        b.menu = true;
        b.stage.player.set_paused(true);
        self.keys.release();
        self.nav.open("menu.", Key::new("menu.resume", 0));
        self.ui().global::<nettai_ui::Shell>().set_battle_menu(true);
    }

    /// Close the menu: the battle has the keys again.
    pub fn close_menu(&mut self) {
        let Some(b) = &mut self.battle else { return };
        if !b.menu {
            return;
        }
        b.menu = false;
        // (What is held as the menu closes isn't a press: START held doesn't open it again.)
        b.held = !0;
        b.stage.player.set_paused(false);
        b.stage.hold();
        self.keys.release();
        self.nav.close("menu.");
        self.ui().global::<nettai_ui::Shell>().set_battle_menu(false);
    }

    /// End the battle: the owner's screen shows again.
    pub fn end_battle(&mut self) {
        if self.battle.take().is_none() {
            return;
        }
        self.nav.close("menu.");
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        shell.set_battle(BattleKind::None);
        shell.set_battle_menu(false);
        shell.set_connection_shown(false);
        self.keys.release();
    }

    /// A frame of the battle: the ticks due with the buttons held, its picture, its samples (into `samples`).
    /// The ticks that ran, for the menus' sound to keep time with (none: the battle is paused); and what its
    /// owner should hear.
    pub(crate) fn run_battle(&mut self, now: Instant, samples: &mut Vec<[f32; 2]>) -> (Option<u32>, Option<(ScreenId, BattleEvent)>) {
        let Some(mut b) = self.battle.take() else { return (None, None) };
        let mut buttons = if b.playing() && self.dialog.is_none() { self.keys.held | self.pad.held(&self.bindings) | self.touch } else { 0 };
        // START is nettai's outside the custom screen: its press opens the menu, and the game never sees it.
        let in_custom = b.stage.player.battle().round.mode == nettai_battle::battle::mode::CUSTOM;
        let fresh = buttons & !b.held;
        b.held = buttons;
        let mut open_menu = false;
        if !in_custom {
            open_menu = fresh & keys::START != 0 && b.playing();
            buttons &= !keys::START;
        }
        let ran = b.stage.advance(now, buttons, &mut self.keys.pressed);
        let ticks = (!b.menu || b.stage.player.real_time()).then_some(ran);
        std::mem::swap(samples, &mut b.stage.samples);
        b.stage.samples.clear();
        self.show(&mut b.stage);
        b.stage.report(now);
        if let Some(s) = b.stage.stats.as_mut() {
            s.audio_queued = s.audio_queued.max(self.sound.queued());
        }
        // The connection's corner.
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        if let Some(net) = b.stage.player.net_status() {
            let ping = net.ping_ms.map(|ms| ms.round() as i32);
            let level = match ping {
                None => 0,
                Some(ms) if ms < 70 => 4,
                Some(ms) if ms < 130 => 3,
                Some(ms) if ms < 220 => 2,
                Some(_) => 1,
            };
            // (A link that loses packets is a bar worse than its time says.)
            let level = if net.loss > 0.05 && level > 1 { level - 1 } else { level };
            shell.set_connection_shown(true);
            shell.set_connection_level(level);
            shell.set_connection_ping(ping.unwrap_or(-1));
            shell.set_connection_down(net.reconnecting.map_or(-1, |d| d.as_secs() as i32));
        }
        // How it ended, told once.
        let mut event = None;
        if !b.over {
            if b.stage.player.finished() {
                b.over = true;
                event = Some(BattleEvent::Finished(b.stage.player.result()));
            } else if let Some(why) = b.stage.player.stopped() {
                crate::errors::log("the battle stopped", &why);
                b.over = true;
                event = Some(BattleEvent::Stopped);
            }
        }
        let owner = b.owner;
        self.battle = Some(b);
        if open_menu && event.is_none() {
            self.open_menu();
        }
        (ticks, event.map(|e| (owner, e)))
    }

    /// How the picture fits a room of `width` by `height` display pixels: `stage::fit`, held to the settings'
    /// picture size.
    pub fn fit(&self, width: f32, height: f32) -> stage::Fit {
        let mut fit = stage::fit(width, height, self.scale.os, self.sharp);
        if self.settings.picture_size > 0 && fit.scale > self.settings.picture_size {
            fit.scale = self.settings.picture_size;
            fit.density = fit.density.min(fit.scale);
        }
        fit
    }

    /// Show `stage`'s picture in the room the screen's `Picture` has: presented if a tick ran or the room
    /// changed, or if another stage's picture was shown last.
    pub fn show(&mut self, stage: &mut Stage) {
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        let factor = ui.window().scale_factor();
        let (width, height) = (shell.get_picture_room_width() * factor, shell.get_picture_room_height() * factor);
        if width < 1.0 || height < 1.0 {
            return;
        }
        if self.shown_stage != stage.id {
            self.shown_stage = stage.id;
            stage.refresh();
        }
        let fit = self.fit(width, height);
        if let stage::Shown::New(image) = stage.picture(fit, self.gl.as_mut()) {
            shell.set_picture(image);
        }
        shell.set_picture_width(nettai_render::compose::WIDTH as f32 * fit.scale as f32 / factor);
        shell.set_picture_height(nettai_render::compose::HEIGHT as f32 * fit.scale as f32 / factor);
    }

    /// A recorder of a set to a new file in the replays folder, this player on `side`, both players' names by
    /// side. None if the file can't be made (why is in the log).
    pub fn recorder(
        &self,
        content: &std::sync::Arc<nettai_battle::Content>,
        m: &nettai_match::Match,
        seed: u32,
        game: &str,
        side: u8,
        names: [String; 2],
    ) -> Option<Recorder> {
        use nettai_frontend::replay::Info;
        let dir = crate::paths::replays();
        let made = (|| -> Result<Recorder, String> {
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let stamp = chrono::DateTime::from_timestamp(when as i64, 0).map(|t| t.with_timezone(&chrono::Local).format("%Y%m%d-%H%M%S").to_string()).unwrap_or_default();
            let path = dir.join(format!("{stamp}-{game}.ntrp"));
            let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let info = Info { when, side, names };
            let m = nettai_match::Match { seed: Some(seed), ..m.clone() };
            Recorder::new(Box::new(std::io::BufWriter::new(file)), content, &m, &info)
        })();
        made.map_err(|why| crate::errors::log("making a replay's file", &why)).ok()
    }
}
