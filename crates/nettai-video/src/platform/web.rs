//! The browser's codecs ([`crate::codecs`]): WebCodecs, the browser's own H.264 for the picture, and AAC (where the
//! browser has it, else Opus) for each sound track, a console shown.
//!
//! [`Encoder`] is fed on a thread that may wait (a worker: never the page's main thread), a frame at a time. The
//! coding itself runs on a thread of its own, on its event loop, which is where WebCodecs answers: the frames go to
//! it through a short queue (a frame waits while it is full, so the codecs set the pace), and what they make comes
//! back through another.
//!
//! - **The picture** is the NV12 every platform is given (a `VideoFrame` of format NV12, its color space BT.709 in
//!   the video range), H.264 (High, else Main, else Constrained Baseline) at the bit rate asked, with no frame
//!   reordering (WebCodecs' H.264 encoders make none), a key frame every [`codecs::KEY_FRAMES`] frames, as Annex B
//!   (`avc: { format: "annexb" }`: each key frame carries its parameter sets).
//! - **AAC's delay.** An AAC encoder puts samples before the sound (its priming: AAC's blocks overlap, and an encoder
//!   looks ahead), which no AAC packet says and WebCodecs doesn't tell: Chromium stamps the packets from 0, its macOS
//!   encoder (AudioToolbox) putting 2112 first and its Windows one (Media Foundation) its own. So it is measured as a
//!   video starts, a click through an encoder of the same settings and a decoder ([`aac_delay`]) (the
//!   user: "is aac in the browser broken then?", then "ok fix it then": before, the sound played late by it, 44 ms on
//!   macOS). AAC's config is the AudioSpecificConfig its first chunk's decoder configuration says.
//! - **Opus** (a browser with no AAC encoder) is asked for 20 ms frames, and says its own lookahead, its pre-skip:
//!   312, Chromium's libopus's.

use crate::Error;
use crate::codecs::{self, Audio, Codecs, Coded, H264, Sound, Video};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use wasm_bindgen::prelude::*;

/// How many frames may wait for the codecs.
const QUEUE: usize = 8;

/// How long a frame waits for the codecs before they are taken to have stopped (their thread never started, or the
/// browser's codec never answered), in seconds.
const STALLED: u64 = 20;

/// Opus's pre-skip from Chromium's libopus: its encoder's lookahead, in samples at 48 kHz.
const OPUS_PRE_SKIP: u16 = 312;

/// What the codecs' thread is told of the video.
#[derive(Clone, Copy, Debug)]
struct Spec {
    width: u32,
    height: u32,
    /// A frame's time: its numerator in the time scale that is its denominator.
    frame: u32,
    timescale: u32,
    /// How many sound tracks (none: no sound).
    tracks: usize,
    bitrate: u32,
}

enum Msg {
    /// A frame's NV12 and each sound track's sound (stereo, interleaved, at 48 kHz).
    Frame(Vec<u8>, Vec<Vec<f32>>),
    Finish,
}

/// What the codecs' thread says: what they made, that they are done, or why they stopped.
enum Said {
    Made(Coded),
    Done,
    Failed(String),
}

/// The browser's codecs for a video of `video` and `audio`.
pub(crate) fn start(video: Video, audio: Option<Audio>) -> Result<Box<dyn Codecs>, Error> {
    Ok(Box::new(Encoder::start(video, audio)?))
}

/// The browser coding a video.
struct Encoder {
    frames: SyncSender<Msg>,
    said: Receiver<Said>,
}

impl Encoder {
    /// Start coding the picture `video` and the sound `audio`: the codecs' thread is started, and tells the browser's
    /// codecs.
    fn start(video: Video, audio: Option<Audio>) -> Result<Encoder, Error> {
        let (frame, timescale) = video.frame_time;
        let spec = Spec { width: video.width, height: video.height, frame, timescale, tracks: audio.map_or(0, |a| a.tracks), bitrate: codecs::bitrate(&video) };
        let (frames, taken) = mpsc::sync_channel(QUEUE);
        let (tell, said) = mpsc::channel();
        nettai_threads::spawn_async("video", move || {
            Box::pin(async move {
                let ended = encode(spec, &taken, &tell).await;
                let _ = tell.send(match ended {
                    Ok(()) => Said::Done,
                    Err(why) => Said::Failed(why),
                });
            })
        })
        .map_err(|e| Error::Encoder(format!("the codecs' thread: {e}")))?;
        Ok(Encoder { frames, said })
    }

