//! Android's codecs ([`crate::codecs`]): MediaCodec, the system's own H.264 for the picture and AAC for each sound
//! track, a console shown (through the NDK, the `ndk` crate).
//!
//! - **The picture** is the NV12 every platform is given, a frame an input buffer, packed: every video's sides are
//!   multiples of 16 (240 and 160 times its scale), which every encoder's rows and planes are aligned to. H.264 at
//!   the bit rate asked ([`codecs::bitrate`]), a key frame every two seconds (`i-frame-interval`), no B-frames
//!   (`max-bframes`), tagged BT.709 in the video range. It comes out as Annex B; its parameter sets as its codec
//!   configuration (a buffer of its own, or `csd-0` and `csd-1` in its output format), said apart.
//! - **The sound** is AAC-LC at the bit rate asked, of the 48 kHz samples as they come, as 16-bit PCM, the input
//!   every Android AAC encoder takes. Its config is its `csd-0`. An AAC encoder puts samples before the sound (its
//!   priming), which Android doesn't tell and which differs by encoder: it is measured as a video starts, a click
//!   through an encoder of the same settings and the system's decoder ([`aac_delay`]), as the browser's is.
//!
//! Only type-checked: no device has run it.

use crate::Error;
use crate::codecs::{self, Audio, Codecs, Coded, H264, Sound, Video};
use crate::resample::RATE;
use ndk::media::media_codec::{DequeuedInputBufferResult, DequeuedOutputBufferInfoResult, MediaCodec, MediaCodecDirection};
use ndk::media::media_format::MediaFormat;
use std::time::{Duration, Instant};

/// H.264's and AAC's MIME types, as MediaCodec names its codecs.
const AVC: &str = "video/avc";
const AAC: &str = "audio/mp4a-latm";

/// `MediaCodecInfo.CodecCapabilities.COLOR_FormatYUV420SemiPlanar`: NV12.
const NV12: i32 = 21;

/// `MediaFormat`'s BT.709, its video range and its SDR transfer (`COLOR_STANDARD_BT709`, `COLOR_RANGE_LIMITED`,
/// `COLOR_TRANSFER_SDR_VIDEO`).
const BT709: i32 = 1;
const VIDEO_RANGE: i32 = 2;
const SDR_VIDEO: i32 = 3;

/// `MediaCodecInfo.CodecProfileLevel.AACObjectLC`.
const AAC_LC: i32 = 2;

/// `MediaCodec`'s buffer flags: the codec's configuration, the stream's end.
const CODEC_CONFIG: u32 = 2;
const END_OF_STREAM: u32 = 4;

/// An AAC packet's samples.
const AAC_FRAME: u32 = 1024;

/// How long a wait for one of the codec's buffers is.
const WAIT: Duration = Duration::from_millis(10);

/// How long the codec may take nothing in, or make nothing, before it is taken to have stopped, in seconds.
const STALLED: u64 = 20;

/// Android's codecs for a video of `video` and `audio`.
pub(crate) fn start(video: Video, audio: Option<Audio>) -> Result<Box<dyn Codecs>, Error> {
    let tracks = audio.map_or(0, |a| a.tracks);
    let priming = if tracks > 0 { aac_delay(codecs::SOUND_BITRATE)? } else { 0 };
    let picture = Coder::start(AVC, &picture_format(&video, codecs::bitrate(&video)), MediaCodecDirection::Encoder)?;
    let sound = (0..tracks).map(|_| Coder::start(AAC, &sound_format(codecs::SOUND_BITRATE), MediaCodecDirection::Encoder)).collect::<Result<Vec<_>, _>>()?;
    let (frame, timescale) = video.frame_time;
    Ok(Box::new(Encoder {
        frame_us: frame as f64 * 1e6 / timescale as f64,
        picture,
        sound,
        given: 0,
        heard: vec![0; tracks],
        priming,
        sets: false,
        codings: vec![false; tracks],
        pcm: Vec::new(),
    }))
}

