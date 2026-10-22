//! The sound made 48 kHz before it is encoded: the rate both encoders' codecs take (AAC and Opus), which the
//! consoles' 32768 Hz isn't. It is done here, before either encoder is given it, so ffmpeg on the desktop and the
//! browser's encoder are given the same samples (the user: "i want to make the input to the codec layer the same
//! for both paths"; ffmpeg resampled for itself before).
//!
//! Each output sample is the input about its place through a windowed sinc: [`TAPS`] input samples, a Kaiser
//! window ([`BETA`]: about 90 dB down past the cutoff), cut off at [`CUTOFF`] of the lower rate's half (15.9 kHz
//! from 32768 Hz: flat to about 15.2, gone from 16.6, where the image of what is under 16.1 starts). The places are exact: from 32768 Hz, output sample `n` is at input sample `n * 256 / 375` (the
//! rates over their greatest common divisor), one of 375 phases, each with its weights worked out once. So nothing
//! drifts over any length, and the output is the same, sample for sample, however the input is split into runs.
//! Output sample 0 is at input sample 0, so the sound keeps its place against the picture; the last half a
//! filter of the output waits for the input after it, and [`Resampler::finish`] makes it, the input ending there.

/// The rate the sound is made.
pub const RATE: u32 = 48_000;

/// The input samples an output sample is made of: as many after its place as before.
const TAPS: usize = 128;
const HALF: usize = TAPS / 2;

/// The Kaiser window's β.
const BETA: f64 = 9.0;

/// Where the filter cuts off, as a part of the lower rate's half.
const CUTOFF: f64 = 0.97;

/// Stereo sound at one rate made [`RATE`].
pub struct Resampler {
    /// The rates over their greatest common divisor: `up` output samples for every `down` input.
    up: u64,
    down: u64,
    /// Each phase's weights: output sample `n`'s phase is `n * down % up`, and its weights are for the input from
    /// `HALF - 1` samples before its place's to `HALF` after.
    phases: Vec<[f32; TAPS]>,
    /// The input still reached, from its sample `first` (before the input's first sample: silence).
    input: Vec<[f32; 2]>,
    first: i64,
    /// The input samples given, and the output samples made.
    given: u64,
    made: u64,
}

impl Resampler {
    /// A resampler of sound at `rate` samples a second.
    pub fn new(rate: u32) -> Resampler {
        let common = gcd(rate as u64, RATE as u64);
        let (up, down) = (RATE as u64 / common, rate as u64 / common);
        let cutoff = CUTOFF * (up as f64 / down as f64).min(1.0);
        let phases = (0..up)
            .map(|phase| {
                let mut weights = [0f64; TAPS];
                for (t, w) in weights.iter_mut().enumerate() {
                    // (How far input sample `t` of the run is from the output sample's place, in input samples.)
                    let x = phase as f64 / up as f64 - (t as f64 - (HALF as f64 - 1.0));
                    *w = sinc(cutoff * x) * kaiser(x / HALF as f64);
                }
                // (Each phase's weights sum to 1: a steady level stays that level.)
                let sum: f64 = weights.iter().sum();
                weights.map(|w| (w / sum) as f32)
            })
            .collect();
        Resampler { up, down, phases, input: vec![[0.0; 2]; HALF - 1], first: -(HALF as i64 - 1), given: 0, made: 0 }
    }

    /// The output samples `sound`, the input's next run, reaches: every one whose filter the input now covers.
    pub fn push(&mut self, sound: &[[f32; 2]]) -> Vec<[f32; 2]> {
        self.input.extend_from_slice(sound);
        self.given += sound.len() as u64;
        self.make(u64::MAX)
    }

    /// The rest of the output, the input ending here: each output sample placed before the input's end, the
    /// input after it silence. Nothing is pushed after.
    pub fn finish(&mut self) -> Vec<[f32; 2]> {
        let all = (self.given * self.up).div_ceil(self.down);
        self.input.extend(std::iter::repeat_n([0.0; 2], HALF));
        self.make(all)
    }