    /// What the codecs' thread said since, without waiting (or, `until_done`, till it is done): what they made, or
    /// why they stopped.
    fn heard(&mut self, until_done: bool) -> Result<Vec<Coded>, Error> {
        let mut made = Vec::new();
        loop {
            let said = if until_done { self.said.recv().ok() } else { self.said.try_recv().ok() };
            match said {
                Some(Said::Made(coded)) => made.push(coded),
                Some(Said::Done) => return Ok(made),
                Some(Said::Failed(why)) => return Err(Error::Encoder(why)),
                None if until_done => return Err(Error::Encoder("the codecs stopped".into())),
                None => return Ok(made),
            }
        }
    }
}

impl Codecs for Encoder {
    fn frame(&mut self, nv12: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        let samples = sound.iter().map(|s| s.as_flattened().to_vec()).collect();
        let mut msg = Msg::Frame(nv12, samples);
        let waited = web_time::Instant::now();
        loop {
            match self.frames.try_send(msg) {
                Ok(()) => return Ok(Vec::new()),
                // (The codecs are behind: this worker waits, as a worker may.)
                Err(TrySendError::Full(back)) => {
                    msg = back;
                    // (Codecs that failed let go of their queue, below; ones that never answer, this.)
                    if waited.elapsed().as_secs() >= STALLED {
                        return Err(Error::Encoder(format!("the browser's codecs took no frame for {STALLED} seconds")));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(TrySendError::Disconnected(_)) => return Err(self.heard(true).err().unwrap_or_else(|| Error::Encoder("the codecs stopped".into()))),
            }
        }
    }

    fn coded(&mut self) -> Result<Vec<Coded>, Error> {
        self.heard(false)
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<Coded>, Error> {
        let _ = self.frames.send(Msg::Finish);
        self.heard(true)
    }

    fn cancel(self: Box<Self>) {
        // (Their queue let go, the codecs' thread stops of itself.)
    }
}

/// What went wrong in a codec (the last it said), shared with its callbacks.
type Failed = Rc<RefCell<Option<JsValue>>>;

/// What the browser said went wrong, in words.
fn said(why: JsValue) -> String {
    why.dyn_ref::<js_sys::Error>().map(|e| String::from(e.message())).unwrap_or_else(|| format!("{why:?}"))
}

/// What went wrong in a codec, if anything did.
fn check(failed: &Failed) -> Result<(), JsValue> {
    failed.borrow().clone().map_or(Ok(()), Err)
}

/// A codec's error callback: what it says kept in `failed`.
fn failing(failed: &Failed) -> JsValue {
    let failed = failed.clone();
    Closure::<dyn FnMut(JsValue)>::new(move |why: JsValue| *failed.borrow_mut() = Some(why)).into_js_value()
}

/// A dictionary of `fields`, for what web-sys has no type of (a codec's own settings: `avc`, `aac`, `opus`).
fn dictionary(fields: &[(&str, JsValue)]) -> Result<JsValue, JsValue> {
    let out = js_sys::Object::new();
    for (name, value) in fields {
        js_sys::Reflect::set(&out, &(*name).into(), value)?;
    }
    Ok(out.into())
}

/// `bytes` copied out of the shared memory: the browser takes no view of it.
fn copied(bytes: &[u8]) -> js_sys::Uint8Array {
    let copy = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    copy.copy_from(bytes);
    copy
}

/// `samples` copied out of the shared memory.
fn copied_samples(samples: &[f32]) -> js_sys::Float32Array {
    let copy = js_sys::Float32Array::new_with_length(samples.len() as u32);
    copy.copy_from(samples);
    copy
}

/// A BufferSource's bytes: an ArrayBuffer's, or a view's of its buffer.
fn source_bytes(source: &JsValue) -> Vec<u8> {
    if !js_sys::ArrayBuffer::is_view(source) {
        return js_sys::Uint8Array::new(source).to_vec();
    }
    let get = |name: &str| js_sys::Reflect::get(source, &name.into()).unwrap_or_default();
    let at = |name: &str| get(name).as_f64().unwrap_or_default() as u32;
    js_sys::Uint8Array::new_with_byte_offset_and_length(&get("buffer"), at("byteOffset"), at("byteLength")).to_vec()
}

/// Some time on this worker's event loop, `ms` milliseconds.
async fn sleep(ms: i32) {
    let slept = js_sys::Promise::new(&mut |kept, _| {
        if let Ok(worker) = js_sys::global().dyn_into::<web_sys::WorkerGlobalScope>() {
            let _ = worker.set_timeout_with_callback_and_timeout_and_arguments_0(&kept, ms);
        }
    });
    let _ = slept.await;
}

/// The stereo audio of `samples` (interleaved, at 48 kHz) at `timestamp`, in microseconds.
fn audio_data(samples: &[f32], timestamp: f64) -> Result<web_sys::AudioData, JsValue> {
    let init = web_sys::AudioDataInit::new(&copied_samples(samples), web_sys::AudioSampleFormat::F32, 2, (samples.len() / 2) as u32, 48_000.0, 0);
    init.set_timestamp_f64(timestamp);
    web_sys::AudioData::new(&init)
}

/// How many samples the AAC encoder `config` makes puts before the sound: its priming, which no AAC packet says and
/// WebCodecs doesn't tell (Chromium stamps the packets from 0: macOS's AudioToolbox puts 2112 first). A click,
/// encoded and decoded again: how much later it comes out.
async fn aac_delay(config: &web_sys::AudioEncoderConfig) -> Result<u32, JsValue> {
    const AT: usize = 3000;
    const LENGTH: usize = 16384;
    let mut click = vec![0.0; LENGTH * 2];
    click[AT * 2] = 0.5;
    click[AT * 2 + 1] = 0.5;
    let failed: Failed = Rc::default();
    let chunks: Rc<RefCell<Vec<web_sys::EncodedAudioChunk>>> = Rc::default();
    let decoder_config: Rc<RefCell<Option<web_sys::AudioDecoderConfig>>> = Rc::default();
    let made = {
        let (chunks, decoder_config) = (chunks.clone(), decoder_config.clone());
        Closure::<dyn FnMut(web_sys::EncodedAudioChunk, JsValue)>::new(move |chunk, meta: JsValue| {
            chunks.borrow_mut().push(chunk);
            let mut config = decoder_config.borrow_mut();
            if config.is_none() && meta.is_object() {
                *config = meta.unchecked_ref::<web_sys::EncodedAudioChunkMetadata>().get_decoder_config();
            }
        })
        .into_js_value()
    };
    let encoder = web_sys::AudioEncoder::new(&web_sys::AudioEncoderInit::new(failing(&failed).unchecked_ref(), made.unchecked_ref()))?;
    encoder.configure(config)?;
    let data = audio_data(&click, 0.0)?;
    let encoded = encoder.encode(&data);
    data.close();
    encoded?;
    encoder.flush().await?;
    encoder.close()?;
    check(&failed)?;
    let decoder_config = decoder_config.borrow_mut().take().ok_or_else(|| js_sys::Error::new("the AAC encoder said no decoder configuration"))?;
    // (The left channel, as it comes out.)
    let heard: Rc<RefCell<Vec<f32>>> = Rc::default();
    let decoded = {
        let (heard, failed) = (heard.clone(), failed.clone());
        Closure::<dyn FnMut(web_sys::AudioData)>::new(move |d: web_sys::AudioData| {
            let left = js_sys::Float32Array::new_with_length(d.number_of_frames());
            let options = web_sys::AudioDataCopyToOptions::new(0);
            options.set_format(web_sys::AudioSampleFormat::F32Planar);
            match d.copy_to_with_buffer_source(&left, &options) {
                Ok(()) => heard.borrow_mut().extend(left.to_vec()),
                Err(why) => *failed.borrow_mut() = Some(why),
            }
            d.close();
        })
        .into_js_value()
    };
    let decoder = web_sys::AudioDecoder::new(&web_sys::AudioDecoderInit::new(failing(&failed).unchecked_ref(), decoded.unchecked_ref()))?;
    decoder.configure(&decoder_config)?;
    for chunk in chunks.borrow().iter() {
        decoder.decode(chunk)?;
    }
    decoder.flush().await?;
    decoder.close()?;
    check(&failed)?;
    let (place, loudest) = heard
        .borrow()
        .iter()
        .enumerate()
        .fold((None, 0.0f32), |(place, loudest), (n, v)| if v.abs() > loudest { (Some(n), v.abs()) } else { (place, loudest) });
    match place {
        Some(place) if loudest >= 0.1 && place >= AT => Ok((place - AT) as u32),
        _ => Err(js_sys::Error::new(&format!(
            "the AAC encoder's delay couldn't be told (its click came out at {}, {loudest} loud)",
            place.map_or(-1, |p| p as i64)
        ))
        .into()),
    }
}

/// A chunk the codecs made: its track (0 the picture, then each sound track's), its bytes, and the description of the
/// decoder configuration it came with, if it did (a sound track's first: AAC's AudioSpecificConfig).
struct Chunk {
    track: usize,
    data: Vec<u8>,
    config: Option<Vec<u8>>,
}

/// What an encoder of track `track` made (`chunk`, with `meta`), copied out.
fn chunk_of(track: usize, chunk: &JsValue, meta: &JsValue) -> Result<Chunk, JsValue> {
    let data = if track == 0 {
        let chunk: &web_sys::EncodedVideoChunk = chunk.unchecked_ref();
        let data = js_sys::Uint8Array::new_with_length(chunk.byte_length());
        chunk.copy_to_with_u8_array(&data)?;
        data
    } else {
        let chunk: &web_sys::EncodedAudioChunk = chunk.unchecked_ref();
        let data = js_sys::Uint8Array::new_with_length(chunk.byte_length());
        chunk.copy_to_with_u8_array(&data)?;
        data
    };
    let config = (track > 0 && meta.is_object())
        .then(|| meta.unchecked_ref::<web_sys::EncodedAudioChunkMetadata>().get_decoder_config())
        .flatten()
        .and_then(|c| c.get_description())
        .map(|d| source_bytes(&d));
    Ok(Chunk { track, data: data.to_vec(), config })
}

/// The browser's codecs, open: the picture's encoder and each sound track's, what they made since it was taken, and
/// what went wrong.
struct Session {
    spec: Spec,
    /// A frame's time, in microseconds.
    frame_us: f64,
    video: web_sys::VideoEncoder,
    audio: Vec<web_sys::AudioEncoder>,
    /// Whether the sound is Opus (else AAC), and the samples its encoder puts before the sound (AAC's priming; 0 for
    /// Opus, whose pre-skip says it).
    opus: bool,
    audio_delay: u32,
    made: Rc<RefCell<Vec<Chunk>>>,
    failed: Failed,
    /// The frames given, and each sound track's samples.
    frames: u64,
    samples: Vec<u64>,
}

impl Session {
    async fn open(spec: Spec, frame_us: f64) -> Result<Session, JsValue> {
        if !js_sys::Reflect::has(&js_sys::global(), &"VideoEncoder".into())? {
            return Err(js_sys::Error::new("this browser has no WebCodecs").into());
        }
        let made: Rc<RefCell<Vec<Chunk>>> = Rc::default();
        let failed: Failed = Rc::default();
        let error = failing(&failed);
        let output = |track: usize| {
            let (made, failed) = (made.clone(), failed.clone());
            Closure::<dyn FnMut(JsValue, JsValue)>::new(move |chunk: JsValue, meta: JsValue| match chunk_of(track, &chunk, &meta) {
                Ok(chunk) => made.borrow_mut().push(chunk),
                Err(why) => *failed.borrow_mut() = Some(why),
            })
            .into_js_value()
        };
        // H.264 at level 4.2 (two pictures side by side at four times the frame, 60 a second): High, Main, else the
        // constrained baseline every encoder has.
        let mut config = None;
        for codec in ["avc1.64002A", "avc1.4D402A", "avc1.42E02A"] {
            let c = web_sys::VideoEncoderConfig::new(codec, spec.height, spec.width);
            c.set_bitrate(spec.bitrate);
            c.set_framerate(1e6 / frame_us);
            c.set_latency_mode(web_sys::LatencyMode::Quality);
            js_sys::Reflect::set(&c, &"avc".into(), &dictionary(&[("format", "annexb".into())])?)?;
            if web_sys::VideoEncoder::is_config_supported(&c).await?.get_supported() == Some(true) {
                config = Some(c);
                break;
            }
        }
        let Some(config) = config else {
            return Err(js_sys::Error::new(&format!("this browser can't encode H.264 at {}x{}", spec.width, spec.height)).into());
        };
        let video = web_sys::VideoEncoder::new(&web_sys::VideoEncoderInit::new(error.unchecked_ref(), output(0).unchecked_ref()))?;
        video.configure(&config)?;
        let (mut opus, mut audio_delay, mut audio) = (false, 0, Vec::new());
        if spec.tracks > 0 {
            let mut chosen = None;
            for (codec, own, settings) in
                [("mp4a.40.2", "aac", dictionary(&[("format", "aac".into())])?), ("opus", "opus", dictionary(&[("frameDuration", 20_000.into())])?)]
            {
                let c = web_sys::AudioEncoderConfig::new(codec, 2, 48_000);
                c.set_bitrate(codecs::SOUND_BITRATE);
                js_sys::Reflect::set(&c, &own.into(), &settings)?;
                if web_sys::AudioEncoder::is_config_supported(&c).await?.get_supported() == Some(true) {
                    chosen = Some((own, c));
                    break;
                }
            }
            let Some((own, c)) = chosen else { return Err(js_sys::Error::new("this browser can encode neither AAC nor Opus").into()) };
            for t in 0..spec.tracks {
                let encoder = web_sys::AudioEncoder::new(&web_sys::AudioEncoderInit::new(error.unchecked_ref(), output(1 + t).unchecked_ref()))?;
                encoder.configure(&c)?;
                audio.push(encoder);
            }
            opus = own == "opus";
            // (Opus says its own: the pre-skip, its encoder's lookahead.)
            if !opus {
                audio_delay = aac_delay(&c).await?;
            }
        }
        Ok(Session { spec, frame_us, video, audio, opus, audio_delay, made, failed, frames: 0, samples: vec![0; spec.tracks] })
    }

    /// A frame's NV12 coded, and each sound track's sound with it (stereo, interleaved, at 48 kHz).
    fn frame(&mut self, nv12: &[u8], sounds: &[Vec<f32>]) -> Result<(), JsValue> {
        check(&self.failed)?;
        let color = web_sys::VideoColorSpaceInit::new();
        color.set_primaries(Some(web_sys::VideoColorPrimaries::Bt709));
        color.set_transfer(Some(web_sys::VideoTransferCharacteristics::Bt709));
        color.set_matrix(Some(web_sys::VideoMatrixCoefficients::Bt709));
        color.set_full_range(Some(false));
        let init = web_sys::VideoFrameBufferInit::new(self.spec.height, self.spec.width, web_sys::VideoPixelFormat::Nv12, 0);
        init.set_color_space(&color);
        init.set_timestamp_f64((self.frames as f64 * self.frame_us).round());
        init.set_duration_f64(self.frame_us.round());
        let frame = web_sys::VideoFrame::new_with_u8_array_and_video_frame_buffer_init(&copied(nv12), &init)?;
        // (A key frame every `KEY_FRAMES`: where the MP4's chunks end, and seeking lands.)
        let options = web_sys::VideoEncoderEncodeOptions::new();
        options.set_key_frame(self.frames.is_multiple_of(u64::from(codecs::KEY_FRAMES)));
        let encoded = self.video.encode_with_options(&frame, &options);
        frame.close();
        encoded?;
        self.frames += 1;
        for (t, sound) in sounds.iter().enumerate() {
            let Some(encoder) = self.audio.get(t).filter(|_| !sound.is_empty()) else { continue };
            let data = audio_data(sound, (self.samples[t] as f64 * 1e6 / 48_000.0).round())?;
            let encoded = encoder.encode(&data);
            data.close();
            encoded?;
            self.samples[t] += (sound.len() / 2) as u64;
        }
        Ok(())
    }

    /// How many frames and sounds wait for the codecs.
    fn waiting(&self) -> u32 {
        self.video.encode_queue_size() + self.audio.iter().map(|a| a.encode_queue_size()).sum::<u32>()
    }

    /// What the codecs made since it was last taken.
    fn take(&self) -> Result<Vec<Chunk>, JsValue> {
        check(&self.failed)?;
        Ok(std::mem::take(&mut *self.made.borrow_mut()))
    }

    /// Everything given coded, and the codecs closed.
    async fn flush(&self) -> Result<(), JsValue> {
        self.video.flush().await?;
        for a in &self.audio {
            a.flush().await?;
        }
        self.video.close()?;
        for a in &self.audio {
            a.close()?;
        }
        check(&self.failed)
    }

    fn close(&self) {
        if self.video.state() != web_sys::CodecState::Closed {
            let _ = self.video.close();
        }
        for a in self.audio.iter().filter(|a| a.state() != web_sys::CodecState::Closed) {
            let _ = a.close();
        }
    }
}

/// The codecs' thread: every frame `taken` gives coded, what is made told on `tell` as it comes. Ended by the
/// feeder's Finish (everything flushed), or by its letting go (canceled: nothing more is made).
async fn encode(spec: Spec, taken: &Receiver<Msg>, tell: &mpsc::Sender<Said>) -> Result<(), String> {
    let frame_us = spec.frame as f64 * 1e6 / spec.timescale as f64;
    let mut session = Session::open(spec, frame_us).await.map_err(said)?;
    let (opus, priming) = (session.opus, session.audio_delay);
    // (Each sound track's coding, said with its first chunk.)
    let mut said_coding = vec![false; spec.tracks];
    let take = |session: &Session, said_coding: &mut Vec<bool>| -> Result<(), String> {
        for chunk in session.take().map_err(said)? {
            if chunk.track == 0 {
                let _ = tell.send(Said::Made(Coded::Frame(H264::AnnexB(chunk.data))));
                continue;
            }
            let t = chunk.track - 1;
            if !said_coding[t] {
                let sound = if opus {
                    Sound::Opus { pre_skip: OPUS_PRE_SKIP }
                } else {
                    Sound::Aac { config: chunk.config.ok_or("the AAC encoder said no AudioSpecificConfig")?, priming }
                };
                said_coding[t] = true;
                let _ = tell.send(Said::Made(Coded::Coding(t, sound)));
            }
            let _ = tell.send(Said::Made(Coded::Sound(t, chunk.data)));
        }
        Ok(())
    };
    loop {
        match taken.try_recv() {
            Ok(Msg::Frame(nv12, sounds)) => {
                if let Err(why) = session.frame(&nv12, &sounds) {
                    session.close();
                    return Err(said(why));
                }
                while session.waiting() > 4 {
                    sleep(1).await;
                }
            }
            Ok(Msg::Finish) => break,
            Err(TryRecvError::Empty) => sleep(1).await,
            Err(TryRecvError::Disconnected) => {
                session.close();
                return Ok(());
            }
        }
        if let Err(why) = take(&session, &mut said_coding) {
            session.close();
            return Err(why);
        }
    }
    session.flush().await.map_err(said)?;
    take(&session, &mut said_coding)
}
