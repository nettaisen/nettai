//! A replay as video: played through a [`Viewer`] frame by frame, and made an MP4 the same way on every platform.
//! The pieces:
//!
//! - The playing: a replay through a [`Viewer`], the whole of it or a part (a round, a range of ticks), from either
//!   console, both on one screen, or both side by side ([`Sides`]), the picture at a whole multiple of the frame's
//!   240x160, a frame and its sound at a time.
//! - [`encode`]: the encoder, the platform's: `ffmpeg` on the desktop (a child a track, written its raw frames or
//!   samples, which it encodes and says the packets of back), the browser's own in a browser
//!   (WebCodecs), the phone's own on a phone (iOS's VideoToolbox and AudioToolbox, Android's MediaCodec, each
//!   given the frames as NV12 made here: `yuv`). Each is given the same frames and the same samples, and makes
//!   the picture's H.264 and a sound track's AAC (or the browser's Opus) a console shown.
//! - `resample`: the sound made 48 kHz, the rate every encoder's codecs take, before any is given it.
//! - [`mp4`]: the MP4 of what the encoder makes, its samples written to any `std::io::Write` a chunk at a time as
//!   they are made (a file; in a browser, the file being made in the page's storage) and its head, the index of
//!   them, made last, for the host to put before them: its tracks, its sound tracks' titles and its rounds as
//!   chapters. No fragmented MP4: one every player and editor takes, that tells its length and seeks at once.
//! - [`export`] and [`write_mp4`]: the three together, frames to the encoder to the MP4.
//! - `presenters`: the frames' pictures made at the video's size on threads of their own, beside the playing.
//!
//! **What a video holds.** One frame of the picture a tick, and the sound
//! the tick made, exactly as a viewer playing the part straight shows and
//! plays them: the frames are the ones [`Viewer::present`] gives, text
//! included (made on threads of their own, the same: below), and the
//! samples the ones [`Viewer::take_samples`] gives, made 48 kHz
//! (`resample`: the consoles' 32768 Hz is a rate neither AAC nor Opus
//! has). The picture's time base is a tick's exact length (4389/262144 s:
//! `nettai_frontend::player::TICK_SECONDS`) and the sound's a sample's
//! (1/48000 s), each sample at an exact place in the consoles' (375 for
//! every 256), so the two keep step over any length. A part starts in the
//! state playing straight to it reaches, sound included (the viewer's
//! seek), so a part's frames and samples are the whole video's for those
//! ticks.
//!
//! **Its tracks and chapters.** The sound is a track a console shown: the
//! console's own, as [`Viewer::take_samples_of`] gives it, titled with its
//! player's name ([`Titles`]); the first is the default. Both consoles on
//! one screen are one track, both consoles' sound as the viewer shown so
//! plays it ([`Viewer::take_samples`]), titled with both names. Each round is a
//! chapter, titled in the host's words, from the frame its picture begins
//! on ([`chapters`]). They are said in the stream's info packets
//! ([`Info`]), and the MP4's ([`mp4`]).
//!
//! **The picture is scaled here**, by whole multiples and the nearest
//! pixel, not left at 240x160 for ffmpeg: in the enhanced mode the
//! battle's words are drawn at the output's resolution over the scaled
//! frame, so only a frame presented at its final size has them sharp. (In
//! the original mode the scaling is plain pixel repetition either
//! way.)
//!
//! **The same replay gives the encoder the same frames and samples.** The
//! simulation, the drawing, the sound and its resampling are functions of
//! the replay.
//!
//! **Three things at once** (the user: "is replay export slower than it
//! should be?"). The thread a video is made on plays the battle, a tick at
//! a time in order, and draws each tick's frame at 240x160
//! (`Viewer::frame_view`); the picture at the video's size, scaled with the
//! enhanced mode's text over it, is made on a few threads beside it, a
//! whole frame each, each with a text renderer of its own, and comes back
//! in order (`presenters`); and on the desktop ffmpeg is written from a
//! thread of its own through a queue of a few frames, so the drawing never
//! waits on ffmpeg's pipe and ffmpeg never waits on the drawing (`ffmpeg`).
//! Measured on a 19:42 EXE5 replay's first two minutes (7,200 frames at four
//! times the frame, release builds, `nettai-tool --mp4`): the MP4 in 20 s
//! (it took 34 s when one thread did all three in turns), as fast as ffmpeg
//! alone encodes the same frames; a frame drawn in 0.66 ms, from 3.5 (the
//! frames alone, then written as a NUT stream by `nettai-tool --nut`, since
//! gone). The MP4 was byte for byte what it had been. The frames are presented
//! one way: the window's `present`, on these threads, the same on the
//! desktop and in a browser (whose threads are Web Workers); where they
//! don't start, no video is made ([`Error::Threads`]).
//!
//! ```ignore
//! // On a thread of its own: the progress goes to the UI, which may cancel. `to` is the file (a dialog's).
//! let titles = Titles { rounds: vec!["Round 1".into(), "Round 2".into()], sides: Default::default() };
//! let options = Options { scale: 3, view: None, sides: Sides::One, part: Part::Round(2), titles };
//! let result = nettai_video::export(&game, &replay, &options, &Encoding::default(), to, &mut |p| {
//!     tx.send(p).ok();
//!     if canceled.load(Ordering::Relaxed) { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
//! });
//! // Err(Error::NoFfmpeg(_)) | Err(Error::FfmpegFailed { .. }) | Err(Error::Encoder(_)) | Err(Error::Canceled) ...
//! ```

