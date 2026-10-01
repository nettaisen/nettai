//! Reading a GBA ROM's M4A data into a [`SoundBank`]: the driver's sound
//! mode, its song and player tables, song headers, track command streams
//! and voicegroups. The one place that knows the ROM layout.

use crate::bank::*;
pub use crate::tables::CLOCK;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;

const ROM_BASE: u32 = 0x0800_0000;
const MAX_SONGS: usize = 1024;

/// Reading a ROM's sound data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RomError {
    TooSmall,
    /// No `m4aSongNumStart` found.
    NoDriver,
    NoSoundMode,
    /// Data at an address that isn't in the ROM.
    OutOfRange(u32),
    /// A track command the driver doesn't have, or this port doesn't play.
    Command {
        at: u32,
        byte: u8,
    },
    /// A data byte where a command must start (a jump target, or after a
    /// pattern call): its meaning would depend on the path taken there.
    RunningStatus(u32),
}

impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RomError::TooSmall => write!(f, "too small to be a GBA ROM"),
            RomError::NoDriver => write!(f, "no M4A sound driver found"),
            RomError::NoSoundMode => write!(f, "M4A sound mode not found"),
            RomError::OutOfRange(a) => write!(f, "no data at {a:#010x}"),
            RomError::Command { at, byte } => write!(f, "unsupported track command {byte:#04x} at {at:#010x}"),
            RomError::RunningStatus(a) => write!(f, "running status across a jump at {a:#010x}"),
        }
    }
}

impl std::error::Error for RomError {}

type Result<T, E = RomError> = std::result::Result<T, E>;

/// A ROM image addressed by GBA address.
struct Rom<'a>(&'a [u8]);

impl Rom<'_> {
    fn offset(&self, addr: u32) -> Result<usize> {
        let o = addr.wrapping_sub(ROM_BASE) as usize;
        // (Well inside the address space, so small offsets from it can't wrap.)
        if !(ROM_BASE..0x0E00_0000).contains(&addr) || o >= self.0.len() {
            return Err(RomError::OutOfRange(addr));
        }
        Ok(o)
    }
    fn bytes(&self, addr: u32, n: usize) -> Result<&[u8]> {
        let o = self.offset(addr)?;
        self.0.get(o..o + n).ok_or(RomError::OutOfRange(addr))
    }
    fn u8(&self, addr: u32) -> Result<u8> {
        Ok(self.bytes(addr, 1)?[0])
    }
    fn u16(&self, addr: u32) -> Result<u16> {
        Ok(u16::from_le_bytes(self.bytes(addr, 2)?.try_into().unwrap()))
    }
    fn u32(&self, addr: u32) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(addr, 4)?.try_into().unwrap()))
    }
    /// A pointer stored at `addr`, which must point into the ROM.
    fn ptr(&self, addr: u32) -> Result<u32> {
        let p = self.u32(addr)?;
        self.offset(p)?;
        Ok(p)
    }
}

/// Start of m4aSongNumStart: push {lr}; lsl r0,#16; ldr r2|r3,=mplayTable;
/// ldr r1,=songTable; lsr r0,#13; add r0,r1. Its literal pool holds the
/// player and song table pointers.
fn find_song_num_start(data: &[u8]) -> Option<usize> {
    const HEAD: [u8; 5] = [0x00, 0xB5, 0x00, 0x04, 0x07];
    const TAIL: [u8; 6] = [0x08, 0x49, 0x40, 0x0B, 0x40, 0x18];
    (0..data.len().saturating_sub(12)).find(|&i| {
        data[i..i + 5] == HEAD && (data[i + 5] == 0x4A || data[i + 5] == 0x4B) && data[i + 6..i + 12] == TAIL
    })
}

fn is_sound_mode(w: u32) -> bool {
    w >> 24 == 0
        && (8..=11).contains(&((w >> 20) & 15))
        && (1..=12).contains(&((w >> 16) & 15))
        && (1..=12).contains(&((w >> 8) & 15))
        && (w >> 12) & 15 > 0
}

