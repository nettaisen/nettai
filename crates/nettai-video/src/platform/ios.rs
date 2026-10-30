//! iOS's codecs ([`crate::codecs`]): VideoToolbox's H.264 for the picture and AudioToolbox's AAC for each sound
//! track, a console shown: the system's own encoders (the picture's in hardware on every iPhone). macOS has the same
//! frameworks, and the tests run this there.
//!
//! - **The picture** is the NV12 every platform is given, each frame written into a pixel buffer of the session's
//!   pool and given at its time on the tick's time base. H.264 High at the level its size needs, at the bit rate
//!   asked ([`codecs::bitrate`]), a key frame at least every [`codecs::KEY_FRAMES`] frames, no frame reordering (no
//!   B-frames), tagged BT.709 throughout. The session answers on a thread of its own ([`encoded`]) with each frame's
//!   access unit, its NAL units after their lengths, and its format description, whose parameter sets are said
//!   once, before the first frame. What it made is taken from there as the frames go in ([`Codecs::coded`]), and a
//!   feeder far ahead waits for it.
//! - **The sound** is AAC-LC at the bit rate asked, of the 48 kHz samples as they come, a converter a track. Its
//!   config is AAC-LC's at 48 kHz in stereo, what the converter was asked for; its priming, the samples it puts
//!   before the sound, is what AudioToolbox says (`kAudioConverterPrimeInfo`: 2112).

use crate::Error;
use crate::aac;
use crate::codecs::{self, Audio, Codecs, Coded, H264, Sound, Video};
use crate::resample::RATE;
use objc2_audio_toolbox::{
    AudioConverterDispose, AudioConverterFillComplexBuffer, AudioConverterGetProperty, AudioConverterNew, AudioConverterPrimeInfo, AudioConverterRef,
    AudioConverterSetProperty, kAudioConverterEncodeBitRate, kAudioConverterPrimeInfo, kAudioConverterPropertyMaximumOutputPacketSize,
};
use objc2_core_audio_types::{
    AudioBuffer, AudioBufferList, AudioStreamBasicDescription, AudioStreamPacketDescription, kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked,
    kAudioFormatLinearPCM, kAudioFormatMPEG4AAC,
};
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_media::{CMFormatDescription, CMSampleBuffer, CMTime, CMVideoFormatDescriptionGetH264ParameterSetAtIndex, kCMTimeInvalid};
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
const H264_CODE: u32 = u32::from_be_bytes(*b"avc1");

/// The most frames given to the picture's encoder before what it made of them is waited for.
const AHEAD: u64 = 6;

/// AAC-LC's object type, as an AudioStreamBasicDescription's flags say it (`kMPEG4Object_AAC_LC`).
const AAC_LC: u32 = 2;

/// An AAC packet's samples.
const AAC_FRAME: u32 = 1024;

/// What the sound's input says when it has nothing more to give for now (no error AudioToolbox has: the
/// converter stops and says it back).
const WAIT_FOR_MORE: i32 = i32::from_be_bytes(*b"more");

/// iOS's codecs for a video of `video` and `audio`.
pub(crate) fn start(video: Video, audio: Option<Audio>) -> Result<Box<dyn Codecs>, Error> {
    let picture = Picture::start(video, codecs::bitrate(&video))?;
    let sound = (0..audio.map_or(0, |a| a.tracks)).map(|t| Aac::start(t, codecs::SOUND_BITRATE)).collect::<Result<Vec<_>, _>>()?;
    let config = aac::Config::lc(RATE, 2).bytes();
    let codings = sound.iter().map(|a| Coded::Coding(a.track, Sound::Aac { config: config.clone(), priming: a.leading })).collect();
    Ok(Box::new(Encoder { picture, sound, codings }))
}

/// iOS coding a video.
struct Encoder {
    picture: Picture,
    sound: Vec<Aac>,
    /// The sound tracks' codings, said with what is first taken.
    codings: Vec<Coded>,
}

impl Encoder {
    /// What was made since; `wait`: everything given is made first.
    fn take(&mut self, wait: bool) -> Result<Vec<Coded>, Error> {
        if wait {
            self.picture.complete()?;
        }
        let mut made = std::mem::take(&mut self.codings);
        made.extend(self.picture.made()?);
        for aac in &mut self.sound {
            made.extend(aac.frames.drain(..).map(|f| Coded::Sound(aac.track, f)));
        }
        Ok(made)
    }
}

impl Codecs for Encoder {
    fn frame(&mut self, nv12: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        self.picture.frame(&nv12)?;
        for (aac, samples) in self.sound.iter_mut().zip(sound) {
            aac.push(samples)?;
        }
        Ok(nv12)
    }

