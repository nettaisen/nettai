//! The hardware channels and what the driver does with them each frame
//! once the sequencers have run:
//!
//! - `CgbSound`: the PSG channels' envelopes (counted in frames) and the
//!   register writes that start, steer and stop the PSG voices;
//! - `SoundMainRAM`: the Direct Sound channels' envelopes and their mix
//!   into the PCM ring buffer, eight bits a sample, with the ring's reverb.
//!
//! Both follow the driver's code step for step, in its integer arithmetic,
//! so the PCM buffer holds the bytes the game's buffer holds.
//!
//! That is [`Mixer::Original`]. The driver mixes with [`Mixer::Hq`] unless
//! told otherwise: the same channels mixed after ipatix's HQ mixer
//! (gba-hq-mixer, MIT), at a higher precision, and played at it.

use crate::apu::{Apu, Reg};
use crate::bank::{Envelope, NO_SWEEP, Sample, SampleId, SoundBank, VoiceKind, WaveId};
use crate::tables;

/// A track of a music player, as a channel names its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TrackRef {
    pub player: u8,
    pub track: u8,
}

/// Where a channel's track sorts when channels of equal priority compete
/// (the driver compares track addresses; a channel its track let go has
/// none and sorts first).
pub(crate) type TrackOrder = Option<(u8, u8)>;

/// A channel's envelope phase (the status byte's low two bits).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// Releasing, or idle.
    #[default]
    Release,
    Sustain,
    Decay,
    Attack,
}

impl Phase {
    /// The phase after this one (attack, decay, sustain).
    fn next(self) -> Phase {
        match self {
            Phase::Attack => Phase::Decay,
            Phase::Decay => Phase::Sustain,
            _ => Phase::Release,
        }
    }
}

/// A channel's status (the driver's status byte).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Status {
    /// A note to start: set by `ply_note`, started by the mixer.
    pub start: bool,
    /// Released (gate end, EOT, FINE): the envelope releases.
    pub stop: bool,
    /// Its sample loops (Direct Sound).
    pub looping: bool,
    /// Pseudo-echo: held at the echo volume for its length.
    pub echo: bool,
    pub phase: Phase,
}

impl Status {
    /// Sounding or about to (the driver's `status & 0xC7`).
    pub fn active(&self) -> bool {
        self.start || self.stop || self.echo || self.phase != Phase::Release
    }
    /// What EOT may release (`status & 0x83` and not stopped).
    pub fn holding(&self) -> bool {
        (self.start || self.phase != Phase::Release) && !self.stop
    }
    fn off() -> Status {
        Status::default()
    }
    /// The driver's status byte (for comparing with the game's memory).
    pub fn driver_byte(&self) -> u8 {
        (self.start as u8) << 7
            | (self.stop as u8) << 6
            | (self.looping as u8) << 4
            | (self.echo as u8) << 2
            | self.phase as u8
    }
}

/// What every channel has: the note and the track it belongs to.
#[derive(Clone, Debug, Default)]
pub(crate) struct Note {
    pub owner: Option<TrackRef>,
    /// The player whose track started it: kept when the track lets the note go (its release still sounds at
    /// that player's level, [`crate::Driver::set_level`]).
    pub player: u8,
    pub order: TrackOrder,
    pub priority: u8,
    /// The key the note named (what EOT matches).
    pub midi_key: u8,
    /// The key it plays (a drum's own key), before the track's shifts.
    pub key: u8,
    pub velocity: u8,
    pub rhythm_pan: i8,
    /// Ticks until release; 0 holds (a tie).
    pub gate: u8,
    pub right_volume: u8,
    pub left_volume: u8,
    pub envelope: Envelope,
    pub envelope_volume: u8,
    pub echo_volume: u8,
    pub echo_length: u8,
    pub frequency: u32,
}

/// A Direct Sound channel (the driver's `SoundChannel`).
#[derive(Clone, Debug)]
pub(crate) struct DsChannel {
    pub status: Status,
    pub note: Note,
    pub sample: SampleId,
    /// Played at the mixing rate whatever the key.
    pub fixed: bool,
    /// The envelope times the volumes, for the mix.
    pub mix_right: u8,
    pub mix_left: u8,
    /// Samples left to the end of the data, from `position`.
    pub remaining: i32,
    /// The sample being played and the fraction past it (23 bits).
    pub position: u32,
    pub fraction: u32,
}

impl DsChannel {
    pub fn new() -> DsChannel {
        DsChannel {
            status: Status::off(),
            note: Note::default(),
            sample: SampleId(0),
            fixed: false,
            mix_right: 0,
            mix_left: 0,
            remaining: 0,
            position: 0,
            fraction: 0,
        }
    }
}

