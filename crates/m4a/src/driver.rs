//! The M4A driver, one frame at a time: music players, their tracks'
//! sequencers, and the hardware channels they share.
//!
//! Follows the GBA driver routine by routine, in its integer arithmetic:
//! `MPlayStart` decides whether a song may take its player; each frame
//! (`SoundMain`), `MPlayMain` runs each player's tracks at their tempo,
//! `ply_note` finds a channel for each note (stealing from a
//! lower-priority note of any player), `TrkVolPitSet` and the channel pass
//! carry volume and pitch changes to the notes, `CgbSound` runs the PSG
//! envelopes and writes the PSG's registers, and `SoundMainRAM` runs the
//! Direct Sound envelopes and mixes them into the PCM ring buffer. The
//! player controls (tempo, pitch, volume, fades, stops) are the driver's
//! `m4aMPlay*` calls. [`crate::apu`] then plays the PSG registers and the
//! PCM buffer as the GBA's sound hardware does.

use crate::apu::{Apu, FRAME_CYCLES, Reg};
use crate::bank::*;
use crate::mixer::{self, CgbChannel, DsChannel, Note, PcmRing, Phase, Status, TrackOrder, TrackRef};
use crate::tables;
use std::collections::VecDeque;
use std::sync::Arc;

/// Tempo counter steps per tick (the driver plays a tick each time the
/// per-frame tempo it adds reaches this).
const TICK: u16 = 150;
/// Pattern calls nest this deep.
const MAX_CALLS: usize = 3;
/// Bytes of the driver's memory area a MEMACC can reach.
pub const MEMORY_SIZE: usize = 256;
/// Bytes of it the game sets aside (`gMPlayMemAccArea`); a song reaching
/// past them changes other game memory.
pub const MEMORY_AREA: usize = 16;

/// What a track's commands changed, for `TrkVolPitSet` and the channel
/// pass (the track flags' low nibble).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Changes {
    /// Recompute the track's volume (and pan).
    volume_set: bool,
    /// Pass the volume on to the track's notes.
    volume_changed: bool,
    pitch_set: bool,
    pitch_changed: bool,
}

impl Changes {
    const VOLUME: Changes = Changes { volume_set: true, volume_changed: true, pitch_set: false, pitch_changed: false };
    const PITCH: Changes = Changes { volume_set: false, volume_changed: false, pitch_set: true, pitch_changed: true };
    const ALL: Changes = Changes { volume_set: true, volume_changed: true, pitch_set: true, pitch_changed: true };

    fn add(&mut self, c: Changes) {
        self.volume_set |= c.volume_set;
        self.volume_changed |= c.volume_changed;
        self.pitch_set |= c.pitch_set;
        self.pitch_changed |= c.pitch_changed;
    }
    fn any(&self) -> bool {
        self.volume_set || self.volume_changed || self.pitch_set || self.pitch_changed
    }
}

/// A track's state (the driver's `MusicPlayerTrack`).
#[derive(Clone, Debug)]
struct TrackState {
    /// Playing (the EXIST flag): cleared by FINE.
    exists: bool,
    /// Just started: reset before its first tick.
    starting: bool,
    changes: Changes,
    pc: usize,
    wait: u8,
    calls: Vec<usize>,
    repeats: u8,
    gate: u8,
    key: u8,
    velocity: u8,
    /// The key shift and fine pitch TrkVolPitSet works out.
    key_m: i8,
    pitch_m: u8,
    key_shift: i8,
    /// Player controls: key shift and fine pitch, volume (0..=64), pan.
    key_shift_control: i8,
    pitch_control: u8,
    tune: i8,
    bend: i8,
    bend_range: u8,
    /// Right and left volume TrkVolPitSet works out.
    volume_right: u8,
    volume_left: u8,
    volume: u8,
    volume_control: u8,
    /// PAN - 64.
    pan: i8,
    pan_control: i8,
    /// The LFO's current offset (vibrato, tremolo or pan).
    mod_value: i8,
    modulation: u8,
    mod_type: u8,
    lfo_speed: u8,
    lfo_phase: u8,
    lfo_delay: u8,
    lfo_delay_count: u8,
    priority: u8,
    echo_volume: u8,
    echo_length: u8,
    voice: Voice,
    /// Its channels, newest first.
    channels: Vec<Slot>,
}

impl TrackState {
    fn new() -> TrackState {
        TrackState {
            exists: false,
            starting: false,
            changes: Changes::default(),
            pc: 0,
            wait: 0,
            calls: Vec::new(),
            repeats: 0,
            gate: 0,
            key: 0,
            velocity: 0,
            key_m: 0,
            pitch_m: 0,
            key_shift: 0,
            key_shift_control: 0,
            pitch_control: 0,
            tune: 0,
            bend: 0,
            bend_range: 0,
            volume_right: 0,
            volume_left: 0,
            volume: 0,
            volume_control: 0,
            pan: 0,
            pan_control: 0,
            mod_value: 0,
            modulation: 0,
            mod_type: 0,
            lfo_speed: 0,
            lfo_phase: 0,
            lfo_delay: 0,
            lfo_delay_count: 0,
            priority: 0,
            echo_volume: 0,
            echo_length: 0,
            voice: START_VOICE,
            channels: Vec::new(),
        }
    }

    /// `TrkVolPitSet`: the track's volumes and pitch from its settings.
    fn vol_pit_set(&mut self) {
        if self.changes.volume_set {
            let mut x = (self.volume as u32 * self.volume_control as u32) >> 5;
            if self.mod_type == 1 {
                x = (x * (self.mod_value as i32 + 128) as u32) >> 7;
            }
            let mut y = 2 * self.pan as i32 + self.pan_control as i32;
            if self.mod_type == 2 {
                y += self.mod_value as i32;
            }
            let y = y.clamp(-128, 127);
            self.volume_right = (((y + 128) as u32 * x) >> 8) as u8;
            self.volume_left = (((127 - y) as u32 * x) >> 8) as u8;
        }
        if self.changes.pitch_set {
            let mut x = (self.bend as i32 * self.bend_range as i32 + self.tune as i32) * 4
                + ((self.key_shift as i32) << 8)
                + ((self.key_shift_control as i32) << 8)
                + self.pitch_control as i32;
            if self.mod_type == 0 {
                x += 16 * self.mod_value as i32;
            }
            self.key_m = (x >> 8) as i8;
            self.pitch_m = x as u8;
        }
        self.changes.volume_set = false;
        self.changes.pitch_set = false;
    }

