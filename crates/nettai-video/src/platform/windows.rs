//! Windows' codecs ([`crate::codecs`]): Media Foundation's built-in H.264 encoder MFT for the picture and AAC-LC
//! encoder MFT for each sound track, a console shown: the system's own encoders on Windows 10 and 11 (x64 and
//! arm64), with nothing bundled.
//!
//! - **The picture** is the NV12 every platform is given, tightly packed (`MF_MT_DEFAULT_STRIDE` the width: every
//!   video's width is a multiple of 240), given at its time on the tick's time base (in 100 ns units). H.264 High
//!   (else Main, else Baseline) at the bit rate asked ([`codecs::bitrate`]), a key frame every
//!   [`codecs::KEY_FRAMES`] frames (`CODECAPI_AVEncMPVGOPSize`), no B-frames
//!   (`CODECAPI_AVEncMPVDefaultBPictureCount` 0), tagged BT.709 in the video range. It comes out as Annex B; any
//!   sequence header the output type says (`MF_MT_MPEG_SEQUENCE_HEADER`) is said up front, and each key frame carries
//!   its parameter sets too.
//! - **The sound** is AAC-LC at [`codecs::SOUND_BITRATE`] (24,000 bytes/s, one of Windows' supported AAC rates), of
//!   the 48 kHz samples as 16-bit PCM (what Media Foundation's AAC encoder takes), raw frames
//!   (`MF_MT_AAC_PAYLOAD_TYPE` 0). Its `AudioSpecificConfig` is taken from the output type's `MF_MT_USER_DATA` (past
//!   its 12-byte `HEAACWAVEINFO` header). Its priming (the samples the encoder puts before the sound) is measured as
//!   an export starts, a click through the AAC encoder and Windows' AAC decoder ([`aac_delay`]), as on Android and in
//!   a browser.

use crate::Error;
use crate::aac;
use crate::codecs::{self, Audio, Codecs, Coded, H264, Sound, Video};
use crate::resample::RATE;
use std::mem::ManuallyDrop;
use std::ptr;
use windows::Win32::Foundation::{RPC_E_CHANGED_MODE, S_FALSE, S_OK};
use windows::Win32::Media::MediaFoundation::{
    CODECAPI_AVEncMPVDefaultBPictureCount, CODECAPI_AVEncMPVGOPSize, ICodecAPI, IMFActivate, IMFMediaBuffer, IMFMediaType, IMFSample, IMFTransform,
    MF_E_NOTACCEPTING, MF_E_TRANSFORM_NEED_MORE_INPUT, MF_E_TRANSFORM_STREAM_CHANGE, MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION, MF_MT_AAC_PAYLOAD_TYPE,
    MF_MT_AUDIO_AVG_BYTES_PER_SECOND, MF_MT_AUDIO_BITS_PER_SAMPLE, MF_MT_AUDIO_BLOCK_ALIGNMENT, MF_MT_AUDIO_NUM_CHANNELS, MF_MT_AUDIO_SAMPLES_PER_SECOND,
    MF_MT_AVG_BITRATE, MF_MT_DEFAULT_STRIDE, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_INTERLACE_MODE, MF_MT_MAJOR_TYPE, MF_MT_MPEG_SEQUENCE_HEADER,
    MF_MT_MPEG2_PROFILE, MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE, MF_MT_TRANSFER_FUNCTION, MF_MT_USER_DATA, MF_MT_VIDEO_NOMINAL_RANGE,
    MF_MT_VIDEO_PRIMARIES, MF_MT_YUV_MATRIX, MF_VERSION, MFAudioFormat_AAC, MFAudioFormat_Float, MFAudioFormat_PCM, MFCreateMediaType,
    MFCreateMemoryBuffer, MFCreateSample, MFMediaType_Audio, MFMediaType_Video, MFNominalRange_16_235, MFSTARTUP_LITE, MFShutdown, MFStartup,
    MFT_CATEGORY_AUDIO_DECODER, MFT_CATEGORY_AUDIO_ENCODER, MFT_CATEGORY_VIDEO_ENCODER, MFT_ENUM_FLAG, MFT_ENUM_FLAG_LOCALMFT,
    MFT_ENUM_FLAG_SORTANDFILTER, MFT_ENUM_FLAG_SYNCMFT, MFT_MESSAGE_COMMAND_DRAIN, MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, MFT_MESSAGE_NOTIFY_END_OF_STREAM,
    MFT_MESSAGE_NOTIFY_START_OF_STREAM, MFT_OUTPUT_DATA_BUFFER, MFT_OUTPUT_STREAM_CAN_PROVIDE_SAMPLES, MFT_OUTPUT_STREAM_PROVIDES_SAMPLES,
    MFT_REGISTER_TYPE_INFO, MFTEnumEx, MFVideoFormat_H264, MFVideoFormat_NV12, MFVideoInterlace_Progressive, MFVideoPrimaries_BT709,
    MFVideoTransFunc_709, MFVideoTransferMatrix_BT709, eAVEncH264VProfile_Base, eAVEncH264VProfile_High, eAVEncH264VProfile_Main,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize};