/// A PSG channel (the driver's `CgbChannel`).
#[derive(Clone, Debug)]
pub(crate) struct CgbChannel {
    pub status: Status,
    pub note: Note,
    /// The voice it plays (duty, wave, noise width, fixed frequency).
    pub voice: VoiceKind,
    /// The level the envelope goes up to and the one it sustains at.
    pub envelope_goal: u8,
    pub sustain_goal: u8,
    /// Frames until the envelope's next step.
    pub envelope_counter: u8,
    /// NR51's bits for it, and the mask of its own bits.
    pub pan: u8,
    pub pan_mask: u8,
    /// The NRx4 value it writes: length enable (bit 6), the frequency's
    /// high bits; the wave channel's restart (bit 7) until its next
    /// volume write.
    pub nrx4: u8,
    /// Its volume or its pitch changed since the last frame.
    pub volume_changed: bool,
    pub pitch_changed: bool,
    pub length: u8,
    pub sweep: u8,
    /// The wave in wave RAM (the wave channel).
    pub loaded_wave: Option<WaveId>,
}

impl CgbChannel {
    /// PSG channel `n` (1..=4) as `MPlayExtender` sets it up.
    pub fn new(n: usize) -> CgbChannel {
        let voice = match n {
            1 => VoiceKind::Square1 { duty: 0, sweep: NO_SWEEP, fixed: false },
            2 => VoiceKind::Square2 { duty: 0, fixed: false },
            3 => VoiceKind::Wave { wave: WaveId(0), fixed: false },
            _ => VoiceKind::Noise { narrow: false },
        };
        CgbChannel {
            status: Status::off(),
            note: Note::default(),
            voice,
            envelope_goal: 0,
            sustain_goal: 0,
            envelope_counter: 0,
            pan: 0,
            pan_mask: 0x11 << (n - 1),
            nrx4: 0,
            volume_changed: false,
            pitch_changed: false,
            length: 0,
            sweep: 0,
            loaded_wave: None,
        }
    }
}

/// `ChnVolSetAsm`: a channel's volumes from its track's, its velocity and
/// its drum pan.
pub(crate) fn set_volume(note: &mut Note, track_right: u8, track_left: u8) {
    let (vel, rpan) = (note.velocity as i32, note.rhythm_pan as i32);
    note.right_volume = (((0x80 + rpan) * vel * track_right as i32) >> 14).min(255) as u8;
    note.left_volume = (((0x7F - rpan) * vel * track_left as i32) >> 14).min(255) as u8;
}

/// The Direct Sound PCM buffer: a ring of frames, right and left (the
/// driver's `pcmBuffer`, which the FIFOs play from), in [`BYTE_STEP`]ths
/// of a byte's step: the game's mix holds whole bytes in it (its samples
/// are eight bits), the HQ mix its samples at its precision.
#[derive(Clone, Debug)]
pub(crate) struct PcmRing {
    pub right: Vec<i16>,
    pub left: Vec<i16>,
    pub samples_per_frame: usize,
    /// Frames the ring holds (`pcmDmaPeriod`).
    pub frames: usize,
}

/// The driver's PCM buffer, in samples a side.
const PCM_DMA_BUF_SIZE: usize = 1584;

/// A byte's step of the PCM buffer (and the FIFOs), in the units the ring
/// holds its samples in: the HQ mix's precision, seven bits below the
/// eight a game's sample has.
pub const BYTE_STEP: i16 = 128;

impl PcmRing {
    pub fn new(samples_per_frame: usize) -> PcmRing {
        let frames = (PCM_DMA_BUF_SIZE / samples_per_frame).max(1);
        PcmRing { right: vec![0; PCM_DMA_BUF_SIZE], left: vec![0; PCM_DMA_BUF_SIZE], samples_per_frame, frames }
    }
}

/// The byte a sample of the ring is in the game's buffer (the HQ mix's
/// sample rounded down).
fn byte(v: i16) -> i8 {
    v.div_euclid(BYTE_STEP) as i8
}

/// A byte of the game's buffer as a sample of the ring.
fn from_byte(b: i8) -> i16 {
    b as i16 * BYTE_STEP
}

// ---- SoundMainRAM: Direct Sound --------------------------------------------------

/// The reverb into, or clearing of, frame slot `slot`, from the slot
/// itself (mixed a ring ago) and `next` (mixed a ring ago less a frame).
pub(crate) fn reverb_or_clear(ring: &mut PcmRing, slot: usize, next: usize, reverb: u8) {
    let n = ring.samples_per_frame;
    let (cur, nxt) = (slot * n, next * n);
    if reverb == 0 {
        ring.right[cur..cur + n].fill(0);
        ring.left[cur..cur + n].fill(0);
        return;
    }
    let b = |v: i16| byte(v) as i32;
    for k in 0..n {
        let sum = b(ring.left[cur + k]) + b(ring.right[cur + k]) + b(ring.left[nxt + k]) + b(ring.right[nxt + k]);
        let mut v = (sum * reverb as i32) >> 9;
        // (A negative result rounds toward zero, but one less than a
        // multiple of 512 rounds to zero too.)
        if v & 0x80 != 0 {
            v += 1;
        }
        ring.right[cur + k] = from_byte(v as i8);
        ring.left[cur + k] = from_byte(v as i8);
    }
}

