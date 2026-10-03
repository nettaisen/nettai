//! The chip lab's BN5 recordings: JSON lines with BN6's line kinds (a
//! `setup` line, `exchange` lines, one line per frame) read from BN5's RAM
//! (the verification workspace's oracle-trace `BN5` layout), their
//! decoding into the engine's terms ([`codec`](crate::codec)), and their
//! replay: a round's setup, then each battle frame ticked and compared
//! ([`run_round`]), as far as the engine gets (docs/design/bn5-map.md
//! §15.5: the setup stops at what BN5's content doesn't define yet).

use crate::codec::{self, ChipHand, NAVI_STATS, NaviStats, Panel};
use crate::{Compat, pool_of_type, pool_slots};
use nettai_battle::content::ChipCode;
use nettai_battle::custom::{BattleFolder, FolderChip, PlayerSetup, Unlocks};
use nettai_battle::console::ConsoleSetup;
use nettai_battle::custom::GameVersion;
use nettai_battle::setup::NaviWeapons;
use nettai_battle::{Battle, Content, NaviStats as EngineNaviStats, PlayerTick, RoundSetup, TickEvents};
use nettai_content_api::{RecordHandle, WeaponHandle};
use nettai_content_api::Pool;
use serde::Deserialize;
use std::io::BufRead;
use std::sync::Arc;

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
    /// Both players' computer-navi data as the link exchanged it (0xE0
    /// bytes each, side 0's first), hex: their tactics. Older recordings
    /// have none.
    #[serde(default)]
    pub ai_lists: Option<[String; 2]>,
}

/// A player's computer-navi data block (0xE0 bytes, BN5's 0x02034C20 by
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
/// BN6's codec matches its stages), hex.
fn stage_bytes(settings: &[u8; 0x10]) -> String {
    [0usize, 2, 3, 5, 6, 12, 13, 14, 15].iter().map(|&i| format!("{:02x}", settings[i])).collect()
}

impl Round {
    /// The frames the engine simulates: the running and end states, from
    /// the setup's frame up to the next round's init (BN5's battle state
    /// machine has BN6's top states: 0 init, 4 running, 8 end).
    pub fn battle_frames(&self) -> impl Iterator<Item = &Frame> {
        self.frames.iter().filter(|f| f.frame >= self.setup.frame).take_while(|f| f.state[0] == 4 || f.state[0] == 8)
    }