/// The picture's encoder's format.
fn picture_format(video: &Video, bitrate: u32) -> MediaFormat {
    let (frame, timescale) = video.frame_time;
    let mut f = MediaFormat::new();
    f.set_str("mime", AVC);
    f.set_i32("width", video.width as i32);
    f.set_i32("height", video.height as i32);
    f.set_i32("color-format", NV12);
    f.set_i32("bitrate", bitrate as i32);
    f.set_f32("frame-rate", timescale as f32 / frame as f32);
    // (A key frame every this many seconds: `KEY_FRAMES` frames of ticks, near enough.)
    f.set_f32("i-frame-interval", codecs::KEY_FRAMES as f32 * frame as f32 / timescale as f32);
    f.set_i32("max-bframes", 0);
    f.set_i32("color-standard", BT709);
    f.set_i32("color-range", VIDEO_RANGE);
    f.set_i32("color-transfer", SDR_VIDEO);
    f
}
/// A sound track's encoder's format: AAC-LC, 48 kHz stereo.
fn sound_format(bitrate: u32) -> MediaFormat {
    let mut f = MediaFormat::new();
    f.set_str("mime", AAC);
    f.set_i32("sample-rate", RATE as i32);
    f.set_i32("channel-count", 2);
    f.set_i32("aac-profile", AAC_LC);
    f.set_i32("bitrate", bitrate as i32);
    f
}