use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_UI4};
use windows::core::{GUID, Interface};

/// 100-nanosecond ticks in a second (Media Foundation's time unit).
const HNS_PER_SEC: u128 = 10_000_000;

/// An AAC packet's samples.
const AAC_FRAME: u32 = 1024;

/// AAC-LC Level 2 profile-level indication, as Microsoft's AAC encoder expects (`MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION`).
const AAC_LC_LEVEL_2: u32 = 0x29;

/// Leading bytes of `MF_MT_USER_DATA` before `AudioSpecificConfig` (`HEAACWAVEINFO` after `WAVEFORMATEX`).
const HEAAC_HEADER: usize = 12;

/// Windows' codecs for a video of `video` and `audio`.
pub(crate) fn start(video: Video, audio: Option<Audio>) -> Result<Box<dyn Codecs>, Error> {
    let session = Session::start()?;
    let tracks = audio.map_or(0, |a| a.tracks);
    let priming = if tracks > 0 { aac_delay(codecs::SOUND_BITRATE)? } else { 0 };
    let (picture, sets) = start_picture(&video, codecs::bitrate(&video))?;
    let mut sound = Vec::with_capacity(tracks);
    let mut initial = Vec::new();
    if let Some(sets) = sets {
        initial.push(Coded::Sets(H264::AnnexB(sets)));
    }
    for track in 0..tracks {
        let (mft, config, _) = start_aac(codecs::SOUND_BITRATE)?;
        sound.push(mft);
        initial.push(Coded::Coding(track, Sound::Aac { config, priming }));
    }
    Ok(Box::new(Encoder {
        picture,
        sound,
        frame: video.frame_time.0,
        timescale: video.frame_time.1,
        given: 0,
        heard: vec![0; tracks],
        initial,
        pcm: Vec::new(),
        _session: session,
    }))
}

/// COM and Media Foundation started for this thread; shut down when dropped (after every MFT is released).
struct Session {
    com_owned: bool,
}

impl Session {
    fn start() -> Result<Session, Error> {
        // SAFETY: standard COM initialization for the calling thread.
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let com_owned = if hr == S_OK || hr == S_FALSE {
            true
        } else if hr == RPC_E_CHANGED_MODE {
            false
        } else {
            return Err(Error::Encoder(format!("COM initialization failed ({hr})")));
        };
        // SAFETY: pairs with `MFShutdown` in `Drop`.
        if let Err(e) = unsafe { MFStartup(MF_VERSION, MFSTARTUP_LITE) } {
            if com_owned {
                // SAFETY: matches `CoInitializeEx` above.
                unsafe { CoUninitialize() };
            }
            return Err(Error::Encoder(format!("Media Foundation startup failed: {e}")));
        }
        Ok(Session { com_owned })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: pairs with `MFStartup` and `CoInitializeEx` in `Session::start`.
        unsafe {
            let _ = MFShutdown();
            if self.com_owned {
                CoUninitialize();
            }
        }
    }
}

/// Pack two 32-bit values into a Media Foundation attribute pair (`MFSetAttributeSize` / `MFSetAttributeRatio`).
fn pack(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | lo as u64
}

/// A `VT_UI4` `VARIANT` of `value`, for `ICodecAPI::SetValue`.
fn variant_u32(value: u32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_UI4,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 { ulVal: value },
            }),
        },
    }
}

