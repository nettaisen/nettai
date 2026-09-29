//! The hardware channels and the mixer: the M4A envelope for Direct Sound
//! and the 16-level CGB envelope for PSG voices, pseudo-echo, sample
//! resampling into the 8-bit Direct Sound mix buffer with its ring-buffer
//! reverb, PSG synthesis, and the output at 32768 Hz. Ported from
//! m4a-engine's `render.rs`; operations follow m4a.py's order in f64.

use crate::bank::{MixerConfig, SoundBank, Voice, VoiceKind};
use crate::{FPS, OUT_RATE};

/// The driver's PCM DMA buffer, in samples (it sets the reverb delay).
const PCM_DMA_BUF_SIZE: usize = 1584;
/// NR43 (noise clock) for keys 21 and up.
const NOISE_NR43: [u8; 60] = [
    0xD7, 0xD6, 0xD5, 0xD4, 0xC7, 0xC6, 0xC5, 0xC4, 0xB7, 0xB6, 0xB5, 0xB4, 0xA7, 0xA6, 0xA5, 0xA4, 0x97, 0x96, 0x95,
    0x94, 0x87, 0x86, 0x85, 0x84, 0x77, 0x76, 0x75, 0x74, 0x67, 0x66, 0x65, 0x64, 0x57, 0x56, 0x55, 0x54, 0x47, 0x46,
    0x45, 0x44, 0x37, 0x36, 0x35, 0x34, 0x27, 0x26, 0x25, 0x24, 0x17, 0x16, 0x15, 0x14, 0x07, 0x06, 0x05, 0x04, 0x03,
    0x02, 0x01, 0x00,
];
/// Wave-channel output level per envelope level.
const WAVE_LEVEL: [f64; 16] = [0.0, 0.0, 0.25, 0.25, 0.25, 0.25, 0.5, 0.5, 0.5, 0.5, 0.75, 0.75, 0.75, 0.75, 1.0, 1.0];
const DUTY: [f64; 4] = [0.125, 0.25, 0.5, 0.75];
/// PSG level -> 10-bit units (Direct Sound is x4).
const PSG_SCALE: f64 = 16.0;

fn lfsr(width: u32) -> Vec<f64> {
    let mut reg: u32 = (1 << width) - 1;
    let mut out = Vec::with_capacity((1 << width) - 1);
    for _ in 0..(1u32 << width) - 1 {
        let bit = (reg ^ (reg >> 1)) & 1;
        reg = (reg >> 1) | (bit << (width - 1));
        out.push((!reg & 1) as f64);
    }
    out
}

/// A track of a music player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TrackRef {
    pub player: u8,
    pub track: u8,
}

/// Where a channel's track sorts when channels of equal priority compete
/// (the driver compares track addresses; a channel its track let go has
/// none and sorts first).
pub(crate) type TrackOrder = Option<(u8, u8)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Start,
    Attack,
    Decay,
    Sustain,
    Release,
    Echo,
}

/// A hardware channel playing one note.
#[derive(Clone, Debug)]
pub(crate) struct Channel {
    /// The track whose note it plays, until the track lets it go.
    pub owner: Option<TrackRef>,
    pub order: TrackOrder,
    pub priority: u8,
    pub voice: Voice,
    /// The key the note named (what EOT matches).
    pub midi_key: u8,
    pub velocity: u8,
    pub rhythm_pan: i32,
    /// Ticks until release; 0 holds (a tie).
    pub gate: u8,
    pub echo_volume: u8,
    pub echo_length: u8,
    /// Right and left volume (0..=255), from the track.
    pub right: i64,
    pub left: i64,
    /// Key with the track's shifts, and the fraction above it (1/256).
    pub key: i32,
    pub pitch: u8,
    /// Sounding (the driver's status flags are non-zero).
    pub on: bool,
    /// Released (the driver's STOP flag).
    pub stopping: bool,
    stage: Stage,
    env: i64,
    level: i64,
    counter: i64,
    echo_left: i64,
    pos: f64,
    phase: f64,
    noise_at: f64,
}

