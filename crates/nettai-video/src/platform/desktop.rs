//! The desktop's codecs: `ffmpeg` run as children, encoding and nothing more, one a track: one for the picture,
//! written the frames' NV12 on its standard input, and one for each sound track (a console shown), written its
//! samples, 48 kHz stereo 32-bit floats, on its own. Each writes its codec's raw output on its standard output: the
//! picture's H.264 as Annex B, each frame's access unit after a delimiter (x264's `aud`), and a sound track's AAC as
//! ADTS. They are read as they come, cut into frames by the crate's own readers (`h264::Stream`, `aac::Adts`), and
//! made a video the way every platform's are ([`crate::assemble`], [`crate::mp4`]). It links no ffmpeg library and
//! no encoder; it is the command lines
//!
//! ```text
//! ffmpeg -hide_banner -nostdin -loglevel error
//!        -f rawvideo -pixel_format nv12 -video_size 480x320 -framerate 262144/4389 -i pipe:0
//!        -c:v libx264 -preset medium -tune animation -crf 16 -g 120 -bf 0
//!        -x264-params aud=1:colorprim=bt709:transfer=bt709:colormatrix=bt709:range=tv
//!        -f h264 pipe:1
//!
//! ffmpeg -hide_banner -nostdin -loglevel error
//!        -f f32le -sample_rate 48000 -ch_layout stereo -i pipe:0
//!        -c:a aac -b:a 192k
//!        -f adts pipe:1
//! ```
//!
//! (the picture's size the video's, its rate a tick's: [`picture_arguments`], [`sound_arguments`]).
//!
//! - **A child a track** (the user: "can we avoid emitting nut entirely?", then "well if i feed them separately
//!   will it cause pipe problems?"). Each child reads its own standard input and nothing else, and its output
//!   and what it says are read as they come, on threads of their own; so no child waits on another, and a full
//!   pipe only means that child is busy. The frames are written to them in turns, from a thread of its own that
//!   takes them through a queue of three (`QUEUE`: a write waits while a child is behind; the frames after it are
//!   drawn meanwhile). One ffmpeg reading every track would need a pipe a track beyond its standard input (more
//!   descriptors on Unix, named pipes on Windows), and could stall: its output interleaves the tracks, so it may
//!   wait on one input while the writer waits on another's full pipe.
//! - **The picture** is H.264 by libx264 at a constant quality ([`codecs::QUALITY`], tuned for flat color), of the
//!   NV12 every platform is given (`yuv`: ffmpeg converts nothing), with no B-frames (`-bf 0`: its frames come out
//!   in the order shown, as every platform's do), a key frame at least every 120 frames (`-g`), and its colors tagged
//!   BT.709 in the video range in its SPS (`x264-params`, which x264 takes over what ffmpeg says of the frames). Its
//!   times are the frames' count on a tick's time base (`-framerate` the tick's rate as a fraction), exact.
//! - **The sound** is AAC at [`codecs::SOUND_BITRATE`], of the samples as they come: ffmpeg resamples nothing. ADTS
//!   says no priming: ffmpeg's AAC encoder puts 1024 samples before the sound ([`PRIMING`]), which its track is shown
//!   from past.
//! - **Errors** say which of two things went wrong ([`crate::Error`]): ffmpeg couldn't be started, or one of the
//!   children failed (with what it said; one that said the disk is full is the disk's error).
//! - **Which ffmpeg**: [`find_ffmpeg`]'s, the one nettai's packages bundle, else one installed (`$NETTAI_FFMPEG`
//!   says another). The bundled one is a minimal build with what these commands need and nothing more
//!   (nettaisen/ffmpeg-build: the rawvideo and pcm_f32le demuxers, libx264 and aac, and the h264 and adts muxers,
//!   which its build at ffmpeg-9.0.2-2 hasn't: it has the mp4 muxer these commands used before; docs/app.md §19).

