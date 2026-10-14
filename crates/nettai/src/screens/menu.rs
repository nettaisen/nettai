//! The main menu (docs/design/app-ui.md §6): PLAY, REPLAYS, SETTINGS, EXIT, About, and the demo reel.
//!
//! The reel is a battle of the selected game playing itself: the player's latest replays of the game, one after
//! another (`crate::replays::Reel`: found and read on a thread as the menu shows; one that doesn't play here
//! is passed over); with none, a set between new builds of the game's first two versions
//! (`nettai_match::build`: each with the folder a new save starts with), each side pressing by itself. It is
//! silent (the title music plays over the menu) and takes no keys.

use crate::games::Ready;
use crate::screens::{Screen, ScreenId};
use crate::shell::Shell;
use crate::stage::Stage;
use nettai_frontend::driver::{Driver, InFight, LivePlayer, OnCustomScreen, Opponent};
use nettai_frontend::game::TextMode;
use nettai_frontend::player::Player;
use nettai_frontend::replay::ReplayPlayer;
use nettai_render::vfont::TextRenderer;
use nettai_ui::MenuView;
use slint::ComponentHandle;
use std::rc::Rc;
use std::time::Instant;

/// The tick the screenshot pass holds the reel at: in the first turn's fight.
const STILL_AT: u32 = 700;

/// What the reel shows.
struct Reel {
    stage: Stage,
    game: String,
    /// The ticks played, for the buttons the left side presses by itself.
    ticks: u32,
    /// Its left side is pressed by the reel (a set made for it); a replay's sides are its recording's.
    pressed: bool,
}

#[derive(Default)]
pub struct Menu {
    reel: Option<Reel>,
    /// Where the reel's replays come from: the game they are of, and its latest.
    source: Option<(String, crate::replays::Reel)>,
    /// The reel's battles, counted for the UI.
    reels: i32,
    /// After a change of game, the next reel isn't made until then: making it (its match, its battle, its player)
    /// is 25 to 35 ms of the window's thread, which in the change's first frame held the sweep of the new game's
    /// ground (docs/app.md §10). It fades in once the sweep has crossed.
    hold: Option<Instant>,
    /// The game the menu was last shown with, while it is shown.
    shown_game: Option<String>,
}

/// How long after the sweep a change of game's reel is held for (the sweep is the theme's `settle`).
const HOLD_PAST_SWEEP: std::time::Duration = std::time::Duration::from_millis(60);

impl Screen for Menu {
    fn main(&self) -> &'static str {
        "play"
    }

    fn show(&mut self, shell: &mut Shell) {
        // Another game: its reel, once its ground has swept in (with motion off, at once). A change while the menu
        // is shown holds it from that change, the second of two from the second.
        if self.reel.as_ref().is_some_and(|r| Some(r.game.as_str()) != shell.game()) {
            self.reel = None;
        }
        if self.shown_game.is_some() && self.shown_game.as_deref() != shell.game() {
            let sweep = shell.ui().global::<nettai_ui::Theme>().get_settle();
            self.hold = (sweep > 0).then(|| crate::clock::now() + std::time::Duration::from_millis(sweep as u64) + HOLD_PAST_SWEEP);
        }
        self.shown_game = shell.game().map(str::to_string);
        match (&mut self.reel, shell.ready()) {
            // The language may have changed under it: its own drawing.
            (Some(reel), Some((_, ready))) => {
                reel.stage.player.set_language(&ready.graphics(shell.lang));
                reel.stage.surface.stale = true;
                shell.ui().global::<MenuView>().set_playing(true);
            }
            _ => shell.ui().global::<MenuView>().set_playing(false),
        }
    }

    fn hide(&mut self, _shell: &mut Shell) {
        self.reel = None;
        // (Shown again, it makes its reel as it arrives, whatever game was chosen meanwhile.)
        self.shown_game = None;
        self.hold = None;
        // (The replays are found again when the menu next shows: a match played since is among them.)
        self.source = None;
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, _index: i32) {
        match id {
            "play" => shell.go(ScreenId::Play),
            "replays" => shell.go(ScreenId::Replays),
            "settings" => shell.go(ScreenId::Settings),
            "about" => shell.go(ScreenId::About),
            "exit" => shell.quit(),
            _ => {}
        }
    }

    fn frame(&mut self, shell: &mut Shell, now: Instant, shown: bool) {
        if !shown || shell.battle.is_some() || shell.loading_battle() {
            return;
        }
        let Some((game, ready)) = shell.ready() else { return };
        if self.hold.is_some_and(|until| now < until) {
            return;
        }
        self.hold = None;
        // The next battle, when there is none or the last one is over: it fades in.
        if self.reel.as_ref().is_none_or(|r| r.stage.player.stopped().is_some())
            && let Some(next) = self.next_reel(shell, &game, &ready)
        {
            self.reel = Some(next);
            self.reels += 1;
            shell.ui().global::<MenuView>().set_reel_seq(self.reels);
        }
        let Some(reel) = &mut self.reel else { return };
        if shell.still && !shell.reel {
            // The screenshot pass: the reel played to one tick and held there, so two shots of the menu differ
            // only where the menu does.
            while reel.ticks < STILL_AT && reel.stage.player.tick(demo_buttons(reel.stage.player.battle(), reel.ticks)) {
                reel.ticks += 1;
            }
            reel.stage.surface.stale |= reel.ticks <= STILL_AT;
            reel.ticks = STILL_AT + 1;
        } else {
            let buttons = if reel.pressed { demo_buttons(reel.stage.player.battle(), reel.ticks) } else { 0 };
            reel.ticks += reel.stage.advance(now, buttons, &mut Vec::new());
        }
        reel.stage.samples.clear();
        // (START in a reel's fight opens nothing.)
        reel.stage.player.take_pause_requests();
        shell.show(&mut reel.stage);
        // (`NETTAI_PLAY_STATS`: the menu's frames a second, with everything on it moving.)
        reel.stage.report(now);
    }
}