/// A Direct Sound channel's envelope for a frame, and its volumes for the
/// mix (`mix_right`, `mix_left`): the sample it plays, or none if it is
/// silent or stops instead.
fn envelope<'a>(ch: &mut DsChannel, bank: &'a SoundBank, master: u8) -> Option<&'a Sample> {
    if !ch.status.active() {
        return None;
    }
    let sample = &bank.samples[ch.sample.0 as usize];
    let e = ch.note.envelope;
    let mut env: u32;
    if ch.status.start {
        if ch.status.stop {
            ch.status = Status::off();
            return None;
        }
        ch.status = Status { phase: Phase::Attack, looping: sample.loop_start.is_some(), ..Status::off() };
        ch.position = 0;
        ch.remaining = sample.data.len() as i32;
        ch.fraction = 0;
        env = e.attack as u32;
        if env >= 0xFF {
            env = 0xFF;
            ch.status.phase = Phase::Decay;
        }
    } else {
        env = ch.note.envelope_volume as u32;
        if ch.status.echo {
            let was = ch.note.echo_length;
            ch.note.echo_length = was.wrapping_sub(1);
            if was <= 1 {
                ch.status = Status::off();
                return None;
            }
        } else if ch.status.stop {
            env = (env * e.release as u32) >> 8;
            if env <= ch.note.echo_volume as u32 {
                env = ch.note.echo_volume as u32;
                if env == 0 {
                    ch.status = Status::off();
                    return None;
                }
                ch.status.echo = true;
            }
        } else {
            match ch.status.phase {
                Phase::Decay => {
                    env = (env * e.decay as u32) >> 8;
                    if env <= e.sustain as u32 {
                        env = e.sustain as u32;
                        if env == 0 {
                            env = ch.note.echo_volume as u32;
                            if env == 0 {
                                ch.status = Status::off();
                                return None;
                            }
                            ch.status.echo = true;
                        } else {
                            ch.status.phase = Phase::Sustain;
                        }
                    }
                }
                Phase::Attack => {
                    env += e.attack as u32;
                    if env >= 0xFF {
                        env = 0xFF;
                        ch.status.phase = Phase::Decay;
                    }
                }
                _ => {}
            }
        }
    }
    ch.note.envelope_volume = env as u8;
    let level = (env * (master as u32 + 1)) >> 4;
    ch.mix_right = ((ch.note.right_volume as u32 * level) >> 8) as u8;
    ch.mix_left = ((ch.note.left_volume as u32 * level) >> 8) as u8;
    Some(sample)
}

/// A Direct Sound channel's samples for a frame of `n`, from where it is,
/// moving on by `step(k)` after output sample `k` (in 2^23ths of a sample
/// of the data; a fixed-frequency channel moves on by one): `out(k, s, d,
/// left)` for output sample `k`, with `s` the sample of the data it is on,
/// `d` the way to the next one it has gone, times 2^23 (the fraction past
/// `s` times the step to the next: the driver interpolates linearly
/// between the two; 0 at a fixed frequency), and `left` the samples of the
/// data left from `s` to its end. It stops at the end of data that doesn't
/// loop.
fn walk(ch: &mut DsChannel, sample: &Sample, step: impl Fn(usize) -> u32, n: usize, mut out: impl FnMut(usize, i32, i32, i32)) {
    let data = &sample.data;
    let at = |i: u32| -> i32 { data.get(i as usize).copied().unwrap_or(sample.tail) as i32 };
    let loop_start = if ch.status.looping { sample.loop_start } else { None };
    let loop_len = loop_start.map(|l| data.len() as i32 - l as i32);
    if ch.fixed {
        // One sample of the data per output sample.
        let mut pos = ch.position;
        let mut remaining = ch.remaining;
        for k in 0..n {
            out(k, at(pos), 0, remaining);
            pos += 1;
            remaining -= 1;
            if remaining <= 0 {
                match (loop_start, loop_len) {
                    (Some(l), Some(len)) => {
                        pos = l;
                        remaining = len;
                    }
                    _ => {
                        ch.status = Status::off();
                        return;
                    }
                }
            }
        }
        ch.position = pos;
        ch.remaining = remaining;
        return;
    }
    // Linear interpolation, the position in 23-bit fixed point.
    let mut frac = ch.fraction;
    let mut remaining = ch.remaining;
    let mut pos = ch.position;
    let mut cur = at(pos);
    let mut delta = at(pos + 1) - cur;
    for k in 0..n {
        out(k, cur, (frac as i32).wrapping_mul(delta), remaining);
        frac = frac.wrapping_add(step(k));
        let advance = frac >> 23;
        if advance == 0 {
            continue;
        }
        frac &= !0x3F80_0000;
        remaining -= advance as i32;
        if remaining <= 0 {
            let (Some(l), Some(len)) = (loop_start, loop_len) else {
                ch.status = Status::off();
                return;
            };
            let mut over = -remaining;
            loop {
                remaining += len;
                if remaining > 0 {
                    break;
                }
                over -= len;
            }
            pos = (l as i32 + over) as u32;
            cur = at(pos);
        } else if advance == 1 {
            pos += 1;
            cur += delta;
        } else {
            pos += advance;
            cur = at(pos);
        }
        delta = at(pos + 1) - cur;
    }
    ch.fraction = frac;
    ch.remaining = remaining;
    ch.position = pos;
}