/// Read every song of a ROM's song table, with the instruments they use.
/// Songs whose data can't be read are left out (`None`), with the reason
/// in the second value.
pub fn extract(data: &[u8]) -> Result<(SoundBank, Vec<(SongId, RomError)>)> {
    if data.len() < 0xC0 {
        return Err(RomError::TooSmall);
    }
    let rom = Rom(data);
    let at = find_song_num_start(data).ok_or(RomError::NoDriver)?;
    let at_addr = ROM_BASE + at as u32;
    let song_table = rom.ptr(at_addr + 40)?;
    let player_table = rom.ptr(at_addr + 36).or_else(|_| rom.u32(at_addr + 36))?;
    // The boot sound mode is a literal just before m4aSongNumStart.
    let mode = (at.saturating_sub(64)..at)
        .step_by(4)
        .map(|o| rom.u32(ROM_BASE + o as u32).unwrap_or(0))
        .find(|&w| is_sound_mode(w))
        .ok_or(RomError::NoSoundMode)?;
    let mixer = MixerConfig {
        mix_rate: MIX_RATES[((mode >> 16) & 15) as usize - 1],
        ds_channels: ((mode >> 8) & 15) as u8,
        master_volume: ((mode >> 12) & 15) as u8,
        reverb: if mode & 0x80 != 0 { (mode & 0x7F) as u8 } else { 0 },
        dac_resolution: ((mode >> 20) & 15) as u8 - 8,
    };

    let mut x = Extractor {
        rom,
        groups: HashMap::new(),
        bank_groups: Vec::new(),
        key_maps: HashMap::new(),
        key_map_list: Vec::new(),
        samples: HashMap::new(),
        sample_list: Vec::new(),
        waves: HashMap::new(),
        wave_list: Vec::new(),
    };
    let mut songs = Vec::new();
    let mut failures = Vec::new();
    let mut player_count = 0usize;
    for index in 0..MAX_SONGS {
        let entry = song_table + 8 * index as u32;
        let Ok(header) = x.rom.ptr(entry) else { break };
        let (Ok(tracks), Ok(voicegroup)) = (x.rom.u8(header), x.rom.u32(header + 4)) else { break };
        // The table ends where entries stop looking like songs.
        if tracks > 16 || (tracks > 0 && x.rom.offset(voicegroup).is_err()) {
            break;
        }
        let player = x.rom.u16(entry + 4)?;
        if tracks == 0 {
            songs.push(None);
            continue;
        }
        match x.song(header, player) {
            Ok(s) => {
                player_count = player_count.max(player as usize + 1);
                songs.push(Some(s));
            }
            Err(e) => {
                failures.push((SongId(index as u16), e));
                songs.push(None);
            }
        }
    }
    // The player table: (info, tracks, max tracks, uses priority) per player.
    let mut order: Vec<(u32, usize)> =
        (0..player_count).map(|p| (x.rom.u32(player_table + 12 * p as u32 + 4).unwrap_or(0), p)).collect();
    order.sort();
    let mut rank = vec![0u8; player_count];
    for (r, &(_, p)) in order.iter().enumerate() {
        rank[p] = r as u8;
    }
    let players = (0..player_count)
        .map(|p| {
            let e = player_table + 12 * p as u32;
            Ok(PlayerConfig {
                max_tracks: x.rom.u8(e + 8)?,
                uses_priority: x.rom.u16(e + 10)? != 0,
                track_order: rank[p],
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let bank = SoundBank {
        mixer,
        players,
        songs,
        voicegroups: x.bank_groups,
        key_maps: x.key_map_list,
        samples: x.sample_list,
        waves: x.wave_list,
    };
    debug_assert!(bank.validate().is_ok());
    Ok((bank, failures))
}

struct Extractor<'a> {
    rom: Rom<'a>,
    /// Voicegroups by (address, entries, inside a kit or split).
    groups: HashMap<(u32, usize, bool), VoicegroupId>,
    bank_groups: Vec<Voicegroup>,
    key_maps: HashMap<u32, KeyMapId>,
    key_map_list: Vec<KeyMap>,
    samples: HashMap<u32, Option<SampleId>>,
    sample_list: Vec<Sample>,
    waves: HashMap<u32, Option<WaveId>>,
    wave_list: Vec<Wave>,
}

impl Extractor<'_> {
    fn song(&mut self, header: u32, player: u16) -> Result<Song> {
        let tracks = self.rom.u8(header)?;
        let priority = self.rom.u8(header + 2)?;
        let reverb = self.rom.u8(header + 3)?;
        let voicegroup = self.rom.ptr(header + 4)?;
        let tracks = (0..tracks as u32)
            .map(|t| decode_track(&self.rom, self.rom.ptr(header + 8 + 4 * t)?))
            .collect::<Result<Vec<_>>>()?;
        Ok(Song {
            player: PlayerId(player as u8),
            priority,
            reverb: (reverb & 0x80 != 0).then_some(reverb & 0x7F),
            voicegroup: self.voicegroup(voicegroup, 128, 0)?,
            tracks,
        })
    }

    /// The `n` voices at `addr` (a voicegroup or a drum kit: 128; a key
    /// split's group: as many as its key map needs). A kit's or a split's
    /// own kits and splits play nothing (the driver refuses them), so they
    /// are read as silent; a group read both ways is kept twice.
    fn voicegroup(&mut self, addr: u32, n: usize, depth: u8) -> Result<VoicegroupId> {
        let key = (addr, n, depth > 0);
        if let Some(&id) = self.groups.get(&key) {
            return Ok(id);
        }
        let id = VoicegroupId(self.bank_groups.len() as u16);
        self.groups.insert(key, id);
        self.bank_groups.push(Voicegroup::default());
        let voices = (0..n as u32).map(|i| self.voice(addr + 12 * i, depth)).collect::<Vec<_>>();
        self.bank_groups[id.0 as usize].voices = voices;
        Ok(id)
    }

    /// One 12-byte ToneData: type, key, length, pan/sweep, a pointer or
    /// PSG setting, attack, decay, sustain, release.
    fn voice(&mut self, at: u32, depth: u8) -> Voice {
        let silent = Voice { kind: VoiceKind::Silent, key: 60, pan: None, length: 0, envelope: Envelope::default() };
        let Ok(b) = self.rom.bytes(at, 12) else { return silent };
        let b: [u8; 12] = b.try_into().unwrap();
        let t = b[0];
        let param = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
        let envelope = Envelope { attack: b[8], decay: b[9], sustain: b[10], release: b[11] };
        let pan = (b[3] & 0x80 != 0).then(|| ((b[3] as i32 - 0xC0) * 2) as i8);
        let kind = if t & 0xC0 != 0 {
            // Kits and splits do not nest (the driver refuses a nested one).
            if depth > 0 {
                VoiceKind::Silent
            } else if t & 0x40 != 0 {
                let map_addr = u32::from_le_bytes([b[8], b[9], b[10], b[11]]);
                match self.key_map(map_addr) {
                    Some((map, n)) => match self.voicegroup(param, n, depth + 1) {
                        Ok(group) if self.rom.offset(param).is_ok() => VoiceKind::Split { group, map },
                        _ => VoiceKind::Silent,
                    },
                    None => VoiceKind::Silent,
                }
            } else if self.rom.offset(param).is_ok() {
                match self.voicegroup(param, 128, depth + 1) {
                    Ok(kit) => VoiceKind::Drums { kit },
                    Err(_) => VoiceKind::Silent,
                }
            } else {
                VoiceKind::Silent
            }
        } else {
            let fixed = t & 8 != 0;
            match t & 7 {
                // ply_note: the PSG's sweep is the voice's pan/sweep byte
                // unless that is a pan or has no sweep time.
                1 => VoiceKind::Square1 {
                    duty: (param & 3) as u8,
                    sweep: if b[3] & 0x80 == 0 && b[3] & 0x70 != 0 { b[3] } else { NO_SWEEP },
                    fixed,
                },
                2 => VoiceKind::Square2 { duty: (param & 3) as u8, fixed },
                3 => match self.wave(param) {
                    Some(wave) => VoiceKind::Wave { wave, fixed },
                    None => VoiceKind::Silent,
                },
                4 => VoiceKind::Noise { narrow: param != 0 },
                _ => match self.sample(param) {
                    Some(sample) => VoiceKind::DirectSound { sample, fixed },
                    None => VoiceKind::Silent,
                },
            }
        };
        let envelope = if matches!(kind, VoiceKind::Split { .. }) { Envelope::default() } else { envelope };
        let length = if kind.psg_channel().is_some() { b[2] } else { 0 };
        Voice { kind, key: b[1], pan, length, envelope }
    }

    fn key_map(&mut self, addr: u32) -> Option<(KeyMapId, usize)> {
        let map: [u8; 128] = self.rom.bytes(addr, 128).ok()?.try_into().unwrap();
        let n = *map.iter().max().unwrap() as usize + 1;
        let id = *self.key_maps.entry(addr).or_insert_with(|| {
            self.key_map_list.push(KeyMap(map));
            KeyMapId(self.key_map_list.len() as u16 - 1)
        });
        Some((id, n))
    }

    /// A sample: u16 0, u16 flags (0x4000: loops), u32 rate (Hz * 1024),
    /// u32 loop start, u32 length, then the signed 8-bit data.
    fn sample(&mut self, addr: u32) -> Option<SampleId> {
        if let Some(&s) = self.samples.get(&addr) {
            return s;
        }
        let read = || -> Result<Sample> {
            self.rom.bytes(addr, 16)?;
            let flags = self.rom.u16(addr + 2)?;
            let rate = self.rom.u32(addr + 4)?;
            let loop_start = self.rom.u32(addr + 8)?;
            let len = self.rom.u32(addr + 12)? as usize;
            if len == 0 || len > 0x40_0000 || (flags & 0x4000 != 0 && loop_start as usize >= len) {
                return Err(RomError::OutOfRange(addr));
            }
            let data: Vec<i8> = self.rom.bytes(addr + 16, len)?.iter().map(|&x| x as i8).collect();
            let loop_start = (flags & 0x4000 != 0).then_some(loop_start);
            // The byte after the data, which the mixer reads.
            let tail = self.rom.u8(addr + 16 + len as u32).map_or_else(|_| Sample::usual_tail(&data, loop_start), |b| b as i8);
            Ok(Sample { rate, loop_start, data, tail })
        };
        let id = read().ok().map(|s| {
            self.sample_list.push(s);
            SampleId(self.sample_list.len() as u16 - 1)
        });
        self.samples.insert(addr, id);
        id
    }

    /// A wave: 16 bytes of two 4-bit steps each, high nibble first.
    fn wave(&mut self, addr: u32) -> Option<WaveId> {
        if let Some(&w) = self.waves.get(&addr) {
            return w;
        }
        let id = self.rom.bytes(addr, 16).ok().map(|b| {
            let mut w = [0u8; 32];
            for (i, &x) in b.iter().enumerate() {
                w[2 * i] = x >> 4;
                w[2 * i + 1] = x & 15;
            }
            self.wave_list.push(Wave(w));
            WaveId(self.wave_list.len() as u16 - 1)
        });
        self.waves.insert(addr, id);
        id
    }
}

/// Decode a track's commands, starting at `start`. The byte stream is
/// read in memory order from the start and from every jump target, with
/// running status carried along memory order; where flows merge (jump
/// targets, returns from patterns) a command byte must start, so that
/// order and every path agree.
fn decode_track(rom: &Rom, start: u32) -> Result<Track> {
    let mut commands: Vec<Command> = Vec::new();
    let mut index: BTreeMap<u32, u32> = BTreeMap::new();
    // Jumps to patch: (command index, target address).
    let mut jumps: Vec<(usize, u32)> = Vec::new();
    let mut segments: VecDeque<u32> = VecDeque::from([start]);
    // Where flows merge: a command byte must start there.
    let mut merges: Vec<u32> = vec![start];
    while let Some(seg) = segments.pop_front() {
        if index.contains_key(&seg) {
            continue;
        }
        let mut at = seg;
        let mut running: Option<u8> = None;
        loop {
            if let Some(&i) = index.get(&at) {
                // Falls into code already decoded.
                merges.push(at);
                commands.push(Command::Goto(i));
                break;
            }
            index.insert(at, commands.len() as u32);
            let mut c = rom.u8(at)?;
            let here = at;
            if c < 0x80 {
                c = running.ok_or(RomError::RunningStatus(here))?;
            } else {
                at += 1;
                if c >= 0xBD {
                    running = Some(c);
                }
            }
            let arg = |at: &mut u32| -> Result<u8> {
                let v = rom.u8(*at)?;
                *at += 1;
                Ok(v)
            };
            // An optional data byte (< 0x80) that follows.
            let data = |at: &mut u32| -> Result<Option<u8>> {
                let v = rom.u8(*at)?;
                Ok((v < 0x80).then(|| {
                    *at += 1;
                    v
                }))
            };
            let cmd = match c {
                0x80..=0xB0 => Command::Wait(CLOCK[(c - 0x80) as usize]),
                0xB1 => Command::Fine,
                0xB2 | 0xB3 => {
                    let target = rom.ptr(at)?;
                    at += 4;
                    jumps.push((commands.len(), target));
                    segments.push_back(target);
                    merges.push(target);
                    if c == 0xB2 { Command::Goto(0) } else { Command::Call(0) }
                }
                0xB4 => Command::Return,
                // Unused command bytes: the driver's table sends them to
                // ply_fine.
                0xB6..=0xB8 | 0xC6 | 0xC7 | 0xC9..=0xCB => Command::Fine,
                0xB9 => {
                    let (kind, address, operand) = (arg(&mut at)?, arg(&mut at)?, arg(&mut at)?);
                    let op = match kind {
                        0 => MemOp::Set,
                        1 => MemOp::Add,
                        2 => MemOp::Sub,
                        3 => MemOp::SetFromMemory,
                        4 => MemOp::AddFromMemory,
                        5 => MemOp::SubFromMemory,
                        6..=17 => {
                            let test = [
                                MemTest::Equal,
                                MemTest::NotEqual,
                                MemTest::Greater,
                                MemTest::GreaterOrEqual,
                                MemTest::LessOrEqual,
                                MemTest::Less,
                            ][(kind as usize - 6) % 6];
                            let target = rom.ptr(at)?;
                            at += 4;
                            jumps.push((commands.len(), target));
                            segments.push_back(target);
                            merges.push(target);
                            MemOp::JumpIf { test, with_memory: kind >= 12, target: 0 }
                        }
                        // Past the driver's 18 operations it does nothing.
                        _ => continue,
                    };
                    Command::MemAcc { op, address, operand }
                }
                0xB5 => {
                    let count = arg(&mut at)?;
                    let target = rom.ptr(at)?;
                    at += 4;
                    jumps.push((commands.len(), target));
                    segments.push_back(target);
                    merges.push(target);
                    Command::Repeat { count, target: 0 }
                }
                0xBA => Command::Priority(arg(&mut at)?),
                0xBB => Command::Tempo(arg(&mut at)?),
                0xBC => Command::KeyShift(arg(&mut at)? as i8),
                0xBD => Command::Voice(arg(&mut at)?),
                0xBE => Command::Volume(arg(&mut at)?),
                0xBF => Command::Pan(arg(&mut at)?),
                0xC0 => Command::Bend(arg(&mut at)?),
                0xC1 => Command::BendRange(arg(&mut at)?),
                0xC2 => Command::LfoSpeed(arg(&mut at)?),
                0xC3 => Command::LfoDelay(arg(&mut at)?),
                0xC4 => Command::Modulation(arg(&mut at)?),
                0xC5 => Command::ModulationType(arg(&mut at)?),
                0xC8 => Command::Tune(arg(&mut at)?),
                0xCD => match arg(&mut at)? {
                    // xxx: the driver's table sends these to ply_fine.
                    0x00 | 0x03 => Command::Fine,
                    0x08 => Command::EchoVolume(arg(&mut at)?),
                    0x09 => Command::EchoLength(arg(&mut at)?),
                    _ => return Err(RomError::Command { at: here, byte: c }),
                },
                0xCE => Command::EndTie { key: data(&mut at)? },
                0xCF..=0xFF => {
                    let mut gate = CLOCK[(c - 0xCF) as usize];
                    let key = data(&mut at)?;
                    let velocity = if key.is_some() { data(&mut at)? } else { None };
                    if velocity.is_some()
                        && let Some(extra) = data(&mut at)?
                    {
                        gate = gate.wrapping_add(extra);
                    }
                    Command::Note { gate, key, velocity }
                }
                _ => return Err(RomError::Command { at: here, byte: c }),
            };
            commands.push(cmd);
            match cmd {
                // (PEND outside a pattern call does nothing: the flow goes on.)
                Command::Fine | Command::Goto(_) => break,
                // Flows merge after a pattern returns.
                Command::Call(_) => merges.push(at),
                _ => {}
            }
        }
    }
    for &m in &merges {
        if rom.u8(m)? < 0x80 {
            return Err(RomError::RunningStatus(m));
        }
    }
    for (i, target) in jumps {
        let t = index[&target];
        match &mut commands[i] {
            Command::Goto(x)
            | Command::Call(x)
            | Command::Repeat { target: x, .. }
            | Command::MemAcc { op: MemOp::JumpIf { target: x, .. }, .. } => *x = t,
            _ => unreachable!(),
        }
    }
    Ok(Track { commands })
}