impl Channel {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        owner: TrackRef,
        order: (u8, u8),
        priority: u8,
        voice: Voice,
        midi_key: u8,
        velocity: u8,
        rhythm_pan: i32,
        gate: u8,
        echo: (u8, u8),
    ) -> Channel {
        Channel {
            owner: Some(owner),
            order: Some(order),
            priority,
            voice,
            midi_key,
            velocity,
            rhythm_pan,
            gate,
            echo_volume: echo.0,
            echo_length: echo.1,
            right: 0,
            left: 0,
            key: 0,
            pitch: 0,
            on: true,
            stopping: false,
            stage: Stage::Start,
            env: 0,
            level: 0,
            counter: 0,
            echo_left: 0,
            pos: 0.0,
            phase: 0.0,
            noise_at: 0.0,
        }
    }

    /// Semitones above C4-relative key 0 the note sounds at, with the fraction.
    fn semis(&self) -> f64 {
        self.key as f64 + self.pitch as f64 / 256.0
    }
}

/// Stereo samples addressed by absolute index, with an offset so old ones
/// can be dropped.
#[derive(Default)]
struct Buf {
    data: Vec<[f64; 2]>,
    off: usize,
}

impl Buf {
    fn ensure(&mut self, end: usize) {
        if end > self.off + self.data.len() {
            self.data.resize(end - self.off, [0.0; 2]);
        }
    }
    fn get(&self, i: usize) -> [f64; 2] {
        if i < self.off { [0.0; 2] } else { self.data.get(i - self.off).copied().unwrap_or([0.0; 2]) }
    }
    fn set(&mut self, i: usize, v: [f64; 2]) {
        self.ensure(i + 1);
        self.data[i - self.off] = v;
    }
    fn add(&mut self, i: usize, v: [f64; 2]) {
        self.ensure(i + 1);
        let x = &mut self.data[i - self.off];
        x[0] += v[0];
        x[1] += v[1];
    }
    /// Drop everything before absolute index `keep_from`.
    fn trim(&mut self, keep_from: usize) {
        if keep_from > self.off {
            let n = (keep_from - self.off).min(self.data.len());
            self.data.drain(..n);
            self.off += n;
        }
    }
}

pub(crate) struct Mixer {
    /// Direct Sound channels.
    pub ds: Vec<Option<Channel>>,
    /// PSG channels: square 1, square 2, wave, noise.
    pub psg: [Option<Channel>; 4],
    /// Reverb level (0 = off).
    pub reverb: u8,
    master: i64,
    mr: f64,
    per_frame: f64,
    taps: (usize, usize),
    noise: [Vec<f64>; 2],
    ds_buf: Buf,
    psg_buf: Buf,
    frame: usize,
    /// Output samples handed out so far.
    produced: usize,
}

impl Mixer {
    pub fn new(cfg: &MixerConfig) -> Mixer {
        let mr = cfg.mix_rate as f64;
        let per_frame = mr / FPS;
        let spf = per_frame.round_ties_even() as usize;
        let ring = PCM_DMA_BUF_SIZE / spf.max(1);
        Mixer {
            ds: vec![None; cfg.ds_channels as usize],
            psg: [None, None, None, None],
            reverb: cfg.reverb,
            master: cfg.master_volume as i64,
            mr,
            per_frame,
            taps: ((ring - 1) * spf, ring * spf),
            noise: [lfsr(15), lfsr(7)],
            ds_buf: Buf::default(),
            psg_buf: Buf::default(),
            frame: 0,
            produced: 0,
        }
    }

