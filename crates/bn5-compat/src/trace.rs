//! The chip lab's BN5 recordings: JSON lines with BN6's line kinds (a
//! `setup` line, `exchange` lines, one line per frame) read from BN5's RAM
//! (the verification workspace's oracle-trace `BN5` layout), and their
//! decoding into the engine's terms ([`codec`](crate::codec)). Nothing
//! replays them yet: the engine runs no BN5 content until rules-in-luau.md
//! R and the BN5 port.

use crate::codec::{self, ChipHand, NAVI_STATS, NaviStats, Panel};
use crate::{Compat, pool_of_type, pool_slots};
use nettai_content_api::Pool;
use serde::Deserialize;
use std::io::BufRead;

/// The static inputs of a round, as a BN5 recording's setup line has them.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub frame: u32,
    /// "bn5".
    pub game: String,
    pub settings_ptr: u32,
    /// BattleSettings (0x10 bytes), hex.
    pub settings: String,
    /// Both sides' NaviStats (0x60 bytes each), hex.
    pub navi_stats: [String; 2],
    /// The recording console's battle folder (0x50 bytes), hex, and both
    /// consoles' by side.
    pub folder: String,
    #[serde(default)]
    pub folders: Option<[String; 2]>,
    /// BattleState (0xF0 bytes), hex.
    pub battle_state: String,
    pub rng1: u32,
    pub rng2: u32,
    /// Both sides' versions ("protoman" or "colonel").
    pub game_versions: [String; 2],
    /// Both sides' regions ("us" or "jp"); recordings of US consoles have
    /// none.
    #[serde(default)]
    pub game_regions: Option<[String; 2]>,
    #[serde(default)]
    pub joypad_phases: Option<[u8; 2]>,
    #[serde(default)]
    pub frame_counter: Option<u16>,
}

/// A battle object as the recording has it (BN6's fields).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Object {
    #[serde(rename = "type")]
    pub kind: u8,
    pub index: u8,
    pub flags: u8,
    pub params: u32,
    pub state: [u8; 4],
    pub panel: [u8; 2],
    pub alliance: u8,
    pub flip: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub pos: [i32; 3],
    pub timer: u16,
    pub anim: u8,
    pub status: u32,
}

/// One battle frame (BN6's fields).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub frame: u32,
    pub state: [u8; 4],
    pub frames: u32,
    pub ticks: u32,
    pub link: u8,
    pub rng1: u32,
    pub rng2: u32,
    pub bs: String,
    pub fight: String,
    pub gauge: u16,
    pub gauge_rate: u16,
    pub paused: u8,
    pub hud_tasks: u32,
    pub banner: String,
    pub input: [[u16; 3]; 2],
    pub objects: Vec<Object>,
    pub panels: Vec<[u8; 2]>,
    pub chip_blocks: [String; 2],
}

/// A custom-screen exchange: both sides' NaviStats and transform records.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exchange {
    pub frame: u32,
    pub navi_stats: [String; 2],
    pub transform: [String; 2],
}

#[derive(Clone, Debug)]
pub enum Line {
    Setup(Box<Setup>),
    Exchange(Exchange),
    Frame(Box<Frame>),
}

#[derive(Deserialize)]
struct SetupLine {
    setup: Setup,
}

#[derive(Deserialize)]
struct ExchangeLine {
    exchange: Exchange,
}

/// One line of a recording.
pub fn parse_line(l: &str) -> Result<Line, String> {
    if l.starts_with("{\"setup\"") {
        Ok(Line::Setup(Box::new(serde_json::from_str::<SetupLine>(l).map_err(|e| format!("setup: {e}"))?.setup)))
    } else if l.starts_with("{\"exchange\"") {
        Ok(Line::Exchange(serde_json::from_str::<ExchangeLine>(l).map_err(|e| format!("exchange: {e}"))?.exchange))
    } else {
        Ok(Line::Frame(Box::new(serde_json::from_str(l).map_err(|e| format!("frame: {e}"))?)))
    }
}

/// A recording's lines.
pub fn read(path: impl AsRef<std::path::Path>) -> Result<Vec<Line>, String> {
    let path = path.as_ref();
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    std::io::BufReader::new(f)
        .lines()
        .enumerate()
        .map(|(n, l)| {
            let l = l.map_err(|e| format!("{}: {e}", path.display()))?;
            parse_line(&l).map_err(|e| format!("{}:{}: {e}", path.display(), n + 1))
        })
        .collect()
}

/// Decode a hex string.
pub fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err(format!("odd hex string of {} digits", s.len()));
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("hex {s:.16}: {e}"))).collect()
}

fn navi_stats_hex(s: &str) -> Result<NaviStats, String> {
    let b: [u8; NAVI_STATS] = unhex(s)?.try_into().map_err(|b: Vec<u8>| format!("a NaviStats block of {:#x} bytes", b.len()))?;
    codec::navi_stats(&b)
}

/// A side's version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    Protoman,
    Colonel,
}

