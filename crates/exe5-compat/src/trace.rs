//! The chip lab's EXE5 recordings: JSON lines with EXE6's line kinds (a
//! `setup` line, `exchange` lines, one line per frame) read from EXE5's RAM
//! (the verification workspace's oracle-trace `EXE5` layout), their
//! decoding into the engine's terms ([`codec`](crate::codec)), and their
//! replay: a round's setup, then each battle frame ticked and compared
//! ([`run_round`]), as far as the engine gets (docs/design/exe5-map.md
//! §15.5: the setup stops at what EXE5's content doesn't define yet).

use crate::codec::{self, ChipHand, NAVI_STATS, NaviStats, Panel};
use crate::{Compat, pool_of_type, pool_slots};
use nettai_battle::content::ChipCode;
use nettai_battle::custom::{BattleFolder, FolderChip, PlayerSetup};
use nettai_battle::console::ConsoleSetup;
use nettai_battle::navicust::{NaviCust, PlacedProgram};
use nettai_battle::setup::{NaviCustBugs, NaviWeapons};
use nettai_battle::{Battle, Content, NaviStats as EngineNaviStats, PlayerTick, RoundSetup, TickEvents};
use nettai_content_api::{RecordHandle, WeaponHandle};
use nettai_content_api::Pool;
use serde::Deserialize;
use std::io::BufRead;
use std::sync::Arc;

/// The static inputs of a round, as an EXE5 recording's setup line has them.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub frame: u32,
    /// "exe5".
    pub game: String,
    pub settings_ptr: u32,
    /// BattleSettings (0x10 bytes), hex.
    pub settings: String,
    /// Both sides' NaviStats (0x60 bytes each), hex.
    pub navi_stats: [String; 2],
    /// The recording console's battle folder (0x50 bytes), hex, and both
    /// consoles' by side.
    pub folder: String,
    pub folders: [String; 2],
    /// BattleState (0xF0 bytes), hex.
    pub battle_state: String,
    pub rng1: u32,
    pub rng2: u32,
    /// Both sides' versions ("protoman" or "colonel").
    pub game_versions: [String; 2],
    /// Both sides' regions ("us" or "jp").
    pub game_regions: [String; 2],
    /// Both consoles' joypad repeat beats on the round's first frame.
    pub joypad_phases: [u8; 2],
    /// The recording console's frame counter on the setup's frame.
    pub frame_counter: u16,
    /// Both players' computer-navi data as the link exchanged it (0xE0
    /// bytes each, side 0's first), hex: their tactics. Older recordings
    /// have none.
    #[serde(default)]
    pub ai_lists: Option<[String; 2]>,
    /// Both consoles' emotion window glitches as their window's start reads
    /// them (0x0813F650: with patch cards in the save's list the cards' flag
    /// 0x10C4, else the NaviCust's 0x10C1). Older recordings have none.
    #[serde(default)]
    pub emotion_window_glitches: Option<[bool; 2]>,
    /// Both consoles' RNG1 and Regular flags (BattleState+0x17), when the
    /// other console's last capture was on the setup's frame.
    #[serde(default)]
    pub rng1s: Option<[u32; 2]>,
    #[serde(default)]
    pub regular_flags: Option<[u8; 2]>,
    /// Both consoles' NaviCusts as their saves have them: what the battle's
    /// start compiled into the recorded stats (the reload, 0x0813F97C). A
    /// round with them is replayed by compiling them, as a match is set up
    /// ([`Round::round_setup`]). Older recordings have none: their stats are
    /// replayed as recorded.
    #[serde(default)]
    pub navicusts: Option<[NaviCustSetup; 2]>,
    /// Both consoles' installed patch cards, each its save's list (the
    /// card's number, bit 7 set when switched off); none when neither has
    /// any, or in an older recording.
    #[serde(default)]
    pub patch_cards: Option<[Vec<u8>; 2]>,
}

/// A console's NaviCust in a setup line.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviCustSetup {
    /// The save's list ([`crate::save::NAVICUST_PARTS`] parts of 8 bytes,
    /// 0x02004D6C), hex.
    pub list: String,
    /// The event flag bytes of the compression flags (0x1EC0 to 0x1FBF: 32
    /// bytes), hex: part `p` is compressed when bit `0x80 >> (p & 7)` of
    /// byte `p >> 3` is set.
    pub compressed: String,
    /// The compile leaves the HP (the console is in the cyberworld, or has
    /// event flag 0x10B2: the navicust system's setup `cyberworld`).
    pub cyberworld: bool,
    /// The board's memory expansions (key item 0x61's count, which the
    /// editor sizes the board by, 0x081329B0; the compile reads none).
    pub expansions: u8,
}

impl NaviCustSetup {
    /// The list's bytes, and whether part `part` is compressed.
    pub fn decode(&self) -> Result<(Vec<u8>, [u8; 32]), String> {
        let list = unhex(&self.list)?;
        if list.len() != crate::save::NAVICUST_PARTS * 8 {
            return Err(format!("a NaviCust list of {:#x} bytes", list.len()));
        }
        let flags: [u8; 32] = unhex(&self.compressed)?.try_into().map_err(|b: Vec<u8>| format!("{:#x} bytes of compression flags", b.len()))?;
        Ok((list, flags))
    }
}

