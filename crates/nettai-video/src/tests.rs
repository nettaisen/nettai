use super::*;
use crate::nut::reading;
use nettai_battle::content::Content;
use nettai_frontend::Player;
use nettai_frontend::game::TextMode;
use nettai_frontend::replay::ReplayPlayer;
use nettai_frontend::testing::{audio, graphics, recorded};
use std::sync::Arc;

/// A viewer of `replay` from `view`'s side, on graphics without a pack and
/// a sound of every cue.
pub(crate) fn viewer(content: &Arc<Content>, replay: &Replay, view: u8) -> Viewer {
    let renderer = graphics(content, 0x7C1F).renderer(TextMode::Original, None);
    Viewer::with(content, replay, Some(view), renderer, None, Some(audio(content))).unwrap()
}

fn go_on(_: Progress) -> ControlFlow<()> {
    ControlFlow::Continue(())
}

/// `part` of `replay` from `view`'s side as a NUT stream, the picture
/// `scale` times the frame.
fn exported(content: &Arc<Content>, replay: &Replay, view: u8, scale: u32, part: &Part) -> Vec<u8> {
    write(&mut viewer(content, replay, view), scale, part, Vec::new(), &mut go_on).unwrap()
}

/// What a player playing `replay` straight presents and plays, a tick at
/// a time: each tick's picture at `scale` times the frame as RGB bytes,
/// and its samples. (A [`Player`]: nothing of the viewer or its seeking.)
fn played(content: &Arc<Content>, replay: &Replay, view: u8, scale: usize) -> Vec<(Vec<u8>, Vec<[f32; 2]>)> {
    let driver = ReplayPlayer::new(content, replay, Some(view)).unwrap();
    let renderer = graphics(content, 0x7C1F).renderer(TextMode::Original, None);
    let mut p = Player::with(renderer, None, Some(audio(content)), Box::new(driver));
    let (w, h) = (WIDTH * scale, HEIGHT * scale);
    let mut buffer = vec![0u32; w * h];
    let mut out = Vec::new();
    while p.tick(0) {
        p.present(&mut buffer, w, h);
        let rgb = buffer.iter().flat_map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]).collect();
        let mut sound = Vec::new();
        p.take_samples(&mut sound);
        out.push((rgb, sound));
    }
    out
}

/// A stream's pictures in order, and its sound as one run of samples.
fn pictures_and_sound(file: &reading::File) -> (Vec<&[u8]>, Vec<[f32; 2]>) {
    let pictures = file.frames.iter().filter(|f| f.stream == 0).map(|f| &f.data[..]).collect();
    let sound = file
        .frames
        .iter()
        .filter(|f| f.stream == 1)
        .flat_map(|f| f.data.chunks_exact(8))
        .map(|s| [f32::from_le_bytes(s[..4].try_into().unwrap()), f32::from_le_bytes(s[4..].try_into().unwrap())])
        .collect();
    (pictures, sound)
}

/// A replay written whole is, frame for frame and sample for sample, what
/// a player playing it straight presents and plays: one picture a tick at
/// the scale asked for, on the tick's exact time base, and each tick's
/// sound beside it, in a stream that reads back by NUT's syntax. From
/// either console; and the two consoles' videos differ.
#[test]
fn a_replay_is_written_as_it_is_shown_and_heard() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let mut sides = Vec::new();
    for (view, scale) in [(0u8, 2u32), (1, 1)] {
        let bytes = exported(&content, &replay, view, scale, &Part::Whole);
        let file = reading::read(&bytes).unwrap_or_else(|e| panic!("side {view}: {e}"));
        let straight = played(&content, &replay, view, scale as usize);
        assert_eq!(straight.len() as u64, ticks);
        let (picture, sound) = (&file.streams[0], &file.streams[1]);
        assert_eq!((picture.size, picture.time_base, &picture.fourcc[..]), ((240 * scale as u64, 160 * scale as u64), (4389, 262_144), &b"RGB\x18"[..]));
        assert_eq!((sound.sample_rate, sound.channels, sound.time_base, &sound.fourcc[..]), ((32_768, 1), 2, (1, 32_768), &b"PFD\x20"[..]));
        let (pictures, heard) = pictures_and_sound(&file);
        assert_eq!(pictures.len() as u64, ticks, "side {view}: a picture a tick");
        for (tick, (shown, (presented, _))) in pictures.iter().zip(&straight).enumerate() {
            assert!(shown == presented, "side {view}: the picture of tick {tick}");
        }
        let made: Vec<[f32; 2]> = straight.iter().flat_map(|(_, sound)| sound.iter().copied()).collect();
        assert!(heard == made, "side {view}: {} samples for {}", heard.len(), made.len());
        // The pictures aren't all one, nor the sound silence; and the
        // sound is the picture's length to within a sample a tick.
        assert!(pictures.iter().any(|p| p != &pictures[0]) && heard.iter().any(|s| s[0] != 0.0), "side {view}: nothing to tell by");
        assert!((heard.len() as f64 - ticks as f64 * 4389.0 / 8.0).abs() < 2.0, "side {view}: {} samples for {ticks} ticks", heard.len());
        // Each picture's pts is its number; each sound frame's, the
        // samples before it.
        let mut samples = 0;
        for (n, f) in file.frames.iter().filter(|f| f.stream == 0).enumerate() {
            assert_eq!(f.pts, n as u64);
        }
        for f in file.frames.iter().filter(|f| f.stream == 1) {
            assert_eq!(f.pts, samples);
            samples += f.data.len() as u64 / 8;
        }
        sides.push(bytes);
    }
    assert!(sides[0] != sides[1]);
}

