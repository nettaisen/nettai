//! What the platform's codecs are to the rest ([`crate::platform`]): the frames and the sound go in, the same bytes
//! on every platform (each picture NV12, made here: `yuv`; each sound track 48 kHz stereo floats: `resample`), and
//! out comes the codecs' own output, the H.264 of each frame in the order shown and each sound track's coded frames,
//! with what describes them ([`Coded`]). Everything a video's file needs beyond that (the tracks' descriptions,
//! their times, what of each is shown) is made of it in one place for all ([`crate::assemble`]), and the MP4 of that
//! ([`crate::mp4`]).

use crate::Error;

/// The video's picture: its size in pixels, and the time one frame takes as a fraction of a second (numerator,
/// denominator), in lowest terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Video {
    pub width: u32,
    pub height: u32,
    pub frame_time: (u32, u32),
}

/// The sound's tracks: `tracks` of them (a console shown each), each stereo, at this many samples a second (the
/// codecs': 48000, [`crate::resample`]'s).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audio {
    pub sample_rate: u32,
    pub tracks: usize,
}

/// The picture's quality where a codec keeps one (the desktop's x264: its constant rate factor; lower is closer to
/// the frames and larger).
#[cfg_attr(not(desktop), allow(dead_code))]
pub(crate) const QUALITY: u8 = 16;

/// The picture's bits for each pixel of each frame where a codec takes a bit rate instead (the browser's, the
/// phones'): a generous rate for the pixels' edges, less than x264 makes at [`QUALITY`] of a busy battle.
#[cfg_attr(desktop, allow(dead_code))]
const BITS_PER_PIXEL: f64 = 0.2;

/// The most bits a second the picture is given (two pictures side by side at four times the frame).
#[cfg_attr(desktop, allow(dead_code))]
const MOST_BITRATE: f64 = 40e6;

/// Each sound track's bits a second.
pub(crate) const SOUND_BITRATE: u32 = 192_000;

/// The picture's bit rate, for a codec that takes one (the desktop's takes [`QUALITY`]).
#[cfg_attr(desktop, allow(dead_code))]
pub(crate) fn bitrate(video: &Video) -> u32 {
    let (frame, timescale) = video.frame_time;
    (video.width as f64 * video.height as f64 * timescale as f64 / frame as f64 * BITS_PER_PIXEL).min(MOST_BITRATE) as u32
}

/// The platform's codecs at work: a picture's and a sound track's each. Every codec is asked for H.264 with no frame
/// reordering (no B-frames), so its frames come out in the order they go in, which is the order they are shown, and
/// a key frame at least every [`KEY_FRAMES`] frames; and for AAC (or, where a browser has none, Opus) at
/// [`SOUND_BITRATE`].
pub(crate) trait Codecs {
    /// A frame: its picture as NV12 (`yuv`), and the sound each sound track plays with it. May wait while the codecs
    /// are behind. Gives back a buffer for a next frame's picture: this one's, or one it is done with (an empty one
    /// where it has none yet).
    fn frame(&mut self, nv12: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error>;
    /// What they made since, without waiting.
    fn coded(&mut self) -> Result<Vec<Coded>, Error>;
    /// The frames are done: the rest of what they make, once they have.
    fn finish(self: Box<Self>) -> Result<Vec<Coded>, Error>;
    /// Stop, making no more.
    fn cancel(self: Box<Self>);
}

/// A key frame at least this often, in frames: two seconds of ticks, near enough, where the MP4's chunks end and
/// seeking lands.
pub(crate) const KEY_FRAMES: u32 = 120;

/// H.264 as a codec makes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum H264 {
    /// Its NAL units each after a start code (Annex B: ffmpeg's raw H.264, the browser's, Android's).
    #[cfg_attr(target_os = "ios", allow(dead_code))]
    AnnexB(Vec<u8>),
    /// Its NAL units each after its length in four bytes (iOS's).
    #[cfg_attr(not(target_os = "ios"), allow(dead_code))]
    Lengths(Vec<u8>),
}

/// How a sound track is coded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Sound {
    /// AAC: its AudioSpecificConfig, and how many samples the encoder puts before the sound (its priming), which
    /// the track is shown from past.
    Aac { config: Vec<u8>, priming: u32 },
    /// Opus (a browser with no AAC encoder), 20 ms a frame: its pre-skip, which says its own encoder's lookahead.
    #[cfg_attr(not(web), allow(dead_code))]
    Opus { pre_skip: u16 },
}

/// What the codecs made, each track's in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Coded {
    /// The picture's parameter sets (its SPS and PPS), where its codec says them apart from its frames (Android's
    /// codec configuration, iOS's format description); a codec that puts them in its key frames says none.
    #[cfg_attr(not(phone), allow(dead_code))]
    Sets(H264),
    /// The picture's next frame: its access unit (a key frame is one with an IDR slice).
    Frame(H264),
    /// Sound track `track`'s coding (from 0, the first after the picture), before its first frame.
    Coding(usize, Sound),
    /// Sound track `track`'s next coded frame.
    Sound(usize, Vec<u8>),
}