/// A player's computer-navi data block (0xE0 bytes, EXE5's 0x02034C20 by
/// side) as the halfwords it holds: the entries in order (the count at
/// +0x54) and, for each pattern an entry names, its place (`dx`, `dy`) and
/// chips (the halfwords after it to the first 0xFFFF, at most six).
pub fn tactic_block(block: &[u8]) -> Result<(Vec<u16>, Vec<(u8, i8, i8, Vec<u16>)>), String> {
    if block.len() != 0xE0 {
        return Err(format!("a computer-navi data block is 0xE0 bytes, not {:#x}", block.len()));
    }
    let half = |o: usize| u16::from_le_bytes([block[o], block[o + 1]]);
    let count = u32::from_le_bytes(block[0x54..0x58].try_into().expect("four bytes")) as usize;
    if count > nettai_battle::tactics::MAX_ENTRIES {
        return Err(format!("a computer-navi data block counts {count} entries, more than {}", nettai_battle::tactics::MAX_ENTRIES));
    }
    let entries: Vec<u16> = (0..count).map(|i| half(i * 2)).collect();
    let mut patterns: Vec<(u8, i8, i8, Vec<u16>)> = Vec::new();
    for &e in &entries {
        if e & 0x8000 == 0 || e == 0xFFFF {
            continue;
        }
        let i = (e & 0x7FFF) as usize;
        if i >= nettai_battle::tactics::MAX_PATTERNS {
            return Err(format!("a computer-navi data entry names pattern {i}, past the block's {}", nettai_battle::tactics::MAX_PATTERNS));
        }
        if patterns.iter().any(|(p, ..)| *p as usize == i) {
            continue;
        }
        let at = 0x58 + i * 16;
        let chips: Vec<u16> = (1..8).map(|k| half(at + k * 2)).take_while(|&c| c != 0xFFFF).collect();
        if chips.len() > nettai_battle::tactics::MAX_PATTERN_CHIPS {
            return Err(format!("a computer-navi pattern runs {} chips with no end", chips.len()));
        }
        patterns.push((i as u8, block[at] as i8, block[at + 1] as i8, chips));
    }
    Ok((entries, patterns))
}

/// A battle object as the recording has it (EXE6's fields).
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

/// One battle frame (EXE6's fields).
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

pub use crate::Version;

/// A round's setup, decoded.
#[derive(Clone, Debug)]
pub struct DecodedSetup {
    pub navi_stats: [NaviStats; 2],
    pub versions: [Version; 2],
    /// Both sides' regions: true for a Japanese console.
    pub japanese: [bool; 2],
    /// The settings record (stages are by EXE5's numbers: no stage content
    /// yet).
    pub settings: [u8; 0x10],
    pub battle_state: Vec<u8>,
}

impl DecodedSetup {
    /// The traced console's ROM: its side's version and region (the
    /// BattleState's local side, +0x0D), whose addresses its settings
    /// record (its RAM's) holds.
    pub fn traced_rom(&self) -> (Version, bool) {
        let local = self.battle_state[0x0D] as usize & 1;
        (self.versions[local], self.japanese[local])
    }
}

pub fn decode_setup(s: &Setup) -> Result<DecodedSetup, String> {
    if s.game != crate::ROOT {
        return Err(format!("an {} recording", s.game));
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
    let regions = [region(&s.game_regions[0])?, region(&s.game_regions[1])?];
    let battle_state = unhex(&s.battle_state)?;
    if battle_state.len() != 0xF0 {
        return Err(format!("a BattleState of {:#x} bytes", battle_state.len()));
    }
    for f in std::iter::once(&s.folder).chain(s.folders.iter()) {
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

/// An object, in the engine's terms: its pool, and EXE5's kind number in it
/// (its slot's owner: EXE5's kinds have no content keys yet).
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
            return Err(format!("frame {}: more {pool:?} objects than EXE5's {} slots", f.frame, pool_slots(pool)));
        }
        objects.push(DecodedObject { pool, kind: o.index, object: o.clone() });
    }
    let hand = |s: &str| codec::chip_hand(compat, &unhex(s)?);
    Ok(DecodedFrame { frame: f.frame, panels, objects, hands: [hand(&f.chip_blocks[0])?, hand(&f.chip_blocks[1])?] })
}

/// An exchange, decoded: both sides' stats as the custom screen sent them,
/// and the transform records (raw: EXE5's forms have no content yet).
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

// ---- Replaying a round ------------------------------------------------------

/// One round of a recording: its setup, the exchanges and its frames.
#[derive(Clone, Debug)]
pub struct Round {
    pub setup: Setup,
    pub exchanges: Vec<Exchange>,
    pub frames: Vec<Frame>,
}

/// A recording's rounds: a setup line starts one; exchanges before the
/// first setup are the first round's; frames before it (the intro's) are
/// none of a round's.
pub fn rounds(path: impl AsRef<std::path::Path>) -> Result<Vec<Round>, String> {
    let mut rounds: Vec<Round> = Vec::new();
    let mut pending = Vec::new();
    for line in read(path)? {
        match line {
            Line::Setup(s) => rounds.push(Round { setup: *s, exchanges: std::mem::take(&mut pending), frames: Vec::new() }),
            Line::Exchange(e) => match rounds.last_mut() {
                Some(r) => r.exchanges.push(e),
                None => pending.push(e),
            },
            Line::Frame(f) => {
                if let Some(r) = rounds.last_mut() {
                    r.frames.push(*f);
                }
            }
        }
    }
    Ok(rounds)
}

/// The settings record's stage: the bytes that pick it (layout, music,
/// mode, battle number, panel pattern and the actor list's address, as
/// EXE6's codec matches its stages), hex.
fn stage_bytes(settings: &[u8; 0x10]) -> String {
    [0usize, 2, 3, 5, 6, 12, 13, 14, 15].iter().map(|&i| format!("{:02x}", settings[i])).collect()
}

impl Round {
    /// The frames the engine simulates: the running and end states, from
    /// the setup's frame up to the next round's init (EXE5's battle state
    /// machine has EXE6's top states: 0 init, 4 running, 8 end).
    pub fn battle_frames(&self) -> impl Iterator<Item = &Frame> {
        self.frames.iter().filter(|f| f.frame >= self.setup.frame).take_while(|f| f.state[0] == 4 || f.state[0] == 8)
    }

    /// The link's delay: the chip lab's emulated cable's (EXE5's recordings
    /// carry none of their own).
    pub fn link_delay(&self) -> u8 {
        nettai_battle::link::Link::RECORDED_DELAY
    }

    fn frame(&self, number: u32) -> Option<&Frame> {
        let first = self.frames.first()?.frame;
        let f = self.frames.get(number.checked_sub(first)? as usize)?;
        (f.frame == number).then_some(f).or_else(|| self.frames.iter().find(|f| f.frame == number))
    }

