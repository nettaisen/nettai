//! A replay as video. Three pieces, each usable without the others:
//!
//! - [`nut`]: the NUT container (FFmpeg's), written: raw frames and PCM
//!   sound in one stream on any `std::io::Write`. It knows nothing of
//!   replays.
//! - [`write`] and [`export`]: a replay played through a
//!   [`Viewer`] and written as such a stream: the whole replay or a part of
//!   it (a round, a range of ticks), from either console, the picture at a
//!   whole multiple of the frame's 240x160.
//! - [`mp4`]: `ffmpeg` run as a child that reads a NUT stream from a pipe
//!   and writes an MP4 (H.264 and AAC), into a file that takes its name
//!   only once it is whole. [`export_mp4`] is the two together. No encoder
//!   and no ffmpeg library is linked: without ffmpeg installed, a host can
//!   still save the NUT stream itself.
//!
//! **What a video holds.** One frame of the picture a tick, and the sound
//! the tick made, exactly as a viewer playing the part straight shows and
//! plays them: the frames are the ones [`Viewer::present`] gives, text
//! included, and the samples the ones [`Viewer::take_samples`] gives. The
//! picture's time base is a tick's exact length (4389/262144 s:
//! `nettai_frontend::player::TICK_SECONDS`) and the sound's a sample's
//! (1/32768 s), so the two keep step over any length. A part starts in the
//! state playing straight to it reaches, sound included (the viewer's
//! seek), so a part's frames and samples are the whole video's for those
//! ticks.
//!
//! **The picture is scaled here**, by whole multiples and the nearest
//! pixel, not left at 240x160 for ffmpeg: in the font text mode the
//! battle's words are drawn at the output's resolution over the scaled
//! frame, so only a frame presented at its final size has them sharp. (In
//! the original text mode the scaling is plain pixel repetition either
//! way.)
//!
//! **The same replay gives the same bytes.** The stream has no time of
//! day, no name and no encoder string in it, and the simulation, the
//! drawing and the sound are functions of the replay.
//!
//! ```ignore
//! // On a thread of its own: the progress goes to the UI, which may cancel.
//! let options = Options { scale: 3, view: None, part: Part::Round(2) };
//! let result = nettai_video::export_mp4(&game, &replay, &options, &mp4::Settings::default(), &path, &mut |p| {
//!     tx.send(p).ok();
//!     if canceled.load(Ordering::Relaxed) { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
//! });
//! // Err(Error::NoFfmpeg(_)) | Err(Error::FfmpegFailed { .. }) | Err(Error::Disk(_)) | Err(Error::Canceled) ...
//! ```

pub mod mp4;
pub mod nut;

use nettai_frontend::game::Loaded;
use nettai_frontend::player::TICK_SECONDS;
use nettai_frontend::replay::{Refused, Replay};
use nettai_frontend::viewer::{Speed, Viewer};
use nettai_render::compose::{HEIGHT, WIDTH};
use std::fmt;
use std::io::{self, Write};
use std::ops::{ControlFlow, Range};
use std::path::Path;

/// The largest multiple of the frame a video's picture is made at: 3840 by
/// 2560.
pub const MOST_SCALE: u32 = 16;

/// The ticks a viewer plays ahead between two reports of progress while it
/// gets to a part's start.
const REACH_SLICE: u32 = 600;

/// Which of a replay a video shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    /// All of it.
    Whole,
    /// One round, from 1.
    Round(u32),
    /// The ticks of this range, counted through the set from 0: what
    /// playing from the position `start` to the position `end` shows and
    /// sounds (a frame a tick: the picture each leaves).
    Ticks(Range<u32>),
}

impl Part {
    /// The part's ticks in a replay of `rounds` (each round's first tick)
    /// and `length` ticks: or why it has none.
    pub fn ticks(&self, rounds: &[u32], length: u32) -> Result<Range<u32>, Error> {
        let ticks = match self {
            Part::Whole => 0..length,
            Part::Round(n) => {
                let i = (*n as usize).checked_sub(1).filter(|&i| i < rounds.len());
                let i = i.ok_or_else(|| Error::NoSuchPart(format!("round {n} of a replay of {} rounds", rounds.len())))?;
                rounds[i]..rounds.get(i + 1).copied().unwrap_or(length)
            }
            Part::Ticks(range) => range.clone(),
        };
        if ticks.start >= ticks.end || ticks.end > length {
            return Err(Error::NoSuchPart(format!("ticks {} to {} of a replay of {length} ticks", ticks.start, ticks.end)));
        }
        Ok(ticks)
    }
}

