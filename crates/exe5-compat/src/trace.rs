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
use nettai_battle::rules::Fact;
use nettai_battle::setup::{NaviCustBugs, NaviWeapons};
use nettai_battle::{Battle, Content, NaviStats as EngineNaviStats, PlayerTick, RoundSetup, TickEvents};
use nettai_content_api::{RecordHandle, Registry, Value, WeaponHandle};
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
    /// Both players' auto battle data as the link exchanged it (0xE0
    /// bytes each, side 0's first, hex: 0x02034C20, the blocks the send
    /// shuffled and counted). What the consoles played; the round's setup
    /// is `auto_battle` and `send_rng2`, which the rules' send turns into
    /// it. Older recordings have none.
    #[serde(default)]
    pub ai_lists: Option<[String; 2]>,
    /// Both players' auto battle data as their saves hold it (0xE0 bytes
    /// each, side 0's first, hex: save +0x554C, before the send), the
    /// round's setup's `auto_battle_places` and `auto_battle_records`; and
    /// the RNG2 the send starts from (0x0802C7BE: 84 draws, both consoles
    /// alike, then the fight's `rng2`), the round's battle RNG, from which
    /// the rules' send (content/exe5/rules/auto_battle/block.luau) arrives
    /// at the recorded blocks and `rng2`. Recordings older than these have
    /// none, and are refused (the verification workspace's
    /// tools/exe5/convert_send.py states them).
    #[serde(default)]
    pub auto_battle: Option<[String; 2]>,
    #[serde(default)]
    pub send_rng2: Option<u32>,
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
    /// Both players' team navi levels (0x0203C870, a word a side, from each
    /// console's init block: the count of its save's event flags 0x300 to
    /// 0x305, 0 to 6), which the team navis' attacks read their damage by.
    /// Older recordings have none (MegaMan reads no level).
    #[serde(default)]
    pub navi_levels: Option<[u32; 2]>,
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
    /// Whether the console's own compile left the HP: it is in the
    /// cyberworld (its area from 0x80), or has event flag 0x10B2. What the
    /// recording says of its console, which no rule reads: the engine's
    /// compile ends the one way, the real world's, which a link battle
    /// can't tell from the other (content/exe5/rules/navicust). Verify's
    /// compile test reads it to know which bytes of the console's block
    /// the other ending wrote.
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

