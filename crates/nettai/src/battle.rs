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
use nettai_frontend::game::GraphicsMode;
use nettai_frontend::preload::{Part, Preload};
use nettai_frontend::replay::{Recorder, Replay};
use nettai_frontend::viewer::Viewer;
use nettai_match::Set;
use nettai_render::vfont::TextRenderer;
use nettai_ui::{BattleKind, VersusCard};
use slint::ComponentHandle;
use std::time::Duration;
use nettai_platform::time::Instant;

mod hold;
pub(crate) mod link;
pub(crate) mod party;

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

/// The music a battle plays (docs/app.md §1, "The battle's music").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleMusic {
    /// The player's pick: Practice's own sessions (the pick is Practice's alone: the user, "why does the music
    /// setting carry over from practice mode into pvp mode?").
    Picked,
    /// The battle's game's own: a match ("for pvp it should use the game's default for now"), a replay.
    GameOwn,
    /// None of its own: the menus' music goes on under it (the practice battle beside a waiting invite: the user,
    /// "in the practice mode while the lobby is waiting, don't switch the music").
    None,
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
    /// Its music.
    pub music: BattleMusic,
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
    Finished(#[allow(dead_code)] Option<BattleResult>),
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
    /// Its music: with none, the menus' music goes on under it.
    pub music: BattleMusic,
    /// The side whose console is shown.
    pub viewer: u8,
    /// nettai's menu is over it.
    menu: bool,
    /// Something of its owner's is over it, which has the keys (a match's result: [`Shell::cover_battle`]).
    covered: bool,
    /// The owner has heard how it ended.
    over: bool,
    /// Its owner keeps the keys from it while it shows something over it that they move (a match's builds while
    /// Tab is held: [`Shell::keep_keys`]): no button reaches the battle, and the moves are the owner's to scroll.
    kept: bool,
    /// Under the menu it is held where it stands, its sound and its scenery going on, if it can stop (not an
    /// online match): `battle/hold.rs`.
    hold: hold::Hold,
    /// Online: the round trip's recent past, for the link panel (`battle/link.rs`).
    link: link::LinkHistory,
    /// A party battle's party beside it: the switches asked, and what the window shows (`battle/party.rs`).
    party: party::PartyState,
}

impl Battle {
    /// Whether the battle has the keys: it runs, and nothing is over it.
    pub fn playing(&self) -> bool {
        !self.menu && !self.over && !self.covered
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
        self.hold.reset(&mut self.stage.player);
        self.over = false;
        self.stage.hold();
        self.stage.surface.stale = true;
        true
    }
}

