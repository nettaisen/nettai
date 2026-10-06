//! Real-time output through the default audio device (cpal).
//!
//! The host renders a frame of audio per battle tick on its own thread and
//! queues it ([`Output::queue`]); the device callback plays the queue,
//! resampled linearly from 32768 Hz to the device rate (after m4a-engine's
//! `Player`). The two meet in a lock-free ring (one writer, one reader), so
//! the callback never waits on the host.
//!
//! The host's ticks come on its display's frames, a frame's ticks at once,
//! and its clock is not the device's: the queue swings by a few ticks and
//! drifts. The callback keeps it near [`TARGET_FILL`] by playing a little
//! faster or slower (at most [`MAX_RATE_SHIFT`], too little to hear: dynamic
//! rate control, as RetroArch does), so it neither runs dry nor grows. A
//! host that stalls still runs it dry: the sound fades out over a moment
//! (no click) and comes back, faded in, once [`RESUME_FILL`] is queued; a
//! host that then catches up at once (the player runs a stall's ticks
//! together) has its oldest sound skipped back to the target, so the sound
//! stays as late as it was.

use crate::{BattleAudio, SAMPLE_RATE, SoundCue};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use m4a::SoundBank;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// Opening the audio output failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputError {
    NoDevice,
    Device(String),
    SampleFormat(String),
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            OutputError::NoDevice => write!(f, "no audio output device"),
            OutputError::Device(e) => write!(f, "audio output: {e}"),
            OutputError::SampleFormat(s) => write!(f, "unsupported sample format {s}"),
        }
    }
}

impl std::error::Error for OutputError {}

/// Samples (at 32768 Hz) the callback keeps queued: about three ticks
/// (50 ms), room for a frame's ticks coming together and a late frame.
pub const TARGET_FILL: usize = 1650;
/// Samples queued before the sound starts again after running dry: two
/// ticks (34 ms), so a stall leaves a short gap.
pub const RESUME_FILL: usize = 1100;
/// Past this many queued (about 200 ms: a host that caught up after a
/// stall), the oldest are skipped down to [`TARGET_FILL`].
const MOST_FILL: usize = 6600;
/// The ring's room: a second of sound.
const ROOM: usize = 32768;
/// The most the callback plays faster or slower than the host's rate to
/// keep the queue at its target: half a percent (under a tenth of a
/// semitone).
pub const MAX_RATE_SHIFT: f64 = 0.005;
/// How fast the callback's reading of the queue follows it (an average over
/// about this many callbacks, half a second at a usual buffer size).
const FILL_SMOOTHING: f64 = 1.0 / 40.0;
/// Device frames a fade out or in takes (about 3 ms).
const FADE_FRAMES: u32 = 128;

