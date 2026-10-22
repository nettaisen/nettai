//! The browser's encoder ([`crate::encode`]): WebCodecs, the browser's own H.264 for the picture, and AAC (where
//! the browser has it, else Opus) for each sound track, a console shown. The MP4 is the same as the desktop's
//! ([`crate::mp4`]).
//!
//! [`Encoder`] is fed on a thread that may wait (a worker: never the page's main thread), a frame at a time. The
//! encoding itself runs on a thread of its own, on its event loop, which is where WebCodecs answers: the frames go
//! to it through a short queue (a frame waits while it is full, so the encoder sets the pace), and what it makes
//! comes back through another. The sound comes at 48 kHz, the rate both codecs take ([`crate::resample`]: the
//! samples ffmpeg is given on the desktop).
//!
//! **AAC's delay.** An AAC encoder puts samples before the sound (its priming: AAC's blocks overlap, and an encoder
//! looks ahead), which no AAC packet says and WebCodecs doesn't tell: Chromium stamps the packets from 0, its macOS
//! encoder (AudioToolbox) putting 2112 first and its Windows one (Media Foundation) its own. So it is measured as an
//! export starts, a click through an encoder of the same settings and a decoder (`nettai_video_aac_delay`), and each
//! sound track is shown from past it (its edit list), as the desktop's are by ffmpeg's (the user: "is aac in the
//! browser broken then?", then "ok fix it then": before, the sound played late by it, 44 ms on macOS). Opus says its
//! own, its pre-skip (312, Chromium's libopus lookahead).
//!
//! **Times.** The picture's time scale is a frame's own (a tick is 4389/262144 s: the time scale 262144, a frame
//! 4389 of it), so its times are exact; the sound's is its samples (48000).

use crate::Error;
use crate::encode::{Audio, Backend, Encoding, Event, Kind, Packet, Track, Video};
use crate::resample::RATE;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// How many frames may wait for the encoder.
const QUEUE: usize = 8;

/// How long a frame waits for the encoder before it is taken to have stopped (its thread never started, or the
/// browser's codec never answered), in seconds.
const STALLED: u64 = 20;

/// What the encoder's thread is told of the video.
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
    audio_bitrate: u32,
}

enum Msg {
    /// A frame's pixels (RGBX) and each sound track's sound (stereo, interleaved, at 48 kHz).
    Frame(Vec<u8>, Vec<Vec<f32>>),
    Finish,
}

/// What the encoder's thread says: what it made, that it is done, or why it stopped.
enum Said {
    Made(Event),
    Done,
    Failed(String),
}

/// The browser's encoder for a video of `video` and `audio`, as `encoding` says (its titles and chapters are the
/// MP4's to write).
pub(crate) fn start(video: Video, audio: Option<Audio>, encoding: &Encoding) -> Result<Box<dyn Backend>, Error> {
    Ok(Box::new(Encoder::start(video, audio, encoding)?))
}

/// The browser encoding a video.
struct Encoder {
    frames: SyncSender<Msg>,
    said: Receiver<Said>,
    width: usize,
    height: usize,
}

impl Encoder {
    /// Start encoding the picture `video` and the sound `audio` as `encoding` says (its rate factor made a bit
    /// rate): the encoder's thread is started, and tells the browser's codecs.
    fn start(video: Video, audio: Option<Audio>, encoding: &Encoding) -> Result<Encoder, Error> {
        let (frame, timescale) = video.frame_time;
        // (A browser's encoder takes a bit rate: a generous one for the pixels' edges, less as the rate factor is
        // higher; at 16, a fifth of a bit a pixel a frame.)
        let per_pixel = 0.2 * 2f64.powf((16.0 - encoding.crf as f64) / 6.0);
        let bitrate = (video.width as f64 * video.height as f64 * timescale as f64 / frame as f64 * per_pixel).min(40e6) as u32;
        let spec = Spec {
            width: video.width,
            height: video.height,
            frame,
            timescale,
            tracks: audio.map_or(0, |a| a.tracks),
            bitrate,
            audio_bitrate: encoding.audio_kbps * 1000,
        };
        let (frames, taken) = mpsc::sync_channel(QUEUE);
        let (tell, said) = mpsc::channel();
        nettai_platform::thread::spawn_async("video", move || {
            Box::pin(async move {
                let ended = encode(spec, &taken, &tell).await;
                let _ = tell.send(match ended {
                    Ok(()) => Said::Done,
                    Err(why) => Said::Failed(why),
                });
            })
        })
        .map_err(|e| Error::Encoder(format!("the encoder's thread: {e}")))?;
        Ok(Encoder { frames, said, width: video.width as usize, height: video.height as usize })
    }

