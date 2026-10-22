//! The desktop's encoder: `ffmpeg` run as children, encoding and nothing more, one a track: one for the picture,
//! written its raw RGB frames on its standard input, and one for each sound track (a console shown), written its
//! samples, 48 kHz stereo 32-bit floats ([`crate::resample`]'s), on its own. Each writes a fragmented MP4 of its
//! encoded track on its standard output, which is read back as it comes ([`Ffmpeg`]): the track from its `moov`,
//! each fragment's samples from its `moof` and `mdat` (ffmpeg's fragments are how it says them as it goes; no file
//! is made of them). The tracks are put together by the crate's own muxer, the same MP4 the browser's encoder
//! makes ([`crate::mp4`]: its tracks titled, its chapters, its index first). It links no ffmpeg library and no
//! encoder; it is the command lines
//!
//! ```text
//! ffmpeg -hide_banner -nostdin -loglevel error
//!        -f rawvideo -pixel_format rgb24 -video_size 480x320 -framerate 262144/4389 -i pipe:0
//!        -vf scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format=yuv420p,setparams=color_primaries=bt709:color_trc=bt709
//!        -c:v libx264 -preset medium -tune animation -crf 16 -g 120
//!        -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv
//!        -f mp4 -movflags +frag_keyframe+empty_moov+delay_moov+default_base_moof -frag_duration 2000000 pipe:1
//!
//! ffmpeg -hide_banner -nostdin -loglevel error
//!        -f f32le -sample_rate 48000 -ch_layout stereo -i pipe:0
//!        -c:a aac -b:a 192k
//!        -f mp4 -movflags +empty_moov+delay_moov+default_base_moof -frag_duration 2000000 pipe:1
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
//!   wait on one input while the writer waits on another's full pipe. The interleaving is the muxer's here.
//! - **The picture** is H.264 by libx264 at a constant quality (CRF 16 by
//!   default, tuned for flat color), in 4:2:0, which every player and site
//!   takes. It is not scaled here: the frames come at their final size, so
//!   every pixel stays the square it was made as. What 4:2:0 costs is
//!   color at half the resolution, and how much depends on the size. At an
//!   even multiple of the frame every game pixel covers whole color
//!   samples, which are made as the mean of the pixels they cover (the
//!   `area` flag: ffmpeg's default filter reaches into the neighbors), so
//!   edges stay clean; at an odd multiple a color sample straddles two
//!   game pixels and every edge gets a fringe of mixed color, whatever the
//!   quality. Measured on 120 frames of an EXE6 fight (the decoded MP4
//!   against the frames, RGB PSNR): in 4:2:0, 40 dB at twice the frame
//!   and 44 dB at four times, each at about 2 Mb/s, but 25 dB at three
//!   times, at 4 Mb/s; in 4:4:4, 38 dB at three times. So a host offers
//!   even sizes, or [`Chroma::Full`], which keeps every edge at any size
//!   in a file fewer players take. The colors are converted and
//!   tagged as BT.709, so players agree on them whatever their default for
//!   the size. (The primaries and the transfer are tagged on the frames,
//!   by `setparams`: ffmpeg 9.0's encoder takes them from the frames, which
//!   the raw RGB gives none, and with `-color_primaries` and
//!   `-color_trc` alone the MP4 said "unknown" for both.) Its times are the frames' count on a tick's time base
//!   (`-framerate` the tick's rate as a fraction), exact.
//! - **The sound** is AAC at 192 kb/s, of the samples as they come: they come resampled ([`crate::resample`]:
//!   the GBA's 32768 Hz is no rate AAC has), the samples the browser's encoder is given, and ffmpeg resamples
//!   nothing.
//! - **The fragments** are about two seconds each: the picture's from each key frame, one every 120 frames
//!   (`-g`); the sound's by their length. The `moov` waits for the first (`delay_moov`), so it has each track's
//!   edit list: what is shown of the picture past its reordering, and of the sound past AAC's encoder delay (its
//!   first 1024 samples, which a player skips by it). Without it (the `moov` written before any sample) there was
//!   none, and the sound played 1024 samples, 21 ms, late.
//! - **Errors** say which of two things went wrong ([`crate::Error`]): ffmpeg couldn't be started, or one of the
//!   children failed (with what it said; one that said the disk is full is the disk's error).
//! - **Which ffmpeg**: [`find_ffmpeg`]'s, the one nettai's packages bundle, else one installed (`$NETTAI_FFMPEG`
//!   says another). The bundled one is a minimal build with what these commands need and nothing more
//!   (nettaisen/ffmpeg-build).

