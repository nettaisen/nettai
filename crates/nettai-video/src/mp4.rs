//! An MP4 of a NUT stream, by `ffmpeg` run as a child: [`Encoder`] starts
//! it reading the stream from a pipe, is written the stream (it is a
//! `std::io::Write`), and ends with the file whole under its name, or with
//! nothing. It links no ffmpeg library and no encoder; it is the command
//! line
//!
//! ```text
//! ffmpeg -hide_banner -nostdin -loglevel error -f nut -i pipe:0
//!        -vf scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format=yuv420p
//!        -c:v libx264 -preset medium -tune animation -crf 16
//!        -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv
//!        -c:a aac -b:a 192k -ar 48000
//!        -movflags +faststart -f mp4 -y FILE.part
//! ```
//!
//! and works on any NUT stream ([`crate::nut`]'s, or a saved `.nut`
//! copied into it).
//!
//! - **The picture** is H.264 by libx264 at a constant quality (CRF 16 by
//!   default, tuned for flat color), in 4:2:0, which every player and site
//!   takes. It is not scaled here: the stream comes at its final size, so
//!   every pixel stays the square it was made as. What 4:2:0 costs is
//!   color at half the resolution, and how much depends on the size. At an
//!   even multiple of the frame every game pixel covers whole color
//!   samples, which are made as the mean of the pixels they cover (the
//!   `area` flag: ffmpeg's default filter reaches into the neighbors), so
//!   edges stay clean; at an odd multiple a color sample straddles two
//!   game pixels and every edge gets a fringe of mixed color, whatever the
//!   quality. Measured on 120 frames of an EXE6 fight (the decoded MP4
//!   against the stream, RGB PSNR): in 4:2:0, 40 dB at twice the frame
//!   and 44 dB at four times, each at about 2 Mb/s, but 25 dB at three
//!   times, at 4 Mb/s; in 4:4:4, 38 dB at three times. So a host offers
//!   even sizes, or [`Chroma::Full`], which keeps every edge at any size
//!   in a file fewer players take. The colors are converted and
//!   tagged as BT.709, so players agree on them whatever their default for
//!   the size.
//! - **The sound** is AAC at 192 kb/s, resampled to 48 kHz: the GBA's
//!   32768 Hz is no rate AAC has.
//! - **The file** is written beside its place as `NAME.part` and takes its
//!   name when ffmpeg has ended well, with its index at the front
//!   (`+faststart`). Canceled, failed or dropped, the child is stopped and
//!   the part is removed: no half-written file is left under either name.
//! - **Errors** say which of three things went wrong ([`crate::Error`]):
//!   ffmpeg couldn't be started, ffmpeg failed (with what it said), or the
//!   disk (the file couldn't be made, or there is no room).

use crate::Error;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread::JoinHandle;

/// How much of what ffmpeg says is kept for a log: the last of it.
const MOST_LOG: usize = 16 * 1024;

/// How an MP4's color is sampled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Chroma {
    /// 4:2:0 (`yuv420p`): color at half the picture's resolution. Every
    /// player takes it. Clean at an even multiple of the frame; at an odd
    /// one, every pixel's edge has a fringe of mixed color.
    #[default]
    Half,
    /// 4:4:4 (`yuv444p`): color at the picture's resolution, every pixel
    /// edge sharp at any size. Browsers' and phones' hardware decoders
    /// mostly don't take it.
    Full,
}

/// How an MP4 is encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The ffmpeg program: a name looked up on the path, or a path.
    pub ffmpeg: PathBuf,
    /// libx264's constant rate factor: lower is closer to the frames and
    /// larger (0 lossless, 16 the default here, 23 libx264's own).
    pub crf: u8,
    pub chroma: Chroma,
    /// The sound's bit rate, in kb/s.
    pub audio_kbps: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { ffmpeg: "ffmpeg".into(), crf: 16, chroma: Chroma::Half, audio_kbps: 192 }
    }
}

impl Settings {
    /// ffmpeg's arguments to read a NUT stream from its standard input and
    /// write an MP4 to `out`.
    pub fn arguments(&self, out: &Path) -> Vec<OsString> {
        let format = match self.chroma {
            Chroma::Half => "yuv420p",
            Chroma::Full => "yuv444p",
        };
        let filter = format!("scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format={format}");
        let (crf, audio) = (self.crf.to_string(), format!("{}k", self.audio_kbps));
        let mut args: Vec<OsString> = [
            "-hide_banner", "-nostdin", "-loglevel", "error",
            "-f", "nut", "-i", "pipe:0",
            "-vf", &filter,
            "-c:v", "libx264", "-preset", "medium", "-tune", "animation", "-crf", &crf,
            "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv",
            "-c:a", "aac", "-b:a", &audio, "-ar", "48000",
            "-movflags", "+faststart", "-f", "mp4", "-y",
        ]
        .iter()
        .map(OsString::from)
        .collect();
        args.push(out.into());
        args
    }
}