    /// `clear_modM`: the LFO starts over.
    fn clear_modulation(&mut self) {
        self.mod_value = 0;
        self.lfo_phase = 0;
        self.changes.add(if self.mod_type == 0 { Changes::PITCH } else { Changes::VOLUME });
    }
}

/// The voice a track has before its first VOICE command: the driver's
/// cleared tone with type 1 (square 1, all settings zero).
const START_VOICE: Voice = Voice {
    kind: VoiceKind::Square1 { duty: 0, sweep: NO_SWEEP, fixed: false },
    key: 0,
    pan: None,
    length: 0,
    envelope: Envelope { attack: 0, decay: 0, sustain: 0, release: 0 },
};

/// A hardware channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Ds(usize),
    /// PSG channel 1..=4, stored at index 0..=3.
    Psg(usize),
}

/// A music player (the driver's `MusicPlayerInfo`).
#[derive(Clone, Debug)]
pub struct MusicPlayer {
    config: PlayerConfig,
    song: Option<SongId>,
    priority: u8,
    /// Tracks that played on the last tick (the status bits).
    active: u16,
    /// Stopped or finished (the status PAUSE bit).
    paused: bool,
    /// Ticks played.
    clock: u32,
    /// The song's tempo (BPM), the tempo control (1/256) and the per-frame
    /// step they make; the tempo counter.
    tempo: u16,
    tempo_control: u16,
    tempo_step: u16,
    tempo_counter: u16,
    /// A fade's interval in frames (0: none), its countdown, and its level:
    /// 4 times the volume (0..=64), plus [`FADE_KEEPS_TRACKS`] and
    /// [`FADE_IN`] in the low bits.
    fade_interval: u16,
    fade_counter: u16,
    fade_level: u16,
    tracks: Vec<TrackState>,
}

/// A fade's level bit: at its end the player pauses and keeps its tracks
/// (to be continued) instead of ending them.
pub const FADE_KEEPS_TRACKS: u16 = 1;
/// A fade's level bit: it fades in.
pub const FADE_IN: u16 = 2;

impl MusicPlayer {
    fn new(config: PlayerConfig) -> MusicPlayer {
        MusicPlayer {
            config,
            song: None,
            priority: 0,
            active: 0,
            paused: true,
            clock: 0,
            tempo: TICK,
            tempo_control: 0x100,
            tempo_step: TICK,
            tempo_counter: 0,
            fade_interval: 0,
            fade_counter: 0,
            fade_level: 0,
            tracks: (0..config.max_tracks.min(16)).map(|_| TrackState::new()).collect(),
        }
    }

    /// The song it was last given.
    pub fn song(&self) -> Option<SongId> {
        self.song
    }

    /// Still playing its song (not stopped, not finished).
    pub fn is_playing(&self) -> bool {
        self.song.is_some() && !self.paused && (self.active != 0 || self.tracks.first().is_some_and(|t| t.starting))
    }

    /// Ticks played since the song started.
    pub fn clock(&self) -> u32 {
        self.clock
    }

    /// The per-frame tempo step (150 plays one tick a frame).
    pub fn tempo_step(&self) -> u16 {
        self.tempo_step
    }

    /// Whether a new song must yield to the playing one (MPlayStart).
    fn refuses(&self, priority: u8) -> bool {
        if !self.config.uses_priority {
            return false;
        }
        let busy = (self.song.is_some() && self.tracks.first().is_some_and(|t| t.starting))
            || (self.active != 0 && !self.paused);
        busy && self.priority > priority
    }
}

/// A hardware channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hardware {
    /// A Direct Sound (sample) channel.
    DirectSound(usize),
    /// PSG channel 1..=4: square 1, square 2, wave, noise.
    Psg(usize),
}

/// A sounding hardware channel, for inspection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelInfo {
    pub hardware: Hardware,
    /// The player and track whose note it plays (none once the track let it go).
    pub owner: Option<(PlayerId, u8)>,
    pub priority: u8,
    /// The key the note named.
    pub midi_key: u8,
    /// The key it plays (a drum's own key), before the track's shifts.
    pub key: u8,
    /// Released, fading out.
    pub released: bool,
    /// Its status: about to start, the envelope's phase, in its echo.
    pub starting: bool,
    pub phase: Phase,
    pub echo: bool,
    /// Left and right volume.
    pub volume: (u8, u8),
    /// The envelope's level (0..=255 for Direct Sound, 0..=15 for the PSG).
    pub envelope: u8,
    /// The frequency: the sample step's factor for Direct Sound, the
    /// frequency register (NR43 for noise) for the PSG.
    pub frequency: u32,
    /// The gate's ticks left (0: a tie).
    pub gate: u8,
}

/// What every channel holds of its note, in the driver's terms (for
/// checking the driver against the game's memory).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteState {
    pub status: Status,
    pub key: u8,
    pub midi_key: u8,
    pub velocity: u8,
    pub priority: u8,
    pub gate: u8,
    pub rhythm_pan: i8,
    /// Right and left.
    pub volume: (u8, u8),
    pub envelope: Envelope,
    pub envelope_volume: u8,
    /// Echo volume and length.
    pub echo: (u8, u8),
    pub frequency: u32,
}

/// A Direct Sound channel's state (the driver's `SoundChannel`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DsState {
    pub note: NoteState,
    pub sample: SampleId,
    /// The envelope times the volumes, right and left, for the mix.
    pub mix: (u8, u8),
    /// Samples left to the data's end, the sample playing, the fraction
    /// past it (23 bits).
    pub remaining: i32,
    pub position: u32,
    pub fraction: u32,
}

