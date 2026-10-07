//! A battle on the screen: the library's player, driven a frame at a time,
//! and the picture it presents, as the window shows it.
//!
//! The window's rendering is the clock (`main`'s rendering notifier, as
//! the retired nettai-demo's picture widget was): before each frame is drawn the battle
//! runs the ticks due and, if one ran (or the space changed), presents a
//! new picture, which that same frame draws. The picture is the size of the
//! frame's largest whole multiple that fits the space, in the display's
//! pixels ([`fit`]): presented at the window's density (or the display's,
//! sharper), drawn with the nearest pixel up to the display's. Nothing else
//! of the window is presented.

use nettai_frontend::player::Player;
use nettai_render::compose::{HEIGHT, WIDTH};
use slint::{Image, Rgb8Pixel, SharedPixelBuffer};
use std::time::{Duration, Instant};

/// How the picture fits a space: the display's pixels a frame pixel takes
/// (`scale`), and the pixels a frame pixel takes in the picture presented
/// (`density`, which divides `scale`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fit {
    pub scale: u32,
    pub density: u32,
}

/// The picture in a space of `width` by `height` display pixels on a
/// display of `factor` pixels a logical one: the largest whole multiple of
/// the frame that fits, at least the frame; presented at the window's
/// density (the scale over the factor, rounded up), or the display's when
/// `sharp`.
///
/// The density needn't divide the scale: drawn with the nearest pixel at
/// exactly `scale` display pixels a frame pixel, every frame pixel is a
/// square of `scale` display pixels whatever the density, as each display
/// pixel samples the presented pixel its center falls in, inside the frame
/// pixel's square of `density` presented ones. Only the text layer, drawn at
/// the density, is scaled by a fraction.
pub fn fit(width: f32, height: f32, factor: f32, sharp: bool) -> Fit {
    let scale = ((width / WIDTH as f32).min(height / HEIGHT as f32).floor() as u32).max(1);
    if sharp {
        return Fit { scale, density: scale };
    }
    let density = ((scale as f32 / factor.max(1.0)).ceil() as u32).clamp(1, scale);
    Fit { scale, density }
}

/// What the picture costs and how late it is (`NETTAI_PLAY_STATS`).
#[derive(Default)]
pub struct Stats {
    pub since: Option<Instant>,
    pub frames: u32,
    pub presented: u32,
    pub present: Duration,
    pub worst_present: Duration,
    pub worst_gap: Duration,
    /// The time between each two frames.
    pub gaps: Vec<Duration>,
    /// The ticks run, and the time running them took.
    pub ticks: u32,
    pub ticking: Times,
    /// The most sound queued for the device, in seconds (its latency).
    pub audio_queued: f64,
    /// From a key's event to the tick that saw it.
    pub key_to_tick: Times,
    /// From a key's event to the frame that drew its tick's picture
    /// (handed to the GPU: the rendering's end).
    pub key_to_drawn: Times,
    /// From a picture presented to its frame drawn.
    pub presented_to_drawn: Times,
}

#[derive(Clone, Copy, Default)]
pub struct Times {
    pub count: u32,
    pub total: Duration,
    pub worst: Duration,
}

impl Times {
    pub fn add(&mut self, t: Duration) {
        self.count += 1;
        self.total += t;
        self.worst = self.worst.max(t);
    }

    pub fn mean(&self) -> Duration {
        self.total / self.count.max(1)
    }
}

/// What [`Stage::picture`] did.
pub enum Shown {
    /// Nothing changed.
    Same,
    /// The picture was written into the image shown.
    Written,
    /// A new image to show.
    New(Image),
}

/// A battle being shown: the player, and its picture.
pub struct Stage {
    pub player: Player,
    last: Option<Instant>,
    /// The picture is out of date: a tick ran, or the space or the
    /// language changed.
    pub stale: bool,
    fit: Option<Fit>,
    buffer: Vec<u32>,
    pub samples: Vec<[f32; 2]>,
    pub stats: Option<Stats>,
    /// The first key a tick since the last picture saw, when it came; and
    /// the picture's, once presented, waiting to be drawn.
    seen: Option<Instant>,
    drawing: Option<(Instant, Option<Instant>)>,
}

