//! What the driver plays: the mixer settings, the music players, songs as
//! typed command lists, and instruments (voicegroups, samples, PSG waves).
//!
//! A bank is read out of a ROM image once ([`crate::rom::extract`]) and
//! saved in a small binary format ([`SoundBank::to_bytes`],
//! [`SoundBank::from_bytes`]); nothing here refers to ROM addresses.

use std::fmt;

/// A song-table entry: music and sound effects share one table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SongId(pub u16);

/// A music player (the driver's `MusicPlayerInfo`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(pub u8);

/// An index into [`SoundBank::voicegroups`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VoicegroupId(pub u16);

/// An index into [`SoundBank::samples`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SampleId(pub u16);

/// An index into [`SoundBank::waves`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WaveId(pub u16);

/// An index into [`SoundBank::key_maps`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyMapId(pub u16);

/// The sound mode the game sets up at boot (`m4aSoundInit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MixerConfig {
    /// Direct Sound mixing rate, Hz.
    pub mix_rate: u32,
    /// Direct Sound channels (the PSG always has four).
    pub ds_channels: u8,
    /// Master volume, 0..=15.
    pub master_volume: u8,
    /// Reverb level before any song sets one (0 = off).
    pub reverb: u8,
}

/// A music player's fixed configuration (the driver's player table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerConfig {
    /// Tracks it can play; a song's extra tracks are dropped.
    pub max_tracks: u8,
    /// A new song must have at least the playing song's priority to
    /// replace it. Without this, any song replaces the playing one.
    pub uses_priority: bool,
    /// Where this player's tracks sort among all players' tracks. Two
    /// channels of equal priority compete by their tracks' places in
    /// memory; this is the rank of the player's track array there.
    pub track_order: u8,
}

/// A song: which player plays it, how it competes, its tracks and voices.
#[derive(Clone, Debug, PartialEq)]
pub struct Song {
    pub player: PlayerId,
    /// Song priority: decides whether it may replace a playing song on its
    /// player, and (plus a track's own) which notes keep hardware channels.
    pub priority: u8,
    /// The reverb level it switches the whole mixer to, if any.
    pub reverb: Option<u8>,
    pub voicegroup: VoicegroupId,
    pub tracks: Vec<Track>,
}

/// One track's commands, jumps resolved to command indices.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Track {
    pub commands: Vec<Command>,
}

/// One command of a track, decoded from the M4A byte stream. Running
/// status is resolved; optional arguments the data leaves out stay `None`
/// (the track keeps its previous value).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// W00..W96.
    Wait(u8),
    /// FINE: the track ends.
    Fine,
    /// GOTO.
    Goto(u32),
    /// PATT: call a pattern.
    Call(u32),
    /// PEND: return from a pattern.
    Return,
    /// REPT: jump back `count` times.
    Repeat { count: u8, target: u32 },
    /// PRIO: the track's priority, added to the song's.
    Priority(u8),
    /// TEMPO, in half BPM.
    Tempo(u8),
    /// KEYSH, semitones.
    KeyShift(i8),
    /// VOICE: an instrument of the song's voicegroup.
    Voice(u8),
    /// VOL, 0..=127.
    Volume(u8),
    /// PAN, 64 = center.
    Pan(u8),
    /// BEND, 64 = none.
    Bend(u8),
    /// BENDR: bend range in semitones.
    BendRange(u8),
    /// LFOS.
    LfoSpeed(u8),
    /// LFODL: ticks before the LFO starts after a note.
    LfoDelay(u8),
    /// MOD: LFO depth.
    Modulation(u8),
    /// MODT: what the LFO moves (0 pitch, 1 volume, 2 pan).
    ModulationType(u8),
    /// TUNE, 64 = none.
    Tune(u8),
    /// XCMD 0x08: pseudo-echo volume.
    EchoVolume(u8),
    /// XCMD 0x09: pseudo-echo length in frames.
    EchoLength(u8),
    /// EOT: release a tied note (of `key`, else the track's last key).
    EndTie { key: Option<u8> },
    /// TIE (gate 0: held until EOT) or N01..N96 (gate in ticks).
    Note { gate: u8, key: Option<u8>, velocity: Option<u8> },
}