/// Samples as 16-bit little-endian PCM, appended to `out`.
fn pcm(samples: &[[f32; 2]], out: &mut Vec<u8>) {
    for s in samples.as_flattened() {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
}

/// Activate the first synchronous MFT matching `category` and optional input/output media types.
fn activate(category: GUID, input: Option<&MFT_REGISTER_TYPE_INFO>, output: Option<&MFT_REGISTER_TYPE_INFO>, what: &str) -> Result<IMFTransform, Error> {
    let flags = MFT_ENUM_FLAG(MFT_ENUM_FLAG_SYNCMFT.0 | MFT_ENUM_FLAG_LOCALMFT.0 | MFT_ENUM_FLAG_SORTANDFILTER.0);
    let mut activates: *mut Option<IMFActivate> = ptr::null_mut();
    let mut count = 0u32;
    // SAFETY: out pointers are valid locals; `activates` is freed below.
    unsafe { MFTEnumEx(category, flags, input.map(ptr::from_ref), output.map(ptr::from_ref), &mut activates, &mut count) }
        .map_err(|e| Error::Encoder(format!("Media Foundation: {what} enumeration failed: {e}")))?;
    let mut chosen = None;
    let mut last_err = None;
    if !activates.is_null() {
        // SAFETY: `MFTEnumEx` allocated `count` `IMFActivate` slots with `CoTaskMemAlloc`.
        unsafe {
            let list = std::slice::from_raw_parts_mut(activates, count as usize);
            for slot in list.iter_mut() {
                if chosen.is_none()
                    && let Some(act) = slot.as_ref()
                {
                    match act.ActivateObject::<IMFTransform>() {
                        Ok(mft) => chosen = Some(mft),
                        Err(e) => last_err = Some(e),
                    }
                }
                ptr::drop_in_place(slot);
            }
            CoTaskMemFree(Some(activates.cast()));
        }
    }
    chosen.ok_or_else(|| match last_err {
        Some(e) => Error::Encoder(format!("Media Foundation: no {what} could be activated: {e}")),
        None => Error::Encoder(format!("Media Foundation: no {what} found")),
    })
}

/// Read an attribute blob from `ty` if present.
fn get_blob(ty: &IMFMediaType, key: &GUID) -> Option<Vec<u8>> {
    // SAFETY: `ty` is a valid COM interface; `key` is a valid GUID.
    unsafe {
        let len = ty.GetBlobSize(key).ok()? as usize;
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len];
        ty.GetBlob(key, &mut buf, None).ok()?;
        Some(buf)
    }
}

/// Build an `IMFSample` holding a copy of `bytes`, stamped at `time_hns` for `dur_hns`.
fn make_sample(bytes: &[u8], time_hns: i64, dur_hns: i64, name: &str) -> Result<IMFSample, Error> {
    // SAFETY: every Media Foundation object is initialized and released on error.
    unsafe {
        let buffer: IMFMediaBuffer = MFCreateMemoryBuffer(bytes.len() as u32).map_err(|e| Error::Encoder(format!("{name}: input buffer: {e}")))?;
        if !bytes.is_empty() {
            let mut ptr = ptr::null_mut();
            buffer.Lock(&mut ptr, None, None).map_err(|e| Error::Encoder(format!("{name}: lock input buffer: {e}")))?;
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            buffer.Unlock().map_err(|e| Error::Encoder(format!("{name}: unlock input buffer: {e}")))?;
        }
        buffer.SetCurrentLength(bytes.len() as u32).map_err(|e| Error::Encoder(format!("{name}: input length: {e}")))?;
        let sample = MFCreateSample().map_err(|e| Error::Encoder(format!("{name}: input sample: {e}")))?;
        sample.AddBuffer(&buffer).map_err(|e| Error::Encoder(format!("{name}: add buffer: {e}")))?;
        sample.SetSampleTime(time_hns).map_err(|e| Error::Encoder(format!("{name}: sample time: {e}")))?;
        sample.SetSampleDuration(dur_hns).map_err(|e| Error::Encoder(format!("{name}: sample duration: {e}")))?;
        Ok(sample)
    }
}

/// Allocate an empty output `IMFSample` with a buffer of `size` bytes (for MFTs that don't allocate their own).
fn alloc_output_sample(size: u32, name: &str) -> Result<IMFSample, Error> {
    // SAFETY: standard Media Foundation sample + memory buffer allocation.
    unsafe {
        let buffer = MFCreateMemoryBuffer(size).map_err(|e| Error::Encoder(format!("{name}: output buffer: {e}")))?;
        let sample = MFCreateSample().map_err(|e| Error::Encoder(format!("{name}: output sample: {e}")))?;
        sample.AddBuffer(&buffer).map_err(|e| Error::Encoder(format!("{name}: add output buffer: {e}")))?;
        Ok(sample)
    }
}