    /// Mix one frame: Direct Sound at the mixing rate, PSG at the output rate.
    pub fn mix_frame(&mut self, bank: &SoundBank) {
        let f = self.frame;
        let a = (f as f64 * self.per_frame) as usize;
        let b = ((f + 1) as f64 * self.per_frame) as usize;
        let n = b - a;
        let mut mix = vec![[0.0f64; 2]; n];
        if self.reverb != 0 && a >= self.taps.1 {
            for (k, m) in mix.iter_mut().enumerate() {
                let p = self.ds_buf.get(a + k - self.taps.0);
                let q = self.ds_buf.get(a + k - self.taps.1);
                let fb = (((p[0] + p[1]) + (q[0] + q[1])) * self.reverb as f64 / 512.0).trunc();
                m[0] += fb;
                m[1] += fb;
            }
        }
        for slot in self.ds.iter_mut() {
            let Some(ch) = slot.as_mut() else { continue };
            if !ch.on {
                continue;
            }
            ds_env(ch);
            if !ch.on {
                continue;
            }
            let VoiceKind::DirectSound { sample, fixed } = ch.voice.kind else {
                ch.on = false;
                continue;
            };
            let y = mix_ds(ch, &bank.samples[sample.0 as usize], fixed, n, self.master, self.mr);
            for (m, v) in mix.iter_mut().zip(&y) {
                m[0] += v[0];
                m[1] += v[1];
            }
        }
        for (k, m) in mix.iter().enumerate() {
            self.ds_buf.set(a + k, [m[0].clamp(-128.0, 127.0), m[1].clamp(-128.0, 127.0)]);
        }

        let a = ((f * OUT_RATE as usize) as f64 / FPS) as usize;
        let b = (((f + 1) * OUT_RATE as usize) as f64 / FPS) as usize;
        let n = b - a;
        self.psg_buf.ensure(b);
        for slot in self.psg.iter_mut() {
            let Some(ch) = slot.as_mut() else { continue };
            if !ch.on {
                continue;
            }
            if let Some(y) = mix_psg(ch, bank, n, &self.noise) {
                for (k, v) in y.iter().enumerate() {
                    self.psg_buf.add(a + k, *v);
                }
            }
        }
        self.frame += 1;
    }

    /// Output samples (32768 Hz, +-1.0 full scale) that are final, appended
    /// to `out`; drops buffered data older than reverb and the next call need.
    pub fn take_output(&mut self, out: &mut Vec<[f32; 2]>) {
        let from = self.produced;
        let ds_ready = (self.frame as f64 * self.per_frame) as usize;
        let ds_index = |k: usize| ((k as u64 * self.mr as u64) as f64 / OUT_RATE as f64) as usize;
        // An output sample needs its held Direct Sound sample written.
        let mut to = ((self.frame * OUT_RATE as usize) as f64 / FPS) as usize;
        while to > from && ds_index(to - 1) >= ds_ready {
            to -= 1;
        }
        for k in from..to {
            let d = self.ds_buf.get(ds_index(k));
            let p = self.psg_buf.get(k);
            out.push([((d[0] * 4.0 + p[0]) / 512.0) as f32, ((d[1] * 4.0 + p[1]) / 512.0) as f32]);
        }
        self.produced = to;
        self.psg_buf.trim(to);
        let keep = ds_index(to);
        self.ds_buf.trim(keep.min(ds_ready.saturating_sub(self.taps.1 + 1)));
    }
}

fn ds_env(ch: &mut Channel) {
    let (echo_vol, echo_len) = (ch.echo_volume as i64, ch.echo_length as i64);
    if ch.stage == Stage::Start {
        // A note released before it ever sounded doesn't start.
        if ch.stopping {
            ch.on = false;
            return;
        }
        ch.env = 0;
        ch.stage = Stage::Attack;
    }
    if ch.stage == Stage::Echo {
        ch.echo_left -= 1;
        if ch.echo_left <= 0 {
            ch.on = false;
        }
        return;
    }
    if ch.stopping && ch.stage != Stage::Release {
        ch.stage = Stage::Release;
    }
    let e = ch.voice.envelope;
    match ch.stage {
        Stage::Release => {
            ch.env = (ch.env * e.release as i64) >> 8;
            if ch.env <= echo_vol {
                if echo_vol != 0 && echo_len != 0 {
                    ch.env = echo_vol;
                    ch.stage = Stage::Echo;
                    ch.echo_left = echo_len;
                } else {
                    ch.on = false;
                }
            }
        }
        Stage::Attack => {
            ch.env += e.attack as i64;
            if ch.env >= 255 {
                ch.env = 255;
                ch.stage = Stage::Decay;
            }
        }
        Stage::Decay => {
            ch.env = (ch.env * e.decay as i64) >> 8;
            if ch.env <= e.sustain as i64 {
                ch.env = e.sustain as i64;
                ch.stage = Stage::Sustain;
                if ch.env == 0 {
                    ch.on = false;
                }
            }
        }
        _ => {}
    }
}

