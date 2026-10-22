use super::*;

/// The command lines are the ones the module states: the picture's, at the video's size and a tick's rate, and a
/// sound track's, at its rate.
#[test]
fn the_command_lines_are_as_stated() {
    let said = |arguments: Vec<OsString>| arguments.iter().map(|a| a.to_string_lossy().into_owned()).collect::<Vec<_>>().join(" ");
    let (video, audio) = (Video { width: 480, height: 320, frame_time: (4389, 262_144) }, Audio { sample_rate: 48_000, tracks: 2 });
    assert_eq!(
        said(picture_arguments(&video, &Encoding::default())),
        "-hide_banner -nostdin -loglevel error \
         -f rawvideo -pixel_format rgb24 -video_size 480x320 -framerate 262144/4389 -i pipe:0 \
         -vf scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format=yuv420p,setparams=color_primaries=bt709:color_trc=bt709 \
         -c:v libx264 -preset medium -tune animation -crf 16 -g 120 \
         -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv \
         -f mp4 -movflags +frag_keyframe+empty_moov+delay_moov+default_base_moof -frag_duration 2000000 pipe:1"
    );
    assert_eq!(
        said(sound_arguments(&audio, &Encoding::default())),
        "-hide_banner -nostdin -loglevel error -f f32le -sample_rate 48000 -ch_layout stereo -i pipe:0 -c:a aac -b:a 192k \
         -f mp4 -movflags +empty_moov+delay_moov+default_base_moof -frag_duration 2000000 pipe:1"
    );
    let full = Encoding { crf: 0, chroma: Chroma::Full, audio_kbps: 320 };
    let picture = said(picture_arguments(&video, &full));
    assert!(picture.contains("format=yuv444p,") && picture.contains("-crf 0 "), "{picture}");
    assert!(said(sound_arguments(&audio, &full)).contains("-b:a 320k "));
}

/// The ffmpeg found: `$NETTAI_FFMPEG` as it is, else the one beside the
/// program before the path's, else the path's; never one in a relative
/// folder of the path; found nowhere, the place beside the program.
#[test]
fn ffmpeg_is_found_beside_the_program_first() {
    let dir = scratch("find");
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let (program, installed, here) = (dir.join("program"), dir.join("installed"), dir.join("here"));
    for folder in [&program, &installed, &here] {
        std::fs::create_dir_all(folder).unwrap();
    }
    std::fs::write(installed.join(name), "").unwrap();
    std::fs::write(here.join(name), "").unwrap();
    let path = |dirs: &[&Path]| std::env::join_paths(dirs).unwrap();
    assert_eq!(found_ffmpeg(Some("mine".into()), Some(&program), &path(&[&installed]), &[]), PathBuf::from("mine"));
    // Nothing beside the program: the path's.
    assert_eq!(found_ffmpeg(None, Some(&program), &path(&[&installed]), &[]), installed.join(name));
    // Bundled beside the program: that one, before the path's.
    std::fs::write(program.join(name), "").unwrap();
    assert_eq!(found_ffmpeg(None, Some(&program), &path(&[&installed]), &[]), program.join(name));
    std::fs::remove_file(program.join(name)).unwrap();
    // A relative folder of the path is passed by, though it has one (the
    // scratch folder, from the working directory: up to the root and down).
    #[cfg(unix)]
    {
        let cwd = std::env::current_dir().unwrap();
        let mut relative: PathBuf = std::iter::repeat_n("..", cwd.components().count() - 1).collect();
        relative.push(here.strip_prefix("/").unwrap());
        assert!(relative.is_relative() && relative.join(name).is_file());
        assert_eq!(found_ffmpeg(None, Some(&program), &path(&[&relative]), &[]), program.join(name));
    }
    // A package manager's folder, after the path's.
    let usual = installed.to_str().unwrap();
    assert_eq!(found_ffmpeg(None, Some(&program), &path(&[]), &[usual]), installed.join(name));
    // Found nowhere: the place beside the program.
    assert_eq!(found_ffmpeg(None, Some(&program), &path(&[]), &[]), program.join(name));
    let _ = std::fs::remove_dir_all(&dir);
}

/// One of these tests at a time: each starts a child, and a stand-in
/// written while another test's child is between its fork and its exec
/// can't be run on Linux ("text file busy").
fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());
    TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A video as the tests' stand-ins take it: a picture of 2 by 2, one sound track.
fn small() -> (Video, Option<Audio>) {
    (Video { width: 2, height: 2, frame_time: (4389, 262_144) }, Some(Audio { sample_rate: 48_000, tracks: 1 }))
}