/// A voicegroup, a drum kit or a key split's instruments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Voicegroup {
    pub voices: Vec<Voice>,
}

/// One instrument (the driver's `ToneData`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Voice {
    pub kind: VoiceKind,
    /// The key a drum kit's instrument plays at.
    pub key: u8,
    /// A drum kit instrument's own pan (-128..=126).
    pub pan: Option<i8>,
    pub envelope: Envelope,
}

/// Attack, decay, sustain and release, as the driver takes them: Direct
/// Sound steps 0..=255 per frame, the PSG counts frames per level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Envelope {
    pub attack: u8,
    pub decay: u8,
    pub sustain: u8,
    pub release: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceKind {
    /// A sample; `fixed` plays it at the mixing rate whatever the key.
    DirectSound {
        sample: SampleId,
        fixed: bool,
    },
    /// PSG square 1 (duty 0..=3: 12.5%, 25%, 50%, 75%).
    Square1 {
        duty: u8,
    },
    Square2 {
        duty: u8,
    },
    /// PSG wave channel.
    Wave {
        wave: WaveId,
    },
    /// PSG noise; `narrow` is the 7-bit LFSR.
    Noise {
        narrow: bool,
    },
    /// A drum kit: each key plays its own instrument.
    Drums {
        kit: VoicegroupId,
    },
    /// A key split: a key map picks the instrument.
    Split {
        group: VoicegroupId,
        map: KeyMapId,
    },
    /// A voicegroup entry with nothing behind it (past the end of what a
    /// game defines); it plays nothing.
    Silent,
}

impl VoiceKind {
    /// The PSG channel (1..=4) it plays on, or `None` for Direct Sound.
    pub fn psg_channel(self) -> Option<usize> {
        match self {
            VoiceKind::Square1 { .. } => Some(1),
            VoiceKind::Square2 { .. } => Some(2),
            VoiceKind::Wave { .. } => Some(3),
            VoiceKind::Noise { .. } => Some(4),
            _ => None,
        }
    }
}

/// A Direct Sound sample: signed 8-bit PCM.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sample {
    /// Playback rate at key 60 (C4), in 1/1024 Hz.
    pub rate: u32,
    pub loop_start: Option<u32>,
    pub data: Vec<i8>,
}

/// A PSG wave: 32 4-bit steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wave(pub [u8; 32]);

/// A key split's instrument per key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyMap(pub [u8; 128]);

/// Everything the driver needs to play a game's sound.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundBank {
    pub mixer: MixerConfig,
    pub players: Vec<PlayerConfig>,
    /// By song id; `None` for entries without a playable song.
    pub songs: Vec<Option<Song>>,
    pub voicegroups: Vec<Voicegroup>,
    pub key_maps: Vec<KeyMap>,
    pub samples: Vec<Sample>,
    pub waves: Vec<Wave>,
}

impl SoundBank {
    pub fn song(&self, id: SongId) -> Option<&Song> {
        self.songs.get(id.0 as usize)?.as_ref()
    }
}

// ---- The bank file -------------------------------------------------------------

const MAGIC: &[u8; 8] = b"M4ABANK\0";
const VERSION: u32 = 1;

/// Reading a bank file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankError {
    NotABank,
    Version(u32),
    Truncated,
    Invalid(&'static str),
}

impl fmt::Display for BankError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BankError::NotABank => write!(f, "not a sound bank"),
            BankError::Version(v) => write!(f, "sound bank version {v} (this build reads version {VERSION})"),
            BankError::Truncated => write!(f, "sound bank is truncated"),
            BankError::Invalid(what) => write!(f, "sound bank has an invalid {what}"),
        }
    }
}