/// One Direct Sound channel for a frame: its envelope, then its mix into
/// frame slot `slot` of the ring, at the host's level for its player
/// (`host`: 256 is as the driver mixes it, to the bit).
pub(crate) fn mix_ds(ch: &mut DsChannel, bank: &SoundBank, master: u8, div_freq: u32, ring: &mut PcmRing, slot: usize, host: u16) {
    let Some(sample) = envelope(ch, bank, master) else {
        return;
    };
    let n = ring.samples_per_frame;
    let base = slot * n;
    let (vr, vl) = ((ch.mix_right as i32 * host as i32) >> 8, (ch.mix_left as i32 * host as i32) >> 8);
    let add = |v: &mut i16, x: i32| *v = from_byte(byte(*v).wrapping_add(x as i8));
    let step = div_freq.wrapping_mul(ch.note.frequency);
    walk(ch, sample, |_| step, n, |k, s, d, _| {
        let s = s + (d >> 23);
        add(&mut ring.right[base + k], (vr * s) >> 8);
        add(&mut ring.left[base + k], (vl * s) >> 8);
    });
}

// ---- The HQ mixer ----------------------------------------------------------------

/// How the Direct Sound channels go into the PCM buffer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mixer {
    /// The driver's own `SoundMainRAM`: each channel rounded to eight bits
    /// and added into the buffer in turn, wrapping, and the reverb rounded
    /// too (the game's buffer, to the byte: what verification compares with
    /// the original).
    Original,
    /// After ipatix's HQ mixer (gba-hq-mixer, MIT): the reverb and every
    /// channel added up at a higher precision (seven bits below the
    /// buffer's eight, with the volumes and the interpolated sample
    /// unrounded) and the sum clamped, not wrapped; then played at that
    /// precision, never rounded to eight bits (which the GBA's FIFOs would
    /// need). The driver sets each channel's volume and pitch once a frame:
    /// here they glide across the frame from the last frame's to this
    /// one's (the mix reads each note on a copy of its channel, so the
    /// driver's own moves on as the game's does); a note the driver cuts
    /// off (another note taking its channel, a stop, the end of its echo)
    /// fades out over [`CUT_FADE`] samples instead of stopping at once, and
    /// a sample that ends fades out over its last [`END_FADE`]; a note
    /// starts at its volume and pitch (its sample's own attack). The rest
    /// stays the game's: the envelopes, the interpolation, the volumes and
    /// pitches each frame reaches, the reverb's level and timing, and where
    /// each channel is in its sample, so the channels are the same either
    /// way. None of the HQ mixer's 16-bit register shortcuts are kept
    /// (halved volumes, the reverb level in steps of four, both sides
    /// packed in one word).
    #[default]
    Hq,
}

/// Samples a note the driver cuts off takes to fade out with [`Mixer::Hq`]
/// (6 ms at EXE6's 10512 Hz).
pub const CUT_FADE: u32 = 64;

/// The samples at the end of a sample that doesn't loop that fade out with
/// [`Mixer::Hq`] (1.5 ms at 10512 Hz: data that stops short of zero would
/// click).
pub const END_FADE: u32 = 16;

/// The HQ mix's volume, right and left, of a channel whose envelope has
/// run this frame: the driver's ((volume * ((envelope * (master + 1)) >>
/// 4)) >> 8) * host >> 8, with nothing rounded off (full volume is 2^28).
fn hq_volume(ch: &DsChannel, master: u8, host: u16) -> [i64; 2] {
    let level = ch.note.envelope_volume as i64 * (master as i64 + 1) * host as i64;
    [level * ch.note.right_volume as i64, level * ch.note.left_volume as i64]
}

