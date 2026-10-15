//! The NUT container, written: raw video frames and PCM sound in one
//! stream, on any `std::io::Write`. NUT is FFmpeg's and MPlayer's container
//! ("NUT Open Container Format", `docs/nut.txt` of the NUT repository; the
//! FourCCs of raw formats are its `nut4cc.txt`'s), which `ffmpeg -f nut`
//! reads from a pipe. This writes version 3 of it, the frozen one, and is
//! the only place that knows its bytes.
//!
//! What a stream written here is, in the specification's terms:
//!
//! ```text
//! file      := "nut/multimedia container\0", headers, { tick }, headers [, headers], index
//! headers   := main header, a stream header a stream, { info packet }
//! tick      := syncpoint, frame, { syncpoint, frame }      (the picture and each sound
//!                                                           track's, in time order)
//! packet    := startcode (8 bytes), forward_ptr (v), [its checksum, past 4096,] body,
//!              checksum (CRC-32 of the body)
//! info      := a packet: a sound track's title and disposition, or a chapter's start,
//!              length and title
//! frame     := frame_code (1: the picture, 2 on: the sound tracks), coded_pts (v: the pts + 1),
//!              data_size_msb (v: the size), checksum (CRC-32 of those), the data
//! index     := a packet: the last pts, every syncpoint's place, and by stream which
//!              syncpoints its frames follow and their pts; then its own length
//! ```
//!
//! - **The streams**: the picture, then the sound's tracks (none, one, or
//!   more: a console's each, with both side by side). **Two time bases**,
//!   each exact: the picture's frames are one tick of the picture's time
//!   base apart (the caller's fraction of a second: nettai's 4389/262144,
//!   the GBA's frame), the sound's pts counts samples (1 over the sample
//!   rate; every track's the same base). Nothing is rounded to
//!   milliseconds, so they keep step however long the stream.
//! - **The picture** is `RGB[24]`: 8-bit red, green and blue a pixel, rows
//!   from the top, square pixels. **The sound** is `PFD[32]`: little-endian
//!   32-bit floats, left then right.
//! - **Info packets** follow the stream headers, in every set of them
//!   ([`Info`]): each sound track's title (as `Title`, the specification's
//!   name, and as `handler_name`, FFmpeg's name for an MP4 track's handler,
//!   which its MP4 writer takes from the stream's metadata as it takes the
//!   title), the first track marked `default` (its `Disposition`), and a
//!   packet a chapter: its start and length on the picture's time base and
//!   its title. FFmpeg's NUT reader makes these the streams' metadata and
//!   dispositions and the file's chapters, and its MP4 writer keeps them
//!   (the chapters as a chapter text track and Nero's `chpl`).
//! - **Every frame is a keyframe with its own syncpoint.** A raw frame is
//!   far larger than `max_distance` (32768), and the specification lets
//!   startcodes be further apart only across "a syncpoint followed by a
//!   single frame"; so each frame has one, and its header a checksum (a
//!   frame over twice `max_distance` must). A syncpoint's back pointer
//!   names the nearest earlier syncpoint with a keyframe of every stream
//!   after it, which is the one before the other stream's last frame.
//! - **Frames are in time order across the streams** (a frame's pts is
//!   never before an earlier frame's, in common time), as the
//!   specification requires: a tick's sound goes before its picture when it
//!   starts no later, and the tracks' frames of one time go in the tracks'
//!   order. A sound that ran ahead of the picture by more than a frame
//!   couldn't be ordered and is refused.
//! - **The headers are written three times at least**: at the start, at
//!   the end before the index (the specification's "immediately before the
//!   index"), and after each of 8 MiB, 64 MiB, 512 MiB... of the stream
//!   ("the earliest possible position after 2^x"), or once more at the end
//!   of a stream too short for that. There is no info packet.
//! - **The index is the stream's last packet.** A reader of a pipe skips
//!   it; a reader of a file finds it from the end (its length is its last
//!   field) and has the stream's length and every frame's place from it
//!   without looking for them. (Without one, ffmpeg looks for the last
//!   syncpoint from the file's end, from a kilobyte back and twice as far
//!   each try, and says "read_timestamp failed" for each until it is past
//!   the last raw frame.) It takes about ten bytes a frame.
//! - **Nothing varies**: no time of day, no encoder string, no name but the
//!   caller's titles. The same frames, samples and info give the same bytes.
//!
//! Not written: end-of-relevance frames (a "SHOULD": the picture's frames
//! are a fixed time apart, so the last one's length is known).

