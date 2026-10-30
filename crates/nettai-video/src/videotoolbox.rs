//! iOS's encoder ([`crate::encode`]): VideoToolbox's H.264 for the picture and AudioToolbox's AAC for each sound
//! track, a console shown: the system's own encoders (the picture's in hardware on every iPhone). The MP4 is the
//! same as the desktop's ([`crate::mp4`]). macOS has the same frameworks, and the tests run this there.
//!
//! - **The picture** is given as NV12 in the video range, made of the frames' RGB here ([`crate::yuv`]: BT.709, as
//!   the desktop's ffmpeg makes it of the same frames, and as Android's encoder is given it), each frame written
//!   into a pixel buffer of the session's pool. H.264 High at the level its size needs, at a bit rate made of the
//!   rate factor as the browser's is ([`bitrate`]), a key frame at least every 120 frames, and no frame reordering
//!   (no B-frames): each frame is decoded when it is shown, so a packet's time is its frame's count on the tick's
//!   time base, exact. Tagged BT.709 throughout. The encoder answers on a thread of its own ([`encoded`]); what it
//!   made is taken from there as the frames go in ([`Backend::events`]), and a feeder far ahead waits for it.
//! - **The sound** is AAC-LC at the bit rate asked, of the 48 kHz samples as they come (the same samples ffmpeg
//!   and the browser are given), a converter a track. Its first packets hold its priming, the samples it puts
//!   before the sound (AudioToolbox's 2112, as it says them: `kAudioConverterPrimeInfo`), which each track's edit
//!   list leaves out, as the desktop's and the browser's do. A packet is 1024 samples; the last runs past the
//!   sound's end by the rest of its frame.
//! - **4:4:4** isn't coded here: the picture is 4:2:0 whatever [`Encoding::chroma`] says, as in a browser.