/// A PSG channel's state (the driver's `CgbChannel`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PsgState {
    pub note: NoteState,
    pub envelope_goal: u8,
    pub sustain_goal: u8,
    pub envelope_counter: u8,
    pub pan: u8,
    pub nrx4: u8,
    pub length: u8,
    pub sweep: u8,
}

fn note_state(status: Status, n: &Note) -> NoteState {
    NoteState {
        status,
        key: n.key,
        midi_key: n.midi_key,
        velocity: n.velocity,
        priority: n.priority,
        gate: n.gate,
        rhythm_pan: n.rhythm_pan,
        volume: (n.right_volume, n.left_volume),
        envelope: n.envelope,
        envelope_volume: n.envelope_volume,
        echo: (n.echo_volume, n.echo_length),
        frequency: n.frequency,
    }
}

/// Samples of FIFO latency between the driver's PCM buffer and the DAC.
const FIFO_DELAY: usize = 0;

/// The sound driver: every music player and the hardware they share.
pub struct Driver {
    bank: Arc<SoundBank>,
    players: Vec<MusicPlayer>,
    /// Direct Sound channels (SoundInfo's `chans`).
    ds: Vec<DsChannel>,
    /// PSG channels 1..=4.
    psg: [CgbChannel; 4],
    reverb: u8,
    master_volume: u8,
    /// The mixer's step factor (`divFreq`).
    div_freq: u32,
    /// The PSG envelopes' 15-frame counter (SoundInfo's c15).
    psg_tick: u8,
    /// The frame the DMA is on in the PCM ring (`pcmDmaCounter`).
    dma_counter: u8,
    ring: PcmRing,
    /// The slot mixed last, which the FIFOs play next.
    last_slot: Option<usize>,
    /// The memory area MEMACC works on (the game's `gMPlayMemAccArea` and
    /// what lies past it).
    memory: [u8; MEMORY_SIZE],
    apu: Apu,
    /// Direct Sound bytes on their way to the DAC: (right, left).
    fifo: VecDeque<(i8, i8)>,
    /// CPU cycles into the current Direct Sound sample, and into the
    /// current DAC sample.
    ds_phase: u32,
    dac_phase: u32,
    /// Output at the DAC's rate, not taken yet: (left, right).
    pending: Vec<[i16; 2]>,
    /// Half a 32768 Hz sample waiting for its other half (at 65536 Hz).
    carry: Vec<[i16; 2]>,
    /// The output capacitor's last input and output, left and right.
    dc_in: [f32; 2],
    dc_out: [f32; 2],
}

impl Driver {
    pub fn new(bank: Arc<SoundBank>) -> Driver {
        let players = bank.players.iter().map(|&c| MusicPlayer::new(c)).collect();
        let m = bank.mixer;
        let spf = m.samples_per_frame() as usize;
        let pcm_freq = (597_275 * spf as u32 + 5000) / 10_000;
        let mut apu = Apu::default();
        // MPlayExtender's and SoundInit's setup of the sound registers.
        apu.write(Reg::Nr52, 0x8F);
        apu.write(Reg::Nr50, 0);
        apu.write(Reg::Nr51, 0);
        apu.write(Reg::Nr12, 8);
        apu.write(Reg::Nr22, 8);
        apu.write(Reg::Nr42, 8);
        apu.write(Reg::Nr14, 0x80);
        apu.write(Reg::Nr24, 0x80);
        apu.write(Reg::Nr44, 0x80);
        apu.write(Reg::Nr30, 0);
        apu.write(Reg::Nr50, 0x77);
        apu.set_psg_level(2);
        let ring = PcmRing::new(spf);
        Driver {
            players,
            ds: (0..m.ds_channels).map(|_| DsChannel::new()).collect(),
            psg: [CgbChannel::new(1), CgbChannel::new(2), CgbChannel::new(3), CgbChannel::new(4)],
            reverb: m.reverb,
            master_volume: m.master_volume,
            div_freq: ((16_777_216 / pcm_freq.max(1)) + 1) >> 1,
            psg_tick: 0,
            dma_counter: ring.frames as u8,
            ring,
            last_slot: None,
            memory: [0; MEMORY_SIZE],
            apu,
            fifo: std::iter::repeat_n((0, 0), FIFO_DELAY).collect(),
            ds_phase: 0,
            dac_phase: 0,
            pending: Vec::new(),
            carry: Vec::new(),
            dc_in: [0.0; 2],
            dc_out: [0.0; 2],
            bank,
        }
    }

    pub fn bank(&self) -> &Arc<SoundBank> {
        &self.bank
    }

    pub fn player(&self, p: PlayerId) -> Option<&MusicPlayer> {
        self.players.get(p.0 as usize)
    }

    pub fn players(&self) -> &[MusicPlayer] {
        &self.players
    }

    /// The memory area MEMACC commands work on (bytes 0..16 are the game's
    /// area, [`MEMORY_AREA`]).
    pub fn memory(&self) -> &[u8; MEMORY_SIZE] {
        &self.memory
    }

    /// The game writes a byte of the memory area.
    pub fn set_memory(&mut self, address: u8, value: u8) {
        self.memory[address as usize] = value;
    }

    /// The reverb level in effect (0 = none).
    pub fn reverb(&self) -> u8 {
        self.reverb
    }

    /// `m4aSoundMode`'s reverb: the level the mixer uses from now on
    /// (songs with a reverb set it when they start).
    pub fn set_reverb(&mut self, level: u8) {
        self.reverb = level & 0x7F;
    }

    /// The PSG envelopes' 15-frame counter: when it reaches 0, every
    /// envelope counts one frame more (SoundInfo's c15, 14..=0).
    pub fn psg_tick(&self) -> u8 {
        self.psg_tick
    }

    /// Set that counter, to line the driver up with a recording.
    pub fn set_psg_tick(&mut self, tick: u8) {
        self.psg_tick = tick.min(14);
    }

