//! The chip lab's EXE5 recordings: JSON lines with EXE6's line kinds (a
//! `setup` line, `exchange` lines, one line per frame) read from EXE5's RAM
//! (the verification workspace's oracle-trace `EXE5` layout), their
//! decoding into the engine's terms ([`codec`](crate::codec)), and their
//! replay: a round's setup, then each battle frame ticked and compared
//! ([`run_round`]), as far as the engine gets (docs/design/exe5-map.md
//! §15.5: the setup stops at what EXE5's content doesn't define yet).
//!
//! The DS's party battle (EXE5 DS, the engine's docs/design/exe5ds-map.md)
//! records the same lines over the DS's RAM (`"game":"exe5ds"`: the
//! verification workspace's oracle-trace `ds` module), with the party's own
//! on top: the setup's member blocks, party blocks and panels, `party` lines
//! when they change, and each frame's party blocks, switch inputs and the
//! navis' switch state. A DS round replays as a GBA one, its party from its
//! member blocks and each tick's switch request and microphone level from
//! the side's input record.

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
    /// The DS's party battle (`"game":"exe5ds"`): its third RNG; each side's
    /// member blocks (0x120 bytes: three NaviStats, MegaMan's then the two
    /// team navis'), party block (0x24 bytes: +0xB the rescue spent, +0x15
    /// the member count) and bottom-screen party panel (0x10 bytes: +4 the
    /// active member, +0xD the limit, +0xE the switches used), hex; the
    /// team words (0x1C bytes), hex.
    #[serde(default)]
    pub rng3: Option<u32>,
    #[serde(default)]
    pub members: Option<[String; 2]>,
    #[serde(default)]
    pub party: Option<[String; 2]>,
    #[serde(default)]
    pub panels: Option<[String; 2]>,
    #[serde(default)]
    pub team: Option<String>,
}

/// The game a DS recording's setup names.
pub const DS_GAME: &str = "exe5ds";

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
    /// The DS's party battle: the third RNG, both party blocks (hex), each
    /// side's switch input (its input record's +8, the member + 1 or 0;
    /// +0xA, its new edge; +9, the microphone's level) and each side's
    /// navi's switch state (AIData +0x44, +0x48 and +0x70..+0x73 as hex;
    /// null without a navi).
    #[serde(default)]
    pub rng3: Option<u32>,
    #[serde(default)]
    pub party: Option<[String; 2]>,
    #[serde(default)]
    pub switch: Option<[[u8; 3]; 2]>,
    #[serde(default)]
    pub party_ai: Option<[Option<(u32, u32, String)>; 2]>,
    /// The traced console's ROM (its version, and whether it is Japanese):
    /// `rounds` gives it its round's setup's; none, Team ProtoMan US's.
    #[serde(skip)]
    pub console: Option<(Version, bool)>,
    /// A DS recording's frame (its round's setup is `"game":"exe5ds"`).
    #[serde(skip)]
    pub ds: bool,
}

/// A custom-screen exchange: both sides' NaviStats and transform records.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exchange {
    pub frame: u32,
    pub navi_stats: [String; 2],
    pub transform: [String; 2],
}

/// The DS's party, written when it changes: both sides' member blocks, the
/// soul and switch marks, both consoles' party panels (hex; a panel empty
/// where its console has none).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyRecord {
    pub frame: u32,
    pub members: [String; 2],
    pub marks: String,
    pub panels: [String; 2],
}

#[derive(Clone, Debug)]
pub enum Line {
    Setup(Box<Setup>),
    Exchange(Exchange),
    Party(Box<PartyRecord>),
    Frame(Box<Frame>),
}

#[derive(Deserialize)]
struct PartyLine {
    party: PartyRecord,
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
    } else if l.starts_with("{\"party\"") {
        Ok(Line::Party(Box::new(serde_json::from_str::<PartyLine>(l).map_err(|e| format!("party: {e}"))?.party)))
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
    /// A DS recording's (the party battle's): its member blocks by side
    /// (MegaMan's, then the two team navis'), and its members by side (the
    /// party blocks' +0x15).
    pub ds: Option<DsSetup>,
}