// (Unsafe only where iOS's frameworks are called: `videotoolbox`.)
#![deny(unsafe_code)]

pub mod encode;
pub mod mp4;
pub use mp4::Made;
mod presenters;

// The platform's encoder, each with its `start`: ffmpeg on the desktop (a child a track), the
// browser's own in a browser, the phone's own on a phone (iOS's VideoToolbox and AudioToolbox,
// Android's MediaCodec), each given the same frames and samples. (macOS has iOS's frameworks:
// the tests run `videotoolbox` there.)
#[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
pub mod ffmpeg;
#[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
use ffmpeg as platform;
#[cfg(target_arch = "wasm32")]
mod webcodecs;
#[cfg(target_arch = "wasm32")]
use webcodecs as platform;
#[cfg(any(target_os = "ios", all(test, target_os = "macos")))]
#[allow(unsafe_code)]
mod videotoolbox;
#[cfg(target_os = "ios")]
use videotoolbox as platform;
#[cfg(target_os = "android")]
mod mediacodec;
#[cfg(target_os = "android")]
use mediacodec as platform;
// (A phone's encoder takes YUV, made here: the same bytes on both.)
#[cfg(any(target_os = "ios", target_os = "android", test))]
mod yuv;
// (Android's encoder makes H.264 as Annex B, made the MP4's here.)
#[cfg(any(target_os = "android", test))]
mod annex_b;
mod resample;

pub use encode::{Audio, Chapter, Chroma, Encoding, Info, Video};

use nettai_frontend::game::Loaded;
use nettai_frontend::player::TICK_SECONDS;
use nettai_frontend::replay::{Info as ReplayInfo, Refused, Replay};
use nettai_frontend::viewer::{Speed, Viewer};
use nettai_render::compose::{HEIGHT, WIDTH};
use presenters::{Presented, Presenters};
use resample::Resampler;
use std::fmt;
use std::io::{self, Write};
use std::ops::{ControlFlow, Range};

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

/// Which consoles a video's picture shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sides {
    /// The console the viewer shows, and its sound.
    #[default]
    One,
    /// Both consoles on one screen (`Viewer::set_both`): the field as the
    /// console the viewer shows draws it, at the left, each player's own
    /// pieces on its own side; and one sound track, both consoles' (the
    /// left console's sound with the right side's own effects over it, as
    /// the viewer shown so plays it).
    Both,
    /// Both consoles side by side in a picture twice as wide: the one the
    /// viewer shows at the left, the other at the right, each as a viewer
    /// shown from its side presents it at the tick (one simulation: the
    /// other is drawn from the same battle, `Viewer::present_view`). Each
    /// console's sound is a track of its own, the left one's first (the
    /// default) and the right one's second, each what that console played
    /// (`Viewer::take_samples_of`): not a mix, since the two hear one
    /// battle, the same music and nearly all the same effects, so the two
    /// together would be every shared sound twice as loud, and at the set's
    /// end the winner's music over the loser's; apart, what only one hears
    /// (its own charge, its own navi's hit) is in its own track. (A width
    /// twice an even one is even: nothing changes for 4:2:0, `mp4`.)
    SideBySide,
}