    /// The PCM ring's DMA frame counter (`pcmDmaCounter`): the next frame
    /// is mixed into the slot it then names.
    pub fn dma_counter(&self) -> u8 {
        self.dma_counter
    }

    /// Set that counter, to line the driver up with a recording.
    pub fn set_dma_counter(&mut self, counter: u8) {
        self.dma_counter = counter.clamp(1, self.ring.frames as u8);
    }

    /// The Direct Sound bytes the last frame mixed: (right, left), the
    /// PCM buffer's slot for it (FIFO A plays the right, B the left).
    pub fn last_mix(&self) -> Option<(&[i8], &[i8])> {
        let slot = self.last_slot?;
        let n = self.ring.samples_per_frame;
        Some((&self.ring.right[slot * n..(slot + 1) * n], &self.ring.left[slot * n..(slot + 1) * n]))
    }

    /// A PSG register's value as the CPU reads it back.
    pub fn psg_register(&self, reg: Reg) -> u8 {
        self.apu.read(reg)
    }

    /// Every Direct Sound channel's state, sounding or not.
    pub fn ds_states(&self) -> Vec<DsState> {
        self.ds
            .iter()
            .map(|c| DsState {
                note: note_state(c.status, &c.note),
                sample: c.sample,
                mix: (c.mix_right, c.mix_left),
                remaining: c.remaining,
                position: c.position,
                fraction: c.fraction,
            })
            .collect()
    }

    /// PSG channels 1..=4's state, sounding or not.
    pub fn psg_states(&self) -> [PsgState; 4] {
        std::array::from_fn(|i| {
            let c = &self.psg[i];
            PsgState {
                note: note_state(c.status, &c.note),
                envelope_goal: c.envelope_goal,
                sustain_goal: c.sustain_goal,
                envelope_counter: c.envelope_counter,
                pan: c.pan,
                nrx4: c.nrx4,
                length: c.length,
                sweep: c.sweep,
            }
        })
    }

    /// The hardware channels sounding now: the Direct Sound channels, then
    /// PSG channels 1 to 4.
    pub fn channels(&self) -> Vec<ChannelInfo> {
        let info = |hardware, status: Status, n: &Note, envelope: u8| ChannelInfo {
            hardware,
            owner: n.owner.map(|o| (PlayerId(o.player), o.track)),
            priority: n.priority,
            midi_key: n.midi_key,
            key: n.key,
            released: status.stop,
            starting: status.start,
            phase: status.phase,
            echo: status.echo,
            volume: (n.left_volume, n.right_volume),
            envelope,
            frequency: n.frequency,
            gate: n.gate,
        };
        let ds = self
            .ds
            .iter()
            .enumerate()
            .filter(|(_, c)| c.status.active())
            .map(|(i, c)| info(Hardware::DirectSound(i), c.status, &c.note, c.note.envelope_volume));
        let psg = self
            .psg
            .iter()
            .enumerate()
            .filter(|(_, c)| c.status.active())
            .map(|(i, c)| info(Hardware::Psg(i + 1), c.status, &c.note, c.note.envelope_volume));
        ds.chain(psg).collect()
    }

    /// `m4aSongNumStart`: start a song on its player (MPlayStart), unless
    /// the player's song outranks it. Returns whether it started.
    pub fn start(&mut self, id: SongId) -> bool {
        let bank = self.bank.clone();
        let Some(song) = bank.song(id) else { return false };
        let p = song.player.0 as usize;
        if self.players[p].refuses(song.priority) {
            return false;
        }
        for t in 0..self.players[p].tracks.len() {
            self.track_stop(p, t);
        }
        let pl = &mut self.players[p];
        pl.active = 0;
        pl.paused = false;
        pl.song = Some(id);
        pl.priority = song.priority;
        pl.clock = 0;
        pl.tempo = TICK;
        pl.tempo_step = TICK;
        pl.tempo_control = 0x100;
        pl.tempo_counter = 0;
        pl.fade_interval = 0;
        for (t, tr) in pl.tracks.iter_mut().enumerate() {
            tr.channels.clear();
            if t < song.tracks.len() {
                tr.exists = true;
                tr.starting = true;
                tr.pc = 0;
            } else {
                tr.exists = false;
                tr.starting = false;
            }
        }
        if let Some(r) = song.reverb {
            self.reverb = r;
        }
        true
    }

    /// `m4aSongNumStop`: stop a player if it is playing this song.
    pub fn stop_song(&mut self, id: SongId) {
        if let Some(song) = self.bank.song(id) {
            let p = song.player;
            if self.players[p.0 as usize].song == Some(id) {
                self.stop(p);
            }
        }
    }

    /// `m4aMPlayStop`: stop a player; its notes end at once.
    pub fn stop(&mut self, p: PlayerId) {
        let p = p.0 as usize;
        if p >= self.players.len() {
            return;
        }
        self.players[p].paused = true;
        for t in 0..self.players[p].tracks.len() {
            self.track_stop(p, t);
        }
    }

    /// `m4aMPlayAllStop`.
    pub fn stop_all(&mut self) {
        for p in 0..self.players.len() {
            self.stop(PlayerId(p as u8));
        }
    }

    /// `m4aMPlayTempoControl`: scale a player's tempo (0x100 = as written).
    pub fn set_tempo(&mut self, p: PlayerId, control: u16) {
        if let Some(pl) = self.players.get_mut(p.0 as usize) {
            pl.tempo_control = control;
            pl.tempo_step = ((pl.tempo as u32 * control as u32) >> 8) as u16;
        }
    }

    /// `m4aMPlayVolumeControl`: the volume (0x100 = full) of the tracks in
    /// the `tracks` bit mask.
    pub fn set_volume(&mut self, p: PlayerId, tracks: u16, volume: u16) {
        self.for_tracks(p, tracks, |t| {
            t.volume_control = (volume >> 2) as u8;
            t.changes.add(Changes::VOLUME);
        });
    }