use crate::Error;
use crate::codecs::{self, Audio, Codecs, Coded, H264, Sound, Video};
use crate::{aac, h264};
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::thread::JoinHandle;

/// How much of what each child says is kept for a log: the last of it.
const MOST_LOG: usize = 16 * 1024;

/// The samples ffmpeg's AAC encoder puts before the sound: its encoder's delay, `initial_padding`, which
/// libavcodec's `aacenc.c` sets to 1024 as it starts (`aac_encode_init`), and which ffmpeg's MP4 muxer wrote as the
/// track's edit list before.
pub const PRIMING: u32 = 1024;

/// The picture's ffmpeg's arguments: the NV12 frames of `video` read from its standard input, its H.264 written to
/// its standard output as Annex B.
pub fn picture_arguments(video: &Video) -> Vec<OsString> {
    let (size, rate, crf, keys) = (format!("{}x{}", video.width, video.height), format!("{}/{}", video.frame_time.1, video.frame_time.0), codecs::QUALITY.to_string(), codecs::KEY_FRAMES.to_string());
    let input = ["-f", "rawvideo", "-pixel_format", "nv12", "-video_size", &size, "-framerate", &rate, "-i", "pipe:0"];
    let encode = [
        "-c:v", "libx264", "-preset", "medium", "-tune", "animation", "-crf", &crf, "-g", &keys, "-bf", "0", "-x264-params",
        "aud=1:colorprim=bt709:transfer=bt709:colormatrix=bt709:range=tv",
    ];
    arguments(&[&input[..], &encode, &["-f", "h264", "pipe:1"]])
}

/// A sound track's ffmpeg's arguments: stereo samples of `audio`, 32-bit floats, read from its standard input, its
/// AAC written to its standard output as ADTS.
pub fn sound_arguments(audio: &Audio) -> Vec<OsString> {
    let (rate, bits) = (audio.sample_rate.to_string(), codecs::SOUND_BITRATE.to_string());
    let input = ["-f", "f32le", "-sample_rate", &rate, "-ch_layout", "stereo", "-i", "pipe:0"];
    arguments(&[&input[..], &["-c:a", "aac", "-b:a", &bits], &["-f", "adts", "pipe:1"]])
}

/// A child's arguments: quiet, never reading the terminal, then `parts`.
fn arguments(parts: &[&[&str]]) -> Vec<OsString> {
    ["-hide_banner", "-nostdin", "-loglevel", "error"].iter().chain(parts.iter().copied().flatten()).map(OsString::from).collect()
}

/// The `ffmpeg` to run: the first of
///
/// 1. `$NETTAI_FFMPEG`, as it is (a path, or a name the system looks up);
/// 2. `ffmpeg` beside the running program: the one a package bundles (beside
///    `nettai.exe`, in a macOS bundle's `Contents/MacOS`, in the AppImage's
///    `usr/lib/nettai`);
/// 3. `ffmpeg` in a folder of the path, or where a package manager puts it
///    (`/opt/homebrew/bin`, `/usr/local/bin`, `/opt/local/bin`: an app
///    started from the desktop has a short path).
///
/// Only the path's absolute folders are looked in: a relative one (`.`, or
/// an empty entry, which means the same) would run whatever `ffmpeg` the
/// working directory has. Found nowhere, it is the bundled one's place,
/// which then fails to start ([`Error::NoFfmpeg`]).
pub fn find_ffmpeg() -> PathBuf {
    let named = std::env::var_os("NETTAI_FFMPEG").filter(|n| !n.is_empty());
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let usual: &[&str] = if cfg!(unix) {
        &["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"]
    } else {
        &[]
    };
    found_ffmpeg(
        named,
        beside.as_deref(),
        &std::env::var_os("PATH").unwrap_or_default(),
        usual,
    )
}