/// `n` samples of a channel added into `mix`, moving on by `step(k)` (as
/// [`walk`]), at the volume `volume(k)` gives for sample `k` (as
/// [`hq_volume`]); the last [`END_FADE`] before the end of data that
/// doesn't loop fading out.
fn hq_add(mix: &mut [[i32; 2]], ch: &mut DsChannel, sample: &Sample, step: impl Fn(usize) -> u32, n: usize, volume: impl Fn(usize) -> [i64; 2]) {
    // (The sample is times 2^23 and the volume times 2^28; a byte's step is
    // BYTE_STEP of the mix's.)
    let shift = 23 + 28 - BYTE_STEP.trailing_zeros();
    let ends = !(ch.status.looping && sample.loop_start.is_some());
    let fixed = ch.fixed;
    walk(ch, sample, &step, n, |k, s, d, left| {
        let s = ((s as i64) << 23) + d as i64;
        let mut v = volume(k);
        if ends {
            // (Output samples to the end, about.)
            let to_end = if fixed { left as i64 } else { ((left as i64) << 23) / step(k).max(1) as i64 };
            if to_end < END_FADE as i64 {
                v = v.map(|x| x * to_end.max(0) / END_FADE as i64);
            }
        }
        mix[k][0] += ((v[0] * s) >> shift) as i32;
        mix[k][1] += ((v[1] * s) >> shift) as i32;
    });
}

/// A channel as the HQ mix last played it: the copy of it the mix reads
/// its note on (the driver's moves on at the game's pitch, this one's
/// glides), and the volumes and step it reached.
#[derive(Clone, Debug)]
struct Played {
    ch: DsChannel,
    volume: [i64; 2],
    step: u32,
}

/// A note the driver cut off, fading out: the mix's copy of its channel,
/// the volumes and step it fades from, and the samples of its fade played
/// so far.
#[derive(Clone, Debug)]
struct Fading {
    ch: DsChannel,
    volume: [i64; 2],
    step: u32,
    done: u32,
}

/// The HQ mixer's state: the frame being mixed, and what each channel
/// played last.
#[derive(Clone, Debug)]
pub(crate) struct HqMix {
    /// The frame, (right, left) a sample, in the ring's units
    /// ([`BYTE_STEP`]ths of a byte's step).
    mix: Vec<[i32; 2]>,
    /// Each channel as the last frame played it (none: silent, or its data
    /// ended): where its volume and pitch glide from, and what fades out if
    /// the driver cuts it off.
    last: Vec<Option<Played>>,
    fading: Vec<Fading>,
}

impl HqMix {
    pub fn new(samples_per_frame: usize, channels: usize) -> HqMix {
        HqMix { mix: vec![[0; 2]; samples_per_frame], last: vec![None; channels], fading: Vec::new() }
    }

    /// The frame's start: the ring's reverb at its precision (from the slot
    /// itself and `next`, as [`reverb_or_clear`] takes it), or silence.
    pub fn reverb_or_clear(&mut self, ring: &PcmRing, slot: usize, next: usize, reverb: u8) {
        let n = ring.samples_per_frame;
        let (cur, nxt) = (slot * n, next * n);
        for (k, m) in self.mix.iter_mut().enumerate() {
            let sum = ring.left[cur + k] as i32 + ring.right[cur + k] as i32 + ring.left[nxt + k] as i32 + ring.right[nxt + k] as i32;
            // The driver's (sum * reverb) >> 9.
            let v = (sum * reverb as i32) >> 9;
            *m = [v, v];
        }
    }

    /// Channel `i` for the frame: its envelope (as [`mix_ds`]), then its
    /// mix, its volumes and pitch gliding from the last frame's; the note it
    /// played last frame fading out if the driver cut it off.
    pub fn channel(&mut self, i: usize, ch: &mut DsChannel, bank: &SoundBank, master: u8, div_freq: u32, host: u16) {
        let was = self.last[i].take();
        let new_note = ch.status.start;
        let Some(sample) = envelope(ch, bank, master) else {
            if let Some(p) = was {
                self.fade(p);
            }
            return;
        };
        let to = hq_volume(ch, master, host);
        let step = div_freq.wrapping_mul(ch.note.frequency);
        let (mut read, from, from_step) = match was {
            Some(p) if !new_note => (p.ch, p.volume, p.step),
            was => {
                // A new note starts where the driver starts it, at its volume
                // and pitch; one it took the channel from fades out.
                if let Some(p) = was {
                    self.fade(p);
                }
                (ch.clone(), to, step)
            }
        };
        let n = self.mix.len();
        // The driver's channel moves on as the game's does.
        walk(ch, sample, |_| step, n, |_, _, _, _| {});
        // (The mix's copy may end a few samples before or after it: the
        // difference its glides made, bounded by the pitch's change since
        // the note started.)
        let n64 = n as i64;
        let glide = |k: usize| -> [i64; 2] { std::array::from_fn(|c| from[c] + (to[c] - from[c]) * (k as i64 + 1) / n64) };
        let pitch = |k: usize| -> u32 { (from_step as i64 + (step as i64 - from_step as i64) * (k as i64 + 1) / n64) as u32 };
        if read.status.active() {
            hq_add(&mut self.mix, &mut read, sample, pitch, n, glide);
        }
        if ch.status.active() || read.status.active() {
            self.last[i] = Some(Played { ch: read, volume: to, step });
        }
    }