use std::io::{self, Write};

/// The first bytes of every NUT file.
pub const FILE_ID: &[u8] = b"nut/multimedia container\0";

/// The most bytes between two startcodes that aren't around a single frame
/// (`max_distance`: the specification's recommended most).
pub const MAX_DISTANCE: u64 = 32_768;

/// The most samples a frame of raw sound holds ("for raw audio up to 4096
/// samples shall be considered 1 codec frame").
pub const MOST_SAMPLES: usize = 4096;

const MAIN_STARTCODE: u64 = 0x7A56_1F5F_04AD + (((b'N' as u64) << 8 | b'M' as u64) << 48);
const STREAM_STARTCODE: u64 = 0x1140_5BF2_F9DB + (((b'N' as u64) << 8 | b'S' as u64) << 48);
const SYNCPOINT_STARTCODE: u64 = 0xE4AD_EECA_4569 + (((b'N' as u64) << 8 | b'K' as u64) << 48);
const INDEX_STARTCODE: u64 = 0xDD67_2F23_E64E + (((b'N' as u64) << 8 | b'X' as u64) << 48);
const INFO_STARTCODE: u64 = 0xAB68_B596_BA78 + (((b'N' as u64) << 8 | b'I' as u64) << 48);

/// The most sound tracks a stream has.
pub const MOST_TRACKS: usize = 16;

/// How many syncpoints' flags one number of the index holds.
const INDEX_FLAGS: usize = 32;

/// A frame's flags (`flags[frame_code]`).
const FLAG_KEY: u64 = 1 << 0;
const FLAG_CODED_PTS: u64 = 1 << 3;
const FLAG_SIZE_MSB: u64 = 1 << 5;
const FLAG_CHECKSUM: u64 = 1 << 6;
const FLAG_INVALID: u64 = 1 << 13;

/// The flags of every frame written: a keyframe, its pts and its size in
/// its header, the header checked.
const FRAME_FLAGS: u64 = FLAG_KEY | FLAG_CODED_PTS | FLAG_SIZE_MSB | FLAG_CHECKSUM;

/// The frame codes: a picture, and from the next one a sound track's each.
/// Every other code is invalid.
const PICTURE_CODE: u8 = 1;

/// The first position the headers are repeated after, and how much further
/// each next one is (2^23, then 2^26, 2^29...).
const FIRST_REPEAT: u64 = 1 << 23;
const REPEAT_SHIFT: u32 = 3;

/// The picture's stream: its size in pixels, and the time one frame takes
/// as a fraction of a second (numerator, denominator), in lowest terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Video {
    pub width: u32,
    pub height: u32,
    pub frame_time: (u32, u32),
}

/// The sound's streams: `tracks` of them (1 to [`MOST_TRACKS`]), each
/// stereo, at this many samples a second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audio {
    pub sample_rate: u32,
    pub tracks: usize,
}

/// What a stream says of itself besides its frames, in its info packets.
/// The first sound track is the default one.
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

/// A stream's time base: the seconds one of its pts units takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TimeBase {
    num: u32,
    den: u32,
}

impl TimeBase {
    /// Whether `a` units of `self` are later than `b` units of `other`.
    fn later(self, a: u64, other: TimeBase, b: u64) -> bool {
        a as u128 * self.num as u128 * other.den as u128 > b as u128 * other.num as u128 * self.den as u128
    }
}