    fn coded(&mut self) -> Result<Vec<Coded>, Error> {
        self.take(false)
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<Coded>, Error> {
        for aac in &mut self.sound {
            aac.finish()?;
        }
        self.take(true)
    }

    fn cancel(self: Box<Self>) {
        // (Dropped: the session is ended without waiting, the converters let go.)
    }
}

/// What the picture's encoder made, told from its thread ([`encoded`]).
#[derive(Default)]
struct Made {
    coded: Vec<Coded>,
    /// Its parameter sets are said.
    sets: bool,
    /// How many frames it answered for.
    answered: u64,
    failed: Option<String>,
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
    // (A frame dropped: the frames' times are their count, so none may be. This session drops none, at no
    // real-time limit.)
    // SAFETY: a sample the session made is valid for this call.
    let Some(sample) = (unsafe { sample.as_ref() }) else {
        made.failed.get_or_insert_with(|| "VideoToolbox dropped a frame".into());
        return;
    };
    // SAFETY: the sample is valid for this call.
    match unsafe { access_unit(sample, !made.sets) } {
        Ok((unit, sets)) => {
            if let Some(sets) = sets {
                made.sets = true;
                made.coded.push(Coded::Sets(H264::Lengths(sets)));
            }
            made.coded.push(Coded::Frame(H264::Lengths(unit)));
        }
        Err(why) => {
            made.failed.get_or_insert(why);
        }
    }
}

/// The access unit of a sample VideoToolbox made (its NAL units after their lengths), and with `sets`, the
/// parameter sets of its format description, the same way.
///
/// # Safety
/// `sample` is valid.
unsafe fn access_unit(sample: &CMSampleBuffer, sets: bool) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
    // SAFETY: the sample is valid; what it gives is retained.
    let (data, description) = unsafe { (sample.data_buffer(), sample.format_description()) };
    let data = data.ok_or("a sample with no data")?;
    // SAFETY: the block buffer is valid; `bytes` is as long as it says it is.
    let bytes = unsafe {
        let mut bytes = vec![0u8; data.data_length()];
        if !bytes.is_empty() && data.copy_data_bytes(0, bytes.len(), NonNull::new_unchecked(bytes.as_mut_ptr().cast())) != 0 {
            return Err("its sample's bytes couldn't be read".into());
        }
        bytes
    };
    if !sets {
        return Ok((bytes, None));
    }
    let description = description.ok_or("its first sample has no format description")?;
    // SAFETY: the description is valid and retained while its sets are read.
    Ok((bytes, Some(unsafe { parameter_sets(&description) }?)))
}

/// The parameter sets of an H.264 format description, each after its length in four bytes.
///
/// # Safety
/// `description` is valid.
unsafe fn parameter_sets(description: &CMFormatDescription) -> Result<Vec<u8>, String> {
    let (mut count, mut length) = (0usize, 0i32);
    // SAFETY: the description is valid; the counts are written to locals.
    let status = unsafe { CMVideoFormatDescriptionGetH264ParameterSetAtIndex(description, 0, ptr::null_mut(), ptr::null_mut(), &mut count, &mut length) };
    if status != 0 || length != 4 {
        return Err(format!("its parameter sets couldn't be read (status {status}, lengths of {length} bytes)"));
    }
    let mut out = Vec::new();
    for index in 0..count {
        let (mut at, mut size) = (ptr::null::<u8>(), 0usize);
        // SAFETY: as above; the set's bytes are the description's, read while it is held.
        let set = unsafe {
            let status = CMVideoFormatDescriptionGetH264ParameterSetAtIndex(description, index, &mut at, &mut size, ptr::null_mut(), ptr::null_mut());
            if status != 0 || at.is_null() {
                return Err(format!("its parameter set {index} couldn't be read (status {status})"));
            }
            std::slice::from_raw_parts(at, size)
        };
        out.extend_from_slice(&(set.len() as u32).to_be_bytes());
        out.extend_from_slice(set);
    }
    Ok(out)
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
        let made = Arc::new(Mutex::new(Made::default()));
        let held = Arc::into_raw(made.clone());
        let mut session: *mut VTCompressionSession = ptr::null_mut();
        // SAFETY: every pointer is valid for the call; the callback's reference is `held`, which the session keeps till
        // it is invalidated.
        let status = unsafe {
            VTCompressionSession::create(None, video.width as i32, video.height as i32, H264_CODE, None, Some(attributes.as_opaque()), None, Some(encoded), held as *mut c_void, NonNull::from(&mut session))
        };
        let Some(session) = NonNull::new(session).filter(|_| status == 0) else {
            // SAFETY: no session was made: the reference is this function's to let go.
            drop(unsafe { Arc::from_raw(held) });
            return Err(failed("no H.264 encoder", status));
        };
        // SAFETY: the session was made for us (+1), and holds `held` from here.
        let session = Session { session: unsafe { CFRetained::from_raw(session) }, made, held };
        let (rate, keys) = (number(bitrate as i32), number(codecs::KEY_FRAMES as i32));
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

    /// What was made since; or why it failed.
    fn made(&mut self) -> Result<Vec<Coded>, Error> {
        let mut made = self.session.made.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(why) = made.failed.take() {
            return Err(Error::Encoder(why));
        }
        Ok(std::mem::take(&mut made.coded))
    }
}