impl std::error::Error for BankError {}

#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn opt(&mut self, v: Option<u8>) {
        match v {
            Some(x) => {
                self.u8(1);
                self.u8(x);
            }
            None => self.u8(0),
        }
    }
    fn len(&mut self, n: usize) {
        self.u32(n as u32);
    }
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8], BankError> {
        let b = self.data.get(self.at..self.at + n).ok_or(BankError::Truncated)?;
        self.at += n;
        Ok(b)
    }
    fn u8(&mut self) -> Result<u8, BankError> {
        Ok(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, BankError> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, BankError> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn bool(&mut self) -> Result<bool, BankError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(BankError::Invalid("flag")),
        }
    }
    fn opt(&mut self) -> Result<Option<u8>, BankError> {
        Ok(if self.bool()? { Some(self.u8()?) } else { None })
    }
    /// A count, checked against what is left (each item takes at least
    /// `min` bytes), so a corrupt length can't allocate wildly.
    fn len(&mut self, min: usize) -> Result<usize, BankError> {
        let n = self.u32()? as usize;
        if n.saturating_mul(min) > self.data.len() - self.at {
            return Err(BankError::Truncated);
        }
        Ok(n)
    }
}

// Command tags.
const C_WAIT: u8 = 0;
const C_FINE: u8 = 1;
const C_GOTO: u8 = 2;
const C_CALL: u8 = 3;
const C_RETURN: u8 = 4;
const C_REPEAT: u8 = 5;
const C_PRIORITY: u8 = 6;
const C_TEMPO: u8 = 7;
const C_KEY_SHIFT: u8 = 8;
const C_VOICE: u8 = 9;
const C_VOLUME: u8 = 10;
const C_PAN: u8 = 11;
const C_BEND: u8 = 12;
const C_BEND_RANGE: u8 = 13;
const C_LFO_SPEED: u8 = 14;
const C_LFO_DELAY: u8 = 15;
const C_MOD: u8 = 16;
const C_MOD_TYPE: u8 = 17;
const C_TUNE: u8 = 18;
const C_ECHO_VOLUME: u8 = 19;
const C_ECHO_LENGTH: u8 = 20;
const C_END_TIE: u8 = 21;
const C_NOTE: u8 = 22;

// Voice tags.
const V_DS: u8 = 0;
const V_SQUARE1: u8 = 1;
const V_SQUARE2: u8 = 2;
const V_WAVE: u8 = 3;
const V_NOISE: u8 = 4;
const V_DRUMS: u8 = 5;
const V_SPLIT: u8 = 6;
const V_SILENT: u8 = 7;