/// Read all bytes out of `sample`.
fn read_sample(sample: &IMFSample, name: &str) -> Result<Vec<u8>, Error> {
    // SAFETY: locks the contiguous buffer, copies its current bytes, and unlocks it.
    unsafe {
        let buffer = sample.ConvertToContiguousBuffer().map_err(|e| Error::Encoder(format!("{name}: contiguous buffer: {e}")))?;
        let mut ptr = ptr::null_mut();
        let mut len = 0u32;
        buffer.Lock(&mut ptr, None, Some(&mut len)).map_err(|e| Error::Encoder(format!("{name}: lock output buffer: {e}")))?;
        let bytes = if len > 0 && !ptr.is_null() { std::slice::from_raw_parts(ptr, len as usize).to_vec() } else { Vec::new() };
        let _ = buffer.Unlock();
        Ok(bytes)
    }
}

/// A synchronous Media Foundation transform, fed samples and drained of what it makes.
struct Mft {
    transform: IMFTransform,
    provides_samples: bool,
    output_size: u32,
    made: Vec<Vec<u8>>,
    name: &'static str,
}

impl Mft {
    fn new(transform: IMFTransform, name: &'static str) -> Result<Mft, Error> {
        // SAFETY: `transform` has both input and output types configured.
        unsafe {
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0).map_err(|e| Error::Encoder(format!("{name}: begin streaming: {e}")))?;
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0).map_err(|e| Error::Encoder(format!("{name}: start of stream: {e}")))?;
        }
        let mut mft = Mft { transform, provides_samples: false, output_size: 0, made: Vec::new(), name };
        mft.refresh_output_info()?;
        Ok(mft)
    }

    fn refresh_output_info(&mut self) -> Result<(), Error> {
        // SAFETY: stream 0 is the transform's single output stream.
        let info = unsafe { self.transform.GetOutputStreamInfo(0) }.map_err(|e| Error::Encoder(format!("{}: output stream info: {e}", self.name)))?;
        let flags = (MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 | MFT_OUTPUT_STREAM_CAN_PROVIDE_SAMPLES.0) as u32;
        self.provides_samples = (info.dwFlags & flags) != 0;
        self.output_size = info.cbSize;
        Ok(())
    }

    /// Give `bytes` at `time_hns` lasting `dur_hns`, draining outputs whenever ready or when input is full.
    fn give(&mut self, bytes: &[u8], time_hns: i64, dur_hns: i64, on_change: &mut dyn FnMut(&IMFTransform) -> Result<(), Error>) -> Result<(), Error> {
        let sample = make_sample(bytes, time_hns, dur_hns, self.name)?;
        for _ in 0..1024 {
            // SAFETY: `sample` is a valid `IMFSample` for stream 0.
            match unsafe { self.transform.ProcessInput(0, &sample, 0) } {
                Ok(()) => return self.drain_ready(on_change),
                Err(e) if e.code() == MF_E_NOTACCEPTING => {
                    let before = self.made.len();
                    self.drain_ready(on_change)?;
                    if self.made.len() == before {
                        return Err(Error::Encoder(format!("{}: refused input and produced no output", self.name)));
                    }
                }
                Err(e) => return Err(Error::Encoder(format!("{}: ProcessInput: {e}", self.name))),
            }
        }
        Err(Error::Encoder(format!("{}: stalled refusing input", self.name)))
    }

    /// Pull every output sample currently ready.
    fn drain_ready(&mut self, on_change: &mut dyn FnMut(&IMFTransform) -> Result<(), Error>) -> Result<(), Error> {
        loop {
            let allocated = if self.provides_samples { None } else { Some(alloc_output_sample(self.output_size.max(1), self.name)?) };
            let mut out = [MFT_OUTPUT_DATA_BUFFER { dwStreamID: 0, pSample: ManuallyDrop::new(allocated), dwStatus: 0, pEvents: ManuallyDrop::new(None) }];
            let mut status = 0u32;
            // SAFETY: `out` is a 1-element slice for stream 0; both `ManuallyDrop` slots are taken right after.
            let res = unsafe { self.transform.ProcessOutput(0, &mut out, &mut status) };
            // SAFETY: we own both `ManuallyDrop` fields and take each exactly once before `out` goes out of scope.
            let sample = unsafe { ManuallyDrop::take(&mut out[0].pSample) };
            let _events = unsafe { ManuallyDrop::take(&mut out[0].pEvents) };
            match res {
                Ok(()) => {
                    if let Some(sample) = sample {
                        let bytes = read_sample(&sample, self.name)?;
                        if !bytes.is_empty() {
                            self.made.push(bytes);
                        }
                    }
                }
                Err(e) if e.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => return Ok(()),
                Err(e) if e.code() == MF_E_TRANSFORM_STREAM_CHANGE => {
                    on_change(&self.transform)?;
                    self.refresh_output_info()?;
                }
                Err(e) => return Err(Error::Encoder(format!("{}: ProcessOutput: {e}", self.name))),
            }
        }
    }

    /// End of stream: drain every remaining buffered sample out of the transform.
    fn end(&mut self, on_change: &mut dyn FnMut(&IMFTransform) -> Result<(), Error>) -> Result<(), Error> {
        // SAFETY: standard end-of-stream and drain sequence for a synchronous MFT.
        unsafe {
            let _ = self.transform.ProcessMessage(MFT_MESSAGE_NOTIFY_END_OF_STREAM, 0);
            self.transform.ProcessMessage(MFT_MESSAGE_COMMAND_DRAIN, 0).map_err(|e| Error::Encoder(format!("{}: drain: {e}", self.name)))?;
        }
        self.drain_ready(on_change)
    }
}