    /// Every output sample the input covers, up to `most` made in all; the input no later one reaches let go.
    fn make(&mut self, most: u64) -> Vec<[f32; 2]> {
        let end = self.first + self.input.len() as i64;
        let mut out = Vec::new();
        while self.made < most {
            let at = self.made * self.down;
            let (sample, phase) = ((at / self.up) as i64, (at % self.up) as usize);
            if sample + HALF as i64 >= end {
                break;
            }
            let from = (sample - (HALF as i64 - 1) - self.first) as usize;
            let mut v = [0f32; 2];
            for (w, s) in self.phases[phase].iter().zip(&self.input[from..from + TAPS]) {
                v[0] += w * s[0];
                v[1] += w * s[1];
            }
            out.push(v.map(|v| v.clamp(-1.0, 1.0)));
            self.made += 1;
        }
        let next = (self.made * self.down / self.up) as i64 - (HALF as i64 - 1);
        let past = (next - self.first).clamp(0, self.input.len() as i64) as usize;
        self.input.drain(..past);
        self.first += past as i64;
        out
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// sin(πx) / πx.
fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        return 1.0;
    }
    let px = std::f64::consts::PI * x;
    px.sin() / px
}

/// The Kaiser window at `u`, -1 to 1 across it.
fn kaiser(u: f64) -> f64 {
    bessel_i0(BETA * (1.0 - u * u).max(0.0).sqrt()) / bessel_i0(BETA)
}

/// The modified Bessel function of the first kind, of order 0: its series.
fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term) = (1.0, 1.0);
    for k in 1..64 {
        let half = x / (2.0 * k as f64);
        term *= half * half;
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `sound` resampled from 32768 Hz in runs of the sizes `runs` (then the rest at once), and finished.
    fn resampled(sound: &[[f32; 2]], runs: &[usize]) -> Vec<[f32; 2]> {
        let mut r = Resampler::new(32_768);
        let mut out = Vec::new();
        let mut at = 0;
        for &size in runs.iter().chain([&(sound.len() - runs.iter().sum::<usize>())]) {
            out.extend(r.push(&sound[at..at + size]));
            at += size;
        }
        out.extend(r.finish());
        out
    }

    /// A second of sound is a second at 48 kHz exactly, a steady level stays that level (but where the filter
    /// reaches the silence either side), and the samples are the same whatever runs the input came in.
    #[test]
    fn the_sound_is_resampled_across_runs() {
        let steady = vec![[0.5f32, -0.25]; 32_768];
        let whole = resampled(&steady, &[]);
        assert_eq!(whole.len(), 48_000);
        assert!(whole[100..whole.len() - 100].iter().all(|s| (s[0] - 0.5).abs() < 1e-6 && (s[1] + 0.25).abs() < 1e-6));
        let tone: Vec<[f32; 2]> = (0..32_768).map(|n| [(n as f32 * 0.37).sin() * 0.8, (n as f32 * 0.011).cos() * 0.6]).collect();
        let at_once = resampled(&tone, &[]);
        assert!(resampled(&tone, &[546, 547, 1, 0, 548, 3000, 31]) == at_once);
        assert!(resampled(&tone, &[1; 700]) == at_once);
        // (A run's length in output: 375 for every 256 in, rounded up.)
        assert_eq!(resampled(&tone[..549], &[]).len(), 805);
    }

    /// A tone resampled is that tone at 48 kHz, in its place: sample `n` is the tone at `n / 48000` s, from the
    /// low notes to past 13 kHz, what is left of its image (at 32768 Hz less its pitch) and the filter's ripple
    /// 85 dB down or more (measured: 94 at 13.5 kHz, 99 at 10 kHz, 109 to 119 below).
    #[test]
    fn a_tone_is_the_same_tone_at_48_khz() {
        let tone = |hz: f64, rate: f64, n: usize| -> Vec<[f32; 2]> {
            (0..n).map(|i| {
                let t = i as f64 / rate;
                let v = (2.0 * std::f64::consts::PI * hz * t).sin() * 0.5;
                [v as f32, (-v) as f32]
            }).collect()
        };
        for hz in [55.0, 440.0, 1000.0, 5000.0, 10_000.0, 13_500.0] {
            let made = resampled(&tone(hz, 32_768.0, 32_768), &[]);
            let want = tone(hz, 48_000.0, 48_000);
            let off = made[100..47_900].iter().zip(&want[100..47_900]).map(|(a, b)| (a[0] - b[0]).abs().max((a[1] - b[1]).abs())).fold(0f32, f32::max);
            assert!(off < 0.5 * 10f32.powf(-85.0 / 20.0), "{hz} Hz: {:.1} dB off", 20.0 * (off / 0.5).log10());
        }
    }
}