    fn fade(&mut self, p: Played) {
        if p.volume != [0; 2] && p.ch.status.active() {
            self.fading.push(Fading { ch: p.ch, volume: p.volume, step: p.step, done: 0 });
        }
    }

    /// The notes fading out, each for the rest of its fade or of the frame.
    pub fn fades(&mut self, bank: &SoundBank) {
        let mix = &mut self.mix;
        self.fading.retain_mut(|f| {
            let n = mix.len().min((CUT_FADE - f.done) as usize);
            let sample = &bank.samples[f.ch.sample.0 as usize];
            let (from, done, step) = (f.volume, f.done as i64, f.step);
            let fade = |k: usize| -> [i64; 2] { from.map(|v| v * (CUT_FADE as i64 - done - k as i64 - 1) / CUT_FADE as i64) };
            hq_add(mix, &mut f.ch, sample, |_| step, n, fade);
            f.done += n as u32;
            f.done < CUT_FADE && f.ch.status.active()
        });
    }

    /// The frame into frame slot `slot` of the ring, each side clamped to
    /// the buffer's range.
    pub fn store(&self, ring: &mut PcmRing, slot: usize) {
        let base = slot * ring.samples_per_frame;
        let clamp = |v: i32| v.clamp(-128 * BYTE_STEP as i32, 128 * BYTE_STEP as i32 - 1) as i16;
        for (k, m) in self.mix.iter().enumerate() {
            ring.right[base + k] = clamp(m[0]);
            ring.left[base + k] = clamp(m[1]);
        }
    }
}

/// A sample's playback rate for a note (`MidiKeyToFreq` of its wave).
pub(crate) fn ds_frequency(sample: &Sample, key: u8, fine: u8) -> u32 {
    tables::midi_key_to_freq(sample.rate, key, fine)
}

// ---- CgbSound: the PSG -----------------------------------------------------------

/// The registers a PSG channel drives: (NRx0, NRx1, NRx2, NRx3, NRx4).
fn registers(n: usize) -> (Reg, Reg, Reg, Reg, Reg) {
    match n {
        1 => (Reg::Nr10, Reg::Nr11, Reg::Nr12, Reg::Nr13, Reg::Nr14),
        2 => (Reg::Nr10, Reg::Nr21, Reg::Nr22, Reg::Nr23, Reg::Nr24),
        3 => (Reg::Nr30, Reg::Nr31, Reg::Nr32, Reg::Nr33, Reg::Nr34),
        _ => (Reg::Nr10, Reg::Nr41, Reg::Nr42, Reg::Nr43, Reg::Nr44),
    }
}

/// `CgbOscOff`: silence PSG channel `n` at once.
pub(crate) fn cgb_off(apu: &mut Apu, n: usize) {
    match n {
        1 => {
            apu.write(Reg::Nr12, 8);
            apu.write(Reg::Nr14, 0x80);
        }
        2 => {
            apu.write(Reg::Nr22, 8);
            apu.write(Reg::Nr24, 0x80);
        }
        3 => apu.write(Reg::Nr30, 0),
        _ => {
            apu.write(Reg::Nr42, 8);
            apu.write(Reg::Nr44, 0x80);
        }
    }
}

/// `CgbModVol`: the pan bits and the envelope's goals from the volumes.
fn cgb_mod_volume(ch: &mut CgbChannel) {
    let (r, l) = (ch.note.right_volume as u32, ch.note.left_volume as u32);
    let one_sided = if r >= l {
        (r >> 1 >= l).then_some(0x0F)
    } else {
        (l >> 1 >= r).then_some(0xF0)
    };
    match one_sided {
        Some(pan) => {
            ch.pan = pan;
            ch.envelope_goal = ((r + l) >> 4).min(15) as u8;
        }
        None => {
            // (Centered notes aren't capped at 15.)
            ch.pan = 0xFF;
            ch.envelope_goal = ((r + l) >> 4) as u8;
        }
    }
    ch.sustain_goal = ((ch.envelope_goal as u32 * ch.note.envelope.sustain as u32 + 15) >> 4) as u8;
    ch.pan &= ch.pan_mask;
}

/// `CgbSound`: every PSG channel's frame. `tick` is the driver's 15-frame
/// counter (SoundInfo's c15): when it is 0, each envelope counts down twice.
pub(crate) fn cgb_sound(channels: &mut [CgbChannel; 4], apu: &mut Apu, bank: &SoundBank, tick: &mut u8, dac_resolution: u8) {
    if *tick != 0 {
        *tick -= 1;
    } else {
        *tick = 14;
    }
    for (i, ch) in channels.iter_mut().enumerate() {
        if ch.status.active() {
            cgb_channel(ch, i + 1, apu, bank, *tick == 0, dac_resolution);
        }
    }
}