/// The same replay gives the same bytes, from a viewer made anew each
/// time; and a viewer that has been about the replay first (sought,
/// played, shown from the other side) writes them too.
#[test]
fn the_same_replay_gives_the_same_bytes() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let first = exported(&content, &replay, 0, 1, &Part::Whole);
    assert!(first == exported(&content, &replay, 0, 1, &Part::Whole), "another run, other bytes");
    let mut v = viewer(&content, &replay, 1);
    v.seek(ticks as u32 / 2);
    for _ in 0..30 {
        v.step_forward();
    }
    v.set_view(0);
    v.set_speed(Speed::Double);
    let after = write(&mut v, 1, &Part::Whole, Vec::new(), &mut go_on).unwrap();
    assert!(first == after, "a viewer that had been about the replay writes other bytes");
    assert!(v.paused() && v.at_end() && v.speed() == Speed::Normal);
}

/// A part is the whole video's frames and samples for its ticks: a round
/// alone, and a range of ticks from the middle of one into the next,
/// start where playing straight there gets (the sound in mid-note
/// included) and stop at their last tick; their pts count from the part's
/// start.
#[test]
fn a_part_is_the_whole_videos_for_its_ticks() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let (length, second) = (ticks as u32, replay.round_starts()[1]);
    let straight = played(&content, &replay, 0, 1);
    // (A tick in the first round's later half that the sound sounds
    // through: a part from it starts in mid-note.)
    let sounds = |tick: usize| straight[tick].1.iter().any(|s| s[0] != 0.0);
    let sounding = (second as usize / 2..second as usize - 60).find(|&t| sounds(t - 1) && sounds(t) && sounds(t + 1)).expect("a tick in mid-sound") as u32;
    for (part, range) in [
        (Part::Round(1), 0..second),
        (Part::Round(2), second..length),
        (Part::Ticks(second - 100..second + 150), second - 100..second + 150),
        (Part::Ticks(sounding..sounding + 50), sounding..sounding + 50),
        (Part::Ticks(length - 1..length), length - 1..length),
    ] {
        assert_eq!(part.ticks(&replay.round_starts(), length).unwrap(), range, "{part:?}");
        let file = reading::read(&exported(&content, &replay, 0, 1, &part)).unwrap_or_else(|e| panic!("{part:?}: {e}"));
        let (pictures, heard) = pictures_and_sound(&file);
        let of_part = &straight[range.start as usize..range.end as usize];
        assert_eq!(pictures.len(), of_part.len(), "{part:?}");
        assert!(pictures.iter().zip(of_part).all(|(shown, (presented, _))| shown == presented), "{part:?}: its pictures");
        let made: Vec<[f32; 2]> = of_part.iter().flat_map(|(_, sound)| sound.iter().copied()).collect();
        assert!(heard == made, "{part:?}: its sound");
        assert_eq!(file.frames.iter().find(|f| f.stream == 0).map(|f| f.pts), Some(0), "{part:?}");
        assert_eq!(file.frames.iter().find(|f| f.stream == 1).map(|f| f.pts), Some(0), "{part:?}");
    }
}