impl Sides {
    /// The pictures side by side, and the sound tracks.
    pub fn count(self) -> usize {
        match self {
            Sides::One | Sides::Both => 1,
            Sides::SideBySide => 2,
        }
    }
}

/// What a video is made of ([`export`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// The picture's size as a multiple of the frame's 240x160, 1 to
    /// [`MOST_SCALE`] (each side's, with both).
    pub scale: u32,
    /// The side whose console is shown (none: the side that recorded the
    /// replay); with both sides, the one at the left, whose sound is the
    /// first track.
    pub view: Option<u8>,
    pub sides: Sides,
    pub part: Part,
    pub titles: Titles,
}

/// What a video calls its chapters and its sound tracks, in its host's
/// words (the app's, in the language it shows).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Titles {
    /// Each round's chapter, by round from 1; an empty one, or one past
    /// these, is "Round N".
    pub rounds: Vec<String>,
    /// The track of each side's console's sound, by side; an empty one is
    /// the player's name as the replay has it, else "Player 1" or
    /// "Player 2".
    pub sides: [String; 2],
}

impl Titles {
    /// Round `round`'s chapter (from 1).
    pub fn round(&self, round: u32) -> String {
        let given = (round as usize).checked_sub(1).and_then(|i| self.rounds.get(i)).filter(|t| !t.trim().is_empty());
        given.cloned().unwrap_or_else(|| format!("Round {round}"))
    }

    /// The track of both consoles' sound on one screen, `left`'s at the left:
    /// both players' tracks' titles.
    pub fn both(&self, left: u8, info: &ReplayInfo) -> String {
        format!("{} & {}", self.side(left, info), self.side(left ^ 1, info))
    }

    /// The track of `side`'s sound, in a replay that says `info` of itself.
    pub fn side(&self, side: u8, info: &ReplayInfo) -> String {
        let side = side.min(1) as usize;
        [self.sides[side].trim(), info.names[side].trim()]
            .into_iter()
            .find(|t| !t.is_empty())
            .map_or_else(|| format!("Player {}", side + 1), str::to_string)
    }
}