/// [`find_ffmpeg`] with what it reads given: the variable, the running
/// program's folder, the path, and the package managers' folders.
fn found_ffmpeg(
    named: Option<OsString>,
    beside: Option<&Path>,
    path: &std::ffi::OsStr,
    usual: &[&str],
) -> PathBuf {
    if let Some(named) = named {
        return named.into();
    }
    let name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let path = std::env::split_paths(path).filter(|dir| dir.is_absolute());
    let folders = beside
        .map(Path::to_path_buf)
        .into_iter()
        .chain(path)
        .chain(usual.iter().map(PathBuf::from));
    let mut places = folders.map(|dir| dir.join(name));
    places
        .find(|p| p.is_file())
        .or_else(|| beside.map(|dir| dir.join(name)))
        .unwrap_or_default()
}


/// How many frames may wait for the children to take them: the frames are drawn while ffmpeg reads and encodes
/// those before, not in turns with it.
const QUEUE: usize = 3;

/// ffmpeg coding a video, a child a track: written each track's raw frames or samples, what it codes read back.
pub struct Ffmpeg {
    /// The picture's child, then each sound track's.
    children: Vec<Coder>,
    /// The frames to the children's standard inputs, written on a thread of its own, until they end.
    writing: Option<Writing>,
    /// What the children's standard outputs said, read on threads of their own.
    heard: Receiver<Result<Coded, Error>>,
}

/// One track's ffmpeg: the child, the thread reading its output, and the one reading what it says.
struct Coder {
    child: Child,
    reading: Option<JoinHandle<()>>,
    log: Option<JoinHandle<String>>,
}

/// Which track a child codes: the picture, or a sound track (from 0).
#[derive(Clone, Copy)]
enum Output {
    Picture,
    Sound(usize),
}

/// The thread that writes the frames to the children's standard inputs (each write waits while that child is
/// behind): the frames to write, at most [`QUEUE`] waiting; their pictures' buffers back once written; and how it
/// ended (the inputs closed, or let go of once the frames stop; or the write that failed, and whose).
struct Writing {
    frames: SyncSender<Written>,
    spent: Receiver<Vec<u8>>,
    thread: JoinHandle<Result<(), (usize, io::Error)>>,
}

/// What the writing thread is given: a frame's picture and each sound track's sound, or the end.
enum Written {
    Frame(Vec<u8>, Vec<Vec<[f32; 2]>>),
    End,
}

impl Writing {
    /// Write to `inputs` (the picture's child's, then each sound track's) on a thread of its own.
    fn start(mut inputs: Vec<ChildStdin>) -> io::Result<Writing> {
        let (frames, given) = mpsc::sync_channel(QUEUE);
        let (back, spent) = mpsc::channel();
        let thread = std::thread::Builder::new().name("ffmpeg-input".into()).spawn(move || {
            let mut bytes = Vec::new();
            for written in given {
                match written {
                    Written::Frame(nv12, sound) => {
                        inputs[0].write_all(&nv12).map_err(|e| (0, e))?;
                        for (track, sound) in sound.iter().enumerate() {
                            bytes.clear();
                            bytes.extend(sound.as_flattened().iter().flat_map(|s| s.to_le_bytes()));
                            inputs[1 + track].write_all(&bytes).map_err(|e| (1 + track, e))?;
                        }
                        let _ = back.send(nv12);
                    }
                    // (The inputs closed: each child finishes its track.)
                    Written::End => return Ok(()),
                }
            }
            // (Let go of with no end: canceled.)
            Ok(())
        })?;
        Ok(Writing { frames, spent, thread })
    }

    /// How the thread ended, once it has: every input closed, or the write that failed, and whose.
    fn ended(self) -> Result<(), (usize, io::Error)> {
        drop(self.frames);
        self.thread.join().unwrap_or_else(|_| Err((0, io::Error::other("the thread writing to ffmpeg stopped"))))
    }
}

/// The desktop's codecs for a video of `video` and `audio`: ffmpeg's ([`find_ffmpeg`]).
pub(crate) fn start(video: Video, audio: Option<Audio>) -> Result<Box<dyn Codecs>, Error> {
    Ok(Box::new(Ffmpeg::start(&find_ffmpeg(), video, audio)?))
}