    /// The link's delay: the chip lab's emulated cable's (BN5's recordings
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
    /// chip 0, no chip of BN5's).
    pub fn chip_ids(&self) -> Result<Vec<u16>, String> {
        let mut ids = Vec::new();
        let mut add = |v: u16| {
            if v != 0xFFFF && v & 0x1FF != 0 && !ids.contains(&(v & 0x1FF)) {
                ids.push(v & 0x1FF);
            }
        };
        for folder in std::iter::once(&self.setup.folder).chain(self.setup.folders.iter().flatten()) {
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
    /// (`chip bn5:cannon (0x001)`), BN5's navis, forms and stage (none of
    /// which content/bn5 defines yet).
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
                out.push(format!("side {side}'s soul {:#04x} (BN5's forms)", s.form));
            }
            if s.raw[0x4C] != 0 {
                out.push(format!("side {side}'s spread program (NaviStats +0x4C = {:#04x})", s.raw[0x4C]));
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
        match compat.stage(st[0], actor_list) {
            Some(k) if content.defs.stage_by_key(&k).is_some() => {}
            Some(k) => out.push(format!("the stage {k} (settings {})", stage_bytes(st))),
            None => out.push(format!("the stage (settings {}: no BN5 netbattle stage)", stage_bytes(st))),
        }
        if content.assets.pack(crate::ROOT).is_none() {
            out.push("BN5's pack".into());
        }
        out.dedup();
        Ok(out)
    }

    /// The engine's starting point for this round on `content`: what the
    /// round needs is defined ([`Round::needs`]), then BN5's records go
    /// into the engine's: the settings record's stage and background, both
    /// NaviStats ([`navi_stats`]), the folders, the RNGs, the set's score,
    /// both players on BN5's stock rules.
    pub fn round_setup(&self, content: &Content, compat: &Compat) -> Result<RoundSetup, String> {
        // BN5's light and dark system (content/bn5/rules/light-dark).
        const LIGHT_DARK: &str = "bn5:light-dark";
        let needs = self.needs(content, compat)?;
        if !needs.is_empty() {
            return Err(format!("content lacks {}", needs.join(", ")));
        }
        let d = decode_setup(&self.setup)?;
        let bs = &d.battle_state;
        let st = &d.settings;
        let actor_list = u32::from_le_bytes([st[12], st[13], st[14], st[15]]);
        let stage = compat.stage(st[0], actor_list).and_then(|k| content.defs.stage_by_key(&k)).expect("needs saw the stage");
        let pack = content.assets.pack(crate::ROOT).expect("needs saw the pack");
        let background = nettai_battle::content::BackgroundId(
            content
                .assets
                .number_handle(nettai_content_api::AssetKind::Background, pack, st[4] as u16)
                .ok_or_else(|| format!("BN5's pack has no background {:#04x}", st[4]))?,
        );
        let settings = nettai_battle::BattleSettings { stage, background, effects: u32::from_le_bytes([st[8], st[9], st[10], st[11]]) };
        let ruleset = content.defs.stock_ruleset_of(crate::ROOT).ok_or("the content has no BN5 stock ruleset")?;
        let local = bs[0x0D] & 1;
        let players = [0u8, 1].map(|side| -> Result<PlayerSetup, String> {
            let folder = match (&self.setup.folders, side == local) {
                (Some(f), _) => Some(battle_folder(content, compat, &unhex(&f[side as usize])?, side == local && bs[0x17] != 0)?),
                (None, true) => Some(battle_folder(content, compat, &unhex(&self.setup.folder)?, bs[0x17] != 0)?),
                (None, false) => None,
            };
            let frames = match self.setup.frame_counter {
                Some(c) => (c as u32).wrapping_sub(1) & 0xFFFF,
                None => self.battle_frames().next().map_or(0, |f| f.frame + 1),
            };
            Ok(PlayerSetup {
                folder,
                // No Cross, no Beast Out; BN5's Soul Unison as a finished
                // save has it (the save's event flags aren't in a
                // recording): the soul button, the version's six souls
                // (Team ProtoMan's 1 to 6, Team Colonel's 7 to 12:
                // 0x08024BF0's flags) and Chaos Unison.
                unlocks: Unlocks {
                    version: GameVersion::Falzar,
                    crosses: Default::default(),
                    beast_out: false,
                    beast_out_sealed: false,
                    cross_list: None,
                    souls: nettai_battle::custom::SoulUnlocks {
                        button: true,
                        owned: if d.versions[side as usize] == Version::Colonel { 0b1_1111_1000_0000 } else { 0b111_1110 },
                        chaos: true,
                        turn_bonus: d.navi_stats[side as usize].raw[0x32] as i8,
                    },
                },
                joypad_phase: self.setup.joypad_phases.map(|p| p[side as usize]).unwrap_or((self.setup.frame % 5) as u8),
                bug_frags: 0,
                navi_level: 0,
                console: ConsoleSetup {
                    rng: if side == local { self.setup.rng1 } else { 0 },
                    tag_pair: None,
                    emotion_window_glitch: false,
                    frames,
                },
                ruleset: Some(ruleset),
                rules: Vec::new(),
                patch_cards: Default::default(),
                // The stats are the save's (no NaviCust compiled over them).
                navicust: None,
                tactics: match &self.setup.ai_lists {
                    Some(lists) => tactics(content, compat, &unhex(&lists[side as usize])?)?,
                    None => Default::default(),
                },
            })
        });
        let [mut p0, mut p1] = players;
        // Each side's light and dark MegaMan: his save's value (NaviStats
        // +0x44) and Hub Style (+0x4C), BN5's light and dark system's setup.
        for (p, stats) in [(&mut p0, &d.navi_stats[0]), (&mut p1, &d.navi_stats[1])] {
            if let Ok(p) = p {
                p.set_rule(content, LIGHT_DARK, "value", nettai_content_api::Value::Int(stats.light_dark.0 as i64))?;
                p.set_rule(content, LIGHT_DARK, "hub_style", nettai_content_api::Value::Int(stats.hub_style as i64))?;
            }
        }
        Ok(RoundSetup {
            content: content.hash(),
            settings,
            navi_stats: [navi_stats(content, compat, &d.navi_stats[0])?, navi_stats(content, compat, &d.navi_stats[1])?],
            rng: self.setup.rng2,
            local_side: bs[0x0D],
            score: nettai_battle::SetScore { wins: bs[0x18], losses: bs[0x19], round: bs[0x1A], max_combo: bs[0x1B] },
            later_stages: [nettai_battle::Stage { stage, background }; 2],
            low_hp_music_latched: bs[0x20] | bs[0x21] != 0,
            sp_times: Default::default(),
            players: [p0?, p1?],
            link_delay: self.link_delay(),
        })
    }

    /// The round's battle at its start on `content`, with the counters its
    /// init carried in (BattleState +0x60, +0x64).
    pub fn start(&self, content: Arc<Content>, compat: &Compat) -> Result<Battle, String> {
        let setup = self.round_setup(&content, compat)?;
        let bs = decode_setup(&self.setup)?.battle_state;
        let mut b = Battle::new(setup, content);
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
/// BN5's chips' handles.
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

/// A side's stats in the engine's terms: BN5's NaviStats fields where the
/// engine has them (docs/design/bn5-map.md §3.3, §13), its navi, weapons,
/// programs and first barrier by compat (records.toml); MegaMan in the base
/// form (BN5's souls are its forms, to come); what BN5 has none of (BN6's
/// Beast Out counter, the sun, the version, the NaviCust's bugs: BN5's
/// bytes there aren't read) none.
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
        chip_drops: 0,
        encounters: 0,
        navi,
        navi_variant: s.navi_variant,
        form: base,
        folder: 0,
        folder_reg: [0xFF, 0xFF],
        max_base_hp: s.max_base_hp,
        hp: s.hp,
        max_hp: s.max_hp,
        chip_recovery: 0,
        folder_tags: [[0xFF, 0xFF], [0xFF, 0xFF]],
        chip_shuffle: false,
        number_open: false,
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
        bugs: Default::default(),
    })
}

