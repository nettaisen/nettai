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
//!   video starts, a click through an encoder of the same settings and a decoder (`nettai_video_aac_delay`) (the
//!   user: "is aac in the browser broken then?", then "ok fix it then": before, the sound played late by it, 44 ms on
//!   macOS). AAC's config is the AudioSpecificConfig its first chunk's decoder configuration says.
//! - **Opus** (a browser with no AAC encoder) is asked for 20 ms frames, and says its own lookahead, its pre-skip:
//!   312, Chromium's libopus's.

use crate::Error;
use crate::codecs::{self, Audio, Codecs, Coded, H264, Sound, Video};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

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
        nettai_platform::thread::spawn_async("video", move || {
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
        let waited = nettai_platform::time::Instant::now();
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
export async function nettai_video_open(width, height, frameUs, bitrate, tracks, audioBitrate, keyFrames) {
    if (!self.VideoEncoder) throw new Error("this browser has no WebCodecs");
    const chunks = [];
    let failed = null;
    const out = (track) => (chunk, meta) => {
        // (A sound track's decoder configuration, with its first chunk: AAC's AudioSpecificConfig.)
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
        const c = { codec, width, height, bitrate, framerate: 1e6 / frameUs, avc: { format: "annexb" }, latencyMode: "quality" };
        if ((await VideoEncoder.isConfigSupported(c)).supported) { config = c; break; }
    }
    if (!config) throw new Error(`this browser can't encode H.264 at ${width}x${height}`);
    const video = new VideoEncoder({ output: out(0), error: (e) => (failed = e) });
    video.configure(config);
    let [audioCodec, audioDelay] = ["", 0];
    const audio = [];
    if (tracks > 0) {
        for (const [codec, extra, name] of [["mp4a.40.2", { aac: { format: "aac" } }, "aac"], ["opus", { opus: { frameDuration: 20000 } }, "opus"]]) {
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
        frame(nv12, sounds) {
            check();
            const colorSpace = { primaries: "bt709", transfer: "bt709", matrix: "bt709", fullRange: false };
            const f = new VideoFrame(nv12, { format: "NV12", codedWidth: width, codedHeight: height, colorSpace, timestamp: Math.round(frames * frameUs), duration: Math.round(frameUs) });
            // (A key frame every `keyFrames`: where the MP4's chunks end, and seeking lands.)
            video.encode(f, { keyFrame: frames % keyFrames === 0 });
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
    fn nettai_video_open(width: u32, height: u32, frame_us: f64, bitrate: u32, tracks: u32, audio_bitrate: u32, key_frames: u32) -> Result<js_sys::Promise, JsValue>;

    type Session;
    #[wasm_bindgen(method, getter, js_name = audioCodec)]
    fn audio_codec(this: &Session) -> String;
    /// The samples the sound's encoder puts before the sound (AAC's priming; 0 for Opus, whose pre-skip says it).
    #[wasm_bindgen(method, getter, js_name = audioDelay)]
    fn audio_delay(this: &Session) -> u32;
    #[wasm_bindgen(method, catch)]
    fn frame(this: &Session, nv12: &js_sys::Uint8Array, sounds: &js_sys::Array) -> Result<(), JsValue>;
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

/// The codecs' thread: every frame `taken` gives coded, what is made told on `tell` as it comes. Ended by the
/// feeder's Finish (everything flushed), or by its letting go (canceled: nothing more is made).
async fn encode(spec: Spec, taken: &Receiver<Msg>, tell: &mpsc::Sender<Said>) -> Result<(), String> {
    let frame_us = spec.frame as f64 * 1e6 / spec.timescale as f64;
    let opening = nettai_video_open(spec.width, spec.height, frame_us, spec.bitrate, spec.tracks as u32, codecs::SOUND_BITRATE, codecs::KEY_FRAMES);
    let session: Session = match opening {
        Ok(opening) => JsFuture::from(opening).await.map_err(said)?.unchecked_into(),
        Err(why) => return Err(said(why)),
    };
    let (opus, priming) = (session.audio_codec() == "opus", session.audio_delay());
    // (Each sound track's coding, said with its first chunk.)
    let mut said_coding = vec![false; spec.tracks];
    let take = |said_coding: &mut Vec<bool>| -> Result<(), String> {
        for chunk in session.take().map_err(said)?.iter() {
            let chunk = js_sys::Array::from(&chunk);
            let track = chunk.get(0).as_f64().unwrap_or(0.0) as usize;
            let data = js_sys::Uint8Array::new(&chunk.get(1)).to_vec();
            if track == 0 {
                let _ = tell.send(Said::Made(Coded::Frame(H264::AnnexB(data))));
                continue;
            }
            let t = track - 1;
            if !said_coding[t] {
                let sound = if opus {
                    Sound::Opus { pre_skip: OPUS_PRE_SKIP }
                } else {
                    let config = chunk.get(5).dyn_ref::<js_sys::Uint8Array>().map(js_sys::Uint8Array::to_vec).ok_or("the AAC encoder said no AudioSpecificConfig")?;
                    Sound::Aac { config, priming }
                };
                said_coding[t] = true;
                let _ = tell.send(Said::Made(Coded::Coding(t, sound)));
            }
            let _ = tell.send(Said::Made(Coded::Sound(t, data)));
        }
        Ok(())
    };
    loop {
        match taken.try_recv() {
            Ok(Msg::Frame(nv12, sounds)) => {
                // (Copied out of the shared memory: the browser takes no view of it.)
                let pixels = js_sys::Uint8Array::new_with_length(nv12.len() as u32);
                pixels.copy_from(&nv12);
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
        if let Err(why) = take(&mut said_coding) {
            session.close();
            return Err(why);
        }
    }
    JsFuture::from(session.flush().map_err(said)?).await.map_err(said)?;
    take(&mut said_coding)
}