    /// What the encoder's thread said since, without waiting: what it made, or why it stopped.
    fn heard(&mut self, until_done: bool) -> Result<(Vec<Event>, bool), Error> {
        let mut made = Vec::new();
        loop {
            let said = if until_done { self.said.recv().ok() } else { self.said.try_recv().ok() };
            match said {
                Some(Said::Made(event)) => made.push(event),
                Some(Said::Done) => return Ok((made, true)),
                Some(Said::Failed(why)) => return Err(Error::Encoder(why)),
                None if until_done => return Err(Error::Encoder("the encoder stopped".into())),
                None => return Ok((made, false)),
            }
        }
    }
}

impl Backend for Encoder {
    fn frame(&mut self, rgb: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        assert_eq!(rgb.len(), self.width * self.height * 3, "a frame's pixels are its width by its height");
        let mut rgbx = vec![255u8; self.width * self.height * 4];
        for (x, px) in rgbx.chunks_exact_mut(4).zip(rgb.chunks_exact(3)) {
            x[..3].copy_from_slice(px);
        }
        let samples = sound.iter().map(|s| s.as_flattened().to_vec()).collect();
        let mut msg = Msg::Frame(rgbx, samples);
        let waited = nettai_platform::time::Instant::now();
        loop {
            match self.frames.try_send(msg) {
                Ok(()) => return Ok(rgb),
                // (The encoder is behind: this worker waits, as a worker may.)
                Err(TrySendError::Full(back)) => {
                    msg = back;
                    // (An encoder that failed lets go of its queue, below; one that never answers, this.)
                    if waited.elapsed().as_secs() >= STALLED {
                        return Err(Error::Encoder(format!("the browser's encoder took no frame for {STALLED} seconds")));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(TrySendError::Disconnected(_)) => return Err(self.heard(true).err().unwrap_or_else(|| Error::Encoder("the encoder stopped".into()))),
            }
        }
    }

    fn events(&mut self) -> Result<Vec<Event>, Error> {
        self.heard(false).map(|(made, _)| made)
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<Event>, Error> {
        let _ = self.frames.send(Msg::Finish);
        self.heard(true).map(|(made, _)| made)
    }

    fn cancel(self: Box<Self>) {
        // (Its queue let go, the encoder's thread stops of itself.)
    }
}

#[wasm_bindgen(inline_js = r#"
export function nettai_video_sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
// How many samples the AAC encoder `config` makes puts before the sound: its priming, which no AAC packet says and
// WebCodecs doesn't tell (Chromium stamps the packets from 0: macOS's AudioToolbox puts 2112 first). A click,
// encoded and decoded again: how much later it comes out.
export async function nettai_video_aac_delay(config) {
    const [at, length] = [3000, 16384];
    const click = new Float32Array(length * 2);
    click[at * 2] = click[at * 2 + 1] = 0.5;
    const chunks = [];
    let [decoderConfig, failed] = [null, null];
    const encoder = new AudioEncoder({ output: (chunk, meta) => { chunks.push(chunk); decoderConfig ??= meta?.decoderConfig; }, error: (e) => (failed = e) });
    encoder.configure(config);
    const data = new AudioData({ format: "f32", sampleRate: 48000, numberOfChannels: 2, numberOfFrames: length, timestamp: 0, data: click });
    encoder.encode(data);
    data.close();
    await encoder.flush();
    encoder.close();
    if (failed) throw failed;
    if (!decoderConfig) throw new Error("the AAC encoder said no decoder configuration");
    const heard = [];
    const decoder = new AudioDecoder({
        output: (d) => {
            const left = new Float32Array(d.numberOfFrames);
            d.copyTo(left, { planeIndex: 0, format: "f32-planar" });
            heard.push(left);
            d.close();
        },
        error: (e) => (failed = e),
    });
    decoder.configure(decoderConfig);
    for (const chunk of chunks) decoder.decode(chunk);
    await decoder.flush();
    decoder.close();
    if (failed) throw failed;
    let [n, place, loudest] = [0, -1, 0];
    for (const part of heard) {
        for (const v of part) {
            if (Math.abs(v) > loudest) [place, loudest] = [n, Math.abs(v)];
            n++;
        }
    }
    if (loudest < 0.1 || place < at) throw new Error(`the AAC encoder's delay couldn't be told (its click came out at ${place}, ${loudest} loud)`);
    return place - at;
}
export async function nettai_video_open(width, height, frameUs, bitrate, tracks, audioBitrate) {
    if (!self.VideoEncoder) throw new Error("this browser has no WebCodecs");
    const chunks = [];
    let failed = null;
    const out = (track) => (chunk, meta) => {
        // (A track's decoder configuration, with its first chunk: the picture's avcC, AAC's AudioSpecificConfig.)
        const description = meta?.decoderConfig?.description;
        const config = description ? new Uint8Array(ArrayBuffer.isView(description) ? description.buffer : description).slice() : null;
        const data = new Uint8Array(chunk.byteLength);
        chunk.copyTo(data);
        chunks.push([track, data, chunk.timestamp, chunk.duration ?? 0, chunk.type === "key", config]);
    };
    // H.264 at level 4.2 (two pictures side by side at four times the frame, 60 a second): High, Main, else the
    // constrained baseline every encoder has.
    let config = null;
    for (const codec of ["avc1.64002A", "avc1.4D402A", "avc1.42E02A"]) {
        const c = { codec, width, height, bitrate, framerate: 1e6 / frameUs, avc: { format: "avc" }, latencyMode: "quality" };
        if ((await VideoEncoder.isConfigSupported(c)).supported) { config = c; break; }
    }
    if (!config) throw new Error(`this browser can't encode H.264 at ${width}x${height}`);
    const video = new VideoEncoder({ output: out(0), error: (e) => (failed = e) });
    video.configure(config);
    let [audioCodec, audioDelay] = ["", 0];
    const audio = [];
    if (tracks > 0) {
        for (const [codec, extra, name] of [["mp4a.40.2", { aac: { format: "aac" } }, "aac"], ["opus", {}, "opus"]]) {
            const c = { codec, sampleRate: 48000, numberOfChannels: 2, bitrate: audioBitrate, ...extra };
            if ((await AudioEncoder.isConfigSupported(c)).supported) {
                for (let t = 0; t < tracks; t++) {
                    const encoder = new AudioEncoder({ output: out(1 + t), error: (e) => (failed = e) });
                    encoder.configure(c);
                    audio.push(encoder);
                }
                audioCodec = name;
                // (Opus says its own: the pre-skip, its encoder's lookahead.)
                if (name === "aac") audioDelay = await nettai_video_aac_delay(c);
                break;
            }
        }
        if (!audioCodec) throw new Error("this browser can encode neither AAC nor Opus");
    }
    let frames = 0;
    const samples = audio.map(() => 0);
    const check = () => { if (failed) throw failed; };
    return {
        audioCodec,
        audioDelay,
        frame(rgbx, sounds) {
            check();
            const f = new VideoFrame(rgbx, { format: "RGBX", codedWidth: width, codedHeight: height, timestamp: Math.round(frames * frameUs), duration: Math.round(frameUs) });
            // (A key frame every two seconds: where a fragment may start, and seeking lands.)
            video.encode(f, { keyFrame: frames % 120 === 0 });
            f.close();
            frames++;
            sounds.forEach((sound, t) => {
                if (!sound.length) return;
                const d = new AudioData({ format: "f32", sampleRate: 48000, numberOfChannels: 2, numberOfFrames: sound.length / 2, timestamp: Math.round(samples[t] * 1e6 / 48000), data: sound });
                audio[t].encode(d);
                d.close();
                samples[t] += sound.length / 2;
            });
        },
        waiting() { return video.encodeQueueSize + audio.reduce((n, a) => n + a.encodeQueueSize, 0); },
        take() { check(); return chunks.splice(0); },
        async flush() {
            await video.flush();
            for (const a of audio) await a.flush();
            video.close();
            for (const a of audio) a.close();
            check();
        },
        close() {
            if (video.state !== "closed") video.close();
            for (const a of audio) if (a.state !== "closed") a.close();
        },
    };
}
"#)]
extern "C" {
    fn nettai_video_sleep(ms: u32) -> js_sys::Promise;
    #[wasm_bindgen(catch)]
    fn nettai_video_open(width: u32, height: u32, frame_us: f64, bitrate: u32, tracks: u32, audio_bitrate: u32) -> Result<js_sys::Promise, JsValue>;

    type Session;
    #[wasm_bindgen(method, getter, js_name = audioCodec)]
    fn audio_codec(this: &Session) -> String;
    /// The samples the sound's encoder puts before the sound (AAC's priming; 0 for Opus, whose pre-skip says it).
    #[wasm_bindgen(method, getter, js_name = audioDelay)]
    fn audio_delay(this: &Session) -> u32;
    #[wasm_bindgen(method, catch)]
    fn frame(this: &Session, rgbx: &js_sys::Uint8Array, sounds: &js_sys::Array) -> Result<(), JsValue>;
    #[wasm_bindgen(method)]
    fn waiting(this: &Session) -> u32;
    #[wasm_bindgen(method, catch)]
    fn take(this: &Session) -> Result<js_sys::Array, JsValue>;
    #[wasm_bindgen(method, catch)]
    fn flush(this: &Session) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(method)]
    fn close(this: &Session);
}

/// What the browser said went wrong, in words.
fn said(why: JsValue) -> String {
    why.dyn_ref::<js_sys::Error>().map(|e| String::from(e.message())).unwrap_or_else(|| format!("{why:?}"))
}

async fn sleep(ms: u32) {
    let _ = JsFuture::from(nettai_video_sleep(ms)).await;
}

/// The tracks as their first chunks describe them: the picture's from its avcC, each sound track's from AAC's
/// AudioSpecificConfig (or Opus's own), and what is shown of each sound track: past its encoder's delay.
struct Described {
    spec: Spec,
    opus: bool,
    delay: u32,
    codecs: Vec<Option<mp4_atom::Codec>>,
    said: bool,
}

impl Described {
    fn codec(&mut self, track: usize, config: &[u8]) -> Result<(), String> {
        use mp4_atom::esds::{DecoderConfig, DecoderSpecific, EsDescriptor};
        use mp4_atom::*;
        if self.codecs[track].is_some() {
            return Ok(());
        }
        let audio = || Audio { data_reference_index: 1, channel_count: 2, sample_size: 16, sample_rate: FixedPoint::new(RATE as u16, 0) };
        let codec = if track == 0 {
            let avcc = Avcc::decode_body(&mut &config[..]).map_err(|e| format!("the picture's avcC: {e}"))?;
            Codec::Avc1(Avc1 {
                visual: Visual { data_reference_index: 1, width: self.spec.width as u16, height: self.spec.height as u16, ..Default::default() },
                avcc,
                ..Default::default()
            })
        } else if self.opus {
            Codec::Opus(Opus { audio: audio(), dops: Dops { output_channel_count: 2, pre_skip: 312, input_sample_rate: RATE, output_gain: 0 }, btrt: None })
        } else {
            Codec::Mp4a(Mp4a {
                audio: audio(),
                esds: Esds {
                    es_desc: EsDescriptor {
                        es_id: track as u16 + 1,
                        dec_config: DecoderConfig {
                            object_type_indication: 0x40,
                            stream_type: 5,
                            max_bitrate: self.spec.audio_bitrate,
                            avg_bitrate: self.spec.audio_bitrate,
                            dec_specific: Some(DecoderSpecific { profile: 2, freq_index: 3, chan_conf: 2, raw: config.to_vec() }),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                },
                btrt: None,
                taic: None,
            })
        };
        self.codecs[track] = Some(codec);
        Ok(())
    }

    /// The tracks, once every one is described.
    fn tracks(&mut self) -> Option<Event> {
        if self.said || self.codecs.iter().any(Option::is_none) {
            return None;
        }
        self.said = true;
        let s = self.spec;
        let tracks = self
            .codecs
            .iter()
            .enumerate()
            .map(|(i, codec)| Track {
                kind: if i == 0 { Kind::Video { width: s.width, height: s.height } } else { Kind::Audio },
                timescale: if i == 0 { s.timescale } else { RATE },
                codec: codec.clone().expect("described"),
                // (A sound track shown from past its encoder's delay, to its end: an edit list as ffmpeg's.)
                edits: (i > 0 && self.delay > 0).then(|| {
                    let entry = mp4_atom::ElstEntry { segment_duration: 0, media_time: Some(self.delay as u64), media_rate: mp4_atom::FixedPoint::new(1, 0) };
                    mp4_atom::Edts { elst: Some(mp4_atom::Elst { entries: vec![entry] }) }
                }),
            })
            .collect();
        Some(Event::Tracks(tracks))
    }
}

/// The encoder's thread: every frame `taken` gives encoded, what is made told on `tell` as it comes. Ended by the
/// feeder's Finish (everything flushed), or by its letting go (canceled: nothing more is made).
async fn encode(spec: Spec, taken: &Receiver<Msg>, tell: &mpsc::Sender<Said>) -> Result<(), String> {
    let frame_us = spec.frame as f64 * 1e6 / spec.timescale as f64;
    let session: Session = match nettai_video_open(spec.width, spec.height, frame_us, spec.bitrate, spec.tracks as u32, spec.audio_bitrate) {
        Ok(opening) => JsFuture::from(opening).await.map_err(said)?.unchecked_into(),
        Err(why) => return Err(said(why)),
    };
    let mut described = Described { spec, opus: session.audio_codec() == "opus", delay: session.audio_delay(), codecs: vec![None; 1 + spec.tracks], said: false };
    // Chunks wait till every track is described.
    let mut early: Vec<Packet> = Vec::new();
    let mut take = |described: &mut Described| -> Result<(), String> {
        for chunk in session.take().map_err(said)?.iter() {
            let chunk = js_sys::Array::from(&chunk);
            let track = chunk.get(0).as_f64().unwrap_or(0.0) as usize;
            if let Some(config) = chunk.get(5).dyn_ref::<js_sys::Uint8Array>() {
                described.codec(track, &config.to_vec())?;
            }
            let at = chunk.get(2).as_f64().unwrap_or(0.0);
            let (time, duration) = if track == 0 {
                // (The frame's own count, exact in the frame's time scale.)
                ((at / frame_us).round() as u64 * spec.frame as u64, spec.frame)
            } else {
                ((at * RATE as f64 / 1e6).round() as u64, (chunk.get(3).as_f64().unwrap_or(0.0) * RATE as f64 / 1e6).round() as u32)
            };
            early.push(Packet { track, data: js_sys::Uint8Array::new(&chunk.get(1)).to_vec(), time, offset: 0, duration, key: chunk.get(4).is_truthy() });
        }
        if let Some(tracks) = described.tracks() {
            let _ = tell.send(Said::Made(tracks));
        }
        if described.said {
            for packet in early.drain(..) {
                let _ = tell.send(Said::Made(Event::Packet(packet)));
            }
        }
        Ok(())
    };
    loop {
        match taken.try_recv() {
            Ok(Msg::Frame(rgbx, sounds)) => {
                // (Copied out of the shared memory: the browser takes no view of it.)
                let pixels = js_sys::Uint8Array::new_with_length(rgbx.len() as u32);
                pixels.copy_from(&rgbx);
                let tracks = js_sys::Array::new();
                for sound in &sounds {
                    let samples = js_sys::Float32Array::new_with_length(sound.len() as u32);
                    samples.copy_from(sound);
                    tracks.push(&samples);
                }
                if let Err(why) = session.frame(&pixels, &tracks) {
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
        if let Err(why) = take(&mut described) {
            session.close();
            return Err(why);
        }
    }
    JsFuture::from(session.flush().map_err(said)?).await.map_err(said)?;
    take(&mut described)
}