/// Samples as 16-bit PCM, appended to `out`.
fn pcm(samples: &[[f32; 2]], out: &mut Vec<u8>) {
    for s in samples.as_flattened() {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
}

/// What a codec made: its buffer's bytes and its flags.
struct Made {
    bytes: Vec<u8>,
    flags: u32,
}

/// A MediaCodec at work: what is given goes in an input buffer at a time; what it makes is taken as it comes.
struct Coder {
    codec: MediaCodec,
    /// Its configuration (`csd-0`, and `csd-1` for H.264), once its output format says it.
    config: Option<Vec<Vec<u8>>>,
    made: Vec<Made>,
    ended: bool,
}

impl Coder {
    fn start(mime: &str, format: &MediaFormat, direction: MediaCodecDirection) -> Result<Coder, Error> {
        let codec = match direction {
            MediaCodecDirection::Encoder => MediaCodec::from_encoder_type(mime),
            MediaCodecDirection::Decoder => MediaCodec::from_decoder_type(mime),
        };
        let codec = codec.ok_or_else(|| Error::Encoder(format!("MediaCodec has no {mime} {direction:?}")))?;
        codec.configure(format, None, direction).map_err(|e| Error::Encoder(format!("MediaCodec's {mime}: {e}")))?;
        codec.start().map_err(|e| Error::Encoder(format!("MediaCodec's {mime} didn't start: {e}")))?;
        Ok(Coder { codec, config: None, made: Vec::new(), ended: false })
    }

    /// `bytes` given, as many input buffers as they take, the first at `time_us` (`flags` on the last: the stream's
    /// end), taking what it makes while it waits for a buffer.
    fn give(&mut self, mut bytes: &[u8], time_us: i64, flags: u32) -> Result<(), Error> {
        let waited = Instant::now();
        loop {
            match self.codec.dequeue_input_buffer(WAIT).map_err(|e| Error::Encoder(format!("MediaCodec's input: {e}")))? {
                DequeuedInputBufferResult::Buffer(mut buffer) => {
                    let room = buffer.buffer_mut();
                    let n = bytes.len().min(room.len());
                    if n == 0 && !bytes.is_empty() {
                        return Err(Error::Encoder("MediaCodec gave an input buffer with no room".into()));
                    }
                    for (to, &b) in room.iter_mut().zip(&bytes[..n]) {
                        to.write(b);
                    }
                    let last = n == bytes.len();
                    self.codec
                        .queue_input_buffer(buffer, 0, n, time_us.max(0) as u64, if last { flags } else { 0 })
                        .map_err(|e| Error::Encoder(format!("MediaCodec's input: {e}")))?;
                    bytes = &bytes[n..];
                    if last {
                        return Ok(());
                    }
                }
                DequeuedInputBufferResult::TryAgainLater => {
                    if waited.elapsed().as_secs() >= STALLED {
                        return Err(Error::Encoder(format!("MediaCodec took nothing for {STALLED} seconds")));
                    }
                }
            }
            self.take(Duration::ZERO)?;
        }
    }

    /// What it made, waiting up to `wait` for the first.
    fn take(&mut self, wait: Duration) -> Result<(), Error> {
        let mut wait = wait;
        loop {
            match self.codec.dequeue_output_buffer(wait).map_err(|e| Error::Encoder(format!("MediaCodec's output: {e}")))? {
                DequeuedOutputBufferInfoResult::Buffer(buffer) => {
                    let info = *buffer.info();
                    let (from, size) = (info.offset().max(0) as usize, info.size().max(0) as usize);
                    let bytes = buffer.buffer().get(from..from + size).map(<[u8]>::to_vec);
                    self.codec.release_output_buffer(buffer, false).map_err(|e| Error::Encoder(format!("MediaCodec's output: {e}")))?;
                    let bytes = bytes.ok_or_else(|| Error::Encoder("MediaCodec said a buffer past its end".into()))?;
                    if info.flags() & END_OF_STREAM != 0 {
                        self.ended = true;
                    }
                    if !bytes.is_empty() {
                        self.made.push(Made { bytes, flags: info.flags() });
                    }
                    if self.ended {
                        return Ok(());
                    }
                }
                DequeuedOutputBufferInfoResult::OutputFormatChanged => {
                    let format = self.codec.output_format();
                    let config: Vec<Vec<u8>> = ["csd-0", "csd-1"].iter().filter_map(|k| format.buffer(k).map(<[u8]>::to_vec)).collect();
                    if !config.is_empty() {
                        self.config = Some(config);
                    }
                }
                DequeuedOutputBufferInfoResult::OutputBuffersChanged => {}
                DequeuedOutputBufferInfoResult::TryAgainLater => return Ok(()),
            }
            wait = Duration::ZERO;
        }
    }

    /// No more is given: the stream's end, then everything it makes, to its end.
    fn end(&mut self, time_us: i64) -> Result<(), Error> {
        self.give(&[], time_us, END_OF_STREAM)?;
        let waited = Instant::now();
        while !self.ended {
            self.take(WAIT)?;
            if waited.elapsed().as_secs() >= STALLED {
                return Err(Error::Encoder(format!("MediaCodec didn't end its stream in {STALLED} seconds")));
            }
        }
        Ok(())
    }
}

impl Drop for Coder {
    fn drop(&mut self) {
        let _ = self.codec.stop();
    }
}

/// Android coding a video.
struct Encoder {
    /// A frame's time, in microseconds (the codecs' input times).
    frame_us: f64,
    picture: Coder,
    sound: Vec<Coder>,
    /// The frames given; each sound track's samples given.
    given: u64,
    heard: Vec<u64>,
    /// The samples the AAC encoder puts before the sound.
    priming: u32,
    /// Whether the picture's parameter sets, and each sound track's coding, are said.
    sets: bool,
    codings: Vec<bool>,
    pcm: Vec<u8>,
}

impl Encoder {
    /// What the codecs made since: the picture's sets and each sound track's coding once each, then their frames.
    fn take(&mut self) -> Vec<Coded> {
        let mut out = Vec::new();
        if !self.sets
            && let Some(config) = self.picture.config.take()
        {
            self.sets = true;
            out.push(Coded::Sets(H264::AnnexB(config.concat())));
        }
        for made in self.picture.made.drain(..) {
            out.push(if made.flags & CODEC_CONFIG != 0 { Coded::Sets(H264::AnnexB(made.bytes)) } else { Coded::Frame(H264::AnnexB(made.bytes)) });
        }
        for (t, coder) in self.sound.iter_mut().enumerate() {
            if !self.codings[t]
                && let Some(config) = coder.config.take().and_then(|c| c.into_iter().next())
            {
                self.codings[t] = true;
                out.push(Coded::Coding(t, Sound::Aac { config, priming: self.priming }));
            }
            out.extend(coder.made.drain(..).filter(|m| m.flags & CODEC_CONFIG == 0).map(|m| Coded::Sound(t, m.bytes)));
        }
        out
    }
}

impl Codecs for Encoder {
    fn frame(&mut self, nv12: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        let at = (self.given as f64 * self.frame_us).round() as i64;
        self.picture.give(&nv12, at, 0)?;
        self.given += 1;
        for (t, (coder, samples)) in self.sound.iter_mut().zip(sound).enumerate() {
            if samples.is_empty() {
                continue;
            }
            self.pcm.clear();
            pcm(samples, &mut self.pcm);
            coder.give(&self.pcm, (self.heard[t] as f64 * 1e6 / RATE as f64).round() as i64, 0)?;
            self.heard[t] += samples.len() as u64;
        }
        Ok(nv12)
    }

    fn coded(&mut self) -> Result<Vec<Coded>, Error> {
        self.picture.take(Duration::ZERO)?;
        for coder in &mut self.sound {
            coder.take(Duration::ZERO)?;
        }
        Ok(self.take())
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<Coded>, Error> {
        let end = (self.given as f64 * self.frame_us).round() as i64;
        self.picture.end(end)?;
        for (t, coder) in self.sound.iter_mut().enumerate() {
            coder.end((self.heard[t] as f64 * 1e6 / RATE as f64).round() as i64)?;
        }
        Ok(self.take())
    }

    fn cancel(self: Box<Self>) {
        // (Dropped: each codec stopped and let go.)
    }
}

/// How many samples the AAC encoder of `bitrate` puts before the sound: a click, encoded and decoded again by the
/// system's own codecs, comes out that much later. (As the browser's is measured: `webcodecs`.)
fn aac_delay(bitrate: u32) -> Result<u32, Error> {
    const AT: usize = 3000;
    const LENGTH: usize = 16384;
    let mut click = vec![[0.0f32; 2]; LENGTH];
    click[AT] = [0.5, 0.5];
    let mut bytes = Vec::new();
    pcm(&click, &mut bytes);
    let mut encoder = Coder::start(AAC, &sound_format(bitrate), MediaCodecDirection::Encoder)?;
    encoder.give(&bytes, 0, 0)?;
    encoder.end((LENGTH as f64 * 1e6 / RATE as f64) as i64)?;
    let config = encoder.config.as_ref().and_then(|c| c.first()).cloned().ok_or_else(|| Error::Encoder("MediaCodec's AAC said no configuration".into()))?;
    let mut format = MediaFormat::new();
    format.set_str("mime", AAC);
    format.set_i32("sample-rate", RATE as i32);
    format.set_i32("channel-count", 2);
    format.set_i32("is-adts", 0);
    format.set_buffer("csd-0", &config);
    let mut decoder = Coder::start(AAC, &format, MediaCodecDirection::Decoder)?;
    let packets: Vec<Made> = encoder.made.drain(..).filter(|m| m.flags & CODEC_CONFIG == 0).collect();
    for (n, packet) in packets.iter().enumerate() {
        decoder.give(&packet.bytes, (n as f64 * AAC_FRAME as f64 * 1e6 / RATE as f64) as i64, 0)?;
    }
    decoder.end((packets.len() as f64 * AAC_FRAME as f64 * 1e6 / RATE as f64) as i64)?;
    // (The decoder's PCM: 16-bit, unless its format says floats.)
    let floats = decoder.codec.output_format().i32("pcm-encoding") == Some(4);
    let left: Vec<f32> = decoder
        .made
        .iter()
        .flat_map(|m| {
            if floats {
                m.bytes.chunks_exact(8).map(|s| f32::from_le_bytes(s[..4].try_into().unwrap())).collect::<Vec<_>>()
            } else {
                m.bytes.chunks_exact(4).map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0).collect()
            }
        })
        .collect();
    let (place, loudest) = left.iter().enumerate().fold((0, 0.0f32), |(p, l), (i, v)| if v.abs() > l { (i, v.abs()) } else { (p, l) });
    if loudest < 0.1 || place < AT {
        return Err(Error::Encoder(format!("MediaCodec's AAC delay couldn't be told (its click came out at {place}, {loudest} loud)")));
    }
    Ok((place - AT) as u32)
}