/// What a video is made of ([`export`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// The picture's size as a multiple of the frame's 240x160, 1 to
    /// [`MOST_SCALE`].
    pub scale: u32,
    /// The side whose console is shown (none: the side that recorded the
    /// replay).
    pub view: Option<u8>,
    pub part: Part,
}

/// How far a video is, for whoever shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// The frames written, and how many the video has.
    pub written: u32,
    pub of: u32,
    /// The round being written, from 1, and the replay's rounds. (Before
    /// the first frame, while the replay is played up to the part's start,
    /// the part's first round with nothing written.)
    pub round: u32,
    pub rounds: u32,
}

impl Progress {
    /// The part of the video written, 0 to 1.
    pub fn fraction(&self) -> f32 {
        self.written as f32 / self.of.max(1) as f32
    }
}

/// Why a video wasn't made. The variant is what a host tells its user;
/// what it says (`Display`) is the detail, for a log.
#[derive(Debug)]
pub enum Error {
    /// The replay isn't played here (another game's, another version's, a
    /// damaged match).
    Replay(Refused),
    /// The part asked for isn't in the replay: a round it hasn't, a range
    /// of no ticks or past its end.
    NoSuchPart(String),
    /// The picture's scale isn't 1 to [`MOST_SCALE`].
    Scale(u32),
    /// The battle stopped before the part's end: the engine stopped at
    /// this tick (the replay doesn't play past it).
    Stopped { tick: u32, why: String },
    /// Writing the stream failed (the sink's own error: a file's disk, a
    /// pipe).
    Write(io::Error),
    /// The file couldn't be made, or the disk is full.
    Disk(io::Error),
    /// `ffmpeg` couldn't be started: it isn't installed, or isn't where
    /// the settings say.
    NoFfmpeg(io::Error),
    /// `ffmpeg` ran and failed: its exit code (none: a signal ended it)
    /// and the last of what it wrote to its standard error.
    FfmpegFailed { code: Option<i32>, log: String },
    /// The caller canceled it. Nothing is left of the file.
    Canceled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Replay(why) => write!(f, "the replay isn't played: {why}"),
            Error::NoSuchPart(what) => write!(f, "no {what}"),
            Error::Scale(scale) => write!(f, "a picture {scale} times the frame's size (1 to {MOST_SCALE})"),
            Error::Stopped { tick, why } => write!(f, "the replay stopped at tick {tick}: {why}"),
            Error::Write(e) => write!(f, "the video couldn't be written: {e}"),
            Error::Disk(e) => write!(f, "the video's file couldn't be written: {e}"),
            Error::NoFfmpeg(e) => write!(f, "ffmpeg couldn't be started: {e}"),
            Error::FfmpegFailed { code: Some(code), log } => write!(f, "ffmpeg failed (exit code {code}): {}", log.trim()),
            Error::FfmpegFailed { code: None, log } => write!(f, "ffmpeg was stopped: {}", log.trim()),
            Error::Canceled => write!(f, "canceled"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Replay(e) => Some(e),
            Error::Write(e) | Error::Disk(e) | Error::NoFfmpeg(e) => Some(e),
            _ => None,
        }
    }
}

/// What a video's progress is told to, and what it answers: go on, or
/// stop (the video is canceled).
pub type OnProgress<'a> = &'a mut dyn FnMut(Progress) -> ControlFlow<()>;