impl SoundBank {
    /// The bank as a file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.0.extend_from_slice(MAGIC);
        w.u32(VERSION);
        let m = &self.mixer;
        w.u32(m.mix_rate);
        w.u8(m.ds_channels);
        w.u8(m.master_volume);
        w.u8(m.reverb);
        w.len(self.players.len());
        for p in &self.players {
            w.u8(p.max_tracks);
            w.u8(p.uses_priority as u8);
            w.u8(p.track_order);
        }
        w.len(self.songs.len());
        for s in &self.songs {
            let Some(s) = s else {
                w.u8(0);
                continue;
            };
            w.u8(1);
            w.u8(s.player.0);
            w.u8(s.priority);
            w.opt(s.reverb);
            w.u16(s.voicegroup.0);
            w.len(s.tracks.len());
            for t in &s.tracks {
                w.len(t.commands.len());
                for c in &t.commands {
                    write_command(&mut w, c);
                }
            }
        }
        w.len(self.voicegroups.len());
        for g in &self.voicegroups {
            w.len(g.voices.len());
            for v in &g.voices {
                write_voice(&mut w, v);
            }
        }
        w.len(self.key_maps.len());
        for k in &self.key_maps {
            w.0.extend_from_slice(&k.0);
        }
        w.len(self.samples.len());
        for s in &self.samples {
            w.u32(s.rate);
            w.u32(s.loop_start.unwrap_or(u32::MAX));
            w.len(s.data.len());
            w.0.extend(s.data.iter().map(|&x| x as u8));
        }
        w.len(self.waves.len());
        for wv in &self.waves {
            w.0.extend_from_slice(&wv.0);
        }
        w.0
    }

    /// Read a bank file, checking every reference in it.
    pub fn from_bytes(data: &[u8]) -> Result<SoundBank, BankError> {
        let mut r = Reader { data, at: 0 };
        if r.bytes(8).map_err(|_| BankError::NotABank)? != MAGIC {
            return Err(BankError::NotABank);
        }
        let version = r.u32()?;
        if version != VERSION {
            return Err(BankError::Version(version));
        }
        let mixer = MixerConfig { mix_rate: r.u32()?, ds_channels: r.u8()?, master_volume: r.u8()?, reverb: r.u8()? };
        if mixer.mix_rate == 0 || mixer.ds_channels == 0 || mixer.ds_channels > 12 {
            return Err(BankError::Invalid("mixer configuration"));
        }
        let n = r.len(3)?;
        let players = (0..n)
            .map(|_| Ok(PlayerConfig { max_tracks: r.u8()?, uses_priority: r.bool()?, track_order: r.u8()? }))
            .collect::<Result<Vec<_>, BankError>>()?;
        let n = r.len(1)?;
        let mut songs = Vec::with_capacity(n);
        for _ in 0..n {
            if !r.bool()? {
                songs.push(None);
                continue;
            }
            let (player, priority, reverb, voicegroup) = (PlayerId(r.u8()?), r.u8()?, r.opt()?, VoicegroupId(r.u16()?));
            let tracks = (0..r.len(4)?)
                .map(|_| {
                    let commands = (0..r.len(1)?).map(|_| read_command(&mut r)).collect::<Result<Vec<_>, _>>()?;
                    Ok(Track { commands })
                })
                .collect::<Result<Vec<_>, BankError>>()?;
            songs.push(Some(Song { player, priority, reverb, voicegroup, tracks }));
        }
        let n = r.len(4)?;
        let voicegroups = (0..n)
            .map(|_| {
                let voices = (0..r.len(1)?).map(|_| read_voice(&mut r)).collect::<Result<Vec<_>, _>>()?;
                Ok(Voicegroup { voices })
            })
            .collect::<Result<Vec<_>, BankError>>()?;
        let n = r.len(128)?;
        let key_maps =
            (0..n).map(|_| Ok(KeyMap(r.bytes(128)?.try_into().unwrap()))).collect::<Result<Vec<_>, BankError>>()?;
        let n = r.len(12)?;
        let samples = (0..n)
            .map(|_| {
                let rate = r.u32()?;
                let loop_start = match r.u32()? {
                    u32::MAX => None,
                    l => Some(l),
                };
                let len = r.len(1)?;
                let data = r.bytes(len)?.iter().map(|&x| x as i8).collect();
                Ok(Sample { rate, loop_start, data })
            })
            .collect::<Result<Vec<_>, BankError>>()?;
        let n = r.len(32)?;
        let waves =
            (0..n).map(|_| Ok(Wave(r.bytes(32)?.try_into().unwrap()))).collect::<Result<Vec<_>, BankError>>()?;
        if r.at != data.len() {
            return Err(BankError::Invalid("length"));
        }
        let bank = SoundBank { mixer, players, songs, voicegroups, key_maps, samples, waves };
        bank.validate()?;
        Ok(bank)
    }

    /// Every index in range, so the driver can trust the bank.
    pub fn validate(&self) -> Result<(), BankError> {
        let group_ok = |g: VoicegroupId| (g.0 as usize) < self.voicegroups.len();
        for g in &self.voicegroups {
            for v in &g.voices {
                let ok = match v.kind {
                    VoiceKind::DirectSound { sample, .. } => self.samples.get(sample.0 as usize).is_some_and(|s| {
                        !s.data.is_empty() && s.loop_start.is_none_or(|l| (l as usize) < s.data.len())
                    }),
                    VoiceKind::Wave { wave } => (wave.0 as usize) < self.waves.len(),
                    VoiceKind::Drums { kit } => group_ok(kit),
                    VoiceKind::Split { group, map } => {
                        group_ok(group)
                            && self.key_maps.get(map.0 as usize).is_some_and(|m| {
                                m.0.iter().all(|&i| (i as usize) < self.voicegroups[group.0 as usize].voices.len())
                            })
                    }
                    _ => true,
                };
                if !ok {
                    return Err(BankError::Invalid("voice"));
                }
            }
        }
        for s in self.songs.iter().flatten() {
            if !group_ok(s.voicegroup) || (s.player.0 as usize) >= self.players.len() {
                return Err(BankError::Invalid("song"));
            }
            for t in &s.tracks {
                let n = t.commands.len() as u32;
                let ok = t.commands.iter().all(|c| match *c {
                    Command::Goto(i) | Command::Call(i) | Command::Repeat { target: i, .. } => i < n,
                    _ => true,
                });
                if !ok {
                    return Err(BankError::Invalid("jump"));
                }
            }
        }
        Ok(())
    }
}