    /// A player's buttons on a frame: what the link delivered, pressed
    /// `link_delay` frames earlier.
    pub fn joypad(&self, frame: u32, side: usize) -> u16 {
        self.frame(frame + self.link_delay() as u32).map_or(0, |f| f.input[side][0] & 0x3FF)
    }

    /// Inputs and events for the `i`th of `frames`.
    pub fn tick_inputs(&self, i: usize, frames: &[&Frame]) -> ([PlayerTick; 2], TickEvents) {
        let f = frames[i];
        let input = std::array::from_fn(|p| PlayerTick { held: self.joypad(f.frame, p) });
        let mut events = TickEvents::default();
        // The link session closed on the tick the end state moved on.
        if i > 0 && f.state[0] == 8 && f.state[1] == 4 && frames[i - 1].state[1] == 0 {
            events.link_closed = true;
        }
        (input, events)
    }

    /// Every chip the round deals with: its folders' and every frame's
    /// hands (ids, by first appearance; 0xFFFF is none, and a zeroed field,
    /// chip 0, no chip of EXE5's).
    pub fn chip_ids(&self) -> Result<Vec<u16>, String> {
        let mut ids = Vec::new();
        let mut add = |v: u16| {
            if v != 0xFFFF && v & 0x1FF != 0 && !ids.contains(&(v & 0x1FF)) {
                ids.push(v & 0x1FF);
            }
        };
        for folder in std::iter::once(&self.setup.folder).chain(self.setup.folders.iter()) {
            let b = unhex(folder)?;
            for i in 0..b.len() / 2 {
                add(u16::from_le_bytes([b[2 * i], b[2 * i + 1]]));
            }
        }
        for f in &self.frames {
            for block in &f.chip_blocks {
                let b = unhex(block)?;
                if b.len() == codec::CHIP_BLOCK {
                    for off in [0x02, 0x32] {
                        for i in 0..6 {
                            add(u16::from_le_bytes([b[off + 2 * i], b[off + 2 * i + 1]]));
                        }
                    }
                }
            }
        }
        Ok(ids)
    }

    /// What the round needs that `content` doesn't define: its chips
    /// (`chip cannon (0x001)`), EXE5's navis, forms and stage (none of
    /// which content/exe5 defines yet).
    pub fn needs(&self, content: &Content, compat: &Compat) -> Result<Vec<String>, String> {
        let d = decode_setup(&self.setup)?;
        let mut out = Vec::new();
        for id in self.chip_ids()? {
            match compat.chip(id) {
                Some(key) if content.defs.chip_by_key(&key).is_some() => {}
                Some(key) => out.push(format!("chip {key} ({id:#05x})")),
                None => out.push(format!("chip {id:#05x} (no key in compat)")),
            }
        }
        for (side, s) in d.navi_stats.iter().enumerate() {
            let navi = navi_key(s.navi);
            if !navi.as_ref().is_some_and(|k| content.defs.navi_by_key(k).is_some()) {
                out.push(format!("side {side}'s navi {} ({:#04x})", navi.as_deref().unwrap_or("with no key"), s.navi));
            }
            if s.form != 0 {
                out.push(format!("side {side}'s soul {:#04x} (EXE5's forms)", s.form));
            }
            for (what, n) in [("buster", s.raw[0x04]), ("charged shot", s.raw[0x05]), ("B+Back", s.raw[0x07]), ("A charge", s.raw[0x39])] {
                match compat.weapon(n) {
                    Ok(Some(k)) if content.defs.weapon_by_key(&k).is_none() => out.push(format!("side {side}'s {what} weapon {k}")),
                    Err(e) => out.push(format!("side {side}'s {what}: {e}")),
                    _ => {}
                }
            }
            for (what, n) in [("buster-shot program", s.raw[0x4D]), ("charged-shot program", s.raw[0x4F])] {
                match compat.projectile_variant(n) {
                    Ok(k) if content.defs.record(&k).is_none() => out.push(format!("side {side}'s {what} {k}")),
                    Err(e) => out.push(format!("side {side}'s {what}: {e}")),
                    _ => {}
                }
            }
            match compat.barrier(s.first_barrier) {
                Ok(Some(k)) if content.defs.record(&k).is_none() => out.push(format!("side {side}'s first barrier {k}")),
                Err(e) => out.push(format!("side {side}'s first barrier: {e}")),
                _ => {}
            }
        }
        // The chips of both players' tactics.
        if let Some(lists) = &self.setup.ai_lists {
            for (side, l) in lists.iter().enumerate() {
                let (entries, patterns) = tactic_block(&unhex(l)?).map_err(|e| format!("side {side}'s tactics: {e}"))?;
                let chips = entries
                    .iter()
                    .filter(|&&e| e & 0x8000 == 0 && e != 0)
                    .copied()
                    .chain(patterns.iter().flat_map(|(.., c)| c.iter().copied()));
                for id in chips {
                    match compat.chip(id) {
                        Some(key) if content.defs.chip_by_key(&key).is_some() => {}
                        Some(key) => out.push(format!("chip {key} ({id:#05x}, side {side}'s tactics)")),
                        None => out.push(format!("chip {id:#05x} (side {side}'s tactics: no key in compat)")),
                    }
                }
            }
        }
        let st = &d.settings;
        let actor_list = u32::from_le_bytes([st[12], st[13], st[14], st[15]]);
        let (version, japanese) = d.traced_rom();
        match compat.stage(st[0], actor_list, version, japanese) {
            Some(k) if content.defs.stage_by_key(&k).is_some() => {}
            Some(k) => out.push(format!("the stage {k} (settings {})", stage_bytes(st))),
            None => out.push(format!("the stage (settings {}: no EXE5 netbattle stage)", stage_bytes(st))),
        }
        if content.assets.pack(crate::ROOT).is_none() {
            out.push("EXE5's pack".into());
        }
        out.dedup();
        Ok(out)
    }