/// Write `part` of the replay `viewer` plays to `out` as a NUT stream, the
/// picture `scale` times the frame's size, from the console the viewer
/// shows, with its sound if it has any. The viewer is left paused at the
/// part's end, at the original's speed. `progress` is told before the
/// first frame and after each, and may stop it ([`Error::Canceled`]: what
/// was written of the stream stays in `out`, which is the caller's to
/// throw away).
pub fn write<W: Write>(viewer: &mut Viewer, scale: u32, part: &Part, out: W, progress: OnProgress) -> Result<W, Error> {
    if !(1..=MOST_SCALE).contains(&scale) {
        return Err(Error::Scale(scale));
    }
    let ticks = part.ticks(viewer.rounds(), viewer.length())?;
    let rounds = viewer.rounds().len() as u32;
    let mut at = Progress { written: 0, of: ticks.end - ticks.start, round: viewer.round_at(ticks.start), rounds };
    let mut tell = |at: Progress| if progress(at).is_break() { Err(Error::Canceled) } else { Ok(()) };
    tell(at)?;
    // To the part's start, as playing straight there leaves the viewer: a
    // slice at a time, so a long way is told of and can be canceled.
    viewer.set_paused(true);
    viewer.set_speed(Speed::Normal);
    while viewer.indexed() < ticks.start && !viewer.index(REACH_SLICE) {
        tell(at)?;
    }
    viewer.seek(ticks.start);
    let stopped = |viewer: &Viewer| Error::Stopped { tick: viewer.position(), why: viewer.stopped().unwrap_or("the replay ends there").to_string() };
    if viewer.position() != ticks.start {
        return Err(stopped(viewer));
    }
    let (width, height) = (WIDTH * scale as usize, HEIGHT * scale as usize);
    let video = nut::Video { width: width as u32, height: height as u32, frame_time: TICK_SECONDS };
    let audio = viewer.has_sound().then_some(nut::Audio { sample_rate: nettai_audio::SAMPLE_RATE });
    let mut stream = nut::Writer::new(out, video, audio).map_err(Error::Write)?;
    let mut picture = vec![0u32; width * height];
    let mut rgb = Vec::with_capacity(width * height * 3);
    let mut sound = Vec::new();
    for tick in ticks.clone() {
        at.round = viewer.round_at(tick);
        if !viewer.step_forward() {
            return Err(stopped(viewer));
        }
        viewer.present(&mut picture, width, height);
        rgb.clear();
        rgb.extend(picture.iter().flat_map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]));
        sound.clear();
        viewer.take_samples(&mut sound);
        stream.frame(&rgb, &sound).map_err(Error::Write)?;
        at.written += 1;
        tell(at)?;
    }
    stream.finish().map_err(Error::Write)
}

/// Write a replay to `out` as a NUT stream ([`write`]), on a loaded game:
/// its graphics, its text as it was loaded, its sound if it was. It plays
/// the replay through a viewer of its own, on the thread it is called on
/// (a host calls it on one that isn't its window's).
pub fn export<W: Write>(game: &Loaded, replay: &Replay, options: &Options, out: W, progress: OnProgress) -> Result<W, Error> {
    let mut viewer = Viewer::open(game, replay, options.view).map_err(Error::Replay)?;
    write(&mut viewer, options.scale, &options.part, out, progress)
}

/// Write a replay as an MP4 at `to`, through `ffmpeg` ([`mp4::Encoder`]):
/// the file is there, whole, only when this returns `Ok`; canceled or
/// failed, nothing of it is left.
pub fn export_mp4(game: &Loaded, replay: &Replay, options: &Options, encoding: &mp4::Settings, to: &Path, progress: OnProgress) -> Result<(), Error> {
    let mut viewer = Viewer::open(game, replay, options.view).map_err(Error::Replay)?;
    write_mp4(&mut viewer, options.scale, &options.part, encoding, to, progress)
}

/// [`write`], to an MP4 at `to` through `ffmpeg`.
pub fn write_mp4(viewer: &mut Viewer, scale: u32, part: &Part, encoding: &mp4::Settings, to: &Path, progress: OnProgress) -> Result<(), Error> {
    // (What can't be made is said before ffmpeg is started.)
    if !(1..=MOST_SCALE).contains(&scale) {
        return Err(Error::Scale(scale));
    }
    part.ticks(viewer.rounds(), viewer.length())?;
    let mut encoder = mp4::Encoder::start(to, encoding)?;
    match write(viewer, scale, part, &mut encoder, progress) {
        Ok(_) => encoder.finish(),
        // (A pipe that closed is ffmpeg having stopped: why is its to say.)
        Err(Error::Write(e)) => Err(encoder.failed(e)),
        Err(other) => {
            encoder.cancel();
            Err(other)
        }
    }
}

// A game loaded is shared between threads, so a host exports on a thread
// of its own with a reference to what its window plays from.
const _: () = {
    const fn sync<T: Sync>() {}
    sync::<Loaded>();
};

#[cfg(test)]
mod tests;
