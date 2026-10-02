//! Real-time output through the default audio device (cpal).
//!
//! The frontend's tick renders a frame of audio on its own thread and
//! queues it; the device callback plays the queue, resampled linearly from
//! 32768 Hz to the device rate (after m4a-engine's `Player`). A few frames
//! are buffered before playback starts; if the frontend runs ahead the
//! oldest audio is dropped, if it falls behind the device plays silence.

use crate::{BattleAudio, SAMPLE_RATE, SoundCue};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use m4a::SoundBank;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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

/// Samples (at 32768 Hz) buffered before playback starts, and the most
/// kept: about three and twelve frames.
const START_FILL: usize = 1650;
const MAX_FILL: usize = 6600;

struct Shared {
    queue: VecDeque<[f32; 2]>,
    /// Read position between the first two queued samples.
    frac: f64,
    playing: bool,
    volume: f32,
}

/// The game's sound on the default output device.
pub struct AudioOut {
    audio: BattleAudio,
    shared: Arc<Mutex<Shared>>,
    frame: Vec<[f32; 2]>,
    device_rate: u32,
    _stream: cpal::Stream,
}

impl AudioOut {
    pub fn new(bank: Arc<SoundBank>, songs: crate::Songs) -> Result<AudioOut, OutputError> {
        AudioOut::with_banks(vec![bank], songs)
    }

    /// The sound of several packs (`banks` by `PackId`) on the default
    /// output device.
    pub fn with_banks(banks: Vec<Arc<SoundBank>>, songs: crate::Songs) -> Result<AudioOut, OutputError> {
        let device = cpal::default_host().default_output_device().ok_or(OutputError::NoDevice)?;
        let config = device.default_output_config().map_err(|e| OutputError::Device(e.to_string()))?;
        let device_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let shared = Arc::new(Mutex::new(Shared { queue: VecDeque::new(), frac: 0.0, playing: false, volume: 0.8 }));
        let st = shared.clone();
        let step = SAMPLE_RATE as f64 / device_rate as f64;
        let fill = move |out: &mut [f32]| {
            let mut s = st.lock().unwrap();
            for frame in out.chunks_mut(channels) {
                let v = next_sample(&mut s, step);
                for (c, x) in frame.iter_mut().enumerate() {
                    *x = v[c.min(1)];
                }
            }
        };
        let err = |e| eprintln!("audio stream error: {e}");
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_output_stream(&config.clone().into(), move |d: &mut [f32], _| fill(d), err, None)
            }
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
        Ok(AudioOut { audio: BattleAudio::with_banks(banks, songs), shared, frame: Vec::new(), device_rate, _stream: stream })
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
        let mut s = self.shared.lock().unwrap();
        s.queue.extend(self.frame.iter().copied());
        if s.queue.len() > MAX_FILL {
            let drop = s.queue.len() - START_FILL;
            s.queue.drain(..drop);
        }
        if s.queue.len() >= START_FILL {
            s.playing = true;
        }
    }

    /// Output volume (1.0 = the GBA's full scale).
    pub fn set_volume(&self, v: f32) {
        self.shared.lock().unwrap().volume = v;
    }

    /// Audio queued but not yet played, in seconds.
    pub fn buffered(&self) -> f64 {
        self.shared.lock().unwrap().queue.len() as f64 / SAMPLE_RATE as f64
    }

    pub fn device_rate(&self) -> u32 {
        self.device_rate
    }

    pub fn audio(&self) -> &BattleAudio {
        &self.audio
    }
}

fn next_sample(s: &mut Shared, step: f64) -> [f32; 2] {
    if !s.playing || s.queue.len() < 2 {
        // Ran dry: wait for a new start fill (no clicks from a trickle).
        s.playing = false;
        return [0.0; 2];
    }
    let t = s.frac as f32;
    let (a, b) = (s.queue[0], s.queue[1]);
    let v = [(a[0] + (b[0] - a[0]) * t) * s.volume, (a[1] + (b[1] - a[1]) * t) * s.volume];
    s.frac += step;
    let drop = (s.frac as usize).min(s.queue.len());
    s.queue.drain(..drop);
    s.frac -= drop as f64;
    v
}