/// One writer, one reader: stereo samples (two f32s as one word each), the
/// writer moving `tail`, the reader `head`. Neither waits.
struct Ring {
    slots: Box<[AtomicU64]>,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl Ring {
    fn new(room: usize) -> Ring {
        Ring { slots: (0..room).map(|_| AtomicU64::new(0)).collect(), head: AtomicUsize::new(0), tail: AtomicUsize::new(0) }
    }

    fn pack(v: [f32; 2]) -> u64 {
        v[0].to_bits() as u64 | (v[1].to_bits() as u64) << 32
    }

    fn unpack(w: u64) -> [f32; 2] {
        [f32::from_bits(w as u32), f32::from_bits((w >> 32) as u32)]
    }

    /// The writer: as many of `samples` as there is room for (the rest are
    /// dropped); how many went in.
    fn push(&self, samples: &[[f32; 2]]) -> usize {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        let n = samples.len().min(self.slots.len() - (tail - head));
        for (i, &s) in samples[..n].iter().enumerate() {
            self.slots[(tail + i) % self.slots.len()].store(Ring::pack(s), Ordering::Relaxed);
        }
        self.tail.store(tail + n, Ordering::Release);
        n
    }

    /// How many are queued (either side may ask).
    fn len(&self) -> usize {
        let tail = self.tail.load(Ordering::Acquire);
        tail - self.head.load(Ordering::Acquire).min(tail)
    }

    /// The reader: the sample `i` places on from the oldest (`i` < `len`).
    fn peek(&self, i: usize) -> [f32; 2] {
        let head = self.head.load(Ordering::Relaxed);
        Ring::unpack(self.slots[(head + i) % self.slots.len()].load(Ordering::Relaxed))
    }

    /// The reader: let the `n` oldest go.
    fn skip(&self, n: usize) {
        self.head.fetch_add(n, Ordering::Release);
    }
}

/// What the output saw since the last [`Output::take_stats`]: the device
/// callbacks and their frames; the times the queue ran dry (each a gap,
/// faded) and the device frames played silent; the times a host that
/// caught up had its oldest sound skipped, and the samples skipped or
/// dropped for want of room; the queue's least and most at a callback; the
/// rate shift the callback last played at.
#[derive(Clone, Debug, Default)]
pub struct OutputStats {
    pub callbacks: u32,
    pub frames: u64,
    pub most_frames: u32,
    pub underruns: u32,
    pub silent_frames: u64,
    pub skips: u32,
    pub skipped: u64,
    pub dropped: u64,
    pub least_fill: Option<usize>,
    pub most_fill: usize,
    pub rate_shift: f64,
}

/// The counters the callback keeps for [`OutputStats`], as atomics.
#[derive(Default)]
struct Counters {
    callbacks: AtomicU32,
    frames: AtomicU64,
    most_frames: AtomicU32,
    underruns: AtomicU32,
    silent_frames: AtomicU64,
    skips: AtomicU32,
    skipped: AtomicU64,
    dropped: AtomicU64,
    least_fill: AtomicUsize,
    most_fill: AtomicUsize,
    /// The rate shift, as f64 bits.
    rate_shift: AtomicU64,
}

struct Shared {
    ring: Ring,
    /// The volume, as f32 bits.
    volume: AtomicU32,
    counters: Counters,
}

/// The callback's own state.
struct Reader {
    /// Read position between the two oldest queued samples.
    frac: f64,
    playing: bool,
    /// The queue's length, smoothed (dynamic rate control's reading).
    fill: f64,
    /// The last sample heard; and a fade: from `from`, frames left of it
    /// (to silence while dry, to the queue's sound while playing).
    last: [f32; 2],
    from: [f32; 2],
    blend: u32,
}

impl Reader {
    fn new() -> Reader {
        Reader { frac: 0.0, playing: false, fill: TARGET_FILL as f64, last: [0.0; 2], from: [0.0; 2], blend: 0 }
    }

    /// Fade from what was last heard, over the next frames (a start, a
    /// skip, running dry: no click).
    fn fade_from_last(&mut self) {
        self.from = self.last;
        self.blend = FADE_FRAMES;
    }

    /// The next device frame's sample (before the volume).
    fn next(&mut self, ring: &Ring, step: f64, c: &Counters) -> [f32; 2] {
        let len = ring.len();
        if !self.playing {
            if len < RESUME_FILL {
                c.silent_frames.fetch_add(1, Ordering::Relaxed);
                return self.dry();
            }
            self.playing = true;
            self.fade_from_last();
        }
        if len < 2 {
            // Ran dry: fade out what was playing, and wait for the resume
            // fill.
            self.playing = false;
            self.fade_from_last();
            c.underruns.fetch_add(1, Ordering::Relaxed);
            c.silent_frames.fetch_add(1, Ordering::Relaxed);
            return self.dry();
        }
        let t = self.frac as f32;
        let (a, b) = (ring.peek(0), ring.peek(1));
        let mut v = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
        if self.blend > 0 {
            let k = self.blend as f32 / FADE_FRAMES as f32;
            v = [v[0] + (self.from[0] - v[0]) * k, v[1] + (self.from[1] - v[1]) * k];
            self.blend -= 1;
        }
        self.frac += step;
        let n = (self.frac as usize).min(len);
        ring.skip(n);
        self.frac -= n as f64;
        self.last = v;
        v
    }

    /// Dry: the fade out's next sample, to silence.
    fn dry(&mut self) -> [f32; 2] {
        self.blend = self.blend.saturating_sub(1);
        let g = self.blend as f32 / FADE_FRAMES as f32;
        self.last = [self.from[0] * g, self.from[1] * g];
        self.last
    }

