//! The M4A driver, one frame at a time: music players, their tracks'
//! sequencers, and the hardware channels they share.
//!
//! Follows the GBA driver (m4a_1.s and m4a.c): `MPlayStart` decides whether
//! a song may take its player, `MPlayMain` runs each player's tracks at
//! their tempo, `ply_note` finds a channel for each note (stealing from a
//! lower-priority note of any player), and the player controls (tempo,
//! pitch, volume, fades, stops) are the driver's `m4aMPlay*` calls.

use crate::bank::*;
use crate::mixer::{Channel, Mixer, TrackOrder, TrackRef};
use std::sync::Arc;

/// Tempo counter steps per tick (the driver plays a tick each time the
/// per-frame tempo it adds reaches this).
const TICK: u16 = 150;
/// Pattern calls nest this deep.
const MAX_CALLS: usize = 3;

/// A track's state (the driver's `MusicPlayerTrack`).
#[derive(Clone, Debug)]
struct TrackState {
    /// Playing (the EXIST flag): cleared by FINE.
    exists: bool,
    /// Just started: reset before its first tick.
    starting: bool,
    pc: usize,
    wait: u8,
    calls: Vec<usize>,
    repeats: u8,
    key: u8,
    velocity: u8,
    gate: u8,
    voice: Voice,
    volume: u8,
    /// PAN - 64.
    pan: i8,
    /// BEND - 64.
    bend: i8,
    bend_range: u8,
    lfo_speed: u8,
    lfo_delay: u8,
    lfo_delay_count: u8,
    lfo_phase: u8,
    modulation: u8,
    mod_type: u8,
    /// The LFO's current offset (vibrato, tremolo or pan).
    mod_value: i8,
    /// TUNE - 64.
    tune: i8,
    key_shift: i8,
    priority: u8,
    echo_volume: u8,
    echo_length: u8,
    /// Player controls: volume (0..=64), key shift and fine pitch.
    volume_control: u8,
    key_shift_control: i8,
    pitch_control: u8,
    /// Its channels, newest first.
    channels: Vec<Slot>,
}

impl TrackState {
    fn new() -> TrackState {
        TrackState {
            exists: false,
            starting: false,
            pc: 0,
            wait: 0,
            calls: Vec::new(),
            repeats: 0,
            key: 0,
            velocity: 0,
            gate: 0,
            voice: START_VOICE,
            volume: 0,
            pan: 0,
            bend: 0,
            bend_range: 0,
            lfo_speed: 0,
            lfo_delay: 0,
            lfo_delay_count: 0,
            lfo_phase: 0,
            modulation: 0,
            mod_type: 0,
            mod_value: 0,
            tune: 0,
            key_shift: 0,
            priority: 0,
            echo_volume: 0,
            echo_length: 0,
            volume_control: 0,
            key_shift_control: 0,
            pitch_control: 0,
            channels: Vec::new(),
        }
    }

    /// Right and left track volume (TrkVolPitSet's volMR, volML).
    fn volumes(&self) -> (i64, i64) {
        let mut x = (self.volume as i64 * self.volume_control as i64) >> 5;
        if self.mod_type == 1 {
            x = (x * (self.mod_value as i64 + 128)) >> 7;
        }
        let mut y = 2 * self.pan as i64;
        if self.mod_type == 2 {
            y += self.mod_value as i64;
        }
        let y = y.clamp(-128, 127);
        (((y + 128) * x) >> 8, ((127 - y) * x) >> 8)
    }

    /// The key shift in semitones and the fraction above it (1/256), from
    /// tune, bend, key shifts, the pitch control and vibrato.
    fn pitch(&self) -> (i32, u8) {
        let bend = self.bend as i32 * self.bend_range as i32;
        let mut x = (self.tune as i32 + bend) * 4
            + ((self.key_shift as i32) << 8)
            + ((self.key_shift_control as i32) << 8)
            + self.pitch_control as i32;
        if self.mod_type == 0 {
            x += 16 * self.mod_value as i32;
        }
        (x >> 8, (x & 0xFF) as u8)
    }

    /// clear_modM: the LFO starts over.
    fn clear_modulation(&mut self) {
        self.lfo_phase = 0;
        self.mod_value = 0;
    }
}

/// The voice a track has before its first VOICE command: the driver's
/// cleared tone with type 1 (square 1, all settings zero).
const START_VOICE: Voice = Voice {
    kind: VoiceKind::Square1 { duty: 0 },
    key: 0,
    pan: None,
    envelope: Envelope { attack: 0, decay: 0, sustain: 0, release: 0 },
};