/// Default stream-change handler for encoders: accept the transform's first offered output type.
fn accept_first_output(transform: &IMFTransform) -> Result<(), Error> {
    // SAFETY: stream 0 output type negotiation after `MF_E_TRANSFORM_STREAM_CHANGE`.
    unsafe {
        let ty = transform.GetOutputAvailableType(0, 0).map_err(|e| Error::Encoder(format!("Media Foundation: next output type: {e}")))?;
        transform.SetOutputType(0, &ty, 0).map_err(|e| Error::Encoder(format!("Media Foundation: set changed output type: {e}")))
    }
}

/// Start Windows' H.264 encoder MFT for `video` at `bitrate`; returns the MFT and any out-of-band sequence header.
fn start_picture(video: &Video, bitrate: u32) -> Result<(Mft, Option<Vec<u8>>), Error> {
    let output_info = MFT_REGISTER_TYPE_INFO { guidMajorType: MFMediaType_Video, guidSubtype: MFVideoFormat_H264 };
    let transform = activate(MFT_CATEGORY_VIDEO_ENCODER, None, Some(&output_info), "H.264 encoder")?;
    let (frame, timescale) = video.frame_time;
    // Output type first (required by Microsoft's H.264 MFT): High, else Main, else Baseline.
    let mut configured = None;
    for profile in [eAVEncH264VProfile_High, eAVEncH264VProfile_Main, eAVEncH264VProfile_Base] {
        // SAFETY: configuring a newly created `IMFMediaType` with standard Media Foundation attributes.
        let res = unsafe {
            (|| -> windows::core::Result<()> {
                let ty = MFCreateMediaType()?;
                ty.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
                ty.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
                ty.SetUINT32(&MF_MT_AVG_BITRATE, bitrate)?;
                ty.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
                ty.SetUINT64(&MF_MT_FRAME_SIZE, pack(video.width, video.height))?;
                ty.SetUINT64(&MF_MT_FRAME_RATE, pack(timescale, frame))?;
                ty.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))?;
                ty.SetUINT32(&MF_MT_MPEG2_PROFILE, profile.0 as u32)?;
                ty.SetUINT32(&MF_MT_VIDEO_PRIMARIES, MFVideoPrimaries_BT709.0 as u32)?;
                ty.SetUINT32(&MF_MT_TRANSFER_FUNCTION, MFVideoTransFunc_709.0 as u32)?;
                ty.SetUINT32(&MF_MT_YUV_MATRIX, MFVideoTransferMatrix_BT709.0 as u32)?;
                ty.SetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_16_235.0 as u32)?;
                transform.SetOutputType(0, &ty, 0)
            })()
        };
        if res.is_ok() {
            configured = Some(profile);
            break;
        }
    }
    if configured.is_none() {
        return Err(Error::Encoder(format!("Media Foundation H.264 encoder refused {}x{} at {bitrate} b/s", video.width, video.height)));
    }
    // Input type: tightly packed NV12, BT.709 in the video range.
    // SAFETY: configuring the matching NV12 input media type.
    unsafe {
        let ty = MFCreateMediaType().map_err(|e| Error::Encoder(format!("H.264 input type: {e}")))?;
        let set = || -> windows::core::Result<()> {
            ty.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            ty.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
            ty.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            ty.SetUINT64(&MF_MT_FRAME_SIZE, pack(video.width, video.height))?;
            ty.SetUINT64(&MF_MT_FRAME_RATE, pack(timescale, frame))?;
            ty.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))?;
            ty.SetUINT32(&MF_MT_DEFAULT_STRIDE, video.width)?;
            ty.SetUINT32(&MF_MT_VIDEO_PRIMARIES, MFVideoPrimaries_BT709.0 as u32)?;
            ty.SetUINT32(&MF_MT_TRANSFER_FUNCTION, MFVideoTransFunc_709.0 as u32)?;
            ty.SetUINT32(&MF_MT_YUV_MATRIX, MFVideoTransferMatrix_BT709.0 as u32)?;
            ty.SetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_16_235.0 as u32)?;
            transform.SetInputType(0, &ty, 0)
        };
        set().map_err(|e| Error::Encoder(format!("Media Foundation H.264 input type: {e}")))?;
    }
    // No B-frames (presentation order == decode order), key frame every `KEY_FRAMES`.
    if let Ok(api) = transform.cast::<ICodecAPI>() {
        let zero = variant_u32(0);
        let gop = variant_u32(codecs::KEY_FRAMES);
        // SAFETY: both GUIDs take a `VT_UI4` `VARIANT`.
        unsafe {
            let _ = api.SetValue(&CODECAPI_AVEncMPVDefaultBPictureCount, &zero);
            let _ = api.SetValue(&CODECAPI_AVEncMPVGOPSize, &gop);
        }
    }
    // SAFETY: querying the negotiated output type for any out-of-band SPS/PPS sequence header.
    let sets = unsafe { transform.GetOutputCurrentType(0).ok().and_then(|ty| get_blob(&ty, &MF_MT_MPEG_SEQUENCE_HEADER)) };
    Ok((Mft::new(transform, "Media Foundation H.264")?, sets))
}