/// An ffmpeg that isn't there is said as that.
#[test]
fn no_ffmpeg_is_said() {
    let _turn = one_at_a_time();
    let dir = scratch("none");
    let (video, audio) = small();
    assert!(matches!(Ffmpeg::start(&dir.join("no-such-ffmpeg"), video, audio, &Encoding::default()), Err(Error::NoFfmpeg(_))));
    let _ = std::fs::remove_dir_all(dir);
}

/// A folder of this test's own under the system's temporary one.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nettai-video-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The encoder's dealings with its children, with stand-ins for ffmpeg (shell scripts that take its arguments): one
/// that keeps what it is written, one that fails saying why, one that says the disk is full, one that closes its
/// input, one that never ends.
#[cfg(unix)]
mod with_a_stand_in {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A program in `dir` that runs `script` (a shell's, its arguments ffmpeg's), as ffmpeg.
    fn stand_in(dir: &Path, script: &str) -> PathBuf {
        let program = dir.join("stand-in-ffmpeg");
        std::fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        program
    }

    /// The stand-in started, and written `frames` frames of the small stream (what writing them said).
    fn fed(program: &Path, frames: usize) -> (Box<dyn Backend>, Result<(), Error>) {
        let (video, audio) = small();
        let mut ffmpeg: Box<dyn Backend> = Box::new(Ffmpeg::start(program, video, audio, &Encoding::default()).unwrap());
        let sound = [[0.25f32, -0.25]; 804];
        let written = (0..frames).try_for_each(|_| ffmpeg.frame(vec![7u8; 12], &[&sound[..]]).map(drop));
        (ffmpeg, written)
    }

