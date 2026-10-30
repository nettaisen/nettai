//! The platform's codecs ([`crate::codecs::Codecs`]), a module each, chosen here and nowhere else:
//!
//! - [`desktop`]: `ffmpeg`, run as children, a track each (x264's H.264, ffmpeg's AAC);
//! - [`web`]: the browser's WebCodecs (its H.264; its AAC, else Opus);
//! - [`ios`]: VideoToolbox's H.264 and AudioToolbox's AAC (macOS has them too: the tests run it there);
//! - [`android`]: MediaCodec's H.264 and AAC.
//!
//! Each is given the same NV12 frames and 48 kHz samples, and gives back what its codecs made as they made it
//! ([`crate::codecs::Coded`]); the rest of a video is the same for all.

#[cfg(target_os = "android")]
pub(crate) mod android;
#[cfg(desktop)]
pub(crate) mod desktop;
#[cfg(any(target_os = "ios", all(test, target_os = "macos")))]
#[allow(unsafe_code)]
pub(crate) mod ios;
#[cfg(web)]
pub(crate) mod web;

#[cfg(target_os = "android")]
pub(crate) use android::start;
#[cfg(desktop)]
pub(crate) use desktop::start;
#[cfg(target_os = "ios")]
pub(crate) use ios::start;
#[cfg(web)]
pub(crate) use web::start;