/// What a DS recording's setup adds.
#[derive(Clone, Debug)]
pub struct DsSetup {
    pub members: [[NaviStats; 3]; 2],
    pub counts: [u8; 2],
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
    let ds = s.game == DS_GAME;
    if s.game != crate::ROOT && !ds {
        return Err(format!("an {} recording", s.game));
    }
    // (The DS is both teams' game: its stages and addresses are its own,
    // and its compat lookups go by Team ProtoMan's US ROM's.)
    let version = |v: &str| match v {
        "protoman" => Ok(Version::Protoman),
        "colonel" => Ok(Version::Colonel),
        DS_GAME if ds => Ok(Version::Protoman),
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
        ds: if ds { Some(decode_ds_setup(s)?) } else { None },
    })
}

/// A side's member blocks (0x120 bytes, hex): three NaviStats.
fn member_blocks(hex: &str) -> Result<[NaviStats; 3], String> {
    let b = unhex(hex)?;
    if b.len() != 3 * NAVI_STATS {
        return Err(format!("member blocks of {:#x} bytes", b.len()));
    }
    let block = |m: usize| codec::navi_stats(b[m * NAVI_STATS..(m + 1) * NAVI_STATS].try_into().expect("a block"));
    Ok([block(0)?, block(1)?, block(2)?])
}

/// The party's part of a DS setup: the member blocks and the members'
/// count (each party block's +0x15).
fn decode_ds_setup(s: &Setup) -> Result<DsSetup, String> {
    let (Some(members), Some(party)) = (&s.members, &s.party) else {
        return Err("a DS recording's setup without its member and party blocks".into());
    };
    let count = |hex: &str| -> Result<u8, String> {
        let b = unhex(hex)?;
        b.get(0x15).copied().ok_or_else(|| format!("a party block of {:#x} bytes", b.len()))
    };
    Ok(DsSetup { members: [member_blocks(&members[0])?, member_blocks(&members[1])?], counts: [count(&party[0])?, count(&party[1])?] })
}

/// A DS party line, decoded: both sides' member blocks and both panels.
pub fn decode_party(p: &PartyRecord) -> Result<([[NaviStats; 3]; 2], [Vec<u8>; 2]), String> {
    Ok(([member_blocks(&p.members[0])?, member_blocks(&p.members[1])?], [unhex(&p.panels[0])?, unhex(&p.panels[1])?]))
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
            Line::Party(p) => decode_party(p).map(|_| ()),
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
    /// A DS recording's party lines (before the first setup: the first
    /// round's).
    pub parties: Vec<PartyRecord>,
}