/// Writes a NUT stream a frame of the picture at a time, each with the
/// sound that goes with it.
pub struct Writer<W: Write> {
    out: W,
    /// Bytes written so far: where the next packet starts.
    written: u64,
    video: Video,
    audio: Option<Audio>,
    /// The time bases: the picture's, the sound's (every track's).
    bases: [TimeBase; 2],
    /// The main and stream headers and the info packets, as they are
    /// written each time.
    headers: Vec<u8>,
    /// How many times the headers were written, and the position after
    /// which they are written next.
    header_sets: u32,
    next_repeat: u64,
    /// Frames of the picture written, and samples of each sound track: the
    /// next pts of each.
    frames: u64,
    samples: Vec<u64>,
    /// Where the syncpoint before each stream's last frame starts.
    last_syncpoint: Vec<Option<u64>>,
    /// The last frame written: its stream and pts.
    last: Option<(usize, u64)>,
    /// Every syncpoint written, for the index.
    synced: Vec<Synced>,
}

/// A syncpoint and the frame behind it.
#[derive(Clone, Copy)]
struct Synced {
    /// Where the syncpoint starts.
    at: u64,
    /// The frame's stream and pts.
    stream: u8,
    pts: u64,
}

fn invalid(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, why)
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

impl<W: Write> Writer<W> {
    /// A NUT stream of `video` (and `audio`, if there is sound) on `out`,
    /// with no info: [`Writer::with_info`].
    pub fn new(out: W, video: Video, audio: Option<Audio>) -> io::Result<Writer<W>> {
        Writer::with_info(out, video, audio, &Info::default())
    }

    /// A NUT stream of `video` (and `audio`, if there is sound) on `out`,
    /// saying `info` of itself: the file's id and the headers are written
    /// at once. Refused: a picture of no size, a frame time or sample rate
    /// of 0, a frame time not in lowest terms or past the format's 31 bits,
    /// a sound whose time base is the picture's (the format has each once),
    /// no sound tracks or more than [`MOST_TRACKS`], and titles for sound
    /// the stream hasn't.
    pub fn with_info(out: W, video: Video, audio: Option<Audio>, info: &Info) -> io::Result<Writer<W>> {
        let (num, den) = video.frame_time;
        if video.width == 0 || video.height == 0 {
            return Err(invalid(format!("a picture of {} by {} pixels", video.width, video.height)));
        }
        if num == 0 || den == 0 || num >= 1 << 31 || den >= 1 << 31 || gcd(num, den) != 1 {
            return Err(invalid(format!("a frame time of {num}/{den} s: a fraction in lowest terms, each part under 2^31")));
        }
        let picture = TimeBase { num, den };
        let sound = match audio {
            Some(Audio { sample_rate, .. }) if sample_rate == 0 || sample_rate >= 1 << 31 => return Err(invalid(format!("a sample rate of {sample_rate} Hz"))),
            Some(Audio { tracks, .. }) if !(1..=MOST_TRACKS).contains(&tracks) => return Err(invalid(format!("{tracks} sound tracks (1 to {MOST_TRACKS})"))),
            Some(Audio { sample_rate, .. }) => TimeBase { num: 1, den: sample_rate },
            // (Unused: the stream has the picture alone.)
            None => picture,
        };
        if audio.is_some() && sound == picture {
            return Err(invalid(format!("the sound's time base is the picture's ({num}/{den} s)")));
        }
        let tracks = audio.map_or(0, |a| a.tracks);
        if info.titles.len() > tracks && info.titles[tracks..].iter().any(|t| !t.is_empty()) {
            return Err(invalid(format!("titles for {} sound tracks of a stream of {tracks}", info.titles.len())));
        }
        let mut w = Writer {
            out,
            written: 0,
            video,
            audio,
            bases: [picture, sound],
            headers: Vec::new(),
            header_sets: 0,
            next_repeat: FIRST_REPEAT,
            frames: 0,
            samples: vec![0; tracks],
            last_syncpoint: vec![None; 1 + tracks],
            last: None,
            synced: Vec::new(),
        };
        w.headers = w.header_packets(info);
        w.put(FILE_ID)?;
        w.put_headers()?;
        Ok(w)
    }

