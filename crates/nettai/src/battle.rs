//! A battle on screen (docs/app.md §1): the two cards while it is made ready, then the battle itself, with
//! nettai's menu over it when Start or Esc asks.
//!
//! A screen starts one with [`Shell::start_battle`] and stays its owner: the menu's controls are the owner's
//! (the training menu is Training's, the match menu Play's: their names begin `menu.`), and the owner hears
//! when the battle begins and ends ([`BattleEvent`]). The shell does the rest: it makes the match ready before
//! its first tick (the library's `preload`: docs/frontend.md §9), runs the ticks due each frame with the buttons
//! held, presents the picture, and takes the keys for the battle while nothing is over it.
//!
//! The game's own pause never runs: START in a fight is reported by the library, not acted on, and what it
//! opens is nettai's menu.

use crate::errors::UserError;
use crate::nav::Key;
use crate::screens::ScreenId;
use crate::shell::Shell;
use crate::stage::{self, Stage};
use nettai_battle::BattleResult;
use nettai_frontend::driver::Driver;
use nettai_frontend::game::TextMode;
use nettai_frontend::preload::{Part, Preload};
use nettai_frontend::replay::Recorder;
use nettai_match::Set;
use nettai_render::vfont::TextRenderer;
use nettai_ui::{BattleKind, VersusCard};
use slint::ComponentHandle;
use std::time::{Duration, Instant};

/// How long the two cards show at the least. A match is ready in a few milliseconds once the game is loaded,
/// and a screen shown for two frames is a flash; the bars say how far the loading is, not this.
const CARDS_SHOWN: Duration = Duration::from_millis(500);

/// How long a frame of the cards spends making the match ready, at the most.
const STEP_BUDGET: Duration = Duration::from_millis(6);

/// A battle to start.
pub struct BattleStart {
    /// The screen that starts it: its menu's controls and its events are that screen's.
    pub owner: ScreenId,
    /// Which menu Start opens.
    pub kind: BattleKind,
    /// The game it is of (ready).
    pub game: String,
    /// The set played, and what plays it: a live set against a dummy, a match over a link, a replay.
    pub set: Set,
    pub driver: Box<dyn Driver>,
    /// The side whose console is shown: the local player's.
    pub viewer: u8,
    /// What it is recorded with, if it is.
    pub recorder: Option<Recorder>,
    /// The two cards shown while it is made ready, left and right; none: it starts with no cards (the training
    /// battle beside a waiting invite).
    pub cards: Option<[VersusCard; 2]>,
    /// The picture fills the window. Else the owner's screen shows it in a `Picture` of its own.
    pub full: bool,
    /// Online: called each frame with how far this side is (0 to 1), it tells the other side and says how far
    /// that one is. The battle starts when both are 1. None: nobody to wait for.
    pub gate: Option<Box<dyn FnMut(f32) -> f32>>,
}

/// What a battle's owner hears of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleEvent {
    /// It is ready and its first tick is about to run.
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
    /// The side whose console is shown.
    pub viewer: u8,
    /// nettai's menu is over it.
    menu: bool,
    /// The owner has heard how it ended.
    over: bool,
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

/// A battle being made ready.
pub struct Loading {
    start: BattleStart,
    began: Instant,
    /// Made on the first frame after the cards are up.
    preload: Option<Preload>,
    theirs: f32,
}

