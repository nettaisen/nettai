//! What drives the battle each tick: a golden trace's recorded inputs, or
//! live input from the keyboard.

use nettai_battle::battle::{mode, top};
use nettai_battle::console::ConsoleSetup;
use nettai_battle::cues::CueAction;
use nettai_battle::content::{ChipCode, Content};
use bn6_compat::Unlocks;
use nettai_battle::custom::{self, BattleFolder, FolderChip, GameVersion, Phase, PlayerSetup, SavedFolder, SlotKind, SlotState};
use nettai_battle::input::keys;
use nettai_battle::link::Link;
use nettai_battle::setup::{BattleSettings, RoundSetup, SetScore};
use nettai_battle::{Battle, PlayerTick, Rng, TickEvents};
use bn6_compat::trace::{self, Frame, Round};
use bn6_compat::{Compat, codec};
use std::sync::Arc;

/// One tick's inputs.
pub struct Step {
    pub input: [PlayerTick; 2],
    pub events: TickEvents,
    /// The frame number this tick shows: the trace frame it reproduces,
    /// or the tick count in live play.
    pub frame: Option<u32>,
}

pub trait Driver {
    /// A fresh battle at the start.
    fn start(&mut self) -> Battle;
    /// The next tick's inputs (`keys`: the local player's held buttons),
    /// or None when there is nothing more to play.
    fn next(&mut self, b: &Battle, keys: u16) -> Option<Step>;
    /// Compare the battle after a step with the source, if it records
    /// what should have happened.
    fn check(&self, _b: &Battle) -> Vec<String> {
        Vec::new()
    }
    /// A short description of where playback is.
    fn position(&self) -> String;
    /// Something to show the player now, if anything (the custom screen).
    fn prompt(&self, _b: &Battle) -> Option<String> {
        None
    }
    /// The region of the console whose screen this is ("us" or "jp"): what
    /// the original would show of the assets only one region's ROMs have
    /// (`Renderer::console_region`).
    fn console_region(&self) -> &'static str {
        "us"
    }
    /// The game version of the console whose screen this is, as its pack
    /// names its versions' assets, for a game whose versions the engine
    /// doesn't tell apart (BN5's "protoman" and "colonel": its navi chips'
    /// pictures; `Renderer::console_version`). None: the engine's (BN6's
    /// `Unlocks::version`).
    fn console_version(&self) -> Option<&'static str> {
        None
    }
    /// The trace frames this round covers (a trace's driver).
    fn frame_range(&self) -> Option<(u32, u32)> {
        None
    }
    /// For a driver that runs the battle itself (netplay's rollback
    /// session), one wall-clock frame with the local player's buttons:
    /// put the frame to show in `shown` and say what happened. None: the
    /// driver gives each tick's inputs instead (`next`).
    fn run_frame(&mut self, _keys: u16, _shown: &mut Battle) -> Option<Result<Ran, String>> {
        None
    }
    /// A line to show all the time (netplay's connection and rollbacks).
    fn status(&self) -> Option<String> {
        None
    }
    /// The battle runs in real time with another player: no pause, no
    /// other speed, no restart.
    fn real_time(&self) -> bool {
        false
    }
}

/// What a frame of a driver that runs the battle itself did
/// (`Driver::run_frame`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ran {
    /// A new frame is in `shown` (none: the frame waited, for clock sync or
    /// the stall guard, and the last one stays).
    pub advanced: bool,
    /// A new round started (the presentation starts over).
    pub new_round: bool,
    /// The sound for this frame: cue actions (a cue played on a prediction
    /// that turned out wrong is canceled).
    pub sound: Vec<CueAction>,
}

// ---- Trace playback ----------------------------------------------------------

/// Replays one round of a golden trace.
pub struct TracePlayer {
    round: Round,
    /// The content the trace's battle runs on (BN6's).
    content: Arc<Content>,
    /// The original's numbers for it, which the comparison reads.
    compat: &'static Compat,
    /// Indices of the frames the engine simulates.
    frames: Vec<usize>,
    pos: usize,
    pub round_number: usize,
}

impl TracePlayer {
    pub fn new(round: Round, round_number: usize, content: Arc<Content>) -> TracePlayer {
        let start = round.setup.frame;
        let frames = round
            .frames
            .iter()
            .enumerate()
            .filter(|(_, f)| f.frame >= start)
            .take_while(|(_, f)| f.state[0] == 4 || f.state[0] == 8)
            .map(|(i, _)| i)
            .collect();
        TracePlayer { compat: Compat::bn6_for(&content), round, content, frames, pos: 0, round_number }
    }

    /// Every round of a trace file, on `content`.
    pub fn load(path: &std::path::Path, content: &Arc<Content>) -> std::io::Result<Vec<TracePlayer>> {
        Ok(trace::rounds(path)?.into_iter().enumerate().map(|(i, r)| TracePlayer::new(r, i + 1, content.clone())).collect())
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// The trace frame numbers this round covers.
    pub fn frame_range(&self) -> Option<(u32, u32)> {
        let f = |i: usize| self.round.frames[self.frames[i]].frame;
        (!self.frames.is_empty()).then(|| (f(0), f(self.frames.len() - 1)))
    }

    fn current(&self) -> Option<&Frame> {
        self.pos.checked_sub(1).and_then(|p| self.frames.get(p)).map(|&i| &self.round.frames[i])
    }
}

impl Driver for TracePlayer {
    fn start(&mut self) -> Battle {
        self.pos = 0;
        self.round.start(self.content.clone(), self.compat)
    }