    /// A device buffer is due: past the most queued, the oldest skipped
    /// back to the target (faded); the rate to play at, by the smoothed
    /// fill (a frame's ticks come together, so the queue swings within a
    /// frame).
    fn plan(&mut self, ring: &Ring, base: f64, c: &Counters) -> f64 {
        let mut len = ring.len();
        if len > MOST_FILL {
            let skip = len - TARGET_FILL;
            ring.skip(skip);
            len -= skip;
            self.fill = len as f64;
            if self.playing {
                self.fade_from_last();
            }
            c.skips.fetch_add(1, Ordering::Relaxed);
            c.skipped.fetch_add(skip as u64, Ordering::Relaxed);
        }
        if self.playing {
            self.fill += (len as f64 - self.fill) * FILL_SMOOTHING;
        }
        let shift = ((self.fill - TARGET_FILL as f64) / TARGET_FILL as f64).clamp(-1.0, 1.0) * MAX_RATE_SHIFT;
        c.rate_shift.store(shift.to_bits(), Ordering::Relaxed);
        base * (1.0 + shift)
    }
}

/// The default output device, playing the samples it is given: frames of
/// stereo samples at [`SAMPLE_RATE`], as [`BattleAudio::tick`] renders them
/// (a host that has the battle's sound as samples, nettai-frontend's
/// player's, plays it through this).
pub struct Output {
    shared: Arc<Shared>,
    device_rate: u32,
    _stream: cpal::Stream,
}

impl Output {
    /// Open the default output device.
    pub fn open() -> Result<Output, OutputError> {
        let device = cpal::default_host().default_output_device().ok_or(OutputError::NoDevice)?;
        let config = device.default_output_config().map_err(|e| OutputError::Device(e.to_string()))?;
        let device_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let shared = Arc::new(Shared {
            ring: Ring::new(ROOM),
            volume: AtomicU32::new(0.8f32.to_bits()),
            counters: Counters { least_fill: AtomicUsize::new(usize::MAX), ..Counters::default() },
        });
        let st = shared.clone();
        let base = SAMPLE_RATE as f64 / device_rate as f64;
        let mut reader = Reader::new();
        let mut fill = move |out: &mut [f32]| {
            let (ring, c) = (&st.ring, &st.counters);
            let frames = out.len() / channels.max(1);
            let len = ring.len();
            c.callbacks.fetch_add(1, Ordering::Relaxed);
            c.frames.fetch_add(frames as u64, Ordering::Relaxed);
            c.most_frames.fetch_max(frames as u32, Ordering::Relaxed);
            c.least_fill.fetch_min(len, Ordering::Relaxed);
            c.most_fill.fetch_max(len, Ordering::Relaxed);
            let step = reader.plan(ring, base, c);
            let volume = f32::from_bits(st.volume.load(Ordering::Relaxed));
            for frame in out.chunks_mut(channels.max(1)) {
                let v = reader.next(ring, step, c);
                for (ch, x) in frame.iter_mut().enumerate() {
                    *x = v[ch.min(1)] * volume;
                }
            }
        };
        let err = |e| eprintln!("audio stream error: {e}");
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(&config.clone().into(), move |d: &mut [f32], _| fill(d), err, None),
            cpal::SampleFormat::I16 => {
                let mut buf = Vec::new();
                device.build_output_stream(
                    &config.clone().into(),
                    move |d: &mut [i16], _| {
                        buf.resize(d.len(), 0.0f32);
                        fill(&mut buf);
                        for (o, v) in d.iter_mut().zip(&buf) {
                            *o = (v.clamp(-1.0, 1.0) * 32767.0) as i16;
                        }
                    },
                    err,
                    None,
                )
            }
            f => return Err(OutputError::SampleFormat(format!("{f:?}"))),
        }
        .map_err(|e| OutputError::Device(e.to_string()))?;
        stream.play().map_err(|e| OutputError::Device(e.to_string()))?;
        Ok(Output { shared, device_rate, _stream: stream })
    }

    /// Queue samples for the device: what the battle's ticks since the last
    /// call rendered. (Past a second queued, which the callback never lets
    /// happen while it runs, the newest are dropped.)
    pub fn queue(&self, samples: &[[f32; 2]]) {
        let n = self.shared.ring.push(samples);
        if n < samples.len() {
            self.shared.counters.dropped.fetch_add((samples.len() - n) as u64, Ordering::Relaxed);
        }
    }

    /// Output volume (1.0 = the GBA's full scale).
    pub fn set_volume(&self, v: f32) {
        self.shared.volume.store(v.to_bits(), Ordering::Relaxed);
    }

    /// Audio queued but not yet played, in seconds.
    pub fn buffered(&self) -> f64 {
        self.shared.ring.len() as f64 / SAMPLE_RATE as f64
    }