impl Menu {
    /// The reel's next battle: the next of the game's latest replays that plays on this content, else a set
    /// made for it. None while the replays are still being found.
    fn next_reel(&mut self, shell: &mut Shell, game: &str, ready: &Rc<Ready>) -> Option<Reel> {
        use crate::replays::Next;
        if self.source.as_ref().is_none_or(|(of, _)| of != game) {
            // (The screenshot pass plays no replay: its pictures don't depend on what the player has recorded.)
            let source = if shell.still { crate::replays::Reel::empty() } else { crate::replays::Reel::of(game, crate::replays::folder()) };
            self.source = Some((game.to_string(), source));
        }
        let (_, source) = self.source.as_mut().expect("made above");
        let content = ready.content();
        loop {
            let replay = match source.next() {
                Next::Wait => return None,
                Next::Nothing => break,
                Next::Replay(replay) => replay,
            };
            // (One that isn't played here is passed over, and not given again.)
            let Ok(driver) = ReplayPlayer::new(content, &replay, Some(replay.info.side)) else {
                source.skip();
                continue;
            };
            // (The console shown has its side at the left.)
            shell.ui().global::<MenuView>().set_playing(true);
            return Some(Reel { stage: Stage::new(player(shell, ready, Box::new(driver))), game: game.to_string(), ticks: 0, pressed: false });
        }
        // No replay: a new build of each of the game's first two versions, on a picked match's arenas.
        let seed = if shell.still { 7 } else { clock_seed() };
        let m = starter_match(content, game, seed).map_err(|why| crate::errors::log("making the reel's match", &why)).ok()?;
        shell.ui().global::<MenuView>().set_playing(true);
        let set = nettai_match::Set::of(content, &m, seed);
        let opponent = Opponent { in_fight: InFight::AttacksAtRandom, on_custom_screen: OnCustomScreen::PicksFirst };
        let driver = LivePlayer::against(set, opponent, seed as u64);
        Some(Reel { stage: Stage::new(player(shell, ready, Box::new(driver))), game: game.to_string(), ticks: 0, pressed: true })
    }
}

/// A match between new builds of `game`'s first two versions (of its one version twice, where it has one), its
/// arenas picked from `seed`.
pub fn starter_match(content: &std::sync::Arc<nettai_battle::Content>, game: &str, seed: u32) -> Result<nettai_match::Match, String> {
    let builds = nettai_match::build::Builds::of(content, game)?;
    let mut m = nettai_match::pick::live(content, game, seed, None)?;
    for (i, side) in m.sides.iter_mut().enumerate() {
        *side = builds.new_build(content, &builds.versions[i % builds.versions.len()].id)?;
    }
    Ok(m)
}

/// A silent player of `driver`, drawn as the settings say.
pub fn player(shell: &Shell, ready: &Ready, driver: Box<dyn Driver>) -> Player {
    let text = if shell.settings.sharp_text { TextMode::Font } else { TextMode::Original };
    let graphics = ready.graphics(shell.lang);
    let font = ready.loaded.font.clone().filter(|_| text == TextMode::Font);
    Player::with(graphics.renderer(text, font.clone()), font.map(TextRenderer::new), None, driver)
}

/// A seed from the clock.
pub fn clock_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() ^ d.as_secs() as u32).unwrap_or(1)
}

/// What the reel presses for its left side: the custom screen as the library's own opponent presses it; in the
/// fight, the buster, a chip now and then, a step up or down (through every row in turn).
fn demo_buttons(b: &nettai_battle::Battle, tick: u32) -> u16 {
    use nettai_battle::battle::mode;
    use nettai_battle::input::keys;
    if b.round.mode == mode::CUSTOM {
        return nettai_frontend::driver::bot_buttons(b, 0, tick);
    }
    let mut k = 0;
    if tick % 9 < 2 {
        k |= keys::B;
    }
    if tick % 83 == 40 {
        k |= keys::A;
    }
    // (Up, up, down, down: each row a while; a direction is held a few ticks, as it acts on a hold's second.)
    if (45..48).contains(&(tick % 90)) {
        k |= if (tick / 90) % 4 < 2 { keys::UP } else { keys::DOWN };
    }
    k
}