use crate::Error;
use crate::encode::{Audio, Backend, Chroma, Encoding, Event, Kind, Packet, Track, Video};
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::thread::JoinHandle;

/// How much of what each child says is kept for a log: the last of it.
const MOST_LOG: usize = 16 * 1024;

/// The fragmented MP4 each child writes on its standard output: about two seconds a fragment.
const OUTPUT: [&str; 7] = ["-f", "mp4", "-movflags", "+empty_moov+delay_moov+default_base_moof", "-frag_duration", "2000000", "pipe:1"];

/// The picture's ffmpeg's arguments: the raw RGB frames of `video` read from its standard input, a fragmented MP4
/// of their H.264 written to its standard output.
pub fn picture_arguments(video: &Video, encoding: &Encoding) -> Vec<OsString> {
    let format = match encoding.chroma {
        Chroma::Half => "yuv420p",
        Chroma::Full => "yuv444p",
    };
    let filter = format!(
        "scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format={format},setparams=color_primaries=bt709:color_trc=bt709"
    );
    let (size, rate, crf) = (format!("{}x{}", video.width, video.height), format!("{}/{}", video.frame_time.1, video.frame_time.0), encoding.crf.to_string());
    let input = ["-f", "rawvideo", "-pixel_format", "rgb24", "-video_size", &size, "-framerate", &rate, "-i", "pipe:0"];
    let encode = [
        "-vf", &filter, "-c:v", "libx264", "-preset", "medium", "-tune", "animation", "-crf", &crf, "-g", "120", "-colorspace", "bt709",
        "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv",
    ];
    // (Fragments from each key frame of the picture.)
    let output = OUTPUT.map(|a| if a == OUTPUT[3] { "+frag_keyframe+empty_moov+delay_moov+default_base_moof" } else { a });
    arguments(&[&input[..], &encode, &output])
}