/// AudioToolbox's AAC encoder for a sound track.
struct Aac {
    converter: AudioConverterRef,
    /// The sound track it codes (from 0).
    track: usize,
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
    /// The frames made.
    frames: Vec<Vec<u8>>,
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
        let mut aac = Aac { converter, track, leading: 0, most: 0, waiting: Vec::new(), lent: Vec::new(), ending: false, frames: Vec::new() };
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
        self.frames.push(out);
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
    use crate::{Error, Mp4, Options, Part, Sides, play};
    use mp4_atom::{Codec, Decode, Ftyp, Moov, StszSamples};
    use nettai_frontend::viewer::Viewer;
    use std::ops::ControlFlow;

    /// `options`' MP4 of the replay `viewer` plays, made by iOS's encoders (here macOS's, the same frameworks).
    fn mp4(viewer: &mut Viewer, options: &Options) -> Result<Vec<u8>, Error> {
        let made = play(viewer, options, &mut |_| ControlFlow::Continue(()), |video, audio, info| Ok(Mp4::with(start(video, audio)?, video, audio, info, Vec::new())))?.finish()?;
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
        let file = mp4(&mut viewer(&content, &replay, 0), &options(2, Sides::One, &PART)).unwrap();
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
    /// does (`desktop::tests::ffmpeg_makes_the_mp4`): every frame, close to the NV12 given; the sound as long as it
    /// was given and at no lag from it (the priming left out); the colors tagged; the picture shown from 0.
    #[test]
    #[ignore]
    fn videotoolbox_mp4_plays_as_given() {
        use std::process::Command;
        let content = nettai_match::testing::exe6_content();
        let (replay, _) = nettai_frontend::testing::recorded(&content, "exe6", 7);
        let given = given(&mut viewer(&content, &replay, 0), &options(2, Sides::One, &PART), &mut |_| ControlFlow::Continue(())).unwrap();
        let pictures: Vec<u8> = given.frames.iter().flat_map(|(nv12, _)| nv12.iter().copied()).collect();
        let dir = std::env::temp_dir().join(format!("nettai-video-videotoolbox-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("part.mp4"), mp4(&mut viewer(&content, &replay, 0), &options(2, Sides::One, &PART)).unwrap()).unwrap();
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
        let decoded = run("ffmpeg", &[&quiet[..], &["-i", "part.mp4", "-map", "0:v", "-f", "rawvideo", "-pix_fmt", "nv12", "-fps_mode", "passthrough", "-"]].concat());
        assert_eq!(decoded.len(), pictures.len());
        let off = decoded.iter().zip(&pictures).map(|(a, b)| a.abs_diff(*b) as u64).sum::<u64>() as f64 / pictures.len() as f64;
        eprintln!("the pictures are {off:.3} off a byte");
        assert!(off < 3.0, "the MP4's pictures are {off} off a byte");
        // The sound: as long as was given (its edit's length is the movie's milliseconds: to within one), and at no
        // lag from it.
        let heard: Vec<f32> = run("ffmpeg", &[&quiet[..], &["-i", "part.mp4", "-map", "0:a", "-f", "f32le", "-"]].concat())
            .chunks_exact(8)
            .map(|s| f32::from_le_bytes(s[..4].try_into().unwrap()))
            .collect();
        let sound: Vec<f32> = given.track(0).iter().map(|s| s[0]).collect();
        eprintln!("{} samples decoded of {}", heard.len(), sound.len());
        assert!(heard.len().abs_diff(sound.len()) <= 48, "{} samples decoded of {}", heard.len(), sound.len());
        let along = |lag: i64| -> f64 { (2000..sound.len() as i64 - 2000).map(|n| heard.get((n + lag) as usize).map_or(0.0, |&h| h as f64 * sound[n as usize] as f64)).sum() };
        let best = (-2200..=2200).max_by(|&a, &b| along(a).total_cmp(&along(b))).unwrap();
        assert_eq!(best, 0, "the decoded sound is {best} samples off the sound given");
        let _ = std::fs::remove_dir_all(dir);
    }
}