fn write_command(w: &mut Writer, c: &Command) {
    let simple = |w: &mut Writer, tag: u8, v: u8| {
        w.u8(tag);
        w.u8(v);
    };
    match *c {
        Command::Wait(n) => simple(w, C_WAIT, n),
        Command::Fine => w.u8(C_FINE),
        Command::Goto(i) => {
            w.u8(C_GOTO);
            w.u32(i);
        }
        Command::Call(i) => {
            w.u8(C_CALL);
            w.u32(i);
        }
        Command::Return => w.u8(C_RETURN),
        Command::Repeat { count, target } => {
            w.u8(C_REPEAT);
            w.u8(count);
            w.u32(target);
        }
        Command::Priority(v) => simple(w, C_PRIORITY, v),
        Command::Tempo(v) => simple(w, C_TEMPO, v),
        Command::KeyShift(v) => simple(w, C_KEY_SHIFT, v as u8),
        Command::Voice(v) => simple(w, C_VOICE, v),
        Command::Volume(v) => simple(w, C_VOLUME, v),
        Command::Pan(v) => simple(w, C_PAN, v),
        Command::Bend(v) => simple(w, C_BEND, v),
        Command::BendRange(v) => simple(w, C_BEND_RANGE, v),
        Command::LfoSpeed(v) => simple(w, C_LFO_SPEED, v),
        Command::LfoDelay(v) => simple(w, C_LFO_DELAY, v),
        Command::Modulation(v) => simple(w, C_MOD, v),
        Command::ModulationType(v) => simple(w, C_MOD_TYPE, v),
        Command::Tune(v) => simple(w, C_TUNE, v),
        Command::EchoVolume(v) => simple(w, C_ECHO_VOLUME, v),
        Command::EchoLength(v) => simple(w, C_ECHO_LENGTH, v),
        Command::EndTie { key } => {
            w.u8(C_END_TIE);
            w.opt(key);
        }
        Command::Note { gate, key, velocity } => {
            w.u8(C_NOTE);
            w.u8(gate);
            w.opt(key);
            w.opt(velocity);
        }
    }
}