/// BN5's navi numbers' keys in its root (NaviStats +0x29): MegaMan's.
/// (The Team Battle's navis come with their content.)
pub fn navi_key(n: u8) -> Option<String> {
    (n == 0).then(|| format!("{}:megaman", crate::ROOT))
}

/// Differences between the engine and a BN5 frame: the state machine and
/// its counters, the simulation RNG, the gauge, the panels (by BN5's
/// numbers, through compat) and the objects (pool, panel, side, HP,
/// position: BN5's kinds have no numbers in compat yet).
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
    // Each object as its pool and BN5's kind number (the engine's kinds by
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
    let ours: Vec<String> = b
        .objects
        .in_order()
        .enumerate()
        .map(|(i, o)| {
            let x = b.objects.get(o);
            let key = &b.content.defs.kind(x.kind).key;
            let kind = match entries[i] {
                Some(k) => format!("#{:#04x}", k.index),
                None => format!("{key} (no BN5 number)"),
            };
            let (garbage, zf) = skip(i, x.flags);
            let xy = nettai_battle::kinds::effect::xy_unknown(b, o);
            format!(
                "type {} {kind} panel {} side {} hp {}/{} pos {}",
                pool_type(o.pool),
                panel(i, [x.panel.x, x.panel.y]),
                x.alliance,
                x.hp,
                x.max_hp,
                pos([x.pos.x, x.pos.y, x.pos.z], garbage, xy, zf)
            )
        })
        .collect();
    let order: Vec<_> = b.objects.in_order().collect();
    let theirs: Vec<String> = f
        .objects
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let (garbage, zf) = skip(i, o.flags);
            let xy = order.get(i).is_some_and(|&r| nettai_battle::kinds::effect::xy_unknown(b, r));
            format!(
                "type {} #{:#04x} panel {} side {} hp {}/{} pos {}",
                o.kind,
                o.index,
                panel(i, o.panel),
                o.alliance,
                o.hp,
                o.max_hp,
                pos(o.pos, garbage, xy, zf)
            )
        })
        .collect();
    if ours != theirs {
        check("objects", format!("\n    ours   {}", ours.join("\n           ")), format!("\n    theirs {}", theirs.join("\n           ")));
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
        replay.matched += 1;
    }
    replay
}

/// A panic's message.
pub fn panic_message(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
}
