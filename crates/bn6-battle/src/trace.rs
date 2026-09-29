//! Golden traces recorded from the original game (see data/traces/README.md)
//! and the observable-state comparison the engine is verified with.

use serde::Deserialize;
use std::io::BufRead;

/// The static inputs of a round, captured when its battle state machine
/// starts running.
#[derive(Clone, Debug, Deserialize)]
pub struct Setup {
    pub frame: u32,
    pub settings_ptr: u32,
    /// BattleSettings (0x10 bytes), hex.
    pub settings: String,
    /// Both players' in-battle navi stats (0x64 bytes each), hex.
    pub navi_stats: [String; 2],
    /// eBattleFolder (0x50 bytes), hex.
    pub folder: String,
    /// BattleState (0xF0 bytes), hex.
    pub battle_state: String,
    pub rng1: u32,
    pub rng2: u32,
}

/// A battle object as the trace records it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Object {
    #[serde(rename = "type")]
    pub kind: u8,
    pub index: u8,
    pub flags: u8,
    pub params: u32,
    /// CurState, CurAction, CurPhase, PhaseInitialized.
    pub state: [u8; 4],
    pub panel: [u8; 2],
    pub alliance: u8,
    pub flip: u8,
    pub hp: u16,
    pub max_hp: u16,
    /// 16.16 fixed point x, y, z.
    pub pos: [i32; 3],
    pub timer: u16,
    pub anim: u8,
    /// Collision ObjectFlags1 (status: invisible, flinching, paralyzed...).
    pub status: u32,
}

/// One battle frame of the original game.
#[derive(Clone, Debug, Deserialize)]
pub struct Frame {
    pub frame: u32,
    /// BattleState bytes 0-3: top state, mode sub-state, and two sub-sub-states.
    pub state: [u8; 4],
    /// Frames since the round started (counts stalled frames).
    pub frames: u32,
    /// Logic ticks since the round started.
    pub ticks: u32,
    /// Link status this frame (2 = inputs arrived, 8 = stalled).
    pub link: u8,
    pub rng1: u32,
    pub rng2: u32,
    /// BattleState (0xF0 bytes), hex.
    #[serde(default)]
    pub bs: String,
    /// The fighting-phase machine (0xC bytes at 0x0203CA70), hex.
    #[serde(default)]
    pub fight: String,
    /// Custom gauge value (0..0x4000) and fill per tick.
    #[serde(default)]
    pub gauge: u16,
    #[serde(default)]
    pub gauge_rate: u16,
    /// The battle pause byte.
    #[serde(default)]
    pub paused: u8,
    /// HUD update-task mask (bit 4 = gauge fill, bit 15 = banner).
    #[serde(default)]
    pub hud_tasks: u32,
    /// Banner state (0x10 bytes at 0x02036840), hex.
    #[serde(default)]
    pub banner: String,
    /// Per player: held, pressed, released.
    pub input: [[u16; 3]; 2],
    pub objects: Vec<Object>,
    /// (type, alliance) for the 6x3 field, row-major.
    pub panels: Vec<[u8; 2]>,
    /// Both players' 0x50-byte chip blocks, hex.
    pub chip_blocks: [String; 2],
}

#[derive(Clone, Debug)]
pub enum Line {
    Setup(Setup),
    Frame(Box<Frame>),
}

#[derive(Deserialize)]
struct SetupLine {
    setup: Setup,
}

/// Read a trace file, one line at a time.
pub fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<impl Iterator<Item = Line>> {
    let f = std::io::BufReader::new(std::fs::File::open(path)?);
    Ok(f.lines().map(|l| {
        let l = l.expect("reading trace");
        if l.starts_with("{\"setup\"") {
            Line::Setup(serde_json::from_str::<SetupLine>(&l).expect("parsing setup").setup)
        } else {
            Line::Frame(Box::new(serde_json::from_str(&l).expect("parsing frame")))
        }
    }))
}

/// Decode a hex string from a trace.
pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

// ---- Replaying a trace through the engine -----------------------------------

use crate::battle::{Battle, CustomResult, TickEvents};
use crate::hand::ChipHand;
use crate::input::PlayerTick;
use crate::setup::{BattleSettings, NaviStats, RoundSetup, SetScore};

/// A custom-screen exchange record from a trace.
#[derive(Clone, Debug, Deserialize)]
pub struct Exchange {
    pub frame: u32,
    pub navi_stats: [String; 2],
    pub transform: [String; 2],
}

