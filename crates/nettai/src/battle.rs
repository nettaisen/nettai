//! A battle on screen (docs/app.md §1): the two cards while it is made ready, then the battle itself, with
//! nettai's menu over it when Start or Esc asks.
//!
//! A screen starts one with [`Shell::start_battle`] and stays its owner: the menu's controls are the owner's
//! (the practice menu is Practice's, the match menu Play's: their names begin `menu.`), and the owner hears
//! when the battle begins and ends ([`BattleEvent`]). The shell does the rest: it makes the match ready before
//! its first tick (the library's `preload`: docs/frontend.md §9), runs the ticks due each frame with the buttons
//! held, presents the picture, and takes the keys for the battle while nothing is over it.
//!
//! The game's own pause never runs: START in a fight is reported by the library, not acted on, and what it
//! opens is nettai's menu.
//!
//! A replay is watched through the library's `Viewer`, which is no battle of the shell's: the cards make it
//! ready the same way ([`Play::Watch`]) and hand it to the screen that asked ([`BattleEvent::Watching`]),
//! which keeps it, shows it in a `Picture` of its own ([`Shell::show_surface`]) and plays its sound
//! ([`Shell::play_samples`]).

use crate::errors::UserError;
use crate::nav::Key;
use crate::screens::ScreenId;
use crate::shell::Shell;
use crate::stage::{self, Stage, Surface};
use nettai_battle::BattleResult;
use nettai_frontend::driver::{Driver, StopReason};
use nettai_frontend::game::TextMode;
use nettai_frontend::preload::{Part, Preload};
use nettai_frontend::replay::{Recorder, Replay};
use nettai_frontend::viewer::Viewer;
use nettai_match::Set;
use nettai_render::vfont::TextRenderer;
use nettai_ui::{BattleKind, VersusCard};
use slint::ComponentHandle;
use std::time::{Duration, Instant};

mod game_pause;

/// How long the two cards show at the least. A match is ready in a few milliseconds once the game is loaded,
/// and a screen shown for two frames is a flash; the bars say how far the loading is, not this.
const CARDS_SHOWN: Duration = Duration::from_millis(500);

/// How long a frame of the cards spends making the match ready, at the most.
const STEP_BUDGET: Duration = Duration::from_millis(6);

/// What the cards make ready.
pub enum Play {
    /// A battle the shell runs: what plays its set (a live set against a dummy, a match over a link).
    Drive(Box<dyn Driver>),
    /// The same, its driver made as the battle begins: an online match's, on the channel its lobby gives once
    /// both sides have loaded (the `gate` holds the start until then). None: there is none after all, and no
    /// battle begins.
    Make(Box<dyn FnOnce() -> Option<Box<dyn Driver>>>),
    /// A replay to watch, from a side (none: its recorder's). Its set is `replay::set_of`'s.
    Watch(Box<Replay>, Option<u8>),
}

/// A battle to start.
pub struct BattleStart {
    /// The screen that starts it: its menu's controls and its events are that screen's.
    pub owner: ScreenId,
    /// Which menu Start opens.
    pub kind: BattleKind,
    /// The game it is of (ready).
    pub game: String,
    /// The set played, and what plays it.
    pub set: Set,
    pub play: Play,
    /// The side whose console is shown: the local player's.
    pub viewer: u8,
    /// What it is recorded with, if it is.
    pub recorder: Option<Recorder>,
    /// The two cards shown while it is made ready, left and right; none: it starts with no cards (the practice
    /// battle beside a waiting invite).
    pub cards: Option<[VersusCard; 2]>,
    /// The picture fills the window. Else the owner's screen shows it in a `Picture` of its own.
    pub full: bool,
    /// Online: called each frame with how far this side is (0 to 1), it tells the other side and says how far
    /// that one is. The battle starts when both are 1. None: nobody to wait for.
    pub gate: Option<Box<dyn FnMut(f32) -> f32>>,
}

/// What a battle's owner hears of it.
pub enum BattleEvent {
    /// It is ready and its first tick is about to run.
    Started,
    /// The replay asked for is ready to watch: its viewer, which is the owner's from here.
    Watching(Box<Viewer>),
    /// The set is over: how it went for this player (none: nobody's).
    Finished(Option<BattleResult>),
    /// It stopped without ending: why, as a player can be told it (`UserError::of_stop`). The detail is in
    /// the log.
    Stopped(StopReason),
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
    /// Under the menu it is held in the game's own pause, not stopped ([`Battle::pause_in_game`]).
    game_pause: Option<game_pause::GamePause>,
}

impl Battle {
    /// Whether the battle has the keys: it runs, and nothing is over it.
    pub fn playing(&self) -> bool {
        !self.menu && !self.over
    }