/// Start Windows' AAC-LC encoder MFT at `bitrate` (48 kHz stereo); returns the MFT, its `AudioSpecificConfig`, and
/// the raw `MF_MT_USER_DATA` blob (for the priming decoder).
fn start_aac(bitrate: u32) -> Result<(Mft, Vec<u8>, Vec<u8>), Error> {
    let output_info = MFT_REGISTER_TYPE_INFO { guidMajorType: MFMediaType_Audio, guidSubtype: MFAudioFormat_AAC };
    let transform = activate(MFT_CATEGORY_AUDIO_ENCODER, None, Some(&output_info), "AAC encoder")?;
    // Input type first (required by Microsoft's AAC MFT): 48 kHz stereo 16-bit PCM.
    // SAFETY: standard Media Foundation audio type attributes.
    unsafe {
        let input = MFCreateMediaType().map_err(|e| Error::Encoder(format!("AAC input type: {e}")))?;
        let set = || -> windows::core::Result<()> {
            input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
            input.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM)?;
            input.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)?;
            input.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, RATE)?;
            input.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2)?;
            input.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 4)?;
            input.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, RATE * 4)?;
            transform.SetInputType(0, &input, 0)
        };
        set().map_err(|e| Error::Encoder(format!("Media Foundation AAC input type: {e}")))?;
        let output = MFCreateMediaType().map_err(|e| Error::Encoder(format!("AAC output type: {e}")))?;
        let set_out = || -> windows::core::Result<()> {
            output.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
            output.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_AAC)?;
            output.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)?;
            output.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, RATE)?;
            output.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2)?;
            output.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, bitrate / 8)?;
            output.SetUINT32(&MF_MT_AAC_PAYLOAD_TYPE, 0)?;
            output.SetUINT32(&MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION, AAC_LC_LEVEL_2)?;
            transform.SetOutputType(0, &output, 0)
        };
        set_out().map_err(|e| Error::Encoder(format!("Media Foundation AAC output type: {e}")))?;
    }
    // SAFETY: reading `MF_MT_USER_DATA` populated by the AAC encoder after `SetOutputType`.
    let user_data = unsafe { transform.GetOutputCurrentType(0).ok().and_then(|ty| get_blob(&ty, &MF_MT_USER_DATA)).unwrap_or_default() };
    let config = if user_data.len() > HEAAC_HEADER { user_data[HEAAC_HEADER..].to_vec() } else { aac::Config::lc(RATE, 2).bytes() };
    Ok((Mft::new(transform, "Media Foundation AAC")?, config, user_data))
}