    /// The next frame of the picture, `rgb` (red, green and blue bytes a
    /// pixel, rows from the top: width x height x 3 of them), and the
    /// sound that goes with it, a run of samples a track (left and right,
    /// at the stream's rate; none for a stream without sound). Refused,
    /// with nothing written: pixels that aren't a frame's, sound in a
    /// stream without it, another number of tracks than the stream's, and
    /// a sound that has run more than a frame ahead of the picture or
    /// fallen more than one behind it.
    pub fn frame(&mut self, rgb: &[u8], sound: &[&[[f32; 2]]]) -> io::Result<()> {
        let size = self.video.width as usize * self.video.height as usize * 3;
        if rgb.len() != size {
            return Err(invalid(format!("{} bytes of picture; a frame of {} by {} is {size}", rgb.len(), self.video.width, self.video.height)));
        }
        let tracks = self.samples.len();
        if tracks == 0 && sound.iter().any(|s| !s.is_empty()) {
            return Err(invalid("sound for a stream that has none".into()));
        }
        if tracks > 0 && sound.len() != tracks {
            return Err(invalid(format!("sound of {} tracks for a stream of {tracks}", sound.len())));
        }
        if self.written >= self.next_repeat {
            self.put_headers()?;
            self.next_repeat <<= REPEAT_SHIFT;
        }
        // The frames in time order: the earliest of the picture and each
        // track's next run of samples, a sound before the picture where it
        // starts no later, a track before a later one where they start
        // together.
        let [picture, heard] = self.bases;
        let mut taken = vec![0; tracks];
        let mut shown = false;
        loop {
            let mut next = (!shown).then_some(0);
            for track in (0..tracks).filter(|&t| taken[t] < sound[t].len()) {
                let pts = self.samples[track];
                let sooner = match next {
                    None => true,
                    Some(0) => !heard.later(pts, picture, self.frames),
                    Some(other) => pts < self.samples[other - 1],
                };
                if sooner {
                    next = Some(1 + track);
                }
            }
            match next {
                None => break,
                Some(0) => {
                    self.put_frame(0, rgb)?;
                    shown = true;
                }
                Some(stream) => {
                    let track = stream - 1;
                    let chunk = &sound[track][taken[track]..(taken[track] + MOST_SAMPLES).min(sound[track].len())];
                    let bytes: Vec<u8> = chunk.iter().flat_map(|s| [s[0].to_le_bytes(), s[1].to_le_bytes()]).flatten().collect();
                    self.put_frame(stream, &bytes)?;
                    self.samples[track] += chunk.len() as u64;
                    taken[track] += chunk.len();
                }
            }
        }
        self.frames += 1;
        Ok(())
    }

    /// End the stream: the headers once more (and again, for a stream so
    /// short it has them only twice), the index, and the sink flushed and
    /// given back.
    pub fn finish(mut self) -> io::Result<W> {
        self.put_headers()?;
        while self.header_sets < 3 {
            self.put_headers()?;
        }
        if let Some(index) = self.index_packet() {
            self.put(&index)?;
        }
        self.out.flush()?;
        Ok(self.out)
    }

    /// The picture's frames written.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// The samples written of sound track `track` (none of a track the
    /// stream hasn't).
    pub fn samples(&self, track: usize) -> u64 {
        self.samples.get(track).copied().unwrap_or(0)
    }

    /// The bytes written.
    pub fn bytes(&self) -> u64 {
        self.written
    }

    /// The streams: the picture, and each sound track.
    fn streams(&self) -> usize {
        1 + self.samples.len()
    }

    /// The time bases there are: the picture's, and the sound's if it has
    /// any.
    fn time_bases(&self) -> u64 {
        if self.audio.is_some() { 2 } else { 1 }
    }

    /// A stream's time base, by its index (0 the picture's, 1 the sound's).
    fn base_of(stream: usize) -> usize {
        stream.min(1)
    }

    fn put(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.out.write_all(bytes)?;
        self.written += bytes.len() as u64;
        Ok(())
    }

    fn put_headers(&mut self) -> io::Result<()> {
        let headers = std::mem::take(&mut self.headers);
        let put = self.put(&headers);
        self.headers = headers;
        self.header_sets += 1;
        put
    }