impl Stage {
    pub fn new(player: Player) -> Stage {
        Stage {
            player,
            last: None,
            stale: true,
            fit: None,
            buffer: Vec::new(),
            samples: Vec::new(),
            stats: std::env::var_os("NETTAI_PLAY_STATS").map(|_| Stats::default()),
            seen: None,
            drawing: None,
        }
    }

    /// Run the ticks due at `now` with `buttons` held (`pressed`: when the
    /// presses since the last frame came; the tick that runs saw them).
    /// The ticks run.
    pub fn advance(&mut self, now: Instant, buttons: u16, pressed: &mut Vec<Instant>) -> u32 {
        let elapsed = self.last.map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        self.last = Some(now);
        let started = Instant::now();
        let ran = self.player.advance(elapsed, buttons);
        if let Some(s) = &mut self.stats {
            s.ticks += ran;
            if ran > 0 {
                s.ticking.add(started.elapsed());
            }
        }
        if ran > 0 {
            self.stale = true;
            if let Some(s) = &mut self.stats {
                for &at in pressed.iter() {
                    s.key_to_tick.add(at.elapsed());
                }
            }
            self.seen = self.seen.or(pressed.first().copied());
            pressed.clear();
        }
        if let Some(s) = &mut self.stats {
            s.frames += 1;
            s.worst_gap = s.worst_gap.max(elapsed);
            if elapsed > Duration::ZERO {
                s.gaps.push(elapsed);
            }
        }
        self.player.take_samples(&mut self.samples);
        ran
    }

    /// Forget the time that passed (the battle was paused by the app, or
    /// hidden): the next frame runs no ticks for it.
    pub fn hold(&mut self) {
        self.last = None;
    }

    /// The picture for `fit`, if it changed since the last, presented at
    /// its density into the window's texture (`gl`: written in place), or
    /// else into a new image. The image to show, when there is a new one
    /// (with a texture, only when it is made anew); `Shown::Same` when the
    /// picture didn't change.
    pub fn picture(&mut self, fit: Fit, gl: Option<&mut crate::gl::GlPicture>) -> Shown {
        if self.fit != Some(fit) {
            self.fit = Some(fit);
            self.stale = true;
        }
        if !self.stale {
            return Shown::Same;
        }
        self.stale = false;
        let started = Instant::now();
        let (w, h) = (WIDTH * fit.density as usize, HEIGHT * fit.density as usize);
        self.buffer.resize(w * h, 0);
        self.player.present(&mut self.buffer, w, h);
        let image = match gl {
            Some(gl) => gl.write(&mut self.buffer, w as u32, h as u32),
            None => {
                let mut pixels = SharedPixelBuffer::<Rgb8Pixel>::new(w as u32, h as u32);
                for (out, &px) in pixels.make_mut_slice().iter_mut().zip(&self.buffer) {
                    *out = Rgb8Pixel { r: (px >> 16) as u8, g: (px >> 8) as u8, b: px as u8 };
                }
                Some(Image::from_rgb8(pixels))
            }
        };
        self.drawing = Some((Instant::now(), self.seen.take()));
        if let Some(s) = &mut self.stats {
            let took = started.elapsed();
            s.presented += 1;
            s.present += took;
            s.worst_present = s.worst_present.max(took);
        }
        match image {
            Some(image) => Shown::New(image),
            None => Shown::Written,
        }
    }

    /// Show the picture again from the start (the window's texture was
    /// made anew for another picture).
    pub fn refresh(&mut self) {
        self.stale = true;
        self.fit = None;
    }

    /// The frame was drawn (handed to the GPU): the picture presented for
    /// it is on its way to the display.
    pub fn drawn(&mut self) {
        let Some((presented, key)) = self.drawing.take() else { return };
        if let Some(s) = &mut self.stats {
            s.presented_to_drawn.add(presented.elapsed());
            if let Some(key) = key {
                s.key_to_drawn.add(key.elapsed());
            }
        }
    }