/// A recording's rounds: a setup line starts one; exchanges before the
/// first setup are the first round's; frames before it (the intro's) are
/// none of a round's.
pub fn rounds(path: impl AsRef<std::path::Path>) -> Result<Vec<Round>, String> {
    let mut rounds: Vec<Round> = Vec::new();
    let mut pending = Vec::new();
    let mut pending_parties = Vec::new();
    // (Each frame the traced console's ROM, by its round's setup.)
    let mut console = None;
    let mut ds = false;
    for line in read(path)? {
        match line {
            Line::Setup(s) => {
                console = decode_setup(&s).ok().map(|d| d.traced_rom());
                ds = s.game == DS_GAME;
                rounds.push(Round {
                    setup: *s,
                    exchanges: std::mem::take(&mut pending),
                    frames: Vec::new(),
                    parties: std::mem::take(&mut pending_parties),
                })
            }
            Line::Exchange(e) => match rounds.last_mut() {
                Some(r) => r.exchanges.push(e),
                None => pending.push(e),
            },
            Line::Party(p) => match rounds.last_mut() {
                Some(r) => r.parties.push(*p),
                None => pending_parties.push(*p),
            },
            Line::Frame(mut f) => {
                if let Some(r) = rounds.last_mut() {
                    f.console = console;
                    f.ds = ds;
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
        // (The DS's link delivers a console's packet 5 ticks after it goes
        // out, as its recordings show: the keys its custom screen read.)
        if self.setup.game == DS_GAME { DS_LINK_DELAY } else { 4 }
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
        decode_setup(&self.setup).map_or(0, |d| d.battle_state[0x0D] as usize & 1)
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

    /// Whether the `i`th of `frames` is a DS link stall: a frame whose
    /// battle update didn't run (BattleState +0x64, the running state's tick
    /// count, held from the frame before), the DS's wait for the link's
    /// packet, which its frame routine makes as the GBA's cable never did.
    pub fn stalled(&self, i: usize, frames: &[&Frame]) -> bool {
        let f = frames[i];
        f.ds && i > 0 && f.ticks == frames[i - 1].ticks && f.state[0] == 4 && frames[i - 1].state[0] == 4
    }

    /// A DS recording's custom screen: the direction its player presses on
    /// the next frame, fed a frame early. The DS's joypad (its ARM input
    /// code, not the battle's) repeats a direction on its first frame, where
    /// the GBA's repeats it on the second, which the engine's screen reads
    /// (`Joypad::repeat`): fed a frame early, the engine's cursor moves on
    /// the frame the DS's did. Only while a custom screen takes the side's
    /// keys before its OK.
    fn ds_early_direction(&self, i: usize, frames: &[&Frame], side: usize) -> u16 {
        const DIRECTIONS: u16 = 0xF0;
        let f = frames[i];
        if !f.ds || i + 1 >= frames.len() {
            return 0;
        }
        let Some(opened) = self.screen_opened(f.frame) else { return 0 };
        if self.screen_ok(opened, side).is_some_and(|ok| f.frame + 1 >= ok) {
            return 0;
        }
        let now = self.fed(i, frames, side);
        let next = self.fed(i + 1, frames, side);
        next & !now & DIRECTIONS
    }

    /// Inputs and events for the `i`th of `frames`.
    pub fn tick_inputs(&self, i: usize, frames: &[&Frame]) -> ([PlayerTick; 2], TickEvents) {
        let f = frames[i];
        // (A DS recording's switch request and microphone level are its
        // input record's on the frame, +8 the member + 1 and +9, which both
        // its fight and its custom screen read.)
        let input = std::array::from_fn(|p| {
            let mut t = PlayerTick::buttons(self.fed(i, frames, p) | self.ds_early_direction(i, frames, p));
            if let Some(sw) = f.switch {
                t.switch_to = sw[p][0].checked_sub(1);
                t.mic_level = sw[p][2];
            }
            t
        });
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
        match self.stage_key(content, compat, &d) {
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
        let stage = self.stage_key(content, compat, &d).and_then(|k| content.defs.stage_by_key(&k)).expect("needs saw the stage");
        // (The background by its name in compat: a liberation's map's number
        // is its area's background, which the pack has under the area's
        // first number.)
        let background = nettai_battle::content::BackgroundId(
            compat
                .background(st[4])
                .and_then(|name| content.assets.handle(nettai_content_api::AssetKind::Background, name))
                .ok_or_else(|| format!("EXE5's pack has no background {:#04x}", st[4]))?,
        );
        let mut effects = u32::from_le_bytes([st[8], st[9], st[10], st[11]]);
        if d.ds.is_some() {
            // (The DS's 0x200000 says Navi Select's partners join, where the
            // GBA's says a random battle: a party battle's are its
            // members.)
            effects &= !DS_NAVI_SELECT;
        }
        let settings = nettai_battle::BattleSettings { stage, background, effects };
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
            patch_cards(content, compat, &list, d.versions[side])
        };
        let players = [0u8, 1].map(|side| -> Result<PlayerSetup, String> {
            // (BattleState+0x17 is the recording console's Regular chip's
            // flag; the other console's is in `regular_flags` when the
            // recording has it.)
            let regular = match self.setup.regular_flags {
                Some(r) => r[side as usize] != 0,
                None => side == local && bs[0x17] != 0,
            };
            let folder = battle_folder(content, compat, &unhex(&self.setup.folders[side as usize])?, regular)?;
            // (The console's counter before the round's first tick: one
            // less than on the setup's frame.)
            let frames = (self.setup.frame_counter as u32).wrapping_sub(1) & 0xFFFF;
            // The side's level (the rules' `level`): a team navi's
            // attacks go by it. (Nothing reads MegaMan's side's: an older
            // recording, which has none, replays.)
            let level = match self.setup.navi_levels.map(|l| l[side as usize]) {
                // (A DS side without partners reads no level.)
                Some(_) if d.ds.as_ref().is_some_and(|ds| ds.counts[side as usize] <= 1) && d.navi_stats[side as usize].navi == 0 => None,
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
            // A DS side's party: its member blocks' team navis (members 1 and
            // 2, to the count), with no programs (their blocks, PartyCust's
            // made, go in as recorded: [`Round::start`]).
            if let Some(ds) = &d.ds {
                let mut partners = Vec::new();
                for m in 1..ds.counts[side as usize].min(3) as usize {
                    let n = ds.members[side as usize][m].navi;
                    let key = compat.navi_key(n).ok_or_else(|| format!("side {side}'s member {m}: navi {n:#04x} has no key"))?;
                    let navi = content.defs.navi_by_key(key).ok_or_else(|| format!("side {side}'s member {m}: the content has no navi {key}"))?;
                    partners.push(Fact::Record(vec![("navi", Fact::Value(Value::Def(Registry::Navi, navi.0))), ("programs", Fact::List(Vec::new()))]));
                }
                player.set_fact(content, "party", &partners)?;
            }
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
            let mut recorded = navi_stats(content, compat, &d.navi_stats[side])?;
            if d.ds.is_some() {
                ds_stats(content, &d.navi_stats[side], &mut recorded)?;
            }
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
            rng: match self.setup.send_rng2 {
                Some(r) => r,
                // (A DS recording carries no send's: its RNG2 before the
                // send's draws, which leave the recorded one.)
                None if d.ds.is_some() => rng2_before(self.setup.rng2, SEND_DRAWS),
                None => {
                    return Err("an EXE5 recording without send_rng2 and auto_battle (the RNG2 and the saves' auto battle data \
                                before the send): convert it with the verification workspace's tools/exe5/convert_send.py"
                        .into());
                }
            },
            local_side: bs[0x0D],
            score: nettai_battle::SetScore { wins: bs[0x18], losses: bs[0x19], round: bs[0x1A], max_combo: bs[0x1B] },
            // (A triple battle's: two rounds after the first.)
            later_stages: vec![nettai_battle::Stage { stage, background }; 2],
            low_hp_music_latched: bs[0x20] | bs[0x21] != 0,
            // (A console's battle: it pauses itself, and L is its run key.)
            host_keys: nettai_battle::setup::HostKeys::NONE,
            custom_screen_limit: None,
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
        // A DS side's partners: their blocks as recorded (the save's
        // records, PartyCust's programs in them, 0x02097BDC's copies), at
        // their maximum HP.
        if let Some(ds) = &d.ds {
            for side in 0..2 {
                for m in 1..b.party[side].count {
                    let mut stats = navi_stats(&b.content, compat, &ds.members[side][m as usize])?;
                    ds_stats(&b.content, &ds.members[side][m as usize], &mut stats)?;
                    // (A team navi enters at its maximum HP, whatever its
                    // save's: the round's first frame fills its block's, after
                    // the setup line's read.)
                    stats.hp = stats.max_hp;
                    *b.party[side].member_mut(m).expect("a member") = stats;
                }
            }
        }
        Ok(b)
    }

    /// The round's stage's key: the settings record's (by its layout and its
    /// actor list's address, a ROM's); a DS recording's by its layout and
    /// what its first battle frame has standing (the DS's actor lists are
    /// its own: the content stage of that layout whose navis stand where the
    /// recording's do, with as many other actors).
    fn stage_key(&self, content: &Content, compat: &Compat, d: &DecodedSetup) -> Option<String> {
        let st = &d.settings;
        if d.ds.is_none() {
            let actor_list = u32::from_le_bytes([st[12], st[13], st[14], st[15]]);
            let (version, japanese) = d.traced_rom();
            return compat.stage(st[0], actor_list, version, japanese);
        }
        let first = self.battle_frames().next()?;
        // (Its navis, in the object list's order, which is the actor list's;
        // and the field objects it placed, the attack pool's at the round's
        // start: rocks, boulders.)
        let navis: Vec<(u8, u8, u8)> =
            first.objects.iter().filter(|o| o.kind == 1 && o.index == 0).map(|o| (o.alliance, o.panel[0], o.panel[1])).collect();
        let mut placed: Vec<(u8, u8)> = first.objects.iter().filter(|o| o.kind == 3).map(|o| (o.panel[0], o.panel[1])).collect();
        placed.sort();
        compat.stages.iter().filter(|(_, e)| e.layout == st[0]).map(|(k, _)| k.clone()).find(|k| {
            let Some(h) = content.defs.stage_by_key(k) else { return false };
            let stage = content.stage(h);
            let theirs: Vec<(u8, u8, u8)> =
                stage.actors.iter().filter(|a| a.place == nettai_battle::content::Place::Navi).map(|a| (a.side, a.x, a.y)).collect();
            let mut others: Vec<(u8, u8)> =
                stage.actors.iter().filter(|a| a.place != nettai_battle::content::Place::Navi).map(|a| (a.x, a.y)).collect();
            others.sort();
            theirs == navis && others == placed
        })
    }
}

/// The DS's stats of a block beyond the GBA's, into EXE5's own (its rules'
/// `stats`): the party battle's switches beside the 3 its fresh stats write
/// (+0x17, which NaviCust's NavChg programs raise), a team navi's own
/// chip's bonus (+0x56, PartyCust's P.Chp) and its support flag (+0x59:
/// NaviCust's Spport, PartyCust's P.Spport).
fn ds_stats(content: &Content, s: &NaviStats, stats: &mut EngineNaviStats) -> Result<(), String> {
    let r = &s.raw;
    for (name, v) in [
        ("switch_bonus", Value::Int(r[0x17].saturating_sub(DS_SWITCHES) as i64)),
        ("chip_attack_bonus", Value::Int(u16::from_le_bytes([r[0x56], r[0x57]]) as i64)),
        ("supports_party", Value::Bool(r[0x59] != 0)),
    ] {
        stats.set_game_stat(content, name, v).map_err(|e| format!("EXE5's stat {name}: {e}"))?;
    }
    Ok(())
}

/// The DS's fresh stats' switches (+0x17, 0x0206E108).
const DS_SWITCHES: u8 = 3;

/// Each DS console's main loop draws its RNG1 twice a frame (the GBA's
/// once: `Rules::effects.rng1_per_frame`), whether or not the battle's
/// update ran.
const DS_MAIN_LOOP_DRAWS: u32 = 2;

/// `n` draws of each console's RNG1, the DS's main loop's beyond the
/// battle's.
pub fn ds_main_loop(b: &mut Battle, n: u32) {
    for c in &mut b.consoles {
        for _ in 0..n {
            c.rng.next();
        }
    }
}

/// The DS's link's delay, in ticks.
const DS_LINK_DELAY: u8 = 5;

/// The ticks the DS's link takes to carry a custom screen's result (its
/// packets carry a 16-byte block a tick: its recordings' fight resumes
/// with the engine's at 13, against the GBA's 49 words).
const DS_RESULT_WORDS: u8 = 13;

/// The DS's battle effect for Navi Select's partners (0x02097DD4), which
/// is the GBA's random battle's bit.
const DS_NAVI_SELECT: u32 = 0x20_0000;

/// The auto battle data's send draws RNG2 this many times (0x0802C7BE),
/// both consoles alike.
const SEND_DRAWS: u32 = 84;

/// The RNG2 `n` draws before `r` (the generator's step undone:
/// `rotate_left(1) + 1 ^ 0x873CA9E5`).
fn rng2_before(mut r: u32, n: u32) -> u32 {
    for _ in 0..n {
        r = (r ^ 0x873C_A9E5).wrapping_sub(1).rotate_right(1);
    }
    r
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
    compare_with(b, f, f, f, None, compat)
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
    let exchange = round.exchanges.iter().rfind(|e| e.frame <= f.frame);
    compare_with(b, f, banner, round.local_screen_frame(i, frames), exchange, compat)
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
                    Some(compat.chips.get(&key).map(|c| c.id).unwrap_or(0xFFFE))
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
/// leaves them, and the rules read them from the setup.) The places are
/// compared as a set: their order is the engine's shuffle's.
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
        let sorted = |v: &[u16]| {
            let mut v = v.to_vec();
            v.sort_unstable();
            v
        };
        if (n, sorted(&ours)) != (count, sorted(&theirs)) {
            d.push(format!("side {side}'s sent auto battle data: ours {n}: {} theirs {count}: {}", show(&ours), show(&theirs)));
        }
    }
    d
}

/// A side's stats as the comparison of an exchange line says them: the fields
/// [`navi_stats`] reads of a block (but the folder's, which the battle
/// folder holds, the starting form, the navi's base, and the HP: the block's
/// +0x40 is the save's, which a battle doesn't write, where the navi's own
/// is its object's, compared there; a compiled NaviCust's engine stats hold
/// the maximum), and `form`, the form by EXE5's number (NaviStats +0x2C).
fn stats_fields(s: &EngineNaviStats, form: Option<u8>) -> Vec<(&'static str, String)> {
    vec![
        ("form", format!("{form:?}")),
        ("mood", format!("{:#04x}", s.mood)),
        ("element", s.element.to_string()),
        ("navi", format!("{:?} variant {}", s.navi, s.navi_variant)),
        ("max HP", format!("{} (base {})", s.max_hp, s.max_base_hp)),
        ("buster", format!("{} {} {}", s.attack, s.rapid, s.charge)),
        ("levels", format!("custom {} mega {} giga {} reg-up {}", s.custom_level, s.mega_level, s.giga_level, s.reg_up)),
        ("gauge speed", format!("{:?}", s.gauge_speed)),
        ("first barrier", format!("{:?}", s.first_barrier)),
        ("support", format!("{:?}", s.support)),
        ("armor and shoes", format!("{} {} {} {}", s.super_armor, s.float_shoes, s.air_shoes, s.undershirt)),
        ("weapons", format!("{:?}", s.weapons)),
        // (But the four bug bytes [`navi_stats`] doesn't read of EXE5's
        // block, which the engine's patch cards may set: EXE6's places of
        // them, +0x11, +0x3D, +0x52 and +0x63, are to confirm in EXE5's.)
        ("bugs", format!("{:?}", NaviCustBugs { auto_step: 0, starting_damage: 0, status_immunity: false, hand_shrink_turn: 0, ..s.bugs })),
        ("chip recovery", s.chip_recovery.to_string()),
        ("its own stats", format!("{:?}", s.game)),
    ]
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
/// ([`Round::local_screen_frame`]); each side's stats with `exchange`'s, the
/// exchange line of the blocks' last change.
fn compare_with(b: &Battle, f: &Frame, banner: &Frame, screen: &Frame, exchange: Option<&Exchange>, compat: &Compat) -> Vec<String> {
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
    // BattleState's own, at EXE6's offsets: the actors and the alive navis
    // by side (+0x04, +0x12), the custom screens so far (+0x07), whether
    // the battle runs (+0x0A), the time-up mark (+0x0B), the two counts of
    // fight ticks (+0x0E, +0x16), the winner (+0x10), the set's score
    // (+0x18), the result (+0x1F), the delay (+0x28), the flags (+0x32) and
    // the fight's ticks (+0x40); and the intro bits' low nibble (+0x5C: the
    // fade done, the navis in, each side's chips enabled. Its other bits
    // are EXE5's own: 0x10 and 0x20 ColonelSoul's, 0x40 set from the start,
    // where the engine's 0x10 is the fade's start).
    if let Ok(bs) = unhex(&f.bs) {
        if bs.len() >= 0xF0 {
            check("actors", format!("{:?}", r.actor_count), format!("{:?}", [bs[0x04], bs[0x05]]));
            check("turn", r.turn.to_string(), bs[0x07].to_string());
            check("running", r.running.to_string(), bs[0x0A].to_string());
            check("time up", r.time_up.to_string(), bs[0x0B].to_string());
            check("fight cycles", format!("{:?}", r.cycles), format!("{:?}", [bs[0x0E], bs[0x16]]));
            check("winner", r.winner.to_string(), bs[0x10].to_string());
            check("alive", format!("{:?}", r.alive), format!("{:?}", [bs[0x12], bs[0x13]]));
            check("score", format!("{:?}", [r.wins, r.losses]), format!("{:?}", [bs[0x18], bs[0x19]]));
            // (And the round, +0x1A, as the round's start sets it.)
            check("round", r.round.to_string(), bs[0x1A].to_string());
            check("result", r.result.to_string(), bs[0x1F].to_string());
            check("delay", r.delay.to_string(), i16::from_le_bytes([bs[0x28], bs[0x29]]).to_string());
            check("battle flags", format!("{:#x}", r.flags), format!("{:#x}", u16::from_le_bytes([bs[0x32], bs[0x33]])));
            check("battle time", r.battle_time.to_string(), u32::from_le_bytes([bs[0x40], bs[0x41], bs[0x42], bs[0x43]]).to_string());
            check("intro bits", format!("{:#x}", r.intro_bits & 0x0F), format!("{:#x}", bs[0x5C] & 0x0F));
        }
    }
    // The fighting machine (`fight`, 0xC bytes): its state, sub-state and
    // entry mark (+0, +2, +3), its result (+4), the pausing player (+5), its
    // timer (+8) and the turn timer (+0x0A). EXE5's setup state marks its
    // second step in the byte at +0x0A (0x0800776A), where EXE6's marks it
    // in the sub-state and the engine does as EXE6: in that state the
    // engine's sub-state is compared with that byte; after it the mark
    // stays there (the state's word store clears +0 to +3 alone) until the
    // last turns' timer is written over it (0x080073E6) or the machine is
    // zeroed for the next turn, so a turn timer of none is a 4 there.
    if let Ok(fight) = unhex(&f.fight) {
        if fight.len() >= 0xC {
            let m = &b.fight;
            // (Not before a round's first fight: the fight's start zeroes
            // the machine, on the tick after its entry, and until then the
            // recording's is the last round's, left as that round ended.)
            let untouched = m.state == 0 && m.sub == 0 && m.init == 0 && m.result == 0 && m.pausing_player == 0 && m.timer == 0 && m.turn_timer == 0;
            let before_fight = untouched && (r.mode != nettai_battle::battle::mode::FIGHTING || r.init == 0);
            let word = |at: usize| u16::from_le_bytes([fight[at], fight[at + 1]]);
            let setup = m.state == nettai_battle::battle::fight::SETUP;
            let turn_timer = if setup || (m.turn_timer == 0 && word(0x0A) == 4) { m.turn_timer } else { word(0x0A) };
            let fields = [
                ("state", m.state as u16, fight[0] as u16),
                ("sub-state", m.sub as u16, if setup { fight[0x0A] as u16 } else { fight[2] as u16 }),
                ("entry mark", m.init as u16, fight[3] as u16),
                ("result", m.result as u16, fight[4] as u16),
                ("pausing player", m.pausing_player as u16, fight[5] as u16),
                ("timer", m.timer as u16, word(8)),
                ("turn timer", m.turn_timer, turn_timer),
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
    // Each side's hand, its whole chip block (0x50 bytes a side, EXE6's
    // layout): the cursor, the chips in order, their damage and bonuses, the
    // picks with their codes, their turns and their modifier flags; and the
    // block's bytes no field covers (+1, +0x4A on), zero in every recording.
    for (side, block) in f.chip_blocks.iter().enumerate() {
        let decoded = unhex(block).and_then(|raw| codec::chip_hand(compat, &raw).map(|h| (h, raw)));
        match decoded {
            Ok((mut theirs, raw)) => {
                // (A hand never built is the zeroed block: its picks read
                // chip 0 in code A where a built hand's read none. EXE5's
                // content names no zeroed chip, `roles.chips.zeroed`, so the
                // engine's hand reads no pick there either way: a pick of
                // chip 0 in code A is compared as none.)
                if b.content.zeroed_chip().is_none() {
                    for pick in &mut theirs.selection {
                        if matches!(pick, Some((c, 0)) if c.id == 0) {
                            *pick = None;
                        }
                    }
                }
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
    // Each side's stats as the block stands (the exchange line of its last
    // change), field by field as [`navi_stats`] reads a block, and its form
    // (+0x2C) by EXE5's number. (The exchanged transform records aren't
    // compared: a Soul Unison's record is the screen's own, 0x0203C940, and
    // the exchanged one's +0 reads 0xFF at every install of the lab's souls,
    // its +4 an address.)
    if let Some(e) = exchange {
        for p in 0..2 {
            let ours = &b.stats[p];
            let our_form = compat.form_number(nettai_match::ids::local(&b.content.defs.form(ours.form).key));
            let decoded = navi_stats_hex(&e.navi_stats[p]).and_then(|raw| {
                let mut s = navi_stats(&b.content, compat, &raw)?;
                if f.ds {
                    ds_stats(&b.content, &raw, &mut s)?;
                }
                Ok((raw.form, s))
            });
            match decoded {
                Ok((form, theirs)) => {
                    for ((name, x), (_, y)) in stats_fields(ours, our_form).into_iter().zip(stats_fields(&theirs, Some(form))) {
                        check(&format!("side {p}'s stats: {name}"), x, y);
                    }
                }
                Err(err) => check(&format!("side {p}'s stats"), "a NaviStats block".to_string(), err),
            }
        }
    }
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
    // The header-flag bits the other side's console reads off its open bus
    // on the tick the round's actors are spawned (the intro's init done, its
    // first step not yet: `round_start_open_bus_flags`), skipped on both
    // sides.
    let spawn_tick = [r.top, r.mode, r.sub, r.init] == [4, 0, 0, 4];
    let flags_field = |i: usize, flags: u8, alliance: u8| -> String {
        let skipped = match entries.get(i) {
            Some(Some(k)) if spawn_tick && alliance & 1 != r.local_side & 1 => k.round_start_open_bus_flags,
            _ => 0,
        };
        // (The DS's records have their header byte's bit 0 set, where the
        // GBA's have it clear: its heap pools' mark, which no battle code
        // reads.)
        let ds_mark = if f.ds { 0x01 } else { 0 };
        format!("{:#04x}", flags & !skipped & !ds_mark)
    };
    // The traced console's ROM: an object keeping a code address its
    // spawner left (the content's Team ProtoMan US's) has that ROM's
    // (games.toml).
    let rom = f.console.unwrap_or((Version::Protoman, false));
    // (A DS object without collision keeps whatever its heap slot held at
    // +0x54, which the recorder reads through as a collision.)
    let order_for_status: Vec<_> = b.objects.in_order().collect();
    let status_field = |i: usize, s: String| -> String {
        let no_collision = order_for_status.get(i).is_some_and(|&r| b.objects.get(r).collision.is_none());
        match entries.get(i) {
            Some(Some(k)) if k.scratch_status => "-".to_string(),
            _ if f.ds && no_collision => "-".to_string(),
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
                ("flags", flags_field(i, x.flags, x.alliance)),
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
                ("facing", x.flip.to_string()),
                // (A navi's four parameter bytes, +4: the one kind whose
                // object keeps the original's; the other kinds take their
                // parameters as their own state.)
                ("params", if pool_type(o.pool) == 1 && entries[i].is_some_and(|k| k.index == 0) { format!("{:#010x}", u32::from_le_bytes(x.params)) } else { "-".to_string() }),
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
                ("flags", flags_field(i, o.flags, o.alliance)),
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
                ("facing", o.flip.to_string()),
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
    // (A DS recording's link carries a custom screen's result in fewer
    // ticks than the GBA's words: the replay's content says the DS's.)
    let ds_content;
    let content = if round.setup.game == DS_GAME {
        let mut c = (**content).clone();
        c.rules_mut().flow.result_words = DS_RESULT_WORDS;
        ds_content = Arc::new(c);
        &ds_content
    } else {
        content
    };
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
    if let Some(lists) = &round.setup.ai_lists {
        let differences = sent_auto_battle_differences(&b, compat, lists);
        if !differences.is_empty() {
            replay.stopped = Some(Stop::Differs { frame: round.setup.frame, differences });
            return replay;
        }
    }
    // The recording console's RNG1 (the trace's `rng1`), sampled after the
    // battle's update and before the main loop's draw: the engine after
    // frame N, which made the frame's picks, the battle's and the main
    // loop's, is on the way from the trace at N to the trace at N + 1
    // (`Rng::passes`; at N + 1 when the next frame's battle update draws
    // none). A camera shake's picks come in the battle's update
    // (EXE6's `sub_802FFF4` after the objects), so through a shake the engine is
    // between the two samples, not at either: Timpani's drum shakes for
    // as long as it plays. A difference left at the round's end stops the
    // round at the frame it began; one that begins on the last frame
    // compared has no frame left to agree on, and is left.
    let local = b.setup.local_side as usize & 1;
    let mut rng1_since: Option<(u32, u32, u32)> = None;
    for i in 0..frames.len() {
        // (A DS link stall: the frame routine ran without the battle's
        // update, its tick count held, waiting on the other console's
        // packet. The engine has no link: it has no such frame.)
        if round.stalled(i, &frames) {
            ds_main_loop(&mut b, DS_MAIN_LOOP_DRAWS);
            replay.matched += 1;
            continue;
        }
        let (input, events) = round.tick_inputs(i, &frames);
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.tick(&input, events))) {
            replay.stopped = Some(Stop::Panic { frame: frames[i].frame, message: panic_message(&*e) });
            return replay;
        }
        if frames[i].ds {
            // (The engine's tick drew the GBA's main loop's one.)
            ds_main_loop(&mut b, DS_MAIN_LOOP_DRAWS - 1);
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

/// The most picks a console draws from its RNG1 in a frame that a replay's
/// check looks along (the main loop's one, a camera shake's two, an
/// emotion window's flicker: a few).
const RNG1_PICKS_A_FRAME: u32 = 64;
