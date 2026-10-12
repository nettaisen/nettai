//! The main menu (docs/design/app-ui.md §6): PLAY, TRAINING, BUILDS, REPLAYS, Settings, Exit, and the demo reel.
//!
//! The reel is a battle of the selected game playing itself: the player's latest replays of the game, one after
//! another; with none, a set between new builds of the game's first two versions (`nettai_match::build`: each
//! with the folder a new save starts with), each side pressing by itself. It is silent (the title music plays
//! over the menu) and takes no keys.

use crate::games::{Names, Ready};
use crate::screens::{Screen, ScreenId};
use crate::shell::Shell;
use crate::stage::Stage;
use nettai_frontend::driver::{Driver, InFight, LivePlayer, OnCustomScreen, Opponent};
use nettai_frontend::game::TextMode;
use nettai_frontend::player::Player;
use nettai_frontend::replay::{Replay, ReplayPlayer};
use nettai_render::vfont::TextRenderer;
use nettai_ui::MenuView;
use slint::ComponentHandle;
use std::time::Instant;

/// The replays the reel goes through: the latest few.
const REPLAYS: usize = 5;

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
    /// The selected game's replays, newest first, and the one the reel plays next.
    replays: Vec<Replay>,
    replays_of: Option<String>,
    next: usize,
    /// The reel's battles, counted for the UI.
    reels: i32,
}

impl Screen for Menu {
    fn main(&self) -> &'static str {
        "play"
    }

    fn show(&mut self, shell: &mut Shell) {
        // Another game: its reel.
        if self.reel.as_ref().is_some_and(|r| Some(r.game.as_str()) != shell.game()) {
            self.reel = None;
        }
        if self.reel.is_none() {
            shell.ui().global::<MenuView>().set_playing(false);
        }
    }

    fn hide(&mut self, _shell: &mut Shell) {
        self.reel = None;
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, _index: i32) {
        match id {
            "play" => shell.go(ScreenId::Play),
            "training" => shell.go(ScreenId::Training),
            "builds" => shell.go(ScreenId::Builds),
            "replays" => shell.go(ScreenId::Replays),
            "settings" => shell.go(ScreenId::Settings),
            "exit" => shell.quit(),
            _ => {}
        }
    }

    fn frame(&mut self, shell: &mut Shell, now: Instant, shown: bool) {
        if !shown || shell.battle.is_some() || shell.loading_battle() {
            return;
        }
        let Some((game, ready)) = shell.ready() else { return };
        // The next battle, when there is none or the last one is over: it fades in.
        if self.reel.as_ref().is_none_or(|r| r.stage.player.stopped().is_some()) {
            self.reel = self.next_reel(shell, &game, &ready);
            self.reels += 1;
            shell.ui().global::<MenuView>().set_reel_seq(self.reels);
        }
        let Some(reel) = &mut self.reel else { return };
        if shell.still {
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
    }
}

impl Menu {
    /// The reel's next battle: the next of the game's latest replays that plays on this content, else a set
    /// made for it.
    fn next_reel(&mut self, shell: &mut Shell, game: &str, ready: &Ready) -> Option<Reel> {
        if self.replays_of.as_deref() != Some(game) {
            // (The screenshot pass plays no replay: its pictures don't depend on what the player has recorded.)
            let found = if shell.still { Vec::new() } else { crate::replays::scan(&crate::paths::replays()) };
            self.replays = found.into_iter().filter_map(|e| e.replay.ok()).filter(|r| r.head.game == game).take(REPLAYS).collect();
            self.replays_of = Some(game.to_string());
            self.next = 0;
        }
        let content = ready.content();
        for _ in 0..self.replays.len() {
            let replay = &self.replays[self.next % self.replays.len()];
            self.next += 1;
            let (Ok(driver), Ok(m)) = (ReplayPlayer::new(content, replay, Some(replay.info.side)), nettai_match::binary::read_match(content, &replay.match_bytes)) else { continue };
            // (The console shown has its side at the left.)
            let sides = if replay.info.side == 0 { [&m.sides[0], &m.sides[1]] } else { [&m.sides[1], &m.sides[0]] };
            caption(shell, game, ready, sides);
            return Some(Reel { stage: Stage::new(player(shell, ready, Box::new(driver))), game: game.to_string(), ticks: 0, pressed: false });
        }
        // No replay: a new build of each of the game's first two versions, on a picked match's arenas.
        let seed = if shell.still { 7 } else { clock_seed() };
        let m = starter_match(content, game, seed).map_err(|why| crate::errors::log("making the reel's match", &why)).ok()?;
        caption(shell, game, ready, [&m.sides[0], &m.sides[1]]);
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

/// A side's version as a tag says it: its id in the game's builds data, in capitals. (How a version is worded
/// for a player is the Builds screen's to say; until it does, a tag is the id.)
pub fn version_tag(content: &nettai_battle::Content, game: &str, side: &nettai_match::Side) -> String {
    let id = nettai_match::build::Builds::of(content, game).ok().and_then(|b| b.version_of(content, side).map(str::to_string));
    id.unwrap_or_default().replace(['-', '_'], " ").to_uppercase()
}

/// Who is fighting, under the reel: each side's navi and its version's tag.
fn caption(shell: &Shell, game: &str, ready: &Ready, sides: [&nettai_match::Side; 2]) {
    let content = ready.content();
    let graphics = ready.graphics(shell.lang);
    let names = Names::of(content, &graphics);
    let ui = shell.ui();
    let view = ui.global::<MenuView>();
    view.set_left_navi(names.navi(sides[0].navi(content)).into());
    view.set_left_version(version_tag(content, game, sides[0]).into());
    view.set_right_navi(names.navi(sides[1].navi(content)).into());
    view.set_right_version(version_tag(content, game, sides[1]).into());
    view.set_playing(true);
}

/// A silent player of `driver`, drawn as the settings say.
fn player(shell: &Shell, ready: &Ready, driver: Box<dyn Driver>) -> Player {
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