fn cgb_env(ch: &mut Channel, goal: i64, sus: i64) {
    let (echo_vol, echo_len) = (ch.echo_volume as i64, ch.echo_length as i64);
    let e = ch.voice.envelope;
    let (a, d, r) = (e.attack as i64, e.decay as i64, e.release as i64);
    let echo_or_stop = |ch: &mut Channel| {
        let level = if echo_vol != 0 && echo_len != 0 { (goal * echo_vol + 0xFF) >> 8 } else { 0 };
        if level != 0 {
            ch.level = level;
            ch.stage = Stage::Echo;
            ch.echo_left = echo_len;
        } else {
            ch.on = false;
        }
    };
    if ch.stage == Stage::Start {
        if a == 0 {
            ch.level = goal;
            ch.stage = Stage::Decay;
            ch.counter = d;
            if d == 0 {
                ch.level = sus;
                ch.stage = Stage::Sustain;
            }
        } else {
            ch.level = 0;
            ch.stage = Stage::Attack;
            ch.counter = a;
        }
        return;
    }
    if ch.stage == Stage::Echo {
        ch.echo_left -= 1;
        if ch.echo_left <= 0 {
            ch.on = false;
        }
        return;
    }
    if ch.stopping && ch.stage != Stage::Release {
        ch.stage = Stage::Release;
        ch.counter = r;
        if r == 0 {
            echo_or_stop(ch);
        }
        return;
    }
    if ch.stage == Stage::Sustain {
        ch.level = sus;
        return;
    }
    ch.counter -= 1;
    if ch.counter > 0 {
        return;
    }
    match ch.stage {
        Stage::Attack => {
            ch.level += 1;
            if ch.level >= goal {
                ch.level = goal;
                ch.stage = Stage::Decay;
                ch.counter = d;
                if d == 0 {
                    ch.level = sus;
                    ch.stage = Stage::Sustain;
                }
            } else {
                ch.counter = a;
            }
        }
        Stage::Decay => {
            ch.level -= 1;
            if ch.level <= sus {
                ch.level = sus;
                ch.stage = Stage::Sustain;
                if sus == 0 {
                    echo_or_stop(ch);
                }
            } else {
                ch.counter = d;
            }
        }
        Stage::Release => {
            ch.level -= 1;
            if ch.level <= 0 {
                echo_or_stop(ch);
            } else {
                ch.counter = r;
            }
        }
        _ => {}
    }
}

fn mix_ds(ch: &mut Channel, smp: &crate::bank::Sample, fixed: bool, n: usize, master: i64, mr: f64) -> Vec<[f64; 2]> {
    let data = &smp.data;
    let size = data.len();
    let loop_start = smp.loop_start.map(|l| l as usize);
    let e = (ch.env * (master + 1)) >> 4;
    let er = ((ch.right * e) >> 8).min(255);
    let el = ((ch.left * e) >> 8).min(255);
    let step = if fixed { 1.0 } else { smp.rate as f64 / 1024.0 * 2f64.powf((ch.semis() - 60.0) / 12.0) / mr };
    let mut out = Vec::with_capacity(n);
    let mut last_pos = ch.pos;
    for k in 0..n {
        let mut pos = ch.pos + step * k as f64;
        let live;
        match loop_start {
            None => {
                live = pos < size as f64;
                if !live {
                    pos = (size - 1) as f64;
                }
            }
            Some(lp) => {
                live = true;
                if pos >= size as f64 {
                    let span = (size - lp) as f64;
                    pos = lp as f64 + (pos - lp as f64) % span;
                }
            }
        }
        let i = pos as usize;
        let s = if fixed {
            data[i] as f64
        } else {
            let mut j = i + 1;
            if j >= size {
                j = loop_start.unwrap_or(size - 1);
            }
            let (x, y) = (data[i] as f64, data[j] as f64);
            (x + (y - x) * (pos - i as f64)).floor()
        };
        let s = if live { s } else { s * 0.0 };
        out.push([(s * el as f64 / 256.0).floor(), (s * er as f64 / 256.0).floor()]);
        last_pos = pos;
    }
    ch.pos = last_pos + step;
    match loop_start {
        None => {
            if ch.pos >= size as f64 {
                ch.on = false;
            }
        }
        Some(lp) => {
            if ch.pos >= size as f64 {
                ch.pos = lp as f64 + (ch.pos - lp as f64) % (size - lp) as f64;
            }
        }
    }
    out
}

