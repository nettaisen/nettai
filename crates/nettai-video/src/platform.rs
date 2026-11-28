//! The platform's codecs ([`crate::codecs::Codecs`]), a module each, chosen here and nowhere else:
//!
//! - [`apple`]: macOS and iOS — VideoToolbox's H.264 and AudioToolbox's AAC;
//! - [`windows`]: Windows — Media Foundation's H.264 and AAC MFTs;
//! - [`android`]: Android — MediaCodec's H.264 and AAC;
//! - [`web`]: the browser's WebCodecs (its H.264; its AAC, else Opus);
//! - [`ffmpeg`]: Linux — system `ffmpeg`, run as children, a track each (x264's H.264, ffmpeg's AAC).
//!
//! Each is given the same NV12 frames and 48 kHz samples, and gives back what its codecs made as they made it
//! ([`crate::codecs::Coded`]); the rest of a video is the same for all.

#[cfg(target_os = "android")]
pub(crate) mod android;
#[cfg(apple)]
#[allow(unsafe_code)]
pub(crate) mod apple;
#[cfg(ffmpeg)]
pub(crate) mod ffmpeg;
#[cfg(web)]
pub(crate) mod web;
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
pub(crate) mod windows;

#[cfg(target_os = "android")]
pub(crate) use android::start;
#[cfg(apple)]
pub(crate) use apple::start;
#[cfg(ffmpeg)]
pub(crate) use ffmpeg::start;
#[cfg(web)]
pub(crate) use web::start;
#[cfg(target_os = "windows")]
pub(crate) use windows::start;