/// Where `CgbSound` goes next for a channel (its labels).
enum At {
    /// `loc_814F768`: step the envelope if its counter ran out.
    CheckCounter,
    /// `loc_814F79A`: released to the end; the echo, or silence.
    EchoOrOff,
    /// `loc_814F7F2`: the decay reached the sustain level.
    DecayDone,
    /// `loc_814F82E`: the attack reached its goal (or there was none).
    AttackDone,
    /// `loc_814F85A`: the counter starts over.
    SetCounter(u8),
    /// `loc_814F85C`: the counter counts.
    CountDown,
    /// `loc_814F86E`: the register writes.
    Write,
    /// `loc_814F714`: the channel ends.
    Off,
}

fn cgb_channel(ch: &mut CgbChannel, n: usize, apu: &mut Apu, bank: &SoundBank, mut extra_count: bool, dac_resolution: u8) {
    let (r0, r1, r2, r3, r4) = registers(n);
    // NRx2's direction and period bits as written last, unless changed.
    let mut env_bits = apu.read(r2);
    let e = ch.note.envelope;
    let mut at = if ch.status.start {
        if ch.status.stop {
            At::Off
        } else {
            ch.status = Status { phase: Phase::Attack, ..Status::off() };
            ch.volume_changed = true;
            ch.pitch_changed = true;
            cgb_mod_volume(ch);
            match ch.voice {
                VoiceKind::Wave { wave, .. } => {
                    if ch.loaded_wave != Some(wave) {
                        apu.write(r0, 0x40);
                        let w = &bank.waves[wave.0 as usize].0;
                        let bytes: [u8; 16] = std::array::from_fn(|j| (w[2 * j] << 4) | (w[2 * j + 1] & 15));
                        apu.write_wave_ram(&bytes);
                        ch.loaded_wave = Some(wave);
                    }
                    apu.write(r0, 0);
                    apu.write(r1, ch.length);
                    ch.nrx4 = if ch.length != 0 { 0xC0 } else { 0x80 };
                }
                voice => {
                    match voice {
                        VoiceKind::Noise { narrow } => {
                            apu.write(r1, ch.length);
                            apu.write(r3, (narrow as u8) << 3);
                        }
                        VoiceKind::Square1 { duty, .. } | VoiceKind::Square2 { duty, .. } => {
                            if n == 1 {
                                apu.write(r0, ch.sweep);
                            }
                            apu.write(r1, (duty << 6).wrapping_add(ch.length));
                        }
                        _ => {}
                    }
                    env_bits = e.attack.wrapping_add(8);
                    ch.nrx4 = if ch.length != 0 { 0x40 } else { 0 };
                }
            }
            ch.envelope_counter = e.attack;
            if e.attack != 0 {
                ch.note.envelope_volume = 0;
                At::CountDown
            } else {
                At::AttackDone
            }
        }
    } else if ch.status.echo || (apu.read(Reg::Nr52) >> (n - 1)) & 1 == 0 {
        // The echo counts down; so does a channel the hardware stopped
        // (with no echo it ends now).
        ch.note.echo_length = ch.note.echo_length.wrapping_sub(1);
        if (ch.note.echo_length as i8) <= 0 { At::Off } else { At::Write }
    } else if ch.status.stop && ch.status.phase != Phase::Release {
        ch.status.phase = Phase::Release;
        ch.envelope_counter = e.release;
        if e.release != 0 {
            ch.volume_changed = true;
            if n != 3 {
                env_bits = e.release;
            }
            At::CountDown
        } else {
            At::EchoOrOff
        }
    } else {
        At::CheckCounter
    };
    loop {
        at = match at {
            At::CheckCounter => {
                if ch.envelope_counter != 0 {
                    At::CountDown
                } else {
                    if n == 3 {
                        ch.volume_changed = true;
                    }
                    cgb_mod_volume(ch);
                    let v = &mut ch.note.envelope_volume;
                    match ch.status.phase {
                        Phase::Release => {
                            *v = v.wrapping_sub(1);
                            if (*v as i8) > 0 { At::SetCounter(e.release) } else { At::EchoOrOff }
                        }
                        Phase::Sustain => {
                            *v = ch.sustain_goal;
                            At::SetCounter(7)
                        }
                        Phase::Decay => {
                            *v = v.wrapping_sub(1);
                            if (*v as i8) > (ch.sustain_goal as i8) { At::SetCounter(e.decay) } else { At::DecayDone }
                        }
                        Phase::Attack => {
                            *v = v.wrapping_add(1);
                            if *v < ch.envelope_goal { At::SetCounter(e.attack) } else { At::AttackDone }
                        }
                    }
                }
            }
            At::EchoOrOff => {
                let v = ((ch.envelope_goal as u32 * ch.note.echo_volume as u32 + 0xFF) >> 8) as u8;
                ch.note.envelope_volume = v;
                if v == 0 {
                    At::Off
                } else {
                    ch.status.echo = true;
                    ch.volume_changed = true;
                    if n != 3 {
                        env_bits = 8;
                    }
                    At::Write
                }
            }
            At::DecayDone => {
                if e.sustain == 0 {
                    ch.status.phase = Phase::Release;
                    At::EchoOrOff
                } else {
                    ch.status.phase = Phase::Sustain;
                    ch.volume_changed = true;
                    if n != 3 {
                        env_bits = 8;
                    }
                    ch.note.envelope_volume = ch.sustain_goal;
                    At::SetCounter(7)
                }
            }
            At::AttackDone => {
                ch.status.phase = ch.status.phase.next();
                ch.envelope_counter = e.decay;
                if e.decay == 0 {
                    At::DecayDone
                } else {
                    ch.volume_changed = true;
                    ch.note.envelope_volume = ch.envelope_goal;
                    if n != 3 {
                        env_bits = e.decay;
                    }
                    At::CountDown
                }
            }
            At::SetCounter(v) => {
                ch.envelope_counter = v;
                At::CountDown
            }
            At::CountDown => {
                ch.envelope_counter = ch.envelope_counter.wrapping_sub(1);
                if extra_count {
                    extra_count = false;
                    At::CheckCounter
                } else {
                    At::Write
                }
            }
            At::Write => {
                write_registers(ch, apu, n, (r0, r2, r3, r4), env_bits, dac_resolution);
                break;
            }
            At::Off => {
                cgb_off(apu, n);
                ch.status = Status::off();
                break;
            }
        };
    }
    ch.volume_changed = false;
    ch.pitch_changed = false;
}