    fn next(&mut self, _b: &Battle, _keys: u16) -> Option<Step> {
        let &i = self.frames.get(self.pos)?;
        let f = &self.round.frames[i];
        // The frames around it: the link's events are read from the frame
        // before (the session closing) and the one after (a recorded
        // custom screen's result).
        let mut window = Vec::with_capacity(3);
        if let Some(&h) = self.pos.checked_sub(1).and_then(|p| self.frames.get(p)) {
            window.push(&self.round.frames[h]);
        }
        let at = window.len();
        window.push(f);
        if let Some(&j) = self.frames.get(self.pos + 1) {
            window.push(&self.round.frames[j]);
        }
        let ids = codec::Ids::new(&self.content, self.compat);
        let (input, events) = self.round.tick_inputs(at, &window, &ids);
        self.pos += 1;
        Some(Step { input, events, frame: Some(f.frame) })
    }

    fn check(&self, b: &Battle) -> Vec<String> {
        self.current().map(|f| trace::compare(b, f, self.compat)).unwrap_or_default()
    }

    fn console_region(&self) -> &'static str {
        match self.round.console_game() {
            bn6_compat::Game::JpFalzar | bn6_compat::Game::JpGregar => "jp",
            bn6_compat::Game::Falzar | bn6_compat::Game::Gregar => "us",
        }
    }

    fn frame_range(&self) -> Option<(u32, u32)> {
        TracePlayer::frame_range(self)
    }

    fn position(&self) -> String {
        match self.current() {
            Some(f) => format!("round {} frame {}", self.round_number, f.frame),
            None => format!("round {} start", self.round_number),
        }
    }
}

/// Every round of a trace file, on `content`, each as a driver: a BN6
/// recording's ([`TracePlayer`]), or a BN5 one's (its setup line says
/// `"game":"bn5"`: [`Bn5TracePlayer`]), with its round's number.
pub fn trace_rounds(path: &std::path::Path, content: &Arc<Content>) -> Result<Vec<(usize, Box<dyn Driver>)>, String> {
    if trace_game(path).map_err(|e| e.to_string())?.as_deref() == Some("bn5") {
        let rounds = Bn5TracePlayer::load(path, content)?;
        return Ok(rounds.into_iter().map(|r| (r.round_number, Box::new(r) as Box<dyn Driver>)).collect());
    }
    let rounds = TracePlayer::load(path, content).map_err(|e| e.to_string())?;
    Ok(rounds.into_iter().map(|r| (r.round_number, Box::new(r) as Box<dyn Driver>)).collect())
}

/// The game a trace's first setup line names (`"game"`; BN6's recordings
/// name none).
fn trace_game(path: &std::path::Path) -> std::io::Result<Option<String>> {
    use std::io::BufRead;
    let file = std::io::BufReader::new(std::fs::File::open(path)?);
    for line in file.lines() {
        let line = line?;
        if !line.starts_with("{\"setup\"") {
            continue;
        }
        let game = line.split_once("\"game\":\"").and_then(|(_, rest)| rest.split_once('"')).map(|(g, _)| g.to_string());
        return Ok(game);
    }
    Ok(None)
}

// ---- BN5's recordings -------------------------------------------------------

/// Replays one round of a BN5 recording (the chip lab's BN5 library, read
/// by bn5-compat): its setup on BN5's content, then each battle frame's
/// buttons.
pub struct Bn5TracePlayer {
    round: bn5_compat::trace::Round,
    content: Arc<Content>,
    compat: &'static bn5_compat::Compat,
    /// Indices of the frames the engine simulates.
    frames: Vec<usize>,
    pos: usize,
    pub round_number: usize,
    /// The traced console's region and version (its setup line's).
    region: &'static str,
    version: &'static str,
}

impl Bn5TracePlayer {
    /// Every round of a BN5 recording, on `content`: each round's setup
    /// must be one the content defines (bn5-compat's `Round::needs`).
    pub fn load(path: &std::path::Path, content: &Arc<Content>) -> Result<Vec<Bn5TracePlayer>, String> {
        let compat = bn5_compat::Compat::bn5();
        let mut out = Vec::new();
        for (i, round) in bn5_compat::trace::rounds(path)?.into_iter().enumerate() {
            round.round_setup(content, compat).map_err(|e| format!("round {}: {e}", i + 1))?;
            let d = bn5_compat::trace::decode_setup(&round.setup)?;
            let local = d.battle_state[0x0D] as usize & 1;
            let region = if d.japanese[local] { "jp" } else { "us" };
            let version = match d.versions[local] {
                bn5_compat::trace::Version::Protoman => "protoman",
                bn5_compat::trace::Version::Colonel => "colonel",
            };
            let start = round.setup.frame;
            let frames = round
                .frames
                .iter()
                .enumerate()
                .filter(|(_, f)| f.frame >= start)
                .take_while(|(_, f)| f.state[0] == 4 || f.state[0] == 8)
                .map(|(i, _)| i)
                .collect();
            out.push(Bn5TracePlayer { round, content: content.clone(), compat, frames, pos: 0, round_number: i + 1, region, version });
        }
        Ok(out)
    }

    fn current(&self) -> Option<&bn5_compat::trace::Frame> {
        self.pos.checked_sub(1).and_then(|p| self.frames.get(p)).map(|&i| &self.round.frames[i])
    }
}

impl Driver for Bn5TracePlayer {
    fn start(&mut self) -> Battle {
        self.pos = 0;
        // (`load` saw the setup define.)
        self.round.start(self.content.clone(), self.compat).unwrap_or_else(|e| panic!("round {}: {e}", self.round_number))
    }

