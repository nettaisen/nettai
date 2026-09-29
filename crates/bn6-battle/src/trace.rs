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
