use super::*;

/// The command line is the one the module states.
#[test]
fn the_command_line_is_as_stated() {
    let said: Vec<String> = arguments(&Encoding::default()).iter().map(|a| a.to_string_lossy().into_owned()).collect();
    assert_eq!(
        said.join(" "),
        "-hide_banner -nostdin -loglevel error -f nut -i pipe:0 -map 0:v -map 0:a? \
         -vf scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format=yuv420p,setparams=color_primaries=bt709:color_trc=bt709 \
         -c:v libx264 -preset medium -tune animation -crf 16 -g 120 \
         -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv \
         -c:a aac -b:a 192k -f mp4 -movflags +frag_keyframe+empty_moov+default_base_moof -frag_duration 2000000 pipe:1"
    );
    let full = arguments(&Encoding { crf: 0, chroma: Chroma::Full, audio_kbps: 320, ..Encoding::default() });
    let said: Vec<String> = full.iter().map(|a| a.to_string_lossy().into_owned()).collect();
    assert!(said.contains(&"scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format=yuv444p,setparams=color_primaries=bt709:color_trc=bt709".to_string()));
    assert!(said.windows(2).any(|w| w == ["-crf", "0"]) && said.windows(2).any(|w| w == ["-b:a", "320k"]));
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

/// A video's stream as the tests' stand-ins take it: a picture of 2 by 2, one sound track.
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

/// The encoder's dealings with its child, with stand-ins for ffmpeg (shell scripts that take its arguments): one
/// that fails saying why, one that says the disk is full, one that closes its input, one that never ends.
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
/// stream is read by ffmpeg as the frames and the samples that were
/// written, bit for bit, at the frame rate and sample rate stated; its
/// MP4 is H.264 and AAC of every frame, close to them, its index before
/// its samples and no fragments; and a two-sided one's has a track each
/// side, named and the first the default, and the rounds as chapters.
#[test]
#[ignore]
fn ffmpeg_reads_the_stream_and_makes_the_mp4() {
    use crate::tests::{given, options, viewer};
    use crate::{Options, Part, Sides, Titles, write_mp4};
    use std::ops::ControlFlow;
    use std::process::Command;
    let _turn = one_at_a_time();
    let content = nettai_match::testing::exe6_content();
    let (replay, _) = nettai_frontend::testing::recorded(&content, "exe6", 7);
    let dir = scratch("ffmpeg");
    let part = Part::Ticks(200..320);
    // The stream ffmpeg is given of it: what the encoder is given, as its writer writes it.
    let given = given(&mut viewer(&content, &replay, 0), &options(2, Sides::One, &part), &mut |_| ControlFlow::Continue(())).unwrap();
    let mut stream = nut::Writer::new(Vec::new(), given.video, given.audio).unwrap();
    for (rgb, sound) in &given.frames {
        stream.frame(rgb, &sound.iter().map(Vec::as_slice).collect::<Vec<_>>()).unwrap();
    }
    let nut = stream.finish().unwrap();
    let file = nut::reading::read(&nut).unwrap();
    std::fs::write(dir.join("part.nut"), &nut).unwrap();
    let run = |program: &str, args: &[&str]| -> Vec<u8> {
        let out = Command::new(program).current_dir(&dir).args(args).output().unwrap_or_else(|e| panic!("{program}: {e}"));
        // (Every run is at `-v error`: nothing said is nothing wrong.)
        assert!(out.status.success() && out.stderr.is_empty(), "{program} {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        out.stdout
    };
    // What ffprobe makes of the stream.
    let probed = String::from_utf8(run(
        "ffprobe",
        &["-v", "error", "-show_entries", "stream=codec_name,pix_fmt,width,height,r_frame_rate,time_base,sample_rate,channels,sample_fmt", "-of", "csv=p=0", "part.nut"],
    ))
    .unwrap();
    assert_eq!(probed.lines().collect::<Vec<_>>(), ["rawvideo,480,320,rgb24,262144/4389,4389/262144", "pcm_f32le,flt,48000,2,0/0,1/48000"], "{probed}");
    // The file's length, which ffprobe has from the index: the last
    // frame's time (120 frames: the 120th starts 119 frame times in).
    let length: f64 = String::from_utf8(run("ffprobe", &["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", "part.nut"])).unwrap().trim().parse().unwrap();
    assert!((length - 119.0 * 4389.0 / 262_144.0).abs() < 0.01, "ffprobe says the stream is {length} s long");
    // Its frames and samples, as ffmpeg decodes them: the ones written.
    let pictures: Vec<u8> = file.frames.iter().filter(|f| f.stream == 0).flat_map(|f| f.data.iter().copied()).collect();
    let sound: Vec<u8> = file.frames.iter().filter(|f| f.stream == 1).flat_map(|f| f.data.iter().copied()).collect();
    let quiet = ["-v", "error", "-nostdin"];
    assert!(run("ffmpeg", &[&quiet[..], &["-i", "part.nut", "-map", "0:v", "-f", "rawvideo", "-pix_fmt", "rgb24", "-fps_mode", "passthrough", "-"]].concat()) == pictures);
    assert!(run("ffmpeg", &[&quiet[..], &["-i", "part.nut", "-map", "0:a", "-f", "f32le", "-"]].concat()) == sound);
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