/// How many samples Windows' AAC encoder at `bitrate` puts before the sound: a click, encoded and decoded again by
/// Windows' own AAC decoder MFT, comes out that much later (as on Android and in a browser).
fn aac_delay(bitrate: u32) -> Result<u32, Error> {
    const AT: usize = 3000;
    const LENGTH: usize = 16384;
    let mut click = vec![[0.0f32; 2]; LENGTH];
    click[AT] = [0.5, 0.5];
    let mut bytes = Vec::new();
    pcm(&click, &mut bytes);
    let (mut encoder, config, user_data) = start_aac(bitrate)?;
    let dur = (LENGTH as u128 * HNS_PER_SEC / RATE as u128) as i64;
    encoder.give(&bytes, 0, dur, &mut accept_first_output)?;
    encoder.end(&mut accept_first_output)?;

    // Decode the raw AAC packets through Windows' built-in AAC decoder MFT.
    let input_info = MFT_REGISTER_TYPE_INFO { guidMajorType: MFMediaType_Audio, guidSubtype: MFAudioFormat_AAC };
    let dec_transform = activate(MFT_CATEGORY_AUDIO_DECODER, Some(&input_info), None, "AAC decoder")?;
    let dec_user_data = if user_data.len() > HEAAC_HEADER {
        user_data
    } else {
        let mut blob = vec![0u8; HEAAC_HEADER];
        blob[2] = AAC_LC_LEVEL_2 as u8;
        blob.extend_from_slice(&config);
        blob
    };
    // SAFETY: configuring the AAC decoder's raw AAC input type.
    unsafe {
        let ty = MFCreateMediaType().map_err(|e| Error::Encoder(format!("AAC decoder input type: {e}")))?;
        let set = || -> windows::core::Result<()> {
            ty.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
            ty.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_AAC)?;
            ty.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, RATE)?;
            ty.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2)?;
            ty.SetUINT32(&MF_MT_AAC_PAYLOAD_TYPE, 0)?;
            ty.SetBlob(&MF_MT_USER_DATA, &dec_user_data)?;
            dec_transform.SetInputType(0, &ty, 0)
        };
        set().map_err(|e| Error::Encoder(format!("Media Foundation AAC decoder input: {e}")))?;
    }
    let mut floats = pick_pcm_output(&dec_transform)?;
    let mut decoder = Mft::new(dec_transform, "Media Foundation AAC decoder")?;
    let packet_dur = (AAC_FRAME as u128 * HNS_PER_SEC / RATE as u128) as i64;
    let mut on_change = |t: &IMFTransform| -> Result<(), Error> {
        floats = pick_pcm_output(t)?;
        Ok(())
    };
    for (n, packet) in encoder.made.iter().enumerate() {
        let time = (n as u128 * AAC_FRAME as u128 * HNS_PER_SEC / RATE as u128) as i64;
        decoder.give(packet, time, packet_dur, &mut on_change)?;
    }
    decoder.end(&mut on_change)?;

    let left: Vec<f32> = decoder
        .made
        .iter()
        .flat_map(|chunk| {
            if floats {
                chunk.chunks_exact(8).map(|s| f32::from_le_bytes(s[..4].try_into().unwrap())).collect::<Vec<_>>()
            } else {
                chunk.chunks_exact(4).map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0).collect()
            }
        })
        .collect();
    let (place, loudest) = left.iter().enumerate().fold((0, 0.0f32), |(p, l), (i, v)| if v.abs() > l { (i, v.abs()) } else { (p, l) });
    if loudest < 0.1 || place < AT {
        return Err(Error::Encoder(format!("Media Foundation AAC delay couldn't be told (its click came out at {place}, {loudest} loud)")));
    }
    Ok((place - AT) as u32)
}

