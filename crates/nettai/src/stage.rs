//! A battle on the screen: the library's player, driven a frame at a time,
//! and the picture it presents, as the window shows it.
//!
//! The window's rendering is the clock (`main`'s rendering notifier, as
//! the retired nettai-demo's picture widget was): before each frame is drawn the battle
//! runs the ticks due and, if one ran (or the space changed), presents a
//! new picture, which that same frame draws. The picture is the size of the
//! frame's largest whole multiple that fits the space, in the display's
//! pixels, and in the enhanced mode the frame as wide as the space at that
//! multiple ([`fit`]): presented at the window's density (or the display's,
//! sharper), drawn with the nearest pixel up to the display's. Nothing else
//! of the window is presented.

use nettai_frontend::player::Player;
use nettai_render::compose::{HEIGHT, Size, WIDTH};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::time::Duration;
use web_time::Instant;

/// How the picture fits a space: the display's pixels a frame pixel takes
/// (`scale`), the pixels a frame pixel takes in the picture presented
/// (`density`, which divides `scale`), and the frame's columns (`columns`:
/// the original's 240, or in the enhanced mode as many as the space has at
/// the scale, `nettai_render::compose::Size`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fit {
    pub scale: u32,
    pub density: u32,
    pub columns: u32,
}

impl Fit {
    /// The same fit of a frame of the original's 240 columns (a picture that
    /// stays the original's size: a replay's still, the viewer's inset).
    pub fn original(self) -> Fit {
        Fit { columns: WIDTH as u32, ..self }
    }

    /// The frame it is a picture of.
    pub fn size(self) -> Size {
        Size::new(self.columns as usize, HEIGHT)
    }

    /// The picture presented, in its pixels: the frame `density` times.
    pub fn presented(self) -> (usize, usize) {
        (self.columns as usize * self.density as usize, HEIGHT * self.density as usize)
    }
}

/// The frame's columns in a space `width` display pixels wide at `scale`:
/// in the enhanced mode as many whole ones as it has (240 at least), in the
/// original mode the original's.
pub fn columns(width: f32, scale: u32, enhanced: bool) -> u32 {
    match enhanced {
        true => ((width / scale.max(1) as f32).floor() as u32).max(WIDTH as u32),
        false => WIDTH as u32,
    }
}

/// The picture in a space of `width` by `height` display pixels on a
/// display of `factor` pixels a logical one: the largest whole multiple of
/// the frame that fits, at least the frame, and in the enhanced mode the
/// frame as wide as the space at that multiple ([`columns`]); presented at
/// the window's density (the scale over the factor, rounded up), or the
/// display's when `sharp`.
///
/// What is presented is drawn on the CPU and scaled the rest of the way by
/// the window, with the nearest pixel. So the `enhanced` mode's density is
/// held to [`MOST_DENSITY`] (a 1440x960 picture: on a 4K display it was 9
/// to 13 times the frame, a picture 4 to 7 times the work, every frame), and
/// the original mode's, which has nothing finer than the frame's pixels to
/// draw, is 1: the window's scaling is the same picture.
///
/// The density needn't divide the scale: drawn with the nearest pixel at
/// exactly `scale` display pixels a frame pixel, every frame pixel is a
/// square of `scale` display pixels whatever the density, as each display
/// pixel samples the presented pixel its center falls in, inside the frame
/// pixel's square of `density` presented ones. Only the text layer, drawn at
/// the density, is scaled by a fraction.
pub fn fit(width: f32, height: f32, factor: f32, sharp: bool, enhanced: bool) -> Fit {
    let scale = ((width / WIDTH as f32).min(height / HEIGHT as f32).floor() as u32).max(1);
    let columns = columns(width, scale, enhanced);
    if !enhanced {
        return Fit { scale, density: 1, columns };
    }
    if sharp {
        return Fit { scale, density: scale, columns };
    }
    let density = ((scale as f32 / factor.max(1.0)).ceil() as u32).clamp(1, scale).min(MOST_DENSITY);
    Fit { scale, density, columns }
}

/// The densest the enhanced mode's picture is presented (but `sharp`): six
/// times the frame, 1440x960, about a 1080p display's picture (the user,
/// on a 4K display's stutter: "do that", of holding it there and letting the
/// window scale the rest).
pub const MOST_DENSITY: u32 = 6;

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