    /// The engine's starting point for this round on `content`: what the
    /// round needs is defined ([`Round::needs`]), then EXE5's records go
    /// into the engine's: the settings record's stage and background, both
    /// NaviStats ([`navi_stats`]), the folders, the RNGs, the set's score,
    /// both players on EXE5's rules.
    pub fn round_setup(&self, content: &Content, compat: &Compat) -> Result<RoundSetup, String> {
        // EXE5's light and dark system (content/exe5/rules/light-dark).
        const LIGHT_DARK: &str = "light-dark";
        let needs = self.needs(content, compat)?;
        if !needs.is_empty() {
            return Err(format!("content lacks {}", needs.join(", ")));
        }
        let d = decode_setup(&self.setup)?;
        let bs = &d.battle_state;
        let st = &d.settings;
        let actor_list = u32::from_le_bytes([st[12], st[13], st[14], st[15]]);
        let (version, japanese) = d.traced_rom();
        let stage = compat.stage(st[0], actor_list, version, japanese).and_then(|k| content.defs.stage_by_key(&k)).expect("needs saw the stage");
        let pack = content.assets.pack(crate::ROOT).expect("needs saw the pack");
        let background = nettai_battle::content::BackgroundId(
            content
                .assets
                .number_handle(nettai_content_api::AssetKind::Background, pack, st[4] as u16)
                .ok_or_else(|| format!("EXE5's pack has no background {:#04x}", st[4]))?,
        );
        let settings = nettai_battle::BattleSettings { stage, background, effects: u32::from_le_bytes([st[8], st[9], st[10], st[11]]) };
        if content.defs.ruleset().is_none() {
            return Err("the content has no ruleset (EXE5's)".into());
        }
        let local = bs[0x0D] & 1;
        // A side whose setup carries its console's NaviCust (MegaMan's): the
        // engine compiles it and applies the console's patch cards over the
        // stats EXE5's reset leaves ([`codec::reset`]), as a match is set up.
        // (The recorded stats are what the battle's start made of them: the
        // reload, 0x0813F97C.)
        let compiled = |side: usize| self.setup.navicusts.is_some() && d.navi_stats[side].navi == 0;
        let navicust_of = |side: usize| -> Result<Option<NaviCust>, String> {
            let Some(n) = self.setup.navicusts.as_ref().filter(|_| compiled(side)).map(|n| &n[side]) else { return Ok(None) };
            let (list, flags) = n.decode()?;
            // The board is the recorded ExpMemry's (the one the save's
            // parts were placed on), or the game's largest when its rules
            // have fewer sizes than that.
            let largest = content.rules().navicust.boards.len().saturating_sub(1) as u8;
            navicust(content, compat, &list, n.expansions.min(largest), |part| flags[(part >> 3) as usize] & (0x80 >> (part & 7)) != 0).map(Some)
        };
        let cards_of = |side: usize| -> Result<nettai_battle::patch_cards::PatchCards, String> {
            let Some(lists) = self.setup.patch_cards.as_ref().filter(|_| compiled(side)) else { return Ok(Default::default()) };
            let list: Vec<(u8, bool)> = lists[side].iter().map(|&b| (b & 0x7F, b & 0x80 == 0)).collect();
            patch_cards(content, compat, d.versions[side], &list)
        };
        let players = [0u8, 1].map(|side| -> Result<PlayerSetup, String> {
            let folder = battle_folder(content, compat, &unhex(&self.setup.folders[side as usize])?, side == local && bs[0x17] != 0)?;
            // (The console's counter before the round's first tick: one
            // less than on the setup's frame.)
            let frames = (self.setup.frame_counter as u32).wrapping_sub(1) & 0xFFFF;
            Ok(PlayerSetup {
                folder,
                joypad_phase: self.setup.joypad_phases[side as usize],
                navi_level: None,
                sp_times: Default::default(),
                // (The save's emotion window glitch, which a recording
                // has, is no setup's: EXE5's rules make it. A compiled
                // side's is its compile's and its cards', which `start`
                // checks against the console's. A side without a recorded
                // NaviCust has it from the stats' NaviCust bugs.)
                console: ConsoleSetup {
                    rng: if side == local { self.setup.rng1 } else { self.setup.rng1s.map_or(0, |r| r[side as usize & 1]) },
                    tag_pair: None,
                    frames,
                },
                rules: Vec::new(),
                // (Without a recorded NaviCust, the stats are the battle's
                // start's: nothing is compiled over them.)
                patch_cards: cards_of(side as usize)?,
                navicust: navicust_of(side as usize)?,
                tactics: match &self.setup.ai_lists {
                    Some(lists) => tactics(content, compat, &unhex(&lists[side as usize])?)?,
                    None => Default::default(),
                },
            })
        });
        let [mut p0, mut p1] = players;
        // Each side's light and dark MegaMan: his save's value (NaviStats
        // +0x44), EXE5's light and dark system's setup. (Hub Style, +0x4C,
        // is the stats': `navi_stats`.)
        for (p, stats) in [(&mut p0, &d.navi_stats[0]), (&mut p1, &d.navi_stats[1])] {
            if let Ok(p) = p {
                p.set_rule(content, LIGHT_DARK, "karma", nettai_content_api::Value::Int(stats.light_dark.0 as i64))?;
            }
        }
        // Each side's souls: its version's six (Team ProtoMan's 1 to 6,
        // Team Colonel's 7 to 12: 0x08024BF0's flags), those the content
        // has, into the souls system's setup.
        for (side, p) in [&mut p0, &mut p1].into_iter().enumerate() {
            if let Ok(p) = p {
                let version = d.versions[side];
                let souls: Vec<nettai_battle::rules::Fact> = (0..content.defs.forms.len() as u16)
                    .map(nettai_content_api::FormHandle)
                    .filter(|&f| content.form(f).soul.as_ref().is_some_and(|s| version.soul_flag(s.number).is_some()))
                    .map(|f| nettai_battle::rules::Fact::Value(nettai_content_api::Value::Def(nettai_content_api::Registry::Form, f.0)))
                    .collect();
                p.set_fact(content, "souls", &souls)?;
            }
        }
        // Whether each compiled side's compile leaves the HP: the navicust
        // system's setup.
        for (side, p) in [&mut p0, &mut p1].into_iter().enumerate() {
            if let (Ok(p), Some(n), true) = (p, &self.setup.navicusts, compiled(side)) {
                let took = p.set_fact(content, "cyberworld", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Bool(n[side].cyberworld))])?;
                if took == 0 {
                    return Err("no system of EXE5's rules takes `cyberworld`".into());
                }
            }
        }
        let stats = |side: usize| -> Result<EngineNaviStats, String> {
            if compiled(side) {
                navi_stats(content, compat, &codec::navi_stats(&codec::reset(&d.navi_stats[side].raw))?)
            } else {
                navi_stats(content, compat, &d.navi_stats[side])
            }
        };
        Ok(RoundSetup {
            content: content.hash(),
            settings,
            navi_stats: [stats(0)?, stats(1)?],
            rng: self.setup.rng2,
            local_side: bs[0x0D],
            score: nettai_battle::SetScore { wins: bs[0x18], losses: bs[0x19], round: bs[0x1A], max_combo: bs[0x1B] },
            later_stages: [nettai_battle::Stage { stage, background }; 2],
            low_hp_music_latched: bs[0x20] | bs[0x21] != 0,
            players: [p0?, p1?],
            link_delay: self.link_delay(),
        })
    }

    /// The round's battle at its start on `content`, with the counters its
    /// init carried in (BattleState +0x60, +0x64).
    pub fn start(&self, content: Arc<Content>, compat: &Compat) -> Result<Battle, String> {
        let setup = self.round_setup(&content, compat)?;
        let d = decode_setup(&self.setup)?;
        let bs = d.battle_state;
        let mut b = Battle::new(setup, content);
        // A side whose NaviCust the engine compiled: its emotion window
        // glitches as the console's does (the flag its window's start reads,
        // 0x0813F650, which the reload set).
        if let (Some(_), Some(glitches)) = (&self.setup.navicusts, self.setup.emotion_window_glitches) {
            for side in 0..2usize {
                if d.navi_stats[side].navi == 0 && b.consoles[side].emotion_window_glitch != glitches[side] {
                    return Err(format!(
                        "side {side}'s compile and cards leave the emotion window {}, the console's {}",
                        if b.consoles[side].emotion_window_glitch { "glitching" } else { "steady" },
                        if glitches[side] { "glitches" } else { "is steady" }
                    ));
                }
            }
        }
        b.round.frames = u32::from_le_bytes(bs[0x60..0x64].try_into().unwrap());
        b.round.ticks = u32::from_le_bytes(bs[0x64..0x68].try_into().unwrap());
        Ok(b)
    }
}

