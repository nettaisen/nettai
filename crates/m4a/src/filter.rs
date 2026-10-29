//! The low-pass filter the output paths share, and the decimator that takes
//! the DAC's output down to the host's rate with it.

use std::sync::Arc;

/// A Kaiser-windowed sinc low-pass's weight `t` samples from its center:
/// half down at `cutoff` (cycles a sample), reaching `half` samples either
/// side, its window's beta `beta` (6.76: 70 dB down past the cutoff, the
/// transition 62 / (14.36 * 2 * `half`) cycles a sample wide). Its weights
/// a sample apart add up to about 1.
pub fn low_pass(t: f64, cutoff: f64, half: f64, beta: f64) -> f64 {
    let w = 1.0 - (t / half).powi(2);
    if w <= 0.0 {
        return 0.0;
    }
    let x = std::f64::consts::PI * 2.0 * cutoff * t;
    let sinc = if x == 0.0 { 1.0 } else { x.sin() / x };
    2.0 * cutoff * sinc * bessel_i0(beta * w.sqrt()) / bessel_i0(beta)
}

/// The modified Bessel function of the first kind, order 0 (the Kaiser
/// window's).
fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term, mut k) = (1.0f64, 1.0f64, 1.0f64);
    while term > 1e-12 * sum {
        term *= (x / (2.0 * k)).powi(2);
        sum += term;
        k += 1.0;
    }
    sum
}

/// The [`low_pass`]'s beta for 70 dB.
pub const KAISER_BETA: f64 = 6.76;

/// Takes stereo samples down by a whole ratio: each output sample is the
/// average of the `ratio` input samples it stands for (what the output was
/// first made with) through a low-pass at the output's half rate. Averaging
/// alone lets what lies between the output's half rate and the input's
/// fold back into the sound (a 20 kHz tone in the DAC's 65536 Hz comes back
/// at 12768 Hz, 5 dB down); the low-pass takes it out (70 dB), and below
/// it the tone is the average's to a hundredth of a decibel.
#[derive(Clone, Debug)]
pub(crate) struct Decimator {
    /// The weights, the newest input's first.
    taps: Arc<[f32]>,
    ratio: usize,
    /// The last `taps.len()` inputs, twice over (so a window of them is one
    /// slice), `at` the oldest's place.
    history: Vec<[f32; 2]>,
    at: usize,
    /// Inputs since the last output.
    count: usize,
}

/// The decimator's low-pass reaches this many output samples either side.
const DECIMATOR_HALF: usize = 36;
/// Its cutoff, as a share of the output's half rate.
const DECIMATOR_CUTOFF: f64 = 0.94;

impl Decimator {
    pub fn new(ratio: usize) -> Decimator {
        let ratio = ratio.max(1);
        let taps: Vec<f64> = if ratio == 1 {
            vec![1.0]
        } else {
            let half = (DECIMATOR_HALF * ratio) as i64;
            let cutoff = DECIMATOR_CUTOFF * 0.5 / ratio as f64;
            let lp: Vec<f64> = (-half..=half).map(|k| low_pass(k as f64, cutoff, half as f64 + 1.0, KAISER_BETA)).collect();
            // (The low-pass convolved with the average's `ratio` equal weights.)
            let mut taps = vec![0.0; lp.len() + ratio - 1];
            for (i, &w) in lp.iter().enumerate() {
                for t in &mut taps[i..i + ratio] {
                    *t += w / ratio as f64;
                }
            }
            let sum: f64 = taps.iter().sum();
            taps.iter().map(|t| t / sum).collect()
        };
        let taps: Arc<[f32]> = taps.iter().map(|&t| t as f32).collect();
        let n = taps.len();
        Decimator { taps, ratio, history: vec![[0.0; 2]; 2 * n], at: 0, count: 0 }
    }

    /// One input sample; an output sample each `ratio` of them.
    pub fn push(&mut self, x: [f32; 2]) -> Option<[f32; 2]> {
        let n = self.taps.len();
        self.history[self.at] = x;
        self.history[self.at + n] = x;
        self.at = (self.at + 1) % n;
        self.count += 1;
        if self.count < self.ratio {
            return None;
        }
        self.count = 0;
        // (The window, oldest first: the taps are symmetric.)
        let window = &self.history[self.at..self.at + n];
        let mut y = [0f32; 2];
        for (s, &w) in window.iter().zip(self.taps.iter()) {
            y[0] += s[0] * w;
            y[1] += s[1] * w;
        }
        Some(y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sine at `freq` (Hz, at 65536 Hz) through a 2:1 decimator: its level
    /// at `at` in the output (32768 Hz), dB.
    fn decimated(freq: f64, at: f64) -> f64 {
        let mut d = Decimator::new(2);
        let out: Vec<f32> = (0..65536)
            .filter_map(|i| {
                let v = (2.0 * std::f64::consts::PI * freq * i as f64 / 65536.0).sin() as f32;
                d.push([v, v])
            })
            .skip(1000)
            .map(|s| s[0])
            .collect();
        let (mut re, mut im) = (0.0, 0.0);
        for (j, &v) in out.iter().enumerate() {
            let w = 2.0 * std::f64::consts::PI * at * j as f64 / 32768.0;
            re += v as f64 * w.cos();
            im += v as f64 * w.sin();
        }
        20.0 * ((re * re + im * im).sqrt() * 2.0 / out.len() as f64).max(1e-12).log10()
    }

    #[test]
    fn the_decimator_keeps_the_averages_tone_without_the_fold() {
        for freq in [440.0, 5000.0, 10000.0, 14000.0] {
            // (Two samples' average: cos(pi * f / 65536).)
            let average = 20.0 * (std::f64::consts::PI * freq / 65536.0).cos().log10();
            let level = decimated(freq, freq);
            assert!((level - average).abs() < 0.05, "{freq} Hz: {level:.3} dB, the average {average:.3}");
        }
        for freq in [18000.0, 20000.0, 26000.0] {
            let folded = decimated(freq, 32768.0 - freq);
            assert!(folded < -70.0, "{freq} Hz folds back at {folded:.1} dB");
        }
    }

    #[test]
    fn a_ratio_of_one_passes_the_input() {
        let mut d = Decimator::new(1);
        assert_eq!(d.push([0.25, -0.5]), Some([0.25, -0.5]));
    }
}