    /// `m4aMPlayPitchControl`: shift the tracks in `tracks` by `pitch`
    /// (1/256 semitone).
    pub fn set_pitch(&mut self, p: PlayerId, tracks: u16, pitch: i16) {
        self.for_tracks(p, tracks, |t| {
            t.key_shift_control = (pitch >> 8) as i8;
            t.pitch_control = pitch as u8;
            t.changes.add(Changes::PITCH);
        });
    }

    /// `m4aMPlayFadeOut`: fade a player out over 16 steps of `speed`
    /// frames, then stop it.
    pub fn fade_out(&mut self, p: PlayerId, speed: u16) {
        if let Some(pl) = self.players.get_mut(p.0 as usize) {
            pl.fade_counter = speed;
            pl.fade_interval = speed;
            pl.fade_level = 64 << 2;
        }
    }

    fn for_tracks(&mut self, p: PlayerId, mask: u16, f: impl Fn(&mut TrackState)) {
        if let Some(pl) = self.players.get_mut(p.0 as usize) {
            for (i, t) in pl.tracks.iter_mut().enumerate() {
                if mask & (1 << i) != 0 && t.exists {
                    f(t);
                }
            }
        }
    }

    /// One frame (a VBlank): `m4aSoundVSync` and `SoundMain` (every
    /// player's sequencer, `CgbSound`, the Direct Sound mix), then the
    /// hardware plays the frame.
    pub fn step_frame(&mut self) {
        // m4aSoundVSync: the DMA moves to the next frame of the ring.
        self.dma_counter = self.dma_counter.wrapping_sub(1);
        if (self.dma_counter as i8) <= 0 {
            self.dma_counter = self.ring.frames as u8;
        }
        for p in 0..self.players.len() {
            self.play_frame(p);
        }
        mixer::cgb_sound(&mut self.psg, &mut self.apu, &self.bank, &mut self.psg_tick, self.bank.mixer.dac_resolution);
        let frames = self.ring.frames;
        let c = self.dma_counter as usize;
        let slot = if c >= 2 { frames - (c - 1) } else { 0 };
        let next = if c == 2 { 0 } else { (slot + 1) % frames };
        mixer::reverb_or_clear(&mut self.ring, slot, next, self.reverb);
        for ch in self.ds.iter_mut() {
            mixer::mix_ds(ch, &self.bank, self.master_volume, self.div_freq, &mut self.ring, slot);
        }
        self.play_hardware();
        self.last_slot = Some(slot);
    }

    /// The hardware's frame: the FIFOs play the slot mixed last frame while
    /// the PSG plays; samples at the DAC's rate.
    fn play_hardware(&mut self) {
        if let Some(slot) = self.last_slot {
            let n = self.ring.samples_per_frame;
            for k in slot * n..(slot + 1) * n {
                self.fifo.push_back((self.ring.right[k], self.ring.left[k]));
            }
        }
        let ds_period = FRAME_CYCLES / self.ring.samples_per_frame as u32;
        let dac_period = 512 >> (self.bank.mixer.dac_resolution & 3);
        let mut t = 0;
        while t < FRAME_CYCLES {
            // To the next DAC sample.
            let to_sample = dac_period - self.dac_phase;
            let to_ds = ds_period - self.ds_phase;
            let step = to_sample.min(to_ds).min(FRAME_CYCLES - t);
            self.apu.run(step);
            t += step;
            self.dac_phase += step;
            self.ds_phase += step;
            if self.ds_phase == ds_period {
                self.ds_phase = 0;
                if self.fifo.len() > 1 {
                    self.fifo.pop_front();
                }
            }
            if self.dac_phase == dac_period {
                self.dac_phase = 0;
                let (pl, pr) = self.apu.sample();
                let (dr, dl) = self.fifo.front().copied().unwrap_or((0, 0));
                // FIFO A (right) and B (left) at full volume, four DAC
                // steps a level; the bias clamps to the 10-bit DAC.
                let side = |psg: i32, ds: i8| -> i16 {
                    let v = (psg + ((ds as i32) << 2)).clamp(-0x200, 0x1FF);
                    ((v * 0x100 * 3) >> 4) as i16
                };
                self.pending.push([side(pl, dl), side(pr, dr)]);
            }
        }
    }

    /// Output at the DAC's rate (mGBA's scale: the 10-bit DAC times 48),
    /// (left, right), finished so far, appended to `out`.
    pub fn take_dac_output(&mut self, out: &mut Vec<[i16; 2]>) {
        out.append(&mut self.pending);
    }

    /// The DAC's output rate, Hz.
    pub fn dac_rate(&self) -> u32 {
        self.bank.mixer.dac_rate()
    }

    /// Output at 32768 Hz (+-1.0 full scale) finished so far, appended to
    /// `out`: the DAC's output averaged down to the rate, through the
    /// GBA's output capacitor (a high-pass at about 20 Hz, which takes out
    /// the PSG's offset).
    pub fn take_output(&mut self, out: &mut Vec<[f32; 2]>) {
        let ratio = (self.dac_rate() / crate::OUT_RATE).max(1) as usize;
        let mut samples = std::mem::take(&mut self.carry);
        samples.append(&mut self.pending);
        let whole = samples.len() / ratio * ratio;
        let scale = 1.0 / (ratio as f32 * 24576.0);
        let pole = 1.0 - 2.0 * std::f32::consts::PI * 20.0 / crate::OUT_RATE as f32;
        for chunk in samples[..whole].chunks(ratio) {
            let (l, r) = chunk.iter().fold((0i32, 0i32), |a, s| (a.0 + s[0] as i32, a.1 + s[1] as i32));
            let x = [l as f32 * scale, r as f32 * scale];
            let mut y = [0.0; 2];
            for c in 0..2 {
                y[c] = x[c] - self.dc_in[c] + pole * self.dc_out[c];
                self.dc_in[c] = x[c];
                self.dc_out[c] = y[c];
            }
            out.push(y);
        }
        self.carry = samples[whole..].to_vec();
    }

    fn ds_note(&mut self, i: usize) -> &mut Note {
        &mut self.ds[i].note
    }

