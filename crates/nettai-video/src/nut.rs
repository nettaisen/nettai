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
//! headers   := main header, a stream header a stream (packets: the mandatory ones)
//! tick      := syncpoint, frame, [ syncpoint, frame ]      (the picture and its sound,
//!                                                           the earlier of the two first)
//! packet    := startcode (8 bytes), forward_ptr (v), [its checksum, past 4096,] body,
//!              checksum (CRC-32 of the body)
//! frame     := frame_code (1: the picture, 2: the sound), coded_pts (v: the pts + 1),
//!              data_size_msb (v: the size), checksum (CRC-32 of those), the data
//! index     := a packet: the last pts, every syncpoint's place, and by stream which
//!              syncpoints its frames follow and their pts; then its own length
//! ```
//!
//! - **Two streams, two time bases**, each exact: the picture's frames are
//!   one tick of the picture's time base apart (the caller's fraction of a
//!   second: nettai's 4389/262144, the GBA's frame), the sound's pts counts
//!   samples (1 over the sample rate). Nothing is rounded to milliseconds,
//!   so the two keep step however long the stream.
//! - **The picture** is `RGB[24]`: 8-bit red, green and blue a pixel, rows
//!   from the top, square pixels. **The sound** is `PFD[32]`: little-endian
//!   32-bit floats, left then right.
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
//!   starts no later. A sound that ran ahead of the picture by more than a
//!   frame couldn't be ordered and is refused.
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
//! - **Nothing varies**: no time of day, no name, no encoder string. The
//!   same frames and samples give the same bytes.
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

/// The frame codes: a picture, a sound. Every other code is invalid.
const PICTURE_CODE: u8 = 1;
const SOUND_CODE: u8 = 2;

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

