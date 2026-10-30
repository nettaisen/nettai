//! Android's encoder ([`crate::encode`]): MediaCodec, the system's own H.264 for the picture and AAC for each sound
//! track, a console shown (through the NDK, the `ndk` crate). The MP4 is the same as the desktop's
//! ([`crate::mp4`]).
//!
//! - **The picture** is given as NV12 in the video range, made of the frames' RGB here ([`crate::yuv`]: BT.709, the
//!   bytes iOS's encoder is given, and as the desktop's ffmpeg makes them), a frame an input buffer, packed: every
//!   video's sides are multiples of 16 (240 and 160 times its scale), which every encoder's rows and planes are
//!   aligned to. H.264 at a bit rate made of the rate factor as the browser's and iOS's are, a key frame every two
//!   seconds (120 frames), no B-frames (`max-bframes`), so each frame is decoded when it is shown and a packet's
//!   time is its frame's count on the tick's time base, exact; tagged BT.709 in the video range. The encoder says its
//!   parameter sets (`csd-0`, `csd-1`) as its output format, which make the track's avcC; its packets come as Annex
//!   B, made the MP4's lengths here, the parameter sets in them left out (they are the avcC's).
//! - **The sound** is AAC-LC at the bit rate asked, of the 48 kHz samples as they come (the samples every encoder
//!   is given), as 16-bit PCM, the input every Android AAC encoder takes. An AAC encoder puts samples before the
//!   sound (its priming), which Android doesn't tell and which differs by encoder: it is measured as an export
//!   starts, a click through an encoder of the same settings and the system's decoder ([`aac_delay`]), as the
//!   browser's is, and each track is shown from past it (its edit list). A packet is 1024 samples.
//! - **4:4:4** isn't coded here: the picture is 4:2:0 whatever [`Encoding::chroma`] says, as in a browser and on
//!   iOS.
//!
//! Only type-checked: no device has run it.

use crate::Error;
use crate::annex_b::{lengths, units};
use crate::encode::{Audio, Backend, Encoding, Event, Kind, Packet, Track, Video};
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

/// `MediaCodec`'s buffer flags: a key frame, the codec's configuration, the stream's end.
const KEY_FRAME: u32 = 1;
const CODEC_CONFIG: u32 = 2;
const END_OF_STREAM: u32 = 4;

/// An AAC packet's samples.
const AAC_FRAME: u32 = 1024;

/// How long a wait for one of the codec's buffers is.
const WAIT: Duration = Duration::from_millis(10);

/// How long the codec may take nothing in, or make nothing, before it is taken to have stopped, in seconds.
const STALLED: u64 = 20;

/// Android's encoder for a video of `video` and `audio`, as `encoding` says (its titles and chapters are the MP4's to
/// write).
pub(crate) fn start(video: Video, audio: Option<Audio>, encoding: &Encoding) -> Result<Box<dyn Backend>, Error> {
    let tracks = audio.map_or(0, |a| a.tracks);
    let audio_bitrate = encoding.audio_kbps * 1000;
    let delay = if tracks > 0 { aac_delay(audio_bitrate)? } else { 0 };
    let picture = Coder::start(AVC, &picture_format(&video, bitrate(&video, encoding)), MediaCodecDirection::Encoder)?;
    let sound = (0..tracks).map(|_| Coder::start(AAC, &sound_format(audio_bitrate), MediaCodecDirection::Encoder)).collect::<Result<Vec<_>, _>>()?;
    let (frame, timescale) = video.frame_time;
    Ok(Box::new(Encoder {
        video,
        frame_us: frame as f64 * 1e6 / timescale as f64,
        picture,
        sound,
        given: 0,
        heard: vec![0; tracks],
        made: vec![0; tracks],
        audio_bitrate,
        delay,
        said: false,
        early: Vec::new(),
        nv12: Vec::new(),
        pcm: Vec::new(),
    }))
}