/// A player's tactics from their computer-navi data block (`tactic_block`):
/// its chips by key.
fn tactics(content: &Content, compat: &Compat, block: &[u8]) -> Result<nettai_battle::tactics::Tactics, String> {
    use nettai_battle::tactics::{Tactic, TacticPattern, Tactics};
    let (entries, patterns) = tactic_block(block)?;
    let chip = |id: u16| -> Result<nettai_content_api::ChipHandle, String> {
        compat
            .chip(id)
            .and_then(|k| content.defs.chip_by_key(&k))
            .ok_or_else(|| format!("the tactics' chip {id:#05x} isn't in the content"))
    };
    let mut out = Tactics::default();
    for e in entries {
        out.entries.push(match e {
            0 => Tactic::Nothing,
            0xFFFF => Tactic::Empty,
            e if e & 0x8000 != 0 => Tactic::Pattern((e & 0x7FFF) as u8),
            e => Tactic::Chip(chip(e)?),
        });
    }
    // The patterns in their places (the ones no entry names, empty).
    let n = patterns.iter().map(|(i, ..)| *i as usize + 1).max().unwrap_or(0);
    out.patterns = vec![TacticPattern::default(); n];
    for (i, dx, dy, chips) in patterns {
        out.patterns[i as usize] = TacticPattern { dx, dy, chips: chips.into_iter().map(chip).collect::<Result<_, _>>()? };
    }
    Ok(out)
}

/// A battle folder (0x50 bytes: 30 chips, code << 9 | id, 0xFFFF none) by
/// EXE5's chips' handles.
fn battle_folder(content: &Content, compat: &Compat, b: &[u8], regular_pending: bool) -> Result<BattleFolder, String> {
    let mut chips = [None; nettai_battle::custom::folder::FOLDER_SIZE];
    for (i, c) in chips.iter_mut().enumerate() {
        let v = u16::from_le_bytes([b[2 * i], b[2 * i + 1]]);
        if v == 0xFFFF {
            continue;
        }
        let id = v & 0x1FF;
        let key = compat.chip(id).ok_or_else(|| format!("chip {id:#05x} has no key"))?;
        let h = content.defs.chip_by_key(&key).ok_or_else(|| format!("the content has no {key}"))?;
        *c = Some(FolderChip::new(h, ChipCode((v >> 9) as u8)));
    }
    Ok(BattleFolder { chips, regular_pending })
}