/// The progress is told before the first frame and after each, with the
/// round being written; an answer to stop cancels the video where it is.
/// A viewer without sound writes the picture's stream alone.
#[test]
fn progress_is_told_and_may_cancel() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let (length, second) = (ticks as u32, replay.round_starts()[1]);
    let mut told = Vec::new();
    let mut tell = |p: Progress| {
        told.push(p);
        ControlFlow::Continue(())
    };
    write(&mut viewer(&content, &replay, 0), 1, &Part::Whole, Vec::new(), &mut tell).unwrap();
    assert_eq!(told.len() as u32, length + 1);
    assert_eq!(told[0], Progress { written: 0, of: length, round: 1, rounds: 2 });
    assert!(told.iter().enumerate().all(|(n, p)| p.written == n as u32 && p.of == length && p.rounds == 2));
    // (The frame a tick leaves is of the tick's round.)
    assert_eq!((told[second as usize].round, told[second as usize + 1].round, told[length as usize].round), (1, 2, 2));
    assert_eq!((told[0].fraction(), told[length as usize].fraction()), (0.0, 1.0));
    // A part far in: told of while the viewer gets there, nothing written.
    let mut told = Vec::new();
    let mut tell = |p: Progress| {
        told.push(p);
        ControlFlow::Continue(())
    };
    write(&mut viewer(&content, &replay, 0), 1, &Part::Ticks(length - 10..length), Vec::new(), &mut tell).unwrap();
    let before = told.iter().filter(|p| p.written == 0).count();
    assert!(before > 1 && told.len() == before + 10, "{before} of {}", told.len());
    assert!(told.iter().all(|p| p.of == 10 && p.round == 2));
    // Canceled after its twentieth frame.
    let mut frames = 0;
    let mut stop_at_twenty = |p: Progress| {
        frames = p.written;
        if p.written == 20 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    };
    let canceled = write(&mut viewer(&content, &replay, 0), 1, &Part::Whole, Vec::new(), &mut stop_at_twenty);
    assert!(matches!(canceled, Err(Error::Canceled)) && frames == 20);
    // Without sound.
    let renderer = graphics(&content, 0x7C1F).renderer(TextMode::Original, None);
    let mut silent = Viewer::with(&content, &replay, Some(0), renderer, None, None).unwrap();
    let file = reading::read(&write(&mut silent, 1, &Part::Round(1), Vec::new(), &mut go_on).unwrap()).unwrap();
    assert_eq!((file.streams.len(), file.frames.len() as u32), (1, second));
}

/// What can't be made is refused before anything is written, saying what.
#[test]
fn what_cant_be_made_is_refused() {
    let content = nettai_match::testing::exe6_content();
    let (replay, ticks) = recorded(&content, "exe6", 7);
    let length = ticks as u32;
    let mut v = viewer(&content, &replay, 0);
    let mut refused = |scale, part: Part| {
        let mut out = Vec::new();
        let e = write(&mut v, scale, &part, &mut out, &mut go_on).unwrap_err();
        assert!(out.is_empty(), "{part:?}: something was written");
        e.to_string()
    };
    assert_eq!(refused(0, Part::Whole), "a picture 0 times the frame's size (1 to 16)");
    assert_eq!(refused(17, Part::Whole), "a picture 17 times the frame's size (1 to 16)");
    assert_eq!(refused(1, Part::Round(3)), "no round 3 of a replay of 2 rounds");
    assert_eq!(refused(1, Part::Round(0)), "no round 0 of a replay of 2 rounds");
    assert_eq!(refused(1, Part::Ticks(5..5)), format!("no ticks 5 to 5 of a replay of {length} ticks"));
    assert_eq!(refused(1, Part::Ticks(0..length + 1)), format!("no ticks 0 to {} of a replay of {length} ticks", length + 1));
    // A replay of another game isn't opened.
    let five = nettai_match::testing::exe5_content();
    let renderer = graphics(&five, 0x7C1F).renderer(TextMode::Original, None);
    let other = Viewer::with(&five, &replay, None, renderer, None, None).map_err(Error::Replay).err().unwrap();
    assert_eq!(other.to_string(), "the replay isn't played: the replay is of exe6, not exe5");
}