#[derive(Deserialize)]
struct ExchangeLine {
    exchange: Exchange,
}

/// One round of a trace: its setup, exchanges and frames.
pub struct Round {
    pub setup: Setup,
    pub exchanges: Vec<Exchange>,
    pub frames: Vec<Frame>,
}

/// Split a trace into rounds.
pub fn rounds(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<Round>> {
    let f = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut rounds: Vec<Round> = Vec::new();
    let mut pending_exchanges = Vec::new();
    for l in f.lines() {
        let l = l?;
        if l.starts_with("{\"setup\"") {
            let setup = serde_json::from_str::<SetupLine>(&l).expect("setup").setup;
            rounds.push(Round { setup, exchanges: std::mem::take(&mut pending_exchanges), frames: Vec::new() });
        } else if l.starts_with("{\"exchange\"") {
            let e = serde_json::from_str::<ExchangeLine>(&l).expect("exchange").exchange;
            match rounds.last_mut() {
                Some(r) => r.exchanges.push(e),
                None => pending_exchanges.push(e),
            }
        } else if let Some(r) = rounds.last_mut() {
            r.frames.push(serde_json::from_str(&l).expect("frame"));
        }
    }
    Ok(rounds)
}

impl Round {
    /// The engine's starting point for this round.
    pub fn round_setup(&self) -> RoundSetup {
        let bs = unhex(&self.setup.battle_state);
        let stats = |s: &str| NaviStats(unhex(s).try_into().unwrap());
        RoundSetup {
            settings: BattleSettings::netbattle_from_bytes(&unhex(&self.setup.settings)),
            navi_stats: [stats(&self.setup.navi_stats[0]), stats(&self.setup.navi_stats[1])],
            rng: self.setup.rng2,
            local_side: bs[0x0D],
            score: SetScore { wins: bs[0x18], losses: bs[0x19], round: bs[0x1A], max_combo: bs[0x1B] },
        }
    }

    /// Frames of this round the engine simulates (the running and end
    /// states, up to the next round's init).
    pub fn battle_frames(&self) -> impl Iterator<Item = &Frame> {
        self.frames.iter().filter(|f| f.frame >= self.setup.frame).take_while(|f| f.state[0] == 4 || f.state[0] == 8)
    }

    /// Inputs and events for a frame.
    pub fn tick_inputs(&self, i: usize, frames: &[&Frame]) -> ([PlayerTick; 2], TickEvents) {
        let f = frames[i];
        let bs = unhex(&f.bs);
        let input = std::array::from_fn(|p| PlayerTick {
            held: f.input[p][0] & 0x3FF,
            in_custom: bs.get(0x14 + p).copied().unwrap_or(0) & 4 != 0,
        });
        let mut events = TickEvents::default();
        // The local player confirmed on the tick before their status bit
        // cleared.
        if let Some(next) = frames.get(i + 1) {
            let status = |fr: &Frame| unhex(&fr.bs).get(0x11).copied().unwrap_or(0);
            if status(f) & 4 != 0 && status(next) & 4 == 0 && f.state[1] == 8 {
                events.local_confirm = true;
            }
            // The exchange installed on the tick before the mode left the
            // custom screen.
            if f.state[1] == 8 && next.state[1] == 0x0C {
                let latest = |p: usize| -> (NaviStats, u8) {
                    let e = self.exchanges.iter().filter(|e| e.frame <= f.frame).next_back().expect("exchange record");
                    (NaviStats(unhex(&e.navi_stats[p]).try_into().unwrap()), unhex(&e.transform[p])[0])
                };
                let result = |p: usize| {
                    let hand = ChipHand::from_bytes(&unhex(&f.chip_blocks[p]));
                    let (navi_stats, transform) = latest(p);
                    CustomResult { hand: Some(hand), navi_stats, transform }
                };
                events.exchange = Some(Box::new([result(0), result(1)]));
            }
        }
        (input, events)
    }
}

/// Differences between the engine and a trace frame.
pub fn compare(b: &Battle, f: &Frame) -> Vec<String> {
    let mut d = Vec::new();
    let mut check = |what: &str, ours: String, theirs: String| {
        if ours != theirs {
            d.push(format!("{what}: ours {ours} theirs {theirs}"));
        }
    };
    let r = &b.round;
    check("state", format!("{:?}", [r.top, r.mode, r.sub, r.init]), format!("{:?}", f.state));
    check("ticks", r.ticks.to_string(), f.ticks.to_string());
    check("rng2", format!("{:#010x}", b.rng.state), format!("{:#010x}", f.rng2));
    check("paused", (b.paused as u8).to_string(), f.paused.to_string());
    let bs = unhex(&f.bs);
    if bs.len() >= 0xF0 {
        check("battle flags", format!("{:#x}", r.flags), format!("{:#x}", u16::from_le_bytes([bs[0x32], bs[0x33]])));
        check("intro bits", format!("{:#x}", r.intro_bits), format!("{:#x}", bs[0x5C]));
        check("alive", format!("{:?}", r.alive), format!("{:?}", [bs[0x12], bs[0x13]]));
        check("turn", r.turn.to_string(), bs[7].to_string());
        check("battle time", r.battle_time.to_string(), u32::from_le_bytes(bs[0x40..0x44].try_into().unwrap()).to_string());
    }
    check("gauge", format!("{:#x}", b.gauge.value), format!("{:#x}", f.gauge));
    check("banner", (b.banner.active as u8).to_string(), ((f.hud_tasks >> 15) & 1).to_string());
    let ours: Vec<String> = b.objects.in_order().map(|o| describe(b, o)).collect();
    let theirs: Vec<String> = f.objects.iter().map(describe_trace).collect();
    if ours != theirs {
        check(
            "objects",
            format!("\n    ours   {}", ours.join("\n           ")),
            format!("\n    theirs {}", theirs.join("\n           ")),
        );
    }
    let panels: Vec<[u8; 2]> = (1..=3)
        .flat_map(|y| (1..=6).map(move |x| (x, y)))
        .map(|(x, y)| {
            let p = b.field.panel(x, y).unwrap();
            [p.kind, p.alliance]
        })
        .collect();
    check("panels", format!("{panels:?}"), format!("{:?}", f.panels));
    for p in 0..2 {
        let ours = b.hands[p].to_bytes().iter().map(|x| format!("{x:02x}")).collect::<String>();
        check(&format!("hand {p}"), ours, f.chip_blocks[p].clone());
    }
    d
}

fn describe(b: &Battle, r: crate::object::ObjectRef) -> String {
    let o = b.objects.get(r);
    let status = o.collision.map(|c| b.collision.get(c).f1).unwrap_or(0);
    format!(
        "T{}#{:#04x} f{:#04x} s{:?} p{},{} a{} hp{}/{} pos{},{},{} st{:#x}",
        r.pool.type_number(),
        o.index,
        o.flags,
        [o.state, o.action, o.phase, o.phase_init],
        o.panel.x,
        o.panel.y,
        o.alliance,
        o.hp,
        o.max_hp,
        o.pos.x,
        o.pos.y,
        o.pos.z,
        status
    )
}

fn describe_trace(o: &Object) -> String {
    format!(
        "T{}#{:#04x} f{:#04x} s{:?} p{},{} a{} hp{}/{} pos{},{},{} st{:#x}",
        o.kind, o.index, o.flags, o.state, o.panel[0], o.panel[1], o.alliance, o.hp, o.max_hp, o.pos[0], o.pos[1], o.pos[2], o.status
    )
}

/// Run a round through the engine; returns the number of frames that
/// matched before the first difference, and that difference.
pub fn run_round(round: &Round) -> (usize, Option<(u32, Vec<String>)>) {
    let frames: Vec<&Frame> = round.battle_frames().collect();
    let mut b = Battle::new(round.round_setup());
    // Counters carried in from init.
    let bs = unhex(&round.setup.battle_state);
    b.round.frames = u32::from_le_bytes(bs[0x60..0x64].try_into().unwrap());
    b.round.ticks = u32::from_le_bytes(bs[0x64..0x68].try_into().unwrap());
    for i in 0..frames.len() {
        let (input, events) = round.tick_inputs(i, &frames);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.tick(&input, events)));
        if let Err(e) = result {
            let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
            return (i, Some((frames[i].frame, vec![format!("engine panicked: {}", msg.unwrap_or_default())])));
        }
        let diffs = compare(&b, frames[i]);
        if !diffs.is_empty() {
            return (i, Some((frames[i].frame, diffs)));
        }
    }
    (frames.len(), None)
}