/// A sound track's ffmpeg's arguments: stereo samples of `audio`, 32-bit floats, read from its standard input, a
/// fragmented MP4 of their AAC written to its standard output.
pub fn sound_arguments(audio: &Audio, encoding: &Encoding) -> Vec<OsString> {
    let (rate, kbps) = (audio.sample_rate.to_string(), format!("{}k", encoding.audio_kbps));
    let input = ["-f", "f32le", "-sample_rate", &rate, "-ch_layout", "stereo", "-i", "pipe:0"];
    arguments(&[&input[..], &["-c:a", "aac", "-b:a", &kbps], &OUTPUT])
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

/// ffmpeg encoding a video, a child a track: written each track's raw frames or samples, its encoded track and
/// packets read back.
pub struct Ffmpeg {
    /// The picture's child, then each sound track's.
    children: Vec<Coder>,
    /// The frames to the children's standard inputs, written on a thread of its own, until they end.
    writing: Option<Writing>,
    /// What the children's standard outputs said, read on threads of their own.
    heard: Receiver<Result<Heard, Error>>,
    /// Each track, once its child has said it; and whether they were told.
    tracks: Vec<Option<Track>>,
    told: bool,
}

/// One track's ffmpeg: the child, the thread reading its output, and the one reading what it says.
struct Coder {
    child: Child,
    reading: Option<JoinHandle<()>>,
    log: Option<JoinHandle<String>>,
}

/// What a child's output said: its track (by its place among the tracks), or a packet of it.
enum Heard {
    Track(usize, Track),
    Packet(Packet),
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
                    Written::Frame(rgb, sound) => {
                        inputs[0].write_all(&rgb).map_err(|e| (0, e))?;
                        for (track, sound) in sound.iter().enumerate() {
                            bytes.clear();
                            bytes.extend(sound.as_flattened().iter().flat_map(|s| s.to_le_bytes()));
                            inputs[1 + track].write_all(&bytes).map_err(|e| (1 + track, e))?;
                        }
                        let _ = back.send(rgb);
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

/// The desktop's encoder for a video of `video` and `audio`, as `encoding` says: ffmpeg's ([`find_ffmpeg`]).
pub(crate) fn start(video: Video, audio: Option<Audio>, encoding: &Encoding) -> Result<Box<dyn Backend>, Error> {
    Ok(Box::new(Ffmpeg::start(&find_ffmpeg(), video, audio, encoding)?))
}

impl Ffmpeg {
    /// Start `program` (an ffmpeg) encoding the picture `video` and the sound `audio`, as `encoding` says: a child
    /// for the picture and one for each sound track. An ffmpeg that can't be started is [`Error::NoFfmpeg`].
    fn start(program: &Path, video: Video, audio: Option<Audio>, encoding: &Encoding) -> Result<Ffmpeg, Error> {
        let (tell, heard) = mpsc::channel();
        let mut commands = vec![(picture_arguments(&video, encoding), Kind::Video { width: video.width, height: video.height })];
        if let Some(audio) = audio {
            commands.extend((0..audio.tracks).map(|_| (sound_arguments(&audio, encoding), Kind::Audio)));
        }
        let mut ffmpeg = Ffmpeg { children: Vec::new(), writing: None, heard, tracks: vec![None; commands.len()], told: false };
        let mut inputs = Vec::new();
        for (track, (arguments, kind)) in commands.into_iter().enumerate() {
            let (coder, input) = Coder::start(program, &arguments, track, kind, &tell).map_err(Error::NoFfmpeg)?;
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
    /// `program` started with `arguments`, encoding track `track` of `kind`: what its output says told on `tell`.
    /// Its standard input, to write to.
    fn start(program: &Path, arguments: &[OsString], track: usize, kind: Kind, tell: &Sender<Result<Heard, Error>>) -> io::Result<(Coder, ChildStdin)> {
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
        let output = child.stdout.take().expect("ffmpeg's standard output, piped");
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
        let reading = std::thread::spawn(move || read(output, track, kind, &tell));
        Ok((Coder { child, reading: Some(reading), log: Some(log) }, input))
    }

    /// What the child said, once it has ended.
    fn said(&mut self) -> String {
        self.log.take().and_then(|log| log.join().ok()).unwrap_or_default()
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
        let (which, error) = self.writing.take().map_or(Ok(()), Writing::ended).err().unwrap_or_else(|| (0, io::ErrorKind::BrokenPipe.into()));
        Err(self.failed(which, error))
    }

    fn events(&mut self) -> Result<Vec<Event>, Error> {
        let mut made = Vec::new();
        for heard in self.heard.try_iter() {
            match heard? {
                Heard::Track(at, track) => self.tracks[at] = Some(track),
                Heard::Packet(packet) => made.push(Event::Packet(packet)),
            }
        }
        // (The tracks, once every child has said its own: before any packet. The muxer holds the packets that came
        // before them.)
        if !self.told && self.tracks.iter().all(Option::is_some) {
            self.told = true;
            made.insert(0, Event::Tracks(self.tracks.iter().flatten().cloned().collect()));
        }
        Ok(made)
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<Event>, Error> {
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
            None => self.events(),
        }
    }

    fn cancel(mut self: Box<Self>) {
        self.stop();
    }
}

impl Drop for Ffmpeg {
    /// An encoder let go of before it finished is stopped.
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


/// A child's fragmented MP4, read as it comes: its track (track `at` of the video, of `kind`), then each
/// fragment's packets, told on `tell`. (It ends where the child's output does; what doesn't read is told.)
fn read(mut out: impl Read, at: usize, kind: Kind, tell: &Sender<Result<Heard, Error>>) {
    use mp4_atom::{Decode, Moof, Moov};
    let unreadable = |what: &str, e: &dyn std::fmt::Display| Error::Encoder(format!("ffmpeg's MP4: {what}: {e}"));
    // Its track's id, and its samples' defaults (the `trex`).
    let mut track: Option<(u32, mp4_atom::Trex)> = None;
    let mut moof: Option<(Moof, usize)> = None;
    loop {
        let (name, atom, header) = match next_atom(&mut out) {
            Ok(Some(atom)) => atom,
            Ok(None) => return,
            Err(e) => {
                let _ = tell.send(Err(unreadable("reading", &e)));
                return;
            }
        };
        match &name {
            b"moov" => {
                let moov = match Moov::decode(&mut &atom[..]) {
                    Ok(moov) => moov,
                    Err(e) => {
                        let _ = tell.send(Err(unreadable("its moov", &e)));
                        return;
                    }
                };
                let movie = moov.mvhd.timescale;
                let Some(trak) = moov.trak.into_iter().next() else {
                    let _ = tell.send(Err(unreadable("its moov", &"no track")));
                    return;
                };
                let id = trak.tkhd.track_id;
                let Some(codec) = trak.mdia.minf.stbl.stsd.codecs.into_iter().next() else {
                    let _ = tell.send(Err(unreadable("its track", &"no sample description")));
                    return;
                };
                // (ffmpeg's edit list, in its movie's time scale: made the MP4's.)
                let edits = trak.edts.map(|mut edts| {
                    if let Some(elst) = &mut edts.elst {
                        for e in &mut elst.entries {
                            e.segment_duration = e.segment_duration * crate::mp4::MOVIE_TIMESCALE as u64 / movie.max(1) as u64;
                        }
                    }
                    edts
                });
                let trex = moov.mvex.map(|m| m.trex).unwrap_or_default().into_iter().find(|t| t.track_id == id).unwrap_or_default();
                track = Some((id, trex));
                let _ = tell.send(Ok(Heard::Track(at, Track { kind: kind.clone(), timescale: trak.mdia.mdhd.timescale, codec, edits })));
            }
            b"moof" => match Moof::decode(&mut &atom[..]) {
                Ok(m) => moof = Some((m, atom.len())),
                Err(e) => {
                    let _ = tell.send(Err(unreadable("a moof", &e)));
                    return;
                }
            },
            b"mdat" => {
                let (Some((m, moof_len)), Some((id, trex))) = (moof.take(), &track) else { continue };
                // (Offsets are from the moof's start: the mdat's data begins past the moof and its own header.)
                let data_start = moof_len + header;
                for traf in m.traf.iter().filter(|t| t.tfhd.track_id == *id) {
                    let mut time = traf.tfdt.as_ref().map_or(0, |t| t.base_media_decode_time);
                    for trun in &traf.trun {
                        let mut place = (trun.data_offset.unwrap_or(0) as usize).saturating_sub(data_start) + header;
                        for entry in &trun.entries {
                            let size = entry.size.or(traf.tfhd.default_sample_size).unwrap_or(trex.default_sample_size) as usize;
                            let duration = entry.duration.or(traf.tfhd.default_sample_duration).unwrap_or(trex.default_sample_duration);
                            let flags = entry.flags.or(traf.tfhd.default_sample_flags).unwrap_or(trex.default_sample_flags);
                            let Some(data) = atom.get(place..place + size) else {
                                let _ = tell.send(Err(unreadable("a sample", &"past its mdat")));
                                return;
                            };
                            let packet = Packet { track: at, data: data.to_vec(), time, offset: entry.cts.unwrap_or(0), duration, key: flags & 0x0001_0000 == 0 };
                            if tell.send(Ok(Heard::Packet(packet))).is_err() {
                                return;
                            }
                            place += size;
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