    pub fn device_rate(&self) -> u32 {
        self.device_rate
    }

    /// What the output saw since the last call (`NETTAI_PLAY_STATS`'s
    /// figures).
    pub fn take_stats(&self) -> OutputStats {
        let c = &self.shared.counters;
        let least = c.least_fill.swap(usize::MAX, Ordering::Relaxed);
        OutputStats {
            callbacks: c.callbacks.swap(0, Ordering::Relaxed),
            frames: c.frames.swap(0, Ordering::Relaxed),
            most_frames: c.most_frames.swap(0, Ordering::Relaxed),
            underruns: c.underruns.swap(0, Ordering::Relaxed),
            silent_frames: c.silent_frames.swap(0, Ordering::Relaxed),
            skips: c.skips.swap(0, Ordering::Relaxed),
            skipped: c.skipped.swap(0, Ordering::Relaxed),
            dropped: c.dropped.swap(0, Ordering::Relaxed),
            least_fill: (least != usize::MAX).then_some(least),
            most_fill: c.most_fill.swap(0, Ordering::Relaxed),
            rate_shift: f64::from_bits(c.rate_shift.load(Ordering::Relaxed)),
        }
    }
}

/// The game's sound on the default output device: a [`BattleAudio`] whose
/// frames go to an [`Output`].
pub struct AudioOut {
    audio: BattleAudio,
    out: Output,
    frame: Vec<[f32; 2]>,
}

impl AudioOut {
    pub fn new(bank: Arc<SoundBank>, songs: crate::Songs) -> Result<AudioOut, OutputError> {
        AudioOut::with_banks(vec![bank], songs)
    }

    /// The sound of several packs (`banks` by `PackId`) on the default
    /// output device.
    pub fn with_banks(banks: Vec<Arc<SoundBank>>, songs: crate::Songs) -> Result<AudioOut, OutputError> {
        Ok(AudioOut { out: Output::open()?, audio: BattleAudio::with_banks(banks, songs), frame: Vec::new() })
    }

    /// Queue a tick's cues (see [`BattleAudio::handle`]).
    pub fn handle(&mut self, cues: &[SoundCue]) {
        self.audio.handle(cues);
    }

    /// Queue rollback cue actions (see [`BattleAudio::handle_actions`]).
    pub fn handle_actions(&mut self, actions: impl IntoIterator<Item = nettai_battle::cues::CueAction>) {
        self.audio.handle_actions(actions);
    }

    /// Render one frame and queue it for the device. Call once per battle
    /// tick, at the game's 59.73 Hz.
    pub fn tick(&mut self) {
        self.frame.clear();
        self.audio.tick(&mut self.frame);
        self.out.queue(&self.frame);
    }

    /// Output volume (1.0 = the GBA's full scale).
    pub fn set_volume(&self, v: f32) {
        self.out.set_volume(v);
    }

    /// Audio queued but not yet played, in seconds.
    pub fn buffered(&self) -> f64 {
        self.out.buffered()
    }

    pub fn device_rate(&self) -> u32 {
        self.out.device_rate()
    }

