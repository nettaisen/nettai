//! The desktop's encoder: `ffmpeg` run as a child, encoding and nothing more. It is written the playing's NUT
//! stream ([`crate::nut`]: raw frames and PCM sound, one sound track a console shown) on its standard input, from
//! a thread of its own that takes the frames through a queue of three (`QUEUE`: a write waits while ffmpeg is behind;
//! the frames after it are drawn meanwhile), and writes a fragmented MP4 of the encoded picture and sound on its
//! standard output, which is read back as it comes ([`Ffmpeg`]): the tracks from its `moov`, each fragment's
//! samples from its `moof` and `mdat`. What is made of
//! them is the same MP4 the browser's encoder makes ([`crate::fmp4`]: its tracks titled, its chapters). It links no
//! ffmpeg library and no encoder; it is the command line
//!
//! ```text
//! ffmpeg -hide_banner -nostdin -loglevel error -f nut -i pipe:0 -map 0:v -map 0:a? -map_chapters -1
//!        -vf scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format=yuv420p,setparams=color_primaries=bt709:color_trc=bt709
//!        -c:v libx264 -preset medium -tune animation -crf 16 -g 120
//!        -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv
//!        -c:a aac -b:a 192k -ar 48000
//!        -f mp4 -movflags +frag_keyframe+empty_moov+default_base_moof -frag_duration 2000000 pipe:1
//! ```
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
//!   the size. (The primaries and the transfer are tagged on the frames,
//!   by `setparams`: ffmpeg 9.0's encoder takes them from the frames, which
//!   the stream's RGB gives none, and with `-color_primaries` and
//!   `-color_trc` alone the MP4 said "unknown" for both.)
//! - **The sound** is AAC at 192 kb/s, resampled to 48 kHz: the GBA's 32768 Hz is no rate AAC has. Every sound
//!   track of the stream is one (`-map 0:a?`: by itself ffmpeg takes one sound).
//! - **The fragments** are about two seconds each: a key frame every 120 frames (`-g`), a fragment from each.
//! - **Errors** say which of two things went wrong ([`crate::Error`]): ffmpeg couldn't be started, or it failed
//!   (with what it said; one that said the disk is full is the disk's error).
//! - **Which ffmpeg**: [`find_ffmpeg`]'s, the one nettai's packages bundle, else one installed (`$NETTAI_FFMPEG`
//!   says another). The bundled one is a minimal build with what this command needs and nothing more
//!   (nettaisen/ffmpeg-build).
//!
//! The NUT stream is [`nut`]'s, which [`write_nut`] also saves as it is (a stream ffmpeg can be given by hand).

pub mod nut;

use crate::encode::{Audio, Backend, Chroma, Encoding, Event, Info, Kind, Packet, Track, Video};
use crate::{Error, OnProgress, Options};
use nettai_frontend::game::Loaded;
use nettai_frontend::replay::Replay;
use nettai_frontend::viewer::Viewer;
use std::ffi::OsString;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::thread::JoinHandle;

/// How much of what ffmpeg says is kept for a log: the last of it.
const MOST_LOG: usize = 16 * 1024;