    /// A frame of stream `stream` (0 the picture, then the sound's tracks)
    /// at its stream's next pts, behind a syncpoint of its own. Refused,
    /// with nothing written: one that would come before the frame written
    /// last (frames are in time order across the streams).
    fn put_frame(&mut self, stream: usize, data: &[u8]) -> io::Result<()> {
        let names = |stream: usize| if stream == 0 { ("frame", "the picture".to_string()) } else if self.streams() == 2 { ("sample", "the sound".to_string()) } else { ("sample", format!("sound track {stream}")) };
        let pts = if stream == 0 { self.frames } else { self.samples[stream - 1] };
        let base = |stream: usize| self.bases[Self::base_of(stream)];
        if let Some((before, written)) = self.last
            && base(before).later(written, base(stream), pts)
        {
            let ((unit, what), (other_unit, other)) = (names(stream), names(before));
            return Err(invalid(format!("{unit} {pts} of {what} would come before {other_unit} {written} of {other}, already written")));
        }
        let at = self.written;
        // The syncpoint: the frame's time (a `t`: the pts times the number
        // of time bases, plus the base's index), and the way back to the
        // nearest syncpoint with a frame of every stream after it, in
        // sixteenths (0: none before this one).
        let back = self.last_syncpoint.iter().flatten().min().map_or(0, |&to| (at - to) >> 4);
        let mut body = Vec::with_capacity(16);
        put_v(&mut body, pts * self.time_bases() + Self::base_of(stream) as u64);
        put_v(&mut body, back);
        let mut bytes = packet(SYNCPOINT_STARTCODE, &body);
        // The frame's header, checked from its code on.
        let header = bytes.len();
        bytes.push(PICTURE_CODE + stream as u8);
        // (A whole pts, not its low bits: with `msb_pts_shift` 0, the pts
        // plus one.)
        put_v(&mut bytes, pts + 1);
        put_v(&mut bytes, data.len() as u64);
        let checksum = crc32(&bytes[header..]);
        bytes.extend_from_slice(&checksum.to_be_bytes());
        self.put(&bytes)?;
        self.put(data)?;
        self.last_syncpoint[stream] = Some(at);
        self.last = Some((stream, pts));
        self.synced.push(Synced { at, stream: stream as u8, pts });
        Ok(())
    }

    /// The index, of a stream with frames: the latest pts (a `t`), every
    /// syncpoint's place in sixteenths (each from the one before), and for
    /// each stream which syncpoints have a frame of it before them (since
    /// the syncpoint before) and those frames' pts (each from the stream's
    /// last, the first from -1); then the packet's own length, which a
    /// reader finds twelve bytes from the file's end.
    fn index_packet(&self) -> Option<Vec<u8>> {
        let &(latest, pts) = self.last.as_ref()?;
        let mut body = Vec::new();
        put_v(&mut body, pts * self.time_bases() + Self::base_of(latest) as u64);
        put_v(&mut body, self.synced.len() as u64);
        let mut before = 0;
        for s in &self.synced {
            put_v(&mut body, (s.at >> 4) - before);
            before = s.at >> 4;
        }
        for stream in 0..self.streams() as u8 {
            // (Syncpoint j has the frame behind syncpoint j - 1 before it;
            // the first has none.)
            let before = |j: usize| j.checked_sub(1).map(|k| self.synced[k]).filter(|s| s.stream == stream);
            let mut last_pts: i64 = -1;
            for from in (0..self.synced.len()).step_by(INDEX_FLAGS) {
                let to = (from + INDEX_FLAGS).min(self.synced.len());
                // Flags as bits, the first lowest, under a bit that ends
                // them; the number's own low bit says they are bits.
                let flags = (from..to).rev().fold(1u64, |x, j| x << 1 | before(j).is_some() as u64);
                put_v(&mut body, flags << 1);
                for frame in (from..to).filter_map(before) {
                    put_v(&mut body, (frame.pts as i64 - last_pts) as u64);
                    last_pts = frame.pts as i64;
                }
            }
        }
        // index_ptr: the packet from its startcode to its checksum's end.
        let forward = body.len() as u64 + 8 + 4;
        let mut length = Vec::new();
        put_v(&mut length, forward);
        let whole = 8 + length.len() as u64 + if forward > 4096 { 4 } else { 0 } + forward;
        body.extend_from_slice(&whole.to_be_bytes());
        Some(packet(INDEX_STARTCODE, &body))
    }