fn read_command(r: &mut Reader) -> Result<Command, BankError> {
    Ok(match r.u8()? {
        C_WAIT => Command::Wait(r.u8()?),
        C_FINE => Command::Fine,
        C_GOTO => Command::Goto(r.u32()?),
        C_CALL => Command::Call(r.u32()?),
        C_RETURN => Command::Return,
        C_REPEAT => Command::Repeat { count: r.u8()?, target: r.u32()? },
        C_PRIORITY => Command::Priority(r.u8()?),
        C_TEMPO => Command::Tempo(r.u8()?),
        C_KEY_SHIFT => Command::KeyShift(r.u8()? as i8),
        C_VOICE => Command::Voice(r.u8()?),
        C_VOLUME => Command::Volume(r.u8()?),
        C_PAN => Command::Pan(r.u8()?),
        C_BEND => Command::Bend(r.u8()?),
        C_BEND_RANGE => Command::BendRange(r.u8()?),
        C_LFO_SPEED => Command::LfoSpeed(r.u8()?),
        C_LFO_DELAY => Command::LfoDelay(r.u8()?),
        C_MOD => Command::Modulation(r.u8()?),
        C_MOD_TYPE => Command::ModulationType(r.u8()?),
        C_TUNE => Command::Tune(r.u8()?),
        C_ECHO_VOLUME => Command::EchoVolume(r.u8()?),
        C_ECHO_LENGTH => Command::EchoLength(r.u8()?),
        C_END_TIE => Command::EndTie { key: r.opt()? },
        C_NOTE => Command::Note { gate: r.u8()?, key: r.opt()?, velocity: r.opt()? },
        _ => return Err(BankError::Invalid("command")),
    })
}

fn write_voice(w: &mut Writer, v: &Voice) {
    match v.kind {
        VoiceKind::DirectSound { sample, fixed } => {
            w.u8(V_DS);
            w.u16(sample.0);
            w.u8(fixed as u8);
        }
        VoiceKind::Square1 { duty } => {
            w.u8(V_SQUARE1);
            w.u8(duty);
        }
        VoiceKind::Square2 { duty } => {
            w.u8(V_SQUARE2);
            w.u8(duty);
        }
        VoiceKind::Wave { wave } => {
            w.u8(V_WAVE);
            w.u16(wave.0);
        }
        VoiceKind::Noise { narrow } => {
            w.u8(V_NOISE);
            w.u8(narrow as u8);
        }
        VoiceKind::Drums { kit } => {
            w.u8(V_DRUMS);
            w.u16(kit.0);
        }
        VoiceKind::Split { group, map } => {
            w.u8(V_SPLIT);
            w.u16(group.0);
            w.u16(map.0);
        }
        VoiceKind::Silent => w.u8(V_SILENT),
    }
    w.u8(v.key);
    w.opt(v.pan.map(|p| p as u8));
    let e = v.envelope;
    for x in [e.attack, e.decay, e.sustain, e.release] {
        w.u8(x);
    }
}

fn read_voice(r: &mut Reader) -> Result<Voice, BankError> {
    let kind = match r.u8()? {
        V_DS => VoiceKind::DirectSound { sample: SampleId(r.u16()?), fixed: r.bool()? },
        V_SQUARE1 => VoiceKind::Square1 { duty: r.u8()? & 3 },
        V_SQUARE2 => VoiceKind::Square2 { duty: r.u8()? & 3 },
        V_WAVE => VoiceKind::Wave { wave: WaveId(r.u16()?) },
        V_NOISE => VoiceKind::Noise { narrow: r.bool()? },
        V_DRUMS => VoiceKind::Drums { kit: VoicegroupId(r.u16()?) },
        V_SPLIT => VoiceKind::Split { group: VoicegroupId(r.u16()?), map: KeyMapId(r.u16()?) },
        V_SILENT => VoiceKind::Silent,
        _ => return Err(BankError::Invalid("voice kind")),
    };
    let key = r.u8()?;
    let pan = r.opt()?.map(|p| p as i8);
    let envelope = Envelope { attack: r.u8()?, decay: r.u8()?, sustain: r.u8()?, release: r.u8()? };
    Ok(Voice { kind, key, pan, envelope })
}