/// The chapters of a video of `ticks` of a replay whose rounds start at
/// `rounds` (each one's first tick, the first round's 0): a round each,
/// titled by `titles`, from the frame its picture begins on to the next
/// one's (the frames counted from the video's first, a tick's each). A
/// round's picture begins on the frame of the tick before its first: the
/// tick that ends a round shows the next one at its start (what is heard
/// of it is still the round it ended). The video's first frame is its
/// round's; a round whose picture begins on the video's last frame has no
/// chapter (there is nothing of it to go to: a round's part ends so).
pub fn chapters(rounds: &[u32], ticks: &Range<u32>, titles: &Titles) -> Vec<Chapter> {
    let frames = ticks.end.saturating_sub(ticks.start);
    let mut out: Vec<Chapter> = Vec::new();
    for (i, &start) in rounds.iter().enumerate() {
        let (begins, ends) = (start.saturating_sub(1), rounds.get(i + 1).map_or(u32::MAX, |next| next.saturating_sub(1)));
        if ends <= ticks.start || begins >= ticks.end {
            continue;
        }
        let first = begins.max(ticks.start) - ticks.start;
        if first > 0 && first + 1 >= frames {
            continue;
        }
        out.push(Chapter { start: first as u64, frames: 0, title: titles.round(i as u32 + 1) });
    }
    let starts: Vec<u64> = out.iter().skip(1).map(|c| c.start).chain([frames as u64]).collect();
    for (chapter, end) in out.iter_mut().zip(starts) {
        chapter.frames = end - chapter.start;
    }
    out
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
    /// The platform's encoder refused or failed (the browser's WebCodecs, iOS's VideoToolbox or AudioToolbox,
    /// Android's MediaCodec), or what it made isn't an MP4's: what was said.
    Encoder(String),
    /// The threads the frames' pictures are made on didn't start, or one
    /// stopped: what was said.
    Threads(String),
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
            Error::Encoder(said) => write!(f, "the encoder: {said}"),
            Error::Threads(said) => write!(f, "the threads presenting the frames: {said}"),
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

/// Where a video's frames go (an MP4 being encoded; in the tests, what is kept of them), each its picture's RGB bytes
/// and each console shown's sound, at 48 kHz. Each gives back a buffer a next frame's picture may be made in: the
/// frame's own once written, or one the encoder is done with (an empty one where it has none yet).
pub(crate) trait Frames {
    fn frame(&mut self, rgb: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error>;
}

/// [`write_mp4`]'s playing: the part played and each frame given to what `open` makes of the video's picture,
/// sound and chapters, the sound made 48 kHz ([`resample`]: a track's carried on from frame to frame, and the last
/// frame's with the rest of it).
pub(crate) fn play<F: Frames>(viewer: &mut Viewer, options: &Options, progress: OnProgress, open: impl FnOnce(Video, Option<Audio>, &Info) -> Result<F, Error>) -> Result<F, Error> {
    let Options { scale, view, sides, ref part, ref titles } = *options;
    if !(1..=MOST_SCALE).contains(&scale) {
        return Err(Error::Scale(scale));
    }
    let ticks = part.ticks(viewer.rounds(), viewer.length())?;
    if let Some(view) = view {
        viewer.set_view(view);
    }
    viewer.set_both(sides == Sides::Both);
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
    // (Each console's picture is presented at its own size; with both, the
    // rows of the two are put side by side. The sound is a track a console
    // shown, in the same order.)
    let (width, height) = (WIDTH * scale as usize, HEIGHT * scale as usize);
    let video = Video { width: (width * sides.count()) as u32, height: height as u32, frame_time: TICK_SECONDS };
    let shown = [viewer.view(), viewer.view() ^ 1];
    let shown = &shown[..sides.count()];
    let audio = viewer.has_sound().then_some(Audio { sample_rate: resample::RATE, tracks: shown.len() });
    let mut resamplers: Vec<Resampler> = audio.map(|a| (0..a.tracks).map(|_| Resampler::new(nettai_audio::SAMPLE_RATE)).collect()).unwrap_or_default();
    let track_titles = match sides {
        Sides::Both => vec![titles.both(viewer.view(), viewer.info())],
        _ => shown.iter().map(|&side| titles.side(side, viewer.info())).collect(),
    };
    let info = Info { titles: if audio.is_some() { track_titles } else { Vec::new() }, chapters: chapters(viewer.rounds(), &ticks, titles) };
    let mut stream = open(video, audio, &info)?;
    // (Each tick played and its frame drawn here, in order; the picture at the video's size made on the
    // presenters' threads, a few frames under way at once; written here in order: `presenters`.)
    let mut presenters = Presenters::start(viewer.text_renderer(), width, height, sides.count())?;
    // (The round of each frame under way, for the progress told once it is written.)
    let mut rounds = std::collections::VecDeque::new();
    let mut write = |presenters: &mut Presenters, rounds: &mut std::collections::VecDeque<u32>, stream: &mut F, at: &mut Progress| -> Result<(), Error> {
        let Presented { rgb, sound, .. } = presenters.next()?;
        let tracks: Vec<&[[f32; 2]]> = sound.iter().map(Vec::as_slice).collect();
        presenters.give_back(stream.frame(rgb, &tracks)?);
        at.round = rounds.pop_front().unwrap_or(at.round);
        at.written += 1;
        tell(*at)
    };
    for tick in ticks.clone() {
        rounds.push_back(viewer.round_at(tick));
        if !viewer.step_forward() {
            return Err(stopped(viewer));
        }
        // (Each console's own picture and sound; both on one screen, the viewer's, shown so.)
        let (frames, sound) = match sides {
            Sides::Both => {
                let mut heard = Vec::new();
                viewer.take_samples(&mut heard);
                (vec![viewer.frame()], vec![heard])
            }
            _ => {
                let frames = shown.iter().map(|&side| viewer.frame_view(side)).collect();
                let sound = shown
                    .iter()
                    .map(|&side| {
                        let mut heard = Vec::new();
                        viewer.take_samples_of(side, &mut heard);
                        heard
                    })
                    .collect();
                (frames, sound)
            }
        };
        let last = tick + 1 == ticks.end;
        let sound = sound
            .iter()
            .zip(&mut resamplers)
            .map(|(heard, r)| {
                let mut made = r.push(heard);
                if last {
                    made.extend(r.finish());
                }
                made
            })
            .collect();
        presenters.send(frames, sound);
        if presenters.under_way() >= presenters.ahead() {
            write(&mut presenters, &mut rounds, &mut stream, &mut at)?;
        }
    }
    while presenters.under_way() > 0 {
        write(&mut presenters, &mut rounds, &mut stream, &mut at)?;
    }
    Ok(stream)
}

/// Write a replay as an MP4 to `to`, as it is made, on a loaded game (its graphics, its text as it was loaded, its
/// sound if it was): [`write_mp4`], through a viewer of its own, on the thread it is called on (a host calls it on
/// one that isn't its window's). The same on the desktop, in a browser and on a phone.
pub fn export<W: Write>(game: &Loaded, replay: &Replay, options: &Options, encoding: &Encoding, to: W, progress: OnProgress) -> Result<Made<W>, Error> {
    let mut viewer = Viewer::open(game, replay, options.view).map_err(Error::Replay)?;
    write_mp4(&mut viewer, options, encoding, to, progress)
}

/// Write the replay `viewer` plays as an MP4 to `to`, as it is made: its frames and sound encoded by the
/// platform's encoder ([`encode`]: ffmpeg on the desktop, WebCodecs in a browser, the phone's own on a phone) and
/// their samples written a
/// chunk at a time ([`mp4`]), its sound tracks titled and its rounds chapters. `to` is given back with every
/// sample in it, and the head the file starts with, which goes before them ([`Made`]); canceled or failed, what
/// was written is the caller's to let go.
pub fn write_mp4<W: Write>(viewer: &mut Viewer, options: &Options, encoding: &Encoding, to: W, progress: OnProgress) -> Result<Made<W>, Error> {
    play(viewer, options, progress, |video, audio, info| Mp4::start(video, audio, info, encoding, to))?.finish()
}

/// An MP4 being made: the encoder's frames in, what it makes muxed into the file as it comes.
struct Mp4<W: Write> {
    encoder: Option<Box<dyn encode::Backend>>,
    muxer: Option<mp4::Muxer<W>>,
}

impl<W: Write> Mp4<W> {
    fn start(video: Video, audio: Option<Audio>, info: &Info, encoding: &Encoding, to: W) -> Result<Mp4<W>, Error> {
        Ok(Mp4::with(platform::start(video, audio, encoding)?, video, info, to))
    }

    /// An MP4 made by `encoder` (the platform's; in the tests, another platform's of this one's).
    fn with(encoder: Box<dyn encode::Backend>, video: Video, info: &Info, to: W) -> Mp4<W> {
        let frame = video.frame_time.0 as f64 / video.frame_time.1 as f64;
        let chapters = info.chapters.iter().map(|c| mp4::Chapter { title: c.title.clone(), start: c.start as f64 * frame }).collect();
        Mp4 { encoder: Some(encoder), muxer: Some(mp4::Muxer::new(to, info.titles.clone(), chapters)) }
    }

    fn muxed(&mut self, events: Vec<encode::Event>) -> Result<(), Error> {
        let muxer = self.muxer.as_mut().expect("muxing till finished");
        events.into_iter().try_for_each(|e| muxer.event(e))
    }

    fn finish(mut self) -> Result<Made<W>, Error> {
        let encoder = self.encoder.take().expect("finished once");
        let rest = encoder.finish()?;
        self.muxed(rest)?;
        self.muxer.take().expect("finished once").finish()
    }
}

impl<W: Write> Frames for Mp4<W> {
    fn frame(&mut self, rgb: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        let encoder = self.encoder.as_mut().expect("encoding till finished");
        let spare = encoder.frame(rgb, sound)?;
        let made = encoder.events()?;
        self.muxed(made)?;
        Ok(spare)
    }
}

impl<W: Write> Drop for Mp4<W> {
    /// Canceled or failed: the encoder is stopped.
    fn drop(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            encoder.cancel();
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