    fn note_of(&mut self, s: Slot) -> &mut Note {
        match s {
            Slot::Ds(i) => self.ds_note(i),
            Slot::Psg(i) => &mut self.psg[i].note,
        }
    }

    fn status_of(&self, s: Slot) -> Status {
        match s {
            Slot::Ds(i) => self.ds[i].status,
            Slot::Psg(i) => self.psg[i].status,
        }
    }

    fn status_mut(&mut self, s: Slot) -> &mut Status {
        match s {
            Slot::Ds(i) => &mut self.ds[i].status,
            Slot::Psg(i) => &mut self.psg[i].status,
        }
    }

    /// TrackStop: the track's notes end at once and it lets its channels go.
    fn track_stop(&mut self, p: usize, t: usize) {
        let tr = &mut self.players[p].tracks[t];
        if !tr.exists {
            return;
        }
        for s in std::mem::take(&mut tr.channels) {
            if self.status_of(s) != Status::default() {
                if let Slot::Psg(i) = s {
                    mixer::cgb_off(&mut self.apu, i + 1);
                }
                *self.status_mut(s) = Status::default();
            }
            let n = self.note_of(s);
            n.owner = None;
            n.order = None;
        }
    }

    /// ClearChain: let a channel go from its track.
    fn release_from_owner(&mut self, s: Slot) {
        let n = self.note_of(s);
        let Some(owner) = n.owner.take() else { return };
        n.order = None;
        let tr = &mut self.players[owner.player as usize].tracks[owner.track as usize];
        tr.channels.retain(|&x| x != s);
    }

    /// MPlayMain for one player.
    fn play_frame(&mut self, p: usize) {
        if self.players[p].paused {
            return;
        }
        self.fade_step(p);
        if self.players[p].paused {
            return;
        }
        let bank = self.bank.clone();
        let song_id = self.players[p].song.expect("a playing player has a song");
        let song = bank.song(song_id).expect("songs in the bank");
        let pl = &mut self.players[p];
        pl.tempo_counter = pl.tempo_counter.wrapping_add(pl.tempo_step);
        while self.players[p].tempo_counter >= TICK {
            let mut active = 0u16;
            for t in 0..self.players[p].tracks.len() {
                if !self.players[p].tracks[t].exists {
                    continue;
                }
                active |= 1 << t;
                self.count_gates(p, t);
                let tr = &mut self.players[p].tracks[t];
                if tr.starting {
                    *tr = TrackState {
                        exists: true,
                        bend_range: 2,
                        volume_control: 0x40,
                        lfo_speed: 22,
                        pc: tr.pc,
                        ..TrackState::new()
                    };
                }
                while self.players[p].tracks[t].wait == 0 {
                    self.execute(p, t, &bank, song);
                    if !self.players[p].tracks[t].exists {
                        break;
                    }
                }
                let tr = &mut self.players[p].tracks[t];
                if !tr.exists {
                    continue;
                }
                tr.wait -= 1;
                lfo_step(tr);
            }
            let pl = &mut self.players[p];
            pl.clock = pl.clock.wrapping_add(1);
            if active == 0 {
                // Every track has ended.
                pl.active = 0;
                pl.paused = true;
                return;
            }
            pl.active = active;
            pl.tempo_counter -= TICK;
        }
        self.update_channels(p);
    }

    /// Count down the gates of a track's notes; unlink channels that ended.
    fn count_gates(&mut self, p: usize, t: usize) {
        let slots = self.players[p].tracks[t].channels.clone();
        for s in slots {
            if !self.status_of(s).active() {
                self.release_from_owner(s);
                continue;
            }
            let n = self.note_of(s);
            if n.gate != 0 {
                n.gate -= 1;
                if n.gate == 0 {
                    self.status_mut(s).stop = true;
                }
            }
        }
    }

    /// TrkVolPitSet and the channel pass: a track's changes go on to the
    /// notes it still owns.
    fn update_channels(&mut self, p: usize) {
        for t in 0..self.players[p].tracks.len() {
            let tr = &mut self.players[p].tracks[t];
            if !tr.exists || !tr.changes.any() {
                continue;
            }
            tr.vol_pit_set();
            let (vr, vl, key_m, pitch_m) = (tr.volume_right, tr.volume_left, tr.key_m, tr.pitch_m);
            let changes = tr.changes;
            let slots = tr.channels.clone();
            for s in slots {
                if !self.status_of(s).active() {
                    self.release_from_owner(s);
                    continue;
                }
                self.update_note(s, changes, vr, vl, key_m, pitch_m);
            }
            self.players[p].tracks[t].changes = Changes::default();
        }
    }

    /// A note follows its track's volume or pitch change.
    fn update_note(&mut self, s: Slot, changes: Changes, vr: u8, vl: u8, key_m: i8, pitch_m: u8) {
        if changes.volume_changed {
            mixer::set_volume(self.note_of(s), vr, vl);
            if let Slot::Psg(i) = s {
                self.psg[i].volume_changed = true;
            }
        }
        if changes.pitch_changed {
            let key = (self.note_of(s).key as i32 + key_m as i32).max(0) as u8;
            match s {
                Slot::Psg(i) => {
                    self.psg[i].note.frequency = tables::midi_key_to_cgb_freq(i + 1, key, pitch_m);
                    self.psg[i].pitch_changed = true;
                }
                Slot::Ds(i) => {
                    let sample = &self.bank.samples[self.ds[i].sample.0 as usize];
                    self.ds[i].note.frequency = mixer::ds_frequency(sample, key, pitch_m);
                }
            }
        }
    }

