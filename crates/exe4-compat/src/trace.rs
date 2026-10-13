//! The chip lab's EXE4 recordings: JSON lines with EXE6's line kinds (a
//! `setup` line, `exchange` lines, one line per frame) read from EXE4's RAM
//! (the verification workspace's oracle-trace `EXE4` layout:
//! docs/design/exe4-map.md §17), their decoding into the engine's terms
//! ([`codec`](crate::codec)), and their replay: a round's setup, then each
//! battle frame ticked and compared ([`run_round`]), as far as the engine
//! gets.

use crate::codec::{self, ChipHand, NAVI_STATS, NaviStats, Panel};
use crate::{Compat, Version, pool_of_type, pool_slots, type_of_pool};
use nettai_battle::console::ConsoleSetup;
use nettai_battle::content::ChipCode;
use nettai_battle::custom::{BattleFolder, FolderChip, PlayerSetup};
use nettai_battle::rules::Fact;
use nettai_battle::{Battle, Content, NaviStats as EngineNaviStats, PlayerTick, RoundSetup, TickEvents};
use nettai_content_api::{Pool, Registry, Value, WeaponHandle};
use serde::Deserialize;
use std::io::BufRead;
use std::sync::Arc;

/// A battle folder in EXE4's RAM: 30 chips, a halfword each (the chip's id
/// in the low 9 bits, its code above), 0x3C bytes.
pub const FOLDER: usize = 0x3C;
/// BattleState's size as the recordings carry it.
pub const BATTLE_STATE: usize = 0xF0;

/// The static inputs of a round, as an EXE4 recording's setup line has
/// them (exe4-map.md §17).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub frame: u32,
    /// "exe4".
    pub game: String,
    /// BattleState +0x3C: the battle settings record's address (a ROM
    /// address, the console's ROM's).
    pub settings_ptr: u32,
    /// The record (0x10 bytes from it: the 12-byte record and the next
    /// one's first four), hex.
    pub settings: String,
    /// Both sides' NaviStats (0x40 bytes each) as the PET compiled them,
    /// hex.
    pub navi_stats: [String; 2],
    /// The recording console's battle folder (0x3C bytes), hex, and both
    /// consoles' by side.
    pub folder: String,
    pub folders: [String; 2],
    /// BattleState (0xF0 bytes), hex.
    pub battle_state: String,
    pub rng1: u32,
    pub rng2: u32,
    /// Both consoles' joypad repeat beats on the setup's frame.
    pub joypad_phases: [u8; 2],
    /// Both sides' versions ("redsun" or "bluemoon").
    pub game_versions: [String; 2],
    /// The recording console's frame counter (the toolkit's +0x24).
    pub frame_counter: u16,
    /// Both consoles' RNG1 and Regular flags (BattleState +0x17), when the
    /// other console's last capture was on the setup's frame: a recording
    /// of side 1's console has them, side 0's not (its console runs its
    /// frame first).
    #[serde(default)]
    pub rng1s: Option<[u32; 2]>,
    #[serde(default)]
    pub regular_flags: Option<[u8; 2]>,
    /// Both consoles' NaviCusts and patch cards as their saves hold them:
    /// what the PET compiled into the recorded stats.
    pub navicusts: [NaviCustSetup; 2],
    pub patch_cards: [PatchCardsSetup; 2],
    /// Both sides' wins less their losses against the other version's
    /// players (the battle's copies of the link records' +0xB6, 0x0203BE72
    /// and 0x0203BE7A, signed: EXE4's rules/sun_and_moon). A recording from
    /// before the tracer wrote them has none: the saves' (Tango's netplay
    /// saves hold 100, the setup's default).
    #[serde(default)]
    pub rival_wins: Option<[i16; 2]>,
    /// Both sides' regions ("us" or "jp").
    pub game_regions: [String; 2],
    /// The background the battle shows by the loader's first choice
    /// (0x08085430): the game state's +0x0F (where the link pick's
    /// background goes), 0xFF none (then the settings record's +5).
    pub background: u8,
    /// Both players' auto battle data as the link exchanged it (0xE0 bytes
    /// each, side 0's first, hex: 0x020358B0, the blocks the send shuffled
    /// and counted): what the consoles played. The round's setup is
    /// `auto_battle` and `send_rng2`, which the rules' send turns into it.
    #[serde(default)]
    pub ai_lists: Option<[String; 2]>,
    /// Both players' auto battle data as their saves hold it (0xE0 bytes
    /// each, side 0's first, hex: save 0x5064, before the send), the round's
    /// setup's `auto_battle_places`; and the RNG2 the send starts from
    /// (0x0800F208: 84 draws, both consoles alike, then the fight's
    /// `rng2`), the round's battle RNG, from which the rules' send
    /// (content/exe4/rules/auto_battle/block.luau) arrives at the recorded
    /// blocks and `rng2`. A recording from before the tracer wrote them has
    /// neither: no data (Tango's netplay saves hold none), sent from the
    /// RNG2 84 draws before the fight's.
    #[serde(default)]
    pub auto_battle: Option<[String; 2]>,
    #[serde(default)]
    pub send_rng2: Option<u32>,
}

/// The draws of the link's send of the auto battle data (0x0800F208: two a
/// swap, three swaps among the block's first three places and 39 among the
/// other 39), which both consoles make alike from the RNG2 they share before
/// the fight starts.
const SEND_DRAWS: u32 = 84;

/// The RNG2 `draws` draws before `state`: the step `x' = (rotl(x, 1) + 1) ^
/// 0x873CA9E5` undone.
fn rng_back(state: u32, draws: u32) -> u32 {
    (0..draws).fold(state, |x, _| (x ^ 0x873C_A9E5).wrapping_sub(1).rotate_right(1))
}

/// The places of an auto battle data block as a recording carries it (0xE0
/// bytes: its first 42 halfwords).
fn auto_battle_places(block: &[u8]) -> Result<Vec<u16>, String> {
    if block.len() != 0xE0 {
        return Err(format!("an auto battle data block is 0xe0 bytes, not {:#x}", block.len()));
    }
    Ok((0..crate::save::AUTO_BATTLE_PLACES).map(|i| u16::from_le_bytes([block[2 * i], block[2 * i + 1]])).collect())
}

/// A console's NaviCust as its save holds it, in a setup line.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviCustSetup {
    /// The list: 25 parts of 8 bytes (save 0x4564), hex.
    pub parts: String,
    /// The grid (save 0x4540, 0x24 bytes: the 5x5 grid's cells, each the
    /// list's entry from 1, 0 empty), hex.
    pub grid: String,
    /// The color bar (save 0x190, 6 bytes), hex.
    pub color_bar: String,
}

/// A console's patch cards as its save holds them, in a setup line: the six
/// slots' cards on (save 0x464C) and off (0x4653), 0xFF none, hex.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchCardsSetup {
    pub on: String,
    pub off: String,
}

/// A battle object as the recording has it.
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
    /// The record's +0x17, which oracle-trace names as EXE6's: EXE4 keeps
    /// the object's element there (EXE6's +0x0E: exe4-map.md §3.1).
    pub flip: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub pos: [i32; 3],
    pub timer: u16,
    pub anim: u8,
    /// The collision record's hit flags (+0x54): what hit it this tick.
    pub status: u32,
    /// The collision record's status word (+0x64, the engine's `f1`).
    pub f1: u32,
    /// The sprite block's palette (+4), its palette's other half (+5) and
    /// its palette pointer (+0x34): what 0x0800295C draws the object in.
    pub sprite: [u32; 3],
}

/// One battle frame. (EXE4's BattleState counts no frames or ticks: the
/// recordings have none.)
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub frame: u32,
    pub state: [u8; 4],
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
    /// Both sides' battle NaviStats this frame (hex).
    pub navi_stats: [String; 2],
    pub input: [[u16; 3]; 2],
    pub objects: Vec<Object>,
    pub panels: Vec<[u8; 2]>,
    pub chip_blocks: [String; 2],
}