    pub fn menu_open(&self) -> bool {
        self.menu
    }

    /// Start what it plays over from its beginning (a set from its first round), if it can stop (not an online
    /// match): it runs again, and its owner will hear of its end again (a practice set that ended starts over).
    /// False, and nothing changes, in real time.
    pub fn restart(&mut self) -> bool {
        if !self.stage.player.restart() {
            return false;
        }
        if let Some(g) = &mut self.game_pause {
            g.reset();
        }
        self.over = false;
        self.stage.hold();
        self.stage.surface.stale = true;
        true
    }

    /// Hold it in the game's own pause under the menu, from now on (Practice's: docs/app.md §15): the menu
    /// opening gives the game the START its player would, and the battle steps on in the game's pause, its
    /// sound and what the game moves while paused going on; the menu closing gives it START again. The player's
    /// own START in the fight is the game's, and the menu opens over the pause it makes. Its round must leave
    /// the game its START (`HostKeys::pause` off). Where the game wouldn't pause (its custom screen, a dimming,
    /// a round's end) the battle steps on under the menu with no buttons, neither the player's nor the dummy's:
    /// on the custom screen it waits there, and otherwise it goes into the game's pause once START would pause
    /// it (`battle/game_pause.rs`).
    pub fn pause_in_game(&mut self) {
        self.game_pause.get_or_insert_with(Default::default);
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
            // (The field's hexagons rush as a battle is entered.)
            shell.set_rush_seq(shell.get_rush_seq() + 1);
        }
        shell.set_battle(BattleKind::None);
        shell.set_battle_menu(false);
        self.loading = Some(Loading { start, began: Instant::now(), preload: None, theirs: 0.0 });
    }

    /// A frame of the cards: the match made ready a few steps at a time, and the battle begun once it is, the
    /// other side is, and the cards were up long enough to be seen. What its owner hears, once it is ready.
    pub(crate) fn load_battle(&mut self, now: Instant) -> Option<(ScreenId, BattleEvent)> {
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
                // (The music the player picked, the battle's game's own or a theme: `crate::music`.)
                let sound = crate::music::sound(&self.games, &ready, &self.battle_music());
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
        let driver = match start.play {
            Play::Drive(driver) => driver,
            Play::Make(make) => match make() {
                Some(driver) => driver,
                None => {
                    crate::errors::log("starting a battle", &"its driver wasn't made");
                    self.loading_failed();
                    return None;
                }
            },
            // A replay's viewer is its screen's: the cards go, and the screen shows it.
            // (Its set is started again there: an engine that stops is a failure, not a crash.)
            Play::Watch(replay, side) => {
                shell.set_versus(false);
                return match nettai_frontend::session::in_engine(|| preload.viewer(&replay, side)) {
                    Ok(Ok(viewer)) => Some((start.owner, BattleEvent::Watching(Box::new(viewer)))),
                    Ok(Err(refused)) => {
                        self.fail(UserError::of_refusal("opening a replay", &refused));
                        None
                    }
                    Err(panic) => {
                        crate::errors::log("opening a replay", &format_args!("the engine stopped starting its set: {}", nettai_frontend::session::panic_message(panic.as_ref())));
                        self.fail(UserError::Other);
                        None
                    }
                };
            }
        };
        let mut player = preload.player(driver);
        if let Some(recorder) = start.recorder
            && let Err(why) = player.record(recorder)
        {
            crate::errors::log("recording the battle", &why);
        }
        let mut stage = Stage::new(player);
        let (music, effects) = self.sound.battle_levels();
        stage.player.set_sound_levels(music, effects);
        stage.hold();
        shell.set_versus(false);
        if start.full {
            // (The screen wipes into the battle.)
            shell.set_wipe_seq(shell.get_wipe_seq() + 1);
        }
        shell.set_battle(start.kind);
        shell.set_battle_full(start.full);
        shell.set_battle_menu(false);
        shell.set_connection_shown(false);
        self.keys.pressed.clear();
        self.battle = Some(Battle { stage, owner: start.owner, kind: start.kind, game: start.game, full: start.full, viewer: start.viewer, menu: false, over: false, game_pause: None });
        Some((start.owner, BattleEvent::Started))
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

    /// Whether the battle being made ready is `owner`'s.
    pub fn loading_of(&self, owner: ScreenId) -> bool {
        self.loading.as_ref().is_some_and(|l| l.start.owner == owner)
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

    /// Open nettai's menu over the battle. A battle that can stop (practice, a replay) is paused under it; an
    /// online match keeps running.
    pub fn open_menu(&mut self) {
        let Some(b) = &mut self.battle else { return };
        if b.menu || b.over {
            return;
        }
        b.menu = true;
        // (One held in the game's own pause steps on: `GamePause` pauses it.)
        if b.game_pause.is_none() {
            b.stage.player.set_paused(true);
        }
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
        if b.game_pause.is_none() {
            b.stage.player.set_paused(false);
        }
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
        // Its picture is nobody's now: taken away at once (a screen that leaves next keeps what it showed).
        self.showing.reset();
        if let Some(shown) = self.showing.tell() {
            shell.set_picture_shown(shown);
        }
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
        if let Some(g) = &mut b.game_pause {
            g.before(b.menu, &mut b.stage.player);
        }
        let held = if b.playing() && self.dialog.is_none() { self.keys.held | self.pad.held(&self.bindings) | self.touch.held() } else { 0 };
        let buttons = held | b.game_pause.as_ref().map_or(0, game_pause::GamePause::buttons);
        let ran = b.stage.advance(now, buttons, &mut self.keys.pressed);
        // (A battle held in the game's own pause steps under the menu: its ticks are the sound's.)
        let ticks = (!b.menu || b.stage.player.real_time() || b.game_pause.is_some()).then_some(ran);
        let (viewer, menu) = (b.viewer, b.menu);
        let paused = b.game_pause.as_mut().is_some_and(|g| g.after(ran, &b.stage.player, viewer, menu));
        // START in the fight: the game's pause is nettai's menu (asked of the host, or the game's own).
        let asked = (b.stage.player.take_pause_requests()[b.viewer.min(1) as usize] && b.playing()) || paused;
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
            // (The bars are the library's quality, from the round trip and how steady it is; while the link
            // is down, the seconds until it gives up, or those it has been down where it never does.)
            shell.set_connection_shown(true);
            shell.set_connection_level(net.quality.map_or(0, |q| q.bars() as i32));
            shell.set_connection_ping(net.ping_ms.map_or(-1, |ms| ms.round() as i32));
            shell.set_connection_down(net.reconnecting.map_or(-1, |r| r.gives_up_in.unwrap_or(r.elapsed).as_secs() as i32));
        }
        // How it ended, told once.
        let mut event = None;
        if !b.over {
            if b.stage.player.finished() {
                b.over = true;
                event = Some(BattleEvent::Finished(b.stage.player.result()));
            } else if let Some(why) = b.stage.player.stopped() {
                let reason = b.stage.player.stop_reason().unwrap_or(StopReason::Internal);
                crate::errors::log("the battle stopped", &format_args!("{reason:?}: {why}"));
                b.over = true;
                event = Some(if reason == StopReason::Finished { BattleEvent::Finished(b.stage.player.result()) } else { BattleEvent::Stopped(reason) });
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
    /// changed, or if another surface's picture was shown last.
    pub fn show(&mut self, stage: &mut Stage) {
        // (A frame the window doesn't draw shows nothing: the picture stays out of date for the next it does.)
        if !self.drawing {
            return;
        }
        let Some(fit) = self.room_fit(&mut stage.surface) else { return };
        let shown = stage.picture(fit, self.gl.as_mut());
        self.place_picture(fit, shown);
    }

    /// Show a picture of a screen's own in the room its `Picture` has (a replay's viewer): `present` draws it
    /// into the buffer it is given, when `surface` is out of date (the screen sets `surface.stale` as its
    /// picture changes) or the room changed.
    pub fn show_surface(&mut self, surface: &mut Surface, present: impl FnOnce(&mut [u32], usize, usize)) {
        if !self.drawing {
            return;
        }
        let Some(fit) = self.room_fit(surface) else { return };
        let shown = surface.picture(fit, self.gl.as_mut(), present);
        self.place_picture(fit, shown);
    }

    /// Play samples of a screen's own this frame (a replay's viewer's: 32768 Hz, stereo), which ran `ticks` of
    /// what made them. They are mixed with the menus' sounds, as a battle's are.
    pub fn play_samples(&mut self, samples: &mut Vec<[f32; 2]>, ticks: u32) {
        self.own_samples.append(samples);
        self.own_ticks = Some(self.own_ticks.unwrap_or(0) + ticks);
    }

    /// How the picture fits the room the screen's `Picture` has, if there is one; `surface` is shown this frame
    /// (`stage::Showing`), and is to be presented anew if the window's texture holds another's picture. None
    /// while a screen leaves: the room is still the leaving one's, and its `Picture` keeps what it showed.
    fn room_fit(&mut self, surface: &mut Surface) -> Option<stage::Fit> {
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        let factor = ui.window().scale_factor();
        let (width, height) = (shell.get_picture_room_width() * factor, shell.get_picture_room_height() * factor);
        if width < 1.0 || height < 1.0 || self.leaving() {
            return None;
        }
        self.showing.show(surface);
        Some(self.fit(width, height))
    }

    /// The picture's image (a new one), its size and its place in its room.
    fn place_picture(&mut self, fit: stage::Fit, shown: stage::Shown) {
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        let factor = ui.window().scale_factor();
        let (width, height) = (shell.get_picture_room_width() * factor, shell.get_picture_room_height() * factor);
        if let stage::Shown::New(image) = shown {
            shell.set_picture(image);
        }
        let size = (nettai_render::compose::WIDTH as f32 * fit.scale as f32, nettai_render::compose::HEIGHT as f32 * fit.scale as f32);
        shell.set_picture_width(size.0 / factor);
        shell.set_picture_height(size.1 / factor);
        // In the room's middle, on the display's pixels: each of the game's pixels is one square of them.
        let room = (shell.get_picture_room_x() * factor, shell.get_picture_room_y() * factor);
        shell.set_picture_x(((room.0 + (width - size.0) / 2.0).round() - room.0) / factor);
        shell.set_picture_y(((room.1 + (height - size.1) / 2.0).round() - room.1) / factor);
    }

    /// A recorder of an offline set to a new file in the replays folder, this player on `side`, both players'
    /// names by side. None if the file can't be made (why is in the log). (An online match is recorded from
    /// its agreement: the library's `Recorder::of_agreement`.)
    pub fn recorder(&self, content: &std::sync::Arc<nettai_battle::Content>, m: &nettai_match::Match, seed: u32, game: &str, side: u8, names: [String; 2]) -> Option<Recorder> {
        use nettai_frontend::replay::Info;
        let dir = crate::paths::replays();
        let made = (|| -> Result<Recorder, String> {
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let stamp = chrono::DateTime::from_timestamp(when as i64, 0).map(|t| t.with_timezone(&chrono::Local).format("%Y%m%d-%H%M%S").to_string()).unwrap_or_default();
            let path = dir.join(format!("{stamp}-{game}.ntrp"));
            let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let info = Info { when, names, ..Info::of(nettai_frontend::replay::MatchKind::Offline, side) };
            let m = nettai_match::Match { seed: Some(seed), ..m.clone() };
            Recorder::new(Box::new(std::io::BufWriter::new(file)), content, &m, &info)
        })();
        made.map_err(|why| crate::errors::log("making a replay's file", &why)).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_frontend::driver::LivePlayer;
    use nettai_frontend::player::Player;
    use nettai_frontend::testing::{graphics, short_set};

    /// A battle whose set was played to its end starts over (practice has no end): it has the keys again,
    /// from the set's first tick, and plays on; its owner will hear of its next end.
    #[test]
    fn a_set_that_ended_starts_over() {
        let content = nettai_match::testing::exe6_content();
        // (A set a shot decides: the dummy has 1 HP.)
        let m = short_set::of(&content, "exe6", 7);
        let driver = LivePlayer::new(Set::of(&content, &m, 7));
        let player = Player::with(graphics(&content, 0x7FFF).renderer(TextMode::Original, None), None, None, Box::new(driver));
        let mut b = Battle { stage: Stage::new(player), owner: ScreenId::Practice, kind: BattleKind::Practice, game: "exe6".into(), full: true, viewer: 0, menu: false, over: false, game_pause: None };
        let play_out = |b: &mut Battle| {
            while b.stage.player.tick(short_set::shooter(b.stage.player.battle(), 0, b.stage.player.ticks() as u32)) {
                assert!(b.stage.player.ticks() < 30_000, "the set doesn't end");
            }
            assert!(b.stage.player.finished());
            b.stage.player.ticks()
        };
        let ticks = play_out(&mut b);
        // (As the shell leaves it once its owner has heard.)
        b.over = true;
        assert!(!b.playing());
        assert!(b.restart());
        assert!(b.playing() && !b.stage.player.finished());
        assert_eq!((b.stage.player.ticks(), b.stage.player.stopped()), (0, None));
        // The same set again, to the same end.
        assert_eq!(play_out(&mut b), ticks);
    }

    /// A frame of the shell's for a battle held in the game's own pause, as `Shell::run_battle` runs one, a
    /// tick in it: the player's `keys` (none under the menu), the battle's START given where it is. Whether
    /// the menu is to open.
    fn frame(b: &mut Battle, keys: u16) -> bool {
        b.game_pause.as_mut().expect("held in the game's pause").before(b.menu, &mut b.stage.player);
        let buttons = if b.playing() { keys } else { 0 } | b.game_pause.as_ref().expect("held").buttons();
        // (A battle the player holds runs no tick, as `Player::advance` runs none.)
        let ran = if b.stage.player.paused() { 0 } else { b.stage.player.tick(buttons) as u32 };
        let (viewer, menu) = (b.viewer, b.menu);
        b.game_pause.as_mut().expect("held").after(ran, &b.stage.player, viewer, menu)
    }

    /// The practice menu holds its battle in the game's own pause: the menu opening gives the game its START,
    /// and the battle steps on in its pause, its sound going on, nothing of the fight moving and none of the
    /// player's buttons reaching it; closing gives START again and the fight goes on where it was. The player's
    /// own START is the game's, and asks for the menu. On the custom screen, where the game wouldn't pause, the
    /// battle steps on with no buttons: its sound goes on, and nothing is chosen. Starting over leaves nothing
    /// held.
    #[test]
    fn a_battle_held_in_the_games_own_pause() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 11, None).unwrap();
        let mut first = m.round(&content, 11);
        first.host_keys.pause = false;
        let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
        let set = Set::new(content.clone(), first, folders);
        let opponent = nettai_frontend::driver::Opponent { in_fight: nettai_frontend::driver::InFight::AttacksAtRandom, ..Default::default() };
        use nettai_battle::battle::mode;
        let driver = LivePlayer::against(set, opponent, 11);
        let audio = nettai_frontend::testing::audio(&content);
        let player = Player::with(graphics(&content, 0x7FFF).renderer(TextMode::Original, None), None, Some(audio), Box::new(driver));
        let mut b = Battle { stage: Stage::new(player), owner: ScreenId::Practice, kind: BattleKind::Practice, game: "exe6".into(), full: true, viewer: 0, menu: false, over: false, game_pause: None };
        b.pause_in_game();
        // Onto the custom screen, a chip picked; the menu opened there: the battle steps on with no buttons, its
        // sound going on, both screens' cursors and choices as they were (the dummy's presses held too).
        let custom = |b: &Battle| nettai_frontend::driver::bot_buttons(b.stage.player.battle(), 0, b.stage.player.ticks() as u32);
        let choosing = |b: &Battle| {
            let battle = b.stage.player.battle();
            battle.round.mode == mode::CUSTOM && battle.custom.sides.iter().all(|s| s.screen.as_ref().is_some_and(|s| s.phase == nettai_battle::custom::Phase::Choosing))
        };
        while !choosing(&b) {
            assert!(!frame(&mut b, 0) && b.stage.player.ticks() < 3000);
        }
        // (A chip picked, as the player's screen takes it.)
        for keys in [nettai_battle::input::keys::A, 0, 0, 0] {
            frame(&mut b, keys);
        }
        let chosen = |b: &Battle| {
            let battle = b.stage.player.battle();
            battle.custom.sides.each_ref().map(|side| side.screen.as_ref().map(|s| (s.cursor, s.selected, s.slots.iter().map(|slot| slot.state).collect::<Vec<_>>())))
        };
        let before = chosen(&b);
        b.menu = true;
        b.stage.player.take_samples(&mut Vec::new());
        let (ticks, mut made) = (b.stage.player.ticks(), 0);
        for _ in 0..240 {
            // (The player's buttons are the menu's.)
            frame(&mut b, nettai_battle::input::keys::A | nettai_battle::input::keys::START);
            let mut samples = Vec::new();
            b.stage.player.take_samples(&mut samples);
            made += samples.len();
        }
        assert_eq!(b.stage.player.ticks(), ticks + 240, "it steps on the custom screen");
        assert!((240 * 548..=240 * 549).contains(&made), "{made} samples: the sound goes on");
        assert!(choosing(&b) && b.stage.player.battle().paused_by().is_none(), "no pause there");
        assert_eq!(chosen(&b), before, "nothing chosen, neither side");
        b.menu = false;
        frame(&mut b, 0);
        assert!(!b.stage.player.paused());
        while !b.stage.player.battle().pausable() {
            let keys = custom(&b);
            assert!(!frame(&mut b, keys) && b.stage.player.ticks() < 3000);
        }
        for _ in 0..90 {
            let keys = nettai_battle::input::keys::B * (b.stage.player.ticks() % 2 == 0) as u16;
            frame(&mut b, keys);
        }
        // The menu opens: the game's pause, from the next tick.
        let fight = |b: &Battle| {
            let battle = b.stage.player.battle();
            // (The two navis as they stand, to the field.)
            let navis = [0, 1].map(|side| {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                battle.player_actor(side).map(|a| battle.actors.get(a).hash(&mut h));
                h.finish()
            });
            (battle.sides.map(|s| s.gauge), battle.round.battle_time, navis)
        };
        b.menu = true;
        frame(&mut b, 0);
        assert_eq!(b.stage.player.battle().paused_by(), Some(0));
        let held = fight(&b);
        let mut samples = Vec::new();
        b.stage.player.take_samples(&mut samples);
        samples.clear();
        let ticks = b.stage.player.ticks();
        let mut made = 0;
        for n in 0..300 {
            // (The player's buttons are the menu's.)
            frame(&mut b, nettai_battle::input::keys::A | nettai_battle::input::keys::B * (n % 2) as u16);
            assert_eq!(b.stage.player.battle().paused_by(), Some(0), "tick {n}");
            b.stage.player.take_samples(&mut samples);
            made += samples.len();
            samples.clear();
        }
        assert_eq!(b.stage.player.ticks(), ticks + 300, "it steps in the game's pause");
        assert_eq!(fight(&b), held, "nothing of the fight moved");
        assert!((300 * 548..=300 * 549).contains(&made), "{made} samples: the sound goes on");
        // The menu closes: the game leaves its pause, and the fight goes on from where it was.
        b.menu = false;
        for _ in 0..3 {
            frame(&mut b, 0);
        }
        assert_eq!(b.stage.player.battle().paused_by(), None);
        for _ in 0..60 {
            frame(&mut b, 0);
        }
        assert_ne!(fight(&b), held, "the fight runs again");
        // The player's own START pauses the game, and asks for the menu.
        let asked = (0..3).map(|_| frame(&mut b, nettai_battle::input::keys::START)).any(|asked| asked);
        assert!(asked && b.stage.player.battle().paused_by() == Some(0));
        b.menu = true;
        frame(&mut b, 0);
        assert_eq!(b.stage.player.battle().paused_by(), Some(0));
        // Starting over from the pause: the game's from its start, nothing held.
        b.menu = false;
        assert!(b.restart());
        assert_eq!((b.stage.player.battle().paused_by(), b.stage.player.paused(), b.stage.player.ticks()), (None, false, 0));
        assert!(!frame(&mut b, 0) && b.game_pause.as_ref().unwrap().buttons() == 0);
    }

    /// A practice set put in place of the one playing (Practice's Arena or Background row changed:
    /// `LivePlayer::play_set`) starts over on its place under the open menu, as RESTART starts a set: the
    /// battle steps through the round's opening with no buttons, its sound going on, and waits on the custom
    /// screen; every round of it is on that place.
    #[test]
    fn under_the_menu_a_set_starts_over_on_another_place() {
        use nettai_battle::battle::mode;
        let content = nettai_match::testing::exe6_content();
        let mut m = short_set::of(&content, "exe6", 7);
        let mut b = practice(&content, &m, 7, Default::default());
        for _ in 0..400 {
            frame(&mut b, 0);
        }
        b.menu = true;
        frame(&mut b, 0);
        let stage = *nettai_match::link_battle_stages(&content, "exe6").last().unwrap();
        let background = nettai_match::ids::backgrounds(&content, "exe6").last().unwrap().to_string();
        let place = nettai_match::RoundSettings { stage: Some(stage), background: Some(background.clone()) };
        m.rounds.iter_mut().for_each(|r| *r = place.clone());
        let mut first = m.round(&content, 7);
        first.host_keys.pause = false;
        let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
        assert_ne!(b.stage.player.battle().setup.settings.stage, stage, "(another place before)");
        b.stage.player.live_mut().unwrap().play_set(Set::new(content.clone(), first, folders));
        assert!(b.restart());
        let background = nettai_match::ids::background(&content, "exe6", &background).unwrap();
        let setup = &b.stage.player.battle().setup;
        assert_eq!((setup.settings.stage, setup.settings.background, b.stage.player.ticks()), (stage, background, 0));
        assert!(setup.later_stages.iter().all(|s| (s.stage, s.background) == (stage, background)), "every round there");
        made(&mut b);
        let mut frames = 0;
        while b.stage.player.battle().round.mode != mode::CUSTOM {
            frame(&mut b, nettai_battle::input::keys::A);
            assert!((548..=549).contains(&made(&mut b)), "frame {frames}: the sound goes on");
            frames += 1;
            assert!(frames < 1000, "the custom screen never came");
        }
        for _ in 0..60 {
            frame(&mut b, nettai_battle::input::keys::A);
            let battle = b.stage.player.battle();
            assert!(battle.round.mode == mode::CUSTOM && battle.paused_by().is_none(), "it waits on the custom screen");
        }
    }

    /// A practice set put on another background (Practice's Background row changed: `Player::set_backgrounds`)
    /// goes on where it is: no start over, the arena the same, the battle drawn on the new background; and
    /// since no tick reads a round's background, it steps as the battle without the change does, its digest
    /// the same but for the background its setup says (and the rounds after on it too).
    #[test]
    fn on_another_background_the_battle_goes_on() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 11, None).unwrap();
        let opponent = nettai_frontend::driver::Opponent { in_fight: nettai_frontend::driver::InFight::AttacksAtRandom, ..Default::default() };
        let (mut a, mut b) = (practice(&content, &m, 11, opponent), practice(&content, &m, 11, opponent));
        let keys = |tick: u64| if tick % 37 < 3 { nettai_battle::input::keys::A } else if tick % 11 == 0 { nettai_battle::input::keys::B } else { 0 };
        let step = |x: &mut Battle| {
            let t = x.stage.player.ticks();
            x.stage.player.tick(keys(t));
        };
        for _ in 0..900 {
            step(&mut a);
            step(&mut b);
        }
        let (was, ticks) = (b.stage.player.battle().setup.clone(), b.stage.player.ticks());
        // The set on another background: the last of the game's, named by every round.
        let background = nettai_match::ids::backgrounds(&content, "exe6").last().unwrap().to_string();
        let mut other = m.clone();
        other.rounds.iter_mut().for_each(|r| r.background = Some(background.clone()));
        let mut first = other.round(&content, 11);
        first.host_keys.pause = false;
        let folders = [0, 1].map(|side| other.sides[side].folder(&content).saved().unwrap());
        assert!(b.stage.player.set_backgrounds(Set::new(content.clone(), first, folders)));
        let id = nettai_match::ids::background(&content, "exe6", &background).unwrap();
        let now = &b.stage.player.battle().setup;
        assert_ne!(was.settings.background, id, "(another background before)");
        assert_eq!((now.settings.stage, now.settings.background, b.stage.player.ticks()), (was.settings.stage, id, ticks), "no start over, the arena the same");
        assert!(now.later_stages.iter().zip(&was.later_stages).all(|(n, w)| n.stage == w.stage && n.background == id), "the rounds after on it");
        for _ in 0..1200 {
            step(&mut a);
            step(&mut b);
        }
        let mut seen = b.stage.player.battle().clone();
        assert_ne!(seen.digest(), a.stage.player.battle().digest(), "(the setup says the new background)");
        seen.setup.settings.background = a.stage.player.battle().setup.settings.background;
        seen.setup.later_stages.clone_from(&a.stage.player.battle().setup.later_stages);
        assert_eq!(seen.digest(), a.stage.player.battle().digest(), "stepped as the battle without the change");
    }

    /// A practice battle of `m`'s (its rounds leaving the game its START), with sound, the dummy as given.
    fn practice(content: &std::sync::Arc<nettai_battle::Content>, m: &nettai_match::Match, seed: u32, opponent: nettai_frontend::driver::Opponent) -> Battle {
        let mut first = m.round(content, seed);
        first.host_keys.pause = false;
        let folders = [0, 1].map(|side| m.sides[side].folder(content).saved().unwrap());
        let driver = LivePlayer::against(Set::new(content.clone(), first, folders), opponent, seed as u64);
        let audio = nettai_frontend::testing::audio(content);
        let player = Player::with(graphics(content, 0x7FFF).renderer(TextMode::Original, None), None, Some(audio), Box::new(driver));
        let mut b = Battle { stage: Stage::new(player), owner: ScreenId::Practice, kind: BattleKind::Practice, game: content.game().into(), full: true, viewer: 0, menu: false, over: false, game_pause: None };
        b.pause_in_game();
        b
    }

    /// The dummy that fights, plugged in as Practice plugs it (its Action Fights, its custom screen the AI's):
    /// against a player who only gets through their custom screen, it takes HP away, the battle stepped as the
    /// shell steps a practice battle.
    #[test]
    fn the_dummy_that_fights_takes_hp_away() {
        use nettai_battle::battle::mode;
        use nettai_frontend::driver::{InFight, OnCustomScreen, Opponent, bot_buttons};
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 11, None).unwrap();
        let opponent = Opponent { in_fight: InFight::Ai, on_custom_screen: OnCustomScreen::Ai };
        let mut b = practice(&content, &m, 11, opponent);
        let live = b.stage.player.live_mut().expect("live play");
        assert_eq!(crate::screens::practice::plug(live, opponent, 2, 11, None), Some(2));
        let hp = |b: &Battle| {
            let battle = b.stage.player.battle();
            battle.player(0).map(|r| (battle.objects.get(r).hp, battle.objects.get(r).max_hp))
        };
        for _ in 0..1800 {
            let battle = b.stage.player.battle();
            let keys = if battle.round.mode == mode::CUSTOM { bot_buttons(battle, 0, b.stage.player.ticks() as u32) } else { 0 };
            frame(&mut b, keys);
            if hp(&b).is_some_and(|(hp, max)| hp < max) {
                return;
            }
        }
        panic!("in 30 seconds the dummy took no HP away: {:?}", hp(&b));
    }

    /// The samples a frame made.
    fn made(b: &mut Battle) -> usize {
        let mut samples = Vec::new();
        b.stage.player.take_samples(&mut samples);
        samples.len()
    }

    /// The menu opened where START wouldn't pause the fight: the battle steps on with no buttons, its sound
    /// coming every frame, until START would, and then into the game's pause (a dimming); or onto the next
    /// round's custom screen, where it waits (a round's end).
    #[test]
    fn opened_where_the_game_wouldnt_pause_it_steps_on() {
        use nettai_battle::battle::{fight, mode};
        use nettai_frontend::driver::{InFight, OnCustomScreen, Opponent, bot_buttons};
        let content = nettai_match::testing::exe6_content();
        let opponent = Opponent { in_fight: InFight::AttacksAtRandom, on_custom_screen: OnCustomScreen::PicksFirst };
        // A dimming: play until one begins (the player shooting and using a chip now and then, the dummy at
        // random).
        let m = nettai_match::pick::live(&content, "exe6", 11, None).unwrap();
        let mut b = practice(&content, &m, 11, opponent);
        let pressed = |tick: u32| match tick % 41 {
            0 | 1 => nettai_battle::input::keys::A,
            n if n % 9 < 2 => nettai_battle::input::keys::B,
            _ => 0,
        };
        let dimmed = |b: &Battle| {
            let battle = b.stage.player.battle();
            battle.round.mode == mode::FIGHTING && battle.fight.state == fight::FIGHTING && battle.is_dimmed()
        };
        while !dimmed(&b) {
            let battle = b.stage.player.battle();
            let tick = b.stage.player.ticks() as u32;
            let keys = if battle.round.mode == mode::CUSTOM { bot_buttons(battle, 0, tick) } else { pressed(tick) };
            assert!(!frame(&mut b, keys) && tick < 30_000, "no dimming came");
        }
        b.menu = true;
        made(&mut b);
        let (mut frames, mut waited) = (0, 0);
        while b.stage.player.battle().paused_by().is_none() {
            frame(&mut b, nettai_battle::input::keys::A);
            assert!((548..=549).contains(&made(&mut b)), "frame {frames}: the sound goes on");
            waited += dimmed(&b) as u32;
            frames += 1;
            assert!(frames < 1500, "never paused");
        }
        assert!(waited > 0, "it stepped through the dimming");
        for _ in 0..120 {
            frame(&mut b, 0);
            assert!((548..=549).contains(&made(&mut b)) && b.stage.player.battle().paused_by() == Some(0));
        }
        b.menu = false;
        for _ in 0..3 {
            frame(&mut b, 0);
        }
        assert_eq!(b.stage.player.battle().paused_by(), None, "and out of it as the menu closes");

        // A round's end: a set a shot decides, the menu opened as the round is decided.
        let m = nettai_frontend::testing::short_set::of(&content, "exe6", 7);
        let mut b = practice(&content, &m, 7, Opponent::default());
        let decided = |b: &Battle| {
            let battle = b.stage.player.battle();
            battle.round.mode == mode::FIGHTING && battle.is_battle_over() && !battle.pausable()
        };
        while !decided(&b) {
            let keys = nettai_frontend::testing::short_set::shooter(b.stage.player.battle(), 0, b.stage.player.ticks() as u32);
            assert!(!frame(&mut b, keys) && b.stage.player.ticks() < 5000, "the round wasn't decided");
        }
        b.menu = true;
        made(&mut b);
        let mut frames = 0;
        while b.stage.player.battle().round.mode != mode::CUSTOM {
            frame(&mut b, nettai_battle::input::keys::START);
            assert!((548..=549).contains(&made(&mut b)), "frame {frames}: the sound goes on");
            frames += 1;
            let battle = b.stage.player.battle();
            assert!(frames < 2000, "the next round's custom screen never came: mode {:#x} fight {:#x} round {} top {:#x} ticks {} stopped {:?}", battle.round.mode, battle.fight.state, battle.round.round, battle.round.top, b.stage.player.ticks(), b.stage.player.stopped());
        }
        assert_eq!(b.stage.player.battle().round.round, 2, "the next round's");
        for _ in 0..120 {
            frame(&mut b, nettai_battle::input::keys::START);
            let battle = b.stage.player.battle();
            assert!(battle.round.mode == mode::CUSTOM && battle.paused_by().is_none(), "it waits on the custom screen");
            assert!((548..=549).contains(&made(&mut b)));
        }
    }
}
