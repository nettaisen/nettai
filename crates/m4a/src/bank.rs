//! What the driver plays: the mixer settings, the music players, songs as
//! typed command lists, and instruments (voicegroups, samples, PSG waves).
//!
//! A bank is read out of a ROM image ([`crate::rom::extract`]) or built
//! from a content pack's sound files (bn6-content); nothing here refers to
//! ROM addresses. [`SoundBank::validate`] checks every reference in it.

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

// ---- Validation --------------------------------------------------------------

/// A bank with a reference out of range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankError {
    Invalid(&'static str),
}

impl fmt::Display for BankError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BankError::Invalid(what) => write!(f, "sound bank has an invalid {what}"),
        }
    }
}

impl std::error::Error for BankError {}

impl SoundBank {
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