    /// FadeOutBody.
    fn fade_step(&mut self, p: usize) {
        let pl = &mut self.players[p];
        if pl.fade_interval == 0 {
            return;
        }
        pl.fade_counter = pl.fade_counter.wrapping_sub(1);
        if pl.fade_counter != 0 {
            return;
        }
        pl.fade_counter = pl.fade_interval;
        if pl.fade_level & FADE_IN != 0 {
            pl.fade_level = pl.fade_level.wrapping_add(16);
            if pl.fade_level > 0xFF {
                pl.fade_level = 0x100;
                pl.fade_interval = 0;
            }
        } else {
            pl.fade_level = pl.fade_level.wrapping_sub(16);
            if (pl.fade_level as i16) <= 0 {
                let keep = pl.fade_level & FADE_KEEPS_TRACKS != 0;
                for t in 0..self.players[p].tracks.len() {
                    self.track_stop(p, t);
                    if !keep {
                        self.players[p].tracks[t].exists = false;
                    }
                }
                let pl = &mut self.players[p];
                pl.paused = true;
                if !keep {
                    pl.active = 0;
                }
                pl.fade_interval = 0;
                return;
            }
        }
        let level = (pl.fade_level >> 2) as u8;
        for tr in pl.tracks.iter_mut().filter(|t| t.exists) {
            tr.volume_control = level;
            tr.changes.add(Changes::VOLUME);
        }
    }

    /// Run one command of a track.
    fn execute(&mut self, p: usize, t: usize, bank: &SoundBank, song: &Song) {
        let commands = &song.tracks[t].commands;
        let pl = &mut self.players[p];
        let tr = &mut pl.tracks[t];
        let Some(&cmd) = commands.get(tr.pc) else {
            // Ran off the end (the bank never has this): as FINE.
            self.fine(p, t);
            return;
        };
        tr.pc += 1;
        match cmd {
            Command::Wait(n) => tr.wait = n,
            Command::Fine => self.fine(p, t),
            Command::Goto(i) => tr.pc = i as usize,
            Command::Call(i) => {
                // Too deep: the driver ends the track.
                if tr.calls.len() >= MAX_CALLS {
                    self.fine(p, t);
                    return;
                }
                tr.calls.push(tr.pc);
                tr.pc = i as usize;
            }
            Command::Return => {
                if let Some(r) = tr.calls.pop() {
                    tr.pc = r;
                }
            }
            Command::Repeat { count, target } => {
                if count == 0 {
                    tr.pc = target as usize;
                } else {
                    tr.repeats = tr.repeats.wrapping_add(1);
                    if tr.repeats < count {
                        tr.pc = target as usize;
                    } else {
                        tr.repeats = 0;
                    }
                }
            }
            Command::Priority(v) => tr.priority = v,
            Command::Tempo(v) => {
                pl.tempo = v as u16 * 2;
                pl.tempo_step = ((pl.tempo as u32 * pl.tempo_control as u32) >> 8) as u16;
            }
            Command::KeyShift(v) => {
                tr.key_shift = v;
                tr.changes.add(Changes::PITCH);
            }
            Command::Voice(v) => {
                tr.voice =
                    bank.voicegroups[song.voicegroup.0 as usize].voices.get(v as usize).copied().unwrap_or(START_VOICE);
            }
            Command::Volume(v) => {
                tr.volume = v;
                tr.changes.add(Changes::VOLUME);
            }
            Command::Pan(v) => {
                tr.pan = (v as i32 - 64) as i8;
                tr.changes.add(Changes::VOLUME);
            }
            Command::Bend(v) => {
                tr.bend = (v as i32 - 64) as i8;
                tr.changes.add(Changes::PITCH);
            }
            Command::BendRange(v) => {
                tr.bend_range = v;
                tr.changes.add(Changes::PITCH);
            }
            Command::LfoSpeed(v) => {
                tr.lfo_speed = v;
                if v == 0 {
                    tr.clear_modulation();
                }
            }
            Command::LfoDelay(v) => tr.lfo_delay = v,
            Command::Modulation(v) => {
                tr.modulation = v;
                if v == 0 {
                    tr.clear_modulation();
                }
            }
            Command::ModulationType(v) => {
                if tr.mod_type != v {
                    tr.mod_type = v;
                    tr.changes.add(Changes::ALL);
                }
            }
            Command::Tune(v) => {
                tr.tune = (v as i32 - 64) as i8;
                tr.changes.add(Changes::PITCH);
            }
            Command::EchoVolume(v) => tr.echo_volume = v,
            Command::EchoLength(v) => tr.echo_length = v,
            Command::EndTie { key } => {
                if let Some(k) = key {
                    tr.key = k;
                }
                let key = tr.key;
                let slots = tr.channels.clone();
                for s in slots {
                    if self.status_of(s).holding() && self.note_of(s).midi_key == key {
                        self.status_mut(s).stop = true;
                        break;
                    }
                }
            }
            Command::Note { gate, key, velocity } => {
                tr.gate = gate;
                if let Some(k) = key {
                    tr.key = k;
                }
                if let Some(v) = velocity {
                    tr.velocity = v;
                }
                self.note(p, t);
            }
            Command::MemAcc { op, address, operand } => {
                let m = &mut self.memory;
                let a = address as usize;
                let from = m[operand as usize];
                match op {
                    MemOp::Set => m[a] = operand,
                    MemOp::Add => m[a] = m[a].wrapping_add(operand),
                    MemOp::Sub => m[a] = m[a].wrapping_sub(operand),
                    MemOp::SetFromMemory => m[a] = from,
                    MemOp::AddFromMemory => m[a] = m[a].wrapping_add(from),
                    MemOp::SubFromMemory => m[a] = m[a].wrapping_sub(from),
                    MemOp::JumpIf { test, with_memory, target } => {
                        let other = if with_memory { from } else { operand };
                        if test.holds(m[a], other) {
                            tr.pc = target as usize;
                        }
                    }
                }
            }
        }
    }

    /// ply_fine: the track ends; its notes release.
    fn fine(&mut self, p: usize, t: usize) {
        for s in std::mem::take(&mut self.players[p].tracks[t].channels) {
            if self.status_of(s).active() {
                self.status_mut(s).stop = true;
            }
            let n = self.note_of(s);
            n.owner = None;
            n.order = None;
        }
        let tr = &mut self.players[p].tracks[t];
        tr.exists = false;
        tr.starting = false;
        tr.changes = Changes::default();
    }