/// A side's stats in the engine's terms: EXE5's NaviStats fields where the
/// engine has them (docs/design/exe5-map.md §3.3, §13), its navi, weapons,
/// programs and first barrier by compat (records.toml); MegaMan in the base
/// form (EXE5's souls are its forms, to come); the NaviCust's bugs and what
/// it does to drops and encounters at EXE6's offsets, the ones EXE5's bugs'
/// routine writes (0x08140054's: +0x12, +0x13, +0x14, +0x15, +0x16, +0x1A,
/// +0x24, +0x26, +0x28, +0x31, +0x54) and the patch cards' (0x081382B8's:
/// the HP drains, +0x18 and +0x19, and the chip recovery, +0x50; the rest of
/// EXE6's bug bytes, which only a battle's hits write, none at the start);
/// what EXE5 has none of (EXE6's Beast Out counter, the sun, the version)
/// none.
pub fn navi_stats(content: &Content, compat: &Compat, s: &NaviStats) -> Result<EngineNaviStats, String> {
    let navi_key = navi_key(s.navi).ok_or_else(|| format!("navi {:#04x} has no key", s.navi))?;
    let navi = content.defs.navi_by_key(&navi_key).ok_or_else(|| format!("the content has no {navi_key}"))?;
    let weapon = |n: u8| -> Result<Option<WeaponHandle>, String> {
        match compat.weapon(n)? {
            None => Ok(None),
            Some(k) => content.defs.weapon_by_key(&k).map(Some).ok_or_else(|| format!("the content has no weapon {k}")),
        }
    };
    let variant = |n: u8| -> Result<Option<RecordHandle>, String> {
        let k = compat.projectile_variant(n)?;
        content.defs.record(&k).map(Some).ok_or_else(|| format!("the content has no projectile variant {k}"))
    };
    let first_barrier = match compat.barrier(s.first_barrier)? {
        None => None,
        Some(k) => Some(content.defs.record(&k).ok_or_else(|| format!("the content has no barrier {k}"))?),
    };
    let r = &s.raw;
    let base = content.base_form_for(navi);
    Ok(EngineNaviStats {
        attack: s.attack,
        rapid: s.rapid,
        charge: s.charge,
        first_barrier,
        gauge_speed: s.gauge_speed,
        reg_up: s.reg_up,
        custom_level: s.custom_level,
        mega_level: s.mega_level,
        giga_level: s.giga_level,
        support: s.support,
        mood: s.mood,
        element: s.element,
        starting_form: base,
        float_shoes: s.float_shoes,
        air_shoes: s.air_shoes,
        undershirt: s.undershirt,
        super_armor: s.super_armor,
        version: 0,
        beast_out_counter: 0,
        sun: false,
        chip_drops: r[0x26],
        encounters: r[0x28],
        navi,
        navi_variant: s.navi_variant,
        form: base,
        folder: 0,
        folder_reg: [0xFF, 0xFF],
        max_base_hp: s.max_base_hp,
        hp: s.hp,
        max_hp: s.max_hp,
        chip_recovery: u16::from_le_bytes([r[0x50], r[0x51]]),
        folder_tags: [[0xFF, 0xFF], [0xFF, 0xFF]],
        chip_shuffle: false,
        number_open: false,
        hub_style: s.hub_style,
        soul_turn_bonus: r[0x32] as i8,
        weapons: NaviWeapons {
            buster: weapon(r[0x04])?,
            charge_shot: weapon(r[0x05])?,
            back_special: weapon(r[0x07])?,
            a_charge: weapon(r[0x39])?,
            mode9_a: None,
            buster_shot: variant(r[0x4D])?,
            charge_shot_kind: variant(r[0x4F])?,
            back_special_damage: 0,
        },
        bugs: NaviCustBugs {
            panel_trail_kind: r[0x12],
            panel_trail_level: r[0x13],
            buster_blanks: r[0x14],
            buster_charged: r[0x15],
            hit_status: r[0x16],
            hp_drain: r[0x18],
            custom_drain: r[0x19],
            battle_start: r[0x1A],
            emotion: r[0x24],
            processing: r[0x31],
            custom_damage: u16::from_le_bytes([r[0x54], r[0x55]]),
            ..Default::default()
        },
    })
}

/// A save's NaviCust in the engine's terms (EXE5's list: [`crate::save::NAVICUST_PARTS`]
/// parts of 8 bytes, +0 the part id, +2 the center's column, +3 its row, +4
/// the quarter turns clockwise; +5, the editor's compression mark, isn't
/// what the compile reads). EXE5's 5x5 board is the middle of the engine's
/// 7x7 grid (content/exe5/rules/navicust/board.luau), a cell one column and
/// one row on. A part is compressed when `compressed` says so of its part id
/// (event flag 0x1EC0 + the id, which 0x0813EEFC reads). The list's empty
/// entries (id 0) are left out, the others kept in order, on a board with
/// `expansions` (the save's ExpMemry: [`crate::save::Save::expansions`]).
pub fn navicust(content: &Content, compat: &Compat, list: &[u8], expansions: u8, compressed: impl Fn(u8) -> bool) -> Result<NaviCust, String> {
    let mut parts = Vec::new();
    for e in list.chunks_exact(8) {
        let Some((key, color)) = compat.navicust_part(e[0])? else { continue };
        let program = content.defs.navicust_program_by_key(key).ok_or_else(|| format!("the content has no NaviCust program {key}"))?;
        if e[2] > 4 || e[3] > 4 || e[4] > 3 {
            return Err(format!("NaviCust part {:#04x} at column {}, row {}, turned {}: off EXE5's 5x5 board", e[0], e[2], e[3], e[4]));
        }
        parts.push(PlacedProgram { program, color, x: e[2] + 1, y: e[3] + 1, rotation: e[4], compressed: compressed(e[0]) });
    }
    NaviCust::new(&parts, expansions)
}

/// A save's patch cards (each card's number and whether it is switched on,
/// in the list's order: [`crate::save::Save::patch_cards`]) in the engine's
/// terms, by `version`'s numbers (compat's patch-cards.toml).
pub fn patch_cards(content: &Content, compat: &Compat, version: crate::Version, list: &[(u8, bool)]) -> Result<nettai_battle::patch_cards::PatchCards, String> {
    let mut cards = Vec::new();
    for &(n, enabled) in list {
        let key = compat.patch_card(n, version)?;
        let card = content.defs.patch_card_by_key(key).ok_or_else(|| format!("the content has no patch card {key}"))?;
        cards.push(nettai_battle::patch_cards::InstalledCard { card, enabled });
    }
    nettai_battle::patch_cards::PatchCards::new(&cards)
}

/// EXE5's navi numbers' keys (NaviStats +0x29): MegaMan's. (The Team
/// Battle's navis come with their content.)
pub fn navi_key(n: u8) -> Option<String> {
    (n == 0).then(|| "megaman".to_string())
}

