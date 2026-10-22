//! What a video is to its encoder ([`Video`], [`Audio`], [`Info`]), and what an encoder is to the rest: frames and
//! sound in, and out its tracks (once it knows them) and their packets, which [`crate::fmp4`] makes an MP4 of. The
//! encoder is the platform's (ffmpeg on the desktop, the browser's own in a browser), each module's `start`; the
//! playing before and the MP4 after are the same for both.

use crate::Error;

/// The video's picture: its size in pixels, and the time one frame takes as a fraction of a second (numerator,
/// denominator), in lowest terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Video {
    pub width: u32,
    pub height: u32,
    pub frame_time: (u32, u32),
}

/// The sound's tracks: `tracks` of them (a console shown each), each stereo, at this many samples a second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audio {
    pub sample_rate: u32,
    pub tracks: usize,
}

/// What a video says of itself besides its frames: its sound tracks' titles and its chapters. The first sound
/// track is the default one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Info {
    /// The sound tracks' titles, by track (an empty one, or one past these:
    /// the track has none).
    pub titles: Vec<String>,
    /// The chapters, in the order they start.
    pub chapters: Vec<Chapter>,
}

/// A chapter: from a frame of the picture, for some frames, under a title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chapter {
    /// Its first frame (the frames counted from 0), and how many it has.
    pub start: u64,
    pub frames: u64,
    pub title: String,
}

/// What a track is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Video { width: u32, height: u32 },
    Audio,
}

/// A track of the video: its kind, its time scale (its times' units a second), how its samples are coded (its
/// sample description, as an MP4 says it), and what of them is shown (an edit list, in the movie's time scale,
/// [`crate::fmp4::MOVIE_TIMESCALE`]: AAC's encoder delay and padding left out; none: all of it).
#[derive(Clone, Debug)]
pub struct Track {
    pub kind: Kind,
    pub timescale: u32,
    pub codec: mp4_atom::Codec,
    pub edits: Option<mp4_atom::Edts>,
}

/// An encoded sample of a track (its index in the tracks), in decoding order: its bytes as an MP4 holds them, its
/// decoding time, how much later it is shown (a picture decoded before one shown ahead of it), and how long it lasts
/// (all in the track's time scale), and whether it starts on its own (a key frame; sound's always do).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    pub track: usize,
    pub data: Vec<u8>,
    pub time: u64,
    pub offset: i32,
    pub duration: u32,
    pub key: bool,
}

/// What an encoder has made.
#[derive(Clone, Debug)]
pub enum Event {
    /// The tracks, in their order: the picture's, then each sound track. Said once, before any packet.
    Tracks(Vec<Track>),
    Packet(Packet),
}

/// How a picture's color is sampled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Chroma {
    /// 4:2:0: color at half the picture's resolution. Every player takes it. Clean at an even multiple of the
    /// frame; at an odd one, every pixel's edge has a fringe of mixed color.
    #[default]
    Half,
    /// 4:4:4: color at the picture's resolution, every pixel edge sharp at any size. Browsers' and phones'
    /// hardware decoders mostly don't take it, and a browser's encoder makes none (it is 4:2:0 there).
    Full,
}

/// How a video is encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoding {
    /// The picture's constant rate factor: lower is closer to the frames and larger (0 lossless, 16 the default
    /// here). (A browser's encoder takes a bit rate instead: one is made of it.)
    pub crf: u8,
    pub chroma: Chroma,
    /// The sound's bit rate, in kb/s.
    pub audio_kbps: u32,
}

impl Default for Encoding {
    fn default() -> Encoding {
        Encoding { crf: 16, chroma: Chroma::Half, audio_kbps: 192 }
    }
}

/// An encoder at work.
pub(crate) trait Backend {
    /// A frame: its picture's pixels, RGB, and the sound each sound track plays with it (stereo, at the video's
    /// rate). May wait while the encoder is behind. Gives back a buffer for a next frame's picture: this one's, or
    /// one it is done with (an empty one where it has none yet).
    fn frame(&mut self, rgb: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error>;
    /// What it has made since, without waiting.
    fn events(&mut self) -> Result<Vec<Event>, Error>;
    /// The frames are done: the rest of what it makes, once it has.
    fn finish(self: Box<Self>) -> Result<Vec<Event>, Error>;
    /// Stop, making no more.
    fn cancel(self: Box<Self>);
}
