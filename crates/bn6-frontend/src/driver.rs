//! What drives the battle each tick: a golden trace's recorded inputs, or
//! live input from the keyboard.

use bn6_battle::battle::{mode, top};
use bn6_battle::console::ConsoleSetup;
use bn6_battle::content::{ChipCode, ChipId, Content};
use bn6_battle::custom::{self, BattleFolder, FolderChip, GameVersion, Phase, PlayerSetup, SavedFolder, SlotKind, SlotState, Unlocks};
use bn6_battle::input::keys;
use bn6_battle::link::Link;
use bn6_battle::setup::{BattleSettings, RoundSetup, SetScore};
use bn6_battle::{Battle, PlayerTick, Rng, TickEvents};
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
        TracePlayer { round, content, compat: Compat::bn6(), frames, pos: 0, round_number }
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
        let mut window = vec![f];
        if let Some(&j) = self.frames.get(self.pos + 1) {
            window.push(&self.round.frames[j]);
        }
        let ids = codec::Ids::new(&self.content, self.compat);
        let (input, events) = self.round.tick_inputs(0, &window, &ids);
        self.pos += 1;
        Some(Step { input, events, frame: Some(f.frame) })
    }

    fn check(&self, b: &Battle) -> Vec<String> {
        self.current().map(|f| trace::compare(b, f, self.compat)).unwrap_or_default()
    }

    fn position(&self) -> String {
        match self.current() {
            Some(f) => format!("round {} frame {}", self.round_number, f.frame),
            None => format!("round {} start", self.round_number),
        }
    }
}

// ---- Live play -------------------------------------------------------------------

/// The battle settings live play uses: the netbattle of the recorded
/// matches (field layout 0xE3, background 0x0B, one MegaMan per side).
const LIVE_SETTINGS: &str = "e36415000b0038008c0e000092190b08";
/// A MegaMan with 1000 HP and no NaviCust programs of note.
const LIVE_NAVI: &str = "08000000000100ff00320505010080000000ff00000000000000000101000001010301000000001f0000000a0000ffffff0000000000000000ff00000000e803e803e8030000010000000a0000000000000000000000ffffffffffff0000000000000000";

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// The live round on BN6's content: the netbattle of the recorded
/// matches, both players with the live folder (`LIVE_FOLDER`).
pub fn bn6_live_setup(content: &Content, seed: u32) -> RoundSetup {
    let settings = codec::battle_settings(&unhex(LIVE_SETTINGS), &codec::Ids::new(content, Compat::bn6()));
    live_setup(content, settings, &LIVE_FOLDER, seed)
}

/// A round to play live on `content` with these battle settings: two
/// MegaMen with 1000 HP who both bring `folder` (chip ids with codes,
/// repeated to 30 chips; BN6's numbers, which a chip content defines under
/// compat's key takes), each shuffled from the seed.
pub fn live_setup(content: &Content, settings: BattleSettings, folder: &[(ChipId, u8)], seed: u32) -> RoundSetup {
    let ids = codec::Ids::new(content, Compat::bn6());
    let stats = codec::navi_stats(&unhex(LIVE_NAVI).try_into().unwrap(), &ids);
    let player = |side: u32| {
        let saved = SavedFolder {
            chips: std::array::from_fn(|i| {
                let (id, code) = folder[i % folder.len()];
                FolderChip::new(ids.chip(id), ChipCode(code))
            }),
            regular: None,
            tags: None,
        };
        // Each console shuffles its folder with its own RNG (RNG1), which
        // goes on from there.
        let mut rng = Rng::new(seed ^ side.wrapping_mul(0x9E37_79B9));
        let (folder, tag_pair) = BattleFolder::shuffled_with_tag_pair(&saved, 0, &mut rng, content);
        PlayerSetup {
            folder: Some(folder),
            unlocks: Unlocks::everything(GameVersion::Falzar),
            joypad_phase: 0,
            bug_frags: 0,
            navi_level: 0,
            console: ConsoleSetup { rng: rng.state, tag_pair, emotion_window_glitch: false },
        }
    };
    RoundSetup {
        content: content.hash(),
        settings,
        navi_stats: [stats, stats],
        rng: seed,
        local_side: 0,
        score: SetScore::default(),
        // A single round: the stages are never used.
        later_stages: Default::default(),
        low_hp_music_latched: false,
        sp_times: Default::default(),
        players: [player(0), player(1)],
        link_delay: Link::RECORDED_DELAY,
    }
}

/// The live folder on BN6's content (chip ids with their codes), repeated
/// to 30: GunDelSols, Geddon, Invisibl and EraseMan.
const LIVE_FOLDER: [(u16, u8); 6] = [(0x11, 13), (0x0F, 2), (0xA7, 26), (0x11, 16), (0xB1, 26), (0xEC, 10)];

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

    fn prompt(&self, b: &Battle) -> Option<String> {
        custom_screen_text(b, b.setup.local_side as usize)
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
        Phase::CrossWindow { .. } | Phase::CrossWindowOpening { .. } | Phase::CrossWindowClosing { .. } => {
            "CUSTOM: CROSS (UP/DOWN, A CHOOSE, B BACK)"
        }
        Phase::CrossChosen { .. } => "CUSTOM: CROSS!",
        Phase::BeastOutChosen { .. } => "CUSTOM: BEAST OUT!",
        _ => "CUSTOM",
    };
    out.push_str(title);
    let names = |slot: u8| -> String {
        let x = &screen.slots[slot as usize];
        let label = match x.kind {
            SlotKind::Ok => "OK".to_string(),
            SlotKind::BeastOut => "BEAST OUT".to_string(),
            SlotKind::Scrap { right_half: false } => "SCRAP".to_string(),
            SlotKind::Redeal { right_half: false } => "REDEAL".to_string(),
            SlotKind::Empty | SlotKind::Hidden | SlotKind::Scrap { .. } | SlotKind::Redeal { .. } => return String::new(),
            _ => screen.chip_in(slot, folder).map(|c| format!("{} {}", b.content.chip(c.id).name, c.code.letter())).unwrap_or_default(),
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
            Some(c) => format!("{} {}", b.content.chip(c.id).name, c.code.letter()),
            None => "BEAST OUT".to_string(),
        })
        .collect();
    if !picks.is_empty() {
        out.push_str(&format!("\nPICKED: {}", picks.join(", ")));
    }
    let w = &screen.crosses;
    if matches!(screen.phase, Phase::CrossWindow { .. }) {
        let entries: Vec<String> = (0..w.count)
            .map(|i| format!("{}{}CROSS {}", if w.cursor == i { ">" } else { " " }, if w.marked[i as usize] { "+" } else { "" }, w.offered[i as usize] + 1))
            .collect();
        out.push_str(&format!("\n{}", entries.join(" ")));
    }
    if let Some(c) = w.chosen {
        out.push_str(&format!("\nCROSS {} CHOSEN", c + 1));
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
        let content = bn6_battle::content::testing::content();
        let stage = content.stage_numbered(bn6_battle::content::testing::LINK_BATTLE);
        let settings = BattleSettings::on(&content, stage);
        let folder = [(bn6_battle::content::testing::SUN_GUN_3, 0)];
        let mut live = LivePlayer::new(live_setup(&content, settings, &folder, 7), content.clone());
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
}