    /// The main header, each stream's, and the info packets, as packets.
    fn header_packets(&self, info: &Info) -> Vec<u8> {
        let streams = self.streams() as u64;
        let mut main = Vec::new();
        put_v(&mut main, 3); // version
        put_v(&mut main, streams); // stream_count
        put_v(&mut main, MAX_DISTANCE);
        put_v(&mut main, self.time_bases()); // time_base_count
        for base in &self.bases[..self.time_bases() as usize] {
            put_v(&mut main, base.num as u64);
            put_v(&mut main, base.den as u64);
        }
        // The frame codes, 0 to 255 ('N' is never one): 0 invalid, a
        // stream's each from 1, the rest invalid. An entry is its flags,
        // how many fields follow, then those: pts_delta (s), size_mul,
        // stream_id, size_lsb, reserved_count, count, match_time_delta (s).
        let invalid_codes = |main: &mut Vec<u8>, count: u64| {
            put_v(main, FLAG_INVALID);
            put_v(main, 6);
            put_s(main, 0);
            for field in [1, 0, 0, 0, count] {
                put_v(main, field);
            }
        };
        let frame_code = |main: &mut Vec<u8>, stream: u64| {
            put_v(main, FRAME_FLAGS);
            put_v(main, 7);
            put_s(main, 0);
            for field in [1, stream, 0, 0, 1] {
                put_v(main, field);
            }
            // (A raw frame is whole at once: match_time_delta 0.)
            put_s(main, 0);
        };
        invalid_codes(&mut main, 1);
        for stream in 0..streams {
            frame_code(&mut main, stream);
        }
        // (The codes left, less 'N', which no entry counts.)
        invalid_codes(&mut main, 255 - 1 - streams);
        put_v(&mut main, 0); // header_count_minus1: no elision headers
        put_v(&mut main, 0); // main_flags
        let mut out = packet(MAIN_STARTCODE, &main);

        let mut picture = Vec::new();
        put_v(&mut picture, 0); // stream_id
        put_v(&mut picture, 0); // stream_class: video
        put_vb(&mut picture, b"RGB\x18");
        put_v(&mut picture, 0); // time_base_id
        put_v(&mut picture, 0); // msb_pts_shift: every pts is whole
        // (max_pts_distance: no more than a second's units; every frame's
        // header is checked whatever its distance.)
        put_v(&mut picture, (self.bases[0].den / self.bases[0].num).max(1) as u64);
        put_v(&mut picture, 0); // decode_delay
        put_v(&mut picture, 0); // stream_flags
        put_vb(&mut picture, b""); // codec_specific_data
        put_v(&mut picture, self.video.width as u64);
        put_v(&mut picture, self.video.height as u64);
        put_v(&mut picture, 1); // sample_width : sample_height, square
        put_v(&mut picture, 1);
        put_v(&mut picture, 0); // colorspace_type: unknown (it is RGB)
        out.extend_from_slice(&packet(STREAM_STARTCODE, &picture));

        if let Some(audio) = self.audio {
            for stream in 1..streams {
                let mut sound = Vec::new();
                put_v(&mut sound, stream); // stream_id
                put_v(&mut sound, 1); // stream_class: audio
                put_vb(&mut sound, b"PFD\x20");
                put_v(&mut sound, 1); // time_base_id
                put_v(&mut sound, 0); // msb_pts_shift
                put_v(&mut sound, audio.sample_rate as u64); // max_pts_distance
                put_v(&mut sound, 0); // decode_delay
                put_v(&mut sound, 0); // stream_flags
                put_vb(&mut sound, b""); // codec_specific_data
                put_v(&mut sound, audio.sample_rate as u64); // samplerate_num
                put_v(&mut sound, 1); // samplerate_denom
                put_v(&mut sound, 2); // channel_count
                out.extend_from_slice(&packet(STREAM_STARTCODE, &sound));
            }
        }

        // The info: each sound track's (its title; the first the default),
        // then each chapter's, numbered from 1, on the picture's time base.
        for track in 0..self.samples.len() {
            let title = info.titles.get(track).map(String::as_str).filter(|t| !t.is_empty());
            let mut fields = Vec::new();
            if let Some(title) = title {
                fields.extend([("Title", title), ("handler_name", title)]);
            }
            if track == 0 {
                fields.push(("Disposition", "default"));
            }
            if !fields.is_empty() {
                out.extend_from_slice(&info_packet(2 + track as u64, 0, 0, 0, &fields));
            }
        }
        for (n, chapter) in info.chapters.iter().enumerate() {
            let start = chapter.start * self.time_bases();
            out.extend_from_slice(&info_packet(0, 1 + n as i64, start, chapter.frames, &[("Title", &chapter.title)]));
        }
        out
    }
}