/// The picture's bit rate: as the browser's and iOS's encoders are given it.
fn bitrate(video: &Video, encoding: &Encoding) -> u32 {
    let (frame, timescale) = video.frame_time;
    let per_pixel = 0.2 * 2f64.powf((16.0 - encoding.crf as f64) / 6.0);
    (video.width as f64 * video.height as f64 * timescale as f64 / frame as f64 * per_pixel).min(40e6) as u32
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
    // (120 frames: two seconds of ticks, near enough.)
    f.set_f32("i-frame-interval", 2.0);
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

/// What a codec made: its buffer's bytes, its flags and its time (in microseconds).
struct Made {
    bytes: Vec<u8>,
    flags: u32,
    time_us: i64,
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
                        self.made.push(Made { bytes, flags: info.flags(), time_us: info.presentation_time_us() });
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

/// Android encoding a video.
struct Encoder {
    video: Video,
    /// A frame's time, in microseconds (the codecs' times).
    frame_us: f64,
    picture: Coder,
    sound: Vec<Coder>,
    /// The frames given; each sound track's samples given, and its packets made.
    given: u64,
    heard: Vec<u64>,
    made: Vec<u64>,
    audio_bitrate: u32,
    /// The samples the AAC encoder puts before the sound.
    delay: u32,
    /// The tracks are said (every codec said its configuration), and what was made before.
    said: bool,
    early: Vec<Packet>,
    nv12: Vec<u8>,
    pcm: Vec<u8>,
}

impl Encoder {
    /// What the codecs made since, as packets: the tracks first, once every codec has said its configuration.
    fn packets(&mut self) -> Result<Vec<Event>, Error> {
        let frame = self.video.frame_time.0;
        for made in self.picture.made.drain(..) {
            if made.flags & CODEC_CONFIG != 0 {
                continue;
            }
            // (Its frame's count on the tick's time base: no B-frames, so decoded when shown.)
            let time = (made.time_us as f64 / self.frame_us).round().max(0.0) as u64 * frame as u64;
            self.early.push(Packet { track: 0, data: lengths(&made.bytes), time, offset: 0, duration: frame, key: made.flags & KEY_FRAME != 0 });
        }
        for (t, coder) in self.sound.iter_mut().enumerate() {
            for made in coder.made.drain(..) {
                if made.flags & CODEC_CONFIG != 0 {
                    continue;
                }
                self.early.push(Packet { track: 1 + t, data: made.bytes, time: self.made[t] * AAC_FRAME as u64, offset: 0, duration: AAC_FRAME, key: true });
                self.made[t] += 1;
            }
        }
        let mut events = Vec::new();
        if !self.said {
            let Some(tracks) = self.tracks()? else { return Ok(events) };
            events.push(Event::Tracks(tracks));
            self.said = true;
        }
        events.extend(self.early.drain(..).map(Event::Packet));
        Ok(events)
    }

    /// The tracks, once every codec has said its configuration: the picture's avcC from its parameter sets, each
    /// sound track's AudioSpecificConfig, shown from past the AAC encoder's delay.
    fn tracks(&self) -> Result<Option<Vec<Track>>, Error> {
        use mp4_atom::esds::{DecoderConfig, DecoderSpecific, EsDescriptor};
        use mp4_atom::{Avc1, Avcc, Codec, Colr, Edts, Elst, ElstEntry, Esds, FixedPoint, Mp4a, Visual};
        let Some(sets) = &self.picture.config else { return Ok(None) };
        if self.sound.iter().any(|c| c.config.is_none()) {
            return Ok(None);
        }
        let nals: Vec<&[u8]> = sets.iter().flat_map(|s| units(s)).collect();
        let (sps, pps) = (nals.iter().find(|n| n.first().is_some_and(|b| b & 0x1F == 7)), nals.iter().find(|n| n.first().is_some_and(|b| b & 0x1F == 8)));
        let (Some(sps), Some(pps)) = (sps, pps) else {
            return Err(Error::Encoder("MediaCodec's H.264 said no SPS and PPS".into()));
        };
        let avcc = Avcc::new(sps, pps).map_err(|e| Error::Encoder(format!("MediaCodec's parameter sets: {e}")))?;
        let (width, height) = (self.video.width, self.video.height);
        let visual = Visual { data_reference_index: 1, width: width as u16, height: height as u16, ..Default::default() };
        let colr = Colr::Nclx { colour_primaries: 1, transfer_characteristics: 1, matrix_coefficients: 1, full_range_flag: false };
        let codec = Codec::Avc1(Avc1 { visual, avcc, colr: Some(colr), ..Default::default() });
        let mut tracks = vec![Track { kind: Kind::Video { width, height }, timescale: self.video.frame_time.1, codec, edits: None }];
        for (t, coder) in self.sound.iter().enumerate() {
            let config = coder.config.as_ref().and_then(|c| c.first()).cloned().unwrap_or_default();
            let esds = Esds {
                es_desc: EsDescriptor {
                    es_id: t as u16 + 2,
                    dec_config: DecoderConfig {
                        object_type_indication: 0x40,
                        stream_type: 5,
                        max_bitrate: self.audio_bitrate,
                        avg_bitrate: self.audio_bitrate,
                        dec_specific: Some(DecoderSpecific { profile: 2, freq_index: 3, chan_conf: 2, raw: config }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            };
            let audio = mp4_atom::Audio { data_reference_index: 1, channel_count: 2, sample_size: 16, sample_rate: FixedPoint::new(RATE as u16, 0) };
            let entry = ElstEntry { segment_duration: 0, media_time: Some(self.delay as u64), media_rate: FixedPoint::new(1, 0) };
            tracks.push(Track {
                kind: Kind::Audio,
                timescale: RATE,
                codec: Codec::Mp4a(Mp4a { audio, esds, btrt: None, taic: None }),
                edits: (self.delay > 0).then(|| Edts { elst: Some(Elst { entries: vec![entry] }) }),
            });
        }
        Ok(Some(tracks))
    }
}

impl Backend for Encoder {
    fn frame(&mut self, rgb: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        crate::yuv::nv12(&rgb, self.video.width as usize, self.video.height as usize, &mut self.nv12);
        let at = (self.given as f64 * self.frame_us).round() as i64;
        self.picture.give(&self.nv12, at, 0)?;
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
        Ok(rgb)
    }

    fn events(&mut self) -> Result<Vec<Event>, Error> {
        self.picture.take(Duration::ZERO)?;
        for coder in &mut self.sound {
            coder.take(Duration::ZERO)?;
        }
        self.packets()
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<Event>, Error> {
        let end = (self.given as f64 * self.frame_us).round() as i64;
        self.picture.end(end)?;
        for (t, coder) in self.sound.iter_mut().enumerate() {
            coder.end((self.heard[t] as f64 * 1e6 / RATE as f64).round() as i64)?;
        }
        let made = self.packets()?;
        if !self.said {
            return Err(Error::Encoder("MediaCodec made no picture".into()));
        }
        Ok(made)
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