    /// ply_note: find the instrument and a channel for the track's note.
    fn note(&mut self, p: usize, t: usize) {
        let bank = self.bank.clone();
        let pl = &self.players[p];
        let tr = &pl.tracks[t];
        let (voice, key, rhythm_pan) = match tr.voice.kind {
            VoiceKind::Drums { kit } => {
                let Some(&sub) = bank.voicegroups[kit.0 as usize].voices.get(tr.key as usize) else { return };
                (sub, sub.key, sub.pan.unwrap_or(0))
            }
            VoiceKind::Split { group, map } => {
                let i = bank.key_maps[map.0 as usize].0[(tr.key & 0x7F) as usize];
                (bank.voicegroups[group.0 as usize].voices[i as usize], tr.key, 0)
            }
            _ => (tr.voice, tr.key, 0),
        };
        if matches!(voice.kind, VoiceKind::Drums { .. } | VoiceKind::Split { .. } | VoiceKind::Silent) {
            return;
        }
        let priority = (pl.priority as u32 + tr.priority as u32).min(255) as u8;
        let order = Some((pl.config.track_order, t as u8));
        let Some(slot) = self.allocate(voice.kind.psg_channel(), priority, order) else { return };
        self.release_from_owner(slot);
        let tr = &mut self.players[p].tracks[t];
        tr.channels.insert(0, slot);
        tr.lfo_delay_count = tr.lfo_delay;
        if tr.lfo_delay != 0 {
            tr.clear_modulation();
        }
        tr.vol_pit_set();
        let note = Note {
            owner: Some(TrackRef { player: p as u8, track: t as u8 }),
            order,
            priority,
            midi_key: tr.key,
            key,
            velocity: tr.velocity,
            rhythm_pan,
            gate: tr.gate,
            right_volume: 0,
            left_volume: 0,
            envelope: voice.envelope,
            envelope_volume: 0,
            echo_volume: tr.echo_volume,
            echo_length: tr.echo_length,
            frequency: 0,
        };
        let (vr, vl, key_m, pitch_m) = (tr.volume_right, tr.volume_left, tr.key_m, tr.pitch_m);
        tr.changes = Changes::default();
        let shifted = (key as i32 + key_m as i32).max(0) as u8;
        match slot {
            Slot::Ds(i) => {
                let VoiceKind::DirectSound { sample, fixed } = voice.kind else { unreachable!() };
                let ch = &mut self.ds[i];
                // (The envelope level and position carry over until the
                // mixer starts the note.)
                ch.note = Note { envelope_volume: ch.note.envelope_volume, ..note };
                ch.sample = sample;
                ch.fixed = fixed;
                mixer::set_volume(&mut ch.note, vr, vl);
                ch.note.frequency = mixer::ds_frequency(&bank.samples[sample.0 as usize], shifted, pitch_m);
                ch.status = Status { start: true, ..Status::default() };
            }
            Slot::Psg(i) => {
                let ch = &mut self.psg[i];
                ch.note = Note { envelope_volume: ch.note.envelope_volume, ..note };
                ch.voice = voice.kind;
                ch.length = voice.length;
                ch.sweep = match voice.kind {
                    VoiceKind::Square1 { sweep, .. } => sweep,
                    _ => NO_SWEEP,
                };
                mixer::set_volume(&mut ch.note, vr, vl);
                ch.note.frequency = tables::midi_key_to_cgb_freq(i + 1, shifted, pitch_m);
                ch.status = Status { start: true, ..Status::default() };
            }
        }
    }

    /// A channel for a note of `priority` from a track at `order`, as
    /// ply_note chooses: a PSG voice needs its own channel, free, released,
    /// or held by a lower note; Direct Sound takes a free channel, else the
    /// weakest released one, else the weakest that yields.
    fn allocate(&self, psg: Option<usize>, priority: u8, order: TrackOrder) -> Option<Slot> {
        if let Some(n) = psg {
            let i = n - 1;
            let c = &self.psg[i];
            if c.status.active() && !c.status.stop {
                let yields = c.note.priority < priority || (c.note.priority == priority && c.note.order >= order);
                return yields.then_some(Slot::Psg(i));
            }
            return Some(Slot::Psg(i));
        }
        let mut best = (priority, order);
        let mut found_released = false;
        let mut pick = None;
        for (i, c) in self.ds.iter().enumerate() {
            if !c.status.active() {
                return Some(Slot::Ds(i));
            }
            let (cp, co) = (c.note.priority, c.note.order);
            if c.status.stop {
                if !found_released {
                    found_released = true;
                    best = (cp, co);
                    pick = Some(i);
                    continue;
                }
            } else if found_released {
                continue;
            }
            if cp < best.0 {
                best = (cp, co);
                pick = Some(i);
            } else if cp == best.0 && co >= best.1 {
                best.1 = co;
                pick = Some(i);
            }
        }
        pick.map(Slot::Ds)
    }
}

/// A tick of a track's LFO: count the delay down, else move the phase and
/// take the triangle wave times the depth; a new value is a change.
fn lfo_step(tr: &mut TrackState) {
    if tr.lfo_speed == 0 || tr.modulation == 0 {
        return;
    }
    if tr.lfo_delay_count != 0 {
        tr.lfo_delay_count -= 1;
        return;
    }
    let c = tr.lfo_phase as i32 + tr.lfo_speed as i32;
    tr.lfo_phase = c as u8;
    // The triangle: rising through 0..64, falling to -64 at 192, rising
    // again. (The falling side takes the sum before it wraps: past 255 it
    // comes out 256 lower.)
    let r = if ((c - 0x40) as u8 as i8) < 0 { c as u8 as i8 as i32 } else { 0x80 - c };
    let v = ((tr.modulation as i32 * r) >> 6) as i8;
    if v != tr.mod_value {
        tr.mod_value = v;
        tr.changes.add(if tr.mod_type == 0 { Changes::PITCH } else { Changes::VOLUME });
    }
}
