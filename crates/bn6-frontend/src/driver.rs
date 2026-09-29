//! What drives the battle each tick: a golden trace's recorded inputs, or
//! live input from the keyboard.

use bn6_battle::battle::mode;
use bn6_battle::input::keys;
use bn6_battle::hand::{ChipHand, NO_CHIP};
use bn6_battle::setup::{BattleSettings, Form, NaviStats, RoundSetup, SetScore};
use bn6_battle::trace::{self, Frame, Round};
use bn6_battle::transform::TransformRequest;
use bn6_battle::{Battle, CustomResult, PlayerTick, TickEvents};

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
    /// Something the player has to do now, if anything.
    fn prompt(&self) -> Option<&str> {
        None
    }
}

// ---- Trace playback ----------------------------------------------------------

/// Replays one round of a golden trace.
pub struct TracePlayer {
    round: Round,
    /// Indices of the frames the engine simulates.
    frames: Vec<usize>,
    pos: usize,
    pub round_number: usize,
}

impl TracePlayer {
    pub fn new(round: Round, round_number: usize) -> TracePlayer {
        let start = round.setup.frame;
        let frames = round
            .frames
            .iter()
            .enumerate()
            .filter(|(_, f)| f.frame >= start)
            .take_while(|(_, f)| f.state[0] == 4 || f.state[0] == 8)
            .map(|(i, _)| i)
            .collect();
        TracePlayer { round, frames, pos: 0, round_number }
    }

    /// Every round of a trace file.
    pub fn load(path: &std::path::Path) -> std::io::Result<Vec<TracePlayer>> {
        Ok(trace::rounds(path)?.into_iter().enumerate().map(|(i, r)| TracePlayer::new(r, i + 1)).collect())
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
        let mut b = Battle::new(self.round.round_setup());
        // Counters carried in from the round's init.
        let bs = trace::unhex(&self.round.setup.battle_state);
        b.round.frames = u32::from_le_bytes(bs[0x60..0x64].try_into().unwrap());
        b.round.ticks = u32::from_le_bytes(bs[0x64..0x68].try_into().unwrap());
        b
    }

    fn next(&mut self, _b: &Battle, _keys: u16) -> Option<Step> {
        let &i = self.frames.get(self.pos)?;
        let f = &self.round.frames[i];
        let mut window = vec![f];
        if let Some(&j) = self.frames.get(self.pos + 1) {
            window.push(&self.round.frames[j]);
        }
        let (input, events) = self.round.tick_inputs(0, &window);
        self.pos += 1;
        Some(Step { input, events, frame: Some(f.frame) })
    }

    fn check(&self, b: &Battle) -> Vec<String> {
        self.current().map(|f| trace::compare(b, f)).unwrap_or_default()
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

/// A round to play live.
pub fn live_setup(seed: u32) -> RoundSetup {
    let stats = NaviStats::from_bytes(&unhex(LIVE_NAVI).try_into().unwrap());
    RoundSetup {
        settings: BattleSettings::netbattle_from_bytes(&unhex(LIVE_SETTINGS)),
        navi_stats: [stats, stats],
        rng: seed,
        local_side: 0,
        score: SetScore::default(),
    }
}

/// The chips both sides get at every custom screen (chip ids with their
/// codes, as a custom screen would hand them over): GunDelS3 N twice and
/// Geddon * twice.
const LIVE_HAND: [(u16, u8); 4] = [(0x11, 13), (0x11, 13), (0xA7, 26), (0xA7, 26)];

/// The live hand as the custom screen's result (the game's chip block).
pub fn live_hand() -> ChipHand {
    let mut block = [0u8; 0x50];
    for i in 0..6 {
        let (id, selection) = match LIVE_HAND.get(i) {
            Some(&(id, code)) => (id, (code as u16) << 9 | id),
            None => (NO_CHIP, NO_CHIP),
        };
        block[0x02 + 2 * i..0x04 + 2 * i].copy_from_slice(&id.to_le_bytes());
        block[0x32 + 2 * i..0x34 + 2 * i].copy_from_slice(&selection.to_le_bytes());
    }
    ChipHand::from_bytes(&block)
}

/// How the live custom screen is going.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Custom {
    /// Not on the custom screen.
    Closed,
    /// Open; waiting for the player to choose.
    Choosing,
    /// Confirmed this many ticks ago (with Beast Out or not).
    Confirmed(u32, bool),
    /// Results sent; waiting for the fight to resume.
    Sent,
}

/// Plays a round from the keyboard: the local player is the left navi;
/// the right navi stands still. The custom screen is a stand-in: A takes
/// a fixed hand of chips, B the same and Beast Out.
pub struct LivePlayer {
    pub setup: RoundSetup,
    custom: Custom,
    previous_keys: u16,
    ticks: u32,
}

/// Ticks between confirming the custom screen and the results arriving
/// (the link exchange).
const EXCHANGE_DELAY: u32 = 12;

impl LivePlayer {
    pub fn new(setup: RoundSetup) -> LivePlayer {
        LivePlayer { setup, custom: Custom::Closed, previous_keys: 0, ticks: 0 }
    }

    /// Whether the stand-in custom screen is waiting for A.
    pub fn choosing(&self) -> bool {
        self.custom == Custom::Choosing
    }
}

impl Driver for LivePlayer {
    fn start(&mut self) -> Battle {
        self.custom = Custom::Closed;
        self.ticks = 0;
        self.previous_keys = 0;
        Battle::new(self.setup.clone())
    }

    fn next(&mut self, b: &Battle, keys: u16) -> Option<Step> {
        let pressed = keys & !self.previous_keys;
        self.previous_keys = keys;
        self.ticks += 1;
        let mut events = TickEvents::default();
        let in_custom = b.round.mode == mode::CUSTOM;
        self.custom = match (self.custom, in_custom) {
            (_, false) => Custom::Closed,
            (Custom::Closed, true) => Custom::Choosing,
            (Custom::Choosing, true) if pressed & (keys::A | keys::B) != 0 => {
                events.local_confirm = true;
                let beast = pressed & keys::B != 0 && !b.stats[b.setup.local_side as usize].form.is_beast();
                Custom::Confirmed(0, beast)
            }
            (Custom::Confirmed(n, beast), true) if n >= EXCHANGE_DELAY => {
                let local = b.setup.local_side as usize;
                let result = |side: usize| {
                    let form = (side == local && beast).then_some(Form::FALZAR_BEAST);
                    let transform = TransformRequest { form, ..TransformRequest::NONE };
                    CustomResult { hand: Some(live_hand()), navi_stats: b.stats[side], transform }
                };
                events.exchange = Some(Box::new([result(0), result(1)]));
                Custom::Sent
            }
            (Custom::Confirmed(n, beast), true) => Custom::Confirmed(n + 1, beast),
            (c, true) => c,
        };
        let choosing = self.custom == Custom::Choosing;
        let held = if in_custom { 0 } else { keys & 0x3FF };
        let input = [PlayerTick { held, in_custom: choosing }, PlayerTick { held: 0, in_custom: choosing }];
        Some(Step { input, events, frame: Some(self.ticks) })
    }

    fn position(&self) -> String {
        format!("live tick {}", self.ticks)
    }

    fn prompt(&self) -> Option<&str> {
        self.choosing().then_some("CUSTOM: A = GUNDELS3 X2 + GEDDON X2, B = THE SAME + BEAST OUT")
    }
}