/// An info packet: of the stream `stream_id_plus1` - 1 (0: of the file or a
/// chapter), of the chapter `chapter_id` (0: none), its start (a `t`) and
/// length, and its fields, each a name and UTF-8 text.
fn info_packet(stream_id_plus1: u64, chapter_id: i64, start: u64, length: u64, fields: &[(&str, &str)]) -> Vec<u8> {
    let mut body = Vec::new();
    put_v(&mut body, stream_id_plus1);
    put_s(&mut body, chapter_id);
    put_v(&mut body, start);
    put_v(&mut body, length);
    put_v(&mut body, fields.len() as u64);
    for (name, value) in fields {
        put_vb(&mut body, name.as_bytes());
        // (A value of -1: UTF-8 text, as bytes behind their count.)
        put_s(&mut body, -1);
        put_vb(&mut body, value.as_bytes());
    }
    packet(INFO_STARTCODE, &body)
}

/// A packet: its startcode, the length of what follows (the body and its
/// checksum: the forward pointer), past 4096 that header's own checksum,
/// the body, the body's checksum.
fn packet(startcode: u64, body: &[u8]) -> Vec<u8> {
    let mut out = startcode.to_be_bytes().to_vec();
    let forward = body.len() as u64 + 4;
    put_v(&mut out, forward);
    if forward > 4096 {
        let header = crc32(&out);
        out.extend_from_slice(&header.to_be_bytes());
    }
    out.extend_from_slice(body);
    out.extend_from_slice(&crc32(body).to_be_bytes());
    out
}

/// A `v`: an unsigned number, seven bits a byte from the most significant,
/// every byte but the last with its high bit set.
fn put_v(out: &mut Vec<u8>, value: u64) {
    let mut shift = (1..).map(|n| n * 7).find(|&bits| bits >= 64 || value >> bits == 0).expect("some length") - 7;
    while shift > 0 {
        out.push(0x80 | (value >> shift) as u8 & 0x7F);
        shift -= 7;
    }
    out.push(value as u8 & 0x7F);
}

/// An `s`: a signed number as a `v` (0, 1, -1, 2, -2... as 0, 1, 2, 3, 4...).
fn put_s(out: &mut Vec<u8>, value: i64) {
    put_v(out, if value > 0 { 2 * value as u64 - 1 } else { 2 * value.unsigned_abs() });
}

/// A `vb`: bytes behind their count.
fn put_vb(out: &mut Vec<u8>, bytes: &[u8]) {
    put_v(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

/// NUT's checksum: a CRC-32 with the generator 0x04C11DB7, from zero, the
/// most significant bit first, nothing inverted.
pub fn crc32(bytes: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = (i as u32) << 24;
            let mut bit = 0;
            while bit < 8 {
                c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0x04C1_1DB7 } else { c << 1 };
                bit += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    };
    bytes.iter().fold(0, |crc, &b| (crc << 8) ^ TABLE[((crc >> 24) as u8 ^ b) as usize])
}

#[cfg(test)]
pub(crate) mod reading;

#[cfg(test)]
mod tests;
