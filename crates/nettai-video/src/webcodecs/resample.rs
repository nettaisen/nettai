//! Stereo sound made 48 kHz, the rate the browser's encoders take.

/// The rate the sound is made.
pub const RATE: u32 = 48_000;

/// Stereo sound at one rate made 48 kHz: each output sample interpolated between its input's four nearest
/// (Catmull-Rom), the input's place carried from frame to frame.
pub struct Resampler {
    step: f64,
    /// Where the next output sample is, in the input's samples since `history[1]`.
    at: f64,
    /// The input's last samples before what comes next (the interpolation's reach back).
    history: Vec<[f32; 2]>,
}

impl Resampler {
    pub fn new(rate: u32) -> Resampler {
        Resampler { step: rate as f64 / RATE as f64, at: 0.0, history: vec![[0.0; 2]; 3] }
    }

    pub fn push(&mut self, sound: &[[f32; 2]]) -> Vec<f32> {
        let mut input = std::mem::take(&mut self.history);
        input.extend_from_slice(sound);
        let mut out = Vec::new();
        // (Each output sample at `at` from input[1] needs input[0] to input[3] around it.)
        while self.at + 3.0 < input.len() as f64 {
            let i = self.at.floor() as usize;
            let t = (self.at - i as f64) as f32;
            for ch in 0..2 {
                let [p0, p1, p2, p3] = [input[i][ch], input[i + 1][ch], input[i + 2][ch], input[i + 3][ch]];
                let v = p1 + 0.5 * t * (p2 - p0 + t * (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3 + t * (3.0 * (p1 - p2) + p3 - p0)));
                out.push(v.clamp(-1.0, 1.0));
            }
            self.at += self.step;
        }
        let keep = (self.at.floor() as usize).min(input.len());
        self.at -= keep as f64;
        self.history = input.split_off(keep);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sound resampled keeps its place from frame to frame: a second of it, in frames of any size, is a
    /// second at 48 kHz (to within a sample), and a steady level stays that level.
    #[test]
    fn the_sound_is_resampled_across_frames() {
        let mut r = Resampler::new(32_768);
        let mut out = 0;
        for size in [546, 547, 546, 1, 32_768 - 1640] {
            let made = r.push(&vec![[0.5, -0.25]; size]);
            // (Past the start, whose first few samples rise from the silence before it.)
            let past = if out == 0 { 5 } else { 0 };
            assert!(made.chunks(2).skip(past).all(|s| (s[0] - 0.5).abs() < 1e-6 && (s[1] + 0.25).abs() < 1e-6));
            out += made.len() / 2;
        }
        assert!((out as i64 - 48_000).abs() <= 3, "{out}");
    }
}