/// A picture on the window: the buffer something presents itself into (a battle's player, a replay's viewer),
/// how it fits its room, and whether it is out of date. The window has one picture: the image of the surface
/// last presented (`Shell::show_surface`), shown only while some surface is ([`Showing`]).
pub struct Surface {
    /// Which surface this is, of those made: another one shown after it presents its own picture whether or
    /// not anything changed.
    pub id: u64,
    /// The picture is out of date: a tick ran, or the room or the language changed.
    pub stale: bool,
    fit: Option<Fit>,
    buffer: Vec<u32>,
    /// The images' pixels: two, written in turn. The one written last is the image the window shows while the
    /// other is written, which the window let go of when it drew the last picture, so writing it allocates
    /// nothing. (Were an image of it still held, Slint copies it first rather than write under it.)
    pixels: [SharedPixelBuffer<Rgba8Pixel>; 2],
    turn: usize,
}

impl Default for Surface {
    fn default() -> Surface {
        Surface::new()
    }
}

impl Surface {
    pub fn new() -> Surface {
        static MADE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Surface {
            id: MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            stale: true,
            fit: None,
            buffer: Vec::new(),
            pixels: [SharedPixelBuffer::new(0, 0), SharedPixelBuffer::new(0, 0)],
            turn: 0,
        }
    }

    /// The fit last presented at.
    pub fn fit(&self) -> Option<Fit> {
        self.fit
    }

    /// Show the picture again from the start (the window's picture is another surface's).
    pub fn refresh(&mut self) {
        self.stale = true;
        self.fit = None;
    }

    /// The picture for `fit`, if it changed since the last: `present` draws a frame as wide as it is given
    /// into a buffer of the width and height it is given, which becomes a new image (an image is what every
    /// renderer draws: docs/app.md §1), its pixels the next of the two kept. None when the picture didn't change.
    pub fn picture(&mut self, fit: Fit, present: impl FnOnce(Size, &mut [u32], usize, usize)) -> Option<Image> {
        if self.fit != Some(fit) {
            self.fit = Some(fit);
            self.stale = true;
        }
        if !self.stale {
            return None;
        }
        self.stale = false;
        let (w, h) = fit.presented();
        self.buffer.resize(w * h, 0);
        present(fit.size(), &mut self.buffer, w, h);
        let pixels = &mut self.pixels[self.turn];
        self.turn ^= 1;
        if (pixels.width(), pixels.height()) != (w as u32, h as u32) {
            *pixels = SharedPixelBuffer::new(w as u32, h as u32);
        }
        rgba(&self.buffer, pixels.make_mut_slice());
        Some(Image::from_rgba8_premultiplied(pixels.clone()))
    }
}

/// An image of `w` by `h` presented pixels (0x00RRGGBB, as `present` writes them), of its own (a replay's
/// preview, kept).
pub fn image_of(buffer: &[u32], w: usize, h: usize) -> Image {
    let mut pixels = SharedPixelBuffer::<Rgba8Pixel>::new(w as u32, h as u32);
    rgba(buffer, pixels.make_mut_slice());
    Image::from_rgba8_premultiplied(pixels)
}

/// Presented pixels (0x00RRGGBB) as opaque RGBA, which every renderer uploads as it is (wgpu has no RGB format,
/// and OpenGL ES takes RGBA from any driver); opaque, it is the same premultiplied, which Skia keeps.
fn rgba(buffer: &[u32], out: &mut [Rgba8Pixel]) {
    for (out, &px) in out.iter_mut().zip(buffer) {
        *out = Rgba8Pixel { r: (px >> 16) as u8, g: (px >> 8) as u8, b: px as u8, a: 0xFF };
    }
}

/// The window's one picture, which every `Picture` shows: which surface's image it is, and whether
/// it is shown. It is shown only in a frame in which some surface was (a battle, the menu's reel, a replay's
/// still or its viewer), and not past a change of what the window shows (another screen, a battle ended): a
/// screen with nothing of its own to present (a replay that can't be played, a still not made yet, a screen
/// just arrived) shows its `Picture`'s ground, never what another presented last.
#[derive(Default)]
pub struct Showing {
    /// The surface whose image the picture is (0: none).
    holds: u64,
    /// A surface was shown since the frame began and since what the window shows last changed.
    used: bool,
    /// Whether the UI was last told the picture is shown.
    told: Option<bool>,
}

impl Showing {
    /// Nothing is shown (yet): a frame begins, or what the window shows changed under the picture.
    pub fn reset(&mut self) {
        self.used = false;
    }