/// ffmpeg's arguments: a NUT stream read from its standard input, a fragmented MP4 written to its standard output.
pub fn arguments(encoding: &Encoding) -> Vec<OsString> {
    let format = match encoding.chroma {
        Chroma::Half => "yuv420p",
        Chroma::Full => "yuv444p",
    };
    let filter = format!(
        "scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format={format},setparams=color_primaries=bt709:color_trc=bt709"
    );
    let (crf, audio) = (encoding.crf.to_string(), format!("{}k", encoding.audio_kbps));
    [
        "-hide_banner", "-nostdin", "-loglevel", "error", "-f", "nut", "-i", "pipe:0", "-map", "0:v", "-map", "0:a?",
        "-map_chapters", "-1", "-vf", &filter, "-c:v", "libx264", "-preset", "medium", "-tune", "animation", "-crf", &crf,
        "-g", "120", "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv",
        "-c:a", "aac", "-b:a", &audio, "-ar", "48000", "-f", "mp4", "-movflags", "+frag_keyframe+empty_moov+default_base_moof",
        "-frag_duration", "2000000", "pipe:1",
    ]
    .iter()
    .map(OsString::from)
    .collect()
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

/// How many frames may wait for ffmpeg to take them: the frames are drawn while ffmpeg reads and encodes those
/// before, not in turns with it.
const QUEUE: usize = 3;

/// ffmpeg encoding a video: written its frames as a NUT stream, its encoded tracks and packets read back.
pub struct Ffmpeg {
    child: Child,
    /// The stream to ffmpeg's standard input, written on a thread of its own, until it ends.
    writing: Option<Writing>,
    /// What its standard output said, read on a thread of its own.
    events: Receiver<Result<Event, Error>>,
    reading: Option<JoinHandle<()>>,
    /// What ffmpeg says, being read.
    log: Option<JoinHandle<String>>,
}

/// The thread that writes the stream to ffmpeg's standard input (each write waits while ffmpeg is behind): the
/// frames to write, at most [`QUEUE`] waiting; their pictures' buffers back once written; and how it ended (the
/// stream finished, or let go of once the frames stop; or the write that failed).
struct Writing {
    frames: SyncSender<Written>,
    spent: Receiver<Vec<u8>>,
    thread: JoinHandle<io::Result<()>>,
}

/// What the writing thread is given: a frame's picture and each sound track's sound, or the stream's end.
enum Written {
    Frame(Vec<u8>, Vec<Vec<[f32; 2]>>),
    End,
}

impl Writing {
    /// Write `stream` on a thread of its own.
    fn start(mut stream: nut::Writer<ChildStdin>) -> io::Result<Writing> {
        let (frames, given) = mpsc::sync_channel(QUEUE);
        let (back, spent) = mpsc::channel();
        let thread = std::thread::Builder::new().name("ffmpeg-input".into()).spawn(move || {
            for written in given {
                match written {
                    Written::Frame(rgb, sound) => {
                        let tracks: Vec<&[[f32; 2]]> = sound.iter().map(Vec::as_slice).collect();
                        stream.frame(&rgb, &tracks)?;
                        let _ = back.send(rgb);
                    }
                    Written::End => return stream.finish().map(drop),
                }
            }
            // (Let go of with no end: canceled. The stream's end isn't written.)
            Ok(())
        })?;
        Ok(Writing { frames, spent, thread })
    }

    /// How the thread ended, once it has: the stream written, or the write that failed.
    fn ended(self) -> io::Result<()> {
        drop(self.frames);
        self.thread.join().unwrap_or_else(|_| Err(io::Error::other("the thread writing to ffmpeg stopped")))
    }
}

/// The desktop's encoder for a video of `video` and `audio`, as `encoding` says: ffmpeg's ([`find_ffmpeg`]).
pub(crate) fn start(video: Video, audio: Option<Audio>, info: &Info, encoding: &Encoding) -> Result<Box<dyn Backend>, Error> {
    Ok(Box::new(Ffmpeg::start(&find_ffmpeg(), video, audio, info, encoding)?))
}

/// Write the replay `viewer` plays to `out` as a NUT stream (ffmpeg's input: raw frames and PCM sound), as
/// `options` say, its sound tracks titled and its rounds chapters as an MP4's are. The viewer is left paused at the
/// part's end, at the original's speed. `progress` is told before the first frame and after each, and may stop it
/// ([`Error::Canceled`]: what was written of the stream stays in `out`, which is the caller's to throw away).
pub fn write_nut<W: std::io::Write>(viewer: &mut Viewer, options: &Options, out: W, progress: OnProgress) -> Result<W, Error> {
    let stream = crate::play(viewer, options, progress, |video, audio, info| nut::Writer::with_info(out, video, audio, info).map_err(Error::Write))?;
    stream.finish().map_err(Error::Write)
}

/// Write a replay to `out` as a NUT stream ([`write_nut`]), as [`crate::export`] plays one.
pub fn export_nut<W: std::io::Write>(game: &Loaded, replay: &Replay, options: &Options, out: W, progress: OnProgress) -> Result<W, Error> {
    let mut viewer = Viewer::open(game, replay, options.view).map_err(Error::Replay)?;
    write_nut(&mut viewer, options, out, progress)
}

impl<W: std::io::Write> crate::Frames for nut::Writer<W> {
    fn frame(&mut self, rgb: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        nut::Writer::frame(self, &rgb, sound).map_err(Error::Write)?;
        Ok(rgb)
    }
}

impl Ffmpeg {
    /// Start `program` (an ffmpeg) encoding the picture `video` and the sound `audio` (their stream's info `info`),
    /// as `encoding` says. An ffmpeg that can't be started is [`Error::NoFfmpeg`].
    fn start(program: &Path, video: Video, audio: Option<Audio>, info: &Info, encoding: &Encoding) -> Result<Ffmpeg, Error> {
        let mut command = Command::new(program);
        command.args(arguments(encoding)).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        // (On Windows a console program started by a windowed one opens a console window of its own unless told
        // not to: the user, "make ffmpeg not open a terminal window on windows".)
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command.spawn().map_err(Error::NoFfmpeg)?;
        let stdin = child.stdin.take().expect("ffmpeg's standard input, piped");
        let stdout = child.stdout.take().expect("ffmpeg's standard output, piped");
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
        let (tell, events) = mpsc::channel();
        let (width, height) = (video.width, video.height);
        let reading = std::thread::spawn(move || read(stdout, width, height, &tell));
        let mut ffmpeg = Ffmpeg { child, writing: None, events, reading: Some(reading), log: Some(log) };
        match nut::Writer::with_info(stdin, video, audio, info).and_then(Writing::start) {
            Ok(writing) => ffmpeg.writing = Some(writing),
            Err(e) => return Err(ffmpeg.failed(e)),
        }
        Ok(ffmpeg)
    }

    /// Writing to ffmpeg failed with `error`: a pipe that closed is ffmpeg having ended, and what it said is the
    /// reason; otherwise the error itself is. It is stopped either way.
    fn failed(&mut self, error: io::Error) -> Error {
        self.writing = None;
        let ended = if error.kind() == io::ErrorKind::BrokenPipe { self.child.wait().ok() } else { None };
        match ended {
            Some(status) if !status.success() => failure(status.code(), self.said()),
            _ => {
                self.stop();
                Error::Write(error)
            }
        }
    }

    /// What ffmpeg said, once it has ended.
    fn said(&mut self) -> String {
        self.log.take().and_then(|log| log.join().ok()).unwrap_or_default()
    }

    /// Stop the child and wait for it, and for the thread writing to it (a write it waits in fails as the child
    /// ends).
    fn stop(&mut self) {
        let writing = self.writing.take();
        let _ = self.child.kill();
        if let Some(writing) = writing {
            let _ = writing.ended();
        }
        let _ = self.child.wait();
        self.log = None;
    }
}

impl Backend for Ffmpeg {
    fn frame(&mut self, rgb: Vec<u8>, sound: &[&[[f32; 2]]]) -> Result<Vec<u8>, Error> {
        let writing = self.writing.as_ref().ok_or_else(|| Error::Write(io::ErrorKind::BrokenPipe.into()))?;
        let sound = sound.iter().map(|s| s.to_vec()).collect();
        if writing.frames.send(Written::Frame(rgb, sound)).is_ok() {
            return Ok(writing.spent.try_recv().unwrap_or_default());
        }
        // (The thread ended: a write failed.)
        let ended = self.writing.take().map_or(Ok(()), Writing::ended);
        Err(self.failed(ended.err().unwrap_or_else(|| io::ErrorKind::BrokenPipe.into())))
    }

    fn events(&mut self) -> Result<Vec<Event>, Error> {
        self.events.try_iter().collect()
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<Event>, Error> {
        if let Some(writing) = self.writing.take() {
            let _ = writing.frames.send(Written::End);
            if let Err(e) = writing.ended() {
                return Err(self.failed(e));
            }
        }
        if let Some(reading) = self.reading.take() {
            let _ = reading.join();
        }
        let status = self.child.wait();
        let log = self.said();
        match status {
            Ok(status) if status.success() => self.events.try_iter().collect(),
            Ok(status) => Err(failure(status.code(), log)),
            Err(e) => Err(Error::FfmpegFailed { code: None, log: format!("{e}\n{log}") }),
        }
    }

    fn cancel(mut self: Box<Self>) {
        self.stop();
    }
}

impl Drop for Ffmpeg {
    /// An encoder let go of before it finished is stopped.
    fn drop(&mut self) {
        if self.log.is_some() {
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

/// ffmpeg's fragmented MP4, read as it comes: its tracks (the picture `width` by `height`, then the sound tracks),
/// then each fragment's packets, told on `tell`. (It ends where ffmpeg's output does; what doesn't read is told.)
fn read(mut out: impl Read, width: u32, height: u32, tell: &Sender<Result<Event, Error>>) {
    use mp4_atom::{Decode, Moof, Moov};
    let unreadable = |what: &str, e: &dyn std::fmt::Display| Error::Encoder(format!("ffmpeg's MP4: {what}: {e}"));
    // Each track by its id: its place among the tracks, and its samples' defaults (the `trex`).
    let mut tracks: Vec<(u32, mp4_atom::Trex)> = Vec::new();
    let mut moof: Option<(Moof, usize)> = None;
    loop {
        let (kind, atom, header) = match next_atom(&mut out) {
            Ok(Some(atom)) => atom,
            Ok(None) => return,
            Err(e) => {
                let _ = tell.send(Err(unreadable("reading", &e)));
                return;
            }
        };
        match &kind {
            b"moov" => {
                let moov = match Moov::decode(&mut &atom[..]) {
                    Ok(moov) => moov,
                    Err(e) => {
                        let _ = tell.send(Err(unreadable("its moov", &e)));
                        return;
                    }
                };
                let trex = moov.mvex.map(|m| m.trex).unwrap_or_default();
                let movie = moov.mvhd.timescale;
                let mut made = Vec::new();
                for trak in moov.trak {
                    let id = trak.tkhd.track_id;
                    let video = trak.mdia.hdlr.handler == b"vide".into();
                    let Some(codec) = trak.mdia.minf.stbl.stsd.codecs.into_iter().next() else { continue };
                    // (ffmpeg's edit list, in its movie's time scale: made the MP4's.)
                    let edits = trak.edts.map(|mut edts| {
                        if let Some(elst) = &mut edts.elst {
                            for e in &mut elst.entries {
                                e.segment_duration = e.segment_duration * crate::fmp4::MOVIE_TIMESCALE as u64 / movie.max(1) as u64;
                            }
                        }
                        edts
                    });
                    made.push(Track { kind: if video { Kind::Video { width, height } } else { Kind::Audio }, timescale: trak.mdia.mdhd.timescale, codec, edits });
                    tracks.push((id, trex.iter().find(|t| t.track_id == id).cloned().unwrap_or_default()));
                }
                let _ = tell.send(Ok(Event::Tracks(made)));
            }
            b"moof" => match Moof::decode(&mut &atom[..]) {
                Ok(m) => moof = Some((m, atom.len())),
                Err(e) => {
                    let _ = tell.send(Err(unreadable("a moof", &e)));
                    return;
                }
            },
            b"mdat" => {
                let Some((m, moof_len)) = moof.take() else { continue };
                // (Offsets are from the moof's start: the mdat's data begins past the moof and its own header.)
                let data_start = moof_len + header;
                for traf in &m.traf {
                    let Some(place) = tracks.iter().position(|(id, _)| *id == traf.tfhd.track_id) else { continue };
                    let trex = &tracks[place].1;
                    let mut time = traf.tfdt.as_ref().map_or(0, |t| t.base_media_decode_time);
                    for trun in &traf.trun {
                        let mut at = (trun.data_offset.unwrap_or(0) as usize).saturating_sub(data_start) + header;
                        for entry in &trun.entries {
                            let size = entry.size.or(traf.tfhd.default_sample_size).unwrap_or(trex.default_sample_size) as usize;
                            let duration = entry.duration.or(traf.tfhd.default_sample_duration).unwrap_or(trex.default_sample_duration);
                            let flags = entry.flags.or(traf.tfhd.default_sample_flags).unwrap_or(trex.default_sample_flags);
                            let Some(data) = atom.get(at..at + size) else {
                                let _ = tell.send(Err(unreadable("a sample", &"past its mdat")));
                                return;
                            };
                            let packet = Packet { track: place, data: data.to_vec(), time, offset: entry.cts.unwrap_or(0), duration, key: flags & 0x0001_0000 == 0 };
                            if tell.send(Ok(Event::Packet(packet))).is_err() {
                                return;
                            }
                            at += size;
                            time += duration as u64;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// The next top-level atom of an MP4 being read: its kind, all its bytes (its header with them) and its header's
/// length; none at the end.
fn next_atom(out: &mut impl Read) -> io::Result<Option<([u8; 4], Vec<u8>, usize)>> {
    let mut head = [0u8; 8];
    match out.read_exact(&mut head) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let kind = [head[4], head[5], head[6], head[7]];
    let mut atom = head.to_vec();
    let size = match u32::from_be_bytes([head[0], head[1], head[2], head[3]]) {
        1 => {
            let mut large = [0u8; 8];
            out.read_exact(&mut large)?;
            atom.extend_from_slice(&large);
            u64::from_be_bytes(large) as usize
        }
        0 => {
            out.read_to_end(&mut atom)?;
            return Ok(Some((kind, atom, 8)));
        }
        size => size as usize,
    };
    let header = atom.len();
    let mut body = vec![0u8; size.saturating_sub(header)];
    out.read_exact(&mut body)?;
    atom.extend_from_slice(&body);
    Ok(Some((kind, atom, header)))
}

#[cfg(test)]
mod tests;