/// A hardware channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Ds(usize),
    /// PSG channel 1..=4, stored at index 0..=3.
    Psg(usize),
}

/// A fade in progress (the driver's fadeOI, fadeOC, fadeOV).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fade {
    interval: u16,
    counter: u16,
    /// Volume in 1/4 steps of 0..=64.
    volume: i32,
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
    /// Song tempo (BPM), tempo control (1/256) and the resulting per-frame step.
    tempo: u16,
    tempo_control: u16,
    tempo_step: u16,
    tempo_counter: u16,
    fade: Option<Fade>,
    tracks: Vec<TrackState>,
}

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
            fade: None,
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
    /// The key it sounds at (with the track's shifts).
    pub key: i32,
    /// Released, fading out.
    pub released: bool,
    /// Left and right volume.
    pub volume: (u8, u8),
}

/// The sound driver: every music player and the hardware they share.
pub struct Driver {
    bank: Arc<SoundBank>,
    players: Vec<MusicPlayer>,
    mixer: Mixer,
}

impl Driver {
    pub fn new(bank: Arc<SoundBank>) -> Driver {
        let players = bank.players.iter().map(|&c| MusicPlayer::new(c)).collect();
        let mixer = Mixer::new(&bank.mixer);
        Driver { bank, players, mixer }
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

    /// The hardware channels sounding now: the Direct Sound channels, then
    /// PSG channels 1 to 4.
    pub fn channels(&self) -> Vec<ChannelInfo> {
        let ds = self.mixer.ds.iter().enumerate().map(|(i, c)| (Hardware::DirectSound(i), c));
        let psg = self.mixer.psg.iter().enumerate().map(|(i, c)| (Hardware::Psg(i + 1), c));
        ds.chain(psg)
            .filter_map(|(hardware, c)| {
                let c = c.as_ref().filter(|c| c.on)?;
                Some(ChannelInfo {
                    hardware,
                    owner: c.owner.map(|o| (PlayerId(o.player), o.track)),
                    priority: c.priority,
                    key: c.key,
                    released: c.stopping,
                    volume: (c.left as u8, c.right as u8),
                })
            })
            .collect()
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
        pl.fade = None;
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
            self.mixer.reverb = r;
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
        self.for_tracks(p, tracks, |t| t.volume_control = (volume >> 2) as u8);
    }

    /// `m4aMPlayPitchControl`: shift the tracks in `tracks` by `pitch`
    /// (1/256 semitone).
    pub fn set_pitch(&mut self, p: PlayerId, tracks: u16, pitch: i16) {
        self.for_tracks(p, tracks, |t| {
            t.key_shift_control = (pitch >> 8) as i8;
            t.pitch_control = pitch as u8;
        });
    }

    /// `m4aMPlayFadeOut`: fade a player out over 16 steps of `speed`
    /// frames, then stop it.
    pub fn fade_out(&mut self, p: PlayerId, speed: u16) {
        if let Some(pl) = self.players.get_mut(p.0 as usize)
            && pl.song.is_some()
        {
            pl.fade = Some(Fade { interval: speed, counter: speed, volume: 64 << 2 });
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

    /// One frame (a VBlank): every player's sequencer, then the mix.
    pub fn step_frame(&mut self) {
        for p in 0..self.players.len() {
            self.play_frame(p);
        }
        self.mixer.mix_frame(&self.bank);
    }

    /// Output at 32768 Hz (+-1.0 full scale) finished so far, appended to `out`.
    pub fn take_output(&mut self, out: &mut Vec<[f32; 2]>) {
        self.mixer.take_output(out);
    }

    fn channel(&mut self, s: Slot) -> Option<&mut Channel> {
        match s {
            Slot::Ds(i) => self.mixer.ds[i].as_mut(),
            Slot::Psg(i) => self.mixer.psg[i].as_mut(),
        }
    }

    /// TrackStop: the track's notes end at once and it lets its channels go.
    fn track_stop(&mut self, p: usize, t: usize) {
        let tr = &mut self.players[p].tracks[t];
        if !tr.exists {
            return;
        }
        for s in std::mem::take(&mut tr.channels) {
            if let Some(ch) = self.channel(s) {
                ch.on = false;
                ch.owner = None;
                ch.order = None;
            }
        }
    }

    /// Let a channel go from its track (ClearChain).
    fn release_from_owner(&mut self, s: Slot) {
        let Some(ch) = self.channel(s) else { return };
        let Some(owner) = ch.owner.take() else { return };
        ch.order = None;
        let tr = &mut self.players[owner.player as usize].tracks[owner.track as usize];
        tr.channels.retain(|&x| x != s);
    }

    /// MPlayMain for one player.
    fn play_frame(&mut self, p: usize) {
        if !self.players[p].paused {
            self.fade_step(p);
        }
        if !self.players[p].paused {
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
                        let volume_control = 0x40;
                        *tr = TrackState {
                            exists: true,
                            bend_range: 2,
                            lfo_speed: 22,
                            volume_control,
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
                    break;
                }
                pl.active = active;
                pl.tempo_counter -= TICK;
            }
        }
        self.update_channels(p);
    }

    /// Count down the gates of a track's notes; unlink channels that ended.
    fn count_gates(&mut self, p: usize, t: usize) {
        let slots = self.players[p].tracks[t].channels.clone();
        for s in slots {
            let ch = self.channel(s).expect("linked channels exist");
            if !ch.on {
                self.release_from_owner(s);
                continue;
            }
            if ch.gate != 0 {
                ch.gate -= 1;
                if ch.gate == 0 {
                    ch.stopping = true;
                }
            }
        }
    }

    /// TrkVolPitSet and the channel updates after the ticks: every note a
    /// track still owns follows its volume and pitch.
    fn update_channels(&mut self, p: usize) {
        for t in 0..self.players[p].tracks.len() {
            let tr = &self.players[p].tracks[t];
            if !tr.exists || tr.channels.is_empty() {
                continue;
            }
            let (vr, vl) = tr.volumes();
            let (shift, pitch) = tr.pitch();
            let slots = tr.channels.clone();
            for s in slots {
                let ch = self.channel(s).expect("linked channels exist");
                if ch.on {
                    set_channel_volume(ch, vr, vl);
                    ch.key = (ch.midi_key as i32 + shift).max(0);
                    ch.pitch = pitch;
                }
            }
        }
    }

    /// FadeOutBody.
    fn fade_step(&mut self, p: usize) {
        let Some(mut fade) = self.players[p].fade else { return };
        fade.counter = fade.counter.wrapping_sub(1);
        if fade.counter != 0 {
            self.players[p].fade = Some(fade);
            return;
        }
        fade.counter = fade.interval;
        fade.volume -= 4 << 2;
        if fade.volume <= 0 {
            for t in 0..self.players[p].tracks.len() {
                self.track_stop(p, t);
                self.players[p].tracks[t].exists = false;
            }
            let pl = &mut self.players[p];
            pl.paused = true;
            pl.active = 0;
            pl.fade = None;
            return;
        }
        let pl = &mut self.players[p];
        pl.fade = Some(fade);
        for tr in pl.tracks.iter_mut().filter(|t| t.exists) {
            tr.volume_control = (fade.volume >> 2) as u8;
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
                    tr.repeats += 1;
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
            Command::KeyShift(v) => tr.key_shift = v,
            Command::Voice(v) => {
                tr.voice =
                    bank.voicegroups[song.voicegroup.0 as usize].voices.get(v as usize).copied().unwrap_or(START_VOICE);
            }
            Command::Volume(v) => tr.volume = v,
            Command::Pan(v) => tr.pan = (v as i32 - 64) as i8,
            Command::Bend(v) => tr.bend = (v as i32 - 64) as i8,
            Command::BendRange(v) => tr.bend_range = v,
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
            Command::ModulationType(v) => tr.mod_type = v,
            Command::Tune(v) => tr.tune = (v as i32 - 64) as i8,
            Command::EchoVolume(v) => tr.echo_volume = v,
            Command::EchoLength(v) => tr.echo_length = v,
            Command::EndTie { key } => {
                if let Some(k) = key {
                    tr.key = k;
                }
                let key = tr.key;
                let slots = tr.channels.clone();
                for s in slots {
                    let ch = self.channel(s).expect("linked channels exist");
                    if ch.on && !ch.stopping && ch.midi_key == key {
                        ch.stopping = true;
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
        }
    }

    /// ply_fine: the track ends; its notes release.
    fn fine(&mut self, p: usize, t: usize) {
        for s in std::mem::take(&mut self.players[p].tracks[t].channels) {
            if let Some(ch) = self.channel(s) {
                if ch.on {
                    ch.stopping = true;
                }
                ch.owner = None;
                ch.order = None;
            }
        }
        self.players[p].tracks[t].exists = false;
    }

    /// ply_note: find the instrument and a channel for the track's note.
    fn note(&mut self, p: usize, t: usize) {
        let bank = self.bank.clone();
        let pl = &self.players[p];
        let tr = &pl.tracks[t];
        let (voice, midi_key, rhythm_pan) = match tr.voice.kind {
            VoiceKind::Drums { kit } => {
                let Some(&sub) = bank.voicegroups[kit.0 as usize].voices.get(tr.key as usize) else { return };
                (sub, sub.key, sub.pan.map_or(0, |x| x as i32))
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
        let order = (pl.config.track_order, t as u8);
        let Some(slot) = self.allocate(voice.kind.psg_channel(), priority, Some(order)) else { return };
        self.release_from_owner(slot);
        let tr = &mut self.players[p].tracks[t];
        tr.channels.insert(0, slot);
        tr.lfo_delay_count = tr.lfo_delay;
        if tr.lfo_delay != 0 {
            tr.clear_modulation();
        }
        let (vr, vl) = tr.volumes();
        let (shift, pitch) = tr.pitch();
        let owner = TrackRef { player: p as u8, track: t as u8 };
        let mut ch = Channel::new(
            owner,
            order,
            priority,
            voice,
            midi_key,
            tr.velocity,
            rhythm_pan,
            tr.gate,
            (tr.echo_volume, tr.echo_length),
        );
        set_channel_volume(&mut ch, vr, vl);
        ch.key = (midi_key as i32 + shift).max(0);
        ch.pitch = pitch;
        match slot {
            Slot::Ds(i) => self.mixer.ds[i] = Some(ch),
            Slot::Psg(i) => self.mixer.psg[i] = Some(ch),
        }
    }

    /// A channel for a note of `priority` from a track at `order`, as
    /// ply_note chooses: a PSG voice needs its own channel, free, released,
    /// or held by a lower note; Direct Sound takes a free channel, else the
    /// weakest released one, else the weakest that yields.
    fn allocate(&self, psg: Option<usize>, priority: u8, order: TrackOrder) -> Option<Slot> {
        if let Some(n) = psg {
            let i = n - 1;
            return match &self.mixer.psg[i] {
                Some(c) if c.on && !c.stopping => {
                    let yields = c.priority < priority || (c.priority == priority && c.order >= order);
                    yields.then_some(Slot::Psg(i))
                }
                _ => Some(Slot::Psg(i)),
            };
        }
        let mut best = (priority, order);
        let mut found_released = false;
        let mut pick = None;
        for (i, c) in self.mixer.ds.iter().enumerate() {
            let Some(c) = c.as_ref().filter(|c| c.on) else { return Some(Slot::Ds(i)) };
            if c.stopping {
                if !found_released {
                    found_released = true;
                    best = (c.priority, c.order);
                    pick = Some(i);
                    continue;
                }
            } else if found_released {
                continue;
            }
            if c.priority < best.0 {
                best = (c.priority, c.order);
                pick = Some(i);
            } else if c.priority == best.0 && c.order >= best.1 {
                best.1 = c.order;
                pick = Some(i);
            }
        }
        pick.map(Slot::Ds)
    }
}

/// ChnVolSetAsm: a channel's volumes from its track's and its velocity and
/// drum pan.
fn set_channel_volume(ch: &mut Channel, vr: i64, vl: i64) {
    let (vel, rpan) = (ch.velocity as i64, ch.rhythm_pan as i64);
    ch.right = (((0x80 + rpan) * vel * vr) >> 14).min(255);
    ch.left = (((0x7F - rpan) * vel * vl) >> 14).min(255);
}

/// A tick of a track's LFO: count the delay down, else move the phase and
/// take the triangle wave times the depth.
fn lfo_step(tr: &mut TrackState) {
    if tr.lfo_speed == 0 || tr.modulation == 0 {
        return;
    }
    if tr.lfo_delay_count != 0 {
        tr.lfo_delay_count -= 1;
        return;
    }
    tr.lfo_phase = tr.lfo_phase.wrapping_add(tr.lfo_speed);
    let c = tr.lfo_phase as i32;
    // The triangle: rising through 0..64, falling to -64 at 192, rising again.
    let r = if (c.wrapping_sub(0x40) as i8) < 0 { c as i8 as i32 } else { 0x80 - c };
    tr.mod_value = ((tr.modulation as i32 * r) >> 6) as i8;
}