use crate::Error;
use crate::encode::{Audio, Backend, Encoding, Event, Kind, Packet, Track, Video};
use crate::resample::RATE;
use objc2_audio_toolbox::{
    AudioConverterDispose, AudioConverterFillComplexBuffer, AudioConverterGetProperty, AudioConverterNew, AudioConverterPrimeInfo, AudioConverterRef,
    AudioConverterSetProperty, kAudioConverterEncodeBitRate, kAudioConverterPrimeInfo, kAudioConverterPropertyMaximumOutputPacketSize,
};
use objc2_core_audio_types::{
    AudioBuffer, AudioBufferList, AudioStreamBasicDescription, AudioStreamPacketDescription, kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked,
    kAudioFormatLinearPCM, kAudioFormatMPEG4AAC,
};
use objc2_core_foundation::{CFData, CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_media::{CMSampleBuffer, CMTime, kCMFormatDescriptionExtension_SampleDescriptionExtensionAtoms, kCMSampleAttachmentKey_NotSync, kCMTimeInvalid};
use objc2_core_video::{
    CVAttachmentMode, CVPixelBuffer, CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferLockBaseAddress,
    CVPixelBufferLockFlags, CVPixelBufferPool, CVPixelBufferUnlockBaseAddress, kCVImageBufferColorPrimariesKey, kCVImageBufferColorPrimaries_ITU_R_709_2,
    kCVImageBufferTransferFunctionKey, kCVImageBufferTransferFunction_ITU_R_709_2, kCVImageBufferYCbCrMatrixKey, kCVImageBufferYCbCrMatrix_ITU_R_709_2,
    kCVPixelBufferHeightKey, kCVPixelBufferPixelFormatTypeKey, kCVPixelBufferWidthKey, kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, kCVReturnSuccess,
};
use objc2_video_toolbox::{
    VTCompressionSession, VTEncodeInfoFlags, VTSessionSetProperty, kVTCompressionPropertyKey_AllowFrameReordering, kVTCompressionPropertyKey_AverageBitRate,
    kVTCompressionPropertyKey_ColorPrimaries, kVTCompressionPropertyKey_MaxKeyFrameInterval, kVTCompressionPropertyKey_ProfileLevel,
    kVTCompressionPropertyKey_RealTime, kVTCompressionPropertyKey_TransferFunction, kVTCompressionPropertyKey_YCbCrMatrix, kVTProfileLevel_H264_High_AutoLevel,
};
use std::ffi::c_void;
use std::ptr::{self, NonNull};
use std::sync::{Arc, Mutex};

/// H.264's code in a format description (`'avc1'`).
const H264: u32 = u32::from_be_bytes(*b"avc1");

/// The most frames given to the picture's encoder before what it made of them is waited for.
const AHEAD: u64 = 6;

/// AAC-LC's object type, as an AudioStreamBasicDescription's flags say it (`kMPEG4Object_AAC_LC`).
const AAC_LC: u32 = 2;

/// An AAC packet's samples.
const AAC_FRAME: u32 = 1024;

/// What the sound's input says when it has nothing more to give for now (no error AudioToolbox has: the
/// converter stops and says it back).
const WAIT_FOR_MORE: i32 = i32::from_be_bytes(*b"more");

/// iOS's encoder for a video of `video` and `audio`, as `encoding` says (its titles and chapters are the MP4's to
/// write).
pub(crate) fn start(video: Video, audio: Option<Audio>, encoding: &Encoding) -> Result<Box<dyn Backend>, Error> {
    let picture = Picture::start(video, bitrate(&video, encoding))?;
    let sound = (0..audio.map_or(0, |a| a.tracks)).map(|t| Aac::start(1 + t, encoding.audio_kbps * 1000)).collect::<Result<Vec<_>, _>>()?;
    Ok(Box::new(Encoder { picture, sound, said: false, early: Vec::new(), nv12: Vec::new() }))
}

/// The picture's bit rate: a generous one for the pixels' edges, less as the rate factor is higher (at 16, a fifth
/// of a bit a pixel a frame), as the browser's encoder is given (`webcodecs`), at most 40 Mb/s.
pub(crate) fn bitrate(video: &Video, encoding: &Encoding) -> u32 {
    let (frame, timescale) = video.frame_time;
    let per_pixel = 0.2 * 2f64.powf((16.0 - encoding.crf as f64) / 6.0);
    (video.width as f64 * video.height as f64 * timescale as f64 / frame as f64 * per_pixel).min(40e6) as u32
}

/// iOS encoding a video.
struct Encoder {
    picture: Picture,
    sound: Vec<Aac>,
    /// The tracks are said: the picture's first sample said its parameter sets.
    said: bool,
    /// What was made before.
    early: Vec<Packet>,
    nv12: Vec<u8>,
}

impl Encoder {
    /// What was made since: the tracks once they are known, then every packet; `wait`: everything given is made
    /// first.
    fn take(&mut self, wait: bool) -> Result<Vec<Event>, Error> {
        if wait {
            self.picture.complete()?;
        }
        let (packets, avcc) = self.picture.made()?;
        self.early.extend(packets);
        for aac in &mut self.sound {
            self.early.append(&mut aac.packets);
        }
        let mut events = Vec::new();
        if !self.said {
            let Some(avcc) = avcc else { return Ok(events) };
            events.push(Event::Tracks(self.tracks(&avcc)?));
            self.said = true;
        }
        events.extend(self.early.drain(..).map(Event::Packet));
        Ok(events)
    }

    /// The tracks: the picture's from the avcC VideoToolbox made, each sound track's AAC-LC, shown from past its
    /// priming.
    fn tracks(&self, avcc: &[u8]) -> Result<Vec<Track>, Error> {
        use mp4_atom::esds::{DecoderConfig, DecoderSpecific, EsDescriptor};
        use mp4_atom::{Atom, Avc1, Avcc, Codec, Colr, Edts, Elst, ElstEntry, Esds, FixedPoint, Mp4a, Visual};
        let p = &self.picture;
        let avcc = Avcc::decode_body(&mut &avcc[..]).map_err(|e| Error::Encoder(format!("VideoToolbox's avcC: {e}")))?;
        let visual = Visual { data_reference_index: 1, width: p.width as u16, height: p.height as u16, ..Default::default() };
        // (BT.709 in the video range, as its SPS says it, and as ffmpeg's MP4 says it.)
        let colr = Colr::Nclx { colour_primaries: 1, transfer_characteristics: 1, matrix_coefficients: 1, full_range_flag: false };
        let codec = Codec::Avc1(Avc1 { visual, avcc, colr: Some(colr), ..Default::default() });
        let mut tracks = vec![Track { kind: Kind::Video { width: p.width, height: p.height }, timescale: p.timescale, codec, edits: None }];
        for aac in &self.sound {
            let esds = Esds {
                es_desc: EsDescriptor {
                    es_id: aac.track as u16 + 1,
                    dec_config: DecoderConfig {
                        object_type_indication: 0x40,
                        stream_type: 5,
                        max_bitrate: aac.bitrate,
                        avg_bitrate: aac.bitrate,
                        // (AAC-LC, 48 kHz, stereo: its AudioSpecificConfig, 0x1190.)
                        dec_specific: Some(DecoderSpecific { profile: 2, freq_index: 3, chan_conf: 2, raw: vec![0x11, 0x90] }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            };
            let audio = mp4_atom::Audio { data_reference_index: 1, channel_count: 2, sample_size: 16, sample_rate: FixedPoint::new(RATE as u16, 0) };
            // (Shown from past the priming, to the end: an edit list as ffmpeg's.)
            let entry = ElstEntry { segment_duration: 0, media_time: Some(aac.leading as u64), media_rate: FixedPoint::new(1, 0) };
            tracks.push(Track {
                kind: Kind::Audio,
                timescale: RATE,
                codec: Codec::Mp4a(Mp4a { audio, esds, btrt: None, taic: None }),
                edits: (aac.leading > 0).then(|| Edts { elst: Some(Elst { entries: vec![entry] }) }),
            });
        }
        Ok(tracks)
    }
}

impl Backend for Encoder {
    fn frame(&mut self, rgb: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        crate::yuv::nv12(&rgb, self.picture.width as usize, self.picture.height as usize, &mut self.nv12);
        self.picture.frame(&self.nv12)?;
        for (aac, samples) in self.sound.iter_mut().zip(sound) {
            aac.push(samples)?;
        }
        Ok(rgb)
    }

    fn events(&mut self) -> Result<Vec<Event>, Error> {
        self.take(false)
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<Event>, Error> {
        for aac in &mut self.sound {
            aac.finish()?;
        }
        let made = self.take(true)?;
        if !self.said {
            return Err(Error::Encoder("VideoToolbox made no picture".into()));
        }
        Ok(made)
    }

    fn cancel(self: Box<Self>) {
        // (Dropped: the session is ended without waiting, the converters let go.)
    }
}

/// What the picture's encoder made, told from its thread ([`encoded`]).
#[derive(Default)]
struct Made {
    packets: Vec<Packet>,
    /// Its avcC, from the first sample's format description.
    avcc: Option<Vec<u8>>,
    /// How many frames it answered for (made, or dropped).
    answered: u64,
    failed: Option<String>,
    /// A frame's time, in the picture's time scale.
    frame: u32,
}

/// VideoToolbox's H.264 session for the picture.
struct Picture {
    session: Session,
    pool: CFRetained<CVPixelBufferPool>,
    width: u32,
    height: u32,
    frame: u32,
    timescale: u32,
    given: u64,
}

/// A compression session and what its thread tells ([`encoded`]): the session holds a reference of its own to it,
/// given back once the session is invalidated (dropped), when it calls back no more.
struct Session {
    session: CFRetained<VTCompressionSession>,
    made: Arc<Mutex<Made>>,
    held: *const Mutex<Made>,
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: the session is valid; invalidated, it calls back no more, and its reference is let go once.
        unsafe {
            self.session.invalidate();
            drop(Arc::from_raw(self.held));
        }
    }
}

/// The session's thread: a frame encoded (or dropped, or failed), told to the [`Made`] at `refcon`.
unsafe extern "C-unwind" fn encoded(refcon: *mut c_void, _frame: *mut c_void, status: i32, _flags: VTEncodeInfoFlags, sample: *mut CMSampleBuffer) {
    // SAFETY: `refcon` is the `Arc<Mutex<Made>>`'s pointer the session was made with, which `Picture` keeps alive
    // until the session is invalidated (no callback after that).
    let made = unsafe { &*(refcon as *const Mutex<Made>) };
    let mut made = made.lock().unwrap_or_else(|e| e.into_inner());
    made.answered += 1;
    if status != 0 {
        made.failed.get_or_insert_with(|| format!("VideoToolbox failed a frame (status {status})"));
        return;
    }
    // (A frame dropped: none to tell. This session drops none, at no real-time limit.)
    // SAFETY: a sample the session made is valid for this call.
    let Some(sample) = (unsafe { sample.as_ref() }) else { return };
    // SAFETY: the sample is valid for this call.
    match unsafe { packet(sample, made.frame) } {
        Ok((packet, avcc)) => {
            if made.avcc.is_none() {
                made.avcc = avcc;
            }
            made.packets.push(packet);
        }
        Err(why) => {
            made.failed.get_or_insert(why);
        }
    }
}

/// The packet of a sample VideoToolbox made, and its avcC (from its format description).
///
/// # Safety
/// `sample` is valid.
unsafe fn packet(sample: &CMSampleBuffer, frame: u32) -> Result<(Packet, Option<Vec<u8>>), String> {
    // SAFETY: the sample is valid; what it gives is retained.
    let (data, shown, attachments, description) = unsafe { (sample.data_buffer(), sample.presentation_time_stamp(), sample.sample_attachments_array(false), sample.format_description()) };
    let data = data.ok_or("a sample with no data")?;
    // SAFETY: the block buffer is valid; `bytes` is as long as it says it is.
    let bytes = unsafe {
        let mut bytes = vec![0u8; data.data_length()];
        if !bytes.is_empty() && data.copy_data_bytes(0, bytes.len(), NonNull::new_unchecked(bytes.as_mut_ptr().cast())) != 0 {
            return Err("its sample's bytes couldn't be read".into());
        }
        bytes
    };
    // (A sample that isn't a sync sample says so; one that says nothing is a key frame.)
    // SAFETY: the array, if any, holds a dictionary a sample, and the key is a CFString.
    let key = !attachments.is_some_and(|array| unsafe {
        array.count() > 0 && {
            let first = array.value_at_index(0) as *const CFDictionary;
            first.as_ref().is_some_and(|d| d.contains_ptr_key((kCMSampleAttachmentKey_NotSync as *const CFString).cast()))
        }
    });
    let avcc = description.and_then(|d| {
        // SAFETY: the description is valid; its extension is a dictionary of atoms by name, `avcC` a CFData.
        unsafe {
            let atoms = d.extension(kCMFormatDescriptionExtension_SampleDescriptionExtensionAtoms)?.downcast::<CFDictionary>().ok()?;
            let name = CFString::from_static_str("avcC");
            let value = (atoms.value((&*name as *const CFString).cast()) as *const CFType).as_ref()?;
            Some(value.downcast_ref::<CFData>()?.to_vec())
        }
    });
    // (Its time is its frame's, in the time scale the frames were given in: no reordering, so decoded as shown.)
    Ok((Packet { track: 0, data: bytes, time: shown.value.max(0) as u64, offset: 0, duration: frame, key }, avcc))
}

/// A `CFNumber` of `value`.
fn number(value: i32) -> CFRetained<CFNumber> {
    CFNumber::new_i32(value)
}

impl Picture {
    fn start(video: Video, bitrate: u32) -> Result<Picture, Error> {
        let (frame, timescale) = video.frame_time;
        let failed = |what: &str, status: i32| Error::Encoder(format!("VideoToolbox: {what} (status {status})"));
        let (format, width, height) = (number(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange as i32), number(video.width as i32), number(video.height as i32));
        let values: [&CFType; 3] = [&format, &width, &height];
        // SAFETY: the keys are CoreVideo's own statics.
        let attributes = unsafe { CFDictionary::<CFString, CFType>::from_slices(&[kCVPixelBufferPixelFormatTypeKey, kCVPixelBufferWidthKey, kCVPixelBufferHeightKey], &values) };
        let made = Arc::new(Mutex::new(Made { frame, ..Made::default() }));
        let held = Arc::into_raw(made.clone());
        let mut session: *mut VTCompressionSession = ptr::null_mut();
        // SAFETY: every pointer is valid for the call; the callback's reference is `held`, which the session keeps till
        // it is invalidated.
        let status = unsafe {
            VTCompressionSession::create(None, video.width as i32, video.height as i32, H264, None, Some(attributes.as_opaque()), None, Some(encoded), held as *mut c_void, NonNull::from(&mut session))
        };
        let Some(session) = NonNull::new(session).filter(|_| status == 0) else {
            // SAFETY: no session was made: the reference is this function's to let go.
            drop(unsafe { Arc::from_raw(held) });
            return Err(failed("no H.264 encoder", status));
        };
        // SAFETY: the session was made for us (+1), and holds `held` from here.
        let session = Session { session: unsafe { CFRetained::from_raw(session) }, made, held };
        let (rate, keys) = (number(bitrate as i32), number(120));
        // SAFETY: the keys are VideoToolbox's statics, each given a value of the kind it takes.
        unsafe {
            let no: &CFType = objc2_core_foundation::CFBoolean::new(false);
            let settings: [(&CFString, &CFType); 8] = [
                (kVTCompressionPropertyKey_RealTime, no),
                (kVTCompressionPropertyKey_AllowFrameReordering, no),
                (kVTCompressionPropertyKey_ProfileLevel, kVTProfileLevel_H264_High_AutoLevel),
                (kVTCompressionPropertyKey_AverageBitRate, &rate),
                (kVTCompressionPropertyKey_MaxKeyFrameInterval, &keys),
                (kVTCompressionPropertyKey_ColorPrimaries, kCVImageBufferColorPrimaries_ITU_R_709_2),
                (kVTCompressionPropertyKey_TransferFunction, kCVImageBufferTransferFunction_ITU_R_709_2),
                (kVTCompressionPropertyKey_YCbCrMatrix, kCVImageBufferYCbCrMatrix_ITU_R_709_2),
            ];
            for (key, value) in settings {
                let status = VTSessionSetProperty(&session.session, key, Some(value));
                if status != 0 {
                    return Err(failed(&format!("its property {key}"), status));
                }
            }
        }
        // SAFETY: the session is valid.
        let pool = unsafe { session.session.pixel_buffer_pool() }.ok_or_else(|| failed("no pixel buffer pool", 0))?;
        Ok(Picture { session, pool, width: video.width, height: video.height, frame, timescale, given: 0 })
    }

    /// One more frame, its NV12 `nv12`, given at its time; waits while the encoder is far behind.
    fn frame(&mut self, nv12: &[u8]) -> Result<(), Error> {
        let (width, height) = (self.width as usize, self.height as usize);
        let mut buffer: *mut CVPixelBuffer = ptr::null_mut();
        // SAFETY: the pool is the session's; `buffer` is written with a retained buffer on success.
        let status = unsafe { CVPixelBufferPool::create_pixel_buffer(None, &self.pool, NonNull::from(&mut buffer)) };
        let Some(buffer) = NonNull::new(buffer).filter(|_| status == kCVReturnSuccess) else {
            return Err(Error::Encoder(format!("VideoToolbox: no pixel buffer (status {status})")));
        };
        // SAFETY: made by the pool, retained for us.
        let buffer = unsafe { CFRetained::from_raw(buffer) };
        // SAFETY: the buffer is valid; it is unlocked below.
        if unsafe { CVPixelBufferLockBaseAddress(&buffer, CVPixelBufferLockFlags::empty()) } != kCVReturnSuccess {
            return Err(Error::Encoder("VideoToolbox: its pixel buffer couldn't be locked".into()));
        }
        // The two planes, a row at a time: the luma's `height` rows, the chroma's half as many, each `width` bytes.
        for (plane, rows, from) in [(0, height, &nv12[..width * height]), (1, height / 2, &nv12[width * height..])] {
            let base = CVPixelBufferGetBaseAddressOfPlane(&buffer, plane).cast::<u8>();
            let stride = CVPixelBufferGetBytesPerRowOfPlane(&buffer, plane);
            assert!(!base.is_null() && stride >= width, "a locked 4:2:0 pixel buffer has both planes");
            for (row, bytes) in from.chunks_exact(width).take(rows).enumerate() {
                // SAFETY: the buffer is locked; the plane holds `rows` rows of `stride` bytes, each at least `width`.
                unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), base.add(row * stride), width) };
            }
        }
        // SAFETY: locked above.
        unsafe { CVPixelBufferUnlockBaseAddress(&buffer, CVPixelBufferLockFlags::empty()) };
        // SAFETY: the keys and values are CoreVideo's own.
        unsafe {
            for (key, value) in [
                (kCVImageBufferColorPrimariesKey, kCVImageBufferColorPrimaries_ITU_R_709_2),
                (kCVImageBufferTransferFunctionKey, kCVImageBufferTransferFunction_ITU_R_709_2),
                (kCVImageBufferYCbCrMatrixKey, kCVImageBufferYCbCrMatrix_ITU_R_709_2),
            ] {
                buffer.set_attachment(key, value, CVAttachmentMode::ShouldPropagate);
            }
        }
        // SAFETY: the times are valid CMTimes in the picture's time scale; the buffer is retained by the session.
        let status = unsafe {
            let at = CMTime::new((self.given * self.frame as u64) as i64, self.timescale as i32);
            self.session.session.encode_frame(&buffer, at, CMTime::new(self.frame as i64, self.timescale as i32), None, ptr::null_mut(), ptr::null_mut())
        };
        if status != 0 {
            return Err(Error::Encoder(format!("VideoToolbox refused a frame (status {status})")));
        }
        self.given += 1;
        // (Far ahead of what it made: wait till it made all but a few.)
        let answered = self.session.made.lock().unwrap_or_else(|e| e.into_inner()).answered;
        if self.given - answered > AHEAD {
            // SAFETY: a valid time in the picture's time scale.
            let until = unsafe { CMTime::new(((self.given - AHEAD / 2) * self.frame as u64) as i64, self.timescale as i32) };
            // SAFETY: the session is valid.
            let status = unsafe { self.session.session.complete_frames(until) };
            if status != 0 {
                return Err(Error::Encoder(format!("VideoToolbox: its frames couldn't be completed (status {status})")));
            }
        }
        Ok(())
    }

    /// Every frame given made.
    fn complete(&mut self) -> Result<(), Error> {
        // SAFETY: the session is valid; an invalid time completes every frame.
        match unsafe { self.session.session.complete_frames(kCMTimeInvalid) } {
            0 => Ok(()),
            status => Err(Error::Encoder(format!("VideoToolbox: its frames couldn't be completed (status {status})"))),
        }
    }

    /// What was made since, and its avcC once known; or why it failed.
    fn made(&mut self) -> Result<(Vec<Packet>, Option<Vec<u8>>), Error> {
        let mut made = self.session.made.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(why) = made.failed.take() {
            return Err(Error::Encoder(why));
        }
        Ok((std::mem::take(&mut made.packets), made.avcc.clone()))
    }
}

/// AudioToolbox's AAC encoder for a sound track.
struct Aac {
    converter: AudioConverterRef,
    /// The track's index in the video.
    track: usize,
    bitrate: u32,
    /// The samples it puts before the sound.
    leading: u32,
    /// The largest packet it makes.
    most: usize,
    /// Samples given and not yet taken (stereo, interleaved); and those the converter took last, which it may read
    /// till it asks again.
    waiting: Vec<f32>,
    lent: Vec<f32>,
    /// No more samples come: what is left is flushed.
    ending: bool,
    /// The packets made, and the time of the next (in samples).
    packets: Vec<Packet>,
    time: u64,
}

impl Aac {
    fn start(track: usize, bitrate: u32) -> Result<Aac, Error> {
        let failed = |what: &str, status: i32| Error::Encoder(format!("AudioToolbox's AAC: {what} (status {status})"));
        let mut from = AudioStreamBasicDescription {
            mSampleRate: RATE as f64,
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked,
            mBytesPerPacket: 8,
            mFramesPerPacket: 1,
            mBytesPerFrame: 8,
            mChannelsPerFrame: 2,
            mBitsPerChannel: 32,
            mReserved: 0,
        };
        let mut to = AudioStreamBasicDescription {
            mSampleRate: RATE as f64,
            mFormatID: kAudioFormatMPEG4AAC,
            mFormatFlags: AAC_LC,
            mBytesPerPacket: 0,
            mFramesPerPacket: AAC_FRAME,
            mBytesPerFrame: 0,
            mChannelsPerFrame: 2,
            mBitsPerChannel: 0,
            mReserved: 0,
        };
        let mut converter: AudioConverterRef = ptr::null_mut();
        // SAFETY: the descriptions and the out pointer are valid for the call.
        let status = unsafe { AudioConverterNew(NonNull::from(&mut from), NonNull::from(&mut to), NonNull::from(&mut converter)) };
        if status != 0 || converter.is_null() {
            return Err(failed("no encoder", status));
        }
        let mut aac = Aac { converter, track, bitrate, leading: 0, most: 0, waiting: Vec::new(), lent: Vec::new(), ending: false, packets: Vec::new(), time: 0 };
        // SAFETY: each property is read or written with a value of its own size.
        unsafe {
            let mut rate = bitrate;
            let status = AudioConverterSetProperty(converter, kAudioConverterEncodeBitRate, 4, NonNull::from(&mut rate).cast());
            if status != 0 {
                return Err(failed(&format!("a bit rate of {bitrate}"), status));
            }
            let mut prime = AudioConverterPrimeInfo { leadingFrames: 0, trailingFrames: 0 };
            let mut size = size_of::<AudioConverterPrimeInfo>() as u32;
            let status = AudioConverterGetProperty(converter, kAudioConverterPrimeInfo, NonNull::from(&mut size), NonNull::from(&mut prime).cast());
            if status != 0 {
                return Err(failed("its priming", status));
            }
            let mut most = 0u32;
            let mut size = 4u32;
            let status = AudioConverterGetProperty(converter, kAudioConverterPropertyMaximumOutputPacketSize, NonNull::from(&mut size), NonNull::from(&mut most).cast());
            if status != 0 || most == 0 {
                return Err(failed("its largest packet", status));
            }
            (aac.leading, aac.most) = (prime.leadingFrames, most as usize);
        }
        Ok(aac)
    }

    /// More samples: every whole packet they make is made.
    fn push(&mut self, samples: &[[f32; 2]]) -> Result<(), Error> {
        self.waiting.extend_from_slice(samples.as_flattened());
        while self.waiting.len() / 2 >= AAC_FRAME as usize {
            if !self.convert()? {
                break;
            }
        }
        Ok(())
    }

    /// No more samples: the rest made, the encoder's own lookahead with it.
    fn finish(&mut self) -> Result<(), Error> {
        self.ending = true;
        while self.convert()? {}
        Ok(())
    }

    /// A packet made of what waits, if one was: false when the converter wants more (or, ending, has made all).
    fn convert(&mut self) -> Result<bool, Error> {
        let mut out = vec![0u8; self.most];
        let mut list = AudioBufferList { mNumberBuffers: 1, mBuffers: [AudioBuffer { mNumberChannels: 2, mDataByteSize: out.len() as u32, mData: out.as_mut_ptr().cast() }] };
        let mut described = AudioStreamPacketDescription { mStartOffset: 0, mVariableFramesInPacket: 0, mDataByteSize: 0 };
        let mut packets = 1u32;
        // SAFETY: the converter is valid; the input proc is given `self`, borrowed for the call, and lends it `lent`,
        // which changes only as it asks again; the output list and description are valid for one packet of the
        // largest size.
        let status = unsafe {
            AudioConverterFillComplexBuffer(self.converter, Some(input), (self as *mut Aac).cast(), NonNull::from(&mut packets), NonNull::from(&mut list), &mut described)
        };
        if status != 0 && status != WAIT_FOR_MORE {
            return Err(Error::Encoder(format!("AudioToolbox's AAC failed (status {status})")));
        }
        if packets == 0 {
            return Ok(false);
        }
        let size = described.mDataByteSize as usize;
        out.truncate(size.min(list.mBuffers[0].mDataByteSize as usize));
        self.packets.push(Packet { track: self.track, data: out, time: self.time, offset: 0, duration: AAC_FRAME, key: true });
        self.time += AAC_FRAME as u64;
        Ok(true)
    }
}

/// The converter asks for samples: those waiting, lent to it till it asks again; or (none waiting) none, which ends
/// the sound when it is ending, and else stops the conversion till more come.
unsafe extern "C-unwind" fn input(_converter: AudioConverterRef, packets: NonNull<u32>, data: NonNull<AudioBufferList>, _described: *mut *mut AudioStreamPacketDescription, aac: *mut c_void) -> i32 {
    // SAFETY: `aac` is the `Aac` converting, borrowed for the conversion; the pointers are the converter's, valid for
    // this call.
    unsafe {
        let aac = &mut *aac.cast::<Aac>();
        let given = (aac.waiting.len() / 2).min(*packets.as_ptr() as usize);
        aac.lent.clear();
        aac.lent.extend(aac.waiting.drain(..given * 2));
        let list = &mut *data.as_ptr();
        list.mNumberBuffers = 1;
        list.mBuffers[0] = AudioBuffer { mNumberChannels: 2, mDataByteSize: (given * 8) as u32, mData: aac.lent.as_mut_ptr().cast() };
        *packets.as_ptr() = given as u32;
        if given == 0 && !aac.ending { WAIT_FOR_MORE } else { 0 }
    }
}

impl Drop for Aac {
    fn drop(&mut self) {
        // SAFETY: the converter is this one's, let go once.
        unsafe { AudioConverterDispose(self.converter) };
    }
}

#[cfg(test)]
mod tests {
    use super::start;
    use crate::tests::{given, options, viewer};
    use crate::{Encoding, Error, Mp4, Options, Part, Sides, play};
    use mp4_atom::{Codec, Decode, Ftyp, Moov, StszSamples};
    use nettai_frontend::viewer::Viewer;
    use std::ops::ControlFlow;

    /// `options`' MP4 of the replay `viewer` plays, made by iOS's encoders (here macOS's, the same frameworks).
    fn mp4(viewer: &mut Viewer, options: &Options, encoding: &Encoding) -> Result<Vec<u8>, Error> {
        let made = play(viewer, options, &mut |_| ControlFlow::Continue(()), |video, audio, info| Ok(Mp4::with(start(video, audio, encoding)?, video, info, Vec::new())))?.finish()?;
        Ok([made.head, made.body].concat())
    }

    /// The ticks the tests write: 120 frames of a set's first round.
    const PART: Part = Part::Ticks(200..320);

    /// iOS's encoders make the MP4: its head, then its samples; H.264 of every frame at the size asked, tagged
    /// BT.709 in the video range; and AAC of the sound, shown from past AudioToolbox's priming.
    #[test]
    fn videotoolbox_makes_the_mp4() {
        let content = nettai_match::testing::exe6_content();
        let (replay, _) = nettai_frontend::testing::recorded(&content, "exe6", 7);
        let file = mp4(&mut viewer(&content, &replay, 0), &options(2, Sides::One, &PART), &Encoding::default()).unwrap();
        let mut at = &file[..];
        Ftyp::decode(&mut at).unwrap();
        let moov = Moov::decode(&mut at).unwrap();
        assert_eq!(&at[4..8], b"mdat", "the samples follow the head");
        assert_eq!(moov.trak.len(), 2);
        let samples = |t: usize| match &moov.trak[t].mdia.minf.stbl.stsz.samples {
            StszSamples::Identical { count, .. } => *count as usize,
            StszSamples::Different { sizes } => sizes.len(),
        };
        let picture = &moov.trak[0];
        let Codec::Avc1(avc1) = &picture.mdia.minf.stbl.stsd.codecs[0] else { panic!("the picture isn't H.264") };
        assert_eq!((avc1.visual.width, avc1.visual.height, avc1.avcc.length_size), (480, 320, 4));
        assert_eq!(avc1.avcc.avc_profile_indication, 100, "High");
        assert_eq!(samples(0), 120);
        assert!(picture.edts.is_none(), "no reordering: the picture is shown from its first sample");
        let sound = &moov.trak[1];
        assert!(matches!(sound.mdia.minf.stbl.stsd.codecs[0], Codec::Mp4a(_)));
        let from = sound.edts.as_ref().and_then(|e| e.elst.as_ref()).map(|l| l.entries[0].media_time);
        assert_eq!(from, Some(Some(2112)), "the sound is shown from past AudioToolbox's priming");
        // (120 ticks are 96,450 samples at 48 kHz: with the priming, 97 or 98 AAC frames.)
        assert!((96..=99).contains(&samples(1)), "{} AAC frames", samples(1));
    }

    /// The MP4 plays as it was given (ffmpeg installed; `cargo test -p nettai-video -- --ignored`), as the desktop's
    /// does (`ffmpeg::tests::ffmpeg_makes_the_mp4`): every frame, close to the frames; the sound as long as it was
    /// given and at no lag from it (the priming left out); the colors tagged; the picture shown from 0.
    #[test]
    #[ignore]
    fn videotoolbox_mp4_plays_as_given() {
        use std::process::Command;
        let content = nettai_match::testing::exe6_content();
        let (replay, _) = nettai_frontend::testing::recorded(&content, "exe6", 7);
        let given = given(&mut viewer(&content, &replay, 0), &options(2, Sides::One, &PART), &mut |_| ControlFlow::Continue(())).unwrap();
        let pictures: Vec<u8> = given.frames.iter().flat_map(|(rgb, _)| rgb.iter().copied()).collect();
        let dir = std::env::temp_dir().join(format!("nettai-video-videotoolbox-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("part.mp4"), mp4(&mut viewer(&content, &replay, 0), &options(2, Sides::One, &PART), &Encoding::default()).unwrap()).unwrap();
        let run = |program: &str, args: &[&str]| -> Vec<u8> {
            let out = Command::new(program).current_dir(&dir).args(args).output().unwrap_or_else(|e| panic!("{program}: {e}"));
            assert!(out.status.success() && out.stderr.is_empty(), "{program} {args:?}: {}", String::from_utf8_lossy(&out.stderr));
            out.stdout
        };
        let entries = "stream=codec_name,profile,pix_fmt,width,height,r_frame_rate,nb_read_frames,sample_rate,color_range,color_space,color_primaries,color_transfer,start_time";
        let probed = String::from_utf8(run("ffprobe", &["-v", "error", "-count_frames", "-show_entries", entries, "-of", "csv=p=0", "part.mp4"])).unwrap();
        eprintln!("ffprobe: {probed}");
        let lines: Vec<&str> = probed.lines().collect();
        assert!(lines[0].starts_with("h264,High,480,320,yuv420p,tv,bt709,bt709,bt709,262144/4389,0.000000,120"), "{probed}");
        assert!(lines[1].starts_with("aac,LC,48000"), "{probed}");
        let quiet = ["-v", "error", "-nostdin"];
        let decoded = run("ffmpeg", &[&quiet[..], &["-i", "part.mp4", "-map", "0:v", "-f", "rawvideo", "-pix_fmt", "rgb24", "-fps_mode", "passthrough", "-"]].concat());
        assert_eq!(decoded.len(), pictures.len());
        let off = decoded.iter().zip(&pictures).map(|(a, b)| a.abs_diff(*b) as u64).sum::<u64>() as f64 / pictures.len() as f64;
        eprintln!("the pictures are {off:.3} off a byte");
        assert!(off < 3.0, "the MP4's pictures are {off} off a byte");
        // The sound: as long as was given (to within its last AAC frame), and at no lag from it.
        let heard: Vec<f32> = run("ffmpeg", &[&quiet[..], &["-i", "part.mp4", "-map", "0:a", "-f", "f32le", "-"]].concat())
            .chunks_exact(8)
            .map(|s| f32::from_le_bytes(s[..4].try_into().unwrap()))
            .collect();
        let sound: Vec<f32> = given.track(0).iter().map(|s| s[0]).collect();
        eprintln!("{} samples decoded of {}", heard.len(), sound.len());
        assert!(heard.len() >= sound.len() && heard.len() - sound.len() <= 1024 + 48, "{} samples decoded of {}", heard.len(), sound.len());
        let along = |lag: i64| -> f64 { (2000..sound.len() as i64 - 2000).map(|n| heard.get((n + lag) as usize).map_or(0.0, |&h| h as f64 * sound[n as usize] as f64)).sum() };
        let best = (-2200..=2200).max_by(|&a, &b| along(a).total_cmp(&along(b))).unwrap();
        assert_eq!(best, 0, "the decoded sound is {best} samples off the sound given");
        let _ = std::fs::remove_dir_all(dir);
    }
}