impl Ffmpeg {
    /// Start `program` (an ffmpeg) coding the picture `video` and the sound `audio`: a child for the picture and one
    /// for each sound track. An ffmpeg that can't be started is [`Error::NoFfmpeg`].
    fn start(program: &Path, video: Video, audio: Option<Audio>) -> Result<Ffmpeg, Error> {
        let (tell, heard) = mpsc::channel();
        let mut commands = vec![(picture_arguments(&video), Output::Picture)];
        if let Some(audio) = audio {
            commands.extend((0..audio.tracks).map(|t| (sound_arguments(&audio), Output::Sound(t))));
        }
        let mut ffmpeg = Ffmpeg { children: Vec::new(), writing: None, heard };
        let mut inputs = Vec::new();
        for (arguments, output) in commands {
            let (coder, input) = Coder::start(program, &arguments, output, &tell).map_err(Error::NoFfmpeg)?;
            ffmpeg.children.push(coder);
            inputs.push(input);
        }
        match Writing::start(inputs) {
            Ok(writing) => ffmpeg.writing = Some(writing),
            Err(e) => return Err(ffmpeg.failed(0, e)),
        }
        Ok(ffmpeg)
    }

    /// Writing to child `which` failed with `error`: a pipe that closed is that child having ended, and what it
    /// said is the reason; otherwise the error itself is. Every child is stopped either way.
    fn failed(&mut self, which: usize, error: io::Error) -> Error {
        self.writing = None;
        let ended = if error.kind() == io::ErrorKind::BrokenPipe { self.children[which].child.wait().ok() } else { None };
        let why = match ended {
            Some(status) if !status.success() => failure(status.code(), self.children[which].said()),
            _ => Error::Write(error),
        };
        self.stop();
        why
    }

    /// Stop the children and wait for them, and for the thread writing to them (a write it waits in fails as its
    /// child ends).
    fn stop(&mut self) {
        let writing = self.writing.take();
        for coder in &mut self.children {
            let _ = coder.child.kill();
        }
        if let Some(writing) = writing {
            let _ = writing.ended();
        }
        for coder in &mut self.children {
            let _ = coder.child.wait();
            coder.log = None;
        }
    }
}

impl Coder {
    /// `program` started with `arguments`, coding `output`: what its output says told on `tell`. Its standard
    /// input, to write to.
    fn start(program: &Path, arguments: &[OsString], output: Output, tell: &Sender<Result<Coded, Error>>) -> io::Result<(Coder, ChildStdin)> {
        let mut command = Command::new(program);
        command.args(arguments).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        // (On Windows a console program started by a windowed one opens a console window of its own unless told
        // not to: the user, "make ffmpeg not open a terminal window on windows".)
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command.spawn()?;
        let input = child.stdin.take().expect("ffmpeg's standard input, piped");
        let out = child.stdout.take().expect("ffmpeg's standard output, piped");
        let mut said = child.stderr.take().expect("ffmpeg's standard error, piped");
        // (Read as it comes, so the child never waits on a full pipe.)
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
        let tell = tell.clone();
        let reading = std::thread::spawn(move || match output {
            Output::Picture => read_picture(out, &tell),
            Output::Sound(track) => read_sound(out, track, &tell),
        });
        Ok((Coder { child, reading: Some(reading), log: Some(log) }, input))
    }

    /// What the child said, once it has ended.
    fn said(&mut self) -> String {
        self.log.take().and_then(|log| log.join().ok()).unwrap_or_default()
    }
}

impl Codecs for Ffmpeg {
    fn frame(&mut self, nv12: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        let writing = self.writing.as_ref().ok_or_else(|| Error::Write(io::ErrorKind::BrokenPipe.into()))?;
        let sound = sound.iter().map(|s| s.to_vec()).collect();
        if writing.frames.send(Written::Frame(nv12, sound)).is_ok() {
            return Ok(writing.spent.try_recv().unwrap_or_default());
        }
        // (The thread ended: a write failed.)
        let (which, error) = self.writing.take().map_or(Ok(()), Writing::ended).err().unwrap_or_else(|| (0, io::ErrorKind::BrokenPipe.into()));
        Err(self.failed(which, error))
    }