/// A player's auto battle data block as their save holds it (0xE0 bytes, a
/// recording's `auto_battle`: `save::AutoBattleBlock`, its count and last
/// eight bytes left out): its places to the last one that isn't empty, and
/// its eight pattern records. A pattern entry names one of the eight.
pub fn saved_auto_battle_block(block: &[u8]) -> Result<(Vec<u16>, [crate::save::AutoBattlePattern; crate::save::AUTO_BATTLE_PATTERNS]), String> {
    use crate::save::{AUTO_BATTLE_EMPTY, AUTO_BATTLE_PATTERN, AUTO_BATTLE_PATTERNS, AutoBattleBlock};
    let read = AutoBattleBlock::read(block)?;
    let used = read.places.iter().rposition(|&p| p != AUTO_BATTLE_EMPTY).map_or(0, |i| i + 1);
    let places = read.places[..used].to_vec();
    for &e in &places {
        let i = (e & !AUTO_BATTLE_PATTERN) as usize;
        if e & AUTO_BATTLE_PATTERN != 0 && e != AUTO_BATTLE_EMPTY && i >= AUTO_BATTLE_PATTERNS {
            return Err(format!("an auto battle data place names pattern {i}, past the block's {AUTO_BATTLE_PATTERNS}"));
        }
    }
    Ok((places, read.patterns))
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
    /// The traced console's ROM (its version, and whether it is Japanese):
    /// `rounds` gives it its round's setup's; none, Team ProtoMan US's.
    #[serde(skip)]
    pub console: Option<(Version, bool)>,
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
    // (Each frame the traced console's ROM, by its round's setup.)
    let mut console = None;
    for line in read(path)? {
        match line {
            Line::Setup(s) => {
                console = decode_setup(&s).ok().map(|d| d.traced_rom());
                rounds.push(Round { setup: *s, exchanges: std::mem::take(&mut pending), frames: Vec::new() })
            }
            Line::Exchange(e) => match rounds.last_mut() {
                Some(r) => r.exchanges.push(e),
                None => pending.push(e),
            },
            Line::Frame(mut f) => {
                if let Some(r) = rounds.last_mut() {
                    f.console = console;
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

    /// The link's delay: the chip lab's emulated cable's, which delivered
    /// each console's packet 4 ticks after it went out (EXE5's recordings
    /// carry none of their own).
    pub fn link_delay(&self) -> u8 {
        4
    }

    fn frame(&self, number: u32) -> Option<&Frame> {
        let first = self.frames.first()?.frame;
        let f = self.frames.get(number.checked_sub(first)? as usize)?;
        (f.frame == number).then_some(f).or_else(|| self.frames.iter().find(|f| f.frame == number))
    }

    /// A player's buttons on a frame as the original's fight saw them:
    /// what the link delivered, which the trace records (pressed
    /// `link_delay` frames earlier). The engine has no link: its fight and
    /// both custom screens read them, so its screens run `link_delay`
    /// frames behind the original's.
    pub fn joypad(&self, frame: u32, side: usize) -> u16 {
        self.frame(frame).map_or(0, |f| f.input[side][0] & 0x3FF)
    }

    /// The buttons the engine is fed for side `side` on the `i`th of
    /// `frames`, so that it plays as the original did, with no link of its
    /// own. The original's fight read the buttons the link delivered,
    /// `link_delay` frames late (what the trace records, [`Round::joypad`]),
    /// and its custom screen read the joypad at once; the engine's fight and
    /// screens read the same buttons. So the engine is fed:
    ///
    /// - while the fight runs, what the original's fight saw;
    /// - from a custom screen's opening to the side's OK, what the
    ///   original's screen saw (the buttons `link_delay` frames on), so the
    ///   screen plays as the original's;
    /// - for `link_delay` frames from the OK, the buttons of the frame
    ///   before it; then again what the fight sees. The OK reaches the
    ///   engine's screen `link_delay` frames late, so its result goes out
    ///   that much later and, its words over, arrives on the original's
    ///   frame (the screen takes no buttons from its OK on, and the fight
    ///   is paused until the results are in).
    pub fn fed(&self, i: usize, frames: &[&Frame], side: usize) -> u16 {
        let d = self.link_delay() as u32;
        let frame = frames[i].frame;
        let Some(opened) = self.screen_opened(frame) else { return self.joypad(frame, side) };
        match self.screen_ok(opened, side) {
            Some(ok) if frame >= ok + d => self.joypad(frame, side),
            Some(ok) if frame >= ok => self.joypad((ok + d).saturating_sub(1), side),
            _ => self.joypad(frame + d, side),
        }
    }

    /// How many frames late the engine's custom screen of the recording
    /// console's side is on the `i`th of `frames`: `link_delay` from its OK
    /// until the fight resumes ([`Round::fed`]: its OK came that late), else
    /// none. What the screen does then from its OK on (the OK's sound, the
    /// slide-out) is the original's that many frames earlier; what it began
    /// before (a dark chip's fade, stepping on) the original's on the same
    /// frame.
    pub fn screen_late(&self, i: usize, frames: &[&Frame]) -> u32 {
        let d = self.link_delay() as u32;
        let frame = frames[i].frame;
        match self.screen_opened(frame).and_then(|o| self.screen_ok(o, decode_setup(&self.setup).map_or(0, |d| d.battle_state[0x0D] as usize & 1))) {
            Some(ok) if frame >= ok + d => d,
            _ => 0,
        }
    }

    /// The index of frame `number` in the round's frames.
    fn index(&self, number: u32) -> Option<usize> {
        let first = self.frames.first()?.frame;
        let i = number.checked_sub(first)? as usize;
        match self.frames.get(i) {
            Some(f) if f.frame == number => Some(i),
            _ => self.frames.iter().position(|f| f.frame == number),
        }
    }

    /// The index of the frame the custom screen open on frame `number`
    /// opened on (the first frame of the running battle's custom mode);
    /// none while the fight runs.
    fn screen_opened(&self, number: u32) -> Option<usize> {
        let custom = |f: &Frame| f.state[0] == 4 && f.state[1] == 8;
        let i = self.index(number)?;
        if !custom(&self.frames[i]) {
            return None;
        }
        let mut o = i;
        while o > 0 && custom(&self.frames[o - 1]) {
            o -= 1;
        }
        Some(o)
    }

    /// The frame of side `side`'s OK on the custom screen opened on the
    /// `opened`th of the round's frames, if the side pressed it: found from
    /// the side's custom screen bit as the recording console received it
    /// (BattleState +0x14 + side, bit 2). The screen clears its bit on the
    /// tick after the OK, and the next tick's packet takes it over the
    /// link, `link_delay` frames late: it arrives cleared `2 + link_delay`
    /// frames after the OK.
    fn screen_ok(&self, opened: usize, side: usize) -> Option<u32> {
        let open = |g: &&Frame| g.bs.get(2 * (0x14 + side)..2 * (0x15 + side)).and_then(|h| u8::from_str_radix(h, 16).ok()).is_some_and(|b| b & 4 != 0);
        let screen = self.frames[opened..].iter().take_while(|g| g.state[0] == 4 && g.state[1] == 8);
        let cleared = screen.skip_while(|g| !open(g)).find(|g| !open(g))?;
        cleared.frame.checked_sub(2 + self.link_delay() as u32)
    }

    /// Inputs and events for the `i`th of `frames`.
    pub fn tick_inputs(&self, i: usize, frames: &[&Frame]) -> ([PlayerTick; 2], TickEvents) {
        let f = frames[i];
        let input = std::array::from_fn(|p| PlayerTick { held: self.fed(i, frames, p) });
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
            let navi = compat.navi_key(s.navi);
            if !navi.is_some_and(|k| content.defs.navi_by_key(k).is_some()) {
                out.push(format!("side {side}'s navi {} ({:#04x})", navi.unwrap_or("with no key"), s.navi));
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
                match compat.shot_program(n) {
                    Ok(Some(k)) if content.defs.record(&k).is_none() => out.push(format!("side {side}'s {what} {k}")),
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
        // The chips of both players' auto battle data.
        if let Some(blocks) = &self.setup.auto_battle {
            for (side, l) in blocks.iter().enumerate() {
                let (entries, patterns) = saved_auto_battle_block(&unhex(l)?).map_err(|e| format!("side {side}'s auto battle data: {e}"))?;
                let chips = entries
                    .iter()
                    .filter(|&&e| e & 0x8000 == 0 && e != 0)
                    .copied()
                    .chain(patterns.iter().flat_map(|p| p.chips.iter().copied()).filter(|&c| c != 0 && c != 0xFFFF));
                for id in chips {
                    match compat.chip(id) {
                        Some(key) if content.defs.chip_by_key(&key).is_some() => {}
                        Some(key) => out.push(format!("chip {key} ({id:#05x}, side {side}'s auto battle data)")),
                        None => out.push(format!("chip {id:#05x} (side {side}'s auto battle data: no key in compat)")),
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
        // EXE5's rules/light_dark (content/exe5/rules/light_dark).
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
        // (The background by its name in compat: a liberation's map's number
        // is its area's background, which the pack has under the area's
        // first number.)
        let background = nettai_battle::content::BackgroundId(
            compat
                .background(st[4])
                .and_then(|name| content.assets.handle(nettai_content_api::AssetKind::Background, name))
                .ok_or_else(|| format!("EXE5's pack has no background {:#04x}", st[4]))?,
        );
        let settings = nettai_battle::BattleSettings { stage, background, effects: u32::from_le_bytes([st[8], st[9], st[10], st[11]]) };
        if content.defs.rules().is_none() {
            return Err("the content has no rules (EXE5's)".into());
        }
        let local = bs[0x0D] & 1;
        // A side whose setup carries its console's NaviCust (MegaMan's): the
        // engine compiles it and applies the console's patch cards over the
        // stats EXE5's reset leaves ([`reset`]), as a match is set up.
        // (The recorded stats are what the battle's start made of them: the
        // reload, 0x0813F97C.)
        let compiled = |side: usize| self.setup.navicusts.is_some() && d.navi_stats[side].navi == 0;
        // (Its board's expansions and its programs, the rules'
        // `navicust_expansions` and `navicust_programs`.)
        let navicust_of = |side: usize| -> Result<Option<(u8, Vec<Fact>)>, String> {
            let Some(n) = self.setup.navicusts.as_ref().filter(|_| compiled(side)).map(|n| &n[side]) else { return Ok(None) };
            let (list, flags) = n.decode()?;
            // The board is the recorded ExpMemry's (the one the save's
            // parts were placed on), or the game's largest when its rules
            // have fewer sizes than that.
            let largest = match content.defs.rules().map(|r| r.setup_block()) {
                // (The rules' largest: a new side's, their default.)
                Some(b) => {
                    let schema = content.defs.schema(content.defs.rules().expect("the rules").setup);
                    match schema.index_of("navicust_expansions").map(|i| b.get(schema, i)) {
                        Some(nettai_content_api::FieldValue::OptionalU8(Some(n))) => n,
                        _ => 0,
                    }
                }
                None => 0,
            };
            let programs = navicust(content, compat, &list, |part| flags[(part >> 3) as usize] & (0x80 >> (part & 7)) != 0)?;
            Ok(Some((n.expansions.min(largest), programs)))
        };
        let cards_of = |side: usize| -> Result<Vec<Fact<'static>>, String> {
            let Some(lists) = self.setup.patch_cards.as_ref().filter(|_| compiled(side)) else { return Ok(Vec::new()) };
            let list: Vec<(u8, bool)> = lists[side].iter().map(|&b| (b & 0x7F, b & 0x80 == 0)).collect();
            patch_cards(content, compat, d.versions[side], &list)
        };
        let players = [0u8, 1].map(|side| -> Result<PlayerSetup, String> {
            let folder = battle_folder(content, compat, &unhex(&self.setup.folders[side as usize])?, side == local && bs[0x17] != 0)?;
            // (The console's counter before the round's first tick: one
            // less than on the setup's frame.)
            let frames = (self.setup.frame_counter as u32).wrapping_sub(1) & 0xFFFF;
            // The side's level (the rules' `level`): a team navi's
            // attacks go by it. (Nothing reads MegaMan's side's: an older
            // recording, which has none, replays.)
            let level = match self.setup.navi_levels.map(|l| l[side as usize]) {
                None if d.navi_stats[side as usize].navi == 0 => None,
                None => {
                    return Err(format!(
                        "side {side} operates a team navi (navi {}), but the recording's setup has no navi_levels: \
                         its attacks' damage goes by its side's level; record it again with a chip lab that writes them",
                        d.navi_stats[side as usize].navi
                    ));
                }
                Some(l) if l <= u8::MAX as u32 => Some(l as u8),
                Some(l) => return Err(format!("side {side}'s navi level {l}")),
            };
            let mut player = PlayerSetup {
                folder,
                joypad_phase: self.setup.joypad_phases[side as usize],
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
                rules: None,
            };
            let level = level.map_or(nettai_content_api::Value::Nil, |l| nettai_content_api::Value::Int(l as i64));
            player.set_fact(content, "level", &[nettai_battle::rules::Fact::Value(level)])?;
            // The navi (the engine's navi fact): the recording's.
            let navi = navi_stats(content, compat, &d.navi_stats[side as usize])?.navi;
            player.set_fact(content, "navi", &[Fact::Value(Value::Def(Registry::Navi, navi.0))])?;
            // The NaviCust and the patch cards: a compiled side's, the
            // recording's; without a recorded NaviCust, none (the stats are
            // the battle's start's: nothing is compiled over them).
            match navicust_of(side as usize)? {
                Some((expansions, programs)) => {
                    player.set_fact(content, "navicust_programs", &programs)?;
                    player.set_fact(content, "navicust_expansions", &[Fact::Value(Value::Int(expansions as i64))])?;
                }
                None => {
                    player.set_fact(content, "navicust_expansions", &[Fact::Value(Value::Nil)])?;
                }
            }
            player.set_fact(content, "patch_cards", &cards_of(side as usize)?)?;
            // The auto battle data as the save holds it (the rules'
            // `auto_battle_places` and `auto_battle_records`), which the
            // rules' send shuffles as the console's did; none recorded,
            // none.
            let (places, records) = match &self.setup.auto_battle {
                Some(blocks) => auto_battle_facts(content, compat, &unhex(&blocks[side as usize])?)?,
                None => (Vec::new(), Vec::new()),
            };
            player.set_fact(content, "auto_battle_places", &places)?;
            player.set_fact(content, "auto_battle_records", &records)?;
            Ok(player)
        });
        let [mut p0, mut p1] = players;
        // Each side's light and dark MegaMan: his save's value (NaviStats
        // +0x44), the rules' setup. (Hub Style, +0x4C,
        // is the stats': `navi_stats`.)
        for (p, stats) in [(&mut p0, &d.navi_stats[0]), (&mut p1, &d.navi_stats[1])] {
            if let Ok(p) = p {
                p.set_fact(content, "karma", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(stats.light_dark.0 as i64))])?;
            }
        }
        // Each side's souls: its version's six (Team ProtoMan's 1 to 6,
        // Team Colonel's 7 to 12: 0x08024BF0's flags, by the forms'
        // numbers, records.toml's), those the content has, into the souls
        // part's setup.
        for (side, p) in [&mut p0, &mut p1].into_iter().enumerate() {
            if let Ok(p) = p {
                let version = d.versions[side];
                let has = |f: nettai_content_api::FormHandle| {
                    compat.form_number(&content.defs.form(f).key).is_some_and(|n| version.soul_flag(n).is_some())
                };
                let souls: Vec<nettai_battle::rules::Fact> = (0..content.defs.forms.len() as u16)
                    .map(nettai_content_api::FormHandle)
                    // (A soul: a form with EXE5's `soul`, its rules' extension.)
                    .filter(|&f| {
                        content.defs.extension(nettai_content_api::Registry::Form, &content.defs.form(f).key, "soul").is_some() && has(f)
                    })
                    .map(|f| nettai_battle::rules::Fact::Value(nettai_content_api::Value::Def(nettai_content_api::Registry::Form, f.0)))
                    .collect();
                p.set_fact(content, "souls", &souls)?;
            }
        }
        // What each save brings to its navi's stats (EXE5's rules/save's
        // setup), from the recorded block: the rules write it into a side
        // whose stats they build (a compiled MegaMan, a team navi), which
        // then comes out as recorded.
        for (side, p) in [&mut p0, &mut p1].into_iter().enumerate() {
            if let Ok(p) = p {
                let s = &d.navi_stats[side];
                for (field, value) in [("hp", s.max_base_hp as i64), ("reg_up", s.reg_up as i64)] {
                    p.set_fact(content, field, &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Int(value))])?;
                }
                p.set_fact(content, "sun", &[nettai_battle::rules::Fact::Value(nettai_content_api::Value::Bool(s.raw[0x22] != 0))])?;
            }
        }
        // A team navi's stats are the rules' to build from its level (EXE5's
        // rules/save: its story's HP), from its fresh stats with what the
        // save keeps, which the replay then compares with the recorded block.
        let stats = |side: usize| -> Result<EngineNaviStats, String> {
            let recorded = navi_stats(content, compat, &d.navi_stats[side])?;
            if compiled(side) {
                reset(content, &recorded)
            } else if content.navi(recorded.navi).story.is_some() {
                team_navi_reset(content, &recorded)
            } else {
                Ok(recorded)
            }
        };
        Ok(RoundSetup {
            content: content.hash(),
            settings,
            navi_stats: [stats(0)?, stats(1)?],
            // (The RNG2 the send starts from: the rules' send draws from it
            // and leaves the recorded `rng2`.)
            rng: self.setup.send_rng2.ok_or(
                "an EXE5 recording without send_rng2 and auto_battle (the RNG2 and the saves' auto battle data before \
                 the send): convert it with the verification workspace's tools/exe5/convert_send.py",
            )?,
            local_side: bs[0x0D],
            score: nettai_battle::SetScore { wins: bs[0x18], losses: bs[0x19], round: bs[0x1A], max_combo: bs[0x1B] },
            // (A triple battle's: two rounds after the first.)
            later_stages: vec![nettai_battle::Stage { stage, background }; 2],
            low_hp_music_latched: bs[0x20] | bs[0x21] != 0,
            players: [p0?, p1?],
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

/// A player's auto battle data from the block a recording carries as the
/// save holds it (`saved_auto_battle_block`): the rules' setup's
/// `auto_battle_places` (its places, each `{ chip }`, `{ pattern }` from 1,
/// `{ zero = true }` or `{}` empty) and `auto_battle_records` (the eight
/// records in their places, used or not: the AI's read of a pattern can
/// run on into the ones after it).
fn auto_battle_facts(content: &Content, compat: &Compat, block: &[u8]) -> Result<(Vec<Fact<'static>>, Vec<Fact<'static>>), String> {
    let (entries, patterns) = saved_auto_battle_block(block)?;
    let chip = |id: u16| -> Result<Fact<'static>, String> {
        let h = compat
            .chip(id)
            .and_then(|k| content.defs.chip_by_key(&k))
            .ok_or_else(|| format!("the auto battle data's chip {id:#05x} isn't in the content"))?;
        Ok(Fact::Value(Value::Def(Registry::Chip, h.0)))
    };
    let mut places = Vec::new();
    for e in entries {
        places.push(Fact::Record(match e {
            0 => vec![("zero", Fact::Value(Value::Bool(true)))],
            0xFFFF => Vec::new(),
            e if e & 0x8000 != 0 => vec![("pattern", Fact::Value(Value::Int((e & 0x7FFF) as i64 + 1)))],
            e => vec![("chip", chip(e)?)],
        }));
    }
    let mut records = Vec::new();
    for p in patterns {
        let mut chips = Vec::new();
        for &c in &p.chips {
            chips.push(Fact::Record(match c {
                0 => vec![("zero", Fact::Value(Value::Bool(true)))],
                0xFFFF => Vec::new(),
                c => vec![("chip", chip(c)?)],
            }));
        }
        records.push(Fact::Record(vec![
            ("dx", Fact::Value(Value::Int(p.dx as i64))),
            ("dy", Fact::Value(Value::Int(p.dy as i64))),
            ("chips", Fact::List(chips)),
            ("score", Fact::Value(Value::Int(p.score as i64))),
        ]));
    }
    Ok((places, records))
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
    let navi_key = compat.navi_key(s.navi).ok_or_else(|| format!("navi {:#04x} has no key", s.navi))?;
    let navi = content.defs.navi_by_key(navi_key).ok_or_else(|| format!("the content has no {navi_key}"))?;
    let weapon = |n: u8| -> Result<Option<WeaponHandle>, String> {
        match compat.weapon(n)? {
            None => Ok(None),
            Some(k) => content.defs.weapon_by_key(&k).map(Some).ok_or_else(|| format!("the content has no weapon {k}")),
        }
    };
    // (A shot program's byte 0 is no program.)
    let variant = |n: u8| -> Result<Option<RecordHandle>, String> {
        match compat.shot_program(n)? {
            None => Ok(None),
            Some(k) => content.defs.record(&k).map(Some).ok_or_else(|| format!("the content has no projectile variant {k}")),
        }
    };
    let first_barrier = match compat.barrier(s.first_barrier)? {
        None => None,
        Some(k) => Some(content.defs.record(&k).ok_or_else(|| format!("the content has no barrier {k}"))?),
    };
    let r = &s.raw;
    let base = content.base_form_for(navi);
    let mut stats = EngineNaviStats {
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
        weapons: NaviWeapons {
            buster: weapon(r[0x04])?,
            charge_shot: weapon(r[0x05])?,
            back_special: weapon(r[0x07])?,
            a_charge: weapon(r[0x39])?,
            mode9_a: None,
            buster_shot: variant(r[0x4D])?,
            charge_shot_kind: variant(r[0x4F])?,
            // (EXE5's +0x48, as EXE6's: a team navi's B+Back special's.)
            back_special_damage: u16::from_le_bytes([r[0x48], r[0x49]]),
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
        game: Default::default(),
    };
    // EXE5's own stats (its rules' `stats`), by name: the sun (+0x22), the
    // drops' and encounters' NaviCust effects (+0x26, +0x28), Hub Style
    // (+0x4C) and the soul's extra turns (+0x32, signed).
    for (name, v) in [
        ("sun", nettai_content_api::Value::Bool(r[0x22] != 0)),
        ("chip_drops", nettai_content_api::Value::Int(r[0x26] as i64)),
        ("encounters", nettai_content_api::Value::Int(r[0x28] as i64)),
        ("hub_style", nettai_content_api::Value::Int(s.hub_style as i64)),
        ("soul_turn_bonus", nettai_content_api::Value::Int(r[0x32] as i8 as i64)),
    ] {
        stats.set_game_stat(content, name, v).map_err(|e| format!("EXE5's stat {name}: {e}"))?;
    }
    Ok(stats)
}

/// The stats EXE5's reset leaves of `recorded`, which a NaviCust is compiled
/// over (the reload a battle's start runs, 0x0813F97C: the reset,
/// 0x08133DBC, then the compile, 0x0813FA10, and the cards, 0x08138214). The
/// reset lays the navi's fresh stats (0x080111AA: the engine's
/// `NaviStats::fresh`, what the game's rules and the navi's definition
/// state) and keeps of the save's the mood, the base HP, the soul, the
/// folder and its Regular chips, the Regular memory, the sun (+0x22) and
/// the HP (and +0x21 and the light/dark value, which the engine's stats
/// don't hold). The
/// navi's variant is the recording's: the console sets it as the battle
/// starts, after the reload.
pub fn reset(content: &Content, recorded: &EngineNaviStats) -> Result<EngineNaviStats, String> {
    let fresh = EngineNaviStats::fresh(recorded.navi, content)
        .ok_or_else(|| format!("{} has no fresh stats (its definition's `fresh`)", content.defs.navi(recorded.navi).key))?;
    Ok(EngineNaviStats {
        mood: recorded.mood,
        max_base_hp: recorded.max_base_hp,
        hp: recorded.hp,
        form: recorded.form,
        starting_form: recorded.starting_form,
        folder: recorded.folder,
        folder_reg: recorded.folder_reg,
        reg_up: recorded.reg_up,
        navi_variant: recorded.navi_variant,
        // (The sun: the game's own stat, below.)
        ..fresh
    })
    .map(|mut s| {
        if let Some(nettai_content_api::FieldValue::Bool(sun)) = recorded.game_stat(content, "sun") {
            s.set_game_stat(content, "sun", nettai_content_api::Value::Bool(sun)).expect("EXE5's sun");
        }
        s
    })
}

/// The stats a team navi's are built from (a navi with a `story`: EXE5's
/// rules/save sets its HP by its level), of `recorded`: its fresh stats
/// (`NaviStats::fresh`) with what the save keeps (the folder, its Regular
/// and tag chips; the Regular memory, rules/save's to write) and the
/// recording's own mood and variant (the battle's start's, as [`reset`]
/// keeps them).
pub fn team_navi_reset(content: &Content, recorded: &EngineNaviStats) -> Result<EngineNaviStats, String> {
    let fresh = EngineNaviStats::fresh(recorded.navi, content)
        .ok_or_else(|| format!("{} has no fresh stats (its definition's `fresh`)", content.defs.navi(recorded.navi).key))?;
    Ok(EngineNaviStats {
        mood: recorded.mood,
        form: recorded.form,
        starting_form: recorded.starting_form,
        folder: recorded.folder,
        folder_reg: recorded.folder_reg,
        folder_tags: recorded.folder_tags,
        navi_variant: recorded.navi_variant,
        ..fresh
    })
}

// (A save's NaviCust and patch cards as setup facts are the codec's, which a
// save's import reads too.)
pub use crate::codec::{navicust, patch_cards};

/// Differences between the engine and an EXE5 frame: the state machine and
/// its counters, the simulation RNG, the gauge, the panels (by EXE5's
/// numbers, through compat) and the objects, as EXE6's comparison sees them:
/// each object's pool and kind (EXE5's numbers, through compat's
/// kinds.toml), header flags, state, action (a navi's by EXE5's numbers:
/// `Compat::navi_action`), phase and its init byte, panel, side, HP,
/// position, timer, animation and its collision's status flags.
pub fn compare(b: &Battle, f: &Frame, compat: &Compat) -> Vec<String> {
    compare_with(b, f, f, compat)
}

/// [`compare`] on the `i`th of a round's `frames`, the engine having been fed
/// [`Round::fed`]: while the recording console's custom screen runs late
/// (from its OK until the fight resumes, [`Round::screen_late`]), the
/// banner it shows is the recording's that many frames earlier; nothing
/// else is let go.
pub fn compare_at(b: &Battle, round: &Round, frames: &[&Frame], i: usize, compat: &Compat) -> Vec<String> {
    let f = frames[i];
    let late = round.screen_late(i, frames);
    let banner = if late > 0 { round.frame(f.frame - late).unwrap_or(f) } else { f };
    compare_with(b, f, banner, compat)
}

/// [`compare`], the banner compared with `banner`'s.
fn compare_with(b: &Battle, f: &Frame, banner: &Frame, compat: &Compat) -> Vec<String> {
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
    let task = (banner.hud_tasks >> 15) & 1 != 0;
    check("banner", (b.banner.active as u8).to_string(), (task as u8).to_string());
    if let (Some(id), true) = (b.banner_for(r.local_side), task) {
        let ours = match b.telop_for(r.local_side) {
            Some(_) => Some(0),
            None => b.content.assets.number(nettai_content_api::AssetKind::Banner, id.0).map(|n| n.id as u8),
        };
        let theirs = unhex(&banner.banner).ok().and_then(|x| x.get(1).copied());
        let show = |n: Option<u8>| n.map_or("none".to_string(), |n| format!("{n:#04x}"));
        check("banner number", show(ours), show(theirs));
    }
    // (A panel type is the game's number of it.)
    let panels: Vec<String> = (1..=3)
        .flat_map(|y| (1..=6).map(move |x| (x, y)))
        .map(|(x, y)| {
            let p = b.field.panel(x, y).expect("a field panel");
            format!("[{}, {}]", p.kind.0, p.alliance)
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
    // The traced console's ROM: an object keeping a code address its
    // spawner left (the content's Team ProtoMan US's) has that ROM's
    // (games.toml).
    let rom = f.console.unwrap_or((Version::Protoman, false));
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
            // (A kind moving from its spawner's address in Z, or what wears
            // its Z, its first related object's (its form overlay), is offset
            // as a whole; any other Z that is such an address is mapped.)
            let mover = |r: Option<nettai_battle::object::ObjectRef>| {
                let k = &b.content.defs.kind(b.objects.get(r?).kind).key;
                compat.kinds.get(k).filter(|e| e.z_moves_from_spawner).map(|_| k.clone())
            };
            let wearer = x.related[0].filter(|&r| b.objects.get(r).pos.z == x.pos.z);
            at[2] = match mover(Some(o)).or_else(|| mover(wearer)) {
                Some(k) => at[2].wrapping_add(compat.games.z_offset(rom, &k)),
                None => match compat.games.z(rom, at[2]) {
                    // (A kind falling from such a Z, once it falls.)
                    z if z == at[2] => match entries[i].and_then(|e| e.z_falls_from_spawner) {
                        Some(ticks) => z.wrapping_add(compat.games.fall_offset(rom, key, z, x.timer, ticks)),
                        None => z,
                    },
                    z => z,
                },
            };
            vec![
                ("kind", kind),
                ("flags", format!("{:#04x}", x.flags)),
                ("state", format!("{:#04x}", x.state)),
                ("action", action),
                ("phase", format!("{:#04x}", x.phase)),
                ("phase init", format!("{:#04x}", x.phase_init)),
                ("panel", panel(i, compat.games.panel(rom, key, [x.panel.x, x.panel.y]))),
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

/// Side `side`'s arm chip for the turn (the rules' `arm_chip`, EXE5's souls
/// part's: ColonelSoul's Arm Change, the transform record's +6), by EXE5's
/// number.
fn arm_chip_number(b: &Battle, compat: &Compat, side: u8) -> Option<u16> {
    let (schema, state) = b.rules_state(side)?;
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
    // frame N is the trace at N + 1. Picks the console makes after the
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
        let differences = compare_at(&b, round, &frames, i, compat);
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