    /// `surface` is shown this frame: presented anew where the picture is another's.
    pub fn show(&mut self, surface: &mut Surface) {
        if self.holds != surface.id {
            self.holds = surface.id;
            surface.refresh();
        }
        self.used = true;
    }

    /// Whether the picture is shown, where that changed since the UI was last told.
    pub fn tell(&mut self) -> Option<bool> {
        (self.told != Some(self.used)).then(|| {
            self.told = Some(self.used);
            self.used
        })
    }
}

/// A battle being shown: the player, and its picture.
pub struct Stage {
    pub player: Player,
    pub surface: Surface,
    last: Option<Instant>,
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
            surface: Surface::new(),
            last: None,
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
        // (A battle held where it stands sounds its ticks and draws its scenery on: its picture is new too, but
        // no key reached it.)
        if ran > 0 {
            self.surface.stale = true;
        }
        if ran > 0 && !self.player.held() {
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

    /// The picture for `fit`, if it changed since the last ([`Surface::picture`]), with what it cost noted for
    /// the latency figures.
    pub fn picture(&mut self, fit: Fit) -> Option<Image> {
        let started = Instant::now();
        let player = &mut self.player;
        let shown = self.surface.picture(fit, |size, buffer, w, h| {
            player.renderer().size = size;
            player.present(buffer, w, h)
        });
        if shown.is_none() {
            return shown;
        }
        self.drawing = Some((Instant::now(), self.seen.take()));
        if let Some(s) = &mut self.stats {
            let took = started.elapsed();
            s.presented += 1;
            s.present += took;
            s.worst_present = s.worst_present.max(took);
        }
        shown
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
        let fit = self.surface.fit().unwrap_or(Fit { scale: 0, density: 0, columns: 0 });
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
            fit.columns * fit.density,
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
        let enhanced = |w, h, factor, sharp| fit(w, h, factor, sharp, true);
        let at = |scale, density| Fit { scale, density, columns: WIDTH as u32 };
        assert_eq!(enhanced(1920.0, 1280.0, 2.0, false), at(8, 4));
        assert_eq!(enhanced(2200.0, 1500.0, 2.0, false), Fit { columns: 244, ..at(9, 5) }, "an odd scale: half of it, rounded up");
        assert_eq!(enhanced(1920.0, 1280.0, 2.0, true), at(8, 8));
        assert_eq!(enhanced(960.0, 640.0, 1.0, false), at(4, 4));
        assert_eq!(enhanced(1440.0, 960.0, 1.5, false), at(6, 4));
        assert_eq!(enhanced(2880.0, 1920.0, 3.0, false), at(12, 4));
        // Smaller than the frame: the frame, cropped by the window.
        assert_eq!(enhanced(100.0, 100.0, 1.0, false), at(1, 1));
        // A 4K display, at 150% and at 100%: held to six times the frame (but sharp); the frame as wide as the
        // display at 13 times, 295 columns.
        assert_eq!(enhanced(3840.0, 2160.0, 1.5, false), Fit { columns: 295, ..at(13, MOST_DENSITY) });
        assert_eq!(enhanced(3840.0, 2160.0, 1.0, false), Fit { columns: 295, ..at(13, MOST_DENSITY) });
        assert_eq!(enhanced(3840.0, 2160.0, 1.0, true), Fit { columns: 295, ..at(13, 13) });
        // The original mode: the frame's own pixels, the window scaling them, its 240 columns.
        assert_eq!(fit(3840.0, 2160.0, 1.5, false, false), at(13, 1));
        assert_eq!(fit(1920.0, 1280.0, 2.0, true, false), at(8, 1));
    }

    /// The enhanced mode's frame is as wide as the space at the largest whole multiple that fits: a 16:9 window,
    /// a phone held sideways (19.5:9), an ultrawide one; a space narrower than 3:2 (a phone held upright) has the
    /// original's 240 columns, the picture's bars above and below.
    #[test]
    fn the_enhanced_frame_is_as_wide_as_the_space() {
        let enhanced = |w, h| fit(w, h, 1.0, true, true);
        assert_eq!(enhanced(1920.0, 1080.0), Fit { scale: 6, density: 6, columns: 320 });
        assert_eq!(enhanced(2532.0, 1170.0), Fit { scale: 7, density: 7, columns: 361 });
        assert_eq!(enhanced(3440.0, 1440.0), Fit { scale: 9, density: 9, columns: 382 });
        assert_eq!(enhanced(1170.0, 2532.0), Fit { scale: 4, density: 4, columns: 292 });
        assert_eq!(enhanced(720.0, 1600.0), Fit { scale: 3, density: 3, columns: 240 });
        assert_eq!(enhanced(1920.0, 1080.0).presented(), (1920, 960));
        assert_eq!(enhanced(1920.0, 1080.0).original().presented(), (1440, 960));
    }

    /// A surface's pictures are written into two buffers in turn: once the window let go of a picture's image,
    /// the picture after the next is written into its pixels, nothing allocated; an image still held is never
    /// written under (Slint copies its pixels first).
    #[test]
    fn a_surfaces_pictures_are_written_into_two_buffers_in_turn() {
        let mut surface = Surface::new();
        let fit = Fit { scale: 1, density: 1, columns: WIDTH as u32 };
        let next = |surface: &mut Surface, n: u32| {
            surface.stale = true;
            surface.picture(fit, |_, buffer, _, _| buffer.fill(n)).expect("a stale picture is presented")
        };
        let pixels = |image: &Image| image.to_rgba8_premultiplied().expect("an image of pixels");
        let (first, second) = (next(&mut surface, 1), next(&mut surface, 2));
        let (at_first, at_second) = (pixels(&first).as_slice().as_ptr(), pixels(&second).as_slice().as_ptr());
        assert_ne!(at_first, at_second);
        // The window drew the second: the first is let go, and the third is written into its pixels.
        drop(first);
        let third = next(&mut surface, 3);
        assert_eq!(pixels(&third).as_slice().as_ptr(), at_first, "the first's pixels, nothing allocated");
        assert_eq!(pixels(&third).as_slice()[0], Rgba8Pixel { r: 0, g: 0, b: 3, a: 0xFF });
        // The second still held: the fourth is written elsewhere, and the second is as it was.
        let fourth = next(&mut surface, 4);
        assert_ne!(pixels(&fourth).as_slice().as_ptr(), at_second);
        assert_eq!(pixels(&second).as_slice()[0], Rgba8Pixel { r: 0, g: 0, b: 2, a: 0xFF });
        assert_eq!(pixels(&fourth).as_slice()[0], Rgba8Pixel { r: 0, g: 0, b: 4, a: 0xFF });
    }

    /// The window's picture is shown only in a frame in which some surface was: after the menu's reel, a screen
    /// with nothing of its own (Replays with a replay that can't be played selected) shows none of the reel. A
    /// surface shown after another's is presented anew; one shown again after frames with none isn't (the
    /// picture is still its own). A change of what the window shows (another screen, a battle ended) takes the
    /// picture away in the frame it comes, whatever was shown before it in that frame. The UI is told only of a
    /// change.
    #[test]
    fn the_picture_is_shown_only_in_a_frame_something_was_shown_in() {
        let (mut showing, mut reel, mut still) = (Showing::default(), Surface::new(), Surface::new());
        // (A surface presented: as `Surface::picture` leaves it.)
        let presented = |s: &mut Surface| s.stale = false;
        // The menu, its reel shown.
        showing.reset();
        showing.show(&mut reel);
        presented(&mut reel);
        assert_eq!(showing.tell(), Some(true));
        showing.reset();
        showing.show(&mut reel);
        assert_eq!((reel.stale, showing.tell()), (false, None), "the same picture, nothing to tell");
        // Replays shows, in the frame the reel was shown in: the reel's picture is taken away at once.
        showing.reset();
        assert_eq!(showing.tell(), Some(false));
        // Frames of the list with a replay that can't be played selected: nothing is shown.
        for _ in 0..3 {
            showing.reset();
            assert_eq!(showing.tell(), None);
        }
        // Another is selected and its still comes: presented anew (the picture was the reel's), and shown.
        showing.reset();
        showing.show(&mut still);
        assert!(still.stale);
        presented(&mut still);
        assert_eq!(showing.tell(), Some(true));
        // A frame without it, then it again: shown again as it is, the picture still its own.
        showing.reset();
        assert_eq!(showing.tell(), Some(false));
        showing.reset();
        showing.show(&mut still);
        assert_eq!((still.stale, showing.tell()), (false, Some(true)));
        // The reel shown after it is presented anew.
        showing.reset();
        showing.show(&mut reel);
        assert!(reel.stale);
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