/// Differences between the engine and an EXE5 frame: the state machine and
/// its counters, the simulation RNG, the gauge, the panels (by EXE5's
/// numbers, through compat) and the objects, as EXE6's comparison sees them:
/// each object's pool and kind (EXE5's numbers, through compat's
/// kinds.toml), header flags, state, action (a navi's by EXE5's numbers:
/// `Compat::navi_action`), phase and its init byte, panel, side, HP,
/// position, timer, animation and its collision's status flags.
pub fn compare(b: &Battle, f: &Frame, compat: &Compat) -> Vec<String> {
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
    check("gauge", format!("{:#x}", b.gauge.value), format!("{:#x}", f.gauge));
    // The banner: its task (bit 15 of the HUD's tasks) and, while it shows,
    // the number the traced console's record holds (its +1: the banner
    // 0x0801B02E started, 0 for a telop), which for a result's is the
    // console's own (0x080074D2, 0x0800758A).
    let task = (f.hud_tasks >> 15) & 1 != 0;
    check("banner", (b.banner.active as u8).to_string(), (task as u8).to_string());
    if let (Some(id), true) = (b.banner_for(r.local_side), task) {
        let ours = match b.telop_for(r.local_side) {
            Some(_) => Some(0),
            None => b.content.assets.number(nettai_content_api::AssetKind::Banner, id.0).map(|n| n.id as u8),
        };
        let theirs = unhex(&f.banner).ok().and_then(|x| x.get(1).copied());
        let show = |n: Option<u8>| n.map_or("none".to_string(), |n| format!("{n:#04x}"));
        check("banner number", show(ours), show(theirs));
    }
    let number = |t: nettai_battle::field::PanelType| {
        compat.panels.keys().copied().find(|&n| compat.panel_type(n) == Ok(Some(t)))
    };
    let panels: Vec<String> = (1..=3)
        .flat_map(|y| (1..=6).map(move |x| (x, y)))
        .map(|(x, y)| {
            let p = b.field.panel(x, y).expect("a field panel");
            match number(p.kind) {
                Some(n) => format!("[{n}, {}]", p.alliance),
                None => format!("[{:?}, {}]", p.kind, p.alliance),
            }
        })
        .collect();
    let theirs: Vec<String> = f.panels.iter().map(|[t, a]| format!("[{t}, {a}]")).collect();
    check("panels", panels.join(", "), theirs.join(", "));
    let pool_type = |p: Pool| match p {
        Pool::Actor => 1,
        Pool::Attack => 3,
        Pool::Effect => 4,
    };
    // Each object as its pool and EXE5's kind number (the engine's kinds by
    // compat's kinds.toml), panel, side, HP and position: a position the
    // kind leaves as register garbage is skipped on both sides, X and Y the
    // engine doesn't know too (matched by list position).
    let entries: Vec<Option<&crate::KindEntry>> =
        b.objects.in_order().map(|o| compat.kinds.get(&b.content.defs.kind(b.objects.get(o).kind).key)).collect();
    let skip = |i: usize, flags: u8| -> (bool, bool) {
        let Some(Some(k)) = entries.get(i) else { return (false, false) };
        let garbage = k.scratch_position || (k.scratch_position_without_sprite && flags & nettai_battle::object::flags::NO_SPRITE_UPDATE != 0);
        (garbage, k.scratch_z_fraction)
    };
    let panel = |i: usize, p: [u8; 2]| -> String {
        match entries.get(i) {
            Some(Some(k)) if k.scratch_panel => "-".to_string(),
            _ => format!("{p:?}"),
        }
    };
    let status_field = |i: usize, s: String| -> String {
        match entries.get(i) {
            Some(Some(k)) if k.scratch_status => "-".to_string(),
            _ => s,
        }
    };
    let pos = |p: [i32; 3], garbage: bool, xy_unknown: bool, z_fraction: bool| {
        if garbage {
            "-".to_string()
        } else if xy_unknown {
            format!("-,-,{}", p[2])
        } else if z_fraction {
            format!("{},{},{}+?", p[0], p[1], p[2] >> 16)
        } else {
            format!("{},{},{}", p[0], p[1], p[2])
        }
    };
    // Each object's fields, by name: the list's shape (pool and kind, in
    // update order) first, then each object's fields one by one, so that a
    // difference names its field.
    type Fields = Vec<(&'static str, String)>;
    let ours: Vec<Fields> = b
        .objects
        .in_order()
        .enumerate()
        .map(|(i, o)| {
            let x = b.objects.get(o);
            let key = &b.content.defs.kind(x.kind).key;
            let kind = match entries[i] {
                Some(k) => format!("type {} #{:#04x}", pool_type(o.pool), k.index),
                None => format!("type {} {key} (no EXE5 number)", pool_type(o.pool)),
            };
            let (garbage, zf) = skip(i, x.flags);
            let xy = nettai_battle::kinds::effect::xy_unknown(b, o);
            let action = match compat.navi_action(b, o) {
                Ok(n) => format!("{n:#04x}"),
                Err(e) => format!("? ({e})"),
            };
            let status = x.collision.map(|c| b.collision.get(c).f1).unwrap_or(0);
            // NumberMan's face on NumberSoul's image copies the image's
            // spawn registers for a tick (0x0801201E's read of the transform
            // record): its +6, the turn's arm chip by number, where the
            // engine's image has 0xFFFF whatever the chip (rules/souls/image).
            let mut at = [x.pos.x, x.pos.y, x.pos.z];
            if key.as_str() == "numbersoul/layer" && at[1] == 0xFFFF {
                if let Some(n) = arm_chip_number(b, compat, x.alliance) {
                    at[1] = n as i32;
                }
            }
            vec![
                ("kind", kind),
                ("flags", format!("{:#04x}", x.flags)),
                ("state", format!("{:#04x}", x.state)),
                ("action", action),
                ("phase", format!("{:#04x}", x.phase)),
                ("phase init", format!("{:#04x}", x.phase_init)),
                ("panel", panel(i, [x.panel.x, x.panel.y])),
                ("side", x.alliance.to_string()),
                ("hp", format!("{}/{}", x.hp, x.max_hp)),
                ("pos", pos(at, garbage, xy, zf)),
                ("timer", x.timer.to_string()),
                ("anim", x.anim.to_string()),
                ("status", status_field(i, format!("{status:#x}"))),
            ]
        })
        .collect();
    let order: Vec<_> = b.objects.in_order().collect();
    let theirs: Vec<Fields> = f
        .objects
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let (garbage, zf) = skip(i, o.flags);
            let xy = order.get(i).is_some_and(|&r| nettai_battle::kinds::effect::xy_unknown(b, r));
            vec![
                ("kind", format!("type {} #{:#04x}", o.kind, o.index)),
                ("flags", format!("{:#04x}", o.flags)),
                ("state", format!("{:#04x}", o.state[0])),
                ("action", format!("{:#04x}", o.state[1])),
                ("phase", format!("{:#04x}", o.state[2])),
                ("phase init", format!("{:#04x}", o.state[3])),
                ("panel", panel(i, o.panel)),
                ("side", o.alliance.to_string()),
                ("hp", format!("{}/{}", o.hp, o.max_hp)),
                ("pos", pos(o.pos, garbage, xy, zf)),
                ("timer", o.timer.to_string()),
                ("anim", o.anim.to_string()),
                ("status", status_field(i, format!("{:#x}", o.status))),
            ]
        })
        .collect();
    let line = |o: &Fields| o.iter().map(|(k, v)| if *k == "kind" { v.clone() } else { format!("{k} {v}") }).collect::<Vec<_>>().join(", ");
    let shape = |l: &[Fields]| l.iter().map(|o| o[0].1.clone()).collect::<Vec<_>>();
    if shape(&ours) != shape(&theirs) {
        let all = |l: &[Fields]| l.iter().map(line).collect::<Vec<_>>().join("\n           ");
        check("objects", format!("\n    ours   {}", all(&ours)), format!("\n    theirs {}", all(&theirs)));
    } else {
        for (i, (a, t)) in ours.iter().zip(&theirs).enumerate() {
            for ((name, x), (_, y)) in a.iter().zip(t).skip(1) {
                if x != y {
                    check(&format!("object {name}"), format!("{x} (object {i}, {})", a[0].1), y.clone());
                }
            }
        }
    }
    d
}