/// An exchange: both sides' NaviStats as they changed (EXE4 exchanges no
/// transform records).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exchange {
    pub frame: u32,
    pub navi_stats: [String; 2],
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
    Ok(codec::navi_stats(&b))
}

/// A round's setup, decoded.
#[derive(Clone, Debug)]
pub struct DecodedSetup {
    pub navi_stats: [NaviStats; 2],
    pub versions: [Version; 2],
    /// Both sides' regions: true for a Japanese console.
    pub japanese: [bool; 2],
    /// The settings record (its 12 bytes and the next one's first four).
    pub settings: [u8; 0x10],
    pub battle_state: Vec<u8>,
    /// Both folders, by side: each entry its chip's id and code (none: an
    /// entry with no chip).
    pub folders: [[Option<(u16, u8)>; 30]; 2],
}

impl DecodedSetup {
    /// The recording console's side (BattleState +0x0D).
    pub fn local(&self) -> usize {
        self.battle_state[0x0D] as usize & 1
    }

    /// The recording console's ROM: its side's version and region, whose
    /// addresses its settings record holds.
    pub fn traced_rom(&self) -> (Version, bool) {
        let local = self.local();
        (self.versions[local], self.japanese[local])
    }

    /// The settings record's layout (+1) and actor list (+8).
    pub fn stage_record(&self) -> (u8, u32) {
        let s = &self.settings;
        (s[1], u32::from_le_bytes([s[8], s[9], s[10], s[11]]))
    }
}

/// A folder's 30 entries (an entry of the game's empty 0xFFFF, or of chip 0,
/// none).
fn folder_entries(b: &[u8]) -> Result<[Option<(u16, u8)>; 30], String> {
    if b.len() != FOLDER {
        return Err(format!("a battle folder of {:#x} bytes, not {FOLDER:#x}", b.len()));
    }
    Ok(std::array::from_fn(|i| {
        let v = u16::from_le_bytes([b[2 * i], b[2 * i + 1]]);
        (v != 0xFFFF && v & 0x1FF != 0).then_some((v & 0x1FF, (v >> 9) as u8))
    }))
}

pub fn decode_setup(s: &Setup) -> Result<DecodedSetup, String> {
    if s.game != crate::ROOT {
        return Err(format!("an {} recording", s.game));
    }
    let version = |v: &str| Version::named(v).ok_or_else(|| format!("version {v:?}"));
    let region = |r: &str| match r {
        "us" => Ok(false),
        "jp" => Ok(true),
        r => Err(format!("region {r:?}")),
    };
    let battle_state = unhex(&s.battle_state)?;
    if battle_state.len() != BATTLE_STATE {
        return Err(format!("a BattleState of {:#x} bytes", battle_state.len()));
    }
    folder_entries(&unhex(&s.folder)?)?;
    let folders = [folder_entries(&unhex(&s.folders[0])?)?, folder_entries(&unhex(&s.folders[1])?)?];
    for c in &s.navicusts {
        let (parts, grid, bar) = (unhex(&c.parts)?, unhex(&c.grid)?, unhex(&c.color_bar)?);
        if parts.len() != crate::save::NAVICUST_PARTS * 8 || grid.len() != 0x24 || bar.len() != 6 {
            return Err(format!("a NaviCust of {:#x}, {:#x} and {:#x} bytes", parts.len(), grid.len(), bar.len()));
        }
    }
    for c in &s.patch_cards {
        // (The recorder takes the PET's six slots; the reload reads a seventh, which no console fills.)
        if unhex(&c.on)?.len() != RECORDED_PATCH_CARD_SLOTS || unhex(&c.off)?.len() != RECORDED_PATCH_CARD_SLOTS {
            return Err("patch card slots that aren't six".into());
        }
    }
    Ok(DecodedSetup {
        navi_stats: [navi_stats_hex(&s.navi_stats[0])?, navi_stats_hex(&s.navi_stats[1])?],
        versions: [version(&s.game_versions[0])?, version(&s.game_versions[1])?],
        japanese: [region(&s.game_regions[0])?, region(&s.game_regions[1])?],
        settings: unhex(&s.settings)?.try_into().map_err(|_| "a settings record that isn't 0x10 bytes".to_string())?,
        battle_state,
        folders,
    })
}

/// An object, in the engine's terms: its pool, and EXE4's kind number in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedObject {
    pub pool: Pool,
    pub kind: u8,
    /// The object's element (its +0x17).
    pub element: u8,
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
    let panels = f.panels.iter().map(|&[t, a]| codec::panel(t, a)).collect::<Result<Vec<_>, _>>().map_err(|e| format!("frame {}: {e}", f.frame))?;
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
            return Err(format!("frame {}: more {pool:?} objects than EXE4's {} slots", f.frame, pool_slots(pool)));
        }
        objects.push(DecodedObject { pool, kind: o.index, element: o.flip, object: o.clone() });
    }
    let hand = |s: &str| codec::chip_hand(compat, &unhex(s)?);
    Ok(DecodedFrame { frame: f.frame, panels, objects, hands: [hand(&f.chip_blocks[0])?, hand(&f.chip_blocks[1])?] })
}