/// The frame's register writes for a channel whose pitch or volume changed.
fn write_registers(ch: &mut CgbChannel, apu: &mut Apu, n: usize, regs: (Reg, Reg, Reg, Reg), env_bits: u8, dac_resolution: u8) {
    let (r0, r2, r3, r4) = regs;
    if ch.pitch_changed {
        let fixed = match ch.voice {
            VoiceKind::Square1 { fixed, .. } | VoiceKind::Square2 { fixed, .. } | VoiceKind::Wave { fixed, .. } => fixed,
            _ => false,
        };
        if n <= 3 && fixed {
            // Rounded to what the DAC's resolution plays exactly.
            match dac_resolution {
                0 => ch.note.frequency = (ch.note.frequency + 2) & 0x7FC,
                1 => ch.note.frequency = (ch.note.frequency + 1) & 0x7FE,
                _ => {}
            }
        }
        let low = if n == 4 { (apu.read(r3) & 8) | ch.note.frequency as u8 } else { ch.note.frequency as u8 };
        apu.write(r3, low);
        ch.nrx4 = (ch.nrx4 & 0xC0).wrapping_add(((ch.note.frequency & 0x3F00) >> 8) as u8);
        apu.write(r4, ch.nrx4);
    }
    if ch.volume_changed {
        let nr51 = (apu.read(Reg::Nr51) & !ch.pan_mask) | ch.pan;
        apu.write(Reg::Nr51, nr51);
        if n == 3 {
            // (Levels past 15, which a centered note can reach, read the
            // table after gCgb3Vol: the clock table.)
            let v = ch.note.envelope_volume as usize;
            let level = if v < 16 { tables::WAVE_VOLUME[v] } else { tables::CLOCK[v - 16] };
            apu.write(r2, level);
            if ch.nrx4 & 0x80 != 0 {
                apu.write(r0, 0x80);
                apu.write(r4, ch.nrx4);
                ch.nrx4 &= 0x7F;
            }
        } else {
            apu.write(r2, (ch.note.envelope_volume << 4).wrapping_add(env_bits & 15));
            apu.write(r4, ch.nrx4 | 0x80);
            if n == 1 && apu.read(Reg::Nr10) & 8 == 0 {
                apu.write(r4, ch.nrx4 | 0x80);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hq_mix_keeps_its_precision_and_clamps() {
        let n = 176;
        let mut ring = PcmRing::new(n);
        let mut hq = HqMix::new(n, 4);
        for m in hq.mix.iter_mut() {
            *m = [300, -90];
        }
        hq.store(&mut ring, 0);
        assert!(ring.right[..n].iter().all(|&v| v == 300) && ring.left[..n].iter().all(|&v| v == -90));
        for m in hq.mix.iter_mut() {
            *m = [3 * 127 * BYTE_STEP as i32, -3 * 128 * BYTE_STEP as i32];
        }
        hq.store(&mut ring, 1);
        assert!(ring.right[n..2 * n].iter().all(|&v| v == 128 * BYTE_STEP - 1));
        assert!(ring.left[n..2 * n].iter().all(|&v| v == -128 * BYTE_STEP));
    }

    #[test]
    fn the_games_bytes_are_whole_steps_of_the_ring() {
        for b in [-128i8, -1, 0, 1, 127] {
            assert_eq!(byte(from_byte(b)), b);
        }
        assert_eq!((byte(-1), byte(127)), (-1, 0), "an HQ sample's byte is it rounded down");
    }
}