/// A round's setup, decoded.
#[derive(Clone, Debug)]
pub struct DecodedSetup {
    pub navi_stats: [NaviStats; 2],
    pub versions: [Version; 2],
    /// Both sides' regions: true for a Japanese console.
    pub japanese: [bool; 2],
    /// The settings record (stages are by BN5's numbers: no stage content
    /// yet).
    pub settings: [u8; 0x10],
    pub battle_state: Vec<u8>,
}

pub fn decode_setup(s: &Setup) -> Result<DecodedSetup, String> {
    if s.game != "bn5" {
        return Err(format!("a {} recording", s.game));
    }
    let version = |v: &str| match v {
        "protoman" => Ok(Version::Protoman),
        "colonel" => Ok(Version::Colonel),
        v => Err(format!("version {v:?}")),
    };
    let region = |r: &str| match r {
        "us" => Ok(false),
        "jp" => Ok(true),
        r => Err(format!("region {r:?}")),
    };
    let regions = match &s.game_regions {
        Some([a, b]) => [region(a)?, region(b)?],
        None => [false, false],
    };
    let battle_state = unhex(&s.battle_state)?;
    if battle_state.len() != 0xF0 {
        return Err(format!("a BattleState of {:#x} bytes", battle_state.len()));
    }
    for f in std::iter::once(&s.folder).chain(s.folders.iter().flatten()) {
        if unhex(f)?.len() != 0x50 {
            return Err("a battle folder that isn't 0x50 bytes".into());
        }
    }
    Ok(DecodedSetup {
        navi_stats: [navi_stats_hex(&s.navi_stats[0])?, navi_stats_hex(&s.navi_stats[1])?],
        versions: [version(&s.game_versions[0])?, version(&s.game_versions[1])?],
        japanese: regions,
        settings: unhex(&s.settings)?.try_into().map_err(|_| "a settings record that isn't 0x10 bytes".to_string())?,
        battle_state,
    })
}

/// An object, in the engine's terms: its pool, and BN5's kind number in it
/// (its slot's owner: BN5's kinds have no content keys yet).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedObject {
    pub pool: Pool,
    pub kind: u8,
    pub object: Object,
}

/// A frame, decoded.
#[derive(Clone, Debug)]
pub struct DecodedFrame {
    pub frame: u32,
    /// The 6x3 field, row-major.
    pub panels: Vec<Panel>,
    pub objects: Vec<DecodedObject>,
    pub hands: [ChipHand; 2],
}

pub fn decode_frame(compat: &Compat, f: &Frame) -> Result<DecodedFrame, String> {
    if f.panels.len() != 18 {
        return Err(format!("frame {}: {} panels", f.frame, f.panels.len()));
    }
    let panels = f.panels.iter().map(|&[t, a]| codec::panel(compat, t, a)).collect::<Result<Vec<_>, _>>()?;
    let mut counts = [0usize; 3];
    let mut objects = Vec::with_capacity(f.objects.len());
    for o in &f.objects {
        let pool = pool_of_type(o.kind).ok_or_else(|| format!("frame {}: object type {}", f.frame, o.kind))?;
        let n = &mut counts[match pool {
            Pool::Actor => 0,
            Pool::Attack => 1,
            Pool::Effect => 2,
        }];
        *n += 1;
        if *n > pool_slots(pool) {
            return Err(format!("frame {}: more {pool:?} objects than BN5's {} slots", f.frame, pool_slots(pool)));
        }
        objects.push(DecodedObject { pool, kind: o.index, object: o.clone() });
    }
    let hand = |s: &str| codec::chip_hand(compat, &unhex(s)?);
    Ok(DecodedFrame { frame: f.frame, panels, objects, hands: [hand(&f.chip_blocks[0])?, hand(&f.chip_blocks[1])?] })
}

/// An exchange, decoded: both sides' stats as the custom screen sent them,
/// and the transform records (raw: BN5's forms have no content yet).
pub fn decode_exchange(e: &Exchange) -> Result<([NaviStats; 2], [Vec<u8>; 2]), String> {
    let t = |s: &str| -> Result<Vec<u8>, String> {
        let b = unhex(s)?;
        if b.len() != 0x10 {
            return Err(format!("a transform record of {:#x} bytes", b.len()));
        }
        Ok(b)
    };
    Ok(([navi_stats_hex(&e.navi_stats[0])?, navi_stats_hex(&e.navi_stats[1])?], [t(&e.transform[0])?, t(&e.transform[1])?]))
}

/// Decode every line of a recording; the count of frames decoded.
pub fn decode_all(compat: &Compat, path: impl AsRef<std::path::Path>) -> Result<usize, String> {
    let path = path.as_ref();
    let mut frames = 0;
    for (n, line) in read(path)?.iter().enumerate() {
        let r = match line {
            Line::Setup(s) => decode_setup(s).map(|_| ()),
            Line::Exchange(e) => decode_exchange(e).map(|_| ()),
            Line::Frame(f) => {
                frames += 1;
                decode_frame(compat, f).map(|_| ())
            }
        };
        r.map_err(|e| format!("{}:{}: {e}", path.display(), n + 1))?;
    }
    Ok(frames)
}