    pub fn audio(&self) -> &BattleAudio {
        &self.audio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ring: what goes in comes out in order, as much as there is room
    /// for, and the reader lets them go.
    #[test]
    fn the_ring_keeps_order_and_room() {
        let r = Ring::new(4);
        assert_eq!(r.push(&[[1.0, -1.0], [2.0, -2.0], [3.0, -3.0]]), 3);
        assert_eq!(r.push(&[[4.0, -4.0], [5.0, -5.0]]), 1, "room for one more");
        assert_eq!((r.len(), r.peek(0), r.peek(3)), (4, [1.0, -1.0], [4.0, -4.0]));
        r.skip(3);
        assert_eq!(r.push(&[[6.0, -6.0], [7.0, -7.0]]), 2, "round the end");
        assert_eq!((r.len(), r.peek(0), r.peek(1), r.peek(2)), (3, [4.0, -4.0], [6.0, -6.0], [7.0, -7.0]));
    }

    /// The reader starts once the resume fill is there, faded in; fades out
    /// when the queue runs dry (no click), and back in.
    #[test]
    fn the_reader_fades_out_dry_and_back_in() {
        let ring = Ring::new(ROOM);
        let c = Counters::default();
        let mut r = Reader::new();
        ring.push(&vec![[0.5, 0.5]; RESUME_FILL - 1]);
        assert_eq!(r.next(&ring, 1.0, &c), [0.0; 2], "not yet");
        ring.push(&[[0.5, 0.5]]);
        let first = r.next(&ring, 1.0, &c);
        assert!(first[0] < 0.01, "faded in from silence: {first:?}");
        let mut v = first;
        for _ in 0..FADE_FRAMES {
            v = r.next(&ring, 1.0, &c);
        }
        assert_eq!(v, [0.5, 0.5]);
        while ring.len() >= 2 {
            r.next(&ring, 1.0, &c);
        }
        let out = r.next(&ring, 1.0, &c);
        assert!(out[0] > 0.45 && out[0] <= 0.5, "down over the fade, not at once: {out:?}");
        for _ in 0..FADE_FRAMES {
            v = r.next(&ring, 1.0, &c);
        }
        assert_eq!(v, [0.0; 2]);
        assert_eq!(c.underruns.load(Ordering::Relaxed), 1);
        ring.push(&vec![[0.25, 0.25]; RESUME_FILL]);
        assert!(r.next(&ring, 1.0, &c)[0] < 0.01, "back, faded in");
    }

    /// The rate follows the queue: faster above the target, slower below
    /// it, by half a percent at most; a queue past the most is skipped back
    /// to the target.
    #[test]
    fn the_rate_keeps_the_queue_at_its_target() {
        let c = Counters::default();
        let ring = Ring::new(ROOM);
        let mut r = Reader::new();
        r.playing = true;
        ring.push(&vec![[0.0; 2]; 2 * TARGET_FILL]);
        let mut step = 1.0;
        for _ in 0..400 {
            step = r.plan(&ring, 1.0, &c);
        }
        assert!((step - (1.0 + MAX_RATE_SHIFT)).abs() < 1e-4, "faster: {step}");
        ring.skip(ring.len() - TARGET_FILL / 2);
        for _ in 0..400 {
            step = r.plan(&ring, 1.0, &c);
        }
        assert!(step < 1.0 && step > 1.0 - MAX_RATE_SHIFT, "slower: {step}");
        ring.push(&vec![[0.0; 2]; MOST_FILL]);
        r.plan(&ring, 1.0, &c);
        assert_eq!((ring.len(), c.skips.load(Ordering::Relaxed)), (TARGET_FILL, 1));
    }

    /// A simulated minute: the host queues a tick's samples at 59.7275 Hz
    /// on frames that come unevenly (a frame's ticks at once, a late frame
    /// now and then), the device takes 512 frames at 44.1 kHz on its own
    /// clock (a little off the host's): the queue never runs dry, and stays
    /// near its target.
    #[test]
    fn uneven_frames_and_two_clocks_leave_no_gap() {
        let c = Counters::default();
        let ring = Ring::new(ROOM);
        let mut r = Reader::new();
        let base = SAMPLE_RATE as f64 / 44100.0;
        // (The device's clock 0.2% fast against the host's.)
        let device_rate = 44100.0 * 1.002;
        let tick = 1.0 / 59.7275;
        let callback = 512.0 / device_rate;
        let (mut host, mut device, mut owed, mut frame_no) = (0.0f64, 0.0f64, 0.0f64, 0u64);
        let mut buf = vec![0f32; 1024];
        let samples_per_tick = SAMPLE_RATE as f64 / 59.7275;
        let mut made = 0.0f64;
        let mut least = usize::MAX;
        while device < 60.0 {
            if host <= device {
                // A frame: 75 a second, every 37th a 27 ms one.
                frame_no += 1;
                let gap = if frame_no % 37 == 0 { 0.027 } else { 1.0 / 75.0 };
                host += gap;
                owed += gap / tick;
                while owed >= 1.0 {
                    owed -= 1.0;
                    made += samples_per_tick;
                    let n = made.floor() as usize;
                    made -= n as f64;
                    ring.push(&vec![[0.1, 0.1]; n]);
                }
            } else {
                device += callback;
                let step = r.plan(&ring, base, &c);
                if r.playing {
                    least = least.min(ring.len());
                }
                for frame in buf.chunks_mut(2) {
                    let v = r.next(&ring, step, &c);
                    frame[0] = v[0];
                }
            }
        }
        assert_eq!(c.underruns.load(Ordering::Relaxed), 0, "least {least}");
        assert_eq!(c.skips.load(Ordering::Relaxed), 0);
        assert!(least > 300, "least {least}");
    }
}