/// Side `side`'s arm chip for the turn (EXE5's souls system's `arm_chip`:
/// ColonelSoul's Arm Change, the transform record's +6), by EXE5's number.
fn arm_chip_number(b: &Battle, compat: &Compat, side: u8) -> Option<u16> {
    let (schema, state) = b.system_state(side, "souls")?;
    let nettai_content_api::FieldValue::Ref(Some((nettai_content_api::Registry::Chip, h))) = state.get(schema, schema.index_of("arm_chip")?) else {
        return None;
    };
    let key = &b.content.defs.chips.get(h as usize)?.key;
    compat.chips.get(key).map(|c| c.id)
}

/// How far a round's replay got.
#[derive(Clone, Debug)]
pub struct Replay {
    /// The round's battle frames.
    pub frames: usize,
    /// The frames that matched before the replay stopped.
    pub matched: usize,
    /// Why it stopped (none: every frame matched).
    pub stopped: Option<Stop>,
}

#[derive(Clone, Debug)]
pub enum Stop {
    /// The round's setup: what the engine's needs that isn't there.
    Setup(String),
    /// The engine panicked on this frame.
    Panic { frame: u32, message: String },
    /// The engine differs from the recording on this frame.
    Differs { frame: u32, differences: Vec<String> },
}

/// Replay a round on `content`: its setup, then each battle frame ticked
/// and compared, up to the first difference.
pub fn run_round(round: &Round, content: &Arc<Content>, compat: &Compat) -> Replay {
    let frames: Vec<&Frame> = round.battle_frames().collect();
    let mut replay = Replay { frames: frames.len(), matched: 0, stopped: None };
    let mut b = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| round.start(content.clone(), compat))) {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => {
            replay.stopped = Some(Stop::Setup(e));
            return replay;
        }
        Err(e) => {
            replay.stopped = Some(Stop::Panic { frame: round.setup.frame, message: panic_message(&*e) });
            return replay;
        }
    };
    // The recording console's RNG1 (the trace's `rng1`), sampled after the
    // battle's update and before the main loop's draw: the engine after
    // frame N is the trace at N + 1. Draws the console makes after the
    // sample (an emotion window's flicker, a camera shake's) make the two
    // differ for a frame or a few, then agree again; a difference still
    // there at the round's end is the model's, and stops the round at the
    // frame it began; one that begins on the last frame compared (a flicker's
    // draw as the recording ends) has no frame left to agree on, and is
    // left.
    let local = b.setup.local_side as usize & 1;
    let mut rng1_since: Option<(u32, u32, u32)> = None;
    for i in 0..frames.len() {
        let (input, events) = round.tick_inputs(i, &frames);
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.tick(&input, events))) {
            replay.stopped = Some(Stop::Panic { frame: frames[i].frame, message: panic_message(&*e) });
            return replay;
        }
        let differences = compare(&b, frames[i], compat);
        if !differences.is_empty() {
            replay.stopped = Some(Stop::Differs { frame: frames[i].frame, differences });
            return replay;
        }
        if let Some(next) = frames.get(i + 1) {
            let engine = b.consoles[local].rng.state;
            if engine == next.rng1 {
                rng1_since = None;
            } else if rng1_since.is_none() {
                rng1_since = Some((frames[i].frame, engine, next.rng1));
            }
        }
        replay.matched += 1;
    }
    let last_compared = frames.len().checked_sub(2).map(|i| frames[i].frame);
    if let Some((frame, engine, trace)) = rng1_since.filter(|&(frame, _, _)| Some(frame) != last_compared) {
        let differences = vec![format!(
            "rng1: the recording console's RNG1 differs from frame {frame} to the round's end (engine {engine:#010x}, trace {trace:#010x} at frame {})",
            frame + 1
        )];
        replay.stopped = Some(Stop::Differs { frame, differences });
    }
    replay
}

/// A panic's message.
pub fn panic_message(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
}