    /// Each child is written its own track and nothing else: the picture's child the frames' pixels one after
    /// another, each sound track's child that track's samples, 32-bit little-endian floats, left then right.
    #[test]
    fn each_child_is_written_its_own_track() {
        let _turn = one_at_a_time();
        let dir = scratch("tracks");
        // (Each keeps its input in a file named for its input's format, and the track's place for a sound's.)
        let keeper = stand_in(&dir, &format!("case \"$6\" in rawvideo) cat > '{0}/picture';; *) cat > \"{0}/sound.$$\";; esac", dir.display()));
        let (video, _) = small();
        let audio = Some(Audio { sample_rate: 48_000, tracks: 2 });
        let mut ffmpeg: Box<dyn Backend> = Box::new(Ffmpeg::start(&keeper, video, audio, &Encoding::default()).unwrap());
        let (left, right) = ([[0.25f32, -0.5]; 3], [[1.0f32, 0.125]; 2]);
        for n in 0..4u8 {
            ffmpeg.frame(vec![n; 12], &[&left[..], &right[..]]).unwrap();
        }
        // (No MP4 came back: the stand-in says none.)
        let finished = ffmpeg.finish();
        assert!(matches!(&finished, Ok(events) if events.is_empty()), "{finished:?}");
        let picture = std::fs::read(dir.join("picture")).unwrap();
        assert_eq!(picture, (0..4u8).flat_map(|n| [n; 12]).collect::<Vec<_>>());
        let bytes = |sound: &[[f32; 2]]| -> Vec<u8> { (0..4).flat_map(|_| sound.iter().flatten().flat_map(|s| s.to_le_bytes())).collect() };
        let mut sounds: Vec<Vec<u8>> = std::fs::read_dir(&dir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with("sound.")).map(|e| std::fs::read(e.path()).unwrap()).collect();
        sounds.sort_by_key(Vec::len);
        assert_eq!(sounds, [bytes(&right), bytes(&left)]);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// An ffmpeg that fails is said with its exit code and what it wrote to its standard error, whether it fails at
    /// the end or closes its input in mid-stream; one that says the disk is full is the disk's error.
    #[test]
    fn a_failure_is_said_with_what_ffmpeg_said() {
        let _turn = one_at_a_time();
        let dir = scratch("failed");
        // At the end, having read everything.
        let (ffmpeg, written) = fed(&stand_in(&dir, "cat > /dev/null; echo 'Unknown encoder libx264' >&2; exit 8"), 30);
        written.unwrap();
        match ffmpeg.finish() {
            Err(Error::FfmpegFailed { code: Some(8), log }) => assert_eq!(log, "Unknown encoder libx264\n"),
            other => panic!("{other:?}"),
        }
        // In mid-stream: the pipe closes under the writer.
        let (_, written) = fed(&stand_in(&dir, "echo 'pipe:0: Invalid data found when processing input' >&2; exit 183"), 20_000);
        match written {
            Err(Error::FfmpegFailed { code: Some(183), log }) => assert_eq!(log, "pipe:0: Invalid data found when processing input\n"),
            other => panic!("{other:?}"),
        }
        // The disk full.
        let (ffmpeg, written) = fed(&stand_in(&dir, "cat > /dev/null; echo 'Error writing trailer: No space left on device' >&2; exit 228"), 30);
        written.unwrap();
        match ffmpeg.finish() {
            Err(Error::Disk(e)) => assert!(e.to_string().contains("No space left on device")),
            other => panic!("{other:?}"),
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Canceled, the child is stopped, one that would never end too.
    #[test]
    fn canceled_the_child_is_stopped() {
        let _turn = one_at_a_time();
        let dir = scratch("canceled");
        let (ffmpeg, written) = fed(&stand_in(&dir, "exec sleep 600"), 1);
        written.unwrap();
        let started = std::time::Instant::now();
        ffmpeg.cancel();
        assert!(started.elapsed().as_secs() < 60, "the child wasn't stopped");
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// With ffmpeg itself (not run by default: it needs `ffmpeg` and `ffprobe`
/// installed; `cargo test -p nettai-video -- --ignored`): a short set's
/// MP4 is H.264 of every frame, close to them, and AAC of its sound, as
/// many samples as it was given, its index before its samples and no
/// fragments; and a two-sided one's has a track each side, named and the
/// first the default, and the rounds as chapters.
#[test]
#[ignore]
fn ffmpeg_makes_the_mp4() {
    use crate::tests::{given, options, viewer};
    use crate::{Options, Part, Sides, Titles, write_mp4};
    use std::ops::ControlFlow;
    use std::process::Command;
    let _turn = one_at_a_time();
    let content = nettai_match::testing::exe6_content();
    let (replay, _) = nettai_frontend::testing::recorded(&content, "exe6", 7);
    let dir = scratch("ffmpeg");
    let part = Part::Ticks(200..320);
    // What the encoder is given of it: the frames, and the sound at 48 kHz.
    let given = given(&mut viewer(&content, &replay, 0), &options(2, Sides::One, &part), &mut |_| ControlFlow::Continue(())).unwrap();
    let pictures: Vec<u8> = given.frames.iter().flat_map(|(rgb, _)| rgb.iter().copied()).collect();
    let run = |program: &str, args: &[&str]| -> Vec<u8> {
        let out = Command::new(program).current_dir(&dir).args(args).output().unwrap_or_else(|e| panic!("{program}: {e}"));
        // (Every run is at `-v error`: nothing said is nothing wrong.)
        assert!(out.status.success() && out.stderr.is_empty(), "{program} {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        out.stdout
    };
    let quiet = ["-v", "error", "-nostdin"];
    // The MP4: every frame, H.264 and AAC, the picture close to the frames; its head first, then its samples.
    let mp4 = |viewer: &mut nettai_frontend::viewer::Viewer, options: &Options| -> Vec<u8> {
        let made = write_mp4(viewer, options, &Encoding::default(), Vec::new(), &mut |_| ControlFlow::Continue(())).unwrap();
        [made.head, made.body].concat()
    };
    let file = mp4(&mut viewer(&content, &replay, 0), &options(2, Sides::One, &part));
    let mut kinds = Vec::new();
    let mut at = 0;
    while at < file.len() {
        let size = match u32::from_be_bytes(file[at..at + 4].try_into().unwrap()) {
            1 => u64::from_be_bytes(file[at + 8..at + 16].try_into().unwrap()) as usize,
            size => size as usize,
        };
        kinds.push(String::from_utf8_lossy(&file[at + 4..at + 8]).into_owned());
        at += size;
    }
    assert_eq!((kinds, at), (vec!["ftyp".to_string(), "moov".into(), "mdat".into()], file.len()));
    std::fs::write(dir.join("part.mp4"), &file).unwrap();
    let probed = String::from_utf8(run(
        "ffprobe",
        &["-v", "error", "-count_frames", "-show_entries", "stream=codec_name,pix_fmt,width,height,r_frame_rate,nb_read_frames,sample_rate", "-of", "csv=p=0", "part.mp4"],
    ))
    .unwrap();
    let lines: Vec<&str> = probed.lines().collect();
    assert_eq!(lines[0], "h264,480,320,yuv420p,262144/4389,120", "{probed}");
    assert!(lines[1].starts_with("aac,48000"), "{probed}");
    let decoded = run("ffmpeg", &[&quiet[..], &["-i", "part.mp4", "-map", "0:v", "-f", "rawvideo", "-pix_fmt", "rgb24", "-fps_mode", "passthrough", "-"]].concat());
    assert_eq!(decoded.len(), pictures.len());
    // Its sound: as many samples as it was given, its encoder's delay and padding left out by its edit list (where
    // there is none, the encoder's first 1024 samples play before it, late by them).
    // (The edit's length is in the movie's milliseconds: to within one.) And it is in its place: the decoded sound
    // is the sound given at no lag, not 1024 samples after.
    let heard: Vec<f32> = run("ffmpeg", &[&quiet[..], &["-i", "part.mp4", "-map", "0:a", "-f", "f32le", "-"]].concat())
        .chunks_exact(8)
        .map(|s| f32::from_le_bytes(s[..4].try_into().unwrap()))
        .collect();
    let sound: Vec<f32> = given.track(0).iter().map(|s| s[0]).collect();
    assert!(heard.len().abs_diff(sound.len()) <= 48, "{} samples decoded of {}", heard.len(), sound.len());
    let along = |lag: i64| -> f64 { (2000..sound.len() as i64 - 2000).map(|n| heard.get((n + lag) as usize).map_or(0.0, |&h| h as f64 * sound[n as usize] as f64)).sum() };
    let best = (-1100..=1100).max_by(|&a, &b| along(a).total_cmp(&along(b))).unwrap();
    assert_eq!(best, 0, "the decoded sound is {best} samples off the sound given");
    let off: u64 = decoded.iter().zip(&pictures).map(|(a, b)| a.abs_diff(*b) as u64).sum();
    assert!((off as f64 / pictures.len() as f64) < 2.0, "the MP4's pictures are {} off a byte", off as f64 / pictures.len() as f64);
    // Both sides across the second round's start: a track each, the left
    // console's first and the default, each named for its player; the
    // rounds as chapters (Nero's chpl), the second from the frame its picture begins on
    // (59 frames in: the tick before its first).
    let second = replay.round_starts()[1];
    let titles = Titles { rounds: Vec::new(), sides: ["Lan".into(), "Chaud".into()] };
    let both = Options { scale: 1, view: None, sides: Sides::SideBySide, part: Part::Ticks(second - 60..second + 60), titles };
    std::fs::write(dir.join("both.mp4"), mp4(&mut viewer(&content, &replay, 1), &both)).unwrap();
    let entries = "stream=codec_type,codec_name:stream_disposition=default:stream_tags=name,handler_name";
    let probed = String::from_utf8(run("ffprobe", &["-v", "error", "-show_entries", entries, "-of", "csv=p=0", "both.mp4"])).unwrap();
    assert_eq!(
        probed.lines().collect::<Vec<_>>(),
        ["h264,video,1,nettai", "aac,audio,1,Chaud", "aac,audio,0,Lan"],
        "{probed}"
    );
    let entries = "chapter=start_time,end_time:chapter_tags=title";
    let probed = String::from_utf8(run("ffprobe", &["-v", "error", "-show_entries", entries, "-of", "csv=p=0", "both.mp4"])).unwrap();
    let at = |frames: f64| format!("{:.6}", frames * 4389.0 / 262_144.0);
    // (The last ends with the file, whose sound runs to its last AAC frame's end, a frame past the picture's.)
    let length = String::from_utf8(run("ffprobe", &["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", "both.mp4"])).unwrap();
    let length = format!("{:.6}", length.trim().parse::<f64>().unwrap());
    assert!(length.parse::<f64>().unwrap() - 120.0 * 4389.0 / 262_144.0 < 0.05, "the file is {length} s long");
    assert_eq!(probed.lines().collect::<Vec<_>>(), [format!("0.000000,{},Round 1", at(59.0)), format!("{},{length},Round 2", at(59.0))], "{probed}");
    // The picture is shown from its start (its edit list past x264's reordering).
    let starts = String::from_utf8(run("ffprobe", &["-v", "error", "-select_streams", "v", "-show_entries", "stream=start_time", "-of", "csv=p=0", "both.mp4"])).unwrap();
    assert_eq!(starts.trim(), "0.000000", "the picture starts at {starts}");
    // Each track decodes to its side's sound: they differ.
    let left = run("ffmpeg", &[&quiet[..], &["-i", "both.mp4", "-map", "0:a:0", "-f", "f32le", "-"]].concat());
    let right = run("ffmpeg", &[&quiet[..], &["-i", "both.mp4", "-map", "0:a:1", "-f", "f32le", "-"]].concat());
    assert!(left.len() == right.len() && left != right);
    let _ = std::fs::remove_dir_all(dir);
}