    fn next(&mut self, _b: &Battle, _keys: u16) -> Option<Step> {
        let &i = self.frames.get(self.pos)?;
        // The frame before too: the link's session closes on the tick the
        // end state moves on.
        let mut window = Vec::with_capacity(2);
        if let Some(&h) = self.pos.checked_sub(1).and_then(|p| self.frames.get(p)) {
            window.push(&self.round.frames[h]);
        }
        let at = window.len();
        window.push(&self.round.frames[i]);
        let (input, events) = self.round.tick_inputs(at, &window);
        self.pos += 1;
        Some(Step { input, events, frame: Some(self.round.frames[i].frame) })
    }

    fn check(&self, b: &Battle) -> Vec<String> {
        self.current().map(|f| bn5_compat::trace::compare(b, f, self.compat)).unwrap_or_default()
    }

    fn console_region(&self) -> &'static str {
        self.region
    }

    fn console_version(&self) -> Option<&'static str> {
        Some(self.version)
    }

    fn frame_range(&self) -> Option<(u32, u32)> {
        let f = |i: usize| self.round.frames[self.frames[i]].frame;
        (!self.frames.is_empty()).then(|| (f(0), f(self.frames.len() - 1)))
    }

    fn position(&self) -> String {
        match self.current() {
            Some(f) => format!("round {} frame {}", self.round_number, f.frame),
            None => format!("round {} start", self.round_number),
        }
    }
}

// ---- Live play -------------------------------------------------------------------
//
// What a live round is made of (the arena, each side's player) is a match
// (`nettai_match`): live play's random draw of one (`nettai_match::draw`),
// or a match file (`--match`).

/// A round to play live on `content` with these battle settings: two
/// MegaMen with 1000 HP (`nettai_match::draw::live_navi`), each bringing
/// their folder, shuffled from the seed, with every Cross and Beast Out of
/// Falzar.
pub fn live_setup(content: &Content, settings: BattleSettings, folders: [SavedFolder; 2], seed: u32) -> RoundSetup {
    let stats = nettai_match::draw::live_navi(content);
    let player = |side: u32| {
        // Each console shuffles its folder with its own RNG (RNG1), which
        // goes on from there.
        let mut rng = Rng::new(seed ^ side.wrapping_mul(0x9E37_79B9));
        let (folder, tag_pair) = BattleFolder::shuffled_with_tag_pair(&folders[side as usize], 0, &mut rng, content);
        let mut player = PlayerSetup {
            folder: Some(folder),
            souls: Default::default(),
            joypad_phase: 0,
            bug_frags: 0,
            navi_level: nettai_match::default_navi_level(content, stats.navi),
            sp_times: Default::default(),
            console: ConsoleSetup { rng: rng.state, tag_pair, ..ConsoleSetup::default() },
            rules: Vec::new(),
            patch_cards: Default::default(),
            navicust: None,
            tactics: Default::default(),
        };
        Unlocks::everything(GameVersion::Falzar).write(content, None, &mut player).expect("BN6's setup");
        player
    };
    RoundSetup {
        content: content.hash(),
        settings,
        ruleset: None,
        navi_stats: [stats, stats],
        rng: seed,
        local_side: 0,
        score: SetScore::default(),
        later_stages: Default::default(),
        low_hp_music_latched: false,
        players: [player(0), player(1)],
        link_delay: Link::RECORDED_DELAY,
    }
}

/// A folder of these chips (the content's, by key, with codes) repeated to
/// 30, with no Regular or tag chips.
pub fn folder_of(content: &Content, chips: &[(&str, u8)]) -> SavedFolder {
    let chip = |key: &str| content.defs.chip_by_key(key).unwrap_or_else(|| panic!("the content defines no chip {key:?}"));
    SavedFolder {
        chips: std::array::from_fn(|i| {
            let (key, code) = chips[i % chips.len()];
            FolderChip::new(chip(key), ChipCode(code))
        }),
        regular: None,
        tags: None,
    }
}

/// Plays a round from the keyboard: the local player is the left navi,
/// with their own custom screen; the right navi stands still, and its
/// custom screen picks the first chip it can and presses OK.
pub struct LivePlayer {
    pub setup: RoundSetup,
    content: Arc<Content>,
    ticks: u32,
}

impl LivePlayer {
    pub fn new(setup: RoundSetup, content: Arc<Content>) -> LivePlayer {
        LivePlayer { setup, content, ticks: 0 }
    }
}

/// The right navi's buttons: on its custom screen, A on the chip under the
/// cursor (the first one) if it can be picked, then START and A, a press
/// every other tick.
fn bot_buttons(b: &Battle, side: usize, tick: u32) -> u16 {
    let s = &b.custom.sides[side];
    let Some(screen) = s.screen.as_ref().filter(|_| b.round.mode == mode::CUSTOM && s.in_custom) else { return 0 };
    if screen.phase != Phase::Choosing || tick % 2 == 0 {
        return 0;
    }
    let here = &screen.slots[screen.cursor as usize];
    if screen.selected == 0 && matches!(here.kind, SlotKind::Chip { .. }) && here.state == SlotState::Selectable {
        keys::A
    } else if screen.cursor != custom::screen::OK_SLOT {
        keys::START
    } else {
        keys::A
    }
}

impl Driver for LivePlayer {
    fn start(&mut self) -> Battle {
        self.ticks = 0;
        Battle::new(self.setup.clone(), self.content.clone())
    }