impl Shell {
    /// Start a battle: the two cards while it is made ready, then the battle. The battle before it, if there
    /// is one, ends unheard.
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
        self.loading = Some(Loading { start, began: Instant::now(), preload: None, theirs: 0.0 });
    }

    /// A frame of the cards: the match made ready a few steps at a time, and the battle begun once it is, the
    /// other side is, and the cards were up long enough to be seen. The owner, when the battle began.
    pub(crate) fn load_battle(&mut self, now: Instant) -> Option<ScreenId> {
        let mut loading = self.loading.take()?;
        let Some(ready) = self.games.ready(&loading.start.game) else {
            crate::errors::log("starting a battle", &format_args!("{} isn't ready", loading.start.game));
            self.loading_failed();
            return None;
        };
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        match &mut loading.preload {
            // (The first frame shows the cards; the work starts on the next.)
            None => {
                // The picture's size as the battle will show it: the text is laid out at it.
                let size = ui.window().size();
                let fit = self.fit(size.width as f32, size.height as f32);
                let size = (nettai_render::compose::WIDTH * fit.density as usize, nettai_render::compose::HEIGHT * fit.density as usize);
                let text = if self.settings.sharp_text { TextMode::Font } else { TextMode::Original };
                let graphics = ready.graphics(self.lang);
                let font = ready.loaded.font.clone().filter(|_| text == TextMode::Font);
                let renderer = graphics.renderer(text, font.clone());
                let sound = ready.loaded.sound.as_ref().map(|s| (s.banks.clone(), s.songs.clone()));
                loading.preload = Some(Preload::with(renderer, font.map(TextRenderer::new), sound, &loading.start.set, loading.start.viewer, size));
            }
            Some(preload) => {
                let began = Instant::now();
                while preload.step() && began.elapsed() < STEP_BUDGET {}
                if let Some(why) = preload.stopped() {
                    crate::errors::log("making a battle ready", &why);
                    self.loading_failed();
                    return None;
                }
            }
        }
        let preload = loading.preload.as_ref().expect("made above");
        let mine = preload.fraction();
        loading.theirs = match &mut loading.start.gate {
            Some(gate) => gate(mine),
            None => mine,
        };
        if loading.start.cards.is_some() {
            // A bar a card: online each is its machine's; alone both are this one's, each side's own steps
            // with the match's.
            let side = |n: u8| (preload.fraction_of(Part::Match) + preload.fraction_of(Part::Side(n))) / 2.0;
            let bars = if loading.start.gate.is_some() { [mine, loading.theirs] } else { [side(0), side(1)] };
            let bars = if loading.start.viewer == 1 && loading.start.gate.is_none() { [bars[1], bars[0]] } else { bars };
            let (mut left, mut right) = (shell.get_versus_left(), shell.get_versus_right());
            (left.progress, right.progress) = (bars[0], bars[1]);
            shell.set_versus_left(left);
            shell.set_versus_right(right);
        }
        let seen = loading.start.cards.is_none() || now.saturating_duration_since(loading.began) >= CARDS_SHOWN;
        if !preload.done() || loading.theirs < 1.0 || !seen || self.hold_cards {
            self.loading = Some(loading);
            return None;
        }
        // The battle.
        let Loading { start, preload, .. } = loading;
        let preload = preload.expect("made above");
        for missing in preload.missing() {
            crate::errors::log("the pack lacks something this battle draws", &missing);
        }
        let mut player = preload.player(start.driver);
        if let Some(recorder) = start.recorder
            && let Err(why) = player.record(recorder)
        {
            crate::errors::log("recording the battle", &why);
        }
        let mut stage = Stage::new(player);
        stage.hold();
        shell.set_versus(false);
        shell.set_battle(start.kind);
        shell.set_battle_full(start.full);
        shell.set_battle_menu(false);
        shell.set_connection_shown(false);
        self.keys.pressed.clear();
        self.battle = Some(Battle { stage, owner: start.owner, kind: start.kind, game: start.game, full: start.full, viewer: start.viewer, menu: false, over: false });
        Some(start.owner)
    }

    fn loading_failed(&mut self) {
        self.loading = None;
        self.ui().global::<nettai_ui::Shell>().set_versus(false);
        self.fail(UserError::Other);
    }

    /// Whether a battle is being made ready (the cards show).
    pub fn loading_battle(&self) -> bool {
        self.loading.is_some()
    }

    /// Give up the battle being made ready (its owner asks: an invite canceled).
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
        b.stage.player.set_paused(false);
        b.stage.hold();
        self.keys.release();
        self.nav.close("menu.");
        self.ui().global::<nettai_ui::Shell>().set_battle_menu(false);
    }

    /// End the battle: the owner's screen shows again.
    pub fn end_battle(&mut self) {
        let Some(b) = self.battle.take() else { return };
        // The loading screens' check: a battle made ready loads no runtime and reads nothing while it plays.
        let late = b.stage.player.late();
        if late.runtimes > 0 || late.reads > 0 {
            crate::errors::log("a battle loaded late", &format_args!("{} runtimes, {} parts read from disk, while it played", late.runtimes, late.reads));
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
        let buttons = if b.playing() && self.dialog.is_none() { self.keys.held | self.pad.held(&self.bindings) | self.touch } else { 0 };
        let ran = b.stage.advance(now, buttons, &mut self.keys.pressed);
        let ticks = (!b.menu || b.stage.player.real_time()).then_some(ran);
        // START in the fight: the game's pause is nettai's menu.
        let asked = b.stage.player.take_pause_requests()[b.viewer.min(1) as usize] && b.playing();
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
        if asked && event.is_none() {
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
    pub fn recorder(&self, content: &std::sync::Arc<nettai_battle::Content>, m: &nettai_match::Match, seed: u32, game: &str, side: u8, names: [String; 2]) -> Option<Recorder> {
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