    fn coded(&mut self) -> Result<Vec<Coded>, Error> {
        self.heard.try_iter().collect()
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<Coded>, Error> {
        if let Some(writing) = self.writing.take() {
            let _ = writing.frames.send(Written::End);
            if let Err((which, e)) = writing.ended() {
                return Err(self.failed(which, e));
            }
        }
        // (Each child, its input closed, finishes its track and ends; the first that failed is the reason.)
        let mut failed = None;
        for coder in &mut self.children {
            if let Some(reading) = coder.reading.take() {
                let _ = reading.join();
            }
            let status = coder.child.wait();
            let log = coder.said();
            let why = match status {
                Ok(status) if status.success() => None,
                Ok(status) => Some(failure(status.code(), log)),
                Err(e) => Some(Error::FfmpegFailed { code: None, log: format!("{e}\n{log}") }),
            };
            failed = failed.or(why);
        }
        match failed {
            Some(why) => Err(why),
            None => self.coded(),
        }
    }

    fn cancel(mut self: Box<Self>) {
        self.stop();
    }
}

impl Drop for Ffmpeg {
    /// Codecs let go of before they finished are stopped.
    fn drop(&mut self) {
        if self.children.iter().any(|c| c.log.is_some()) {
            self.stop();
        }
    }
}

/// An ffmpeg that ended badly, by what it said: the disk's error where it ran out of room, else its own.
fn failure(code: Option<i32>, log: String) -> Error {
    if log.contains("No space left on device") {
        return Error::Disk(io::Error::new(io::ErrorKind::StorageFull, log));
    }
    Error::FfmpegFailed { code, log }
}

/// The picture's child's H.264, read as it comes: a frame each access unit, told on `tell`. (It ends where the
/// child's output does.)
fn read_picture(mut out: impl Read, tell: &Sender<Result<Coded, Error>>) {
    let mut stream = h264::Stream::default();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let n = match out.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                let _ = tell.send(Err(Error::Encoder(format!("reading ffmpeg's H.264: {e}"))));
                return;
            }
        };
        for unit in stream.push(&buffer[..n]) {
            if tell.send(Ok(Coded::Frame(H264::AnnexB(unit)))).is_err() {
                return;
            }
        }
    }
    if let Some(unit) = stream.end() {
        let _ = tell.send(Ok(Coded::Frame(H264::AnnexB(unit))));
    }
}

/// Sound track `track`'s child's ADTS, read as it comes: its coding once its first header says it, then a frame
/// each, told on `tell`.
fn read_sound(mut out: impl Read, track: usize, tell: &Sender<Result<Coded, Error>>) {
    let mut adts = aac::Adts::default();
    let mut said = false;
    let mut buffer = vec![0u8; 1 << 14];
    loop {
        let n = match out.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                let _ = tell.send(Err(Error::Encoder(format!("reading ffmpeg's AAC: {e}"))));
                return;
            }
        };
        let frames = match adts.push(&buffer[..n]) {
            Ok(frames) => frames,
            Err(why) => {
                let _ = tell.send(Err(Error::Encoder(format!("ffmpeg's AAC: {why}"))));
                return;
            }
        };
        if !said && let Some(config) = adts.config {
            said = true;
            let _ = tell.send(Ok(Coded::Coding(track, Sound::Aac { config: config.bytes(), priming: PRIMING })));
        }
        for frame in frames {
            if tell.send(Ok(Coded::Sound(track, frame))).is_err() {
                return;
            }
        }
    }
    if adts.torn() {
        let _ = tell.send(Err(Error::Encoder("ffmpeg's AAC ended in the middle of a frame".into())));
    }
}

#[cfg(test)]
mod tests;