    fn next(&mut self, b: &Battle, keys: u16) -> Option<Step> {
        self.ticks += 1;
        let local = b.setup.local_side as usize;
        let held = |side: usize| if side == local { keys & 0x3FF } else { bot_buttons(b, side, self.ticks) };
        let input = [PlayerTick { held: held(0) }, PlayerTick { held: held(1) }];
        // The end state asks the link session to close; it closes at once.
        let r = &b.round;
        let events = TickEvents { link_closed: r.top == top::END && r.mode == 0 && r.sub == 4 && r.init == 4, ..TickEvents::default() };
        Some(Step { input, events, frame: Some(self.ticks) })
    }

    fn position(&self) -> String {
        format!("live tick {}", self.ticks)
    }

}

/// A plain-text custom screen for a player: the dealt chips in the grid's
/// order with the cursor, the picks, OK and Beast Out, and the Cross
/// window when it's open.
pub fn custom_screen_text(b: &Battle, side: usize) -> Option<String> {
    let s = &b.custom.sides[side];
    let (screen, folder) = (s.screen.as_ref()?, s.folder.as_ref()?);
    if b.round.mode != mode::CUSTOM {
        return None;
    }
    let mut out = String::new();
    if !s.in_custom {
        out.push_str(if s.sent.is_some() { "CUSTOM: WAITING FOR THE OTHER PLAYER" } else { "CUSTOM: SENDING" });
        return Some(out);
    }
    let title = match screen.phase {
        Phase::Opening { .. } => "CUSTOM",
        Phase::Choosing => "CUSTOM: A PICK, B UNDO, START OK, UP CROSS, R INFO",
        Phase::Hidden { .. } => "CUSTOM (HIDDEN: ANY KEY)",
        Phase::Description { .. } => "CUSTOM: CHIP INFO (ANY KEY)",
        Phase::RunMessage { .. } => "CUSTOM: NO TIME TO RUN (A)",
        // A system's window, by its name (BN6's Beast Out and Cross window).
        Phase::Window { window, .. } => match b.content.defs.window(window).name.as_str() {
            "beast_out" => "CUSTOM: BEAST OUT!",
            "cross_opening" | "cross_window" | "cross_closing" => "CUSTOM: CROSS (UP/DOWN, A CHOOSE, B BACK)",
            "cross_chosen" => "CUSTOM: CROSS!",
            _ => "CUSTOM",
        },
        Phase::SoulChosen { .. } => "CUSTOM: SOUL UNISON!",
        _ => "CUSTOM",
    };
    out.push_str(title);
    let names = |slot: u8| -> String {
        let x = &screen.slots[slot as usize];
        let label = match x.kind {
            SlotKind::Ok => "OK".to_string(),
            SlotKind::Soul => "SOUL".to_string(),
            // A system's button, by its name ("redeal": "REDEAL").
            SlotKind::Button { button, cell: nettai_battle::custom::ButtonCell::Only | nettai_battle::custom::ButtonCell::Left } => {
                b.content.defs.button(button).name.replace('_', " ").to_uppercase()
            }
            SlotKind::Empty | SlotKind::Hidden | SlotKind::Button { .. } => return String::new(),
            _ => screen.chip_in(slot, folder).map(|c| format!("{} {}", nettai_render::strings::own_chip_name(&b.content, c.id), c.code.letter())).unwrap_or_default(),
        };
        let mark = match x.state {
            SlotState::Selected => "+",
            SlotState::Unavailable => "-",
            _ => " ",
        };
        let cursor = if screen.cursor == slot && matches!(screen.phase, Phase::Choosing) { ">" } else { " " };
        format!("{cursor}{mark}{label}")
    };
    for row in [[0u8, 1, 2, 3, 4, 10], [5, 6, 7, 8, 9, 11]] {
        let cells: Vec<String> = row.iter().map(|&s| names(s)).filter(|c| !c.is_empty()).collect();
        if !cells.is_empty() {
            out.push('\n');
            out.push_str(&cells.join(" "));
        }
    }
    let picks: Vec<String> = screen
        .selection()
        .iter()
        .map(|&s| match screen.chip_in(s, folder) {
            Some(c) => format!("{} {}", nettai_render::strings::own_chip_name(&b.content, c.id), c.code.letter()),
            None => "BEAST OUT".to_string(),
        })
        .collect();
    if !picks.is_empty() {
        out.push_str(&format!("\nPICKED: {}", picks.join(", ")));
    }
    // BN6's Cross window (the cross system's).
    let w = nettai_render::custom::CrossWindow::of(b, side).unwrap_or_default();
    let unlocks = Unlocks::of_side(b, side as u8);
    let cross_name = |place: u8| match unlocks.cross_at(&*b.content, b.stats[side].navi, place) {
        Some(f) => nettai_render::strings::own_form_name(&b.content, f).to_uppercase(),
        None => format!("CROSS {}", place + 1),
    };
    if matches!(screen.phase, Phase::Window { .. })
        && nettai_render::custom::cross_stage(b, screen) == Some(nettai_render::custom::CrossStage::Up)
    {
        let entries: Vec<String> = (0..w.count)
            .map(|i| {
                let (cursor, marked) = (if w.cursor == i { ">" } else { " " }, if w.marked[i as usize] { "+" } else { "" });
                format!("{cursor}{marked}{}", cross_name(w.offered[i as usize]))
            })
            .collect();
        out.push_str(&format!("\n{}", entries.join(" ")));
    }
    if let Some(c) = w.chosen {
        out.push_str(&format!("\n{} CHOSEN", cross_name(c)));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live play through the first custom screen: the local player picks
    /// the first chip and presses OK, the right navi's screen does the
    /// same, and the fight starts with both hands.
    #[test]
    fn live_custom_screen() {
        let content = nettai_battle::content::testing::content();
        let stage = content.stage_by_key(nettai_battle::content::testing::LINK_BATTLE);
        let settings = BattleSettings::on(&content, stage);
        // GunDelS3 N, which the test content has.
        let folder = folder_of(&content, &[("gundels3", 13)]);
        let mut live = LivePlayer::new(live_setup(&content, settings, [folder, folder], 7), content.clone());
        let mut b = live.start();
        let mut shown = false;
        for tick in 0..3000u32 {
            let s = &b.custom.sides[0];
            let choosing = s.in_custom && s.screen.as_ref().is_some_and(|x| x.phase == Phase::Choosing);
            shown |= choosing && custom_screen_text(&b, 0).is_some_and(|t| t.contains("OK"));
            let picked = s.screen.as_ref().is_some_and(|x| x.selected > 0);
            let on_ok = s.screen.as_ref().is_some_and(|x| x.cursor == custom::screen::OK_SLOT);
            // A press every other tick: A on the first chip, START, A on OK.
            let held = match (choosing && tick % 2 == 1, picked, on_ok) {
                (false, _, _) => 0,
                (true, false, _) | (true, true, true) => keys::A,
                (true, true, false) => keys::START,
            };
            let step = live.next(&b, held).unwrap();
            b.tick(&step.input, step.events);
            if b.round.turn == 1 && b.round.mode == mode::FIGHTING {
                break;
            }
        }
        assert!(shown);
        assert_eq!((b.round.turn, b.round.mode), (1, mode::FIGHTING));
        for side in 0..2 {
            assert_eq!(b.hands[side].remaining(), 1, "side {side}");
        }
    }

    /// `--save-match` then `--match`: live play's draw for a seed, written
    /// as a match file and played from it, is the same battle as playing
    /// the draw itself, the same digest every tick (the local player
    /// mashing, the right navi the stand-in).
    #[test]
    fn a_saved_match_plays_the_same_battle() {
        let content = nettai_match::testing::bn6_content();
        for seed in [5, 77] {
            let drawn = nettai_match::draw::live(&content, seed, None).unwrap();
            let text = nettai_match::write(&content, &drawn);
            let read = nettai_match::parse(&content, &text).unwrap();
            let mut a = LivePlayer::new(drawn.round(&content, seed), content.clone());
            let mut b = LivePlayer::new(read.round(&content, read.seed.unwrap()), content.clone());
            let (mut x, mut y) = (a.start(), b.start());
            let mut masher = nettai_netplay::standin::Masher::new(seed as u64);
            for tick in 0..2500 {
                // On the custom screen, a pick and OK as the stand-in does;
                // in the fight, anything.
                let mashed = masher.buttons();
                let keys = if x.round.mode == mode::CUSTOM { bot_buttons(&x, 0, tick) } else { mashed };
                let (sa, sb) = (a.next(&x, keys).unwrap(), b.next(&y, keys).unwrap());
                x.tick(&sa.input, sa.events);
                y.tick(&sb.input, sb.events);
                assert_eq!(x.digest(), y.digest(), "seed {seed}: tick {tick}");
            }
            assert!(x.round.turn > 1, "seed {seed}: the battle went on (turn {})", x.round.turn);
        }
    }

    /// Run `live` until `done`, with the local player's buttons from
    /// `keys` (tick, battle); panics past `limit` ticks.
    fn play_until(
        live: &mut LivePlayer,
        b: &mut Battle,
        limit: u32,
        mut keys: impl FnMut(u32, &Battle) -> u16,
        mut done: impl FnMut(&Battle) -> bool,
    ) {
        for tick in 0..limit {
            if done(b) {
                return;
            }
            let held = keys(tick, b);
            let step = live.next(b, held).unwrap();
            b.tick(&step.input, step.events);
        }
        panic!("not done in {limit} ticks: mode {:#x}, turn {}", b.round.mode, b.round.turn);
    }

    /// The local player's screen, while it takes keys.
    fn choosing(b: &Battle) -> Option<&custom::Screen> {
        let s = &b.custom.sides[0];
        s.screen.as_ref().filter(|x| s.in_custom && b.round.mode == mode::CUSTOM && x.phase == Phase::Choosing)
    }

    /// nettai's Cross list on BN6's content: a Falzar player offered
    /// HeatCross, Gregar's, chooses it on the custom screen and fights in
    /// it (its form, element, buster and charged shot: HeatCross's flame);
    /// on the next screen Beast Out from it is HeatCross's Beast form, a
    /// Gregar Beast, with its weapons.
    #[test]
    fn a_falzar_player_plays_a_gregar_cross() {
        use nettai_battle::battle::battle_flags;
        use nettai_battle::content::Element;
        use nettai_battle::kinds::player::{NaviAction, navi_action};
        let content = nettai_match::testing::bn6_content();
        let heat = content.defs.form_by_key("heatcross").unwrap();
        let heat_beast = content.defs.form_by_key("heatcross-beast").unwrap();
        let stage = nettai_match::link_battle_stages(&content)[0];
        let settings = BattleSettings { stage, background: Default::default(), effects: content.stage(stage).effects | nettai_match::MATCH_EFFECTS };
        let folder = folder_of(&content, &[("cannon", 0)]);
        let mut setup = live_setup(&content, settings, [folder, folder], 5);
        Unlocks { cross_list: Some(bn6_compat::CrossList::new(&[heat])), ..Unlocks::everything(GameVersion::Falzar) }
            .write(&content, None, &mut setup.players[0])
            .unwrap();
        let mut live = LivePlayer::new(setup, content.clone());
        let mut b = live.start();
        // The first screen: UP opens the Cross window (a hold acts on its
        // second tick), A chooses HeatCross, START and A press OK.
        play_until(
            &mut live,
            &mut b,
            3000,
            |tick, b| {
                let s = &b.custom.sides[0];
                let Some(screen) = s.screen.as_ref().filter(|_| s.in_custom && b.round.mode == mode::CUSTOM) else { return 0 };
                let w = nettai_render::custom::CrossWindow::of(b, 0).unwrap_or_default();
                match screen.phase {
                    Phase::Choosing if w.chosen.is_none() => [keys::UP, keys::UP, 0][tick as usize % 3],
                    // (The window up, past its first tick, which reads no
                    // keys.)
                    Phase::Window { window, tick: 1.. } if b.content.defs.window(window).name == "cross_window" && w.chosen.is_none() => {
                        assert_eq!((w.count, custom_screen_text(b, 0).unwrap().contains(">HEATCROSS")), (1, true));
                        if tick % 2 == 1 { keys::A } else { 0 }
                    }
                    Phase::Choosing if tick % 2 == 1 => {
                        if screen.cursor == custom::screen::OK_SLOT { keys::A } else { keys::START }
                    }
                    _ => 0,
                }
            },
            |b| b.custom.sides[0].sent.is_some(),
        );
        assert_eq!(b.custom.sides[0].sent.as_ref().unwrap().result.transform.form, Some(heat));
        // The fight resumes and the navi changes into HeatCross.
        let p0 = b.player(0).unwrap();
        play_until(&mut live, &mut b, 1000, |_, _| 0, |b| b.stats[0].form == heat && navi_action(b, p0) == NaviAction::Idle);
        let actor = b.objects.get(p0).actor.unwrap();
        let weapons = content.form(heat).weapons;
        assert_eq!((b.actors.get(actor).buster, b.actors.get(actor).charge_shot), (weapons.buster, weapons.charge_shot));
        assert_eq!(b.objects.get(p0).element & 0xF, Element::Fire as u8);
        // B held charges the buster; let go, HeatCross's flame.
        let flame = content.defs.action_by_key("heatcross/charge/action").unwrap();
        let mut charged = false;
        play_until(
            &mut live,
            &mut b,
            600,
            |tick, _| if tick < 200 { keys::B } else { 0 },
            |b| {
                charged |= navi_action(b, p0) == NaviAction::Content(flame);
                charged
            },
        );
        // The next screen, opened with L once the gauge is full: Beast Out
        // (START, DOWN, A) from HeatCross is HeatCross's Beast form, of
        // Gregar's Beast: the Beast Out button and pictures are Gregar's.
        assert_eq!(Unlocks::of(&b, 0).unwrap().beast_game(&*content, heat), GameVersion::Gregar);
        let mut beast = false;
        play_until(
            &mut live,
            &mut b,
            6000,
            |tick, b| {
                if b.round.mode == mode::FIGHTING {
                    return if b.round.flags & battle_flags::GAUGE_FULL != 0 && tick % 2 == 1 { keys::L } else { 0 };
                }
                let Some(screen) = choosing(b) else { return 0 };
                if b.round.turn < 2 {
                    return 0;
                }
                beast |= screen.form.is_some();
                // Each key held two ticks (a direction acts on a hold's
                // second), then let go.
                let key = match (beast, screen.cursor) {
                    (false, custom::screen::OK_SLOT) => keys::DOWN,
                    (false, custom::screen::SPECIAL_SLOT) => keys::A,
                    (false, _) => keys::START,
                    (true, custom::screen::SPECIAL_SLOT) => keys::UP,
                    (true, _) => keys::A,
                };
                [key, key, 0][tick as usize % 3]
            },
            |b| b.round.turn >= 2 && b.custom.sides[0].sent.is_some(),
        );
        assert_eq!(b.custom.sides[0].sent.as_ref().unwrap().result.transform.form, Some(heat_beast));
        play_until(&mut live, &mut b, 1000, |_, _| 0, |b| b.stats[0].form == heat_beast && navi_action(b, p0) == NaviAction::Idle);
        let weapons = content.form(heat_beast).weapons;
        assert_eq!((b.actors.get(actor).buster, b.actors.get(actor).charge_shot), (weapons.buster, weapons.charge_shot));
        assert_eq!(bn6_compat::forms::game(&content, heat_beast), Some(GameVersion::Gregar));
    }

    // BN6's Cross window (the cross system's: content/bn6/rules/cross/
    // window.luau) with nettai's Cross list, which no recording covers.

    /// A link battle on BN6's content whose side 0 is a `version` player
    /// with the Cross list `list` (form keys; none: the version's Crosses),
    /// its stats changed by `tweak`, run to its first screen's choosing.
    fn cross_battle(
        version: GameVersion,
        list: Option<&[&str]>,
        tweak: impl FnOnce(&Content, &mut nettai_battle::setup::NaviStats),
    ) -> (Arc<Content>, LivePlayer, Battle) {
        let content = nettai_match::testing::bn6_content();
        let stage = nettai_match::link_battle_stages(&content)[0];
        let settings = BattleSettings { stage, background: Default::default(), effects: content.stage(stage).effects | nettai_match::MATCH_EFFECTS };
        let folder = folder_of(&content, &[("cannon", 0)]);
        let mut setup = live_setup(&content, settings, [folder, folder], 5);
        Unlocks {
            cross_list: list.map(|l| bn6_compat::CrossList::new(&l.iter().map(|k| form_of(&content, k)).collect::<Vec<_>>())),
            ..Unlocks::everything(version)
        }
        .write(&content, None, &mut setup.players[0])
        .unwrap();
        tweak(&content, &mut setup.navi_stats[0]);
        let mut live = LivePlayer::new(setup, content.clone());
        let mut b = live.start();
        play_until(&mut live, &mut b, 3000, |_, _| 0, |b| choosing(b).is_some());
        (content, live, b)
    }

    /// The form `key` (BN6's, without its prefix).
    fn form_of(content: &Content, key: &str) -> nettai_content_api::FormHandle {
        content.defs.form_by_key(&format!("{key}")).unwrap_or_else(|| panic!("no form {key}"))
    }

    /// Side 0's keys, one per tick, then on to the next tick.
    fn keys_in_turn(live: &mut LivePlayer, b: &mut Battle, held: &[u16]) {
        for &h in held {
            let step = live.next(b, h).unwrap();
            b.tick(&step.input, step.events);
        }
    }

    /// From choosing, UP opens the Cross window (a direction acts on a
    /// hold's second tick); then the window is up and takes keys.
    fn open_cross_window(live: &mut LivePlayer, b: &mut Battle) {
        keys_in_turn(live, b, &[keys::UP, keys::UP, 0]);
        play_until(live, b, 100, |_, _| 0, |b| {
            let s = b.custom.sides[0].screen.as_ref().unwrap();
            matches!(s.phase, Phase::Window { window, tick: 1.. } if b.content.defs.window(window).name == "cross_window")
        });
    }

    /// In the open Cross window, DOWN `down` times and A: the Cross under
    /// the cursor is chosen, and the screen is back to choosing chips.
    fn choose_cross(live: &mut LivePlayer, b: &mut Battle, down: usize) {
        for _ in 0..down {
            keys_in_turn(live, b, &[keys::DOWN, keys::DOWN, 0]);
        }
        keys_in_turn(live, b, &[keys::A, 0]);
        play_until(live, b, 100, |_, _| 0, |b| choosing(b).is_some());
    }

    /// OK, and what side 0 sends.
    fn confirm(live: &mut LivePlayer, b: &mut Battle) -> nettai_battle::CustomResult {
        keys_in_turn(live, b, &[keys::START, 0, keys::A, 0]);
        play_until(live, b, 200, |_, _| 0, |b| b.custom.sides[0].sent.is_some());
        b.custom.sides[0].sent.as_ref().unwrap().result.clone()
    }

    /// The cross system's record of the Crosses used this round.
    fn crosses_used(b: &Battle) -> [bool; 5] {
        let (schema, state) = b.system_state(0, "cross").expect("BN6's cross system");
        let i = schema.index_of("crosses_used").unwrap();
        std::array::from_fn(|k| state.get_elem(schema, i, k) == Some(nettai_content_api::FieldValue::Bool(true)))
    }

    /// A setup's Cross list offers Crosses of either game in its order: a
    /// Falzar player offered HeatCross (Gregar's first) and GroundCross
    /// (Falzar's fourth) gets those two; the one chosen shows its face and
    /// goes out, is used for the round, and isn't offered on the round's
    /// next screen.
    #[test]
    fn a_cross_list_offers_crosses_of_either_game_once_a_round() {
        use nettai_battle::battle::battle_flags;
        use nettai_render::custom::CrossWindow;
        let (content, mut live, mut b) = cross_battle(GameVersion::Falzar, Some(&["heatcross", "groundcross"]), |_, _| {});
        let heat = form_of(&content, "heatcross");
        let w = CrossWindow::of(&b, 0).unwrap();
        assert_eq!((w.count, &w.offered[..2]), (2, &[0, 1][..]));
        open_cross_window(&mut live, &mut b);
        choose_cross(&mut live, &mut b, 0);
        let screen = b.custom.sides[0].screen.unwrap();
        assert_eq!((screen.look.face, CrossWindow::of(&b, 0).unwrap().chosen), (Some(heat), Some(0)));
        assert_eq!(confirm(&mut live, &mut b).transform.form, Some(heat));
        assert_eq!(crosses_used(&b), [true, false, false, false, false]);
        // The round's next screen, opened with L once the gauge is full.
        play_until(
            &mut live,
            &mut b,
            6000,
            |tick, b| if b.round.mode == mode::FIGHTING && b.round.flags & battle_flags::GAUGE_FULL != 0 && tick % 2 == 1 { keys::L } else { 0 },
            |b| b.round.turn >= 2 && choosing(b).is_some(),
        );
        let w = CrossWindow::of(&b, 0).unwrap();
        assert_eq!((w.count, w.offered[0]), (1, 1));
    }

    /// R in the Cross window describes the Cross under the cursor by its
    /// form: with a Cross list mixing both games, a Falzar player's window
    /// shows HeatCross's own description, not that of Falzar's Cross in its
    /// place (SpoutCross); without a list, the version's Crosses in order.
    #[test]
    fn r_in_the_cross_window_describes_the_cross_under_the_cursor() {
        for (list, down, key) in [(true, 0, "heatcross"), (true, 1, "groundcross"), (false, 0, "spoutcross"), (false, 1, "tomahawkcross")] {
            let list = list.then_some(&["heatcross", "groundcross"][..]);
            let (content, mut live, mut b) = cross_battle(GameVersion::Falzar, list, |_, _| {});
            open_cross_window(&mut live, &mut b);
            for _ in 0..down {
                keys_in_turn(&mut live, &mut b, &[keys::DOWN, keys::DOWN, 0]);
            }
            keys_in_turn(&mut live, &mut b, &[keys::R]);
            let phase = b.custom.sides[0].screen.unwrap().phase;
            let Phase::Description { window: Some(_), form, chatbox } = phase else { panic!("{key}: no description: {phase:?}") };
            let want = form_of(&content, key);
            assert_eq!(form, Some(want), "{key}");
            let breaks = content.form(want).description_lines - 1;
            assert_eq!(chatbox.script(), custom::chatbox::Script::Description { breaks }, "{key}");
        }
    }

    /// In a Beast form a Cross list offers the Crosses whose Beast it is: in
    /// Falzar's Beast Falzar's, in a Gregar Cross's Beast form Gregar's;
    /// each takes the navi to its form in Beast Out. (The navi starts the
    /// battle in the Beast form: a spawned navi takes its starting form.)
    #[test]
    fn in_a_beast_form_a_cross_list_offers_that_beasts_crosses() {
        for (beast, place, result) in [("falzar-beast", 1, "groundcross-beast"), ("heatcross-beast", 0, "heatcross-beast")] {
            let (content, mut live, mut b) = cross_battle(GameVersion::Falzar, Some(&["heatcross", "groundcross"]), |c, stats| {
                stats.form = form_of(c, beast);
                stats.starting_form = stats.form;
            });
            assert_eq!(b.stats[0].form, form_of(&content, beast));
            let w = nettai_render::custom::CrossWindow::of(&b, 0).unwrap();
            assert_eq!((w.count, w.offered[0]), (1, place), "{beast}");
            open_cross_window(&mut live, &mut b);
            choose_cross(&mut live, &mut b, 0);
            assert_eq!(confirm(&mut live, &mut b).transform.form, Some(form_of(&content, result)), "{beast}");
        }
    }

    /// A Cross list offers Crosses only, and leaves out the navi's starting
    /// form, as the original's window does.
    #[test]
    fn a_cross_list_offers_crosses_only() {
        let list = ["spoutcross", "gregar-beast", "eleccross", "tomahawkcross"];
        let (_, _, b) = cross_battle(GameVersion::Gregar, Some(&list), |c, stats| stats.starting_form = form_of(c, "eleccross"));
        let w = nettai_render::custom::CrossWindow::of(&b, 0).unwrap();
        assert_eq!((w.count, &w.offered[..2]), (2, &[0, 3][..]));
    }

    /// B with nothing picked takes the Cross chosen back: its face and its
    /// form go, Beast Out is on offer again, and nothing goes out.
    #[test]
    fn b_takes_the_cross_back() {
        let (_, mut live, mut b) = cross_battle(GameVersion::Falzar, None, |_, _| {});
        open_cross_window(&mut live, &mut b);
        choose_cross(&mut live, &mut b, 1);
        let screen = b.custom.sides[0].screen.unwrap();
        assert!(screen.form.is_some() && screen.look.face.is_some());
        assert_eq!(screen.slots[custom::screen::SPECIAL_SLOT as usize].state, SlotState::Unavailable);
        keys_in_turn(&mut live, &mut b, &[keys::B, 0]);
        let screen = b.custom.sides[0].screen.unwrap();
        assert_eq!((screen.form, screen.look.face), (None, None));
        assert_eq!(screen.slots[custom::screen::SPECIAL_SLOT as usize].state, SlotState::Selectable);
        assert_eq!(nettai_render::custom::CrossWindow::of(&b, 0).unwrap().chosen, None);
        assert_eq!(confirm(&mut live, &mut b).transform.form, None);
        assert_eq!(crosses_used(&b), [false; 5]);
    }

    /// MegaMan received from a navi code (event flag 0x163, the setup's
    /// level) has no Beast Out button (`sub_8029FB4`), and his Cross window
    /// stays (`sub_8029F70`: with the flag, MegaMan's); without a code the
    /// button is there.
    #[test]
    fn a_navi_code_seals_beast_out() {
        for level in [None, Some(3)] {
            let content = nettai_match::testing::bn6_content();
            let stage = nettai_match::link_battle_stages(&content)[0];
            let settings = BattleSettings { stage, background: Default::default(), effects: content.stage(stage).effects | nettai_match::MATCH_EFFECTS };
            let folder = folder_of(&content, &[("cannon", 0)]);
            let mut setup = live_setup(&content, settings, [folder, folder], 5);
            setup.players[0].navi_level = level;
            let mut live = LivePlayer::new(setup, content.clone());
            let mut b = live.start();
            play_until(&mut live, &mut b, 3000, |_, _| 0, |b| choosing(b).is_some());
            let screen = b.custom.sides[0].screen.unwrap();
            let button = match screen.slots[custom::screen::SPECIAL_SLOT as usize].kind {
                SlotKind::Button { button, .. } => Some(b.content.defs.button(button).name.as_str()),
                _ => None,
            };
            assert_eq!(button, if level.is_none() { Some("beast_out") } else { None }, "level {level:?}");
            assert_eq!(nettai_render::custom::CrossWindow::of(&b, 0).unwrap().count, 5, "level {level:?}");
        }
    }
}