/// A battle being made ready.
pub struct Loading {
    pub(crate) start: BattleStart,
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
                let mode = self.settings.graphics();
                let graphics = ready.graphics(self.i18n.code);
                let font = ready.loaded.font.clone().filter(|_| mode == GraphicsMode::Enhanced);
                let mut renderer = graphics.renderer(mode, font.clone());
                renderer.reduce_motion = self.reduces_motion();
                // (The music the player picked, the battle's game's own or a theme, where the battle plays the pick:
                // `crate::music`.)
                let music = if loading.start.music == BattleMusic::Picked { self.battle_music() } else { crate::music::Music::GameOwn };
                let sound = crate::music::sound(&self.games, &ready, &music);
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
                self.jack_in();
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
        stage.player.set_sound_levels(if start.music == BattleMusic::None { 0.0 } else { music }, effects);
        stage.hold();
        shell.set_versus(false);
        if start.cards.is_some() {
            self.jack_in();
        }
        shell.set_battle(start.kind);
        shell.set_battle_full(start.full);
        shell.set_battle_menu(false);
        shell.set_connection_shown(false);
        shell.set_connection_open(false);
        self.keys.pressed.clear();
        self.battle = Some(Battle { stage, owner: start.owner, kind: start.kind, game: start.game, full: start.full, music: start.music, viewer: start.viewer, menu: false, covered: false, over: false, kept: false, hold: Default::default(), link: Default::default(), party: Default::default() });
        Some((start.owner, BattleEvent::Started))
    }

    /// A battle that had its cards begins: it jacks in (docs/design/app-ui.md §4, "Motion"), the set piece drawn
    /// over its first frames, the field rushing again, and the game's own encounter sound. The battle starts
    /// when it would have all the same: the set piece is the UI's alone, and nothing waits on it.
    fn jack_in(&mut self) {
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        shell.set_jack_in_seq(shell.get_jack_in_seq() + 1);
        shell.set_rush_seq(shell.get_rush_seq() + 1);
        self.sound.play(crate::sound::Cue::JackIn);
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

    /// The battle's owner keeps the keys from it, or gives them back ([`Battle::kept`]).
    pub fn keep_keys(&mut self, kept: bool) {
        if let Some(b) = &mut self.battle {
            b.kept = kept;
        }
    }

    /// Whether the battle's owner keeps the keys from it now (its moves are the owner's to scroll).
    pub fn keys_kept(&self) -> bool {
        self.battle.as_ref().is_some_and(|b| b.kept)
    }

    /// Whether the battle has the keys now.
    pub fn battle_has_keys(&self) -> bool {
        self.battle.as_ref().is_some_and(Battle::playing) && self.dialog.is_none()
    }

    /// Open nettai's menu over the battle. A battle that can stop (practice) is held under it from the next
    /// frame, its sound and its scenery going on (`battle/hold.rs`); an online match keeps running.
    pub fn open_menu(&mut self) {
        let Some(b) = &mut self.battle else { return };
        if b.menu || b.over {
            return;
        }
        b.menu = true;
        self.keys.release();
        self.nav.open("menu.", Key::new("menu.resume", 0));
        self.ui().global::<nettai_ui::Shell>().set_battle_menu(true);
    }

    /// Put something of the battle's owner's over it, or take it away (a match's result, its controls' names
    /// beginning `scope`, `focus` focused): while it is there the keys are its own, not the battle's.
    pub fn cover_battle(&mut self, scope: &str, focus: Option<Key>) {
        let Some(b) = &mut self.battle else { return };
        b.covered = focus.is_some();
        self.keys.release();
        match focus {
            Some(focus) => self.nav.open(scope, focus),
            None => self.nav.close(scope),
        }
    }

    /// Close the menu: the battle has the keys again.
    pub fn close_menu(&mut self) {
        let Some(b) = &mut self.battle else { return };
        if !b.menu {
            return;
        }
        b.menu = false;
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
        shell.set_connection_open(false);
        shell.set_party(Default::default());
        self.keys.release();
    }

    /// A frame of the battle: the ticks due with the buttons held, its picture, its samples (into `samples`).
    /// The ticks that ran, for the menus' sound to keep time with (a battle held under the menu runs its ticks
    /// for its sound and its scenery); and what its owner should hear.
    pub(crate) fn run_battle(&mut self, now: Instant, samples: &mut Vec<[f32; 2]>) -> (Option<u32>, Option<(ScreenId, BattleEvent)>) {
        let Some(mut b) = self.battle.take() else { return (None, None) };
        b.hold.before(b.menu, &mut b.stage.player);
        let held = if b.playing() && !b.kept && self.dialog.is_none() { self.keys.held | self.pad.held(&self.bindings) | self.touch.held() } else { 0 };
        // (A party battle's switches are asked of the player; the battle's buttons alone go on.)
        let buttons = b.party.take_switches(&mut b.stage.player, b.viewer, held);
        let ran = b.stage.advance(now, buttons, &mut self.keys.pressed);
        let ticks = Some(ran);
        // START in the fight: the game's pause is nettai's menu (asked of the host: the game's own never runs).
        let asked = b.stage.player.take_pause_requests()[b.viewer.min(1) as usize] && b.playing();
        std::mem::swap(samples, &mut b.stage.samples);
        b.stage.samples.clear();
        self.show(&mut b.stage);
        b.stage.report(now);
        if let Some(s) = b.stage.stats.as_mut() {
            s.audio_queued = s.audio_queued.max(self.sound.queued());
        }
        // The connection's corner, and the party beside the battle.
        let ui = self.ui();
        let shell = ui.global::<nettai_ui::Shell>();
        let kit = self.games.ready(&b.game).and_then(|ready| crate::builds::kit_of(&b.game, &ready));
        let keys = [crate::input::SWITCH_LEFT, crate::input::SWITCH_RIGHT].map(|bit| nettai_ui::Hint {
            key: crate::input::key_cap(self.bindings.key(bit)).into(),
            pad: crate::input::pad_face(self.bindings.pad_button(bit)),
            ..Default::default()
        });
        let faces = |n| kit.as_ref().map(|k| k.pictures.navi(n)).unwrap_or_default();
        b.party.show(&shell, b.stage.player.battle(), &b.game, b.viewer, faces, keys);
        if let Some(net) = b.stage.player.net_status() {
            // (The bars are the library's quality, from the round trip and how steady it is; while the link
            // is down, the seconds until it gives up, or those it has been down where it never does.)
            shell.set_connection_shown(true);
            shell.set_connection_level(net.quality.map_or(0, |q| q.bars() as i32));
            shell.set_connection_ping(net.ping_ms.map_or(-1, |ms| ms.round() as i32));
            shell.set_connection_down(net.reconnecting.map_or(-1, |r| r.gives_up_in.unwrap_or(r.elapsed).as_secs() as i32));
            // The link panel's figures, while it is open, at each sample of the round trip.
            if b.link.sample(now, &net) && shell.get_connection_open() {
                shell.set_link(b.link.figures(&net));
            }
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
        let mut fit = stage::fit(width, height, self.scale.os, self.sharp, self.settings.enhanced);
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

    /// A second, small picture over the window's one (the replay viewer's other console): `multiple` times the
    /// frame in the display's pixels, presented at the window's density as the picture is (`stage::fit`), into
    /// a texture of its own (`inset_gl`; else an image made anew), when `surface` is out of date or its size
    /// changed. Its image when there is a new one to show, and its size in the window's logical pixels; none in
    /// a frame the window doesn't draw.
    pub fn show_inset(&mut self, surface: &mut Surface, multiple: u32, present: impl FnOnce(&mut [u32], usize, usize)) -> Option<(Option<slint::Image>, f32, f32)> {
        if !self.drawing {
            return None;
        }
        let factor = self.ui().window().scale_factor();
        let (width, height) = ((nettai_render::compose::WIDTH as u32 * multiple) as f32, (nettai_render::compose::HEIGHT as u32 * multiple) as f32);
        let fit = stage::fit(width, height, self.scale.os, self.sharp, self.settings.enhanced);
        let image = match surface.picture(fit, self.inset_gl.as_mut(), present) {
            stage::Shown::New(image) => Some(image),
            stage::Shown::Same | stage::Shown::Written => None,
        };
        Some((image, width / factor, height / factor))
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
    #[allow(dead_code)]
    pub fn recorder(&self, content: &std::sync::Arc<nettai_battle::Content>, m: &nettai_match::Match, seed: u32, game: &str, side: u8, names: [String; 2]) -> Option<Recorder> {
        use nettai_frontend::replay::Info;
        let dir = crate::paths::replays();
        let made = (|| -> Result<Recorder, String> {
            nettai_platform::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let when = nettai_platform::time::SystemTime::now().duration_since(nettai_platform::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let stamp = chrono::DateTime::from_timestamp(when as i64, 0).map(|t| t.with_timezone(&chrono::Local).format("%Y%m%d-%H%M%S").to_string()).unwrap_or_default();
            let path = dir.join(format!("{stamp}-{game}.{}", nettai_frontend::replay::EXTENSION));
            let file = nettai_platform::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
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
        let player = Player::with(graphics(&content, 0x7FFF).renderer(GraphicsMode::Original, None), None, None, Box::new(driver));
        let mut b = Battle { stage: Stage::new(player), owner: ScreenId::Practice, kind: BattleKind::Practice, game: "exe6".into(), full: true, music: BattleMusic::Picked, viewer: 0, menu: false, covered: false, over: false, kept: false, hold: Default::default(), link: Default::default(), party: Default::default() };
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

    /// A frame of the shell's, as `Shell::run_battle` runs one, a tick in it: the player's `keys` (none under
    /// the menu). Whether the menu is to open (the player's START in the fight, asked of the host).
    fn frame(b: &mut Battle, keys: u16) -> bool {
        b.hold.before(b.menu, &mut b.stage.player);
        let buttons = if b.playing() { keys } else { 0 };
        b.stage.player.tick(buttons);
        b.stage.player.take_pause_requests()[b.viewer as usize] && b.playing()
    }

    /// Whether the fight runs, START there the menu's.
    fn fighting(b: &Battle) -> bool {
        use nettai_battle::battle::{fight, mode};
        let battle = b.stage.player.battle();
        battle.round.mode == mode::FIGHTING && battle.fight.state == fight::FIGHTING && !battle.is_battle_over() && !battle.is_dimmed()
    }

    /// The clock its stage's scenery is drawn on.
    fn scenery(b: &Battle) -> nettai_render::stage::StageClock {
        b.stage.player.stage_clock()
    }

    /// The menu open over `b` for `frames` frames, the player's `keys` going to the menu: the battle stands
    /// where it is (its ticks and its digest the same: no tick, no button and no cue reaches it, and the game's
    /// own pause never runs), its sound comes every frame, and its scenery moves on a frame each, the
    /// background drawn anew. The ticks it is held at.
    fn hold_for(b: &mut Battle, frames: u32, keys: u16) -> u64 {
        b.menu = true;
        made(b);
        let (ticks, digest, clock) = (b.stage.player.ticks(), b.stage.player.battle().digest(), scenery(b));
        let running = (b.stage.player.battle().round.top == nettai_battle::battle::top::RUNNING) as u32;
        let (mut last, mut drawn_anew) = (b.stage.player.frame().pixels, 0);
        for n in 1..=frames {
            frame(b, keys);
            assert!((548..=549).contains(&made(b)), "frame {n}: the sound goes on");
            assert_eq!((b.stage.player.ticks(), b.stage.player.battle().digest()), (ticks, digest), "frame {n}: held");
            let moved = nettai_render::stage::StageClock { background: n, field: n * running };
            assert_eq!(scenery(b), clock.ahead(moved), "frame {n}: the scenery a frame on");
            if n <= 60 {
                let now = b.stage.player.frame().pixels;
                drawn_anew += (now != last) as u32;
                last = now;
            }
        }
        assert!(b.stage.player.battle().paused_by().is_none(), "the game's own pause never runs");
        assert!(drawn_anew >= frames.min(60) / 3, "the background drawn anew ({drawn_anew} of {} frames, mode {:#x})", frames.min(60), b.stage.player.battle().round.mode);
        ticks
    }

    /// RESUME after [`hold_for`]: the battle goes on from the tick it was held at, and its background a frame
    /// on from where the hold left it, no jump back.
    fn resume(b: &mut Battle) {
        b.menu = false;
        let (ticks, clock) = (b.stage.player.ticks(), scenery(b));
        frame(b, 0);
        assert_eq!(b.stage.player.ticks(), ticks + 1, "on from the tick it was held at");
        assert_eq!(scenery(b).background, clock.background + 1, "the background on from where the hold left it");
    }

    /// The practice menu holds its battle wherever it opens. On the custom screen: no tick steps it, nothing
    /// is chosen (neither side's), its sound goes on and its scenery moves. Mid-fight the same: the player's
    /// own START (the host's, the game's own pause never running) asks for the menu, and under it nothing of the
    /// fight moves while the sound and the scenery go on; RESUME goes on where it was. Starting over leaves
    /// nothing held.
    #[test]
    fn a_battle_held_under_the_menu_on_the_custom_screen_and_mid_fight() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 11, None).unwrap();
        let opponent = nettai_frontend::driver::Opponent { in_fight: nettai_frontend::driver::InFight::AttacksAtRandom, ..Default::default() };
        use nettai_battle::battle::mode;
        let mut b = practice(&content, &m, 11, opponent);
        // Onto the custom screen, a chip picked; the menu opened there: held, both screens' cursors and choices
        // as they were (the dummy's presses held too).
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
        // (The player's buttons are the menu's.)
        hold_for(&mut b, 240, nettai_battle::input::keys::A | nettai_battle::input::keys::START);
        assert!(choosing(&b), "held on the custom screen");
        assert_eq!(chosen(&b), before, "nothing chosen, neither side");
        resume(&mut b);
        assert!(!b.stage.player.held());
        while !fighting(&b) {
            let keys = custom(&b);
            assert!(!frame(&mut b, keys) && b.stage.player.ticks() < 3000);
        }
        for _ in 0..90 {
            let keys = nettai_battle::input::keys::B * (b.stage.player.ticks() % 2 == 0) as u16;
            frame(&mut b, keys);
        }
        // (A dimming chip the dummy used is let run out: START is asked for in the fight itself.)
        while !fighting(&b) {
            assert!(!frame(&mut b, 0) && b.stage.player.ticks() < 3000);
        }
        // The player's own START asks for the menu: the host's, nothing of the game's paused.
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
        let asked = (0..3).map(|_| frame(&mut b, nettai_battle::input::keys::START)).any(|asked| asked);
        assert!(asked && b.stage.player.battle().paused_by().is_none() && fighting(&b));
        let held = fight(&b);
        hold_for(&mut b, 300, nettai_battle::input::keys::A);
        assert_eq!(fight(&b), held, "nothing of the fight moved");
        // RESUME: the fight goes on from where it was.
        resume(&mut b);
        for _ in 0..60 {
            frame(&mut b, 0);
        }
        assert_ne!(fight(&b), held, "the fight runs again");
        // Starting over from the menu: from its start, nothing held.
        b.menu = true;
        frame(&mut b, 0);
        assert!(b.stage.player.held());
        b.menu = false;
        assert!(b.restart());
        assert_eq!((b.stage.player.held(), b.stage.player.ticks()), (false, 0));
        assert!(!frame(&mut b, 0) && b.stage.player.ticks() == 1);
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
        let first = m.round(&content, 7);
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
        held_from_its_custom_screen(&mut b);
    }

    /// Under the menu, a practice battle in its round's opening steps on to its custom screen, and is held as
    /// the screen begins: for ten seconds of frames no tick steps it (its ticks and its digest the same), while
    /// its sound comes every frame and its scenery moves. Then the menu closes, and it goes on from the tick it
    /// was held at: the screen comes in. The ticks it was held at, and the battle as it was held.
    fn held_from_its_custom_screen(b: &mut Battle) -> (u64, nettai_battle::battle::Battle) {
        use nettai_battle::battle::mode;
        let up = |b: &Battle| b.stage.player.battle().round.mode == mode::CUSTOM;
        let mut frames = 0;
        while !up(b) {
            frame(b, nettai_battle::input::keys::A);
            frames += 1;
            assert!(frames < 1000, "the custom screen never came");
        }
        // (The frame it began on, and the next, which finds it begun.)
        frame(b, 0);
        let screen = |b: &Battle| b.stage.player.battle().custom.sides[0].screen.as_ref().map(|s| s.phase.chooses());
        assert_ne!(screen(b), Some(true), "held before the screen is in");
        let was = b.stage.player.battle().clone();
        // (The player's buttons are the menu's.)
        let ticks = hold_for(b, 600, nettai_battle::input::keys::A | nettai_battle::input::keys::START);
        assert!(up(b), "on the custom screen");
        // RESUME: on from there, the screen coming in.
        resume(b);
        for _ in 0..30 {
            frame(b, 0);
        }
        assert_eq!(screen(b), Some(true), "the screen in");
        (ticks, was)
    }

    /// A practice set of `m`'s whose first round `edit` changes (a NaviCust bug given), with sound.
    fn practice_edited(content: &std::sync::Arc<nettai_battle::Content>, m: &nettai_match::Match, seed: u32, edit: impl FnOnce(&mut nettai_battle::setup::RoundSetup)) -> Battle {
        let mut first = m.round(content, seed);
        edit(&mut first);
        let folders = [0, 1].map(|side| m.sides[side].folder(content).saved().unwrap());
        let driver = LivePlayer::against(Set::new(content.clone(), first, folders), Default::default(), seed as u64);
        battle_of(content, driver)
    }

    /// A practice battle of what `driver` plays, as the shell has one, with sound, drawn on a background that
    /// scrolls (`testing::scrolling_background`).
    fn battle_of(content: &std::sync::Arc<nettai_battle::Content>, driver: LivePlayer) -> Battle {
        let mut graphics = graphics(content, 0x7FFF);
        nettai_frontend::testing::scrolling_background(&mut graphics);
        let audio = nettai_frontend::testing::audio(content);
        let player = Player::with(graphics.renderer(GraphicsMode::Original, None), None, Some(audio), Box::new(driver));
        Battle { stage: Stage::new(player), owner: ScreenId::Practice, kind: BattleKind::Practice, game: content.game().into(), full: true, music: BattleMusic::Picked, viewer: 0, menu: false, covered: false, over: false, kept: false, hold: Default::default(), link: Default::default(), party: Default::default() }
    }

    /// A practice session of Practice's own, as the editor's PRACTICE starts one: the menu open before the battle's
    /// first tick (`BattleEvent::Started`). The battle steps through its round's opening (at its first tick the
    /// field isn't drawn yet and no music plays) and is held from its custom screen: no tick steps it while the
    /// menu is open, so the custom screen's HP drain (the user: "pause at start doesn't seem to stop custom hp
    /// drain") takes nothing; RESUME goes on from there, and the drain drains again. The same after EDIT's
    /// return (a session started again, the menu open as it starts) and a Dummy's build changed under the menu
    /// (the set started over: `under_the_menu_a_set_starts_over_on_another_place` for an arena).
    #[test]
    fn a_practice_opened_with_its_menu_is_held_from_its_custom_screen() {
        let content = nettai_match::testing::exe6_content();
        let m = nettai_match::pick::live(&content, "exe6", 11, None).unwrap();
        // (The custom screen's HP bug at its quickest: a point every 2 ticks.)
        let mut b = practice_edited(&content, &m, 11, |first| first.navi_stats[0].bugs.custom_drain = 7);
        let hp = |b: &Battle| {
            let battle = b.stage.player.battle();
            battle.player(0).map(|r| battle.objects.get(r).hp)
        };
        b.menu = true;
        assert_eq!(b.stage.player.ticks(), 0);
        let (ticks, was) = held_from_its_custom_screen(&mut b);
        let full = was.player(0).map(|r| (was.objects.get(r).hp, was.objects.get(r).max_hp));
        assert!(ticks > 0 && full.is_some_and(|(hp, max)| hp == max), "(it stepped through the opening, and nothing drained there: {full:?})");
        assert!(hp(&b) < full.map(|(hp, _)| hp), "the drain drains once the menu closes: {:?} of {full:?}", hp(&b));
        // Started over under the menu (a Dummy's build changed): the same again.
        b.menu = true;
        frame(&mut b, 0);
        assert!(b.restart());
        assert_eq!(held_from_its_custom_screen(&mut b).0, ticks, "the same opening, held at the same tick");
    }

    /// Mid-fight, START opens the menu and the NaviCust's HP bug takes nothing under it, in EXE6 and in EXE4
    /// alike (EXE4's own pause drains it: the game's pause no longer holds a practice battle).
    #[test]
    fn under_the_menu_mid_fight_the_hp_bug_waits() {
        use nettai_frontend::driver::bot_buttons;
        for content in [nettai_match::testing::exe6_content(), nettai_match::testing::exe4_content()] {
            let game = content.game().to_string();
            let m = nettai_match::pick::live(&content, &game, 11, None).unwrap();
            let mut b = practice_edited(&content, &m, 11, |first| first.navi_stats[0].bugs.hp_drain = 7);
            let hp = |b: &Battle| {
                let battle = b.stage.player.battle();
                battle.player(0).map(|r| battle.objects.get(r).hp)
            };
            while !fighting(&b) {
                let keys = bot_buttons(b.stage.player.battle(), 0, b.stage.player.ticks() as u32);
                frame(&mut b, keys);
                assert!(b.stage.player.ticks() < 3000, "{game}");
            }
            let fought = hp(&b);
            for _ in 0..60 {
                frame(&mut b, 0);
            }
            assert!(hp(&b) < fought, "{game}: (it drains in the fight)");
            // The player's own START: the menu.
            let asked = (0..3).map(|_| frame(&mut b, nettai_battle::input::keys::START)).any(|asked| asked);
            assert!(asked, "{game}: START asks for the menu");
            let held = hp(&b);
            hold_for(&mut b, 600, nettai_battle::input::keys::A);
            assert_eq!(hp(&b), held, "{game}: nothing drained under the menu");
            resume(&mut b);
            for _ in 0..120 {
                frame(&mut b, 0);
            }
            assert!(hp(&b) < held, "{game}: and drains again after");
        }
    }

    /// A practice set put on another background (Practice's Background row changed: `Player::set_backgrounds`)
    /// goes on where it is: no start over, the arena the same, the battle drawn on the new background; and
    /// since no tick reads a round's background, it steps as the battle without the change does, its digest
    /// the same but for the background its setup says (and the rounds after on it too). The same with another
    /// game's background (the background pool's).
    #[test]
    fn on_another_background_the_battle_goes_on() {
        let content = nettai_match::testing::exe6_content();
        // (The last of the game's, and one of another game's.)
        let own = nettai_match::ids::backgrounds(&content, "exe6").last().unwrap().to_string();
        for background in [own, "exe4:acdc-area".to_string()] {
            on_another_background(&content, &background);
        }
    }

    fn on_another_background(content: &std::sync::Arc<nettai_battle::Content>, background: &str) {
        let content = content.clone();
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
        // The set on another background, named by every round.
        let mut other = m.clone();
        other.rounds.iter_mut().for_each(|r| r.background = Some(background.to_string()));
        let first = other.round(&content, 11);
        let folders = [0, 1].map(|side| other.sides[side].folder(&content).saved().unwrap());
        assert!(b.stage.player.set_backgrounds(Set::new(content.clone(), first, folders)));
        let id = nettai_match::ids::background(&content, "exe6", background).unwrap();
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

    /// A practice battle of `m`'s (its START the host's, as Practice's set has it), with sound, the dummy as
    /// given.
    fn practice(content: &std::sync::Arc<nettai_battle::Content>, m: &nettai_match::Match, seed: u32, opponent: nettai_frontend::driver::Opponent) -> Battle {
        let first = m.round(content, seed);
        let folders = [0, 1].map(|side| m.sides[side].folder(content).saved().unwrap());
        battle_of(content, LivePlayer::against(Set::new(content.clone(), first, folders), opponent, seed as u64))
    }

    /// The samples a frame made.
    fn made(b: &mut Battle) -> usize {
        let mut samples = Vec::new();
        b.stage.player.take_samples(&mut samples);
        samples.len()
    }

    /// The menu opened in a dimming: the battle is held where it stands as anywhere, nothing of it changing
    /// while its sound comes every frame and its scenery moves, and goes on from there as the menu closes; the
    /// same at a round's end, after which the next round's custom screen comes.
    #[test]
    fn held_in_a_dimming_and_at_a_rounds_end() {
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
        hold_for(&mut b, 300, nettai_battle::input::keys::A);
        assert!(dimmed(&b), "held in the dimming");
        resume(&mut b);
        let ticks = b.stage.player.ticks();
        let mut frames = 0;
        while dimmed(&b) {
            frame(&mut b, 0);
            frames += 1;
            assert!(frames < 1500, "the dimming never ended");
        }
        assert_eq!(b.stage.player.ticks(), ticks + frames, "on from where it was held");

        // A round's end: a set a shot decides, the menu opened as the round is decided.
        let m = nettai_frontend::testing::short_set::of(&content, "exe6", 7);
        let mut b = practice(&content, &m, 7, Opponent::default());
        let decided = |b: &Battle| {
            let battle = b.stage.player.battle();
            battle.round.mode == mode::FIGHTING && battle.is_battle_over()
        };
        while !decided(&b) {
            let keys = nettai_frontend::testing::short_set::shooter(b.stage.player.battle(), 0, b.stage.player.ticks() as u32);
            assert!(!frame(&mut b, keys) && b.stage.player.ticks() < 5000, "the round wasn't decided");
        }
        hold_for(&mut b, 300, nettai_battle::input::keys::START);
        resume(&mut b);
        let mut frames = 0;
        while b.stage.player.battle().round.mode != mode::CUSTOM {
            frame(&mut b, 0);
            frames += 1;
            let battle = b.stage.player.battle();
            assert!(frames < 2000, "the next round's custom screen never came: mode {:#x} fight {:#x} round {} top {:#x} ticks {} stopped {:?}", battle.round.mode, battle.fight.state, battle.round.round, battle.round.top, b.stage.player.ticks(), b.stage.player.stopped());
        }
        assert_eq!(b.stage.player.battle().round.round, 2, "the next round's");
    }
}