    /// Every two seconds, what the picture cost and how late it was, said
    /// on the terminal (`NETTAI_PLAY_STATS`).
    pub fn report(&mut self, now: Instant) {
        let Some(s) = &mut self.stats else { return };
        let since = *s.since.get_or_insert(now);
        if now.duration_since(since) < Duration::from_secs(2) {
            return;
        }
        let fit = self.fit.unwrap_or(Fit { scale: 0, density: 0 });
        let seconds = now.duration_since(since).as_secs_f64();
        s.gaps.sort();
        let at = |q: f64| s.gaps.get(((s.gaps.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or_default();
        // (A frame that took half again the usual: a refresh missed.)
        let missed = s.gaps.iter().filter(|g| g.as_secs_f64() > at(0.5).as_secs_f64() * 1.5).count();
        eprintln!(
            "play stats: {:.1} frames/s (frame time p50 {:.2?} p95 {:.2?} p99 {:.2?} max {:.2?}, {missed} long), {:.2} ticks/s ({:.2?} a frame that ran them); \
             {} pictures ({}x{}, scale {} density {}): {:.2?} each (worst {:.2?}), drawn {:.2?} after (worst {:.2?}); \
             {} keys: {:.2?} to the tick (worst {:.2?}), {:.2?} to its picture drawn (worst {:.2?}); sound queued at most {:.0} ms",
            s.frames as f64 / seconds,
            at(0.5),
            at(0.95),
            at(0.99),
            s.worst_gap,
            s.ticks as f64 / seconds,
            s.ticking.mean(),
            s.presented,
            WIDTH as u32 * fit.density,
            HEIGHT as u32 * fit.density,
            fit.scale,
            fit.density,
            s.present / s.presented.max(1),
            s.worst_present,
            s.presented_to_drawn.mean(),
            s.presented_to_drawn.worst,
            s.key_to_tick.count,
            s.key_to_tick.mean(),
            s.key_to_tick.worst,
            s.key_to_drawn.mean(),
            s.key_to_drawn.worst,
            s.audio_queued * 1000.0,
        );
        *s = Stats { since: Some(now), ..Stats::default() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The picture takes the largest whole multiple of the frame that fits,
    /// presented at the window's density: on a display of two pixels a
    /// point, half the scale (each picture pixel drawn as four).
    #[test]
    fn the_picture_fits_by_whole_pixels() {
        assert_eq!(fit(1920.0, 1280.0, 2.0, false), Fit { scale: 8, density: 4 });
        assert_eq!(fit(2200.0, 1500.0, 2.0, false), Fit { scale: 9, density: 5 }, "an odd scale: half of it, rounded up");
        assert_eq!(fit(1920.0, 1280.0, 2.0, true), Fit { scale: 8, density: 8 });
        assert_eq!(fit(960.0, 640.0, 1.0, false), Fit { scale: 4, density: 4 });
        assert_eq!(fit(1440.0, 960.0, 1.5, false), Fit { scale: 6, density: 4 });
        assert_eq!(fit(2880.0, 1920.0, 3.0, false), Fit { scale: 12, density: 4 });
        // Smaller than the frame: the frame, cropped by the window.
        assert_eq!(fit(100.0, 100.0, 1.0, false), Fit { scale: 1, density: 1 });
    }

    /// Drawn with the nearest pixel, a picture of any density at least the
    /// frame's makes every frame pixel the same square of display pixels:
    /// each display pixel's center falls in a presented pixel of the frame
    /// pixel whose square it is in.
    #[test]
    fn every_frame_pixel_is_a_square_at_any_density() {
        for scale in 1..=12u32 {
            for density in 1..=scale {
                for x in 0..WIDTH as u32 * scale {
                    // (The presented pixel the display pixel's center samples.)
                    let presented = ((x as f64 + 0.5) * density as f64 / scale as f64).floor() as u32;
                    assert_eq!(presented / density, x / scale, "scale {scale} density {density}: display pixel {x}");
                }
            }
        }
    }
}