/// An exchange, decoded: both sides' stats as they changed.
pub fn decode_exchange(e: &Exchange) -> Result<[NaviStats; 2], String> {
    Ok([navi_stats_hex(&e.navi_stats[0])?, navi_stats_hex(&e.navi_stats[1])?])
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
            Line::Setup(s) => {
                rounds.push(Round { setup: *s, exchanges: std::mem::take(&mut pending), frames: Vec::new() })
            }
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

impl Round {
    /// The frames the engine simulates: the running and end states, from
    /// the setup's frame up to the next round's init (EXE4's battle state
    /// machine has EXE6's top states: 0 init, 4 running, 8 end).
    pub fn battle_frames(&self) -> impl Iterator<Item = &Frame> {
        self.frames.iter().filter(|f| f.frame >= self.setup.frame).take_while(|f| f.state[0] == 4 || f.state[0] == 8)
    }

    /// The link's delay: the chip lab's emulated cable's, which delivered
    /// each console's packet this many ticks after it went out.
    pub fn link_delay(&self) -> u8 {
        LINK_DELAY
    }

    fn frame(&self, number: u32) -> Option<&Frame> {
        let first = self.frames.first()?.frame;
        let f = self.frames.get(number.checked_sub(first)? as usize)?;
        (f.frame == number).then_some(f).or_else(|| self.frames.iter().find(|f| f.frame == number))
    }

    /// A player's buttons on a frame as the original's fight saw them:
    /// what the link delivered, which the recording has (pressed
    /// `link_delay` frames earlier).
    pub fn joypad(&self, frame: u32, side: usize) -> u16 {
        self.frame(frame).map_or(0, |f| f.input[side][0] & 0x3FF)
    }

    /// The buttons the engine is fed for side `side` on the `i`th of
    /// `frames`, so that it plays as the original did with no link of its
    /// own (as exe5-compat's): while the fight runs, what the original's
    /// fight saw, the frame a custom screen opens on among them (its tick
    /// is the fight's still, which takes the L or R that opens it,
    /// 0x08007A2E); from the frame after to the side's OK, what the
    /// original's screen saw (the buttons `link_delay` frames on); for
    /// `link_delay` frames from the OK, the buttons of the frame before
    /// it; then again what the fight sees.
    pub fn fed(&self, i: usize, frames: &[&Frame], side: usize) -> u16 {
        let d = self.link_delay() as u32;
        let frame = frames[i].frame;
        let Some(opened) = self.screen_opened(frame) else { return self.joypad(frame, side) };
        // (A round's first screen opens with no fight before it.)
        let from_fight = opened > 0 && self.frames[opened - 1].state[0] == 4 && self.frames[opened - 1].state[1] == 12;
        if from_fight && self.frames[opened].frame == frame {
            return self.joypad(frame, side);
        }
        match self.screen_ok(opened, side) {
            Some(ok) if frame >= ok + d => self.joypad(frame, side),
            Some(ok) if frame >= ok => self.joypad((ok + d).saturating_sub(1), side),
            _ => self.joypad(frame + d, side),
        }
    }

    /// How many frames late the engine's custom screen of the recording
    /// console's side is on the `i`th of `frames`: `link_delay` from its OK
    /// until the fight resumes ([`Round::fed`]), else none.
    pub fn screen_late(&self, i: usize, frames: &[&Frame]) -> u32 {
        let d = self.link_delay() as u32;
        let frame = frames[i].frame;
        match self.local_ok(frame) {
            Some(ok) if frame >= ok + d => d,
            _ => 0,
        }
    }

    /// The recording's frame the engine's custom screen of the recording
    /// console's side stands at on the `i`th of `frames`, for what that
    /// screen's OK changes on its console alone (the Regular chip still to
    /// come): the frame's own; for `link_delay` frames from the side's OK,
    /// the frame before the OK (the engine's screen takes its OK that much
    /// later, [`Round::fed`]); then the frame `link_delay` earlier, until
    /// the fight resumes ([`Round::screen_late`]).
    pub fn local_screen_frame<'a>(&'a self, i: usize, frames: &[&'a Frame]) -> &'a Frame {
        let d = self.link_delay() as u32;
        let f = frames[i];
        match self.local_ok(f.frame) {
            Some(ok) if f.frame >= ok + d => self.frame(f.frame - d).unwrap_or(f),
            Some(ok) if f.frame >= ok => ok.checked_sub(1).and_then(|before| self.frame(before)).unwrap_or(f),
            _ => f,
        }
    }

    /// The recording console's side (its setup's BattleState +0x0D).
    fn local_side(&self) -> usize {
        decode_setup(&self.setup).map_or(0, |d| d.local())
    }

    /// The recording console's side's OK on the custom screen open on frame
    /// `frame`, if the side has pressed it by the screen's end (none while
    /// the fight runs): the frame the original's screen took it, which the
    /// replay feeds the engine's `link_delay` frames later ([`Round::fed`]).
    /// For the frame comparison's known shift: from it until the engine's
    /// screen takes it, that screen holds the OK back; then it runs
    /// `link_delay` frames behind the original's until the fight resumes
    /// ([`Round::screen_late`]).
    pub fn local_ok(&self, frame: u32) -> Option<u32> {
        self.screen_opened(frame).and_then(|o| self.screen_ok(o, self.local_side()))
    }

    /// The recording console's side's first OK of the round ([`Round::local_ok`]
    /// of its first screen it pressed OK on): from `link_delay` frames
    /// after it on, what counts from that side's send (the HUD's full
    /// gauge's stripes, which "waiting" sets) runs that many frames behind
    /// the original's, to the round's end.
    pub fn first_local_ok(&self) -> Option<u32> {
        let custom = |f: &Frame| f.state[0] == 4 && f.state[1] == 8;
        let mut i = 0;
        while i < self.frames.len() {
            if custom(&self.frames[i]) {
                if let Some(ok) = self.screen_ok(i, self.local_side()) {
                    return Some(ok);
                }
                while i < self.frames.len() && custom(&self.frames[i]) {
                    i += 1;
                }
                continue;
            }
            i += 1;
        }
        None
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
    /// the side's choosing bit as the recording console received it
    /// (BattleState +0x14 + side, bit 0: set as the selection starts,
    /// 0x08020348, and cleared by OK on the tick it takes the key,
    /// 0x08020652), which goes out with the next tick's packet and arrives
    /// `SCREEN_BIT_CLEARED + link_delay` frames after the OK. (Bit 2, the
    /// screen's open bit, clears as its result is sent, 0x0801E986: eleven
    /// ticks after the OK, or after a Program Advance's animation.)
    fn screen_ok(&self, opened: usize, side: usize) -> Option<u32> {
        let choosing = |g: &&Frame| g.bs.get(2 * (0x14 + side)..2 * (0x15 + side)).and_then(|h| u8::from_str_radix(h, 16).ok()).is_some_and(|b| b & 1 != 0);
        let screen = self.frames[opened..].iter().take_while(|g| g.state[0] == 4 && g.state[1] == 8);
        let cleared = screen.skip_while(|g| !choosing(g)).find(|g| !choosing(g))?;
        cleared.frame.checked_sub(SCREEN_BIT_CLEARED + self.link_delay() as u32)
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
    /// hands (ids, by first appearance).
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

    /// What the round needs that `content` doesn't define, or that EXE4's
    /// recorded stats hold that the port can't say yet: its chips (`chip
    /// cannon (0x001)`), the navis, forms and weapons the stats name, the
    /// stage, EXE4's pack.
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
            if let Err(e) = navi_stats(content, compat, s) {
                out.push(format!("side {side}'s stats: {e}"));
            }
        }
        let (layout, actor_list) = d.stage_record();
        let (version, japanese) = d.traced_rom();
        match compat.stage(layout, actor_list, version, japanese) {
            Some(k) if content.defs.stage_by_key(&k).is_some() => {}
            Some(k) => out.push(format!("the stage {k} (layout {layout:#04x}, actors {actor_list:#010x})")),
            None => out.push(format!("the stage (layout {layout:#04x}, actors {actor_list:#010x}: no EXE4 netbattle stage)")),
        }
        if content.assets.pack(crate::ROOT).is_none() {
            out.push("EXE4's pack".into());
        }
        out.dedup();
        Ok(out)
    }

    /// The engine's starting point for this round on `content`: what the
    /// round needs is defined ([`Round::needs`]), then EXE4's records go
    /// into the engine's: the settings record's stage and background, both
    /// NaviStats ([`navi_stats`]), the folders, the RNGs, the set's score,
    /// both players on EXE4's rules.
    pub fn round_setup(&self, content: &Content, compat: &Compat) -> Result<RoundSetup, String> {
        let needs = self.needs(content, compat)?;
        if !needs.is_empty() {
            return Err(format!("content lacks {}", needs.join(", ")));
        }
        let d = decode_setup(&self.setup)?;
        let bs = &d.battle_state;
        let st = &d.settings;
        let (layout, actor_list) = d.stage_record();
        let (version, japanese) = d.traced_rom();
        let stage = compat.stage(layout, actor_list, version, japanese).and_then(|k| content.defs.stage_by_key(&k)).expect("needs saw the stage");
        let pack = content.assets.pack(crate::ROOT).expect("needs saw the pack");
        // (The background loader, 0x08085430: the game state's +0x0F, else,
        // where it is 0xFF, the settings record's +5: every netbattle
        // record's is 3.)
        let number = match self.setup.background {
            0xFF => st[5],
            n => n,
        };
        let background = nettai_battle::content::BackgroundId(
            content
                .assets
                .number_handle(nettai_content_api::AssetKind::Background, pack, number as u16)
                .ok_or_else(|| format!("EXE4's pack has no background {number:#04x}"))?,
        );
        // (EXE4's records have no effects word: the stage's own, with what a
        // link battle's match adds, as a match sets a round up; a single
        // battle is no set.)
        let effects = content.stage(stage).effects | LINK_BATTLE_EFFECTS & !if triple_battle(bs[0x0F]) { 0 } else { SET };
        let settings = nettai_battle::BattleSettings { stage, background, effects };
        if content.defs.rules().is_none() {
            return Err("the content has no rules (EXE4's)".into());
        }
        let local = d.local();
        // The rules compile each side's NaviCust and patch cards (rules/navicust, rules/patch_cards) over the stats
        // the reload starts from; a card whose effects wait (exe4-map.md §18) is refused.
        let mut cards: [Vec<Fact>; 2] = [Vec::new(), Vec::new()];
        for (side, c) in self.setup.patch_cards.iter().enumerate() {
            let slots: Vec<Option<u8>> = unhex(&c.on)?.into_iter().map(|n| (n != 0xFF).then_some(n)).collect();
            let (facts, waiting) = crate::setup::patch_cards(content, compat, &slots)?;
            if let Some(n) = waiting.first() {
                return Err(format!("side {side}'s patch card {n}: not ported yet (docs/design/exe4-map.md §18)"));
            }
            cards[side] = facts;
        }
        let side_stats = |side: usize| reset(content, compat, &d.navi_stats[side]);
        let players = [0usize, 1].map(|side| -> Result<PlayerSetup, String> {
            let regular = match self.setup.regular_flags {
                Some(r) => r[side] != 0,
                None => side == local && bs[0x17] != 0,
            };
            let folder = battle_folder(content, compat, &d.folders[side], regular)?;
            // (The console's counter before the round's first tick: one less
            // than on the setup's frame.)
            let frames = (self.setup.frame_counter as u32).wrapping_sub(1) & 0xFFFF;
            let mut player = PlayerSetup {
                folder,
                joypad_phase: self.setup.joypad_phases[side],
                console: ConsoleSetup {
                    rng: if side == local { self.setup.rng1 } else { self.setup.rng1s.map_or(0, |r| r[side]) },
                    tag_pair: None,
                    frames,
                },
                rules: None,
            };
            let stats = side_stats(side)?;
            player.set_fact(content, "navi", &[Fact::Value(Value::Def(Registry::Navi, stats.navi.0))])?;
            // What the save brings to the stats (EXE4's rules/save): the base
            // HP, which the rules write into the HP.
            player.set_fact(content, "hp", &[Fact::Value(Value::Int(d.navi_stats[side].max_base_hp as i64))])?;
            // And fighting in the sun (+0x29), which the reset keeps.
            player.set_fact(content, "sun", &[Fact::Value(Value::Bool(d.navi_stats[side].sun))])?;
            // MegaMan's light/dark value (EXE4's rules/light_dark: the
            // starting mood).
            player.set_fact(content, "karma", &[Fact::Value(Value::Int(d.navi_stats[side].light_dark as i64))])?;
            // Its wins against the other version's players (rules/sun_and_moon:
            // RedSun's and BlueMoon's damage), where the recording has them.
            if let Some(wins) = self.setup.rival_wins {
                player.set_fact(content, "rival_wins", &[Fact::Value(Value::Int(wins[side] as i64))])?;
            }
            // His save's NaviCust, which the rules compile (rules/navicust).
            let list = unhex(&self.setup.navicusts[side].parts)?;
            if list.len() != 8 * crate::save::NAVICUST_PARTS {
                return Err(format!("side {side}'s NaviCust list has {} bytes", list.len()));
            }
            let programs = crate::setup::navicust(content, compat, &crate::save::parts(&list))?;
            player.set_fact(content, "navicust_programs", &programs)?;
            // And its patch cards (rules/patch_cards).
            player.set_fact(content, "patch_cards", &cards[side])?;
            // Double Soul and its souls (rules/souls): the recordings' saves
            // (Tango's) have it (event flag 0x14) and every soul of their
            // version (`Version::soul_flag`: Red Sun's 1 to 6, Blue Moon's 7
            // to 12, by the forms' numbers, records.toml's), those the
            // content has.
            let version = d.versions[side];
            let souls: Vec<Fact> = (1..=12u8)
                .filter(|&n| version.soul_flag(n).is_some())
                .filter_map(|n| compat.form(n).and_then(|key| content.defs.form_by_key(key)))
                .map(|f| Fact::Value(Value::Def(Registry::Form, f.0)))
                .collect();
            player.set_fact(content, "souls", &souls)?;
            player.set_fact(content, "double_soul", &[Fact::Value(Value::Bool(true))])?;
            // Its auto battle data as the save holds it (rules/auto_battle),
            // which the rules' send shuffles as the console's did; none
            // recorded, none.
            if let Some(blocks) = &self.setup.auto_battle {
                let places = auto_battle_places(&unhex(&blocks[side])?).map_err(|e| format!("side {side}'s auto battle data: {e}"))?;
                let (facts, unstated) = crate::setup::auto_battle(&places, |n| compat.chip(n).and_then(|k| content.defs.chip_by_key(&k)));
                if let Some(e) = unstated.first() {
                    return Err(format!("side {side}: {e}"));
                }
                player.set_fact(content, "auto_battle_places", &facts)?;
            }
            Ok(player)
        });
        let [p0, p1] = players;
        Ok(RoundSetup {
            content: content.hash(),
            settings,
            navi_stats: [side_stats(0)?, side_stats(1)?],
            // (The RNG2 the send starts from: the rules' send draws from it
            // and leaves the recorded `rng2`.)
            rng: self.setup.send_rng2.unwrap_or_else(|| rng_back(self.setup.rng2, SEND_DRAWS)),
            local_side: bs[0x0D],
            score: nettai_battle::SetScore { wins: bs[0x18], losses: bs[0x19], round: bs[0x1A], max_combo: bs[0x1B] },
            // (A triple battle's: two rounds after the first.)
            later_stages: vec![nettai_battle::Stage { stage, background }; 2],
            // (EXE4 has no low-HP music: nothing reads the latch.)
            low_hp_music_latched: false,
            players: [p0?, p1?],
        })
    }

    /// The round's battle at its start on `content`.
    pub fn start(&self, content: Arc<Content>, compat: &Compat) -> Result<Battle, String> {
        let setup = self.round_setup(&content, compat)?;
        Ok(Battle::new(setup, content))
    }
}

/// The banner's bit in EXE4's HUD task mask (the HUD block's +0x48: set
/// while a banner shows, as the recordings have it from a round's start
/// banner to a KO's).
const BANNER_TASK: u32 = 0x20;
/// The recording console's own telop's bit there (on the banner block,
/// 0x08016454), and the second block's (0x080164B4: the other player's
/// telop, or the name of the chip that player just used).
const TELOP_TASK: u32 = 0x100;
const SECOND_BLOCK_TASK: u32 = 0x8000;

/// The chip lab's emulated cable's delay, in ticks.
const LINK_DELAY: u8 = 4;
/// How many frames after a side's OK its choosing bit arrives cleared, past
/// the link's delay: the next tick's packet takes it (custom/cannon: side
/// 0's OK on 276, its bit received cleared on 281).
const SCREEN_BIT_CLEARED: u32 = 1;
/// The effects a link battle's match type adds to its stage's (as
/// nettai-match's `MATCH_EFFECTS`: a triple battle's).
const LINK_BATTLE_EFFECTS: u32 = 0x600;
/// The set's bit of them, which a single battle hasn't.
const SET: u32 = nettai_battle::setup::effects::SET;

/// Whether the battle state's battle type (+0x0F) is a set's: a triple
/// battle's 0x48, or 0x49 or 0x4A (the intro, 0x08007464, counts the round
/// on in them; a single battle's 0x47 takes the settings record's).
fn triple_battle(battle_type: u8) -> bool {
    matches!(battle_type, 0x48..=0x4A)
}

/// A battle folder by EXE4's chips' handles.
fn battle_folder(content: &Content, compat: &Compat, entries: &[Option<(u16, u8)>; 30], regular_pending: bool) -> Result<BattleFolder, String> {
    let mut chips = [None; nettai_battle::custom::folder::FOLDER_SIZE];
    for (c, e) in chips.iter_mut().zip(entries) {
        let Some((id, code)) = *e else { continue };
        let key = compat.chip(id).ok_or_else(|| format!("chip {id:#05x} has no key"))?;
        let h = content.defs.chip_by_key(&key).ok_or_else(|| format!("the content has no {key}"))?;
        *c = Some(FolderChip::new(h, ChipCode(code)));
    }
    Ok(BattleFolder { chips, regular_pending })
}

/// A side's stats in the engine's terms: the navi's fresh stats (its
/// definition's and EXE4's rules'), with what EXE4's block says over them
/// where the engine has a field for it (docs/design/exe4-map.md §3.3): the
/// mood, the buster's levels and blank count, the weapons by compat
/// (records.toml), the bugs' drains, the custom level and chip limits, the
/// move lag's column (the engine's navi variant), the soul (the form), the
/// aura, the HP, the supports, and the rules' stats (the weapon level, the
/// move bug, the Full Synchro at the start, MegaMan's color, All Guard).
pub fn navi_stats(content: &Content, compat: &Compat, s: &NaviStats) -> Result<EngineNaviStats, String> {
    let navi_key = compat.navi_key(s.navi).ok_or_else(|| format!("navi {:#04x} has no key", s.navi))?;
    let navi = content.defs.navi_by_key(navi_key).ok_or_else(|| format!("the content has no {navi_key}"))?;
    let mut stats = EngineNaviStats::fresh(navi, content).ok_or_else(|| format!("{navi_key} has no fresh stats"))?;
    let weapon = |n: u8| -> Result<WeaponHandle, String> {
        let k = compat.weapon(n)?;
        content.defs.weapon_by_key(&k).ok_or_else(|| format!("the content has no weapon {k}"))
    };
    let form = compat.form(s.soul).ok_or_else(|| format!("soul {}", s.soul))?;
    let form = content.defs.form_by_key(form).ok_or_else(|| format!("the content has no form {form}"))?;
    let first_barrier = match compat.barrier(s.aura)? {
        None => None,
        Some(k) => Some(content.defs.record(&k).ok_or_else(|| format!("the content has no aura {k}"))?),
    };
    // The supports (+0x18): 0xFF the support bug, else a bit each (Rush 1,
    // Beat 2, Tango 4), as EXE6's +0x0D.
    stats.support = (s.supports != 0xFF).then(|| nettai_battle::setup::Supports {
        rush: s.supports & 1 != 0,
        beat: s.supports & 2 != 0,
        tango: s.supports & 4 != 0,
    });
    stats.mood = s.mood;
    stats.super_armor = s.super_armor;
    stats.float_shoes = s.float_shoes;
    stats.air_shoes = s.air_shoes;
    stats.undershirt = s.undershirt;
    stats.set_game_stat(content, "weapon_level", Value::Int(s.weapon_level as i64))?;
    stats.set_game_stat(content, "move_bug", Value::Int(s.move_bug as i64))?;
    stats.set_game_stat(content, "full_synchro_start", Value::Bool(s.full_synchro))?;
    stats.set_game_stat(content, "color", Value::Int(s.color as i64))?;
    stats.set_game_stat(content, "all_guard", Value::Bool(s.all_guard))?;
    stats.set_game_stat(content, "sun", Value::Bool(s.sun))?;
    stats.set_game_stat(content, "encounter_bug", Value::Bool(s.encounter_bug))?;
    stats.set_game_stat(content, "result_bug", Value::Bool(s.result_bug))?;
    stats.attack = s.attack;
    stats.rapid = s.rapid;
    stats.charge = s.charge;
    stats.custom_level = s.custom_level;
    stats.mega_level = s.mega_level;
    stats.giga_level = s.giga_level;
    stats.navi_variant = s.move_lag_column;
    stats.first_barrier = first_barrier;
    stats.form = form;
    stats.starting_form = form;
    stats.max_base_hp = s.max_base_hp;
    stats.hp = s.hp;
    stats.max_hp = s.max_hp;
    stats.bugs.buster_blanks = s.buster_blanks;
    // (The panel trail: every step, its kind 0xFF none, the rules' `effects.panel_trail`.)
    stats.bugs.panel_trail_kind = s.panel_trail.unwrap_or(0xFF);
    stats.bugs.hp_drain = s.hp_drain;
    stats.bugs.custom_drain = s.custom_drain;
    stats.weapons.buster = Some(weapon(s.buster_weapon)?);
    stats.weapons.charge_shot = Some(weapon(s.charged_weapon)?);
    stats.weapons.back_special = s.back_special.map(weapon).transpose()?;
    Ok(stats)
}

/// What the NaviCust and the patch cards compile into a side's stats (rules/navicust), as EXE4's block's bytes say
/// them: the abilities (+0x01 to +0x04), the buster's levels and blanks (+0x05 to +0x08), the weapon level and the
/// move bug (+0x0B, +0x0D), the drains (+0x0E, +0x0F), the custom level and chip limits (+0x12 to +0x14), the supports
/// (+0x18), the panel trail (+0x1B), the Full Synchro at the start (+0x1F), the soul (+0x24: the form's number, a soul
/// patch card's from the start), MegaMan's color (+0x27), All Guard (+0x28), the max HP (+0x32).
pub fn compiled(b: &Battle, s: &EngineNaviStats) -> String {
    let game = |name: &str| match s.game_stat(&b.content, name) {
        Some(nettai_content_api::FieldValue::U8(n)) => n,
        Some(nettai_content_api::FieldValue::Bool(on)) => on as u8,
        _ => 0,
    };
    let supports = match s.support {
        None => 0xFF,
        Some(n) => (n.rush as u8) | (n.beat as u8) << 1 | (n.tango as u8) << 2,
    };
    // (A form compat doesn't number reads 0xFF, which no block holds.)
    let soul = Compat::exe4().form_number(nettai_content_api::keys::local(&b.content.defs.form(s.form).key)).unwrap_or(0xFF);
    compiled_bytes(
        [
            s.super_armor as u8,
            s.float_shoes as u8,
            s.air_shoes as u8,
            s.undershirt as u8,
            s.attack,
            s.rapid,
            s.charge,
            s.bugs.buster_blanks,
            game("weapon_level"),
            game("move_bug"),
            s.bugs.hp_drain,
            s.bugs.custom_drain,
            s.custom_level,
            s.mega_level,
            s.giga_level,
            supports,
            s.bugs.panel_trail_kind,
            game("full_synchro_start"),
            soul,
            game("color"),
            game("all_guard"),
        ],
        s.max_hp,
    )
}

/// [`compiled`] of a recorded block.
pub fn compiled_of(s: &NaviStats) -> String {
    let b = &s.raw;
    compiled_bytes(
        [
            b[0x01], b[0x02], b[0x03], b[0x04], b[0x05], b[0x06], b[0x07], b[0x08], b[0x0B], b[0x0D], b[0x0E], b[0x0F], b[0x12], b[0x13], b[0x14], b[0x18], b[0x1B],
            b[0x1F], b[0x24], b[0x27], b[0x28],
        ],
        s.max_hp,
    )
}

fn compiled_bytes(v: [u8; 21], max_hp: u16) -> String {
    const NAMES: [&str; 21] = [
        "super armor",
        "float shoes",
        "air shoes",
        "undershirt",
        "attack",
        "rapid",
        "charge",
        "blanks",
        "weapon level",
        "move bug",
        "hp drain",
        "custom drain",
        "custom",
        "mega",
        "giga",
        "supports",
        "panel trail",
        "full synchro",
        "soul",
        "color",
        "all guard",
    ];
    let mut out: Vec<String> = NAMES.iter().zip(v).map(|(n, v)| format!("{n} {v:#04x}")).collect();
    out.push(format!("max hp {max_hp}"));
    out.join(", ")
}

/// A side's stats as EXE4's reload starts from them (0x08036CC0: the navi's
/// fresh stats, keeping the mood, the sun and the light/dark value), with the save's
/// HP and what the battle's start writes after the PET (the move lag's
/// column, +0x25): what the rules compile a NaviCust over
/// (rules/navicust), so that the round's stats are the compile's.
pub fn reset(content: &Content, compat: &Compat, s: &NaviStats) -> Result<EngineNaviStats, String> {
    let navi_key = compat.navi_key(s.navi).ok_or_else(|| format!("navi {:#04x} has no key", s.navi))?;
    let navi = content.defs.navi_by_key(navi_key).ok_or_else(|| format!("the content has no {navi_key}"))?;
    let mut stats = EngineNaviStats::fresh(navi, content).ok_or_else(|| format!("{navi_key} has no fresh stats"))?;
    stats.mood = s.mood;
    stats.set_game_stat(content, "sun", Value::Bool(s.sun))?;
    stats.navi_variant = s.move_lag_column;
    stats.max_base_hp = s.max_base_hp;
    stats.hp = s.hp;
    stats.max_hp = s.max_base_hp;
    Ok(stats)
}

/// The patch card slots a recording's setup has (the PET's six: 0x464C to
/// 0x4651).
const RECORDED_PATCH_CARD_SLOTS: usize = 6;

/// Differences between the engine and an EXE4 frame: the state machine, the
/// simulation RNG, the pause, the gauge, the banner, the panels (by EXE4's
/// numbers, the rules' `panels.numbers`) and the objects: each object's
/// pool and kind (EXE4's numbers, through compat's kinds.toml), header
/// flags, state, action (a navi's by EXE4's numbers: `Compat::navi_action`),
/// phase and its init byte, panel, side, element (the recording's +0x17),
/// HP, position, timer, animation, its collision's hit flags: what hit it
/// this tick (the collision record's +0x54, EXE5's +0x68, the engine's
/// `acc.hit_flags`), its status word (+0x64, the engine's `f1`) and its
/// sprite's palette (the sprite block's +4); and both sides' stats as the
/// NaviCust and the patch cards compile them.
pub fn compare(b: &Battle, f: &Frame, compat: &Compat) -> Vec<String> {
    compare_with(b, f, f, f, compat)
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
    compare_with(b, f, banner, round.local_screen_frame(i, frames), compat)
}

/// Side `side`'s auto battle data as the rules' send left it (their state's
/// `auto_battle_places`), in the block's terms: each place's halfword (a chip
/// by its number, a pattern record by its index with bit 15, 0 for `zero`),
/// in order. None: the rules keep no such list.
fn sent_places(b: &Battle, compat: &Compat, side: u8) -> Option<Vec<u16>> {
    use nettai_content_api::FieldValue;
    let (schema, state) = b.rules_state(side)?;
    let list = schema.place(schema.index_of("auto_battle_places")?);
    let n = state.len_at(list)?;
    (0..n)
        .map(|k| {
            let p = list.elem(k)?;
            let chip = match state.get_at(p.field("chip")?) {
                FieldValue::Ref(Some((Registry::Chip, h))) => {
                    let key = nettai_match::ids::local(&b.content.defs.chip(nettai_content_api::ChipHandle(h)).key).to_string();
                    Some(compat.chip_entry(&key).map(|e| e.id).unwrap_or(0xFFFE))
                }
                _ => None,
            };
            let pattern = match state.get_at(p.field("pattern")?) {
                FieldValue::U8(n) => n,
                _ => 0,
            };
            let zero = matches!(state.get_at(p.field("zero")?), FieldValue::Bool(true));
            Some(match (chip, pattern, zero) {
                (Some(id), _, _) => id,
                (None, n, _) if n > 0 => 0x8000 | (n as u16 - 1),
                (None, _, true) => 0,
                _ => 0xFFFF,
            })
        })
        .collect()
}

/// The auto battle data as the consoles sent it (the setup's `ai_lists`:
/// each block's 42 places, the entries packed to the front, and their count
/// at +0x54) against the rules' send ([`sent_places`]), as the round starts.
/// (The block's pattern records, from +0x58, are the save's: the send
/// leaves them, and the rules read them from the setup.)
fn sent_auto_battle_differences(b: &Battle, compat: &Compat, lists: &[String; 2]) -> Vec<String> {
    let mut d = Vec::new();
    for side in 0..2u8 {
        let Ok(block) = unhex(&lists[side as usize]) else { continue };
        if block.len() < 0x58 {
            d.push(format!("side {side}'s sent auto battle data: a block of {:#x} bytes", block.len()));
            continue;
        }
        let theirs: Vec<u16> = (0..42).map(|i| u16::from_le_bytes([block[2 * i], block[2 * i + 1]])).collect();
        let count = u32::from_le_bytes([block[0x54], block[0x55], block[0x56], block[0x57]]);
        let Some(mut ours) = sent_places(b, compat, side) else {
            d.push(format!("side {side}'s sent auto battle data: the rules keep no `auto_battle_places`"));
            continue;
        };
        let n = ours.len() as u32;
        ours.resize(42, 0xFFFF);
        let show = |v: &[u16]| v.iter().map(|x| format!("{x:04x}")).collect::<Vec<_>>().join(" ");
        if (n, &ours) != (count, &theirs) {
            d.push(format!("side {side}'s sent auto battle data: ours {n}: {} theirs {count}: {}", show(&ours), show(&theirs)));
        }
    }
    d
}

/// The engine's hand in a chip block's terms ([`codec::ChipHand`]): each
/// chip by its key and the number compat's chips.toml gives it (0xFFFF for a
/// chip it has none for, which no block holds).
fn engine_hand(b: &Battle, compat: &Compat, h: &nettai_battle::hand::ChipHand) -> ChipHand {
    let chip = |c: nettai_content_api::ChipHandle| {
        let key = nettai_match::ids::local(&b.content.defs.chip(c).key).to_string();
        codec::Chip { id: compat.chip_entry(&key).map_or(0xFFFF, |e| e.id), key: Some(key) }
    };
    ChipHand {
        cursor: h.cursor,
        ids: h.ids.map(|c| c.map(chip)),
        damage: h.damage,
        attack_bonus: h.attack_bonus,
        charge_bonus: h.charge_bonus,
        selection: h.selection.map(|c| c.map(|c| (chip(c.id), c.code.0))),
        turn: h.turn,
        modifiers: h.modifiers,
    }
}

/// A hand's fields, each as the comparison says it: the cursor (the next
/// chip's place: the chips before it are used), the six chips (`-`: none; a
/// block keeps what lay past its first empty entry), their damage and their
/// two bonuses, the six picks with their codes, the turn each was picked in
/// and their modifier flags.
fn hand_fields(h: &ChipHand) -> Vec<(&'static str, String)> {
    let chip = |c: &codec::Chip| match &c.key {
        Some(k) => format!("{k} ({:#05x})", c.id),
        None => format!("{:#05x}", c.id),
    };
    let chips: Vec<String> = h.ids.iter().map(|c| c.as_ref().map_or("-".to_string(), chip)).collect();
    let picks: Vec<String> = h.selection.iter().map(|c| c.as_ref().map_or("-".to_string(), |(c, code)| format!("{} code {code}", chip(c)))).collect();
    vec![
        ("cursor", h.cursor.to_string()),
        ("chips", chips.join(", ")),
        ("damage", format!("{:?}", h.damage)),
        ("attack bonus", format!("{:?}", h.attack_bonus)),
        ("charge bonus", format!("{:?}", h.charge_bonus)),
        ("picks", picks.join(", ")),
        ("turns", format!("{:?}", h.turn)),
        ("modifiers", format!("{:?}", h.modifiers)),
    ]
}

/// [`compare`], the banner compared with `banner`'s and what the recording
/// console's screen's OK changes on that console alone with `screen`'s
/// ([`Round::local_screen_frame`]).
fn compare_with(b: &Battle, f: &Frame, banner: &Frame, screen: &Frame, compat: &Compat) -> Vec<String> {
    let mut d = Vec::new();
    let mut check = |what: &str, ours: String, theirs: String| {
        if ours != theirs {
            d.push(format!("{what}: ours {ours} theirs {theirs}"));
        }
    };
    let r = &b.round;
    check("state", format!("{:?}", [r.top, r.mode, r.sub, r.init]), format!("{:?}", f.state));
    check("rng2", format!("{:#010x}", b.rng.state), format!("{:#010x}", f.rng2));
    check("paused", (b.paused as u8).to_string(), f.paused.to_string());
    // BattleState's own, at EXE4's offsets (verify's census,
    // docs/replay-coverage.md §3.1): the actors and the alive navis by side
    // (+0x04, +0x12), the custom screens so far (+0x08), the time-up mark
    // (+0x0B), the two counts of fight ticks (+0x0E, +0x16), the winner
    // (+0x10), the set's score (+0x18), the delay (+0x28), the flags
    // (+0x32) but the gauge's (below) and the stage's flag's (0x08, which a
    // broken flag sets, 0x080E0842, and nothing in EXE4 reads: the engine
    // keeps none, docs/design/exe4-map.md §18.3) and the fight's ticks
    // (+0x40); and
    // the intro bits' low nibble (+0x64: the fade done, the navis in, each
    // side's chips enabled, which SignlRed's signal clears and sets; the
    // engine's 0x10, the fade's start, has no bit there).
    let gauge_full = nettai_battle::battle::battle_flags::GAUGE_FULL;
    const FLAG_BROKEN: u16 = 0x08;
    if let Ok(bs) = unhex(&f.bs) {
        if bs.len() >= 0xF0 {
            check("actors", format!("{:?}", r.actor_count), format!("{:?}", [bs[0x04], bs[0x05]]));
            check("turn", r.turn.to_string(), bs[0x08].to_string());
            check("time up", r.time_up.to_string(), bs[0x0B].to_string());
            check("fight cycles", format!("{:?}", r.cycles), format!("{:?}", [bs[0x0E], bs[0x16]]));
            check("winner", r.winner.to_string(), bs[0x10].to_string());
            check("alive", format!("{:?}", r.alive), format!("{:?}", [bs[0x12], bs[0x13]]));
            check("score", format!("{:?}", [r.wins, r.losses]), format!("{:?}", [bs[0x18], bs[0x19]]));
            check("delay", r.delay.to_string(), i16::from_le_bytes([bs[0x28], bs[0x29]]).to_string());
            let flags = u16::from_le_bytes([bs[0x32], bs[0x33]]);
            check("battle flags", format!("{:#x}", r.flags & !gauge_full), format!("{:#x}", flags & !(gauge_full | FLAG_BROKEN)));
            check("battle time", r.battle_time.to_string(), u32::from_le_bytes([bs[0x40], bs[0x41], bs[0x42], bs[0x43]]).to_string());
            check("intro bits", format!("{:#x}", r.intro_bits & 0x0F), format!("{:#x}", bs[0x64]));
        }
    }
    // The fighting machine (`fight`, 0xC bytes): its state, sub-state and
    // entry mark (+0, +2, +3), its result (+4), its timer (+8) and the turn
    // timer (+0x0A). EXE4's machine has no state for the turn's setup (no
    // sequencer: the turn starts in its start banner's state): its states
    // are the engine's, EXE6's, less 4 from the start banner on.
    if let Ok(fight) = unhex(&f.fight) {
        if fight.len() >= 0xC {
            let m = &b.fight;
            // (Not before a round's first fight: the fight's start zeroes
            // the machine, on the tick after its entry, and until then the
            // recording's is the last round's, left as that round ended.)
            let untouched = m.state == 0 && m.sub == 0 && m.init == 0 && m.result == 0 && m.pausing_player == 0 && m.timer == 0 && m.turn_timer == 0;
            let before_fight = untouched && (r.mode != nettai_battle::battle::mode::FIGHTING || r.init == 0);
            let word = |at: usize| u16::from_le_bytes([fight[at], fight[at + 1]]);
            let fields = [
                ("state", m.state.saturating_sub(4) as u16, fight[0] as u16),
                ("sub-state", m.sub as u16, fight[2] as u16),
                ("entry mark", m.init as u16, fight[3] as u16),
                ("result", m.result as u16, fight[4] as u16),
                ("timer", m.timer as u16, word(8)),
                ("turn timer", m.turn_timer, word(0x0A)),
            ];
            for (name, ours, theirs) in fields.into_iter().filter(|_| !before_fight) {
                check(&format!("fight {name}"), format!("{ours:#x}"), format!("{theirs:#x}"));
            }
        }
    }
    // The recording console's own Regular chip still to come (+0x17), which
    // its screen's OK clears: compared with the recording of the frame the
    // engine's screen stands at (`screen`).
    if let Ok(theirs) = unhex(&screen.bs) {
        if theirs.len() >= 0xF0 {
            let local = r.local_side as usize & 1;
            check("Regular chip to come", (b.custom.sides[local].folder.regular_pending as u8).to_string(), theirs[0x17].to_string());
        }
    }
    // The flags' gauge-full mark, the recording console's own as its gauge
    // is (it keeps both until its own send, 0x0801E1B4), compared as the
    // gauge, with the recording of the frame the console's late screen is
    // at. (While the engine's gauge for the console isn't the battle's,
    // emptied at the other side's send, the battle's mark isn't the
    // console's and isn't compared.)
    if let Ok(late) = unhex(&banner.bs) {
        if late.len() >= 0xF0 {
            if b.gauge_for(r.local_side) == b.gauge.value {
                let theirs = u16::from_le_bytes([late[0x32], late[0x33]]) & gauge_full != 0;
                check("gauge full", ((r.flags & gauge_full != 0) as u8).to_string(), (theirs as u8).to_string());
            }
        }
    }
    // The gauge as the recording console holds it (EXE4 keeps it full until
    // its own send), on the frame its screen is as late as the banner.
    check("gauge", format!("{:#x}", b.gauge_for(r.local_side)), format!("{:#x}", banner.gauge));
    // The HUD's pieces by their tasks (EXE4's HUD's task mask, +0x48): on
    // the banner block a banner's (bit 5; EXE6's bit 15) or the recording
    // console's own telop's, and the second block's (the other player's
    // telop, or the name of the chip that player just used); and, while a
    // banner shows, the number its block holds (its +1).
    let tasks = banner.hud_tasks;
    let own_telop = b.banner.shows_telop();
    let shows = |ours: bool, task: u32| ((ours as u8).to_string(), ((tasks & task != 0) as u8).to_string());
    let (ours, theirs) = shows(b.banner.active && !own_telop, BANNER_TASK);
    check("banner", ours, theirs);
    let (ours, theirs) = shows(own_telop, TELOP_TASK);
    check("telop", ours, theirs);
    let (ours, theirs) = shows(b.remote_telop.active || b.used_chip_for(r.local_side).is_some(), SECOND_BLOCK_TASK);
    check("second block", ours, theirs);
    if let (Some(id), true) = (b.banner_for(r.local_side), tasks & BANNER_TASK != 0) {
        let ours = b.content.assets.number(nettai_content_api::AssetKind::Banner, id.0).map(|n| n.id as u8);
        let theirs = unhex(&banner.banner).ok().and_then(|x| x.get(1).copied());
        let show = |n: Option<u8>| n.map_or("none".to_string(), |n| format!("{n:#04x}"));
        check("banner number", show(ours), show(theirs));
    }
    // What the NaviCust and the patch cards compile into each side's stats.
    for (side, hex) in f.navi_stats.iter().enumerate() {
        let Ok(raw) = unhex(hex) else { continue };
        let Ok(raw) = <[u8; crate::codec::NAVI_STATS]>::try_from(raw.as_slice()) else { continue };
        check(&format!("side {side}'s compiled stats"), compiled(b, &b.stats[side]), compiled_of(&crate::codec::navi_stats(&raw)));
        // (And its mood, +0x00, which the fight changes.)
        check(&format!("side {side}'s mood"), format!("{:#04x}", b.stats[side].mood), format!("{:#04x}", raw[0x00]));
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
    // Each side's hand, its whole chip block (0x50 bytes a side, EXE6's
    // layout): the cursor, the chips in order, their damage and bonuses, the
    // picks with their codes, their turns and their modifier flags; and the
    // block's bytes no field covers (+1, +0x4A on), zero in every recording.
    for (side, block) in f.chip_blocks.iter().enumerate() {
        let decoded = unhex(block).and_then(|raw| codec::chip_hand(compat, &raw).map(|h| (h, raw)));
        match decoded {
            Ok((theirs, raw)) => {
                let ours = engine_hand(b, compat, &b.hands[side]);
                for ((name, x), (_, y)) in hand_fields(&ours).into_iter().zip(hand_fields(&theirs)) {
                    check(&format!("hand {side} {name}"), x, y);
                }
                let rest: Vec<u8> = std::iter::once(raw[1]).chain(raw[0x4A..].iter().copied()).collect();
                check(&format!("hand {side}'s bytes no field covers"), format!("{:?}", [0u8; 7]), format!("{rest:?}"));
            }
            Err(e) => check(&format!("hand {side}"), "a chip block".to_string(), e),
        }
    }
    let entry_of =|o: nettai_battle::object::ObjectRef| compat.kinds.get(&b.content.defs.kind(b.objects.get(o).kind).key);
    let entries: Vec<Option<&crate::KindEntry>> = b.objects.in_order().map(entry_of).collect();
    // (Whether each object's owner, its first related object, is of a kind
    // whose position is scratch.)
    let owners_scratch: Vec<bool> = b
        .objects
        .in_order()
        .map(|o| b.objects.get(o).related[0].and_then(entry_of).is_some_and(|k| k.scratch_position))
        .collect();
    let panel_garbage = |i: usize| entries.get(i).copied().flatten().is_some_and(|k| k.scratch_panel);
    let skip = |i: usize, flags: u8| -> (bool, bool) {
        let Some(Some(k)) = entries.get(i) else { return (false, false) };
        let garbage = k.scratch_position
            || (k.scratch_position_without_sprite && flags & nettai_battle::object::flags::NO_SPRITE_UPDATE != 0)
            || (k.scratch_with_owner && owners_scratch.get(i).copied().unwrap_or(false));
        (garbage, k.scratch_z_fraction)
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
    type Fields = Vec<(&'static str, String)>;
    let ours: Vec<Fields> = b
        .objects
        .in_order()
        .enumerate()
        .map(|(i, o)| {
            let x = b.objects.get(o);
            let key = &b.content.defs.kind(x.kind).key;
            let kind = match entries[i] {
                Some(k) => format!("type {} #{:#04x}", type_of_pool(o.pool), k.index),
                None => format!("type {} {key} (no EXE4 number)", type_of_pool(o.pool)),
            };
            let (garbage, zf) = skip(i, x.flags);
            let xy = nettai_battle::kinds::effect::xy_unknown(b, o);
            let action = match compat.navi_action(b, o) {
                Ok(n) => format!("{n:#04x}"),
                Err(e) => format!("? ({e})"),
            };
            let flags = format!("{:#x}", x.collision.map(|c| b.collision.get(c).acc.hit_flags).unwrap_or(0));
            let f1 = format!("{:#x}", x.collision.map(|c| b.collision.get(c).f1).unwrap_or(0));
            let palette = b.objects.sprite(o).look.palette.to_string();
            vec![
                ("kind", kind),
                ("flags", format!("{:#04x}", x.flags)),
                ("state", format!("{:#04x}", x.state)),
                ("action", action),
                ("phase", format!("{:#04x}", x.phase)),
                ("phase init", format!("{:#04x}", x.phase_init)),
                ("panel", if panel_garbage(i) { "-".to_string() } else { format!("{:?}", [x.panel.x, x.panel.y]) }),
                ("side", x.alliance.to_string()),
                ("element", format!("{:#04x}", x.element)),
                ("hp", format!("{}/{}", x.hp, x.max_hp)),
                ("pos", pos([x.pos.x, x.pos.y, x.pos.z], garbage, xy, zf)),
                ("timer", x.timer.to_string()),
                ("anim", x.anim.to_string()),
                ("status", flags),
                ("f1", f1),
                ("palette", palette),
                // (A navi's four parameter bytes, +4: the one kind whose
                // object keeps the original's; the other kinds take their
                // parameters as their own state.)
                ("params", if type_of_pool(o.pool) == 1 && entries[i].is_some_and(|k| k.index == 0) { format!("{:#010x}", u32::from_le_bytes(x.params)) } else { "-".to_string() }),
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
                ("panel", if panel_garbage(i) { "-".to_string() } else { format!("{:?}", o.panel) }),
                ("side", o.alliance.to_string()),
                ("element", format!("{:#04x}", o.flip)),
                ("hp", format!("{}/{}", o.hp, o.max_hp)),
                ("pos", pos(o.pos, garbage, xy, zf)),
                ("timer", o.timer.to_string()),
                ("anim", o.anim.to_string()),
                ("status", format!("{:#x}", o.status)),
                ("f1", format!("{:#x}", o.f1)),
                ("palette", o.sprite[0].to_string()),
                ("params", if o.kind == 1 && o.index == 0 { format!("{:#010x}", o.params) } else { "-".to_string() }),
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
    /// The round's setup: what the engine needs that isn't there.
    Setup(String),
    /// The engine panicked on this frame.
    Panic { frame: u32, message: String },
    /// The engine differs from the recording on this frame.
    Differs { frame: u32, differences: Vec<String> },
}

/// Replay a round on `content`: its setup, then each battle frame ticked
/// and compared, up to the first difference. The recording console's RNG1
/// is compared too, as exe5-compat's `run_round` does: a difference still
/// there at the round's end stops the round at the frame it began.
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
    // What the rules compiled from the recording's NaviCusts (rules/navicust), against the stats the console's
    // reload left (the setup's blocks).
    if let Ok(d) = decode_setup(&round.setup) {
        let differences: Vec<String> = (0..2)
            .filter_map(|side| {
                let (ours, theirs) = (compiled(&b, &b.stats[side]), compiled_of(&d.navi_stats[side]));
                (ours != theirs).then(|| format!("side {side}'s compiled stats: ours {ours} theirs {theirs}"))
            })
            .collect();
        if !differences.is_empty() {
            replay.stopped = Some(Stop::Differs { frame: round.setup.frame, differences });
            return replay;
        }
    }
    if let Some(lists) = &round.setup.ai_lists {
        let differences = sent_auto_battle_differences(&b, compat, lists);
        if !differences.is_empty() {
            replay.stopped = Some(Stop::Differs { frame: round.setup.frame, differences });
            return replay;
        }
    }
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
            if nettai_battle::rng::Rng::new(frames[i].rng1).passes(engine, next.rng1, RNG1_PICKS_A_FRAME) {
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
            "rng1: the recording console's RNG1 differs from frame {frame} to the round's end (engine {engine:#010x}, recording {trace:#010x} at frame {})",
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

/// The most picks a console draws from its RNG1 in a frame that a replay's
/// check looks along (the main loop's one, a camera shake's two, an
/// emotion window's flicker: a few).
const RNG1_PICKS_A_FRAME: u32 = 64;
