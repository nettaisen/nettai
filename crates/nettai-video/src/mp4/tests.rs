use super::*;

/// The command line is the one the module states.
#[test]
fn the_command_line_is_as_stated() {
    let args = Settings::default().arguments(Path::new("out.mp4.part"));
    let said: Vec<String> = args.iter().map(|a| a.to_string_lossy().into_owned()).collect();
    assert_eq!(
        said.join(" "),
        "-hide_banner -nostdin -loglevel error -f nut -i pipe:0 \
         -vf scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int+area,format=yuv420p,setparams=color_primaries=bt709:color_trc=bt709 \
         -c:v libx264 -preset medium -tune animation -crf 16 \
         -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv \
         -c:a aac -b:a 192k -ar 48000 -movflags +faststart -f mp4 -y out.mp4.part"
    );
    let full = Settings { crf: 0, chroma: Chroma::Full, audio_kbps: 320, ..Settings::default() }.arguments(Path::new("o"));
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

/// An ffmpeg that isn't there is said as that, and leaves nothing; so is a
/// place no file can be made in, as the disk's.
#[test]
fn no_ffmpeg_and_no_place_are_told_apart() {
    let _turn = one_at_a_time();
    let dir = scratch("none");
    let to = dir.join("out.mp4");
    let settings = Settings { ffmpeg: dir.join("no-such-ffmpeg"), ..Settings::default() };
    assert!(matches!(Encoder::start(&to, &settings), Err(Error::NoFfmpeg(_))));
    assert_eq!(left_in(&dir), Vec::<String>::new());
    let nowhere = dir.join("no-such-folder").join("out.mp4");
    assert!(matches!(Encoder::start(&nowhere, &settings), Err(Error::Disk(_))));
    assert!(matches!(Encoder::start(Path::new("/"), &settings), Err(Error::Disk(_))));
    let _ = std::fs::remove_dir_all(dir);
}

/// A folder of this test's own under the system's temporary one.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nettai-video-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The files in `dir`, by name.
fn left_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

/// The encoder's dealings with its child, with stand-ins for ffmpeg (shell
/// scripts that take its arguments: the file to write is the last): one
/// that copies what it is piped into the file, one that fails saying why,
/// one that says the disk is full, one that never ends.
#[cfg(unix)]
mod with_a_stand_in {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A program in `dir` that runs `script` (a shell's, its arguments
    /// ffmpeg's), as the settings' ffmpeg.
    fn stand_in(dir: &Path, script: &str) -> Settings {
        let program = dir.join("stand-in-ffmpeg");
        std::fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        Settings { ffmpeg: program, ..Settings::default() }
    }

    /// (The last argument: the file.)
    const OUT: &str = "for last; do :; done";

    /// A megabyte of stream: more than a pipe holds.
    fn stream() -> Vec<u8> {
        (0..1 << 20).map(|i| (i % 251) as u8).collect()
    }

    /// The file takes its name once the child has ended well, whole;
    /// until then only the part is there.
    #[test]
    fn the_file_takes_its_name_when_it_is_whole() {
        let _turn = one_at_a_time();
        let dir = scratch("whole");
        let settings = stand_in(&dir, &format!("{OUT}; cat > \"$last\""));
        let to = dir.join("out.mp4");
        std::fs::write(&to, b"an older video").unwrap();
        let mut encoder = Encoder::start(&to, &settings).unwrap();
        encoder.write_all(&stream()).unwrap();
        assert_eq!(left_in(&dir), ["out.mp4", "out.mp4.part", "stand-in-ffmpeg"]);
        assert_eq!(std::fs::read(&to).unwrap(), b"an older video", "the file before is there until the new one is whole");
        encoder.finish().unwrap();
        assert_eq!(left_in(&dir), ["out.mp4", "stand-in-ffmpeg"]);
        assert!(std::fs::read(&to).unwrap() == stream());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Canceled or let go of, the child is stopped (one that would never
    /// end, too) and nothing is left; a file of that name from before
    /// stays as it was.
    #[test]
    fn canceled_nothing_is_left() {
        let _turn = one_at_a_time();
        let dir = scratch("canceled");
        let settings = stand_in(&dir, &format!("{OUT}; head -c 1048576 > \"$last\"; exec sleep 600"));
        let to = dir.join("out.mp4");
        let mut encoder = Encoder::start(&to, &settings).unwrap();
        encoder.write_all(&stream()).unwrap();
        let started = std::time::Instant::now();
        encoder.cancel();
        assert!(started.elapsed().as_secs() < 60, "the child wasn't stopped");
        assert_eq!(left_in(&dir), ["stand-in-ffmpeg"]);
        std::fs::write(&to, b"an older video").unwrap();
        let mut encoder = Encoder::start(&to, &settings).unwrap();
        encoder.write_all(&stream()).unwrap();
        drop(encoder);
        assert_eq!(left_in(&dir), ["out.mp4", "stand-in-ffmpeg"]);
        assert_eq!(std::fs::read(&to).unwrap(), b"an older video");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// An ffmpeg that fails is said with its exit code and what it wrote
    /// to its standard error, whether it fails at the end or closes its
    /// pipe in mid-stream; one that says the disk is full is the disk's
    /// error. Nothing is left either way.
    #[test]
    fn a_failure_is_said_with_what_ffmpeg_said() {
        let _turn = one_at_a_time();
        let dir = scratch("failed");
        let to = dir.join("out.mp4");
        // At the end, having read everything.
        let settings = stand_in(&dir, "cat > /dev/null; echo 'Unknown encoder libx264' >&2; exit 8");
        let mut encoder = Encoder::start(&to, &settings).unwrap();
        encoder.write_all(&stream()).unwrap();
        match encoder.finish() {
            Err(Error::FfmpegFailed { code: Some(8), log }) => assert_eq!(log, "Unknown encoder libx264\n"),
            other => panic!("{other:?}"),
        }
        assert_eq!(left_in(&dir), ["stand-in-ffmpeg"]);
        // In mid-stream: the pipe closes under the writer.
        let settings = stand_in(&dir, "echo 'pipe:0: Invalid data found when processing input' >&2; exit 183");
        let mut encoder = Encoder::start(&to, &settings).unwrap();
        let broke = (0..64).find_map(|_| encoder.write_all(&stream()).err()).expect("the pipe closes");
        match encoder.failed(broke) {
            Error::FfmpegFailed { code: Some(183), log } => assert_eq!(log, "pipe:0: Invalid data found when processing input\n"),
            other => panic!("{other:?}"),
        }
        assert_eq!(left_in(&dir), ["stand-in-ffmpeg"]);
        // The disk full.
        let settings = stand_in(&dir, "cat > /dev/null; echo 'Error writing trailer: No space left on device' >&2; exit 228");
        let mut encoder = Encoder::start(&to, &settings).unwrap();
        encoder.write_all(&stream()).unwrap();
        match encoder.finish() {
            Err(Error::Disk(e)) => assert!(e.to_string().contains("No space left on device")),
            other => panic!("{other:?}"),
        }
        assert_eq!(left_in(&dir), ["stand-in-ffmpeg"]);
        // An error of the writer's own (not the pipe's) is the writer's.
        let settings = stand_in(&dir, "cat > /dev/null");
        let encoder = Encoder::start(&to, &settings).unwrap();
        assert!(matches!(encoder.failed(io::Error::other("the replay's own")), Error::Write(_)));
        assert_eq!(left_in(&dir), ["stand-in-ffmpeg"]);
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// With ffmpeg itself (not run by default: it needs `ffmpeg` and `ffprobe`
/// installed; `cargo test -p nettai-video -- --ignored`): a short set's
/// stream is read by ffmpeg as the frames and the samples that were
/// written, bit for bit, at the frame rate and sample rate stated; and
/// its MP4 is H.264 and AAC of every frame, close to them.
#[test]
#[ignore]
fn ffmpeg_reads_the_stream_and_makes_the_mp4() {
    use crate::tests::viewer;
    use crate::{Part, Sides, write, write_mp4};
    use std::ops::ControlFlow;
    use std::process::Command;
    let _turn = one_at_a_time();
    let content = nettai_match::testing::exe6_content();
    let (replay, _) = nettai_frontend::testing::recorded(&content, "exe6", 7);
    let dir = scratch("ffmpeg");
    let part = Part::Ticks(200..320);
    let nut = write(&mut viewer(&content, &replay, 0), 2, Sides::One, &part, Vec::new(), &mut |_| ControlFlow::Continue(())).unwrap();
    let file = crate::nut::reading::read(&nut).unwrap();
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
    assert_eq!(probed.lines().collect::<Vec<_>>(), ["rawvideo,480,320,rgb24,262144/4389,4389/262144", "pcm_f32le,flt,32768,2,0/0,1/32768"], "{probed}");
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
    // The MP4: every frame, H.264 and AAC, the picture close to the frames.
    let to = dir.join("part.mp4");
    write_mp4(&mut viewer(&content, &replay, 0), 2, Sides::One, &part, &Settings::default(), &to, &mut |_| ControlFlow::Continue(())).unwrap();
    assert_eq!(left_in(&dir), ["part.mp4", "part.nut"]);
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
    let _ = std::fs::remove_dir_all(dir);
}