/// ffmpeg making an MP4 of the NUT stream written to this. [`Encoder::finish`]
/// ends it and gives the file its name; anything else (canceling, a
/// failure, dropping it) leaves no file.
pub struct Encoder {
    child: Child,
    /// ffmpeg's standard input, until the stream ends.
    stdin: Option<ChildStdin>,
    /// What ffmpeg says, being read.
    log: Option<JoinHandle<String>>,
    /// The file's place, and where it is written until it is whole.
    to: PathBuf,
    part: PathBuf,
}

impl Encoder {
    /// Start ffmpeg writing an MP4 that will be `to`. Refused: a file that
    /// can't be made there ([`Error::Disk`]), and an ffmpeg that can't be
    /// started ([`Error::NoFfmpeg`]).
    pub fn start(to: &Path, settings: &Settings) -> Result<Encoder, Error> {
        let mut name = to.file_name().ok_or_else(|| Error::Disk(io::Error::new(io::ErrorKind::InvalidInput, format!("{} names no file", to.display()))))?.to_os_string();
        name.push(".part");
        let part = to.with_file_name(name);
        // (The file made here first: a place that can't be written is the
        // disk's error, told before ffmpeg is started.)
        std::fs::File::create(&part).map_err(Error::Disk)?;
        let spawned = Command::new(&settings.ffmpeg)
            .args(settings.arguments(&part))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => {
                let _ = std::fs::remove_file(&part);
                return Err(Error::NoFfmpeg(e));
            }
        };
        let stdin = child.stdin.take();
        let mut said = child.stderr.take().expect("ffmpeg's standard error, piped");
        // (Read as it comes, so ffmpeg never waits on a full pipe.)
        let log = std::thread::spawn(move || {
            let mut kept = Vec::new();
            let mut buffer = [0u8; 4096];
            while let Ok(n @ 1..) = said.read(&mut buffer) {
                kept.extend_from_slice(&buffer[..n]);
                if kept.len() > 2 * MOST_LOG {
                    kept.drain(..kept.len() - MOST_LOG);
                }
            }
            let from = kept.len().saturating_sub(MOST_LOG);
            String::from_utf8_lossy(&kept[from..]).into_owned()
        });
        Ok(Encoder { child, stdin, log: Some(log), to: to.to_path_buf(), part })
    }

    /// The stream is whole: let ffmpeg finish, and give the file its name.
    /// An ffmpeg that ended badly is [`Error::FfmpegFailed`] with what it
    /// said (or [`Error::Disk`], where it said the disk is full); the file
    /// isn't there then.
    pub fn finish(mut self) -> Result<(), Error> {
        self.stdin = None;
        let status = self.child.wait();
        let log = self.said();
        let done = match status {
            Ok(status) if status.success() => std::fs::rename(&self.part, &self.to).map_err(Error::Disk),
            Ok(status) => Err(failure(status.code(), log)),
            Err(e) => Err(Error::FfmpegFailed { code: None, log: format!("{e}\n{log}") }),
        };
        if done.is_err() {
            let _ = std::fs::remove_file(&self.part);
        }
        done
    }

    /// Stop ffmpeg and leave no file.
    pub fn cancel(mut self) {
        self.stop();
    }

    /// Writing the stream failed with `error`: stop, leave no file, and say
    /// why. A pipe that closed is ffmpeg having ended: what it said is the
    /// reason; otherwise the error itself is.
    pub fn failed(mut self, error: io::Error) -> Error {
        // (An ffmpeg that ended on its own is let say how.)
        self.stdin = None;
        let ended = if error.kind() == io::ErrorKind::BrokenPipe { self.child.wait().ok() } else { None };
        let log = match ended {
            Some(_) => self.said(),
            None => {
                self.stop();
                String::new()
            }
        };
        let _ = std::fs::remove_file(&self.part);
        match ended {
            Some(status) if !status.success() => failure(status.code(), log),
            _ => Error::Write(error),
        }
    }

    /// What ffmpeg said, once it has ended.
    fn said(&mut self) -> String {
        self.log.take().and_then(|log| log.join().ok()).unwrap_or_default()
    }

    /// Stop the child, wait for it, and remove the part written. (What it
    /// said is let go unread: nobody is told it.)
    fn stop(&mut self) {
        self.stdin = None;
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.log = None;
        let _ = std::fs::remove_file(&self.part);
    }
}

/// An ffmpeg that ended badly, by what it said: the disk's error where it
/// ran out of room, else its own.
fn failure(code: Option<i32>, log: String) -> Error {
    if log.contains("No space left on device") {
        return Error::Disk(io::Error::new(io::ErrorKind::StorageFull, log));
    }
    Error::FfmpegFailed { code, log }
}

impl Write for Encoder {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match &mut self.stdin {
            Some(stdin) => stdin.write(bytes),
            None => Err(io::ErrorKind::BrokenPipe.into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.stdin {
            Some(stdin) => stdin.flush(),
            None => Ok(()),
        }
    }
}

impl Drop for Encoder {
    /// An encoder let go of before it finished leaves no file. (After
    /// `finish`, `cancel` or `failed` there is nothing left to stop.)
    fn drop(&mut self) {
        if self.log.is_some() {
            self.stop();
        }
    }
}

#[cfg(test)]
mod tests;