/// The sound's stream: stereo, at this many samples a second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audio {
    pub sample_rate: u32,
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
    /// The time bases, by stream (the picture's, the sound's).
    bases: [TimeBase; 2],
    /// The main and stream headers, as they are written each time.
    headers: Vec<u8>,
    /// How many times the headers were written, and the position after
    /// which they are written next.
    header_sets: u32,
    next_repeat: u64,
    /// Frames of the picture and samples of the sound written: the next
    /// pts of each.
    frames: u64,
    samples: u64,
    /// Where the syncpoint before each stream's last frame starts.
    last_syncpoint: [Option<u64>; 2],
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
    /// A NUT stream of `video` (and `audio`, if there is sound) on `out`:
    /// the file's id and the headers are written at once. Refused: a
    /// picture of no size, a frame time or sample rate of 0, a frame time
    /// not in lowest terms or past the format's 31 bits, and a sound whose
    /// time base is the picture's (the format has each once).
    pub fn new(out: W, video: Video, audio: Option<Audio>) -> io::Result<Writer<W>> {
        let (num, den) = video.frame_time;
        if video.width == 0 || video.height == 0 {
            return Err(invalid(format!("a picture of {} by {} pixels", video.width, video.height)));
        }
        if num == 0 || den == 0 || num >= 1 << 31 || den >= 1 << 31 || gcd(num, den) != 1 {
            return Err(invalid(format!("a frame time of {num}/{den} s: a fraction in lowest terms, each part under 2^31")));
        }
        let picture = TimeBase { num, den };
        let sound = match audio {
            Some(Audio { sample_rate }) if sample_rate == 0 || sample_rate >= 1 << 31 => return Err(invalid(format!("a sample rate of {sample_rate} Hz"))),
            Some(Audio { sample_rate }) => TimeBase { num: 1, den: sample_rate },
            // (Unused: the stream has the picture alone.)
            None => picture,
        };
        if audio.is_some() && sound == picture {
            return Err(invalid(format!("the sound's time base is the picture's ({num}/{den} s)")));
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
            samples: 0,
            last_syncpoint: [None; 2],
            last: None,
            synced: Vec::new(),
        };
        w.headers = w.header_packets();
        w.put(FILE_ID)?;
        w.put_headers()?;
        Ok(w)
    }

    /// The next frame of the picture, `rgb` (red, green and blue bytes a
    /// pixel, rows from the top: width x height x 3 of them), and the
    /// sound that goes with it (left and right, at the stream's rate; none
    /// for a stream without sound). Refused, with nothing written: pixels
    /// that aren't a frame's, sound in a stream without it, and a sound
    /// that has run more than a frame ahead of the picture or fallen more
    /// than one behind it.
    pub fn frame(&mut self, rgb: &[u8], sound: &[[f32; 2]]) -> io::Result<()> {
        let size = self.video.width as usize * self.video.height as usize * 3;
        if rgb.len() != size {
            return Err(invalid(format!("{} bytes of picture; a frame of {} by {} is {size}", rgb.len(), self.video.width, self.video.height)));
        }
        if self.audio.is_none() && !sound.is_empty() {
            return Err(invalid("sound for a stream that has none".into()));
        }
        let [picture, heard] = self.bases;
        if self.written >= self.next_repeat {
            self.put_headers()?;
            self.next_repeat <<= REPEAT_SHIFT;
        }
        // The sound that starts no later than the picture goes before it.
        let mut shown = false;
        for chunk in sound.chunks(MOST_SAMPLES) {
            if !shown && heard.later(self.samples, picture, self.frames) {
                self.put_frame(0, PICTURE_CODE, rgb)?;
                shown = true;
            }
            let bytes: Vec<u8> = chunk.iter().flat_map(|s| [s[0].to_le_bytes(), s[1].to_le_bytes()]).flatten().collect();
            self.put_frame(1, SOUND_CODE, &bytes)?;
            self.samples += chunk.len() as u64;
        }
        if !shown {
            self.put_frame(0, PICTURE_CODE, rgb)?;
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

    /// The sound's samples written.
    pub fn samples(&self) -> u64 {
        self.samples
    }

    /// The bytes written.
    pub fn bytes(&self) -> u64 {
        self.written
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

    /// A frame of stream `stream` (0 the picture, 1 the sound) at its
    /// stream's next pts, behind a syncpoint of its own. Refused, with
    /// nothing written: one that would come before the frame written last
    /// (frames are in time order across the streams).
    fn put_frame(&mut self, stream: usize, code: u8, data: &[u8]) -> io::Result<()> {
        const NAMES: [(&str, &str); 2] = [("frame", "picture"), ("sample", "sound")];
        let pts = [self.frames, self.samples][stream];
        if let Some((before, written)) = self.last
            && self.bases[before].later(written, self.bases[stream], pts)
        {
            let ((unit, what), (other_unit, other)) = (NAMES[stream], NAMES[before]);
            return Err(invalid(format!("{unit} {pts} of the {what} would come before {other_unit} {written} of the {other}, already written")));
        }
        let at = self.written;
        // The syncpoint: the frame's time (a `t`: the pts times the number
        // of time bases, plus the base's index), and the way back to the
        // nearest syncpoint with a frame of every stream after it, in
        // sixteenths (0: none before this one).
        let bases = if self.audio.is_some() { 2 } else { 1 };
        let back = self.last_syncpoint.iter().flatten().min().map_or(0, |&to| (at - to) >> 4);
        let mut body = Vec::with_capacity(16);
        put_v(&mut body, pts * bases + stream as u64);
        put_v(&mut body, back);
        let mut bytes = packet(SYNCPOINT_STARTCODE, &body);
        // The frame's header, checked from its code on.
        let header = bytes.len();
        bytes.push(code);
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
        let streams = if self.audio.is_some() { 2 } else { 1 };
        let mut body = Vec::new();
        put_v(&mut body, pts * streams + latest as u64);
        put_v(&mut body, self.synced.len() as u64);
        let mut before = 0;
        for s in &self.synced {
            put_v(&mut body, (s.at >> 4) - before);
            before = s.at >> 4;
        }
        for stream in 0..streams as u8 {
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

    /// The main header and each stream's, as packets.
    fn header_packets(&self) -> Vec<u8> {
        let streams: u64 = if self.audio.is_some() { 2 } else { 1 };
        let mut main = Vec::new();
        put_v(&mut main, 3); // version
        put_v(&mut main, streams); // stream_count
        put_v(&mut main, MAX_DISTANCE);
        put_v(&mut main, streams); // time_base_count: one a stream
        for base in &self.bases[..streams as usize] {
            put_v(&mut main, base.num as u64);
            put_v(&mut main, base.den as u64);
        }
        // The frame codes, 0 to 255 ('N' is never one): 0 invalid, the
        // picture's, the sound's, the rest invalid. An entry is its flags,
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
        frame_code(&mut main, 0);
        if streams == 2 {
            frame_code(&mut main, 1);
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
            let mut sound = Vec::new();
            put_v(&mut sound, 1); // stream_id
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
        out
    }
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