/// The GB frequency register for a key (MidiKeyToCgbFreq).
fn cgb_register(key: f64) -> f64 {
    let k = if key > 36.0 { key } else { 36.0 };
    let f = 440.0 * 2f64.powf((k - 69.0) / 12.0);
    (2048.0 - 131072.0 / f).round_ties_even().clamp(0.0, 2047.0)
}

fn mix_psg(ch: &mut Channel, bank: &SoundBank, n: usize, noise: &[Vec<f64>; 2]) -> Option<Vec<[f64; 2]>> {
    let (rv, lv) = (ch.right, ch.left);
    let goal = ((rv + lv) >> 4).min(15);
    let sus = (goal * ch.voice.envelope.sustain as i64 + 15) >> 4;
    cgb_env(ch, goal, sus);
    if !ch.on {
        return None;
    }
    let level = ch.level as f64;
    let mut out: Vec<f64> = Vec::with_capacity(n);
    match ch.voice.kind {
        VoiceKind::Noise { narrow } => {
            let k = ch.key;
            let nr43 = NOISE_NR43[if k <= 20 { 0 } else { ((k - 21) as usize).min(59) }] as u32;
            let (shift, ratio) = (nr43 >> 4, nr43 & 7);
            let r = if ratio == 0 { 0.5 } else { ratio as f64 };
            let clock = 524288.0 / r / 2f64.powi(shift as i32 + 1);
            let bits = &noise[narrow as usize];
            let inc = clock / OUT_RATE as f64;
            for k in 0..n {
                let idx = ((ch.noise_at + inc * k as f64) as usize) % bits.len();
                out.push((bits[idx] - 0.5) * PSG_SCALE * level);
            }
            ch.noise_at += inc * n as f64;
        }
        kind => {
            let wave = match kind {
                VoiceKind::Wave { wave } => Some(&bank.waves[wave.0 as usize].0),
                VoiceKind::Square1 { .. } | VoiceKind::Square2 { .. } => None,
                _ => {
                    ch.on = false;
                    return None;
                }
            };
            let duty = match kind {
                VoiceKind::Square1 { duty } | VoiceKind::Square2 { duty } => DUTY[(duty & 3) as usize],
                _ => 0.0,
            };
            let x = cgb_register(ch.semis());
            let freq = (if wave.is_none() { 131072.0 } else { 65536.0 }) / (2048.0 - x);
            let inc = freq / OUT_RATE as f64;
            let mut last = ch.phase;
            for k in 1..=n {
                let ph = ch.phase + inc * k as f64;
                let frac = ph % 1.0;
                match wave {
                    Some(w) => out.push(
                        (w[(frac * 32.0) as usize] as f64 - 7.5)
                            * WAVE_LEVEL[ch.level.clamp(0, 15) as usize]
                            * PSG_SCALE,
                    ),
                    None => out.push(((if frac < duty { 1.0 } else { 0.0 }) - duty) * PSG_SCALE * level),
                }
                last = ph;
            }
            ch.phase = last % 1.0;
        }
    }
    let right = rv >= lv && rv / 2 >= lv;
    let left = lv > rv && lv / 2 >= rv;
    Some(out.into_iter().map(|v| [if right { 0.0 } else { v }, if left { 0.0 } else { v }]).collect())
}