/// Pick a 32-bit float or 16-bit integer PCM output type on the AAC decoder; returns true if float.
fn pick_pcm_output(transform: &IMFTransform) -> Result<bool, Error> {
    // SAFETY: enumerating the decoder's available output types on stream 0.
    unsafe {
        for idx in 0..32 {
            let Ok(ty) = transform.GetOutputAvailableType(0, idx) else { break };
            let Ok(sub) = ty.GetGUID(&MF_MT_SUBTYPE) else { continue };
            let bits = ty.GetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE).unwrap_or(0);
            if (sub == MFAudioFormat_Float && bits == 32) || (sub == MFAudioFormat_PCM && bits == 16) {
                transform.SetOutputType(0, &ty, 0).map_err(|e| Error::Encoder(format!("Media Foundation AAC decoder output: {e}")))?;
                return Ok(sub == MFAudioFormat_Float);
            }
        }
    }
    Err(Error::Encoder("Media Foundation AAC decoder offered neither float nor 16-bit PCM".into()))
}

/// Windows coding a video. (MFT fields come before `_session` so they drop before `MFShutdown`.)
struct Encoder {
    picture: Mft,
    sound: Vec<Mft>,
    frame: u32,
    timescale: u32,
    given: u64,
    heard: Vec<u64>,
    /// Initial sets and sound track codings, emitted with the first `coded()` call.
    initial: Vec<Coded>,
    pcm: Vec<u8>,
    _session: Session,
}

impl Encoder {
    fn take(&mut self) -> Vec<Coded> {
        let mut out = std::mem::take(&mut self.initial);
        out.extend(self.picture.made.drain(..).map(|b| Coded::Frame(H264::AnnexB(b))));
        for (t, mft) in self.sound.iter_mut().enumerate() {
            out.extend(mft.made.drain(..).map(|b| Coded::Sound(t, b)));
        }
        out
    }
}

impl Codecs for Encoder {
    fn frame(&mut self, nv12: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        let time = (self.given as u128 * self.frame as u128 * HNS_PER_SEC / self.timescale as u128) as i64;
        let dur = (self.frame as u128 * HNS_PER_SEC / self.timescale as u128) as i64;
        self.picture.give(&nv12, time, dur, &mut accept_first_output)?;
        self.given += 1;
        for (t, (mft, samples)) in self.sound.iter_mut().zip(sound).enumerate() {
            if samples.is_empty() {
                continue;
            }
            self.pcm.clear();
            pcm(samples, &mut self.pcm);
            let time = (self.heard[t] as u128 * HNS_PER_SEC / RATE as u128) as i64;
            let dur = (samples.len() as u128 * HNS_PER_SEC / RATE as u128) as i64;
            mft.give(&self.pcm, time, dur, &mut accept_first_output)?;
            self.heard[t] += samples.len() as u64;
        }
        Ok(nv12)
    }

    fn coded(&mut self) -> Result<Vec<Coded>, Error> {
        Ok(self.take())
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<Coded>, Error> {
        self.picture.end(&mut accept_first_output)?;
        for mft in &mut self.sound {
            mft.end(&mut accept_first_output)?;
        }
        Ok(self.take())
    }

    fn cancel(self: Box<Self>) {
        // (Dropped: MFTs are released, then Media Foundation and COM are shut down.)
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::{options, viewer};
    use crate::{Part, Sides, write_mp4};
    use mp4_atom::{Codec, Decode, Ftyp, Moov, StszSamples};
    use std::ops::ControlFlow;

    /// Windows' Media Foundation encoders make the MP4: its head, then its samples; H.264 of every frame at the
    /// size asked, and AAC of the sound shown from past its measured priming.
    #[test]
    fn mediafoundation_makes_the_mp4() {
        let content = nettai_match::testing::exe6_content();
        let (replay, _) = nettai_frontend::testing::recorded(&content, "exe6", 7);
        let part = Part::Ticks(200..320);
        let made = write_mp4(&mut viewer(&content, &replay, 0), &options(2, Sides::One, &part), Vec::new(), &mut |_| ControlFlow::Continue(())).unwrap();
        let file = [made.head, made.body].concat();
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
        assert_eq!(samples(0), 120);
        assert!(picture.edts.is_none(), "no reordering: the picture is shown from its first sample");
        let sound = &moov.trak[1];
        assert!(matches!(sound.mdia.minf.stbl.stsd.codecs[0], Codec::Mp4a(_)));
        let from = sound.edts.as_ref().and_then(|e| e.elst.as_ref()).map(|l| l.entries[0].media_time);
        assert!(matches!(from, Some(Some(p)) if p > 0), "unexpected AAC priming: {from:?}");
        assert!((94..=102).contains(&samples(1)), "{} AAC frames", samples(1));
    }
}
